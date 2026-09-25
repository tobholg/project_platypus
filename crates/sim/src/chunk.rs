use std::sync::atomic::{AtomicBool, AtomicI32, Ordering::Relaxed};

use crate::cell::Cell;
use crate::coords::{CHUNK_AREA, ChunkPos, Rect, local_index};

/// A dirty rectangle that several stepping jobs may grow concurrently.
#[derive(Debug)]
pub(crate) struct AtomicRect {
    min_x: AtomicI32,
    min_y: AtomicI32,
    max_x: AtomicI32,
    max_y: AtomicI32,
}

impl AtomicRect {
    pub(crate) fn new(r: Rect) -> Self {
        AtomicRect {
            min_x: AtomicI32::new(r.min_x),
            min_y: AtomicI32::new(r.min_y),
            max_x: AtomicI32::new(r.max_x),
            max_y: AtomicI32::new(r.max_y),
        }
    }

    /// Commutative, so the result does not depend on job order.
    #[inline]
    pub(crate) fn include(&self, r: Rect) {
        if r.is_empty() {
            return;
        }
        self.min_x.fetch_min(r.min_x, Relaxed);
        self.min_y.fetch_min(r.min_y, Relaxed);
        self.max_x.fetch_max(r.max_x, Relaxed);
        self.max_y.fetch_max(r.max_y, Relaxed);
    }

    pub(crate) fn take(&self) -> Rect {
        Rect {
            min_x: self.min_x.swap(Rect::EMPTY.min_x, Relaxed),
            min_y: self.min_y.swap(Rect::EMPTY.min_y, Relaxed),
            max_x: self.max_x.swap(Rect::EMPTY.max_x, Relaxed),
            max_y: self.max_y.swap(Rect::EMPTY.max_y, Relaxed),
        }
    }

    pub(crate) fn peek(&self) -> Rect {
        Rect {
            min_x: self.min_x.load(Relaxed),
            min_y: self.min_y.load(Relaxed),
            max_x: self.max_x.load(Relaxed),
            max_y: self.max_y.load(Relaxed),
        }
    }
}

pub struct Chunk {
    pub pos: ChunkPos,
    /// The playfield: what creatures collide with and most rules act on.
    pub(crate) cells: Box<[Cell]>,
    /// The background layer behind it (walls, tree trunks, leaves). Creatures
    /// pass in front of it; it burns, can be mined and blown up, and falls
    /// into the playfield when nothing holds it up (SPEC §3.10).
    pub(crate) bg: Box<[Cell]>,
    /// Region to update next tick.
    pub(crate) next_dirty: AtomicRect,
    /// Cells changed since the renderer last looked.
    pub(crate) render_dirty: AtomicBool,
    /// Differs from what worldgen would produce; must be persisted.
    pub(crate) modified: AtomicBool,
}

impl Chunk {
    pub fn new(pos: ChunkPos, cells: Vec<Cell>) -> Self {
        Self::with_background(pos, cells, vec![Cell::AIR; CHUNK_AREA])
    }

    pub fn with_background(pos: ChunkPos, cells: Vec<Cell>, bg: Vec<Cell>) -> Self {
        assert_eq!(cells.len(), CHUNK_AREA, "a chunk has exactly CHUNK_AREA cells");
        assert_eq!(bg.len(), CHUNK_AREA, "a chunk's background has exactly CHUNK_AREA cells");
        Chunk {
            pos,
            cells: cells.into_boxed_slice(),
            bg: bg.into_boxed_slice(),
            // Fresh chunks settle once: generated sand over a cave should fall.
            next_dirty: AtomicRect::new(Rect::FULL),
            render_dirty: AtomicBool::new(true),
            modified: AtomicBool::new(false),
        }
    }

    pub fn filled(pos: ChunkPos, cell: Cell) -> Self {
        Self::new(pos, vec![cell; CHUNK_AREA])
    }

    #[inline]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    #[inline]
    pub fn get(&self, lx: usize, ly: usize) -> Cell {
        self.cells[local_index(lx, ly)]
    }

    #[inline]
    pub fn background(&self) -> &[Cell] {
        &self.bg
    }

    #[inline]
    pub fn get_bg(&self, lx: usize, ly: usize) -> Cell {
        self.bg[local_index(lx, ly)]
    }

    pub fn set_bg(&mut self, lx: usize, ly: usize, cell: Cell) {
        self.bg[local_index(lx, ly)] = cell;
        let (x, y) = (lx as i32, ly as i32);
        self.next_dirty.include(Rect { min_x: x - 1, min_y: y - 1, max_x: x + 1, max_y: y + 1 }.clamp_to_chunk());
        *self.render_dirty.get_mut() = true;
        *self.modified.get_mut() = true;
    }

    /// Direct write; wakes the cell and its neighbours inside this chunk.
    /// (`World::set_cell` also wakes across chunk borders.)
    #[inline]
    pub fn set(&mut self, lx: usize, ly: usize, cell: Cell) {
        self.cells[local_index(lx, ly)] = cell;
        let (x, y) = (lx as i32, ly as i32);
        self.next_dirty.include(Rect { min_x: x - 1, min_y: y - 1, max_x: x + 1, max_y: y + 1 }.clamp_to_chunk());
        *self.render_dirty.get_mut() = true;
        *self.modified.get_mut() = true;
    }

    pub fn wake(&self, r: Rect) {
        self.next_dirty.include(r.clamp_to_chunk());
    }

    /// Is anything scheduled to update next tick?
    pub fn is_awake(&self) -> bool {
        !self.next_dirty.peek().is_empty()
    }

    pub fn dirty_rect(&self) -> Rect {
        self.next_dirty.peek()
    }

    /// Returns true once after cells changed; the renderer calls this.
    pub fn take_render_dirty(&self) -> bool {
        self.render_dirty.swap(false, Relaxed)
    }

    pub fn is_modified(&self) -> bool {
        self.modified.load(Relaxed)
    }

    pub fn mark_modified(&self) {
        self.modified.store(true, Relaxed);
    }
}
