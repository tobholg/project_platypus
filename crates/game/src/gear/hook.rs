//! Grappling hooks (DESIGN §7c): a rope from the belt (the Hook slot),
//! whatever is in the hand. Terraria's by default, a real rope when you
//! want one.
//!
//! - E throws it at the cursor, out to the rope's length; it takes hold of
//!   what it meets: rock (anything solid, or a platform), a chest or a body,
//!   a creature.
//! - Held by something that stays put (a cell, a creature bigger than you),
//!   it's a rope: you can't be further from it than its length, and you
//!   swing on it. When it bites, the rope reels itself in fast (Terraria's
//!   pull) until you're up at it, hanging.
//! - S lets rope out (and stops the reeling in: hook a ledge, rappel down a
//!   shaft), W climbs; A/D pump a swing (nothing brakes it); jump lets go,
//!   with a full jump on top of how you were swinging; E throws it again.
//!   Touching a wall on the rope, push away from it: a kick into a swing.
//! - Hanging from a ledge's lip, W or pressing toward the ledge climbs you
//!   up onto it (a mantle). Pulled in and caught on a corner, you slip
//!   round it.
//! - The rope wraps round corners on the way and unwraps swinging back.
//!   The cell it holds is only rock while it's there: dig it out, blast it,
//!   melt it, and the hook comes loose.
//! - Something smaller than you (a chest, a body, a bat): it's pulled to you.
//!
//! The rope is drawn a cell at a time, on its own canvas (`canvas.rs`).
//! `physics::tether` holds the body at its end; `actors::move_creatures`
//! applies it, between steering and moving.

use bevy::prelude::*;
use platypus_physics::{Grid, Occupancy, tether};
use platypus_sim::CellPos;
use serde::Deserialize;

use super::Equipment;
use crate::actors::{Controls, Creature, Kinematics, MoveStats, WorldGrid};
use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprite, CanvasSprites};
use crate::combat::Hit;
use crate::hands::chests::Container;
use crate::hands::items::Items;
use crate::magic::runes::Emitter;
use crate::vfx::Sparks;
use crate::world::{ChunkLoader, SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;

/// A hook's rope: how far it reaches (cells), how fast it reels in on its
/// own (and pulls what it holds in) and flies out (cells/s), what the hook
/// does to a creature it bites into, and how it looks (its colour;
/// `links`: a chain, drawn link by link).
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct HookDef {
    pub length: f32,
    pub reel: f32,
    pub speed: f32,
    pub damage: f32,
    pub rope: (u8, u8, u8),
    pub links: bool,
}

impl Default for HookDef {
    fn default() -> Self {
        HookDef { length: 120.0, reel: 240.0, speed: 800.0, damage: 4.0, rope: (176, 140, 90), links: false }
    }
}

/// The least rope between you and what it holds (cells), past your box
/// (reeled all the way in, you hang just off it).
const SHORTEST: f32 = 2.0;
/// W and S: rope taken in and let out (cells/s).
const CLIMB: f32 = 90.0;
/// Something with less than this share of your size is pulled to you;
/// more, you to it.
const SMALLER: f32 = 0.75;
/// A pulled thing further than this past where it should be (stuck behind
/// something as you go): the hook lets go.
const STRAIN: f32 = 24.0;
/// The rope's line to a body blocked this long: it lets go.
const BLOCKED: f32 = 0.25;
/// Letting go with a jump: half a jump on top of how you were moving up,
/// and at least a whole one.
const LEAP_ON_TOP: f32 = 0.5;
/// How fast the hook comes back when it misses (× its speed).
const RETURN: f32 = 1.6;
/// Pulled in and getting no nearer this long, against something: you slip
/// round it, up to this many cells along it.
const STALL: f32 = 0.08;
const SLIP: i32 = 4;
/// A kick off a wall: sideways (cells/s) and a little up; and how long
/// before another.
const KICK: f32 = 260.0;
const KICK_UP: f32 = 60.0;
const KICK_AGAIN: f32 = 0.35;
/// Rock this near the point the rope is held at doesn't count as in its way
/// (the rock it's hooked on, a corner it's wrapped round).
const HELD_CLEAR: f32 = 2.0;
/// Within this much of hanging, you can mantle onto the ledge you hang from.
const MANTLE_NEAR: f32 = 3.0;

/// What the hook has hold of.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Anchor {
    /// A cell (it holds while the cell is solid), at a point just outside it.
    Cell { cell: CellPos, at: Vec2 },
    /// A body, at an offset from its middle.
    Body { entity: Entity, off: Vec2 },
}

/// A corner the rope is wrapped round, and which side of it (the sign of
/// the turn the rope makes there) you were on when it wrapped.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pivot {
    at: Vec2,
    side: f32,
}

/// Holding: what; round which corners; how long the rope is (all of it,
/// anchor to you); reeling itself in (it does when it bites, until you're
/// up at it or you let rope out); pulling that thing to you (else you to
/// it); how long the line has been blocked, or you've been stuck.
#[derive(Clone, Debug, PartialEq)]
struct Hold {
    anchor: Anchor,
    pivots: Vec<Pivot>,
    length: f32,
    reeling: bool,
    leash: bool,
    blocked: f32,
    stalled: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
enum Line {
    #[default]
    Stowed,
    /// Thrown: where the hook is, how it's going, how much rope is out.
    Out { tip: Vec2, vel: Vec2, paid: f32 },
    /// Coming back, having missed or let go.
    Back { tip: Vec2 },
    Held(Hold),
}

/// A creature's grappling hook: the rope's state, and what `move_creatures`
/// needs from it (the point it swings round, and the rope from there).
#[derive(Component, Default)]
pub struct Rope {
    line: Line,
    held: bool,
    /// Where the body was last tick (the rope was clear to there).
    last: Option<Vec2>,
    /// Where the hook was a tick ago (drawing it between ticks).
    prev_tip: Vec2,
    tether: Option<(Vec2, f32)>,
    /// Steering last tick (a kick is a press away from the wall), and when
    /// the next kick can come.
    prev_x: f32,
    kick: f32,
    /// What it's doing, for scenarios.
    doing: &'static str,
    /// The hook it threw (how its rope looks).
    look: HookDef,
}

impl Rope {
    /// On the rope: the point it swings round, and how much rope there is
    /// from there.
    pub fn tether(&self) -> Option<(Vec2, f32)> {
        self.tether
    }

    /// What the rope's doing: "stowed", "out", "back", "reel" (pulling you
    /// in), "hang" (up at it), "swing" (on the rope), "leash" (pulling a
    /// thing to you); and how many corners it's wrapped round.
    pub fn state(&self) -> (&'static str, usize) {
        match &self.line {
            Line::Stowed => ("stowed", 0),
            Line::Out { .. } => ("out", 0),
            Line::Back { .. } => ("back", 0),
            Line::Held(h) => (self.doing, h.pivots.len()),
        }
    }
}

/// The best hook it wears (the longest rope).
fn worn_hook<'a>(eq: &'a Equipment, items: &'a Items) -> Option<&'a HookDef> {
    eq.pieces(items).filter_map(|s| items.def(s.item).gear.as_ref()?.hook.as_ref()).max_by(|a, b| a.length.total_cmp(&b.length))
}

/// Does the rope hold on to this cell?
fn grips(grid: &WorldGrid, c: CellPos) -> bool {
    matches!(grid.occupancy(c.x, c.y), Occupancy::Solid | Occupancy::Platform)
}

fn solid(grid: &WorldGrid, c: CellPos) -> bool {
    grid.occupancy(c.x, c.y) == Occupancy::Solid
}

/// The first solid cell on the line from `a` to `b` (a look every half
/// cell), past the first `skip` cells from `a`; platforms don't block a rope.
fn first_solid(grid: &WorldGrid, a: Vec2, b: Vec2, skip: f32) -> Option<CellPos> {
    let d = b - a;
    let len = d.length();
    let n = (len * 2.0).ceil() as i32;
    (1..n).map(|i| a + d * (i as f32 / n as f32)).filter(|p| p.distance(a) > skip && p.distance(b) > 1.0).map(|p| CellPos::from_world(p.x, p.y)).find(|c| solid(grid, *c))
}

/// Room for a box of half size `half` at `at`: nothing solid in it.
fn fits(grid: &WorldGrid, at: Vec2, half: Vec2) -> bool {
    let (lo, hi) = ((at - half).floor().as_ivec2(), (at + half - Vec2::splat(0.01)).floor().as_ivec2());
    (lo.y..=hi.y).all(|y| (lo.x..=hi.x).all(|x| matches!(grid.occupancy(x, y), Occupancy::Empty | Occupancy::Liquid)))
}

fn cross(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

/// The rope from `from` (the last point it's held at) was clear to `was`
/// and isn't to `now`: the corner it caught on, a little outside the rock.
fn corner(grid: &WorldGrid, from: Vec2, was: Vec2, now: Vec2) -> Option<Vec2> {
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..10 {
        let mid = (lo + hi) / 2.0;
        if first_solid(grid, from, was.lerp(now, mid), HELD_CLEAR).is_some() {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let hit = first_solid(grid, from, was.lerp(now, hi), HELD_CLEAR)?;
    let clear = was.lerp(now, lo);
    let centre = Vec2::new(hit.x as f32 + 0.5, hit.y as f32 + 0.5);
    // The point of the clear line nearest the rock, then out from the rock
    // (so the rope bends round it, not through it).
    let line = clear - from;
    let t = ((centre - from).dot(line) / line.length_squared().max(1e-4)).clamp(0.0, 1.0);
    let near = from + line * t;
    let out = (near - centre).normalize_or(Vec2::Y);
    let at = centre + out * 1.2;
    let c = CellPos::from_world(at.x, at.y);
    Some(if solid(grid, c) { near } else { at })
}

/// The rope from `from` to `to` runs through rock: a corner a little
/// outside the first rock in the way, on the side the line passes (a rope
/// that caught on something before there was a clear line to wrap from).
fn round(grid: &WorldGrid, from: Vec2, to: Vec2) -> Option<Vec2> {
    let hit = first_solid(grid, from, to, HELD_CLEAR)?;
    let centre = Vec2::new(hit.x as f32 + 0.5, hit.y as f32 + 0.5);
    let line = to - from;
    let t = ((centre - from).dot(line) / line.length_squared().max(1e-4)).clamp(0.0, 1.0);
    let near = from + line * t;
    let out = (near - centre).normalize_or(line.perp().normalize_or(Vec2::Y));
    (1..=3).map(|k| centre + out * (0.7 + k as f32 * 0.5)).find(|p| !solid(grid, CellPos::from_world(p.x, p.y)))
}

/// How far from the point it hangs from a body hangs (its box's edge just
/// off it), seen along `d`.
fn hang_distance(half: Vec2, d: Vec2) -> f32 {
    (half.x * d.x).abs() + (half.y * d.y).abs() + SHORTEST
}

/// Onto the ledge at `point` (the lip it hangs from, or a face just below
/// it), to the side `side`, for a body of `half` at `pos`: where it can stand
/// on top, if the top's in reach (a few cells over its head) and there's room.
fn mantle_spot(grid: &WorldGrid, point: Vec2, side: f32, pos: Vec2, half: Vec2) -> Option<Vec2> {
    // The rock the point is on (it sits just off the face).
    let c = CellPos::from_world(point.x + side * 0.6, point.y);
    let reach = (c.y + 4).max((pos.y + half.y) as i32 + 3);
    let top = (c.y - 2..=reach).find(|&y| solid(grid, CellPos::new(c.x, y - 1)) && !solid(grid, CellPos::new(c.x, y)))?;
    // Along the top, the first place that fits with rock under its middle
    // (lifted a little over a rounded or ragged top).
    (0..=(half.x * 2.0) as i32 + 2)
        .flat_map(|k| (0..=4).map(move |lift| Vec2::new(point.x + side * k as f32, top as f32 + half.y + 0.05 + lift as f32)))
        .find(|&p| fits(grid, p, half) && solid(grid, CellPos::from_world(p.x, p.y - half.y - 0.5)))
}

type Holder<'a> = (Entity, &'a Controls, &'a Equipment, Option<&'a mut Rope>);
type Thing<'a> = (Entity, &'a mut Kinematics, Option<&'a MoveStats>, Has<Creature>, Has<Container>);

/// Each tick, before bodies move: throw, fly, take hold, wrap, reel, climb,
/// slip, mantle, kick, let go.
#[allow(clippy::too_many_arguments)]
pub fn rope(
    mut commands: Commands,
    items: Option<Res<Items>>,
    sim: Res<SimWorld>,
    mut sparks: ResMut<Sparks>,
    mut hits: MessageWriter<Hit>,
    mut holders: Query<Holder>,
    mut things: Query<Thing>,
    tempo: Res<crate::tempo::Tempo>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    let Some(items) = items else { return };
    let grid = WorldGrid(&sim.world);
    for (e, controls, eq, rope) in &mut holders {
        let Some(def) = worn_hook(eq, &items) else {
            if let Some(mut r) = rope
                && r.line != Line::Stowed
            {
                r.line = Line::Stowed;
                r.tether = None;
            }
            continue;
        };
        let Some(mut rope) = rope else {
            commands.entity(e).try_insert(Rope::default());
            continue;
        };
        let rope = &mut *rope;
        rope.look = def.clone();
        rope.tether = None;
        rope.kick = (rope.kick - DT).max(0.0);
        let intent = controls.0;
        let pressed = intent.hook && !rope.held;
        rope.held = intent.hook;
        let prev_x = std::mem::replace(&mut rope.prev_x, intent.move_x);
        let Ok((_, me, my_stats, ..)) = things.get(e) else { continue };
        let (pos, half, facing, contacts) = (me.body.pos, me.body.half, me.loco.facing, me.loco.contacts);
        let my_size = half.x * half.y;
        // (At the game's tempo: hooks are the players'.)
        let stats = tempo.apply(&my_stats.map(|s| s.0.clone()).unwrap_or_default(), true);
        let hand = pos + Vec2::new(facing * 2.0, half.y * 0.3);
        if pressed {
            let aim = if intent.aim != Vec2::ZERO { intent.aim } else { hand + Vec2::new(facing, 1.0) };
            let dir = (aim - hand).normalize_or(Vec2::new(facing, 0.0));
            rope.line = Line::Out { tip: hand, vel: dir * def.speed, paid: 0.0 };
            rope.prev_tip = hand;
            rope.last = None;
            sparks.emit(&THROW, THROW.count as usize, hand, dir, Vec2::ZERO);
            sounds.write(crate::sound::PlaySound::at("hook_throw", hand));
        }
        match rope.line.clone() {
            Line::Stowed => {}
            Line::Out { tip, vel, paid } => {
                rope.prev_tip = tip;
                let go = vel.length() * DT;
                let steps = go.ceil().max(1.0) as i32;
                let step = vel * DT / steps as f32;
                let mut at = tip;
                let mut took = None;
                for _ in 0..steps {
                    let next = at + step;
                    let c = CellPos::from_world(next.x, next.y);
                    if !sim.world.is_loaded(c.chunk()) {
                        break;
                    }
                    if grips(&grid, c) {
                        took = Some((Anchor::Cell { cell: c, at }, None));
                        break;
                    }
                    // A body in the way: the nearest of them.
                    let hit = things.iter().filter(|(t, _, _, creature, container)| *t != e && (*creature || *container)).find(|(_, k, ..)| {
                        let d = (next - k.body.pos).abs() - k.body.half;
                        d.max_element() <= 0.5
                    });
                    if let Some((t, k, _, creature, _)) = hit {
                        let off = (next - k.body.pos).clamp(-k.body.half, k.body.half);
                        let size = k.body.half.x * k.body.half.y;
                        // Pulled to you, or you to it.
                        let leash = !creature || size < my_size * SMALLER;
                        took = Some((Anchor::Body { entity: t, off }, Some((leash, creature))));
                        break;
                    }
                    at = next;
                }
                let paid = paid + go;
                match took {
                    Some((anchor, body)) => {
                        let point = match anchor {
                            Anchor::Cell { at, .. } => at,
                            Anchor::Body { entity, off } => things.get(entity).map_or(at, |(_, k, ..)| k.body.pos + off),
                        };
                        let leash = body.is_some_and(|(l, _)| l);
                        rope.line = Line::Held(Hold { anchor, pivots: Vec::new(), length: pos.distance(point), reeling: true, leash, blocked: 0.0, stalled: 0.0 });
                        rope.prev_tip = point;
                        sparks.emit(&CLINK, CLINK.count as usize, point, -vel.normalize_or(Vec2::Y), Vec2::ZERO);
                        sounds.write(crate::sound::PlaySound::at("hook_bite", point));
                        if let (Anchor::Body { entity, .. }, Some((_, true))) = (anchor, body) {
                            let dir = vel.normalize_or(Vec2::X);
                            hits.write(Hit { target: entity, damage: def.damage, harm: crate::actors::Harm::Pierce, knock: Vec2::ZERO, stun: 0.0, at: point, dir, weight: 0.4, crit: false });
                        }
                        if !leash && let Ok((_, mut k, ..)) = things.get_mut(e) {
                            // (Hooked on in the air: your jumps back, as on landing.)
                            k.loco.refresh_air(&stats);
                        }
                    }
                    None if paid >= def.length || at == tip => rope.line = Line::Back { tip: at },
                    None => rope.line = Line::Out { tip: at, vel, paid },
                }
            }
            Line::Back { tip } => {
                rope.prev_tip = tip;
                let to = hand - tip;
                let go = def.speed * RETURN * DT;
                rope.line = if to.length() <= go { Line::Stowed } else { Line::Back { tip: tip + to.normalize() * go } };
            }
            Line::Held(mut h) => {
                // What it holds, and whether it still does.
                let point = match h.anchor {
                    Anchor::Cell { cell, at } => grips(&grid, cell).then_some(at),
                    Anchor::Body { entity, off } => things.get(entity).ok().map(|(_, k, ..)| k.body.pos + off),
                };
                let Some(point) = point else {
                    // Torn loose: the cell's gone, the body's gone.
                    let at = match h.anchor {
                        Anchor::Cell { at, .. } => at,
                        Anchor::Body { .. } => rope.prev_tip,
                    };
                    sparks.emit(&CLINK, CLINK.count as usize, at, Vec2::Y, Vec2::ZERO);
                    rope.line = Line::Back { tip: at };
                    continue;
                };
                rope.prev_tip = point;
                if h.leash {
                    // Pulled to you (reeled in on its own), as you'd be to it.
                    let Anchor::Body { entity, off } = h.anchor else { unreachable!("a leash holds a body") };
                    h.blocked = if first_solid(&grid, point, pos, HELD_CLEAR).is_some() { h.blocked + DT } else { 0.0 };
                    if h.blocked > BLOCKED {
                        rope.line = Line::Back { tip: point };
                        continue;
                    }
                    let d = (pos - point).normalize_or(Vec2::NEG_Y);
                    if let Ok((_, mut k, _, creature, _)) = things.get_mut(entity) {
                        let k = &mut *k;
                        let stop = hang_distance(half, d) + (k.body.half.x * d.x).abs() + (k.body.half.y * d.y).abs();
                        let dist = (k.body.pos + off).distance(pos);
                        if dist > def.length + STRAIN {
                            rope.line = Line::Back { tip: point };
                            continue;
                        }
                        let mut body = k.body;
                        body.pos += off;
                        if tether(&mut body, pos, (dist - def.reel * DT).max(stop), DT) {
                            if creature {
                                // (It struggles: it can't walk off.)
                                k.loco.knock(&mut k.body, body.vel, 0.12);
                            } else {
                                k.body.vel = body.vel;
                            }
                        }
                    }
                    rope.doing = "leash";
                    rope.line = Line::Held(h);
                    continue;
                }
                let Ok((_, mut k, ..)) = things.get_mut(e) else { continue };
                let k = &mut *k;
                // Jump: let go, with a jump on top of the swing.
                if k.loco.jump_pressed(&intent) {
                    let up = (k.body.vel.y + stats.jump_speed() * LEAP_ON_TOP).max(stats.jump_speed());
                    k.body.vel.y = up;
                    k.loco.leap(&stats, &intent, &mut k.body, 0.0);
                    rope.line = Line::Back { tip: point };
                    continue;
                }
                // Round the corners (a cell's rope): unwrap what you've swung
                // back past, wrap on what's come between. A body's can't
                // bend: blocked long enough, it lets go.
                if matches!(h.anchor, Anchor::Cell { .. }) {
                    while let Some(p) = h.pivots.last().copied() {
                        let before = if h.pivots.len() >= 2 { h.pivots[h.pivots.len() - 2].at } else { point };
                        if cross(p.at - before, pos - p.at).signum() != p.side && first_solid(&grid, before, pos, HELD_CLEAR).is_none() {
                            h.pivots.pop();
                        } else {
                            break;
                        }
                    }
                    let mut through = false;
                    for _ in 0..4 {
                        let from = h.pivots.last().map_or(point, |p| p.at);
                        if first_solid(&grid, from, pos, HELD_CLEAR).is_none() {
                            break;
                        }
                        // (Where it caught, from where the body was; or, with no
                        // clear place it was, round what's in the way.)
                        let wrapped = rope.last.and_then(|was| first_solid(&grid, from, was, HELD_CLEAR).is_none().then(|| corner(&grid, from, was, pos)).flatten()).or_else(|| round(&grid, from, pos));
                        match wrapped {
                            Some(at) if at.distance(from) > 1.5 => h.pivots.push(Pivot { at, side: cross(at - from, pos - at).signum() }),
                            // (The same corner again.)
                            Some(_) if !h.pivots.is_empty() => break,
                            _ => {
                                through = true;
                                break;
                            }
                        }
                    }
                    if through {
                        // (Somehow through the rock: the hook comes free.)
                        rope.line = Line::Back { tip: h.pivots.last().map_or(point, |p| p.at) };
                        continue;
                    }
                } else {
                    h.blocked = if first_solid(&grid, point, pos, HELD_CLEAR).is_some() { h.blocked + DT } else { 0.0 };
                    if h.blocked > BLOCKED {
                        rope.line = Line::Back { tip: point };
                        continue;
                    }
                }
                // The rope spent round the corners, and from the last one.
                let mut chain = 0.0;
                let mut from = point;
                for p in &h.pivots {
                    chain += from.distance(p.at);
                    from = p.at;
                }
                let dist = from.distance(pos);
                let d = (pos - from).normalize_or(Vec2::NEG_Y);
                let stop = hang_distance(half, d);
                let taut = h.length.min(chain + dist);
                // S lets rope out (and stops the reeling in); W climbs; when
                // it bites it reels itself in until you're up at it. (Never
                // more than a couple of ticks' reeling short of where you
                // are: caught on something, the rope doesn't wind up, to
                // fling you when you come free.)
                let speed = if intent.move_y > 0.0 { CLIMB.max(if h.reeling { def.reel } else { 0.0 }) } else if h.reeling { def.reel } else { 0.0 };
                if intent.move_y < 0.0 {
                    h.reeling = false;
                    h.length += CLIMB * DT;
                } else if speed > 0.0 {
                    h.length = (taut - speed * DT).max(chain + dist - 2.0 * speed * DT);
                }
                h.length = h.length.clamp(chain + stop, def.length.max(chain + stop));
                let pulling = h.reeling || intent.move_y > 0.0;
                // (Up at the hook itself, not a corner on the way.)
                if h.reeling && h.pivots.is_empty() && h.length <= chain + stop + 0.5 {
                    h.reeling = false;
                }
                let hanging = chain + dist <= h.length.max(chain + stop) + MANTLE_NEAR && h.length <= chain + stop + MANTLE_NEAR;
                // Pulled in, but caught on something and getting no nearer:
                // slip round it, along what's in the way.
                // (Or reeled up to a corner the rope's wrapped round: round it,
                // so it unwraps and the pull goes on.)
                let touching = contacts.wall_left || contacts.wall_right || contacts.ceiling || contacts.ground;
                let nearer = rope.last.is_some_and(|was| was.distance(from) - dist > CLIMB.min(def.reel) * DT * 0.3);
                let at_corner = !h.pivots.is_empty() && dist <= stop + 1.5;
                h.stalled = if pulling && ((touching && !nearer && dist > stop + 1.0) || at_corner) { h.stalled + DT } else { 0.0 };
                if h.stalled > STALL {
                    let along = d.perp();
                    // (Past a corner: where the rope from the one before is clear.)
                    let beyond = if at_corner { h.pivots.len().checked_sub(2).map_or(point, |i| h.pivots[i].at) } else { from };
                    let slip = (1..=SLIP + 2)
                        .flat_map(|n| [n as f32, -(n as f32)])
                        .flat_map(|n| [pos + along * n - d, pos + along * n - d * 3.0])
                        .find(|&p| fits(&grid, p, half) && first_solid(&grid, beyond, p, HELD_CLEAR).is_none());
                    if let Some(p) = slip {
                        k.body.pos = p;
                        k.prev_pos = p;
                        // (Round the corner: the rope's clear of it now.)
                        if at_corner {
                            h.pivots.pop();
                        }
                    }
                    h.stalled = 0.0;
                }
                // Hanging from a ledge's lip: W, or pressing toward the ledge,
                // climbs up onto it.
                let side = if (point.x - pos.x).abs() > 0.5 { (point.x - pos.x).signum() } else { facing };
                let toward = intent.move_x != 0.0 && intent.move_x.signum() == side;
                if hanging
                    && h.pivots.is_empty()
                    && matches!(h.anchor, Anchor::Cell { .. })
                    && (intent.move_y > 0.0 || toward)
                    && let Some(spot) = mantle_spot(&grid, point, side, pos, half).filter(|s| s.y > pos.y)
                {
                    k.body.pos = spot;
                    k.prev_pos = spot;
                    k.body.vel = Vec2::ZERO;
                    sparks.emit(&THROW, THROW.count as usize, spot - Vec2::Y * half.y, Vec2::Y, Vec2::ZERO);
                    rope.line = Line::Back { tip: point };
                    rope.last = Some(spot);
                    continue;
                }
                // On the rope, against a wall: push away from it, a kick into
                // a swing.
                let wall = if contacts.wall_left { -1.0 } else if contacts.wall_right { 1.0 } else { 0.0 };
                let away = intent.move_x != 0.0 && intent.move_x.signum() == -wall && (prev_x == 0.0 || prev_x.signum() != intent.move_x.signum());
                if wall != 0.0 && away && rope.kick <= 0.0 {
                    k.body.vel.x = -wall * KICK;
                    k.body.vel.y = k.body.vel.y.max(0.0) + KICK_UP;
                    rope.kick = KICK_AGAIN;
                    sparks.emit(&THROW, THROW.count as usize, pos + Vec2::new(wall * half.x, 0.0), Vec2::new(-wall, 0.0), Vec2::ZERO);
                }
                rope.doing = if pulling && !hanging {
                    "reel"
                } else if hanging {
                    "hang"
                } else {
                    "swing"
                };
                rope.tether = Some((from, (h.length - chain).max(stop)));
                rope.line = Line::Held(h);
            }
        }
        rope.last = Some(things.get(e).map_or(pos, |(_, k, ..)| k.body.pos));
    }
}

/// The rope's canvas, over the bodies (the rope comes from the hand).
#[derive(Resource)]
pub struct RopeCanvas(Canvas);

impl Default for RopeCanvas {
    fn default() -> Self {
        RopeCanvas(Canvas::new("Ropes", 10.8))
    }
}

/// Every rope, a cell at a time: hand, round its corners, to the hook
/// (sagging when slack), and the hook's claws.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    mut commands: Commands,
    mut canvas: ResMut<RopeCanvas>,
    mut images: ResMut<Assets<Image>>,
    time: Res<Time<Fixed>>,
    camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>,
    ropes: Query<(&Rope, &Transform, &Kinematics), Without<CanvasSprite>>,
    bodies: Query<&Transform, Without<CanvasSprite>>,
    mut sprites: CanvasSprites,
) {
    let Ok((cam, loader)) = camera.single() else { return };
    let any = ropes.iter().any(|(r, ..)| r.line != Line::Stowed);
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, any) else { return };
    let a = time.overstep_fraction();
    for (rope, tf, k) in &ropes {
        let def = &rope.look;
        let me = tf.translation.truncate();
        let hand = me + Vec2::new(k.loco.facing * 2.0, k.body.half.y * 0.3);
        let (tip, length, pivots): (Vec2, Option<f32>, &[Pivot]) = match &rope.line {
            Line::Stowed => continue,
            Line::Out { tip, .. } | Line::Back { tip } => (rope.prev_tip.lerp(*tip, a), None, &[]),
            Line::Held(h) => {
                let at = match h.anchor {
                    Anchor::Cell { at, .. } => at,
                    Anchor::Body { entity, off } => bodies.get(entity).map_or(rope.prev_tip, |t| t.translation.truncate() + off),
                };
                (at, (!h.leash).then_some(h.length), h.pivots.as_slice())
            }
        };
        // Hook, round the corners, to the hand.
        let mut points = vec![tip];
        points.extend(pivots.iter().map(|p| p.at));
        points.push(hand);
        let spent: f32 = points.windows(2).map(|w| w[0].distance(w[1])).sum();
        let slack = length.map_or(0.0, |l| (l - spent).max(0.0));
        let (r, g, b) = def.rope;
        let dark = [(r as f32 * 0.62) as u8, (g as f32 * 0.62) as u8, (b as f32 * 0.62) as u8, 255];
        let light = [r, g, b, 255];
        let mut n = 0u32;
        let segs = points.len() - 1;
        for (i, w) in points.windows(2).enumerate() {
            let (p, q) = (w[0], w[1]);
            // The last stretch (to the hand) sags with the slack.
            let sag = if i + 1 == segs { (slack * p.distance(q)).sqrt().min(40.0) * 0.5 } else { 0.0 };
            let mid = (p + q) / 2.0 - Vec2::Y * sag;
            let steps = (p.distance(q) * 2.0).ceil().max(1.0) as i32 + (sag * 2.0) as i32;
            let mut last = IVec2::MAX;
            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let c = p.lerp(mid, t).lerp(mid.lerp(q, t), t).floor().as_ivec2();
                if c == last {
                    continue;
                }
                last = c;
                n += 1;
                // Twisted rope: light and dark by turns; a chain: links.
                let col = if def.links { if (n / 2).is_multiple_of(2) { light } else { dark } } else if n.is_multiple_of(3) { dark } else { light };
                px.put(c.x, c.y, col);
            }
        }
        // The hook: a point and two barbs back along the rope.
        let back = (points[1] - tip).normalize_or(Vec2::NEG_Y);
        let side = back.perp();
        let metal = [206, 208, 216, 255];
        let shade = [120, 122, 134, 255];
        for (o, col) in [(Vec2::ZERO, metal), (back, metal), (back * 2.0 + side * 1.5, shade), (back * 2.0 - side * 1.5, shade), (back + side, metal), (back - side, metal)] {
            let c = (tip + o).floor().as_ivec2();
            px.put(c.x, c.y, col);
        }
    }
}

/// The throw's whoosh (and a kick's, a mantle's).
static THROW: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 3.0,
    life: (0.08, 0.18),
    colors: vec![(230, 230, 240), (180, 180, 200)],
    speed: 60.0,
    spread: 0.5,
    gravity: 0.0,
    drag: 5.0,
    size: 1.0,
    jitter: 0.0,
    glow: false,
});

/// The hook biting in (and tearing loose): sparks off it.
static CLINK: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 6.0,
    life: (0.06, 0.2),
    colors: vec![(255, 250, 220), (255, 220, 140), (200, 200, 210)],
    speed: 110.0,
    spread: 1.2,
    gravity: -200.0,
    drag: 3.0,
    size: 1.0,
    jitter: 0.0,
    glow: true,
});
