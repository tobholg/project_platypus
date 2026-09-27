//! Parallax backdrops (DESIGN §4.3b): the surface's far layers behind the
//! world, Noita-like (`platypus_backdrop`), and the night sky's stars and
//! moon.
//!
//! - Four layers, far to near, each scrolling at its own share of the
//!   camera's motion (`PARALLAX`), drawn in tiles of strip as the camera
//!   goes (in the background, cached), blended between the biomes around
//!   each place (the world plan's), sitting on the ground as generated
//!   and fading out underground.
//! - Drawn in daylight colours, behind the world and under the lighting:
//!   the light overlay grades them by the hour as it does everything (gold
//!   at dusk, dark silhouettes at night).
//! - Stars and the moon are drawn over the lighting (a black night keeps
//!   them), but only where there's open sky: no cell, no back wall, no
//!   backdrop in front, not under cloud. They twinkle.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use platypus_backdrop::tile::{SURFACE_LAYERS, surface_tile};
use platypus_backdrop::{Scene, scenes};
use platypus_sim::{CellPos, Kind};
use platypus_worldgen::ChunkGenerator;

use crate::camera::MainCamera;
use crate::world::{ChunkLoader, SimWorld};

pub struct BackdropPlugin;

impl Plugin for BackdropPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Backdrops::new())
            .add_systems(Startup, (stars_setup, under_setup))
            .add_systems(PostUpdate, (tiles, place, stars, under).chain().after(crate::camera::follow).before(bevy::transform::TransformSystems::Propagate));
    }
}

/// Each layer's share of the camera's motion (far: little).
const PARALLAX: [f32; SURFACE_LAYERS] = [0.06, 0.16, 0.3, 0.5];
/// Strip columns a tile, rows a layer.
const TILE: usize = 256;
const HEIGHT: usize = 300;
/// Behind the world's back walls (-1), in front of the clouds (-2).
const Z: f32 = -1.8;
/// How far below a tile its bottom row is stretched (cells).
const SKIRT: f32 = 300.0;
/// How far to either side of a place the biomes blend (world cells).
const BLEND: f32 = 500.0;

struct Tile {
    entity: Entity,
    /// Its bottom row stretched down (valleys lower than its foot).
    skirt: Entity,
    /// Its alpha (stars hide behind it).
    alpha: Vec<u8>,
}

#[derive(Resource)]
pub struct Backdrops {
    scenes: Arc<Vec<Scene>>,
    tiles: HashMap<(usize, i64), Tile>,
    making: HashMap<(usize, i64), Task<Vec<u8>>>,
    /// The ground's height where the camera is (smoothed), the layers' foot.
    ground: Option<f32>,
    /// How much they show (0 underground).
    shown: f32,
}

impl Backdrops {
    fn new() -> Self {
        let all: Vec<Scene> = scenes().into_iter().filter(|s| s.underground.is_none() && s.layers.len() == SURFACE_LAYERS).collect();
        Backdrops { scenes: Arc::new(all), tiles: HashMap::new(), making: HashMap::new(), ground: None, shown: 1.0 }
    }

    /// Where a layer's tile is drawn: its centre, from where the camera is.
    fn centre(&self, k: usize, t: i64, cam: Vec2) -> Vec2 {
        let p = PARALLAX[k];
        let ground = self.ground.unwrap_or(cam.y);
        let x = (t as f32 + 0.5) * TILE as f32 + cam.x * (1.0 - p);
        // The layer's foot a little under the ground, scrolling up and
        // down at its share too.
        let foot = ground - 40.0 + (cam.y - ground) * (1.0 - p);
        Vec2::new(x.round(), (foot + HEIGHT as f32 / 2.0).round())
    }

    /// Is there backdrop drawn at this world point (for the stars)?
    fn covers(&self, at: Vec2, cam: Vec2) -> bool {
        (0..SURFACE_LAYERS).any(|k| {
            let p = PARALLAX[k];
            let sx = at.x - cam.x * (1.0 - p);
            let t = (sx / TILE as f32).floor() as i64;
            let Some(tile) = self.tiles.get(&(k, t)) else { return false };
            let c = self.centre(k, t, cam);
            let (lx, ly) = ((sx - t as f32 * TILE as f32) as usize, (c.y + HEIGHT as f32 / 2.0 - at.y) as i64);
            (0..HEIGHT as i64).contains(&ly) && lx < TILE && tile.alpha[ly as usize * TILE + lx] > 40
        })
    }
}

/// The scenes around a world column, weighted (sampled across `BLEND`).
fn blend<'s>(scenes: &'s [Scene], plan: &dyn ChunkGenerator, x: f32) -> Vec<(&'s Scene, f32)> {
    let mut out: Vec<(&Scene, f32)> = Vec::new();
    for (k, w) in [(-2.0, 0.1), (-1.0, 0.2), (0.0, 0.4), (1.0, 0.2), (2.0, 0.1)] {
        let name = plan.biome_hint((x + k * BLEND / 2.0) as i32).unwrap_or("forest");
        let Some(scene) = scenes.iter().find(|s| s.name == name).or(scenes.first()) else { continue };
        match out.iter_mut().find(|(s, _)| s.name == scene.name) {
            Some(e) => e.1 += w,
            None => out.push((scene, w)),
        }
    }
    out
}

/// Tiles in view (and a little either side) made, far ones dropped.
fn tiles(mut commands: Commands, mut bd: ResMut<Backdrops>, sim: Res<SimWorld>, cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>, mut images: ResMut<Assets<Image>>) {
    let (tf, loader) = *cam;
    let c = tf.translation.truncate();
    // The ground where the camera is, eased (a cliff isn't a jolt).
    let ground = sim.generator.surface_hint(c.x as i32).map_or(c.y - 60.0, |g| g as f32);
    bd.ground = Some(bd.ground.map_or(ground, |g| g + (ground - g) * 0.05));
    let g = bd.ground.unwrap_or(ground);
    // Underground (well below the ground), they fade away.
    bd.shown = (1.0 - (g - c.y - 40.0) / 80.0).clamp(0.0, 1.0);
    let seed = sim.world.seed();
    let pool = AsyncComputeTaskPool::get();
    let reach = loader.half_extent.x + TILE as f32;
    for (k, &p) in PARALLAX.iter().enumerate() {
        let lo = ((c.x * p - reach) / TILE as f32).floor() as i64;
        let hi = ((c.x * p + reach) / TILE as f32).floor() as i64;
        for t in lo..=hi {
            if bd.tiles.contains_key(&(k, t)) || bd.making.contains_key(&(k, t)) || bd.shown <= 0.0 {
                continue;
            }
            let (scenes, plan) = (bd.scenes.clone(), sim.generator.clone());
            bd.making.insert((k, t), pool.spawn(async move { surface_tile(k, t * TILE as i64, TILE, HEIGHT, seed, &|sx| blend(&scenes, plan.as_ref(), sx / p)) }));
        }
        // Far out of view: gone.
        let stale: Vec<(usize, i64)> = bd.tiles.keys().filter(|(kk, t)| *kk == k && (*t < lo - 2 || *t > hi + 2)).copied().collect();
        for key in stale {
            if let Some(tile) = bd.tiles.remove(&key) {
                commands.entity(tile.entity).despawn();
                commands.entity(tile.skirt).despawn();
            }
        }
    }
    // Made: to images, to sprites.
    let ready: Vec<((usize, i64), Vec<u8>)> = bd.making.iter_mut().filter_map(|(key, task)| block_on(poll_once(task)).map(|px| (*key, px))).collect();
    for (key, px) in ready {
        bd.making.remove(&key);
        let alpha: Vec<u8> = px.chunks(4).map(|p| p[3]).collect();
        let image = Image::new(Extent3d { width: TILE as u32, height: HEIGHT as u32, depth_or_array_layers: 1 }, TextureDimension::D2, px, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
        let handle = images.add(image);
        let z = Z + key.0 as f32 * 0.02;
        let entity = commands.spawn((Name::new("Backdrop tile"), Sprite::from_image(handle.clone()), Transform::from_xyz(0.0, 0.0, z))).id();
        let skirt = commands
            .spawn((
                Name::new("Backdrop skirt"),
                Sprite { image: handle, rect: Some(Rect::new(0.0, HEIGHT as f32 - 1.0, TILE as f32, HEIGHT as f32)), custom_size: Some(Vec2::new(TILE as f32, SKIRT)), ..default() },
                Transform::from_xyz(0.0, 0.0, z),
            ))
            .id();
        bd.tiles.insert(key, Tile { entity, skirt, alpha });
    }
}

/// Each tile where its layer puts it for the camera now; faded underground.
fn place(bd: Res<Backdrops>, cam: Single<&Transform, (With<MainCamera>, Without<Sprite>)>, mut sprites: Query<(&mut Transform, &mut Sprite), Without<MainCamera>>) {
    let c = cam.translation.truncate();
    for ((k, t), tile) in &bd.tiles {
        let at = bd.centre(*k, *t, c);
        for (e, y) in [(tile.entity, at.y), (tile.skirt, at.y - HEIGHT as f32 / 2.0 - SKIRT / 2.0)] {
            if let Ok((mut tf, mut sprite)) = sprites.get_mut(e) {
                tf.translation.x = at.x;
                tf.translation.y = y;
                sprite.color = Color::srgba(1.0, 1.0, 1.0, bd.shown);
            }
        }
    }
}

// ---- the night sky ----

/// The stars (fixed in the sky: a slight drift with the camera) and the
/// moon, over the lighting, where the sky is open.
#[derive(Resource)]
struct NightSky {
    image: Handle<Image>,
    sprite: Entity,
    size: UVec2,
    stars: Vec<Star>,
    since: f32,
}

struct Star {
    /// Where in the sky (0..1 across a sky wider than the view, 0..1 down).
    at: Vec2,
    bright: f32,
    color: [f32; 3],
    /// Twinkling: how fast, from where.
    rate: f32,
    phase: f32,
}

/// Above the light overlay (15), under the glow haze (15.5).
const Z_STARS: f32 = 15.2;
/// The sky's width in views (stars wrap round it).
const SKY_WIDE: f32 = 3.0;

fn stars_setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = UVec2::new(640, 360);
    let image = images.add(Image::new_fill(Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 }, TextureDimension::D2, &[0, 0, 0, 0], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default()));
    let sprite = commands.spawn((Name::new("Night sky"), Sprite::from_image(image.clone()), Transform::from_xyz(0.0, 0.0, Z_STARS), Visibility::Hidden)).id();
    let mut rng = crate::sound::synth::Rng::new(0x5747);
    let stars = (0..1400)
        .map(|_| {
            let b = rng.unit();
            let color = if b > 0.93 { [1.0, 0.84, 0.7] } else if b > 0.85 { [0.72, 0.8, 1.0] } else { [0.93, 0.94, 1.0] };
            Star { at: Vec2::new(rng.unit(), rng.unit().powf(1.3)), bright: 0.2 + 0.8 * b.powi(4), color, rate: 0.5 + 3.0 * rng.unit(), phase: rng.unit() * std::f32::consts::TAU }
        })
        .collect();
    commands.insert_resource(NightSky { image, sprite, size, stars, since: 1.0 });
}

#[allow(clippy::too_many_arguments)]
fn stars(
    time: Res<Time<Real>>,
    day: Option<Res<crate::light::Daylight>>,
    sim: Res<SimWorld>,
    bd: Res<Backdrops>,
    mut sky: ResMut<NightSky>,
    mut images: ResMut<Assets<Image>>,
    cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>,
    mut sprites: Query<(&mut Transform, &mut Sprite, &mut Visibility), Without<MainCamera>>,
) {
    let (tf, loader) = *cam;
    let c = tf.translation.truncate();
    let half = loader.half_extent;
    // Night: how much (0 by day), from the sun's height.
    let night = day.as_ref().map_or(0.0, |d| {
        let sun = -(d.time * std::f32::consts::TAU).cos();
        ((0.05 - sun) / 0.3).clamp(0.0, 1.0)
    }) * bd.shown;
    let sprite = sky.sprite;
    if let Ok((mut stf, mut s, mut vis)) = sprites.get_mut(sprite) {
        *vis = if night > 0.01 { Visibility::Inherited } else { Visibility::Hidden };
        stf.translation = Vec3::new(c.x, c.y, Z_STARS);
        s.custom_size = Some(half * 2.0);
    }
    // (Redrawn 20 times a second: twinkling doesn't need more.)
    sky.since += time.delta_secs();
    if night <= 0.01 || sky.since < 0.05 {
        return;
    }
    sky.since = 0.0;
    let t = time.elapsed_secs();
    let size = sky.size;
    let Some(mut image) = images.get_mut(&sky.image) else { return };
    let Some(data) = image.data.as_mut() else { return };
    data.fill(0);
    let world = &sim.world;
    let mats = world.materials();
    let open = |p: Vec2| {
        let cell = CellPos::from_world(p.x, p.y);
        let empty = |c: Option<platypus_sim::Cell>| c.is_none_or(|c| matches!(mats.phys(c.material).kind, Kind::Empty | Kind::Gas));
        empty(world.get(cell)) && empty(world.get_bg(cell)) && !bd.covers(p, c)
    };
    let cloud = |x: f32| world.weather().map_or(0.0, |w| ((w.overcast(x as i32 - 4, x as i32 + 4) - 0.2) / 0.4).clamp(0.0, 1.0));
    let (vw, vh) = (half.x * 2.0, half.y * 2.0);
    let mut put = |px: f32, py: f32, rgb: [f32; 3], a: f32| {
        // (Pixel coordinates of the image, which spans the view.)
        let (ix, iy) = ((px / vw * size.x as f32) as i64, ((1.0 - py / vh) * size.y as f32) as i64);
        if ix < 0 || iy < 0 || ix >= size.x as i64 || iy >= size.y as i64 {
            return;
        }
        let i = ((iy as u32 * size.x + ix as u32) * 4) as usize;
        for (k, v) in rgb.iter().enumerate() {
            data[i + k] = data[i + k].max((v * 255.0).clamp(0.0, 255.0) as u8);
        }
        data[i + 3] = data[i + 3].max((a * 255.0).clamp(0.0, 255.0) as u8);
    };
    // The sky drifts a little as the camera goes (it's very far).
    let drift = Vec2::new(c.x * 0.02, c.y * 0.01);
    for s in &sky.stars {
        let sx = (s.at.x * vw * SKY_WIDE - drift.x).rem_euclid(vw * SKY_WIDE);
        if sx >= vw {
            continue;
        }
        let sy = vh - s.at.y * vh * 0.9 + drift.y.rem_euclid(1.0);
        let world_at = c - half + Vec2::new(sx, sy);
        if !open(world_at) {
            continue;
        }
        let twinkle = 0.65 + 0.35 * (t * s.rate + s.phase).sin() * (t * s.rate * 0.37 + s.phase * 2.0).sin();
        let a = s.bright * twinkle * night * (1.0 - cloud(world_at.x));
        if a > 0.02 {
            put(sx, sy, s.color, a);
        }
    }
    // The moon: across the sky through the night.
    if let Some(d) = day {
        // (0 at dusk, 1 at dawn: the night half of the day.)
        let phase = (d.time - 0.75).rem_euclid(1.0) * 2.0;
        if (0.0..1.0).contains(&phase) {
            let mx = vw * (0.15 + 0.7 * phase) - drift.x * 0.2;
            let my = vh * (0.55 + 0.35 * (std::f32::consts::PI * phase).sin());
            let r = 7.0;
            for oy in -8..=8 {
                for ox in -8..=8 {
                    let (dx, dy) = (ox as f32, oy as f32);
                    if dx * dx + dy * dy > r * r {
                        continue;
                    }
                    let (px, py) = (mx + dx, my + dy);
                    let world_at = c - half + Vec2::new(px, py);
                    if !open(world_at) {
                        continue;
                    }
                    let crater = platypus_backdrop::noise2(px * 0.5 + 3.0, py * 0.5, 77);
                    let k = 0.78 + 0.22 * crater;
                    put(px, py, [0.9 * k, 0.92 * k, 0.96 * k], night * (1.0 - 0.7 * cloud(world_at.x)));
                }
            }
        }
    }
}

// ---- underground: looks to try (PLATYPUS_UNDERBG) ----

/// The underground's look, to choose from: `void` (dark: what's behind open
/// cave is black but for what light reaches), `motes` (a field of drifting,
/// twinkling glows in open caverns, their colour the zone's; self-lit),
/// `walls` (bioluminescent specks on the back walls, fixed to them),
/// `layers` (cave silhouettes behind, under the lighting: seen only where
/// light reaches).
#[derive(Resource)]
struct Underground {
    look: String,
    image: Handle<Image>,
    sprite: Entity,
    size: UVec2,
    /// `layers`: the backdrop behind (a still of the caverns, drifting).
    far: Option<Entity>,
    since: f32,
}

fn under_setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let look = std::env::var("PLATYPUS_UNDERBG").unwrap_or_else(|_| "void".into());
    let size = UVec2::new(640, 360);
    let image = images.add(Image::new_fill(Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 }, TextureDimension::D2, &[0, 0, 0, 0], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default()));
    let sprite = commands.spawn((Name::new("Underground glows"), Sprite::from_image(image.clone()), Transform::from_xyz(0.0, 0.0, Z_STARS - 0.1), Visibility::Hidden)).id();
    let far = (look == "layers").then(|| {
        let all = scenes();
        let scene = all.iter().find(|s| s.name == "caverns").expect("caverns");
        let (w, h) = (900usize, 400usize);
        let mut c = platypus_backdrop::render(scene, platypus_backdrop::Time::Night, platypus_backdrop::Style::Noita, 5, w, h);
        // (Lit as by day: the lighting darkens it, and only where light
        // reaches does it show.)
        for p in c.px.iter_mut() {
            *p = p.map(|v| (v * 2.8).min(1.0));
        }
        let px = c.to_rgba(32.0);
        let img = images.add(Image::new(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, TextureDimension::D2, px, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD));
        commands.spawn((Name::new("Underground layers"), Sprite::from_image(img), Transform::from_xyz(0.0, 0.0, Z - 0.05), Visibility::Hidden)).id()
    });
    commands.insert_resource(Underground { look, image, sprite, size, far, since: 1.0 });
}

#[allow(clippy::too_many_arguments)]
fn under(
    time: Res<Time<Real>>,
    sim: Res<SimWorld>,
    bd: Res<Backdrops>,
    mut ug: ResMut<Underground>,
    mut images: ResMut<Assets<Image>>,
    cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>,
    mut sprites: Query<(&mut Transform, &mut Sprite, &mut Visibility), Without<MainCamera>>,
) {
    let (tf, loader) = *cam;
    let c = tf.translation.truncate();
    let half = loader.half_extent;
    let deep = 1.0 - bd.shown;
    if let Some(far) = ug.far
        && let Ok((mut ftf, mut s, mut vis)) = sprites.get_mut(far)
    {
        *vis = if deep > 0.01 { Visibility::Inherited } else { Visibility::Hidden };
        // (Drifting at a fifth of the camera's pace: far off.)
        // (A still, for now: centred on the camera.)
        ftf.translation = Vec3::new(c.x, c.y, Z - 0.05);
        s.custom_size = Some(half * 2.0);
        s.color = Color::srgba(1.0, 1.0, 1.0, deep);
    }
    let glows = ug.look == "motes" || ug.look == "walls";
    if let Ok((mut stf, mut s, mut vis)) = sprites.get_mut(ug.sprite) {
        *vis = if glows && deep > 0.01 { Visibility::Inherited } else { Visibility::Hidden };
        stf.translation = Vec3::new(c.x, c.y, Z_STARS - 0.1);
        s.custom_size = Some(half * 2.0);
    }
    ug.since += time.delta_secs();
    if !glows || deep <= 0.01 || ug.since < 0.066 {
        return;
    }
    ug.since = 0.0;
    let t = time.elapsed_secs();
    let size = ug.size;
    let look = ug.look.clone();
    let Some(mut image) = images.get_mut(&ug.image) else { return };
    let Some(data) = image.data.as_mut() else { return };
    data.fill(0);
    let world = &sim.world;
    let mats = world.materials();
    let empty = |c: Option<platypus_sim::Cell>| c.is_none_or(|c| matches!(mats.phys(c.material).kind, Kind::Empty | Kind::Gas));
    let (vw, vh) = (half.x * 2.0, half.y * 2.0);
    let origin = c - half;
    // The zone's colour.
    let tint = match sim.generator.zone_at(c.x as i32, c.y as i32) {
        Some("fungal") => [0.45, 1.0, 0.85],
        Some("crystal") => [0.8, 0.6, 1.0],
        Some("toxic") => [0.7, 1.0, 0.3],
        _ => [0.55, 0.78, 1.0],
    };
    let mut put = |wx: f32, wy: f32, rgb: [f32; 3], a: f32| {
        let (ix, iy) = (((wx - origin.x) / vw * size.x as f32) as i64, ((1.0 - (wy - origin.y) / vh) * size.y as f32) as i64);
        if ix < 0 || iy < 0 || ix >= size.x as i64 || iy >= size.y as i64 {
            return;
        }
        let i = ((iy as u32 * size.x + ix as u32) * 4) as usize;
        for (k, v) in rgb.iter().enumerate() {
            data[i + k] = data[i + k].max((v * 255.0).clamp(0.0, 255.0) as u8);
        }
        data[i + 3] = data[i + 3].max((a * 255.0).clamp(0.0, 255.0) as u8);
    };
    if look == "motes" {
        // A field of glows three views wide, drifting up, in open cave only.
        let mut rng = crate::sound::synth::Rng::new(0x3073);
        for _ in 0..1600 {
            let (u, v, b, rate, phase) = (rng.unit(), rng.unit(), rng.unit(), 0.3 + 2.0 * rng.unit(), rng.unit() * std::f32::consts::TAU);
            let x = (u * vw * 3.0 - c.x * 0.15).rem_euclid(vw * 3.0);
            if x >= vw {
                continue;
            }
            let y = (v * vh + t * (1.0 + 3.0 * b) - c.y * 0.1).rem_euclid(vh);
            let (wx, wy) = (origin.x + x, origin.y + y);
            let cell = CellPos::from_world(wx, wy);
            if !(empty(world.get(cell)) && empty(world.get_bg(cell))) {
                continue;
            }
            let tw = 0.5 + 0.5 * (t * rate + phase).sin();
            let a = (0.15 + 0.85 * b.powi(3)) * tw * deep;
            put(wx, wy, tint, a);
            if b > 0.9 {
                for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                    put(wx + dx, wy + dy, tint, a * 0.35);
                }
            }
        }
        // A faint haze of it (in half-size blocks), in open cave.
        for by in (0..size.y).step_by(2) {
            for bx in (0..size.x).step_by(2) {
                let wx = origin.x + bx as f32 / size.x as f32 * vw;
                let wy = origin.y + (1.0 - by as f32 / size.y as f32) * vh;
                let cell = CellPos::from_world(wx, wy);
                if !(empty(world.get(cell)) && empty(world.get_bg(cell))) {
                    continue;
                }
                let n = platypus_backdrop::fbm2(wx * 0.01 + t * 0.02, wy * 0.015, 3, 91);
                let a = ((n - 0.5) * 0.25).clamp(0.0, 0.05) * deep;
                if a > 0.0 {
                    for dy in 0..2 {
                        for dx in 0..2 {
                            let i = (((by + dy).min(size.y - 1) * size.x + (bx + dx).min(size.x - 1)) * 4) as usize;
                            if data[i + 3] == 0 {
                                data[i] = (tint[0] * 255.0 * 0.6) as u8;
                                data[i + 1] = (tint[1] * 255.0 * 0.6) as u8;
                                data[i + 2] = (tint[2] * 255.0 * 0.6) as u8;
                                data[i + 3] = (a * 255.0) as u8;
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Specks fixed on the back walls (where the playfield is open): a
        // glowworm's thread, a lichen's dot, slowly breathing.
        let (x0, y0) = (origin.x.floor() as i32, origin.y.floor() as i32);
        let (x1, y1) = (x0 + vw as i32, y0 + vh as i32);
        for by in (y0.div_euclid(5))..=(y1.div_euclid(5)) {
            for bx in (x0.div_euclid(5))..=(x1.div_euclid(5)) {
                let h = platypus_sim::rng::hash(&[bx as u64, by as u64, 0x6c]) % 1000;
                if h > 70 {
                    continue;
                }
                let (wx, wy) = ((bx * 5 + (h % 5) as i32) as f32 + 0.5, (by * 5 + (h / 5 % 5) as i32) as f32 + 0.5);
                let cell = CellPos::from_world(wx, wy);
                if !empty(world.get(cell)) || empty(world.get_bg(cell)) {
                    continue;
                }
                let breathe = 0.55 + 0.45 * (t * (0.4 + (h % 7) as f32 * 0.1) + h as f32).sin();
                let a = (0.5 + 0.5 * (h as f32 / 70.0)) * breathe * deep;
                put(wx, wy, tint, a);
                put(wx, wy - 1.0, tint, a * 0.3);
            }
        }
    }
}
