//! Chunk serialisation. The same bytes serve the save file, the in-memory
//! store for unloaded chunks, and co-op resync of a diverged chunk.

use rustc_hash::FxHashMap;

use crate::cell::Cell;
use crate::material::MaterialId;
use crate::chunk::Chunk;
use crate::coords::{CHUNK_AREA, ChunkPos};

/// Playfield cells, then background cells, lz4-compressed.
pub fn encode(chunk: &Chunk) -> Vec<u8> {
    let mut raw = Vec::with_capacity(2 * CHUNK_AREA * size_of::<Cell>());
    raw.extend_from_slice(bytemuck::cast_slice(chunk.cells()));
    raw.extend_from_slice(bytemuck::cast_slice(chunk.background()));
    lz4_flex::compress_prepend_size(&raw)
}

pub fn decode(pos: ChunkPos, bytes: &[u8]) -> Result<Chunk, String> {
    decode_remapped(pos, bytes, None)
}

/// As `decode`, with each cell's material id turned into another (`map`,
/// indexed by the old id; a save made with materials in another order).
pub fn decode_remapped(pos: ChunkPos, bytes: &[u8], map: Option<&[MaterialId]>) -> Result<Chunk, String> {
    let raw = lz4_flex::decompress_size_prepended(bytes).map_err(|e| e.to_string())?;
    let layer = CHUNK_AREA * size_of::<Cell>();
    if raw.len() != 2 * layer {
        return Err(format!("chunk {pos:?}: {} bytes, expected {}", raw.len(), 2 * layer));
    }
    let mut cells: Vec<Cell> = bytemuck::pod_collect_to_vec(&raw[..layer]);
    let mut bg: Vec<Cell> = bytemuck::pod_collect_to_vec(&raw[layer..]);
    if let Some(map) = map {
        for c in cells.iter_mut().chain(bg.iter_mut()) {
            c.material = map.get(c.material.0 as usize).copied().unwrap_or(MaterialId::AIR);
        }
    }
    let chunk = Chunk::with_background(pos, cells, bg);
    chunk.mark_modified();
    Ok(chunk)
}

/// Stable 64-bit FNV-1a over both layers. Used to detect co-op divergence.
pub fn checksum(chunk: &Chunk) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for layer in [chunk.cells(), chunk.background()] {
        for &b in bytemuck::cast_slice::<Cell, u8>(layer) {
            h ^= b as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
    }
    h
}

/// Modified chunks that are not currently loaded. (Each shared, so a save
/// takes them without copying: thousands of them.)
#[derive(Default)]
pub struct ChunkStore {
    chunks: FxHashMap<ChunkPos, std::sync::Arc<[u8]>>,
}

impl ChunkStore {
    pub fn put(&mut self, chunk: &Chunk) {
        self.chunks.insert(chunk.pos, encode(chunk).into());
    }

    pub fn take(&mut self, pos: ChunkPos) -> Option<Chunk> {
        let bytes = self.chunks.remove(&pos)?;
        Some(decode(pos, &bytes).expect("chunk store holds only chunks it encoded"))
    }

    pub fn contains(&self, pos: ChunkPos) -> bool {
        self.chunks.contains_key(&pos)
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    pub fn bytes(&self) -> usize {
        self.chunks.values().map(|b| b.len()).sum()
    }

    /// Every chunk kept, encoded (a save writes these as they are).
    pub fn iter(&self) -> impl Iterator<Item = (ChunkPos, &std::sync::Arc<[u8]>)> {
        self.chunks.iter().map(|(p, b)| (*p, b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A chunk through the bytes and back is the same chunk; with a map,
    /// its materials are renumbered (a save read with materials reordered).
    #[test]
    fn a_chunk_round_trips_and_remaps() {
        let pos = ChunkPos::new(3, -2);
        let mut cells = vec![Cell::AIR; CHUNK_AREA];
        cells[5] = Cell::new(MaterialId(2), 7);
        cells[6] = Cell::new(MaterialId(3), 9);
        let chunk = Chunk::with_background(pos, cells, vec![Cell::new(MaterialId(2), 1); CHUNK_AREA]);
        let bytes = encode(&chunk);
        let back = decode(pos, &bytes).unwrap();
        assert_eq!(checksum(&back), checksum(&chunk));
        // Old 2 is now 4, old 3 is gone.
        let map = [MaterialId(0), MaterialId(1), MaterialId(4)];
        let moved = decode_remapped(pos, &bytes, Some(&map)).unwrap();
        assert_eq!(moved.cells()[5].material, MaterialId(4));
        assert_eq!(moved.cells()[5].shade, 7);
        assert_eq!(moved.cells()[6].material, MaterialId::AIR);
        assert!(moved.background().iter().all(|c| c.material == MaterialId(4)));
    }
}
