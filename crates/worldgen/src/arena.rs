//! The arena (`PLATYPUS_WORLD=arena`): a small walled sandbox for trying
//! weapons, spells, creatures and animations. One long floor with a bit of
//! everything along it, left to right: stairs and a ramp, a water pool,
//! one-way platforms, the open floor with training dummies (the player
//! starts here), a lava pit, a sand heap, and two tall columns to wall-jump
//! between beside a block of planks to burn. High in the top right, sealed
//! in bedrock, the bestiary's stage (`STAGE`): a floor with a step, where
//! a creature is shown off live.
//!
//! That's the `Sandbox` layout. Others (`Layout`, `PLATYPUS_ARENA` or the
//! arena panel), each in the same walls with the same stage: `Flat` (one
//! long floor), `Cave` (a low tunnel, its roof and floor uneven, pillars),
//! `Slopes` (rolling hills), `Stairs` (flights up and down, ledges), and
//! `Real` (the real world's terrain, copied whole from a seed and a place:
//! "fight it here").

use platypus_sim::{CHUNK, CellPos, Cell, Chunk, ChunkPos, MaterialId, MaterialTable};

use crate::{ChunkGenerator, Preset, Spawn, TerrainGen};

/// What the arena's floor is.
#[derive(Clone, Debug, PartialEq)]
pub enum Layout {
    Sandbox,
    Flat,
    Cave,
    Slopes,
    Stairs,
    /// The real world (`seed`, the medium preset) around (x, y): its
    /// chunks, copied whole, that place at the arena's start.
    Real { seed: u64, x: i32, y: i32 },
}

impl Layout {
    /// Read from a name (`PLATYPUS_ARENA`): `flat`, `cave`, `slopes`,
    /// `stairs`, `real` (seed 1's start), `real:SEED:X:Y`; else the sandbox.
    pub fn parse(s: &str) -> Layout {
        let mut parts = s.split(':');
        match parts.next().unwrap_or("") {
            "flat" => Layout::Flat,
            "cave" => Layout::Cave,
            "slopes" => Layout::Slopes,
            "stairs" => Layout::Stairs,
            "real" => {
                let n: Vec<i64> = parts.filter_map(|p| p.parse().ok()).collect();
                match n[..] {
                    [seed, x, y] => Layout::Real { seed: seed as u64, x: x as i32, y: y as i32 },
                    [seed] => Layout::Real { seed: seed as u64, x: i32::MIN, y: i32::MIN },
                    _ => Layout::Real { seed: 1, x: i32::MIN, y: i32::MIN },
                }
            }
            _ => Layout::Sandbox,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Layout::Sandbox => "sandbox",
            Layout::Flat => "flat",
            Layout::Cave => "cave",
            Layout::Slopes => "slopes",
            Layout::Stairs => "stairs",
            Layout::Real { .. } => "real",
        }
    }
}

/// The real world under a `Real` layout, and how far its chunks are moved
/// (in chunks) to land in the arena.
struct RealCopy {
    terrain: TerrainGen,
    shift: ChunkPos,
    /// Where that place lands in the arena (the start).
    start: CellPos,
}

/// Size in chunks.
const W: i32 = 30;
const H: i32 = 15;
/// The floor's top: solid below it.
pub const FLOOR: i32 = 160;
/// Where the player starts, and the dummies stand (x, what).
const START: i32 = 960;
const DUMMIES: [(i32, &str); 4] = [(1050, "dummy"), (1140, "dummy"), (1230, "dummy"), (1305, "sandbag")];
/// The stage's inside (x0, floor, x1, ceiling), its bedrock this thick
/// round it; a step up this high at its right end, from `STEP_AT`.
pub const STAGE: (i32, i32, i32, i32) = (1530, 744, 1860, 900);
const STAGE_WALL: i32 = 12;
const STEP_AT: i32 = 1800;
const STEP: i32 = 9;

pub struct ArenaGen {
    pub layout: Layout,
    real: Option<RealCopy>,
    stone: MaterialId,
    bedrock: MaterialId,
    water: MaterialId,
    lava: MaterialId,
    sand: MaterialId,
    planks: MaterialId,
    platform: MaterialId,
}

impl ArenaGen {
    pub fn new(m: &MaterialTable) -> Self {
        Self::with_layout(m, Layout::Sandbox)
    }

    pub fn with_layout(m: &MaterialTable, layout: Layout) -> Self {
        // The real world's place at the arena's start: a whole number of
        // chunks away (they're copied whole), its ground near the floor.
        let real = match layout {
            Layout::Real { seed, x, y } => {
                let terrain = TerrainGen::new(seed, Preset::Medium, m);
                let x = if x == i32::MIN { terrain.spawn_point().x } else { x };
                let y = if y == i32::MIN { terrain.surface_at(x) } else { y };
                let shift = ChunkPos::new((x - START).div_euclid(CHUNK), (y - FLOOR).div_euclid(CHUNK));
                let o = shift.origin();
                Some(RealCopy { terrain, shift, start: CellPos::new(x - o.x, y - o.y) })
            }
            _ => None,
        };
        ArenaGen {
            layout,
            real,
            stone: m.expect_id("stone"),
            bedrock: m.expect_id("bedrock"),
            water: m.expect_id("water"),
            lava: m.expect_id("lava"),
            sand: m.expect_id("sand"),
            planks: m.expect_id("planks"),
            platform: m.expect_id("platform"),
        }
    }

    /// What every layout has: the bedrock round it, the stage, the side
    /// walls. `None`: up to the layout.
    fn frame(&self, x: i32, y: i32) -> Option<Option<MaterialId>> {
        let f = FLOOR;
        let right = W * CHUNK;
        if y < 12 || x < 12 || x >= right - 12 {
            return Some(Some(self.bedrock));
        }
        // The stage: stone underfoot (a step at the right), bedrock round.
        let (sx0, sf, sx1, sy1) = STAGE;
        if (sx0 - STAGE_WALL..sx1 + STAGE_WALL).contains(&x) && (sf - STAGE_WALL..sy1 + STAGE_WALL).contains(&y) {
            let within = (sx0..sx1).contains(&x);
            let ground = if x >= STEP_AT { sf + STEP } else { sf };
            return Some(match () {
                _ if within && (sf - 6..ground).contains(&y) => Some(self.stone),
                _ if within && (ground..sy1).contains(&y) => None,
                _ => Some(self.bedrock),
            });
        }
        // Side walls, higher than anything can be thrown.
        if (x < 36 || x >= right - 36) && y < f + 480 {
            return Some(Some(self.stone));
        }
        None
    }

    /// The ground's top at x, for the layouts drawn from a height (the
    /// cave's floor; not the sandbox's, nor the real world's).
    fn ground(&self, x: i32) -> i32 {
        let f = FLOOR;
        let xf = x as f32;
        match self.layout {
            Layout::Slopes => f + (40.0 * (xf / 150.0).sin() + 18.0 * (xf / 53.0 + 1.0).sin()).round() as i32 + 40,
            Layout::Cave => f + (5.0 * (xf / 37.0).sin() + 3.0 * (xf / 13.0).sin()).round() as i32,
            Layout::Stairs => {
                // Up from 300 to 780 (8-cell steps every 24), a landing,
                // down to 1500, then a high ledge at the right.
                let up = |from: i32, to: i32| ((x.clamp(from, to) - from) / 24) * 8;
                f + up(300, 780) - up(1020, 1500) + if (1620..1800).contains(&x) { 96 } else { 0 }
            }
            _ => f,
        }
    }

    /// What's at (x, y) (`None`: air).
    fn at(&self, x: i32, y: i32) -> Option<MaterialId> {
        if let Some(c) = self.frame(x, y) {
            return c;
        }
        match self.layout {
            Layout::Sandbox | Layout::Real { .. } => self.sandbox(x, y),
            Layout::Flat | Layout::Slopes => (y < self.ground(x)).then_some(self.stone),
            Layout::Stairs => {
                // (And platforms between the flights, to jump up on.)
                let p = [(840, 960, FLOOR + 230), (1100, 1180, FLOOR + 150), (1300, 1380, FLOOR + 90)];
                if p.iter().any(|&(x0, x1, py)| (x0..x1).contains(&x) && (py..py + 3).contains(&y)) {
                    return Some(self.platform);
                }
                (y < self.ground(x)).then_some(self.stone)
            }
            Layout::Cave => {
                // A tunnel: its roof 70 up and wavering, rock above to the
                // walls' top; a few pillars, roof to floor.
                let xf = x as f32;
                let roof = FLOOR + 70 + (10.0 * (xf / 61.0).cos() + 5.0 * (xf / 23.0).sin()).round() as i32;
                let pillar = [420, 1320, 1700].iter().any(|&px| (x - px).abs() < 9);
                (y < self.ground(x) || (y >= roof && y < FLOOR + 480) || (pillar && y < FLOOR + 480)).then_some(self.stone)
            }
        }
    }

    /// The sandbox: a bit of everything along one floor.
    fn sandbox(&self, x: i32, y: i32) -> Option<MaterialId> {
        let f = FLOOR;
        // Stairs up (7–8-cell steps every 24), a ledge, a ramp down.
        let ground = match x {
            60..240 => f + ((x - 60) / 24 + 1) * 15 / 2,
            240..300 => f + 60,
            300..420 => f + 60 - (x - 300) / 2,
            _ => f,
        };
        // Pits, sunk into the floor: (from, to, depth, what fills them, the
        // gap left above).
        let pits = [(465, 630, 75, self.water, 0), (1350, 1440, 36, self.lava, 6)];
        for (x0, x1, depth, fill, gap) in pits {
            if (x0..x1).contains(&x) && y >= f - depth && y < f {
                return (y < f - gap).then_some(fill);
            }
        }
        if y < ground {
            return Some(self.stone);
        }
        // Platforms to drop through and jump up on.
        let platforms = [(675, 765, f + 42), (705, 795, f + 84)];
        if platforms.iter().any(|&(x0, x1, py)| (x0..x1).contains(&x) && (py..py + 3).contains(&y)) {
            return Some(self.platform);
        }
        // A heap of sand (it'll settle a little).
        if (1485..1590).contains(&x) && y < f + 52 - (x - 1537).abs() {
            return Some(self.sand);
        }
        // Two columns to wall-jump between, and planks to burn.
        if ((1650..1674).contains(&x) && y < f + 210) || ((1725..1749).contains(&x) && y < f + 150) {
            return Some(self.stone);
        }
        if (1785..1845).contains(&x) && y < f + 45 {
            return Some(self.planks);
        }
        None
    }
}

impl ChunkGenerator for ArenaGen {
    fn bounds(&self) -> (ChunkPos, ChunkPos) {
        (ChunkPos::new(0, 0), ChunkPos::new(W - 1, H - 1))
    }

    fn spawn_point(&self) -> CellPos {
        match &self.real {
            // (Its ground there, wherever that lands.)
            Some(r) => r.start,
            None if self.layout == Layout::Sandbox => CellPos::new(START, FLOOR),
            None => CellPos::new(START, self.ground(START)),
        }
    }

    fn wild(&self) -> bool {
        false
    }

    fn stage(&self) -> Option<(CellPos, CellPos)> {
        Some((CellPos::new(STAGE.0, STAGE.1), CellPos::new(STAGE.2, STAGE.3)))
    }

    fn surface_hint(&self, x: i32) -> Option<i32> {
        match &self.real {
            Some(r) => Some(r.terrain.surface_at(x + r.shift.origin().x) - r.shift.origin().y),
            None if matches!(self.layout, Layout::Sandbox | Layout::Cave) => Some(FLOOR),
            None => Some(self.ground(x)),
        }
    }

    fn generate(&self, pos: ChunkPos) -> Chunk {
        let o = pos.origin();
        // The real world's chunk, moved here, inside the arena's frame.
        if let Some(r) = &self.real {
            let mut cells = r.terrain.generate(ChunkPos::new(pos.x + r.shift.x, pos.y + r.shift.y)).cells().to_vec();
            for ly in 0..CHUNK {
                for lx in 0..CHUNK {
                    if let Some(m) = self.frame(o.x + lx, o.y + ly) {
                        cells[(ly * CHUNK + lx) as usize] = m.map_or(Cell::AIR, |m| Cell::new(m, (((o.x + lx) * 7 + (o.y + ly) * 13) & 255) as u8));
                    }
                }
            }
            return Chunk::new(pos, cells);
        }
        let cells = (0..CHUNK)
            .flat_map(|ly| (0..CHUNK).map(move |lx| (lx, ly)))
            .map(|(lx, ly)| {
                let (x, y) = (o.x + lx, o.y + ly);
                self.at(x, y).map_or(Cell::AIR, |m| Cell::new(m, ((x * 7 + y * 13) & 255) as u8))
            })
            .collect();
        Chunk::new(pos, cells)
    }

    fn generate_with_spawns(&self, pos: ChunkPos) -> (Chunk, Vec<(CellPos, Spawn)>) {
        // (The dummies on the ground where they stand; none in the real
        // world's.)
        let ground = |x: i32| self.surface_hint(x).unwrap_or(FLOOR);
        let spawns = DUMMIES
            .iter()
            .filter(|_| self.real.is_none())
            .map(|&(x, kind)| (CellPos::new(x, ground(x)), Spawn::Creature(kind)))
            .filter(|(at, _)| at.chunk() == pos)
            .collect();
        (self.generate(pos), spawns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arena_has_its_pieces_and_every_dummy_once() {
        let m = MaterialTable::from_ron(include_str!("../../../assets/data/materials.ron")).unwrap();
        let g = ArenaGen::new(&m);
        assert_eq!(g.at(START, FLOOR), None, "the start is open");
        assert_eq!(g.at(START, FLOOR - 1), Some(g.stone), "and stood on");
        assert_eq!(g.at(540, FLOOR - 15), Some(g.water));
        assert_eq!(g.at(1395, FLOOR - 15), Some(g.lava));
        assert_eq!(g.at(1395, FLOOR - 3), None, "lava sits below the lip");
        let (lo, hi) = g.bounds();
        let mut n = 0;
        for cy in lo.y..=hi.y {
            for cx in lo.x..=hi.x {
                n += g.generate_with_spawns(ChunkPos::new(cx, cy)).1.len();
            }
        }
        assert_eq!(n, DUMMIES.len());
        // The stage: sealed, open inside, stood on, a step at the right.
        let (x0, f, x1, top) = STAGE;
        assert_eq!(g.at(x0 + 40, f), None);
        assert_eq!(g.at(x0 + 40, f - 1), Some(g.stone));
        assert_eq!(g.at(STEP_AT + 5, f + STEP - 1), Some(g.stone));
        assert_eq!(g.at(x0 - 1, f + 20), Some(g.bedrock));
        assert_eq!(g.at(x1, f + 20), Some(g.bedrock));
        assert_eq!(g.at(x0 + 40, top), Some(g.bedrock));
        assert!(g.at(x0 + 40, f - STAGE_WALL - 1).is_none(), "open sky below it");
    }

    #[test]
    fn every_layout_has_its_walls_its_stage_and_a_start_to_stand_on() {
        let m = MaterialTable::from_ron(include_str!("../../../assets/data/materials.ron")).unwrap();
        for l in ["flat", "cave", "slopes", "stairs", "real:1"] {
            let g = ArenaGen::with_layout(&m, Layout::parse(l));
            assert_eq!(g.layout.name(), l.split(':').next().unwrap());
            let s = g.spawn_point();
            let at = |x: i32, y: i32| {
                let c = g.generate(CellPos::new(x, y).chunk());
                let o = c.pos.origin();
                c.cells()[((y - o.y) * CHUNK + (x - o.x)) as usize].material
            };
            assert_ne!(at(s.x, s.y - 1), MaterialId::AIR, "{l}: stood on");
            let open = |x: i32, y: i32| !matches!(m.def(at(x, y)).kind, platypus_sim::Kind::Static | platypus_sim::Kind::Powder);
            assert!(open(s.x, s.y + 5) && open(s.x, s.y + 20), "{l}: room at the start");
            assert_eq!(at(5, 300), g.bedrock, "{l}: walled");
            assert_eq!(at(STAGE.0 + 40, STAGE.1 + 20), MaterialId::AIR, "{l}: the stage");
            assert_eq!(at(STAGE.0 - 1, STAGE.1 + 20), g.bedrock, "{l}: sealed");
        }
        assert_eq!(Layout::parse("real:7:100:-20"), Layout::Real { seed: 7, x: 100, y: -20 });
        assert_eq!(Layout::parse("nonsense"), Layout::Sandbox);
    }
}
