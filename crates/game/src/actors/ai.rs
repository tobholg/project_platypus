//! Built-in AI brains. Each is a component (its tunables, from the creature
//! file) plus one system writing `Controls`. Copy one to make a new behaviour.

use bevy::prelude::*;
use platypus_sim::rng::Rng;
use serde::Deserialize;

use super::brain::RegisterBrain;
use super::{Controls, Kinematics, Team};
use crate::world::{SimWorld, TickSet};

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.register_brain::<MeleeWalker>("melee_walker")
            .register_brain::<Idle>("idle")
            .add_systems(FixedUpdate, melee_walker.in_set(TickSet::Intent));
    }
}

/// Stands still. Useful for training dummies and for testing new art.
#[derive(Component, Deserialize, Default)]
pub struct Idle;

/// Walks toward the nearest player, jumps over obstacles and up to ledges,
/// wanders when nobody is near. (Attacks arrive with the combat phase.)
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct MeleeWalker {
    /// Notices players within this distance (cells).
    pub aggro_range: f32,
    /// Stops approaching at this horizontal distance.
    pub keep_distance: f32,
    /// Fraction of run speed used while wandering.
    pub wander_speed: f32,
    /// Seconds between wander decisions (randomised ±50%).
    pub wander_every: f32,
    /// Jump when a player is this much higher (cells) and close horizontally.
    pub jump_to_reach: f32,
}

impl Default for MeleeWalker {
    fn default() -> Self {
        MeleeWalker { aggro_range: 220.0, keep_distance: 10.0, wander_speed: 0.4, wander_every: 2.5, jump_to_reach: 18.0 }
    }
}

/// Per-creature memory for simple brains.
#[derive(Component, Default)]
pub struct WanderState {
    until_tick: u64,
    dir: f32,
}

fn melee_walker(
    mut commands: Commands,
    sim: Res<SimWorld>,
    players: Query<(&Kinematics, &Team), Without<MeleeWalker>>,
    mut q: Query<(Entity, &MeleeWalker, &Kinematics, &mut Controls, Option<&mut WanderState>)>,
) {
    let tick = sim.world.tick();
    for (entity, brain, k, mut controls, wander) in &mut q {
        let pos = k.body.pos;
        let target = players
            .iter()
            .filter(|(_, t)| **t == Team::Player)
            .map(|(pk, _)| pk.body.pos)
            .filter(|p| p.distance(pos) < brain.aggro_range)
            .min_by(|a, b| a.distance_squared(pos).total_cmp(&b.distance_squared(pos)));

        let mut want_jump = false;
        let c = &k.loco.contacts;
        let move_x = match target {
            Some(t) => {
                let d = t - pos;
                want_jump = k.loco.grounded() && d.y > brain.jump_to_reach && d.x.abs() < brain.aggro_range * 0.25;
                if d.x.abs() > brain.keep_distance { d.x.signum() } else { 0.0 }
            }
            None => {
                let Some(mut w) = wander else {
                    commands.entity(entity).insert(WanderState::default());
                    continue;
                };
                if tick >= w.until_tick {
                    let mut rng = Rng::seeded(&[sim.world.seed(), tick, entity.to_bits()]);
                    w.dir = [-1.0, 0.0, 1.0][(rng.next_u32() % 3) as usize];
                    let secs = brain.wander_every * (0.5 + (rng.next_u32() % 1000) as f32 / 1000.0);
                    w.until_tick = tick + (secs * 60.0) as u64;
                }
                w.dir * brain.wander_speed
            }
        };
        // Blocked by a wall we're walking into: jump it.
        if (move_x > 0.0 && c.wall_right) || (move_x < 0.0 && c.wall_left) {
            want_jump = k.loco.grounded();
        }
        controls.0.move_x = move_x;
        // Brains hold buttons; release after a press so the next press registers.
        controls.0.jump = want_jump && !controls.0.jump;
    }
}
