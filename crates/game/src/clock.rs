//! The world clock (DESIGN §13): a slow simulation of the whole world, on
//! the game's own time (days, as the sky counts them: `Daylight::days`).
//!
//! Its processes run each at its own pace, catching up whole steps when time
//! jumps (a skipped hour, a long frame), each with two faces: abstract where
//! no one is (numbers), live where someone is (through the simulation). So
//! far:
//!
//! - **Moisture** (every 10 game minutes): how wet the land's living plants
//!   are, per column across the world (`Climate::wet`, 512 of them). Rain
//!   soaks it within hours; left alone it dries back over a day or two
//!   toward the land's own humidity (its biome's, the plan's `wet`), which
//!   slow dry spells move over the days (a dry week makes tinder of a
//!   forest). The rain is the weather's, anywhere (`Weather::rain_outlook`:
//!   the fronts the live clouds follow), so the abstract and the live agree.
//!   Wet plants are slow to catch fire (`MatPhys::living`).
//!
//! Saved with the world (`save.rs`: `clock`).

use bevy::prelude::*;
use platypus_sim::climate::WET_COLUMNS;
use platypus_sim::rng::hash;
use serde::{Deserialize, Serialize};

use crate::light::Daylight;
use crate::world::SimWorld;

pub struct ClockPlugin;

impl Plugin for ClockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldClock>().add_systems(PostStartup, start).add_systems(Update, run.after(crate::light::update_daylight));
    }
}

/// Game minutes between moisture steps.
const MOISTURE_STEP: f64 = 10.0 / (24.0 * 60.0);
/// Most steps a frame catches up (a skipped day is 144 moisture steps; more
/// than this and the rest wait for the next frames).
const MAX_STEPS: usize = 400;
/// Rain soaks the land toward wet at this rate (a day⁻¹: most of the way
/// in a few hours of rain)...
const SOAK: f64 = 8.0;
/// ... and it dries back toward its humidity at this rate (a day⁻¹: most
/// of the way in a day or two).
const DRY: f64 = 1.2;
/// Rain lighter than this (the outlook, 0..1) doesn't count.
const RAIN_MIN: f32 = 0.02;
/// Dry spells: the land's humidity times between these, drifting over this
/// many days (a noise across the world and the days).
const SPELL: (f64, f64) = (0.35, 1.25);
const SPELL_DAYS: f64 = 6.0;
const SPELL_WIDTH: f64 = 12_000.0;

/// The world clock's state.
#[derive(Resource, Default)]
pub struct WorldClock {
    /// Game time (days), as the sky has it.
    pub now: f64,
    /// Steps done, per process (in its own steps since day 0).
    moisture_done: i64,
    /// How wet the land is per column (0..1), and what it is left to itself.
    pub wet: Vec<f32>,
    pub humidity: Vec<f32>,
    /// Cells per wet column.
    column: i32,
    started: bool,
    /// A save's, until the clock has started.
    pending: Option<ClockFile>,
    /// Held at this wetness everywhere (a scenario's), or none.
    pub pinned: Option<f32>,
}

/// What of the clock is saved (the rest follows from the world).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ClockFile {
    pub moisture_done: i64,
    /// The land's wetness per column, 0..255.
    pub wet: Vec<u8>,
}

impl WorldClock {
    pub fn save(&self) -> ClockFile {
        ClockFile { moisture_done: self.moisture_done, wet: self.wet.iter().map(|w| (w * 255.0).round() as u8).collect() }
    }

    pub fn load(&mut self, f: &ClockFile) {
        if !self.started {
            self.pending = Some(f.clone());
            return;
        }
        if f.wet.len() == self.wet.len() {
            self.wet = f.wet.iter().map(|&w| w as f32 / 255.0).collect();
            self.moisture_done = f.moisture_done;
        }
    }

    fn column_of(&self, x: i32) -> usize {
        ((x.max(0) / self.column.max(1)) as usize).min(self.wet.len().saturating_sub(1))
    }

    /// The land over world x, in words (the dev readout).
    pub fn describe(&self, x: i32) -> String {
        let i = self.column_of(x);
        let (w, h) = (self.wet.get(i).copied().unwrap_or(0.0), self.humidity.get(i).copied().unwrap_or(0.0));
        let feel = match w {
            w if w < 0.15 => "tinder-dry",
            w if w < 0.35 => "dry",
            w if w < 0.6 => "damp",
            w if w < 0.85 => "wet",
            _ => "soaked",
        };
        format!("{feel} ({:.0} % wet, {:.0} % left to itself)", w * 100.0, h * 100.0)
    }

    /// The dry spell over a column now: its humidity is multiplied by this.
    fn spell(&self, seed: u64, i: usize, day: f64) -> f64 {
        let x = (i as f64 + 0.5) * self.column as f64 / SPELL_WIDTH;
        let t = day / SPELL_DAYS;
        let n = smooth_noise(seed, x, t);
        SPELL.0 + (SPELL.1 - SPELL.0) * n
    }
}

/// The land's own humidity, from the plan (`Climate::wet` as generated).
fn start(sim: Res<SimWorld>, mut clock: ResMut<WorldClock>) {
    let climate = sim.world.climate();
    clock.column = 1 << climate.wet_bits;
    clock.humidity = climate.wet.iter().map(|&w| w as f32 / 255.0).collect();
    if clock.wet.len() != WET_COLUMNS {
        clock.wet = clock.humidity.clone();
    }
    clock.started = true;
    if let Some(f) = clock.pending.take() {
        clock.load(&f);
    }
}

/// Each frame: the time, then every process's steps due.
fn run(day: Res<Daylight>, mut sim: ResMut<SimWorld>, mut clock: ResMut<WorldClock>) {
    if !clock.started {
        return;
    }
    clock.now = day.days;
    let due = (clock.now / MOISTURE_STEP).floor() as i64;
    // (A new world, or one from before the clock: from now.)
    if clock.moisture_done == 0 || clock.moisture_done > due {
        clock.moisture_done = due;
    }
    let steps = (due - clock.moisture_done).clamp(0, MAX_STEPS as i64);
    // (Pinned: held there, at once.)
    if let Some(p) = clock.pinned
        && clock.wet.iter().any(|&w| w != p)
    {
        clock.wet.iter_mut().for_each(|w| *w = p);
    } else if steps == 0 {
        return;
    }
    let seed = sim.world.seed();
    let tick = sim.world.tick();
    for _ in 0..steps {
        clock.moisture_done += 1;
        let t = clock.moisture_done as f64 * MOISTURE_STEP;
        moisture(&mut clock, &sim, seed, tick, t);
    }
    // Into the simulation: wet plants are slow to catch.
    let mut climate = sim.world.climate();
    for (w, &m) in climate.wet.iter_mut().zip(&clock.wet) {
        *w = (m * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    sim.world.set_climate(climate);
}

/// One moisture step at day `t`: each column soaks in the rain, or dries
/// toward its humidity (in this dry spell).
fn moisture(clock: &mut WorldClock, sim: &SimWorld, seed: u64, tick: u64, t: f64) {
    let dt = MOISTURE_STEP;
    let (soak, dry) = (1.0 - (-SOAK * dt).exp(), 1.0 - (-DRY * dt).exp());
    for i in 0..clock.wet.len() {
        let x = (i as i32 * clock.column) + clock.column / 2;
        let rain = sim.world.weather().map_or(0.0, |w| w.rain_outlook(x, tick));
        let m = clock.wet[i] as f64;
        let next = if let Some(p) = clock.pinned {
            p as f64
        } else if rain > RAIN_MIN {
            m + (1.0 - m) * soak * (0.5 + rain as f64).min(1.0)
        } else {
            let base = (clock.humidity[i] as f64 * clock.spell(seed, i, t)).clamp(0.0, 1.0);
            m + (base - m) * dry
        };
        clock.wet[i] = next.clamp(0.0, 1.0) as f32;
    }
}

/// Smooth value noise in 0..1 over (x, t).
fn smooth_noise(seed: u64, x: f64, t: f64) -> f64 {
    let (xi, ti) = (x.floor(), t.floor());
    let (fx, ft) = (x - xi, t - ti);
    let v = |i: f64, j: f64| (hash(&[seed, 0xD4E5, i as i64 as u64, j as i64 as u64]) >> 40) as f64 / (1u64 << 24) as f64;
    let s = |u: f64| u * u * (3.0 - 2.0 * u);
    let (sx, st) = (s(fx), s(ft));
    let a = v(xi, ti) + (v(xi + 1.0, ti) - v(xi, ti)) * sx;
    let b = v(xi, ti + 1.0) + (v(xi + 1.0, ti + 1.0) - v(xi, ti + 1.0)) * sx;
    a + (b - a) * st
}
