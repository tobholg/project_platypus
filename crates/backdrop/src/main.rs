//! `platypus-backdrop <out dir> [width height]`: every scene as stills —
//! each at dawn, day, dusk and night (the underground's at one), in each
//! style — `<scene>-<time>-<style>.png`, at 1 art pixel a pixel; and the
//! game's own tiles as they come out: `tiles-cave.png` (both cave
//! layers), `vistas.png` and `vista-*.png` (the surface's).

use platypus_backdrop::{Style, Time, render, scenes};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // `platypus-backdrop lit <out dir>`: the surface's looks lit at dawn,
    // noon, dusk and night (`lit-<look>.png`, four up), and dusk alone, 3x.
    // `platypus-backdrop bench`: how long a range's tile takes, each layer
    // of the first look, at the game's size.
    if args.get(1).map(String::as_str) == Some("bench") {
        let look = &platypus_backdrop::peaks::looks()[0];
        for k in 0..look.layers.len() {
            let t = std::time::Instant::now();
            let px = platypus_backdrop::peaks::layer_data(look, k, 0, 384, 570, 1.5, 11);
            println!("layer {k}: {:.0} ms ({} texels)", t.elapsed().as_secs_f64() * 1000.0, px.len() / 4);
        }
        return;
    }
    // `platypus-backdrop layer <look> <k> <out.png> [x0]`: one layer of a
    // look at noon as the game draws it (its detail, its strip), 3×, on
    // its sky, cropped to what shows: to look at one closely.
    if args.get(1).map(String::as_str) == Some("layer") {
        let (name, k, out) = (&args[2], args[3].parse().expect("layer"), std::path::Path::new(&args[4]));
        layer(name, k, out, args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0));
        return;
    }
    if args.get(1).map(String::as_str) == Some("lit") {
        lit(std::path::Path::new(args.get(2).map_or("backdrops", |s| s.as_str())));
        return;
    }
    let out = std::path::PathBuf::from(args.get(1).map_or("backdrops", |s| s.as_str()));
    let w: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(400);
    let h: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(225);
    std::fs::create_dir_all(&out).expect("out dir");
    for (k, scene) in scenes().iter().enumerate() {
        let times: &[Time] = if scene.underground.is_some() { &[Time::Night] } else { &[Time::Dawn, Time::Day, Time::Dusk, Time::Night] };
        for &time in times {
            for style in [Style::Flat, Style::Noita, Style::Moody] {
                let c = render(scene, time, style, 1000 + k as u64, w, h);
                let px = platypus_art::Pixels { w: w as u32, h: h as u32, rgba: c.to_rgba(if style == Style::Moody { 20.0 } else { 32.0 }) };
                let name = format!("{}-{:?}-{:?}.png", scene.name.replace(' ', "_"), time, style).to_lowercase();
                platypus_art::write_png(&px, &out.join(&name)).expect("png");
            }
        }
        println!("{}", scene.name);
    }
    tiles(&out);
    vistas(&out);
    peaks(&out);
    depths(&out);
    underdarks(&out);
    skies(&out);
}

/// The sky's lights to choose from (`skies.rs`): `skies-day.png` (four suns,
/// two across) and `skies-night.png` (four moons and starfields), and each
/// at the game's scale (3x): `sky-day-<a..d>.png`, `sky-night-<a..d>.png`.
fn skies(out: &std::path::Path) {
    use platypus_backdrop::skies::{day, night};
    let look = platypus_backdrop::peaks::looks().into_iter().find(|l| l.name == "noita").expect("noita");
    let (w, h) = (504usize, 284usize);
    let up = |px: &[u8], k: usize| -> Vec<u8> {
        let mut o = Vec::with_capacity(w * h * k * k * 4);
        for y in 0..h * k {
            for x in 0..w * k {
                o.extend_from_slice(&px[((y / k) * w + x / k) * 4..][..4]);
            }
        }
        o
    };
    for (name, f) in [("day", &day as &dyn Fn(&platypus_backdrop::peaks::Look, usize, usize, usize, u64) -> Vec<u8>), ("night", &night)] {
        let (sw, sh) = (w * 2 * 2 + 8, h * 2 * 2 + 8);
        let mut sheet = [20u8, 20, 24, 255].repeat(sw * sh);
        for v in 0..4 {
            let px = f(&look, v, w, h, 11);
            let big = up(&px, 2);
            let (ox, oy) = ((v % 2) * (w * 2 + 8), (v / 2) * (h * 2 + 8));
            for y in 0..h * 2 {
                sheet[((oy + y) * sw + ox) * 4..][..w * 2 * 4].copy_from_slice(&big[y * w * 2 * 4..][..w * 2 * 4]);
            }
            let letter = (b'a' + v as u8) as char;
            platypus_art::write_png(&platypus_art::Pixels { w: 3 * w as u32, h: 3 * h as u32, rgba: up(&px, 3) }, &out.join(format!("sky-{name}-{letter}.png"))).expect("png");
        }
        platypus_art::write_png(&platypus_art::Pixels { w: sw as u32, h: sh as u32, rgba: sheet }, &out.join(format!("skies-{name}.png"))).expect("png");
    }
}

/// The underground's looks (`underdark.rs`), 3× (a cell 3 pixels): each
/// underdark theme (`underdark-<theme>.png`), the ordinary caves' dark by
/// band (`cave-dark-<k>.png`), the underworld (`underworld.png`).
fn underdarks(out: &std::path::Path) {
    use platypus_backdrop::underdark::{THEMES, cave, underworld, vista};
    let (w, h) = (504usize, 284usize);
    let png = |px: Vec<u8>, name: String| {
        let mut big = Vec::with_capacity(w * h * 36);
        for y in 0..h * 3 {
            for x in 0..w * 3 {
                big.extend_from_slice(&px[((y / 3) * w + x / 3) * 4..][..4]);
            }
        }
        platypus_art::write_png(&platypus_art::Pixels { w: 3 * w as u32, h: 3 * h as u32, rgba: big }, &out.join(name)).expect("png");
    };
    for t in THEMES {
        png(vista(t, w, h, 31), format!("underdark-{t:?}.png").to_lowercase());
        println!("underdark {t:?}");
    }
    for k in 0..3 {
        png(cave(k, w, h, 41 + k as u64), format!("cave-dark-{k}.png"));
    }
    png(underworld(w, h, 51), "underworld.png".into());
}

/// The underground's bands (`depths.rs`): `depths.png`, each band a row,
/// fully lit on the left and as the game would show it on the right (the
/// world's rock in front, a torch, what glows by itself); and each as the
/// game would, 3×: `depth-<band>.png`.
fn depths(out: &std::path::Path) {
    use platypus_backdrop::depths::{bands, still};
    let (w, h) = (504usize, 284usize);
    let all = bands();
    let (sw, sh) = (w * 2 * 2 + 8, (h * 2 + 8) * all.len() - 8);
    let mut sheet = [20u8, 20, 24, 255].repeat(sw * sh);
    for (n, band) in all.iter().enumerate() {
        for (col, torch) in [(0usize, false), (1, true)] {
            let px = still(band, 0, 4000, w, h, torch, 21);
            let (ox, oy) = (col * (w * 2 + 8), n * (h * 2 + 8));
            for y in 0..h * 2 {
                for x in 0..w * 2 {
                    let i = ((oy + y) * sw + ox + x) * 4;
                    sheet[i..i + 4].copy_from_slice(&px[((y / 2) * w + x / 2) * 4..][..4]);
                }
            }
            if torch {
                let mut big = Vec::with_capacity(w * h * 36);
                for y in 0..h * 3 {
                    for x in 0..w * 3 {
                        big.extend_from_slice(&px[((y / 3) * w + x / 3) * 4..][..4]);
                    }
                }
                platypus_art::write_png(&platypus_art::Pixels { w: 3 * w as u32, h: 3 * h as u32, rgba: big }, &out.join(format!("depth-{}.png", band.name))).expect("png");
            }
        }
        println!("depth {}", band.name);
    }
    platypus_art::write_png(&platypus_art::Pixels { w: sw as u32, h: sh as u32, rgba: sheet }, &out.join("depths.png")).expect("png");
}

/// The Noita-like looks (`peaks.rs`): `peaks.png` (all, by day, two
/// across) and each at game scale (3×) by day, elsewhere along, at dusk
/// and at night (graded roughly as the game's lighting would).
fn peaks(out: &std::path::Path) {
    use platypus_backdrop::peaks::{grade, looks, still};
    let (w, h) = (504usize, 284usize);
    let up = |px: &[u8], k: usize| -> Vec<u8> {
        let mut o = Vec::with_capacity(w * h * k * k * 4);
        for y in 0..h * k {
            for x in 0..w * k {
                o.extend_from_slice(&px[((y / k) * w + x / k) * 4..][..4]);
            }
        }
        o
    };
    let all = looks();
    let (cols, rows) = (2usize, all.len().div_ceil(2));
    let (sw, sh) = (w * 2 * cols + 8 * (cols - 1), h * 2 * rows + 8 * (rows - 1));
    let mut sheet = [20u8, 20, 24, 255].repeat(sw * sh);
    for (n, look) in all.iter().enumerate() {
        let px = up(&still(look, 0, w, h, 11), 2);
        let (ox, oy) = ((n % cols) * (w * 2 + 8), (n / cols) * (h * 2 + 8));
        for y in 0..h * 2 {
            sheet[((oy + y) * sw + ox) * 4..][..w * 2 * 4].copy_from_slice(&px[y * w * 2 * 4..][..w * 2 * 4]);
        }
        let png = |px: Vec<u8>, name: String| platypus_art::write_png(&platypus_art::Pixels { w: 3 * w as u32, h: 3 * h as u32, rgba: px }, &out.join(name)).expect("png");
        let day = still(look, 0, w, h, 11);
        png(up(&day, 3), format!("peaks-{}.png", look.name));
        png(up(&still(look, 2300, w, h, 11), 3), format!("peaks-{}-b.png", look.name));
        let mut dusk = day.clone();
        grade(&mut dusk, [1.0, 0.62, 0.42]);
        png(up(&dusk, 3), format!("peaks-{}-dusk.png", look.name));
        let mut night = day;
        grade(&mut night, [0.14, 0.18, 0.32]);
        png(up(&night, 3), format!("peaks-{}-night.png", look.name));
        println!("peaks {}", look.name);
    }
    platypus_art::write_png(&platypus_art::Pixels { w: sw as u32, h: sh as u32, rgba: sheet }, &out.join("peaks.png")).expect("png");
}

/// The surface's vistas (`vista.rs`) as the game would show them, 3× (a
/// cell is 3 screen pixels): `vista-<name>.png` by day at two places,
/// `vista-<name>-night.png`.
fn vistas(out: &std::path::Path) {
    let (w, h) = (504usize, 284usize);
    let up = |px: &[u8]| -> Vec<u8> {
        let mut o = Vec::with_capacity(w * h * 36);
        for y in 0..h * 3 {
            for x in 0..w * 3 {
                o.extend_from_slice(&px[((y / 3) * w + x / 3) * 4..][..4]);
            }
        }
        o
    };
    // All of them on one sheet (2×, two across), day.
    let all = platypus_backdrop::vista::vistas();
    let (cols, rows) = (2usize, all.len().div_ceil(2));
    let (sw, sh) = (w * 2 * cols + 8 * (cols - 1), h * 2 * rows + 8 * (rows - 1));
    let mut sheet = [20u8, 20, 24, 255].repeat(sw * sh);
    for (n, v) in all.iter().enumerate() {
        let px = platypus_backdrop::vista::still(v, 0, w, h, 11);
        let (ox, oy) = ((n % cols) * (w * 2 + 8), (n / cols) * (h * 2 + 8));
        for y in 0..h * 2 {
            for x in 0..w * 2 {
                let i = ((oy + y) * sw + ox + x) * 4;
                sheet[i..i + 4].copy_from_slice(&px[((y / 2) * w + x / 2) * 4..][..4]);
            }
        }
    }
    platypus_art::write_png(&platypus_art::Pixels { w: sw as u32, h: sh as u32, rgba: sheet }, &out.join("vistas.png")).expect("png");
    for v in all {
        for (tag, x) in [("", 0i64), ("-b", 1700)] {
            let px = platypus_backdrop::vista::still(&v, x, w, h, 11);
            platypus_art::write_png(&platypus_art::Pixels { w: 3 * w as u32, h: 3 * h as u32, rgba: up(&px) }, &out.join(format!("vista-{}{tag}.png", v.name))).expect("png");
            if tag.is_empty() {
                let mut dark = px.clone();
                platypus_backdrop::vista::night(&mut dark);
                platypus_art::write_png(&platypus_art::Pixels { w: 3 * w as u32, h: 3 * h as u32, rgba: up(&dark) }, &out.join(format!("vista-{}-night.png", v.name))).expect("png");
            }
        }
        println!("vista {}", v.name);
    }
}

/// The cave's tiles, a stretch of each layer, far over near.
fn tiles(out: &std::path::Path) {
    use platypus_backdrop::tile::{CAVE_LAYERS, cave_tile};
    let over = |under: &mut [u8], top: &[u8]| {
        for (u, t) in under.chunks_mut(4).zip(top.chunks(4)) {
            if t[3] > 0 {
                u.copy_from_slice(t);
            }
        }
    };
    let (w, h) = (512usize, 384usize);
    let mut px: Vec<u8> = [12u8, 12, 16, 255].repeat(w * h);
    for k in 0..CAVE_LAYERS {
        let mut layer = cave_tile(k, 0, 5000, w, h, 7);
        // (The far one dimmer, as the lighting would leave it.)
        if k == 0 {
            layer.chunks_mut(4).for_each(|p| p[..3].iter_mut().for_each(|v| *v = (*v as f32 * 0.6) as u8));
        }
        over(&mut px, &layer);
    }
    platypus_art::write_png(&platypus_art::Pixels { w: w as u32, h: h as u32, rgba: px }, &out.join("tiles-cave.png")).expect("png");
}

/// One layer of a look at noon, as the game draws it (`layer` above).
fn layer(name: &str, k: usize, out: &std::path::Path, x0: i64) {
    use platypus_backdrop::peaks::{layer_tile, looks};
    let look = looks().into_iter().find(|l| l.name == name).expect("no such look");
    let (w, h) = (640usize, 570usize);
    let px = layer_tile(&look, k, x0, w, h, 1.5, 11);
    let rows: Vec<usize> = (0..h).filter(|&y| (0..w).any(|x| px[(y * w + x) * 4 + 3] > 0)).collect();
    let (y0, y1) = (rows.first().copied().unwrap_or(0), rows.last().map_or(0, |y| y + 1));
    let s = 3;
    let mut big = Vec::with_capacity(w * (y1 - y0) * s * s * 4);
    for y in y0 * s..y1 * s {
        for x in 0..w * s {
            let p = &px[((y / s) * w + x / s) * 4..][..4];
            big.extend_from_slice(&if p[3] > 0 { [p[0], p[1], p[2], 255] } else { look.sky.1.map(|c| (c * 255.0) as u8).into_iter().chain([255]).collect::<Vec<_>>().try_into().unwrap() });
        }
    }
    platypus_art::write_png(&platypus_art::Pixels { w: (w * s) as u32, h: ((y1 - y0) * s) as u32, rgba: big }, out).expect("png");
}

/// The looks lit through the day: dawn, noon, dusk, night (each graded as
/// the game's overlay roughly would: by the sky's light of the hour; the
/// haze the sky's colour of the hour).
fn lit(out: &std::path::Path) {
    use platypus_backdrop::peaks::{grade, looks, still_lit};
    use platypus_backdrop::terrain::hour;
    std::fs::create_dir_all(out).expect("out dir");
    let (w, h) = (504usize, 284usize);
    // (time, the sky's light, the sky's colour)
    let hours: [(f32, [f32; 3], [f32; 3]); 4] = [
        (0.27, [1.0, 0.7, 0.5], [0.93, 0.66, 0.6]),
        (0.5, [1.0, 0.97, 0.92], [0.6, 0.76, 0.93]),
        (0.73, [1.0, 0.62, 0.45], [0.95, 0.6, 0.55]),
        (0.0, [0.16, 0.2, 0.34], [0.07, 0.09, 0.2]),
    ];
    for look in looks() {
        let mut sheet = [20u8, 20, 24, 255].repeat((w * 2 + 8) * (h * 2 + 8));
        let sw = w * 2 + 8;
        for (n, &(time, sky, color)) in hours.iter().enumerate() {
            let light = hour(time, color);
            let (mut px, open) = still_lit(&look, 0, w, h, 11, Some(&light));
            // The sky behind: its colour of the hour.
            for (p, o) in px.chunks_mut(4).zip(&open) {
                if *o {
                    let t = 0.6;
                    for c in 0..3 {
                        p[c] = ((p[c] as f32 / 255.0 * (1.0 - t) + color[c] * t) * 255.0) as u8;
                    }
                }
            }
            grade(&mut px, sky);
            if n == 2 {
                let mut big = Vec::with_capacity(w * h * 9 * 4);
                for y in 0..h * 3 {
                    for x in 0..w * 3 {
                        big.extend_from_slice(&px[((y / 3) * w + x / 3) * 4..][..4]);
                    }
                }
                platypus_art::write_png(&platypus_art::Pixels { w: 3 * w as u32, h: 3 * h as u32, rgba: big }, &out.join(format!("lit-{}-dusk.png", look.name))).expect("png");
            }
            let (ox, oy) = ((n % 2) * (w + 8), (n / 2) * (h + 8));
            for y in 0..h {
                sheet[((oy + y) * sw + ox) * 4..][..w * 4].copy_from_slice(&px[y * w * 4..][..w * 4]);
            }
        }
        platypus_art::write_png(&platypus_art::Pixels { w: sw as u32, h: (h * 2 + 8) as u32, rgba: sheet }, &out.join(format!("lit-{}.png", look.name))).expect("png");
        println!("{}", look.name);
    }
}
