//! Trees and tall grass. Trees are planned once for the whole world (a list,
//! sorted by x) and rasterised into whichever chunks they overlap, so a tree
//! crossing a chunk border comes out whole. They live in the background
//! layer: you walk in front of them, chop them, burn them.
//!
//! A tree is a trunk (flared at the root, tapering, slightly leaning), a set
//! of branches with twigs, and clusters of foliage blobs around every branch
//! end. Parts come with a shade so bark reads as round (lit from the right)
//! and crowns are lighter on top.

use noise::{NoiseFn, Perlin};
use platypus_sim::rng::Rng;

/// Widest a tree reaches from its trunk, for chunk overlap tests.
pub const TREE_REACH: i32 = 140;
/// Deepest a trunk is rooted below the surface (anchors it in the ground).
const ROOTS: i32 = 6;

#[derive(Clone, Debug)]
pub struct Branch {
    pub from: (f32, f32),
    pub to: (f32, f32),
    /// Thickness at `from` and at `to`.
    pub thick: (f32, f32),
}

#[derive(Clone, Copy, Debug)]
pub struct Blob {
    pub x: f32,
    pub y: f32,
    pub r: f32,
}

#[derive(Clone, Debug)]
pub struct Tree {
    pub x: i32,
    /// First air cell above the ground at the trunk.
    pub base: i32,
    pub height: i32,
    /// Half-width of the trunk just above the flare.
    pub girth: f32,
    /// Sideways drift of the trunk at the top (cells).
    pub lean: f32,
    pub branches: Vec<Branch>,
    pub blobs: Vec<Blob>,
    /// Bounding box (min x, min y, max x, max y) for quick rejects.
    pub bbox: (i32, i32, i32, i32),
}

/// What a tree puts at a cell, and a shade (0 dark … 255 light) for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreePart {
    Wood(u8),
    Leaves(u8),
}

fn unit(rng: &mut Rng) -> f32 {
    rng.next_u32() as f32 / u32::MAX as f32
}

fn range(rng: &mut Rng, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * unit(rng)
}

/// A cluster of foliage: one big blob and several smaller ones around it.
fn foliage(blobs: &mut Vec<Blob>, rng: &mut Rng, cx: f32, cy: f32, r: f32) {
    blobs.push(Blob { x: cx, y: cy, r });
    for _ in 0..(3 + rng.next_u32() % 3) {
        let a = unit(rng) * std::f32::consts::TAU;
        let d = r * range(rng, 0.45, 0.8);
        blobs.push(Blob { x: cx + a.cos() * d, y: cy + a.sin() * d * 0.7 + r * 0.15, r: r * range(rng, 0.55, 0.8) });
    }
}

impl Tree {
    pub fn plan(x: i32, base: i32, rng: &mut Rng) -> Tree {
        // Mostly medium trees, some saplings, big ones, and one in eight a
        // giant (up to ~160 cells tall).
        let size = if rng.chance(32) { range(rng, 1.9, 2.5) } else { 0.55 + 1.35 * unit(rng).powf(1.7) };
        let height = (range(rng, 46.0, 66.0) * size) as i32;
        let h = height as f32;
        let girth = (range(rng, 3.0, 4.4) * size.powf(0.9)).max(2.0);
        let lean = range(rng, -0.08, 0.08) * h;
        let trunk_x = |t: f32| x as f32 + 0.5 + lean * t * t;

        let mut branches = Vec::new();
        let mut blobs = Vec::new();

        // Crown on top of the trunk.
        let top = (trunk_x(1.0), base as f32 + h);
        let r = h * range(rng, 0.22, 0.28);
        foliage(&mut blobs, rng, top.0, top.1 + h * 0.02, r);

        let n = 3 + (rng.next_u32() % 3) as usize + (size > 1.2) as usize * 2;
        let mut side = if rng.coin() { 1.0 } else { -1.0 };
        for k in 0..n {
            side = -side;
            let t = 0.38 + 0.52 * (k as f32 + unit(rng) * 0.6) / n as f32;
            let from = (trunk_x(t), base as f32 + h * t);
            let len = h * range(rng, 0.24, 0.4) * (1.2 - t * 0.55);
            let angle = range(rng, 0.35, 0.95); // radians above horizontal
            let to = (from.0 + side * len * angle.cos(), from.1 + len * angle.sin());
            let thick0 = (girth * range(rng, 0.55, 0.75) * (1.25 - t * 0.6)).max(1.4);
            branches.push(Branch { from, to, thick: (thick0, (thick0 * 0.45).max(0.9)) });
            // A twig from the middle, angled further out.
            if rng.chance(170) {
                let m = (from.0 + (to.0 - from.0) * 0.55, from.1 + (to.1 - from.1) * 0.55);
                let tl = len * range(rng, 0.35, 0.5);
                let ta = angle + range(rng, 0.2, 0.5);
                let tt = (m.0 + side * tl * ta.cos() * 0.6, m.1 + tl * ta.sin());
                branches.push(Branch { from: m, to: tt, thick: ((thick0 * 0.5).max(0.9), 0.8) });
                let r = h * range(rng, 0.11, 0.15);
                foliage(&mut blobs, rng, tt.0, tt.1, r);
            }
            let r = h * range(rng, 0.15, 0.2);
            foliage(&mut blobs, rng, to.0, to.1 + 1.0, r);
            // Leaves partway along the branch too, so the canopy reads as one mass.
            let mid = (from.0 + (to.0 - from.0) * 0.6, from.1 + (to.1 - from.1) * 0.6 + h * 0.04);
            blobs.push(Blob { x: mid.0, y: mid.1, r: h * range(rng, 0.1, 0.13) });
        }

        let mut bbox = (x - (girth * 2.0) as i32 - 2, base - ROOTS, x + (girth * 2.0) as i32 + 2, base + height);
        for b in &blobs {
            bbox.0 = bbox.0.min((b.x - b.r * 1.3) as i32);
            bbox.2 = bbox.2.max((b.x + b.r * 1.3) as i32 + 1);
            bbox.3 = bbox.3.max((b.y + b.r * 1.3) as i32 + 1);
        }
        for b in &branches {
            bbox.0 = bbox.0.min(b.to.0 as i32 - 2);
            bbox.2 = bbox.2.max(b.to.0 as i32 + 2);
        }
        Tree { x, base, height, girth, lean, branches, blobs, bbox }
    }

    /// Horizontal extent.
    pub fn span(&self) -> (i32, i32) {
        (self.bbox.0, self.bbox.2)
    }

    /// Wood or leaves at a cell, with a shade. `edge` is noise for ragged crowns.
    pub fn part_at(&self, x: i32, y: i32, edge: &Perlin) -> Option<TreePart> {
        let (x0, y0, x1, y1) = self.bbox;
        if x < x0 || x > x1 || y < y0 || y > y1 {
            return None;
        }
        let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5);

        // Trunk: flared roots, taper, a gentle lean.
        if y <= self.base + self.height {
            let t = ((yf - self.base as f32) / self.height as f32).clamp(0.0, 1.0);
            let flare = if t < 0.1 { 1.0 + 0.9 * (1.0 - t / 0.1).powi(2) } else { 1.0 };
            let half = (self.girth * (1.0 - 0.55 * t) * flare).max(0.9);
            let cx = self.x as f32 + 0.5 + self.lean * t * t;
            let u = (xf - cx) / half; // -1 … 1 across the trunk
            if u.abs() <= 1.0 {
                return Some(TreePart::Wood(bark(u, x, y)));
            }
        }
        for b in &self.branches {
            let (d, t) = segment_distance((xf, yf), b.from, b.to);
            // At least ~0.75 so a diagonal branch stays edge-connected
            // (corner-only pixels break off as floating specks).
            let half = ((b.thick.0 + (b.thick.1 - b.thick.0) * t) * 0.5 + 0.15).max(0.75);
            if d <= half {
                // Branches: lit from above.
                let u = ((yf - (b.from.1 + (b.to.1 - b.from.1) * t)) / half).clamp(-1.0, 1.0);
                return Some(TreePart::Wood(bark(u, x, y)));
            }
        }
        // Leaves: take the blob this cell is deepest inside, for its lighting.
        let mut best: Option<(f32, f32)> = None; // (depth, light)
        for b in &self.blobs {
            let (dx, dy) = (xf - b.x, (yf - b.y) * 1.15);
            let d2 = dx * dx + dy * dy;
            let outer = b.r * 1.25;
            if d2 > outer * outer {
                continue;
            }
            // Ragged by direction from the blob's centre, not per cell: every
            // leaf then has a straight run of leaves back to the centre, so
            // no islands float just outside the edge.
            let d = d2.sqrt().max(1e-3);
            let dir = [(dx / d) as f64 * 1.6 + b.x as f64 * 0.37, (dy / d) as f64 * 1.6 + b.y as f64 * 0.37];
            let ragged = b.r * (1.0 + 0.25 * edge.get(dir) as f32);
            if d <= ragged {
                let depth = 1.0 - d / ragged;
                if best.is_none_or(|(v, _)| depth > v) {
                    // Lighter towards the top-right of each blob, darker deep inside.
                    let light = (0.55 + 0.4 * (yf - b.y) / b.r + 0.2 * (xf - b.x) / b.r - 0.25 * depth).clamp(0.0, 1.0);
                    best = Some((depth, light));
                }
            }
        }
        best.map(|(_, light)| {
            let jitter = edge.get([x as f64 * 0.9, y as f64 * 0.9]) as f32 * 0.25;
            TreePart::Leaves(((light + jitter).clamp(0.0, 1.0) * 255.0) as u8)
        })
    }
}

/// Bark shade across a trunk or branch: dark on the left/bottom, lit on the
/// right/top, with vertical grain.
fn bark(u: f32, x: i32, y: i32) -> u8 {
    let grain = ((x.wrapping_mul(7919) ^ (y / 3).wrapping_mul(104_729)) & 31) as f32 / 31.0 - 0.5;
    ((0.5 + 0.42 * u + 0.18 * grain).clamp(0.0, 1.0) * 255.0) as u8
}

/// Distance from `p` to segment a–b, and where along it (0..1).
fn segment_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let len2 = (abx * abx + aby * aby).max(1e-6);
    let t = (((p.0 - a.0) * abx + (p.1 - a.1) * aby) / len2).clamp(0.0, 1.0);
    let (qx, qy) = (a.0 + abx * t, a.1 + aby * t);
    (((p.0 - qx).powi(2) + (p.1 - qy).powi(2)).sqrt(), t)
}

/// Trees sorted by the left edge of their extent, for fast lookup by chunk.
pub struct Forest {
    trees: Vec<Tree>,
}

impl Forest {
    /// `surface(x)` gives the first air cell above ground; `growable(x)` says
    /// whether a tree may stand there (not snow, not a cliff).
    pub fn plan(seed: u64, width: i32, surface: impl Fn(i32) -> i32, growable: impl Fn(i32) -> bool) -> Forest {
        let density = Perlin::new((seed ^ 0xF0_4E57) as u32);
        let mut rng = Rng::seeded(&[seed, 0x7EE5]);
        let mut trees: Vec<Tree> = Vec::new();
        let mut x = TREE_REACH;
        while x < width - TREE_REACH {
            let d = density.get([x as f64 / 1100.0, 0.5]);
            // Dense woods, open woodland, or meadow.
            let gap = if d > 0.12 { 34 } else if d > -0.22 { 90 } else { 0 };
            if gap == 0 {
                x += 60;
                continue;
            }
            x += gap + (rng.next_u32() % (gap as u32 / 2 + 1)) as i32;
            if x >= width - TREE_REACH || !growable(x) {
                continue;
            }
            trees.push(Tree::plan(x, surface(x), &mut rng));
        }
        trees.sort_by_key(|t| t.span().0);
        Forest { trees }
    }

    /// Trees whose extent overlaps `x0..=x1`.
    pub fn near(&self, x0: i32, x1: i32) -> Vec<&Tree> {
        // Sorted by left edge; no tree is wider than 2 × TREE_REACH.
        let lo = self.trees.partition_point(|t| t.span().0 < x0 - 2 * TREE_REACH);
        let hi = self.trees.partition_point(|t| t.span().0 <= x1);
        self.trees[lo..hi.max(lo)].iter().filter(|t| t.span().1 >= x0).collect()
    }

    pub fn len(&self) -> usize {
        self.trees.len()
    }

    pub fn is_empty(&self) -> bool {
        self.trees.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Forest::near` relies on this bound; giants are the ones that test it.
    #[test]
    fn every_tree_fits_its_reach_and_some_are_giants() {
        let mut rng = Rng::seeded(&[9, 9]);
        let mut tallest = 0;
        for _ in 0..4000 {
            let t = Tree::plan(0, 0, &mut rng);
            let (l, r) = t.span();
            assert!(-l <= TREE_REACH && r <= TREE_REACH, "tree {} tall spans {l}..{r}", t.height);
            tallest = tallest.max(t.height);
        }
        assert!(tallest >= 140, "giants grow ({tallest})");
    }
}
