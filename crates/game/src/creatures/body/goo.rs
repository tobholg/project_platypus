//! A slime's body (`goo` in its creature file): real goo, cells of the
//! world, held by its core (the creature: a nucleus with eyes, and its box
//! for knocks and blows).
//!
//! Each tick the core lays its goo out round itself: as many cells as its
//! health is of its most (`GooDef::cells` at full), filled out from its
//! middle through the open cells near it (a little readier downward, so it
//! sits on the ground in a dome; squashed flat as it lands, drawn out as
//! it flies), the goo it had carried into the new places (alight, it
//! stays alight). The world has its way with the goo between: fire burns
//! it, acid eats it, a pick digs it; what's gone of it it's lost (its
//! health with it). Hurt, it lets go of what it can no longer hold: that
//! runs and pools, loose goo (to collect); dead, it all spills.
//!
//! Held goo stays where it's put (the sim's `HELD` flag: it doesn't flow,
//! nothing pushes it aside), bodies pass through it, and it's let go a few
//! ticks after it's no longer put back (`GRACE`: a slime gone, its goo
//! runs).

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};

use bevy::prelude::*;
use platypus_sim::cell::flags::HELD;
use platypus_sim::{Cell, CellPos, Kind};
use serde::Deserialize;

use crate::creatures::{Health, Kinematics};
use crate::world::SimWorld;

/// A slime's goo, as written: what it's made of, and how many cells of it
/// at full health.
#[derive(Clone, Debug, Deserialize)]
pub struct GooDef {
    pub material: String,
    pub cells: usize,
}

/// Ticks held goo stays held without being put back.
const GRACE: u8 = 6;
/// Steps out from its middle, a cell (costs): across, down, up. Squashed
/// as it lands (flatter for a moment), drawn out as it flies.
const ACROSS: u32 = 10;
const DOWN: u32 = 9;
const UP: u32 = 13;
/// Healing: loose goo drawn back in at most this much a tick (from this
/// far past its size), new goo grown this much a tick.
const DRAW_IN: usize = 6;
const REACH: i32 = 18;
const GROW: usize = 2;
/// Goo flung off (cells a tick).
const FLUNG: f32 = 1.4;
/// How long a landing's squash takes to spring back (s), and how hard a
/// landing (cells/s down) squashes it all the way.
const SPRING: f32 = 0.35;
const HARD_LANDING: f32 = 160.0;

/// What a slime holds: where its goo is, and how squashed it is.
#[derive(Component, Default)]
pub struct Goo {
    placed: Vec<CellPos>,
    /// What it holds that there wasn't room to lay out (in it, for now).
    reserve: Vec<Cell>,
    made: bool,
    squash: f32,
    falling: f32,
    /// Where its goo reaches (lowest corner, highest), for what it touches.
    extent: Option<(Vec2, Vec2)>,
}

/// Each slime puts its goo back round itself (and loses what the world
/// took of it, lets go what it can't hold, spills it all as it dies).
/// After it moves, before deaths and the cells' tick.
#[allow(clippy::type_complexity)]
pub fn hold(mut sim: ResMut<SimWorld>, time: Res<Time>, mut q: Query<(&mut Goo, &Kinematics, &mut Health, &super::animation::Animator)>) {
    let dt = time.delta_secs();
    for (mut goo, k, mut health, anim) in &mut q {
        let Some(def) = anim.def.goo.as_ref() else { continue };
        let Some(mat) = sim.world.materials().id(&def.material) else { continue };
        // What's still its goo where it left it (the rest the world took).
        let mut kept: Vec<Cell> = Vec::with_capacity(goo.placed.len());
        let mut lost = 0usize;
        for &p in &goo.placed {
            match sim.world.get(p) {
                Some(c) if c.material == mat && c.flags & HELD != 0 => kept.push(c),
                _ => lost += 1,
            }
        }
        kept.append(&mut goo.reserve);
        if lost > 0 && health.hp > 0.0 {
            health.hp -= lost as f32 / def.cells.max(1) as f32 * health.max;
        }
        // Dead: it all spills, where it is.
        if health.hp <= 0.0 {
            for &p in &goo.placed {
                if let Some(mut c) = sim.world.get(p).filter(|c| c.material == mat && c.flags & HELD != 0) {
                    c.flags &= !HELD;
                    sim.world.set(p, c);
                }
            }
            goo.placed.clear();
            let at = k.body.pos;
            for c in kept.drain(..) {
                sim.world.spill([at.x, at.y], [0.0, 1.0], c.material, 1, FLUNG, 1.4);
            }
            continue;
        }
        let want = ((health.hp / health.max.max(1.0)) * def.cells as f32).round().max(1.0) as usize;
        if !goo.made {
            goo.made = true;
            let mut rng = platypus_sim::rng::Rng::seeded(&[k.body.pos.x.to_bits() as u64, 0x600]);
            kept = (0..want).map(|_| sim.world.materials().spawn(mat, &mut rng)).collect();
        }
        // Lifted out of where it was...
        for &p in &goo.placed {
            if sim.world.get(p).is_some_and(|c| c.material == mat && c.flags & HELD != 0) {
                sim.world.set(p, Cell::AIR);
            }
        }
        // Healed (it regrows: `regen`), it takes back what it holds less of
        // than it may: loose goo of its kind near it first (its puddles
        // drawn back in), then new.
        if kept.len() < want {
            let need = (want - kept.len()).min(DRAW_IN);
            let taken = draw_in(&mut sim, k.body.pos, mat, need, (def.cells as f32).sqrt() as i32 + REACH);
            kept.extend(taken);
            let grow = (want - kept.len()).min(GROW);
            let mut rng = platypus_sim::rng::Rng::seeded(&[sim.world.tick(), k.body.pos.x.to_bits() as u64, 0x6A0]);
            for _ in 0..grow {
                kept.push(sim.world.materials().spawn(mat, &mut rng));
            }
        }
        // Hurt: what it can't hold it flings off (it runs and pools where
        // it lands: loose goo), out of its outermost.
        let hold = kept.len().min(want);
        for (i, c) in kept.drain(hold..).enumerate() {
            let from = goo.placed.get(goo.placed.len().saturating_sub(1 + i)).map_or(k.body.pos, |p| Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5));
            let out = (from - k.body.pos).normalize_or(Vec2::Y) + Vec2::Y * 0.6;
            let mut c = c;
            c.flags &= !HELD;
            sim.world.spill([from.x, from.y], [out.x, out.y], c.material, 1, FLUNG, 0.5);
        }
        // ...and laid out round it now: squashed as it lands, drawn out as
        // it flies.
        let vy = k.body.vel.y;
        if goo.falling < -40.0 && vy > -5.0 {
            goo.squash = (goo.falling.abs() / HARD_LANDING).min(1.0);
        }
        goo.falling = vy;
        goo.squash = (goo.squash - dt / SPRING).max(0.0);
        let flying = !k.loco.grounded();
        let (down, up) = if flying { (ACROSS, ACROSS) } else { (DOWN, UP + (goo.squash * 20.0) as u32) };
        let across = if flying { ACROSS } else { ACROSS - (goo.squash * 4.0) as u32 };
        let places = lay_out(&sim, k.body.pos, kept.len(), (across, down, up));
        for (cell, &p) in kept.iter().zip(&places) {
            let mut c = *cell;
            c.flags |= HELD;
            c.life = GRACE;
            c.vy = 0;
            sim.world.set(p, c);
        }
        // (No room for the rest just now: it keeps it in it.)
        goo.reserve = kept.split_off(places.len().min(kept.len()));
        goo.extent = places.iter().fold(None, |e: Option<(Vec2, Vec2)>, p| {
            let (a, b) = (Vec2::new(p.x as f32, p.y as f32), Vec2::new(p.x as f32 + 1.0, p.y as f32 + 1.0));
            Some(e.map_or((a, b), |(lo, hi)| (lo.min(a), hi.max(b))))
        });
        goo.placed = places;
    }
}

/// The `n` open cells nearest `at` by steps (costs across, down, up):
/// filled out from there through air (and smoke), never through what's
/// solid or wet; in the order they're reached (its middle first).
fn lay_out(sim: &SimWorld, at: Vec2, n: usize, (across, down, up): (u32, u32, u32)) -> Vec<CellPos> {
    if n == 0 {
        return Vec::new();
    }
    let open = |p: CellPos| sim.world.get(p).is_some_and(|c| c.is_air() || matches!(sim.world.materials().phys(c.material).kind, Kind::Gas));
    // (Searched through what isn't solid, loose goo too; laid only in the
    // open; never far: it isn't that big.)
    let through = |p: CellPos| sim.world.get(p).is_some_and(|c| c.is_air() || !matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder));
    let far = (n as f32).sqrt() as i32 * 3 + 6;
    let start = CellPos::from_world(at.x, at.y);
    let mut out = Vec::with_capacity(n);
    let mut seen: HashSet<CellPos> = HashSet::with_capacity(n * 3);
    let mut heap = BinaryHeap::new();
    heap.push(Reverse((0u32, start.x, start.y)));
    seen.insert(start);
    while let Some(Reverse((cost, x, y))) = heap.pop() {
        let p = CellPos::new(x, y);
        if !through(p) || (p.x - start.x).abs() > far || (p.y - start.y).abs() > far {
            continue;
        }
        if open(p) {
            out.push(p);
            if out.len() >= n {
                break;
            }
        }
        // (Diagonals too, at their length: round, not a diamond.)
        let (dd, du) = ((across + down) * 7 / 10, (across + up) * 7 / 10);
        for (dx, dy, c) in [(1, 0, across), (-1, 0, across), (0, -1, down), (0, 1, up), (1, -1, dd), (-1, -1, dd), (1, 1, du), (-1, 1, du)] {
            let q = CellPos::new(x + dx, y + dy);
            if seen.insert(q) {
                heap.push(Reverse((cost + c, q.x, q.y)));
            }
        }
    }
    out
}

impl Goo {
    /// How many cells it holds now.
    pub fn held(&self) -> usize {
        self.placed.len()
    }

    /// Where its goo reaches (lowest corner, highest), if it holds any.
    pub fn extent(&self) -> Option<(Vec2, Vec2)> {
        self.extent
    }
}

/// Up to `n` cells of loose goo (`mat`, not held) within `reach` of `at`,
/// the nearest first: taken out of the world.
fn draw_in(sim: &mut SimWorld, at: Vec2, mat: platypus_sim::MaterialId, n: usize, reach: i32) -> Vec<Cell> {
    let c = CellPos::from_world(at.x, at.y);
    let mut found: Vec<(i32, CellPos)> = Vec::new();
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let p = CellPos::new(c.x + dx, c.y + dy);
            if sim.world.get(p).is_some_and(|cell| cell.material == mat && cell.flags & HELD == 0) {
                found.push((dx * dx + dy * dy, p));
            }
        }
    }
    found.sort_unstable_by_key(|f| f.0);
    found
        .into_iter()
        .take(n)
        .filter_map(|(_, p)| {
            let cell = sim.world.get(p)?;
            sim.world.set(p, Cell::AIR);
            Some(cell)
        })
        .collect()
}
