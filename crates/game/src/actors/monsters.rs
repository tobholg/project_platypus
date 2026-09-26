//! Brains for the things that live underground (their creature files pick
//! one by name and tune it; see each's params). They hurt by touch
//! (`combat::Touch`) or with what they wield, like any creature.
//!
//! - `crawler` (spiders): goes at you over floors, walls and ceilings (its
//!   movement's `cling`), pounces when near, drops on you from above.
//! - `hopper` (slimes): hops at you, a hop every so often.
//! - `swooper` (vampire bats): flits in the air above; dives at you, then
//!   flies back up.
//! - `hatchery` (egg sacs): lies still until you come near (or hit it),
//!   then bursts into its brood.
//!
//! Each targets the nearest player within its `aggro_range`, and wanders
//! when none is near.

use bevy::prelude::*;
use platypus_sim::rng::Rng;
use serde::Deserialize;

use super::brain::RegisterBrain;
use super::creature::spawn_creature;
use super::{Controls, Health, Kinematics, Team};
use crate::world::{SimWorld, TickSet};

pub struct MonstersPlugin;

impl Plugin for MonstersPlugin {
    fn build(&self, app: &mut App) {
        app.register_brain::<Crawler>("crawler")
            .register_brain::<Hopper>("hopper")
            .register_brain::<Swooper>("swooper")
            .register_brain::<Hatchery>("hatchery")
            .add_systems(FixedUpdate, (crawler, hopper, swooper, hatchery).in_set(TickSet::Intent));
    }
}

type Players<'w, 's> = Query<'w, 's, (&'static Kinematics, &'static Team)>;

/// The nearest player within `range` of `pos`.
fn nearest(players: &Players, pos: Vec2, range: f32) -> Option<Vec2> {
    players
        .iter()
        .filter(|(_, t)| **t == Team::Player)
        .map(|(k, _)| k.body.pos)
        .filter(|p| p.distance(pos) < range)
        .min_by(|a, b| a.distance_squared(pos).total_cmp(&b.distance_squared(pos)))
}

/// 0..1 from a seeded roll.
fn unit(sim: &SimWorld, e: Entity, salt: u64) -> f32 {
    let mut rng = Rng::seeded(&[sim.world.seed(), sim.world.tick(), e.to_bits(), salt]);
    rng.next_u32() as f32 / u32::MAX as f32
}

/// What a monster remembers: when it may act next, where it wanders.
#[derive(Component, Default)]
pub struct MonsterMind {
    next: f32,
    wander: Vec2,
    wander_until: f32,
    /// A swooper: diving until, resting until.
    dive_until: f32,
}

fn mind<'a>(commands: &mut Commands, e: Entity, m: Option<Mut<'a, MonsterMind>>) -> Option<Mut<'a, MonsterMind>> {
    if m.is_none() {
        commands.entity(e).insert(MonsterMind::default());
    }
    m
}

/// Spiders: at you over any surface, pouncing when near.
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Crawler {
    pub aggro_range: f32,
    /// Leaps at you from this near (cells), every `pounce_every` s at most.
    pub pounce_range: f32,
    pub pounce_every: f32,
    pub wander_speed: f32,
}

impl Default for Crawler {
    fn default() -> Self {
        Crawler { aggro_range: 160.0, pounce_range: 30.0, pounce_every: 1.6, wander_speed: 0.4 }
    }
}

type Mover<'a, B> = (Entity, &'a B, &'a Kinematics, &'a mut Controls, Option<&'a mut MonsterMind>);

fn crawler(mut commands: Commands, time: Res<Time>, sim: Res<SimWorld>, players: Players, mut q: Query<Mover<Crawler>>) {
    let now = time.elapsed_secs();
    for (e, b, k, mut c, m) in &mut q {
        let Some(mut m) = mind(&mut commands, e, m) else { continue };
        let pos = k.body.pos;
        let mut jump = false;
        let (mx, my): (f32, f32);
        match nearest(&players, pos, b.aggro_range) {
            Some(t) => {
                let d = t - pos;
                mx = if d.x.abs() > 2.0 { d.x.signum() } else { 0.0 };
                // (Up walls toward you, and along ceilings; down, it lets go.)
                my = if d.y > 4.0 { 1.0 } else if d.y < -4.0 { -1.0 } else { 0.0 };
                let clinging = k.loco.clinging();
                let over_you = clinging == Some(Vec2::Y) && d.x.abs() < 8.0;
                let near = d.length() < b.pounce_range;
                if now >= m.next && (over_you || (near && (k.loco.grounded() || clinging.is_some()))) {
                    jump = true;
                    m.next = now + b.pounce_every * (0.7 + 0.6 * unit(&sim, e, 1));
                }
                c.0.aim = t;
            }
            None => {
                if now >= m.wander_until {
                    let a = unit(&sim, e, 2) * std::f32::consts::TAU;
                    m.wander = Vec2::from_angle(a);
                    m.wander_until = now + 1.0 + 2.0 * unit(&sim, e, 3);
                }
                mx = m.wander.x.signum() * b.wander_speed;
                my = m.wander.y.signum();
                c.0.aim = Vec2::ZERO;
            }
        }
        c.0.move_x = mx;
        c.0.move_y = my;
        c.0.jump = jump && !c.0.jump;
    }
}

/// Slimes: a hop at you every so often.
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Hopper {
    pub aggro_range: f32,
    /// Seconds between hops (±30 %).
    pub hop_every: f32,
}

impl Default for Hopper {
    fn default() -> Self {
        Hopper { aggro_range: 140.0, hop_every: 1.2 }
    }
}

fn hopper(mut commands: Commands, time: Res<Time>, sim: Res<SimWorld>, players: Players, mut q: Query<Mover<Hopper>>) {
    let now = time.elapsed_secs();
    for (e, b, k, mut c, m) in &mut q {
        let Some(mut m) = mind(&mut commands, e, m) else { continue };
        let target = nearest(&players, k.body.pos, b.aggro_range);
        let grounded = k.loco.grounded();
        // Airborne, it keeps going the way it hopped; down, it waits.
        let mut jump = false;
        if grounded {
            c.0.move_x = 0.0;
            if now >= m.next {
                let dir = match target {
                    Some(t) => (t.x - k.body.pos.x).signum(),
                    // (Idle: a hop now and then, either way.)
                    None if unit(&sim, e, 4) < 0.3 => if unit(&sim, e, 5) < 0.5 { -1.0 } else { 1.0 },
                    None => 0.0,
                };
                if dir != 0.0 {
                    jump = true;
                    c.0.move_x = dir;
                }
                let every = if target.is_some() { b.hop_every } else { b.hop_every * 2.5 };
                m.next = now + every * (0.7 + 0.6 * unit(&sim, e, 6));
            }
        }
        c.0.aim = target.unwrap_or(Vec2::ZERO);
        c.0.jump = jump && !c.0.jump;
    }
}

/// Vampire bats: flitting above; diving at you, then back up.
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Swooper {
    pub aggro_range: f32,
    /// Idle, it stays this high above the ground (cells).
    pub hover: (f32, f32),
    /// A dive lasts at most this long (s), and it rests this long between.
    pub dive_time: f32,
    pub dive_every: f32,
}

impl Default for Swooper {
    fn default() -> Self {
        Swooper { aggro_range: 150.0, hover: (20.0, 50.0), dive_time: 1.2, dive_every: 2.5 }
    }
}

fn swooper(mut commands: Commands, time: Res<Time>, sim: Res<SimWorld>, players: Players, mut q: Query<Mover<Swooper>>) {
    let now = time.elapsed_secs();
    for (e, b, k, mut c, m) in &mut q {
        let Some(mut m) = mind(&mut commands, e, m) else { continue };
        let pos = k.body.pos;
        let target = nearest(&players, pos, b.aggro_range);
        let mut steer: Vec2;
        match target {
            // Diving: straight at you.
            Some(t) if now < m.dive_until => steer = (t - pos).normalize_or_zero(),
            Some(t) if now >= m.next => {
                m.dive_until = now + b.dive_time;
                m.next = m.dive_until + b.dive_every * (0.7 + 0.6 * unit(&sim, e, 7));
                steer = (t - pos).normalize_or_zero();
            }
            _ => {
                if now >= m.wander_until {
                    m.wander = Vec2::from_angle(unit(&sim, e, 8) * std::f32::consts::TAU);
                    m.wander_until = now + 0.3 + 0.4 * unit(&sim, e, 9);
                }
                steer = m.wander;
                // Back up into its band above the ground.
                let h = super::critters::height(&sim.world, pos, (b.hover.1 as i32) * 2);
                if h < b.hover.0 {
                    steer.y = steer.y.abs().max(0.7);
                } else if h > b.hover.1 {
                    steer.y = -steer.y.abs().max(0.5);
                }
            }
        }
        c.0.move_x = steer.x;
        c.0.move_y = if steer.y == 0.0 { 0.15 } else { steer.y };
        c.0.aim = target.unwrap_or(Vec2::ZERO);
        c.0.jump = false;
    }
}

/// Egg sacs: still until you come near or hit it; then its brood bursts out.
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Hatchery {
    /// What hatches, and how many.
    pub brood: String,
    pub count: u32,
    /// Hatches when a player comes this near (cells).
    pub range: f32,
}

impl Default for Hatchery {
    fn default() -> Self {
        Hatchery { brood: "spiderling".into(), count: 3, range: 40.0 }
    }
}

fn hatchery(mut commands: Commands, sim: Res<SimWorld>, players: Players, mut q: Query<(Entity, &Hatchery, &Kinematics, &mut Health)>) {
    for (e, h, k, mut hp) in &mut q {
        let near = nearest(&players, k.body.pos, h.range).is_some();
        if !near && hp.hp >= hp.max {
            continue;
        }
        for i in 0..h.count {
            let dx = (unit(&sim, e, 10 + i as u64) - 0.5) * 8.0;
            spawn_creature(&mut commands, &h.brood, k.body.pos + Vec2::new(dx, 1.0), |_| {});
        }
        // It bursts (its `blood`: what it's full of).
        hp.hp = 0.0;
    }
}
