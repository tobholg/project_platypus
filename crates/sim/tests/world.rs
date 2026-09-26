//! Behavioural tests of the cell world, using the real `materials.ron`.

use std::sync::Arc;

use platypus_sim::rng::Rng;
use platypus_sim::{CHUNK, CHUNK_AREA, Cell, CellPos, Chunk, ChunkPos, Landing, MaterialId, MaterialTable, Particle, World, WorldEdit, store};

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
    assert!(hi - lo <= 2, "water not level: {lo}..{hi}: {heights:?}");
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

/// Wood left of a 40×5 slab lit at one end with a fire of `radius`, per seed.
fn slab_fire(radius: i32) -> Vec<usize> {
    let mut left: Vec<usize> = (100..120)
        .map(|seed| {
            let mut w = boxed_world(1, 1, seed);
            let m = w.materials().clone();
            let wood = m.expect_id("wood");
            for x in 10..50 {
                for y in 1..6 {
                    w.set(CellPos::new(x, y), Cell::new(wood, 0));
                }
            }
            w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(12, 3), radius });
            // Wood fire creeps; give it time to finish.
            for _ in 0..12_000 {
                w.step();
            }
            assert_eq!(burning(&w), 0, "seed {seed}: burned out");
            assert_eq!(count(&w, m.expect_id("fire")), 0, "seed {seed}: flames gone");
            count(&w, wood)
        })
        .collect();
    left.sort();
    left
}

/// Wood is hard to get going (SPEC §3.8): a spark sometimes fizzles, often
/// only scorches; a proper fire usually burns most of it. Every fire ends.
#[test]
fn a_spark_on_wood_often_fizzles_a_proper_fire_takes() {
    let spark = slab_fire(1);
    let fire = slab_fire(3);
    assert!(spark.iter().filter(|&&l| l > 150).count() >= 3, "sparks sometimes fizzle: {spark:?} of 200 left");
    assert!(spark.iter().filter(|&&l| l < 60).count() >= 3, "and sometimes take: {spark:?}");
    assert!(fire[fire.len() / 2] < 110, "a proper fire burns most of it: {fire:?}");
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
    // A bonfire's worth of flames on top of it.
    w.apply_edit(&WorldEdit::Paint { center: CellPos::new(14, 7), radius: 4, material: m.expect_id("fire"), overwrite: true });
    for _ in 0..6_000 {
        w.step();
    }
    assert!(count(&w, wood) < 150, "the flames lit the wood, {} left", count(&w, wood));
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
            plant_tree(&mut w, 200);
            w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(200, 10), radius: 2 });
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
        w.apply_edit(&WorldEdit::Mine { center: at, radius: 0, power, max_hardness: 200, back: false });
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
    w.apply_edit(&WorldEdit::Mine { center: at, radius: 0, power: 10, max_hardness: 200, back: false });
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
    // Well past the thresholds: a second or so.
    for _ in 0..60 {
        w.step();
    }
    assert!(count(&w, m.expect_id("steam")) > 10, "boiled");
    assert!(count(&w, m.expect_id("ice")) > 10, "froze");
    // In a 15 °C world the ice melts again.
    // Ice in a 15 °C room melts over a minute or so.
    for _ in 0..12_000 {
        w.step();
    }
    assert_eq!(count(&w, m.expect_id("ice")), 0, "ice melted back");
}

#[test]
fn blood_boils_and_freezes_like_water() {
    let mut w = boxed_world(1, 1, 25);
    let m = w.materials().clone();
    fill(&mut w, "blood", 10, 20, 1, 6);
    fill(&mut w, "blood", 40, 50, 1, 6);
    w.apply_edit(&WorldEdit::Heat { center: CellPos::new(15, 3), radius: 6, amount: 300 });
    w.apply_edit(&WorldEdit::Heat { center: CellPos::new(45, 3), radius: 6, amount: -120 });
    // Well past the thresholds: a second or so.
    for _ in 0..60 {
        w.step();
    }
    assert!(count(&w, m.expect_id("blood_steam")) > 10, "boiled");
    assert!(count(&w, m.expect_id("frozen_blood")) > 10, "froze");
    // Ice in a 15 °C room melts over a minute or so.
    for _ in 0..12_000 {
        w.step();
    }
    assert_eq!(count(&w, m.expect_id("frozen_blood")), 0, "thawed");
    assert_eq!(count(&w, m.expect_id("blood_steam")), 0, "the haze settled");
    assert!(count(&w, m.expect_id("blood")) > 40, "most came back down as blood");
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

/// The climate varies across the world too (biomes): the same water freezes
/// in a cold column and stays water in a mild one.
#[test]
fn water_freezes_in_a_cold_column_only() {
    let mut w = boxed_world(2, 1, 26);
    let mut climate = platypus_sim::Climate { surface_temp: 5, column_bits: 6, ..Default::default() };
    climate.columns[1] = -15; // x 64..127: -10 °C
    w.set_climate(climate);
    fill(&mut w, "stone", 1, 127, 1, 4);
    for x in [19, 44, 83, 108] {
        fill(&mut w, "stone", x, x + 1, 4, 12); // pool walls
    }
    fill(&mut w, "water", 20, 44, 4, 8);
    fill(&mut w, "water", 84, 108, 4, 8);
    for _ in 0..12_000 {
        w.step();
    }
    let ice = w.materials().expect_id("ice");
    let frozen = |x0: i32| (x0..x0 + 24).flat_map(|x| (4..8).map(move |y| (x, y))).filter(|&(x, y)| w.get(CellPos::new(x, y)).unwrap().material == ice).count();
    assert_eq!(frozen(20), 0, "the mild column stays water");
    assert!(frozen(84) > 60, "the cold column froze ({} of 96)", frozen(84));
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
        // Snow at 15 °C melts over a few seconds (latent heat); give it a minute.
        for _ in 0..3_600 {
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
    let mut reported = 0;
    for _ in 0..600 {
        reported += w.step().detonated.len();
    }
    let methane_left = count(&w, m.expect_id("methane"));
    let blasted = stone_before - count(&w, m.expect_id("stone"));
    assert!(methane_left < 200, "the pocket burned through ({methane_left} left)");
    assert!(blasted > 1_000, "chain of explosions tore up the rock ({blasted} cells)");
    // The game shakes the camera for each one.
    assert!(reported >= 3, "detonations are reported ({reported})");
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
    // Each has a chance (and some go out on the way): a shower lights it.
    for i in 0..20 {
        w.emit(Particle::new([21.5 + i as f32, 12.0], [0.0, 0.0], ember, 200, Landing::Ember));
    }
    run_until_landed(&mut w, 200);
    assert!(burning(&w) > 0, "the embers lit the wood");
}

#[test]
fn mining_dust_is_only_visual() {
    let mut w = boxed_world(1, 1, 44);
    fill(&mut w, "dirt", 20, 44, 1, 20);
    let dirt = w.materials().expect_id("dirt");
    let before = count(&w, dirt) as u32;
    let mut removed = 0;
    for _ in 0..200 {
        let r = w.apply_edit(&WorldEdit::Mine { center: CellPos::new(32, 12), radius: 5, power: 6, max_hardness: 200, back: false });
        removed += r.removed.iter().map(|&(_, n)| n).sum::<u32>();
        w.step();
    }
    run_until_landed(&mut w, 200);
    assert!(removed > 50);
    assert_eq!(count(&w, dirt) as u32, before - removed, "dust never became dirt");
}

// ---- background layer, plants, wind -----------------------------------------

fn fill_bg(w: &mut World, name: &str, x0: i32, x1: i32, y0: i32, y1: i32) {
    let id = w.materials().expect_id(name);
    let mut rng = Rng::seeded(&[x0 as u64, y0 as u64, 7]);
    for x in x0..x1 {
        for y in y0..y1 {
            let c = w.materials().spawn(id, &mut rng);
            w.set_bg(CellPos::new(x, y), c);
        }
    }
}

fn count_bg(world: &World, id: MaterialId) -> usize {
    world.chunks().flat_map(|c| c.background()).filter(|c| c.material == id).count()
}

/// Background cells not connected (by edges, through background) to one that
/// rests against solid playfield.
fn floating_background(w: &World) -> Vec<CellPos> {
    let m = w.materials();
    let solid_front = |p: CellPos| {
        w.get(p).is_some_and(|c| matches!(m.phys(c.material).kind, platypus_sim::Kind::Static | platypus_sim::Kind::Powder) && c.flags & platypus_sim::cell::flags::LOOSE == 0)
    };
    let all: Vec<CellPos> = w
        .chunks()
        .flat_map(|c| (0..CHUNK_AREA).filter(move |&i| !c.background()[i].is_air()).map(move |i| c.pos.origin().offset((i % 64) as i32, (i / 64) as i32)))
        .collect();
    let set: std::collections::HashSet<CellPos> = all.iter().copied().collect();
    let mut held: std::collections::HashSet<CellPos> = std::collections::HashSet::new();
    let mut stack: Vec<CellPos> = all.iter().copied().filter(|&p| solid_front(p)).collect();
    while let Some(p) = stack.pop() {
        if !held.insert(p) {
            continue;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let q = p.offset(dx, dy);
            if set.contains(&q) && !held.contains(&q) {
                stack.push(q);
            }
        }
    }
    all.into_iter().filter(|p| !held.contains(p)).collect()
}

/// A background tree: trunk rooted behind the stone floor, a crown of leaves.
fn plant_tree(w: &mut World, x: i32) {
    fill_bg(w, "wood", x - 2, x + 2, 0, 60);
    fill_bg(w, "wood", x - 14, x + 14, 44, 46);
    fill_bg(w, "leaves", x - 16, x + 16, 46, 58);
}

#[test]
fn a_background_tree_stands_on_its_own() {
    let mut w = boxed_world(2, 2, 50);
    plant_tree(&mut w, 64);
    let wood = w.materials().expect_id("wood");
    let before = count_bg(&w, wood);
    run_until_asleep(&mut w, 500);
    assert_eq!(count_bg(&w, wood), before, "nothing fell");
}

/// Step until no body is flying; returns ticks taken.
fn run_until_bodies_settle(w: &mut World, max_ticks: usize) -> usize {
    for t in 0..max_ticks {
        if w.bodies().is_empty() {
            return t;
        }
        w.step();
    }
    panic!("{} bodies still flying after {max_ticks} ticks", w.bodies().len());
}

/// Playfield cells of `id`: (count, lowest y, highest y, leftmost x, rightmost x).
fn extent(w: &World, id: MaterialId) -> (usize, i32, i32, i32, i32) {
    let mut e = (0, i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for c in w.chunks() {
        for (i, cell) in c.cells().iter().enumerate() {
            if cell.material == id {
                let p = c.pos.origin().offset((i % 64) as i32, (i / 64) as i32);
                e = (e.0 + 1, e.1.min(p.y), e.2.max(p.y), e.3.min(p.x), e.4.max(p.x));
            }
        }
    }
    e
}

#[test]
fn a_felled_tree_topples_as_one_piece_and_lies_on_the_ground_as_a_log() {
    let mut w = boxed_world(3, 2, 51);
    fill(&mut w, "stone", 1, 191, 1, 10);
    // Trunk rooted behind the stone floor, a crown. (With a crossbar branch
    // it can land propped up on the branch, like a fallen T.)
    let tree = |w: &mut World| {
        fill_bg(w, "wood", 94, 99, 1, 70);
        fill_bg(w, "leaves", 80, 113, 60, 76);
    };
    tree(&mut w);
    let (wood, leaves) = (w.materials().expect_id("wood"), w.materials().expect_id("leaves"));
    let bg_wood = count_bg(&w, wood);
    // Cut through the trunk just above the ground.
    w.apply_edit(&WorldEdit::Dig { center: CellPos::new(96, 14), radius: 4, max_hardness: 200 });
    assert_eq!(w.bodies().len(), 1, "the tree came away whole");
    assert_eq!(count_bg(&w, leaves), 0, "crown and all");
    let ticks = run_until_bodies_settle(&mut w, 900);
    assert!(ticks > 30, "it takes a while to fall ({ticks} ticks)");
    run_until_landed(&mut w, 600);

    let (logs, lo, hi, left, right) = extent(&w, wood);
    assert!(logs > (bg_wood - 50) * 6 / 10, "most of the wood lies in the playfield ({logs} of ~{bg_wood})");
    assert!(lo >= 10, "on the ground, not in it (lowest {lo})");
    assert!(hi - lo < 12, "lying down: {}x{} ", right - left, hi - lo);
    assert!(right - left > 35, "the trunk is still long ({})", right - left);
    let hanging = floating_solids(&w, 1, 191, 127);
    assert!(hanging.len() < 20, "{} wood cells hanging in the air", hanging.len());
    assert!(w.get_bg(CellPos::new(96, 5)).unwrap().material == wood, "the stump stays");
}

#[test]
fn a_felled_tree_is_not_held_up_by_its_neighbours_crown() {
    let mut w = boxed_world(3, 2, 57);
    fill(&mut w, "stone", 1, 191, 1, 10);
    let (wood, leaves) = (w.materials().expect_id("wood"), w.materials().expect_id("leaves"));
    // Two trees whose crowns overlap.
    fill_bg(&mut w, "wood", 60, 65, 1, 70);
    fill_bg(&mut w, "wood", 110, 115, 1, 70);
    fill_bg(&mut w, "leaves", 40, 136, 60, 80);
    fill_bg(&mut w, "wood", 60, 65, 60, 70);
    fill_bg(&mut w, "wood", 110, 115, 60, 70);
    let crown = count_bg(&w, leaves);
    w.apply_edit(&WorldEdit::Dig { center: CellPos::new(62, 14), radius: 4, max_hardness: 200 });
    assert_eq!(w.bodies().len(), 1, "the cut tree falls");
    run_until_bodies_settle(&mut w, 900);
    run_until_landed(&mut w, 600);
    run_until_asleep(&mut w, 2_000);
    assert!(w.get_bg(CellPos::new(112, 65)).unwrap().material == wood, "the other tree still stands");
    let kept = count_bg(&w, leaves);
    assert!(kept > crown / 5 && kept < crown * 4 / 5, "the crown was shared out ({kept} of {crown})");
    let hanging = floating_background(&w);
    assert!(hanging.is_empty(), "{} background cells hanging", hanging.len());
}

#[test]
fn felling_is_deterministic() {
    let run = || {
        let mut w = boxed_world(3, 2, 55);
        fill(&mut w, "stone", 1, 191, 1, 10);
        fill_bg(&mut w, "wood", 90, 95, 1, 80);
        fill_bg(&mut w, "wood", 95, 120, 60, 62);
        w.apply_edit(&WorldEdit::Dig { center: CellPos::new(92, 14), radius: 4, max_hardness: 200 });
        let mut trace = Vec::new();
        for _ in 0..300 {
            w.step();
            trace.extend(w.bodies().iter().map(|b| (b.pos[0].to_bits(), b.pos[1].to_bits(), b.rot[0].to_bits())));
        }
        trace
    };
    let a = run();
    assert!(!a.is_empty(), "something fell");
    assert_eq!(a, run());
}

#[test]
fn a_small_branch_still_drops_as_rubble() {
    // Also guards the ground check: the trunk above the cut once came away
    // as a body because cells proven grounded were skipped as "seen".
    let mut w = boxed_world(2, 2, 56);
    fill(&mut w, "stone", 1, 127, 1, 10);
    fill_bg(&mut w, "wood", 60, 64, 1, 60);
    fill_bg(&mut w, "wood", 64, 76, 40, 42); // 24 cells
    w.apply_edit(&WorldEdit::Dig { center: CellPos::new(65, 41), radius: 1, max_hardness: 200 });
    assert!(w.bodies().is_empty());
    run_until_landed(&mut w, 400);
    assert!(count(&w, w.materials().expect_id("wood")) > 10, "it fell as rubble");
}

#[test]
fn a_forest_fire_burns_through_the_background_and_leaves_nothing_hanging() {
    let mut w = boxed_world(3, 2, 52);
    // Crowns touching: fire spreads through the canopy (embers alone
    // seldom carry it across a gap).
    plant_tree(&mut w, 40);
    plant_tree(&mut w, 72);
    plant_tree(&mut w, 104);
    let leaves = w.materials().expect_id("leaves");
    // A crown fire (a spark on a trunk may just fizzle).
    w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(40, 52), radius: 3 });
    for _ in 0..8_000 {
        w.step();
    }
    assert!(count_bg(&w, leaves) < 150, "the canopy burned ({} leaves left)", count_bg(&w, leaves));
    let hanging = floating_background(&w);
    assert!(hanging.is_empty(), "{} background cells left hanging, e.g. {:?}", hanging.len(), &hanging[..hanging.len().min(5)]);
    assert_eq!(burning(&w), 0, "the fire burned out");
}

#[test]
fn tall_grass_is_crushed_by_sand_and_withers_without_ground() {
    let mut w = boxed_world(1, 1, 53);
    let m = w.materials().clone();
    let (grass, sand) = (m.expect_id("tall_grass"), m.expect_id("sand"));
    fill(&mut w, "dirt", 1, 63, 1, 10);
    fill(&mut w, "tall_grass", 10, 50, 10, 16);
    run_until_asleep(&mut w, 200);
    let start = count(&w, grass);
    assert_eq!(start, 40 * 6, "grass stands on dirt");
    // Dig out the ground under the left part: those blades wither.
    w.apply_edit(&WorldEdit::Dig { center: CellPos::new(15, 5), radius: 5, max_hardness: 100 });
    // Pour sand on the right part: it crushes the grass it lands on.
    w.apply_edit(&WorldEdit::Paint { center: CellPos::new(42, 30), radius: 5, material: sand, overwrite: false });
    run_until_asleep(&mut w, 3_000);
    // Columns fully undercut by the dig (its round edge leaves dirt at x=10 and 19).
    let left = (12..18).flat_map(|x| (1..30).map(move |y| (x, y))).filter(|&(x, y)| w.get(CellPos::new(x, y)).unwrap().material == grass).count();
    assert_eq!(left, 0, "unsupported grass withered");
    assert!(count(&w, grass) < start - 60, "sand crushed grass");
}

#[test]
fn wind_is_smooth_bounded_and_seeded() {
    let m = mats();
    let mut a = World::new(1, m.clone());
    let b = World::new(1, m.clone());
    let c = World::new(2, m);
    assert_eq!(a.wind(), b.wind());
    assert_ne!(a.wind(), c.wind());
    let mut last = a.wind();
    let (mut lo, mut hi) = (1.0f32, -1.0f32);
    for _ in 0..20_000 {
        a.step();
        let w = a.wind();
        assert!((-1.0..=1.0).contains(&w));
        assert!((w - last).abs() < 0.02, "wind jumps");
        lo = lo.min(w);
        hi = hi.max(w);
        last = w;
    }
    assert!(hi - lo > 0.3, "wind varies ({lo}..{hi})");
}

#[test]
fn store_roundtrips_the_background() {
    let mut w = boxed_world(1, 1, 54);
    plant_tree(&mut w, 32);
    let chunk = w.chunk(ChunkPos::new(0, 0)).unwrap();
    let back = store::decode(chunk.pos, &store::encode(chunk)).unwrap();
    assert_eq!(store::checksum(&back), store::checksum(chunk));
    assert_eq!(back.background(), chunk.background());
}

/// A meadow like the generated ones: dirt, a grass surface, tall grass
/// blades of varied height with bare gaps. Returns (world, grass cells).
fn meadow(seed: u64) -> (World, usize) {
    let mut w = boxed_world(3, 1, seed);
    fill(&mut w, "dirt", 1, 191, 1, 8);
    fill(&mut w, "grass", 1, 191, 8, 9);
    let tall = w.materials().expect_id("tall_grass");
    let mut rng = Rng::seeded(&[seed, 0x3EAD]);
    for x in 1..191 {
        let h = [0, 2, 3, 4, 5, 6, 7, 8][(rng.next_u32() % 8) as usize];
        for y in 9..9 + h {
            let c = w.materials().spawn(tall, &mut rng);
            w.set(CellPos::new(x, y), c);
        }
    }
    let n = count(&w, tall) + count(&w, w.materials().expect_id("grass"));
    (w, n)
}

/// How much of a meadow one spark burns, across seeds (sorted fractions).
fn meadow_burn_fractions(seeds: std::ops::Range<u64>) -> Vec<f32> {
    let mut out: Vec<f32> = seeds
        .map(|seed| {
            let (mut w, n) = meadow(seed);
            let (tall, grass) = (w.materials().expect_id("tall_grass"), w.materials().expect_id("grass"));
            w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(96, 10), radius: 1 });
            for _ in 0..6_000 {
                w.step();
            }
            let left = count(&w, tall) + count(&w, grass);
            1.0 - left as f32 / n as f32
        })
        .collect();
    out.sort_by(|a, b| a.total_cmp(b));
    out
}

/// Fire spread is tuned near the tipping point: one spark in a meadow sometimes
/// fizzles, usually burns a patch, rarely takes everything (SPEC §3.8).
#[test]
fn a_spark_in_a_meadow_burns_a_patch_not_the_world() {
    // 40 sparks: with 20, whether one fizzles was down to luck.
    let f = meadow_burn_fractions(300..340);
    let median = f[f.len() / 2];
    assert!(f[0] < 0.25, "some sparks fizzle: {f:?}");
    assert!((0.3..=0.75).contains(&median), "a typical spark burns a patch: median {median}");
    assert!(f.iter().filter(|&&v| v > 0.95).count() <= 3, "a spark rarely takes everything: {f:?}");
}

/// A tree catches over seconds, not instantly, and burns for a while.
#[test]
fn a_tree_fire_climbs_then_burns_for_a_while() {
    let mut w = boxed_world(2, 2, 60);
    plant_tree(&mut w, 64);
    w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(64, 10), radius: 2 });
    let bg_burning = |w: &World| w.chunks().flat_map(|c| c.background()).filter(|b| b.flags & platypus_sim::cell::flags::BURNING != 0).count();
    for _ in 0..60 {
        w.step();
    }
    assert!(bg_burning(&w) < 300, "one second in, the fire is still climbing ({} burning)", bg_burning(&w));
    // The standing tree burns for a while; what falls burns on the ground
    // (a log pile) a good while longer, and then it's all out.
    let mut secs = 1;
    let mut tree_secs = None;
    while (bg_burning(&w) > 0 || burning(&w) > 0) && secs < 200 {
        for _ in 0..60 {
            w.step();
        }
        secs += 1;
        if bg_burning(&w) == 0 {
            tree_secs.get_or_insert(secs);
        }
    }
    let tree_secs = tree_secs.expect("the tree stopped burning");
    assert!((8..=40).contains(&tree_secs), "the tree burned for {tree_secs} s");
    assert!(secs < 200, "the fallen wood burned out ({secs} s)");
}

#[test]
fn a_blast_flings_loose_sand_beyond_the_crater() {
    let mut w = boxed_world(3, 2, 27);
    fill(&mut w, "sand", 1, 191, 1, 60);
    let m = w.materials().clone();
    let sand = m.expect_id("sand");
    let before = count(&w, sand);
    w.apply_edit(&WorldEdit::Explode { center: CellPos::new(96, 59), radius: 16, power: 100 });
    let in_flight = w.particles().len();
    // Past the crater and its shattered rim, some sand is gone into the air.
    let rim_sand = (0..128).flat_map(|x| (1..60).map(move |y| CellPos::new(x + 32, y)))
        .filter(|p| { let d = (((p.x - 96).pow(2) + (p.y - 59).pow(2)) as f32).sqrt(); d > 20.0 && d < 23.0 })
        .filter(|&p| w.get(p).is_some_and(|c| c.material == sand))
        .count();
    let rim_total = (0..128).flat_map(|x| (1..60).map(move |y| CellPos::new(x + 32, y)))
        .filter(|p| { let d = (((p.x - 96).pow(2) + (p.y - 59).pow(2)) as f32).sqrt(); d > 20.0 && d < 23.0 })
        .count();
    assert!(in_flight > 300, "debris and flung sand fly ({in_flight})");
    assert!(rim_sand < rim_total * 3 / 4, "the shockwave thinned the rim ({rim_sand}/{rim_total})");
    run_until_landed(&mut w, 1200);
    assert!(count(&w, sand) > before * 7 / 10, "flung sand lands again, it isn't deleted");
}


#[test]
fn a_blast_spares_stone_and_earth_walls_but_not_wood() {
    // A dug-out tunnel: stone playfield with a stone and dirt back wall, and
    // a plank wall in one stretch of it.
    let mut w = boxed_world(3, 2, 28);
    fill(&mut w, "stone", 1, 191, 1, 100);
    fill_bg(&mut w, "stone", 1, 96, 1, 100);
    fill_bg(&mut w, "dirt", 96, 191, 1, 100);
    fill_bg(&mut w, "planks", 88, 104, 40, 60);
    let m = w.materials().clone();
    let (stone, dirt, planks) = (m.expect_id("stone"), m.expect_id("dirt"), m.expect_id("planks"));
    let (s0, d0, p0) = (count_bg(&w, stone), count_bg(&w, dirt), count_bg(&w, planks));
    w.apply_edit(&WorldEdit::Explode { center: CellPos::new(96, 50), radius: 16, power: 100 });
    assert!(count(&w, stone) < 190 * 99 - 400, "the blast dug a crater");
    assert_eq!((count_bg(&w, stone), count_bg(&w, dirt)), (s0, d0), "the back wall still stands");
    assert!(count_bg(&w, planks) < p0 / 4, "a wooden wall is blown away ({} of {p0} left)", count_bg(&w, planks));
}


// ---- charring ----------------------------------------------------------------

/// A background tree in a box, lit at the base. Returns (tick the first
/// piece broke off, wood in it).
fn base_fire(materials: Arc<MaterialTable>) -> (Option<usize>, usize) {
    let stone = Cell::new(materials.expect_id("stone"), 0);
    let mut w = World::new(61, materials.clone());
    for cy in 0..2 {
        for cx in 0..3 {
            w.insert_chunk(Chunk::filled(ChunkPos::new(cx, cy), Cell::AIR));
        }
    }
    for x in 0..192 {
        for y in 0..10 {
            w.set(CellPos::new(x, y), stone);
        }
    }
    fill_bg(&mut w, "wood", 90, 100, 1, 90);
    fill_bg(&mut w, "leaves", 76, 114, 76, 100);
    let wood = materials.expect_id("wood");
    // A fire right round the base (a spark on a trunk may just fizzle).
    w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(95, 12), radius: 6 });
    for t in 0..6_000 {
        w.step();
        if let Some(b) = w.bodies().first() {
            return (Some(t), b.world_cells().filter(|(_, c)| c.material == wood).count());
        }
    }
    (None, 0)
}

#[test]
fn a_tree_burning_at_the_base_snaps_before_it_burns_through() {
    let charring = base_fire(mats());
    // The same tree if charred wood held until it was gone.
    let tough = MATERIALS.replace("chars_into: \"charcoal\",", "chars_into: \"charcoal\", chars_at: 1.0,");
    assert_ne!(tough, MATERIALS);
    let holding = base_fire(Arc::new(MaterialTable::from_ron(&tough).unwrap()));
    let (Some(t_char), Some(t_hold)) = (charring.0, holding.0) else {
        panic!("both trees fell: charring {charring:?}, holding {holding:?}");
    };
    assert!(t_char < t_hold * 2 / 3, "charred wood gives way sooner: {t_char} vs {t_hold} ticks");
    // (The crown has burned off either way: flames race up the trunk.)
    assert!(charring.1 > 400, "the trunk comes down in one big piece ({} wood of ~780)", charring.1);
}

#[test]
fn burnt_wood_leaves_charcoal_and_doused_charred_wood_is_charcoal() {
    let mut w = boxed_world(2, 1, 62);
    let m = w.materials().clone();
    let (charcoal, wood) = (m.expect_id("charcoal"), m.expect_id("wood"));
    // Burn a block of wood out completely.
    fill(&mut w, "wood", 20, 40, 1, 12);
    w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(30, 6), radius: 12 });
    for _ in 0..4_000 {
        w.step();
    }
    assert_eq!(count(&w, wood), 0, "it burned");
    let left = count(&w, charcoal);
    assert!(left > 0, "some charcoal is left");

    // Charred wood that meets water stops burning as charcoal.
    let mut w = boxed_world(2, 1, 63);
    for x in 60..70 {
        let mut c = Cell::new(wood, 0);
        c.flags |= platypus_sim::cell::flags::BURNING;
        c.life = 10; // well into its burn
        assert!(m.is_charred(c));
        w.set(CellPos::new(x, 1), c);
    }
    fill(&mut w, "water", 60, 70, 2, 6);
    for _ in 0..4 {
        w.step();
    }
    assert!(count(&w, charcoal) >= 8, "put out as charcoal ({})", count(&w, charcoal));
}

// ---- acid and exposure ---------------------------------------------------------

/// Boil a pool of acid (under a stone roof at `roof`, if any); returns the
/// world, the most fumes at once, and the most acid seen after it boiled away.
fn boil_acid(roof: Option<i32>, seed: u64) -> (World, usize, usize) {
    let mut w = boxed_world(2, 2, seed);
    let m = w.materials().clone();
    let (acid, fumes) = (m.expect_id("acid"), m.expect_id("acid_fumes"));
    if let Some(y) = roof {
        fill(&mut w, "stone", 1, 127, y, y + 10);
    }
    // A glass floor, which acid doesn't eat, so the rain collects.
    fill(&mut w, "glass", 1, 127, 1, 3);
    fill(&mut w, "acid", 50, 70, 3, 7);
    w.apply_edit(&WorldEdit::Heat { center: CellPos::new(60, 5), radius: 12, amount: 300 });
    let mut most_fumes = 0;
    for _ in 0..300 {
        w.step();
        most_fumes = most_fumes.max(count(&w, fumes));
    }
    assert!(count(&w, acid) < 10, "the pool boiled away");
    let mut rained = 0;
    for _ in 0..3_000 {
        w.step();
        rained = rained.max(count(&w, acid));
    }
    (w, most_fumes, rained)
}

#[test]
fn acid_fumes_trapped_under_rock_eat_into_it() {
    let (w, most_fumes, _) = boil_acid(Some(40), 70);
    assert!(most_fumes > 30, "it boiled into fumes ({most_fumes})");
    let stone = w.materials().expect_id("stone");
    let roof_left = (1..127).flat_map(|x| (40..50).map(move |y| CellPos::new(x, y))).filter(|&p| w.get(p).unwrap().material == stone).count();
    assert!(roof_left < 126 * 10 - 20, "the fumes ate into the roof ({} of {} left)", roof_left, 126 * 10);
}

#[test]
fn acid_fumes_in_the_open_rain_back_down_as_acid() {
    let (_, most_fumes, rained) = boil_acid(None, 73);
    assert!(most_fumes > 30, "it boiled into fumes ({most_fumes})");
    assert!(rained > 5, "and came back down as acid ({rained})");
}

#[test]
fn a_spark_flashes_acid_fumes_into_fire() {
    let mut w = boxed_world(1, 1, 71);
    let m = w.materials().clone();
    let fumes = m.expect_id("acid_fumes");
    fill(&mut w, "stone", 1, 63, 50, 60);
    fill(&mut w, "acid_fumes", 10, 50, 40, 50);
    let before = count(&w, fumes);
    w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(12, 44), radius: 2 });
    for _ in 0..120 {
        w.step();
    }
    assert!(count(&w, fumes) < before / 4, "the cloud burned ({} of {before} left)", count(&w, fumes));
}

#[test]
fn exposure_reads_heat_corrosion_fire_and_water_from_the_cells() {
    let mut w = boxed_world(1, 1, 72);
    let box_at = |x: i32| (CellPos::new(x, 1), CellPos::new(x + 3, 6));
    fill(&mut w, "lava", 5, 9, 1, 7);
    fill(&mut w, "acid", 15, 19, 1, 7);
    fill(&mut w, "fire", 25, 29, 1, 7);
    fill(&mut w, "water", 35, 39, 1, 7);
    let (lo, hi) = box_at(5);
    let lava = w.exposure(lo, hi);
    assert!(lava.heat > 80.0, "lava burns: {lava:?}");
    let (lo, hi) = box_at(15);
    let acid = w.exposure(lo, hi);
    assert_eq!(acid.corrosion, 30.0, "{acid:?}");
    assert!(acid.heat == 0.0 && !acid.ignites, "{acid:?}");
    let (lo, hi) = box_at(25);
    assert!(w.exposure(lo, hi).ignites, "flames set you alight");
    let (lo, hi) = box_at(35);
    let water = w.exposure(lo, hi);
    assert_eq!(water.coat, Some(w.materials().expect_id("water")), "{water:?}");
    assert!(water.submerged > 0.8 && water.heat == 0.0 && water.corrosion == 0.0, "{water:?}");
    let (lo, hi) = box_at(45);
    assert_eq!(w.exposure(lo, hi), platypus_sim::Exposure::default(), "air is harmless");
}

#[test]
fn an_oil_slick_burns_on_the_water_it_floats_on() {
    let mut w = boxed_world(2, 1, 80);
    fill(&mut w, "water", 1, 127, 1, 12);
    fill(&mut w, "oil", 20, 100, 12, 15);
    run_until_asleep(&mut w, 2_000);
    let oil = w.materials().expect_id("oil");
    let before = count(&w, oil);
    w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(22, 13), radius: 2 });
    for _ in 0..900 {
        w.step();
    }
    assert!(count(&w, oil) < before / 10, "the slick burned away ({} of {before} left)", count(&w, oil));
    // The fire boils the top of the water; the steam comes back down later.
    let wet = count(&w, w.materials().expect_id("water")) + count(&w, w.materials().expect_id("steam"));
    assert!(wet > 1100, "the water under it is still there, some as steam ({wet} of 1386)");
}

#[test]
fn a_freezing_floor_chills_and_a_hot_one_burns_what_stands_on_it() {
    let mut w = boxed_world(1, 1, 81);
    fill(&mut w, "stone", 1, 63, 1, 5);
    // A body standing on the floor: its box starts just above it.
    let (lo, hi) = (CellPos::new(20, 5), CellPos::new(23, 12));
    assert_eq!(w.exposure(lo, hi), platypus_sim::Exposure::default(), "ordinary ground does nothing");
    w.apply_edit(&WorldEdit::Heat { center: CellPos::new(21, 3), radius: 4, amount: -150 });
    let cold = w.exposure(lo, hi);
    assert!(cold.cold > 0.9 && cold.heat > 5.0, "frozen stone chills and bites: {cold:?}");
    w.apply_edit(&WorldEdit::Heat { center: CellPos::new(21, 3), radius: 4, amount: 800 });
    let hot = w.exposure(lo, hi);
    assert!(hot.heat > 30.0 && hot.cold == 0.0, "glowing stone burns feet: {hot:?}");
}

// ---- weather -----------------------------------------------------------------

/// Three background trees under a band of sky, the first set alight; `rain`
/// feeds the clouds over them. Returns (leaves, water) after 20 s.
fn forest_fire_under_sky(rain: bool) -> (usize, usize) {
    let mut w = boxed_world(3, 2, 90);
    w.set_weather(platypus_sim::Weather::new(90, 192, 96, 28));
    plant_tree(&mut w, 40);
    plant_tree(&mut w, 100);
    plant_tree(&mut w, 150);
    w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(40, 50), radius: 6 });
    for _ in 0..1_200 {
        if rain {
            for x in (4..188).step_by(4) {
                w.weather_mut().unwrap().feed(x, 0.4);
            }
        }
        w.step();
    }
    (count_bg(&w, w.materials().expect_id("leaves")), count(&w, w.materials().expect_id("water")))
}

/// Rain douses what it falls through (flames, burning canopies) and what it
/// lands on. A big burning block only loses its surface fire: rain doesn't
/// soak in.
#[test]
fn rain_stops_a_forest_fire_and_puddles() {
    let (dry, _) = forest_fire_under_sky(false);
    let (wet, puddles) = forest_fire_under_sky(true);
    // Three crowns of 32×12 leaves; dry, the first one burns down.
    assert!(dry < 1_152 - 300, "without rain the fire takes a crown ({dry} left)");
    // (Measured 1033: the hottest leaves boil the first drops off.)
    assert!(wet > 1_152 * 85 / 100, "rain saved the canopy ({wet} of 1152 left)");
    assert!(puddles > 0, "and left water on the ground ({puddles})");
}


#[test]
fn lightning_strikes_the_first_thing_in_its_way_and_sets_it_alight() {
    let mut w = boxed_world(3, 2, 91);
    w.set_weather(platypus_sim::Weather::new(91, 192, 100, 24));
    plant_tree(&mut w, 100);
    // Straight down onto the tree (its trunk tops out at 59), not the ground under it.
    w.apply_edit(&WorldEdit::Lightning { x: 100, from_y: 120 });
    let s = w.step().lightning;
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].hit, CellPos::new(100, 59), "hit the top of the tree");
    assert!(s[0].top >= 100, "from the cloud base");
    assert_eq!(s[0].earth, CellPos::new(100, 0), "and ran down the trunk to the ground");
    let burning = |w: &World| w.chunks().flat_map(|c| c.background()).filter(|b| b.flags & platypus_sim::cell::flags::BURNING != 0).count();
    // The whole length of the trunk went up at once, not a spot in the
    // crown; the top of it was blown off.
    let alight = |y: i32| w.get_bg(CellPos::new(100, y)).is_some_and(|b| b.flags & platypus_sim::cell::flags::BURNING != 0);
    let trunk_alight = (1..60).filter(|&y| alight(y)).count();
    assert!(trunk_alight >= 50, "the trunk flashed alight top to bottom ({trunk_alight} of 59)");
    assert!(w.get_bg(CellPos::new(100, 59)).unwrap().is_air(), "the top of the trunk was blown off");
    assert!(burning(&w) > 150, "and the crown round the strike ({})", burning(&w));
}

/// Wood left (anywhere: standing, fallen, flying) `ticks` after a tree is
/// struck by lightning (or lit at its foot).
fn wood_left_after(strike: bool, ticks: usize) -> usize {
    let mut w = boxed_world(3, 2, 94);
    w.set_weather(platypus_sim::Weather::new(94, 192, 100, 24));
    fill(&mut w, "stone", 1, 191, 1, 4);
    plant_tree(&mut w, 100);
    if strike {
        w.apply_edit(&WorldEdit::Lightning { x: 100, from_y: 120 });
    } else {
        w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(100, 10), radius: 5 });
    }
    for _ in 0..ticks {
        w.step();
    }
    let wood = w.materials().expect_id("wood");
    count_bg(&w, wood) + count(&w, wood) + w.bodies().iter().map(|b| b.world_cells().filter(|(_, c)| c.material == wood).count()).sum::<usize>()
}

/// Struck by lightning, a tree burns up far faster than one lit at its foot.
#[test]
fn a_lightning_struck_tree_burns_up_faster_than_a_lit_one() {
    // Measured: after 20 s, 47 of 240 left struck, 146 lit at the foot.
    let (struck, lit) = (wood_left_after(true, 1_200), wood_left_after(false, 1_200));
    assert!(struck * 2 < lit, "struck {struck} vs lit {lit} of the wood left");
}

#[test]
fn a_thunderstorm_throws_lightning() {
    let mut w = boxed_world(3, 2, 92);
    // The real band's height (thin clouds can't rain that hard).
    w.set_weather(platypus_sim::Weather::new(92, 192, 100, 176));
    w.apply_edit(&WorldEdit::Weather { x: 96, radius: 90, storm: true });
    let mut strikes = 0;
    let mut most_drops = 0;
    for _ in 0..6_000 {
        strikes += w.step().lightning.len();
        most_drops = most_drops.max(w.particles().len());
    }
    // The clouds are mostly above the loaded world (like zoomed in): rain and
    // lightning still come down into it.
    assert!(most_drops > 1_000, "it poured ({most_drops} drops)");
    assert!(strikes >= 1, "a storm over the world for 100 s struck at least once");
}

#[test]
fn a_body_trades_places_with_water_and_a_fast_one_splashes() {
    let mut w = boxed_world(1, 1, 93);
    let water = w.materials().expect_id("water");
    fill(&mut w, "water", 1, 63, 1, 20);
    let before = count(&w, water);
    let wet = |w: &World, x: i32, y: i32| w.get(CellPos::new(x, y)).unwrap().material == water;
    // Still: nothing moves.
    w.apply_edit(&WorldEdit::Displace { min: CellPos::new(20, 5), max: CellPos::new(27, 14), vel: [0, 0] });
    assert!((20..28).all(|x| wet(&w, x, 5)), "left alone");
    // Sinking through the surface a cell a tick: the water it moves into goes
    // where it just was (on top of it, like sand sinking), none is lost.
    w.apply_edit(&WorldEdit::Displace { min: CellPos::new(20, 14), max: CellPos::new(27, 23), vel: [0, -16] });
    assert!((20..28).all(|x| !wet(&w, x, 14)), "the row it moved into is clear");
    assert!((20..28).all(|x| wet(&w, x, 24)), "the water is where it was");
    assert_eq!(count(&w, water), before, "traded, not lost");
    // Deep under water, it's liquid all round: nothing to trade, nothing moves.
    let snapshot: Vec<bool> = (30..38).flat_map(|x| (2..12).map(move |y| (x, y))).map(|(x, y)| wet(&w, x, y)).collect();
    w.apply_edit(&WorldEdit::Displace { min: CellPos::new(30, 2), max: CellPos::new(37, 11), vel: [16, 0] });
    assert_eq!(snapshot, (30..38).flat_map(|x| (2..12).map(move |y| (x, y))).map(|(x, y)| wet(&w, x, y)).collect::<Vec<_>>(), "no wall of water pumped up");
    // Diving in fast: a splash flies.
    w.apply_edit(&WorldEdit::Displace { min: CellPos::new(40, 12), max: CellPos::new(47, 21), vel: [0, -110] });
    assert!(w.particles().len() > 10, "splash ({} drops)", w.particles().len());
}

#[test]
fn scorch_lights_what_burns_without_putting_flames_in_the_air() {
    let mut w = boxed_world(1, 1, 94);
    fill(&mut w, "tall_grass", 10, 20, 1, 4);
    w.apply_edit(&WorldEdit::Scorch { center: CellPos::new(15, 4), radius: 4 });
    assert!(burning(&w) > 0, "the grass caught");
    assert_eq!(count(&w, w.materials().expect_id("fire")), 0, "no flames in the air");
}

#[test]
fn a_scorched_trunk_can_still_be_set_alight() {
    let mut w = boxed_world(2, 2, 95);
    plant_tree(&mut w, 64);
    let wood = w.materials().expect_id("wood");
    // Burn a bit of it, put it out with water (charred by then), clear the
    // water away; then light it again.
    let water = w.materials().expect_id("water");
    w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(64, 20), radius: 2 });
    for _ in 0..90 {
        w.step();
    }
    // Over the flames too (they fill the cells in front of the trunk).
    w.apply_edit(&WorldEdit::Paint { center: CellPos::new(64, 30), radius: 28, material: water, overwrite: true });
    w.step();
    w.apply_edit(&WorldEdit::Dig { center: CellPos::new(64, 30), radius: 30, max_hardness: 1 });
    for _ in 0..600 {
        w.step();
    }
    let charcoal = w.materials().expect_id("charcoal");
    assert_eq!(count_bg(&w, charcoal), 0, "no charcoal in the background: it couldn't be relit");
    let left = count_bg(&w, wood);
    assert_eq!(w.get_bg(CellPos::new(64, 40)).unwrap().material, wood, "the tree still stands after the first fire");
    // Hold the igniter on it for two seconds.
    for _ in 0..120 {
        w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(64, 25), radius: 4 });
        w.step();
    }
    // It chars through and what's above comes down.
    let mut fell = false;
    for _ in 0..1_800 {
        w.step();
        fell |= !w.bodies().is_empty();
    }
    let top = w.get_bg(CellPos::new(64, 40)).unwrap();
    assert!(fell || top.is_air(), "the trunk burned through and fell ({} of {left} wood left)", count_bg(&w, wood));
}

#[test]
fn a_heat_gun_on_a_tree_sets_it_alight() {
    let mut w = boxed_world(2, 2, 96);
    plant_tree(&mut w, 64);
    // The heat tool: 60 °C a tick at the centre, held for a second.
    for _ in 0..60 {
        w.apply_edit(&WorldEdit::Heat { center: CellPos::new(64, 30), radius: 5, amount: 60 });
        w.step();
    }
    let burning_bg = w.chunks().flat_map(|c| c.background()).filter(|b| b.flags & platypus_sim::cell::flags::BURNING != 0).count();
    assert!(burning_bg > 0, "the trunk caught");
}

/// Background fire is heat-driven (SPEC §3.8): a lone flame can't keep itself
/// hot and goes out; a fire big enough to heat itself and what's above it
/// climbs and takes the tree.
#[test]
fn a_spark_on_a_trunk_dies_a_real_fire_takes_the_tree() {
    let outcome = |seed: u64, radius: i32| {
        let mut w = boxed_world(2, 2, seed);
        fill(&mut w, "stone", 1, 127, 1, 4);
        fill_bg(&mut w, "wood", 58, 70, 1, 70);
        let wood = w.materials().expect_id("wood");
        let before = count_bg(&w, wood);
        w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(64, 20), radius });
        for _ in 0..3_000 {
            w.step();
        }
        count_bg(&w, wood) as f32 / before as f32
    };
    let sparks: Vec<f32> = (0..10).map(|s| outcome(200 + s, 0)).collect();
    let fires: Vec<f32> = (0..6).map(|s| outcome(300 + s, 5)).collect();
    assert!(sparks.iter().filter(|&&k| k > 0.95).count() >= 7, "sparks mostly die: {sparks:?} of the wood left");
    assert!(fires.iter().filter(|&&k| k < 0.6).count() >= 4, "a real fire takes most of it: {fires:?}");
}

#[test]
fn ice_melts_slowly_in_a_warm_room_and_water_freezes_slowly_in_the_cold() {
    let mut w = boxed_world(1, 1, 97);
    let m = w.materials().clone();
    let (ice, water) = (m.expect_id("ice"), m.expect_id("water"));
    fill(&mut w, "ice", 10, 30, 1, 6);
    for _ in 0..120 {
        w.step();
    }
    let left = count(&w, ice);
    assert!(left > 80, "two seconds in, most of the ice is still there ({left} of 100)");
    for _ in 0..6_000 {
        w.step();
    }
    assert!(count(&w, ice) < 10, "after 100 s it's mostly water ({} ice)", count(&w, ice));
    // And water in a -5 °C world takes a while to freeze.
    let mut cold = World::new(98, m.clone());
    cold.insert_chunk(Chunk::filled(ChunkPos::new(0, 0), Cell::AIR));
    cold.set_climate(platypus_sim::Climate { surface_temp: -5, ..Default::default() });
    fill(&mut cold, "stone", 0, 64, 0, 1);
    fill(&mut cold, "water", 10, 30, 1, 6);
    for _ in 0..60 {
        cold.step();
    }
    assert!(count(&cold, water) > 80, "a second in, still water ({})", count(&cold, water));
    for _ in 0..12_000 {
        cold.step();
    }
    assert!(count(&cold, ice) > 60, "it froze in the end ({} ice)", count(&cold, ice));
}


#[test]
fn a_warm_wall_behind_a_pond_cools_and_sleeps() {
    let mut w = boxed_world(1, 1, 99);
    fill(&mut w, "stone", 1, 63, 1, 5);
    fill_bg(&mut w, "stone", 1, 63, 5, 30);
    fill(&mut w, "water", 1, 63, 5, 15);
    // Warm the wall behind the pond (a fire there earlier).
    w.apply_edit(&WorldEdit::Heat { center: CellPos::new(30, 10), radius: 3, amount: 0 });
    for x in 20..40 {
        let mut b = w.get_bg(CellPos::new(x, 10)).unwrap();
        b.heat = 19;
        w.set_bg(CellPos::new(x, 10), b);
    }
    run_until_asleep(&mut w, 3_000);
    assert!((20..40).all(|x| w.get_bg(CellPos::new(x, 10)).unwrap().heat == 0), "cooled to ambient");
}

/// Viscous liquids settle slower and heap; water settles flat (SPEC §3.4).
#[test]
fn liquids_settle_by_viscosity() {
    let (water, lava) = (settle_probe("water", 500), settle_probe("lava", 500));
    assert!(water.0 < 400 && water.1 <= 3, "water settles fast and flat: {water:?}");
    assert!(lava.0 > water.0 * 3 && lava.1 > water.1 + 4, "lava is slow and heaps: {lava:?}");
}

/// (ticks until asleep or the cap, highest minus lowest surface column)
fn settle_probe(liquid: &str, seed: u64) -> (usize, i32) {
    let mut w = boxed_world(2, 1, seed);
    let m = w.materials().clone();
    let id = m.expect_id(liquid);
    fill(&mut w, liquid, 40, 70, 20, 50);
    let mut t = 0;
    while t < 6_000 {
        w.step();
        t += 1;
        if w.chunks().all(|c| !c.is_awake()) {
            break;
        }
    }
    let heights: Vec<i32> = (1..127)
        .map(|x| (1..64).rev().find(|&y| w.get(CellPos::new(x, y)).unwrap().material == id).unwrap_or(0))
        .filter(|&h| h > 0)
        .collect();
    let spread = heights.iter().max().unwrap_or(&0) - heights.iter().min().unwrap_or(&0);
    (t, spread)
}



#[test]
fn water_on_lava_makes_a_hot_obsidian_crust_and_boils_off() {
    let mut w = boxed_world(1, 1, 101);
    let m = w.materials().clone();
    let (lava, water, steam, obsidian) = (m.expect_id("lava"), m.expect_id("water"), m.expect_id("steam"), m.expect_id("obsidian"));
    fill(&mut w, "lava", 1, 63, 1, 10);
    fill(&mut w, "water", 20, 44, 20, 30);
    let poured = count(&w, water);
    let (mut most_steam, mut crust_hot) = (0, false);
    for _ in 0..600 {
        w.step();
        most_steam = most_steam.max(count(&w, steam));
        crust_hot |= w.chunks().flat_map(|c| c.cells()).any(|c| c.material == obsidian && c.heat > 400);
    }
    assert!(count(&w, obsidian) > 10, "a crust of obsidian");
    assert!(crust_hot, "fresh obsidian is hot");
    assert!(most_steam > poured / 3, "much of the water boiled off ({most_steam} steam from {poured})");
    assert!(count(&w, lava) > 0, "lava under the crust");
}

/// A dam break: a block of water 50 wide, 60 high at the left of an empty
/// basin. Returns (tick, flood front x, height at x=10) samples.
fn dam_break(liquid: &str) -> Vec<(usize, i32, i32)> {
    let mut w = boxed_world(4, 2, 7);
    let id = w.materials().expect_id(liquid);
    fill(&mut w, liquid, 1, 51, 1, 61);
    let mut out = Vec::new();
    for t in 1..=240 {
        w.step();
        if [15, 30, 60, 120, 240].contains(&t) {
            let front = (1..255).rev().find(|&x| (1..8).any(|y| w.get(CellPos::new(x, y)).unwrap().material == id)).unwrap_or(0);
            let h = (1..127).filter(|&y| w.get(CellPos::new(10, y)).unwrap().material == id).count() as i32;
            out.push((t, front, h));
        }
    }
    out
}

/// Water released in a block collapses like a dam break (pressure pushes it
/// out through itself), not a wall eroding from its face (SPEC §3.2).
#[test]
fn a_block_of_water_collapses_like_a_dam_break() {
    let samples = dam_break("water");
    let height_at = |t: usize| samples.iter().find(|s| s.0 == t).unwrap().2;
    let front_at = |t: usize| samples.iter().find(|s| s.0 == t).unwrap().1;
    assert!(front_at(15) > 150, "the flood races along the floor: {samples:?}");
    assert!(height_at(30) <= 40, "half a second in, the block has slumped: {samples:?}");
    assert!(height_at(120) <= 24, "two seconds in, it's mostly spread out: {samples:?}");
}


#[test]
fn a_pickaxe_mines_the_playfield_an_axe_the_background() {
    let mut w = boxed_world(2, 2, 102);
    plant_tree(&mut w, 64);
    let wood = w.materials().expect_id("wood");
    let before = count_bg(&w, wood);
    let at = CellPos::new(64, 20);
    for _ in 0..200 {
        w.apply_edit(&WorldEdit::Mine { center: at, radius: 4, power: 4, max_hardness: 150, back: false });
    }
    assert_eq!(count_bg(&w, wood), before, "the pickaxe leaves the tree alone");
    for _ in 0..40 {
        w.apply_edit(&WorldEdit::Mine { center: at, radius: 4, power: 4, max_hardness: 150, back: true });
    }
    assert!(count_bg(&w, wood) < before - 20, "the axe cuts into the trunk");
    // Behind solid ground the axe can't reach.
    fill(&mut w, "stone", 90, 110, 1, 30);
    fill_bg(&mut w, "stone", 90, 110, 1, 30);
    let walls = count_bg(&w, w.materials().expect_id("stone"));
    for _ in 0..200 {
        w.apply_edit(&WorldEdit::Mine { center: CellPos::new(100, 15), radius: 4, power: 4, max_hardness: 150, back: true });
    }
    assert_eq!(count_bg(&w, w.materials().expect_id("stone")), walls);
}


/// A thunderstorm over a row of trees: the struck one burns through, the
/// rain keeps it from spreading and puts it out.
#[test]
fn in_a_storm_a_struck_tree_burns_alone_and_the_rain_puts_it_out() {
    let mut w = boxed_world(4, 2, 94);
    w.set_weather(platypus_sim::Weather::new(94, 256, 100, 176));
    fill(&mut w, "stone", 1, 255, 1, 4);
    let trees = [50, 110, 170, 230];
    for x in trees {
        plant_tree(&mut w, x);
    }
    w.apply_edit(&WorldEdit::Weather { x: 128, radius: 120, storm: true });
    for _ in 0..1_200 {
        w.step();
    }
    let wood = w.materials().expect_id("wood");
    let standing = |w: &World| trees.map(|t| (t - 20..t + 20).flat_map(|x| (0..60).map(move |y| CellPos::new(x, y))).filter(|&p| w.get_bg(p).is_some_and(|b| b.material == wood)).count());
    let before = standing(&w);
    w.apply_edit(&WorldEdit::Lightning { x: 110, from_y: 120 });
    for _ in 0..1_800 {
        w.step();
    }
    let after = standing(&w);
    let alight = |w: &World| w.chunks().flat_map(|c| c.background()).filter(|b| b.flags & platypus_sim::cell::flags::BURNING != 0).count();
    // The last few smoulder on a little longer.
    let mut out_by = None;
    for t in 1_800..3_600 {
        if alight(&w) == 0 {
            out_by = Some(t);
            break;
        }
        w.step();
    }
    let burning = alight(&w);
    // Measured: [240, 232, 232, 240] -> [240, 5, 226, 240], all out at ~31 s.
    assert!(out_by.is_some(), "out within a minute");
    assert!(after[1] < before[1] / 4, "the struck tree burned through ({} of {} standing)", after[1], before[1]);
    for i in [0, 2, 3] {
        assert!(after[i] + 10 >= before[i], "tree {i} didn't catch ({} of {})", after[i], before[i]);
    }
    assert_eq!(burning, 0, "and the rain put it out");
}

/// A crown fire throws embers, but each touches one leaf and lights it only
/// by chance, cooling as it flies, and some go out: the next tree over
/// sometimes catches, one beyond it hardly ever.
#[test]
fn embers_seldom_carry_a_fire_across_a_gap() {
    let mut near = 0;
    let mut far = 0;
    for seed in 0..8 {
        let mut w = boxed_world(5, 2, 500 + seed);
        fill(&mut w, "stone", 1, 319, 1, 4);
        for x in [60, 120, 230] {
            plant_tree(&mut w, x);
        }
        w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(60, 52), radius: 14 });
        for _ in 0..3_600 {
            w.step();
        }
        let leaves = w.materials().expect_id("leaves");
        let crown = |w: &World, t: i32| (t - 16..=t + 16).flat_map(|x| (46..=58).map(move |y| CellPos::new(x, y))).filter(|&p| w.get_bg(p).is_some_and(|b| b.material == leaves)).count();
        let (a, b, c) = (crown(&w, 60), crown(&w, 120), crown(&w, 230));
        assert!(a < 50, "the lit crown burned ({a} left)");
        near += (b < 350) as u32;
        far += (c < 350) as u32;
    }
    // Measured: 2/8 and 0/8 (before, when an ember lit the first leaf it
    // brushed: 4/8 and 2/8).
    assert!(near <= 3, "a tree 28 cells off caught {near}/8 times");
    assert!(far <= 1, "a tree 108 cells off caught {far}/8 times");
}

// ---- blocks (hands) -----------------------------------------------------------

#[test]
fn a_block_breaks_all_at_once_after_enough_hits() {
    use platypus_sim::{BLOCK, block_cells};
    let mut w = boxed_world(1, 1, 60);
    fill(&mut w, "stone", 1, 63, 1, 20);
    let stone = w.materials().expect_id("stone");
    let block = CellPos::new(5, 3); // cells 20..24 × 12..16
    let before = count(&w, stone);
    // Stone is 60 hard: a power-35 pick takes two hits, and the first
    // removes nothing.
    let hit = WorldEdit::MineBlock { block, power: 35, max_hardness: 100, back: false };
    assert!(w.apply_edit(&hit).removed.is_empty(), "one hit only cracks it");
    assert_eq!(count(&w, stone), before);
    let r = w.apply_edit(&hit);
    assert_eq!(r.removed, vec![(stone, (BLOCK * BLOCK) as u32)], "the second takes all 16 cells");
    assert!(block_cells(block).all(|p| w.get(p).unwrap().is_air()));
    // Just the block: its neighbours are whole.
    assert_eq!(count(&w, stone), before - 16);
    assert_ne!(w.get(CellPos::new(19, 13)).unwrap().material, MaterialId::AIR);
}

#[test]
fn ore_beyond_a_tools_tier_stays_in_the_block() {
    let mut w = boxed_world(1, 1, 61);
    fill(&mut w, "stone", 1, 63, 1, 20);
    let (stone, obsidian) = (w.materials().expect_id("stone"), w.materials().expect_id("obsidian"));
    // Two cells of the block are too hard for this pick.
    w.set(CellPos::new(21, 13), Cell::new(obsidian, 0));
    w.set(CellPos::new(22, 13), Cell::new(obsidian, 0));
    let hit = WorldEdit::MineBlock { block: CellPos::new(5, 3), power: 35, max_hardness: 100, back: false };
    w.apply_edit(&hit);
    let r = w.apply_edit(&hit);
    assert_eq!(r.removed, vec![(stone, 14)]);
    assert_eq!(w.get(CellPos::new(21, 13)).unwrap().material, obsidian, "the hard bit stays");
}

#[test]
fn placing_a_block_fills_its_empty_cells_with_the_pattern() {
    use platypus_sim::block_cells;
    let mut w = boxed_world(1, 1, 62);
    fill(&mut w, "stone", 1, 63, 1, 8);
    let brick = w.materials().expect_id("brick");
    let block = CellPos::new(5, 2); // cells 20..24 × 8..12, on the floor
    w.set(CellPos::new(21, 9), Cell::new(w.materials().expect_id("dirt"), 0));
    let r = w.apply_edit(&WorldEdit::PlaceBlock { block, material: brick, back: false });
    assert_eq!(r.placed, 15, "all but the one taken");
    for p in block_cells(block).filter(|&p| p != CellPos::new(21, 9)) {
        let c = w.get(p).unwrap();
        assert_eq!(c.material, brick);
        assert_eq!(Some(c.shade), w.materials().pattern_shade(brick, p.x, p.y), "shaded by the pattern at {p:?}");
    }
    // The pattern is anchored to the world: the same cell of the next block
    // over (8 cells on, one pattern width) has the same shade.
    w.apply_edit(&WorldEdit::PlaceBlock { block: CellPos::new(7, 2), material: brick, back: false });
    assert_eq!(w.get(CellPos::new(20, 10)).unwrap().shade, w.get(CellPos::new(28, 10)).unwrap().shade);
}

#[test]
fn an_axe_block_takes_the_background_only_where_the_front_is_open() {
    let mut w = boxed_world(1, 1, 63);
    fill_bg(&mut w, "wood", 20, 28, 8, 12);
    // Something standing in front of half of it.
    fill(&mut w, "stone", 24, 28, 8, 12);
    let wood = w.materials().expect_id("wood");
    let r = w.apply_edit(&WorldEdit::MineBlock { block: CellPos::new(5, 2), power: 50, max_hardness: 100, back: true });
    assert_eq!(r.removed, vec![(wood, 16)]);
    let r = w.apply_edit(&WorldEdit::MineBlock { block: CellPos::new(6, 2), power: 50, max_hardness: 100, back: true });
    assert!(r.removed.is_empty(), "behind the stone it can't reach");
}

#[test]
fn cobwebs_hang_from_a_ceiling_and_nothing_else() {
    let mut w = boxed_world(2, 2, 29);
    fill(&mut w, "stone", 20, 40, 80, 84);
    fill(&mut w, "cobweb", 20, 30, 74, 80); // under the ceiling
    fill(&mut w, "cobweb", 60, 64, 70, 74); // in the air
    fill(&mut w, "moss", 60, 64, 72, 73); // (a plant that doesn't hang: gone too)
    let web = w.materials().expect_id("cobweb");
    for _ in 0..30 {
        w.step();
    }
    let at = |x0: i32, x1: i32, y0: i32, y1: i32| (x0..x1).flat_map(|x| (y0..y1).map(move |y| CellPos::new(x, y))).filter(|&p| w.get(p).is_some_and(|c| c.material == web)).count();
    assert_eq!(at(20, 30, 74, 80), 60, "the web under the ceiling stays");
    assert_eq!(at(60, 64, 70, 74), 0, "a web in the air falls apart");
}

// ---- wand lightning ---------------------------------------------------------

/// A zap reaches what it's aimed at through open air, lighting the
/// background it passes; a wall stops it; the step reports it, with the
/// burst where it ended.
#[test]
fn a_zap_reaches_its_target_and_a_wall_stops_it() {
    let mut w = boxed_world(3, 1, 11);
    fill_bg(&mut w, "wood", 20, 60, 16, 26);
    let end = w.zap(CellPos::new(10, 20), CellPos::new(150, 20));
    assert!((end.x - 150).abs() <= 1 && (end.y - 20).abs() <= 1, "open air all the way: ended at {end:?}");
    let stats = w.step();
    assert_eq!(stats.zaps.len(), 1);
    assert_eq!(stats.zaps[0].to, end);
    assert!(stats.detonated.iter().any(|&(at, ..)| at == end), "it bursts where it ends");
    let caught = (20..60).any(|x| w.get_bg(CellPos::new(x, 20)).is_some_and(|c| c.flags & platypus_sim::cell::flags::BURNING != 0));
    assert!(caught, "the wood it passed caught");

    let mut w = boxed_world(3, 1, 12);
    fill(&mut w, "stone", 80, 84, 1, 60);
    let end = w.zap(CellPos::new(10, 20), CellPos::new(150, 20));
    assert!((79..=84).contains(&end.x), "the wall stops it: ended at {end:?}");
}

/// Acid lasts: each cell eats several before it's spent, so a little acid
/// digs a pit bigger than itself; glass (inert) holds it.
#[test]
fn acid_eats_more_than_itself_and_not_glass() {
    let m = mats();
    let (dirt, glass, acid) = (m.expect_id("dirt"), m.expect_id("glass"), m.expect_id("acid"));
    let mut w = boxed_world(2, 1, 21);
    fill(&mut w, "dirt", 10, 50, 1, 30);
    fill(&mut w, "glass", 70, 110, 1, 30);
    fill(&mut w, "acid", 25, 35, 30, 33);
    fill(&mut w, "acid", 85, 95, 30, 33);
    let (dirt0, glass0) = (count(&w, dirt), count(&w, glass));
    for _ in 0..3000 {
        w.step();
    }
    let eaten = dirt0 - count(&w, dirt);
    assert!(eaten > 60, "30 cells of acid ate {eaten} dirt");
    assert_eq!(count(&w, glass), glass0, "glass is inert");
    assert!(count(&w, acid) >= 25, "the acid on glass is still there");
}

/// Lightning into water charges the whole pool it's connected to, and no
/// more: not a separate puddle, not dry ground.
#[test]
fn a_zap_into_water_charges_the_whole_pool() {
    let mut w = boxed_world(3, 1, 13);
    fill(&mut w, "stone", 20, 140, 1, 12);
    fill(&mut w, "water", 30, 90, 12, 20);
    fill(&mut w, "stone", 90, 94, 12, 22);
    fill(&mut w, "water", 94, 130, 12, 20);
    let end = w.zap(CellPos::new(60, 50), CellPos::new(60, 15));
    let stats = w.step();
    let z = &stats.zaps[0];
    assert_eq!(z.to, end);
    assert!(z.charged.len() >= 60 * 8 - 20, "the whole first pool: {}", z.charged.len());
    assert!(z.charged.iter().all(|p| p.x < 90), "not the pool across the wall");
}
