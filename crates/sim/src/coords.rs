//! The only place world positions, cells and chunks are converted.
//!
//! 1 world unit = 1 cell, y points up, a position maps to its cell by `floor`.

pub const CHUNK_BITS: i32 = 6;
/// Chunk edge length in cells.
pub const CHUNK: i32 = 1 << CHUNK_BITS;
pub const CHUNK_AREA: usize = (CHUNK * CHUNK) as usize;
const MASK: i32 = CHUNK - 1;

/// How far (in cells) an update may reach outside the chunk being updated.
/// Half a chunk keeps same-parity chunks in a checkerboard pass disjoint.
pub const MAX_REACH: i32 = CHUNK / 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellPos {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
}

impl CellPos {
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// The cell containing a world-space point.
    #[inline]
    pub fn from_world(x: f32, y: f32) -> Self {
        Self::new(x.floor() as i32, y.floor() as i32)
    }

    #[inline]
    pub const fn chunk(self) -> ChunkPos {
        // Arithmetic shift is floor division, also for negatives.
        ChunkPos::new(self.x >> CHUNK_BITS, self.y >> CHUNK_BITS)
    }

    /// Position inside its chunk, `0..CHUNK` on both axes.
    #[inline]
    pub const fn local(self) -> (usize, usize) {
        ((self.x & MASK) as usize, (self.y & MASK) as usize)
    }

    #[inline]
    pub const fn offset(self, dx: i32, dy: i32) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }
}

impl ChunkPos {
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// World cell of this chunk's bottom-left corner.
    #[inline]
    pub const fn origin(self) -> CellPos {
        CellPos::new(self.x << CHUNK_BITS, self.y << CHUNK_BITS)
    }

    #[inline]
    pub const fn offset(self, dx: i32, dy: i32) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }

    /// Checkerboard pass (0..4) this chunk belongs to.
    #[inline]
    pub const fn parity(self) -> usize {
        ((self.x & 1) | ((self.y & 1) << 1)) as usize
    }
}

/// Row-major index into a chunk's cell array (row 0 = bottom).
#[inline]
pub const fn local_index(lx: usize, ly: usize) -> usize {
    ly * CHUNK as usize + lx
}

/// Inclusive rectangle in chunk-local cell coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl Rect {
    pub const EMPTY: Rect = Rect { min_x: i32::MAX, min_y: i32::MAX, max_x: i32::MIN, max_y: i32::MIN };
    pub const FULL: Rect = Rect { min_x: 0, min_y: 0, max_x: CHUNK - 1, max_y: CHUNK - 1 };

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.min_x > self.max_x || self.min_y > self.max_y
    }

    #[inline]
    pub fn include(&mut self, x: i32, y: i32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    #[inline]
    pub fn union(&mut self, o: Rect) {
        self.min_x = self.min_x.min(o.min_x);
        self.min_y = self.min_y.min(o.min_y);
        self.max_x = self.max_x.max(o.max_x);
        self.max_y = self.max_y.max(o.max_y);
    }

    /// Clamp to the chunk; empty stays empty.
    #[inline]
    pub fn clamp_to_chunk(self) -> Rect {
        if self.is_empty() {
            return Rect::EMPTY;
        }
        let r = Rect {
            min_x: self.min_x.max(0),
            min_y: self.min_y.max(0),
            max_x: self.max_x.min(CHUNK - 1),
            max_y: self.max_y.min(CHUNK - 1),
        };
        if r.is_empty() { Rect::EMPTY } else { r }
    }

    pub fn area(&self) -> usize {
        if self.is_empty() { 0 } else { ((self.max_x - self.min_x + 1) * (self.max_y - self.min_y + 1)) as usize }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_cells_floor_into_chunks() {
        assert_eq!(CellPos::new(-1, -1).chunk(), ChunkPos::new(-1, -1));
        assert_eq!(CellPos::new(-1, -1).local(), (63, 63));
        assert_eq!(CellPos::new(-64, 0).chunk(), ChunkPos::new(-1, 0));
        assert_eq!(CellPos::new(64, 63).chunk(), ChunkPos::new(1, 0));
        assert_eq!(CellPos::from_world(-0.5, 3.99), CellPos::new(-1, 3));
    }

    #[test]
    fn origin_roundtrips() {
        for p in [ChunkPos::new(-3, 7), ChunkPos::new(0, 0), ChunkPos::new(5, -2)] {
            assert_eq!(p.origin().chunk(), p);
            assert_eq!(p.origin().local(), (0, 0));
        }
    }

    #[test]
    fn same_parity_chunks_are_two_apart() {
        let a = ChunkPos::new(2, 4);
        assert_eq!(a.parity(), a.offset(2, 0).parity());
        assert_ne!(a.parity(), a.offset(1, 0).parity());
        assert_ne!(a.parity(), a.offset(0, 1).parity());
    }
}
