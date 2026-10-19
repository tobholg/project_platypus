//! Storm boots: a double jump that kicks off a storm. The cloud a double
//! jump leaves (`vfx::air_puffs`) is a thundercloud for them: dark,
//! lit violet from inside, flickering; out of it, in two volleys a breath
//! apart, bolts leap: at the wearer's foes near (the nearest first, below
//! before above, with a clear line), or with none, fanning down into the
//! ground. Each bolt hurts and stuns what it ends on (storm), and what it
//! lands in that carries a charge (a pool: all of it) shocks what's in it.
//! Never the wearer. Walking, their soles spark now and then.

use bevy::prelude::*;
use platypus_sim::CellPos;
use serde::Deserialize;

use super::Equipment;
use crate::creatures::{Harm, Kinematics, Team};
use crate::hands::items::Items;
use crate::magic::runes::Emitter;
use crate::vfx::Sparks;
use crate::world::SimWorld;

/// Storm boots' storm: bolts a double jump looses (its first volley; the
/// second, half), how far they reach (cells), what each does (damage,
/// stun seconds).
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct StormDef {
    pub bolts: u8,
    pub reach: f32,
    pub damage: f32,
    pub stun: f32,
}

impl Default for StormDef {
    fn default() -> Self {
        StormDef { bolts: 4, reach: 95.0, damage: 22.0, stun: 0.35 }
    }
}

/// The storm of what's worn, if any.
pub fn storm_of<'a>(eq: &'a Equipment, items: &'a Items) -> Option<&'a StormDef> {
    eq.pieces(items).find_map(|s| items.def(s.item).gear.as_ref()?.storm.as_ref())
}

/// A storm under way: whose, where its cloud is, how long since, the
/// volleys loosed.
#[derive(Component)]
struct Burst {
    who: Entity,
    at: Vec2,
    t: f32,
    loosed: u8,
    def: StormDef,
}

/// The volleys: when (s after the jump), and what share of its bolts.
const VOLLEYS: [(f32, f32); 2] = [(0.04, 1.0), (0.17, 0.5)];
/// Bolts leap from inside the cloud, this far round its middle (cells).
const CLOUD: Vec2 = Vec2::new(7.0, 3.5);
/// A bolt with no foe to go for: down into the ground, this far (cells,
/// from..to), fanned this wide either side of straight down (radians).
const EARTH: (f32, f32) = (45.0, 90.0);
const FAN: f32 = 1.15;
/// What a charged pool does to what's in it, of a bolt's.
const SHOCK: f32 = 0.6;

pub fn build(app: &mut App) {
    app.add_systems(Update, (start, volleys, soles));
}

/// A double jump in storm boots: its storm begins (the cloud's own look is
/// `vfx::air_puffs`').
fn start(mut commands: Commands, mut jumps: MessageReader<crate::creatures::AirJumped>, items: Option<Res<Items>>, worn: Query<&Equipment>, mut sounds: MessageWriter<crate::sound::PlaySound>) {
    let Some(items) = items else { return };
    for j in jumps.read() {
        let Some(def) = worn.get(j.who).ok().and_then(|eq| storm_of(eq, &items)) else { continue };
        commands.spawn(Burst { who: j.who, at: j.at - Vec2::Y * 2.0, t: 0.0, loosed: 0, def: def.clone() });
        sounds.write(crate::sound::PlaySound::at("zap", j.at).pitch(1.25).volume(0.8));
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn volleys(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<SimWorld>,
    mut bursts: Query<(Entity, &mut Burst)>,
    bodies: Query<(Entity, &Kinematics, &Team), With<crate::creatures::Health>>,
    mut images: ResMut<Assets<Image>>,
    (mut hits, mut sounds, mut flares, mut sparks, mut trauma): (MessageWriter<crate::combat::Hit>, MessageWriter<crate::sound::PlaySound>, MessageWriter<crate::fx::Flare>, ResMut<Sparks>, ResMut<crate::fx::Trauma>),
) {
    for (e, mut b) in &mut bursts {
        b.t += time.delta_secs();
        let Some(&(when, share)) = VOLLEYS.get(b.loosed as usize) else {
            commands.entity(e).despawn();
            continue;
        };
        if b.t < when {
            continue;
        }
        b.loosed += 1;
        let side = bodies.get(b.who).map_or(Team::Player, |(_, _, t)| *t);
        let n = ((b.def.bolts as f32 * share).round() as usize).max(1);
        let seed = (b.at.x.to_bits() as u64) ^ ((b.at.y.to_bits() as u64) << 17) ^ b.loosed as u64;
        let roll = |k: u64| (platypus_sim::rng::hash(&[seed, k]) % 10_000) as f32 / 10_000.0;
        // Its foes near, nearest first (below before above), with a clear
        // line from the cloud.
        let mut foes: Vec<(Entity, Vec2, f32)> = bodies
            .iter()
            .filter(|(f, _, t)| *f != b.who && !t.allied(side) && **t != Team::Neutral)
            .map(|(f, k, _)| (f, k.body.pos, k.body.half.max_element()))
            .filter(|(_, p, _)| p.distance(b.at) < b.def.reach && crate::creatures::moves::clear(&sim, b.at, *p))
            .collect();
        foes.sort_by(|x, y| {
            let score = |p: Vec2| p.distance(b.at) + if p.y > b.at.y { 25.0 } else { 0.0 };
            score(x.1).total_cmp(&score(y.1))
        });
        let mut struck: Vec<Entity> = Vec::new();
        let mut charged: Vec<CellPos> = Vec::new();
        for i in 0..n {
            let from = b.at + Vec2::new((roll(i as u64 * 3) - 0.5) * 2.0 * CLOUD.x, (roll(i as u64 * 3 + 1) - 0.5) * 2.0 * CLOUD.y);
            // (Each foe takes a bolt or two, the nearest first; the rest
            // go to ground.)
            let target = if i < foes.len() * 2 { foes.get(i % foes.len()) } else { None };
            let to = match target {
                Some(&(_, p, _)) => p,
                None => {
                    let a = (roll(i as u64 * 3 + 2) - 0.5) * 2.0 * FAN;
                    from + Vec2::new(a.sin(), -a.cos()) * (EARTH.0 + (EARTH.1 - EARTH.0) * roll(i as u64 * 7 + 5))
                }
            };
            let zap = sim.world.spark(CellPos::from_world(from.x, from.y), CellPos::from_world(to.x, to.y));
            let end = Vec2::new(zap.to.x as f32 + 0.5, zap.to.y as f32 + 0.5);
            // (It got there: what it ends on is struck.)
            if let Some(&(f, p, r)) = target
                && end.distance(p) <= r + 4.0
                && !struck.contains(&f)
            {
                let dir = (p - from).normalize_or(Vec2::NEG_Y);
                hits.write(crate::combat::Hit { target: f, damage: b.def.damage, harm: Harm::Storm, knock: dir * 120.0, stun: b.def.stun, at: end, dir, weight: 0.4, crit: false });
                struck.push(f);
            }
            charged.extend(zap.charged.iter().copied());
            crate::fx::spawn_bolt(&mut commands, &mut images, &zap);
            sparks.emit(&CRACKLE, CRACKLE.count as usize, end, (end - from).normalize_or(Vec2::NEG_Y), Vec2::ZERO);
        }
        // A pool it landed in, all of it: what's in it is shocked (never
        // the wearer).
        if !charged.is_empty() {
            let cells: std::collections::HashSet<CellPos> = charged.into_iter().collect();
            for (f, k, t) in &bodies {
                if f == b.who || t.allied(side) || struck.contains(&f) {
                    continue;
                }
                let (lo, hi) = ((k.body.pos - k.body.half).floor().as_ivec2(), (k.body.pos + k.body.half).floor().as_ivec2());
                if (lo.y..=hi.y).any(|y| (lo.x..=hi.x).any(|x| cells.contains(&CellPos::new(x, y)))) {
                    hits.write(crate::combat::Hit { target: f, damage: b.def.damage * SHOCK, harm: Harm::Storm, knock: Vec2::ZERO, stun: b.def.stun, at: k.body.pos, dir: Vec2::Y, weight: 0.2, crit: false });
                }
            }
        }
        // The cloud lit from inside as they leap (not a flash that blinds:
        // under a full one).
        flares.write(crate::fx::Flare { at: b.at, color: [0.78, 0.72, 0.98], life: 0.28 });
        sparks.emit(&STORM_GLOW, STORM_GLOW.count as usize, b.at, Vec2::NEG_Y, Vec2::ZERO);
        trauma.0 = (trauma.0 + if b.loosed == 1 { 0.14 } else { 0.06 }).min(1.0);
        if b.loosed == 2 {
            sounds.write(crate::sound::PlaySound::at("zap", b.at).pitch(1.6).volume(0.5));
        }
    }
}

/// Walking in storm boots, the soles spark now and then.
fn soles(time: Res<Time>, items: Option<Res<Items>>, worn: Query<(&Equipment, &Kinematics)>, mut sparks: ResMut<Sparks>, mut clock: Local<f32>) {
    let Some(items) = items else { return };
    *clock += time.delta_secs();
    if *clock < 0.11 {
        return;
    }
    *clock = 0.0;
    for (eq, k) in &worn {
        if k.loco.grounded() && k.body.vel.x.abs() > 40.0 && storm_of(eq, &items).is_some() {
            let feet = k.body.pos - Vec2::new(k.body.vel.x.signum() * 2.0, k.body.half.y - 0.5);
            sparks.emit(&SOLE, SOLE.count as usize, feet, Vec2::new(-k.body.vel.x.signum(), 0.6), Vec2::ZERO);
        }
    }
}

/// Where a bolt ends: a spray of hot white and violet sparks.
static CRACKLE: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 7.0,
    life: (0.08, 0.25),
    colors: vec![(255, 255, 255), (210, 200, 255), (150, 130, 255)],
    speed: 140.0,
    spread: 1.3,
    gravity: -120.0,
    drag: 4.0,
    size: 1.0,
    jitter: 0.0,
    glow: true,
});

/// Inside the thundercloud as it flashes: motes glowing violet.
static STORM_GLOW: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 14.0,
    life: (0.12, 0.32),
    colors: vec![(235, 230, 255), (180, 160, 255), (130, 110, 240)],
    speed: 55.0,
    spread: 3.1,
    gravity: 0.0,
    drag: 5.0,
    size: 1.3,
    jitter: 5.0,
    glow: true,
});

/// A spark off a sole.
static SOLE: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.06, 0.16),
    colors: vec![(230, 225, 255), (160, 140, 255)],
    speed: 45.0,
    spread: 0.8,
    gravity: -60.0,
    drag: 4.0,
    size: 1.0,
    jitter: 0.0,
    glow: true,
});
