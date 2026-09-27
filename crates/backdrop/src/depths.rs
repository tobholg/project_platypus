//! The underground's backdrop by depth band and zone: rock far off through
//! the caves, two layers (far, nearer), in daylight colours (the game's
//! lighting shows only what light reaches), plus what glows by itself
//! (lava in cracks, mushroom caps, crystals), drawn over the dark. A band
//! is its rock (colours, how it breaks: stones, strata, columns), how open
//! it is, and its features (roots, stalactites, mushrooms, crystals,
//! icicles, glowing cracks). Tiles from absolute coordinates, across and
//! down (they join up).

use crate::{Rgb, Rock, bayer, fbm2, hash, mix, noise1, rgb, rock_shade, scale};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Feature {
    /// Roots hanging from the ceilings.
    Roots,
    /// Small stalactites and stalagmites.
    Spikes,
    /// Long thin icicles.
    Icicles,
    /// Big mushrooms standing on the floors, their caps' spots glowing.
    Mushrooms,
    /// Crystal clusters on floors and ceilings, glowing faintly.
    Crystals,
    /// Cracks in the rock glowing with lava; a red glow from below.
    Cracks,
    /// Glowworms: threads hanging from the ceilings, beaded with light.
    Glowworms,
}

#[derive(Clone, Debug)]
pub struct Band {
    pub name: &'static str,
    /// The rock, dark and light (lit from above), and how it breaks.
    pub dark: Rgb,
    pub light: Rgb,
    pub rock: Rock,
    /// The far layer's haze colour (what it fades towards).
    pub haze: Rgb,
    /// How much is open (0..1).
    pub open: f32,
    pub features: &'static [Feature],
    /// What glows (features' light).
    pub glow: Rgb,
}

/// Layer `k` (0 far, 1 nearer) of `band`, `w`×`h` from (`x0`, `y0`) (rows
/// down), as (colour RGBA, glow RGBA): rock where it's solid, clear where
/// it opens; the glow, what shines in the dark.
pub fn band_tile(band: &Band, k: usize, x0: i64, y0: i64, w: usize, h: usize, seed: u64) -> (Vec<u8>, Vec<u8>) {
    let s = seed.wrapping_add(0xde9 + k as u64 * 7919);
    let far = k == 0;
    // Far: finer, more open, hazier; nearer: bigger, darker, fewer holes.
    // Far: denser (the back of the caverns), finer, darker and hazier;
    // nearer: bigger shapes, more open (it frames the caverns).
    let (f, open) = if far { (1.0 / 60.0, band.open + 0.12) } else { (1.0 / 95.0, band.open - 0.1) };
    let rock = |x: f32, y: f32| {
        let wx = x + 36.0 * (fbm2(x * f * 0.5, y * f * 0.5, 2, s ^ 1) - 0.5);
        let wy = y + 36.0 * (fbm2(x * f * 0.5 + 9.0, y * f * 0.5, 2, s ^ 2) - 0.5);
        // (Caves wider than tall.)
        fbm2(wx * f, wy * f * 1.6, 4, s) > 1.0 - open
    };
    // The solid grid, padded (features reach across tiles).
    let pad = 28i64;
    let (cw, ch) = (w as i64 + 2 * pad, h as i64 + 2 * pad);
    let solid: Vec<bool> = (0..ch).flat_map(|j| (0..cw).map(move |i| (i, j))).map(|(i, j)| rock((x0 - pad + i) as f32, (y0 - pad + j) as f32)).collect();
    let at = |x: i64, y: i64| -> bool {
        let (i, j) = (x - (x0 - pad), y - (y0 - pad));
        if i < 0 || j < 0 || i >= cw || j >= ch { rock(x as f32, y as f32) } else { solid[(j * cw + i) as usize] }
    };
    let has = |ft: Feature| band.features.contains(&ft);
    let glow = band.glow;
    let mut out = vec![0u8; w * h * 4];
    let mut lit = vec![0u8; w * h * 4];
    for yi in 0..h {
        let ya = y0 + yi as i64;
        for xi in 0..w {
            let xa = x0 + xi as i64;
            let (xf, yf) = (xa as f32, ya as f32);
            let wall = at(xa, ya);
            // (colour, glow)
            let mut px: Option<(Rgb, Option<(Rgb, f32)>)> = None;
            if wall {
                // Stones or strata or columns, lit from above; a lighter lip
                // where the cave opens above, darker where it opens below.
                let size = if far { 5.0 } else { 8.0 };
                let shade = rock_shade(band.rock, xf, yf, size, s ^ 0x40c);
                let mut c = mix(band.dark, band.light, ((shade - 0.6) * 1.4).clamp(0.0, 1.0));
                if !at(xa, ya - 1) || !at(xa, ya - 2) {
                    c = mix(c, band.light, 0.45);
                } else if !at(xa, ya + 1) {
                    c = scale(c, 0.75);
                }
                let mut g = None;
                // Lava in the cracks: thin lines through the rock, glowing.
                if has(Feature::Cracks) {
                    let n = fbm2(xf * 0.035, yf * 0.035, 3, s ^ 0xc4);
                    if (n - 0.5).abs() < 0.012 {
                        c = rgb(255, 150, 60);
                        g = Some((glow, 1.0));
                    }
                }
                px = Some((c, g));
            } else if let Some(p) = feature(band, xa, ya, &at, s) {
                px = Some(p);
            }
            let Some((mut c, g)) = px else { continue };
            if far {
                c = scale(mix(c, band.haze, 0.45), 0.8);
            }
            write(&mut out, xi, yi, w, xa, c, 1.0);
            if let Some((gc, a)) = g {
                write(&mut lit, xi, yi, w, xa, gc, if far { a * 0.6 } else { a });
            }
        }
    }
    (out, lit)
}

/// A feature in the open at (x, y), if one reaches here: its colour, and
/// its glow (colour, strength).
#[allow(clippy::type_complexity)]
fn feature(band: &Band, x: i64, y: i64, at: &dyn Fn(i64, i64) -> bool, s: u64) -> Option<(Rgb, Option<(Rgb, f32)>)> {
    let has = |ft: Feature| band.features.contains(&ft);
    // The ceiling above (within `n`), and the floor below.
    // (Only from a column open here: a wall's side is neither.)
    let up = |sx: i64, n: i64| if at(sx, y) { None } else { (1..=n).find(|&d| at(sx, y - d)) };
    let down = |sx: i64, n: i64| if at(sx, y) { None } else { (1..=n).find(|&d| at(sx, y + d)) };
    // Hanging things: roots, spikes, icicles, from the ceiling, tapering.
    if has(Feature::Roots) {
        for sx in x - 3..=x + 3 {
            if hash(sx, 21, s) > 0.22 {
                continue;
            }
            let len = 5.0 + 22.0 * hash(sx, 22, s);
            if let Some(d) = up(sx, len as i64) {
                // A root wanders as it hangs; thinner towards its tip, and a
                // few rootlets.
                let wob = (noise1(y as f32 * 0.25 + sx as f32 * 3.1, s ^ 23) - 0.5) * 3.0 * (d as f32 / len);
                if ((x - sx) as f32 - wob).abs() < 0.6 + 0.5 * (1.0 - d as f32 / len) {
                    return Some((scale(band.dark, 0.8), None));
                }
            }
        }
    }
    let hang = |kind: i64, every: f32, lmin: f32, lmax: f32, wide: f32| {
        (-2..=2).any(|dx| {
            let sx = x + dx;
            if hash(sx, kind, s) > every {
                return false;
            }
            let len = lmin + (lmax - lmin) * hash(sx, kind + 1, s).powi(2);
            let w = 0.6 + wide * hash(sx, kind + 2, s);
            up(sx, len as i64).is_some_and(|d| (dx as f32).abs() < w * (1.0 - d as f32 / len))
        })
    };
    if has(Feature::Spikes) && (hang(31, 0.25, 3.0, 12.0, 1.4) || {
        (-2..=2).any(|dx| {
            let sx = x + dx;
            hash(sx, 35, s) < 0.2 && {
                let len = 2.0 + 6.0 * hash(sx, 36, s);
                down(sx, len as i64).is_some_and(|d| (dx as f32).abs() < 1.2 * (1.0 - d as f32 / len))
            }
        })
    }) {
        return Some((mix(band.dark, band.light, 0.55), None));
    }
    // Glowworms: fine threads down from the ceiling, a bead of light at
    // their ends and a few along them.
    if has(Feature::Glowworms) && hash(x, 71, s) < 0.18 {
        let len = 3.0 + 14.0 * hash(x, 72, s);
        if let Some(d) = up(x, len as i64) {
            let d = d as f32;
            if d >= len - 1.0 || hash(x * 31 + y, 73, s) < 0.12 {
                return Some((rgb(170, 255, 240), Some((band.glow, 1.0))));
            }
            if hash(x, 74, s) < 0.5 {
                return Some((scale(band.light, 0.6), Some((band.glow, 0.12))));
            }
        }
    }
    if has(Feature::Icicles) && hang(41, 0.35, 6.0, 26.0, 1.1) {
        return Some((mix(band.light, rgb(236, 246, 255), 0.5), Some((band.glow, 0.15))));
    }
    // Mushrooms: a stem up from the floor, a domed cap, glowing spots.
    if has(Feature::Mushrooms) {
        for sx in x - 12..=x + 12 {
            let q = sx.div_euclid(14);
            let cx = q * 14 + (hash(q, 51, s) * 10.0) as i64;
            if cx != sx || hash(q, 52, s) > 0.55 {
                continue;
            }
            let tall = 6.0 + 16.0 * hash(q, 53, s);
            let cap = 4.0 + 7.0 * hash(q, 54, s);
            let Some(d) = down(sx, (tall + cap) as i64 + 1) else { continue };
            let above = d as f32; // rows over the floor
            let dx = (x - sx) as f32;
            if above <= tall && dx.abs() < 0.8 + 0.08 * (tall - above) {
                return Some((rgb(150, 186, 170), None));
            }
            let (cy, ry) = (tall + cap * 0.3, cap * 0.7);
            let e = (dx / cap).powi(2) + ((above - cy) / ry).powi(2);
            if above >= tall && e < 1.0 {
                // Spots on the cap glow.
                let spot = hash(x.div_euclid(2) + q * 31, y.div_euclid(2), s ^ 55) < 0.18;
                return Some(if spot { (rgb(170, 255, 230), Some((band.glow, 0.9))) } else { (mix(rgb(40, 120, 120), rgb(80, 190, 170), 1.0 - (above - tall) / (cap * 1.2)), Some((band.glow, 0.12))) });
            }
        }
    }
    // Crystals: clusters of pointed prisms leaning out of floors and
    // ceilings, a lit face and a shaded one, glowing.
    if has(Feature::Crystals) {
        for sx in x - 16..=x + 16 {
            let q = sx.div_euclid(11);
            if q * 11 + (hash(q, 60, s) * 8.0) as i64 != sx || hash(q, 61, s) > 0.4 {
                continue;
            }
            for (dir, reach) in [(1.0f32, down(sx, 18)), (-1.0, up(sx, 18))] {
                let Some(d) = reach else { continue };
                // The base on the rock (x, y down), this pixel from it; only
                // on a real ledge (rock running on level a few cells either
                // side, open over it), not a jag in a wall.
                let base_y = y as f32 + dir * d as f32;
                let b = base_y as i64;
                let side = if dir > 0.0 { -1 } else { 1 };
                if ![-3i64, -2, 2, 3].iter().all(|&dx| (-2..=2).any(|dy| at(sx + dx, b + dy) && !at(sx + dx, b + dy + side))) || !(1..=10).all(|k| !at(sx, b + side * k)) {
                    continue;
                }
                let (vx, vy) = ((x - sx) as f32, y as f32 - base_y);
                // One big prism, one or two smaller leaning off it.
                for j in 0..(2 + (hash(q, 62, s) * 2.0) as i64) {
                    let r = |k: i64| hash(q * 13 + j, k, s);
                    let a = if j == 0 { (r(63) - 0.5) * 0.3 } else { (if j % 2 == 0 { 1.0 } else { -1.0 }) * (0.35 + 0.3 * r(63)) };
                    let len = if j == 0 { 10.0 + 9.0 * r(64) } else { 5.0 + 6.0 * r(64) };
                    let wide = if j == 0 { 2.6 + 1.2 * r(65) } else { 1.8 + 1.0 * r(65) };
                    // Grows away from the rock: up from a floor, down from
                    // a ceiling.
                    let (dx, dy) = (a.sin(), -dir * a.cos());
                    let along = vx * dx + vy * dy;
                    let across = vx * dy - vy * dx;
                    let half = wide * ((len - along) / (wide * 1.6)).min(1.0);
                    if along > -1.0 && along < len && across.abs() < half {
                        let c = if across * dir < 0.0 { rgb(226, 190, 255) } else { rgb(132, 84, 204) };
                        let tip = along > len - wide * 1.6;
                        return Some((if tip { rgb(246, 226, 255) } else { c }, Some((band.glow, if tip { 0.9 } else { 0.5 }))));
                    }
                }
            }
        }
    }
    None
}

fn write(out: &mut [u8], x: usize, y: usize, w: usize, xa: i64, p: Rgb, a: f32) {
    let levels = 48.0;
    let d = (bayer(xa.rem_euclid(4) as usize, y) - 0.5) * 0.8;
    let i = (y * w + x) * 4;
    for (c, v) in p.iter().enumerate() {
        out[i + c] = (((v.clamp(0.0, 1.0) * levels + d).round() / levels).clamp(0.0, 1.0) * 255.0) as u8;
    }
    out[i + 3] = (a.clamp(0.0, 1.0) * 255.0) as u8;
}

/// The bands, top down, then the zones.
pub fn bands() -> Vec<Band> {
    vec![
        // Just under the surface: earth, roots hanging.
        Band { name: "dirt", dark: rgb(62, 40, 28), light: rgb(150, 104, 66), rock: Rock::Strata, haze: rgb(40, 30, 26), open: 0.45, features: &[Feature::Roots], glow: rgb(0, 0, 0) },
        // Stone: grey-blue, stones, small spikes.
        Band { name: "stone", dark: rgb(52, 56, 66), light: rgb(140, 146, 158), rock: Rock::Cobble, haze: rgb(30, 34, 44), open: 0.48, features: &[Feature::Spikes], glow: rgb(0, 0, 0) },
        // Deep: black basalt in columns; glowworms in the dark.
        Band { name: "deep", dark: rgb(28, 30, 40), light: rgb(96, 100, 124), rock: Rock::Basalt, haze: rgb(18, 20, 30), open: 0.46, features: &[Feature::Glowworms], glow: rgb(90, 230, 220) },
        // Over the underworld: dark red rock, lava in its cracks.
        Band { name: "underworld", dark: rgb(46, 22, 20), light: rgb(140, 70, 50), rock: Rock::Cobble, haze: rgb(60, 20, 12), open: 0.5, features: &[Feature::Cracks, Feature::Spikes], glow: rgb(255, 120, 40) },
        // Zones: fungal (teal, mushrooms), crystal (violet), ice.
        Band { name: "fungal", dark: rgb(30, 50, 54), light: rgb(90, 140, 136), rock: Rock::Cobble, haze: rgb(16, 36, 40), open: 0.5, features: &[Feature::Mushrooms], glow: rgb(80, 255, 200) },
        Band { name: "crystal", dark: rgb(40, 34, 60), light: rgb(120, 106, 160), rock: Rock::Cobble, haze: rgb(26, 20, 44), open: 0.48, features: &[Feature::Crystals], glow: rgb(190, 120, 255) },
        Band { name: "ice", dark: rgb(60, 84, 110), light: rgb(170, 206, 232), rock: Rock::Ice, haze: rgb(36, 52, 76), open: 0.48, features: &[Feature::Icicles], glow: rgb(160, 220, 255) },
    ]
}

/// A band as the game might show it, `w`×`h` cells at (`x0`, `y0`): both
/// layers; if `torch`, the world's rock in front (a cavern opening in the
/// middle), everything dark but what a torch at the centre lights, and
/// what glows by itself. RGBA.
pub fn still(band: &Band, x0: i64, y0: i64, w: usize, h: usize, torch: bool, seed: u64) -> Vec<u8> {
    let (far, far_glow) = band_tile(band, 0, x0, y0, w, h, seed);
    let (near, near_glow) = band_tile(band, 1, x0 + 7000, y0 + 3000, w, h, seed);
    let mut out = vec![0u8; w * h * 4];
    let (cx, cy) = (w as f32 * 0.5, h as f32 * 0.6);
    // What glows lights around it a little: the glows at a quarter size,
    // blurred (roughly as the game's light grid would carry them).
    let (qw, qh) = (w.div_ceil(4), h.div_ceil(4));
    let mut halo = vec![[0.0f32; 3]; qw * qh];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            for gl in [&far_glow, &near_glow] {
                let a = gl[i + 3] as f32 / 255.0;
                for k in 0..3 {
                    halo[(y / 4) * qw + x / 4][k] += gl[i + k] as f32 / 255.0 * a / 16.0;
                }
            }
        }
    }
    for _ in 0..4 {
        let prev = halo.clone();
        for y in 0..qh {
            for x in 0..qw {
                let mut sum = [0.0f32; 3];
                for (dx, dy) in [(0i64, 0i64), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = ((x as i64 + dx).clamp(0, qw as i64 - 1) as usize, (y as i64 + dy).clamp(0, qh as i64 - 1) as usize);
                    for k in 0..3 {
                        sum[k] += prev[ny * qw + nx][k] / 5.0;
                    }
                }
                halo[y * qw + x] = sum;
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let (xf, yf) = (x as f32, y as f32);
            // (A cavern in the world in front: an open blob, its rock
            // near black.)
            let front = torch && {
                let n = fbm2(xf * 0.02, yf * 0.03, 3, seed ^ 0xf0);
                let e = ((xf - cx) / (w as f32 * 0.46)).powi(2) + ((yf - cy * 0.9) / (h as f32 * 0.44)).powi(2);
                e + (n - 0.5) * 0.9 > 0.8
            };
            let mut c = [0.0f32; 3];
            let mut g = [0.0f32; 3];
            for (px, gl) in [(&far, &far_glow), (&near, &near_glow)] {
                if px[i + 3] > 0 {
                    c = [px[i] as f32, px[i + 1] as f32, px[i + 2] as f32].map(|v| v / 255.0);
                }
                let a = gl[i + 3] as f32 / 255.0;
                if a > 0.0 {
                    for k in 0..3 {
                        g[k] = g[k].max(gl[i + k] as f32 / 255.0 * a);
                    }
                }
            }
            if front {
                let n = noise1(xf * 0.4 + yf * 0.7, seed ^ 3);
                c = scale(mix(band.dark, [0.0; 3], 0.55), 0.8 + 0.3 * n);
                g = [0.0; 3];
            }
            if torch {
                // A warm torch at the centre, falling off; nothing else.
                let d = ((xf - cx).powi(2) + (yf - cy).powi(2)).sqrt();
                let l = (1.0 - d / 120.0).max(0.0).powf(1.5) * 1.3;
                let warm = [1.0, 0.82, 0.6];
                // (Sampled smoothly, as the game's light grid is.)
                let (fx, fy) = ((xf / 4.0 - 0.5).max(0.0), (yf / 4.0 - 0.5).max(0.0));
                let (ix, iy) = ((fx as usize).min(qw - 1), (fy as usize).min(qh - 1));
                let (jx, jy) = ((ix + 1).min(qw - 1), (iy + 1).min(qh - 1));
                let (tx, ty) = (fx.fract(), fy.fract());
                let hl: [f32; 3] = [0, 1, 2].map(|k| {
                    let top = halo[iy * qw + ix][k] * (1.0 - tx) + halo[iy * qw + jx][k] * tx;
                    let bot = halo[jy * qw + ix][k] * (1.0 - tx) + halo[jy * qw + jx][k] * tx;
                    top * (1.0 - ty) + bot * ty
                });
                c = [0, 1, 2].map(|k| c[k] * (l * warm[k] + hl[k] * 3.0) + g[k] + hl[k] * 0.4);
            }
            for k in 0..3 {
                out[i + k] = (c[k].clamp(0.0, 1.0) * 255.0) as u8;
            }
            out[i + 3] = 255;
        }
    }
    if torch {
        // The torch itself.
        for dy in -3i64..=3 {
            for dx in -1i64..=1 {
                let (x, y) = ((cx as i64 + dx) as usize, (cy as i64 + dy) as usize);
                let i = (y * w + x) * 4;
                out[i..i + 3].copy_from_slice(&if dy < 0 { [255, 210, 120] } else { [110, 70, 40] });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_tiles_join_without_a_seam() {
        let (w, h) = (64, 48);
        for band in bands() {
            let (tall, tall_g) = band_tile(&band, 1, 0, 0, w, 2 * h, 5);
            let (lower, lower_g) = band_tile(&band, 1, 0, h as i64, w, h, 5);
            let (wide, _) = band_tile(&band, 0, 0, 0, 2 * w, h, 5);
            let (right, _) = band_tile(&band, 0, w as i64, 0, w, h, 5);
            for y in 0..h {
                for x in 0..w {
                    assert_eq!(&tall[((y + h) * w + x) * 4..][..4], &lower[(y * w + x) * 4..][..4], "{}: down ({x}, {y})", band.name);
                    assert_eq!(&tall_g[((y + h) * w + x) * 4..][..4], &lower_g[(y * w + x) * 4..][..4], "{}: glow ({x}, {y})", band.name);
                    assert_eq!(&wide[(y * 2 * w + w + x) * 4..][..4], &right[(y * w + x) * 4..][..4], "{}: across ({x}, {y})", band.name);
                }
            }
        }
    }
}
