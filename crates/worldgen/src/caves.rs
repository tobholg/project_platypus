//! Caves, planned (DESIGN §3.2 step 4, v2). Noise caves came in every width,
//! so a lot of them were just too narrow for the player; these are laid out
//! when the world is planned, sized in player units, the way Terraria's
//! tile runners carve: chambers, and tunnels that wander between them.
//!
//! - Chambers: ragged ellipses, small in the underground (the first layer
//!   is easy), bigger in the caverns and the deep. Some hold a pool.
//! - Tunnels: every chamber is joined to its neighbours (a spanning tree of
//!   nearest links, so they all connect, and some more for loops), each a
//!   wandering line at least 1.5 player heights wide.
//! - Crevices: a few extra links are cracks too thin for the player (throw a
//!   glow stick in), never the only way through.
//! - Mouths: tunnels down from the surface into the nearest chamber.
//!
//! Noise only roughens the walls. A chunk asks only the shapes binned to it.
//!
//! Underground biomes (regions in the caverns and the deep; the rest keeps
//! its plain rock): fungal hollows (giant glowing mushrooms in the chambers,
//! sprouts on the floors, fungal soil walls), crystal caves (glowing crystal
//! spikes growing into the chambers), toxic grottos (acid in every chamber,
//! glowing crust on the walls). The walls and floors are dressed as a chunk
//! is made (lib.rs); what's shaped by a chamber is planned here.

use std::collections::HashMap;

use noise::{NoiseFn, Perlin};
use platypus_sim::CHUNK;
use platypus_sim::rng::Rng;

/// The player is 15 cells tall: tunnels are at least this wide.
pub const MIN_TUNNEL: f32 = 22.0;
/// Crevices: this wide at most.
pub const MAX_CREVICE: f32 = 7.0;

/// What a chamber's bottom holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pool {
    Water,
    Oil,
    Lava,
    Acid,
}

/// An underground biome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Zone {
    Fungal,
    Crystal,
    Toxic,
}

impl Zone {
    pub const ALL: [Zone; 3] = [Zone::Fungal, Zone::Crystal, Zone::Toxic];

    pub fn name(self) -> &'static str {
        match self {
            Zone::Fungal => "fungal hollows",
            Zone::Crystal => "crystal caves",
            Zone::Toxic => "toxic grottos",
        }
    }
}

/// Where an underground biome is: an ellipse.
#[derive(Clone, Debug)]
pub struct Area {
    pub zone: Zone,
    pub x: f32,
    pub y: f32,
    pub rx: f32,
    pub ry: f32,
}

impl Area {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        ((x - self.x) / self.rx).powi(2) + ((y - self.y) / self.ry).powi(2) < 1.0
    }
}

/// A crystal growing into a chamber: a triangle from its base on the wall
/// to its tip.
#[derive(Clone, Debug)]
pub struct Spike {
    pub base: [(f32, f32); 2],
    pub tip: (f32, f32),
}

/// A giant mushroom standing in a chamber (background): its stem from the
/// floor up, its cap on top.
#[derive(Clone, Copy, Debug)]
pub struct Mushroom {
    pub x: f32,
    pub foot: f32,
    pub top: f32,
    pub stem: f32,
    pub cap: f32,
}

/// What a giant mushroom has at a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shroom {
    Stem,
    /// How far down the cap (0 top … 1 rim), for shading.
    Cap(u8),
}

#[derive(Clone, Debug)]
pub struct Chamber {
    pub x: f32,
    pub y: f32,
    /// Half width and half height (cells).
    pub rx: f32,
    pub ry: f32,
    /// Liquid up to this height, if any.
    pub pool: Option<(Pool, f32)>,
    /// The underground biome it's in, if any.
    pub zone: Option<Zone>,
    pub spikes: Vec<Spike>,
    pub mushrooms: Vec<Mushroom>,
}

#[derive(Clone, Debug)]
pub struct Tunnel {
    pub points: Vec<(f32, f32)>,
    pub width: f32,
    /// The chambers it joins (a mouth: `None`, it comes from the surface).
    pub joins: Option<(u32, u32)>,
}

impl Tunnel {
    pub fn crevice(&self) -> bool {
        self.width <= MAX_CREVICE
    }
}

/// How caves go at a depth: chamber sizes (half width, half height ranges),
/// spacing between chambers, tunnel widths.
struct Layer {
    rx: (f32, f32),
    ry: (f32, f32),
    spacing: f32,
    width: (f32, f32),
}

/// Per chunk: the chambers and (tunnel, segment)s touching it.
type Bins = HashMap<(i32, i32), (Vec<u32>, Vec<(u32, u32)>)>;

pub struct Caves {
    pub areas: Vec<Area>,
    pub chambers: Vec<Chamber>,
    pub tunnels: Vec<Tunnel>,
    bins: Bins,
    rim: Perlin,
}

/// What a cave puts at a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Open {
    Air,
    Pool(Pool),
    /// A crystal growing into a chamber.
    Crystal,
}

fn unit(rng: &mut Rng) -> f32 {
    rng.next_u32() as f32 / u32::MAX as f32
}

fn range(rng: &mut Rng, (lo, hi): (f32, f32)) -> f32 {
    lo + unit(rng) * (hi - lo)
}

/// Distance from p to the segment a–b.
fn to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0) };
    let (qx, qy) = (a.0 + t * dx, a.1 + t * dy);
    ((p.0 - qx).powi(2) + (p.1 - qy).powi(2)).sqrt()
}

/// What the plan tells caves about the world.
pub struct Ground<'a> {
    pub width: i32,
    /// Where caves may be: (lowest y, highest y).
    pub span: (i32, i32),
    /// First air above the ground per column; water surface per column (0:
    /// none).
    pub surface: &'a [i32],
    pub water: &'a [i32],
    /// The top of the caverns and of the deep (the layers change there).
    pub caverns_top: i32,
    pub deep_top: i32,
    /// Keep mouths away from here (the spawn).
    pub spawn_x: i32,
    /// World size relative to the large world (scales counts).
    pub scale: (f64, f64),
}

impl Caves {
    pub fn plan(seed: u64, g: &Ground) -> Caves {
        let mut rng = Rng::seeded(&[seed, 0xCA7E5]);
        let surface = |x: i32| g.surface[x.clamp(0, g.width - 1) as usize];
        let wet = |x: i32| g.water[x.clamp(0, g.width - 1) as usize] > 0;
        let layer = |y: f32| {
            if y >= g.caverns_top as f32 {
                // The underground (and inside the mountains): small, easy.
                Layer { rx: (26.0, 60.0), ry: (18.0, 34.0), spacing: 200.0, width: (22.0, 30.0) }
            } else if y >= g.deep_top as f32 {
                Layer { rx: (45.0, 140.0), ry: (28.0, 80.0), spacing: 330.0, width: (24.0, 40.0) }
            } else {
                Layer { rx: (55.0, 170.0), ry: (35.0, 100.0), spacing: 380.0, width: (26.0, 44.0) }
            }
        };

        let (lo, hi) = g.span;
        // Underground biomes: two of each (one in a small world), in the
        // caverns (toxic grottos in the deep too), apart from each other.
        let mut areas: Vec<Area> = Vec::new();
        let (sw, sh) = (g.scale.0 as f32, g.scale.1 as f32);
        let each = if sw < 0.5 { 1 } else { 2 };
        for zone in Zone::ALL {
            let mut placed = 0;
            for _ in 0..400 {
                if placed == each {
                    break;
                }
                let (rx, ry) = (range(&mut rng, (900.0, 1_500.0)) * sw.max(0.3), range(&mut rng, (450.0, 750.0)) * sh.max(0.3));
                let x = rx + 600.0 + unit(&mut rng) * (g.width as f32 - 2.0 * rx - 1_200.0);
                let (ylo, yhi) = if zone == Zone::Toxic { (lo as f32 + ry, g.caverns_top as f32 - ry) } else { (g.deep_top as f32 + ry * 0.5, g.caverns_top as f32 - ry * 0.7) };
                if yhi <= ylo {
                    continue;
                }
                let y = ylo + unit(&mut rng) * (yhi - ylo);
                if areas.iter().any(|a| ((a.x - x) / (a.rx + rx)).powi(2) + ((a.y - y) / (a.ry + ry)).powi(2) < 1.0) {
                    continue;
                }
                areas.push(Area { zone, x, y, rx, ry });
                placed += 1;
            }
        }
        let zone_at = |x: f32, y: f32| areas.iter().find(|a| a.contains(x, y)).map(|a| a.zone);

        // Chambers: tries spread over the span, kept apart by their layer's
        // spacing, under enough ground (more under lakes and the sea).
        let area = g.width as f64 * (hi - lo) as f64;
        let tries = (area / (150.0 * 150.0)) as usize;
        let mut chambers: Vec<Chamber> = Vec::new();
        let cell = 400.0;
        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        let key = |x: f32, y: f32| ((x / cell).floor() as i32, (y / cell).floor() as i32);
        for _ in 0..tries {
            let x = unit(&mut rng) * g.width as f32;
            let y = lo as f32 + unit(&mut rng) * (hi - lo) as f32;
            let l = layer(y);
            let (rx, ry) = (range(&mut rng, l.rx), range(&mut rng, l.ry));
            let cover = if wet(x as i32) { 140.0 } else { 45.0 };
            let top = ((x - rx) as i32..=(x + rx) as i32).step_by(8).map(surface).min().unwrap_or(0) as f32;
            if y + ry + cover > top || y - ry < lo as f32 || x - rx < 40.0 || x + rx > g.width as f32 - 40.0 {
                continue;
            }
            let (kx, ky) = key(x, y);
            let near = (-1..=1).flat_map(|dy| (-1..=1).map(move |dx| (kx + dx, ky + dy))).flat_map(|k| grid.get(&k).into_iter().flatten()).any(|&j| {
                let c: &Chamber = &chambers[j];
                ((c.x - x).powi(2) + (c.y - y).powi(2)).sqrt() < l.spacing.max(layer(c.y).spacing) * 0.5 + (rx + c.rx) * 0.5
            });
            if near {
                continue;
            }
            // Some hold a pool: water mostly, oil now and then, lava deep down;
            // in the toxic grottos, every one a pool of acid.
            let zone = zone_at(x, y);
            let pool = if zone == Some(Zone::Toxic) {
                Some((Pool::Acid, y - ry + 2.0 * ry * range(&mut rng, (0.25, 0.45))))
            } else if unit(&mut rng) < 0.3 {
                let kind = if y < g.deep_top as f32 && unit(&mut rng) < 0.5 {
                    Pool::Lava
                } else if unit(&mut rng) < 0.15 {
                    Pool::Oil
                } else {
                    Pool::Water
                };
                Some((kind, y - ry + 2.0 * ry * range(&mut rng, (0.2, 0.4))))
            } else {
                None
            };
            grid.entry((kx, ky)).or_default().push(chambers.len());
            chambers.push(Chamber { x, y, rx, ry, pool, zone, spikes: Vec::new(), mushrooms: Vec::new() });
        }

        // Links: each chamber to its nearest few; a spanning tree of them
        // (shortest first) so every chamber connects, then some more for
        // loops, a few of those as crevices.
        let mut links: Vec<(f32, usize, usize)> = Vec::new();
        for (i, c) in chambers.iter().enumerate() {
            let (kx, ky) = key(c.x, c.y);
            let mut near: Vec<(f32, usize)> = (-2..=2)
                .flat_map(|dy| (-2..=2).map(move |dx| (kx + dx, ky + dy)))
                .flat_map(|k| grid.get(&k).into_iter().flatten())
                .filter(|&&j| j != i)
                .map(|&j| (((chambers[j].x - c.x).powi(2) + (chambers[j].y - c.y).powi(2)).sqrt(), j))
                .collect();
            near.sort_by(|a, b| a.0.total_cmp(&b.0));
            for &(d, j) in near.iter().take(4) {
                if i < j {
                    links.push((d, i, j));
                }
            }
        }
        links.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut parent: Vec<usize> = (0..chambers.len()).collect();
        fn root(p: &mut [usize], mut i: usize) -> usize {
            while p[i] != i {
                p[i] = p[p[i]];
                i = p[i];
            }
            i
        }
        let mut tunnels = Vec::new();
        let noise = Perlin::new((seed as u32).wrapping_mul(2654435761) ^ 0x7E1);
        for (n, &(d, i, j)) in links.iter().enumerate() {
            let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
            let tree = ri != rj;
            if tree {
                parent[ri] = rj;
            } else if unit(&mut rng) > 0.3 {
                continue;
            }
            let (a, b) = (&chambers[i], &chambers[j]);
            let l = layer(a.y.min(b.y));
            let width = if !tree && unit(&mut rng) < 0.2 { range(&mut rng, (3.0, MAX_CREVICE)) } else { range(&mut rng, l.width) };
            let mut t = wander((a.x, a.y), (b.x, b.y), width, d, &noise, n as f64);
            t.joins = Some((i as u32, j as u32));
            tunnels.push(t);
        }

        // Mouths: down from the surface into the nearest chamber, on dry land
        // away from the spawn.
        let mouths = (40.0 * g.scale.0).round().max(4.0) as usize;
        let mut made = 0;
        for n in 0..mouths * 20 {
            if made == mouths {
                break;
            }
            let x = 300.0 + unit(&mut rng) * (g.width as f32 - 600.0);
            if (x as i32 - g.spawn_x).abs() < 250 || (x as i32 - 60..x as i32 + 60).step_by(10).any(wet) {
                continue;
            }
            let top = (x, surface(x as i32) as f32 + 4.0);
            let Some(c) = chambers.iter().filter(|c| c.y < top.1 - 60.0).min_by(|a, b| {
                let da = (a.x - top.0).powi(2) + (a.y - top.1).powi(2);
                let db = (b.x - top.0).powi(2) + (b.y - top.1).powi(2);
                da.total_cmp(&db)
            }) else {
                continue;
            };
            let d = ((c.x - top.0).powi(2) + (c.y - top.1).powi(2)).sqrt();
            if d > 700.0 {
                continue;
            }
            let w = range(&mut rng, (22.0, 28.0));
            tunnels.push(wander(top, (c.x, c.y), w, d, &noise, 10_000.0 + n as f64));
            made += 1;
        }

        // A pool stays below where any tunnel comes into its chamber, or it
        // would run out into the tunnel (no pool if that leaves too little).
        for tun in &tunnels {
            for &(px, py) in &tun.points {
                let (kx, ky) = key(px, py);
                let near: Vec<usize> = (-1..=1).flat_map(|dy| (-1..=1).map(move |dx| (kx + dx, ky + dy))).flat_map(|k| grid.get(&k).into_iter().flatten().copied()).collect();
                for i in near {
                    let c = &mut chambers[i];
                    if c.pool.is_none() {
                        continue;
                    }
                    let (dx, dy) = ((px - c.x) / c.rx, (py - c.y) / c.ry);
                    if dx * dx + dy * dy < 1.6 {
                        let entry = py - tun.width * 0.65 - 2.0;
                        if let Some((_, level)) = &mut c.pool {
                            *level = level.min(entry);
                        }
                    }
                }
            }
        }
        for c in &mut chambers {
            if c.pool.is_some_and(|(_, level)| level < c.y - c.ry * 0.8) {
                c.pool = None;
            }
        }

        // What grows in the biomes' chambers: crystals from the walls (not
        // where a tunnel comes in), giant mushrooms on the floors.
        let mut ways: Vec<Vec<f32>> = vec![Vec::new(); chambers.len()];
        for t in &tunnels {
            if let Some((a, b)) = t.joins {
                let n = t.points.len();
                let (p0, p1) = (t.points[0], t.points[1.min(n - 1)]);
                let (q0, q1) = (t.points[n - 1], t.points[n.saturating_sub(2)]);
                ways[a as usize].push((p1.1 - p0.1).atan2(p1.0 - p0.0));
                ways[b as usize].push((q1.1 - q0.1).atan2(q1.0 - q0.0));
            }
        }
        for (c, ways) in chambers.iter_mut().zip(&ways) {
            match c.zone {
                Some(Zone::Crystal) => {
                    for _ in 0..(6.0 + c.rx / 12.0) as usize {
                        let a = unit(&mut rng) * std::f32::consts::TAU;
                        let clear = ways.iter().all(|w| {
                            let d = (a - w).rem_euclid(std::f32::consts::TAU);
                            d.min(std::f32::consts::TAU - d) > 0.45
                        });
                        if !clear {
                            continue;
                        }
                        // From just outside the wall toward the middle, a third
                        // to a half of the way, a little askew.
                        let base = (c.x + c.rx * 1.05 * a.cos(), c.y + c.ry * 1.05 * a.sin());
                        let reach = range(&mut rng, (0.3, 0.5));
                        let skew = range(&mut rng, (-0.25, 0.25));
                        let (dx, dy) = (c.x - base.0, c.y - base.1);
                        let tip = (base.0 + (dx - dy * skew) * reach, base.1 + (dy + dx * skew) * reach);
                        let half = range(&mut rng, (3.0, 8.0));
                        let len = (dx * dx + dy * dy).sqrt().max(1.0);
                        let (px, py) = (-dy / len * half, dx / len * half);
                        c.spikes.push(Spike { base: [(base.0 + px, base.1 + py), (base.0 - px, base.1 - py)], tip });
                    }
                }
                Some(Zone::Fungal) => {
                    for _ in 0..(1.0 + c.rx / 45.0) as usize {
                        let u = range(&mut rng, (-0.7, 0.7));
                        let x = c.x + c.rx * u;
                        let floor = c.y - c.ry * (1.0 - u * u).sqrt();
                        let room = c.y + c.ry * (1.0 - u * u).sqrt() - floor;
                        let h = (room * range(&mut rng, (0.45, 0.8))).clamp(24.0, 110.0);
                        let cap = (h * range(&mut rng, (0.25, 0.4))).clamp(8.0, 30.0);
                        c.mushrooms.push(Mushroom { x, foot: floor - 12.0, top: floor + h, stem: (cap / 5.0).clamp(2.0, 5.0), cap });
                    }
                }
                _ => {}
            }
        }

        // Bins: every chunk a shape's box touches.
        let mut bins: Bins = HashMap::new();
        let chunks = |x0: f32, y0: f32, x1: f32, y1: f32| {
            let (cx0, cy0, cx1, cy1) = ((x0 as i32).div_euclid(CHUNK), (y0 as i32).div_euclid(CHUNK), (x1 as i32).div_euclid(CHUNK), (y1 as i32).div_euclid(CHUNK));
            (cy0..=cy1).flat_map(move |cy| (cx0..=cx1).map(move |cx| (cx, cy)))
        };
        for (i, c) in chambers.iter().enumerate() {
            // (The rim can reach 30% past the ellipse.)
            for k in chunks(c.x - c.rx * 1.35, c.y - c.ry * 1.35, c.x + c.rx * 1.35, c.y + c.ry * 1.35) {
                bins.entry(k).or_default().0.push(i as u32);
            }
        }
        for (t, tun) in tunnels.iter().enumerate() {
            let r = tun.width * 0.6 + 2.0;
            for (s, w) in tun.points.windows(2).enumerate() {
                let (a, b) = (w[0], w[1]);
                for k in chunks(a.0.min(b.0) - r, a.1.min(b.1) - r, a.0.max(b.0) + r, a.1.max(b.1) + r) {
                    bins.entry(k).or_default().1.push((t as u32, s as u32));
                }
            }
        }
        Caves { areas, chambers, tunnels, bins, rim: Perlin::new((seed as u32) ^ 0xC0FE) }
    }

    /// The underground biome at a cell, if any.
    pub fn zone_at(&self, x: i32, y: i32) -> Option<Zone> {
        self.areas.iter().find(|a| a.contains(x as f32, y as f32)).map(|a| a.zone)
    }

    /// A giant mushroom at a cell (background), if one stands there.
    pub fn mushroom_at(&self, x: i32, y: i32) -> Option<Shroom> {
        let (chambers, _) = self.bins.get(&(x.div_euclid(CHUNK), y.div_euclid(CHUNK)))?;
        let (px, py) = (x as f32, y as f32);
        for &i in chambers {
            for m in &self.chambers[i as usize].mushrooms {
                let dx = px - m.x;
                // The cap: a dome, flat underneath.
                if py >= m.top - m.cap * 0.45 && py <= m.top + m.cap * 0.55 {
                    let (u, v) = (dx / m.cap, (py - (m.top - m.cap * 0.45)) / m.cap);
                    if u * u + v * v < 1.0 && v >= 0.0 {
                        return Some(Shroom::Cap((v * 255.0).min(255.0) as u8));
                    }
                }
                if dx.abs() <= m.stem / 2.0 + (m.top - py).max(0.0) / 40.0 && py >= m.foot && py < m.top - m.cap * 0.45 {
                    return Some(Shroom::Stem);
                }
            }
        }
        None
    }

    /// What the caves put at a cell, if they reach it.
    pub fn at(&self, x: i32, y: i32) -> Option<Open> {
        let (cx, cy) = (x.div_euclid(CHUNK), y.div_euclid(CHUNK));
        let (chambers, segments) = self.bins.get(&(cx, cy))?;
        let p = (x as f32, y as f32);
        for &i in chambers {
            let c = &self.chambers[i as usize];
            let (dx, dy) = ((p.0 - c.x) / c.rx, (p.1 - c.y) / c.ry);
            let d = dx * dx + dy * dy;
            if d > 1.7 {
                continue;
            }
            // A ragged rim, finer on small chambers.
            let s = (c.rx.min(c.ry) * 0.5).max(10.0) as f64;
            let n = self.rim.get([x as f64 / s, y as f64 / s, i as f64 * 0.37]) as f32;
            if c.spikes.iter().any(|s| in_triangle(p, s.base[0], s.base[1], s.tip)) {
                return Some(Open::Crystal);
            }
            if d < 1.0 + 0.45 * n {
                return Some(match c.pool {
                    Some((kind, level)) if p.1 < level => Open::Pool(kind),
                    _ => Open::Air,
                });
            }
        }
        for &(t, s) in segments {
            let tun = &self.tunnels[t as usize];
            let (a, b) = (tun.points[s as usize], tun.points[s as usize + 1]);
            let half = tun.width / 2.0;
            let d = to_segment(p, a, b);
            if d > half * 1.3 {
                continue;
            }
            let wob = if tun.crevice() { 0.0 } else { self.rim.get([x as f64 / 14.0, y as f64 / 14.0, 50.0 + t as f64 * 0.11]) as f32 * 0.25 };
            if d < half * (1.0 + wob) {
                if !tun.crevice() && ledge(p, a, b) {
                    continue;
                }
                return Some(Open::Air);
            }
        }
        None
    }
}

fn in_triangle(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let side = |p: (f32, f32), a: (f32, f32), b: (f32, f32)| (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
    let (d1, d2, d3) = (side(p, a, b), side(p, b, c), side(p, c, a));
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

/// Steep tunnels get ledges to climb back up: every `LEDGE_EVERY` cells a
/// shelf of rock `LEDGE` thick out from one wall and then the other, halfway
/// across (the player jumps 40 cells and is 6 wide: room to fall past, a
/// ledge to jump to).
const LEDGE_EVERY: i32 = 30;
const LEDGE: i32 = 4;

fn ledge(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    if dy.abs() / len < 0.75 {
        return false;
    }
    let y = p.1 as i32;
    if y.rem_euclid(LEDGE_EVERY) >= LEDGE {
        return false;
    }
    // Which wall this row's ledge grows from, and is p on that side of the
    // tunnel's middle?
    let side = if y.div_euclid(LEDGE_EVERY) % 2 == 0 { 1.0 } else { -1.0 };
    let cross = (dx * (p.1 - a.1) - dy * (p.0 - a.0)) / len * dy.signum();
    cross * side > 0.0
}

/// A line from a to b that wanders sideways (up to a fifth of its length,
/// at most 140 cells), a point every ~30 cells.
fn wander(a: (f32, f32), b: (f32, f32), width: f32, len: f32, noise: &Perlin, salt: f64) -> Tunnel {
    let n = (len / 30.0).ceil().max(2.0) as usize;
    let (dx, dy) = ((b.0 - a.0) / len.max(1.0), (b.1 - a.1) / len.max(1.0));
    let (px, py) = (-dy, dx);
    let amp = (len * 0.2).min(140.0);
    let points = (0..=n)
        .map(|k| {
            let t = k as f32 / n as f32;
            // Pinned at both ends.
            let off = noise.get([t as f64 * 2.3, salt]) as f32 * amp * (std::f32::consts::PI * t).sin();
            (a.0 + (b.0 - a.0) * t + px * off, a.1 + (b.1 - a.1) * t + py * off)
        })
        .collect();
    Tunnel { points, width, joins: None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Preset, WorldPlan};

    #[test]
    fn caves_connect_without_crevices_and_tunnels_fit_the_player() {
        let p = WorldPlan::new(1, Preset::Large);
        let c = &p.caves;
        assert!(c.chambers.len() > 500, "{} chambers", c.chambers.len());
        let crevices = c.tunnels.iter().filter(|t| t.crevice()).count();
        assert!(c.tunnels.iter().all(|t| t.crevice() || t.width >= MIN_TUNNEL), "every tunnel fits the player, or is clearly a crack");
        assert!(crevices * 100 > c.tunnels.len() && crevices * 10 < c.tunnels.len(), "a few crevices: {crevices} of {}", c.tunnels.len());
        // Joined by tunnels the player fits (crevices don't count): nearly
        // every chamber in one network.
        let mut parent: Vec<usize> = (0..c.chambers.len()).collect();
        fn root(p: &mut [usize], mut i: usize) -> usize {
            while p[i] != i {
                p[i] = p[p[i]];
                i = p[i];
            }
            i
        }
        for t in c.tunnels.iter().filter(|t| !t.crevice()) {
            if let Some((a, b)) = t.joins {
                let (ra, rb) = (root(&mut parent, a as usize), root(&mut parent, b as usize));
                parent[ra] = rb;
            }
        }
        let mut sizes: HashMap<usize, usize> = HashMap::new();
        for i in 0..c.chambers.len() {
            *sizes.entry(root(&mut parent, i)).or_default() += 1;
        }
        let biggest = sizes.values().max().copied().unwrap_or(0);
        assert!(biggest * 100 >= c.chambers.len() * 95, "one network holds {biggest} of {} chambers", c.chambers.len());
    }
}
