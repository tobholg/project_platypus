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

fn unit(rng: &mut Rng) -> f64 {
    rng.next_u32() as f64 / u32::MAX as f64
}

/// Up to `count` islands between `x_lo..x_hi`, their tops within `y_lo..y_hi`,
/// half-widths `rx_lo..rx_hi`, never overlapping in x (one island per column,
/// so each column has at most one island top for trees).
pub fn plan(seed: u64, count: usize, (x_lo, x_hi): (i32, i32), (y_lo, y_hi): (i32, i32), (rx_lo, rx_hi): (f64, f64)) -> Vec<Island> {
    let mut rng = Rng::seeded(&[seed, 0x15_1A4D]);
    let surf = Perlin::new((seed ^ 0x15AF) as u32);
    let edge = Perlin::new((seed ^ 0x15ED) as u32);
    let mut islands: Vec<Island> = Vec::new();
    const GAP: i32 = 60;
    for _ in 0..count {
        for _try in 0..200 {
            let rx = rx_lo + unit(&mut rng) * (rx_hi - rx_lo);
            let depth = rx * 0.5;
            let cx = x_lo + rx as i32 + (unit(&mut rng) * (x_hi - x_lo - 2 * rx as i32).max(1) as f64) as i32;
            let top = y_lo + depth as i32 + (unit(&mut rng) * (y_hi - y_lo - depth as i32).max(1) as f64) as i32;
            let (x0, x1) = (cx - rx as i32, cx + rx as i32);
            if islands.iter().any(|i| x1 + GAP >= i.x0 && x0 - GAP <= i.x0 + i.w) {
                continue;
            }
            islands.push(carve(&mut rng, &surf, &edge, cx, top, rx, depth));
            break;
        }
    }
    islands.sort_by_key(|i| i.x0);
    islands
}

fn carve(rng: &mut Rng, surf: &Perlin, edge: &Perlin, cx: i32, top: i32, rx: f64, depth: f64) -> Island {
    let w = 2 * rx as i32 + 1;
    let h = (depth * 1.4) as i32 + 12;
    let x0 = cx - rx as i32;
    let y0 = top - h + 6;
    let mut isl = Island { x0, y0, w, h, cells: vec![IslandCell::None; (w * h) as usize], top: vec![None; w as usize] };
    for lx in 0..w {
        let x = x0 + lx;
        let nx = (lx as f64 - rx) / rx;
        if nx.abs() >= 1.0 {
            continue;
        }
        // A gently waving grass line; the edges roll off.
        let crest = top + (surf.get([x as f64 * 0.03, 0.5]) * 3.0) as i32 - (nx.powi(6) * 6.0) as i32;
        // A ragged, tapering underside, sometimes hanging down in a point.
        let taper = (1.0 - nx * nx).sqrt();
        let hang = 1.0 + 0.35 * edge.get([x as f64 * 0.02, 3.1]);
        let under = (depth * taper * hang) as i32 + 3;
        for d in 0..under {
            let y = crest - 1 - d;
            // Jitter the rim without punching holes through it.
            if d >= under - 3 && edge.get([x as f64 * 0.2, y as f64 * 0.2]) < -0.1 {
                continue;
            }
            let c = match d {
                0 => IslandCell::Grass,
                1..=5 => IslandCell::Dirt,
                _ => IslandCell::Stone,
            };
            isl.set(lx, y - y0, c);
        }
        isl.top[lx as usize] = Some(crest);
    }
    // Caves: a walker or two wandering inside, tunnels and small rooms,
    // turning back before the shell.
    for _ in 0..1 + (rng.next_u32() % 2) {
        let (mut px, mut py) = (rx + (unit(rng) - 0.5) * rx * 0.6, (h - 6) as f64 - depth * (0.35 + unit(rng) * 0.3));
        let a = unit(rng) * std::f64::consts::TAU;
        let (mut dx, mut dy) = (a.cos(), a.sin() * 0.3);
        let steps = (rx * 1.6) as usize + (rng.next_u32() % 100) as usize;
        for _ in 0..steps {
            let r = if rng.chance(18) { 4 + (rng.next_u32() % 3) as i32 } else { 1 + (rng.next_u32() % 2) as i32 };
            for oy in -r..=r {
                for ox in -r..=r {
                    if ox * ox + oy * oy > r * r + r {
                        continue;
                    }
                    let (lx, ly) = (px as i32 + ox, py as i32 + oy);
                    // Keep a shell: stay 3 cells inside solid rock.
                    let shell = (-3..=3).all(|k| isl.get(lx + k, ly) != IslandCell::None && isl.get(lx, ly + k) != IslandCell::None);
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
            if isl.get(nx as i32, ny as i32) == IslandCell::None || isl.get(nx as i32, ny as i32 + 5) == IslandCell::None {
                (dx, dy) = (-dx, -dy);
            } else {
                (px, py) = (nx, ny);
            }
        }
    }
    isl
}
