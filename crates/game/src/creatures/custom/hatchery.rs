//! Egg sacs: still until a player comes near (or something hurts it); then
//! it bursts, and its brood spills out (its file's `blood`: what it's full
//! of).

use bevy::prelude::*;
use platypus_sim::rng::Rng;
use serde::Deserialize;

use super::{CustomCreature, CustomSet};
use crate::creatures::def::spawn_creature;
use crate::creatures::{Health, Kinematics, Team};
use crate::world::SimWorld;

/// What hatches, how many, and how near a player sets it off (cells).
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Hatchery {
    pub brood: String,
    pub count: u32,
    pub range: f32,
}

impl Default for Hatchery {
    fn default() -> Self {
        Hatchery { brood: "spiderling".into(), count: 3, range: 60.0 }
    }
}

impl CustomCreature for Hatchery {
    const NAME: &'static str = "hatchery";

    fn build(app: &mut App) {
        app.add_systems(FixedUpdate, hatch.in_set(CustomSet::Think));
    }
}

/// Hurt, or someone it hunts near: its brood out, and it bursts.
fn hatch(mut commands: Commands, sim: Res<SimWorld>, hunted: Query<(&Kinematics, &Team), Without<crate::creatures::brain::villager::Hiding>>, mut q: Query<(Entity, &Hatchery, &Kinematics, &mut Health)>) {
    for (e, h, k, mut hp) in &mut q {
        let near = hunted.iter().any(|(pk, t)| t.hunted() && pk.body.pos.distance(k.body.pos) < h.range);
        if !near && hp.hp >= hp.max {
            continue;
        }
        let mut rng = Rng::seeded(&[sim.world.seed(), sim.world.tick(), e.to_bits(), 0xE66]);
        for _ in 0..h.count {
            let dx = (rng.next_u32() as f32 / u32::MAX as f32 - 0.5) * 12.0;
            spawn_creature(&mut commands, &h.brood, k.body.pos + Vec2::new(dx, 1.5), |_| {});
        }
        hp.hp = 0.0;
    }
}
