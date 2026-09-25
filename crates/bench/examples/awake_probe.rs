//! Which cells keep chunks awake? `cargo run -p platypus_bench --release --example awake_probe [surface|deep]`
use std::collections::BTreeMap;
use std::sync::Arc;

use platypus_sim::*;
use platypus_worldgen::*;

fn main() {
    let deep = std::env::args().nth(1).as_deref() == Some("deep");
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/data/materials.ron")).unwrap();
    let m = Arc::new(MaterialTable::from_ron(&src).unwrap());
    let g = TerrainGen::new(1234, Preset::Large, &m);
    let mut w = World::new(1234, m.clone());
    w.set_climate(g.climate());
    let cy = if deep { 2 } else { g.surface_at(128 * CHUNK) / CHUNK - 4 };
    for y in 0..8 {
        for x in 0..12 {
            w.insert_chunk(g.generate(ChunkPos::new(122 + x, cy + y)));
        }
    }
    for _ in 0..8000 {
        w.step();
    }
    let mut totals: BTreeMap<String, (usize, i32, i32)> = BTreeMap::new();
    for c in w.chunks() {
        let r = c.dirty_rect().clamp_to_chunk();
        if r.is_empty() {
            continue;
        }
        for y in r.min_y..=r.max_y {
            for x in r.min_x..=r.max_x {
                let cell = c.get(x as usize, y as usize);
                let e = totals.entry(m.def(cell.material).name.clone()).or_insert((0, i32::MAX, i32::MIN));
                e.0 += 1;
                e.1 = e.1.min(cell.heat as i32);
                e.2 = e.2.max(cell.heat as i32);
            }
        }
    }
    println!("awake chunks: {}", w.chunks().filter(|c| c.is_awake()).count());
    // What actually changes in one tick?
    let snap: Vec<(ChunkPos, Vec<Cell>)> = w.chunks().map(|c| (c.pos, c.cells().to_vec())).collect();
    w.step();
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut shown = 0;
    for (pos, before) in &snap {
        let after = w.chunk(*pos).unwrap().cells();
        for (i, (a, b)) in before.iter().zip(after).enumerate() {
            if a != b {
                let key = format!("{} -> {}", m.def(a.material).name, m.def(b.material).name);
                *kinds.entry(key.clone()).or_default() += 1;
                if shown < 12 {
                    shown += 1;
                    println!("  {pos:?} ({},{}) {key}: heat {}->{} flags {}->{} vy {}->{} life {}->{}", i % 64, i / 64, a.heat, b.heat, a.flags, b.flags, a.vy, b.vy, a.life, b.life);
                }
            }
        }
    }
    println!("changes per kind in one tick: {kinds:?}");
    for (name, (n, lo, hi)) in totals {
        println!("{name:>10}: {n:6} cells in dirty rects, heat {lo}..{hi}");
    }
}
