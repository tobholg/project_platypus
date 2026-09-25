//! Chunk serialisation. The same bytes serve the save file, the in-memory
//! store for unloaded chunks, and co-op resync of a diverged chunk.

use rustc_hash::FxHashMap;

use crate::cell::Cell;
use crate::chunk::Chunk;
use crate::coords::{CHUNK_AREA, ChunkPos};

pub fn encode(chunk: &Chunk) -> Vec<u8> {
    lz4_flex::compress_prepend_size(bytemuck::cast_slice(chunk.cells()))
}

pub fn decode(pos: ChunkPos, bytes: &[u8]) -> Result<Chunk, String> {
    let raw = lz4_flex::decompress_size_prepended(bytes).map_err(|e| e.to_string())?;
    if raw.len() != CHUNK_AREA * size_of::<Cell>() {
        return Err(format!("chunk {pos:?}: {} bytes, expected {}", raw.len(), CHUNK_AREA * size_of::<Cell>()));
    }
    let cells: Vec<Cell> = bytemuck::pod_collect_to_vec(&raw);
    let chunk = Chunk::new(pos, cells);
    chunk.mark_modified();
    Ok(chunk)
}

/// Stable 64-bit FNV-1a over the cell bytes. Used to detect co-op divergence.
pub fn checksum(chunk: &Chunk) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &b in bytemuck::cast_slice::<Cell, u8>(chunk.cells()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// Modified chunks that are not currently loaded.
#[derive(Default)]
pub struct ChunkStore {
    chunks: FxHashMap<ChunkPos, Vec<u8>>,
}

impl ChunkStore {
    pub fn put(&mut self, chunk: &Chunk) {
        self.chunks.insert(chunk.pos, encode(chunk));
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
        self.chunks.values().map(Vec::len).sum()
    }
}
