//! The light solver (SPEC §4.1): a coarse grid over the view, filled from the
//! cells (what glows, what blocks), seeded with sky light down open columns,
//! carried lights and flashlight rays, then spread Terraria-style: every
//! texel takes the brightest of its neighbours' light, dimmed by what it
//! passed through. Eight directions (a chamfer sweep), so light pools come
//! out round, not diamond-shaped.
//!
//! Pure data in, pure data out: no Bevy, no GPU. The game uploads `light`
//! and `glow` as textures and multiplies (adds) them over the scene.

use platypus_sim::cell::flags;
use platypus_sim::{CHUNK, CellPos, ChunkPos, Kind, World};
use rayon::prelude::*;

pub type Rgb = [f32; 3];

/// How light behaves, from `lighting.ron`.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// Light kept per cell of clear air.
    pub air_falloff: f32,
    /// Sweep pairs; 2 lets light turn a corner and come back.
    pub iterations: usize,
    /// Light kept per cell going into rock, for showing a lit face a few
    /// cells deep (it never comes back out: no light through walls).
    pub rim: f32,
}

pub struct LightGrid {
    /// Cells per texel, each way (1, 2, 4 or 8: divides a chunk).
    pub texel: i32,
    pub w: usize,
    pub h: usize,
    /// World cell at the bottom-left of texel (0, 0). Row 0 is the bottom.
    pub origin: CellPos,
    /// Light stopped per cell in the texel (0..1): the playfield only.
    pub opacity: Vec<f32>,
    /// The same for sky light, which the background (tree crowns, walls)
    /// also shades.
    pub sky_opacity: Vec<f32>,
    /// Light given off in the texel.
    pub emit: Vec<Rgb>,
    /// Seeds: sky, carried lights (and `emit`, added when solving).
    pub seed: Vec<Rgb>,
    /// Direct light from beams: not spread (so shadows stay sharp), only a
    /// share of it scatters into `seed`.
    pub direct: Vec<Rgb>,
    /// The result: light falling on each texel (0..1+).
    pub light: Vec<Rgb>,
    /// Light from glowing things only, for the haze drawn over the dark.
    pub glow: Vec<Rgb>,
    /// What light meets in each texel (`Optics`): its refractive index (1:
    /// air, or nothing clear), how that spreads by colour, how much of it
    /// its faces mirror at least, and its tint (per `Optics::TINT_DEPTH`
    /// cells).
    pub ior: Vec<f32>,
    pub disperse: Vec<f32>,
    pub reflect: Vec<f32>,
    pub tint: Vec<Rgb>,
    /// Anything clear in the view at all (else rays go straight, as ever).
    pub optical: bool,
    /// A cell in the texel that glints in light (a gem, crystal, glass, ice,
    /// water at its surface; one at random each frame), and its colour.
    /// (And whether any light makes it glint, a torch's spread as much as a
    /// beam: metal and gems, `sparkle`; else only direct light and the sky.)
    pub glint: Vec<Option<(CellPos, Rgb, bool)>>,
    /// Luster in the texel (`MaterialDef::luster`): its colour and the share
    /// of the texel it fills. It shines as light falls on it (`shine`).
    pub luster: Vec<Option<(Rgb, f32)>>,
    /// Phosphor in the texel (a quarter of it or more): the colour it glows,
    /// and how charged it is (0 … 1, kept from frame to frame).
    pub phosphor: Vec<Option<Rgb>>,
    pub charge: Vec<f32>,
    /// Caustics: light bent, mirrored or split by something clear, where it
    /// lands on a face; added over the scene (the glow layer), so it shows
    /// in daylight too (light only darkens: a spire's colours on lit
    /// ground didn't show).
    pub caustic: Vec<Rgb>,
}

impl LightGrid {
    /// A grid of `w`×`h` texels of `texel` cells from `origin`, all clear.
    pub fn new(origin: CellPos, w: usize, h: usize, texel: i32) -> Self {
        let n = w * h;
        LightGrid {
            texel,
            w,
            h,
            origin,
            opacity: vec![0.0; n],
            sky_opacity: vec![0.0; n],
            emit: vec![[0.0; 3]; n],
            seed: vec![[0.0; 3]; n],
            direct: vec![[0.0; 3]; n],
            light: vec![[0.0; 3]; n],
            glow: vec![[0.0; 3]; n],
            ior: vec![1.0; n],
            disperse: vec![0.0; n],
            reflect: vec![0.0; n],
            tint: vec![[1.0; 3]; n],
            optical: false,
            glint: vec![None; n],
            luster: vec![None; n],
            caustic: vec![[0.0; 3]; n],
            phosphor: vec![None; n],
            charge: vec![0.0; n],
        }
    }

    #[inline]
    pub fn index_of(&self, p: CellPos) -> Option<usize> {
        let (dx, dy) = (p.x - self.origin.x, p.y - self.origin.y);
        if dx < 0 || dy < 0 {
            return None;
        }
        let (tx, ty) = ((dx / self.texel) as usize, (dy / self.texel) as usize);
        (tx < self.w && ty < self.h).then_some(ty * self.w + tx)
    }

    /// Fill `opacity`, `sky_opacity` and `emit` from the loaded world.
    /// `flicker(texel x, texel y)` scales what burns (0..1). Unloaded parts
    /// stay clear and dark.
    /// `flicker`: a fire's brightness at a cell now; `breath`: 0..1, where a
    /// breathing glow (`MaterialDef::pulse`) is in its cycle at a cell now.
    /// `glimmer`: 0..1, how bright a glimmering glow (`MaterialDef::shimmer`)
    /// is at a cell now.
    pub fn fill_from(
        &mut self,
        world: &World,
        flicker: &(impl Fn(i32, i32) -> f32 + Sync),
        breath: &(impl Fn(i32, i32) -> f32 + Sync),
        glimmer: &(impl Fn(i32, i32) -> f32 + Sync),
    ) {
        let t = self.texel;
        let per_chunk = (CHUNK / t) as usize;
        let x_end = self.origin.x + self.w as i32 * t;
        let y_end = self.origin.y + self.h as i32 * t;
        let (c0, c1) = (self.origin.chunk(), CellPos::new(x_end - 1, y_end - 1).chunk());
        let chunks: Vec<ChunkPos> = (c0.y..=c1.y).flat_map(|cy| (c0.x..=c1.x).map(move |cx| ChunkPos::new(cx, cy))).collect();
        let mats = world.materials();
        let climate = world.climate();
        let area = (t * t) as f32;
        // Each material's optics, at hand.
        let optics: Vec<Option<platypus_sim::Optics>> = (0..mats.len()).map(|m| mats.def(platypus_sim::MaterialId(m as u16)).optics).collect();
        // Per chunk, the texels it covers (each texel lies in one chunk).
        // (opacity, sky opacity, glow) per texel, and its clear cells: how
        // many, and their index, spread, mirroring and tint, summed.
        // (Luster: how many of its cells shine when lit, and their colour.)
        type Texels = Vec<(f32, f32, Rgb, Clear, Option<(CellPos, Rgb, u64, bool)>, (f32, Rgb), (f32, Rgb))>;
        let rgb = |(r, g, b): (u8, u8, u8)| [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0];
        let glows: Vec<Option<Rgb>> = (0..mats.len()).map(|m| mats.def(platypus_sim::MaterialId(m as u16)).phosphor.map(rgb)).collect();
        let lusters: Vec<Option<Rgb>> = (0..mats.len()).map(|m| mats.def(platypus_sim::MaterialId(m as u16)).luster.map(rgb)).collect();
        // (What sparkles, and in what colour: its brightest, lifted.)
        let sparkles: Vec<Option<Rgb>> = (0..mats.len())
            .map(|m| {
                let def = mats.def(platypus_sim::MaterialId(m as u16));
                def.sparkle.then(|| def.colors.iter().copied().max_by_key(|&(r, g, b)| r as u32 + g as u32 + b as u32).map_or([1.0; 3], |c| rgb(c).map(|v| 0.5 + 0.5 * v)))
            })
            .collect();
        let tick = world.tick();
        let parts: Vec<(ChunkPos, Texels)> = chunks
            .par_iter()
            .filter_map(|&cp| {
                let chunk = world.chunk(cp)?;
                let mut out = vec![(0.0f32, 0.0f32, [0.0f32; 3], Clear::default(), None, (0.0f32, [0.0f32; 3]), (0.0f32, [0.0f32; 3])); per_chunk * per_chunk];
                let (cells, bg) = (chunk.cells(), chunk.background());
                let o = cp.origin();
                for ly in 0..CHUNK {
                    let ambient = climate.ambient(o.x, o.y + ly);
                    for lx in 0..CHUNK {
                        let i = (ly * CHUNK + lx) as usize;
                        let (c, b) = (cells[i], bg[i]);
                        let slot = &mut out[(ly / t) as usize * per_chunk + (lx / t) as usize];
                        let (mut op, mut sky_op) = (0.0, 0.0);
                        if !c.is_air() {
                            let ph = mats.phys(c.material);
                            op = ph.opacity as f32 / 255.0;
                            if let Some(g) = glows[c.material.0 as usize] {
                                slot.5.0 += 1.0;
                                slot.5.1 = g;
                            }
                            if let Some(l) = lusters[c.material.0 as usize] {
                                slot.6.0 += 1.0;
                                slot.6.1 = l;
                            }
                            // (A face of it open to the air glints in any
                            // light: metal in ore, a gem. Buried, never.)
                            if let Some(col) = sparkles[c.material.0 as usize] {
                                let open = |dx: i32, dy: i32| {
                                    let (x, y) = (lx + dx, ly + dy);
                                    (0..CHUNK).contains(&x) && (0..CHUNK).contains(&y) && {
                                        let n = cells[(y * CHUNK + x) as usize];
                                        n.is_air() || !matches!(mats.phys(n.material).kind, Kind::Static | Kind::Powder)
                                    }
                                };
                                if open(1, 0) || open(-1, 0) || open(0, 1) || open(0, -1) {
                                    let roll = platypus_sim::rng::hash(&[(o.x + lx) as u64, (o.y + ly) as u64, tick, 0x5BA4]);
                                    if slot.4.is_none_or(|(_, _, r, _)| roll > r) {
                                        slot.4 = Some((CellPos::new(o.x + lx, o.y + ly), col, roll, true));
                                    }
                                }
                            }
                            if let Some(opt) = optics[c.material.0 as usize] {
                                slot.3.add(opt);
                                // (A glint: a clear cell, a liquid only at its
                                // surface; one at random, the highest roll.)
                                let surface = ph.kind != Kind::Liquid || (ly + 1 < CHUNK && cells[((ly + 1) * CHUNK + lx) as usize].is_air());
                                if surface {
                                    let roll = platypus_sim::rng::hash(&[(o.x + lx) as u64, (o.y + ly) as u64, tick]);
                                    if slot.4.is_none_or(|(_, _, r, _)| roll > r) {
                                        let tint = [opt.tint.0, opt.tint.1, opt.tint.2].map(|v| 0.55 + 0.45 * v as f32 / 255.0);
                                        slot.4 = Some((CellPos::new(o.x + lx, o.y + ly), tint, roll, false));
                                    }
                                }
                            }
                            sky_op = op;
                            let mut e = [ph.glow[0] as f32 / 255.0, ph.glow[1] as f32 / 255.0, ph.glow[2] as f32 / 255.0];
                            if ph.glow != [0; 3] {
                                let def = mats.def(c.material);
                                let mut k = 1.0;
                                if def.pulse > 0 {
                                    k *= 1.0 - def.pulse as f32 / 255.0 * breath(o.x + lx, o.y + ly);
                                }
                                if def.shimmer > 0 {
                                    k *= 1.0 - def.shimmer as f32 / 255.0 * (1.0 - glimmer(o.x + lx, o.y + ly));
                                }
                                e = e.map(|v| v * k);
                            }
                            if ph.kind == Kind::Fire || c.flags & flags::BURNING != 0 {
                                let f = flicker(o.x + lx, o.y + ly);
                                let fire = if c.flags & flags::BURNING != 0 && mats.is_charred(c) { 0.35 } else { 0.8 };
                                e = max3(e, [fire * f, fire * 0.55 * f, fire * 0.2 * f]);
                            }
                            e = max3(e, heat_glow(ambient + c.heat as i32));
                            add3(&mut slot.2, e);
                        }
                        if !b.is_air() {
                            let bp = mats.phys(b.material);
                            // Trees barely shade (a forest by day is bright,
                            // as in Terraria); walls behind rock block the sky.
                            let tree = bp.kind == Kind::Plant || bp.flammability > 0;
                            let k = if tree { 0.012 } else { 0.85 };
                            sky_op = f32::max(sky_op, bp.opacity as f32 / 255.0 * k);
                            if b.flags & flags::BURNING != 0 {
                                let f = flicker(o.x + lx, o.y + ly);
                                add3(&mut slot.2, [0.6 * f, 0.33 * f, 0.1 * f]);
                            }
                            // Glowing things behind glow too (a giant
                            // mushroom's cap), where nothing's in front.
                            if c.is_air() && bp.glow != [0; 3] {
                                let k = 1.0 - mats.def(b.material).pulse as f32 / 255.0 * breath(o.x + lx, o.y + ly);
                                add3(&mut slot.2, [bp.glow[0] as f32 / 255.0 * k, bp.glow[1] as f32 / 255.0 * k, bp.glow[2] as f32 / 255.0 * k]);
                            }
                        }
                        slot.0 += op;
                        slot.1 += sky_op;
                    }
                }
                for s in &mut out {
                    s.0 /= area;
                    s.1 /= area;
                    for ch in &mut s.2 {
                        *ch /= area;
                    }
                    s.3.settle(area);
                }
                Some((cp, out))
            })
            .collect();
        for (cp, out) in parts {
            let o = cp.origin();
            for ty in 0..per_chunk {
                for tx in 0..per_chunk {
                    let p = CellPos::new(o.x + tx as i32 * t, o.y + ty as i32 * t);
                    if let Some(i) = self.index_of(p) {
                        let (op, sky, e, clear, glint, phos, lus) = out[ty * per_chunk + tx];
                        self.glint[i] = glint.map(|(p, c, _, any)| (p, c, any));
                        self.phosphor[i] = (phos.0 >= area * 0.25).then_some(phos.1);
                        self.luster[i] = (lus.0 > 0.0).then_some((lus.1, lus.0 / area));
                        self.opacity[i] = op;
                        self.sky_opacity[i] = sky;
                        self.emit[i] = e.map(|v| v.min(1.5));
                        if clear.n > 0.0 {
                            self.ior[i] = clear.ior;
                            self.disperse[i] = clear.disperse;
                            self.reflect[i] = clear.reflect;
                            self.tint[i] = clear.tint;
                            self.optical = true;
                        }
                    }
                }
            }
        }
    }

    /// Light from far away (the sun, the moon, a patch of sky) travelling
    /// along `dir` (unit, downward), entering the grid wherever it's open to
    /// the sky: the top of every column in `open_top`, and the side it comes
    /// from down to where that edge column meets the ground. Dimmed by what
    /// it passes (crowns barely, the ground completely), so it casts shadows
    /// that lean with the sun.
    pub fn seed_directional(&mut self, dir: [f32; 2], color: Rgb, open_top: &[bool]) {
        let (w, h, t) = (self.w, self.h, self.texel as f32);
        if dir[1] >= 0.0 || color.iter().all(|&c| c <= 0.0) {
            return;
        }
        // Step one texel along the main axis per step.
        let steep = dir[1].abs() >= dir[0].abs();
        let (sx, sy) = if steep { (dir[0] / dir[1].abs(), -1.0) } else { (dir[0].signum(), dir[1] / dir[0].abs()) };
        let step_cells = (sx * sx + sy * sy).sqrt() * t;
        // What passes one step through a texel, by opacity (256 levels).
        let lut: Vec<f32> = (0..=255).map(|k| (1.0 - k as f32 / 255.0).powf(step_cells)).collect();
        // Where each column's open sky ends, going down from the top.
        let floor: Vec<Option<usize>> = (0..w)
            .map(|x| {
                if !open_top[x] {
                    return None;
                }
                let solid = (0..h).rev().find(|&y| self.sky_opacity[y * w + x] > 0.5);
                Some(solid.map_or(0, |y| y + 1))
            })
            .collect();
        // The rays lie on a lattice fixed in the world (one a texel across
        // them), not on the grid: the grid follows the camera, and rays
        // started from its top row crossed the world a fraction of a texel
        // elsewhere each time it moved, the sun's edge on a slope or a crown
        // flickering as you walk. A ray starts on the lattice where it's in
        // its start texel: steep, `X + sx·Y` is a half; shallow, `Y − k·X`
        // is (texels, k the rise a texel across). X and Y are counted from
        // a corner of `LATTICE` texels, not the world's: the sun creeps
        // across the sky, and its creep times a whole world's rows would
        // sweep the rays over everything every frame.
        let (ox, oy) = ((self.origin.x / self.texel).rem_euclid(LATTICE) as f64, (self.origin.y / self.texel).rem_euclid(LATTICE) as f64);
        let (sx64, k) = (sx as f64, (sy * sx.signum()) as f64);
        let start = |x: usize, y: usize| -> (f32, f32) {
            if steep {
                let phase = (0.5 - sx64 * (oy + y as f64 + 0.5)).rem_euclid(1.0);
                (x as f32 + phase as f32, y as f32 + 0.5)
            } else {
                let phase = (0.5 + k * (ox + x as f64 + 0.5)).rem_euclid(1.0);
                (x as f32 + 0.5, y as f32 + phase as f32)
            }
        };
        let mut starts: Vec<(f32, f32)> = (0..w).filter(|&x| open_top[x]).map(|x| start(x, h - 1)).collect();
        let edge = if dir[0] > 0.0 { 0 } else { w - 1 };
        if dir[0] != 0.0
            && let Some(f) = floor[edge]
        {
            starts.extend((f..h).map(|y| start(edge, y)));
        }
        for (x, y) in starts {
            self.trace(Ray { x, y, dx: sx, dy: sy, v: color, s: 0, steps: i32::MAX, n: 1.0, chan: None, bounces: 0, edge: 1.0, bent: false }, Lit::Sun, &lut);
        }
    }

    /// A point light at `at` (cells).
    pub fn seed_point(&mut self, at: [f32; 2], color: Rgb) {
        if let Some(i) = self.index_of(CellPos::from_world(at[0], at[1])) {
            max_into(&mut self.seed[i], color);
        }
    }

    /// A glowing point at `at`: it lights its surroundings and hazes over
    /// the dark, as a glowing cell does (a firefly).
    pub fn seed_emit(&mut self, at: [f32; 2], color: Rgb) {
        if let Some(i) = self.index_of(CellPos::from_world(at[0], at[1])) {
            max_into(&mut self.emit[i], color);
        }
    }

    /// A cone of light from `from` toward `dir` (unit), `half_angle` radians
    /// either side, `range` cells: rays marched texel by texel, stopped by
    /// what they pass through, so it throws hard shadows.
    pub fn seed_beam(&mut self, from: [f32; 2], dir: [f32; 2], half_angle: f32, range: f32, color: Rgb) {
        let t = self.texel as f32;
        // Enough rays that neighbouring ones are well under a texel apart at
        // the end (no gaps for the steps to show through).
        let rays = ((2.0 * half_angle * range / t) * 3.0).ceil().max(3.0) as i32;
        let base = dir[1].atan2(dir[0]);
        let steps = (range / (t * 0.5)) as i32;
        let lut: Vec<f32> = (0..=255).map(|k| (1.0 - k as f32 / 255.0).powf(t * 0.5)).collect();
        // (In texels from the grid's corner.)
        let (fx, fy) = ((from[0] - self.origin.x as f32) / t, (from[1] - self.origin.y as f32) / t);
        for r in 0..=rays {
            let k = r as f32 / rays as f32 * 2.0 - 1.0; // -1 … 1 across the cone
            let a = base + k * half_angle;
            let (dx, dy) = (a.cos() * 0.5, a.sin() * 0.5);
            // Full in the middle, fading smoothly off over its outer 40 % (a
            // hard rim shows the grid's steps).
            let edge = {
                let u = ((1.0 - k.abs()) / BEAM_RIM).min(1.0);
                u * u * (3.0 - 2.0 * u)
            };
            // (In whatever the lamp is in: a lamp under water starts in it.)
            let n = self.index_of(CellPos::from_world(from[0], from[1])).map_or(1.0, |i| self.ior[i]);
            self.trace(Ray { x: fx + dx, y: fy + dy, dx, dy, v: color, s: 1, steps, n, chan: None, bounces: 0, edge, bent: false }, Lit::Beam, &lut);
        }
    }

    /// The index a ray carrying `chan` (or all colours) meets at texel `i`.
    fn index_at(&self, i: usize, chan: Option<usize>) -> f32 {
        let n = self.ior[i];
        match chan {
            Some(c) => n + (n - 1.0) * self.disperse[i] * [-1.0, 0.0, 1.0][c],
            None => n,
        }
    }

    /// The face's normal at texel (`x`, `y`), facing back along `d`: from how
    /// the index changes over the 5×5 texels round it (a least-squares
    /// slope: a 3×3 one flipped between flat, steep and diagonal on a
    /// sloping face, and the rays leaving it went off in steps); none to
    /// tell, head on.
    fn normal_at(&self, x: usize, y: usize, d: (f32, f32)) -> (f32, f32) {
        let (w, h) = (self.w as i32, self.h as i32);
        let (mut gx, mut gy) = (0.0f32, 0.0f32);
        for dy in -2..=2i32 {
            for dx in -2..=2i32 {
                let (px, py) = ((x as i32 + dx).clamp(0, w - 1), (y as i32 + dy).clamp(0, h - 1));
                let n = self.ior[(py * w + px) as usize];
                gx += n * dx as f32;
                gy += n * dy as f32;
            }
        }
        let len = (gx * gx + gy * gy).sqrt();
        if len < 1e-4 {
            return (-d.0, -d.1);
        }
        let (nx, ny) = (-gx / len, -gy / len);
        if nx * d.0 + ny * d.1 > 0.0 { (-nx, -ny) } else { (nx, ny) }
    }

    /// A ray, and every ray its faces make of it (mirrored off them, split by
    /// colour going into what splits light), marched to their ends: lighting
    /// as they go (the sun seeds what's spread; a beam lights directly,
    /// fading over its range, and scatters a share off what it lands on),
    /// dimmed by what they pass (`pass`: kept per step by opacity, 256
    /// levels), tinted by what's clear. At a face between two indices a ray
    /// bends (Snell), a share mirroring back (Fresnel: more at a grazing
    /// angle; at least what the material's `reflect` says), or all of it if
    /// it can't get out (total internal reflection).
    fn trace(&mut self, first: Ray, lit: Lit, pass: &[f32]) {
        let (w, h, t) = (self.w, self.h, self.texel as f32);
        let mut stack = vec![first];
        let mut budget = RAY_STEPS_MOST;
        while let Some(mut r) = stack.pop() {
            let step = (r.dx * r.dx + r.dy * r.dy).sqrt();
            let tint_k = step * t / platypus_sim::Optics::TINT_DEPTH;
            loop {
                if r.x < 0.0 || r.y < 0.0 || r.x >= w as f32 || r.y >= h as f32 || r.s >= r.steps {
                    break;
                }
                budget -= 1;
                if budget == 0 {
                    return;
                }
                let (xi, yi) = (r.x as usize, r.y as usize);
                let i = yi * w + xi;
                if self.optical {
                    let n2 = self.index_at(i, r.chan);
                    if (n2 - r.n).abs() > 1e-3 {
                        // (Into what splits light, white: on as its colours,
                        // each meeting this face its own way.)
                        if r.chan.is_none() && n2 > r.n && self.disperse[i] > 0.0 {
                            for c in 0..3 {
                                if r.v[c] > 0.0 {
                                    let mut v = [0.0; 3];
                                    v[c] = r.v[c];
                                    stack.push(Ray { v, chan: Some(c), bent: true, ..r });
                                }
                            }
                            break;
                        }
                        let d = (r.dx / step, r.dy / step);
                        let nrm = self.normal_at(xi, yi, d);
                        let cosi = -(d.0 * nrm.0 + d.1 * nrm.1);
                        let eta = r.n / n2;
                        let r0 = ((r.n - n2) / (r.n + n2)).powi(2);
                        let back = (r.x - r.dx, r.y - r.dy);
                        let was = (back.0.max(0.0) as usize).min(w - 1) + (back.1.max(0.0) as usize).min(h - 1) * w;
                        let mirrored = (r0 + (1.0 - r0) * (1.0 - cosi).clamp(0.0, 1.0).powi(5)).max(self.reflect[i].max(self.reflect[was]));
                        let m = (d.0 + 2.0 * cosi * nrm.0, d.1 + 2.0 * cosi * nrm.1);
                        let k = 1.0 - eta * eta * (1.0 - cosi * cosi);
                        if k < 0.0 {
                            // (Can't get out: all of it back in.)
                            r.bounces += 1;
                            if r.bounces > BOUNCES_MOST {
                                break;
                            }
                            (r.x, r.y, r.dx, r.dy) = (back.0, back.1, m.0 * step, m.1 * step);
                            r.x += r.dx;
                            r.y += r.dy;
                            continue;
                        }
                        if r.bounces < BOUNCES_MOST && mirrored * r.v.iter().copied().fold(0.0, f32::max) > 0.01 {
                            stack.push(Ray { x: back.0 + m.0 * step, y: back.1 + m.1 * step, dx: m.0 * step, dy: m.1 * step, v: r.v.map(|c| c * mirrored), bounces: r.bounces + 1, bent: true, ..r });
                        }
                        r.v = r.v.map(|c| c * (1.0 - mirrored));
                        let s = eta * cosi - k.sqrt();
                        let (bx, by) = (eta * d.0 + s * nrm.0, eta * d.1 + s * nrm.1);
                        let bl = (bx * bx + by * by).sqrt().max(1e-6);
                        (r.dx, r.dy) = (bx / bl * step, by / bl * step);
                        r.n = n2;
                        r.bent = true;
                    }
                }
                match lit {
                    Lit::Sun => max_into(&mut self.seed[i], r.v),
                    Lit::Beam => {
                        let d = r.s as f32 / r.steps as f32;
                        let fall = (1.0 - d) * (1.0 - d);
                        let v = r.v.map(|c| c * fall * r.edge);
                        max_into(&mut self.direct[i], v);
                        // It scatters off what it lands on, not off the air.
                        if self.opacity[i] > 0.05 {
                            max_into(&mut self.seed[i], v.map(|c| c * BEAM_SCATTER * self.opacity[i].min(1.0)));
                        }
                    }
                }
                let op = if lit == Lit::Sun { self.sky_opacity[i] } else { self.opacity[i] };
                // (Bent light landing on a face (not clear itself): a caustic.)
                if r.bent && op > CAUSTIC_ON && self.ior[i] <= 1.0 {
                    let k = if lit == Lit::Sun { CAUSTIC_SUN } else { CAUSTIC_BEAM * r.edge };
                    max_into(&mut self.caustic[i], r.v.map(|c| c * k));
                }
                let keep = pass[(op.clamp(0.0, 1.0) * 255.0) as usize];
                r.v = r.v.map(|c| c * keep);
                if self.optical && self.ior[i] > 1.0 {
                    let tint = self.tint[i];
                    for (c, v) in r.v.iter_mut().enumerate() {
                        *v *= tint[c].powf(tint_k);
                    }
                }
                if r.v[0] + r.v[1] + r.v[2] < if lit == Lit::Sun { 0.003 } else { 0.01 } {
                    break;
                }
                r.x += r.dx;
                r.y += r.dy;
                r.s += 1;
            }
        }
    }

    /// Spread light and glow (the chamfer sweeps), the two in parallel.
    /// (The grid's outermost texels don't take part: they're off screen.)
    pub fn solve(&mut self, p: Params) {
        let t = self.texel;
        let air = p.air_falloff.powi(t);
        let air_d = air.powf(std::f32::consts::SQRT_2);
        // What leaves a texel through it, by opacity (walls pass almost
        // nothing on): a table, since opacity comes in 256 steps.
        let lut: Vec<(f32, f32)> = (0..=255)
            .map(|k| {
                let f = air * (1.0 - k as f32 / 255.0).powi(t);
                (f, f.powf(std::f32::consts::SQRT_2))
            })
            .collect();
        let (through, through_d): (Vec<f32>, Vec<f32>) = self.opacity.iter().map(|&o| lut[(o.clamp(0.0, 1.0) * 255.0) as usize]).unzip();
        // Or straight off it, if it glows (a burning log lights the room).
        let (w, h) = (self.w, self.h);
        let chan = |v: &[Rgb], ch: usize, k: f32| -> Vec<f32> { v.iter().map(|e| e[ch] * k).collect() };
        let (ft, fdt) = (transpose(&through, w, h), transpose(&through_d, w, h));
        let (emit, seed) = (&self.emit, &self.seed);
        // Six jobs: light and glow (the haze: glow only, one pass) per channel.
        let results: Vec<(usize, bool, Vec<f32>)> = (0..6)
            .into_par_iter()
            .map(|job| {
                let (ch, glow) = (job / 2, job % 2 == 1);
                let (e, ed) = (chan(emit, ch, air), chan(emit, ch, air_d));
                let (et, edt) = (transpose(&e, w, h), transpose(&ed, w, h));
                let m = Media { f: &through, fd: &through_d, e: &e, ed: &ed };
                let mt = Media { f: &ft, fd: &fdt, e: &et, ed: &edt };
                let mut buf: Vec<f32> = if glow {
                    emit.iter().map(|v| v[ch]).collect()
                } else {
                    seed.iter().zip(emit).map(|(s, v)| s[ch].max(v[ch])).collect()
                };
                spread(&mut buf, &m, &mt, w, h, if glow { 1 } else { p.iterations });
                (ch, glow, buf)
            })
            .collect();
        // The rim: light creeping into rock from its lit faces, shown only
        // on solid texels.
        let rim_f = air * p.rim.clamp(0.0, 1.0).powi(t);
        let solid: Vec<bool> = self.opacity.iter().map(|&o| o > 0.35).collect();
        let rf: Vec<f32> = solid.iter().zip(&through).map(|(&s, &f)| if s { rim_f } else { f }).collect();
        let rfd: Vec<f32> = rf.iter().map(|f| f.powf(std::f32::consts::SQRT_2)).collect();
        let (rft, rfdt) = (transpose(&rf, w, h), transpose(&rfd, w, h));
        let zeros = vec![0.0; w * h];
        let results: Vec<(usize, bool, Vec<f32>)> = results
            .into_par_iter()
            .map(|(ch, glow, mut buf)| {
                if !glow && p.rim > 0.0 {
                    let mut rim = buf.clone();
                    let m = Media { f: &rf, fd: &rfd, e: &zeros, ed: &zeros };
                    let mt = Media { f: &rft, fd: &rfdt, e: &zeros, ed: &zeros };
                    spread(&mut rim, &m, &mt, w, h, 1);
                    for ((b, r), &s) in buf.iter_mut().zip(rim).zip(&solid) {
                        if s {
                            *b = b.max(r);
                        }
                    }
                }
                (ch, glow, buf)
            })
            .collect();
        let n = w * h;
        self.light = vec![[0.0; 3]; n];
        self.glow = vec![[0.0; 3]; n];
        for (ch, glow, buf) in results {
            let out = if glow { &mut self.glow } else { &mut self.light };
            for (o, v) in out.iter_mut().zip(buf) {
                o[ch] = v;
            }
        }
    }

    /// Phosphor drinks light and glows with it after: each phosphor texel's
    /// charge (carried from last frame's grid, world-anchored) rises toward
    /// the light falling on it beyond its own glow (last frame's spread
    /// light, this frame's beams and sky), in `PHOSPHOR_RISE` s, and in the
    /// dark fades over `PHOSPHOR_FADE` s; it glows as charged, lighting what's
    /// round it and hazing over the dark (`emit`). Before `solve`.
    pub fn charge_phosphor(&mut self, prev: Option<&Previous>, dt: f32) {
        let (w, h) = (self.w, self.h);
        let (rise, fade) = (1.0 - (-dt / PHOSPHOR_RISE).exp(), (-dt / PHOSPHOR_FADE).exp());
        let lit = |v: &Rgb| v.iter().copied().fold(0.0f32, f32::max);
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let Some(color) = self.phosphor[i] else { continue };
                // Last frame's charge and light here (none if it's new).
                let (was, light) = prev
                    .filter(|p| p.texel == self.texel)
                    .and_then(|p| {
                        let (px, py) = (x as i32 + (self.origin.x - p.origin.x) / self.texel, y as i32 + (self.origin.y - p.origin.y) / self.texel);
                        (px >= 0 && py >= 0 && (px as usize) < p.w && (py as usize) < p.h).then(|| {
                            let j = py as usize * p.w + px as usize;
                            (p.charge[j], lit(&p.light[j]))
                        })
                    })
                    .unwrap_or((0.0, 0.0));
                let own = was * PHOSPHOR_GLOW * lit(&color);
                let falling = (light - own).max(0.0).max(lit(&self.direct[i])).max(lit(&self.seed[i]));
                let want = (falling * PHOSPHOR_GAIN).min(1.0);
                let c = if want > was { was + (want - was) * rise } else { was * fade };
                self.charge[i] = c;
                if c > 0.01 {
                    max_into(&mut self.emit[i], color.map(|v| v * c * PHOSPHOR_GLOW));
                }
            }
        }
    }

    /// Luster shines as light falls on it (last frame's, where it is, and
    /// this frame's beams and seeds): in its colour, as bright as the light
    /// on it times `LUSTER_GAIN` (under 1: its own shine never keeps it
    /// shining), nothing in the dark.
    pub fn shine(&mut self, prev: Option<&Previous>) {
        let (w, h) = (self.w, self.h);
        let lit = |v: &Rgb| v.iter().copied().fold(0.0f32, f32::max);
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let Some((color, share)) = self.luster[i] else { continue };
                let was = prev
                    .filter(|p| p.texel == self.texel)
                    .and_then(|p| {
                        let (px, py) = (x as i32 + (self.origin.x - p.origin.x) / self.texel, y as i32 + (self.origin.y - p.origin.y) / self.texel);
                        (px >= 0 && py >= 0 && (px as usize) < p.w && (py as usize) < p.h).then(|| lit(&p.light[py as usize * p.w + px as usize]))
                    })
                    .unwrap_or(0.0);
                let falling = was.max(lit(&self.direct[i])).max(lit(&self.seed[i])).min(1.5);
                if falling > 0.02 {
                    let k = falling * LUSTER_GAIN * (share * 2.0).min(1.0);
                    max_into(&mut self.emit[i], color.map(|v| v * k));
                }
            }
        }
    }

    /// Glints this frame: where a glinting cell catches direct light (a
    /// beam) or the sky's, by chance (`dt` seconds' worth: brighter, more
    /// often), a sparkle there in its colour, as bright as the light.
    pub fn glints(&self, dt: f32, seed: u64) -> Vec<(CellPos, Rgb)> {
        let mut out = Vec::new();
        for (i, g) in self.glint.iter().enumerate() {
            let Some((at, color, any)) = g else { continue };
            let lit = |v: &Rgb| v.iter().copied().fold(0.0f32, f32::max);
            let b = lit(&self.direct[i]).max(lit(&self.seed[i])).max(if *any { lit(&self.light[i]) * SPARKLE_GAIN } else { 0.0 });
            if b < GLINT_FROM {
                continue;
            }
            let chance = ((b - GLINT_FROM) * dt * GLINT_RATE).min(0.5);
            let roll = platypus_sim::rng::hash(&[i as u64, seed]) as f64 / u64::MAX as f64;
            if roll < chance as f64 {
                out.push((*at, color.map(|c| c * b.min(1.6))));
            }
        }
        out
    }

    /// The beams' direct light over what's spread, once that's eased (a
    /// beam is sharp: eased, a moving one left a fading trail behind it, a
    /// falling player's flashlight smeared up the screen). Kept: the light
    /// before it, for the next frame to ease from.
    pub fn add_direct(&mut self) -> (Vec<Rgb>, Vec<Rgb>) {
        let spread = self.light.clone();
        for (l, d) in self.light.iter_mut().zip(&self.direct) {
            max_into(l, *d);
        }
        // (Caustics go over the glow as it's drawn, at full strength, not
        // the haze's share: `compute_light`.)
        (spread, self.glow.clone())
    }
}

/// What was shown last frame, for easing into this one.
pub struct Previous {
    pub origin: CellPos,
    pub texel: i32,
    pub w: usize,
    pub h: usize,
    pub light: Vec<Rgb>,
    pub glow: Vec<Rgb>,
    /// Phosphor's charge per texel.
    pub charge: Vec<f32>,
}

impl LightGrid {
    /// Ease from last frame's light toward this one's: `rise` and `fall` are
    /// the shares of the way covered this frame when brightening / darkening.
    /// Moving smoke, flames and embers change the light every frame; eased,
    /// fire glows and flickers instead of strobing. (Texels new to the view
    /// take their value as is.)
    pub fn ease_from(&mut self, prev: &Previous, rise: f32, fall: f32) {
        if prev.texel != self.texel {
            return;
        }
        let (dx, dy) = ((self.origin.x - prev.origin.x) / self.texel, (self.origin.y - prev.origin.y) / self.texel);
        let ease = |now: &mut Rgb, was: Rgb| {
            for ch in 0..3 {
                let k = if now[ch] > was[ch] { rise } else { fall };
                now[ch] = was[ch] + (now[ch] - was[ch]) * k;
            }
        };
        for y in 0..self.h {
            let py = y as i32 + dy;
            if py < 0 || py >= prev.h as i32 {
                continue;
            }
            for x in 0..self.w {
                let px = x as i32 + dx;
                if px < 0 || px >= prev.w as i32 {
                    continue;
                }
                let (i, j) = (y * self.w + x, py as usize * prev.w + px as usize);
                ease(&mut self.light[i], prev.light[j]);
                ease(&mut self.glow[i], prev.glow[j]);
            }
        }
    }

    /// What to ease from next frame: `light` the spread light and `glow`
    /// the glow before caustics (`add_direct`'s); and phosphor's charge.
    pub fn into_previous(self, (light, glow): (Vec<Rgb>, Vec<Rgb>)) -> Previous {
        Previous { origin: self.origin, texel: self.texel, w: self.w, h: self.h, light, glow, charge: self.charge }
    }
}

/// Sun rays are laid out from a corner every this many texels (see
/// `seed_directional`).
const LATTICE: i32 = 64;

/// Share of a beam's light that scatters off what it hits into the room.
/// A beam's rim, fading out: this share of its half width.
const BEAM_RIM: f32 = 0.4;
const BEAM_SCATTER: f32 = 0.35;

/// Caustics: on a face at least this opaque, and this share of the sun's
/// bent light (it adds over daylight) or a beam's.
const CAUSTIC_ON: f32 = 0.2;
const CAUSTIC_SUN: f32 = 1.0;
const CAUSTIC_BEAM: f32 = 1.0;

/// Phosphor: seconds to charge (toward the light on it), seconds to fade in
/// the dark, how much light makes a full charge (×), how bright it glows.
const PHOSPHOR_RISE: f32 = 1.5;
const PHOSPHOR_FADE: f32 = 20.0;
const PHOSPHOR_GAIN: f32 = 1.6;
const PHOSPHOR_GLOW: f32 = 0.55;

/// Light (on a glinting cell) from which it glints, and how often then
/// (per second, per unit of light over that).
const GLINT_FROM: f32 = 0.3;
/// A sparkling face (metal, gems) counts the light spread on it at this:
/// a torch's or a lantern's glints it as a beam does.
const SPARKLE_GAIN: f32 = 1.6;
/// Luster's shine against the light falling on it (under 1).
const LUSTER_GAIN: f32 = 0.7;
const GLINT_RATE: f32 = 9.0;

/// Steps one traced ray and all it splits into may take, at most.
const RAY_STEPS_MOST: u32 = 4_000;
/// Faces a ray is mirrored off before it's let go.
const BOUNCES_MOST: u8 = 3;

/// A ray of light being traced (texels from the grid's corner; a step's
/// worth of texels per step).
#[derive(Clone, Copy)]
struct Ray {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    v: Rgb,
    /// Steps taken, and the most (a beam fades over its range).
    s: i32,
    steps: i32,
    /// The index of what it's in (1: air); the one colour it carries, once
    /// split (red, green, violet); faces it's been mirrored off.
    n: f32,
    chan: Option<usize>,
    bounces: u8,
    /// A beam's fade at its rim (1 in its middle).
    edge: f32,
    /// Bent, mirrored or split by something clear on its way (its light
    /// where it lands is a caustic).
    bent: bool,
}

/// Whose ray: the sky's (seeds the spread light) or a beam's (lights
/// directly).
#[derive(Clone, Copy, PartialEq)]
enum Lit {
    Sun,
    Beam,
}

/// A texel's clear cells, summed, then averaged (`settle`): what light
/// meets there if at least half the texel is clear.
#[derive(Clone, Copy, Default)]
struct Clear {
    n: f32,
    ior: f32,
    disperse: f32,
    reflect: f32,
    tint: Rgb,
}

impl Clear {
    fn add(&mut self, o: platypus_sim::Optics) {
        self.n += 1.0;
        self.ior += o.ior;
        self.disperse += o.disperse;
        self.reflect += o.reflect;
        for (c, v) in self.tint.iter_mut().zip([o.tint.0, o.tint.1, o.tint.2]) {
            *c += v as f32 / 255.0;
        }
    }

    fn settle(&mut self, area: f32) {
        if self.n < area * 0.5 {
            *self = Clear::default();
            return;
        }
        let k = 1.0 / self.n;
        self.ior *= k;
        self.disperse *= k;
        self.reflect *= k;
        self.tint = self.tint.map(|c| (c * k).max(0.001));
    }
}

/// Spread one channel: sweeps up, down, then (on transposed copies) right
/// and left. Each sweep takes, for every texel, the best of the three
/// texels behind it (straight and the two diagonals), so together they
/// cover all eight directions; and within a row every texel is independent,
/// so the inner loop vectorizes. `f`/`fd`: what passes on through a texel
/// straight / diagonally; `e`/`ed`: what leaves it by glowing.
struct Media<'a> {
    f: &'a [f32],
    fd: &'a [f32],
    e: &'a [f32],
    ed: &'a [f32],
}

fn spread(buf: &mut Vec<f32>, m: &Media, mt: &Media, w: usize, h: usize, iterations: usize) {
    for _ in 0..iterations {
        sweep_rows(buf, m, w, h, false);
        sweep_rows(buf, m, w, h, true);
        let mut t = transpose(buf, w, h);
        sweep_rows(&mut t, mt, h, w, false);
        sweep_rows(&mut t, mt, h, w, true);
        *buf = transpose(&t, h, w);
    }
}

/// Row by row, bottom to top (or top to bottom): each texel takes what
/// leaves the three texels in the row before it.
fn sweep_rows(buf: &mut [f32], m: &Media, w: usize, h: usize, down: bool) {
    if w < 3 || h < 2 {
        return;
    }
    let rows: Box<dyn Iterator<Item = usize>> = if down { Box::new((0..h - 1).rev()) } else { Box::new(1..h) };
    for y in rows {
        let prev_row = if down { y + 1 } else { y - 1 };
        let (p, c) = (prev_row * w, y * w);
        // Equal-length slices, so the loop has no bounds checks.
        let n = w - 2;
        let (bp, cur) = if down {
            let (lo, hi) = buf.split_at_mut(p);
            (&hi[..w], &mut lo[c..c + w])
        } else {
            let (lo, hi) = buf.split_at_mut(c);
            (&lo[p..p + w], &mut hi[..w])
        };
        let cur = &mut cur[1..1 + n];
        let (b0, bl, br) = (&bp[1..1 + n], &bp[..n], &bp[2..2 + n]);
        let (f0, fl, fr) = (&m.f[p + 1..p + 1 + n], &m.fd[p..p + n], &m.fd[p + 2..p + 2 + n]);
        let (e0, el, er) = (&m.e[p + 1..p + 1 + n], &m.ed[p..p + n], &m.ed[p + 2..p + 2 + n]);
        for x in 0..n {
            let straight = (b0[x] * f0[x]).max(e0[x]);
            let left = (bl[x] * fl[x]).max(el[x]);
            let right = (br[x] * fr[x]).max(er[x]);
            cur[x] = cur[x].max(straight).max(left.max(right));
        }
    }
}

fn transpose(v: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut out = vec![0.0; w * h];
    // In tiles, to stay in cache.
    const T: usize = 32;
    for y0 in (0..h).step_by(T) {
        for x0 in (0..w).step_by(T) {
            for y in y0..(y0 + T).min(h) {
                for x in x0..(x0 + T).min(w) {
                    out[x * h + y] = v[y * w + x];
                }
            }
        }
    }
    out
}

/// Incandescence as light: dull red from ~480 °C to yellow-white.
fn heat_glow(celsius: i32) -> Rgb {
    if celsius <= 480 {
        return [0.0; 3];
    }
    let k = ((celsius - 480) as f32 / 1000.0).clamp(0.0, 1.0);
    [0.25 + 0.75 * k, 0.05 + 0.55 * k * k, 0.02 + 0.3 * k * k * k]
}

#[inline]
fn max3(a: Rgb, b: Rgb) -> Rgb {
    [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])]
}

#[inline]
fn add3(a: &mut Rgb, b: Rgb) {
    a[0] += b[0];
    a[1] += b[1];
    a[2] += b[2];
}

#[inline]
fn max_into(a: &mut Rgb, b: Rgb) {
    *a = max3(*a, b);
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: Params = Params { air_falloff: 0.955, iterations: 2, rim: 0.0 };

    /// A 60×40-texel grid of 1-cell texels: `walls` are opaque rectangles.
    fn grid(walls: &[(usize, usize, usize, usize)]) -> LightGrid {
        let mut g = LightGrid::new(CellPos::new(0, 0), 60, 40, 1);
        for &(x0, y0, x1, y1) in walls {
            for y in y0..y1 {
                for x in x0..x1 {
                    g.opacity[y * 60 + x] = 0.95;
                    g.sky_opacity[y * 60 + x] = 0.95;
                }
            }
        }
        g
    }

    fn at(g: &LightGrid, x: usize, y: usize) -> f32 {
        g.light[y * g.w + x][0]
    }

    #[test]
    fn light_falls_off_with_distance_and_round() {
        let mut g = grid(&[]);
        g.seed_point([30.5, 20.5], [1.0; 3]);
        g.solve(P);
        g.add_direct();
        assert!((at(&g, 30, 20) - 1.0).abs() < 1e-5);
        assert!(at(&g, 40, 20) < at(&g, 35, 20) && at(&g, 35, 20) < 1.0, "falls off");
        // Round, not a diamond: 10 cells out diagonally ≈ 10 cells straight out.
        let (straight, diag) = (at(&g, 40, 20), at(&g, 37, 27));
        assert!((straight - diag).abs() < 0.08, "straight {straight} vs diagonal {diag}");
    }

    #[test]
    fn walls_are_lit_on_their_face_and_dark_behind() {
        let mut g = grid(&[(35, 0, 38, 40)]);
        g.seed_point([30.5, 20.5], [1.0; 3]);
        g.solve(P);
        g.add_direct();
        assert!(at(&g, 35, 20) > 0.6, "the wall's face is lit ({})", at(&g, 35, 20));
        assert!(at(&g, 40, 20) < 0.01, "behind it is dark ({})", at(&g, 40, 20));
    }

    #[test]
    fn a_lit_rock_face_shows_a_few_cells_deep_but_no_light_comes_through() {
        let mut g = grid(&[(35, 0, 41, 40)]);
        g.seed_point([30.5, 20.5], [1.0; 3]);
        g.solve(Params { rim: 0.8, ..P });
        let face = |x| at(&g, x, 20);
        assert!(face(37) > 0.2, "two cells into the rock it's still lit ({})", face(37));
        assert!(face(40) < face(37), "fading");
        assert!(at(&g, 44, 20) < 0.01, "the air behind the wall stays dark ({})", at(&g, 44, 20));
    }

    #[test]
    fn light_eases_between_frames_and_follows_the_view() {
        let mut a = grid(&[]);
        a.seed_point([30.5, 20.5], [1.0; 3]);
        a.solve(P);
        let a_spread = a.add_direct();
        let prev = a.into_previous(a_spread);
        // Next frame: the light is out, and the view moved 3 texels right.
        let mut b = LightGrid::new(CellPos::new(3, 0), 60, 40, 1);
        b.solve(P);
        b.add_direct();
        b.ease_from(&prev, 0.5, 0.25);
        // World cell (30, 20) is now texel (27, 20): it dims, not snaps off.
        let v = b.light[20 * 60 + 27][0];
        assert!((v - 0.75).abs() < 1e-4, "eased toward dark: {v}");
    }

    #[test]
    fn a_glowing_solid_lights_the_room() {
        // A burning log: opaque, but it glows.
        let mut g = grid(&[(28, 18, 32, 20)]);
        for x in 28..32 {
            g.emit[18 * 60 + x] = [0.8, 0.4, 0.1];
        }
        g.solve(P);
        g.add_direct();
        assert!(at(&g, 30, 24) > 0.4, "above it is lit ({})", at(&g, 30, 24));
        assert!(g.glow[24 * 60 + 30][0] > 0.4, "and it glows");
    }

    #[test]
    fn sky_lights_open_columns_not_under_a_roof_and_spills_in_a_little() {
        let mut g = grid(&[(0, 20, 30, 24)]);
        g.seed_directional([0.0, -1.0], [1.0; 3], &[true; 60]);
        g.solve(P);
        g.add_direct();
        assert!(at(&g, 45, 5) > 0.95, "open ground is in daylight");
        // Daylight spills in under the edge and fades going deeper.
        let (edge, mid, deep) = (at(&g, 28, 10), at(&g, 20, 10), at(&g, 5, 10));
        assert!(edge > mid && mid > deep, "{edge} > {mid} > {deep}");
        // A rock face keeps a thin lit rim; two cells into the rock it's dark.
        assert!(deep < 0.4 && at(&g, 10, 22) < 0.1, "deep in is dim ({deep}), inside the roof dark");
    }

    #[test]
    fn a_low_sun_casts_a_long_slanted_shadow() {
        // A post on open ground; the sun low in the east (light going left and down).
        let mut g = grid(&[(0, 0, 60, 5), (40, 5, 42, 15)]);
        let d = [-0.9f32, -0.44];
        let n = (d[0] * d[0] + d[1] * d[1]).sqrt();
        g.seed_directional([d[0] / n, d[1] / n], [1.0; 3], &[true; 60]);
        let lit = |x: usize, y: usize| g.seed[y * 60 + x][0];
        assert!(lit(30, 6) < 0.05, "shadow on the ground west of the post");
        assert!(lit(41, 16) > 0.9 && lit(45, 6) > 0.9, "the top of the post and the ground east of it in sun");
        assert!(lit(20, 6) > 0.9, "the shadow ends");
    }

    #[test]
    fn sunlight_stays_put_as_the_view_moves() {
        // A slope with an overhang, in world cells; the view (the grid) moved
        // a texel at a time about it. The sun must light the same world texels
        // the same way whichever texel the grid starts on, or shadow edges
        // shimmer as the camera moves (climbing, walking past a crown).
        let solid = |x: i32, y: i32| y < x / 2 || ((50..58).contains(&x) && (44..47).contains(&y));
        for dir in [[-0.5f32, -0.87], [0.3, -0.95], [-0.9, -0.44], [0.85, -0.53]] {
            let n = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
            let dir = [dir[0] / n, dir[1] / n];
            let lit = |ox: i32, oy: i32| {
                let mut g = LightGrid::new(CellPos::new(ox, oy), 60, 40, 1);
                for y in 0..40 {
                    for x in 0..60 {
                        if solid(ox + x as i32, oy + y as i32) {
                            g.opacity[y * 60 + x] = 0.95;
                            g.sky_opacity[y * 60 + x] = 0.95;
                        }
                    }
                }
                let open: Vec<bool> = (0..60).map(|x| (0..100).all(|d| !solid(ox + x, oy + 40 + d))).collect();
                g.seed_directional(dir, [1.0; 3], &open);
                g
            };
            let base = lit(40, 20);
            // The sun creeps: a frame's worth (and more) moves an edge a
            // texel at most, here and there, and doesn't sweep the rays.
            let creep = |d: [f32; 2], a: f32| [d[0] * a.cos() - d[1] * a.sin(), d[0] * a.sin() + d[1] * a.cos()];
            for (ox, oy) in [(40, 20), (30_000, 12_600)] {
                let at = |d: [f32; 2]| {
                    let mut g = LightGrid::new(CellPos::new(ox, oy), 60, 40, 1);
                    let open = vec![true; 60];
                    for y in 0..40 {
                        for x in 0..60 {
                            if solid(40 + x as i32, 20 + y as i32) {
                                g.opacity[y * 60 + x] = 0.95;
                                g.sky_opacity[y * 60 + x] = 0.95;
                            }
                        }
                    }
                    g.seed_directional(d, [1.0; 3], &open);
                    g
                };
                let (a, b) = (at(dir), at(creep(dir, 2e-4)));
                let flips = (0..60 * 40).filter(|&i| (a.seed[i][0] - b.seed[i][0]).abs() > 0.2).count();
                assert!(flips <= 4, "sun {dir:?} at ({ox}, {oy}): a creep of the sun flips {flips} texels");
            }
            for (dx, dy) in [(1, 0), (0, 1), (0, -1), (3, 2), (-2, 5)] {
                let moved = lit(40 + dx, 20 + dy);
                // (Away from the grid's edges, where rays enter.)
                for y in 12..28 {
                    for x in 12..48 {
                        let (wx, wy) = (40 + x, 20 + y);
                        let (mx, my) = ((wx - 40 - dx) as usize, (wy - 20 - dy) as usize);
                        let (a, b) = (base.seed[y as usize * 60 + x as usize][0], moved.seed[my * 60 + mx][0]);
                        assert!((a - b).abs() < 0.02, "sun {dir:?}, view moved ({dx}, {dy}): world ({wx}, {wy}) lit {a} then {b}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_beam_lights_ahead_throws_shadows_and_not_behind() {
        let beam = |walls: &[(usize, usize, usize, usize)]| {
            let mut g = grid(walls);
            g.seed_beam([20.5, 20.5], [1.0, 0.0], 0.35, 50.0, [1.0; 3]);
            g.solve(Params { iterations: 1, ..P });
            g.add_direct();
            g
        };
        let (open, g) = (beam(&[]), beam(&[(40, 18, 42, 22)]));
        assert!(at(&g, 30, 20) > 0.5, "ahead is lit");
        let (shade, lit) = (at(&g, 45, 20), at(&open, 45, 20));
        assert!(shade < lit * 0.6, "a pillar casts a shadow ({shade} vs {lit} without it)");
        assert!(at(&open, 10, 20) < 0.01, "behind is dark ({})", at(&open, 10, 20));
    }
}
