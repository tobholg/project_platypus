//! Backdrops (DESIGN §4.3b): what's far behind the world.
//!
//! - The surface: a look by biome (`platypus_backdrop::peaks`), Noita-
//!   like: tall sharp peaks cut into flat lit and shaded faces, snow on
//!   the tops, range behind range fading into the sky, a dark lowland at
//!   their feet, each range moving at a small share of the camera's
//!   motion (far, so little); big cloud heaps rising behind the ranges,
//!   drifting. The biomes around the camera choose the look: travelling,
//!   one fades into the next. They sit on the ground as generated and
//!   fade out underground. Over them, the sun by day, the moon and stars
//!   by night; behind, the sky darkening upward.
//! - Underground: behind the back walls, a dark void, tinted a little by
//!   the band and zone you're in (under the lighting: only near a light
//!   does the tint show), and in its deep dark a few faint twinkles, far
//!   off, drifting slowly (over the lighting, where it's open).
//! - Tiles are made in the background as the camera goes (from absolute
//!   coordinates: they join up) and dropped when far. The sun, moon,
//!   stars and twinkles are drawn over the lighting, where it's open.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use platypus_backdrop::peaks::{Look, cloud_tile, for_biome, layer_tile, looks};
use platypus_sim::{CellPos, Kind};

use crate::camera::MainCamera;
use crate::sound::synth::Rng;
use crate::world::{ChunkLoader, SimWorld};

pub struct BackdropPlugin;

impl Plugin for BackdropPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Backdrops::new())
            .add_systems(Startup, (sky_setup, gradient_setup, void_setup))
            .add_systems(PostUpdate, (tiles, place, gradient, void, sky).chain().after(crate::camera::follow).before(bevy::transform::TransformSystems::Propagate));
    }
}

/// The surface's layers, far to near: their shares of the camera's motion
/// across run from the farthest's to the nearest's (far, so little), up
/// and down far less (the ranges sit still in the view however high or
/// low on the surface you are); their feet step down to the ground by
/// `FEET_STEP` a layer (cells).
const PARALLAX: (f32, f32) = (0.01, 0.04);
const PARALLAX_Y: (f32, f32) = (0.004, 0.015);
const FEET_STEP: f32 = 12.0;

/// Layer `k` of `n`: its share of the camera's motion across and up and
/// down, its feet over the ground.
fn layer_at(k: usize, n: usize) -> (f32, f32, f32) {
    let t = k as f32 / (n.max(2) - 1) as f32;
    let lerp = |(a, b): (f32, f32)| a + (b - a) * t;
    (lerp(PARALLAX), lerp(PARALLAX_Y), (n - 1 - k.min(n - 1)) as f32 * FEET_STEP)
}
/// The far clouds: their share of the camera's motion, how fast they
/// drift (cells a second), where their strip's bottom sits above the
/// ground.
const CLOUD_PARALLAX: f32 = 0.008;
const CLOUD_PARALLAX_Y: f32 = 0.004;
const CLOUD_DRIFT: f32 = 1.2;
const CLOUD_FOOT: f32 = 12.0;
/// The surface's strips: columns a tile, rows (layers, clouds).
const TILE: usize = 256;
const HEIGHT: usize = 380;
const CLOUD_HEIGHT: usize = 170;
/// How far below a surface tile its bottom row is stretched (valleys).
const SKIRT: f32 = 300.0;
/// Behind the weather's clouds (-2) and the world's back walls (-1); the
/// sky's gradient behind all.
const Z: f32 = -2.6;
const Z_GRADIENT: f32 = -3.0;
/// Over the light overlay (15), under the glow haze (15.5).
const Z_SKY: f32 = 15.2;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Part {
    /// A look's layer.
    Land,
    Clouds,
}

/// A tile: what, which look and layer, which column (`ty` unused: 0).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Key {
    part: Part,
    v: usize,
    k: usize,
    tx: i64,
    ty: i64,
}

struct Tile {
    entity: Entity,
    /// A land tile's bottom row stretched down.
    skirt: Option<Entity>,
    /// Its alpha (the stars hide behind it).
    alpha: Vec<u8>,
}

#[derive(Resource)]
pub struct Backdrops {
    looks: Arc<Vec<Look>>,
    /// Each look's share of the view now (eased towards the biomes'
    /// around the camera).
    weights: Vec<f32>,
    tiles: HashMap<Key, Tile>,
    making: HashMap<Key, Task<Vec<u8>>>,
    /// The ground's height where the camera is (eased), the layers' foot.
    ground: Option<f32>,
    /// The surface's showing (1) or the underground's (0).
    shown: f32,
    /// How far the clouds have drifted (cells).
    drift: f32,
}

impl Backdrops {
    fn new() -> Self {
        let looks = Arc::new(looks());
        let weights = looks.iter().map(|v| if v.name == "noita" { 1.0 } else { 0.0 }).collect();
        Backdrops { looks, weights, tiles: HashMap::new(), making: HashMap::new(), ground: None, shown: 1.0, drift: 0.0 }
    }

    /// A tile's size in cells.
    fn size(part: Part) -> (usize, usize) {
        match part {
            Part::Land => (TILE, HEIGHT),
            Part::Clouds => (TILE, CLOUD_HEIGHT),
        }
    }

    /// A strip's scroll for the camera at `cam`: world x = strip x + this.
    /// (For land, `k` counts layers of look `v`.)
    fn scroll(&self, part: Part, v: usize, k: usize, cam: Vec2) -> f32 {
        match part {
            Part::Land => cam.x * (1.0 - layer_at(k, self.looks[v].layers.len()).0),
            Part::Clouds => cam.x * (1.0 - CLOUD_PARALLAX) + self.drift,
        }
    }

    /// Where a tile is drawn (its centre), for the camera at `cam`.
    fn centre(&self, key: Key, cam: Vec2) -> Vec2 {
        let (w, h) = Self::size(key.part);
        let x = (key.tx as f32 + 0.5) * w as f32 + self.scroll(key.part, key.v, key.k, cam);
        let ground = self.ground.unwrap_or(cam.y);
        let y = match key.part {
            // Its foot (the layer's base row) a little above the ground; it
            // hardly moves as you climb.
            Part::Land => {
                let (_, py, feet) = layer_at(key.k, self.looks[key.v].layers.len());
                let foot = ground + feet + (cam.y - ground) * (1.0 - py);
                foot + self.looks[key.v].layers[key.k].base * h as f32 - h as f32 / 2.0
            }
            Part::Clouds => ground + CLOUD_FOOT + (cam.y - ground) * (1.0 - CLOUD_PARALLAX_Y) + h as f32 / 2.0,
        };
        Vec2::new(x, y)
    }

    /// What hides the sky now: each mostly shown look tile (layer or
    /// cloud) where it's drawn, for `Cover::covers`.
    fn cover(&self, cam: Vec2) -> Cover<'_> {
        let tiles = self
            .tiles
            .iter()
            .filter(|(key, _)| self.weights[key.v] >= 0.5)
            .map(|(key, tile)| {
                let (w, h) = Self::size(key.part);
                let c = self.centre(*key, cam);
                (Vec2::new(c.x - w as f32 / 2.0, c.y + h as f32 / 2.0), w, h, key.part == Part::Land, tile.alpha.as_slice())
            })
            .collect();
        Cover { tiles }
    }
}

/// The tiles hiding the sky: top-left corner, size, land (its skirt below
/// is solid), alpha.
struct Cover<'a> {
    tiles: Vec<(Vec2, usize, usize, bool, &'a [u8])>,
}

impl Cover<'_> {
    /// Is a look's layer or cloud in front of this point of the sky?
    fn covers(&self, at: Vec2) -> bool {
        self.tiles.iter().any(|&(corner, w, h, land, alpha)| {
            let (lx, ly) = ((at.x - corner.x).floor() as i64, (corner.y - at.y).floor() as i64);
            if lx < 0 || lx >= w as i64 || ly < 0 {
                return false;
            }
            (land && ly >= h as i64) || (ly < h as i64 && alpha[ly as usize * w + lx as usize] > 40)
        })
    }
}

/// The looks' shares: the biomes along the ground around the camera,
/// nearer ones counting more.
fn targets(bd: &Backdrops, sim: &SimWorld, x: f32) -> Vec<f32> {
    let mut out = vec![0.0; bd.looks.len()];
    for (d, w) in [(-3.0, 0.05), (-2.0, 0.1), (-1.0, 0.2), (0.0, 0.3), (1.0, 0.2), (2.0, 0.1), (3.0, 0.05)] {
        let name = for_biome(sim.generator.biome_hint((x + d * 150.0) as i32).unwrap_or("forest"));
        if let Some(i) = bd.looks.iter().position(|v| v.name == name) {
            out[i] += w;
        }
    }
    // (A trace of a look isn't worth its tiles: the rest share it.)
    let kept: f32 = out.iter().filter(|&&w| w >= 0.15).sum();
    out.iter().map(|&w| if w >= 0.15 { w / kept } else { 0.0 }).collect()
}

/// Tiles in view made, far ones dropped; the surface's (the looks
/// showing) or the cave's by depth.
fn tiles(mut commands: Commands, mut bd: ResMut<Backdrops>, sim: Res<SimWorld>, time: Res<Time>, cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>, mut images: ResMut<Assets<Image>>) {
    let (tf, loader) = *cam;
    let c = tf.translation.truncate();
    let half = loader.half_extent;
    // The ground where the camera is, eased (a cliff isn't a jolt).
    let ground = sim.generator.surface_hint(c.x as i32).map_or(c.y - 60.0, |g| g as f32);
    bd.ground = Some(bd.ground.map_or(ground, |g| g + (ground - g) * 0.05));
    let g = bd.ground.unwrap_or(ground);
    bd.shown = (1.0 - (g - c.y - 40.0) / 80.0).clamp(0.0, 1.0);
    bd.drift += time.delta_secs() * CLOUD_DRIFT;
    // The looks' shares, eased (about a second to change over).
    let target = targets(&bd, &sim, c.x);
    let ease = 1.0 - (-time.delta_secs() / 0.8).exp();
    for (w, t) in bd.weights.iter_mut().zip(&target) {
        *w += (t - *w) * ease;
        if *w < 0.01 && *t == 0.0 {
            *w = 0.0;
        }
    }
    let seed = sim.world.seed();
    let pool = AsyncComputeTaskPool::get();
    let mut want: Vec<Key> = Vec::new();
    let span = |bd: &Backdrops, part: Part, v: usize, k: usize| {
        let (w, _) = Backdrops::size(part);
        let s = c.x - bd.scroll(part, v, k, c);
        (((s - half.x - 32.0) / w as f32).floor() as i64, ((s + half.x + 32.0) / w as f32).floor() as i64)
    };
    if bd.shown > 0.0 {
        for v in (0..bd.looks.len()).filter(|&v| bd.weights[v] > 0.0) {
            for k in 0..bd.looks[v].layers.len() {
                let (lo, hi) = span(&bd, Part::Land, v, k);
                want.extend((lo..=hi).map(|tx| Key { part: Part::Land, v, k, tx, ty: 0 }));
            }
            let (lo, hi) = span(&bd, Part::Clouds, v, 0);
            want.extend((lo..=hi).map(|tx| Key { part: Part::Clouds, v, k: 0, tx, ty: 0 }));
        }
    }
    for &key in &want {
        if bd.tiles.contains_key(&key) || bd.making.contains_key(&key) {
            continue;
        }
        let (w, h) = Backdrops::size(key.part);
        let looks = bd.looks.clone();
        let task = match key.part {
            Part::Land => pool.spawn(async move { layer_tile(&looks[key.v], key.k, key.tx * w as i64, w, h, seed) }),
            Part::Clouds => pool.spawn(async move { cloud_tile(&looks[key.v], key.tx * w as i64, w, h, seed) }),
        };
        bd.making.insert(key, task);
    }
    // Out of view (and a margin), or its look gone: gone.
    let stale: Vec<Key> = bd.tiles.keys().filter(|key| !want.iter().any(|w| w.part == key.part && w.v == key.v && w.k == key.k && (w.tx - key.tx).abs() <= 2 && (w.ty - key.ty).abs() <= 2)).copied().collect();
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
        let (w, h) = Backdrops::size(key.part);
        let alpha: Vec<u8> = px.chunks(4).map(|p| p[3]).collect();
        let handle = images.add(Image::new(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, TextureDimension::D2, px, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD));
        // The farthest range, the clouds, the nearer ranges.
        let z = Z + key.v as f32 * 0.002
            + match key.part {
                Part::Land => key.k as f32 * 0.03,
                Part::Clouds => 0.015,
            };
        let entity = commands.spawn((Name::new("Backdrop tile"), Sprite::from_image(handle.clone()), Transform::from_xyz(0.0, 0.0, z))).id();
        let skirt = (key.part == Part::Land).then(|| {
            commands
                .spawn((
                    Name::new("Backdrop skirt"),
                    Sprite { image: handle, rect: Some(Rect::new(0.0, h as f32 - 1.0, w as f32, h as f32)), custom_size: Some(Vec2::new(w as f32, SKIRT)), ..default() },
                    Transform::from_xyz(0.0, 0.0, z),
                ))
                .id()
        });
        bd.tiles.insert(key, Tile { entity, skirt, alpha });
    }
}

/// Each tile where its layer puts it for the camera now, as strong as its
/// look's share; the surface's faded out underground, the cave's in.
fn place(bd: Res<Backdrops>, zoom: Res<crate::camera::Zoom>, cam: Single<&Transform, (With<MainCamera>, Without<Sprite>)>, mut sprites: Query<(&mut Transform, &mut Sprite), Without<MainCamera>>) {
    let c = cam.translation.truncate();
    // (Snapped to whole screen pixels from the camera: crisp, and gliding
    // a pixel at a time. Snapped to whole cells it hopped 3 pixels at once.)
    let px = zoom.0.max(1) as f32;
    for (key, tile) in &bd.tiles {
        let at = c + ((bd.centre(*key, c) - c) * px).round() / px;
        let alpha = bd.shown * bd.weights[key.v];
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

// ---- the underground's void ----

/// Behind the back walls underground: a flat dark, tinted by the band and
/// zone the camera is in (eased as they change), under the lighting.
#[derive(Resource)]
struct Void {
    sprite: Entity,
    tint: [f32; 3],
}

fn void_setup(mut commands: Commands) {
    let sprite = commands.spawn((Name::new("Underground void"), Sprite::from_color(Color::BLACK, Vec2::ONE), Transform::from_xyz(0.0, 0.0, Z_GRADIENT + 0.1))).id();
    commands.insert_resource(Void { sprite, tint: VOID_TINTS[1].1 });
}

/// The void's tint by band (the world plan's names), then by zone (which
/// wins where there is one): dark, a touch of colour.
const VOID_TINTS: [(&str, [f32; 3]); 9] = [
    ("underground", [0.235, 0.165, 0.118]),
    ("caverns", [0.172, 0.18, 0.204]),
    ("deep", [0.11, 0.125, 0.19]),
    ("underworld", [0.2, 0.07, 0.05]),
    ("fungal", [0.08, 0.17, 0.16]),
    ("crystal", [0.15, 0.1, 0.22]),
    ("toxic", [0.12, 0.16, 0.07]),
    // (Above the underground band: the dirt's.)
    ("surface", [0.235, 0.165, 0.118]),
    ("peaks", [0.172, 0.18, 0.204]),
];

fn void_tint(sim: &SimWorld, x: i32, y: i32) -> [f32; 3] {
    let key = sim.generator.zone_at(x, y).or_else(|| sim.generator.band_hint(y)).unwrap_or("caverns");
    VOID_TINTS.iter().find(|(k, _)| *k == key).map_or(VOID_TINTS[1].1, |(_, t)| *t)
}

fn void(time: Res<Time>, bd: Res<Backdrops>, sim: Res<SimWorld>, mut v: ResMut<Void>, cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>, mut sprites: Query<(&mut Transform, &mut Sprite), Without<MainCamera>>) {
    let (tf, loader) = *cam;
    let c = tf.translation.truncate();
    let want = void_tint(&sim, c.x as i32, c.y as i32);
    let ease = 1.0 - (-time.delta_secs() / 1.5).exp();
    v.tint = [0, 1, 2].map(|k| v.tint[k] + (want[k] - v.tint[k]) * ease);
    let Ok((mut stf, mut s)) = sprites.get_mut(v.sprite) else { return };
    stf.translation = Vec3::new(c.x, c.y, Z_GRADIENT + 0.1);
    s.custom_size = Some(loader.half_extent * 2.0 + Vec2::splat(8.0));
    s.color = Color::srgba(v.tint[0], v.tint[1], v.tint[2], 1.0 - bd.shown);
}

// ---- the sky's gradient ----

/// The sky darkening upward (behind everything): the sky's colour (the
/// clear colour, by the hour) at the horizon, deeper overhead.
#[derive(Resource)]
struct Gradient(Entity);

fn gradient_setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let h = 64u32;
    // (Top row first: 0.62 overhead .. 1 at the horizon and below.)
    let px: Vec<u8> = (0..h).flat_map(|y| {
        let t = y as f32 / (h - 1) as f32;
        let v = (0.62 + 0.38 * (t * t * (3.0 - 2.0 * t))) * 255.0;
        [v as u8, v as u8, (v * 0.5 + 127.5) as u8, 255]
    }).collect();
    let image = images.add(Image::new(Extent3d { width: 1, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, px, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD));
    let e = commands.spawn((Name::new("Sky gradient"), Sprite::from_image(image), Transform::from_xyz(0.0, 0.0, Z_GRADIENT))).id();
    commands.insert_resource(Gradient(e));
}

/// Over the view, tinted the sky's colour; its horizon at the ground (as
/// far off as the layers), faded out underground.
fn gradient(g: Res<Gradient>, bd: Res<Backdrops>, clear: Res<ClearColor>, cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>, mut sprites: Query<(&mut Transform, &mut Sprite), Without<MainCamera>>) {
    let (tf, loader) = *cam;
    let c = tf.translation.truncate();
    let Ok((mut stf, mut s)) = sprites.get_mut(g.0) else { return };
    let half = loader.half_extent;
    // From the ground (a little above it: the horizon) up to 1.5 views
    // over it, hardly moving as you climb.
    let horizon = bd.ground.unwrap_or(c.y) + 30.0 + (c.y - bd.ground.unwrap_or(c.y)) * 0.995;
    let (bottom, top) = (horizon - half.y * 2.0, horizon + half.y * 1.6);
    stf.translation = Vec3::new(c.x, (bottom + top) / 2.0, Z_GRADIENT);
    s.custom_size = Some(Vec2::new(half.x * 2.0 + 4.0, top - bottom));
    let k = clear.0.to_srgba();
    // (Lifted a little: the horizon a touch paler than the clear colour.)
    s.color = Color::srgba((k.red * 1.12).min(1.0), (k.green * 1.1).min(1.0), (k.blue * 1.04).min(1.0), bd.shown);
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

/// The underground's twinkles: one in `TWINKLE_CHANCE`/1000 cells of
/// `TWINKLE_CELL`² (sparse), moving at this share of the camera's motion.
const TWINKLE_CELL: f32 = 9.0;
const TWINKLE_CHANCE: u64 = 200;
const TWINKLE_PARALLAX: f32 = 0.02;

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
        let cover = bd.cover(c);
        let sky_open = |p: Vec2| open(p) && !cover.covers(p);
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
        // A disc crossing the sky, the sun by day, the moon by night.
        let is_sun = (0.25..0.75).contains(&d.time);
        let phase = if is_sun { (d.time - 0.25) * 2.0 } else { (d.time - 0.75).rem_euclid(1.0) * 2.0 };
        let arc = (std::f32::consts::PI * phase).sin();
        let (dx, dy) = (vw * (0.12 + 0.76 * phase) - drift * 0.3, vh * (0.6 + 0.34 * arc));
        let low = 1.0 - arc;
        let fade = if is_sun { surface } else { night };
        // (Over the image's pixels, finer than cells, from the pixel the
        // disc's centre is in: the pixels aren't square unless the window
        // is 16:9, so each axis steps by its own; a wide glow every other
        // pixel, drawn 2 × 2.)
        let (sx, sy) = (vw / size.x as f32, vh / size.y as f32);
        let (dx, dy) = (((dx / sx).floor() + 0.5) * sx, ((dy / sy).floor() + 0.5) * sy);
        let each = |reach: f32, coarse: bool, f: &dyn Fn(f32, f32, f32) -> Option<([f32; 3], f32)>, put: &mut dyn FnMut(f32, f32, [f32; 3], f32)| {
            let (nx, ny) = ((reach / sx) as i32 + 1, (reach / sy) as i32 + 1);
            let by = if coarse { 2 } else { 1 };
            for oy in (-ny..=ny).step_by(by) {
                for ox in (-nx..=nx).step_by(by) {
                    let (ex, ey) = (ox as f32 * sx, oy as f32 * sy);
                    if (ex * ex + ey * ey).sqrt() > reach + 2.0 * sx.max(sy) {
                        continue;
                    }
                    let (px, py) = (dx + ex, dy + ey);
                    let p = origin + Vec2::new(px, py);
                    if !sky_open(p) {
                        continue;
                    }
                    let dim = fade * (1.0 - 0.8 * cloud(p.x));
                    // (Coarse: the openness and the cloud looked up once for
                    // the 2 × 2, but each pixel its own shade, so the glow
                    // meets what's inside it without a gap or an overlap.)
                    let subs: &[(i32, i32)] = if coarse { &[(0, 0), (1, 0), (0, 1), (1, 1)] } else { &[(0, 0)] };
                    for &(ix, iy) in subs {
                        let (ex, ey) = ((ox + ix) as f32 * sx, (oy + iy) as f32 * sy);
                        let r = (ex * ex + ey * ey).sqrt();
                        if r > reach {
                            continue;
                        }
                        if let Some((c, a)) = f(ex, ey, r) {
                            put(dx + ex, dy + ey, c, a * dim);
                        }
                    }
                }
            }
        };
        if fade > 0.01 {
            if is_sun {
                // The sun: a small white core, a warm glow round it, the sky
                // brightened wide about it; golden when low.
                let warm = [1.0, 0.93 - 0.2 * low, 0.8 - 0.45 * low];
                each(110.0, true, &|_, _, r| (r > 18.0).then(|| (warm, (-(r / 55.0).powi(2)).exp() * 0.3)), &mut put);
                each(18.0, false, &|_, _, r| (r > 5.0).then(|| (warm, (-(r / 9.0).powi(2)).exp() * 0.75 + (-(r / 55.0).powi(2)).exp() * 0.3)), &mut put);
                each(5.0, false, &|_, _, r| (r <= 5.0).then_some(([1.0, 1.0, 0.96 - 0.3 * low], 1.0)), &mut put);
            } else {
                // The moon: big, its seas in two tones, lit from one side, in
                // tonight's phase (the unlit part faintly there), a wide soft
                // halo as bright as it's lit.
                let mp = d.moon_phase();
                let lit_share = 0.5 - 0.5 * (std::f32::consts::TAU * mp).cos();
                let radius = 11.0;
                each(46.0, true, &|_, _, r| (r > radius).then(|| ([0.63, 0.7, 0.95], (-(r / 24.0).powi(2)).exp() * 0.28 * (0.25 + 0.75 * lit_share))), &mut put);
                each(radius, false, &|ex, ey, r| {
                    if r > radius {
                        return None;
                    }
                    let (nx, ny) = (ex / radius, ey / radius);
                    // The terminator: lit east of it waxing, west waning.
                    let k = (std::f32::consts::TAU * mp).cos() * (1.0 - ny * ny).max(0.0).sqrt();
                    let lit = if mp < 0.5 { nx > k } else { nx < -k };
                    if !lit {
                        return Some(([0.23, 0.26, 0.38], 0.9));
                    }
                    let sea = platypus_backdrop::fbm2(nx * 2.4 + 4.0, ny * 2.4, 3, 91);
                    let mut c = if sea > 0.58 { [0.59, 0.62, 0.71] } else if sea > 0.5 { [0.77, 0.79, 0.86] } else { [0.91, 0.93, 0.96] };
                    // (The far limb a shade darker.)
                    if nx + ny > 0.9 {
                        c = c.map(|v| v * 0.8);
                    }
                    Some((c, 1.0))
                }, &mut put);
            }
        }
    }
    // Underground: a few faint twinkles far off in the void (Siofra's,
    // but sparse and very dim, one pixel each), where the cave is open to
    // it, barely drifting with the camera (very far: 2 % of its motion), each fading in
    // and out on its own slow beat; the zone's colour.
    let deep = 1.0 - surface;
    if deep > 0.01 {
        let tint = match sim.generator.zone_at(c.x as i32, c.y as i32) {
            Some("fungal") => [0.55, 1.0, 0.88],
            Some("crystal") => [0.85, 0.7, 1.0],
            Some("toxic") => [0.8, 1.0, 0.55],
            _ if sim.generator.band_hint(c.y as i32) == Some("underworld") => [1.0, 0.6, 0.35],
            _ => [0.7, 0.85, 1.0],
        };
        // (Twinkle space: world = it + the camera × (1 − TWINKLE_PARALLAX).)
        let shift = c * (1.0 - TWINKLE_PARALLAX);
        let (lo, hi) = (origin - shift, origin + Vec2::new(vw, vh) - shift);
        for by in (lo.y / TWINKLE_CELL).floor() as i64..=(hi.y / TWINKLE_CELL).floor() as i64 {
            for bx in (lo.x / TWINKLE_CELL).floor() as i64..=(hi.x / TWINKLE_CELL).floor() as i64 {
                let h = platypus_sim::rng::hash(&[bx as u64, by as u64, 0x7_1111]);
                if h % 1000 >= TWINKLE_CHANCE {
                    continue;
                }
                let r = |k: u64| ((h >> (k * 10)) % 1024) as f32 / 1024.0;
                let at = Vec2::new((bx as f32 + r(1)) * TWINKLE_CELL, (by as f32 + r(2)) * TWINKLE_CELL) + shift;
                if !open(at) {
                    continue;
                }
                // Mostly dark, now and then a slow swell.
                let beat = (t * (0.15 + 0.35 * r(3)) + r(4) * std::f32::consts::TAU).sin();
                // (Far off: dim, and their colour half lost in the dark.)
                let a = (0.2 + 0.8 * (beat * 0.5 + 0.5).powi(2)) * (0.1 + 0.2 * r(5).powi(2)) * deep;
                put(at.x - origin.x, at.y - origin.y, tint.map(|v| v * 0.6 + 0.1), a);
            }
        }
    }
}
