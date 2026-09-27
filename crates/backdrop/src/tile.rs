//! The underground's backdrop, a tile at a time, from absolute coordinates
//! (tiles side by side join up), in daylight colours (the game's lighting
//! grades them). (The surface's: `vista.rs`.)
//!
//! - Underground: rock seen far off through the cave (`cave_tile`, tiles
//!   across and down): cave shapes out of noise, stones lit from above,
//!   small stalactites and stalagmites along their edges. Its colours are
//!   daylight's: in the game only what light reaches shows.

use crate::{Rgb, Rock, bayer, fbm1, fbm2, hash, noise1, rgb, rock_shade, scale};

/// Underground layers (far, near).
pub const CAVE_LAYERS: usize = 2;

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

    /// Two tiles stacked are one column.
    #[test]
    fn tiles_join_without_a_seam() {
        let (w, h) = (96, 80);
        let tall = cave_tile(1, 0, 0, w, 2 * h, 7);
        let lower = cave_tile(1, 0, h as i64, w, h, 7);
        for y in 0..h {
            for x in 0..w {
                assert_eq!(&tall[((y + h) * w + x) * 4..][..4], &lower[(y * w + x) * 4..][..4], "cave: ({x}, {y})");
            }
        }
    }
}
