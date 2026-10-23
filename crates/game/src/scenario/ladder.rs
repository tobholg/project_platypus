//! `ladder` (a generated world, `PLATYPUS_MODE=play`, a long
//! `PLATYPUS_SCENARIO_SECS`): a bot plays the first rungs of the game from
//! the play kit, through real input (the cursor, the left button, A/D and
//! Space; crafting through the panel's request; a hotbar slot picked as a
//! player would, dragged up from the pack when it must be). It fells trees
//! for wood, makes a workbench, digs down, mines the ore it sees and camps
//! to smelt and make what it can: a furnace, an anvil, an iron pickaxe, a
//! mason's bench, a sawmill, a bed it sleeps in. It logs each rung as it's
//! reached (the time, how deep), and every 15 s where it is, what it has,
//! what hurt it: a playtest of the ladder's pacing (`ladder:` lines).
//! `PLATYPUS_LADDER_WOOD=n`: the wood it fells first (50 blocks);
//! `PLATYPUS_LADDER_SPEED=2`: the game twice as fast.

use std::collections::BTreeSet;

use bevy::prelude::*;
use platypus_sim::{CellPos, MaterialId, World};

use super::Scenario;
use crate::camera::CursorOverride;
use crate::craft::{CraftRequest, Crafting, Station};
use crate::creatures::player::LocalPlayer;
use crate::creatures::{Health, Kinematics};
use crate::hands::Hand;
use crate::hands::items::{HOTBAR, Inventory, Items};
use crate::progress::Progress;
use crate::world::SimWorld;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    #[default]
    Start,
    Wood,
    Dig,
    Ore,
    Camp,
    Done,
}

/// The ores it looks for, the likeliest wanted first.
const ORES: [&str; 9] = ["copper_ore", "iron_ore", "coal", "silver_ore", "gold_ore", "amethyst", "emerald", "ruby", "mithril_ore"];
/// What it makes when it can, in order (and how many in all).
const WANTS: [(&str, u32); 11] = [
    ("furnace", 1),
    ("copper_bar", 999),
    ("iron_bar", 999),
    ("anvil", 1),
    ("iron_pickaxe", 1),
    ("mason_bench", 1),
    ("sawmill", 1),
    ("bed", 1),
    ("door", 1),
    ("table", 1),
    ("chair", 1),
];
/// What it carries and sets down at a camp.
const STATIONS: [&str; 6] = ["workbench", "furnace", "anvil", "mason_bench", "sawmill", "bed"];

#[derive(Default)]
pub(super) struct Bot {
    phase: Phase,
    since: f32,
    prev_t: f32,
    step: u8,
    step_at: f32,
    target: Option<CellPos>,
    target_at: f32,
    skip: Vec<CellPos>,
    scan_at: f32,
    last: Vec2,
    still: f32,
    jump: f32,
    low: f32,
    low_at: f32,
    side: f32,
    next_log: f32,
    hp: f32,
    hurt: f32,
    deaths: u32,
    rungs: BTreeSet<String>,
    camps: u32,
    camp_end: f32,
    idle: f32,
    summed: bool,
    wood: u32,
    anchor: Vec2,
    moved_at: f32,
    shelf: bool,
    failed: Vec<String>,
    room: Vec<CellPos>,
    block_at: f32,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn ladder_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    crafting: Res<Crafting>,
    items: Option<Res<Items>>,
    mut player: Query<(&Kinematics, &mut Inventory, Option<&Progress>, &Health), With<LocalPlayer>>,
    stations: Query<(&Station, &Kinematics)>,
    mut hand: ResMut<Hand>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut asks: MessageWriter<CraftRequest>,
    mut virt: ResMut<Time<Virtual>>,
    them: Query<(&crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    checkpoint: Res<crate::respawn::Checkpoint>,
    mut bot: Local<Bot>,
) {
    if s.name != "ladder" {
        return;
    }
    let Some(items) = items else { return };
    let Ok((k, mut inv, progress, health)) = player.single_mut() else { return };
    let Some(progress) = progress else { return };
    let t = s.elapsed;
    let dt = (t - bot.prev_t).max(0.0);
    bot.prev_t = t;
    let p = k.body.pos;
    let feet = p - Vec2::Y * k.body.half.y;
    let world = &sim.world;
    let mats = world.materials();
    let depth = progress.deepest;
    for key in [KeyCode::KeyA, KeyCode::KeyD, KeyCode::Space] {
        keys.release(key);
    }
    mouse.release(MouseButton::Left);
    mouse.release(MouseButton::Right);
    let (mut aim, mut hold, mut walk) = (None::<Vec2>, false, 0.0f32);
    let made = |name: &str| progress.crafted.get(name).copied().unwrap_or(0);
    // (What ore's still wanted: copper and coal only till it has enough.)
    let wanted: Vec<&str> = ORES.iter().copied().filter(|&o| !(o == "copper_ore" && count(&inv, &items, o) + 3 * count(&inv, &items, "copper_bar") >= 24) && !(o == "coal" && count(&inv, &items, o) >= 8)).collect();
    let near = crate::craft::near(&crafting, p, &stations);

    // Hurt, and deaths (a scenario heals in place: counted, not undone).
    if bot.phase != Phase::Start {
        if health.hp < bot.hp {
            bot.hurt += bot.hp - health.hp;
            // (Who: the nearest creature, if one's close.)
            if bot.hp - health.hp >= 5.0 {
                let who = them.iter().map(|(c, ck)| (c.kind.clone(), ck.body.pos.distance(p))).filter(|(_, d)| *d < 90.0).min_by(|a, b| a.1.total_cmp(&b.1));
                info!("ladder: hurt {:.0} at t {t:.0}s, {depth} deep ({:?}; hp {:.0})", bot.hp - health.hp, who.map(|(k, d)| format!("{k} {d:.0} off")), health.hp);
            }
            if health.hp <= 0.0 && bot.hp > 0.0 {
                bot.deaths += 1;
                info!("ladder: DIED at t {t:.0}s, {depth} deep, at {:?} (the {}th time)", p.round(), bot.deaths);
            }
        }
    }
    bot.hp = health.hp;
    // Rungs: what it made, the milestones reached.
    for name in progress.crafted.keys().map(|c| format!("made {c}")).chain(progress.done.iter().map(|d| format!("milestone {d}"))) {
        if !bot.rungs.contains(&name) {
            info!("ladder: RUNG {name} at t {t:.0}s, {depth} deep");
            bot.rungs.insert(name);
        }
    }

    let phase = bot.phase;
    match phase {
        Phase::Start => {
            // (Once it's standing on the loaded ground.)
            if t < 2.0 || !k.loco.grounded() {
                return;
            }
            if let Some(speed) = std::env::var("PLATYPUS_LADDER_SPEED").ok().and_then(|v| v.parse::<f32>().ok()) {
                virt.set_relative_speed(speed);
            }
            info!("ladder: start at {:?}, carrying {}", p.round(), carried(&inv, &items));
            bot.last = p;
            bot.low = p.y;
            bot.side = 1.0;
            go(&mut bot, Phase::Wood, t);
        }
        Phase::Wood => {
            let want = std::env::var("PLATYPUS_LADDER_WOOD").ok().and_then(|v| v.parse().ok()).unwrap_or(50);
            if count(&inv, &items, "block:wood") >= want || t - bot.since > 240.0 {
                info!("ladder: felled {} blocks of wood in {:.0}s", count(&inv, &items, "block:wood"), t - bot.since);
                // Planks enough for a workbench, and the workbench (no station needed).
                if let (Some(planks), Some(bench)) = (crafting.recipe("block:planks"), crafting.recipe("workbench")) {
                    for _ in 0..3 {
                        asks.write(CraftRequest(planks));
                    }
                    asks.write(CraftRequest(bench));
                }
                go(&mut bot, Phase::Dig, t);
            } else {
                if t >= bot.scan_at || bot.target.is_none_or(|c| !is_either(world, c, &ids(mats, &["wood"]))) {
                    bot.scan_at = t + 0.5;
                    bot.target = nearest(world, p, &ids(mats, &["wood"]), 500, feet.y as i32 + 3, feet.y as i32 + 160, 2, &bot.skip, true);
                }
                // (A tree it gets no wood from in 15 s: another.)
                let wood = count(&inv, &items, "block:wood");
                if wood != bot.wood {
                    bot.wood = wood;
                    bot.target_at = t;
                }
                if let Some(c) = bot.target.filter(|_| t - bot.target_at > 15.0) {
                    info!("ladder: no wood from the tree at {:?} in 15 s; another", (c.x, c.y));
                    bot.skip.push(c);
                    bot.target = None;
                    bot.target_at = t;
                }
                if p.distance(bot.anchor) > 6.0 {
                    bot.anchor = p;
                    bot.moved_at = t;
                }
                match bot.target {
                    Some(c) => {
                        let at = centre(c);
                        if at.distance(p) > 40.0 {
                            walk = (at.x - p.x).signum();
                        }
                        // (Out of reach and getting nowhere, in a pit: a
                        // tunnel toward it with the pickaxe.)
                        let tool = if at.distance(p) > 46.0 && t - bot.moved_at > 2.5 { best_pick(&mut inv, &items) } else { slot_for(&mut inv, &items, "copper_axe") };
                        if let Some(i) = tool {
                            select(&mut hand, i);
                        }
                        aim = Some(at);
                        hold = at.distance(p) <= 46.0 || t - bot.moved_at > 2.5;
                        if t - bot.moved_at > 2.5 {
                            bot.jump = t + 0.2;
                        }
                    }
                    None => walk = bot.side,
                }
            }
        }
        Phase::Dig => {
            if let Some(i) = best_pick(&mut inv, &items) {
                select(&mut hand, i);
            }
            if p.y < bot.low - 2.0 {
                bot.low = p.y;
                bot.low_at = t;
            }
            if t - bot.low_at > 4.0 {
                info!("ladder: stuck digging at {depth} deep, {:?}: under it {}", p.round(), under(world, feet));
                bot.side = -bot.side;
                bot.low_at = t;
            }
            let diag = if t - bot.low_at > 2.0 { bot.side * 24.0 } else { 0.0 };
            aim = Some(p + Vec2::new(diag, -45.0));
            hold = true;
            if diag != 0.0 {
                walk = bot.side;
            }
            if t >= bot.scan_at {
                bot.scan_at = t + 0.5;
                if let Some(c) = nearest(world, p, &ids(mats, &wanted), 72, p.y as i32 - 64, p.y as i32 + 24, 2, &bot.skip, false) {
                    bot.target = Some(c);
                    bot.target_at = t;
                    go(&mut bot, Phase::Ore, t);
                }
            }
            if t - bot.camp_end > 20.0 && wants_camp(&inv, &items, &made, checkpoint.0.is_some()) {
                go(&mut bot, Phase::Camp, t);
            }
        }
        Phase::Ore => {
            let ores = ids(mats, &wanted);
            if bot.target.is_none_or(|c| !is(world, c, &ores)) {
                bot.target = nearest(world, p, &ores, 72, p.y as i32 - 64, p.y as i32 + 24, 2, &bot.skip, false);
                bot.target_at = t;
            }
            match bot.target {
                None => go(&mut bot, Phase::Dig, t),
                Some(c) if t - bot.target_at > 10.0 => {
                    bot.skip.push(c);
                    bot.target = None;
                }
                Some(c) => {
                    if let Some(i) = best_pick(&mut inv, &items) {
                        select(&mut hand, i);
                    }
                    let at = centre(c);
                    if (at.x - p.x).abs() > 12.0 {
                        walk = (at.x - p.x).signum();
                    }
                    aim = Some(at);
                    hold = true;
                }
            }
            if t - bot.camp_end > 20.0 && wants_camp(&inv, &items, &made, checkpoint.0.is_some()) {
                go(&mut bot, Phase::Camp, t);
            }
        }
        Phase::Camp => {
            let since = t - bot.step_at;
            match bot.step {
                // Room: a tunnel the body fits, each way.
                // A room, as a player makes one: whole blocks mined out in
                // precise mode (flat faces), five high and thirteen across
                // from the row its feet are in, the nearest first; a block
                // it can't get in 3 s is passed over.
                0 | 1 => {
                    let bs = platypus_sim::BLOCK;
                    if bot.room.is_empty() && bot.step == 0 {
                        let (fb, pb) = ((feet.y / bs as f32 + 0.25).floor() as i32, (p.x / bs as f32).floor() as i32);
                        bot.room = (fb..fb + 5).flat_map(|by| (pb - 6..=pb + 6).map(move |bx| CellPos::new(bx, by))).collect();
                        bot.step = 1;
                        bot.block_at = t;
                    }
                    let hand_at = p + Vec2::Y * 4.0;
                    let solid_in = |b: CellPos| platypus_sim::block_cells(b).any(|c| world.get(c).is_some_and(|c| matches!(mats.phys(c.material).kind, platypus_sim::Kind::Static | platypus_sim::Kind::Powder)));
                    let before = bot.room.len();
                    bot.room.retain(|&b| solid_in(b));
                    if bot.room.len() != before {
                        bot.block_at = t;
                    }
                    let centre = |b: CellPos| Vec2::new((b.x as f32 + 0.5) * bs as f32, (b.y as f32 + 0.5) * bs as f32);
                    bot.room.sort_by(|a, b| centre(*a).distance(hand_at).total_cmp(&centre(*b).distance(hand_at)));
                    if !bot.room.is_empty() && t - bot.block_at > 3.0 {
                        bot.room.remove(0);
                        bot.block_at = t;
                    }
                    match bot.room.first() {
                        Some(&b) => {
                            if let Some(i) = best_pick(&mut inv, &items) {
                                select(&mut hand, i);
                            }
                            hand.smart = false;
                            aim = Some(centre(b));
                            hold = true;
                        }
                        None => {
                            hand.smart = true;
                            info!("ladder: a room mined out at {:?} in {:.0}s", p.round(), t - bot.step_at);
                            bot.step = 2;
                            bot.step_at = t;
                            bot.shelf = false;
                        }
                    }
                }
                // Set down what it carries, one every 0.4 s: where there's
                // flat ground for it in reach, else on a shelf of blocks.
                2 => {
                    if since > 0.4 {
                        bot.step_at = t;
                        // (What wouldn't go down this camp is passed over.)
                        // (Only what the next things it wants are made at: they
                        // don't all fit in reach; and its bed, to sleep in.)
                        let needed: Vec<&str> = WANTS
                            .iter()
                            .filter(|(w, n)| made(w) < *n)
                            .filter_map(|(w, _)| crafting.recipe(w).and_then(|r| crafting.recipes[r].at.as_deref()))
                            .chain(["bed"])
                            .collect();
                        let todo = STATIONS.iter().find(|n| needed.contains(n) && count(&inv, &items, n) > 0 && !bot.failed.contains(&n.to_string()));
                        match todo.and_then(|n| crafting.station(n).map(|kind| (*n, kind))) {
                            Some((name, kind)) => {
                                let set: Vec<(Vec2, Vec2)> = stations.iter().map(|(_, sk)| (sk.body.pos, sk.body.half)).collect();
                                match spot(world, &crafting, kind, p, &set) {
                                    Some(at) => {
                                        if let Some(slot) = slot_for(&mut inv, &items, name) {
                                            select(&mut hand, slot);
                                            aim = Some(at);
                                            hold = true;
                                        }
                                        bot.idle += 1.0;
                                        if bot.idle > 4.0 {
                                            info!("ladder: the {name} wouldn't go down at {:?}", at.round());
                                            bot.failed.push(name.to_string());
                                            bot.idle = 0.0;
                                        }
                                    }
                                    None if !bot.shelf => {
                                        info!("ladder: no flat ground for the {name} at {:?}: a floor of blocks", p.round());
                                        bot.shelf = true;
                                        bot.step = 6;
                                        bot.step_at = t;
                                        bot.idle = 0.0;
                                    }
                                    None => {
                                        let mut why: std::collections::BTreeMap<String, u32> = Default::default();
                                        for dy in (-16..=16).step_by(4) {
                                            for dx in (-44..=44).step_by(4) {
                                                let tr = crafting.try_place(world, p + Vec2::new(dx as f32, dy as f32), kind);
                                                let k = match tr.at { Ok(_) if tr.feet().distance(p) > 46.0 => "far".to_string(), Ok(_) => "ok (overlaps)".into(), Err(w) => format!("{w:?}") };
                                                *why.entry(k).or_default() += 1;
                                            }
                                        }
                                        info!("ladder: nowhere for the {name} even on a floor of blocks, at {:?}: {why:?}", p.round());
                                        bot.failed.push(name.to_string());
                                        bot.idle = 0.0;
                                    }
                                }
                            }
                            None => {
                                bot.step = 3;
                                bot.idle = 0.0;
                            }
                        }
                    }
                }
                // A shelf: blocks along the row its feet are in, either side
                // of it (their tops flat), one every 0.15 s; then back to 2.
                // A floor laid: blocks into the row under its feet wherever
                // that row's top isn't whole (their tops flat), one every
                // 0.15 s; then back to 2.
                6 => {
                    let bs = platypus_sim::BLOCK;
                    let fb = (feet.y / bs as f32 + 0.25).floor() as i32 - 1;
                    let pb = (p.x / bs as f32).floor() as i32;
                    let holed = |bx: i32| (0..bs).any(|dx| !world.get(CellPos::new(bx * bs + dx, fb * bs + bs - 1)).is_some_and(|c| matches!(mats.phys(c.material).kind, platypus_sim::Kind::Static | platypus_sim::Kind::Powder)));
                    let row: Vec<i32> = (pb - 6..=pb + 6).filter(|&bx| holed(bx)).collect();
                    let block = ["block:stone", "block:dirt", "block:cobblestone"].into_iter().find(|n| count(&inv, &items, n) > 0);
                    match (row.first(), block) {
                        (Some(&bx), Some(b)) if since < 4.0 => {
                            if let Some(slot) = slot_for(&mut inv, &items, b) {
                                select(&mut hand, slot);
                                aim = Some(Vec2::new((bx as f32 + 0.5) * bs as f32, (fb as f32 + 0.5) * bs as f32));
                                hold = true;
                            }
                        }
                        _ => {
                            bot.step = 2;
                            bot.step_at = t;
                        }
                    }
                }
                // Make what it can, one a frame.
                3 => {
                    let wood = count(&inv, &items, "block:wood");
                    let planks = count(&inv, &items, "block:planks");
                    let keep = if made("sawmill") > 0 { 0 } else { 6 };
                    let mut asked = false;
                    if planks < 14 && wood > keep {
                        if let Some(r) = crafting.recipe("block:planks") {
                            asks.write(CraftRequest(r));
                            asked = true;
                        }
                    }
                    if !asked {
                        for (name, n) in WANTS {
                            let Some(r) = crafting.recipe(name) else { continue };
                            if made(name) < n && crafting.can_make(&crafting.recipes[r], &items, &inv, &near) {
                                asks.write(CraftRequest(r));
                                asked = true;
                                break;
                            }
                        }
                    }
                    if asked {
                        bot.idle = 0.0;
                    } else {
                        bot.idle += dt;
                    }
                    if bot.idle > 0.5 {
                        bot.idle = 0.0;
                        // A bed set down: sleep in it.
                        bot.step = if stations.iter().any(|(st, sk)| crafting.stations[st.kind].bed && sk.body.pos.distance(p) < 60.0) { 4 } else { 5 };
                        bot.step_at = t;
                    }
                }
                4 => {
                    if let Some((_, b)) = stations.iter().find(|(st, sk)| crafting.stations[st.kind].bed && sk.body.pos.distance(p) < 60.0) {
                        cursor.0 = Some(b.body.pos);
                        mouse.press(MouseButton::Right);
                    }
                    bot.step = 5;
                    bot.step_at = t;
                    return;
                }
                // Take it all back up (the pickaxe), unless it's done (a bed
                // slept in stays: it wakes there).
                _ => {
                    if checkpoint.0.is_some() && made("iron_pickaxe") > 0 {
                        info!("ladder: the ladder climbed: {}", summary(&bot, &inv, &items, depth, t));
                        go(&mut bot, Phase::Done, t);
                    } else if let Some((_, sk)) = stations.iter().filter(|(st, sk)| sk.body.pos.distance(p) < 64.0 && !(crafting.stations[st.kind].bed && checkpoint.0.is_some())).min_by(|a, b| a.1.body.pos.distance(p).total_cmp(&b.1.body.pos.distance(p))) {
                        if let Some(i) = best_pick(&mut inv, &items) {
                            select(&mut hand, i);
                        }
                        aim = Some(sk.body.pos);
                        hold = true;
                        if since > 20.0 {
                            info!("ladder: left a station behind at {:?}", sk.body.pos.round());
                            bot.camp_end = t;
                            go(&mut bot, Phase::Dig, t);
                        }
                    } else {
                        bot.camps += 1;
                        info!("ladder: camp {} done at t {t:.0}s ({:.0}s), {depth} deep: {}", bot.camps, t - bot.since, carried(&inv, &items));
                        bot.camp_end = t;
                        go(&mut bot, Phase::Dig, t);
                    }
                }
            }
        }
        Phase::Done => {}
    }

    // Walking: jump when it doesn't get anywhere.
    if walk != 0.0 {
        keys.press(if walk > 0.0 { KeyCode::KeyD } else { KeyCode::KeyA });
        if (p.x - bot.last.x).abs() < 0.2 {
            bot.still += dt;
        } else {
            bot.still = 0.0;
        }
        if bot.still > 0.3 {
            bot.jump = t + 0.4;
            bot.still = 0.0;
        }
    }
    if t < bot.jump {
        keys.press(KeyCode::Space);
    }
    bot.last = p;
    cursor.0 = aim;
    if hold {
        mouse.press(MouseButton::Left);
    }

    if t >= bot.next_log {
        bot.next_log = t + 15.0;
        info!("ladder: t {t:.0}s {:?} {depth} deep at {:?}, hp {:.0}/{:.0} | {}", bot.phase, p.round(), health.hp, health.max, carried(&inv, &items));
    }
    if t > s.duration - 0.6 && !bot.summed {
        bot.summed = true;
        info!("ladder: END {}", summary(&bot, &inv, &items, depth, t));
    }
}

fn go(bot: &mut Bot, phase: Phase, t: f32) {
    bot.phase = phase;
    bot.failed.clear();
    bot.room.clear();
    bot.since = t;
    bot.step = 0;
    bot.step_at = t;
    bot.idle = 0.0;
    bot.low_at = t;
}

/// Whether, with the stations it carries, it could make a new rung now
/// (smelting what ore it has).
fn wants_camp(inv: &Inventory, items: &Items, made: &dyn Fn(&str) -> u32, slept: bool) -> bool {
    let n = |name: &str| count(inv, items, name);
    let iron = n("iron_bar") + n("block:iron_ore") / 3;
    let copper = n("copper_bar") + n("block:copper_ore") / 3;
    (made("furnace") == 0 && n("block:stone") >= 12 && n("workbench") > 0)
        || (made("furnace") > 0 && made("anvil") == 0 && iron >= 5)
        || (made("anvil") > 0 && made("iron_pickaxe") == 0 && iron >= 8)
        || (made("furnace") > 0 && made("mason_bench") == 0 && copper >= 2 && n("block:stone") >= 20)
        || (made("furnace") > 0 && made("sawmill") == 0 && copper >= 2)
        || (made("sawmill") > 0 && made("bed") == 0)
        || (n("bed") > 0 && !slept)
}

/// Where to aim to set a station down: flat ground in reach with room
/// over it, clear of the stations already down.
fn spot(world: &World, crafting: &Crafting, kind: usize, p: Vec2, set: &[(Vec2, Vec2)]) -> Option<Vec2> {
    let (w, h) = crafting.stations[kind].size();
    let half = Vec2::new(w as f32, h as f32) / 2.0;
    let mut best: Option<(Vec2, f32)> = None;
    for dy in (-16..=16).step_by(4) {
        for dx in (-44..=44).step_by(2) {
            let aim = p + Vec2::new(dx as f32, dy as f32);
            let Some(feet) = crafting.place_spot(world, aim, kind) else { continue };
            let c = feet + Vec2::Y * half.y;
            if feet.distance(p) > 46.0 || set.iter().any(|(sc, sh)| (sc.x - c.x).abs() < sh.x + half.x && (sc.y - c.y).abs() < sh.y + half.y) {
                continue;
            }
            // (Packed from the left: what's set down after has room.)
            let d = c.x - half.x;
            if best.is_none_or(|(_, b)| d < b) {
                best = Some((aim, d));
            }
        }
    }
    best.map(|(a, _)| a)
}

/// How many it has (blocks as blocks).
fn count(inv: &Inventory, items: &Items, name: &str) -> u32 {
    items.id(name).map_or(0, |id| inv.count(id) / items.unit(id))
}

fn carried(inv: &Inventory, items: &Items) -> String {
    inv.slots.iter().flatten().map(|st| format!("{} {}", items.def(st.item).id.trim_start_matches("block:"), st.count / items.unit(st.item))).collect::<Vec<_>>().join(", ")
}

fn summary(bot: &Bot, inv: &Inventory, items: &Items, depth: i32, t: f32) -> String {
    format!("t {t:.0}s, {depth} deep, {} camps, hurt {:.0}, died {} times; rungs [{}]; carrying {}", bot.camps, bot.hurt, bot.deaths, bot.rungs.iter().cloned().collect::<Vec<_>>().join(", "), carried(inv, items))
}

/// A hotbar slot holding it (dragged up from the pack into the last slot
/// if it's only there).
fn slot_for(inv: &mut Inventory, items: &Items, name: &str) -> Option<usize> {
    let id = items.id(name)?;
    let i = inv.slots.iter().position(|s| s.is_some_and(|s| s.item == id))?;
    if i < HOTBAR {
        return Some(i);
    }
    inv.slots.swap(i, HOTBAR - 1);
    Some(HOTBAR - 1)
}

fn best_pick(inv: &mut Inventory, items: &Items) -> Option<usize> {
    slot_for(inv, items, "iron_pickaxe").or_else(|| slot_for(inv, items, "copper_pickaxe"))
}

fn select(hand: &mut Hand, slot: usize) {
    hand.bar = 0;
    hand.slot = slot;
    // (The bot mines a block at a time, as it was tuned to.)
    hand.area = false;
}

fn ids(mats: &platypus_sim::MaterialTable, names: &[&str]) -> Vec<MaterialId> {
    names.iter().filter_map(|n| mats.id(n)).collect()
}

fn is(world: &World, c: CellPos, of: &[MaterialId]) -> bool {
    world.get(c).is_some_and(|cell| of.contains(&cell.material))
}

/// In front or behind (most trees stand in the back layer: an axe reaches it).
fn is_either(world: &World, c: CellPos, of: &[MaterialId]) -> bool {
    is(world, c, of) || world.get_bg(c).is_some_and(|cell| of.contains(&cell.material))
}

fn centre(c: CellPos) -> Vec2 {
    Vec2::new(c.x as f32 + 0.5, c.y as f32 + 0.5)
}

/// The nearest cell of any of `of` within `rx` across and rows `y0..y1`
/// (`back`: in the back layer too).
#[allow(clippy::too_many_arguments)]
fn nearest(world: &World, from: Vec2, of: &[MaterialId], rx: i32, y0: i32, y1: i32, step: i32, skip: &[CellPos], back: bool) -> Option<CellPos> {
    let x0 = from.x as i32;
    let mut best: Option<(CellPos, f32)> = None;
    for y in (y0..y1).step_by(step as usize) {
        for x in (x0 - rx..x0 + rx).step_by(step as usize) {
            let c = CellPos::new(x, y);
            if !(if back { is_either(world, c, of) } else { is(world, c, of) }) || skip.iter().any(|s| (s.x - x).abs() < 6 && (s.y - y).abs() < 6) {
                continue;
            }
            let d = centre(c).distance(from);
            if best.is_none_or(|(_, b)| d < b) {
                best = Some((c, d));
            }
        }
    }
    best.map(|(c, _)| c)
}

fn under(world: &World, feet: Vec2) -> String {
    let c = CellPos::new(feet.x as i32, feet.y as i32 - 2);
    world.get(c).map_or("nothing loaded".into(), |cell| world.materials().def(cell.material).name.clone())
}
