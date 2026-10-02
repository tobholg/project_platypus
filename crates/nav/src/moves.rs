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
    /// Through what's in the way, dug (its cost the digging's).
    Dig,
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
        // Diggers dig every way (up only if it holds on: a climber).
        if p.dig.is_some() && (d.y <= 0 || p.climb) {
            out.push(Move { kind: Kind::Dig, d, secs: diag(d, p.run) });
        }
        // Everyone swims; swimmers well, the rest slowly.
        out.push(Move { kind: Kind::Swim, d, secs: diag(d, if p.swim > 0.0 { p.swim } else { p.run * 0.3 }) });
    }
    out
}

/// What a move costs besides its own time, from `from`, here (None: it
/// can't be made): digging, the time to dig what's in the way.
pub fn cost<W: NavWorld>(v: &mut View<W>, p: &Profile, from: NodePos, m: &Move) -> Option<f32> {
    if m.kind != Kind::Dig {
        return check(v, p, from, m).then_some(0.0);
    }
    p.dig?;
    let to = from + m.d;
    // From somewhere it can be, or a node it dug its way into (in its own
    // tunnel); into somewhere it doesn't fit (it digs), or out of its
    // tunnel into the open.
    let tunnel = !v.fits(from);
    if tunnel {
        // (Somewhere it could have dug: all of it diggable.)
        v.dig_secs(p, from, from + IVec2::new(0, 1 << 20))?;
    } else if !(v.stand(from).is_some() || (p.climb && v.hold(from))) {
        return None;
    }
    if v.fits(to) {
        return tunnel.then_some(0.0);
    }
    v.dig_secs(p, to, from)
}

/// Whether a move can be made from `from`, here.
pub fn check<W: NavWorld>(v: &mut View<W>, p: &Profile, from: NodePos, m: &Move) -> bool {
    let to = from + m.d;
    match m.kind {
        Kind::Dig => cost(v, p, from, m).is_some(),
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
        Kind::Jump(_) => {
            let Some(k) = p.moves.iter().position(|o| o == m) else { return false };
            let mut out = Vec::new();
            jumps(v, p, from, &mut out);
            out.iter().any(|(i, _)| *i as usize == k)
        }
        Kind::Climb => {
            let ok = |v: &mut View<W>, n: NodePos| v.hold(n) || v.stand(n).is_some();
            // (Round a corner it goes by the open side: one is enough.)
            let round = m.d.x == 0 || m.d.y == 0 || v.fits(from + IVec2::new(m.d.x, 0)) || v.fits(from + IVec2::new(0, m.d.y));
            ok(v, from) && ok(v, to) && round
        }
        Kind::Fly => v.fits(from) && v.fits(to) && corners(v, from, m.d),
        Kind::Swim => {
            let (a, b) = (v.wet(from), v.wet(to));
            // In it, along it; or out of it onto something (and in from it).
            (a && b || a && v.stand(to).is_some() || b && v.stand(from).is_some()) && v.fits(to) && corners(v, from, m.d)
        }
    }
}

/// The jumps a body can make from a node (moves by index, nothing extra to
/// pay), into `out`: each way of jumping's arc played over the cells once,
/// each way, a tick at a time, from where it takes off (the middle of its
/// node, its floor): up and down as it was; across as it was unless
/// something's in the way, when it stays put (the way across it lost is
/// lost: a body stopped by a wall goes on up it, not on through); no room
/// even where it is: no further. Coming down onto ground, it's there (if
/// the ground goes on a node past it: not onto an edge), and no further; a
/// climber catches hold of what it passes (from ground or a hold).
pub fn jumps<W: NavWorld>(v: &mut View<W>, p: &Profile, from: NodePos, out: &mut Vec<(u16, f32)>) {
    let floor = v.stand(from);
    let holds = p.climb && v.hold(from);
    if floor.is_none() && !holds {
        return;
    }
    let base = IVec2::new(from.x * NODE + NODE / 2, floor.unwrap_or(from.y * NODE));
    for a in &p.arcs {
        let lands = floor.is_some() && !a.lands.is_empty();
        if !lands && a.catches.is_empty() {
            continue;
        }
        for (side, flip) in [(0, 1), (1, -1)] {
            let push = |out: &mut Vec<(u16, f32)>, k: Option<&[u16; 2]>| {
                if let Some(&k) = k.map(|k| &k[side])
                    && k != u16::MAX
                    && !out.iter().any(|(i, _)| *i == k)
                {
                    out.push((k, 0.0));
                }
            };
            let (mut x, mut last, mut top, mut was) = (base.x, 0, 0, base.y);
            for s in &a.arc {
                let y = base.y + s.y;
                // Coming down onto a floor between where it was and where
                // it is (the first it meets): there.
                if lands && s.y < top {
                    let col = x.div_euclid(NODE);
                    let met = (y.div_euclid(NODE)..=was.div_euclid(NODE)).rev().find_map(|ny| v.stand(IVec2::new(col, ny)).filter(|f| y <= *f && *f <= was).map(|f| (ny, f)));
                    if let Some((ny, f)) = met {
                        let to = IVec2::new(col, ny);
                        let d = to - from;
                        let beyond = to + IVec2::new(flip, 0);
                        let on = [beyond, beyond + IVec2::Y, beyond - IVec2::Y].iter().any(|n| v.stand(*n).is_some());
                        if on && v.fits_cell(x, f) {
                            push(out, a.lands.get(&IVec2::new(d.x * flip, d.y)));
                        }
                        break;
                    }
                }
                top = top.max(s.y);
                let next = x + (s.x - last) * flip;
                last = s.x;
                if v.fits_cell(next, y) {
                    x = next;
                } else if !v.fits_cell(x, y) {
                    break;
                }
                was = y;
                if p.climb && !a.catches.is_empty() {
                    let n = IVec2::new(x.div_euclid(NODE), y.div_euclid(NODE));
                    let d = n - from;
                    if let Some(k) = a.catches.get(&IVec2::new(d.x * flip, d.y))
                        && v.hold(n)
                        && v.stand(n).is_none()
                    {
                        push(out, Some(k));
                    }
                }
            }
        }
    }
}

/// A diagonal doesn't cut a corner: both nodes beside it are open.
fn corners<W: NavWorld>(v: &mut View<W>, from: NodePos, d: IVec2) -> bool {
    d.x == 0 || d.y == 0 || (v.fits(from + IVec2::new(d.x, 0)) && v.fits(from + IVec2::new(0, d.y)))
}
