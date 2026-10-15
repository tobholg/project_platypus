//! Necromancers: hunched, hooded, robed, a horned skull on their staff and
//! nothing in the hood but two green eyes. They keep away from you (their
//! brain: `Range`) and work at a distance:
//!
//! - **Bolts**: the green gathers in its palm (the tell), then three grave
//!   bolts, one after another, each aimed at you anew.
//! - **Raise**: its arms go up and green motes rise where the dead will
//!   stand (the tell), then they stand: bodies lying near get up as
//!   skeletons (what they had spills), and skeletons claw up out of the
//!   earth (soft ground) or pull themselves together out of the rock in a
//!   green flash. At most `MINIONS` at a time.
//! - **Blink**: come too close and it's gone in a puff of grave-smoke, and
//!   back further off.
//!
//! What it raised is held up by it: kill it, and they fall apart for good.

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind};
use serde::Deserialize;

use super::{CustomCreature, CustomSet, frame_anchor};
use crate::creatures::body::animation::Animator;
use crate::creatures::{Controls, Harm, Kinematics, Team};
use crate::world::SimWorld;

#[derive(Component, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Necro {
    #[serde(skip)]
    doing: Doing,
    /// Its waits: bolts, raising, blinking.
    #[serde(skip)]
    ready: [f32; 3],
    /// Where the dead will stand (a body, or the ground), while it raises.
    #[serde(skip)]
    raising: Vec<(Vec2, Option<Entity>)>,
    #[serde(skip)]
    started: bool,
}

/// Raised by that necromancer: held up by it.
#[derive(Component, Clone, Copy, Debug)]
pub struct RaisedBy(pub Entity);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Doing {
    /// Keeping its distance (its brain).
    #[default]
    Keep,
    /// The green gathering in its palm (the bolts' tell); the bolts.
    Gather(f32),
    Volley(f32, u32),
    /// Arms up, motes rising (the raise's tell).
    Raise(f32),
    /// Gone in a puff, about to be back.
    Blink(f32),
}

impl CustomCreature for Necro {
    const NAME: &'static str = "necro";

    fn build(app: &mut App) {
        app.add_systems(FixedUpdate, (think, orphans).chain().in_set(CustomSet::Think));
    }
}

impl Necro {
    /// What it's doing, in a word (a scenario's log).
    pub fn doing(&self) -> &'static str {
        match self.doing {
            Doing::Keep => "keep",
            Doing::Gather(_) => "gather",
            Doing::Volley(..) => "volley",
            Doing::Raise(_) => "raise",
            Doing::Blink(_) => "blink",
        }
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
const BOLT_I: usize = 0;
const RAISE_I: usize = 1;
const BLINK_I: usize = 2;
/// Hunting what's this near (cells).
const AGGRO: f32 = 340.0;
/// The bolts: gathering this long; three, this far apart (s); at what's
/// from this near to this far; then it waits.
const GATHER: f32 = 0.55;
const VOLLEY: (u32, f32) = (3, 0.24);
const BOLT_RANGE: (f32, f32) = (40.0, 300.0);
const BOLT_EVERY: f32 = 3.2;
/// Raising: the tell this long; at most this many up at once; bodies this
/// near get up; the rest stand this far toward you from it (cells, each a
/// little apart), coming up this long apart; then it waits.
const RAISE: f32 = 1.1;
const MINIONS: usize = 3;
const BODIES_NEAR: f32 = 150.0;
const STAND: f32 = 45.0;
const APART: f32 = 0.45;
const RAISE_EVERY: f32 = 11.0;
/// Blinking: when you're this near; gone this long; back this far from you
/// (cells, on the far side of it if it can, else past you); then it waits.
const BLINK_NEAR: f32 = 48.0;
const BLINK: f32 = 0.35;
const BLINK_TO: (f32, f32) = (140.0, 190.0);
const BLINK_EVERY: f32 = 6.5;

fn solid(sim: &SimWorld, p: Vec2) -> bool {
    sim.world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| !c.is_air() && matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder))
}

fn clear(sim: &SimWorld, a: Vec2, b: Vec2) -> bool {
    let n = (a.distance(b) / 2.0).ceil() as i32;
    (1..n).all(|i| !solid(sim, a.lerp(b, i as f32 / n as f32)))
}

/// The ground under `at` (its top), within `reach` cells down.
fn ground(sim: &SimWorld, at: Vec2, reach: i32) -> Option<Vec2> {
    (0..reach).map(|n| at - Vec2::Y * n as f32).find(|&p| solid(sim, p - Vec2::Y)).map(|p| Vec2::new(p.x, p.y.floor()))
}

fn motes() -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter {
        count: 0.0,
        life: (0.4, 0.9),
        colors: vec![(210, 255, 220), (120, 230, 140), (50, 120, 70)],
        speed: 40.0,
        spread: 0.5,
        gravity: -60.0,
        drag: 1.0,
        size: 1.2,
        jitter: 0.0,
        glow: true,
    }
}

fn smoke() -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter {
        count: 0.0,
        life: (0.4, 1.0),
        colors: vec![(90, 120, 96), (60, 80, 64), (140, 200, 150)],
        speed: 90.0,
        spread: 3.14,
        gravity: -30.0,
        drag: 2.5,
        size: 2.0,
        jitter: 0.0,
        glow: false,
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn think(
    mut commands: Commands,
    sim: Res<SimWorld>,
    book: Res<crate::magic::Spellbook>,
    mut q: Query<(Entity, &mut Necro, &mut Kinematics, &mut Controls, &mut Animator)>,
    hunted: Query<(Entity, &Kinematics, &Team), Without<Necro>>,
    raised: Query<&RaisedBy>,
    bodies: Query<(Entity, &crate::hands::corpses::Corpse, &Kinematics), (Without<Necro>, Without<Team>)>,
    (mut chests, items): (ResMut<crate::hands::chests::Chests>, Option<Res<crate::hands::items::Items>>),
    (mut casts, mut sounds): (MessageWriter<crate::magic::CastRequest>, MessageWriter<crate::sound::PlaySound>),
    (mut sparks, mut flares): (ResMut<crate::vfx::Sparks>, MessageWriter<crate::fx::Flare>),
) {
    let mote = motes();
    for (e, mut o, mut k, mut c, mut anim) in &mut q {
        let (pos, half, facing) = (k.body.pos, k.body.half, k.loco.facing);
        if !o.started {
            o.started = true;
            o.ready = [1.5, 2.5, 0.0];
        }
        for r in o.ready.iter_mut() {
            *r -= DT;
        }
        let target = hunted.iter().filter(|(_, _, t)| t.hunted()).map(|(e, tk, _)| (e, tk.body.pos, tk.body.vel)).filter(|(_, p, _)| p.distance(pos) < AGGRO).min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)));
        let palm = frame_anchor(&anim, &k, "palm", "").unwrap_or(pos + Vec2::new(facing * 10.0, 6.0));
        let skull = frame_anchor(&anim, &k, "skull", "").unwrap_or(pos + Vec2::new(facing * 9.0, 18.0));
        let doing = o.doing;
        let mut next = doing;
        match doing {
            Doing::Keep => {
                anim.force = None;
                if let Some((_, tp, _)) = target {
                    let dist = tp.distance(pos);
                    let up = raised.iter().filter(|r| r.0 == e).count();
                    if dist < BLINK_NEAR && o.ready[BLINK_I] <= 0.0 {
                        sounds.write(crate::sound::PlaySound::at("cast", pos).pitch(1.6).volume(0.9));
                        sparks.emit(&smoke(), 50, pos, Vec2::Y, Vec2::ZERO);
                        next = Doing::Blink(0.0);
                    } else if o.ready[RAISE_I] <= 0.0 && up < MINIONS && k.loco.grounded() {
                        // Where they'll stand: the bodies near first, then
                        // the ground toward you, a little apart.
                        let room = MINIONS - up;
                        let mut spots: Vec<(Vec2, Option<Entity>)> = bodies.iter().filter(|(_, _, bk)| bk.body.pos.distance(pos) < BODIES_NEAR).take(room).map(|(be, _, bk)| (bk.body.pos, Some(be))).collect();
                        let toward = (tp.x - pos.x).signum();
                        for n in 0..room.saturating_sub(spots.len()) {
                            let x = pos.x + toward * (STAND + n as f32 * 18.0);
                            if let Some(g) = ground(&sim, Vec2::new(x, pos.y + 20.0), 60) {
                                spots.push((g, None));
                            }
                        }
                        if !spots.is_empty() {
                            o.raising = spots;
                            sounds.write(crate::sound::PlaySound::at("groan", pos).pitch(0.5).volume(1.0));
                            next = Doing::Raise(0.0);
                        } else {
                            o.ready[RAISE_I] = 2.0;
                        }
                    } else if (BOLT_RANGE.0..BOLT_RANGE.1).contains(&dist) && o.ready[BOLT_I] <= 0.0 && clear(&sim, palm, tp) {
                        sounds.write(crate::sound::PlaySound::at("pour", palm).pitch(0.45).volume(0.8));
                        next = Doing::Gather(0.0);
                    }
                }
            }
            Doing::Gather(t) => {
                anim.play("cast");
                let f = t / GATHER;
                for i in 0..(1 + (f * 4.0) as usize) {
                    let dir = Vec2::from_angle(t * 9.0 + i as f32 * 2.1);
                    sparks.emit(&mote, 1, palm + dir * (12.0 - f * 8.0), -dir, Vec2::ZERO);
                }
                if ((t + DT) / 0.12) as u32 != (t / 0.12) as u32 {
                    flares.write(crate::fx::Flare { at: palm, color: [0.4, 1.2 * f + 0.3, 0.5], life: 0.15 });
                }
                next = if t + DT >= GATHER { Doing::Volley(0.0, 0) } else { Doing::Gather(t + DT) };
            }
            Doing::Volley(t, n) => {
                anim.play("cast");
                next = if t <= 0.0 {
                    // A bolt at where they'll be, a little.
                    if let (Some((_, tp, tv)), Some(i)) = (target, book.spells.iter().position(|s| s.id == "grave_bolt")) {
                        let at = tp + tv * (tp.distance(palm) / 300.0) * 0.5;
                        casts.write(crate::magic::CastRequest { caster: e, spell: i, from: palm, toward: at, alt: false });
                        sounds.write(crate::sound::PlaySound::at("cast", palm).pitch(0.55 + n as f32 * 0.08).volume(0.8));
                        flares.write(crate::fx::Flare { at: palm, color: [0.5, 1.6, 0.7], life: 0.2 });
                    }
                    if n + 1 >= VOLLEY.0 {
                        o.ready[BOLT_I] = BOLT_EVERY;
                        Doing::Keep
                    } else {
                        Doing::Volley(VOLLEY.1, n + 1)
                    }
                } else {
                    Doing::Volley(t - DT, n)
                };
            }
            Doing::Raise(t) => {
                anim.play("raise");
                // Motes rising where they'll stand; its skull aglow.
                for (at, _) in &o.raising {
                    let x = (t * 37.0 + at.x).sin() * 5.0;
                    sparks.emit(&mote, 1, *at + Vec2::new(x, 1.0), Vec2::Y, Vec2::ZERO);
                }
                if ((t + DT) / 0.15) as u32 != (t / 0.15) as u32 {
                    flares.write(crate::fx::Flare { at: skull, color: [0.4, 1.4, 0.6], life: 0.2 });
                }
                next = if t + DT >= RAISE {
                    let spots = std::mem::take(&mut o.raising);
                    for (n, (at, body)) in spots.into_iter().enumerate() {
                        let delay = n as f32 * APART;
                        if let Some(be) = body {
                            // A body gets up: what it had spills out.
                            if let (Ok((_, corpse, _)), Some(items)) = (bodies.get(be), items.as_deref()) {
                                chests.smash(&mut commands, &sim.world, items, corpse.key, at, false);
                            }
                            commands.entity(be).despawn();
                            sparks.emit(&mote, 30, at, Vec2::Y, Vec2::ZERO);
                            flares.write(crate::fx::Flare { at, color: [0.4, 1.6, 0.6], life: 0.4 });
                            crate::creatures::def::spawn_creature(&mut commands, "skeleton", at, move |w| {
                                w.insert(RaisedBy(e));
                            });
                        } else {
                            let soft = sim.world.get(CellPos::from_world(at.x, at.y - 2.0)).is_some_and(|cell| !cell.is_air() && crate::creatures::body::emerge::soft(sim.world.materials().phys(cell.material)));
                            if soft {
                                // Out of the earth, clawing.
                                crate::creatures::def::spawn_creature(&mut commands, "skeleton", at, move |w| {
                                    w.insert((RaisedBy(e), crate::creatures::body::emerge::Emerging::after(delay), crate::creatures::Dormant, Visibility::Hidden));
                                });
                            } else {
                                // Out of the rock: pulled together in a flash.
                                sparks.emit(&mote, 40, at + Vec2::Y * 10.0, Vec2::Y, Vec2::ZERO);
                                flares.write(crate::fx::Flare { at: at + Vec2::Y * 10.0, color: [0.5, 1.8, 0.7], life: 0.5 });
                                crate::creatures::def::spawn_creature(&mut commands, "skeleton", at, move |w| {
                                    w.insert(RaisedBy(e));
                                });
                            }
                        }
                    }
                    sounds.write(crate::sound::PlaySound::at("groan", pos).pitch(0.4).volume(1.0));
                    o.ready[RAISE_I] = RAISE_EVERY;
                    Doing::Keep
                } else {
                    Doing::Raise(t + DT)
                };
            }
            Doing::Blink(t) => {
                anim.play("hurt");
                next = if t + DT >= BLINK {
                    // Back, further off: on its side of you if it can stand
                    // there, else past you.
                    let away = target.map_or(-facing, |(_, tp, _)| (pos.x - tp.x).signum());
                    let from = target.map_or(pos, |(_, tp, _)| tp);
                    let mut to = None;
                    for side in [away, -away] {
                        for d in [BLINK_TO.1, (BLINK_TO.0 + BLINK_TO.1) / 2.0, BLINK_TO.0] {
                            let probe = Vec2::new(from.x + side * d, pos.y + 30.0);
                            if let Some(g) = ground(&sim, probe, 120)
                                && let Some(spot) = crate::creatures::free_spot(&sim.world, g + Vec2::Y * (half.y + 1.0), half, 6)
                            {
                                to = Some(spot);
                                break;
                            }
                        }
                        if to.is_some() {
                            break;
                        }
                    }
                    if let Some(spot) = to {
                        k.body.pos = spot;
                        k.prev_pos = spot;
                        k.body.vel = Vec2::ZERO;
                        sparks.emit(&smoke(), 50, spot, Vec2::Y, Vec2::ZERO);
                        sounds.write(crate::sound::PlaySound::at("cast", spot).pitch(1.3).volume(0.9));
                    }
                    o.ready[BLINK_I] = BLINK_EVERY;
                    Doing::Keep
                } else {
                    Doing::Blink(t + DT)
                };
            }
        }
        o.doing = next;
        if !matches!(next, Doing::Keep) {
            c.0.move_x = 0.0;
            c.0.jump = false;
        }
    }
}

/// What a necromancer raised falls apart, for good, when it's gone.
fn orphans(raised: Query<(Entity, &RaisedBy, &Kinematics)>, necros: Query<(), With<Necro>>, mut hits: MessageWriter<crate::combat::Hit>, mut sparks: ResMut<crate::vfx::Sparks>) {
    for (e, r, k) in &raised {
        if necros.get(r.0).is_err() {
            hits.write(crate::combat::Hit { target: e, damage: 9_999.0, harm: Harm::Blunt, knock: Vec2::ZERO, stun: 0.0, at: k.body.pos, dir: Vec2::Y, weight: 1.0, crit: false });
            sparks.emit(&motes(), 20, k.body.pos, Vec2::Y, Vec2::ZERO);
        }
    }
}
