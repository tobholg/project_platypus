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
use std::collections::BinaryHeap;

use bevy::prelude::*;
use platypus_sim::cell::flags::HELD;
use platypus_sim::{Cell, CellPos, Kind};
use serde::Deserialize;

use crate::creatures::{Health, Kinematics};
use crate::world::SimWorld;

/// A slime's goo, as written: what it's made of, and how many cells of it
/// at full health; its spikes, if it has them.
#[derive(Clone, Debug, Deserialize)]
pub struct GooDef {
    pub material: String,
    pub cells: usize,
    #[serde(default)]
    pub spikes: Option<SpikesDef>,
}

/// A slime's spikes (like iron filings on a speaker over a magnet): near
/// what it's after (`range` cells), on the ground and ready, its face
/// toward it shivers (`tell` s); then a fan of spikes of its goo (`count`
/// of them, in a cone `cone` degrees either side) stabs out at it, each
/// as long as a wave running across the fan has it (out to `reach` cells
/// at its crest, a pulse every `pulse` s), jittering; a tip that reaches
/// it hurts (`damage`, once a pulse). It holds still while they're out
/// (`secs`), then rests (`every`).
#[derive(Clone, Debug, Deserialize)]
pub struct SpikesDef {
    pub range: f32,
    pub tell: f32,
    pub secs: f32,
    pub every: f32,
    pub count: usize,
    pub cone: f32,
    pub reach: f32,
    pub pulse: f32,
    pub damage: f32,
}

/// Where its spikes are at: shivering (since), out (since), and when it
/// may next; which way they point, and when a tip last hurt.
#[derive(Clone, Copy, Debug, Default)]
pub enum Spiking {
    #[default]
    Rest,
    Tell(f32),
    Out(f32),
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
/// Spikes: at most this share of its goo in them; each at least this
/// share of its reach (between the wave's crests); the wave's lag from one
/// spike to the next (of a pulse); how grainy a shivering face is (cost).
const SPIKE_SHARE: f32 = 0.35;
const SPIKE_LEAST: f32 = 0.25;
const WAVE_STEP: f32 = 0.16;
const SHIVER: u64 = 24;
/// Cut off at once, at least this much goo is a slime of its own (if as
/// much is left); flung off at this (cells/s). Split, neither merges
/// again for `MERGE_WAIT` s; merged, never bigger than `MERGE_MOST` whole
/// ones. Its box: from this share of its kind's to this.
const SPLIT_LEAST: usize = 110;
const SPLIT_FLING: f32 = 90.0;
const MERGE_WAIT: f32 = 4.0;
const MERGE_MOST: f32 = 3.0;
const BOX_LEAST: f32 = 0.55;
/// The lowest gap it squeezes through (cells).
const SQUEEZE_LEAST: f32 = 7.0;
const BOX_MOST: f32 = 1.5;
/// Swallowing: a small creature is at most this big (cells across and up,
/// times); digested at this (health a second: a creature, the player);
/// an item dissolved after this (s); the player let out after this (s),
/// or worked loose by `STRUGGLE` presses; swallowed again no sooner than
/// `ENGULF_WAIT` s; a slime grows by this share of a whole one for each
/// small creature it digests.
const SMALL: f32 = 160.0;
const DIGEST: f32 = 9.0;
const DIGEST_PLAYER: f32 = 7.0;
const DISSOLVE: f32 = 12.0;
const HELD_MOST: f32 = 3.0;
const STRUGGLE: f32 = 6.0;
const ENGULF_WAIT: f32 = 5.0;
const GROWS: f32 = 0.06;
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
    /// Its spikes: where they're at, when they may come again, which way
    /// they point, when a tip last hurt; where its tips are now.
    pub spiking: Spiking,
    spikes_ready: f32,
    aim: Vec2,
    stung: f32,
    tips: Vec<(Vec2, Vec2)>,
    /// It won't merge before this (just split, it'd only merge back).
    merge_after: f32,
    /// It may swallow the player again from this; it was in the air last
    /// tick (it swallows what it lands on).
    engulf_ready: f32,
    flew: bool,
    /// What its goo is now, if not its kind's (the glutton's, as it takes
    /// on what it's struck with), and what it was (its cells turned).
    pub material: Option<platypus_sim::MaterialId>,
    was: Option<platypus_sim::MaterialId>,
}

impl Goo {
    /// Its goo turned to `m` (the glutton's).
    pub fn turn(&mut self, m: platypus_sim::MaterialId) {
        if self.material != Some(m) {
            self.was = self.material.or(self.was);
            self.material = Some(m);
        }
    }
}

/// Swallowed by a slime (an item, a small creature, the player): held in
/// its goo where it went in (sinking to its middle), going round with it;
/// a creature digested as it's held, an item dissolved (not ore, gems or
/// gear: they stay in it, and fall out of it as it dies).
#[derive(Component, Clone, Copy, Debug)]
pub struct Swallowed {
    pub by: Entity,
    offset: Vec2,
    since: f32,
    /// The player's struggle: how loose they've worked it, and what they
    /// pressed last tick.
    loose: f32,
    last: (bool, bool, f32),
}

/// A slime with the player in it (their blows from inside hurt it more).
#[derive(Component)]
pub struct Engulfing;

/// How big it is, against a whole one of its kind (its most health, and
/// so its goo, and its box): split off, less; merged, more.
#[derive(Component, Clone, Copy, Debug)]
pub struct Mass(pub f32);

/// Each slime puts its goo back round itself (and loses what the world
/// took of it, lets go what it can't hold, spills it all as it dies).
/// After it moves, before deaths and the cells' tick.
#[allow(clippy::type_complexity)]
#[allow(clippy::type_complexity)]
pub fn hold(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    time: Res<Time>,
    mut felt: MessageReader<crate::combat::Felt>,
    mut q: Query<(Entity, &mut Goo, &mut Kinematics, &mut Health, &super::animation::Animator, Option<&mut Mass>, &crate::creatures::Creature)>,
) {
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    let started = std::time::Instant::now();
    let slimes = q.iter().count();
    // (The blows that cut, this tick: which way, by whom.)
    let cut: std::collections::HashMap<Entity, Vec2> = felt.read().filter(|f| matches!(f.harm, crate::creatures::Harm::Slash | crate::creatures::Harm::Pierce)).map(|f| (f.target, f.dir)).collect();
    for (e, mut goo, mut k, mut health, anim, mass, creature) in &mut q {
        let Some(def) = anim.def.goo.as_ref() else { continue };
        let Some(kind_mat) = sim.world.materials().id(&def.material) else { continue };
        let full = anim.def.health.max(1.0);
        let size = mass.as_ref().map_or(1.0, |m| m.0);
        // (Its box as big as it is; flattened to squeeze through a gap
        // lower than it, here or just ahead the way it's going.)
        let mut half = Vec2::new(anim.def.size.0, anim.def.size.1) * 0.5 * size.sqrt().clamp(BOX_LEAST, BOX_MOST);
        let room = {
            let feet = k.body.pos.y - k.body.half.y;
            let ahead = if k.body.vel.x.abs() > 1.0 { k.body.vel.x.signum() } else { k.loco.facing };
            let grid = crate::creatures::WorldGrid(&sim.world);
            let solid = |p: CellPos| platypus_physics::Grid::occupancy(&grid, p.x, p.y) == platypus_physics::Occupancy::Solid;
            let column = |x: f32| (0..(half.y * 2.0) as i32 + 2).take_while(|&dy| !solid(CellPos::from_world(x, feet + 0.5 + dy as f32))).count() as f32;
            [k.body.pos.x - half.x * 0.6, k.body.pos.x, k.body.pos.x + half.x * 0.6, k.body.pos.x + ahead * (half.x + 2.0)].iter().map(|&x| column(x)).fold(f32::MAX, f32::min)
        };
        if room < half.y * 2.0 && room >= SQUEEZE_LEAST {
            half.y = (room - 1.0) * 0.5;
        }
        if (half - k.body.half).abs().max_element() > 0.25 {
            let drop = k.body.half.y - half.y;
            k.body.half = half;
            k.body.pos.y -= drop;
            k.prev_pos.y -= drop;
        }
        let mat = goo.material.unwrap_or(kind_mat);
        // (Turned, its old goo's its still: turned as it's laid out.)
        let old = goo.was.take().or(Some(kind_mat)).filter(|&o| o != mat);
        // What's still its goo where it left it (the rest the world took).
        let mut kept: Vec<Cell> = Vec::with_capacity(goo.placed.len());
        let mut lost = 0usize;
        for &p in &goo.placed {
            match sim.world.get(p) {
                Some(c) if (c.material == mat || Some(c.material) == old) && c.flags & HELD != 0 => kept.push(c),
                _ => lost += 1,
            }
        }
        if old.is_some() {
            let mut rng = platypus_sim::rng::Rng::seeded(&[sim.world.tick(), 0x7A2]);
            for c in kept.iter_mut().filter(|c| c.material != mat) {
                let flags = c.flags;
                *c = sim.world.materials().spawn(mat, &mut rng);
                c.flags = flags & !platypus_sim::cell::flags::BURNING;
            }
            for c in goo.reserve.iter_mut().filter(|c| c.material != mat) {
                *c = sim.world.materials().spawn(mat, &mut rng);
            }
        }
        kept.append(&mut goo.reserve);
        if lost > 0 && health.hp > 0.0 {
            health.hp -= lost as f32 / def.cells.max(1) as f32 * full;
        }
        // Dead: it all spills, where it is.
        if health.hp <= 0.0 {
            for &p in &goo.placed {
                if let Some(mut c) = sim.world.get(p).filter(|c| c.flags & HELD != 0 && (c.material == mat || Some(c.material) == old)) {
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
        // (As much goo as its health is of a whole one's: a small one,
        // split off, holds little and heals only to its own size.)
        let want = ((health.hp / full) * def.cells as f32).round().max(1.0) as usize;
        if !goo.made {
            goo.made = true;
            let mut rng = platypus_sim::rng::Rng::seeded(&[k.body.pos.x.to_bits() as u64, 0x600]);
            kept = (0..want).map(|_| sim.world.materials().spawn(mat, &mut rng)).collect();
        }
        // Lifted out of where it was...
        for &p in &goo.placed {
            if sim.world.get(p).is_some_and(|c| (c.material == mat || Some(c.material) == old) && c.flags & HELD != 0) {
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
        // Cut through, enough off at once (and enough left): what's cut off
        // is a slime of its own, beside it, away from the blow; and it's
        // that much smaller for good.
        let hold = kept.len().min(want);
        if let Some(&dir) = cut.get(&e)
            && kept.len() - hold >= SPLIT_LEAST
            && hold >= SPLIT_LEAST
        {
            let piece: Vec<Cell> = kept.drain(hold..).map(|mut c| {
                c.flags &= !HELD;
                c
            }).collect();
            let share = piece.len() as f32 / def.cells as f32;
            let left = (size - share).max(share);
            match mass {
                Some(mut m) => m.0 = left,
                None => {
                    commands.entity(e).insert(Mass(left));
                }
            }
            health.max = (full * left).max(1.0);
            health.hp = health.hp.min(health.max);
            goo.merge_after = now + MERGE_WAIT;
            let side = if dir.x.abs() > 0.1 { dir.x.signum() } else { 1.0 };
            let at = k.body.pos + Vec2::new(side * (k.body.half.x + 8.0), 4.0 - k.body.half.y);
            let fling = Vec2::new(side * SPLIT_FLING, SPLIT_FLING * 0.8);
            let wait = now + MERGE_WAIT;
            crate::creatures::def::spawn_creature(&mut commands, &creature.kind, at, move |c| {
                c.insert((Mass(share), Goo { reserve: piece, made: true, merge_after: wait, ..Default::default() }));
                if let Some(mut h) = c.get_mut::<Health>() {
                    h.max = (full * share).max(1.0);
                    h.hp = h.max;
                }
                if let Some(mut k) = c.get_mut::<Kinematics>() {
                    k.body.vel = fling;
                }
            });
        }
        // Hurt: what it can't hold it flings off (it runs and pools where
        // it lands: loose goo), out of its outermost.
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
        // Its spikes out: a fan of them first, of its goo (at most a share
        // of it), then the rest laid out round it as ever.
        let mut spiked: Vec<CellPos> = Vec::new();
        goo.tips.clear();
        if let (Spiking::Out(t), Some(sd)) = (goo.spiking, def.spikes.as_ref()) {
            let budget = (kept.len() as f32 * SPIKE_SHARE) as usize;
            let r = (kept.len() as f32 / std::f32::consts::PI).sqrt();
            let tick = sim.world.tick();
            for i in 0..sd.count {
                let f = if sd.count > 1 { i as f32 / (sd.count - 1) as f32 * 2.0 - 1.0 } else { 0.0 };
                let dir = Vec2::from_angle(f * sd.cone.to_radians()).rotate(goo.aim);
                // (A wave running across the fan, and a jitter of its own.)
                let wave = ((t / sd.pulse - i as f32 * WAVE_STEP) * std::f32::consts::TAU).sin().max(0.0);
                let jit = (platypus_sim::rng::hash(&[tick, i as u64, 0x5B1C]) % 5) as f32 - 2.0;
                let len = (sd.reach * (SPIKE_LEAST + (1.0 - SPIKE_LEAST) * wave) + jit).max(2.0);
                let from = k.body.pos + dir * (r * 0.7);
                let mut tip = from;
                let steps = (r * 0.3 + len) as i32;
                for d in 0..steps {
                    if spiked.len() >= budget {
                        break;
                    }
                    let at = from + dir * d as f32;
                    let p = CellPos::from_world(at.x, at.y);
                    if !sim.world.get(p).is_some_and(|c| c.is_air()) {
                        if sim.world.get(p).is_some_and(|c| c.material == mat) {
                            continue;
                        }
                        break;
                    }
                    if !spiked.contains(&p) {
                        spiked.push(p);
                    }
                    // (Three cells thick at its root, two most of the way,
                    // one at its tip.)
                    let along = d as f32 / steps as f32;
                    let sides: &[f32] = if along < 0.4 { &[1.0, -1.0] } else if along < 0.8 { &[1.0] } else { &[] };
                    for &w in sides {
                        let side = CellPos::from_world(at.x - dir.y * w, at.y + dir.x * w);
                        if !spiked.contains(&side) && sim.world.get(side).is_some_and(|c| c.is_air()) && spiked.len() < budget {
                            spiked.push(side);
                        }
                    }
                    tip = at;
                }
                goo.tips.push((tip, dir));
            }
            spiked.truncate(kept.len());
            for (cell, &p) in kept.iter().zip(&spiked) {
                let mut c = *cell;
                c.flags |= HELD;
                c.life = GRACE;
                c.vy = 0;
                sim.world.set(p, c);
            }
        }
        let shiver = matches!(goo.spiking, Spiking::Tell(_)).then(|| (goo.aim, sim.world.tick()));
        let body = kept.split_off(spiked.len().min(kept.len()));
        let places = lay_out_shivering(&sim, k.body.pos, body.len(), (across, down, up), shiver);
        let mut kept = body;
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
        goo.placed = spiked.into_iter().chain(places).collect();
    }
    // (PLATYPUS_GOOTIME: how long holding every slime's goo takes.)
    if slimes > 0 && std::env::var("PLATYPUS_GOOTIME").is_ok() && sim.world.tick().is_multiple_of(60) {
        info!("goo: {slimes} slimes held in {:.2} ms", started.elapsed().as_secs_f64() * 1000.0);
    }
}

/// The `n` open cells nearest `at` by steps (costs across, down, up):
/// filled out from there through air (and smoke), never through what's
/// solid or wet; in the order they're reached (its middle first).
/// Shivering (`shiver`: toward, the tick), its face that way is grainy,
/// a step's cost there a little off at random, new every tick.
fn lay_out_shivering(sim: &SimWorld, at: Vec2, n: usize, (across, down, up): (u32, u32, u32), shiver: Option<(Vec2, u64)>) -> Vec<CellPos> {
    if n == 0 {
        return Vec::new();
    }
    let open = |p: CellPos| sim.world.get(p).is_some_and(|c| c.is_air() || matches!(sim.world.materials().phys(c.material).kind, Kind::Gas));
    // (Searched through what isn't solid, loose goo too; laid only in the
    // open; never far: it isn't that big.)
    let through = |p: CellPos| sim.world.get(p).is_some_and(|c| c.is_air() || !matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder));
    let far = ((n as f32).sqrt() * 1.6) as i32 + 8;
    let start = CellPos::from_world(at.x, at.y);
    let mut out = Vec::with_capacity(n);
    // (What's been reached: a little grid round its middle.)
    let side = (far * 2 + 1) as usize;
    let mut seen = vec![false; side * side];
    let mut mark = |p: CellPos| {
        let (dx, dy) = (p.x - start.x + far, p.y - start.y + far);
        if dx < 0 || dy < 0 || dx as usize >= side || dy as usize >= side {
            return false;
        }
        let i = dy as usize * side + dx as usize;
        !std::mem::replace(&mut seen[i], true)
    };
    let mut heap = BinaryHeap::with_capacity(n * 2);
    heap.push(Reverse((0u32, start.x, start.y)));
    mark(start);
    while let Some(Reverse((cost, x, y))) = heap.pop() {
        let p = CellPos::new(x, y);
        if !through(p) {
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
            if mark(q) {
                let grain = shiver.map_or(0, |(dir, tick)| {
                    let toward = Vec2::new((q.x - start.x) as f32, (q.y - start.y) as f32).normalize_or_zero().dot(dir);
                    if toward > 0.3 { (platypus_sim::rng::hash(&[q.x as u64, q.y as u64, tick]) % (SHIVER + 1)) as u32 } else { 0 }
                });
                heap.push(Reverse((cost + c + grain, q.x, q.y)));
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

/// A slime's spikes: ready, near what it's after, on the ground: it
/// shivers, then they're out, then it rests; out, a tip on what it's after
/// hurts it (once a pulse). It holds still meanwhile. After the brain,
/// before it moves (and before its goo's laid out).
#[allow(clippy::type_complexity)]
pub fn spike(
    time: Res<Time>,
    mut q: Query<(Entity, &mut Goo, &Kinematics, &super::animation::Animator, &mut crate::creatures::Controls, Option<&crate::creatures::brain::senses::Alert>, Has<Engulfing>)>,
    bodies: Query<&Kinematics>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    let now = time.elapsed_secs();
    for (e, mut goo, k, anim, mut controls, alert, full) in &mut q {
        let Some(sd) = anim.def.goo.as_ref().and_then(|g| g.spikes.as_ref()) else { continue };
        let target = alert.filter(|a| a.engaged()).and_then(|a| a.target).filter(|&t| t != e);
        let tk = target.and_then(|t| bodies.get(t).ok());
        match goo.spiking {
            Spiking::Rest => {
                let Some(tk) = tk else { continue };
                // (Not with you in it already.)
                if !full && now >= goo.spikes_ready && k.loco.grounded() && tk.body.pos.distance(k.body.pos) <= sd.range {
                    goo.spiking = Spiking::Tell(now);
                    goo.aim = (tk.body.pos - k.body.pos).normalize_or(Vec2::X);
                    sounds.write(crate::sound::PlaySound::at("pour", k.body.pos).pitch(0.45).volume(0.8));
                }
            }
            Spiking::Tell(since) => {
                if let Some(tk) = tk {
                    goo.aim = (tk.body.pos - k.body.pos).normalize_or(goo.aim);
                }
                if now - since >= sd.tell {
                    goo.spiking = Spiking::Out(0.0);
                    goo.stung = -1.0;
                    sounds.write(crate::sound::PlaySound::at("spit", k.body.pos).pitch(0.6));
                }
            }
            Spiking::Out(t) => {
                let t = t + time.delta_secs();
                goo.spiking = Spiking::Out(t);
                if t >= sd.secs {
                    goo.spiking = Spiking::Rest;
                    goo.spikes_ready = now + sd.every;
                } else if let (Some(tt), Some(tk)) = (target, tk) {
                    // A tip on it hurts it, once a pulse.
                    let (lo, hi) = (tk.body.pos - tk.body.half - 1.0, tk.body.pos + tk.body.half + 1.0);
                    if t - goo.stung >= sd.pulse
                        && let Some(&(tip, dir)) = goo.tips.iter().find(|(p, _)| p.cmpge(lo).all() && p.cmple(hi).all())
                    {
                        goo.stung = t;
                        hits.write(crate::combat::Hit { target: tt, damage: sd.damage, harm: crate::creatures::Harm::Pierce, knock: dir * 160.0 + Vec2::Y * 40.0, stun: 0.15, at: tip, dir, weight: sd.damage / 12.0, crit: false });
                        sounds.write(crate::sound::PlaySound::at("hit", tip).pitch(1.3).volume(0.7));
                    }
                }
            }
        }
        // (Shivering or spiking, it holds still.)
        if !matches!(goo.spiking, Spiking::Rest) {
            controls.0.move_x = 0.0;
            controls.0.jump = false;
        }
    }
}

/// Slimes of a kind whose goo touches merge: the bigger takes the smaller
/// in (its size, its health; its goo let go, to be drawn in as the bigger
/// heals up to its new size), never past `MERGE_MOST` whole ones.
#[allow(clippy::type_complexity)]
pub fn merge(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Goo, &mut Health, &crate::creatures::Creature, &super::animation::Animator, Option<&mut Mass>), Without<crate::creatures::Dormant>>,
) {
    let now = time.elapsed_secs();
    let all: Vec<(Entity, String, f32, (Vec2, Vec2), bool)> = q
        .iter()
        .filter_map(|(e, g, _, c, a, m)| {
            let ok = now >= g.merge_after && matches!(g.spiking, Spiking::Rest) && a.def.goo.is_some();
            Some((e, c.kind.clone(), m.map_or(1.0, |m| m.0), g.extent?, ok))
        })
        .collect();
    let mut gone: Vec<Entity> = Vec::new();
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            if !a.4 || !b.4 || a.1 != b.1 || gone.contains(&a.0) || gone.contains(&b.0) || a.2 + b.2 > MERGE_MOST {
                continue;
            }
            let ((alo, ahi), (blo, bhi)) = (a.3, b.3);
            if alo.x > bhi.x || blo.x > ahi.x || alo.y > bhi.y || blo.y > ahi.y {
                continue;
            }
            let (big, small) = if a.2 >= b.2 { (a, b) } else { (b, a) };
            let Ok([(_, mut bg, mut bh, _, banim, bm), (_, mut sg, sh, _, _, _)]) = q.get_many_mut([big.0, small.0]) else { continue };
            let Some(mat) = banim.def.goo.as_ref().and_then(|d| sim.world.materials().id(&d.material)) else { continue };
            let full = banim.def.health.max(1.0);
            let size = big.2 + small.2;
            match bm {
                Some(mut m) => m.0 = size,
                None => {
                    commands.entity(big.0).insert(Mass(size));
                }
            }
            bh.max = full * size;
            bh.hp = (bh.hp + sh.hp).min(bh.max);
            bg.merge_after = now + MERGE_WAIT * 0.5;
            // (Its goo let go where it is: drawn in as the bigger heals.)
            for &p in &sg.placed {
                if let Some(mut c) = sim.world.get(p).filter(|c| c.material == mat && c.flags & HELD != 0) {
                    c.flags &= !HELD;
                    sim.world.set(p, c);
                }
            }
            sg.placed.clear();
            commands.entity(small.0).despawn();
            gone.push(small.0);
            break;
        }
    }
}

/// What a slime's goo closes round, it swallows: an item lying in it, a
/// small creature in it, and the player when it lands on them. After its
/// goo's laid out.
#[allow(clippy::type_complexity)]
pub fn engulf(
    mut commands: Commands,
    time: Res<Time>,
    mut slimes: Query<(Entity, &mut Goo, &Kinematics), Without<Swallowed>>,
    items: Query<(Entity, &Kinematics), (With<crate::hands::Dropped>, Without<Swallowed>, Without<Goo>)>,
    small: Query<(Entity, &Kinematics), (With<crate::creatures::brain::critters::Critter>, Without<Swallowed>, Without<Goo>)>,
    players: Query<(Entity, &Kinematics), (With<crate::creatures::player::LocalPlayer>, Without<Swallowed>, Without<Goo>)>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    let now = time.elapsed_secs();
    for (s, mut goo, k) in &mut slimes {
        let Some((lo, hi)) = goo.extent else { continue };
        let inside = |p: Vec2| p.cmpge(lo + 2.0).all() && p.cmple(hi - 2.0).all();
        let swallow = |commands: &mut Commands, e: Entity, at: Vec2| {
            let offset = (at - k.body.pos).clamp_length_max(k.body.half.min_element());
            commands.entity(e).insert(Swallowed { by: s, offset, since: now, loose: 0.0, last: (false, false, 0.0) });
        };
        for (e, ik) in &items {
            if inside(ik.body.pos) {
                swallow(&mut commands, e, ik.body.pos);
            }
        }
        for (e, ck) in &small {
            let area = ck.body.half.x * ck.body.half.y * 4.0;
            if area <= SMALL && inside(ck.body.pos) {
                swallow(&mut commands, e, ck.body.pos);
                sounds.write(crate::sound::PlaySound::at("pour", ck.body.pos).pitch(0.6));
            }
        }
        // (The player: when it lands on them.)
        let landed = goo.flew && k.loco.grounded();
        goo.flew = !k.loco.grounded();
        if landed && now >= goo.engulf_ready && matches!(goo.spiking, Spiking::Rest) {
            for (e, pk) in &players {
                if inside(pk.body.pos) {
                    swallow(&mut commands, e, pk.body.pos);
                    commands.entity(s).insert(Engulfing);
                    goo.engulf_ready = now + ENGULF_WAIT;
                    sounds.write(crate::sound::PlaySound::at("pour", pk.body.pos).pitch(0.5).volume(1.0));
                }
            }
        }
    }
}

/// What's swallowed goes round in its slime, sinking to its middle: a
/// creature held helpless and digested (a small one digested, the slime's
/// the bigger for it), an item dissolved (not ore, gems or gear); the
/// player struggles (`STRUGGLE` presses) and is let out (or after
/// `HELD_MOST` s, spat out). Its slime gone, it's let go. After creatures
/// move.
#[allow(clippy::type_complexity)]
pub fn digest(
    mut commands: Commands,
    time: Res<Time>,
    items: Option<Res<crate::hands::items::Items>>,
    mut held: Query<(Entity, &mut Swallowed, &mut Kinematics, Option<&mut Health>, Option<&crate::hands::Dropped>, Option<&crate::creatures::Controls>, Has<crate::creatures::player::LocalPlayer>), Without<Goo>>,
    mut slimes: Query<(&Kinematics, &mut Health, Option<&mut Mass>, &super::animation::Animator), With<Goo>>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    for (e, mut sw, mut k, health, dropped, controls, player) in &mut held {
        let Ok((sk, mut sh, mass, anim)) = slimes.get_mut(sw.by) else {
            // (Its slime's gone: let go, out it falls.)
            commands.entity(e).remove::<Swallowed>();
            continue;
        };
        let slime_dead = sh.hp <= 0.0;
        // Sinking in, going round with it.
        sw.offset *= (1.0 - dt * 1.5).max(0.0);
        let k = &mut *k;
        k.body.pos = sk.body.pos + sw.offset;
        k.prev_pos = k.body.pos;
        k.body.vel = sk.body.vel;
        let side = if sw.offset.x >= 0.0 { 120.0 } else { -120.0 };
        let release = |commands: &mut Commands, k: &mut Kinematics, up: f32| {
            commands.entity(e).remove::<Swallowed>();
            k.body.vel = Vec2::new(side, up);
        };
        if slime_dead {
            release(&mut commands, k, 160.0);
            if player {
                commands.entity(sw.by).remove::<Engulfing>();
            }
            continue;
        }
        if let Some(d) = dropped {
            // An item: dissolved, unless it's what goo can't eat.
            let keeps = items.as_ref().is_some_and(|i| {
                let def = i.def(d.stack.item);
                matches!(def.loot, platypus_sim::Loot::Ore | platypus_sim::Loot::Gem) || def.gear.is_some()
            });
            if !keeps && now - sw.since >= DISSOLVE {
                commands.entity(e).despawn();
            }
            continue;
        }
        let Some(mut h) = health else { continue };
        // Held helpless, digested.
        k.loco.knock(&mut k.body, Vec2::ZERO, 2.0 * dt);
        k.body.pos = sk.body.pos + sw.offset;
        h.harm((if player { DIGEST_PLAYER } else { DIGEST }) * dt, crate::creatures::Harm::Acid);
        if !player {
            if h.hp <= 0.0 {
                // (A small creature digested: the slime's the bigger for it.)
                let full = anim.def.health.max(1.0);
                let size = mass.as_ref().map_or(1.0, |m| m.0) + GROWS;
                if size <= MERGE_MOST {
                    match mass {
                        Some(mut m) => m.0 = size,
                        None => {
                            commands.entity(sw.by).insert(Mass(size));
                        }
                    }
                    sh.max = full * size;
                    sh.hp = (sh.hp + full * GROWS).min(sh.max);
                }
            }
            continue;
        }
        // The player struggles: each press of jump or dash, each turn of
        // the keys the other way, works it looser; or it spits them out.
        if let Some(c) = controls {
            let x = if c.0.move_x.abs() > 0.3 { c.0.move_x.signum() } else { 0.0 };
            let (jump, dash, was_x) = sw.last;
            let mut pressed = (c.0.jump && !jump) as u8 + (c.0.dash && !dash) as u8;
            if x != 0.0 && x != was_x {
                pressed += 1;
            }
            sw.last = (c.0.jump, c.0.dash, if x != 0.0 { x } else { was_x });
            sw.loose += pressed as f32 / STRUGGLE;
        }
        if sw.loose >= 1.0 || now - sw.since >= HELD_MOST {
            let hp = h.hp;
            release(&mut commands, k, 220.0);
            commands.entity(e).insert(crate::combat::Invulnerable { left: 0.6, hp });
            commands.entity(sw.by).remove::<Engulfing>();
            sounds.write(crate::sound::PlaySound::at("spit", k.body.pos).pitch(0.7));
        }
    }
}

/// What the glutton's taken on (fire, frost, lightning), and how much of
/// it's left (draining away).
#[derive(Component, Default)]
pub struct Glutton {
    pub took: Option<crate::creatures::Harm>,
    charge: f32,
}

/// What the glutton takes on: a strike of it at least this much (not the
/// trickle of flames or cold it's in).
const STRIKE: f32 = 2.0;
/// What the glutton holds drains away in this long (s); fire on frost
/// (or frost on fire) goes off this hard.
const DRAIN: f32 = 10.0;
const UNDONE: (f32, f32) = (34.0, 45.0);

/// The glutton takes on what it's struck with: fire, frost or lightning,
/// its goo turned (hot, cold, charged), its touch with it, a light of it;
/// struck with what undoes what it holds, the two go off together; what it
/// took on drains away.
#[allow(clippy::type_complexity)]
pub fn glut(
    mut commands: Commands,
    time: Res<Time>,
    mut sim: ResMut<SimWorld>,
    mut q: Query<(Entity, &mut Glutton, &mut Goo, &Kinematics, &super::animation::Animator, Option<&mut crate::combat::Touch>, &Health)>,
    mut booms: MessageWriter<crate::fx::Explosion>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    use crate::creatures::Harm;
    let dt = time.delta_secs();
    for (e, mut g, mut goo, k, anim, touch, health) in &mut q {
        let before = g.took;
        // (Whatever way it came: a blow, a spell's arc, lightning: what
        // it's taken this tick.)
        // (Not what it holds already: its own hot or cold goo would keep
        // it topped up, and it'd never drain.)
        // (A strike, not the trickle of the flames it stands in.)
        let struck: Vec<Harm> = [Harm::Fire, Harm::Frost, Harm::Storm].into_iter().filter(|h| health.took[*h as usize].0 >= STRIKE && g.took != Some(*h)).collect();
        for harm in struck {
            let undoes = matches!((g.took, harm), (Some(Harm::Fire), Harm::Frost) | (Some(Harm::Frost), Harm::Fire));
            if undoes {
                // (Fire and frost together: they go off.)
                g.took = None;
                g.charge = 0.0;
                booms.write(crate::fx::Explosion { at: k.body.pos, radius: UNDONE.0, power: UNDONE.1 });
                sim.queue(platypus_sim::WorldEdit::Explode { center: CellPos::from_world(k.body.pos.x, k.body.pos.y), radius: 6, power: 30 });
                sounds.write(crate::sound::PlaySound::at("boom", k.body.pos).volume(0.8));
            } else {
                g.took = Some(harm);
                g.charge = 1.0;
            }
        }
        if g.took.is_some() {
            g.charge -= dt / DRAIN;
            if g.charge <= 0.0 {
                g.took = None;
            }
        }
        if g.took == before {
            continue;
        }
        // What it holds now: its goo, its touch, its light.
        let (goo_name, harm, damage, light) = match g.took {
            Some(Harm::Fire) => ("ember_goo", Harm::Fire, 11.0, Some([1.4, 0.7, 0.25])),
            Some(Harm::Frost) => ("frost_goo", Harm::Frost, 9.0, Some([0.6, 0.9, 1.3])),
            Some(Harm::Storm) => ("spark_goo", Harm::Storm, 12.0, Some([0.7, 1.1, 1.5])),
            _ => (anim.def.goo.as_ref().map_or("prism_goo", |d| d.material.as_str()), Harm::Blunt, 7.0, None),
        };
        if let Some(m) = sim.world.materials().id(goo_name) {
            goo.turn(m);
        }
        if let Some(mut t) = touch {
            t.harm = harm;
            t.damage = damage;
        }
        match light {
            Some(color) => {
                commands.entity(e).insert(crate::light::LightSource { color, flicker: 0.15 });
            }
            None => {
                commands.entity(e).remove::<crate::light::LightSource>();
            }
        }
        sounds.write(crate::sound::PlaySound::at("pour", k.body.pos).pitch(1.4));
    }
}

/// What bounces (a blue slime) springs back up as it lands, at a share of
/// how hard it came down.
pub fn bounce(mut landed: MessageReader<crate::creatures::Landed>, mut q: Query<(&mut Kinematics, &super::animation::Animator)>) {
    for l in landed.read() {
        let Ok((mut k, anim)) = q.get_mut(l.entity) else { continue };
        let b = anim.def.bounce;
        if b > 0.0 && l.slam > BOUNCE_LEAST {
            k.body.vel.y = l.slam * b;
        }
    }
}

/// Coming down slower than this (cells/s), it doesn't bounce.
const BOUNCE_LEAST: f32 = 90.0;
