//! Concept stills for the underground's three looks (DESIGN §4.3b):
//!
//! - Ordinary caves: behind the back walls, a flat dark tinted by depth
//!   (earth, stone, the deep), seen only where light reaches.
//! - Underdarks: vast caverns (a few a world) with their own dim light, a
//!   vista in layers: far walls in haze, pillars joining floor and roof,
//!   hanging rock, the theme's landmarks (a starlit roof and glowworm
//!   curtains; a forest of giant mushrooms; crystal spires; a dead city
//!   with its lamps), a glowing lake, motes in the air.
//! - The underworld: a vault over a lava sea, lit from below, embers
//!   rising, cracks in the far rock glowing.
//!
//! Stills at the game's scale (a cell a pixel), to decide on before the
//! game's tiles.

use crate::depths::{band_tile, bands};
use crate::{Rgb, fbm2, hash, mix, noise1, noise2, rgb, scale};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Theme {
    /// Siofra's: a roof of glowworm stars, curtains of them, a pale river.
    Starlit,
    /// A forest of giant mushrooms, spores drifting.
    Myconid,
    /// Crystal spires and geodes.
    Crystal,
    /// A dead city far off, its lamps still lit.
    Ruin,
}

pub const THEMES: [Theme; 4] = [Theme::Starlit, Theme::Myconid, Theme::Crystal, Theme::Ruin];

struct Look {
    /// The air far off (what distance fades to), the ambient light, what
    /// glows.
    mist: Rgb,
    ambient: Rgb,
    glow: Rgb,
    /// The rock, dark and lit.
    dark: Rgb,
    light: Rgb,
    /// Which depth band's far rock (`depths::bands`).
    band: &'static str,
}

fn look(t: Theme) -> Look {
    match t {
        Theme::Starlit => Look { mist: rgb(34, 58, 96), ambient: [0.42, 0.52, 0.78], glow: rgb(130, 230, 255), dark: rgb(34, 40, 58), light: rgb(110, 124, 160), band: "stone" },
        Theme::Myconid => Look { mist: rgb(30, 76, 70), ambient: [0.36, 0.58, 0.5], glow: rgb(90, 255, 190), dark: rgb(28, 46, 46), light: rgb(90, 136, 124), band: "fungal" },
        Theme::Crystal => Look { mist: rgb(60, 42, 100), ambient: [0.5, 0.4, 0.72], glow: rgb(200, 140, 255), dark: rgb(38, 32, 58), light: rgb(122, 104, 164), band: "crystal" },
        Theme::Ruin => Look { mist: rgb(76, 52, 30), ambient: [0.58, 0.44, 0.3], glow: rgb(255, 176, 80), dark: rgb(44, 36, 30), light: rgb(140, 118, 96), band: "stone" },
    }
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A canvas of lit colour and of glow (added over, and haloed).
struct Paint {
    w: usize,
    h: usize,
    col: Vec<Rgb>,
    glow: Vec<Rgb>,
}

impl Paint {
    fn new(w: usize, h: usize, fill: impl Fn(usize, usize) -> Rgb) -> Self {
        let col = (0..w * h).map(|i| fill(i % w, i / w)).collect();
        Paint { w, h, col, glow: vec![[0.0; 3]; w * h] }
    }

    fn set(&mut self, x: usize, y: usize, c: Rgb) {
        self.col[y * self.w + x] = c;
    }

    fn shine(&mut self, x: usize, y: usize, g: Rgb) {
        let p = &mut self.glow[y * self.w + x];
        *p = [p[0].max(g[0]), p[1].max(g[1]), p[2].max(g[2])];
    }

    /// Colour plus glow, the glow haloed (spread at a quarter size, as the
    /// game's light grid would), as RGBA.
    fn finish(&self, halo: f32) -> Vec<u8> {
        let (w, h) = (self.w, self.h);
        let (qw, qh) = (w.div_ceil(4), h.div_ceil(4));
        let mut q = vec![[0.0f32; 3]; qw * qh];
        for y in 0..h {
            for x in 0..w {
                let g = self.glow[y * w + x];
                for k in 0..3 {
                    q[(y / 4) * qw + x / 4][k] += g[k] / 16.0;
                }
            }
        }
        for _ in 0..5 {
            let prev = q.clone();
            for y in 0..qh {
                for x in 0..qw {
                    let mut sum = [0.0f32; 3];
                    for (dx, dy) in [(0i64, 0i64), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (nx, ny) = ((x as i64 + dx).clamp(0, qw as i64 - 1) as usize, (y as i64 + dy).clamp(0, qh as i64 - 1) as usize);
                        for k in 0..3 {
                            sum[k] += prev[ny * qw + nx][k] / 5.0;
                        }
                    }
                    q[y * qw + x] = sum;
                }
            }
        }
        let mut out = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let (fx, fy) = ((x as f32 / 4.0 - 0.5).max(0.0), (y as f32 / 4.0 - 0.5).max(0.0));
                let (ix, iy) = ((fx as usize).min(qw - 1), (fy as usize).min(qh - 1));
                let (jx, jy) = ((ix + 1).min(qw - 1), (iy + 1).min(qh - 1));
                let (tx, ty) = (fx.fract(), fy.fract());
                let c = self.col[y * w + x];
                let g = self.glow[y * w + x];
                for k in 0..3 {
                    let top = q[iy * qw + ix][k] * (1.0 - tx) + q[iy * qw + jx][k] * tx;
                    let bot = q[jy * qw + ix][k] * (1.0 - tx) + q[jy * qw + jx][k] * tx;
                    let hl = top * (1.0 - ty) + bot * ty;
                    // What's lit by the glow around it, the glow itself, a
                    // haze of it in the air.
                    let v = c[k] * (1.0 + hl * halo * 4.0) + g[k] + hl * halo * 0.5;
                    out.push((v.clamp(0.0, 1.0) * 255.0) as u8);
                }
                out.push(255);
            }
        }
        out
    }
}

/// Lit by `ambient`, faded into `mist` by `far` (0 near .. 1 far off).
fn lit(c: Rgb, ambient: Rgb, mist: Rgb, far: f32) -> Rgb {
    mix([c[0] * ambient[0], c[1] * ambient[1], c[2] * ambient[2]], mist, far)
}

/// An underdark of `theme`, seen from a ledge on its left, `w`×`h` cells.
pub fn vista(theme: Theme, w: usize, h: usize, seed: u64) -> Vec<u8> {
    let lk = look(theme);
    let s = seed ^ 0x0d4c;
    let (wf, hf) = (w as f32, h as f32);
    // The air: darker overhead, the mist thicker low down.
    let top = scale(lk.mist, 0.25);
    let mut p = Paint::new(w, h, |_, y| mix(top, lk.mist, smoothstep(0.0, 0.9, y as f32 / hf)));
    // Starlit: the roof far overhead is a sky of glowworms.
    if theme == Theme::Starlit {
        for y in 0..(h as f32 * 0.7) as usize {
            for x in 0..w {
                let r = hash(x as i64, y as i64, s ^ 0x57);
                if r < 0.012 {
                    let b = 0.4 + 0.6 * hash(x as i64, y as i64, s ^ 0x58);
                    let fade = 1.0 - y as f32 / (hf * 0.7);
                    p.shine(x, y, scale(lk.glow, b * fade));
                }
            }
        }
    }
    // The far walls: the band's far rock, deep in the mist.
    let band = bands().into_iter().find(|b| b.name == lk.band).expect("band");
    let (far_px, far_glow) = band_tile(&band, 0, 11_000, 9_000, w, h, seed);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            // (Only the lower two thirds: the roof is far above.)
            if far_px[i + 3] > 0 && (y as f32) > hf * (0.3 + 0.1 * noise1(x as f32 * 0.02, s ^ 1)) {
                let c = [far_px[i] as f32, far_px[i + 1] as f32, far_px[i + 2] as f32].map(|v| v / 255.0);
                p.set(x, y, lit(c, lk.ambient, lk.mist, 0.72));
                let a = far_glow[i + 3] as f32 / 255.0;
                if a > 0.0 {
                    p.shine(x, y, scale([far_glow[i] as f32, far_glow[i + 1] as f32, far_glow[i + 2] as f32].map(|v| v / 255.0), a * 0.3));
                }
            }
        }
    }
    // Pillars joining floor and roof (some broken: a stalactite over a
    // stalagmite), two depths.
    for (depth, every, wide) in [(0.6f32, 70.0f32, 10.0f32), (0.35, 120.0, 18.0)] {
        let n = (wf / every) as i64 + 2;
        for k in 0..n {
            if hash(k, 3, s ^ depth.to_bits() as u64) > 0.7 {
                continue;
            }
            let r = |j: i64| hash(k, j, s ^ depth.to_bits() as u64);
            let cx = (k as f32 + 0.2 + 0.6 * r(4)) * every;
            let half = wide * (0.6 + 0.8 * r(5));
            let broken = r(6) < 0.35;
            let gap = hf * (0.35 + 0.3 * r(7));
            for y in 0..h {
                let yf = y as f32;
                // Narrowest in the middle, flaring into roof and floor.
                let t = (yf / hf - 0.5).abs() * 2.0;
                let mut wy = half * (0.55 + 0.9 * t * t) + (noise1(yf * 0.06, s ^ k as u64) - 0.5) * half * 0.5;
                if broken {
                    // A stalactite over a stalagmite: a gap, each tapering
                    // to its point.
                    let (a, b) = (gap - 20.0 * r(8), gap + 20.0 * r(9) + 14.0);
                    if yf > a && yf < b {
                        continue;
                    }
                    let d = if yf <= a { a - yf } else { yf - b };
                    wy *= (d / 60.0).clamp(0.08, 1.0);
                }
                let x0 = (cx - wy).max(0.0) as usize;
                let x1 = ((cx + wy).min(wf - 1.0)).max(0.0) as usize;
                for x in x0..=x1 {
                    if (x as f32) < cx - wy || x as f32 > cx + wy {
                        continue;
                    }
                    // Lit from the upper left: its left side lighter.
                    let u = (x as f32 - cx) / wy.max(1.0);
                    let c = mix(lk.light, lk.dark, smoothstep(-0.6, 0.6, u));
                    p.set(x, y, lit(c, lk.ambient, lk.mist, depth));
                }
            }
        }
    }
    // The theme's landmarks, mid-distance.
    landmarks(&mut p, theme, &lk, s);
    // A mass of rock hanging from the roof, near; stalactites under it.
    for x in 0..w {
        let xf = x as f32;
        let bottom = hf * (0.12 + 0.14 * fbm2(xf * 0.008, 3.0, 3, s ^ 0x44)) + (hash(x as i64 / 3, 5, s) < 0.15) as i32 as f32 * 8.0 * hash(x as i64, 6, s);
        let cut = smoothstep(wf * 0.35, wf * 0.55, xf) * (1.0 - smoothstep(wf * 0.85, wf * 0.95, xf));
        let bottom = bottom * cut;
        for y in 0..bottom as usize {
            let under = bottom - y as f32 <= 2.0;
            let c = if under { lk.light } else { lk.dark };
            p.set(x, y, lit(c, lk.ambient, lk.mist, 0.15));
        }
    }
    // The lake on the floor, far below: dark water, the glow's reflection
    // in streaks.
    let lake = hf * 0.84;
    for y in lake as usize..h {
        for x in 0..w {
            let yf = y as f32;
            let c = mix(scale(lk.mist, 0.5), scale(lk.mist, 0.2), (yf - lake) / (hf - lake));
            p.set(x, y, c);
            let streak = noise2(x as f32 * 0.08, yf * 0.9, s ^ 0x1a);
            if streak > 0.72 {
                p.shine(x, y, scale(lk.glow, (streak - 0.72) * 1.2));
            }
        }
    }
    // The ledge the player stands on, near, on the left; its lip lit.
    for x in 0..w {
        let xf = x as f32;
        let edge = wf * 0.3 + 18.0 * (noise1(xf * 0.05, s ^ 0x61) - 0.5);
        if xf > edge {
            continue;
        }
        let top = hf * 0.62 + 10.0 * (noise1(xf * 0.03, s ^ 0x62) - 0.5) + smoothstep(edge - 40.0, edge, xf) * 60.0;
        for y in top.max(0.0) as usize..h {
            let lip = (y as f32 - top) < 2.0;
            let c = if lip { lit(lk.light, lk.ambient, lk.mist, 0.0) } else { scale(lk.dark, 0.35) };
            p.set(x, y, c);
        }
    }
    // Motes in the air: spores, dust, sparks of light.
    for k in 0..(w * h / 260) {
        let (x, y) = ((hash(k as i64, 81, s) * wf) as usize, (hash(k as i64, 82, s) * hf) as usize);
        let b = hash(k as i64, 83, s).powi(3);
        p.shine(x.min(w - 1), y.min(h - 1), scale(lk.glow, 0.2 + 0.8 * b));
    }
    p.finish(0.9)
}

/// Mid-distance: glowworm curtains, giant mushrooms, crystal spires, a
/// city.
fn landmarks(p: &mut Paint, theme: Theme, lk: &Look, s: u64) {
    let (w, h) = (p.w, p.h);
    let (wf, hf) = (w as f32, h as f32);
    let floor = hf * 0.8;
    match theme {
        Theme::Starlit => {
            // Curtains of glowworm threads from the roof, in drapes.
            for x in 0..w {
                let xf = x as f32;
                let drape = fbm2(xf * 0.02, 1.0, 2, s ^ 0x71);
                if drape < 0.5 || hash(x as i64, 72, s) > 0.5 {
                    continue;
                }
                let len = hf * (0.1 + 0.4 * (drape - 0.5) * 2.0) * (0.6 + 0.4 * hash(x as i64, 73, s));
                for y in 0..len as usize {
                    if hash(x as i64 * 97 + y as i64, 74, s) < 0.25 {
                        p.shine(x, y, scale(lk.glow, 0.5 * (1.0 - y as f32 / len)));
                    }
                }
                p.shine(x, len as usize, lk.glow);
            }
            // A pale river winding across the floor.
            for x in 0..w {
                let c = floor + 6.0 * (noise1(x as f32 * 0.02, s ^ 0x75) - 0.5);
                for y in c as usize..(c + 2.0) as usize {
                    p.shine(x, y.min(h - 1), scale(lk.glow, 0.7));
                }
            }
        }
        Theme::Myconid => {
            // Giant mushrooms: stems leaning, domed caps, gills underneath
            // and spots glowing.
            for k in 0..7i64 {
                let r = |j: i64| hash(k, j, s ^ 0x91);
                let (cx, tall, cap) = (wf * (0.3 + 0.1 * k as f32 + 0.06 * r(1)), hf * (0.25 + 0.3 * r(2)), 14.0 + 26.0 * r(3));
                let lean = (r(4) - 0.5) * 30.0;
                let far = 0.25 + 0.3 * r(5);
                let top = floor - tall;
                for y in top.max(0.0) as usize..floor as usize {
                    let t = (floor - y as f32) / tall;
                    let x = cx + lean * t * t;
                    let half = 2.0 + cap * 0.12 * (1.0 - t) + 1.0;
                    for xx in (x - half) as i64..=(x + half) as i64 {
                        if (0..w as i64).contains(&xx) {
                            p.set(xx as usize, y, lit(rgb(150, 180, 170), lk.ambient, lk.mist, far));
                        }
                    }
                }
                let (ccx, ccy) = (cx + lean, top);
                for y in (ccy - cap * 0.8).max(0.0) as usize..(ccy + cap * 0.25).min(hf) as usize {
                    for xx in (ccx - cap) as i64..=(ccx + cap) as i64 {
                        if !(0..w as i64).contains(&xx) {
                            continue;
                        }
                        let (dx, dy) = ((xx as f32 - ccx) / cap, (y as f32 - ccy) / (cap * 0.8));
                        if dx * dx + dy.min(0.0).powi(2) > 1.0 || dy > 0.3 {
                            continue;
                        }
                        let under = dy > 0.0;
                        let c = if under { rgb(60, 150, 130) } else { mix(rgb(70, 170, 150), rgb(30, 90, 90), smoothstep(-0.5, 0.8, dx)) };
                        p.set(xx as usize, y, lit(c, lk.ambient, lk.mist, far * 0.6));
                        if under && hash(xx, y as i64, s ^ 0x92) < 0.5 {
                            p.shine(xx as usize, y, scale(lk.glow, 0.6));
                        } else if !under && hash(xx / 2, y as i64 / 2, s ^ 0x93) < 0.1 {
                            p.shine(xx as usize, y, lk.glow);
                        }
                    }
                }
            }
        }
        Theme::Crystal => {
            // Spires: big prisms leaning out of the floor, lit face and
            // shaded, glowing at their tips; smaller ones around.
            for k in 0..14i64 {
                let r = |j: i64| hash(k, j, s ^ 0xa1);
                let (bx, len, wide) = (wf * (0.25 + 0.7 * r(1)), hf * (0.12 + 0.45 * r(2).powi(2)), 5.0 + 12.0 * r(3));
                let a = (r(4) - 0.5) * 0.7;
                let far = 0.15 + 0.45 * r(5);
                let (dx, dy) = (a.sin(), -a.cos());
                for y in 0..h {
                    for x in 0..w {
                        let (vx, vy) = (x as f32 - bx, y as f32 - floor);
                        let along = vx * dx + vy * dy;
                        let across = vx * dy - vy * dx;
                        let half = wide * ((len - along) / (wide * 1.8)).min(1.0);
                        if along > -4.0 && along < len && across.abs() < half {
                            let c = if across < 0.0 { rgb(210, 176, 250) } else { rgb(110, 70, 180) };
                            p.set(x, y, lit(c, lk.ambient, lk.mist, far));
                            if along > len - wide * 2.5 || across.abs() < 0.8 {
                                p.shine(x, y, scale(lk.glow, 0.5 * (1.0 - far)));
                            }
                        }
                    }
                }
            }
        }
        Theme::Ruin => {
            // The city's light in the mist behind it.
            for y in (hf * 0.35) as usize..floor as usize {
                for x in (wf * 0.3) as usize..w {
                    let t = smoothstep(hf * 0.35, floor, y as f32);
                    p.set(x, y, mix(p.col[y * w + x], scale(lk.glow, 0.45), t * 0.35));
                }
            }
            // A city on a rise far off: blocks and towers, arches, lamps
            // lit in a few windows.
            for x in (wf * 0.32) as usize..(wf * 0.98) as usize {
                let xf = x as f32;
                // (The rise tapers into the floor at either end.)
                let ends = smoothstep(wf * 0.32, wf * 0.42, xf) * (1.0 - smoothstep(wf * 0.9, wf * 0.98, xf));
                let base = floor - (10.0 + 22.0 * fbm2(xf * 0.01, 2.0, 2, s ^ 0xb3)) * ends;
                if ends < 0.3 {
                    continue;
                }
                let block = (xf / 9.0).floor() as i64;
                let r = |j: i64| hash(block, j, s ^ 0xb1);
                let tall = if r(1) < 0.15 { 50.0 + 60.0 * r(2) } else { 12.0 + 26.0 * r(2) };
                let roof = base - tall;
                for y in roof.max(0.0) as usize..base as usize {
                    let yf = y as f32;
                    // An arch at the foot of some.
                    let lx = xf - block as f32 * 9.0;
                    if r(3) < 0.3 && yf > base - 9.0 && (2.0..7.0).contains(&lx) {
                        continue;
                    }
                    // Spires on towers.
                    // (A dark silhouette against the lit mist, its lamps.)
                    let c = lit(mix(lk.dark, lk.light, 0.15), lk.ambient, lk.mist, 0.15);
                    p.set(x, y, c);
                    if lx.round() as i64 % 3 == 1 && (y as i64) % 5 == 2 && hash(x as i64, y as i64, s ^ 0xb2) < 0.3 {
                        p.shine(x, y, lk.glow);
                    }
                }
                // The rise it stands on.
                for y in base as usize..(floor + 4.0) as usize {
                    p.set(x, y, lit(lk.dark, lk.ambient, lk.mist, 0.2));
                }
            }
        }
    }
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

/// The underworld's vault over the lava sea, `w`×`h` cells.
pub fn underworld(w: usize, h: usize, seed: u64) -> Vec<u8> {
    let s = seed ^ 0x1a7a;
    let (wf, hf) = (w as f32, h as f32);
    let mist = rgb(90, 30, 16);
    let lava_top = hf * 0.8;
    // Lit from below: brighter low down.
    let light = |y: f32| -> Rgb { let t = smoothstep(0.0, lava_top, y); [0.25 + 0.95 * t, 0.12 + 0.4 * t, 0.08 + 0.18 * t] };
    let mut p = Paint::new(w, h, |_, y| mix(rgb(20, 6, 6), mist, smoothstep(0.1, 0.8, y as f32 / hf)));
    // The far rock, cracks glowing.
    let band = bands().into_iter().find(|b| b.name == "underworld").expect("band");
    // (The nearer layer's shapes: more open, the vault airier.)
    let (far, far_glow) = band_tile(&band, 1, 3_000, 2_000, w, h, seed);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            if far[i + 3] > 0 {
                let c = [far[i] as f32, far[i + 1] as f32, far[i + 2] as f32].map(|v| v / 255.0);
                let l = light(y as f32);
                p.set(x, y, mix([c[0] * l[0], c[1] * l[1], c[2] * l[2]], mist, 0.55));
                let a = far_glow[i + 3] as f32 / 255.0;
                if a > 0.0 {
                    p.shine(x, y, scale(rgb(255, 120, 40), a * 0.5));
                }
            }
        }
    }
    // The roof: rock hanging, stalactites.
    for x in 0..w {
        let xf = x as f32;
        let roof = hf * (0.08 + 0.1 * fbm2(xf * 0.01, 1.0, 3, s)) + if hash(x as i64 / 2, 1, s) < 0.2 { 20.0 * hash(x as i64 / 2, 2, s) } else { 0.0 };
        for y in 0..roof as usize {
            let l = light(y as f32);
            p.set(x, y, [0.2 * l[0], 0.12 * l[1], 0.1 * l[2]]);
        }
    }
    // Basalt islands in the sea, obsidian crusts; the sea glowing.
    for x in 0..w {
        let xf = x as f32;
        let island = fbm2(xf * 0.006, 5.0, 3, s ^ 3);
        let crest = if island > 0.44 { lava_top - ((island - 0.5) * 5.0).min(1.0).sqrt() * 40.0 - 8.0 * noise1(xf * 0.1, s ^ 8) } else { f32::MAX };
        for y in 0..h {
            let yf = y as f32;
            if yf >= crest {
                let edge = yf - crest < 2.0;
                p.set(x, y, if edge { rgb(200, 90, 50) } else { rgb(24, 14, 14) });
            } else if yf >= lava_top {
                let n = noise2(xf * 0.06, yf * 0.15 - xf * 0.01, s ^ 4);
                let c = mix(rgb(255, 90, 20), rgb(255, 210, 90), (n - 0.5).max(0.0) * 2.0);
                p.set(x, y, scale(c, 0.8));
                p.shine(x, y, scale(c, 0.5));
            }
        }
    }
    // Embers rising.
    for k in 0..(w * h / 400) {
        let (x, y) = ((hash(k as i64, 5, s) * wf) as usize, (hash(k as i64, 6, s) * lava_top) as usize);
        let b = hash(k as i64, 7, s);
        p.shine(x.min(w - 1), y.min(h - 1), scale(rgb(255, 160, 60), 0.3 + 0.7 * b * (y as f32 / lava_top)));
    }
    p.finish(0.8)
}
