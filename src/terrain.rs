//! world‑generation, digging, sprite streaming & helpers

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::Rng;
use std::collections::VecDeque;

use crate::constants::*;
use crate::components::{AnimationIndices, AnimationTimer, Player, TileSprite, Velocity};

/* ===========================================================
   tiny helpers (row‑0 = top)
   =========================================================== */
pub fn tile_to_world_y(terrain_h: usize, tile_y: usize) -> f32 {
    (terrain_h as f32 - 1. - tile_y as f32) * TILE_SIZE
}
pub fn world_to_tile_y(terrain_h: usize, world_y: f32) -> i32 {
    (terrain_h as f32 - 1. - (world_y / TILE_SIZE).floor()) as i32
}

/* ===========================================================
   tile kinds and data
   =========================================================== */
#[derive(Clone, Copy, PartialEq)]
pub enum TileKind {
    Air,
    Sky,
    Dirt,
    Stone,
}

#[derive(Clone, Copy)]
pub struct Tile {
    pub kind:     TileKind,
    pub visible:  bool,
    pub explored: bool,
}

/* ===========================================================
   terrain resource
   =========================================================== */
#[derive(Resource)]
pub struct Terrain {
    pub tiles: Vec<Vec<Tile>>,
    pub sprite_entities: Vec<Vec<Option<Entity>>>,
    pub changed_tiles: VecDeque<(usize, usize)>,
    pub width: usize,
    pub height: usize,
    pub height_map: Vec<usize>,
    pub color_noise: Perlin,
}

/* ===========================================================
   “active window” rectangle around the camera
   =========================================================== */
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ActiveRect {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

/* ===========================================================
   constants (world‑gen only)
   =========================================================== */
const MIN_CAVE_DEPTH: usize = 8;
const EXPLORED_BRIGHTNESS: f32 = 0.25;
const BACKGROUND_BROWN: Vec3 = Vec3::new(0.20, 0.10, 0.05);

/* ===========================================================
   startup: generate world + player  ➜  medium‑cave edition
   =========================================================== */
   pub fn generate_world_and_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    /* ----- player sprite‑sheet ----- */
    let sheet = asset_server.load("textures/player_sheet.png");
    let layout =
        TextureAtlasLayout::from_grid(UVec2::new(100, 100), 6, 1, None, None);
    let layout_handle = atlas_layouts.add(layout);

    /* ----- world dimensions ----- */
    let w = CHUNK_WIDTH * NUM_CHUNKS_X;
    let h = CHUNK_HEIGHT * NUM_CHUNKS_Y;

    /* ----- height‑map (perlin + cliffs) ----- */
    let mut height_map = vec![0usize; w];
    let noise_hills  = Perlin::new(rand::thread_rng().gen());
    let noise_cliffs = Perlin::new(rand::thread_rng().gen());

    let base = h as f32 * 0.35;
    let amp_low  =  5.0;
    let amp_high = 12.0;

    let cliff_freq      = 0.12;
    let cliff_threshold = 0.85;
    let cliff_strength  = 18.0;

    for x in 0..w {
        let n = noise_hills.get([x as f64 * 0.01, 0.0]);
        let mut elev = if n >= 0.0 {
            base - n as f32 * amp_high
        } else {
            base - n as f32 * amp_low
        };

        let cliff_sample = noise_cliffs.get([x as f64 * cliff_freq, 100.0]);
        if cliff_sample.abs() > cliff_threshold {
            elev -= cliff_sample.signum() as f32 * cliff_strength;
        }

        height_map[x] = elev.clamp(4.0, (h - 10) as f32) as usize;
    }

    /* ----- fill tiles (ground, caves, sky) ----- */
    let mut tiles = vec![
        vec![
            Tile {
                kind: TileKind::Air,
                visible: false,
                explored: false,
            };
            w
        ];
        h
    ];
    let sprite_entities = vec![vec![None; w]; h];

    /*  MEDIUM‑CAVE parameters  */
    let cave_freq:   f64 = 0.04;  // halfway between 0.08 and 0.02
    let cave_thresh: f64 = 0.30;  // halfway between 0.40 and 0.20
    let noise_cave = Perlin::new(rand::thread_rng().gen());

    for x in 0..w {
        let surface = height_map[x];

        // sky
        for y in 0..surface {
            tiles[y][x].kind = TileKind::Sky;
        }
        // ground + caverns
        for y in surface..h {
            let depth = y - surface;
            if depth < MIN_CAVE_DEPTH {
                tiles[y][x].kind =
                    if depth > h / 4 { TileKind::Stone } else { TileKind::Dirt };
                continue;
            }

            let n = noise_cave.get([x as f64 * cave_freq, y as f64 * cave_freq]);
            tiles[y][x].kind = if n > cave_thresh {
                TileKind::Air
            } else if depth > h / 4 {
                TileKind::Stone
            } else {
                TileKind::Dirt
            };
        }
    }

    /* ----- spawn player at map centre ----- */
    let spawn_x  = w / 2;
    let surf_row = height_map[spawn_x];
    let spawn = Vec2::new(
        spawn_x as f32 * TILE_SIZE,
        tile_to_world_y(h, surf_row)
            + TILE_SIZE * 0.5
            + PLAYER_HEIGHT * 0.5
            + 4.0,
    );

    commands.spawn((
        Sprite::from_atlas_image(
            sheet.clone(),
            TextureAtlas { layout: layout_handle.clone(), index: 0 },
        ),
        Transform {
            translation: spawn.extend(10.0),
            scale: Vec3::splat(1.8),
            ..default()
        },
        Player { grounded: false },
        Velocity(Vec2::ZERO),
        AnimationIndices { first: 0, last: 5 },
        AnimationTimer(Timer::from_seconds(0.12, TimerMode::Repeating)),
    ));

    /* ----- insert terrain resource ----- */
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

/* ===========================================================
   streaming tile sprites – runs every frame
   =========================================================== */
pub fn stream_tiles_system(
    mut commands: Commands,
    mut terrain: ResMut<Terrain>,
    rect: Res<ActiveRect>,
) {
    let w = terrain.width as i32;
    let h = terrain.height as i32;

    /* -------- despawn tiles that left the rect -------- */
    for y in 0..h {
        for x in 0..w {
            let (ux, uy) = (x as usize, y as usize);
            if let Some(e) = terrain.sprite_entities[uy][ux] {
                if x < rect.min_x || x > rect.max_x || y < rect.min_y || y > rect.max_y {
                    commands.entity(e).despawn();
                    terrain.sprite_entities[uy][ux] = None;
                }
            }
        }
    }

    /* -------- spawn tiles that entered the rect -------- */
    for y in rect.min_y..=rect.max_y {
        for x in rect.min_x..=rect.max_x {
            let (ux, uy) = (x as usize, y as usize);
            if terrain.sprite_entities[uy][ux].is_none()
                && matches!(terrain.tiles[uy][ux].kind,
                            TileKind::Dirt | TileKind::Stone | TileKind::Air)
            {
                terrain.sprite_entities[uy][ux] =
                    Some(spawn_tile(&mut commands, &terrain, ux, uy));
            }
        }
    }
}

/* ===========================================================
   update the ActiveRect after the camera has moved
   =========================================================== */
pub fn update_active_rect_system(
    cam_q: Query<&Transform, With<Camera>>,
    window_q: Query<&Window>,
    terrain: Res<Terrain>,
    mut rect_res: Option<ResMut<ActiveRect>>,
    mut commands: Commands,
) {
    let cam_tf = match cam_q.get_single() {
        Ok(t) => t,
        Err(_) => return,
    };
    let window = window_q.single();

    let half_w = window.width()  * 0.5;
    let half_h = window.height() * 0.5;

    // half‑viewport in *tiles*
    let pad_x = ((window.width()  * 0.5) / TILE_SIZE).ceil() as i32 + ACTIVE_MARGIN;
    let pad_y = ((window.height() * 0.5) / TILE_SIZE).ceil() as i32 + ACTIVE_MARGIN;

    let px = (cam_tf.translation.x / TILE_SIZE).round() as i32;
    let py = world_to_tile_y(terrain.height, cam_tf.translation.y);

    let new = ActiveRect {
        min_x: (px - pad_x).clamp(0, terrain.width  as i32 - 1),
        max_x: (px + pad_x).clamp(0, terrain.width  as i32 - 1),
        min_y: (py - pad_y).clamp(0, terrain.height as i32 - 1),
        max_y: (py + pad_y).clamp(0, terrain.height as i32 - 1),
    };

    match rect_res {
        Some(mut r) if *r != new => *r = new,
        None                     => { commands.insert_resource(new); }
        _ => {}
    }

    // suppress warning about unused vars (half_w/h) – they’re kept for clarity
    let _ = (half_w, half_h);
}

/* ===========================================================
   helpers
   =========================================================== */
fn brightness(tile: &Tile) -> f32 {
    if tile.visible {
        1.0
    } else if tile.explored {
        EXPLORED_BRIGHTNESS
    } else {
        0.0
    }
}

pub fn spawn_tile(commands: &mut Commands, terrain: &Terrain, x: usize, y: usize) -> Entity {
    use crate::constants::{COLOR_NOISE_SCALE, COLOR_VARIATION_LEVELS, COLOR_VARIATION_STRENGTH};

    let raw = terrain
        .color_noise
        .get([x as f64 * COLOR_NOISE_SCALE, y as f64 * COLOR_NOISE_SCALE])
        as f32;

    let step = (((raw + 1.0) * 0.5) * COLOR_VARIATION_LEVELS as f32)
        .floor()
        .clamp(0.0, (COLOR_VARIATION_LEVELS - 1) as f32);

    let norm   = step / (COLOR_VARIATION_LEVELS as f32 - 1.0) * 2.0 - 1.0;
    let factor = 1.0 + norm * COLOR_VARIATION_STRENGTH;

    let base_rgb = match terrain.tiles[y][x].kind {
        TileKind::Dirt  => Vec3::new(0.55, 0.27, 0.07) * factor,
        TileKind::Stone => Vec3::new(0.50, 0.50, 0.50) * factor,
        TileKind::Air   => BACKGROUND_BROWN,
        _ => unreachable!(),
    } * brightness(&terrain.tiles[y][x]);

    commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(
                        base_rgb.x.clamp(0.0, 1.0),
                        base_rgb.y.clamp(0.0, 1.0),
                        base_rgb.z.clamp(0.0, 1.0),
                    ),
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
                transform: Transform::from_xyz(
                    x as f32 * TILE_SIZE,
                    tile_to_world_y(terrain.height, y),
                    if terrain.tiles[y][x].kind == TileKind::Air { -1.0 } else { 0.0 },
                ),
                ..default()
            },
            TileSprite { x, y },
        ))
        .id()
}

/* ===========================================================
   redraw tiles whose kind/visibility changed
   =========================================================== */
pub fn redraw_changed_tiles_system(mut commands: Commands, mut terrain: ResMut<Terrain>) {
    while let Some((x, y)) = terrain.changed_tiles.pop_front() {
        if let Some(e) = terrain.sprite_entities[y][x] {
            commands.entity(e).despawn();
            terrain.sprite_entities[y][x] = None;
        }
        if matches!(terrain.tiles[y][x].kind,
                    TileKind::Dirt | TileKind::Stone | TileKind::Air)
        {
            terrain.sprite_entities[y][x] =
                Some(spawn_tile(&mut commands, &terrain, x, y));
        }
    }
}

/* ===========================================================
   digging with the mouse
   =========================================================== */
pub fn digging_system(
    mouse: Res<ButtonInput<MouseButton>>,
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

    let Ok(world) = cam.viewport_to_world_2d(cam_tf, cursor) else { return };

    let min_x = ((world.x - DIG_RADIUS) / TILE_SIZE).floor() as i32;
    let max_x = ((world.x + DIG_RADIUS) / TILE_SIZE).ceil() as i32;

    let min_y_world = world.y - DIG_RADIUS;
    let max_y_world = world.y + DIG_RADIUS;
    let min_y = world_to_tile_y(terrain.height, max_y_world);
    let max_y = world_to_tile_y(terrain.height, min_y_world);

    for ty in min_y..=max_y {
        for tx in min_x..=max_x {
            if tx < 0 || ty < 0 || tx >= terrain.width as i32 || ty >= terrain.height as i32 {
                continue;
            }
            let dx = tx as f32 * TILE_SIZE - world.x;
            let dy = tile_to_world_y(terrain.height, ty as usize) - world.y;
            if dx * dx + dy * dy < DIG_RADIUS * DIG_RADIUS {
                let (ux, uy) = (tx as usize, ty as usize);
                if matches!(terrain.tiles[uy][ux].kind, TileKind::Dirt | TileKind::Stone) {
                    terrain.tiles[uy][ux].kind = TileKind::Air;
                    terrain.changed_tiles.push_back((ux, uy));
                }
            }
        }
    }
}

/* ===========================================================
   physics helper
   =========================================================== */
pub fn solid(terrain: &Terrain, tx: i32, ty: i32) -> bool {
    if tx < 0 || ty < 0 || tx >= terrain.width as i32 || ty >= terrain.height as i32 {
        return true;
    }
    matches!(
        terrain.tiles[ty as usize][tx as usize].kind,
        TileKind::Dirt | TileKind::Stone
    )
}