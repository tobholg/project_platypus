//! Channelled spells at the cursor, open while the wand is held and paid
//! for (mana a second, none coming back meanwhile): the gravity well
//! (`Carrier::Well`) and force (`Carrier::Force`, push with the left
//! button, pull with the right). Both follow the cursor on a spring with
//! heft, bend the picture around them (`warp.rs`) and let go when the wand
//! is.
//!
//! **A well** pulls in what's loose (powder, liquid, plants, particles in
//! flight) and tears out solids up to its strength, and holds it all in a
//! spinning ball, up to what it can `lift` (a cell weighs its density; a
//! body its size): each held cell steers toward its place in the ball with
//! a limited grip, so whip the cursor and the outer cells can't keep up and
//! fly off (past 1.4 × the reach) with the speed they had. Creatures within
//! what's left of the lift are carried in its heart (not its caster);
//! heavier ones are only tugged. Held rock grinds whatever it's inside. Let
//! go and it all drops, keeping its momentum.
//!
//! **Force** flings everything within its reach outward (push) or inward
//! (pull), strongest nearest: loose cells and soft solids torn out as
//! flying cells, particles in flight, bodies (not its caster) knocked about.

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
/// The field's spring toward the cursor (1/s²), critically damped, its top
/// speed (cells/s) and most acceleration (cells/s²): its heft.
const FOLLOW: f32 = 110.0;
const TOP_SPEED: f32 = 260.0;
const HEFT: f32 = 2400.0;
/// How fast a held cell corrects toward its place in the ball (1/s).
const STEER: f32 = 10.0;
/// Spin of the ball (radians/s), give or take 30 % a cell.
const SPIN: f32 = 2.6;
/// Held things this far out (× reach) have slipped away.
const SLIP: f32 = 1.4;
/// Grinding: damage a tick for each solid held cell inside a body, per
/// cell/s of speed between them.
const GRIND: f32 = 0.0005;
/// How hard a well tugs a body too heavy to lift (cells/s²).
const TUG: f32 = 180.0;
/// Every so many ticks after tearing solids out, what they held up is let go.
const LOOSEN_EVERY: u32 = 6;
/// Gravity on bodies (cancelled while carried).
const GRAVITY: f32 = 900.0;

/// What the field is doing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Hold,
    Push,
    Pull,
}

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
    pub mode: Mode,
    pos: Vec2,
    vel: Vec2,
    target: Vec2,
    /// Kept open this tick (the wand is still held and paid for).
    fed: bool,
    reach: f32,
    strength: u8,
    pull: u32,
    /// What a well can lift, and what it's carrying (cells and bodies).
    lift: f32,
    carried: f32,
    grip: f32,
    /// Force: how hard it flings (cells/s at the heart).
    power: f32,
    cast: Arc<Cast>,
    held: Vec<Floating>,
    bodies: Vec<(Entity, f32)>,
    tick: u32,
    rng: Rng,
}

impl Well {
    /// Kept open another tick, aimed at `toward` (force: `pull` on the
    /// right button).
    pub fn feed(&mut self, toward: Vec2, pull: bool) {
        self.fed = true;
        self.target = toward;
        if self.mode != Mode::Hold {
            self.mode = if pull { Mode::Pull } else { Mode::Push };
        }
    }

    /// Cells it's holding.
    pub fn holding(&self) -> usize {
        self.held.len()
    }

    /// Bodies it's carrying.
    pub fn carrying(&self) -> usize {
        self.bodies.len()
    }

    /// How hard it bends the picture, 0..1.
    fn bend(&self) -> f32 {
        match self.mode {
            Mode::Hold => 0.35 + 0.65 * (self.carried / self.lift.max(1.0) * 1.5).min(1.0),
            Mode::Push | Mode::Pull => 0.6 + 0.15 * (self.tick as f32 * 0.35).sin(),
        }
    }
}

/// A cell's weight in a well: its density against water's.
fn weight_of(mats: &platypus_sim::MaterialTable, c: Cell) -> f32 {
    (mats.phys(c.material).density as f32 / 1000.0).clamp(0.2, 5.0)
}

/// A body's weight: its size in cells.
fn body_weight(k: &Kinematics) -> f32 {
    4.0 * k.body.half.x * k.body.half.y
}

/// Open a well or force field at `at` (`pull`: force pulling).
pub fn spawn_field(commands: &mut Commands, cast: Arc<Cast>, caster: Entity, at: Vec2, pull: bool) -> Option<Entity> {
    let (mode, reach, strength, tries, lift, grip, power) = match cast.carrier {
        Carrier::Well { radius, strength, lift, pull, grip, .. } => (Mode::Hold, radius, strength, pull, lift, grip, 0.0),
        Carrier::Force { radius, power, strength, pull: tries, .. } => (if pull { Mode::Pull } else { Mode::Push }, radius, strength, tries, 0.0, 0.0, power),
        _ => return None,
    };
    let (r, g, b) = cast.color;
    let e = commands
        .spawn((
            Name::new("Field"),
            Well {
                caster,
                mode,
                pos: at,
                vel: Vec2::ZERO,
                target: at,
                fed: true,
                reach,
                strength,
                pull: tries,
                lift,
                carried: 0.0,
                grip,
                power,
                cast,
                held: Vec::new(),
                bodies: Vec::new(),
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

fn unit(rng: &mut Rng) -> f32 {
    rng.next_u32() as f32 / u32::MAX as f32
}

/// How likely a field takes a cell (a random pick within its reach, `near`
/// 0 at the rim … 1 at the heart): loose things easily, solids up to its
/// strength harder the harder they are.
fn odds(ph: &platypus_sim::material::MatPhys, strength: u8, near: f32) -> f32 {
    match ph.kind {
        Kind::Powder | Kind::Liquid | Kind::Plant => 0.3 + 0.7 * near,
        Kind::Static if ph.hardness < u8::MAX && ph.hardness <= strength => (1.0 - ph.hardness as f32 / (strength as f32 + 1.0)).sqrt() * near * 0.9,
        _ => 0.0,
    }
}

type Bodies<'w, 's> = Query<'w, 's, (Entity, &'static mut Kinematics, Option<&'static mut Health>)>;

/// Fields pull, hold, spin, spill, fling and drop.
pub fn channel(mut commands: Commands, mut sim: ResMut<SimWorld>, mut wells: Query<(Entity, &mut Well)>, mut bodies: Bodies, mut sparks: ResMut<Sparks>) {
    for (e, mut well) in &mut wells {
        let well = &mut *well;
        let world = &mut sim.world;
        if !well.fed {
            // Let go: it all drops, as it was moving; bodies fall free.
            for f in &well.held {
                drop_cell(world, f);
            }
            for &(b, _) in &well.bodies {
                if let Ok((_, mut k, _)) = bodies.get_mut(b) {
                    let k = &mut *k;
                    let v = k.body.vel;
                    k.loco.knock(&mut k.body, v, 0.3);
                }
            }
            commands.entity(e).despawn();
            continue;
        }
        well.fed = false;
        well.tick += 1;

        // Follow the cursor, with heft.
        let damp = 2.0 * FOLLOW.sqrt();
        let acc = ((well.target - well.pos) * FOLLOW - well.vel * damp).clamp_length_max(HEFT);
        well.vel = (well.vel + acc * DT).clamp_length_max(TOP_SPEED);
        well.pos += well.vel * DT;
        let (center, reach) = (well.pos, well.reach);
        let mats = world.materials().clone();

        if well.mode != Mode::Hold {
            force(world, well, &mut bodies, &mut sparks);
            continue;
        }

        // Pull in what it can reach and lift: particles in flight first,
        // then cells, nearer and softer first.
        let mut tore = false;
        let room = ((well.lift - well.carried).max(0.0) as usize).min(well.pull as usize);
        for p in world.take_particles([center.x, center.y], reach, room / 2) {
            well.carried += weight_of(&mats, p.cell);
            let (depth, spin) = (unit(&mut well.rng), SPIN * (0.7 + 0.6 * unit(&mut well.rng)));
            well.held.push(Floating { cell: p.cell, pos: Vec2::new(p.pos[0], p.pos[1]), vel: Vec2::new(p.vel[0], p.vel[1]) * TICK_HZ as f32, depth, spin });
        }
        for _ in 0..well.pull {
            if well.carried >= well.lift {
                break;
            }
            let rng = &mut well.rng;
            let (a, d) = (unit(rng) * std::f32::consts::TAU, unit(rng).sqrt() * reach);
            let at = center + Vec2::from_angle(a) * d;
            let p = CellPos::from_world(at.x, at.y);
            let Some(c) = world.get(p) else { continue };
            if c.is_air() {
                continue;
            }
            let ph = mats.phys(c.material);
            if unit(rng) >= odds(ph, well.strength, 1.0 - d / reach) {
                continue;
            }
            let Some(mut cell) = world.pluck(p) else { continue };
            tore |= ph.kind == Kind::Static;
            cell.vy = 0;
            well.carried += weight_of(&mats, cell);
            let (depth, spin) = (unit(&mut well.rng), SPIN * (0.7 + 0.6 * unit(&mut well.rng)));
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
                well.carried -= weight_of(&mats, f.cell);
                drop_cell(world, &f);
            } else {
                kept.push(f);
            }
        }
        well.held = kept;

        // Bodies: carried in its heart while it can lift them (not its
        // caster), otherwise tugged; held rock grinds what it's inside.
        for (b, mut k, health) in &mut bodies {
            let to = center - k.body.pos;
            let d = to.length();
            let held = well.bodies.iter().position(|&(e, _)| e == b);
            if b != well.caster {
                let w = body_weight(&k);
                match held {
                    Some(i) if d > reach * SLIP => {
                        // Flung loose.
                        well.carried -= well.bodies.swap_remove(i).1;
                    }
                    Some(_) => carry(&mut k, to, vel, grip),
                    None if d <= reach && well.carried + w <= well.lift => {
                        well.bodies.push((b, w));
                        well.carried += w;
                        carry(&mut k, to, vel, grip);
                    }
                    None if d <= reach => {
                        let k = &mut *k;
                        k.body.vel += to.normalize_or_zero() * TUG * (1.0 - d / reach) * DT;
                    }
                    None => {}
                }
            }
            if d <= reach
                && let Some(mut h) = health
            {
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
        // (Bodies gone from the world are gone from its hold.)
        let before = well.bodies.len();
        well.bodies.retain(|&(b, _)| bodies.contains(b));
        if well.bodies.len() != before {
            well.carried = well.held.iter().map(|f| weight_of(&mats, f.cell)).sum::<f32>() + well.bodies.iter().map(|b| b.1).sum::<f32>();
        }

        // Motes swirling in toward it.
        for t in &well.cast.trails {
            let a = unit(&mut well.rng) * std::f32::consts::TAU;
            let at = center + Vec2::from_angle(a) * reach * 0.9;
            let inward = (center - at).normalize_or_zero();
            sparks.emit(t, t.count.round() as usize, at, inward.perp() + inward * 0.6, well.vel);
        }
    }
}

/// A carried body: steered to the ball's heart with the grip, gravity
/// cancelled, stunned so it can't walk out.
fn carry(k: &mut Kinematics, to: Vec2, vel: Vec2, grip: f32) {
    let want = vel + to * 6.0;
    let acc = ((want - k.body.vel) * STEER).clamp_length_max(grip);
    let v = k.body.vel + (acc + Vec2::new(0.0, GRAVITY)) * DT;
    k.loco.knock(&mut k.body, v, 0.1);
}

/// Force, push or pull, for a tick.
fn force(world: &mut platypus_sim::World, well: &mut Well, bodies: &mut Bodies, sparks: &mut Sparks) {
    let (center, reach) = (well.pos, well.reach);
    let sign = if well.mode == Mode::Push { 1.0 } else { -1.0 };
    let mats = world.materials().clone();
    // Cells: torn out and flung, outermost first for a push (innermost for
    // a pull), so the ones in front make way for those behind them in the
    // same tick and a pile blows apart; only what has somewhere to go (a
    // buried grain would land straight back where it was); at most `pull`
    // a tick. A push lifts a little.
    let mut tore = false;
    let mut moved = 0;
    let r = reach.ceil() as i32;
    let mut order: Vec<(i32, i32, f32)> = (-r..=r).flat_map(|dy| (-r..=r).map(move |dx| (dx, dy, ((dx * dx + dy * dy) as f32).sqrt()))).filter(|c| c.2 <= reach).collect();
    order.sort_by(|a, b| if sign > 0.0 { b.2.total_cmp(&a.2) } else { a.2.total_cmp(&b.2) });
    let lift = if sign > 0.0 { Vec2::new(0.0, 0.35) } else { Vec2::ZERO };
    let base = CellPos::from_world(center.x, center.y);
    for (dx, dy, d) in order {
        if moved >= well.pull {
            break;
        }
        let p = CellPos::new(base.x + dx, base.y + dy);
        let Some(c) = world.get(p) else { continue };
        if c.is_air() {
            continue;
        }
        let near = 1.0 - d / reach;
        let ph = mats.phys(c.material);
        let rng = &mut well.rng;
        if unit(rng) >= odds(ph, well.strength, near) {
            continue;
        }

        let at = Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5);
        // Straight out (or in); blocked, off whatever's in the way like a
        // blast off the ground: mirrored upward, then flat to the side.
        let straight = ((at - center).normalize_or(Vec2::Y) * sign + lift).normalize();
        let side = if straight.x < 0.0 { -1.0 } else { 1.0 };
        let free = |d: Vec2| {
            let ahead = CellPos::from_world(at.x + d.x * 1.5, at.y + d.y * 1.5);
            world.get(ahead).is_some_and(|c| c.is_air() || matches!(mats.phys(c.material).kind, Kind::Gas | Kind::Fire))
        };
        let Some(dir) = [straight, Vec2::new(straight.x, straight.y.abs()).normalize(), Vec2::new(side, 0.25).normalize()].into_iter().find(|&d| free(d)) else { continue };
        let Some(mut cell) = world.pluck(p) else { continue };
        moved += 1;
        tore |= ph.kind == Kind::Static;
        if ph.kind == Kind::Static {
            cell.flags |= platypus_sim::cell::flags::LOOSE;
        }
        let speed = well.power * (0.35 + 0.65 * near) * (0.7 + 0.3 * unit(rng)) / TICK_HZ as f32;
        let v = dir * speed + well.vel / TICK_HZ as f32;
        world.emit(Particle::new([at.x, at.y], [v.x, v.y], cell, 300, Landing::Settle));
    }
    if tore && well.tick.is_multiple_of(LOOSEN_EVERY) {
        world.loosen_fragments(CellPos::from_world(center.x, center.y), reach as i32 + 6);
    }
    // Particles in flight: shoved.
    for p in world.particles_mut() {
        let d = Vec2::new(p.pos[0], p.pos[1]) - center;
        let dist = d.length();
        if dist < reach && dist > 0.01 {
            let kick = d / dist * sign * well.power * 0.08 * (1.0 - dist / reach) / TICK_HZ as f32;
            p.vel[0] += kick.x;
            p.vel[1] += kick.y;
        }
    }
    // Bodies (not its caster): launched outward, a little up (a push), or
    // hauled in (a pull), at least as fast as it flings at their distance;
    // stunned till they land.
    for (b, mut k, _) in bodies.iter_mut() {
        let d = k.body.pos - center;
        let dist = d.length();
        if b == well.caster || dist > reach {
            continue;
        }
        let k = &mut *k;
        let near = 1.0 - dist / reach;
        let dir = d.normalize_or(Vec2::Y) * sign;
        let lift = if sign > 0.0 { Vec2::new(0.0, 0.35) } else { Vec2::new(0.0, 0.15) };
        let want = (dir + lift).normalize() * well.power * (0.45 + 0.55 * near);
        let along = k.body.vel.dot(want.normalize());
        let v = if along < want.length() { k.body.vel + want.normalize() * (want.length() - along) } else { k.body.vel };
        k.loco.knock(&mut k.body, v, 0.45);
    }
    // Motes: out from the heart, or in from the rim.
    for t in &well.cast.trails {
        let a = unit(&mut well.rng) * std::f32::consts::TAU;
        let dir = Vec2::from_angle(a);
        let (at, heading) = if sign > 0.0 { (center + dir * 2.0, dir) } else { (center + dir * reach, -dir) };
        sparks.emit(t, t.count.round() as usize, at, heading, well.vel);
    }
}

/// Draw what wells hold (each cell its own colour), move their light, and
/// bend the picture around the strongest.
pub fn show(
    time: Res<Time>,
    sim: Res<SimWorld>,
    mut wells: Query<(&Well, &mut Transform)>,
    mut sparks: ResMut<Sparks>,
    camera: Single<(&Camera, &GlobalTransform, &mut super::warp::Warp), With<MainCamera>>,
) {
    let mats = sim.materials();
    let (cam, cam_tf, mut warp) = camera.into_inner();
    let mut best: Option<(f32, Vec2, f32, Mode)> = None;
    let t = time.elapsed_secs();
    for (well, mut tf) in &mut wells {
        tf.translation.x = well.pos.x;
        tf.translation.y = well.pos.y;
        // Held cells shimmer with the well's colour; some glow faintly.
        let (wr, wg, wb) = well.cast.color;
        let tint = [wr as f32 / 255.0, wg as f32 / 255.0, wb as f32 / 255.0];
        for (i, f) in well.held.iter().enumerate() {
            let c = mats.color(f.cell);
            let k = 0.12 + 0.12 * (t * 5.0 + i as f32 * 0.7).sin();
            let rgb: [f32; 3] = std::array::from_fn(|j| c[j] as f32 / 255.0 * (1.0 - k) + tint[j] * k);
            if i % 5 == 0 {
                sparks.draw_now(f.pos, 3.0, [tint[0], tint[1], tint[2], 0.12]);
            }
            sparks.draw_now(f.pos, 1.0, [rgb[0], rgb[1], rgb[2], 1.0]);
        }
        let bend = well.bend();
        if best.is_none_or(|b| bend > b.0) {
            best = Some((bend, well.pos, well.reach, well.mode));
        }
    }
    let size = cam.logical_viewport_size().unwrap_or(Vec2::ONE);
    warp.strength = 0.0;
    let Some((bend, at, reach, mode)) = best else { return };
    let (Ok(c), Ok(edge)) = (cam.world_to_viewport(cam_tf, at.extend(0.0)), cam.world_to_viewport(cam_tf, (at + Vec2::new(reach * 1.3, 0.0)).extend(0.0))) else { return };
    warp.center = c / size;
    warp.radius = (edge.x - c.x).abs() / size.y;
    warp.strength = bend;
    warp.aspect = size.x / size.y;
    warp.time = t;
    // A well and a pull draw the picture in; a push swells it and ripples out.
    warp.push = if mode == Mode::Push { 1.0 } else { 0.0 };
}

/// The camera wears the (off) warp from the start.
pub fn give_warp(mut commands: Commands, camera: Query<Entity, (With<MainCamera>, Without<super::warp::Warp>)>) {
    for e in &camera {
        commands.entity(e).insert(super::warp::Warp::default());
    }
}
