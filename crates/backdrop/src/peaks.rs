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
use serde::Deserialize;

/// A layer of land: terrain, a band of it seen from the front
/// (`assets/data/backdrops.ron` says what each field means).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Layer {
    pub every: f32,
    pub size: (f32, f32),
    pub slope: (f32, f32),
    pub rough: f32,
    pub base: f32,
    #[serde(deserialize_with = "rgb8")]
    pub ground: Rgb,
    #[serde(deserialize_with = "rgb8")]
    pub growth: Rgb,
    #[serde(deserialize_with = "rgb8")]
    pub snow: Rgb,
    pub snow_line: f32,
    pub water: f32,
    pub trees: f32,
    #[serde(default)]
    pub crowns: f32,
    #[serde(default)]
    pub decor: f32,
    /// How far its feet sit below where the stack puts them (cells): a
    /// low layer, showing more of what's behind.
    #[serde(default)]
    pub drop: f32,
    pub haze: f32,
    pub mist: f32,
}

/// A look's clouds: how many, how big, lit and shaded, how far above the
/// ground their bottoms sit (cells).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Clouds {
    pub many: f32,
    pub big: f32,
    #[serde(deserialize_with = "rgb8")]
    pub lit: Rgb,
    #[serde(deserialize_with = "rgb8")]
    pub shade: Rgb,
    pub foot: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Look {
    pub name: String,
    /// The sky, overhead and at the horizon (noon's).
    #[serde(deserialize_with = "rgb8_pair")]
    pub sky: (Rgb, Rgb),
    pub clouds: Clouds,
    pub layers: Vec<Layer>,
}

/// The looks, and which biome shows which (`assets/data/backdrops.ron`).
#[derive(Clone, Debug, Deserialize)]
pub struct Looks {
    pub biomes: std::collections::HashMap<String, String>,
    pub default: String,
    pub looks: Vec<Look>,
}

impl Looks {
    /// Which look a biome (the world plan's name) shows behind it.
    pub fn for_biome(&self, biome: &str) -> &str {
        self.biomes.get(biome).unwrap_or(&self.default)
    }
}

/// The looks from a backdrops file's text.
pub fn parse(text: &str) -> Result<Looks, String> {
    ron::from_str(text).map_err(|e| format!("backdrops: {e}"))
}

/// The looks as the game ships them (the file, built in).
pub fn shipped() -> Looks {
    parse(include_str!("../../../assets/data/backdrops.ron")).expect("assets/data/backdrops.ron")
}

/// The looks to choose from (as shipped).
pub fn looks() -> Vec<Look> {
    shipped().looks
}

fn rgb8<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Rgb, D::Error> {
    let (r, g, b): (u8, u8, u8) = Deserialize::deserialize(d)?;
    Ok(rgb(r, g, b))
}

fn rgb8_pair<'de, D: serde::Deserializer<'de>>(d: D) -> Result<(Rgb, Rgb), D::Error> {
    let ((r, g, b), (r2, g2, b2)): ((u8, u8, u8), (u8, u8, u8)) = Deserialize::deserialize(d)?;
    Ok((rgb(r, g, b), rgb(r2, g2, b2)))
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Layer `k`'s terrain, for a strip `hf` units tall.
pub fn shape(look: &Look, k: usize, hf: f32, seed: u64) -> Shape {
    let l = &look.layers[k];
    let (every, size, slope) = (l.every, l.size, l.slope);
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
        snow: l.snow_line,
        rough: l.rough,
        water: l.water,
        trees: l.trees,
        crowns: l.crowns,
        decor: l.decor,
        seed: seed.wrapping_add(k as u64 * 9173 + 5),
    }
}

/// What layer `k` is made of: its ground, its snow, its growth; its haze.
pub fn albedo(look: &Look, k: usize) -> Albedo {
    let l = &look.layers[k];
    Albedo { rock: l.ground, snow: l.snow, scrub: l.growth, haze: l.haze, mist: l.mist, dither: 0.25 + 0.75 * l.rough.clamp(0.0, 1.0) }
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
    let Clouds { many, big, lit, shade, .. } = look.clouds;
    let every = if many > 0.0 { 150.0 / many } else { f32::INFINITY };
    let hf = h as f32 / detail;
    let (xs, xe) = (x0 as f32 / detail, (x0 + w as i64) as f32 / detail);
    let blobs = heaps(s, every, big, hf, xs, xe);
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

/// Clouds, strip columns `x0 .. x0 + w`, `h` rows, as texels for the light
/// to colour (`cloud.wgsl`; `cloud_shade` here): the puff's face (its
/// normal's x and y; z toward the viewer follows), how deep into the heap
/// (0 its top .. 1 its flat bottom; 6 bits) and whether it's on the heap's
/// outline (a bit), how solid (alpha: the bottom thins into the air).
pub fn cloud_data(look: &Look, x0: i64, w: usize, h: usize, detail: f32, seed: u64) -> Vec<u8> {
    let s = seed ^ 0xc10d5;
    let (many, big) = (look.clouds.many, look.clouds.big);
    let every = if many > 0.0 { 150.0 / many } else { f32::INFINITY };
    let hf = h as f32 / detail;
    let (xs, xe) = (x0 as f32 / detail, (x0 + w as i64) as f32 / detail);
    let blobs = heaps(s, every, big, hf, xs, xe);
    let mut out = vec![0u8; w * h * 4];
    let inside = |b: &(f32, f32, f32, u64, f32), xf: f32, yf: f32| {
        let (dx, dy) = (xf - b.0, yf - b.1);
        if yf > b.4 {
            return false;
        }
        let a = dy.atan2(dx);
        let r = b.2 * (1.0 + 0.1 * (noise1(a * 2.5 + b.3 as f32, s ^ 4) - 0.5));
        dx * dx + dy * dy < r * r
    };
    for xi in 0..w {
        let xa = x0 + xi as i64;
        let xf = (xa as f32 + 0.5) / detail;
        let col: Vec<&(f32, f32, f32, u64, f32)> = blobs.iter().filter(|b| (xf - b.0).abs() < b.2 * 1.15 + 2.0).collect();
        if col.is_empty() {
            continue;
        }
        for y in 0..h {
            let yf = (y as f32 + 0.5) / detail;
            let Some(b) = col.iter().filter(|b| inside(b, xf, yf)).max_by(|a, b| a.1.total_cmp(&b.1)) else { continue };
            let top = col.iter().filter(|c| c.4 == b.4).map(|c| c.1 - (c.2 * c.2 - (xf - c.0).powi(2)).max(0.0).sqrt()).fold(f32::MAX, f32::min);
            let depth = smoothstep(0.0, (b.4 - top).max(8.0) * 0.9, yf - top);
            // The puff's face: a sphere's (y up), flattened a little so a
            // heap reads as one mass.
            let (dx, dy) = ((xf - b.0) / b.2, (yf - b.1) / b.2);
            // (Mostly the heap's: its puffs only round it a little.)
            let (nx, ny) = (dx * 0.45, 0.35 - dy * 0.45);
            // On the heap's outline: a step out is in no puff.
            let e = 1.6 / detail;
            let edge = [(e, 0.0), (-e, 0.0), (0.0, -e)].iter().any(|&(ox, oy)| !col.iter().any(|c| inside(c, xf + ox, yf + oy)));
            // (Its flat bottom thins into the air over a few rows: high up,
            // with no range in front of it, a hard line showed.)
            let a = smoothstep(b.4, b.4 - 24.0, yf);
            let i = (y * w + xi) * 4;
            let enc = |v: f32| ((v.clamp(-1.0, 1.0) * 0.5 + 0.5) * 255.0).round() as u8;
            out[i] = enc(nx);
            out[i + 1] = enc(ny);
            out[i + 2] = (depth.clamp(0.0, 1.0) * 63.0).round() as u8 | (edge as u8) << 6;
            out[i + 3] = (a * 255.0) as u8;
        }
    }
    out
}

/// The heaps' blobs (x, y, r, seed, the heap's bottom) for columns `xs ..
/// xe` (units): a broad bottom row, smaller ones stacked on top towards
/// the middle.
fn heaps(s: u64, every: f32, big: f32, hf: f32, xs: f32, xe: f32) -> Vec<(f32, f32, f32, u64, f32)> {
    // (A look with no clouds: `many` 0.)
    if !every.is_finite() {
        return Vec::new();
    }
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
    blobs
}

/// A cloud texel's colour in `light`, RGBA: as `cloud.wgsl` does it.
pub fn cloud_shade(px: &[u8], look: &Look, light: &Light, dither: f32) -> Option<[f32; 4]> {
    let golden = light.golden;
    if px[3] == 0 {
        return None;
    }
    let f = |v: u8| v as f32 / 255.0 * 2.0 - 1.0;
    let (nx, ny) = (f(px[0]), f(px[1]));
    let n = [nx, ny, -(1.0 - nx * nx - ny * ny).max(0.0).sqrt()];
    let depth = (px[2] & 63) as f32 / 63.0;
    let edge = px[2] & 64 != 0;
    let (lit, shade) = (look.clouds.lit, look.clouds.shade);
    let facing = (n[0] * light.to[0] + n[1] * light.to[1] + n[2] * light.to[2]).max(0.0);
    let lam = ((facing * 4.0 + 0.25 + 0.5 * dither).floor() / 4.0).min(1.0);
    // Lit puffs the sun's colour on white; the shade the sky's; deeper in
    // the heap darker, unless the sun's low: then its underside glows.
    let lit_c = [0, 1, 2].map(|k| lit[k] * (light.ambient[k] * 0.5 + light.color[k] * 0.75));
    // (A cloud's shade is bright still: light scatters through it.)
    let shade_c = [0, 1, 2].map(|k| shade[k] * (light.ambient[k] * 0.6 + 0.45));
    let mut c = mix(shade_c, lit_c, lam);
    c = c.map(|v| v * (1.0 - 0.3 * depth * (1.0 - golden)));
    c = mix(c, [1.0, 0.6, 0.45].map(|v| v * light.color[0]), golden * depth * 0.45);
    if edge && facing > 0.2 {
        c = [0, 1, 2].map(|k| c[k] * 0.5 + light.color[k] * 0.62 + 0.12);
    }
    let c = mix(c, light.haze, 0.12);
    Some([c[0], c[1], c[2], px[3] as f32 / 255.0])
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
        let foot = ground - (n - 1 - k) as f32 * 12.0 + look.layers[k].drop;
        let oy = (foot - look.layers[k].base * lh as f32) as i64;
        over(&mut out, &layer_lit(look, k, x, w, lh, 1.0, seed, light), oy, lh);
        if k == 0 {
            // The clouds, lit as the game lights them (`cloud_shade`).
            let ch = (h as f32 * 0.6) as usize;
            let data = cloud_data(look, x, w, ch, 1.0, seed);
            let mut lit = vec![0u8; data.len()];
            for (i, p) in data.chunks(4).enumerate() {
                if let Some(c) = cloud_shade(p, look, light, bayer(i % w, i / w)) {
                    let q = i * 4;
                    let (bg, a) = (look.sky.1, c[3]);
                    for k in 0..3 {
                        lit[q + k] = ((c[k] * a + bg[k] * (1.0 - a)).clamp(0.0, 1.0) * 255.0) as u8;
                    }
                    lit[q + 3] = if a > 0.05 { 255 } else { 0 };
                }
            }
            over(&mut out, &lit, (ground - h as f32 * 0.12) as i64 - ch as i64, ch);
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
    fn the_shipped_backdrops_name_looks_that_are_there() {
        let all = shipped();
        let has = |n: &str| all.looks.iter().any(|l| l.name == n);
        assert!(has(&all.default), "the default, {}", all.default);
        for (biome, look) in &all.biomes {
            assert!(has(look), "{biome}'s look {look}");
        }
    }

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
        for (name, f) in [("clouds", cloud_tile as fn(&Look, i64, usize, usize, f32, u64) -> Vec<u8>), ("cloud data", cloud_data)] {
            let whole = f(look, 0, 2 * w, h, 1.5, 3);
            let right = f(look, w as i64, w, h, 1.5, 3);
            for y in 0..h {
                for x in 0..w {
                    assert_eq!(&whole[(y * 2 * w + w + x) * 4..][..4], &right[(y * w + x) * 4..][..4], "{name}: ({x}, {y})");
                }
            }
        }
    }
}
