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
    /// Share of run (or fly, or swim) speed when wandering.
    pub wander_speed: f32,
    /// Stays in the air, between these heights above the ground (cells):
    /// fireflies drifting, bats flitting (with a short `wander_every`).
    #[serde(deserialize_with = "crate::data::some")]
    pub hovers: Option<(f32, f32)>,
    /// Lives in water: swims about in it and turns back at its edges;
    /// stranded, it flops.
    pub swims: bool,
}

impl Default for Critter {
    fn default() -> Self {
        Critter { flee_range: 40.0, calm_after: 3.0, hops: false, flies: false, wander_every: 2.0, rest: 0.6, wander_speed: 0.5, hovers: None, swims: false }
    }
}

/// A critter's mind: fleeing till when, from where; which way it's going.
#[derive(Component, Default)]
pub struct CritterMind {
    until: f32,
    from: Vec2,
    dir: Vec2,
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

/// How far above the ground (or water) a point is, up to `most` cells.
pub(crate) fn height(world: &platypus_sim::World, at: Vec2, most: i32) -> f32 {
    let (x, y) = (at.x.floor() as i32, at.y.floor() as i32);
    (0..most)
        .find(|d| world.get(CellPos::new(x, y - d)).is_none_or(|c| matches!(world.materials().phys(c.material).kind, Kind::Static | Kind::Powder | Kind::Liquid)))
        .unwrap_or(most) as f32
}

fn liquid(world: &platypus_sim::World, at: Vec2) -> bool {
    world.get(CellPos::from_world(at.x, at.y)).is_some_and(|c| world.materials().phys(c.material).kind == Kind::Liquid)
}

fn critters(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<SimWorld>,
    players: Query<(&Kinematics, &Team), Without<Critter>>,
    mut q: Query<(Entity, &Critter, &Kinematics, &mut Controls, Option<&mut CritterMind>)>,
) {
    let now = time.elapsed_secs();
    let world = &sim.world;
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
        let mut rng = Rng::seeded(&[world.seed(), world.tick(), e.to_bits()]);
        let unit = |rng: &mut Rng| rng.next_u32() as f32 / u32::MAX as f32;
        let away = (pos - m.from).normalize_or(Vec2::X);
        // Every so often, a new way to go (or none: a rest).
        let decide = |m: &mut CritterMind, rng: &mut Rng, two_d: bool| {
            if now >= m.next {
                let a = unit(rng) * std::f32::consts::TAU;
                m.dir = if unit(rng) < c.rest {
                    Vec2::ZERO
                } else if two_d {
                    Vec2::from_angle(a)
                } else {
                    Vec2::new(if unit(rng) < 0.5 { -1.0 } else { 1.0 }, 0.0)
                };
                m.next = now + c.wander_every * (0.5 + unit(rng));
            }
        };
        let cts = &k.loco.contacts;
        let (mut move_x, mut move_y, mut jump) = (0.0, 0.0, false);
        if c.swims {
            if cts.submerged > 0.3 {
                let mut dir = if fleeing { away } else {
                    decide(&mut m, &mut rng, true);
                    m.dir
                };
                // Not out of the water: turn back at its edges, and stay
                // under its surface.
                let ahead = pos + dir.normalize_or_zero() * (k.body.half.x + 3.0);
                if dir != Vec2::ZERO && !liquid(world, ahead) {
                    dir = -dir;
                    m.dir = dir;
                }
                if !liquid(world, pos + Vec2::new(0.0, k.body.half.y + 2.0)) {
                    dir.y = dir.y.min(-0.3);
                }
                let speed = if fleeing { 1.0 } else { c.wander_speed };
                (move_x, move_y) = (dir.x * speed, dir.y * speed);
            } else if grounded && unit(&mut rng) < 0.05 {
                // Stranded: it flops.
                jump = true;
                move_x = if unit(&mut rng) < 0.5 { -1.0 } else { 1.0 };
            }
        } else if let Some((low, high)) = c.hovers {
            let mut dir = if fleeing { (away + Vec2::Y).normalize() } else {
                decide(&mut m, &mut rng, true);
                m.dir
            };
            let h = height(world, pos, (high as i32) * 2);
            if h < low {
                dir.y = dir.y.abs().max(0.6);
            } else if h > high {
                dir.y = -dir.y.abs().max(0.6);
            }
            if cts.ceiling {
                dir.y = -dir.y.abs();
            }
            if (dir.x > 0.0 && cts.wall_right) || (dir.x < 0.0 && cts.wall_left) {
                dir.x = -dir.x;
                m.dir.x = -m.dir.x;
            }
            let speed = if fleeing { 1.0 } else { c.wander_speed };
            (move_x, move_y) = (dir.x * speed, dir.y * speed);
            if move_y == 0.0 {
                // (Hold its height: a flier not steering sinks.)
                move_y = 0.15;
            }
        } else {
            if fleeing {
                move_x = if pos.x >= m.from.x { 1.0 } else { -1.0 };
                if c.flies {
                    move_y = 0.8;
                }
            } else {
                decide(&mut m, &mut rng, false);
                move_x = m.dir.x * c.wander_speed;
                // A flyer that's calm glides down to land.
                if c.flies && !grounded {
                    move_y = -0.6;
                    move_x = m.dir.x.signum() * 0.5;
                }
            }
            // Walled: turn round.
            if (move_x > 0.0 && cts.wall_right) || (move_x < 0.0 && cts.wall_left) {
                m.dir.x = -m.dir.x;
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
    /// On the open surface (`Surface`), near water (`Shore`), in it
    /// (`Water`), or in the air of caves (`Cave`).
    #[serde(default = "surface")]
    pub place: Place,
    /// At what time of day (`Any`, `Day`, `Night`); out of its hours it
    /// leaves (once it's out of sight).
    #[serde(default)]
    pub when: When,
    /// Put this high above the ground (cells, low..high): fireflies.
    #[serde(default)]
    pub above: Option<(f32, f32)>,
    /// Only this deep under the surface (cells, from..to).
    #[serde(default)]
    pub depth: Option<(f32, f32)>,
    /// Only in this underground biome (`fungal`, `crystal`, `toxic`, or
    /// `none` for plain rock).
    #[serde(default)]
    pub zone: Option<String>,
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
    Water,
    Cave,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum When {
    #[default]
    Any,
    Day,
    Night,
}

impl When {
    /// `time`: of day, 0..1 from midnight.
    fn now(self, time: f32) -> bool {
        let night = !(0.22..0.8).contains(&time);
        match self {
            When::Any => true,
            When::Day => !night,
            When::Night => night,
        }
    }
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
    if !life.bypass_change_detection().watch.changed() {
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
/// the screen's edge, on the surface; anywhere near for water and caves),
/// gone when far away, or out of their hours and out of sight.
const NEAR: f32 = 450.0;
const GONE: f32 = 800.0;
const OUT_OF_SIGHT: f32 = 260.0;
/// Tries to find a spot of water or cave a spawn.
const TRIES: usize = 16;

#[allow(clippy::too_many_arguments)]
fn ambient(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<SimWorld>,
    life: Res<Life>,
    day: Res<crate::light::Daylight>,
    player: Query<&Kinematics, With<super::player::LocalPlayer>>,
    critters: Query<(Entity, &Creature, &Kinematics), Without<super::player::LocalPlayer>>,
    mut clock: Local<f32>,
    mut seed: Local<u64>,
) {
    let Ok(pk) = player.single() else { return };
    if !sim.generator.wild() {
        return;
    }
    let p = pk.body.pos;
    // (Only what life.ron brings comes and goes this way.)
    for (e, c, k) in &critters {
        let d = k.body.pos.distance(p);
        let haunts = life.haunts.iter().any(|h| h.kind == c.kind);
        let off_hours = life.haunts.iter().any(|h| h.kind == c.kind && !h.when.now(day.time));
        if haunts && (d > GONE || (off_hours && d > OUT_OF_SIGHT)) {
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
    let world = &sim.world;
    let mats = world.materials();
    let kind_at = |x: i32, y: i32| world.get(CellPos::new(x, y)).map(|c| (c.is_air(), mats.phys(c.material)));
    for h in &life.haunts {
        if !h.when.now(day.time) {
            continue;
        }
        let near = critters.iter().filter(|(_, c, k)| c.kind == h.kind && k.body.pos.distance(p) < NEAR).count();
        if near >= h.most || unit(&mut rng) > h.rate * 0.5 {
            continue;
        }
        let spot = match h.place {
            Place::Surface | Place::Shore => {
                let side = if unit(&mut rng) < 0.5 { -1.0 } else { 1.0 };
                let x = (p.x + side * (330.0 + unit(&mut rng) * 110.0)) as i32;
                // The ground there: the first solid cell under open air, near
                // the player's height (not in a cave far below).
                let top = sim.generator.surface_hint(x).unwrap_or(p.y as i32) + 40;
                let Some(y) = (top - 160..=top).rev().find(|&y| {
                    let here = world.get(CellPos::new(x, y));
                    let above = world.get(CellPos::new(x, y + 1));
                    here.is_some_and(|c| matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder)) && above.is_some_and(|c| c.is_air() || mats.phys(c.material).kind == Kind::Plant)
                }) else {
                    continue;
                };
                if (y as f32 - p.y).abs() > 200.0 {
                    continue;
                }
                let shore = (-30..=30).any(|dx| world.get(CellPos::new(x + dx, y)).is_some_and(|c| mats.phys(c.material).kind == Kind::Liquid));
                if (h.place == Place::Shore) != shore {
                    continue;
                }
                Some(Vec2::new(x as f32 + 0.5, y as f32 + 1.0))
            }
            // Somewhere near in cool liquid, with room round it.
            Place::Water => (0..TRIES).find_map(|_| {
                let (x, y) = ((p.x + (unit(&mut rng) - 0.5) * 760.0) as i32, (p.y + (unit(&mut rng) - 0.5) * 400.0) as i32);
                let wet = |dx: i32, dy: i32| kind_at(x + dx, y + dy).is_some_and(|(_, ph)| ph.kind == Kind::Liquid && !ph.hot);
                (wet(0, 0) && wet(-4, 0) && wet(4, 0) && wet(0, 3) && wet(0, -3)).then(|| Vec2::new(x as f32 + 0.5, y as f32 - 1.0))
            }),
            // In the air of a cave: open round it, a roof above, well under
            // the surface.
            Place::Cave => (0..TRIES).find_map(|_| {
                let (x, y) = ((p.x + (unit(&mut rng) - 0.5) * 800.0) as i32, (p.y + (unit(&mut rng) - 0.5) * 520.0) as i32);
                if sim.generator.surface_hint(x).is_some_and(|s| y > s - 30) {
                    return None;
                }
                let open = |dx: i32, dy: i32| kind_at(x + dx, y + dy).is_some_and(|(air, _)| air);
                let roofed = (4..40).any(|dy| kind_at(x, y + dy).is_some_and(|(_, ph)| matches!(ph.kind, Kind::Static)));
                (open(0, 0) && open(-4, 0) && open(4, 0) && open(0, 4) && open(0, -4) && roofed).then(|| Vec2::new(x as f32 + 0.5, y as f32))
            }),
        };
        let Some(mut at) = spot else { continue };
        // Deep enough, in its biome, and out of sight (not popping in).
        let depth = sim.generator.surface_hint(at.x as i32).map_or(0.0, |s| s as f32 - at.y);
        if h.depth.is_some_and(|(lo, hi)| !(lo..hi).contains(&depth)) {
            continue;
        }
        if let Some(z) = &h.zone
            && sim.generator.zone_at(at.x as i32, at.y as i32).unwrap_or("none") != z
        {
            continue;
        }
        if h.place == Place::Cave && at.distance(p) < OUT_OF_SIGHT * 0.7 {
            continue;
        }
        if let Some((lo, hi)) = h.above {
            at.y += lo + unit(&mut rng) * (hi - lo);
        }
        spawn_creature(&mut commands, &h.kind, at, |_| {});
    }
}
