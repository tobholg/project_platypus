//! Grappling hooks (DESIGN §7c): Terraria's, from the belt (the Hook
//! slot), whatever is in the hand.
//!
//! - E throws it at the cursor, out to the rope's length; it takes hold of
//!   what it meets: rock (anything solid, or a platform), a chest or a body,
//!   a creature.
//! - Held by something that stays put (a cell, a creature bigger than you):
//!   it pulls you straight to it, and you hang there. Jump lets go, with a
//!   full jump (up a shaft: hook, pull, jump, hook again). E again throws
//!   it somewhere else.
//! - The cell it holds is only rock while it's there: dig it out, blast it,
//!   melt it, and the hook comes loose; so does anything coming between you.
//! - Something smaller (a chest, a body, a bat, a slime): it's pulled to you.
//!
//! The rope is drawn a cell at a time, on its own canvas (`canvas.rs`).
//! `actors::move_creatures` does the pulling (`Rope::pull`), after
//! steering; `physics::tether` drags what's leashed.

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

/// A hook's rope: how far it reaches (cells), how fast it pulls you (or
/// what it holds) in and flies out (cells/s), what the hook does to a
/// creature it bites into, and how it looks (its colour; `links`: a chain,
/// drawn link by link).
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
/// (pulled all the way in, you hang just off it).
const SHORTEST: f32 = 2.0;
/// Something with less than this share of your size is pulled to you;
/// more, you to it.
const SMALLER: f32 = 0.75;
/// A pulled thing further than this past where it should be (stuck behind
/// something as you go): the hook lets go.
const STRAIN: f32 = 24.0;
/// The rope's line blocked this long (something between you and what it
/// holds): it lets go.
const BLOCKED: f32 = 0.25;
/// Letting go with a jump: this share of a jump's speed up, at least.
const LEAP: f32 = 1.0;
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

#[derive(Clone, Debug, Default, PartialEq)]
enum Line {
    #[default]
    Stowed,
    /// Thrown: where the hook is, how it's going, how much rope is out.
    Out { tip: Vec2, vel: Vec2, paid: f32 },
    /// Coming back, having missed.
    Back { tip: Vec2 },
    /// Holding: what, whether it's pulling that thing to you (else you to
    /// it), and how long the line's been blocked.
    Held { anchor: Anchor, leash: bool, blocked: f32 },
}

/// A creature's grappling hook: the rope's state, and what `move_creatures`
/// needs from it (where it's pulling the body to).
#[derive(Component, Default)]
pub struct Rope {
    line: Line,
    held: bool,
    /// Where the hook was a tick ago (drawing it between ticks).
    prev_tip: Vec2,
    pull: Option<Pull>,
    /// The hook it threw (how its rope looks).
    look: HookDef,
}

/// Where a hook is pulling its holder: to `to` at `speed`, stopping `stop`
/// short of it (and hanging there).
#[derive(Clone, Copy, Debug)]
pub struct Pull {
    pub to: Vec2,
    pub speed: f32,
    pub stop: f32,
}

impl Pull {
    /// The velocity that pulls a body at `pos` in (none, hanging).
    pub fn vel(&self, pos: Vec2, dt: f32) -> Vec2 {
        let d = self.to - pos;
        let dist = d.length();
        if dist <= self.stop + 0.01 {
            return Vec2::ZERO;
        }
        d / dist * self.speed.min((dist - self.stop) / dt)
    }
}

impl Rope {
    /// Held by what stays put: where it pulls you.
    pub fn pull(&self) -> Option<Pull> {
        self.pull
    }

    /// What the rope's doing, for scenarios: "stowed", "out", "back",
    /// "pull" (you to it), "hang" (there) or "leash" (it to you).
    pub fn state(&self, pos: Vec2) -> &'static str {
        match &self.line {
            Line::Stowed => "stowed",
            Line::Out { .. } => "out",
            Line::Back { .. } => "back",
            Line::Held { leash: true, .. } => "leash",
            Line::Held { .. } => match self.pull {
                Some(p) if p.vel(pos, DT) == Vec2::ZERO => "hang",
                _ => "pull",
            },
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

type Holder<'a> = (Entity, &'a Controls, &'a Equipment, Option<&'a mut Rope>);
type Thing<'a> = (Entity, &'a mut Kinematics, Option<&'a MoveStats>, Has<Creature>, Has<Container>);

/// Each tick, before bodies move: throw, fly, take hold, pull, let go.
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
                r.pull = None;
            }
            continue;
        };
        let Some(mut rope) = rope else {
            commands.entity(e).try_insert(Rope::default());
            continue;
        };
        let rope = &mut *rope;
        rope.look = def.clone();
        rope.pull = None;
        let intent = controls.0;
        let pressed = intent.hook && !rope.held;
        rope.held = intent.hook;
        let Ok((_, me, my_stats, ..)) = things.get(e) else { continue };
        let (pos, half, facing) = (me.body.pos, me.body.half, me.loco.facing);
        let my_size = half.x * half.y;
        let stats = my_stats.map(|s| s.0.clone()).unwrap_or_default();
        let hand = pos + Vec2::new(facing * 2.0, half.y * 0.3);
        if pressed {
            let aim = if intent.aim != Vec2::ZERO { intent.aim } else { hand + Vec2::new(facing, 1.0) };
            let dir = (aim - hand).normalize_or(Vec2::new(facing, 0.0));
            rope.line = Line::Out { tip: hand, vel: dir * def.speed, paid: 0.0 };
            rope.prev_tip = hand;
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
                        rope.line = Line::Held { anchor, leash, blocked: 0.0 };
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
            Line::Held { anchor, leash, mut blocked } => {
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
                // Jump: let go, with a full jump.
                if !leash
                    && let Ok((_, mut k, ..)) = things.get_mut(e)
                    && k.loco.jump_pressed(&intent)
                {
                    let k = &mut *k;
                    k.loco.leap(&stats, &intent, &mut k.body, LEAP);
                    rope.line = Line::Back { tip: point };
                    continue;
                }
                // Something between you and it: it lets go.
                blocked = if first_solid(&grid, point, pos, 1.0).is_some() { blocked + DT } else { 0.0 };
                if blocked > BLOCKED {
                    rope.line = Line::Back { tip: point };
                    continue;
                }
                let d = (pos - point).normalize_or(Vec2::NEG_Y);
                if leash {
                    let Anchor::Body { entity, off } = anchor else { unreachable!("a leash holds a body") };
                    // Pulled to you, as you'd be to it.
                    if let Ok((_, mut k, _, creature, _)) = things.get_mut(entity) {
                        let k = &mut *k;
                        let there = k.body.pos + off;
                        let stop = (half.x * d.x).abs() + (half.y * d.y).abs() + (k.body.half.x * d.x).abs() + (k.body.half.y * d.y).abs() + SHORTEST;
                        let dist = there.distance(pos);
                        if dist > def.length + STRAIN {
                            rope.line = Line::Back { tip: point };
                            continue;
                        }
                        let length = (dist - def.reel * DT).max(stop);
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
                    // You to it: straight in, then hanging just off it.
                    let stop = (half.x * d.x).abs() + (half.y * d.y).abs() + SHORTEST;
                    rope.pull = Some(Pull { to: point, speed: def.reel, stop });
                }
                rope.line = Line::Held { anchor, leash, blocked };
            }
        }
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
        let tip = match &rope.line {
            Line::Stowed => continue,
            Line::Out { tip, .. } | Line::Back { tip } => rope.prev_tip.lerp(*tip, a),
            Line::Held { anchor, .. } => match anchor {
                Anchor::Cell { at, .. } => *at,
                Anchor::Body { entity, off } => bodies.get(*entity).map_or(rope.prev_tip, |t| t.translation.truncate() + *off),
            },
        };
        let points = [tip, hand];
        let (r, g, b) = def.rope;
        let dark = [(r as f32 * 0.62) as u8, (g as f32 * 0.62) as u8, (b as f32 * 0.62) as u8, 255];
        let light = [r, g, b, 255];
        let steps = (tip.distance(hand) * 2.0).ceil().max(1.0) as i32;
        let (mut n, mut last) = (0u32, IVec2::MAX);
        for s in 0..=steps {
            let c = tip.lerp(hand, s as f32 / steps as f32).floor().as_ivec2();
            if c == last {
                continue;
            }
            last = c;
            n += 1;
            // Twisted rope: light and dark by turns; a chain: links.
            let col = if def.links { if (n / 2).is_multiple_of(2) { light } else { dark } } else if n.is_multiple_of(3) { dark } else { light };
            px.put(c.x, c.y, col);
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
