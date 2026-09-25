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
fn blood_boils_and_freezes_like_water() {
    let mut w = boxed_world(1, 1, 25);
    let m = w.materials().clone();
    fill(&mut w, "blood", 10, 20, 1, 6);
    fill(&mut w, "blood", 40, 50, 1, 6);
    w.apply_edit(&WorldEdit::Heat { center: CellPos::new(15, 3), radius: 6, amount: 300 });
    w.apply_edit(&WorldEdit::Heat { center: CellPos::new(45, 3), radius: 6, amount: -120 });
    for _ in 0..5 {
        w.step();
    }
    assert!(count(&w, m.expect_id("blood_steam")) > 10, "boiled");
    assert!(count(&w, m.expect_id("frozen_blood")) > 10, "froze");
    for _ in 0..4_000 {
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
    plant_tree(&mut w, 40);
    plant_tree(&mut w, 100);
    plant_tree(&mut w, 150);
    let leaves = w.materials().expect_id("leaves");
    w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(40, 10), radius: 2 });
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
    let mut secs = 1;
    while (bg_burning(&w) > 0 || burning(&w) > 0) && secs < 60 {
        for _ in 0..60 {
            w.step();
        }
        secs += 1;
    }
    assert!((12..=40).contains(&secs), "the tree burned for {secs} s");
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
    w.apply_edit(&WorldEdit::Ignite { center: CellPos::new(95, 11), radius: 3 });
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
    assert!(water.douses && water.heat == 0.0 && water.corrosion == 0.0, "{water:?}");
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
