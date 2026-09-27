//! Concept stills for the underground's three looks (DESIGN §4.3b):
//!
//! - Ordinary caves: behind the back walls, a flat dark tinted by depth
//!   (earth, stone, the deep), seen only where light reaches.
//! - Underdarks: vast caverns (a few a world) with their own dim light.
//!   Kept calm: a mist gradient, three silhouette layers of cavern (flat
//!   tones, nearer darker), a handful of big landmarks, and a few glows
//!   with soft round halos. Starlit (a roof of glowworm stars, a pale
//!   river), myconid (giant mushrooms), crystal (spires), ruin (a dead
//!   city, its lamps).
//! - The underworld: a red vault over a lava sea, lit from below.
//!
//! Stills at the game's scale (a cell a pixel), to decide on before the
//! game's tiles.

use crate::{Rgb, fbm2, hash, mix, noise1, noise2, rgb};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Theme {
    /// Siofra's: a roof of glowworm stars, threads of them, a pale river.
    Starlit,
    /// Giant mushrooms, their gills glowing.
    Myconid,
    /// Crystal spires, glowing at their tips.
    Crystal,
    /// A dead city far off, its lamps still lit.
    Ruin,
}

pub const THEMES: [Theme; 4] = [Theme::Starlit, Theme::Myconid, Theme::Crystal, Theme::Ruin];

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A light: where, its colour, how far its halo reaches, how strong.
struct Glow {
    x: f32,
    y: f32,
    c: Rgb,
    r: f32,
    k: f32,
}

/// Colour, plus soft round halos (added), as RGBA; flat, no dither.
fn finish(w: usize, h: usize, col: &[Rgb], glows: &[Glow]) -> Vec<u8> {
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let mut c = col[y * w + x];
            for g in glows {
                let d2 = (x as f32 - g.x).powi(2) + (y as f32 - g.y).powi(2);
                if d2 > g.r * g.r * 9.0 {
                    continue;
                }
                let a = g.k * (-d2 / (g.r * g.r)).exp();
                for (k, v) in c.iter_mut().enumerate() {
                    *v += g.c[k] * a;
                }
            }
            for v in c {
                out.push((v.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
            out.push(255);
        }
    }
    out
}

/// A cavern layer: its ceiling's lowest row and its floor's highest at a
/// column (rows down), with a few big stalactites and stalagmites and a
/// pillar or two; smooth, low-frequency shapes.
struct Cavern {
    roof: f32,
    floor: f32,
    /// How much the edges wander (rows), and how far apart the spikes.
    wander: f32,
    every: f32,
    spike: f32,
    pillars: bool,
    seed: u64,
}

impl Cavern {
    fn roof_at(&self, x: f32) -> f32 {
        let s = self.seed;
        let mut y = self.roof + self.wander * (noise1(x / 90.0, s) - 0.5) * 2.0;
        // A stalactite now and then: a long smooth cone.
        let k = (x / self.every).floor() as i64;
        for j in k - 1..=k + 1 {
            if hash(j, 1, s) > 0.45 {
                continue;
            }
            let cx = (j as f32 + 0.2 + 0.6 * hash(j, 2, s)) * self.every;
            let wide = self.every * (0.12 + 0.12 * hash(j, 3, s));
            let len = self.spike * (0.4 + 0.6 * hash(j, 4, s));
            let d = ((x - cx).abs() / wide).min(1.0);
            y = y.max(self.roof + len * (1.0 - d).powf(1.6));
        }
        y
    }

    fn floor_at(&self, x: f32) -> f32 {
        let s = self.seed ^ 0x77;
        let mut y = self.floor - self.wander * (noise1(x / 110.0, s) - 0.5) * 2.0;
        let k = (x / (self.every * 1.3)).floor() as i64;
        for j in k - 1..=k + 1 {
            if hash(j, 1, s) > 0.35 {
                continue;
            }
            let cx = (j as f32 + 0.2 + 0.6 * hash(j, 2, s)) * self.every * 1.3;
            let wide = self.every * (0.14 + 0.14 * hash(j, 3, s));
            let len = self.spike * 0.6 * (0.4 + 0.6 * hash(j, 4, s));
            let d = ((x - cx).abs() / wide).min(1.0);
            y = y.min(self.floor - len * (1.0 - d).powf(1.6));
        }
        y
    }

    /// Is (x, y) rock?
    fn solid(&self, x: f32, y: f32) -> bool {
        if y <= self.roof_at(x) || y >= self.floor_at(x) {
            return true;
        }
        // Pillars: roof to floor, narrow in the middle.
        self.pillars && {
            let every = self.every * 3.2;
            let k = (x / every).floor() as i64;
            (k - 1..=k + 1).any(|j| {
                if hash(j, 9, self.seed) > 0.5 {
                    return false;
                }
                let cx = (j as f32 + 0.3 + 0.4 * hash(j, 10, self.seed)) * every;
                let t = ((y - self.roof) / (self.floor - self.roof)).clamp(0.0, 1.0);
                let half = self.every * (0.07 + 0.05 * hash(j, 11, self.seed)) * (0.6 + 1.6 * (t - 0.5).powi(2) * 4.0);
                (x - cx).abs() < half
            })
        }
    }
}

struct Palette {
    /// Overhead, and the mist low down (the far air's glow).
    top: Rgb,
    mist: Rgb,
    /// The nearest rock (layers far to near go from the mist to this).
    near: Rgb,
    glow: Rgb,
}

fn palette(t: Theme) -> Palette {
    match t {
        Theme::Starlit => Palette { top: rgb(6, 9, 22), mist: rgb(52, 78, 128), near: rgb(8, 11, 24), glow: rgb(150, 225, 255) },
        Theme::Myconid => Palette { top: rgb(5, 14, 16), mist: rgb(44, 96, 90), near: rgb(6, 18, 20), glow: rgb(110, 255, 200) },
        Theme::Crystal => Palette { top: rgb(10, 6, 22), mist: rgb(82, 58, 132), near: rgb(14, 9, 28), glow: rgb(210, 150, 255) },
        Theme::Ruin => Palette { top: rgb(14, 8, 5), mist: rgb(112, 72, 38), near: rgb(18, 11, 7), glow: rgb(255, 180, 90) },
    }
}

/// An underdark of `theme`, `w`×`h` cells.
pub fn vista(theme: Theme, w: usize, h: usize, seed: u64) -> Vec<u8> {
    let pl = palette(theme);
    let (wf, hf) = (w as f32, h as f32);
    // (Each theme its own cavern.)
    let s = seed ^ 0x0d4c ^ (theme as u64 * 0x9e37);
    // The air: dark overhead, the mist glowing low down.
    let mut col: Vec<Rgb> = (0..w * h).map(|i| mix(pl.top, pl.mist, smoothstep(0.15, 0.85, (i / w) as f32 / hf))).collect();
    let mut glows: Vec<Glow> = Vec::new();
    // Starlit: a roof of glowworm stars overhead, sparse.
    if theme == Theme::Starlit {
        for k in 0..70i64 {
            let (x, y) = (hash(k, 1, s) * wf, hash(k, 2, s).powf(1.5) * hf * 0.5);
            let b = 0.3 + 0.7 * hash(k, 3, s).powi(2);
            col[(y as usize).min(h - 1) * w + (x as usize).min(w - 1)] = mix(pl.mist, pl.glow, b);
            if b > 0.7 {
                glows.push(Glow { x, y, c: pl.glow, r: 3.0, k: 0.25 * b });
            }
        }
    }
    // Three layers of cavern, far to near: each one flat tone, from near
    // the mist to near black; the far ones' openings wider.
    let layers = [
        Cavern { roof: hf * 0.1, floor: hf * 0.74, wander: 12.0, every: 70.0, spike: 50.0, pillars: true, seed: s ^ 1 },
        Cavern { roof: hf * 0.04, floor: hf * 0.84, wander: 18.0, every: 110.0, spike: 80.0, pillars: true, seed: s ^ 2 },
        Cavern { roof: -hf * 0.02, floor: hf * 0.95, wander: 22.0, every: 170.0, spike: 110.0, pillars: false, seed: s ^ 3 },
    ];
    let tones = [mix(pl.mist, pl.near, 0.45), mix(pl.mist, pl.near, 0.72), pl.near];
    for (k, cav) in layers.iter().enumerate() {
        // (The landmarks stand between the far layer and the middle one.)
        if k == 1 {
            landmarks(&mut col, &mut glows, theme, &pl, w, h, s, tones[0], tones[1]);
        }
        for x in 0..w {
            let floor = cav.floor_at(x as f32);
            for y in 0..h {
                let yf = y as f32;
                if cav.solid(x as f32, yf) {
                    // A faint lift low down: the mist.
                    let lift = smoothstep(floor - 30.0, floor + 40.0, yf) * 0.12 * (1.0 - k as f32 * 0.4);
                    col[y * w + x] = mix(tones[k], pl.mist, lift);
                }
            }
        }
    }
    // Starlit: a few glowworm threads from the near roof, and the pale
    // river along the far floor.
    if theme == Theme::Starlit {
        let near = &layers[2];
        for k in 0..14i64 {
            let x = hash(k, 21, s) * wf;
            let top = near.roof_at(x).max(0.0);
            let len = 10.0 + 40.0 * hash(k, 22, s);
            for y in top as usize..((top + len) as usize).min(h - 1) {
                col[y * w + x as usize] = mix(pl.near, pl.glow, 0.25);
            }
            glows.push(Glow { x, y: top + len, c: pl.glow, r: 4.0, k: 0.6 });
            col[((top + len) as usize).min(h - 1) * w + x as usize] = pl.glow;
        }
    }
    finish(w, h, &col, &glows)
}

/// The theme's few big landmarks, between the far cavern layer and the
/// middle one: shapes in tone `far`..`mid`, a few glows.
#[allow(clippy::too_many_arguments)]
fn landmarks(col: &mut [Rgb], glows: &mut Vec<Glow>, theme: Theme, pl: &Palette, w: usize, h: usize, s: u64, far: Rgb, mid: Rgb) {
    let (wf, hf) = (w as f32, h as f32);
    let ground = hf * 0.8;
    let tone = mix(far, mid, 0.5);
    let mut put = |x: i64, y: i64, c: Rgb| {
        if (0..w as i64).contains(&x) && (0..h as i64).contains(&y) {
            col[y as usize * w + x as usize] = c;
        }
    };
    match theme {
        Theme::Starlit => {
            // The river: a pale thin line winding along the far floor, a
            // soft glow over it.
            for x in 0..w {
                let y = hf * 0.73 + 4.0 * (noise1(x as f32 / 60.0, s ^ 5) - 0.5);
                put(x as i64, y as i64, mix(pl.mist, pl.glow, 0.7));
                if x % 24 == 0 {
                    glows.push(Glow { x: x as f32, y, c: pl.glow, r: 14.0, k: 0.12 });
                }
            }
        }
        Theme::Myconid => {
            // Four giant mushrooms: a leaning stem, a flat dome; their gills
            // glowing along the cap's underside.
            for k in 0..4i64 {
                let r = |j: i64| hash(k, j, s ^ 0x91);
                let cx = wf * (0.15 + 0.23 * k as f32 + 0.08 * (r(1) - 0.5));
                let tall = hf * (0.28 + 0.2 * r(2));
                let cap = 20.0 + 18.0 * r(3);
                let lean = (r(4) - 0.5) * 24.0;
                let top = ground - tall;
                for y in top as i64..ground as i64 {
                    let t = (ground - y as f32) / tall;
                    let x = cx + lean * t * t;
                    let half = 2.5 + 2.0 * (1.0 - t);
                    for xx in (x - half) as i64..=(x + half) as i64 {
                        put(xx, y, tone);
                    }
                }
                let ccx = cx + lean;
                for y in (top - cap * 0.55) as i64..=(top + 1.0) as i64 {
                    let dy = (y as f32 - top) / (cap * 0.55);
                    let half = cap * (1.0 - dy.min(0.0).powi(2)).max(0.0).sqrt();
                    for xx in (ccx - half) as i64..=(ccx + half) as i64 {
                        put(xx, y, tone);
                    }
                }
                // The gills: a glowing line under the cap.
                for xx in (ccx - cap * 0.9) as i64..=(ccx + cap * 0.9) as i64 {
                    put(xx, (top + 2.0) as i64, mix(tone, pl.glow, 0.8));
                }
                glows.push(Glow { x: ccx, y: top + 4.0, c: pl.glow, r: cap * 0.9, k: 0.22 });
            }
        }
        Theme::Crystal => {
            // Three clusters of big spires: two tones (a lit face, a shaded
            // one), a glow at their tips.
            for k in 0..3i64 {
                let bx = wf * (0.2 + 0.3 * k as f32 + 0.12 * (hash(k, 1, s) - 0.5));
                // (Each cluster its own size and count.)
                let size = 0.55 + 0.6 * hash(k, 2, s);
                for j in 0..(1 + (hash(k, 3, s) * 3.0) as i64) {
                    let r = |i: i64| hash(k * 7 + j, i, s ^ 0xa1);
                    let a = if j == 0 { (r(1) - 0.5) * 0.2 } else { (if j == 1 { -1.0 } else { 1.0 }) * (0.3 + 0.25 * r(1)) };
                    let len = size * if j == 0 { hf * (0.35 + 0.15 * r(2)) } else { hf * (0.15 + 0.12 * r(2)) };
                    let wide = size * if j == 0 { 12.0 } else { 7.0 };
                    let (dx, dy) = (a.sin(), -a.cos());
                    for y in 0..h {
                        for x in 0..w {
                            let (vx, vy) = (x as f32 - bx, y as f32 - ground);
                            let along = vx * dx + vy * dy;
                            let across = vx * dy - vy * dx;
                            let half = wide * ((len - along) / (wide * 1.6)).min(1.0);
                            if along > -2.0 && along < len && across.abs() < half {
                                let c = if across < 0.0 { mix(tone, pl.glow, 0.35) } else { tone };
                                put(x as i64, y as i64, c);
                            }
                        }
                    }
                    let tip = (bx + dx * len, ground + dy * len);
                    glows.push(Glow { x: tip.0, y: tip.1, c: pl.glow, r: 10.0 + wide, k: 0.3 });
                }
            }
        }
        Theme::Ruin => {
            // A city on a low rise: one flat silhouette, a few towers, and
            // its lamps; its light in the air behind.
            glows.push(Glow { x: wf * 0.6, y: ground - 30.0, c: pl.glow, r: wf * 0.22, k: 0.12 });
            for x in (wf * 0.3) as usize..(wf * 0.92) as usize {
                let xf = x as f32;
                let ends = smoothstep(wf * 0.3, wf * 0.38, xf) * (1.0 - smoothstep(wf * 0.84, wf * 0.92, xf));
                let base = ground - 12.0 * ends;
                let block = (xf / 10.0).floor() as i64;
                let r = |j: i64| hash(block, j, s ^ 0xb1);
                let tall = (if r(1) < 0.12 { 40.0 + 30.0 * r(2) } else { 8.0 + 18.0 * r(2) }) * ends;
                for y in (base - tall) as i64..ground as i64 {
                    put(x as i64, y, tone);
                }
                // A lamp in a window now and then.
                if ends > 0.5 && x % 5 == 2 && r(3) < 0.5 {
                    let y = base - 4.0 - (r(4) * (tall - 6.0).max(0.0) / 6.0).floor() * 6.0;
                    put(x as i64, y as i64, pl.glow);
                    glows.push(Glow { x: xf, y, c: pl.glow, r: 3.0, k: 0.25 });
                }
            }
        }
    }
}

/// The underworld's vault over the lava sea, `w`×`h` cells: a red haze
/// lit from below, three layers of rock in silhouette, the sea's glow.
pub fn underworld(w: usize, h: usize, seed: u64) -> Vec<u8> {
    let (wf, hf) = (w as f32, h as f32);
    let s = seed ^ 0x1a7a;
    let (top, mist, near, lava) = (rgb(12, 3, 3), rgb(150, 46, 18), rgb(16, 5, 4), rgb(255, 150, 50));
    let sea = hf * 0.86;
    let mut col: Vec<Rgb> = (0..w * h).map(|i| mix(top, mist, smoothstep(0.1, 0.86, (i / w) as f32 / hf))).collect();
    let mut glows: Vec<Glow> = Vec::new();
    let layers = [
        Cavern { roof: hf * 0.08, floor: hf * 0.8, wander: 14.0, every: 80.0, spike: 50.0, pillars: true, seed: s ^ 1 },
        Cavern { roof: hf * 0.02, floor: hf * 0.84, wander: 20.0, every: 120.0, spike: 80.0, pillars: false, seed: s ^ 2 },
        Cavern { roof: -hf * 0.04, floor: hf * 0.9, wander: 24.0, every: 180.0, spike: 100.0, pillars: false, seed: s ^ 3 },
    ];
    let tones = [mix(mist, near, 0.45), mix(mist, near, 0.72), near];
    for (k, cav) in layers.iter().enumerate() {
        for x in 0..w {
            for y in 0..h {
                if y as f32 >= sea {
                    continue;
                }
                if cav.solid(x as f32, y as f32) {
                    // Lit from below: the undersides and the low parts warmer.
                    let lift = smoothstep(hf * 0.3, sea, y as f32) * 0.25 * (1.0 - k as f32 * 0.3);
                    col[y * w + x] = mix(tones[k], mist, lift);
                }
            }
        }
    }
    // The sea: bright, a few slow swirls.
    for y in sea as usize..h {
        for x in 0..w {
            let n = fbm2(x as f32 * 0.02, y as f32 * 0.08, 2, s ^ 4);
            col[y * w + x] = mix(rgb(230, 90, 20), rgb(255, 200, 90), smoothstep(0.45, 0.7, n));
        }
    }
    for x in (0..w).step_by(30) {
        glows.push(Glow { x: x as f32 + 15.0, y: sea, c: lava, r: 40.0, k: 0.18 });
    }
    // A few embers rising.
    for k in 0..14i64 {
        let (x, y) = (hash(k, 1, s) * wf, sea - hash(k, 2, s) * hf * 0.5);
        col[(y as usize).min(h - 1) * w + (x as usize).min(w - 1)] = lava;
        glows.push(Glow { x, y, c: lava, r: 2.5, k: 0.4 });
    }
    finish(w, h, &col, &glows)
}

/// An ordinary cave, dug: the world's rock around a tunnel, the dark
/// behind tinted for band `k` (0 earth, 1 stone, 2 the deep), a torch.
pub fn cave(k: usize, w: usize, h: usize, seed: u64) -> Vec<u8> {
    let tints = [rgb(60, 42, 30), rgb(44, 46, 52), rgb(28, 32, 48)];
    let rock = [(rgb(92, 64, 44), rgb(150, 110, 76)), (rgb(78, 80, 88), rgb(130, 134, 144)), (rgb(46, 50, 64), rgb(92, 98, 120))];
    let tint = tints[k.min(2)];
    let (dark, light) = rock[k.min(2)];
    let (wf, hf) = (w as f32, h as f32);
    let (cx, cy) = (wf * 0.5, hf * 0.58);
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let (xf, yf) = (x as f32, y as f32);
            // A tunnel winding across, a chamber in the middle.
            let tunnel = (yf - (cy + 26.0 * (noise1(xf * 0.012, seed) - 0.5) * 2.0)).abs() < 14.0 + 10.0 * noise1(xf * 0.05, seed ^ 2);
            let room = ((xf - cx) / 70.0).powi(2) + ((yf - cy + 10.0) / 36.0).powi(2) < 1.0 + 0.3 * (fbm2(xf * 0.05, yf * 0.05, 2, seed ^ 3) - 0.5);
            let open = tunnel || room;
            let c = if open {
                tint
            } else {
                let n = noise2(xf * 0.5, yf * 0.5, seed ^ 4) * 0.5 + fbm2(xf * 0.06, yf * 0.06, 2, seed ^ 5) * 0.5;
                mix(dark, light, n)
            };
            // A torch at the middle.
            let d = ((xf - cx).powi(2) + (yf - cy).powi(2)).sqrt();
            let l = (1.0 - d / 90.0).max(0.0).powf(1.5) * 1.4;
            let warm = [1.0, 0.82, 0.6];
            for i in 0..3 {
                out.push(((c[i] * l * warm[i]).clamp(0.0, 1.0) * 255.0) as u8);
            }
            out.push(255);
        }
    }
    for dy in -3i64..=3 {
        for dx in -1i64..=1 {
            let i = (((cy as i64 + dy) as usize) * w + (cx as i64 + dx) as usize) * 4;
            out[i..i + 3].copy_from_slice(&if dy < 0 { [255, 210, 120] } else { [110, 70, 40] });
        }
    }
    out
}
