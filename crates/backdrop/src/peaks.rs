//! The surface's backdrop, Noita-like: tall sharp peaks cut into flat
//! faces (lit from the left, shade on the right, snow on the tops), range
//! behind range fading into the sky, big billowing clouds rising behind
//! them, a dark lowland at their feet. A look is a sky, clouds and layers
//! far to near; each layer is drawn a tile at a time from absolute
//! coordinates (tiles side by side join up), in daylight colours (the
//! game's lighting grades them).

use crate::{Rgb, hash, mix, noise1, noise2, rgb};

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    /// Peaks: one every `every` cells (most slots), `size` their heights
    /// (shares of the strip), `slope` how wide they spread a row down
    /// (cells; sides differ).
    Peaks { every: f32, size: (f32, f32), slope: (f32, f32) },
}

#[derive(Clone, Copy, Debug)]
pub struct Layer {
    pub kind: Kind,
    /// The feet (the base row, a share of the strip from its top). Below
    /// them the range runs on down to the strip's bottom: its flanks
    /// steepen into shoulders, the gaps between filled with its body.
    pub base: f32,
    /// Faces towards the light, and away.
    pub lit: Rgb,
    pub shade: Rgb,
    /// Snow on the tops: how far down (a share of each peak's height; 0
    /// none).
    pub snow: f32,
    /// How much the sky's haze fades it; mist rising at its feet.
    pub haze: f32,
    pub mist: f32,
}

#[derive(Clone, Debug)]
pub struct Look {
    pub name: &'static str,
    /// The sky, overhead and at the horizon.
    pub sky: (Rgb, Rgb),
    /// Clouds: how many, how big, lit and shaded.
    pub clouds: (f32, f32, Rgb, Rgb),
    pub layers: Vec<Layer>,
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A peak: apex (x, row), height, spread each side, how it sits in front
/// of the others, its seed.
#[derive(Clone, Copy)]
struct Peak {
    x: f32,
    y: f32,
    tall: f32,
    left: f32,
    right: f32,
    front: f32,
    seed: u64,
}

impl Peak {
    /// Its edges at row `y` (jagged), or `None` above its apex.
    fn span(&self, y: f32) -> Option<(f32, f32)> {
        let d = y - self.y;
        if d < 0.0 {
            return None;
        }
        // Concave flanks (steep near the summit, flaring down to the
        // feet, then steepening again: the range runs on down), rough
        // edges: a wander that grows downhill, shoulders, a small crag.
        let spread = if d < self.tall { self.tall * (d / self.tall).powf(1.35) } else { self.tall + (d - self.tall) * 0.6 };
        // (The wander stops growing past the feet: shoulders run straight
        // on down.)
        let g = d.min(self.tall * 1.1);
        let jag = |side: u64| {
            (noise1(d * 0.04, self.seed ^ side) - 0.5) * g * 0.16 + (noise1(d * 0.13, self.seed ^ side ^ 0x33) - 0.5) * g * 0.1 + (noise1(d * 0.4, self.seed ^ side ^ 0x55) - 0.5) * 1.5
        };
        Some((self.x - spread * self.left + jag(1), self.x + spread * self.right + jag(2)))
    }
}

/// The peaks of layer `l` whose spread may reach columns `x0 .. x1`.
fn peaks(l: &Layer, k: usize, x0: f32, x1: f32, h: f32, seed: u64) -> Vec<Peak> {
    let Kind::Peaks { every, size, slope } = l.kind;
    let s = seed.wrapping_add(k as u64 * 9173 + 5);
    let reach = (size.1 * h + (1.0 - l.base) * h * 0.6) * slope.1 * 1.3 + every;
    let mut out = Vec::new();
    for slot in ((x0 - reach) / every).floor() as i64..=((x1 + reach) / every).floor() as i64 {
        // A main peak most slots, a lower shoulder beside it now and then,
        // and low broad foothills in front (the valleys between hold small
        // mountains, not a floor).
        for (part, chance, scale_h, wide) in [(0i64, 0.85, 1.0, 1.0), (1, 0.55, 0.55, 1.0), (2, 0.8, 0.28, 1.7)] {
            let q = slot * 3 + part;
            if hash(q, 1, s) > chance {
                continue;
            }
            let tall = (size.0 + (size.1 - size.0) * hash(q, 2, s).powf(1.4)) * h * scale_h;
            let x = (slot as f32 + 0.1 + 0.8 * hash(q, 3, s)) * every + if part > 0 { (hash(q, 7, s) - 0.5) * every } else { 0.0 };
            let (a, b) = (slope.0 + (slope.1 - slope.0) * hash(q, 4, s), slope.0 + (slope.1 - slope.0) * hash(q, 5, s));
            out.push(Peak { x, y: l.base * h - tall, tall, left: a * wide, right: b * wide, front: hash(q, 6, s) + part as f32, seed: s ^ (q as u64).wrapping_mul(0x9E37_79B9) });
        }
    }
    out
}

/// Layer `k` of `look`, strip columns `x0 .. x0 + w`, `h` rows, RGBA (clear
/// above the skyline).
pub fn layer_tile(look: &Look, k: usize, x0: i64, w: usize, h: usize, seed: u64) -> Vec<u8> {
    let l = &look.layers[k];
    let hf = h as f32;
    let foot = l.base * hf;
    let haze = look.sky.1;
    let fade = |p: Rgb, y: f32| {
        let mist = l.mist * smoothstep(foot - hf * 0.15, foot + hf * 0.1, y);
        mix(p, haze, l.haze + (1.0 - l.haze) * mist * 0.7)
    };
    // The range's body (between its shoulders, and at the very bottom,
    // which the game stretches down).
    let body = |y: f32| fade(mix(l.shade, l.lit, 0.3), y);
    let last = h.saturating_sub(2) as f32;
    let l_seed = seed.wrapping_add(k as u64 * 9173 + 5);
    let mut out = vec![0u8; w * h * 4];
    let all = peaks(l, k, x0 as f32, (x0 + w as i64) as f32, hf, seed);
    for xi in 0..w {
        let xa = x0 + xi as i64;
        let xf = xa as f32 + 0.5;
        // The peaks this column may cross (as wide as they get at the
        // strip's bottom), frontmost first.
        let widest = |p: &Peak| (p.tall + (hf - p.y - p.tall).max(0.0) * 0.6) * 1.3 + 4.0;
        let mut here: Vec<&Peak> = all.iter().filter(|p| xf > p.x - widest(p) * p.left && xf < p.x + widest(p) * p.right).collect();
        here.sort_by(|a, b| b.front.total_cmp(&a.front));
        // The body's rolling edge a little above the feet, and its slope.
        let roll = |x: f32| foot - hf * 0.06 * (0.15 + 0.6 * noise1(x / 60.0, l_seed ^ 0x70) + 0.3 * noise1(x / 17.0, l_seed ^ 0x71));
        let rolling = roll(xf);
        let slope_r = roll(xf + 2.0) - roll(xf - 2.0);
        for y in 0..h {
            let yf = y as f32 + 0.5;
            let p = if yf >= last {
                body(foot + hf)
            } else if let Some((pk, (a, b))) = here.iter().find_map(|pk| pk.span(yf).filter(|(a, b)| xf >= *a && xf < *b).map(|sp| (*pk, sp))) {
                let d = yf - pk.y;
                // Faces: wedges from the apex. Where across the peak this
                // is (-1 left edge .. 1 right edge), the main ridge
                // wandering a little from the apex down.
                let ridge = pk.x + (noise1(d * 0.02, pk.seed ^ 3) - 0.5) * d * 0.2;
                let u = if xf < ridge { -(ridge - xf) / (ridge - a).max(1.0) } else { (xf - ridge) / (b - ridge).max(1.0) };
                let facets = 3.0;
                let wobble = (noise1(d * 0.025, pk.seed ^ 9) - 0.5) * 0.14;
                let f = ((u + wobble) * facets).floor();
                // Lit on the left (brighter nearer the ridge), shaded on
                // the right, each face a touch different.
                let jitter = (hash(f as i64, 11, pk.seed) - 0.5) * 0.16;
                let amount = if u < 0.0 { 0.95 - 0.12 * (-f - 1.0) + jitter } else { 0.28 - 0.1 * f + jitter };
                let mut c = mix(l.shade, l.lit, amount.clamp(0.0, 1.0));
                // Snow on the top, ragged, reaching further down the
                // gullies (the faces' edges).
                if l.snow > 0.0 {
                    let edge = 1.0 - ((u + wobble) * facets).fract().min(1.0 - ((u + wobble) * facets).fract()) * 2.0;
                    let line = pk.tall * l.snow * (0.65 + 0.35 * noise1(xf * 0.12, pk.seed ^ 5) + 0.4 * edge + 0.15 * (noise2(xf * 0.15, yf * 0.1, pk.seed ^ 6) - 0.5));
                    if d < line {
                        let white = if u < 0.0 { rgb(240, 246, 252) } else { rgb(170, 190, 222) };
                        c = mix(c, white, 0.85);
                    }
                }
                fade(c, yf)
            } else if yf > rolling {
                // Below the feet, between the shoulders: the body, under a
                // rolling edge (no straight line across the valleys), its
                // faces towards the light a little brighter near the top.
                let lit = if slope_r > 0.15 && yf - rolling < 10.0 { 0.12 } else { 0.0 };
                fade(mix(mix(l.shade, l.lit, 0.3), l.lit, lit), yf)
            } else {
                continue;
            };
            write(&mut out, xi, y, w, xa, p);
        }
    }
    out
}

/// Clouds, strip columns `x0 .. x0 + w`, `h` rows, RGBA: big billowing
/// heaps, lit from the upper left, their lower parts shaded blue, meant
/// to rise from behind a range.
pub fn cloud_tile(look: &Look, x0: i64, w: usize, h: usize, seed: u64) -> Vec<u8> {
    let s = seed ^ 0xc10d5;
    let (many, big, lit, shade) = look.clouds;
    let every = 150.0 / many.max(0.05);
    let hf = h as f32;
    // Blobs (x, y, r), heaped: a broad bottom row, smaller ones stacked
    // on top towards the middle.
    let reach = 140.0 * big;
    let mut blobs: Vec<(f32, f32, f32, u64, f32)> = Vec::new();
    for k in ((x0 as f32 - reach) / every).floor() as i64..=((x0 as f32 + w as f32 + reach) / every).floor() as i64 {
        if hash(k, 1, s) > 0.8 {
            continue;
        }
        let size = big * (0.6 + 0.8 * hash(k, 2, s));
        let cx = (k as f32 + 0.2 + 0.6 * hash(k, 3, s)) * every;
        let bottom = hf * (0.98 - 0.08 * hash(k, 4, s));
        let wide = size * (70.0 + 60.0 * hash(k, 5, s));
        let n = 5 + (hash(k, 6, s) * 6.0) as i64;
        for j in 0..n {
            let u = (j as f32 + 0.5) / n as f32;
            let r = size * (16.0 + 10.0 * hash(k * 37 + j, 7, s));
            blobs.push((cx + (u - 0.5) * wide, bottom - r * 0.45 + (hash(k * 41 + j, 12, s) - 0.5) * r * 0.3, r, (k * 37 + j) as u64, bottom));
        }
        // The heap: rows narrowing upward.
        for row in 1..=2 {
            let m = (n - row * 2).max(1);
            for j in 0..m {
                let u = (j as f32 + 0.5) / m as f32;
                let q = k * 101 + row * 13 + j;
                let r = size * (14.0 + 12.0 * hash(q, 8, s)) * (1.0 - 0.12 * row as f32);
                let x = cx + (u - 0.5) * wide * (1.0 - 0.28 * row as f32) + (hash(q, 9, s) - 0.5) * 12.0 * size;
                blobs.push((x, bottom - r * 0.6 - row as f32 * size * 13.0 * (0.8 + 0.3 * hash(q, 10, s)), r, q as u64, bottom));
            }
        }
    }
    let mut out = vec![0u8; w * h * 4];
    for xi in 0..w {
        let xa = x0 + xi as i64;
        let xf = xa as f32 + 0.5;
        let col: Vec<&(f32, f32, f32, u64, f32)> = blobs.iter().filter(|b| (xf - b.0).abs() < b.2 * 1.15).collect();
        if col.is_empty() {
            continue;
        }
        for y in 0..h {
            let yf = y as f32 + 0.5;
            // The blob this pixel is in, the lowest (nearest) in front; its
            // outline wobbles.
            // (A heap's bottom is flat.)
            let inside = |b: &&&(f32, f32, f32, u64, f32)| {
                let (dx, dy) = (xf - b.0, yf - b.1);
                if yf > b.4 {
                    return false;
                }
                let a = dy.atan2(dx);
                let r = b.2 * (1.0 + 0.1 * (noise1(a * 2.5 + b.3 as f32, s ^ 4) - 0.5));
                dx * dx + dy * dy < r * r
            };
            let Some(b) = col.iter().filter(inside).max_by(|a, b| a.1.total_cmp(&b.1)) else { continue };
            // Shaded by the heap as a whole: bright under its top edge here,
            // bluer further down; a little by the puff (lit from the upper
            // left), for roundness without each puff's outline.
            let top = col.iter().filter(|c| c.4 == b.4).map(|c| c.1 - (c.2 * c.2 - (xf - c.0).powi(2)).max(0.0).sqrt()).fold(f32::MAX, f32::min);
            let depth = smoothstep(0.0, (b.4 - top).max(8.0) * 0.9, yf - top);
            let (dx, dy) = ((xf - b.0) / b.2, (yf - b.1) / b.2);
            let puff = smoothstep(-0.5, 1.0, dx * 0.5 + dy * 0.85);
            let t = (0.65 * depth + 0.35 * puff).clamp(0.0, 1.0);
            let c = mix(lit, shade, t);
            // Its bottom thins into the air (hidden behind a range, mostly).
            let a = smoothstep(b.4, b.4 - 10.0, yf);
            write(&mut out, xi, y, w, xa, mix(c, look.sky.1, 0.12));
            out[(y * w + xi) * 4 + 3] = (a * 255.0) as u8;
        }
    }
    out
}

/// One pixel, flat shapes in fine steps, lightly dithered (by its
/// absolute column: tiles join) so gradients don't band.
fn write(out: &mut [u8], x: usize, y: usize, w: usize, xa: i64, p: Rgb) {
    let levels = 64.0;
    let d = (crate::bayer(xa.rem_euclid(4) as usize, y) - 0.5) * 0.8;
    let i = (y * w + x) * 4;
    for (c, v) in p.iter().enumerate() {
        out[i + c] = (((v.clamp(0.0, 1.0) * levels + d).round() / levels).clamp(0.0, 1.0) * 255.0) as u8;
    }
    out[i + 3] = 255;
}

/// The looks to choose from.
pub fn looks() -> Vec<Look> {
    let sky = (rgb(70, 118, 206), rgb(150, 188, 232));
    let clouds = (1.0, 1.0, rgb(246, 248, 255), rgb(150, 172, 214));
    let l = |kind, base, lit, shade, snow, haze, mist| Layer { kind, base, lit, shade, snow, haze, mist };
    let peaks = |every, lo, hi, s0, s1| Kind::Peaks { every, size: (lo, hi), slope: (s0, s1) };
    vec![
        // As the screenshot: pale snowy ranges, a deep blue-violet one in
        // front.
        Look {
            name: "noita",
            sky,
            clouds,
            layers: vec![
                l(peaks(90.0, 0.070, 0.150, 0.9, 1.6), 0.5, rgb(196, 212, 236), rgb(128, 150, 200), 0.35, 0.45, 0.6),
                l(peaks(130.0, 0.095, 0.220, 0.8, 1.5), 0.5, rgb(206, 220, 242), rgb(104, 126, 190), 0.3, 0.12, 0.5),
                l(peaks(220.0, 0.085, 0.235, 0.9, 1.7), 0.5, rgb(116, 128, 196), rgb(58, 66, 136), 0.0, 0.0, 0.25),
            ],
        },
        // Whiter, snowier, a paler sky.
        Look {
            name: "alpine",
            sky: (rgb(84, 140, 220), rgb(178, 208, 240)),
            clouds: (0.8, 1.1, rgb(250, 252, 255), rgb(166, 186, 222)),
            layers: vec![
                l(peaks(80.0, 0.085, 0.165, 0.8, 1.4), 0.5, rgb(214, 226, 244), rgb(146, 166, 210), 0.55, 0.45, 0.6),
                l(peaks(120.0, 0.110, 0.235, 0.7, 1.3), 0.5, rgb(226, 234, 248), rgb(120, 142, 198), 0.5, 0.12, 0.5),
                l(peaks(200.0, 0.070, 0.165, 1.0, 1.8), 0.5, rgb(112, 136, 170), rgb(64, 82, 118), 0.25, 0.0, 0.3),
            ],
        },
        // Layer on layer fading into mist: four ranges, low contrast.
        Look {
            name: "misty",
            sky: (rgb(92, 128, 196), rgb(176, 196, 226)),
            clouds: (1.2, 0.9, rgb(236, 240, 250), rgb(162, 178, 212)),
            layers: vec![
                l(peaks(70.0, 0.070, 0.140, 0.9, 1.5), 0.5, rgb(178, 196, 226), rgb(146, 164, 208), 0.25, 0.55, 0.9),
                l(peaks(90.0, 0.070, 0.150, 0.9, 1.5), 0.5, rgb(160, 180, 220), rgb(122, 142, 196), 0.2, 0.4, 0.9),
                l(peaks(120.0, 0.085, 0.180, 0.8, 1.5), 0.5, rgb(140, 160, 212), rgb(98, 116, 180), 0.15, 0.22, 0.9),
                l(peaks(170.0, 0.085, 0.195, 0.9, 1.6), 0.5, rgb(108, 124, 186), rgb(70, 82, 146), 0.0, 0.05, 0.8),
            ],
        },
        // Warmer: violet and rose ranges (a stranger place).
        Look {
            name: "violet",
            sky: (rgb(86, 104, 196), rgb(196, 176, 222)),
            clouds: (1.0, 1.0, rgb(252, 240, 248), rgb(180, 150, 200)),
            layers: vec![
                l(peaks(90.0, 0.070, 0.150, 0.9, 1.6), 0.5, rgb(222, 196, 226), rgb(160, 132, 196), 0.3, 0.45, 0.6),
                l(peaks(130.0, 0.095, 0.220, 0.8, 1.5), 0.5, rgb(226, 204, 232), rgb(128, 102, 176), 0.25, 0.12, 0.5),
                l(peaks(220.0, 0.085, 0.235, 0.9, 1.7), 0.5, rgb(140, 102, 176), rgb(76, 52, 120), 0.0, 0.0, 0.25),
            ],
        },
        // Needles: narrow, steep, crowded.
        Look {
            name: "needles",
            sky,
            clouds,
            layers: vec![
                l(peaks(45.0, 0.085, 0.165, 0.35, 0.7), 0.5, rgb(196, 212, 236), rgb(128, 150, 200), 0.3, 0.45, 0.6),
                l(peaks(70.0, 0.110, 0.235, 0.3, 0.65), 0.5, rgb(206, 220, 242), rgb(104, 126, 190), 0.25, 0.12, 0.5),
                l(peaks(120.0, 0.095, 0.250, 0.35, 0.8), 0.5, rgb(116, 128, 196), rgb(58, 66, 136), 0.0, 0.0, 0.25),
            ],
        },
        // Low and broad: old worn ranges, big skies, more cloud.
        Look {
            name: "broad",
            sky: (rgb(64, 112, 204), rgb(150, 190, 236)),
            clouds: (1.4, 1.2, rgb(246, 248, 255), rgb(150, 172, 214)),
            layers: vec![
                l(peaks(120.0, 0.040, 0.095, 1.6, 2.8), 0.5, rgb(188, 206, 234), rgb(130, 152, 204), 0.3, 0.45, 0.6),
                l(peaks(170.0, 0.055, 0.125, 1.4, 2.6), 0.5, rgb(176, 196, 232), rgb(100, 122, 186), 0.2, 0.15, 0.5),
                l(peaks(260.0, 0.040, 0.110, 1.8, 3.0), 0.5, rgb(96, 118, 170), rgb(56, 70, 124), 0.0, 0.0, 0.3),
            ],
        },
    ]
}

/// Which look a biome (the world plan's name) shows behind it. (`violet`,
/// `misty`, `needles` and `broad` wait for places of their own.)
pub fn for_biome(biome: &str) -> &'static str {
    match biome {
        "mountains" | "tundra" => "alpine",
        _ => "noita",
    }
}

/// A look as the game would show it at `x`: sky, the far layers, the
/// clouds rising behind the middle range, the near layers, a strip of the
/// world's ground (for scale); `w`×`h` art pixels (cells), RGBA.
pub fn still(look: &Look, x: i64, w: usize, h: usize, seed: u64) -> Vec<u8> {
    still_masked(look, x, w, h, seed).0
}

/// `still`, and which of its pixels are open sky (where the sun, moon and
/// stars would show).
pub fn still_masked(look: &Look, x: i64, w: usize, h: usize, seed: u64) -> (Vec<u8>, Vec<bool>) {
    let mut out: Vec<u8> = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        let p = mix(look.sky.0, look.sky.1, smoothstep(0.0, 0.7, y as f32 / h as f32));
        for _ in 0..w {
            out.extend(p.map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8));
            // (254: sky, for the mask.)
            out.push(254);
        }
    }
    let over = |out: &mut [u8], px: &[u8], oy: i64, oh: usize| {
        for y in 0..oh {
            let yy = y as i64 + oy;
            if !(0..h as i64).contains(&yy) {
                continue;
            }
            for xi in 0..w {
                let (i, j) = ((yy as usize * w + xi) * 4, (y * w + xi) * 4);
                if px[j + 3] > 0 {
                    out[i..i + 4].copy_from_slice(&px[j..j + 4]);
                }
            }
        }
    };
    // Strips as tall as the view; each layer's feet where its base puts
    // them over the ground line, the clouds behind all but the farthest.
    let lh = 2 * h;
    let ground = h as f32 * 0.8;
    let n = look.layers.len();
    for k in 0..n {
        // (Feet from a little above the ground, far, down to it, near, as
        // the game stacks them.)
        let foot = ground - (n - 1 - k) as f32 * 12.0;
        let oy = (foot - look.layers[k].base * lh as f32) as i64;
        over(&mut out, &layer_tile(look, k, x, w, lh, seed), oy, lh);
        if k == 0 {
            let ch = (h as f32 * 0.6) as usize;
            over(&mut out, &cloud_tile(look, x, w, ch, seed), (ground - h as f32 * 0.12) as i64 - ch as i64, ch);
        }
    }
    for xi in 0..w {
        let g = (ground + 6.0 * (noise1((x + xi as i64) as f32 * 0.03, 5) - 0.5)) as usize;
        for y in g.min(h)..h {
            let i = (y * w + xi) * 4;
            let c = if y < g + 2 { [70, 150, 60] } else if y < g + 14 { [112, 78, 52] } else { [40, 30, 26] };
            out[i..i + 3].copy_from_slice(&c);
            out[i + 3] = 255;
        }
    }
    // (The sky was left at 254: everything drawn over it is 255.)
    let sky = out.chunks(4).map(|p| p[3] == 254).collect();
    for p in out.chunks_mut(4) {
        p[3] = 255;
    }
    (out, sky)
}

/// Roughly as the game's lighting grades a still: `light` multiplies
/// everything.
pub fn grade(px: &mut [u8], light: Rgb) {
    for p in px.chunks_mut(4) {
        for c in 0..3 {
            p[c] = (p[c] as f32 * light[c]).min(255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_tiles_join_without_a_seam() {
        let look = &looks()[0];
        let (w, h) = (80, 120);
        for k in 0..look.layers.len() {
            let whole = layer_tile(look, k, 0, 2 * w, h, 3);
            let right = layer_tile(look, k, w as i64, w, h, 3);
            for y in 0..h {
                for x in 0..w {
                    assert_eq!(&whole[(y * 2 * w + w + x) * 4..][..4], &right[(y * w + x) * 4..][..4], "layer {k}: ({x}, {y})");
                }
            }
        }
        let whole = cloud_tile(look, 0, 2 * w, h, 3);
        let right = cloud_tile(look, w as i64, w, h, 3);
        for y in 0..h {
            for x in 0..w {
                assert_eq!(&whole[(y * 2 * w + w + x) * 4..][..4], &right[(y * w + x) * 4..][..4], "clouds: ({x}, {y})");
            }
        }
    }
}
