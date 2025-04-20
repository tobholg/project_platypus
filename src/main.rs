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
    EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
};
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};
use bevy::ecs::schedule::common_conditions::resource_changed;

use camera::camera_follow_system;
use player::{
    animate_player_system, bullet_update_system, cursor_highlight_system,
    debris_update_system, exhaust_update_system, gun_shoot_system,
    inventory_input_system, physics_and_collision_system, pickaxe_mining_system,
    place_stone_system, player_input_system, health_regen_system,
    dash_start_system, dash_update_system,
};
use terrain::{
    generate_world_and_player, redraw_changed_tiles_system, stream_tiles_system,
    update_active_rect_system,
};
use components::{Health, HealthBarFill, HeldItem, Inventory, Player, ToolbarText, InventorySlot};
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
/* Escape = toggle borderless‑fullscreen                                    */
/* ------------------------------------------------------------------------ */
fn toggle_fullscreen(
    keys: Res<ButtonInput<KeyCode>>,
    mut window_q: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
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
/* HUD (toolbar & health bar)                                               */
/* ------------------------------------------------------------------------ */
fn setup_hud(mut commands: Commands, asset_server: Res<AssetServer>) {
    // ── inventory slots ────────────────────────────────────────────
    for i in 0..3 {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left:  Val::Px(10.0 + i as f32 * 28.0),
                top:   Val::Px(10.0),
                width: Val::Px(24.0),
                height: Val::Px(24.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.0, 1.0, 0.0)),   // bright green
            InventorySlot(i + 1),                          // 1, 2, 3
        ));
    }

    // ── health‑bar background ───────────────────────────────────────────
    let bg = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(10.0),
                width: Val::Px(200.0),
                height: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
        ))
        .id();

    // ── health‑bar fill (child) ─────────────────────────────────────────
    commands.entity(bg).with_children(|parent| {
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.8, 0.0, 0.0)),
            HealthBarFill,
        ));
    });
}

fn add_player_health_system(
    mut commands: Commands,
    q: Query<Entity, Added<Player>>,
) {
    if let Ok(player) = q.get_single() {
        commands.entity(player).insert(Health { current: 100.0, max: 100.0, last_damage: 0.0 });
    }
}

fn update_inventory_hud_system(
    inv_q:  Query<&Inventory>,
    mut q:  Query<(&InventorySlot, &mut BackgroundColor)>,
) {
    if let Ok(inv) = inv_q.get_single() {
        let selected = match inv.selected {
            HeldItem::Pickaxe    => 1,
            HeldItem::Gun        => 2,
            HeldItem::StoneBlock => 3,
        };
        for (slot, mut bg) in &mut q {
            bg.0 = if slot.0 == selected {
                Color::srgb(0.0, 0.7, 0.0)     // darker green
            } else {
                Color::srgb(0.0, 1.0, 0.0)     // bright green
            };
        }
    }
}

fn update_health_bar_system(
    health_q: Query<&Health>,
    mut fill_q: Query<&mut Node, With<HealthBarFill>>,
) {
    if let (Ok(health), Ok(mut node)) = (health_q.get_single(), fill_q.get_single_mut()) {
        let pct = (health.current / health.max).clamp(0.0, 1.0) * 100.0;
        node.width = Val::Percent(pct);
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
        /* engine core ----------------------------------------------------- */
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
        .add_systems(Startup, add_player_health_system.after(generate_world_and_player))
        .add_systems(Startup, setup_camera)
        .add_systems(
            Startup,
            update_active_rect_system.after(setup_camera),
        ) // ensure ActiveRect exists
        .add_systems(Startup, setup_hud.after(setup_camera))
        .add_systems(Startup, startup_fov_system.after(setup_camera))
        /* frame‑update systems ------------------------------------------- */
        .add_systems(
            Update,
            (
                /* player -------------------------------------------------- */
                inventory_input_system,
                cursor_highlight_system,
                player_input_system,
                dash_start_system,
                dash_update_system,
                physics_and_collision_system,
                pickaxe_mining_system,
                place_stone_system,
                gun_shoot_system,
                bullet_update_system,
                debris_update_system,
                exhaust_update_system,
                animate_player_system,
            ),
        )
        .add_systems(
            Update,
            (
                /* world & enemies ---------------------------------------- */
                stream_tiles_system
                    .run_if(resource_changed::<terrain::ActiveRect>),
                redraw_changed_tiles_system,
                enemy::update_active_tag_system,
                enemy::enemy_ai_system,
                enemy::enemy_attack_system,
                enemy::enemy_physics_system,
                enemy::animate_enemy_system,
                /* HUD & misc --------------------------------------------- */
                update_inventory_hud_system,
                health_regen_system,
                update_health_bar_system,
                toggle_fullscreen,
                detect_player_tile_change_system,
            ),
        )
        /* post‑update (camera / FOV) -------------------------------------- */
        .add_systems(
            PostUpdate,
            (
                camera_follow_system,
                update_active_rect_system,
                recompute_fov_system,
            ),
        )
        .run();
}