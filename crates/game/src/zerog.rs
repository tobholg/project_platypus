//! Weightless (the zerog arc): in the void over the sky, and wherever
//! gravity's all but gone, bodies float (`Locomotion::float`: holding on
//! to what they touch, pushing off it, drifting on), and what they do
//! pushes them: every shot, strike and blow kicks its maker the other way
//! (`drift::Recoil`; on a drifter, the drifter takes it: `drift`), and
//! from off its middle sets it turning (`Turning`: the whole of it drawn
//! turned, gear and all; righted once it holds on to something).

use bevy::prelude::*;

use crate::creatures::Kinematics;
use crate::drift::Recoil;
use crate::world::{SimWorld, TickSet};

pub struct ZeroGPlugin;

impl Plugin for ZeroGPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, kick.after(crate::drift::drift).before(crate::creatures::move_creatures).in_set(TickSet::Bodies))
            .add_systems(PostUpdate, turn.after(crate::creatures::interpolate).before(bevy::transform::TransformSystems::Propagate));
    }
}

/// Under this much gravity (a share of the ground's) a body floats.
pub const WEIGHTLESS: f32 = 1.0 / 6.0;

/// A pickaxe's blow (back off what it struck), as `drift`'s kicks.
pub const KICK_PICK: f32 = 3_000.0;
/// Struck (a sword, a fist), the striker's pushed back this share of the
/// blow's knock (no more than the knock itself, whatever it struck).
const BLOW_BACK: f32 = 0.5;
/// A kick from off its middle (the hand's out to one side) sets it
/// turning: this share of what it would as a stiff rod its height.
const TORQUE: f32 = 0.25;
/// Holding on to something (or with weight again), it rights itself this
/// fast (a share of the way a second).
const RIGHTING: f32 = 8.0;

/// Weightless and turning: how far it's turned from upright (radians,
/// counter-clockwise) and how fast it turns. Looks only: its box doesn't
/// turn.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct Turning {
    pub angle: f32,
    pub omega: f32,
}

/// How much gravity there is at `y` (1: the ground's … 0: none).
pub fn gravity_at(sim: &SimWorld, y: f32) -> f32 {
    sim.world.gravity(y)
}

/// Recoils and blows push their maker, weightless and holding on to
/// nothing (riding a drifter, it's the drifter's), the harder the lighter
/// they are, and (from off its middle) set it tumbling.
fn kick(mut commands: Commands, sim: Res<SimWorld>, mut recoils: MessageReader<Recoil>, mut hits: MessageReader<crate::combat::Hit>, riders: Res<crate::drift::Riders>, mut q: Query<(&mut Kinematics, Option<&mut Turning>)>) {
    let mut pushes: Vec<(Entity, Vec2, Vec2)> = recoils.read().map(|r| (r.who, r.from, r.push)).collect();
    for h in hits.read() {
        let Some(by) = h.by else { continue };
        let Ok([(striker, _), (struck, _)]) = q.get_many([by, h.target]) else { continue };
        let share = (crate::drift::weight(&struck.body) / crate::drift::weight(&striker.body)).min(1.0);
        pushes.push((by, h.at, -h.knock * share * BLOW_BACK * crate::drift::weight(&striker.body)));
    }
    for (who, from, push) in pushes {
        if riders.on.contains_key(&who) {
            continue;
        }
        let Ok((mut k, turning)) = q.get_mut(who) else { continue };
        let c = k.loco.contacts;
        let g = gravity_at(&sim, k.body.pos.y);
        if g >= WEIGHTLESS || k.body.pos.distance(from) > crate::drift::HAND || c.ground || c.ceiling || c.wall_left || c.wall_right {
            continue;
        }
        let m = crate::drift::weight(&k.body);
        let dv = push * (1.0 - g) / m;
        k.body.vel += dv;
        // (Turned about its middle by a push from its hand, as a rod its
        // height would be: ω = r × Δv / (h² / 3).)
        let h = k.body.half.y;
        let r = (from - k.body.pos).clamp_length_max(h);
        let spin = (r.x * dv.y - r.y * dv.x) / (h * h / 3.0) * TORQUE;
        match turning {
            Some(mut t) => t.omega += spin,
            None => {
                commands.entity(who).insert(Turning { angle: 0.0, omega: spin });
            }
        }
    }
}

/// Turning as it drifts, drawn turned (the whole of it); holding on to
/// something, or out of the void's weightlessness, righted again.
fn turn(mut commands: Commands, time: Res<Time>, sim: Res<SimWorld>, mut q: Query<(Entity, &Kinematics, &mut Turning, &mut Transform)>) {
    let dt = time.delta_secs();
    for (e, k, mut t, mut tf) in &mut q {
        let c = k.loco.contacts;
        let held = c.ground || c.ceiling || c.wall_left || c.wall_right || gravity_at(&sim, k.body.pos.y) >= WEIGHTLESS;
        if held {
            t.omega = 0.0;
            t.angle = crate::creatures::body::tumble::wrap(t.angle) * (-dt * RIGHTING).exp();
            if t.angle.abs() < 0.01 {
                tf.rotation = Quat::IDENTITY;
                commands.entity(e).remove::<Turning>();
                continue;
            }
        } else {
            t.angle = crate::creatures::body::tumble::wrap(t.angle + t.omega * dt);
        }
        tf.rotation = Quat::from_rotation_z(t.angle);
    }
}
