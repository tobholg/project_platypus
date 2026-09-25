use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::cell::{Cell, flags};
use crate::chunk::Chunk;
use crate::climate::Climate;
use crate::coords::{CHUNK, CellPos, ChunkPos, Rect};
use crate::edit::{EditReport, WorldEdit, disc};
use crate::material::{ExplosionDef, Kind, MatPhys, MaterialId, MaterialTable};
use crate::rng::Rng;
use crate::step::{StepStats, step_chunks};
use crate::store;

/// A connected solid piece bigger than this counts as ground (anchored). Smaller
/// pieces that touch neither bedrock nor unloaded world are floating and fall.
/// (With rigid bodies, PLAN W4, big floating pieces will fall as bodies.)
const ANCHOR_BUDGET: usize = 3_000;
/// Fragment checks run per tick for cells the simulation destroyed (a forest
/// fire can break thousands); the rest wait for the next tick.
const MAX_FRAGMENT_CHECKS_PER_TICK: usize = 24;
/// Destroyed cells are grouped into tiles of this size, one check per tile.
const FRAGMENT_TILE_BITS: i32 = 4;
/// Most explosions applied per tick; nearby requests merge into one.
const MAX_EXPLOSIONS_PER_TICK: usize = 6;
/// Heat (°C) a blast leaves in the rock at its centre, fading to its rim.
const BLAST_HEAT: f32 = 700.0;

/// The loaded part of the cell world. Unloaded chunks behave as solid walls.
pub struct World {
    chunks: FxHashMap<ChunkPos, Box<Chunk>>,
    materials: Arc<MaterialTable>,
    seed: u64,
    tick: u64,
    edits: Vec<WorldEdit>,
    climate: Climate,
    /// Explosions requested by burning explosives, applied next tick.
    pending_explosions: Vec<(CellPos, ExplosionDef)>,
    /// Tiles where the simulation destroyed solids, awaiting a fragment check.
    pending_fragment_tiles: Vec<CellPos>,
}

impl World {
    pub fn new(seed: u64, materials: Arc<MaterialTable>) -> Self {
        World {
            chunks: FxHashMap::default(),
            materials,
            seed,
            tick: 0,
            edits: Vec::new(),
            climate: Climate::default(),
            pending_explosions: Vec::new(),
            pending_fragment_tiles: Vec::new(),
        }
    }

    pub fn climate(&self) -> Climate {
        self.climate
    }

    pub fn set_climate(&mut self, climate: Climate) {
        self.climate = climate;
    }

    /// Absolute temperature (°C) of a cell.
    pub fn temperature(&self, p: CellPos) -> Option<i32> {
        self.get(p).map(|c| self.climate.ambient(p.y) + c.heat as i32)
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Number of ticks stepped so far.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn materials(&self) -> &Arc<MaterialTable> {
        &self.materials
    }

    /// Swap in reloaded materials. Ids must be stable (`MaterialTable::reload_from_ron`).
    pub fn set_materials(&mut self, materials: Arc<MaterialTable>) {
        self.materials = materials;
        for c in self.chunks.values() {
            c.wake(Rect::FULL);
            c.render_dirty.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    // ---- chunks -----------------------------------------------------------

    pub fn insert_chunk(&mut self, chunk: Chunk) {
        let pos = chunk.pos;
        // Neighbours just lost a wall on that side; let their edge cells re-check.
        for dy in -1..=1 {
            for dx in -1..=1 {
                if let Some(n) = self.chunks.get(&pos.offset(dx, dy)) {
                    n.wake(edge_facing(-dx, -dy));
                }
            }
        }
        self.chunks.insert(pos, Box::new(chunk));
    }

    pub fn remove_chunk(&mut self, pos: ChunkPos) -> Option<Box<Chunk>> {
        self.chunks.remove(&pos)
    }

    pub fn chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&pos).map(|b| &**b)
    }

    pub fn is_loaded(&self, pos: ChunkPos) -> bool {
        self.chunks.contains_key(&pos)
    }

    pub fn chunks(&self) -> impl Iterator<Item = &Chunk> {
        self.chunks.values().map(|b| &**b)
    }

    pub fn loaded_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn checksum(&self, pos: ChunkPos) -> Option<u64> {
        self.chunk(pos).map(store::checksum)
    }

    // ---- cells ------------------------------------------------------------

    pub fn get(&self, p: CellPos) -> Option<Cell> {
        let (lx, ly) = p.local();
        self.chunks.get(&p.chunk()).map(|c| c.get(lx, ly))
    }

    /// Is the cell something a body collides with? Unloaded counts as solid.
    pub fn is_solid(&self, p: CellPos) -> bool {
        match self.get(p) {
            None => true,
            Some(c) => matches!(self.materials.phys(c.material).kind, Kind::Static | Kind::Powder),
        }
    }

    /// Write one cell, waking its neighbourhood across chunk borders.
    /// Returns false if the chunk is not loaded.
    pub fn set(&mut self, p: CellPos, cell: Cell) -> bool {
        let Some(chunk) = self.chunks.get_mut(&p.chunk()) else { return false };
        let (lx, ly) = p.local();
        chunk.set(lx, ly, cell);
        let (lx, ly) = (lx as i32, ly as i32);
        if lx == 0 || ly == 0 || lx == CHUNK - 1 || ly == CHUNK - 1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let q = p.offset(dx, dy);
                    if let Some(n) = self.chunks.get(&q.chunk()) {
                        let (qx, qy) = q.local();
                        n.wake(Rect { min_x: qx as i32, min_y: qy as i32, max_x: qx as i32, max_y: qy as i32 });
                    }
                }
            }
        }
        true
    }

    // ---- edits ------------------------------------------------------------

    /// Queue an edit for the start of the next tick.
    pub fn queue_edit(&mut self, edit: WorldEdit) {
        self.edits.push(edit);
    }

    /// Apply an edit now. Randomness (shade, life) is seeded by tick and position.
    pub fn apply_edit(&mut self, edit: &WorldEdit) -> EditReport {
        let mut report = EditReport::default();
        match *edit {
            WorldEdit::Paint { center, radius, material, overwrite } => self.paint(center, radius, material, overwrite, &mut report),
            WorldEdit::Ignite { center, radius } => {
                self.ignite(center, radius, &mut report);
                if report.placed > 0 {
                    self.loosen_fragments(center, radius);
                }
            }
            WorldEdit::Heat { center, radius, amount } => self.heat(center, radius, amount),
            WorldEdit::Dig { center, radius, max_hardness } => {
                self.dig(center, radius, max_hardness, &mut report);
                self.loosen_if_removed(center, radius, &report);
            }
            WorldEdit::Mine { center, radius, power, max_hardness } => {
                self.mine(center, radius, power, max_hardness, &mut report);
                self.loosen_if_removed(center, radius, &report);
            }
            WorldEdit::Explode { center, radius, power } => {
                self.explode(center, radius, power, &mut report);
                self.loosen_if_removed(center, radius + 3, &report);
            }
        }
        report
    }

    fn rng_for(&self, salt: u64, center: CellPos) -> Rng {
        Rng::seeded(&[self.seed, self.tick, salt, center.x as u64, center.y as u64])
    }

    fn paint(&mut self, center: CellPos, radius: i32, material: MaterialId, overwrite: bool, report: &mut EditReport) {
        let mats = self.materials.clone();
        let mut rng = self.rng_for(0x9A17, center);
        for p in disc(center, radius) {
            let Some(old) = self.get(p) else { continue };
            if old.material == material || (!overwrite && !old.is_air()) {
                continue;
            }
            if !old.is_air() {
                report.add_removed(old.material);
            }
            let cell = mats.spawn(material, &mut rng);
            self.set(p, cell);
            report.placed += 1;
        }
    }

    fn dig(&mut self, center: CellPos, radius: i32, max_hardness: u8, report: &mut EditReport) {
        let mats = self.materials.clone();
        for p in disc(center, radius) {
            let Some(old) = self.get(p) else { continue };
            if old.is_air() || mats.phys(old.material).hardness > max_hardness {
                continue;
            }
            report.add_removed(old.material);
            self.set(p, Cell::AIR);
        }
    }

    fn mine(&mut self, center: CellPos, radius: i32, power: u8, max_hardness: u8, report: &mut EditReport) {
        let mats = self.materials.clone();
        let mut rng = self.rng_for(0x3113, center);
        for p in disc(center, radius) {
            let Some(mut c) = self.get(p) else { continue };
            let ph = mats.phys(c.material);
            if c.is_air() || !matches!(ph.kind, Kind::Static | Kind::Powder) || ph.hardness > max_hardness || ph.hardness == u8::MAX {
                continue;
            }
            // Centre digs faster than the rim, so holes come out round.
            let d = distance(center, p) / (radius as f32 + 0.5);
            let dmg = stochastic_round(power as f32 * (1.0 - 0.6 * d), &mut rng);
            let total = c.life as u32 + dmg;
            if total >= ph.hardness.max(1) as u32 {
                report.add_removed(c.material);
                self.set(p, Cell::AIR);
            } else if dmg > 0 {
                c.life = total as u8;
                self.set(p, c);
            }
        }
    }

    fn explode(&mut self, center: CellPos, radius: i32, power: u8, report: &mut EditReport) {
        let mats = self.materials.clone();
        let mut rng = self.rng_for(0xB00B, center);
        let (fire, smoke) = (mats.id("fire"), mats.id("smoke"));
        let r = radius.max(1) as f32;
        let breakable = |h: u8| h < u8::MAX && h <= power;
        for p in disc(center, radius + 3) {
            let Some(c) = self.get(p) else { continue };
            let ph = *mats.phys(c.material);
            let d = distance(center, p);
            if d <= r {
                let force = power as f32 * (1.0 - 0.5 * (d / r).powi(2));
                let mut now_air = c.is_air();
                if !c.is_air() {
                    if breakable(ph.hardness) && ph.hardness as f32 <= force {
                        report.add_removed(c.material);
                        self.set(p, Cell::AIR);
                        now_air = true;
                    } else if breakable(ph.hardness) && ph.crumbles_into != MaterialId::AIR {
                        let rubble = mats.spawn(ph.crumbles_into, &mut rng);
                        self.set(p, rubble);
                    } else if ph.flammability > 0 {
                        self.ignite_cell(p, &ph, &mut rng);
                    } else {
                        self.add_heat(p, (BLAST_HEAT * (1.0 - d / (r + 3.0))) as i16);
                    }
                }
                if now_air && d < r * 0.85 {
                    let roll = rng.next_u8();
                    let fill = if roll < 70 { fire } else if roll < 130 { smoke } else { None };
                    if let Some(m) = fill {
                        let cell = mats.spawn(m, &mut rng);
                        self.set(p, cell);
                    }
                }
            } else if !c.is_air() {
                // The rim: shattered, not destroyed.
                if breakable(ph.hardness) && ph.crumbles_into != MaterialId::AIR && rng.chance(150) {
                    let rubble = mats.spawn(ph.crumbles_into, &mut rng);
                    self.set(p, rubble);
                } else if ph.flammability > 0 && rng.chance(ph.flammability.saturating_mul(4)) {
                    self.ignite_cell(p, &ph, &mut rng);
                } else {
                    self.add_heat(p, (BLAST_HEAT * 0.4 * (1.0 - (d - r) / 3.0).max(0.0)) as i16);
                }
            }
        }
    }

    /// Edit-side ignition, same rules as the stepper's: gases flash into
    /// flame, everything else starts burning in place; explosives may go off.
    fn ignite_cell(&mut self, p: CellPos, ph: &MatPhys, rng: &mut Rng) {
        if let Some(ex) = ph.explodes
            && rng.chance(ex.chance)
        {
            self.pending_explosions.push((p, ex));
        }
        let Some(mut c) = self.get(p) else { return };
        if matches!(ph.kind, Kind::Gas | Kind::Fire) {
            let flame = self.materials.spawn(self.materials.fire(), rng);
            self.set(p, flame);
        } else if c.flags & flags::BURNING == 0 {
            c.flags |= flags::BURNING;
            c.life = ph.burn_time;
            c.heat = c.heat.max(650);
            self.set(p, c);
        }
    }

    fn add_heat(&mut self, p: CellPos, amount: i16) {
        if amount == 0 {
            return;
        }
        if let Some(mut c) = self.get(p)
            && !c.is_air()
        {
            c.heat = c.heat.saturating_add(amount).clamp(-300, 4000);
            self.set(p, c);
        }
    }

    fn heat(&mut self, center: CellPos, radius: i32, amount: i16) {
        let r = radius as f32 + 0.5;
        for p in disc(center, radius) {
            let falloff = 1.0 - 0.5 * distance(center, p) / r;
            self.add_heat(p, (amount as f32 * falloff) as i16);
        }
    }

    fn ignite(&mut self, center: CellPos, radius: i32, report: &mut EditReport) {
        let mats = self.materials.clone();
        let mut rng = self.rng_for(0xF14E, center);
        let Some(fire) = mats.id("fire") else { return };
        for p in disc(center, radius) {
            let Some(c) = self.get(p) else { continue };
            let ph = mats.phys(c.material);
            if c.is_air() {
                if rng.chance(60) {
                    let flame = mats.spawn(fire, &mut rng);
                    self.set(p, flame);
                    report.placed += 1;
                }
            } else if ph.flammability > 0 {
                let ph = *ph;
                self.ignite_cell(p, &ph, &mut rng);
                report.placed += 1;
            }
        }
    }

    fn loosen_if_removed(&mut self, center: CellPos, reach: i32, report: &EditReport) {
        if !report.removed.is_empty() {
            self.loosen_fragments(center, reach);
        }
    }

    /// After destruction around `center`: every solid (`Static`) piece near it
    /// that no longer rests on ground becomes loose and falls as rubble of its
    /// own material. Returns cells loosened.
    ///
    /// "Ground" is anything connected (sharing an edge) to bedrock, to the
    /// unloaded world, or to more than `ANCHOR_BUDGET` cells of solid. A piece
    /// hanging by a diagonal corner is not attached.
    pub fn loosen_fragments(&mut self, center: CellPos, reach: i32) -> usize {
        self.loosen_around(center, reach, &mut FxHashSet::default())
    }

    /// `anchored` is shared across calls in one tick, so ground explored by
    /// one check is known to the next.
    fn loosen_around(&mut self, center: CellPos, reach: i32, anchored: &mut FxHashSet<CellPos>) -> usize {
        let mats = self.materials.clone();
        enum Probe {
            Solid,
            Anchor,
            Open,
        }
        let probe = |w: &World, p: CellPos| match w.get(p) {
            None => Probe::Anchor, // world edge / unloaded
            Some(c) => {
                let ph = mats.phys(c.material);
                if ph.kind != Kind::Static || c.flags & flags::LOOSE != 0 {
                    Probe::Open
                } else if ph.hardness == u8::MAX {
                    Probe::Anchor // bedrock
                } else {
                    Probe::Solid
                }
            }
        };

        let mut loosened = 0;
        let mut piece: Vec<CellPos> = Vec::new();
        let mut seen: FxHashSet<CellPos> = FxHashSet::default();
        let mut stack: Vec<CellPos> = Vec::new();
        // Start from the damaged area and its rim: what was attached through it.
        let reach = reach + 2;
        let mut seeds: Vec<CellPos> =
            (-reach..=reach).rev().flat_map(|dy| (-reach..=reach).rev().map(move |dx| center.offset(dx, dy))).collect();
        while let Some(start) = seeds.pop() {
            {
                if anchored.contains(&start) || seen.contains(&start) || !matches!(probe(self, start), Probe::Solid) {
                    continue;
                }
                piece.clear();
                stack.clear();
                stack.push(start);
                seen.insert(start);
                let mut grounded = false;
                while let Some(p) = stack.pop() {
                    piece.push(p);
                    if piece.len() > ANCHOR_BUDGET {
                        grounded = true;
                        break;
                    }
                    for (ox, oy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let q = p.offset(ox, oy);
                        if seen.contains(&q) {
                            continue;
                        }
                        if anchored.contains(&q) {
                            grounded = true;
                            break;
                        }
                        match probe(self, q) {
                            Probe::Anchor => grounded = true,
                            Probe::Solid => {
                                seen.insert(q);
                                stack.push(q);
                            }
                            Probe::Open => {}
                        }
                    }
                    if grounded {
                        break;
                    }
                }
                if grounded {
                    anchored.extend(piece.iter().copied());
                    anchored.extend(stack.iter().copied());
                    continue;
                }
                for &p in &piece {
                    if let Some(mut c) = self.get(p) {
                        c.flags |= flags::LOOSE;
                        self.set(p, c);
                        loosened += 1;
                    }
                    // Whatever this piece held up, even by a corner, may be next.
                    for (ox, oy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                        seeds.push(p.offset(ox, oy));
                    }
                }
            }
        }
        loosened
    }

    // ---- time -------------------------------------------------------------

    /// Apply queued edits and pending explosions, then advance one tick.
    pub fn step(&mut self) -> StepStats {
        for edit in std::mem::take(&mut self.edits) {
            self.apply_edit(&edit);
        }
        self.detonate_pending();
        self.tick += 1;
        let stats = step_chunks(&mut self.chunks, &self.materials, self.seed, self.tick, self.climate);
        self.pending_explosions.extend(stats.explosions.iter().copied());
        self.check_broken(&stats.broken);
        stats
    }

    /// Fire, melting and acid destroy solids inside the step; afterwards,
    /// loosen whatever those losses left hanging (same rule as for edits).
    fn check_broken(&mut self, broken: &[CellPos]) {
        let tiles = broken.iter().map(|p| CellPos::new(p.x >> FRAGMENT_TILE_BITS, p.y >> FRAGMENT_TILE_BITS));
        self.pending_fragment_tiles.extend(tiles);
        if self.pending_fragment_tiles.is_empty() {
            return;
        }
        self.pending_fragment_tiles.sort();
        self.pending_fragment_tiles.dedup();
        let n = self.pending_fragment_tiles.len().min(MAX_FRAGMENT_CHECKS_PER_TICK);
        let now: Vec<CellPos> = self.pending_fragment_tiles.drain(..n).collect();
        let half = 1 << (FRAGMENT_TILE_BITS - 1);
        let mut anchored = FxHashSet::default();
        for t in now {
            let center = CellPos::new((t.x << FRAGMENT_TILE_BITS) + half, (t.y << FRAGMENT_TILE_BITS) + half);
            self.loosen_around(center, half + 2, &mut anchored);
        }
    }

    /// Merge nearby requests (a burning gas pocket asks for hundreds) and apply
    /// at most a few per tick; the rest are covered by the blasts that happen.
    fn detonate_pending(&mut self) {
        if self.pending_explosions.is_empty() {
            return;
        }
        let mut requests = std::mem::take(&mut self.pending_explosions);
        requests.sort_by_key(|(p, _)| (p.y, p.x));
        let mut chosen: Vec<(CellPos, ExplosionDef)> = Vec::new();
        for (p, ex) in requests {
            if chosen.len() >= MAX_EXPLOSIONS_PER_TICK {
                break;
            }
            if chosen.iter().all(|(q, qe)| distance(p, *q) > (ex.radius.max(qe.radius) as f32) * 1.2) {
                chosen.push((p, ex));
            }
        }
        for (center, ex) in chosen {
            self.apply_edit(&WorldEdit::Explode { center, radius: ex.radius, power: ex.power });
        }
    }
}

fn distance(a: CellPos, b: CellPos) -> f32 {
    (((a.x - b.x).pow(2) + (a.y - b.y).pow(2)) as f32).sqrt()
}

/// 2.3 → 2 or 3 with 30% chance of 3, so small per-tick amounts still add up.
fn stochastic_round(v: f32, rng: &mut Rng) -> u32 {
    let v = v.max(0.0);
    let base = v.floor();
    base as u32 + ((rng.next_u32() as f32 / u32::MAX as f32) < v - base) as u32
}

/// The two-cell strip of a chunk that faces the neighbour at (dx, dy).
fn edge_facing(dx: i32, dy: i32) -> Rect {
    let span = |d: i32| match d {
        -1 => (0, 1),
        1 => (CHUNK - 2, CHUNK - 1),
        _ => (0, CHUNK - 1),
    };
    let ((min_x, max_x), (min_y, max_y)) = (span(dx), span(dy));
    Rect { min_x, min_y, max_x, max_y }
}
