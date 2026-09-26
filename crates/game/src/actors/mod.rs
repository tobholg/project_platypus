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
pub mod critters;
pub mod dummy;
pub mod elements;
pub mod hurt;
pub mod player;
pub mod spawn;

use bevy::prelude::*;
use platypus_physics::{Body, Grid, Intent, Locomotion, MovementStats, Occupancy, move_and_collide};
use platypus_sim::{CellPos, Kind, World, WorldEdit};
use serde::Deserialize;

use brain::RegisterBrain;

use crate::world::{SimWorld, TICK_HZ, TickSet};

pub struct ActorsPlugin;

impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.register_brain::<dummy::Dummy>("dummy")
            .add_message::<Landed>()
            .add_message::<AirJumped>()
            .add_plugins((creature::CreaturePlugin, brain::BrainPlugin, spawn::SpawnPlugin, animation::AnimationPlugin))
            .add_plugins((player::PlayerPlugin, ai::AiPlugin, critters::CrittersPlugin))
            .add_systems(FixedUpdate, (move_creatures, fall_damage, elements::expose, crate::combat::guard, hurt::notice, dummy::tally, deaths).chain().in_set(TickSet::Bodies))
            .insert_resource(elements::Coatings::load())
            .init_resource::<PlayerDeaths>()
            .add_systems(FixedUpdate, displace_liquid.after(move_creatures).in_set(TickSet::Bodies))
            .add_systems(Update, (elements::tint, elements::reload_coatings, hurt::watch, hurt::float, dummy::show))
            .add_systems(FixedUpdate, (elements::struck, elements::zapped, blasted, pelted).after(TickSet::Cells))
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

/// Fall damage, by how far it fell (Terraria-style; speed saturates at
/// max fall within ~50 cells, so it can't tell a double jump from a cliff):
/// falling further than `safe_height` cells, from the highest point since
/// it left the ground, hurts `per_cell` a cell over; slamming into a wall
/// or ceiling faster than `slam_speed` (flung by a spell, a blast) hurts
/// `per_speed` per cell/s over.
#[derive(Component, Clone, Copy, Debug, Deserialize)]
pub struct FallDamage {
    pub safe_height: f32,
    pub per_cell: f32,
    #[serde(default = "slam_speed")]
    pub slam_speed: f32,
    #[serde(default = "slam_per")]
    pub per_speed: f32,
}

fn slam_speed() -> f32 {
    450.0
}

fn slam_per() -> f32 {
    0.25
}

/// The highest a body has been since it last stood on something (for fall
/// damage).
#[derive(Component, Default)]
pub struct FallTrack {
    top: Option<f32>,
}

/// Hitting a wall or ceiling slower than this isn't worth reporting (walking
/// into a wall).
const SLAM_MIN: f32 = 150.0;

/// A body jumped off thin air (a double jump) with its feet at `at`: a puff
/// of cloud there (`vfx`), a soft flash of light (`light`).
#[derive(Message, Clone, Copy, Debug)]
pub struct AirJumped {
    pub at: Vec2,
}

/// A body landed after falling `drop` cells, or slammed into a wall or
/// ceiling at `slam` cells/s: fall damage reads it.
#[derive(Message, Clone, Copy, Debug)]
pub struct Landed {
    pub entity: Entity,
    pub drop: f32,
    pub slam: f32,
}

/// The cell world as bodies see it. Unloaded chunks are solid.
pub struct WorldGrid<'a>(pub &'a World);

impl Grid for WorldGrid<'_> {
    #[inline]
    fn occupancy(&self, x: i32, y: i32) -> Occupancy {
        match self.0.get(CellPos::new(x, y)) {
            None => Occupancy::Solid,
            Some(c) => match self.0.materials().phys(c.material) {
                p if p.platform => Occupancy::Platform,
                p => match p.kind {
                    Kind::Static | Kind::Powder => Occupancy::Solid,
                Kind::Liquid => Occupancy::Liquid,
                    Kind::Empty | Kind::Gas | Kind::Fire | Kind::Plant => Occupancy::Empty,
                },
            },
        }
    }
}

const DT: f32 = (1.0 / TICK_HZ) as f32;

/// One movement code path for every creature.
type Movers<'a> = (Entity, &'a mut Kinematics, &'a MoveStats, &'a Controls, Option<&'a elements::Chilled>, Option<&'a mut FallTrack>);

fn move_creatures(
    sim: Res<SimWorld>,
    mut q: Query<Movers>,
    mut landed: MessageWriter<Landed>,
    mut air: MessageWriter<AirJumped>,
    mut dashed: MessageWriter<crate::combat::Dashed>,
) {
    let grid = WorldGrid(&sim.world);
    for (entity, mut k, stats, controls, chilled, track) in &mut q {
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
        let ev = k.loco.steer(stats, &controls.0, &mut k.body, DT);
        if ev.dashed {
            dashed.write(crate::combat::Dashed(entity));
        }
        if ev.air_jumped {
            air.write(AirJumped { at: k.body.pos - Vec2::new(0.0, k.body.half.y) });
        }
        // A jump off air or a wall starts the fall over (a double jump just
        // before landing saves you, as in Terraria).
        let rejumped = ev.air_jumped || ev.wall_jumped;
        let before = k.body.vel;
        let contacts = move_and_collide(&grid, &mut k.body, DT);
        // Slammed into a wall or a ceiling (flung by a spell, a blast): an
        // impact like a landing, by the speed it hit at.
        let walled = if (contacts.wall_left && before.x < 0.0) || (contacts.wall_right && before.x > 0.0) { before.x.abs() } else { 0.0 };
        let roofed = if contacts.ceiling && before.y > 0.0 { before.y } else { 0.0 };
        let slam = walled.max(roofed);
        let y = k.body.pos.y;
        let drop = track.map_or(0.0, |mut t| {
            if rejumped {
                t.top = None;
            }
            let top = t.top.unwrap_or(y).max(y);
            // (Standing on something, or in water: the fall starts over.)
            t.top = if contacts.ground || contacts.submerged > 0.5 { None } else { Some(top) };
            top - y
        });
        if k.loco.after_move(contacts).is_some() {
            landed.write(Landed { entity, drop, slam });
        } else if slam > SLAM_MIN {
            landed.write(Landed { entity, drop: 0.0, slam });
        }
    }
}

/// Bodies push liquid aside: whatever flowed into a creature's box moves up
/// its column (the level rises around it), splashing if it arrived fast.
fn displace_liquid(mut sim: ResMut<SimWorld>, q: Query<&Kinematics>) {
    for k in &q {
        let (lo, hi) = k.body.cells_at(k.body.pos);
        let grid = WorldGrid(&sim.world);
        let wet = (lo.y..=hi.y).any(|y| (lo.x..=hi.x).any(|x| grid.occupancy(x, y) == Occupancy::Liquid));
        if !wet {
            continue;
        }
        // Cells per tick, in sixteenths (edits hold integers).
        let v = k.body.vel / TICK_HZ as f32 * 16.0;
        sim.queue(WorldEdit::Displace {
            min: CellPos::new(lo.x, lo.y),
            max: CellPos::new(hi.x, hi.y),
            vel: [v.x.round() as i16, v.y.round() as i16],
        });
    }
}

fn fall_damage(mut landed: MessageReader<Landed>, mut q: Query<(&FallDamage, &mut Health), Without<crate::magic::well::Carried>>) {
    for l in landed.read() {
        if let Ok((f, mut h)) = q.get_mut(l.entity) {
            let fell = (l.drop - f.safe_height).max(0.0) * f.per_cell;
            let slammed = (l.slam - f.slam_speed).max(0.0) * f.per_speed;
            h.hp -= fell.max(slammed);
        }
    }
}

/// How many times the player has died (the HUD shows it).
#[derive(Resource, Default)]
pub struct PlayerDeaths(pub u32);

/// Dead creatures burst into blood particles that land as real cells (they
/// run and pool). The player, while developing, just gets its health back
/// where it stands; with `PLATYPUS_RESPAWN=1` it respawns at the start.
/// Particles of solid, powder or liquid (not rain, dust or embers) faster
/// than this (cells/s) hurt what they fly through...
const PELT_SAFE: f32 = 90.0;
/// ... by their weight (density against water's; liquids half) × how many
/// times faster × this, and are mostly stopped by it, shoving it.
const PELT: f32 = 0.8;

/// Heavy things flying fast hurt what they hit: a ball of rock dropped
/// from a well, blast debris, a flung stream of sand.
fn pelted(mut sim: ResMut<SimWorld>, mut q: Query<(&mut Kinematics, &mut Health)>) {
    let boxes: Vec<(Vec2, Vec2)> = q.iter().map(|(k, _)| (k.body.pos - k.body.half, k.body.pos + k.body.half)).collect();
    if boxes.is_empty() {
        return;
    }
    let mats = sim.world.materials().clone();
    let mut hurt = vec![0.0f32; boxes.len()];
    let mut shove = vec![Vec2::ZERO; boxes.len()];
    for p in sim.world.particles_mut() {
        if p.landing != platypus_sim::Landing::Settle {
            continue;
        }
        let v = Vec2::new(p.vel[0], p.vel[1]) * TICK_HZ as f32;
        let speed = v.length();
        if speed < PELT_SAFE {
            continue;
        }
        let at = Vec2::new(p.pos[0], p.pos[1]);
        let Some(i) = boxes.iter().position(|(lo, hi)| at.cmpge(*lo).all() && at.cmple(*hi).all()) else { continue };
        let ph = mats.phys(p.cell.material);
        let weight = (ph.density as f32 / 1000.0).clamp(0.2, 5.0) * if ph.kind == Kind::Liquid { 0.5 } else { 1.0 };
        hurt[i] += weight * (speed - PELT_SAFE) / PELT_SAFE * PELT;
        shove[i] += v * weight * 0.02;
        p.vel = [p.vel[0] * 0.3, p.vel[1] * 0.3];
    }
    for (i, (mut k, mut h)) in q.iter_mut().enumerate() {
        if hurt[i] > 0.0 {
            h.hp -= hurt[i];
            k.body.vel += shove[i];
        }
    }
}

/// Every explosion (a bomb, a fireball, a gas pocket, lightning's burst)
/// hurts and throws the creatures near it, less the farther they are: out
/// to 1.6 times its radius, up to 0.65 × its power in damage and 3 × in
/// knockback (a bomb: 90 and 420).
fn blasted(mut blasts: MessageReader<crate::fx::Explosion>, mut q: Query<(&mut Kinematics, &mut Health)>) {
    for b in blasts.read() {
        let reach = b.radius * 1.6;
        for (mut k, mut health) in &mut q {
            let d = k.body.pos - b.at;
            let dist = d.length();
            if dist > reach {
                continue;
            }
            let f = 1.0 - dist / reach;
            health.hp -= b.power * 0.65 * f;
            let dir = (d.normalize_or(Vec2::Y) + Vec2::new(0.0, 0.6)).normalize();
            let k = &mut *k;
            k.loco.knock(&mut k.body, dir * b.power * 3.0 * (0.4 + 0.6 * f), 0.35);
        }
    }
}

type Mortal<'a> = (Entity, &'a mut Health, &'a mut Kinematics, Has<player::LocalPlayer>, Option<&'a hurt::Bleeds>);

fn deaths(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    mut deaths: ResMut<PlayerDeaths>,
    mut q: Query<Mortal>,
) {
    let spawn = sim.generator.spawn_point();
    for (entity, mut h, mut k, is_player, bleeds) in &mut q {
        if h.hp > 0.0 {
            continue;
        }
        if let Some(&hurt::Bleeds(blood)) = bleeds {
            // A burst of real blood cells: they fly, land, run and pool.
            sim.world.splash([k.body.pos.x, k.body.pos.y], blood, hurt::DEATH_BLOOD, 2.6);
        }
        if is_player {
            deaths.0 += 1;
            commands.entity(entity).remove::<(elements::Burning, elements::Coated, elements::Chilled)>();
            h.hp = h.max;
            if std::env::var("PLATYPUS_RESPAWN").is_ok() {
                k.body.pos = Vec2::new(spawn.x as f32, spawn.y as f32 + 60.0);
                k.body.vel = Vec2::ZERO;
                k.prev_pos = k.body.pos;
            }
        } else {
            commands.entity(entity).despawn();
        }
    }
}

/// Render between the last two ticks so 120 Hz displays stay smooth.
/// (Paused, as the arena pauses to step a tick at a time: where it is now.)
fn interpolate(time: Res<Time<Fixed>>, virt: Res<Time<Virtual>>, mut q: Query<(&Kinematics, &mut Transform)>) {
    let a = if virt.is_paused() { 1.0 } else { time.overstep_fraction() };
    for (k, mut tf) in &mut q {
        let p = k.prev_pos.lerp(k.body.pos, a);
        tf.translation.x = p.x;
        tf.translation.y = p.y;
    }
}
