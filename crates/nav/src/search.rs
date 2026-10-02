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
    // (A digger: anywhere it could dig itself into, too.)
    v.stand(n).is_some() || (p.fly > 0.0 && v.fits(n)) || (p.climb && v.hold(n)) || v.wet(n) || (p.dig.is_some() && !v.fits(n))
}

/// The node to start from: where it is, or the nearest it can be (a body
/// mid-jump, or its feet just over a node's line).
pub fn settle<W: NavWorld>(v: &mut View<W>, p: &Profile, at: NodePos) -> Option<NodePos> {
    let near = [IVec2::ZERO, IVec2::new(0, -1), IVec2::new(0, 1), IVec2::new(1, 0), IVec2::new(-1, 0), IVec2::new(0, -2), IVec2::new(1, -1), IVec2::new(-1, -1), IVec2::new(0, -3)];
    // (Somewhere it fits first: a digger's feet just over a node's line
    // are not in a tunnel.)
    let open = |v: &mut View<W>, n: NodePos| v.stand(n).is_some() || (p.fly > 0.0 && v.fits(n)) || (p.climb && v.hold(n)) || v.wet(n);
    near.iter().map(|d| at + *d).find(|n| open(v, *n)).or_else(|| near.iter().map(|d| at + *d).find(|n| can_be(v, p, *n)))
}

/// A* from `from` to within `near` nodes of `to`, looking at no more than
/// `budget` nodes.
pub fn find<W: NavWorld>(v: &mut View<W>, p: &Profile, from: NodePos, to: NodePos, near: i32, budget: usize) -> Path {
    find_until(v, p, from, to, near, budget, None)
}

/// The same, stopping at `deadline` too (the way as near as it got by
/// then): a first search over new ground works out every node's moves.
pub fn find_until<W: NavWorld>(v: &mut View<W>, p: &Profile, from: NodePos, to: NodePos, near: i32, budget: usize, deadline: Option<std::time::Instant>) -> Path {
    let mut s = Search::new(v, p, from, to, near, budget);
    s.run(v, p, deadline).unwrap_or_else(|| s.so_far())
}

/// An A* that can be put down and taken up again (a tick's time for
/// planning spent, it goes on next tick): from `start` (where the body was,
/// settled) to within `near` nodes of `to`, looking at no more than
/// `budget` nodes. (What it learnt of the grid may be out of date by the
/// time it's done: the way's followed and looked at again anyway.)
pub struct Search {
    pub start: NodePos,
    pub to: NodePos,
    near: i32,
    budget: usize,
    open: BinaryHeap<Open>,
    came: HashMap<NodePos, (f32, Option<(NodePos, Move)>)>,
    best: NodePos,
    best_h: f32,
    looked: usize,
    whole: bool,
    over: bool,
}

impl std::fmt::Debug for Search {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Search {{ {} -> {}, looked {} }}", self.start, self.to, self.looked)
    }
}

impl Search {
    pub fn new<W: NavWorld>(v: &mut View<W>, p: &Profile, from: NodePos, to: NodePos, near: i32, budget: usize) -> Search {
        let start = settle(v, p, from);
        let mut s = Search { start: start.unwrap_or(from), to, near, budget, open: BinaryHeap::new(), came: HashMap::default(), best: from, best_h: f32::MAX, looked: 0, whole: false, over: start.is_none() };
        if let Some(start) = start {
            let h = s.guess(p, start);
            s.came.insert(start, (0.0, None));
            s.open.push(Open { f: h, g: 0.0, n: start });
            (s.best, s.best_h) = (start, h);
        }
        s
    }

    fn guess(&self, p: &Profile, n: NodePos) -> f32 {
        (n - self.to).as_vec2().length() / (p.fastest() / NODE as f32)
    }

    /// On until it's done (the way: whole, or as near as it got, its
    /// budget spent or nowhere left to look), or until `deadline` (None:
    /// not done yet).
    pub fn run<W: NavWorld>(&mut self, v: &mut View<W>, p: &Profile, deadline: Option<std::time::Instant>) -> Option<Path> {
        let mut here = 0;
        while !self.over {
            let Some(Open { g, n, .. }) = self.open.pop() else {
                self.over = true;
                break;
            };
            if self.came.get(&n).is_some_and(|c| c.0 < g) {
                continue;
            }
            self.looked += 1;
            here += 1;
            let h = self.guess(p, n);
            if h < self.best_h {
                (self.best, self.best_h) = (n, h);
            }
            if (n - self.to).as_vec2().length() <= self.near as f32 {
                (self.best, self.whole, self.over) = (n, true, true);
                break;
            }
            if self.looked >= self.budget {
                self.over = true;
                break;
            }
            for &(mi, extra) in v.edges(p, n).iter() {
                let m = &p.moves[mi as usize];
                let next = n + m.d;
                let ng = g + m.secs + extra;
                if self.came.get(&next).is_some_and(|c| c.0 <= ng) {
                    continue;
                }
                self.came.insert(next, (ng, Some((n, *m))));
                self.open.push(Open { f: ng + self.guess(p, next), g: ng, n: next });
            }
            if here % 16 == 0 && deadline.is_some_and(|d| std::time::Instant::now() >= d) {
                return None;
            }
        }
        Some(self.so_far())
    }

    /// The way as near as it's got.
    pub fn so_far(&self) -> Path {
        let mut steps = Vec::new();
        let mut at = self.best;
        while let Some((_, Some((prev, m)))) = self.came.get(&at) {
            steps.push((at, *m));
            at = *prev;
        }
        steps.reverse();
        Path { steps, whole: self.whole, looked: self.looked }
    }

    /// Nodes looked at so far.
    pub fn looked(&self) -> usize {
        self.looked
    }
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
            // (Somewhere it can be at all, first: most nodes are air or rock.)
            if f.cost[j] <= g + m.secs || !can_be(v, p, from) {
                continue;
            }
            let Some(extra) = v.can(p, from, mi as u16) else { continue };
            let ng = g + m.secs + extra;
            if f.cost[j] <= ng {
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
                b'#' | b'd' | b's' | b'o' | b'g' => Occupancy::Solid,
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

        /// `d` dirt (20), `s` stone (60), `o` obsidian (120), `g` glass
        /// (30, nothing eats it); `#` bedrock.
        fn solid_cell(&self, x: i32, y: i32) -> Option<(u8, bool)> {
            match self.cell(x, y) {
                b'd' => Some((20, false)),
                b's' => Some((60, false)),
                b'o' => Some((120, false)),
                b'g' => Some((30, true)),
                b'#' => Some((255, true)),
                _ => None,
            }
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
    fn a_digger_digs_in_slower_through_stone_and_not_through_obsidian_or_glass() {
        // The cave spider: claws through dirt, acid through stone.
        let dig = crate::profile::Digging { claws: 25, claw_rate: 40.0, acid: 90, acid_rate: 6.0 };
        let spider = Profile::new((24.0, 18.0), &MovementStats { cling: true, jump_height: 30.0, run_speed: 120.0, ..MovementStats::default() }, 60.0).digging(dig);
        // The player in a room walled in by a block 30 thick of `wall`.
        let walled = |wall: u8| room(240, 120, move |x, y| ((100..170).contains(&x) && y < 60 && !((115..155).contains(&x) && (4..40).contains(&y))).then_some(wall));
        let time = |wall: u8| {
            let w = walled(wall);
            let mut nav = Nav::default();
            let path = find(&mut nav.view(&w, spider.size), &spider, feet(30, 4), feet(135, 4), 1, 50_000);
            path.whole.then(|| path.steps.iter().map(|(_, m)| m.secs).sum::<f32>()).map(|_| {
                // (Its cost with the digging: the search's own sum.)
                let mut v = nav.view(&w, spider.size);
                let mut at = feet(30, 4);
                let mut total = 0.0;
                for (n, m) in &path.steps {
                    total += m.secs + crate::moves::cost(&mut v, &spider, at, m).unwrap_or(0.0);
                    at = *n;
                }
                total
            })
        };
        let dirt = time(b'd').expect("through dirt");
        let stone = time(b's').expect("through stone");
        assert!(stone > dirt * 2.0, "stone {stone:.1} s, dirt {dirt:.1} s");
        assert!(time(b'o').is_none(), "obsidian stops it");
        assert!(time(b'g').is_none(), "glass stops it");
        // One that doesn't dig can't get in at all.
        let plain = Profile::new((24.0, 18.0), &MovementStats { cling: true, jump_height: 30.0, ..MovementStats::default() }, 60.0);
        let w = walled(b'd');
        let mut nav = Nav::default();
        assert!(!find(&mut nav.view(&w, plain.size), &plain, feet(30, 4), feet(135, 4), 1, 50_000).whole);
    }

    #[test]
    fn a_search_put_down_and_taken_up_finds_the_same_way() {
        let p = walker();
        let w = room(300, 80, |x, y| ((100..112).contains(&x) && y < 20 || (180..200).contains(&x) && y < 10).then_some(b'#'));
        let mut nav = Nav::default();
        let whole = find(&mut nav.view(&w, p.size), &p, feet(20, 4), feet(260, 4), 1, 5000);
        assert!(whole.whole);
        // (A deadline already gone: a slice of nodes a time.)
        let mut nav = Nav::default();
        let mut v = nav.view(&w, p.size);
        let mut s = Search::new(&mut v, &p, feet(20, 4), feet(260, 4), 1, 5000);
        let mut slices = 0;
        let sliced = loop {
            slices += 1;
            if let Some(path) = s.run(&mut v, &p, Some(std::time::Instant::now())) {
                break path;
            }
        };
        assert!(slices > 2, "it was put down: {slices} slices");
        let nodes = |p: &Path| p.steps.iter().map(|(n, _)| *n).collect::<Vec<_>>();
        assert_eq!(nodes(&sliced), nodes(&whole));
        assert_eq!(sliced.looked, whole.looked);
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
