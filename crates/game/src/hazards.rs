//! The caves' hazards, and what gives them away (survivable, if you heed
//! it): read off the world's cells, not the plan, so they hold for what
//! the player's dug, built and flooded as well as for what was generated.
//!
//! - **Drips**: under liquid held up by a few cells of rock (the caves'
//!   perched pockets, `worldgen::caves::Pocket`; a pool over a thin floor),
//!   drops fall from the ceiling now and then and splash below. Dig up
//!   there and it all comes down on you.
//! - **Cave-ins**: a loose roof (the caves' loose sand and gravel, ore
//!   glinting in it) trickles grains on whoever's under it. Disturb it (dig
//!   into it, blast near it, a quake) and it rumbles, and a moment later
//!   all of it that's joined comes down (`CaveIns`).
//! - **Firedamp**: gas gathered under some chambers' ceilings (coal seams
//!   in the walls round them, a faint haze). A naked flame in it (a torch,
//!   held or planted, a lantern, a candle) sets it off, the blast running
//!   through all of it and lighting the coal. Flashlights are safe; a
//!   safety lamp is safe, and its flame burns blue near it (`Firedamp`).

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use platypus_sim::particles::{Landing, Particle};
use platypus_sim::rng::Rng;
use platypus_sim::{Cell, CellPos, Kind, MaterialId, cell::flags};

use crate::creatures::Kinematics;
use crate::creatures::player::LocalPlayer;
use crate::sound::PlaySound;
use crate::world::{SimWorld, TickSet};

pub struct HazardsPlugin;

impl Plugin for HazardsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CaveIns>().init_resource::<Firedamp>().add_systems(FixedUpdate, (drips, trickles, cave_ins, firedamp).after(TickSet::Bodies).before(TickSet::Cells));
    }
}

/// Where drips are looked for: this far round the player (cells, across
/// and up and down), this many cells tried a tick (a pocket drips about
/// once a second or two, a bigger one more).
const DRIP_REACH: (i32, i32) = (220, 130);
const DRIP_TRIES: usize = 5;
/// How deep a pocket's liquid may be looked down through, and how thick the
/// rock under it may be, for it to drip (cells).
const DRIP_DEEP: i32 = 30;
const DRIP_ROOF: i32 = 8;

/// Drips under perched liquid near the player, each splashing where it
/// lands (a drop's worth of sound, late by its fall).
fn drips(mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut rng: Local<Option<Rng>>, mut splashes: Local<Vec<(u32, Vec2)>>, mut sounds: MessageWriter<PlaySound>) {
    let rng = rng.get_or_insert_with(|| Rng::seeded(&[0xD219]));
    let unit = |rng: &mut Rng| rng.next_u32() as f32 / u32::MAX as f32;
    splashes.retain_mut(|(ticks, at)| {
        *ticks = ticks.saturating_sub(1);
        if *ticks == 0 {
            sounds.write(PlaySound::at("drip", *at).volume(0.8));
        }
        *ticks > 0
    });
    let Ok(k) = player.single() else { return };
    let c = CellPos::from_world(k.body.pos.x, k.body.pos.y);
    let mats = sim.materials().clone();
    let kind = |sim: &SimWorld, p: CellPos| sim.world.get(p).map(|cell| (cell, mats.phys(cell.material).kind));
    for _ in 0..DRIP_TRIES {
        let p = c.offset(((unit(rng) * 2.0 - 1.0) * DRIP_REACH.0 as f32) as i32, ((unit(rng) * 2.0 - 1.0) * DRIP_REACH.1 as f32) as i32);
        let Some((wet, Kind::Liquid)) = kind(&sim, p) else { continue };
        // Down through it to its bottom, down through what holds it up, to
        // open air under that.
        let Some(bottom) = (1..=DRIP_DEEP).find(|&d| kind(&sim, p.offset(0, -d)).is_none_or(|(_, k)| k != Kind::Liquid)) else { continue };
        let roof = (0..=DRIP_ROOF).find(|&d| kind(&sim, p.offset(0, -bottom - d)).is_none_or(|(_, k)| !matches!(k, Kind::Static | Kind::Powder)));
        let Some(roof) = roof.filter(|&r| r > 0) else { continue };
        let under = p.offset(0, -bottom - roof);
        if !kind(&sim, under).is_some_and(|(_, k)| matches!(k, Kind::Empty | Kind::Gas)) {
            continue;
        }
        let drop = mats.spawn(wet.material, rng);
        let at = [under.x as f32 + 0.2 + 0.6 * unit(rng), under.y as f32 + 0.9];
        sim.world.emit(Particle::new(at, [0.0, -0.05], drop, 240, Landing::Vanish));
        // Its splash, when it's fallen as far as the floor.
        let fall = (1..240).find(|&d| !kind(&sim, under.offset(0, -d)).is_some_and(|(_, k)| matches!(k, Kind::Empty | Kind::Gas))).unwrap_or(240);
        let ticks = (2.0 * fall as f32 / platypus_sim::particles::GRAVITY).sqrt() as u32 + 1;
        splashes.push((ticks, Vec2::new(at[0], (under.y - fall + 1) as f32)));
    }
}

/// What a loose roof is (the caves': `worldgen::caves::Loose`), and what's
/// in it that comes down with it (its nuggets).
const LOOSE: [&str; 2] = ["loose_sand", "loose_gravel"];
const NUGGETS: [&str; 3] = ["copper_ore", "iron_ore", "amethyst"];

/// Trickles: looked for this far round the player (cells), this many
/// ceilings tried a tick, one in this many of a loose one's tries trickling.
const TRICKLE_REACH: (i32, i32) = (130, 90);
const TRICKLE_TRIES: usize = 3;
const TRICKLE_ONE_IN: u32 = 5;

/// Loose roofs near the player, counted a tile at a time (`TILE` cells
/// square) every `SCAN_EVERY` ticks, this far round (cells): a tile with
/// fewer than last time was disturbed.
const TILE: i32 = 16;
const SCAN_EVERY: u64 = 15;
const SCAN_REACH: (i32, i32) = (160, 112);
/// A shake this hard (`fx::Trauma`: a blast, a quake) brings down the loose
/// roofs this near the player.
const SHAKEN: f32 = 0.45;
const SHAKEN_REACH: f32 = 150.0;
/// A cave-in: the loose cells joined to where it starts, this far from it at
/// most (and this many); the rumble's warning (ticks) before it comes down;
/// how long it takes coming down (ticks, lowest first).
const CAVE_IN_REACH: i32 = 60;
const CAVE_IN_MOST: usize = 5_000;
const CAVE_IN_WARNING: u64 = 45;
const CAVE_IN_FALLING: usize = 24;
/// What it does to whoever it comes down on: a cell of it over them this
/// much (gravel half as much again), this much at most.
const CAVE_IN_HURTS: f32 = 0.16;
const CAVE_IN_WORST: f32 = 25.0;

/// Loose roofs: what was counted, and the cave-ins under way.
#[derive(Resource, Default)]
pub struct CaveIns {
    counts: HashMap<(i32, i32), u32>,
    /// Tiles a cave-in's own falling empties (not disturbed: let be) till
    /// this tick.
    busy: HashMap<(i32, i32), u64>,
    claimed: HashSet<CellPos>,
    pub under_way: Vec<CaveIn>,
    shaken: bool,
}

pub struct CaveIn {
    /// Lowest first.
    pub cells: Vec<CellPos>,
    pub at: Vec2,
    /// The tick it gives way.
    pub falls: u64,
    done: usize,
}

/// Grains trickling from loose ceilings near the player (now and then a
/// whisper of it), each the colour of what it'll come down as.
fn trickles(mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut rng: Local<Option<Rng>>, mut hushed: Local<u64>, mut sounds: MessageWriter<PlaySound>) {
    let rng = rng.get_or_insert_with(|| Rng::seeded(&[0x7121]));
    let unit = |rng: &mut Rng| rng.next_u32() as f32 / u32::MAX as f32;
    let Ok(k) = player.single() else { return };
    let c = CellPos::from_world(k.body.pos.x, k.body.pos.y);
    let mats = sim.materials().clone();
    let loose: Vec<MaterialId> = LOOSE.iter().filter_map(|n| mats.id(n)).collect();
    let open = |sim: &SimWorld, p: CellPos| sim.world.get(p).is_some_and(|cell| matches!(mats.phys(cell.material).kind, Kind::Empty | Kind::Gas));
    let tick = sim.world.tick();
    for _ in 0..TRICKLE_TRIES {
        let p = c.offset(((unit(rng) * 2.0 - 1.0) * TRICKLE_REACH.0 as f32) as i32, ((unit(rng) * 2.0 - 1.0) * TRICKLE_REACH.1 as f32) as i32);
        if !open(&sim, p) {
            continue;
        }
        let Some(up) = (1..90).find(|&d| !open(&sim, p.offset(0, d))) else { continue };
        let roof = p.offset(0, up);
        let Some(cell) = sim.world.get(roof).filter(|cell| loose.contains(&cell.material)) else { continue };
        if !rng.next_u32().is_multiple_of(TRICKLE_ONE_IN) {
            continue;
        }
        let grain = mats.phys(cell.material).crumbles_into;
        for _ in 0..2 + rng.next_u32() % 3 {
            let g = mats.spawn(grain, rng);
            let at = [roof.x as f32 + (unit(rng) * 5.0 - 2.5), roof.y as f32 - 0.2];
            sim.world.emit(Particle::new(at, [0.0, -0.02 - 0.08 * unit(rng)], g, 150, Landing::Vanish));
        }
        if tick >= *hushed {
            *hushed = tick + 50;
            sounds.write(PlaySound::at("mine_dirt", Vec2::new(roof.x as f32, roof.y as f32)).volume(0.22).pitch(1.6));
        }
    }
}

/// Loose roofs disturbed near the player rumble, then come down: each tile
/// counted against last time (fewer: dug, blown away), and a hard shake
/// brings down whatever's near.
#[allow(clippy::too_many_arguments)]
fn cave_ins(
    mut sim: ResMut<SimWorld>,
    mut roofs: ResMut<CaveIns>,
    mut trauma: ResMut<crate::fx::Trauma>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut bodies: Query<(&Kinematics, &mut crate::creatures::Health), Without<crate::creatures::Dormant>>,
    mut rng: Local<Option<Rng>>,
    mut sounds: MessageWriter<PlaySound>,
) {
    let rng = rng.get_or_insert_with(|| Rng::seeded(&[0xCA7E]));
    let unit = |rng: &mut Rng| rng.next_u32() as f32 / u32::MAX as f32;
    let mats = sim.materials().clone();
    let loose: Vec<MaterialId> = LOOSE.iter().filter_map(|n| mats.id(n)).collect();
    let nuggets: Vec<MaterialId> = NUGGETS.iter().filter_map(|n| mats.id(n)).collect();
    let tick = sim.world.tick();
    let is = |sim: &SimWorld, p: CellPos, of: &[MaterialId]| sim.world.get(p).is_some_and(|cell| of.contains(&cell.material));
    let roofs = &mut *roofs;
    roofs.busy.retain(|_, until| *until > tick);

    // Where it starts.
    let mut starts: Vec<CellPos> = Vec::new();
    if let Ok(k) = player.single() {
        let c = CellPos::from_world(k.body.pos.x, k.body.pos.y);
        let shaken = trauma.0 >= SHAKEN;
        let shook = shaken && !roofs.shaken;
        roofs.shaken = shaken;
        if tick.is_multiple_of(SCAN_EVERY) || shook {
            let (tx0, ty0) = ((c.x - SCAN_REACH.0).div_euclid(TILE), (c.y - SCAN_REACH.1).div_euclid(TILE));
            let (tx1, ty1) = ((c.x + SCAN_REACH.0).div_euclid(TILE), (c.y + SCAN_REACH.1).div_euclid(TILE));
            let mut counts = HashMap::new();
            for ty in ty0..=ty1 {
                for tx in tx0..=tx1 {
                    let mut n = 0;
                    let mut any = None;
                    for y in ty * TILE..(ty + 1) * TILE {
                        for x in tx * TILE..(tx + 1) * TILE {
                            let p = CellPos::new(x, y);
                            if is(&sim, p, &loose) {
                                n += 1;
                                any = any.or(Some(p));
                            }
                        }
                    }
                    let Some(any) = any else { continue };
                    counts.insert((tx, ty), n);
                    let disturbed = roofs.counts.get(&(tx, ty)).is_some_and(|&was| n < was) && !roofs.busy.contains_key(&(tx, ty));
                    let near = Vec2::new(any.x as f32, any.y as f32).distance(k.body.pos) < SHAKEN_REACH;
                    if disturbed || shook && near {
                        starts.push(any);
                    }
                }
            }
            roofs.counts = counts;
        }
    }

    // Each start: the loose roof joined to it (and the nuggets in it).
    for start in starts {
        if roofs.claimed.contains(&start) {
            continue;
        }
        let mut cells = Vec::new();
        let mut seen = HashSet::from([start]);
        let mut todo = vec![start];
        while let Some(p) = todo.pop() {
            cells.push(p);
            if cells.len() >= CAVE_IN_MOST {
                break;
            }
            if !is(&sim, p, &loose) {
                continue;
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)] {
                let n = p.offset(dx, dy);
                if (n.x - start.x).abs().max((n.y - start.y).abs()) > CAVE_IN_REACH || roofs.claimed.contains(&n) || !seen.insert(n) {
                    continue;
                }
                if is(&sim, n, &loose) || is(&sim, n, &nuggets) {
                    todo.push(n);
                }
            }
        }
        cells.sort_by_key(|p| (p.y, p.x));
        let sum = cells.iter().fold(Vec2::ZERO, |s, p| s + Vec2::new(p.x as f32, p.y as f32));
        let at = sum / cells.len() as f32;
        let falls = tick + CAVE_IN_WARNING;
        for p in &cells {
            roofs.claimed.insert(*p);
            roofs.busy.insert((p.x.div_euclid(TILE), p.y.div_euclid(TILE)), falls + CAVE_IN_FALLING as u64 + SCAN_EVERY * 2);
        }
        sounds.write(PlaySound::at("thunder", at).pitch(0.6 + 0.2 * unit(rng)).volume((cells.len() as f32 / 800.0).clamp(0.35, 1.0)));
        if let Ok(k) = player.single()
            && k.body.pos.distance(at) < 200.0
        {
            trauma.0 = (trauma.0 + 0.2).min(1.0);
        }
        info!("hazards: a cave-in of {} cells at ({:.0}, {:.0}), coming down in {CAVE_IN_WARNING} ticks", cells.len(), at.x, at.y);
        roofs.under_way.push(CaveIn { cells, at, falls, done: 0 });
    }

    // Under way: rumbling (grains shaken loose), then coming down, lowest
    // first, the nuggets with it.
    for cave in &mut roofs.under_way {
        if tick < cave.falls {
            if tick.is_multiple_of(4) {
                let p = cave.cells[(rng.next_u32() as usize) % cave.cells.len().min(400)];
                if let Some(cell) = sim.world.get(p).filter(|cell| loose.contains(&cell.material)) {
                    let g = mats.spawn(mats.phys(cell.material).crumbles_into, rng);
                    sim.world.emit(Particle::new([p.x as f32 + unit(rng), p.y as f32 - 0.2], [0.0, -0.05], g, 150, Landing::Vanish));
                }
            }
            continue;
        }
        if cave.done == 0 {
            // Whoever's under it is hit by what's over them (as it lands:
            // its fall's about as long as its coming down).
            let gravel = mats.id("loose_gravel");
            for (k, mut h) in &mut bodies {
                let (lo, hi) = k.body.cells_at(k.body.pos);
                let over: f32 = cave
                    .cells
                    .iter()
                    .filter(|p| (lo.x..=hi.x).contains(&p.x) && p.y > hi.y && p.y < hi.y + 160)
                    .map(|p| if sim.world.get(*p).map(|c| c.material) == gravel { 1.5 } else { 1.0 })
                    .sum();
                if over > 0.0 {
                    h.harm((over * CAVE_IN_HURTS).min(CAVE_IN_WORST), crate::creatures::nature::Harm::Blunt);
                }
            }
            sounds.write(PlaySound::at("break", cave.at).pitch(0.5).volume(0.9));
            if let Ok(k) = player.single()
                && k.body.pos.distance(cave.at) < 200.0
            {
                trauma.0 = (trauma.0 + 0.3).min(1.0);
            }
        }
        let batch = cave.cells.len().div_ceil(CAVE_IN_FALLING);
        let end = (cave.done + batch).min(cave.cells.len());
        for &p in &cave.cells[cave.done..end] {
            let Some(cell) = sim.world.get(p) else { continue };
            if loose.contains(&cell.material) {
                let mut rubble = mats.spawn(mats.phys(cell.material).crumbles_into, rng);
                rubble.flags |= flags::LOOSE;
                sim.world.set(p, rubble);
            } else if nuggets.contains(&cell.material) {
                sim.world.set(p, Cell::AIR);
                let mut nugget = cell;
                nugget.flags |= flags::LOOSE;
                sim.world.emit(Particle::new([p.x as f32 + 0.5, p.y as f32 + 0.5], [(unit(rng) - 0.5) * 0.3, -0.1], nugget, 180, Landing::Settle));
            }
        }
        cave.done = end;
    }
    let finished: Vec<CaveIn> = roofs.under_way.extract_if(.., |c| c.done >= c.cells.len()).collect();
    for cave in finished {
        for p in cave.cells {
            roofs.claimed.remove(&p);
        }
    }
}

/// Firedamp near the player, for the safety lamp: 0 none in reach, 1 thick
/// round it.
#[derive(Resource, Default)]
pub struct Firedamp {
    pub near: f32,
}

/// A naked flame lights firedamp this near it (cells).
const FLAME_REACH: i32 = 8;
/// The safety lamp: firedamp sniffed for this far round the player
/// (cells), every this many cells, every this many ticks; this many cells'
/// worth right by it is as thick as it reads.
const SNIFF_REACH: i32 = 64;
const SNIFF_STEP: i32 = 4;
const SNIFF_EVERY: u64 = 10;
const SNIFF_FULL: f32 = 14.0;

/// Naked flames in firedamp set it off; and how much of it's near the
/// player.
fn firedamp(
    mut sim: ResMut<SimWorld>,
    mut damp: ResMut<Firedamp>,
    toggles: Res<crate::light::LightToggles>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    flames: Query<(&GlobalTransform, &crate::light::LightSource, Has<crate::light::torch::HeldFlame>)>,
    mut quiet: Local<u64>,
) {
    let Some(gas) = sim.materials().id("firedamp") else { return };
    let tick = sim.world.tick();
    let is_gas = |sim: &SimWorld, p: CellPos| sim.world.get(p).is_some_and(|c| c.material == gas);
    // (A light that flickers is a flame; the one in hand is one unless it's
    // a safety lamp's, or a flashlight's.)
    let mut lit = Vec::new();
    for (tf, light, held) in &flames {
        let naked = if held { matches!(toggles.carry, crate::light::Carry::Torch | crate::light::Carry::Lantern) } else { light.flicker > 0.05 };
        if !naked {
            continue;
        }
        let at = tf.translation();
        let c = CellPos::from_world(at.x, at.y);
        if let Some(p) = (-FLAME_REACH..=FLAME_REACH).flat_map(|dy| (-FLAME_REACH..=FLAME_REACH).map(move |dx| c.offset(dx, dy))).find(|&p| is_gas(&sim, p)) {
            lit.push(p);
        }
    }
    // (Once lit, it's the sim's to burn: a flame tries again only after a
    // while.)
    if let Some(p) = lit.first()
        && tick >= *quiet
    {
        *quiet = tick + 30;
        info!("hazards: firedamp lit at ({}, {}) by a naked flame", p.x, p.y);
        for &p in &lit {
            sim.queue(platypus_sim::WorldEdit::Ignite { center: p, radius: 2 });
        }
    }
    // Sniffed: each cell of it in reach counts the more the nearer.
    if !tick.is_multiple_of(SNIFF_EVERY) {
        return;
    }
    let Ok(k) = player.single() else { return };
    let c = CellPos::from_world(k.body.pos.x, k.body.pos.y);
    let mut sum = 0.0;
    for dy in (-SNIFF_REACH..=SNIFF_REACH).step_by(SNIFF_STEP as usize) {
        for dx in (-SNIFF_REACH..=SNIFF_REACH).step_by(SNIFF_STEP as usize) {
            if is_gas(&sim, c.offset(dx, dy)) {
                sum += (1.0 - ((dx * dx + dy * dy) as f32).sqrt() / SNIFF_REACH as f32).max(0.0);
            }
        }
    }
    let now = (sum / SNIFF_FULL).min(1.0);
    damp.near += (now - damp.near) * 0.5;
}
