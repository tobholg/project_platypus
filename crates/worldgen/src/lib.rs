//! Seeded world generation. A chunk is a pure function of `(seed, ChunkPos)`,
//! so any chunk can be generated at any time, on any thread, by any co-op peer,
//! and come out identical.
//!
//! Two levels (SPEC §3.5): a small global `WorldPlan` computed once (surface
//! heights, later biomes and cave paths), and per-chunk rasterisation from it.

use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use platypus_sim::rng::{Rng, hash};
use platypus_sim::{CHUNK, CHUNK_AREA, Cell, CellPos, Chunk, ChunkPos, Climate, MaterialId, MaterialTable};

pub mod flora;

use flora::{Forest, TreePart};

/// Anything that can fill a chunk. The game streams through this trait, so
/// a test world, a flat sandbox or the real generator are interchangeable.
pub trait ChunkGenerator: Send + Sync {
    /// World size in chunks; chunks outside are never generated (the edge is a wall).
    fn bounds(&self) -> (ChunkPos, ChunkPos);
    fn generate(&self, pos: ChunkPos) -> Chunk;

    /// Where players start: standing on the ground in the middle of the world.
    fn spawn_point(&self) -> CellPos;

    /// Ambient temperature by height.
    fn climate(&self) -> Climate {
        Climate::default()
    }

    fn in_bounds(&self, pos: ChunkPos) -> bool {
        let (lo, hi) = self.bounds();
        pos.x >= lo.x && pos.y >= lo.y && pos.x <= hi.x && pos.y <= hi.y
    }
}

#[derive(Clone, Debug)]
pub struct TerrainConfig {
    pub width_chunks: i32,
    pub height_chunks: i32,
    /// Average surface height as a fraction of world height.
    pub surface: f64,
    pub hills: f64,
    pub cave_threshold: f64,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        // 16384 × 4096 cells: a Terraria "medium" world at 3 px per cell.
        TerrainConfig { width_chunks: 256, height_chunks: 64, surface: 0.68, hills: 90.0, cave_threshold: 0.18 }
    }
}

struct Ids {
    air: MaterialId,
    bedrock: MaterialId,
    stone: MaterialId,
    dirt: MaterialId,
    grass: MaterialId,
    snow: MaterialId,
    sand: MaterialId,
    gravel: MaterialId,
    water: MaterialId,
    lava: MaterialId,
    oil: MaterialId,
    coal: MaterialId,
    obsidian: MaterialId,
    methane: MaterialId,
    wood: MaterialId,
    leaves: MaterialId,
    tall_grass: MaterialId,
}

/// Phase-1 terrain: hills with cliffs, dirt over stone, noise caves, and
/// pockets of sand, gravel, water, oil and deep lava to exercise the sim.
/// Ports of the legacy generators (mountains, sky islands, walker caves) move
/// into the `WorldPlan` in phase 3.
pub struct TerrainGen {
    seed: u64,
    cfg: TerrainConfig,
    ids: Ids,
    /// Surface height per world column (the plan).
    surface: Vec<i32>,
    caves: Fbm<Perlin>,
    worms: Fbm<Perlin>,
    pockets: Perlin,
    strata: Perlin,
    /// Starting heat per material id (lava is born hot).
    heat: Vec<i16>,
    forest: Forest,
    /// Ragged edges of tree crowns; tall grass height.
    leaf_edge: Perlin,
    meadow: Perlin,
}

impl TerrainGen {
    pub fn new(seed: u64, cfg: TerrainConfig, mats: &MaterialTable) -> Self {
        let s = |salt: u64| (hash(&[seed, salt]) & 0xFFFF_FFFF) as u32;
        let ids = Ids {
            air: MaterialId::AIR,
            bedrock: mats.expect_id("bedrock"),
            stone: mats.expect_id("stone"),
            dirt: mats.expect_id("dirt"),
            grass: mats.expect_id("grass"),
            snow: mats.expect_id("snow"),
            sand: mats.expect_id("sand"),
            gravel: mats.expect_id("gravel"),
            water: mats.expect_id("water"),
            lava: mats.expect_id("lava"),
            oil: mats.expect_id("oil"),
            coal: mats.expect_id("coal"),
            obsidian: mats.expect_id("obsidian"),
            methane: mats.expect_id("methane"),
            wood: mats.expect_id("wood"),
            leaves: mats.expect_id("leaves"),
            tall_grass: mats.expect_id("tall_grass"),
        };

        let width = cfg.width_chunks * CHUNK;
        let height = cfg.height_chunks * CHUNK;
        let hills = Fbm::<Perlin>::new(s(1)).set_octaves(5).set_frequency(1.0 / 900.0);
        let cliffs = Perlin::new(s(2));
        let base = height as f64 * cfg.surface;
        let surface: Vec<i32> = (0..width)
            .map(|x| {
                let xf = x as f64;
                let mut h = base + hills.get([xf, 0.0]) * cfg.hills * 2.0;
                // Occasional sharp steps (legacy "cliffs").
                let c = cliffs.get([xf / 140.0, 3.7]);
                if c.abs() > 0.55 {
                    h += c.signum() * (c.abs() - 0.55) * 120.0;
                }
                h.clamp(height as f64 * 0.3, height as f64 - 200.0) as i32
            })
            .collect();

        let snow_line = (height as f64 * cfg.surface + cfg.hills * 0.9) as i32;
        let forest = {
            let surface: &[i32] = &surface;
            let at = |x: i32| surface[x.clamp(0, width - 1) as usize];
            // Not on snow, not on a cliff edge.
            Forest::plan(seed, width, at, |x| at(x) <= snow_line && (at(x - 3) - at(x + 3)).abs() < 7)
        };
        TerrainGen {
            seed,
            ids,
            surface,
            forest,
            leaf_edge: Perlin::new(s(7)),
            meadow: Perlin::new(s(8)),
            caves: Fbm::<Perlin>::new(s(3)).set_octaves(4).set_frequency(1.0 / 160.0),
            worms: Fbm::<Perlin>::new(s(4)).set_octaves(3).set_frequency(1.0 / 260.0),
            pockets: Perlin::new(s(5)),
            strata: Perlin::new(s(6)),
            heat: mats.iter().map(|(id, _)| mats.phys(id).heat).collect(),
            cfg,
        }
    }

    /// Surface height (first air cell above ground) at a world column.
    pub fn surface_at(&self, x: i32) -> i32 {
        self.surface[x.clamp(0, self.surface.len() as i32 - 1) as usize]
    }

    pub fn size_cells(&self) -> (i32, i32) {
        (self.cfg.width_chunks * CHUNK, self.cfg.height_chunks * CHUNK)
    }

    /// The background layer: walls underground (what you see in caves), trees
    /// above ground.
    fn background_at(&self, x: i32, y: i32, trees: &[flora::Tree]) -> MaterialId {
        let i = &self.ids;
        let depth = self.surface_at(x) - y;
        for t in trees {
            match t.part_at(x, y, &self.leaf_edge) {
                Some(TreePart::Wood) => return i.wood,
                Some(TreePart::Leaves) => return i.leaves,
                None => {}
            }
        }
        if depth > 6 {
            if depth < 16 { i.dirt } else { i.stone }
        } else {
            i.air
        }
    }

    /// Tall grass growing out of grassy ground.
    fn grass_at(&self, x: i32, y: i32) -> bool {
        let surface = self.surface_at(x);
        let above = y - surface; // 0 = first cell above the grass
        if !(0..=8).contains(&above) {
            return false;
        }
        let ground = self.material_at(x, surface - 1);
        if ground != self.ids.grass {
            return false;
        }
        let m = self.meadow.get([x as f64 / 60.0, 3.3]);
        let blade = (hash(&[self.seed, 0x6A55, x as u64]) % 100) as f64 / 100.0;
        let height = ((m + 0.35) * 9.0 * (0.4 + 0.6 * blade)) as i32;
        above < height
    }

    fn material_at(&self, x: i32, y: i32) -> MaterialId {
        let i = &self.ids;
        let height = self.cfg.height_chunks * CHUNK;
        if y < 6 + (hash(&[self.seed, 77, x as u64]) % 4) as i32 {
            return i.bedrock;
        }
        let surface = self.surface_at(x);
        let depth = surface - y;
        if depth <= 0 {
            return i.air;
        }
        let (xf, yf) = (x as f64, y as f64);

        // Caves: blobby caverns plus long worm tunnels, fading in below the topsoil.
        let fade = ((depth as f64 - 12.0) / 60.0).clamp(0.0, 1.0);
        let cave = self.caves.get([xf, yf * 1.6]) * fade;
        let worm = self.worms.get([xf, yf]).abs();
        let open = cave > self.cfg.cave_threshold || (worm < 0.035 * fade && depth > 20);
        let deep = y < height / 8;
        if open {
            // Fill the bottoms of some caverns with a liquid pool.
            let pool = self.pockets.get([xf / 90.0, yf / 90.0, 1.3]);
            if pool > 0.35 && self.caves.get([xf, (yf - 10.0) * 1.6]) * fade <= self.cfg.cave_threshold {
                return if deep { i.lava } else if pool > 0.62 { i.oil } else { i.water };
            }
            return i.air;
        }

        let snowy = surface > (height as f64 * self.cfg.surface + self.cfg.hills * 0.9) as i32;
        if depth == 1 {
            return if snowy { i.snow } else { i.grass };
        }
        let topsoil = 8 + (self.strata.get([xf / 40.0, 0.5]) * 5.0) as i32;
        if depth <= topsoil {
            return if snowy && depth <= 4 { i.snow } else { i.dirt };
        }
        // Sealed gas bubbles deep in the rock: bomb or dig into one with fire nearby.
        if depth > 90 && self.pockets.get([xf / 34.0, yf / 22.0, 23.3]) > 0.66 {
            return i.methane;
        }
        // Pockets inside rock.
        let p = self.pockets.get([xf / 45.0, yf / 45.0, 7.1]);
        if p > 0.55 {
            return i.sand;
        }
        if p < -0.6 {
            return i.gravel;
        }
        let ore = self.pockets.get([xf / 18.0, yf / 18.0, 11.9]);
        if ore > 0.62 {
            return i.coal;
        }
        if deep && self.strata.get([xf / 70.0, yf / 70.0]) > 0.45 {
            return i.obsidian;
        }
        i.stone
    }
}

impl ChunkGenerator for TerrainGen {
    fn bounds(&self) -> (ChunkPos, ChunkPos) {
        (ChunkPos::new(0, 0), ChunkPos::new(self.cfg.width_chunks - 1, self.cfg.height_chunks - 1))
    }

    fn climate(&self) -> Climate {
        let height = self.cfg.height_chunks * CHUNK;
        // 15 °C at the average surface, ~-1 °C on the snowy peaks, warmer deep down.
        Climate {
            sea_level: (height as f64 * self.cfg.surface) as i32,
            surface_temp: 15,
            cells_per_degree_up: 5,
            cells_per_degree_down: 40,
        }
    }

    fn spawn_point(&self) -> CellPos {
        let x = self.cfg.width_chunks * CHUNK / 2;
        CellPos::new(x, self.surface_at(x) + 2)
    }

    fn generate(&self, pos: ChunkPos) -> Chunk {
        let origin = pos.origin();
        let mut rng = Rng::seeded(&[self.seed, 0xC4C4, pos.x as u64, pos.y as u64]);
        let trees = self.forest.near(origin.x, origin.x + CHUNK - 1);
        let mut cells = Vec::with_capacity(CHUNK_AREA);
        let mut bg = Vec::with_capacity(CHUNK_AREA);
        let make = |m: MaterialId, rng: &mut Rng| {
            if m == self.ids.air { Cell::AIR } else { Cell { heat: self.heat[m.0 as usize], ..Cell::new(m, rng.next_u8()) } }
        };
        for ly in 0..CHUNK {
            for lx in 0..CHUNK {
                let (x, y) = (origin.x + lx, origin.y + ly);
                let mut m = self.material_at(x, y);
                if m == self.ids.air && self.grass_at(x, y) {
                    m = self.ids.tall_grass;
                }
                cells.push(make(m, &mut rng));
                bg.push(make(self.background_at(x, y, trees), &mut rng));
            }
        }
        Chunk::with_background(pos, cells, bg)
    }
}

/// A flat box for tests and the sandbox: stone floor, air above.
pub struct FlatGen {
    pub width_chunks: i32,
    pub height_chunks: i32,
    pub floor: i32,
    pub stone: MaterialId,
}

impl ChunkGenerator for FlatGen {
    fn bounds(&self) -> (ChunkPos, ChunkPos) {
        (ChunkPos::new(0, 0), ChunkPos::new(self.width_chunks - 1, self.height_chunks - 1))
    }

    fn spawn_point(&self) -> CellPos {
        CellPos::new(self.width_chunks * CHUNK / 2, self.floor + 2)
    }

    fn generate(&self, pos: ChunkPos) -> Chunk {
        let o = pos.origin();
        let cells = (0..CHUNK)
            .flat_map(|ly| (0..CHUNK).map(move |lx| (lx, ly)))
            .map(|(lx, ly)| {
                if o.y + ly < self.floor { Cell::new(self.stone, ((lx * 7 + ly * 13) & 255) as u8) } else { Cell::AIR }
            })
            .collect();
        Chunk::new(pos, cells)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platypus_sim::store;

    fn mats() -> MaterialTable {
        MaterialTable::from_ron(include_str!("../../../assets/data/materials.ron")).unwrap()
    }

    #[test]
    fn forests_grow_and_trees_are_rooted() {
        let m = mats();
        let g = TerrainGen::new(3, TerrainConfig::default(), &m);
        assert!(g.forest.len() > 50, "a world has forests ({} trees)", g.forest.len());
        // A tree's trunk continues into the ground behind the surface.
        let t = &g.forest.near(8000, 12000)[0];
        let root = g.background_at(t.x, t.base - 2, g.forest.near(t.x, t.x));
        assert_eq!(root, m.expect_id("wood"));
        assert_ne!(g.material_at(t.x, t.base - 2), MaterialId::AIR, "the root is behind solid ground");
    }

    #[test]
    fn same_seed_same_chunk() {
        let m = mats();
        let cfg = TerrainConfig { width_chunks: 16, height_chunks: 16, ..Default::default() };
        let (a, b) = (TerrainGen::new(7, cfg.clone(), &m), TerrainGen::new(7, cfg.clone(), &m));
        let c = TerrainGen::new(8, cfg, &m);
        for pos in [ChunkPos::new(3, 10), ChunkPos::new(8, 5)] {
            assert_eq!(store::checksum(&a.generate(pos)), store::checksum(&b.generate(pos)));
        }
        let differs = (0..16).any(|x| store::checksum(&a.generate(ChunkPos::new(x, 10))) != store::checksum(&c.generate(ChunkPos::new(x, 10))));
        assert!(differs, "different seeds give different worlds");
    }

    #[test]
    fn has_sky_ground_and_bedrock() {
        let m = mats();
        let g = TerrainGen::new(1, TerrainConfig::default(), &m);
        let x = 5000;
        let s = g.surface_at(x);
        assert_eq!(g.material_at(x, s + 5), MaterialId::AIR);
        assert_ne!(g.material_at(x, s - 3), MaterialId::AIR);
        assert_eq!(g.material_at(x, 0), m.expect_id("bedrock"));
    }
}
