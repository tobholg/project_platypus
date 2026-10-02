//! The coarse grid, a tile (one chunk) at a time, for one size of body.

use glam::{IVec2, Vec2};
use platypus_physics::Occupancy;
use rustc_hash::FxHashMap as HashMap;

use crate::NavWorld;
use crate::moves::check;
use crate::profile::Profile;

/// Cells to a node, each way.
pub const NODE: i32 = 4;
/// Nodes to a tile, each way (`TILE` × `NODE` = 64 cells: a chunk).
pub const TILE: i32 = 16;
const TILE_CELLS: i32 = TILE * NODE;

/// A node, in node coordinates (cells / `NODE`).
pub type NodePos = IVec2;

/// The node a world point (cells) is in.
pub fn node_of(cell: Vec2) -> NodePos {
    (cell / NODE as f32).floor().as_ivec2()
}

/// The tile a node is in.
pub fn tile_of(n: NodePos) -> IVec2 {
    IVec2::new(n.x.div_euclid(TILE), n.y.div_euclid(TILE))
}

/// Where a body standing at the bottom of a node has its feet (the middle
/// of its bottom edge, in cells).
pub fn node_feet(n: NodePos) -> Vec2 {
    Vec2::new((n.x * NODE + NODE / 2) as f32, (n.y * NODE) as f32)
}

/// One tile, for one size of body: per node (row-major, 16 × 16), the
/// floor's height in it (cells up from its bottom; -1: nothing to stand
/// on), and bits per row: fits (the body free with its feet at the node's
/// bottom), wet (its middle in liquid), hold (a climber can hold on there).
#[derive(Clone)]
pub struct Tile {
    /// Each of its cells' clearance: open cells upward, up to the body's
    /// height (a jump's arc is checked against these, to the cell).
    open: Box<[u8]>,
    floor: [i8; (TILE * TILE) as usize],
    fits: [u16; TILE as usize],
    wet: [u16; TILE as usize],
    hold: [u16; TILE as usize],
}

impl Tile {
    /// Made from the world (the tile's cells, and a body's size round them).
    pub fn build(world: &impl NavWorld, tile: IVec2, w: i32, h: i32) -> Tile {
        let (x0, y0) = (tile.x * TILE_CELLS, tile.y * TILE_CELLS);
        // (A body centred on a cell overlaps `half` cells each way: an odd
        // width's edges fall in the middle of a cell.)
        let half = (w + 1) / 2;
        // Columns a body centred in any node here can cover, a cell beyond
        // for its sides; rows from the one under the tile to a body's height
        // over it.
        // (And a node's width beyond, where a climber reaches for a wall.)
        let (cx0, cx1) = (x0 + NODE / 2 - half - NODE, x0 + TILE_CELLS + w + NODE);
        let (ry0, ry1) = (y0 - 1, y0 + TILE_CELLS + h + NODE);
        let (cols, rows) = ((cx1 - cx0) as usize, (ry1 - ry0) as usize);
        // Open cells upward from each (capped at the body's height: all a
        // fit needs), and what stands under each.
        let mut up = vec![0u8; cols * rows];
        let mut solid = vec![false; cols * rows];
        let mut under = vec![false; cols * rows];
        let cap = h.clamp(1, 255) as u8;
        let mut column = vec![Occupancy::Empty; rows];
        for c in 0..cols {
            let x = cx0 + c as i32;
            world.column(x, ry0, &mut column);
            let mut above = cap;
            for r in (0..rows).rev() {
                let o = column[r];
                let i = c * rows + r;
                solid[i] = o == Occupancy::Solid;
                under[i] = matches!(o, Occupancy::Solid | Occupancy::Platform);
                up[i] = if solid[i] { 0 } else { above.saturating_add(1).min(cap) };
                above = up[i];
            }
        }
        let at = |x: i32, y: i32| ((x - cx0) as usize) * rows + (y - ry0) as usize;
        let fits = |xc: i32, y: i32| (xc - half..xc + half).all(|x| up[at(x, y)] >= cap);
        let stood_on = |xc: i32, y: i32| (xc - half..xc + half).any(|x| under[at(x, y - 1)]);
        let mut open = vec![0u8; (TILE_CELLS * TILE_CELLS) as usize].into_boxed_slice();
        for y in 0..TILE_CELLS {
            for x in 0..TILE_CELLS {
                open[(y * TILE_CELLS + x) as usize] = up[at(x0 + x, y0 + y)];
            }
        }
        let mut t = Tile { open, floor: [-1; (TILE * TILE) as usize], fits: [0; TILE as usize], wet: [0; TILE as usize], hold: [0; TILE as usize] };
        for ny in 0..TILE {
            for nx in 0..TILE {
                let xc = x0 + nx * NODE + NODE / 2;
                let yb = y0 + ny * NODE;
                let bit = 1u16 << nx;
                for oy in 0..NODE {
                    if fits(xc, yb + oy) && stood_on(xc, yb + oy) {
                        t.floor[(ny * TILE + nx) as usize] = oy as i8;
                        break;
                    }
                }
                if !fits(xc, yb) {
                    continue;
                }
                t.fits[ny as usize] |= bit;
                if world.occupancy(xc, yb + h / 2) == Occupancy::Liquid {
                    t.wet[ny as usize] |= bit;
                }
                // Something to hold on to, within a node's reach: a wall at
                // either side, the ceiling over it, the ground, or a wall
                // behind.
                let side = |x: i32| [yb, yb + h / 2, yb + h - 1].iter().any(|&y| solid[at(x, y)]);
                let walled = (1..=NODE).any(|d| side(xc - half - d) || side(xc + half - 1 + d));
                let roof = (0..NODE).any(|d| (xc - half..xc + half).any(|x| solid[at(x, yb + h + d)]));
                if walled || roof || stood_on(xc, yb) || world.backed(xc, yb + h / 2) {
                    t.hold[ny as usize] |= bit;
                }
            }
        }
        t
    }

    fn bit(rows: &[u16; TILE as usize], l: IVec2) -> bool {
        rows[l.y as usize] & (1 << l.x) != 0
    }
}

/// A tile's nodes' moves, for one profile (none: not worked out yet).
type EdgeTile = Vec<Option<Box<[u16]>>>;

/// Every tile made so far, by tile and body size: kept in one list, found
/// by an index (a search asks thousands of times a node; the view keeps the
/// last few at hand).
#[derive(Default)]
pub struct Nav {
    arena: Vec<Tile>,
    index: HashMap<(IVec2, (u16, u16)), u32>,
    free: Vec<u32>,
    /// Each node's moves that can be made from it, by profile and tile
    /// (worked out once; forgotten with the tiles round it).
    edges: HashMap<(u64, IVec2), EdgeTile>,
    /// Tiles made (for readouts).
    pub built: u64,
}

impl Nav {
    /// A tile's cells changed: its tiles (every size) made again when next
    /// needed.
    pub fn forget(&mut self, tile: IVec2) {
        let gone: Vec<(IVec2, (u16, u16))> = self.index.keys().filter(|k| k.0 == tile).copied().collect();
        for k in gone {
            if let Some(i) = self.index.remove(&k) {
                self.free.push(i);
            }
        }
        // (A move from a tile beside it may pass through it: a jump.)
        self.edges.retain(|(_, t), _| (t.x - tile.x).abs() > 1 || (t.y - tile.y).abs() > 1);
    }

    pub fn forget_all(&mut self) {
        self.arena.clear();
        self.index.clear();
        self.free.clear();
        self.edges.clear();
    }

    /// How many tiles it holds (all sizes).
    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// The grid as one size of body sees it, over a world.
    pub fn view<'a, W: NavWorld>(&'a mut self, world: &'a W, size: (u16, u16)) -> View<'a, W> {
        View { nav: self, world, size, near: [(IVec2::MAX, 0); 4], next: 0 }
    }

    fn slot(&mut self, world: &impl NavWorld, t: IVec2, size: (u16, u16)) -> u32 {
        if let Some(&i) = self.index.get(&(t, size)) {
            return i;
        }
        let tile = Tile::build(world, t, size.0 as i32, size.1 as i32);
        self.built += 1;
        let i = match self.free.pop() {
            Some(i) => {
                self.arena[i as usize] = tile;
                i
            }
            None => {
                self.arena.push(tile);
                (self.arena.len() - 1) as u32
            }
        };
        self.index.insert((t, size), i);
        i
    }
}

/// The grid for one size of body (tiles made as they're asked for).
pub struct View<'a, W: NavWorld> {
    nav: &'a mut Nav,
    world: &'a W,
    pub size: (u16, u16),
    /// The last few tiles asked for, and which to replace next.
    near: [(IVec2, u32); 4],
    next: usize,
}

impl<W: NavWorld> View<'_, W> {
    /// Whether the body fits with its feet's middle at this cell (to the
    /// cell: a jump's arc).
    pub fn fits_cell(&mut self, x: i32, y: i32) -> bool {
        let (w, h) = (self.size.0 as i32, self.size.1 as i32);
        let cap = h.clamp(1, 255) as u8;
        let half = (w + 1) / 2;
        (x - half..x + half).all(|cx| {
            let n = IVec2::new(cx.div_euclid(NODE), y.div_euclid(NODE));
            self.with(n, |t, _| {
                let (lx, ly) = (cx.rem_euclid(TILE_CELLS), y.rem_euclid(TILE_CELLS));
                t.open[(ly * TILE_CELLS + lx) as usize] >= cap
            })
        })
    }

    #[inline]
    fn with<T>(&mut self, n: NodePos, f: impl FnOnce(&Tile, IVec2) -> T) -> T {
        let t = tile_of(n);
        let local = n - t * TILE;
        let i = match self.near.iter().find(|(p, _)| *p == t) {
            Some(&(_, i)) => i,
            None => {
                let i = self.nav.slot(self.world, t, self.size);
                self.near[self.next] = (t, i);
                self.next = (self.next + 1) % self.near.len();
                i
            }
        };
        f(&self.nav.arena[i as usize], local)
    }

    /// Where a body can stand in this node: the floor's height (cells).
    #[inline]
    pub fn stand(&mut self, n: NodePos) -> Option<i32> {
        self.with(n, |t, l| {
            let f = t.floor[(l.y * TILE + l.x) as usize];
            (f >= 0).then_some(n.y * NODE + f as i32)
        })
    }

    #[inline]
    pub fn fits(&mut self, n: NodePos) -> bool {
        self.with(n, |t, l| Tile::bit(&t.fits, l))
    }

    #[inline]
    pub fn wet(&mut self, n: NodePos) -> bool {
        self.with(n, |t, l| Tile::bit(&t.wet, l))
    }

    #[inline]
    pub fn hold(&mut self, n: NodePos) -> bool {
        self.with(n, |t, l| Tile::bit(&t.hold, l))
    }

    /// The moves a profile can make from a node (indices into its
    /// `moves`), worked out the first time and kept.
    pub fn edges(&mut self, p: &Profile, n: NodePos) -> Box<[u16]> {
        self.with_edges(p, n, |e| e.into())
    }

    /// Whether a profile can make a move (by its index) from a node.
    pub fn can(&mut self, p: &Profile, n: NodePos, m: u16) -> bool {
        self.with_edges(p, n, |e| e.contains(&m))
    }

    fn with_edges<T>(&mut self, p: &Profile, n: NodePos, f: impl FnOnce(&[u16]) -> T) -> T {
        let t = tile_of(n);
        let l = n - t * TILE;
        let i = (l.y * TILE + l.x) as usize;
        if let Some(Some(e)) = self.nav.edges.get(&(p.key, t)).map(|v| &v[i]) {
            return f(e);
        }
        let e: Box<[u16]> = p.moves.iter().enumerate().filter(|(_, m)| check(self, p, n, m)).map(|(k, _)| k as u16).collect();
        let out = f(&e);
        self.nav.edges.entry((p.key, t)).or_insert_with(|| vec![None; (TILE * TILE) as usize])[i] = Some(e);
        out
    }
}
