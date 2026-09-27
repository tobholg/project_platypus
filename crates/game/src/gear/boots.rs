//! Rocket boots' exhaust (DESIGN §7c). While a creature's rocket boots fire
//! (`physics::Locomotion`, jump held in the air), what they spew goes into
//! the world: with `fire` boots, flames down out of the soles (most of them
//! only flame, burning out in the air; one in five an ember that falls on
//! and lights what it lands on, grass and wood; not when it's close over
//! the ground, landing, or you'd land in your own fire), and
//! whatever is in the jet under them is scorched and may catch; always a
//! puff of hot air, as a double jump's.

use bevy::prelude::*;
use platypus_sim::rng::Rng;
use platypus_sim::{Landing, MaterialId, Particle};

use crate::actors::elements::{Coated, Coatings, Resist, catch_fire};
use crate::actors::{Harm, Health, Kinematics, Rocketed};
use crate::hands::items::Items;
use crate::magic::runes::Emitter;
use crate::vfx::Sparks;
use crate::world::{SimWorld, TICK_HZ};

use super::Equipment;

/// How far down the jet reaches (cells), and how wide it opens (radians
/// either side).
const JET: f32 = 26.0;
const SPREAD: f32 = 0.3;
/// Fire damage a tick to what's in the jet, and its chance (/256) a tick to
/// catch fire.
const SCORCH: f32 = 0.6;
/// Closer than this to the ground (cells), no embers: you'd land in grass
/// your own boots lit.
const SETTLE_ABOVE: i32 = 20;
/// An ember's life (ticks): long enough to reach the ground.
const EMBER_LIFE: f32 = 90.0;
const CATCH: u8 = 24;

type Scorched<'a> = (Entity, &'a Kinematics, &'a mut Health, Option<&'a Resist>, Option<&'a Coated>);

/// Each tick a pair of boots fires: flames, a puff, and the jet's scorch.
#[allow(clippy::too_many_arguments)]
pub fn exhaust(
    mut commands: Commands,
    mut fired: MessageReader<Rocketed>,
    items: Option<Res<Items>>,
    coatings: Res<Coatings>,
    mut sim: ResMut<SimWorld>,
    mut sparks: ResMut<Sparks>,
    wearers: Query<&Equipment>,
    mut bodies: Query<Scorched>,
) {
    let Some(items) = items else { return };
    let fired: Vec<Rocketed> = fired.read().copied().collect();
    if fired.is_empty() {
        return;
    }
    let tick = sim.world.tick();
    for r in fired {
        // Under water: bubbles only, no flame (nothing burns there).
        let sole = platypus_sim::CellPos::from_world(r.at.x, r.at.y - 1.0);
        let wet = sim.world.get(sole).is_some_and(|c| sim.world.materials().phys(c.material).kind == platypus_sim::Kind::Liquid);
        let fire = !wet && wearers.get(r.entity).is_ok_and(|eq| eq.pieces(&items).any(|s| items.def(s.item).gear.as_ref().and_then(|g| g.rocket.as_ref()).is_some_and(|rd| rd.fire)));
        // The hot air, always (thinner than a double jump's cloud): under
        // water, bubbles.
        sparks.emit(&PUFF, PUFF.count as usize, r.at, Vec2::NEG_Y, r.vel * 0.3);
        if !fire {
            continue;
        }
        sparks.emit(&FLAME, FLAME.count as usize, r.at, Vec2::NEG_Y, r.vel * 0.5);
        let mut rng = Rng::seeded(&[tick, r.entity.to_bits(), 0xB0075]);
        let unit = |rng: &mut Rng| rng.next_u32() as f32 / u32::MAX as f32;
        let world = &mut sim.world;
        let mats = world.materials().clone();
        let flame = mats.fire();
        // Low over the ground (landing), the flames only flash: you'd land
        // in your own fire.
        let feet = platypus_sim::CellPos::from_world(r.at.x, r.at.y);
        let low = (1..=SETTLE_ABOVE).any(|d| world.is_solid(platypus_sim::CellPos::new(feet.x, feet.y - d)));
        if flame != MaterialId::AIR {
            for i in 0..5 {
                let a = (unit(&mut rng) - 0.5) * 2.0 * SPREAD;
                let v = Vec2::from_angle(a).rotate(Vec2::NEG_Y) * (180.0 + 120.0 * unit(&mut rng)) / TICK_HZ as f32 + r.vel / TICK_HZ as f32 * 0.5;
                let at = r.at - Vec2::Y * 2.0;
                // Mostly flame that burns out in the air; one in five an
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
        // What's in the jet under it is scorched.
        for (e, k, mut h, resist, coated) in &mut bodies {
            if e == r.entity {
                continue;
            }
            let d = k.body.pos - r.at;
            let down = -d.y;
            if down < -k.body.half.y || down > JET + k.body.half.y || d.x.abs() > k.body.half.x + 4.0 + down.max(0.0) * SPREAD {
                continue;
            }
            h.harm(SCORCH, Harm::Fire);
            if rng.chance(CATCH) {
                catch_fire(&mut commands, e, resist, coated, &coatings);
            }
        }
    }
}

/// The jet's flame: bright at the sole, orange, red, gone.
static FLAME: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 6.0,
    life: (0.06, 0.22),
    colors: vec![(255, 250, 210), (255, 200, 80), (255, 110, 30), (150, 40, 20)],
    speed: 220.0,
    spread: SPREAD,
    gravity: -80.0,
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
    speed: 70.0,
    spread: 0.9,
    gravity: -20.0,
    drag: 4.0,
    size: 1.0,
    jitter: 0.0,
    glow: false,
});
