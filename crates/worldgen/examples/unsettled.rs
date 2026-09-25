//! What's generated unsettled around a spot: powder over air, liquid beside air.
use platypus_sim::{CHUNK, ChunkPos, Kind, MaterialTable};
use platypus_worldgen::{ChunkGenerator, Preset, TerrainGen};
use std::collections::HashMap;

fn main() {
    let a: Vec<i32> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    let mats = MaterialTable::from_ron(include_str!("../../../assets/data/materials.ron")).unwrap();
    let g = TerrainGen::new(1, Preset::Large, &mats);
    let (cx, cy) = (a[0] / CHUNK, a[1] / CHUNK);
    let mut counts: HashMap<String, usize> = HashMap::new();
    for y in cy - 5..=cy + 5 {
        for x in cx - 8..=cx + 8 {
            let c = g.generate(ChunkPos::new(x, y));
            for ly in 1..CHUNK as usize - 1 {
                for lx in 1..CHUNK as usize - 1 {
                    let m = c.get(lx, ly).material;
                    let k = mats.phys(m).kind;
                    let air = |dx: i32, dy: i32| c.get((lx as i32 + dx) as usize, (ly as i32 + dy) as usize).is_air();
                    let loose = match k {
                        Kind::Powder => air(0, -1),
                        Kind::Liquid => air(0, -1) || air(-1, 0) || air(1, 0),
                        _ => false,
                    };
                    if loose {
                        *counts.entry(mats.def(m).name.clone()).or_default() += 1;
                    }
                }
            }
        }
    }
    println!("{counts:?}");
}
