//! Seeded world generation. A chunk is a pure function of `(seed, ChunkPos)`,
//! so any chunk can be generated at any time, on any thread, by any co-op peer,
//! and come out identical.
//!
//! Two levels (SPEC §3.5): a small global `WorldPlan` computed once (surface
//! heights, later biomes and cave paths), and per-chunk rasterisation from it.

use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use platypus_sim::rng::{Rng, hash};
use platypus_sim::{CHUNK, CHUNK_AREA, Cell, CellPos, Chunk, ChunkPos, Climate, MaterialId, MaterialTable};

pub mod arena;
pub mod biome;
pub mod caves;
pub mod flora;
pub mod islands;
pub mod lairs;
pub mod minerals;
pub mod plan;
pub mod structures;

use std::sync::Arc;

use flora::{Foliage, TreePart};
pub use arena::ArenaGen;
pub use biome::Biome;
use islands::IslandCell;
use structures::{Glyph, StructureKind};
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

    /// A chunk and what it starts with besides cells (see `Spawn`), each
    /// reported by exactly one chunk; the game makes each once (an
    /// unmodified chunk is generated again when it comes back into view).
    fn generate_with_spawns(&self, pos: ChunkPos) -> (Chunk, Vec<(CellPos, Spawn)>) {
        (self.generate(pos), Vec::new())
    }

    /// The underground biome at a cell (`fungal`, `crystal`, `toxic`), if
    /// any: ambient life keyed to it.
    fn zone_at(&self, _x: i32, _y: i32) -> Option<&'static str> {
        None
    }

    /// Named places (a spider nest's middle), for tools and tests.
    fn landmarks(&self) -> Vec<(CellPos, String)> {
        Vec::new()
    }

    /// Whether the world has life of its own: enemies about the start,
    /// critters coming and going. (Not the arena: only what's put there.)
    fn wild(&self) -> bool {
        true
    }

    fn in_bounds(&self, pos: ChunkPos) -> bool {
        let (lo, hi) = self.bounds();
        pos.x >= lo.x && pos.y >= lo.y && pos.x <= hi.x && pos.y <= hi.y
    }
}

/// What the world puts somewhere that isn't cells, at a place (the feet:
/// bottom middle).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spawn {
    /// A creature, by id: a crypt's guard.
    Creature(&'static str),
    /// A chest, its loot rolled from where it was put.
    Chest,
}

/// A chest's size in cells (the game draws it this size; the world makes
/// room for it).
pub const CHEST_SIZE: (i32, i32) = (12, 10);

/// How far mountain ground wanders from the planned surface (overhangs,
/// arches, ledges), at full ruggedness.
const OVERHANG: f64 = 34.0;

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
    obsidian: MaterialId,
    methane: MaterialId,
    wood: MaterialId,
    leaves: MaterialId,
    needles: MaterialId,
    dark_leaves: MaterialId,
    tall_grass: MaterialId,
    ice: MaterialId,
    sandstone: MaterialId,
    slate: MaterialId,
    basalt: MaterialId,
    crypt_stone: MaterialId,
    cracked_stone: MaterialId,
    false_wall: MaterialId,
    candle: MaterialId,
    spikes: MaterialId,
    planks: MaterialId,
    ashlar: MaterialId,
    cracked_ashlar: MaterialId,
    false_ashlar: MaterialId,
    moss: MaterialId,
    cobweb: MaterialId,
    acid: MaterialId,
    fungal_soil: MaterialId,
    glowcap: MaterialId,
    mushroom_stem: MaterialId,
    mushroom_cap: MaterialId,
    mushroom_gills: MaterialId,
    mushroom_glow: MaterialId,
    fungus_shelf: MaterialId,
    glow_vine: MaterialId,
    crystal: MaterialId,
    toxic_crust: MaterialId,
    platform: MaterialId,
}

impl Ids {
    /// A structure's (wall, weak wall, illusory wall).
    fn style(&self, kind: StructureKind) -> (MaterialId, MaterialId, MaterialId) {
        match kind {
            StructureKind::Crypt => (self.crypt_stone, self.cracked_stone, self.false_wall),
            StructureKind::Castle => (self.ashlar, self.cracked_ashlar, self.false_ashlar),
            StructureKind::Sunken => (self.sand, self.sand, self.sand),
        }
    }
}

/// Terrain rasterised from a `WorldPlan`: hills with cliffs, dirt over
/// stone, noise caves, pockets of sand, gravel, water, oil, and lava in the
/// underworld. The plan grows stage by stage (DESIGN §3.2).
pub struct TerrainGen {
    plan: Arc<WorldPlan>,
    ids: Ids,
    pockets: Perlin,
    strata: Perlin,
    /// Starting heat per material id (lava is born hot).
    heat: Vec<i16>,
    /// Ragged edges of tree crowns; tall grass height.
    leaf_edge: Perlin,
    meadow: Perlin,
    overhang: Fbm<Perlin>,
    /// Cavern chambers; stalactites and pillars; the underworld's vault.
    caverns: Fbm<Perlin>,
    drips: Perlin,
    vault: Fbm<Perlin>,
    /// Ores and gems, and the noise they lie in.
    ores: Vec<minerals::Ore>,
    gems: Vec<minerals::Gem>,
    minerals: Perlin,
    /// Materials laid in a pattern (crypt stone, planks): their shades over
    /// a chunk-sized tile, by material id.
    patterns: Vec<Option<Vec<u8>>>,
    /// By material id: solid (static or powder), a plant.
    solid: Vec<bool>,
    plant: Vec<bool>,
    /// Lairs (`with_lairs`), and which each chamber is, if any.
    lairs: Vec<lairs::Lair>,
    lair_of: Vec<Option<u16>>,
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
            obsidian: mats.expect_id("obsidian"),
            methane: mats.expect_id("methane"),
            wood: mats.expect_id("wood"),
            leaves: mats.expect_id("leaves"),
            needles: mats.expect_id("needles"),
            dark_leaves: mats.expect_id("dark_leaves"),
            tall_grass: mats.expect_id("tall_grass"),
            ice: mats.expect_id("ice"),
            sandstone: mats.expect_id("sandstone"),
            slate: mats.expect_id("slate"),
            basalt: mats.expect_id("basalt"),
            crypt_stone: mats.expect_id("crypt_stone"),
            cracked_stone: mats.expect_id("cracked_stone"),
            false_wall: mats.expect_id("false_wall"),
            candle: mats.expect_id("candle"),
            spikes: mats.expect_id("spikes"),
            planks: mats.expect_id("planks"),
            ashlar: mats.expect_id("ashlar"),
            cracked_ashlar: mats.expect_id("cracked_ashlar"),
            false_ashlar: mats.expect_id("false_ashlar"),
            moss: mats.expect_id("moss"),
            cobweb: mats.expect_id("cobweb"),
            acid: mats.expect_id("acid"),
            fungal_soil: mats.expect_id("fungal_soil"),
            glowcap: mats.expect_id("glowcap"),
            mushroom_stem: mats.expect_id("mushroom_stem"),
            mushroom_cap: mats.expect_id("mushroom_cap"),
            mushroom_gills: mats.expect_id("mushroom_gills"),
            mushroom_glow: mats.expect_id("mushroom_glow"),
            fungus_shelf: mats.expect_id("fungus_shelf"),
            glow_vine: mats.expect_id("glow_vine"),
            crystal: mats.expect_id("crystal"),
            toxic_crust: mats.expect_id("toxic_crust"),
            platform: mats.expect_id("platform"),
        };

        let (ores, gems) = minerals::rules(&plan, mats);
        TerrainGen {
            plan,
            ids,
            ores,
            gems,
            minerals: Perlin::new(s(18)),
            solid: mats.iter().map(|(id, _)| matches!(mats.phys(id).kind, platypus_sim::Kind::Static | platypus_sim::Kind::Powder)).collect(),
            plant: mats.iter().map(|(id, _)| mats.phys(id).kind == platypus_sim::Kind::Plant).collect(),
            patterns: mats
                .iter()
                .map(|(id, _)| mats.pattern_shade(id, 0, 0).map(|_| (0..CHUNK * CHUNK).map(|i| mats.pattern_shade(id, i % CHUNK, i / CHUNK).unwrap_or(136)).collect()))
                .collect(),
            leaf_edge: Perlin::new(s(7)),
            meadow: Perlin::new(s(8)),
            overhang: Fbm::<Perlin>::new(s(9)).set_octaves(3).set_frequency(1.0 / 60.0),
            caverns: Fbm::<Perlin>::new(s(15)).set_octaves(3).set_frequency(1.0 / 600.0),
            drips: Perlin::new(s(16)),
            vault: Fbm::<Perlin>::new(s(17)).set_octaves(3).set_frequency(1.0 / 400.0),
            pockets: Perlin::new(s(5)),
            strata: Perlin::new(s(6)),
            heat: mats.iter().map(|(id, _)| mats.phys(id).heat).collect(),
            lairs: Vec::new(),
            lair_of: Vec::new(),
        }
    }

    /// Lairs in its caves (`lairs.rs`): which chambers they take is from the
    /// seed.
    pub fn with_lairs(mut self, defs: &[lairs::LairDef], mats: &MaterialTable) -> Self {
        self.lair_of = lairs::place(&self.plan, &self.plan.caves, defs);
        self.lairs = lairs::ready(defs, mats);
        self
    }

    /// Lairs dress their chambers: the lining on the walls (where rock is
    /// within two cells), threads of it from the roof.
    fn dress_lairs(&self, pos: ChunkPos, cells: &mut [Cell]) {
        let o = pos.origin();
        for &ci in self.plan.caves.chambers_in(pos.x, pos.y) {
            let Some(Some(k)) = self.lair_of.get(ci as usize) else { continue };
            let (c, lair) = (&self.plan.caves.chambers[ci as usize], &self.lairs[*k as usize]);
            let at = |lx: i32, ly: i32| (0..CHUNK).contains(&lx) && (0..CHUNK).contains(&ly);
            let solid = |cells: &[Cell], lx: i32, ly: i32| at(lx, ly) && self.solid[cells[(ly * CHUNK + lx) as usize].material.0 as usize];
            let mut lining = Vec::new();
            for ly in 0..CHUNK {
                for lx in 0..CHUNK {
                    let (x, y) = (o.x + lx, o.y + ly);
                    let (dx, dy) = ((x as f32 - c.x) / c.rx, (y as f32 - c.y) / c.ry);
                    if dx * dx + dy * dy > 1.6 || !cells[(ly * CHUNK + lx) as usize].is_air() {
                        continue;
                    }
                    let h = hash(&[self.plan.seed, 0x3EB, x as u64, y as u64]);
                    let wall = (-2..=2).any(|oy| (-2..=2).any(|ox| solid(cells, lx + ox, ly + oy)));
                    let clumps = hash(&[self.plan.seed, 0x3EC, (x >> 2) as u64, (y >> 2) as u64]) % 1000;
                    if wall && (clumps as f32) < lair.density * 1000.0 && !h.is_multiple_of(3) {
                        lining.push((lx, ly));
                    }
                    // A thread from the roof.
                    if solid(cells, lx, ly + 1) && h.is_multiple_of(11) {
                        let long = 3 + (h >> 8) as i32 % 12;
                        for d in 0..long {
                            if !at(lx, ly - d) || !cells[((ly - d) * CHUNK + lx) as usize].is_air() {
                                break;
                            }
                            lining.push((lx, ly - d));
                        }
                    }
                }
            }
            for (lx, ly) in lining {
                cells[(ly * CHUNK + lx) as usize] = Cell::new(lair.lining, (hash(&[lx as u64, ly as u64, 0x3ED]) & 255) as u8);
            }
        }
    }

    /// Lairs' keepers, from the chunk its chamber's middle is in: across the
    /// middle (they fall to the floor).
    fn lair_spawns(&self, pos: ChunkPos) -> Vec<(CellPos, Spawn)> {
        let mut out = Vec::new();
        for &ci in self.plan.caves.chambers_in(pos.x, pos.y) {
            let Some(Some(k)) = self.lair_of.get(ci as usize) else { continue };
            let c = &self.plan.caves.chambers[ci as usize];
            let mid = CellPos::new(c.x as i32, c.y as i32);
            if mid.chunk() != pos {
                continue;
            }
            let keepers = &self.lairs[*k as usize].keepers;
            let gap = (c.rx * 1.2 / keepers.len().max(1) as f32).min(10.0);
            for (n, kind) in keepers.iter().enumerate() {
                let dx = (n as f32 - (keepers.len() as f32 - 1.0) / 2.0) * gap;
                out.push((CellPos::new(mid.x + dx as i32, mid.y), Spawn::Creature(kind)));
            }
        }
        out
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
        (front, self.background_at(x, y, &self.trees_near(x, x)).0)
    }

    /// Trees (ground and sky islands) overlapping `x0..=x1`.
    fn trees_near(&self, x0: i32, x1: i32) -> Vec<&flora::Tree> {
        let mut t = self.plan.forest.near(x0, x1);
        t.extend(self.plan.island_forest.near(x0, x1));
        t
    }

    /// The background layer: walls underground (what you see in caves), trees
    /// above ground. Trees come with a shade (bark lit from one side, crowns
    /// lighter on top).
    fn background_at(&self, x: i32, y: i32, trees: &[&flora::Tree]) -> (MaterialId, Option<u8>) {
        let i = &self.ids;
        // (Behind a chest, whatever would be there.)
        if let Some((g, kind)) = self.plan.structures.glyph_at(x, y).filter(|(g, _)| *g != Glyph::Chest) {
            return (if g == Glyph::Sky { i.air } else { i.style(kind).0 }, None);
        }
        // Wood before leaves, whichever tree they belong to: a neighbour's
        // crown must not hide a trunk (its own leaves would lose their wood).
        let mut leaves = None;
        for t in trees {
            match t.part_at(x, y, &self.leaf_edge) {
                Some(TreePart::Wood(shade)) => return (i.wood, Some(shade)),
                Some(TreePart::Leaves(kind, shade)) if leaves.is_none() => leaves = Some((kind, shade)),
                _ => {}
            }
        }
        if let Some((kind, shade)) = leaves {
            let m = match kind {
                Foliage::Leaves => i.leaves,
                Foliage::Needles => i.needles,
                Foliage::Dark => i.dark_leaves,
            };
            return (m, Some(shade));
        }
        if let Some(isl) = self.plan.island_at(x)
            && isl.at(x, y) != IslandCell::None
        {
            return (i.stone, None);
        }
        let depth = self.surface_at(x) - y;
        if depth > 16
            && let Some((m, foot)) = self.plan.caves.mushroom_at(x, y)
            && self.rooted(foot)
        {
            return match m {
                caves::Shroom::Stem => (i.mushroom_stem, None),
                caves::Shroom::Cap(shade) => (i.mushroom_cap, Some(shade)),
                caves::Shroom::Gills => (i.mushroom_gills, None),
                caves::Shroom::Glow => (i.mushroom_glow, None),
            };
        }
        let wall = if depth > 16 { self.rock(x, y, self.plan.band_at(y)) } else if depth > 6 { i.dirt } else { i.air };
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
        let plan = &*self.plan;
        if y < 6 + (hash(&[plan.seed, 77, x as u64]) % 4) as i32 {
            return i.bedrock;
        }
        if let Some((g, kind)) = plan.structures.glyph_at(x, y) {
            return self.built(g, kind, x, y);
        }
        let surface = plan.surface_at(x);
        let (xf, yf) = (x as f64, y as f64);
        // Steep mountain faces wander sideways: the ground here is the
        // planned surface a little way off, so they get overhangs and ledges
        // while crests and gentle slopes (where trees stand) stay put.
        let rugged = plan.rugged_at(x) as f64;
        let slope = (plan.surface_at(x + 4) - plan.surface_at(x - 4)).abs() as f64 / 8.0;
        let wander = OVERHANG * rugged * ((slope - 0.8) / 0.8).clamp(0.0, 1.0);
        let ground = if wander > 0.5 { plan.surface_at(x + (self.overhang.get([xf, yf * 1.3]) * wander) as i32) } else { surface };
        let depth = ground - y;
        // The ground's own temperature: snow and ice where it's freezing.
        let cold = plan.climate.ambient(x, surface) <= 0;
        if depth <= 0 {
            if let Some(w) = plan.water_at(x)
                && y < w
                && y >= surface
            {
                return if plan.climate.ambient(x, y) <= 0 { i.ice } else { i.water };
            }
            if let Some(isl) = plan.island_at(x) {
                match isl.at(x, y) {
                    IslandCell::Grass => return if plan.climate.ambient(x, y) <= 0 { i.snow } else { i.grass },
                    IslandCell::Dirt => return i.dirt,
                    IslandCell::Stone => return i.stone,
                    IslandCell::Cave | IslandCell::None => {}
                }
            }
            return i.air;
        }

        let band = plan.band_at(y);
        if band == Band::Underworld {
            return self.underworld(x, y);
        }
        if plan.chasm_at(x, y) {
            return i.air;
        }
        if let Some(open) = self.cavity(x, y, depth, band) {
            return open;
        }
        let under_water = plan.water_at(x).is_some();

        let biome = plan.biome_at(x);
        let soil = 8 + (self.strata.get([xf / 40.0, 0.5]) * 5.0) as i32;
        if under_water {
            // Lake and sea beds: sand over the usual ground.
            if depth <= 5 + soil / 2 {
                return i.sand;
            }
        } else if matches!(biome, Biome::Ocean | Biome::Desert) && slope < 1.2 {
            let sand = if biome == Biome::Desert { 30 + (self.strata.get([xf / 90.0, 2.5]) * 14.0) as i32 } else { 12 };
            if depth <= sand {
                return i.sand;
            }
            if biome == Biome::Desert && depth <= sand + 60 {
                return i.sandstone;
            }
        } else {
            // Snow where the ground freezes (deeper the colder), bare rock
            // on rugged slopes, grass and dirt elsewhere.
            // (Not on steep faces: snow slides off, so ledges hold it.)
            // (Very cold, it clings to steeper faces too: snow-filled peaks.)
            let t = plan.climate.ambient(x, surface);
            if cold && slope < if t <= -15 { 5.0 } else if t <= -8 { 3.0 } else { 1.6 } {
                // As thick across a steep face as on the flat (depth here is
                // measured straight down).
                if depth as f64 <= ((2 - t).min(24) as f64) * (1.0 + slope) {
                    return i.snow;
                }
            }
            // Soil thins on steep ground; bare rock on cliffs.
            let soil = (soil as f64 * (1.0 - (slope - 0.6) / 0.8).clamp(0.0, 1.0)) as i32;
            if depth == 1 && soil > 0 && !cold {
                return i.grass;
            }
            if depth <= soil {
                return i.dirt;
            }
            if rugged > 0.3 && depth <= 3 && self.pockets.get([xf / 12.0, yf / 12.0, 5.5]) > 0.3 {
                return i.gravel;
            }
        }
        // Sealed gas bubbles deep in the rock: bomb or dig into one with fire nearby.
        if depth > 90 && self.pockets.get([xf / 34.0, yf / 22.0, 23.3]) > 0.66 {
            return i.methane;
        }
        let shallow = matches!(band, Band::Sky | Band::Peaks | Band::Surface | Band::Underground);
        // Pockets inside rock: sand and gravel up high, rarer below.
        let p = self.pockets.get([xf / 45.0, yf / 45.0, 7.1]);
        if p > if shallow { 0.55 } else { 0.7 } && depth > 12 {
            return i.sand;
        }
        if p < if shallow { -0.6 } else { -0.7 } && depth > 12 {
            return i.gravel;
        }
        if let Some(m) = self.mineral(x, y, band) {
            return m;
        }
        self.rock(x, y, band)
    }

    /// A dry cave floor with room to stand (8 × 20 cells), searching out from
    /// `x` and down from `y`.
    fn cave_floor_near(&self, x: i32, y: i32) -> Option<CellPos> {
        let air = |x: i32, y: i32| self.material_at(x, y) == self.ids.air;
        let solid = |x: i32, y: i32| {
            let m = self.material_at(x, y);
            m != self.ids.air && m != self.ids.water && m != self.ids.lava && m != self.ids.oil && m != self.ids.acid
        };
        (0..3_000).step_by(8).flat_map(|d| [x + d, x - d]).find_map(|x| {
            (y - 800..=y).rev().find(|&y| solid(x, y - 1) && (0..20).all(|dy| air(x, y + dy) && air(x + 7, y + dy)) && solid(x + 7, y - 1)).map(|y| CellPos::new(x + 4, y + 2))
        })
    }

    /// What a structure's glyph makes at a cell. Candles and spikes are
    /// shapes inside their block; chests aren't cells (see
    /// `structure_spawns`).
    fn built(&self, g: Glyph, kind: StructureKind, x: i32, y: i32) -> MaterialId {
        let i = &self.ids;
        let (bx, by) = (x & 3, y & 3);
        let (wall, weak, illusory) = i.style(kind);
        let s = &self.plan.structures;
        let glyph = |dx: i32, dy: i32| s.glyph_at(x + dx * 4, y + dy * 4).map(|(g, _)| g);
        // Age, from the place alone: a block's roll, and a cell's.
        let block = hash(&[self.plan.seed, 0xA6E, (x >> 2) as u64, (y >> 2) as u64]);
        let cell = hash(&[self.plan.seed, 0xA6F, x as u64, y as u64]);
        let crypt = kind == StructureKind::Crypt;
        match g {
            // A crypt's ceiling has fallen in here and there.
            Glyph::Wall if crypt && block % 100 < 4 && glyph(0, -1) == Some(Glyph::Open) && glyph(0, 1) == Some(Glyph::Wall) => i.gravel,
            Glyph::Wall => wall,
            Glyph::Open => {
                // Moss on a crypt's floors, in patches.
                if crypt && by == 0 && glyph(0, -1) == Some(Glyph::Wall) && block.is_multiple_of(3) && !cell.is_multiple_of(4) {
                    return i.moss;
                }
                // Cobwebs in the top corners of rooms: a triangle up to
                // seven cells from the corner, ragged.
                let up = if glyph(0, 1) == Some(Glyph::Wall) { 3 - by } else if glyph(0, 2) == Some(Glyph::Wall) && glyph(0, 1) == Some(Glyph::Open) { 7 - by } else { return i.air };
                let side = [(-1, bx), (1, 3 - bx)].into_iter().filter_map(|(d, near)| {
                    if glyph(d, 0) == Some(Glyph::Wall) {
                        Some(near)
                    } else if glyph(d * 2, 0) == Some(Glyph::Wall) && glyph(d, 0) == Some(Glyph::Open) {
                        Some(near + 4)
                    } else {
                        None
                    }
                });
                // (Some corners: by the 8-cell tile.)
                let webbed = hash(&[self.plan.seed, 0xA70, (x >> 3) as u64, (y >> 3) as u64]).is_multiple_of(3);
                match side.min() {
                    Some(d) if up + d < 7 && webbed && !cell.is_multiple_of(5) => i.cobweb,
                    _ => i.air,
                }
            }
            Glyph::Weak => weak,
            Glyph::Illusory => illusory,
            Glyph::Planks => i.planks,
            Glyph::Platform if by >= 2 => i.platform,
            Glyph::Rubble => i.gravel,
            Glyph::Water => i.water,
            Glyph::Lava => i.lava,
            Glyph::Candle if bx == 1 && by <= 2 => i.candle,
            // Two spikes a block, pointing up.
            Glyph::Spikes if by == 0 || (by == 1 && bx != 3) || (by == 2 && bx == 1) => i.spikes,
            _ => i.air,
        }
    }

    /// Rock floating in open space (a stalactite a tunnel cut off, a
    /// crystal whose wall another cave took): any solid piece lying wholly
    /// inside this chunk, touching none of its edges, becomes what it floats
    /// in. (Pieces reaching over an edge are left: this chunk can't see them
    /// whole.) Structures keep theirs.
    fn sweep_specks(&self, pos: ChunkPos, cells: &mut [Cell]) {
        let mats_solid = |c: Cell| !c.is_air() && self.solid[c.material.0 as usize];
        let built = self.plan.structures.pieces_in(pos.x, pos.y).next().is_some();
        let o = pos.origin();
        let n = CHUNK as usize;
        let mut seen = vec![false; n * n];
        let mut piece = Vec::new();
        let mut stack = Vec::new();
        for start in 0..n * n {
            if seen[start] || !mats_solid(cells[start]) {
                continue;
            }
            piece.clear();
            stack.push(start);
            seen[start] = true;
            let (mut edge, mut around) = (false, None);
            while let Some(i) = stack.pop() {
                piece.push(i);
                let (x, y) = (i % n, i / n);
                edge |= x == 0 || y == 0 || x == n - 1 || y == n - 1;
                for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                    let (qx, qy) = (x as i32 + dx, y as i32 + dy);
                    if qx < 0 || qy < 0 || qx >= CHUNK || qy >= CHUNK {
                        continue;
                    }
                    let q = qy as usize * n + qx as usize;
                    if mats_solid(cells[q]) {
                        if !seen[q] {
                            seen[q] = true;
                            stack.push(q);
                        }
                    } else if around.is_none() || cells[q].is_air() {
                        around = Some(cells[q]);
                    }
                }
            }
            if edge || piece.len() > 600 {
                continue;
            }
            if built && piece.iter().any(|&i| self.plan.structures.glyph_at(o.x + (i % n) as i32, o.y + (i / n) as i32).is_some()) {
                continue;
            }
            // What it floats in: a liquid if it's in one, else air (plants
            // on it go too, below, where they lose their hold).
            let fill = around.filter(|c| !self.solid[c.material.0 as usize] && !self.plant[c.material.0 as usize]).unwrap_or(Cell::AIR);
            for &i in &piece {
                cells[i] = fill;
            }
        }
    }

    /// The underground biomes dress the rock where it meets open space:
    /// fungal soil and glowing sprouts in the fungal hollows, crystal studs
    /// in the crystal caves, a glowing crust in the toxic grottos.
    fn dress(&self, pos: ChunkPos, cells: &mut [Cell], rng: &mut Rng) {
        let i = &self.ids;
        let o = pos.origin();
        let areas: Vec<&caves::Area> = self
            .plan
            .caves
            .areas
            .iter()
            .filter(|a| {
                let (cx, cy) = ((o.x + CHUNK / 2) as f32, (o.y + CHUNK / 2) as f32);
                ((cx - a.x).abs() - CHUNK as f32) < a.rx && ((cy - a.y).abs() - CHUNK as f32) < a.ry
            })
            .collect();
        if areas.is_empty() {
            return;
        }
        let built = self.plan.structures.pieces_in(pos.x, pos.y).next().is_some();
        // The chunk as generated, with a margin of two cells from the chunks
        // around (asked once each).
        const M: i32 = 2;
        const W: i32 = CHUNK + 2 * M;
        let mut before = vec![MaterialId::AIR; (W * W) as usize];
        for y in -M..CHUNK + M {
            for x in -M..CHUNK + M {
                before[((y + M) * W + x + M) as usize] = if (0..CHUNK).contains(&x) && (0..CHUNK).contains(&y) {
                    cells[(y * CHUNK + x) as usize].material
                } else {
                    self.material_at(o.x + x, o.y + y)
                };
            }
        }
        let at = |x: i32, y: i32| before[((y + M) * W + x + M) as usize];
        let open = |m: MaterialId| m == i.air || m == i.water || m == i.acid || m == i.lava;
        let rock = |m: MaterialId| m == i.stone || m == i.slate || m == i.dirt || m == i.basalt;
        let near_open = |x: i32, y: i32, r: i32| (-r..=r).any(|dy| (-r..=r).any(|dx| open(at(x + dx, y + dy))));
        let set = |cells: &mut [Cell], lx: i32, ly: i32, m: MaterialId, shade: u8| {
            cells[(ly * CHUNK + lx) as usize] = Cell { heat: self.heat[m.0 as usize], ..Cell::new(m, shade) };
        };
        for ly in 0..CHUNK {
            for lx in 0..CHUNK {
                let (x, y) = (o.x + lx, o.y + ly);
                let Some(zone) = areas.iter().find(|a| a.contains(x as f32, y as f32)).map(|a| a.zone) else { continue };
                if built && self.plan.structures.glyph_at(x, y).is_some() {
                    continue;
                }
                let m = at(lx, ly);
                let h = hash(&[self.plan.seed, 0xD7E5, x as u64, y as u64]);
                match zone {
                    caves::Zone::Fungal => {
                        if rock(m) && near_open(lx, ly, 2) {
                            set(cells, lx, ly, i.fungal_soil, rng.next_u8());
                        } else if m == i.air && rock(at(lx, ly + 1)) && h.is_multiple_of(7) {
                            // A vine hanging from the ceiling, brighter toward its tip.
                            let long = 4 + (h >> 8) as i32 % 16;
                            for k in 0..long {
                                if ly - k < 0 || at(lx, ly - k) != i.air {
                                    break;
                                }
                                set(cells, lx, ly - k, i.glow_vine, (k * 255 / long).min(255) as u8);
                            }
                        } else if m == i.air && ly > 0 && rock(at(lx, ly - 1)) && h.is_multiple_of(5) {
                            // A sprout: a stem or two, a glowing tip.
                            let tall = 1 + (h >> 8) as i32 % 3;
                            for k in 0..=tall {
                                if ly + k >= CHUNK || at(lx, ly + k) != i.air {
                                    break;
                                }
                                set(cells, lx, ly + k, i.glowcap, if k == tall { 230 } else { 40 + (h >> 16) as u8 % 60 });
                            }
                        }
                    }
                    caves::Zone::Crystal => {
                        // Studs of two by two.
                        if rock(m) && near_open(lx, ly, 1) && hash(&[self.plan.seed, 0xC7A1, (x >> 1) as u64, (y >> 1) as u64]).is_multiple_of(5) {
                            set(cells, lx, ly, i.crystal, rng.next_u8());
                        }
                    }
                    caves::Zone::Toxic => {
                        // (Anything touching the acid is crust, ore and gravel
                        // too: acid eats what isn't, so a pool with one ore in
                        // its lining would eat its way out into the rock.)
                        let touches_acid = || (-1..=1).any(|dy| (-1..=1).any(|dx| at(lx + dx, ly + dy) == i.acid));
                        if (rock(m) && near_open(lx, ly, 1)) || (!open(m) && m != i.toxic_crust && touches_acid()) {
                            set(cells, lx, ly, i.toxic_crust, rng.next_u8());
                        }
                    }
                }
            }
        }
    }

    /// A structure's chests and guards whose feet are in this chunk.
    fn structure_spawns(&self, pos: ChunkPos) -> Vec<(CellPos, Spawn)> {
        let o = pos.origin();
        let inside = |(x, y): &(i32, i32)| (o.x..o.x + CHUNK).contains(x) && (o.y..o.y + CHUNK).contains(y);
        let mut out = Vec::new();
        for (s, piece) in self.plan.structures.pieces_in(pos.x, pos.y) {
            // The dead keep crypts; castles are orcs'.
            let guard = if s.kind == StructureKind::Crypt { "skeleton" } else { "orc" };
            // (A chest's glyph is its bottom-left block of four: its feet
            // are two blocks in.)
            for (x, y) in piece.blocks_of(Glyph::Chest).map(|(x, y)| (x + 4, y)).filter(inside) {
                out.push((CellPos::new(x, y), Spawn::Chest));
            }
            for (x, y) in piece.blocks_of(Glyph::Spawn).map(|(x, y)| (x + 2, y)).filter(inside) {
                out.push((CellPos::new(x, y), Spawn::Creature(guard)));
            }
            // A boss is a pack, until there are bosses.
            for (x, y) in piece.blocks_of(Glyph::Boss).filter(inside) {
                for dx in [-10, 2, 14] {
                    out.push((CellPos::new(x + dx, y), Spawn::Creature(guard)));
                }
            }
        }
        out
    }

    /// Ore or a gem at a cell of rock, if there is one (minerals.rs).
    fn mineral(&self, x: i32, y: i32, band: Band) -> Option<MaterialId> {
        let (xf, yf) = (x as f64, y as f64);
        for g in self.gems.iter().filter(|g| g.band == band) {
            let n = self.minerals.get([xf / minerals::GEM_SCALE, yf / minerals::GEM_SCALE, g.salt]);
            if n > minerals::GEM_THRESHOLD
                && self.minerals.get([xf / minerals::GEM_ZONE_SCALE, yf / minerals::GEM_ZONE_SCALE, g.salt + 0.5]) > minerals::GEM_ZONE
                && self.open_near(x, y, minerals::GEM_REACH)
            {
                return Some(g.material);
            }
        }
        // (Whether a cave is near is asked at most once, and only when it
        // could matter.)
        let mut exposed = None;
        for o in &self.ores {
            let Some(threshold) = o.threshold_at(y) else { continue };
            let n = self.minerals.get([xf / o.scale.0, yf / o.scale.1, o.salt]);
            if n > threshold || (n > threshold - minerals::EXPOSED_BONUS && *exposed.get_or_insert_with(|| self.open_near(x, y, minerals::EXPOSED_REACH))) {
                return Some(o.material);
            }
        }
        None
    }

    /// Is there open space (a cave, a chasm, the surface) `r` cells away, in
    /// any of the four directions? As generated, from the plan alone.
    fn open_near(&self, x: i32, y: i32, r: i32) -> bool {
        [(r, 0), (-r, 0), (0, r), (0, -r)].into_iter().any(|(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            let depth = self.plan.surface_at(nx) - ny;
            let band = self.plan.band_at(ny);
            depth <= 0 || (band != Band::Underworld && (self.plan.chasm_at(nx, ny) || self.cavity(nx, ny, depth, band).is_some()))
        })
    }

    /// A chest in a cave pocket (DESIGN §3.2): some chunks (more the
    /// deeper) get one on the first cave floor found, with room for it (air
    /// on solid ground, inside the chunk). Its feet.
    fn cave_chest(&self, pos: ChunkPos, cells: &[Cell], rng: &mut Rng) -> Option<CellPos> {
        let (w, h) = CHEST_SIZE;
        let origin = pos.origin();
        // (Most chunks have no cave with room for one: these are tries.)
        let chance = match self.plan.band_at(origin.y + CHUNK / 2) {
            Band::Underground => 36,
            Band::Caverns => 54,
            Band::Deep => 72,
            _ => return None,
        };
        if !rng.chance(chance) {
            return None;
        }
        // Every spot (columns 4 apart, any height), from a random start: the
        // first cave floor.
        let spots: Vec<(i32, i32)> = (0..=(CHUNK - w) / 4).flat_map(|i| (1..=CHUNK - h).map(move |ly| (i * 4, ly))).collect();
        let start = rng.next_u32() as usize % spots.len();
        let at = |lx: i32, ly: i32| cells[(ly * CHUNK + lx) as usize];
        let (lx, ly) = (0..spots.len()).map(|k| spots[(start + k) % spots.len()]).find(|&(lx, ly)| {
            let open = (0..h).all(|dy| (0..w).all(|dx| at(lx + dx, ly + dy).is_air()));
            // Standing on rock (most of it: caves aren't flat), not over water
            // or lava.
            let floor = (0..w)
                .filter(|&dx| {
                    let c = at(lx + dx, ly - 1);
                    !c.is_air() && c.material != self.ids.water && c.material != self.ids.lava
                })
                .count();
            open && floor >= (w * 3 / 4) as usize
        })?;
        Some(CellPos::new(origin.x + lx + w / 2, origin.y + ly))
    }

    /// The bedrock of a band: stone, then slate in the deep (with obsidian
    /// seams), basalt in the underworld; borders dither over ~100 cells.
    fn rock(&self, x: i32, y: i32, band: Band) -> MaterialId {
        let i = &self.ids;
        let (xf, yf) = (x as f64, y as f64);
        let (deep_lo, deep_hi) = self.plan.band_span(Band::Deep);
        let jitter = (self.strata.get([xf / 30.0, yf / 30.0]) * 60.0) as i32;
        if band == Band::Underworld || y + jitter < deep_lo {
            return i.basalt;
        }
        if y + jitter <= deep_hi {
            if self.strata.get([xf / 70.0, yf / 70.0]) > 0.5 {
                return i.obsidian;
            }
            return i.slate;
        }
        i.stone
    }

    /// Open space underground, and what fills it: the planned chambers and
    /// tunnels (`caves.rs`), and in the caverns and the deep, huge chambers
    /// (with stalactites and pillars, flooded below the water table).
    fn cavity(&self, x: i32, y: i32, depth: i32, band: Band) -> Option<MaterialId> {
        let i = &self.ids;
        let plan = &*self.plan;
        if let Some(open) = plan.caves.at(x, y) {
            // One water table for every cave in the caverns: where the
            // tunnels meet the flooded chambers, the water's already level.
            let air = if band == Band::Caverns && y < plan.water_table(x) { i.water } else { i.air };
            match open {
                caves::Open::Air => return Some(air),
                caves::Open::Pool(caves::Pool::Water) => return Some(if plan.climate.ambient(x, y) <= 0 { i.ice } else { i.water }),
                caves::Open::Pool(caves::Pool::Oil) => return Some(i.oil),
                caves::Open::Pool(caves::Pool::Lava) => return Some(i.lava),
                caves::Open::Pool(caves::Pool::Acid) => return Some(i.acid),
                caves::Open::Grown { what, root, inside } => {
                    if self.rooted(root) {
                        return match what {
                            caves::Growth::Crystal => Some(i.crystal),
                            caves::Growth::Shelf => Some(i.fungus_shelf),
                            caves::Growth::Ledge => None,
                        };
                    }
                    // (Its wall is gone: no growth.)
                    if inside {
                        return Some(air);
                    }
                }
            }
        }
        self.cavern(x, y, depth, band)
    }

    /// Is this cell rock once every cave is carved? (Where something grown
    /// from a wall is rooted.)
    fn rooted(&self, (x, y): (i32, i32)) -> bool {
        let plan = &*self.plan;
        let depth = plan.surface_at(x) - y;
        depth > 0
            && !matches!(plan.caves.at(x, y), Some(caves::Open::Air | caves::Open::Pool(_)))
            && self.cavern(x, y, depth, plan.band_at(y)).is_none()
    }

    /// The caverns' and the deep's huge chambers (noise), and what fills
    /// them.
    fn cavern(&self, x: i32, y: i32, depth: i32, band: Band) -> Option<MaterialId> {
        let i = &self.ids;
        let plan = &*self.plan;
        if !matches!(band, Band::Caverns | Band::Deep) {
            return None;
        }
        let (xf, yf) = (x as f64, y as f64);
        // Kept away from lake and ocean beds.
        let under_water = plan.water_at(x).is_some();
        if depth < if under_water { 120 } else { 60 } {
            return None;
        }
        // Chambers, wider than tall, with stalactites: each column of a
        // chamber hangs one as long as a smooth noise across the columns
        // says (so they taper to points), rock wherever the ceiling is
        // within that of it. Hanging from the real ceiling, they never
        // float, and floors stay clear. (Fading in over the band's top 300
        // cells.)
        let entry = ((plan.band_span(Band::Caverns).1 - y) as f64 / 300.0).clamp(0.0, 1.0);
        let fade = if band == Band::Caverns { entry } else { 1.0 };
        let base = |y: f64| self.caverns.get([xf / 1.6, y]) * fade;
        if base(yf) <= 0.22 {
            return None;
        }
        let long = ((self.drips.get([xf / 6.0, 0.3, 7.7]).abs() - 0.35) * 110.0).max(0.0);
        // How far up the ceiling is, from how fast the chamber closes going
        // up (smooth, so a stalactite has no gaps in it).
        let chamber = long < 1.0 || {
            let open = base(yf);
            let closing = (open - base(yf + 4.0)) / 4.0;
            let up = if closing > 1e-5 { (open - 0.22) / closing } else { f64::MAX };
            // (And nothing else carved between here and that ceiling: a
            // tunnel through it would leave the rest hanging.)
            // (And the ceiling really is there: the estimate's short where
            // the ceiling domes up, which would leave a stalactite hanging
            // under the dome.)
            up >= long || base(yf + up + 2.0) > 0.22 || (1..=(up as i32 + 3) / 3).any(|k| plan.caves.at(x, y + k * 3).is_some())
        };
        chamber.then(|| if y < plan.water_table(x) && band == Band::Caverns { i.water } else { i.air })
    }

    /// The underworld: a lava sea under a huge vault with a ragged roof, rock
    /// islands hanging in it.
    fn underworld(&self, x: i32, y: i32) -> MaterialId {
        let i = &self.ids;
        let (xf, yf) = (x as f64, y as f64);
        let (lo, hi) = self.plan.band_span(Band::Underworld);
        let h = (hi - lo) as f64;
        // One level, so the sea is asleep on load.
        let lava = (lo as f64 + h * 0.2).floor();
        let roof = hi as f64 - h * 0.15 + self.vault.get([xf, 3.3]) * h * 0.12 - self.drips.get([xf / 9.0, yf / 50.0, 1.7]).abs() * h * 0.05;
        if yf > roof {
            return self.rock(x, y, Band::Underworld);
        }
        // Islands: blobs of basalt, crusted with obsidian near the lava.
        let island = self.vault.get([xf / 1.3, yf * 1.1 + 900.0]);
        if island > 0.3 {
            return if yf < lava + 30.0 { i.obsidian } else { i.basalt };
        }
        if yf < lava { i.lava } else { i.air }
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

    fn landmarks(&self) -> Vec<(CellPos, String)> {
        self.lair_of
            .iter()
            .enumerate()
            .filter_map(|(i, l)| l.map(|k| (i, k)))
            .map(|(i, k)| {
                let c = &self.plan.caves.chambers[i];
                (CellPos::new(c.x as i32, c.y as i32), self.lairs[k as usize].name.clone())
            })
            .collect()
    }

    fn zone_at(&self, x: i32, y: i32) -> Option<&'static str> {
        self.plan.caves.zone_at(x, y).map(|z| match z {
            caves::Zone::Fungal => "fungal",
            caves::Zone::Crystal => "crystal",
            caves::Zone::Toxic => "toxic",
        })
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
        // The middle of the world (or PLATYPUS_SPAWN_X, to try a biome), on
        // the nearest dry ground.
        let var = |name: &str| std::env::var(name).ok().and_then(|v| v.parse::<i32>().ok());
        let mid = var("PLATYPUS_SPAWN_X").unwrap_or(self.plan.width / 2);
        // PLATYPUS_SPAWN_Y: the nearest cave floor at or below that height
        // instead (to look at the deep bands).
        if let Some(y) = var("PLATYPUS_SPAWN_Y")
            && let Some(p) = self.cave_floor_near(mid, y)
        {
            return p;
        }
        let x = (0..2_000).flat_map(|d| [mid + d, mid - d]).find(|&x| self.plan.water_at(x).is_none()).unwrap_or(mid);
        CellPos::new(x, self.surface_at(x) + 2)
    }

    fn generate(&self, pos: ChunkPos) -> Chunk {
        self.generate_with_spawns(pos).0
    }

    fn generate_with_spawns(&self, pos: ChunkPos) -> (Chunk, Vec<(CellPos, Spawn)>) {
        let origin = pos.origin();
        let mut rng = Rng::seeded(&[self.plan.seed, 0xC4C4, pos.x as u64, pos.y as u64]);
        let trees = self.trees_near(origin.x, origin.x + CHUNK - 1);
        let mut cells = Vec::with_capacity(CHUNK_AREA);
        let mut bg = Vec::with_capacity(CHUNK_AREA);
        let make = |m: MaterialId, rng: &mut Rng, lx: i32, ly: i32| {
            if m == self.ids.air {
                return Cell::AIR;
            }
            // Laid in a pattern, or a candle's wax and flame, or random.
            let shade = match &self.patterns[m.0 as usize] {
                Some(tile) => tile[(ly * CHUNK + lx) as usize],
                None if m == self.ids.candle => if (origin.y + ly) & 3 == 2 { 230 } else { 40 + rng.next_u8() / 4 },
                None => rng.next_u8(),
            };
            Cell { heat: self.heat[m.0 as usize], ..Cell::new(m, shade) }
        };
        for ly in 0..CHUNK {
            for lx in 0..CHUNK {
                let (x, y) = (origin.x + lx, origin.y + ly);
                let mut m = self.material_at(x, y);
                if m == self.ids.air && self.grass_at(x, y) {
                    m = self.ids.tall_grass;
                }
                cells.push(make(m, &mut rng, lx, ly));
                let (b, shade) = self.background_at(x, y, &trees);
                let mut back = make(b, &mut rng, lx, ly);
                if let Some(shade) = shade {
                    back.shade = shade;
                }
                bg.push(back);
            }
        }
        self.dress(pos, &mut cells, &mut rng);
        self.dress_lairs(pos, &mut cells);
        self.sweep_specks(pos, &mut cells);
        let mut spawns = self.structure_spawns(pos);
        spawns.extend(self.lair_spawns(pos));
        spawns.extend(self.cave_chest(pos, &cells, &mut rng).map(|p| (p, Spawn::Chest)));
        (Chunk::with_background(pos, cells, bg), spawns)
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

    /// Ores lie in their bands, harder the deeper; gems only in cave walls;
    /// ore shows on cave walls more than inside the rock.
    #[test]
    fn ores_and_gems_lie_where_they_belong() {
        use std::collections::HashMap;
        let m = mats();
        let g = TerrainGen::new(5, Preset::Large, &m);
        let names = ["coal", "copper_ore", "iron_ore", "silver_ore", "gold_ore", "mithril_ore", "amethyst", "emerald", "ruby"];
        let ids: Vec<MaterialId> = names.iter().map(|n| m.expect_id(n)).collect();
        let (mut found, mut rock, mut wall_rock, mut wall_ore, mut ore_total) = (HashMap::new(), HashMap::new(), 0u32, 0u32, 0u32);
        for x in (0..g.plan.width).step_by(61) {
            for y in (g.plan.band_span(Band::Deep).0..g.plan.band_span(Band::Underground).1).step_by(17) {
                let here = g.material_at(x, y);
                if here == MaterialId::AIR || m.phys(here).kind != platypus_sim::Kind::Static {
                    continue;
                }
                let band = g.plan.band_at(y);
                *rock.entry(band).or_insert(0u32) += 1;
                if let Some(i) = ids.iter().position(|&id| id == here) {
                    *found.entry((names[i], band)).or_insert(0u32) += 1;
                    if i >= 6 {
                        assert!(g.open_near(x, y, minerals::GEM_REACH), "{} inside the rock at {x},{y}", names[i]);
                    }
                }
                if ids[..6].contains(&here) {
                    ore_total += 1;
                }
                if [(2, 0), (-2, 0), (0, 2), (0, -2)].iter().any(|(dx, dy)| g.material_at(x + dx, y + dy) == MaterialId::AIR) {
                    wall_rock += 1;
                    wall_ore += u32::from(ids[..6].contains(&here));
                }
            }
        }
        let n = |name: &str, band: Band| found.get(&(name, band)).copied().unwrap_or(0);
        for (name, band) in [("copper_ore", Band::Underground), ("iron_ore", Band::Underground), ("iron_ore", Band::Caverns), ("silver_ore", Band::Caverns), ("gold_ore", Band::Caverns), ("gold_ore", Band::Deep), ("mithril_ore", Band::Deep), ("amethyst", Band::Underground), ("emerald", Band::Caverns), ("ruby", Band::Deep)] {
            assert!(n(name, band) > 0, "no {name} in the {}", band.name());
        }
        for (name, band) in [("copper_ore", Band::Caverns), ("copper_ore", Band::Deep), ("silver_ore", Band::Underground), ("mithril_ore", Band::Caverns), ("ruby", Band::Underground)] {
            assert_eq!(n(name, band), 0, "{name} in the {}", band.name());
        }
        // Harder ore the deeper.
        let hardness = |band: Band| {
            let (sum, count) = names[..6].iter().fold((0u32, 0u32), |(s, c), name| {
                let k = n(name, band);
                (s + k * m.phys(m.expect_id(name)).hardness as u32, c + k)
            });
            sum as f32 / count.max(1) as f32
        };
        assert!(hardness(Band::Underground) < hardness(Band::Caverns) && hardness(Band::Caverns) < hardness(Band::Deep), "ore hardness by band: {} {} {}", hardness(Band::Underground), hardness(Band::Caverns), hardness(Band::Deep));
        // Enough to find, not so much it's cheap.
        let all_rock: u32 = rock.values().sum();
        let share = ore_total as f32 / all_rock as f32;
        assert!((0.01..0.08).contains(&share), "ore is {:.1}% of the rock", share * 100.0);
        let wall_share = wall_ore as f32 / wall_rock as f32;
        eprintln!("ore {:.1}% of rock, {:.1}% of cave walls; by band: {found:?}", share * 100.0, wall_share * 100.0);
        assert!(wall_share > share * 1.3, "ore shows on cave walls: {:.1}% there vs {:.1}% overall", wall_share * 100.0, share * 100.0);
    }

    /// Crypts stand on level ground with no tree in their ruin; chunks report
    /// each chest and guard exactly once.
    #[test]
    fn crypts_are_sited_and_rasterised_whole() {
        use std::collections::HashSet;
        let m = mats();
        let g = TerrainGen::new(1, Preset::Small, &m);
        let crypts: Vec<_> = g.plan.structures.list.iter().filter(|s| s.kind == structures::StructureKind::Crypt).collect();
        assert!(crypts.len() >= 2, "a small world has crypts ({})", crypts.len());
        for c in crypts {
            let ruin = &c.pieces[0];
            let (x0, _, x1, _) = ruin.bbox();
            assert!((ruin.y + 4 - g.surface_at(c.site.0)).abs() <= 4, "the ruin's floor is the ground");
            // (A neighbour's crown may reach over it.)
            assert!(g.plan.forest.near(x0, x1).iter().all(|t| t.x < x0 - 8 || t.x > x1 + 8), "no tree grows in the ruin at {}", c.site.0);
            let (bx0, by0, bx1, by1) = c.bbox();
            let (lo, hi) = (CellPos::new(bx0, by0).chunk(), CellPos::new(bx1, by1).chunk());
            let (mut chests_seen, mut spawns) = (Vec::new(), Vec::new());
            for cy in lo.y..=hi.y {
                for cx in lo.x..=hi.x {
                    for (p, what) in g.generate_with_spawns(ChunkPos::new(cx, cy)).1 {
                        match what {
                            Spawn::Chest => chests_seen.push(p),
                            Spawn::Creature(_) => spawns.push(p),
                        }
                    }
                }
            }
            let chests: usize = c.pieces.iter().map(|p| p.blocks_of(Glyph::Chest).count()).sum();
            // (Cave chests can turn up in the crypt's chunks too.)
            let placed = chests_seen.iter().filter(|p| c.pieces.iter().any(|piece| piece.blocks_of(Glyph::Chest).any(|(x, y)| (x + 4, y) == (p.x, p.y)))).count();
            assert_eq!(placed, chests, "each chest reported once");
            let guards: usize = c.pieces.iter().map(|p| p.blocks_of(Glyph::Spawn).count() + 3 * p.blocks_of(Glyph::Boss).count()).sum();
            assert_eq!(spawns.len(), guards, "each guard reported once");
            assert_eq!(spawns.iter().collect::<HashSet<_>>().len(), guards, "no guard twice");
        }
    }

    /// Deep lakes keep a chest on their bed, under water.
    #[test]
    fn deep_lakes_hide_a_chest() {
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let sunken: Vec<_> = g.plan.structures.list.iter().filter(|s| s.kind == structures::StructureKind::Sunken).collect();
        assert!(sunken.len() >= 3, "{} lake chests", sunken.len());
        for s in sunken {
            let (x, bed) = s.site;
            assert!(g.plan.water_at(x).is_some_and(|w| w > bed + 20), "deep water over the chest at {x}");
            let piece = &s.pieces[0];
            let feet = CellPos::new(piece.x + 4, piece.y);
            assert!(g.generate_with_spawns(feet.chunk()).1.contains(&(feet, Spawn::Chest)), "the chest is on the bed at {x}");
        }
    }

    /// Along the tunnels, as generated, there's room for the player (a
    /// 6 × 15 box within a few cells of the path; ledges take half the width
    /// at some heights).
    #[test]
    fn the_player_fits_through_the_tunnels() {
        use platypus_sim::Kind;
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let solid = |x: i32, y: i32| {
            let k = m.phys(g.material_at(x, y)).kind;
            matches!(k, Kind::Static | Kind::Powder)
        };
        let (mut checked, mut blocked) = (0, Vec::new());
        for t in g.plan.caves.tunnels.iter().filter(|t| !t.crevice()).step_by(11).take(160) {
            for &(px, py) in &t.points[1..t.points.len() - 1] {
                let (x, y) = (px as i32, py as i32);
                // (Structures and chasms are their own business.)
                if g.plan.structures.glyph_at(x, y).is_some() || g.plan.chasm_at(x, y) || y >= g.surface_at(x) - 20 {
                    continue;
                }
                checked += 1;
                let fits = (-8..=8).any(|dx| (-10..=2).any(|dy| (0..6).all(|bx| (0..15).all(|by| !solid(x + dx + bx - 3, y + dy + by - 7)))));
                if !fits {
                    blocked.push((x, y));
                }
            }
        }
        assert!(checked > 500, "{checked} points");
        assert!(blocked.len() * 100 <= checked, "the player fits along the tunnels: blocked at {} of {checked}: {:?}", blocked.len(), &blocked[..blocked.len().min(8)]);
    }

    /// Each underground biome is there and dressed: fungal soil and sprouts
    /// and giant mushrooms, crystals, toxic crust and acid.
    #[test]
    fn underground_biomes_are_dressed() {
        use std::collections::HashMap;
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let mut seen: HashMap<caves::Zone, Vec<MaterialId>> = HashMap::new();
        for a in &g.plan.caves.areas {
            let mut found = Vec::new();
            for k in 0..60 {
                let (x, y) = (a.x + a.rx * 0.8 * ((k * 37 % 60) as f32 / 30.0 - 1.0), a.y + a.ry * 0.8 * ((k * 17 % 60) as f32 / 30.0 - 1.0));
                let pos = CellPos::new(x as i32, y as i32).chunk();
                let (c, _) = g.generate_with_spawns(pos);
                found.extend(c.cells().iter().map(|c| c.material));
                found.extend(c.background().iter().map(|c| c.material));
            }
            found.sort();
            found.dedup();
            seen.entry(a.zone).or_default().extend(found);
        }
        let has = |z: caves::Zone, name: &str| seen.get(&z).is_some_and(|v| v.contains(&m.expect_id(name)));
        for (zone, names) in [
            (caves::Zone::Fungal, ["fungal_soil", "glowcap", "mushroom_cap", "mushroom_gills", "mushroom_glow", "fungus_shelf", "glow_vine"].as_slice()),
            (caves::Zone::Crystal, ["crystal"].as_slice()),
            (caves::Zone::Toxic, ["toxic_crust", "acid"].as_slice()),
        ] {
            for n in names {
                assert!(has(zone, n), "{} have {n}", zone.name());
            }
        }
    }

    /// No rock floats in the caves: in windows of generated chunks, solid
    /// pieces that touch nothing (not the window's edge, not another piece)
    /// are counted; there are next to none.
    #[test]
    fn no_rock_floats_in_the_caves() {
        use platypus_sim::Kind;
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let solid = |c: Cell| matches!(m.phys(c.material).kind, Kind::Static | Kind::Powder);
        let mut floating = Vec::new();
        let areas: Vec<(i32, i32)> = g.plan.caves.areas.iter().map(|a| (a.x as i32, a.y as i32)).chain([(16_000, 11_000), (16_000, 7_600), (12_000, 3_500)]).collect();
        for (x, y) in areas {
            let c0 = CellPos::new(x, y).chunk();
            const R: i32 = 3;
            let w = (2 * R + 1) * CHUNK;
            let mut grid = vec![false; (w * w) as usize];
            for cy in -R..=R {
                for cx in -R..=R {
                    let ch = g.generate(ChunkPos::new(c0.x + cx, c0.y + cy));
                    for ly in 0..CHUNK {
                        for lx in 0..CHUNK {
                            let (gx, gy) = ((cx + R) * CHUNK + lx, (cy + R) * CHUNK + ly);
                            grid[(gy * w + gx) as usize] = solid(ch.get(lx as usize, ly as usize));
                        }
                    }
                }
            }
            let mut seen = vec![false; grid.len()];
            for start in 0..grid.len() {
                if !grid[start] || seen[start] {
                    continue;
                }
                let (mut stack, mut size, mut edge) = (vec![start], 0, false);
                seen[start] = true;
                while let Some(i) = stack.pop() {
                    size += 1;
                    let (px, py) = (i as i32 % w, i as i32 / w);
                    edge |= px == 0 || py == 0 || px == w - 1 || py == w - 1;
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (qx, qy) = (px + dx, py + dy);
                        if qx >= 0 && qy >= 0 && qx < w && qy < w {
                            let q = (qy * w + qx) as usize;
                            if grid[q] && !seen[q] {
                                seen[q] = true;
                                stack.push(q);
                            }
                        }
                    }
                }
                if !edge {
                    let (px, py) = (start as i32 % w, start as i32 / w);
                    floating.push(((c0.x - R) * CHUNK + px, (c0.y - R) * CHUNK + py, size));
                    if (6..2000).contains(&size) && std::env::var("DUMP").is_ok() {
                        let mut out = String::new();
                        for qy in (py - 5..py + 75).rev() {
                            for qx in px - 30..px + 30 {
                                let q = (qy * w + qx) as usize;
                                out.push(if qx < 0 || qy < 0 || qx >= w || qy >= w { ' ' } else if qx == px && qy == py { '@' } else if grid[q] { '#' } else { '.' });
                            }
                            out.push('\n');
                        }
                        eprintln!("piece at {:?}:\n{out}", ((c0.x - R) * CHUNK + px, (c0.y - R) * CHUNK + py));
                    }
                }
            }
        }
        // Small pieces (specks, cut-off stalactites, crystals whose wall is
        // gone): none. Big masses of rock between caves (thousands of cells)
        // can stand free, like boulders in a cavern.
        let small: Vec<_> = floating.iter().filter(|f| f.2 < 1_000).collect();
        assert!(small.len() <= 2, "{} small pieces of rock floating: {:?}", small.len(), &small[..small.len().min(10)]);
    }

    /// A giant mushroom stands on the floor, and cut through its stem it
    /// comes down like a tree (a body), leaving nothing hanging.
    #[test]
    fn a_cut_mushroom_falls_like_a_tree() {
        use platypus_sim::{World, WorldEdit};
        use std::sync::Arc;
        let m = Arc::new(mats());
        let g = TerrainGen::new(1, Preset::Large, &m);
        let parasols: Vec<caves::Mushroom> = g
            .plan
            .caves
            .chambers
            .iter()
            .flat_map(|c| c.mushrooms.iter().copied())
            .filter(|s| s.species == caves::Species::Parasol && s.top - s.foot > 50.0 && s.lean.abs() < 6.0 && g.rooted((s.x as i32, s.foot as i32 + 9)))
            // (Alone: no other mushroom within reach of its stem.)
            .filter(|s| g.plan.caves.chambers.iter().flat_map(|c| &c.mushrooms).filter(|o| (o.x - s.x).abs() < 40.0 && (o.foot - s.foot).abs() < 80.0).count() == 1)
            .take(3)
            .collect();
        assert!(!parasols.is_empty(), "a parasol to fell");
        let stem = m.expect_id("mushroom_stem");
        for s in parasols {
            let (x, floor) = (s.x as i32, s.foot as i32 + 10);
            // It stands on its floor: the cell under its foot of stem is rock.
            assert!(g.material_at(x, floor - 1) != MaterialId::AIR, "the mushroom at {x} stands on the floor");
            let mut w = World::new(1, m.clone());
            w.set_climate(g.climate());
            let (cx, cy) = (x.div_euclid(CHUNK), floor.div_euclid(CHUNK));
            for dy in -2..=3 {
                for dx in -3..=3 {
                    w.insert_chunk(g.generate(ChunkPos::new(cx + dx, cy + dy)));
                }
            }
            assert_eq!(w.get_bg(CellPos::new(x, floor + 12)).map(|c| c.material), Some(stem), "its stem at {x}");
            w.apply_edit(&WorldEdit::Dig { center: CellPos::new(x, floor + 12), radius: s.stem as i32 + 4, max_hardness: 200 });
            if w.bodies().is_empty() {
                for y in (floor + 12..s.top as i32 + 4).step_by(3) {
                    let row: String = (x - 8..=x + 8)
                        .map(|xx| {
                            let p = CellPos::new(xx, y);
                            let f = w.get(p).map(|c| c.material).unwrap_or(MaterialId::AIR);
                            let b = w.get_bg(p).map(|c| c.material).unwrap_or(MaterialId::AIR);
                            if f != MaterialId::AIR { 'F' } else if b == stem { 'S' } else if b == MaterialId::AIR { '.' } else if m.phys(b).kind == platypus_sim::Kind::Plant { 'p' } else { 'w' }
                        })
                        .collect();
                    eprintln!("{y}: {row}");
                }
            }
            assert!(!w.bodies().is_empty(), "the mushroom at {x} came down");
        }
    }

    /// Water runs over each column as (start, end, level), left to right.
    fn lakes(p: &WorldPlan) -> Vec<(i32, i32, i32)> {
        let mut out = Vec::new();
        let mut x = 0;
        while x < p.width {
            match p.water_at(x) {
                Some(level) => {
                    let start = x;
                    while x < p.width && p.water_at(x) == Some(level) {
                        x += 1;
                    }
                    out.push((start, x, level));
                }
                None => x += 1,
            }
        }
        out
    }

    #[test]
    fn oceans_at_both_ends_and_held_lakes_inland() {
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let p = g.plan();
        assert_eq!(p.water_at(10), Some(p.sea_level), "an ocean on the left");
        assert_eq!(p.water_at(p.width - 10), Some(p.sea_level), "and on the right");
        let inland: Vec<_> = lakes(p).into_iter().filter(|&(a, b, _)| p.biome_at(a) != Biome::Ocean && p.biome_at(b - 1) != Biome::Ocean).collect();
        assert!(inland.len() >= 3, "lakes inland ({})", inland.len());
        assert!(inland.iter().any(|&(a, b, _)| b - a > 600), "at least one big one: {inland:?}");
        for (a, b, level) in inland {
            // Held: the ground either side reaches the water line, so it's
            // asleep on load, not pouring away.
            assert!(p.surface_at(a - 1) >= level && p.surface_at(b) >= level, "lake {a}..{b} at {level} spills");
            // Frozen exactly where it's freezing.
            let x = (a + b) / 2;
            let expect = if p.climate.ambient(x, level - 1) <= 0 { m.expect_id("ice") } else { m.expect_id("water") };
            assert_eq!(g.material_at(x, level - 1), expect, "lake at {x}");
        }
    }

    #[test]
    fn high_peaks_are_snowy() {
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let p = g.plan();
        let (peaks_floor, _) = p.band_span(Band::Peaks);
        assert!((0..p.width).any(|x| p.surface_at(x) > peaks_floor + 600), "mountains rise well into the peaks band");
        // Where it's well below freezing and not steep, the ground is snow.
        let cold: Vec<i32> = (0..p.width)
            .filter(|&x| p.climate.ambient(x, p.surface_at(x)) <= -5 && (p.surface_at(x + 4) - p.surface_at(x - 4)).abs() <= 4)
            .filter(|&x| p.water_at(x).is_none()) // (a frozen lake's bed is sand)
            .filter(|&x| !p.chasm_at(x, p.surface_at(x) - 1)) // (a chasm's mouth is air)
            .filter(|&x| !p.structures.near_column(x, 250)) // (a castle's roof, its stair)
            .collect();
        assert!(cold.len() > 100, "some cold, gentle ground ({})", cold.len());
        let snow = m.expect_id("snow");
        let snowy = cold.iter().filter(|&&x| g.material_at(x, p.surface_at(x) - 1) == snow).count();
        assert!(snowy * 10 >= cold.len() * 9, "{snowy} of {} cold, gentle columns are snow", cold.len());
    }

    #[test]
    fn sky_islands_float_green_with_trees() {
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let p = g.plan();
        assert!(p.islands.len() >= 6, "{} islands", p.islands.len());
        let (sky_floor, _) = p.band_span(Band::Sky);
        let grass = m.expect_id("grass");
        for i in &p.islands {
            assert!(i.y0 > sky_floor, "island at {} floats in the sky band", i.x0);
            let green = (i.x0..i.x0 + i.w).filter(|&x| i.top_at(x).is_some_and(|t| g.material_at(x, t - 1) == grass)).count();
            assert!(green as i32 > i.w / 2, "island at {} is grassy ({green} of {})", i.x0, i.w);
        }
        assert!(p.island_forest.len() >= p.islands.len(), "trees on them ({})", p.island_forest.len());
    }

    #[test]
    fn spawn_is_dry_forest_ground() {
        let m = mats();
        for preset in [Preset::Small, Preset::Large] {
            let g = TerrainGen::new(1, preset, &m);
            let s = g.spawn_point();
            assert_eq!(g.plan().biome_at(s.x), Biome::Forest);
            assert!(g.plan().water_at(s.x).is_none());
            assert_eq!(g.material_at(s.x, s.y), MaterialId::AIR);
            assert_ne!(g.material_at(s.x, s.y - 3), MaterialId::AIR, "standing on ground");
        }
    }

    #[test]
    fn chasms_run_from_the_surface_into_the_deep() {
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let p = g.plan();
        assert!(p.chasms.len() >= 3, "{} chasms", p.chasms.len());
        let (deep_lo, deep_hi) = p.band_span(Band::Deep);
        for c in &p.chasms {
            assert!(c.bottom > deep_lo && c.bottom < deep_hi, "chasm at {} ends in the deep", c.x);
            // Open down its whole length: its centre is air every 50 cells.
            let blocked: Vec<i32> = (c.bottom + 400..c.top - 10).step_by(50).filter(|&y| g.material_at(c.at(y).0 as i32, y) != MaterialId::AIR).collect();
            assert!(blocked.is_empty(), "chasm at {} blocked at {:?}", c.x, &blocked[..blocked.len().min(5)]);
        }
    }

    #[test]
    fn each_band_has_its_rock_and_the_underworld_a_lava_sea() {
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let p = g.plan();
        let share = |band: Band, id: MaterialId| {
            let (lo, hi) = p.band_span(band);
            let cells: Vec<MaterialId> = (0..400).map(|k| g.material_at(2_000 + k * 71, lo + (k * 37) % (hi - lo))).collect();
            cells.iter().filter(|&&c| c == id).count() as f32 / cells.len() as f32
        };
        assert!(share(Band::Underground, m.expect_id("stone")) > 0.4);
        assert!(share(Band::Deep, m.expect_id("slate")) > 0.4);
        let (lo, hi) = p.band_span(Band::Underworld);
        // The sea is flat (asleep on load) and a vault opens over it.
        let level = (lo as f64 + (hi - lo) as f64 * 0.2).floor() as i32;
        let lava = m.expect_id("lava");
        let seas = (0..200).map(|k| 1_000 + k * 150).filter(|&x| g.material_at(x, level - 1) == lava).count();
        assert!(seas > 120, "lava sea under most of the world ({seas} of 200)");
        assert!((0..200).map(|k| 1_000 + k * 150).all(|x| g.material_at(x, level) != lava), "flat at {level}");
        let open = (0..200).map(|k| 1_000 + k * 150).filter(|&x| g.material_at(x, level + 80) == MaterialId::AIR).count();
        assert!(open > 120, "a vault over it ({open} of 200)");
    }

    #[test]
    fn some_cavern_chambers_are_flooded() {
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let p = g.plan();
        let (lo, hi) = p.band_span(Band::Caverns);
        let water = m.expect_id("water");
        let (mut wet, mut open) = (0, 0);
        for x in (1_700..31_000).step_by(97) {
            for y in (lo..hi).step_by(53) {
                match g.material_at(x, y) {
                    id if id == water => wet += 1,
                    MaterialId::AIR => open += 1,
                    _ => {}
                }
            }
        }
        assert!(open > 1_500 && wet > 200, "caverns: {open} open, {wet} flooded samples");
    }

    #[test]
    fn chests_sit_on_cave_floors_with_room() {
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let (lo, _) = g.plan().band_span(Band::Deep);
        let (_, hi) = g.plan().band_span(Band::Underground);
        let (w, h) = CHEST_SIZE;
        let (mut chunks, mut chests) = (0, 0);
        for cx in (30..480).step_by(7) {
            for cy in (lo / CHUNK..hi / CHUNK).step_by(9) {
                let (c, spawns) = g.generate_with_spawns(ChunkPos::new(cx, cy));
                chunks += 1;
                let o = c.pos.origin();
                let at = |x: i32, y: i32| {
                    let (lx, ly) = (x - o.x, y - o.y);
                    if (0..CHUNK).contains(&lx) && (0..CHUNK).contains(&ly) { c.get(lx as usize, ly as usize).material } else { MaterialId::AIR }
                };
                for (feet, what) in spawns {
                    if what != Spawn::Chest || g.plan.structures.glyph_at(feet.x, feet.y).is_some() {
                        continue; // (a structure's)
                    }
                    chests += 1;
                    let x0 = feet.x - w / 2;
                    assert!((0..h).all(|dy| (0..w).all(|dx| at(x0 + dx, feet.y + dy) == MaterialId::AIR)), "room for a chest at {feet:?}");
                    assert!((0..w).filter(|&dx| at(x0 + dx, feet.y - 1) != MaterialId::AIR).count() >= (w * 3 / 4) as usize, "on a floor at {feet:?}");
                }
            }
        }
        // Roughly one in fifty underground chunks.
        println!("{chests} chests in {chunks} chunks");
        assert!(chests * 150 > chunks && chests * 20 < chunks, "{chests} chests in {chunks} chunks");
    }


    #[test]
    fn the_tundra_is_a_snowy_pine_forest() {
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let p = g.plan();
        let (x0, x1, _) = p.regions().into_iter().find(|r| r.2 == Biome::Tundra).expect("a tundra");
        let trees = p.forest.near(x0 + 300, x1 - 300);
        let pines = trees.iter().filter(|t| t.species == flora::Species::Conifer && t.snowy).count();
        assert!(pines * 1000 >= (x1 - x0 - 600) as usize * 8, "a forest: {pines} snowy pines over {} cells", x1 - x0);
        assert!(trees.iter().all(|t| t.species == flora::Species::Conifer), "no broadleaves in the cold");
    }

    #[test]
    fn the_deep_forest_is_wide_and_grows_giants_at_its_heart() {
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let p = g.plan();
        let deep: Vec<_> = p.regions().into_iter().filter(|r| r.2 == Biome::DeepForest).collect();
        let (x0, x1) = (deep.first().expect("a deep forest").0, deep.last().unwrap().1);
        assert!(x1 - x0 >= 5_000, "wide enough to get lost in ({} cells)", x1 - x0);
        let (w, mid) = ((x1 - x0) / 5, (x0 + x1) / 2);
        let heart = p.forest.near(mid - w / 2, mid + w / 2);
        let edge = p.forest.near(x0, x0 + w);
        let avg = |ts: &[&flora::Tree]| ts.iter().map(|t| t.height as f32).sum::<f32>() / ts.len().max(1) as f32;
        assert!(heart.iter().filter(|t| t.species == flora::Species::Elder).count() * 2 > heart.len(), "elders at the heart");
        assert!(avg(&heart) > avg(&edge) * 1.2, "bigger toward the heart: {:.0} vs {:.0}", avg(&heart), avg(&edge));
    }

    #[test]
    fn the_mountain_range_has_several_snowy_peaks() {
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let p = g.plan();
        let (x0, x1, _) = p.regions().into_iter().find(|r| r.2 == Biome::Mountains).expect("a range");
        // Peaks: local maxima over ±300 cells, 1 200+ above sea level.
        let peaks: Vec<i32> = (x0..x1)
            .step_by(20)
            .filter(|&x| p.surface_at(x) > p.sea_level + 1_200 && (x - 300..=x + 300).step_by(20).all(|q| p.surface_at(q) <= p.surface_at(x)))
            .collect();
        assert!(peaks.len() >= 4, "{} peaks: {peaks:?}", peaks.len());
        let snow = m.expect_id("snow");
        // (A castle on a summit isn't snow.)
        for &x in peaks.iter().filter(|&&x| !p.structures.near_column(x, 250)) {
            let top = (p.surface_at(x) - 60..p.surface_at(x) + 60).rev().find(|&y| g.material_at(x, y) != MaterialId::AIR).unwrap();
            let white = (-40..=40).filter(|&d| g.material_at(x + d, (top - 400..top + 80).rev().find(|&y| g.material_at(x + d, y) != MaterialId::AIR).unwrap_or(top)) == snow).count();
            assert!(white > 40, "peak at {x} is snowy ({white} of 81 columns)");
        }
    }

    #[test]
    fn has_sky_ground_and_bedrock() {
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        // On gentle ground (the spawn's), not a mountain face.
        let x = g.spawn_point().x;
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
            // The tree and every tree overlapping it: a neighbour's crown
            // reaching in is held by its own wood, which may be outside t's box.
            let trees = g.plan.forest.near(t.bbox.0 - 2, t.bbox.2 + 2);
            let (x0, y0, x1, y1) = trees.iter().fold(t.bbox, |b, n| (b.0.min(n.bbox.0), b.1.min(n.bbox.1), b.2.max(n.bbox.2), b.3.max(n.bbox.3)));
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
            // to anything resting on solid playfield (or running out of the
            // loaded region: a neighbour's trunk may root below it).
            let hanging = |w: &World| -> Vec<CellPos> {
                let bg = |p: CellPos| w.get_bg(p).is_some_and(|b| !b.is_air());
                let anchor = |p: CellPos| w.get(p).is_some_and(|f| !f.is_air() && matches!(m.phys(f.material).kind, Kind::Static | Kind::Powder));
                let mut held = std::collections::HashSet::new();
                let mut stack: Vec<CellPos> = (x0..x1).flat_map(|x| (y0..y1).map(move |y| CellPos::new(x, y))).filter(|&p| bg(p) && (anchor(p) || p.x == x0 || p.x == x1 - 1 || p.y == y0 || p.y == y1 - 1)).collect();
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




    /// Lairs take some chambers (the same ones every time), line them, and
    /// report their keepers once, from the chunk of the chamber's middle.
    #[test]
    fn lairs_take_chambers_line_them_and_keep_them() {
        let m = mats();
        let defs = vec![lairs::LairDef {
            name: "spider nest".into(),
            depth: (80.0, 100_000.0),
            zones: vec![],
            chance: 0.2,
            lining: "cobweb".into(),
            density: 0.6,
            keepers: vec![("spider".into(), 1), ("egg_sac".into(), 2)],
            min_size: 14.0,
        }];
        let g = TerrainGen::new(7, Preset::Small, &m).with_lairs(&defs, &m);
        let again = TerrainGen::new(7, Preset::Small, &m).with_lairs(&defs, &m);
        assert_eq!(g.lair_of, again.lair_of, "the same chambers every time");
        let taken: Vec<usize> = g.lair_of.iter().enumerate().filter(|(_, l)| l.is_some()).map(|(i, _)| i).collect();
        assert!(taken.len() >= 3, "some chambers are lairs: {}", taken.len());
        let c = &g.plan.caves.chambers[taken[0]];
        let mid = CellPos::new(c.x as i32, c.y as i32).chunk();
        let (chunk, spawns) = g.generate_with_spawns(mid);
        let kinds: Vec<&str> = spawns.iter().filter_map(|(_, s)| if let Spawn::Creature(k) = s { Some(*k) } else { None }).collect();
        assert!(kinds.contains(&"spider") && kinds.iter().filter(|k| **k == "egg_sac").count() == 2, "its keepers: {kinds:?}");
        let web = m.expect_id("cobweb");
        let near: i32 = (-1..=1).flat_map(|dy| (-1..=1).map(move |dx| (dx, dy))).map(|(dx, dy)| {
            let (ch, _) = g.generate_with_spawns(ChunkPos::new(mid.x + dx, mid.y + dy));
            let _ = &chunk;
            (0..CHUNK).flat_map(|y| (0..CHUNK).map(move |x| (x, y))).filter(|&(x, y)| ch.get(x as usize, y as usize).material == web).count() as i32
        }).sum();
        assert!(near > 20, "webs line it: {near} cells");
    }
}
