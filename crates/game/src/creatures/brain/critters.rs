//! Critters: small harmless life (rabbits, birds, frogs). A brain that
//! rests, wanders and flees (from players, blasts, being hurt), hopping or
//! flying as its kind does; and ambient spawning round the player from
//! `assets/data/life.ron`, so the world is lived in without placing
//! anything by hand.

use bevy::prelude::*;
use platypus_sim::rng::Rng;
use platypus_sim::{CellPos, Kind};
use serde::Deserialize;

use crate::creatures::brain::RegisterBrain;
use crate::creatures::def::spawn_creature;
use crate::creatures::{Controls, Creature, Health, Kinematics, Team};
use crate::data::{Watched, data_path, load_ron};
use crate::world::SimWorld;

pub struct CrittersPlugin;

impl Plugin for CrittersPlugin {
    fn build(&self, app: &mut App) {
        app.register_brain::<Critter>("critter")
            .insert_resource(Life::load())
            .add_systems(FixedUpdate, (scare, critters).chain().in_set(super::BrainSet))
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
        Critter { flee_range: 60.0, calm_after: 3.0, hops: false, flies: false, wander_every: 2.0, rest: 0.6, wander_speed: 0.5, hovers: None, swims: false }
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
            m.from = k.body.pos - Vec2::new(k.loco.facing, 0.0) * 15.0;
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
    ids: Query<&crate::creatures::Stable>,
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
        let mut rng = Rng::seeded(&[world.seed(), world.tick(), crate::creatures::stable(&ids, e)]);
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
                let ahead = pos + dir.normalize_or_zero() * (k.body.half.x + 4.5);
                if dir != Vec2::ZERO && !liquid(world, ahead) {
                    dir = -dir;
                    m.dir = dir;
                }
                if !liquid(world, pos + Vec2::new(0.0, k.body.half.y + 3.0)) {
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
                    m.from = pos + Vec2::new(move_x * 15.0, 0.0);
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
    /// Only in these depth bands (`surface`, `underground`, `caverns`,
    /// `deep`, `underworld`: as deep as each is in this world's size); none:
    /// any.
    #[serde(default)]
    pub bands: Vec<String>,
    /// At most this many of it round the player (with `packs`, no more
    /// than the biggest pack at the player's depth either), its lairs'
    /// keepers counted.
    pub most: usize,
    /// A chance a second to spawn one when there are fewer.
    #[serde(default = "half")]
    pub rate: f32,
    /// Only over these surface biomes (`plains`, `forest`, `desert`,
    /// `tundra`, `jungle`, `swamp`, `deep forest`, `mountains`, `ocean`);
    /// none: anywhere.
    #[serde(default)]
    pub biomes: Vec<String>,
    /// Come as a group (a pack): between this many and that many at once,
    /// side by side (still no more than `most` near).
    #[serde(default)]
    pub group: Option<(usize, usize)>,
    /// Packs by depth, in place of `group`: from this deep (cells), one up
    /// to this many (`pack_size`: from four up, the smaller more often).
    #[serde(default)]
    pub packs: Vec<(f32, usize)>,
    /// Never within this many cells of where the world starts (the big
    /// ones stay out of a new player's first days).
    #[serde(default)]
    pub away: f32,
    /// Only within this many cells of the world's start (0: anywhere): with
    /// `away`, a ring round it (the start valley's: home, then its edges).
    #[serde(default)]
    pub within: f32,
    /// It comes up out of the ground (the dead: `emerge.rs`), on the
    /// ground nearer than the rest (`EMERGE_NEAR`: so it's seen coming
    /// up), a group one after another.
    #[serde(default)]
    pub emerges: bool,
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
const NEAR: f32 = 675.0;
const GONE: f32 = 1200.0;
const OUT_OF_SIGHT: f32 = 390.0;
/// Tries to find a spot of water or cave a spawn.
const TRIES: usize = 16;
/// Spawn rates scaled (`PLATYPUS_LIFE_RATE`: scenarios that look at what
/// lives where, without waiting minutes).
/// What lives in the caves turns up at this share of its `rate` (the
/// underground was too busy: halved).
const CAVE_RATE: f32 = 0.5;

fn rate_scale() -> f32 {
    static K: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *K.get_or_init(|| std::env::var("PLATYPUS_LIFE_RATE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0))
}

/// Water life: one of a kind for this many columns of a lake's breadth,
/// looked for this far either side.
const WATER_PER: i32 = 60;
const WATER_SPAN: i32 = 1200;

/// The biggest pack at a depth (`Haunt::packs`).
pub fn pack_most(packs: &[(f32, usize)], depth: f32) -> usize {
    packs.iter().filter(|(d, _)| depth >= *d).map(|&(_, m)| m).last().unwrap_or(1).max(1)
}

/// A pack's size at a depth (`Haunt::packs`): one up to the most for
/// that deep, by `roll` (0..1); from four up the smaller more often (each
/// size's weight `2 × most + 1 − size`: of five, 25 % one, 15 % five), so a
/// deep pack is often a few, now and then the most.
pub fn pack_size(packs: &[(f32, usize)], depth: f32, roll: f32) -> usize {
    let most = pack_most(packs, depth);
    let weight = |n: usize| if most >= 4 { (2 * most + 1 - n) as f32 } else { 1.0 };
    let total: f32 = (1..=most).map(weight).sum();
    let mut left = roll.clamp(0.0, 0.999_999) * total;
    for n in 1..=most {
        left -= weight(n);
        if left < 0.0 {
            return n;
        }
    }
    most
}

/// A group's members this far apart (cells).
const PACK_GAP: f32 = 24.0;
/// What comes up out of the ground does it this near the player (cells,
/// from..to), one after another this far apart (seconds).
const EMERGE_NEAR: (f32, f32) = (150.0, 420.0);
const EMERGE_APART: f32 = 0.9;

/// Passing life: not the player, not a lair's keeper (they stay).
type Passing = (Without<crate::creatures::player::LocalPlayer>, Without<crate::clock::Keeps>, Without<super::Staged>);

#[allow(clippy::too_many_arguments)]
fn ambient(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<SimWorld>,
    life: Res<Life>,
    day: Res<crate::light::Daylight>,
    player: Query<&Kinematics, With<crate::creatures::player::LocalPlayer>>,
    critters: Query<(Entity, &Creature, &Kinematics), Passing>,
    keepers: Query<(&Creature, &Kinematics), With<crate::clock::Keeps>>,
    mut clock: Local<f32>,
    mut seed: Local<u64>,
    show: Res<crate::show::ShowMode>,
) {
    let Ok(pk) = player.single() else { return };
    // (Show mode, `show.rs`: life often, the big ones anywhere, what lives
    // deep a quarter as deep.)
    let show = show.0;
    if !sim.generator.wild() {
        return;
    }
    let p = pk.body.pos;
    // (Only what life.ron brings comes and goes this way.)
    for (e, c, k) in &critters {
        let d = k.body.pos.distance(p);
        let haunts = life.haunts.iter().any(|h| h.kind == c.kind);
        // (Off its hours only when none of its kind's haunts are in theirs:
        // a zombie of the night, by day, is still the dark wood's.)
        let off_hours = life.haunts.iter().filter(|h| h.kind == c.kind).all(|h| !h.when.now(day.time));
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
    let start = sim.generator.spawn_point();
    for h in &life.haunts {
        if !h.when.now(day.time) {
            continue;
        }
        // (Its lairs' keepers count: none brought to a roost already full.)
        let near = critters.iter().map(|(_, c, k)| (c, k)).chain(keepers.iter()).filter(|(c, k)| c.kind == h.kind && k.body.pos.distance(p) < NEAR).count();
        // (No more round the player than its biggest pack where it is: two
        // bats near the top, five deep down.)
        let here = sim.generator.surface_hint(p.x as i32).map_or(0.0, |s| s as f32 - p.y);
        let most = if h.packs.is_empty() { h.most } else { h.most.min(pack_most(&h.packs, here)) };
        let below = if h.place == Place::Cave { CAVE_RATE } else { 1.0 };
        if near >= most || unit(&mut rng) > h.rate * 0.5 * below * rate_scale() * if show { crate::show::SHOW_RATE } else { 1.0 } {
            continue;
        }
        // (Not near the start, for the big ones.)
        let from_start = (p.x - start.x as f32).abs();
        if !show && ((h.away > 0.0 && from_start < h.away) || (h.within > 0.0 && from_start > h.within)) {
            continue;
        }
        let spot = match h.place {
            Place::Surface | Place::Shore => {
                let side = if unit(&mut rng) < 0.5 { -1.0 } else { 1.0 };
                let (near, far) = if h.emerges { EMERGE_NEAR } else { (495.0, 660.0) };
                let x = (p.x + side * (near + unit(&mut rng) * (far - near))) as i32;
                // The ground there: the first solid cell under open air, near
                // the player's height (not in a cave far below).
                let top = sim.generator.surface_hint(x).unwrap_or(p.y as i32) + 60;
                let Some(y) = (top - 240..=top).rev().find(|&y| {
                    let here = world.get(CellPos::new(x, y));
                    let above = world.get(CellPos::new(x, y + 1));
                    // (The ground, not a tree's wood: through the woods to
                    // the floor.)
                    here.is_some_and(|c| matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder) && !mats.phys(c.material).object) && above.is_some_and(|c| c.is_air() || mats.phys(c.material).kind == Kind::Plant)
                }) else {
                    continue;
                };
                if (y as f32 - p.y).abs() > 300.0 {
                    continue;
                }
                // (A shore: by one of the seed's lakes or oceans, at its
                // level, with water still in it. Not a puddle, a pond the
                // rain filled, a pool of blood or acid.)
                let lake = (-45..=45).step_by(3).any(|dx| sim.generator.water_hint(x + dx).is_some_and(|level| (level - y).abs() <= 30));
                let shore = lake && (-45..=45).any(|dx| world.get(CellPos::new(x + dx, y)).is_some_and(|c| mats.phys(c.material).kind == Kind::Liquid && !mats.phys(c.material).hot));
                if (h.place == Place::Shore) != shore {
                    continue;
                }
                Some(Vec2::new(x as f32 + 0.5, y as f32 + 1.0))
            }
            // Somewhere near in one of the seed's lakes or oceans (not a
            // pond the rain filled, nor water underground), in cool liquid
            // with room round it; no more of its kind there than the water's
            // breadth holds (a small lake, one or two).
            Place::Water => (0..TRIES).find_map(|_| {
                let (x, y) = ((p.x + (unit(&mut rng) - 0.5) * 1140.0) as i32, (p.y + (unit(&mut rng) - 0.5) * 600.0) as i32);
                let level = sim.generator.water_hint(x).filter(|&level| y < level - 2)?;
                let wet = |dx: i32, dy: i32| kind_at(x + dx, y + dy).is_some_and(|(_, ph)| ph.kind == Kind::Liquid && !ph.hot);
                if !(wet(0, 0) && wet(-6, 0) && wet(6, 0) && wet(0, 5) && wet(0, -5)) {
                    return None;
                }
                let reach = |dir: i32| (1..=WATER_SPAN / 4).take_while(|&n| sim.generator.water_hint(x + dir * n * 4).is_some()).count() as i32 * 4;
                let (lo, hi) = (x - reach(-1), x + reach(1));
                let room = ((hi - lo) / WATER_PER).max(1) as usize;
                let there = critters.iter().filter(|(_, c, k)| c.kind == h.kind && (lo as f32..=hi as f32).contains(&k.body.pos.x) && k.body.pos.y < level as f32 + 5.0).count();
                (there < room).then(|| Vec2::new(x as f32 + 0.5, y as f32 - 1.0))
            }),
            // In the air of a cave: open round it, a roof above, well under
            // the surface.
            Place::Cave => (0..TRIES).find_map(|_| {
                let (x, y) = ((p.x + (unit(&mut rng) - 0.5) * 1200.0) as i32, (p.y + (unit(&mut rng) - 0.5) * 780.0) as i32);
                if sim.generator.surface_hint(x).is_some_and(|s| y > s - 45) {
                    return None;
                }
                let open = |dx: i32, dy: i32| kind_at(x + dx, y + dy).is_some_and(|(air, _)| air);
                let roofed = (6..60).any(|dy| kind_at(x, y + dy).is_some_and(|(_, ph)| matches!(ph.kind, Kind::Static)));
                if !(open(0, 0) && open(-6, 0) && open(6, 0) && open(0, 6) && open(0, -6) && roofed) {
                    return None;
                }
                // (Coming up out of the ground: on the floor under there.)
                if h.emerges {
                    let floor = (y - 80..y).rev().find(|&fy| kind_at(x, fy).is_some_and(|(air, ph)| !air && matches!(ph.kind, Kind::Static | Kind::Powder)))?;
                    return Some(Vec2::new(x as f32 + 0.5, floor as f32 + 1.0));
                }
                Some(Vec2::new(x as f32 + 0.5, y as f32))
            }),
        };
        let Some(mut at) = spot else { continue };
        // (What comes up out of the ground: only through soft ground round
        // it, where its pit goes, not rock; the ground there as it lies,
        // up or down a little, on a slope or among roots.)
        let soft_at = |x: i32| {
            (at.y as i32 - 10..=at.y as i32 + 10).rev().find(|&y| kind_at(x, y).is_some_and(|(air, ph)| !air && matches!(ph.kind, Kind::Static | Kind::Powder))).is_some_and(|y| kind_at(x, y).is_some_and(|(_, ph)| crate::creatures::body::emerge::soft(ph)))
        };
        if h.emerges && (-12..=12).step_by(3).filter(|&dx| soft_at(at.x as i32 + dx)).count() < 7 {
            continue;
        }
        // Deep enough, in its biome, and out of sight (not popping in).
        let depth = sim.generator.surface_hint(at.x as i32).map_or(0.0, |s| s as f32 - at.y);
        if h.depth.is_some_and(|(lo, hi)| !((if show { lo / 4.0 } else { lo })..hi).contains(&depth)) {
            continue;
        }
        if let Some(z) = &h.zone
            && sim.generator.zone_at(at.x as i32, at.y as i32).unwrap_or("none") != z
        {
            continue;
        }
        if !h.bands.is_empty() && !sim.generator.band_hint(at.y as i32).is_some_and(|b| h.bands.iter().any(|w| w == b)) {
            continue;
        }
        if h.place == Place::Cave && at.distance(p) < if h.emerges { EMERGE_NEAR.0 } else { OUT_OF_SIGHT * 0.7 } {
            continue;
        }
        if !show && !h.biomes.is_empty() && !sim.generator.biome_hint(at.x as i32).is_some_and(|b| h.biomes.iter().any(|w| w == b)) {
            continue;
        }
        if let Some((lo, hi)) = h.above {
            at.y += lo + unit(&mut rng) * (hi - lo);
        }
        // (A pack: several side by side, each on the ground there, no more
        // than its room for more.)
        let roll = unit(&mut rng);
        let count = if h.packs.is_empty() { h.group.map_or(1, |(lo, hi)| lo + (roll * (hi.saturating_sub(lo) + 1) as f32) as usize) } else { pack_size(&h.packs, depth, roll) }.clamp(1, most.saturating_sub(near).max(1));
        // (Together, a pack: the first its leader.)
        let pack = (count > 1).then(|| ((rng.next_u32() as u64) << 32) | sim.world.tick() & 0xFFFF_FFFF);
        for n in 0..count {
            let mut here = at + Vec2::X * (n as f32 * PACK_GAP);
            if n > 0 && matches!(h.place, Place::Surface | Place::Shore) {
                let x = here.x as i32;
                let top = here.y as i32 + 40;
                let Some(y) = (top - 80..=top).rev().find(|&y| kind_at(x, y).is_some_and(|(air, ph)| !air && matches!(ph.kind, Kind::Static | Kind::Powder)) && kind_at(x, y + 1).is_some_and(|(air, ph)| air || ph.kind == Kind::Plant)) else {
                    continue;
                };
                here.y = y as f32 + 1.0;
            }
            let emerges = h.emerges;
            spawn_creature(&mut commands, &h.kind, here, move |e| {
                if let Some(id) = pack {
                    e.insert(super::tactics::Pack { id, rank: n as u32 });
                }
                // (Up out of the ground, hidden and still till it's out.)
                if emerges {
                    e.insert((crate::creatures::body::emerge::Emerging::after(n as f32 * EMERGE_APART), crate::creatures::Dormant, Visibility::Hidden));
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Packs grow with depth: two at most near the top, five deep down, a
    /// deep one still often only a few, never more than five.
    #[test]
    fn packs_grow_with_depth_and_still_vary() {
        let packs = [(0.0, 2), (300.0, 3), (800.0, 4), (1600.0, 5)];
        let sizes = |depth: f32| (0..1000).map(|i| pack_size(&packs, depth, i as f32 / 1000.0)).collect::<Vec<_>>();
        let shallow = sizes(100.0);
        assert!(shallow.iter().all(|&n| (1..=2).contains(&n)) && shallow.contains(&1) && shallow.contains(&2), "1-2 near the top");
        let deep = sizes(3000.0);
        let share = |n: usize| deep.iter().filter(|&&m| m == n).count() as f32 / deep.len() as f32;
        assert!(deep.iter().all(|&n| (1..=5).contains(&n)), "never past five");
        assert!(share(1) > share(5) && share(5) > 0.1 && share(1) < 0.3, "deep: one {:.2}, five {:.2}", share(1), share(5));
        assert_eq!(pack_size(&[], 500.0, 0.5), 1, "no packs: one");
    }
}
