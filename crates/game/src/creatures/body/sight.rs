//! Eyes in the dark, only where the player could see them: a creature's
//! glowing eyes are drawn over the darkness (they'd show through rock
//! otherwise, a cave's every spider seen through the hill), so each
//! creature keeps how much the player sees it (`Sighted`), looked at a few
//! times a second along a line from the player's eye to its eyes and its
//! middle, and its eyes are drawn that bright. Seen, they come up quickly;
//! lost behind rock, they linger a moment and fade (a glimpse round a
//! corner). With the lighting off they're always seen.

use bevy::prelude::*;

use crate::creatures::player::LocalPlayer;
use crate::creatures::{Creature, Dormant, Kinematics};
use crate::world::SimWorld;

pub struct SightPlugin;

impl Plugin for SightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (sight, eyes).chain().before(bevy::transform::TransformSystems::Propagate));
    }
}

/// How much the player sees it (0 to 1, eased), and what that's heading for.
#[derive(Component, Default)]
pub struct Sighted {
    pub seen: f32,
    target: f32,
}

/// Each creature looked at once every this many frames (spread over them).
const EVERY: u32 = 6;
/// Further than this from the player, it isn't seen at all (cells).
const FAR: f32 = 900.0;
/// Seconds to come up seen, and to fade once lost.
const RISE: f32 = 0.15;
const FADE: f32 = 0.7;

#[allow(clippy::type_complexity)]
fn sight(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<SimWorld>,
    toggles: Res<crate::light::LightToggles>,
    mut frame: Local<u32>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut q: Query<(Entity, &Kinematics, Option<&mut Sighted>), (With<Creature>, Without<LocalPlayer>, Without<Dormant>)>,
) {
    *frame = frame.wrapping_add(1);
    let Ok(pk) = player.single() else { return };
    let eye = pk.body.pos + Vec2::Y * pk.body.half.y * 0.4;
    let dt = time.delta_secs();
    for (e, k, sighted) in &mut q {
        let Some(mut s) = sighted else {
            commands.entity(e).insert(Sighted::default());
            continue;
        };
        if !toggles.enabled {
            s.target = 1.0;
        } else if (e.index_u32().wrapping_add(*frame)).is_multiple_of(EVERY) {
            let p = k.body.pos;
            let its_eyes = p + Vec2::Y * k.body.half.y * 0.5;
            let near = p.distance(eye) < FAR;
            s.target = if near && (crate::creatures::moves::clear(&sim, eye, its_eyes) || crate::creatures::moves::clear(&sim, eye, p)) { 1.0 } else { 0.0 };
        }
        let rate = if s.target > s.seen { dt / RISE } else { dt / FADE };
        s.seen = if s.target > s.seen { (s.seen + rate).min(s.target) } else { (s.seen - rate).max(s.target) };
    }
}

/// Its eyes drawn as bright as it's seen.
#[allow(clippy::type_complexity)]
fn eyes(creatures: Query<(&Sighted, &Children)>, mut sprites: Query<&mut Sprite, Or<(With<super::animation::CreatureEyes>, With<super::legs::LegEyes>)>>) {
    for (s, children) in &creatures {
        for c in children.iter() {
            if let Ok(mut sprite) = sprites.get_mut(c) {
                sprite.color.set_alpha(s.seen);
            }
        }
    }
}
