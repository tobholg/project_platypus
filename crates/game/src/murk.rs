//! The murk (the space arc): a mass of dark motes round a hidden heart,
//! adrift in the void. It has no shape of its own. Its motes are pulled
//! toward a lumpy, breathing blob round the heart and stirred by a flow
//! that never settles, so it writhes; out of it reach a few tendrils.
//!
//! - **It moves by pushes.** It drifts on surges: a push throws it on
//!   (its back squirting out behind it like a jet), and its stabilisers
//!   settle it before the next. Its edge glows a faint violet.
//! - **It charges.** Now and then it coils back away from you, its maw
//!   opening your way, eyes wide, and then it comes at you all at once,
//!   turning after you a little; through you, it flings you off to the
//!   side and stops. Get out of its line while it's coiled.
//! - **It shoots.** It knots a row of black spikes on its front, pointing
//!   at you, quivering; then shoots them one after another, needles of
//!   itself, straight at where you'll be. Move across their line.
//! - **It hunts.** Near you, it keeps a little way off and circles. It
//!   tears clumps of itself off and throws them at you: each one a small
//!   swarm that cuts through you, loops back for another pass, and drifts
//!   home to merge into the mass again (it shrinks as it throws, and swells
//!   as they come back). Close in, its tendrils wind back and lash.
//! - **It spears you.** Now and then, from well off, it gathers itself
//!   toward you (its mass bunching your way, its eyes opening on that side,
//!   its whisper gone quiet, a tendril drawn back, quivering) and shoots
//!   that tendril out at you, fed by motes streaming out of its mass. Missed,
//!   it hangs there twitching and reels slowly back. Struck, it holds on and
//!   hauls you in while the mass opens a maw your way, its heart bare in it,
//!   eyes round it; if it gets you there, everything lashes at once and it
//!   closes on you. Thrusting away holds you off (it hauls far less hard)
//!   and pulls the line out till it snaps, your plume burning it; a harpoon
//!   in rock holds you too. Held, your shots run along the line, into its
//!   open maw: your one clear shot at its heart.
//! - **It's made of what it's lost.** Shot, the motes a bullet meets are
//!   gone and the bullet stops in it; a blast scatters it, and it takes a
//!   few seconds to pull itself together. A tendril shot through is cut
//!   off there (what's past the cut drifts off and fades) and grows back
//!   from the mass, as everything it's lost does, slowly, out of the heart.
//! - **It eats rubble.** Loose rock and dust it drifts through it takes
//!   in, to grow back faster, and past its size (to half again as big):
//!   where you've been mining, it's worse.
//! - **Light hurts it.** It shies from bright light (a lamp, a flare,
//!   your rocket plume) and what's in the bright of it burns away; its
//!   heart flees from it. (Not a shot's glow: shots strike it.)
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

/// Its name in the dev menu's Creatures tab (placed there like a creature).
pub const KIND: &str = "murk";

impl Plugin for MurkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Murks>()
            .init_resource::<MurkCanvas>()
            .add_systems(FixedUpdate, (come_and_go, live).chain().in_set(TickSet::Bodies).after(crate::creatures::move_creatures))
            .add_systems(Update, numbers)
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
const HUNT_SPEED: f32 = 64.0;
/// How it moves: pushes, now and then (seconds between; sooner hunting,
/// this share of it), each to this much past what it wants (it surges,
/// then glides); between them its stabilisers settle it (a share of its
/// speed lost a second). A push throws its back out behind it (a jet).
const PULSE_EVERY: (f32, f32) = (0.55, 1.1);
const PULSE_HUNTING: f32 = 0.65;
const PULSE_OVER: f32 = 1.8;
const STABILISE: f32 = 1.9;
const JET: f32 = 60.0;
/// Its charge: every so often (s), you so far off; coiled back (s, its maw
/// opening), then at you this fast (cells/s), turning after you this fast
/// (rad/s), so long at most; what it does going through you.
const CHARGE_EVERY: (f32, f32) = (8.0, 13.0);
const CHARGE_FROM: (f32, f32) = (70.0, 280.0);
const COIL: f32 = 0.55;
const CHARGE_SPEED: f32 = 330.0;
const CHARGE_TURN: f32 = 1.3;
const CHARGE_SECS: f32 = 1.1;
const CHARGE_HIT: f32 = 9.0;
const CHARGE_KNOCK: f32 = 280.0;
/// Its volley: shards of itself (so many motes each) knotted on its front
/// (the tell, s), then shot one after another (s apart), straight, at
/// where you'll be (a share of the lead), a little spread (rad); gone in
/// so long, or when they meet something.
const VOLLEY_EVERY: (f32, f32) = (6.0, 10.0);
/// After a spear, a charge or a volley, a breath before the next of
/// them (s).
const BREATHE: f32 = 2.5;
const VOLLEY_FROM: (f32, f32) = (60.0, 330.0);
const VOLLEY: (usize, usize) = (6, 10);
const VOLLEY_TELL: f32 = 0.7;
const VOLLEY_GAP: f32 = 0.085;
const SHARD_MOTES: usize = 4;
const SHARD_SPEED: f32 = 290.0;
const SHARD_LEAD: f32 = 0.6;
const SHARD_SPREAD: f32 = 0.07;
const SHARD_SECS: f32 = 1.6;
const SHARD_HIT: f32 = 1.6;
/// Clumps: so many motes (from, to), one this often (s, from, to), so many
/// out at most, thrown at you from this near; flying this fast, turning
/// this hard; two passes, then home; gone home at most after this long.
const CLUMP: (usize, usize) = (34, 72);
const CLUMP_EVERY: (f32, f32) = (3.2, 5.5);
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
/// The spear: one this often (s, from, to), at you from this far (from,
/// to; cells); its tell (s); out this fast, this far at most (cells/s,
/// cells, from its root); fed so many motes more from its mass; what it
/// does striking, hauling you (cells/s², at most cells/s), for at most this
/// long; missed, it hangs this long, then reels in this fast; the feast
/// (s) and its bite. (Harm before the difficulty's share.)
const SPEAR_EVERY: (f32, f32) = (8.0, 12.0);
const SPEAR_FROM: (f32, f32) = (55.0, 190.0);
const TELL: f32 = 0.6;
const SPEAR_SPEED: f32 = 620.0;
const SPEAR_REACH: f32 = 210.0;
const SPEAR_FEED: usize = 110;
const SPEAR_HIT: f32 = 4.0;
const HAUL: f32 = 520.0;
const HAUL_TOP: f32 = 100.0;
/// Thrusting away, it hauls only this share as hard; stretched this far
/// past its reach, it snaps.
const HAUL_THRUSTING: f32 = 0.2;
const SNAP: f32 = 30.0;
const HOLD_MOST: f32 = 3.0;
const HANG: f32 = 0.45;
const REEL: f32 = 150.0;
const FEAST: f32 = 1.5;
const FEAST_BITE: f32 = 4.0;
/// Shot: a bullet takes the motes this near where it stops (cells, and so
/// much more for each point of its damage), at most so many for each; a
/// blast takes all within this share of its radius and scatters those out
/// to this many times it.
const SHOT_R: (f32, f32) = (2.5, 1.0 / 8.0);
const SHOT_MOST: f32 = 1.2;
/// A blade cuts it once a swing (ticks before the same blade may again),
/// and all along it: past the first cut, this share of its damage every
/// third cell.
const CUT_AGAIN: u64 = 18;
const CUT_ALONG: f32 = 0.6;
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

/// `PLATYPUS_MURK_ONLY=spear|charge|volley|clump`: it does only that (and
/// sooner), to look at.
static ONLY: std::sync::LazyLock<Option<String>> = std::sync::LazyLock::new(|| std::env::var("PLATYPUS_MURK_ONLY").ok());

fn may(what: &str) -> bool {
    ONLY.as_deref().is_none_or(|o| o == what)
}

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
    /// A shard of a volley.
    Shard(u16),
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
    /// Its tip's speed last tick (its motes keep up with it).
    tip_vel: Vec2,
}

/// The spear (a tendril, `i`): gathering itself; out (that way, so
/// long); holding you (so many motes in it when it struck); missed or let
/// go, hanging and reeling back; you in its maw.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Spear {
    Idle,
    Tell { t: f32, i: usize },
    Fly { i: usize, dir: Vec2, len: f32 },
    Held { t: f32, i: usize, motes: usize },
    Miss { t: f32, i: usize, dir: Vec2, len: f32 },
    Feast { t: f32 },
}

impl Spear {
    /// The tendril it's using, steered by it (not curling, not lashing).
    fn tendril(self) -> Option<usize> {
        match self {
            Spear::Tell { i, .. } | Spear::Fly { i, .. } | Spear::Held { i, .. } | Spear::Miss { i, .. } => Some(i),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Spear::Idle => "idle",
            Spear::Tell { .. } => "tell",
            Spear::Fly { .. } => "fly",
            Spear::Held { .. } => "held",
            Spear::Miss { .. } => "miss",
            Spear::Feast { .. } => "feast",
        }
    }
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

/// Its charge: coiling back (aimed that way), then rushing at you (once
/// through you at most).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Charge {
    Idle,
    Coil { t: f32, dir: Vec2 },
    Rush { t: f32, dir: Vec2, hit: bool },
}

/// Its volley: knotting its shards, then shooting them (the next in so
/// long).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Volley {
    Idle,
    Tell { t: f32 },
    Fire { next: f32 },
}

/// A shard: held in the knot (its place in it) till shot.
struct Shard {
    id: u16,
    pos: Vec2,
    vel: Vec2,
    age: f32,
    held: bool,
    slot: usize,
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
    /// The spear, and till the next; how much its mass leans your way
    /// (its tell), how open its maw is, and which way.
    spear: Spear,
    spear_next: f32,
    lean: f32,
    maw: f32,
    maw_dir: Vec2,
    /// Till its next push; its charge (and till the next), how coiled
    /// back it is; its volley (and till the next), its shards.
    pulse: f32,
    charge: Charge,
    charge_next: f32,
    coil: f32,
    volley: Volley,
    volley_next: f32,
    shards: Vec<Shard>,
    shard_ids: u16,
    /// Till it may spear, charge or shoot again.
    breath: f32,
    /// Where it's been struck, just now (drawn: a flare, a ripple).
    impacts: Vec<Impact>,
}

/// A blow landing in it: where, which way, how hard, how long ago; at its
/// heart.
struct Impact {
    at: Vec2,
    dir: Vec2,
    power: f32,
    age: f32,
    heart: bool,
}

/// What a spell did meeting a murk: where it stopped, which way it was
/// going, its damage, its blast's radius.
#[derive(Clone, Copy, Debug)]
pub struct MurkHit {
    pub at: Vec2,
    pub dir: Vec2,
    pub damage: f32,
    pub blast: f32,
    /// Its damage floats up off it as a number (one of a blade's cuts
    /// along it doesn't: the first says it).
    pub shown: bool,
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
    /// Numbers to float up off it: where, how much, at its heart.
    numbers: Vec<(Vec2, f32, bool)>,
    /// Blades that cut it lately (one cut a swing), and when (tick).
    cut_by: Vec<(Entity, u64)>,
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

    /// Each one's spear (idle, tell, fly, held, miss, feast) and the line
    /// from its root to its tip when it's out (for the scenarios).
    pub fn spears(&self) -> Vec<(&'static str, Option<(Vec2, Vec2)>)> {
        self.list
            .iter()
            .map(|m| {
                let line = m.spear.tendril().filter(|_| !matches!(m.spear, Spear::Tell { .. })).map(|i| (m.tendrils[i].pts[0], m.tendrils[i].pts[SEGS]));
                (m.spear.name(), line)
            })
            .collect()
    }

    /// Each one's charge and volley (for the scenarios).
    pub fn moves(&self) -> Vec<(&'static str, &'static str)> {
        self.list
            .iter()
            .map(|m| {
                let c = match m.charge {
                    Charge::Idle => "-",
                    Charge::Coil { .. } => "coil",
                    Charge::Rush { .. } => "rush",
                };
                let v = match m.volley {
                    Volley::Idle => "-",
                    Volley::Tell { .. } => "knot",
                    Volley::Fire { .. } => "fire",
                };
                (c, v)
            })
            .collect()
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
                Tendril { pts: [at; SEGS + 1], angle, lash: Lash::Idle, rest: unit(&[seed, i as u64, 0x7F]) * LASH_REST, struck: false, tip_vel: Vec2::ZERO }
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
            spear: Spear::Idle,
            spear_next: SPEAR_EVERY.0 * 0.4,
            lean: 0.0,
            maw: 0.0,
            maw_dir: Vec2::X,
            pulse: 0.0,
            charge: Charge::Idle,
            charge_next: CHARGE_EVERY.0 * 0.6,
            coil: 0.0,
            volley: Volley::Idle,
            volley_next: VOLLEY_EVERY.0 * 0.5,
            shards: Vec::new(),
            shard_ids: 0,
            breath: 0.0,
            impacts: Vec::new(),
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

/// What blows did to it, floating up (with damage numbers on): a pale
/// violet number for what's torn out of its mass (a stream of shots adds
/// up on one), a big gold one with a "!" for its heart.
fn numbers(mut commands: Commands, display: Res<crate::display::Display>, mut murks: ResMut<Murks>, mut q: Query<(&mut crate::creatures::body::hurt::DamageNumber, &mut Text2d)>, mut last: Local<Option<Entity>>, mut n: Local<u32>) {
    for (at, amount, heart) in std::mem::take(&mut murks.numbers) {
        if !display.damage_numbers || amount <= 0.0 {
            continue;
        }
        *n = n.wrapping_add(1);
        let at = at + Vec2::new((*n % 5) as f32 * 3.0 - 6.0, 7.0);
        let color = if heart { Color::srgb(1.0, 0.82, 0.2) } else { Color::srgb(0.84, 0.7, 1.0) };
        crate::creatures::body::hurt::float_number(&mut commands, &mut q, &mut last, at, amount, color, heart);
    }
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

/// Struck: bright violet flecks of it, sprayed back.
static SPLASH: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 12.0,
    life: (0.12, 0.42),
    colors: vec![(236, 220, 255), (190, 140, 255), (130, 70, 235)],
    speed: 150.0,
    spread: 0.9,
    gravity: 0.0,
    drag: 5.0,
    size: 1.0,
    jitter: 0.4,
    glow: true,
});

/// Struck in its heart: hot pale-gold and rose, more of it.
static HEART_SPLASH: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 20.0,
    life: (0.2, 0.6),
    colors: vec![(255, 246, 220), (255, 200, 120), (240, 90, 160)],
    speed: 190.0,
    spread: 1.4,
    gravity: 0.0,
    drag: 4.0,
    size: 1.0,
    jitter: 0.5,
    glow: true,
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
    mut trauma: ResMut<crate::fx::Trauma>,
    (blades, mut stop): (Res<crate::combat::LiveBlades>, ResMut<crate::combat::HitStop>),
) {
    let mut hits = std::mem::take(&mut murks.hits);
    // (Blasts: a bomb's, a meteor's. Not a shot's that struck it: its blast
    // is one of these too, a tick or two after.)
    let tick = sim.world.tick();
    murks.lately.retain(|(_, at)| tick.saturating_sub(*at) < 4);
    murks.lately.extend(hits.iter().map(|h| (h.at, tick)));
    // (Blades through it: a cut a swing, along the blade; the first, or
    // the one at its heart, says what it did.)
    murks.cut_by.retain(|(_, at)| tick.saturating_sub(*at) < CUT_AGAIN);
    if !murks.list.is_empty() {
        let mut struck = false;
        for b in &blades.0 {
            if murks.cut_by.iter().any(|(w, _)| *w == b.who) {
                continue;
            }
            let thick: Vec<Vec2> = b.cells.iter().copied().filter(|c| murks.thick(*c)).collect();
            if thick.is_empty() {
                continue;
            }
            let near_heart = |c: &Vec2| murks.list.iter().any(|m| m.dying.is_none() && m.core.distance(*c) < CORE_R + 3.0);
            let first = thick.iter().copied().find(|c| near_heart(c)).unwrap_or(thick[0]);
            hits.push(MurkHit { at: first, dir: b.dir, damage: b.damage, blast: 0.0, shown: true });
            for c in thick.iter().step_by(3).filter(|c| **c != first && !near_heart(c)) {
                hits.push(MurkHit { at: *c, dir: b.dir, damage: b.damage * CUT_ALONG, blast: 0.0, shown: false });
            }
            murks.cut_by.push((b.who, tick));
            struck = true;
        }
        if struck {
            stop.hit(crate::combat::STOP);
            trauma.0 = (trauma.0 + 0.12).min(1.0);
        }
    }
    let lately = &murks.lately;
    let blasts: Vec<MurkHit> = booms.read().filter(|b| lately.iter().all(|(s, _)| s.distance(b.at) > 6.0)).map(|b| MurkHit { at: b.at, dir: Vec2::ZERO, damage: b.power * 0.25, blast: b.radius, shown: true }).collect();
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
    let mut numbers = Vec::new();
    for m in &mut murks.list {
        m.age += dt;
        m.impacts.retain_mut(|i| {
            i.age += dt;
            i.age < IMPACT_LIFE
        });
        m.hurt = (m.hurt - dt * 2.5).max(0.0);
        m.scared = (m.scared - dt).max(0.0);
        m.lashed = (m.lashed - dt).max(0.0);
        m.breath = (m.breath - dt).max(0.0);
        let you = me.as_ref().map(|(k, _)| (k.body.pos, k.body.vel, k.body.half, k.loco.rocketing(), k.loco.thrust()));
        // What struck it.
        if m.dying.is_none() {
            for h in &hits {
                if let Some(n) = take(m, h, &mut sparks, &mut sounds, tick) {
                    // (At its heart: the world jolts.)
                    if n.2 {
                        trauma.0 = (trauma.0 + 0.3).min(1.0);
                    }
                    numbers.push(n);
                }
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
            m.shards.clear();
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
        let mut in_shard: HashMap<u16, usize> = HashMap::default();
        for o in &m.motes {
            match o.role {
                Role::Body => in_body += 1,
                Role::Tendril(i) => in_tendril[i as usize] += 1,
                Role::Clump(id) => *in_clump.entry(id).or_default() += 1,
                Role::Shard(id) => *in_shard.entry(id).or_default() += 1,
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
        } else if m.spear != Spear::Idle || m.volley != Volley::Idle {
            Vec2::ZERO
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
        // (Pushed, now and then, past what it wants; settling between.
        // Charging, it's its charge's.)
        let mut pushed = Vec2::ZERO;
        if m.charge == Charge::Idle {
            m.vel *= (-STABILISE * dt).exp();
            m.pulse -= dt;
            if m.pulse <= 0.0 {
                let need = want * PULSE_OVER - m.vel;
                if need.length() > 12.0 {
                    pushed = need;
                }
                let every = PULSE_EVERY.0 + (PULSE_EVERY.1 - PULSE_EVERY.0) * unit(&[m.seed, tick, 0x9A]);
                m.pulse = every * if m.hunting { PULSE_HUNTING } else { 1.0 };
            }
        }
        if solid(&sim, m.core + m.vel * dt) {
            m.vel = -m.vel * 0.3;
        }
        m.core += m.vel * dt;
        if let Ok((mut tf, _)) = hearts.get_mut(m.light) {
            tf.translation = m.core.extend(0.0);
        }
        // The spear: gathering, out, holding, reeling back; what it does to
        // you. (Its tendril's tip is set here: `steer`.)
        m.spear_next -= dt;
        let r_root = r_body * 0.55;
        let root_of = |core: Vec2, td: &Tendril| core + Vec2::from_angle(td.angle) * r_root;
        let mut steer: Option<(usize, Vec2, f32, bool)> = None;
        if let (Spear::Idle, Some((p, ..))) = (m.spear, you)
            && m.hunting
            && m.spear_next <= 0.0
            && m.scared <= 0.0
            && m.charge == Charge::Idle
            && m.volley == Volley::Idle
            && m.breath <= 0.0
            && may("spear")
            && (SPEAR_FROM.0..SPEAR_FROM.1).contains(&dist)
            && in_body as f32 > full_body as f32 * 0.6
        {
            // (The tendril rooted nearest your way.)
            let way = (p - m.core).to_angle();
            let off = |a: f32| ((a - way + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI).abs();
            let i = (0..TENDRILS).min_by(|&a, &b| off(m.tendrils[a].angle).total_cmp(&off(m.tendrils[b].angle))).unwrap_or(0);
            m.tendrils[i].lash = Lash::Idle;
            // (Fed out of its mass, from round its root.)
            let root = root_of(m.core, &m.tendrils[i]);
            let mut near: Vec<(usize, f32)> = m.motes.iter().enumerate().filter(|(_, o)| o.role == Role::Body).map(|(k, o)| (k, o.pos.distance_squared(root))).collect();
            near.sort_by(|a, b| a.1.total_cmp(&b.1));
            let fed = SPEAR_FEED.min(near.len());
            for (n, &(k, _)) in near.iter().take(fed).enumerate() {
                let o = &mut m.motes[k];
                o.role = Role::Tendril(i as u8);
                o.a = unit(&[m.seed, tick, n as u64, 0x5E1]);
                o.b = unit(&[m.seed, tick, n as u64, 0x5E2]) * 2.0 - 1.0;
            }
            in_tendril[i] += fed;
            in_body -= fed;
            // (Its eyes open on your side.)
            for n in 0..5u64 {
                let a = way + (unit(&[m.seed, tick, n, 0x5E3]) - 0.5) * 1.6;
                let r = HOLLOW + 3.0 + unit(&[m.seed, tick, n, 0x5E4]) * (r_body * 0.6 - HOLLOW).max(1.0);
                m.eyes.push(Eye { off: Vec2::from_angle(a) * r, age: 0.0, life: TELL + 1.2 + n as f32 * 0.1 });
            }
            m.spear = Spear::Tell { t: 0.0, i };
            sounds.write(crate::sound::PlaySound::at("murk_hiss", m.core).volume(0.9).pitch(0.6));
            debug!("murk: spear: tell (tendril {i}), you {dist:.0} off");
        }
        let was_idle = m.spear == Spear::Idle;
        match m.spear {
            Spear::Idle => m.lean = (m.lean - dt * 3.0).max(0.0),
            Spear::Tell { t, i } => {
                m.lean = (t / TELL).min(1.0);
                if let Some((p, pv, ..)) = you {
                    // (Its root turned your way, the tendril drawn back,
                    // quivering.)
                    let core = m.core;
                    let td = &mut m.tendrils[i];
                    let way = (p - core).to_angle();
                    let d = (way - td.angle + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI;
                    td.angle += d.clamp(-5.0 * dt, 5.0 * dt);
                    let root = root_of(core, td);
                    let to = (p - root).normalize_or(Vec2::X);
                    let tip = root - to * 7.0 + to.perp() * (t * 45.0).sin() * 3.0;
                    steer = Some((i, tip, REACH * 0.4, false));
                    if t + dt >= TELL {
                        let dir = (p + pv * 0.12 - root).normalize_or(to);
                        let len = td.pts[SEGS].distance(root).min(REACH * 0.4);
                        m.spear = Spear::Fly { i, dir, len };
                        sounds.write(crate::sound::PlaySound::at("murk_spear", root));
                        debug!("murk: spear: out");
                    } else {
                        m.spear = Spear::Tell { t: t + dt, i };
                    }
                } else {
                    m.spear = Spear::Miss { t: HANG, i, dir: Vec2::X, len: REACH * 0.4 };
                }
            }
            Spear::Fly { i, dir, len } => {
                m.lean = (m.lean - dt * 3.0).max(0.0);
                let root = root_of(m.core, &m.tendrils[i]);
                let was = root + dir * len;
                let len = len + SPEAR_SPEED * dt;
                let tip = root + dir * len;
                steer = Some((i, tip, len, true));
                // (Through you, this tick?)
                let struck = me.as_ref().is_some_and(|(k, _)| {
                    let (p, half) = (k.body.pos, k.body.half);
                    let ab = tip - was;
                    let f = ((p - was).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
                    ((was + ab * f - p).abs() - half).max_element() < 2.5
                });
                if struck {
                    // (Hurt, not knocked: you can thrust at once.)
                    if let Some((_, h)) = me.as_mut() {
                        h.harm_by(SPEAR_HIT, Harm::Void, || crate::death::Cause::new("The murk", "speared by the murk"));
                    }
                    sounds.write(crate::sound::PlaySound::at("murk_tear", tip));
                    sounds.write(crate::sound::PlaySound::at("slam", tip).volume(0.5));
                    m.spear = Spear::Held { t: 0.0, i, motes: in_tendril[i] };
                    debug!("murk: spear: struck, {len:.0} out");
                } else if len > SPEAR_REACH || solid(&sim, tip) {
                    m.spear = Spear::Miss { t: 0.0, i, dir, len };
                    debug!("murk: spear: missed, {len:.0} out");
                } else {
                    m.spear = Spear::Fly { i, dir, len };
                }
            }
            Spear::Held { t, i, motes } => {
                let root = root_of(m.core, &m.tendrils[i]);
                let tip = m.tendrils[i].pts[SEGS];
                let loose = Spear::Miss { t: HANG, i, dir: (tip - root).normalize_or(Vec2::X), len: tip.distance(root) };
                match me.as_mut() {
                    // (Cut through, burnt: let go.)
                    Some(_) if in_tendril[i] * 10 < motes * 6 => {
                        m.spear = loose;
                        sounds.write(crate::sound::PlaySound::at("murk_death", root).volume(0.4).pitch(1.4));
                        debug!("murk: spear: cut loose, {} of {motes} left", in_tendril[i]);
                    }
                    Some((k, _)) if k.body.pos.distance(m.core) < r_body * 0.8 => {
                        m.spear = Spear::Feast { t: 0.0 };
                        for td in &mut m.tendrils {
                            td.lash = Lash::Wind(WIND * 0.4);
                            td.struck = false;
                            td.rest = 0.0;
                        }
                        debug!("murk: spear: the feast");
                    }
                    Some((k, _)) if k.body.pos.distance(root) > SPEAR_REACH + SNAP => {
                        m.spear = loose;
                        sounds.write(crate::sound::PlaySound::at("murk_death", root).volume(0.4).pitch(1.4));
                        debug!("murk: spear: snapped, pulled free");
                    }
                    Some((k, _)) if t < HOLD_MOST => {
                        // (Hauled in, no faster than so fast toward it;
                        // thrusting away, hardly: you pull it out till it
                        // snaps. A rope holds you too.)
                        let p = k.body.pos;
                        let to = (root - p).normalize_or(Vec2::X);
                        let along = k.body.vel.dot(to);
                        let fighting = k.loco.rocketing() && k.loco.thrust().dot(-to) > 0.5;
                        let haul = if fighting { HAUL * HAUL_THRUSTING } else { HAUL };
                        if along < HAUL_TOP {
                            k.body.vel += to * (haul * dt).min(HAUL_TOP - along);
                        }
                        steer = Some((i, p, p.distance(root), true));
                        m.maw = (m.maw + dt * 2.5).min(1.0);
                        m.maw_dir = (p - m.core).normalize_or(m.maw_dir);
                        m.spear = Spear::Held { t: t + dt, i, motes };
                    }
                    _ => {
                        m.spear = loose;
                        debug!("murk: spear: let go");
                    }
                }
            }
            Spear::Miss { t, i, dir, len } => {
                let root = root_of(m.core, &m.tendrils[i]);
                let rest = REACH * (in_tendril[i].min(TENDRIL_MOTES) as f32 / TENDRIL_MOTES as f32).max(0.06);
                let len = if t < HANG { len } else { (len - REEL * dt).max(rest) };
                let twitch = if t < HANG { (t * 50.0).sin() * 3.0 * (1.0 - t / HANG) } else { 0.0 };
                steer = Some((i, root + dir * len + dir.perp() * twitch, len, true));
                m.spear = if t >= HANG && len <= rest + 0.5 { Spear::Idle } else { Spear::Miss { t: t + dt, i, dir, len } };
            }
            Spear::Feast { t } => {
                // (Held in it while it closes on you.)
                if let Some((k, h)) = me.as_mut() {
                    if t == 0.0 {
                        h.harm_by(FEAST_BITE, Harm::Void, || crate::death::Cause::new("The murk", "devoured by the murk"));
                        sounds.write(crate::sound::PlaySound::at("murk_death", m.core).volume(0.8).pitch(0.7));
                    }
                    let to = m.core - k.body.pos;
                    k.body.vel = k.body.vel * (1.0 - 6.0 * dt) + to * 4.0 * dt;
                }
                m.spear = if t + dt > FEAST { Spear::Idle } else { Spear::Feast { t: t + dt } };
            }
        }
        if m.spear == Spear::Idle {
            if !was_idle {
                m.breath = BREATHE;
                m.spear_next = SPEAR_EVERY.0 + (SPEAR_EVERY.1 - SPEAR_EVERY.0) * unit(&[m.seed, tick, 0x5E7]);
            }
            m.maw = (m.maw - dt * 1.5).max(0.0);
            // (What it fed out goes back into its mass.)
            for (i, count) in in_tendril.iter_mut().enumerate() {
                let mut extra = count.saturating_sub(TENDRIL_MOTES);
                *count -= extra;
                in_body += extra;
                for (n, o) in m.motes.iter_mut().enumerate() {
                    if extra == 0 {
                        break;
                    }
                    if o.role == Role::Tendril(i as u8) {
                        o.role = Role::Body;
                        o.a = unit(&[m.seed, tick, n as u64, 0x5E5]) * TAU;
                        o.b = unit(&[m.seed, tick, n as u64, 0x5E6]).sqrt();
                        extra -= 1;
                    }
                }
            }
        } else if !matches!(m.spear, Spear::Held { .. }) {
            m.maw = (m.maw - dt * 1.2).max(0.0);
        }
        // Its charge: coiled back, its maw opening your way; then at you,
        // turning after you a little, through you; settling after.
        m.charge_next -= dt;
        if let (Charge::Idle, Spear::Idle, Volley::Idle, Some((p, ..))) = (m.charge, m.spear, m.volley, you)
            && m.hunting
            && m.charge_next <= 0.0
            && m.scared <= 0.0
            && m.breath <= 0.0
            && may("charge")
            && (CHARGE_FROM.0..CHARGE_FROM.1).contains(&dist)
            && in_body as f32 > full_body as f32 * 0.5
        {
            m.charge = Charge::Coil { t: 0.0, dir: (p - m.core).normalize_or(Vec2::X) };
            for n in 0..6u64 {
                let a = (p - m.core).to_angle() + (unit(&[m.seed, tick, n, 0xCA]) - 0.5) * 2.2;
                let r = HOLLOW + 3.0 + unit(&[m.seed, tick, n, 0xCB]) * (r_body * 0.6 - HOLLOW).max(1.0);
                m.eyes.push(Eye { off: Vec2::from_angle(a) * r, age: 0.0, life: COIL + 1.6 });
            }
            sounds.write(crate::sound::PlaySound::at("murk_hiss", m.core).volume(1.0).pitch(0.45));
            debug!("murk: charge: coil, you {dist:.0} off");
        }
        match (m.charge, you) {
            (Charge::Idle, _) => m.coil = (m.coil - dt * 3.0).max(0.0),
            (Charge::Coil { t, dir }, Some((p, pv, ..))) => {
                // (Drawn back, gathering; its aim on where you're going.)
                let aim = (p + pv * 0.25 - m.core).normalize_or(dir);
                m.vel += (-aim * 35.0 - m.vel).clamp_length_max(220.0 * dt);
                m.coil = (t / COIL).min(1.0);
                m.maw = (m.maw + dt * 2.0).min(0.8);
                m.maw_dir = aim;
                if t + dt >= COIL {
                    let dv = aim * CHARGE_SPEED - m.vel;
                    pushed += dv;
                    m.charge = Charge::Rush { t: 0.0, dir: aim, hit: false };
                    sounds.write(crate::sound::PlaySound::at("murk_spear", m.core).volume(1.2).pitch(0.55));
                    sounds.write(crate::sound::PlaySound::at("murk_death", m.core).volume(0.6).pitch(0.5));
                    trauma.0 = (trauma.0 + 0.2).min(1.0);
                    debug!("murk: charge: rush");
                } else {
                    m.charge = Charge::Coil { t: t + dt, dir: aim };
                }
            }
            (Charge::Rush { t, dir, hit }, Some((p, ..))) => {
                m.coil = (m.coil - dt * 6.0).max(0.0);
                m.maw = 1.0;
                // (After you, a little, while you're ahead of it.)
                let to = p - m.core;
                let dir = if to.dot(dir) > 0.0 {
                    let d = (to.to_angle() - dir.to_angle() + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI;
                    Vec2::from_angle(d.clamp(-CHARGE_TURN * dt, CHARGE_TURN * dt)).rotate(dir)
                } else {
                    dir
                };
                m.vel = dir * CHARGE_SPEED;
                m.maw_dir = dir;
                let mut hit = hit;
                if let (false, Some((k, h))) = (hit, me.as_mut())
                    && k.body.pos.distance(m.core) < r_body * 0.6 + k.body.half.length()
                {
                    hit = true;
                    // (Flung out of its way, off to the side you're on; it
                    // runs on past.)
                    let side = if (k.body.pos - m.core).perp_dot(dir) > 0.0 { -dir.perp() } else { dir.perp() };
                    strike(k, h, CHARGE_HIT, dir * CHARGE_KNOCK * 0.6 + side * CHARGE_KNOCK * 0.8, "run down by the murk");
                    sounds.write(crate::sound::PlaySound::at("slam", k.body.pos).volume(0.8));
                    sounds.write(crate::sound::PlaySound::at("murk_tear", k.body.pos));
                    trauma.0 = (trauma.0 + 0.35).min(1.0);
                    debug!("murk: charge: through you");
                }
                let passed = to.dot(dir) < -r_body * 1.2;
                let walled = solid(&sim, m.core + dir * (r_body * 0.6 + 4.0));
                // (Through you: a moment more, then it stops.)
                let t = if hit { t.max(CHARGE_SECS - 0.2) } else { t };
                if t + dt > CHARGE_SECS || passed || walled {
                    if walled {
                        m.vel = -m.vel * 0.2;
                    }
                    m.charge = Charge::Idle;
                    m.breath = BREATHE;
                    m.charge_next = CHARGE_EVERY.0 + (CHARGE_EVERY.1 - CHARGE_EVERY.0) * unit(&[m.seed, tick, 0xCC]);
                    m.pulse = 0.4;
                    debug!("murk: charge: done");
                } else {
                    m.charge = Charge::Rush { t: t + dt, dir, hit };
                }
            }
            (_, None) => m.charge = Charge::Idle,
        }
        // A push: its motes go with it, its front more than its back; its
        // back thrown out behind (a jet).
        if pushed != Vec2::ZERO {
            let dir = pushed.normalize_or(Vec2::X);
            let jet = JET * (pushed.length() / 60.0).min(2.0);
            for o in m.motes.iter_mut().filter(|o| o.role == Role::Body) {
                let f = ((o.pos - m.core).dot(dir) / r_body).clamp(-1.0, 1.0);
                o.vel += pushed * (0.7 + 0.3 * f) - dir * jet * (-f).max(0.0);
            }
            m.vel += pushed;
        }
        // Its volley: shards knotted on its front, quivering; then shot,
        // one after another, at where you'll be.
        m.volley_next -= dt;
        if let (Volley::Idle, Spear::Idle, Charge::Idle, Some((p, ..))) = (m.volley, m.spear, m.charge, you)
            && m.hunting
            && m.volley_next <= 0.0
            && m.scared <= 0.0
            && m.breath <= 0.0
            && may("volley")
            && (VOLLEY_FROM.0..VOLLEY_FROM.1).contains(&dist)
            && in_body as f32 > full_body as f32 * 0.5
        {
            let n = VOLLEY.0 + (unit(&[m.seed, tick, 0xF1]) * (VOLLEY.1 - VOLLEY.0 + 1) as f32) as usize;
            let to = (p - m.core).normalize_or(Vec2::X);
            let mut front: Vec<(usize, f32)> = m.motes.iter().enumerate().filter(|(_, o)| o.role == Role::Body).map(|(i, o)| (i, (o.pos - m.core).dot(to))).collect();
            front.sort_by(|a, b| b.1.total_cmp(&a.1));
            for slot in 0..n.min(front.len() / SHARD_MOTES) {
                let id = m.shard_ids;
                m.shard_ids = m.shard_ids.wrapping_add(1);
                let mut mid = Vec2::ZERO;
                for (k, &(i, _)) in front[slot * SHARD_MOTES..(slot + 1) * SHARD_MOTES].iter().enumerate() {
                    let o = &mut m.motes[i];
                    o.role = Role::Shard(id);
                    o.a = unit(&[m.seed, tick, (slot * SHARD_MOTES + k) as u64, 0xF2]) * TAU;
                    mid += o.pos;
                }
                m.shards.push(Shard { id, pos: mid / SHARD_MOTES as f32, vel: m.vel, age: 0.0, held: true, slot });
                in_shard.insert(id, SHARD_MOTES);
                in_body -= SHARD_MOTES;
            }
            m.volley = Volley::Tell { t: 0.0 };
            sounds.write(crate::sound::PlaySound::at("murk_hiss", m.core).volume(0.9).pitch(1.7));
            debug!("murk: volley: knot of {n}");
        }
        let held = m.shards.iter().filter(|s| s.held).count();
        match (m.volley, you) {
            (Volley::Idle, _) => {}
            (Volley::Tell { t }, Some(_)) => {
                m.volley = if t + dt >= VOLLEY_TELL { Volley::Fire { next: 0.0 } } else { Volley::Tell { t: t + dt } };
            }
            (Volley::Fire { next }, Some((p, pv, ..))) => {
                let next = next - dt;
                if next <= 0.0
                    && let Some(sh) = m.shards.iter_mut().filter(|s| s.held).min_by_key(|s| s.slot)
                {
                    // (At where you'll be, a share of the way; a little
                    // off.)
                    let lead = p.distance(sh.pos) / SHARD_SPEED * SHARD_LEAD;
                    let aim = (p + pv * lead - sh.pos).normalize_or(Vec2::X);
                    let off = (unit(&[m.seed, tick, sh.id as u64, 0xF3]) - 0.5) * 2.0 * SHARD_SPREAD;
                    sh.vel = Vec2::from_angle(off).rotate(aim) * SHARD_SPEED;
                    sh.held = false;
                    let (id, v) = (sh.id, sh.vel);
                    for o in m.motes.iter_mut().filter(|o| o.role == Role::Shard(id)) {
                        o.vel = v;
                    }
                    sounds.write(crate::sound::PlaySound::at("murk_spear", sh.pos).volume(0.45).pitch(1.9));
                }
                m.volley = if held <= 1 && next <= 0.0 {
                    m.breath = BREATHE;
                    m.volley_next = VOLLEY_EVERY.0 + (VOLLEY_EVERY.1 - VOLLEY_EVERY.0) * unit(&[m.seed, tick, 0xF4]);
                    debug!("murk: volley: spent");
                    Volley::Idle
                } else {
                    Volley::Fire { next: if next <= 0.0 { VOLLEY_GAP } else { next } }
                };
            }
            (_, None) => {
                m.volley = Volley::Idle;
                for sh in &mut m.shards {
                    sh.held = false;
                }
            }
        }
        // The shards: knotted on its front, quivering; or flying, through
        // you, into rock, or gone.
        let to_you = you.map_or(Vec2::X, |y| (y.0 - m.core).normalize_or(Vec2::X));
        let knot = m.shards.iter().filter(|s| s.held).count().max(1) as f32;
        let mut spent: Vec<u16> = Vec::new();
        for sh in &mut m.shards {
            if in_shard.get(&sh.id).copied().unwrap_or(0) < 2 {
                spent.push(sh.id);
                continue;
            }
            if sh.held {
                let across = sh.slot as f32 - (knot - 1.0) * 0.5;
                let quiver = (t * 47.0 + sh.slot as f32 * 1.9).sin() * 0.8;
                let at = m.core + to_you * (r_body * 0.95 + 3.0 + quiver) + to_you.perp() * across * 2.6;
                sh.vel = (at - sh.pos) * 20.0;
                sh.pos += sh.vel * dt;
                continue;
            }
            sh.age += dt;
            let was = sh.pos;
            sh.pos += sh.vel * dt;
            if let Some((k, h)) = me.as_mut() {
                let ab = sh.pos - was;
                let f = ((k.body.pos - was).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
                if ((was + ab * f - k.body.pos).abs() - k.body.half).max_element() < 1.5 {
                    // (Hurt, a nudge, no stun: you can still get out of
                    // the way of the next.)
                    h.harm_by(SHARD_HIT, Harm::Void, || crate::death::Cause::new("The murk", "shot through by the murk"));
                    debug!("murk: shot through: {SHARD_HIT:.1}");
                    k.body.vel += sh.vel.normalize_or(Vec2::X) * 25.0;
                    sounds.write(crate::sound::PlaySound::at("murk_tear", sh.pos).volume(0.6).pitch(1.5));
                    spent.push(sh.id);
                    continue;
                }
            }
            if sh.age > SHARD_SECS || solid(&sim, sh.pos) {
                spent.push(sh.id);
            }
        }
        if !spent.is_empty() {
            m.shards.retain(|s| !spent.contains(&s.id));
            for (i, o) in m.motes.iter_mut().enumerate() {
                if let Role::Shard(id) = o.role
                    && spent.contains(&id)
                {
                    o.role = Role::Loose;
                    o.life = 0.3 + 0.4 * unit(&[m.seed, tick, i as u64, 0xF5]);
                    o.vel = o.vel * 0.2 + Vec2::from_angle(unit(&[m.seed, tick, i as u64, 0xF6]) * TAU) * 40.0;
                }
            }
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
            let (mut len, mut seg) = (len, seg);
            let mut direct = false;
            let spearing = steer.is_some_and(|s| s.0 == i);
            if let Some((_, tip, l, d)) = steer.filter(|_| spearing) {
                target = tip;
                len = l.max(1.0);
                seg = len / SEGS as f32;
                direct = d;
                speed = 260.0;
                td.lash = Lash::Idle;
            }
            match (td.lash, you) {
                _ if spearing => {}
                (Lash::Idle, Some((p, ..))) if td.rest <= 0.0 && m.lashed <= 0.0 && !matches!(m.spear, Spear::Held { .. }) && share > 0.5 && p.distance(root) < len * 1.05 + 6.0 => {
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
            let was = td.pts[SEGS];
            let tip = if direct { target } else { was + (target - was).clamp_length_max(speed * dt) };
            td.tip_vel = (tip - was) / dt;
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
                // (In its feast: held, not flung off; half as hard, all
                // at once.)
                let feast = matches!(m.spear, Spear::Feast { .. });
                let push = if feast { Vec2::ZERO } else { (td.pts[SEGS] - td.pts[SEGS - 2]).normalize_or(Vec2::X) * LASH_KNOCK };
                strike(k, h, if feast { LASH_HIT * 0.5 } else { LASH_HIT }, push, "lashed by the murk");
                sounds.write(crate::sound::PlaySound::at("slam", td.pts[SEGS]).volume(0.4));
            }
        }
        // Clumps: torn off the side toward you and thrown.
        m.next_clump -= dt;
        if let (true, Some((p, ..))) = (m.hunting, you)
            && m.next_clump <= 0.0
            && m.clumps.len() < CLUMPS_MOST
            && m.charge == Charge::Idle
            && may("clump")
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
        let alive = in_body + in_tendril.iter().sum::<usize>() + in_clump.values().sum::<usize>() + in_shard.values().sum::<usize>();
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
        let shards: HashMap<u16, (Vec2, Vec2)> = m.shards.iter().map(|s| (s.id, (s.pos, s.vel))).collect();
        let spearing = m.spear.tendril();
        let tendrils: Vec<([Vec2; SEGS + 1], f32, Vec2, bool)> = m.tendrils.iter().enumerate().map(|(i, td)| (td.pts, (in_tendril[i] as f32 / TENDRIL_MOTES as f32).clamp(0.0, 1.0), td.tip_vel, spearing == Some(i))).collect();
        // (Its tell: its mass bunched your way. Its maw: what's in front of
        // its heart parted to either side.)
        // (Coiled to charge: drawn back away from you.)
        let lean = you.map_or(Vec2::ZERO, |y| (y.0 - m.core).normalize_or(Vec2::X) * r_body * 0.3 * (m.lean - 0.8 * m.coil));
        let (maw_at, gap) = (m.maw_dir.to_angle(), 0.8 * m.maw);
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
                        let mut a = o.a + t * spin;
                        if gap > 0.0 {
                            let d = (a - maw_at + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI;
                            if d.abs() < gap {
                                a = maw_at + d.signum() * (gap + (gap - d.abs()) * 0.3);
                            }
                        }
                        let home = m.core + lean * o.b + Vec2::from_angle(a) * (HOLLOW + o.b * (edge(r_body, a, t, seedf) - HOLLOW).max(1.0));
                        (home - o.pos) * 7.0 - (o.vel - m.vel) * 2.6 + swirl * 26.0
                    }
                    Role::Tendril(i) => {
                        let (pts, share, tip_vel, stiff) = &tendrils[i as usize];
                        let (c, d) = along(pts, o.a);
                        let width = (3.2 * (1.0 - o.a) + 0.7) * share.max(0.5);
                        let home = c + d.perp() * o.b * width;
                        // (Keeping up with its tip: a spear's hard.)
                        let (k, d) = if *stiff { (170.0, 24.0) } else { (55.0, 9.0) };
                        (home - o.pos) * k - (o.vel - *tip_vel * o.a) * d + swirl * 10.0
                    }
                    Role::Shard(id) => match shards.get(&id) {
                        Some(&(c, v)) => {
                            let home = c + Vec2::from_angle(o.a + t * 9.0) * 0.8;
                            (home - o.pos) * 220.0 - (o.vel - v) * 22.0
                        }
                        None => Vec2::ZERO,
                    },
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
                debug!("murk: smothered: {:.2}", SMOTHER * dt);
            }
        }
        m.motes.retain(|o| o.role != Role::Loose || o.life > 0.0);
        // Its eyes: opening here and there in it, more when it hunts.
        // (Round its maw.)
        if m.maw > 0.5 && m.eyes.len() < 14 && tick.is_multiple_of(4) {
            let side = if tick.is_multiple_of(8) { 1.0 } else { -1.0 };
            let a = m.maw_dir.to_angle() + side * (0.8 * m.maw + 0.15 + 0.25 * unit(&[m.seed, tick, 0xE5]));
            let r = r_body * (0.35 + 0.4 * unit(&[m.seed, tick, 0xE6]));
            m.eyes.push(Eye { off: Vec2::from_angle(a) * r, age: 0.0, life: 0.8 + unit(&[m.seed, tick, 0xE7]) });
        }
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
        // (Gathering itself, it's quiet.)
        if matches!(m.spear, Spear::Tell { .. }) {
            m.whisper = m.whisper.max(1.0);
        }
        if m.whisper <= 0.0 && dist < 380.0 {
            m.whisper = 2.2 + 2.0 * unit(&[m.seed, tick, 0x77]);
            let v = 0.25 + 0.15 * m.clumps.len() as f32;
            sounds.write(crate::sound::PlaySound::at("murk_whisper", m.core).volume(v));
        }
    }
    murks.numbers.extend(numbers);
    // Rubble eaten.
    for p in eaten {
        sim.world.set(p, platypus_sim::Cell::AIR);
    }
    // Where it's thick, for spells.
    let Murks { list, field, .. } = &mut *murks;
    field.clear();
    for m in list.iter().filter(|m| m.dying.is_none()) {
        // (The line it holds you by is in the way of every shot you fire
        // from its end: they run along it, into its maw.)
        let line = if let Spear::Held { i, .. } = m.spear { Some(Role::Tendril(i as u8)) } else { None };
        for o in m.motes.iter().filter(|o| o.role != Role::Loose && Some(o.role) != line) {
            *field.entry(bin(o.pos)).or_default() += 1;
        }
    }
}

/// A blow's flare and ripple last so long (s); the motes this far round
/// where it landed (cells, and this much more a point of damage) are
/// thrown back from it this hard (cells/s), drifting free a moment: a
/// dent in it, filling back in.
const IMPACT_LIFE: f32 = 0.34;
const RIPPLE_R: (f32, f32) = (8.0, 0.3);
const RIPPLE_KICK: f32 = 110.0;
const RIPPLE_STUN: f32 = 0.16;
/// A blow shoves the whole of it a little: cells/s a point of damage, at
/// most this much.
const SHOVE: (f32, f32) = (0.2, 6.0);
/// What a blow landing shows at most at once.
const IMPACTS: usize = 24;

/// A shot or blast in it: the motes it meets gone (a tendril cut through
/// there: what's past the cut drifts off), a blast's scattering the rest;
/// its heart, if it got there. Each shows: a flare and a ripple where it
/// landed, the motes round it thrown back, a spray of it, a wet tear of a
/// sound. What it did, to float up: where, how much, at its heart.
fn take(m: &mut Murk, h: &MurkHit, sparks: &mut crate::vfx::Sparks, sounds: &mut MessageWriter<crate::sound::PlaySound>, tick: u64) -> Option<(Vec2, f32, bool)> {
    let reach = body_radius(m.most) + REACH + h.blast * BLAST_SCATTERS + 20.0;
    let clumped = m.clumps.iter().any(|c| c.pos.distance(h.at) < 20.0 + h.blast * BLAST_SCATTERS);
    if h.at.distance(m.core) > reach && !clumped {
        return None;
    }
    let mut heart = 0.0;
    if h.at.distance(m.core) < CORE_R + 1.5 + h.blast * 0.5 {
        heart = h.damage + h.blast * 2.0;
        m.hp -= heart;
        m.hurt = 1.0;
        m.scared = m.scared.max(0.8);
        m.scared_from = h.at - h.dir * 10.0;
        sounds.write(crate::sound::PlaySound::at("murk_death", h.at).volume(0.35).pitch(1.6));
    }
    let r = SHOT_R.0 + h.damage * SHOT_R.1;
    // (Not the line it holds you by: what you fire runs along it.)
    let line = if let Spear::Held { i, .. } = m.spear { Some(Role::Tendril(i as u8)) } else { None };
    let mut near: Vec<(usize, f32)> = m.motes.iter().enumerate().filter(|(_, o)| o.role != Role::Loose && Some(o.role) != line).map(|(i, o)| (i, o.pos.distance(h.at))).filter(|(_, d)| *d < r.max(h.blast * BLAST_SCATTERS)).collect();
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
        sparks.emit(&TEAR, taken.clamp(6, 40), h.at, -h.dir, Vec2::ZERO);
        m.hurt = m.hurt.max(0.45);
    }
    if taken == 0 && heart <= 0.0 {
        return None;
    }
    // (A blow, not a blast: it dents it, and shoves it.)
    if h.blast <= 0.0 {
        let rr = RIPPLE_R.0 + h.damage * RIPPLE_R.1;
        for o in m.motes.iter_mut().filter(|o| !matches!(o.role, Role::Loose | Role::Shard(_))) {
            let d = o.pos.distance(h.at);
            if d < rr {
                let f = 1.0 - d / rr;
                let out = (o.pos - h.at).normalize_or(h.dir);
                o.vel += (out * 0.75 + h.dir * 0.45) * RIPPLE_KICK * f;
                o.stun = o.stun.max(RIPPLE_STUN * f);
            }
        }
        if h.shown {
            m.vel += h.dir * (h.damage * SHOVE.0).min(SHOVE.1);
        }
    }
    // (A spray of it back the way the blow came: bright flecks.)
    if h.shown {
        let n = (8.0 + h.damage * 0.7).min(34.0) as usize;
        sparks.emit(if heart > 0.0 { &HEART_SPLASH } else { &SPLASH }, n, h.at, -h.dir, Vec2::ZERO);
        let loud = (0.45 + h.damage / 40.0).min(1.0);
        sounds.write(crate::sound::PlaySound::at("murk_hit", h.at).volume(loud).pitch(if heart > 0.0 { 0.7 } else { 1.0 }));
    }
    if m.impacts.len() >= IMPACTS {
        m.impacts.remove(0);
    }
    m.impacts.push(Impact { at: h.at, dir: h.dir, power: if h.blast > 0.0 { h.blast } else { h.damage }, age: 0.0, heart: heart > 0.0 });
    match () {
        _ if heart > 0.0 => Some((h.at, heart, true)),
        _ if h.shown => Some((h.at, h.damage, false)),
        _ => None,
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
/// Its edge's glow: how far out (cells), how much rim near makes it full,
/// how strong.
const GLOW_R: f32 = 5.0;
const GLOW_FULL: f32 = 22.0;
const GLOW: f32 = 0.32;

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
        // (Not its shards: they're drawn as themselves.)
        let live = m.motes.iter().filter(|o| !matches!(o.role, Role::Loose | Role::Shard(_)));
        let (lo, hi) = live.clone().fold((Vec2::MAX, Vec2::MIN), |(lo, hi), o| (lo.min(o.pos), hi.max(o.pos)));
        if lo.x > hi.x {
            continue;
        }
        // (Room round it for its glow.)
        let lo = lo.floor().as_ivec2() - IVec2::splat(2 + GLOW_R as i32);
        let hi = hi.floor().as_ivec2() + IVec2::splat(2 + GLOW_R as i32);
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
        // (Its edge: in it, the outside within two cells.)
        let at = |x: usize, y: usize, dx: i32, dy: i32| {
            let (x, y) = (x as i32 + dx, y as i32 + dy);
            if x < 0 || y < 0 || x as usize >= w || y as usize >= h { 0 } else { thick[y as usize * w + x as usize] }
        };
        let rim: Vec<u8> = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                u8::from(thick[i] >= 3 && [(2, 0), (-2, 0), (0, 2), (0, -2), (1, 1), (-1, -1), (1, -1), (-1, 1)].iter().any(|&(dx, dy)| at(x, y, dx, dy) < 3))
            })
            .collect();
        // (Its glow: how much rim is near, out past it, a soft violet.)
        let gr = GLOW_R as usize;
        let mut rows = vec![0u16; w * h];
        for y in 0..h {
            for x in 0..w {
                rows[y * w + x] = (x.saturating_sub(gr)..(x + gr + 1).min(w)).map(|i| rim[y * w + i] as u16).sum();
            }
        }
        let glow: Vec<u16> = (0..w * h).map(|i| (i / w).saturating_sub(gr)..(i / w + gr + 1).min(h)).enumerate().map(|(i, ys)| ys.map(|j| rows[j * w + i % w]).sum()).collect();
        // (Brighter hunting, brightest charging; breathing.)
        let hot = if m.dying.is_some() { 0.0 } else { 0.6 + 0.25 * f32::from(m.hunting) + 0.5 * m.coil.max(f32::from(matches!(m.charge, Charge::Rush { .. }))) };
        let breathe = 0.8 + 0.2 * (t * 2.3 + m.seed as f32).sin();
        // (Struck just now: the mass round the wound flares pale, fading
        // as it fills back in.)
        let lit: Vec<(Vec2, f32, f32, bool)> = m
            .impacts
            .iter()
            .map(|i| {
                let f = i.age / IMPACT_LIFE;
                let r = if i.heart { 16.0 } else { (4.0 + i.power * 0.45).min(15.0) };
                (i.at, r, (1.0 - f).powi(2), i.heart)
            })
            .collect();
        let lit_at = |p: Vec2| {
            lit.iter().fold((0.0f32, false), |(k, gold), &(at, r, s, heart)| {
                let d = at.distance(p);
                if d >= r {
                    return (k, gold);
                }
                let here = (1.0 - d / r).powf(1.5) * s;
                if here > k { (here, heart) } else { (k, gold) }
            })
        };
        for y in 0..h {
            for x in 0..w {
                let (d, n) = (thick[y * w + x], here[y * w + x]);
                let (cx, cy) = (lo.x + x as i32, lo.y + y as i32);
                // (Only out past it: not in its gaps.)
                if d == 0 {
                    let g = glow[y * w + x];
                    if g > 0 {
                        let k = (g as f32 / GLOW_FULL).min(1.0);
                        let a = GLOW * k * k * hot * breathe + flash * 0.3 * k;
                        px.put(cx, cy, rgba([130.0 + 60.0 * flash, 64.0 + 50.0 * flash, 240.0], a.min(0.8)));
                    }
                    continue;
                }
                let shimmer = 0.5 + 0.5 * (cx as f32 * 0.7 + cy as f32 * 0.45 + t * 4.0).sin();
                let edge = rim[y * w + x] == 1;
                // (Inside: black, a dark sheen where a mote is (it moves in
                // there); its edge violet; past it, fuzz.)
                let (c, a): ([f32; 3], f32) = match (d, edge, n > 0) {
                    (3.., false, false) => ([2.0, 0.0, 5.0], 1.0),
                    (3.., false, true) => ([10.0 + 8.0 * shimmer, 3.0, 22.0 + 14.0 * shimmer], 1.0),
                    (3.., true, true) => ([120.0 + 60.0 * shimmer, 62.0 + 26.0 * shimmer, 205.0 + 50.0 * shimmer], 0.97),
                    (3.., true, false) => ([62.0 + 26.0 * shimmer, 28.0, 120.0 + 36.0 * shimmer], 0.95),
                    (_, _, true) => ([110.0 + 40.0 * shimmer, 60.0, 190.0 + 40.0 * shimmer], 0.55),
                    _ => continue,
                };
                // (Hurt: its edge flares.)
                let c = if edge && flash > 0.0 { [c[0] + (190.0 - c[0]) * flash, c[1] + (120.0 - c[1]) * flash, c[2] + (255.0 - c[2]) * flash] } else { c };
                let (k, gold) = if lit.is_empty() { (0.0, false) } else { lit_at(Vec2::new(cx as f32 + 0.5, cy as f32 + 0.5)) };
                let (c, a) = if k > 0.0 {
                    let to = if gold { [255.0, 228.0, 170.0] } else { [222.0, 190.0, 255.0] };
                    let k = (k * 0.9).min(0.9);
                    ([c[0] + (to[0] - c[0]) * k, c[1] + (to[1] - c[1]) * k, c[2] + (to[2] - c[2]) * k], a.max(k))
                } else {
                    (c, a)
                };
                px.put(cx, cy, rgba(c, a));
            }
        }
        // Where it was struck: a white-hot point, and a ripple racing out
        // through it (two, gold, at its heart), brighter in its thick.
        for i in &m.impacts {
            let f = i.age / IMPACT_LIFE;
            let (pale, deep) = if i.heart { ([255.0, 246.0, 220.0], [255.0, 190.0, 110.0]) } else { ([246.0, 236.0, 255.0], [196.0, 150.0, 255.0]) };
            if f < 0.3 {
                let r = (1.4 + i.power * 0.07).min(if i.heart { 5.0 } else { 3.5 });
                let a = 1.0 - f / 0.3;
                let ri = r.ceil() as i32;
                let c = i.at.floor().as_ivec2();
                for dy in -ri..=ri {
                    for dx in -ri..=ri {
                        let l = Vec2::new(dx as f32, dy as f32).length();
                        if l <= r {
                            px.put(c.x + dx, c.y + dy, rgba(pale, a * (1.0 - 0.4 * l / r)));
                        }
                    }
                }
            }
            let reach = if i.heart { 22.0 } else { (6.0 + i.power * 0.5).min(18.0) };
            let rings: &[f32] = if i.heart { &[1.0, 0.6] } else { &[1.0] };
            for &share in rings {
                let out = 1.0 - (1.0 - f).powi(3);
                let r = 1.5 + reach * share * out;
                let a = 0.9 * (1.0 - f).powf(1.5);
                let steps = (r * TAU * 1.4).ceil().max(8.0) as usize;
                for s in 0..steps {
                    let q = i.at + Vec2::from_angle(s as f32 / steps as f32 * TAU) * r;
                    let cell = q.floor().as_ivec2();
                    let inside = {
                        let at = cell - lo;
                        at.x >= 0 && at.y >= 0 && (at.x as usize) < w && (at.y as usize) < h && thick[at.y as usize * w + at.x as usize] >= 3
                    };
                    // (Leaning the way the blow went: brighter on its far side.)
                    let ahead = 0.75 + 0.25 * (q - i.at).normalize_or_zero().dot(i.dir);
                    let c = if inside { pale } else { deep };
                    px.put(cell.x, cell.y, rgba(c, a * ahead * if inside { 1.0 } else { 0.7 }));
                }
            }
        }
        if m.dying.is_some() {
            continue;
        }
        // Its shards: knotted, black spikes growing out of its front at
        // you, pale-tipped, quivering; shot, needles, a pale hot head and
        // a long violet streak.
        let grown = match m.volley {
            Volley::Tell { t } => (t / VOLLEY_TELL).min(1.0),
            _ => 1.0,
        };
        let to_you = you.map_or(Vec2::X, |y| (y - m.core).normalize_or(Vec2::X));
        for sh in &m.shards {
            let (dir, long) = if sh.held { (to_you, 3.0 + 10.0 * grown) } else { (sh.vel.normalize_or(Vec2::X), 0.0) };
            if sh.held {
                let base = sh.pos - dir * 3.0;
                let mut f = 0.0;
                while f <= long + 3.0 {
                    let q = base + dir * f;
                    let tip = f > long + 1.0;
                    let width = if f < 3.0 { 1.0 } else { 0.0 };
                    for side in [-1.0f32, 0.0, 1.0] {
                        if side != 0.0 && width == 0.0 {
                            continue;
                        }
                        let q = q + dir.perp() * side;
                        let (c, a) = if tip { ([230.0, 210.0, 255.0], 1.0) } else if side == 0.0 { ([6.0, 0.0, 14.0], 1.0) } else { ([120.0, 64.0, 220.0], 0.8) };
                        px.put(q.x.floor() as i32, q.y.floor() as i32, rgba(c, a));
                    }
                    f += 0.6;
                }
                continue;
            }
            for i in 0..26 {
                let q = sh.pos - dir * i as f32 * 0.8;
                let k = 1.0 - i as f32 / 26.0;
                let (c, a) = if i < 3 { ([244.0, 232.0, 255.0], 1.0) } else { ([140.0 + 60.0 * k, 70.0 + 40.0 * k, 240.0], 0.9 * k) };
                px.put(q.x.floor() as i32, q.y.floor() as i32, rgba(c, a));
                if i < 10 {
                    for side in [-1.0f32, 1.0] {
                        let s = q + dir.perp() * side;
                        px.put(s.x.floor() as i32, s.y.floor() as i32, rgba([110.0, 50.0, 210.0], 0.45 * k));
                    }
                }
            }
        }
        // Its heart, beating in its hollow (bigger, brighter, bared).
        let beat = (t * TAU * if m.hunting { 1.6 } else { 0.9 }).sin().max(0.0).powi(3);
        let bare = m.maw;
        let heart = [120.0 + 110.0 * beat + 60.0 * flash + 60.0 * bare, 18.0 + 50.0 * beat + 20.0 * bare, 66.0 + 80.0 * beat + 60.0 * flash];
        let heart = heart.map(|v| v.min(255.0));
        let r = CORE_R * (0.55 + 0.12 * beat + 0.3 * bare);
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
