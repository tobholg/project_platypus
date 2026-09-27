//! Parallax backgrounds, generated (Noita-like: dark, layered, dithered,
//! misty). A scene is a sky and layers from far to near (ridges, hills,
//! treelines, dunes, mesas, sea; underground: rock walls, cavern depths,
//! stalactites and columns, crystals, mushrooms, lava's glow), each drawn
//! from noise at a depth that sets its colour, its haze and (in the game)
//! how slowly it scrolls. `render` draws a scene as one still (the
//! concept sheets); the game draws its layers one by one.

use std::f32::consts::TAU;

pub type Rgb = [f32; 3];

// ---- noise ----

fn hash(a: i64, b: i64, seed: u64) -> f32 {
    let mut h = seed ^ (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (b as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^= h >> 33;
    (h >> 40) as f32 / (1u64 << 24) as f32
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Value noise in 0..1.
pub fn noise1(x: f32, seed: u64) -> f32 {
    let i = x.floor();
    let f = smooth(x - i);
    let (a, b) = (hash(i as i64, 0, seed), hash(i as i64 + 1, 0, seed));
    a + (b - a) * f
}

pub fn noise2(x: f32, y: f32, seed: u64) -> f32 {
    let (ix, iy) = (x.floor(), y.floor());
    let (fx, fy) = (smooth(x - ix), smooth(y - iy));
    let (ix, iy) = (ix as i64, iy as i64);
    let a = hash(ix, iy, seed) + (hash(ix + 1, iy, seed) - hash(ix, iy, seed)) * fx;
    let b = hash(ix, iy + 1, seed) + (hash(ix + 1, iy + 1, seed) - hash(ix, iy + 1, seed)) * fx;
    a + (b - a) * fy
}

pub fn fbm1(x: f32, octaves: u32, seed: u64) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += noise1(x * freq, seed.wrapping_add(o as u64 * 7919)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

pub fn fbm2(x: f32, y: f32, octaves: u32, seed: u64) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += noise2(x * freq, y * freq, seed.wrapping_add(o as u64 * 104_729)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// A 4×4 ordered dither threshold, 0..1.
fn bayer(x: usize, y: usize) -> f32 {
    const M: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    (M[y & 3][x & 3] as f32 + 0.5) / 16.0
}

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn scale(a: Rgb, k: f32) -> Rgb {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn add(a: Rgb, b: Rgb) -> Rgb {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn mul(a: Rgb, b: Rgb) -> Rgb {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}

pub fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

// ---- the canvas ----

/// Colours 0..1, y down.
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Rgb>,
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Self {
        Canvas { w, h, px: vec![[0.0; 3]; w * h] }
    }

    fn at(&mut self, x: usize, y: usize) -> &mut Rgb {
        &mut self.px[y * self.w + x]
    }

    /// Quantised to `levels` steps a channel, ordered-dithered (the pixel
    /// look: gradients in steps), as RGBA bytes.
    pub fn to_rgba(&self, levels: f32) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.w * self.h * 4);
        for y in 0..self.h {
            for x in 0..self.w {
                let c = self.px[y * self.w + x];
                let d = bayer(x, y) - 0.5;
                for v in c {
                    let q = ((v.clamp(0.0, 1.0) * levels + d).round() / levels).clamp(0.0, 1.0);
                    out.push((q * 255.0) as u8);
                }
                out.push(255);
            }
        }
        out
    }
}

// ---- time of day ----

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Time {
    Dawn,
    Day,
    Dusk,
    Night,
}

/// The sky and what it lights things with.
#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub top: Rgb,
    pub horizon: Rgb,
    /// The sun or moon's light on things (rims, lit faces).
    pub sun: Rgb,
    /// How bright the land is (its colours × this × `sun`-ish white).
    pub land: Rgb,
    /// The haze distance takes on.
    pub fog: Rgb,
    pub stars: f32,
    /// Where the sun or moon is (0..1 across, 0..1 down), and which.
    pub disc: (f32, f32),
    pub moon: bool,
}

pub fn light(t: Time) -> Light {
    match t {
        Time::Day => Light { top: rgb(70, 128, 206), horizon: rgb(176, 208, 236), sun: rgb(255, 244, 222), land: [1.0, 1.0, 0.97], fog: rgb(170, 198, 224), stars: 0.0, disc: (0.72, 0.16), moon: false },
        Time::Dawn => Light { top: rgb(62, 78, 140), horizon: rgb(240, 186, 150), sun: rgb(255, 200, 150), land: [0.6, 0.55, 0.62], fog: rgb(150, 130, 150), stars: 0.1, disc: (0.18, 0.62), moon: false },
        Time::Dusk => Light { top: rgb(38, 42, 96), horizon: rgb(246, 136, 84), sun: rgb(255, 150, 80), land: [0.42, 0.32, 0.4], fog: rgb(104, 72, 104), stars: 0.25, disc: (0.8, 0.66), moon: false },
        // (Night's haze a touch paler than the land: layers read as
        // silhouettes, each nearer one darker.)
        Time::Night => Light { top: rgb(2, 3, 8), horizon: rgb(10, 14, 28), sun: rgb(120, 140, 200), land: [0.07, 0.08, 0.13], fog: rgb(24, 30, 50), stars: 1.0, disc: (0.28, 0.2), moon: true },
    }
}

// ---- scenes ----

/// How a scene's layers are drawn (the three looks to choose between).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style {
    /// Flat silhouettes in steps of colour by depth.
    Flat,
    /// Noita-like: dithered texture in each layer, lit rims, mist in the
    /// valleys between layers.
    Noita,
    /// Heavier mist and light shafts, fewer colours, a vignette.
    Moody,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Trees {
    None,
    Pine,
    Round,
    Palm,
    Dead,
    /// Huge old trunks, a canopy closing overhead.
    Giant,
    Cactus,
}

#[derive(Clone, Debug)]
pub enum Kind {
    /// Jagged peaks; snow above `snow` (0..1 of the layer's height; 1:
    /// none).
    Ridge { snow: f32 },
    Hills,
    Dunes,
    /// Flat-topped buttes.
    Mesas,
    /// Open water to the horizon.
    Sea,
    // Underground:
    /// A rock face filling the view: strata, cracks.
    Wall,
    /// Hollows opening deeper into the rock (lighter, hazier).
    Depths,
    /// Stalactites from above, stalagmites from below, a column now and
    /// then.
    Teeth,
}

/// One layer: what, how far (0 the horizon .. 1 near), where its skyline
/// sits (0 top .. 1 bottom), how tall its relief is, what grows on it.
#[derive(Clone, Debug)]
pub struct Layer {
    pub kind: Kind,
    pub depth: f32,
    pub base: f32,
    pub height: f32,
    pub trees: Trees,
    pub color: Rgb,
}

/// Little lights in a scene (glowing crystals, mushrooms, a lava sea's
/// light from below, embers, fireflies).
#[derive(Clone, Debug, Default)]
pub struct Glows {
    pub crystals: Option<Rgb>,
    pub mushrooms: Option<Rgb>,
    /// Light rising from below (lava), its colour.
    pub below: Option<Rgb>,
    pub embers: bool,
    pub fireflies: bool,
    pub aurora: bool,
    /// Icicles (ice caves).
    pub icicles: bool,
    /// Still water at the bottom, reflecting.
    pub pool: bool,
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub name: String,
    /// Underground: no sky; the darkness and the rock's own colour.
    pub underground: Option<Rgb>,
    pub layers: Vec<Layer>,
    pub glows: Glows,
}

// ---- drawing ----

/// Where a layer's skyline is at column `x` (0..w), in rows from the top.
fn skyline(l: &Layer, i: usize, x: f32, w: f32, h: f32, seed: u64) -> f32 {
    let s = seed.wrapping_add(i as u64 * 31);
    let u = x / w;
    // (Nearer layers: finer, bigger features.)
    let f = 2.0 + 4.0 * (1.0 - l.depth);
    let rise = match l.kind {
        Kind::Ridge { .. } => {
            let r = fbm1(u * f, 5, s);
            let ridged = 1.0 - (2.0 * fbm1(u * f * 1.7 + 3.1, 4, s ^ 9) - 1.0).abs();
            (0.55 * r + 0.45 * ridged).powf(1.4)
        }
        Kind::Hills => fbm1(u * f * 0.6, 3, s),
        Kind::Dunes => {
            let p = (u * f * 1.3 + fbm1(u * 2.0, 2, s) * 0.8).fract();
            // (Steep on the lee side.)
            let d = if p < 0.7 { p / 0.7 } else { (1.0 - p) / 0.3 };
            0.25 + 0.55 * smooth(d) + 0.2 * fbm1(u * 9.0, 2, s ^ 5)
        }
        Kind::Mesas => {
            let n = fbm1(u * f * 0.8, 3, s);
            let step = if n > 0.55 { 1.0 } else if n > 0.45 { (n - 0.45) * 10.0 } else { 0.1 };
            step * (0.8 + 0.2 * fbm1(u * 30.0, 2, s ^ 3))
        }
        Kind::Sea => 0.0,
        Kind::Wall | Kind::Depths | Kind::Teeth => 0.0,
    };
    (l.base - l.height * rise) * h
}

/// Does tree `style`, standing at `root` (x, skyline y) and `size` tall,
/// cover pixel (px, py)?
fn tree_covers(style: Trees, px: f32, py: f32, root: (f32, f32), size: f32, seed: f32) -> bool {
    let (dx, up) = (px - root.0, root.1 - py);
    if up < -1.0 || up > size * 1.2 {
        return false;
    }
    match style {
        Trees::None => false,
        Trees::Pine => {
            // Tiers narrowing upward, a thin trunk below.
            if up < size * 0.12 {
                return dx.abs() < (size * 0.05).max(0.6);
            }
            let k = (up - size * 0.12) / (size * 0.88);
            if k > 1.0 {
                return false;
            }
            let tier = (k * 4.0).fract();
            let half = size * 0.28 * (1.0 - k) * (0.55 + 0.45 * (1.0 - tier));
            dx.abs() < half.max(0.5)
        }
        Trees::Round => {
            if up < size * 0.35 {
                return dx.abs() < (size * 0.06).max(0.6);
            }
            let c = (0.0, size * 0.68);
            let r = size * 0.36 * (0.9 + 0.2 * noise1(px * 0.3 + seed, 7));
            (dx - c.0).powi(2) + (up - c.1).powi(2) * 1.3 < r * r
        }
        Trees::Palm => {
            let lean = (seed.fract() - 0.5) * 0.5;
            let trunk_x = lean * up;
            if up < size * 0.9 && (dx - trunk_x).abs() < (size * 0.035).max(0.6) {
                return true;
            }
            // Fronds: arcs drooping from the crown.
            let (cx, cy) = (lean * size * 0.9, size * 0.9);
            let (fx, fy) = (dx - cx, up - cy);
            (fx.abs() < size * 0.45) && (fy - (-(fx * fx) / (size * 0.35))).abs() < (size * 0.05).max(0.7) + 0.02 * size * (1.0 - fx.abs() / (size * 0.45))
        }
        Trees::Dead => {
            if (dx).abs() < (size * 0.04).max(0.6) && up < size {
                return true;
            }
            // A few bare branches, angled up.
            (0..4).any(|b| {
                let at = size * (0.35 + 0.15 * b as f32);
                let dir = if (b + (seed * 10.0) as usize).is_multiple_of(2) { 1.0 } else { -1.0 };
                let along = dx * dir;
                along > 0.0 && along < size * 0.3 && (up - at - along * 0.9).abs() < 0.7
            })
        }
        Trees::Giant => {
            // A trunk tapering up, its roots flaring into the ground, a
            // limb or two.
            let half = size * (0.07 + 0.02 * (1.0 - up / size)) * (1.0 + 1.6 * (-(up / (size * 0.06))).exp());
            if dx.abs() < half {
                return true;
            }
            (0..2).any(|b| {
                let at = size * (0.55 + 0.2 * b as f32 + 0.1 * seed.fract());
                let dir = if b == 0 { 1.0 } else { -1.0 };
                let along = dx * dir - half;
                along > 0.0 && along < size * 0.25 && (up - at - along * 0.6).abs() < (size * 0.02).max(0.8)
            })
        }
        Trees::Cactus => {
            let w = (size * 0.07).max(0.8);
            if dx.abs() < w && up < size * 0.8 {
                return true;
            }
            // Arms: out, then up.
            (0..2).any(|b| {
                let dir = if b == 0 { 1.0 } else { -1.0 };
                let at = size * (0.3 + 0.15 * b as f32);
                let along = dx * dir;
                (along > 0.0 && along < size * 0.22 && (up - at).abs() < w) || ((along - size * 0.22).abs() < w && up > at && up < at + size * 0.3)
            })
        }
    }
}

/// Draw a scene at `time` in `style` as one still, `w`×`h` art pixels.
pub fn render(scene: &Scene, time: Time, style: Style, seed: u64, w: usize, h: usize) -> Canvas {
    let mut c = Canvas::new(w, h);
    let lt = light(time);
    let (wf, hf) = (w as f32, h as f32);
    let (mist, rim) = match style {
        Style::Flat => (0.0, 0.0),
        Style::Noita => (0.55, 1.0),
        Style::Moody => (1.0, 0.8),
    };
    match scene.underground {
        None => sky(&mut c, &lt, &scene.glows, style, seed),
        Some(dark) => {
            for p in c.px.iter_mut() {
                *p = dark;
            }
        }
    }
    // What's drawn where so far (for rims: the layer above a pixel).
    let mut owner = vec![usize::MAX; w * h];
    for (i, l) in scene.layers.iter().enumerate() {
        match l.kind {
            Kind::Wall => wall(&mut c, l, scene, seed.wrapping_add(i as u64)),
            Kind::Depths => depths(&mut c, l, scene, &lt, seed.wrapping_add(i as u64)),
            Kind::Teeth => teeth(&mut c, &mut owner, i, l, scene, seed.wrapping_add(i as u64)),
            _ => land(&mut c, &mut owner, i, l, scene, &lt, style, seed, mist, rim),
        }
    }
    glows(&mut c, scene, &lt, seed);
    if style == Style::Moody {
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x as f32 / wf - 0.5, y as f32 / hf - 0.5);
                let v = 1.0 - 0.55 * (dx * dx * 1.2 + dy * dy * 1.6).min(1.0);
                let p = c.at(x, y);
                *p = scale(*p, v);
            }
        }
    }
    c
}

fn sky(c: &mut Canvas, lt: &Light, glows: &Glows, style: Style, seed: u64) {
    let (w, h) = (c.w, c.h);
    let (wf, hf) = (w as f32, h as f32);
    for y in 0..h {
        let k = (y as f32 / (hf * 0.75)).min(1.0);
        let row = mix(lt.top, lt.horizon, k.powf(1.6));
        for x in 0..w {
            *c.at(x, y) = row;
        }
    }
    // The sun or moon, and its glow.
    let (sx, sy) = (lt.disc.0 * wf, lt.disc.1 * hf);
    let r = if lt.moon { hf * 0.035 } else { hf * 0.05 };
    for y in 0..h {
        for x in 0..w {
            let d = ((x as f32 - sx).powi(2) + (y as f32 - sy).powi(2)).sqrt();
            let glow = (1.0 - d / (hf * 0.5)).max(0.0).powi(3) * if lt.moon { 0.25 } else { 0.5 };
            let p = c.at(x, y);
            *p = add(*p, scale(lt.sun, glow));
            if d < r {
                let body = if lt.moon {
                    // (Craters: a few darker blots.)
                    let m = 0.82 + 0.18 * noise2(x as f32 * 0.5, y as f32 * 0.5, seed ^ 77);
                    scale(rgb(226, 230, 240), m)
                } else {
                    rgb(255, 250, 232)
                };
                *p = body;
            }
        }
    }
    // Stars: many faint, a few bright (twinkling in the game), a band of
    // the galaxy across.
    if lt.stars > 0.0 {
        for y in 0..(h as f32 * 0.8) as usize {
            for x in 0..w {
                let n = hash(x as i64, y as i64, seed ^ 0x5747);
                let band = (-((y as f32 / hf - 0.3 - (x as f32 / wf) * 0.25).powi(2)) / 0.012).exp();
                let chance = 0.006 + 0.03 * band;
                if n < chance {
                    let b = hash(x as i64, y as i64, seed ^ 0x51) ;
                    let bright = (0.25 + 0.75 * b.powi(3)) * lt.stars * (1.0 - y as f32 / (hf * 0.8));
                    let tint = if b > 0.9 { rgb(255, 214, 180) } else if b > 0.8 { rgb(180, 200, 255) } else { rgb(235, 238, 255) };
                    let p = c.at(x, y);
                    *p = add(*p, scale(tint, bright));
                    // (The brightest have a little cross.)
                    if b > 0.97 && x > 0 && x + 1 < w && y > 0 && y + 1 < h {
                        for (ax, ay) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                            let q = c.at(ax, ay);
                            *q = add(*q, scale(tint, bright * 0.35));
                        }
                    }
                }
                // The galaxy's haze.
                if band > 0.05 {
                    let haze = band * fbm2(x as f32 * 0.03, y as f32 * 0.05, 3, seed ^ 0x6a) * 0.05 * lt.stars;
                    let p = c.at(x, y);
                    *p = add(*p, [haze * 0.8, haze * 0.85, haze]);
                }
            }
        }
    }
    // Aurora (the far north at night): curtains of green rays, violet at
    // their feet, folding across the sky; soft at the top, ragged below.
    if glows.aurora && lt.stars > 0.5 {
        for x in 0..w {
            let u = x as f32 / wf;
            let fold = fbm1(u * 2.5, 3, seed ^ 0xa0);
            let top = hf * (0.04 + 0.2 * fold);
            let len = hf * (0.18 + 0.2 * fbm1(u * 4.0 + 9.0, 3, seed ^ 0xa1));
            // (Rays: fine stripes across, brighter where the curtain folds.)
            let rays = (0.35 + 0.65 * noise1(u * 180.0, seed ^ 0xa2)) * (0.4 + 0.6 * fbm1(u * 12.0, 2, seed ^ 0xa3));
            for y in 0..h {
                let k = (y as f32 - top) / len;
                if !(0.0..1.0).contains(&k) {
                    continue;
                }
                let shape = (k / 0.35).min(1.0).powi(2) * (1.0 - k).powf(1.5);
                let a = shape * rays * 0.4;
                let col = mix(rgb(80, 255, 160), rgb(190, 90, 255), (k - 0.5).max(0.0) * 2.0);
                let p = c.at(x, y);
                *p = add(*p, scale(col, a));
            }
        }
    }
    // Clouds: soft, dithered, lit on the sun's side (dark at night).
    for y in 0..(h as f32 * 0.55) as usize {
        for x in 0..w {
            let n = fbm2(x as f32 * 0.012, y as f32 * 0.035, 5, seed ^ 0xc10d);
            let fade = (1.0 - y as f32 / (hf * 0.55)).min(1.0) * (y as f32 / (hf * 0.08)).min(1.0);
            let a = ((n - 0.52) * 4.0).clamp(0.0, 1.0) * fade * if style == Style::Flat { 0.7 } else { 0.85 };
            if a > 0.0 {
                let lit = mix(lt.horizon, lt.sun, 0.35);
                let body = if lt.moon { rgb(22, 26, 40) } else { mix(lt.top, lit, 0.75) };
                let p = c.at(x, y);
                *p = mix(*p, body, a);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn land(c: &mut Canvas, owner: &mut [usize], i: usize, l: &Layer, scene: &Scene, lt: &Light, style: Style, seed: u64, mist: f32, rim: f32) {
    let (w, h) = (c.w, c.h);
    let (wf, hf) = (w as f32, h as f32);
    let s = seed.wrapping_add(i as u64 * 131);
    // Its colour: its own, lit by the time of day, hazed by distance.
    let haze = (1.0 - l.depth).powf(1.3) * 0.85;
    let lit = mul(l.color, lt.land);
    let base = mix(lit, lt.fog, haze);
    let levels = if style == Style::Moody { 0.6 } else { 1.0 };
    let tops: Vec<f32> = (0..w).map(|x| skyline(l, i, x as f32, wf, hf, seed)).collect();
    // Trees along the skyline.
    let size = hf * (0.03 + 0.09 * l.depth.powi(2)) * if l.trees == Trees::Giant { 5.0 } else { 1.0 };
    let spacing = (size * if l.trees == Trees::Giant { 1.6 } else { 0.55 }).max(2.0);
    let roots: Vec<(f32, f32, f32, f32)> = if l.trees == Trees::None {
        Vec::new()
    } else {
        let mut v = Vec::new();
        let mut x = 0.0;
        let mut k = 0i64;
        while x < wf + size {
            let r = hash(k, 1, s);
            // (Clumps and gaps.)
            if fbm1(x / wf * 6.0, 2, s ^ 0x7) > 0.38 {
                let tx = x + (r - 0.5) * spacing;
                let ty = tops[(tx.max(0.0) as usize).min(w - 1)];
                v.push((tx, ty + 1.0, size * (0.7 + 0.6 * hash(k, 2, s)), r * 100.0));
            }
            x += spacing * (0.6 + 0.8 * hash(k, 3, s));
            k += 1;
        }
        v
    };
    for x in 0..w {
        let top = tops[x];
        let near: Vec<&(f32, f32, f32, f32)> = roots.iter().filter(|r| (r.0 - x as f32).abs() < r.2 * 0.6).collect();
        for y in 0..h {
            let (xf, yf) = (x as f32, y as f32);
            let ground = yf >= top;
            let tree = !ground && near.iter().any(|r| tree_covers(l.trees, xf, yf, (r.0, r.1), r.2, r.3));
            if !(ground || tree) {
                continue;
            }
            let mut col = base;
            if matches!(l.kind, Kind::Sea) && ground {
                // Water: darker down, glints of the sky in rows.
                let k = (yf - top) / (hf - top).max(1.0);
                col = mix(mix(lt.horizon, lt.top, 0.5), scale(base, 0.6), k.powf(0.5));
                let glint = noise2(xf * 0.08, yf * 0.9, s ^ 3) > 0.78;
                if glint {
                    col = mix(col, lt.sun, 0.35 * (1.0 - k));
                }
            } else {
                // Texture (dithered noise), darker toward its foot.
                if style != Style::Flat {
                    // (Fine grain over a broad, gentle shading.)
                    let fine = noise2(xf * 0.45, yf * 0.45, s ^ 0x7e);
                    let broad = fbm2(xf * 0.025, yf * 0.025, 3, s ^ 0x7f);
                    let shade = 0.92 + (0.05 * fine + 0.14 * broad) * levels;
                    col = scale(col, shade);
                    let down = ((yf - top) / (hf * 0.35)).clamp(0.0, 1.0);
                    col = scale(col, 1.0 - 0.3 * down * l.depth);
                }
                // Snow on high peaks.
                // Snow on the high peaks: how high this column's top reaches
                // (0..1 of the relief), down from it a little (more on higher
                // peaks), ragged.
                if let Kind::Ridge { snow } = l.kind
                    && ground
                    && snow < 1.0
                {
                    let reach = ((l.base * hf - top) / (l.height * hf).max(1.0)).clamp(0.0, 1.0);
                    let cap = (reach - snow).max(0.0) * l.height * hf * 0.7 * (0.7 + 0.6 * noise1(xf * 0.3, s ^ 11));
                    if yf - top < cap {
                        col = mix(mul(rgb(236, 240, 250), lt.land), lt.fog, haze * 0.8);
                    }
                }
            }
            *c.at(x, y) = col;
            owner[y * w + x] = i;
        }
    }
    // A canopy closing overhead (the giants').
    if l.trees == Trees::Giant {
        for x in 0..w {
            let hang = hf * (0.06 + 0.16 * fbm1(x as f32 / wf * 9.0, 4, s ^ 0xca)) * (0.5 + l.depth);
            for y in 0..(hang as usize).min(h) {
                *c.at(x, y) = scale(base, 0.85);
                owner[y * w + x] = i;
            }
        }
    }
    // Rims: the edge against what's behind, lit by the sun or moon.
    if rim > 0.0 {
        for y in 1..h {
            for x in 0..w {
                if owner[y * w + x] == i && owner[(y - 1) * w + x] != i {
                    let p = c.at(x, y);
                    *p = mix(*p, lt.sun, 0.35 * rim * (0.4 + 0.6 * l.depth));
                }
            }
        }
    }
    // Mist lying at this layer's foot, rising over the layer behind.
    if mist > 0.0 && !matches!(l.kind, Kind::Sea) {
        let foot = (l.base * hf).min(hf);
        let thick = hf * 0.12 * mist;
        for y in 0..h {
            let dy = foot - y as f32;
            if !(0.0..thick).contains(&dy) {
                continue;
            }
            for x in 0..w {
                let wisp = fbm2(x as f32 * 0.015, y as f32 * 0.08, 3, s ^ 0x3157);
                let a = (1.0 - dy / thick).powi(2) * (0.65 + 0.35 * wisp) * 0.5 * mist * (1.0 - l.depth * 0.6);
                let p = c.at(x, y);
                *p = mix(*p, lt.fog, a);
            }
        }
    }
    let _ = scene;
}

/// A rock face: its colour broken by strata, cracks and a lumpy grain.
fn wall(c: &mut Canvas, l: &Layer, scene: &Scene, seed: u64) {
    let (w, h) = (c.w, c.h);
    let dark = scene.underground.unwrap_or([0.0; 3]);
    for y in 0..h {
        for x in 0..w {
            let (xf, yf) = (x as f32, y as f32);
            let warp = fbm2(xf * 0.01, yf * 0.01, 3, seed ^ 1) * 30.0;
            let strata = 0.5 + 0.5 * ((yf + warp) * 0.09).sin();
            let grain = fbm2(xf * 0.06, yf * 0.1, 4, seed ^ 2);
            let crack = (fbm2(xf * 0.02, yf * 0.02, 4, seed ^ 3) - 0.5).abs() < 0.005;
            let k = 0.6 + 0.2 * strata + 0.3 * grain;
            let mut col = mix(dark, scale(l.color, k), 0.15 + 0.4 * l.depth);
            if crack {
                col = scale(col, 0.75);
            }
            *c.at(x, y) = col;
        }
    }
}

/// Hollows opening deeper into the rock: shapes out of noise, hazier and
/// lighter the deeper they go (a cavern's far side seen through them).
fn depths(c: &mut Canvas, l: &Layer, scene: &Scene, lt: &Light, seed: u64) {
    let (w, h) = (c.w, c.h);
    let dark = scene.underground.unwrap_or([0.0; 3]);
    let glow = scene.glows.below;
    for y in 0..h {
        for x in 0..w {
            let (xf, yf) = (x as f32, y as f32);
            let n = fbm2(xf * 0.008, yf * 0.014, 4, seed);
            let open = ((n - 0.47) * 6.0).clamp(0.0, 1.0);
            if open <= 0.0 {
                continue;
            }
            // Deeper in: fainter, hazier (a far cavern's glow).
            let far = mix(scale(l.color, 0.9), mix(dark, l.color, 0.35), 1.0 - open);
            let mut col = mix(*c.at(x, y), far, open * 0.9);
            if let Some(g) = glow {
                let k = (yf / h as f32).powi(2);
                col = add(col, scale(g, k * 0.35 * open));
            }
            *c.at(x, y) = col;
        }
    }
    let _ = lt;
}

/// Stalactites down from the top, stalagmites up from the bottom, columns
/// where they meet; icicles in the cold.
fn teeth(c: &mut Canvas, owner: &mut [usize], i: usize, l: &Layer, scene: &Scene, seed: u64) {
    let (w, h) = (c.w, c.h);
    let (wf, hf) = (w as f32, h as f32);
    let dark = scene.underground.unwrap_or([0.0; 3]);
    // (Near, and between you and the far side's light: darker than it.)
    let col = mix(dark, l.color, 0.12 + 0.18 * (1.0 - l.depth));
    let ceiling = hf * (0.08 + 0.06 * (1.0 - l.depth));
    let floor = hf * (0.88 + 0.05 * (1.0 - l.depth));
    let mut k = 0i64;
    let mut x = 0.0;
    let mut spikes = Vec::new();
    while x < wf {
        let r = hash(k, 0, seed);
        let len = hf * (0.08 + 0.35 * r.powi(2)) * (0.5 + l.depth);
        let width = (len * (0.12 + 0.1 * hash(k, 1, seed))).max(2.0);
        let down = hash(k, 2, seed) < 0.6;
        let column = hash(k, 3, seed) < 0.08;
        spikes.push((x, width, len, down, column));
        x += width * (1.2 + 2.5 * hash(k, 4, seed));
        k += 1;
    }
    for y in 0..h {
        for x in 0..w {
            let (xf, yf) = (x as f32, y as f32);
            let mut on = yf < ceiling + 4.0 * noise1(xf * 0.1, seed ^ 5) || yf > floor - 4.0 * noise1(xf * 0.1, seed ^ 6);
            for &(sx, sw, len, down, column) in &spikes {
                let dx = (xf - sx).abs();
                if column {
                    if dx < sw * (0.5 + 0.5 * (((yf - hf / 2.0) / (hf / 2.0)).powi(2))) {
                        on = true;
                    }
                } else if down {
                    let d = yf - ceiling;
                    if d >= 0.0 && d < len && dx < sw * (1.0 - d / len) {
                        on = true;
                    }
                } else {
                    let d = floor - yf;
                    if d >= 0.0 && d < len * 0.7 && dx < sw * 1.2 * (1.0 - d / (len * 0.7)) {
                        on = true;
                    }
                }
            }
            if on {
                let n = fbm2(xf * 0.1, yf * 0.1, 3, seed ^ 8);
                let mut p = scale(col, 0.85 + 0.25 * n);
                if scene.glows.icicles && yf < hf * 0.5 {
                    p = mix(p, rgb(120, 170, 220), 0.25);
                }
                if let Some(g) = scene.glows.below {
                    p = add(p, scale(g, (yf / hf).powi(3) * 0.4));
                }
                *c.at(x, y) = p;
                owner[y * w + x] = i;
            }
        }
    }
}

/// Crystals, mushrooms, embers, fireflies, a pool: the little lights, and
/// the light they throw.
fn glows(c: &mut Canvas, scene: &Scene, lt: &Light, seed: u64) {
    let (w, h) = (c.w, c.h);
    let (wf, hf) = (w as f32, h as f32);
    let g = &scene.glows;
    let mut lights: Vec<(f32, f32, Rgb, f32)> = Vec::new();
    if let Some(col) = g.crystals {
        for k in 0..14 {
            // (On the floor, in clusters, and a few up on ledges.)
            let (x, y) = (hash(k, 0, seed ^ 0xc1) * wf, hf * if k % 4 == 0 { 0.55 + 0.25 * hash(k, 1, seed ^ 0xc1) } else { 0.86 + 0.05 * hash(k, 1, seed ^ 0xc1) });
            let size = 2.0 + 6.0 * hash(k, 2, seed ^ 0xc1);
            // A cluster of shards.
            for s in 0..4 {
                let (ox, lean) = ((s as f32 - 1.5) * size * 0.5, (s as f32 - 1.5) * 0.35);
                for t in 0..(size * (1.0 + 0.5 * hash(k, s + 3, seed))) as i32 {
                    let (px, py) = (x + ox + lean * t as f32, y - t as f32);
                    let half = ((size * 0.25) * (1.0 - t as f32 / (size * 1.5))).max(0.5);
                    for dx in -(half as i32)..=(half as i32) {
                        let (qx, qy) = ((px + dx as f32) as usize, py as usize);
                        if qx < w && qy < h {
                            let edge = dx as f32 / half.max(1.0);
                            *c.at(qx, qy) = mix(scale(col, 0.7), [1.0, 1.0, 1.0], 0.25 + 0.25 * edge);
                        }
                    }
                }
            }
            lights.push((x, y - size, col, size * 5.0));
        }
    }
    if let Some(col) = g.mushrooms {
        for k in 0..18 {
            let (x, y) = (hash(k, 0, seed ^ 0x3c) * wf, hf * (0.86 + 0.08 * hash(k, 1, seed ^ 0x3c)));
            let size = 2.0 + 5.0 * hash(k, 2, seed ^ 0x3c).powi(2);
            for t in 0..(size * 1.4) as i32 {
                let (qx, qy) = (x as usize, (y - t as f32) as usize);
                if qx < w && qy < h {
                    *c.at(qx, qy) = rgb(190, 190, 170);
                }
            }
            let top = y - size * 1.4;
            for dy in 0..(size * 0.6) as i32 {
                let half = size * (1.0 - dy as f32 / (size * 0.6)).sqrt();
                for dx in -(half as i32)..=(half as i32) {
                    let (qx, qy) = ((x + dx as f32) as usize, (top - dy as f32) as usize);
                    if qx < w && qy < h {
                        *c.at(qx, qy) = mix(col, [1.0; 3], 0.2 * (1.0 - dy as f32 / (size * 0.6)));
                    }
                }
            }
            lights.push((x, top, col, size * 6.0));
        }
    }
    if g.fireflies && lt.stars > 0.5 {
        for k in 0..40 {
            let (x, y) = (hash(k, 0, seed ^ 0xf1) * wf, hf * (0.5 + 0.45 * hash(k, 1, seed ^ 0xf1)));
            lights.push((x, y, rgb(200, 255, 120), 6.0));
        }
    }
    if g.embers {
        for k in 0..80 {
            let (x, y) = (hash(k, 0, seed ^ 0xe0) * wf, hf * hash(k, 1, seed ^ 0xe0).sqrt());
            let (qx, qy) = (x as usize, y as usize);
            if qx < w && qy < h {
                *c.at(qx, qy) = rgb(255, 170, 80);
            }
            lights.push((x, y, rgb(255, 120, 40), 4.0));
        }
    }
    if g.pool {
        let top = (hf * 0.9) as usize;
        for y in top..h {
            for x in 0..w {
                let mirror = top.saturating_sub(y - top + 1);
                let above = *c.at(x, mirror);
                let ripple = (noise2(x as f32 * 0.1, y as f32 * 0.8, seed ^ 0x9) - 0.5) * 0.3;
                *c.at(x, y) = scale(mix(above, rgb(20, 40, 60), 0.5), 0.6 + ripple);
            }
        }
    }
    // The light they throw (additive, falling off).
    for (lx, ly, col, r) in lights {
        let (x0, x1) = ((lx - r).max(0.0) as usize, ((lx + r) as usize).min(w));
        let (y0, y1) = ((ly - r).max(0.0) as usize, ((ly + r) as usize).min(h));
        for y in y0..y1 {
            for x in x0..x1 {
                let d = ((x as f32 - lx).powi(2) + (y as f32 - ly).powi(2)).sqrt() / r;
                if d < 1.0 {
                    let p = c.at(x, y);
                    *p = add(*p, scale(col, (1.0 - d).powi(2) * 0.25));
                }
            }
        }
    }
    let _ = TAU;
}

// ---- the scenes, as first placeholders ----

fn layer(kind: Kind, depth: f32, base: f32, height: f32, trees: Trees, color: Rgb) -> Layer {
    Layer { kind, depth, base, height, trees, color }
}

/// Every scene there is so far, by name.
pub fn scenes() -> Vec<Scene> {
    use Kind::*;
    use Trees as T;
    let sky = |name: &str, layers: Vec<Layer>, glows: Glows| Scene { name: name.into(), underground: None, layers, glows };
    let cave = |name: &str, dark: Rgb, layers: Vec<Layer>, glows: Glows| Scene { name: name.into(), underground: Some(dark), layers, glows };
    vec![
        sky(
            "forest",
            vec![
                layer(Ridge { snow: 0.5 }, 0.05, 0.62, 0.35, T::None, rgb(96, 110, 130)),
                layer(Hills, 0.3, 0.7, 0.18, T::Pine, rgb(60, 84, 70)),
                layer(Hills, 0.55, 0.78, 0.14, T::Pine, rgb(40, 62, 46)),
                layer(Hills, 0.85, 0.9, 0.1, T::Round, rgb(26, 42, 30)),
            ],
            Glows { fireflies: true, ..default() },
        ),
        sky(
            "plains",
            vec![
                layer(Hills, 0.1, 0.72, 0.1, T::None, rgb(110, 130, 120)),
                layer(Hills, 0.4, 0.8, 0.08, T::Round, rgb(84, 112, 70)),
                layer(Hills, 0.8, 0.9, 0.06, T::None, rgb(60, 88, 48)),
            ],
            Glows { fireflies: true, ..default() },
        ),
        sky(
            "deep forest",
            vec![
                layer(Hills, 0.15, 0.7, 0.1, T::Pine, rgb(50, 70, 60)),
                layer(Hills, 0.45, 0.85, 0.05, T::Giant, rgb(34, 46, 36)),
                layer(Hills, 0.85, 0.95, 0.04, T::Giant, rgb(18, 26, 20)),
            ],
            Glows { fireflies: true, mushrooms: Some(rgb(120, 220, 255)), ..default() },
        ),
        sky(
            "desert",
            vec![
                layer(Mesas, 0.1, 0.66, 0.2, T::None, rgb(186, 130, 96)),
                layer(Dunes, 0.4, 0.78, 0.1, T::None, rgb(214, 170, 110)),
                layer(Dunes, 0.8, 0.9, 0.08, T::Cactus, rgb(176, 130, 80)),
            ],
            Glows::default(),
        ),
        sky(
            "tundra",
            vec![
                layer(Ridge { snow: 0.0 }, 0.05, 0.62, 0.38, T::None, rgb(150, 160, 180)),
                layer(Ridge { snow: 0.3 }, 0.35, 0.72, 0.2, T::Pine, rgb(90, 100, 118)),
                layer(Hills, 0.8, 0.88, 0.08, T::Pine, rgb(200, 210, 224)),
            ],
            Glows { aurora: true, ..default() },
        ),
        sky(
            "jungle",
            vec![
                layer(Ridge { snow: 1.0 }, 0.1, 0.62, 0.3, T::None, rgb(70, 110, 90)),
                layer(Hills, 0.4, 0.76, 0.12, T::Round, rgb(40, 96, 56)),
                layer(Hills, 0.8, 0.88, 0.08, T::Palm, rgb(24, 64, 34)),
            ],
            Glows { fireflies: true, ..default() },
        ),
        sky(
            "swamp",
            vec![
                layer(Hills, 0.15, 0.74, 0.05, T::Dead, rgb(80, 96, 84)),
                layer(Sea, 0.4, 0.82, 0.0, T::None, rgb(40, 56, 50)),
                layer(Hills, 0.85, 0.92, 0.04, T::Dead, rgb(30, 38, 30)),
            ],
            Glows { fireflies: true, ..default() },
        ),
        sky(
            "mountains",
            vec![
                layer(Ridge { snow: 0.0 }, 0.02, 0.6, 0.45, T::None, rgb(140, 150, 170)),
                layer(Ridge { snow: 0.25 }, 0.3, 0.7, 0.35, T::None, rgb(100, 106, 120)),
                layer(Ridge { snow: 0.6 }, 0.6, 0.82, 0.22, T::Pine, rgb(66, 72, 80)),
                layer(Hills, 0.9, 0.93, 0.06, T::Pine, rgb(36, 46, 40)),
            ],
            Glows::default(),
        ),
        sky(
            "ocean",
            vec![
                layer(Hills, 0.05, 0.64, 0.06, T::None, rgb(110, 130, 150)),
                layer(Sea, 0.3, 0.66, 0.0, T::None, rgb(40, 80, 120)),
                layer(Dunes, 0.85, 0.94, 0.05, T::Palm, rgb(200, 180, 130)),
            ],
            Glows::default(),
        ),
        cave(
            "underground",
            rgb(10, 8, 6),
            vec![layer(Wall, 0.25, 0.0, 0.0, T::None, rgb(92, 66, 48)), layer(Depths, 0.3, 0.0, 0.0, T::None, rgb(54, 42, 36)), layer(Teeth, 0.7, 0.0, 0.0, T::None, rgb(60, 44, 34))],
            Glows::default(),
        ),
        cave(
            "caverns",
            rgb(6, 7, 10),
            vec![layer(Wall, 0.2, 0.0, 0.0, T::None, rgb(70, 74, 86)), layer(Depths, 0.3, 0.0, 0.0, T::None, rgb(40, 60, 80)), layer(Teeth, 0.75, 0.0, 0.0, T::None, rgb(56, 60, 70))],
            Glows { crystals: Some(rgb(160, 110, 255)), pool: true, ..default() },
        ),
        cave(
            "fungal grotto",
            rgb(4, 8, 10),
            vec![layer(Wall, 0.2, 0.0, 0.0, T::None, rgb(40, 64, 64)), layer(Depths, 0.3, 0.0, 0.0, T::None, rgb(30, 70, 80)), layer(Teeth, 0.7, 0.0, 0.0, T::None, rgb(34, 54, 50))],
            Glows { mushrooms: Some(rgb(90, 230, 255)), ..default() },
        ),
        cave(
            "ice cave",
            rgb(4, 7, 14),
            vec![layer(Wall, 0.3, 0.0, 0.0, T::None, rgb(70, 96, 130)), layer(Depths, 0.35, 0.0, 0.0, T::None, rgb(70, 120, 170)), layer(Teeth, 0.75, 0.0, 0.0, T::None, rgb(90, 130, 170))],
            Glows { icicles: true, crystals: Some(rgb(140, 220, 255)), ..default() },
        ),
        cave(
            "lava depths",
            rgb(10, 4, 3),
            vec![layer(Wall, 0.25, 0.0, 0.0, T::None, rgb(60, 32, 26)), layer(Depths, 0.3, 0.0, 0.0, T::None, rgb(120, 40, 20)), layer(Teeth, 0.7, 0.0, 0.0, T::None, rgb(34, 20, 18))],
            Glows { below: Some(rgb(255, 90, 30)), embers: true, ..default() },
        ),
    ]
}

fn default() -> Glows {
    Glows::default()
}
