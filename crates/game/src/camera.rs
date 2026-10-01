//! Camera: follows a `CameraTarget` if there is one, otherwise flies freely
//! (WASD / arrows, Shift for speed). Tab detaches it from the player.
//! The keys that type `+` and `-` zoom in whole screen-pixels per cell,
//! whatever the keyboard layout (`=` too, and the keypad's).

use bevy::input::keyboard::Key;
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
        let start_zoom = std::env::var("PLATYPUS_ZOOM").ok().and_then(|z| z.parse().ok()).filter(|z| ZOOM_LEVELS.contains(z)).unwrap_or(if crate::data::hd() { 2 } else { 3 });
        app.insert_resource(Zoom(start_zoom))
            .init_resource::<CursorWorld>()
            .init_resource::<CursorOverride>()
            .init_resource::<FreeCamera>()
            .insert_resource(StartAt(self.start))
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, ((toggle_free, zoom, fly, apply_zoom).chain(), fit_mirror))
            .add_systems(PreUpdate, track_cursor)
            .add_systems(PostUpdate, follow.before(TransformSystems::Propagate));
    }
}

/// `PLATYPUS_OFFSCREEN=1`: the camera draws into this image (screenshots
/// from scenarios still work with the screen locked or asleep, when the
/// window isn't drawn), and the window shows it.
#[derive(Resource)]
pub struct Offscreen(pub Handle<Image>);

fn spawn_camera(mut commands: Commands, start: Res<StartAt>, mut images: ResMut<Assets<Image>>) {
    let mut cam = commands.spawn((
        Camera2d,
        MainCamera,
        Transform::from_translation(start.0.extend(100.0)),
        ChunkLoader { half_extent: Vec2::new(400.0, 240.0) },
    ));
    if std::env::var("PLATYPUS_OFFSCREEN").is_ok_and(|v| !v.is_empty()) {
        let image = images.add(Image::new_target_texture(1512, 917, bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb, None));
        // (The panels too: the UI follows the camera marked for it.)
        cam.insert((bevy::camera::RenderTarget::Image(image.clone().into()), IsDefaultUiCamera));
        // And the window shows that image (so a scenario can be watched):
        // a camera of its own, seeing only the image, on a layer of its own.
        let mirror = bevy::camera::visibility::RenderLayers::layer(MIRROR_LAYER);
        commands.spawn((Name::new("Window mirror camera"), Camera2d, Camera { order: 1, ..default() }, mirror.clone()));
        commands.spawn((Name::new("Window mirror"), OffscreenMirror, Sprite { image: image.clone(), ..default() }, mirror));
        commands.insert_resource(Offscreen(image));
    }
}

/// The render layer the offscreen image is shown to the window on.
const MIRROR_LAYER: usize = 31;

/// The sprite showing the offscreen image in the window.
#[derive(Component)]
struct OffscreenMirror;

/// The mirror fills the window, whatever its size.
fn fit_mirror(window: Single<&Window, With<PrimaryWindow>>, mut mirror: Query<&mut Sprite, With<OffscreenMirror>>) {
    let size = Vec2::new(window.width(), window.height());
    for mut s in &mut mirror {
        if s.custom_size != Some(size) {
            s.custom_size = Some(size);
        }
    }
}

fn zoom(chars: Res<ButtonInput<Key>>, keys: Res<ButtonInput<KeyCode>>, mut zoom: ResMut<Zoom>) {
    // What the key types, not where it is: + is Shift+= on a US keyboard and
    // beside 0 on a Norwegian one.
    let typed = |c: &str| chars.just_pressed(Key::Character(c.into()));
    let step = (typed("+") || typed("=") || keys.just_pressed(KeyCode::NumpadAdd)) as i32 - (typed("-") || keys.just_pressed(KeyCode::NumpadSubtract)) as i32;
    if step == 0 {
        return;
    }
    let i = ZOOM_LEVELS.iter().position(|&z| z == zoom.0).unwrap_or(2) as i32;
    let j = (i + step).clamp(0, ZOOM_LEVELS.len() as i32 - 1);
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

/// The target moved further than this in a frame (cells): it went through
/// a portal or blinked, and the camera glides after it...
const TELEPORT: f32 = 40.0;
/// ... over this long (seconds), easing in and out.
const GLIDE: f32 = 0.3;
/// Pixels added before snapping the camera (see `follow`).
const SNAP_BIAS: f32 = 0.25;

/// Where the camera last saw its target, and a glide under way (where it
/// started, how far in); the height it's at and how fast that's moving
/// (`rise`).
#[derive(Default)]
pub struct Glide {
    last: Option<Vec2>,
    from: Option<(Vec2, f32)>,
    height: Option<(f32, f32)>,
    /// Where the target was last frame (its height).
    to: Option<f32>,
}

/// On the ground the camera's height follows the target's through a
/// critically damped spring this quick (seconds): a body stepping up a hill
/// snaps up a cell or two a tick and pauses, and a camera on it lurched
/// with every step; on the spring a staircase is a ramp. Never more than
/// `RISE_LAG` cells behind. In the air it moves with the target from the
/// first frame (rocket boots, a jump: a spring started from rest sat still
/// for frames as they took off), what it was behind closing over
/// `AIR_CATCH` seconds.
const RISE_TIME: f32 = 0.1;
const RISE_LAG: f32 = 6.0;
const AIR_CATCH: f32 = 0.05;

/// A critically damped spring from `at` (moving at `speed`) toward `to`
/// over `dt` (as `SmoothDamp`, Game Programming Gems 4, 1.10): the new
/// height and speed.
fn rise(at: f32, speed: f32, to: f32, dt: f32) -> (f32, f32) {
    let omega = 2.0 / RISE_TIME;
    let x = omega * dt;
    let decay = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
    let change = (at - to).clamp(-RISE_LAG, RISE_LAG);
    let temp = (speed + omega * change) * dt;
    let speed = (speed - omega * temp) * decay;
    let mut next = to + (change + temp) * decay;
    // (Not past the target.)
    if (to - at > 0.0) == (next > to) {
        next = to;
        return (next, 0.0);
    }
    (next, speed)
}

/// What the camera follows: where it's drawn, and its body (on the ground?).
type Target<'a> = (&'a GlobalTransform, Option<&'a crate::creatures::Kinematics>);

#[allow(clippy::too_many_arguments)]
pub fn follow(
    time: Res<Time>,
    zoom: Res<Zoom>,
    free: Res<FreeCamera>,
    shake: Res<crate::fx::ShakeOffset>,
    mut shaken: Local<Vec2>,
    mut glide: Local<Glide>,
    target: Query<Target, (With<CameraTarget>, Without<MainCamera>)>,
    mut cam: Single<&mut Transform, With<MainCamera>>,
) {
    // Undo last frame's shake, so a free camera doesn't drift.
    cam.translation -= shaken.extend(0.0);
    if let Some((t, k)) = target.iter().next().filter(|_| !free.0) {
        let p = t.translation().truncate();
        let grounded = k.is_none_or(|k| k.loco.grounded());
        // A jump across the world (a blink, a portal): glide there rather
        // than cut.
        if glide.last.is_some_and(|l| l.distance(p) > TELEPORT) {
            glide.from = Some((cam.translation.truncate(), 0.0));
        }
        glide.last = Some(p);
        let at = match glide.from {
            Some((from, done)) => {
                let done = done + time.delta_secs();
                let k = (done / GLIDE).clamp(0.0, 1.0);
                glide.from = (k < 1.0).then_some((from, done));
                from.lerp(p, k * k * (3.0 - 2.0 * k))
            }
            None => p,
        };
        cam.translation.x = at.x;
        // (Gliding, or just taken up: straight there.)
        let dt = time.delta_secs().min(0.1);
        let (y, speed) = match glide.height {
            Some((y, speed)) if glide.from.is_none() && grounded => rise(y, speed, at.y, dt),
            // (In the air: with it, the gap closing; how fast it went, for
            // the spring when it lands.)
            Some((y, _)) if glide.from.is_none() && dt > 0.0 => {
                let gap = (y - glide.to.unwrap_or(at.y)) * (-dt / AIR_CATCH).exp();
                let next = at.y + gap;
                (next, (next - y) / dt)
            }
            _ => (at.y, 0.0),
        };
        glide.to = Some(at.y);
        glide.height = Some((y, speed));
        cam.translation.y = y;
    } else {
        glide.height = None;
        glide.to = None;
    }
    // Snap to whole screen pixels so cells never shimmer; the shake too.
    // (A quarter pixel off the halves: a body at rest stands half a cell
    // up, which at 3 pixels a cell is a tie, and float noise flipped it,
    // the whole world hopping a pixel up and down against the sky.)
    let ppc = zoom.0 as f32;
    let snap = |v: f32| (v * ppc + SNAP_BIAS).round() / ppc;
    cam.translation.x = snap(cam.translation.x);
    cam.translation.y = snap(cam.translation.y);
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
