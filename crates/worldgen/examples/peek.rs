//! The generated cells around a point, as letters (a legend follows).
use platypus_sim::{CellPos, MaterialId, MaterialTable};
use platypus_worldgen::{ChunkGenerator, Preset, TerrainGen};
use std::collections::HashMap;

fn main() {
    let a: Vec<i32> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    let (x0, y0, r) = (a[0], a[1], *a.get(2).unwrap_or(&20));
    let mats = MaterialTable::from_ron(include_str!("../../../assets/data/materials.ron")).unwrap();
    let g = TerrainGen::new(1, Preset::Large, &mats);
    let mut letters: HashMap<MaterialId, char> = HashMap::new();
    let pool: Vec<char> = "#abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    let mut chunks = HashMap::new();
    for y in (y0 - r / 2..=y0 + r / 2).rev() {
        let mut line = String::new();
        for x in x0 - r..=x0 + r {
            let p = CellPos::new(x, y);
            let c = chunks.entry(p.chunk()).or_insert_with(|| g.generate(p.chunk()));
            let (lx, ly) = p.local();
            let m = c.get(lx, ly).material;
            let ch = if m == MaterialId::AIR { '.' } else { let n = letters.len(); *letters.entry(m).or_insert(pool[n % pool.len()]) };
            line.push(if x == x0 && y == y0 { '@' } else { ch });
        }
        println!("{line}");
    }
    for (m, c) in letters {
        println!("{c} = {}", mats.def(m).name);
    }
}
