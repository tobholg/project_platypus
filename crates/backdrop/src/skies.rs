//! Options for the sky's lights (stills to choose from): the sun by day,
//! the moon and the stars by night, drawn over the surface's backdrop
//! (`peaks`) only where there's open sky, as the game draws them over its
//! lighting. Four of each: `a` as it is now, then three others.

use crate::peaks::{Look, still_masked};
use crate::{Rgb, bayer, fbm2, hash, mix, noise2, rgb};

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A canvas of 0..1 colours over the still, and its sky mask.
struct Sky {
    w: usize,
    h: usize,
    px: Vec<Rgb>,
    open: Vec<bool>,
}

impl Sky {
    /// Blend `c` over (x, y) by `a`, where the sky is open.
    fn over(&mut self, x: i64, y: i64, c: Rgb, a: f32) {
        if x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 || a <= 0.0 {
            return;
        }
        let i = y as usize * self.w + x as usize;
        if self.open[i] {
            self.px[i] = mix(self.px[i], c, a.min(1.0));
        }
    }

    /// Add `c` × `a` (light: a glow, a bloom).
    fn add(&mut self, x: i64, y: i64, c: Rgb, a: f32) {
        if x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 || a <= 0.0 {
            return;
        }
        let i = y as usize * self.w + x as usize;
        if self.open[i] {
            for (k, v) in self.px[i].iter_mut().enumerate() {
                *v = (*v + c[k] * a).min(1.0);
            }
        }
    }

    fn rgba(&self) -> Vec<u8> {
        self.px.iter().flat_map(|c| c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8).into_iter().chain([255])).collect()
    }
}

fn scene(look: &Look, w: usize, h: usize, seed: u64, night: bool) -> Sky {
    let (px, open) = still_masked(look, 1300, w, h, seed);
    let mut px: Vec<Rgb> = px.chunks(4).map(|p| [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0]).collect();
    if night {
        // As the game's night: the land graded dark and blue, the sky near
        // black, a little lighter low down.
        let g = [0.14, 0.18, 0.32];
        for (i, p) in px.iter_mut().enumerate() {
            if open[i] {
                let t = (i / w) as f32 / h as f32;
                *p = mix(rgb(6, 8, 22), rgb(16, 22, 48), smoothstep(0.2, 0.8, t));
            } else {
                *p = [p[0] * g[0], p[1] * g[1], p[2] * g[2]];
            }
        }
    }
    Sky { w, h, px, open }
}

/// The day with sun option `v` (0..4), `w`×`h`, RGBA.
pub fn day(look: &Look, v: usize, w: usize, h: usize, seed: u64) -> Vec<u8> {
    let mut s = scene(look, w, h, seed, false);
    let (cx, cy) = (w as f32 * 0.68, h as f32 * 0.16);
    let (wi, hi) = (w as i64, h as i64);
    let each = |s: &mut Sky, reach: f32, f: &dyn Fn(f32, f32, f32) -> Option<(Rgb, f32, bool)>| {
        let r = reach as i64 + 1;
        for y in (cy as i64 - r).max(0)..(cy as i64 + r).min(hi) {
            for x in (cx as i64 - r).max(0)..(cx as i64 + r).min(wi) {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                if let Some((c, a, add)) = f(dx, dy, (dx * dx + dy * dy).sqrt()) {
                    if add { s.add(x, y, c, a) } else { s.over(x, y, c, a) }
                }
            }
        }
    };
    let warm = rgb(255, 244, 214);
    match v {
        // a: now: a disc and a soft halo.
        0 => {
            each(&mut s, 40.0, &|_, _, d| (d > 7.0).then(|| (warm, (1.0 - (d - 7.0) / 33.0).max(0.0).powi(2) * 0.45, false)));
            each(&mut s, 8.0, &|_, _, d| (d <= 7.0).then_some((warm, 1.0, false)));
        }
        // b: bloom: a small white core, a warm glow, the sky brightened
        // wide around it.
        1 => {
            each(&mut s, 150.0, &|_, _, d| Some((rgb(255, 236, 200), (-(d / 70.0).powi(2)).exp() * 0.28, true)));
            each(&mut s, 30.0, &|_, _, d| Some((rgb(255, 230, 170), (-(d / 11.0).powi(2)).exp() * 0.7, true)));
            each(&mut s, 6.0, &|_, _, d| (d <= 5.0).then_some((rgb(255, 255, 245), 1.0, false)));
        }
        // c: rays: a disc with thin pixel rays, and a dithered halo in two
        // rings.
        2 => {
            each(&mut s, 34.0, &|dx, dy, d| {
                let ring = if d < 16.0 { 0.35 } else if d < 26.0 { 0.18 } else { 0.0 };
                let dither = bayer(((dx + 100.0) as usize) % 4, ((dy + 100.0) as usize) % 4) < ring * 1.6;
                (d > 7.0 && ring > 0.0 && dither).then_some((warm, 0.35, false))
            });
            each(&mut s, 24.0, &|dx, dy, d| {
                let a = dy.atan2(dx);
                let k = (a / std::f32::consts::TAU * 12.0).rem_euclid(1.0);
                let long = ((a / std::f32::consts::TAU * 12.0).floor() as i64).rem_euclid(2) == 0;
                let reach = if long { 22.0 } else { 15.0 };
                (d > 8.0 && d < reach && !(0.08..0.92).contains(&k)).then_some((warm, 0.7 * (1.0 - (d - 8.0) / (reach - 8.0)), false))
            });
            each(&mut s, 8.0, &|_, _, d| (d <= 7.0).then_some((rgb(255, 250, 225), 1.0, false)));
        }
        // d: pixel art: a chunky disc, a bright core and an orange rim, a
        // two-step glow.
        _ => {
            each(&mut s, 22.0, &|_, _, d| (d > 9.0).then(|| (rgb(255, 210, 120), if d < 13.0 { 0.35 } else if d < 18.0 { 0.16 } else { 0.0 }, false)));
            each(&mut s, 10.0, &|dx, dy, d| {
                if d > 9.0 {
                    return None;
                }
                let c = if d > 7.5 { rgb(255, 170, 60) } else if dx + dy < -3.0 && d > 4.0 { rgb(255, 255, 235) } else { rgb(255, 226, 120) };
                Some((c, 1.0, false))
            });
        }
    }
    s.rgba()
}

/// The night with moon and stars option `v` (0..4), `w`×`h`, RGBA.
pub fn night(look: &Look, v: usize, w: usize, h: usize, seed: u64) -> Vec<u8> {
    let mut s = scene(look, w, h, seed, true);
    let (wf, hf) = (w as f32, h as f32);
    let star = |k: i64, j: i64| hash(k, j, seed ^ 0x5747);
    // Stars.
    let n = match v {
        0 => 700,
        3 => 260,
        _ => 520,
    };
    // c: the milky way first, a diagonal band of faint dust with dark lanes.
    if v == 2 {
        for y in 0..h {
            for x in 0..w {
                let (fx, fy) = (x as f32 / wf, y as f32 / hf);
                let band = (-((fy - (0.75 - 0.6 * fx)) / 0.14).powi(2)).exp();
                let dust = fbm2(x as f32 * 0.02, y as f32 * 0.03, 4, seed ^ 0x3a) * band;
                let lane = fbm2(x as f32 * 0.035, y as f32 * 0.05, 3, seed ^ 0x3b);
                let a = (dust - 0.25).max(0.0) * 0.5 * smoothstep(0.35, 0.6, lane);
                s.add(x as i64, y as i64, rgb(150, 160, 210), a * (1.0 - fy * 0.6));
            }
        }
    }
    for k in 0..n {
        let (mut x, mut y) = (star(k, 1) * wf, star(k, 2).powf(1.4) * hf * 0.75);
        // (c: more of them in the band.)
        if v == 2 && star(k, 7) < 0.35 {
            let t = star(k, 8);
            x = t * wf;
            y = (0.75 - 0.6 * t + (star(k, 9) - 0.5) * 0.18) * hf;
        }
        let b = star(k, 3);
        let tint = if b > 0.93 { rgb(255, 214, 180) } else if b > 0.85 { rgb(184, 204, 255) } else { rgb(236, 240, 255) };
        let (xi, yi) = (x as i64, y as i64);
        match v {
            // a: now: single pixels, a few bright.
            0 => s.over(xi, yi, tint, 0.2 + 0.8 * b.powi(4)),
            // d: sparse and calm: mostly very dim, a few with a soft glow.
            3 => {
                s.over(xi, yi, tint, 0.12 + 0.6 * b.powi(5));
                if b > 0.96 {
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        s.add(xi + dx, yi + dy, tint, 0.18);
                    }
                }
            }
            // b, c: mostly dim pixels; the brightest few a small cross.
            _ => {
                s.over(xi, yi, tint, 0.15 + 0.7 * b.powi(5));
                if b > 0.975 {
                    for (dx, dy, a) in [(1, 0, 0.45), (-1, 0, 0.45), (0, 1, 0.45), (0, -1, 0.45), (2, 0, 0.15), (-2, 0, 0.15), (0, 2, 0.15), (0, -2, 0.15)] {
                        s.over(xi + dx, yi + dy, tint, a);
                    }
                }
            }
        }
    }
    // The moon.
    let (cx, cy) = (wf * 0.3, hf * 0.2);
    let moon = rgb(232, 236, 246);
    let each = |s: &mut Sky, reach: f32, f: &dyn Fn(f32, f32, f32) -> Option<(Rgb, f32, bool)>| {
        let r = reach as i64 + 1;
        for y in (cy as i64 - r)..(cy as i64 + r) {
            for x in (cx as i64 - r)..(cx as i64 + r) {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                if let Some((c, a, add)) = f(dx, dy, (dx * dx + dy * dy).sqrt()) {
                    if add { s.add(x, y, c, a) } else { s.over(x, y, c, a) }
                }
            }
        }
    };
    match v {
        // a: now: a disc with noise craters, a small glow.
        0 => {
            each(&mut s, 14.0, &|_, _, d| (d > 5.0).then(|| (moon, (1.0 - (d - 5.0) / 9.0).max(0.0).powi(2) * 0.18, false)));
            each(&mut s, 5.0, &|dx, dy, d| (d <= 5.0).then(|| (mix(moon, rgb(150, 156, 176), 0.22 * (1.0 - noise2(dx * 0.5 + 3.0, dy * 0.5, 77))), 1.0, false)));
        }
        // b: a crescent: lit on one side, the rest faintly there
        // (earthshine), a soft halo.
        1 => {
            each(&mut s, 30.0, &|_, _, d| Some((rgb(170, 190, 255), (-(d / 16.0).powi(2)).exp() * 0.22, true)));
            each(&mut s, 7.0, &|dx, dy, d| {
                if d > 7.0 {
                    return None;
                }
                let lit = ((dx + 3.2).powi(2) + dy * dy).sqrt() > 6.4;
                Some(if lit { (moon, 1.0, false) } else { (rgb(58, 66, 96), 0.9, false) })
            });
        }
        // c: a big moon, drawn: seas in two tones, lit from one side, a
        // wide soft halo.
        2 => {
            each(&mut s, 40.0, &|_, _, d| Some((rgb(160, 180, 240), (-(d / 22.0).powi(2)).exp() * 0.25, true)));
            each(&mut s, 10.0, &|dx, dy, d| {
                if d > 9.5 {
                    return None;
                }
                let sea = fbm2(dx * 0.22 + 4.0, dy * 0.22, 3, 91);
                let mut c = if sea > 0.58 { rgb(150, 158, 182) } else if sea > 0.5 { rgb(196, 202, 220) } else { moon };
                // (Lit from the upper left: the far limb a shade darker.)
                if dx + dy > 5.0 {
                    c = mix(c, rgb(120, 128, 156), 0.35);
                }
                Some((c, 1.0, false))
            });
        }
        // d: a full moon and a thin halo ring around it.
        _ => {
            each(&mut s, 46.0, &|_, _, d| Some((rgb(190, 205, 255), ((-((d - 38.0) / 2.2).powi(2)).exp() * 0.2 + (-(d / 10.0).powi(2)).exp() * 0.3), true)));
            each(&mut s, 6.0, &|dx, dy, d| (d <= 6.0).then(|| (mix(moon, rgb(170, 176, 196), 0.18 * (1.0 - noise2(dx * 0.6 + 7.0, dy * 0.6, 12))), 1.0, false)));
        }
    }
    s.rgba()
}
