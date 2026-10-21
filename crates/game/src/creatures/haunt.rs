//! Hauntings: what a ghost (`CreatureDef::haunt`: the chain wraith, the
//! frost wraith) does to the world round it, whatever else it does.
//!
//! - **Cold**: the air round it chills (`WorldEdit::Heat`, below zero, a
//!   few times a second): water near it freezes over, and stays frozen a
//!   while after it's gone.
//! - **Flames gutter**: torches, lanterns, candles, the player's own glow
//!   shrink toward it, guttering, almost going out, as if a draught off
//!   the grave caught them (`Haunts::flame`).
//! - **Lamps fail**: a flashlight's beam, a mining helmet's lamp stutter,
//!   brown out, black out for a moment and come back (`Haunts::electric`).
//!
//! Both fade with distance (`HauntDef::reach`): the dark closes in before
//! you see what's coming. The light pass reads `Haunts` (`light::compute_light`).

use bevy::prelude::*;
use serde::Deserialize;

use super::Kinematics;
use super::body::animation::Animator;
use crate::world::SimWorld;

/// A ghost's haunting (`CreatureDef::haunt`).
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default)]
pub struct HauntDef {
    /// How far its lights' failing reaches (cells).
    pub reach: f32,
    /// How far its cold reaches (cells), and how cold (heat a chill, below
    /// zero: `WorldEdit::Heat`).
    pub chill: (i32, i16),
}

impl Default for HauntDef {
    fn default() -> Self {
        HauntDef { reach: 260.0, chill: (48, -30) }
    }
}

/// Where the ghosts are this frame, and how far each haunts.
#[derive(Resource, Default, Clone)]
pub struct Haunts {
    pub at: Vec<(Vec2, f32)>,
}

/// How often a ghost chills the air round it (s).
const CHILL_EVERY: f32 = 0.25;

impl Haunts {
    /// How haunted a point is: 1 at a ghost, nothing past its reach (eased:
    /// it comes on gently, then hard).
    pub fn at(&self, p: Vec2) -> f32 {
        self.at.iter().map(|&(g, r)| (1.0 - g.distance(p) / r).clamp(0.0, 1.0)).fold(0.0, f32::max).powf(1.6)
    }

    /// A flame's light near a ghost (times its own): shrunk, and guttering
    /// (dipping hard, near out, a moment, a few times a second near it,
    /// then swelling back); 1 where nothing haunts.
    pub fn flame(&self, p: Vec2, t: f32, salt: u64) -> f32 {
        let h = self.at(p);
        if h <= 0.0 {
            return 1.0;
        }
        let s = salt as f32 * 0.73;
        // (A slow sway and quick gutters, out of step light to light.)
        let sway = 0.5 + 0.5 * (t * 1.7 + s).sin() * (t * 0.61 + s * 1.9).sin();
        let gutter = (0.5 + 0.5 * (t * 6.3 + s * 3.1).sin() + 0.35 * (t * 13.7 + s).sin()).clamp(0.0, 1.0);
        let dip = (gutter * gutter * gutter) * h;
        (1.0 - h * (0.45 + 0.25 * sway) - dip * 0.35).clamp(0.06, 1.0)
    }

    /// An electric lamp near a ghost (times its own): mostly on, then
    /// faults: a stutter, a brownout, a blackout a moment, coming back with
    /// a flicker; more and longer near it; 1 where nothing haunts.
    pub fn electric(&self, p: Vec2, t: f32, salt: u64) -> f32 {
        let h = self.at(p);
        if h <= 0.0 {
            return 1.0;
        }
        let roll = |a: u64, b: u64| (platypus_sim::rng::hash(&[salt, a, b]) % 10_000) as f32 / 10_000.0;
        // A fault now and then (a stretch of a second or so), likelier near
        // it; in a fault, the lamp stutters step by step (~16 a second).
        let stretch = (t / 0.9) as u64;
        let faulting = roll(1, stretch) < 0.15 + 0.75 * h;
        if !faulting {
            // (A faint unsteadiness even between faults.)
            return 1.0 - 0.08 * h * roll(2, (t * 30.0) as u64);
        }
        let step = (t * 16.0) as u64;
        let r = roll(3, step);
        if r < 0.35 + 0.4 * h {
            // Out: a blackout, or nearly.
            0.03 * roll(4, step)
        } else if r < 0.75 {
            // Browned out.
            0.25 + 0.3 * roll(5, step)
        } else {
            1.0
        }
    }
}

/// Where the ghosts are, for the light; and their cold on the world.
pub fn haunt(time: Res<Time>, mut haunts: ResMut<Haunts>, mut sim: ResMut<SimWorld>, mut last: Local<f32>, ghosts: Query<(&Animator, &Kinematics), Without<super::Dormant>>) {
    haunts.at.clear();
    let now = time.elapsed_secs();
    let chill = now - *last >= CHILL_EVERY;
    if chill {
        *last = now;
    }
    for (anim, k) in &ghosts {
        let Some(h) = anim.def.haunt else { continue };
        haunts.at.push((k.body.pos, h.reach));
        if chill && h.chill.0 > 0 {
            sim.queue(platypus_sim::WorldEdit::Heat { center: platypus_sim::CellPos::from_world(k.body.pos.x, k.body.pos.y), radius: h.chill.0, amount: h.chill.1 });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lights_fail_near_a_ghost_and_not_far_off() {
        let h = Haunts { at: vec![(Vec2::ZERO, 200.0)] };
        let far = Vec2::new(500.0, 0.0);
        assert_eq!(h.flame(far, 1.0, 1), 1.0);
        assert_eq!(h.electric(far, 1.0, 1), 1.0);
        // Near: flames shrink on average; lamps black out some of the time.
        let near = Vec2::new(20.0, 0.0);
        let flame: f32 = (0..500).map(|i| h.flame(near, i as f32 * 0.013, 3)).sum::<f32>() / 500.0;
        assert!(flame < 0.6, "a flame near a ghost averages {flame}");
        let out = (0..2000).filter(|&i| h.electric(near, i as f32 * 0.011, 3) < 0.1).count();
        assert!((200..1600).contains(&out), "a lamp near a ghost out {out} of 2000 moments");
    }
}
