//! world generation, streaming sprites, digging & helpers
//!
//! Fast‑path optimisations currently enabled
//! -----------------------------------------
//! 1. “Stripe differencing” – only the rows / columns that actually enter or
//!    leave the active window are touched (O(w+h) work per tile‑crossing).
//! 2. **Sprite pooling**   – tiles that scroll out of view are not despawned;
//!    their entities are pushed into `Terrain::free_sprites` and recycled for
//!    incoming tiles. This eliminates Bevy’s archetype churn and cuts CPU
//!    time by an order of magnitude.
//
//! Compatible with **Bevy 0.15**.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::Rng;
use std::collections::VecDeque;

use crate::components::*;
use crate::constants::*;

/* ===========================================================
   helpers (row‑0 = top)
   =========================================================== */
#[inline]
pub fn tile_to_world_y(terrain_h: usize, tile_y: usize) -> f32 {
    (terrain_h as f32 - 1. - tile_y as f32) * TILE_SIZE
}
#[inline]
pub fn world_to_tile_y(terrain_h: usize, world_y: f32) -> i32 {
    (terrain_h as f32 - 1. - (world_y / TILE_SIZE).floor()) as i32
}

/* ===========================================================
   tile data
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
   resources
   =========================================================== */
#[derive(Resource)]
pub struct Terrain {
    pub tiles:           Vec<Vec<Tile>>,
    pub sprite_entities: Vec<Vec<Option<Entity>>>,
    pub changed_tiles:   VecDeque<(usize, usize)>,
    pub free_sprites:    Vec<Entity>,          //  ← sprite pool
    pub width:           usize,
    pub height:          usize,
    pub height_map:      Vec<usize>,
    pub color_noise:     Perlin,
}

/* sliding active rectangle ------------------------------------------------ */
#[derive(Resource, Copy, Clone, PartialEq, Eq, Debug)]
pub struct ActiveRect {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

/* rectangle from previous frame (for early‑out) --------------------------- */
#[derive(Resource, Default)]
pub struct LastRect(pub Option<ActiveRect>);

/* ===========================================================
   parameters
   =========================================================== */
const MIN_CAVE_DEPTH: usize = 8;
const BACKGROUND_BROWN: Vec3 = Vec3::new(0.20, 0.10, 0.05);
const EXPLORED_BRIGHTNESS: f32 = 0.25;

/* ===========================================================
   generate world + player (unchanged except for pool init)
   =========================================================== */
pub fn generate_world_and_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    /* --- sprite sheet ---------------------------------------------------- */
    let sheet = asset_server.load("textures/player_sheet.png");
    let layout =
        TextureAtlasLayout::from_grid(UVec2::new(100, 100), 6, 1, None, None);
    let layout_handle = atlas_layouts.add(layout);

    /* --- overall dimensions --------------------------------------------- */
    let w = CHUNK_WIDTH  * NUM_CHUNKS_X;
    let h = CHUNK_HEIGHT * NUM_CHUNKS_Y;

    /* --- height‑map (hills + cliffs) ------------------------------------ */
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

    /* --- terrain tiles --------------------------------------------------- */
    let mut tiles = vec![
        vec![
            Tile {
                kind:     TileKind::Air,
                visible:  false,
                explored: false,
            };
            w
        ];
        h
    ];
    let sprite_entities = vec![vec![None; w]; h];

    let cave_freq   = 0.04;
    let cave_thresh = 0.30;
    let noise_cave  = Perlin::new(rand::thread_rng().gen());

    for x in 0..w {
        let surface = height_map[x];

        /* sky */
        for y in 0..surface {
            tiles[y][x].kind = TileKind::Sky;
        }

        /* ground / caves */
        for y in surface..h {
            let depth = y - surface;
            if depth < MIN_CAVE_DEPTH {
                tiles[y][x].kind =
                    if depth > h / 4 { TileKind::Stone } else { TileKind::Dirt };
                continue;
            }
            let n =
                noise_cave.get([x as f64 * cave_freq, y as f64 * cave_freq]);
            tiles[y][x].kind = if n > cave_thresh {
                TileKind::Air
            } else if depth > h / 4 {
                TileKind::Stone
            } else {
                TileKind::Dirt
            };
        }
    }

    /* --- spawn player ---------------------------------------------------- */
    let spawn_x  = w / 2;
    let surf_row = height_map[spawn_x];
    let spawn = Vec2::new(
        spawn_x as f32 * TILE_SIZE,
        tile_to_world_y(h, surf_row)
            + TILE_SIZE     * 0.5
            + PLAYER_HEIGHT * 0.5
            + 4.0,
    );

    commands.spawn((
        Sprite::from_atlas_image(
            sheet,
            TextureAtlas {
                layout: layout_handle,
                index: 0,
            },
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

    /* --- resources ------------------------------------------------------- */
    commands.insert_resource(Terrain {
        tiles,
        sprite_entities,
        changed_tiles: VecDeque::new(),
        free_sprites: Vec::new(),          // ← initialise empty pool
        width: w,
        height: h,
        height_map,
        color_noise: Perlin::new(rand::thread_rng().gen()),
    });
    commands.insert_resource(LastRect::default());
}

/* ===========================================================
   helpers for streaming sprites
   =========================================================== */
#[inline]
fn color_and_z(terrain: &Terrain, x: usize, y: usize) -> (Color, f32) {
    use crate::constants::{
        COLOR_NOISE_SCALE,
        COLOR_VARIATION_LEVELS,
        COLOR_VARIATION_STRENGTH,
    };

    let tile = terrain.tiles[y][x];

    let raw = terrain
        .color_noise
        .get([
            x as f64 * COLOR_NOISE_SCALE,
            y as f64 * COLOR_NOISE_SCALE,
        ]) as f32;

    let step = (((raw + 1.0) * 0.5) * COLOR_VARIATION_LEVELS as f32)
        .floor()
        .clamp(0.0, (COLOR_VARIATION_LEVELS - 1) as f32);

    let norm = step / (COLOR_VARIATION_LEVELS as f32 - 1.0) * 2.0 - 1.0;
    let factor = 1.0 + norm * COLOR_VARIATION_STRENGTH;

    let base_rgb = match tile.kind {
        TileKind::Dirt  => Vec3::new(0.55, 0.27, 0.07) * factor,
        TileKind::Stone => Vec3::new(0.50, 0.50, 0.50) * factor,
        TileKind::Air   => BACKGROUND_BROWN,
        _ => unreachable!(),
    } * brightness(&tile);

    let color = Color::srgb(
        base_rgb.x.clamp(0.0, 1.0),
        base_rgb.y.clamp(0.0, 1.0),
        base_rgb.z.clamp(0.0, 1.0),
    );
    let z = if tile.kind == TileKind::Air { -1.0 } else { 0.0 };
    (color, z)
}

#[inline]
fn ensure_sprite(commands: &mut Commands, terrain: &mut Terrain, x: i32, y: i32) {
    if x < 0 || y < 0 ||
       x >= terrain.width as i32 || y >= terrain.height as i32 {
        return;
    }

    let (ux, uy) = (x as usize, y as usize);

    if terrain.sprite_entities[uy][ux].is_some() {
        return; // already alive
    }

    if !matches!(
        terrain.tiles[uy][ux].kind,
        TileKind::Dirt | TileKind::Stone | TileKind::Air
    ) {
        return; // Sky never gets a sprite
    }

    /* -------- get a sprite, either new or from the pool ---------- */
    let (color, z) = color_and_z(terrain, ux, uy);

    let entity = if let Some(e) = terrain.free_sprites.pop() {
        // recycle
        commands.entity(e).insert((
            Visibility::Visible,
            Sprite {
                color,
                custom_size: Some(Vec2::splat(TILE_SIZE)),
                ..default()
            },
            Transform::from_xyz(
                ux as f32 * TILE_SIZE,
                tile_to_world_y(terrain.height, uy),
                z,
            ),
            TileSprite { x: ux, y: uy },
        ));
        e
    } else {
        // pool empty → spawn new
        spawn_tile(commands, terrain, ux, uy)
    };

    terrain.sprite_entities[uy][ux] = Some(entity);
}

/* ===========================================================
   stream_tiles_system – stripe differencing + pooling
   =========================================================== */
pub fn stream_tiles_system(
    mut commands: Commands,
    mut terrain: ResMut<Terrain>,
    rect: Res<ActiveRect>,
    mut last_rect: ResMut<LastRect>,
) {
    let new = *rect;

    /* ---------- early‑out: camera stayed within same tile ---------- */
    if last_rect.0 == Some(new) {
        return;
    }

    /* ---------- first frame: populate full window ----------------- */
    let Some(prev) = last_rect.0 else {
        for y in new.min_y..=new.max_y {
            for x in new.min_x..=new.max_x {
                ensure_sprite(&mut commands, &mut terrain, x, y);
            }
        }
        last_rect.0 = Some(new);
        return;
    };

    /* ---------- spawn the stripes that slid *into* view ----------- */
    // vertical stripes (new columns)
    for x in new.min_x..=new.max_x {
        if x < prev.min_x || x > prev.max_x {
            for y in new.min_y..=new.max_y {
                ensure_sprite(&mut commands, &mut terrain, x, y);
            }
        }
    }
    // horizontal stripes (new rows)
    for y in new.min_y..=new.max_y {
        if y < prev.min_y || y > prev.max_y {
            for x in new.min_x..=new.max_x {
                ensure_sprite(&mut commands, &mut terrain, x, y);
            }
        }
    }

    /* ---------- hide (pool) the stripes that slid *out* ----------- */
    // columns that left
    for x in prev.min_x..=prev.max_x {
        if x < new.min_x || x > new.max_x {
            for y in prev.min_y..=prev.max_y {
                let (ux, uy) = (x as usize, y as usize);
                if let Some(e) = terrain.sprite_entities[uy][ux] {
                    commands.entity(e).insert(Visibility::Hidden);
                    terrain.free_sprites.push(e);
                    terrain.sprite_entities[uy][ux] = None;
                }
            }
        }
    }
    // rows that left
    for y in prev.min_y..=prev.max_y {
        if y < new.min_y || y > new.max_y {
            for x in prev.min_x..=prev.max_x {
                if x >= new.min_x && x <= new.max_x {
                    let (ux, uy) = (x as usize, y as usize);
                    if let Some(e) = terrain.sprite_entities[uy][ux] {
                        commands.entity(e).insert(Visibility::Hidden);
                        terrain.free_sprites.push(e);
                        terrain.sprite_entities[uy][ux] = None;
                    }
                }
            }
        }
    }

    last_rect.0 = Some(new);
}

/* ===========================================================
   update ActiveRect (PostUpdate – camera follow already done)
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

    let pad_x =
        ((window.width()  * 0.5) / TILE_SIZE).ceil() as i32 + ACTIVE_MARGIN;
    let pad_y =
        ((window.height() * 0.5) / TILE_SIZE).ceil() as i32 + ACTIVE_MARGIN;

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
        None => commands.insert_resource(new),
        _ => {}
    }
}

/* ===========================================================
   redraw tiles whose state changed (pool‑aware)
   =========================================================== */
pub fn redraw_changed_tiles_system(
    mut commands: Commands,
    mut terrain: ResMut<Terrain>,
) {
    while let Some((x, y)) = terrain.changed_tiles.pop_front() {
        let tile = terrain.tiles[y][x];

        /* ---- tiles that become Sky → hide & pool sprite -------- */
        if tile.kind == TileKind::Sky {
            if let Some(e) = terrain.sprite_entities[y][x] {
                commands.entity(e).insert(Visibility::Hidden);
                terrain.free_sprites.push(e);
                terrain.sprite_entities[y][x] = None;
            }
            continue;
        }

        /* ---- tint & z‑layer ------------------------------------ */
        let (color, z) = color_and_z(&terrain, x, y);

        /* ---- update existing sprite or grab one from pool ------- */
        match terrain.sprite_entities[y][x] {
            Some(e) => {
                commands.entity(e).insert((
                    Visibility::Visible,
                    Sprite {
                        color,
                        custom_size: Some(Vec2::splat(TILE_SIZE)),
                        ..default()
                    },
                    Transform {
                        translation: Vec3::new(
                            x as f32 * TILE_SIZE,
                            tile_to_world_y(terrain.height, y),
                            z,
                        ),
                        ..default()
                    },
                ));
            }
            None => {
                // reuse from pool if available, else spawn
                let entity = if let Some(e) = terrain.free_sprites.pop() {
                    commands.entity(e).insert((
                        Visibility::Visible,
                        Sprite {
                            color,
                            custom_size: Some(Vec2::splat(TILE_SIZE)),
                            ..default()
                        },
                        Transform::from_xyz(
                            x as f32 * TILE_SIZE,
                            tile_to_world_y(terrain.height, y),
                            z,
                        ),
                        TileSprite { x, y },
                    ));
                    e
                } else {
                    spawn_tile(&mut commands, &terrain, x, y)
                };
                terrain.sprite_entities[y][x] = Some(entity);
            }
        }
    }
}

/* ===========================================================
   spawn a single tile sprite (new entity, visible)
   =========================================================== */
#[inline]
fn brightness(tile: &Tile) -> f32 {
    if tile.visible {
        1.0
    } else if tile.explored {
        EXPLORED_BRIGHTNESS
    } else {
        0.0
    }
}

pub fn spawn_tile(
    commands: &mut Commands,
    terrain: &Terrain,
    x: usize,
    y: usize,
) -> Entity {
    let (color, z) = color_and_z(terrain, x, y);

    commands
        .spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::splat(TILE_SIZE)),
                ..default()
            },
            Transform::from_xyz(
                x as f32 * TILE_SIZE,
                tile_to_world_y(terrain.height, y),
                z,
            ),
            TileSprite { x, y },
        ))
        .id()
}

/* ===========================================================
   digging_system – left‑click to remove terrain
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

    /* --- bounding box of the dig circle ------------------------ */
    let min_x = ((world.x - DIG_RADIUS) / TILE_SIZE).floor() as i32;
    let max_x = ((world.x + DIG_RADIUS) / TILE_SIZE).ceil()  as i32;

    let min_y_world = world.y - DIG_RADIUS;
    let max_y_world = world.y + DIG_RADIUS;
    let min_y = world_to_tile_y(terrain.height, max_y_world);
    let max_y = world_to_tile_y(terrain.height, min_y_world);

    for ty in min_y..=max_y {
        for tx in min_x..=max_x {
            if tx < 0 || ty < 0 ||
               tx >= terrain.width as i32 || ty >= terrain.height as i32 {
                continue;
            }
            /* ---- circle test ----------------------------------- */
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
   solid‑tile check for collision
   =========================================================== */
#[inline]
pub fn solid(terrain: &Terrain, tx: i32, ty: i32) -> bool {
    if tx < 0 || ty < 0 ||
       tx >= terrain.width as i32 || ty >= terrain.height as i32 {
        return true;
    }
    matches!(
        terrain.tiles[ty as usize][tx as usize].kind,
        TileKind::Dirt | TileKind::Stone
    )
}