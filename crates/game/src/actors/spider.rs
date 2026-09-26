//! A big spider's attacks (its `crawler` brain's `bite`, `spit`, `sting`):
//! each with a moment to read before it lands.
//!
//! - Bite: it crouches back, then lunges; what its jaws meet is hurt and
//!   thrown hard.
//! - Spit: at range, with a clear line, it rears and lobs a glob of acid
//!   (`spider_spit` in spells.ron: any creature casts as the player does).
//! - Sting: close, it rises on its legs and curls its stinger up over its
//!   back, holds it trembling, then strikes down past its head: a heavy
//!   blow, and venom (a coating only water washes off).
//!
//! While it attacks it stands (holding whatever it holds); `legs::Rear`
//! draws the crouch, the rearing and the curl.

use bevy::prelude::*;
use serde::Deserialize;

use super::elements::Coatings;
use super::legs::Rear;
use super::monsters::Crawler;
use super::{Controls, Kinematics, Team};
use crate::combat::Hit;
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;

/// A bite: within `range` (cells), a crouch of `windup` s, a lunge at
/// `lunge` cells/s; `damage`, knockback `knock` (cells/s), `stun`; then
/// `every` s before the next.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Bite {
    pub range: f32,
    pub windup: f32,
    pub lunge: f32,
    pub damage: f32,
    pub knock: f32,
    pub stun: f32,
    pub every: f32,
}

impl Default for Bite {
    fn default() -> Self {
        Bite { range: 24.0, windup: 0.3, lunge: 280.0, damage: 16.0, knock: 330.0, stun: 0.35, every: 1.5 }
    }
}

/// A spit: a `spell` cast when the target is within `range` (near, far)
/// with nothing in the way, after rearing `windup` s; `every` s apart.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Spit {
    pub spell: String,
    pub range: (f32, f32),
    pub windup: f32,
    pub every: f32,
}

impl Default for Spit {
    fn default() -> Self {
        Spit { spell: "spider_spit".into(), range: (16.0, 170.0), windup: 0.45, every: 3.5 }
    }
}

/// A sting: within `range`, it rises for `rise` s, holds `hold` s, strikes;
/// `damage`, `knock`, `stun`, and the coating `venom` left behind; `every`
/// s apart.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Sting {
    pub range: f32,
    pub rise: f32,
    pub hold: f32,
    pub damage: f32,
    pub knock: f32,
    pub stun: f32,
    pub venom: String,
    pub every: f32,
}

impl Default for Sting {
    fn default() -> Self {
        Sting { range: 28.0, rise: 0.5, hold: 0.35, damage: 30.0, knock: 220.0, stun: 0.4, venom: "venom".into(), every: 5.0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Move {
    Bite,
    Spit,
    Sting,
}

/// What it's doing: an attack under way (which, how far in, at whom, which
/// way), and when each may come again.
#[derive(Component, Default)]
pub struct Assault {
    doing: Option<(Move, f32, Entity, Vec2)>,
    struck: bool,
    ready: [f32; 3],
}

impl Assault {
    /// The attack under way, if any: its name and how far in (seconds).
    pub fn doing(&self) -> Option<(&'static str, f32)> {
        self.doing.map(|(m, t, ..)| {
            let name = match m {
                Move::Bite => "bite",
                Move::Spit => "spit",
                Move::Sting => "sting",
            };
            (name, t)
        })
    }
}

/// Seconds a strike (the sting's stab, the bite's lunge) lasts, and the
/// recovery after.
const STRIKE: f32 = 0.12;
const LUNGE: f32 = 0.22;
const RECOVER: f32 = 0.45;
/// Its jaws (and its stinger's reach past its head), from its middle.
const HEAD: f32 = 10.0;
/// A spit's speed and how fast it falls (the `arrow` rune's, with `heavy`:
/// cells/s, cells/s²), to aim the lob.
const SPIT_SPEED: f32 = 300.0;
const SPIT_FALL: f32 = 450.0;

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Nothing solid on the line from `a` to `b` (a look every two cells).
fn clear(sim: &SimWorld, a: Vec2, b: Vec2) -> bool {
    let n = (a.distance(b) / 2.0).ceil() as i32;
    (1..n).all(|i| {
        let p = a.lerp(b, i as f32 / n as f32);
        !sim.world.is_solid(platypus_sim::CellPos::from_world(p.x, p.y))
    })
}

type Attacker<'a> = (Entity, &'a Crawler, &'a mut Kinematics, &'a mut Controls, Option<&'a mut Assault>, Option<&'a mut Rear>);
type Prey<'a> = (Entity, &'a Kinematics, &'a Team);

/// Start an attack when one's in reach and ready; carry on the one under
/// way (after the brain: an attacking spider stands).
#[allow(clippy::too_many_arguments)]
pub fn attack(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<SimWorld>,
    book: Res<crate::magic::Spellbook>,
    coatings: Res<Coatings>,
    mut hits: MessageWriter<Hit>,
    mut casts: MessageWriter<crate::magic::CastRequest>,
    mut spiders: Query<Attacker, Without<crate::actors::player::LocalPlayer>>,
    prey: Query<Prey, With<crate::actors::player::LocalPlayer>>,
) {
    let now = time.elapsed_secs();
    for (e, brain, mut k, mut c, assault, rear) in &mut spiders {
        if brain.bite.is_none() && brain.spit.is_none() && brain.sting.is_none() {
            continue;
        }
        let (Some(mut a), Some(mut rear)) = (assault, rear) else {
            commands.entity(e).insert((Assault::default(), Rear::default()));
            continue;
        };
        let pos = k.body.pos;
        let holds = k.loco.grounded() || k.loco.clinging().is_some();
        // Start one.
        if a.doing.is_none() {
            *rear = Rear::default();
            let Some((pe, pk, _)) = prey.iter().filter(|(_, _, t)| **t == Team::Player).min_by(|x, y| x.1.body.pos.distance(pos).total_cmp(&y.1.body.pos.distance(pos))) else { continue };
            let to = pk.body.pos - pos;
            let dist = to.length();
            let dir = to.normalize_or(Vec2::X);
            let start = if let Some(s) = &brain.sting
                && holds
                && dist < s.range
                && now >= a.ready[2]
            {
                Some(Move::Sting)
            } else if let Some(b) = &brain.bite
                && holds
                && dist < b.range
                && now >= a.ready[0]
            {
                Some(Move::Bite)
            } else if let Some(s) = &brain.spit
                && (s.range.0..s.range.1).contains(&dist)
                && now >= a.ready[1]
                && clear(&sim, pos, pk.body.pos)
            {
                Some(Move::Spit)
            } else {
                None
            };
            if let Some(m) = start {
                a.doing = Some((m, 0.0, pe, dir));
                a.struck = false;
            }
            continue;
        }
        let Some((m, t, pe, dir)) = a.doing else { continue };
        let t = t + DT;
        a.doing = Some((m, t, pe, dir));
        // It stands while it attacks.
        c.0.move_x = 0.0;
        c.0.move_y = 0.0;
        c.0.jump = false;
        let target = prey.get(pe).ok().map(|(_, pk, _)| pk);
        if let Some(pk) = target {
            c.0.aim = pk.body.pos;
        }
        let head = pos + dir * HEAD;
        // Does the strike reach them?
        let reaches = |pk: &Kinematics, reach: f32| ((pk.body.pos - head).abs() - pk.body.half).max_element() <= reach;
        let done = match m {
            Move::Bite => {
                let b = brain.bite.as_ref().expect("biting");
                if t < b.windup {
                    rear.back = 3.0 * smooth(t / b.windup);
                    rear.lift = 2.0 * smooth(t / b.windup);
                } else if t < b.windup + LUNGE {
                    rear.back = 0.0;
                    rear.lift = 1.0;
                    if t - DT < b.windup {
                        // (Off the ground a little: a leap, not a slide.)
                        k.body.vel = dir * b.lunge + Vec2::Y * 60.0;
                    }
                    if !a.struck
                        && let Some(pk) = target
                        && reaches(pk, 4.0)
                    {
                        a.struck = true;
                        hits.write(Hit { target: pe, damage: b.damage, knock: (dir + Vec2::Y * 0.4).normalize() * b.knock, stun: b.stun, at: head, dir, weight: b.damage / 12.0, crit: false });
                    }
                } else {
                    rear.lift = 0.0;
                }
                t >= b.windup + LUNGE + RECOVER
            }
            Move::Spit => {
                let s = brain.spit.as_ref().expect("spitting");
                if t < s.windup {
                    rear.lift = 4.0 * smooth(t / s.windup);
                    rear.back = 2.0 * smooth(t / s.windup);
                } else if !a.struck {
                    a.struck = true;
                    rear.back = -2.0;
                    if let (Some(pk), Some(spell)) = (target, book.spells.iter().position(|x| x.id == s.spell)) {
                        // Where they'll be when it gets there, and above
                        // that by as far as the glob falls on the way (a lob).
                        let from = head + Vec2::Y * 3.0;
                        let flight = from.distance(pk.body.pos) / SPIT_SPEED;
                        let lead = pk.body.pos + pk.body.vel * flight + Vec2::Y * 0.5 * SPIT_FALL * flight * flight;
                        casts.write(crate::magic::CastRequest { caster: e, spell, from, toward: lead, alt: false });
                    }
                } else {
                    rear.lift *= 0.85;
                    rear.back *= 0.85;
                }
                t >= s.windup + RECOVER
            }
            Move::Sting => {
                let s = brain.sting.as_ref().expect("stinging");
                if t < s.rise {
                    // Up on its legs, the stinger curling up over its back.
                    let f = smooth(t / s.rise);
                    rear.lift = 8.0 * f;
                    rear.curl = 0.55 * f;
                } else if t < s.rise + s.hold {
                    // Held, trembling.
                    rear.lift = 8.0;
                    rear.curl = 0.55 + 0.03 * ((t * 40.0).sin());
                } else if t < s.rise + s.hold + STRIKE {
                    // Down past its head.
                    let f = (t - s.rise - s.hold) / STRIKE;
                    rear.curl = 0.55 + 0.45 * f;
                    rear.lift = 8.0 - 4.0 * f;
                    if !a.struck
                        && f > 0.5
                        && let Some(pk) = target
                        && reaches(pk, 8.0)
                    {
                        a.struck = true;
                        hits.write(Hit { target: pe, damage: s.damage, knock: (dir + Vec2::Y * 0.3).normalize() * s.knock, stun: s.stun, at: head, dir, weight: s.damage / 12.0, crit: false });
                        if let Some(v) = coatings.by_name.get(&s.venom) {
                            commands.entity(pe).try_insert(super::elements::Coated { name: s.venom.clone(), left: v.secs, total: v.secs });
                        }
                    }
                } else {
                    let f = ((t - s.rise - s.hold - STRIKE) / RECOVER).clamp(0.0, 1.0);
                    rear.curl = 1.0 - f;
                    rear.lift = 4.0 * (1.0 - f);
                }
                t >= s.rise + s.hold + STRIKE + RECOVER
            }
        };
        if done {
            let every = match m {
                Move::Bite => brain.bite.as_ref().map_or(1.0, |b| b.every),
                Move::Spit => brain.spit.as_ref().map_or(1.0, |s| s.every),
                Move::Sting => brain.sting.as_ref().map_or(1.0, |s| s.every),
            };
            a.ready[m as usize] = now + every;
            // (Not straight into another: a breath between.)
            for r in &mut a.ready {
                *r = r.max(now + 0.4);
            }
            a.doing = None;
            *rear = Rear::default();
        }
    }
}
