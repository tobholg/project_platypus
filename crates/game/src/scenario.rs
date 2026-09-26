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
//! - `inventory`  opens the inventory screen (Esc) at 1 s, switches to the
//!   second hotbar (X) at 1.5 s, hovers the spark wand at 2 s (its tooltip;
//!   this moves the real mouse pointer), drags it to hotbar 3 at 2.6–3 s;
//!   logs where it ended up
//!
//! Prints one line per second and a summary, then exits.
//! `PLATYPUS_SCREENSHOT=out.png` saves the window one second before the end.

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
            .add_systems(Update, (tree_script, blast_script, fell_script, acid_script, rain_script, swim_script, dark_script, flood_script))
            .add_systems(PreUpdate, (hands_script, chest_script, drop_script, chestfall_script, zoom_script, shroom_script, magic_script, shock_script, inventory_script, well_script, force_script).after(InputSystems).before(crate::camera::track_cursor));
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
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
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
        toggles.flashlight = std::env::var("PLATYPUS_NOBEAM").is_err();
        toggles.torch = true;
        cursor.0 = Some(k.body.pos + Vec2::new(90.0, -10.0));
        // A torch planted to the left, glow sticks thrown both ways.
        crate::light::plant_torch(&mut commands, k.body.pos + Vec2::new(-40.0, -4.0), &lights);
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
            inv.slots[8] = Some(Stack { item: chest, count: 1 });
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
                    chests.contents(c, &sim.world, &items).add(&items, Stack { item: bomb, count: taken });
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
