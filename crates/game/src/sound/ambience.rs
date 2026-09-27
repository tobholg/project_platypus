//! Ambience and music, driven by the world around the camera.
//!
//! A few times a second the cells around the view are sampled: what burns
//! (and where, for panning), lava, water that moved in the last few ticks,
//! rain falling; and whether there's rock overhead (underground). Each
//! looping bed (`*_bed` in sounds.ron) fades toward what that calls for:
//! a fire's roar and crackle as it spreads, a cave's rumble and air under
//! rock, wind and rain in the open. Underground, now and then a drop falls
//! from a ceiling in view (`drip`).
//!
//! Music: a loop per mood (`music_day`, `music_night`, `music_cave`), all
//! playing, crossfaded by where you are and the time of day; and on the
//! surface, now and then (every 40-90 s), a melody over it: one of the
//! `melody_day_*` or `melody_night_*` takes (`PLATYPUS_MELODY=1`: one
//! soon and every 12 s, to hear them).

use bevy::prelude::*;
use bevy_seedling::prelude::*;
use platypus_sim::{CellPos, Kind, cell::flags};

use super::{AmbiencePool, MusicPool, PlaySound, SoundBank, synth::Rng};
use crate::camera::MainCamera;
use crate::world::SimWorld;

pub struct AmbiencePlugin;

impl Plugin for AmbiencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Heard>().add_systems(Update, (start, listen, rockets, drips, melodies, fade, report).chain().after(super::finish));
    }
}

/// The beds and the music, and what drives each.
const BEDS: [(&str, Drive); 10] = [
    ("rocket_bed", Drive::Rocket),
    ("fire_bed", Drive::Fire),
    ("lava_bed", Drive::Lava),
    ("water_bed", Drive::Water),
    ("rain_bed", Drive::Rain),
    ("wind_bed", Drive::Wind),
    ("cave_bed", Drive::Cave),
    ("music_day", Drive::Day),
    ("music_night", Drive::Night),
    ("music_cave", Drive::Deep),
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Drive {
    Fire,
    Lava,
    Water,
    Rain,
    Wind,
    Cave,
    Day,
    Night,
    Deep,
    Rocket,
}

/// A looping bed: how loud it is now (0..1 of its own volume).
#[derive(Component)]
struct Bed {
    drive: Drive,
    level: f32,
    volume: f32,
}

/// What's around the camera (0..1 each, but `fire_at`: where the fire is).
#[derive(Resource, Default, Debug)]
pub struct Heard {
    pub fire: f32,
    pub fire_at: Vec2,
    pub lava: f32,
    pub water: f32,
    pub rain: f32,
    /// Rock overhead: 0 in the open, 1 deep in a cave.
    pub under: f32,
    /// 0 night .. 1 day.
    pub day: f32,
    /// The player's rocket boots firing (seconds of it left), and where.
    pub rocket: f32,
    pub rocket_at: Vec2,
    since: f32,
}

/// How far around the camera it listens (cells).
const REACH: f32 = 200.0;
/// Seconds between listens.
const EVERY: f32 = 0.25;

/// A new set of sounds: the beds (re)started, silent, to fade in.
fn start(mut commands: Commands, bank: Res<SoundBank>, mut seen: Local<u32>, beds: Query<Entity, With<Bed>>) {
    if bank.generation == *seen {
        return;
    }
    *seen = bank.generation;
    for e in &beds {
        commands.entity(e).despawn();
    }
    for (name, drive) in BEDS {
        let (Some(def), Some(takes)) = (bank.defs.get(name), bank.takes(name)) else { continue };
        let player = SamplePlayer::new(takes[0].clone()).looping();
        let quiet = VolumeNode { volume: Volume::SILENT, ..default() };
        let bed = Bed { drive, level: 0.0, volume: def.volume };
        if matches!(drive, Drive::Day | Drive::Night | Drive::Deep) {
            commands.spawn((MusicPool, player, bed, sample_effects![quiet]));
        } else {
            commands.spawn((AmbiencePool, player, bed, Transform::default(), sample_effects![quiet, SpatialBasicNode::default()]));
        }
    }
}

/// On the surface, a melody now and then: the time of day's takes, never
/// the same one twice running.
fn melodies(time: Res<Time<Real>>, heard: Res<Heard>, bank: Res<SoundBank>, mut next: Local<Option<f32>>, mut last: Local<String>, mut rng: Local<Option<Rng>>, mut out: MessageWriter<PlaySound>) {
    let rng = rng.get_or_insert_with(|| Rng::new(0x3e10));
    let now = time.elapsed_secs();
    let test = std::env::var("PLATYPUS_MELODY").is_ok();
    let when = *next.get_or_insert(if test { 2.0 } else { 15.0 + 20.0 * rng.unit() });
    if now < when {
        return;
    }
    *next = Some(now + if test { 12.0 } else { 40.0 + 50.0 * rng.unit() });
    // (Not underground: the cave's music is its own.)
    if heard.under > 0.3 {
        return;
    }
    let kind = if heard.day > 0.5 { "melody_day_" } else { "melody_night_" };
    let takes: Vec<&String> = bank.defs.keys().filter(|n| n.starts_with(kind) && **n != *last).collect();
    if takes.is_empty() {
        return;
    }
    let name = takes[(rng.unit() * takes.len() as f32) as usize % takes.len()].clone();
    if test {
        info!("melody: {name}");
    }
    out.write(PlaySound::music(name.clone()).volume(1.0 - heard.under));
    *last = name;
}

/// Sample the world around the camera.
fn listen(time: Res<Time<Real>>, sim: Res<SimWorld>, day: Option<Res<crate::light::Daylight>>, cam: Query<&GlobalTransform, With<MainCamera>>, mut heard: ResMut<Heard>) {
    heard.since += time.delta_secs();
    if heard.since < EVERY {
        return;
    }
    heard.since = 0.0;
    let Some(c) = cam.iter().next().map(|t| t.translation().truncate()) else { return };
    let world = &sim.world;
    let mats = world.materials();
    let tick = world.tick() as u8;
    let (mut fire, mut fire_at, mut lava, mut water) = (0.0f32, Vec2::ZERO, 0.0f32, 0.0f32);
    // (Every other cell each way: a quarter of them, counted four times.)
    let (rx, ry) = (REACH as i32, (REACH * 0.7) as i32);
    for y in (-ry..=ry).step_by(2) {
        for x in (-rx..=rx).step_by(2) {
            let p = CellPos::new(c.x as i32 + x, c.y as i32 + y);
            let Some(cell) = world.get(p) else { continue };
            let ph = mats.phys(cell.material);
            if ph.kind == Kind::Empty {
                continue;
            }
            let d = ((x * x + y * y) as f32).sqrt() / REACH;
            if d >= 1.0 {
                continue;
            }
            let w = 4.0 * (1.0 - d) * (1.0 - d);
            if ph.kind == Kind::Fire || cell.flags & flags::BURNING != 0 {
                fire += w;
                fire_at += Vec2::new(x as f32, y as f32) * w;
            } else if ph.kind == Kind::Liquid && ph.hot {
                lava += w;
            } else if ph.kind == Kind::Liquid && tick.wrapping_sub(cell.clock) < 4 {
                water += w;
            }
        }
    }
    let rain = world.particles().iter().filter(|p| p.vel[1] < -0.3 && mats.phys(p.cell.material).kind == Kind::Liquid && (Vec2::from(p.pos) - c).length() < REACH).count() as f32;
    // Rock overhead: columns about the camera, looking up from open air (a
    // hillside beside you, or a tree, isn't a roof).
    let rock = |cell: platypus_sim::Cell| {
        let name = mats.def(cell.material).name.as_str();
        mats.phys(cell.material).kind == Kind::Static && !(name.contains("wood") || name.contains("log") || name.contains("plank") || name.contains("leaf"))
    };
    let (mut roofed, mut open) = (0, 0);
    for k in -3..=3 {
        let x = c.x as i32 + k * 20;
        if world.get(CellPos::new(x, c.y as i32)).is_some_and(|cell| mats.phys(cell.material).kind != Kind::Empty) {
            continue;
        }
        open += 1;
        if (4..160).step_by(2).any(|up| world.get(CellPos::new(x, c.y as i32 + up)).is_some_and(rock)) {
            roofed += 1;
        }
    }
    let roof = if open > 0 { roofed as f32 / open as f32 } else { heard.under };
    // And how deep below the ground as generated: well down, it's
    // underground whatever's straight overhead (a shaft dug up to the sky,
    // a chasm).
    let deep = sim.generator.surface_hint(c.x as i32).map_or(0.0, |s| ((s as f32 - c.y - 15.0) / 45.0).clamp(0.0, 1.0));
    let covered = roof.max(deep);
    let sat = |x: f32, k: f32| 1.0 - (-x / k).exp();
    heard.fire = sat(fire, 60.0);
    heard.fire_at = if fire > 0.0 { fire_at / fire } else { Vec2::ZERO };
    heard.lava = sat(lava, 400.0);
    heard.water = sat(water, 150.0);
    heard.rain = sat(rain, 250.0) * (1.0 - covered);
    heard.under = covered;
    heard.day = day.map_or(1.0, |d| {
        let sun = -(d.time * std::f32::consts::TAU).cos();
        ((sun + 0.12) / 0.42).clamp(0.0, 1.0)
    });
}

/// The player's rocket boots firing (the exhaust, `Rocketed`, each tick
/// they do).
fn rockets(time: Res<Time<Real>>, mut fired: MessageReader<crate::actors::Rocketed>, player: Query<Entity, With<crate::actors::player::LocalPlayer>>, mut heard: ResMut<Heard>) {
    heard.rocket = (heard.rocket - time.delta_secs()).max(0.0);
    let me = player.single().ok();
    for r in fired.read() {
        if Some(r.entity) == me {
            heard.rocket = 0.12;
            heard.rocket_at = r.at;
        }
    }
}

/// Underground, now and then a drop from a ceiling in view.
fn drips(time: Res<Time<Real>>, sim: Res<SimWorld>, heard: Res<Heard>, cam: Query<&GlobalTransform, With<MainCamera>>, mut next: Local<f32>, mut rng: Local<Option<Rng>>, mut out: MessageWriter<PlaySound>) {
    let rng = rng.get_or_insert_with(|| Rng::new(99));
    *next -= time.delta_secs();
    if *next > 0.0 {
        return;
    }
    // (More often by water.)
    *next = rng.unit() * 3.5 + 1.2 - heard.water * 0.8;
    if heard.under < 0.5 {
        return;
    }
    let Some(c) = cam.iter().next().map(|t| t.translation().truncate()) else { return };
    let world = &sim.world;
    let mats = world.materials();
    let open = |p: CellPos| world.get(p).is_some_and(|cell| matches!(mats.phys(cell.material).kind, Kind::Empty | Kind::Gas));
    let solid = |p: CellPos| world.get(p).is_some_and(|cell| mats.phys(cell.material).kind == Kind::Static);
    for _ in 0..24 {
        let p = CellPos::new(c.x as i32 + (rng.unit() * 360.0 - 180.0) as i32, c.y as i32 + (rng.unit() * 220.0 - 110.0) as i32);
        if !open(p) {
            continue;
        }
        // Up to the ceiling above it.
        if let Some(up) = (1..40).find(|&k| !open(CellPos::new(p.x, p.y + k))) {
            let roof = CellPos::new(p.x, p.y + up);
            if solid(roof) {
                out.write(PlaySound::at("drip", Vec2::new(p.x as f32 + 0.5, (p.y + up) as f32 - 1.0)).volume(0.6 + 0.4 * rng.unit()));
                return;
            }
        }
    }
}

/// `PLATYPUS_SOUND_LOG=1`: what's heard and each bed's level, every 2 s.
fn report(time: Res<Time<Real>>, heard: Res<Heard>, beds: Query<&Bed>, mut since: Local<f32>) {
    if std::env::var("PLATYPUS_SOUND_LOG").is_err() {
        return;
    }
    *since += time.delta_secs();
    if *since < 2.0 {
        return;
    }
    *since = 0.0;
    let levels: Vec<String> = beds.iter().filter(|b| b.level > 0.01).map(|b| format!("{:?} {:.2}", b.drive, b.level)).collect();
    info!(
        "sound: fire {:.2} lava {:.2} water {:.2} rain {:.2} under {:.2} day {:.2}; playing: {}",
        heard.fire,
        heard.lava,
        heard.water,
        heard.rain,
        heard.under,
        heard.day,
        levels.join(", ")
    );
}

/// Each bed toward what the world calls for; the cave's reverb as deep as
/// you are.
#[allow(clippy::too_many_arguments)]
fn fade(time: Res<Time<Real>>, heard: Res<Heard>, bank: Res<SoundBank>, mut verb: Query<&mut VolumeNode, With<super::CaveVerb>>, cam: Query<&GlobalTransform, (With<MainCamera>, Without<Bed>)>, mut beds: Query<(&mut Bed, &SampleEffects, Option<&mut Transform>)>, mut vols: Query<&mut VolumeNode, Without<super::CaveVerb>>) {
    let dt = time.delta_secs();
    let c = cam.iter().next().map_or(Vec2::ZERO, |t| t.translation().truncate());
    let open = 1.0 - heard.under;
    for mut v in &mut verb {
        let want = heard.under * bank.volumes.cave_reverb;
        let now = v.volume.linear();
        let next = now + (want - now) * (dt * 1.5).min(1.0);
        if (next - now).abs() > 1e-4 {
            v.volume = Volume::Linear(next);
        }
    }
    for (mut bed, effects, tf) in &mut beds {
        let want = match bed.drive {
            Drive::Fire => heard.fire,
            Drive::Lava => heard.lava,
            Drive::Water => heard.water,
            Drive::Rain => heard.rain,
            Drive::Wind => open * 0.6,
            Drive::Cave => heard.under,
            Drive::Day => open * heard.day,
            Drive::Night => open * (1.0 - heard.day),
            Drive::Deep => heard.under,
            Drive::Rocket => if heard.rocket > 0.0 { 1.0 } else { 0.0 },
        };
        // (Music eases slower than the world's sounds; a rocket at once.)
        let rate = match bed.drive {
            Drive::Day | Drive::Night | Drive::Deep => 0.4,
            Drive::Rocket => 14.0,
            _ => 2.0,
        };
        bed.level += (want - bed.level) * (rate * dt).min(1.0);
        // A fire sounds from where it is (panned; its loudness is ours).
        if let Some(mut tf) = tf {
            tf.translation = match bed.drive {
                Drive::Fire => c + heard.fire_at * 0.3,
                Drive::Rocket => heard.rocket_at,
                _ => c,
            }
            .extend(0.0);
        }
        if let Ok(mut v) = vols.get_effect_mut(effects) {
            let gain = bed.level * bed.volume;
            if (v.volume.linear() - gain).abs() > 1e-3 {
                v.volume = Volume::Linear(gain);
            }
        }
    }
}
