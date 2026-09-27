//! Backdrops (DESIGN §4.3b): what's far behind the world.
//!
//! - The surface: distant mountains, the same everywhere (a far snowy
//!   range, a nearer one with foothills and tiny trees), barely moving as
//!   you go (they're far), sitting on the ground as generated, fading out
//!   underground. Over them, the sun by day, the moon and stars by night.
//! - Underground: rock far off through the cave, two layers, drifting
//!   slowly, in daylight colours under the lighting: only what your light
//!   reaches shows. In fungal and crystal zones the back walls glow with
//!   little specks; big open caverns fill with faint drifting motes.
//! - Tiles are made in the background as the camera goes (from absolute
//!   coordinates: they join up) and dropped when far. The sun, moon,
//!   stars, specks and motes are drawn over the lighting, where there's
//!   open sky or open cave.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use platypus_backdrop::tile::{CAVE_LAYERS, SURFACE_LAYERS, cave_tile, mountains_tile};
use platypus_sim::{CellPos, Kind};

use crate::camera::MainCamera;
use crate::sound::synth::Rng;
use crate::world::{ChunkLoader, SimWorld};

pub struct BackdropPlugin;

impl Plugin for BackdropPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Backdrops::default())
            .add_systems(Startup, sky_setup)
            .add_systems(PostUpdate, (tiles, place, sky).chain().after(crate::camera::follow).before(bevy::transform::TransformSystems::Propagate));
    }
}

/// Each layer's share of the camera's motion: far, so little.
const SURFACE_PARALLAX: [f32; SURFACE_LAYERS] = [0.02, 0.05];
const CAVE_PARALLAX: [f32; CAVE_LAYERS] = [0.04, 0.1];
/// The surface's strips: columns a tile, rows.
const TILE: usize = 256;
const HEIGHT: usize = 260;
/// The underground's tiles: square.
const CAVE_TILE: usize = 128;
/// How far below a surface tile its bottom row is stretched (valleys).
const SKIRT: f32 = 300.0;
/// Behind the world's back walls (-1), in front of the clouds (-2).
const Z: f32 = -1.8;
/// Over the light overlay (15), under the glow haze (15.5).
const Z_SKY: f32 = 15.2;

/// A layer's tile: which kind, which layer, where (column; row for caves).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Key {
    cave: bool,
    k: usize,
    tx: i64,
    ty: i64,
}

struct Tile {
    entity: Entity,
    /// A surface tile's bottom row stretched down.
    skirt: Option<Entity>,
    /// Its alpha (the stars hide behind it).
    alpha: Vec<u8>,
}

#[derive(Resource, Default)]
pub struct Backdrops {
    tiles: HashMap<Key, Tile>,
    making: HashMap<Key, Task<Vec<u8>>>,
    /// The ground's height where the camera is (eased), the mountains' foot.
    ground: Option<f32>,
    /// The surface's showing (1) or the underground's (0).
    shown: f32,
}

impl Backdrops {
    /// Where a tile is drawn (its centre), for the camera at `cam`.
    fn centre(&self, key: Key, cam: Vec2) -> Vec2 {
        if key.cave {
            let p = CAVE_PARALLAX[key.k];
            let t = CAVE_TILE as f32;
            // (Rows go down: world y is up.)
            return Vec2::new(((key.tx as f32 + 0.5) * t + cam.x * (1.0 - p)).round(), (-(key.ty as f32 + 0.5) * t + cam.y * (1.0 - p)).round());
        }
        let p = SURFACE_PARALLAX[key.k];
        let ground = self.ground.unwrap_or(cam.y);
        let x = (key.tx as f32 + 0.5) * TILE as f32 + cam.x * (1.0 - p);
        // Its foot a little under the ground; it hardly moves as you climb.
        let foot = ground - 30.0 + (cam.y - ground) * (1.0 - p);
        Vec2::new(x.round(), (foot + HEIGHT as f32 / 2.0).round())
    }

    /// Is a mountain in front of this point of the sky?
    fn covers(&self, at: Vec2, cam: Vec2) -> bool {
        (0..SURFACE_LAYERS).any(|k| {
            let p = SURFACE_PARALLAX[k];
            let sx = at.x - cam.x * (1.0 - p);
            let tx = (sx / TILE as f32).floor() as i64;
            let key = Key { cave: false, k, tx, ty: 0 };
            let Some(tile) = self.tiles.get(&key) else { return false };
            let c = self.centre(key, cam);
            let (lx, ly) = ((sx - tx as f32 * TILE as f32) as usize, (c.y + HEIGHT as f32 / 2.0 - at.y) as i64);
            // (Below a tile: its skirt, which is solid.)
            ly >= HEIGHT as i64 || ((0..HEIGHT as i64).contains(&ly) && lx < TILE && tile.alpha[ly as usize * TILE + lx] > 40)
        })
    }
}

/// Tiles in view made, far ones dropped; the surface's or the cave's by
/// depth.
fn tiles(mut commands: Commands, mut bd: ResMut<Backdrops>, sim: Res<SimWorld>, cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>, mut images: ResMut<Assets<Image>>) {
    let (tf, loader) = *cam;
    let c = tf.translation.truncate();
    let half = loader.half_extent;
    // The ground where the camera is, eased (a cliff isn't a jolt).
    let ground = sim.generator.surface_hint(c.x as i32).map_or(c.y - 60.0, |g| g as f32);
    bd.ground = Some(bd.ground.map_or(ground, |g| g + (ground - g) * 0.05));
    let g = bd.ground.unwrap_or(ground);
    bd.shown = (1.0 - (g - c.y - 40.0) / 80.0).clamp(0.0, 1.0);
    let seed = sim.world.seed();
    let pool = AsyncComputeTaskPool::get();
    let mut want: Vec<Key> = Vec::new();
    if bd.shown > 0.0 {
        for (k, &p) in SURFACE_PARALLAX.iter().enumerate() {
            let (lo, hi) = (((c.x * p - half.x - 32.0) / TILE as f32).floor() as i64, ((c.x * p + half.x + 32.0) / TILE as f32).floor() as i64);
            want.extend((lo..=hi).map(|tx| Key { cave: false, k, tx, ty: 0 }));
        }
    }
    if bd.shown < 1.0 {
        let t = CAVE_TILE as f32;
        for (k, &p) in CAVE_PARALLAX.iter().enumerate() {
            let (sx, sy) = (c.x * p, -c.y * p);
            let (x0, x1) = (((sx - half.x - 16.0) / t).floor() as i64, ((sx + half.x + 16.0) / t).floor() as i64);
            let (y0, y1) = (((sy - half.y - 16.0) / t).floor() as i64, ((sy + half.y + 16.0) / t).floor() as i64);
            for ty in y0..=y1 {
                want.extend((x0..=x1).map(|tx| Key { cave: true, k, tx, ty }));
            }
        }
    }
    for &key in &want {
        if bd.tiles.contains_key(&key) || bd.making.contains_key(&key) {
            continue;
        }
        let task = if key.cave {
            let t = CAVE_TILE as i64;
            pool.spawn(async move { cave_tile(key.k, key.tx * t, key.ty * t, CAVE_TILE, CAVE_TILE, seed) })
        } else {
            pool.spawn(async move { mountains_tile(key.k, key.tx * TILE as i64, TILE, HEIGHT, seed) })
        };
        bd.making.insert(key, task);
    }
    // Out of view (and a margin): gone.
    let stale: Vec<Key> = bd.tiles.keys().filter(|key| !want.iter().any(|w| w.cave == key.cave && w.k == key.k && (w.tx - key.tx).abs() <= 2 && (w.ty - key.ty).abs() <= 2)).copied().collect();
    for key in stale {
        if let Some(tile) = bd.tiles.remove(&key) {
            commands.entity(tile.entity).despawn();
            if let Some(s) = tile.skirt {
                commands.entity(s).despawn();
            }
        }
    }
    // Made: to images, to sprites.
    let ready: Vec<(Key, Vec<u8>)> = bd.making.iter_mut().filter_map(|(key, task)| block_on(poll_once(task)).map(|px| (*key, px))).collect();
    for (key, px) in ready {
        bd.making.remove(&key);
        let (w, h) = if key.cave { (CAVE_TILE, CAVE_TILE) } else { (TILE, HEIGHT) };
        let alpha: Vec<u8> = if key.cave { Vec::new() } else { px.chunks(4).map(|p| p[3]).collect() };
        let handle = images.add(Image::new(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, TextureDimension::D2, px, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD));
        // (The caves behind the mountains: they never show together.)
        let z = Z + key.k as f32 * 0.02 - if key.cave { 0.1 } else { 0.0 };
        let entity = commands.spawn((Name::new("Backdrop tile"), Sprite::from_image(handle.clone()), Transform::from_xyz(0.0, 0.0, z))).id();
        let skirt = (!key.cave).then(|| {
            commands
                .spawn((
                    Name::new("Backdrop skirt"),
                    Sprite { image: handle, rect: Some(Rect::new(0.0, HEIGHT as f32 - 1.0, TILE as f32, HEIGHT as f32)), custom_size: Some(Vec2::new(TILE as f32, SKIRT)), ..default() },
                    Transform::from_xyz(0.0, 0.0, z),
                ))
                .id()
        });
        bd.tiles.insert(key, Tile { entity, skirt, alpha });
    }
}

/// Each tile where its layer puts it for the camera now; the surface's
/// faded out underground, the cave's in.
fn place(bd: Res<Backdrops>, cam: Single<&Transform, (With<MainCamera>, Without<Sprite>)>, mut sprites: Query<(&mut Transform, &mut Sprite), Without<MainCamera>>) {
    let c = cam.translation.truncate();
    for (key, tile) in &bd.tiles {
        let at = bd.centre(*key, c);
        let alpha = if key.cave { 1.0 - bd.shown } else { bd.shown };
        let skirt = tile.skirt.map(|s| (s, at.y - HEIGHT as f32 / 2.0 - SKIRT / 2.0));
        for (e, y) in std::iter::once((tile.entity, at.y)).chain(skirt) {
            if let Ok((mut tf, mut sprite)) = sprites.get_mut(e) {
                tf.translation.x = at.x;
                tf.translation.y = y;
                sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
            }
        }
    }
}

// ---- the sky, and the cave's own lights ----

/// The sun, the moon and the stars (the sky's: a slight drift with the
/// camera); underground, glowing specks on the walls (fungal and crystal
/// zones) and motes in big caverns. Over the lighting, where it's open.
#[derive(Resource)]
struct Sky {
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

/// The sky's width in views (stars wrap round it).
const SKY_WIDE: f32 = 3.0;

fn sky_setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = UVec2::new(640, 360);
    let image = images.add(Image::new_fill(Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 }, TextureDimension::D2, &[0, 0, 0, 0], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default()));
    let sprite = commands.spawn((Name::new("Sky"), Sprite::from_image(image.clone()), Transform::from_xyz(0.0, 0.0, Z_SKY))).id();
    let mut rng = Rng::new(0x5747);
    let stars = (0..1400)
        .map(|_| {
            let b = rng.unit();
            let color = if b > 0.93 { [1.0, 0.84, 0.7] } else if b > 0.85 { [0.72, 0.8, 1.0] } else { [0.93, 0.94, 1.0] };
            Star { at: Vec2::new(rng.unit(), rng.unit().powf(1.3)), bright: 0.2 + 0.8 * b.powi(4), color, rate: 0.5 + 3.0 * rng.unit(), phase: rng.unit() * std::f32::consts::TAU }
        })
        .collect();
    commands.insert_resource(Sky { image, sprite, size, stars, since: 1.0 });
}

#[allow(clippy::too_many_arguments)]
fn sky(
    time: Res<Time<Real>>,
    day: Option<Res<crate::light::Daylight>>,
    sim: Res<SimWorld>,
    bd: Res<Backdrops>,
    mut sky: ResMut<Sky>,
    mut images: ResMut<Assets<Image>>,
    cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>,
    mut sprites: Query<(&mut Transform, &mut Sprite), Without<MainCamera>>,
) {
    let (tf, loader) = *cam;
    let c = tf.translation.truncate();
    let half = loader.half_extent;
    if let Ok((mut stf, mut s)) = sprites.get_mut(sky.sprite) {
        stf.translation = Vec3::new(c.x, c.y, Z_SKY);
        s.custom_size = Some(half * 2.0);
    }
    // (Redrawn 20 times a second: twinkling doesn't need more.)
    sky.since += time.delta_secs();
    if sky.since < 0.05 {
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
    let empty = |cell: Option<platypus_sim::Cell>| cell.is_none_or(|c| matches!(mats.phys(c.material).kind, Kind::Empty | Kind::Gas));
    let open = |p: Vec2| {
        let cell = CellPos::from_world(p.x, p.y);
        empty(world.get(cell)) && empty(world.get_bg(cell))
    };
    let (vw, vh) = (half.x * 2.0, half.y * 2.0);
    let origin = c - half;
    // (x, y: from the view's bottom left, in cells.)
    let mut put = |x: f32, y: f32, rgb: [f32; 3], a: f32| {
        let (ix, iy) = ((x / vw * size.x as f32) as i64, ((1.0 - y / vh) * size.y as f32) as i64);
        if ix < 0 || iy < 0 || ix >= size.x as i64 || iy >= size.y as i64 || a <= 0.0 {
            return;
        }
        let i = ((iy as u32 * size.x + ix as u32) * 4) as usize;
        for (k, v) in rgb.iter().enumerate() {
            data[i + k] = data[i + k].max((v * 255.0).clamp(0.0, 255.0) as u8);
        }
        data[i + 3] = data[i + 3].max((a * 255.0).clamp(0.0, 255.0) as u8);
    };
    let surface = bd.shown;
    if surface > 0.01
        && let Some(d) = day.as_ref()
    {
        // Night: how much (0 by day), from the sun's height.
        let sun_h = -(d.time * std::f32::consts::TAU).cos();
        let night = ((0.05 - sun_h) / 0.3).clamp(0.0, 1.0) * surface;
        let cloud = |x: f32| world.weather().map_or(0.0, |w| ((w.overcast(x as i32 - 4, x as i32 + 4) - 0.2) / 0.4).clamp(0.0, 1.0));
        let sky_open = |p: Vec2| open(p) && !bd.covers(p, c);
        // (The sky drifts a little as the camera goes: it's very far.)
        let drift = c.x * 0.01;
        if night > 0.01 {
            for s in &sky.stars {
                let sx = (s.at.x * vw * SKY_WIDE - drift).rem_euclid(vw * SKY_WIDE);
                if sx >= vw {
                    continue;
                }
                let sy = vh - s.at.y * vh * 0.9;
                let p = origin + Vec2::new(sx, sy);
                if !sky_open(p) {
                    continue;
                }
                let twinkle = 0.65 + 0.35 * (t * s.rate + s.phase).sin() * (t * s.rate * 0.37 + s.phase * 2.0).sin();
                put(sx, sy, s.color, s.bright * twinkle * night * (1.0 - cloud(p.x)));
            }
        }
        // A disc crossing the sky: the sun by day (low and golden at the
        // ends of the day), the moon by night; a soft glow round it.
        let is_sun = (0.25..0.75).contains(&d.time);
        let phase = if is_sun { (d.time - 0.25) * 2.0 } else { (d.time - 0.75).rem_euclid(1.0) * 2.0 };
        let arc = (std::f32::consts::PI * phase).sin();
        let (dx, dy) = (vw * (0.12 + 0.76 * phase) - drift * 0.3, vh * (0.6 + 0.34 * arc));
        let low = 1.0 - arc;
        let (radius, glow, body) = if is_sun { (6.0, 26.0, [1.0, 0.96 - 0.25 * low, 0.84 - 0.5 * low]) } else { (5.0, 14.0, [0.9, 0.92, 0.97]) };
        let fade = if is_sun { surface } else { night };
        // (Over the image's pixels, finer than cells: no gaps.)
        let step = vw / size.x as f32;
        let reach = (glow / step) as i32;
        for oy in -reach..=reach {
            for ox in -reach..=reach {
                let r = ((ox * ox + oy * oy) as f32).sqrt() * step;
                if r > glow || fade <= 0.01 {
                    continue;
                }
                let (px, py) = (dx + ox as f32 * step, dy + oy as f32 * step);
                let p = origin + Vec2::new(px, py);
                if !sky_open(p) {
                    continue;
                }
                let clouded = 1.0 - 0.8 * cloud(p.x);
                if r <= radius {
                    let k = if is_sun { 1.0 } else { 0.78 + 0.22 * platypus_backdrop::noise2(px * 0.5 + 3.0, py * 0.5, 77) };
                    put(px, py, body.map(|v| v * k), fade * clouded);
                } else {
                    let a = (1.0 - (r - radius) / (glow - radius)).powi(2) * if is_sun { 0.35 } else { 0.18 };
                    put(px, py, body, a * fade * clouded);
                }
            }
        }
    }
    if surface < 0.99 {
        let deep = 1.0 - surface;
        let tint = |zone: Option<&str>| match zone {
            Some("fungal") => [0.45, 1.0, 0.85],
            Some("crystal") => [0.8, 0.6, 1.0],
            Some("toxic") => [0.7, 1.0, 0.3],
            _ => [0.55, 0.78, 1.0],
        };
        // Specks on the back walls, fungal and crystal zones only: a
        // glowworm's thread, a lichen's dot, fixed to the walls, slowly
        // breathing.
        let (x0, y0) = (origin.x.floor() as i32, origin.y.floor() as i32);
        let (x1, y1) = (x0 + vw as i32, y0 + vh as i32);
        for by in y0.div_euclid(5)..=y1.div_euclid(5) {
            for bx in x0.div_euclid(5)..=x1.div_euclid(5) {
                let h = platypus_sim::rng::hash(&[bx as u64, by as u64, 0x6c]) % 1000;
                if h > 70 {
                    continue;
                }
                let (wx, wy) = ((bx * 5 + (h % 5) as i32) as f32 + 0.5, (by * 5 + (h / 5 % 5) as i32) as f32 + 0.5);
                let cell = CellPos::from_world(wx, wy);
                if !empty(world.get(cell)) || empty(world.get_bg(cell)) {
                    continue;
                }
                let zone = sim.generator.zone_at(wx as i32, wy as i32);
                if !matches!(zone, Some("fungal") | Some("crystal")) {
                    continue;
                }
                let breathe = 0.55 + 0.45 * (t * (0.4 + (h % 7) as f32 * 0.1) + h as f32).sin();
                let a = (0.5 + 0.5 * (h as f32 / 70.0)) * breathe * deep;
                put(wx - origin.x, wy - origin.y, tint(zone), a);
                put(wx - origin.x, wy - origin.y - 1.0, tint(zone), a * 0.3);
            }
        }
        // Motes: only where the cave opens up (most of the view open).
        let (gx, gy) = (24, 14);
        let opened = (0..gx).flat_map(|i| (0..gy).map(move |j| (i, j))).filter(|&(i, j)| open(origin + Vec2::new((i as f32 + 0.5) / gx as f32 * vw, (j as f32 + 0.5) / gy as f32 * vh))).count();
        let big = ((opened as f32 / (gx * gy) as f32 - 0.3) / 0.25).clamp(0.0, 1.0) * deep;
        if big > 0.01 {
            let color = tint(sim.generator.zone_at(c.x as i32, c.y as i32));
            let mut rng = Rng::new(0x3073);
            for _ in 0..1200 {
                let (u, v, b, rate, phase) = (rng.unit(), rng.unit(), rng.unit(), 0.3 + 2.0 * rng.unit(), rng.unit() * std::f32::consts::TAU);
                let x = (u * vw * 3.0 - c.x * 0.05).rem_euclid(vw * 3.0);
                if x >= vw {
                    continue;
                }
                let y = (v * vh + t * (1.0 + 3.0 * b) - c.y * 0.05).rem_euclid(vh);
                if !open(origin + Vec2::new(x, y)) {
                    continue;
                }
                let tw = 0.5 + 0.5 * (t * rate + phase).sin();
                put(x, y, color, (0.12 + 0.7 * b.powi(3)) * tw * big);
            }
        }
    }
}
