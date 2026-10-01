//! The world clock's events (DESIGN §13, PLAN L1 `world-events`).
//!
//! Each kind on the pattern of the wildfires (`clock.rs`): a timetable
//! rolled from the seed and the day, so whatever happens is the same
//! however the time passed, watched or skipped; a **live** face where
//! someone is (it happens through the simulation, arriving from out of
//! sight); an **away** face where no one is (it's applied to the land as
//! its chunks load, out of view: you find it). What happened is kept a
//! while (`WorldClock::events`, saved with the world): the villagers tell
//! the news (`News`), and a line on screen says so when it's near you.
//!
//! So far:
//!
//! - **Falling stars** (some nights): a streak across the sky, a blast where
//!   it lands; a crater with a glowing meteorite (`meteorite`) and mithril
//!   ore in it, star wisps keeping it. Away, the crater's there when you
//!   come. Seen from far off, a streak toward where it fell.

use bevy::prelude::*;
use platypus_sim::rng::hash;
use platypus_sim::{CellPos, MaterialId, WorldEdit};
use serde::{Deserialize, Serialize};

use crate::actors::player::LocalPlayer;
use crate::actors::creature::spawn_creature;
use crate::actors::spawn::find_ground;
use crate::clock::WorldClock;
use crate::light::{Daylight, LightSource};
use crate::magic::runes::Emitter;
use crate::world::{ChunkLoader, SimWorld};

pub struct EventsPlugin;

impl Plugin for EventsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<News>()
            .add_message::<Begins>()
            .add_systems(Update, (timetable, dev_event, begin, land, stars, news).chain().after(crate::clock::ClockSet));
    }
}

/// What can happen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    Star,
}

/// Where a happening is up to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    /// Its time hasn't come.
    Coming,
    /// It happened where no one was: still to be put into the land (as its
    /// chunks load).
    Away,
    /// It's happening, or has (in the world for good).
    Done,
}

/// One event: what, when (game days), where (cells across), how far along.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Happening {
    pub kind: EventKind,
    pub day: f64,
    pub x: i32,
    pub stage: Stage,
}

/// What the villagers have to tell (newest first).
#[derive(Resource, Default)]
pub struct News(pub Vec<String>);

/// Someone this near (cells across) and it happens live.
const LIVE_NEAR: i32 = 700;
/// An event's time passed longer ago than this (days: a skipped hour) and
/// no one saw it: it happened away.
const LATE: f64 = 0.05;
/// What's happened is told this long (days), and kept this long.
const NEWS_DAYS: f64 = 2.0;
const KEEP_DAYS: f64 = 10.0;

/// Falling stars: the chance a night has one; it falls between these
/// hours; this far from the spawn (cells, either way).
const STAR_CHANCE: f64 = 0.35;
const STAR_HOURS: (f64, f64) = (21.0, 29.0);
const STAR_FROM: (f64, f64) = (900.0, 4_000.0);
/// The streak: how long it flies (seconds), from how far up and across.
const STAR_FLIGHT: f32 = 1.4;
const STAR_FROM_UP: f32 = 360.0;
const STAR_FROM_ACROSS: f32 = 480.0;
/// The crater: the blast's size and power (live), the hole (away), the
/// meteorite's radius and how deep it lies, the mithril beside it.
const CRATER: i32 = 16;
const CRATER_POWER: u8 = 150;
const METEORITE: i32 = 4;
const METEORITE_DEEP: i32 = 9;
const MITHRIL: [(i32, i32, i32); 4] = [(-9, -12, 2), (8, -13, 2), (-3, -16, 1), (4, -17, 2)];
/// Its heat (°C, over this radius): the rim glows, grass round it catches.
const CRATER_HEAT: (i32, i16) = (10, 700);
/// What keeps it: how many, hovering this high over the ground.
const GUARDS: usize = 2;
const GUARD_UP: f32 = 22.0;

/// The whole days rolled (the timetable), each kind's chance a day: what
/// happens on day `d`, if anything, and when and where.
pub fn roll(seed: u64, spawn_x: i32, d: i64, kind: EventKind) -> Option<(f64, i32)> {
    let unit = |salt: u64| (hash(&[seed, d as u64, kind as u64, salt, 0xE7E1]) % 1_000_000) as f64 / 1_000_000.0;
    match kind {
        EventKind::Star => {
            if unit(0) >= STAR_CHANCE {
                return None;
            }
            let hour = STAR_HOURS.0 + (STAR_HOURS.1 - STAR_HOURS.0) * unit(1);
            let side = if unit(2) < 0.5 { -1.0 } else { 1.0 };
            let off = STAR_FROM.0 + (STAR_FROM.1 - STAR_FROM.0) * unit(3);
            Some((d as f64 + hour / 24.0, spawn_x + (side * off) as i32))
        }
    }
}

const KINDS: [EventKind; 1] = [EventKind::Star];

/// One begins, and whether someone's there.
#[derive(Message, Clone, Debug)]
struct Begins {
    h: Happening,
    live: bool,
}

/// Where the players are (cells).
fn players(loaders: &Query<(&GlobalTransform, &ChunkLoader)>) -> Vec<Vec2> {
    loaders.iter().map(|(t, _)| t.translation().truncate()).collect()
}

/// Roll the days to come; start what's due (live where someone is, away
/// where not); forget what's long past.
fn timetable(sim: Res<SimWorld>, mut clock: ResMut<WorldClock>, loaders: Query<(&GlobalTransform, &ChunkLoader)>, mut out: MessageWriter<Begins>) {
    if !clock.started {
        return;
    }
    let now = clock.now;
    let today = now.floor() as i64;
    // (A new world: from today on, not since the world began.)
    if clock.events_rolled == 0 {
        clock.events_rolled = today;
    }
    let (seed, spawn_x) = (sim.world.seed(), sim.generator.spawn_point().x);
    while clock.events_rolled <= today {
        let d = clock.events_rolled + 1;
        for kind in KINDS {
            if let Some((day, x)) = roll(seed, spawn_x, d, kind) {
                clock.events.push(Happening { kind, day, x, stage: Stage::Coming });
            }
        }
        clock.events_rolled = d;
    }
    let near = players(&loaders);
    for h in clock.events.iter_mut() {
        if h.stage != Stage::Coming || h.day > now {
            continue;
        }
        let live = now - h.day < LATE && near.iter().any(|p| (p.x as i32 - h.x).abs() < LIVE_NEAR);
        h.stage = if live { Stage::Done } else { Stage::Away };
        out.write(Begins { h: h.clone(), live });
    }
    clock.events.retain(|h| h.stage != Stage::Done || now - h.day < KEEP_DAYS);
}

/// The dev panel's "A falling star": one now, a little ahead of you.
fn dev_event(mut acts: MessageReader<crate::dev::DevAction>, mut clock: ResMut<WorldClock>, player: Query<&crate::actors::Kinematics, With<LocalPlayer>>) {
    for a in acts.read() {
        if !matches!(a, crate::dev::DevAction::Event) {
            continue;
        }
        let Ok(k) = player.single() else { continue };
        let ahead = if k.loco.facing >= 0.0 { 1.0 } else { -1.0 };
        let x = (k.body.pos.x + ahead * 140.0) as i32;
        let day = clock.now;
        clock.events.push(Happening { kind: EventKind::Star, day, x, stage: Stage::Coming });
    }
}

/// A star falling, seen: where it flies from and to, how far along.
#[derive(Component)]
struct Streak {
    from: Vec2,
    to: Vec2,
    t: f32,
    /// It lands (seen near), or flies out of sight (far off).
    lands: bool,
}

/// Something begins: a star falls (seen near, seen far, or not at all).
#[allow(clippy::too_many_arguments)]
fn begin(
    mut commands: Commands,
    mut begins: MessageReader<Begins>,
    clock: Res<WorldClock>,
    sim: Res<SimWorld>,
    day: Res<Daylight>,
    cam: Single<&Transform, With<crate::camera::MainCamera>>,
    mut toasts: MessageWriter<crate::progress::Toast>,
) {
    for b in begins.read() {
        let h = &b.h;
        match h.kind {
            EventKind::Star => {
                let c = cam.translation.truncate();
                let west = (h.x as f32) < c.x;
                let side = if west { -1.0 } else { 1.0 };
                if b.live {
                    let ground = sim.generator.surface_hint(h.x).unwrap_or(c.y as i32) as f32;
                    let to = Vec2::new(h.x as f32 + 0.5, ground);
                    // (In from the far side, high: it crosses the sky.)
                    let from = to + Vec2::new(-side * STAR_FROM_ACROSS, STAR_FROM_UP);
                    commands.spawn(streak(from, to, true));
                    toasts.write(crate::progress::Toast("A star is falling!".into()));
                } else {
                    // Away: there when you come (`land`). Seen from the
                    // surface by night, a streak toward it, gone over the
                    // horizon.
                    let dark = !(0.27..0.77).contains(&day.time);
                    let above = sim.generator.surface_hint(c.x as i32).is_none_or(|s| c.y > s as f32 - 40.0);
                    if dark && above && clock.now - h.day < LATE {
                        let from = c + Vec2::new(-side * 200.0, 150.0);
                        let to = c + Vec2::new(side * 320.0, 20.0);
                        commands.spawn(streak(from, to, false));
                        toasts.write(crate::progress::Toast(format!("A star falls, far to the {}", if west { "west" } else { "east" })));
                    }
                }
            }
        }
    }
}

fn streak(from: Vec2, to: Vec2, lands: bool) -> impl Bundle {
    (
        Name::new("Falling star"),
        Streak { from, to, t: 0.0, lands },
        Transform::from_translation(from.extend(40.0)),
        Sprite::from_color(Color::srgb(1.0, 1.0, 1.0), Vec2::splat(3.0)),
        LightSource { color: [0.9, 0.95, 1.4], flicker: 0.1 },
    )
}

/// The trail behind a star (cells; glowing, fading blue).
fn trail() -> Emitter {
    Emitter {
        count: 1.0,
        life: (0.25, 0.7),
        colors: vec![(255, 255, 255), (190, 210, 255), (90, 120, 230)],
        speed: 14.0,
        spread: 0.5,
        gravity: 0.0,
        drag: 2.0,
        size: 1.6,
        jitter: 0.6,
        glow: true,
    }
}

/// Stars in flight: along, faster as they fall, a trail behind; landing,
/// the blast, the meteorite and ore in the crater, its heat, its keepers.
fn stars(
    mut commands: Commands,
    time: Res<Time>,
    mut sim: ResMut<SimWorld>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut q: Query<(Entity, &mut Streak, &mut Transform)>,
) {
    let e = trail();
    for (entity, mut s, mut tf) in &mut q {
        s.t += time.delta_secs() / STAR_FLIGHT;
        // (Speeding up: it's falling.)
        let k = s.t.min(1.0).powf(1.6);
        let at = s.from.lerp(s.to, k);
        let back = (s.from - s.to).normalize_or_zero();
        sparks.emit(&e, 6, at, back, Vec2::ZERO);
        tf.translation = at.extend(40.0);
        if s.t < 1.0 {
            continue;
        }
        commands.entity(entity).despawn();
        if !s.lands {
            continue;
        }
        // Where it really lands: the ground under it as it is now.
        let x = s.to.x as i32;
        let ground = find_ground(&sim.world, x, s.to.y as i32 + 120, 400).unwrap_or(s.to.y as i32);
        sim.queue(WorldEdit::Explode { center: CellPos::new(x, ground), radius: CRATER, power: CRATER_POWER });
        fill_crater(&mut sim, x, ground);
        guard(&mut commands, x, ground);
    }
}

/// The meteorite and its mithril in a crater at (x, ground), and its heat.
fn fill_crater(sim: &mut SimWorld, x: i32, ground: i32) {
    let mats = sim.world.materials();
    let (Some(meteorite), Some(mithril)) = (mats.id("meteorite"), mats.id("mithril_ore")) else { return };
    let paint = |sim: &mut SimWorld, dx: i32, dy: i32, r: i32, m: MaterialId| sim.queue(WorldEdit::Paint { center: CellPos::new(x + dx, ground + dy), radius: r, material: m, overwrite: true });
    paint(sim, 0, -METEORITE_DEEP, METEORITE, meteorite);
    for (dx, dy, r) in MITHRIL {
        paint(sim, dx, dy, r, mithril);
    }
    sim.queue(WorldEdit::Heat { center: CellPos::new(x, ground - METEORITE_DEEP), radius: CRATER_HEAT.0, amount: CRATER_HEAT.1 });
}

/// Star wisps hovering over the crater.
fn guard(commands: &mut Commands, x: i32, ground: i32) {
    for i in 0..GUARDS {
        let dx = (i as f32 * 2.0 - 1.0) * 10.0;
        // (Its place, kept with it in a save: it stays on its leash.)
        spawn_creature(commands, "star_wisp", Vec2::new(x as f32 + dx, ground as f32 + GUARD_UP), move |e| {
            e.insert(crate::clock::Keeps((x, ground)));
        });
    }
}

/// A star that fell where no one was: the crater's dug into the land once
/// its chunks are in round it (they load beyond the screen's edge), quietly
/// (no blast to hear): the hole, the scorched rim, what's in it, its keepers.
fn land(mut commands: Commands, mut sim: ResMut<SimWorld>, mut clock: ResMut<WorldClock>) {
    for h in clock.events.iter_mut().filter(|h| h.kind == EventKind::Star && h.stage == Stage::Away) {
        let x = h.x;
        let Some(hint) = sim.generator.surface_hint(x) else { continue };
        let loaded = (-1..=1).all(|i| sim.world.get(CellPos::new(x + i * 40, hint)).is_some() && sim.world.get(CellPos::new(x + i * 40, hint - 60)).is_some());
        if !loaded {
            continue;
        }
        let Some(ground) = find_ground(&sim.world, x, hint + 120, 400) else { continue };
        sim.queue(WorldEdit::Dig { center: CellPos::new(x, ground), radius: CRATER - 2, max_hardness: 200 });
        sim.queue(WorldEdit::Scorch { center: CellPos::new(x, ground), radius: CRATER + 4 });
        fill_crater(&mut sim, x, ground);
        guard(&mut commands, x, ground);
        h.stage = Stage::Done;
        info!("events: a star fell at x {x} while no one was there; its crater's dug (ground {ground})");
    }
}

/// What the villagers say happened: the last few days', newest first.
fn news(clock: Res<WorldClock>, sim: Res<SimWorld>, mut news: ResMut<News>) {
    let village = sim.generator.spawn_point().x;
    let now = clock.now;
    let mut told: Vec<(f64, String)> = clock
        .events
        .iter()
        .filter(|h| h.stage != Stage::Coming && now - h.day < NEWS_DAYS)
        .map(|h| {
            let when = if now - h.day < 0.6 { "last night" } else { "the other night" };
            let (dir, far) = (if h.x < village { "west" } else { "east" }, walk((h.x - village).abs()));
            let line = match h.kind {
                EventKind::Star => format!("A star fell {when}, {far} {dir} of here. Something keeps it."),
            };
            (h.day, line)
        })
        .collect();
    told.sort_by(|a, b| b.0.total_cmp(&a.0));
    news.0 = told.into_iter().map(|(_, l)| l).collect();
}

/// How far a walk is, said (a walk is ~3 600 cells a minute).
fn walk(cells: i32) -> String {
    match cells / 3_600 {
        0 => "a short walk".into(),
        1 => "a minute's walk".into(),
        n => format!("{n} minutes' walk"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_timetable_is_the_same_however_it_is_read() {
        // Rolled twice, the same; some nights a star, not most.
        let a: Vec<_> = (0..200).filter_map(|d| roll(7, 65_536, d, EventKind::Star)).collect();
        let b: Vec<_> = (0..200).filter_map(|d| roll(7, 65_536, d, EventKind::Star)).collect();
        assert_eq!(a, b);
        assert!((40..100).contains(&a.len()), "{} stars in 200 nights", a.len());
        for (day, x) in a {
            let hour = day.fract() * 24.0;
            assert!(!(5.0..21.0).contains(&hour), "a star by day ({hour:.1} h)");
            let off = (x - 65_536).abs() as f64;
            assert!((STAR_FROM.0 - 1.0..=STAR_FROM.1).contains(&off), "{off} cells from the spawn");
        }
    }
}
