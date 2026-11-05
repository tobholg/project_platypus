//! Weightless (the zerog arc): in the void over the sky, and wherever
//! gravity's all but gone, bodies float (`Locomotion::float`: holding on
//! to what they touch, pushing off it, drifting on), and what they do
//! pushes them: every shot, strike and blow kicks its maker the other way
//! (`drift::Recoil`; on a drifter, the drifter takes it: `drift`), and
//! from off its middle sets it turning (`Turning`: the whole of it drawn
//! turned, gear and all; righted once it holds on to something).

use bevy::prelude::*;

use crate::creatures::Kinematics;
use crate::drift::Recoil;
use crate::world::{SimWorld, TickSet};

pub struct ZeroGPlugin;

impl Plugin for ZeroGPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, kick.after(crate::drift::drift).before(crate::creatures::move_creatures).in_set(TickSet::Bodies))
            .add_systems(FixedUpdate, kick_debris.after(kick).before(crate::creatures::move_creatures).in_set(TickSet::Bodies))
            .add_systems(PostUpdate, turn.after(crate::creatures::interpolate).before(bevy::transform::TransformSystems::Propagate));
    }
}

/// Under this much gravity (a share of the ground's) a body floats.
pub const WEIGHTLESS: f32 = 1.0 / 6.0;

/// A pickaxe's blow (back off what it struck), as `drift`'s kicks.
pub const KICK_PICK: f32 = 3_000.0;
/// Struck (a sword, a fist), the striker's pushed back this share of the
/// blow's knock (no more than the knock itself, whatever it struck).
const BLOW_BACK: f32 = 0.5;
/// A blow going down more steeply than this (its direction's share down)
/// is a down-strike: no blow-back, its pogo bounces it.
const DOWNWARD: f32 = 0.7;
/// A kick from off its middle (the hand's out to one side) sets it
/// turning: this share of what it would as a stiff rod its height.
const TORQUE: f32 = 0.25;
/// Holding on to something (or with weight again), it rights itself this
/// fast (a share of the way a second).
const RIGHTING: f32 = 8.0;

/// Weightless and turning: how far it's turned from upright (radians,
/// counter-clockwise) and how fast it turns. Looks only: its box doesn't
/// turn.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct Turning {
    pub angle: f32,
    pub omega: f32,
}

/// How much gravity there is at `y` (1: the ground's … 0: none).
pub fn gravity_at(sim: &SimWorld, y: f32) -> f32 {
    sim.world.gravity(y)
}

/// How a body moves under `g` of the ground's gravity: it falls slower,
/// and a jump (as fast off the ground as ever) goes higher.
pub fn lighter(stats: &platypus_physics::MovementStats, g: f32) -> platypus_physics::MovementStats {
    platypus_physics::MovementStats { gravity: stats.gravity * g, jump_height: stats.jump_height / g.max(0.05), ..stats.clone() }
}

/// Gravity in eighths (8: the ground's), for what's made for one gravity
/// (a creature's jumps: `brain::way`); never under a quarter (under that
/// it all but floats).
pub fn eighths(g: f32) -> u8 {
    (g * 8.0).round().clamp(2.0, 8.0) as u8
}

/// Recoils and blows push their maker, weightless and holding on to
/// nothing (riding a drifter, it's the drifter's), the harder the lighter
/// they are, and (from off its middle) set it tumbling.
fn kick(mut commands: Commands, sim: Res<SimWorld>, mut recoils: MessageReader<Recoil>, mut hits: MessageReader<crate::combat::Hit>, riders: Res<crate::drift::Riders>, mut q: Query<(&mut Kinematics, Option<&mut Turning>)>) {
    let mut pushes: Vec<(Entity, Vec2, Vec2, bool)> = recoils.read().map(|r| (r.who, r.from, r.push, r.turns)).collect();
    for h in hits.read() {
        // (A strike downward is a pogo: that's its bounce, `combat`.)
        let Some(by) = h.by.filter(|_| h.dir.y > -DOWNWARD) else { continue };
        let Ok([(striker, _), (struck, _)]) = q.get_many([by, h.target]) else { continue };
        let share = (crate::drift::weight(&struck.body) / crate::drift::weight(&striker.body)).min(1.0);
        pushes.push((by, h.at, -h.knock * share * BLOW_BACK * crate::drift::weight(&striker.body), true));
    }
    for (who, from, push, turns) in pushes {
        if riders.on.contains_key(&who) {
            continue;
        }
        let Ok((mut k, turning)) = q.get_mut(who) else { continue };
        let c = k.loco.contacts;
        let g = gravity_at(&sim, k.body.pos.y);
        if g >= WEIGHTLESS || k.body.pos.distance(from) > crate::drift::HAND || c.ground || c.ceiling || c.wall_left || c.wall_right {
            continue;
        }
        let m = crate::drift::weight(&k.body);
        let dv = push * (1.0 - g) / m;
        k.body.vel += dv;
        if !turns {
            continue;
        }
        // (Turned about its middle by a push from its hand, as a rod its
        // height would be: ω = r × Δv / (h² / 3).)
        let h = k.body.half.y;
        let r = (from - k.body.pos).clamp_length_max(h);
        let spin = (r.x * dv.y - r.y * dv.x) / (h * h / 3.0) * TORQUE;
        match turning {
            Some(mut t) => t.omega += spin,
            None => {
                commands.entity(who).insert(Turning { angle: 0.0, omega: spin });
            }
        }
    }
}

/// Turning as it drifts, drawn turned (the whole of it); holding on to
/// something, or out of the void's weightlessness, righted again.
pub fn turn(mut commands: Commands, time: Res<Time>, sim: Res<SimWorld>, mut q: Query<(Entity, &Kinematics, &mut Turning, &mut Transform)>) {
    let dt = time.delta_secs();
    for (e, k, mut t, mut tf) in &mut q {
        let c = k.loco.contacts;
        let held = c.ground || c.ceiling || c.wall_left || c.wall_right || gravity_at(&sim, k.body.pos.y) >= WEIGHTLESS;
        if held {
            t.omega = 0.0;
            t.angle = crate::creatures::body::tumble::wrap(t.angle) * (-dt * RIGHTING).exp();
            if t.angle.abs() < 0.01 {
                tf.rotation = Quat::IDENTITY;
                commands.entity(e).remove::<Turning>();
                continue;
            }
        } else {
            t.angle = crate::creatures::body::tumble::wrap(t.angle + t.omega * dt);
        }
        tf.rotation = Quat::from_rotation_z(t.angle);
    }
}

/// A clump of solid this small (cells), touching nothing bigger, is debris
/// to a weightless body: no wall (`kick_debris`).
const DEBRIS_MOST: usize = 12;
/// What a body knocks loose flies off at its speed, give or take this share
/// (and as much again sideways).
const DEBRIS_SCATTER: f32 = 0.25;

/// Weightless, what's floating loose in a body's way (a speck of dust, a
/// chip, a crumb: a clump of `DEBRIS_MOST` cells or fewer, touching nothing
/// bigger) isn't a wall: the body knocks it aside, flying off at about
/// its speed, and loses only as much of its own as the clump's weight is
/// of its own. (Rock, an asteroid, stops it as ever.)
pub fn kick_debris(mut sim: ResMut<SimWorld>, mut q: Query<&mut Kinematics>) {
    use platypus_sim::{CellPos, Kind, Landing, Particle};
    let tick = sim.world.tick();
    for mut k in &mut q {
        if gravity_at(&sim, k.body.pos.y) >= WEIGHTLESS || k.body.vel.length_squared() < 1.0 {
            continue;
        }
        let k = &mut *k;
        // (Where it's going this tick, and a cell round it.)
        let step = k.body.vel / crate::world::TICK_HZ as f32;
        let (lo, hi) = (k.body.pos.min(k.body.pos + step) - k.body.half - 1.0, k.body.pos.max(k.body.pos + step) + k.body.half + 1.0);
        let world = &mut sim.world;
        let solid = |w: &platypus_sim::World, p: CellPos| w.get(p).is_some_and(|c| !c.is_air() && c.flags & platypus_sim::cell::flags::HELD == 0 && matches!(w.materials().phys(c.material).kind, Kind::Static | Kind::Powder));
        let mut seen: Vec<CellPos> = Vec::new();
        let mut knocked: Vec<CellPos> = Vec::new();
        for y in lo.y.floor() as i32..=hi.y.floor() as i32 {
            for x in lo.x.floor() as i32..=hi.x.floor() as i32 {
                let p = CellPos::new(x, y);
                if seen.contains(&p) || !solid(world, p) {
                    continue;
                }
                // Its clump (eight ways), as far as `DEBRIS_MOST` and one.
                let mut clump = vec![p];
                let mut i = 0;
                while i < clump.len() && clump.len() <= DEBRIS_MOST {
                    let c = clump[i];
                    i += 1;
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                        let n = c.offset(dx, dy);
                        if !clump.contains(&n) && solid(world, n) {
                            clump.push(n);
                        }
                    }
                }
                seen.extend(clump.iter().copied());
                if clump.len() <= DEBRIS_MOST {
                    knocked.extend(clump);
                }
            }
        }
        if knocked.is_empty() {
            continue;
        }
        let mut rng = platypus_sim::rng::Rng::seeded(&[tick, k.body.pos.x.to_bits() as u64, 0xDEB2]);
        let mut unit = || rng.next_u32() as f32 / u32::MAX as f32 - 0.5;
        let v = k.body.vel / crate::world::TICK_HZ as f32;
        let side = v.perp();
        for &p in &knocked {
            let Some(mut c) = world.get(p) else { continue };
            world.set(p, platypus_sim::Cell::AIR);
            c.flags &= !platypus_sim::cell::flags::LOOSE;
            let fly = v * (1.0 + unit() * 2.0 * DEBRIS_SCATTER) + side * unit() * 2.0 * DEBRIS_SCATTER;
            world.emit(Particle::new([p.x as f32 + 0.5, p.y as f32 + 0.5], [fly.x, fly.y], c, 600, Landing::Settle));
        }
        // (It gave the clump its speed: as much of its own as it weighs to it.)
        let mass = (k.body.half.x * k.body.half.y * 4.0).max(1.0);
        k.body.vel *= (1.0 - knocked.len() as f32 / mass).max(0.5);
    }
}
