//! Necromancers: twice your height, gaunt, floating a few cells over the
//! ground, a long hood over a grey skull-jawed face with its eyes alight,
//! bare bony arms, a cape and rags of cloth trailing under it where its legs
//! would be, charms swinging off its skull-crook. It keeps away from you
//! (its brain: `Range`; it floats: `fly`) and works from there:
//!
//! - **Wailing skulls**: the green gathers in its outstretched hand (the
//!   tell), then four spectral skulls fly out and steer for you, slow at
//!   first, curving round what's in the way; a blade bursts one.
//! - **Grave hands**: its hand turns down and a ring of green spreads on the
//!   ground under you (the tell); then rotting hands burst up there and
//!   grab whoever's in it (struggle free, or don't be there).
//! - **Soul drain**: its hand claws at you (the tell) and a tether crackles
//!   out to you, draining you and mending it, while it can see you and
//!   you're neither too near nor too far.
//! - **Raise**: its arms go up and motes rise where the dead will stand,
//!   then zombies: bodies lying near get up (what they had spills), others
//!   claw up out of the earth (soft ground) or heave up out of cracked rock.
//!   At most `MINIONS`; what it raised doesn't burn in the sun, stays down
//!   when killed, and is gone the moment it dies.
//! - **Blink**: come too close and it folds into a column of grave-smoke,
//!   a ring snapping out, a streak of souls to where it's back; its shade
//!   lingers a moment where it was.

use std::sync::Arc;

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind};
use serde::Deserialize;

use super::{CustomCreature, CustomSet, frame_anchor};
use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprites};
use crate::cloth::{ClothDef, Drapes, Links, draw_links};
use crate::creatures::body::animation::{Animator, CreatureSprite};
use crate::creatures::{Controls, Harm, Health, Kinematics, Team};
use crate::world::{ChunkLoader, SimWorld};

#[derive(Component, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Necro {
    #[serde(skip)]
    doing: Doing,
    /// Its waits: skulls, hands, drain, raising, blinking.
    #[serde(skip)]
    ready: [f32; 5],
    /// Where the dead will stand (a body, or the ground), while it raises.
    #[serde(skip)]
    raising: Vec<(Vec2, Option<Entity>)>,
    /// Its skulls in flight.
    #[serde(skip)]
    skulls: Vec<Skull>,
    /// Its grave hands: where, how long since, whom they hold.
    #[serde(skip)]
    hands: Option<(Vec2, f32, Option<Entity>)>,
    /// Its tether's far end (while it drains).
    #[serde(skip)]
    tether: Option<Vec2>,
    /// Rings spreading (where, how old, how far they go); a streak of souls
    /// (from, to, how old).
    #[serde(skip)]
    rings: Vec<(Vec2, f32, f32)>,
    #[serde(skip)]
    streak: Option<(Vec2, Vec2, f32)>,
    /// Charms off its crook.
    #[serde(skip)]
    charms: Vec<Links>,
    #[serde(skip)]
    palm: Vec2,
    #[serde(skip)]
    clock: f32,
    #[serde(skip)]
    started: bool,
}

#[derive(Clone, Copy, Debug)]
struct Skull {
    pos: Vec2,
    vel: Vec2,
    age: f32,
}

/// Raised by that necromancer: held up by it.
#[derive(Component, Clone, Copy, Debug)]
pub struct RaisedBy(pub Entity);

/// A necromancer's shade, where it was before it blinked: fading.
#[derive(Component)]
struct Shade(f32);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Doing {
    /// Floating, keeping its distance (its brain).
    #[default]
    Float,
    /// The green gathering (the skulls' tell); loosing them.
    Wail(f32, u32),
    /// Its hand turned down at the ground under you (the hands' tell).
    Beckon(f32),
    /// Clawing at you (the drain's tell), then draining whom.
    Drain(f32, Entity),
    /// Arms up, motes rising (the raise's tell).
    Raise(f32),
}

impl CustomCreature for Necro {
    const NAME: &'static str = "necro";

    fn build(app: &mut App) {
        app.init_resource::<NecroCanvas>()
            .add_observer(on_spawn)
            .add_systems(FixedUpdate, (think, orphans).chain().in_set(CustomSet::Think))
            .add_systems(Update, (draw, fade_shades));
    }
}

impl Necro {
    /// What it's doing, in a word (a scenario's log).
    pub fn doing(&self) -> &'static str {
        match self.doing {
            Doing::Float => "float",
            Doing::Wail(..) => "wail",
            Doing::Beckon(_) => "beckon",
            Doing::Drain(..) => "drain",
            Doing::Raise(_) => "raise",
        }
    }

    /// Its skulls in flight; whether its hands are up (a scenario's log).
    pub fn skulls(&self) -> usize {
        self.skulls.len()
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
const WAIL_I: usize = 0;
const HANDS_I: usize = 1;
const DRAIN_I: usize = 2;
const RAISE_I: usize = 3;
const BLINK_I: usize = 4;
/// Hunting what's this near (cells).
const AGGRO: f32 = 360.0;
/// Floating this high over the ground (its feet, cells), bobbing this much.
const HOVER: (f32, f32) = (10.0, 2.0);
/// After any of its works, nothing for at least this long.
const BREATHER: f32 = 0.7;
/// The skulls: gathering this long; so many, this far apart (s); their
/// speed from this to that (cells/s), how fast they turn (rad/s), how long
/// they fly; what one does; how near it must come; it looses them at what's
/// this near; then it waits.
const WAIL: f32 = 0.5;
const SKULLS: (u32, f32) = (4, 0.16);
const SKULL_SPEED: (f32, f32) = (70.0, 165.0);
const SKULL_TURN: f32 = 2.6;
const SKULL_LIFE: f32 = 4.5;
const SKULL_HIT: f32 = 9.0;
const SKULL_NEAR: f32 = 3.5;
const WAIL_RANGE: f32 = 320.0;
const WAIL_EVERY: f32 = 3.8;
/// The hands: its tell this long; the ring (cells across) spreading this
/// long before they burst up; they hold whom they catch this long; what
/// they do catching; it calls them at what's this near; then it waits.
const BECKON: f32 = 0.4;
const RING: f32 = 15.0;
const RISE: f32 = 0.75;
const GRASP: f32 = 1.7;
const GRASP_HIT: f32 = 10.0;
const HANDS_RANGE: f32 = 280.0;
const HANDS_EVERY: f32 = 6.5;
/// The drain: its tell this long; at most this long, every so often this
/// much taken (and mended); from this near to this far; then it waits.
const CLAW: f32 = 0.45;
const DRAIN: (f32, f32, f32) = (3.0, 0.25, 2.2);
const DRAIN_RANGE: (f32, f32) = (34.0, 250.0);
const DRAIN_EVERY: f32 = 10.0;
/// Raising: the tell this long; at most this many up at once; bodies this
/// near get up; the rest stand this far toward you from it (cells, each a
/// little apart), coming up this long apart; then it waits.
const RAISE: f32 = 1.1;
const MINIONS: usize = 3;
const BODIES_NEAR: f32 = 160.0;
const STAND: f32 = 45.0;
const APART: f32 = 0.45;
const RAISE_EVERY: f32 = 14.0;
/// Blinking: when you're this near; back this far from you (cells, on its
/// side of you if it can, else past you); then it waits.
const BLINK_NEAR: f32 = 52.0;
const BLINK_TO: (f32, f32) = (150.0, 200.0);
const BLINK_EVERY: f32 = 6.0;

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
        life: (0.5, 1.2),
        colors: vec![(40, 56, 46), (24, 30, 28), (90, 150, 104), (14, 16, 18)],
        speed: 50.0,
        spread: 0.6,
        gravity: -90.0,
        drag: 2.0,
        size: 2.4,
        jitter: 0.0,
        glow: false,
    }
}

/// Its cape, its hood's tail, the rags off its waist; its charms; it can
/// grab (`Moves`: its hands' hold).
fn on_spawn(add: On<Add, Necro>, mut commands: Commands) {
    let e = add.entity;
    let seed = e.to_bits();
    let mut drapes = Drapes::default();
    let rag = |color: (u8, u8, u8), cols, rows, pin, torn, front, behind, arc| Arc::new(ClothDef { color, lining: (40, 70, 50), cols, rows, gap: 2.4, pin, torn, weight: 0.8, front, behind, arc });
    // A long cape off its shoulders, torn, trailing below it.
    drapes.own(Arc::new(ClothDef { color: (30, 26, 38), lining: (36, 66, 46), cols: 7, rows: 24, gap: 2.5, pin: (-7.0, 3.0, 22.0), torn: 0.65, weight: 0.9, front: false, behind: 140.0, arc: (-20.0, 150.0) }), seed);
    // Its hood's tail.
    drapes.own(rag((34, 30, 44), 2, 7, (-5.0, -2.0, 31.0), 0.4, false, 60.0, (-10.0, 150.0)), seed ^ 1);
    // Rags off its waist, where its legs would be: long, ragged.
    for (n, (x0, x1, rows, front)) in [(-5.0, -1.0, 15, false), (-1.0, 3.0, 17, true), (2.0, 5.0, 13, true), (-4.0, 0.0, 12, true)].into_iter().enumerate() {
        drapes.own(rag((38, 32, 46), 2, rows, (x0, x1, 4.0), 0.7, front, 20.0, (-25.0, 140.0)), seed ^ (n as u64 + 7));
    }
    commands.entity(e).insert((drapes, crate::creatures::moves::Moves::new(Vec::new())));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn think(
    mut commands: Commands,
    sim: Res<SimWorld>,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Necro, &mut Kinematics, &mut Controls, &mut Animator, &mut Health, Option<&mut Drapes>, Option<&Children>)>,
    hunted: Query<(Entity, &Kinematics, &Team), Without<Necro>>,
    raised: Query<&RaisedBy>,
    bodies: Query<(Entity, &crate::hands::corpses::Corpse, &Kinematics), (Without<Necro>, Without<Team>)>,
    sprites: Query<(&Sprite, &GlobalTransform), With<CreatureSprite>>,
    (mut chests, items, blades): (ResMut<crate::hands::chests::Chests>, Option<Res<crate::hands::items::Items>>, Res<crate::combat::LiveBlades>),
    (mut hits, mut sounds): (MessageWriter<crate::combat::Hit>, MessageWriter<crate::sound::PlaySound>),
    (mut sparks, mut flares): (ResMut<crate::vfx::Sparks>, MessageWriter<crate::fx::Flare>),
) {
    let mote = motes();
    let now = time.elapsed_secs();
    for (e, mut o, mut k, mut c, mut anim, mut health, drapes, children) in &mut q {
        let (pos, half, facing) = (k.body.pos, k.body.half, k.loco.facing);
        if !o.started {
            o.started = true;
            o.ready = [1.5, 3.0, 5.0, 2.5, 0.0];
            o.charms = vec![Links::new(5, 2.2), Links::new(4, 2.2)];
        }
        o.clock += DT;
        for r in o.ready.iter_mut() {
            *r -= DT;
        }
        let target = hunted.iter().filter(|(_, _, t)| t.hunted()).map(|(e, tk, _)| (e, tk.body.pos, tk.body.half, tk.loco.grounded())).filter(|(_, p, ..)| p.distance(pos) < AGGRO).min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)));
        o.palm = frame_anchor(&anim, &k, "palm", "").unwrap_or(pos + Vec2::new(facing * 12.0, 8.0));
        let skull = frame_anchor(&anim, &k, "skull", "").unwrap_or(pos + Vec2::new(facing * 11.0, 26.0));
        let palm = o.palm;
        // Its charms, swinging off its crook.
        for (i, l) in o.charms.iter_mut().enumerate() {
            l.swing(skull + Vec2::new((i as f32 * 2.0 - 1.0) * 2.0, -1.0), 260.0);
        }
        let doing = o.doing;
        let mut next = doing;
        match doing {
            Doing::Float => {
                anim.force = None;
                if let Some((who, tp, _, grounded)) = target {
                    let dist = tp.distance(pos);
                    let up = raised.iter().filter(|r| r.0 == e).count();
                    if dist < BLINK_NEAR && o.ready[BLINK_I] <= 0.0 {
                        blink(&mut o, &mut k, drapes, children, &sprites, &mut commands, &sim, tp, &mut sparks, &mut flares, &mut sounds);
                    } else if o.ready[RAISE_I] <= 0.0 && up < MINIONS {
                        let room = MINIONS - up;
                        let mut spots: Vec<(Vec2, Option<Entity>)> = bodies.iter().filter(|(_, _, bk)| bk.body.pos.distance(pos) < BODIES_NEAR).take(room).map(|(be, _, bk)| (bk.body.pos, Some(be))).collect();
                        let toward = (tp.x - pos.x).signum();
                        for n in 0..room.saturating_sub(spots.len()) {
                            let x = pos.x + toward * (STAND + n as f32 * 20.0);
                            if let Some(g) = ground(&sim, Vec2::new(x, pos.y + 10.0), 90) {
                                spots.push((g, None));
                            }
                        }
                        if spots.is_empty() {
                            o.ready[RAISE_I] = 2.0;
                        } else {
                            o.raising = spots;
                            sounds.write(crate::sound::PlaySound::at("groan", pos).pitch(0.45).volume(1.0));
                            next = Doing::Raise(0.0);
                        }
                    } else if o.ready[DRAIN_I] <= 0.0 && (DRAIN_RANGE.0..DRAIN_RANGE.1).contains(&dist) && clear(&sim, palm, tp) {
                        sounds.write(crate::sound::PlaySound::at("zap", palm).pitch(0.4).volume(0.6));
                        next = Doing::Drain(0.0, who);
                    } else if o.ready[HANDS_I] <= 0.0 && o.hands.is_none() && grounded && dist < HANDS_RANGE {
                        sounds.write(crate::sound::PlaySound::at("groan", pos).pitch(0.7).volume(0.8));
                        next = Doing::Beckon(0.0);
                    } else if o.ready[WAIL_I] <= 0.0 && dist < WAIL_RANGE {
                        sounds.write(crate::sound::PlaySound::at("pour", palm).pitch(0.4).volume(0.8));
                        next = Doing::Wail(0.0, 0);
                    }
                }
            }
            Doing::Wail(t, n) => {
                anim.play("wail");
                if t < WAIL {
                    let f = t / WAIL;
                    for i in 0..(1 + (f * 4.0) as usize) {
                        let dir = Vec2::from_angle(t * 9.0 + i as f32 * 2.1);
                        sparks.emit(&mote, 1, palm + dir * (12.0 - f * 8.0), -dir, Vec2::ZERO);
                    }
                    next = Doing::Wail(t + DT, n);
                } else if t >= WAIL + n as f32 * SKULLS.1 {
                    // One more: up and out, fanned, to curve round to you.
                    let spread = (n as f32 - (SKULLS.0 - 1) as f32 / 2.0) * 0.45;
                    let up = Vec2::from_angle(55f32.to_radians() + spread);
                    o.skulls.push(Skull { pos: palm, vel: Vec2::new(up.x * facing.signum(), up.y) * SKULL_SPEED.0, age: 0.0 });
                    sounds.write(crate::sound::PlaySound::at("groan", palm).pitch(2.2 - n as f32 * 0.15).volume(0.6));
                    next = if n + 1 >= SKULLS.0 {
                        o.ready[WAIL_I] = WAIL_EVERY;
                        Doing::Float
                    } else {
                        Doing::Wail(t + DT, n + 1)
                    };
                } else {
                    next = Doing::Wail(t + DT, n);
                }
            }
            Doing::Beckon(t) => {
                anim.play("hands");
                if t < DT
                    && let Some((_, tp, th, _)) = target
                {
                    // The ring, under their feet.
                    let feet = tp - Vec2::Y * th.y;
                    let at = ground(&sim, feet + Vec2::Y * 4.0, 40).unwrap_or(feet);
                    o.hands = Some((at, 0.0, None));
                    sounds.write(crate::sound::PlaySound::at("pour", at).pitch(0.3).volume(0.8));
                }
                next = if t + DT >= BECKON {
                    o.ready[HANDS_I] = HANDS_EVERY;
                    Doing::Float
                } else {
                    Doing::Beckon(t + DT)
                };
            }
            Doing::Drain(t, who) => {
                anim.play("drain");
                let theirs = hunted.get(who).ok().map(|h| h.1.body.pos);
                let holds = theirs.is_some_and(|tp| {
                    let d = tp.distance(palm);
                    d > DRAIN_RANGE.0 * 0.7 && d < DRAIN_RANGE.1 * 1.1 && clear(&sim, palm, tp)
                });
                if t < CLAW {
                    if (t / 0.1) as u32 != ((t + DT) / 0.1) as u32 {
                        flares.write(crate::fx::Flare { at: palm, color: [0.3, 1.2, 0.5], life: 0.12 });
                    }
                    next = if holds { Doing::Drain(t + DT, who) } else { Doing::Float };
                    if !holds {
                        o.ready[DRAIN_I] = DRAIN_EVERY * 0.4;
                    }
                } else if !holds || t >= CLAW + DRAIN.0 {
                    o.tether = None;
                    o.ready[DRAIN_I] = DRAIN_EVERY;
                    sounds.write(crate::sound::PlaySound::at("zap", palm).pitch(1.4).volume(0.5));
                    next = Doing::Float;
                } else {
                    let tp = theirs.unwrap_or(palm);
                    o.tether = Some(tp);
                    // Every so often a little taken, and mended in it.
                    let beat = ((t - CLAW) / DRAIN.1) as u32 != ((t - CLAW + DT) / DRAIN.1) as u32;
                    if beat {
                        hits.write(crate::combat::Hit { target: who, damage: DRAIN.2, harm: Harm::Void, knock: Vec2::ZERO, stun: 0.0, at: tp, dir: (palm - tp).normalize_or(Vec2::Y), weight: 0.2, crit: false });
                        health.hp = (health.hp + DRAIN.2).min(health.max);
                        sounds.write(crate::sound::PlaySound::at("zap", tp).pitch(0.5).volume(0.35));
                    }
                    // Souls streaming from them to it.
                    let along = (palm - tp).normalize_or(Vec2::Y);
                    sparks.emit(&mote, 2, tp + along.perp() * ((now * 31.0).sin() * 3.0), along * 4.0, Vec2::ZERO);
                    next = Doing::Drain(t + DT, who);
                }
            }
            Doing::Raise(t) => {
                anim.play("raise");
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
                        o.rings.push((at, 0.0, 18.0));
                        if let Some(be) = body {
                            // A body gets up: what it had spills out.
                            if let (Ok((_, corpse, _)), Some(items)) = (bodies.get(be), items.as_deref()) {
                                chests.smash(&mut commands, &sim.world, items, corpse.key, at, false);
                            }
                            commands.entity(be).despawn();
                            sparks.emit(&mote, 30, at, Vec2::Y, Vec2::ZERO);
                            flares.write(crate::fx::Flare { at, color: [0.4, 1.6, 0.6], life: 0.4 });
                            crate::creatures::def::spawn_creature(&mut commands, "zombie", at, move |w| {
                                w.insert((RaisedBy(e), crate::creatures::body::emerge::SunProof));
                            });
                        } else {
                            let soft = sim.world.get(CellPos::from_world(at.x, at.y - 2.0)).is_some_and(|cell| !cell.is_air() && crate::creatures::body::emerge::soft(sim.world.materials().phys(cell.material)));
                            if soft {
                                // Out of the earth, clawing.
                                crate::creatures::def::spawn_creature(&mut commands, "zombie", at, move |w| {
                                    w.insert((RaisedBy(e), crate::creatures::body::emerge::SunProof, crate::creatures::body::emerge::Emerging::after(delay), crate::creatures::Dormant, Visibility::Hidden));
                                });
                            } else {
                                // Out of cracked rock, heaving up in a glow.
                                sparks.emit(&mote, 40, at + Vec2::Y * 8.0, Vec2::Y, Vec2::ZERO);
                                sparks.emit(&smoke(), 20, at, Vec2::Y, Vec2::ZERO);
                                flares.write(crate::fx::Flare { at: at + Vec2::Y * 10.0, color: [0.5, 1.8, 0.7], life: 0.5 });
                                crate::creatures::def::spawn_creature(&mut commands, "zombie", at, move |w| {
                                    w.insert((RaisedBy(e), crate::creatures::body::emerge::SunProof));
                                });
                            }
                        }
                    }
                    sounds.write(crate::sound::PlaySound::at("groan", pos).pitch(0.4).volume(1.0));
                    o.ready[RAISE_I] = RAISE_EVERY;
                    Doing::Float
                } else {
                    Doing::Raise(t + DT)
                };
            }
        }
        // A work done: a breather before the next.
        if next == Doing::Float && doing != Doing::Float {
            for r in o.ready[..BLINK_I].iter_mut() {
                *r = r.max(BREATHER);
            }
        }
        o.doing = next;
        // Floating: held over the ground, bobbing; still while it works.
        let feet = pos.y - half.y;
        let over = ground(&sim, Vec2::new(pos.x, feet + 2.0), 90).map(|g| feet - g.y);
        let want = HOVER.0 + (o.clock * 1.7).sin() * HOVER.1;
        c.0.move_y = over.map_or(-0.6, |h| ((want - h) * 0.12).clamp(-1.0, 1.0));
        c.0.jump = false;
        if !matches!(next, Doing::Float) {
            c.0.move_x = 0.0;
        }

        // Its skulls: steering for you, round what's in the way; bursting
        // on you, on a blade, on rock, or done.
        let aim = target.map(|(_, tp, ..)| tp);
        let mut gone = Vec::new();
        for (i, s) in o.skulls.iter_mut().enumerate() {
            s.age += DT;
            let speed = SKULL_SPEED.0 + (SKULL_SPEED.1 - SKULL_SPEED.0) * (s.age / 1.5).min(1.0);
            let mut want = aim.map_or(s.vel, |a| (a - s.pos).normalize_or(s.vel.normalize_or(Vec2::X)) * speed);
            // (Something in the way ahead: turn off it, the open way.)
            let ahead = s.pos + s.vel.normalize_or(Vec2::X) * 8.0;
            if solid(&sim, ahead) {
                let side = s.vel.perp().normalize_or(Vec2::Y);
                want = if solid(&sim, s.pos + side * 8.0) { -side * speed } else { side * speed };
            }
            let turn = (want.to_angle() - s.vel.to_angle() + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            let a = s.vel.to_angle() + turn.clamp(-SKULL_TURN * DT, SKULL_TURN * DT);
            s.vel = Vec2::from_angle(a) * speed;
            let from = s.pos;
            s.pos += s.vel * DT;
            sparks.emit(&mote, 1, s.pos, -s.vel.normalize_or(Vec2::X), Vec2::ZERO);
            let struck = hunted.iter().filter(|(_, _, t)| t.hunted()).find(|(_, tk, _)| ((tk.body.pos - s.pos).abs() - tk.body.half).max(Vec2::ZERO).length() <= SKULL_NEAR);
            let cut = blades.0.iter().filter(|b| b.who != e).any(|b| b.cells.iter().any(|cell| cell.distance(s.pos) <= 4.0 || cell.distance(from) <= 4.0));
            if let Some((who, tk, _)) = struck {
                let dir = s.vel.normalize_or(Vec2::X);
                hits.write(crate::combat::Hit { target: who, damage: SKULL_HIT, harm: Harm::Void, knock: dir * 160.0, stun: 0.15, at: s.pos, dir, weight: 0.8, crit: false });
                let _ = tk;
                gone.push(i);
            } else if cut || solid(&sim, s.pos) || s.age > SKULL_LIFE {
                gone.push(i);
            }
        }
        for &i in gone.iter().rev() {
            let s = o.skulls.remove(i);
            sparks.emit(&mote, 18, s.pos, Vec2::Y, Vec2::ZERO);
            sounds.write(crate::sound::PlaySound::at("groan", s.pos).pitch(2.6).volume(0.4));
        }

        // Its hands: the ring spreading, then up out of the ground, holding
        // whoever's in it.
        if let Some((at, t, held)) = o.hands {
            let t = t + DT;
            if t < RISE {
                if (t * 20.0) as u32 % 2 == 0 {
                    let x = (t * 53.0).sin() * RING;
                    sparks.emit(&mote, 1, at + Vec2::new(x, 1.0), Vec2::Y, Vec2::ZERO);
                }
                o.hands = Some((at, t, held));
            } else if t - DT < RISE {
                // Up: whoever's feet are in the ring, grabbed.
                sounds.write(crate::sound::PlaySound::at("groan", at).pitch(0.6).volume(1.0));
                sounds.write(crate::sound::PlaySound::at("break", at).pitch(0.6).volume(0.8));
                sparks.emit(&smoke(), 16, at, Vec2::Y, Vec2::ZERO);
                let caught = hunted.iter().filter(|(_, _, tm)| tm.hunted()).find(|(_, tk, _)| {
                    let feet = tk.body.pos - Vec2::Y * tk.body.half.y;
                    (feet.x - at.x).abs() <= RING + tk.body.half.x && (feet.y - at.y).abs() <= 8.0
                });
                let held = caught.map(|(who, tk, _)| {
                    commands.entity(who).insert(crate::creatures::moves::Held { by: e, at: tk.body.pos });
                    hits.write(crate::combat::Hit { target: who, damage: GRASP_HIT, harm: Harm::Pierce, knock: Vec2::ZERO, stun: 0.2, at: tk.body.pos, dir: Vec2::Y, weight: 0.6, crit: false });
                    who
                });
                o.hands = Some((at, t, held));
            } else if t >= RISE + GRASP {
                if let Some(who) = held
                    && let Ok(mut ec) = commands.get_entity(who)
                {
                    ec.remove::<crate::creatures::moves::Held>();
                }
                o.hands = None;
            } else {
                o.hands = Some((at, t, held));
            }
        }

        // Rings spreading, its streak of souls fading.
        for r in o.rings.iter_mut() {
            r.1 += DT;
        }
        o.rings.retain(|r| r.1 < 0.6);
        if let Some((a, b, t)) = o.streak {
            o.streak = (t + DT < 0.5).then_some((a, b, t + DT));
        }
    }
}

/// Gone in a column of grave-smoke (its shade left a moment), back further
/// off from `from`: on its side of them if it can, else past them.
#[allow(clippy::too_many_arguments)]
fn blink(
    o: &mut Necro,
    k: &mut Kinematics,
    drapes: Option<Mut<Drapes>>,
    children: Option<&Children>,
    sprites: &Query<(&Sprite, &GlobalTransform), With<CreatureSprite>>,
    commands: &mut Commands,
    sim: &SimWorld,
    from: Vec2,
    sparks: &mut crate::vfx::Sparks,
    flares: &mut MessageWriter<crate::fx::Flare>,
    sounds: &mut MessageWriter<crate::sound::PlaySound>,
) {
    let (pos, half) = (k.body.pos, k.body.half);
    let away = (pos.x - from.x).signum();
    let mut to = None;
    'find: for side in [away, -away] {
        for d in [BLINK_TO.1, (BLINK_TO.0 + BLINK_TO.1) / 2.0, BLINK_TO.0] {
            let probe = Vec2::new(from.x + side * d, pos.y + 40.0);
            if let Some(g) = ground(sim, probe, 140)
                && let Some(spot) = crate::creatures::free_spot(&sim.world, g + Vec2::Y * (half.y + HOVER.0), half, 8)
            {
                to = Some(spot);
                break 'find;
            }
        }
    }
    o.ready[BLINK_I] = BLINK_EVERY;
    let Some(spot) = to else { return };
    // Its shade, where it was: its picture, pale and fading.
    for c in children.into_iter().flat_map(|ch| ch.iter()) {
        if let Ok((s, gt)) = sprites.get(c) {
            let mut shade = s.clone();
            shade.color = Color::srgba(0.55, 1.0, 0.65, 0.55);
            commands.spawn((shade, gt.compute_transform().with_translation(gt.translation() - Vec3::Z * 0.05), Shade(1.0)));
        }
    }
    sparks.emit(&smoke(), 70, pos - Vec2::Y * half.y, Vec2::Y, Vec2::ZERO);
    sparks.emit(&motes(), 30, pos, Vec2::Y, Vec2::ZERO);
    flares.write(crate::fx::Flare { at: pos, color: [0.3, 1.0, 0.45], life: 0.35 });
    o.rings.push((pos, 0.0, 30.0));
    o.streak = Some((pos, spot, 0.0));
    // Moved, its cloth with it (not dragged across).
    if let Some(mut d) = drapes {
        d.shift(spot - pos);
    }
    k.body.pos = spot;
    k.prev_pos = spot;
    k.body.vel = Vec2::ZERO;
    // Back: up out of a pool of shadow.
    sparks.emit(&smoke(), 50, spot - Vec2::Y * (half.y + HOVER.0), Vec2::Y, Vec2::ZERO);
    sparks.emit(&motes(), 30, spot - Vec2::Y * half.y, Vec2::Y, Vec2::ZERO);
    o.rings.push((spot - Vec2::Y * (half.y + HOVER.0 - 1.0), 0.0, 22.0));
    sounds.write(crate::sound::PlaySound::at("cast", pos).pitch(1.6).volume(0.9));
    sounds.write(crate::sound::PlaySound::at("groan", spot).pitch(1.8).volume(0.6));
}

/// Its shades fade.
fn fade_shades(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Shade, &mut Sprite)>) {
    for (e, mut s, mut sprite) in &mut q {
        s.0 -= time.delta_secs();
        if s.0 <= 0.0 {
            commands.entity(e).despawn();
        } else {
            sprite.color = Color::srgba(0.55, 1.0, 0.65, 0.55 * s.0);
        }
    }
}

/// What a necromancer raised stays down when killed (no rising), and is
/// gone, crumbling, the moment the necromancer is.
#[allow(clippy::type_complexity)]
fn orphans(
    mut commands: Commands,
    raised: Query<(Entity, &RaisedBy, &Kinematics, Has<crate::creatures::body::rise::Rises>)>,
    necros: Query<(), With<Necro>>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    for (e, r, k, rises) in &raised {
        if necros.get(r.0).is_err() {
            sparks.emit(&motes(), 30, k.body.pos, Vec2::Y, Vec2::ZERO);
            sparks.emit(&smoke(), 24, k.body.pos, Vec2::Y, Vec2::ZERO);
            sounds.write(crate::sound::PlaySound::at("groan", k.body.pos).pitch(0.8).volume(0.7));
            commands.entity(e).despawn();
        } else if rises {
            commands.entity(e).remove::<crate::creatures::body::rise::Rises>();
        }
    }
}

/// Its skulls, hands, tether, rings and charms.
#[derive(Resource)]
pub struct NecroCanvas(Canvas);

impl Default for NecroCanvas {
    fn default() -> Self {
        NecroCanvas(Canvas::new("Necromancy", 11.3))
    }
}

/// A skull's look (5 × 5, from its middle, facing right): bone, sockets.
const SKULL_PX: [&str; 5] = [".bbb.", "bBBBb", "bkBkb", "bBBBb", ".b.b."];

#[allow(clippy::too_many_arguments)]
fn draw(mut commands: Commands, mut canvas: ResMut<NecroCanvas>, mut images: ResMut<Assets<Image>>, camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>, q: Query<&Necro>, time: Res<Time>, mut sprites: CanvasSprites) {
    let Ok((cam, loader)) = camera.single() else { return };
    let any = q.iter().any(|o| o.started);
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, any) else { return };
    let now = time.elapsed_secs();
    let glow = [150, 255, 165, 255];
    let dim = [70, 160, 90, 255];
    for o in q.iter().filter(|o| o.started) {
        for l in &o.charms {
            draw_links(&mut px, l.points(), [214, 206, 178, 255], [120, 112, 96, 255]);
        }
        // The skulls: a green haze, bone, dark sockets.
        for s in &o.skulls {
            let c = s.pos.floor().as_ivec2();
            let flip = s.vel.x < 0.0;
            for (dy, row) in SKULL_PX.iter().enumerate() {
                for (dx, ch) in row.chars().enumerate() {
                    let x = if flip { 2 - dx as i32 } else { dx as i32 - 2 };
                    let col = match ch {
                        'b' => [200, 230, 205, 255],
                        'B' => [236, 255, 240, 255],
                        'k' => [20, 60, 30, 255],
                        _ => continue,
                    };
                    px.put(c.x + x, c.y + 2 - dy as i32, col);
                }
            }
            for i in 0..6 {
                let p = (s.pos - s.vel.normalize_or(Vec2::X) * (3.0 + i as f32 * 1.5) + Vec2::new(0.0, ((now * 20.0 + i as f32).sin()) * 0.8)).floor().as_ivec2();
                px.put(p.x, p.y, if i < 3 { glow } else { dim });
            }
        }
        // The hands: a ring of green on the ground, spreading; then up.
        if let Some((at, t, _)) = o.hands {
            if t < RISE {
                let r = RING * (t / RISE).min(1.0);
                for i in 0..48 {
                    let a = i as f32 / 48.0 * std::f32::consts::TAU + now * 2.0;
                    let p = (at + Vec2::new(a.cos() * r, a.sin() * 1.6 + 0.5)).floor().as_ivec2();
                    px.put(p.x, p.y, if i % 3 == 0 { glow } else { dim });
                }
            } else {
                // Rotting hands up out of the earth, clawing.
                let up = ((t - RISE) / 0.15).min(1.0) * (1.0 - ((t - RISE - GRASP + 0.3) / 0.3).clamp(0.0, 1.0));
                for n in 0..5 {
                    let x = at.x - RING * 0.8 + n as f32 * RING * 0.4;
                    let tall = (7.0 + (n * 3 % 4) as f32) * up;
                    let sway = ((now * 6.0 + n as f32).sin()) * 1.2;
                    for h in 0..tall as i32 {
                        let p = Vec2::new(x + sway * h as f32 / tall.max(1.0), at.y + h as f32).floor().as_ivec2();
                        px.put(p.x, p.y, [104, 112, 92, 255]);
                        px.put(p.x + 1, p.y, [78, 84, 70, 255]);
                    }
                    if tall > 3.0 {
                        let top = Vec2::new(x + sway, at.y + tall).floor().as_ivec2();
                        for (fx, fy) in [(-1, 1), (0, 2), (1, 2), (2, 1)] {
                            px.put(top.x + fx, top.y + fy, [130, 138, 116, 255]);
                        }
                    }
                }
            }
        }
        // The tether: a crackling line, new every few frames.
        if let Some(tp) = o.tether {
            let (a, b) = (o.palm, tp);
            let seed = (now * 24.0) as u32;
            let n = (a.distance(b) / 4.0).ceil().max(2.0) as u32;
            let side = (b - a).perp().normalize_or(Vec2::Y);
            let mut last = a;
            for i in 1..=n {
                let f = i as f32 / n as f32;
                let jag = if i == n { 0.0 } else { (((seed.wrapping_mul(2654435761).wrapping_add(i * 977)) % 7) as f32 - 3.0) * 1.2 };
                let p = a.lerp(b, f) + side * jag;
                let m = (p.distance(last) * 1.5).ceil() as i32;
                for s in 0..=m {
                    let q = last.lerp(p, s as f32 / m.max(1) as f32).floor().as_ivec2();
                    px.put(q.x, q.y, glow);
                    px.put(q.x, q.y + 1, dim);
                }
                last = p;
            }
        }
        // Rings snapping out (a blink, a raising).
        for &(at, t, reach) in &o.rings {
            let r = reach * (t / 0.6).sqrt();
            for i in 0..56 {
                let a = i as f32 / 56.0 * std::f32::consts::TAU;
                let p = (at + Vec2::from_angle(a) * r).floor().as_ivec2();
                px.put(p.x, p.y, if t < 0.3 { glow } else { dim });
            }
        }
        // The streak of souls from where it was to where it is.
        if let Some((a, b, t)) = o.streak {
            let n = (a.distance(b) / 3.0) as i32;
            for i in 0..n {
                let f = i as f32 / n.max(1) as f32;
                if (f - t * 2.0).abs() < 0.35 || ((i as f32 * 7.3 + now * 40.0).sin() > 0.6 && t < 0.3) {
                    let p = (a.lerp(b, f) + Vec2::Y * ((f * 12.0 + now * 9.0).sin() * 3.0)).floor().as_ivec2();
                    px.put(p.x, p.y, glow);
                }
            }
        }
    }
}
