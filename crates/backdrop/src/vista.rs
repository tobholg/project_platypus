//! The surface's backdrop, Terraria-like: a bright sky, far hazy ranges,
//! cliffs of streaked rock with grass on their tops draping over the edges,
//! green hills with tree lines in front, and chunky clouds drifting far
//! off. A vista is a sky, a cloud look and layers far to near; each layer
//! is drawn a tile at a time from absolute coordinates (tiles side by side
//! join up), in daylight colours (the game's lighting grades them).

use crate::{Rgb, Trees, bayer, fbm1, hash, mix, noise1, noise2, rgb, scale, tree_covers};

/// A layer's skyline.
#[derive(Clone, Copy, Debug)]
pub enum Shape {
    /// Flat-topped plateaus in steps, steep cliffs between.
    Mesas { width: f32, steps: f32 },
    /// Craggy peaks.
    Peaks { width: f32 },
    /// Soft rolling hills.
    Hills { width: f32 },
    /// Tall thin spires over low hills.
    Spires { every: f32 },
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub rock: Rgb,
    /// Faces towards the light (the left).
    pub lit: Rgb,
    pub grass: Rgb,
    pub grass_top: Rgb,
}

#[derive(Clone, Copy, Debug)]
pub struct VLayer {
    pub shape: Shape,
    /// 0 far .. 1 near: how much the sky's haze fades it.
    pub depth: f32,
    /// The skyline's lowest (base) and highest (base - height) rows, as
    /// shares of the strip's height from its top.
    pub base: f32,
    pub height: f32,
    pub colors: Palette,
    /// Grass on the tops (0 none .. 1 thick, draping far).
    pub grass: f32,
    /// A tree line along the top.
    pub trees: Trees,
    /// Mist rising from the foot (0..1).
    pub mist: f32,
}

#[derive(Clone, Debug)]
pub struct Vista {
    pub name: &'static str,
    /// The sky, top and horizon.
    pub sky: (Rgb, Rgb),
    /// Clouds: how many (0..1), how big.
    pub clouds: (f32, f32),
    pub layers: Vec<VLayer>,
}

impl Vista {
    fn haze(&self) -> Rgb {
        self.sky.1
    }
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Noise (mostly 0.25..0.75) stretched to 0..1: skylines that dip low
/// between their heights, not walls.
fn spread(n: f32) -> f32 {
    ((n - 0.25) * 2.0).clamp(0.0, 1.0)
}

/// The skyline's height at `x`, 0..1 (1 the tallest).
fn profile(shape: Shape, x: f32, s: u64) -> f32 {
    match shape {
        Shape::Mesas { width, steps } => {
            let v = spread(fbm1(x / width, 3, s)).min(0.999) * steps;
            let (lvl, f) = (v.floor(), v.fract());
            // Flat on each step, a steep cliff up to the next.
            let h = (lvl + smoothstep(0.78, 0.97, f)) / steps;
            (h + 0.015 * (noise1(x * 0.25, s ^ 3) - 0.5)).clamp(0.0, 1.0)
        }
        Shape::Peaks { width } => {
            let u = x / width;
            let broad = spread(fbm1(u * 0.5, 2, s));
            let ridged = 1.0 - (2.0 * fbm1(u * 1.6 + 5.0, 4, s ^ 9) - 1.0).abs();
            (0.35 * broad + 0.65 * ridged * (0.35 + 0.65 * broad)).powf(1.3) + 0.02 * (noise1(x * 0.3, s ^ 5) - 0.5)
        }
        Shape::Hills { width } => spread(fbm1(x / width, 3, s)) * 0.9 + 0.1 * fbm1(x / (width * 0.25), 2, s ^ 4),
        Shape::Spires { every } => {
            let hills = 0.25 * fbm1(x / (every * 2.0), 2, s);
            let slot = (x / every).floor() as i64;
            let spire = (slot - 1..=slot + 1)
                .map(|k| {
                    if hash(k, 1, s) < 0.35 {
                        return 0.0;
                    }
                    let cx = (k as f32 + 0.2 + 0.6 * hash(k, 2, s)) * every;
                    let w = every * (0.08 + 0.12 * hash(k, 3, s));
                    let tall = 0.5 + 0.5 * hash(k, 4, s);
                    let d = ((x - cx).abs() / w).min(1.0);
                    // A column: a rounded crown, sheer sides, flaring at
                    // the foot.
                    tall * (1.0 - smoothstep(0.55, 1.0, d)) * (1.0 - 0.08 * d * d) * (1.0 - 0.06 * (d * 5.0).min(1.0))
                })
                .fold(0.0, f32::max);
            hills.max(spire)
        }
    }
}

/// Layer `k` of `v`, strip columns `x0 .. x0 + w`, `h` rows, RGBA (clear
/// above the skyline).
pub fn layer_tile(v: &Vista, k: usize, x0: i64, w: usize, h: usize, seed: u64) -> Vec<u8> {
    let l = &v.layers[k];
    let s = seed.wrapping_add(k as u64 * 7717 + 11);
    let hf = h as f32;
    let top = |x: f32| (l.base - l.height * profile(l.shape, x, s)) * hf;
    let haze = (1.0 - l.depth).powf(1.5) * 0.42;
    let foot = l.base * hf;
    let margin = 24i64;
    // Trees along the top: a slot every 4 columns, in clumps.
    let mut roots: Vec<(f32, f32, f32, f32)> = Vec::new();
    if !matches!(l.trees, Trees::None) {
        for slot in (x0 - margin).div_euclid(4)..=(x0 + w as i64 + margin).div_euclid(4) {
            let x = slot as f32 * 4.0 + hash(slot, 1, s) * 3.0;
            if fbm1(x / 50.0, 2, s ^ 0x7) < 0.4 || hash(slot, 2, s) < 0.25 {
                continue;
            }
            let size = (6.0 + 10.0 * hash(slot, 4, s)) * (0.5 + 0.5 * l.depth);
            roots.push((x, top(x) + 1.5, size, hash(slot, 5, s) * 100.0));
        }
    }
    // Below the foot, one flat colour, undithered (the game stretches the
    // bottom row down over valleys: a stripe there would run down the view).
    let hills = matches!(l.shape, Shape::Hills { .. }) && l.grass >= 1.0;
    let body = mix(if hills { scale(l.colors.grass, 0.9) } else { l.colors.rock }, v.haze(), haze + (1.0 - haze) * l.mist * 0.55);
    let below = (foot + 3.0).ceil() as usize;
    let mut out = vec![0u8; w * h * 4];
    for xi in 0..w {
        let xa = x0 + xi as i64;
        let xf = xa as f32;
        let t = top(xf);
        for y in below.min(h)..h {
            let i = (y * w + xi) * 4;
            for c in 0..3 {
                out[i + c] = (((body[c].clamp(0.0, 1.0) * 40.0).round() / 40.0) * 255.0) as u8;
            }
            out[i + 3] = 255;
        }
        let near: Vec<&(f32, f32, f32, f32)> = roots.iter().filter(|q| (q.0 - xf).abs() < q.2 * 0.5).collect();
        // How flat the top is here, and a drape of grass hanging from it.
        let slope = (top(xf + 2.0) - top(xf - 2.0)) / 4.0;
        let flat = 1.0 - (slope.abs() / 1.2).clamp(0.0, 1.0);
        let drip = {
            let n = noise1(xf * 0.45, s ^ 0xd1);
            let long = if hash(xa.div_euclid(2), 9, s) < 0.35 { 1.0 } else { 0.35 };
            ((n - 0.35).max(0.0) * 18.0 * long) * l.grass
        };
        let thick = if l.grass > 0.0 { (1.5 + 4.0 * flat) * l.grass.sqrt() + drip } else { 0.0 };
        for y in 0..below.min(h) {
            let yf = y as f32;
            let tree = yf < t && near.iter().any(|q| tree_covers(l.trees, xf, yf, (q.0, q.1), q.2, q.3));
            if yf < t && !tree {
                continue;
            }
            let d = yf - t;
            let mut p = if tree || d < thick {
                // Grass: a bright lip on top, darker as it hangs.
                if tree {
                    scale(l.colors.grass, 0.82 + 0.1 * noise2(xf * 0.5, yf * 0.5, s ^ 0x71))
                } else if d < 1.5 && flat > 0.3 {
                    l.colors.grass_top
                } else {
                    scale(l.colors.grass, 1.0 - 0.15 * (d / thick.max(1.0)))
                }
            } else {
                // Rock: faces towards the light brighter (the skyline's
                // slope over a span widening with depth), streaked
                // up and down, a little lighter near the top.
                let span = (2.0 + d * 0.4).min(24.0);
                let face = ((top(xf + span) - top(xf - span)) / span).clamp(-1.0, 1.0);
                let fade = (1.0 - d / (l.height * hf * 0.9 + 1.0)).max(0.0);
                let lit = (-face * fade).max(0.0);
                let mut p = mix(l.colors.rock, l.colors.lit, (lit * 2.5).min(1.0) * 0.9 + 0.1 * fade);
                let streak = noise2(xf * 0.55, yf * 0.035, s ^ 0x5e);
                let fine = noise2(xf * 1.3, yf * 0.12, s ^ 0x5f);
                if streak < 0.3 {
                    p = scale(p, 0.86);
                } else if streak > 0.72 {
                    p = scale(p, 1.06);
                }
                if fine < 0.18 {
                    p = scale(p, 0.93);
                }
                if face > 0.1 {
                    p = scale(p, 1.0 - 0.22 * (face * 2.0).min(1.0) * fade.max(0.3));
                }
                p
            };
            // Haze by distance; mist rising from the foot.
            let mist = l.mist * smoothstep(foot - hf * 0.35, foot, yf);
            p = mix(p, v.haze(), haze + (1.0 - haze) * mist * 0.55);
            write(&mut out, xi, y, w, xa, p);
        }
    }
    out
}

/// Clouds, strip columns `x0 .. x0 + w`, `h` rows, RGBA: chunky, lit from
/// above, their bottoms flat and shaded, in a band across the strip.
pub fn cloud_tile(v: &Vista, x0: i64, w: usize, h: usize, seed: u64) -> Vec<u8> {
    let s = seed ^ 0xc10d;
    let (many, big) = v.clouds;
    let every = 34.0 / many.max(0.05);
    let hf = h as f32;
    // Each cloud: blobs along a line (x, y, r), its flat bottom, its size.
    struct Cloud {
        blobs: Vec<(f32, f32, f32)>,
        bottom: f32,
        top: f32,
        far: f32,
    }
    let reach = 90.0 * big;
    let (lo, hi) = (((x0 as f32 - reach) / every).floor() as i64, ((x0 as f32 + w as f32 + reach) / every).floor() as i64);
    let mut clouds: Vec<Cloud> = Vec::new();
    for k in lo..=hi {
        if hash(k, 1, s) > 0.75 {
            continue;
        }
        let far = hash(k, 7, s);
        let size = big * (0.6 + 0.9 * hash(k, 2, s)) * (1.0 - 0.45 * far);
        let cx = (k as f32 + hash(k, 3, s)) * every;
        // (Farther: higher in the band's middle... nearer: lower, bigger.)
        let cy = hf * (0.3 + 0.55 * hash(k, 4, s));
        let n = 4 + (hash(k, 5, s) * 6.0) as i64;
        let len = size * (34.0 + 40.0 * hash(k, 6, s));
        // A row of puffs along the bottom, bigger ones heaped on top.
        let mut blobs: Vec<(f32, f32, f32)> = (0..n)
            .map(|j| {
                let u = j as f32 / (n - 1) as f32;
                let arch = 1.0 - (2.0 * u - 1.0).powi(2);
                let r = size * (4.0 + 3.0 * hash(k * 31 + j, 8, s));
                (cx + (u - 0.5) * len, cy - r * 0.4 - size * 2.0 * arch, r)
            })
            .collect();
        for j in 0..(n / 2).max(1) {
            let u = (j as f32 + 0.5 + 0.6 * (hash(k * 13 + j, 10, s) - 0.5)) / (n / 2).max(1) as f32;
            let arch = 1.0 - (2.0 * u - 1.0).powi(2);
            let r = size * (6.0 + 7.0 * arch * hash(k * 19 + j, 11, s) + 3.0);
            blobs.push((cx + (u - 0.5) * len * 0.7, cy - r * 0.6 - size * 4.0 * arch, r));
        }
        let bottom = cy + size * 1.5;
        let top = blobs.iter().map(|b| b.1 - b.2).fold(f32::MAX, f32::min);
        clouds.push(Cloud { blobs, bottom, top, far });
    }
    let white = rgb(250, 252, 255);
    let shade = rgb(196, 214, 238);
    let mut out = vec![0u8; w * h * 4];
    for xi in 0..w {
        let xa = x0 + xi as i64;
        let xf = xa as f32;
        for y in 0..h {
            let yf = y as f32;
            // (Nearest cloud drawn last: the first found in front.)
            let Some(c) = clouds.iter().filter(|c| yf <= c.bottom && c.blobs.iter().any(|b| (xf - b.0).powi(2) + (yf - b.1).powi(2) < b.2 * b.2)).min_by(|a, b| a.far.total_cmp(&b.far)) else {
                continue;
            };
            // Lit from above: white at the top, shaded towards the flat
            // bottom; a lighter rim on each blob's upper edge.
            let depth = ((yf - c.top) / (c.bottom - c.top).max(1.0)).clamp(0.0, 1.0);
            let mut p = mix(white, shade, smoothstep(0.35, 0.95, depth));
            // (Its top edge: sky just above.)
            let inside = |x: f32, y: f32| c.blobs.iter().any(|b| (x - b.0).powi(2) + (y - b.1).powi(2) < b.2 * b.2);
            if !inside(xf, yf - 2.0) {
                p = mix(p, white, 0.6);
            }
            if yf > c.bottom - 1.0 {
                p = scale(p, 0.93);
            }
            // Far ones fade into the sky.
            p = mix(p, v.sky.1, 0.15 + 0.45 * c.far);
            write(&mut out, xi, y, w, xa, p);
        }
    }
    out
}

/// One pixel, quantised and dithered by its absolute column.
fn write(out: &mut [u8], x: usize, y: usize, w: usize, xa: i64, p: Rgb) {
    let levels = 40.0;
    let d = bayer(xa.rem_euclid(4) as usize, y) - 0.5;
    let i = (y * w + x) * 4;
    for (c, v) in p.iter().enumerate() {
        out[i + c] = (((v.clamp(0.0, 1.0) * levels + d).round() / levels).clamp(0.0, 1.0) * 255.0) as u8;
    }
    out[i + 3] = 255;
}

/// The vistas to choose from.
pub fn vistas() -> Vec<Vista> {
    let sky = (rgb(46, 92, 214), rgb(134, 180, 246));
    // Terraria's: lilac-grey rock, teal grass; far off, bluer.
    let lilac = Palette { rock: rgb(112, 96, 138), lit: rgb(176, 158, 196), grass: rgb(52, 132, 100), grass_top: rgb(100, 184, 118) };
    let lilac_far = Palette { rock: rgb(118, 122, 176), lit: rgb(158, 164, 212), grass: rgb(88, 150, 150), grass_top: rgb(116, 176, 160) };
    let green = Palette { rock: rgb(46, 116, 82), lit: rgb(72, 146, 96), grass: rgb(50, 128, 84), grass_top: rgb(96, 178, 104) };
    let slate = Palette { rock: rgb(84, 98, 120), lit: rgb(146, 160, 178), grass: rgb(44, 108, 74), grass_top: rgb(86, 158, 92) };
    let slate_far = Palette { rock: rgb(108, 126, 166), lit: rgb(150, 168, 202), grass: rgb(104, 148, 158), grass_top: rgb(128, 170, 172) };
    let sand = Palette { rock: rgb(170, 110, 80), lit: rgb(222, 170, 118), grass: rgb(108, 134, 66), grass_top: rgb(152, 172, 84) };
    let sand_far = Palette { rock: rgb(178, 136, 144), lit: rgb(212, 176, 170), grass: rgb(150, 160, 120), grass_top: rgb(170, 176, 130) };
    let l = |shape, depth, base, height, colors, grass, trees, mist| VLayer { shape, depth, base, height, colors, grass, trees, mist };
    vec![
        // Like the screenshot: far flat-topped mesas, big lilac crags with
        // grass on their tops, low green hills with round trees.
        Vista {
            name: "mesas",
            sky,
            clouds: (1.0, 1.0),
            layers: vec![
                l(Shape::Mesas { width: 120.0, steps: 3.0 }, 0.15, 0.7, 0.5, lilac_far, 0.6, Trees::None, 0.5),
                l(Shape::Peaks { width: 110.0 }, 0.55, 0.8, 0.6, lilac, 1.0, Trees::None, 0.5),
                l(Shape::Hills { width: 120.0 }, 0.85, 0.97, 0.18, green, 1.0, Trees::Round, 0.2),
            ],
        },
        // Far crags; nearer, stepped plateaus; pines in front.
        Vista {
            name: "crags",
            sky,
            clouds: (0.8, 1.1),
            layers: vec![
                l(Shape::Peaks { width: 110.0 }, 0.15, 0.7, 0.55, lilac_far, 0.4, Trees::None, 0.5),
                l(Shape::Mesas { width: 90.0, steps: 3.0 }, 0.55, 0.8, 0.6, lilac, 1.0, Trees::None, 0.5),
                l(Shape::Hills { width: 90.0 }, 0.85, 0.97, 0.16, green, 1.0, Trees::Pine, 0.2),
            ],
        },
        // Cold blue-grey ranges, pines.
        Vista {
            name: "highlands",
            sky: (rgb(56, 104, 206), rgb(160, 196, 238)),
            clouds: (1.2, 0.9),
            layers: vec![
                l(Shape::Peaks { width: 130.0 }, 0.1, 0.7, 0.6, slate_far, 0.0, Trees::None, 0.6),
                l(Shape::Peaks { width: 100.0 }, 0.5, 0.8, 0.62, slate, 0.7, Trees::None, 0.6),
                l(Shape::Hills { width: 110.0 }, 0.85, 0.97, 0.18, green, 1.0, Trees::Pine, 0.2),
            ],
        },
        // Warm sandstone steps (a desert's?).
        Vista {
            name: "canyons",
            sky: (rgb(62, 112, 220), rgb(176, 204, 242)),
            clouds: (0.6, 1.0),
            layers: vec![
                l(Shape::Mesas { width: 120.0, steps: 4.0 }, 0.15, 0.7, 0.5, sand_far, 0.0, Trees::None, 0.5),
                l(Shape::Mesas { width: 90.0, steps: 3.0 }, 0.55, 0.8, 0.6, sand, 0.5, Trees::None, 0.5),
                l(Shape::Hills { width: 100.0 }, 0.85, 0.97, 0.16, sand, 0.8, Trees::None, 0.2),
            ],
        },
        // No rock: green hills over green hills, woods.
        Vista {
            name: "rolling",
            sky,
            clouds: (1.4, 1.1),
            layers: vec![
                l(Shape::Hills { width: 120.0 }, 0.1, 0.7, 0.45, lilac_far, 0.8, Trees::Round, 0.5),
                l(Shape::Hills { width: 100.0 }, 0.5, 0.8, 0.45, green, 1.0, Trees::Round, 0.5),
                l(Shape::Hills { width: 110.0 }, 0.85, 0.97, 0.18, green, 1.0, Trees::Round, 0.2),
            ],
        },
        // Stone columns rising out of misty hills.
        Vista {
            name: "spires",
            sky: (rgb(52, 100, 212), rgb(166, 198, 244)),
            clouds: (1.0, 1.2),
            layers: vec![
                l(Shape::Spires { every: 80.0 }, 0.12, 0.72, 0.7, lilac_far, 0.6, Trees::None, 0.7),
                l(Shape::Spires { every: 64.0 }, 0.55, 0.8, 0.7, lilac, 1.0, Trees::None, 0.7),
                l(Shape::Hills { width: 110.0 }, 0.85, 0.97, 0.18, green, 1.0, Trees::Round, 0.2),
            ],
        },
    ]
}

/// Which vista a biome (the world plan's name) shows behind it.
pub fn for_biome(biome: &str) -> &'static str {
    match biome {
        "mountains" => "crags",
        "tundra" => "highlands",
        "desert" => "canyons",
        "jungle" | "swamp" => "rolling",
        // forest, plains, deep forest, ocean
        _ => "mesas",
    }
}

/// A vista as the game would show it at `x`: sky, sun, the far layer,
/// clouds, the nearer layers, over a strip of ground at the bottom (for
/// scale), `w`×`h` art pixels (cells), RGBA.
pub fn still(v: &Vista, x: i64, w: usize, h: usize, seed: u64) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for xi in 0..w {
            let t = y as f32 / h as f32;
            let p = mix(v.sky.0, v.sky.1, smoothstep(0.0, 0.75, t));
            let d = bayer(xi, y) - 0.5;
            out.extend(p.map(|c| (((c * 40.0 + d).round() / 40.0).clamp(0.0, 1.0) * 255.0) as u8));
            out.push(255);
        }
    }
    let over = |out: &mut [u8], px: &[u8], oy: usize, oh: usize| {
        for y in 0..oh {
            for xi in 0..w {
                let (i, j) = (((y + oy) * w + xi) * 4, (y * w + xi) * 4);
                if y + oy < h && px[j + 3] > 0 {
                    out[i..i + 4].copy_from_slice(&px[j..j + 4]);
                }
            }
        }
    };
    // The sun: a disc and a wide soft halo.
    let (sx, sy) = (w as f32 * 0.68, h as f32 * 0.14);
    for y in 0..h {
        for xi in 0..w {
            let r = ((xi as f32 - sx).powi(2) + (y as f32 - sy).powi(2)).sqrt();
            let i = (y * w + xi) * 4;
            let a = if r < 7.0 { 1.0 } else { (1.0 - (r - 7.0) / 30.0).max(0.0).powi(2) * 0.55 };
            let sun = [255.0, 248.0, 220.0];
            for c in 0..3 {
                out[i + c] = (out[i + c] as f32 + (sun[c] - out[i + c] as f32) * a) as u8;
            }
        }
    }
    // Each layer's foot a little lower than the one behind it; the clouds
    // over the far layer, behind the nearer ones.
    let lh = (h as f32 * 0.72) as usize;
    for k in 0..v.layers.len() {
        let foot = h as f32 * [0.56, 0.7, 0.8][k.min(2)];
        let oy = (foot - v.layers[k].base * lh as f32).max(0.0) as usize;
        over(&mut out, &layer_tile(v, k, x, w, lh, seed), oy, lh);
        if k == 0 {
            over(&mut out, &cloud_tile(v, x, w, h / 2, seed), 0, h / 2);
        }
    }
    // The world's ground, for scale.
    for xi in 0..w {
        let g = (h as f32 * 0.8 + 6.0 * (noise1((x + xi as i64) as f32 * 0.03, 5) - 0.5)) as usize;
        for y in g.min(h)..h {
            let i = (y * w + xi) * 4;
            let c = if y < g + 2 { [70, 150, 60] } else if y < g + 14 { [112, 78, 52] } else { [40, 30, 26] };
            out[i..i + 3].copy_from_slice(&c);
        }
    }
    out
}

/// A vista at night: everything darker and bluer (roughly as the game's
/// lighting grades it), for a look at the silhouettes.
pub fn night(px: &mut [u8]) {
    for p in px.chunks_mut(4) {
        let g = [0.16, 0.2, 0.34];
        for c in 0..3 {
            p[c] = (p[c] as f32 * g[c]) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vista_tiles_join_without_a_seam() {
        let v = &vistas()[0];
        let (w, h) = (80, 90);
        for k in 0..v.layers.len() {
            let whole = layer_tile(v, k, 0, 2 * w, h, 3);
            let right = layer_tile(v, k, w as i64, w, h, 3);
            for y in 0..h {
                for x in 0..w {
                    assert_eq!(&whole[(y * 2 * w + w + x) * 4..][..4], &right[(y * w + x) * 4..][..4], "layer {k}: ({x}, {y})");
                }
            }
        }
        let whole = cloud_tile(v, 0, 2 * w, h, 3);
        let right = cloud_tile(v, w as i64, w, h, 3);
        assert!(whole.chunks(4).skip(w).step_by(2 * w).count() > 0);
        for y in 0..h {
            for x in 0..w {
                assert_eq!(&whole[(y * 2 * w + w + x) * 4..][..4], &right[(y * w + x) * 4..][..4], "clouds: ({x}, {y})");
            }
        }
    }
}
