//! What one kind of body can do, read from what it already has: its size,
//! its movement (`MovementStats`), how far it may fall. Its jumps are the
//! real physics run once in an empty room: every arc it can make, where it
//! comes down on the way, how long it holds the key.

use glam::{IVec2, Vec2};
use platypus_physics::{Body, Grid, Intent, Locomotion, MovementStats, Occupancy, move_and_collide};

use crate::moves::{Kind, Move};
use crate::tile::NODE;

const DT: f32 = 1.0 / 60.0;
/// The longest an arc is followed (s).
const LONGEST: f32 = 2.5;

/// One jump: where it comes down (node offset, facing right), where its
/// feet are each tick on the way (cells from where it took off: the body
/// must fit at each), how long it takes, how long the key is held, and
/// whether it runs at it first.
#[derive(Clone, Debug, PartialEq)]
pub struct Jump {
    pub to: IVec2,
    pub arc: Vec<IVec2>,
    pub secs: f32,
    pub hold: f32,
    pub run_up: bool,
    /// A climber's leap: it catches hold of something at `to` (a wall, a
    /// ledge's edge, a ceiling), on the way up or down, rather than coming
    /// down on ground.
    pub catch: bool,
}

/// One way of jumping (standing or at a run, the key held so long): its
/// whole arc (feet each tick, cells from where it took off, facing right),
/// and its jumps by where they come down or catch hold (node offset, facing
/// right) and their moves (right, left). A jump's edges are found by
/// playing its way's arc over the cells once: where it comes down first,
/// what it passes it could catch hold of.
#[derive(Clone, Debug, Default)]
pub struct Arc {
    pub arc: Vec<IVec2>,
    pub hold: f32,
    pub run_up: bool,
    pub lands: rustc_hash::FxHashMap<IVec2, [u16; 2]>,
    pub catches: rustc_hash::FxHashMap<IVec2, [u16; 2]>,
}

/// How a body digs (its file's `dig`): claws through what's no harder
/// than `claws`, `claw_rate` cells a second of dirt (hardness 20; harder
/// is slower); acid (spat) through what's no harder than `acid` and not
/// inert, `acid_rate` cells a second of dirt.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Digging {
    pub claws: u8,
    pub claw_rate: f32,
    pub acid: u8,
    pub acid_rate: f32,
}

impl Digging {
    /// Seconds to dig one cell of this hardness (None: it can't).
    pub fn secs(&self, hardness: u8, inert: bool) -> Option<f32> {
        if self.claws > 0 && hardness <= self.claws && self.claw_rate > 0.0 {
            Some(hardness.max(4) as f32 / 20.0 / self.claw_rate)
        } else if self.acid > 0 && hardness <= self.acid && !inert && self.acid_rate > 0.0 {
            Some(hardness.max(10) as f32 / 20.0 / self.acid_rate)
        } else {
            None
        }
    }

    /// What it is, as the grid's kept digging times know it.
    pub fn key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = rustc_hash::FxHasher::default();
        (self.claws, self.acid, self.claw_rate.to_bits(), self.acid_rate.to_bits()).hash(&mut h);
        h.finish()
    }
}

/// One kind of body's ways of getting about.
#[derive(Clone, Debug)]
pub struct Profile {
    /// Its box (cells).
    pub size: (u16, u16),
    /// Walking speed (cells/s), the highest step it walks up (cells), the
    /// furthest it'll drop (cells).
    pub run: f32,
    pub step: i32,
    pub drop: i32,
    pub gravity: f32,
    /// Holds on to walls, ceilings and walls behind (a spider).
    pub climb: bool,
    /// Flies (cells/s; 0: doesn't).
    pub fly: f32,
    /// Swims (cells/s in liquid; 0: as bodies do, slowly).
    pub swim: f32,
    /// Digs (`digging`).
    pub dig: Option<Digging>,
    pub jumps: Vec<Jump>,
    /// Its ways of jumping, each with the jumps along it.
    pub arcs: Vec<Arc>,
    /// Every move it has (both ways), its jumps' among them.
    pub moves: Vec<Move>,
    /// What it is, as the grid's cached edges know it (the same for two
    /// profiles that move the same way).
    pub key: u64,
}

/// An empty room with a floor to stand on at the start.
struct Room(i32);

impl Grid for Room {
    fn occupancy(&self, x: i32, y: i32) -> Occupancy {
        if y < 0 && x.abs() <= self.0 { Occupancy::Solid } else { Occupancy::Empty }
    }
}

impl Profile {
    /// From a body's size (cells), its movement, and the furthest it'll
    /// drop on purpose (cells).
    pub fn new(size: (f32, f32), stats: &MovementStats, drop: f32) -> Profile {
        let size = (size.0.ceil().max(1.0) as u16, size.1.ceil().max(1.0) as u16);
        let walks = stats.fly_speed <= 0.0 && stats.swim_speed <= 0.0 || stats.cling;
        let mut p = Profile {
            size,
            run: stats.run_speed.max(1.0),
            step: stats.step_height,
            drop: drop as i32,
            gravity: stats.gravity * stats.fall_gravity.max(1.0),
            climb: stats.cling,
            fly: stats.fly_speed,
            swim: stats.swim_speed,
            jumps: Vec::new(),
            arcs: Vec::new(),
            moves: Vec::new(),
            key: 0,
            dig: None,
        };
        if walks && stats.jump_height > 0.0 {
            (p.jumps, p.arcs) = jumps(size, stats, drop, stats.cling);
        }
        p.moves = crate::moves::all(&p);
        p.index_arcs();
        p.key = p.make_key();
        p
    }

    /// Each jump's moves, on its way's arc.
    fn index_arcs(&mut self) {
        for a in &mut self.arcs {
            a.lands.clear();
            a.catches.clear();
        }
        for (k, m) in self.moves.iter().enumerate() {
            let Kind::Jump(i) = m.kind else { continue };
            let j = &self.jumps[i as usize];
            let Some(a) = self.arcs.iter_mut().find(|a| a.hold == j.hold && a.run_up == j.run_up) else { continue };
            let side = if m.d.x > 0 { 0 } else { 1 };
            let at = if j.catch { &mut a.catches } else { &mut a.lands };
            at.entry(j.to).or_insert([u16::MAX; 2])[side] = k as u16;
        }
    }

    /// The same body, digging as it says.
    pub fn digging(mut self, d: Digging) -> Profile {
        self.dig = Some(d);
        self.moves = crate::moves::all(&self);
        self.index_arcs();
        self.key = self.make_key();
        self
    }

    fn make_key(&self) -> u64 {
        let p = self;
        {
            use std::hash::{Hash, Hasher};
            let mut h = std::hash::DefaultHasher::new();
            (p.size, p.step, p.drop, p.climb, p.fly > 0.0, p.swim > 0.0).hash(&mut h);
            for j in &p.jumps {
                (j.to, &j.arc).hash(&mut h);
            }
            if let Some(d) = p.dig {
                (d.claws, d.acid, d.claw_rate.to_bits(), d.acid_rate.to_bits()).hash(&mut h);
            }
            h.finish()
        }
    }

    /// Its fastest way of getting about (cells/s): what the searches'
    /// guesses are made with.
    pub fn fastest(&self) -> f32 {
        self.run.max(self.fly).max(self.swim)
    }

    /// How far it can get back up (nodes): its highest jump; any height
    /// for a climber or a flyer.
    pub fn back_up(&self) -> i32 {
        if self.climb || self.fly > 0.0 { i32::MAX } else { self.jumps.iter().map(|j| j.to.y).max().unwrap_or(0).max(self.step / crate::tile::NODE) }
    }

    /// The move a kind and offset is (a path's steps say which).
    pub fn move_of(&self, kind: Kind, d: IVec2) -> Option<&Move> {
        self.moves.iter().find(|m| m.kind == kind && m.d == d)
    }
}

/// Every arc its jump can make, run in the physics: held short, half and
/// full, standing and at a run. Where each comes down on the way down (a
/// node it passes falling), each way of jumping's arc to each.
fn jumps(size: (u16, u16), stats: &MovementStats, drop: f32, catches: bool) -> (Vec<Jump>, Vec<Arc>) {
    let (w, h) = (size.0 as f32, size.1 as f32);
    let room = Room(size.0 as i32);
    // Standing on the floor, settled.
    let mut body = Body::new(Vec2::new(0.5, h / 2.0), Vec2::new(w, h));
    body.step_height = stats.step_height;
    let mut loco = Locomotion::default();
    for _ in 0..30 {
        loco.steer(stats, &Intent::default(), &mut body, DT);
        let c = move_and_collide(&room, &mut body, DT);
        loco.after_move(c);
    }
    let start = Vec2::new(body.pos.x, body.pos.y - h / 2.0);
    let holds: Vec<f32> = if stats.jump_hold > 0.0 { vec![0.0, stats.jump_hold * 0.5, stats.jump_hold] } else { vec![0.0, 0.08, 0.2] };
    let mut best: Vec<Jump> = Vec::new();
    let mut arcs: Vec<Arc> = Vec::new();
    for run_up in [false, true] {
        for &hold in &holds {
            let (mut b, mut l) = (body, loco);
            if run_up {
                b.vel.x = stats.run_speed;
            }
            let mut arc: Vec<IVec2> = Vec::new();
            let mut t = 0.0;
            let mut rising = true;
            while t < LONGEST {
                // Pressed, held a while (a tap: one tick), forward all the way.
                let intent = Intent { move_x: 1.0, jump: t < hold.max(DT * 0.5), ..default_intent() };
                l.steer(stats, &intent, &mut b, DT);
                let c = move_and_collide(&room, &mut b, DT);
                l.after_move(c);
                t += DT;
                let feet = Vec2::new(b.pos.x, b.pos.y - h / 2.0) - start;
                // (Nodes from where it stood: its feet in the middle of a node's bottom.)
                let n = IVec2::new(((feet.x + (NODE / 2) as f32) / NODE as f32).floor() as i32, (feet.y / NODE as f32).floor() as i32);
                arc.push(IVec2::new(feet.x.round() as i32, feet.y.floor() as i32));
                rising &= b.vel.y > 0.0;
                if feet.y < -drop {
                    break;
                }
                // Coming down: it could land here. (A climber, anywhere on
                // the way: it could catch hold here.)
                let landing = !rising && b.vel.y <= 0.0;
                for catch in [false, true] {
                    if !(if catch { catches } else { landing }) || n.x < 1 || (n.x <= 1 && n.y == 0) || n == IVec2::ZERO {
                        continue;
                    }
                    let to = n;
                    match best.iter_mut().find(|j| j.to == to && j.hold == hold && j.run_up == run_up && j.catch == catch) {
                        Some(j) if j.secs <= t => {}
                        Some(j) => *j = Jump { to, arc: arc.clone(), secs: t, hold, run_up, catch },
                        None => best.push(Jump { to, arc: arc.clone(), secs: t, hold, run_up, catch }),
                    }
                }
            }
            arcs.push(Arc { arc, hold, run_up, ..Arc::default() });
        }
    }
    // To each landing, the quickest arc and the highest (a higher, slower
    // arc gets up a wall the quickest doesn't); to each hold caught, the
    // quickest (holding on is forgiving).
    let apex = |j: &Jump| j.arc.iter().map(|a| a.y).max().unwrap_or(0);
    let mut kept: Vec<Jump> = Vec::new();
    for j in &best {
        let same: Vec<&Jump> = best.iter().filter(|k| k.to == j.to && k.catch == j.catch).collect();
        let quickest = same.iter().min_by(|a, b| a.secs.total_cmp(&b.secs)).copied();
        let highest = same.iter().max_by_key(|k| apex(k)).copied();
        let keep = quickest.is_some_and(|q| std::ptr::eq(q, j)) || (!j.catch && highest.is_some_and(|h| std::ptr::eq(h, j)));
        if keep && !kept.iter().any(|k| k.to == j.to && k.catch == j.catch && k.arc == j.arc) {
            kept.push(j.clone());
        }
    }
    (kept, arcs)
}

fn default_intent() -> Intent {
    Intent::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_jump_reaches_as_high_and_far_as_the_physics_lets_it() {
        let stats = MovementStats::default();
        let p = Profile::new((9.0, 23.0), &stats, 60.0);
        assert!(!p.jumps.is_empty());
        // Up onto something about its jump's height (60 cells: 15 nodes),
        // never higher.
        let top = p.jumps.iter().map(|j| j.to.y).max().unwrap();
        assert!((10..=15).contains(&top), "highest landing {top} nodes up");
        // Further at a run than standing.
        let far = |run: bool| p.jumps.iter().filter(|j| j.run_up == run && j.to.y == 0).map(|j| j.to.x).max().unwrap_or(0);
        assert!(far(true) > far(false), "run {} vs stand {}", far(true), far(false));
        // A flyer has no jumps; a climber has its jumps and climbs.
        assert!(Profile::new((9.0, 9.0), &MovementStats { fly_speed: 200.0, ..stats.clone() }, 60.0).jumps.is_empty());
        assert!(Profile::new((24.0, 18.0), &MovementStats { cling: true, ..stats }, 60.0).climb);
    }
}





