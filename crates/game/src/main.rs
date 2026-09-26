//! Project Platypus. See SPEC.md.
//!
//! Environment:
//! - `PLATYPUS_SEED`      world seed (default 1)
//! - `PLATYPUS_WORLD`     `terrain` (default), `small`, `flat` (an empty box) or
//!   `arena` (the sandbox: dummies, time controls, overlays, the art editor)
//! - `PLATYPUS_SCENARIO`  scripted perf run, see `scenario.rs`

mod actors;
mod arena;
mod camera;
mod data;
mod debug;
mod dev;
mod fx;
mod hands;
mod hud;
mod light;
mod magic;
mod particles;
mod props;
mod render;
mod rigid;
mod scenario;
#[cfg(feature = "spikes")]
mod spikes;
mod sky;
mod tools;
mod vfx;
mod world;

use std::sync::Arc;

use bevy::prelude::*;
use bevy::window::PresentMode;
use platypus_sim::MaterialTable;
use platypus_worldgen::{ArenaGen, ChunkGenerator, FlatGen, Preset, TerrainGen};

fn main() {
    let seed: u64 = std::env::var("PLATYPUS_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let materials_path = data::data_path("materials.ron");
    let src = std::fs::read_to_string(&materials_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", materials_path.display()));
    let materials = Arc::new(MaterialTable::from_ron(&src).unwrap_or_else(|e| panic!("{e}")));

    let generator: Arc<dyn ChunkGenerator> = match std::env::var("PLATYPUS_WORLD").as_deref() {
        // PLATYPUS_WORLD=arena: a sandbox for weapons, spells and creatures
        // (dummies, time controls, overlays, the art editor: `arena.rs`).
        Ok("arena") => Arc::new(ArenaGen::new(&materials)),
        Ok("flat") => Arc::new(FlatGen { width_chunks: 64, height_chunks: 24, floor: 200, stone: materials.expect_id("stone") }),
        // PLATYPUS_WORLD=small: the small preset (quicker to look around).
        Ok("small") => Arc::new(TerrainGen::new(seed, Preset::Small, &materials)),
        _ => Arc::new(TerrainGen::new(seed, Preset::Large, &materials)),
    };
    let spawn = generator.spawn_point();
    // Scenarios run uncapped, unless PLATYPUS_VSYNC=1 (to see the frame
    // pacing a player gets).
    let benchmarking = std::env::var("PLATYPUS_SCENARIO").is_ok() && std::env::var("PLATYPUS_VSYNC").is_err();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Project Platypus".into(),
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
            camera::CameraPlugin { start: Vec2::new(spawn.x as f32, spawn.y as f32 + 40.0) },
            tools::ToolsPlugin,
            hands::HandsPlugin,
            props::PropsPlugin,
            rigid::RigidPlugin,
            particles::ParticlesPlugin,
            sky::SkyPlugin,
            actors::ActorsPlugin,
            debug::DebugPlugin,
            fx::FxPlugin,
            hud::HudPlugin,
            light::LightPlugin,
            scenario::ScenarioPlugin,
        ))
        .add_plugins((dev::DevPlugin, magic::MagicPlugin, vfx::VfxPlugin, arena::ArenaPlugin))
        .add_plugins(spikes_plugin)
        .run();
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
