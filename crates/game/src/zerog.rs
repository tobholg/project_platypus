//! Weightless (the zerog arc): in the void over the sky, and wherever
//! gravity's all but gone, bodies float (`Locomotion::float`: holding on
//! to what they touch, pushing off it, drifting on), and what they do
//! pushes them: every shot, strike and blow kicks its maker the other way
//! (`drift::Recoil`; on a drifter, the drifter takes it: `drift`).

use bevy::prelude::*;

use crate::creatures::Kinematics;
use crate::drift::Recoil;
use crate::world::{SimWorld, TickSet};

pub struct ZeroGPlugin;

impl Plugin for ZeroGPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, kick.after(crate::drift::drift).before(crate::creatures::move_creatures).in_set(TickSet::Bodies));
    }
}

/// Under this much gravity (a share of the ground's) a body floats.
pub const WEIGHTLESS: f32 = 1.0 / 6.0;

/// A pickaxe's blow (back off what it struck), as `drift`'s kicks.
pub const KICK_PICK: f32 = 3_000.0;
/// Struck (a sword, a fist), the striker's pushed back this share of the
/// blow's knock (no more than the knock itself, whatever it struck).
const BLOW_BACK: f32 = 0.5;

/// How much gravity there is at `y` (1: the ground's … 0: none).
pub fn gravity_at(sim: &SimWorld, y: f32) -> f32 {
    sim.generator.sky_layers().map_or(1.0, |s| s.gravity(y))
}

/// Recoils and blows push their maker, weightless and holding on to
/// nothing (riding a drifter, it's the drifter's), the harder the lighter
/// they are.
fn kick(sim: Res<SimWorld>, mut recoils: MessageReader<Recoil>, mut hits: MessageReader<crate::combat::Hit>, riders: Res<crate::drift::Riders>, mut q: Query<&mut Kinematics>) {
    let mut pushes: Vec<(Entity, Vec2, Vec2)> = recoils.read().map(|r| (r.who, r.from, r.push)).collect();
    for h in hits.read() {
        let Some(by) = h.by else { continue };
        let Ok([striker, struck]) = q.get_many([by, h.target]) else { continue };
        let share = (crate::drift::weight(&struck.body) / crate::drift::weight(&striker.body)).min(1.0);
        pushes.push((by, h.at, -h.knock * share * BLOW_BACK * crate::drift::weight(&striker.body)));
    }
    for (who, from, push) in pushes {
        if riders.on.contains_key(&who) {
            continue;
        }
        let Ok(mut k) = q.get_mut(who) else { continue };
        let c = k.loco.contacts;
        let g = gravity_at(&sim, k.body.pos.y);
        if g >= WEIGHTLESS || k.body.pos.distance(from) > crate::drift::HAND || c.ground || c.ceiling || c.wall_left || c.wall_right {
            continue;
        }
        let m = crate::drift::weight(&k.body);
        k.body.vel += push * (1.0 - g) / m;
    }
}
