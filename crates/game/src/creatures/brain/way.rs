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
use platypus_nav::{Kind, Move, NODE, Nav, NodePos, Profile, Search, node_feet, node_of};

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
        app.init_resource::<Ways>()
            .add_systems(FixedUpdate, forget_changed.after(TickSet::Cells))
            .add_systems(FixedUpdate, dig.in_set(TickSet::Bodies).before(crate::creatures::move_creatures))
            .add_systems(Update, (draw, reread));
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
    /// Where it's digging (a node: its body's room there), if it is;
    /// whether the way digs down there; and the feet its hole is dug round
    /// (along: the node's column at its own feet' height, so it tunnels on
    /// level and doesn't climb the face for a way a few cells up).
    pub dig: Option<NodePos>,
    pub dig_down: bool,
    pub dig_feet: Vec2,
    /// A search under way (the tick's planning ran out before it was
    /// done: it goes on next tick, the way it has followed meanwhile).
    search: Option<Box<Search>>,
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
                let p = Profile::new(def.size, &tempo.apply(&def.movement, false), drop);
                Arc::new(match &def.dig {
                    Some(d) => p.digging(d.digging()),
                    None => p,
                })
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
        // (Digging, it stays a while where it is: that's headway too.)
        let digging = way.dig.is_some();
        let stuck = grounded && !digging && tick.saturating_sub(way.since) > STUCK && tick.saturating_sub(way.hold_until) > STUCK;
        // Where on the way it is: the nearest step from here on, reached
        // (within a node: a flyer cuts across them).
        let near = way.path.iter().enumerate().skip(way.i).min_by_key(|(_, (n, _))| (*n - at).abs().max_element());
        // (A dig step: in its column, a few nodes up or down is there: it
        // digs along at its own height, not up its face for a way a little
        // higher. A climb step, for a climbing digger: a node off is there
        // (on its feet on a pile of its own crumbs, it can't get to the
        // node exactly; any other climber can, and skipping it a node off
        // had a centipede miss its way round an overhang).)
        if let Some((j, (n, m))) = near
            && ((*n - at).abs().max_element() <= 1 && (*n == at || !grounded || p.fly > 0.0 || p.climb && p.dig.is_some() && m.kind == Kind::Climb) || m.kind == Kind::Dig && n.x == at.x && (n.y - at.y).abs() <= ALONG_SLACK)
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
        // (One under way that's after somewhere it no longer is: dropped.)
        if way.search.as_ref().is_some_and(|s| (s.to - to).abs().max_element() > MOVED) {
            way.search = None;
        }
        let want = moved || stale || off || stuck;
        if !airborne && room && (way.search.is_some() || want) {
            let started = std::time::Instant::now();
            let grid = WorldGrid(world);
            let mut v = self.nav.view(&grid, p.size);
            // (No more than what's left of the tick's planning.)
            let deadline = started + std::time::Duration::from_micros(TICK_BUDGET.saturating_sub(self.spent.1).max(200));
            let mut search = way.search.take().unwrap_or_else(|| Box::new(Search::new(&mut v, &p, at, to, 1, BUDGET)));
            let before = search.looked();
            let done = search.run(&mut v, &p, Some(deadline));
            let took = started.elapsed().as_micros() as u64;
            if took > 3000 {
                warn!("way: a {} search took {:.1} ms ({} nodes)", c.kind, took as f32 / 1000.0, search.looked() - before);
            }
            self.looked += search.looked() - before;
            self.micros += took;
            self.spent.1 += took;
            match done {
                None => way.search = Some(search),
                Some(path) => {
                    self.searches += 1;
                    if waylog() {
                        let steps: Vec<String> = path.steps.iter().map(|(n, m)| format!("{:?}>{n}", m.kind)).collect();
                        info!("way: {} {} -> {to}: whole {}, looked {}, {}", c.kind, search.start, path.whole, path.looked, steps.join(" "));
                    }
                    // A way that doesn't get there goes no further than it
                    // could come back from: not down into a pit it can't
                    // jump out of (it waits at the edge, and looks again).
                    let mut steps = path.steps;
                    if !path.whole
                        && let Some(cut) = steps.iter().position(|(_, m)| m.d.y < 0 && -m.d.y > p.back_up())
                    {
                        steps.truncate(cut);
                    }
                    // (Cut short with nowhere to go, it keeps the way it has.)
                    if steps.is_empty() && !path.whole && way.path.len() > way.i {
                        way.made = tick;
                        way.goal = Some(to);
                    } else {
                        *way = Way { goal: Some(to), start: search.start, whole: path.whole, path: steps, i: 0, made: tick, hold_until: 0, flight: 0.0, at, since: if stuck { tick } else { way.since }, dig: None, dig_down: false, dig_feet: Vec2::ZERO, search: None };
                    }
                }
            }
        }
        way.dig = None;
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
        // (Digging, it digs there (the dig system); a climber goes at it as
        // it climbs, the rest as they walk.)
        if m.kind == Kind::Dig {
            way.dig = Some(n);
            way.dig_down = m.d.y < 0;
            way.dig_feet = if m.d.y == 0 && (n.y - at.y).abs() <= ALONG_SLACK { Vec2::new(node_feet(n).x, feet.y) } else { node_feet(n) };
        }
        let kind = match m.kind {
            Kind::Walk if on_wall => Kind::Climb,
            // (A climber digging up, or holding on to something (a wall, a
            // ceiling), digs as it climbs; on its feet on the ground digging
            // along, it stands to the face.)
            Kind::Dig if p.climb && (m.d.y > 0 || !k.loco.grounded()) => Kind::Climb,
            k => k,
        };
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
                    if waylog() {
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
                let ahead = way.path[way.i..].iter().take(4).take_while(|(_, m)| matches!(m.kind, Kind::Climb | Kind::Dig) || on_wall).last().map_or(aim, |(n, _)| node_feet(*n));
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
                }
                // (At a lip, the way on over the top: up a little first.)
                if on_wall && s.move_y == 0.0 && m.kind == Kind::Walk {
                    s.move_y = 1.0;
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
            // Digging: at it (the dig system scrapes and spits), pressing
            // into it.
            Kind::Dig => {
                let d = aim + Vec2::Y * k.body.half.y - k.body.pos;
                s.move_x = toward(d.x);
                // (Digging along: level, at its own height. Down: down to
                // the hole. Up is a climber's (`Kind::Climb`).)
                s.move_y = if m.d.y < 0 && d.y < -1.0 { -1.0 } else { 0.0 };
            }
            Kind::Fly | Kind::Swim => {
                let d = (aim + Vec2::Y * k.body.half.y - k.body.pos).normalize_or_zero();
                s.move_x = d.x;
                s.move_y = d.y;
            }
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

/// Plans and jumps logged (`PLATYPUS_WAYLOG`), for finding out why a
/// creature went the way it did.
fn waylog() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("PLATYPUS_WAYLOG").is_ok())
}

/// Cells a tick's digging (everyone's) may take, at most.
const DIG_CAP: u32 = 400;

/// A digger's face as its legs see it (a legged body, `legs.rs`): cells to
/// strike at (spread over what's in its way, nearest first), till when
/// it's digging; and where its claws struck since the last tick (the dig
/// takes what's round each strike: the cells go where the claw goes).
#[derive(Component, Default)]
pub struct DigFace {
    pub face: Vec<Vec2>,
    pub until: f32,
    pub strikes: Vec<Vec2>,
    /// Each digging leg's last stroke struck (its count).
    pub struck: [i64; 4],
}

/// Digging along, a way this many nodes up or down from its feet is dug at
/// its own height (and its step counts as reached in its column).
const ALONG_SLACK: i32 = 3;

/// A dug hole's room past the digger's body, all round (cells).
const ROOM_PAD: f32 = 5.0;

/// What digs: its kind, body and way; what it has going; its face (for its
/// legs, if it has them).
type Digging<'a> = (Entity, &'a Creature, &'a Kinematics, &'a Way, Option<&'a mut Digger>, Option<&'a mut DigFace>, Has<crate::creatures::body::legs::Legs>);

/// A claw's strike takes what's within this of its tip (cells).
const STRIKE: f32 = 4.5;
/// Digging owed past this (seconds) and the claws haven't struck it: the
/// nearest goes anyway (a dig never stalls on its animation).
const OWED_MOST: f32 = 0.6;

/// What a digger has going (the digging owed it, seconds; its next spit,
/// its next scratch).
#[derive(Component, Default)]
pub struct Digger {
    owed: f32,
    spit_at: f32,
    sound_at: f32,
}

/// Diggers at it (DESIGN §14.4): the cells in their way where they're
/// going (`Way::dig`), nearest first, each taking as long as the planner
/// counted it (`Digging::secs`: claws where they can, harder slower; acid
/// where they can't, and nothing inert). Claws throw a pinch of what they
/// scrape out, as what it crumbles into, scratching; acid is spat at the
/// face every so often, hissing (a few drops: the eating is the digger's,
/// so a face eats as fast sideways as down).
/// No more than `DIG_CAP` cells' worth of digging a tick.
#[allow(clippy::too_many_arguments)]
fn dig(
    mut commands: Commands,
    time: Res<Time>,
    mut sim: ResMut<SimWorld>,
    creatures: Res<Creatures>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut q: Query<Digging>,
) {
    const DT: f32 = 1.0 / crate::world::TICK_HZ as f32;
    let now = time.elapsed_secs();
    let mut cap = DIG_CAP as f32;
    for (e, c, k, way, digger, face, legged) in &mut q {
        let Some(_) = way.dig else {
            if let Some(mut f) = face
                && !f.face.is_empty()
            {
                f.face.clear();
                f.strikes.clear();
            }
            continue;
        };
        let Some(def) = creatures.get(&c.kind).and_then(|d| d.dig.clone()) else { continue };
        let Some(mut dg) = digger else {
            commands.entity(e).insert(Digger::default());
            continue;
        };
        // (A legged body digs with its legs: its face, for them.)
        let Some(mut face) = face.filter(|_| legged).or(None) else {
            if legged {
                commands.entity(e).insert(DigFace::default());
                continue;
            }
            dig_by_reach(&mut sim, k, way.dig_feet, way.dig_down, &def, &mut dg, &mut cap, now, &mut sounds);
            continue;
        };
        // (Only at the face: its body by the hole it's digging.)
        if !at_face(k, way.dig_feet) {
            continue;
        }
        let digging = def.digging();
        let solid = dig_room(&sim, k, way.dig_feet, &digging, way.dig_down);
        if solid.is_empty() || cap <= 0.0 {
            dg.owed = 0.0;
            face.face.clear();
            face.strikes.clear();
            continue;
        }
        dg.owed = (dg.owed + DT).min(solid[0].1.max(1.0));
        // Its face for its legs: what its claws can take, spread nearest to
        // furthest (each stroke at the next).
        let clawable: Vec<&Cut> = solid.iter().filter(|s| s.3).collect();
        face.face = (0..FACE_POINTS.min(clawable.len())).map(|j| mid(&clawable[j * clawable.len() / FACE_POINTS.min(clawable.len()).max(1)].0)).collect();
        face.until = now + 0.25;
        let mats = sim.world.materials().clone();
        let (mut scraped, mut eaten) = (None, None);
        let take = |sim: &mut SimWorld, at: platypus_sim::CellPos, secs: f32, owed: &mut f32, cap: &mut f32| -> bool {
            if *owed < secs || *cap < 1.0 || !sim.world.is_solid(at) {
                return false;
            }
            *owed -= secs;
            *cap -= 1.0;
            sim.world.set(at, platypus_sim::Cell::AIR);
            sim.world.note_broken(at);
            true
        };
        // Each strike takes what's round the claw's tip.
        for strike in std::mem::take(&mut face.strikes) {
            let mut near: Vec<&&Cut> = clawable.iter().filter(|s| mid(&s.0).distance(strike) <= STRIKE).collect();
            near.sort_by(|a, b| mid(&a.0).distance_squared(strike).total_cmp(&mid(&b.0).distance_squared(strike)));
            for &&&(at, secs, m, _) in &near {
                if take(&mut sim, at, secs, &mut dg.owed, &mut cap) {
                    scraped = scraped.or(Some((at, m)));
                } else if dg.owed < secs {
                    break;
                }
            }
        }
        // What claws can't take, acid eats (nearest first); and owed piling
        // up (strikes not keeping up), the nearest goes anyway.
        for &(at, secs, m, clawed) in &solid {
            if clawed && dg.owed < OWED_MOST {
                continue;
            }
            if take(&mut sim, at, secs, &mut dg.owed, &mut cap) {
                if clawed {
                    scraped = scraped.or(Some((at, m)));
                } else {
                    eaten = eaten.or(Some(at));
                }
            } else if dg.owed < secs || cap < 1.0 {
                break;
            }
        }
        dug(&mut sim, &mats, &def, &mut dg, now, scraped, eaten, &mut sounds);
    }
}

/// Its body by the hole it's digging (within its size and two nodes): a
/// digger doesn't dig what's out of its reach.
fn at_face(k: &Kinematics, feet: Vec2) -> bool {
    let middle = feet + Vec2::Y * k.body.half.y;
    (middle - k.body.pos).abs().cmple(k.body.half + Vec2::splat(NODE as f32 * 2.0)).all()
}

/// A cell in a digger's way: where, seconds to dig it, what it is, and
/// whether claws take it (else acid).
type Cut = (platypus_sim::CellPos, f32, platypus_sim::MaterialId, bool);

/// How many points of its face a legged digger strikes at in turn.
const FACE_POINTS: usize = 6;

fn mid(p: &platypus_sim::CellPos) -> Vec2 {
    Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5)
}

/// Its room at the node: the cells in its way it can dig, nearest first. A
/// round hole a little bigger than it all round (`ROOM_PAD`), so it isn't
/// wedged against the face, the roof and the floor at once; nothing below
/// its feet unless the way goes down (not the floor it walks on).
fn dig_room(sim: &SimWorld, k: &Kinematics, feet: Vec2, digging: &platypus_nav::Digging, down: bool) -> Vec<Cut> {
    let (xc, yb) = (feet.x as i32, feet.y.round() as i32);
    let (rx, ry) = (k.body.half.x + ROOM_PAD, k.body.half.y + ROOM_PAD);
    let mid_y = yb as f32 + k.body.half.y;
    let bottom = if down { (mid_y - ry).floor() as i32 } else { yb };
    let mats = sim.world.materials();
    let mut solid: Vec<Cut> = Vec::new();
    for x in (xc as f32 - rx).floor() as i32..=(xc as f32 + rx).ceil() as i32 {
        for y in bottom..=(mid_y + ry).ceil() as i32 {
            let (dx, dy) = ((x as f32 + 0.5 - xc as f32) / rx, (y as f32 + 0.5 - mid_y) / ry);
            if dx * dx + dy * dy > 1.0 {
                continue;
            }
            let p = platypus_sim::CellPos::new(x, y);
            if let Some(cell) = sim.world.get(p)
                && sim.world.is_solid(p)
            {
                let ph = mats.phys(cell.material);
                if let Some(secs) = digging.secs(ph.dig_hardness(), ph.inert) {
                    let clawed = digging.claws > 0 && ph.dig_hardness() <= digging.claws;
                    solid.push((p, secs, cell.material, clawed));
                }
            }
        }
    }
    let me = k.body.pos;
    solid.sort_by(|a, b| mid(&a.0).distance_squared(me).total_cmp(&mid(&b.0).distance_squared(me)));
    solid
}

/// A body without legs digs as it reaches: the nearest cells, each as its
/// time comes.
#[allow(clippy::too_many_arguments)]
fn dig_by_reach(sim: &mut SimWorld, k: &Kinematics, feet: Vec2, down: bool, def: &crate::creatures::def::DigDef, dg: &mut Digger, cap: &mut f32, now: f32, sounds: &mut MessageWriter<crate::sound::PlaySound>) {
    const DT: f32 = 1.0 / crate::world::TICK_HZ as f32;
    if !at_face(k, feet) {
        return;
    }
    let solid = dig_room(sim, k, feet, &def.digging(), down);
    if solid.is_empty() || *cap <= 0.0 {
        dg.owed = 0.0;
        return;
    }
    dg.owed = (dg.owed + DT).min(solid[0].1.max(1.0));
    let (mut scraped, mut eaten) = (None, None);
    for &(at, secs, m, clawed) in &solid {
        if dg.owed < secs || *cap < 1.0 {
            break;
        }
        dg.owed -= secs;
        *cap -= 1.0;
        sim.world.set(at, platypus_sim::Cell::AIR);
        sim.world.note_broken(at);
        if clawed {
            scraped = scraped.or(Some((at, m)));
        } else {
            eaten = eaten.or(Some(at));
        }
    }
    let mats = sim.world.materials().clone();
    dug(sim, &mats, def, dg, now, scraped, eaten, sounds);
}

/// What digging shows: claws throw a pinch of what they scrape out (as what
/// it crumbles into), scratching; acid's spat at the face now and then,
/// hissing.
#[allow(clippy::too_many_arguments)]
fn dug(sim: &mut SimWorld, mats: &platypus_sim::MaterialTable, def: &crate::creatures::def::DigDef, dg: &mut Digger, now: f32, scraped: Option<(platypus_sim::CellPos, platypus_sim::MaterialId)>, eaten: Option<platypus_sim::CellPos>, sounds: &mut MessageWriter<crate::sound::PlaySound>) {
    if let Some((at, m)) = scraped
        && now >= dg.sound_at
    {
        dg.sound_at = now + 0.3;
        let from = mid(&at);
        // (What it scrapes falls as what it crumbles into, a pinch, tossed
        // gently: not hard enough to pelt the digger itself.)
        let ph = mats.phys(m);
        if ph.crumbles_into != platypus_sim::MaterialId::AIR {
            sim.world.splash([from.x, from.y], ph.crumbles_into, 2, 0.8);
        }
        let name = if ph.hardness >= 40 { "mine_stone" } else { "mine_dirt" };
        sounds.write(crate::sound::PlaySound::at(name, from).volume(0.6));
    }
    if let (Some(at), Some(acid)) = (eaten, &def.acid)
        && now >= dg.spit_at
        && let Some(m) = mats.id(&acid.material)
    {
        dg.spit_at = now + acid.every;
        let from = mid(&at);
        sim.world.splash([from.x, from.y], m, 3, 1.0);
        sounds.write(crate::sound::PlaySound::at("pour", from).volume(0.7));
    }
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
                    Kind::Dig => Color::srgb(0.75, 0.5, 0.25),
                    Kind::Fly | Kind::Swim => Color::srgb(0.3, 0.9, 1.0),
                };
                let color = if i < w.i { color.with_alpha(0.25) } else { color };
                gizmos.line_2d(f, p, color);
            }
            from = Some(p);
        }
    }
}
