//! tiny bootstrap – all real code lives in the modules
mod constants;
mod components;
mod terrain;
mod player;
mod enemy;
mod camera;
mod visibility;

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};

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
                title: "Terraria‑like (streaming tiles)".into(),
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))

        /* ---------- startup ---------- */
        .add_systems(Startup, generate_world_and_player) // inserts Terrain
        .add_systems(Startup, enemy::spawn_enemies.after(generate_world_and_player))
        .add_systems(Startup, setup_camera)
        .add_systems(Startup, update_active_rect_system.after(setup_camera))
        .add_systems(Startup, startup_fov_system.after(setup_camera))

        /* ---------- main update loop ---------- */
        .add_systems(
            Update,
            (
                player_input_system,
                physics_and_collision_system,
                enemy::update_active_tag_system,    // NEW – tag/untag
                enemy::enemy_ai_system,
                enemy::enemy_physics_system,
                terrain::stream_tiles_system,       // NEW – spawn/kill tile sprites
                digging_system,
                redraw_changed_tiles_system,
                exhaust_update_system,
                animate_player_system,
                enemy::animate_enemy_system,
                toggle_fullscreen,
                detect_player_tile_change_system,
            ),
        )

        /* ---------- camera + active‑rect + FOV ---------- */
        .add_systems(
            PostUpdate,
            (
                camera_follow_system,
                update_active_rect_system,  // NEW – compute rect each frame
                recompute_fov_system,
            ),
        )

        .run();
}