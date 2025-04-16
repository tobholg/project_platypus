use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::Rng;
use std::collections::VecDeque;

// -------------------------
// Tunables
// -------------------------
const TILE_SIZE: f32 = 8.0;
const CHUNK_WIDTH: usize = 160;
const CHUNK_HEIGHT: usize = 90;
const NUM_CHUNKS_X: usize = 3;
const NUM_CHUNKS_Y: usize = 2;

const DIG_RADIUS: f32 = 16.0;

const PLAYER_WIDTH: f32 = 8.0;
const PLAYER_HEIGHT: f32 = 20.0;
const PLAYER_BBOX: Vec2 = Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT);

const GRAVITY: f32 = -400.0;     // px / s²
const JUMP_SPEED: f32 = 230.0;   // px / s
const JET_ACCEL: f32 = 800.0;    // px / s²
const WALK_SPEED: f32 = 160.0;   // px / s
const COLLISION_STEPS: i32 = 4;  // micro‑steps per frame

// ‑‑‑ bolder jet‑pack exhaust ‑‑‑
const EXHAUST_LIFETIME: f32 = 0.6;
const EXHAUST_RATE: usize = 6;             // sprites per frame
const EXHAUST_SIZE: f32 = 3.0;             // px
const EXHAUST_COLOR: Color = Color::rgba(1.0, 0.6, 0.2, 1.0);
const EXHAUST_SPEED_Y: std::ops::Range<f32> = -300.0..-120.0;
const EXHAUST_SPEED_X: std::ops::Range<f32> = -50.0..50.0;

// -------------------------
// Terrain
// -------------------------
#[derive(Resource)]
struct Terrain {
    tiles: Vec<Vec<Tile>>,
    sprite_entities: Vec<Vec<Option<Entity>>>,
    changed_tiles: VecDeque<(usize, usize)>,
    width: usize,
    height: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum TileKind {
    Air,
    Dirt,
    Stone,
}

#[derive(Clone, Copy)]
struct Tile {
    kind: TileKind,
}

// -------------------------
// Components
// -------------------------
#[derive(Component)]
struct Player {
    grounded: bool,
}

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component)]
struct Exhaust {
    life: f32,
}

#[allow(dead_code)]
#[derive(Component)]
struct TileSprite {
    x: usize,
    y: usize,
}

// -------------------------
// App
// -------------------------
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Terraria‑like (bold jet‑pack)".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_startup_system(generate_world_and_player)
        .add_startup_system(setup_camera.after(generate_world_and_player))
        .add_system(spawn_initial_tiles) // once
        .add_systems((
            player_input_system,
            physics_and_collision_system,
            digging_system,
            redraw_changed_tiles_system,
            exhaust_update_system,
            camera_follow_system,
        ))
        .run();
}

// -------------------------
// Startup
// -------------------------
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn generate_world_and_player(mut commands: Commands) {
    let w = CHUNK_WIDTH * NUM_CHUNKS_X;
    let h = CHUNK_HEIGHT * NUM_CHUNKS_Y;

    let mut tiles = vec![vec![Tile { kind: TileKind::Air }; w]; h];
    let sprite_entities = vec![vec![None; w]; h];
    let noise = Perlin::new(rand::thread_rng().gen());

    for y in 0..h {
        for x in 0..w {
            let n = noise.get([x as f64 * 0.05, y as f64 * 0.05]);
            if n > 0.0 {
                tiles[y][x].kind = TileKind::Dirt;
            }
            if y < h / 4 && n > 0.3 {
                tiles[y][x].kind = TileKind::Stone;
            }
        }
    }

    // spawn pocket
    let (cx, cy) = (w / 2, h / 2);
    for dy in -2..=2 {
        for dx in -2..=2 {
            tiles[(cy as isize + dy) as usize][(cx as isize + dx) as usize].kind = TileKind::Air;
        }
    }

    // stick‑figure parent with full visibility
    let spawn_pos = Vec2::new(w as f32 * TILE_SIZE * 0.5, h as f32 * TILE_SIZE * 0.5);
    commands
        .spawn((
            SpatialBundle::from_transform(Transform::from_xyz(spawn_pos.x, spawn_pos.y, 10.0)),
            Player { grounded: false },
            Velocity(Vec2::ZERO),
        ))
        .with_children(|p| {
            // torso
            p.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::WHITE,
                    custom_size: Some(Vec2::new(2.0, 12.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                ..default()
            });
            // head
            p.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::WHITE,
                    custom_size: Some(Vec2::splat(4.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 10.0, 0.0),
                ..default()
            });
            // legs
            for x in [-1.0, 1.0] {
                p.spawn(SpriteBundle {
                    sprite: Sprite {
                        color: Color::WHITE,
                        custom_size: Some(Vec2::new(1.5, 8.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(x, -10.0, 0.0),
                    ..default()
                });
            }
        });

    commands.insert_resource(Terrain {
        tiles,
        sprite_entities,
        changed_tiles: VecDeque::new(),
        width: w,
        height: h,
    });
}

// -------------------------
// Tile helpers
// -------------------------
fn spawn_initial_tiles(
    mut commands: Commands,
    mut terrain: ResMut<Terrain>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    for y in 0..terrain.height {
        for x in 0..terrain.width {
            if terrain.tiles[y][x].kind != TileKind::Air {
                terrain.sprite_entities[y][x] =
                    Some(spawn_tile(&mut commands, &terrain, x, y));
            }
        }
    }
    *done = true;
}

fn spawn_tile(commands: &mut Commands, terrain: &Terrain, x: usize, y: usize) -> Entity {
    let colour = match terrain.tiles[y][x].kind {
        TileKind::Dirt => Color::rgb(0.55, 0.27, 0.07),
        TileKind::Stone => Color::rgb(0.5, 0.5, 0.5),
        _ => unreachable!(),
    };
    commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: colour,
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
                transform: Transform::from_xyz(x as f32 * TILE_SIZE, y as f32 * TILE_SIZE, 0.0),
                ..default()
            },
            TileSprite { x, y },
        ))
        .id()
}

fn redraw_changed_tiles_system(mut commands: Commands, mut terrain: ResMut<Terrain>) {
    while let Some((x, y)) = terrain.changed_tiles.pop_front() {
        if let Some(e) = terrain.sprite_entities[y][x] {
            commands.entity(e).despawn();
            terrain.sprite_entities[y][x] = None;
        }
        if terrain.tiles[y][x].kind != TileKind::Air {
            terrain.sprite_entities[y][x] =
                Some(spawn_tile(&mut commands, &terrain, x, y));
        }
    }
}

// -------------------------
// Input (WASD + jump)
// -------------------------
fn player_input_system(
    keys: Res<Input<KeyCode>>,
    mut q: Query<(&mut Velocity, &Player)>,
) {
    let (mut vel, ply) = if let Ok(v) = q.get_single_mut() { v } else { return };

    vel.0.x = match (keys.pressed(KeyCode::A), keys.pressed(KeyCode::D)) {
        (true, false) => -WALK_SPEED,
        (false, true) => WALK_SPEED,
        _ => 0.0,
    };

    if keys.just_pressed(KeyCode::Space) && ply.grounded {
        vel.0.y = JUMP_SPEED;
    }
}

// -------------------------
// Physics, collision & jet‑pack
// -------------------------
fn physics_and_collision_system(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    mut q: Query<(&mut Transform, &mut Velocity, &mut Player)>,
    terrain: Res<Terrain>,
) {
    let dt = time.delta_seconds();
    let (mut tf, mut vel, mut ply) = if let Ok(v) = q.get_single_mut() { v } else { return };

    vel.0.y += GRAVITY * dt;

    if keys.pressed(KeyCode::Space) && !ply.grounded {
        vel.0.y += JET_ACCEL * dt;
    }

    // micro‑steps
    let step_dt = dt / COLLISION_STEPS as f32;
    let half = PLAYER_BBOX / 2.0;
    ply.grounded = false;

    for _ in 0..COLLISION_STEPS {
        // X axis
        if vel.0.x != 0.0 {
            let new_x = tf.translation.x + vel.0.x * step_dt;
            let dir = vel.0.x.signum();
            let probe_x = new_x + dir * half.x;
            let tx = (probe_x / TILE_SIZE).floor() as i32;
            let y_top = ((tf.translation.y + half.y - 0.1) / TILE_SIZE).floor() as i32;
            let y_bot = ((tf.translation.y - half.y + 0.1) / TILE_SIZE).floor() as i32;
            if (y_bot..=y_top).any(|ty| solid(&terrain, tx, ty)) {
                vel.0.x = 0.0;
            } else {
                tf.translation.x = new_x;
            }
        }

        // Y axis
        if vel.0.y != 0.0 {
            let new_y = tf.translation.y + vel.0.y * step_dt;
            let dir = vel.0.y.signum();
            let probe_y = new_y + dir * half.y;
            let ty = (probe_y / TILE_SIZE).floor() as i32;
            let x_left = ((tf.translation.x - half.x + 0.1) / TILE_SIZE).floor() as i32;
            let x_right = ((tf.translation.x + half.x - 0.1) / TILE_SIZE).floor() as i32;
            if (x_left..=x_right).any(|tx| solid(&terrain, tx, ty)) {
                if vel.0.y < 0.0 {
                    ply.grounded = true;
                }
                vel.0.y = 0.0;
            } else {
                tf.translation.y = new_y;
            }
        }
    }

    // bold exhaust
    if keys.pressed(KeyCode::Space) && !ply.grounded {
        for _ in 0..EXHAUST_RATE {
            let mut rng = rand::thread_rng();
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: EXHAUST_COLOR,
                        custom_size: Some(Vec2::splat(EXHAUST_SIZE)),
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        tf.translation.x + rng.gen_range(-2.0..2.0),
                        tf.translation.y - half.y,
                        5.0,
                    ),
                    ..default()
                },
                Velocity(Vec2::new(
                    rng.gen_range(EXHAUST_SPEED_X.clone()),
                    rng.gen_range(EXHAUST_SPEED_Y.clone()),
                )),
                Exhaust {
                    life: EXHAUST_LIFETIME,
                },
            ));
        }
    }
}

#[inline]
fn solid(terrain: &Terrain, tx: i32, ty: i32) -> bool {
    if tx < 0 || ty < 0 || tx >= terrain.width as i32 || ty >= terrain.height as i32 {
        return false;
    }
    matches!(
        terrain.tiles[ty as usize][tx as usize].kind,
        TileKind::Dirt | TileKind::Stone
    )
}

// -------------------------
// Exhaust update
// -------------------------
fn exhaust_update_system(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, &mut Sprite, &Velocity, &mut Exhaust)>,
) {
    let dt = time.delta_seconds();
    for (e, mut tf, mut spr, vel, mut ex) in &mut q {
        tf.translation.x += vel.0.x * dt;
        tf.translation.y += vel.0.y * dt;
        ex.life -= dt;
        spr.color.set_a(ex.life / EXHAUST_LIFETIME);
        if ex.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

// -------------------------
// Digging (continuous)
// -------------------------
fn digging_system(
    mouse: Res<Input<MouseButton>>,
    windows: Query<&Window>,
    cam_q: Query<(&Camera, &GlobalTransform)>,
    mut terrain: ResMut<Terrain>,
) {
    if !mouse.pressed(MouseButton::Left) {
        return;
    }
    let window = windows.single();
    let Some(cursor) = window.cursor_position() else { return };
    let (cam, cam_tf) = cam_q.single();

    let ndc = (cursor / Vec2::new(window.width(), window.height())) * 2.0 - Vec2::ONE;
    let world =
        (cam_tf.compute_matrix() * cam.projection_matrix().inverse() * ndc.extend(-1.0).extend(1.0))
            .truncate();

    let min_x = ((world.x - DIG_RADIUS) / TILE_SIZE).floor() as i32;
    let max_x = ((world.x + DIG_RADIUS) / TILE_SIZE).ceil() as i32;
    let min_y = ((world.y - DIG_RADIUS) / TILE_SIZE).floor() as i32;
    let max_y = ((world.y + DIG_RADIUS) / TILE_SIZE).ceil() as i32;

    for ty in min_y..=max_y {
        for tx in min_x..=max_x {
            if tx < 0 || ty < 0 || tx >= terrain.width as i32 || ty >= terrain.height as i32 {
                continue;
            }
            let dx = tx as f32 * TILE_SIZE - world.x;
            let dy = ty as f32 * TILE_SIZE - world.y;
            if dx * dx + dy * dy < DIG_RADIUS * DIG_RADIUS {
                let (ux, uy) = (tx as usize, ty as usize);
                if terrain.tiles[uy][ux].kind != TileKind::Air {
                    terrain.tiles[uy][ux].kind = TileKind::Air;
                    terrain.changed_tiles.push_back((ux, uy));
                }
            }
        }
    }
}

// -------------------------
// Camera
// -------------------------
fn camera_follow_system(
    mut cam_q: Query<&mut Transform, (With<Camera>, Without<Player>)>,
    player_q: Query<&Transform, With<Player>>,
    window_q: Query<&Window>,
    terrain: Res<Terrain>,
) {
    let mut cam_tf = if let Ok(t) = cam_q.get_single_mut() { t } else { return };
    let player_tf = if let Ok(t) = player_q.get_single() { t } else { return };
    let window = window_q.single();

    let half_w = window.width() * 0.5;
    let half_h = window.height() * 0.5;
    let world_w = terrain.width as f32 * TILE_SIZE;
    let world_h = terrain.height as f32 * TILE_SIZE;

    cam_tf.translation.x = player_tf.translation.x.clamp(half_w, world_w - half_w);
    cam_tf.translation.y = player_tf.translation.y.clamp(half_h, world_h - half_h);
}