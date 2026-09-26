//! The player is a creature like any other (`creatures/player.ron`); its
//! brain is the keyboard. Co-op adds a network brain that writes the same
//! `Controls` from a remote peer's input.

use bevy::prelude::*;
use platypus_physics::Intent;
use serde::Deserialize;

use super::Controls;
use super::brain::RegisterBrain;
use crate::camera::{CursorWorld, FreeCamera};
use crate::world::TickSet;

pub struct PlayerPlugin;

/// The player controlled on this machine (camera follows, chunks load around it).
#[derive(Component)]
pub struct LocalPlayer;

/// Reads the local keyboard. A/D or ←/→ move, Space jump, Shift dash, E
/// the grappling hook.
/// (W stays free: it is the free-camera fly key and will be "up/look up".)
#[derive(Component, Deserialize, Default)]
pub struct KeyboardBrain;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.register_brain::<KeyboardBrain>("keyboard")
            .init_resource::<HeldKeys>()
            .add_systems(PreUpdate, sample_keys)
            .add_systems(FixedUpdate, keyboard_brain.in_set(TickSet::Intent));
    }
}

/// Keys are sampled every frame and consumed every tick, so a tap between two
/// ticks is never lost (frames and ticks run at different rates).
#[derive(Resource, Default)]
struct HeldKeys {
    intent: Intent,
    jump_tapped: bool,
    dash_tapped: bool,
    hook_tapped: bool,
}

fn sample_keys(keys: Res<ButtonInput<KeyCode>>, cursor: Res<CursorWorld>, taken: Res<crate::dev::KeyboardTaken>, mut held: ResMut<HeldKeys>) {
    if taken.0 {
        // (The art editor has the keyboard: stand still, keep looking.)
        held.intent = Intent { aim: held.intent.aim, ..default() };
        return;
    }
    let right = keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) as i32 as f32;
    let left = keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) as i32 as f32;
    held.intent.move_x = right - left;
    held.intent.jump = keys.pressed(KeyCode::Space);
    held.intent.dash = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    held.intent.hook = keys.pressed(KeyCode::KeyE);
    held.intent.down = keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    held.intent.move_y = keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) as i32 as f32 - held.intent.down as i32 as f32;
    held.jump_tapped |= keys.just_pressed(KeyCode::Space);
    held.dash_tapped |= keys.any_just_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    held.hook_tapped |= keys.just_pressed(KeyCode::KeyE);
    if let Some(c) = cursor.0 {
        held.intent.aim = c;
    }
}

fn keyboard_brain(mut held: ResMut<HeldKeys>, free: Res<FreeCamera>, mut q: Query<(&mut Controls, Option<&crate::combat::Stamina>), With<KeyboardBrain>>) {
    let mut intent = if free.0 { Intent { aim: held.intent.aim, ..default() } } else { held.intent };
    // A press released before this tick still counts as a press this tick.
    if !free.0 {
        intent.jump |= held.jump_tapped;
        intent.dash |= held.dash_tapped;
        intent.hook |= held.hook_tapped;
    }
    held.jump_tapped = false;
    held.dash_tapped = false;
    held.hook_tapped = false;
    for (mut c, stamina) in &mut q {
        c.0 = intent;
        // (A dash is a dodge: too tired, no dodge.)
        if stamina.is_some_and(|s| s.cur < 1.0) {
            c.0.dash = false;
        }
    }
}
