//! The leviathan: a sea serpent of the underground seas, near a thousand
//! cells of abyssal blue, lit cold blue along its flanks, its frill and its
//! fins. It swims as the rest of the sea's swimmers do (`swim.rs`), stroke
//! by stroke, but slow to turn: it swings round you in great arcs, and
//! every attack has a tell, its glow showing what's coming.
//!
//! - **Circling**: it swims round you, far out, on a wandering orbit.
//! - **Strike**: its glow gathers at its head, it draws back gaping; then a
//!   kick and a burst of strokes along that line, a bite at the end.
//! - **Sweep**: it swims past beside you at a run: its body is the blow.
//! - **Coil**: it circles you close, closer, its body round you.
//! - **Abyssal bolts** (its favourite, out of reach): light runs up its
//!   body from its tail to its head, and it looses a volley of slow orbs
//!   that curve onto you and burst (a spell, `abyssal_bolt`: they hit
//!   whatever they meet): a fan of them, a spiral all round it that curls
//!   in, or a quick stream straight at you.
//! - **Its tail**: a sheaf of charged tendrils (`swim.rs`): touch them and
//!   they shock you.
//! - **Pressure pulse**: crowded, it flashes all over and a ring of force
//!   runs out through the water from its head, knocking everything in the
//!   water away and stunning it (not itself).
//! - **Undertow** (hurt past half): it hangs gaping and the water swirls
//!   into its maw, drawing what's near in; then it strikes.
//!
//! Hurt past half it's in a rage: quicker tells, shorter waits, more bolts.

use bevy::prelude::*;
use serde::Deserialize;

use super::{CustomCreature, CustomSet};
use crate::creatures::body::animation::{Animator, Hug};
use crate::creatures::swim::{Swimmer, anchor};
use crate::creatures::{Controls, Harm, Health, Kinematics, Team};
use crate::world::SimWorld;

#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Leviathan {
    /// Its bite and its body's blows this many times as hard.
    pub bite: f32,
    /// Bolts in a volley: calm, and in a rage.
    pub volley: (u32, u32),
    #[serde(skip)]
    doing: Doing,
    /// Its waits, one a trick (`STRIKE_I`...).
    #[serde(skip)]
    ready: [f32; 6],
    #[serde(skip)]
    orbit: f32,
    #[serde(skip)]
    clock: f32,
    /// The way it strikes; the side it sweeps past on.
    #[serde(skip)]
    aim: Vec2,
    #[serde(skip)]
    side: f32,
    /// What its body's struck lately (each a wait).
    #[serde(skip)]
    struck: Vec<(Entity, f32)>,
    /// Its pulse running out: from where, how far it's got; what it's hit.
    #[serde(skip)]
    pulse: Option<(Vec2, f32)>,
    #[serde(skip)]
    pulsed: Vec<Entity>,
    #[serde(skip)]
    rage: bool,
    /// The volley's pattern: a fan, a spiral, a stream.
    #[serde(skip)]
    pattern: u8,
    #[serde(skip)]
    started: bool,
}

impl Default for Leviathan {
    fn default() -> Self {
        Leviathan {
            bite: 1.0,
            volley: (3, 5),
            doing: Doing::Circle,
            ready: [0.0; 6],
            orbit: 0.0,
            clock: 0.0,
            aim: Vec2::X,
            side: 1.0,
            struck: Vec::new(),
            pulse: None,
            pulsed: Vec::new(),
            rage: false,
            pattern: 0,
            started: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Doing {
    /// Swimming round its prey, far out.
    #[default]
    Circle,
    /// The strike's tell (drawn back, gaping, its glow at its head); the
    /// strike (whether it's bitten).
    Wind(f32),
    Strike(f32, bool),
    /// Past beside them at a run.
    Sweep(f32),
    /// Round them, closer and closer.
    Coil(f32),
    /// Its glow running up it; then the volley (bolts left).
    Gather(f32),
    Volley(f32, u32),
    /// Flashing all over; then the pulse.
    Charge(f32),
    /// Hanging, gaping, drawing them in.
    Undertow(f32),
    /// After a run: gliding, slowing.
    Spent(f32),
}

impl CustomCreature for Leviathan {
    const NAME: &'static str = "leviathan";

    fn build(app: &mut App) {
        app.add_systems(FixedUpdate, think.in_set(CustomSet::Think));
    }
}

impl Leviathan {
    /// What it's doing, in a word (a scenario's log).
    pub fn doing(&self) -> &'static str {
        match self.doing {
            Doing::Circle => "circle",
            Doing::Wind(_) => "wind",
            Doing::Strike(..) => "strike",
            Doing::Sweep(_) => "sweep",
            Doing::Coil(_) => "coil",
            Doing::Gather(_) => "gather",
            Doing::Volley(..) => "volley",
            Doing::Charge(_) => "charge",
            Doing::Undertow(_) => "undertow",
            Doing::Spent(_) => "spent",
        }
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
/// Hunting what's this near (cells, from its head).
const AGGRO: f32 = 1_100.0;
/// Circling: this far out (and this much nearer and further by turns), this
/// fast round (rad/s).
const ORBIT: (f32, f32, f32) = (380.0, 90.0, 0.32);
const STRIKE_I: usize = 0;
const SWEEP_I: usize = 1;
const COIL_I: usize = 2;
const VOLLEY_I: usize = 3;
const PULSE_I: usize = 4;
const UNDERTOW_I: usize = 5;
/// The strike: at what's from this near to this far; its tell (calm, in a
/// rage); its kick and burst; its bite (damage, knock, stun); its wait.
const STRIKE_RANGE: (f32, f32) = (130.0, 560.0);
const WIND: (f32, f32) = (0.85, 0.55);
const STRIKE: (f32, f32) = (600.0, 0.9);
const BITE: (f32, f32, f32) = (40.0, 620.0, 0.45);
const STRIKE_EVERY: f32 = 4.5;
/// The sweep: at what's this near to this far; past them this far to the
/// side, on for this long; its wait.
const SWEEP_RANGE: (f32, f32) = (160.0, 460.0);
const SWEEP: (f32, f32) = (90.0, 1.7);
const SWEEP_EVERY: f32 = 7.0;
/// The coil: at what's this near; from this far round to this near, this
/// long, this fast round (rad/s); its wait.
const COIL_NEAR: f32 = 280.0;
const COIL: (f32, f32, f32, f32) = (250.0, 90.0, 3.6, 2.2);
const COIL_EVERY: f32 = 11.0;
/// Its body's blows (damage, knock, stun), at most once a body this often.
const BODY_HIT: (f32, f32, f32) = (18.0, 440.0, 0.25);
const BODY_EVERY: f32 = 0.6;
/// The volley: at what's this near to this far (picked this share of the
/// times it's ready, before anything else); its glow's run up it; a
/// fan's bolts this often, a spiral's (so many, all round), a stream's
/// (so many); its wait.
const VOLLEY_RANGE: (f32, f32) = (200.0, 1_100.0);
const VOLLEY_FIRST: f32 = 0.55;
const GATHER: f32 = 1.1;
const BOLT_EVERY: f32 = 0.28;
const SPIRAL: (f32, u32) = (0.11, 10);
const STREAM: (f32, u32) = (0.13, 6);
const VOLLEY_EVERY: f32 = 5.0;
/// The pulse: at what's this near its head; its tell; the ring out to this
/// far at this speed; what it does (damage, knock, stun); its wait.
const PULSE_NEAR: f32 = 190.0;
const CHARGE: f32 = 0.65;
const RING: (f32, f32) = (300.0, 650.0);
const PULSE_HIT: (f32, f32, f32) = (12.0, 760.0, 0.7);
const PULSE_EVERY: f32 = 9.0;
/// The undertow (in a rage): at what's from this near to this far; this
/// long, pulling what's within this far at this (cells/s²), a swirl round
/// with it this share; its wait.
const UNDERTOW_RANGE: (f32, f32) = (150.0, 480.0);
const UNDERTOW: (f32, f32, f32, f32) = (2.6, 520.0, 520.0, 0.5);
const UNDERTOW_EVERY: f32 = 13.0;
/// Spent after a run (seconds).
const SPENT: f32 = 0.9;
/// In a rage below this share of its health: waits this share as long.
const RAGE: (f32, f32) = (0.5, 0.6);
/// Where it goes for: their chest.
const CHEST: f32 = 6.0;

fn emitter(colors: Vec<(u8, u8, u8)>, speed: f32, life: (f32, f32), size: f32) -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter { count: 1.0, life, colors, speed, spread: 3.14, gravity: 0.0, drag: 2.5, size, jitter: 0.0, glow: true }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn think(
    sim: Res<SimWorld>,
    book: Res<crate::magic::Spellbook>,
    mut q: Query<(Entity, &mut Leviathan, &mut Kinematics, &mut Controls, &mut Animator, &Health, &Hug, Option<&mut Swimmer>)>,
    mut hunted: Query<(Entity, &mut Kinematics, &Team), Without<Leviathan>>,
    (mut hits, mut casts, mut sounds): (MessageWriter<crate::combat::Hit>, MessageWriter<crate::magic::CastRequest>, MessageWriter<crate::sound::PlaySound>),
    (mut sparks, mut flares, mut trauma): (ResMut<crate::vfx::Sparks>, MessageWriter<crate::fx::Flare>, ResMut<crate::fx::Trauma>),
) {
    let water = sim.materials().id("water");
    let wet = |p: Vec2| sim.world.get(platypus_sim::CellPos::from_world(p.x, p.y)).is_some_and(|c| Some(c.material) == water);
    // (Where it swims for: under the water, not up in the air over it.)
    let under = |p: Vec2| (0..60).map(|i| p - Vec2::Y * (i as f32 * 8.0)).find(|q| wet(*q) && wet(*q + Vec2::Y * 40.0)).unwrap_or(p);
    for (e, mut o, mut k, mut c, mut anim, health, hug, swimmer) in &mut q {
        let Some(mut sw) = swimmer else { continue };
        let pos = k.body.pos;
        if !o.started {
            o.started = true;
            o.ready = [2.0, 4.0, 6.0, 3.0, 2.0, 5.0];
            o.orbit = (e.to_bits() % 628) as f32 / 100.0;
        }
        o.clock += DT;
        o.rage = health.hp < health.max * RAGE.0;
        let quick = if o.rage { RAGE.1 } else { 1.0 };
        for r in o.ready.iter_mut() {
            *r -= DT;
        }
        for s in o.struck.iter_mut() {
            s.1 -= DT;
        }
        o.struck.retain(|s| s.1 > 0.0);
        let maw = anchor(&anim, &k, hug.0, "maw").unwrap_or(pos);
        let target = hunted.iter().filter(|(_, _, t)| t.hunted()).filter(|(_, tk, _)| tk.body.pos.distance(maw) < AGGRO && wet(tk.body.pos)).min_by(|a, b| a.1.body.pos.distance(maw).total_cmp(&b.1.body.pos.distance(maw))).map(|(te, tk, _)| (te, tk.body.pos + Vec2::Y * CHEST));
        let tp = target.map(|(_, p)| p);
        let dist = tp.map_or(f32::MAX, |p| p.distance(maw));
        let doing = o.doing;
        let mut next = doing;
        // Its moves: where it swims and how keenly (its swimmer strokes it
        // there), where it looks.
        let (mut to, mut hurry) = (Vec2::ZERO, 0.0);
        sw.flare.0 *= 0.9;
        match doing {
            Doing::Circle => {
                if let Some(tp) = tp {
                    o.orbit += ORBIT.2 * DT;
                    let r = ORBIT.0 + ORBIT.1 * (o.clock * 0.27).sin();
                    let goal = under(tp + Vec2::new(o.orbit.cos() * r, o.orbit.sin() * r * 0.6));
                    to = goal - maw;
                    hurry = (to.length() / 220.0).clamp(0.35, 0.8);
                    // A trick when one's ready, picked at random of those
                    // that suit how far they are.
                    let roll = (platypus_sim::rng::hash(&[e.to_bits(), (o.clock * 3.0) as u64]) % 100) as f32 / 100.0;
                    let ready = |i: usize| o.ready[i] <= 0.0;
                    let mut can: Vec<usize> = Vec::new();
                    if dist < PULSE_NEAR && ready(PULSE_I) {
                        can.push(PULSE_I);
                    }
                    if o.rage && ready(UNDERTOW_I) && (UNDERTOW_RANGE.0..UNDERTOW_RANGE.1).contains(&dist) {
                        can.push(UNDERTOW_I);
                    }
                    if dist < COIL_NEAR && ready(COIL_I) {
                        can.push(COIL_I);
                    }
                    if ready(STRIKE_I) && (STRIKE_RANGE.0..STRIKE_RANGE.1).contains(&dist) {
                        can.push(STRIKE_I);
                    }
                    if ready(SWEEP_I) && (SWEEP_RANGE.0..SWEEP_RANGE.1).contains(&dist) {
                        can.push(SWEEP_I);
                    }
                    if ready(VOLLEY_I) && (VOLLEY_RANGE.0..VOLLEY_RANGE.1).contains(&dist) {
                        can.push(VOLLEY_I);
                    }
                    // (Its magic first, more often than not, when it can.)
                    let first = can.contains(&VOLLEY_I) && (platypus_sim::rng::hash(&[e.to_bits(), (o.clock * 3.0) as u64, 0xB01]) % 100) as f32 / 100.0 < VOLLEY_FIRST;
                    let pick = if first { Some(&VOLLEY_I) } else { can.get((roll * can.len() as f32) as usize) };
                    if let Some(&pick) = pick {
                        next = match pick {
                            PULSE_I => Doing::Charge(0.0),
                            UNDERTOW_I => Doing::Undertow(0.0),
                            COIL_I => Doing::Coil(0.0),
                            STRIKE_I => Doing::Wind(0.0),
                            SWEEP_I => {
                                o.side = if roll < 0.5 { 1.0 } else { -1.0 };
                                Doing::Sweep(0.0)
                            }
                            _ => {
                                o.pattern = (platypus_sim::rng::hash(&[e.to_bits(), (o.clock * 7.0) as u64, 0xB02]) % 3) as u8;
                                Doing::Gather(0.0)
                            }
                        };
                        let groan = if matches!(next, Doing::Charge(_) | Doing::Undertow(_)) { 0.5 } else { 0.8 };
                        sounds.write(crate::sound::PlaySound::at("leviathan_groan", maw).pitch(groan).volume(1.0));
                    }
                } else {
                    // (Nothing about: a slow wander.)
                    o.orbit += 0.15 * DT;
                    to = Vec2::from_angle(o.orbit);
                    hurry = 0.3;
                }
            }
            Doing::Wind(t) => {
                // Still, gaping, its head turned after them, its glow
                // gathering there (the tell).
                let tell = if o.rage { WIND.1 } else { WIND.0 };
                if let Some(tp) = tp {
                    o.aim = (tp - maw).normalize_or(o.aim);
                }
                sw.flare = ((t / tell).min(1.0), 0.0);
                next = if t + DT >= tell {
                    sw.strike(&mut k, o.aim, STRIKE.0, STRIKE.1);
                    sounds.write(crate::sound::PlaySound::at("snap", maw).pitch(0.4).volume(1.0));
                    Doing::Strike(0.0, false)
                } else {
                    Doing::Wind(t + DT)
                };
            }
            Doing::Strike(t, bit) => {
                // On along its line, bending a little after them.
                to = tp.map_or(o.aim, |tp| (o.aim * 3.0 + (tp - maw).normalize_or(o.aim)).normalize_or(o.aim));
                hurry = 1.0;
                let mut bit = bit;
                if !bit && let Some((who, tp)) = target
                    && tp.distance(maw) < 26.0
                {
                    hits.write(crate::combat::Hit { by: Some(e), target: who, damage: BITE.0 * o.bite, harm: Harm::Pierce, knock: o.aim * BITE.1, stun: BITE.2, at: maw, dir: o.aim, weight: 1.0, crit: false });
                    sounds.write(crate::sound::PlaySound::at("chomp", maw).pitch(0.5).volume(1.2));
                    trauma.0 = (trauma.0 + 0.35).min(1.0);
                    bit = true;
                }
                next = if t + DT >= STRIKE.1 {
                    o.ready[STRIKE_I] = STRIKE_EVERY * quick;
                    Doing::Spent(0.0)
                } else {
                    Doing::Strike(t + DT, bit)
                };
            }
            Doing::Sweep(t) => {
                // Past beside them and on: its body the blow.
                if let Some(tp) = tp {
                    let way = (tp - maw).normalize_or(Vec2::from_angle(sw.heading));
                    to = under(tp + way.perp() * SWEEP.0 * o.side + way * 260.0) - maw;
                }
                hurry = 1.0;
                sw.burst = sw.burst.max(0.2);
                next = if t + DT >= SWEEP.1 {
                    o.ready[SWEEP_I] = SWEEP_EVERY * quick;
                    Doing::Spent(0.0)
                } else {
                    Doing::Sweep(t + DT)
                };
            }
            Doing::Coil(t) => {
                if let Some(tp) = tp {
                    o.orbit += COIL.3 * DT;
                    let r = COIL.0 + (COIL.1 - COIL.0) * (t / COIL.2).min(1.0);
                    to = under(tp + Vec2::from_angle(o.orbit) * r) - maw;
                }
                hurry = 1.0;
                next = if t + DT >= COIL.2 || tp.is_none() {
                    o.ready[COIL_I] = COIL_EVERY * quick;
                    Doing::Spent(0.0)
                } else {
                    Doing::Coil(t + DT)
                };
            }
            Doing::Gather(t) => {
                // Holding off, its light running up it, tail to head.
                if let Some(tp) = tp {
                    let away = maw - tp;
                    to = if away.length() < VOLLEY_RANGE.0 { away } else { Vec2::ZERO };
                    hurry = if to == Vec2::ZERO { 0.0 } else { 0.4 };
                }
                sw.flare = (1.0, 1.0 - (t / GATHER).min(1.0));
                next = if t + DT >= GATHER {
                    let more = if o.rage { 2 } else { 1 };
                    Doing::Volley(0.0, match o.pattern {
                        0 => if o.rage { o.volley.1 } else { o.volley.0 },
                        1 => SPIRAL.1 * more,
                        _ => STREAM.1 + 2 * (more - 1),
                    })
                } else {
                    Doing::Gather(t + DT)
                };
            }
            Doing::Volley(t, left) => {
                sw.flare = (1.0, 0.0);
                next = if left == 0 || tp.is_none() {
                    o.ready[VOLLEY_I] = VOLLEY_EVERY * quick;
                    Doing::Circle
                } else if t <= 0.0 {
                    if let (Some(tp), Some(i)) = (tp, book.spells.iter().position(|s| s.id == "abyssal_bolt")) {
                        let at = (tp - maw).normalize_or(Vec2::X);
                        // A fan, one after another; a spiral all round it
                        // (they curl in onto you); a stream straight at you.
                        let (way, every) = match o.pattern {
                            0 => (Vec2::from_angle((left as f32 - 2.0) * 0.18).rotate(at), BOLT_EVERY),
                            1 => (Vec2::from_angle(left as f32 * std::f32::consts::TAU / SPIRAL.1 as f32).rotate(at), SPIRAL.0),
                            _ => (at, STREAM.0),
                        };
                        casts.write(crate::magic::CastRequest { caster: e, spell: i, from: maw + way * 14.0, toward: maw + way * 300.0, alt: false });
                        sounds.write(crate::sound::PlaySound::at("cast", maw).pitch(0.35).volume(0.9));
                        Doing::Volley(every, left - 1)
                    } else {
                        Doing::Volley(BOLT_EVERY, left - 1)
                    }
                } else {
                    Doing::Volley(t - DT, left)
                };
            }
            Doing::Charge(t) => {
                // Flashing all over (the tell), then the pulse.
                sw.flare = ((t / CHARGE).min(1.0), -1.0);
                next = if t + DT >= CHARGE {
                    o.pulse = Some((maw, 0.0));
                    o.pulsed.clear();
                    sounds.write(crate::sound::PlaySound::at("thoom", maw).pitch(0.6).volume(1.3));
                    trauma.0 = (trauma.0 + 0.3).min(1.0);
                    flares.write(crate::fx::Flare { at: maw, color: [0.5, 0.8, 1.0], life: 0.25 });
                    o.ready[PULSE_I] = PULSE_EVERY * quick;
                    Doing::Spent(0.0)
                } else {
                    Doing::Charge(t + DT)
                };
            }
            Doing::Undertow(t) => {
                // Hanging, gaping: the water swirls into its maw.
                sw.flare = (0.8, 0.0);
                for (who, mut tk, _) in hunted.iter_mut() {
                    if who == e {
                        continue;
                    }
                    let d = maw - tk.body.pos;
                    let l = d.length();
                    if l < UNDERTOW.1 && l > 8.0 && wet(tk.body.pos) {
                        let pull = d / l * UNDERTOW.2 * (1.0 - l / UNDERTOW.1).max(0.25);
                        tk.body.vel += (pull + pull.perp() * UNDERTOW.3) * DT;
                    }
                }
                if (o.clock * 20.0) as u32 % 2 == 0 {
                    let a = o.clock * 7.0;
                    let at = maw + Vec2::from_angle(a) * 120.0;
                    sparks.emit(&emitter(vec![(220, 245, 255), (120, 200, 255), (30, 70, 150)], 30.0, (0.3, 0.7), 1.2), 2, at, (maw - at).normalize_or(Vec2::X), (maw - at) * 1.5);
                }
                next = if t + DT >= UNDERTOW.0 {
                    o.ready[UNDERTOW_I] = UNDERTOW_EVERY * quick;
                    Doing::Wind(WIND.0 * 0.6)
                } else {
                    Doing::Undertow(t + DT)
                };
            }
            Doing::Spent(t) => {
                next = if t + DT >= SPENT { Doing::Circle } else { Doing::Spent(t + DT) };
            }
        }
        o.doing = next;
        // Its body's a blow, swept or coiled through what's near (each once
        // in a while).
        if matches!(next, Doing::Sweep(_) | Doing::Coil(_) | Doing::Strike(..)) && k.body.vel.length() > 150.0 {
            let r = 14.0;
            for (who, tk, team) in hunted.iter() {
                if !team.hunted() || o.struck.iter().any(|s| s.0 == who) {
                    continue;
                }
                if let Some(&(at, dir)) = sw.plates.iter().find(|(p, _)| p.distance(tk.body.pos) < r + tk.body.half.length()) {
                    let away = (tk.body.pos - at).normalize_or(dir.perp());
                    hits.write(crate::combat::Hit { by: Some(e), target: who, damage: BODY_HIT.0 * o.bite, harm: Harm::Blunt, knock: away * BODY_HIT.1, stun: BODY_HIT.2, at, dir: away, weight: 1.0, crit: false });
                    sounds.write(crate::sound::PlaySound::at("slam", at).pitch(0.5).volume(1.0));
                    o.struck.push((who, BODY_EVERY));
                }
            }
        }
        // The pulse running out: a ring of force through the water, what it
        // passes knocked away from it and stunned (all in the water but
        // itself).
        if let Some((from, r)) = o.pulse {
            let r2 = r + RING.1 * DT;
            for (who, tk, _) in hunted.iter() {
                let d = tk.body.pos.distance(from);
                if who == e || o.pulsed.contains(&who) || d > r2 || !wet(tk.body.pos) {
                    continue;
                }
                let away = (tk.body.pos - from).normalize_or(Vec2::Y);
                hits.write(crate::combat::Hit { by: Some(e), target: who, damage: PULSE_HIT.0 * o.bite, harm: Harm::Blunt, knock: away * PULSE_HIT.1, stun: PULSE_HIT.2, at: tk.body.pos, dir: away, weight: 1.0, crit: false });
                o.pulsed.push(who);
            }
            // (Its ring: light all round it.)
            let n = (r2 / 6.0) as usize + 8;
            for i in 0..n.min(90) {
                let a = i as f32 / n as f32 * std::f32::consts::TAU + o.clock;
                let at = from + Vec2::from_angle(a) * r2;
                if wet(at) && (platypus_sim::rng::hash(&[i as u64, (o.clock * 60.0) as u64]) % 3 == 0) {
                    sparks.emit(&emitter(vec![(235, 250, 255), (130, 210, 255), (30, 80, 170)], 40.0, (0.15, 0.4), 1.4), 1, at, Vec2::from_angle(a), Vec2::ZERO);
                }
            }
            o.pulse = (r2 < RING.0).then_some((from, r2));
        }
        // Its face: gaping in its tells and as it strikes, snapped after.
        let clip = match next {
            Doing::Wind(_) | Doing::Charge(_) | Doing::Undertow(_) | Doing::Gather(_) | Doing::Volley(..) => "gape",
            Doing::Strike(_, false) => "gape",
            Doing::Strike(_, true) | Doing::Spent(_) => "ajar",
            _ => "idle",
        };
        if anim.force.as_deref() != Some(clip) {
            anim.play(clip);
        }
        let l = to.length();
        let want = if l > 1.0 { to / l * hurry } else { Vec2::ZERO };
        c.0.move_x = want.x;
        c.0.move_y = want.y;
        c.0.jump = false;
        c.0.aim = tp.unwrap_or(pos + Vec2::from_angle(sw.heading) * 60.0);
    }
}
