//! Gravity wells (`Carrier::Well`): a channelled spell that lives while its
//! wand is held, at the cursor. It pulls in what's loose (powder, liquid,
//! plants) and tears out solids up to its strength, and holds them in a
//! spinning ball; bodies in its reach float helplessly toward it (not its
//! caster), and what's in the ball grinds whatever is caught in it.
//!
//! The ball has weight: the well follows the cursor on a spring, and each
//! held cell is steered toward its place in the spinning ball with a
//! limited grip. Whip the cursor and the outer cells can't keep up: past
//! 1.4 × the reach they fly off with the speed they had. Let go (or run out
//! of mana) and everything drops as real cells, keeping its momentum: lift a
//! ball of rock, swing it over an orc, let go.

use std::sync::Arc;

use bevy::prelude::*;
use platypus_sim::rng::Rng;
use platypus_sim::{Cell, CellPos, Kind, Landing, Particle};

use super::runes::{Carrier, Cast};
use crate::actors::{Health, Kinematics};
use crate::camera::MainCamera;
use crate::light::LightSource;
use crate::vfx::Sparks;
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// The well's spring toward the cursor (1/s²), critically damped, and its
/// top speed (cells/s).
const FOLLOW: f32 = 110.0;
const TOP_SPEED: f32 = 260.0;
/// Most it can speed up or slow down (cells/s²): its heft.
const HEFT: f32 = 2400.0;
/// How fast a held cell corrects toward its place in the ball (1/s).
const STEER: f32 = 10.0;
/// Spin of the ball (radians/s), give or take 30 % a cell.
const SPIN: f32 = 2.6;
/// Held cells this far out (× reach) have slipped away.
const SLIP: f32 = 1.4;
/// Grinding: damage a tick for each solid held cell inside a body, per
/// cell/s of speed between them.
const GRIND: f32 = 0.0005;
/// How hard it draws bodies in (cells/s²).
const DRAW: f32 = 700.0;
/// Every so many ticks after tearing solids out, what they held up is let go.
const LOOSEN_EVERY: u32 = 6;

/// One held cell.
struct Floating {
    cell: Cell,
    pos: Vec2,
    vel: Vec2,
    /// Where in the ball (0 middle … 1 edge, by area) and how fast it turns.
    depth: f32,
    spin: f32,
}

#[derive(Component)]
pub struct Well {
    pub caster: Entity,
    pos: Vec2,
    prev: Vec2,
    vel: Vec2,
    target: Vec2,
    /// Kept alive this tick (the wand is still held and paid for).
    pub fed: bool,
    reach: f32,
    strength: u8,
    pull: u32,
    most: u32,
    grip: f32,
    cast: Arc<Cast>,
    held: Vec<Floating>,
    tick: u32,
    rng: Rng,
}

impl Well {
    pub fn feed(&mut self, toward: Vec2) {
        self.fed = true;
        self.target = toward;
    }

    /// Cells it's holding.
    pub fn holding(&self) -> usize {
        self.held.len()
    }

    /// How much it's holding, 0..1 (how hard it bends the picture).
    fn fullness(&self) -> f32 {
        (self.held.len() as f32 / self.most.max(1) as f32 * 2.0).min(1.0)
    }
}

/// Start a well at `at`.
pub fn spawn_well(commands: &mut Commands, cast: Arc<Cast>, caster: Entity, at: Vec2) -> Option<Entity> {
    let &Carrier::Well { radius, strength, pull, most, grip, .. } = &cast.carrier else { return None };
    let (r, g, b) = cast.color;
    let e = commands
        .spawn((
            Name::new("Gravity well"),
            Well {
                caster,
                pos: at,
                prev: at,
                vel: Vec2::ZERO,
                target: at,
                fed: true,
                reach: radius,
                strength,
                pull,
                most,
                grip,
                cast,
                held: Vec::new(),
                tick: 0,
                rng: Rng::seeded(&[at.x.to_bits() as u64, at.y.to_bits() as u64, 0x3E11]),
            },
            LightSource { color: [r as f32 / 255.0 * 0.6, g as f32 / 255.0 * 0.6, b as f32 / 255.0 * 0.6], flicker: 0.3 },
            Transform::from_translation(at.extend(12.0)),
        ))
        .id();
    Some(e)
}

/// Let a held cell go into the world, flying as it was.
fn drop_cell(world: &mut platypus_sim::World, f: &Floating) {
    let v = f.vel / TICK_HZ as f32;
    world.emit(Particle::new([f.pos.x, f.pos.y], [v.x, v.y], f.cell, 400, Landing::Settle));
}

/// Wells pull, hold, spin, spill and drop.
pub fn channel(mut commands: Commands, mut sim: ResMut<SimWorld>, mut wells: Query<(Entity, &mut Well)>, mut bodies: Query<(Entity, &mut Kinematics, Option<&mut Health>)>, mut sparks: ResMut<Sparks>) {
    for (e, mut well) in &mut wells {
        let well = &mut *well;
        let world = &mut sim.world;
        if !well.fed {
            // Let go: it all drops, as it was moving.
            for f in &well.held {
                drop_cell(world, f);
            }
            commands.entity(e).despawn();
            continue;
        }
        well.fed = false;
        well.tick += 1;

        // Follow the cursor, with weight.
        well.prev = well.pos;
        let damp = 2.0 * FOLLOW.sqrt();
        let acc = ((well.target - well.pos) * FOLLOW - well.vel * damp).clamp_length_max(HEFT);
        well.vel = (well.vel + acc * DT).clamp_length_max(TOP_SPEED);
        well.pos += well.vel * DT;
        let (center, reach) = (well.pos, well.reach);

        // Pull in what it can reach, nearer first, softer first.
        let mats = world.materials().clone();
        let mut tore = false;
        for _ in 0..well.pull {
            if well.held.len() >= well.most as usize {
                break;
            }
            let rng = &mut well.rng;
            let unit = |rng: &mut Rng| rng.next_u32() as f32 / u32::MAX as f32;
            let (a, d) = (unit(rng) * std::f32::consts::TAU, unit(rng).sqrt() * reach);
            let at = center + Vec2::from_angle(a) * d;
            let p = CellPos::from_world(at.x, at.y);
            let Some(c) = world.get(p) else { continue };
            if c.is_air() {
                continue;
            }
            let ph = mats.phys(c.material);
            let near = 1.0 - d / reach;
            let odds = match ph.kind {
                Kind::Powder | Kind::Liquid | Kind::Plant => 0.3 + 0.7 * near,
                // Solids up to its strength: the softer, the easier.
                Kind::Static if ph.hardness < u8::MAX && ph.hardness <= well.strength => {
                    (1.0 - ph.hardness as f32 / (well.strength as f32 + 1.0)).sqrt() * near * 0.9
                }
                _ => 0.0,
            };
            if unit(rng) >= odds {
                continue;
            }
            let Some(mut cell) = world.pluck(p) else { continue };
            tore |= ph.kind == Kind::Static;
            cell.vy = 0;
            let rng = &mut well.rng;
            let (depth, spin) = (unit(rng), SPIN * (0.7 + 0.6 * unit(rng)));
            well.held.push(Floating { cell, pos: Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5), vel: well.vel, depth, spin });
        }
        if tore && well.tick.is_multiple_of(LOOSEN_EVERY) {
            world.loosen_fragments(CellPos::from_world(center.x, center.y), reach as i32 + 6);
        }

        // Hold: each cell toward its place in the spinning ball, as far as
        // the grip allows; those flung too far are gone.
        let ball = (well.held.len() as f32 / std::f32::consts::PI).sqrt() * 1.25 + 1.5;
        let (grip, vel) = (well.grip, well.vel);
        let mut kept = Vec::with_capacity(well.held.len());
        for mut f in std::mem::take(&mut well.held) {
            let to = center - f.pos;
            let dist = to.length().max(0.01);
            let n = to / dist;
            let ring = ball * f.depth.sqrt();
            let want = vel + n.perp() * f.spin * ring.max(0.5) + n * (dist - ring) * 8.0;
            let acc = ((want - f.vel) * STEER).clamp_length_max(grip);
            f.vel += acc * DT;
            f.pos += f.vel * DT;
            if (f.pos - center).length() > reach * SLIP {
                drop_cell(world, &f);
            } else {
                kept.push(f);
            }
        }
        well.held = kept;

        // Bodies: drawn in and held floating (not its caster); what's in the
        // ball grinds whatever's caught in it.
        for (b, mut k, health) in &mut bodies {
            let to = center - k.body.pos;
            let d = to.length();
            if d > reach {
                continue;
            }
            if b != well.caster {
                let k = &mut *k;
                let pull = to.normalize_or_zero() * DRAW * (1.0 - d / reach) * DT;
                // (Stunned a moment each tick: it can't walk out.)
                let v = (k.body.vel + pull + Vec2::new(0.0, 900.0 * DT * 0.9)).lerp(vel, 0.05);
                k.loco.knock(&mut k.body, v, 0.1);
            }
            if let Some(mut h) = health {
                let (lo, hi) = (k.body.pos - k.body.half - Vec2::ONE, k.body.pos + k.body.half + Vec2::ONE);
                let hits: f32 = well
                    .held
                    .iter()
                    .filter(|f| matches!(mats.phys(f.cell.material).kind, Kind::Static | Kind::Powder))
                    .filter(|f| f.pos.cmpge(lo).all() && f.pos.cmple(hi).all())
                    .map(|f| (f.vel - k.body.vel).length())
                    .sum();
                h.hp -= hits * GRIND;
            }
        }

        // Motes swirling in toward it.
        for t in &well.cast.trails {
            let rng = &mut well.rng;
            let a = rng.next_u32() as f32 / u32::MAX as f32 * std::f32::consts::TAU;
            let at = center + Vec2::from_angle(a) * reach * 0.9;
            let inward = (center - at).normalize_or_zero();
            sparks.emit(t, t.count.round() as usize, at, inward.perp() + inward * 0.6, well.vel);
        }
    }
}

/// Draw what wells hold (each cell its own colour), move their light, and
/// bend the picture around the fullest.
pub fn show(
    time: Res<Time>,
    sim: Res<SimWorld>,
    mut wells: Query<(&Well, &mut Transform)>,
    mut sparks: ResMut<Sparks>,
    camera: Single<(&Camera, &GlobalTransform, &mut super::warp::Warp), With<MainCamera>>,
) {
    let mats = sim.materials();
    let (cam, cam_tf, mut warp) = camera.into_inner();
    let mut best: Option<(f32, Vec2, f32)> = None;
    for (well, mut tf) in &mut wells {
        tf.translation.x = well.pos.x;
        tf.translation.y = well.pos.y;
        // Held cells shimmer with the well's colour; some glow faintly.
        let (wr, wg, wb) = well.cast.color;
        let tint = [wr as f32 / 255.0, wg as f32 / 255.0, wb as f32 / 255.0];
        let t = time.elapsed_secs();
        for (i, f) in well.held.iter().enumerate() {
            let c = mats.color(f.cell);
            let k = 0.12 + 0.12 * (t * 5.0 + i as f32 * 0.7).sin();
            let rgb: [f32; 3] = std::array::from_fn(|j| c[j] as f32 / 255.0 * (1.0 - k) + tint[j] * k);
            if i % 5 == 0 {
                sparks.draw_now(f.pos, 3.0, [tint[0], tint[1], tint[2], 0.12]);
            }
            sparks.draw_now(f.pos, 1.0, [rgb[0], rgb[1], rgb[2], 1.0]);
        }
        let strength = 0.35 + 0.65 * well.fullness();
        if best.is_none_or(|b| strength > b.0) {
            best = Some((strength, well.pos, well.reach));
        }
    }
    let size = cam.logical_viewport_size().unwrap_or(Vec2::ONE);
    warp.strength = 0.0;
    let Some((strength, at, reach)) = best else { return };
    let (Ok(c), Ok(edge)) = (cam.world_to_viewport(cam_tf, at.extend(0.0)), cam.world_to_viewport(cam_tf, (at + Vec2::new(reach * 1.3, 0.0)).extend(0.0))) else { return };
    warp.center = c / size;
    warp.radius = (edge.x - c.x).abs() / size.y;
    warp.strength = strength;
    warp.aspect = size.x / size.y;
    warp.time = time.elapsed_secs();
}

/// The camera wears the (off) warp from the start.
pub fn give_warp(mut commands: Commands, camera: Query<Entity, (With<MainCamera>, Without<super::warp::Warp>)>) {
    for e in &camera {
        commands.entity(e).insert(super::warp::Warp::default());
    }
}
