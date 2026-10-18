//! Cave bats: colonies asleep on the roofs of caves (a roost lair's, or a
//! few anywhere in the upper caves), guano heaped under them. Asleep, a
//! bat hangs upside down, wrapped in its wings, its eyes shut: nothing
//! shines back in the dark.
//!
//! - **Waking**: what it hears (mining, a fight, a blast, footsteps near; a
//!   crouch is quieter), a lamp near, someone right under it, a blow, a
//!   flash. It stirs, opens its eyes, and screeches, and the screech wakes
//!   the rest: the colony pours off the roof in a ripple.
//! - **Swarming**: awake, it circles what it's after with the rest, each its
//!   own way round, keeping off the others; now and then one (no more than
//!   two at once) holds, screeches, its eyes flaring (the tell), and dives
//!   straight at you, bites once, and pulls away up.
//! - **A flash** (a blast, a flare, a radiant spell) knocks it senseless: it
//!   drops.
//! - With no one to hunt a while, it flies up to a roof and sleeps again.
//!
//! One blow kills one; a swing can take two. A swarm's dangerous as a
//! swarm: stand still in it and it bites you down.

use bevy::prelude::*;
use serde::Deserialize;

use super::watcher::clear;
use super::{CustomCreature, CustomSet};
use crate::creatures::body::animation::Animator;
use crate::creatures::{Controls, Harm, Kinematics, Team};
use crate::world::SimWorld;

#[derive(Component, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct CaveBat {
    #[serde(skip)]
    doing: Doing,
    /// Till its next dive (s).
    #[serde(skip)]
    wait: f32,
    /// Its way round what it's after: angle, which way (±1), how far out.
    #[serde(skip)]
    orbit: (f32, f32, f32),
    /// How long it's had no one to hunt.
    #[serde(skip)]
    lost: f32,
    /// Its dive's way.
    #[serde(skip)]
    aim: Vec2,
    #[serde(skip)]
    clock: f32,
    #[serde(skip)]
    seed: f32,
    #[serde(skip)]
    started: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Doing {
    /// Flying up to a roof to sleep.
    #[default]
    Settle,
    /// Asleep, hanging.
    Roost,
    /// Woken, its eyes open, about to screech and go.
    Stir(f32),
    /// Circling with the rest.
    Swarm,
    /// Holding, screeching, its eyes flaring: about to dive.
    Tell(f32),
    /// Diving; whether it's bitten.
    Dive(f32, bool),
    /// Pulling away up after a dive.
    Pull(f32),
}

impl CustomCreature for CaveBat {
    const NAME: &'static str = "cave_bat";

    fn build(app: &mut App) {
        app.add_systems(FixedUpdate, (wake, think).chain().in_set(CustomSet::Think));
    }
}

impl CaveBat {
    /// What it's doing, in a word (a scenario's log).
    pub fn doing(&self) -> &'static str {
        match self.doing {
            Doing::Settle => "settle",
            Doing::Roost => "roost",
            Doing::Stir(_) => "stir",
            Doing::Swarm => "swarm",
            Doing::Tell(_) => "tell",
            Doing::Dive(..) => "dive",
            Doing::Pull(_) => "pull",
        }
    }

    fn awake(&self) -> bool {
        !matches!(self.doing, Doing::Settle | Doing::Roost | Doing::Stir(_))
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
/// Hearing: a sound this loud (its volume 1) wakes it from this far (cells);
/// footsteps carry this share of that, a blast this many times it.
const HEAR: f32 = 90.0;
const STEPS: f32 = 0.35;
const BLAST: f32 = 2.5;
/// Someone this near wakes it (crouching, this near; under it counts half
/// as far: it looks down); a lamp this near and this bright.
const NEAR: (f32, f32) = (34.0, 14.0);
const LAMP: (f32, f32) = (30.0, 0.3);
/// Woken, it stirs this long (s, from..to: the colony ripples awake).
const STIR: (f32, f32) = (0.12, 0.7);
/// Hunting what's this near; giving up after this long with no one, and
/// going back up to sleep.
const HUNT: f32 = 420.0;
const GIVE_UP: f32 = 8.0;
/// Circling: how far out (from..to), how high over them, how fast round
/// (rad/s, from..to); keeping this far off the others.
const RING: (f32, f32) = (34.0, 62.0);
const OVER: f32 = 26.0;
const ROUND: (f32, f32) = (1.2, 2.2);
const APART: f32 = 9.0;
/// Its dives: every so often (s, from..to), no more than this many of a
/// swarm at once; its tell this long; diving this fast, this long at most;
/// its bite (damage, knock); pulling away this long.
const DIVE_EVERY: (f32, f32) = (2.2, 4.8);
const DIVING: usize = 2;
const TELL: f32 = 0.35;
const DIVE: (f32, f32) = (270.0, 0.5);
const BITE: (f32, f32) = (5.0, 140.0);
const PULL: f32 = 0.45;
/// A flash this near and this bright knocks it senseless this long.
const FLASH: (f32, f32, f32) = (80.0, 1.0, 1.4);

fn between(seed: f32, (lo, hi): (f32, f32)) -> f32 {
    lo + (hi - lo) * seed
}

/// What wakes the sleepers (sounds, someone near, a lamp, a blow, a flash),
/// and what knocks the awake out of the air (a flash).
#[allow(clippy::type_complexity)]
fn wake(
    sim: Res<SimWorld>,
    mut q: Query<(Entity, &mut CaveBat, &mut Kinematics)>,
    hunted: Query<(&Kinematics, &Team, Option<&Controls>), Without<CaveBat>>,
    lamps: Query<(&GlobalTransform, &crate::light::LightSource), Without<crate::magic::Spell>>,
    (mut heard, mut struck, mut flashes): (MessageReader<crate::sound::PlaySound>, MessageReader<crate::combat::Hit>, MessageReader<crate::fx::Flare>),
) {
    // (Ambient beds, drips, music: not what wakes a bat.)
    let noises: Vec<(Vec2, f32)> = heard
        .read()
        .filter_map(|s| {
            let at = s.at?;
            let name = s.name.as_ref();
            if name.ends_with("_bed") || name.starts_with("music") || name == "drip" {
                return None;
            }
            let loud = if name.starts_with("step") {
                STEPS
            } else if name == "boom" || name == "thunder" {
                BLAST
            } else {
                1.0
            };
            Some((at, HEAR * loud * s.volume))
        })
        .collect();
    let blows: Vec<Entity> = struck.read().map(|h| h.target).collect();
    let flashes: Vec<Vec2> = flashes.read().filter(|f| f.color.iter().copied().fold(0.0f32, f32::max) >= FLASH.1).map(|f| f.at).collect();
    let lamps: Vec<Vec2> = lamps.iter().filter(|(_, l)| l.color.iter().copied().fold(0.0f32, f32::max) >= LAMP.1).map(|(t, _)| t.translation().truncate()).collect();
    for (e, mut o, mut k) in &mut q {
        if !o.started {
            continue;
        }
        let pos = k.body.pos;
        let flashed = flashes.iter().any(|&f| f.distance(pos) < FLASH.0 && clear(&sim, f, pos));
        if o.awake() {
            // Knocked senseless: it drops.
            if flashed {
                let k = &mut *k;
                let drop = Vec2::new(k.body.vel.x * 0.3, -20.0);
                k.loco.knock(&mut k.body, drop, FLASH.2);
                o.doing = Doing::Swarm;
                o.wait = o.wait.max(FLASH.2 + 0.5);
            }
            continue;
        }
        let near = hunted.iter().filter(|(_, t, _)| t.hunted()).any(|(hk, _, c)| {
            let d = hk.body.pos - pos;
            let below = if d.y < 0.0 { 0.5 } else { 1.0 };
            Vec2::new(d.x, d.y * below).length() < if c.is_some_and(|c| c.0.crouch) { NEAR.1 } else { NEAR.0 }
        });
        let woken = flashed || near || blows.contains(&e) || noises.iter().any(|&(at, far)| at.distance(pos) < far) || lamps.iter().any(|l| l.distance(pos) < LAMP.0);
        if woken && matches!(o.doing, Doing::Roost | Doing::Settle) {
            o.doing = Doing::Stir(between(o.seed, STIR));
        }
    }
}

#[allow(clippy::type_complexity)]
fn think(
    sim: Res<SimWorld>,
    mut q: Query<(Entity, &mut CaveBat, &mut Kinematics, &mut Controls, &mut Animator)>,
    hunted: Query<(Entity, &Kinematics, &Team), Without<CaveBat>>,
    (mut hits, mut sounds, mut flares): (MessageWriter<crate::combat::Hit>, MessageWriter<crate::sound::PlaySound>, MessageWriter<crate::fx::Flare>),
) {
    // (Where the swarm is, and how many of it are diving.)
    let flock: Vec<(Entity, Vec2)> = q.iter().filter(|x| x.1.awake()).map(|(e, _, k, ..)| (e, k.body.pos)).collect();
    let mut diving = q.iter().filter(|x| matches!(x.1.doing, Doing::Tell(_) | Doing::Dive(..))).count();
    for (e, mut o, mut k, mut c, mut anim) in &mut q {
        let pos = k.body.pos;
        if !o.started {
            o.started = true;
            o.seed = (e.to_bits() % 1009) as f32 / 1009.0;
            o.orbit = (o.seed * std::f32::consts::TAU, if o.seed < 0.5 { 1.0 } else { -1.0 }, between(o.seed, RING));
            o.wait = between((o.seed * 7.3).fract(), DIVE_EVERY);
        }
        o.clock += DT;
        o.wait -= DT;
        let target = hunted.iter().filter(|(_, tk, t)| t.hunted() && tk.body.pos.distance(pos) < HUNT).min_by(|a, b| a.1.body.pos.distance(pos).total_cmp(&b.1.body.pos.distance(pos)));
        let tp = target.map(|(_, tk, _)| tk.body.pos);
        if o.awake() {
            o.lost = if tp.is_some() { 0.0 } else { o.lost + DT };
        }
        // (Knocked senseless: nothing to do but fall.)
        let stunned = k.loco.state == platypus_physics::MoveState::Stunned;
        let doing = o.doing;
        let mut next = doing;
        match doing {
            Doing::Settle => {
                if k.loco.contacts.ceiling {
                    next = Doing::Roost;
                }
            }
            Doing::Roost => {
                // (Its roof gone from over it: up to another.)
                if !k.loco.contacts.ceiling {
                    next = Doing::Settle;
                }
            }
            Doing::Stir(t) => {
                if t <= 0.0 {
                    // The screech: it wakes the rest.
                    sounds.write(crate::sound::PlaySound::at("bat_screech", pos).volume(0.9).pitch(0.9 + o.seed * 0.3));
                    next = Doing::Swarm;
                } else {
                    next = Doing::Stir(t - DT);
                }
            }
            Doing::Swarm => {
                if o.lost > GIVE_UP {
                    next = Doing::Settle;
                } else if let Some(tp) = tp
                    && o.wait <= 0.0
                    && diving < DIVING
                    && !stunned
                    && clear(&sim, pos, tp)
                {
                    diving += 1;
                    sounds.write(crate::sound::PlaySound::at("bat_screech", pos).volume(0.6).pitch(1.3 + o.seed * 0.2));
                    next = Doing::Tell(0.0);
                }
            }
            Doing::Tell(t) => {
                if let Some((_, tk, _)) = target {
                    // (Where they'll be as it gets there, near enough.)
                    o.aim = (tk.body.pos + tk.body.vel * 0.12 - pos).normalize_or(Vec2::NEG_Y);
                }
                next = if t + DT >= TELL { Doing::Dive(0.0, false) } else { Doing::Tell(t + DT) };
            }
            Doing::Dive(t, bit) => {
                let mut bit = bit;
                if !bit && let Some((who, _, _)) = target.filter(|(_, tk, _)| ((pos - tk.body.pos).abs() - tk.body.half - k.body.half).max_element() <= 1.5) {
                    hits.write(crate::combat::Hit { target: who, damage: BITE.0, harm: Harm::Pierce, knock: o.aim * BITE.1, stun: 0.08, at: pos, dir: o.aim, weight: 0.3, crit: false });
                    sounds.write(crate::sound::PlaySound::at("snap", pos).pitch(1.7).volume(0.45));
                    bit = true;
                }
                // (Off its line: it struck rock.)
                let blocked = t > 0.03 && (k.body.vel - o.aim * DIVE.0).length() > DIVE.0 * 0.3;
                next = if bit || blocked || stunned || t + DT >= DIVE.1 { Doing::Pull(0.0) } else { Doing::Dive(t + DT, bit) };
            }
            Doing::Pull(t) => {
                next = if t + DT >= PULL {
                    o.wait = between((o.seed * 13.7 + o.clock * 0.37).fract(), DIVE_EVERY);
                    Doing::Swarm
                } else {
                    Doing::Pull(t + DT)
                };
            }
        }
        o.doing = next;
        // Its picture: hanging asleep, eyes open stirring, wings beating.
        let clip = match next {
            Doing::Roost => "hang",
            Doing::Stir(_) => "stir",
            Doing::Dive(..) => "dive",
            _ => "fly",
        };
        if anim.force.as_deref() != Some(clip) {
            anim.play(clip);
        }
        // Its flight.
        c.0.jump = false;
        c.0.aim = Vec2::ZERO;
        match next {
            Doing::Settle => {
                // Straight up, a little this way and that.
                c.0.move_x = (o.clock * 1.7 + o.seed * 9.0).sin() * 0.4;
                c.0.move_y = 1.0;
            }
            Doing::Roost | Doing::Stir(_) => {
                // Pressed to the roof, still.
                c.0.move_x = 0.0;
                c.0.move_y = 0.6;
                k.body.vel.x = 0.0;
            }
            Doing::Swarm | Doing::Pull(_) => {
                // Round what it's after, each its own way, high over them,
                // off the others, jittering (a bat never flies straight).
                let mut steer = Vec2::new((o.clock * 0.9 + o.seed * 5.0).sin(), (o.clock * 0.6).cos() * 0.5);
                if let Some(tp) = tp {
                    o.orbit.0 += o.orbit.1 * between(o.seed, ROUND) * DT;
                    let (a, r) = (o.orbit.0, o.orbit.2);
                    let want = tp + Vec2::new(a.cos() * r, OVER + a.sin() * r * 0.45);
                    steer = (want - pos) / 20.0;
                }
                for &(other, at) in &flock {
                    let d = pos.distance(at);
                    if other != e && d < APART && d > 0.01 {
                        steer += (pos - at) / d * (1.0 - d / APART) * 2.0;
                    }
                }
                steer += Vec2::new((o.clock * 7.0 + o.seed * 31.0).sin(), (o.clock * 5.3 + o.seed * 17.0).cos()) * 0.35;
                // Pulling away: up and back the way it came.
                if matches!(next, Doing::Pull(_)) {
                    steer = Vec2::new(-o.aim.x.signum() * 0.6, 1.0);
                }
                let s = steer.clamp_length_max(1.0);
                c.0.move_x = s.x;
                c.0.move_y = s.y;
            }
            Doing::Tell(t) => {
                // Holding, its eyes flaring, facing its dive.
                k.body.vel *= 0.85;
                c.0.move_x = 0.0;
                c.0.move_y = 0.05;
                c.0.aim = pos + o.aim * 20.0;
                if t == 0.0 {
                    flares.write(crate::fx::Flare { at: pos, color: [0.9, 0.12, 0.08], life: TELL });
                }
            }
            Doing::Dive(..) => {
                k.body.vel = o.aim * DIVE.0;
                c.0.move_x = o.aim.x;
                c.0.move_y = o.aim.y;
                c.0.aim = pos + o.aim * 20.0;
            }
        }
    }
}
