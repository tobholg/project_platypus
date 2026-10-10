//! Thrown about: a creature knocked off its feet spins as it flies
//! (`Tumble`), set going by the blow (`spin_of`: harder blows, faster; a
//! heavy body less), and rights itself once it's down. A body that dies
//! keeps its spin (`hands::corpses`). Looks only: the body's box doesn't
//! turn.

use bevy::prelude::*;

use crate::creatures::Kinematics;

/// How far it's turned from upright (radians, counter-clockwise), and how
/// fast it's turning.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct Tumble {
    pub angle: f32,
    pub omega: f32,
}

/// Spin per cell/s of knock (radians/s), and the most.
const SPIN_PER_KNOCK: f32 = 0.022;
const SPIN_MOST: f32 = 22.0;
/// Down again, it rights itself this fast (per second).
const RIGHTING: f32 = 14.0;

/// The spin a knock gives a body of `heft`: over backwards, away from the
/// blow.
pub fn spin_of(knock: Vec2, heft: f32) -> f32 {
    (-knock.x.signum() * knock.length() * SPIN_PER_KNOCK / heft.max(1.0).sqrt()).clamp(-SPIN_MOST, SPIN_MOST)
}

/// Between -π and π.
pub fn wrap(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    (a + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI
}

/// Spinning in the air; down, upright again.
pub fn spin(time: Res<Time>, mut q: Query<(&Kinematics, &mut Tumble)>) {
    let dt = time.delta_secs();
    for (k, mut t) in &mut q {
        // (Down and not rising: the tick it's knocked it's still on its
        // feet, going up.)
        let down = k.loco.grounded() && k.body.vel.y <= 0.0;
        if !down && t.omega != 0.0 {
            t.angle = wrap(t.angle + t.omega * dt);
            t.omega *= (1.0 - 0.4 * dt).max(0.0);
        } else if t.angle != 0.0 || t.omega != 0.0 {
            t.omega = 0.0;
            t.angle = wrap(t.angle) * (-dt * RIGHTING).exp();
            if t.angle.abs() < 0.01 {
                t.angle = 0.0;
            }
        }
    }
}
