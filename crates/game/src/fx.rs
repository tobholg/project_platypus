//! Screen-space feedback that isn't simulated: camera shake and blast
//! flashes. Anything that explodes sends an `Explosion` message; bombs send
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
            .init_resource::<Trauma>()
            .init_resource::<ShakeOffset>()
            .add_systems(Startup, setup)
            .add_systems(Update, (on_explosion, shake, fade_flashes).chain());
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
