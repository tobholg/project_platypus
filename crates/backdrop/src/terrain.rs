//! The surface's ranges as terrain: a heightfield seen from the front, a
//! band of it each layer, drawn column by column front to back (each column
//! on its own: tiles side by side join up). Not colours: what each pixel
//! is, for the light to colour as the day goes (`backdrop.wgsl` in the
//! game, `shade` here for stills) — which way its face turns, rock, snow or
//! scrub, a crest or an edge against what's behind (they catch the light),
//! a rock's dark stroke, how high up the slope.
//!
//! A texel (RGBA8): its normal's x and y (0..255 for -1..1; z, toward the
//! viewer, follows); bits: material (2), crest (1), stroke (1), height up
//! the range (4); alpha 255 where there's land, 0 sky.

use crate::{Rgb, hash, mix, noise1, noise2};

/// A range's shape (all in the strip's units).
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    /// About how far apart its summits (the noise's scale).
    pub scale: f32,
    /// Its tallest, over its feet.
    pub amp: f32,
    /// Where the range sinks along its length, this share of `amp` at most.
    pub low: f32,
    /// The band's depth (how far back it runs), and how much higher its
    /// back shows than its front (units up a unit back: looking a little
    /// down on it).
    pub deep: f32,
    pub tilt: f32,
    /// Snow from this share of its height down (0: none).
    pub snow: f32,
    /// 0 smooth rolling hills .. 1 sharp eroded ridges.
    pub rough: f32,
    /// Below this share of its height, a lake (flat, showing the sky).
    pub water: f32,
    /// How much of it is forest (0..1): clumps of growth on its slopes.
    pub trees: f32,
    /// A treeline: tree crowns about this big (units; 0 none), domes
    /// standing close all over the band (as many as `trees` says), their
    /// tops a bumpy canopy, each lit as a ball of leaves.
    pub crowns: f32,
    /// Flowers and small mushrooms scattered over its grass (0 none .. 1).
    pub decor: f32,
    pub seed: u64,
}

/// What a texel is: its layer's ground (rock, grass), snow, growth (scrub,
/// forest), water.
pub const ROCK: u8 = 0;
pub const SNOW: u8 = 1;
pub const SCRUB: u8 = 2;
pub const WATER: u8 = 3;

/// A texel, read back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Texel {
    /// Its face's normal (x across, y up, z away from the viewer: what
    /// faces you has z < 0).
    pub n: [f32; 3],
    pub mat: u8,
    /// A crest (an arête, a spur's back) or an edge against what's behind.
    pub crest: bool,
    pub stroke: bool,
    /// How high up the range (0 its feet .. 1 its tallest).
    pub up: f32,
}

/// A decoration's colour (`decor`): a flower white or yellow, a mushroom's
/// cap (red, its lit top), its stem.
pub const FLOWER: u8 = 0;
pub const BLOSSOM: u8 = 1;
pub const CAP: u8 = 2;
pub const CAP_LIT: u8 = 3;
pub const STEM: u8 = 4;
pub const DECOR: [Rgb; 5] = [[0.95, 0.96, 0.92], [0.98, 0.86, 0.36], [0.78, 0.22, 0.16], [0.95, 0.45, 0.32], [0.9, 0.84, 0.72]];

/// A decoration's texel (both crest and stroke: nothing else is both; its
/// colour in `up`).
pub fn decor(i: u8) -> Texel {
    Texel { n: [0.0, 0.7, -0.714], mat: ROCK, crest: true, stroke: true, up: i as f32 / 15.0 }
}

/// Which decoration a texel is, if it is one.
pub fn decor_of(t: &Texel) -> Option<u8> {
    (t.crest && t.stroke).then(|| (t.up * 15.0).round() as u8)
}

pub fn encode(t: &Texel) -> [u8; 4] {
    let b = |v: f32| ((v.clamp(-1.0, 1.0) * 0.5 + 0.5) * 255.0).round() as u8;
    let up = (t.up.clamp(0.0, 1.0) * 15.0).round() as u8;
    [b(t.n[0]), b(t.n[1]), (t.mat & 3) | (t.crest as u8) << 2 | (t.stroke as u8) << 3 | up << 4, 255]
}

pub fn decode(p: &[u8]) -> Option<Texel> {
    if p[3] == 0 {
        return None;
    }
    let f = |v: u8| v as f32 / 255.0 * 2.0 - 1.0;
    let (x, y) = (f(p[0]), f(p[1]));
    let z = -(1.0 - x * x - y * y).max(0.0).sqrt();
    Some(Texel { n: [x, y, z], mat: p[2] & 3, crest: p[2] & 4 != 0, stroke: p[2] & 8 != 0, up: (p[2] >> 4) as f32 / 15.0 })
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smooth rolling hills: a few octaves, gentle, rounded on top. About 0..1.
fn rolling(x: f32, z: f32, seed: u64) -> f32 {
    let (mut total, mut freq, mut amp, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..4 {
        total += noise2(x * freq + o as f32 * 17.3, z * freq + o as f32 * 53.1, seed.wrapping_add(o as u64 * 104_729) ^ 0x5300) * amp;
        norm += amp;
        freq *= 2.1;
        amp *= 0.42;
    }
    let v = total / norm;
    // (Rounded: the tops swell, the valleys broaden.)
    smoothstep(0.2, 0.85, v)
}

/// Ridged multifractal (Musgrave): sharp crests, each octave weighted by
/// the last, so the detail gathers on the ridges and the valleys stay
/// smooth. About 0..1.
fn ridged(x: f32, z: f32, octaves: u32, seed: u64) -> f32 {
    let (mut total, mut weight, mut freq, mut amp) = (0.0, 1.0, 1.0, 1.0);
    for o in 0..octaves {
        let n = noise2(x * freq + o as f32 * 37.1, z * freq + o as f32 * 91.7, seed.wrapping_add(o as u64 * 7919)) * 2.0 - 1.0;
        let s = (1.0 - n.abs()).powi(2) * weight;
        weight = (s * 1.6).clamp(0.0, 1.0);
        total += s * amp;
        freq *= 2.05;
        amp *= 0.5;
    }
    total / 1.35
}

impl Shape {
    /// Its height at `x` across, `z` back into the band.
    pub fn height(&self, x: f32, z: f32) -> f32 {
        let s = self.scale;
        // (Warped, so the ranges wander; swelling and sinking along the
        // way: a massif here, a saddle there; rising toward the back.)
        let wx = x + 0.35 * s * (noise2(x / (s * 2.0), z / (s * 2.0), self.seed ^ 0xa1) - 0.5);
        let wz = z + 0.35 * s * (noise2(x / (s * 2.0) + 7.3, z / (s * 2.0), self.seed ^ 0xa2) - 0.5);
        // (Sharp ridges, or smooth rolling hills, or between.)
        let r = if self.rough >= 1.0 {
            ridged(wx / s, wz / s, 6, self.seed)
        } else {
            let smooth = rolling(wx / s, wz / s, self.seed);
            if self.rough <= 0.0 { smooth } else { smooth + (ridged(wx / s, wz / s, 6, self.seed) - smooth) * self.rough }
        };
        let e = noise1(x / (s * 3.2), self.seed ^ 0xa3);
        let env = self.low + (1.0 - self.low) * e * e * (3.0 - 2.0 * e);
        // (Rising toward its back: a range shows its far summits. Smooth
        // hills less: their near ones show too.)
        let front = 0.75 - 0.5 * self.rough.clamp(0.0, 1.0);
        let back = front + (1.0 - front) * smoothstep(0.0, 1.0, z / self.deep);
        self.amp * env * r * back + self.canopy(x, z)
    }

    /// The tree crowns' height over the ground at (x, z) (0 where there's
    /// none): domes on a jittered grid, each its own size, lifted on its
    /// trunk.
    fn canopy(&self, x: f32, z: f32) -> f32 {
        let (crowns, floor, _) = self.crowns_at(x, z);
        crowns.max(floor)
    }

    /// The crowns' height at (x, z), the wood's floor under them (its
    /// undergrowth: bushes, in clumps), and the tree whose crown that is
    /// (its middle across, its size), if one is. Each crown a low dome on
    /// its trunk over the floor, a few bunches of leaves on its top (a
    /// bumpy outline, light and shade within it); gently rounded at its
    /// rim (a sphere's would stand steep there, blocky from the front).
    fn crowns_at(&self, x: f32, z: f32) -> (f32, f32, Option<Crown>) {
        let r = self.crowns;
        if r <= 0.0 {
            return (0.0, 0.0, None);
        }
        // (Its edge a slope, rising over a few trees' width: seen from the
        // front, bushes row on row, not a wall.)
        let edge = smoothstep(-r * 2.0, r * 0.6, z);
        let floor = r * 0.9 * self.trees * edge;
        let cell = r * 1.15;
        let (cx, cz) = ((x / cell).floor() as i64, (z / cell).floor() as i64);
        let (mut best, mut tree) = (0.0f32, None);
        for i in -1..=1 {
            for j in -1..=1 {
                let (gx, gz) = (cx + i, cz + j);
                let s = self.seed ^ 0x7ee;
                if hash(gx, gz, s) > self.trees {
                    continue;
                }
                let px = (gx as f32 + 0.15 + 0.7 * hash(gx, gz, s ^ 1)) * cell;
                let pz = (gz as f32 + 0.15 + 0.7 * hash(gx, gz, s ^ 2)) * cell;
                // (No tree before the wood's edge.)
                if pz < 0.0 {
                    continue;
                }
                let ri = r * (0.6 + 0.8 * hash(gx, gz, s ^ 3));
                // (Lifted on its trunk: under it, the trunk, the shade, the
                // undergrowth, drawn in `strip`.)
                let lift = floor + ri * CROWN_LIFT;
                let u2 = ((x - px).powi(2) + (z - pz).powi(2)) / (ri * ri);
                // (Its dome: round, as a ball's top.)
                let (dome, dk) = if u2 < 1.0 { (ri * 0.9 * (1.0 - u2).sqrt(), -0.9 / (1.0 - u2).max(0.06).sqrt()) } else { (0.0, 0.0) };
                let ds = [dk * (x - px) / ri, dk * (z - pz) / ri];
                if u2 < 1.0 && dome + lift > best {
                    (best, tree) = (dome + lift, Some(Crown { px, ri, slope: ds }));
                }
                // Its bunches of leaves, bumps on it (on its rim, a lobe:
                // its outline bumpy all round, not a wall).
                for b in 0..4u64 {
                    let a = (hash(gx, gz, s ^ (10 + b)) + b as f32 * 0.25) * std::f32::consts::TAU;
                    let (bx, bz) = (px + a.cos() * ri * 0.55, pz + a.sin() * ri * 0.45);
                    let rb = ri * (0.38 + 0.2 * hash(gx, gz, s ^ (20 + b)));
                    let ub = ((x - bx).powi(2) + (z - bz).powi(2)) / (rb * rb);
                    if ub < 1.0 {
                        let h = rb * 0.75 * (1.0 - ub * ub) + dome * 0.9 + lift;
                        if h > best {
                            let k = -3.0 * ub;
                            let slope = [k * (x - bx) / rb + ds[0] * 0.9, k * (z - bz) / rb + ds[1] * 0.9];
                            (best, tree) = (h, Some(Crown { px, ri, slope }));
                        }
                    }
                }
            }
        }
        // (Under the crowns, a canopy's floor: no gaps through the wood;
        // bushes in clumps on it, the undergrowth.)
        (best, floor + self.undergrowth(x, z) * edge, tree)
    }

    /// The tree whose crown is at (x, z), if one is: its middle across, its
    /// size.
    fn tree_at(&self, x: f32, z: f32) -> Option<Crown> {
        let (crowns, floor, tree) = self.crowns_at(x, z);
        if crowns > floor + 0.3 { tree } else { None }
    }

    /// The wood's undergrowth over its floor at (x, z): bushes, small
    /// domes, in clumps (thin in places).
    fn undergrowth(&self, x: f32, z: f32) -> f32 {
        let r = self.crowns * 0.42;
        let cell = r * 1.2;
        let (cx, cz) = ((x / cell).floor() as i64, (z / cell).floor() as i64);
        let s = self.seed ^ 0xb054;
        let mut best = 0.0f32;
        for i in -1..=1 {
            for j in -1..=1 {
                let (gx, gz) = (cx + i, cz + j);
                // (Thick in clumps, thin between.)
                let px = (gx as f32 + 0.1 + 0.8 * hash(gx, gz, s ^ 1)) * cell;
                let pz = (gz as f32 + 0.1 + 0.8 * hash(gx, gz, s ^ 2)) * cell;
                if hash(gx, gz, s) > 0.35 + 0.55 * noise2(px / 30.0, pz / 20.0, s ^ 5) {
                    continue;
                }
                let ri = r * (0.55 + 0.7 * hash(gx, gz, s ^ 3));
                let d2 = (x - px).powi(2) + (z - pz).powi(2);
                if d2 < ri * ri {
                    best = best.max((ri * ri - d2).sqrt());
                }
            }
        }
        best
    }

    /// What the land is at (x, z), its height `t` there.
    fn texel(&self, x: f32, z: f32, t: f32) -> Texel {
        // (Its face over a few units: broad calm faces, not every bump's.)
        let e = 1.6;
        let hx = (self.height(x + e, z) - self.height(x - e, z)) / (2.0 * e);
        let hz = (self.height(x, z + e) - self.height(x, z - e)) / (2.0 * e);
        let len = (hx * hx + 1.0 + hz * hz).sqrt();
        let n = [-hx / len, 1.0 / len, -hz / len];
        let steep = 1.0 - n[1];
        let up = (t / self.amp.max(1e-3)).clamp(0.0, 1.0);
        // (A snowline wandering broadly, not pixel by pixel; only what's too
        // steep to hold it bare.)
        let line = 1.0 - self.snow + 0.18 * (noise2(x / 26.0, z / 18.0, self.seed ^ 0xb1) - 0.5);
        let snowy = self.snow > 0.0 && up > line && steep < 0.68;
        // Forest in clumps, on what isn't too steep, thinning up high.
        // (Laid out where they show, across and up the view: into the band
        // they were streaks, by height columns.)
        let row = t + z * self.tilt;
        let clump = noise2(x / 18.0, row / 5.0, self.seed ^ 0xc1) * 0.82 + noise2(x / 5.0, row / 2.0, self.seed ^ 0xc2) * 0.18;
        let wooded = self.trees > 0.0 && steep < 0.55 && clump > 1.0 - self.trees * (1.1 - up * 0.6);
        // (A crown's leaves; under them, the wood's dark floor: its ground,
        // its undergrowth, a trunk here and there.)
        let (crowns, floor, tree) = self.crowns_at(x, z);
        let crown = self.crowns > 0.0 && crowns > floor + 0.3;
        let under = self.crowns > 0.0 && !crown;
        // (A crown's face is its own, smooth, the bunch of leaves it's in;
        // its leaves ruffle it, a little light and shade in clumps.)
        let n = match tree.filter(|_| crown) {
            Some(c) => {
                let leaf = |k: u64| noise2(x / 1.9, row / 1.6, self.seed ^ k) - 0.5;
                let (sx, sz) = (c.slope[0] + leaf(0xe1) * 1.1, c.slope[1] + leaf(0xe2) * 0.8);
                normalize([-sx, 1.0, -sz])
            }
            None => n,
        };
        let mat = if snowy {
            SNOW
        } else if crown || wooded || (up < 0.12 && z < self.deep * 0.35) {
            SCRUB
        } else {
            ROCK
        };
        let r = 3.0;
        let around = 0.25 * (self.height(x + r, z) + self.height(x - r, z) + self.height(x, z + r) + self.height(x, z - r));
        let crest = t - around > 0.011 * self.amp;
        // (Strokes along the slope's height, not into the band: they
        // cluster across a face, not in columns.)
        let stroke = self.rough > 0.5 && !snowy && steep > 0.3 && noise2(x / 2.4, t / 3.2, self.seed ^ 0xb2) > 0.74 && noise2(x / 30.0, t / 24.0, self.seed ^ 0xb3) > 0.45;
        // (Under the crowns, dappled: deep shade here, a little light there,
        // by `up`, which shades it.)
        let up = if under { 0.15 + 0.6 * noise2(x / 7.0, row / 3.5, self.seed ^ 0xd4) } else { up };
        // (Never both a crest and a stroke: that's a decoration's mark.)
        let crest = crest && self.crowns <= 0.0;
        Texel { n, mat, crest, stroke: stroke && !crest, up }
    }
}

/// A tree, where its crown is: its middle across, its size; the slope of
/// its crown there (across, into the band: its own, not the wood's round
/// it, so each bunch of leaves is lit as itself).
#[derive(Clone, Copy)]
struct Crown {
    px: f32,
    ri: f32,
    slope: [f32; 2],
}

/// How high a crown's rim sits over the wood's floor (a share of its
/// size): the room under it, for its trunk.
const CROWN_LIFT: f32 = 0.75;

/// What shows `below` units under a crown's rim at `x` (the tree's middle
/// across `px`, its size `ri`; `row` units from the strip's top): the
/// crown's underside, curving away, dark; its trunk, bark lit on the
/// light's side; round it the wood's deep shade, a far trunk here and
/// there.
fn under_crown(shape: &Shape, x: f32, below: f32, row: f32, px: f32, ri: f32) -> Texel {
    let dx = x - px;
    let k = noise2(x / 4.0, row / 3.0, shape.seed ^ 0xd7);
    if below < ri * 0.22 + k * 0.8 {
        return Texel { n: normalize([dx / ri * 0.5, -0.5, -0.8]), mat: SCRUB, crest: false, stroke: false, up: 0.3 };
    }
    let tw = 0.55 + ri * 0.07;
    if dx.abs() < tw {
        // (Round: its face turns from the light across it.)
        let u = (dx / tw).clamp(-1.0, 1.0);
        return Texel { n: normalize([u * 0.85, 0.1, -0.6]), mat: SNOW, crest: false, stroke: noise2(x * 2.0, row / 1.5, shape.seed ^ 0xd8) > 0.78, up: 0.55 };
    }
    // (Far trunks, thin and dark, now and then.)
    let far = hash((x * 0.8).floor() as i64, 7, shape.seed ^ 0xd9) < 0.1;
    Texel { n: normalize([0.0, 0.15, -0.99]), mat: ROCK, crest: false, stroke: far, up: 0.05 + 0.3 * k }
}

/// The range's body below its feet: facing you, tilted up a little, its
/// lowest.
const BODY: Texel = Texel { n: [0.0, 0.45, -0.893], mat: SCRUB, crest: false, stroke: false, up: 0.0 };

/// Columns `x0 .. x0 + w` of a range's strip, `h` pixel rows, its feet
/// `foot` units down from the strip's top, `detail` pixels a unit: texels
/// (`encode`), clear above its skyline. Below its feet the front of the
/// band runs on down to the strip's bottom.
pub fn strip(shape: &Shape, x0: i64, w: usize, h: usize, foot: f32, detail: f32) -> Vec<u8> {
    let mut out = vec![0u8; w * h * 4];
    let dz = 0.6 / detail;
    for xi in 0..w {
        let xf = (x0 + xi as i64) as f32 / detail + 0.5 / detail;
        // Rows from `top` down are drawn.
        let mut top = h;
        // (A wood starts a little in front of its first trees: their fronts,
        // the undergrowth and trunks under them are drawn, not filled.)
        let mut z = -shape.crowns * 2.0;
        while z <= shape.deep && top > 0 {
            // (Under its water line, a lake: flat, its surface.)
            let land = shape.height(xf, z);
            let level = shape.water * shape.amp;
            let wet = shape.water > 0.0 && land < level;
            let t = if wet { level } else { land };
            let row = foot - (t + z * shape.tilt);
            let py = (row * detail).ceil().max(0.0) as usize;
            if py < top {
                let mut texel = if wet { Texel { n: [0.0, 1.0, 0.0], mat: WATER, crest: false, stroke: false, up: shape.water } } else { shape.texel(xf, z, t) };
                // (A crown's rim reached: the rows down to what was drawn
                // under it are what's under the crown.)
                let tree = if wet || shape.crowns <= 0.0 { None } else { shape.tree_at(xf, z) };
                for y in py..top {
                    // (Its top pixel stands against what's behind: an edge.
                    // The front's rows below its feet: the range's body, one
                    // even face toward you, not a column's face pulled down.)
                    let below = top == h && (y as f32) > (foot - t) * detail + 2.0;
                    // (The body's what its front is: a lake runs on as lake;
                    // smooth hills as their ground; a range as its scrub.)
                    let body = if wet {
                        texel
                    } else if shape.crowns > 0.0 {
                        // (A wood's lowest rows: a hedge of undergrowth in
                        // its shade, leaves in clumps, by where they show:
                        // no streaks, no flat band.)
                        let row = y as f32 / detail;
                        let k = noise2(xf / 6.0, row / 3.0, shape.seed ^ 0xd5);
                        let leaf = |s: u64| noise2(xf / 1.9, row / 1.6, shape.seed ^ s) - 0.5;
                        Texel { n: normalize([(k - 0.5) * 0.8 + leaf(0xe3) * 0.9, 0.45, -0.85 + leaf(0xe4) * 0.5]), mat: SCRUB, crest: false, stroke: false, up: 0.05 + 0.3 * k }
                    } else if shape.rough < 0.5 {
                        Texel { mat: ROCK, ..BODY }
                    } else {
                        BODY
                    };
                    let run = (top.min(h) - py).max(1) as f32;
                    // (Under a crown's rim: its underside, its trunk, the
                    // shade of the wood round it; its rim the crest.)
                    let under = tree.filter(|_| run > 4.0 && y > py);
                    // (A wood's crowns: an edge only where one stands over
                    // what's behind, not every row of its face.)
                    let rim = y == py && (shape.crowns <= 0.0 || run > 4.0);
                    let edge = if below {
                        body
                    } else if let Some(c) = under {
                        under_crown(shape, xf, (y - py) as f32 / detail, y as f32 / detail, c.px, c.ri)
                    } else if texel.crest || rim {
                        Texel { crest: true, stroke: false, ..texel }
                    } else {
                        texel
                    };
                    out[(y * w + xi) * 4..][..4].copy_from_slice(&encode(&edge));
                }
                texel.crest = false;
                top = py;
            }
            z += dz;
        }
        // Flowers and mushrooms over its grass, sparse (each where its
        // place says: tiles join).
        if shape.decor > 0.0 {
            let xa = x0 + xi as i64;
            for y in 0..h {
                let i = (y * w + xi) * 4;
                let Some(t) = decode(&out[i..i + 4]) else { continue };
                if t.mat == WATER || t.mat == SNOW || decor_of(&t).is_some() {
                    continue;
                }
                let yy = y as i64;
                // A mushroom: its cap a row over its stem (two tall), its
                // stem's foot at (ax, ay).
                let s = shape.seed ^ 0x3005;
                let mut put = None;
                for (dx, dy, what) in [(0, 0, STEM), (0, 1, STEM), (-1, 2, CAP), (0, 2, CAP_LIT), (1, 2, CAP)] {
                    let (ax, ay) = (xa - dx, yy + dy);
                    if hash(ax, ay, s) < 0.0004 * shape.decor {
                        put = Some(what);
                    }
                }
                // (Flowers in drifts, none between.)
                let drift = smoothstep(0.55, 0.85, noise2(xa as f32 / 40.0, yy as f32 / 10.0, shape.seed ^ 0xf12));
                if put.is_none() && hash(xa, yy, shape.seed ^ 0xf10) < 0.008 * shape.decor * drift {
                    put = Some(if hash(xa, yy, shape.seed ^ 0xf11) < 0.7 { FLOWER } else { BLOSSOM });
                }
                if let Some(what) = put {
                    out[i..i + 4].copy_from_slice(&encode(&decor(what)));
                }
            }
        }
    }
    out
}

/// The light of an hour, as the shading wants it (the game's overlay
/// grades the result by the sky's light after: these are relative).
#[derive(Clone, Copy, Debug)]
pub struct Light {
    /// Toward the sun or moon (x across, y up, z away: on the viewer's
    /// side, z < 0).
    pub to: [f32; 3],
    pub color: Rgb,
    pub ambient: Rgb,
    /// How bright a crest facing it lights up.
    pub rim: f32,
    /// What distance fades into: the sky's colour.
    pub haze: Rgb,
    /// How low and golden the sun is (`golden_night`): clouds' undersides
    /// glow.
    pub golden: f32,
}

/// What a layer is made of, and how it fades.
#[derive(Clone, Copy, Debug)]
pub struct Albedo {
    pub rock: Rgb,
    pub snow: Rgb,
    pub scrub: Rgb,
    /// Its share of the sky's haze, and the mist at its feet.
    pub haze: f32,
    pub mist: f32,
    /// How wide the dither where one tone meets the next (0..1): narrow on
    /// smooth hills, whose gentle faces spread it wide.
    pub dither: f32,
}

/// The light on a face, stepped (a few flat tones, dithered only where
/// one meets the next: `dither` 0..1, the texel's ordered threshold).
pub fn lambert(t: &Texel, l: &Light, dither: f32, width: f32) -> f32 {
    let lam = (t.n[0] * l.to[0] + t.n[1] * l.to[1] + t.n[2] * l.to[2]).max(0.0);
    ((lam * 5.0 + 0.5 + 0.5 * width * (dither - 0.5)).floor() / 5.0).min(1.0)
}

/// A texel's colour in this light (`mist`: how far into its feet' mist,
/// 0..1). As `backdrop.wgsl` does it.
pub fn shade(t: &Texel, a: &Albedo, l: &Light, dither: f32, mist: f32) -> Rgb {
    // Water: the sky's colour, a little deeper, a faint line or two.
    if t.mat == WATER {
        let c = mix(l.haze.map(|v| v * 0.92), [0.2, 0.32, 0.42], 0.22);
        return mix(c, l.haze, a.haze + (1.0 - a.haze) * a.mist * mist * 0.7);
    }
    if let Some(d) = decor_of(t) {
        let c = DECOR[(d as usize).min(DECOR.len() - 1)];
        let lit = [0, 1, 2].map(|k| c[k] * (l.ambient[k] * 0.7 + l.color[k] * 0.55));
        return mix(lit, l.haze, a.haze);
    }
    let lam = lambert(t, l, dither, a.dither);
    let albedo = match t.mat {
        SNOW => a.snow,
        SCRUB => a.scrub,
        _ => a.rock,
    };
    // (The lower slopes darker: the light grazes up high.)
    let low = 0.62 + 0.38 * t.up;
    let mut c = [0, 1, 2].map(|k| albedo[k] * (l.ambient[k] + l.color[k] * lam * 0.95) * low);
    let raw = (t.n[0] * l.to[0] + t.n[1] * l.to[1] + t.n[2] * l.to[2]).max(0.0);
    if t.crest && raw > 0.3 {
        c = [0, 1, 2].map(|k| c[k] * (1.0 - 0.35 * l.rim) + l.color[k] * 0.75 * l.rim + 0.02);
    }
    if t.stroke {
        c = c.map(|v| v * 0.55);
    }
    mix(c, l.haze, a.haze + (1.0 - a.haze) * a.mist * mist * 0.7)
}

/// Where the sun (or, at night, the moon) is: the direction its light
/// travels (unit, downward; x across, y up) as its disc crosses the sky,
/// rising on the left, overhead at noon, setting on the right, never lower
/// than ~12° so it still reaches in. (The game's lighting's too.)
pub fn sun_dir(time: f32) -> [f32; 2] {
    // The sun's half of the day is 06:00–18:00, the moon's the other half.
    let phase = ((time - 0.25).rem_euclid(0.5)) / 0.5; // 0 rising … 1 setting
    let a = phase * std::f32::consts::PI;
    let (x, y) = (a.cos(), a.sin().max(0.2));
    let n = (x * x + y * y).sqrt();
    [x / n, -y / n]
}

/// The ranges' light at `time` (0..1, 0 midnight), fading into `haze`:
/// toward the sun by day, the moon by night (a little from the viewer's
/// side, so what faces you isn't all shade); white and soft at noon, warm
/// on the lit faces and violet in the shade near dawn and dusk (their
/// crests catching it), cool and silver by night. Relative: the game's
/// overlay grades it by the sky's light after.
pub fn hour(time: f32, haze: Rgb) -> Light {
    let sun = -(time * std::f32::consts::TAU).cos();
    let golden = (1.0 - ((sun - 0.02).abs() / 0.3).min(1.0)).powi(2);
    let night = smoothstep(0.05, -0.25, sun);
    let d = sun_dir(time);
    let ambient = mix(mix([0.44, 0.48, 0.62], [0.5, 0.44, 0.74], golden), [0.42, 0.48, 0.76], night);
    let color = mix(mix([0.86, 0.86, 0.84], [1.08, 0.86, 0.76], golden), [0.82, 0.92, 1.15], night);
    let rim = 0.35 + 0.65 * golden.max(night * 0.6);
    // (From the left a little always, as a painter would, the more the
    // higher it is: overhead at noon every face got the same.)
    let day = 1.0 - golden.max(night);
    // (And less from your side by day: lit from where you stand, every face
    // turned to you got the same.)
    Light { to: normalize([-d[0] - 0.3 - 0.6 * day, (-d[1]).max(0.12), -0.45 + 0.3 * day]), color, ambient, rim, haze, golden }
}

/// How golden the hour is (0 noon or night .. 1 the sun on the horizon)
/// and how far into night (0 by day .. 1).
pub fn golden_night(time: f32) -> (f32, f32) {
    let sun = -(time * std::f32::consts::TAU).cos();
    ((1.0 - ((sun - 0.02).abs() / 0.3).min(1.0)).powi(2), smoothstep(0.05, -0.25, sun))
}

/// The sky's colours at an hour, from its colour at the horizon (`base`:
/// the game's sky colour by the hour): overhead, at the horizon, the glow
/// round the sun or moon (and how strong), the far horizon's (the belt
/// opposite the sun near dawn and dusk), the sun side's.
#[derive(Clone, Copy, Debug)]
pub struct SkyLight {
    pub zenith: Rgb,
    pub horizon: Rgb,
    pub glow: Rgb,
    pub glow_strength: f32,
    pub belt: Rgb,
    pub warm: Rgb,
    pub golden: f32,
}

pub fn sky(time: f32, base: Rgb) -> SkyLight {
    let (golden, night) = golden_night(time);
    let horizon = base.map(|v| (v * 1.12).min(1.0));
    let zenith = [base[0] * 0.6, base[1] * 0.64, base[2] * 0.82];
    // (The sun's glow white by day, warm low; the moon's cold and faint.)
    let glow = mix(mix([1.0, 0.97, 0.88], [1.0, 0.7, 0.45], golden), [0.62, 0.7, 0.95], night);
    let glow_strength = 0.24 + 0.54 * golden - 0.1 * night;
    let belt = [0.62, 0.48, 0.72];
    let warm = [1.0, 0.62, 0.42];
    SkyLight { zenith, horizon, glow, glow_strength, belt, warm, golden }
}

/// The noon light (stills, the old tiles' look).
pub fn noon(haze: Rgb) -> Light {
    Light { to: normalize([-0.35, 0.85, -0.4]), color: [0.72, 0.72, 0.72], ambient: [0.58, 0.62, 0.74], rim: 0.35, haze, golden: 0.0 }
}

pub fn normalize(v: [f32; 3]) -> [f32; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / n, v[1] / n, v[2] / n]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape() -> Shape {
        Shape { scale: 110.0, amp: 120.0, low: 0.35, deep: 150.0, tilt: 0.2, snow: 0.4, rough: 1.0, water: 0.0, trees: 0.0, crowns: 0.0, decor: 0.0, seed: 5 }
    }

    #[test]
    fn strips_join_without_a_seam() {
        let (w, h) = (64, 200);
        let whole = strip(&shape(), 0, 2 * w, h, 120.0, 1.5);
        let right = strip(&shape(), w as i64, w, h, 120.0, 1.5);
        for y in 0..h {
            for x in 0..w {
                assert_eq!(&whole[(y * 2 * w + w + x) * 4..][..4], &right[(y * w + x) * 4..][..4], "({x}, {y})");
            }
        }
    }

    #[test]
    fn a_texel_reads_back_as_written() {
        let t = Texel { n: normalize([0.3, 0.8, -0.5]), mat: SNOW, crest: true, stroke: false, up: 0.6 };
        let d = decode(&encode(&t)).unwrap();
        assert!((d.n[0] - t.n[0]).abs() < 0.01 && (d.n[1] - t.n[1]).abs() < 0.01 && (d.n[2] - t.n[2]).abs() < 0.02);
        assert_eq!((d.mat, d.crest, d.stroke), (SNOW, true, false));
        assert!((d.up - 0.6).abs() < 0.04);
    }

    #[test]
    fn faces_turn_from_the_light_as_it_crosses() {
        // Across a strip, at dawn more faces turn to the light on the left
        // of their summits, at dusk on the right.
        let s = shape();
        let px = strip(&s, 0, 300, 220, 140.0, 1.0);
        let lit = |to: [f32; 3]| px.chunks(4).filter_map(decode).filter(|t| t.n[0] * to[0] > 0.2).count();
        assert!(lit([-1.0, 0.0, 0.0]) > 100 && lit([1.0, 0.0, 0.0]) > 100);
    }
}
