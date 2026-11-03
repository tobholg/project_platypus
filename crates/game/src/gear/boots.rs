//! Rocket boots' exhaust (DESIGN §7c), and how a body's drawn while they
//! fire. While a creature's rocket boots fire (`physics::Locomotion`, jump
//! held in the air), what they spew goes into the world, a jet out of each
//! boot (where its pose has its feet: `animation::Soles`; one under the
//! middle for a body without them), out of the soles whichever way they
//! point (`Lean`):
//! - in the air, a puff of hot air, and with `fire` boots flames (most of
//!   them only flame, burning out in the air; one in six an ember that
//!   falls on and lights what it lands on, grass and wood; not when it's
//!   close over the ground, landing, or you'd land in your own fire), and
//!   whatever is in the jet is scorched and may catch; low over the ground
//!   the jet kicks up what's there (dust off sand, spray off water);
//! - under water, bubbles;
//! - in space (no air), a wide white-blue plume, no smoke, no flame (but
//!   dust off the rock it blasts).
//!
//! Lit (`fx::Flare`) where they fire; a pop as they light, a puff as they
//! stop; nearly out (`SPUTTER`), they cough and flicker.
//!
//! `Lean`: weightless or under water a body's drawn turned the way its
//! boots push (its soles the nozzle: flying right it lies along it, head
//! first; pushing down it dives), easing there quickly, and back upright
//! as they stop (not spun about as it really would be: dizzying); in the
//! air it leans a little into the way it's going.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use platypus_sim::rng::Rng;
use platypus_sim::{CellPos, Kind, Landing, MaterialId, Particle};

use crate::creatures::body::animation::Soles;
use crate::creatures::body::elements::{Coated, Coatings, catch_fire};
use crate::creatures::{Harm, Health, Kinematics, Rocketed};
use crate::hands::items::Items;
use crate::magic::runes::Emitter;
use crate::vfx::Sparks;
use crate::world::{SimWorld, TICK_HZ};

use super::Equipment;

/// How far the jet reaches (cells), and how wide it opens (radians
/// either side).
const JET: f32 = 39.0;
const SPREAD: f32 = 0.3;
/// Fire damage a tick to what's in the jet, and its chance (/256) a tick to
/// catch fire.
const SCORCH: f32 = 0.6;
/// Closer than this to the ground (cells), no embers: you'd land in grass
/// your own boots lit.
const SETTLE_ABOVE: i32 = 30;
/// An ember's life (ticks): long enough to reach the ground.
const EMBER_LIFE: f32 = 90.0;
const CATCH: u8 = 24;
/// What the jet kicks up, this near what it blows on (cells).
const GROUND_EFFECT: i32 = 40;
/// Under this share of their fuel, they sputter.
const SPUTTER: f32 = 0.2;
/// How far into space (`sky::space_at`) there's no air for a flame.
const VACUUM: f32 = 0.5;

/// A body drawn turned by its boots (radians, counter-clockwise).
#[derive(Component, Default)]
pub struct Lean {
    pub angle: f32,
}

/// Easing to the way the boots push (a share of the way, a second's
/// worth); leaning in the air at most this far (radians), at this speed
/// sideways (cells/s).
const LEAN_EASE: f32 = 14.0;
const LEAN_MOST: f32 = 0.26;
const LEAN_AT: f32 = 300.0;

fn wrap(a: f32) -> f32 {
    (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// Turned as its boots push: see the module notes. (Recoil's spin,
/// `zerog::Turning`, gives way to them.)
pub fn lean(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &Kinematics, Option<&mut Lean>, Has<crate::zerog::Turning>, &mut Transform)>) {
    let dt = time.delta_secs();
    for (e, k, lean, turning, mut tf) in &mut q {
        let firing = k.loco.rocketing();
        if lean.is_none() && !firing {
            continue;
        }
        let t = k.loco.thrust();
        let target = if !firing {
            0.0
        } else if k.loco.floating() || k.loco.contacts.submerged > 0.5 {
            -t.x.atan2(t.y)
        } else {
            -(k.body.vel.x / LEAN_AT).clamp(-1.0, 1.0) * LEAN_MOST
        };
        let Some(mut lean) = lean else {
            commands.entity(e).insert(Lean::default());
            continue;
        };
        if firing && turning {
            commands.entity(e).remove::<crate::zerog::Turning>();
        }
        lean.angle = wrap(lean.angle + wrap(target - lean.angle) * (1.0 - (-dt * LEAN_EASE).exp()));
        if !firing && lean.angle.abs() < 0.01 {
            if !turning {
                tf.rotation = Quat::IDENTITY;
            }
            commands.entity(e).remove::<Lean>();
            continue;
        }
        tf.rotation = Quat::from_rotation_z(lean.angle);
    }
}

type Scorched<'a> = (Entity, &'a Kinematics, &'a mut Health, Option<&'a Coated>);

/// Last tick a pair of boots fired: where, which way out, in space.
pub struct Firing {
    tick: u64,
    at: Vec2,
    out: Vec2,
    vacuum: bool,
}

/// Each tick a pair of boots fires: its jet, and the jet's scorch; as they
/// light and as they stop.
#[allow(clippy::too_many_arguments)]
pub fn exhaust(
    mut commands: Commands,
    mut fired: MessageReader<Rocketed>,
    items: Option<Res<Items>>,
    coatings: Res<Coatings>,
    mut sim: ResMut<SimWorld>,
    mut sparks: ResMut<Sparks>,
    wearers: Query<&Equipment>,
    soles: Query<(&Kinematics, Option<&Soles>, Option<&Lean>)>,
    mut bodies: Query<Scorched>,
    mut flares: MessageWriter<crate::fx::Flare>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut firing: Local<HashMap<Entity, Firing>>,
) {
    let tick = sim.world.tick();
    // Stopped (last tick they fired, not this): a puff, the glow gone.
    let fired: Vec<Rocketed> = fired.read().copied().collect();
    firing.retain(|e, f| {
        let on = fired.iter().any(|r| r.entity == *e);
        if !on && f.tick + 1 < tick {
            sparks.emit(if f.vacuum { &VACUUM_PUFF } else { &CUTOFF }, 6, f.at, f.out, Vec2::ZERO);
            return false;
        }
        true
    });
    let Some(items) = items else { return };
    for r in fired {
        // Which way the soles point (out of them: the jet), and from where.
        let (k, sole, lean) = match soles.get(r.entity) {
            Ok(x) => x,
            Err(_) => continue,
        };
        let turn = Vec2::from_angle(lean.map_or(0.0, |l| l.angle));
        let out = turn.rotate(Vec2::NEG_Y);
        let jets: Vec<Vec2> = sole
            .and_then(|s| Some(vec![k.body.pos + turn.rotate(s.near?) + out * 0.5, k.body.pos + turn.rotate(s.far?) + out * 0.5]))
            .unwrap_or_else(|| vec![k.body.pos + turn.rotate(r.at - k.body.pos)]);
        let mid = jets.iter().copied().sum::<Vec2>() / jets.len() as f32;
        let sole_cell = CellPos::from_world(mid.x + out.x, mid.y + out.y);
        let wet = sim.world.get(sole_cell).is_some_and(|c| sim.world.materials().phys(c.material).kind == Kind::Liquid);
        let vacuum = !wet && crate::sky::space_at(&sim, mid.y) > VACUUM;
        let fire = !wet && !vacuum && wearers.get(r.entity).is_ok_and(|eq| eq.pieces(&items).any(|s| items.def(s.item).gear.as_ref().and_then(|g| g.rocket.as_ref()).is_some_and(|rd| rd.fire)));
        let mut rng = Rng::seeded(&[tick, r.entity.to_bits(), 0xB0075]);
        let unit = |rng: &mut Rng| rng.next_u32() as f32 / u32::MAX as f32;
        // Lit: a pop, a ring of it.
        let lit = !firing.contains_key(&r.entity);
        if lit {
            for &at in &jets {
                sparks.emit(if vacuum { &VACUUM_PUFF } else { &IGNITE }, 8, at, out, r.vel * 0.2);
            }
            sounds.write(crate::sound::PlaySound::at("rocket_light", mid).volume(0.6));
        }
        firing.insert(r.entity, Firing { tick, at: mid, out, vacuum });
        // Nearly out: it coughs, flickering (a tick in three dark).
        let sputter = r.fuel < SPUTTER;
        if sputter && rng.chance(18) {
            sounds.write(crate::sound::PlaySound::at("rocket_cough", mid).volume(0.5));
            sparks.emit(&SMOKE, 3, mid, out, r.vel * 0.3);
        }
        if sputter && rng.chance(85) {
            continue;
        }
        let share = |count: f32| (count / jets.len() as f32).ceil() as usize;
        flares.write(crate::fx::Flare { at: mid + out * 3.0, color: if vacuum { [0.45, 0.55, 0.8] } else { [0.7, 0.5, 0.28] }, life: 0.08 });
        if vacuum {
            for &at in &jets {
                sparks.emit(&PLUME, share(PLUME.count), at, out, r.vel * 0.5);
            }
            // (No air, but the jet still blasts dust off the rock it's on.)
            kick_up(&sim, &mut sparks, mid, out, &mut rng);
            continue;
        }
        // The hot air, always (thinner than a double jump's cloud): under
        // water, bubbles.
        for &at in &jets {
            sparks.emit(&PUFF, share(PUFF.count), at, out, r.vel * 0.3);
        }
        if !wet {
            kick_up(&sim, &mut sparks, mid, out, &mut rng);
        }
        if !fire {
            continue;
        }
        for &at in &jets {
            sparks.emit(&FLAME, share(FLAME.count), at, out, r.vel * 0.5);
        }
        let world = &mut sim.world;
        let mats = world.materials().clone();
        let flame = mats.fire();
        // Low over the ground (landing), the flames only flash: you'd land
        // in your own fire.
        let low = (1..=SETTLE_ABOVE).any(|d| world.is_solid(CellPos::from_world(mid.x + out.x * d as f32, mid.y + out.y * d as f32)));
        if flame != MaterialId::AIR {
            for i in 0..6 {
                let jet = jets[i % jets.len()];
                let a = (unit(&mut rng) - 0.5) * 2.0 * SPREAD;
                let v = Vec2::from_angle(a).rotate(out) * (270.0 + 180.0 * unit(&mut rng)) / TICK_HZ as f32 + r.vel / TICK_HZ as f32 * 0.5;
                let at = jet + out * 3.0;
                // Mostly flame that burns out in the air; one in six an
                // ember that falls on, lighting what it lands on if it burns
                // (an ember, not fire: flame left where it ran out of life
                // would hang in the air, and you'd fall back through it).
                let ember = i == 0 && !low;
                let landing = if ember { Landing::Ember } else { Landing::Vanish };
                let life = if ember { EMBER_LIFE } else { (0.12 + 0.18 * unit(&mut rng)) * TICK_HZ as f32 };
                let cell = mats.spawn(flame, &mut rng);
                world.emit(Particle { gravity: -0.05, ..Particle::new([at.x, at.y], [v.x, v.y], cell, life as u16 + 2, landing) });
            }
        }
        // What's in the jet is scorched.
        for (e, k, mut h, coated) in &mut bodies {
            if e == r.entity {
                continue;
            }
            let d = k.body.pos - mid;
            let along = d.dot(out);
            let across = (d - out * along).length();
            if along < -k.body.half.y || along > JET + k.body.half.y || across > k.body.half.x + 6.0 + along.max(0.0) * SPREAD {
                continue;
            }
            h.harm_by(SCORCH, Harm::Fire, || crate::death::Cause::new("Rocket boots", "scorched"));
            if rng.chance(CATCH) {
                catch_fire(&mut commands, e, &h, coated, &coatings, 0.3);
            }
        }
    }
}

/// Low over the ground the jet kicks up what it blows on: dust (its
/// colour) blown out to both sides along it, spray thrown up off water;
/// the nearer, the more.
fn kick_up(sim: &SimWorld, sparks: &mut Sparks, from: Vec2, out: Vec2, rng: &mut Rng) {
    let world = &sim.world;
    let mats = world.materials();
    let hit = (1..=GROUND_EFFECT).find_map(|d| {
        let p = from + out * d as f32;
        let c = world.get(CellPos::from_world(p.x, p.y))?;
        let kind = mats.phys(c.material).kind;
        matches!(kind, Kind::Static | Kind::Powder | Kind::Liquid).then_some((d, p, c, kind))
    });
    let Some((d, at, c, kind)) = hit else { return };
    let near = 1.0 - d as f32 / GROUND_EFFECT as f32;
    let colors = mats.def(c.material).colors.clone();
    if colors.is_empty() {
        return;
    }
    let liquid = kind == Kind::Liquid;
    let e = Emitter { colors, ..if liquid { SPRAY.clone() } else { DUST.clone() } };
    let n = (near * if liquid { 5.0 } else { 4.0 }).ceil() as usize;
    // (Back off it, out to each side: it blows along the surface.)
    let back = -out;
    let side = Vec2::new(-out.y, out.x);
    for s in [side, -side] {
        let dir = (s * 0.85 + back * if liquid { 0.9 } else { 0.25 }).normalize();
        if rng.coin() || near > 0.5 {
            sparks.emit(&e, n, at - out, dir, Vec2::ZERO);
        }
    }
}

/// The jet's flame: bright at the sole, orange, red, gone.
static FLAME: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 6.0,
    life: (0.06, 0.22),
    colors: vec![(255, 250, 210), (255, 200, 80), (255, 110, 30), (150, 40, 20)],
    speed: 330.0,
    spread: SPREAD,
    gravity: -120.0,
    drag: 3.0,
    size: 1.5,
    jitter: 0.0,
    glow: true,
});

/// The hot air around it.
static PUFF: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 2.0,
    life: (0.2, 0.5),
    colors: vec![(255, 255, 255), (220, 232, 255), (160, 180, 225)],
    speed: 105.0,
    spread: 0.9,
    gravity: -30.0,
    drag: 4.0,
    size: 1.0,
    jitter: 0.0,
    glow: false,
});

/// As they light: a quick ring of flame and hot air.
static IGNITE: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 8.0,
    life: (0.08, 0.2),
    colors: vec![(255, 250, 220), (255, 190, 90), (240, 110, 40)],
    speed: 140.0,
    spread: 1.6,
    gravity: 0.0,
    drag: 6.0,
    size: 1.0,
    jitter: 0.0,
    glow: true,
});

/// As they stop: a soft grey puff.
static CUTOFF: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 6.0,
    life: (0.25, 0.6),
    colors: vec![(220, 220, 225), (170, 170, 180), (120, 120, 130)],
    speed: 50.0,
    spread: 0.8,
    gravity: -20.0,
    drag: 3.0,
    size: 1.0,
    jitter: 0.0,
    glow: false,
});

/// Nearly out: sooty coughs.
static SMOKE: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 3.0,
    life: (0.3, 0.7),
    colors: vec![(110, 105, 100), (75, 72, 70), (50, 48, 46)],
    speed: 60.0,
    spread: 0.6,
    gravity: -25.0,
    drag: 3.0,
    size: 1.0,
    jitter: 0.0,
    glow: false,
});

/// In space: a wide plume, a white-blue core, no smoke.
static PLUME: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 7.0,
    life: (0.05, 0.16),
    colors: vec![(250, 252, 255), (180, 215, 255), (120, 160, 255), (255, 170, 90)],
    speed: 380.0,
    spread: 0.6,
    gravity: 0.0,
    drag: 1.5,
    size: 1.5,
    jitter: 0.0,
    glow: true,
});

/// In space, lighting and stopping: a faint blue-white flash.
static VACUUM_PUFF: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 6.0,
    life: (0.06, 0.18),
    colors: vec![(230, 240, 255), (150, 190, 255)],
    speed: 120.0,
    spread: 1.4,
    gravity: 0.0,
    drag: 4.0,
    size: 1.0,
    jitter: 0.0,
    glow: true,
});

/// What the jet kicks up (its colours are what it blew on): dust along
/// the ground, spray thrown off water.
static DUST: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 4.0,
    life: (0.25, 0.6),
    colors: vec![],
    speed: 150.0,
    spread: 0.35,
    gravity: 40.0,
    drag: 3.5,
    size: 1.0,
    jitter: 0.0,
    glow: false,
});
static SPRAY: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 5.0,
    life: (0.3, 0.6),
    colors: vec![],
    speed: 170.0,
    spread: 0.3,
    gravity: 260.0,
    drag: 1.0,
    size: 1.0,
    jitter: 0.0,
    glow: false,
});
