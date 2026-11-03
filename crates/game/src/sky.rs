//! The high air (the sky arc), up among the highest sky islands: the jet
//! stream, a band of wind blowing the way the weather's wind does,
//! gusting, carrying what's in the air through it (creatures, drifters:
//! `drift`; calm in the lee of the islands' ridges and down on them:
//! `wind_at`), streaks of it showing where it runs. (The air thinning, `breath`; gravity fading,
//! `creatures::move_creatures`.)
//!
//! And fulgurite, stormglass: where lightning strikes rock, sand or soil,
//! a vein of it forks down from the strike (the islands are scarred with
//! old ones). Charged in a severe storm, and for a while after lightning
//! strikes near it (`Charge`), it crackles (arcs crawling over every vein
//! of it in view, however small, and a buzz near it: `crackle`) and shocks
//! what touches it wet; uncharged it's only glass.

use bevy::prelude::*;

use crate::magic::runes::Emitter;
use crate::world::{ChunkLoader, SimWorld};

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Charge>()
            .init_resource::<Veins>()
            .add_systems(Update, streaks)
            .add_systems(FixedUpdate, (fuse, (remember_strikes, find_veins, crackle, charged).chain()).in_set(crate::world::TickSet::Bodies));
    }
}

/// The jet stream's speed in its middle (cells/s). (What's in it steers
/// through the air, and the air's going this way: `move_creatures`.)
pub const JET: f32 = 160.0;
/// Streaks in view in its middle, a second, at most.
const STREAKS: f32 = 70.0;

/// How far into space a height is (0: under the thinning air … 1: the
/// void, where the sky's black and the stars are out by day).
pub fn space_at(sim: &SimWorld, y: f32) -> f32 {
    sim.generator.sky_layers().map_or(0.0, |s| s.thin(y))
}

/// Space's black (a little blue in it).
pub const SPACE: [f32; 3] = [0.01, 0.012, 0.03];

/// The jet stream gusts: from a lull (this share of it) to its full
/// strength and back, about every this many seconds (and a quicker
/// flutter over it), each gust running along with it: a crossing waits
/// for a lull.
const LULL: f32 = 0.3;
const GUST_SECS: (f32, f32) = (7.0, 2.9);

/// Which way the jet stream blows (the weather's wind's way): 1 or −1.
fn jet_way(sim: &SimWorld) -> f32 {
    if sim.world.wind() < 0.0 { -1.0 } else { 1.0 }
}

/// The gusts' two waves at `x` now: their phases (radians), and how each
/// turns along x (radians a cell), for drawing them (the cirrus riding the
/// jet: `backdrop`). Gusting: 0.5 + 0.3 sin(first) + 0.2 sin(second).
pub fn gusts(sim: &SimWorld, x: f32) -> [f32; 4] {
    let way = jet_way(sim) as f64;
    let tau = std::f64::consts::TAU;
    let u = sim.world.tick() as f64 / crate::world::TICK_HZ - way * x as f64 / JET as f64;
    let (a, b) = (GUST_SECS.0 as f64, GUST_SECS.1 as f64);
    [(tau * u / a).rem_euclid(tau) as f32, (tau * u / b + 1.3).rem_euclid(tau) as f32, (-tau * way / (a * JET as f64)) as f32, (-tau * way / (b * JET as f64)) as f32]
}

/// How far into a gust it is at `x` now (0 … 1).
pub fn gust_at(sim: &SimWorld, x: f32) -> f32 {
    let g = gusts(sim, x);
    0.5 + 0.3 * g[0].sin() + 0.2 * g[1].sin()
}

/// Which way the jet stream blows (the weather's wind's way), and how hard
/// at `x`, `y` (gusting): its velocity there.
pub fn jet_at(sim: &SimWorld, x: f32, y: f32) -> f32 {
    let Some(sky) = sim.generator.sky_layers() else { return 0.0 };
    jet_way(sim) * JET * sky.jet(y) * (LULL + (1.0 - LULL) * gust_at(sim, x))
}

/// Out of the jet stream: right behind ground upwind it's calm, this far
/// off it (cells) it blows full; over the ground, the same (calm on it,
/// full this far over it); looked for this many cells at a time.
const LEE: i32 = 64;
const ALOFT: i32 = 96;
const LEE_STEP: usize = 4;

/// The jet stream where a body is (its middle, and half its height): calm
/// in the lee of what's solid upwind of it (behind a ridge, down in a
/// hollow, by a trunk) and down by the ground, rising to the full wind out
/// in the open (off an island's edge, over the gulf between two).
pub fn wind_at(sim: &SimWorld, at: Vec2, half: f32) -> f32 {
    let jet = jet_at(sim, at.x, at.y);
    if jet == 0.0 {
        return 0.0;
    }
    let solid = |x: i32, y: i32| sim.world.is_solid(platypus_sim::CellPos::new(x, y));
    let up = -jet.signum() as i32;
    let (x, feet) = (at.x.floor() as i32, (at.y - half).floor() as i32);
    let rows = [at.y.floor() as i32, (at.y - 0.5 * half).floor() as i32];
    let lee = (LEE_STEP as i32..=LEE).step_by(LEE_STEP).find(|d| rows.iter().any(|&y| solid(x + up * d, y))).map_or(1.0, |d| d as f32 / LEE as f32);
    let aloft = (1..=ALOFT).step_by(LEE_STEP).find(|d| solid(x, feet - d)).map_or(1.0, |d| d as f32 / ALOFT as f32);
    jet * lee.min(aloft)
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
        let jet = wind_at(&sim, at, 0.0);
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
/// Wet (this much or more) and touching charged fulgurite: shocked this
/// hard (more the wetter; as what goes on hurting it, by difficulty:
/// `Health::linger`), stunned this long, no oftener than this (s).
const WET_ENOUGH: f32 = 0.25;
const SHOCK: f32 = 4.0;
const STUN: f32 = 0.35;
const SHOCK_EVERY: f32 = 1.2;
/// The first shock after this long off it (s) is only this share: a
/// warning (a zap and a push), not a wound.
const FIRST_AFTER: f32 = 5.0;
const FIRST_SHARE: f32 = 0.2;
/// Lightning this near (cells) charges fulgurite for this long (s).
const STRUCK_NEAR: f32 = 220.0;
const STRUCK_FOR: f32 = 30.0;
/// A storm this severe over it (`Weather::severe_at`) charges it.
const SEVERE_ENOUGH: f32 = 0.25;

/// Where lightning struck lately, and when (s).
#[derive(Resource, Default)]
pub struct Charge {
    strikes: Vec<(Vec2, f32)>,
}

impl Charge {
    /// Is fulgurite at `at` charged now (`now`: s)?
    pub fn at(&self, sim: &SimWorld, at: Vec2, now: f32) -> bool {
        sim.world.weather().is_some_and(|w| w.severe_at(at.x.floor() as i32) >= SEVERE_ENOUGH)
            || self.strikes.iter().any(|&(p, t)| now - t < STRUCK_FOR && p.distance(at) < STRUCK_NEAR)
            || forced()
    }
}

/// `PLATYPUS_CHARGED=1`: fulgurite's always charged (to look at it).
fn forced() -> bool {
    static F: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| std::env::var("PLATYPUS_CHARGED").is_ok_and(|v| v != "0"));
    *F
}

fn remember_strikes(time: Res<Time>, mut strikes: MessageReader<crate::fx::Lightning>, mut charge: ResMut<Charge>) {
    let now = time.elapsed_secs();
    charge.strikes.retain(|&(_, t)| now - t < STRUCK_FOR);
    for crate::fx::Lightning(s) in strikes.read() {
        charge.strikes.push((Vec2::new(s.earth.x as f32 + 0.5, s.earth.y as f32 + 0.5), now));
    }
}

/// Fulgurite in view (and a margin round it): found a band of rows a tick,
/// all of it every `SWEEP` ticks (it only changes where lightning strikes
/// or it's mined).
#[derive(Resource, Default)]
struct Veins {
    /// What the last whole sweep found.
    cells: Vec<platypus_sim::CellPos>,
    /// This sweep's so far, its box, and the next row.
    finding: Vec<platypus_sim::CellPos>,
    span: (i32, i32, i32, i32),
    row: i32,
}

const SWEEP: i32 = 12;
const MARGIN: f32 = 24.0;

fn find_veins(sim: Res<SimWorld>, camera: Query<(&GlobalTransform, &ChunkLoader), With<crate::camera::MainCamera>>, mut veins: ResMut<Veins>) {
    let Some(glass) = sim.materials().id("fulgurite") else { return };
    let v = &mut *veins;
    if v.row > v.span.3 {
        v.cells = std::mem::take(&mut v.finding);
        let Ok((tf, l)) = camera.single() else { return };
        let c = tf.translation().truncate();
        let (lo, hi) = (c - l.half_extent - MARGIN, c + l.half_extent + MARGIN);
        v.span = (lo.x.floor() as i32, hi.x.ceil() as i32, lo.y.floor() as i32, hi.y.ceil() as i32);
        v.row = v.span.2;
    }
    let rows = ((v.span.3 - v.span.2 + 1) / SWEEP).max(1);
    let end = (v.row + rows).min(v.span.3 + 1);
    for y in v.row..end {
        for x in v.span.0..=v.span.1 {
            let p = platypus_sim::CellPos::new(x, y);
            if sim.world.get(p).is_some_and(|c| c.material == glass) {
                v.finding.push(p);
            }
        }
    }
    v.row = end;
}

/// Charged fulgurite crackles: arcs crawl over it, a few a tick on every
/// vein in view (a single cell of it as plainly as a whole seam), and it
/// buzzes near the player.
fn crackle(
    time: Res<Time>,
    sim: Res<SimWorld>,
    charge: Res<Charge>,
    veins: Res<Veins>,
    player: Query<&crate::creatures::Kinematics, With<crate::creatures::player::LocalPlayer>>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut buzzed: Local<f32>,
) {
    if veins.cells.is_empty() {
        return;
    }
    let now = time.elapsed_secs();
    let tick = sim.world.tick();
    let mut rng = platypus_sim::rng::Rng::seeded(&[tick, 0xC4A6]);
    let me = player.single().ok().map(|k| k.body.pos);
    let mut nearest: Option<(f32, Vec2)> = None;
    // (Every patch of it (`PATCH` cells square) its own arcs, a lone speck
    // as plainly as a seam: about every other tick each, at most `MOST`.)
    const PATCH: i32 = 8;
    const MOST: usize = 24;
    let mut patches: Vec<((i32, i32), Vec<platypus_sim::CellPos>)> = Vec::new();
    for &p in &veins.cells {
        let key = (p.x.div_euclid(PATCH), p.y.div_euclid(PATCH));
        match patches.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => v.push(p),
            None => patches.push((key, vec![p])),
        }
    }
    let mut emitted = 0;
    for (_, cells) in &patches {
        if emitted >= MOST {
            break;
        }
        if !rng.coin() {
            continue;
        }
        let p = cells[rng.next_u32() as usize % cells.len()];
        let at = Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5);
        if !charge.at(&sim, at, now) || !sim.world.get(p).is_some_and(|c| sim.materials().id("fulgurite") == Some(c.material)) {
            continue;
        }
        emitted += 1;
        // (An arc: a spray of sparks off its face, up and to the sides; now
        // and then a big one, leaping up.)
        let big = rng.chance(40);
        let dir = Vec2::new((rng.next_u8() as f32 / 255.0 - 0.5) * 1.6, 1.0).normalize();
        sparks.emit(if big { &LEAP } else { &CRACKLE }, if big { 7 } else { 3 + (rng.next_u8() % 3) as usize }, at, dir, Vec2::ZERO);
        if let Some(me) = me {
            let d = me.distance(at);
            if nearest.is_none_or(|(b, _)| d < b) {
                nearest = Some((d, at));
            }
        }
    }
    // A buzz, now and then, the nearer the oftener.
    const HEAR: f32 = 70.0;
    if let Some((d, at)) = nearest
        && d < HEAR
        && now - *buzzed > 0.5 + d / HEAR * 1.5
    {
        *buzzed = now;
        sounds.write(crate::sound::PlaySound::at("zap", at).volume(0.18 * (1.0 - d / HEAR) + 0.05));
    }
}

/// Crawling arcs over charged fulgurite: short, bright, lit; and now and
/// then one leaping up off it.
static CRACKLE: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.08, 0.22),
    colors: vec![(250, 252, 255), (190, 225, 255), (130, 180, 255)],
    speed: 55.0,
    spread: 1.3,
    gravity: 0.0,
    drag: 7.0,
    size: 1.0,
    jitter: 0.8,
    glow: true,
});
static LEAP: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.12, 0.3),
    colors: vec![(255, 255, 255), (200, 232, 255), (150, 200, 255)],
    speed: 110.0,
    spread: 0.5,
    gravity: 0.0,
    drag: 5.0,
    size: 1.0,
    jitter: 1.2,
    glow: true,
});

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

/// Touching charged fulgurite wet: shocked (the wetter, the harder; only a
/// warning the first time in a while), stunned a moment, sparks off it.
#[allow(clippy::type_complexity)]
fn charged(
    time: Res<Time>,
    sim: Res<SimWorld>,
    coatings: Res<crate::creatures::body::elements::Coatings>,
    mut q: Query<(Entity, &mut crate::creatures::Health, &mut crate::creatures::Kinematics, &crate::creatures::body::elements::Coated)>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    charge: Res<Charge>,
    mut last: Local<bevy::platform::collections::HashMap<Entity, f32>>,
) {
    let Some(glass) = sim.materials().id("fulgurite") else { return };
    let now = time.elapsed_secs();
    last.retain(|_, t| now - *t < FIRST_AFTER);
    for (e, mut health, mut k, coated) in &mut q {
        let wet = coated.wet(&coatings.by_name).min(1.0);
        let since = last.get(&e).map(|t| now - t);
        if wet < WET_ENOUGH || since.is_some_and(|s| s < SHOCK_EVERY) {
            continue;
        }
        let (lo, hi) = (k.body.pos - k.body.half, k.body.pos + k.body.half);
        let touch = (lo.y.floor() as i32 - 1..=hi.y.ceil() as i32).flat_map(|y| (lo.x.floor() as i32 - 1..=hi.x.ceil() as i32).map(move |x| platypus_sim::CellPos::new(x, y))).find(|&p| sim.world.get(p).is_some_and(|c| c.material == glass));
        let Some(p) = touch else { continue };
        let at = Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5);
        if !charge.at(&sim, at, now) {
            continue;
        }
        last.insert(e, now);
        let share = if since.is_none() { FIRST_SHARE } else { 1.0 };
        health.linger(SHOCK * (1.0 + 0.75 * wet) * share, crate::creatures::nature::Harm::Storm, || crate::death::Cause::new("Charged fulgurite", "shocked"));
        let k = &mut *k;
        let vel = k.body.vel * 0.3;
        k.loco.knock(&mut k.body, vel + Vec2::new(0.0, 60.0), STUN);
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
