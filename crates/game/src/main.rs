//! Project Platypus. See SPEC.md.
//!
//! Environment:
//! - `PLATYPUS_SEED`      world seed (default 1)
//! - `PLATYPUS_WORLD`     `large` (default: 131 072 × 32 768 cells), `medium`
//!   (the reference world, a quarter as wide and half as deep), `small`,
//!   `flat` (an empty box) or
//!   `arena` (the sandbox: dummies, time controls, overlays, the art editor)
//! - `PLATYPUS_SCENARIO`  scripted perf run, see `scenario.rs`
//! - `PLATYPUS_SAVE`      the save to use (default: world and seed, `large-1`;
//!   `off`: none; scenarios and the arena and flat sandboxes save only when
//!   it's given), `PLATYPUS_FRESH=1`
//!   to start it over: see `save.rs`
//! - none of these: the title screen (`title.rs`; `PLATYPUS_TITLE=1` shows
//!   it anyway, `0` never)
//! - `PLATYPUS_MODE`      `play` (a new game's kit, no dev tools) or `dev`
//!   (the default; a save keeps its own): see `mode.rs`

mod archery;
mod arena;
mod backdrop;
mod bestiary;
mod camera;
mod clock;
mod cloth;
mod canvas;
mod combat;
mod craft;
mod creatures;
mod data;
mod debug;
mod dev;
mod devmenu;
mod difficulty;
mod display;
mod editor;
mod events;
mod fight;
mod fx;
mod gear;
mod gold;
mod pickups;
mod cursor;
mod hands;
mod hud;
mod kick;
mod light;
mod magic;
mod mode;
mod particles;
mod pause;
mod potion;
mod observe;
mod progress;
mod props;
mod render;
mod replay;
mod reset;
mod respawn;
mod rigid;
mod save;
mod scenario;
mod screen_fx;
mod show;
#[cfg(feature = "spikes")]
mod spikes;
mod talk;
mod tempo;
mod title;
mod sound;
mod tools;
mod traps;
mod vfx;
mod world;

use std::sync::Arc;

use bevy::prelude::*;
use bevy::window::PresentMode;
use platypus_sim::MaterialTable;
use platypus_worldgen::{ArenaGen, ChunkGenerator, FlatGen, Preset, TerrainGen};

fn main() {
    // The sim's workers (rayon) share the cores with Bevy's: about two
    // thirds of them (8 of an M2 Max's 12: its efficiency cores slowed a
    // pass more than they helped). PLATYPUS_SIM_THREADS overrides.
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    let sim_threads = std::env::var("PLATYPUS_SIM_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or((cores * 2 / 3).max(2));
    let _ = rayon::ThreadPoolBuilder::new().num_threads(sim_threads).build_global();
    let materials_path = data::data_path("materials.ron");
    let src = std::fs::read_to_string(&materials_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", materials_path.display()));
    let materials = Arc::new(MaterialTable::from_ron(&src).unwrap_or_else(|e| panic!("{e}")));

    // Started plainly (no world, scenario or save named): the title screen,
    // over the last save there is or a new game's world (`title.rs`).
    let named = |k: &str| std::env::var(k).is_ok();
    // (A scenario with PLATYPUS_TITLE=1 sees the title as a plain start would.)
    let forced = std::env::var("PLATYPUS_TITLE").is_ok_and(|v| v == "1");
    let plain = !named("PLATYPUS_WORLD") && (!named("PLATYPUS_SCENARIO") || forced) && !named("PLATYPUS_SAVE");
    let show_title = match std::env::var("PLATYPUS_TITLE").ok().as_deref() {
        Some("0" | "off") => false,
        Some(_) => true,
        None => plain && !camera::headless(),
    };
    let behind = (show_title && plain).then(title::behind_the_title);
    let seed: u64 = match &behind {
        Some((l, _)) => l.seed,
        None => std::env::var("PLATYPUS_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(1),
    };
    let kind = match &behind {
        Some((l, _)) => l.kind.clone(),
        None => std::env::var("PLATYPUS_WORLD").unwrap_or_else(|_| "large".into()),
    };
    let save = match behind.as_ref().map_or_else(|| std::env::var("PLATYPUS_SAVE").ok(), |(l, _)| Some(l.save.clone().unwrap_or_else(|| "off".into()))).as_deref() {
        Some("off" | "") => None,
        Some(name) => Some(name.to_string()),
        // (A scenario doesn't touch your save unless told to; the sandboxes
        // start clean every time.)
        None if std::env::var("PLATYPUS_SCENARIO").is_ok() || matches!(kind.as_str(), "arena" | "flat") => None,
        None => Some(format!("{kind}-{seed}")),
    };
    // Play or dev: as asked, else as the save was, else dev (`mode.rs`).
    let mode = behind.as_ref().map(|(l, _)| l.mode).or_else(mode::Mode::from_env).or_else(|| save.as_deref().and_then(save::peek).map(|s| s.mode)).unwrap_or_default();
    // How hard it hits: as asked, else as the save was, else Wayfarer.
    let difficulty = behind.as_ref().and_then(|(l, _)| l.difficulty).or_else(difficulty::Difficulty::from_env).or_else(|| save.as_deref().and_then(save::peek).map(|s| s.difficulty)).unwrap_or_default();
    let (launch, fresh) = behind.unwrap_or_else(|| (title::Launch { kind: kind.clone(), seed, save: save.clone(), mode, difficulty: None }, false));
    let generator: Arc<dyn ChunkGenerator> = match kind.as_str() {
        // PLATYPUS_WORLD=arena: a sandbox for weapons, spells and creatures
        // (dummies, time controls, overlays, the art editor: `arena.rs`).
        // PLATYPUS_ARENA: its layout (`flat`, `cave`, `slopes`, `stairs`,
        // `real`, `real:SEED:X:Y`; the sandbox by default).
        "arena" => Arc::new(ArenaGen::with_layout(&materials, platypus_worldgen::arena::Layout::parse(&std::env::var("PLATYPUS_ARENA").unwrap_or_default()))),
        "flat" => Arc::new(FlatGen { width_chunks: 96, height_chunks: 36, floor: 300, stone: materials.expect_id("stone") }),
        // PLATYPUS_WORLD=small: the small preset (quicker to look around).
        "small" => Arc::new(TerrainGen::new(seed, Preset::Small, &materials).with_lairs(&lairs(), &materials)),
        "medium" => Arc::new(TerrainGen::new(seed, Preset::Medium, &materials).with_lairs(&lairs(), &materials)),
        _ => Arc::new(TerrainGen::new(seed, Preset::Large, &materials).with_lairs(&lairs(), &materials)),
    };
    let spawn = generator.spawn_point();
    // Scenarios run uncapped, unless PLATYPUS_VSYNC=1 (to see the frame
    // pacing a player gets).
    let benchmarking = std::env::var("PLATYPUS_SCENARIO").is_ok() && std::env::var("PLATYPUS_VSYNC").is_err();
    // Offscreen (scenarios, tests): no window, so a run never takes the
    // focus (macOS brings a new window's app to the front); frames run on
    // as fast as they can instead of with the screen.
    let headless = camera::headless();
    let mut plugins = DefaultPlugins.build();
    if headless {
        plugins = plugins.disable::<bevy::winit::WinitPlugin>().add(bevy::app::ScheduleRunnerPlugin::run_loop(std::time::Duration::ZERO));
    }

    let exit = App::new()
        // A command on something that's gone this tick (a creature killed by
        // one thing while another acts on it) is logged, not fatal.
        .set_error_handler(bevy::ecs::error::warn)
        .add_plugins(
            plugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: title::GameInfo::load().title,
                        resolution: (1920, 1080).into(),
                        // Uncapped while measuring, so numbers aren't hidden behind vsync.
                        present_mode: if benchmarking { PresentMode::AutoNoVsync } else { PresentMode::AutoVsync },
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin { file_path: data::assets_dir().to_string_lossy().into_owned(), ..default() })
                .set(log_plugin()),
        )
        .add_plugins((
            world::WorldPlugin { seed, materials, materials_path, generator },
            render::ChunkRenderPlugin,
            camera::CameraPlugin { start: Vec2::new(spawn.x as f32, spawn.y as f32 + 60.0) },
            tools::ToolsPlugin,
            hands::HandsPlugin,
            props::PropsPlugin,
            rigid::RigidPlugin,
            particles::ParticlesPlugin,
            creatures::CreaturesPlugin,
            debug::DebugPlugin,
            fx::FxPlugin,
            hud::HudPlugin,
            light::LightPlugin,
            scenario::ScenarioPlugin,
        ))
        .add_plugins((dev::DevPlugin, magic::MagicPlugin, vfx::VfxPlugin, arena::ArenaPlugin, editor::EditorPlugin, combat::CombatPlugin, archery::ArcheryPlugin, gear::GearPlugin))
        .add_plugins((replay::ReplayPlugin, fight::FightPlugin, bestiary::panel::BestiaryPlugin, bestiary::stage::StagePlugin, bestiary::cli::CliPlugin))
        .add_plugins((reset::ResetPlugin, clock::ClockPlugin, gold::GoldPlugin, kick::KickPlugin, traps::TrapsPlugin, talk::TalkPlugin, events::EventsPlugin, show::ShowPlugin, display::DisplayPlugin, pickups::PickupsPlugin, cursor::CursorPlugin))
        .add_plugins((tempo::TempoPlugin, potion::PotionPlugin, screen_fx::ScreenFxPlugin, cloth::ClothPlugin))
        .add_plugins(sound::SoundPlugin)
        .add_plugins(backdrop::BackdropPlugin)
        .insert_resource(mode)
        .insert_resource(difficulty)
        .add_plugins(difficulty::DifficultyPlugin)
        .add_plugins((respawn::RespawnPlugin, devmenu::DevMenuPlugin, pause::PausePlugin, title::TitlePlugin { show: show_title, behind: launch, fresh }))
        .add_plugins((progress::ProgressPlugin, craft::CraftPlugin, save::SavePlugin { name: save, kind, seed, mode }))
        .add_plugins(spikes_plugin)
        .add_systems(Last, pace_frames)
        .run();
    // (The title or the game menu asked for another world: `title.rs`.)
    title::relaunch_if_asked();
    // (A replay run as a test says how it went: `replay.rs`.)
    if let AppExit::Error(code) = exit {
        std::process::exit(code.get() as i32);
    }
}

/// `PLATYPUS_FRAME_MS=8.33`: frames no quicker than that (a run offscreen
/// paced as a player's screen paces it: timing-dependent behaviour shows
/// as it would for them).
fn pace_frames(mut last: Local<Option<std::time::Instant>>, mut ms: Local<Option<Option<f32>>>) {
    let Some(ms) = *ms.get_or_insert_with(|| std::env::var("PLATYPUS_FRAME_MS").ok().and_then(|v| v.parse().ok())) else { return };
    let frame = std::time::Duration::from_secs_f32(ms / 1000.0);
    if let Some(at) = *last {
        let spent = at.elapsed();
        if spent < frame {
            std::thread::sleep(frame - spent);
        }
    }
    *last = Some(std::time::Instant::now());
}

/// The lairs in the caves (`assets/data/lairs.ron`).
fn lairs() -> Vec<platypus_worldgen::lairs::LairDef> {
    data::load_ron(&data::data_path("lairs.ron")).unwrap_or_else(|e| {
        warn!("{e}");
        Vec::new()
    })
}

#[cfg(feature = "spikes")]
fn log_plugin() -> bevy::log::LogPlugin {
    bevy::log::LogPlugin { custom_layer: spikes::layer, ..default() }
}

#[cfg(not(feature = "spikes"))]
fn log_plugin() -> bevy::log::LogPlugin {
    default()
}

fn spikes_plugin(_app: &mut App) {
    #[cfg(feature = "spikes")]
    _app.add_plugins(spikes::SpikesPlugin);
}
