//! Behavioural tests of the cell world, using the real `materials.ron`.

use std::sync::Arc;

use platypus_sim::rng::Rng;
use platypus_sim::{CHUNK, Cell, CellPos, Chunk, ChunkPos, Landing, MaterialId, MaterialTable, Particle, World, WorldEdit, store};

const MATERIALS: &str = include_str!("../../../assets/data/materials.ron");

fn mats() -> Arc<MaterialTable> {
    Arc::new(MaterialTable::from_ron(MATERIALS).expect("materials.ron is valid"))
}

/// A `w`×`h`-chunk box: stone floor and walls, air inside.
fn boxed_world(w: i32, h: i32, seed: u64) -> World {
    let m = mats();
    let stone = Cell::new(m.expect_id("stone"), 0);
    let mut world = World::new(seed, m);
    for cy in 0..h {
        for cx in 0..w {
            world.insert_chunk(Chunk::filled(ChunkPos::new(cx, cy), Cell::AIR));
        }
    }
    let (wc, hc) = (w * CHUNK, h * CHUNK);
    for x in 0..wc {
        world.set(CellPos::new(x, 0), stone);
    }
    for y in 0..hc {
        world.set(CellPos::new(0, y), stone);
        world.set(CellPos::new(wc - 1, y), stone);
    }
    world
}

fn count(world: &World, id: MaterialId) -> usize {
    world.chunks().flat_map(|c| c.cells()).filter(|c| c.material == id).count()
}

fn burning(world: &World) -> usize {
    world.chunks().flat_map(|c| c.cells()).filter(|c| c.flags & platypus_sim::cell::flags::BURNING != 0).count()
}

fn run_until_asleep(world: &mut World, max_ticks: usize) -> usize {
    for t in 0..max_ticks {
        if world.step().active_chunks == 0 {
            return t;
        }
    }
    panic!("world did not settle within {max_ticks} ticks");
}

#[test]
fn sand_falls_piles_and_sleeps() {
    let mut w = boxed_world(1, 1, 1);
    let sand = w.materials().expect_id("sand");
    w.apply_edit(&WorldEdit::Paint { center: CellPos::new(32, 50), radius: 6, material: sand, overwrite: false });
    let before = count(&w, sand);
    run_until_asleep(&mut w, 2_000);
    assert_eq!(count(&w, sand), before, "sand is conserved");
    // Everything rests on the floor or on other sand: nothing floating.
    for x in 1..CHUNK - 1 {
        for y in 2..CHUNK {
            let here = w.get(CellPos::new(x, y)).unwrap();
            if here.material == sand {
                let below = w.get(CellPos::new(x, y - 1)).unwrap();
                assert!(!below.is_air(), "floating sand at ({x},{y})");
            }
        }
    }
    // A pile, not a column: sand spread sideways.
    let bottom: usize = (1..CHUNK - 1).filter(|&x| w.get(CellPos::new(x, 1)).unwrap().material == sand).count();
    assert!(bottom > 13, "sand should spread into a pile, bottom row has {bottom}");
}

#[test]
fn water_flows_across_chunk_borders_and_levels() {
    let mut w = boxed_world(3, 1, 2);
    let water = w.materials().expect_id("water");
    w.apply_edit(&WorldEdit::Paint { center: CellPos::new(20, 30), radius: 14, material: water, overwrite: false });
    let before = count(&w, water);
    for _ in 0..3_000 {
        w.step();
    }
    assert_eq!(count(&w, water), before, "water is conserved");
    let right_chunk = w.chunk(ChunkPos::new(2, 0)).unwrap().cells().iter().filter(|c| c.material == water).count();
    assert!(right_chunk > 0, "water reached the far chunk");
    // Level: no column is more than 2 cells higher than another.
    let heights: Vec<i32> = (1..3 * CHUNK - 1)
        .map(|x| (1..CHUNK).filter(|&y| w.get(CellPos::new(x, y)).unwrap().material == water).count() as i32)
        .collect();
    let (lo, hi) = (heights.iter().min().unwrap(), heights.iter().max().unwrap());
    assert!(hi - lo <= 2, "water not level: {lo}..{hi}");
}

#[test]
fn oil_floats_on_water() {
    let mut w = boxed_world(1, 1, 3);
    let (water, oil) = (w.materials().expect_id("water"), w.materials().expect_id("oil"));
    w.apply_edit(&WorldEdit::Paint { center: CellPos::new(32, 12), radius: 6, material: oil, overwrite: false });
    w.apply_edit(&WorldEdit::Paint { center: CellPos::new(32, 40), radius: 8, material: water, overwrite: false });
    for _ in 0..3_000 {
        w.step();
    }
    let mean_y = |id| {
        let ys: Vec<i32> =
            (0..CHUNK).flat_map(|x| (0..CHUNK).map(move |y| (x, y))).filter(|&(x, y)| w.get(CellPos::new(x, y)).unwrap().material == id).map(|(_, y)| y).collect();
        ys.iter().sum::<i32>() as f32 / ys.len() as f32
    };
    assert!(mean_y(oil) > mean_y(water), "oil should end up above water");
}

#[test]
fn lava_meets_water() {
    let mut w = boxed_world(1, 1, 4);
    let m = w.materials().clone();
    w.apply_edit(&WorldEdit::Paint { center: CellPos::new(32, 8), radius: 5, material: m.expect_id("water"), overwrite: false });
    w.apply_edit(&WorldEdit::Paint { center: CellPos::new(32, 40), radius: 4, material: m.expect_id("lava"), overwrite: false });
    for _ in 0..400 {
        w.step();
    }
    assert!(count(&w, m.expect_id("obsidian")) > 0, "lava + water makes obsidian");
    assert!(count(&w, m.expect_id("steam")) + count(&w, m.expect_id("water")) > 0);
}

/// Fire must reliably take: across 20 seeds, a lit slab of wood burns up.
#[test]
fn fire_spreads_through_wood_and_burns_out() {
    for seed in 100..120 {
        let mut w = boxed_world(1, 1, seed);
        let m = w.materials().clone();
        let wood = m.expect_id("wood");
        for x in 10..50 {
            for y in 1..6 {
                w.set(CellPos::new(x, y), Cell::new(wood, 0));
            }
        }
        w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(10, 5), radius: 1 });
        for _ in 0..6_000 {
            w.step();
        }
        assert!(count(&w, wood) < 20, "seed {seed}: {} of 200 wood left", count(&w, wood));
        assert_eq!(burning(&w), 0, "seed {seed}: burned out");
        assert_eq!(count(&w, m.expect_id("fire")), 0, "seed {seed}: flames gone");
    }
}

#[test]
fn painted_flames_light_wood() {
    let mut w = boxed_world(1, 1, 5);
    let m = w.materials().clone();
    let wood = m.expect_id("wood");
    for x in 10..50 {
        for y in 1..6 {
            w.set(CellPos::new(x, y), Cell::new(wood, 0));
        }
    }
    w.apply_edit(&WorldEdit::Paint { center: CellPos::new(10, 7), radius: 1, material: m.expect_id("fire"), overwrite: true });
    for _ in 0..6_000 {
        w.step();
    }
    assert!(count(&w, wood) < 40, "most of the wood burned, {} left", count(&w, wood));
    assert_eq!(count(&w, m.expect_id("fire")), 0, "fire burned out");
}

#[test]
fn settled_world_costs_nothing() {
    let mut w = boxed_world(2, 2, 6);
    let sand = w.materials().expect_id("sand");
    w.apply_edit(&WorldEdit::Paint { center: CellPos::new(60, 100), radius: 10, material: sand, overwrite: false });
    run_until_asleep(&mut w, 3_000);
    let s = w.step();
    assert_eq!(s.active_chunks, 0);
    assert_eq!(s.visited_cells, 0);
}

#[test]
fn unloaded_neighbours_are_walls_until_loaded() {
    let m = mats();
    let water = m.expect_id("water");
    let stone = Cell::new(m.expect_id("stone"), 0);
    let mut w = World::new(7, m);
    w.insert_chunk(Chunk::filled(ChunkPos::new(0, 0), Cell::AIR));
    for x in 0..CHUNK {
        w.set(CellPos::new(x, 0), stone);
    }
    w.apply_edit(&WorldEdit::Paint { center: CellPos::new(60, 10), radius: 6, material: water, overwrite: false });
    let before = count(&w, water);
    run_until_asleep(&mut w, 3_000);
    assert_eq!(count(&w, water), before, "nothing leaks into unloaded chunks");

    // Loading the neighbour wakes the border and water flows in.
    let mut right = Chunk::filled(ChunkPos::new(1, 0), Cell::AIR);
    for x in 0..CHUNK as usize {
        right.set(x, 0, stone);
    }
    w.insert_chunk(right);
    for _ in 0..500 {
        w.step();
    }
    let moved = w.chunk(ChunkPos::new(1, 0)).unwrap().cells().iter().filter(|c| c.material == water).count();
    assert!(moved > 0, "water flowed into the newly loaded chunk");
}

#[test]
fn store_roundtrips_chunks() {
    let mut w = boxed_world(1, 1, 8);
    let sand = w.materials().expect_id("sand");
    w.apply_edit(&WorldEdit::Paint { center: CellPos::new(30, 30), radius: 9, material: sand, overwrite: false });
    let chunk = w.chunk(ChunkPos::new(0, 0)).unwrap();
    let bytes = store::encode(chunk);
    assert!(bytes.len() < 32 * 1024 / 4, "compresses well ({} bytes)", bytes.len());
    let back = store::decode(chunk.pos, &bytes).unwrap();
    assert_eq!(store::checksum(&back), store::checksum(chunk));
}

#[test]
fn dig_reports_removed_materials_and_respects_hardness() {
    let mut w = boxed_world(1, 1, 9);
    let m = w.materials().clone();
    let (dirt, bedrock) = (m.expect_id("dirt"), m.expect_id("bedrock"));
    for x in 20..40 {
        for y in 20..40 {
            w.set(CellPos::new(x, y), Cell::new(if x < 30 { dirt } else { bedrock }, 0));
        }
    }
    let r = w.apply_edit(&WorldEdit::Dig { center: CellPos::new(30, 30), radius: 4, max_hardness: 100 });
    assert_eq!(r.removed.len(), 1);
    assert_eq!(r.removed[0].0, dirt);
    assert!(w.get(CellPos::new(31, 30)).unwrap().material == bedrock, "bedrock survives");
}

/// The co-op contract: same seed + same edits = same world, for any thread count.
#[test]
fn stepping_is_deterministic_across_thread_counts() {
    fn run(threads: usize) -> Vec<u64> {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
        pool.install(|| {
            let mut w = boxed_world(4, 3, 42);
            let m = w.materials().clone();
            let palette = ["sand", "water", "oil", "lava", "gravel", "fire", "wood", "acid", "stone", "methane", "ice"];
            let mut rng = Rng::seeded(&[99]);
            for _ in 0..60 {
                let name = palette[(rng.next_u32() as usize) % palette.len()];
                let x = 8 + (rng.next_u32() % (4 * CHUNK as u32 - 16)) as i32;
                let y = 8 + (rng.next_u32() % (3 * CHUNK as u32 - 16)) as i32;
                w.apply_edit(&WorldEdit::Paint { center: CellPos::new(x, y), radius: 5, material: m.expect_id(name), overwrite: false });
            }
            w.apply_edit(&WorldEdit::Heat { center: CellPos::new(100, 90), radius: 20, amount: 1600 });
            w.apply_edit(&WorldEdit::Explode { center: CellPos::new(60, 40), radius: 12, power: 90 });
            w.splash([150.0, 150.0], m.expect_id("water"), 120, 3.0);
            for _ in 0..600 {
                w.step();
            }
            let mut sums: Vec<(ChunkPos, u64)> = w.chunks().map(|c| (c.pos, store::checksum(c))).collect();
            sums.sort();
            sums.into_iter().map(|(_, s)| s).collect()
        })
    }
    let one = run(1);
    assert_eq!(one, run(8));
    assert_eq!(one, run(3));
}

/// Ticks of mining a single cell of `name` needs with a given pickaxe.
fn ticks_to_mine(name: &str, power: u8) -> Option<usize> {
    let mut w = boxed_world(1, 1, 10);
    let id = w.materials().expect_id(name);
    let at = CellPos::new(30, 30);
    w.set(at, Cell::new(id, 0));
    for t in 1..=400 {
        w.apply_edit(&WorldEdit::Mine { center: at, radius: 0, power, max_hardness: 200 });
        if w.get(at).unwrap().is_air() {
            return Some(t);
        }
    }
    None
}

#[test]
fn mining_time_scales_with_hardness() {
    let (dirt, stone, obsidian) = (ticks_to_mine("dirt", 4), ticks_to_mine("stone", 4), ticks_to_mine("obsidian", 4));
    let (dirt, stone, obsidian) = (dirt.unwrap(), stone.unwrap(), obsidian.unwrap());
    assert!(dirt < stone && stone < obsidian, "dirt {dirt} < stone {stone} < obsidian {obsidian}");
    assert_eq!(ticks_to_mine("bedrock", 50), None, "bedrock never breaks");
    assert!(ticks_to_mine("stone", 12).unwrap() < stone, "a stronger pickaxe is faster");
}

#[test]
fn mining_damage_persists_between_swings() {
    let mut w = boxed_world(1, 1, 11);
    let stone = w.materials().expect_id("stone");
    let at = CellPos::new(20, 20);
    w.set(at, Cell::new(stone, 0));
    w.apply_edit(&WorldEdit::Mine { center: at, radius: 0, power: 10, max_hardness: 200 });
    let c = w.get(at).unwrap();
    assert_eq!(c.material, stone);
    assert!(c.life >= 10, "damage recorded in the cell");
}

#[test]
fn explosions_crater_crumble_and_spare_bedrock() {
    let mut w = boxed_world(2, 2, 12);
    let m = w.materials().clone();
    let (stone, bedrock, wood) = (m.expect_id("stone"), m.expect_id("bedrock"), m.expect_id("wood"));
    for x in 1..127 {
        for y in 1..60 {
            let id = if x == 64 { bedrock } else if y > 50 { wood } else { stone };
            w.set(CellPos::new(x, y), Cell::new(id, 0));
        }
    }
    let r = w.apply_edit(&WorldEdit::Explode { center: CellPos::new(64, 40), radius: 14, power: 90 });
    assert!(r.removed.iter().any(|&(id, n)| id == stone && n > 200), "crater dug: {:?}", r.removed);
    assert!((1..60).all(|y| w.get(CellPos::new(64, y)).unwrap().material == bedrock), "bedrock untouched");
    assert!(count(&w, m.expect_id("gravel")) > 0, "rim crumbled into gravel");
    assert!(count(&w, m.expect_id("fire")) > 0, "fire in the crater or burning wood");
}

#[test]
fn igniting_burns_flammables_only() {
    let mut w = boxed_world(1, 1, 13);
    let m = w.materials().clone();
    let (stone, oil) = (m.expect_id("stone"), m.expect_id("oil"));
    for x in 10..30 {
        w.set(CellPos::new(x, 10), Cell::new(if x < 20 { stone } else { oil }, 0));
    }
    w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(20, 10), radius: 12 });
    let burning_at = |x| w.get(CellPos::new(x, 10)).unwrap().flags & platypus_sim::cell::flags::BURNING != 0;
    assert!((10..20).all(|x| w.get(CellPos::new(x, 10)).unwrap().material == stone && !burning_at(x)), "stone doesn't burn");
    assert!((20..30).all(|x| w.get(CellPos::new(x, 10)).unwrap().material == oil && burning_at(x)), "oil burns in place");
}

#[test]
fn detached_fragments_fall_but_anchored_rock_stays() {
    let mut w = boxed_world(1, 1, 14);
    let stone = w.materials().expect_id("stone");
    // A floating 6×2 slab, and a pillar standing on the floor.
    for x in 20..26 {
        for y in 40..42 {
            w.set(CellPos::new(x, y), Cell::new(stone, 0));
        }
    }
    for y in 1..30 {
        w.set(CellPos::new(45, y), Cell::new(stone, 0));
    }
    // Knock a corner off the slab and the top off the pillar.
    w.apply_edit(&WorldEdit::Dig { center: CellPos::new(20, 40), radius: 0, max_hardness: 200 });
    w.apply_edit(&WorldEdit::Dig { center: CellPos::new(45, 29), radius: 1, max_hardness: 200 });
    for _ in 0..300 {
        w.step();
    }
    let slab_left = (20..26).flat_map(|x| (40..42).map(move |y| (x, y))).filter(|&(x, y)| w.get(CellPos::new(x, y)).unwrap().material == stone).count();
    assert_eq!(slab_left, 0, "the slab fell");
    let on_floor = (1..63).filter(|&x| w.get(CellPos::new(x, 1)).unwrap().material == stone && x != 45).count();
    assert!(on_floor >= 3, "slab rubble reached the floor ({on_floor})");
    assert!((1..28).all(|y| w.get(CellPos::new(45, y)).unwrap().material == stone), "pillar still stands");
}

#[test]
fn explosions_leave_no_floating_specks() {
    let mut w = boxed_world(2, 2, 15);
    let m = w.materials().clone();
    let stone = m.expect_id("stone");
    for x in 1..127 {
        for y in 1..90 {
            w.set(CellPos::new(x, y), Cell::new(stone, 0));
        }
    }
    let center = CellPos::new(64, 60);
    w.apply_edit(&WorldEdit::Explode { center, radius: 20, power: 100 });
    for _ in 0..400 {
        w.step();
    }
    // No non-loose stone cell near the crater may be without solid neighbours.
    let solid = |x: i32, y: i32| {
        w.get(CellPos::new(x, y)).is_none_or(|c| matches!(m.phys(c.material).kind, platypus_sim::Kind::Static | platypus_sim::Kind::Powder))
    };
    for x in 30..98 {
        for y in 30..90 {
            let c = w.get(CellPos::new(x, y)).unwrap();
            if c.material == stone && c.flags & platypus_sim::cell::flags::LOOSE == 0 {
                let neighbours = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().filter(|(dx, dy)| solid(x + dx, y + dy)).count();
                assert!(neighbours > 0, "floating stone speck at ({x},{y})");
            }
        }
    }
}

// ---- temperature ----------------------------------------------------------

fn fill(w: &mut World, name: &str, x0: i32, x1: i32, y0: i32, y1: i32) {
    let id = w.materials().expect_id(name);
    let mut rng = Rng::seeded(&[x0 as u64, y0 as u64]);
    for x in x0..x1 {
        for y in y0..y1 {
            let c = w.materials().spawn(id, &mut rng);
            w.set(CellPos::new(x, y), c);
        }
    }
}

#[test]
fn enough_heat_melts_stone_into_lava() {
    let mut w = boxed_world(1, 1, 20);
    fill(&mut w, "stone", 1, 63, 1, 30);
    w.apply_edit(&WorldEdit::Heat { center: CellPos::new(32, 20), radius: 5, amount: 2500 });
    for _ in 0..30 {
        w.step();
    }
    assert!(count(&w, w.materials().expect_id("lava")) > 20, "stone melted");
}

#[test]
fn lava_heats_rock_without_melting_it_and_the_region_sleeps() {
    let mut w = boxed_world(1, 1, 21);
    fill(&mut w, "stone", 1, 63, 1, 40);
    fill(&mut w, "lava", 20, 44, 20, 40);
    let stone_before = count(&w, w.materials().expect_id("stone"));
    let settled = run_until_asleep(&mut w, 6_000);
    let stone = w.materials().expect_id("stone");
    assert_eq!(count(&w, stone), stone_before, "lava (1200 °C) can't melt stone (1400 °C)");
    let touching = w.get(CellPos::new(19, 30)).unwrap();
    assert_eq!(touching.material, stone);
    assert!(touching.heat > 500, "rock next to lava glows, heat {}", touching.heat);
    let far = w.get(CellPos::new(3, 5)).unwrap();
    assert_eq!(far.heat, 0, "far rock stays at ambient");
    assert!(settled < 6_000);
}

#[test]
fn hot_rock_cools_back_to_ambient_and_sleeps() {
    let mut w = boxed_world(1, 1, 22);
    fill(&mut w, "stone", 1, 63, 1, 30);
    w.apply_edit(&WorldEdit::Heat { center: CellPos::new(32, 15), radius: 8, amount: 900 });
    run_until_asleep(&mut w, 6_000);
    let hot = w.chunks().flat_map(|c| c.cells()).filter(|c| c.heat != 0).count();
    assert_eq!(hot, 0, "all heat dissipated");
}

#[test]
fn water_boils_and_freezes() {
    let mut w = boxed_world(1, 1, 23);
    let m = w.materials().clone();
    fill(&mut w, "water", 10, 20, 1, 6);
    fill(&mut w, "water", 40, 50, 1, 6);
    w.apply_edit(&WorldEdit::Heat { center: CellPos::new(15, 3), radius: 6, amount: 300 });
    w.apply_edit(&WorldEdit::Heat { center: CellPos::new(45, 3), radius: 6, amount: -120 });
    for _ in 0..5 {
        w.step();
    }
    assert!(count(&w, m.expect_id("steam")) > 10, "boiled");
    assert!(count(&w, m.expect_id("ice")) > 10, "froze");
    // In a 15 °C world the ice melts again.
    for _ in 0..4_000 {
        w.step();
    }
    assert_eq!(count(&w, m.expect_id("ice")), 0, "ice melted back");
}

#[test]
fn wood_catches_fire_from_heat_alone() {
    let mut w = boxed_world(1, 1, 24);
    fill(&mut w, "wood", 10, 30, 1, 10);
    w.apply_edit(&WorldEdit::Heat { center: CellPos::new(20, 5), radius: 3, amount: 400 });
    for _ in 0..10 {
        w.step();
    }
    assert!(burning(&w) > 0, "hot wood started burning");
}

#[test]
fn snow_depends_on_climate() {
    for (temp, survives) in [(-10, true), (15, false)] {
        let mut w = boxed_world(1, 1, 25);
        w.set_climate(platypus_sim::Climate { surface_temp: temp, ..Default::default() });
        let snow = w.materials().expect_id("snow");
        for x in 10..20 {
            w.set(CellPos::new(x, 1), Cell::new(snow, 0)); // at ambient
        }
        for _ in 0..200 {
            w.step();
        }
        assert_eq!(count(&w, snow) > 0, survives, "snow at {temp} °C");
    }
}

#[test]
fn a_methane_pocket_goes_up_in_a_chain_of_explosions() {
    let mut w = boxed_world(3, 2, 26);
    fill(&mut w, "stone", 1, 191, 1, 127);
    let m = w.materials().clone();
    // Carve a long sealed cavity and fill it with gas.
    fill(&mut w, "methane", 30, 160, 50, 70);
    let stone_before = count(&w, m.expect_id("stone"));
    w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(31, 60), radius: 2 });
    for _ in 0..600 {
        w.step();
    }
    let methane_left = count(&w, m.expect_id("methane"));
    let blasted = stone_before - count(&w, m.expect_id("stone"));
    assert!(methane_left < 200, "the pocket burned through ({methane_left} left)");
    assert!(blasted > 1_000, "chain of explosions tore up the rock ({blasted} cells)");
}

/// Every non-loose solid cell must be attached (edge-connected through solids)
/// to the floor row or a wall; returns the ones that aren't.
fn floating_solids(w: &World, x0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    use platypus_sim::cell::flags::LOOSE;
    let m = w.materials();
    let solid = |x: i32, y: i32| {
        w.get(CellPos::new(x, y)).is_some_and(|c| m.phys(c.material).kind == platypus_sim::Kind::Static && c.flags & LOOSE == 0)
    };
    let mut attached = std::collections::HashSet::new();
    let mut stack: Vec<(i32, i32)> = (x0..x1).map(|x| (x, 0)).filter(|&(x, y)| solid(x, y)).collect();
    while let Some((x, y)) = stack.pop() {
        if !attached.insert((x, y)) {
            continue;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= x0 && nx < x1 && ny >= 0 && ny < y1 && solid(nx, ny) && !attached.contains(&(nx, ny)) {
                stack.push((nx, ny));
            }
        }
    }
    (x0..x1).flat_map(|x| (0..y1).map(move |y| (x, y))).filter(|&(x, y)| solid(x, y) && !attached.contains(&(x, y))).collect()
}

#[test]
fn a_burning_tree_leaves_nothing_floating() {
    let mut w = boxed_world(2, 2, 30);
    // Trunk, two branches, and twigs hanging off them.
    fill(&mut w, "wood", 62, 66, 1, 70);
    fill(&mut w, "wood", 40, 62, 50, 53);
    fill(&mut w, "wood", 66, 90, 60, 63);
    for x in (42..60).step_by(4) {
        fill(&mut w, "wood", x, x + 1, 53, 58);
    }
    for x in (68..88).step_by(3) {
        fill(&mut w, "wood", x, x + 1, 55, 60);
    }
    // Single-pixel twigs touching the branches only by a corner.
    for x in (70..88).step_by(5) {
        for k in 0..6 {
            fill(&mut w, "wood", x + k, x + k + 1, 63 + k, 64 + k);
        }
    }
    w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(64, 30), radius: 3 });
    for _ in 0..4_000 {
        w.step();
    }
    let floating = floating_solids(&w, 1, 127, 127);
    assert!(floating.is_empty(), "{} solid cells left floating, e.g. {:?}", floating.len(), &floating[..floating.len().min(5)]);
}

// ---- particles --------------------------------------------------------------

fn run_until_landed(w: &mut World, max_ticks: usize) {
    for _ in 0..max_ticks {
        if w.particles().is_empty() {
            return;
        }
        w.step();
    }
    panic!("{} particles still flying after {max_ticks} ticks", w.particles().len());
}

#[test]
fn a_splash_lands_as_real_cells() {
    let mut w = boxed_world(2, 2, 40);
    let water = w.materials().expect_id("water");
    w.splash([64.0, 60.0], water, 200, 2.5);
    assert_eq!(w.particles().len(), 200);
    run_until_landed(&mut w, 400);
    assert_eq!(count(&w, water), 200, "every drop became a water cell");
}

#[test]
fn blast_debris_lands_as_loose_rubble() {
    let mut w = boxed_world(3, 2, 41);
    fill(&mut w, "stone", 1, 191, 1, 40);
    let stone = w.materials().expect_id("stone");
    w.apply_edit(&WorldEdit::Explode { center: CellPos::new(96, 39), radius: 16, power: 100 });
    assert!(w.particles().len() > 50, "the blast threw debris and sparks");
    run_until_landed(&mut w, 600);
    run_until_asleep(&mut w, 3_000);
    let loose = w.chunks().flat_map(|c| c.cells()).filter(|c| c.material == stone && c.flags & platypus_sim::cell::flags::LOOSE != 0).count();
    assert!(loose > 20, "debris piled up as loose rubble ({loose})");
    assert!(floating_solids(&w, 1, 191, 127).is_empty(), "nothing floating");
}

#[test]
fn particles_leaving_the_loaded_world_disappear() {
    let mut w = boxed_world(1, 1, 42);
    let sand = w.materials().expect_id("sand");
    let cell = w.materials().spawn(sand, &mut Rng::seeded(&[1]));
    // Straight up and out of the only loaded chunk.
    w.emit(Particle::new([32.0, 60.0], [0.0, 7.0], cell, 500, Landing::Settle));
    run_until_landed(&mut w, 100);
    assert_eq!(count(&w, sand), 0);
}

#[test]
fn embers_set_fire_where_they_land() {
    let mut w = boxed_world(1, 1, 43);
    fill(&mut w, "wood", 20, 40, 1, 5);
    let wood = w.materials().expect_id("wood");
    let ember = Cell::new(wood, 0);
    w.emit(Particle::new([30.5, 30.0], [0.0, 0.0], ember, 200, Landing::Ember));
    run_until_landed(&mut w, 200);
    assert!(burning(&w) > 0, "the ember lit the wood");
}

#[test]
fn mining_dust_is_only_visual() {
    let mut w = boxed_world(1, 1, 44);
    fill(&mut w, "dirt", 20, 44, 1, 20);
    let dirt = w.materials().expect_id("dirt");
    let before = count(&w, dirt) as u32;
    let mut removed = 0;
    for _ in 0..200 {
        let r = w.apply_edit(&WorldEdit::Mine { center: CellPos::new(32, 12), radius: 5, power: 6, max_hardness: 200 });
        removed += r.removed.iter().map(|&(_, n)| n).sum::<u32>();
        w.step();
    }
    run_until_landed(&mut w, 200);
    assert!(removed > 50);
    assert_eq!(count(&w, dirt) as u32, before - removed, "dust never became dirt");
}
