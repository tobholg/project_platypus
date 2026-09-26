//! Grappling hooks (DESIGN §7c): a rope from the belt (the Hook slot),
//! whatever is in the hand. Worms' ninja rope more than Terraria's hook:
//!
//! - E throws it at the cursor, out to the rope's length; it takes hold of
//!   what it meets: rock (anything solid, or a platform), a chest or a body,
//!   a creature.
//! - Held by something that stays put (a cell, a creature bigger than you):
//!   you hang from it and swing, pumping with A/D; W climbs, S lets out
//!   rope, holding E reels you in; jump lets go, keeping your speed (and a
//!   little lift). The rope wraps round corners on the way and unwraps
//!   swinging back.
//! - The cell it holds is only rock while it's there: dig it out, blast it,
//!   melt it, and the rope comes loose.
//! - Something smaller (a chest, a body, a bat, a slime): it's on your
//!   leash; holding E reels it in.
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

/// A hook's rope: how far it reaches (cells), how fast it reels in and flies
/// out (cells/s), what the hook does to a creature it bites into, and how it
/// looks (its colour; `links`: a chain, drawn link by link).
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

/// W and S: rope taken in or let out (cells/s).
const CLIMB: f32 = 90.0;
/// The least rope between you and what it holds (cells), past half your
/// height (reeled all the way in, you hang with your head just under it).
const SHORTEST: f32 = 3.0;
/// Something with less than this share of your size is pulled to you;
/// more, you to it.
const SMALLER: f32 = 0.75;
/// A leashed thing further than this past the rope's length (stuck behind
/// something as you go): the rope lets go.
const STRAIN: f32 = 24.0;
/// A rope's line blocked this long (something between you and a creature
/// it holds): it lets go.
const BLOCKED: f32 = 0.4;
/// Letting go in the air: at least this share of a jump's speed up.
const LEAP: f32 = 0.55;
/// How fast the hook comes back when it misses (× its speed).
const RETURN: f32 = 1.6;

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

#[derive(Clone, Debug, Default, PartialEq)]
enum Line {
    #[default]
    Stowed,
    /// Thrown: where the hook is, how it's going, how much rope is out.
    Out { tip: Vec2, vel: Vec2, paid: f32 },
    /// Coming back, having missed.
    Back { tip: Vec2 },
    /// Holding: what, round which corners, how long the rope is (all of it,
    /// anchor to you), and whether it's pulling that thing to you.
    Held { anchor: Anchor, pivots: Vec<Pivot>, length: f32, leash: bool, blocked: f32 },
}

/// A creature's grappling hook: the rope's state, and what `move_creatures`
/// needs from it (the point it swings from, and the rope left from there).
#[derive(Component, Default)]
pub struct Rope {
    line: Line,
    held: bool,
    /// Where the body was last tick (the rope was clear to there).
    last: Option<Vec2>,
    /// Where the hook was a tick ago (drawing it between ticks).
    prev_tip: Vec2,
    tether: Option<(Vec2, f32)>,
    /// The hook it threw (how its rope looks).
    look: HookDef,
}

impl Rope {
    /// Hanging from the rope: the point it swings round, and how much rope
    /// there is from there.
    pub fn tether(&self) -> Option<(Vec2, f32)> {
        self.tether
    }

    /// What the rope's doing, for scenarios: "stowed", "out", "back",
    /// "swing" or "leash", and how many corners it's wrapped round.
    pub fn state(&self) -> (&'static str, usize) {
        match &self.line {
            Line::Stowed => ("stowed", 0),
            Line::Out { .. } => ("out", 0),
            Line::Back { .. } => ("back", 0),
            Line::Held { leash: true, .. } => ("leash", 0),
            Line::Held { pivots, .. } => ("swing", pivots.len()),
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

/// The first solid cell on the line from `a` to `b` (a look every half
/// cell), past the first `skip` cells from `a`; platforms don't block a rope.
fn first_solid(grid: &WorldGrid, a: Vec2, b: Vec2, skip: f32) -> Option<CellPos> {
    let d = b - a;
    let len = d.length();
    let n = (len * 2.0).ceil() as i32;
    (1..n).map(|i| a + d * (i as f32 / n as f32)).filter(|p| p.distance(a) > skip && p.distance(b) > 1.0).map(|p| CellPos::from_world(p.x, p.y)).find(|c| grid.occupancy(c.x, c.y) == Occupancy::Solid)
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
        if first_solid(grid, from, was.lerp(now, mid), 1.0).is_some() {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let hit = first_solid(grid, from, was.lerp(now, hi), 1.0)?;
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
    Some(if grid.occupancy(c.x, c.y) == Occupancy::Solid { near } else { at })
}

type Holder<'a> = (Entity, &'a Controls, &'a Equipment, Option<&'a mut Rope>);
type Thing<'a> = (Entity, &'a mut Kinematics, Option<&'a MoveStats>, Has<Creature>, Has<Container>);

/// Each tick, before bodies move: throw, fly, take hold, wrap, reel, let go.
#[allow(clippy::too_many_arguments)]
pub fn rope(
    mut commands: Commands,
    items: Option<Res<Items>>,
    sim: Res<SimWorld>,
    mut sparks: ResMut<Sparks>,
    mut hits: MessageWriter<Hit>,
    mut holders: Query<Holder>,
    mut things: Query<Thing>,
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
        let intent = controls.0;
        let pressed = intent.hook && !rope.held;
        rope.held = intent.hook;
        let Ok((_, me, my_stats, ..)) = things.get(e) else { continue };
        let (pos, half, facing, grounded) = (me.body.pos, me.body.half, me.loco.facing, me.loco.grounded());
        let my_size = half.x * half.y;
        let stats = my_stats.map(|s| s.0.clone()).unwrap_or_default();
        let hand = pos + Vec2::new(facing * 2.0, half.y * 0.3);
        let shortest = SHORTEST + half.y;
        if pressed {
            let aim = if intent.aim != Vec2::ZERO { intent.aim } else { hand + Vec2::new(facing, 1.0) };
            let dir = (aim - hand).normalize_or(Vec2::new(facing, 0.0));
            rope.line = Line::Out { tip: hand, vel: dir * def.speed, paid: 0.0 };
            rope.prev_tip = hand;
            rope.last = None;
            sparks.emit(&THROW, THROW.count as usize, hand, dir, Vec2::ZERO);
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
                        let length = pos.distance(point).max(shortest);
                        rope.line = Line::Held { anchor, pivots: Vec::new(), length, leash, blocked: 0.0 };
                        rope.prev_tip = point;
                        sparks.emit(&CLINK, CLINK.count as usize, point, -vel.normalize_or(Vec2::Y), Vec2::ZERO);
                        if let (Anchor::Body { entity, .. }, Some((_, true))) = (anchor, body) {
                            let dir = vel.normalize_or(Vec2::X);
                            hits.write(Hit { target: entity, damage: def.damage, knock: Vec2::ZERO, stun: 0.0, at: point, dir, weight: 0.4, crit: false });
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
            Line::Held { anchor, mut pivots, mut length, leash, mut blocked } => {
                // What it holds, and whether it still does.
                let point = match anchor {
                    Anchor::Cell { cell, at } => grips(&grid, cell).then_some(at),
                    Anchor::Body { entity, off } => things.get(entity).ok().map(|(_, k, ..)| k.body.pos + off),
                };
                let Some(point) = point else {
                    // Torn loose: the cell's gone, the body's gone.
                    let at = match anchor {
                        Anchor::Cell { at, .. } => at,
                        Anchor::Body { .. } => rope.prev_tip,
                    };
                    sparks.emit(&CLINK, CLINK.count as usize, at, Vec2::Y, Vec2::ZERO);
                    rope.line = Line::Back { tip: at };
                    continue;
                };
                rope.prev_tip = point;
                // Jump: let go (in the air, with a little lift, your speed
                // kept; on the ground the jump is a jump).
                if !leash
                    && let Ok((_, mut k, ..)) = things.get_mut(e)
                    && k.loco.jump_pressed(&intent)
                {
                    if !grounded {
                        let k = &mut *k;
                        k.loco.leap(&stats, &intent, &mut k.body, LEAP);
                    }
                    rope.line = Line::Back { tip: point };
                    continue;
                }
                // Round the corners (a cell anchor, swinging): unwrap what
                // you've swung back past, wrap on what's come between.
                if !leash && matches!(anchor, Anchor::Cell { .. }) {
                    while let Some(p) = pivots.last().copied() {
                        let before = if pivots.len() >= 2 { pivots[pivots.len() - 2].at } else { point };
                        let side = cross(p.at - before, pos - p.at).signum();
                        if side != p.side && first_solid(&grid, before, pos, 1.0).is_none() {
                            pivots.pop();
                        } else {
                            break;
                        }
                    }
                    let mut snapped = false;
                    for _ in 0..4 {
                        let from = pivots.last().map_or(point, |p| p.at);
                        if first_solid(&grid, from, pos, 1.0).is_none() {
                            break;
                        }
                        let wrapped = rope.last.and_then(|was| (first_solid(&grid, from, was, 1.0).is_none()).then(|| corner(&grid, from, was, pos)).flatten());
                        match wrapped {
                            Some(at) if at.distance(from) > 0.5 => {
                                let before = pivots.last().map_or(point, |p| p.at);
                                pivots.push(Pivot { at, side: cross(at - before, pos - at).signum() });
                            }
                            _ => {
                                snapped = true;
                                break;
                            }
                        }
                    }
                    if snapped {
                        // (Somehow through the rock: the rope comes free.)
                        rope.line = Line::Back { tip: pivots.last().map_or(point, |p| p.at) };
                        continue;
                    }
                } else {
                    // A body: the rope can't bend round things; blocked
                    // long enough, it lets go.
                    let far = if leash { point } else { pivots.last().map_or(point, |p| p.at) };
                    blocked = if first_solid(&grid, far, pos, 1.0).is_some() { blocked + DT } else { 0.0 };
                    if blocked > BLOCKED {
                        rope.line = Line::Back { tip: point };
                        continue;
                    }
                }
                // The rope already spent round the corners.
                let mut chain = 0.0;
                let mut from = point;
                for p in &pivots {
                    chain += from.distance(p.at);
                    from = p.at;
                }
                let dist = from.distance(pos);
                // Reel in (E held), climb (W), let out (S).
                let taut = length.min(chain + dist);
                if intent.hook && !pressed {
                    length = taut - def.reel * DT;
                } else if intent.move_y > 0.0 {
                    length = taut - CLIMB * DT;
                } else if intent.move_y < 0.0 && !leash {
                    length += CLIMB * DT;
                }
                length = length.clamp(chain + shortest, def.length.max(chain + shortest));
                if leash {
                    let Anchor::Body { entity, off } = anchor else { unreachable!("a leash holds a body") };
                    // Pulled to you.
                    if let Ok((_, mut k, _, creature, _)) = things.get_mut(entity) {
                        let k = &mut *k;
                        let d = (k.body.pos + off).distance(pos);
                        if d > length + STRAIN {
                            rope.line = Line::Back { tip: point };
                            continue;
                        }
                        let mut body = k.body;
                        body.pos += off;
                        if tether(&mut body, pos, length, DT) {
                            if creature {
                                // (It struggles: it can't walk off.)
                                k.loco.knock(&mut k.body, body.vel, 0.12);
                            } else {
                                k.body.vel = body.vel;
                            }
                        }
                    }
                } else {
                    rope.tether = Some((from, (length - chain).max(shortest)));
                }
                rope.line = Line::Held { anchor, pivots, length, leash, blocked };
            }
        }
        rope.last = Some(pos);
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
            Line::Held { anchor, pivots, length, .. } => {
                let at = match anchor {
                    Anchor::Cell { at, .. } => *at,
                    Anchor::Body { entity, off } => bodies.get(*entity).map_or(rope.prev_tip, |t| t.translation.truncate() + *off),
                };
                (at, Some(*length), pivots.as_slice())
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
                let at = p.lerp(mid, t).lerp(mid.lerp(q, t), t);
                let c = at.floor().as_ivec2();
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

/// The throw's whoosh.
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
