//! tiny bootstrap – all real code lives in the modules
mod constants;
mod components;
mod terrain;
mod player;
mod camera;

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
        .insert_resource(ClearColor(Color::srgb(0.25, 0.55, 1.0)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Terraria‑like (chunky cave colours)".into(),
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        /* ---------- startup ---------- */
        .add_systems(Startup, (generate_world_and_player, setup_camera))
        /* ---------- one‑shot after terrain exists ---------- */
        .add_systems(Update, spawn_initial_tiles.before(player_input_system))
        /* ---------- main game loop ---------- */
        .add_systems(
            Update,
            (
                player_input_system,
                physics_and_collision_system,
                digging_system,
                redraw_changed_tiles_system,
                exhaust_update_system,
                animate_player_system,
                camera_follow_system,
                toggle_fullscreen,
            ),
        )
        .run();
}