//! Ambient temperature. Cells store heat *relative* to this, so a cell at
//! ambient has heat 0 and costs nothing. Colder with altitude (snowy peaks
//! stay frozen), warmer with depth, and warmer or colder across the world
//! (a hot desert, a frozen tundra).

/// Entries in the across-the-world temperature table.
pub const CLIMATE_COLUMNS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Climate {
    /// Height (cell y) where `surface_temp` holds.
    pub sea_level: i32,
    /// °C at sea level.
    pub surface_temp: i32,
    /// Cells of climb per 1 °C colder above sea level.
    pub cells_per_degree_up: i32,
    /// Cells of descent per 1 °C warmer below sea level.
    pub cells_per_degree_down: i32,
    /// °C added across the world's width, one entry per `1 << column_bits`
    /// cells (a whole number of chunks, so a chunk has one entry). Zero by
    /// default.
    pub columns: [i8; CLIMATE_COLUMNS],
    pub column_bits: u32,
    /// Above this height the air warms again (an inversion over the peaks,
    /// so the sky islands are mild), 1 °C per `cells_per_degree_inversion`,
    /// back up to `surface_temp`.
    pub warm_above: i32,
    pub cells_per_degree_inversion: i32,
}

impl Default for Climate {
    /// Same temperature everywhere (tests, flat sandbox).
    fn default() -> Self {
        Climate {
            sea_level: 0,
            surface_temp: 15,
            cells_per_degree_up: i32::MAX,
            cells_per_degree_down: i32::MAX,
            columns: [0; CLIMATE_COLUMNS],
            column_bits: 16,
            warm_above: i32::MAX,
            cells_per_degree_inversion: i32::MAX,
        }
    }
}

impl Climate {
    /// Ambient °C at a world cell.
    #[inline]
    pub fn ambient(&self, x: i32, y: i32) -> i32 {
        let column = self.columns[((x.max(0) >> self.column_bits) as usize).min(CLIMATE_COLUMNS - 1)] as i32;
        let height = if y >= self.warm_above {
            // (Back up to sea level's warmth, no further.)
            (self.surface_temp - (self.warm_above - self.sea_level) / self.cells_per_degree_up.max(1) + (y - self.warm_above) / self.cells_per_degree_inversion.max(1))
                .min(self.surface_temp)
        } else if y >= self.sea_level {
            self.surface_temp - (y - self.sea_level) / self.cells_per_degree_up.max(1)
        } else {
            self.surface_temp + (self.sea_level - y) / self.cells_per_degree_down.max(1)
        };
        height + column
    }
}
