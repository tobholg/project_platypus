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
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};
use platypus_backdrop::peaks::{Look, Looks, albedo, cloud_data, cloud_sea_data, layer_data};
use platypus_sim::{CellPos, Kind};

use crate::camera::MainCamera;
use crate::sound::synth::Rng;
use crate::world::{ChunkLoader, SimWorld};

pub struct BackdropPlugin;

impl Plugin for BackdropPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((Material2dPlugin::<RangeMaterial>::default(), Material2dPlugin::<CloudMaterial>::default(), Material2dPlugin::<SkyMaterial>::default(), Material2dPlugin::<ReflectMaterial>::default()))
            .insert_resource(Backdrops::new())
            .add_systems(Startup, (sky_setup, gradient_setup, void_setup, picture_setup))
            .add_systems(PostUpdate, (tiles, place, gradient, void, sky, picture).chain().after(crate::camera::follow).before(bevy::transform::TransformSystems::Propagate));
    }
}

/// The surface's layers, far to near: their shares of the camera's motion
/// across run from the farthest's to the nearest's (far, so little), up
/// and down far less (the ranges sit still in the view however high or
/// low on the surface you are); their feet step down to the ground by
/// `FEET_STEP` a layer (cells).
const PARALLAX: (f32, f32) = (0.01, 0.04);
const PARALLAX_Y: (f32, f32) = (0.004, 0.015);
const FEET_STEP: f32 = 18.0;
/// The layers (and the far clouds) keep up with the camera only this far
/// over the ground (cells): higher (up in the sky, on the cloud deck),
/// they fall away under you.
const FOLLOW_UP: f32 = 420.0;

/// Layer `k` of `n`: its share of the camera's motion across and up and
/// down, its feet over the ground.
fn layer_at(k: usize, n: usize) -> (f32, f32, f32) {
    let t = k as f32 / (n.max(2) - 1) as f32;
    let lerp = |(a, b): (f32, f32)| a + (b - a) * t;
    (lerp(PARALLAX), lerp(PARALLAX_Y), (n - 1 - k.min(n - 1)) as f32 * FEET_STEP)
}
/// The far clouds: their share of the camera's motion, how fast they
/// drift (cells a second). (How high: their look's `clouds.foot`.)
const CLOUD_PARALLAX: f32 = 0.008;
const CLOUD_PARALLAX_Y: f32 = 0.004;
/// The sea of clouds: its share of the camera's motion across, and how
/// much of the way from where you are to its height it seems to lie
/// (a world with no high air's layers: far, a little); its strip's rows.
const SEA_PARALLAX: f32 = 0.05;
const SEA_NEAR: f32 = 0.08;
/// Where on the view it seems to lie (its heaps' tops over it, its base
/// under it: −1 the view's bottom … 1 its top), so it's in view all
/// through the islands: at its height, `SEA_AT`; climbing, easing down to
/// `SEA_UNDER` by the highest islands (the air thinning: `thin_from`),
/// then sinking out of view (`SEA_GONE`) by the void; going down, easing
/// up to `SEA_OVER` `SEA_LOW` under it, then out of view up `SEA_DEEP`
/// under it.
const SEA_AT: f32 = 0.1;
const SEA_UNDER: f32 = -0.5;
const SEA_OVER: f32 = 0.55;
const SEA_GONE: f32 = 1.6;
const SEA_LOW: f32 = 1_500.0;
const SEA_DEEP: f32 = 3_600.0;
/// The jet stream's cirrus: its share of the camera's motion across, how
/// much of the way from where you are to the jet stream's middle it seems
/// to lie (about you in it, over you under it), its share of the jet's
/// speed, its strip's rows.
const CIRRUS_PARALLAX: f32 = 0.06;
const CIRRUS_NEAR: f32 = 0.3;
const CIRRUS_SPEED: f32 = 0.25;
const CIRRUS_HEIGHT: usize = 300;
/// Over it, its base's solid row stretched this far on down (cells: past
/// the view's bottom, zoomed all the way out).
const SEA_SKIRT: f32 = 1_600.0;
/// Passing through the sea's height, it turns from hanging over you (by
/// its bottom) to lying under you (by its heaps' tops) over this (cells).
const SEA_TURN: f32 = 300.0;
const SEA_HEIGHT: usize = platypus_backdrop::peaks::SEA_ROWS;
const CLOUD_DRIFT: f32 = 1.8;
/// The surface's strips: columns a tile, rows (layers, clouds).
const TILE: usize = 384;
/// The looks' shapes are drawn this fine: a cell a pixel, the ranges 1.5×
/// the size their numbers (`peaks::looks`) say.
const DETAIL: f32 = 1.5;
const HEIGHT: usize = 570;
const CLOUD_HEIGHT: usize = 255;
/// How far below a surface tile its bottom row is stretched (valleys).
const SKIRT: f32 = 450.0;
/// Behind the weather's clouds (-2) and the world's back walls (-1); the
/// sky's gradient behind all.
const Z: f32 = -2.6;
const Z_GRADIENT: f32 = -3.0;
/// Over the light overlay (15), under the glow haze (15.5).
const Z_SKY: f32 = 15.2;
/// The backdrop's picture behind everything (under the void's dark); the
/// water's reflection over the world's cells, under what's in the water.
const Z_PICTURE: f32 = -3.2;
const Z_REFLECT: f32 = 0.05;
/// The render layer the backdrop's drawn on (by its own camera, into a
/// picture: the view shows it; open water mirrors it).
const BACKDROP_LAYER: usize = 29;

fn backdrop_layer() -> bevy::camera::visibility::RenderLayers {
    bevy::camera::visibility::RenderLayers::layer(BACKDROP_LAYER)
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Part {
    /// A look's layer.
    Land,
    Clouds,
    /// The sea of clouds, far off at its height in the sky (`cloud_sea`).
    Sea,
    /// Cirrus streaming along the jet stream, far off at its height.
    Cirrus,
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
    /// A land tile's bottom row stretched down (the sea's, under it).
    skirt: Option<Entity>,
    /// The sea's from under it: drawn upside down (a ceiling of hanging
    /// heaps), its deck stretched on up over it, lit by its own material.
    under: Option<(Entity, Entity, Handle<CloudMaterial>)>,
    /// Its alpha (the stars hide behind it).
    alpha: Vec<u8>,
    /// Its material (lit by the hour).
    material: Lit,
}

/// How a tile's lit: a range's material or a cloud's.
enum Lit {
    Range(Handle<RangeMaterial>),
    Cloud(Handle<CloudMaterial>),
}

/// A range's tile, lit by the hour (`shaders/backdrop.wgsl`): its texels
/// say what each pixel is (`platypus_backdrop::terrain`), the light what
/// colour it is now.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct RangeMaterial {
    #[uniform(0)]
    params: RangeParams,
    #[texture(1)]
    #[sampler(2)]
    texture: Handle<Image>,
}

/// As `backdrop.wgsl`'s `Range` says.
#[derive(Clone, Copy, Default, ShaderType, Debug)]
struct RangeParams {
    to: Vec4,
    color: Vec4,
    ambient: Vec4,
    haze: Vec4,
    rock: Vec4,
    snow: Vec4,
    scrub: Vec4,
    glow: Vec4,
    sun: Vec4,
}

impl RangeParams {
    /// Close enough to `o` that the difference wouldn't show.
    fn near(&self, o: &RangeParams) -> bool {
        let pairs = [(self.to, o.to), (self.color, o.color), (self.ambient, o.ambient), (self.haze, o.haze), (self.rock, o.rock), (self.snow, o.snow), (self.scrub, o.scrub), (self.glow, o.glow)];
        // (The sun's place by the cell; its reach as it is.)
        pairs.iter().all(|(a, b)| (*a - *b).abs().max_element() < 0.003) && (self.sun - o.sun).abs().max_element() < 1.0
    }
}

/// The far clouds, lit by the hour (`shaders/cloud.wgsl`; their texels:
/// `peaks::cloud_data`).
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct CloudMaterial {
    #[uniform(0)]
    params: CloudParams,
    #[texture(1)]
    #[sampler(2)]
    texture: Handle<Image>,
}

/// As `cloud.wgsl`'s `Cloud` says.
#[derive(Clone, Copy, Default, ShaderType, Debug, PartialEq)]
struct CloudParams {
    to: Vec4,
    color: Vec4,
    ambient: Vec4,
    haze: Vec4,
    lit: Vec4,
    shade: Vec4,
    gust: Vec4,
    gust_turn: Vec4,
}

impl CloudParams {
    fn near(&self, o: &CloudParams) -> bool {
        [(self.to, o.to), (self.color, o.color), (self.ambient, o.ambient), (self.haze, o.haze), (self.lit, o.lit), (self.shade, o.shade), (self.gust, o.gust), (self.gust_turn, o.gust_turn)].iter().all(|(a, b)| (*a - *b).abs().max_element() < 0.003)
    }
}

impl Material2d for CloudMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/cloud.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// The sky behind everything (`shaders/sky.wgsl`).
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct SkyMaterial {
    #[uniform(0)]
    params: SkyParams,
}

/// As `sky.wgsl`'s `Sky` says.
#[derive(Clone, Copy, Default, ShaderType, Debug)]
struct SkyParams {
    zenith: Vec4,
    horizon: Vec4,
    glow: Vec4,
    belt: Vec4,
    warm: Vec4,
    sun: Vec4,
    view: Vec4,
}

impl Material2d for SkyMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/sky.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Where the sun (by day) or the moon (by night) is in the view: from its
/// bottom left (cells), the view's half size `half`; and whether it's the
/// sun. Rising on the left, high at noon (or midnight), setting on the
/// right. (The sky draws it there; its glow, the clouds' and the ranges'
/// light know it.)
fn disc_at(time: f32, half: Vec2) -> (Vec2, bool) {
    let is_sun = (0.25..0.75).contains(&time);
    let phase = if is_sun { (time - 0.25) * 2.0 } else { (time - 0.75).rem_euclid(1.0) * 2.0 };
    let arc = (std::f32::consts::PI * phase).sin();
    (Vec2::new(half.x * 2.0 * (0.12 + 0.76 * phase), half.y * 2.0 * (0.6 + 0.34 * arc)), is_sun)
}

impl Material2d for RangeMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/backdrop.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Resource)]
pub struct Backdrops {
    looks: Arc<Vec<Look>>,
    /// Which biome shows which look (the backdrops file's).
    biomes: Arc<Looks>,
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
    /// The sea of clouds' height, if the world has one, and the high air's
    /// layers.
    sea: Option<i32>,
    sky: Option<platypus_worldgen::SkyLayers>,
    /// The view's half size (cells).
    half: Vec2,
    /// A range tile's quad and its skirt's (all the same size); a cloud
    /// tile's.
    meshes: Option<(Handle<Mesh>, Handle<Mesh>)>,
    cloud_mesh: Option<Handle<Mesh>>,
    cirrus_mesh: Option<Handle<Mesh>>,
    /// How far the cirrus has streamed along the jet stream (cells).
    cirrus: f32,
    /// The sea of clouds' tile, upside down, and its skirt (its last row
    /// stretched).
    sea_meshes: Option<(Handle<Mesh>, Handle<Mesh>, Handle<Mesh>)>,
}

impl Backdrops {
    fn new() -> Self {
        // (The backdrops file as it is now; as shipped if it won't read.)
        let all = std::fs::read_to_string(crate::data::data_path("backdrops.ron")).map_err(|e| e.to_string()).and_then(|t| platypus_backdrop::peaks::parse(&t)).unwrap_or_else(|e| {
            warn!("backdrops: {e}; as shipped");
            platypus_backdrop::peaks::shipped()
        });
        let looks = Arc::new(all.looks.clone());
        let weights = looks.iter().map(|v| if v.name == all.default { 1.0 } else { 0.0 }).collect();
        let biomes = Arc::new(all);
        Backdrops { looks, biomes, weights, tiles: HashMap::new(), making: HashMap::new(), ground: None, shown: 1.0, drift: 0.0, sea: None, sky: None, half: Vec2::new(380.0, 230.0), meshes: None, cloud_mesh: None, cirrus_mesh: None, cirrus: 0.0, sea_meshes: None }
    }

    /// A tile's size in cells.
    fn size(part: Part) -> (usize, usize) {
        match part {
            Part::Land => (TILE, HEIGHT),
            Part::Clouds => (TILE, CLOUD_HEIGHT),
            Part::Sea => (TILE, SEA_HEIGHT),
            Part::Cirrus => (TILE, CIRRUS_HEIGHT),
        }
    }

    /// A strip's scroll for the camera at `cam`: world x = strip x + this.
    /// (For land, `k` counts layers of look `v`.)
    fn scroll(&self, part: Part, v: usize, k: usize, cam: Vec2) -> f32 {
        match part {
            Part::Land => cam.x * (1.0 - layer_at(k, self.looks[v].layers.len()).0),
            Part::Clouds => cam.x * (1.0 - CLOUD_PARALLAX) + self.drift,
            Part::Sea => cam.x * (1.0 - SEA_PARALLAX) + self.drift * 0.5,
            Part::Cirrus => cam.x * (1.0 - CIRRUS_PARALLAX) + self.cirrus,
        }
    }

    /// Where a tile is drawn (its centre), for the camera at `cam`.
    fn centre(&self, key: Key, cam: Vec2) -> Vec2 {
        let (w, h) = Self::size(key.part);
        let (w, h) = (w as f32, h as f32);
        let x = (key.tx as f32 + 0.5) * w + self.scroll(key.part, key.v, key.k, cam);
        let ground = self.ground.unwrap_or(cam.y);
        // (How far over the ground they keep up with you.)
        let up = (cam.y - ground).min(FOLLOW_UP);
        let y = match key.part {
            // Its foot (the layer's base row) a little above the ground; it
            // hardly moves as you climb.
            Part::Land => {
                let (_, py, feet) = layer_at(key.k, self.looks[key.v].layers.len());
                let foot = ground + feet - self.looks[key.v].layers[key.k].drop + up * (1.0 - py);
                foot + self.looks[key.v].layers[key.k].base * h - h / 2.0
            }
            Part::Clouds => ground + self.looks[key.v].clouds.foot + up * (1.0 - CLOUD_PARALLAX_Y) + h / 2.0,
            // (Its farthest heaps' tops where the sea seems to be; from under
            // it, upside down, they hang down to there: `under_centre`.)
            Part::Sea => self.sea_top(cam).unwrap_or(cam.y + 1e6) - h * (0.5 - platypus_backdrop::peaks::SEA_CROWN),
            Part::Cirrus => self.cirrus_y(cam).unwrap_or(cam.y + 1e6),
        };
        Vec2::new(x, y)
    }

    /// Where the sea of clouds seems to be for the camera at `cam` (its
    /// heaps' tops over it, its base under it), if the world has one.
    fn sea_top(&self, cam: Vec2) -> Option<f32> {
        let sea = self.sea? as f32;
        let Some(sky) = self.sky else { return Some(cam.y + (sea - cam.y) * SEA_NEAR) };
        let ease = |a: f32, b: f32, t: f32| a + (b - a) * smooth(t.clamp(0.0, 1.0));
        let up = cam.y - sea;
        let (lift, out) = ((sky.thin_from as f32 - sea).max(1.0), (sky.weightless.1 as f32 - sea).max(2.0));
        let at = if up >= 0.0 {
            if up <= lift { ease(SEA_AT, SEA_UNDER, up / lift) } else { ease(SEA_UNDER, -SEA_GONE, (up - lift) / (out - lift).max(1.0)) }
        } else if -up <= SEA_LOW {
            ease(SEA_AT, SEA_OVER, -up / SEA_LOW)
        } else {
            ease(SEA_OVER, SEA_GONE, (-up - SEA_LOW) / (SEA_DEEP - SEA_LOW))
        };
        Some(cam.y + at * self.half.y)
    }

    /// Where the jet stream's cirrus seems to be for the camera at `cam`
    /// (its strip's middle), if the world has a jet stream.
    fn cirrus_y(&self, cam: Vec2) -> Option<f32> {
        let sky = self.sky?;
        let jet = (sky.jet.0 + sky.jet.1) as f32 / 2.0;
        Some(cam.y + (jet - cam.y) * CIRRUS_NEAR)
    }

    /// How much the sea of clouds is seen from over it (1) or from under it
    /// (0: upside down), crossing its height over `SEA_TURN` either side.
    fn over_sea(&self, cam: Vec2) -> f32 {
        self.sea.map_or(1.0, |y| smooth(((cam.y - y as f32 + SEA_TURN) / (2.0 * SEA_TURN)).clamp(0.0, 1.0)))
    }

    /// Where the sea's tile is drawn from under it (its centre: upside
    /// down, its farthest heaps hanging down to where it seems to be).
    fn under_centre(&self, key: Key, cam: Vec2) -> Vec2 {
        let c = self.centre(key, cam);
        Vec2::new(c.x, c.y + SEA_HEIGHT as f32 * (1.0 - 2.0 * platypus_backdrop::peaks::SEA_CROWN))
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
                // (Solid past its last row: a range's skirt, the sea's; from
                // under the sea, upside down, its last row its top.)
                let flip = key.part == Part::Sea && self.over_sea(cam) < 0.5;
                let c = if flip { self.under_centre(*key, cam) } else { self.centre(*key, cam) };
                CoverTile { corner: Vec2::new(c.x - w as f32 / 2.0, c.y + h as f32 / 2.0), w, h, skirted: matches!(key.part, Part::Land | Part::Sea), flip, alpha: tile.alpha.as_slice() }
            })
            .collect();
        Cover { tiles }
    }
}

/// The tiles hiding the sky.
struct Cover<'a> {
    tiles: Vec<CoverTile<'a>>,
}

/// A tile hiding the sky: its top-left corner, size, whether it's solid on
/// past its last row (a skirt), drawn upside down, its alpha.
struct CoverTile<'a> {
    corner: Vec2,
    w: usize,
    h: usize,
    skirted: bool,
    flip: bool,
    alpha: &'a [u8],
}

impl Cover<'_> {
    /// Is a look's layer or cloud in front of this point of the sky?
    fn covers(&self, at: Vec2) -> bool {
        self.tiles.iter().any(|t| {
            let (w, h) = (t.w as i64, t.h as i64);
            let (lx, mut ly) = ((at.x - t.corner.x).floor() as i64, (t.corner.y - at.y).floor() as i64);
            if t.flip {
                ly = h - 1 - ly;
            }
            if lx < 0 || lx >= w || ly < 0 {
                return false;
            }
            (t.skirted && ly >= h) || (ly < h && t.alpha[(ly * w + lx) as usize] > 40)
        })
    }
}

/// The looks' shares: the look of the biome the camera is in, whole (as
/// Terraria: crossing into a biome, its look fades in over a moment, not
/// spread along the ground as you walk).
fn targets(bd: &Backdrops, sim: &SimWorld, x: f32) -> Vec<f32> {
    let name = bd.biomes.for_biome(sim.generator.biome_hint(x as i32).unwrap_or("forest"));
    let i = bd.looks.iter().position(|v| v.name == name).unwrap_or(0);
    (0..bd.looks.len()).map(|v| if v == i { 1.0 } else { 0.0 }).collect()
}

/// Tiles in view made, far ones dropped; the surface's (the looks
/// showing) or the cave's by depth.
#[allow(clippy::too_many_arguments)]
fn tiles(
    mut commands: Commands,
    mut bd: ResMut<Backdrops>,
    sim: Res<SimWorld>,
    time: Res<Time>,
    cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut ranges: ResMut<Assets<RangeMaterial>>,
    mut cloud_mats: ResMut<Assets<CloudMaterial>>,
) {
    let (tf, loader) = *cam;
    let c = tf.translation.truncate();
    let half = loader.half_extent;
    // The ground where the camera is, eased (a cliff isn't a jolt).
    let ground = sim.generator.surface_hint(c.x as i32).map_or(c.y - 90.0, |g| g as f32);
    bd.ground = Some(bd.ground.map_or(ground, |g| g + (ground - g) * 0.05));
    let g = bd.ground.unwrap_or(ground);
    bd.shown = (1.0 - (g - c.y - 60.0) / 120.0).clamp(0.0, 1.0);
    bd.sea = sim.generator.cloud_sea();
    bd.sky = sim.generator.sky_layers();
    bd.half = half;
    bd.drift += time.delta_secs() * CLOUD_DRIFT;
    // (The cirrus streams with the jet stream, gusting as it does.)
    if let Some(sky) = bd.sky {
        bd.cirrus += time.delta_secs() * CIRRUS_SPEED * crate::sky::jet_at(&sim, c.x, (sky.jet.0 + sky.jet.1) as f32 / 2.0);
    }
    // The looks' shares, eased (about a second to change over).
    let target = targets(&bd, &sim, c.x);
    let ease = 1.0 - (-time.delta_secs() / 0.35).exp();
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
        (((s - half.x - 48.0) / w as f32).floor() as i64, ((s + half.x + 48.0) / w as f32).floor() as i64)
    };
    if bd.shown > 0.0 {
        for v in (0..bd.looks.len()).filter(|&v| bd.weights[v] > 0.0) {
            for k in 0..bd.looks[v].layers.len() {
                let (lo, hi) = span(&bd, Part::Land, v, k);
                want.extend((lo..=hi).map(|tx| Key { part: Part::Land, v, k, tx, ty: 0 }));
            }
            let (lo, hi) = span(&bd, Part::Clouds, v, 0);
            want.extend((lo..=hi).map(|tx| Key { part: Part::Clouds, v, k: 0, tx, ty: 0 }));
            // (The sea of clouds only when it's in view: its heaps' tops not
            // under the view, from over it; their hanging ends not over it,
            // from under it.)
            let over = bd.over_sea(c);
            if bd.sea_top(c).is_some_and(|at| (over > 0.0 && at > c.y - half.y - 8.0) || (over < 1.0 && at < c.y + half.y + 8.0)) {
                let (lo, hi) = span(&bd, Part::Sea, v, 0);
                want.extend((lo..=hi).map(|tx| Key { part: Part::Sea, v, k: 0, tx, ty: 0 }));
            }
            // (The jet stream's cirrus, when its strip's in view.)
            if let Some(y) = bd.cirrus_y(c)
                && (y - c.y).abs() < half.y + CIRRUS_HEIGHT as f32 / 2.0
            {
                let (lo, hi) = span(&bd, Part::Cirrus, v, 0);
                want.extend((lo..=hi).map(|tx| Key { part: Part::Cirrus, v, k: 0, tx, ty: 0 }));
            }
        }
    }
    for &key in &want {
        if bd.tiles.contains_key(&key) || bd.making.contains_key(&key) {
            continue;
        }
        let (w, h) = Backdrops::size(key.part);
        let looks = bd.looks.clone();
        let task = match key.part {
            Part::Land => pool.spawn(async move { layer_data(&looks[key.v], key.k, key.tx * w as i64, w, h, DETAIL, seed) }),
            Part::Clouds => pool.spawn(async move { cloud_data(&looks[key.v], key.tx * w as i64, w, h, DETAIL, seed) }),
            Part::Sea => pool.spawn(async move { cloud_sea_data(&looks[key.v], key.tx * w as i64, w, h, DETAIL, seed) }),
            Part::Cirrus => pool.spawn(async move { platypus_backdrop::peaks::cirrus_data(key.tx * w as i64, w, h, seed) }),
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
            if let Some((a, b, _)) = tile.under {
                commands.entity(a).despawn();
                commands.entity(b).despawn();
            }
        }
    }
    // Made: to images, to sprites.
    let ready: Vec<(Key, Vec<u8>)> = bd.making.iter_mut().filter_map(|(key, task)| block_on(poll_once(task)).map(|px| (*key, px))).collect();
    for (key, px) in ready {
        bd.making.remove(&key);
        let (w, h) = Backdrops::size(key.part);
        let alpha: Vec<u8> = px.chunks(4).map(|p| p[3]).collect();
        let clouds = matches!(key.part, Part::Clouds | Part::Sea | Part::Cirrus);
        // The farthest range, the clouds, the nearer ranges.
        let z = Z + key.v as f32 * 0.002
            + match key.part {
                Part::Land => key.k as f32 * 0.03,
                Part::Clouds => 0.015,
                Part::Sea => 0.016,
                Part::Cirrus => 0.017,
            };
        if clouds {
            // Their texels (read exactly; the shader blends them after
            // lighting each, so they drift smoothly).
            let image = Image::new(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, TextureDimension::D2, px, TextureFormat::Rgba8Unorm, RenderAssetUsages::RENDER_WORLD);
            let texture = images.add(image);
            let material = cloud_mats.add(CloudMaterial { params: CloudParams::default(), texture: texture.clone() });
            if key.part != Part::Sea {
                let mesh = if key.part == Part::Clouds { &mut bd.cloud_mesh } else { &mut bd.cirrus_mesh };
                let quad = mesh.get_or_insert_with(|| meshes.add(Rectangle::new(w as f32, h as f32))).clone();
                let entity = commands.spawn((Name::new("Backdrop clouds"), Mesh2d(quad), MeshMaterial2d(material.clone()), Transform::from_xyz(0.0, 0.0, z), backdrop_layer())).id();
                bd.tiles.insert(key, Tile { entity, skirt: None, under: None, alpha, material: Lit::Cloud(material) });
                continue;
            }
            // The sea: from over it, its deck stretched on down under it;
            // from under it, upside down (by its own material, lit from
            // under), its deck stretched on up over it.
            let (quad, flipped, skirt_mesh) = bd.sea_meshes.get_or_insert_with(|| {
                let quad = Mesh::from(Rectangle::new(w as f32, h as f32));
                let mut flipped = quad.clone();
                flipped.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]]);
                let mut skirt = Mesh::from(Rectangle::new(w as f32, SEA_SKIRT));
                let v = (h as f32 - 0.5) / h as f32;
                skirt.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[1.0, v], [0.0, v], [0.0, v], [1.0, v]]);
                (meshes.add(quad), meshes.add(flipped), meshes.add(skirt))
            }).clone();
            let below = cloud_mats.add(CloudMaterial { params: CloudParams::default(), texture });
            let mut part = |name: &'static str, mesh: &Handle<Mesh>, m: &Handle<CloudMaterial>| commands.spawn((Name::new(name), Mesh2d(mesh.clone()), MeshMaterial2d(m.clone()), Transform::from_xyz(0.0, 0.0, z), backdrop_layer())).id();
            let entity = part("Backdrop sea", &quad, &material);
            let skirt = part("Backdrop sea skirt", &skirt_mesh, &material);
            let under = (part("Backdrop sea from under", &flipped, &below), part("Backdrop sea skirt from under", &skirt_mesh, &below), below);
            bd.tiles.insert(key, Tile { entity, skirt: Some(skirt), under: Some(under), alpha, material: Lit::Cloud(material) });
            continue;
        }
        // A range: its texels (not colours: `Rgba8Unorm`, read exactly),
        // lit by its material; its bottom row stretched down below it.
        let image = Image::new(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, TextureDimension::D2, px, TextureFormat::Rgba8Unorm, RenderAssetUsages::RENDER_WORLD);
        let material = ranges.add(RangeMaterial { params: RangeParams::default(), texture: images.add(image) });
        let (quad, skirt_mesh) = bd.meshes.get_or_insert_with(|| {
            let quad = meshes.add(Rectangle::new(w as f32, h as f32));
            let mut skirt = Mesh::from(Rectangle::new(w as f32, SKIRT));
            // (Every row of the skirt: the tile's last.)
            let v = (h as f32 - 0.5) / h as f32;
            skirt.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[1.0, v], [0.0, v], [0.0, v], [1.0, v]]);
            (quad, meshes.add(skirt))
        }).clone();
        let entity = commands.spawn((Name::new("Backdrop range"), Mesh2d(quad), MeshMaterial2d(material.clone()), Transform::from_xyz(0.0, 0.0, z), backdrop_layer())).id();
        let skirt = commands.spawn((Name::new("Backdrop skirt"), Mesh2d(skirt_mesh), MeshMaterial2d(material.clone()), Transform::from_xyz(0.0, 0.0, z), backdrop_layer())).id();
        bd.tiles.insert(key, Tile { entity, skirt: Some(skirt), under: None, alpha, material: Lit::Range(material) });
    }
}

/// Each tile where its layer puts it for the camera now, as strong as its
/// look's share; the surface's faded out underground, the cave's in. The
/// ranges and the clouds lit by the hour: toward the sun by day, the moon
/// by night, fading into the sky's colour (`terrain::hour`), the disc's
/// glow over the ranges near it.
#[allow(clippy::too_many_arguments)]
fn place(
    bd: Res<Backdrops>,
    sim: Res<SimWorld>,
    zoom: Res<crate::camera::Zoom>,
    gloom: Res<crate::light::Gloom>,
    day: Option<Res<crate::light::Daylight>>,
    clear: Res<ClearColor>,
    cam: Single<(&Transform, &ChunkLoader), (With<MainCamera>, Without<Sprite>)>,
    mut places: Query<(&mut Transform, &mut Visibility), Without<MainCamera>>,
    mut ranges: ResMut<Assets<RangeMaterial>>,
    mut cloud_mats: ResMut<Assets<CloudMaterial>>,
) {
    let (tf, loader) = *cam;
    let c = tf.translation.truncate();
    let half = loader.half_extent;
    let time = day.as_ref().map_or(0.5, |d| d.time);
    let sky = clear.0.to_srgba();
    let base = [sky.red, sky.green, sky.blue];
    let light = platypus_backdrop::terrain::hour(time, base);
    let heavens = platypus_backdrop::terrain::sky(time, base);
    let sun = c - half + disc_at(time, half).0;
    let v4 = |rgb: [f32; 3], w: f32| Vec4::new(rgb[0], rgb[1], rgb[2], w);
    // (The ranges snapped to whole screen pixels from the camera: crisp,
    // and gliding a pixel at a time; snapped to whole cells they hopped 2
    // pixels at once. The clouds, drifting slowly on their own, aren't:
    // their shader blends their texels.)
    let px = zoom.0.max(1.0);
    for (key, tile) in &bd.tiles {
        let exact = bd.centre(*key, c);
        let at = if key.part == Part::Land { c + ((exact - c) * px).round() / px } else { exact };
        // (In heavy weather the ranges haze over into the sky, the far ones
        // most; the far clouds thin into the overcast.)
        let haze = gloom.rain * 0.55 + gloom.cover * 0.25;
        let far = match key.part {
            Part::Land => (1.0 - key.k as f32 * 0.18).max(0.3),
            Part::Clouds | Part::Sea | Part::Cirrus => 0.8,
        };
        let alpha = bd.shown * bd.weights[key.v] * (1.0 - haze * far).max(0.0);
        // (A range's skirt under its last row. The sea from over it, its
        // skirt under it; from under it, upside down, its skirt over it;
        // the two crossing over as you pass through its height.)
        let over = if key.part == Part::Sea { bd.over_sea(c) } else { 1.0 };
        let (h, skirt_h) = if key.part == Part::Sea { (SEA_HEIGHT as f32, SEA_SKIRT) } else { (HEIGHT as f32, SKIRT) };
        let mut parts = vec![(tile.entity, at, over > 0.0)];
        parts.extend(tile.skirt.map(|s| (s, Vec2::new(at.x, at.y - h / 2.0 - skirt_h / 2.0), over > 0.0)));
        if let Some((e, s, _)) = &tile.under {
            let u = bd.under_centre(*key, c);
            parts.push((*e, u, over < 1.0));
            parts.push((*s, Vec2::new(u.x, u.y + h / 2.0 + skirt_h / 2.0), over < 1.0));
        }
        for (e, p, shown) in parts {
            if let Ok((mut tf, mut vis)) = places.get_mut(e) {
                tf.translation.x = p.x;
                tf.translation.y = p.y;
                vis.set_if_neq(if shown { Visibility::Inherited } else { Visibility::Hidden });
            }
        }
        let look = &bd.looks[key.v];
        // (A material's re-sent only when its light's moved on enough to
        // see: the hour creeps every tick.)
        match &tile.material {
            Lit::Range(m) => {
                let a = albedo(look, key.k);
                let params = RangeParams {
                    to: v4(light.to, light.rim),
                    color: v4(light.color, alpha),
                    ambient: v4(light.ambient, a.haze),
                    haze: v4(light.haze, a.mist),
                    rock: v4(a.rock, look.layers[key.k].base),
                    snow: v4(a.snow, a.dither),
                    scrub: v4(a.scrub, (key.tx * TILE as i64).rem_euclid(4) as f32),
                    // (Over the ranges mostly when it's low and golden; at
                    // noon it bleached them.)
                    glow: v4(heavens.glow, heavens.glow_strength * (0.2 + 0.8 * light.golden)),
                    sun: Vec4::new(sun.x, sun.y, half.y * 1.2, 0.0),
                };
                if ranges.get(m).is_some_and(|mat| !mat.params.near(&params))
                    && let Some(mut mat) = ranges.get_mut(m)
                {
                    mat.params = params;
                }
            }
            Lit::Cloud(m) => {
                let (lit, shade) = (look.clouds.lit, look.clouds.shade);
                // (The cirrus thinning in the jet stream's lulls, thick in its
                // gusts, where they are along it: the gusts' waves from the
                // tile's middle as drawn.)
                let (gust, gust_turn) = if key.part == Part::Cirrus {
                    let g = crate::sky::gusts(&sim, at.x);
                    (Vec4::new(g[0], g[1], 1.0, 0.0), Vec4::new(g[2], g[3], 0.0, 0.0))
                } else {
                    (Vec4::ZERO, Vec4::ZERO)
                };
                // (Upside down from under the sea: its faces turned over.)
                let params = |alpha: f32, flip: f32| CloudParams {
                    to: v4(light.to, light.golden),
                    color: v4(light.color, alpha),
                    ambient: v4(light.ambient, 0.0),
                    haze: v4(light.haze, 0.0),
                    lit: v4(lit, flip),
                    shade: v4(shade, 0.0),
                    gust,
                    gust_turn,
                };
                let under = tile.under.as_ref().map(|(_, _, m)| (m, params(alpha * (1.0 - over), 1.0)));
                for (m, params) in std::iter::once((m, params(alpha * over, 0.0))).chain(under) {
                    if cloud_mats.get(m).is_some_and(|mat| !mat.params.near(&params))
                        && let Some(mut mat) = cloud_mats.get_mut(m)
                    {
                        mat.params = params;
                    }
                }
            }
        }
    }
}

// ---- the backdrop's picture, and water mirroring it ----

/// The backdrop drawn into a picture by its own camera (seeing only its
/// layer, as the view sees): shown behind everything, and mirrored by open
/// water (`ReflectMaterial`).
#[derive(Resource)]
struct Picture {
    camera: Entity,
    sprite: Entity,
    image: Handle<Image>,
    size: UVec2,
    reflect: Entity,
    material: Handle<ReflectMaterial>,
}

/// The backdrop's camera.
#[derive(Component)]
struct BackdropCamera;

/// Open water mirroring the backdrop (`shaders/reflect.wgsl`).
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct ReflectMaterial {
    #[uniform(0)]
    params: ReflectParams,
    #[texture(1)]
    #[sampler(2)]
    mask: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    backdrop: Handle<Image>,
}

/// As `reflect.wgsl`'s `Reflect` says.
#[derive(Clone, Copy, Default, ShaderType, Debug)]
struct ReflectParams {
    grid: Vec4,
    size: Vec4,
    view: Vec4,
    strength: Vec4,
}

impl Material2d for ReflectMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/reflect.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// How strongly open water mirrors the sky.
const MIRROR: f32 = 0.55;

fn picture_target(images: &mut Assets<Image>, size: UVec2) -> Handle<Image> {
    images.add(Image::new_target_texture(size.x.max(1), size.y.max(1), TextureFormat::Rgba8UnormSrgb, None))
}

fn picture_setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut reflects: ResMut<Assets<ReflectMaterial>>) {
    let size = UVec2::new(16, 16);
    let image = picture_target(&mut images, size);
    let camera = commands
        .spawn((
            Name::new("Backdrop camera"),
            BackdropCamera,
            Camera2d,
            Camera { order: -1, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
            bevy::camera::RenderTarget::Image(image.clone().into()),
            backdrop_layer(),
        ))
        .id();
    let sprite = commands.spawn((Name::new("Backdrop picture"), Sprite { image: image.clone(), ..default() }, Transform::from_xyz(0.0, 0.0, Z_PICTURE))).id();
    let mask = images.add(Image::new_fill(Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, TextureDimension::D2, &[0, 0, 0, 0], TextureFormat::Rgba8Unorm, RenderAssetUsages::RENDER_WORLD));
    let material = reflects.add(ReflectMaterial { params: ReflectParams::default(), mask, backdrop: image.clone() });
    let reflect = commands.spawn((Name::new("Water's reflection"), Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))), MeshMaterial2d(material.clone()), Transform::from_xyz(0.0, 0.0, Z_REFLECT), Visibility::Hidden)).id();
    commands.insert_resource(Picture { camera, sprite, image, size, reflect, material });
}

/// The backdrop's camera sees as the view does (where, how far zoomed, as
/// many pixels); its picture fills the view; the water's reflection over
/// the light grid's open water.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn picture(
    mut commands: Commands,
    mut pic: ResMut<Picture>,
    bd: Res<Backdrops>,
    time: Res<Time>,
    water: Res<crate::light::WaterMask>,
    main: Single<(&Transform, &Projection, &Camera), (With<MainCamera>, Without<BackdropCamera>)>,
    mut cams: Query<(&mut Transform, &mut Projection), (With<BackdropCamera>, Without<MainCamera>)>,
    mut places: Query<(&mut Transform, &mut Visibility), (Without<MainCamera>, Without<BackdropCamera>)>,
    mut sprites: Query<&mut Sprite>,
    mut images: ResMut<Assets<Image>>,
    mut reflects: ResMut<Assets<ReflectMaterial>>,
) {
    let (mtf, mproj, mcam) = *main;
    let Some(px) = mcam.physical_viewport_size() else { return };
    // As many pixels as the view: a new picture if that's changed.
    if px != pic.size {
        let image = picture_target(&mut images, px);
        commands.entity(pic.camera).insert(bevy::camera::RenderTarget::Image(image.clone().into()));
        if let Ok(mut s) = sprites.get_mut(pic.sprite) {
            s.image = image.clone();
        }
        if let Some(mut m) = reflects.get_mut(&pic.material) {
            m.backdrop = image.clone();
        }
        pic.image = image;
        pic.size = px;
    }
    if let Ok((mut tf, mut proj)) = cams.get_mut(pic.camera) {
        *tf = *mtf;
        *proj = mproj.clone();
    }
    let scale = match mproj {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    let view = px.as_vec2() * scale;
    let c = mtf.translation.truncate();
    if let Ok((mut tf, _)) = places.get_mut(pic.sprite) {
        tf.translation = c.extend(Z_PICTURE);
    }
    if let Ok(mut s) = sprites.get_mut(pic.sprite) {
        s.custom_size = Some(view);
    }
    // Open water mirroring it.
    let shown = water.any && bd.shown > 0.01 && water.image.is_some();
    if let Ok((mut tf, mut vis)) = places.get_mut(pic.reflect) {
        *vis = if shown { Visibility::Inherited } else { Visibility::Hidden };
        *tf = Transform::from_translation(water.center.extend(Z_REFLECT)).with_scale(water.extent.extend(1.0));
    }
    if shown
        && let (Some(mask), Some(mut m)) = (water.image.clone(), reflects.get_mut(&pic.material))
    {
        let corner = water.center - water.extent / 2.0;
        m.mask = mask;
        m.params = ReflectParams {
            grid: Vec4::new(corner.x, corner.y, water.extent.x, water.extent.y),
            size: Vec4::new(water.size.x as f32, water.size.y as f32, water.extent.x / water.size.x.max(1) as f32, time.elapsed_secs()),
            view: Vec4::new(c.x - view.x / 2.0, c.y - view.y / 2.0, view.x, view.y),
            strength: Vec4::new(MIRROR * bd.shown, 0.0, 0.0, 0.0),
        };
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

/// The sky behind everything: its colour at the horizon (the clear
/// colour, by the hour) up to a deeper one overhead, in bands; the glow
/// round the sun or moon; low, a warm horizon on the sun's side, a violet
/// belt across the far side (`shaders/sky.wgsl`).
#[derive(Resource)]
struct Gradient {
    entity: Entity,
    material: Handle<SkyMaterial>,
}

fn gradient_setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut skies: ResMut<Assets<SkyMaterial>>) {
    let material = skies.add(SkyMaterial { params: SkyParams::default() });
    let entity = commands.spawn((Name::new("Sky gradient"), Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))), MeshMaterial2d(material.clone()), Transform::from_xyz(0.0, 0.0, Z_GRADIENT), backdrop_layer())).id();
    commands.insert_resource(Gradient { entity, material });
}

/// Over the view; its horizon at the ground (as far off as the layers),
/// faded out underground.
#[allow(clippy::too_many_arguments)]
fn gradient(
    sim: Res<SimWorld>,
    g: Res<Gradient>,
    bd: Res<Backdrops>,
    clear: Res<ClearColor>,
    day: Option<Res<crate::light::Daylight>>,
    cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>,
    mut places: Query<&mut Transform, Without<MainCamera>>,
    mut skies: ResMut<Assets<SkyMaterial>>,
) {
    let (tf, loader) = *cam;
    let c = tf.translation.truncate();
    let half = loader.half_extent;
    // From the ground (a little above it: the horizon) up to 1.5 views
    // over it, hardly moving as you climb.
    let horizon = bd.ground.unwrap_or(c.y) + 45.0 + (c.y - bd.ground.unwrap_or(c.y)) * 0.995;
    let (bottom, top) = (horizon - half.y * 2.0, (horizon + half.y * 1.6).max(c.y + half.y + 2.0));
    if let Ok(mut stf) = places.get_mut(g.entity) {
        stf.translation = Vec3::new(c.x, (bottom + top) / 2.0, Z_GRADIENT);
        stf.scale = Vec3::new(half.x * 2.0 + 4.0, top - bottom, 1.0);
    }
    let time = day.as_ref().map_or(0.5, |d| d.time);
    let k = clear.0.to_srgba();
    let mut heavens = platypus_backdrop::terrain::sky(time, [k.red, k.green, k.blue]);
    // (Into space: black overhead and all round, no glow of air about the
    // sun.)
    let space = crate::sky::space_at(&sim, c.y);
    let black = |rgb: [f32; 3]| [0, 1, 2].map(|i| rgb[i] + (crate::sky::SPACE[i] - rgb[i]) * space);
    heavens.zenith = black(heavens.zenith);
    heavens.horizon = black(heavens.horizon);
    heavens.glow_strength *= 1.0 - space;
    let sun = c - half + disc_at(time, half).0;
    let v4 = |rgb: [f32; 3], w: f32| Vec4::new(rgb[0], rgb[1], rgb[2], w);
    if let Some(mut m) = skies.get_mut(&g.material) {
        m.params = SkyParams {
            zenith: v4(heavens.zenith, bd.shown),
            horizon: v4(heavens.horizon, horizon),
            glow: v4(heavens.glow, heavens.glow_strength),
            belt: v4(heavens.belt, heavens.golden),
            warm: v4(heavens.warm, 0.0),
            sun: Vec4::new(sun.x, sun.y, 0.0, 0.0),
            view: Vec4::new(c.x - half.x, c.y - half.y, half.x, half.y),
        };
    }
}

// ---- the sky, and the cave's own lights ----

/// The sun, the moon and the stars (fixed to the view: they're so far
/// off that nothing the camera does moves them); underground, glowing specks on the walls (fungal and crystal
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

/// Stars are made over this many views' width and those in the first
/// kept, spread over the view (the field as it was when the sky drifted).
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
        .filter(|s: &Star| s.at.x * SKY_WIDE < 1.0)
        .map(|s| Star { at: Vec2::new(s.at.x * SKY_WIDE, s.at.y), ..s })
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
    // (Laid over what's there: the moon over its halo over the stars.)
    let mut put = |x: f32, y: f32, rgb: [f32; 3], a: f32| {
        let (ix, iy) = ((x / vw * size.x as f32).floor() as i64, ((1.0 - y / vh) * size.y as f32).floor() as i64);
        if ix < 0 || iy < 0 || ix >= size.x as i64 || iy >= size.y as i64 || a <= 0.0 {
            return;
        }
        let i = ((iy as u32 * size.x + ix as u32) * 4) as usize;
        let a = a.min(1.0);
        let under = data[i + 3] as f32 / 255.0;
        let out = a + under * (1.0 - a);
        for (k, v) in rgb.iter().enumerate() {
            let was = data[i + k] as f32 / 255.0;
            data[i + k] = ((v * a + was * under * (1.0 - a)) / out.max(1e-4) * 255.0).round().clamp(0.0, 255.0) as u8;
        }
        // (A still 4 × 4 ordered dither on the alpha: a faint glow's few
        // levels would show as rings.)
        const BAYER: [f32; 16] = [0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0];
        let d = BAYER[((iy & 3) * 4 + (ix & 3)) as usize] / 16.0;
        data[i + 3] = (out * 255.0 + d).clamp(0.0, 255.0) as u8;
    };
    let surface = bd.shown;
    if surface > 0.01
        && let Some(d) = day.as_ref()
    {
        // Night: how much (0 by day), from the sun's height. (In space
        // the stars are out by day too.)
        let sun_h = -(d.time * std::f32::consts::TAU).cos();
        let night = ((0.05 - sun_h) / 0.3).clamp(0.0, 1.0).max(crate::sky::space_at(&sim, c.y)) * surface;
        // (The weather's clouds hide the sky by column; up in space
        // they're far below, hiding nothing.)
        let over = 1.0 - crate::sky::space_at(&sim, c.y);
        let cloud = |x: f32| world.weather().map_or(0.0, |w| ((w.overcast(x as i32 - 4, x as i32 + 4) - 0.2) / 0.4).clamp(0.0, 1.0)) * over;
        let cover = bd.cover(c);
        let sky_open = |p: Vec2| open(p) && !cover.covers(p);
        // A disc crossing the sky, the sun by day, the moon by night.
        let (at, is_sun) = disc_at(d.time, half);
        let (dx, dy) = (at.x, at.y);
        let phase = if is_sun { (d.time - 0.25) * 2.0 } else { (d.time - 0.75).rem_euclid(1.0) * 2.0 };
        let arc = (std::f32::consts::PI * phase).sin();
        // (An image pixel's size on each axis: not square unless the window
        // is 16:9.)
        let (pw, ph) = (vw / size.x as f32, vh / size.y as f32);
        let low = 1.0 - arc;
        // The moon's disc (stars behind it hidden) and how lit it is.
        let mp = d.moon_phase();
        let lit_share = 0.5 - 0.5 * (std::f32::consts::TAU * mp).cos();
        let radius = 16.5;
        if night > 0.01 {
            for s in &sky.stars {
                let (sx, sy) = (s.at.x * vw, vh - s.at.y * vh * 0.9);
                // (Behind the moon: hidden; near it, washed out by its
                // light, the more the fuller.)
                let r = Vec2::new(sx - dx, sy - dy).length();
                if !is_sun && r <= radius {
                    continue;
                }
                let washed = if is_sun { 1.0 } else { 1.0 - 0.8 * lit_share * (-(r / 30.0).powi(2)).exp() };
                let p = origin + Vec2::new(sx, sy);
                if !sky_open(p) {
                    continue;
                }
                let twinkle = 0.65 + 0.35 * (t * s.rate + s.phase).sin() * (t * s.rate * 0.37 + s.phase * 2.0).sin();
                put(sx, sy, s.color, s.bright * twinkle * washed * night * (1.0 - cloud(p.x)));
            }
        }
        let fade = if is_sun { surface } else { night };
        // Over the image's pixels, each shaded by where its centre is from
        // the disc's (not snapped to a pixel: it glides, its edges
        // anti-aliased); a wide glow in blocks of 2 × 2 (on even pixels),
        // the openness and the cloud looked up once a block but each pixel
        // its own shade.
        let (ci, cj) = ((dx / pw).floor() as i32, (dy / ph).floor() as i32);
        let each = |reach: f32, coarse: bool, f: &dyn Fn(f32, f32, f32) -> Option<([f32; 3], f32)>, put: &mut dyn FnMut(f32, f32, [f32; 3], f32)| {
            let (nx, ny) = ((reach / pw) as i32 + 2, (reach / ph) as i32 + 2);
            let by = if coarse { 2 } else { 1 };
            let even = |v: i32| if coarse { v.div_euclid(2) * 2 } else { v };
            for j in (even(cj - ny)..=cj + ny).step_by(by) {
                for i in (even(ci - nx)..=ci + nx).step_by(by) {
                    let (x, y) = ((i as f32 + 0.5) * pw, (j as f32 + 0.5) * ph);
                    if Vec2::new(x - dx, y - dy).length() > reach + 2.0 * pw.max(ph) {
                        continue;
                    }
                    let p = origin + Vec2::new(x, y);
                    if !sky_open(p) {
                        continue;
                    }
                    let dim = fade * (1.0 - 0.8 * cloud(p.x));
                    let subs: &[(i32, i32)] = if coarse { &[(0, 0), (1, 0), (0, 1), (1, 1)] } else { &[(0, 0)] };
                    for &(si, sj) in subs {
                        let (x, y) = ((i + si) as f32 * pw + 0.5 * pw, (j + sj) as f32 * ph + 0.5 * ph);
                        let (ex, ey) = (x - dx, y - dy);
                        let r = (ex * ex + ey * ey).sqrt();
                        if r > reach {
                            continue;
                        }
                        if let Some((c, a)) = f(ex, ey, r) {
                            put(x, y, c, a * dim);
                        }
                    }
                }
            }
        };
        // (How much of a pixel lies inside an edge `d` cells in from it.)
        let pix = pw.max(ph);
        let inside = |d: f32| (d / pix + 0.5).clamp(0.0, 1.0);
        let smooth = |e0: f32, e1: f32, v: f32| {
            let t = ((v - e0) / (e1 - e0)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        let mix = |a: [f32; 3], b: [f32; 3], t: f32| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
        if fade > 0.01 {
            if is_sun {
                // The sun: a small white core, a warm glow round it, the sky
                // brightened wide about it; golden when low.
                let warm = [1.0, 0.93 - 0.2 * low, 0.8 - 0.45 * low];
                let core = 7.5;
                // (In space, no air about it to glow: only the core and a
                // tight halo.)
                let air = 1.0 - crate::sky::space_at(&sim, c.y);
                each(165.0, true, &|_, _, r| Some((warm, (-(r / 83.0).powi(2)).exp() * 0.3 * air)), &mut put);
                each(33.0, false, &|_, _, r| Some((warm, (-(r / 13.5).powi(2)).exp() * 0.9)), &mut put);
                each(core + pix, false, &|_, _, r| Some(([1.0, 1.0, 0.96 - 0.3 * low], inside(core - r))), &mut put);
            } else {
                // The moon: big, its seas in two tones, lit from one side, in
                // tonight's phase (the unlit part faintly there), a soft halo
                // as bright as it's lit.
                each(51.0, true, &|_, _, r| Some(([0.63, 0.7, 0.95], (-(r / 25.5).powi(2)).exp() * 0.14 * (0.25 + 0.75 * lit_share))), &mut put);
                each(radius + pix, false, &|ex, ey, r| {
                    let edge = inside(radius - r);
                    if edge <= 0.0 {
                        return None;
                    }
                    let (nx, ny) = (ex / radius, ey / radius);
                    // The terminator: lit east of it waxing, west waning
                    // (how far past it, in cells, for its soft edge).
                    let k = (std::f32::consts::TAU * mp).cos() * (1.0 - ny * ny).max(0.0).sqrt();
                    let past = if mp < 0.5 { nx - k } else { -k - nx } * radius;
                    let lit = inside(past);
                    // (Its tones blended over a sliver: they drift as it
                    // does, not hop a pixel at a time.)
                    let sea = platypus_backdrop::fbm2(nx * 2.4 + 4.0, ny * 2.4, 3, 91);
                    let mut c = mix(mix([0.91, 0.93, 0.96], [0.77, 0.79, 0.86], smooth(0.49, 0.51, sea)), [0.59, 0.62, 0.71], smooth(0.57, 0.59, sea));
                    // (The far limb a shade darker.)
                    c = c.map(|v| v * (1.0 - 0.2 * smooth(0.85, 0.95, nx + ny)));
                    Some((mix([0.23, 0.26, 0.38], c, lit), (0.9 + 0.1 * lit) * edge))
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
