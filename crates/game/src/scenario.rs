//! Scripted runs for measuring the real game, not just the headless sim.
//!
//! `PLATYPUS_SCENARIO=pan cargo run -p platypus --release`
//!
//! - `idle`       camera still
//! - `pan`        camera sweeps right at 150 cells/s (legacy "walking" was 83 tiles/s at 3 px)
//! - `avalanche`  big blobs of sand, water, lava and oil every half second, slow pan
//! - `run`        the player runs right through real input: holds D, jumps, dashes
//! - `tools`      scripted cursor: pickaxe, bomb, pour water and oil, ignite, melt rock
//! - `tree`       builds a wooden tree beside the player and sets it on fire
//! - `blast`      three bombs dropped down one shaft beside the player, from t = 2 s
//! - `fell`       cuts through the trunk of the nearest tree to the right at t = 2 s
//! - `burn`       sets the base of that tree alight at t = 2 s instead
//! - `strike`     lightning onto that tree at t = 2 s instead
//! - `acid`       pours acid into a glass basin beside the player, boils it at 2 s, lights the fumes at 3.6 s
//! - `rain`       lights the forest beside the player at 2 s, a storm over it at 3 s (F5), lightning at 6 s (F7)
//! - `swim`       a pool beside the player; oil on the player, set alight, then it walks into the water
//! - `night`      the surface at 23:00 (`dusk`: 18:15)
//! - `flood`      a big block of water released in a dug-out hall beside the player
//! - `cave`       a chamber dug under the player (lava and acid pools), flashlight and torch on,
//!   a torch planted, two glow sticks thrown (`PLATYPUS_NOBEAM=1`: no flashlight)
//! - `chest`      places a chest with real input, opens it (RMB), puts bombs in, closes it,
//!   mines it: the bombs and the chest come back
//! - `hands`      the hands through real input: digs down and sideways with the pickaxe,
//!   builds a wall with what it dug, chops at a tree with auto tool (Ctrl),
//!   plants a torch; logs the inventory
//! - `chestfall`  a chest beside the player at 1 s, the ground under it dug
//!   out at 2 s: it falls; a blast beside it at 4 s throws it, one on it at 5 s
//!   breaks it
//! - `zoom`       types + twice and − once (as characters, as a Norwegian
//!   keyboard would), then scrolls the wheel two notches down; logs the zoom
//!   and the hotbar slot
//! - `shroom`     cuts through the stem of the nearest giant mushroom at 2 s
//!   (spawn in a fungal hollow): it falls as a body; logs the bodies
//! - `drop`       stands still until 3 s, then holds S: on a platform (e.g. a
//!   crypt's entrance, `PLATYPUS_SPAWN_X` at a ruin) it drops through; logs
//!   the feet before and after
//! - `magic`      three orcs well to the right at 1 s; then each starter wand
//!   through real input, held: sparks at the nearest orc (1.5 s), a fireball
//!   at the ground ahead (3 s), acid lobbed up and over (4.5 s), the flame
//!   wand at the orcs (5.5 s), lightning at them (7 s); logs mana, the orcs'
//!   health, spells in flight, blasts and zaps each second
//! - `shock`      a pool dug beside the player (flat world), two orcs in its
//!   far end at 3.5 s, lightning at them at 3.9 s: logs the zap and their
//!   health
//! - `well`       (flat world) two orcs to the right at 1 s; the gravity wand
//!   (hotbar 2) held on the ground ahead from 1.5 s, lifting it; swept over
//!   the orcs (2.6 s: it can carry two), carried up (3 s), swung hard up
//!   and right and let go mid-swing (3.6–3.75 s): thrown; logs what it
//!   holds, whom it carries, mana and the orcs (fall damage when they land)
//! - `force`      (flat world) a sand pile and two orcs to the right at 1 s;
//!   the force wand (hotbar 2) pushing toward them from 1.5 s, then pulling
//!   (right button) from 3 s, then pushing straight down (5–6 s: the
//!   recoil lifts the player); logs the orcs' distance, the player's height
//!   and mana
//! - `wellwater`  (flat world) a pool dug and filled beside the player; the
//!   gravity wand lifts it and lets go, twice; logs all the water there is
//!   (in cells, in flight, held) each half second: it should stay the same
//! - `splash`     (flat world) a pool beside the player, a pit of oil past
//!   it; a fireball lobbed into it (2 s: doused, a steam blast),
//!   one fired flat across it (3.5 s: skips, then doused), a spark bolt and
//!   an acid arrow into it (5, 6 s: plunge), a fireball lobbed onto the oil
//!   (7 s: alight), frost onto the pool (8.5 s: ice); logs water, steam,
//!   ice, oil and burning (`RUST_LOG=warn,platypus::magic=debug` traces the
//!   spells)
//! - `airjump`    lifts the player 160 cells at 1 s; it falls and air jumps
//!   (a cloud) just above the ground: no fall damage (`PLATYPUS_NOSAVE=1`:
//!   no air jump, it hurts; `PLATYPUS_NIGHT=1`: at night); logs height and
//!   health
//! - `critters`   (flat world) a rabbit, a bird and a frog placed 60–90 cells
//!   to the right at 1 s; the player walks at them from 2 s: they should hop
//!   and fly away; logs where the critters near the player are
//! - `editor`     (`PLATYPUS_WORLD=arena`, `PLATYPUS_EDIT_DIR` = a folder
//!   with copies of sprites: it WRITES them) opens the art editor, paints
//!   three pixels (9, 14–16) of the first thing in the first file in one stroke with
//!   the real pointer (a pose paints its first layer's part), logs what
//!   changed on disk; Ctrl+Z; logs whether the file is back as it was
//! - `webs`       (`PLATYPUS_WORLD=arena`) a walk right through open air, then
//!   through a thicket of cobweb; logs how far each got in 1.5 s
//! - `nest`       (a generated world, `small` is quickest) the player put in
//!   the spider nest nearest the start, a torch in hand; logs what's about
//!   after 3 s
//! - `underground` (`PLATYPUS_WORLD=arena`) each underground enemy in turn
//!   against the player (standing still): a spider, two slimes, two vampire
//!   bats, a skeleton, an egg sac; logs the health each phase cost and what
//!   they did; then the player on top of a column and a spider at its foot
//!   (logs how high it climbed)
//! - `life`       (`PLATYPUS_WORLD=arena`; try `PLATYPUS_HOUR=22`) fireflies
//!   over the floor, fish in the pool, bats in the air above; logs
//!   after 5 s whether each is still where it lives
//! - `crossing`   (`PLATYPUS_WORLD=arena`) a stream of sand poured 30 cells
//!   ahead, walked through (logs how far the player got); then a pit dug
//!   with water 12 cells below its rim, the player put in it, swimming up
//!   and jumping for the bank (logs whether it got out)
//! - `held`       (`PLATYPUS_WORLD=arena`) each tool in the hand in turn: the
//!   pickaxe into the floor (logs cells dug and swings), the torch, the
//!   spark wand at the first dummy, a bomb thrown, the axe swung
//! - `warband`    (`PLATYPUS_WORLD=arena`) O (the dev action) 120 cells off at
//!   1 s; logs what stands there at 3 s (a troll, three orcs, two archers)
//! - `archery`    (`PLATYPUS_WORLD=arena`) the bow (hotbar 2, slot 9): a full
//!   draw at the first dummy, a short one into the floor, one down through a
//!   lava puddle put behind (logs whether it burns); walks over the stuck arrows (logs arrows before and
//!   after), then an orc archer put 110 cells off shoots back (logs hp)
//! - `fight`      (`PLATYPUS_WORLD=arena`) the shortsword against an orc put
//!   40 cells off, held at it from 1.5 s; a troll put 70 cells off at 5 s;
//!   logs both sides' health, swings and staggers twice a second
//! - `melee`      (`PLATYPUS_WORLD=arena`) the shortsword (hotbar 2, slot 7)
//!   held down at the first dummy for 1.5 s, then the longsword (slot 8):
//!   logs hits, damage and stamina; a jump over the dummy striking down
//!   (logs the pogo); a blast on the player mid-dodge and one without (logs
//!   what each cost)
//! - `wands`      (`PLATYPUS_WORLD=arena`) the spark wand into the floor (logs
//!   the cells it broke) and at a sandbag put 40 cells off (logs how far
//!   it went), the
//!   flame wand at the dummies 60 and 120 cells off (logs what each took)
//! - `arena`      (`PLATYPUS_WORLD=arena`) overlays on; the spark wand at the
//!   first dummy, a fireball at the second; an orc picked and spawned with O;
//!   paused at 5 s and stepped three times (logs the sim ticks: 3), then a
//!   quarter speed from 6 s (logs ticks a second: ~15); logs the dummies'
//!   readouts
//! - `inventory`  opens the inventory screen (Esc) at 1 s, switches to the
//!   second hotbar (X) at 1.5 s, hovers the spark wand at 2 s (its tooltip;
//!   this moves the real mouse pointer), drags it to hotbar 3 at 2.6–3 s;
//!   logs where it ended up
//! - `gear`       (`PLATYPUS_WORLD=arena`) a blast beside the player with
//!   nothing on (logs what it cost), then a full iron set and a ring put on
//!   (logs its stats) and the same blast (logs what it cost now); the
//!   inventory opened, a leather jerkin in the pack hovered (its tooltip,
//!   against the chainmail worn)
//!
//! Prints one line per second and a summary, then exits.
//! `PLATYPUS_SCREENSHOT=out.png` saves the window one second before the end
//! (`PLATYPUS_OFFSCREEN=1`: what the camera draws, without the window: works
//! with the screen locked).

use bevy::input::InputSystems;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use platypus_sim::{CellPos, WorldEdit};

use crate::actors::Kinematics;
use crate::actors::player::LocalPlayer;
use crate::actors::spawn::find_ground;
use crate::camera::{CursorOverride, MainCamera};
use crate::props::spawn_bomb;
use crate::tools::{Toolbelt, ToolsConfig};
use crate::debug::FrameStats;
use crate::world::{SimMetrics, SimWorld};

pub struct ScenarioPlugin;

#[derive(Resource)]
struct Scenario {
    name: String,
    elapsed: f32,
    duration: f32,
    next_report: f32,
    next_drop: f32,
    reports: Vec<(f32, f32, f32)>,
    screenshot: Option<String>,
}

impl Plugin for ScenarioPlugin {
    fn build(&self, app: &mut App) {
        let Ok(name) = std::env::var("PLATYPUS_SCENARIO") else { return };
        let duration = std::env::var("PLATYPUS_SCENARIO_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(20.0);
        app.insert_resource(Scenario { name, elapsed: 0.0, duration, next_report: 2.0, next_drop: 1.0, reports: Vec::new(), screenshot: std::env::var("PLATYPUS_SCREENSHOT").ok() })
            .add_systems(Update, run)
            // Inject input where real input arrives: after Bevy reads devices,
            // before anything reads the cursor or buttons.
            .add_systems(PreUpdate, tools_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, editor_script.after(InputSystems).before(crate::editor::capture))
            .add_systems(Update, (warband_script, life_script, underground_script, nest_script))
            .add_systems(PreUpdate, webs_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, crossing_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, held_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, gear_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, (tree_script, blast_script, fell_script, acid_script, rain_script, swim_script, dark_script, flood_script))
            .add_systems(PreUpdate, (hands_script, chest_script, drop_script, chestfall_script, zoom_script, shroom_script, magic_script, shock_script, inventory_script, well_script, force_script, wellwater_script, splash_script, airjump_script, critters_script, arena_script, wands_script, melee_script, fight_script, archery_script).after(InputSystems).before(crate::camera::track_cursor));
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut s: ResMut<Scenario>,
    stats: Res<FrameStats>,
    metrics: Res<SimMetrics>,
    mut sim: ResMut<SimWorld>,
    mut cam: Single<&mut Transform, With<MainCamera>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
    offscreen: Option<Res<crate::camera::Offscreen>>,
) {
    let dt = time.delta_secs().min(0.1);
    s.elapsed += dt;
    let pan = match s.name.as_str() {
        "pan" => 150.0,
        "avalanche" => 20.0,
        _ => 0.0,
    };
    cam.translation.x += pan * dt;

    if s.name == "run" {
        // Real key presses through the input system, not a shortcut into the player.
        keys.press(KeyCode::KeyD);
        let phase = s.elapsed % 1.4;
        if phase < 0.35 { keys.press(KeyCode::Space) } else { keys.release(KeyCode::Space) }
        let dash = s.elapsed % 2.3;
        if (0.5..0.55).contains(&dash) { keys.press(KeyCode::ShiftLeft) } else { keys.release(KeyCode::ShiftLeft) }
    }

    if s.name == "avalanche" && s.elapsed >= s.next_drop {
        s.next_drop += 0.5;
        let names = ["sand", "water", "lava", "oil", "gravel"];
        let k = (s.elapsed * 2.0) as usize;
        for i in 0..4 {
            let name = names[(k + i) % names.len()];
            let Some(material) = sim.materials().id(name) else { continue };
            let center = CellPos::new((cam.translation.x - 240.0 + i as f32 * 160.0) as i32, (cam.translation.y + 120.0) as i32);
            sim.queue(WorldEdit::Paint { center, radius: 22, material, overwrite: false });
        }
    }

    // The first two seconds are startup (initial chunk loads); skip them.
    if s.elapsed >= s.next_report {
        s.next_report += 1.0;
        let tick_ms = metrics.tick_time_avg.as_secs_f32() * 1e3;
        s.reports.push((stats.avg_ms, stats.worst_ms, tick_ms));
        println!(
            "scenario={} t={:>4.1}s frame_avg={:.2}ms frame_worst={:.2}ms sim_tick={:.3}ms active_chunks={} loaded={} bodies={}",
            s.name,
            s.elapsed,
            stats.avg_ms,
            stats.worst_ms,
            tick_ms,
            metrics.last.active_chunks,
            sim.world.loaded_count(),
            sim.world.bodies().len()
        );
    }
    if s.elapsed >= s.duration - 1.0
        && let Some(path) = s.screenshot.take()
    {
        let shot = match &offscreen {
            Some(o) => Screenshot::image(o.0.clone()),
            None => Screenshot::primary_window(),
        };
        commands.spawn(shot).observe(save_to_disk(path));
    }
    if s.elapsed >= s.duration {
        let n = s.reports.len().max(1) as f32;
        let avg = s.reports.iter().map(|r| r.0).sum::<f32>() / n;
        let worst = s.reports.iter().map(|r| r.1).fold(0.0, f32::max);
        let tick = s.reports.iter().map(|r| r.2).sum::<f32>() / n;
        println!("SUMMARY scenario={} frame_avg={avg:.2}ms frame_worst={worst:.2}ms sim_tick_avg={tick:.3}ms", s.name);
        exit.write(AppExit::Success);
    }
}

/// Drives the toolbelt like a player would: real mouse buttons and number keys,
/// with the cursor pointed by script.
#[allow(clippy::too_many_arguments)]
fn tools_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut belt: ResMut<Toolbelt>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut started: Local<Option<f32>>,
    mut dev: ResMut<crate::hands::DevTools>,
) {
    if s.name != "tools" {
        return;
    }
    dev.0 = true;
    let Ok(p) = player.single() else { return };
    let t0 = *started.get_or_insert(s.elapsed);
    let t = s.elapsed - t0;
    // Aim relative to the ground surface dx cells away (dy > 0 above it, < 0 into it).
    let at = |dx: f32, dy: f32| {
        let x = p.body.pos.x + dx;
        let ground = find_ground(&sim.world, x as i32, p.body.pos.y as i32 + 60, 300)? as f32;
        Some(Vec2::new(x, ground + dy))
    };
    let hold = |mouse: &mut ButtonInput<MouseButton>, on: bool| {
        if on { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    };
    for k in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit6] {
        keys.release(k);
    }
    match t {
        t if t < 0.5 => {}
        t if t < 2.5 => {
            keys.press(KeyCode::Digit1);
            cursor.0 = at(30.0, -3.0);
            hold(&mut mouse, true);
        }
        t if t < 2.7 => hold(&mut mouse, false),
        t if t < 2.8 => {
            keys.press(KeyCode::Digit2);
            cursor.0 = at(70.0, 2.0);
            hold(&mut mouse, true);
        }
        t if t < 5.0 => hold(&mut mouse, false),
        t if t < 6.2 => {
            keys.press(KeyCode::Digit3);
            if let Some(w) = sim.materials().id("water") {
                belt.material = w;
            }
            cursor.0 = at(-60.0, 40.0);
            hold(&mut mouse, true);
        }
        t if t < 7.2 => {
            if let Some(o) = sim.materials().id("oil") {
                belt.material = o;
            }
            cursor.0 = at(-60.0, 40.0);
        }
        t if t < 8.2 => hold(&mut mouse, false),
        t if t < 8.8 => {
            keys.press(KeyCode::Digit4);
            cursor.0 = at(-60.0, 3.0);
            hold(&mut mouse, true);
        }
        t if t < 9.0 => hold(&mut mouse, false),
        t if t < 11.5 => {
            // Melt a hole into the rock next to the player.
            keys.press(KeyCode::Digit6);
            cursor.0 = at(25.0, -6.0);
            hold(&mut mouse, true);
        }
        _ => {
            hold(&mut mouse, false);
            cursor.0 = at(0.0, 70.0);
        }
    }
}

/// Build a branching wooden tree next to the player, then light its base.
fn tree_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut state: Local<(Option<f32>, u8, Vec2)>,
) {
    if s.name != "tree" {
        return;
    }
    let Ok(p) = player.single() else { return };
    let t0 = *state.0.get_or_insert(s.elapsed);
    let t = s.elapsed - t0;
    let Some(wood) = sim.materials().id("wood") else { return };
    if state.1 == 0 && t > 0.3 {
        let x = p.body.pos.x + 70.0;
        let Some(ground) = find_ground(&sim.world, x as i32, p.body.pos.y as i32 + 60, 300) else { return };
        let base = Vec2::new(x, ground as f32);
        state.2 = base;
        let mut line = |a: Vec2, b: Vec2, r: i32| {
            let n = a.distance(b).ceil() as i32;
            for i in 0..=n {
                let q = a.lerp(b, i as f32 / n.max(1) as f32);
                sim.queue(WorldEdit::Paint { center: CellPos::from_world(q.x, q.y), radius: r, material: wood, overwrite: true });
            }
        };
        let top = base + Vec2::new(0.0, 70.0);
        line(base, top, 2);
        for (from, dir) in [(40.0, -1.0), (52.0, 1.0), (62.0, -1.0), (30.0, 1.0)] {
            let start = base + Vec2::new(0.0, from);
            let end = start + Vec2::new(dir * 26.0, 12.0);
            line(start, end, 1);
            for k in 0..3 {
                let twig = start.lerp(end, 0.4 + k as f32 * 0.25);
                line(twig, twig + Vec2::new(dir * 3.0, 7.0), 0);
            }
        }
        state.1 = 1;
    }
    if state.1 == 1 && t > 1.2 {
        let base = state.2;
        sim.queue(WorldEdit::Ignite { center: CellPos::from_world(base.x, base.y + 4.0), radius: 3 });
        state.1 = 2;
    }
}

fn blast_script(
    mut commands: Commands,
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    tools: Res<ToolsConfig>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut shaft: Local<Option<(f32, i32)>>,
    mut dropped: Local<u32>,
) {
    // Three bombs dropped down the same shaft, each after the last went off:
    // with no chain reaction they dig instead of going off together.
    let due = 2.0 + *dropped as f32 * (tools.bomb.fuse + 0.4);
    if s.name != "blast" || *dropped >= 3 || s.elapsed < due {
        return;
    }
    let Ok(p) = player.single() else { return };
    let (x, top) = *shaft.get_or_insert_with(|| {
        let x = p.body.pos.x + 70.0;
        (x, find_ground(&sim.world, x as i32, p.body.pos.y as i32 + 60, 300).unwrap_or(p.body.pos.y as i32) + 12)
    });
    spawn_bomb(&mut commands, Vec2::new(x, top as f32), Vec2::ZERO, tools.bomb.clone());
    *dropped += 1;
}

fn fell_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut done: Local<bool>) {
    if !matches!(s.name.as_str(), "fell" | "burn" | "strike") || *done || s.elapsed < 2.0 {
        return;
    }
    let Ok(p) = player.single() else { return };
    let (px, py) = (p.body.pos.x as i32, p.body.pos.y as i32);
    // The first background wood 20 cells above the ground (clear of tall
    // grass, below the branches): a trunk.
    for x in px + 30..px + 260 {
        let Some(ground) = find_ground(&sim.world, x, py + 80, 300) else { continue };
        let at = CellPos::new(x, ground + 20);
        let wood = |x: i32| {
            let q = CellPos::new(x, at.y);
            sim.world.get_bg(q).is_some_and(|b| !b.is_air() && sim.world.materials().phys(b.material).kind == platypus_sim::Kind::Static)
        };
        if !wood(x) {
            continue;
        }
        // Both edges: the search may start inside a trunk.
        let left = x - (1..40).take_while(|&d| wood(x - d)).count() as i32;
        let width = (left..left + 60).take_while(|&x| wood(x)).count() as i32;
        let center = CellPos::new(left + width / 2, at.y);
        if s.name == "burn" {
            sim.queue(WorldEdit::Ignite { center: CellPos::new(center.x, ground + 3), radius: width / 2 + 2 });
        } else if s.name == "strike" {
            sim.queue(WorldEdit::Lightning { x: center.x, from_y: py + 300 });
        } else {
            // Twice: a dig clears the playfield first where anything stands in front.
            for _ in 0..2 {
                sim.queue(WorldEdit::Dig { center, radius: width / 2 + 3, max_hardness: 200 });
            }
        }
        info!("{}: trunk {width} wide at {center:?}, player at {px}", s.name);
        *done = true;
        return;
    }
}

fn acid_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut step: Local<u8>) {
    if s.name != "acid" {
        return;
    }
    let Ok(p) = player.single() else { return };
    let x = p.body.pos.x as i32 + 60;
    let Some(ground) = find_ground(&sim.world, x, p.body.pos.y as i32 + 60, 300) else { return };
    let at = CellPos::new(x, ground + 4);
    match *step {
        0 if s.elapsed > 1.0 => {
            // A glass basin (acid eats dirt and stone, not glass), then the acid.
            let (Some(glass), Some(acid)) = (sim.materials().id("glass"), sim.materials().id("acid")) else { return };
            sim.queue(WorldEdit::Paint { center: at.offset(0, -10), radius: 14, material: glass, overwrite: true });
            sim.queue(WorldEdit::Dig { center: at.offset(0, -2), radius: 9, max_hardness: 40 });
            sim.queue(WorldEdit::Paint { center: at.offset(0, 2), radius: 8, material: acid, overwrite: false });
            *step = 1;
        }
        1 if s.elapsed > 2.0 => {
            sim.queue(WorldEdit::Heat { center: at, radius: 12, amount: 250 });
            *step = 2;
        }
        2 if s.elapsed > 3.6 => {
            sim.queue(WorldEdit::Ignite { center: at.offset(0, 20), radius: 6 });
            *step = 3;
        }
        _ => {}
    }
}

fn rain_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut step: Local<u8>) {
    if s.name != "rain" {
        return;
    }
    let Ok(p) = player.single() else { return };
    let x = p.body.pos.x as i32;
    match *step {
        0 if s.elapsed > 2.0 => {
            if let Some(ground) = find_ground(&sim.world, x + 80, p.body.pos.y as i32 + 60, 300) {
                sim.queue(WorldEdit::Ignite { center: CellPos::new(x + 80, ground + 2), radius: 6 });
            }
            *step = 1;
        }
        // What F5 does.
        1 if s.elapsed > 3.0 => {
            sim.queue(WorldEdit::Weather { x, radius: 400, storm: true });
            *step = 2;
        }
        // What F7 does, onto a spot left of the player.
        2 if s.elapsed > 6.0 => {
            sim.queue(WorldEdit::Lightning { x: x - 60, from_y: p.body.pos.y as i32 + 200 });
            *step = 3;
        }
        _ => {}
    }
}

fn swim_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut pool: Local<Option<(i32, i32)>>,
    mut step: Local<u8>,
) {
    if s.name != "swim" {
        return;
    }
    let Ok(p) = player.single() else { return };
    let (px, py) = (p.body.pos.x as i32, p.body.pos.y as i32);
    let &mut (x, ground) = pool.get_or_insert_with(|| (px + 40, find_ground(&sim.world, px + 40, py + 60, 300).unwrap_or(py)));
    match *step {
        0 if s.elapsed > 0.5 => {
            let Some(glass) = sim.materials().id("glass") else { return };
            // A glass-lined pit, 28 wide and 30 deep, full of water.
            for dx in (-16..=16).step_by(2) {
                sim.queue(WorldEdit::Paint { center: CellPos::new(x + dx, ground - 18), radius: 16, material: glass, overwrite: true });
            }
            for dx in (-14..=14).step_by(2) {
                sim.queue(WorldEdit::Dig { center: CellPos::new(x + dx, ground - 14), radius: 14, max_hardness: 200 });
            }
            *step = 1;
        }
        1 if s.elapsed > 0.6 => {
            if let Some(water) = sim.materials().id("water") {
                for dx in (-12..=12).step_by(4) {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x + dx, ground - 12), radius: 12, material: water, overwrite: false });
                }
            }
            *step = 2;
        }
        2 if s.elapsed > 2.0 => {
            if let Some(oil) = sim.materials().id("oil") {
                sim.queue(WorldEdit::Paint { center: CellPos::new(px, py + 14), radius: 4, material: oil, overwrite: false });
            }
            *step = 3;
        }
        3 if s.elapsed > 2.6 => {
            sim.queue(WorldEdit::Ignite { center: CellPos::new(px, py), radius: 6 });
            *step = 4;
        }
        4 if s.elapsed > 3.2 => keys.press(KeyCode::KeyD),
        _ => {}
    }
    if *step == 4 && px > x {
        keys.release(KeyCode::KeyD);
    }
}

#[allow(clippy::too_many_arguments)]
fn dark_script(
    mut commands: Commands,
    lights: Res<crate::light::LightSettings>,
    torch_art: Option<Res<crate::light::TorchArt>>,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut day: ResMut<crate::light::Daylight>,
    mut toggles: ResMut<crate::light::LightToggles>,
    mut cursor: ResMut<CursorOverride>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut done: Local<bool>,
) {
    if !matches!(s.name.as_str(), "night" | "cave" | "dusk") || *done || s.elapsed < 1.0 {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    // 23:00, or 18:15 for dusk (the day starts at the configured hour; skip
    // the difference).
    let hour = if s.name == "dusk" { 18.25 } else { 23.0 };
    day.skipped = (hour - day.time * 24.0).rem_euclid(24.0);
    if s.name == "cave" {
        // A chamber 90 below the surface, a lava pool on one side, acid on the
        // other; the player on its floor with the flashlight on.
        let x = k.body.pos.x as i32;
        let c = CellPos::new(x, k.body.pos.y as i32 - 100);
        let (Some(lava), Some(acid), Some(glass)) = (sim.materials().id("lava"), sim.materials().id("acid"), sim.materials().id("glass")) else { return };
        for dx in (-50..=50).step_by(10) {
            sim.queue(WorldEdit::Dig { center: c.offset(dx, (dx.abs() / 6) - 4), radius: 22, max_hardness: 250 });
        }
        sim.queue(WorldEdit::Paint { center: c.offset(62, -26), radius: 9, material: lava, overwrite: true });
        sim.queue(WorldEdit::Paint { center: c.offset(-58, -30), radius: 10, material: glass, overwrite: true });
        sim.queue(WorldEdit::Dig { center: c.offset(-58, -26), radius: 7, max_hardness: 250 });
        sim.queue(WorldEdit::Paint { center: c.offset(-58, -25), radius: 6, material: acid, overwrite: false });
        k.body.pos = Vec2::new(c.x as f32, c.y as f32 - 8.0);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
        toggles.carry = if std::env::var("PLATYPUS_NOBEAM").is_err() { crate::light::Carry::BigBeam } else { crate::light::Carry::Torch };
        cursor.0 = Some(k.body.pos + Vec2::new(90.0, -10.0));
        // A torch planted to the left, glow sticks thrown both ways.
        if let Some(art) = torch_art.as_deref() {
            crate::light::plant_torch(&mut commands, k.body.pos + Vec2::new(-40.0, -k.body.half.y), &lights, art);
        }
        let s = lights.glowstick.strength;
        crate::props::spawn_glowstick(&mut commands, k.body.pos + Vec2::new(-20.0, 4.0), Vec2::new(-60.0, 40.0), [0.25 * s, s, 0.45 * s], 90.0);
        crate::props::spawn_glowstick(&mut commands, k.body.pos + Vec2::new(30.0, 4.0), Vec2::new(60.0, 40.0), [0.2 * s, 0.55 * s, 1.1 * s], 90.0);
    }
    *done = true;
}

fn flood_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut step: Local<u8>, mut at: Local<Option<CellPos>>) {
    if s.name != "flood" {
        return;
    }
    let Ok(p) = player.single() else { return };
    let c = *at.get_or_insert_with(|| CellPos::new(p.body.pos.x as i32 + 20, p.body.pos.y as i32 - 170));
    match *step {
        // A hall 200 wide, 70 high, with stone floor; then a block of water
        // 50 wide and 60 high at its left end.
        0 if s.elapsed > 0.5 => {
            let (Some(stone), Some(water)) = (sim.materials().id("stone"), sim.materials().id("water")) else { return };
            for x in (-110..=110).step_by(6) {
                for y in (-6..=76).step_by(6) {
                    sim.queue(WorldEdit::Dig { center: c.offset(x, y), radius: 5, max_hardness: 250 });
                }
            }
            for x in (-110..=110).step_by(4) {
                sim.queue(WorldEdit::Paint { center: c.offset(x, -10), radius: 4, material: stone, overwrite: true });
            }
            *step = 1;
            let _ = water;
        }
        1 if s.elapsed > 0.8 => {
            let Some(water) = sim.materials().id("water") else { return };
            for x in (-100..=-55).step_by(3) {
                for y in (-3..=60).step_by(3) {
                    sim.queue(WorldEdit::Paint { center: c.offset(x, y), radius: 2, material: water, overwrite: false });
                }
            }
            *step = 2;
        }
        _ => {}
    }
}

/// The hands through real input (keys, mouse, a scripted cursor).
#[allow(clippy::too_many_arguments)]
fn chestfall_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut commands: Commands,
    mut chests: ResMut<crate::hands::chests::Chests>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    found: Query<&Kinematics, With<crate::hands::chests::Chest>>,
    mut step: Local<u8>,
    mut feet: Local<Option<Vec2>>,
) {
    if s.name != "chestfall" {
        return;
    }
    let Ok(k) = player.single() else { return };
    let chest_y = || found.iter().map(|c| c.body.bottom()).next();
    match *step {
        0 if s.elapsed > 1.0 => {
            // Somewhere to the side with room for it.
            let spot = (20..120).step_by(6).flat_map(|dx| [dx, -dx]).find_map(|dx| (-30..30).step_by(6).find_map(|dy| crate::hands::chests::place_spot(&sim.world, k.body.pos + Vec2::new(dx as f32, dy as f32))));
            let Some(at) = spot else { return };
            chests.spawn_placed(&mut commands, at);
            *feet = Some(at);
            *step = 1;
        }
        1 if s.elapsed > 1.9 => {
            info!("chestfall: chest standing at {:?}", chest_y());
            let at = feet.expect("placed");
            sim.queue(WorldEdit::Dig { center: CellPos::new(at.x as i32, at.y as i32 - 14), radius: 14, max_hardness: 255 });
            *step = 2;
        }
        2 if s.elapsed > 4.0 => {
            info!("chestfall: chest now at {:?}, centre {:?}", chest_y(), found.iter().next().map(|c| c.body.pos));
            // A small blast beside it: thrown, not broken; then a big one on it.
            let c = found.iter().next().map(|c| c.body.pos).expect("a chest");
            sim.queue(WorldEdit::Explode { center: CellPos::new(c.x as i32 + 12, c.y as i32), radius: 5, power: 60 });
            *step = 3;
        }
        3 if s.elapsed > 5.0 => {
            info!("chestfall: after a blast beside it: {:?}", found.iter().next().map(|c| c.body.pos));
            if let Some(c) = found.iter().next().map(|c| c.body.pos) {
                sim.queue(WorldEdit::Explode { center: CellPos::new(c.x as i32, c.y as i32), radius: 14, power: 100 });
            }
            *step = 4;
        }
        4 if s.elapsed > 6.0 => {
            info!("chestfall: after a blast on it: {} chests", found.iter().count());
            *step = 5;
        }
        _ => {}
    }
}

fn zoom_script(
    s: Res<Scenario>,
    mut chars: ResMut<ButtonInput<bevy::input::keyboard::Key>>,
    mut scroll: ResMut<bevy::input::mouse::AccumulatedMouseScroll>,
    zoom: Res<crate::camera::Zoom>,
    hand: Res<crate::hands::Hand>,
    mut step: Local<u8>,
) {
    use bevy::input::keyboard::Key;
    if s.name != "zoom" {
        return;
    }
    chars.release_all();
    let t = s.elapsed;
    let at = |k: u8| *step == k && t > 1.0 + k as f32 * 0.4;
    if at(0) {
        info!("zoom: starts at {} px/cell, slot {}", zoom.0, hand.slot);
        chars.press(Key::Character("+".into()));
        *step = 1;
    } else if at(1) {
        chars.press(Key::Character("+".into()));
        *step = 2;
    } else if at(2) {
        info!("zoom: after + +: {} px/cell", zoom.0);
        chars.press(Key::Character("-".into()));
        *step = 3;
    } else if at(3) {
        info!("zoom: after -: {} px/cell", zoom.0);
        scroll.delta.y = -2.0;
        *step = 4;
    } else if at(4) {
        info!("zoom: after two wheel notches down: slot {}", hand.slot);
        *step = 5;
    }
}

fn shroom_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut step: Local<u8>) {
    if s.name != "shroom" {
        return;
    }
    let Ok(k) = player.single() else { return };
    match *step {
        0 if s.elapsed > 2.0 => {
            let Some(stem) = sim.materials().id("mushroom_stem") else { return };
            // The nearest stem cell with open air in front, a little up it.
            let p = CellPos::from_world(k.body.pos.x, k.body.pos.y);
            let found = (0..200).flat_map(|r: i32| (-r..=r).flat_map(move |dx| [(dx, r), (dx, -r), (r, dx), (-r, dx)])).map(|(dx, dy)| p.offset(dx, dy)).find(|&q| {
                sim.world.get_bg(q).is_some_and(|c| c.material == stem) && sim.world.get_bg(q.offset(0, -6)).is_some_and(|c| c.material == stem) && sim.world.get(q).is_some_and(|c| c.is_air())
            });
            let Some(at) = found else {
                info!("shroom: no mushroom near");
                *step = 9;
                return;
            };
            info!("shroom: cutting the stem at {at:?} ({} bodies)", sim.world.bodies().len());
            for dx in -3..=3 {
                let block = CellPos::new((at.x + dx * 4).div_euclid(4), at.y.div_euclid(4));
                sim.world.apply_edit(&WorldEdit::MineBlock { block, power: 255, max_hardness: 254, back: true });
            }
            *step = 1;
        }
        1 if s.elapsed > 3.5 => {
            info!("shroom: after the cut, {} bodies", sim.world.bodies().len());
            *step = 2;
        }
        _ => {}
    }
}

fn drop_script(s: Res<Scenario>, player: Query<&Kinematics, With<LocalPlayer>>, mut keys: ResMut<ButtonInput<KeyCode>>, mut logged: Local<u8>) {
    if s.name != "drop" {
        return;
    }
    let Ok(k) = player.single() else { return };
    if s.elapsed > 3.0 { keys.press(KeyCode::KeyS) } else { keys.release(KeyCode::KeyS) }
    let feet = k.body.bottom();
    if *logged == 0 && s.elapsed > 2.9 {
        info!("drop: feet at {feet:.0} before holding S");
        *logged = 1;
    } else if *logged == 1 && s.elapsed > 5.5 {
        info!("drop: feet at {feet:.0} after");
        *logged = 2;
    }
}

#[allow(clippy::too_many_arguments)]
fn hands_script(
    s: Res<Scenario>,
    player: Query<(&Kinematics, Option<&crate::hands::items::Inventory>), With<LocalPlayer>>,
    items: Option<Res<crate::hands::items::Items>>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut logged: Local<u8>,
    mut build_slot: Local<Option<usize>>,
    mut start_y: Local<Option<f32>>,
) {
    use crate::hands::items::{BLOCK_CELLS, Use};
    if s.name != "hands" {
        return;
    }
    let Ok((k, inv)) = player.single() else { return };
    let (Some(inv), Some(items)) = (inv, items) else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    const DIGITS: [KeyCode; 10] = [
        KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5,
        KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9, KeyCode::Digit0,
    ];
    for key in DIGITS {
        keys.release(key);
    }
    let pick = |keys: &mut ButtonInput<KeyCode>, slot: usize| keys.press(DIGITS[slot]);
    // Straight down, held: the smart cursor digs a shaft the player drops into.
    if t < 0.8 {
        *start_y = Some(p.y);
    }
    let (aim, hold, ctrl) = match t {
        t if t < 0.8 => (None, false, false),
        t if t < 6.0 => {
            pick(&mut keys, 0);
            if (5.9..6.0).contains(&t) && *logged == 0 {
                *logged = 2;
                info!("hands: dug down {:.0} cells in 5.2 s", start_y.unwrap_or(p.y) - p.y);
            }
            (Some(p + Vec2::new(0.0, -30.0)), true, false)
        }
        // Into the shaft's wall.
        t if t < 7.0 => (Some(p + Vec2::new(14.0, -2.0)), true, false),
        // Build with a stack that has a whole block.
        t if t < 8.5 => {
            if build_slot.is_none() {
                *build_slot = (0..10).find(|&i| inv.slots[i].is_some_and(|st| matches!(items.def(st.item).use_, Use::Block(_)) && st.count >= BLOCK_CELLS));
            }
            if let Some(i) = *build_slot {
                pick(&mut keys, i);
            }
            (Some(p + Vec2::new(-10.0, 4.0)), true, false)
        }
        // Auto tool (Ctrl) at whatever's up and to the right.
        t if t < 10.0 => (Some(p + Vec2::new(20.0, 24.0)), true, true),
        // A torch on the floor.
        t if t < 10.3 => {
            if let Some(i) = items.id("torch").and_then(|torch| (0..10).find(|&i| inv.slots[i].is_some_and(|st| st.item == torch))) {
                pick(&mut keys, i);
            }
            (Some(p + Vec2::new(-6.0, -8.0)), false, false)
        }
        t if t < 10.4 => (Some(p + Vec2::new(-6.0, -8.0)), true, false),
        _ => (None, false, false),
    };
    cursor.0 = aim;
    if hold { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    if ctrl { keys.press(KeyCode::ControlLeft) } else { keys.release(KeyCode::ControlLeft) }
    if t > 11.0 && *logged < 3 {
        *logged = 3;
        let held: Vec<String> = inv.slots.iter().flatten().map(|st| format!("{} {}", items.def(st.item).name, st.count / items.unit(st.item))).collect();
        let torches = items.id("torch").map_or(0, |torch| inv.count(torch));
        info!("hands: inventory {} (built with slot {:?}; {torches} torches left)", held.join(", "), build_slot.map(|i| i + 1));
    }
}

/// A chest through real input: place, open, fill (directly), close, mine.
#[allow(clippy::too_many_arguments)]
fn chest_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut player: Query<(&Kinematics, Option<&mut crate::hands::items::Inventory>), With<LocalPlayer>>,
    items: Option<Res<crate::hands::items::Items>>,
    mut chests: ResMut<crate::hands::chests::Chests>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut step: Local<u8>,
    mut spot: Local<Option<Vec2>>,
) {
    use crate::hands::items::Stack;
    if s.name != "chest" {
        return;
    }
    let Ok((k, Some(mut inv))) = player.single_mut() else { return };
    let Some(items) = items else { return };
    let (Some(chest), Some(bomb)) = (items.id("chest"), items.id("bomb")) else { return };
    let p = k.body.pos;
    for key in [KeyCode::Digit1, KeyCode::Digit9, KeyCode::Escape] {
        keys.release(key);
    }
    mouse.release(MouseButton::Left);
    mouse.release(MouseButton::Right);
    let at = *spot.get_or_insert(p + Vec2::new(16.0, 0.0));
    match *step {
        0 if s.elapsed > 0.8 => {
            // A chest in slot 9.
            inv.slots[8] = Some(Stack::new(chest, 1));
            keys.press(KeyCode::Digit9);
            *step = 1;
        }
        1 if s.elapsed > 1.2 => {
            cursor.0 = Some(at);
            mouse.press(MouseButton::Left);
            *step = 2;
        }
        2 if s.elapsed > 1.6 => {
            cursor.0 = Some(at + Vec2::new(0.0, -4.0));
            mouse.press(MouseButton::Right);
            *step = 3;
        }
        3 if s.elapsed > 2.0 => {
            match chests.open {
                Some(c) => {
                    let taken = inv.slots.iter_mut().flatten().find(|st| st.item == bomb).map(|st| std::mem::replace(&mut st.count, 0)).unwrap_or(0);
                    inv.slots.iter_mut().for_each(|sl| if sl.is_some_and(|st| st.count == 0) { *sl = None });
                    chests.contents(c, &sim.world, &items).add(&items, Stack::new(bomb, taken));
                    info!("chest: placed and opened at {c:?}, {taken} bombs put in");
                }
                None => info!("chest: not open"),
            }
            keys.press(KeyCode::Escape);
            *step = 4;
        }
        4 if s.elapsed > 2.4 => {
            keys.press(KeyCode::Digit1);
            *step = 5;
        }
        5 if s.elapsed > 2.5 && s.elapsed < 5.0 => {
            cursor.0 = Some(at + Vec2::new(0.0, -4.0));
            mouse.press(MouseButton::Left);
        }
        5 if s.elapsed >= 6.0 => {
            let bombs = inv.count(bomb);
            let chests_back = inv.count(chest);
            info!("chest: after mining it, {bombs} bombs and {chests_back} chest in the pack");
            *step = 6;
        }
        _ => {}
    }
}

type Others = (With<crate::actors::Creature>, Without<LocalPlayer>);

/// Every starter wand, through real input (see the module notes).
#[allow(clippy::too_many_arguments)]
fn magic_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut commands: Commands,
    mut player: Query<(&Kinematics, &mut crate::actors::Health, Option<&mut crate::magic::Mana>), With<LocalPlayer>>,
    orcs: Query<(&Kinematics, &crate::actors::Health), Others>,
    spells: Query<(), With<crate::magic::Spell>>,
    mut blasts: MessageReader<crate::fx::Explosion>,
    mut zaps: MessageReader<crate::fx::Zapped>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, u32, u32)>,
) {
    if s.name != "magic" {
        return;
    }
    let Ok((k, mut me, mut mana)) = player.single_mut() else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    // Full mana and health for each wand, so each is seen on its own
    // (what it costs, and what it does to its caster).
    if [1.5, 3.0, 4.5, 5.5, 7.0].iter().any(|&at| (at..at + 0.05).contains(&t)) {
        me.hp = me.max;
        if let Some(m) = mana.as_deref_mut() {
            m.cur = m.max;
        }
    }
    state.2 += blasts.read().count() as u32;
    state.3 += zaps.read().count() as u32;
    if state.0 == 0 && t > 1.0 {
        for dx in [90, 120, 150] {
            let x = p.x as i32 + dx;
            if let Some(y) = find_ground(&sim.world, x, p.y as i32 + 200, 400) {
                crate::actors::creature::spawn_creature(&mut commands, "orc", Vec2::new(x as f32, y as f32), |_| {});
            }
        }
        state.0 = 1;
    }
    const SLOTS: [KeyCode; 5] = [KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9, KeyCode::Digit0];
    for key in SLOTS {
        keys.release(key);
    }
    let nearest = orcs.iter().map(|(o, _)| o.body.pos).min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)));
    let at_orc = nearest.unwrap_or(p + Vec2::new(60.0, 0.0));
    let (slot, aim) = match t {
        t if t < 1.5 => (None, None),
        t if t < 3.0 => (Some(0), Some(at_orc)),
        t if t < 4.5 => (Some(1), Some(p + Vec2::new(50.0, -12.0))),
        t if t < 5.5 => (Some(2), Some(p + Vec2::new(40.0, 40.0))),
        t if t < 7.0 => (Some(3), Some(at_orc)),
        t if t < 8.5 => (Some(4), Some(at_orc)),
        _ => (None, None),
    };
    if let Some(i) = slot {
        keys.press(SLOTS[i]);
    }
    cursor.0 = aim.or(Some(p + Vec2::new(30.0, 0.0)));
    if aim.is_some() { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    if t >= state.1 && state.0 == 1 {
        state.1 = (t * 4.0).floor() / 4.0 + 0.25;
        let hp: Vec<String> = orcs.iter().map(|(_, h)| format!("{:.0}", h.hp)).collect();
        info!(
            "magic: t {:.0} slot {:?} hp {:.0} mana {:.0} orcs [{}] spells {} blasts {} zaps {}",
            t,
            slot.map(|i| i + 6),
            me.hp,
            mana.map_or(0.0, |m| m.cur),
            hp.join(", "),
            spells.iter().count(),
            state.2,
            state.3
        );
    }
}

/// Lightning into a pool with orcs in it (see the module notes).
#[allow(clippy::too_many_arguments)]
fn shock_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut commands: Commands,
    player: Query<&Kinematics, With<LocalPlayer>>,
    orcs: Query<(&Kinematics, &crate::actors::Health), Others>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut step: Local<u8>,
    mut pool: Local<Option<(i32, i32)>>,
    mut zaps: MessageReader<crate::fx::Zapped>,
) {
    if s.name != "shock" {
        return;
    }
    for crate::fx::Zapped(z) in zaps.read() {
        let orcs_at: Vec<String> = orcs.iter().map(|(o, _)| format!("({:.0},{:.0})", o.body.pos.x, o.body.pos.y)).collect();
        info!("shock: zap {:?} -> {:?}, {} cells charged; orcs {}", z.from, z.to, z.charged.len(), orcs_at.join(" "));
    }
    let Ok(k) = player.single() else { return };
    let p = k.body.pos;
    let near = |x: i32, y: i32| orcs.iter().filter(|(o, _)| (o.body.pos.x as i32 - x).abs() < 50 && (o.body.pos.y as i32 - y).abs() < 30).map(|(_, h)| format!("{:.0}", h.hp)).collect::<Vec<_>>();
    match *step {
        0 if s.elapsed > 1.0 => {
            // A trough 80 wide, 14 deep, 40 to the right; then water in it.
            let Some(ground) = find_ground(&sim.world, p.x as i32 + 80, p.y as i32 + 40, 100) else { return };
            for x in (p.x as i32 + 40..p.x as i32 + 120).step_by(4) {
                sim.queue(WorldEdit::Dig { center: CellPos::new(x, ground - 8), radius: 8, max_hardness: 255 });
            }
            *pool = Some((p.x as i32 + 80, ground - 8));
            *step = 1;
        }
        1 if s.elapsed > 1.3 => {
            let (cx, cy) = pool.expect("dug");
            let water = sim.materials().expect_id("water");
            for x in (cx - 40..cx + 40).step_by(3) {
                for dy in [-4, 2] {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, cy + dy), radius: 7, material: water, overwrite: false });
                }
            }
            *step = 2;
        }
        // (In the far end, and struck before they wade out toward the player.)
        2 if s.elapsed > 3.5 => {
            let (cx, cy) = pool.expect("dug");
            for dx in [22, 34] {
                crate::actors::creature::spawn_creature(&mut commands, "orc", Vec2::new((cx + dx) as f32, cy as f32 + 2.0), |_| {});
            }
            *step = 3;
        }
        3 if s.elapsed > 3.9 => {
            let (cx, cy) = pool.expect("dug");
            info!("shock: before, orcs at the pool [{}]", near(cx, cy).join(", "));
            keys.press(KeyCode::Digit0);
            cursor.0 = Some(Vec2::new(cx as f32, cy as f32));
            mouse.press(MouseButton::Left);
            *step = 4;
        }
        4 if s.elapsed > 4.1 => {
            keys.release(KeyCode::Digit0);
            mouse.release(MouseButton::Left);
            *step = 5;
        }
        5 if s.elapsed > 4.6 => {
            let (cx, cy) = pool.expect("dug");
            info!("shock: after, orcs at the pool [{}]", near(cx, cy).join(", "));
            *step = 6;
        }
        _ => {}
    }
}

/// The inventory screen through real input (see the module notes).
#[allow(clippy::too_many_arguments)]
fn inventory_script(
    s: Res<Scenario>,
    mut window: Single<&mut Window, With<bevy::window::PrimaryWindow>>,
    slots: Query<(&crate::hands::ui::SlotUi, &bevy::ui::UiGlobalTransform, &InheritedVisibility)>,
    items: Option<Res<crate::hands::items::Items>>,
    player: Query<&crate::hands::items::Inventory, With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut step: Local<u8>,
) {
    use crate::hands::ui::Holder;
    if s.name != "inventory" {
        return;
    }
    let t = s.elapsed;
    keys.release(KeyCode::Escape);
    keys.release(KeyCode::KeyX);
    let scale = window.scale_factor();
    // Where a slot of the inventory screen is, in window (logical) pixels.
    let at = |i: usize| slots.iter().find(|(sl, _, v)| sl.0 == Holder::Pack && sl.1 == i && v.get()).map(|(_, tf, _)| tf.translation / scale);
    match *step {
        0 if t > 1.0 => {
            keys.press(KeyCode::Escape);
            *step = 1;
        }
        1 if t > 1.5 => {
            keys.press(KeyCode::KeyX);
            *step = 2;
        }
        2 if t > 2.0 => {
            window.set_cursor_position(at(5));
            *step = 3;
        }
        3 if t > 2.6 => {
            mouse.press(MouseButton::Left);
            *step = 4;
        }
        4 if t > 2.8 => {
            window.set_cursor_position(at(20));
            *step = 5;
        }
        5 if t > 3.0 => {
            mouse.release(MouseButton::Left);
            *step = 6;
        }
        6 if t > 3.3 => {
            if let (Some(items), Ok(inv)) = (items, player.single()) {
                let wand = items.id("spark_wand");
                let found = inv.slots.iter().position(|s| s.is_some_and(|s| Some(s.item) == wand));
                info!("inventory: the spark wand is in slot {found:?} (was 5; hotbar 3 starts at 20)");
            }
            window.set_cursor_position(None);
            *step = 7;
        }
        _ => {}
    }
}

/// The gravity wand through real input (see the module notes).
#[allow(clippy::too_many_arguments)]
fn well_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut commands: Commands,
    player: Query<(&Kinematics, Option<&crate::magic::Mana>), With<LocalPlayer>>,
    orcs: Query<(&Kinematics, &crate::actors::Health), Others>,
    wells: Query<&crate::magic::well::Well>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, Option<Vec2>)>,
) {
    if s.name != "well" {
        return;
    }
    let Ok((k, mana)) = player.single() else { return };
    let t = s.elapsed;
    keys.release(KeyCode::KeyX);
    keys.release(KeyCode::Digit2);
    let home = *state.2.get_or_insert(k.body.pos);
    if state.0 == 0 && t > 1.0 {
        for dx in [70, 85] {
            let x = home.x as i32 + dx;
            if let Some(y) = find_ground(&sim.world, x, home.y as i32 + 60, 200) {
                crate::actors::creature::spawn_creature(&mut commands, "orc", Vec2::new(x as f32, y as f32), |_| {});
            }
        }
        // The second hotbar, its second slot: the gravity wand.
        keys.press(KeyCode::KeyX);
        keys.press(KeyCode::Digit2);
        state.0 = 1;
    }
    let lerp = |a: Vec2, b: Vec2, f: f32| a.lerp(b, f.clamp(0.0, 1.0));
    let nearest = orcs.iter().map(|(o, _)| o.body.pos).filter(|o| o.x > home.x + 20.0).min_by(|a, b| a.x.total_cmp(&b.x)).unwrap_or(home + Vec2::new(70.0, 8.0));
    let (ground, up, throw) = (home + Vec2::new(35.0, -6.0), home + Vec2::new(40.0, 30.0), home + Vec2::new(160.0, 130.0));
    let aim = match t {
        t if t < 1.5 => None,
        t if t < 2.6 => Some(ground),
        t if t < 3.0 => Some(nearest),
        t if t < 3.6 => Some(lerp(nearest, up, (t - 3.0) / 0.4)),
        t if t < 3.75 => Some(lerp(up, throw, (t - 3.6) / 0.15)),
        _ => None,
    };
    cursor.0 = aim.or(Some(home + Vec2::new(30.0, 20.0)));
    if aim.is_some() { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    if t >= state.1 {
        state.1 = (t * 4.0).floor() / 4.0 + 0.25;
        let held: Vec<(usize, usize)> = wells.iter().map(|w| (w.holding(), w.carrying())).collect();
        let hp: Vec<String> = orcs.iter().filter(|(o, _)| o.body.pos.x > home.x + 20.0).map(|(o, h)| format!("{:.0}@{:.0},{:.0}", h.hp, o.body.pos.x - home.x, o.body.pos.y - home.y)).collect();
        info!("well: t {t:.1} (cells, bodies) {held:?} mana {:.0} orcs [{}] particles {}", mana.map_or(0.0, |m| m.cur), hp.join(" "), sim.world.particles().len());
    }
}

/// The force wand through real input (see the module notes).
#[allow(clippy::too_many_arguments)]
fn force_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut commands: Commands,
    player: Query<(&Kinematics, Option<&crate::magic::Mana>, &crate::actors::Health), With<LocalPlayer>>,
    orcs: Query<(&Kinematics, &crate::actors::Health), Others>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, Option<Vec2>)>,
) {
    if s.name != "force" {
        return;
    }
    let Ok((k, mana, me)) = player.single() else { return };
    let t = s.elapsed;
    keys.release(KeyCode::KeyX);
    keys.release(KeyCode::Digit3);
    let home = *state.2.get_or_insert(k.body.pos);
    if state.0 == 0 && t > 1.0 {
        let sand = sim.materials().expect_id("sand");
        for dx in [34, 40, 46] {
            sim.queue(WorldEdit::Paint { center: CellPos::new(home.x as i32 + dx, home.y as i32 + 2), radius: 6, material: sand, overwrite: false });
        }
        for dx in [50, 62] {
            let x = home.x as i32 + dx;
            if let Some(y) = find_ground(&sim.world, x, home.y as i32 + 60, 200) {
                crate::actors::creature::spawn_creature(&mut commands, "orc", Vec2::new(x as f32, y as f32), |_| {});
            }
        }
        // The second hotbar, its third slot: the force wand.
        keys.press(KeyCode::KeyX);
        keys.press(KeyCode::Digit3);
        state.0 = 1;
    }
    let down = t > 5.0 && t < 6.0;
    let (push, pull) = ((t > 1.5 && t < 2.0) || down, t > 3.0 && t < 4.4);
    cursor.0 = Some(if down { k.body.pos + Vec2::new(0.0, -40.0) } else { home + Vec2::new(60.0, 0.0) });
    if push { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    if pull { mouse.press(MouseButton::Right) } else { mouse.release(MouseButton::Right) }
    if t >= state.1 {
        state.1 = (t * 4.0).floor() / 4.0 + 0.25;
        let at: Vec<String> = orcs.iter().filter(|(o, _)| (o.body.pos.x - home.x).abs() < 250.0 && o.body.pos.x > home.x + 10.0).map(|(o, h)| format!("{:.0}@{:.0},{:.0}", h.hp, o.body.pos.x - home.x, o.body.pos.y - home.y)).collect();
        info!("force: t {t:.2} {} player {:+.0} hp {:.0} mana {:.0} orcs [{}] particles {}", if push { "push" } else if pull { "pull" } else { "-" }, k.body.pos.y - home.y, me.hp, mana.map_or(0.0, |m| m.cur), at.join(" "), sim.world.particles().len());
    }
}

/// Lift a pool and drop it, counting the water (see the module notes).
#[allow(clippy::too_many_arguments)]
fn wellwater_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    wells: Query<&crate::magic::well::Well>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, Option<Vec2>)>,
) {
    if s.name != "wellwater" {
        return;
    }
    let Ok(k) = player.single() else { return };
    let t = s.elapsed;
    keys.release(KeyCode::KeyX);
    keys.release(KeyCode::Digit2);
    keys.release(KeyCode::Digit4);
    // PLATYPUS_STAFF=1: the gravity staff (hotbar 2, slot 4), four times.
    let staff = std::env::var("PLATYPUS_STAFF").is_ok();
    let home = *state.2.get_or_insert(k.body.pos);
    let water = sim.materials().expect_id("water");
    if state.0 == 0 && t > 0.5 {
        let Some(ground) = find_ground(&sim.world, home.x as i32 + 60, home.y as i32 + 40, 100) else { return };
        for x in (home.x as i32 + 40..home.x as i32 + 80).step_by(4) {
            sim.queue(WorldEdit::Dig { center: CellPos::new(x, ground - 6), radius: 6, max_hardness: 255 });
        }
        state.0 = 1;
    }
    if state.0 == 1 && t > 0.8 {
        let Some(ground) = find_ground(&sim.world, home.x as i32 + 60, home.y as i32 + 40, 100) else { return };
        for x in (home.x as i32 + 42..home.x as i32 + 78).step_by(3) {
            sim.queue(WorldEdit::Paint { center: CellPos::new(x, ground - 3), radius: 4, material: water, overwrite: false });
        }
        keys.press(KeyCode::KeyX);
        keys.press(if staff { KeyCode::Digit4 } else { KeyCode::Digit2 });
        state.0 = 2;
    }
    let cycles = if staff { 4 } else { 2 };
    let phase = t - 1.8;
    let lift = phase > 0.0 && (phase as i32) < cycles * 2 && phase.rem_euclid(2.4) < 1.4 && phase < cycles as f32 * 2.4;
    let high = lift && phase.rem_euclid(2.4) > 0.8;
    let over = home + Vec2::new(60.0, if high { 40.0 } else { -2.0 });
    cursor.0 = Some(over);
    if lift { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    if t >= state.1 && state.0 == 2 {
        state.1 = (t * 2.0).floor() / 2.0 + 0.5;
        let cells: usize = sim.world.chunks().map(|c| c.cells().iter().filter(|c| c.material == water).count()).sum();
        let flying = sim.world.particles().iter().filter(|p| p.cell.material == water).count();
        let held: usize = wells.iter().map(|w| w.holding_of(water)).sum();
        let others: Vec<String> = ["blood", "acid", "steam", "ice"]
            .iter()
            .filter_map(|n| {
                let m = sim.materials().id(n)?;
                let c: usize = sim.world.chunks().map(|c| c.cells().iter().filter(|c| c.material == m).count()).sum::<usize>() + sim.world.particles().iter().filter(|p| p.cell.material == m).count();
                Some(format!("{n} {c}"))
            })
            .collect();
        let stuff: usize = sim.world.chunks().map(|c| c.cells().iter().filter(|c| !c.is_air()).count()).sum::<usize>() + sim.world.particles().len() + wells.iter().map(|w| w.holding()).sum::<usize>();
        info!("wellwater: t {t:.1} {} water: cells {cells} flying {flying} held {held} = {} | {} | everything {stuff}", if lift { "lift" } else { "-" }, cells + flying + held, others.join(" "));
    }
}

/// Spells into water (see the module notes).
#[allow(clippy::too_many_arguments)]
fn splash_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, Option<Vec2>, i32)>,
    mut commands: Commands,
    orcs: Query<Entity, Others>,
) {
    if s.name != "splash" {
        return;
    }
    // (No orcs wading in the way.)
    for e in &orcs {
        commands.entity(e).despawn();
    }
    let Ok(k) = player.single() else { return };
    let t = s.elapsed;
    for key in [KeyCode::KeyX, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8] {
        keys.release(key);
    }
    let home = *state.2.get_or_insert(k.body.pos);
    let (water, oil) = (sim.materials().expect_id("water"), sim.materials().expect_id("oil"));
    if state.0 == 0 && t > 0.5 {
        let Some(ground) = find_ground(&sim.world, home.x as i32 + 80, home.y as i32 + 40, 100) else { return };
        for x in (home.x as i32 + 40..home.x as i32 + 130).step_by(4) {
            sim.queue(WorldEdit::Dig { center: CellPos::new(x, ground - 7), radius: 7, max_hardness: 255 });
        }
        // A pit of its own for the oil (on the pool it spreads over all of it).
        for x in (home.x as i32 + 150..home.x as i32 + 175).step_by(4) {
            sim.queue(WorldEdit::Dig { center: CellPos::new(x, ground - 5), radius: 5, max_hardness: 255 });
        }
        state.3 = ground;
        state.0 = 1;
    }
    if state.0 == 1 && t > 0.8 {
        let ground = state.3;
        for x in (home.x as i32 + 42..home.x as i32 + 128).step_by(3) {
            for dy in [-7, -3] {
                sim.queue(WorldEdit::Paint { center: CellPos::new(x, ground + dy), radius: 4, material: water, overwrite: false });
            }
        }
        state.0 = 2;
    }
    if state.0 == 2 && t > 1.3 {
        let ground = state.3;
        for x in (home.x as i32 + 152..home.x as i32 + 173).step_by(3) {
            sim.queue(WorldEdit::Paint { center: CellPos::new(x, ground - 3), radius: 3, material: oil, overwrite: false });
        }
        state.0 = 3;
    }
    let ground = state.3 as f32;
    // (wand key, aim, from, to): the fireball is hotbar 1 slot 7; spark 6;
    // acid 8; frost hotbar 2 slot 6.
    let plan: [(KeyCode, bool, Vec2, f32, f32); 6] = [
        (KeyCode::Digit7, false, home + Vec2::new(13.0, 54.0), 2.0, 2.1),
        (KeyCode::Digit7, false, Vec2::new(home.x + 110.0, ground), 3.5, 3.6),
        (KeyCode::Digit6, false, Vec2::new(home.x + 75.0, ground - 6.0), 5.0, 5.1),
        (KeyCode::Digit8, false, Vec2::new(home.x + 75.0, ground + 8.0), 6.0, 6.1),
        (KeyCode::Digit7, false, home + Vec2::new(23.0, 44.0), 7.0, 7.1),
        (KeyCode::Digit6, true, Vec2::new(home.x + 75.0, ground - 6.0), 8.5, 8.6),
    ];
    let now = plan.iter().find(|p| t >= p.3 - 0.3 && t < p.4);
    if let Some(&(key, bar2, aim, from, _)) = now {
        if t < from {
            // Pick the wand first (the second hotbar for frost, and back).
            if bar2 != (state.1 > 0.5) {
                keys.press(KeyCode::KeyX);
                state.1 = if bar2 { 1.0 } else { 0.0 };
            }
            keys.press(key);
        }
        cursor.0 = Some(aim);
        if t >= from { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    } else {
        mouse.release(MouseButton::Left);
        cursor.0 = Some(home + Vec2::new(20.0, 10.0));
    }
    if (t * 2.0).floor() != ((t - 0.017) * 2.0).floor() && state.0 >= 2 {
        let count = |n: &str| sim.materials().id(n).map_or(0, |m| sim.world.chunks().map(|c| c.cells().iter().filter(|c| c.material == m).count()).sum::<usize>());
        let burning: usize = sim.world.chunks().map(|c| c.cells().iter().filter(|c| c.flags & platypus_sim::cell::flags::BURNING != 0).count()).sum();
        let flying = sim.world.particles().iter().filter(|p| p.cell.material == water).count();
        info!("splash: t {t:.1} water {} (+{flying} flying) steam {} ice {} oil {} burning {burning}", count("water"), count("steam"), count("ice"), count("oil"));
    }
}

/// A long fall, saved (or not) by an air jump just above the ground; and
/// air jumps chained up (see the module notes).
fn airjump_script(
    s: Res<Scenario>,
    mut player: Query<(&mut Kinematics, &crate::actors::Health), With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<(Option<f32>, f32, u8, f32)>,
    mut day: ResMut<crate::light::Daylight>,
) {
    if s.name != "airjump" {
        return;
    }
    // PLATYPUS_NIGHT=1: at 23:00 (the cloud's glow in the dark).
    if std::env::var("PLATYPUS_NIGHT").is_ok_and(|v| !v.is_empty()) && state.0.is_none() {
        day.skipped = (23.0 - day.time * 24.0).rem_euclid(24.0);
    }
    let Ok((mut k, h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let ground = *state.0.get_or_insert(k.body.pos.y);
    // PLATYPUS_NOSAVE=1: no air jump before landing (it should hurt).
    let nosave = std::env::var("PLATYPUS_NOSAVE").is_ok_and(|v| !v.is_empty());
    // Lifted 160 cells up at 1 s: a fall well past the safe height.
    if state.2 == 0 && t > 1.0 {
        k.body.pos.y += 160.0;
        k.prev_pos = k.body.pos;
        k.body.vel = Vec2::ZERO;
        state.2 = 1;
    }
    let low = k.body.pos.y - ground < 14.0;
    let save = !nosave && state.2 == 1 && k.body.vel.y < 0.0 && low;
    if save {
        state.2 = 2;
        state.3 = t;
    }
    let hold = state.2 == 2 && t - state.3 < 0.2;
    if hold { keys.press(KeyCode::Space) } else { keys.release(KeyCode::Space) }
    if t >= state.1 {
        state.1 = (t * 4.0).floor() / 4.0 + 0.25;
        info!("airjump: t {t:.2} height {:+.0} hp {:.0}{}", k.body.pos.y - ground, h.hp, if state.2 == 2 { " (saved)" } else { "" });
    }
}

/// Walk at a rabbit (see the module notes).
fn critters_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut commands: Commands,
    player: Query<&Kinematics, With<LocalPlayer>>,
    critters: Query<(&crate::actors::Creature, &Kinematics), Without<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<(u8, f32)>,
) {
    if s.name != "critters" {
        return;
    }
    let Ok(k) = player.single() else { return };
    let t = s.elapsed;
    if state.0 == 0 && t > 1.0 {
        for (dx, kind) in [(60, "rabbit"), (75, "bird"), (90, "frog")] {
            let x = k.body.pos.x as i32 + dx;
            if let Some(y) = find_ground(&sim.world, x, k.body.pos.y as i32 + 40, 100) {
                crate::actors::creature::spawn_creature(&mut commands, kind, Vec2::new(x as f32 + 0.5, y as f32), |_| {});
            }
        }
        state.0 = 1;
    }
    if t > 2.0 && t < 4.5 { keys.press(KeyCode::KeyD) } else { keys.release(KeyCode::KeyD) }
    if t >= state.1 {
        state.1 = (t * 2.0).floor() / 2.0 + 0.5;
        let near: Vec<String> = critters.iter().filter(|(c, r)| c.kind != "orc" && r.body.pos.distance(k.body.pos) < 500.0).map(|(c, r)| format!("{} {:+.0},{:+.0}", c.kind, r.body.pos.x - k.body.pos.x, r.body.pos.y - k.body.pos.y)).collect();
        info!("critters: t {t:.1} [{}]", near.join("; "));
    }
}

/// The arena's tools through their own messages, the wands through real
/// keys and buttons.
#[allow(clippy::too_many_arguments)]
fn arena_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    dummies: Query<(&crate::actors::Creature, &Kinematics, &crate::actors::dummy::Tally)>,
    mut arena: MessageWriter<crate::arena::ArenaAction>,
    mut dev: MessageWriter<crate::dev::DevAction>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, u64, f32)>,
    mut logged: Local<f32>,
) {
    use crate::arena::ArenaAction as A;
    if s.name != "arena" {
        return;
    }
    let Ok(k) = player.single() else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    let tick = sim.world.tick();
    // Each step once, in order.
    let beats: [(f32, u8); 9] = [(0.5, 1), (4.6, 2), (5.0, 3), (5.2, 4), (5.4, 5), (5.6, 6), (5.8, 7), (6.0, 8), (8.0, 9)];
    for (at, n) in beats {
        if t < at || state.0 >= n {
            continue;
        }
        state.0 = n;
        match n {
            1 => {
                arena.write(A::Overlays);
            }
            2 => {
                arena.write(A::Pick("orc".into()));
                dev.write(crate::dev::DevAction::Spawn(Some(p + Vec2::new(-80.0, 10.0))));
            }
            3 => {
                arena.write(A::Pause);
                state.1 = tick;
            }
            4..=6 => {
                arena.write(A::Step);
            }
            7 => {
                info!("arena: paused and stepped 3 times: {} ticks", tick - state.1);
                arena.write(A::Pause);
            }
            8 => {
                arena.write(A::Speed(0.25));
                state.1 = tick;
                state.2 = t;
            }
            _ => {
                info!("arena: at 1/4 speed: {:.1} ticks a second", (tick - state.1) as f32 / (t - state.2));
                arena.write(A::Speed(1.0));
            }
        }
    }
    const SLOTS: [KeyCode; 2] = [KeyCode::Digit6, KeyCode::Digit7];
    for key in SLOTS {
        keys.release(key);
    }
    let dummy = |x: f32| dummies.iter().map(|(_, dk, _)| dk.body.pos).find(|d| (d.x - x).abs() < 8.0);
    let aim = match t {
        t if (1.0..3.0).contains(&t) => {
            keys.press(SLOTS[0]);
            dummy(700.0)
        }
        t if (3.0..4.4).contains(&t) => {
            keys.press(SLOTS[1]);
            dummy(760.0)
        }
        _ => None,
    };
    cursor.0 = aim.or(Some(p + Vec2::new(30.0, 0.0)));
    if aim.is_some() { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    if t >= *logged {
        *logged = (t * 2.0).floor() / 2.0 + 0.5;
        let read: Vec<String> = dummies.iter().map(|(c, dk, tl)| format!("{} @{:.0}: {:.0} dps, {:.0} in {} hits", c.kind, dk.body.pos.x, tl.dps(), tl.total, tl.hits)).collect();
        info!("arena: t {t:.1} [{}]", read.join("; "));
    }
}

/// The art editor through the real pointer and keys, on a scratch copy.
#[allow(clippy::too_many_arguments)]
fn editor_script(
    s: Res<Scenario>,
    mut window: Single<&mut Window, With<bevy::window::PrimaryWindow>>,
    canvas: Query<(&bevy::ui::UiGlobalTransform, &bevy::ui::ComputedNode), With<crate::editor::Canvas>>,
    editor: Res<crate::editor::Editor>,
    mut arena: MessageWriter<crate::arena::ArenaAction>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, String)>,
) {
    if s.name != "editor" {
        return;
    }
    let t = s.elapsed;
    let shown = editor.canvas();
    let read = || shown.as_ref().and_then(|(_, f)| std::fs::read_to_string(f).ok()).unwrap_or_default();
    let scale = window.scale_factor();
    // Pixel (x, y) of the canvas, in window pixels.
    let at = |x: f32, y: f32| {
        let ((w, h), _) = shown.clone()?;
        canvas.single().ok().map(|(tf, node)| {
            let size = node.size() / scale;
            tf.translation / scale + Vec2::new((x + 0.5) / w as f32 - 0.5, (y + 0.5) / h as f32 - 0.5) * size
        })
    };
    let beats = [0.5, 1.0, 1.2, 1.3, 1.4, 1.5, 1.8, 2.0, 2.2, 2.6];
    let Some(n) = beats.iter().rposition(|&b| t >= b) else { return };
    let n = n as u8 + 1;
    if state.0 >= n {
        return;
    }
    state.0 = n;
    match n {
        1 => {
            arena.write(crate::arena::ArenaAction::Editor);
        }
        2 => {
            state.1 = read();
            window.set_cursor_position(at(9.0, 14.0));
        }
        3 => mouse.press(MouseButton::Left),
        4 => window.set_cursor_position(at(9.0, 15.0)),
        5 => window.set_cursor_position(at(9.0, 16.0)),
        6 => mouse.release(MouseButton::Left),
        7 => {
            let now = read();
            let changed = state.1.chars().zip(now.chars()).filter(|(a, b)| a != b).count();
            info!("editor: after one stroke, {changed} characters of the file changed ({} lines before, {} after)", state.1.lines().count(), now.lines().count());
        }
        8 => {
            keys.press(KeyCode::ControlLeft);
            keys.press(KeyCode::KeyZ);
        }
        9 => {
            keys.release(KeyCode::ControlLeft);
            keys.release(KeyCode::KeyZ);
        }
        _ => {
            info!("editor: after Ctrl+Z the file is as it was: {}", read() == state.1);
            window.set_cursor_position(at(7.0, 9.0));
        }
    }
}

/// The spark and flame wands against the arena's floor, sandbag and dummies.
#[allow(clippy::too_many_arguments)]
fn wands_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    dummies: Query<(&crate::actors::Creature, &Kinematics, &crate::actors::dummy::Tally)>,
    mut commands: Commands,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, usize, f32)>,
) {
    if s.name != "wands" {
        return;
    }
    let Ok(k) = player.single() else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    // Solid cells of the floor's top 12 rows around x 680.
    let solid = || (660..700).flat_map(|x| (floor - 12..floor).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| !c.is_air())).count();
    // (The sandbag nearest the player: one is put 40 cells off at 2 s.)
    let bag = || dummies.iter().filter(|(c, ..)| c.kind == "sandbag").map(|(_, dk, _)| dk.body.pos).min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)));
    let dummy = |x: f32| dummies.iter().find(|(c, dk, _)| c.kind == "dummy" && (dk.body.pos.x - x).abs() < 8.0);
    for key in [KeyCode::Digit6, KeyCode::Digit9] {
        keys.release(key);
    }
    let aim = match t {
        t if (0.8..1.0).contains(&t) => {
            state.1 = solid();
            None
        }
        t if (1.0..2.0).contains(&t) => {
            keys.press(KeyCode::Digit6);
            Some(Vec2::new(680.0, floor as f32 - 2.0))
        }
        t if (2.0..2.2).contains(&t) => {
            if state.0 == 0 {
                info!("wands: the spark broke {} floor cells in a second", state.1 as i64 - solid() as i64);
                crate::actors::creature::spawn_creature(&mut commands, "sandbag", Vec2::new(p.x + 40.0, floor as f32), |_| {});
                state.0 = 1;
            }
            None
        }
        t if (2.2..3.2).contains(&t) => {
            if state.2 == 0.0 {
                state.2 = bag().map_or(0.0, |b| b.x);
            }
            keys.press(KeyCode::Digit6);
            bag()
        }
        t if (3.2..3.4).contains(&t) => {
            if state.0 == 1 {
                info!("wands: the sandbag went from x {:.0} to {:?} (a second of sparks, {:?} hits)", state.2, bag().map(|b| b.x.round()), dummies.iter().filter(|(c, ..)| c.kind == "sandbag").map(|d| d.2.hits).max());
                state.0 = 2;
            }
            None
        }
        t if (3.4..5.0).contains(&t) => {
            keys.press(KeyCode::Digit9);
            dummy(700.0).map(|d| d.1.body.pos)
        }
        t if (5.0..6.6).contains(&t) => {
            keys.press(KeyCode::Digit9);
            dummy(760.0).map(|d| d.1.body.pos)
        }
        _ => {
            if state.0 == 2 {
                let took = |x: f32| dummy(x).map_or(0.0, |d| d.2.total);
                info!("wands: flames from x {:.0}: the dummy 60 off took {:.0}, the one 120 off {:.0}", p.x, took(700.0), took(760.0));
                state.0 = 3;
            }
            None
        }
    };
    cursor.0 = aim.or(Some(p + Vec2::new(30.0, 0.0)));
    if aim.is_some() { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
}

type Fighting<'a> = (&'a Kinematics, &'a crate::actors::Health, Option<&'a crate::combat::Stamina>, Option<&'a crate::combat::Wielding>);

/// The swords through real keys and buttons, against the first dummy.
#[allow(clippy::too_many_arguments)]
fn melee_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    player: Query<Fighting, With<LocalPlayer>>,
    dummies: Query<(&crate::actors::Creature, &Kinematics, &crate::actors::dummy::Tally), Without<LocalPlayer>>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, f32, f32, f32)>,
) {
    if s.name != "melee" {
        return;
    }
    if s.elapsed < 0.1 {
        state.4 = 100.0;
        return;
    }
    let Ok((k, h, stamina, wielding)) = player.single() else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    let Some((_, dk, tally)) = dummies.iter().find(|(c, dk, _)| c.kind == "dummy" && (dk.body.pos.x - 700.0).abs() < 8.0) else { return };
    let d = dk.body.pos;
    // Each key held while wanted, so a press is one press.
    let mut want = std::collections::HashSet::new();
    let window = |a: f32, b: f32| t >= a && t < b;
    if window(0.3, 0.4) {
        want.insert(KeyCode::KeyX);
    }
    if window(0.5, 0.6) {
        want.insert(KeyCode::Digit7);
    }
    if window(2.5, 2.6) {
        want.insert(KeyCode::Digit8);
    }
    let stam = stamina.map_or(0.0, |s| s.cur);
    let held = wielding.and_then(|w| w.0.clone()).unwrap_or_default();
    let mut swing = None;
    if window(0.6, 2.4) {
        if d.x - p.x > 12.0 {
            want.insert(KeyCode::KeyD);
        }
        if t > 0.9 {
            swing = Some(d + Vec2::new(0.0, 2.0));
        }
        state.4 = state.4.min(stam);
    }
    if window(2.4, 2.5) && state.0 == 0 {
        info!("melee: {held}: {} hits, {:.0} damage in {:.1}s; stamina went down to {:.0}", tally.hits, tally.total, tally.last - tally.start, state.4);
        state.0 = 1;
        state.1 = tally.total;
        state.4 = 100.0;
    }
    if window(2.7, 4.2) {
        swing = Some(d + Vec2::new(0.0, 2.0));
        state.4 = state.4.min(stam);
    }
    if window(4.2, 4.6) {
        if state.0 == 1 {
            info!("melee: {held}: {:.0} more damage; stamina went down to {:.0}", tally.total - state.1, state.4);
            state.0 = 2;
        }
        want.insert(KeyCode::KeyA);
    }
    if window(4.6, 5.8) {
        if t < 4.9 {
            want.insert(KeyCode::Space);
        }
        if p.x < d.x - 2.0 {
            want.insert(KeyCode::KeyD);
        }
        // Over it: strike down; how fast it rises after is the pogo.
        if !k.loco.grounded() && p.y > d.y + 12.0 {
            swing = Some(d);
            state.3 = f32::max(state.3, tally.hits as f32);
        }
        if t > 5.0 {
            state.2 = state.2.max(k.body.vel.y);
        }
    }
    if window(5.8, 5.9) && state.0 == 2 {
        info!("melee: striking down from above: the fastest rise after 5 s {:.0} cells/s", state.2);
        state.0 = 3;
    }
    // A blast in the middle of a dodge, then the same standing.
    if window(6.2, 6.3) {
        want.insert(KeyCode::ShiftLeft);
    }
    if t > 6.26 && state.0 == 3 {
        state.2 = h.hp;
        sim.queue(WorldEdit::Explode { center: CellPos::new(p.x as i32 + 3, p.y as i32), radius: 3, power: 60 });
        state.0 = 4;
    }
    if t > 6.9 && state.0 == 4 {
        info!("melee: a blast mid-dodge cost {:.0} hp", state.2 - h.hp);
        state.2 = h.hp;
        sim.queue(WorldEdit::Explode { center: CellPos::new(p.x as i32 + 3, p.y as i32), radius: 3, power: 60 });
        state.0 = 5;
    }
    if t > 7.4 && state.0 == 5 {
        info!("melee: the same blast standing cost {:.0} hp", state.2 - h.hp);
        state.0 = 6;
    }
    for key in [KeyCode::KeyX, KeyCode::Digit7, KeyCode::Digit8, KeyCode::KeyD, KeyCode::KeyA, KeyCode::Space, KeyCode::ShiftLeft] {
        match (want.contains(&key), keys.pressed(key)) {
            (true, false) => keys.press(key),
            (false, true) => keys.release(key),
            _ => {}
        }
    }
    cursor.0 = swing.or(Some(p + Vec2::new(30.0, 0.0)));
    match (swing.is_some(), mouse.pressed(MouseButton::Left)) {
        (true, false) => mouse.press(MouseButton::Left),
        (false, true) => mouse.release(MouseButton::Left),
        _ => {}
    }
}

/// The player against an orc, then a troll, through real keys and buttons.
#[allow(clippy::too_many_arguments)]
fn fight_script(
    s: Res<Scenario>,
    mut commands: Commands,
    player: Query<(&Kinematics, &crate::actors::Health), With<LocalPlayer>>,
    foes: Query<(&crate::actors::Creature, &Kinematics, &crate::actors::Health, Has<crate::combat::Swing>), Without<LocalPlayer>>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, u32, u32)>,
) {
    if s.name != "fight" {
        return;
    }
    let Ok((k, h)) = player.single() else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR as f32;
    let mut want = std::collections::HashSet::new();
    if (0.3..0.4).contains(&t) {
        want.insert(KeyCode::KeyX);
    }
    if (0.5..0.6).contains(&t) {
        want.insert(KeyCode::Digit7);
    }
    if state.0 == 0 && t > 0.8 {
        crate::actors::creature::spawn_creature(&mut commands, "orc", Vec2::new(p.x - 40.0, floor), |_| {});
        state.0 = 1;
    }
    if state.0 == 1 && t > 5.0 {
        crate::actors::creature::spawn_creature(&mut commands, "troll", Vec2::new(p.x - 70.0, floor), |_| {});
        state.0 = 2;
    }
    let near = foes.iter().filter(|(c, ..)| c.kind == "orc" || c.kind == "troll").min_by(|a, b| a.1.body.pos.distance(p).total_cmp(&b.1.body.pos.distance(p)));
    let swing = near.filter(|(_, fk, ..)| t > 1.5 && fk.body.pos.distance(p) < 30.0).map(|(_, fk, ..)| fk.body.pos);
    for (_, _, _, swinging) in &foes {
        if swinging {
            state.3 += 1;
        }
    }
    if t >= state.1 {
        state.1 = (t * 2.0).floor() / 2.0 + 0.5;
        let them: Vec<String> = foes.iter().filter(|(c, ..)| c.kind == "orc" || c.kind == "troll").map(|(c, fk, fh, sw)| format!("{} {:+.0} hp {:.0}{}", c.kind, fk.body.pos.x - p.x, fh.hp, if sw { " swinging" } else { "" })).collect();
        info!("fight: t {t:.1} player hp {:.0} [{}] ({} ticks of enemy swings so far)", h.hp, them.join("; "), state.3);
    }
    for key in [KeyCode::KeyX, KeyCode::Digit7] {
        match (want.contains(&key), keys.pressed(key)) {
            (true, false) => keys.press(key),
            (false, true) => keys.release(key),
            _ => {}
        }
    }
    cursor.0 = swing.or(Some(p + Vec2::new(-30.0, 0.0)));
    match (swing.is_some(), mouse.pressed(MouseButton::Left)) {
        (true, false) => mouse.press(MouseButton::Left),
        (false, true) => mouse.release(MouseButton::Left),
        _ => {}
    }
}

/// The bow through real keys and buttons; then an orc archer.
#[allow(clippy::too_many_arguments)]
fn archery_script(
    s: Res<Scenario>,
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    player: Query<(&Kinematics, &crate::actors::Health, &crate::hands::items::Inventory), With<LocalPlayer>>,
    dummies: Query<(&crate::actors::Creature, &Kinematics, &crate::actors::dummy::Tally)>,
    arrows: Query<&crate::archery::Arrow>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, u32)>,
) {
    if s.name != "archery" {
        return;
    }
    let (Ok((k, h, inv)), Some(items)) = (player.single(), items) else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR as f32;
    let quiver = items.id("arrow").map_or(0, |a| inv.count(a));
    let mut want = std::collections::HashSet::new();
    if (0.3..0.4).contains(&t) {
        want.insert(KeyCode::KeyX);
    }
    if (0.5..0.6).contains(&t) {
        want.insert(KeyCode::Digit9);
    }
    let dummy = dummies.iter().find(|(c, dk, _)| c.kind == "dummy" && (dk.body.pos.x - 700.0).abs() < 8.0);
    let mut aim = None;
    if (1.0..1.8).contains(&t) {
        aim = dummy.map(|d| d.1.body.pos + Vec2::new(0.0, 4.0));
    }
    if (2.0..2.2).contains(&t) {
        if state.0 == 0 {
            info!("archery: a full draw at the dummy: it took {:.0} ({} hits); {quiver} arrows left", dummy.map_or(0.0, |d| d.2.total), dummy.map_or(0, |d| d.2.hits));
            state.0 = 1;
        }
        aim = Some(Vec2::new(p.x + 30.0, floor - 2.0));
    }
    // A puddle of lava to shoot down through.
    if (2.3..2.32).contains(&t) && state.0 == 1 {
        if let Some(lava) = sim.materials().id("lava") {
            sim.queue(WorldEdit::Paint { center: CellPos::new(p.x as i32 - 24, floor as i32 + 1), radius: 2, material: lava, overwrite: false });
        }
        state.0 = 2;
    }
    if (2.65..3.3).contains(&t) {
        aim = Some(Vec2::new(p.x - 25.0, floor - 2.0));
    }
    if (3.4..3.42).contains(&t) {
        info!("archery: through the fire: {} of {} arrows burning", arrows.iter().filter(|a| a.burning()).count(), arrows.iter().count());
    }
    if (3.5..3.6).contains(&t) && state.0 == 2 {
        let stuck = arrows.iter().count();
        info!("archery: {stuck} arrows about, {quiver} in the quiver");
        state.0 = 3;
        state.2 = quiver;
    }
    if (3.6..5.0).contains(&t) && p.x < 700.0 - 12.0 {
        want.insert(KeyCode::KeyD);
    }
    if (5.2..5.3).contains(&t) && state.0 == 3 {
        info!("archery: walked over them: {quiver} in the quiver (was {}), {} arrows about", state.2, arrows.iter().count());
        crate::actors::creature::spawn_creature(&mut commands, "orc_archer", Vec2::new(p.x - 110.0, floor), |_| {});
        state.0 = 4;
        state.1 = h.hp;
    }
    if t > 9.5 && state.0 == 4 {
        info!("archery: 4 s of the orc archer cost {:.0} hp", state.1 - h.hp);
        state.0 = 5;
    }
    for key in [KeyCode::KeyX, KeyCode::Digit9, KeyCode::KeyD] {
        match (want.contains(&key), keys.pressed(key)) {
            (true, false) => keys.press(key),
            (false, true) => keys.release(key),
            _ => {}
        }
    }
    cursor.0 = aim.or(Some(p + Vec2::new(30.0, 0.0)));
    match (aim.is_some(), mouse.pressed(MouseButton::Left)) {
        (true, false) => mouse.press(MouseButton::Left),
        (false, true) => mouse.release(MouseButton::Left),
        _ => {}
    }
}

/// O spawns a pack.
fn warband_script(
    s: Res<Scenario>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    foes: Query<&crate::actors::Creature, Without<LocalPlayer>>,
    mut dev: MessageWriter<crate::dev::DevAction>,
    mut state: Local<u8>,
) {
    if s.name != "warband" {
        return;
    }
    let Ok(k) = player.single() else { return };
    if *state == 0 && s.elapsed > 1.0 {
        dev.write(crate::dev::DevAction::Spawn(Some(k.body.pos + Vec2::new(120.0, 20.0))));
        *state = 1;
    }
    if *state == 1 && s.elapsed > 3.0 {
        let mut n = std::collections::BTreeMap::new();
        for c in &foes {
            *n.entry(c.kind.clone()).or_insert(0) += 1;
        }
        info!("warband: {n:?}");
        *state = 2;
    }
}

/// Each tool in the hand, through real keys and buttons.
#[allow(clippy::too_many_arguments)]
fn held_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    player: Query<(&Kinematics, Option<&crate::combat::Swing>, &crate::combat::Wielding), With<LocalPlayer>>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, usize, u32, bool)>,
) {
    if s.name != "held" {
        return;
    }
    let Ok((k, swing, wielding)) = player.single() else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let solid = || (640..680).flat_map(|x| (floor - 16..floor).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| !c.is_air())).count();
    // Swings started (a swing seen after none).
    let swinging = swing.is_some();
    if swinging && !state.3 {
        state.2 += 1;
    }
    state.3 = swinging;
    let mut want = std::collections::HashSet::new();
    let press = |a: f32, want: &mut std::collections::HashSet<KeyCode>, k: KeyCode| {
        if (a..a + 0.1).contains(&t) {
            want.insert(k);
        }
    };
    press(0.3, &mut want, KeyCode::Digit1);
    press(2.2, &mut want, KeyCode::Digit3);
    press(3.0, &mut want, KeyCode::Digit6);
    press(4.0, &mut want, KeyCode::Digit4);
    press(4.8, &mut want, KeyCode::Digit2);
    if (0.4..0.5).contains(&t) {
        state.1 = solid();
    }
    let aim = match t {
        t if (0.5..2.0).contains(&t) => Some(Vec2::new(p.x + 14.0, floor as f32 - 3.0)),
        t if (3.2..3.8).contains(&t) => Some(Vec2::new(700.0, floor as f32 + 10.0)),
        t if (4.3..4.35).contains(&t) => Some(p + Vec2::new(60.0, 40.0)),
        t if (5.0..5.6).contains(&t) => Some(p + Vec2::new(20.0, 5.0)),
        _ => None,
    };
    if (2.05..2.1).contains(&t) && state.0 == 0 {
        info!("held: the pickaxe ({:?}) dug {} cells in 1.5 s over {} swings", wielding.0, state.1 as i64 - solid() as i64, state.2);
        state.0 = 1;
    }
    for (at, name) in [(2.7, "torch"), (3.5, "wand"), (4.2, "bomb"), (5.3, "axe")] {
        if (at..at + 0.02).contains(&t) {
            info!("held: {name}: wielding {:?}, swinging {}", wielding.0, swinging);
        }
    }
    for key in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit6] {
        match (want.contains(&key), keys.pressed(key)) {
            (true, false) => keys.press(key),
            (false, true) => keys.release(key),
            _ => {}
        }
    }
    cursor.0 = aim.or(Some(p + Vec2::new(30.0, 0.0)));
    match (aim.is_some(), mouse.pressed(MouseButton::Left)) {
        (true, false) => mouse.press(MouseButton::Left),
        (false, true) => mouse.release(MouseButton::Left),
        _ => {}
    }
}

/// Through falling sand; out of a pit of water.
fn crossing_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<(u8, f32, f32)>,
) {
    if s.name != "crossing" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let mut want = std::collections::HashSet::new();
    if state.0 == 0 && t > 0.3 {
        state.1 = k.body.pos.x;
        state.0 = 1;
    }
    let x0 = state.1 as i32;
    // Sand pouring from high up, 30 cells ahead.
    if (0.3..3.0).contains(&t)
        && let Some(sand) = sim.materials().id("sand")
    {
        sim.queue(WorldEdit::Paint { center: CellPos::new(x0 + 30, floor + 70), radius: 1, material: sand, overwrite: false });
    }
    if (1.2..3.0).contains(&t) {
        want.insert(KeyCode::KeyD);
    }
    if state.0 == 1 && t > 3.0 {
        info!("crossing: walked {:.0} cells through a sand stream 30 ahead (past it: {})", k.body.pos.x - state.1, k.body.pos.x - state.1 > 36.0);
        // A pit: 30 wide, 40 deep, water to 12 below the rim.
        let px = x0 + 90;
        sim.queue(WorldEdit::Dig { center: CellPos::new(px, floor - 20), radius: 20, max_hardness: 250 });
        if let Some(water) = sim.materials().id("water") {
            for y in (floor - 40..floor - 12).step_by(4) {
                sim.queue(WorldEdit::Paint { center: CellPos::new(px, y), radius: 16, material: water, overwrite: false });
            }
        }
        state.0 = 2;
    }
    if state.0 == 2 && t > 3.6 {
        k.body.pos = Vec2::new((x0 + 90) as f32, (floor - 22) as f32);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
        state.0 = 3;
        state.2 = f32::MIN;
    }
    if state.0 == 3 {
        // Swim up (hold jump), then at the surface jump for the left bank.
        let wet = k.loco.contacts.submerged;
        if t < 7.5 {
            if wet > 0.0 || t < 4.0 {
                want.insert(KeyCode::Space);
            }
            if t > 4.5 {
                want.insert(KeyCode::KeyA);
            }
            // (Let go now and then, so a new press can come.)
            if (t * 3.0).fract() < 0.15 {
                want.remove(&KeyCode::Space);
            }
        }
        state.2 = state.2.max(k.body.bottom());
        if t > 7.5 {
            info!("crossing: out of the pit: feet at {:+.0} from the rim (highest {:+.0}), {}", k.body.bottom() - floor as f32, state.2 - floor as f32, if k.body.bottom() >= floor as f32 - 0.5 && k.loco.contacts.submerged < 0.1 { "out" } else { "still in" });
            state.0 = 4;
        }
    }
    for key in [KeyCode::KeyD, KeyCode::KeyA, KeyCode::Space] {
        match (want.contains(&key), keys.pressed(key)) {
            (true, false) => keys.press(key),
            (false, true) => keys.release(key),
            _ => {}
        }
    }
}

/// Fireflies, fish and bats put where they live, and checked on.
fn life_script(
    s: Res<Scenario>,
    mut commands: Commands,
    sim: Res<SimWorld>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    critters: Query<(&crate::actors::Creature, &Kinematics), Without<LocalPlayer>>,
    mut state: Local<u8>,
) {
    if s.name != "life" {
        return;
    }
    let floor = platypus_worldgen::arena::FLOOR as f32;
    if *state == 0 && s.elapsed > 0.5 {
        // The player by the pool, to watch.
        if let Ok(mut k) = player.single_mut() {
            k.body.pos = Vec2::new(440.0, floor + 8.0);
            k.prev_pos = k.body.pos;
        }
        for i in 0..6 {
            let x = 460.0 + i as f32 * 12.0;
            crate::actors::creature::spawn_creature(&mut commands, "firefly", Vec2::new(x, floor + 8.0 + (i % 3) as f32 * 6.0), |_| {});
        }
        for i in 0..4 {
            crate::actors::creature::spawn_creature(&mut commands, "fish", Vec2::new(330.0 + i as f32 * 22.0, floor - 20.0 - (i % 2) as f32 * 12.0), |_| {});
        }
        for i in 0..4 {
            crate::actors::creature::spawn_creature(&mut commands, "bat", Vec2::new(500.0 + i as f32 * 14.0, floor + 40.0 + i as f32 * 8.0), |_| {});
        }
        *state = 1;
    }
    if *state == 1 && s.elapsed > 5.5 {
        let world = &sim.world;
        let wet = |p: Vec2| world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| world.materials().phys(c.material).kind == platypus_sim::Kind::Liquid);
        for kind in ["firefly", "fish", "bat"] {
            let all: Vec<&Kinematics> = critters.iter().filter(|(c, _)| c.kind == kind).map(|(_, k)| k).collect();
            let fine = all
                .iter()
                .filter(|k| match kind {
                    "fish" => wet(k.body.pos),
                    _ => !k.loco.grounded() && k.body.pos.y > floor + 2.0,
                })
                .count();
            let heights: Vec<String> = all.iter().map(|k| format!("{:.0}", k.body.pos.y - floor)).collect();
            info!("life: {kind}: {fine} of {} where they live (heights over the floor: {})", all.len(), heights.join(" "));
        }
        *state = 2;
    }
}

/// A phase: its name, what's put (kind, dx from the player), how long.
type Phase<'a> = (&'a str, &'a [(&'a str, f32)], f32);

/// Creatures but the player and the arena's dummies.
type Foes = (Without<LocalPlayer>, Without<crate::actors::dummy::Dummy>);

/// The underground enemies, one kind at a time, against a player who stands.
#[allow(clippy::too_many_arguments)]
fn underground_script(
    s: Res<Scenario>,
    mut commands: Commands,
    mut player: Query<(&mut Kinematics, &mut crate::actors::Health), With<LocalPlayer>>,
    foes: Query<(Entity, &crate::actors::Creature, &Kinematics), Foes>,
    deaths: Res<crate::actors::PlayerDeaths>,
    mut state: Local<(usize, f32, f32, u32, f32)>,
) {
    if s.name != "underground" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR as f32;
    // (name, what: (kind, dx from the player), how long)
    let phases: [Phase; 6] = [
        ("spider", &[("spider", 40.0)], 3.5),
        ("slimes", &[("slime", 35.0), ("acid_slime", -35.0)], 3.5),
        ("vampire bats", &[("vampire_bat", 30.0), ("vampire_bat", -30.0)], 4.0),
        ("skeleton", &[("skeleton", 40.0)], 4.0),
        ("egg sac", &[("egg_sac", 28.0)], 3.0),
        ("climb", &[], 5.0),
    ];
    let start: Vec<f32> = phases.iter().scan(0.5, |acc, p| { let a = *acc; *acc += p.2; Some(a) }).collect();
    let (i, _) = (state.0, 0);
    if i >= phases.len() {
        return;
    }
    if t < start[i] {
        return;
    }
    // A phase begins: the floor cleared, the player healed, its foes put.
    if state.4 < start[i] + 0.001 && state.4 <= start[i] {
        for (e, ..) in &foes {
            commands.entity(e).despawn();
        }
        h.hp = h.max;
        state.1 = h.hp;
        state.3 = deaths.0;
        state.2 = 0.0;
        let (name, what, _) = phases[i];
        let at = if name == "climb" {
            // On top of the tall column, a spider at its foot.
            k.body.pos = Vec2::new(1108.0, floor + 140.0 + 8.0);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            crate::actors::creature::spawn_creature(&mut commands, "spider", Vec2::new(1085.0, floor), |_| {});
            k.body.pos
        } else {
            k.body.pos
        };
        for &(kind, dx) in what {
            crate::actors::creature::spawn_creature(&mut commands, kind, Vec2::new(at.x + dx, floor + if kind == "vampire_bat" { 40.0 } else { 0.0 }), |_| {});
        }
        state.4 = start[i] + 0.002;
    }
    // Keep a record: the highest a foe got (the climb).
    for (_, c, fk) in &foes {
        if c.kind == "spider" {
            state.2 = state.2.max(fk.body.pos.y - floor);
        }
    }
    let end = start[i] + phases[i].2;
    if t >= end - 0.05 {
        let lost = state.1 - h.hp + 100.0 * (deaths.0 - state.3) as f32;
        let mut kinds = std::collections::BTreeMap::new();
        for (_, c, _) in &foes {
            *kinds.entry(c.kind.clone()).or_insert(0) += 1;
        }
        let extra = if phases[i].0 == "climb" { format!("; the spider got {:.0} cells up (the player is at {:.0})", state.2, k.body.pos.y - floor) } else { String::new() };
        info!("underground: {}: the player lost {lost:.0} hp in {:.1} s; about: {kinds:?}{extra}", phases[i].0, phases[i].2);
        state.0 += 1;
        if state.0 < phases.len() {
            state.4 = 0.0;
        }
    }
}

/// Into the nearest spider nest, torch in hand.
fn nest_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut toggles: ResMut<crate::light::LightToggles>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    foes: Query<(&crate::actors::Creature, &Kinematics), Without<LocalPlayer>>,
    mut state: Local<(u8, Option<Vec2>)>,
) {
    if s.name != "nest" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    if state.0 == 0 && s.elapsed > 0.3 {
        let home = sim.generator.spawn_point();
        let nest = sim
            .generator
            .landmarks()
            .into_iter()
            .filter(|(_, n)| n == "spider nest")
            .min_by_key(|(p, _)| (p.x - home.x).abs() + (p.y - home.y).abs());
        match nest {
            Some((p, _)) => {
                info!("nest: a spider nest at {},{} ({} cells down from the start)", p.x, p.y, home.y - p.y);
                let at = Vec2::new(p.x as f32 - 20.0, p.y as f32);
                k.body.pos = at;
                k.body.vel = Vec2::ZERO;
                k.prev_pos = at;
                state.1 = Some(at);
            }
            None => info!("nest: no spider nest in this world"),
        }
        toggles.carry = crate::light::Carry::Torch;
        state.0 = 1;
    }
    // (Held there while its chunks load, so it doesn't fall away first.)
    if state.0 == 1 && s.elapsed < 1.5
        && let Some(at) = state.1
    {
        k.body.pos = at;
        k.body.vel = Vec2::ZERO;
    }
    if state.0 == 1 && s.elapsed > 3.3 {
        let mut n = std::collections::BTreeMap::new();
        for (c, fk) in &foes {
            if fk.body.pos.distance(k.body.pos) < 150.0 {
                *n.entry(c.kind.clone()).or_insert(0) += 1;
            }
        }
        info!("nest: about the player: {n:?}");
        state.0 = 2;
    }
}

/// How far a walk gets through open air, then through web.
fn webs_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, mut player: Query<&mut Kinematics, With<LocalPlayer>>, mut keys: ResMut<ButtonInput<KeyCode>>, mut state: Local<(u8, f32)>) {
    if s.name != "webs" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let walking = (1.0..2.5).contains(&t) || (3.5..5.0).contains(&t);
    match (walking, keys.pressed(KeyCode::KeyD)) {
        (true, false) => keys.press(KeyCode::KeyD),
        (false, true) => keys.release(KeyCode::KeyD),
        _ => {}
    }
    if state.0 == 0 && t > 1.0 {
        state.1 = k.body.pos.x;
        state.0 = 1;
    }
    if state.0 == 1 && t > 2.5 {
        info!("webs: open air: {:.0} cells in 1.5 s", k.body.pos.x - state.1);
        // Back, and a thicket of web ahead.
        k.body.pos.x = 560.0;
        k.prev_pos = k.body.pos;
        if let Some(web) = sim.materials().id("cobweb") {
            for dx in (0..60).step_by(8) {
                sim.queue(WorldEdit::Paint { center: CellPos::new(575 + dx, floor + 8), radius: 8, material: web, overwrite: false });
            }
        }
        state.0 = 2;
    }
    if state.0 == 2 && t > 3.5 {
        state.1 = k.body.pos.x;
        state.0 = 3;
    }
    if state.0 == 3 && t > 5.0 {
        info!("webs: through web: {:.0} cells in 1.5 s", k.body.pos.x - state.1);
        state.0 = 4;
    }
}

/// Armour against a blast, and the equipment screen.
#[allow(clippy::too_many_arguments)]
fn gear_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    mut window: Single<&mut Window, With<bevy::window::PrimaryWindow>>,
    slots: Query<(&crate::hands::ui::SlotUi, &bevy::ui::UiGlobalTransform, &InheritedVisibility)>,
    mut player: Query<(&Kinematics, &mut crate::actors::Health, &mut crate::gear::Equipment, &crate::gear::Stats, &mut crate::hands::items::Inventory), With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<(u8, f32)>,
) {
    use crate::hands::items::Stack;
    if s.name != "gear" {
        return;
    }
    let (Some(items), Ok((k, mut h, mut eq, stats, mut inv))) = (items, player.single_mut()) else { return };
    let t = s.elapsed;
    let p = k.body.pos;
    keys.release(KeyCode::Escape);
    let blast = |sim: &mut SimWorld| sim.queue(WorldEdit::Explode { center: CellPos::new(p.x as i32 + 3, p.y as i32), radius: 3, power: 60 });
    match state.0 {
        0 if t > 0.8 => {
            state.1 = h.hp;
            blast(&mut sim);
            state.0 = 1;
        }
        1 if t > 1.4 => {
            info!("gear: the blast with nothing on cost {:.0} hp", state.1 - h.hp);
            h.hp = h.max;
            for (i, id) in ["iron_helm", "chainmail", "iron_gauntlets", "iron_greaves", "iron_boots", "ring_of_vigour"].iter().enumerate() {
                eq.worn[i] = items.id(id).map(|it| Stack::new(it, 1));
            }
            if let Some(j) = items.id("leather_jerkin") {
                inv.add(&items, Stack::new(j, 1));
            }
            state.0 = 2;
        }
        2 if t > 1.8 => {
            let listed: Vec<String> = stats.nonzero().map(|(s, v)| crate::gear::stats::line(s, v)).collect();
            info!("gear: in iron: health {:.0}/{:.0}, {}", h.hp, h.max, listed.join(", "));
            state.1 = h.hp;
            blast(&mut sim);
            state.0 = 3;
        }
        3 if t > 2.4 => {
            info!("gear: the same blast in iron cost {:.0} hp", state.1 - h.hp);
            keys.press(KeyCode::Escape);
            state.0 = 4;
        }
        4 if t > 2.8 => {
            let jerkin = items.id("leather_jerkin");
            let at = inv.slots.iter().position(|s| s.is_some_and(|s| Some(s.item) == jerkin));
            let scale = window.scale_factor();
            let pos = at.and_then(|i| slots.iter().find(|(sl, _, v)| sl.0 == crate::hands::ui::Holder::Pack && sl.1 == i && v.get()).map(|(_, tf, _)| tf.translation / scale));
            info!("gear: hovering the jerkin in slot {at:?} at {pos:?}");
            window.set_cursor_position(pos);
            state.0 = 5;
        }
        _ => {}
    }
}
