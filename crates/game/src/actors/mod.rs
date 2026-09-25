//! Creatures: the player, enemies, anything with a body and a brain.
//!
//! The pieces, and where to go to change each:
//! - what a creature *is*: a RON file in `assets/data/creatures/` (`creature.rs`)
//! - how it *decides*: a brain component + system, registered by name (`brain.rs`, `ai.rs`, `player.rs`)
//! - how it *moves*: shared by all, `platypus_physics::Locomotion` driven by `move_creatures` here
//! - how it *looks*: `animation.rs`, picking clips from movement state
//! - where it *appears*: `spawn.rs`

pub mod ai;
pub mod animation;
pub mod brain;
pub mod creature;
pub mod elements;
pub mod player;
pub mod spawn;

use bevy::prelude::*;
use platypus_physics::{Body, Grid, Intent, Locomotion, MovementStats, Occupancy, move_and_collide};
use platypus_sim::{CellPos, Kind, World};
use serde::Deserialize;

use crate::world::{SimWorld, TICK_HZ, TickSet};

pub struct ActorsPlugin;

impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Landed>()
            .add_plugins((creature::CreaturePlugin, brain::BrainPlugin, spawn::SpawnPlugin, animation::AnimationPlugin))
            .add_plugins((player::PlayerPlugin, ai::AiPlugin))
            .add_systems(FixedUpdate, (move_creatures, fall_damage, elements::expose, deaths).chain().in_set(TickSet::Bodies))
            .add_systems(Update, elements::tint)
            .add_systems(PostUpdate, interpolate.before(TransformSystems::Propagate));
    }
}

/// Instance of a creature definition (file stem in `assets/data/creatures/`).
#[derive(Component, Clone, Debug)]
pub struct Creature {
    pub kind: String,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Team {
    Player,
    Enemy,
    Neutral,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Health {
    pub hp: f32,
    pub max: f32,
}

/// Physical state of a creature. `prev_pos` is for render interpolation.
#[derive(Component, Clone, Debug)]
pub struct Kinematics {
    pub body: Body,
    pub loco: Locomotion,
    pub prev_pos: Vec2,
}

#[derive(Component, Clone, Debug)]
pub struct MoveStats(pub MovementStats);

/// What the brain wants this tick. Brains write it in `TickSet::Intent`.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Controls(pub Intent);

/// Fall damage: landing faster than `safe_speed` hurts `per_speed` per cell/s over.
#[derive(Component, Clone, Copy, Debug, Deserialize)]
pub struct FallDamage {
    pub safe_speed: f32,
    pub per_speed: f32,
}

#[derive(Message, Clone, Copy, Debug)]
pub struct Landed {
    pub entity: Entity,
    pub speed: f32,
}

/// The cell world as bodies see it. Unloaded chunks are solid.
pub struct WorldGrid<'a>(pub &'a World);

impl Grid for WorldGrid<'_> {
    #[inline]
    fn occupancy(&self, x: i32, y: i32) -> Occupancy {
        match self.0.get(CellPos::new(x, y)) {
            None => Occupancy::Solid,
            Some(c) => match self.0.materials().phys(c.material).kind {
                Kind::Static | Kind::Powder => Occupancy::Solid,
                Kind::Liquid => Occupancy::Liquid,
                Kind::Empty | Kind::Gas | Kind::Fire | Kind::Plant => Occupancy::Empty,
            },
        }
    }
}

const DT: f32 = (1.0 / TICK_HZ) as f32;

/// One movement code path for every creature.
fn move_creatures(
    sim: Res<SimWorld>,
    mut q: Query<(Entity, &mut Kinematics, &MoveStats, &Controls, Option<&elements::Chilled>)>,
    mut landed: MessageWriter<Landed>,
) {
    let grid = WorldGrid(&sim.world);
    for (entity, mut k, stats, controls, chilled) in &mut q {
        // Frozen until the ground under it is loaded.
        if !sim.world.is_loaded(CellPos::from_world(k.body.pos.x, k.body.pos.y).chunk()) {
            continue;
        }
        let k = &mut *k;
        k.prev_pos = k.body.pos;
        let slowed;
        let stats = match chilled {
            Some(c) => {
                slowed = stats.0.slowed(c.speed());
                &slowed
            }
            None => &stats.0,
        };
        k.loco.steer(stats, &controls.0, &mut k.body, DT);
        let contacts = move_and_collide(&grid, &mut k.body, DT);
        if let Some(speed) = k.loco.after_move(contacts) {
            landed.write(Landed { entity, speed });
        }
    }
}

fn fall_damage(mut landed: MessageReader<Landed>, mut q: Query<(&FallDamage, &mut Health)>) {
    for l in landed.read() {
        if let Ok((f, mut h)) = q.get_mut(l.entity)
            && l.speed > f.safe_speed
        {
            h.hp -= (l.speed - f.safe_speed) * f.per_speed;
        }
    }
}

/// Dead creatures burst into blood particles that land as real cells (they
/// run and pool). The player respawns instead.
fn deaths(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    mut q: Query<(Entity, &mut Health, &mut Kinematics, Has<player::LocalPlayer>)>,
) {
    let spawn = sim.generator.spawn_point();
    for (entity, mut h, mut k, is_player) in &mut q {
        if h.hp > 0.0 {
            continue;
        }
        if let Some(blood) = sim.materials().id("blood") {
            // A burst of real blood cells: they fly, land, run and pool.
            sim.world.splash([k.body.pos.x, k.body.pos.y], blood, 70, 2.2);
        }
        if is_player {
            commands.entity(entity).remove::<(elements::Burning, elements::Wet, elements::Chilled)>();
            h.hp = h.max;
            k.body.pos = Vec2::new(spawn.x as f32, spawn.y as f32 + 60.0);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
        } else {
            commands.entity(entity).despawn();
        }
    }
}

/// Render between the last two ticks so 120 Hz displays stay smooth.
fn interpolate(time: Res<Time<Fixed>>, mut q: Query<(&Kinematics, &mut Transform)>) {
    let a = time.overstep_fraction();
    for (k, mut tf) in &mut q {
        let p = k.prev_pos.lerp(k.body.pos, a);
        tf.translation.x = p.x;
        tf.translation.y = p.y;
    }
}
