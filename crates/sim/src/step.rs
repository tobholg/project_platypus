//! Parallel, deterministic stepping.
//!
//! Each tick runs four checkerboard passes. In one pass, every awake chunk of
//! that parity is updated by its own job, concurrently. A job reads and writes
//! only cells within `MAX_REACH` (half a chunk) of its chunk, and same-parity
//! chunks are two chunks apart, so no two jobs of a pass ever touch the same
//! cell. That is the invariant everything unsafe in this file relies on; debug
//! builds assert it on every access.

use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

use rayon::prelude::*;
use rustc_hash::FxHashMap;

use crate::cell::{Cell, flags};
use crate::chunk::{AtomicRect, Chunk};
use crate::climate::Climate;
use crate::coords::{CHUNK, CHUNK_BITS, CellPos, ChunkPos, MAX_REACH, Rect};
use crate::material::{ExplosionDef, MaterialTable};
use crate::particles::Particle;
use crate::rng::Rng;
use crate::rules;

/// Raw access to one chunk, handed to jobs.
#[derive(Clone, Copy)]
pub(crate) struct ChunkRaw {
    cells: *mut Cell,
    bg: *mut Cell,
    next_dirty: *const AtomicRect,
    render_dirty: *const AtomicBool,
    modified: *const AtomicBool,
}

// SAFETY: jobs only dereference these for disjoint cells (module docs) or
// through atomics, and the owning `World` is exclusively borrowed by `step`
// for as long as any job runs.
unsafe impl Send for ChunkRaw {}
unsafe impl Sync for ChunkRaw {}

impl ChunkRaw {
    pub(crate) fn of(chunk: &mut Chunk) -> Self {
        ChunkRaw {
            cells: chunk.cells.as_mut_ptr(),
            bg: chunk.bg.as_mut_ptr(),
            next_dirty: &chunk.next_dirty,
            render_dirty: &chunk.render_dirty,
            modified: &chunk.modified,
        }
    }
}

/// The 3×3 chunks around the one being updated, addressed in coordinates
/// local to the centre chunk (`-CHUNK..2*CHUNK`).
pub(crate) struct Hood<'a> {
    chunks: [Option<ChunkRaw>; 9],
    dirty: [Rect; 9],
    touched: u16,
    pub mats: &'a MaterialTable,
    pub rng: Rng,
    pub clock: u8,
    pub tick: u64,
    /// World cell of the centre chunk's (0,0).
    pub origin: CellPos,
    pub climate: Climate,
    /// Explosions requested by igniting cells; the world applies them next tick.
    pub explosions: Vec<(CellPos, ExplosionDef)>,
    /// Where solid cells were destroyed (burned, melted, dissolved); the world
    /// checks those spots for pieces left hanging.
    pub broken: Vec<CellPos>,
    /// Particles emitted this tick (embers).
    pub particles: Vec<Particle>,
    /// Background cells destroyed this tick (burned out).
    pub broken_bg: Vec<CellPos>,
    /// Columns (world x) where vapour faded into the air, feeding the clouds.
    pub vapour: Vec<i32>,
    /// Wind this tick: -1 (hard left) … 1 (hard right).
    pub wind: f32,
}

const LOCAL_MASK: i32 = CHUNK - 1;

impl<'a> Hood<'a> {
    #[inline(always)]
    fn slot(lx: i32, ly: i32) -> (usize, usize) {
        debug_assert!(
            (-MAX_REACH..CHUNK + MAX_REACH).contains(&lx) && (-MAX_REACH..CHUNK + MAX_REACH).contains(&ly),
            "cell access ({lx},{ly}) is outside the job's reach; same-pass jobs could collide"
        );
        let sx = ((lx + CHUNK) >> CHUNK_BITS) as usize;
        let sy = ((ly + CHUNK) >> CHUNK_BITS) as usize;
        let i = ((ly & LOCAL_MASK) << CHUNK_BITS | (lx & LOCAL_MASK)) as usize;
        (sy * 3 + sx, i)
    }

    /// `None` = chunk not loaded; callers treat it as solid.
    #[inline(always)]
    pub fn get(&self, lx: i32, ly: i32) -> Option<Cell> {
        let (s, i) = Self::slot(lx, ly);
        // SAFETY: see module docs.
        self.chunks[s].map(|c| unsafe { *c.cells.add(i) })
    }

    /// Write a cell and wake it and its 8 neighbours. Writes to unloaded chunks are dropped.
    #[inline(always)]
    pub fn set(&mut self, lx: i32, ly: i32, cell: Cell) {
        let (s, i) = Self::slot(lx, ly);
        let Some(c) = self.chunks[s] else { return };
        // SAFETY: see module docs.
        unsafe { *c.cells.add(i) = cell };
        self.touched |= 1 << s;
        self.wake(lx, ly);
    }

    /// Background cell; `None` = chunk not loaded.
    #[inline(always)]
    pub fn get_bg(&self, lx: i32, ly: i32) -> Option<Cell> {
        let (s, i) = Self::slot(lx, ly);
        // SAFETY: see module docs (the background follows the same reach rule).
        self.chunks[s].map(|c| unsafe { *c.bg.add(i) })
    }

    #[inline(always)]
    pub fn set_bg(&mut self, lx: i32, ly: i32, cell: Cell) {
        let (s, i) = Self::slot(lx, ly);
        let Some(c) = self.chunks[s] else { return };
        // SAFETY: see module docs.
        unsafe { *c.bg.add(i) = cell };
        self.touched |= 1 << s;
        self.wake(lx, ly);
    }

    pub fn note_broken_bg(&mut self, lx: i32, ly: i32) {
        self.broken_bg.push(self.origin.offset(lx, ly));
    }

    /// Vapour faded here: it rises to the clouds above.
    pub fn note_vapour(&mut self, lx: i32) {
        self.vapour.push(self.origin.x + lx);
    }

    /// Schedule the 3×3 around a cell for next tick without changing it.
    #[inline(always)]
    pub fn wake(&mut self, lx: i32, ly: i32) {
        if lx > 0 && lx < CHUNK - 1 && ly > 0 && ly < CHUNK - 1 {
            // Fast path: the whole neighbourhood is inside the centre chunk.
            self.dirty[4].union(Rect { min_x: lx - 1, min_y: ly - 1, max_x: lx + 1, max_y: ly + 1 });
            return;
        }
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (x, y) = (lx + dx, ly + dy);
                let sx = ((x + CHUNK) >> CHUNK_BITS) as usize;
                let sy = ((y + CHUNK) >> CHUNK_BITS) as usize;
                self.dirty[sy * 3 + sx].include(x & LOCAL_MASK, y & LOCAL_MASK);
            }
        }
    }

    /// Ambient °C at a local cell.
    #[inline(always)]
    pub fn ambient(&self, lx: i32, ly: i32) -> i32 {
        self.climate.ambient(self.origin.x + lx, self.origin.y + ly)
    }

    /// World-space centre of a local cell, for launching particles.
    pub fn centre(&self, lx: i32, ly: i32) -> [f32; 2] {
        [(self.origin.x + lx) as f32 + 0.5, (self.origin.y + ly) as f32 + 0.5]
    }

    pub fn emit(&mut self, p: Particle) {
        self.particles.push(p);
    }

    pub fn note_broken(&mut self, lx: i32, ly: i32) {
        self.broken.push(self.origin.offset(lx, ly));
    }

    pub fn request_explosion(&mut self, lx: i32, ly: i32, ex: ExplosionDef) {
        self.explosions.push((self.origin.offset(lx, ly), ex));
    }

    /// Publish accumulated dirty rects and flags to the chunks.
    fn finish(self) -> JobOutput {
        for s in 0..9 {
            let Some(c) = self.chunks[s] else { continue };
            // SAFETY: atomics; the chunk outlives the job.
            unsafe {
                (*c.next_dirty).include(self.dirty[s]);
                if self.touched & (1 << s) != 0 {
                    (*c.render_dirty).store(true, Relaxed);
                    (*c.modified).store(true, Relaxed);
                }
            }
        }
        JobOutput { explosions: self.explosions, broken: self.broken, broken_bg: self.broken_bg, particles: self.particles, vapour: self.vapour }
    }
}

#[derive(Clone, Debug, Default)]
pub struct StepStats {
    /// Chunks that had anything to update.
    pub active_chunks: usize,
    /// Cells inside dirty rects (visited, not necessarily moved).
    pub visited_cells: usize,
    /// Explosions requested this tick (burning explosive materials).
    pub explosions: Vec<(CellPos, ExplosionDef)>,
    /// Solid cells destroyed by the simulation this tick.
    pub broken: Vec<CellPos>,
    /// Particles the simulation emitted this tick (the world takes them).
    pub particles: Vec<Particle>,
    /// Background cells the simulation destroyed this tick.
    pub broken_bg: Vec<CellPos>,
    /// Explosions the world detonated at the start of this tick (centre,
    /// radius): for effects like screen shake and flashes.
    pub detonated: Vec<(CellPos, i32)>,
    /// Columns where vapour faded into the air this tick (the world feeds
    /// them to the clouds).
    pub vapour: Vec<i32>,
    /// Lightning that struck this tick (for the bolt, the flash, the thunder,
    /// and whoever stood there).
    pub lightning: Vec<Strike>,
    /// Time spent this tick in: edits and detonations, cells, broken support
    /// checks, particles, bodies, weather (see `PHASES`).
    pub phases: [std::time::Duration; 6],
}

/// Names of `StepStats::phases`.
pub const PHASES: [&str; 6] = ["edits", "cells", "broken", "particles", "bodies", "weather"];

/// A lightning strike: down column `x` from `top` to the first cell it hit,
/// then (through a tree) on down to where it `earth`ed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strike {
    pub x: i32,
    pub top: i32,
    pub hit: CellPos,
    pub earth: CellPos,
}

/// What one job reports back to the world.
#[derive(Default)]
struct JobOutput {
    explosions: Vec<(CellPos, ExplosionDef)>,
    broken: Vec<CellPos>,
    broken_bg: Vec<CellPos>,
    particles: Vec<Particle>,
    vapour: Vec<i32>,
}

pub(crate) fn step_chunks(
    chunks: &mut FxHashMap<ChunkPos, Box<Chunk>>,
    mats: &MaterialTable,
    seed: u64,
    tick: u64,
    climate: Climate,
    wind: f32,
) -> StepStats {
    let clock = tick as u8;
    let mut raws: FxHashMap<ChunkPos, ChunkRaw> = FxHashMap::default();
    raws.reserve(chunks.len());
    let mut passes: [Vec<(ChunkPos, Rect)>; 4] = Default::default();
    let mut stats = StepStats::default();

    for (pos, chunk) in chunks.iter_mut() {
        let rect = chunk.next_dirty.take().clamp_to_chunk();
        if !rect.is_empty() {
            stats.active_chunks += 1;
            stats.visited_cells += rect.area();
            passes[pos.parity()].push((*pos, rect));
        }
        raws.insert(*pos, ChunkRaw::of(chunk));
    }

    for jobs in &passes {
        let results: Vec<JobOutput> = jobs.par_iter().map(|&(pos, rect)| {
            let mut hood = Hood {
                chunks: std::array::from_fn(|s| {
                    let (dx, dy) = (s as i32 % 3 - 1, s as i32 / 3 - 1);
                    raws.get(&pos.offset(dx, dy)).copied()
                }),
                dirty: [Rect::EMPTY; 9],
                touched: 0,
                mats,
                rng: Rng::seeded(&[seed, tick, pos.x as u64, pos.y as u64]),
                clock,
                tick,
                origin: pos.origin(),
                climate,
                explosions: Vec::new(),
                broken: Vec::new(),
                particles: Vec::new(),
                broken_bg: Vec::new(),
                vapour: Vec::new(),
                wind,
            };
            update_rect(&mut hood, rect);
            hood.finish()
        }).collect();
        // `collect` keeps job order, so the lists are deterministic.
        for out in results {
            stats.explosions.extend(out.explosions);
            stats.broken.extend(out.broken);
            stats.particles.extend(out.particles);
            stats.broken_bg.extend(out.broken_bg);
            stats.vapour.extend(out.vapour);
        }
    }
    stats
}

fn update_rect(h: &mut Hood, r: Rect) {
    // Bottom-up so falling cells land in rows already visited; alternate the
    // horizontal direction per row and tick so nothing drifts one way.
    for y in r.min_y..=r.max_y {
        let left_to_right = (y as u64 + h.tick) & 1 == 0;
        for i in 0..=(r.max_x - r.min_x) {
            let x = if left_to_right { r.min_x + i } else { r.max_x - i };
            let b = h.get_bg(x, y).expect("centre chunk is loaded");
            if b.flags & flags::BURNING != 0 || b.heat != 0 {
                rules::background(h, x, y, b);
            }
            let c = h.get(x, y).expect("centre chunk is loaded");
            if !h.mats.phys(c.material).active && c.flags & (flags::LOOSE | flags::BURNING) == 0 && c.heat == 0 {
                continue;
            }
            if c.clock == h.clock {
                // Moved here earlier this tick, or a stale clock from 256·k
                // ticks ago (the byte wraps). Either way: look again next tick,
                // or a still-busy cell could put its region to sleep.
                h.wake(x, y);
                continue;
            }
            rules::update_cell(h, x, y, c);
        }
    }
}
