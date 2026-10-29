//! Sky islands (ported from the legacy generator): floating lumps of rock in
//! the sky band with a grassy top, a ragged underside and caves inside. Each
//! is rasterised once at plan time into a small stamp that chunks read, so a
//! sequential carve (the cave walkers) stays chunk-pure.

use noise::{NoiseFn, Perlin};
use platypus_sim::rng::Rng;

/// What an island stamp holds at a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum IslandCell {
    None = 0,
    Grass,
    Dirt,
    Stone,
    /// Inside the island but carved out: air in front, rock behind.
    Cave,
    /// What keeps it up: aerolite, veined through its underside.
    Aerolite,
    /// An old lightning strike's scar: fulgurite, down from its top.
    Fulgurite,
    /// A big island's lake, on its top.
    Water,
    /// Where its waterfall springs from, in its side under the lake.
    Spring,
}

pub struct Island {
    /// Bottom-left of the stamp (world cells).
    pub x0: i32,
    pub y0: i32,
    pub w: i32,
    pub h: i32,
    cells: Vec<IslandCell>,
    /// First air above the grass per stamp column (world y), for trees.
    top: Vec<Option<i32>>,
}

impl Island {
    pub fn at(&self, x: i32, y: i32) -> IslandCell {
        let (lx, ly) = (x - self.x0, y - self.y0);
        if lx < 0 || ly < 0 || lx >= self.w || ly >= self.h {
            return IslandCell::None;
        }
        self.cells[(ly * self.w + lx) as usize]
    }

    /// Moved up (or down) `dy` cells.
    pub fn lift(&mut self, dy: i32) {
        self.y0 += dy;
        for t in self.top.iter_mut().flatten() {
            *t += dy;
        }
    }

    /// First air cell above the island's grass at column `x`.
    pub fn top_at(&self, x: i32) -> Option<i32> {
        let lx = x - self.x0;
        if lx < 0 || lx >= self.w { None } else { self.top[lx as usize] }
    }

    fn set(&mut self, lx: i32, ly: i32, c: IslandCell) {
        if lx >= 0 && ly >= 0 && lx < self.w && ly < self.h {
            self.cells[(ly * self.w + lx) as usize] = c;
        }
    }

    fn get(&self, lx: i32, ly: i32) -> IslandCell {
        if lx >= 0 && ly >= 0 && lx < self.w && ly < self.h { self.cells[(ly * self.w + lx) as usize] } else { IslandCell::None }
    }
}

/// The lower share of an island's underside its aerolite's veined through,
/// and the rows at its very bottom crusted with it.
const AEROLITE_SHARE: f64 = 0.4;
const AEROLITE_CRUST: i32 = 9;
/// A drifter's: most of it under its soil (a small rock needs more to float).
const DRIFTER_AEROLITE: f64 = 0.75;
/// A drifter's depth under its top, against its half-width.
const DRIFTER_DEPTH: f64 = 1.0;
/// Old lightning scars on an island, at most (and one at least).
const SCARS: u32 = 3;
/// An island this wide (half-width) or wider has a lake on its top, out
/// towards one end, and a waterfall springing from its side under it: the
/// lake this share of its half-width across (at most `LAKE_MOST`), this
/// share of that deep; the spring this many cells tall.
const LAKE_RX: f64 = 420.0;
const LAKE_SHARE: (f64, f64) = (0.12, 0.2);
const LAKE_MOST: f64 = 150.0;
const LAKE_DEPTH: f64 = 0.22;
const SPRING_TALL: i32 = 6;
/// An island's depth (under its top) against its half-width: a small one
/// half as deep as it's wide; past `DEEP_FROM`, deeper by this share only
/// (a great one is a plateau, not a mountain upside down).
const DEPTH: f64 = 0.5;
const DEEP_FROM: f64 = 300.0;
const DEEPER: f64 = 0.25;

fn unit(rng: &mut Rng) -> f64 {
    rng.next_u32() as f64 / u32::MAX as f64
}

/// Up to `count` islands between `x_lo..x_hi`, their tops within `y_lo..y_hi`,
/// half-widths `rx_lo..rx_hi`, never overlapping in x (one island per column,
/// so each column has at most one island top for trees), each where `clear`
/// says it may be (from x, to x, its bottom).
pub fn plan(seed: u64, count: usize, (x_lo, x_hi): (i32, i32), (y_lo, y_hi): (i32, i32), (rx_lo, rx_hi): (f64, f64), clear: impl Fn(i32, i32, i32) -> bool) -> Vec<Island> {
    let mut rng = Rng::seeded(&[seed, 0x15_1A4D]);
    let surf = Perlin::new((seed ^ 0x15AF) as u32);
    let edge = Perlin::new((seed ^ 0x15ED) as u32);
    let mut islands: Vec<Island> = Vec::new();
    const GAP: i32 = 90;
    for _ in 0..count {
        for _try in 0..200 {
            // (Most middling, a few great.)
            let rx = rx_lo + unit(&mut rng).powf(1.6) * (rx_hi - rx_lo);
            let depth = if rx <= DEEP_FROM { rx * DEPTH } else { DEEP_FROM * DEPTH + (rx - DEEP_FROM) * DEEPER };
            let cx = x_lo + rx as i32 + (unit(&mut rng) * (x_hi - x_lo - 2 * rx as i32).max(1) as f64) as i32;
            let top = y_lo + depth as i32 + (unit(&mut rng) * (y_hi - y_lo - depth as i32).max(1) as f64) as i32;
            let (x0, x1) = (cx - rx as i32, cx + rx as i32);
            if islands.iter().any(|i| x1 + GAP >= i.x0 && x0 - GAP <= i.x0 + i.w) || !clear(x0, x1, top - (depth * 1.4) as i32 - 9) {
                continue;
            }
            islands.push(carve(&mut rng, &surf, &edge, cx, top, rx, depth, AEROLITE_SHARE));
            break;
        }
    }
    islands.sort_by_key(|i| i.x0);
    islands
}

/// Drifters about the islands: small rocks with an aerolite heart, free to
/// float (the game moves them: `drift.rs`): `per` (fewest, most) about
/// each island and one more each `every` cells of its width, half-widths `rx_lo..rx_hi`, within the island's width (and
/// `reach` more) across and `rise` up or down of its top; none touching an
/// island or each other, each where `clear` says it may be (from x, to x,
/// its bottom).
pub fn drifters(seed: u64, islands: &[Island], (per_lo, per_hi): (u32, u32), every: i32, (rx_lo, rx_hi): (f64, f64), (reach, rise): (i32, i32), clear: impl Fn(i32, i32, i32) -> bool) -> Vec<Island> {
    let mut rng = Rng::seeded(&[seed, 0xD12F7]);
    let surf = Perlin::new((seed ^ 0xD1AF) as u32);
    let edge = Perlin::new((seed ^ 0xD1ED) as u32);
    let mut out: Vec<Island> = Vec::new();
    const GAP: i32 = 24;
    // (Clear of an island's trees too: they stand this tall over it.)
    const TREES: i32 = 140;
    let apart = |a: (i32, i32, i32, i32), b: &Island, over: i32| a.0 > b.x0 + b.w + GAP || a.1 + GAP < b.x0 || a.2 > b.y0 + b.h + over + GAP || a.3 + GAP < b.y0;
    for isl in islands {
        let n = per_lo + rng.next_u32() % (per_hi - per_lo + 1) + (isl.w / every) as u32;
        for _ in 0..n {
            for _try in 0..40 {
                let rx = rx_lo + unit(&mut rng) * (rx_hi - rx_lo);
                // (Chunkier than an island: a boulder, not a slab.)
                let depth = rx * DRIFTER_DEPTH;
                let cx = isl.x0 - reach + (unit(&mut rng) * (isl.w + 2 * reach) as f64) as i32;
                let top = isl.y0 + isl.h - 9 + ((unit(&mut rng) - 0.5) * 2.0 * rise as f64) as i32;
                let h = (depth * 1.4) as i32 + 18;
                let (x0, x1, y0, y1) = (cx - rx as i32, cx + rx as i32, top - h + 9, top + 9);
                if !islands.iter().all(|i| apart((x0, x1, y0, y1), i, TREES)) || !out.iter().all(|i| apart((x0, x1, y0, y1), i, 0)) || !clear(x0, x1, y0) {
                    continue;
                }
                out.push(carve(&mut rng, &surf, &edge, cx, top, rx, depth, DRIFTER_AEROLITE));
                break;
            }
        }
    }
    out.sort_by_key(|i| i.x0);
    out
}

fn carve(rng: &mut Rng, surf: &Perlin, edge: &Perlin, cx: i32, top: i32, rx: f64, depth: f64, aero: f64) -> Island {
    let w = 2 * rx as i32 + 1;
    let h = (depth * 1.4) as i32 + 18;
    let x0 = cx - rx as i32;
    let y0 = top - h + 9;
    let mut isl = Island { x0, y0, w, h, cells: vec![IslandCell::None; (w * h) as usize], top: vec![None; w as usize] };
    for lx in 0..w {
        let x = x0 + lx;
        let nx = (lx as f64 - rx) / rx;
        if nx.abs() >= 1.0 {
            continue;
        }
        // A gently waving grass line; the edges roll off.
        let crest = top + (surf.get([x as f64 * 0.02, 0.5]) * 4.5) as i32 - (nx.powi(6) * 9.0) as i32;
        // A ragged, tapering underside, sometimes hanging down in a point.
        let taper = (1.0 - nx * nx).sqrt();
        let hang = 1.0 + 0.35 * edge.get([x as f64 / 75.0, 3.1]);
        let under = (depth * taper * hang) as i32 + 5;
        for d in 0..under {
            let y = crest - 1 - d;
            // Jitter the rim without punching holes through it.
            if d >= under - 5 && edge.get([x as f64 / 7.5, y as f64 / 7.5]) < -0.1 {
                continue;
            }
            // (Its underside's lower part veined with aerolite, its last
            // rows crusted with it: what keeps it up.)
            let low = d as f64 >= under as f64 * (1.0 - aero);
            // (Soil a quarter of a small rock's depth, at most 8.)
            let soil = (under / 4).clamp(2, 8);
            let c = match d {
                0 => IslandCell::Grass,
                _ if d <= soil => IslandCell::Dirt,
                _ if low && (d >= under - AEROLITE_CRUST || edge.get([x as f64 / 13.0, y as f64 / 8.0 + 50.0]) > -0.1) => IslandCell::Aerolite,
                _ => IslandCell::Stone,
            };
            isl.set(lx, y - y0, c);
        }
        isl.top[lx as usize] = Some(crest);
    }
    // A great island's lake, and its waterfall.
    if aero < 0.5 && rx >= LAKE_RX {
        lake(rng, &mut isl, rx);
    }
    // Old lightning scars (an island's, not a drifter's): fulgurite in
    // forking veins down from its top.
    if aero < 0.5 {
        for _ in 0..1 + rng.next_u32() % SCARS {
            let lx0 = (unit(rng) * 0.8 + 0.1) * w as f64;
            let Some(top) = isl.top[lx0 as usize] else { continue };
            let mut veins = vec![(lx0, (top - y0 - 1) as f64, 18 + (rng.next_u32() % 40) as i32)];
            while let Some((mut lx, mut ly, mut len)) = veins.pop() {
                while len > 0 && matches!(isl.get(lx as i32, ly as i32), IslandCell::Grass | IslandCell::Dirt | IslandCell::Stone) {
                    isl.set(lx as i32, ly as i32, IslandCell::Fulgurite);
                    if rng.chance(70) {
                        isl.set(lx as i32 + if rng.coin() { 1 } else { -1 }, ly as i32, IslandCell::Fulgurite);
                    }
                    lx += (unit(rng) - 0.5) * 1.6;
                    ly -= 1.0;
                    len -= 1;
                    if rng.chance(14) && veins.len() < 4 {
                        veins.push((lx + if rng.coin() { 2.0 } else { -2.0 }, ly, len / 2));
                    }
                }
            }
        }
    }
    // Caves: a walker or two wandering inside, tunnels and small rooms,
    // turning back before the shell.
    for _ in 0..1 + (rng.next_u32() % 2) {
        let (mut px, mut py) = (rx + (unit(rng) - 0.5) * rx * 0.6, (h - 9) as f64 - depth * (0.35 + unit(rng) * 0.3));
        let a = unit(rng) * std::f64::consts::TAU;
        let (mut dx, mut dy) = (a.cos(), a.sin() * 0.3);
        let steps = (rx * 1.6) as usize + (rng.next_u32() % 150) as usize;
        for _ in 0..steps {
            let r = if rng.chance(18) { 6 + (rng.next_u32() % 4) as i32 } else { 2 + (rng.next_u32() % 2) as i32 };
            for oy in -r..=r {
                for ox in -r..=r {
                    if ox * ox + oy * oy > r * r + r {
                        continue;
                    }
                    let (lx, ly) = (px as i32 + ox, py as i32 + oy);
                    // Keep a shell: stay 5 cells inside solid rock.
                    let inside = |c: IslandCell| !matches!(c, IslandCell::None | IslandCell::Water);
                    let shell = (-5..=5).all(|k| inside(isl.get(lx + k, ly)) && inside(isl.get(lx, ly + k)));
                    if shell && isl.get(lx, ly) == IslandCell::Stone {
                        isl.set(lx, ly, IslandCell::Cave);
                    }
                }
            }
            if rng.chance(40) {
                let t = (unit(rng) - 0.5) * 1.8;
                (dx, dy) = (dx * t.cos() - dy * t.sin(), dx * t.sin() + dy * t.cos());
            }
            let (nx, ny) = (px + dx, py + dy);
            if isl.get(nx as i32, ny as i32) == IslandCell::None || isl.get(nx as i32, ny as i32 + 8) == IslandCell::None {
                (dx, dy) = (-dx, -dy);
            } else {
                (px, py) = (nx, ny);
            }
        }
    }
    isl
}

/// A lake on a great island's top, out towards one end (a quarter of the
/// way in or so): a bowl, its shore flattened to its lower rim's height and
/// filled to just under it; and a waterfall springing from the island's
/// side at that end, a little under the lake (the spring: `Spring` cells
/// the sim pours from).
fn lake(rng: &mut Rng, isl: &mut Island, rx: f64) {
    let lw = (rx * (LAKE_SHARE.0 + unit(rng) * (LAKE_SHARE.1 - LAKE_SHARE.0))).min(LAKE_MOST);
    let ld = lw * LAKE_DEPTH;
    let right = rng.coin();
    let from_end = isl.w as f64 * (0.2 + unit(rng) * 0.1);
    let lc = if right { isl.w as f64 - from_end } else { from_end };
    let (l0, l1) = ((lc - lw) as i32, (lc + lw) as i32);
    let (Some(a), Some(b)) = (isl.top[l0 as usize], isl.top[l1 as usize]) else { return };
    // (Water up to two under its lower rim's top: the rim's grass holds it.)
    let shore = a.min(b) - 1;
    for lx in l0 + 1..l1 {
        let dx = (lx as f64 - lc) / lw;
        let bottom = shore - 2 - (ld * (1.0 - dx * dx).max(0.0).sqrt()) as i32;
        let Some(crest) = isl.top[lx as usize] else { continue };
        // (Never through the island: rock enough under its bed.)
        if isl.get(lx, bottom - isl.y0 - 6) == IslandCell::None {
            continue;
        }
        for y in bottom..crest.max(shore) {
            isl.set(lx, y - isl.y0, if y < shore { IslandCell::Water } else { IslandCell::None });
        }
        isl.set(lx, bottom - 1 - isl.y0, IslandCell::Dirt);
        isl.top[lx as usize] = None;
    }
    // The spring: in the outermost rock at that end, some way under the
    // lake's level (where the edge has rolled off).
    let ly = shore - 12 - isl.y0;
    let mut cols: Box<dyn Iterator<Item = i32>> = if right { Box::new((0..isl.w).rev()) } else { Box::new(0..isl.w) };
    if let Some(lx) = cols.find(|&lx| matches!(isl.get(lx, ly), IslandCell::Grass | IslandCell::Dirt | IslandCell::Stone)) {
        for k in 0..SPRING_TALL {
            if isl.get(lx, ly - k) != IslandCell::None {
                isl.set(lx, ly - k, IslandCell::Spring);
            }
        }
    }
}
