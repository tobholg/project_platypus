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
//! - **Raids** (some evenings): a warband marches on the village. There, it
//!   walks in from out of sight (`ai::Marching`) and you fight it; the
//!   villagers run (one killed comes back days later: they're keepers).
//!   Away, holes are knocked in the houses. Either way the village mends
//!   itself toward what it was, out of view, over the hours after; the guide
//!   takes gold to have it done by morning (`Village`).
//! - **Earthquakes** (rare): felt far and wide. Near you the screen shakes
//!   with a low rumble and cave ceilings round you come loose and fall; at
//!   the quake's heart a chasm opens in the surface (dug as it loads, near
//!   or away).
//! - **A travelling pedlar** (some mornings): walks in to the village from
//!   out of sight (or is there already, if you come later), stays the day
//!   with things from far off, and walks on.

use bevy::prelude::*;
use platypus_sim::rng::hash;
use platypus_sim::{CellPos, MaterialId, WorldEdit};
use serde::{Deserialize, Serialize};

use crate::creatures::brain::ai::Marching;
use crate::creatures::player::LocalPlayer;
use crate::creatures::def::spawn_creature;
use crate::creatures::spawn::find_ground;
use crate::clock::WorldClock;
use crate::light::{Daylight, LightSource};
use crate::magic::runes::Emitter;
use crate::world::{ChunkLoader, SimWorld};

pub struct EventsPlugin;

impl Plugin for EventsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<News>()
            .init_resource::<Village>()
            .add_message::<Begins>()
            .add_systems(Update, (timetable, dev_event, begin, land, stars, shake, mend, pedlar, news).chain().after(crate::clock::ClockSet));
    }
}

/// What can happen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    Star,
    Raid,
    Quake,
    Pedlar,
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
    /// Which way it comes from (a raid: -1 west, 1 east).
    #[serde(default)]
    pub from: i32,
}

/// What the villagers have to tell (newest first).
#[derive(Resource, Default)]
pub struct News(pub Vec<String>);

/// The village's mending: cells of it missing (its walls, roofs, floors,
/// as the seed made them; in the chunks last looked at), whether the guide's
/// been paid to have it done, and how many cells may be put back (it
/// accrues by the hour, loaded or not; a raid starts it again).
#[derive(Resource, Default)]
pub struct Village {
    pub missing: u32,
    pub paid: bool,
    allowance: f64,
    /// The game hour last looked at, the day the allowance was last added
    /// to, and a tick to look again at (after a raid's edits are in).
    hour: i64,
    accrued: Option<f64>,
    look: Option<u64>,
}

/// What the guide takes to have the village mended by morning (gold).
pub const MEND_PRICE: u32 = 60;

/// Someone this near (cells across) and it happens live.
const LIVE_NEAR: i32 = 1_050;
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
const STAR_FROM: (f64, f64) = (1_350.0, 6_000.0);
/// The streak: how long it flies (seconds), from how far up and across.
const STAR_FLIGHT: f32 = 1.4;
const STAR_FROM_UP: f32 = 540.0;
const STAR_FROM_ACROSS: f32 = 720.0;
/// The crater: the blast's size and power (live), the hole (away), the
/// meteorite's radius and how deep it lies, the mithril beside it.
const CRATER: i32 = 24;
const CRATER_POWER: u8 = 150;
const METEORITE: i32 = 6;
const METEORITE_DEEP: i32 = 14;
const MITHRIL: [(i32, i32, i32); 4] = [(-14, -18, 3), (12, -20, 3), (-5, -24, 2), (6, -26, 3)];
/// Its heat (°C, over this radius): the rim glows, grass round it catches.
const CRATER_HEAT: (i32, i16) = (15, 700);
/// What keeps it: how many, hovering this high over the ground.
const GUARDS: usize = 2;
const GUARD_UP: f32 = 33.0;

/// Raids: the chance an evening has one; it comes between these hours.
const RAID_CHANCE: f64 = 0.3;
const RAID_HOURS: (f64, f64) = (17.0, 21.0);
/// Live, the warband comes in from this far beyond the middle of the view
/// (cells: out of sight, in the loaded world).
const RAID_FROM: f32 = 495.0;
/// Away, holes knocked in the houses: how many, how big.
const RAID_HOLES: u64 = 5;
const RAID_HOLE: i32 = 6;
/// Mending: cells put back a game hour (a raid's damage is mended in about
/// half a day), and what's this near a player (across, up) is in view and
/// waits.
const MEND_PER_HOUR: usize = 90;
const VIEW: (i32, i32) = (420, 255);

/// Earthquakes: the chance a day has one; its heart this far from the
/// spawn (cells, either way); felt this far from it (live).
const QUAKE_CHANCE: f64 = 0.12;
const QUAKE_FROM: (f64, f64) = (1_200.0, 9_000.0);
const QUAKE_FELT: i32 = 4_500;
/// It shakes this long (seconds), this hard (camera trauma, 0..1), and
/// brings down a piece of cave ceiling round each player (within this, in
/// cells across and down) this often (seconds).
const QUAKE_SECS: f32 = 5.0;
const QUAKE_TRAUMA: f32 = 0.55;
const QUAKE_FALL_EVERY: f32 = 0.12;
const QUAKE_NEAR: (i32, i32) = (255, 195);
/// The chasm at its heart: how deep (cells, a range), how wide at the top.
const CHASM_DEEP: (i32, i32) = (90, 165);
const CHASM_WIDE: i32 = 12;

/// The pedlar: the chance a morning brings one; it comes between these
/// hours, and stays this long (days).
const PEDLAR_CHANCE: f64 = 0.25;
const PEDLAR_HOURS: (f64, f64) = (8.0, 11.0);
const PEDLAR_STAY: f64 = 1.0;

/// Where things happen from: the spawn, and the village's bounds.
#[derive(Clone, Copy)]
pub struct Places {
    pub spawn_x: i32,
    pub village: Option<(CellPos, CellPos)>,
}

/// The whole days rolled (the timetable), each kind's chance a day: what
/// happens on day `d`, if anything, and when and where.
pub fn roll(seed: u64, places: Places, d: i64, kind: EventKind) -> Option<(f64, i32, i32)> {
    let unit = |salt: u64| (hash(&[seed, d as u64, kind as u64, salt, 0xE7E1]) % 1_000_000) as f64 / 1_000_000.0;
    let side = if unit(2) < 0.5 { -1 } else { 1 };
    match kind {
        EventKind::Star => {
            if unit(0) >= STAR_CHANCE {
                return None;
            }
            let hour = STAR_HOURS.0 + (STAR_HOURS.1 - STAR_HOURS.0) * unit(1);
            let off = STAR_FROM.0 + (STAR_FROM.1 - STAR_FROM.0) * unit(3);
            Some((d as f64 + hour / 24.0, places.spawn_x + side * off as i32, 0))
        }
        EventKind::Quake => {
            if unit(0) >= QUAKE_CHANCE {
                return None;
            }
            let off = QUAKE_FROM.0 + (QUAKE_FROM.1 - QUAKE_FROM.0) * unit(3);
            Some((d as f64 + unit(1), places.spawn_x + side * off as i32, 0))
        }
        EventKind::Pedlar => {
            let (lo, hi) = places.village?;
            if unit(0) >= PEDLAR_CHANCE {
                return None;
            }
            let hour = PEDLAR_HOURS.0 + (PEDLAR_HOURS.1 - PEDLAR_HOURS.0) * unit(1);
            Some((d as f64 + hour / 24.0, (lo.x + hi.x) / 2, side))
        }
        EventKind::Raid => {
            let (lo, hi) = places.village?;
            if unit(0) >= RAID_CHANCE {
                return None;
            }
            let hour = RAID_HOURS.0 + (RAID_HOURS.1 - RAID_HOURS.0) * unit(1);
            Some((d as f64 + hour / 24.0, (lo.x + hi.x) / 2, side))
        }
    }
}

const KINDS: [EventKind; 4] = [EventKind::Star, EventKind::Raid, EventKind::Quake, EventKind::Pedlar];

/// How near (cells across) someone must be for a kind to happen live.
fn live_near(kind: EventKind) -> i32 {
    match kind {
        EventKind::Quake => QUAKE_FELT,
        _ => LIVE_NEAR,
    }
}

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
    let seed = sim.world.seed();
    let places = Places { spawn_x: sim.generator.spawn_point().x, village: sim.generator.village() };
    while clock.events_rolled <= today {
        let d = clock.events_rolled + 1;
        for kind in KINDS {
            if let Some((day, x, from)) = roll(seed, places, d, kind) {
                clock.events.push(Happening { kind, day, x, stage: Stage::Coming, from });
            }
        }
        clock.events_rolled = d;
    }
    let near = players(&loaders);
    for h in clock.events.iter_mut() {
        if h.stage != Stage::Coming || h.day > now {
            continue;
        }
        let live = now - h.day < LATE && near.iter().any(|p| (p.x as i32 - h.x).abs() < live_near(h.kind));
        // (A quake felt here still has its chasm to open, where it struck.)
        // (The pedlar's there or not by the day: `pedlar`.)
        h.stage = if (live && h.kind != EventKind::Quake) || h.kind == EventKind::Pedlar { Stage::Done } else { Stage::Away };
        out.write(Begins { h: h.clone(), live });
    }
    clock.events.retain(|h| h.stage != Stage::Done || now - h.day < KEEP_DAYS);
}

/// The dev panel's "An event": one now, near you, each kind in turn (a
/// star a little ahead; a raid on the village).
fn dev_event(mut acts: MessageReader<crate::dev::DevAction>, mut next: Local<usize>, sim: Res<SimWorld>, mut clock: ResMut<WorldClock>, player: Query<&crate::creatures::Kinematics, With<LocalPlayer>>) {
    for a in acts.read() {
        if !matches!(a, crate::dev::DevAction::Event) {
            continue;
        }
        let Ok(k) = player.single() else { continue };
        let kind = KINDS[*next % KINDS.len()];
        *next += 1;
        let day = clock.now;
        let ahead = if k.loco.facing >= 0.0 { 1 } else { -1 };
        let (x, from) = match kind {
            EventKind::Star => ((k.body.pos.x + ahead as f32 * 210.0) as i32, 0),
            EventKind::Raid => match sim.generator.village() {
                Some((lo, hi)) => ((lo.x + hi.x) / 2, ahead),
                None => continue,
            },
            EventKind::Quake => ((k.body.pos.x + ahead as f32 * 300.0) as i32, 0),
            EventKind::Pedlar => match sim.generator.village() {
                Some((lo, hi)) => ((lo.x + hi.x) / 2, ahead),
                None => continue,
            },
        };
        clock.events.push(Happening { kind, day, x, stage: Stage::Coming, from });
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
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut village: ResMut<Village>,
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
                    let above = sim.generator.surface_hint(c.x as i32).is_none_or(|s| c.y > s as f32 - 60.0);
                    if dark && above && clock.now - h.day < LATE {
                        let from = c + Vec2::new(-side * 300.0, 225.0);
                        let to = c + Vec2::new(side * 480.0, 30.0);
                        commands.spawn(streak(from, to, false));
                        toasts.write(crate::progress::Toast(format!("A star falls, far to the {}", if west { "west" } else { "east" })));
                    }
                }
            }
            EventKind::Raid => {
                if !b.live {
                    // (Away: the damage is done as its chunks load: `land`.)
                    continue;
                }
                // In from out of sight on its side, marching on the village.
                let c = cam.translation.truncate();
                let start = c.x + h.from as f32 * RAID_FROM;
                let members: Vec<String> = crate::creatures::spawn::packs().remove("raiders").unwrap_or_default().into_iter().flat_map(|(k, n)| std::iter::repeat_n(k, n as usize)).collect();
                for (i, kind) in members.into_iter().enumerate() {
                    let x = (start + h.from as f32 * i as f32 * 18.0) as i32;
                    let hint = sim.generator.surface_hint(x).unwrap_or(c.y as i32);
                    let Some(ground) = find_ground(&sim.world, x, hint + 120, 450) else { continue };
                    let target = h.x as f32;
                    spawn_creature(&mut commands, &kind, Vec2::new(x as f32 + 0.5, ground as f32), move |e| {
                        e.insert(Marching(target));
                    });
                }
                toasts.write(crate::progress::Toast(format!("A warband is coming from the {}!", if h.from < 0 { "west" } else { "east" })));
                village.look = Some(sim.world.tick() + 2);
                village.allowance = 0.0;
            }
            EventKind::Pedlar => {
                if b.live {
                    toasts.write(crate::progress::Toast("A travelling pedlar is coming to the village".into()));
                }
            }
            EventKind::Quake => {
                if b.live {
                    commands.spawn((Name::new("Earthquake"), Quaking { left: QUAKE_SECS, next: 0.0, fell: 0 }));
                    sounds.write(crate::sound::PlaySound::here("thunder").pitch(0.45).volume(1.0));
                    toasts.write(crate::progress::Toast("The ground shakes!".into()));
                }
            }
        }
    }
}

/// An earthquake under way: how long it has left, and when the next piece
/// of ceiling comes down.
#[derive(Component)]
struct Quaking {
    left: f32,
    next: f32,
    fell: u32,
}

/// The ground shaking: the camera, and round each player pieces of cave
/// ceiling come loose and fall (rubble, thrown down).
fn shake(
    mut commands: Commands,
    time: Res<Time>,
    mut sim: ResMut<SimWorld>,
    mut trauma: ResMut<crate::fx::Trauma>,
    loaders: Query<(&GlobalTransform, &ChunkLoader)>,
    mut q: Query<(Entity, &mut Quaking)>,
) {
    let dt = time.delta_secs();
    for (e, mut quake) in &mut q {
        quake.left -= dt;
        if quake.left <= 0.0 {
            info!("events: the quake brought down {} pieces of cave ceiling", quake.fell);
            commands.entity(e).despawn();
            continue;
        }
        // (Easing off over its last second.)
        trauma.0 = trauma.0.max(QUAKE_TRAUMA * quake.left.min(1.0));
        quake.next -= dt;
        if quake.next > 0.0 {
            continue;
        }
        quake.next = QUAKE_FALL_EVERY;
        let (tick, seed) = (sim.world.tick(), sim.world.seed());
        for (i, p) in players(&loaders).into_iter().enumerate() {
            let unit = |salt: u64| (hash(&[seed, tick, i as u64, salt, 0x0AE7]) % 10_000) as f32 / 10_000.0;
            let at = CellPos::new(p.x as i32 + ((unit(0) * 2.0 - 1.0) * QUAKE_NEAR.0 as f32) as i32, p.y as i32 - (unit(1) * QUAKE_NEAR.1 as f32) as i32 + 45);
            if let Some(c) = ceiling(&sim, at) {
                quake.fell += 1;
                sim.queue(WorldEdit::Shatter { center: c, from: c.offset(0, 6), radius: 3 + (unit(2) * 3.0) as i32, max_hardness: 120 });
            }
        }
    }
}

/// A cave's ceiling over `at` (open air there), underground: the first
/// solid cell up from it with air under it, within 30 cells.
fn ceiling(sim: &SimWorld, at: CellPos) -> Option<CellPos> {
    let open = |p: CellPos| sim.world.get(p).is_some_and(|c| c.is_air());
    if !open(at) {
        return None;
    }
    let surface = sim.generator.surface_hint(at.x)?;
    (1..45).map(|d| at.offset(0, d)).find(|&p| !open(p) && sim.world.get(p).is_some()).filter(|p| p.y < surface - 15)
}

/// A chasm at x: a jagged crack down from the ground, narrowing (the same
/// from the seed and the day wherever it's dug from).
fn chasm(sim: &mut SimWorld, x: i32, ground: i32, day: f64) {
    let seed = sim.world.seed();
    let u = |salt: u64| (hash(&[seed, day.to_bits(), salt, 0xC4A5]) % 10_000) as i32;
    let deep = CHASM_DEEP.0 + u(0) % (CHASM_DEEP.1 - CHASM_DEEP.0);
    let mut cx = x;
    for (i, y) in (0..deep).step_by(3).enumerate() {
        let k = 1.0 - y as f32 / deep as f32;
        let r = ((CHASM_WIDE as f32 / 2.0) * k).max(1.0) as i32;
        cx += u(10 + i as u64) % 3 - 1;
        sim.queue(WorldEdit::Dig { center: CellPos::new(cx, ground + 2 - y), radius: r, max_hardness: 200 });
    }
}

fn streak(from: Vec2, to: Vec2, lands: bool) -> impl Bundle {
    (
        Name::new("Falling star"),
        Streak { from, to, t: 0.0, lands },
        Transform::from_translation(from.extend(40.0)),
        Sprite::from_color(Color::srgb(1.0, 1.0, 1.0), Vec2::splat(4.5)),
        LightSource { color: [0.9, 0.95, 1.4], flicker: 0.1 },
    )
}

/// The trail behind a star (cells; glowing, fading blue).
fn trail() -> Emitter {
    Emitter {
        count: 1.0,
        life: (0.25, 0.7),
        colors: vec![(255, 255, 255), (190, 210, 255), (90, 120, 230)],
        speed: 21.0,
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
        let ground = find_ground(&sim.world, x, s.to.y as i32 + 180, 600).unwrap_or(s.to.y as i32);
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
        let dx = (i as f32 * 2.0 - 1.0) * 15.0;
        // (Its place, kept with it in a save: it stays on its leash.)
        spawn_creature(commands, "star_wisp", Vec2::new(x as f32 + dx, ground as f32 + GUARD_UP), move |e| {
            e.insert(crate::clock::Keeps((x, ground)));
        });
    }
}

/// A star that fell where no one was: the crater's dug into the land once
/// its chunks are in round it (they load beyond the screen's edge), quietly
/// (no blast to hear): the hole, the scorched rim, what's in it, its keepers.
fn land(mut commands: Commands, mut sim: ResMut<SimWorld>, mut clock: ResMut<WorldClock>, mut village: ResMut<Village>) {
    // A raid where no one was: holes knocked in the houses, once they're in.
    if let Some((lo, hi)) = sim.generator.village() {
        let seed = sim.world.seed();
        for h in clock.events.iter_mut().filter(|h| h.kind == EventKind::Raid && h.stage == Stage::Away) {
            let corners = [(lo.x, lo.y), (hi.x, lo.y), (lo.x, hi.y), (hi.x, hi.y)];
            if corners.iter().any(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_none()) {
                continue;
            }
            for i in 0..RAID_HOLES {
                let u = |salt: u64| (hash(&[seed, h.day.to_bits(), i, salt, 0x4A1D]) % 10_000) as i32;
                let at = CellPos::new(lo.x + u(0) % (hi.x - lo.x).max(1), lo.y + 12 + u(1) % (hi.y - lo.y - 12).max(1));
                sim.queue(WorldEdit::Dig { center: at, radius: RAID_HOLE, max_hardness: 200 });
            }
            h.stage = Stage::Done;
            village.look = Some(sim.world.tick() + 2);
            village.allowance = 0.0;
            info!("events: a raid on the village while no one was there; {RAID_HOLES} holes knocked in its houses");
        }
    }
    // A quake's chasm, where it struck, once it's in.
    for h in clock.events.iter_mut().filter(|h| h.kind == EventKind::Quake && h.stage == Stage::Away) {
        let x = h.x;
        let Some(hint) = sim.generator.surface_hint(x) else { continue };
        if [hint + 15, hint - CHASM_DEEP.1].iter().any(|&y| sim.world.get(CellPos::new(x, y)).is_none()) {
            continue;
        }
        let Some(ground) = find_ground(&sim.world, x, hint + 180, 600) else { continue };
        chasm(&mut sim, x, ground, h.day);
        h.stage = Stage::Done;
        info!("events: a quake's chasm opened at x {x} (ground {ground})");
    }
    for h in clock.events.iter_mut().filter(|h| h.kind == EventKind::Star && h.stage == Stage::Away) {
        let x = h.x;
        let Some(hint) = sim.generator.surface_hint(x) else { continue };
        let loaded = (-1..=1).all(|i| sim.world.get(CellPos::new(x + i * 60, hint)).is_some() && sim.world.get(CellPos::new(x + i * 60, hint - 90)).is_some());
        if !loaded {
            continue;
        }
        let Some(ground) = find_ground(&sim.world, x, hint + 180, 600) else { continue };
        sim.queue(WorldEdit::Dig { center: CellPos::new(x, ground), radius: CRATER - 3, max_hardness: 200 });
        sim.queue(WorldEdit::Scorch { center: CellPos::new(x, ground), radius: CRATER + 6 });
        fill_crater(&mut sim, x, ground);
        guard(&mut commands, x, ground);
        h.stage = Stage::Done;
        info!("events: a star fell at x {x} while no one was there; its crater's dug (ground {ground})");
    }
}

/// The village mends itself toward what the seed made. What's missing of
/// its walls, roofs and floors (as they were made: planks, brick, platforms;
/// in front and behind) where there's nothing now (or ash, charcoal,
/// rubble) is put back, out of view, as the allowance allows
/// (`MEND_PER_HOUR` a game hour; all of it, paid for). Looked at chunk by
/// chunk, each as it's in: hourly, just after a raid, and as one loads (it
/// loads beyond the screen's edge: coming back, what's due is done before
/// you see it). What's been built over stays.
fn mend(
    mut sim: ResMut<SimWorld>,
    clock: Res<WorldClock>,
    mut village: ResMut<Village>,
    loaded: Res<crate::world::LoadedChunks>,
    loaders: Query<(&GlobalTransform, &ChunkLoader)>,
) {
    let Some((lo, hi)) = sim.generator.village() else { return };
    if !clock.started {
        return;
    }
    // The allowance, by the hour.
    let accrued = *village.accrued.get_or_insert(clock.now);
    let hours = ((clock.now - accrued) * 24.0).floor().max(0.0);
    village.accrued = Some(accrued + hours / 24.0);
    village.allowance = (village.allowance + hours * MEND_PER_HOUR as f64).min(1e6);
    let (c0, c1) = (lo.chunk(), hi.chunk());
    let hour = (clock.now * 24.0).floor() as i64;
    let looking = village.look.is_some_and(|t| sim.world.tick() >= t);
    let came_in = loaded.0.iter().any(|(p, _)| (c0.x..=c1.x).contains(&p.x) && (c0.y..=c1.y).contains(&p.y));
    if hour == village.hour && !looking && !came_in {
        return;
    }
    village.hour = hour;
    village.look = None;
    let mats = sim.world.materials().clone();
    let made: Vec<MaterialId> = ["planks", "brick", "platform"].iter().filter_map(|n| mats.id(n)).collect();
    let gone: Vec<MaterialId> = ["ash", "charcoal", "gravel", "fire", "smoke"].iter().filter_map(|n| mats.id(n)).collect();
    let empty = |c: platypus_sim::Cell| c.is_air() || gone.contains(&c.material);
    let near: Vec<Vec2> = players(&loaders);
    let in_view = |p: CellPos| near.iter().any(|v| (p.x - v.x as i32).abs() < VIEW.0 && (p.y - v.y as i32).abs() < VIEW.1);
    // What's missing, front and back, in the chunks that are in (each made
    // again from the seed).
    let mut missing: Vec<(CellPos, platypus_sim::Cell, bool)> = Vec::new();
    let mut all_in = true;
    for cy in c0.y..=c1.y {
        for cx in c0.x..=c1.x {
            let pos = platypus_sim::ChunkPos::new(cx, cy);
            if sim.world.get(pos.origin()).is_none() {
                all_in = false;
                continue;
            }
            let made_chunk = sim.generator.generate(pos);
            let origin = made_chunk.pos.origin();
            for (i, (front, back)) in made_chunk.cells().iter().zip(made_chunk.background()).enumerate() {
                let p = origin.offset((i % platypus_sim::CHUNK as usize) as i32, (i / platypus_sim::CHUNK as usize) as i32);
                if p.x < lo.x || p.x > hi.x || p.y < lo.y || p.y > hi.y {
                    continue;
                }
                if made.contains(&front.material) && sim.world.get(p).is_some_and(empty) {
                    missing.push((p, *front, false));
                }
                if made.contains(&back.material) && sim.world.get_bg(p).is_some_and(|b| b.is_air()) {
                    missing.push((p, *back, true));
                }
            }
        }
    }
    // From the bottom up (a wall before its roof).
    missing.sort_by_key(|(p, _, _)| (p.y, p.x));
    let most = if village.paid { usize::MAX } else { village.allowance as usize };
    let mut put = 0;
    for &(p, cell, back) in missing.iter().filter(|(p, _, _)| !in_view(*p)).take(most) {
        if back {
            sim.world.set_bg(p, cell);
        } else {
            sim.world.set(p, cell);
        }
        put += 1;
    }
    village.allowance = (village.allowance - put as f64).max(0.0);
    village.missing = (missing.len() - put) as u32;
    // (All of it whole: nothing owed, nothing saved up.)
    if all_in && village.missing == 0 {
        village.paid = false;
        village.allowance = 0.0;
    }
}

/// The pedlar, while one's staying: there's someone near the village and
/// no pedlar about, one's made, at the village if that's out of view, else
/// walking in from out of sight on its side (its home the village's
/// middle). Its stay over, it walks off that way and is gone once no one
/// sees it.
#[allow(clippy::too_many_arguments)]
fn pedlar(
    mut commands: Commands,
    sim: Res<SimWorld>,
    clock: Res<WorldClock>,
    cam: Single<&Transform, With<crate::camera::MainCamera>>,
    loaders: Query<(&GlobalTransform, &ChunkLoader)>,
    mut folk: Query<(Entity, &crate::creatures::brain::villager::Villager, &crate::creatures::Kinematics, Option<&mut crate::creatures::brain::villager::Home>)>,
) {
    let Some((lo, hi)) = sim.generator.village() else { return };
    let mid = (lo.x + hi.x) / 2;
    let now = clock.now;
    let staying = clock.events.iter().find(|h| h.kind == EventKind::Pedlar && h.day <= now && now < h.day + PEDLAR_STAY);
    let near = players(&loaders);
    let seen = |x: f32, y: f32| near.iter().any(|p| (p.x - x).abs() < VIEW.0 as f32 && (p.y - y).abs() < VIEW.1 as f32);
    let mut here = false;
    for (e, v, k, home) in &mut folk {
        if v.role != "pedlar" {
            continue;
        }
        here = true;
        if staying.is_some() {
            continue;
        }
        // Off: away the way it came, gone once out of sight.
        if seen(k.body.pos.x, k.body.pos.y) {
            if let Some(mut home) = home {
                let side = if k.body.pos.x < mid as f32 { -1.0 } else { 1.0 };
                home.0.x = mid as f32 + side * 1_350.0;
            }
        } else {
            commands.entity(e).despawn();
        }
    }
    let Some(h) = staying else { return };
    if here || !near.iter().any(|p| (p.x as i32 - mid).abs() < LIVE_NEAR) {
        return;
    }
    let ground = |x: i32| sim.generator.surface_hint(x).and_then(|hint| find_ground(&sim.world, x, hint + 120, 450));
    let Some(home_y) = ground(mid) else { return };
    let home = Vec2::new(mid as f32 + 0.5, home_y as f32);
    let x = if seen(home.x, home.y) { (cam.translation.x + h.from as f32 * RAID_FROM) as i32 } else { mid };
    let Some(y) = ground(x) else { return };
    spawn_creature(&mut commands, "pedlar", Vec2::new(x as f32 + 0.5, y as f32), move |e| {
        e.insert((crate::creatures::brain::villager::Home(home), crate::creatures::brain::villager::Routine::default()));
    });
    info!("events: the pedlar at x {:+} from the village's middle", x - mid);
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
                EventKind::Pedlar if now < h.day + PEDLAR_STAY => "A pedlar came to the village this morning. Off again tomorrow, he says.".into(),
                EventKind::Pedlar => "The pedlar's gone on. He'll be back some day.".into(),
                EventKind::Quake => format!("Did you feel the ground shake {}? They say it split open, {far} {dir} of here.", if now - h.day < 0.6 { "today" } else { "the other day" }),
                EventKind::Raid => {
                    let when = if now - h.day < 0.6 { "this evening" } else { "the other evening" };
                    format!("Orcs came at us from the {} {when}. We're mending what they broke.", if h.from < 0 { "west" } else { "east" })
                }
            };
            (h.day, line)
        })
        .collect();
    told.sort_by(|a, b| b.0.total_cmp(&a.0));
    news.0 = told.into_iter().map(|(_, l)| l).collect();
}

/// How far a walk is, said (a walk is ~5 400 cells a minute).
fn walk(cells: i32) -> String {
    match cells / 5_400 {
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
        let places = Places { spawn_x: 65_536, village: None };
        let a: Vec<_> = (0..200).filter_map(|d| roll(7, places, d, EventKind::Star)).collect();
        let b: Vec<_> = (0..200).filter_map(|d| roll(7, places, d, EventKind::Star)).collect();
        assert_eq!(a, b);
        assert!((40..100).contains(&a.len()), "{} stars in 200 nights", a.len());
        for (day, x, _) in a {
            let hour = day.fract() * 24.0;
            assert!(!(5.0..21.0).contains(&hour), "a star by day ({hour:.1} h)");
            let off = (x - 65_536).abs() as f64;
            assert!((STAR_FROM.0 - 1.0..=STAR_FROM.1).contains(&off), "{off} cells from the spawn");
        }
        // No village, no raids; with one, some evenings, at it.
        assert!((0..50).all(|d| roll(7, places, d, EventKind::Raid).is_none()));
        let village = Places { village: Some((CellPos::new(65_600, 25_470), CellPos::new(65_800, 25_530))), ..places };
        let raids: Vec<_> = (0..200).filter_map(|d| roll(7, village, d, EventKind::Raid)).collect();
        assert!((30..90).contains(&raids.len()), "{} raids in 200 days", raids.len());
        assert!(raids.iter().all(|&(day, x, from)| x == 65_700 && from.abs() == 1 && (17.0..21.0).contains(&(day.fract() * 24.0))));
    }
}
