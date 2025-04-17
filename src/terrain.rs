//! world‑generation, digging & tile‑sprite helpers
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::Rng;
use std::collections::VecDeque;

use crate::constants::*;
use crate::components::{AnimationIndices, AnimationTimer, Player, TileSprite, Velocity};

/// helper conversions (row‑0 = top)
pub fn tile_to_world_y(terrain_h: usize, tile_y: usize) -> f32 {
    (terrain_h as f32 - 1.0 - tile_y as f32) * TILE_SIZE
}
pub fn world_to_tile_y(terrain_h: usize, world_y: f32) -> i32 {
    (terrain_h as f32 - 1.0 - (world_y / TILE_SIZE).floor()) as i32
}

/// -------- tiles --------
#[derive(Clone, Copy, PartialEq)]
pub enum TileKind {
    Air,
    Sky,
    Dirt,
    Stone,
}

#[derive(Clone, Copy)]
pub struct Tile {
    pub kind: TileKind,
}

/// -------- resource --------
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

/* ---------- startup ---------- */

/// generate terrain, player & add Terrain resource
pub fn generate_world_and_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlases: ResMut<Assets<TextureAtlas>>,
) {
    /* -------- player sprite sheet -------- */
    let sheet_handle = asset_server.load("textures/player_sheet.png");
    let atlas_handle =
        atlases.add(TextureAtlas::from_grid(sheet_handle, Vec2::new(100., 100.), 6, 1, None, None));

    /* -------- world dimensions -------- */
    let w = CHUNK_WIDTH * NUM_CHUNKS_X;
    let h = CHUNK_HEIGHT * NUM_CHUNKS_Y;

    /* -------- smoother height‑map with rare cliffs -------- */
    let mut height_map = vec![0usize; w];

    let noise_hills = Perlin::new(rand::thread_rng().gen());
    let noise_cliffs = Perlin::new(rand::thread_rng().gen());

    let base = h as f32 * 0.35;
    let amp_low: f32 = 5.0;   // gentle dips
    let amp_high: f32 = 12.0; // gentle peaks

    let cliff_freq: f64 = 0.12;
    let cliff_threshold: f64 = 0.85; // rarer → closer to 1.0
    let cliff_strength: f32 = 18.0;  // how tall the “yanked” cliffs can be

    for x in 0..w {
        /* rolling hills */
        let n = noise_hills.get([x as f64 * 0.01, 0.0]);
        let mut elev = if n >= 0.0 {
            base - n as f32 * amp_high
        } else {
            base - n as f32 * amp_low
        };

        /* sprinkle rare dramatic cliffs / mountains */
        let cliff_sample = noise_cliffs.get([x as f64 * cliff_freq, 100.0]);
        if cliff_sample.abs() > cliff_threshold {
            elev -= cliff_sample.signum() as f32 * cliff_strength;
        }

        height_map[x] = elev.clamp(4.0, (h - 10) as f32) as usize;
    }

    /* -------- tile grid with *rarer* caverns -------- */
    let mut tiles = vec![vec![Tile { kind: TileKind::Air }; w]; h];
    let sprite_entities = vec![vec![None; w]; h];

    let noise_cave = Perlin::new(rand::thread_rng().gen());
    let mut rng = rand::thread_rng();

    for x in 0..w {
        let surface = height_map[x];

        /* sky */
        for y in 0..surface {
            tiles[y][x].kind = TileKind::Sky;
        }

        /* dirt / stone / occasional air pocket */
        for y in surface..h {
            let depth = y - surface; // depth *below* surface
            let n = noise_cave.get([x as f64 * 0.08, y as f64 * 0.08]);

            tiles[y][x].kind = if n > 0.40 {
                // ⬆ threshold raised (was 0.25) → fewer caverns
                TileKind::Air
            } else if depth > h / 4 {
                TileKind::Stone
            } else {
                TileKind::Dirt
            };
        }
    }

    /* connect a few surface cave mouths so early game isn’t boring */
    for _ in 0..((w as f32 / 120.0) as usize) {
        let ex = rng.gen_range(4..w - 4);
        let surf = height_map[ex];

        for dy in 0..12 {
            let ty = surf + dy;
            if ty >= h {
                break;
            }
            for dx in -3..=3 {
                tiles[ty][(ex as isize + dx) as usize].kind = TileKind::Air;
            }
        }
    }

    /* -------- player spawn (mid‑map, on ground) -------- */
    let spawn_x = w / 2;
    let surf_row = height_map[spawn_x];

    let spawn = Vec2::new(
        spawn_x as f32 * TILE_SIZE,
        tile_to_world_y(h, surf_row) + TILE_SIZE * 0.5 + PLAYER_HEIGHT * 0.5,
    );

    commands.spawn((
        SpriteSheetBundle {
            texture_atlas: atlas_handle,
            sprite: TextureAtlasSprite {
                index: 0,
                ..default()
            },
            transform: Transform {
                translation: spawn.extend(10.0),
                scale: Vec3::splat(1.8),
                ..default()
            },
            ..default()
        },
        Player { grounded: false },
        Velocity(Vec2::ZERO),
        AnimationIndices { first: 0, last: 5 },
        AnimationTimer(Timer::from_seconds(0.12, TimerMode::Repeating)),
    ));

    /* -------- insert Terrain resource -------- */
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

/// initial bulk sprite spawn (runs exactly once)
pub fn spawn_initial_tiles(
    mut commands: Commands,
    mut terrain: ResMut<Terrain>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    for y in 0..terrain.height {
        for x in 0..terrain.width {
            if matches!(
                terrain.tiles[y][x].kind,
                TileKind::Dirt | TileKind::Stone
            ) {
                terrain.sprite_entities[y][x] =
                    Some(spawn_tile(&mut commands, &terrain, x, y));
            }
        }
    }
    *done = true;
}

/* ---------- helpers ---------- */

/// single tile sprite with quantised colour variation
pub fn spawn_tile(commands: &mut Commands, terrain: &Terrain, x: usize, y: usize) -> Entity {
    use crate::constants::{COLOR_NOISE_SCALE, COLOR_VARIATION_LEVELS, COLOR_VARIATION_STRENGTH};
    let raw = terrain
        .color_noise
        .get([x as f64 * COLOR_NOISE_SCALE, y as f64 * COLOR_NOISE_SCALE])
        as f32;

    /* bucket‑based colour banding (for pixel‑arty look) */
    let step = (((raw + 1.0) * 0.5) * COLOR_VARIATION_LEVELS as f32)
        .floor()
        .clamp(0.0, (COLOR_VARIATION_LEVELS - 1) as f32);

    let norm = step / (COLOR_VARIATION_LEVELS as f32 - 1.0) * 2.0 - 1.0;
    let factor = 1.0 + norm * COLOR_VARIATION_STRENGTH;

    let base = match terrain.tiles[y][x].kind {
        TileKind::Dirt => Vec3::new(0.55, 0.27, 0.07),
        TileKind::Stone => Vec3::new(0.50, 0.50, 0.50),
        _ => unreachable!(),
    } * factor;

    commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(base.x.clamp(0.0, 1.0), base.y.clamp(0.0, 1.0), base.z.clamp(0.0, 1.0)),
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

/// tidy‑up & redraw tiles whose kind changed
pub fn redraw_changed_tiles_system(mut commands: Commands, mut terrain: ResMut<Terrain>) {
    while let Some((x, y)) = terrain.changed_tiles.pop_front() {
        if let Some(e) = terrain.sprite_entities[y][x] {
            commands.entity(e).despawn();
            terrain.sprite_entities[y][x] = None;
        }
        if matches!(
            terrain.tiles[y][x].kind,
            TileKind::Dirt | TileKind::Stone
        ) {
            terrain.sprite_entities[y][x] =
                Some(spawn_tile(&mut commands, &terrain, x, y));
        }
    }
}

/// “Minecraft‑style” digging with the mouse
pub fn digging_system(
    mouse: Res<Input<MouseButton>>,
    windows: Query<&Window>,
    cam_q: Query<(&Camera, &GlobalTransform)>,
    mut terrain: ResMut<Terrain>,
) {
    if !mouse.pressed(MouseButton::Left) {
        return;
    }
    let window = windows.single();
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let (cam, cam_tf) = cam_q.single();

    /* screen‑space → world */
    let ndc = (cursor / Vec2::new(window.width(), window.height())) * 2.0 - Vec2::ONE;
    let world =
        (cam_tf.compute_matrix() * cam.projection_matrix().inverse() * ndc.extend(-1.0).extend(1.0))
            .truncate();

    /* bounding square around brush */
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
            let dy = tile_to_world_y(terrain.height, ty as usize) - world.y;
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

/// shortcut used by physics
pub fn solid(terrain: &Terrain, tx: i32, ty: i32) -> bool {
    if tx < 0 || ty < 0 || tx >= terrain.width as i32 || ty >= terrain.height as i32 {
        return true; /* out‑of‑bounds treated as solid */
    }
    matches!(
        terrain.tiles[ty as usize][tx as usize].kind,
        TileKind::Dirt | TileKind::Stone
    )
}