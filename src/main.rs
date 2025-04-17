//! tiny bootstrap – all real code lives in the modules
mod constants;
mod components;
mod terrain;
mod player;
mod camera;

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowMode};

use terrain::{
    generate_world_and_player, spawn_initial_tiles, digging_system, redraw_changed_tiles_system,
};
use player::{
    player_input_system, physics_and_collision_system, animate_player_system,
    exhaust_update_system,
};
use camera::camera_follow_system;

/// one‑shot camera spawn
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

/// Toggle between fullscreen and windowed whenever the user presses F11.
/// – `Input<KeyCode>` is the keyboard resource in Bevy 0.10.  [oai_citation_attribution:0‡GitHub](https://github.com/bevyengine/bevy/issues/8391)
fn toggle_fullscreen(
    keys: Res<Input<KeyCode>>,
    mut window_q: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        let mut window = window_q.single_mut();
        window.mode = match window.mode {
            WindowMode::Windowed => WindowMode::BorderlessFullscreen, // ← no argument in 0.10  [oai_citation_attribution:1‡GitHub](https://github.com/bevyengine/bevy/issues/5875)
            _ => WindowMode::Windowed,
        };
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.25, 0.55, 1.0)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Terraria‑like (chunky cave colours)".into(),
                mode: WindowMode::BorderlessFullscreen,
                // resolution is filled in automatically in this mode
                ..default()
            }),
            ..default()
        }))
        .add_startup_system(generate_world_and_player)
        .add_startup_system(setup_camera.after(generate_world_and_player))
        .add_system(spawn_initial_tiles)          // one‑shot
        .add_systems((
            player_input_system,
            physics_and_collision_system,
            digging_system,
            redraw_changed_tiles_system,
            exhaust_update_system,
            animate_player_system,
            camera_follow_system,
            toggle_fullscreen,                    // ← new
        ))
        .run();
}