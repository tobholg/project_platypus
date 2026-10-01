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
//! - **Regrowth** (hourly, and as chunks load): nature heals toward what the
//!   seed made (`ChunkGenerator::heal`). Burnt grass greens over about a
//!   day, ash and charcoal are gone in two; a tree burnt or felled (mostly
//!   gone) is a sapling a day later and full grown by day five: a new tree
//!   of its kind, where it stood. The clock keeps what it needs (`trees`:
//!   when each was lost; `land`: since when each chunk has been healing);
//!   the cells change where no one is looking: out of view, or on load
//!   (catching up however long it was away). What was built stays.
//! - **Wildfires** (hourly): lightning in dry forest where no one is. The
//!   trees it takes are lost (they regrow as any), the land under it is
//!   scorched (`Healing::scorched`: grass burnt, ash lying, healing as any
//!   burn from the fire's day): you find the scar, and it heals.
//! - **Lairs refill**: a lair's keepers (`Spawn::Keeper`) come back three
//!   days after the clock last saw any of them about, out of sight.
//!
//! Saved with the world (`save.rs`: `clock`).

use std::collections::{BTreeMap, HashMap};

use bevy::prelude::*;
use platypus_sim::climate::WET_COLUMNS;
use platypus_sim::rng::hash;
use platypus_sim::{CHUNK, ChunkPos};
use platypus_worldgen::{Healed, Healing, SAPLING};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::creatures::Health;
use crate::light::Daylight;
use crate::world::{ChunkLoader, LoadedChunks, SimWorld};

pub struct ClockPlugin;

impl Plugin for ClockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldClock>()
            .add_systems(PostStartup, start)
            .add_systems(Update, (run, wildfires, regrow, lairs).chain().in_set(ClockSet).after(crate::light::update_daylight));
    }
}

/// The clock's systems (what follows the clock runs after them: `events.rs`).
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClockSet;

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
/// A lost tree is a sapling this many days later...
const SAPLING_DAYS: f64 = 1.0;
/// ... and full grown this many after that.
const GROW_DAYS: f64 = 4.0;
/// Chunks healed a frame, at most (the hourly pass spreads over frames:
/// each is about 0.2 ms, making the chunk again to compare).
const HEALS_PER_FRAME: usize = 4;
/// Trees reach at most this far either side of their x (cells, a giant's
/// crown and its lean): chunks this near a lost tree heal.
const TREE_REACH: i32 = 2 * platypus_worldgen::flora::TREE_REACH;
/// A chunk away this long (days: an hour) comes back with its fires out.
const COOL_DAYS: f64 = 1.0 / 24.0;
/// Wildfires: the chance an hour that lightning lights a column this wide
/// (cells) of tinder-dry land; land wetter than `FIRE_DRY` doesn't catch
/// (the chance falls off to it).
const FIRE_RATE: f64 = 0.0004;
const FIRE_WIDTH: f64 = 256.0;
const FIRE_DRY: f32 = 0.3;
/// How far a wildfire runs either side of its strike (cells), from damp to
/// tinder-dry; a gap this wide between trees stops it.
const FIRE_REACH: (f32, f32) = (80.0, 600.0);
const FIRE_GAP: i32 = 80;
/// Where players are and this far round them (cells), no wildfires: those
/// are the live simulation's.
const FIRE_AWAY: f32 = 2_000.0;
/// A wildfire's scar is kept this long (days; it's healed by then).
const SCAR_DAYS: f64 = 5.0;
/// A lair's keeper gone comes back this many days after it was last seen
/// alive.
const REFILL_DAYS: f64 = 3.0;

/// The world clock's state.
#[derive(Resource, Default)]
pub struct WorldClock {
    /// Game time (days), as the sky has it.
    pub now: f64,
    /// The events (`events.rs`): what's happened and what's coming, and the
    /// last whole day their timetable has been rolled for.
    pub events: Vec<crate::events::Happening>,
    pub events_rolled: i64,
    /// Steps done, per process (in its own steps since day 0).
    moisture_done: i64,
    /// How wet the land is per column (0..1), and what it is left to itself.
    pub wet: Vec<f32>,
    pub humidity: Vec<f32>,
    /// Cells per wet column.
    column: i32,
    pub started: bool,
    /// A save's, until the clock has started.
    pending: Option<ClockFile>,
    /// Held at this wetness everywhere (a scenario's), or none.
    pub pinned: Option<f32>,
    /// Trees lost, by x: regrowing (or regrown).
    pub trees: BTreeMap<i32, TreeRecord>,
    /// Chunks with land still to heal.
    pub land: HashMap<(i32, i32), LandRecord>,
    /// The hour last healed, and the chunks still to heal for it (and
    /// whether to judge their trees: the hour before was healed too).
    hour: i64,
    queue: Vec<ChunkPos>,
    judge: bool,
    /// Wildfires: hours done, and the scars still healing.
    fires_done: i64,
    pub fires: Vec<Wildfire>,
    /// Lairs' keepers, by where they were put.
    pub keepers: HashMap<(i32, i32), Keeper>,
    lair_hour: i64,
    /// When each stored chunk was put away (this session: one missing has
    /// been away since before, long).
    away: HashMap<(i32, i32), f64>,
}

/// A tree lost (burnt, felled), and the time it's growing back: each time
/// a new one of its kind (its look from the time).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TreeRecord {
    pub lost: f64,
    pub time: u32,
}

/// A wildfire's scar: across these x, from this day.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Wildfire {
    pub x0: i32,
    pub x1: i32,
    pub at: f64,
}

/// On a lair's keeper: where it was put (its record in `WorldClock::keepers`;
/// it comes back there). On a star's guardian (`events.rs`): the crater it
/// keeps (no record: it doesn't come back), its leash's end.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Keeps(pub (i32, i32));

/// A lair's keeper: what it is, and when it was last seen alive.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Keeper {
    pub kind: String,
    pub seen: f64,
}

/// A chunk's land healing: since when, and how many cells were left then.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct LandRecord {
    pub since: f64,
    pub unhealed: u32,
}

/// What of the clock is saved (the rest follows from the world).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ClockFile {
    pub moisture_done: i64,
    /// The land's wetness per column, 0..255.
    pub wet: Vec<u8>,
    #[serde(default)]
    pub trees: Vec<(i32, TreeRecord)>,
    #[serde(default)]
    pub land: Vec<((i32, i32), LandRecord)>,
    #[serde(default)]
    pub fires_done: i64,
    #[serde(default)]
    pub fires: Vec<Wildfire>,
    #[serde(default)]
    pub keepers: Vec<((i32, i32), Keeper)>,
    /// The events' (`events.rs`): what's happened and is coming, and the
    /// last day rolled.
    #[serde(default)]
    pub events: Vec<crate::events::Happening>,
    #[serde(default)]
    pub events_rolled: i64,
}

impl WorldClock {
    pub fn save(&self) -> ClockFile {
        ClockFile {
            moisture_done: self.moisture_done,
            wet: self.wet.iter().map(|w| (w * 255.0).round() as u8).collect(),
            trees: self.trees.iter().map(|(&x, &r)| (x, r)).collect(),
            land: self.land.iter().map(|(&p, &r)| (p, r)).collect(),
            fires_done: self.fires_done,
            fires: self.fires.clone(),
            keepers: self.keepers.iter().map(|(&p, k)| (p, k.clone())).collect(),
            events: self.events.clone(),
            events_rolled: self.events_rolled,
        }
    }

    /// Lightning in dry forest at x (dryness 0..1): the trees it takes
    /// (walking out from x while they stand close) are lost, the land under
    /// them scorched. How many trees it took.
    pub fn wildfire(&mut self, world: &dyn platypus_worldgen::ChunkGenerator, x: i32, dryness: f32) -> usize {
        let reach = (FIRE_REACH.0 + (FIRE_REACH.1 - FIRE_REACH.0) * dryness.clamp(0.0, 1.0)) as i32;
        let trees = world.trees_between(x - reach, x + reach);
        let Some(start) = trees.iter().position(|&t| t >= x).or(trees.len().checked_sub(1)) else { return 0 };
        let (mut lo, mut hi) = (start, start);
        while lo > 0 && trees[lo] - trees[lo - 1] <= FIRE_GAP {
            lo -= 1;
        }
        while hi + 1 < trees.len() && trees[hi + 1] - trees[hi] <= FIRE_GAP {
            hi += 1;
        }
        let at = self.now;
        let mut took = 0;
        for &t in &trees[lo..=hi] {
            // (Saplings and young trees don't carry it: already burnt.)
            if self.tree(t).is_some_and(|(g, _)| g < 1.0) {
                continue;
            }
            let time = self.trees.get(&t).map_or(1, |r| r.time + 1);
            self.trees.insert(t, TreeRecord { lost: at, time });
            took += 1;
        }
        if took > 0 {
            self.fires.push(Wildfire { x0: trees[lo] - FIRE_GAP / 2, x1: trees[hi] + FIRE_GAP / 2, at });
        }
        took
    }

    /// Days since a wildfire swept over any of x0..x1, the latest.
    fn scorched(&self, x0: i32, x1: i32) -> Option<f32> {
        self.fires.iter().filter(|f| f.x0 <= x1 && f.x1 >= x0).map(|f| (self.now - f.at) as f32).reduce(f32::min)
    }

    /// The world begins again (a reset): nothing is healing.
    pub fn forget(&mut self) {
        self.trees.clear();
        self.land.clear();
        self.queue.clear();
        self.fires.clear();
        self.keepers.clear();
    }

    /// How far the tree at x has grown back (0 not yet … 1), and which
    /// time; none: it stands as the seed made it.
    pub fn tree(&self, x: i32) -> Option<(f32, u32)> {
        let r = self.trees.get(&x)?;
        let d = self.now - r.lost;
        let g = if d < SAPLING_DAYS { 0.0 } else { SAPLING + (1.0 - SAPLING) * ((d - SAPLING_DAYS) / GROW_DAYS).clamp(0.0, 1.0) as f32 };
        Some((g, r.time))
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
        self.trees = f.trees.iter().copied().collect();
        self.land = f.land.iter().copied().collect();
        self.fires_done = f.fires_done;
        self.fires = f.fires.clone();
        self.events = f.events.clone();
        self.events_rolled = f.events_rolled;
        self.keepers = f.keepers.iter().cloned().collect();
    }

    fn column_of(&self, x: i32) -> usize {
        ((x.max(0) / self.column.max(1)) as usize).min(self.wet.len().saturating_sub(1))
    }

    /// The land at a world point, in words (the dev readout): how wet, and
    /// what's healing there.
    pub fn describe(&self, at: Vec2) -> String {
        let (x, chunk) = (at.x as i32, ChunkPos::new(at.x.div_euclid(CHUNK as f32) as i32, at.y.div_euclid(CHUNK as f32) as i32));
        let i = self.column_of(x);
        let (w, h) = (self.wet.get(i).copied().unwrap_or(0.0), self.humidity.get(i).copied().unwrap_or(0.0));
        let feel = match w {
            w if w < 0.15 => "tinder-dry",
            w if w < 0.35 => "dry",
            w if w < 0.6 => "damp",
            w if w < 0.85 => "wet",
            _ => "soaked",
        };
        let mut out = format!("{feel} ({:.0} % wet, {:.0} % left to itself)", w * 100.0, h * 100.0);
        if let Some(d) = self.scorched(x, x) {
            out += &format!("; a wildfire's scar, {d:.1} days old");
        }
        if let Some(r) = self.land.get(&(chunk.x, chunk.y)) {
            out += &format!("; healing {} cells, {:.1} days in", r.unhealed, self.now - r.since);
        }
        if let Some((&tx, _)) = self.trees.range(x - 40..=x + 40).min_by_key(|(tx, _)| (**tx - x).abs())
            && let Some((g, time)) = self.tree(tx)
        {
            out += &match g {
                0.0 => format!("; a tree lost here (a sapling in {:.1} days)", self.trees[&tx].lost + SAPLING_DAYS - self.now),
                g if g < 1.0 => format!("; a tree regrowing ({:.0} %, its {time}. time)", g * 100.0),
                _ => format!("; a tree grown back (its {time}. time)"),
            };
        }
        out
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

/// Regrowth: the chunks just loaded heal at once (catching up), and every
/// game hour the loaded ones that have anything to heal, a few a frame
/// (growing back is gradual, so it happens in view too; what's left of a
/// lost tree is cleared only out of view). The hourly pass finds the hurt
/// (so healing counts from about when it happened), as long as time ran
/// steadily: after a jump (a skipped day) it only draws.
fn regrow(mut sim: ResMut<SimWorld>, mut clock: ResMut<WorldClock>, loaded: Res<LoadedChunks>, loaders: Query<(&GlobalTransform, &ChunkLoader)>) {
    if !clock.started {
        return;
    }
    let views: Vec<(Vec2, Vec2)> = loaders.iter().map(|(tf, l)| (tf.translation().truncate() - l.half_extent, tf.translation().truncate() + l.half_extent)).collect();
    let seen = |p: ChunkPos| {
        let (lo, hi) = (Vec2::new((p.x * CHUNK) as f32, (p.y * CHUNK) as f32), Vec2::new(((p.x + 1) * CHUNK) as f32, ((p.y + 1) * CHUNK) as f32));
        views.iter().any(|(a, b)| lo.x < b.x && hi.x > a.x && lo.y < b.y && hi.y > a.y)
    };
    let at = clock.now;
    for p in &loaded.1 {
        clock.away.insert((p.x, p.y), at);
    }
    let near_tree = |clock: &WorldClock, p: ChunkPos| clock.trees.range(p.x * CHUNK - TREE_REACH..(p.x + 1) * CHUNK + TREE_REACH).next().is_some();
    // Just loaded (not yet shown): from the store (maybe hurt), or made
    // anew near a tree lost.
    // (Hurt found on load happened before it was put away: from then.)
    let mut now: Vec<(ChunkPos, bool, bool, bool)> = Vec::new();
    let mut put_away: HashMap<(i32, i32), f64> = HashMap::new();
    for &(p, stored) in &loaded.0 {
        let away = clock.away.remove(&(p.x, p.y)).filter(|_| stored);
        if let Some(t) = away {
            put_away.insert((p.x, p.y), t);
        }
        let cooled = stored && away.is_none_or(|t| at - t > COOL_DAYS);
        if stored || near_tree(&clock, p) || clock.scorched(p.x * CHUNK, (p.x + 1) * CHUNK).is_some() {
            now.push((p, false, false, cooled));
        }
    }
    // The hour: whatever out of view has something to heal.
    let hour = (clock.now * 24.0).floor() as i64;
    if hour != clock.hour {
        clock.judge = hour == clock.hour + 1;
        clock.hour = hour;
        let due: Vec<ChunkPos> = sim.world.chunks().map(|c| (c.pos, c.is_modified())).filter(|&(p, modified)| modified || clock.land.contains_key(&(p.x, p.y)) || near_tree(&clock, p)).map(|(p, _)| p).collect();
        clock.queue = due;
    }
    while now.len() < HEALS_PER_FRAME {
        let Some(p) = clock.queue.pop() else { break };
        if sim.world.is_loaded(p) && !now.iter().any(|&(q, ..)| q == p) {
            now.push((p, seen(p), clock.judge, false));
        }
    }
    if now.is_empty() {
        return;
    }
    let generator = sim.generator.clone();
    let healed: Vec<(ChunkPos, bool, f64, Healed)> = {
        let (clock, world) = (&*clock, &sim.world);
        now.par_iter()
            .filter_map(|&(p, looking, judge, cooled)| {
                let chunk = world.chunk(p)?;
                let since = clock.land.get(&(p.x, p.y)).map(|r| r.since).or_else(|| put_away.get(&(p.x, p.y)).copied()).unwrap_or(clock.now);
                let tree = |x: i32| clock.tree(x);
                let scorched = clock.scorched(p.x * CHUNK, (p.x + 1) * CHUNK);
                generator.heal(chunk, &Healing { days: (clock.now - since) as f32, tree: &tree, judge, cooled, scorched }).map(|h| (p, looking, since, h))
            })
            .collect()
    };
    for (p, looking, since, h) in healed {
        for (c, cell) in h.sudden.into_iter().filter(|_| !looking) {
            sim.world.set_bg(c, cell);
        }
        for (c, back, cell) in h.edits {
            if back {
                sim.world.set_bg(c, cell);
            } else {
                sim.world.set(c, cell);
            }
        }
        for x in h.hurt_trees {
            let time = clock.trees.get(&x).map_or(1, |r| r.time + 1);
            clock.trees.insert(x, TreeRecord { lost: at, time });
        }
        let key = (p.x, p.y);
        if h.unhealed == 0 {
            clock.land.remove(&key);
        } else {
            let r = clock.land.entry(key).or_insert(LandRecord { since, unhealed: h.unhealed });
            // (Hurt again: healing starts over for what's new.)
            if h.unhealed > r.unhealed {
                r.since = at;
            }
            r.unhealed = h.unhealed;
        }
    }
}

/// Wildfires, an hour at a time (catching up): lightning in dry forest
/// where no one is.
fn wildfires(sim: Res<SimWorld>, mut clock: ResMut<WorldClock>, loaders: Query<(&GlobalTransform, &ChunkLoader)>) {
    if !clock.started || clock.wet.is_empty() {
        return;
    }
    let due = (clock.now * 24.0).floor() as i64;
    if clock.fires_done == 0 || clock.fires_done > due {
        clock.fires_done = due;
    }
    if clock.fires_done == due {
        return;
    }
    let near: Vec<f32> = loaders.iter().map(|(tf, _)| tf.translation().x).collect();
    let seed = sim.world.seed();
    let chance = FIRE_RATE * clock.column as f64 / FIRE_WIDTH;
    let generator = sim.generator.clone();
    while clock.fires_done < due {
        clock.fires_done += 1;
        let hour = clock.fires_done;
        for i in 0..clock.wet.len() {
            let dry = ((FIRE_DRY - clock.wet[i]) / FIRE_DRY).clamp(0.0, 1.0);
            let roll = (hash(&[seed, 0xF1BE, i as u64, hour as u64]) >> 11) as f64 / (1u64 << 53) as f64;
            if dry <= 0.0 || roll >= chance * dry as f64 {
                continue;
            }
            let x = i as i32 * clock.column + (hash(&[seed, 0xF1BF, i as u64, hour as u64]) % clock.column.max(1) as u64) as i32;
            if near.iter().any(|&n| (n - x as f32).abs() < FIRE_AWAY) {
                continue;
            }
            let took = clock.wildfire(&*generator, x, dry);
            if took > 0 {
                info!("world clock: a wildfire at x {x} (day {:.1}, {:.0} % wet): {took} trees", clock.now, clock.wet[i] * 100.0);
            }
        }
    }
    let now = clock.now;
    clock.fires.retain(|f| now - f.at < SCAR_DAYS);
}

/// Lairs, hourly: a keeper alive (wherever it wandered) keeps its place's
/// clock; one gone long enough is back (out of sight).
fn lairs(mut commands: Commands, mut clock: ResMut<WorldClock>, creatures: Query<(&Keeps, &Health)>, loaders: Query<(&GlobalTransform, &ChunkLoader)>) {
    let hour = (clock.now * 24.0).floor() as i64;
    if !clock.started || hour == clock.lair_hour {
        return;
    }
    clock.lair_hour = hour;
    let now = clock.now;
    let views: Vec<(Vec2, Vec2)> = loaders.iter().map(|(tf, l)| (tf.translation().truncate() - l.half_extent - 32.0, tf.translation().truncate() + l.half_extent + 32.0)).collect();
    for (&(x, y), keeper) in clock.keepers.iter_mut() {
        let spot = Vec2::new(x as f32, y as f32);
        if creatures.iter().any(|(k, h)| k.0 == (x, y) && h.hp > 0.0) {
            keeper.seen = now;
        } else if now - keeper.seen >= REFILL_DAYS && !views.iter().any(|(a, b)| spot.cmpge(*a).all() && spot.cmple(*b).all()) {
            crate::creatures::def::spawn_creature(&mut commands, &keeper.kind, spot, move |e| {
                e.insert(Keeps((x, y)));
            });
            info!("world clock: a {} back in its lair at ({x}, {y})", keeper.kind);
            keeper.seen = now;
        }
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
