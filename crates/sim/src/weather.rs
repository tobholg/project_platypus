//! Weather (SPEC §3.13): clouds as a coarse moisture field over the whole
//! world width, in a band of sky above the surface. Not cells: a sky full of
//! drifting gas cells would keep every chunk up there awake. The field meets
//! the cell world at its edges: rain and snow fall out of it (as particles
//! that douse fires and land as water or snow), and vapour rising from boiled
//! water feeds it.
//!
//! Stepped over the whole width regardless of what is loaded, from the seed
//! and tick only, so co-op peers agree. Only `+ − × ÷` on `f32` (no trig).

use crate::rng::hash;

/// Cells per weather texel, each way.
pub const TEXEL: i32 = 4;
/// The field steps every this many ticks.
pub const STEP_EVERY: u64 = 4;
/// Moisture above which a texel shows as cloud.
pub const CLOUD_AT: f32 = 0.35;
/// Moisture above which it rains (or snows) out.
pub const RAIN_AT: f32 = 0.9;
/// Cloud drift at full wind, cells per tick (~3 cells/s).
const DRIFT: f32 = 0.05;
/// A forced storm or clear sky fades back to the natural weather by this
/// factor per step (half-life ~2.5 minutes).
const BIAS_FADE: f32 = 0.9997;
/// Share of the gap to the target closed per step: fronts build and clear
/// over about a minute.
const RELAX: f32 = 0.006;
/// Rain out of a column per step above which it's a thunderstorm.
pub const STORM_RAIN: f32 = 0.06;
/// Moisture drained per step from a texel above `RAIN_AT`, per unit over.
const RAIN_OUT: f32 = 0.03;

pub struct Weather {
    seed: u64,
    /// Width of the world in cells (the field wraps around it).
    width: i32,
    /// Bottom of the band (cell y).
    pub y0: i32,
    pub cols: usize,
    pub rows: usize,
    /// `rows * cols`, row 0 at the bottom.
    moisture: Vec<f32>,
    /// How far the air has moved in total (cells, wraps at `width`): fronts
    /// are pinned to the moving air, not to the ground.
    offset: f32,
    /// Moisture added since the last step, per column (vapour from below).
    fed: Vec<f32>,
    /// Rain intensity per column after the last step (moisture rained out).
    rain: Vec<f32>,
    /// The cloud each column relaxes toward. Changes over minutes, so a
    /// sixteenth of the columns is recomputed per step; it moves with the air.
    shapes: Vec<Shape>,
    /// Forced weather per column (dev tools, spells): +1 storm, -1 clear.
    /// Moves with the air and fades back to 0.
    bias: Vec<f32>,
}

/// Columns whose shape is recomputed per step: all of them every this many steps.
const SHAPE_REFRESH: u64 = 16;

/// A drop or flake leaving a cloud this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Precipitation {
    /// Column (cells) and cloud base (cell y).
    pub x: i32,
    pub y: i32,
    /// Moisture rained out of this column this step.
    pub amount: f32,
}

impl Weather {
    /// A band `height` cells tall starting at `y0`, over a world `width` cells wide.
    pub fn new(seed: u64, width: i32, y0: i32, height: i32) -> Weather {
        let cols = (width / TEXEL).max(1) as usize;
        let rows = (height / TEXEL).max(1) as usize;
        let mut w = Weather {
            seed,
            width: cols as i32 * TEXEL,
            y0,
            cols,
            rows,
            moisture: vec![0.0; rows * cols],
            offset: 0.0,
            fed: vec![0.0; cols],
            rain: vec![0.0; cols],
            shapes: Vec::new(),
            bias: vec![0.0; cols],
        };
        w.shapes = (0..cols).map(|c| w.shape(c, 0)).collect();
        // Start in the weather the seed has for tick 0, not a clear sky.
        for c in 0..cols {
            for r in 0..rows {
                w.moisture[r * cols + c] = w.shapes[c].target(r);
            }
        }
        w
    }

    /// Width the field wraps at (cells).
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Top of the band (cell y, exclusive).
    pub fn y1(&self) -> i32 {
        self.y0 + self.rows as i32 * TEXEL
    }

    /// Column index for world x (wrapping).
    #[inline]
    fn col(&self, x: i32) -> usize {
        (x.rem_euclid(self.width) / TEXEL) as usize
    }

    /// Moisture at a world cell, bilinearly smoothed (0 outside the band).
    pub fn moisture_at(&self, x: f32, y: f32) -> f32 {
        let fx = x / TEXEL as f32 - 0.5;
        let fy = (y - self.y0 as f32) / TEXEL as f32 - 0.5;
        if fy < -0.5 || fy > self.rows as f32 - 0.5 {
            return 0.0;
        }
        let (c0, r0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - c0, fy - r0);
        let get = |c: f32, r: f32| {
            let r = (r as i32).clamp(0, self.rows as i32 - 1) as usize;
            let c = (c as i32).rem_euclid(self.cols as i32) as usize;
            self.moisture[r * self.cols + c]
        };
        let a = get(c0, r0) + (get(c0 + 1.0, r0) - get(c0, r0)) * tx;
        let b = get(c0, r0 + 1.0) + (get(c0 + 1.0, r0 + 1.0) - get(c0, r0 + 1.0)) * tx;
        a + (b - a) * ty
    }

    /// Bottom of the cloud over world x (cell y), if there is one.
    pub fn cloud_base(&self, x: i32) -> Option<i32> {
        let c = self.col(x);
        (0..self.rows).find(|&r| self.moisture[r * self.cols + c] > CLOUD_AT).map(|r| self.y0 + r as i32 * TEXEL)
    }

    /// How hard it is raining out of the column over world x (0 = dry).
    pub fn rain_at(&self, x: i32) -> f32 {
        self.rain[self.col(x)]
    }

    /// Average moisture over a span of columns: how overcast the sky is.
    pub fn overcast(&self, x0: i32, x1: i32) -> f32 {
        let (mut sum, mut n) = (0.0, 0);
        for x in (x0..=x1).step_by(TEXEL as usize) {
            let c = self.col(x);
            for r in 0..self.rows {
                sum += self.moisture[r * self.cols + c];
            }
            n += self.rows;
        }
        if n == 0 { 0.0 } else { sum / n as f32 }
    }

    /// How far the air has moved (cells, wrapping at `width`). Clouds are
    /// drawn in air coordinates (`world x - offset`) and placed with this, so
    /// they slide smoothly although the field itself moves in whole texels.
    pub fn offset(&self) -> f32 {
        self.offset
    }

    /// World x the field's texels currently sit at for air coordinate `a`.
    #[inline]
    fn air_to_field(&self, a: f32) -> f32 {
        a + (self.offset / TEXEL as f32).floor() * TEXEL as f32
    }

    /// Moisture at air coordinate `a` (see `offset`), height `y`.
    pub fn moisture_air(&self, a: f32, y: f32) -> f32 {
        self.moisture_at(self.air_to_field(a), y)
    }

    /// Rain out of the column at air coordinate `a`.
    pub fn rain_air(&self, a: i32) -> f32 {
        self.rain_at(self.air_to_field(a as f32) as i32)
    }

    /// Force a storm (`storm`) or clear sky over world x ± `radius`, starting
    /// now and fading back over minutes.
    pub fn force(&mut self, x: i32, radius: i32, storm: bool, tick: u64) {
        let (cols, rows) = (self.cols, self.rows);
        let r = (radius / TEXEL).max(1);
        for d in -r..=r {
            let c = (self.col(x) as i32 + d).rem_euclid(cols as i32) as usize;
            // Full strength in the middle, tapering to the edges.
            let k = 1.0 - (d.abs() as f32 / r as f32).powi(2);
            let b = if storm { k } else { -k * 2.0 };
            if storm { self.bias[c] = self.bias[c].max(b) } else { self.bias[c] = self.bias[c].min(b) }
            self.shapes[c] = self.shape(c, tick);
            // Now, not in a minute: it's a tool.
            for row in 0..rows {
                let m = &mut self.moisture[row * cols + c];
                *m = if storm { m.max(self.shapes[c].target(row)) } else { m.min(self.shapes[c].target(row)) };
            }
        }
    }

    /// Vapour rising into the sky over world x (a cell of steam ≈ 0.02).
    pub fn feed(&mut self, x: i32, amount: f32) {
        let c = self.col(x);
        self.fed[c] += amount;
    }

    /// Advance one tick. Every `STEP_EVERY` ticks the field moves with the
    /// wind, relaxes toward the weather for this time, takes in fed vapour and
    /// rains out; returns what leaves it this tick.
    pub fn step(&mut self, tick: u64, wind: f32) -> Vec<Precipitation> {
        if !tick.is_multiple_of(STEP_EVERY) {
            return Vec::new();
        }
        let (cols, rows) = (self.cols, self.rows);
        // Drift with the wind in whole texels (no blurring); the fraction is
        // carried in `offset` and used by the renderer.
        let before = (self.offset / TEXEL as f32).floor() as i64;
        self.offset = (self.offset + wind * DRIFT * STEP_EVERY as f32).rem_euclid(self.width as f32);
        let after = (self.offset / TEXEL as f32).floor() as i64;
        let shift = after - before;
        if shift != 0 {
            let shift = shift.rem_euclid(cols as i64) as usize;
            for r in 0..rows {
                self.moisture[r * cols..(r + 1) * cols].rotate_right(shift);
            }
            self.shapes.rotate_right(shift);
            self.bias.rotate_right(shift);
        }
        for b in &mut self.bias {
            *b *= BIAS_FADE;
        }
        let phase = (tick / STEP_EVERY) % SHAPE_REFRESH;
        for c in (phase as usize..cols).step_by(SHAPE_REFRESH as usize) {
            self.shapes[c] = self.shape(c, tick);
        }

        let mut out = Vec::new();
        for c in 0..cols {
            // Vapour rising from below spreads through the lower half of the
            // band (all into one row, it piled up as a thin bright bar).
            let fed = std::mem::take(&mut self.fed[c]);
            if fed > 0.0 {
                let n = (rows / 2).max(1);
                for r in 0..n {
                    self.moisture[r * cols + c] += fed / n as f32;
                }
            }
            let mut rained = 0.0;
            let mut base = None;
            let shape = self.shapes[c];
            for r in 0..rows {
                let t = shape.target(r);
                let m = &mut self.moisture[r * cols + c];
                *m += (t - *m) * RELAX;
                if *m > RAIN_AT {
                    let d = (*m - RAIN_AT) * RAIN_OUT;
                    *m -= d;
                    rained += d;
                }
                if base.is_none() && *m > CLOUD_AT {
                    base = Some(r);
                }
            }
            self.rain[c] = rained;
            if rained > 0.0 {
                let r = base.unwrap_or(0) as i32;
                out.push(Precipitation { x: c as i32 * TEXEL, y: self.y0 + r * TEXEL, amount: rained });
            }
        }
        out
    }

    /// The cloud the weather wants over column `c` now. Humid fronts (pinned
    /// to the moving air) over a mostly clear sky; within them cumulus: flat
    /// bases, heaped tops of varying height, the thickest ones raining.
    fn shape(&self, c: usize, tick: u64) -> Shape {
        // Air coordinate of this column (texels move in whole steps).
        let x = c as f32 * TEXEL as f32 - (self.offset / TEXEL as f32).floor() * TEXEL as f32;
        let t = tick as f32;
        // Fronts come and go over tens of minutes; a storm lasts minutes.
        let broad = noise2(self.seed, 0xC10D, x / 1_600.0, t / 90_000.0);
        let medium = noise2(self.seed, 0xC10E, x / 260.0, t / 30_000.0);
        let bias = self.bias[c];
        let humid = (smoothstep(0.45, 0.8, 0.72 * broad + 0.28 * medium) + bias).clamp(0.0, 1.0);
        let heap = noise2(self.seed, 0xC10F, x / 70.0, t / 15_000.0);
        let rows = self.rows as f32;
        let base = rows * 0.12 + 3.0 * noise2(self.seed, 0xC110, x / 150.0, t / 20_000.0);
        // Separate heaps with clear sky between, more of them joined up the
        // more humid it is.
        let heaped = smoothstep(0.55 - 0.35 * humid, 0.85 - 0.25 * humid, heap).max(bias);
        let thickness = rows * (humid * (0.25 + 0.65 * heaped) * heaped).min(0.7);
        // Storms: the wettest parts of big fronts rain hard (and throw lightning).
        let storm = (smoothstep(0.62, 0.8, broad) * humid).max(bias);
        Shape { base, thickness, peak: 0.45 + 0.75 * humid + 0.9 * storm }
    }
}

/// One column's cloud, in rows of the band.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shape {
    base: f32,
    thickness: f32,
    /// Moisture at its heart; above `RAIN_AT` it rains.
    peak: f32,
}

impl Shape {
    fn target(&self, r: usize) -> f32 {
        if self.thickness < 1.0 {
            return 0.0;
        }
        let v = (r as f32 + 0.5 - self.base) / self.thickness;
        if !(0.0..=1.0).contains(&v) {
            return 0.0;
        }
        // Full through the middle, thinning at the base and (more) the top.
        let edge = (2.0 * v - 1.0).abs();
        self.peak * (1.0 - edge * edge * edge)
    }
}

#[inline]
fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Seeded 2D value noise in 0..1, smooth, no trig.
fn noise2(seed: u64, salt: u64, x: f32, y: f32) -> f32 {
    let (xi, yi) = (x.floor(), y.floor());
    let (fx, fy) = (x - xi, y - yi);
    let v = |i: f32, j: f32| (hash(&[seed, salt, i as i64 as u64, j as i64 as u64]) >> 40) as f32 / (1u64 << 24) as f32;
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(fx), s(fy));
    let a = v(xi, yi) + (v(xi + 1.0, yi) - v(xi, yi)) * sx;
    let b = v(xi, yi + 1.0) + (v(xi + 1.0, yi + 1.0) - v(xi, yi + 1.0)) * sx;
    a + (b - a) * sy
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weather() -> Weather {
        Weather::new(7, 16_384, 3_000, 160)
    }

    #[test]
    fn there_are_clouds_and_clear_sky() {
        let w = weather();
        let cloudy = (0..w.cols).filter(|&c| (0..w.rows).any(|r| w.moisture[r * w.cols + c] > CLOUD_AT)).count();
        let share = cloudy as f32 / w.cols as f32;
        assert!((0.1..0.8).contains(&share), "some sky is cloudy, some clear: {share}");
    }

    #[test]
    fn clouds_drift_with_the_wind() {
        let mut w = weather();
        let before = w.moisture.clone();
        for t in 1..=400 {
            w.step(t, 1.0);
        }
        // 400 ticks at full wind: 20 cells = 5 texels to the right.
        assert!((w.offset() - 20.0).abs() < 0.01, "air moved {}", w.offset());
        let (cols, row) = (w.cols, w.rows / 2);
        let moved = (0..cols).filter(|&c| (w.moisture[row * cols + (c + 5) % cols] - before[row * cols + c]).abs() < 0.2).count();
        assert!(moved as f32 > cols as f32 * 0.9, "the pattern moved 5 texels right ({moved} of {cols} match)");
    }

    #[test]
    fn heavy_clouds_rain_and_vapour_feeds_them() {
        let mut w = weather();
        let mut rained = 0.0;
        for t in 1..=4_000 {
            rained += w.step(t, 0.3).iter().map(|p| p.amount).sum::<f32>();
        }
        assert!(rained > 1.0, "somewhere it rained ({rained})");
        let c = (0..w.cols).find(|&c| (0..w.rows).all(|r| w.moisture[r * w.cols + c] < 0.05)).expect("a clear column");
        let x = c as i32 * TEXEL;
        let step = |w: &mut Weather| w.step(4_004, 0.0);
        w.feed(x, 5.0);
        step(&mut w);
        assert!(w.moisture[c] > 0.1 && w.moisture[(w.rows / 3) * w.cols + c] > 0.1, "vapour spread into the lower band");
    }

    #[test]
    fn a_forced_storm_rains_hard_and_a_forced_clear_sky_is_clear() {
        let mut w = weather();
        let x = 8_000;
        w.force(x, 200, true, 0);
        let mut storm = 0.0f32;
        for t in 1..=400 {
            let out = w.step(t, 0.0);
            storm = storm.max(out.iter().filter(|p| (p.x - x).abs() < 40).map(|p| p.amount).fold(0.0, f32::max));
        }
        assert!(storm > STORM_RAIN, "a thunderstorm ({storm})");
        w.force(x, 200, false, 400);
        let c = w.col(x);
        assert!((0..w.rows).all(|r| w.moisture[r * w.cols + c] < CLOUD_AT), "clear at once");
        for t in 401..=2_000 {
            w.step(t, 0.0);
        }
        assert!((0..w.rows).all(|r| w.moisture[r * w.cols + c] < CLOUD_AT), "and stays clear for a while");
    }

    #[test]
    fn deterministic() {
        let run = || {
            let mut w = weather();
            let mut drops = Vec::new();
            for t in 1..=2_000 {
                drops.extend(w.step(t, (t as f32 * 0.001).min(1.0)));
            }
            (drops, w.moisture)
        };
        assert_eq!(run(), run());
    }
}

