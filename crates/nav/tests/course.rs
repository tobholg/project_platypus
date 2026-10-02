//! The arena's obstacle course (`worldgen::arena::Layout::Course`), the real
//! cells: each kind of body finds the way from the far left up onto the
//! tower (or, the troll, can't: the first wall is higher than its jump).

use std::sync::Arc;

use glam::{IVec2, Vec2};
use platypus_nav::world::WorldGrid;
use platypus_nav::{Nav, Profile, find, node_of};
use platypus_physics::MovementStats;
use platypus_sim::{ChunkPos, MaterialTable, World};
use platypus_worldgen::ChunkGenerator;
use platypus_worldgen::arena::{ArenaGen, COURSE, FLOOR, Layout};

fn course() -> World {
    let m = Arc::new(MaterialTable::from_ron(include_str!("../../../assets/data/materials.ron")).unwrap());
    let g = ArenaGen::with_layout(&m, Layout::Course);
    let mut w = World::new(1, m);
    let (lo, hi) = g.bounds();
    for y in lo.y..=hi.y {
        for x in lo.x..=hi.x {
            w.insert_chunk(g.generate(ChunkPos::new(x, y)));
        }
    }
    w
}

/// At the game's pace (the arena's "Middle" tempo: 0.8).
fn paced(mut s: MovementStats) -> MovementStats {
    let k = 0.8;
    s.run_speed *= k;
    s.max_fall *= k;
    for a in [&mut s.ground_accel, &mut s.ground_decel, &mut s.air_accel, &mut s.gravity] {
        *a *= k * k;
    }
    s
}

fn reaches(size: (f32, f32), stats: MovementStats, drop: f32) -> (bool, usize) {
    let w = course();
    let grid = WorldGrid(&w);
    let p = Profile::new(size, &paced(stats), drop);
    let mut nav = Nav::default();
    let from = node_of(Vec2::new(150.0, FLOOR as f32));
    let to = node_of(Vec2::new((COURSE.0 + 60) as f32, (FLOOR + COURSE.1) as f32));
    let path = find(&mut nav.view(&grid, p.size), &p, from, to, 1, 20_000);
    (path.whole, path.looked)
}

#[test]
fn walkers_climbers_and_the_troll() {
    let orc = MovementStats { run_speed: 93.0, jump_height: 36.0, step_height: 5, ..MovementStats::default() };
    assert!(reaches((11.0, 24.0), orc, 105.0).0, "the orc");
    let spider = MovementStats { run_speed: 123.0, jump_height: 33.0, step_height: 3, cling: true, ..MovementStats::default() };
    let s = reaches((24.0, 18.0), spider, 150.0);
    assert!(s.0, "the spider ({} nodes)", s.1);
    let troll = MovementStats { run_speed: 57.0, jump_height: 24.0, step_height: 8, ..MovementStats::default() };
    assert!(!reaches((23.0, 51.0), troll, 135.0).0, "the troll can't");
    let _ = IVec2::ZERO;
}

