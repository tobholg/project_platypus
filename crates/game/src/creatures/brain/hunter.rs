//! `hunter`: the general fighting brain (DESIGN §14.1). Every creature that
//! hunts uses it, set in its file from a few choices:
//!
//! - `aggro`: it notices what it hunts (players and villagers, not one
//!   hiding at home) within this (cells).
//! - `close`: how it goes at what it's noticed:
//!   - `Walk(keep, jump_to_reach)`: on foot to `keep` cells off, jumping
//!     walls, and up at you when you're `jump_to_reach` higher and near
//!     (orcs, skeletons, the troll);
//!   - `Range(near, far)`: on foot, keeping between `near` and `far`
//!     (archers);
//!   - `Swoop(hover, dive_time, dive_every)`: flying in a band (cells over
//!     the ground) and diving at you now and then (bats, star wisps);
//!   - `Hop(every)`: a hop at you every so often (slimes);
//!   - `Crawl(pounce_range, pounce_every)`: over floors, walls and ceilings
//!     (its movement's `cling`), pouncing when near or right above you
//!     (spiders).
//! - `attack`: `Touch` (what it touches it hurts: its file's `touch`, and
//!   its own moves), `Swing(reach, every, combo)` (what it wields, `combo`
//!   moves in a row when you're within `reach`, then a wait of `every` s),
//!   `Shoot(draw, every, wobble)` (its bow: a `draw` of a full draw, leading
//!   you, a shot every `every` s, `wobble` degrees off).
//! - `wander`: with no one about, on foot or crawling: how fast (a share of
//!   its run) and how often it changes its mind (seconds).
//! - `leash`: one keeping a place (`clock::Keeps`: a star's crater) goes
//!   for you only within this of it, and drifts back when you've gone.
//! - On the march (`Marching`: a raid), with no one to fight it walks there
//!   instead of wandering.
//!
//! What a creature file said as a brain of its own before (`melee_walker`,
//! `archer`, `swooper`, `hopper`, `crawler`) is one of these settings now.

use bevy::prelude::*;
use platypus_sim::rng::Rng;
use serde::Deserialize;

use super::RegisterBrain;
use super::ai::Marching;
use crate::creatures::{Controls, Kinematics, Team};
use crate::world::SimWorld;

pub struct HunterPlugin;

impl Plugin for HunterPlugin {
    fn build(&self, app: &mut App) {
        app.register_brain::<Hunter>("hunter").add_systems(FixedUpdate, (hunt, crate::creatures::moves::run).chain().in_set(super::BrainSet));
    }
}

const TICKS: f32 = crate::world::TICK_HZ as f32;

/// The settings (its file's `brain.params`).
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Hunter {
    pub aggro: f32,
    pub close: Close,
    pub attack: Attack,
    pub wander: Wander,
    pub leash: f32,
}

impl Default for Hunter {
    fn default() -> Self {
        Hunter { aggro: 300.0, close: Close::default(), attack: Attack::Touch, wander: Wander::default(), leash: 0.0 }
    }
}

/// How it goes at what it's noticed.
#[derive(Deserialize, Clone, Debug)]
pub enum Close {
    Walk {
        #[serde(default = "keep")]
        keep: f32,
        #[serde(default = "jump_to_reach")]
        jump_to_reach: f32,
    },
    Range { near: f32, far: f32 },
    Swoop { hover: (f32, f32), dive_time: f32, dive_every: f32 },
    Hop { every: f32 },
    Crawl { pounce_range: f32, pounce_every: f32 },
}

impl Default for Close {
    fn default() -> Self {
        Close::Walk { keep: keep(), jump_to_reach: jump_to_reach() }
    }
}

fn keep() -> f32 {
    15.0
}

fn jump_to_reach() -> f32 {
    27.0
}

/// How it attacks.
#[derive(Deserialize, Clone, Debug, Default)]
pub enum Attack {
    #[default]
    Touch,
    Swing { reach: f32, every: f32, combo: u8 },
    Shoot { draw: f32, every: f32, wobble: f32 },
}

/// With no one about: how fast it potters (a share of its run), how often
/// it changes its mind (seconds, ±50 %).
#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(default)]
pub struct Wander {
    pub speed: f32,
    pub every: f32,
}

impl Default for Wander {
    fn default() -> Self {
        Wander { speed: 0.4, every: 2.5 }
    }
}

/// What a hunter remembers (ticks).
#[derive(Component, Default)]
pub struct HunterMind {
    /// Where it's wandering, until when.
    wander: Vec2,
    wander_until: u64,
    /// When it may act next (pounce, hop, attack, shot), a dive's end.
    next: u64,
    dive_until: u64,
    /// A swing under way: moves still to ask for, whether one's on.
    swings_left: u8,
    attacking: bool,
    /// Drawing its bow since.
    drawing: Option<u64>,
}

/// What hunters go after (a villager hiding at home is let be).
type Hunted<'w, 's> = Query<'w, 's, (&'static Kinematics, &'static Team), Without<super::villager::Hiding>>;

type Hunting<'a> = (
    Entity,
    &'a Hunter,
    &'a Kinematics,
    &'a mut Controls,
    Option<&'a mut HunterMind>,
    Option<&'a Marching>,
    Option<&'a crate::clock::Keeps>,
    Has<crate::combat::Swing>,
    Option<&'a crate::combat::Wielding>,
);

/// 0..1 from a roll seeded by the world, the tick, the creature and a salt.
fn unit(sim: &SimWorld, e: Entity, salt: u64) -> f32 {
    let mut rng = Rng::seeded(&[sim.world.seed(), sim.world.tick(), e.to_bits(), salt]);
    (rng.next_u32() % 10_001) as f32 / 10_000.0
}

/// Seconds as ticks.
fn ticks(secs: f32) -> u64 {
    (secs.max(0.0) * TICKS) as u64
}

/// Which way a march goes from `x` (0: there, or not marching).
fn march(m: Option<&Marching>, x: f32) -> f32 {
    super::ai::march(m, x)
}

#[allow(clippy::too_many_arguments)]
fn hunt(
    mut commands: Commands,
    sim: Res<SimWorld>,
    weapons: Option<Res<crate::combat::Weapons>>,
    hunted: Hunted,
    mut q: Query<Hunting>,
    mut swings: MessageWriter<crate::combat::MeleeRequest>,
    mut draws: MessageWriter<crate::archery::DrawBow>,
) {
    let tick = sim.world.tick();
    for (e, h, k, mut c, mind, marching, keeps, swinging, wielding) in &mut q {
        let Some(mut m) = mind else {
            commands.entity(e).insert(HunterMind::default());
            continue;
        };
        let pos = k.body.pos;
        // (On a leash: only what's near its place.)
        let home = keeps.filter(|_| h.leash > 0.0).map(|kp| Vec2::new(kp.0.0 as f32, kp.0.1 as f32));
        let target = hunted
            .iter()
            .filter(|(_, t)| t.hunted())
            .map(|(pk, _)| (pk.body.pos, pk.body.vel))
            .filter(|(p, _)| p.distance(pos) < h.aggro && home.is_none_or(|hm| p.distance(hm) < h.leash))
            .min_by(|a, b| a.0.distance_squared(pos).total_cmp(&b.0.distance_squared(pos)));
        let contacts = k.loco.contacts;
        let grounded = k.loco.grounded();
        let stunned = k.loco.state == platypus_physics::MoveState::Stunned;
        c.0.aim = target.map_or(Vec2::ZERO, |t| t.0);
        let mut jump = false;
        let mut move_y = 0.0;
        let move_x: f32 = match (&h.close, target) {
            // On foot at it: up to `keep` off, its weapon's combo when near.
            (Close::Walk { keep, jump_to_reach }, Some((t, _))) => {
                let d = t - pos;
                jump = grounded && d.y > *jump_to_reach && d.x.abs() < h.aggro * 0.25;
                if let Attack::Swing { reach, every, combo } = h.attack {
                    let near = d.x.abs() < reach && d.y.abs() < reach;
                    if m.attacking && !swinging && m.swings_left == 0 {
                        m.attacking = false;
                        m.next = tick + ticks(every * (0.7 + 0.6 * unit(&sim, e, 0xA77)));
                    }
                    if near && !stunned && reach > 0.0 && tick >= m.next && !m.attacking && !swinging {
                        m.swings_left = combo.max(1);
                        m.attacking = true;
                    }
                    if m.swings_left > 0 && !stunned {
                        swings.write(crate::combat::MeleeRequest { attacker: e, at: t });
                        // (A request during a swing queues the next move: one each.)
                        if !swinging || combo > 1 {
                            m.swings_left -= 1;
                        }
                    }
                }
                if swinging || d.x.abs() <= *keep { 0.0 } else { d.x.signum() }
            }
            // On foot, at a distance: back off, close in, draw and loose.
            (Close::Range { near, far }, Some((t, tv))) => {
                let d = t - pos;
                let bow = match (&h.attack, &weapons) {
                    (Attack::Shoot { .. }, Some(w)) => wielding.and_then(|wd| wd.0.as_deref()).and_then(|id| w.bow_index(id)).map(|i| w.bow(i).clone()),
                    _ => None,
                };
                match (h.attack.clone(), bow, &weapons) {
                    (Attack::Shoot { draw, every, wobble }, Some(bow), Some(w)) => {
                        if let Some(since) = m.drawing {
                            // Drawing: keep at it, then let go.
                            let held = (tick - since) as f32 / TICKS;
                            if stunned || held >= bow.draw * draw {
                                m.drawing = None;
                                m.next = tick + ticks(every * (0.7 + 0.6 * unit(&sim, e, 0xB0E)));
                            } else {
                                // Where it'll be when the arrow gets there, and the drop.
                                let speed = bow.speed.0 + (bow.speed.1 - bow.speed.0) * draw.min(1.0);
                                let flight = d.length() / speed.max(1.0);
                                let g = w.arrow_def().map_or(0.0, |a| a.gravity);
                                let aim = t + tv * flight + Vec2::new(0.0, 0.5 * g * flight * flight);
                                // (A wobble per shot, not per tick.)
                                let mut rng = Rng::seeded(&[sim.world.seed(), since, e.to_bits(), 0xA1A]);
                                let off = ((rng.next_u32() % 2001) as f32 / 1000.0 - 1.0) * wobble.to_radians();
                                let aim = pos + Vec2::from_angle(off).rotate(aim - pos);
                                draws.write(crate::archery::DrawBow { archer: e, at: aim });
                            }
                            0.0
                        } else if !stunned && tick >= m.next && d.x.abs() <= far * 1.2 && grounded {
                            m.drawing = Some(tick);
                            draws.write(crate::archery::DrawBow { archer: e, at: t });
                            0.0
                        } else {
                            range(d, *near, *far)
                        }
                    }
                    _ => range(d, *near, *far),
                }
            }
            // Flying: a dive at it now and then, else hovering in its band.
            (Close::Swoop { hover, dive_time, dive_every }, Some((t, _))) => {
                let steer = if tick < m.dive_until {
                    (t - pos).normalize_or_zero()
                } else if tick >= m.next {
                    m.dive_until = tick + ticks(*dive_time);
                    m.next = m.dive_until + ticks(dive_every * (0.7 + 0.6 * unit(&sim, e, 7)));
                    (t - pos).normalize_or_zero()
                } else {
                    flit(&sim, e, tick, &mut m, pos, *hover, home, h.leash)
                };
                move_y = if steer.y == 0.0 { 0.15 } else { steer.y };
                steer.x
            }
            (Close::Swoop { hover, .. }, None) => {
                let steer = flit(&sim, e, tick, &mut m, pos, *hover, home, h.leash);
                move_y = if steer.y == 0.0 { 0.15 } else { steer.y };
                let go = march(marching, pos.x);
                if go != 0.0 { go } else { steer.x }
            }
            // Hopping: down, it waits; a hop at it every so often.
            (Close::Hop { every }, target) => {
                let mut mx = 0.0;
                if grounded && tick >= m.next {
                    let dir = match target {
                        Some((t, _)) => (t.x - pos.x).signum(),
                        // (Idle: a hop now and then, either way.)
                        None if unit(&sim, e, 4) < 0.3 => {
                            if unit(&sim, e, 5) < 0.5 {
                                -1.0
                            } else {
                                1.0
                            }
                        }
                        None => 0.0,
                    };
                    if dir != 0.0 {
                        jump = true;
                        mx = dir;
                    }
                    let every = if target.is_some() { *every } else { every * 2.5 };
                    m.next = tick + ticks(every * (0.7 + 0.6 * unit(&sim, e, 6)));
                }
                // (Airborne, it keeps going the way it hopped.)
                if grounded { mx } else { c.0.move_x }
            }
            // Crawling: up walls toward it, along ceilings; a pounce when
            // near, or dropping on it from above.
            (Close::Crawl { pounce_range, pounce_every }, Some((t, _))) => {
                let d = t - pos;
                move_y = if d.y > 6.0 {
                    1.0
                } else if d.y < -6.0 {
                    -1.0
                } else {
                    0.0
                };
                let clinging = k.loco.clinging();
                let over_you = clinging == Some(Vec2::Y) && d.x.abs() < 12.0;
                let near = d.length() < *pounce_range;
                if tick >= m.next && (over_you || (near && (grounded || clinging.is_some()))) {
                    jump = true;
                    m.next = tick + ticks(pounce_every * (0.7 + 0.6 * unit(&sim, e, 1)));
                }
                if d.x.abs() > 3.0 { d.x.signum() } else { 0.0 }
            }
            (Close::Crawl { .. }, None) => {
                if tick >= m.wander_until {
                    m.wander = Vec2::from_angle(unit(&sim, e, 2) * std::f32::consts::TAU);
                    m.wander_until = tick + ticks(1.0 + 2.0 * unit(&sim, e, 3));
                }
                move_y = m.wander.y.signum();
                m.wander.x.signum() * h.wander.speed
            }
            // On foot, no one about: on the march, or pottering.
            (Close::Walk { .. } | Close::Range { .. }, None) => {
                let go = march(marching, pos.x);
                if go != 0.0 {
                    go
                } else {
                    if tick >= m.wander_until {
                        let mut rng = Rng::seeded(&[sim.world.seed(), tick, e.to_bits()]);
                        m.wander.x = [-1.0, 0.0, 1.0][(rng.next_u32() % 3) as usize];
                        m.wander_until = tick + ticks(h.wander.every * (0.5 + (rng.next_u32() % 1000) as f32 / 1000.0));
                    }
                    m.wander.x * h.wander.speed
                }
            }
        };
        // On foot or hopping, a wall it's walking into: jump it.
        if matches!(h.close, Close::Walk { .. } | Close::Range { .. }) && ((move_x > 0.0 && contacts.wall_right) || (move_x < 0.0 && contacts.wall_left)) {
            jump = grounded;
        }
        c.0.move_x = move_x;
        c.0.move_y = move_y;
        // Brains hold buttons; release after a press so the next press registers.
        c.0.jump = jump && !c.0.jump;
    }
}

/// Keeping between `near` and `far` across.
fn range(d: Vec2, near: f32, far: f32) -> f32 {
    let dist = d.x.abs();
    if dist < near {
        -d.x.signum()
    } else if dist > far {
        d.x.signum()
    } else {
        0.0
    }
}

/// Flitting about in its band over the ground (and back toward its place,
/// strayed past half its leash).
#[allow(clippy::too_many_arguments)]
fn flit(sim: &SimWorld, e: Entity, tick: u64, m: &mut HunterMind, pos: Vec2, hover: (f32, f32), home: Option<Vec2>, leash: f32) -> Vec2 {
    if tick >= m.wander_until {
        m.wander = Vec2::from_angle(unit(sim, e, 8) * std::f32::consts::TAU);
        m.wander_until = tick + ticks(0.3 + 0.4 * unit(sim, e, 9));
    }
    let mut steer = m.wander;
    if let Some(h) = home
        && pos.distance(h) > leash * 0.5
    {
        steer.x = (h.x - pos.x).signum();
    }
    let h = crate::creatures::brain::critters::height(&sim.world, pos, (hover.1 as i32) * 2);
    if h < hover.0 {
        steer.y = steer.y.abs().max(0.7);
    } else if h > hover.1 {
        steer.y = -steer.y.abs().max(0.5);
    }
    steer
}
