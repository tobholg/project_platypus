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
    mut q: Query<(Entity, &mut Goo, &Kinematics, &super::animation::Animator, &mut crate::creatures::Controls, Option<&crate::creatures::brain::senses::Alert>)>,
    bodies: Query<&Kinematics>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    let now = time.elapsed_secs();
    for (e, mut goo, k, anim, mut controls, alert) in &mut q {
        let Some(sd) = anim.def.goo.as_ref().and_then(|g| g.spikes.as_ref()) else { continue };
        let target = alert.filter(|a| a.engaged()).and_then(|a| a.target).filter(|&t| t != e);
        let tk = target.and_then(|t| bodies.get(t).ok());
        match goo.spiking {
            Spiking::Rest => {
                let Some(tk) = tk else { continue };
                if now >= goo.spikes_ready && k.loco.grounded() && tk.body.pos.distance(k.body.pos) <= sd.range {
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
