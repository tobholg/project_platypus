//! Screen-space feedback that isn't simulated: camera shake, blast
//! flashes, lightning bolts and the sky flash that goes with them. Anything that explodes sends an `Explosion` message; bombs send
//! their own, and the sim's detonations (gas pockets, chains) are forwarded
//! from `StepStats::detonated`.

use bevy::prelude::*;

use crate::camera::MainCamera;

pub struct FxPlugin;

/// Something blew up at `at` (cells) with this blast radius.
#[derive(Message, Clone, Copy, Debug)]
pub struct Explosion {
    pub at: Vec2,
    pub radius: f32,
}

/// Lightning struck (from the sim's `StepStats::lightning`).
#[derive(Message, Clone, Copy, Debug)]
pub struct Lightning(pub platypus_sim::Strike);

/// 0..1, how bright the sky flashes right now (lightning).
#[derive(Resource, Default)]
pub struct SkyFlash(pub f32);

#[derive(Component)]
struct Bolt {
    age: f32,
}

/// Seconds a bolt stays on screen (it flickers).
const BOLT_LIFE: f32 = 0.35;

/// 0..1. Shake amplitude grows with its square, so small bumps stay subtle.
#[derive(Resource, Default)]
pub struct Trauma(pub f32);

/// Offset the camera by this (cells) after following; set by `shake`.
#[derive(Resource, Default)]
pub struct ShakeOffset(pub Vec2);

#[derive(Component)]
struct Flash {
    age: f32,
    radius: f32,
}

#[derive(Resource)]
struct FlashAssets {
    mesh: Handle<Mesh>,
}

/// Seconds a flash lasts.
const FLASH_LIFE: f32 = 0.2;
/// Trauma lost per second.
const TRAUMA_DECAY: f32 = 1.4;
/// Maximum shake, in screen pixels.
const MAX_SHAKE_PX: f32 = 18.0;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Explosion>()
            .add_message::<Lightning>()
            .init_resource::<SkyFlash>()
            .init_resource::<Trauma>()
            .init_resource::<ShakeOffset>()
            .add_systems(Startup, setup)
            .add_systems(Update, (on_explosion, on_lightning, shake, fade_flashes, fade_bolts).chain());
    }
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(FlashAssets { mesh: meshes.add(Circle::new(1.0)) });
}

fn on_explosion(
    mut commands: Commands,
    mut blasts: MessageReader<Explosion>,
    mut trauma: ResMut<Trauma>,
    assets: Res<FlashAssets>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    cam: Single<&Transform, With<MainCamera>>,
) {
    for e in blasts.read() {
        // Closer and bigger shakes harder; far-off blasts are a rumble.
        let dist = cam.translation.truncate().distance(e.at);
        let near = (1.0 - dist / (e.radius * 14.0)).clamp(0.0, 1.0);
        trauma.0 = (trauma.0 + near * (e.radius / 26.0).min(1.5) * 0.75).min(1.0);
        commands.spawn((
            Flash { age: 0.0, radius: e.radius },
            Mesh2d(assets.mesh.clone()),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgba(1.0, 0.92, 0.7, 0.9)))),
            Transform::from_translation(e.at.extend(8.0)).with_scale(Vec3::splat(e.radius * 0.6)),
        ));
    }
}

fn shake(time: Res<Time>, mut trauma: ResMut<Trauma>, mut offset: ResMut<ShakeOffset>, zoom: Res<crate::camera::Zoom>) {
    let t = time.elapsed_secs();
    let amp = trauma.0 * trauma.0 * MAX_SHAKE_PX / zoom.0 as f32;
    // Sums of incommensurate sines: noisy enough, and smooth frame to frame.
    let n = |a: f32, b: f32| ((t * a).sin() + 0.6 * (t * b).sin()) / 1.6;
    offset.0 = Vec2::new(n(47.0, 71.0), n(53.0, 83.0)) * amp;
    trauma.0 = (trauma.0 - TRAUMA_DECAY * time.delta_secs()).max(0.0);
}

fn fade_flashes(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut q: Query<(Entity, &mut Flash, &mut Transform, &MeshMaterial2d<ColorMaterial>)>,
) {
    for (entity, mut flash, mut tf, mat) in &mut q {
        flash.age += time.delta_secs();
        let f = flash.age / FLASH_LIFE;
        if f >= 1.0 {
            materials.remove(&mat.0);
            commands.entity(entity).despawn();
            continue;
        }
        tf.scale = Vec3::splat(flash.radius * (0.6 + 0.9 * f.sqrt()));
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.color = Color::srgba(1.0, 0.92 - 0.4 * f, 0.7 - 0.5 * f, 0.9 * (1.0 - f) * (1.0 - f));
        }
    }
}

fn on_lightning(
    mut commands: Commands,
    mut strikes: MessageReader<Lightning>,
    mut images: ResMut<Assets<Image>>,
    mut trauma: ResMut<Trauma>,
    mut flash: ResMut<SkyFlash>,
    cam: Single<&Transform, With<MainCamera>>,
) {
    for Lightning(s) in strikes.read() {
        let near = (1.0 - cam.translation.truncate().distance(Vec2::new(s.x as f32, s.hit.y as f32)) / 900.0).clamp(0.0, 1.0);
        trauma.0 = (trauma.0 + 0.35 * near).min(1.0);
        flash.0 = flash.0.max(0.4 + 0.6 * near);
        let (image, x0, y0) = bolt_image(s);
        let size = Vec2::new(image.width() as f32, image.height() as f32);
        commands.spawn((
            Bolt { age: 0.0 },
            Sprite { image: images.add(image), custom_size: Some(size), ..default() },
            Transform::from_xyz(x0 as f32 + size.x / 2.0, y0 as f32 + size.y / 2.0, 6.0),
        ));
    }
}

/// A jagged bolt from the cloud to what it hit, with a few forks, one cell
/// wide with a glow either side. Returns the image and its bottom-left cell.
fn bolt_image(s: &platypus_sim::Strike) -> (Image, i32, i32) {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    use platypus_sim::rng::Rng;
    let mut rng = Rng::seeded(&[s.x as u64, s.top as u64, s.hit.y as u64]);
    let height = (s.top - s.hit.y).max(1);
    let mut points: Vec<(i32, i32)> = Vec::new();
    let walk = |rng: &mut Rng, from: (i32, i32), steps: i32, pull: Option<i32>, out: &mut Vec<(i32, i32)>| {
        let (mut x, mut y) = from;
        for _ in 0..steps {
            y -= 1;
            let r = (rng.next_u8() % 5) as i32 - 2;
            let toward = pull.map_or(0, |t| (t - x).signum());
            // Jag: mostly straight down, now and then a kink.
            if rng.next_u8() < 90 {
                x += r.signum() + if rng.next_u8() < 60 { toward } else { 0 };
            }
            out.push((x, y));
        }
    };
    walk(&mut rng, (s.x, s.top), height, Some(s.hit.x), &mut points);
    // Pull the last stretch onto the hit cell.
    let main = points.clone();
    for (i, &(px, py)) in main.iter().enumerate().skip(main.len().saturating_sub(12)) {
        let k = (i + 12 - main.len()) as f32 / 12.0;
        points[i] = ((px as f32 + (s.hit.x - px) as f32 * k).round() as i32, py);
    }
    for _ in 0..3 {
        let at = main[(rng.next_u32() as usize) % main.len().max(1)];
        let len = (height / 6).max(4) + (rng.next_u8() as i32 % 20);
        walk(&mut rng, at, len, None, &mut points);
    }
    let (x0, x1) = points.iter().fold((i32::MAX, i32::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
    let (x0, x1) = (x0 - 2, x1 + 2);
    let (y0, y1) = (s.hit.y, s.top + 1);
    let (w, h) = ((x1 - x0 + 1) as u32, (y1 - y0 + 1) as u32);
    let mut image = Image::new_fill(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let data = image.data.as_mut().expect("fresh image");
    let mut put = |x: i32, y: i32, c: [u8; 4]| {
        if x < x0 || x > x1 || y < y0 || y > y1 {
            return;
        }
        let i = (((y1 - y) as u32 * w + (x - x0) as u32) * 4) as usize;
        if data[i + 3] < c[3] {
            data[i..i + 4].copy_from_slice(&c);
        }
    };
    for &(x, y) in &points {
        put(x - 1, y, [150, 170, 255, 120]);
        put(x + 1, y, [150, 170, 255, 120]);
    }
    for &(x, y) in &points {
        put(x, y, [250, 250, 255, 255]);
    }
    (image, x0, y0)
}

fn fade_bolts(mut commands: Commands, time: Res<Time>, mut flash: ResMut<SkyFlash>, mut q: Query<(Entity, &mut Bolt, &mut Sprite)>) {
    flash.0 = (flash.0 - time.delta_secs() * 3.0).max(0.0);
    for (entity, mut bolt, mut sprite) in &mut q {
        bolt.age += time.delta_secs();
        if bolt.age >= BOLT_LIFE {
            commands.entity(entity).despawn();
            continue;
        }
        // Flicker: on, dim, on, fading.
        let f = bolt.age / BOLT_LIFE;
        let on = if f < 0.2 || (0.35..0.55).contains(&f) { 1.0 } else { 0.35 };
        sprite.color = Color::srgba(1.0, 1.0, 1.0, on * (1.0 - f * f));
    }
}
