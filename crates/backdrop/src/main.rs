//! `platypus-backdrop <out dir> [width height]`: every scene as stills —
//! each at dawn, day, dusk and night (the underground's at one), in each
//! style — `<scene>-<time>-<style>.png`, at 1 art pixel a pixel; and the
//! game's own tiles (`tile.rs`) as they come out: `tiles-mountains.png`
//! (both ranges, over a sky), `tiles-cave.png` (both cave layers).

use platypus_backdrop::{Style, Time, render, scenes};

fn main() {
    let args: Vec<String> = std::env::args().collect();
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
}

/// The game's tiles, a stretch of each layer, far over near.
fn tiles(out: &std::path::Path) {
    use platypus_backdrop::tile::{CAVE_LAYERS, SURFACE_LAYERS, cave_tile, mountains_tile};
    let over = |under: &mut [u8], top: &[u8]| {
        for (u, t) in under.chunks_mut(4).zip(top.chunks(4)) {
            if t[3] > 0 {
                u.copy_from_slice(t);
            }
        }
    };
    let (w, h) = (1024usize, 260usize);
    let mut px: Vec<u8> = (0..w * h).flat_map(|i| { let y = (i / w) as f32 / h as f32; [(110.0 + 60.0 * y) as u8, (160.0 + 50.0 * y) as u8, 225, 255] }).collect();
    for k in 0..SURFACE_LAYERS {
        over(&mut px, &mountains_tile(k, 0, w, h, 7));
    }
    platypus_art::write_png(&platypus_art::Pixels { w: w as u32, h: h as u32, rgba: px }, &out.join("tiles-mountains.png")).expect("png");
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
