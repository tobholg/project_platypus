//! Setting furniture down (a chest, a crafting station): where it would go
//! for the cursor, and if it can't, why.
//!
//! A floor piece stands on the highest ground under it. Soft ground there
//! (`MaterialTable::soft`: dirt, grass, sand, gravel, snow) is levelled for
//! it, up to `LEVEL` cells (a block), what's taken out going into the pack
//! as dug; rock isn't: a lump of it in the way wants mining flat (the
//! pickaxe's precise mode takes whole blocks: flat faces). Under it, half
//! its columns must rest on the floor exactly and three quarters have
//! ground no more than `ROUGH` down (a dip is fine; a lump to perch on
//! isn't), and over it, room. Hung and wall
//! pieces have their own checks (`Crafting::try_place`).

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind, World};

/// How much soft ground a piece levels away under it (a block).
pub const LEVEL: i32 = 8;
/// How far under its floor ground may lie and still hold it up (half a
/// block).
pub const ROUGH: i32 = 4;

/// Why a piece can't go where the cursor is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// Nothing (enough) to stand on.
    Ground,
    /// Rock in the way under it: mine it flat.
    Rock,
    /// More soft ground in the way than it levels: dig it down.
    Mound,
    /// Something in the way where it would stand.
    Room,
    /// A hung piece with no ceiling over it.
    Ceiling,
    /// A wall piece with no wall behind it.
    Wall,
    /// Further off than the hands reach.
    Far,
}

impl Why {
    /// What the player is told.
    pub fn say(self) -> &'static str {
        match self {
            Why::Ground => "Needs solid ground under it",
            Why::Rock => "Uneven rock: mine it flat",
            Why::Mound => "Too much ground in the way: dig it down",
            Why::Room => "No room here",
            Why::Ceiling => "Needs a ceiling to hang from",
            Why::Wall => "Needs a wall behind it",
            Why::Far => "Too far away",
        }
    }
}

/// Where a piece would go for the cursor: its box (`x0`, `floor`: its
/// feet's row, `size`), and either the soft ground it levels (`Ok`) or why
/// not and what's in the way (`Err`), for the outline (`outline`) and the
/// hands.
#[derive(Clone, Debug)]
pub struct Tried {
    pub x0: i32,
    pub floor: i32,
    pub size: (i32, i32),
    pub at: Result<Level, Why>,
    /// What's in the way (rock, something where it would stand) or under
    /// it missing (ground).
    pub cells: Vec<CellPos>,
}

/// The soft ground a piece levels away (inclusive rows), if any.
#[derive(Clone, Copy, Debug)]
pub struct Level(pub Option<(CellPos, CellPos)>);

impl Tried {
    /// Its feet (the middle of its bottom).
    pub fn feet(&self) -> Vec2 {
        Vec2::new((self.x0 + self.size.0 / 2) as f32, self.floor as f32)
    }

    /// Its feet, if it can go there.
    pub fn spot(&self) -> Option<Vec2> {
        self.at.is_ok().then(|| self.feet())
    }

    /// Too far from `from` (the hands) to set down: so told.
    pub fn within(mut self, from: Vec2, reach: f32) -> Tried {
        if self.at.is_ok() && self.feet().distance(from) > reach {
            self.at = Err(Why::Far);
        }
        self
    }

    /// The edit that levels its ground, if it has any to level.
    pub fn level(&self) -> Option<platypus_sim::WorldEdit> {
        match self.at {
            Ok(Level(Some((min, max)))) => Some(platypus_sim::WorldEdit::Level { min, max }),
            _ => None,
        }
    }
}

/// Whether a cell leaves room for a piece: nothing solid (tall grass,
/// smoke, flames, water or blood it stands in, a slime's goo bodies pass
/// through).
pub fn room(mats: &platypus_sim::MaterialTable, c: platypus_sim::Cell) -> bool {
    c.is_air() || c.flags & platypus_sim::cell::flags::HELD != 0 || matches!(mats.phys(c.material).kind, Kind::Plant | Kind::Gas | Kind::Fire | Kind::Liquid)
}

/// Where a floor piece `w` × `h` would go for the cursor (see the module).
pub fn try_floor(world: &World, cursor: Vec2, (w, h): (i32, i32)) -> Tried {
    let mats = world.materials();
    let solid = |p: CellPos| world.get(p).is_some_and(|c| matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder));
    let soft = |p: CellPos| world.get(p).is_some_and(|c| mats.soft(c.material));
    let room = |p: CellPos| world.get(p).is_some_and(|c| room(mats, c));
    let x0 = cursor.x.floor() as i32 - w / 2;
    let y = cursor.y.floor() as i32 + 6;
    // Each column's ground: the first solid cell going down.
    let tops: Vec<Option<i32>> = (0..w).map(|dx| (0..30).map(|d| y - d).find(|&yy| solid(CellPos::new(x0 + dx, yy)))).collect();
    let Some(&top) = tops.iter().flatten().max() else {
        return Tried { x0, floor: y - 30, size: (w, h), at: Err(Why::Ground), cells: Vec::new() };
    };
    let high = top + 1;
    // Each floor from the highest ground down to `LEVEL` under it: what
    // it levels, what's in its way, what holds it up. The best is flush on
    // the most columns (then the least dug).
    struct Try {
        floor: i32,
        rock: Vec<CellPos>,
        blocked: Vec<CellPos>,
        bare: Vec<CellPos>,
        flush: usize,
    }
    let tries: Vec<Try> = (high - LEVEL..=high)
        .rev()
        .map(|floor| {
            let (mut rock, mut blocked) = (Vec::new(), Vec::new());
            for dy in 0..h {
                for dx in 0..w {
                    let p = CellPos::new(x0 + dx, floor + dy);
                    if room(p) || (p.y < high && soft(p)) {
                        continue;
                    }
                    if p.y < high && solid(p) { rock.push(p) } else { blocked.push(p) }
                }
            }
            let bare: Vec<CellPos> = (0..w).map(|dx| x0 + dx).filter(|&x| !(floor - 1 - ROUGH..floor).any(|yy| solid(CellPos::new(x, yy)))).map(|x| CellPos::new(x, floor - 1)).collect();
            let flush = (0..w).filter(|&dx| solid(CellPos::new(x0 + dx, floor - 1))).count();
            Try { floor, rock, blocked, bare, flush }
        })
        .collect();
    let held = |t: &Try| (w as usize - t.bare.len()) * 4 >= w as usize * 3 && t.flush * 2 >= w as usize;
    if let Some(t) = tries.iter().filter(|t| t.rock.is_empty() && t.blocked.is_empty() && held(t)).max_by_key(|t| (t.flush, t.floor)) {
        let level = (t.floor < high).then(|| (CellPos::new(x0, t.floor), CellPos::new(x0 + w - 1, high - 1)));
        return Tried { x0, floor: t.floor, size: (w, h), at: Ok(Level(level)), cells: Vec::new() };
    }
    // Why not: something where it would stand at the top; else rock in the
    // way of every floor that would be held up; else more soft ground than
    // it levels; else no ground.
    let first = &tries[0];
    if !first.blocked.is_empty() {
        return Tried { x0, floor: high, size: (w, h), at: Err(Why::Room), cells: first.blocked.clone() };
    }
    if let Some(t) = tries.iter().filter(|t| held(t) && !t.rock.is_empty()).min_by_key(|t| t.rock.len()) {
        return Tried { x0, floor: t.floor, size: (w, h), at: Err(Why::Rock), cells: t.rock.clone() };
    }
    if tops.iter().flatten().any(|&t| t < high - 1 - LEVEL) && (0..w).any(|dx| soft(CellPos::new(x0 + dx, top))) {
        let mound = (0..w).map(|dx| CellPos::new(x0 + dx, top)).filter(|&p| soft(p)).collect();
        return Tried { x0, floor: high, size: (w, h), at: Err(Why::Mound), cells: mound };
    }
    Tried { x0, floor: high, size: (w, h), at: Err(Why::Ground), cells: first.bare.clone() }
}

/// The piece's box outlined where it would go: cyan if it can, red if not
/// (what's in the way or missing under it marked), the soft ground it'd
/// level tinted.
pub fn outline(gizmos: &mut Gizmos, world: &World, tried: &Tried) {
    let (w, h) = tried.size;
    let size = Vec2::new(w as f32, h as f32);
    let ok = tried.at.is_ok();
    let color = if ok { Color::srgba(0.4, 0.9, 1.0, 0.9) } else { Color::srgba(1.0, 0.35, 0.3, 0.9) };
    gizmos.rect_2d(bevy::math::Isometry2d::from_translation(tried.feet() + Vec2::new(0.0, size.y / 2.0)), size, color);
    let mark = |gizmos: &mut Gizmos, p: CellPos, c: Color| gizmos.rect_2d(bevy::math::Isometry2d::from_translation(Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5)), Vec2::splat(0.8), c);
    for &p in &tried.cells {
        mark(gizmos, p, Color::srgba(1.0, 0.25, 0.2, 0.95));
    }
    if let Ok(Level(Some((min, max)))) = tried.at {
        let mats = world.materials();
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                let p = CellPos::new(x, y);
                if world.get(p).is_some_and(|c| mats.soft(c.material)) {
                    mark(gizmos, p, Color::srgba(1.0, 0.85, 0.3, 0.6));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use platypus_sim::{Cell, Chunk, ChunkPos, MaterialTable};

    use super::*;

    /// A chunk of air with stone up to row 19 (its top at 20), and what
    /// `paint` puts over it (material, x range, y range).
    fn world(paint: &[(&str, std::ops::Range<i32>, std::ops::Range<i32>)]) -> World {
        let mats = Arc::new(MaterialTable::from_ron(include_str!("../../../../assets/data/materials.ron")).unwrap());
        let mut w = World::new(1, mats.clone());
        w.insert_chunk(Chunk::filled(ChunkPos::new(0, 0), Cell::AIR));
        let mut rng = platypus_sim::rng::Rng::seeded(&[1]);
        let mut put = |name: &str, xs: std::ops::Range<i32>, ys: std::ops::Range<i32>| {
            let id = mats.expect_id(name);
            for x in xs {
                for y in ys.clone() {
                    w.set(CellPos::new(x, y), mats.spawn(id, &mut rng));
                }
            }
        };
        put("stone", 0..64, 0..20);
        for (m, xs, ys) in paint {
            put(m, xs.clone(), ys.clone());
        }
        w
    }

    const SIZE: (i32, i32) = (24, 12);
    const CURSOR: Vec2 = Vec2::new(32.0, 22.0);

    #[test]
    fn on_flat_rock_it_stands_on_the_rock() {
        let t = try_floor(&world(&[]), CURSOR, SIZE);
        assert!(matches!(t.at, Ok(Level(None))), "{t:?}");
        assert_eq!(t.floor, 20);
    }

    #[test]
    fn a_dirt_lump_is_levelled_away_and_dug_into_the_pack() {
        let mut w = world(&[("dirt", 28..36, 20..23)]);
        let t = try_floor(&w, CURSOR, SIZE);
        assert!(t.at.is_ok(), "{t:?}");
        assert_eq!(t.floor, 20, "flush on the rock, the lump cut down");
        let report = w.apply_edit(&t.level().expect("a lump to level"));
        assert_eq!(report.removed.iter().map(|r| r.1).sum::<u32>(), 8 * 3);
        assert!(try_floor(&w, CURSOR, SIZE).at.is_ok());
    }

    #[test]
    fn a_rock_lump_wants_mining_flat() {
        let t = try_floor(&world(&[("stone", 28..36, 20..23)]), CURSOR, SIZE);
        assert_eq!(t.at.err(), Some(Why::Rock), "{t:?}");
        assert!(!t.cells.is_empty(), "the lump is marked");
    }

    #[test]
    fn a_mound_up_to_a_block_is_levelled_and_a_higher_one_wants_digging() {
        let t = try_floor(&world(&[("dirt", 28..36, 20..26)]), CURSOR, SIZE);
        assert!(t.at.is_ok(), "{t:?}");
        assert_eq!(t.floor, 20, "six high: levelled to the rock");
        let t = try_floor(&world(&[("dirt", 28..36, 20..32)]), Vec2::new(32.0, 34.0), SIZE);
        assert_eq!(t.at.err(), Some(Why::Mound), "twelve high, aimed over it: {t:?}");
    }

    #[test]
    fn it_stands_in_a_puddle() {
        let t = try_floor(&world(&[("water", 20..44, 20..23)]), CURSOR, SIZE);
        assert!(matches!(t.at, Ok(Level(None))), "{t:?}");
        assert_eq!(t.floor, 20);
    }

    #[test]
    fn a_dip_under_an_edge_will_do() {
        let mut w = world(&[]);
        (20..24).for_each(|x| (17..20).for_each(|y| _ = w.set(CellPos::new(x, y), Cell::AIR)));
        let t = try_floor(&w, CURSOR, SIZE);
        assert!(matches!(t.at, Ok(Level(None))), "{t:?}");
        assert_eq!(t.floor, 20);
    }

    #[test]
    fn nothing_under_it_or_something_where_it_would_stand_says_so() {
        let mut w = world(&[]);
        let air = |w: &mut World| (20..64).for_each(|x| (0..20).for_each(|y| _ = w.set(CellPos::new(x, y), Cell::AIR)));
        air(&mut w);
        assert_eq!(try_floor(&w, CURSOR, SIZE).at.err(), Some(Why::Ground));
        let t = try_floor(&world(&[("stone", 20..44, 28..34)]), CURSOR, SIZE);
        assert_eq!(t.at.err(), Some(Why::Room), "a ceiling 8 over the floor, under a 12-tall piece");
    }
}
