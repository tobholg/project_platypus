//! The world plan (DESIGN §3.1): what is decided once from the seed, before
//! any chunk: the world's size, sea level and vertical bands, biomes, the
//! surface with its mountains, oceans and lakes, sky islands, forests, the
//! climate. Chunks rasterise from it and never read each other, so any chunk
//! can be generated at any time, on any thread, by any co-op peer.

use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use platypus_sim::climate::CLIMATE_COLUMNS;
use platypus_sim::rng::{Rng, hash};
use platypus_sim::{CHUNK, Climate};

use crate::biome::Biome;
use crate::flora::{Forest, Species};
use crate::islands::{self, Island};
use crate::structures::{self, Structure, Structures};

/// World sizes. `Large` is the world we play in; `Small` is quick to look at
/// and to test with (the same world scaled down; small things like hills and
/// trees keep their size).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Small,
    Large,
}

impl Preset {
    /// Size in chunks: (width, height).
    pub fn chunks(self) -> (i32, i32) {
        match self {
            Preset::Small => (128, 64),
            Preset::Large => (512, 256),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Preset::Small => "small",
            Preset::Large => "large",
        }
    }

    pub fn from_name(name: &str) -> Option<Preset> {
        match name {
            "small" => Some(Preset::Small),
            "large" => Some(Preset::Large),
            _ => None,
        }
    }
}

/// Vertical bands, top to bottom (DESIGN §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Band {
    Sky,
    Peaks,
    Surface,
    Underground,
    Caverns,
    Deep,
    Underworld,
}

impl Band {
    pub const ALL: [Band; 7] = [Band::Sky, Band::Peaks, Band::Surface, Band::Underground, Band::Caverns, Band::Deep, Band::Underworld];

    pub fn name(self) -> &'static str {
        match self {
            Band::Sky => "sky",
            Band::Peaks => "peaks",
            Band::Surface => "surface",
            Band::Underground => "underground",
            Band::Caverns => "caverns",
            Band::Deep => "deep",
            Band::Underworld => "underworld",
        }
    }
}

/// Where each band below the sky starts, in cells above (+) or below (−)
/// sea level, in the large world; other heights scale them.
const BAND_TOPS: [i32; 6] = [2_500, 800, -200, -2_500, -7_000, -11_000];
const LARGE_WIDTH: f64 = 32_768.0;
const LARGE_HEIGHT: f64 = 16_384.0;
/// Sea level, as a share of the world's height (a quarter from the top).
const SEA_LEVEL: f64 = 0.75;
/// °C at sea level (before a biome's warmth).
const SURFACE_TEMP: i32 = 15;
/// Cells of climb per 1 °C colder (large world): 0 °C at +900 in a
/// temperate biome, so snow caps the mountains.
const CELLS_PER_DEGREE_UP: f64 = 60.0;
/// Above the peaks the air warms again, 1 °C per this many cells (large
/// world): the sky islands are mild while the summits are the coldest place.
const INVERSION: f64 = 18.0;
/// °C warmer at the bottom of the world than at sea level.
const WARMER_AT_BOTTOM: i32 = 85;
/// Broadleaf trees grow where the ground is at least this warm (°C);
/// conifers down to `CONIFER_LINE`, snowy where it freezes.
const TREE_LINE: i32 = 4;
const CONIFER_LINE: i32 = -18;

// Large-world sizes (scaled by width or height for smaller worlds).
const OCEAN_WIDTH: f64 = 1_600.0;
const OCEAN_DEPTH: f64 = 380.0;
const REGION_WIDTH: (f64, f64) = (1_800.0, 4_500.0);
/// Biome borders blend over this many cells each side.
const BLEND: f64 = 300.0;
/// Lone massifs outside the mountain ranges (large world).
const MOUNTAINS: f64 = 3.0;
/// A mountain range: a peak every so often across the region, each a massif.
const RANGE_STEP: (f64, f64) = (900.0, 1_400.0);
const RANGE_HALF_WIDTH: (f64, f64) = (850.0, 1_400.0);
const RANGE_PEAK: (f64, f64) = (1_400.0, 2_200.0);
/// The ridge a range's peaks stand on (large world).
const RANGE_BODY: (f64, f64) = (700.0, 1_000.0);
const MOUNTAIN_HALF_WIDTH: (f64, f64) = (1_400.0, 3_000.0);
const MOUNTAIN_PEAK: (f64, f64) = (900.0, 2_400.0);
/// No mountain within this of the spawn (the start is a gentle forest).
const SPAWN_CLEAR: f64 = 1_600.0;
const LAKE_BOWLS: f64 = 3.0;
const ISLANDS: f64 = 12.0;
/// How far either side a lake's rims are looked for (large world): wider
/// valleys are dry land, not one giant lake.
const LAKE_REACH: f64 = 1_200.0;
const CHASMS: f64 = 5.0;
/// Crypts under ruins in the lowlands (large world), at least this far apart.
const CRYPTS: f64 = 10.0;
const CRYPT_SPACING: f64 = 1_400.0;
/// Castles on the summits (large world).
const CASTLES: f64 = 3.0;
const CHASM_WIDTH: (f64, f64) = (110.0, 240.0);
/// Columns sharing one cavern water table.
const WATER_TABLE_SPAN: i32 = 2_048;
/// Height of a mountain's cliff bands (large world).
const TERRACE: f64 = 70.0;

pub struct WorldPlan {
    pub seed: u64,
    pub preset: Preset,
    /// Size in cells.
    pub width: i32,
    pub height: i32,
    pub sea_level: i32,
    /// The lowest y of each band, in `Band::ALL` order (the underworld's is 0).
    band_floors: [i32; 7],
    biomes: Vec<Biome>,
    /// First air cell above the ground, per world column.
    surface: Vec<i32>,
    /// Water surface per column (first air above the water), or 0 for none.
    water: Vec<i32>,
    /// 0 … 1: how mountainous a column is (overhangs, bare rock).
    rugged: Vec<f32>,
    pub islands: Vec<Island>,
    /// Vertical shafts from the surface down into the deep.
    pub chasms: Vec<Chasm>,
    /// The water table in the caverns, per `WATER_TABLE_SPAN` columns:
    /// cavern chambers below it are flooded.
    water_tables: Vec<i32>,
    pub climate: Climate,
    pub forest: Forest,
    /// Trees on the sky islands.
    pub island_forest: Forest,
    /// Crypts (castles next).
    pub structures: Structures,
}

fn unit(rng: &mut Rng) -> f64 {
    rng.next_u32() as f64 / u32::MAX as f64
}

fn range(rng: &mut Rng, (lo, hi): (f64, f64)) -> f64 {
    lo + unit(rng) * (hi - lo)
}

/// Box blur of a per-column value (prefix sums; `r` cells each side).
fn blur(v: &[f64], r: i32) -> Vec<f64> {
    let n = v.len() as i32;
    let mut prefix = Vec::with_capacity(v.len() + 1);
    prefix.push(0.0);
    for &x in v {
        prefix.push(prefix.last().unwrap() + x);
    }
    (0..n)
        .map(|i| {
            let (a, b) = ((i - r).max(0), (i + r + 1).min(n));
            (prefix[b as usize] - prefix[a as usize]) / (b - a) as f64
        })
        .collect()
}

impl WorldPlan {
    pub fn new(seed: u64, preset: Preset) -> WorldPlan {
        let s = |salt: u64| (hash(&[seed, salt]) & 0xFFFF_FFFF) as u32;
        let mut rng = Rng::seeded(&[seed, 0x9A4B]);
        let (wc, hc) = preset.chunks();
        let (width, height) = (wc * CHUNK, hc * CHUNK);
        let (sw, sh) = (width as f64 / LARGE_WIDTH, height as f64 / LARGE_HEIGHT);
        let sea_level = (height as f64 * SEA_LEVEL) as i32;
        let sea = sea_level as f64;
        let mut band_floors = [0; 7];
        for (i, top) in BAND_TOPS.iter().enumerate() {
            band_floors[i] = sea_level + (*top as f64 * sh) as i32;
        }
        let mid = width / 2;

        // Biome regions, laid out like Terraria's: a forest at the spawn, a
        // tundra towards one edge and a jungle towards the other, a desert
        // somewhere between, temperate land (forest, plains, swamp) around
        // them, oceans at both ends.
        let ocean_w = (OCEAN_WIDTH * sw) as i32;
        let mut cuts = vec![ocean_w];
        while *cuts.last().unwrap() < width - ocean_w {
            let mut x1 = (cuts.last().unwrap() + (range(&mut rng, REGION_WIDTH) * sw) as i32).min(width - ocean_w);
            if width - ocean_w - x1 < (REGION_WIDTH.0 * sw * 0.5) as i32 {
                x1 = width - ocean_w;
            }
            cuts.push(x1);
        }
        let n = cuts.len() - 1;
        let spawn = (0..n).find(|&r| (cuts[r]..cuts[r + 1]).contains(&mid)).unwrap_or(n / 2);
        let mut kinds: Vec<Option<Biome>> = vec![None; n];
        kinds[spawn] = Some(Biome::Forest);
        let cold_left = rng.coin();
        let pick = |rng: &mut Rng, lo: usize, hi: usize| if hi > lo { lo + (rng.next_u32() as usize) % (hi - lo) } else { lo };
        // The outer half of each side.
        let (left, right) = ((0, spawn.saturating_sub(1).max(1)), ((spawn + 2).min(n - 1), n));
        let (cold, warm) = if cold_left { (left, right) } else { (right, left) };
        let t = pick(&mut rng, cold.0, (cold.0 + cold.1).div_ceil(2));
        let j = pick(&mut rng, (warm.0 + warm.1) / 2, warm.1);
        for (r, b) in [(t, Biome::Tundra), (j, Biome::Jungle)] {
            if r < n && kinds[r].is_none() {
                kinds[r] = Some(b);
            }
        }
        // A mountain range on the cold side, between the tundra and the
        // spawn; a deep forest anywhere else away from the spawn. Each takes a
        // free neighbour too, so they're wide.
        let away = |r: usize| r.abs_diff(spawn) >= 2;
        let free_in = |kinds: &[Option<Biome>], lo: usize, hi: usize| (lo..hi.min(n)).filter(|&r| kinds[r].is_none() && away(r)).collect::<Vec<_>>();
        let grow = |kinds: &mut Vec<Option<Biome>>, rng: &mut Rng, choices: Vec<usize>, b: Biome| {
            if choices.is_empty() {
                return;
            }
            let r = choices[(rng.next_u32() as usize) % choices.len()];
            kinds[r] = Some(b);
            let next: Vec<usize> = [r.wrapping_sub(1), r + 1].into_iter().filter(|&q| q < n && kinds[q].is_none() && q != spawn).collect();
            if !next.is_empty() {
                kinds[next[(rng.next_u32() as usize) % next.len()]] = Some(b);
            }
        };
        let range_choices = free_in(&kinds, cold.0, cold.1);
        grow(&mut kinds, &mut rng, range_choices, Biome::Mountains);
        let forest_choices = free_in(&kinds, 0, n);
        grow(&mut kinds, &mut rng, forest_choices, Biome::DeepForest);
        // The desert: anywhere free, not next to the tundra or the range.
        let cold_ones: Vec<usize> = (0..n).filter(|&r| matches!(kinds[r], Some(Biome::Tundra | Biome::Mountains))).collect();
        let free: Vec<usize> = (0..n).filter(|&r| kinds[r].is_none() && cold_ones.iter().all(|&c| r.abs_diff(c) > 1)).collect();
        if !free.is_empty() {
            kinds[free[(rng.next_u32() as usize) % free.len()]] = Some(Biome::Desert);
        }
        let temperate = [Biome::Forest, Biome::Plains, Biome::Forest, Biome::Swamp, Biome::Plains];
        for r in 0..n {
            if kinds[r].is_none() {
                let mut b = temperate[(rng.next_u32() as usize) % temperate.len()];
                if r > 0 && kinds[r - 1] == Some(b) {
                    b = if b == Biome::Forest { Biome::Plains } else { Biome::Forest };
                }
                kinds[r] = Some(b);
            }
        }
        let mut regions: Vec<(i32, i32, Biome)> = vec![(0, ocean_w, Biome::Ocean)];
        for r in 0..n {
            regions.push((cuts[r], cuts[r + 1], kinds[r].unwrap_or(Biome::Forest)));
        }
        regions.push((width - ocean_w, width, Biome::Ocean));
        let biomes: Vec<Biome> = regions.iter().flat_map(|&(x0, x1, b)| std::iter::repeat_n(b, (x1 - x0) as usize)).collect();

        // How far into a deep forest (0 at its edge, 1 at its heart).
        let heart: Vec<f32> = {
            let mut h = vec![0.0f32; biomes.len()];
            for &(x0, x1, _) in regions.iter().filter(|r| r.2 == Biome::DeepForest) {
                // Neighbouring deep-forest regions are one forest.
                let (mut a, mut b) = (x0, x1);
                while a > 0 && biomes[a as usize - 1] == Biome::DeepForest {
                    a -= 1;
                }
                while (b as usize) < biomes.len() && biomes[b as usize] == Biome::DeepForest {
                    b += 1;
                }
                let half = (b - a) as f32 / 2.0;
                for x in x0..x1 {
                    h[x as usize] = (1.0 - ((x - a) as f32 - half).abs() / half).clamp(0.0, 1.0);
                }
            }
            h
        };
        let blend = (BLEND * sw).max(40.0) as i32;
        let per_column = |f: fn(Biome) -> f64| blur(&biomes.iter().map(|&b| f(b)).collect::<Vec<_>>(), blend);
        let (warmth, lift, hill) = (per_column(Biome::warmth), per_column(Biome::lift), per_column(Biome::hills));

        // The climate: base by height, a biome's warmth across the world.
        let column_bits = (width / CLIMATE_COLUMNS as i32).trailing_zeros();
        let mut columns = [0i8; CLIMATE_COLUMNS];
        for (i, c) in columns.iter_mut().enumerate() {
            let (a, b) = (i << column_bits, (i + 1) << column_bits);
            *c = (warmth[a..b].iter().sum::<f64>() / (b - a) as f64).round() as i8;
        }
        let climate = Climate {
            sea_level,
            surface_temp: SURFACE_TEMP,
            cells_per_degree_up: (CELLS_PER_DEGREE_UP * sh).max(1.0) as i32,
            cells_per_degree_down: (sea_level / WARMER_AT_BOTTOM).max(1),
            columns,
            column_bits,
            warm_above: band_floors[0],
            cells_per_degree_inversion: (INVERSION * sh).max(1.0) as i32,
        };

        // The land: rolling hills and cliffs on each biome's lift; oceans
        // shelve down from the beach.
        let hills = Fbm::<Perlin>::new(s(1)).set_octaves(5).set_frequency(1.0 / 900.0);
        let cliffs = Perlin::new(s(2));
        let mut surface: Vec<f64> = (0..width)
            .map(|x| {
                let xf = x as f64;
                let mut h = sea + lift[x as usize] + hills.get([xf, 0.0]) * hill[x as usize] * 2.0;
                let c = cliffs.get([xf / 140.0, 3.7]);
                if c.abs() > 0.55 {
                    h += c.signum() * (c.abs() - 0.55) * 120.0 * (hill[x as usize] / 90.0).min(1.0);
                }
                let from_edge = x.min(width - 1 - x) as f64;
                if from_edge < ocean_w as f64 {
                    let t = 1.0 - from_edge / ocean_w as f64; // 0 at the shore, 1 at the edge
                    h -= OCEAN_DEPTH * sh * smoothstep(0.0, 0.35, t) + lift[x as usize] * t;
                }
                h
            })
            .collect();

        // Big lake bowls in the gentler land (rain would fill them anyway).
        let land = |x: i32| matches!(biomes[x as usize], Biome::Plains | Biome::Forest | Biome::Swamp);
        let mut bowls: Vec<(i32, i32)> = Vec::new();
        for _ in 0..(LAKE_BOWLS * sw).round().max(1.0) as usize {
            for _try in 0..100 {
                let half = (range(&mut rng, (400.0, 800.0)) * sw.max(0.4)) as i32;
                let cx = ocean_w + half + (unit(&mut rng) * (width - 2 * (ocean_w + half)) as f64) as i32;
                let clear = (cx - mid).abs() > half + (600.0 * sw) as i32 && bowls.iter().all(|&(bx, bh)| (cx - bx).abs() > half + bh + 200);
                if !clear || !land(cx - half) || !land(cx + half) {
                    continue;
                }
                let depth = range(&mut rng, (90.0, 180.0)) * sh.max(0.4);
                for x in cx - half..=cx + half {
                    let k = 1.0 - ((x - cx) as f64 / half as f64).powi(2);
                    surface[x as usize] -= depth * k.max(0.0).powf(1.5);
                }
                bowls.push((cx, half));
                break;
            }
        }

        // Mountain massifs: a few overlapping peaks each, jagged ridges.
        let ridge = Fbm::<Perlin>::new(s(12)).set_octaves(4).set_frequency(1.0 / 420.0);
        let mut mountains = vec![0.0f64; width as usize];
        let mut placed: Vec<(i32, i32)> = Vec::new();
        let terrace = TERRACE * sh;
        // The ranges first: peak after peak across each mountain region.
        let mut k = 0u64;
        for &(x0, x1, _) in regions.iter().filter(|r| r.2 == Biome::Mountains) {
            // The range's body: a long ridge, so the saddles between peaks
            // stay high and it reads as one chain.
            let body = range(&mut rng, RANGE_BODY) * sh;
            let ramp = (900.0 * sw).max(100.0) as i32;
            let rough = Perlin::new(s(59 + k));
            for x in x0..x1 {
                let edge = smoothstep(0.0, 1.0, ((x - x0).min(x1 - x) as f64 / ramp as f64).min(1.0));
                let m = body * edge * (0.85 + 0.15 * rough.get([x as f64 / 600.0, 0.4]));
                mountains[x as usize] = mountains[x as usize].max(m);
            }
            let mut cx = x0 + (range(&mut rng, (300.0, 700.0)) * sw) as i32;
            while cx < x1 - (300.0 * sw) as i32 {
                let half = (range(&mut rng, RANGE_HALF_WIDTH) * sw) as i32;
                let peak = range(&mut rng, RANGE_PEAK) * sh;
                k += 1;
                if (cx - mid).abs() > half + (SPAWN_CLEAR * sw) as i32 {
                    massif(&mut mountains, &mut rng, &ridge, &Perlin::new(s(60 + k)), (cx, half, peak), terrace, width);
                    placed.push((cx, half));
                }
                cx += (range(&mut rng, RANGE_STEP) * sw) as i32;
            }
        }
        for _ in 0..(MOUNTAINS * sw).round().max(1.0) as usize {
            for _try in 0..200 {
                let half = (range(&mut rng, MOUNTAIN_HALF_WIDTH) * sw) as i32;
                let peak = range(&mut rng, MOUNTAIN_PEAK) * sh;
                let lo = ocean_w + half + (200.0 * sw) as i32;
                let hi = width - ocean_w - half - (200.0 * sw) as i32;
                if hi <= lo {
                    break;
                }
                let cx = lo + (unit(&mut rng) * (hi - lo) as f64) as i32;
                // Not in the deep forest or on a range (they have their own).
                let own = |x: i32| matches!(biomes[x.clamp(0, width - 1) as usize], Biome::DeepForest | Biome::Mountains);
                let clear = (cx - mid).abs() > half + (SPAWN_CLEAR * sw) as i32
                    && !own(cx - half) && !own(cx) && !own(cx + half)
                    // Ranges may run into each other (a joined range), but not stack.
                    && placed.iter().all(|&(px, ph)| (cx - px).abs() > (half + ph) * 3 / 5)
                    && bowls.iter().all(|&(bx, bh)| (cx - bx).abs() > half + bh);
                if !clear {
                    continue;
                }
                k += 1;
                massif(&mut mountains, &mut rng, &ridge, &Perlin::new(s(60 + k)), (cx, half, peak), terrace, width);
                placed.push((cx, half));
                break;
            }
        }
        let mut rugged: Vec<f32> = mountains.iter().map(|&m| (m / (350.0 * sh)).min(1.0) as f32).collect();
        // (Past the top of the peaks band the ground rises ever more slowly,
        // so no peak is sliced flat.)
        let top = band_floors[0] as f64 - 150.0 * sh;
        let soft = |h: f64| if h > top { top + (h - top) * 150.0 * sh / (h - top + 150.0 * sh) } else { h };
        let surface: Vec<i32> = surface.iter().zip(&mountains).map(|(&h, &m)| soft(h + m).max(band_floors[3] as f64) as i32).collect();

        let water = water_levels(&surface, &biomes, &rugged, &bowls, sea_level, ocean_w, sh, sw, seed);
        for (r, &w) in rugged.iter_mut().zip(&water) {
            if w > 0 {
                *r = 0.0; // no overhangs over a lake
            }
        }

        // Sky islands, high in the sky band.
        let (sky_lo, sky_hi) = (band_floors[0], height - 1);
        let islands = islands::plan(
            seed,
            (ISLANDS * sw).round().max(2.0) as usize,
            (ocean_w, width - ocean_w),
            (sky_lo + (sky_hi - sky_lo) * 9 / 20, sky_hi - (sky_hi - sky_lo) / 8),
            (90.0 * sw.max(0.5), 240.0 * sw.max(0.5)),
        );

        // Chasms: a few shafts from the lowland surface down into the deep,
        // away from the spawn, lakes and mountains, wandering as they fall.
        let mut chasms: Vec<Chasm> = Vec::new();
        for k in 0..(CHASMS * sw).round().max(2.0) as usize {
            for _try in 0..200 {
                let x = ocean_w + 400 + (unit(&mut rng) * (width - 2 * ocean_w - 800) as f64) as i32;
                let w = range(&mut rng, CHASM_WIDTH) * sw.max(0.5);
                let dry = (x - 200..x + 200).all(|x| water[x.clamp(0, width - 1) as usize] == 0 && rugged[x.clamp(0, width - 1) as usize] < 0.15);
                let clear = (x - mid).abs() > (SPAWN_CLEAR * sw) as i32 && chasms.iter().all(|c| (c.x - x).abs() > (2_000.0 * sw) as i32);
                if !dry || !clear {
                    continue;
                }
                chasms.push(Chasm {
                    x,
                    top: surface[x as usize],
                    bottom: band_floors[5] + (400.0 * sh) as i32,
                    width: w,
                    wander: 250.0 * sw.max(0.5),
                    noise: Perlin::new(s(40 + k as u64)),
                });
                break;
            }
        }
        chasms.sort_by_key(|c| c.x);
        let water_tables: Vec<i32> = (0..=width / WATER_TABLE_SPAN)
            .map(|i| {
                let (lo, hi) = (band_floors[4], band_floors[3]);
                lo + ((hi - lo) as f64 * (0.1 + 0.35 * (hash(&[seed, 0x7AB1E, i as u64]) % 1000) as f64 / 1000.0)) as i32
            })
            .collect();

        let mut list = crypts(seed, &surface, &water, &biomes, &chasms, (width, ocean_w, mid), (sw, sh), band_floors);
        list.extend(castles(seed, &surface, &biomes, width, (sw, sh)));
        let structures = Structures::new(list);

        let forest = {
            let at = |x: i32| surface[x.clamp(0, width - 1) as usize];
            let lush = blur(&biomes.iter().map(|&b| b.lushness()).collect::<Vec<_>>(), blend);
            let i = |x: i32| x.clamp(0, width - 1) as usize;
            Forest::plan(
                seed,
                width,
                at,
                // Denser toward a deep forest's heart.
                |x| lush[i(x)] + heart[i(x)] as f64 * 0.6,
                |x| {
                    let b = biomes[i(x)];
                    hash(&[seed, 0x72EE, x as u64]) % 256 < b.trees()
                        && chasms.iter().all(|c| (x - c.x).abs() as f64 > c.width * 1.5 + 80.0)
                        && !structures.near_column(x, 40)
                        && water[i(x)] <= surface[i(x)]
                        && climate.ambient(x, surface[i(x)]) >= CONIFER_LINE
                        && (at(x - 3) - at(x + 3)).abs() < 7
                        && (at(x - 12) - at(x + 12)).abs() < 20
                },
                |x| {
                    let t = climate.ambient(x, surface[i(x)]);
                    let deep = heart[i(x)];
                    if t < TREE_LINE {
                        // Pines where it's cold: the taiga, the mountainsides.
                        (Species::Conifer, t <= 0, 1.0)
                    } else if deep > 0.3 {
                        // The heart of the deep forest: old giants.
                        (Species::Elder, false, 1.0 + deep * 0.45)
                    } else {
                        (Species::Broadleaf, false, 1.0 + deep * 0.6)
                    }
                },
            )
        };
        let island_forest = {
            let island = |x: i32| islands.iter().find(|i| x >= i.x0 && x < i.x0 + i.w);
            Forest::plan(
                seed ^ 0x15F0,
                width,
                |x| island(x).and_then(|i| i.top_at(x)).unwrap_or(0),
                |_| 0.3,
                |x| {
                    island(x).is_some_and(|i| {
                        (x - i.x0 - i.w / 2).abs() < i.w / 3 && i.top_at(x).is_some_and(|top| climate.ambient(x, top) >= TREE_LINE)
                    })
                },
                |_| (Species::Broadleaf, false, 1.0),
            )
        };

        WorldPlan { seed, preset, width, height, sea_level, band_floors, biomes, surface, water, rugged, islands, chasms, water_tables, climate, forest, island_forest, structures }
    }

    /// First air cell above the ground at a world column.
    pub fn surface_at(&self, x: i32) -> i32 {
        self.surface[x.clamp(0, self.width - 1) as usize]
    }

    pub fn biome_at(&self, x: i32) -> Biome {
        self.biomes[x.clamp(0, self.width - 1) as usize]
    }

    /// The water surface over a column (first air above it), if any.
    pub fn water_at(&self, x: i32) -> Option<i32> {
        let w = self.water[x.clamp(0, self.width - 1) as usize];
        (w > 0).then_some(w)
    }

    /// 0 … 1: how mountainous a column is.
    pub fn rugged_at(&self, x: i32) -> f32 {
        self.rugged[x.clamp(0, self.width - 1) as usize]
    }

    /// The sky island over a column, if any.
    pub fn island_at(&self, x: i32) -> Option<&Island> {
        let i = self.islands.partition_point(|i| i.x0 + i.w <= x);
        self.islands.get(i).filter(|i| x >= i.x0)
    }

    /// Is a cell inside a chasm?
    pub fn chasm_at(&self, x: i32, y: i32) -> bool {
        self.chasms.iter().any(|c| c.open(x, y))
    }

    /// Cavern chambers below this height are flooded.
    pub fn water_table(&self, x: i32) -> i32 {
        self.water_tables[(x.max(0) / WATER_TABLE_SPAN) as usize % self.water_tables.len()]
    }

    /// The band a height lies in.
    pub fn band_at(&self, y: i32) -> Band {
        Band::ALL.into_iter().zip(self.band_floors).find(|&(_, floor)| y >= floor).map_or(Band::Underworld, |(b, _)| b)
    }

    /// The band's extent: (lowest y, highest y), inclusive.
    pub fn band_span(&self, band: Band) -> (i32, i32) {
        let i = Band::ALL.iter().position(|&b| b == band).expect("every band is listed");
        let top = if i == 0 { self.height - 1 } else { self.band_floors[i - 1] - 1 };
        (self.band_floors[i], top)
    }

    /// Biome regions as (x0, x1, biome), left to right.
    pub fn regions(&self) -> Vec<(i32, i32, Biome)> {
        let mut out: Vec<(i32, i32, Biome)> = Vec::new();
        for (x, &b) in self.biomes.iter().enumerate() {
            match out.last_mut() {
                Some(last) if last.2 == b => last.1 = x as i32 + 1,
                _ => out.push((x as i32, x as i32 + 1, b)),
            }
        }
        out
    }

    /// A fingerprint of everything planned: equal plans, equal worlds.
    pub fn checksum(&self) -> u64 {
        let mut h = hash(&[self.seed, self.width as u64, self.height as u64, self.sea_level as u64]);
        let fold = |v: &mut dyn Iterator<Item = u64>| v.fold(0u64, |a, x| a.wrapping_mul(31).wrapping_add(x));
        for f in self.band_floors {
            h = hash(&[h, f as u64]);
        }
        h = hash(&[h, fold(&mut self.surface.iter().map(|&v| v as u64))]);
        h = hash(&[h, fold(&mut self.water.iter().map(|&v| v as u64))]);
        h = hash(&[h, fold(&mut self.biomes.iter().map(|&b| b as u64))]);
        h = hash(&[h, fold(&mut self.climate.columns.iter().map(|&c| c as u64))]);
        for t in self.forest.near(0, self.width).into_iter().chain(self.island_forest.near(0, self.width)) {
            h = hash(&[h, t.x as u64, t.base as u64, t.height as u64]);
        }
        for i in &self.islands {
            h = hash(&[h, i.x0 as u64, i.y0 as u64, i.w as u64, i.h as u64]);
        }
        for c in &self.chasms {
            h = hash(&[h, c.x as u64, c.top as u64, c.bottom as u64, c.width.to_bits()]);
        }
        h = hash(&[h, fold(&mut self.water_tables.iter().map(|&v| v as u64))]);
        hash(&[h, self.structures.checksum()])
    }
}

/// A vertical shaft: opens as a funnel at the surface, wanders sideways,
/// narrows and widens (ledges), and ends in the deep.
pub struct Chasm {
    pub x: i32,
    pub top: i32,
    pub bottom: i32,
    /// Typical width (cells).
    pub width: f64,
    /// How far it wanders from `x` (cells).
    pub wander: f64,
    noise: Perlin,
}

impl Chasm {
    /// Centre and width at a height.
    pub fn at(&self, y: i32) -> (f64, f64) {
        let yf = y as f64;
        let centre = self.x as f64 + self.noise.get([yf / 700.0, 0.5]) * self.wander;
        let mut w = self.width * (0.55 + 0.9 * (self.noise.get([yf / 260.0, 7.3]) * 0.5 + 0.5));
        // A funnel at the top, a tapering end at the bottom.
        w *= 1.0 + 1.5 * smoothstep(self.top as f64 - 220.0, self.top as f64, yf);
        w *= smoothstep(self.bottom as f64, self.bottom as f64 + 300.0, yf).max(0.15);
        (centre, w)
    }

    pub fn open(&self, x: i32, y: i32) -> bool {
        if y > self.top + 40 || y < self.bottom || (x - self.x).abs() as f64 > self.wander + self.width * 3.0 {
            return false;
        }
        let (c, w) = self.at(y);
        (x as f64 - c).abs() < w / 2.0
    }
}

/// Crypt sites: flat, dry lowland (not the mountains, not the sea), away
/// from the spawn, the chasms and each other; a ruin on the surface over a
/// shaft down to a grid of rooms in the underground.
#[allow(clippy::too_many_arguments)]
fn crypts(seed: u64, surface: &[i32], water: &[i32], biomes: &[Biome], chasms: &[Chasm], (width, ocean_w, mid): (i32, i32, i32), (sw, sh): (f64, f64), band_floors: [i32; 7]) -> Vec<Structure> {
    let mut rng = Rng::seeded(&[seed, 0xC7497]);
    let at = |x: i32| surface[x.clamp(0, width - 1) as usize];
    let wanted = (CRYPTS * sw).round().max(2.0) as usize;
    let spacing = (CRYPT_SPACING * sw.max(0.25)) as i32;
    let (surface_lo, surface_hi) = (band_floors[2], band_floors[1]);
    let mut out: Vec<Structure> = Vec::new();
    for _try in 0..wanted * 200 {
        if out.len() == wanted {
            break;
        }
        let x = ocean_w + 300 + (unit(&mut rng) * (width - 2 * ocean_w - 600) as f64) as i32;
        let y = at(x);
        // (The ruin is 64 cells wide; its ground must be level with it.)
        let flat = (x - 32..=x + 32).step_by(4).all(|x| (at(x) - y).abs() <= 8);
        let dry = (x - 150..=x + 150).step_by(10).all(|x| water[x.clamp(0, width - 1) as usize] == 0);
        let lowland = !matches!(biomes[x as usize], Biome::Ocean | Biome::Mountains) && (surface_lo..surface_hi).contains(&y);
        let clear = (x - mid).abs() > (700.0 * sw.max(0.4)) as i32
            && chasms.iter().all(|c| (x - c.x).abs() as f64 > c.width + c.wander + 420.0)
            && out.iter().all(|s| (s.site.0 - x).abs() > spacing);
        if !(flat && dry && lowland && clear) {
            continue;
        }
        let grid = if sh < 0.5 { (3 + (rng.next_u32() % 2) as i32, 3) } else { (4 + (rng.next_u32() % 3) as i32, 3 + (rng.next_u32() % 3) as i32) };
        let depth = ((30 + (rng.next_u32() % 40) as i32) as f64 * sh.max(0.4)) as i32;
        out.push(structures::crypt(structures::crypt_rooms(), &mut rng, (x, y), grid, depth));
    }
    out.sort_by_key(|s| s.site.0);
    out
}

/// Castles high in the mountains, where it's high but not too steep (the
/// best height less three times the ground's fall under it, at most 300
/// cells): a keep between two
/// towers, a gate and a stair down the mountainside, foundations to the
/// rock where the ground falls away.
fn castles(seed: u64, surface: &[i32], biomes: &[Biome], width: i32, (sw, sh): (f64, f64)) -> Vec<Structure> {
    let mut rng = Rng::seeded(&[seed, 0xCA57]);
    let at = |x: i32| surface[x.clamp(0, width - 1) as usize];
    let small = sh < 0.5;
    let keep_w = if small { 2 } else { 3 };
    let half = (keep_w + 2) * structures::SLOT_W * structures::BLOCK / 2;
    let spacing = (2_000.0 * sw.max(0.25)) as i32;
    // (x, the ground's highest and lowest under the castle there).
    let mut sites: Vec<(i32, i32, i32)> = (0..width)
        .step_by(16)
        .filter(|&x| biomes[x as usize] == Biome::Mountains)
        .map(|x| {
            let ys = (x - half..=x + half).step_by(8).map(at);
            let (hi, lo) = ys.fold((i32::MIN, i32::MAX), |(h, l), y| (h.max(y), l.min(y)));
            (x, hi, lo)
        })
        .filter(|&(_, hi, lo)| hi - lo <= 300)
        .collect();
    sites.sort_by_key(|&(_, hi, lo)| -(hi - 3 * (hi - lo)));
    let wanted = (CASTLES * sw).round().max(1.0) as usize;
    let mut out: Vec<Structure> = Vec::new();
    for (x, hi, lo) in sites {
        if out.len() == wanted {
            break;
        }
        if out.iter().any(|s| (s.site.0 - x).abs() < spacing) {
            continue;
        }
        let keep = (keep_w, if small { 2 } else { 2 + (rng.next_u32() % 2) as i32 });
        let tower = keep.1 + 1 + (rng.next_u32() % 2) as i32;
        // The floor between the ground's highest and lowest: the rooms cut
        // into the high side, foundations hold up the low side.
        let site = (x, (hi + lo) / 2);
        out.push(structures::castle(structures::castle_rooms(), &mut rng, site, keep, tower, &at));
    }
    out
}

fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Water over each column: oceans up to sea level from the edges inward;
/// inland, local basins (rims within `LAKE_REACH` either side) filled to the
/// lower rim, levelled flat per lake and capped at the biome's lake depth;
/// no lakes in rugged notches; puddles dropped.
#[allow(clippy::too_many_arguments)]
fn water_levels(surface: &[i32], biomes: &[Biome], rugged: &[f32], bowls: &[(i32, i32)], sea_level: i32, ocean_w: i32, sh: f64, sw: f64, seed: u64) -> Vec<i32> {
    let n = surface.len();
    let mut water = vec![0; n];
    // Oceans: from each edge until the beach.
    for x in (0..ocean_w as usize).chain((n - ocean_w as usize..n).rev()) {
        if surface[x] < sea_level {
            water[x] = sea_level;
        }
    }
    let (a, b) = (ocean_w as usize, n - ocean_w as usize);
    let reach = (LAKE_REACH * sw).max(150.0) as usize;
    let left = window_max(surface, reach, false);
    let right = window_max(surface, reach, true);
    let hold: Vec<i32> = (0..n).map(|x| left[x].min(right[x])).collect();
    // Each run of columns under their rims is a lake: flat at the lowest rim
    // over it (so it's held everywhere), capped at the biome's depth.
    let mut x = a;
    while x < b {
        if hold[x] <= surface[x] {
            x += 1;
            continue;
        }
        let start = x;
        while x < b && hold[x] > surface[x] {
            x += 1;
        }
        let run = start..x;
        if run.clone().any(|i| rugged[i] > 0.2) {
            continue; // not in the notches between crags
        }
        // The big bowls always hold a lake; other hollows only sometimes
        // (a swamp is mostly pools, a forest has the odd pond).
        let bowl = bowls.iter().any(|&(bx, _)| run.contains(&(bx as usize)));
        let chance = run.clone().map(|i| biomes[i].lake_chance()).fold(0, u64::max);
        if !bowl && hash(&[seed, 0x1A4E, start as u64]) % 256 >= chance {
            continue;
        }
        let level = run.clone().map(|i| hold[i]).min().unwrap_or(0);
        let floor = run.clone().map(|i| surface[i]).min().unwrap_or(level);
        let deepest = run.clone().map(|i| biomes[i].lake_depth()).fold(0.0, f64::max);
        // Deserts are dry, but for the odd oasis.
        let depth = if deepest == 0.0 && hash(&[seed, 0x0A515, start as u64]).is_multiple_of(8) { 30.0 } else { deepest * sh.max(0.3) };
        let lvl = level.min(floor + depth as i32);
        for i in run {
            if surface[i] < lvl {
                water[i] = lvl;
            }
        }
    }
    // Drop puddles: pools shallower than 6 or narrower than 16.
    let mut x = a;
    while x < b {
        if water[x] <= surface[x] {
            x += 1;
            continue;
        }
        let start = x;
        let lvl = water[x];
        while x < b && water[x] == lvl && lvl > surface[x] {
            x += 1;
        }
        let deepest = (start..x).map(|i| lvl - surface[i]).max().unwrap_or(0);
        if deepest < 6 || x - start < 16 {
            for w in &mut water[start..x] {
                *w = 0;
            }
        }
    }
    water
}



/// Highest value within `r` columns to the left of each column (inclusive),
/// or to the right if `rightward`.
fn window_max(v: &[i32], r: usize, rightward: bool) -> Vec<i32> {
    let n = v.len();
    let mut out = vec![0; n];
    let mut dq: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let order: Box<dyn Iterator<Item = usize>> = if rightward { Box::new((0..n).rev()) } else { Box::new(0..n) };
    for i in order {
        while dq.back().is_some_and(|&j| v[j] <= v[i]) {
            dq.pop_back();
        }
        dq.push_back(i);
        while dq.front().is_some_and(|&j| j.abs_diff(i) > r) {
            dq.pop_front();
        }
        out[i] = v[*dq.front().expect("just pushed")];
    }
    out
}

/// One massif into `mountains`: lopsided, a broad shoulder under a concave
/// peak plus sub-peaks, a wandering ridge line, terraced cliff bands.
fn massif(mountains: &mut [f64], rng: &mut Rng, ridge: &Fbm<Perlin>, warp: &Perlin, (cx, half, peak): (i32, i32, f64), terrace: f64, width: i32) {
    // Lopsided: one flank longer than the other.
    let skew = range(rng, (0.65, 1.35));
    let (hl, hr) = (half as f64 * skew.min(1.0), half as f64 / skew.max(1.0));
    // Sub-peaks along the range, each a concave cone.
    let mut bumps = vec![(cx as f64, 1.0, peak)];
    for _ in 0..3 + rng.next_u32() % 4 {
        let off = (unit(rng) * 2.0 - 1.0) * 0.75;
        bumps.push((cx as f64 + off * if off < 0.0 { hl } else { hr }, range(rng, (0.3, 0.6)), peak * range(rng, (0.3, 0.75))));
    }
    for x in cx - hl as i32..=cx + hr as i32 {
        // A wandering ridge line, not a ruler-straight flank.
        let xf = x as f64 + warp.get([x as f64 / half as f64 * 2.0, 0.7]) * half as f64 * 0.12;
        let flank = |bx: f64| if xf < bx { hl } else { hr };
        let massif = {
            let u = ((xf - cx as f64) / flank(cx as f64)).abs().min(1.0);
            // A broad shoulder under a concave peak.
            peak * (0.45 * (1.0 - u).powf(2.4) + 0.55 * (1.0 - u * u).max(0.0).powf(2.0))
        };
        let mut m = bumps.iter().skip(1).map(|&(bx, bw, bp)| bp * (1.0 - ((xf - bx) / (bw * flank(bx))).abs()).max(0.0).powf(1.4)).fold(massif, f64::max);
        // Sharp crests: a ridged profile, stronger the higher it is.
        m += m * 0.07 * (1.0 - 2.0 * ridge.get([xf, 0.3]).abs());
        // Terraces: cliff bands with ledges between.
        if m > terrace {
            let f = m / terrace;
            let stepped = (f.floor() + smoothstep(0.65, 1.0, f.fract())) * terrace;
            m += (stepped - m) * 0.45;
        }
        let i = x.clamp(0, width - 1) as usize;
        mountains[i] = mountains[i].max(m);
    }
}
