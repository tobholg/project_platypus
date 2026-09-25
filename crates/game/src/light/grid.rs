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
    pub fn fill_from(&mut self, world: &World, flicker: &(impl Fn(i32, i32) -> f32 + Sync)) {
        let t = self.texel;
        let per_chunk = (CHUNK / t) as usize;
        let x_end = self.origin.x + self.w as i32 * t;
        let y_end = self.origin.y + self.h as i32 * t;
        let (c0, c1) = (self.origin.chunk(), CellPos::new(x_end - 1, y_end - 1).chunk());
        let chunks: Vec<ChunkPos> = (c0.y..=c1.y).flat_map(|cy| (c0.x..=c1.x).map(move |cx| ChunkPos::new(cx, cy))).collect();
        let mats = world.materials();
        let climate = world.climate();
        let area = (t * t) as f32;
        // Per chunk, the texels it covers (each texel lies in one chunk).
        // (opacity, sky opacity, glow) per texel.
        type Texels = Vec<(f32, f32, Rgb)>;
        let parts: Vec<(ChunkPos, Texels)> = chunks
            .par_iter()
            .filter_map(|&cp| {
                let chunk = world.chunk(cp)?;
                let mut out = vec![(0.0f32, 0.0f32, [0.0f32; 3]); per_chunk * per_chunk];
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
                            sky_op = op;
                            let mut e = [ph.glow[0] as f32 / 255.0, ph.glow[1] as f32 / 255.0, ph.glow[2] as f32 / 255.0];
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
                                add3(&mut slot.2, [bp.glow[0] as f32 / 255.0, bp.glow[1] as f32 / 255.0, bp.glow[2] as f32 / 255.0]);
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
                        let (op, sky, e) = out[ty * per_chunk + tx];
                        self.opacity[i] = op;
                        self.sky_opacity[i] = sky;
                        self.emit[i] = e.map(|v| v.min(1.5));
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
        let mut starts: Vec<(f32, f32)> = (0..w).filter(|&x| open_top[x]).map(|x| (x as f32 + 0.5, h as f32 - 0.5)).collect();
        let edge = if dir[0] > 0.0 { 0 } else { w - 1 };
        if dir[0] != 0.0
            && let Some(f) = floor[edge]
        {
            starts.extend((f..h).map(|y| (edge as f32 + 0.5, y as f32 + 0.5)));
        }
        for (mut x, mut y) in starts {
            let mut v = color;
            loop {
                if x < 0.0 || y < 0.0 || x >= w as f32 || y >= h as f32 {
                    break;
                }
                let i = y as usize * w + x as usize;
                max_into(&mut self.seed[i], v);
                let pass = lut[(self.sky_opacity[i].clamp(0.0, 1.0) * 255.0) as usize];
                v = v.map(|c| c * pass);
                if v[0] + v[1] + v[2] < 0.003 {
                    break;
                }
                x += sx;
                y += sy;
            }
        }
    }

    /// A point light at `at` (cells).
    pub fn seed_point(&mut self, at: [f32; 2], color: Rgb) {
        if let Some(i) = self.index_of(CellPos::from_world(at[0], at[1])) {
            max_into(&mut self.seed[i], color);
        }
    }

    /// A cone of light from `from` toward `dir` (unit), `half_angle` radians
    /// either side, `range` cells: rays marched texel by texel, stopped by
    /// what they pass through, so it throws hard shadows.
    pub fn seed_beam(&mut self, from: [f32; 2], dir: [f32; 2], half_angle: f32, range: f32, color: Rgb) {
        let t = self.texel as f32;
        // Enough rays that neighbouring ones are under a texel apart at the end.
        let rays = ((2.0 * half_angle * range / t) * 1.5).ceil().max(3.0) as i32;
        let base = dir[1].atan2(dir[0]);
        for r in 0..=rays {
            let k = r as f32 / rays as f32 * 2.0 - 1.0; // -1 … 1 across the cone
            let a = base + k * half_angle;
            let (dx, dy) = (a.cos() * t * 0.5, a.sin() * t * 0.5);
            // Brightest in the middle, soft at the edges.
            let edge = 1.0 - k * k * k * k;
            let mut pass = 1.0f32;
            let steps = (range / (t * 0.5)) as i32;
            let (mut x, mut y) = (from[0], from[1]);
            for s in 0..steps {
                x += dx;
                y += dy;
                let Some(i) = self.index_of(CellPos::from_world(x, y)) else { break };
                let d = s as f32 / steps as f32;
                let fall = (1.0 - d) * (1.0 - d);
                let v = color.map(|c| c * pass * fall * edge);
                max_into(&mut self.direct[i], v);
                // It scatters off what it lands on, not off the air.
                if self.opacity[i] > 0.05 {
                    max_into(&mut self.seed[i], v.map(|c| c * BEAM_SCATTER * self.opacity[i].min(1.0)));
                }
                pass *= (1.0 - self.opacity[i]).max(0.0).powf(t * 0.5);
                if pass < 0.01 {
                    break;
                }
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
        for (l, d) in self.light.iter_mut().zip(&self.direct) {
            max_into(l, *d);
        }
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

    pub fn into_previous(self) -> Previous {
        Previous { origin: self.origin, texel: self.texel, w: self.w, h: self.h, light: self.light, glow: self.glow }
    }
}

/// Share of a beam's light that scatters off what it hits into the room.
const BEAM_SCATTER: f32 = 0.35;

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
        let prev = a.into_previous();
        // Next frame: the light is out, and the view moved 3 texels right.
        let mut b = LightGrid::new(CellPos::new(3, 0), 60, 40, 1);
        b.solve(P);
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
        assert!(at(&g, 30, 24) > 0.4, "above it is lit ({})", at(&g, 30, 24));
        assert!(g.glow[24 * 60 + 30][0] > 0.4, "and it glows");
    }

    #[test]
    fn sky_lights_open_columns_not_under_a_roof_and_spills_in_a_little() {
        let mut g = grid(&[(0, 20, 30, 24)]);
        g.seed_directional([0.0, -1.0], [1.0; 3], &[true; 60]);
        g.solve(P);
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
    fn a_beam_lights_ahead_throws_shadows_and_not_behind() {
        let beam = |walls: &[(usize, usize, usize, usize)]| {
            let mut g = grid(walls);
            g.seed_beam([20.5, 20.5], [1.0, 0.0], 0.35, 50.0, [1.0; 3]);
            g.solve(Params { iterations: 1, ..P });
            g
        };
        let (open, g) = (beam(&[]), beam(&[(40, 18, 42, 22)]));
        assert!(at(&g, 30, 20) > 0.5, "ahead is lit");
        let (shade, lit) = (at(&g, 45, 20), at(&open, 45, 20));
        assert!(shade < lit * 0.6, "a pillar casts a shadow ({shade} vs {lit} without it)");
        assert!(at(&open, 10, 20) < 0.01, "behind is dark ({})", at(&open, 10, 20));
    }
}
