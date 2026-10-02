//! Moves: what a body can do from a node, each an offset (nodes) and a cost
//! (seconds), and whether it can, here (`check`). The searches know nothing
//! else: a new way of getting about is a new kind of move.

use glam::IVec2;

use crate::NavWorld;
use crate::profile::Profile;
use crate::tile::{NODE, NodePos, View};

/// How a move is made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Along the ground, stepping up what it can.
    Walk,
    /// Off an edge, falling straight down beside it.
    Drop,
    /// One of its jumps (by its place in `Profile::jumps`), mirrored if it
    /// goes left.
    Jump(u16),
    /// Holding on to walls, ceilings, the wall behind.
    Climb,
    Fly,
    Swim,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Move {
    pub kind: Kind,
    pub d: IVec2,
    pub secs: f32,
}

const AROUND: [IVec2; 8] = [IVec2::new(1, 0), IVec2::new(-1, 0), IVec2::new(0, 1), IVec2::new(0, -1), IVec2::new(1, 1), IVec2::new(-1, 1), IVec2::new(1, -1), IVec2::new(-1, -1)];

/// Every move a profile has, both ways.
pub fn all(p: &Profile) -> Vec<Move> {
    let node = NODE as f32;
    let mut out = Vec::new();
    let walks = p.fly <= 0.0 && p.swim <= 0.0 || p.climb;
    if walks {
        for dx in [-1, 1] {
            for dy in -1..=1 {
                out.push(Move { kind: Kind::Walk, d: IVec2::new(dx, dy), secs: node / p.run });
            }
            // Falling k nodes: how long a fall that far takes, and the step off.
            for k in 1..=(p.drop / NODE).max(1) {
                let fall = (2.0 * (k * NODE) as f32 / p.gravity.max(1.0)).sqrt();
                out.push(Move { kind: Kind::Drop, d: IVec2::new(dx, -k), secs: node / p.run + fall });
            }
        }
        for (i, j) in p.jumps.iter().enumerate() {
            for dx in [1, -1] {
                out.push(Move { kind: Kind::Jump(i as u16), d: IVec2::new(j.to.x * dx, j.to.y), secs: j.secs });
            }
        }
    }
    let diag = |d: IVec2, speed: f32| node * (d.as_vec2().length()) / speed.max(1.0);
    for d in AROUND {
        if p.climb {
            out.push(Move { kind: Kind::Climb, d, secs: diag(d, p.run) });
        }
        if p.fly > 0.0 {
            out.push(Move { kind: Kind::Fly, d, secs: diag(d, p.fly) });
        }
        // Everyone swims; swimmers well, the rest slowly.
        out.push(Move { kind: Kind::Swim, d, secs: diag(d, if p.swim > 0.0 { p.swim } else { p.run * 0.3 }) });
    }
    out
}

/// Whether a move can be made from `from`, here.
pub fn check<W: NavWorld>(v: &mut View<W>, p: &Profile, from: NodePos, m: &Move) -> bool {
    let to = from + m.d;
    match m.kind {
        Kind::Walk => match (v.stand(from), v.stand(to)) {
            (Some(a), Some(b)) => (b - a).abs() <= p.step.max(NODE - 1) && (b <= a || p.step > 0),
            _ => false,
        },
        Kind::Drop => {
            // Off the edge into the open beside it, down through open
            // nodes (no ground on the way), onto ground.
            if v.stand(from).is_none() || v.stand(to).is_none() {
                return false;
            }
            let side = IVec2::new(to.x, from.y);
            (0..(from.y - to.y)).all(|k| {
                let n = side - IVec2::new(0, k);
                v.fits(n) && v.stand(n).is_none()
            })
        }
        Kind::Jump(i) => {
            let Some(j) = p.jumps.get(i as usize) else { return false };
            // (Where it lands first: most nodes aren't ground.)
            if v.stand(to).is_none() || v.stand(from).is_none() {
                return false;
            }
            let flip = if m.d.x < 0 { -1 } else { 1 };
            j.through.iter().all(|n| {
                let at = from + IVec2::new(n.x * flip, n.y);
                at == from || v.fits(at)
            })
        }
        Kind::Climb => {
            let ok = |v: &mut View<W>, n: NodePos| v.hold(n) || v.stand(n).is_some();
            ok(v, from) && ok(v, to) && corners(v, from, m.d)
        }
        Kind::Fly => v.fits(from) && v.fits(to) && corners(v, from, m.d),
        Kind::Swim => {
            let (a, b) = (v.wet(from), v.wet(to));
            // In it, along it; or out of it onto something (and in from it).
            (a && b || a && v.stand(to).is_some() || b && v.stand(from).is_some()) && v.fits(to) && corners(v, from, m.d)
        }
    }
}

/// A diagonal doesn't cut a corner: both nodes beside it are open.
fn corners<W: NavWorld>(v: &mut View<W>, from: NodePos, d: IVec2) -> bool {
    d.x == 0 || d.y == 0 || (v.fits(from + IVec2::new(d.x, 0)) && v.fits(from + IVec2::new(0, d.y)))
}
