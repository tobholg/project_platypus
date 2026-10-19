//! The surface's backdrop: ranges of mountains, range behind range fading
//! into the sky, big billowing clouds rising behind them, a dark lowland
//! at their feet. A look is a sky, clouds and layers far to near; each
//! layer is terrain (`terrain`: a heightfield seen from the front, spurs
//! and gullies, snow up high), drawn a tile at a time from absolute
//! coordinates (tiles side by side join up) as what each pixel is, for the
//! light of the hour to colour (`layer_data`; the game's `backdrop.wgsl`);
//! `layer_tile` shades it at noon (stills).

use crate::terrain::{self, Albedo, Light, Shape};
use crate::{Rgb, bayer, hash, mix, noise1, rgb};

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    /// Peaks: summits about `every` units apart, `size` their heights where
    /// the range sinks and where it rises highest (shares of the strip),
    /// `slope` how broad they spread (the shapes' scale across: wider,
    /// gentler).
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

/// Layer `k`'s terrain, for a strip `hf` units tall.
pub fn shape(look: &Look, k: usize, hf: f32, seed: u64) -> Shape {
    let l = &look.layers[k];
    let Kind::Peaks { every, size, slope } = l.kind;
    let (deep, tilt) = (every * 1.3, 0.06);
    // (Never past the strip's top: its tallest and its back's rise within
    // the room over its feet.)
    let amp = (size.1 * hf * 1.1).min(l.base * hf * 0.9 - deep * tilt);
    Shape {
        scale: every * (slope.0 + slope.1) * 0.5 * 1.1,
        amp,
        low: (size.0 / size.1.max(1e-3)).clamp(0.05, 1.0),
        deep,
        tilt,
        snow: l.snow,
        seed: seed.wrapping_add(k as u64 * 9173 + 5),
    }
}

/// What layer `k` is made of: rock a little darker than its lit colour
/// (lit full on, it shows as that), snow, scrub as its shade; its haze.
pub fn albedo(look: &Look, k: usize) -> Albedo {
    let l = &look.layers[k];
    Albedo { rock: l.lit.map(|v| v * 0.85), snow: [0.8, 0.82, 0.86], scrub: l.shade.map(|v| v * 0.9), haze: l.haze, mist: l.mist }
}

/// How far into its feet' mist a row is (`y` rows from the strip's top).
pub fn mist(look: &Look, k: usize, y: f32, h: f32) -> f32 {
    let foot = look.layers[k].base * h;
    smoothstep(foot - h * 0.15, foot + h * 0.1, y)
}

/// Layer `k` of `look`, strip columns `x0 .. x0 + w`, `h` rows: texels
/// (`terrain::encode`; clear above the skyline). `detail`: pixels per unit
/// of the shapes: 1.5 draws the same ranges 1.5x as fine.
pub fn layer_data(look: &Look, k: usize, x0: i64, w: usize, h: usize, detail: f32, seed: u64) -> Vec<u8> {
    let hf = h as f32 / detail;
    terrain::strip(&shape(look, k, hf, seed), x0, w, h, look.layers[k].base * hf, detail)
}

/// Layer `k` lit by `light`, RGBA (as the game shades it, before its
/// overlay grades it).
pub fn layer_lit(look: &Look, k: usize, x0: i64, w: usize, h: usize, detail: f32, seed: u64, light: &Light) -> Vec<u8> {
    let data = layer_data(look, k, x0, w, h, detail, seed);
    let a = albedo(look, k);
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        let m = mist(look, k, y as f32, h as f32);
        for x in 0..w {
            let i = (y * w + x) * 4;
            let Some(t) = terrain::decode(&data[i..i + 4]) else { continue };
            let d = bayer((x0 + x as i64).rem_euclid(4) as usize, y);
            let c = terrain::shade(&t, &a, light, d, m);
            for (j, v) in c.iter().enumerate() {
                out[i + j] = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            out[i + 3] = 255;
        }
    }
    out
}

/// Layer `k` of `look` at noon, RGBA (stills).
pub fn layer_tile(look: &Look, k: usize, x0: i64, w: usize, h: usize, detail: f32, seed: u64) -> Vec<u8> {
    layer_lit(look, k, x0, w, h, detail, seed, &terrain::noon(look.sky.1))
}

/// Clouds, strip columns `x0 .. x0 + w`, `h` rows, RGBA: big billowing
/// heaps, lit from the upper left, their lower parts shaded blue, meant
/// to rise from behind a range. (`detail` as `layer_tile`'s.)
pub fn cloud_tile(look: &Look, x0: i64, w: usize, h: usize, detail: f32, seed: u64) -> Vec<u8> {
    let s = seed ^ 0xc10d5;
    let (many, big, lit, shade) = look.clouds;
    let every = 150.0 / many.max(0.05);
    let hf = h as f32 / detail;
    let (xs, xe) = (x0 as f32 / detail, (x0 + w as i64) as f32 / detail);
    // Blobs (x, y, r), heaped: a broad bottom row, smaller ones stacked
    // on top towards the middle.
    let reach = 140.0 * big;
    let mut blobs: Vec<(f32, f32, f32, u64, f32)> = Vec::new();
    for k in ((xs - reach) / every).floor() as i64..=((xe + reach) / every).floor() as i64 {
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
        let xf = (xa as f32 + 0.5) / detail;
        let col: Vec<&(f32, f32, f32, u64, f32)> = blobs.iter().filter(|b| (xf - b.0).abs() < b.2 * 1.15).collect();
        if col.is_empty() {
            continue;
        }
        for y in 0..h {
            let yf = (y as f32 + 0.5) / detail;
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
    still_lit(look, x, w, h, seed, None)
}

/// `still_masked` in `light` (noon if none).
pub fn still_lit(look: &Look, x: i64, w: usize, h: usize, seed: u64, light: Option<&Light>) -> (Vec<u8>, Vec<bool>) {
    let noon = terrain::noon(look.sky.1);
    let light = light.unwrap_or(&noon);
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
        over(&mut out, &layer_lit(look, k, x, w, lh, 1.0, seed, light), oy, lh);
        if k == 0 {
            let ch = (h as f32 * 0.6) as usize;
            over(&mut out, &cloud_tile(look, x, w, ch, 1.0, seed), (ground - h as f32 * 0.12) as i64 - ch as i64, ch);
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
            let whole = layer_tile(look, k, 0, 2 * w, h, 1.5, 3);
            let right = layer_tile(look, k, w as i64, w, h, 1.5, 3);
            for y in 0..h {
                for x in 0..w {
                    assert_eq!(&whole[(y * 2 * w + w + x) * 4..][..4], &right[(y * w + x) * 4..][..4], "layer {k}: ({x}, {y})");
                }
            }
        }
        let whole = cloud_tile(look, 0, 2 * w, h, 1.5, 3);
        let right = cloud_tile(look, w as i64, w, h, 1.5, 3);
        for y in 0..h {
            for x in 0..w {
                assert_eq!(&whole[(y * 2 * w + w + x) * 4..][..4], &right[(y * w + x) * 4..][..4], "clouds: ({x}, {y})");
            }
        }
    }
}
