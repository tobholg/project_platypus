//! Trees and tall grass. Trees are planned once for the whole world (a list,
//! sorted by x) and rasterised into whichever chunks they overlap, so a tree
//! crossing a chunk border comes out whole. They live in the background
//! layer: you walk in front of them, chop them, burn them.

use noise::{NoiseFn, Perlin};
use platypus_sim::rng::Rng;

/// Widest a tree reaches from its trunk, for chunk overlap tests.
pub const TREE_REACH: i32 = 40;
/// Deepest a trunk is rooted below the surface (anchors it in the ground).
const ROOTS: i32 = 5;

#[derive(Clone, Debug)]
pub struct Branch {
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub thick: f32,
}

#[derive(Clone, Debug)]
pub struct Tree {
    pub x: i32,
    /// First air cell above the ground at the trunk.
    pub base: i32,
    pub height: i32,
    /// Half-width of the trunk at its foot (cells).
    pub girth: f32,
    pub branches: Vec<Branch>,
    /// Leaf blobs: centre and radius.
    pub crowns: Vec<(f32, f32, f32)>,
}

/// What a tree puts at a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreePart {
    Wood,
    Leaves,
}

impl Tree {
    pub fn plan(x: i32, base: i32, rng: &mut Rng) -> Tree {
        let f = |rng: &mut Rng, lo: f32, hi: f32| lo + (hi - lo) * (rng.next_u32() as f32 / u32::MAX as f32);
        let height = f(rng, 34.0, 78.0) as i32;
        let girth = f(rng, 1.5, 3.0);
        let top = (x as f32, (base + height) as f32);
        let mut branches = Vec::new();
        let mut crowns = vec![(top.0, top.1 + 2.0, f(rng, 10.0, 15.0))];
        let n = 3 + (rng.next_u32() % 4) as usize;
        let first_right = rng.coin();
        for k in 0..n {
            // Alternate sides, starting on a random one; higher branches shorter.
            let dir = if (k % 2 == 0) == first_right { 1.0 } else { -1.0 };
            let at = f(rng, 0.42, 0.9);
            let y0 = base as f32 + height as f32 * at;
            let len = f(rng, 9.0, 24.0) * (1.15 - at * 0.5);
            let rise = len * f(rng, 0.25, 0.8);
            let end = (x as f32 + dir * len, y0 + rise);
            branches.push(Branch { from: (x as f32, y0), to: end, thick: if at < 0.65 { 1.6 } else { 1.1 } });
            crowns.push((end.0, end.1 + 1.0, f(rng, 6.0, 11.0)));
        }
        Tree { x, base, height, girth, branches, crowns }
    }

    /// Horizontal extent.
    pub fn span(&self) -> (i32, i32) {
        (self.x - TREE_REACH, self.x + TREE_REACH)
    }

    /// Wood or leaves at a cell, if any. `edge` is a noise field for ragged crowns.
    pub fn part_at(&self, x: i32, y: i32, edge: &Perlin) -> Option<TreePart> {
        if (x - self.x).abs() > TREE_REACH || y < self.base - ROOTS || y > self.base + self.height + 24 {
            return None;
        }
        let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5);
        // Trunk, tapering towards the top, rooted below the surface.
        if y >= self.base - ROOTS && y <= self.base + self.height {
            let t = ((y - self.base).max(0) as f32 / self.height as f32).min(1.0);
            let half = (self.girth * (1.0 - 0.6 * t)).max(0.6);
            if (xf - self.x as f32 - 0.5).abs() <= half {
                return Some(TreePart::Wood);
            }
        }
        for b in &self.branches {
            if segment_distance((xf, yf), b.from, b.to) <= b.thick * 0.5 + 0.2 {
                return Some(TreePart::Wood);
            }
        }
        for &(cx, cy, r) in &self.crowns {
            let (dx, dy) = (xf - cx, (yf - cy) * 1.25);
            let d2 = dx * dx + dy * dy;
            // Cheap reject before sampling noise: the ragged edge stays within ±28%.
            if d2 > (r * 1.28) * (r * 1.28) {
                continue;
            }
            if d2 <= (r * 0.72) * (r * 0.72) {
                return Some(TreePart::Leaves);
            }
            let ragged = r * (1.0 + 0.28 * edge.get([x as f64 * 0.18, y as f64 * 0.18]) as f32);
            if d2 <= ragged * ragged {
                return Some(TreePart::Leaves);
            }
        }
        None
    }
}

fn segment_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let len2 = (abx * abx + aby * aby).max(1e-6);
    let t = (((p.0 - a.0) * abx + (p.1 - a.1) * aby) / len2).clamp(0.0, 1.0);
    let (qx, qy) = (a.0 + abx * t, a.1 + aby * t);
    ((p.0 - qx).powi(2) + (p.1 - qy).powi(2)).sqrt()
}

/// Trees sorted by x, for fast lookup by chunk.
pub struct Forest {
    trees: Vec<Tree>,
}

impl Forest {
    /// `surface(x)` gives the first air cell above ground; `growable(x)` says
    /// whether a tree may stand there (not snow, not a cliff).
    pub fn plan(seed: u64, width: i32, surface: impl Fn(i32) -> i32, growable: impl Fn(i32) -> bool) -> Forest {
        let density = Perlin::new((seed ^ 0xF0_4E57) as u32);
        let mut rng = Rng::seeded(&[seed, 0x7EE5]);
        let mut trees = Vec::new();
        let mut x = TREE_REACH;
        while x < width - TREE_REACH {
            let d = density.get([x as f64 / 900.0, 0.5]);
            // Dense woods, open woodland, or meadow.
            let gap = if d > 0.15 { 18 } else if d > -0.2 { 55 } else { 0 };
            if gap == 0 {
                x += 40;
                continue;
            }
            let jitter = (rng.next_u32() % (gap as u32 / 2 + 1)) as i32;
            x += gap + jitter;
            if x >= width - TREE_REACH || !growable(x) {
                continue;
            }
            trees.push(Tree::plan(x, surface(x), &mut rng));
        }
        Forest { trees }
    }

    /// Trees whose extent overlaps `x0..=x1`.
    pub fn near(&self, x0: i32, x1: i32) -> &[Tree] {
        let lo = self.trees.partition_point(|t| t.span().1 < x0);
        let hi = self.trees.partition_point(|t| t.span().0 <= x1);
        &self.trees[lo..hi.max(lo)]
    }

    pub fn len(&self) -> usize {
        self.trees.len()
    }

    pub fn is_empty(&self) -> bool {
        self.trees.is_empty()
    }
}
