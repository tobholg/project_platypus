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

/// One jump: where it comes down (node offset, facing right), the nodes
/// it passes through on the way (the body must fit in each), how long it
/// takes, how long the key is held, and whether it runs at it first.
#[derive(Clone, Debug, PartialEq)]
pub struct Jump {
    pub to: IVec2,
    pub through: Vec<IVec2>,
    pub secs: f32,
    pub hold: f32,
    pub run_up: bool,
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
    pub jumps: Vec<Jump>,
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
            jumps: if walks && stats.jump_height > 0.0 { jumps(size, stats, drop) } else { Vec::new() },
            moves: Vec::new(),
            key: 0,
        };
        p.moves = crate::moves::all(&p);
        p.key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::hash::DefaultHasher::new();
            (p.size, p.step, p.drop, p.climb, p.fly > 0.0, p.swim > 0.0).hash(&mut h);
            for j in &p.jumps {
                (j.to, &j.through).hash(&mut h);
            }
            h.finish()
        };
        p
    }

    /// Its fastest way of getting about (cells/s): what the searches'
    /// guesses are made with.
    pub fn fastest(&self) -> f32 {
        self.run.max(self.fly).max(self.swim)
    }

    /// The move a kind and offset is (a path's steps say which).
    pub fn move_of(&self, kind: Kind, d: IVec2) -> Option<&Move> {
        self.moves.iter().find(|m| m.kind == kind && m.d == d)
    }
}

/// Every arc its jump can make, run in the physics: held short, half and
/// full, standing and at a run. Where each comes down on the way down (a
/// node it passes falling), the quickest arc to each.
fn jumps(size: (u16, u16), stats: &MovementStats, drop: f32) -> Vec<Jump> {
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
    for run_up in [false, true] {
        for &hold in &holds {
            let (mut b, mut l) = (body, loco);
            if run_up {
                b.vel.x = stats.run_speed;
            }
            let mut through: Vec<IVec2> = Vec::new();
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
                if through.last() != Some(&n) {
                    through.push(n);
                }
                rising &= b.vel.y > 0.0;
                if feet.y < -drop {
                    break;
                }
                // Coming down: it could land here.
                if !rising && b.vel.y <= 0.0 && n.x >= 1 && !(n.x <= 1 && n.y == 0) && n != IVec2::ZERO {
                    let to = n;
                    let path: Vec<IVec2> = through[..through.len() - 1].to_vec();
                    match best.iter_mut().find(|j| j.to == to) {
                        Some(j) if j.secs <= t => {}
                        Some(j) => *j = Jump { to, through: path, secs: t, hold, run_up },
                        None => best.push(Jump { to, through: path, secs: t, hold, run_up }),
                    }
                }
            }
        }
    }
    best
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

