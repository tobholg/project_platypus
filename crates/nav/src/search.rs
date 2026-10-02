//! The searches: A* from a body toward a goal (`find`), and a map spreading
//! out from a target that many bodies walk downhill on (`field`). Both over
//! a profile's moves, both with a budget of nodes.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use rustc_hash::FxHashMap as HashMap;

use glam::IVec2;

use crate::NavWorld;
use crate::moves::{Kind, Move};
use crate::profile::Profile;
use crate::tile::{NODE, NodePos, View};

/// A way found: each step the node reached and how. `whole`: it reaches the
/// goal; else it goes as near as the search got.
#[derive(Clone, Debug, Default)]
pub struct Path {
    pub steps: Vec<(NodePos, Move)>,
    pub whole: bool,
    /// Nodes the search looked at.
    pub looked: usize,
}

#[derive(PartialEq)]
struct Open {
    f: f32,
    g: f32,
    n: NodePos,
}

impl Eq for Open {}

impl Ord for Open {
    fn cmp(&self, o: &Self) -> Ordering {
        // (Least first; ties to the one further along.)
        o.f.total_cmp(&self.f).then(self.g.total_cmp(&o.g))
    }
}

impl PartialOrd for Open {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// Where a body can be at a node, for its profile: standing (walkers), or
/// anywhere it fits (flyers), or holding on (climbers), or in liquid.
pub fn can_be<W: NavWorld>(v: &mut View<W>, p: &Profile, n: NodePos) -> bool {
    v.stand(n).is_some() || (p.fly > 0.0 && v.fits(n)) || (p.climb && v.hold(n)) || v.wet(n)
}

/// The node to start from: where it is, or the nearest it can be (a body
/// mid-jump, or its feet just over a node's line).
pub fn settle<W: NavWorld>(v: &mut View<W>, p: &Profile, at: NodePos) -> Option<NodePos> {
    let near = [IVec2::ZERO, IVec2::new(0, -1), IVec2::new(0, 1), IVec2::new(1, 0), IVec2::new(-1, 0), IVec2::new(0, -2), IVec2::new(1, -1), IVec2::new(-1, -1), IVec2::new(0, -3)];
    near.iter().map(|d| at + *d).find(|n| can_be(v, p, *n))
}

/// A* from `from` to within `near` nodes of `to`, looking at no more than
/// `budget` nodes.
pub fn find<W: NavWorld>(v: &mut View<W>, p: &Profile, from: NodePos, to: NodePos, near: i32, budget: usize) -> Path {
    let Some(start) = settle(v, p, from) else { return Path::default() };
    let speed = p.fastest() / NODE as f32;
    let guess = |n: NodePos| (n - to).as_vec2().length() / speed;
    let mut open = BinaryHeap::new();
    let mut came: HashMap<NodePos, (f32, Option<(NodePos, Move)>)> = HashMap::default();
    came.insert(start, (0.0, None));
    open.push(Open { f: guess(start), g: 0.0, n: start });
    let (mut best, mut best_h) = (start, guess(start));
    let mut looked = 0;
    let mut whole = false;
    while let Some(Open { g, n, .. }) = open.pop() {
        if came.get(&n).is_some_and(|c| c.0 < g) {
            continue;
        }
        looked += 1;
        let h = guess(n);
        if h < best_h {
            (best, best_h) = (n, h);
        }
        if (n - to).as_vec2().length() <= near as f32 {
            (best, whole) = (n, true);
            break;
        }
        if looked >= budget {
            break;
        }
        for &mi in v.edges(p, n).iter() {
            let m = &p.moves[mi as usize];
            let next = n + m.d;
            let ng = g + m.secs;
            if came.get(&next).is_some_and(|c| c.0 <= ng) {
                continue;
            }
            came.insert(next, (ng, Some((n, *m))));
            open.push(Open { f: ng + guess(next), g: ng, n: next });
        }
    }
    let mut steps = Vec::new();
    let mut at = best;
    while let Some((_, Some((prev, m)))) = came.get(&at) {
        steps.push((at, *m));
        at = *prev;
    }
    steps.reverse();
    Path { steps, whole, looked }
}

/// The way to a target from everywhere near it, for one profile: each
/// node's time to it and its first move.
#[derive(Clone, Debug)]
pub struct Field {
    pub goal: NodePos,
    lo: IVec2,
    w: i32,
    h: i32,
    cost: Vec<f32>,
    step: Vec<u16>,
    pub looked: usize,
}

impl Field {
    fn at(&self, n: NodePos) -> Option<usize> {
        let l = n - self.lo;
        (l.x >= 0 && l.y >= 0 && l.x < self.w && l.y < self.h).then(|| (l.y * self.w + l.x) as usize)
    }

    /// Seconds to the target from here (if it's known).
    pub fn cost(&self, n: NodePos) -> Option<f32> {
        self.at(n).map(|i| self.cost[i]).filter(|c| c.is_finite())
    }

    /// The first move from here toward the target (and where it leads).
    pub fn next<'p>(&self, p: &'p Profile, n: NodePos) -> Option<(NodePos, &'p Move)> {
        let i = self.at(n)?;
        let m = p.moves.get(*self.step.get(i)? as usize)?;
        self.cost[i].is_finite().then_some((n + m.d, m))
    }
}

/// Spread out from `goal` (and the nodes within `near` of it a body can
/// be at) across `lo`..`hi` (nodes), backward along every move, looking at
/// no more than `budget` nodes.
pub fn field<W: NavWorld>(v: &mut View<W>, p: &Profile, goal: NodePos, near: i32, lo: IVec2, hi: IVec2, budget: usize) -> Field {
    let (w, h) = ((hi.x - lo.x).max(1), (hi.y - lo.y).max(1));
    let mut f = Field { goal, lo, w, h, cost: vec![f32::INFINITY; (w * h) as usize], step: vec![u16::MAX; (w * h) as usize], looked: 0 };
    let mut open = BinaryHeap::new();
    for dy in -near..=near {
        for dx in -near..=near {
            let n = goal + IVec2::new(dx, dy);
            if dx * dx + dy * dy <= near * near
                && let Some(i) = f.at(n)
                && can_be(v, p, n)
            {
                f.cost[i] = 0.0;
                open.push(Open { f: 0.0, g: 0.0, n });
            }
        }
    }
    while let Some(Open { g, n, .. }) = open.pop() {
        let Some(i) = f.at(n) else { continue };
        if f.cost[i] < g {
            continue;
        }
        f.looked += 1;
        if f.looked >= budget {
            break;
        }
        // Who could get here by a move: from n - d.
        for (mi, m) in p.moves.iter().enumerate() {
            let from = n - m.d;
            let Some(j) = f.at(from) else { continue };
            let ng = g + m.secs;
            // (Somewhere it can be at all, first: most nodes are air or rock.)
            if f.cost[j] <= ng || !can_be(v, p, from) || !v.can(p, from, mi as u16) {
                continue;
            }
            f.cost[j] = ng;
            f.step[j] = mi as u16;
            open.push(Open { f: ng, g: ng, n: from });
        }
    }
    f
}

/// Whether a kind of move leaves the ground (the follower times it).
pub fn airborne(k: Kind) -> bool {
    matches!(k, Kind::Jump(_) | Kind::Drop)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::Nav;
    use platypus_physics::{Grid, MovementStats, Occupancy};

    /// A world drawn as text: `#` solid, `~` water, `-` a platform, `|` a
    /// wall behind; the bottom row is y 0. Outside: solid.
    struct Ascii(Vec<Vec<u8>>);

    impl Ascii {
        fn new(rows: &[&str]) -> Self {
            Ascii(rows.iter().rev().map(|r| r.bytes().collect()).collect())
        }
        fn cell(&self, x: i32, y: i32) -> u8 {
            if x < 0 || y < 0 {
                return b'#';
            }
            self.0.get(y as usize).and_then(|r| r.get(x as usize)).copied().unwrap_or(b'#')
        }
    }

    impl Grid for Ascii {
        fn occupancy(&self, x: i32, y: i32) -> Occupancy {
            match self.cell(x, y) {
                b'#' => Occupancy::Solid,
                b'~' => Occupancy::Liquid,
                b'-' => Occupancy::Platform,
                _ => Occupancy::Empty,
            }
        }
    }

    impl NavWorld for Ascii {
        fn backed(&self, x: i32, y: i32) -> bool {
            self.cell(x, y) == b'|'
        }
    }

    /// A room `w` × `h` cells, walls round it, and what `paint` adds.
    fn room(w: usize, h: usize, paint: impl Fn(usize, usize) -> Option<u8>) -> Ascii {
        let rows: Vec<String> = (0..h)
            .rev()
            .map(|y| (0..w).map(|x| if x < 4 || x >= w - 4 || y < 4 { '#' } else { paint(x, y).map_or(' ', |c| c as char) }).collect())
            .collect();
        Ascii::new(&rows.iter().map(String::as_str).collect::<Vec<_>>())
    }

    fn walker() -> Profile {
        Profile::new((9.0, 23.0), &MovementStats::default(), 40.0)
    }

    fn feet(x: i32, y: i32) -> NodePos {
        IVec2::new(x / NODE, y / NODE)
    }

    fn kinds(p: &Path) -> Vec<Kind> {
        p.steps.iter().map(|(_, m)| m.kind).collect()
    }

    #[test]
    fn it_walks_where_it_can_and_jumps_where_it_must() {
        let mut nav = Nav::default();
        // A flat floor: walking all the way.
        let flat = room(200, 80, |_, _| None);
        let p = walker();
        let path = find(&mut nav.view(&flat, p.size), &p, feet(20, 4), feet(180, 4), 0, 5000);
        assert!(path.whole, "{path:?}");
        assert!(kinds(&path).iter().all(|k| *k == Kind::Walk), "{:?}", kinds(&path));
        // A wall 30 high (too high to step, low enough to jump): a jump.
        let mut nav = Nav::default();
        let wall = room(200, 120, |x, y| ((100..112).contains(&x) && y < 34).then_some(b'#'));
        let path = find(&mut nav.view(&wall, p.size), &p, feet(20, 4), feet(180, 4), 0, 20000);
        assert!(path.whole, "over the wall: {:?}", kinds(&path));
        assert!(kinds(&path).iter().any(|k| matches!(k, Kind::Jump(_))), "{:?}", kinds(&path));
        // A wall far too high: no way (the nearest it can get, at the wall).
        let mut nav = Nav::default();
        let cliff = room(200, 160, |x, y| ((100..112).contains(&x) && y < 140).then_some(b'#'));
        let path = find(&mut nav.view(&cliff, p.size), &p, feet(20, 4), feet(180, 4), 0, 20000);
        assert!(!path.whole);
        let end = path.steps.last().map_or(0, |s| s.0.x * NODE);
        assert!((80..100).contains(&end), "stops at the wall: {end}");
    }

    #[test]
    fn it_crosses_a_gap_and_drops_off_a_ledge() {
        let p = walker();
        // A pit 24 wide and deep, too wide to step: jumped.
        let mut nav = Nav::default();
        let pit = room(200, 100, |x, y| (y < 40 && !(90..114).contains(&x)).then_some(b'#'));
        let path = find(&mut nav.view(&pit, p.size), &p, feet(20, 40), feet(180, 40), 0, 20000);
        assert!(path.whole && kinds(&path).iter().any(|k| matches!(k, Kind::Jump(_))), "{:?}", kinds(&path));
        // Down off a ledge 20 high: a drop, not a jump.
        let mut nav = Nav::default();
        let ledge = room(200, 100, |x, y| (x < 100 && y < 24).then_some(b'#'));
        let path = find(&mut nav.view(&ledge, p.size), &p, feet(20, 24), feet(180, 4), 0, 20000);
        // (Stepping off, or a hop down: either is fine.)
        assert!(path.whole && kinds(&path).iter().any(|k| matches!(k, Kind::Drop | Kind::Jump(_))), "{:?}", kinds(&path));
        // Too far to fall: it won't (a 200-cell cliff for one that falls 40).
        let mut nav = Nav::default();
        let deep = room(200, 260, |x, y| (x < 100 && y < 220).then_some(b'#'));
        let path = find(&mut nav.view(&deep, p.size), &p, feet(20, 220), feet(180, 4), 0, 20000);
        assert!(!path.whole, "{:?}", kinds(&path));
    }

    #[test]
    fn climbers_climb_flyers_fly_and_big_bodies_keep_out_of_small_holes() {
        // A climber up a wall 120 high to a shelf.
        let climber = Profile::new((24.0, 18.0), &MovementStats { cling: true, jump_height: 30.0, ..MovementStats::default() }, 40.0);
        let mut nav = Nav::default();
        let shelf = room(200, 200, |x, y| (x >= 120 && y < 124).then_some(b'#'));
        let path = find(&mut nav.view(&shelf, climber.size), &climber, feet(40, 4), feet(170, 124), 1, 20000);
        assert!(path.whole && kinds(&path).contains(&Kind::Climb), "{:?}", kinds(&path));
        // A flyer over a wall it couldn't jump.
        let flyer = Profile::new((9.0, 6.0), &MovementStats { fly_speed: 210.0, ..MovementStats::default() }, 40.0);
        let mut nav = Nav::default();
        let wall = room(200, 200, |x, y| ((100..112).contains(&x) && y < 150).then_some(b'#'));
        let path = find(&mut nav.view(&wall, flyer.size), &flyer, feet(30, 20), feet(170, 20), 0, 20000);
        assert!(path.whole && kinds(&path).iter().all(|k| *k == Kind::Fly), "{:?}", kinds(&path));
        // A tunnel 14 high: the 23-tall walker can't; a 9 × 8 one can.
        let tunnel = room(200, 100, |x, y| ((60..140).contains(&x) && y >= 18).then_some(b'#'));
        let mut nav = Nav::default();
        let tall = walker();
        assert!(!find(&mut nav.view(&tunnel, tall.size), &tall, feet(20, 4), feet(180, 4), 0, 20000).whole);
        let small = Profile::new((9.0, 8.0), &MovementStats::default(), 40.0);
        assert!(find(&mut nav.view(&tunnel, small.size), &small, feet(20, 4), feet(180, 4), 0, 20000).whole);
    }

    #[test]
    fn a_field_leads_the_same_way_a_search_does() {
        let p = walker();
        let mut nav = Nav::default();
        let wall = room(240, 120, |x, y| ((120..132).contains(&x) && y < 34).then_some(b'#'));
        let goal = feet(220, 4);
        let mut v = nav.view(&wall, p.size);
        let f = field(&mut v, &p, goal, 0, IVec2::new(0, 0), IVec2::new(60, 30), 50_000);
        // From the far side, walking the field gets there.
        let mut at = feet(20, 4);
        let mut steps = 0;
        while at != goal && steps < 200 {
            let Some((next, _)) = f.next(&p, at) else { panic!("no way on from {at}") };
            at = next;
            steps += 1;
        }
        assert_eq!(at, goal);
        // And it's no slower than A*'s way.
        let path = find(&mut v, &p, feet(20, 4), goal, 0, 50_000);
        let astar: f32 = path.steps.iter().map(|(_, m)| m.secs).sum();
        let fc = f.cost(feet(20, 4)).unwrap();
        assert!((fc - astar).abs() < 0.05, "field {fc:.3} s, A* {astar:.3} s");
    }

    #[test]
    fn up_onto_a_block_from_right_beside_it() {
        // An orc at the game's pace, a block 22 high it can't step: jumped,
        // from right against it as from further off.
        let k = 0.8f32;
        let mut st = MovementStats { run_speed: 93.0, jump_height: 36.0, step_height: 5, ..MovementStats::default() };
        st.run_speed *= k;
        st.max_fall *= k;
        for a in [&mut st.ground_accel, &mut st.ground_decel, &mut st.air_accel, &mut st.gravity] {
            *a *= k * k;
        }
        let p = Profile::new((11.0, 24.0), &st, 105.0);
        let block = room(220, 100, |x, y| ((100..120).contains(&x) && y < 26).then_some(b'#'));
        for start in [30, 92, 94] {
            let mut nav = Nav::default();
            let path = find(&mut nav.view(&block, p.size), &p, feet(start, 4), feet(110, 26), 0, 20000);
            assert!(path.whole, "from {start}: {:?}", kinds(&path));
        }
    }

    #[test]
    fn out_of_budget_it_goes_as_near_as_it_got() {
        let p = walker();
        let mut nav = Nav::default();
        let flat = room(400, 60, |_, _| None);
        let path = find(&mut nav.view(&flat, p.size), &p, feet(20, 4), feet(380, 4), 0, 30);
        assert!(!path.whole);
        assert!(path.looked <= 30);
        let end = path.steps.last().unwrap().0.x;
        assert!(end > feet(20, 4).x + 5, "it got somewhere: {end}");
    }
}
