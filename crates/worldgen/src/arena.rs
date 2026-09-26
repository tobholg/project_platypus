//! The arena (`PLATYPUS_WORLD=arena`): a small walled sandbox for trying
//! weapons, spells, creatures and animations. One long floor with a bit of
//! everything along it, left to right: stairs and a ramp, a water pool,
//! one-way platforms, the open floor with training dummies (the player
//! starts here), a lava pit, a sand heap, and two tall columns to wall-jump
//! between beside a block of planks to burn.

use platypus_sim::{CHUNK, CellPos, Cell, Chunk, ChunkPos, MaterialId, MaterialTable};

use crate::{ChunkGenerator, Spawn};

/// Size in chunks.
const W: i32 = 20;
const H: i32 = 10;
/// The floor's top: solid below it.
pub const FLOOR: i32 = 160;
/// Where the player starts, and the dummies stand (x, what).
const START: i32 = 640;
const DUMMIES: [(i32, &str); 4] = [(700, "dummy"), (760, "dummy"), (820, "dummy"), (870, "sandbag")];

pub struct ArenaGen {
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
        ArenaGen {
            stone: m.expect_id("stone"),
            bedrock: m.expect_id("bedrock"),
            water: m.expect_id("water"),
            lava: m.expect_id("lava"),
            sand: m.expect_id("sand"),
            planks: m.expect_id("planks"),
            platform: m.expect_id("platform"),
        }
    }

    /// What's at (x, y) (`None`: air).
    fn at(&self, x: i32, y: i32) -> Option<MaterialId> {
        let f = FLOOR;
        let right = W * CHUNK;
        if y < 8 || x < 8 || x >= right - 8 {
            return Some(self.bedrock);
        }
        // Side walls, higher than anything can be thrown.
        if (x < 24 || x >= right - 24) && y < f + 320 {
            return Some(self.stone);
        }
        // Stairs up (5-cell steps every 16), a ledge, a ramp down.
        let ground = match x {
            40..160 => f + ((x - 40) / 16 + 1) * 5,
            160..200 => f + 40,
            200..280 => f + 40 - (x - 200) / 2,
            _ => f,
        };
        // Pits, sunk into the floor: (from, to, depth, what fills them, the
        // gap left above).
        let pits = [(310, 420, 50, self.water, 0), (900, 960, 24, self.lava, 4)];
        for (x0, x1, depth, fill, gap) in pits {
            if (x0..x1).contains(&x) && y >= f - depth && y < f {
                return (y < f - gap).then_some(fill);
            }
        }
        if y < ground {
            return Some(self.stone);
        }
        // Platforms to drop through and jump up on.
        let platforms = [(450, 510, f + 28), (470, 530, f + 56)];
        if platforms.iter().any(|&(x0, x1, py)| (x0..x1).contains(&x) && (py..py + 2).contains(&y)) {
            return Some(self.platform);
        }
        // A heap of sand (it'll settle a little).
        if (990..1060).contains(&x) && y < f + 35 - (x - 1025).abs() {
            return Some(self.sand);
        }
        // Two columns to wall-jump between, and planks to burn.
        if ((1100..1116).contains(&x) && y < f + 140) || ((1150..1166).contains(&x) && y < f + 100) {
            return Some(self.stone);
        }
        if (1190..1230).contains(&x) && y < f + 30 {
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
        CellPos::new(START, FLOOR)
    }

    fn wild(&self) -> bool {
        false
    }

    fn surface_hint(&self, _x: i32) -> Option<i32> {
        Some(FLOOR)
    }

    fn generate(&self, pos: ChunkPos) -> Chunk {
        let o = pos.origin();
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
        let spawns = DUMMIES
            .iter()
            .map(|&(x, kind)| (CellPos::new(x, FLOOR), Spawn::Creature(kind)))
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
        assert_eq!(g.at(360, FLOOR - 10), Some(g.water));
        assert_eq!(g.at(930, FLOOR - 10), Some(g.lava));
        assert_eq!(g.at(930, FLOOR - 2), None, "lava sits below the lip");
        let (lo, hi) = g.bounds();
        let mut n = 0;
        for cy in lo.y..=hi.y {
            for cx in lo.x..=hi.x {
                n += g.generate_with_spawns(ChunkPos::new(cx, cy)).1.len();
            }
        }
        assert_eq!(n, DUMMIES.len());
    }
}
