//! Camera: follows a `CameraTarget` if there is one, otherwise flies freely
//! (WASD / arrows, Shift for speed). Tab detaches it from the player.
//! Mouse wheel zooms in whole screen-pixels per cell.

use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::world::ChunkLoader;

pub struct CameraPlugin {
    pub start: Vec2,
}

#[derive(Component)]
pub struct MainCamera;

/// The camera follows the first entity with this (the local player).
#[derive(Component)]
pub struct CameraTarget;

/// Where the mouse points, in world space (cells). `None` off-window.
#[derive(Resource, Default)]
pub struct CursorWorld(pub Option<Vec2>);

/// Scripts (scenarios, tests) can point the "mouse" here instead.
#[derive(Resource, Default)]
pub struct CursorOverride(pub Option<Vec2>);

/// Free camera: detached from the player, which then ignores the keyboard.
#[derive(Resource, Default)]
pub struct FreeCamera(pub bool);

/// Screen pixels per cell. Integer so cells stay crisp.
#[derive(Resource)]
pub struct Zoom(pub u32);

const ZOOM_LEVELS: [u32; 6] = [1, 2, 3, 4, 6, 8];
const FLY_SPEED_PX: f32 = 900.0;

#[derive(Resource)]
struct StartAt(Vec2);

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        // PLATYPUS_ZOOM=6 starts closer in (screenshots, detail work).
        let start_zoom = std::env::var("PLATYPUS_ZOOM").ok().and_then(|z| z.parse().ok()).filter(|z| ZOOM_LEVELS.contains(z)).unwrap_or(3);
        app.insert_resource(Zoom(start_zoom))
            .init_resource::<CursorWorld>()
            .init_resource::<CursorOverride>()
            .init_resource::<FreeCamera>()
            .insert_resource(StartAt(self.start))
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, (toggle_free, zoom, fly, apply_zoom).chain())
            .add_systems(PreUpdate, track_cursor)
            .add_systems(PostUpdate, follow.before(TransformSystems::Propagate));
    }
}

fn spawn_camera(mut commands: Commands, start: Res<StartAt>) {
    commands.spawn((
        Camera2d,
        MainCamera,
        Transform::from_translation(start.0.extend(100.0)),
        ChunkLoader { half_extent: Vec2::new(400.0, 240.0) },
    ));
}

fn zoom(scroll: Res<AccumulatedMouseScroll>, keys: Res<ButtonInput<KeyCode>>, mut zoom: ResMut<Zoom>) {
    // Ctrl+wheel is reserved for tools (brush size).
    if scroll.delta.y == 0.0 || keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        return;
    }
    let i = ZOOM_LEVELS.iter().position(|&z| z == zoom.0).unwrap_or(2) as i32;
    let j = (i + scroll.delta.y.signum() as i32).clamp(0, ZOOM_LEVELS.len() as i32 - 1);
    zoom.0 = ZOOM_LEVELS[j as usize];
}

fn apply_zoom(
    zoom: Res<Zoom>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut cam: Single<(&mut Projection, &mut ChunkLoader), With<MainCamera>>,
) {
    let (projection, loader) = &mut *cam;
    if let Projection::Orthographic(o) = &mut **projection {
        o.scale = 1.0 / zoom.0 as f32;
    }
    loader.half_extent = Vec2::new(window.width(), window.height()) / (2.0 * zoom.0 as f32);
}

fn toggle_free(keys: Res<ButtonInput<KeyCode>>, mut free: ResMut<FreeCamera>) {
    if keys.just_pressed(KeyCode::Tab) {
        free.0 = !free.0;
    }
}

fn fly(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    zoom: Res<Zoom>,
    free: Res<FreeCamera>,
    targets: Query<(), With<CameraTarget>>,
    mut cam: Single<&mut Transform, With<MainCamera>>,
) {
    if !targets.is_empty() && !free.0 {
        return;
    }
    let axis = |neg: [KeyCode; 2], pos: [KeyCode; 2]| {
        keys.any_pressed(pos) as i32 as f32 - keys.any_pressed(neg) as i32 as f32
    };
    let dir = Vec2::new(
        axis([KeyCode::KeyA, KeyCode::ArrowLeft], [KeyCode::KeyD, KeyCode::ArrowRight]),
        axis([KeyCode::KeyS, KeyCode::ArrowDown], [KeyCode::KeyW, KeyCode::ArrowUp]),
    );
    let boost = if keys.pressed(KeyCode::ShiftLeft) { 4.0 } else { 1.0 };
    let speed = FLY_SPEED_PX / zoom.0 as f32 * boost;
    cam.translation += (dir * speed * time.delta_secs()).extend(0.0);
}

fn follow(
    zoom: Res<Zoom>,
    free: Res<FreeCamera>,
    shake: Res<crate::fx::ShakeOffset>,
    mut shaken: Local<Vec2>,
    target: Query<&GlobalTransform, (With<CameraTarget>, Without<MainCamera>)>,
    mut cam: Single<&mut Transform, With<MainCamera>>,
) {
    // Undo last frame's shake, so a free camera doesn't drift.
    cam.translation -= shaken.extend(0.0);
    if let Some(t) = target.iter().next().filter(|_| !free.0) {
        let p = t.translation().truncate();
        cam.translation.x = p.x;
        cam.translation.y = p.y;
    }
    // Snap to whole screen pixels so cells never shimmer; the shake too.
    let ppc = zoom.0 as f32;
    cam.translation.x = (cam.translation.x * ppc).round() / ppc;
    cam.translation.y = (cam.translation.y * ppc).round() / ppc;
    *shaken = (shake.0 * ppc).round() / ppc;
    cam.translation += shaken.extend(0.0);
}

pub fn track_cursor(
    window: Single<&Window, With<PrimaryWindow>>,
    cam: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    over: Res<CursorOverride>,
    mut cursor: ResMut<CursorWorld>,
) {
    let (camera, cam_tf) = *cam;
    cursor.0 = over.0.or_else(|| window.cursor_position().and_then(|p| camera.viewport_to_world_2d(cam_tf, p).ok()));
}
