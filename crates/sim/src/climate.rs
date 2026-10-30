//! Ambient temperature. Cells store heat *relative* to this, so a cell at
//! ambient has heat 0 and costs nothing. Colder with altitude (snowy peaks
//! stay frozen), warmer with depth (gently down to the caverns' floor, so
//! their water's mild; fast through the deep under it), and warmer or
//! colder across the world (a hot desert, a frozen tundra).

use crate::CHUNK;

/// Entries in the across-the-world temperature table (a large world's is
/// 1 024 cells an entry).
pub const CLIMATE_COLUMNS: usize = 192;
/// Entries in the across-the-world wetness table (finer: rain fronts are
/// narrower than biomes).
pub const WET_COLUMNS: usize = 512;
/// Wetness where nothing's said (tests, sandboxes): dry, fire as it was
/// tuned before the land had a wetness.
pub const WET_DEFAULT: u8 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Climate {
    /// Height (cell y) where `surface_temp` holds.
    pub sea_level: i32,
    /// °C at sea level.
    pub surface_temp: i32,
    /// Cells of climb per 1 °C colder above sea level.
    pub cells_per_degree_up: i32,
    /// Cells of descent per 1 °C warmer below sea level, down to
    /// `mild_to`; then per 1 °C warmer below that (the deep heats fast).
    pub cells_per_degree_mild: i32,
    pub mild_to: i32,
    pub cells_per_degree_down: i32,
    /// °C added across the world's width, one entry per `column_width`
    /// cells (a whole number of chunks, so a chunk has one entry: see
    /// `column_width_for`). Zero by default.
    pub columns: [i8; CLIMATE_COLUMNS],
    pub column_width: i32,
    /// Above this height the air warms again (an inversion over the peaks,
    /// so the sky islands are mild), 1 °C per `cells_per_degree_inversion`,
    /// back up to `surface_temp`.
    pub warm_above: i32,
    pub cells_per_degree_inversion: i32,
    /// How wet the living plants are across the world (0 a drought, 255
    /// soaked), one entry per `1 << wet_bits` cells: set by the world clock
    /// (rain, the land's humidity, dry spells). Wet plants are slow to
    /// catch fire (`MatPhys::living`).
    pub wet: [u8; WET_COLUMNS],
    pub wet_bits: u32,
    /// Over this height gravity fades, to none at the second (and over
    /// it: the void, where there's no air either).
    pub weightless: (i32, i32),
    /// Over this height (as the air thins, over the light high islands)
    /// the cold comes on, down to `VOID_COLD` °C in the void.
    pub cold_from: i32,
}

/// °C in the void (no air, nothing to keep warm).
pub const VOID_COLD: i32 = -120;
/// Gravity (of 256) under which nothing falls: weightless.
pub const WEIGHTLESS: u32 = 43;

impl Default for Climate {
    /// Same temperature everywhere (tests, flat sandbox).
    fn default() -> Self {
        Climate {
            sea_level: 0,
            surface_temp: 15,
            cells_per_degree_up: i32::MAX,
            cells_per_degree_mild: i32::MAX,
            mild_to: i32::MIN,
            cells_per_degree_down: i32::MAX,
            columns: [0; CLIMATE_COLUMNS],
            column_width: 1 << 16,
            warm_above: i32::MAX,
            cells_per_degree_inversion: i32::MAX,
            wet: [WET_DEFAULT; WET_COLUMNS],
            wet_bits: 16,
            weightless: (i32::MAX, i32::MAX),
            cold_from: i32::MAX,
        }
    }
}

impl Climate {
    /// How wet the living plants are over world x (0..1).
    #[inline]
    pub fn wetness(&self, x: i32) -> f32 {
        self.wet[((x.max(0) >> self.wet_bits) as usize).min(WET_COLUMNS - 1)] as f32 / 255.0
    }

    /// The wetness table's bits for a world this wide (so its entries
    /// cover it).
    pub fn wet_bits_for(width: i32) -> u32 {
        ((width.max(1) as usize).div_ceil(WET_COLUMNS)).next_power_of_two().trailing_zeros()
    }

    /// The temperature table's entry width for a world this wide: whole
    /// chunks, as few as still cover the world (any width, not only powers
    /// of two).
    pub fn column_width_for(width: i32) -> i32 {
        (width.max(1) as usize).div_ceil(CLIMATE_COLUMNS).next_multiple_of(CHUNK as usize) as i32
    }

    /// How far up through the weightless band a height is (0 … 256: the
    /// void), eased in and out.
    #[inline]
    fn into_void(&self, y: i32) -> i32 {
        let (a, b) = self.weightless;
        if y <= a {
            return 0;
        }
        if y >= b {
            return 256;
        }
        let t = ((y - a) as i64 * 256 / (b - a).max(1) as i64) as i32;
        t * t * (768 - 2 * t) / 65_536
    }

    /// How much gravity there is at a height, of 256 (none in the void).
    #[inline]
    pub fn gravity(&self, y: i32) -> u32 {
        (256 - self.into_void(y)) as u32
    }

    /// The same as a share (1: the ground's … 0).
    #[inline]
    pub fn gravity_f(&self, y: f32) -> f32 {
        let (a, b) = (self.weightless.0 as f32, self.weightless.1 as f32);
        let t = ((y - a) / (b - a).max(1.0)).clamp(0.0, 1.0);
        1.0 - t * t * (3.0 - 2.0 * t)
    }

    /// No air here (the void): nothing burns.
    #[inline]
    pub fn airless(&self, y: i32) -> bool {
        y >= self.weightless.1
    }

    /// Ambient °C at a world cell.
    #[inline]
    pub fn ambient(&self, x: i32, y: i32) -> i32 {
        let air = self.air_temp(x, y);
        let (a, b) = (self.cold_from.min(self.weightless.1), self.weightless.1);
        if y <= a {
            return air;
        }
        let t = if y >= b { 256 } else { ((y - a) as i64 * 256 / (b - a).max(1) as i64) as i32 };
        air + (VOID_COLD - air) * (t * t * (768 - 2 * t) / 65_536) / 256
    }

    /// Ambient °C at a world cell, as the air has it (the void's cold
    /// aside).
    #[inline]
    fn air_temp(&self, x: i32, y: i32) -> i32 {
        let column = self.columns[((x.max(0) / self.column_width.max(1)) as usize).min(CLIMATE_COLUMNS - 1)] as i32;
        let height = if y >= self.warm_above {
            // (Back up to sea level's warmth, no further.)
            (self.surface_temp - (self.warm_above - self.sea_level) / self.cells_per_degree_up.max(1) + (y - self.warm_above) / self.cells_per_degree_inversion.max(1))
                .min(self.surface_temp)
        } else if y >= self.sea_level {
            self.surface_temp - (y - self.sea_level) / self.cells_per_degree_up.max(1)
        } else if y >= self.mild_to {
            self.surface_temp + (self.sea_level - y) / self.cells_per_degree_mild.max(1)
        } else {
            self.surface_temp + (self.sea_level - self.mild_to) / self.cells_per_degree_mild.max(1) + (self.mild_to - y) / self.cells_per_degree_down.max(1)
        };
        height + column
    }
}
