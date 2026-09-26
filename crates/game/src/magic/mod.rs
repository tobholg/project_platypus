//! Magic (DESIGN §7b): wands cast runes. A wand (an item, `Use::Cast`) holds
//! rune ids; `runes.rs` reads them as casts, and this carries them out:
//!
//! - holding a wand asks for a cast every tick (`CastRequest`); the wand
//!   keeps its own time (a delay between casts, a recharge after its last)
//!   and the caster pays in mana;
//! - bolts and orbs fly as `Spell`s through open cells and liquids, stop at
//!   solids and bodies (an orb bounces first), and land: their payloads go
//!   off there, through the same sim edits as everything else (a fireball's
//!   blast is a bomb's blast, smaller);
//! - a stream sprays burning cells (a flamethrower);
//! - lightning is the sky's lightning, from the wand: `WorldEdit::Zap` to up
//!   to N creatures near the aim (or the aim itself), a jagged bolt that
//!   burns what it passes and bursts where it ends.
//!
//! A `Trigger` cast fires what's after it from where it lands.

pub mod runes;
pub mod warp;
pub mod well;

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use platypus_sim::cell::flags;
use platypus_sim::rng::Rng;
use platypus_sim::{CellPos, Kind, Landing, MaterialId, Particle, World, WorldEdit};

use crate::actors::elements::{Coated, Coatings, Resist, catch_fire};
use crate::actors::player::LocalPlayer;
use crate::actors::{Health, Kinematics};
use crate::data::{Watched, data_path, load_ron};
use crate::hands::items::{ItemId, Items, Use};
use crate::light::LightSource;
use crate::vfx::{Halo, Sparks};
use well::Well;
use crate::world::{SimWorld, TICK_HZ, TickSet};
use runes::{Carrier, Cast, Payload, Runes, RunesFile};

pub struct MagicPlugin;

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// What a thrown thing falls at (cells/s²); spells fall at a share of it.
const GRAVITY: f32 = 900.0;
/// An orb falls at this share even without a `Gravity` rune.
const ORB_FALL: f32 = 0.3;
/// A spell can hit its own caster once it has flown this long (a bouncing
/// fireball can come back).
const SELF_SAFE: f32 = 0.3;
/// Lightning looks for creatures within this angle of the aim (radians)...
const LIGHTNING_CONE: f32 = 0.6;
/// ... or this close to where the cursor is (cells).
const LIGHTNING_NEAR_AIM: f32 = 30.0;
/// How far ahead a cast set off where something landed looks for its aim.
const TRIGGERED_REACH: f32 = 80.0;
/// A stream sets alight what stands in it (a chance a cast, in 255ths)
/// and scalds it this much.
const STREAM_CATCH: u8 = 70;
const STREAM_DAMAGE: f32 = 2.0;
/// ... and heats what it plays on this much a cast (°C, radius 3).
const STREAM_HEAT: i16 = 12;
/// A stream starts this far ahead of the hand (so it doesn't douse its
/// caster).
const STREAM_AHEAD: f32 = 6.0;
/// Meeting a liquid: an orb skips off it (at most this many times) when it
/// comes in shallower than this (vertical to horizontal speed) and faster
/// than this (cells/s)...
const ORB_SKIPS: u8 = 3;
const SKIP_SLOPE: f32 = 0.6;
const SKIP_SPEED: f32 = 110.0;
/// ... anything else goes in, keeping this share of its speed a cell and
/// burning its life this much faster, gone below this speed.
const WET_STEP: f32 = 0.9;
const WET_AGE: f32 = 3.0;
const FIZZLE: f32 = 50.0;

/// What casting costs. Refills `regen` a second.
#[derive(Component, Clone, Copy, Debug)]
pub struct Mana {
    pub cur: f32,
    pub max: f32,
    pub regen: f32,
}

impl Default for Mana {
    fn default() -> Self {
        Mana { cur: 100.0, max: 100.0, regen: 30.0 }
    }
}

/// Cast a wand, from `from` toward `toward` (held: every tick; the wand
/// keeps its own time).
#[derive(Message, Clone, Copy, Debug)]
pub struct CastRequest {
    pub caster: Entity,
    pub item: ItemId,
    pub from: Vec2,
    pub toward: Vec2,
    /// The other button (force: pull). Other spells ignore it.
    pub alt: bool,
}

/// Every rune (hot-reloaded), and each wand's runes read as casts.
#[derive(Resource)]
pub struct Spellbook {
    pub runes: Runes,
    wands: HashMap<ItemId, Arc<Vec<Arc<Cast>>>>,
    watch: Watched,
}

impl Spellbook {
    fn load() -> Self {
        let path = data_path("runes.ron");
        let runes = load_ron::<RunesFile>(&path).and_then(Runes::new).unwrap_or_else(|e| panic!("{e}"));
        Spellbook { runes, wands: HashMap::new(), watch: Watched::new(path) }
    }

    /// A wand's runes by name, and the mana of its first cast (with what it
    /// sets off), for its tooltip.
    pub fn describe(&self, ids: &[String]) -> (Vec<String>, f32) {
        let names = ids.iter().map(|id| self.runes.get(id).map_or(format!("?{id}"), |r| r.name.clone())).collect();
        let mana = runes::casts(&self.runes, ids).ok().and_then(|c| c.first().map(|c| c.total_mana())).unwrap_or(0.0);
        (names, mana)
    }

    /// A wand's casts, in order (read once, until the runes change).
    fn casts(&mut self, items: &Items, item: ItemId) -> Arc<Vec<Arc<Cast>>> {
        if let Some(c) = self.wands.get(&item) {
            return c.clone();
        }
        let casts = match &items.def(item).use_ {
            Use::Cast { runes, .. } => runes::casts(&self.runes, runes).unwrap_or_else(|e| {
                warn!("{}: {e}", items.def(item).id);
                Vec::new()
            }),
            _ => Vec::new(),
        };
        let casts = Arc::new(casts);
        self.wands.insert(item, casts.clone());
        casts
    }
}

/// Where a caster's wand is in its runes, how long until it can cast, and
/// the gravity well it's holding open.
#[derive(Default)]
struct WandState {
    next: usize,
    wait: f32,
    well: Option<Entity>,
}

#[derive(Resource, Default)]
struct Wands(HashMap<(Entity, ItemId), WandState>);

/// A cast going off this tick.
struct Fire {
    cast: Arc<Cast>,
    caster: Entity,
    from: Vec2,
    dir: Vec2,
    /// How far the aim is (lightning goes there when nothing's near it).
    reach: f32,
}

/// Casts to go off (set off by a wand, or by a spell landing: next tick).
#[derive(Resource, Default)]
struct Firing(Vec<Fire>);

/// A bolt or orb in flight.
#[derive(Component)]
pub struct Spell {
    cast: Arc<Cast>,
    caster: Entity,
    pos: Vec2,
    prev: Vec2,
    vel: Vec2,
    age: f32,
    life: f32,
    bounces: u8,
    /// Share of `GRAVITY` it falls at.
    fall: f32,
    /// In a liquid (dragged, burning out); times an orb can still skip off
    /// one.
    wet: bool,
    skips: u8,
}

type Hittable<'a> = (Entity, &'a mut Kinematics, &'a mut Health, Option<&'a Resist>, Option<&'a Coated>);

impl Plugin for MagicPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Spellbook::load())
            .init_resource::<Wands>()
            .init_resource::<Firing>()
            .add_message::<CastRequest>()
            .add_plugins(bevy::core_pipeline::fullscreen_material::FullscreenMaterialPlugin::<warp::Warp>::default())
            .add_systems(Update, (reload_runes, give_mana, place_spells, well::give_warp, well::show))
            .add_systems(FixedUpdate, (recharge, request, fire, fly, well::channel).chain().in_set(TickSet::Bodies).before(crate::actors::hurt::notice));
    }
}

fn reload_runes(mut book: ResMut<Spellbook>) {
    if !book.watch.changed() {
        return;
    }
    match load_ron::<RunesFile>(book.watch.path()).and_then(Runes::new) {
        Ok(runes) => {
            book.runes = runes;
            book.wands.clear();
            info!("runes reloaded");
        }
        Err(e) => warn!("runes not reloaded: {e}"),
    }
}

fn give_mana(mut commands: Commands, new: Query<Entity, (With<LocalPlayer>, Without<Mana>)>) {
    for e in &new {
        commands.entity(e).insert(Mana::default());
    }
}

/// Wands recharge; mana comes back, except to someone holding a field open
/// (a channelled spell runs it down).
fn recharge(mut wands: ResMut<Wands>, mut mana: Query<(Entity, &mut Mana)>, fields: Query<&Well>) {
    for w in wands.0.values_mut() {
        w.wait = (w.wait - DT).max(0.0);
    }
    for (e, mut m) in &mut mana {
        if fields.iter().any(|f| f.caster == e) {
            continue;
        }
        m.cur = (m.cur + m.regen * DT).min(m.max);
    }
}

/// A wand ready to cast casts its next cast, if the caster has the mana.
#[allow(clippy::too_many_arguments)]
fn request(
    mut commands: Commands,
    mut wells: Query<&mut Well>,
    mut requests: MessageReader<CastRequest>,
    items: Option<Res<Items>>,
    mut book: ResMut<Spellbook>,
    mut wands: ResMut<Wands>,
    mut firing: ResMut<Firing>,
    mut mana: Query<&mut Mana>,
) {
    let Some(items) = items else { return };
    for r in requests.read() {
        let Use::Cast { delay, recharge, .. } = &items.def(r.item).use_ else { continue };
        let casts = book.casts(&items, r.item);
        let w = wands.0.entry((r.caster, r.item)).or_default();
        if casts.is_empty() {
            continue;
        }
        let cast = casts[w.next % casts.len()].clone();
        let channelled = matches!(cast.carrier, Carrier::Well { .. } | Carrier::Force { .. });
        // (Only force has a use for the other button.)
        if r.alt && !matches!(cast.carrier, Carrier::Force { .. }) {
            continue;
        }
        // An open field stays open while it's held and paid for, a tick at
        // a time (whatever the wand's recharge).
        if channelled && let Some(e) = w.well {
            if let Ok(mut well) = wells.get_mut(e) {
                let drain = match cast.carrier {
                    Carrier::Well { drain, .. } | Carrier::Force { drain, .. } => drain,
                    _ => 0.0,
                };
                let paid = mana.get_mut(r.caster).map_or(true, |mut m| {
                    let ok = m.cur >= drain * DT;
                    if ok {
                        m.cur -= drain * DT;
                    }
                    ok
                });
                if paid {
                    well.feed(r.from, r.toward, r.alt);
                }
                continue;
            }
            w.well = None;
        }
        if w.wait > 0.0 {
            continue;
        }
        if let Ok(mut m) = mana.get_mut(r.caster) {
            let cost = cast.total_mana();
            if m.cur < cost {
                continue;
            }
            m.cur -= cost;
        }
        if channelled {
            w.well = well::spawn_field(&mut commands, cast.clone(), r.caster, r.from, r.toward, r.alt);
            w.wait = *recharge;
            continue;
        }
        w.next = (w.next + 1) % casts.len();
        w.wait = if w.next == 0 { *recharge } else { *delay };
        let aim = r.toward - r.from;
        firing.0.push(Fire { cast, caster: r.caster, from: r.from, dir: aim.normalize_or(Vec2::X), reach: aim.length() });
    }
}

#[allow(clippy::too_many_arguments)]
fn fire(
    mut commands: Commands,
    mut firing: ResMut<Firing>,
    mut sim: ResMut<SimWorld>,
    coatings: Res<Coatings>,
    halo: Option<Res<Halo>>,
    mut sparks: ResMut<Sparks>,
    mut bodies: Query<Hittable>,
) {
    let halo = halo.map(|h| h.0.clone());
    for f in std::mem::take(&mut firing.0) {
        let cast = f.cast.clone();
        match &cast.carrier {
            &Carrier::Bolt { speed, life } => spawn_spell(&mut commands, &f, speed, life, 0, 0.0, halo.clone()),
            &Carrier::Orb { speed, life, bounces } => spawn_spell(&mut commands, &f, speed, life, bounces, ORB_FALL, halo.clone()),
            Carrier::Stream { material, rate, speed, spread, burning } => {
                // (A stream's trail is what comes out of the wand with it.)
                for e in &cast.trails {
                    sparks.emit(e, e.count.round() as usize, f.from + f.dir * STREAM_AHEAD, f.dir, Vec2::ZERO);
                }
                stream(&mut sim.world, &f, material, *rate, *speed, *spread, *burning);
                let reach = speed * 0.25;
                // Where it plays on something, it heats it (wood catches,
                // ice melts, rock glows if you keep at it).
                if *burning {
                    let world = &sim.world;
                    let mats = world.materials();
                    let open = |p: Vec2| world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| c.is_air() || matches!(mats.phys(c.material).kind, Kind::Gas | Kind::Fire | Kind::Plant));
                    let tip = (STREAM_AHEAD as i32..reach as i32).map(|t| f.from + f.dir * t as f32).find(|&p| !open(p));
                    if let Some(tip) = tip {
                        sim.world.apply_edit(&WorldEdit::Heat { center: CellPos::from_world(tip.x, tip.y), radius: 3, amount: STREAM_HEAT });
                    }
                }
                // What stands in it is scalded, and may catch.
                let mut rng = Rng::seeded(&[sim.world.tick(), 0x57AE]);
                for (e, mut k, mut h, resist, coated) in &mut bodies {
                    let d = k.body.pos - f.from;
                    if e == f.caster || d.length() > reach || d.normalize_or_zero().dot(f.dir) < (spread * 1.5 + 0.1).cos() {
                        continue;
                    }
                    h.hp -= STREAM_DAMAGE;
                    k.body.vel += f.dir * 6.0;
                    if rng.chance(STREAM_CATCH) {
                        catch_fire(&mut commands, e, resist, coated, &coatings);
                    }
                }
            }
            // (Held open by `request`, run by `well::channel`.)
            Carrier::Well { .. } | Carrier::Force { .. } => {}
            // (Its hurt is the sim's: `elements::zapped`.)
            &Carrier::Lightning { range, targets } => {
                let mut ends = lightning_targets(&bodies, &f, range, targets as usize);
                if ends.is_empty() {
                    ends.push((f.from + f.dir * f.reach.clamp(12.0, range), None));
                }
                for (to, target) in ends {
                    let end = sim.world.zap(CellPos::from_world(f.from.x, f.from.y), CellPos::from_world(to.x, to.y));
                    let at = Vec2::new(end.x as f32 + 0.5, end.y as f32 + 0.5);
                    // (Walled off: it hit the wall, not them.)
                    let hit = target.filter(|_| at.distance(to) < 4.0);
                    let dir = (to - f.from).normalize_or(f.dir);
                    for e in &cast.bursts {
                        sparks.emit(e, e.count as usize, at, -dir, Vec2::ZERO);
                    }
                    land(&mut commands, &mut sim.world, &coatings, &mut bodies, &cast, at, hit, dir, false);
                    if let Some(then) = &cast.then {
                        firing.0.push(Fire { cast: then.clone(), caster: f.caster, from: at - dir * 2.0, dir, reach: TRIGGERED_REACH });
                    }
                }
            }
        }
    }
}

fn spawn_spell(commands: &mut Commands, f: &Fire, speed: f32, life: f32, bounces: u8, fall: f32, halo: Option<Handle<Image>>) {
    let c = &f.cast;
    let vel = f.dir * speed * c.speed_scale();
    let (r, g, b) = c.color;
    let color = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0];
    let (size, glow) = if matches!(c.carrier, Carrier::Orb { .. }) { (Vec2::splat(5.0), 2.4) } else { (Vec2::new(4.0, 1.5), 1.4) };
    // A hot, pale core in a soft halo of its colour.
    let core = Color::srgb(0.5 + color[0] * 0.5, 0.5 + color[1] * 0.5, 0.5 + color[2] * 0.5);
    let mut spell = commands.spawn((
        Name::new("Spell"),
        Spell {
            cast: c.clone(),
            caster: f.caster,
            pos: f.from,
            prev: f.from,
            vel,
            age: 0.0,
            life,
            bounces,
            fall: fall + c.gravity(),
            wet: false,
            skips: if matches!(c.carrier, Carrier::Orb { .. }) { ORB_SKIPS } else { 0 },
        },
        LightSource { color: color.map(|x| x * glow), flicker: 0.15 },
        Sprite::from_color(core, size),
        Transform::from_translation(f.from.extend(12.5)).with_rotation(Quat::from_rotation_z(vel.to_angle())),
    ));
    if let Some(halo) = halo {
        let across = size.max_element() * 3.5;
        spell.with_child((
            Sprite { image: halo, color: Color::srgba(color[0], color[1], color[2], 0.6), custom_size: Some(Vec2::splat(across)), ..default() },
            Transform::from_xyz(0.0, 0.0, -0.1),
        ));
    }
}

/// Spray a stream's cells, from a little ahead of the wand: flames that
/// become real fire where they stop (rising, flickering, lighting what they
/// touch), and one in six the burning material (a little lingers; more
/// floods back under the caster's feet).
fn stream(world: &mut World, f: &Fire, material: &str, rate: u32, speed: f32, spread: f32, burning: bool) {
    let mats = world.materials().clone();
    let Some(m) = mats.id(material) else { return };
    let fire = mats.fire();
    let mut rng = Rng::seeded(&[world.tick(), f.from.x.to_bits() as u64, f.from.y.to_bits() as u64]);
    let unit = |rng: &mut Rng| rng.next_u32() as f32 / u32::MAX as f32;
    let at = f.from + f.dir * STREAM_AHEAD;
    for i in 0..rate {
        let a = (unit(&mut rng) - 0.5) * 2.0 * spread;
        let v = Vec2::from_angle(a).rotate(f.dir) * speed * (0.75 + 0.5 * unit(&mut rng)) / TICK_HZ as f32;
        let p = if burning && i % 6 != 5 && fire != MaterialId::AIR {
            let flame = mats.spawn(fire, &mut rng);
            Particle { gravity: -0.05, ..Particle::new([at.x, at.y], [v.x, v.y], flame, 9 + rng.next_u8() as u16 / 24, Landing::Settle) }
        } else {
            let mut cell = mats.spawn(m, &mut rng);
            if burning {
                cell.flags |= flags::BURNING;
                cell.life = mats.phys(m).burn_time;
            }
            Particle { gravity: 0.35, ..Particle::new([at.x, at.y], [v.x, v.y], cell, 90, Landing::Settle) }
        };
        world.emit(p);
    }
}

/// Up to `n` creatures lightning goes for: within `range`, toward the aim
/// (or near where it points), nearest the line first.
fn lightning_targets(bodies: &Query<Hittable>, f: &Fire, range: f32, n: usize) -> Vec<(Vec2, Option<Entity>)> {
    let aim_at = f.from + f.dir * f.reach;
    let mut found: Vec<(f32, Vec2, Entity)> = bodies
        .iter()
        .filter(|(e, ..)| *e != f.caster)
        .filter_map(|(e, k, ..)| {
            let d = k.body.pos - f.from;
            let dist = d.length();
            if dist > range || dist < 1.0 {
                return None;
            }
            let cos = d.dot(f.dir) / dist;
            (cos > LIGHTNING_CONE.cos() || k.body.pos.distance(aim_at) < LIGHTNING_NEAR_AIM).then_some((dist * (2.0 - cos), k.body.pos, e))
        })
        .collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    found.into_iter().take(n).map(|(_, p, e)| (p, Some(e))).collect()
}

/// A cast's payloads, where it landed (`hit`: the body it hit).
#[allow(clippy::too_many_arguments)]
///
/// `doused`: it met water (or anything that puts fire out) burning: no fire,
/// and its heat flashes the water around it to steam.
#[allow(clippy::too_many_arguments)]
fn land(commands: &mut Commands, world: &mut World, coatings: &Coatings, bodies: &mut Query<Hittable>, cast: &Cast, at: Vec2, hit: Option<Entity>, dir: Vec2, doused: bool) {
    let center = CellPos::from_world(at.x, at.y);
    let mats = world.materials().clone();
    let mut rng = Rng::seeded(&[world.tick(), center.x as u64, center.y as u64, 0x57EA]);
    for p in &cast.payloads {
        match p {
            &Payload::Damage(d) => {
                if let Some(e) = hit
                    && let Ok((_, mut k, mut h, ..)) = bodies.get_mut(e)
                {
                    h.hp -= d;
                    let k = &mut *k;
                    k.loco.knock(&mut k.body, (dir + Vec2::new(0.0, 0.5)).normalize() * d * 5.0, 0.2);
                }
            }
            &Payload::Blast { radius, power } => {
                world.apply_edit(&WorldEdit::Explode { center, radius, power });
            }
            &Payload::Heat { radius, amount } => {
                world.apply_edit(&WorldEdit::Heat { center, radius, amount });
                // Hot into water: it flashes to steam. Cold: the water it
                // lands on or beside freezes (an ice patch to stand on).
                let r = radius + if amount < 0 { 3 } else { 0 };
                for dy in -r..=r {
                    for dx in -r..=r {
                        if dx * dx + dy * dy > r * r {
                            continue;
                        }
                        let q = CellPos::new(center.x + dx, center.y + dy);
                        let Some(c) = world.get(q) else { continue };
                        let ph = mats.phys(c.material);
                        if ph.kind != Kind::Liquid {
                            continue;
                        }
                        if amount < 0 && ph.below_into != MaterialId::AIR {
                            world.set(q, mats.spawn(ph.below_into, &mut rng));
                        } else if amount > 0 && doused && ph.above_into != MaterialId::AIR && rng.coin() {
                            let mut vapour = mats.spawn(ph.above_into, &mut rng);
                            vapour.heat = vapour.heat.max(amount / 3);
                            world.set(q, vapour);
                        }
                    }
                }
            }
            // (Doused, it lights nothing.)
            Payload::Ignite { .. } if doused => {}
            &Payload::Ignite { radius } => {
                world.apply_edit(&WorldEdit::Ignite { center, radius });
                for (e, k, _, resist, coated) in bodies.iter() {
                    if k.body.pos.distance(at) < radius as f32 + k.body.half.max_element() {
                        catch_fire(commands, e, resist, coated, coatings);
                    }
                }
            }
            Payload::Matter { material, cells } => {
                if let Some(m) = world.materials().id(material) {
                    world.splash([at.x, at.y], m, *cells as usize, 1.2);
                }
            }
        }
    }
}

/// Spells fly through open cells and liquids a cell at a time, and land on
/// the first solid or body (an orb bounces off its first solids), or where
/// they are when their time is up.
fn fly(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    coatings: Res<Coatings>,
    mut firing: ResMut<Firing>,
    mut spells: Query<(Entity, &mut Spell)>,
    mut bodies: Query<Hittable>,
    mut sparks: ResMut<Sparks>,
) {
    let mut landed = Vec::new();
    let mut trail = Vec::new();
    let mut shed = Vec::new();
    // Where spells met a liquid: splash so many cells of it up (and hiss
    // steam, if it doused fire).
    let mut surface: Vec<(Vec2, usize, bool)> = Vec::new();
    {
        let world = &sim.world;
        let mats = world.materials();
        let solid = |p: Vec2| world.get(CellPos::from_world(p.x, p.y)).is_none_or(|c| matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder));
        let liquid = |p: Vec2| world.get(CellPos::from_world(p.x, p.y)).filter(|c| mats.phys(c.material).kind == Kind::Liquid);
        for (e, mut s) in &mut spells {
            let s = &mut *s;
            s.prev = s.pos;
            s.age += DT;
            s.vel.y -= GRAVITY * s.fall * DT;
            let travel = s.vel * DT;
            let steps = travel.length().ceil().max(1.0) as i32;
            let step = travel / steps as f32;
            let mut end: Option<(Option<Entity>, Vec2)> = None;
            let mut doused = false;
            for i in 0..steps {
                let next = s.pos + step;
                if solid(next) {
                    let n = Vec2::new(
                        if solid(Vec2::new(next.x, s.pos.y)) { -step.x.signum() } else { 0.0 },
                        if solid(Vec2::new(s.pos.x, next.y)) { -step.y.signum() } else { 0.0 },
                    );
                    let n = if n == Vec2::ZERO { -step.normalize_or_zero() } else { n.normalize() };
                    if s.bounces > 0 {
                        s.bounces -= 1;
                        if let Some((material, cells, burning)) = s.cast.shed() {
                            shed.push((s.pos, material.to_string(), cells, burning));
                        }
                        s.vel = (s.vel - 2.0 * s.vel.dot(n) * n) * 0.7;
                        break;
                    }
                    end = Some((None, n));
                    break;
                }
                // Meeting a liquid's surface.
                match (liquid(next), s.wet) {
                    (Some(c), false) => {
                        let oil = mats.phys(c.material).flammability > 0;
                        let fiery = s.cast.fiery();
                        let douses = fiery && !oil;
                        // A fast orb coming in shallow skips off it, like a
                        // stone (steaming, if it's burning).
                        // (Fire meeting oil lights it instead.)
                        if s.skips > 0 && !(fiery && oil) && s.vel.y < 0.0 && s.vel.y.abs() < s.vel.x.abs() * SKIP_SLOPE && s.vel.length() > SKIP_SPEED {
                            s.skips -= 1;
                            debug!("spell skipped off a liquid at {next:?}");
                            s.vel = Vec2::new(s.vel.x * 0.85, s.vel.y.abs() * 0.5);
                            surface.push((next, 10, douses));
                            break;
                        }
                        // Fire into water is doused: it goes off at the
                        // surface in a burst of steam. Fire onto oil lights
                        // it; cold freezes it: both at the surface.
                        if fiery || s.cast.frosty() {
                            debug!("spell met a liquid at {next:?}: doused {douses}, oil {oil}");
                            doused = douses;
                            if douses {
                                surface.push((next, 16, true));
                            }
                            end = Some((None, Vec2::Y));
                            break;
                        }
                        // Anything else plunges in.
                        s.wet = true;
                        surface.push((next, 6, false));
                    }
                    (None, true) => s.wet = false,
                    _ => {}
                }
                // Through a liquid each cell slows it (a bolt dies within
                // ~20 cells).
                if s.wet {
                    s.vel *= WET_STEP;
                }
                s.pos = next;
                for e in &s.cast.trails {
                    sparks.trail(e, 1.0, s.pos, -s.vel, s.vel * 0.15);
                }
                let (caster, age) = (s.caster, s.age);
                let inside = |k: &Kinematics| ((next - k.body.pos).abs() - k.body.half).max_element() < 1.0;
                if let Some((hit, ..)) = bodies.iter().find(|(b, k, ..)| (*b != caster || age > SELF_SAFE) && inside(k)) {
                    end = Some((Some(hit), Vec2::ZERO));
                    break;
                }
                if let Some((material, burning)) = s.cast.trail()
                    && (i + world.tick() as i32) % 2 == 0
                {
                    trail.push((s.pos, material.to_string(), burning));
                }
            }
            // In a liquid: burning out fast, fizzling when slow.
            if s.wet {
                s.age += DT * WET_AGE;
                if end.is_none() && s.vel.length() < FIZZLE {
                    end = Some((None, Vec2::ZERO));
                }
            }
            if end.is_none() && s.age >= s.life {
                end = Some((None, Vec2::ZERO));
            }
            if let Some((hit, n)) = end {
                landed.push((e, s.cast.clone(), s.caster, s.pos, hit, s.vel.normalize_or(Vec2::X), n, doused));
            }
        }
    }
    let world = &mut sim.world;
    let mats = world.materials().clone();
    let mut rng = Rng::seeded(&[world.tick(), 0x7A11]);
    for (at, material, burning) in trail {
        let Some(m) = mats.id(&material) else { continue };
        let mut cell = mats.spawn(m, &mut rng);
        if burning {
            cell.flags |= flags::BURNING;
            cell.life = mats.phys(m).burn_time;
        }
        let vel = [(rng.next_u8() as f32 / 255.0 - 0.5) * 0.3, 0.1 + rng.next_u8() as f32 / 255.0 * 0.2];
        world.emit(Particle { gravity: -0.1, ..Particle::new([at.x, at.y], vel, cell, 10 + rng.next_u8() as u16 / 32, Landing::Ember) });
    }
    for (at, material, cells, burning) in shed {
        let Some(m) = mats.id(&material) else { continue };
        for _ in 0..cells {
            let mut cell = mats.spawn(m, &mut rng);
            if burning {
                cell.flags |= flags::BURNING;
                cell.life = mats.phys(m).burn_time;
            }
            let a = rng.next_u32() as f32 / u32::MAX as f32 * std::f32::consts::PI;
            let s = 0.4 + rng.next_u8() as f32 / 255.0 * 0.8;
            world.emit(Particle::new([at.x, at.y], [a.cos() * s, a.sin() * s], cell, 90, Landing::Settle));
        }
    }
    for (at, n, steam) in surface {
        splash_surface(world, at, n, &mut rng);
        if steam {
            sparks.emit(&HISS, HISS.count as usize, at + Vec2::Y * 2.0, Vec2::Y, Vec2::ZERO);
        }
    }
    for (e, cast, caster, at, hit, dir, n, doused) in landed {
        commands.entity(e).despawn();
        debug!("spell landed at {at:?} (hit {hit:?}, doused {doused})");
        // Off the surface it hit, or back the way it came.
        let off = if n == Vec2::ZERO { -dir } else { n };
        // (Doused, its fire bursts are steam instead.)
        if !doused {
            for b in &cast.bursts {
                sparks.emit(b, b.count as usize, at, off, Vec2::ZERO);
            }
        }
        land(&mut commands, world, &coatings, &mut bodies, &cast, at, hit, dir, doused);
        if let Some(then) = &cast.then {
            let dir = if n == Vec2::ZERO { dir } else { dir - 2.0 * dir.dot(n) * n };
            firing.0.push(Fire { cast: then.clone(), caster, from: at, dir, reach: TRIGGERED_REACH });
        }
    }
}

/// Throw up to `n` cells of a liquid's surface where a spell met it (real
/// cells: nothing made, nothing lost).
fn splash_surface(world: &mut World, at: Vec2, n: usize, rng: &mut Rng) {
    let mats = world.materials().clone();
    let unit = |rng: &mut Rng| rng.next_u32() as f32 / u32::MAX as f32;
    for _ in 0..n * 2 {
        let q = CellPos::from_world(at.x + (unit(rng) - 0.5) * 6.0, at.y - unit(rng) * 3.0);
        let Some(c) = world.get(q).filter(|c| mats.phys(c.material).kind == Kind::Liquid) else { continue };
        let Some(c) = world.pluck(q).map(|_| c) else { continue };
        let v = [(unit(rng) - 0.5) * 1.6, 1.0 + unit(rng) * 1.6];
        world.emit(Particle::new([q.x as f32 + 0.5, q.y as f32 + 1.5], v, c, 150, Landing::Settle));
    }
}

/// Steam hissing off doused fire (visual).
static HISS: std::sync::LazyLock<runes::Emitter> = std::sync::LazyLock::new(|| runes::Emitter {
    count: 34.0,
    life: (0.5, 1.4),
    colors: vec![(250, 250, 255), (210, 215, 225), (150, 155, 165)],
    speed: 45.0,
    spread: 1.2,
    gravity: -40.0,
    drag: 1.5,
    size: 2.0,
    jitter: 6.0,
    glow: false,
});

/// Draw spells between the last two ticks, pointing where they go.
fn place_spells(time: Res<Time<Fixed>>, mut q: Query<(&Spell, &mut Transform)>) {
    let a = time.overstep_fraction();
    for (s, mut tf) in &mut q {
        let p = s.prev.lerp(s.pos, a);
        tf.translation.x = p.x;
        tf.translation.y = p.y;
        tf.rotation = Quat::from_rotation_z(s.vel.to_angle());
    }
}
