//! Finding the way, for creatures (`platypus_nav`, DESIGN §14.4): each kind
//! of creature's profile (read from its file: size, movement, how far it
//! may fall), its way planned when the straight line won't do, and followed
//! a step at a time as a brain would press keys: walk, step off, jump (held
//! as the arc was), climb, fly, swim. Brains ask (`Ways::steer`); the
//! hunter does when what it's after can't be walked straight to.
//!
//! A way is planned again when what it's going for has moved off it, when
//! it's been a while and it didn't reach, when the creature's off it, or
//! when it's made no headway for a while. The grid's tiles are made again
//! when their chunk's cells change, no more often than every `REMAKE`
//! ticks (falling sand changes a chunk every tick). With the arena's
//! overlays on (Y), every way is drawn.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use platypus_nav::{Kind, Move, NODE, Nav, NodePos, Profile, find_until, node_feet, node_of};

use crate::creatures::def::{CreatureDef, Creatures};
use crate::creatures::{Creature, Kinematics, WorldGrid};
use crate::world::{SimWorld, TickSet};

/// Ticks a chunk's tiles are kept, at least, once made again.
const REMAKE: u64 = 15;
/// Ticks before a way that didn't reach is planned again; with no headway
/// (the same node, on the ground) this long, it's planned again too.
const REPLAN: u64 = 60;
const STUCK: u64 = 75;
/// Nodes a search may look at.
const BUDGET: usize = 2500;
/// What it's going for, moved this many nodes off the way's end: planned
/// again.
const MOVED: i32 = 3;
/// Planning a tick may spend (µs): past it, the rest plan next tick (a
/// first search over new ground works out every node's moves: slower).
const TICK_BUDGET: u64 = 2000;

pub struct WayPlugin;

impl Plugin for WayPlugin {
    fn build(&self, app: &mut App) {
        assert_eq!(platypus_nav::NODE * platypus_nav::TILE, platypus_sim::CHUNK, "a nav tile is a chunk");
        app.init_resource::<Ways>().add_systems(FixedUpdate, forget_changed.after(TickSet::Cells)).add_systems(Update, (draw, reread));
    }
}

/// Everyone's ways: the grid, each kind's profile, and what it cost.
/// (`PLATYPUS_NONAV=1`: none: everything goes straight at what it's
/// after, as before, to compare.)
#[derive(Resource)]
pub struct Ways {
    off: bool,
    pub nav: Nav,
    profiles: HashMap<String, Arc<Profile>>,
    /// The tempo they were made at (`tempo.rs`: creatures move at its pace;
    /// a jump's arc and hold with it).
    pace: f32,
    remade: HashMap<IVec2, u64>,
    /// Searches made, nodes they looked at, and the time (µs), since the
    /// readouts last took them.
    pub searches: u32,
    pub looked: usize,
    pub micros: u64,
    /// This tick's planning so far (the tick, µs).
    spent: (u64, u64),
}

impl Default for Ways {
    fn default() -> Self {
        Ways { off: std::env::var("PLATYPUS_NONAV").is_ok_and(|v| !v.is_empty()), nav: Nav::default(), profiles: HashMap::new(), pace: 1.0, remade: HashMap::new(), searches: 0, looked: 0, micros: 0, spent: (0, 0) }
    }
}

/// The kinds' files changed, or the tempo: their profiles made again.
fn reread(creatures: Res<Creatures>, tempo: Res<crate::tempo::Tempo>, mut ways: ResMut<Ways>) {
    let pace = tempo.pace();
    if (creatures.is_changed() && !creatures.is_added()) || (pace - ways.pace).abs() > 1e-4 {
        ways.pace = pace;
        ways.forget_profiles();
    }
}

/// One creature's way.
#[derive(Component, Default, Debug)]
pub struct Way {
    /// Where it's going (the node), and the way: where it began, each step
    /// the node reached and how; whether it gets there; the next step; when
    /// it was made.
    goal: Option<NodePos>,
    start: NodePos,
    pub path: Vec<(NodePos, Move)>,
    whole: bool,
    i: usize,
    made: u64,
    /// The jump key held until (tick), and which way the jump goes.
    hold_until: u64,
    flight: f32,
    /// The node it was last at, since when (headway).
    at: NodePos,
    since: u64,
}

/// What following the way asks of the body this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Steer {
    pub move_x: f32,
    pub move_y: f32,
    /// A new press of jump, or the key held (a jump's rise).
    pub jump: bool,
    pub hold: bool,
}

impl Ways {
    /// A kind's profile (made the first time it's asked for), as it moves
    /// at the game's tempo.
    pub fn profile(&mut self, kind: &str, def: &CreatureDef, tempo: &crate::tempo::Tempo) -> Arc<Profile> {
        self.profiles
            .entry(kind.to_string())
            .or_insert_with(|| {
                // (No fall damage: it'll drop a long way.)
                let drop = def.fall_damage.map_or(150.0, |f| f.safe_height);
                Arc::new(Profile::new(def.size, &tempo.apply(&def.movement, false), drop))
            })
            .clone()
    }

    /// The kinds' profiles again (their files changed).
    pub fn forget_profiles(&mut self) {
        self.profiles.clear();
    }

    /// The way to `goal` (a point its feet should reach), followed a tick:
    /// what to press. None: no way's known (go straight at it).
    #[allow(clippy::too_many_arguments)]
    pub fn steer(&mut self, world: &platypus_sim::World, creatures: &Creatures, tempo: &crate::tempo::Tempo, c: &Creature, k: &Kinematics, way: &mut Way, goal: Vec2, tick: u64) -> Option<Steer> {
        if self.off {
            return None;
        }
        let def = creatures.get(&c.kind)?;
        let p = self.profile(&c.kind, def, tempo);
        let feet = k.body.pos - Vec2::Y * k.body.half.y;
        let at = node_of(feet + Vec2::Y * 0.5);
        let to = node_of(goal + Vec2::Y * 0.5);
        let grounded = k.loco.grounded() || k.loco.clinging().is_some();
        // Headway.
        if at != way.at {
            way.at = at;
            way.since = tick;
        }
        let stuck = grounded && tick.saturating_sub(way.since) > STUCK && tick.saturating_sub(way.hold_until) > STUCK;
        // Where on the way it is: the nearest step from here on, reached
        // (within a node: a flyer cuts across them).
        let near = way.path.iter().enumerate().skip(way.i).min_by_key(|(_, (n, _))| (*n - at).abs().max_element());
        if let Some((j, (n, _))) = near
            && (*n - at).abs().max_element() <= 1
            && (*n == at || !grounded || p.fly > 0.0)
        {
            way.i = j + 1;
        }
        let off = grounded && way.i > 0 && way.i <= way.path.len() && {
            let prev = way.path[way.i - 1].0;
            let next = way.path.get(way.i).map_or(prev, |s| s.0);
            (at - prev).abs().max_element() > 1 && (at - next).abs().max_element() > 1
        };
        let moved = way.goal.is_none_or(|g| (g - to).abs().max_element() > MOVED);
        let stale = !way.whole && tick.saturating_sub(way.made) > REPLAN;
        let airborne = tick < way.hold_until || !grounded;
        if self.spent.0 != tick {
            self.spent = (tick, 0);
        }
        let room = self.spent.1 < TICK_BUDGET;
        if !airborne && room && (moved || stale || off || stuck) {
            let started = std::time::Instant::now();
            let grid = WorldGrid(world);
            let mut v = self.nav.view(&grid, p.size);
            // (No more than what's left of the tick's planning.)
            let deadline = started + std::time::Duration::from_micros(TICK_BUDGET.saturating_sub(self.spent.1).max(200));
            let path = find_until(&mut v, &p, at, to, 1, BUDGET, Some(deadline));
            self.searches += 1;
            self.looked += path.looked;
            let took = started.elapsed().as_micros() as u64;
            if took > 3000 {
                warn!("way: a {} search took {:.1} ms ({} nodes, {} tiles made so far)", c.kind, took as f32 / 1000.0, path.looked, self.nav.built);
            }
            self.micros += took;
            self.spent.1 += took;
            if std::env::var("PLATYPUS_WAYLOG").is_ok() {
                let steps: Vec<String> = path.steps.iter().map(|(n, m)| format!("{:?}>{n}", m.kind)).collect();
                info!("way: {} {at} -> {to}: whole {}, looked {}, {}", c.kind, path.whole, path.looked, steps.join(" "));
            }
            // A way that doesn't get there goes no further than it could
            // come back from: not down into a pit it can't jump out of
            // (it waits at the edge, and looks again).
            let mut steps = path.steps;
            if !path.whole
                && let Some(cut) = steps.iter().position(|(_, m)| m.d.y < 0 && -m.d.y > p.back_up())
            {
                steps.truncate(cut);
            }
            let start = platypus_nav::search::settle(&mut self.nav.view(&grid, p.size), &p, at).unwrap_or(at);
            *way = Way { goal: Some(to), start, whole: path.whole, path: steps, i: 0, made: tick, hold_until: 0, flight: 0.0, at, since: if stuck { tick } else { way.since } };
        }
        let (n, m) = *way.path.get(way.i)?;
        let aim = node_feet(n);
        let toward = |d: f32| if d.abs() < 1.0 { 0.0 } else { d.signum() };
        let mut s = Steer::default();
        // In a jump: keep going its way, the key held as long as the arc was.
        if tick < way.hold_until || (!grounded && way.flight != 0.0) {
            s.move_x = way.flight;
            s.hold = tick < way.hold_until;
            if grounded && tick >= way.hold_until {
                way.flight = 0.0;
            }
            return Some(s);
        }
        // (A climber still on a wall, its next step a walk over the lip:
        // climbed, up and over.)
        let on_wall = p.climb && k.loco.clinging().is_some_and(|f| f.x != 0.0);
        let kind = if on_wall && m.kind == Kind::Walk { Kind::Climb } else { m.kind };
        match kind {
            Kind::Walk | Kind::Drop => s.move_x = toward(aim.x - feet.x),
            Kind::Jump(j) => {
                // At the take-off: jump, held as the recorded arc was.
                let from = if way.i == 0 { way.start } else { way.path[way.i - 1].0 };
                let jump = &p.jumps[j as usize];
                // (From the middle of the take-off node, as its arc was
                // recorded: not its near edge.)
                let dir = m.d.x.signum() as f32;
                let centred = (feet.x - node_feet(from).x) * dir >= -0.5;
                // (And going its way, as it was: a standing jump not still
                // moving the other way, a running one at a run.)
                let going = k.body.vel.x * dir;
                let ready = if jump.run_up { going > 0.5 * p.run } else { going > -8.0 };
                if at == from && grounded && centred && !ready && !jump.run_up {
                    // Stop first.
                    return Some(Steer::default());
                }
                if at == from && grounded && centred {
                    if std::env::var("PLATYPUS_WAYLOG").is_ok() {
                        info!("way: jump from {from} to {} (arc to {:?}, hold {:.2}, run-up {}), feet {:?} vel {:?}", n, jump.to, jump.hold, jump.run_up, feet.round(), k.body.vel.round());
                    }
                    s.jump = true;
                    s.hold = true;
                    way.flight = m.d.x.signum() as f32;
                    way.hold_until = tick + (jump.hold * crate::world::TICK_HZ as f32).ceil() as u64 + 1;
                    s.move_x = way.flight;
                } else {
                    s.move_x = toward(node_feet(from).x - feet.x);
                }
            }
            Kind::Climb => {
                // (Toward the furthest of the next few climbing steps: one
                // step's a node, too near to steer by.)
                let ahead = way.path[way.i..].iter().take(4).take_while(|(_, m)| m.kind == Kind::Climb || on_wall).last().map_or(aim, |(n, _)| node_feet(*n));
                let d = ahead + Vec2::Y * k.body.half.y - k.body.pos;
                s.move_x = toward(d.x);
                s.move_y = toward(d.y);
                // Going up or down a wall: pressed into it (a climber holds
                // on to what it pushes against), the one it holds if it does.
                if let Some(f) = k.loco.clinging()
                    && f.x != 0.0
                    && d.x.abs() < NODE as f32
                {
                    s.move_x = f.x;
                } else if d.x.abs() < NODE as f32 && d.y.abs() >= 1.0 {
                    let grid = WorldGrid(world);
                    use platypus_physics::Grid;
                    let side = |dir: f32| {
                        let x = (k.body.pos.x + dir * (k.body.half.x + 1.5)) as i32;
                        [k.body.pos.y - k.body.half.y + 1.0, k.body.pos.y, k.body.pos.y + k.body.half.y - 1.0].iter().any(|y| grid.solid(x, *y as i32))
                    };
                    if side(1.0) {
                        s.move_x = 1.0;
                    } else if side(-1.0) {
                        s.move_x = -1.0;
                    }
                }
            }
            Kind::Fly | Kind::Swim => {
                let d = (aim + Vec2::Y * k.body.half.y - k.body.pos).normalize_or_zero();
                s.move_x = d.x;
                s.move_y = d.y;
            }
        }
        if std::env::var("PLATYPUS_WAYLOG").is_ok() && tick.is_multiple_of(60) {
            info!("way: {} at {at} step {:?}>{n} steer {s:?} clinging {:?} grounded {grounded} vel {:?}", c.kind, m.kind, k.loco.clinging(), k.body.vel.round());
        }
        Some(s)
    }
}

/// Whether a body could walk straight there: no more than a step up or
/// down over the whole way, nothing in the way at its middle, ground under
/// it all along (no gap wider than a node).
pub fn straight(world: &platypus_sim::World, from: Vec2, to: Vec2, half: Vec2, step: f32) -> bool {
    if (to.y - from.y).abs() > step * 2.0 + 2.0 {
        return false;
    }
    let grid = WorldGrid(world);
    use platypus_physics::Grid;
    let n = ((to.x - from.x).abs() / 2.0).ceil() as i32;
    let mut gap = 0;
    (0..=n).all(|i| {
        let x = from.x + (to.x - from.x) * i as f32 / n.max(1) as f32;
        let y = from.y + (to.y - from.y) * i as f32 / n.max(1) as f32;
        let clear = !grid.solid(x as i32, (y + half.y) as i32);
        let ground = (1..=(step as i32 + 3)).any(|d| grid.occupancy(x as i32, y as i32 - d) != platypus_physics::Occupancy::Empty);
        gap = if ground { 0 } else { gap + 2 };
        clear && gap <= platypus_nav::NODE
    })
}

/// Chunks whose cells changed: their tiles made again when next needed
/// (no more often than every `REMAKE` ticks).
fn forget_changed(sim: Res<SimWorld>, mut ways: ResMut<Ways>) {
    let tick = sim.world.tick();
    let ways = &mut *ways;
    let mut gone = Vec::new();
    for c in sim.world.chunks() {
        if !c.nav_dirty() {
            continue;
        }
        let t = IVec2::new(c.pos.x, c.pos.y);
        if ways.remade.get(&t).is_some_and(|&at| tick < at + REMAKE) {
            continue;
        }
        gone.push(t);
        ways.remade.insert(t, tick);
        c.clear_nav_dirty();
    }
    // (All at once: one pass over what's kept.)
    ways.nav.forget_all_of(&gone);
}

/// Every way, drawn with the arena's overlays (walking white, jumps gold,
/// drops blue, climbing green, flying and swimming cyan).
fn draw(view: Res<crate::arena::ArenaView>, mut gizmos: Gizmos, q: Query<&Way>) {
    if !view.overlays {
        return;
    }
    for w in &q {
        let mut from: Option<Vec2> = None;
        for (i, (n, m)) in w.path.iter().enumerate() {
            let p = node_feet(*n) + Vec2::Y;
            if let Some(f) = from {
                let color = match m.kind {
                    Kind::Walk => Color::srgba(1.0, 1.0, 1.0, 0.6),
                    Kind::Jump(_) => Color::srgb(1.0, 0.8, 0.2),
                    Kind::Drop => Color::srgb(0.4, 0.6, 1.0),
                    Kind::Climb => Color::srgb(0.4, 1.0, 0.5),
                    Kind::Fly | Kind::Swim => Color::srgb(0.3, 0.9, 1.0),
                };
                let color = if i < w.i { color.with_alpha(0.25) } else { color };
                gizmos.line_2d(f, p, color);
            }
            from = Some(p);
        }
    }
}
