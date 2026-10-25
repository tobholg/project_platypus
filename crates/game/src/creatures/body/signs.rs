//! What's hurting the player, shown on it, not only in the status icons:
//! searing heat hazes and steams off it (and sizzles, and the screen's
//! edges glow orange: `screen_fx`); what it's coated in shows its `Look`
//! (venom bubbling and glowing faintly, acid fizzing and hissing, oil
//! dripping heavy, water dripping); and some of it reacts: oil shimmers a
//! warning near fire or heat, water steams in heat and sprays on landing,
//! and wet under a storm, sparks crawl over it (lightning hurts the wet
//! more: `elements::conducts`). The tint (pulsing, glinting) is
//! `elements::tint`'s, for every creature.

use bevy::prelude::*;
use platypus_sim::CellPos;

use crate::creatures::body::elements::{Burning, Coated, Coatings};
use crate::creatures::player::LocalPlayer;
use crate::creatures::{Harm, Kinematics, Took};
use crate::light::LightSource;
use crate::magic::runes::Emitter;
use crate::vfx::Sparks;
use crate::world::{SimWorld, TICK_HZ};

/// What it's felt lately, by kind (`Harm as usize`): 1 just now, as hard
/// as it was (at least `FELT_LEAST`), fading to 0 over `FELT_FOR`.
#[derive(Component, Default)]
pub struct Signs {
    felt: [f32; 11],
    /// Frames shown (what the rolls are seeded by).
    n: u64,
    /// Seconds to each coating's next sound, by name.
    sounds: Vec<(String, f32)>,
    sizzle: f32,
    /// Its faint light (venom's, acid's, heat's).
    light: Option<Entity>,
    /// It was on the ground last frame; how fast it was falling.
    ground: bool,
    fall: f32,
}

impl Signs {
    /// How much heat it's taking now (0..1): burning or searing.
    pub fn heat(&self) -> f32 {
        self.felt[Harm::Fire as usize]
    }
}

const FELT_FOR: f32 = 0.5;
const FELT_LEAST: f32 = 0.35;
/// Taking this much a second, it's felt fully.
const FELT_FULL: f32 = 20.0;
/// Searing heat: haze and steam off it (so many a second, fully), a sizzle
/// every so often, a faint light.
const HAZE: f32 = 22.0;
const STEAM: f32 = 7.0;
const SIZZLE_EVERY: f32 = 0.9;
const HEAT_GLOW: f32 = 0.05;
/// Wet in heat, it steams this much more (all of it wet); landing this fast
/// (cells/s), it sprays.
const WET_STEAM: f32 = 16.0;
const SPLASH_FALL: f32 = 70.0;
/// Oily, this near fire or heat (cells) it shimmers a warning.
const OIL_WARN: i32 = 6;
const OIL_SHIMMER: f32 = 16.0;
/// Wet at least this much under a severe storm with the sky above (this
/// many cells up), sparks crawl over it.
const STORM_WET: f32 = 0.3;
const STORM_SKY: i32 = 80;
const STORM_SPARKS: f32 = 12.0;

pub fn watch(mut commands: Commands, new: Query<Entity, (With<LocalPlayer>, Without<Signs>)>) {
    for e in &new {
        commands.entity(e).insert(Signs::default());
    }
}

type Shown<'a> = (Entity, &'a Kinematics, &'a mut Signs, Option<&'a Burning>, Option<&'a Coated>);

#[allow(clippy::too_many_arguments)]
pub fn show(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<SimWorld>,
    coatings: Res<Coatings>,
    mut took: MessageReader<Took>,
    mut sparks: ResMut<Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut player: Query<Shown, With<LocalPlayer>>,
    mut lights: Query<&mut LightSource>,
) {
    let Ok((me, k, mut signs, burning, coated)) = player.single_mut() else {
        took.clear();
        return;
    };
    let dt = time.delta_secs().min(0.05);
    for f in &mut signs.felt {
        *f = (*f - dt / FELT_FOR).max(0.0);
    }
    for t in took.read().filter(|t| t.target == me && t.dealt > 0.0) {
        let f = &mut signs.felt[t.harm as usize];
        *f = f.max((t.dealt * TICK_HZ as f32 / FELT_FULL).clamp(FELT_LEAST, 1.0));
    }
    signs.n = signs.n.wrapping_add(1);
    let (pos, half) = (k.body.pos, k.body.half);
    let carry = k.body.vel * 0.5;
    let n = signs.n;
    let roll = move |salt: u64| (platypus_sim::rng::hash(&[n, salt]) % 10_000) as f32 / 10_000.0;
    // So many a second, this frame (the odd one by chance).
    let many = |rate: f32, salt: u64| {
        let want = (rate * dt).max(0.0);
        want.floor() as usize + usize::from(roll(salt) < want.fract())
    };
    // Somewhere on it (its lower half: what drips).
    let on = |salt: u64, low: bool| {
        let y = if low { -roll(salt + 1) * 0.8 } else { roll(salt + 1) * 1.8 - 0.9 };
        pos + Vec2::new((roll(salt) - 0.5) * 1.6 * half.x, y * half.y)
    };
    let put = |sparks: &mut Sparks, e: &Emitter, rate: f32, salt: u64, low: bool, dir: Vec2| {
        for i in 0..many(rate, salt) {
            sparks.emit(e, 1, on(salt * 31 + i as u64 * 7, low), dir, carry);
        }
    };
    let rules = &coatings.by_name;
    let (wet, oily) = coated.map_or((0.0, 0.0), |c| (c.wet(rules).min(1.0), c.oily(rules).min(1.0)));
    let heat = signs.heat();
    let searing = if burning.is_some() { 0.0 } else { heat };
    let mut glow = [0.0f32; 3];
    let mut light = |(r, g, b): (u8, u8, u8), s: f32| {
        glow[0] += r as f32 / 255.0 * s;
        glow[1] += g as f32 / 255.0 * s;
        glow[2] += b as f32 / 255.0 * s;
    };

    // Searing heat: hazing, steaming, sizzling, glowing.
    if searing > 0.0 {
        put(&mut sparks, &HEAT_HAZE, HAZE * searing, 1, false, Vec2::Y);
        put(&mut sparks, &HEAT_STEAM, STEAM * searing, 2, false, Vec2::Y);
        light((255, 110, 40), HEAT_GLOW * searing);
        signs.sizzle -= dt;
        if signs.sizzle <= 0.0 {
            signs.sizzle = SIZZLE_EVERY;
            sounds.write(crate::sound::PlaySound::at("sizzle", pos));
        }
    } else {
        signs.sizzle = 0.0;
    }

    // Its coatings' looks: what comes off them, their light, their sounds.
    for (i, (name, a)) in coated.map_or(&[][..], |c| &c.coats[..]).iter().enumerate() {
        let Some(c) = rules.get(name) else { continue };
        let look = &c.look;
        for (j, s) in look.shed.iter().enumerate() {
            put(&mut sparks, &s.sparks, s.rate * a, 10 + i as u64 * 8 + j as u64, s.low, Vec2::new(s.dir.0, s.dir.1));
        }
        if look.glow > 0.0 {
            light(c.color, look.glow * a);
        }
        if let Some((sound, every, least)) = &look.sound {
            let left = match signs.sounds.iter_mut().find(|(n, _)| n == name) {
                Some((_, left)) => left,
                None => {
                    signs.sounds.push((name.clone(), 0.0));
                    &mut signs.sounds.last_mut().unwrap().1
                }
            };
            *left -= dt;
            if *a >= *least && *left <= 0.0 {
                *left = *every;
                sounds.write(crate::sound::PlaySound::at(sound.clone(), pos));
            }
        }
    }

    // Water in heat steams; landing hard, it sprays.
    if wet > 0.05 && heat > 0.0 {
        put(&mut sparks, &HEAT_STEAM, WET_STEAM * wet * heat, 60, false, Vec2::Y);
    }
    let grounded = k.loco.grounded();
    let landed = grounded && !signs.ground && signs.fall < -SPLASH_FALL;
    if landed && wet > 0.2 {
        let feet = pos - Vec2::Y * half.y;
        let n = (14.0 * wet) as usize;
        sparks.emit(&WET_SPRAY, n, feet, Vec2::new(-1.0, 0.4), Vec2::ZERO);
        sparks.emit(&WET_SPRAY, n, feet, Vec2::new(1.0, 0.4), Vec2::ZERO);
    }
    signs.ground = grounded;
    signs.fall = if grounded { 0.0 } else { k.body.vel.y };

    // Oil near fire or heat: a warning shimmer.
    if oily > 0.15 && burning.is_none() {
        let (lo, hi) = k.body.cells_at(pos);
        let e = sim.world.exposure(CellPos::new(lo.x - OIL_WARN, lo.y - OIL_WARN), CellPos::new(hi.x + OIL_WARN, hi.y + OIL_WARN));
        if e.heat > 0.0 || e.flames > 0.0 || e.ignites {
            put(&mut sparks, &OIL_WARNING, OIL_SHIMMER * oily, 70, false, Vec2::Y);
        }
    }

    // Wet under a storm: sparks crawling over it.
    if wet >= STORM_WET {
        let x = pos.x.floor() as i32;
        let storm = sim.world.weather().map_or(0.0, |w| w.severe_at(x));
        if storm > 0.0 && sim.world.under_sky(CellPos::new(x, (pos.y + half.y).ceil() as i32), STORM_SKY) {
            put(&mut sparks, &STORM_CRAWL, STORM_SPARKS * wet * storm, 80, false, Vec2::Y);
        }
    }

    // Its light: one, of all of them.
    let color = glow;
    match signs.light {
        Some(e) => {
            if let Ok(mut l) = lights.get_mut(e) {
                l.color = color;
            }
        }
        None if color.iter().any(|&c| c > 0.0) => {
            let e = commands.spawn((Name::new("Signs light"), Transform::default(), Visibility::default(), LightSource { color, flicker: 0.1 })).id();
            commands.entity(me).add_child(e);
            signs.light = Some(e);
        }
        None => {}
    }
}

/// Heat shimmering off it: pale motes wavering up.
static HEAT_HAZE: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.25, 0.6),
    colors: vec![(255, 236, 200), (255, 170, 110)],
    speed: 10.0,
    spread: 0.5,
    gravity: -45.0,
    drag: 2.0,
    size: 1.0,
    jitter: 7.0,
    glow: true,
});

/// Steam wisping off it.
static HEAT_STEAM: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.6, 1.2),
    colors: vec![(236, 236, 240), (170, 172, 180)],
    speed: 9.0,
    spread: 0.6,
    gravity: -26.0,
    drag: 1.5,
    size: 1.0,
    jitter: 2.0,
    glow: false,
});

/// Water off its feet as it lands.
static WET_SPRAY: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.2, 0.45),
    colors: vec![(190, 220, 255), (90, 150, 230)],
    speed: 70.0,
    spread: 0.5,
    gravity: 260.0,
    drag: 1.5,
    size: 1.0,
    jitter: 0.0,
    glow: false,
});

/// Oil near fire: an orange flicker over it.
static OIL_WARNING: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.08, 0.2),
    colors: vec![(255, 200, 90), (255, 120, 30)],
    speed: 6.0,
    spread: 3.14,
    gravity: -20.0,
    drag: 3.0,
    size: 1.0,
    jitter: 10.0,
    glow: true,
});

/// Wet in a storm: sparks crawling over it.
static STORM_CRAWL: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.05, 0.15),
    colors: vec![(235, 245, 255), (140, 190, 255)],
    speed: 20.0,
    spread: 3.14,
    gravity: 0.0,
    drag: 6.0,
    size: 1.0,
    jitter: 40.0,
    glow: true,
});
