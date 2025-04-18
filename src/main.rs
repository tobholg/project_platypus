//! minimal bootstrap for the Terraria‑like demo
//!
//! Updated for inventory, pickaxe mining, gun shooting, debris & bullets.
//! Works with **Bevy 0.15**, Rust 1.77.

mod camera;
mod components;
mod constants;
mod enemy;
mod player;
mod terrain;
mod visibility;

use bevy::diagnostic::{
    EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
    LogDiagnosticsPlugin,
};
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};

use camera::camera_follow_system;
use player::{
    animate_player_system, bullet_update_system, debris_update_system,
    exhaust_update_system, gun_shoot_system, inventory_input_system,
    physics_and_collision_system, pickaxe_mining_system, player_input_system,
};
use terrain::{
    generate_world_and_player, redraw_changed_tiles_system, stream_tiles_system,
    update_active_rect_system,
};
use visibility::{
    detect_player_tile_change_system, recompute_fov_system, startup_fov_system,
};

/* ------------------------------------------------------------------------ */
/* camera                                                                   */
/* ------------------------------------------------------------------------ */
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/* ------------------------------------------------------------------------ */
/* F11 borderless‑fullscreen toggle                                         */
/* ------------------------------------------------------------------------ */
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

/* ------------------------------------------------------------------------ */
/* main                                                                     */
/* ------------------------------------------------------------------------ */
fn main() {
    App::new()
        /* diagnostics ----------------------------------------------------- */
        .add_plugins((
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
        ))

        /* bevy core ------------------------------------------------------- */
        .insert_resource(ClearColor(Color::srgb(0.18, 0.65, 1.0)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (1280., 720.).into(),
                mode: WindowMode::Windowed,
                ..default()
            }),
            ..default()
        }))

        /* startup systems ------------------------------------------------- */
        .add_systems(Startup, generate_world_and_player)
        .add_systems(
            Startup,
            enemy::spawn_enemies.after(generate_world_and_player),
        )
        .add_systems(Startup, setup_camera)
        .add_systems(
            Startup,
            update_active_rect_system.after(setup_camera),
        ) // ensure ActiveRect exists
        .add_systems(Startup, startup_fov_system.after(setup_camera))

        /* frame‑update systems ------------------------------------------- */
        .add_systems(
            Update,
            (
                /* player -------------------------------------------------- */
                inventory_input_system,        // 1/2 hot‑keys
                player_input_system,           // WASD + jump
                physics_and_collision_system,  // movement & collide
                pickaxe_mining_system,         // hold LMB with pickaxe
                gun_shoot_system,              // click to shoot
                bullet_update_system,          // bullet physics
                debris_update_system,          // mining debris fade
                exhaust_update_system,         // jet‑pack exhaust fade
                animate_player_system,         // walk cycle

                /* world --------------------------------------------------- */
                stream_tiles_system,           // stripe‑diff sprite stream
                redraw_changed_tiles_system,   // tint / show / pool

                /* enemies ------------------------------------------------- */
                enemy::update_active_tag_system,
                enemy::enemy_ai_system,
                enemy::enemy_physics_system,
                enemy::animate_enemy_system,

                /* misc ---------------------------------------------------- */
                toggle_fullscreen,
                detect_player_tile_change_system,
            ),
        )

        /* post‑update (camera / FOV) -------------------------------------- */
        .add_systems(
            PostUpdate,
            (
                camera_follow_system,
                update_active_rect_system, // slide rect with camera
                recompute_fov_system,
            ),
        )
        .run();
}