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

/// What a jump costs besides its time (s), and a running one more.
const JUMP_EXTRA: f32 = 0.2;
const RUN_UP_EXTRA: f32 = 0.3;

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
        // (A jump costs more than its time: it's chancier than walking, so
        // a body walks where it can and doesn't hop along the flat.)
        for (i, j) in p.jumps.iter().enumerate() {
            for dx in [1, -1] {
                // (A running jump the more so: it needs the speed it was
                // made at, a standing one only to stop first.)
                let extra = if j.run_up { JUMP_EXTRA + RUN_UP_EXTRA } else { JUMP_EXTRA };
                out.push(Move { kind: Kind::Jump(i as u16), d: IVec2::new(j.to.x * dx, j.to.y), secs: j.secs + extra });
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
            // Not onto an edge: ground goes on a node past where it comes
            // down (it may come down a little further than the arc says).
            let beyond = to + IVec2::new(flip, 0);
            if [beyond, beyond + IVec2::Y, beyond - IVec2::Y].iter().all(|n| v.stand(*n).is_none()) {
                return false;
            }
            // The arc played over the cells, a tick at a time, from where
            // it takes off (the middle of its node, its floor): up and down
            // as it was; across as it was unless something's in the way,
            // when it stays put (and the way across it lost is lost: a body
            // stopped by a wall goes on up it, not on through); no room even
            // where it is: no jump. It must come down where it says (onto
            // its floor, over its node).
            let (Some(floor), Some(land)) = (v.stand(from), v.stand(to)) else { return false };
            let base = IVec2::new(from.x * NODE + NODE / 2, floor);
            let (mut x, mut last, mut top) = (base.x, 0, 0);
            for s in &j.arc {
                let y = base.y + s.y;
                // Coming down onto the floor it's making for: there.
                if s.y < top && y <= land {
                    return x.div_euclid(NODE) == to.x && v.fits_cell(x, land);
                }
                top = top.max(s.y);
                let next = x + (s.x - last) * flip;
                last = s.x;
                if v.fits_cell(next, y) {
                    x = next;
                } else if !v.fits_cell(x, y) {
                    return false;
                }
            }
            x.div_euclid(NODE) == to.x
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
