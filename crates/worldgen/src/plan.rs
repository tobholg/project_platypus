//! The world plan (DESIGN §3.1): what is decided once from the seed, before
//! any chunk: the world's size, sea level and vertical bands, the surface,
//! the forests. Chunks rasterise from it and never read each other, so any
//! chunk can be generated at any time, on any thread, by any co-op peer.

use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use platypus_sim::rng::hash;
use platypus_sim::{CHUNK, Climate};

use crate::flora::Forest;

/// World sizes. `Large` is the world we play in; `Small` is quick to look at
/// and to test with (same bands, scaled to its height).
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
/// sea level, in a world `LARGE_HEIGHT` tall; other heights scale them.
const BAND_TOPS: [i32; 6] = [2_500, 800, -200, -2_500, -7_000, -11_000];
const LARGE_HEIGHT: f64 = 16_384.0;
/// Sea level, as a share of the world's height (a quarter from the top).
const SEA_LEVEL: f64 = 0.75;
/// Amplitude of the rolling hills (cells).
const HILLS: f64 = 90.0;
/// Snow lies this far above sea level (and the climate agrees: 0 °C there).
const SNOW_ABOVE_SEA: i32 = 81;
/// °C at sea level.
const SURFACE_TEMP: i32 = 15;
/// °C warmer at the bottom of the world than at sea level.
const WARMER_AT_BOTTOM: i32 = 85;

pub struct WorldPlan {
    pub seed: u64,
    pub preset: Preset,
    /// Size in cells.
    pub width: i32,
    pub height: i32,
    pub sea_level: i32,
    /// The lowest y of each band, in `Band::ALL` order (the underworld's is 0).
    band_floors: [i32; 7],
    /// First air cell above the ground, per world column.
    surface: Vec<i32>,
    pub snow_line: i32,
    pub climate: Climate,
    pub forest: Forest,
}

impl WorldPlan {
    pub fn new(seed: u64, preset: Preset) -> WorldPlan {
        let s = |salt: u64| (hash(&[seed, salt]) & 0xFFFF_FFFF) as u32;
        let (wc, hc) = preset.chunks();
        let (width, height) = (wc * CHUNK, hc * CHUNK);
        let sea_level = (height as f64 * SEA_LEVEL) as i32;
        let scale = height as f64 / LARGE_HEIGHT;
        let mut band_floors = [0; 7];
        for (i, top) in BAND_TOPS.iter().enumerate() {
            band_floors[i] = sea_level + (*top as f64 * scale) as i32;
        }

        // Rolling hills with occasional sharp steps (legacy "cliffs").
        let hills = Fbm::<Perlin>::new(s(1)).set_octaves(5).set_frequency(1.0 / 900.0);
        let cliffs = Perlin::new(s(2));
        let surface: Vec<i32> = (0..width)
            .map(|x| {
                let xf = x as f64;
                let mut h = sea_level as f64 + hills.get([xf, 0.0]) * HILLS * 2.0;
                let c = cliffs.get([xf / 140.0, 3.7]);
                if c.abs() > 0.55 {
                    h += c.signum() * (c.abs() - 0.55) * 120.0;
                }
                h.clamp(sea_level as f64 - height as f64 * 0.2, sea_level as f64 + height as f64 * 0.2) as i32
            })
            .collect();

        let snow_line = sea_level + SNOW_ABOVE_SEA;
        let forest = {
            let at = |x: i32| surface[x.clamp(0, width - 1) as usize];
            // Not on snow, not on a cliff edge.
            Forest::plan(seed, width, at, |x| at(x) <= snow_line && (at(x - 3) - at(x + 3)).abs() < 7)
        };
        let climate = Climate {
            sea_level,
            surface_temp: SURFACE_TEMP,
            // 0 °C at the snow line.
            cells_per_degree_up: SNOW_ABOVE_SEA / SURFACE_TEMP,
            cells_per_degree_down: (sea_level / WARMER_AT_BOTTOM).max(1),
        };
        WorldPlan { seed, preset, width, height, sea_level, band_floors, surface, snow_line, climate, forest }
    }

    /// First air cell above the ground at a world column.
    pub fn surface_at(&self, x: i32) -> i32 {
        self.surface[x.clamp(0, self.width - 1) as usize]
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

    /// A fingerprint of everything planned: equal plans, equal worlds.
    pub fn checksum(&self) -> u64 {
        let mut h = hash(&[self.seed, self.width as u64, self.height as u64, self.sea_level as u64, self.snow_line as u64]);
        for f in self.band_floors {
            h = hash(&[h, f as u64]);
        }
        for s in self.surface.chunks(64) {
            h = hash(&[h, s.iter().fold(0u64, |a, &v| a.wrapping_mul(31).wrapping_add(v as u64))]);
        }
        for t in self.forest.near(0, self.width) {
            h = hash(&[h, t.x as u64, t.base as u64, t.height as u64]);
        }
        h
    }
}
