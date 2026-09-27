//! The game's backdrops, a tile at a time, from absolute coordinates
//! (tiles side by side join up), in daylight colours (the game's lighting
//! grades them).
//!
//! - The surface: distant mountains, the same everywhere: a far snowy
//!   range, and a nearer, lower one with foothills and tiny trees at its
//!   foot (`mountains_tile`, a strip).
//! - Underground: rock seen far off through the cave (`cave_tile`, tiles
//!   across and down): cave shapes out of noise, stones lit from above,
//!   small stalactites and stalagmites along their edges. Its colours are
//!   daylight's: in the game only what light reaches shows.

use crate::{Rgb, Rock, Time, Trees, bayer, fbm1, fbm2, hash, light, mix, mul, noise1, noise2, rgb, rock_shade, scale, tree_covers};

/// Surface layers (far, near).
pub const SURFACE_LAYERS: usize = 2;
/// Underground layers (far, near).
pub const CAVE_LAYERS: usize = 2;

struct Range {
    /// 0 far .. 1 near: its haze.
    depth: f32,
    /// Where its foot is (rows from the top, a share) and how tall its
    /// peaks rise (a share of the height).
    base: f32,
    height: f32,
    /// Snow on peaks reaching above this much of their height (1: none).
    snow: f32,
    color: Rgb,
    /// Foothills with tiny trees at its foot.
    foothills: bool,
    /// Horizontal scale: strip columns per unit of noise.
    scale: f32,
}

const RANGES: [Range; SURFACE_LAYERS] = [
    Range { depth: 0.0, base: 0.72, height: 0.5, snow: 0.35, color: [0.45, 0.5, 0.6], foothills: false, scale: 260.0 },
    Range { depth: 0.35, base: 0.86, height: 0.34, snow: 0.6, color: [0.32, 0.37, 0.42], foothills: true, scale: 170.0 },
];

/// Surface layer `k`, strip columns `x0 .. x0 + w`, `h` rows, RGBA (clear
/// above the skyline).
pub fn mountains_tile(k: usize, x0: i64, w: usize, h: usize, seed: u64) -> Vec<u8> {
    let r = &RANGES[k.min(SURFACE_LAYERS - 1)];
    let lt = light(Time::Day);
    let hf = h as f32;
    let s = seed.wrapping_add(k as u64 * 977);
    let haze = (1.0 - r.depth).powf(1.2) * 0.7;
    let base = mix(mul(r.color, lt.land), lt.fog, haze);
    // The skyline: ridged peaks over broad swells.
    let top = |x: f32| {
        let u = x / r.scale;
        let broad = fbm1(u * 0.35, 3, s);
        let ridged = 1.0 - (2.0 * fbm1(u * 1.3 + 3.1, 4, s ^ 9) - 1.0).abs();
        let rise = (0.45 * broad + 0.55 * ridged * (0.4 + 0.6 * broad)).powf(1.5);
        (r.base - r.height * rise) * hf
    };
    // Foothills: a low rolling line at the foot, trees on it.
    let hills = |x: f32| (r.base - 0.08 - 0.05 * fbm1(x / 90.0, 3, s ^ 0x51)) * hf;
    let margin = 16i64;
    let mut roots: Vec<(f32, f32, f32, f32, Trees)> = Vec::new();
    if r.foothills {
        // Tiny trees in clumps: a slot every 3 columns, most empty.
        for slot in (x0 - margin).div_euclid(3)..(x0 + w as i64 + margin).div_euclid(3) {
            let x = slot as f32 * 3.0 + hash(slot, 1, s) * 2.0;
            if fbm1(x / 60.0, 2, s ^ 0x7) < 0.45 || hash(slot, 2, s) < 0.35 {
                continue;
            }
            let kind = if hash(slot, 3, s) < 0.7 { Trees::Pine } else { Trees::Round };
            roots.push((x, hills(x) + 1.0, 4.0 + 5.0 * hash(slot, 4, s), hash(slot, 5, s) * 100.0, kind));
        }
    }
    let mut out = vec![0u8; w * h * 4];
    for xi in 0..w {
        let xa = x0 + xi as i64;
        let xf = xa as f32;
        let (t, ht) = (top(xf), hills(xf));
        let near: Vec<&(f32, f32, f32, f32, Trees)> = roots.iter().filter(|q| (q.0 - xf).abs() < q.2).collect();
        for y in 0..h {
            let yf = y as f32;
            let on_hill = r.foothills && yf >= ht;
            let tree = r.foothills && !on_hill && near.iter().any(|q| tree_covers(q.4, xf, yf, (q.0, q.1), q.2, q.3));
            let mountain = yf >= t;
            if !(mountain || on_hill || tree) {
                continue;
            }
            let mut p = base;
            if on_hill || tree {
                // (The foothills a little nearer: darker, greener.)
                p = mix(mul(rgb(58, 76, 62), lt.land), lt.fog, haze * 0.8);
            } else {
                // Snow on the high peaks, ragged; faces in light and shade.
                let reach = ((r.base * hf - t) / (r.height * hf)).clamp(0.0, 1.0);
                let cap = (reach - r.snow).max(0.0) * r.height * hf * 0.8 * (0.6 + 0.8 * noise1(xf * 0.2, s ^ 11));
                if yf - t < cap {
                    p = mix(mul(rgb(232, 238, 248), lt.land), lt.fog, haze * 0.75);
                }
                // Faces lit from the left: the skyline's slope over a
                // span widening with depth (so faces run down from each
                // peak and broaden), fading out towards the foot.
                let below = yf - t;
                let span = 2.0 + below * 0.8;
                let slope = ((top(xf + span) - top(xf - span)) / span).clamp(-1.0, 1.0);
                let fade = (1.0 - below / (r.height * hf * 0.9)).max(0.0);
                p = scale(p, 1.0 - 0.07 * slope * fade);
            }
            let grain = 0.95 + 0.05 * noise2(xf * 0.5, yf * 0.5, s ^ 0x7e) + 0.06 * fbm2(xf * 0.03, yf * 0.03, 2, s ^ 0x7f);
            p = scale(p, grain);
            write(&mut out, xi, y, w, xa, p, 1.0);
        }
    }
    out
}

/// The cave far off, layer `k` (0 far, 1 nearer), `w`×`h` from (`x0`,
/// `y0`) (rows down from `y0`, the tile's top row), RGBA: rock where the
/// cave's walls are, clear where it opens (on to the layer behind).
pub fn cave_tile(k: usize, x0: i64, y0: i64, w: usize, h: usize, seed: u64) -> Vec<u8> {
    let s = seed.wrapping_add(0xca7e + k as u64 * 1543);
    let far = k == 0;
    // Nearer: bigger shapes, darker; farther: finer, paler.
    let (f, open, color) = if far { (1.0 / 70.0, 0.5, rgb(112, 108, 104)) } else { (1.0 / 110.0, 0.47, rgb(86, 80, 74)) };
    let rock = |x: f32, y: f32| {
        // (Warped, so the shapes wander.)
        let wx = x + 40.0 * (fbm2(x * f * 0.5, y * f * 0.5, 2, s ^ 1) - 0.5);
        let wy = y + 40.0 * (fbm2(x * f * 0.5 + 9.0, y * f * 0.5, 2, s ^ 2) - 0.5);
        fbm2(wx * f, wy * f * 1.5, 4, s) > open
    };
    // (A little wider and taller than the tile: spikes reach across.)
    let pad = 16i64;
    let (cw, ch) = (w as i64 + 2 * pad, h as i64 + 2 * pad);
    let solid: Vec<bool> = (0..ch).flat_map(|j| (0..cw).map(move |i| (i, j))).map(|(i, j)| rock((x0 - pad + i) as f32, (y0 - pad + j) as f32)).collect();
    // (Rows go down: y + 1 is below y.)
    let at = |x: i64, y: i64| -> bool {
        let (i, j) = (x - (x0 - pad), y - (y0 - pad));
        if i < 0 || j < 0 || i >= cw || j >= ch { rock(x as f32, y as f32) } else { solid[(j * cw + i) as usize] }
    };
    // Stalactites under the rock, stalagmites on it: small, tapering,
    // wobbling, in clumps.
    let spike = |x: i64, y: i64| -> bool {
        (-2..=2).any(|dx| {
            let sx = x + dx;
            if fbm1(sx as f32 / 25.0, 2, s ^ 0x33) < 0.42 || hash(sx, 7, s) > 0.3 {
                return false;
            }
            let len = 3.0 + 12.0 * hash(sx, 8, s).powi(2);
            let wide = 0.6 + 1.6 * hash(sx, 9, s);
            let wob = (noise1(y as f32 * 0.4 + sx as f32, s ^ 4) - 0.5) * 1.2;
            let off = (dx as f32 + wob).abs();
            let hang = (1..=len as i64).find(|&d| at(sx, y - d)).map(|d| d as f32);
            let down = hang.is_some_and(|d| off < wide * (1.0 - d / len));
            let stand = (1..=(len * 0.5) as i64).find(|&d| at(sx, y + d)).map(|d| d as f32);
            let up = stand.is_some_and(|d| off < wide * 0.8 * (1.0 - d / (len * 0.5)));
            !at(sx, y) && (down || up)
        })
    };
    let mut out = vec![0u8; w * h * 4];
    for yi in 0..h {
        let ya = y0 + yi as i64;
        for xi in 0..w {
            let xa = x0 + xi as i64;
            let wall = at(xa, ya);
            if !(wall || spike(xa, ya)) {
                continue;
            }
            let shade = rock_shade(Rock::Cobble, xa as f32, ya as f32, if far { 4.0 } else { 6.0 }, s ^ 0x40c);
            let mut p = scale(color, 1.0 + (shade - 1.0) * if far { 0.35 } else { 0.6 });
            // Tops open to the cave above catch a little more light.
            if wall && !at(xa, ya - 1) {
                p = scale(p, 1.18);
            }
            write(&mut out, xi, yi, w, xa, p, 1.0);
        }
    }
    out
}

/// One pixel, quantised and dithered (by its absolute column: tiles join
/// without a seam in the pattern).
fn write(out: &mut [u8], x: usize, y: usize, w: usize, xa: i64, p: Rgb, a: f32) {
    let levels = 32.0;
    let d = bayer(xa.rem_euclid(4) as usize, y) - 0.5;
    let i = (y * w + x) * 4;
    for (c, v) in p.iter().enumerate() {
        out[i + c] = (((v.clamp(0.0, 1.0) * levels + d).round() / levels).clamp(0.0, 1.0) * 255.0) as u8;
    }
    out[i + 3] = (a.clamp(0.0, 1.0) * 255.0) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two tiles side by side are one strip; two stacked, one column.
    #[test]
    fn tiles_join_without_a_seam() {
        let (w, h) = (96, 80);
        let whole = mountains_tile(1, 0, 2 * w, h, 7);
        let right = mountains_tile(1, w as i64, w, h, 7);
        for y in 0..h {
            for x in 0..w {
                assert_eq!(&whole[(y * 2 * w + w + x) * 4..][..4], &right[(y * w + x) * 4..][..4], "mountains: ({x}, {y})");
            }
        }
        let tall = cave_tile(1, 0, 0, w, 2 * h, 7);
        let lower = cave_tile(1, 0, h as i64, w, h, 7);
        for y in 0..h {
            for x in 0..w {
                assert_eq!(&tall[((y + h) * w + x) * 4..][..4], &lower[(y * w + x) * 4..][..4], "cave: ({x}, {y})");
            }
        }
    }
}
