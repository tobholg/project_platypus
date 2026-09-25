//! The sky: clouds from the sim's weather field, drawn on the cell grid, and
//! a sky colour that greys as it clouds over. Rain and snow are particles
//! (`particles.rs`).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use platypus_sim::rng::hash;
use platypus_sim::Weather;
use platypus_sim::weather::CLOUD_AT;

use crate::camera::MainCamera;
use crate::render::SKY_COLOR;
use crate::world::{ChunkLoader, SimWorld};

pub struct SkyPlugin;

/// Behind the background layer (trees, walls): clouds are far away.
const Z_CLOUDS: f32 = -2.0;
/// Overcast sky colour.
const STORM_SKY: Color = Color::srgb(0.46, 0.52, 0.6);

#[derive(Resource)]
struct Clouds {
    entity: Entity,
    image: Handle<Image>,
    size: UVec2,
    /// What the texture holds: its left edge (cells), the air offset and the
    /// tick it was drawn at. Between redraws the sprite slides with the air.
    drawn: Option<(i32, f32, u64)>,
    puffs: Vec<f32>,
}

/// Redraw at most this often (ticks): clouds change slowly, and drift is a
/// slide of the whole texture.
const REDRAW_EVERY: u64 = 15;
/// Cells drawn beyond the view each side, so the slide has room.
const MARGIN: i32 = 96;
/// The puff table tiles every this many cells.
const PUFF_TILE: usize = 256;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(PostUpdate, (draw_clouds, tint_sky));
    }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = images.add(blank(UVec2::ONE));
    let entity = commands.spawn((Name::new("Clouds"), Sprite::from_image(image.clone()), Transform::from_xyz(0.0, 0.0, Z_CLOUDS))).id();
    commands.insert_resource(Clouds { entity, image, size: UVec2::ONE, drawn: None, puffs: puff_table() });
}

fn blank(size: UVec2) -> Image {
    Image::new_fill(
        Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

/// Fine puffs for cloud edges: tiling value noise (lattice wraps at the tile),
/// two octaves, precomputed once. Render only.
fn puff_table() -> Vec<f32> {
    let n = PUFF_TILE;
    let octave = |x: usize, y: usize, cell: usize, salt: u64| {
        let period = (n / cell) as u64;
        let v = |i: u64, j: u64| (hash(&[salt, i % period, j % period]) >> 40) as f32 / (1u64 << 24) as f32;
        let (i, j) = ((x / cell) as u64, (y / cell) as u64);
        let (fx, fy) = ((x % cell) as f32 / cell as f32, (y % cell) as f32 / cell as f32);
        let s = |t: f32| t * t * (3.0 - 2.0 * t);
        let (sx, sy) = (s(fx), s(fy));
        let a = v(i, j) + (v(i + 1, j) - v(i, j)) * sx;
        let b = v(i, j + 1) + (v(i + 1, j + 1) - v(i, j + 1)) * sx;
        a + (b - a) * sy
    };
    let mut t = vec![0.0; n * n];
    for y in 0..n {
        for x in 0..n {
            t[y * n + x] = 0.6 * octave(x, y, 32, 0xC1) + 0.4 * octave(x, y, 8, 0xC2);
        }
    }
    t
}

#[inline]
fn puff(table: &[f32], x: i32, y: i32) -> f32 {
    let m = PUFF_TILE as i32;
    table[(y.rem_euclid(m) * m + x.rem_euclid(m)) as usize]
}

fn draw_clouds(
    sim: Res<SimWorld>,
    mut clouds: ResMut<Clouds>,
    mut images: ResMut<Assets<Image>>,
    cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>,
    mut sprites: Query<(&mut Sprite, &mut Transform), Without<MainCamera>>,
) {
    let Some(weather) = sim.world.weather() else { return };
    let (tf, loader) = *cam;
    let tick = sim.world.tick();
    let clouds = &mut *clouds;
    let Ok((mut sprite, mut stf)) = sprites.get_mut(clouds.entity) else { return };
    let (y0, y1) = (weather.y0, weather.y1());
    let half = loader.half_extent;
    let (view0, view1) = ((tf.translation.x - half.x).floor() as i32, (tf.translation.x + half.x).ceil() as i32);
    if (tf.translation.y + half.y) < y0 as f32 || (tf.translation.y - half.y) > y1 as f32 {
        sprite.color = Color::NONE;
        return;
    }
    sprite.color = Color::WHITE;

    // How far the air moved since the texture was drawn (wrapping).
    let width = weather.width() as f32;
    let slid = |from: f32| {
        let d = (weather.offset() - from).rem_euclid(width);
        (if d > width / 2.0 { d - width } else { d }).round() as i32
    };
    let fresh = match clouds.drawn {
        Some((x0, off, at)) => {
            let s = slid(off);
            tick < at + REDRAW_EVERY && view0 >= x0 + s && view1 <= x0 + s + clouds.size.x as i32
        }
        None => false,
    };
    if !fresh {
        let (x0, x1) = (view0 - MARGIN, view1 + MARGIN);
        let size = UVec2::new((x1 - x0) as u32, (y1 - y0) as u32);
        if size != clouds.size {
            clouds.image = images.add(blank(size));
            clouds.size = size;
            sprite.image = clouds.image.clone();
        }
        if let Some(mut image) = images.get_mut(&clouds.image)
            && let Some(data) = image.data.as_mut()
        {
            paint(data, size, x0, x1, weather, &clouds.puffs);
        }
        clouds.drawn = Some((x0, weather.offset(), tick));
    }
    let (x0, off, _) = clouds.drawn.expect("drawn above");
    let size = clouds.size;
    let left = (x0 + slid(off)) as f32;
    stf.translation = Vec3::new(left + size.x as f32 / 2.0, y0 as f32 + size.y as f32 / 2.0, Z_CLOUDS);
    sprite.custom_size = Some(size.as_vec2());
}

/// Paint the whole band height over columns `x0..x1`.
fn paint(data: &mut [u8], size: UVec2, x0: i32, x1: i32, weather: &Weather, puffs: &[f32]) {
    let (y0, y1) = (weather.y0, weather.y1());
    let offset = weather.offset().round() as i32;
    // Billows: the fine pattern bends the field up and down, so tops heap and
    // bellies sag instead of following the coarse field's smooth edge.
    let inside = |x: i32, y: i32| {
        let (cx, cy) = (x - offset, y);
        let billow = 1.0 - (2.0 * puff(puffs, cx, cy) - 1.0).abs();
        let sway = puff(puffs, cx * 7 / 10 + 311, cy * 7 / 10);
        let (wx, wy) = ((sway - 0.5) * 22.0, (billow - 0.5) * 20.0);
        weather.moisture_at(x as f32 + 0.5 + wx, y as f32 + 0.5 + wy) >= CLOUD_AT
    };
    let height = (y1 - y0) as usize;
    let mut column = vec![false; height];
    for x in x0..x1 {
        let rain = weather.rain_at(x).min(0.15) / 0.15;
        for (k, v) in column.iter_mut().enumerate() {
            *v = inside(x, y0 + k as i32);
        }
        // Each run of cloud in this column: a bright rim on top, a light body,
        // a shadowed belly (a quarter of its depth), dithered where they meet.
        let mut k = height;
        while k > 0 {
            k -= 1;
            let px = |j: usize| ((height - 1 - j) * size.x as usize + (x - x0) as usize) * 4;
            if !column[k] {
                let i = px(k);
                data[i..i + 4].copy_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            let top = k;
            let mut bottom = k;
            while bottom > 0 && column[bottom - 1] {
                bottom -= 1;
            }
            let depth = (top - bottom + 1) as f32;
            for j in bottom..=top {
                let y = y0 + j as i32;
                let from_top = (top - j) as f32;
                let from_bottom = (j - bottom) as f32;
                let belly = (depth * 0.28).clamp(2.0, 14.0);
                let checker = (x + y).rem_euclid(2) == 0;
                let tone: [f32; 3] = if from_top < 1.5 {
                    [255.0, 255.0, 255.0]
                } else if from_bottom < belly - 1.0 || (from_bottom < belly + 1.0 && checker) {
                    [168.0, 178.0, 196.0]
                } else if from_top < 5.0 || (from_top < 7.0 && checker) {
                    [240.0, 243.0, 250.0]
                } else {
                    [214.0, 220.0, 232.0]
                };
                let dim = 1.0 - 0.38 * rain;
                let i = px(j);
                data[i..i + 4].copy_from_slice(&[(tone[0] * dim) as u8, (tone[1] * dim) as u8, (tone[2] * dim) as u8, 245]);
            }
            k = bottom;
        }
    }
}

/// The sky greys over as clouds build overhead.
fn tint_sky(sim: Res<SimWorld>, cam: Single<(&Transform, &ChunkLoader), With<MainCamera>>, mut clear: ResMut<ClearColor>) {
    let Some(weather) = sim.world.weather() else { return };
    let (tf, loader) = *cam;
    let x = tf.translation.x as i32;
    let w = loader.half_extent.x as i32;
    let overcast = weather.overcast(x - w, x + w);
    let k = ((overcast - 0.15) / 0.45).clamp(0.0, 1.0);
    clear.0 = SKY_COLOR.mix(&STORM_SKY, k);
}
