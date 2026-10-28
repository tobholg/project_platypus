//! The high air (the sky arc), over the sky islands: the jet stream, a
//! band of wind blowing the way the weather's wind does, carrying what's
//! in the air through it (creatures, drifters: `drift`), streaks of it
//! showing where it runs. (The air thinning, `breath`; gravity fading,
//! `creatures::move_creatures`.)
//!
//! And fulgurite, stormglass: where lightning strikes rock, sand or soil,
//! a vein of it forks down from the strike (the islands are scarred with
//! old ones); storm-charged, it shocks what touches it wet.

use bevy::prelude::*;

use crate::magic::runes::Emitter;
use crate::world::{ChunkLoader, SimWorld};

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, streaks).add_systems(FixedUpdate, (fuse, charged).in_set(crate::world::TickSet::Bodies));
    }
}

/// The jet stream's speed in its middle (cells/s). (What's in it steers
/// through the air, and the air's going this way: `move_creatures`.)
pub const JET: f32 = 160.0;
/// Streaks in view in its middle, a second, at most.
const STREAKS: f32 = 70.0;

/// Which way the jet stream blows (the weather's wind's way), and how hard
/// at `y`: its velocity there.
pub fn jet_at(sim: &SimWorld, y: f32) -> f32 {
    let Some(sky) = sim.generator.sky_layers() else { return 0.0 };
    let way = if sim.world.wind() < 0.0 { -1.0 } else { 1.0 };
    way * JET * sky.jet(y)
}

/// Streaks of wind racing through the jet stream where it's in view.
fn streaks(time: Res<Time>, sim: Res<SimWorld>, camera: Query<(&GlobalTransform, &ChunkLoader), With<crate::camera::MainCamera>>, mut sparks: ResMut<crate::vfx::Sparks>, mut owed: Local<f32>, mut seed: Local<u64>) {
    let Some(sky) = sim.generator.sky_layers() else { return };
    let Ok((tf, l)) = camera.single() else { return };
    let c = tf.translation().truncate();
    let (lo, hi) = ((c.y - l.half_extent.y).max(sky.jet.0 as f32), (c.y + l.half_extent.y).min(sky.jet.1 as f32));
    if hi <= lo {
        *owed = 0.0;
        return;
    }
    let share = (hi - lo) / (sky.jet.1 - sky.jet.0) as f32;
    *owed += time.delta_secs() * STREAKS * share;
    let mut roll = || {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (*seed >> 33) as f32 / (1u64 << 31) as f32
    };
    while *owed >= 1.0 {
        *owed -= 1.0;
        let at = Vec2::new(c.x + (roll() - 0.5) * 2.0 * l.half_extent.x, lo + roll() * (hi - lo));
        let jet = jet_at(&sim, at.y);
        if jet.abs() < 20.0 {
            continue;
        }
        sparks.emit(&STREAK, 1, at, Vec2::new(jet.signum(), 0.0), Vec2::new(jet, 0.0));
    }
}

/// A streak of the jet stream: pale, racing along with it.
static STREAK: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.8, 1.6),
    colors: vec![(236, 244, 255), (210, 226, 246)],
    speed: 30.0,
    spread: 0.04,
    gravity: 0.0,
    drag: 0.0,
    size: 1.0,
    jitter: 0.0,
    glow: false,
});

/// A strike's vein: this long (cells, down from it), forking now and then.
const VEIN: (u32, u32) = (14, 34);
/// Wet (this much or more) and touching fulgurite: shocked this hard (more
/// the wetter), stunned this long, no oftener than this (s).
const WET_ENOUGH: f32 = 0.25;
const SHOCK: f32 = 4.0;
const STUN: f32 = 0.35;
const SHOCK_EVERY: f32 = 1.2;

/// Where lightning strikes rock, sand or soil, it fuses a vein of
/// fulgurite down from the strike, forking.
fn fuse(mut strikes: MessageReader<crate::fx::Lightning>, mut sim: ResMut<SimWorld>) {
    let Some(glass) = sim.materials().id("fulgurite") else { return };
    for crate::fx::Lightning(s) in strikes.read() {
        let mats = sim.materials().clone();
        let fusible = |c: platypus_sim::Cell| !c.is_air() && matches!(mats.phys(c.material).kind, platypus_sim::Kind::Static | platypus_sim::Kind::Powder) && !mats.phys(c.material).object && mats.phys(c.material).hardness < u8::MAX;
        let mut rng = platypus_sim::rng::Rng::seeded(&[sim.world.seed(), 0xF01C, s.hit.x as u64, s.hit.y as u64]);
        // (Where it went to earth (down a tree, the ground under it), or
        // the first rock under that.)
        let Some(start) = (0..12).map(|d| s.earth.offset(0, -d)).find(|&p| sim.world.get(p).is_some_and(fusible)) else { continue };
        let len = VEIN.0 + rng.next_u32() % (VEIN.1 - VEIN.0);
        let mut veins = vec![(start.x as f32 + 0.5, start.y, len)];
        while let Some((mut x, mut y, mut left)) = veins.pop() {
            while left > 0 {
                let p = platypus_sim::CellPos::new(x.floor() as i32, y);
                if !sim.world.get(p).is_some_and(|c| fusible(c) || c.material == glass) {
                    break;
                }
                sim.world.set(p, platypus_sim::Cell::new(glass, rng.next_u8()));
                if rng.chance(70) {
                    let q = p.offset(if rng.coin() { 1 } else { -1 }, 0);
                    if sim.world.get(q).is_some_and(fusible) {
                        sim.world.set(q, platypus_sim::Cell::new(glass, rng.next_u8()));
                    }
                }
                x += (rng.next_u8() as f32 / 255.0 - 0.5) * 1.6;
                y -= 1;
                left -= 1;
                if rng.chance(12) && veins.len() < 3 {
                    veins.push((x + if rng.coin() { 2.0 } else { -2.0 }, y, left / 2));
                }
            }
        }
    }
}

/// Touching fulgurite wet: shocked (the wetter, the harder), stunned a
/// moment, sparks off it.
#[allow(clippy::type_complexity)]
fn charged(
    time: Res<Time>,
    sim: Res<SimWorld>,
    coatings: Res<crate::creatures::body::elements::Coatings>,
    mut q: Query<(Entity, &mut crate::creatures::Health, &mut crate::creatures::Kinematics, &crate::creatures::body::elements::Coated)>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut last: Local<bevy::platform::collections::HashMap<Entity, f32>>,
) {
    let Some(glass) = sim.materials().id("fulgurite") else { return };
    let now = time.elapsed_secs();
    last.retain(|_, t| now - *t < SHOCK_EVERY);
    for (e, mut health, mut k, coated) in &mut q {
        let wet = coated.wet(&coatings.by_name).min(1.0);
        if wet < WET_ENOUGH || last.contains_key(&e) {
            continue;
        }
        let (lo, hi) = (k.body.pos - k.body.half, k.body.pos + k.body.half);
        let touch = (lo.y.floor() as i32 - 1..=hi.y.ceil() as i32).flat_map(|y| (lo.x.floor() as i32 - 1..=hi.x.ceil() as i32).map(move |x| platypus_sim::CellPos::new(x, y))).find(|&p| sim.world.get(p).is_some_and(|c| c.material == glass));
        let Some(p) = touch else { continue };
        last.insert(e, now);
        health.harm_by(SHOCK * (1.0 + 0.75 * wet), crate::creatures::nature::Harm::Storm, || crate::death::Cause::new("Fulgurite", "shocked"));
        let k = &mut *k;
        let vel = k.body.vel * 0.3;
        k.loco.knock(&mut k.body, vel, STUN);
        let at = Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5);
        sparks.emit(&ARC, 10, at, Vec2::Y, Vec2::ZERO);
        sounds.write(crate::sound::PlaySound::at("zap", at).volume(0.6));
    }
}

/// Sparks off charged fulgurite.
static ARC: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.1, 0.3),
    colors: vec![(240, 250, 255), (160, 210, 255), (110, 160, 255)],
    speed: 90.0,
    spread: 3.14,
    gravity: 0.0,
    drag: 5.0,
    size: 1.0,
    jitter: 0.0,
    glow: true,
});
