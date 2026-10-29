//! Space (the space arc): the asteroids over the sky. Planned once (where
//! each is, how big, what kind), read a cell at a time by the chunks (a
//! rock's lumpy edge, its craters, its skin of dust, its veins and
//! hollows: all from its place and its seed), so a chunk's the same
//! whenever it's made.
//!
//! Three reaches up through it: the shallows just over the void's edge
//! (sparse, small, icy and stony), the belts (asteroid fields across the
//! world, some wide and loose, some tight clusters) and high space over
//! them (sparse, metal more often).

use std::collections::HashMap;

use noise::{NoiseFn, Perlin};
use platypus_sim::rng::{Rng, hash};

/// What an asteroid's made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Stone and basalt under a skin of regolith.
    Stony,
    /// Ice, gritty with gravel.
    Icy,
    /// A shell of basalt round a starmetal heart.
    Metal,
    /// Basalt grown through with solarite crystals.
    Sunstone,
}

/// What a cell of space holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    None,
    Stone,
    Basalt,
    Regolith,
    Ice,
    Grit,
    Starmetal,
    Solarite,
    /// Inside a big rock but hollow: air in front, rock behind.
    Hollow,
}

#[derive(Clone, Debug)]
pub struct Asteroid {
    pub x: i32,
    pub y: i32,
    pub r: f32,
    pub kind: Kind,
    seed: u64,
}

/// The reaches, as shares of space's height up from its floor: the
/// shallows to the first, the belts to the second, high space over it.
const REACHES: (f64, f64) = (0.14, 0.78);
/// Asteroid fields in the belts (a reference world's), their half-extents
/// across and up (cells), and how much of a field rock fills (fewest, most).
const FIELDS: f64 = 30.0;
const FIELD_RX: (f64, f64) = (1_200.0, 5_000.0);
const FIELD_RY: (f64, f64) = (400.0, 1_600.0);
const FIELD_FILL: (f64, f64) = (0.012, 0.045);
/// Loose rocks in the shallows and high space: one to this many cells of
/// it (a reference world's).
const SPARSE: f64 = 900_000.0;
/// Radii (cells): the smallest, and the largest in the belts and loose
/// (most are small: `SKEW`).
const R_LO: f64 = 3.0;
const R_BELT: f64 = 150.0;
const R_LOOSE: f64 = 40.0;
const SKEW: f64 = 4.0;
/// No closer than this share of their radii added (a few touch, none sit
/// inside another).
const APART: f32 = 0.85;
/// The index's buckets (cells a side).
const BUCKET: i32 = 256;
/// How lumpy an edge is (of its radius), and how fine its lumps are.
const LUMPS: f64 = 0.3;
/// Over this radius a rock has craters bitten into it (a few, by size),
/// and over this, hollows inside.
const CRATERED: f32 = 10.0;
const HOLLOWED: f32 = 60.0;

pub struct Space {
    pub floor: i32,
    pub top: i32,
    rocks: Vec<Asteroid>,
    index: HashMap<(i32, i32), Vec<u32>>,
    shape: Perlin,
    veins: Perlin,
}

fn unit(rng: &mut Rng) -> f64 {
    rng.next_u32() as f64 / u32::MAX as f64
}

fn range(rng: &mut Rng, (lo, hi): (f64, f64)) -> f64 {
    lo + unit(rng) * (hi - lo)
}

impl Space {
    /// The asteroids from `floor` (the void's edge) to `top` (the world's),
    /// `width` across; `scale` the world's size against the reference's
    /// (across, up), `count` its count across.
    pub fn plan(seed: u64, width: i32, floor: i32, top: i32, (sw, sh): (f64, f64), count: f64) -> Space {
        let mut rng = Rng::seeded(&[seed, 0x5BACE]);
        let mut space = Space { floor, top, rocks: Vec::new(), index: HashMap::new(), shape: Perlin::new((hash(&[seed, 0x5A]) & 0xFFFF_FFFF) as u32), veins: Perlin::new((hash(&[seed, 0x5B]) & 0xFFFF_FFFF) as u32) };
        let span = (top - floor) as f64;
        if span < 200.0 {
            return space;
        }
        let at = |share: f64| floor + (share * span) as i32;
        let (shallows, belts) = (at(REACHES.0), at(REACHES.1));
        let radius = |rng: &mut Rng, most: f64| (R_LO + (most - R_LO) * unit(rng).powf(SKEW)) as f32;
        // The belts' fields.
        for _ in 0..(FIELDS * count).round().max(2.0) as usize {
            let (rx, ry) = (range(&mut rng, FIELD_RX) * sw.max(0.25), range(&mut rng, FIELD_RY) * sh.max(0.25));
            let cx = (unit(&mut rng) * width as f64) as i32;
            let cy = shallows + ry as i32 + (unit(&mut rng) * ((belts - shallows) as f64 - 2.0 * ry).max(1.0)) as i32;
            let fill = range(&mut rng, FIELD_FILL);
            let most = R_BELT * sw.max(0.25).sqrt();
            // (As many as fill that much of it, at a typical rock's size.)
            let typical = (R_LO + (most - R_LO) / (1.0 + SKEW)).powi(2) * std::f64::consts::PI;
            let n = (std::f64::consts::PI * rx * ry * fill / typical) as usize;
            for _ in 0..n {
                // (Thicker toward its middle.)
                let (u, v) = (unit(&mut rng) * 2.0 - 1.0, unit(&mut rng) * 2.0 - 1.0);
                let (u, v) = (u * u.abs().sqrt(), v * v.abs().sqrt());
                let (x, y) = ((cx as f64 + u * rx) as i32, (cy as f64 + v * ry) as i32);
                let r = radius(&mut rng, most);
                let kind = match unit(&mut rng) {
                    k if k < 0.55 => Kind::Stony,
                    k if k < 0.75 => Kind::Metal,
                    k if k < 0.9 => Kind::Icy,
                    _ => Kind::Sunstone,
                };
                space.add(x.rem_euclid(width), y, r, kind, &mut rng);
            }
        }
        // Loose rocks: the shallows (icy and stony), high space (metal more).
        for (lo, hi, metal) in [(floor + 120, shallows, false), (belts, top - 600, true)] {
            if hi <= lo {
                continue;
            }
            let n = (width as f64 * (hi - lo) as f64 / (SPARSE * sw.max(0.25) * sh.max(0.25))) as usize;
            for _ in 0..n {
                let (x, y) = ((unit(&mut rng) * width as f64) as i32, lo + (unit(&mut rng) * (hi - lo) as f64) as i32);
                let r = radius(&mut rng, R_LOOSE);
                let kind = match (unit(&mut rng), metal) {
                    (k, false) if k < 0.5 => Kind::Icy,
                    (k, true) if k < 0.35 => Kind::Metal,
                    (k, true) if k < 0.45 => Kind::Sunstone,
                    _ => Kind::Stony,
                };
                space.add(x, y, r, kind, &mut rng);
            }
        }
        space
    }

    /// One more, if it's clear of the others (and of space's floor and top).
    fn add(&mut self, x: i32, y: i32, r: f32, kind: Kind, rng: &mut Rng) {
        let reach = (r * (1.0 + LUMPS as f32)).ceil() as i32 + 2;
        if y - reach < self.floor || y + reach > self.top - 400 {
            return;
        }
        if self.near(x, y, reach + 2 * R_BELT as i32).any(|o| ((o.x - x) as f32).hypot((o.y - y) as f32) < (o.r + r) * APART) {
            return;
        }
        let i = self.rocks.len() as u32;
        self.rocks.push(Asteroid { x, y, r, kind, seed: rng.next_u64() });
        for by in (y - reach).div_euclid(BUCKET)..=(y + reach).div_euclid(BUCKET) {
            for bx in (x - reach).div_euclid(BUCKET)..=(x + reach).div_euclid(BUCKET) {
                self.index.entry((bx, by)).or_default().push(i);
            }
        }
    }

    /// The rocks whose buckets are within `reach` of (x, y).
    fn near(&self, x: i32, y: i32, reach: i32) -> impl Iterator<Item = &Asteroid> {
        let (b0, b1) = (((x - reach).div_euclid(BUCKET), (y - reach).div_euclid(BUCKET)), ((x + reach).div_euclid(BUCKET), (y + reach).div_euclid(BUCKET)));
        let mut seen: Vec<u32> = (b0.1..=b1.1).flat_map(|by| (b0.0..=b1.0).map(move |bx| (bx, by))).filter_map(|k| self.index.get(&k)).flatten().copied().collect();
        seen.sort_unstable();
        seen.dedup();
        seen.into_iter().map(|i| &self.rocks[i as usize])
    }

    pub fn rocks(&self) -> &[Asteroid] {
        &self.rocks
    }

    /// Any rock might reach into this box (a chunk's, say)?
    pub fn any_in(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        if y1 < self.floor || y0 > self.top {
            return false;
        }
        (y0.div_euclid(BUCKET)..=y1.div_euclid(BUCKET)).any(|by| (x0.div_euclid(BUCKET)..=x1.div_euclid(BUCKET)).any(|bx| self.index.contains_key(&(bx, by))))
    }

    /// What's at a cell (its x as stored: 0 … the world's width).
    pub fn at(&self, x: i32, y: i32) -> Cell {
        if y < self.floor || y > self.top {
            return Cell::None;
        }
        let Some(list) = self.index.get(&(x.div_euclid(BUCKET), y.div_euclid(BUCKET))) else { return Cell::None };
        for &i in list {
            let c = self.rocks[i as usize].at(x, y, &self.shape, &self.veins);
            if c != Cell::None {
                return c;
            }
        }
        Cell::None
    }
}

impl Asteroid {
    /// What it holds at a cell.
    fn at(&self, x: i32, y: i32, shape: &Perlin, veins: &Perlin) -> Cell {
        let (dx, dy) = ((x - self.x) as f64, (y - self.y) as f64);
        let r = self.r as f64;
        let d = dx.hypot(dy);
        if d > r * (1.0 + LUMPS) + 1.0 {
            return Cell::None;
        }
        // Its lumpy edge: further out or in by the noise round it.
        let s = (self.seed % 10_000) as f64 * 7.13;
        let f = 1.0 / (r * 0.6 + 3.0);
        let edge = r * (1.0 + LUMPS * shape.get([dx * f + s, dy * f]) + 0.08 * shape.get([dx * f * 4.0 + s, dy * f * 4.0 + 50.0]));
        if d > edge {
            return Cell::None;
        }
        // Craters bitten out of its face.
        if self.r > CRATERED {
            let n = 1 + (self.seed >> 20) % (2 + (self.r / 25.0) as u64);
            for k in 0..n {
                let h = hash(&[self.seed, k]);
                let a = (h % 6_283) as f64 / 1_000.0;
                let cr = r * (0.15 + ((h >> 16) % 200) as f64 / 1_000.0);
                let (ex, ey) = (a.cos() * (edge + cr * 0.3), a.sin() * (edge + cr * 0.3));
                if (dx - ex).hypot(dy - ey) < cr {
                    return Cell::None;
                }
            }
        }
        let deep = edge - d;
        let v = veins.get([x as f64 / 9.0 + s, y as f64 / 9.0]);
        // Big ones are hollow in places, deep in.
        if self.r > HOLLOWED && deep > 8.0 && veins.get([x as f64 / 23.0 + s, y as f64 / 23.0 + 300.0]) > 0.42 {
            return Cell::Hollow;
        }
        let skin = 1.5 + r * 0.04;
        match self.kind {
            Kind::Stony => {
                if deep < skin {
                    Cell::Regolith
                } else if v > 0.62 {
                    Cell::Starmetal
                } else if v > 0.2 {
                    Cell::Basalt
                } else {
                    Cell::Stone
                }
            }
            Kind::Icy => {
                if deep < skin * 0.5 {
                    Cell::Regolith
                } else if v > 0.35 {
                    Cell::Grit
                } else {
                    Cell::Ice
                }
            }
            Kind::Metal => {
                if deep < skin * 0.6 {
                    Cell::Regolith
                } else if deep > r * 0.3 && v > -0.35 {
                    Cell::Starmetal
                } else {
                    Cell::Basalt
                }
            }
            Kind::Sunstone => {
                // (Crystals cluster toward its face: grown in the sun.)
                if v > 0.3 - (1.0 - deep / r).clamp(0.0, 1.0) * 0.35 {
                    Cell::Solarite
                } else {
                    Cell::Basalt
                }
            }
        }
    }
}
