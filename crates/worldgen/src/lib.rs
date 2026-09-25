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
pub mod plan;

use std::sync::Arc;

use flora::TreePart;
pub use plan::{Band, Preset, WorldPlan};

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

    /// The band of sky clouds live in, as (bottom y, height) in cells; the
    /// weather spans the whole world width. `None`: no weather.
    fn cloud_band(&self) -> Option<(i32, i32)> {
        None
    }

    /// The ground height at x as generated (before anything was dug), for
    /// telling whether unloaded air above a column is open sky.
    fn surface_hint(&self, _x: i32) -> Option<i32> {
        None
    }

    fn in_bounds(&self, pos: ChunkPos) -> bool {
        let (lo, hi) = self.bounds();
        pos.x >= lo.x && pos.y >= lo.y && pos.x <= hi.x && pos.y <= hi.y
    }
}

/// Noise-cave threshold: higher, fewer caves.
const CAVE_THRESHOLD: f64 = 0.18;

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

/// Terrain rasterised from a `WorldPlan`: hills with cliffs, dirt over
/// stone, noise caves, pockets of sand, gravel, water, oil, and lava in the
/// underworld. The plan grows stage by stage (DESIGN §3.2).
pub struct TerrainGen {
    plan: Arc<WorldPlan>,
    ids: Ids,
    caves: Fbm<Perlin>,
    worms: Fbm<Perlin>,
    pockets: Perlin,
    strata: Perlin,
    /// Starting heat per material id (lava is born hot).
    heat: Vec<i16>,
    /// Ragged edges of tree crowns; tall grass height.
    leaf_edge: Perlin,
    meadow: Perlin,
}

impl TerrainGen {
    pub fn new(seed: u64, preset: Preset, mats: &MaterialTable) -> Self {
        Self::from_plan(Arc::new(WorldPlan::new(seed, preset)), mats)
    }

    pub fn from_plan(plan: Arc<WorldPlan>, mats: &MaterialTable) -> Self {
        let seed = plan.seed;
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

        TerrainGen {
            plan,
            ids,
            leaf_edge: Perlin::new(s(7)),
            meadow: Perlin::new(s(8)),
            caves: Fbm::<Perlin>::new(s(3)).set_octaves(4).set_frequency(1.0 / 160.0),
            worms: Fbm::<Perlin>::new(s(4)).set_octaves(3).set_frequency(1.0 / 260.0),
            pockets: Perlin::new(s(5)),
            strata: Perlin::new(s(6)),
            heat: mats.iter().map(|(id, _)| mats.phys(id).heat).collect(),
        }
    }

    pub fn plan(&self) -> &WorldPlan {
        &self.plan
    }

    /// Surface height (first air cell above ground) at a world column.
    pub fn surface_at(&self, x: i32) -> i32 {
        self.plan.surface_at(x)
    }

    pub fn size_cells(&self) -> (i32, i32) {
        (self.plan.width, self.plan.height)
    }

    /// What generation puts at a cell, front and back, without shades: for
    /// looking at the world from far away (`platypus-worldview`).
    pub fn sample(&self, x: i32, y: i32) -> (MaterialId, MaterialId) {
        let mut front = self.material_at(x, y);
        if front == self.ids.air && self.grass_at(x, y) {
            front = self.ids.tall_grass;
        }
        (front, self.background_at(x, y, &self.plan.forest.near(x, x)).0)
    }

    /// The background layer: walls underground (what you see in caves), trees
    /// above ground. Trees come with a shade (bark lit from one side, crowns
    /// lighter on top).
    fn background_at(&self, x: i32, y: i32, trees: &[&flora::Tree]) -> (MaterialId, Option<u8>) {
        let i = &self.ids;
        for t in trees {
            match t.part_at(x, y, &self.leaf_edge) {
                Some(TreePart::Wood(shade)) => return (i.wood, Some(shade)),
                Some(TreePart::Leaves(shade)) => return (i.leaves, Some(shade)),
                None => {}
            }
        }
        let depth = self.surface_at(x) - y;
        let wall = if depth > 16 { i.stone } else if depth > 6 { i.dirt } else { i.air };
        (wall, None)
    }

    /// Tall grass growing out of grassy ground.
    fn grass_at(&self, x: i32, y: i32) -> bool {
        let surface = self.surface_at(x);
        let above = y - surface; // 0 = first cell above the grass
        if !(0..=12).contains(&above) {
            return false;
        }
        let ground = self.material_at(x, surface - 1);
        if ground != self.ids.grass {
            return false;
        }
        // Meadows with bare patches between them (natural firebreaks).
        let m = self.meadow.get([x as f64 / 70.0, 3.3]);
        let blade = (hash(&[self.plan.seed, 0x6A55, x as u64]) % 100) as f64 / 100.0;
        let height = ((m + 0.2) * 13.0 * (0.45 + 0.55 * blade)) as i32;
        above < height
    }

    fn material_at(&self, x: i32, y: i32) -> MaterialId {
        let i = &self.ids;
        if y < 6 + (hash(&[self.plan.seed, 77, x as u64]) % 4) as i32 {
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
        let open = cave > CAVE_THRESHOLD || (worm < 0.035 * fade && depth > 20);
        let deep = self.plan.band_at(y) == Band::Underworld;
        if open {
            // Fill the bottoms of some caverns with a liquid pool.
            let pool = self.pockets.get([xf / 90.0, yf / 90.0, 1.3]);
            if pool > 0.35 && self.caves.get([xf, (yf - 10.0) * 1.6]) * fade <= CAVE_THRESHOLD {
                return if deep { i.lava } else if pool > 0.62 { i.oil } else { i.water };
            }
            return i.air;
        }

        let snowy = surface > self.plan.snow_line;
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
        let (w, h) = self.plan.preset.chunks();
        (ChunkPos::new(0, 0), ChunkPos::new(w - 1, h - 1))
    }

    fn climate(&self) -> Climate {
        self.plan.climate
    }

    fn surface_hint(&self, x: i32) -> Option<i32> {
        Some(self.surface_at(x))
    }

    fn cloud_band(&self) -> Option<(i32, i32)> {
        // Above the tallest trees (lightning needs room to fall), low enough
        // to be in view from the surface; the highest peaks poke into it.
        Some((self.plan.sea_level + 150, 176))
    }

    fn spawn_point(&self) -> CellPos {
        let x = self.plan.width / 2;
        CellPos::new(x, self.surface_at(x) + 2)
    }

    fn generate(&self, pos: ChunkPos) -> Chunk {
        let origin = pos.origin();
        let mut rng = Rng::seeded(&[self.plan.seed, 0xC4C4, pos.x as u64, pos.y as u64]);
        let trees = self.plan.forest.near(origin.x, origin.x + CHUNK - 1);
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
                let (b, shade) = self.background_at(x, y, &trees);
                let mut back = make(b, &mut rng);
                if let Some(shade) = shade {
                    back.shade = shade;
                }
                bg.push(back);
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
        let g = TerrainGen::new(3, Preset::Large, &m);
        assert!(g.plan.forest.len() > 50, "a world has forests ({} trees)", g.plan.forest.len());
        // A tree's trunk continues into the ground behind the surface.
        let t = g.plan.forest.near(8000, 12000)[0];
        let root = g.background_at(t.x, t.base - 2, &g.plan.forest.near(t.x, t.x)).0;
        assert_eq!(root, m.expect_id("wood"));
        assert_ne!(g.material_at(t.x, t.base - 2), MaterialId::AIR, "the root is behind solid ground");
    }

    #[test]
    fn same_seed_same_chunk() {
        let m = mats();
        let (a, b) = (TerrainGen::new(7, Preset::Small, &m), TerrainGen::new(7, Preset::Small, &m));
        let c = TerrainGen::new(8, Preset::Small, &m);
        for pos in [ChunkPos::new(3, 10), ChunkPos::new(8, 5)] {
            assert_eq!(store::checksum(&a.generate(pos)), store::checksum(&b.generate(pos)));
        }
        let differs = (0..16).any(|x| store::checksum(&a.generate(ChunkPos::new(x, 10))) != store::checksum(&c.generate(ChunkPos::new(x, 10))));
        assert!(differs, "different seeds give different worlds");
    }

    /// The same seed makes the same plan and the same chunks, in any order
    /// and on any thread: co-op peers and saved games rely on it.
    #[test]
    fn generation_is_deterministic_across_order_and_threads() {
        let m = mats();
        assert_eq!(WorldPlan::new(11, Preset::Small).checksum(), WorldPlan::new(11, Preset::Small).checksum());
        assert_ne!(WorldPlan::new(11, Preset::Small).checksum(), WorldPlan::new(12, Preset::Small).checksum());
        let positions: Vec<ChunkPos> = (0..128).step_by(9).flat_map(|x| (0..64).step_by(5).map(move |y| ChunkPos::new(x, y))).collect();
        let g = TerrainGen::new(11, Preset::Small, &m);
        let forward: Vec<_> = positions.iter().map(|&p| store::checksum(&g.generate(p))).collect();
        // Backwards, on four threads, from a second generator.
        let g2 = TerrainGen::new(11, Preset::Small, &m);
        let mut backward = vec![Default::default(); positions.len()];
        std::thread::scope(|s| {
            for (lane, out) in backward.chunks_mut(positions.len().div_ceil(4)).enumerate() {
                let (g2, positions) = (&g2, &positions);
                s.spawn(move || {
                    let base = lane * positions.len().div_ceil(4);
                    for i in (0..out.len()).rev() {
                        out[i] = store::checksum(&g2.generate(positions[base + i]));
                    }
                });
            }
        });
        assert_eq!(forward, backward);
    }

    #[test]
    fn has_sky_ground_and_bedrock() {
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let x = 5000;
        let s = g.surface_at(x);
        assert_eq!(g.material_at(x, s + 5), MaterialId::AIR);
        assert_ne!(g.material_at(x, s - 3), MaterialId::AIR);
        assert_eq!(g.material_at(x, 0), m.expect_id("bedrock"));
    }

    /// Leaves are held by wood within `LEAF_REACH` (through leaves); worldgen
    /// must not grow any further out, or they'd drop the first time anything
    /// near them is checked.
    #[test]
    fn generated_leaves_are_near_wood() {
        use std::collections::{HashMap, VecDeque};
        let m = mats();
        let g = TerrainGen::new(3, Preset::Large, &m);
        let (wood, leaves) = (m.expect_id("wood"), m.expect_id("leaves"));
        let mut worst = 0u32;
        for t in g.plan.forest.near(4000, 16000).into_iter().filter(|t| t.height > 110).take(12) {
            let (x0, y0, x1, y1) = t.bbox;
            let trees = g.plan.forest.near(x0 - 2, x1 + 2);
            let at = |x: i32, y: i32| g.background_at(x, y, &trees).0;
            let mut dist: HashMap<(i32, i32), u32> = HashMap::new();
            let mut q = VecDeque::new();
            for y in y0 - 2..=y1 + 2 {
                for x in x0 - 2..=x1 + 2 {
                    if at(x, y) == wood {
                        dist.insert((x, y), 0);
                        q.push_back((x, y));
                    }
                }
            }
            while let Some((x, y)) = q.pop_front() {
                let d = dist[&(x, y)];
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let n = (x + dx, y + dy);
                    if n.0 < x0 - 2 || n.0 > x1 + 2 || n.1 < y0 - 2 || n.1 > y1 + 2 || dist.contains_key(&n) || at(n.0, n.1) != leaves {
                        continue;
                    }
                    dist.insert(n, d + 1);
                    worst = worst.max(d + 1);
                    q.push_back(n);
                }
            }
        }
        assert!(worst > 20 && worst < platypus_sim::LEAF_REACH, "farthest leaf from wood: {worst}");
    }

    /// Felling real generated trees: each comes down as one body, and no
    /// leaf or twig is left hanging in the air afterwards.
    #[test]
    fn felled_generated_trees_leave_nothing_hanging() {
        use platypus_sim::{Kind, World, WorldEdit};
        use std::sync::Arc;
        let m = Arc::new(mats());
        let g = TerrainGen::new(3, Preset::Large, &m);
        let near = g.plan.forest.near(0, g.plan.width);
        // No wood touching another tree's wood: interlocked branches hold each
        // other up, which is right, but not what this checks.
        let wood = |t: &flora::Tree, x: i32, y: i32| matches!(t.part_at(x, y, &g.leaf_edge), Some(TreePart::Wood(_)));
        let alone = |t: &&&flora::Tree| {
            near.iter().filter(|n| n.x != t.x && n.bbox.2 >= t.bbox.0 && n.bbox.0 <= t.bbox.2).all(|n| {
                let (x0, x1) = (t.bbox.0.max(n.bbox.0) - 1, t.bbox.2.min(n.bbox.2) + 1);
                let (y0, y1) = (t.bbox.1.max(n.bbox.1) - 1, t.bbox.3.min(n.bbox.3) + 1);
                !(x0..=x1).any(|x| (y0..=y1).any(|y| wood(t, x, y) && [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| wood(n, x + dx, y + dy))))
            })
        };
        let mut picks: Vec<&&flora::Tree> = near.iter().filter(|t| t.height > 60).filter(alone).take(5).collect();
        picks.extend(near.iter().filter(|t| t.height > 130).filter(alone).take(1));
        assert_eq!(picks.len(), 6, "five trees and a giant");
        let trees: Vec<(i32, i32, f32)> = picks.iter().map(|t| (t.x, t.base, t.girth)).collect();
        for (x, base, girth) in trees {
            let mut w = World::new(3, m.clone());
            w.set_climate(g.climate());
            let (cx, cy) = (x.div_euclid(CHUNK), base.div_euclid(CHUNK));
            for dy in -2..=4 {
                for dx in -4..=4 {
                    w.insert_chunk(g.generate(ChunkPos::new(cx + dx, cy + dy)));
                }
            }
            let (x0, y0) = ((cx - 4) * CHUNK, (cy - 2) * CHUNK);
            let (x1, y1) = ((cx + 5) * CHUNK, (cy + 5) * CHUNK);
            // Background cells not connected (by edges, through background)
            // to anything resting on solid playfield.
            let hanging = |w: &World| -> Vec<CellPos> {
                let bg = |p: CellPos| w.get_bg(p).is_some_and(|b| !b.is_air());
                let anchor = |p: CellPos| w.get(p).is_some_and(|f| !f.is_air() && matches!(m.phys(f.material).kind, Kind::Static | Kind::Powder));
                let mut held = std::collections::HashSet::new();
                let mut stack: Vec<CellPos> = (x0..x1).flat_map(|x| (y0..y1).map(move |y| CellPos::new(x, y))).filter(|&p| bg(p) && (anchor(p) || p.x == x0 || p.x == x1 - 1 || p.y == y1 - 1)).collect();
                while let Some(p) = stack.pop() {
                    if !held.insert(p) {
                        continue;
                    }
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let q = CellPos::new(p.x + dx, p.y + dy);
                        if q.x >= x0 && q.x < x1 && q.y >= y0 && q.y < y1 && bg(q) && !held.contains(&q) {
                            stack.push(q);
                        }
                    }
                }
                (x0..x1).flat_map(|x| (y0..y1).map(move |y| CellPos::new(x, y))).filter(|&p| bg(p) && !held.contains(&p)).collect()
            };
            let before = hanging(&w);
            assert!(before.is_empty(), "worldgen: {} cells hanging, e.g. {:?}", before.len(), &before[..before.len().min(6)]);
            for _ in 0..2 {
                w.apply_edit(&WorldEdit::Dig { center: CellPos::new(x, base + 20), radius: girth as i32 + 4, max_hardness: 200 });
            }
            assert!(!w.bodies().is_empty(), "tree at {x} came down");
            for _ in 0..1_200 {
                w.step();
            }
            assert!(w.bodies().is_empty(), "it settled");
            let hanging = hanging(&w);
            assert!(hanging.is_empty(), "tree at {x}: {} cells hanging, e.g. {:?}", hanging.len(), &hanging[..hanging.len().min(6)]);
        }
    }



}
