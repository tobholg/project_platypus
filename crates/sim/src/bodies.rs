//! Rigid pixel bodies (SPEC §3.11). A big piece that breaks off whole (a
//! felled tree, a severed branch) leaves the grid, flies as one rotating
//! object, and becomes cells again when it comes to rest.
//!
//! The body keeps its cells in a local grid and is drawn and queried by
//! mapping world cells back into it, so it stays pixel-crisp at any angle.
//! Orientation is a unit complex number turned by a short Taylor series
//! rather than `sin`/`cos`, so stepping is the same on every machine (co-op).
//!
//! Contacts are the body's boundary cells that end up inside solid world
//! cells, resolved with sequential impulses (normal + friction). Leaves
//! (plants) don't collide: when the crown hits the ground they are shed as a
//! flurry.

use crate::cell::Cell;
use crate::coords::CellPos;
use crate::material::{Kind, MaterialTable};

/// Detached pieces at least this big become bodies; smaller ones crumble.
pub const BODY_MIN_CELLS: usize = 64;
/// Cells per tick²; the same as particles and creatures.
const GRAVITY: f32 = crate::particles::GRAVITY;
/// Fastest a boundary cell may move per substep (cells), so it can't skip
/// through a thin floor.
const MAX_SUBSTEP_TRAVEL: f32 = 0.6;
const MAX_SUBSTEPS: u32 = 24;
const MAX_SPEED: f32 = 12.0;
const MAX_SPIN: f32 = 0.25;
const FRICTION: f32 = 0.7;
const SOLVER_ITERATIONS: usize = 6;
/// Share of the deepest penetration corrected per substep. (A fixed push
/// lifted a resting log clear of the ground every few ticks, so it bounced
/// forever and never settled.)
const PUSH_OUT: f32 = 0.8;
/// Slower than this (cells/tick at the fastest point) counts as still.
const REST_SPEED: f32 = 0.04;
const REST_TICKS: u32 = 15;
/// Gives up and settles wherever it is (wedged, or balanced on a point).
const MAX_AGE: u32 = 600;
/// Leaves weigh this much relative to wood.
const LEAF_WEIGHT: f32 = 0.15;

/// What a body bumps into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    /// Solid playfield: ground, rock, logs.
    Front,
    /// Solid background: a stump, another tree's trunk.
    Back,
}

pub struct Body {
    pub id: u32,
    w: i32,
    h: i32,
    /// `w * h`, row-major from the bottom; `Cell::AIR` is empty.
    cells: Vec<Cell>,
    /// Centre of mass in local grid coordinates.
    com: [f32; 2],
    /// World position of the centre of mass (cells).
    pub pos: [f32; 2],
    /// Orientation as (cos, sin).
    pub rot: [f32; 2],
    /// Cells per tick.
    pub vel: [f32; 2],
    /// Radians per tick, counter-clockwise.
    pub omega: f32,
    inv_mass: f32,
    inv_inertia: f32,
    /// Colliding boundary cells (centres), relative to the centre of mass.
    hull: Vec<[f32; 2]>,
    /// Outer leaf cells, relative to the centre of mass: touching ground sheds.
    foliage: Vec<[f32; 2]>,
    /// Farthest cell from the centre of mass.
    pub radius: f32,
    pub age: u32,
    still: u32,
    /// Ticks since it last touched anything.
    airborne: u32,
    /// Last contact was with the playfield: it settles there, as a log.
    pub(crate) rests_on_front: bool,
    /// Where it broke off (centre, half-size): the only background it collides with.
    pub(crate) hinge: Option<(CellPos, i32)>,
}

/// What happened to a body this tick.
#[derive(Default)]
pub(crate) struct BodyEvents {
    /// Its crown touched the ground: shed the leaves.
    pub shed: bool,
    /// Came to rest (or timed out): turn it back into cells.
    pub settled: bool,
}

#[inline]
fn cross(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}

#[inline]
fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

#[inline]
fn len(a: [f32; 2]) -> f32 {
    dot(a, a).sqrt()
}

/// Turn a (cos, sin) orientation by a small angle, without libm.
fn turn(rot: [f32; 2], a: f32) -> [f32; 2] {
    let a2 = a * a;
    let c = 1.0 - a2 * 0.5 + a2 * a2 / 24.0;
    let s = a * (1.0 - a2 / 6.0 + a2 * a2 / 120.0);
    let r = [rot[0] * c - rot[1] * s, rot[0] * s + rot[1] * c];
    let n = len(r);
    [r[0] / n, r[1] / n]
}

#[inline]
fn rotate(rot: [f32; 2], v: [f32; 2]) -> [f32; 2] {
    [rot[0] * v[0] - rot[1] * v[1], rot[1] * v[0] + rot[0] * v[1]]
}

#[inline]
fn floor_cell(p: [f32; 2]) -> CellPos {
    CellPos::new(p[0].floor() as i32, p[1].floor() as i32)
}

impl Body {
    /// Lift `cells` (world positions) out of the world into a body at rest.
    pub(crate) fn new(id: u32, cells: &[(CellPos, Cell)], mats: &MaterialTable) -> Body {
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for (p, _) in cells {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x);
            y1 = y1.max(p.y);
        }
        let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
        let mut grid = vec![Cell::AIR; (w * h) as usize];
        for &(p, c) in cells {
            grid[((p.y - y0) * w + (p.x - x0)) as usize] = c;
        }
        let mut body = Body {
            id,
            w,
            h,
            cells: grid,
            com: [0.0; 2],
            pos: [0.0; 2],
            rot: [1.0, 0.0],
            vel: [0.0; 2],
            omega: 0.0,
            inv_mass: 0.0,
            inv_inertia: 0.0,
            hull: Vec::new(),
            foliage: Vec::new(),
            radius: 0.0,
            age: 0,
            still: 0,
            airborne: 0,
            rests_on_front: false,
            hinge: None,
        };
        body.recompute(mats);
        // Unrotated: the local grid's origin sits at (x0, y0).
        body.pos = [x0 as f32 + body.com[0], y0 as f32 + body.com[1]];
        body
    }

    #[inline]
    fn local(&self, lx: i32, ly: i32) -> Option<Cell> {
        (lx >= 0 && ly >= 0 && lx < self.w && ly < self.h)
            .then(|| self.cells[(ly * self.w + lx) as usize])
            .filter(|c| !c.is_air())
    }

    fn collides(mats: &MaterialTable, c: Cell) -> bool {
        !c.is_air() && mats.phys(c.material).kind != Kind::Plant
    }

    /// Mass, centre of mass, inertia and the contact samples, from the cells.
    /// Keeps the body where it is in the world.
    fn recompute(&mut self, mats: &MaterialTable) {
        let old_com = self.com;
        let (mut m, mut mx, mut my) = (0.0f32, 0.0f32, 0.0f32);
        let weight = |c: Cell| if Self::collides(mats, c) { 1.0 } else { LEAF_WEIGHT };
        for ly in 0..self.h {
            for lx in 0..self.w {
                if let Some(c) = self.local(lx, ly) {
                    let k = weight(c);
                    m += k;
                    mx += k * (lx as f32 + 0.5);
                    my += k * (ly as f32 + 0.5);
                }
            }
        }
        let m = m.max(1e-3);
        self.com = [mx / m, my / m];
        // Where the new centre of mass is in the world, before `pos` moves to it.
        if self.inv_mass > 0.0 {
            let shift = rotate(self.rot, [self.com[0] - old_com[0], self.com[1] - old_com[1]]);
            self.pos = [self.pos[0] + shift[0], self.pos[1] + shift[1]];
        }
        let mut inertia = 0.0f32;
        self.hull.clear();
        self.foliage.clear();
        self.radius = 1.0;
        for ly in 0..self.h {
            for lx in 0..self.w {
                let Some(c) = self.local(lx, ly) else { continue };
                let r = [lx as f32 + 0.5 - self.com[0], ly as f32 + 0.5 - self.com[1]];
                inertia += weight(c) * (dot(r, r) + 1.0 / 6.0);
                self.radius = self.radius.max(len(r) + 0.71);
                let solid = Self::collides(mats, c);
                let edge = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| match self.local(lx + dx, ly + dy) {
                    None => true,
                    Some(n) => solid && !Self::collides(mats, n),
                });
                if edge {
                    if solid { self.hull.push(r) } else { self.foliage.push(r) }
                }
            }
        }
        self.inv_mass = 1.0 / m;
        self.inv_inertia = 1.0 / inertia.max(1e-3);
    }

    /// World point of a point given relative to the centre of mass.
    #[inline]
    fn world_of(&self, r: [f32; 2]) -> [f32; 2] {
        let v = rotate(self.rot, r);
        [self.pos[0] + v[0], self.pos[1] + v[1]]
    }

    /// The body's cell covering world point `p`, if any.
    pub fn sample(&self, p: [f32; 2]) -> Option<Cell> {
        let d = [p[0] - self.pos[0], p[1] - self.pos[1]];
        // Inverse rotation: (cos, -sin).
        let l = [self.rot[0] * d[0] + self.rot[1] * d[1], -self.rot[1] * d[0] + self.rot[0] * d[1]];
        let (lx, ly) = ((l[0] + self.com[0]).floor() as i32, (l[1] + self.com[1]).floor() as i32);
        self.local(lx, ly)
    }

    /// World cells the body could cover, as (min, max) inclusive.
    pub fn bounds(&self) -> (CellPos, CellPos) {
        let r = self.radius.ceil() as i32 + 1;
        let c = floor_cell(self.pos);
        (c.offset(-r, -r), c.offset(r, r))
    }

    /// The body as world cells, as it would be written back into the grid.
    pub fn world_cells(&self) -> impl Iterator<Item = (CellPos, Cell)> + '_ {
        let (lo, hi) = self.bounds();
        (lo.y..=hi.y).flat_map(move |y| (lo.x..=hi.x).map(move |x| CellPos::new(x, y))).filter_map(move |p| {
            self.sample([p.x as f32 + 0.5, p.y as f32 + 0.5]).map(|c| (p, c))
        })
    }

    /// Velocity (cells/tick) of the body at world point `p`.
    pub fn velocity_at(&self, p: [f32; 2]) -> [f32; 2] {
        let r = [p[0] - self.pos[0], p[1] - self.pos[1]];
        [self.vel[0] - self.omega * r[1], self.vel[1] + self.omega * r[0]]
    }

    /// Fastest any part of it moves (cells/tick).
    pub fn top_speed(&self) -> f32 {
        len(self.vel) + self.omega.abs() * self.radius
    }

    /// Strip the leaves. Returns them (world position, cell, velocity).
    pub(crate) fn shed(&mut self, mats: &MaterialTable) -> Vec<([f32; 2], Cell, [f32; 2])> {
        let mut out = Vec::new();
        for ly in 0..self.h {
            for lx in 0..self.w {
                let i = (ly * self.w + lx) as usize;
                let c = self.cells[i];
                if c.is_air() || Self::collides(mats, c) {
                    continue;
                }
                let r = [lx as f32 + 0.5 - self.com[0], ly as f32 + 0.5 - self.com[1]];
                let p = self.world_of(r);
                out.push((p, c, self.velocity_at(p)));
                self.cells[i] = Cell::AIR;
            }
        }
        self.recompute(mats);
        out
    }

    pub fn is_empty(&self) -> bool {
        self.hull.is_empty()
    }

    /// One tick. `solid` says what a world cell is to the body.
    pub(crate) fn step(&mut self, solid: &impl Fn(CellPos) -> Option<Surface>) -> BodyEvents {
        let mut ev = BodyEvents::default();
        self.age += 1;
        let n = ((self.top_speed() + GRAVITY) / MAX_SUBSTEP_TRAVEL).ceil().clamp(1.0, MAX_SUBSTEPS as f32) as u32;
        let h = 1.0 / n as f32;
        let mut touched = false;
        let mut contacts: Vec<([f32; 2], [f32; 2])> = Vec::new();
        for _ in 0..n {
            self.vel[1] -= GRAVITY * h;
            self.pos = [self.pos[0] + self.vel[0] * h, self.pos[1] + self.vel[1] * h];
            self.rot = turn(self.rot, self.omega * h);

            contacts.clear();
            let mut deepest = 0.0f32;
            let mut on_front = false;
            for &r in &self.hull {
                let p = self.world_of(r);
                let cell = floor_cell(p);
                if let Some(s) = solid(cell) {
                    let arm = [p[0] - self.pos[0], p[1] - self.pos[1]];
                    let n = surface_normal(solid, cell, self.velocity_at(p));
                    // How far inside the cell, measured from its open side.
                    let from_center = [p[0] - (cell.x as f32 + 0.5), p[1] - (cell.y as f32 + 0.5)];
                    deepest = deepest.max((0.5 - dot(from_center, n)).clamp(0.0, 1.0));
                    contacts.push((arm, n));
                    on_front |= s == Surface::Front;
                }
            }
            if !ev.shed && !self.foliage.is_empty() {
                ev.shed = self.foliage.iter().any(|&r| solid(floor_cell(self.world_of(r))) == Some(Surface::Front));
            }
            if contacts.is_empty() {
                continue;
            }
            touched = true;
            self.rests_on_front = on_front;
            self.solve(&contacts);
            // Out of the ground, along the average normal.
            let mut push = [0.0f32; 2];
            for (_, n) in &contacts {
                push = [push[0] + n[0], push[1] + n[1]];
            }
            let l = len(push);
            if l > 1e-4 {
                let d = deepest * PUSH_OUT;
                self.pos = [self.pos[0] + push[0] / l * d, self.pos[1] + push[1] / l * d];
            }
        }
        // Clamp so a bad contact can never fling it across the world.
        let s = len(self.vel);
        if s > MAX_SPEED {
            self.vel = [self.vel[0] / s * MAX_SPEED, self.vel[1] / s * MAX_SPEED];
        }
        self.omega = self.omega.clamp(-MAX_SPIN, MAX_SPIN);

        self.airborne = if touched { 0 } else { self.airborne + 1 };
        self.still = if self.airborne < 3 && self.top_speed() < REST_SPEED { self.still + 1 } else { 0 };
        ev.settled = self.still >= REST_TICKS || self.age >= MAX_AGE;
        ev
    }

    /// Sequential impulses: stop motion into the ground, with friction.
    fn solve(&mut self, contacts: &[([f32; 2], [f32; 2])]) {
        let mut jn = vec![0.0f32; contacts.len()];
        let mut jt = vec![0.0f32; contacts.len()];
        for _ in 0..SOLVER_ITERATIONS {
            for (i, &(r, n)) in contacts.iter().enumerate() {
                let v = [self.vel[0] - self.omega * r[1], self.vel[1] + self.omega * r[0]];
                let rn = cross(r, n);
                let k = self.inv_mass + rn * rn * self.inv_inertia;
                let new = (jn[i] - dot(v, n) / k).max(0.0);
                let d = new - jn[i];
                jn[i] = new;
                self.apply(r, [n[0] * d, n[1] * d]);

                let t = [-n[1], n[0]];
                let v = [self.vel[0] - self.omega * r[1], self.vel[1] + self.omega * r[0]];
                let rt = cross(r, t);
                let k = self.inv_mass + rt * rt * self.inv_inertia;
                let limit = FRICTION * jn[i];
                let new = (jt[i] - dot(v, t) / k).clamp(-limit, limit);
                let d = new - jt[i];
                jt[i] = new;
                self.apply(r, [t[0] * d, t[1] * d]);
            }
        }
    }

    #[inline]
    fn apply(&mut self, r: [f32; 2], j: [f32; 2]) {
        self.vel = [self.vel[0] + j[0] * self.inv_mass, self.vel[1] + j[1] * self.inv_mass];
        self.omega += cross(r, j) * self.inv_inertia;
    }
}

/// Outward normal of the solid around `cell`: towards the open cells nearby.
/// Buried deep (no open cell in reach), back the way it came.
fn surface_normal(solid: &impl Fn(CellPos) -> Option<Surface>, cell: CellPos, vel: [f32; 2]) -> [f32; 2] {
    let mut n = [0.0f32; 2];
    for dy in -2..=2 {
        for dx in -2..=2 {
            if (dx, dy) != (0, 0) && solid(cell.offset(dx, dy)).is_none() {
                n = [n[0] + dx as f32, n[1] + dy as f32];
            }
        }
    }
    let l = len(n);
    if l > 1e-4 {
        return [n[0] / l, n[1] / l];
    }
    let s = len(vel);
    if s > 1e-4 { [-vel[0] / s, -vel[1] / s] } else { [0.0, 1.0] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_matches_trig_and_stays_unit() {
        let mut rot = [1.0f32, 0.0];
        for _ in 0..1000 {
            rot = turn(rot, 0.01);
        }
        let (c, s) = (10.0f32.cos(), 10.0f32.sin());
        assert!((rot[0] - c).abs() < 1e-3 && (rot[1] - s).abs() < 1e-3, "{rot:?} vs ({c}, {s})");
        assert!((len(rot) - 1.0).abs() < 1e-5);
    }
}
