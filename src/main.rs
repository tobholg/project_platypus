//! tiny bootstrap – all real code lives in the modules
mod constants;
mod components;
mod terrain;
mod player;
mod enemy;
mod camera;
mod visibility;                 //  ← NEW

use bevy::prelude::*;
use bevy::input::ButtonInput;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};

use terrain::{
    generate_world_and_player, spawn_initial_tiles, digging_system,
    redraw_changed_tiles_system,
};
use player::{
    player_input_system, physics_and_collision_system, animate_player_system,
    exhaust_update_system,
};
use camera::camera_follow_system;
use visibility::{
    detect_player_tile_change_system, recompute_fov_system, startup_fov_system,
};

/* --------------- camera --------------- */
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/* --------------- F11 full‑screen toggle --------------- */
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
        .insert_resource(ClearColor(Color::srgb(0.18, 0.65, 1.0)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Terraria‑like (chunky cave colours)".into(),
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        /* ---------- startup ---------- */
        .add_systems(Startup, generate_world_and_player)                 // inserts Terrain
        .add_systems(Startup, enemy::spawn_enemies.after(generate_world_and_player))
        .add_systems(Startup, setup_camera)
        .add_systems(Startup, startup_fov_system.after(setup_camera))    // ← initial FOV

        /* ---------- one‑shot after terrain exists ---------- */
        .add_systems(Update, spawn_initial_tiles.before(player_input_system))

        /* ---------- main game loop (physics, input, etc.) ---------- */
        .add_systems(
            Update,
            (
                player_input_system,
                physics_and_collision_system,
                enemy::enemy_ai_system,
                enemy::enemy_physics_system,
                digging_system,
                redraw_changed_tiles_system,
                exhaust_update_system,
                animate_player_system,
                enemy::animate_enemy_system,
                toggle_fullscreen,
                detect_player_tile_change_system,         // ← NEW
            ),
        )

        /* ---------- camera & visibility  ---------- */
        .add_systems(PostUpdate, (                        // run after movement
            camera_follow_system,
            recompute_fov_system,                         // ← NEW (run_if internal)
        ))

        .run();
}