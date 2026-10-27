//! Shoals: glowfish schooling in the open water under the ground (the
//! flooded caverns, the great halls' lakes). Cheap: a fish is a point that
//! steers, drawn as a few pixels, not a creature with a brain; a shoal is
//! a few dozen of them (now and then a great one of a couple of hundred)
//! and a faint light at its middle.
//!
//! Each fish keeps a little room from its neighbours, swims as they do and
//! keeps with the shoal (boids), wanders a little, keeps to the water (it
//! turns from rock and air ahead), and flees what comes near: you,
//! anything alive. Their scales catch the light: a fish glints silver as
//! its side turns your way, and as the shoal wheels together a ripple of
//! glints runs through it, as sardines' does.
//!
//! Shoals come and go round the player: one turns up in open water out of
//! sight (`RING`) now and then, up to `MOST` about, and goes when it's far.

use bevy::prelude::*;
use platypus_sim::CellPos;

use crate::creatures::player::LocalPlayer;
use crate::creatures::Kinematics;
use crate::world::SimWorld;

pub struct ShoalsPlugin;

impl Plugin for ShoalsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Shoals>()
            .init_resource::<FishCanvas>()
            .add_systems(Update, (come_and_go, swim).chain())
            .add_systems(PostUpdate, show.before(bevy::transform::TransformSystems::Propagate));
    }
}

struct Fish {
    pos: Vec2,
    vel: Vec2,
    /// Its own beat (glint, wander).
    phase: f32,
}

struct Shoal {
    fish: Vec<Fish>,
    light: Entity,
    /// Which way it's meandering (radians), and how fast its fish go.
    heading: f32,
    speed: f32,
}

/// Glowfish glow of themselves: drawn over the light (as a portal is), a
/// cell at a time.
#[derive(Resource)]
struct FishCanvas(crate::canvas::Canvas);

impl Default for FishCanvas {
    fn default() -> Self {
        FishCanvas(crate::canvas::Canvas::new("Glowfish", Z_FISH))
    }
}

/// Over the light overlay, under the void's.
const Z_FISH: f32 = 15.6;

#[derive(Resource, Default)]
pub struct Shoals {
    shoals: Vec<Shoal>,
    /// Seconds to the next look for open water to put one in.
    next: f32,
}

/// Shoals about the player at most; one turns up this far off (cells,
/// from, to), and goes past this far.
const MOST: usize = 3;
const RING: (f32, f32) = (180.0, 420.0);
const GONE: f32 = 900.0;
/// Fish a shoal (from, to: most near the fewest, one in four over a
/// hundred); how often one may turn up (s).
const FISH: (usize, usize) = (22, 200);
const EVERY: f32 = 2.5;
/// Open water round a spot for a shoal to turn up there: this many cells
/// either way all water (for a shoal of `TYPICAL`; more for more), and
/// they start this spread (cells).
const ROOM: i32 = 14;
const SPREAD: f32 = 18.0;
const TYPICAL: f32 = 33.0;
/// Boids: keep this far from each other (cells), swim with those this near,
/// keep with those this near; how hard each pulls (a share a second).
const APART: f32 = 3.0;
const WITH: f32 = 9.0;
const NEAR: f32 = 16.0;
const PULL: (f32, f32, f32) = (6.0, 1.6, 0.9);
/// A fish heeds at most this many of those round it.
const HEED: usize = 16;
/// Speeds (cells/s): cruising (from, to), fleeing; flee from what's this
/// near; look this far ahead for rock or air.
const SPEED: (f32, f32) = (22.0, 40.0);
const FLEE_SPEED: f32 = 95.0;
const FLEE: f32 = 42.0;
const AHEAD: f32 = 5.0;
/// Its body: this long (cells); its colour, its glint, its glow.
const LONG: f32 = 2.6;
const BODY: [f32; 4] = [0.5, 0.86, 0.92, 1.0];
const GLINT: [f32; 3] = [0.92, 0.98, 1.0];
const GLOW: [f32; 3] = [0.03, 0.13, 0.15];

/// Water here (not another liquid: no glowfish in acid).
fn water(sim: &SimWorld, id: Option<platypus_sim::MaterialId>, p: Vec2) -> bool {
    sim.world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| Some(c.material) == id)
}

fn unit(seed: u64) -> f32 {
    (platypus_sim::rng::hash(&[seed]) % 10_000) as f32 / 10_000.0
}

/// Shoals turn up in open water round the player, out of sight, and go when
/// they're far (or their water's gone).
fn come_and_go(mut commands: Commands, time: Res<Time>, sim: Res<SimWorld>, mut shoals: ResMut<Shoals>, player: Query<&Kinematics, With<LocalPlayer>>) {
    let Ok(me) = player.single() else { return };
    let me = me.body.pos;
    if !sim.generator.wild() {
        return;
    }
    let id = sim.materials().id("water");
    shoals.shoals.retain(|s| {
        let mid = s.fish.iter().map(|f| f.pos).sum::<Vec2>() / s.fish.len().max(1) as f32;
        let keep = mid.distance(me) < GONE && s.fish.iter().any(|f| water(&sim, id, f.pos));
        if !keep {
            commands.entity(s.light).despawn();
        }
        keep
    });
    shoals.next -= time.delta_secs();
    if shoals.next > 0.0 || shoals.shoals.len() >= MOST {
        return;
    }
    shoals.next = EVERY;
    let tick = sim.world.tick();
    // A spot in the ring round the player, all water round it.
    for k in 0..8u64 {
        let a = unit(tick ^ (k << 32) ^ 0x5401) * std::f32::consts::TAU;
        // (`PLATYPUS_SHOALS_NEAR=1`: close by, in sight: to look at them.)
        let ring = if std::env::var("PLATYPUS_SHOALS_NEAR").is_ok() { (45.0, 60.0) } else { RING };
        let r = ring.0 + (ring.1 - ring.0) * unit(tick ^ (k << 32) ^ 0x5402);
        let at = me + Vec2::from_angle(a) * r;
        // (A great shoal wants a great water: where there isn't one, fewer.)
        // (`PLATYPUS_SHOALS_FISH=N`: that many, to look at a great one.)
        let mut n = std::env::var("PLATYPUS_SHOALS_FISH").ok().and_then(|v| v.parse().ok()).unwrap_or(FISH.0 + (unit(tick ^ 0x5403).powi(3) * (FISH.1 - FISH.0) as f32) as usize);
        let open = loop {
            let room = (ROOM as f32 * (n as f32 / TYPICAL).sqrt().max(1.0)) as i32;
            if (-room..=room).step_by(4).all(|dy| (-room..=room).step_by(4).all(|dx| water(&sim, id, at + Vec2::new(dx as f32, dy as f32)))) {
                break true;
            }
            if n <= FISH.0 {
                break false;
            }
            n = (n / 2).max(FISH.0);
        };
        if !open {
            continue;
        }
        let size = (n as f32 / TYPICAL).sqrt();
        let heading = unit(tick ^ 0x5404) * std::f32::consts::TAU;
        let speed = SPEED.0 + (SPEED.1 - SPEED.0) * unit(tick ^ 0x5405);
        let fish = (0..n as u64)
            .map(|i| {
                let off = Vec2::new(unit(tick ^ i ^ 0x5406) - 0.5, unit(tick ^ i ^ 0x5407) - 0.5) * SPREAD * size.max(1.0);
                Fish { pos: at + off, vel: Vec2::from_angle(heading) * speed, phase: unit(tick ^ i ^ 0x5408) * 100.0 }
            })
            .collect();
        // (More fish, more glow.)
        let glow = GLOW.map(|c| c * size.clamp(1.0, 2.5));
        let light = commands.spawn((Name::new("Shoal light"), crate::light::LightSource { color: glow, flicker: 0.0 }, Transform::from_translation(at.extend(0.0)))).id();
        debug!("shoals: {n} glowfish turned up at ({:.0}, {:.0})", at.x, at.y);
        shoals.shoals.push(Shoal { fish, light, heading, speed });
        break;
    }
}

/// Each fish: apart, along, together; a wander; away from rock and air
/// ahead; away from what comes near.
fn swim(time: Res<Time>, sim: Res<SimWorld>, mut shoals: ResMut<Shoals>, bodies: Query<&Kinematics>) {
    let dt = time.delta_secs().min(0.05);
    if shoals.shoals.is_empty() {
        return;
    }
    let t = time.elapsed_secs();
    let id = sim.materials().id("water");
    let near: Vec<Vec2> = bodies.iter().map(|k| k.body.pos).collect();
    for s in &mut shoals.shoals {
        // (The shoal meanders as one: its heading drifts.)
        s.heading += (unit((t * 0.2) as u64 ^ s.light.to_bits()) - 0.5) * dt * 0.8;
        let cruise = Vec2::from_angle(s.heading) * s.speed;
        let snapshot: Vec<(Vec2, Vec2)> = s.fish.iter().map(|f| (f.pos, f.vel)).collect();
        // (Neighbours by a grid `NEAR` across: a great shoal's fish look
        // only at those round them, not at all two hundred.)
        let cell = |p: Vec2| (p / NEAR).floor().as_ivec2();
        let mut grid: std::collections::HashMap<IVec2, Vec<usize>> = std::collections::HashMap::new();
        for (j, &(p, _)) in snapshot.iter().enumerate() {
            grid.entry(cell(p)).or_default().push(j);
        }
        for (i, f) in s.fish.iter_mut().enumerate() {
            let (mut apart, mut along, mut mid, mut n) = (Vec2::ZERO, Vec2::ZERO, Vec2::ZERO, 0.0);
            let c = cell(f.pos);
            let round = (-1..=1).flat_map(|dy| (-1..=1).map(move |dx| c + IVec2::new(dx, dy)));
            // (A fish heeds its nearest few, as a real one does: in a
            // great shoal's thick, this many is plenty.)
            let mut heeded = 0;
            for j in round.filter_map(|k| grid.get(&k)).flatten().copied() {
                if j == i {
                    continue;
                }
                if heeded >= HEED {
                    break;
                }
                heeded += 1;
                let (p, v) = snapshot[j];
                let d = f.pos - p;
                let l = d.length();
                if l < APART {
                    apart += d / l.max(0.3);
                }
                if l < WITH {
                    along += v;
                }
                if l < NEAR {
                    mid += p;
                    n += 1.0;
                }
            }
            let mut want = f.vel;
            if n > 0.0 {
                want += (mid / n - f.pos) * PULL.2 * dt * 4.0;
                want += (along / n.max(1.0) - f.vel) * PULL.1 * dt;
            }
            want += apart * PULL.0 * dt * 4.0;
            want += (cruise - f.vel) * 0.6 * dt;
            // A wander of its own.
            want += Vec2::from_angle(f.phase + t * 1.3) * 6.0 * dt;
            // Away from what comes near (you, anything alive).
            let mut fleeing = false;
            for &b in &near {
                let d = f.pos - b;
                if d.length() < FLEE {
                    want += d.normalize_or_zero() * FLEE_SPEED * 3.0 * dt;
                    fleeing = true;
                }
            }
            // Keep to the water: what's ahead not water, turn off it.
            let dir = want.normalize_or(Vec2::X);
            if !water(&sim, id, f.pos + dir * AHEAD) {
                let side = Vec2::new(-dir.y, dir.x);
                let turn = if water(&sim, id, f.pos + side * AHEAD) { side } else if water(&sim, id, f.pos - side * AHEAD) { -side } else { -dir };
                want = turn * want.length();
            }
            let top = if fleeing { FLEE_SPEED } else { s.speed * 1.4 };
            let l = want.length().clamp(SPEED.0 * 0.5, top);
            f.vel = want.normalize_or(Vec2::X) * l;
            let next = f.pos + f.vel * dt;
            if water(&sim, id, next) {
                f.pos = next;
            } else {
                f.vel = -f.vel * 0.5;
            }
        }
    }
}

/// Each fish a few pixels along its way, glowing, glinting as its side
/// turns your way (the shoal wheeling: a ripple of glints through it);
/// each shoal's light at its middle.
#[allow(clippy::too_many_arguments)]
fn show(
    mut commands: Commands,
    time: Res<Time>,
    shoals: Res<Shoals>,
    mut canvas: ResMut<FishCanvas>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<(&GlobalTransform, &crate::world::ChunkLoader), With<crate::camera::MainCamera>>,
    mut sprites: crate::canvas::CanvasSprites,
    mut lights: Query<&mut Transform, (Without<crate::canvas::CanvasSprite>, Without<crate::camera::MainCamera>)>,
) {
    let Ok((cam, loader)) = camera.single() else { return };
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, !shoals.shoals.is_empty()) else { return };
    let t = time.elapsed_secs();
    for s in &shoals.shoals {
        let mut mid = Vec2::ZERO;
        for f in &s.fish {
            mid += f.pos;
            let dir = f.vel.normalize_or(Vec2::X);
            // (Its side to you: swimming across, turning; a wave of it runs
            // through the shoal.)
            let side = dir.y.abs();
            let wave = (f.pos.x * 0.25 + f.pos.y * 0.15 - t * 6.0 + f.phase * 0.1).sin();
            let glint = (side * 0.6 + wave * 0.5).clamp(0.0, 1.0).powi(6);
            let c: [f32; 3] = std::array::from_fn(|k| BODY[k] + (GLINT[k] - BODY[k]) * glint);
            for k in 0..3 {
                let p = (f.pos - dir * (k as f32 * LONG / 2.0)).floor().as_ivec2();
                let a = BODY[3] * (1.0 - k as f32 * 0.3);
                px.put(p.x, p.y, [(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, (a * 255.0) as u8]);
            }
        }
        if let Ok(mut tf) = lights.get_mut(s.light) {
            mid /= s.fish.len().max(1) as f32;
            tf.translation.x = mid.x;
            tf.translation.y = mid.y;
        }
    }
}
