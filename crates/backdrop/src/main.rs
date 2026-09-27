//! `platypus-backdrop <out dir> [width height]`: every scene as stills —
//! each at dawn, day, dusk and night (the underground's at one), in each
//! style — `<scene>-<time>-<style>.png`, at 1 art pixel a pixel.

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
}
