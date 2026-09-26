//! Critters: small harmless life (rabbits, birds, frogs). A brain that
//! rests, wanders and flees (from players, blasts, being hurt), hopping or
//! flying as its kind does; and ambient spawning round the player from
//! `assets/data/life.ron`, so the world is lived in without placing
//! anything by hand.

use bevy::prelude::*;
use platypus_sim::rng::Rng;
use platypus_sim::{CellPos, Kind};
use serde::Deserialize;

use super::brain::RegisterBrain;
use super::creature::spawn_creature;
use super::{Controls, Creature, Health, Kinematics, Team};
use crate::data::{Watched, data_path, load_ron};
use crate::world::{SimWorld, TickSet};

pub struct CrittersPlugin;

impl Plugin for CrittersPlugin {
    fn build(&self, app: &mut App) {
        app.register_brain::<Critter>("critter")
            .insert_resource(Life::load())
            .add_systems(FixedUpdate, (scare, critters).chain().in_set(TickSet::Intent))
            .add_systems(Update, (reload_life, ambient));
    }
}

/// How a critter behaves (its creature file's `brain.params`).
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Critter {
    /// Flees a player nearer than this (cells).
    pub flee_range: f32,
    /// Keeps fleeing this long after the last scare (seconds).
    pub calm_after: f32,
    /// Moves only by hopping (rabbits, frogs).
    pub hops: bool,
    /// Flies away when scared, glides back down when calm (birds).
    pub flies: bool,
    /// Seconds between decisions when calm (randomised ±50 %).
    pub wander_every: f32,
    /// Share of calm decisions that are to stay put.
    pub rest: f32,
    /// Share of run speed when wandering.
    pub wander_speed: f32,
}

impl Default for Critter {
    fn default() -> Self {
        Critter { flee_range: 40.0, calm_after: 3.0, hops: false, flies: false, wander_every: 2.0, rest: 0.6, wander_speed: 0.5 }
    }
}

/// A critter's mind: fleeing till when, from where; which way it's going.
#[derive(Component, Default)]
pub struct CritterMind {
    until: f32,
    from: Vec2,
    dir: f32,
    next: f32,
    last_hp: Option<f32>,
}

/// Blasts and hurts scare critters (a blast: within 4 × its radius).
fn scare(mut blasts: MessageReader<crate::fx::Explosion>, time: Res<Time>, mut q: Query<(&Critter, &Kinematics, &Health, &mut CritterMind)>) {
    let now = time.elapsed_secs();
    let blasts: Vec<_> = blasts.read().copied().collect();
    for (c, k, h, mut m) in &mut q {
        for b in &blasts {
            if k.body.pos.distance(b.at) < b.radius * 4.0 {
                m.until = now + c.calm_after;
                m.from = b.at;
            }
        }
        if m.last_hp.is_some_and(|last| h.hp < last - 0.5) {
            m.until = now + c.calm_after;
            m.from = k.body.pos - Vec2::new(k.loco.facing, 0.0) * 10.0;
        }
        m.last_hp = Some(h.hp);
    }
}

fn critters(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<SimWorld>,
    players: Query<(&Kinematics, &Team), Without<Critter>>,
    mut q: Query<(Entity, &Critter, &Kinematics, &mut Controls, Option<&mut CritterMind>)>,
) {
    let now = time.elapsed_secs();
    for (e, c, k, mut controls, mind) in &mut q {
        let Some(mut m) = mind else {
            commands.entity(e).insert(CritterMind::default());
            continue;
        };
        let pos = k.body.pos;
        if let Some(p) = players.iter().filter(|(_, t)| **t == Team::Player).map(|(pk, _)| pk.body.pos).find(|p| p.distance(pos) < c.flee_range) {
            m.until = now + c.calm_after;
            m.from = p;
        }
        let fleeing = now < m.until;
        let grounded = k.loco.grounded();
        let mut move_y = 0.0;
        let mut jump = false;
        let mut move_x;
        if fleeing {
            move_x = if pos.x >= m.from.x { 1.0 } else { -1.0 };
            if c.flies {
                move_y = 0.8;
            }
        } else {
            if now >= m.next {
                let mut rng = Rng::seeded(&[sim.world.seed(), sim.world.tick(), e.to_bits()]);
                let unit = |rng: &mut Rng| rng.next_u32() as f32 / u32::MAX as f32;
                m.dir = if unit(&mut rng) < c.rest { 0.0 } else if unit(&mut rng) < 0.5 { -1.0 } else { 1.0 };
                m.next = now + c.wander_every * (0.5 + unit(&mut rng));
            }
            move_x = m.dir * c.wander_speed;
            // A flyer that's calm glides down to land.
            if c.flies && !grounded {
                move_y = -0.6;
                move_x = m.dir.signum() * 0.5;
            }
        }
        // Walled: turn round.
        let cts = &k.loco.contacts;
        if (move_x > 0.0 && cts.wall_right) || (move_x < 0.0 && cts.wall_left) {
            m.dir = -m.dir;
            if fleeing {
                m.from = pos + Vec2::new(move_x * 10.0, 0.0);
            }
            move_x = -move_x;
        }
        // Hoppers move in hops: a hop whenever they're down and going.
        if c.hops && move_x != 0.0 {
            jump = grounded;
            // (Ambling, it pauses between hops; fleeing, it doesn't.)
            if grounded && !controls.0.jump && !fleeing {
                move_x *= 0.3;
            }
        }
        controls.0.move_x = move_x;
        controls.0.move_y = move_y;
        controls.0.jump = jump && !controls.0.jump;
    }
}

/// Where a kind lives, from `assets/data/life.ron`.
#[derive(Clone, Debug, Deserialize)]
pub struct Haunt {
    pub kind: String,
    /// On the open surface (`Surface`), or near water (`Shore`).
    #[serde(default = "surface")]
    pub place: Place,
    /// At most this many of it round the player.
    pub most: usize,
    /// A chance a second to spawn one when there are fewer.
    #[serde(default = "half")]
    pub rate: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Place {
    Surface,
    Shore,
}

fn surface() -> Place {
    Place::Surface
}

fn half() -> f32 {
    0.5
}

#[derive(Resource)]
struct Life {
    haunts: Vec<Haunt>,
    watch: Watched,
}

impl Life {
    fn load() -> Self {
        let path = data_path("life.ron");
        let haunts = load_ron(&path).unwrap_or_else(|e| {
            warn!("{e}");
            Vec::new()
        });
        Life { haunts, watch: Watched::new(path) }
    }
}

fn reload_life(mut life: ResMut<Life>) {
    if !life.watch.changed() {
        return;
    }
    match load_ron(life.watch.path()) {
        Ok(h) => {
            life.haunts = h;
            info!("life reloaded");
        }
        Err(e) => warn!("life not reloaded: {e}"),
    }
}

/// Critters come and go round the player: spawned out of sight (just past
/// the screen's edge, on the surface), gone when far away.
const NEAR: f32 = 450.0;
const GONE: f32 = 800.0;

#[allow(clippy::too_many_arguments)]
fn ambient(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<SimWorld>,
    life: Res<Life>,
    player: Query<&Kinematics, With<super::player::LocalPlayer>>,
    critters: Query<(Entity, &Creature, &Kinematics), With<Critter>>,
    mut clock: Local<f32>,
    mut seed: Local<u64>,
) {
    let Ok(pk) = player.single() else { return };
    if !sim.generator.wild() {
        return;
    }
    let p = pk.body.pos;
    for (e, _, k) in &critters {
        if k.body.pos.distance(p) > GONE {
            commands.entity(e).despawn();
        }
    }
    *clock += time.delta_secs();
    if *clock < 0.5 {
        return;
    }
    *clock = 0.0;
    *seed += 1;
    let mut rng = Rng::seeded(&[sim.world.seed(), *seed, 0x1F3]);
    let unit = |rng: &mut Rng| rng.next_u32() as f32 / u32::MAX as f32;
    let mats = sim.world.materials();
    for h in &life.haunts {
        let near = critters.iter().filter(|(_, c, k)| c.kind == h.kind && k.body.pos.distance(p) < NEAR).count();
        if near >= h.most || unit(&mut rng) > h.rate * 0.5 {
            continue;
        }
        let side = if unit(&mut rng) < 0.5 { -1.0 } else { 1.0 };
        let x = (p.x + side * (330.0 + unit(&mut rng) * 110.0)) as i32;
        // The ground there: the first solid cell under open air, near the
        // player's height (not in a cave far below).
        let top = sim.generator.surface_hint(x).unwrap_or(p.y as i32) + 40;
        let Some(y) = (top - 160..=top).rev().find(|&y| {
            let here = sim.world.get(CellPos::new(x, y));
            let above = sim.world.get(CellPos::new(x, y + 1));
            here.is_some_and(|c| matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder)) && above.is_some_and(|c| c.is_air() || mats.phys(c.material).kind == Kind::Plant)
        }) else {
            continue;
        };
        if (y as f32 - p.y).abs() > 200.0 {
            continue;
        }
        let shore = (-30..=30).any(|dx| sim.world.get(CellPos::new(x + dx, y)).is_some_and(|c| mats.phys(c.material).kind == Kind::Liquid));
        if (h.place == Place::Shore) != shore {
            continue;
        }
        spawn_creature(&mut commands, &h.kind, Vec2::new(x as f32 + 0.5, y as f32 + 1.0), |_| {});
    }
}
