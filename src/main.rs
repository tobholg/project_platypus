//! src/main.rs
//! Animated 6‑frame idle player (`assets/textures/player_sheet.png`)
//! * sprite flips when walking left / right
//! * Dirt & Stone colours vary in chunky “pixel” patches (quantised Perlin)
//! * colour variance ±20 % so the underground looks richer.

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
const PLAYER_HEIGHT: f32 = 18.0;
const PLAYER_BBOX: Vec2 = Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT);

const GRAVITY: f32 = -400.0;
const JUMP_SPEED: f32 = 230.0;
const JET_ACCEL: f32 = 800.0;
const WALK_SPEED: f32 = 160.0;
const COLLISION_STEPS: i32 = 4;

// ‑‑‑ jet‑pack exhaust ‑‑‑
const EXHAUST_LIFETIME: f32 = 0.6;
const EXHAUST_RATE: usize = 6;
const EXHAUST_SIZE: f32 = 3.0;
const EXHAUST_COLOR: Color = Color::rgba(1.0, 0.6, 0.2, 1.0);
const EXHAUST_SPEED_Y: std::ops::Range<f32> = -300.0..-120.0;
const EXHAUST_SPEED_X: std::ops::Range<f32> = -50.0..50.0;

// ‑‑‑ colour‑variation controls ‑‑‑
const COLOR_NOISE_SCALE: f64 = 0.05;          // bigger patches  ==  lower frequency
const COLOR_VARIATION_LEVELS: i32 = 4;        // number of discrete brightness steps
const COLOR_VARIATION_STRENGTH: f32 = 0.2;    // ±20 %

// -------------------------------------------------
// Y‑axis helpers (row 0 = top of the world)
// -------------------------------------------------
#[inline]
fn tile_to_world_y(terrain_h: usize, tile_y: usize) -> f32 {
    (terrain_h as f32 - 1.0 - tile_y as f32) * TILE_SIZE
}

#[inline]
fn world_to_tile_y(terrain_h: usize, world_y: f32) -> i32 {
    (terrain_h as f32 - 1.0 - (world_y / TILE_SIZE).floor()) as i32
}

// -------------------------
// Terrain data
// -------------------------
#[derive(Resource)]
struct Terrain {
    tiles: Vec<Vec<Tile>>,
    sprite_entities: Vec<Vec<Option<Entity>>>,
    changed_tiles: VecDeque<(usize, usize)>,
    width: usize,
    height: usize,
    #[allow(dead_code)]
    height_map: Vec<usize>,
    color_noise: Perlin,
}

#[derive(Clone, Copy, PartialEq)]
enum TileKind {
    Air,
    Sky,
    Dirt,
    Stone,
}

#[derive(Clone, Copy)]
struct Tile {
    kind: TileKind,
}

// -------------------------
// Animation helpers
// -------------------------
#[derive(Component)]
struct AnimationIndices {
    first: usize,
    last: usize,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

// -------------------------
// ECS components
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
// App entry‑point
// -------------------------
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.45, 0.68, 1.0)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Terraria‑like (chunky cave colours)".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_startup_system(generate_world_and_player)
        .add_startup_system(setup_camera.after(generate_world_and_player))
        .add_system(spawn_initial_tiles)                 // one‑shot
        .add_systems((
            player_input_system,
            physics_and_collision_system,
            digging_system,
            redraw_changed_tiles_system,
            exhaust_update_system,
            animate_player_system,
            camera_follow_system,
        ))
        .run();
}

// -------------------------
// Startup systems
// -------------------------
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn generate_world_and_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlases: ResMut<Assets<TextureAtlas>>,
) {
    // -------- texture atlas (600 × 100 sheet → 6 × 100 × 100 frames) --------
    let sheet_handle = asset_server.load("textures/player_sheet.png");
    let atlas_handle =
        atlases.add(TextureAtlas::from_grid(sheet_handle, Vec2::new(100.0, 100.0), 6, 1, None, None));

    // -------- world dimensions --------
    let w = CHUNK_WIDTH * NUM_CHUNKS_X;
    let h = CHUNK_HEIGHT * NUM_CHUNKS_Y;

    // -------- height map --------
    let mut height_map = vec![0usize; w];
    let noise_h = Perlin::new(rand::thread_rng().gen());
    let base = h as f32 * 0.35;
    let amp_low = 10.0;
    let amp_high = 25.0;

    for x in 0..w {
        let n = noise_h.get([x as f64 * 0.015, 0.0]);
        let elev_top = if n >= 0.0 {
            base - n as f32 * amp_high
        } else {
            base - n as f32 * amp_low
        };
        height_map[x] = elev_top.clamp(4.0, (h - 10) as f32) as usize;
    }

    // -------- tile grid --------
    let mut tiles = vec![vec![Tile { kind: TileKind::Air }; w]; h];
    let sprite_entities = vec![vec![None; w]; h];

    let noise_cave = Perlin::new(rand::thread_rng().gen());
    let mut rng = rand::thread_rng();

    for x in 0..w {
        let surface_top = height_map[x];

        // sky
        for y in 0..surface_top {
            tiles[y][x].kind = TileKind::Sky;
        }
        // ground + caves
        for y in surface_top..h {
            let depth = y - surface_top;
            let n = noise_cave.get([x as f64 * 0.08, y as f64 * 0.08]);
            if n > 0.25 {
                tiles[y][x].kind = TileKind::Air;
            } else if depth > h / 4 {
                tiles[y][x].kind = TileKind::Stone;
            } else {
                tiles[y][x].kind = TileKind::Dirt;
            }
        }
    }

    // cave entrances
    let entrance_count = (w as f32 / 80.0) as usize;
    for _ in 0..entrance_count {
        let ex = rng.gen_range(4..w - 4);
        let surface_top = height_map[ex];
        for dy in 0..12 {
            let ty = surface_top + dy;
            if ty >= h {
                break;
            }
            for dx in -3..=3 {
                let tx = (ex as isize + dx) as usize;
                tiles[ty][tx].kind = TileKind::Air;
            }
        }
    }

    // -------- player spawn --------
    let spawn_x = w / 2;
    let surface_row = height_map[spawn_x];
    let spawn_pos = Vec2::new(
        spawn_x as f32 * TILE_SIZE,
        tile_to_world_y(h, surface_row) + TILE_SIZE * 0.5 + PLAYER_HEIGHT * 0.5,
    );

    commands.spawn((
        SpriteSheetBundle {
            texture_atlas: atlas_handle,
            sprite: TextureAtlasSprite {
                index: 0,
                flip_x: false,
                ..default()
            },
            transform: Transform {
                translation: spawn_pos.extend(10.0),
                scale: Vec3::splat(1.8),  // makes 100 px frame ≈ 160 px tall in‑world
                ..default()
            },
            ..default()
        },
        Player { grounded: false },
        Velocity(Vec2::ZERO),
        AnimationIndices { first: 0, last: 5 },
        AnimationTimer(Timer::from_seconds(0.12, TimerMode::Repeating)),
    ));

    // -------- terrain resource (with colour noise) --------
    commands.insert_resource(Terrain {
        tiles,
        sprite_entities,
        changed_tiles: VecDeque::new(),
        width: w,
        height: h,
        height_map,
        color_noise: Perlin::new(rand::thread_rng().gen()),
    });
}

// -------------------------
// Tile‑sprite helpers
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
            if matches!(terrain.tiles[y][x].kind, TileKind::Dirt | TileKind::Stone) {
                terrain.sprite_entities[y][x] =
                    Some(spawn_tile(&mut commands, &terrain, x, y));
            }
        }
    }
    *done = true;
}

fn spawn_tile(commands: &mut Commands, terrain: &Terrain, x: usize, y: usize) -> Entity {
    // ----- quantised Perlin factor -----
    let raw = terrain
        .color_noise
        .get([x as f64 * COLOR_NOISE_SCALE, y as f64 * COLOR_NOISE_SCALE]) as f32;

    // map [-1,1] → 0..LEVELS, floor, then back to [-1,1]
    let step = (((raw + 1.0) * 0.5) * COLOR_VARIATION_LEVELS as f32).floor()
        .clamp(0.0, (COLOR_VARIATION_LEVELS - 1) as f32);
    let norm = step / (COLOR_VARIATION_LEVELS as f32 - 1.0) * 2.0 - 1.0;

    let factor = 1.0 + norm * COLOR_VARIATION_STRENGTH;

    // base colours
    let base = match terrain.tiles[y][x].kind {
        TileKind::Dirt => Vec3::new(0.55, 0.27, 0.07),
        TileKind::Stone => Vec3::new(0.50, 0.50, 0.50),
        _ => unreachable!(),
    } * factor;

    let colour = Color::rgb(
        base.x.clamp(0.0, 1.0),
        base.y.clamp(0.0, 1.0),
        base.z.clamp(0.0, 1.0),
    );

    commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: colour,
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
                transform: Transform::from_xyz(
                    x as f32 * TILE_SIZE,
                    tile_to_world_y(terrain.height, y),
                    0.0,
                ),
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
        if matches!(terrain.tiles[y][x].kind, TileKind::Dirt | TileKind::Stone) {
            terrain.sprite_entities[y][x] =
                Some(spawn_tile(&mut commands, &terrain, x, y));
        }
    }
}

// -------------------------
// Input (WASD + flip sprite)
// -------------------------
fn player_input_system(
    keys: Res<Input<KeyCode>>,
    mut q: Query<(&mut Velocity, &mut TextureAtlasSprite, &Player)>,
) {
    if let Ok((mut vel, mut sprite, ply)) = q.get_single_mut() {
        match (keys.pressed(KeyCode::A), keys.pressed(KeyCode::D)) {
            (true, false) => {
                vel.0.x = -WALK_SPEED;
                sprite.flip_x = true;
            }
            (false, true) => {
                vel.0.x = WALK_SPEED;
                sprite.flip_x = false;
            }
            _ => vel.0.x = 0.0,
        }
        if keys.just_pressed(KeyCode::Space) && ply.grounded {
            vel.0.y = JUMP_SPEED;
        }
    }
}

// -------------------------
// Physics, collision & jet‑pack
// (unchanged from previous version)
// -------------------------
fn physics_and_collision_system(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    mut q: Query<(&mut Transform, &mut Velocity, &mut Player)>,
    terrain: Res<Terrain>,
) {
    let dt = time.delta_seconds();
    let Ok((mut tf, mut vel, mut ply)) = q.get_single_mut() else { return };

    vel.0.y += GRAVITY * dt;
    if keys.pressed(KeyCode::Space) && !ply.grounded {
        vel.0.y += JET_ACCEL * dt;
    }

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

            let y_top_idx =
                world_to_tile_y(terrain.height, tf.translation.y + half.y - 0.1);
            let y_bot_idx =
                world_to_tile_y(terrain.height, tf.translation.y - half.y + 0.1);
            let (y_min, y_max) = if y_top_idx <= y_bot_idx {
                (y_top_idx, y_bot_idx)
            } else {
                (y_bot_idx, y_top_idx)
            };

            if (y_min..=y_max).any(|ty| solid(&terrain, tx, ty)) {
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
            let ty = world_to_tile_y(terrain.height, probe_y);

            let x_left =
                ((tf.translation.x - half.x + 0.1) / TILE_SIZE).floor() as i32;
            let x_right =
                ((tf.translation.x + half.x - 0.1) / TILE_SIZE).floor() as i32;

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

    // exhaust particles
    if keys.pressed(KeyCode::Space) && !ply.grounded {
        let mut rng = rand::thread_rng();
        for _ in 0..EXHAUST_RATE {
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
        return true;
    }
    matches!(
        terrain.tiles[ty as usize][tx as usize].kind,
        TileKind::Dirt | TileKind::Stone
    )
}

// -------------------------
// Exhaust & digging & camera & animation systems
// (unchanged from previous file – see above)
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
    let min_y_world = world.y - DIG_RADIUS;
    let max_y_world = world.y + DIG_RADIUS;

    let min_y = world_to_tile_y(terrain.height, max_y_world);
    let max_y = world_to_tile_y(terrain.height, min_y_world);

    for ty in min_y..=max_y {
        for tx in min_x..=max_x {
            if tx < 0
                || ty < 0
                || tx >= terrain.width as i32
                || ty >= terrain.height as i32
            {
                continue;
            }
            let dx = tx as f32 * TILE_SIZE - world.x;
            let ty_world = tile_to_world_y(terrain.height, ty as usize);
            let dy = ty_world - world.y;
            if dx * dx + dy * dy < DIG_RADIUS * DIG_RADIUS {
                let (ux, uy) = (tx as usize, ty as usize);
                if matches!(
                    terrain.tiles[uy][ux].kind,
                    TileKind::Dirt | TileKind::Stone
                ) {
                    terrain.tiles[uy][ux].kind = TileKind::Air;
                    terrain.changed_tiles.push_back((ux, uy));
                }
            }
        }
    }
}

fn camera_follow_system(
    mut cam_q: Query<&mut Transform, (With<Camera>, Without<Player>)>,
    player_q: Query<&Transform, With<Player>>,
    window_q: Query<&Window>,
    terrain: Res<Terrain>,
) {
    let Ok(mut cam_tf) = cam_q.get_single_mut() else { return };
    let Ok(player_tf) = player_q.get_single() else { return };
    let window = window_q.single();

    let half_w = window.width() * 0.5;
    let half_h = window.height() * 0.5;
    let world_w = terrain.width as f32 * TILE_SIZE;
    let world_h = terrain.height as f32 * TILE_SIZE;

    cam_tf.translation.x = player_tf.translation.x.clamp(half_w, world_w - half_w);
    cam_tf.translation.y = player_tf.translation.y.clamp(half_h, world_h - half_h);
}

fn animate_player_system(
    time: Res<Time>,
    mut query: Query<(
        &AnimationIndices,
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
    ), With<Player>>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        if timer.tick(time.delta()).just_finished() {
            sprite.index = if sprite.index == indices.last {
                indices.first
            } else {
                sprite.index + 1
            };
        }
    }
}