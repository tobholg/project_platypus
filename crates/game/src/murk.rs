//! The murk (the space arc): a mass of dark motes round a hidden heart,
//! adrift in the void. It has no shape of its own. Its motes are pulled
//! toward a lumpy, breathing blob round the heart and stirred by a flow
//! that never settles, so it writhes; out of it reach a few tendrils.
//!
//! - **It hunts.** Near you, it keeps a little way off and circles. It
//!   tears clumps of itself off and throws them at you: each one a small
//!   swarm that cuts through you, loops back for another pass, and drifts
//!   home to merge into the mass again (it shrinks as it throws, and swells
//!   as they come back). Close in, its tendrils wind back and lash.
//! - **It's made of what it's lost.** Shot, the motes a bullet meets are
//!   gone and the bullet stops in it; a blast scatters it, and it takes a
//!   few seconds to pull itself together. A tendril shot through is cut
//!   off there (what's past the cut drifts off and fades) and grows back
//!   from the mass, as everything it's lost does, slowly, out of the heart.
//! - **It eats rubble.** Loose rock and dust it drifts through it takes
//!   in, to grow back faster, and past its size (to half again as big):
//!   where you've been mining, it's worse.
//! - **Light hurts it.** It shies from bright light (a sunstone's, a
//!   comet's, your rocket plume) and what's in the bright of it burns
//!   away; its heart flees from it.
//! - **Its heart** is the only thing that kills it, in a hollow in the
//!   middle of it: a shot only gets there through a hole in the mass.
//!
//! Not in the cell grid: a thousand motes moving as one are cheaper as
//! points (as glowfish are, `shoals.rs`), drawn a cell at a time on a
//! canvas, reading the grid for rock (they stream round it) and rubble.
//! Spells read where it's thick (`Murks::thick`) and stop there, telling it
//! (`Murks::struck`).

use std::f32::consts::TAU;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use platypus_sim::rng::hash;
use platypus_sim::cell::flags;
use platypus_sim::{CellPos, Kind};

use crate::creatures::player::LocalPlayer;
use crate::creatures::{Harm, Health, Kinematics};
use crate::light::LightSource;
use crate::magic::runes::Emitter;
use crate::world::{SimWorld, TICK_HZ, TickSet};

pub struct MurkPlugin;

impl Plugin for MurkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Murks>()
            .init_resource::<MurkCanvas>()
            .add_systems(FixedUpdate, (come_and_go, live).chain().in_set(TickSet::Bodies).after(crate::creatures::move_creatures))
            .add_systems(PostUpdate, show.before(bevy::transform::TransformSystems::Propagate));
    }
}

/// Motes in a whole murk, in each tendril of its `TENDRILS` (the rest are
/// its mass); it grows to `EAT_MOST` times as many eating rubble.
const MOTES: usize = 1800;
const TENDRILS: usize = 4;
const TENDRIL_MOTES: usize = 70;
const EAT_MOST: f32 = 1.5;
/// A tendril: so many links, this long whole (cells).
const SEGS: usize = 10;
const REACH: f32 = 66.0;
/// Its heart: this big (cells), in a hollow this big; its mass this thick
/// (motes a cell); its heart's health.
const CORE_R: f32 = 4.5;
const HOLLOW: f32 = 6.5;
const DENSITY: f32 = 0.6;
const HP: f32 = 650.0;
/// Motes a second it grows back (from the heart), and rubble cells a tick
/// it may eat at most.
const REGROW: f32 = 24.0;
const EAT_TICK: usize = 3;
/// It hunts what's this near (cells), keeping this far off, circling;
/// adrift it goes this fast, hunting this fast (cells/s), turning this
/// hard (cells/s²).
const AGGRO: f32 = 460.0;
const STANDOFF: f32 = 96.0;
const SPEED: f32 = 22.0;
const HUNT_SPEED: f32 = 52.0;
const ACCEL: f32 = 70.0;
/// Clumps: so many motes (from, to), one this often (s, from, to), so many
/// out at most, thrown at you from this near; flying this fast, turning
/// this hard; two passes, then home; gone home at most after this long.
const CLUMP: (usize, usize) = (34, 72);
const CLUMP_EVERY: (f32, f32) = (2.4, 4.2);
const CLUMPS_MOST: usize = 3;
const THROW: f32 = 340.0;
const CLUMP_SPEED: f32 = 185.0;
const CLUMP_TURN: f32 = 430.0;
const PASSES: u8 = 2;
const CLUMP_SECS: f32 = 7.0;
/// What a clump's pass through you does (this, and this for each mote in
/// it); a lash; inside its mass, this a second. (Before the difficulty's
/// share: Wayfarer takes 2.4 times these.)
const CLUMP_HIT: f32 = 2.0;
const CLUMP_HIT_PER: f32 = 0.04;
const LASH_HIT: f32 = 6.0;
const LASH_KNOCK: f32 = 210.0;
const SMOTHER: f32 = 5.0;
/// A lash: winding back (s), striking (s), resting after (s); another
/// tendril's no sooner than this after (s).
const WIND: f32 = 0.38;
const STRIKE: f32 = 0.42;
const LASH_REST: f32 = 1.9;
const LASH_APART: f32 = 0.8;
/// Shot: a bullet takes the motes this near where it stops (cells, and so
/// much more for each point of its damage), at most so many for each; a
/// blast takes all within this share of its radius and scatters those out
/// to this many times it.
const SHOT_R: (f32, f32) = (2.5, 1.0 / 8.0);
const SHOT_MOST: f32 = 1.2;
const BLAST_TAKES: f32 = 0.8;
const BLAST_SCATTERS: f32 = 2.4;
/// Light: a light of strength 1 (its colour summed) drives motes back this
/// far (cells; as the root of its strength), burning those in the bright
/// middle of it (this share of it) this often (a share a second); a
/// rocket plume this long, burning this near it.
const LIGHT_R: f32 = 15.0;
const LIGHT_BURNS: f32 = 0.4;
const BURN: f32 = 2.5;
const PLUME: f32 = 30.0;
const PLUME_R: f32 = 4.5;
/// It turns up in space (`sky::space_at` this far up) about this often (s)
/// when there isn't one, past the view's edge (this far past it, cells),
/// and goes when it's this far off.
const SPACE_FROM: f32 = 0.5;
const EVERY: f32 = 150.0;
const BEYOND: (f32, f32) = (90.0, 220.0);
const GONE: f32 = 1500.0;
/// Where spells read it: motes counted in squares this big; a square with
/// this many is thick (a shot stops there).
const BIN: f32 = 2.0;
const THICK: u16 = 2;

fn unit(seed: &[u64]) -> f32 {
    (hash(seed) % 100_000) as f32 / 100_000.0
}

fn bin(p: Vec2) -> IVec2 {
    (p / BIN).floor().as_ivec2()
}

/// Rock (or anything solid) at `p`; unloaded is open space.
fn solid(sim: &SimWorld, p: Vec2) -> bool {
    let mats = sim.world.materials();
    sim.world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| !c.is_air() && matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Role {
    /// Its mass.
    Body,
    /// One of its tendrils.
    Tendril(u8),
    /// A clump thrown off it.
    Clump(u16),
    /// Cut off, scattered in death: drifting, fading (gone at `life` 0).
    Loose,
}

/// A mote. `a`, `b`: where it keeps in what it's part of (the mass: its
/// angle and how far out, a share; a tendril: how far along, how far
/// across; a clump: as the mass).
#[derive(Clone, Copy, Debug)]
struct Mote {
    pos: Vec2,
    vel: Vec2,
    role: Role,
    a: f32,
    b: f32,
    /// Scattered: drifting free this long yet.
    stun: f32,
    life: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Lash {
    Idle,
    Wind(f32),
    Strike(f32),
}

struct Tendril {
    pts: [Vec2; SEGS + 1],
    angle: f32,
    lash: Lash,
    rest: f32,
    struck: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Run {
    Out,
    Through(f32),
    Home,
}

struct Clump {
    id: u16,
    pos: Vec2,
    vel: Vec2,
    run: Run,
    passes: u8,
    age: f32,
    closest: f32,
    struck: bool,
}

/// An eye opening in the mass: where (from the heart), how long it's been
/// open, how long it stays.
struct Eye {
    off: Vec2,
    age: f32,
    life: f32,
}

pub struct Murk {
    core: Vec2,
    vel: Vec2,
    hp: f32,
    motes: Vec<Mote>,
    tendrils: Vec<Tendril>,
    clumps: Vec<Clump>,
    eyes: Vec<Eye>,
    /// Motes owed (growing back), and the most it holds now.
    owed: f32,
    most: usize,
    next_clump: f32,
    clump_ids: u16,
    heading: f32,
    hunting: bool,
    /// Fleeing a light, from where; just hurt (flashing, its eyes wide).
    scared: f32,
    scared_from: Vec2,
    hurt: f32,
    /// Dead this long (motes scattering, fading).
    dying: Option<f32>,
    light: Entity,
    seed: u64,
    whisper: f32,
    age: f32,
    /// Till another tendril may lash (one at a time, near enough).
    lashed: f32,
}

/// What a spell did meeting a murk: where it stopped, which way it was
/// going, its damage, its blast's radius.
#[derive(Clone, Copy, Debug)]
pub struct MurkHit {
    pub at: Vec2,
    pub dir: Vec2,
    pub damage: f32,
    pub blast: f32,
}

#[derive(Resource, Default)]
pub struct Murks {
    list: Vec<Murk>,
    /// Seconds to the next look for a spot for one.
    next: f32,
    /// Motes a square (`BIN`), for spells; what's met it since.
    field: HashMap<IVec2, u16>,
    hits: Vec<MurkHit>,
    /// Where shots struck lately, and when (tick).
    lately: Vec<(Vec2, u64)>,
}

impl Murks {
    /// Thick with motes here, or a heart: a shot stops in it.
    pub fn thick(&self, p: Vec2) -> bool {
        !self.list.is_empty() && (self.field.get(&bin(p)).is_some_and(|n| *n >= THICK) || self.list.iter().any(|m| m.dying.is_none() && m.core.distance(p) < CORE_R))
    }

    /// A spell met one (`thick`): what it does is done next tick.
    pub fn struck(&mut self, hit: MurkHit) {
        self.hits.push(hit);
    }

    /// Each: its heart, its health, its motes (in it, not cut off), its
    /// clumps out (for the scenarios).
    pub fn about(&self) -> Vec<(Vec2, f32, usize, usize)> {
        self.list.iter().map(|m| (m.core, m.hp, m.motes.iter().filter(|o| o.role != Role::Loose).count(), m.clumps.len())).collect()
    }

    /// One at `at`, now.
    pub fn summon(&mut self, commands: &mut Commands, at: Vec2, seed: u64) {
        let light = commands.spawn((Name::new("Murk heart"), MurkLight, LightSource { color: HEART_LIGHT, flicker: 0.0 }, Transform::from_translation(at.extend(0.0)))).id();
        self.list.push(Murk::new(at, seed, light));
    }
}

/// Its heart's faint light.
#[derive(Component)]
struct MurkLight;

const HEART_LIGHT: [f32; 3] = [0.22, 0.05, 0.26];

/// Its mass's radius with so many motes in it.
fn body_radius(n: usize) -> f32 {
    (HOLLOW * HOLLOW + n as f32 / (std::f32::consts::PI * DENSITY)).sqrt()
}

impl Murk {
    fn new(at: Vec2, seed: u64, light: Entity) -> Murk {
        let tendrils = (0..TENDRILS)
            .map(|i| {
                let angle = i as f32 / TENDRILS as f32 * TAU + unit(&[seed, i as u64, 0x7E]) * 0.8;
                Tendril { pts: [at; SEGS + 1], angle, lash: Lash::Idle, rest: unit(&[seed, i as u64, 0x7F]) * LASH_REST, struck: false }
            })
            .collect();
        let mut m = Murk {
            core: at,
            vel: Vec2::ZERO,
            hp: HP,
            motes: Vec::with_capacity(MOTES),
            tendrils,
            clumps: Vec::new(),
            eyes: Vec::new(),
            owed: 0.0,
            most: MOTES,
            next_clump: CLUMP_EVERY.1,
            clump_ids: 0,
            heading: unit(&[seed, 0x4E]) * TAU,
            hunting: false,
            scared: 0.0,
            scared_from: at,
            hurt: 0.0,
            dying: None,
            light,
            seed,
            whisper: 0.0,
            age: 0.0,
            lashed: 0.0,
        };
        let r = body_radius(MOTES - TENDRILS * TENDRIL_MOTES);
        for i in 0..MOTES {
            let role = if i < TENDRILS * TENDRIL_MOTES { Role::Tendril((i / TENDRIL_MOTES) as u8) } else { Role::Body };
            let mut o = m.mote(role, i as u64);
            o.pos = at + Vec2::from_angle(unit(&[seed, i as u64, 1]) * TAU) * unit(&[seed, i as u64, 2]).sqrt() * r;
            m.motes.push(o);
        }
        m
    }

    /// A new mote, at the heart, for `role`.
    fn mote(&self, role: Role, salt: u64) -> Mote {
        let u = |k: u64| unit(&[self.seed, salt, k, (self.age * 1000.0) as u64]);
        let (a, b) = match role {
            Role::Tendril(_) => (u(3), u(4) * 2.0 - 1.0),
            // (One in six of its mass's out past its edge: its fuzz.)
            Role::Body if u(8) < 0.16 => (u(3) * TAU, 1.0 + 0.4 * u(4)),
            _ => (u(3) * TAU, u(4).sqrt()),
        };
        let pos = self.core + Vec2::from_angle(u(5) * TAU) * CORE_R * u(6);
        Mote { pos, vel: Vec2::from_angle(u(7) * TAU) * 20.0, role, a, b, stun: 0.0, life: 0.0 }
    }
}

/// Murks turn up in space round the player now and then, out of sight
/// (`PLATYPUS_MURK=near`: one at once, close by, to look at), and go when
/// they're far off.
fn come_and_go(mut commands: Commands, sim: Res<SimWorld>, mut murks: ResMut<Murks>, player: Query<&Kinematics, With<LocalPlayer>>, camera: Query<(&GlobalTransform, &crate::world::ChunkLoader), With<crate::camera::MainCamera>>, mut forced: Local<bool>) {
    let Ok(me) = player.single() else { return };
    let me = me.body.pos;
    let dt = 1.0 / TICK_HZ as f32;
    let Murks { list, .. } = &mut *murks;
    list.retain(|m| {
        let keep = m.core.distance(me) < GONE && m.dying.is_none_or(|d| d < 3.0);
        if !keep {
            commands.entity(m.light).despawn();
        }
        keep
    });
    let tick = sim.world.tick();
    if !*forced && std::env::var("PLATYPUS_MURK").is_ok_and(|v| v == "near") && tick > 60 {
        *forced = true;
        murks.summon(&mut commands, me + Vec2::new(150.0, 30.0), tick);
        return;
    }
    if !sim.generator.wild() || crate::sky::space_at(&sim, me.y) < SPACE_FROM || !murks.list.is_empty() {
        return;
    }
    // (Not the moment you get there: a while first.)
    if murks.next == 0.0 {
        murks.next = EVERY * 0.5;
    }
    murks.next -= dt;
    if murks.next > 0.0 {
        return;
    }
    murks.next = EVERY * (0.6 + 0.8 * unit(&[tick, 0x3E]));
    let (eye, half) = camera.single().map_or((me, Vec2::new(250.0, 150.0)), |(t, l)| (t.translation().truncate(), l.half_extent));
    let r = body_radius(MOTES);
    for k in 0..8u64 {
        let dir = Vec2::from_angle(unit(&[tick, k, 0x3F]) * TAU);
        let edge = (half.x / dir.x.abs().max(1e-3)).min(half.y / dir.y.abs().max(1e-3));
        let at = eye + dir * (edge + BEYOND.0 + (BEYOND.1 - BEYOND.0) * unit(&[tick, k, 0x40]));
        // (Room for it: nothing solid across it.)
        let open = (-2..=2).all(|y| (-2..=2).all(|x| !solid(&sim, at + Vec2::new(x as f32, y as f32) * r * 0.5)));
        if open {
            info!("murk: one turned up at ({:.0}, {:.0})", at.x, at.y);
            murks.summon(&mut commands, at, tick ^ k);
            return;
        }
    }
}

/// A clump's pass, a lash: harm, a knock.
fn strike(k: &mut Kinematics, h: &mut Health, amount: f32, push: Vec2, how: &'static str) {
    debug!("murk: {how}: {amount:.1}");
    h.harm_by(amount, Harm::Void, || crate::death::Cause::new("The murk", how));
    let knock = k.body.vel + push;
    let k = &mut *k;
    k.loco.knock(&mut k.body, knock, 0.3);
}

/// Where along a tendril (`s`, 0 at the root to 1 at the tip): the point,
/// the way it runs there.
fn along(pts: &[Vec2; SEGS + 1], s: f32) -> (Vec2, Vec2) {
    let f = (s.clamp(0.0, 1.0) * SEGS as f32).min(SEGS as f32 - 1e-3);
    let i = f as usize;
    let d = pts[i + 1] - pts[i];
    (pts[i] + d * f.fract(), d.normalize_or(Vec2::X))
}

/// The flow that stirs it, never the same: a slow swirl across it.
fn flow(p: Vec2, t: f32) -> Vec2 {
    Vec2::new((p.y * 0.17 + t * 1.3).sin() + (p.x * 0.05 - t * 0.7 + p.y * 0.03).sin(), (p.x * 0.15 - t * 1.1).cos() + (p.y * 0.06 + t * 0.6).cos())
}

/// Its mass's edge, that way (`a`), now: lumpy, breathing.
fn edge(r: f32, a: f32, t: f32, seed: f32) -> f32 {
    r * (1.0 + 0.2 * (3.0 * a + 0.9 * t + seed).sin() + 0.12 * (5.0 * a - 1.4 * t + seed * 2.0).sin() + 0.07 * (9.0 * a + 2.1 * t).sin())
}

/// Torn motes: dark violet flecks.
static TEAR: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 6.0,
    life: (0.2, 0.6),
    colors: vec![(70, 40, 120), (40, 22, 70), (150, 100, 220)],
    speed: 60.0,
    spread: 1.2,
    gravity: 0.0,
    drag: 3.0,
    size: 1.0,
    jitter: 0.5,
    glow: false,
});

/// Burnt by light: pale sparks going out.
static BURNT: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.15, 0.4),
    colors: vec![(220, 190, 255), (160, 110, 230)],
    speed: 30.0,
    spread: 3.0,
    gravity: 0.0,
    drag: 4.0,
    size: 1.0,
    jitter: 0.5,
    glow: true,
});

/// Each murk's life, a tick: what struck it, its heart's way, its tendrils,
/// its clumps, growing back, its motes, light, what it does to you, its
/// eyes, its death.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn live(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    mut murks: ResMut<Murks>,
    mut player: Query<(&mut Kinematics, &mut Health), With<LocalPlayer>>,
    lights: Query<(&LightSource, &GlobalTransform), (Without<MurkLight>, Without<crate::magic::Spell>)>,
    mut hearts: Query<(&mut Transform, &mut LightSource), With<MurkLight>>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut booms: MessageReader<crate::fx::Explosion>,
    items: Option<Res<crate::hands::items::Items>>,
) {
    let mut hits = std::mem::take(&mut murks.hits);
    // (Blasts: a bomb's, a meteor's. Not a shot's that struck it: its blast
    // is one of these too, a tick or two after.)
    let tick = sim.world.tick();
    murks.lately.retain(|(_, at)| tick.saturating_sub(*at) < 4);
    murks.lately.extend(hits.iter().map(|h| (h.at, tick)));
    let lately = &murks.lately;
    let blasts: Vec<MurkHit> = booms.read().filter(|b| lately.iter().all(|(s, _)| s.distance(b.at) > 6.0)).map(|b| MurkHit { at: b.at, dir: Vec2::ZERO, damage: b.power * 0.25, blast: b.radius }).collect();
    hits.extend(blasts);
    if murks.list.is_empty() {
        murks.field.clear();
        return;
    }
    let dt = 1.0 / TICK_HZ as f32;
    let t = tick as f32 * dt;
    let shine: Vec<(Vec2, f32)> = lights.iter().map(|(l, g)| (g.translation().truncate(), l.color.iter().sum::<f32>())).filter(|(_, s)| *s > 0.4).collect();
    let mut me = player.single_mut().ok();
    let full_body = MOTES - TENDRILS * TENDRIL_MOTES;
    let mut eaten: Vec<CellPos> = Vec::new();
    for m in &mut murks.list {
        m.age += dt;
        m.hurt = (m.hurt - dt * 2.5).max(0.0);
        m.scared = (m.scared - dt).max(0.0);
        m.lashed = (m.lashed - dt).max(0.0);
        let you = me.as_ref().map(|(k, _)| (k.body.pos, k.body.vel, k.body.half, k.loco.rocketing(), k.loco.thrust()));
        // What struck it.
        if m.dying.is_none() {
            for h in &hits {
                take(m, h, &mut sparks, &mut sounds, tick);
            }
        }
        // Dead: scattering, fading; something left where its heart was.
        if m.hp <= 0.0 && m.dying.is_none() {
            m.dying = Some(0.0);
            info!("murk: its heart's out at ({:.0}, {:.0})", m.core.x, m.core.y);
            let core = m.core;
            for (i, o) in m.motes.iter_mut().enumerate() {
                let out = (o.pos - core).normalize_or(Vec2::from_angle(i as f32));
                o.role = Role::Loose;
                o.vel += out * (40.0 + 90.0 * unit(&[m.seed, i as u64, 0xDE]));
                o.life = 1.0 + 1.6 * unit(&[m.seed, i as u64, 0xDF]);
            }
            m.clumps.clear();
            m.eyes.clear();
            sparks.emit(&TEAR, 80, core, Vec2::Y, Vec2::ZERO);
            sounds.write(crate::sound::PlaySound::at("murk_death", core));
            if let Some(items) = &items
                && let Some(stone) = items.id("sunstone")
            {
                let n = 2 + (hash(&[m.seed, 0x5A]) % 3) as u32;
                crate::hands::spawn_drop(&mut commands, items, core, crate::hands::items::Stack::new(stone, n));
            }
        }
        if let Some(d) = &mut m.dying {
            *d += dt;
            for o in &mut m.motes {
                o.vel *= 1.0 - 0.8 * dt;
                o.pos += o.vel * dt;
                o.life -= dt;
            }
            m.motes.retain(|o| o.life > 0.0);
            if let Ok((_, mut l)) = hearts.get_mut(m.light) {
                l.color = HEART_LIGHT.map(|c| c * (1.0 - *d / 2.0).max(0.0));
            }
            continue;
        }
        // Counts: in its mass, in each tendril, in each clump.
        let mut in_body = 0usize;
        let mut in_tendril = [0usize; TENDRILS];
        let mut in_clump: HashMap<u16, usize> = HashMap::default();
        for o in &m.motes {
            match o.role {
                Role::Body => in_body += 1,
                Role::Tendril(i) => in_tendril[i as usize] += 1,
                Role::Clump(id) => *in_clump.entry(id).or_default() += 1,
                Role::Loose => {}
            }
        }
        // Its mass's size: what's in it (and what's out in clumps: it keeps
        // their room).
        let r_body = body_radius(in_body + in_clump.values().sum::<usize>() / 2);
        // Its heart's way: adrift, or circling you a way off; away from a
        // light; round rock.
        let dist = you.map_or(f32::MAX, |y| y.0.distance(m.core));
        m.hunting = dist < AGGRO;
        let want = if m.scared > 0.0 {
            (m.core - m.scared_from).normalize_or(Vec2::X) * HUNT_SPEED * 1.4
        } else if let (true, Some((p, ..))) = (m.hunting, you) {
            let off = (m.core - p).normalize_or(Vec2::X);
            let around = if m.seed % 2 == 0 { 0.45 } else { -0.45 };
            let to = p + Vec2::from_angle(off.to_angle() + around) * STANDOFF - m.core;
            to.clamp_length_max(HUNT_SPEED)
        } else {
            m.heading += (unit(&[m.seed, (t * 0.5) as u64, 0x4F]) - 0.5) * dt * 1.5;
            Vec2::from_angle(m.heading) * SPEED
        };
        let want = [0.0f32, 0.6, -0.6, 1.2, -1.2, 1.9, -1.9, 2.6, -2.6]
            .iter()
            .map(|&k| Vec2::from_angle(k).rotate(want))
            .find(|w| {
                let d = w.normalize_or(Vec2::X);
                (1..=3).all(|i| !solid(&sim, m.core + d * (r_body * 0.6 + 6.0 * i as f32)))
            })
            .unwrap_or(-want * 0.5);
        m.vel += (want - m.vel).clamp_length_max(ACCEL * dt);
        if solid(&sim, m.core + m.vel * dt) {
            m.vel = -m.vel * 0.3;
        }
        m.core += m.vel * dt;
        if let Ok((mut tf, _)) = hearts.get_mut(m.light) {
            tf.translation = m.core.extend(0.0);
        }
        // Its tendrils: each from a root in its mass, its tip where it
        // wants (curling about; winding back, then lashing at you), the
        // links after it (FABRIK: back from the tip, then out from the
        // root).
        for (i, td) in m.tendrils.iter_mut().enumerate() {
            td.angle += dt * if i % 2 == 0 { 0.13 } else { -0.11 };
            td.rest = (td.rest - dt).max(0.0);
            let share = (in_tendril[i] as f32 / TENDRIL_MOTES as f32).clamp(0.0, 1.0);
            let len = REACH * share.max(0.06);
            let seg = len / SEGS as f32;
            let root = m.core + Vec2::from_angle(td.angle) * r_body * 0.55;
            let sway = Vec2::from_angle(td.angle + 0.7 * (t * 0.55 + i as f32 * 1.7).sin());
            let mut target = root + sway * len * 0.95 + sway.perp() * (t * 1.3 + i as f32).sin() * len * 0.3;
            let mut speed = 60.0;
            match (td.lash, you) {
                (Lash::Idle, Some((p, ..))) if td.rest <= 0.0 && m.lashed <= 0.0 && share > 0.5 && p.distance(root) < len * 1.05 + 6.0 => {
                    m.lashed = LASH_APART;
                    td.lash = Lash::Wind(0.0);
                    td.struck = false;
                }
                (Lash::Wind(w), Some((p, ..))) => {
                    let to = (p - root).normalize_or(Vec2::X);
                    target = root - to * len * 0.25 + to.perp() * len * 0.5;
                    speed = 110.0;
                    td.lash = if w + dt > WIND { Lash::Strike(0.0) } else { Lash::Wind(w + dt) };
                    if matches!(td.lash, Lash::Strike(_)) {
                        sounds.write(crate::sound::PlaySound::at("murk_lash", root));
                    }
                }
                (Lash::Strike(s), Some((p, ..))) => {
                    target = p + (p - root).normalize_or(Vec2::X) * 14.0;
                    speed = 320.0;
                    if s + dt > STRIKE {
                        td.lash = Lash::Idle;
                        td.rest = LASH_REST;
                    } else {
                        td.lash = Lash::Strike(s + dt);
                    }
                }
                (_, None) => td.lash = Lash::Idle,
                _ => {}
            }
            let tip = td.pts[SEGS];
            let tip = tip + (target - tip).clamp_length_max(speed * dt);
            td.pts[SEGS] = tip;
            for j in (0..SEGS).rev() {
                let d = (td.pts[j] - td.pts[j + 1]).normalize_or(-sway);
                td.pts[j] = td.pts[j + 1] + d * seg;
            }
            td.pts[0] = root;
            for j in 1..=SEGS {
                let d = (td.pts[j] - td.pts[j - 1]).normalize_or(sway);
                td.pts[j] = td.pts[j - 1] + d * seg;
            }
            // (A lash meets you: once a strike.)
            if let (Lash::Strike(_), Some((k, h))) = (td.lash, me.as_mut())
                && !td.struck
                && ((td.pts[SEGS] - k.body.pos).abs() - k.body.half).max_element() < 3.0
            {
                td.struck = true;
                let push = (td.pts[SEGS] - td.pts[SEGS - 2]).normalize_or(Vec2::X) * LASH_KNOCK;
                strike(k, h, LASH_HIT, push, "lashed by the murk");
                sounds.write(crate::sound::PlaySound::at("slam", td.pts[SEGS]).volume(0.4));
            }
        }
        // Clumps: torn off the side toward you and thrown.
        m.next_clump -= dt;
        if let (true, Some((p, ..))) = (m.hunting, you)
            && m.next_clump <= 0.0
            && m.clumps.len() < CLUMPS_MOST
            && dist < THROW
            && in_body as f32 > full_body as f32 * 0.45
        {
            let n = CLUMP.0 + (unit(&[m.seed, tick, 0xC1]) * (CLUMP.1 - CLUMP.0) as f32) as usize;
            let to = (p - m.core).normalize_or(Vec2::X);
            let mut side: Vec<(usize, f32)> = m.motes.iter().enumerate().filter(|(_, o)| o.role == Role::Body).map(|(i, o)| (i, (o.pos - m.core).dot(to))).collect();
            side.sort_by(|a, b| b.1.total_cmp(&a.1));
            let id = m.clump_ids;
            m.clump_ids = m.clump_ids.wrapping_add(1);
            let mut mid = Vec2::ZERO;
            for (k, &(i, _)) in side.iter().take(n).enumerate() {
                let o = &mut m.motes[i];
                o.role = Role::Clump(id);
                o.a = unit(&[m.seed, tick, k as u64, 0xC2]) * TAU;
                o.b = unit(&[m.seed, tick, k as u64, 0xC3]).sqrt();
                mid += o.pos;
            }
            mid /= n.min(side.len()).max(1) as f32;
            in_clump.insert(id, n.min(side.len()));
            in_body -= n.min(side.len());
            m.clumps.push(Clump { id, pos: mid, vel: m.vel + to * 80.0, run: Run::Out, passes: 0, age: 0.0, closest: f32::MAX, struck: false });
            m.next_clump = CLUMP_EVERY.0 + (CLUMP_EVERY.1 - CLUMP_EVERY.0) * unit(&[m.seed, tick, 0xC4]);
            sounds.write(crate::sound::PlaySound::at("murk_hiss", mid).volume(0.7));
        }
        let mut dissolved = Vec::new();
        let mut merged = Vec::new();
        for c in &mut m.clumps {
            let n = in_clump.get(&c.id).copied().unwrap_or(0);
            if n < 6 {
                dissolved.push(c.id);
                continue;
            }
            c.age += dt;
            let (p, pv) = you.map_or((m.core, Vec2::ZERO), |y| (y.0, y.1));
            match c.run {
                Run::Out => {
                    let to = p + pv * 0.12 - c.pos;
                    c.vel += (to.normalize_or(Vec2::X) * CLUMP_SPEED - c.vel).clamp_length_max(CLUMP_TURN * dt);
                    let d = to.length();
                    c.closest = c.closest.min(d);
                    if c.closest < 12.0 && d > c.closest + 5.0 {
                        c.passes += 1;
                        c.run = Run::Through(0.0);
                    }
                    if c.age > CLUMP_SECS || you.is_none() {
                        c.run = Run::Home;
                    }
                }
                Run::Through(w) => {
                    c.vel *= 1.0 - 0.6 * dt;
                    c.run = match w + dt {
                        w if w < 0.5 => Run::Through(w),
                        _ if c.passes < PASSES && c.age < CLUMP_SECS => {
                            c.closest = f32::MAX;
                            c.struck = false;
                            Run::Out
                        }
                        _ => Run::Home,
                    };
                }
                Run::Home => {
                    let to = m.core - c.pos;
                    c.vel += (to.normalize_or(Vec2::X) * 150.0 - c.vel).clamp_length_max(CLUMP_TURN * dt);
                    if to.length() < r_body * 0.6 {
                        merged.push(c.id);
                    }
                }
            }
            if solid(&sim, c.pos + c.vel * dt * 3.0) {
                c.vel = -c.vel * 0.5;
                c.run = Run::Home;
            }
            c.pos += c.vel * dt;
            // (Its pass through you: once a pass.)
            if let (false, Some((k, h))) = (c.run == Run::Home || c.struck, me.as_mut())
                && ((c.pos - k.body.pos).abs() - k.body.half).max_element() < 3.5
            {
                c.struck = true;
                strike(k, h, CLUMP_HIT + n as f32 * CLUMP_HIT_PER, c.vel.normalize_or(Vec2::X) * 90.0, "torn apart by the murk");
                sounds.write(crate::sound::PlaySound::at("murk_tear", c.pos).volume(0.8));
            }
        }
        m.clumps.retain(|c| !dissolved.contains(&c.id) && !merged.contains(&c.id));
        for o in &mut m.motes {
            if let Role::Clump(id) = o.role {
                if dissolved.contains(&id) {
                    o.role = Role::Loose;
                    o.life = 0.6 + unit(&[m.seed, tick, o.a.to_bits() as u64]);
                } else if merged.contains(&id) {
                    o.role = Role::Body;
                }
            }
        }
        // Growing back: what it's lost, out of its heart, the tendrils
        // first.
        let alive = in_body + in_tendril.iter().sum::<usize>() + in_clump.values().sum::<usize>();
        if alive < m.most {
            m.owed = (m.owed + REGROW * dt).min((m.most - alive) as f32);
        } else {
            m.owed = 0.0;
        }
        let mut grown = 0;
        while m.owed >= 1.0 {
            m.owed -= 1.0;
            let short = (0..TENDRILS).filter(|&i| in_tendril[i] < TENDRIL_MOTES).min_by_key(|&i| in_tendril[i]);
            let role = match short {
                Some(i) => {
                    in_tendril[i] += 1;
                    Role::Tendril(i as u8)
                }
                None => Role::Body,
            };
            let o = m.mote(role, tick ^ (grown << 20));
            m.motes.push(o);
            grown += 1;
        }
        // Each mote toward where it keeps (in its mass, along its tendril,
        // in its clump), stirred; scattered, drifting; round rock, eating
        // rubble.
        let spin = if m.seed % 3 == 0 { 0.12 } else { -0.09 };
        let seedf = (m.seed % 1000) as f32;
        let clumps: HashMap<u16, (Vec2, Vec2, f32)> = m.clumps.iter().map(|c| (c.id, (c.pos, c.vel, (in_clump.get(&c.id).copied().unwrap_or(1) as f32).sqrt() * 0.85))).collect();
        let tendrils: Vec<([Vec2; SEGS + 1], f32)> = m.tendrils.iter().enumerate().map(|(i, td)| (td.pts, (in_tendril[i] as f32 / TENDRIL_MOTES as f32).clamp(0.0, 1.0))).collect();
        let room = (alive as f32 + m.owed) < MOTES as f32 * EAT_MOST;
        let mut ate = 0;
        for o in &mut m.motes {
            let swirl = flow(o.pos, t) + Vec2::from_angle(o.a * 37.0 + t * 3.0) * 0.25;
            let acc = if o.stun > 0.0 {
                o.stun -= dt;
                -o.vel * 0.6
            } else {
                match o.role {
                    Role::Body => {
                        let a = o.a + t * spin;
                        let home = m.core + Vec2::from_angle(a) * (HOLLOW + o.b * (edge(r_body, a, t, seedf) - HOLLOW).max(1.0));
                        (home - o.pos) * 7.0 - (o.vel - m.vel) * 2.6 + swirl * 26.0
                    }
                    Role::Tendril(i) => {
                        let (pts, share) = &tendrils[i as usize];
                        let (c, d) = along(pts, o.a);
                        let width = (3.2 * (1.0 - o.a) + 0.7) * share.max(0.5);
                        let home = c + d.perp() * o.b * width;
                        (home - o.pos) * 55.0 - o.vel * 9.0 + swirl * 10.0
                    }
                    Role::Clump(id) => match clumps.get(&id) {
                        Some(&(c, v, r)) => {
                            let home = c + Vec2::from_angle(o.a + t * 4.0) * o.b * r;
                            (home - o.pos) * 45.0 - (o.vel - v) * 6.0 + swirl * 30.0
                        }
                        None => Vec2::ZERO,
                    },
                    Role::Loose => {
                        o.life -= dt;
                        -o.vel * 0.5
                    }
                }
            };
            o.vel += acc * dt;
            let next = o.pos + o.vel * dt;
            let p = CellPos::from_world(next.x, next.y);
            let mats = sim.world.materials();
            match sim.world.get(p).filter(|c| !c.is_air() && matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder)) {
                // (Rubble, dust: taken in.)
                Some(c) if o.role != Role::Loose && room && ate < EAT_TICK && (mats.phys(c.material).kind == Kind::Powder || c.flags & flags::LOOSE != 0) => {
                    eaten.push(p);
                    ate += 1;
                    o.pos = next;
                }
                Some(_) => o.vel = -o.vel * 0.3,
                None => o.pos = next,
            }
        }
        m.owed += ate as f32;
        if ate > 0 && alive + m.owed as usize > m.most {
            m.most = (alive + m.owed as usize).min((MOTES as f32 * EAT_MOST) as usize);
        }
        // Light: it shies from it, and what's in the bright of it burns;
        // its heart flees. Your plume burns it.
        let mut lit: Vec<(Vec2, Vec2, f32, f32)> = shine.iter().filter(|(p, _)| p.distance(m.core) < r_body + REACH + 80.0).map(|&(p, s)| (p, p, LIGHT_R * s.sqrt(), s)).collect();
        if let Some((p, _, half, true, th)) = you {
            let from = p - th * half.y;
            lit.push((from, from - th * PLUME, PLUME_R, 3.0));
        }
        for (li, &(a, b, r, s)) in lit.iter().enumerate() {
            let ab = b - a;
            let near = |p: Vec2| {
                let f = if ab == Vec2::ZERO { 0.0 } else { ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0) };
                p - (a + ab * f)
            };
            if near(m.core).length() < r * 1.2 + HOLLOW {
                m.scared = 1.2;
                m.scared_from = a;
            }
            for (i, o) in m.motes.iter_mut().enumerate() {
                if o.role == Role::Loose {
                    continue;
                }
                let d = near(o.pos);
                let l = d.length();
                if l >= r {
                    continue;
                }
                o.vel += d.normalize_or(Vec2::Y) * 260.0 * (1.0 - l / r) * dt;
                let burning = if ab == Vec2::ZERO { l < r * LIGHT_BURNS && s > 0.9 } else { true };
                if burning && unit(&[m.seed, tick, i as u64, li as u64, 0xB0]) < BURN * dt * s.min(3.0) {
                    o.role = Role::Loose;
                    o.life = 0.0;
                    sparks.emit(&BURNT, 1, o.pos, o.vel, Vec2::ZERO);
                }
            }
        }
        // Inside its mass, it smothers you.
        if let Some((k, h)) = me.as_mut() {
            let (p, half) = (k.body.pos, k.body.half + Vec2::ONE);
            let n = m.motes.iter().filter(|o| o.role == Role::Body && (o.pos - p).abs().cmple(half).all()).count();
            if n >= 14 {
                h.harm_by(SMOTHER * dt, Harm::Void, || crate::death::Cause::new("The murk", "swallowed by the murk"));
            }
        }
        m.motes.retain(|o| o.role != Role::Loose || o.life > 0.0);
        // Its eyes: opening here and there in it, more when it hunts.
        let most_eyes = if m.hunting { 9 } else { 3 };
        if m.eyes.len() < most_eyes && unit(&[m.seed, tick, 0xE1]) < dt * if m.hunting { 4.0 } else { 1.6 } {
            let a = unit(&[m.seed, tick, 0xE2]) * TAU;
            let r = HOLLOW + 2.0 + unit(&[m.seed, tick, 0xE3]).sqrt() * (r_body * 0.75 - HOLLOW - 2.0).max(1.0);
            m.eyes.push(Eye { off: Vec2::from_angle(a) * r, age: 0.0, life: 1.0 + 2.2 * unit(&[m.seed, tick, 0xE4]) });
        }
        for e in &mut m.eyes {
            e.age += dt;
            e.off = Vec2::from_angle(spin * dt).rotate(e.off);
        }
        m.eyes.retain(|e| e.age < e.life);
        // A whisper, now and then, near you; louder with clumps out.
        m.whisper -= dt;
        if m.whisper <= 0.0 && dist < 380.0 {
            m.whisper = 2.2 + 2.0 * unit(&[m.seed, tick, 0x77]);
            let v = 0.25 + 0.15 * m.clumps.len() as f32;
            sounds.write(crate::sound::PlaySound::at("murk_whisper", m.core).volume(v));
        }
    }
    // Rubble eaten.
    for p in eaten {
        sim.world.set(p, platypus_sim::Cell::AIR);
    }
    // Where it's thick, for spells.
    let Murks { list, field, .. } = &mut *murks;
    field.clear();
    for m in list.iter().filter(|m| m.dying.is_none()) {
        for o in m.motes.iter().filter(|o| o.role != Role::Loose) {
            *field.entry(bin(o.pos)).or_default() += 1;
        }
    }
}

/// A shot or blast in it: the motes it meets gone (a tendril cut through
/// there: what's past the cut drifts off), a blast's scattering the rest;
/// its heart, if it got there.
fn take(m: &mut Murk, h: &MurkHit, sparks: &mut crate::vfx::Sparks, sounds: &mut MessageWriter<crate::sound::PlaySound>, tick: u64) {
    let reach = body_radius(m.most) + REACH + h.blast * BLAST_SCATTERS + 20.0;
    let clumped = m.clumps.iter().any(|c| c.pos.distance(h.at) < 20.0 + h.blast * BLAST_SCATTERS);
    if h.at.distance(m.core) > reach && !clumped {
        return;
    }
    if h.at.distance(m.core) < CORE_R + 1.5 + h.blast * 0.5 {
        m.hp -= h.damage + h.blast * 2.0;
        m.hurt = 1.0;
        m.scared = m.scared.max(0.8);
        m.scared_from = h.at - h.dir * 10.0;
        sounds.write(crate::sound::PlaySound::at("murk_death", h.at).volume(0.35).pitch(1.6));
    }
    let r = SHOT_R.0 + h.damage * SHOT_R.1;
    let mut near: Vec<(usize, f32)> = m.motes.iter().enumerate().filter(|(_, o)| o.role != Role::Loose).map(|(i, o)| (i, o.pos.distance(h.at))).filter(|(_, d)| *d < r.max(h.blast * BLAST_SCATTERS)).collect();
    near.sort_by(|a, b| a.1.total_cmp(&b.1));
    let most = if h.blast > 0.0 { usize::MAX } else { (h.damage * SHOT_MOST).ceil() as usize };
    let mut taken = 0;
    let mut cut = [f32::MAX; TENDRILS];
    for &(i, d) in &near {
        let o = &mut m.motes[i];
        let takes = if h.blast > 0.0 { d < h.blast * BLAST_TAKES } else { d < r && taken < most };
        if takes {
            if let Role::Tendril(k) = o.role {
                cut[k as usize] = cut[k as usize].min(o.a);
            }
            o.role = Role::Loose;
            o.life = 0.0;
            taken += 1;
        } else if h.blast > 0.0 {
            let f = 1.0 - d / (h.blast * BLAST_SCATTERS);
            o.vel += (o.pos - h.at).normalize_or(h.dir) * 170.0 * f;
            o.stun = o.stun.max(1.0 + 1.8 * f);
        }
    }
    // (Cut through: what's past the cut loose.)
    for o in &mut m.motes {
        if let Role::Tendril(k) = o.role
            && o.a > cut[k as usize] - 0.04
        {
            o.role = Role::Loose;
            o.life = 0.8 + 1.2 * unit(&[m.seed, tick, o.a.to_bits() as u64]);
            o.vel += h.dir * 40.0;
        }
    }
    if taken > 0 {
        sparks.emit(&TEAR, (taken / 2).clamp(3, 30), h.at, -h.dir, Vec2::ZERO);
        m.hurt = m.hurt.max(0.35);
    }
}

/// The murk drawn a cell at a time: thick, black (a hole in the stars),
/// its edge a violet fringe; streaks after what's fast; what's cut off,
/// fading; its heart in its hollow, beating; its eyes.
#[derive(Resource)]
struct MurkCanvas(crate::canvas::Canvas);

impl Default for MurkCanvas {
    fn default() -> Self {
        MurkCanvas(crate::canvas::Canvas::new("Murk", Z_MURK))
    }
}

/// Over the light's glow (its black is black, whatever the light), under
/// glowfish.
const Z_MURK: f32 = 15.55;

#[allow(clippy::too_many_arguments)]
fn show(
    mut commands: Commands,
    time: Res<Time>,
    murks: Res<Murks>,
    mut canvas: ResMut<MurkCanvas>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<(&GlobalTransform, &crate::world::ChunkLoader), With<crate::camera::MainCamera>>,
    mut sprites: crate::canvas::CanvasSprites,
    player: Query<&Kinematics, With<LocalPlayer>>,
) {
    let Ok((cam, loader)) = camera.single() else { return };
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, !murks.list.is_empty()) else { return };
    let t = time.elapsed_secs();
    let you = player.single().ok().map(|k| k.body.pos);
    for m in &murks.list {
        let rgba = |c: [f32; 3], a: f32| [c[0] as u8, c[1] as u8, c[2] as u8, (a * 255.0) as u8];
        // Streaks after the fast (a clump flying, a tendril lashing), and
        // what's loose, fading.
        for o in &m.motes {
            if o.role == Role::Loose {
                let a = (o.life / 1.2).clamp(0.0, 1.0) * 0.8;
                px.put(o.pos.x.floor() as i32, o.pos.y.floor() as i32, rgba([75.0, 42.0, 125.0], a));
            } else if o.vel.length_squared() > 70.0 * 70.0 {
                let b = o.pos - o.vel * 0.03;
                px.put(b.x.floor() as i32, b.y.floor() as i32, rgba([58.0, 32.0, 98.0], 0.5));
            }
        }
        // How thick, cell by cell (each mote counts 2 where it is, 1 round
        // it).
        let live = m.motes.iter().filter(|o| o.role != Role::Loose);
        let (lo, hi) = live.clone().fold((Vec2::MAX, Vec2::MIN), |(lo, hi), o| (lo.min(o.pos), hi.max(o.pos)));
        if lo.x > hi.x {
            continue;
        }
        let lo = lo.floor().as_ivec2() - IVec2::splat(2);
        let hi = hi.floor().as_ivec2() + IVec2::splat(2);
        let (w, h) = ((hi.x - lo.x + 1) as usize, (hi.y - lo.y + 1) as usize);
        if w * h > 1_500_000 {
            continue;
        }
        // (Motes in each cell; then how many in the 5×5 round it: its
        // cover, smooth, so its inside is whole.)
        let mut here = vec![0u8; w * h];
        for o in live {
            let c = o.pos.floor().as_ivec2() - lo;
            if c.x >= 0 && c.y >= 0 && (c.x as usize) < w && (c.y as usize) < h {
                let i = c.y as usize * w + c.x as usize;
                here[i] = here[i].saturating_add(1);
            }
        }
        let mut rows = vec![0u16; w * h];
        for y in 0..h {
            for x in 0..w {
                rows[y * w + x] = (x.saturating_sub(2)..(x + 3).min(w)).map(|i| here[y * w + i] as u16).sum();
            }
        }
        let mut thick = vec![0u16; w * h];
        for y in 0..h {
            for x in 0..w {
                thick[y * w + x] = (y.saturating_sub(2)..(y + 3).min(h)).map(|j| rows[j * w + x]).sum();
            }
        }
        let flash = m.hurt;
        for y in 0..h {
            for x in 0..w {
                let (d, n) = (thick[y * w + x], here[y * w + x]);
                if d == 0 {
                    continue;
                }
                let (cx, cy) = (lo.x + x as i32, lo.y + y as i32);
                let shimmer = 0.5 + 0.5 * (cx as f32 * 0.7 + cy as f32 * 0.45 + t * 4.0).sin();
                // (Its edge: in it, the outside within two cells.)
                let at = |dx: i32, dy: i32| {
                    let (x, y) = (x as i32 + dx, y as i32 + dy);
                    if x < 0 || y < 0 || x as usize >= w || y as usize >= h { 0 } else { thick[y as usize * w + x as usize] }
                };
                let edge = d >= 3 && [(2, 0), (-2, 0), (0, 2), (0, -2), (1, 1), (-1, -1), (1, -1), (-1, 1)].iter().any(|&(dx, dy)| at(dx, dy) < 3);
                // (Inside: black, a dark sheen where a mote is (it moves in
                // there); its edge violet; past it, fuzz.)
                let (c, a) = match (d, edge, n > 0) {
                    (3.., false, false) => ([2.0, 0.0, 5.0], 1.0),
                    (3.., false, true) => ([10.0 + 8.0 * shimmer, 3.0, 22.0 + 14.0 * shimmer], 1.0),
                    (3.., true, true) => ([92.0 + 50.0 * shimmer, 48.0 + 20.0 * shimmer, 165.0 + 55.0 * shimmer], 0.95),
                    (3.., true, false) => ([40.0 + 20.0 * shimmer, 18.0, 80.0 + 30.0 * shimmer], 0.95),
                    (_, _, true) => ([110.0 + 40.0 * shimmer, 60.0, 190.0 + 40.0 * shimmer], 0.55),
                    _ => continue,
                };
                // (Hurt: its edge flares.)
                let c = if edge && flash > 0.0 { [c[0] + (190.0 - c[0]) * flash, c[1] + (120.0 - c[1]) * flash, c[2] + (255.0 - c[2]) * flash] } else { c };
                px.put(cx, cy, rgba(c, a));
            }
        }
        if m.dying.is_some() {
            continue;
        }
        // Its heart, beating in its hollow.
        let beat = (t * TAU * if m.hunting { 1.6 } else { 0.9 }).sin().max(0.0).powi(3);
        let heart = [120.0 + 110.0 * beat + 60.0 * flash, 18.0 + 50.0 * beat, 66.0 + 80.0 * beat + 60.0 * flash];
        let r = CORE_R * (0.55 + 0.12 * beat);
        let c = m.core.floor().as_ivec2();
        let ri = r.ceil() as i32;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let l = Vec2::new(dx as f32, dy as f32).length();
                if l <= r {
                    let k = 1.0 - 0.5 * l / r;
                    px.put(c.x + dx, c.y + dy, rgba(heart.map(|v| v * k), 1.0));
                }
            }
        }
        // Its eyes: pale, opening and shutting, looking your way; all wide
        // when it's hurt. Only in its thick.
        for e in &m.eyes {
            let p = m.core + e.off;
            let at = p.floor().as_ivec2() - lo;
            if at.x < 0 || at.y < 0 || at.x as usize >= w || at.y as usize >= h || thick[at.y as usize * w + at.x as usize] < 9 {
                continue;
            }
            let open = if flash > 0.3 { 1.0 } else { (e.age / 0.3).min((e.life - e.age) / 0.3).clamp(0.0, 1.0) };
            if open < 0.25 {
                continue;
            }
            let c = p.floor().as_ivec2();
            let pale = [222.0, 212.0, 250.0];
            // (Some small, some big: an almond, a slit pupil turned your
            // way.)
            let big = e.life > 2.0;
            let half = if big { 2 } else { 1 };
            for dx in -half..=half {
                px.put(c.x + dx, c.y, rgba(pale, 1.0));
            }
            if open > 0.6 {
                for dx in -(half - 1).max(0)..=(half - 1).max(0) {
                    px.put(c.x + dx, c.y + 1, rgba(pale, 0.95));
                    px.put(c.x + dx, c.y - 1, rgba(pale, 0.95));
                }
                let look = you.map_or(0, |y| if (y.x - p.x).abs() > 4.0 { (y.x - p.x).signum() as i32 } else { 0 });
                let pupil = [40.0 + 60.0 * flash, 0.0, 30.0];
                px.put(c.x + look, c.y, rgba(pupil, 1.0));
                if big {
                    px.put(c.x + look, c.y + 1, rgba(pupil, 1.0));
                    px.put(c.x + look, c.y - 1, rgba(pupil, 1.0));
                }
            }
        }
    }
}
