//! `platypus-worldview`: the world as a picture (DESIGN §3.4), for tuning
//! generation by looking at it (a person or the model reading the PNG).
//!
//! ```text
//! cargo run -p platypus_worldview --release -- [options]
//!   --seed N              world seed (default 1)
//!   --preset small|large  world size (default large)
//!   --scale N             world cells per pixel (default: the whole world ~2048 px wide)
//!   --region X,Y,W,H      only this rectangle (world cells; y up), rasterised
//!                         exactly as the game generates it (trees, shades)
//!   --out FILE            PNG to write (default worldview.png)
//! ```
//!
//! The overview samples one cell per pixel. A strip down the left edge and
//! faint lines mark the vertical bands; the dashed cyan line is sea level.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use platypus_sim::{CHUNK, Cell, CellPos, Chunk, ChunkPos, Kind, MaterialId, MaterialTable};
use platypus_worldgen::{Band, ChunkGenerator, Preset, TerrainGen};
use rayon::prelude::*;

struct Args {
    seed: u64,
    preset: Preset,
    scale: Option<i32>,
    region: Option<(i32, i32, i32, i32)>,
    out: PathBuf,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args { seed: 1, preset: Preset::Large, scale: None, region: None, out: PathBuf::from("worldview.png") };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--seed" => a.seed = value()?.parse().map_err(|e| format!("--seed: {e}"))?,
            "--preset" => {
                let v = value()?;
                a.preset = Preset::from_name(&v).ok_or(format!("--preset: {v}? (small, large)"))?;
            }
            "--scale" => a.scale = Some(value()?.parse().map_err(|e| format!("--scale: {e}"))?),
            "--region" => {
                let v: Vec<i32> = value()?.split(',').map(|s| s.trim().parse()).collect::<Result<_, _>>().map_err(|e| format!("--region: {e}"))?;
                let [x, y, w, h] = v[..] else { return Err("--region X,Y,W,H".into()) };
                a.region = Some((x, y, w, h));
            }
            "--out" => a.out = PathBuf::from(value()?),
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(a)
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("{e}");
            }
            eprintln!("usage: platypus-worldview [--seed N] [--preset small|large] [--scale N] [--region X,Y,W,H] [--out FILE]");
            std::process::exit(2);
        }
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/materials.ron");
    let src = std::fs::read_to_string(&root).unwrap_or_else(|e| panic!("cannot read {}: {e}", root.display()));
    let mats = Arc::new(MaterialTable::from_ron(&src).unwrap_or_else(|e| panic!("{e}")));

    let t = Instant::now();
    let generator = TerrainGen::new(args.seed, args.preset, &mats);
    let plan = generator.plan();
    println!("plan: seed {} {} {}×{} cells, sea level {}, planned in {:.2?}", args.seed, args.preset.name(), plan.width, plan.height, plan.sea_level, t.elapsed());
    for band in Band::ALL {
        let (lo, hi) = plan.band_span(band);
        println!("  {:<12} y {lo:>6} … {hi:>6}  ({:+} … {:+} from sea level)", band.name(), lo - plan.sea_level, hi - plan.sea_level);
    }

    let t = Instant::now();
    let (x0, y0, w, h) = args.region.unwrap_or((0, 0, plan.width, plan.height));
    let scale = args.scale.unwrap_or(if args.region.is_some() { 1 } else { (plan.width / 2048).max(1) }).max(1);
    let (pw, ph) = ((w / scale).max(1) as usize, (h / scale).max(1) as usize);
    let sky = |y: i32| {
        let k = ((y - plan.sea_level) as f32 / (plan.height - plan.sea_level) as f32).clamp(0.0, 1.0);
        mix([150, 195, 235], [40, 70, 130], k)
    };
    let mut rgb = vec![0u8; pw * ph * 3];
    if args.region.is_some() {
        // Exactly what the game generates: whole chunks, with shades.
        let (c0, c1) = (CellPos::new(x0, y0).chunk(), CellPos::new(x0 + w - 1, y0 + h - 1).chunk());
        let positions: Vec<ChunkPos> = (c0.y..=c1.y).flat_map(|cy| (c0.x..=c1.x).map(move |cx| ChunkPos::new(cx, cy))).filter(|&p| generator.in_bounds(p)).collect();
        let chunks: HashMap<ChunkPos, Chunk> = positions.par_iter().map(|&p| (p, generator.generate(p))).collect();
        rgb.par_chunks_mut(pw * 3).enumerate().for_each(|(row, line)| {
            let y = y0 + h - 1 - (row as i32 * scale);
            for col in 0..pw {
                let x = x0 + col as i32 * scale;
                let p = CellPos::new(x, y);
                let (lx, ly) = p.local();
                let c = chunks.get(&p.chunk()).map(|c| (c.get(lx, ly), c.background()[ly * CHUNK as usize + lx]));
                let px = match c {
                    Some((front, back)) => shade(&mats, front, back).unwrap_or_else(|| sky(y)),
                    None => [0, 0, 0],
                };
                line[col * 3..col * 3 + 3].copy_from_slice(&px);
            }
        });
    } else {
        rgb.par_chunks_mut(pw * 3).enumerate().for_each(|(row, line)| {
            let y = y0 + h - 1 - (row as i32 * scale + scale / 2);
            for col in 0..pw {
                let x = x0 + col as i32 * scale + scale / 2;
                let (front, back) = generator.sample(x, y);
                let px = shade(&mats, Cell::new(front, 136), Cell::new(back, 136)).unwrap_or_else(|| sky(y));
                line[col * 3..col * 3 + 3].copy_from_slice(&px);
            }
        });
    }

    // Bands: a strip down the left, a faint line at each floor, sea level dashed.
    for row in 0..ph {
        let y = y0 + h - 1 - row as i32 * scale;
        let c = band_color(plan.band_at(y));
        for col in 0..pw.min(6) {
            rgb[(row * pw + col) * 3..(row * pw + col) * 3 + 3].copy_from_slice(&c);
        }
    }
    for band in Band::ALL {
        let floor = plan.band_span(band).0;
        if let Some(row) = row_of(floor, y0, h, scale, ph) {
            for col in 6..pw {
                let i = (row * pw + col) * 3;
                let blended = mix([rgb[i], rgb[i + 1], rgb[i + 2]], band_color(band), 0.45);
                rgb[i..i + 3].copy_from_slice(&blended);
            }
        }
    }
    if let Some(row) = row_of(plan.sea_level, y0, h, scale, ph) {
        for col in (6..pw).filter(|c| (c / 6) % 2 == 0) {
            rgb[(row * pw + col) * 3..(row * pw + col) * 3 + 3].copy_from_slice(&[0, 230, 230]);
        }
    }

    write_png(&args.out, pw as u32, ph as u32, &rgb);
    println!("wrote {} ({pw}×{ph}, {scale} cells per pixel) in {:.2?}", args.out.display(), t.elapsed());
}

/// Colour of a cell with the background behind it (dimmed as the game
/// draws it); `None` for open sky.
fn shade(mats: &MaterialTable, front: Cell, back: Cell) -> Option<[u8; 3]> {
    if front.material != MaterialId::AIR {
        let [r, g, b, _] = mats.color(front);
        return Some([r, g, b]);
    }
    if back.material != MaterialId::AIR {
        let ph = mats.phys(back.material);
        let dim = if ph.kind == Kind::Plant || ph.flammability > 0 { 0.8 } else { 0.55 };
        let [r, g, b, _] = mats.color(back);
        return Some([(r as f32 * dim) as u8, (g as f32 * dim) as u8, (b as f32 * dim) as u8]);
    }
    None
}

fn band_color(b: Band) -> [u8; 3] {
    match b {
        Band::Sky => [135, 206, 250],
        Band::Peaks => [245, 245, 245],
        Band::Surface => [80, 200, 80],
        Band::Underground => [200, 150, 80],
        Band::Caverns => [160, 100, 210],
        Band::Deep => [70, 70, 180],
        Band::Underworld => [230, 60, 30],
    }
}

fn row_of(y: i32, y0: i32, h: i32, scale: i32, ph: usize) -> Option<usize> {
    let row = (y0 + h - 1 - y) / scale;
    (0..ph as i32).contains(&row).then_some(row as usize)
}

fn mix(a: [u8; 3], b: [u8; 3], k: f32) -> [u8; 3] {
    [0, 1, 2].map(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * k) as u8)
}

fn write_png(path: &std::path::Path, w: u32, h: u32, rgb: &[u8]) {
    let file = std::fs::File::create(path).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().and_then(|mut wr| wr.write_image_data(rgb)).unwrap_or_else(|e| panic!("png: {e}"));
}
