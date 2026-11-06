//! The cave murk: the murk's heavier kin, deep under the ground. Where the
//! space murk is a cloud, this one's a liquid: its motes have weight, so
//! it pours.
//!
//! - **It pours.** Its motes fall, press on each other (so it spreads and
//!   pools, taking the shape of what it's in) and are drawn to its heart.
//!   Its heart leads the way, a little ahead of its mass; the mass flows
//!   after it.
//! - **It falls.** Off a ledge, down a shaft, it just drops, splats flat
//!   and pulls itself back together.
//! - **It clings.** Motes against rock stick, so it creeps up walls and
//!   hangs from ceilings.
//! - **It squeezes.** Points, not cells: it finds its way through cracks
//!   a cell wide, a trickle at a time, where you can't follow.
//! - **Its edge smoulders** a dull ember (the space one's is violet).
//!
//! (Stage 1: how it moves. Not yet: what it does to you, what hurts it,
//! the lights it eats, where it lives.)
//!
//! Like the space murk, not in the cell grid: points, reading the grid for
//! rock, drawn a cell at a time on a canvas.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use platypus_sim::rng::hash;
use platypus_sim::{CellPos, Kind};

use crate::creatures::Kinematics;
use crate::creatures::player::LocalPlayer;
use crate::light::LightSource;
use crate::magic::runes::Emitter;
use crate::world::{SimWorld, TICK_HZ, TickSet};

pub struct CaveMurkPlugin;

/// Its name in the dev menu's Creatures tab.
pub const KIND: &str = "cave_murk";

impl Plugin for CaveMurkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CaveMurks>()
            .init_resource::<CaveCanvas>()
            .add_systems(FixedUpdate, live.in_set(TickSet::Bodies).after(crate::creatures::move_creatures))
            .add_systems(PostUpdate, show.before(bevy::transform::TransformSystems::Propagate));
    }
}

/// Motes in it.
const MOTES: usize = 760;
/// Its motes' weight (cells/s²; you fall at 1350).
const GRAVITY: f32 = 480.0;
/// Motes closer than this (cells) push apart, this hard; and drag each
/// other along (how thick it is, as a liquid).
const PRESS_R: f32 = 1.7;
const PRESS: f32 = 1400.0;
const THICKNESS: f32 = 9.0;
/// Drawn to its heart: so hard a cell off, no harder than at so far.
const PULL: f32 = 55.0;
const PULL_FAR: f32 = 18.0;
/// Against rock: what's left of its weight (it sticks), and how much it
/// drags (s⁻¹). Climbing, it gathers itself: none of its weight against
/// rock, this much off it (it hauls itself up its own film), drawn to its
/// heart so much harder.
const CLING: f32 = 0.12;
const CLIMB_WEIGHT: f32 = 0.18;
const CLIMB_PULL: f32 = 1.4;
const STICKY: f32 = 3.5;
/// Its heart: how fast it goes (cells/s), how hard it's held to its mass.
/// It goes on while so much of it's this near its heart (cells).
const KEEP: f32 = 0.22;
const KEEP_R: f32 = 10.0;
const CRAWL: f32 = 34.0;
const HELD: f32 = 6.0;
/// Waited this long (s) for its mass, its heart goes back to it.
const WAIT: f32 = 2.5;
/// Behind its heart (more than this far back along its way: its cost,
/// two a cell), it flows along its way this fast (cells/s), so hard.
const FLOW_BEHIND: u32 = 16;
const FLOW_FROM: f32 = 16.0;
const FLOW: f32 = 120.0;
const FLOW_K: f32 = 5.0;
/// Its heart's hollow (cells), and its glow.
const CORE_R: f32 = 3.5;
/// It looks for its way (to you) every so many ticks; it comes for you
/// from so far (cells).
const PATH_EVERY: u64 = 12;
/// Its motes it finds the way for, no further off its mass than this.
const SPREAD_MOST: i32 = 220;
const AGGRO: f32 = 260.0;
/// Its way: the point so many cells along it.
const AHEAD: usize = 14;
/// Falling this fast (cells/s), then stopped: a splat.
const SPLAT_FROM: f32 = 100.0;
/// Fastest a mote goes (cells/s).
const TOP: f32 = 520.0;

fn unit(seed: &[u64]) -> f32 {
    (hash(seed) % 100_000) as f32 / 100_000.0
}

/// Rock (or anything solid) at `p`; unloaded is rock too (it doesn't go
/// where the world isn't).
fn solid_cell(sim: &SimWorld, p: IVec2) -> bool {
    let mats = sim.world.materials();
    sim.world.get(CellPos::new(p.x, p.y)).is_none_or(|c| !c.is_air() && matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder))
}

fn solid(sim: &SimWorld, p: Vec2) -> bool {
    solid_cell(sim, p.floor().as_ivec2())
}

#[derive(Clone, Copy, Debug)]
struct Mote {
    pos: Vec2,
    vel: Vec2,
    /// Against rock last tick.
    stuck: bool,
    /// On a floor (rock under it) last tick: no sticking there, it slides.
    floor: bool,
}

pub struct CaveMurk {
    heart: Vec2,
    /// Its mass's middle (near its heart), and how fast that went.
    mass: Vec2,
    mass_vel: Vec2,
    motes: Vec<Mote>,
    /// Where it's going: the way there (cells, from its heart on), and
    /// what it was going to.
    way: Vec<IVec2>,
    goal: Option<Vec2>,
    /// Each cell round it, the next toward where it's going, and how far
    /// (what's behind its heart flows along this).
    field: Option<Field>,
    /// Its heart's falling this fast (cells/s).
    fall: f32,
    /// Its heart's waited for its mass this long.
    waited: f32,
    /// Just splatted (fading), where.
    splat: f32,
    light: Entity,
    seed: u64,
    age: f32,
}

#[derive(Resource, Default)]
pub struct CaveMurks {
    list: Vec<CaveMurk>,
    /// Where scenarios want it to go instead of to you.
    pub goal: Option<Vec2>,
}

impl CaveMurks {
    /// One at `at`, now.
    pub fn summon(&mut self, commands: &mut Commands, at: Vec2, seed: u64) {
        let light = commands.spawn((Name::new("Cave murk heart"), CaveLight, LightSource { color: HEART_LIGHT, flicker: 0.15 }, Transform::from_translation(at.extend(0.0)))).id();
        let r = (MOTES as f32 / std::f32::consts::PI).sqrt();
        let motes = (0..MOTES)
            .map(|i| {
                let i = i as u64;
                let pos = at + Vec2::from_angle(unit(&[seed, i, 1]) * std::f32::consts::TAU) * unit(&[seed, i, 2]).sqrt() * r;
                Mote { pos, vel: Vec2::ZERO, stuck: false, floor: false }
            })
            .collect();
        self.list.push(CaveMurk { heart: at, mass: at, mass_vel: Vec2::ZERO, motes, way: Vec::new(), goal: None, field: None, fall: 0.0, waited: 0.0, splat: 0.0, light, seed, age: 0.0 });
    }

    /// Each: its heart, its mass's middle, how many of its motes are
    /// against rock, its way's length (for the scenarios).
    pub fn about(&self) -> Vec<(Vec2, Vec2, usize, usize)> {
        self.list.iter().map(|m| (m.heart, m.mass, m.motes.iter().filter(|o| o.stuck).count(), m.way.len())).collect()
    }

    /// Each one's motes past `x` (for the scenarios: through the crack).
    /// Each one's cells covered by its motes (how spread out it is).
    pub fn cover(&self) -> Vec<usize> {
        self.list.iter().map(|m| m.motes.iter().map(|o| o.pos.floor().as_ivec2()).collect::<std::collections::HashSet<_>>().len()).collect()
    }

    pub fn past(&self, x: f32) -> Vec<usize> {
        self.list.iter().map(|m| m.motes.iter().filter(|o| o.pos.x > x).count()).collect()
    }
}

#[derive(Component)]
struct CaveLight;

/// The search's results: over the cells from `lo`, `w` across, each one's
/// next toward the goal and its cost there.
struct Field {
    lo: IVec2,
    w: usize,
    h: usize,
    next: Vec<u32>,
    cost: Vec<u32>,
}

impl Field {
    fn idx(&self, p: IVec2) -> Option<usize> {
        let q = p - self.lo;
        (q.x >= 0 && q.y >= 0 && (q.x as usize) < self.w && (q.y as usize) < self.h).then(|| q.y as usize * self.w + q.x as usize)
    }

    /// From `p`: the way on (a step), and how far it is to go.
    fn on(&self, p: Vec2) -> Option<(Vec2, u32)> {
        let i = self.idx(p.floor().as_ivec2())?;
        let n = self.next[i];
        if n == u32::MAX || n as usize == i {
            return None;
        }
        let q = self.lo + IVec2::new((n as usize % self.w) as i32, (n as usize / self.w) as i32);
        Some(((q.as_vec2() + Vec2::splat(0.5) - p).normalize_or_zero(), self.cost[i]))
    }
}

/// Its heart's glow: a dull ember.
const HEART_LIGHT: [f32; 3] = [0.3, 0.12, 0.03];

/// Splatting: dark flecks, a few embers.
static SPLAT: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 14.0,
    life: (0.25, 0.6),
    colors: vec![(20, 10, 6), (40, 18, 10), (255, 150, 60), (200, 80, 30)],
    speed: 120.0,
    spread: 1.3,
    gravity: 500.0,
    drag: 2.0,
    size: 1.0,
    jitter: 0.5,
    glow: false,
});

/// What a cell along its way costs: on a floor, up a wall, over a
/// ceiling; dropping straight down; across open air.
const ROAD: u32 = 2;
const WALL: u32 = 3;
const CEILING: u32 = 7;
const FALL: u32 = 1;
const AIR: u32 = 14;

/// Its way to `to`, from `from`: the cells, through a crack a cell wide,
/// keeping to rock (along floors, up walls, over ceilings: it creeps, it
/// doesn't fly). Dropping straight down's cheap (it falls); across open
/// air, dear (a short gap it'll bridge, not the middle of a cavern).
fn find_way(sim: &SimWorld, from: Vec2, to: Vec2, spread: (Vec2, Vec2)) -> (Vec<IVec2>, Option<Field>) {
    let (a, b) = (from.floor().as_ivec2(), to.floor().as_ivec2());
    // (Round it all: it, where it's going, and all of its motes (no
    // further off it than `SPREAD_MOST`).)
    let lo = a.min(b).min(spread.0.floor().as_ivec2().max(a - IVec2::splat(SPREAD_MOST))) - IVec2::splat(24);
    let hi = a.max(b).max(spread.1.floor().as_ivec2().min(a + IVec2::splat(SPREAD_MOST))) + IVec2::splat(24);
    let (w, h) = ((hi.x - lo.x + 1) as usize, (hi.y - lo.y + 1) as usize);
    let idx = |p: IVec2| -> Option<usize> {
        let q = p - lo;
        (q.x >= 0 && q.y >= 0 && (q.x as usize) < w && (q.y as usize) < h).then(|| q.y as usize * w + q.x as usize)
    };
    let at = |i: usize| lo + IVec2::new((i % w) as i32, (i / w) as i32);
    let open: Vec<bool> = (0..w * h).map(|i| !solid_cell(sim, at(i))).collect();
    let steps = [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y];
    // (Open, against rock: its road. On a floor's cheapest; up a wall,
    // dearer; over a ceiling, dearer still. Open air: none.)
    let shut = |p: IVec2| idx(p).is_none_or(|j| !open[j]);
    let road: Vec<u32> = (0..w * h)
        .map(|i| {
            let p = at(i);
            if !open[i] {
                0
            } else if shut(p - IVec2::Y) {
                ROAD
            } else if shut(p + IVec2::X) || shut(p - IVec2::X) {
                WALL
            } else if shut(p + IVec2::Y) {
                CEILING
            } else {
                0
            }
        })
        .collect();
    // (If you're in rock, or it is, the nearest open cell.)
    let near_open = |p: IVec2| (0..6).flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| p + IVec2::new(dx, dy)))).find_map(|q| idx(q).filter(|&i| open[i]));
    let (Some(bi), Some(ai)) = (near_open(b), near_open(a)) else { return (Vec::new(), None) };
    // (Out from where it's going, cheapest first (a bucket a cost), so the
    // way's read back from its heart: each cell's next toward it.)
    let mut cost = vec![u32::MAX; w * h];
    let mut next = vec![u32::MAX; w * h];
    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); 64];
    cost[bi] = 0;
    next[bi] = bi as u32;
    buckets[0].push(bi as u32);
    let mut c = 0u32;
    let mut left = 1usize;
    while left > 0 {
        let slot = (c % 64) as usize;
        let Some(qi) = buckets[slot].pop() else {
            c += 1;
            continue;
        };
        left -= 1;
        let qi = qi as usize;
        if cost[qi] != c {
            continue;
        }
        if qi == ai {
            break;
        }
        let q = at(qi);
        for s in steps {
            // (From `p` to `q`: a step back along the way.)
            let p = q - s;
            let Some(pi) = idx(p) else { continue };
            if !open[pi] {
                continue;
            }
            let step = if road[qi] > 0 { road[qi] } else if s == IVec2::NEG_Y { FALL } else { AIR };
            let k = c + step;
            if k < cost[pi] {
                cost[pi] = k;
                next[pi] = qi as u32;
                buckets[(k % 64) as usize].push(pi as u32);
                left += 1;
            }
        }
    }
    // (Finish the rest round it: what's behind it needs its way too.)
    while left > 0 {
        let slot = (c % 64) as usize;
        let Some(qi) = buckets[slot].pop() else {
            c += 1;
            continue;
        };
        left -= 1;
        let qi = qi as usize;
        if cost[qi] != c {
            continue;
        }
        let q = at(qi);
        for s in steps {
            let p = q - s;
            let Some(pi) = idx(p) else { continue };
            if !open[pi] {
                continue;
            }
            let step = if road[qi] > 0 { road[qi] } else if s == IVec2::NEG_Y { FALL } else { AIR };
            let k = c + step;
            if k < cost[pi] {
                cost[pi] = k;
                next[pi] = qi as u32;
                buckets[(k % 64) as usize].push(pi as u32);
                left += 1;
            }
        }
    }
    if next[ai] == u32::MAX {
        return (Vec::new(), None);
    }
    let mut way = vec![at(ai)];
    let mut i = ai;
    while i != bi && way.len() < 6000 {
        i = next[i] as usize;
        way.push(at(i));
    }
    (way, Some(Field { lo, w, h, next, cost }))
}

/// Each cave murk's life, a tick: its way, its heart, its motes.
fn live(mut sim: ResMut<SimWorld>, mut murks: ResMut<CaveMurks>, player: Query<&Kinematics, With<LocalPlayer>>, mut hearts: Query<&mut Transform, With<CaveLight>>, mut sparks: ResMut<crate::vfx::Sparks>, mut sounds: MessageWriter<crate::sound::PlaySound>) {
    if murks.list.is_empty() {
        return;
    }
    let sim = &mut *sim;
    let dt = 1.0 / TICK_HZ as f32;
    let tick = sim.world.tick();
    let you = player.single().ok().map(|k| k.body.pos);
    let goal = murks.goal.or(you);
    for m in &mut murks.list {
        m.age += dt;
        m.splat = (m.splat - dt * 1.5).max(0.0);
        // Its way: to you, when you're near enough.
        let target = goal.filter(|g| g.distance(m.heart) < AGGRO);
        if (tick + m.seed).is_multiple_of(PATH_EVERY) || target.is_some() != m.goal.is_some() {
            let spread = m.motes.iter().fold((Vec2::MAX, Vec2::MIN), |(lo, hi), o| (lo.min(o.pos), hi.max(o.pos)));
            (m.way, m.field) = target.map_or((Vec::new(), None), |g| find_way(sim, m.mass, g, spread));
            m.goal = target;
        }
        // (Moving on along it as the heart goes.)
        if let Some(n) = m.way.iter().take(40).position(|c| c.as_vec2().distance(m.heart) < 1.5) {
            m.way.drain(..n);
        }
        // Its mass's middle: what's near its heart (a trickle off through a
        // crack isn't it).
        let (sum, n) = m.motes.iter().filter(|o| o.pos.distance(m.heart) < 26.0).fold((Vec2::ZERO, 0.0), |(s, n), o| (s + o.pos, n + 1.0));
        let mass = if n > 0.0 { sum / n } else { m.motes.iter().fold(Vec2::ZERO, |s, o| s + o.pos) / m.motes.len().max(1) as f32 };
        let vel = (mass - m.mass) / dt;
        // (Falling fast, then not: it splats.)
        if m.mass_vel.y < -SPLAT_FROM && vel.y > -SPLAT_FROM * 0.3 && m.age > 0.5 {
            m.splat = 1.0;
            sparks.emit(&SPLAT, 3, mass, Vec2::Y, Vec2::ZERO);
            sounds.write(crate::sound::PlaySound::at("murk_hit", mass).volume(0.5).pitch(0.5));
            debug!("cave murk: splat at {:.0} cells/s", -m.mass_vel.y);
        }
        m.mass_vel = m.mass_vel.lerp(vel, 0.5);
        m.mass = mass;
        // Its heart: ahead of its mass along its way (no further); held to
        // it, so it falls with it.
        let ahead = m.way.get(AHEAD.min(m.way.len().saturating_sub(1))).map(|c| c.as_vec2() + Vec2::splat(0.5));
        // (Climbing: its way goes up from here.)
        // (Or over its lip: its heart up there, its mass still hanging
        // below.)
        let climbing = ahead.is_some_and(|a| a.y > m.heart.y + 3.0) || (ahead.is_some() && m.heart.y > mass.y + 5.0);
        // (Cell by cell along its way, while its mass keeps up; too far
        // ahead of it, it waits; left behind (it fell), back to it.)
        let on = m.way.get(3.min(m.way.len().saturating_sub(1))).map(|c| c.as_vec2() + Vec2::splat(0.5));
        // (Its mass keeps up: so much of it round its heart.)
        let round = m.motes.iter().filter(|o| o.pos.distance_squared(m.heart) < KEEP_R * KEEP_R).count() as f32 / m.motes.len().max(1) as f32;
        // (Off a ledge, nothing round it: it drops, its mass with it.)
        let hc = m.heart.floor().as_ivec2();
        let unheld = ![IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y, IVec2::new(0, -2)].iter().any(|s| solid_cell(sim, hc + *s));
        let landed = solid_cell(sim, hc - IVec2::Y) || solid_cell(sim, hc - IVec2::new(0, 2));
        let dropping = (unheld && on.is_some_and(|n| n.y < m.heart.y - 0.3)) || (m.fall > 0.0 && !landed);
        m.fall = if dropping { m.fall + GRAVITY * dt } else { 0.0 };
        let (want, speed) = match on {
            Some(_) if dropping => (m.heart - Vec2::Y * 50.0, m.fall.max(CRAWL)),
            Some(n) if round > KEEP => (n, CRAWL),
            Some(_) if round > KEEP * 0.3 && m.waited < WAIT => (m.heart, 0.0),
            _ => (mass, CRAWL.max(m.mass_vel.length()) * 1.5),
        };
        m.waited = if speed == 0.0 { m.waited + dt } else if round > KEEP { 0.0 } else { m.waited };
        let step = ((want - m.heart) * HELD).clamp_length_max(speed) * dt;
        let next = m.heart + step;
        if !solid(sim, next) {
            m.heart = next;
        } else if !solid(sim, Vec2::new(next.x, m.heart.y)) {
            m.heart.x = next.x;
        } else if !solid(sim, Vec2::new(m.heart.x, next.y)) {
            m.heart.y = next.y;
        }
        // Its motes: pressing apart, dragging each other, drawn to its
        // heart, weighed down (less against rock: it sticks).
        let mut grid: HashMap<IVec2, Vec<u16>> = HashMap::default();
        for (i, o) in m.motes.iter().enumerate() {
            grid.entry((o.pos / PRESS_R).floor().as_ivec2()).or_default().push(i as u16);
        }
        let heart = m.heart;
        let field = m.field.as_ref();
        let here = field.and_then(|f| f.on(heart)).map_or(0, |(_, c)| c);
        let mut acc = vec![Vec2::ZERO; m.motes.len()];
        for (i, o) in m.motes.iter().enumerate() {
            let c = (o.pos / PRESS_R).floor().as_ivec2();
            let mut a = Vec2::ZERO;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let Some(near) = grid.get(&(c + IVec2::new(dx, dy))) else { continue };
                    for &j in near {
                        let j = j as usize;
                        if j == i {
                            continue;
                        }
                        let p = m.motes[j];
                        let d = o.pos - p.pos;
                        let l = d.length();
                        if l >= PRESS_R {
                            continue;
                        }
                        let k = 1.0 - l / PRESS_R;
                        let away = if l > 1e-4 { d / l } else { Vec2::from_angle(unit(&[m.seed, i as u64, j as u64]) * std::f32::consts::TAU) };
                        a += away * PRESS * k + (p.vel - o.vel) * THICKNESS * k;
                    }
                }
            }
            // (Behind its heart along its way: flowing along it, round
            // corners, through cracks; with it: drawn to it.)
            let behind = field.filter(|_| o.pos.distance_squared(heart) > FLOW_FROM * FLOW_FROM).and_then(|f| f.on(o.pos)).filter(|(_, c)| *c > here + FLOW_BEHIND);
            if let Some((dir, _)) = behind {
                a += (dir * FLOW - o.vel) * FLOW_K;
            } else {
                let to = heart - o.pos;
                let l = to.length();
                a += to.normalize_or_zero() * PULL * l.min(PULL_FAR) * if climbing { CLIMB_PULL } else { 1.0 };
            }
            let weight = match (o.stuck, climbing) {
                (true, true) => 0.0,
                (true, false) => CLING,
                (false, true) => CLIMB_WEIGHT,
                (false, false) => 1.0,
            };
            a.y -= GRAVITY * weight;
            if o.stuck && !o.floor {
                a -= o.vel * STICKY;
            }
            acc[i] = a;
        }
        // (Moved a cell at most a step, an axis at a time: stopped by
        // rock, sliding along it.)
        for (o, a) in m.motes.iter_mut().zip(acc) {
            o.vel = (o.vel + a * dt).clamp_length_max(TOP);
            let d = o.vel * dt;
            let n = (d.abs().max_element() / 0.8).ceil().max(1.0) as usize;
            let s = d / n as f32;
            for _ in 0..n {
                let x = Vec2::new(o.pos.x + s.x, o.pos.y);
                if solid(sim, x) {
                    o.vel.x *= -0.05;
                    o.vel.y *= 0.9;
                } else {
                    o.pos.x = x.x;
                }
                let y = Vec2::new(o.pos.x, o.pos.y + s.y);
                if solid(sim, y) {
                    o.vel.y *= -0.05;
                    o.vel.x *= 0.9;
                } else {
                    o.pos.y = y.y;
                }
            }
            // (In rock, somehow (it closed round it): out to the nearest
            // open cell.)
            if solid(sim, o.pos) {
                let c = o.pos.floor().as_ivec2();
                match [IVec2::Y, IVec2::NEG_X, IVec2::X, IVec2::NEG_Y, IVec2::new(1, 1), IVec2::new(-1, 1)].iter().map(|s| c + *s).find(|q| !solid_cell(sim, *q)) {
                    Some(q) => o.pos = q.as_vec2() + Vec2::splat(0.5),
                    // (Deep in it: back at its heart.)
                    None => o.pos = heart + Vec2::new(unit(&[m.seed, tick, o.pos.x.to_bits() as u64]) - 0.5, 0.0),
                }
                o.vel = Vec2::ZERO;
            }
            let c = o.pos.floor().as_ivec2();
            o.floor = solid_cell(sim, c - IVec2::Y);
            o.stuck = o.floor || [IVec2::X, IVec2::NEG_X, IVec2::Y].iter().any(|s| solid_cell(sim, c + *s));
        }
        if let Ok(mut tf) = hearts.get_mut(m.light) {
            tf.translation = m.heart.extend(tf.translation.z);
        }
    }
}

#[derive(Resource)]
struct CaveCanvas(crate::canvas::Canvas);

impl Default for CaveCanvas {
    fn default() -> Self {
        CaveCanvas(crate::canvas::Canvas::new("Cave murk", Z_MURK))
    }
}

/// As the space murk's: over the light's glow, under glowfish.
const Z_MURK: f32 = 15.55;
/// Its edge's glow: how far out (cells), how much rim near makes it full,
/// how strong.
const GLOW_R: f32 = 4.0;
const GLOW_FULL: f32 = 18.0;
const GLOW: f32 = 0.3;
/// Its body, drawn whole: this many motes in the 5×5 round a cell (less
/// is spray, drawn drop by drop).
const BODY: u16 = 7;

fn show(
    mut commands: Commands,
    time: Res<Time>,
    murks: Res<CaveMurks>,
    mut canvas: ResMut<CaveCanvas>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<(&GlobalTransform, &crate::world::ChunkLoader), With<crate::camera::MainCamera>>,
    mut sprites: crate::canvas::CanvasSprites,
) {
    let Ok((cam, loader)) = camera.single() else { return };
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, !murks.list.is_empty()) else { return };
    let t = time.elapsed_secs();
    let rgba = |c: [f32; 3], a: f32| [c[0].min(255.0) as u8, c[1].min(255.0) as u8, c[2].min(255.0) as u8, (a.clamp(0.0, 1.0) * 255.0) as u8];
    for m in &murks.list {
        let (lo, hi) = m.motes.iter().fold((Vec2::MAX, Vec2::MIN), |(lo, hi), o| (lo.min(o.pos), hi.max(o.pos)));
        if lo.x > hi.x {
            continue;
        }
        let lo = lo.floor().as_ivec2() - IVec2::splat(2 + GLOW_R as i32);
        let hi = hi.floor().as_ivec2() + IVec2::splat(2 + GLOW_R as i32);
        let (w, h) = ((hi.x - lo.x + 1) as usize, (hi.y - lo.y + 1) as usize);
        if w * h > 1_500_000 {
            continue;
        }
        // How thick, cell by cell: motes in each, then in the 5×5 round it
        // (smooth: its body's whole, what's thinner is spray).
        let mut here = vec![0u8; w * h];
        for o in &m.motes {
            let c = o.pos.floor().as_ivec2() - lo;
            if c.x >= 0 && c.y >= 0 && (c.x as usize) < w && (c.y as usize) < h {
                let i = c.y as usize * w + c.x as usize;
                here[i] = here[i].saturating_add(1);
            }
        }
        let mut rows = vec![0u16; w * h];
        for y in 0..h {
            for x in 0..w {
                rows[y * w + x] = (x.saturating_sub(2)..(x + 3).min(w)).map(|i| here[y * w + i] as u16).sum();
            }
        }
        let mut thick = vec![0u16; w * h];
        for y in 0..h {
            for x in 0..w {
                thick[y * w + x] = (y.saturating_sub(2)..(y + 3).min(h)).map(|j| rows[j * w + x]).sum();
            }
        }
        let at = |x: i32, y: i32| if x < 0 || y < 0 || x as usize >= w || y as usize >= h { 0 } else { thick[y as usize * w + x as usize] };
        let rim: Vec<u8> = (0..w * h)
            .map(|i| {
                let (x, y) = ((i % w) as i32, (i / w) as i32);
                u8::from(thick[i] >= BODY && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| at(x + dx, y + dy) < BODY))
            })
            .collect();
        // (Its glow: how much rim is near, out past it, a smoulder.)
        let gr = GLOW_R as usize;
        let mut rows = vec![0u16; w * h];
        for y in 0..h {
            for x in 0..w {
                rows[y * w + x] = (x.saturating_sub(gr)..(x + gr + 1).min(w)).map(|i| rim[y * w + i] as u16).sum();
            }
        }
        let glow: Vec<u16> = (0..w * h).map(|i| ((i / w).saturating_sub(gr)..(i / w + gr + 1).min(h)).map(|j| rows[j * w + i % w]).sum()).collect();
        let breathe = 0.8 + 0.2 * (t * 1.7 + m.seed as f32).sin();
        let flare = m.splat;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let (d, n) = (thick[i], here[i]);
                let (cx, cy) = (lo.x + x as i32, lo.y + y as i32);
                if d < BODY {
                    if n > 0 {
                        // (Spray, a drop: dark, lit on its top.)
                        let lit = at(x as i32, y as i32 + 1) == 0 || here.get(i + w).is_some_and(|u| *u == 0);
                        px.put(cx, cy, if lit { rgba([200.0, 84.0, 22.0], 0.95) } else { rgba([22.0, 9.0, 5.0], 0.95) });
                        continue;
                    }
                    let g = glow[i];
                    if g > 0 {
                        let k = (g as f32 / GLOW_FULL).min(1.0);
                        px.put(cx, cy, rgba([230.0, 96.0, 24.0], (GLOW * k * k * breathe + 0.25 * flare * k).min(0.7)));
                    }
                    continue;
                }
                // (Inside: black, a slow wet sheen running over it, warmer
                // where it's moving; its edge an ember, hotter on top.)
                let sheen = 0.5 + 0.5 * (cx as f32 * 0.31 - cy as f32 * 0.52 + t * 2.2).sin();
                let (c, a) = if rim[i] == 1 {
                    let top = at(x as i32, y as i32 + 1) < BODY;
                    let hot = if top { 1.0 } else { 0.7 };
                    ([(170.0 + 70.0 * sheen) * hot + 60.0 * flare, (60.0 + 40.0 * sheen) * hot + 40.0 * flare, 14.0 + 10.0 * sheen], 1.0)
                } else if n > 0 {
                    ([14.0 + 14.0 * sheen, 6.0 + 5.0 * sheen, 4.0], 1.0)
                } else {
                    ([5.0, 2.0, 2.0], 1.0)
                };
                px.put(cx, cy, rgba(c, a));
            }
        }
        // Drips: what's falling off it, a short ember streak.
        for o in &m.motes {
            if o.vel.y < -120.0 {
                let b = o.pos - o.vel * 0.02;
                px.put(b.x.floor() as i32, b.y.floor() as i32, rgba([120.0, 46.0, 14.0], 0.6));
            }
        }
        // Its heart, a slow ember beating in it.
        let beat = (t * std::f32::consts::TAU * 0.8).sin().max(0.0).powi(3);
        let c = m.heart.floor().as_ivec2();
        let r = CORE_R * (0.75 + 0.15 * beat);
        let ri = r.ceil() as i32;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let l = Vec2::new(dx as f32, dy as f32).length();
                if l <= r {
                    let k = 1.0 - 0.55 * l / r;
                    px.put(c.x + dx, c.y + dy, rgba([(150.0 + 90.0 * beat) * k, (40.0 + 60.0 * beat) * k, 12.0 * k], 1.0));
                }
            }
        }
    }
}

/// A seed for one put down at `at` (the dev menu).
pub fn seed_at(at: Vec2) -> u64 {
    hash(&[at.x.to_bits() as u64, at.y.to_bits() as u64, 0xCA7E])
}
