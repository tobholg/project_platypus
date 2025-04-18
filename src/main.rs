//! minimal bootstrap (unchanged logic, imports updated)

mod camera;
mod components;
mod constants;
mod enemy;
mod player;
mod terrain;
mod visibility;

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};

use bevy::diagnostic::{
    FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
    EntityCountDiagnosticsPlugin,
};

use camera::camera_follow_system;
use player::{
    animate_player_system, exhaust_update_system, physics_and_collision_system,
    player_input_system,
};
use terrain::{
    digging_system, generate_world_and_player, redraw_changed_tiles_system,
    stream_tiles_system, update_active_rect_system,
};
use visibility::{
    detect_player_tile_change_system, recompute_fov_system, startup_fov_system,
};

/* camera ------------------------------------------------------------------- */
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/* F11 toggle --------------------------------------------------------------- */
fn toggle_fullscreen(
    keys: Res<ButtonInput<KeyCode>>,
    mut window_q: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::F11) {
        let mut window = window_q.single_mut();
        window.mode = match window.mode {
            WindowMode::Windowed => {
                WindowMode::BorderlessFullscreen(MonitorSelection::Primary)
            }
            _ => WindowMode::Windowed,
        };
    }
}

fn main() {
    App::new()
        .add_plugins((
            LogDiagnosticsPlugin::default(),          // prints per‑system times
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
        ))
        .insert_resource(ClearColor(Color::srgb(0.18, 0.65, 1.0)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                //title: "Terraria‑like (streaming tiles)".into(),
                //mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                resolution: (800., 450.).into(),      // NEW – set window size
                mode: WindowMode::Windowed,
                ..default()
            }),
            ..default()
        }))

        /* startup ---------------------------------------------------------- */
        .add_systems(Startup, generate_world_and_player)
        .add_systems(Startup, enemy::spawn_enemies.after(generate_world_and_player))
        .add_systems(Startup, setup_camera)
        .add_systems(Startup, update_active_rect_system.after(setup_camera)) // ensure ActiveRect exists
        .add_systems(Startup, startup_fov_system.after(setup_camera))

        /* update ----------------------------------------------------------- */
        .add_systems(
            Update,
            (
                player_input_system,
                physics_and_collision_system,
                enemy::update_active_tag_system,
                enemy::enemy_ai_system,
                enemy::enemy_physics_system,
                stream_tiles_system,              // strip‑based streaming
                digging_system,
                redraw_changed_tiles_system,
                exhaust_update_system,
                animate_player_system,
                enemy::animate_enemy_system,
                toggle_fullscreen,
                detect_player_tile_change_system,
            ),
        )

        /* post‑update ------------------------------------------------------ */
        .add_systems(
            PostUpdate,
            (
                camera_follow_system,
                update_active_rect_system, // slide rect
                recompute_fov_system,
            ),
        )
        .run();
}