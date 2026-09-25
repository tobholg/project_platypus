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
//! - `acid`       pours acid into a glass basin beside the player, boils it at 2 s, lights the fumes at 3.6 s
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
            .add_systems(Update, (tree_script, blast_script, fell_script, acid_script));
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
) {
    if s.name != "tools" {
        return;
    }
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
    if !matches!(s.name.as_str(), "fell" | "burn") || *done || s.elapsed < 2.0 {
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
        } else {
            // Twice: a dig clears the playfield first where anything stands in front.
            for _ in 0..2 {
                sim.queue(WorldEdit::Dig { center, radius: width / 2 + 3, max_hardness: 200 });
            }
        }
        info!("{}: trunk {width} wide at {center:?}", s.name);
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
