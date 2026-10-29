//! Space (the space arc), the game's side.
//!
//! - **Sun and shade.** Up in the void there's no air to warm or cool
//!   you, only the sun: in its light you bake (slowly at first, then
//!   harder the longer you stay in it), out of it (a rock's shadow, or
//!   night, the world between you and the sun) you freeze (chilled,
//!   slowed). The edge of a shadow is where to be.
//! - **Meteor showers.** Some nights streaks cross space where you are,
//!   all from one way, fast and straight, smashing into what they meet:
//!   a bite out of a rock with a lump of meteorite left in it, a hard
//!   blow to you.

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind};

use platypus_sim::WorldEdit;
use platypus_sim::rng::hash;

use crate::creatures::body::elements::Chilled;
use crate::creatures::{Health, Kinematics};
use crate::light::LightSource;
use crate::magic::runes::Emitter;
use crate::world::{SimWorld, TickSet};

pub struct SpacePlugin;

impl Plugin for SpacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (sun_and_shade.after(crate::creatures::move_creatures), (shower, meteors).chain()).in_set(TickSet::Bodies));
    }
}

/// How far into space (`sky::space_at`) the sun and the shade begin to
/// tell.
const SPACE_FROM: f32 = 0.5;
/// Looking for what shades you: this far toward the sun (cells), this
/// many cells a step, again every this many ticks.
const SHADE_REACH: i32 = 900;
const SHADE_STEP: i32 = 2;
const LOOK_EVERY: u64 = 6;
/// In the sun, it takes this long (s) to bake through (heat 0 → 1), and
/// out of it this long to cool; baked, this much harm a second at full
/// heat (less below: the square of it).
const BAKE_SECS: f32 = 8.0;
const COOL_SECS: f32 = 4.0;
const SCORCH: f32 = 9.0;
/// In shade, chilled this hard (of a frost spell's worst), refreshed for
/// this long.
const SHADE_CHILL: f32 = 0.5;
const SHADE_CHILL_SECS: f32 = 0.5;

/// In space: in the sun or not, and how baked through (0 … 1).
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct Sunlit {
    pub lit: bool,
    pub heat: f32,
}

/// Is the sun up (by day: at night the world's between you and it)?
fn sun_up(time: f32) -> bool {
    -(time * std::f32::consts::TAU).cos() > 0.0
}

/// Nothing solid between `from` and the sun, `toward` it, as far as
/// `SHADE_REACH`?
fn in_the_sun(sim: &SimWorld, from: Vec2, toward: Vec2) -> bool {
    let mats = sim.world.materials();
    (1..SHADE_REACH / SHADE_STEP).all(|i| {
        let p = from + toward * (i * SHADE_STEP) as f32;
        sim.world.get(CellPos::from_world(p.x, p.y)).is_none_or(|c| c.is_air() || !matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder))
    })
}

/// Up in space: in the sun, baked (as fire: fire resistance takes some
/// off); in shade, chilled (frost resistance keeps some out).
fn sun_and_shade(mut commands: Commands, time: Res<Time>, sim: Res<SimWorld>, day: Res<crate::light::Daylight>, mut q: Query<(Entity, &Kinematics, &mut Health, Option<&mut Sunlit>, Option<&Chilled>, Option<&crate::gear::Stats>)>) {
    let dt = time.delta_secs();
    let look = sim.world.tick() % LOOK_EVERY == 0;
    let up = sun_up(day.time);
    let d = crate::light::sun_dir(day.time);
    let toward = -Vec2::new(d[0], d[1]);
    for (e, k, mut health, sunlit, chilled, stats) in &mut q {
        let space = crate::sky::space_at(&sim, k.body.pos.y);
        if space < SPACE_FROM {
            if sunlit.is_some() {
                commands.entity(e).remove::<Sunlit>();
            }
            continue;
        }
        let mut s = sunlit.as_deref().copied().unwrap_or_default();
        if look || sunlit.is_none() {
            s.lit = up && in_the_sun(&sim, k.body.pos, toward);
        }
        if s.lit {
            s.heat = (s.heat + dt / BAKE_SECS).min(1.0);
            health.linger(SCORCH * s.heat * s.heat * dt, crate::creatures::Harm::Fire, || crate::death::Cause::new("The sun", "baked"));
        } else {
            s.heat = (s.heat - dt / COOL_SECS).max(0.0);
            // (Kept warm: frost resistance keeps out the cold, a void suit's
            // most of it.)
            let warm = stats.map_or(0.0, |s| s.share(crate::gear::Stat::FrostResist, 0.9));
            crate::creatures::body::elements::chill(&mut commands, e, &health.nature, chilled, SHADE_CHILL * space * (1.0 - warm), SHADE_CHILL_SECS);
        }
        match sunlit {
            Some(mut now) => *now = s,
            None => {
                commands.entity(e).insert(s);
            }
        }
    }
}

/// One night in this many has a shower (by the seed and the night), from
/// this time to this (the day's share: night is past 0.75 and before
/// 0.25); a meteor in view this often (a second) while it lasts.
const SHOWER_NIGHTS: u64 = 3;
const SHOWER: (f32, f32) = (0.8, 0.2);
const SHOWER_RATE: f32 = 1.6;
/// A meteor's speed (cells/s: slowest, fastest), how long it flies (s),
/// and its size (the bite it takes: a radius, smallest and biggest).
const METEOR_SPEED: (f32, f32) = (380.0, 640.0);
const METEOR_LIFE: f32 = 4.0;
const METEOR_SIZE: (i32, i32) = (2, 6);
/// What it does to a rock (the blast's power) and to you (harm a cell of
/// its size, a knock this hard).
const METEOR_POWER: u8 = 110;
const METEOR_HARM: f32 = 7.0;
const METEOR_KNOCK: f32 = 260.0;

/// A meteor crossing space: how it's going, how long it has left, how big.
#[derive(Component)]
pub struct Meteor {
    vel: Vec2,
    left: f32,
    size: i32,
}

/// Is there a shower in space now (the night's, by the seed; or
/// `PLATYPUS_SHOWER` set, to look at one)? The way its meteors go, if so.
pub fn shower_now(seed: u64, day: &crate::light::Daylight) -> Option<Vec2> {
    let forced = std::env::var("PLATYPUS_SHOWER").is_ok();
    let t = day.time;
    let night = (day.days + 0.5).floor() as u64;
    let on = forced || ((t > SHOWER.0 || t < SHOWER.1) && hash(&[seed, night, 0x5E7E]) % SHOWER_NIGHTS == 0);
    // (All one way, the night's: down across, left or right.)
    let a = -0.25 - (hash(&[seed, night, 0xA9]) % 1_000) as f32 / 1_000.0 * 0.5;
    on.then(|| Vec2::from_angle(a * std::f32::consts::PI))
}

/// While a shower's on and the camera's up in space, meteors come in from
/// beyond the view's edge, the shower's way.
fn shower(mut commands: Commands, time: Res<Time>, sim: Res<SimWorld>, day: Res<crate::light::Daylight>, camera: Query<(&GlobalTransform, &crate::world::ChunkLoader), With<crate::camera::MainCamera>>, mut owed: Local<f32>) {
    let Ok((tf, l)) = camera.single() else { return };
    let c = tf.translation().truncate();
    let Some(way) = shower_now(sim.world.seed(), &day).filter(|_| crate::sky::space_at(&sim, c.y) >= SPACE_FROM) else {
        *owed = 0.0;
        return;
    };
    *owed += time.delta_secs() * SHOWER_RATE;
    let mut n = 0u64;
    while *owed >= 1.0 {
        *owed -= 1.0;
        n += 1;
        let u = |salt: u64| (hash(&[sim.world.seed(), sim.world.tick(), n, salt]) % 10_000) as f32 / 10_000.0;
        // (From beyond the view, back along its way, across it to either
        // side: it crosses somewhere in view, or near it.)
        let reach = l.half_extent.length();
        let across = Vec2::new(-way.y, way.x) * (u(0) * 2.0 - 1.0) * reach;
        let from = c - way * (reach + 60.0) + across;
        let speed = METEOR_SPEED.0 + u(1) * (METEOR_SPEED.1 - METEOR_SPEED.0);
        let size = METEOR_SIZE.0 + (u(2) * (METEOR_SIZE.1 - METEOR_SIZE.0 + 1) as f32) as i32;
        commands.spawn((
            Name::new("Meteor"),
            Meteor { vel: way * speed, left: METEOR_LIFE, size },
            Transform::from_translation(from.extend(40.0)),
            Sprite::from_color(Color::srgb(1.0, 0.95, 0.85), Vec2::splat(size as f32 * 1.2)),
            LightSource { color: [1.3, 1.0, 0.7], flicker: 0.15 },
        ));
    }
}

/// Its trail: glowing, orange to dark.
static TRAIL: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.2, 0.6),
    colors: vec![(255, 240, 200), (255, 170, 80), (200, 80, 30)],
    speed: 20.0,
    spread: 0.4,
    gravity: 0.0,
    drag: 2.0,
    size: 1.4,
    jitter: 0.5,
    glow: true,
});

/// Meteors fly straight on, a cell at a time, till they meet rock (a bite
/// out of it, a lump of meteorite left) or someone (hurt, flung), or their
/// time's up.
#[allow(clippy::type_complexity)]
fn meteors(
    mut commands: Commands,
    time: Res<Time>,
    mut sim: ResMut<SimWorld>,
    mut q: Query<(Entity, &mut Meteor, &mut Transform)>,
    mut bodies: Query<(&mut Kinematics, &mut Health), Without<Meteor>>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    let dt = time.delta_secs();
    let meteorite = sim.materials().id("meteorite");
    for (e, mut m, mut tf) in &mut q {
        m.left -= dt;
        let from = tf.translation.truncate();
        let steps = (m.vel.length() * dt).ceil().max(1.0) as i32;
        let step = m.vel * dt / steps as f32;
        let mut at = from;
        let mut struck = None;
        'flight: for _ in 0..steps {
            at += step;
            let p = CellPos::from_world(at.x, at.y);
            let mats = sim.materials();
            if sim.world.get(p).is_some_and(|c| !c.is_air() && matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder)) {
                struck = Some(None);
                break;
            }
            for (bi, (k, _)) in bodies.iter().enumerate() {
                if (at - k.body.pos).abs().cmple(k.body.half + Vec2::splat(m.size as f32 * 0.5)).all() {
                    struck = Some(Some(bi));
                    break 'flight;
                }
            }
        }
        // (Its trail all along where it went this tick, not in dots.)
        let back = -m.vel.normalize_or_zero();
        let gone = at.distance(from);
        for i in 0..(gone / 2.0).ceil() as usize {
            sparks.emit(&TRAIL, 1 + m.size as usize / 3, at + back * (i as f32 * 2.0), back, Vec2::ZERO);
        }
        tf.translation = at.extend(40.0);
        match struck {
            None if m.left > 0.0 => continue,
            None => {}
            Some(None) => {
                let p = CellPos::from_world(at.x, at.y);
                sim.queue(WorldEdit::Explode { center: p, radius: m.size + 2, power: METEOR_POWER });
                if let Some(id) = meteorite {
                    sim.queue(WorldEdit::Paint { center: p, radius: (m.size / 2).max(1), material: id, overwrite: true });
                }
                sounds.write(crate::sound::PlaySound::at("thoom", at).volume(0.35));
            }
            Some(Some(bi)) => {
                if let Some((mut k, mut health)) = bodies.iter_mut().nth(bi) {
                    health.harm_by(METEOR_HARM * m.size as f32, crate::creatures::Harm::Blunt, || crate::death::Cause::new("A meteor", "struck down"));
                    let k = &mut *k;
                    let knock = k.body.vel + m.vel.normalize_or_zero() * METEOR_KNOCK;
                    k.loco.knock(&mut k.body, knock, 0.4);
                }
                sounds.write(crate::sound::PlaySound::at("slam", at).volume(0.5));
            }
        }
        sparks.emit(&TRAIL, 24, at, -m.vel.normalize_or_zero(), Vec2::ZERO);
        commands.entity(e).despawn();
    }
}
