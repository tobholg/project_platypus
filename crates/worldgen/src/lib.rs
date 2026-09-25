//! Seeded world generation. A chunk is a pure function of `(seed, ChunkPos)`,
//! so any chunk can be generated at any time, on any thread, by any co-op peer,
//! and come out identical.
//!
//! Two levels (SPEC §3.5): a small global `WorldPlan` computed once (surface
//! heights, later biomes and cave paths), and per-chunk rasterisation from it.

use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use platypus_sim::rng::{Rng, hash};
use platypus_sim::{CHUNK, CHUNK_AREA, Cell, CellPos, Chunk, ChunkPos, Climate, MaterialId, MaterialTable};

pub mod biome;
pub mod flora;
pub mod islands;
pub mod minerals;
pub mod plan;
pub mod structures;

use std::sync::Arc;

use flora::{Foliage, TreePart};
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

    /// Creatures a freshly generated chunk starts with (feet, creature id):
    /// the guards of a crypt. Each is reported by exactly one chunk; the
    /// game spawns each once.
    fn spawns(&self, _pos: ChunkPos) -> Vec<(CellPos, &'static str)> {
        Vec::new()
    }

    fn in_bounds(&self, pos: ChunkPos) -> bool {
        let (lo, hi) = self.bounds();
        pos.x >= lo.x && pos.y >= lo.y && pos.x <= hi.x && pos.y <= hi.y
    }
}

/// Noise-cave threshold near the surface and underground: higher, fewer
/// caves (0.18 was cheese).
const SMALL_CAVES: f64 = 0.26;
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
    /// Chests (if the content has them).
    chest: Option<MaterialId>,
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
    caves: Fbm<Perlin>,
    worms: Fbm<Perlin>,
    pockets: Perlin,
    strata: Perlin,
    /// Starting heat per material id (lava is born hot).
    heat: Vec<i16>,
    /// Ragged edges of tree crowns; tall grass height.
    leaf_edge: Perlin,
    meadow: Perlin,
    overhang: Fbm<Perlin>,
    /// Big tunnels through the underground and caverns.
    tunnels: Fbm<Perlin>,
    /// Cavern chambers; stalactites and pillars; the underworld's vault.
    caverns: Fbm<Perlin>,
    drips: Perlin,
    vault: Fbm<Perlin>,
    /// The chest's pattern over a chunk-sized tile (patterns divide 64).
    chest_shades: Vec<u8>,
    /// Ores and gems, and the noise they lie in.
    ores: Vec<minerals::Ore>,
    gems: Vec<minerals::Gem>,
    minerals: Perlin,
    /// Materials laid in a pattern (crypt stone, planks): their shades over
    /// a chunk-sized tile, by material id.
    patterns: Vec<Option<Vec<u8>>>,
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
            chest: mats.id("chest"),
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
        };

        let (ores, gems) = minerals::rules(&plan, mats);
        TerrainGen {
            plan,
            ids,
            ores,
            gems,
            minerals: Perlin::new(s(18)),
            patterns: mats
                .iter()
                .map(|(id, _)| mats.pattern_shade(id, 0, 0).map(|_| (0..CHUNK * CHUNK).map(|i| mats.pattern_shade(id, i % CHUNK, i / CHUNK).unwrap_or(136)).collect()))
                .collect(),
            leaf_edge: Perlin::new(s(7)),
            meadow: Perlin::new(s(8)),
            overhang: Fbm::<Perlin>::new(s(9)).set_octaves(3).set_frequency(1.0 / 60.0),
            tunnels: Fbm::<Perlin>::new(s(14)).set_octaves(2).set_frequency(1.0 / 1_100.0),
            caverns: Fbm::<Perlin>::new(s(15)).set_octaves(3).set_frequency(1.0 / 600.0),
            drips: Perlin::new(s(16)),
            vault: Fbm::<Perlin>::new(s(17)).set_octaves(3).set_frequency(1.0 / 400.0),
            caves: Fbm::<Perlin>::new(s(3)).set_octaves(4).set_frequency(1.0 / 160.0),
            worms: Fbm::<Perlin>::new(s(4)).set_octaves(3).set_frequency(1.0 / 260.0),
            pockets: Perlin::new(s(5)),
            strata: Perlin::new(s(6)),
            heat: mats.iter().map(|(id, _)| mats.phys(id).heat).collect(),
            chest_shades: (0..CHUNK * CHUNK)
                .map(|i| mats.id("chest").and_then(|c| mats.pattern_shade(c, i % CHUNK, i / CHUNK)).unwrap_or(136))
                .collect(),
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
            m != self.ids.air && m != self.ids.water && m != self.ids.lava && m != self.ids.oil
        };
        (0..3_000).step_by(8).flat_map(|d| [x + d, x - d]).find_map(|x| {
            (y - 800..=y).rev().find(|&y| solid(x, y - 1) && (0..20).all(|dy| air(x, y + dy) && air(x + 7, y + dy)) && solid(x + 7, y - 1)).map(|y| CellPos::new(x + 4, y + 2))
        })
    }

    /// What a structure's glyph makes at a cell. Candles and spikes are
    /// shapes inside their block; chests are drawn afterwards
    /// (`place_structure_chests`), whole.
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
            Glyph::Rubble => i.gravel,
            Glyph::Water => i.water,
            Glyph::Lava => i.lava,
            Glyph::Candle if bx == 1 && by <= 2 => i.candle,
            // Two spikes a block, pointing up.
            Glyph::Spikes if by == 0 || (by == 1 && bx != 3) || (by == 2 && bx == 1) => i.spikes,
            _ => i.air,
        }
    }

    /// Chests in structures, whole even where they cross into this chunk
    /// from the next.
    fn place_structure_chests(&self, pos: ChunkPos, cells: &mut [Cell]) {
        const SIDE: i32 = 8;
        let Some(chest) = self.ids.chest else { return };
        let o = pos.origin();
        // (A chest reaches 8 cells right and up from its corner, so the
        // chunks left and below may hold its corner.)
        for (cy, cx) in [(pos.y, pos.x), (pos.y, pos.x - 1), (pos.y - 1, pos.x), (pos.y - 1, pos.x - 1)] {
            for (_, piece) in self.plan.structures.pieces_in(cx, cy) {
                for (x0, y0) in piece.blocks_of(Glyph::Chest) {
                    for dy in 0..SIDE {
                        for dx in 0..SIDE {
                            let (lx, ly) = (x0 + dx - o.x, y0 + dy - o.y);
                            if (0..CHUNK).contains(&lx) && (0..CHUNK).contains(&ly) {
                                let shade = self.chest_shades[(dy * CHUNK + dx) as usize];
                                cells[(ly * CHUNK + lx) as usize] = Cell { heat: self.heat[chest.0 as usize], ..Cell::new(chest, shade) };
                            }
                        }
                    }
                }
            }
        }
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

    /// Chests in cave pockets (DESIGN §3.2): some chunks (more the deeper) get
    /// one on the first cave floor found: an 8 × 8 box of air on solid ground,
    /// inside the chunk (so it stays chunk-pure).
    fn place_chests(&self, pos: ChunkPos, cells: &mut [Cell], rng: &mut Rng) {
        const SIDE: i32 = 8;
        let Some(chest) = self.ids.chest else { return };
        let origin = pos.origin();
        let chance = match self.plan.band_at(origin.y + CHUNK / 2) {
            Band::Underground => 24,
            Band::Caverns => 36,
            Band::Deep => 48,
            _ => return,
        };
        if !rng.chance(chance) {
            return;
        }
        // Every spot (columns 4 apart, any height), from a random start: the
        // first cave floor.
        let spots: Vec<(i32, i32)> = (0..=(CHUNK - SIDE) / 4).flat_map(|i| (1..=CHUNK - SIDE).map(move |ly| (i * 4, ly))).collect();
        let start = rng.next_u32() as usize % spots.len();
        let at = |lx: i32, ly: i32| cells[(ly * CHUNK + lx) as usize];
        let found = (0..spots.len()).map(|k| spots[(start + k) % spots.len()]).find(|&(lx, ly)| {
            let open = (0..SIDE).all(|dy| (0..SIDE).all(|dx| at(lx + dx, ly + dy).is_air()));
            // Standing on rock, not hanging over a drop.
            let floor = (0..SIDE).all(|dx| {
                let c = at(lx + dx, ly - 1);
                !c.is_air() && c.material != self.ids.water && c.material != self.ids.lava
            });
            open && floor
        });
        if let Some((lx, ly)) = found {
            for dy in 0..SIDE {
                for dx in 0..SIDE {
                    // The picture is anchored to the chest's own corner.
                    let shade = self.chest_shades[(dy * CHUNK + dx) as usize];
                    cells[((ly + dy) * CHUNK + lx + dx) as usize] = Cell { heat: self.heat[chest.0 as usize], ..Cell::new(chest, shade) };
                }
            }
        }
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

    /// Open space underground, and what fills it: worm tunnels everywhere,
    /// big tunnels through the underground and caverns, small caves near the
    /// top, huge chambers (with stalactites and pillars, flooded below the
    /// water table) in the caverns and the deep; pools in the small ones.
    fn cavity(&self, x: i32, y: i32, depth: i32, band: Band) -> Option<MaterialId> {
        let i = &self.ids;
        let plan = &*self.plan;
        let (xf, yf) = (x as f64, y as f64);
        let under_water = plan.water_at(x).is_some();
        // Fading in below the topsoil, kept away from lake and ocean beds.
        let fade = ((depth as f64 - if under_water { 60.0 } else { 12.0 }) / 60.0).clamp(0.0, 1.0);
        if fade == 0.0 {
            return None;
        }
        // Fewer caves up in the mountains than under the lowlands.
        let above = ((y - plan.sea_level) as f64 / (600.0 * plan.height as f64 / 16_384.0)).clamp(0.0, 1.0);
        let worm = self.worms.get([xf, yf]).abs() + above * 0.03 < 0.035 * fade && depth > 20;
        let tunnel = matches!(band, Band::Underground | Band::Caverns) && self.tunnels.get([xf, yf * 1.2]).abs() < 0.016 * fade;
        let small = matches!(band, Band::Peaks | Band::Surface | Band::Underground) && self.caves.get([xf, yf * 1.6]) * fade - above * 0.14 > SMALL_CAVES;
        if matches!(band, Band::Caverns | Band::Deep) {
            // Chambers, wider than tall; streaks of noise hang stalactites
            // from their roofs and stand pillars in them.
            // (Fading in over the band's top 300 cells.)
            let entry = ((plan.band_span(Band::Caverns).1 - y) as f64 / 300.0).clamp(0.0, 1.0);
            let drip = self.drips.get([xf / 7.0, yf / 60.0, 0.3]).abs();
            let chamber = self.caverns.get([xf / 1.6, yf]) * (if band == Band::Caverns { entry } else { 1.0 }) - drip * 0.12 > 0.22;
            if chamber {
                return Some(if y < plan.water_table(x) && band == Band::Caverns { i.water } else { i.air });
            }
        }
        if !(worm || tunnel || small) {
            return None;
        }
        // Fill the bottoms of some small caves with a pool (never up high).
        let pool = self.pockets.get([xf / 90.0, yf / 90.0, 1.3]);
        if above == 0.0 && small && pool > 0.35 && self.caves.get([xf, (yf - 10.0) * 1.6]) * fade <= SMALL_CAVES {
            return Some(if band == Band::Deep { i.lava } else if pool > 0.62 { i.oil } else { i.water });
        }
        Some(i.air)
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

    fn surface_hint(&self, x: i32) -> Option<i32> {
        Some(self.surface_at(x))
    }

    fn spawns(&self, pos: ChunkPos) -> Vec<(CellPos, &'static str)> {
        let o = pos.origin();
        let inside = |(x, y): &(i32, i32)| (o.x..o.x + CHUNK).contains(x) && (o.y..o.y + CHUNK).contains(y);
        let mut out = Vec::new();
        for (_, piece) in self.plan.structures.pieces_in(pos.x, pos.y) {
            for (x, y) in piece.blocks_of(Glyph::Spawn).filter(inside) {
                out.push((CellPos::new(x + 2, y), "orc"));
            }
            // A boss is a pack, until there are bosses.
            for (x, y) in piece.blocks_of(Glyph::Boss).filter(inside) {
                for dx in [-10, 2, 14] {
                    out.push((CellPos::new(x + dx, y), "orc"));
                }
            }
        }
        out
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
        self.place_chests(pos, &mut cells, &mut rng);
        self.place_structure_chests(pos, &mut cells);
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

    /// Crypts stand on level ground with no tree in their ruin; chunks draw
    /// their chests whole and report each guard exactly once.
    #[test]
    fn crypts_are_sited_and_rasterised_whole() {
        use std::collections::HashSet;
        let m = mats();
        let g = TerrainGen::new(1, Preset::Small, &m);
        let crypts: Vec<_> = g.plan.structures.list.iter().filter(|s| s.kind == structures::StructureKind::Crypt).collect();
        assert!(crypts.len() >= 2, "a small world has crypts ({})", crypts.len());
        let chest = m.expect_id("chest");
        for c in crypts {
            let ruin = &c.pieces[0];
            let (x0, _, x1, _) = ruin.bbox();
            assert!((ruin.y + 4 - g.surface_at(c.site.0)).abs() <= 4, "the ruin's floor is the ground");
            // (A neighbour's crown may reach over it.)
            assert!(g.plan.forest.near(x0, x1).iter().all(|t| t.x < x0 - 8 || t.x > x1 + 8), "no tree grows in the ruin at {}", c.site.0);
            let (bx0, by0, bx1, by1) = c.bbox();
            let (lo, hi) = (CellPos::new(bx0, by0).chunk(), CellPos::new(bx1, by1).chunk());
            let (mut chest_cells, mut spawns) = (0, Vec::new());
            for cy in lo.y..=hi.y {
                for cx in lo.x..=hi.x {
                    let pos = ChunkPos::new(cx, cy);
                    chest_cells += g.generate(pos).cells().iter().filter(|c| c.material == chest).count();
                    spawns.extend(g.spawns(pos).into_iter().map(|(p, _)| p));
                }
            }
            let chests: usize = c.pieces.iter().map(|p| p.blocks_of(Glyph::Chest).count()).sum();
            assert!(chest_cells >= chests * 64, "{chests} chests drawn whole ({chest_cells} cells)");
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
        let chest = m.expect_id("chest");
        let sunken: Vec<_> = g.plan.structures.list.iter().filter(|s| s.kind == structures::StructureKind::Sunken).collect();
        assert!(sunken.len() >= 3, "{} lake chests", sunken.len());
        for s in sunken {
            let (x, bed) = s.site;
            assert!(g.plan.water_at(x).is_some_and(|w| w > bed + 20), "deep water over the chest at {x}");
            let pos = CellPos::new(x, bed + 2);
            let (lx, ly) = pos.local();
            assert_eq!(g.generate(pos.chunk()).get(lx, ly).material, chest, "the chest is on the bed at {x}");
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
    fn chests_sit_whole_on_cave_floors() {
        let m = mats();
        let g = TerrainGen::new(1, Preset::Large, &m);
        let chest = m.expect_id("chest");
        let (lo, _) = g.plan().band_span(Band::Deep);
        let (_, hi) = g.plan().band_span(Band::Underground);
        let (mut chunks, mut chests) = (0, 0);
        for cx in (30..480).step_by(7) {
            for cy in (lo / CHUNK..hi / CHUNK).step_by(9) {
                let c = g.generate(ChunkPos::new(cx, cy));
                chunks += 1;
                let at = |lx: i32, ly: i32| if (0..CHUNK).contains(&lx) && (0..CHUNK).contains(&ly) { c.get(lx as usize, ly as usize).material } else { MaterialId::AIR };
                for ly in 0..CHUNK {
                    for lx in 0..CHUNK {
                        // A chest's corner: chest here, not left or below.
                        if at(lx, ly) != chest || at(lx - 1, ly) == chest || at(lx, ly - 1) == chest {
                            continue;
                        }
                        chests += 1;
                        assert!((0..8).all(|dy| (0..8).all(|dx| at(lx + dx, ly + dy) == chest)), "a whole chest at {lx},{ly} in {cx},{cy}");
                        assert!(ly > 0 && (0..8).all(|dx| at(lx + dx, ly - 1) != MaterialId::AIR), "on a floor");
                    }
                }
            }
        }
        // Roughly one in a few dozen underground chunks: ~1 000 in the world.
        println!("{chests} chests in {chunks} chunks");
        assert!(chests * 100 > chunks && chests * 8 < chunks, "{chests} chests in {chunks} chunks");
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



}
