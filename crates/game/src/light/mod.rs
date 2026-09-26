//! Lighting and the day (SPEC §4.1). Every frame, a light grid over the view
//! is filled from the cells (`grid.rs`), lit by the sky, the player's lantern
//! and flashlight, burning and glowing things, flying embers, blasts and
//! lightning; then drawn over the scene twice: multiplied (what isn't lit is
//! dark) and added (a haze around what glows). Rendering only: the sim never
//! reads it, so it costs co-op nothing.
//!
//! Keys: L flashlight, F8 +3 hours, F9 lighting on/off. `PLATYPUS_HOUR=19`
//! starts at that hour.

pub mod grid;

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

use crate::actors::Kinematics;
use crate::actors::player::LocalPlayer;
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
const MARGIN: f32 = 48.0;

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct LampCfg {
    pub color: (u8, u8, u8),
    pub strength: f32,
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
    pub torch: LampCfg,
    pub glowstick: StickCfg,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct StickCfg {
    pub strength: f32,
    pub secs: f32,
}

/// Anything that gives off light: a planted torch, a glow stick (later a
/// lantern on a post, glowing eyes). `flicker` 0..1 makes it waver like fire.
#[derive(Component, Clone, Copy, Debug)]
pub struct LightSource {
    pub color: Rgb,
    pub flicker: f32,
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
    /// Hours the dev key (or a scenario) has skipped.
    pub skipped: f32,
    /// Light from the sky right now (sun or moon, cloud, lightning).
    pub sky: Rgb,
}

impl Daylight {
    /// "08:30".
    pub fn clock(&self) -> String {
        let minutes = (self.time * 24.0 * 60.0) as u32;
        format!("{:02}:{:02}", minutes / 60, minutes % 60)
    }
}

#[derive(Resource)]
pub struct LightToggles {
    pub enabled: bool,
    pub flashlight: bool,
    pub torch: bool,
}

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
            .insert_resource(LightToggles { enabled: std::env::var("PLATYPUS_NOLIGHT").is_err(), flashlight: false, torch: false })
            .insert_resource(Daylight { skipped: skip, ..default() })
            .init_resource::<Flashes>()
            .init_resource::<LightMetrics>()
            .init_resource::<Pending>()
            .add_systems(Startup, spawn_overlay)
            .add_systems(Update, (reload_settings, keys, collect_flashes))
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
    mut toggles: ResMut<LightToggles>,
    mut day: ResMut<Daylight>,
    player: Query<&Kinematics, With<LocalPlayer>>,
) {
    use crate::dev::DevAction;
    for a in actions.read() {
        match *a {
            DevAction::Flashlight => toggles.flashlight = !toggles.flashlight,
            DevAction::Torch => toggles.torch = !toggles.torch,
            DevAction::Lighting => toggles.enabled = !toggles.enabled,
            DevAction::Later => day.skipped += 3.0,
            DevAction::PlantTorch(at) => {
                // From the panel: at the player's feet.
                if let Some(at) = at.or_else(|| player.single().ok().map(|k| k.body.pos - Vec2::new(0.0, k.body.half.y - 3.0))) {
                    plant_torch(&mut commands, at, &settings);
                }
            }
            _ => {}
        }
    }
}

fn rgb((r, g, b): (u8, u8, u8), s: f32) -> Rgb {
    [r as f32 / 255.0 * s, g as f32 / 255.0 * s, b as f32 / 255.0 * s]
}

/// A torch stuck in the ground at `at`: a stick with a flame, flickering light.
pub fn plant_torch(commands: &mut Commands, at: Vec2, settings: &LightSettings) {
    commands
        .spawn((
            Name::new("Torch"),
            PlantedTorch,
            LightSource { color: rgb(settings.torch.color, settings.torch.strength), flicker: 0.25 },
            Transform::from_translation(at.extend(9.0)),
            Visibility::default(),
        ))
        .with_children(|t| {
            t.spawn((Sprite::from_color(Color::srgb(0.38, 0.24, 0.12), Vec2::new(1.0, 6.0)), Transform::from_xyz(0.0, -2.0, 0.0)));
            t.spawn((Sprite::from_color(Color::srgb(1.0, 0.75, 0.3), Vec2::new(2.0, 2.0)), Transform::from_xyz(0.0, 2.0, 0.1)));
        });
}

/// Blasts and lightning light up their surroundings for a moment.
fn collect_flashes(time: Res<Time>, mut blasts: MessageReader<Explosion>, mut bolts: MessageReader<Lightning>, mut flashes: ResMut<Flashes>) {
    for e in blasts.read() {
        let k = (e.radius / 20.0).min(2.0);
        flashes.0.push(Flash { at: e.at, color: [1.3 * k, 0.95 * k, 0.55 * k], age: 0.0, life: 0.45 });
    }
    for Lightning(s) in bolts.read() {
        flashes.0.push(Flash { at: Vec2::new(s.hit.x as f32, s.hit.y as f32 + 4.0), color: [1.4, 1.45, 1.7], age: 0.0, life: 0.4 });
    }
    let dt = time.delta_secs();
    flashes.0.retain_mut(|f| {
        f.age += dt;
        f.age < f.life
    });
}

/// A smooth wobble 0..1 over time, different per `salt`: fire flicker.
fn wobble(t: f32, salt: u64) -> f32 {
    let s = salt as f32 * 1.7;
    0.5 + 0.25 * (t * 9.0 + s).sin() + 0.25 * (t * 23.0 + s * 2.3).sin()
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
    let mut color = lerp3([0.3, 0.38, 0.7], [sky.red, sky.green, sky.blue], day);
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

fn update_daylight(
    sim: Res<SimWorld>,
    settings: Res<LightSettings>,
    flash: Res<SkyFlash>,
    toggles: Res<LightToggles>,
    mut day: ResMut<Daylight>,
    mut clear: ResMut<ClearColor>,
    cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>,
) {
    let day_ticks = (settings.day_minutes.max(0.1) * 60.0 * TICK_HZ as f32) as f64;
    let hours = settings.start_hour + day.skipped;
    day.time = ((hours / 24.0) as f64 + sim.world.tick() as f64 / day_ticks).fract() as f32;
    let (mut light, mut color) = sky_at(day.time, settings.moonlight);
    // Clouds overhead: dimmer and greyer.
    if let Some(w) = sim.world.weather() {
        let (tf, loader) = *cam;
        let x = tf.translation.x as i32;
        let hw = loader.half_extent.x as i32;
        let k = ((w.overcast(x - hw, x + hw) - 0.15) / 0.45).clamp(0.0, 1.0);
        let grey = (light[0] + light[1] + light[2]) / 3.0;
        light = lerp3(light, [grey * 0.6; 3], k * 0.6);
        let cgrey = [0.46, 0.52, 0.6];
        color = lerp3(color, cgrey.map(|c| c * (0.4 + 0.6 * grey.min(1.0))), k);
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

/// Cells per light texel at a zoom: about 6 screen pixels or more.
fn texel_for(zoom: u32) -> i32 {
    match zoom {
        1 => 8,
        2 => 4,
        3..=5 => 2,
        _ => 1,
    }
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
    cursor: Res<CursorWorld>,
    mut overlay: ResMut<Overlay>,
    mut pending: ResMut<Pending>,
    mut metrics: ResMut<LightMetrics>,
    mut assets: OverlayAssets,
    cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    sources: Query<(&GlobalTransform, &LightSource)>,
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
        match solved {
            // Not quite done: finish it (rare; the frame had ~a frame's time).
            Err(task) => pending.1 = Some(show(block_on(task), &mut overlay, &mut metrics, &mut assets, &mut sprites)),
            Ok(done) => pending.1 = Some(show(done, &mut overlay, &mut metrics, &mut assets, &mut sprites)),
        }
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
    // Glimmering glows: 4-cell patches each swelling from nearly dark to
    // full and back, quickly (every 1–3 s), mostly dim with bright moments.
    let glimmer = |x: i32, y: i32| {
        let n = platypus_sim::rng::hash(&[0x61177, (x >> 2) as u64, (y >> 2) as u64]);
        let (phase, speed) = ((n % 1000) as f32 / 1000.0 * std::f32::consts::TAU, 2.0 + ((n >> 10) % 1000) as f32 / 1000.0 * 4.0);
        let s = 0.5 + 0.5 * (now * speed + phase).sin();
        s * s * s
    };
    g.fill_from(world, &flicker, &breath, &glimmer);

    // Sky: down every column that is open to the sky above the grid.
    let top = origin.y + h as i32 * t;
    let mats = world.materials();
    let generator = &sim.generator;
    let open = |tx: usize| {
        let x = origin.x + tx as i32 * t + t / 2;
        for y in top..top + 600 {
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

    // What the player carries.
    if let Ok(k) = player.single() {
        let at = k.body.pos + Vec2::new(0.0, k.body.half.y * 0.4);
        let c = |(r, g, b): (u8, u8, u8), s: f32| [r as f32 / 255.0 * s, g as f32 / 255.0 * s, b as f32 / 255.0 * s];
        g.seed_point([at.x, at.y], c(settings.lantern.color, settings.lantern.strength));
        if toggles.torch {
            let f = 0.85 + 0.15 * wobble(time.elapsed_secs(), 1);
            g.seed_point([at.x, at.y + 3.0], c(settings.torch.color, settings.torch.strength * f));
        }
        if toggles.flashlight
            && let Some(aim) = cursor.0
        {
            let dir = (aim - at).normalize_or(Vec2::X);
            let f = settings.flashlight;
            g.seed_beam([at.x, at.y], [dir.x, dir.y], f.angle.to_radians() / 2.0, f.range, c(f.color, f.strength));
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
    for (i, (tf, src)) in sources.iter().enumerate() {
        let p = tf.translation();
        let f = 1.0 - src.flicker * 0.5 * (1.0 - wobble(time.elapsed_secs(), i as u64 + 7));
        g.seed_point([p.x, p.y], src.color.map(|c| c * f));
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
    pending.0 = Some(AsyncComputeTaskPool::get().spawn(async move {
        let started = Instant::now();
        for (dir, color) in sky {
            g.seed_directional(dir, color, &open_top);
        }
        g.solve(params);
        if let Some(prev) = &previous {
            g.ease_from(prev, rise, fall);
        }
        // Upload format: sqrt for precision in the dark, row 0 at the top.
        let encode = |v: f32| (v.clamp(0.0, 1.0).sqrt() * 255.0 + 0.5) as u8;
        let mut light = vec![255u8; w * h * 4];
        let mut glow = vec![255u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let (l, gl) = (g.light[y * w + x], g.glow[y * w + x]);
                let i = ((h - 1 - y) * w + x) * 4;
                for ch in 0..3 {
                    light[i + ch] = encode(l[ch].max(amb));
                    glow[i + ch] = encode(gl[ch] * haze);
                }
            }
        }
        let took = started.elapsed();
        Solved { previous: g.into_previous(), light, glow, size: UVec2::new(w as u32, h as u32), center, extent: Vec2::new(gw, gh), took }
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
