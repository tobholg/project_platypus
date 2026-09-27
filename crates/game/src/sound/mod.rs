//! Sound (DESIGN §7e): bevy_seedling (the Firewheel audio engine) plays
//! it; almost all of it is made here, from recipes in
//! `assets/data/sounds.ron` (`synth.rs`), rendered at startup in the
//! background, a few variants of each, and again when the file is saved.
//! Any sound can be a recording instead (`file:`).
//!
//! - Three buses: effects (placed in the world: panned and fainter with
//!   distance from the camera), ambience (beds whose volume the world
//!   drives: `ambience.rs`) and music (moods crossfading), each with its
//!   volume in sounds.ron.
//! - Anything asks for an effect with a `PlaySound` (a name, where, how
//!   loud); `hooks.rs` turns the game's own messages into them.
//! - F11 mutes it all; `PLATYPUS_MUTE=1` starts muted (batches of test
//!   runs).

pub mod ambience;
pub mod hooks;
pub mod synth;

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroU32;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use bevy_seedling::context::SampleRate;
use bevy_seedling::prelude::*;
use rayon::prelude::*;
use serde::Deserialize;

use crate::camera::MainCamera;
use crate::data::{Watched, data_path, load_ron};

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SeedlingPlugins)
            // (A cell is about a tenth of a metre: a sound at the edge of the
            // view is some 15 dB down.)
            .insert_resource(DefaultSpatialScale(Vec3::splat(0.1)))
            .insert_resource(SoundBank::load())
            .init_resource::<Muted>()
            .add_message::<PlaySound>()
            .add_systems(Startup, buses)
            .add_systems(PostStartup, listen)
            .add_systems(Update, (reload, render, finish, volumes, play).chain())
            .add_plugins((ambience::AmbiencePlugin, hooks::HooksPlugin));
    }
}

/// Where sounds go: effects, ambience, music.
#[derive(NodeLabel, PartialEq, Eq, Debug, Hash, Clone)]
struct SfxBus;
#[derive(NodeLabel, PartialEq, Eq, Debug, Hash, Clone)]
struct AmbienceBus;
#[derive(NodeLabel, PartialEq, Eq, Debug, Hash, Clone)]
struct MusicBus;
/// The cave's reverb: effects are sent to it, as much as you're underground.
#[derive(NodeLabel, PartialEq, Eq, Debug, Hash, Clone)]
pub struct CaveVerb;

#[derive(PoolLabel, PartialEq, Eq, Debug, Hash, Clone)]
pub struct SfxPool;
#[derive(PoolLabel, PartialEq, Eq, Debug, Hash, Clone)]
pub struct AmbiencePool;
#[derive(PoolLabel, PartialEq, Eq, Debug, Hash, Clone)]
pub struct MusicPool;

/// Play an effect: by its name in sounds.ron, where in the world (none:
/// everywhere, centred), how loud (× its own volume), how high (× speed).
#[derive(Message, Clone, Debug)]
pub struct PlaySound {
    pub name: Cow<'static, str>,
    pub at: Option<Vec2>,
    pub volume: f32,
    pub pitch: f32,
}

impl PlaySound {
    /// `name` at `at` in the world.
    pub fn at(name: impl Into<Cow<'static, str>>, at: Vec2) -> Self {
        PlaySound { name: name.into(), at: Some(at), volume: 1.0, pitch: 1.0 }
    }

    /// `name`, not placed (the interface, what's yours).
    pub fn here(name: impl Into<Cow<'static, str>>) -> Self {
        PlaySound { name: name.into(), at: None, volume: 1.0, pitch: 1.0 }
    }

    pub fn volume(mut self, v: f32) -> Self {
        self.volume = v;
        self
    }

    pub fn pitch(mut self, p: f32) -> Self {
        self.pitch = p;
        self
    }
}

/// Fainter than this far from the camera (cells), an effect isn't played.
const HEAR: f32 = 420.0;

#[derive(Clone, Debug, Deserialize)]
pub struct Volumes {
    pub master: f32,
    pub sfx: f32,
    pub ambience: f32,
    pub music: f32,
    /// How much of every effect echoes underground (the cave reverb's send,
    /// as deep as you are).
    #[serde(default = "cave_reverb")]
    pub cave_reverb: f32,
}

fn cave_reverb() -> f32 {
    0.45
}

/// A sound: made from a recipe (`make`) or a recording (`file`, under
/// assets/), and how it's played.
#[derive(Clone, Debug, Deserialize)]
pub struct SoundDef {
    #[serde(default)]
    pub make: Option<synth::Make>,
    #[serde(default)]
    pub file: Option<String>,
    /// Takes made (each a little different), one picked each time.
    #[serde(default = "one_u32")]
    pub variants: u32,
    #[serde(default = "one")]
    pub volume: f32,
    /// Each play this much higher or lower at most (a share of its speed).
    #[serde(default)]
    pub pitch: f32,
    /// Reverb mixed in (`room`: its time, s), echoes (`echo`: each this
    /// much of the last).
    #[serde(default)]
    pub reverb: f32,
    #[serde(default = "room")]
    pub room: f32,
    #[serde(default)]
    pub echo: f32,
    /// A loop this many seconds long (0: a one-shot).
    #[serde(default)]
    pub loops: f32,
    /// At most once this often (s).
    #[serde(default)]
    pub every: f32,
}

fn one() -> f32 {
    1.0
}
fn one_u32() -> u32 {
    1
}
fn room() -> f32 {
    1.6
}

#[derive(Clone, Debug, Deserialize)]
struct SoundsFile {
    volumes: Volumes,
    sounds: BTreeMap<String, SoundDef>,
}

type Takes = Vec<(String, Vec<synth::Buf>)>;

/// Every sound, and what's been made of each so far.
#[derive(Resource)]
pub struct SoundBank {
    pub volumes: Volumes,
    pub defs: BTreeMap<String, SoundDef>,
    pub made: HashMap<String, Vec<Handle<AudioSample>>>,
    /// Bumped each time a new set of sounds is ready (loops restart).
    pub generation: u32,
    task: Option<Task<Takes>>,
    /// Waiting to be made (on start and each save).
    stale: bool,
    watch: Watched,
}

impl SoundBank {
    fn load() -> Self {
        let path = data_path("sounds.ron");
        let file: SoundsFile = load_ron(&path).unwrap_or_else(|e| {
            warn!("sounds.ron: {e}");
            SoundsFile { volumes: Volumes { master: 1.0, sfx: 1.0, ambience: 1.0, music: 1.0, cave_reverb: cave_reverb() }, sounds: BTreeMap::new() }
        });
        SoundBank { volumes: file.volumes, defs: file.sounds, made: HashMap::new(), generation: 0, task: None, stale: true, watch: Watched::new(path) }
    }

    /// A sound's takes, if it's ready.
    pub fn takes(&self, name: &str) -> Option<&[Handle<AudioSample>]> {
        self.made.get(name).map(|v| v.as_slice()).filter(|v| !v.is_empty())
    }
}

/// Everything muted (F11; `PLATYPUS_MUTE=1` from the start).
#[derive(Resource)]
pub struct Muted(pub bool);

impl Default for Muted {
    fn default() -> Self {
        Muted(std::env::var("PLATYPUS_MUTE").is_ok())
    }
}

fn buses(mut commands: Commands) {
    // The cave: a big, darkish room; silent until you're underground.
    commands.spawn((VolumeNode { volume: Volume::SILENT, ..default() }, CaveVerb)).chain_node(FreeverbNode { room_size: 0.88, damping: 0.55, width: 0.9, ..default() });
    commands.spawn((VolumeNode::default(), SfxBus)).connect(MainBus).connect(CaveVerb);
    commands.spawn((VolumeNode::default(), AmbienceBus));
    commands.spawn((VolumeNode::default(), MusicBus));
    commands.spawn((SamplerPool(SfxPool), PoolSize(16..=64), sample_effects![SpatialBasicNode::default()])).connect(SfxBus);
    commands.spawn((SamplerPool(AmbiencePool), PoolSize(8..=16), sample_effects![VolumeNode::default(), SpatialBasicNode::default()])).connect(AmbienceBus);
    commands.spawn((SamplerPool(MusicPool), PoolSize(4..=8), sample_effects![VolumeNode::default()])).connect(MusicBus);
}

/// The camera hears.
fn listen(mut commands: Commands, cam: Query<Entity, With<MainCamera>>) {
    for e in &cam {
        commands.entity(e).insert(SpatialListener2D);
    }
}

type Buses<'a> = (&'a mut VolumeNode, Has<MainBus>, Has<SfxBus>, Has<AmbienceBus>, Has<MusicBus>);

/// The buses' volumes (sounds.ron), muted or not (F11).
fn volumes(bank: Res<SoundBank>, keys: Res<ButtonInput<KeyCode>>, mut muted: ResMut<Muted>, mut nodes: Query<Buses>) {
    if keys.just_pressed(KeyCode::F11) {
        muted.0 = !muted.0;
        info!("sound {}", if muted.0 { "muted" } else { "on" });
    }
    let v = &bank.volumes;
    for (mut node, main, sfx, amb, music) in &mut nodes {
        let want = if main {
            if muted.0 { 0.0 } else { v.master }
        } else if sfx {
            v.sfx
        } else if amb {
            v.ambience
        } else if music {
            v.music
        } else {
            continue;
        };
        if (node.volume.linear() - want).abs() > 1e-4 {
            node.volume = Volume::Linear(want);
        }
    }
}

/// sounds.ron saved: made again.
fn reload(mut bank: ResMut<SoundBank>) {
    if !bank.bypass_change_detection().watch.changed() {
        return;
    }
    let fresh = SoundBank::load();
    bank.volumes = fresh.volumes;
    bank.defs = fresh.defs;
    bank.stale = true;
    info!("sounds.ron reloaded");
}

/// A seed per sound and take: the same file makes the same sounds.
fn seed(name: &str, take: u32) -> u64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)) ^ ((take as u64 + 1) << 32)
}

/// Make one sound's takes at `sr`.
pub fn make(name: &str, def: &SoundDef, sr: f32) -> Vec<synth::Buf> {
    let Some(recipe) = &def.make else { return Vec::new() };
    (0..def.variants.max(1))
        .map(|take| {
            let mut b = synth::render(recipe, sr, seed(name, take), def.loops);
            let tail = def.loops <= 0.0;
            synth::echo(&mut b, 0.23, def.echo, sr, tail);
            synth::reverb(&mut b, def.reverb, 20.0, def.room, sr, tail);
            // (Loops to a steady loudness; one-shots to a peak.)
            if def.loops > 0.0 { synth::level(&mut b, 0.0, 0.2) } else { synth::level(&mut b, 0.9, 0.0) }
            b
        })
        .collect()
}

/// Start making what's stale, in the background, once the audio's running
/// (its sample rate known).
fn render(mut bank: ResMut<SoundBank>, rate: Option<Res<SampleRate>>) {
    let Some(rate) = rate else { return };
    if !bank.stale || bank.task.is_some() {
        return;
    }
    bank.stale = false;
    let sr = rate.get().get() as f32;
    let defs: Vec<(String, SoundDef)> = bank.defs.iter().filter(|(_, d)| d.make.is_some()).map(|(n, d)| (n.clone(), d.clone())).collect();
    bank.task = Some(AsyncComputeTaskPool::get().spawn(async move {
        let started = std::time::Instant::now();
        let takes: Takes = defs.par_iter().map(|(n, d)| (n.clone(), make(n, d, sr))).collect();
        info!("sounds: {} made in {:.2} s", takes.len(), started.elapsed().as_secs_f32());
        takes
    }));
}

/// Made: into samples (and the recordings loaded).
fn finish(mut bank: ResMut<SoundBank>, mut samples: ResMut<Assets<AudioSample>>, rate: Option<Res<SampleRate>>, server: Res<AssetServer>) {
    let Some(task) = bank.task.as_mut() else { return };
    let Some(takes) = block_on(poll_once(task)) else { return };
    bank.task = None;
    let sr = rate.map_or(48_000, |r| r.get().get());
    let rate = NonZeroU32::new(sr).unwrap_or(NonZeroU32::MIN);
    let mut made: HashMap<String, Vec<Handle<AudioSample>>> = HashMap::new();
    for (name, bufs) in takes {
        let handles = bufs.into_iter().map(|b| samples.add(AudioSample::new(vec![b.l, b.r], rate))).collect();
        made.insert(name, handles);
    }
    for (name, def) in &bank.defs {
        if let Some(file) = &def.file {
            made.insert(name.clone(), vec![server.load(file.clone())]);
        }
    }
    bank.made = made;
    bank.generation += 1;
}

/// Effects asked for: played, placed where they happen.
#[allow(clippy::too_many_arguments)]
fn play(
    mut commands: Commands,
    mut asks: MessageReader<PlaySound>,
    bank: Res<SoundBank>,
    time: Res<Time<Real>>,
    cam: Query<&GlobalTransform, With<MainCamera>>,
    mut last: Local<HashMap<String, f32>>,
    mut rng: Local<Option<synth::Rng>>,
    mut unknown: Local<std::collections::HashSet<String>>,
) {
    let rng = rng.get_or_insert_with(|| synth::Rng::new(7));
    let now = time.elapsed_secs();
    let heard = cam.iter().next().map(|t| t.translation().truncate());
    for ask in asks.read() {
        let name = ask.name.as_ref();
        let Some(def) = bank.defs.get(name) else {
            if unknown.insert(name.to_string()) {
                warn!("sound `{name}`: not in sounds.ron");
            }
            continue;
        };
        let Some(takes) = bank.takes(name) else { continue };
        if def.every > 0.0 && last.get(name).is_some_and(|&t| now - t < def.every) {
            continue;
        }
        if let (Some(at), Some(cam)) = (ask.at, heard)
            && at.distance(cam) > HEAR
        {
            continue;
        }
        last.insert(name.to_string(), now);
        if std::env::var("PLATYPUS_SOUND_LOG").is_ok() {
            info!("sound: {name}{}", ask.at.map_or(String::new(), |a| format!(" at ({:.0}, {:.0})", a.x, a.y)));
        }
        let take = takes[(rng.unit() * takes.len() as f32) as usize % takes.len()].clone();
        let speed = (ask.pitch * (1.0 + def.pitch * (rng.unit() * 2.0 - 1.0))).max(0.05) as f64;
        let mut e = commands.spawn((SfxPool, SamplePlayer::new(take).with_volume(Volume::Linear(def.volume * ask.volume)), PlaybackSettings { speed, ..default() }));
        if let Some(at) = ask.at {
            e.insert(Transform::from_translation(at.extend(0.0)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every sound in sounds.ron renders: finite, heard, loops their
    /// length. (`PLATYPUS_SOUND_DUMP=dir`: each first take written there as
    /// a WAV, to listen to or look at.)
    #[test]
    fn every_sound_in_the_file_renders() {
        let file: SoundsFile = load_ron(&data_path("sounds.ron")).expect("sounds.ron");
        let sr = 48_000.0;
        let dump = std::env::var("PLATYPUS_SOUND_DUMP").ok();
        let made: Vec<(String, Vec<synth::Buf>)> = file.sounds.par_iter().map(|(n, d)| (n.clone(), make(n, d, sr))).collect();
        for (name, takes) in made {
            let def = &file.sounds[&name];
            assert_eq!(takes.len(), def.variants.max(1) as usize, "{name}: its takes");
            for b in &takes {
                assert!(!b.is_empty() && b.l.iter().chain(&b.r).all(|x| x.is_finite()), "{name}: finite");
                assert!(b.peak() > 0.05, "{name}: heard (peak {})", b.peak());
                if def.loops > 0.0 {
                    assert_eq!(b.len(), (def.loops * sr) as usize, "{name}: a loop keeps its length");
                }
            }
            if let Some(dir) = &dump {
                write_wav(&std::path::Path::new(dir).join(format!("{name}.wav")), &takes[0], sr as u32);
            }
        }
    }

    fn write_wav(path: &std::path::Path, b: &synth::Buf, sr: u32) {
        let n = b.len() as u32;
        let mut out = Vec::with_capacity(44 + n as usize * 4);
        let le32 = |v: u32| v.to_le_bytes();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&le32(36 + n * 4));
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&le32(16));
        out.extend_from_slice(&[1, 0, 2, 0]);
        out.extend_from_slice(&le32(sr));
        out.extend_from_slice(&le32(sr * 4));
        out.extend_from_slice(&[4, 0, 16, 0]);
        out.extend_from_slice(b"data");
        out.extend_from_slice(&le32(n * 4));
        for i in 0..b.len() {
            for x in [b.l[i], b.r[i]] {
                out.extend_from_slice(&((x.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
            }
        }
        let _ = std::fs::create_dir_all(path.parent().unwrap_or(path));
        std::fs::write(path, out).expect("wav");
    }
}
