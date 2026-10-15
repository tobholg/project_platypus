//! Dying, playing (`mode.rs`): the screen goes dark, "You died", and you
//! wake at your checkpoint (where the world began, until something sets
//! another: a bed, the story) with your health back. In dev mode you get
//! your health back where you stand (`PLATYPUS_RESPAWN=1`: at the
//! checkpoint too). The checkpoint is kept in the save.

use bevy::prelude::*;

use crate::creatures::spawn::find_ground;
use crate::world::SimWorld;

pub struct RespawnPlugin;

/// Where the player wakes after dying (feet; `None`: where the world began).
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct Checkpoint(pub Option<Vec2>);

/// The player died (and has been put back).
#[derive(Message, Clone, Copy, Debug)]
pub struct Respawned;

/// The dark and the words, while they last (seconds left).
#[derive(Resource, Default)]
struct Dark(f32);

#[derive(Component)]
struct Veil;

#[derive(Component)]
struct Words;

/// How long the screen stays dark after dying, and how long it takes to clear.
const DARK: f32 = 2.6;
const CLEAR: f32 = 1.2;

impl Plugin for RespawnPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Checkpoint>()
            .init_resource::<Dark>()
            .add_message::<Respawned>()
            .add_systems(Startup, spawn_veil)
            .add_systems(Update, veil);
    }
}

/// Where a body this tall stands at the checkpoint (its centre): on the
/// ground there, or, not loaded yet, the ground as generated (the body
/// climbs out of anything it's in once it loads).
pub fn respawn_at(sim: &SimWorld, checkpoint: &Checkpoint, half: Vec2) -> Vec2 {
    if let Some(feet) = checkpoint.0 {
        return feet + Vec2::Y * (half.y + 0.5);
    }
    let s = sim.generator.spawn_point();
    let ground = find_ground(&sim.world, s.x, s.y + 180, 900).or_else(|| sim.generator.surface_hint(s.x).map(|y| y + 1)).unwrap_or(s.y);
    Vec2::new(s.x as f32 + 0.5, ground as f32 + half.y)
}

fn spawn_veil(mut commands: Commands) {
    commands
        .spawn((
            Veil,
            Visibility::Hidden,
            GlobalZIndex(50),
            Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|p| {
            p.spawn((Words, Text::new("You died"), TextFont { font_size: FontSize::Px(54.0), ..default() }, TextColor(Color::NONE)));
        });
}

fn veil(time: Res<Time<Real>>, mut died: MessageReader<Respawned>, mut dark: ResMut<Dark>, mut veil: Query<(&mut Visibility, &mut BackgroundColor), With<Veil>>, mut words: Query<&mut TextColor, With<Words>>) {
    if died.read().count() > 0 {
        dark.0 = DARK + CLEAR;
    }
    if dark.0 <= 0.0 {
        return;
    }
    dark.0 = (dark.0 - time.delta_secs()).max(0.0);
    // (Dark at once, then clearing over the last CLEAR seconds.)
    let a = (dark.0 / CLEAR).min(1.0);
    for (mut v, mut bg) in &mut veil {
        *v = if dark.0 > 0.0 { Visibility::Visible } else { Visibility::Hidden };
        bg.0 = Color::srgba(0.02, 0.0, 0.0, 0.85 * a);
    }
    for mut c in &mut words {
        c.0 = Color::srgba(0.75, 0.1, 0.08, a);
    }
}
