//! Headless performance scenarios with budgets (SPEC §9).
//!
//! `cargo run -p platypus_bench --release [-- scenario-name]`
//! Prints a table and exits with status 1 if any scenario is over budget.

use std::sync::Arc;
use std::time::{Duration, Instant};

use platypus_sim::{CHUNK, CellPos, ChunkPos, MaterialTable, World, WorldEdit};
use platypus_worldgen::{ChunkGenerator, FlatGen, TerrainConfig, TerrainGen};
use rayon::prelude::*;

/// The region a 1080p screen at 3 px/cell needs (640×360 cells) plus a margin.
const VIEW_W: i32 = 12;
const VIEW_H: i32 = 8;

struct Outcome {
    name: &'static str,
    what: String,
    avg: Duration,
    worst: Duration,
    budget: Duration,
}

fn materials() -> Arc<MaterialTable> {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/data/materials.ron"))
        .expect("read assets/data/materials.ron");
    Arc::new(MaterialTable::from_ron(&src).expect("valid materials"))
}

fn load_region(world: &mut World, g: &dyn ChunkGenerator, lo: ChunkPos, w: i32, h: i32) {
    let positions: Vec<ChunkPos> = (0..h).flat_map(|y| (0..w).map(move |x| lo.offset(x, y))).collect();
    let chunks: Vec<_> = positions.par_iter().filter(|p| g.in_bounds(**p)).map(|p| g.generate(*p)).collect();
    for c in chunks {
        world.insert_chunk(c);
    }
}

fn time_ticks(world: &mut World, ticks: usize) -> (Duration, Duration, usize) {
    let (mut total, mut worst, mut active) = (Duration::ZERO, Duration::ZERO, 0);
    for _ in 0..ticks {
        let t = Instant::now();
        let s = world.step();
        let d = t.elapsed();
        total += d;
        worst = worst.max(d);
        active = active.max(s.active_chunks);
    }
    (total / ticks as u32, worst, active)
}

/// Real terrain around the surface after its initial collapse has settled.
fn settled(m: &Arc<MaterialTable>) -> Outcome {
    let g = TerrainGen::new(1234, TerrainConfig::default(), m);
    let mut w = World::new(1234, m.clone());
    w.set_climate(g.climate());
    let cx = 128;
    let cy = g.surface_at(cx * CHUNK) / CHUNK - VIEW_H / 2;
    load_region(&mut w, &g, ChunkPos::new(cx - VIEW_W / 2, cy), VIEW_W, VIEW_H);
    let mut settle_ticks = 0;
    while settle_ticks < 5_000 && w.step().active_chunks > 0 {
        settle_ticks += 1;
    }
    let (avg, worst, active) = time_ticks(&mut w, 300);
    Outcome {
        name: "settled",
        what: format!("{} chunks loaded, settled after {settle_ticks} ticks, {active} still active", w.loaded_count()),
        avg,
        worst,
        budget: Duration::from_micros(500),
    }
}

/// Deep underground: lava lakes, obsidian, gas pockets. Heat must settle too.
fn deep(m: &Arc<MaterialTable>) -> Outcome {
    let g = TerrainGen::new(1234, TerrainConfig::default(), m);
    let mut w = World::new(1234, m.clone());
    w.set_climate(g.climate());
    load_region(&mut w, &g, ChunkPos::new(122, 2), VIEW_W, VIEW_H);
    let mut settle_ticks = 0;
    while settle_ticks < 8_000 && w.step().active_chunks > 0 {
        settle_ticks += 1;
    }
    let lava = m.expect_id("lava");
    let lava_cells: usize = w.chunks().flat_map(|c| c.cells()).filter(|c| c.material == lava).count();
    let (avg, worst, active) = time_ticks(&mut w, 300);
    Outcome {
        name: "deep",
        what: format!("{lava_cells} lava cells, settled after {settle_ticks} ticks, {active} still active"),
        avg,
        worst,
        budget: Duration::from_micros(500),
    }
}

/// A screenful of sand and water collapsing at once.
fn avalanche(m: &Arc<MaterialTable>) -> Outcome {
    let g = FlatGen { width_chunks: VIEW_W, height_chunks: VIEW_H, floor: 8, stone: m.expect_id("stone") };
    let mut w = World::new(99, m.clone());
    load_region(&mut w, &g, ChunkPos::new(0, 0), VIEW_W, VIEW_H);
    let (sand, water) = (m.expect_id("sand"), m.expect_id("water"));
    let mut placed = 0;
    for i in 0..12 {
        for j in 0..3 {
            let center = CellPos::new(40 + i * 60, 260 + j * 70);
            let material = if (i + j) % 2 == 0 { sand } else { water };
            placed += w.apply_edit(&WorldEdit::Paint { center, radius: 26, material, overwrite: true }).placed;
        }
    }
    let (avg, worst, active) = time_ticks(&mut w, 240);
    Outcome {
        name: "avalanche",
        what: format!("{placed} cells falling, up to {active} active chunks"),
        avg,
        worst,
        budget: Duration::from_millis(6),
    }
}

/// Cost of bringing a new column of chunks into view (generation + insert).
fn streaming(m: &Arc<MaterialTable>) -> Outcome {
    let g = TerrainGen::new(77, TerrainConfig::default(), m);
    let mut w = World::new(77, m.clone());
    let cy = g.surface_at(64 * CHUNK) / CHUNK - VIEW_H / 2;
    let (mut total, mut worst) = (Duration::ZERO, Duration::ZERO);
    let columns = 40;
    for i in 0..columns {
        let t = Instant::now();
        load_region(&mut w, &g, ChunkPos::new(40 + i, cy), 1, VIEW_H);
        let d = t.elapsed();
        total += d;
        worst = worst.max(d);
    }
    Outcome {
        name: "streaming",
        what: format!("{VIEW_H}-chunk column generated + inserted, {columns} columns"),
        avg: total / columns as u32,
        worst,
        budget: Duration::from_millis(4),
    }
}

fn main() {
    let only = std::env::args().nth(1);
    let m = materials();
    type Scenario = fn(&Arc<MaterialTable>) -> Outcome;
    let scenarios: [(&str, Scenario); 4] =
        [("settled", settled), ("deep", deep), ("avalanche", avalanche), ("streaming", streaming)];
    println!("platypus_bench — {} worker threads\n", rayon::current_num_threads());
    println!("{:<11} {:>10} {:>10} {:>10}", "scenario", "avg", "worst", "budget");
    let mut failed = false;
    for (name, run) in scenarios {
        if only.as_deref().is_some_and(|o| o != name) {
            continue;
        }
        let o = run(&m);
        let ok = o.avg <= o.budget;
        failed |= !ok;
        println!(
            "{:<11} {:>10.3?} {:>10.3?} {:>10.3?}  {} {}",
            o.name,
            o.avg,
            o.worst,
            o.budget,
            if ok { "ok  " } else { "OVER" },
            o.what
        );
    }
    std::process::exit(failed as i32);
}
