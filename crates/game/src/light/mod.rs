//! Lighting and the day (SPEC §4.1). Every frame, a light grid over the view
//! is filled from the cells (`grid.rs`), lit by the sky, the player's lantern
//! and flashlight, burning and glowing things, flying embers, blasts and
//! lightning; then drawn over the scene twice: multiplied (what isn't lit is
//! dark) and added (a haze around what glows). Rendering only: the sim never
//! reads it, so it costs co-op nothing.
//!
//! What the player carries for light is what it wears: in the off hand a
//! torch, a pocket flashlight, a flashlight or a lantern; on its head a
//! mining helmet's lamp (`LightKind`, items.ron). A beam has a dim, wide
//! spill about its hot spot and a glow at its lens (what's right by you
//! shows), and creatures' eyes shine back in it (`shine`). Keys: F8 +3
//! hours, F9 lighting on/off. `PLATYPUS_HOUR=19` starts at that hour.

pub mod grid;
pub mod torch;

pub use torch::{TorchArt, plant_torch};

use std::time::{Duration, Instant};

use bevy::image::ImageSampler;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, Extent3d, RenderPipelineDescriptor,
    SpecializedMeshPipelineError, TextureDimension, TextureFormat,
};
use bevy::asset::RenderAssetUsages;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPlugin};
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use platypus_sim::cell::flags;
use platypus_sim::{CellPos, Landing};
use serde::Deserialize;

use crate::creatures::Kinematics;
use crate::creatures::player::LocalPlayer;
use crate::camera::{CursorWorld, MainCamera, Zoom};
use crate::data::{Watched, data_path, load_ron};
use crate::fx::{Explosion, Lightning, SkyFlash};
use crate::render::SKY_COLOR;
use crate::world::{ChunkLoader, SimWorld, TICK_HZ};
use grid::{LightGrid, Params, Rgb};

pub struct LightPlugin;

/// Over everything in the world (creatures included); under the HUD.
const Z_LIGHT: f32 = 15.0;
const Z_GLOW: f32 = 15.5;
/// Cells of light grid beyond the view, so lights just off screen shine in.
const MARGIN: f32 = 72.0;

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct LampCfg {
    pub color: (u8, u8, u8),
    pub strength: f32,
    /// A haze around it over the dark (a share of its light), as glowing
    /// cells have.
    #[serde(default)]
    pub haze: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct BeamCfg {
    pub color: (u8, u8, u8),
    pub strength: f32,
    pub range: f32,
    /// Degrees, edge to edge.
    pub angle: f32,
}

/// `assets/data/lighting.ron`.
#[derive(Resource, Clone, Debug, Deserialize)]
pub struct LightSettings {
    pub ambient: f32,
    pub air_falloff: f32,
    pub iterations: usize,
    pub rim: f32,
    pub glow_haze: f32,
    pub day_minutes: f32,
    pub start_hour: f32,
    pub moonlight: f32,
    pub lantern: LampCfg,
    pub flashlight: BeamCfg,
    pub flashlight_small: BeamCfg,
    pub torch: LampCfg,
    /// A lantern held: steady, all round.
    pub lamp: LampCfg,
    /// A mining helmet's lamp.
    pub headlamp: BeamCfg,
    pub spill: SpillCfg,
    pub glowstick: StickCfg,
    pub fire: torch::FireLook,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct StickCfg {
    pub strength: f32,
    pub secs: f32,
    #[serde(default)]
    pub haze: f32,
}

/// Anything that gives off light: a planted torch, a glow stick (later a
/// lantern on a post, glowing eyes). `flicker` 0..1 makes it waver like fire.
#[derive(Component, Clone, Copy, Debug)]
pub struct LightSource {
    pub color: Rgb,
    pub flicker: f32,
}

/// A light that hazes over the dark around it too (a torch, a glow stick):
/// this share of its light.
#[derive(Component, Clone, Copy, Debug)]
pub struct Haze(pub f32);

/// A glowing light (glowing eyes; a haze over the dark, as glowing cells
/// have): steady, or with a `period` swelling and fading (a firefly), out
/// of step with the others (`phase`).
#[derive(Component, Clone, Copy, Debug)]
pub struct Glow {
    pub color: Rgb,
    pub period: f32,
    pub phase: f32,
}

fn glow(time: Res<Time>, mut q: Query<(&Glow, &mut LightSource)>) {
    let t = time.elapsed_secs();
    for (g, mut src) in &mut q {
        if g.period <= 0.0 {
            src.color = g.color;
            continue;
        }
        let s = (0.5 + 0.5 * (t / g.period.max(0.1) * std::f32::consts::TAU + g.phase).sin()).powi(3);
        src.color = g.color.map(|c| c * (0.08 + 0.92 * s));
    }
}

/// A torch planted in the world (G).
#[derive(Component)]
pub struct PlantedTorch;

#[derive(Resource)]
struct SettingsWatch(Watched);

/// The time of day and what it does to the sky.
#[derive(Resource, Default)]
pub struct Daylight {
    /// 0 = midnight, 0.25 dawn, 0.5 noon, 0.75 dusk.
    pub time: f32,
    /// Days gone by (the time of day its fraction).
    pub days: f64,
    /// Hours the dev key (or a scenario) has skipped.
    pub skipped: f32,
    /// Light from the sky right now (sun or moon, cloud, lightning).
    pub sky: Rgb,
}

impl Daylight {
    /// The moon's phase tonight (0 new, 0.5 full): eight nights round, the
    /// first full (Terraria's). A night is the one begun at dusk: after
    /// midnight it's still the last day's. (`PLATYPUS_MOON`=0..7 fixes it:
    /// 4 full, 0 new.)
    pub fn moon_phase(&self) -> f32 {
        let night = (self.days + 0.25).floor() as i64 - 1;
        let k = std::env::var("PLATYPUS_MOON").ok().and_then(|v| v.parse::<i64>().ok()).unwrap_or(night + 4);
        k.rem_euclid(8) as f32 / 8.0
    }

    /// "08:30".
    pub fn clock(&self) -> String {
        let minutes = (self.time * 24.0 * 60.0) as u32;
        format!("{:02}:{:02}", minutes / 60, minutes % 60)
    }
}

#[derive(Resource)]
pub struct LightToggles {
    pub enabled: bool,
    /// What the player holds for light in its off hand (besides its faint
    /// lantern): what it wears there (`from_gear`), or what a scenario set.
    pub carry: Carry,
    /// A mining helmet's lamp on its head.
    pub headlamp: bool,
}

/// What's held in the off hand, for light.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Carry {
    #[default]
    Nothing,
    SmallBeam,
    BigBeam,
    Torch,
    Lantern,
}

/// A light an item gives, worn (items.ron `light`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum LightKind {
    /// In the off hand: fire, all round.
    Torch,
    /// In the off hand: a short beam.
    Pocket,
    /// In the off hand: a long bright beam.
    Flashlight,
    /// In the off hand: a steady glow all round.
    Lantern,
    /// On the head (a mining helmet): a narrow beam where you look.
    Headlamp,
}

impl LightKind {
    /// Held in the off hand (not worn on the head).
    pub fn held(self) -> bool {
        self != LightKind::Headlamp
    }

    pub fn carry(self) -> Carry {
        match self {
            LightKind::Torch => Carry::Torch,
            LightKind::Pocket => Carry::SmallBeam,
            LightKind::Flashlight => Carry::BigBeam,
            LightKind::Lantern => Carry::Lantern,
            LightKind::Headlamp => Carry::Nothing,
        }
    }
}

/// What the player wears, for light: its off hand's, its head's (when what
/// it wears changes: a scenario's own choice stands till then).
fn from_gear(items: Option<Res<crate::hands::items::Items>>, mut toggles: ResMut<LightToggles>, worn: Query<&crate::gear::Equipment, (With<LocalPlayer>, Changed<crate::gear::Equipment>)>) {
    let (Some(items), Ok(eq)) = (items, worn.single()) else { return };
    let light = |i: usize| eq.worn[i].and_then(|s| items.def(s.item).light);
    let carry = light(crate::gear::OFF_HAND).filter(|l| l.held()).map_or(Carry::Nothing, LightKind::carry);
    let headlamp = light(0) == Some(LightKind::Headlamp);
    if toggles.carry != carry || toggles.headlamp != headlamp {
        toggles.carry = carry;
        toggles.headlamp = headlamp;
    }
}

/// A beam's spill: a dim, wide cone about its hot spot (real flashlights
/// have one), this share of its light, this wide (degrees) and far (cells).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct SpillCfg {
    pub share: f32,
    pub angle: f32,
    pub range: f32,
    /// The lamp itself, a glow all round it (the hand, the lens's rim):
    /// this share of the beam's light.
    pub glow: f32,
}

/// Eye-shine: how much a creature's eyes catch the player's beams (0..1,
/// eased): in one, nearer brighter (`sight::eyes` draws its eyes so).
#[derive(Component, Default)]
pub struct Shine(pub f32);

/// Seconds for eyes to flare as a beam finds them, and to fade once it's off.
const SHINE_RISE: f32 = 0.06;
const SHINE_FADE: f32 = 0.3;

fn shine(mut commands: Commands, time: Res<Time>, cones: Res<Cones>, mut q: Query<(Entity, &Kinematics, Option<&mut Shine>), (With<crate::creatures::Creature>, Without<LocalPlayer>)>) {
    let dt = time.delta_secs();
    for (e, k, shine) in &mut q {
        let eye = k.body.pos + Vec2::Y * k.body.half.y * 0.5;
        // (Its eyes in a cone: nearer, brighter; its edge, a little less.)
        let target = cones
            .0
            .iter()
            .map(|&(from, dir, half, range, _)| {
                let d = eye - from;
                let dist = d.length();
                if dist > range || dist < 1.0 {
                    return 0.0;
                }
                let off = d.normalize().dot(dir).clamp(-1.0, 1.0).acos();
                let edge = (1.0 - (off - half * 0.5).max(0.0) / (half * 0.75)).clamp(0.0, 1.0);
                edge * (1.0 - dist / range).sqrt()
            })
            .fold(0.0f32, f32::max);
        let Some(mut s) = shine else {
            commands.entity(e).insert(Shine(target));
            continue;
        };
        let rate = if target > s.0 { dt / SHINE_RISE } else { dt / SHINE_FADE };
        s.0 = if target > s.0 { (s.0 + rate).min(target) } else { (s.0 - rate).max(target) };
    }
}

/// Where the player's beams play this frame (from, dir, half angle,
/// range, colour): what eyes shine back in.
#[derive(Resource, Default)]
pub struct Cones(pub Vec<(Vec2, Vec2, f32, f32, Rgb)>);

/// A brief light: a blast, a lightning strike.
struct Flash {
    at: Vec2,
    color: Rgb,
    age: f32,
    life: f32,
}

#[derive(Resource, Default)]
struct Flashes(Vec<Flash>);

#[derive(Resource)]
struct Overlay {
    light: Entity,
    glow: Entity,
    light_material: Handle<LightMultiply>,
    glow_material: Handle<LightAdd>,
    size: UVec2,
}

/// A light grid being solved in the background, and where it goes.
struct Solved {
    /// The grid, for easing the next frame from.
    previous: grid::Previous,
    /// Where gems and the like glint this frame, and in what colour.
    glints: Vec<(CellPos, grid::Rgb)>,
    light: Vec<u8>,
    glow: Vec<u8>,
    size: UVec2,
    center: Vec2,
    extent: Vec2,
    took: Duration,
}

/// The solve runs on the async pool while the frame renders; its result is
/// shown the next frame (world-anchored, so the lag doesn't show).
#[derive(Resource, Default)]
struct Pending(Option<Task<Solved>>, Option<grid::Previous>);

/// Time spent on lighting (the HUD shows it).
#[derive(Resource, Default)]
pub struct LightMetrics {
    /// On the frame: reading the world and seeding.
    pub time: Duration,
    /// In the background: spreading and encoding.
    pub solve: Duration,
    pub texels: usize,
}

impl Plugin for LightPlugin {
    fn build(&self, app: &mut App) {
        let path = data_path("lighting.ron");
        let settings: LightSettings = load_ron(&path).unwrap_or_else(|e| panic!("{e}"));
        // PLATYPUS_HOUR=19 starts at that hour instead (screenshots, testing).
        let skip = std::env::var("PLATYPUS_HOUR").ok().and_then(|h| h.parse::<f32>().ok()).map_or(0.0, |h| (h - settings.start_hour).rem_euclid(24.0));
        app.add_plugins((Material2dPlugin::<LightMultiply>::default(), Material2dPlugin::<LightAdd>::default()))
            .insert_resource(settings)
            .insert_resource(SettingsWatch(Watched::new(path)))
            // PLATYPUS_NOLIGHT=1 starts with lighting off (F9 toggles).
            .insert_resource(LightToggles { enabled: std::env::var("PLATYPUS_NOLIGHT").is_err(), carry: Carry::Nothing, headlamp: false })
            .init_resource::<Cones>()
            .add_systems(Update, (from_gear, shine))
            .init_resource::<Gloom>()
            .insert_resource(Daylight { skipped: skip, ..default() })
            .init_resource::<Flashes>()
            .init_resource::<LightMetrics>()
            .init_resource::<Pending>()
            .add_systems(Startup, (spawn_overlay, torch::load_art))
            .add_systems(Update, (reload_settings, keys, collect_flashes, glow, torch::hold.after(crate::creatures::body::animation::animate), torch::burn))
            .init_resource::<torch::OffLight>()
            .init_resource::<Glints>()
            .add_systems(Update, glimmer)
            .add_systems(Startup, torch::load_off_hand)
            .add_systems(
                PostUpdate,
                (update_daylight, compute_light).chain().after(crate::camera::follow).before(TransformSystems::Propagate),
            );
    }
}

fn reload_settings(mut watch: ResMut<SettingsWatch>, mut settings: ResMut<LightSettings>) {
    if !watch.0.changed() {
        return;
    }
    match load_ron(watch.0.path()) {
        Ok(new) => {
            *settings = new;
            info!("lighting reloaded");
        }
        Err(e) => warn!("lighting not reloaded: {e}"),
    }
}

fn keys(
    mut commands: Commands,
    mut actions: MessageReader<crate::dev::DevAction>,
    settings: Res<LightSettings>,
    art: Option<Res<TorchArt>>,
    mut toggles: ResMut<LightToggles>,
    mut day: ResMut<Daylight>,
    player: Query<&Kinematics, With<LocalPlayer>>,
) {
    use crate::dev::DevAction;
    for a in actions.read() {
        match *a {
            DevAction::Lighting => toggles.enabled = !toggles.enabled,
            DevAction::Later => day.skipped += 3.0,
            DevAction::DayAhead => day.skipped += 24.0,
            DevAction::PlantTorch(at) => {
                // From the panel: at the player's feet.
                if let Some(at) = at.or_else(|| player.single().ok().map(|k| k.body.pos - Vec2::new(0.0, k.body.half.y)))
                    && let Some(art) = art.as_deref()
                {
                    plant_torch(&mut commands, at, &settings, art);
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn rgb((r, g, b): (u8, u8, u8), s: f32) -> Rgb {
    [r as f32 / 255.0 * s, g as f32 / 255.0 * s, b as f32 / 255.0 * s]
}

/// Blasts and lightning light up their surroundings for a moment.
fn collect_flashes(
    time: Res<Time>,
    mut blasts: MessageReader<Explosion>,
    mut bolts: MessageReader<Lightning>,
    mut zaps: MessageReader<crate::fx::Zapped>,
    mut puffs: MessageReader<crate::creatures::AirJumped>,
    mut flashes: ResMut<Flashes>,
    mut flares: MessageReader<crate::fx::Flare>,
) {
    for f in flares.read() {
        flashes.0.push(Flash { at: f.at, color: f.color, age: 0.0, life: f.life });
    }
    // A double jump's cloud glows a moment (it shows in the dark).
    for j in puffs.read() {
        flashes.0.push(Flash { at: j.at, color: [0.5, 0.6, 0.85], age: 0.0, life: 0.45 });
    }
    for e in blasts.read() {
        let k = (e.radius / 30.0).min(2.0);
        flashes.0.push(Flash { at: e.at, color: [1.3 * k, 0.95 * k, 0.55 * k], age: 0.0, life: 0.45 });
    }
    for Lightning(s) in bolts.read() {
        flashes.0.push(Flash { at: Vec2::new(s.hit.x as f32, s.hit.y as f32 + 6.0), color: [1.4, 1.45, 1.7], age: 0.0, life: 0.4 });
        for p in s.charged.iter().step_by(120).take(24) {
            flashes.0.push(Flash { at: Vec2::new(p.x as f32, p.y as f32), color: [0.35, 0.5, 0.9], age: 0.0, life: 0.3 });
        }
    }
    for crate::fx::Zapped(z) in zaps.read() {
        let mid = Vec2::new((z.from.x + z.to.x) as f32 / 2.0, (z.from.y + z.to.y) as f32 / 2.0);
        for at in [mid, Vec2::new(z.to.x as f32, z.to.y as f32)] {
            flashes.0.push(Flash { at, color: [0.8, 0.85, 1.1], age: 0.0, life: 0.25 });
        }
        // A charged pool lights up along its length.
        for p in z.charged.iter().step_by(120).take(24) {
            flashes.0.push(Flash { at: Vec2::new(p.x as f32, p.y as f32), color: [0.35, 0.5, 0.9], age: 0.0, life: 0.3 });
        }
    }
    let dt = time.delta_secs();
    flashes.0.retain_mut(|f| {
        f.age += dt;
        f.age < f.life
    });
}

/// How bright a flame is now, about 0.75..1: a slow sway, a quicker
/// flutter and a jitter a dozen times a second, different per `salt`.
fn fire_flicker(t: f32, salt: u64) -> f32 {
    let s = salt as f32 * 1.7;
    let jitter = (platypus_sim::rng::hash(&[salt, (t * 14.0) as u64]) % 1000) as f32 / 1000.0;
    (0.86 + 0.06 * (t * 2.3 + s).sin() + 0.04 * (t * 11.0 + s * 2.3).sin() + 0.06 * (jitter - 0.5)).clamp(0.0, 1.0)
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp3(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Sky light and sky colour for a time of day (0..1). Day white, golden
/// around dawn and dusk, a dim blue moonlight at night.
pub fn sky_at(time: f32, moonlight: f32) -> (Rgb, Rgb) {
    let sun = -(time * std::f32::consts::TAU).cos(); // -1 midnight … 1 noon
    let day = smoothstep(-0.12, 0.3, sun);
    let golden = (1.0 - ((sun - 0.02).abs() / 0.3).min(1.0)).powi(2);
    let night = [0.09 * moonlight, 0.12 * moonlight, 0.24 * moonlight];
    let mut light = lerp3(night, [1.0, 0.97, 0.92], day);
    let g = day.max(0.6);
    light = lerp3(light, [1.0 * g, 0.6 * g, 0.34 * g], golden * 0.85);
    let sky = SKY_COLOR.to_srgba();
    // (Night: near black, a little blue; the stars carry it.)
    let mut color = lerp3([0.05, 0.065, 0.15], [sky.red, sky.green, sky.blue], day);
    color = lerp3(color, [0.98, 0.58, 0.42], golden * 0.8);
    (light, color)
}

/// Where the sun (or, at night, the moon) is: a direction light travels in
/// (unit, downward), rising in the east (right), setting in the west, never
/// lower than ~12° so it still reaches in.
pub fn sun_dir(time: f32) -> [f32; 2] {
    // The sun's half of the day is 06:00–18:00, the moon's the other half.
    let phase = ((time - 0.25).rem_euclid(0.5)) / 0.5; // 0 rising … 1 setting
    let a = phase * std::f32::consts::PI;
    let (x, y) = (a.cos(), a.sin().max(0.2));
    let n = (x * x + y * y).sqrt();
    [-x / n, -y / n]
}

/// The sky's light as directions: the sun (or moon) carrying all of it, and
/// the rest of the sky from either side of straight up, softer, so shade is
/// never black and nothing casts a hard shadow straight down.
fn sky_lights(time: f32, light: Rgb) -> [([f32; 2], Rgb); 3] {
    let (s, c) = (35f32.to_radians().sin(), 35f32.to_radians().cos());
    let diffuse = light.map(|v| v * 0.55);
    [(sun_dir(time), light), ([s, -c], diffuse), ([-s, -c], diffuse)]
}

/// How heavy the weather is round the view (0..1 each), eased: how
/// overcast, how hard it rains. The light dims and cools with it, the sky
/// darkens, the far ranges haze over (`backdrop`).
#[derive(Resource, Default, Clone, Copy)]
pub struct Gloom {
    pub cover: f32,
    pub rain: f32,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn update_daylight(
    sim: Res<SimWorld>,
    settings: Res<LightSettings>,
    flash: Res<SkyFlash>,
    toggles: Res<LightToggles>,
    time: Res<Time>,
    mut day: ResMut<Daylight>,
    mut gloom: ResMut<Gloom>,
    mut clear: ResMut<ClearColor>,
    cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>,
) {
    let day_ticks = (settings.day_minutes.max(0.1) * 60.0 * TICK_HZ as f32) as f64;
    let hours = settings.start_hour + day.skipped;
    day.days = (hours / 24.0) as f64 + sim.world.tick() as f64 / day_ticks;
    day.time = day.days.fract() as f32;
    let (mut light, mut color) = sky_at(day.time, settings.moonlight);
    // Weather overhead: as it clouds over and rains, the light comes in
    // dimmer, flatter and cooler, the sky a heavy blue-grey, darker in a
    // downpour; easing in and out over seconds, as a front comes over.
    if let Some(w) = sim.world.weather() {
        let (tf, loader) = *cam;
        let x = tf.translation.x as i32;
        let hw = loader.half_extent.x as i32;
        let cover = ((w.overcast(x - hw, x + hw) - 0.15) / 0.45).clamp(0.0, 1.0);
        let n = 9;
        let rain = (0..n).map(|i| w.rain_at(x - hw + i * hw * 2 / (n - 1))).sum::<f32>() / n as f32;
        let rain = (rain / platypus_sim::weather::STORM_RAIN).clamp(0.0, 1.0);
        let k = (time.delta_secs() / GLOOM_EASE).min(1.0);
        gloom.cover += (cover - gloom.cover) * k;
        gloom.rain += (rain - gloom.rain) * k;
        let (cover, rain) = (gloom.cover, gloom.rain);
        let grey = (light[0] + light[1] + light[2]) / 3.0;
        // (An overcast sky is a pale, bright grey, a downpour's a slate
        // one; the land under it flatter, cooler and a little dim.)
        let cool = [grey * 0.84, grey * 0.88, grey * 0.97];
        let dim = 1.0 - 0.1 * cover - 0.16 * rain;
        light = lerp3(light, cool, (cover * 0.7).min(1.0)).map(|v| v * dim);
        let heavy = lerp3([0.7, 0.73, 0.78], [0.47, 0.51, 0.57], rain).map(|c| c * (0.45 + 0.55 * grey.min(1.0)));
        color = lerp3(color, heavy, (cover * 0.9).min(1.0));
    }
    // Lightning lights the whole sky.
    light = light.map(|c| c.max(flash.0 * 1.2));
    color = lerp3(color, [0.88, 0.9, 1.0], flash.0 * 0.7);
    day.sky = light;
    // With lighting off, the sky colour is shown as it is; with it on, the
    // overlay multiplies it by the sky light (dark at night).
    let _ = toggles;
    clear.0 = Color::srgb(color[0], color[1], color[2]);
}

/// God rays: the sun's light in open air shown as haze at this share (at
/// most: `compute_light`).
const SHAFTS: f32 = 0.3;

/// `SHAFTS`, or `PLATYPUS_SHAFTS` (to look at them).
fn shafts_scale() -> f32 {
    static K: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *K.get_or_init(|| std::env::var("PLATYPUS_SHAFTS").ok().and_then(|v| v.parse().ok()).unwrap_or(SHAFTS))
}

/// The weather's gloom follows it over this many seconds.
const GLOOM_EASE: f32 = 4.0;

/// Cells per light texel at a zoom: about 6 screen pixels or more (a power
/// of two: texels tile a chunk). (Zoom 2 at 2 cells doubled the frame time:
/// the overlay's smooth filter hides the steps instead.)
fn texel_for(zoom: u32) -> i32 {
    match zoom {
        1 => 8,
        2 => 4,
        3..=5 => 2,
        _ => 1,
    }
}

/// Sparkles to put out this frame (`glimmer`), from the last solve.
#[derive(Resource, Default)]
pub struct Glints(Vec<(CellPos, grid::Rgb)>);

/// A glint: a point of light flaring and gone.
#[derive(Component)]
struct Glint {
    age: f32,
    life: f32,
    rgb: grid::Rgb,
}

/// Glints over everything (they're light), at most so many at once, each
/// so long (s).
const Z_GLINT: f32 = 16.05;
const GLINTS_MOST: usize = 220;
const GLINT_LIFE: (f32, f32) = (0.12, 0.3);

/// Gems, crystal, glass, ice and water's surface caught by a beam or the
/// sun glint: a point flaring up and gone (from `LightGrid::glints`).
fn glimmer(mut commands: Commands, time: Res<Time>, mut glints: ResMut<Glints>, mut live: Query<(Entity, &mut Glint, &mut Sprite)>) {
    let dt = time.delta_secs();
    let mut n = 0;
    for (e, mut g, mut sprite) in &mut live {
        g.age += dt;
        if g.age >= g.life {
            commands.entity(e).despawn();
            continue;
        }
        n += 1;
        let k = (std::f32::consts::PI * g.age / g.life).sin();
        sprite.color = Color::srgba(g.rgb[0].min(1.0), g.rgb[1].min(1.0), g.rgb[2].min(1.0), k);
    }
    for (at, rgb) in glints.0.drain(..) {
        if n >= GLINTS_MOST {
            break;
        }
        let life = GLINT_LIFE.0 + (GLINT_LIFE.1 - GLINT_LIFE.0) * (platypus_sim::rng::hash(&[at.x as u64, at.y as u64, 0x611]) % 1000) as f32 / 1000.0;
        commands.spawn((Glint { age: 0.0, life, rgb }, Sprite::from_color(Color::NONE, Vec2::ONE), Transform::from_translation(Vec3::new(at.x as f32 + 0.5, at.y as f32 + 0.5, Z_GLINT))));
        n += 1;
    }
}

/// `PLATYPUS_EASE_BEAMS`: ease the beams' light too (the old trail).
fn ease_beams() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("PLATYPUS_EASE_BEAMS").is_ok())
}

#[allow(clippy::too_many_arguments)]
fn compute_light(
    sim: Res<SimWorld>,
    settings: Res<LightSettings>,
    toggles: Res<LightToggles>,
    day: Res<Daylight>,
    flashes: Res<Flashes>,
    zoom: Res<Zoom>,
    time: Res<Time>,
    (cursor, off_light, fixed, mut glints, mut cones, gloom): (Res<CursorWorld>, Res<torch::OffLight>, Res<Time<Fixed>>, ResMut<Glints>, ResMut<Cones>, Res<Gloom>),
    mut overlay: ResMut<Overlay>,
    mut pending: ResMut<Pending>,
    mut metrics: ResMut<LightMetrics>,
    mut assets: OverlayAssets,
    cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>,
    player: Query<(&Kinematics, Option<&crate::creatures::body::animation::HandPos>), With<LocalPlayer>>,
    sources: Query<(&GlobalTransform, &LightSource, Has<Glow>, Option<&Haze>)>,
    mut sprites: Query<(&mut Transform, &mut Visibility), Without<MainCamera>>,
) {
    let started = Instant::now();
    for e in [overlay.light, overlay.glow] {
        if let Ok((_, mut vis)) = sprites.get_mut(e) {
            *vis = if toggles.enabled { Visibility::Inherited } else { Visibility::Hidden };
        }
    }
    if !toggles.enabled {
        return;
    }
    // Show what the last frame's solve made.
    if let Some(task) = pending.0.take() {
        let solved = block_on(async {
            let mut task = task;
            match poll_once(&mut task).await {
                Some(done) => Ok(done),
                None => Err(task),
            }
        });
        // (Not quite done: finish it; rare, the frame had ~a frame's time.)
        let mut done = match solved {
            Err(task) => block_on(task),
            Ok(done) => done,
        };
        glints.0.append(&mut done.glints);
        pending.1 = Some(show(done, &mut overlay, &mut metrics, &mut assets, &mut sprites));
    }
    let world = &sim.world;
    let (tf, loader) = *cam;
    let t = texel_for(zoom.0);
    let half = loader.half_extent + Vec2::splat(MARGIN);
    let lo = tf.translation.truncate() - half;
    let origin = CellPos::new((lo.x / t as f32).floor() as i32 * t, (lo.y / t as f32).floor() as i32 * t);
    let (w, h) = (((2.0 * half.x) / t as f32).ceil() as usize + 1, ((2.0 * half.y) / t as f32).ceil() as usize + 1);
    let mut g = LightGrid::new(origin, w, h, t);

    // What glows and what blocks; fire flickers (a few times a second, per spot).
    let frame = (time.elapsed_secs() * 12.0) as u64;
    let flicker = |x: i32, y: i32| {
        let n = platypus_sim::rng::hash(&[frame, (x >> 1) as u64, (y >> 1) as u64]);
        0.72 + 0.28 * (n % 1000) as f32 / 1000.0
    };
    // Breathing glows: each 16-cell patch on its own slow cycle (3–6 s).
    let now = time.elapsed_secs();
    let breath = |x: i32, y: i32| {
        let n = platypus_sim::rng::hash(&[0xB4EA7, (x >> 4) as u64, (y >> 4) as u64]);
        let (phase, speed) = ((n % 1000) as f32 / 1000.0 * std::f32::consts::TAU, 1.0 + ((n >> 10) % 1000) as f32 / 1000.0 * 1.1);
        0.5 + 0.5 * (now * speed + phase).sin()
    };
    // Glimmering glows: 8-cell patches each slowly swelling from dark to
    // bright and back (every 5–12 s), each out of step with the next.
    let glimmer = |x: i32, y: i32| {
        let n = platypus_sim::rng::hash(&[0x61177, (x >> 3) as u64, (y >> 3) as u64]);
        let (phase, speed) = ((n % 1000) as f32 / 1000.0 * std::f32::consts::TAU, 0.5 + ((n >> 10) % 1000) as f32 / 1000.0 * 0.8);
        let s = 0.5 + 0.5 * (now * speed + phase).sin();
        s * s
    };
    g.fill_from(world, &flicker, &breath, &glimmer);

    // Sky: down every column that is open to the sky above the grid.
    let top = origin.y + h as i32 * t;
    let mats = world.materials();
    let generator = &sim.generator;
    let open = |tx: usize| {
        let x = origin.x + tx as i32 * t + t / 2;
        for y in top..top + 900 {
            match world.get(CellPos::new(x, y)) {
                // Past what's loaded: open if above the ground as generated.
                None => return generator.surface_hint(x).is_none_or(|s| y >= s),
                Some(c) => {
                    let front = if c.is_air() { 0 } else { mats.phys(c.material).opacity };
                    if front > 128 {
                        return false;
                    }
                }
            }
        }
        true
    };
    let open_top: Vec<bool> = (0..w).map(open).collect();
    let sky = sky_lights(day.time, day.sky);
    // God rays: the sun's light in the open air, a faint haze along it
    // (shafts where something shades it: under a crown, past a cliff, in a
    // cave's mouth); most as a sky breaks (clouding over, not pouring).
    let shafts = shafts_scale() * (0.3 + 0.7 * gloom.cover * (1.0 - 0.75 * gloom.rain));

    // What the player carries.
    if let Ok((k, hand)) = player.single() {
        // (Where it's drawn this frame, between ticks: from the tick's own
        // place the light shook against it, moving fast.)
        let drawn = k.prev_pos.lerp(k.body.pos, fixed.overstep_fraction());
        let at = drawn + Vec2::new(0.0, k.body.half.y * 0.4);
        let c = |(r, g, b): (u8, u8, u8), s: f32| [r as f32 / 255.0 * s, g as f32 / 255.0 * s, b as f32 / 255.0 * s];
        g.seed_point([at.x, at.y], c(settings.lantern.color, settings.lantern.strength));
        cones.0.clear();
        let beam = match toggles.carry {
            Carry::SmallBeam => Some(settings.flashlight_small),
            Carry::BigBeam => Some(settings.flashlight),
            _ => None,
        };
        // A beam: its hot spot, the dim wide spill about it, and a glow at
        // the lamp itself (what's right by you shows, whichever way).
        let mut lamp = |g: &mut grid::LightGrid, from: Vec2, dir: Vec2, f: BeamCfg| {
            let sp = settings.spill;
            let col = c(f.color, f.strength);
            g.seed_beam([from.x, from.y], [dir.x, dir.y], f.angle.to_radians() / 2.0, f.range, col);
            g.seed_beam([from.x, from.y], [dir.x, dir.y], sp.angle.to_radians() / 2.0, sp.range, col.map(|v| v * sp.share));
            g.seed_point([from.x, from.y], col.map(|v| v * sp.glow));
            cones.0.push((from, dir, f.angle.to_radians() / 2.0, f.range, col));
        };
        if let (Some(f), Some(aim)) = (beam, cursor.0) {
            // (From the flashlight's lens, in the off hand.)
            let from = off_light.lens.map_or(at, |l| drawn + l);
            let dir = (aim - from).normalize_or(Vec2::X);
            lamp(&mut g, from, dir, f);
        }
        // A mining helmet's lamp, from the brow.
        if let (true, Some(aim)) = (toggles.headlamp, cursor.0) {
            // (Its brow as drawn, where the helmet's lamp is; the head's top
            // with none.)
            let from = drawn + hand.and_then(|h| h.brow).map_or(Vec2::new(k.loco.facing * k.body.half.x * 0.6, k.body.half.y * 0.8), |b| b.truncate());
            let dir = (aim - from).normalize_or(Vec2::X);
            lamp(&mut g, from, dir, settings.headlamp);
        }
    }
    // Embers, sparks, burning debris.
    let fire = mats.fire();
    for p in world.particles() {
        let hot = p.landing == Landing::Ember || p.cell.flags & flags::BURNING != 0 || p.cell.material == fire;
        if hot {
            g.seed_point(p.pos, [0.55, 0.28, 0.06]);
        } else if p.cell.heat > 500 {
            g.seed_point(p.pos, [0.4, 0.15, 0.03]);
        }
    }
    for (i, (tf, src, glowing, haze)) in sources.iter().enumerate() {
        let p = tf.translation();
        if let Some(&Haze(k)) = haze {
            g.seed_emit([p.x, p.y], src.color.map(|c| c * k));
        }
        // Glowing creatures glow as glowing cells do: a haze over the dark.
        if glowing {
            g.seed_emit([p.x, p.y], src.color);
            continue;
        }
        // Fire: brighter and dimmer, and redder as it dims.
        let f = 1.0 - src.flicker * (1.0 - fire_flicker(time.elapsed_secs(), i as u64 + 7));
        g.seed_point([p.x, p.y], [src.color[0] * f, src.color[1] * f.powf(1.5), src.color[2] * f * f]);
    }
    for f in &flashes.0 {
        let k = 1.0 - f.age / f.life;
        g.seed_point([f.at.x, f.at.y], f.color.map(|c| c * k));
    }

    let params = Params { air_falloff: settings.air_falloff, iterations: settings.iterations.clamp(1, 4), rim: settings.rim };
    let amb = settings.ambient.max(0.0);
    let haze = settings.glow_haze.max(0.0);
    let (gw, gh) = (w as f32 * t as f32, h as f32 * t as f32);
    let center = Vec2::new(origin.x as f32 + gw / 2.0, origin.y as f32 + gh / 2.0);
    metrics.time = started.elapsed();
    metrics.texels = w * h;
    // Light eases in over ~2 frames and out over ~5 at 120 fps.
    let dt = time.delta_secs().clamp(0.001, 0.1);
    let (rise, fall) = (1.0 - (-dt / 0.02).exp(), 1.0 - (-dt / 0.05).exp());
    let previous = pending.1.take();
    let glint_seed = sim.world.tick() ^ (time.elapsed_secs_f64() * 1000.0) as u64;
    pending.0 = Some(AsyncComputeTaskPool::get().spawn(async move {
        let started = Instant::now();
        // (The sun's share of the seeds kept apart: the shafts.)
        let mut sun = None;
        for (i, (dir, color)) in sky.into_iter().enumerate() {
            if i == 0 && shafts > 0.0 {
                let before = g.seed.clone();
                g.seed_directional(dir, color, &open_top);
                sun = Some(g.seed.iter().zip(&before).zip(&g.opacity).map(|((a, b), &o)| if o < 0.05 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] } else { [0.0; 3] }).collect::<Vec<_>>());
            } else {
                g.seed_directional(dir, color, &open_top);
            }
        }
        g.charge_phosphor(previous.as_ref(), dt);
        g.shine(previous.as_ref());
        g.solve(params);
        // (`PLATYPUS_EASE_BEAMS=1`: beams eased with the rest, as before:
        // a moving one trails; for comparing.)
        let legacy = ease_beams();
        if legacy {
            g.add_direct();
        }
        if let Some(prev) = &previous {
            g.ease_from(prev, rise, fall);
        }
        let spread = if legacy { (g.light.clone(), g.glow.clone()) } else { g.add_direct() };
        let glints = g.glints(dt, glint_seed);
        // Upload format: sqrt for precision in the dark, row 0 at the top.
        let encode = |v: f32| (v.clamp(0.0, 1.0).sqrt() * 255.0 + 0.5) as u8;
        let mut light = vec![255u8; w * h * 4];
        let mut glow = vec![255u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let (l, gl, c) = (g.light[y * w + x], g.glow[y * w + x], g.caustic[y * w + x]);
                let ray = sun.as_ref().map_or([0.0; 3], |s| s[y * w + x]);
                let i = ((h - 1 - y) * w + x) * 4;
                // (Caustics added whole, not hazed: bent light shows over
                // daylight; the sun's shafts faintly.)
                for ch in 0..3 {
                    light[i + ch] = encode(l[ch].max(amb));
                    glow[i + ch] = encode(gl[ch] * haze + c[ch] + ray[ch] * shafts);
                }
            }
        }
        let took = started.elapsed();
        Solved { previous: g.into_previous(spread), glints, light, glow, size: UVec2::new(w as u32, h as u32), center, extent: Vec2::new(gw, gh), took }
    }));
}

#[derive(bevy::ecs::system::SystemParam)]
struct OverlayAssets<'w> {
    images: ResMut<'w, Assets<Image>>,
    light_mats: ResMut<'w, Assets<LightMultiply>>,
    glow_mats: ResMut<'w, Assets<LightAdd>>,
}

/// Put a solved grid on screen; returns it, to ease the next one from.
fn show(
    s: Solved,
    overlay: &mut Overlay,
    metrics: &mut LightMetrics,
    assets: &mut OverlayAssets,
    sprites: &mut Query<(&mut Transform, &mut Visibility), Without<MainCamera>>,
) -> grid::Previous {
    metrics.solve = s.took;
    let OverlayAssets { images, light_mats, glow_mats } = assets;
    let (light_img, glow_img) = if overlay.size != s.size {
        let l = images.add(light_image(s.size));
        let gi = images.add(light_image(s.size));
        if let Some(mut m) = light_mats.get_mut(&overlay.light_material) {
            m.texture = l.clone();
        }
        if let Some(mut m) = glow_mats.get_mut(&overlay.glow_material) {
            m.texture = gi.clone();
        }
        overlay.size = s.size;
        (l, gi)
    } else {
        let l = light_mats.get(&overlay.light_material).map(|m| m.texture.clone());
        let gi = glow_mats.get(&overlay.glow_material).map(|m| m.texture.clone());
        let (Some(l), Some(gi)) = (l, gi) else { return s.previous };
        (l, gi)
    };
    for (handle, bytes) in [(light_img, s.light), (glow_img, s.glow)] {
        if let Some(mut img) = images.get_mut(&handle) {
            img.data = Some(bytes);
        }
    }
    // Over the grid, snapped to its texels (world-anchored, no swimming).
    for (e, z) in [(overlay.light, Z_LIGHT), (overlay.glow, Z_GLOW)] {
        if let Ok((mut tf, _)) = sprites.get_mut(e) {
            *tf = Transform::from_translation(s.center.extend(z)).with_scale(s.extent.extend(1.0));
        }
    }
    s.previous
}

fn light_image(size: UVec2) -> Image {
    let mut img = Image::new_fill(
        Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = ImageSampler::linear();
    img
}

fn spawn_overlay(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut light_mats: ResMut<Assets<LightMultiply>>,
    mut glow_mats: ResMut<Assets<LightAdd>>,
) {
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    let light_material = light_mats.add(LightMultiply { texture: images.add(light_image(UVec2::ONE)) });
    let glow_material = glow_mats.add(LightAdd { texture: images.add(light_image(UVec2::ONE)) });
    let light = commands
        .spawn((Name::new("Light"), Mesh2d(quad.clone()), MeshMaterial2d(light_material.clone()), Transform::from_xyz(0.0, 0.0, Z_LIGHT)))
        .id();
    let glow = commands
        .spawn((Name::new("Glow"), Mesh2d(quad), MeshMaterial2d(glow_material.clone()), Transform::from_xyz(0.0, 0.0, Z_GLOW)))
        .id();
    commands.insert_resource(Overlay { light, glow, light_material, glow_material, size: UVec2::ONE });
}

/// Multiplies the scene by the light.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct LightMultiply {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
}

/// Adds the glow haze.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct LightAdd {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
}

fn set_blend(descriptor: &mut RenderPipelineDescriptor, color: BlendComponent) {
    if let Some(fragment) = descriptor.fragment.as_mut() {
        for target in fragment.targets.iter_mut().flatten() {
            target.blend = Some(BlendState {
                color,
                // Leave the scene's alpha as it is.
                alpha: BlendComponent { src_factor: BlendFactor::Zero, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
            });
        }
    }
}

impl Material2d for LightMultiply {
    fn fragment_shader() -> ShaderRef {
        "shaders/light.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // scene × light
        set_blend(descriptor, BlendComponent { src_factor: BlendFactor::Dst, dst_factor: BlendFactor::Zero, operation: BlendOperation::Add });
        Ok(())
    }
}

impl Material2d for LightAdd {
    fn fragment_shader() -> ShaderRef {
        "shaders/light.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // scene + glow
        set_blend(descriptor, BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::One, operation: BlendOperation::Add });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::sky_at;

    #[test]
    fn day_is_bright_night_is_dark_blue_dusk_is_golden() {
        let lum = |t: f32| sky_at(t, 1.0).0.iter().sum::<f32>() / 3.0;
        let (noon, midnight) = (lum(0.5), lum(0.0));
        assert!(noon > 0.9, "noon {noon}");
        assert!(midnight < 0.2, "midnight {midnight}");
        let night = sky_at(0.0, 1.0).0;
        assert!(night[2] > night[0], "moonlight is blue");
        let dusk = sky_at(0.75, 1.0).0;
        assert!(dusk[0] > dusk[2] * 1.5, "dusk is golden: {dusk:?}");
        assert!(sky_at(0.0, 0.0).0.iter().all(|&c| c == 0.0), "moonlight 0: pitch black");
    }

    #[test]
    fn the_sun_rises_east_crosses_overhead_and_sets_west() {
        use super::sun_dir;
        let (morning, noon, evening) = (sun_dir(0.3), sun_dir(0.5), sun_dir(0.7));
        assert!(morning[0] < -0.5, "morning light travels west (from the east): {morning:?}");
        assert!(noon[0].abs() < 0.01 && noon[1] < -0.99, "noon: straight down {noon:?}");
        assert!(evening[0] > 0.5, "evening light travels east: {evening:?}");
        assert!(sun_dir(0.26)[1] < -0.15, "never flat: {:?}", sun_dir(0.26));
    }
}
