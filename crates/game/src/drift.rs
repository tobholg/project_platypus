//! Drifters (the sky arc): rocks floating free about the sky islands, held
//! up by their aerolite.
//!
//! - **Its cells are the world's**: a drifter takes over the cells the seed
//!   made in its box (`Spawn::Drifter`), and moves them a cell at a time,
//!   so it's ground like any other: stood on, mined, burnt. What's mined
//!   off it is gone from it.
//! - **Floating**: it rides at the height it was made at, on a soft spring
//!   (lower, it's lifted harder; higher, less), bobbing a little and
//!   wandering about where it was made when left alone.
//! - **Pushed**: landing on it pushes it down (and along, as you were
//!   going), jumping off pushes it away, running into its side shoves it,
//!   each by your weight against its: a small one's flung, a big one
//!   barely moves. What stands on it weighs it down (it dips under you)
//!   and rides along with it.
//! - **Sinking**: its lift is its aerolite: mine that out and it floats
//!   lower; with under half of it left, it falls, and stays where it lands.

use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use platypus_sim::{Cell, CellPos, Kind};

use crate::creatures::Kinematics;
use crate::world::{SimWorld, TickSet};

pub struct DriftPlugin;

impl Plugin for DriftPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (free, adopt, drift).chain().before(crate::creatures::move_creatures).in_set(TickSet::Bodies));
    }
}

/// Its own gravity (cells/s²: floatier than a creature's), and how far
/// under its rest height its lift has doubled (the spring's span: a bob
/// about three seconds long).
const GRAVITY: f32 = 600.0;
const SPAN: f32 = 150.0;
/// Up and down, its bob dies away at this (a second); across, it slows.
const BOB_DAMP: f32 = 0.45;
const DRAG: f32 = 0.35;
/// Left alone: it bobs this much (cells) and wanders this fast (cells/s)
/// about where it was made, turning this often (Hz), never further than
/// this from it.
const BOB: f32 = 2.5;
const WANDER: f32 = 3.0;
const WANDER_HZ: f32 = 0.05;
const WANDER_REACH: f32 = 60.0;
/// What's on it weighs this a cell of its box (a cell of rock weighs 1).
const WEIGHT: f32 = 0.5;
/// A creature landing on it this fast or faster (cells/s) pushes it.
const LANDS: f32 = 40.0;
/// A creature leaving it going up this fast or faster (cells/s) pushes it
/// away (a rocket's lift-off is slow to start).
const LEAPS: f32 = 5.0;
/// A bounce keeps this much of its speed.
const BOUNCE: f32 = 0.3;
/// With under this share of its aerolite left, its lift can't hold it.
const FALLS: f32 = 0.5;

#[derive(Component)]
pub struct Drifter {
    /// Its box's bottom-left, in cells (where its cells are now), and
    /// exactly.
    pub cell: IVec2,
    pub pos: Vec2,
    pub vel: Vec2,
    /// The height (its box's bottom) it floats at, whole and with nothing
    /// on it; and the x it wanders about.
    pub rest: f32,
    pub home: f32,
    /// Its box when it was made (where its cells are taken over from).
    pub size: IVec2,
    /// Its cells (from `cell`), once taken over from the world.
    stamp: Vec<(IVec2, Cell)>,
    /// Its aerolite then: its whole lift.
    whole: u32,
    /// Its wander's and bob's phase.
    phase: f32,
}

impl Drifter {
    /// One made at `cell` (its box's bottom-left), `size` big: it takes its
    /// cells over once they're all loaded.
    pub fn new(cell: IVec2, size: IVec2) -> Drifter {
        let phase = (cell.x as f32 * 0.37 + cell.y as f32 * 0.11).fract() * 100.0;
        Drifter { cell, pos: cell.as_vec2(), vel: Vec2::ZERO, rest: cell.y as f32, home: cell.x as f32, size, stamp: Vec::new(), whole: 0, phase }
    }

    /// One saved: as it was (where it floats and wanders about).
    pub fn saved(cell: IVec2, size: IVec2, rest: f32, home: f32) -> Drifter {
        Drifter { rest, home, ..Drifter::new(cell, size) }
    }

    /// One cut loose (a piece of a sky island with aerolite in it, mined
    /// free): it floats where it is.
    fn freed(cells: Vec<(IVec2, Cell)>, aerolite: Option<platypus_sim::MaterialId>) -> Drifter {
        let lo = cells.iter().fold(IVec2::MAX, |a, &(p, _)| a.min(p));
        let hi = cells.iter().fold(IVec2::MIN, |a, &(p, _)| a.max(p));
        let whole = cells.iter().filter(|(_, c)| Some(c.material) == aerolite).count().max(1) as u32;
        let stamp = cells.into_iter().map(|(p, c)| (p - lo, c)).collect();
        Drifter { stamp, whole, ..Drifter::new(lo, hi - lo + IVec2::ONE) }
    }

    /// Moved a whole world's width (round a wrapping world: `world::gather`).
    pub fn shift(&mut self, dx: f32) {
        self.cell.x += dx as i32;
        self.pos.x += dx;
        self.home += dx;
    }

    /// Holding its cells (taken over from the world).
    pub fn taken(&self) -> bool {
        !self.stamp.is_empty()
    }
}

/// Is a cell open for a drifter to move into (air, gas, flame; grass and
/// leaves crushed)?
fn open(sim: &SimWorld, c: Cell) -> bool {
    c.is_air() || matches!(sim.world.materials().phys(c.material).kind, Kind::Empty | Kind::Gas | Kind::Fire | Kind::Plant)
}

/// Pieces the world cut loose that hold themselves up (aerolite in them):
/// drifters, unless they're already one's (mined off a drifter, it floats
/// on whole).
fn free(mut commands: Commands, mut sim: ResMut<SimWorld>, drifters: Query<&Drifter>) {
    let freed = sim.world.take_freed();
    if freed.is_empty() {
        return;
    }
    let held: HashSet<IVec2> = drifters.iter().flat_map(|d| d.stamp.iter().map(move |&(p, _)| d.cell + p)).collect();
    let aerolite = sim.materials().id("aerolite_ore");
    for piece in freed {
        if piece.iter().any(|p| held.contains(&IVec2::new(p.x, p.y))) {
            continue;
        }
        let cells: Vec<(IVec2, Cell)> = piece.iter().filter_map(|&p| sim.world.get(p).map(|c| (IVec2::new(p.x, p.y), c))).collect();
        if !cells.is_empty() {
            commands.spawn(Drifter::freed(cells, aerolite));
        }
    }
}

/// Drifters not yet holding their cells take them over, once all of their
/// box is loaded: every solid cell in it (it was made floating free, room
/// round it).
fn adopt(mut commands: Commands, sim: Res<SimWorld>, mut drifters: Query<(Entity, &mut Drifter)>) {
    let aerolite = sim.materials().id("aerolite_ore");
    for (e, mut d) in &mut drifters {
        if d.taken() {
            continue;
        }
        let (c, s) = (d.cell, d.size);
        let corners = [(0, 0), (s.x - 1, 0), (0, s.y - 1), (s.x - 1, s.y - 1), (s.x / 2, s.y / 2)];
        if corners.iter().any(|&(x, y)| sim.world.get(CellPos::new(c.x + x, c.y + y)).is_none()) {
            continue;
        }
        let mut stamp = Vec::new();
        for y in 0..s.y {
            for x in 0..s.x {
                if let Some(cell) = sim.world.get(CellPos::new(c.x + x, c.y + y))
                    && !open(&sim, cell)
                {
                    stamp.push((IVec2::new(x, y), cell));
                }
            }
        }
        if stamp.is_empty() {
            commands.entity(e).despawn();
            continue;
        }
        d.whole = stamp.iter().filter(|(_, cell)| Some(cell.material) == aerolite).count().max(1) as u32;
        d.stamp = stamp;
    }
}

/// What a creature was doing on a drifter last tick: its velocity, and
/// whether it stood on one.
#[derive(Default)]
struct Was(HashMap<Entity, (Vec2, bool)>);

/// Each drifter: its cells checked (what's mined is gone), what's on it
/// weighed, the pushes of what landed, leapt or ran into it, lifted by its
/// aerolite, and moved a cell at a time (bouncing off what's in its way),
/// carrying what's on it.
fn drift(
    mut commands: Commands,
    time: Res<Time>,
    mut sim: ResMut<SimWorld>,
    mut drifters: Query<(Entity, &mut Drifter)>,
    mut creatures: Query<(Entity, &mut Kinematics)>,
    mut was: Local<Was>,
) {
    let dt = time.delta_secs();
    let aerolite = sim.materials().id("aerolite_ore");
    let mut now: HashMap<Entity, (Vec2, bool)> = HashMap::new();
    for (e, mut d) in &mut drifters {
        if !d.taken() {
            continue;
        }
        let cell = d.cell;
        // What's been mined, burnt or blasted off it: gone from it.
        d.stamp.retain(|&(p, c)| sim.world.get(CellPos::new(cell.x + p.x, cell.y + p.y)).is_some_and(|w| w.material == c.material));
        if d.stamp.is_empty() {
            commands.entity(e).despawn();
            continue;
        }
        let own: HashSet<IVec2> = d.stamp.iter().map(|&(p, _)| p).collect();
        let mass = d.stamp.len() as f32;
        let aero = d.stamp.iter().filter(|(_, c)| Some(c.material) == aerolite).count() as f32;
        let lift = aero / d.whole as f32;
        // Who's on it (feet on its cells), who's against its sides.
        let (lo, hi) = (cell, cell + d.size);
        let mut riders: Vec<Entity> = Vec::new();
        let mut load = 0.0;
        for (ce, k) in &creatures {
            let (min, max) = (k.body.pos - k.body.half, k.body.pos + k.body.half);
            if max.x < lo.x as f32 - 2.0 || min.x > hi.x as f32 + 2.0 || max.y < lo.y as f32 - 2.0 || min.y > hi.y as f32 + 2.0 {
                continue;
            }
            let mine = |x: i32, y: i32| own.contains(&IVec2::new(x - cell.x, y - cell.y));
            let feet = (min.y - 0.5).floor() as i32;
            // (On it: feet on its cells, not rising off it; as it sinks, a
            // rider carried down with it is still on it.)
            let on = (min.x.floor() as i32..=(max.x - 0.01).floor() as i32).any(|x| mine(x, feet)) && k.body.vel.y <= d.vel.y.max(0.0) + 1.0;
            let m = 4.0 * k.body.half.x * k.body.half.y * WEIGHT;
            let (prev, was_on) = was.0.get(&ce).copied().unwrap_or((k.body.vel, on));
            if on {
                riders.push(ce);
                load += m;
                // Landed: pushed down (and along, as it was going).
                if !was_on && prev.y < -LANDS {
                    d.vel.y += m * prev.y / (mass + m);
                    d.vel.x += m * (prev.x - d.vel.x) / (mass + m);
                }
            } else if was_on && k.body.vel.y > LEAPS {
                // Leapt off: pushed away.
                d.vel.y -= m * (k.body.vel.y - d.vel.y) / (mass + m);
                d.vel.x -= 0.5 * m * (k.body.vel.x - d.vel.x) / (mass + m);
            }
            // Ran into its side: shoved (as hard as it was going, stopped).
            let side = |x: i32| ((min.y + 0.5).floor() as i32..=(max.y - 0.5).floor() as i32).any(|y| mine(x, y));
            let stopped = k.body.vel.x.abs() < prev.x.abs() * 0.5;
            if stopped && ((prev.x > 0.0 && side(max.x.floor() as i32 + 1)) || (prev.x < 0.0 && side(min.x.floor() as i32 - 1))) {
                d.vel.x += m * (prev.x - d.vel.x) / (mass + m);
            }
            now.insert(ce, (k.body.vel, on));
        }
        // Lifted by its aerolite (harder the further it's sunk under where
        // it floats), weighed down by what's on it; left alone, a little
        // bob and a slow wander about where it was made.
        d.phase += dt;
        let t = d.phase;
        let rest = d.rest + BOB * (t * 0.7).sin();
        let held = if lift < FALLS { lift } else { lift * (1.0 + (rest - d.pos.y) / SPAN).clamp(0.0, 2.0) };
        let mut acc = Vec2::new(0.0, -GRAVITY + GRAVITY * held * mass / (mass + load) - BOB_DAMP * d.vel.y);
        // (In the jet stream, carried off with it: it wanders about where
        // it's got to.)
        let jet = crate::sky::jet_at(&sim, d.pos.y);
        if jet != 0.0 {
            acc.x = (jet - d.vel.x) * DRAG;
            d.home = d.pos.x;
        } else if lift >= FALLS {
            let wander = WANDER * (t * WANDER_HZ * std::f32::consts::TAU).sin() - (d.pos.x - d.home) / WANDER_REACH * WANDER;
            acc.x = (wander - d.vel.x) * DRAG;
        } else {
            acc.x = -d.vel.x * DRAG;
        }
        d.vel += acc * dt;
        let vel = d.vel;
        d.pos += vel * dt;
        // A cell at a time, across then up or down: in its way, a bounce.
        let mut moved = IVec2::ZERO;
        for axis in [0usize, 1] {
            loop {
                let want = d.pos[axis].round() as i32 - d.cell[axis];
                if want == 0 {
                    break;
                }
                let mut step = IVec2::ZERO;
                step[axis] = want.signum();
                let blocked = d.stamp.iter().any(|&(p, _)| {
                    let q = p + step;
                    !own.contains(&q) && sim.world.get(CellPos::new(d.cell.x + q.x, d.cell.y + q.y)).is_none_or(|c| !open(&sim, c))
                });
                if blocked {
                    // (Landed with no lift left: it stays, ground now.)
                    if axis == 1 && step.y < 0 && lift < FALLS {
                        commands.entity(e).despawn();
                    }
                    d.vel[axis] *= -BOUNCE;
                    d.pos[axis] = d.cell[axis] as f32;
                    break;
                }
                let at = d.cell;
                for &(p, _) in &d.stamp {
                    sim.world.set(CellPos::new(at.x + p.x, at.y + p.y), Cell::AIR);
                }
                for &(p, c) in &d.stamp {
                    sim.world.set(CellPos::new(at.x + step.x + p.x, at.y + step.y + p.y), c);
                }
                d.cell += step;
                moved += step;
            }
        }
        if moved == IVec2::ZERO {
            continue;
        }
        // What's on it rides along; what it moved into is shoved.
        let (lo, hi) = (d.cell, d.cell + d.size);
        for (ce, mut k) in &mut creatures {
            let ride = riders.contains(&ce);
            let (min, max) = (k.body.pos - k.body.half, k.body.pos + k.body.half);
            let into = !ride && max.x > lo.x as f32 && min.x < hi.x as f32 && max.y > lo.y as f32 && min.y < hi.y as f32 && {
                let c = d.cell;
                (min.x.floor() as i32..=(max.x - 0.01).floor() as i32).any(|x| (min.y.floor() as i32..=(max.y - 0.01).floor() as i32).any(|y| d.stamp.iter().any(|&(p, _)| c + p == IVec2::new(x, y))))
            };
            if ride || into {
                let by = moved.as_vec2();
                k.body.pos += by;
                k.prev_pos += by;
            }
        }
    }
    was.0 = now;
}
