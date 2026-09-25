//! Ambient temperature. Cells store heat *relative* to this, so a cell at
//! ambient has heat 0 and costs nothing. Colder with altitude (snowy peaks
//! stay frozen), warmer with depth.

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
}

impl Default for Climate {
    /// Same temperature everywhere (tests, flat sandbox).
    fn default() -> Self {
        Climate { sea_level: 0, surface_temp: 15, cells_per_degree_up: i32::MAX, cells_per_degree_down: i32::MAX }
    }
}

impl Climate {
    /// Ambient °C at world height `y`.
    #[inline]
    pub fn ambient(&self, y: i32) -> i32 {
        if y >= self.sea_level {
            self.surface_temp - (y - self.sea_level) / self.cells_per_degree_up.max(1)
        } else {
            self.surface_temp + (self.sea_level - y) / self.cells_per_degree_down.max(1)
        }
    }
}
