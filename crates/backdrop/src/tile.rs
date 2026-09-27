//! The game's backdrops, a tile at a time: layer `k` of the surface as a
//! strip of art pixels from any column on (tiles side by side join up:
//! everything is drawn from absolute columns), blended between the scenes
//! (biomes) around each column, in daylight — the game's lighting grades
//! it by the hour, as it does everything else.

use crate::{Kind, Layer, Rgb, Scene, Time, Trees, add, bayer, fbm1, fbm2, hash, light, mix, mul, noise1, noise2, rgb, scale, skyline, tree_covers};

/// The width the shapes were tuned at (the concept stills'): a strip
/// column `x` looks like column `x` of a still that wide.
pub const REF: f32 = 400.0;

/// Layers every surface scene has (far to near).
pub const SURFACE_LAYERS: usize = 4;

/// What the scenes around a strip column make of layer `k` there.
struct Column {
    top: f32,
    color: Rgb,
    foot: f32,
    /// The scene that weighs most there: its trees, its snow, its sea.
    main: Layer,
}

fn column<'s>(k: usize, x: f32, h: f32, seed: u64, at: &dyn Fn(f32) -> Vec<(&'s Scene, f32)>) -> Column {
    let lt = light(Time::Day);
    let blend = at(x);
    let total: f32 = blend.iter().map(|b| b.1).sum::<f32>().max(1e-6);
    let (mut top, mut color, mut foot, mut best) = (0.0, [0.0; 3], 0.0, (0.0f32, None::<Layer>));
    for (scene, w) in &blend {
        let Some(l) = scene.layers.get(k) else { continue };
        let w = w / total;
        top += w * skyline(l, k, x, REF, h, seed);
        let haze = (1.0 - l.depth).powf(1.3) * 0.85;
        color = add(color, scale(mix(mul(l.color, lt.land), lt.fog, haze), w));
        foot += w * l.base * h;
        if w > best.0 {
            best = (w, Some(l.clone()));
        }
    }
    let main = best.1.unwrap_or(Layer { kind: Kind::Hills, depth: 0.5, base: 0.8, height: 0.1, trees: Trees::None, color: rgb(80, 90, 80) });
    Column { top, color, foot, main }
}

/// Layer `k` of the surface, strip columns `x0 .. x0 + w`, `h` rows (the
/// skyline where the stills put it), as RGBA (clear where it isn't: the sky,
/// the layers behind). `at(x)`: the scenes around strip column x, weighted.
pub fn surface_tile<'s>(k: usize, x0: i64, w: usize, h: usize, seed: u64, at: &dyn Fn(f32) -> Vec<(&'s Scene, f32)>) -> Vec<u8> {
    let lt = light(Time::Day);
    let hf = h as f32;
    // Columns, with a margin either side for trees that lean in.
    let margin = 64i64;
    let cols: Vec<Column> = ((x0 - margin)..(x0 + w as i64 + margin)).map(|x| column(k, x as f32, hf, seed, at)).collect();
    let col = |x: i64| &cols[(x - (x0 - margin)).clamp(0, cols.len() as i64 - 1) as usize];
    // Trees: slots along the strip, each a tree or a gap, the kind the
    // column's main scene grows.
    let s = seed.wrapping_add(k as u64 * 131);
    let mut roots: Vec<(f32, f32, f32, f32, Trees)> = Vec::new();
    let mut x = x0 - margin;
    while x < x0 + w as i64 + margin {
        let c = col(x);
        let l = &c.main;
        let size = hf * (0.03 + 0.09 * l.depth.powi(2)) * if l.trees == Trees::Giant { 5.0 } else { 1.0 };
        let spacing = (size * if l.trees == Trees::Giant { 1.6 } else { 0.55 }).max(2.0);
        let slot = (x as f32 / spacing).floor();
        let tx = (slot + 0.5 + (hash(slot as i64, 11, s) - 0.5) * 0.8) * spacing;
        if l.trees != Trees::None && fbm1(tx / REF * 6.0, 2, s ^ 0x7) > 0.38 && (((x0 - margin) as f32)..((x0 + w as i64 + margin) as f32)).contains(&tx) {
            let ty = col(tx as i64).top;
            roots.push((tx, ty + 1.0, size * (0.7 + 0.6 * hash(slot as i64, 12, s)), hash(slot as i64, 13, s) * 100.0, l.trees));
        }
        x = ((slot + 1.0) * spacing).ceil() as i64;
    }
    let mut out = vec![0u8; w * h * 4];
    let mut filled = vec![false; w * h];
    for xi in 0..w {
        let xa = x0 + xi as i64;
        let c = col(xa);
        let xf = xa as f32;
        let near: Vec<&(f32, f32, f32, f32, Trees)> = roots.iter().filter(|r| (r.0 - xf).abs() < r.2 * 0.6).collect();
        for y in 0..h {
            let yf = y as f32;
            let ground = yf >= c.top;
            let tree = !ground && near.iter().any(|r| tree_covers(r.4, xf, yf, (r.0, r.1), r.2, r.3));
            let mut rgba: Option<(Rgb, f32)> = None;
            if ground || tree {
                let mut p = c.color;
                if matches!(c.main.kind, Kind::Sea) && ground {
                    let kk = (yf - c.top) / (hf - c.top).max(1.0);
                    p = mix(mix(lt.horizon, lt.top, 0.5), scale(c.color, 0.6), kk.powf(0.5));
                    if noise2(xf * 0.08, yf * 0.9, s ^ 3) > 0.78 {
                        p = mix(p, lt.sun, 0.35 * (1.0 - kk));
                    }
                } else {
                    let fine = noise2(xf * 0.45, yf * 0.45, s ^ 0x7e);
                    let broad = fbm2(xf * 0.025, yf * 0.025, 3, s ^ 0x7f);
                    p = scale(p, 0.92 + 0.05 * fine + 0.14 * broad);
                    let down = ((yf - c.top) / (hf * 0.35)).clamp(0.0, 1.0);
                    p = scale(p, 1.0 - 0.3 * down * c.main.depth);
                    if let Kind::Ridge { snow } = c.main.kind
                        && ground
                        && snow < 1.0
                    {
                        let l = &c.main;
                        let reach = ((l.base * hf - c.top) / (l.height * hf).max(1.0)).clamp(0.0, 1.0);
                        let cap = (reach - snow).max(0.0) * l.height * hf * 0.7 * (0.7 + 0.6 * noise1(xf * 0.3, s ^ 11));
                        if yf - c.top < cap {
                            let haze = (1.0 - l.depth).powf(1.3) * 0.85;
                            p = mix(mul(rgb(236, 240, 250), lt.land), lt.fog, haze * 0.8);
                        }
                    }
                }
                filled[y * w + xi] = true;
                rgba = Some((p, 1.0));
            } else {
                // Mist lying at this layer's foot.
                let dy = c.foot - yf;
                let thick = hf * 0.12 * 0.55;
                if (0.0..thick).contains(&dy) {
                    let wisp = fbm2(xf * 0.015, yf * 0.08, 3, s ^ 0x3157);
                    let a = (1.0 - dy / thick).powi(2) * (0.65 + 0.35 * wisp) * 0.5 * 0.55 * (1.0 - c.main.depth * 0.6);
                    rgba = Some((lt.fog, a));
                }
            }
            if let Some((p, a)) = rgba {
                write(&mut out, xi, y, w, xa, p, a);
            }
        }
    }
    // Rims: the top edge of what's drawn, sunlit.
    for y in 1..h {
        for xi in 0..w {
            if filled[y * w + xi] && !filled[(y - 1) * w + xi] {
                let xa = x0 + xi as i64;
                let i = (y * w + xi) * 4;
                let p = [out[i] as f32 / 255.0, out[i + 1] as f32 / 255.0, out[i + 2] as f32 / 255.0];
                let depth = col(xa).main.depth;
                write(&mut out, xi, y, w, xa, mix(p, lt.sun, 0.35 * (0.4 + 0.6 * depth)), 1.0);
            }
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
    use crate::scenes;

    /// Two tiles side by side are one strip: the column at their seam
    /// matches either way it's drawn.
    #[test]
    fn tiles_join_without_a_seam() {
        let all = scenes();
        let forest = all.iter().find(|s| s.name == "forest").unwrap();
        let at = |_x: f32| vec![(forest, 1.0)];
        let (w, h) = (128, 120);
        let whole = surface_tile(1, 0, 2 * w, h, 7, &at);
        let right = surface_tile(1, w as i64, w, h, 7, &at);
        for y in 0..h {
            for x in 0..w {
                let a = &whole[(y * 2 * w + w + x) * 4..][..4];
                let b = &right[(y * w + x) * 4..][..4];
                assert_eq!(a, b, "pixel ({x}, {y}) differs across the seam");
            }
        }
    }
}
