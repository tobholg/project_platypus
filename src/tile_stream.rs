//! run‑time terrain streaming, digging, collision & helpers
//!
//! All code that *updates* and *renders* the already‑generated
//! tiles lives here.  Generation itself is in `world_gen.rs`.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::window::Window;
use noise::NoiseFn;

use crate::components::*;
use crate::constants::*;
use crate::world_gen::{
    tile_to_world_y, world_to_tile_y, ActiveRect, LastRect, Terrain, Tile, TileKind,
    EXPLORED_BRIGHTNESS,
};

/* ===========================================================
   helpers for streaming sprites
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

#[inline]
fn color_and_z(terrain: &Terrain, x: usize, y: usize) -> (Color, f32) {
    let tile     = terrain.tiles[y][x];
    let base_rgb = tile.base_rgb * brightness(&tile);

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
    if x < 0
        || y < 0
        || x >= terrain.width as i32
        || y >= terrain.height as i32
    {
        return;
    }
    let (ux, uy) = (x as usize, y as usize);
    let idx      = terrain.idx(ux, uy);
    if terrain.sprite_entities[idx].is_some() {
        return;
    }
    if !matches!(
        terrain.tiles[uy][ux].kind,
        TileKind::Grass
            | TileKind::Dirt
            | TileKind::Stone
            | TileKind::Obsidian
            | TileKind::Snow
            | TileKind::Air
    ) {
        return; // Sky never gets a sprite
    }

    let (color, z) = color_and_z(terrain, ux, uy);

    let entity = if let Some(e) = terrain.free_sprites.pop() {
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
        spawn_tile(commands, terrain, ux, uy)
    };
    terrain.sprite_entities[idx] = Some(entity);
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
    if last_rect.0 == Some(new) {
        return;
    }

    /* initial fill ------------------------------------------------------- */
    let Some(prev) = last_rect.0 else {
        for y in new.min_y..=new.max_y {
            for x in new.min_x..=new.max_x {
                ensure_sprite(&mut commands, &mut terrain, x, y);
            }
        }
        last_rect.0 = Some(new);
        return;
    };

    /* stripes entering view ---------------------------------------------- */
    for x in new.min_x..=new.max_x {
        if x < prev.min_x || x > prev.max_x {
            for y in new.min_y..=new.max_y {
                ensure_sprite(&mut commands, &mut terrain, x, y);
            }
        }
    }
    for y in new.min_y..=new.max_y {
        if y < prev.min_y || y > prev.max_y {
            for x in new.min_x..=new.max_x {
                ensure_sprite(&mut commands, &mut terrain, x, y);
            }
        }
    }

    /* stripes leaving view (re‑pool) ------------------------------------- */
    for x in prev.min_x..=prev.max_x {
        if x < new.min_x || x > new.max_x {
            for y in prev.min_y..=prev.max_y {
                let (ux, uy) = (x as usize, y as usize);
                let idx      = terrain.idx(ux, uy);
                if let Some(e) = terrain.sprite_entities[idx] {
                    commands.entity(e).insert(Visibility::Hidden);
                    terrain.free_sprites.push(e);
                    terrain.sprite_entities[idx] = None;
                }
            }
        }
    }
    for y in prev.min_y..=prev.max_y {
        if y < new.min_y || y > new.max_y {
            for x in prev.min_x..=prev.max_x {
                if x >= new.min_x && x <= new.max_x {
                    let (ux, uy) = (x as usize, y as usize);
                    let idx      = terrain.idx(ux, uy);
                    if let Some(e) = terrain.sprite_entities[idx] {
                        commands.entity(e).insert(Visibility::Hidden);
                        terrain.free_sprites.push(e);
                        terrain.sprite_entities[idx] = None;
                    }
                }
            }
        }
    }

    last_rect.0 = Some(new);
}

/* ===========================================================
   update_active_rect_system
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
        ((window.width() * 0.5) / TILE_SIZE).ceil() as i32 + ACTIVE_MARGIN;
    let pad_y =
        ((window.height() * 0.5) / TILE_SIZE).ceil() as i32 + ACTIVE_MARGIN;

    let px = (cam_tf.translation.x / TILE_SIZE).round() as i32;
    let py = world_to_tile_y(terrain.height, cam_tf.translation.y);

    let new = ActiveRect {
        min_x: (px - pad_x).clamp(0, terrain.width as i32 - 1),
        max_x: (px + pad_x).clamp(0, terrain.width as i32 - 1),
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
   redraw_changed_tiles_system
   =========================================================== */
pub fn redraw_changed_tiles_system(
    mut commands: Commands,
    mut terrain: ResMut<Terrain>,
) {
    use crate::constants::{
        COLOR_NOISE_SCALE, COLOR_VARIATION_LEVELS, COLOR_VARIATION_STRENGTH,
    };

    let mut spawns:  Vec<(Sprite, Transform, TileSprite)> =
        Vec::new();
    let mut inserts: Vec<(Entity, (Visibility, Sprite, Transform, TileSprite))> =
        Vec::new();

    // drain the entire queue once to reduce the number of atomic/pointer operations
    let changed: Vec<(usize, usize)> = terrain.changed_tiles.drain(..).collect();
    for (x, y) in changed {
        let idx_sprite = terrain.idx(x, y);
        let kind       = terrain.tiles[y][x].kind;

        /* SKY → just hide / recycle */
        if kind == TileKind::Sky {
            if let Some(e) = terrain.sprite_entities[idx_sprite] {
                commands.entity(e).insert(Visibility::Hidden);
                terrain.free_sprites.push(e);
                terrain.sprite_entities[idx_sprite] = None;
            }
            continue;
        }

        /* re‑tint --------------------------------------------------------- */
        let raw = terrain.color_noise.get([
            x as f64 * COLOR_NOISE_SCALE,
            y as f64 * COLOR_NOISE_SCALE,
        ]) as f32;

        let step = (((raw + 1.0) * 0.5) * COLOR_VARIATION_LEVELS as f32)
            .floor()
            .clamp(0.0, (COLOR_VARIATION_LEVELS - 1) as f32);
        let norm   = step / (COLOR_VARIATION_LEVELS as f32 - 1.0) * 2.0 - 1.0;
        let factor = 1.0 + norm * COLOR_VARIATION_STRENGTH;

        terrain.tiles[y][x].base_rgb = match kind {
            TileKind::Grass    => Vec3::new(0.13, 0.70, 0.08) * factor,
            TileKind::Snow     => Vec3::new(0.95, 0.95, 0.95) * factor,
            TileKind::Dirt     => Vec3::new(0.55, 0.27, 0.07) * factor,
            TileKind::Stone    => Vec3::new(0.50, 0.50, 0.50) * factor,
            TileKind::Obsidian => Vec3::new(0.20, 0.05, 0.35) * factor,
            TileKind::Air      => Vec3::new(0.20, 0.10, 0.05) * factor,
            _                  => terrain.tiles[y][x].base_rgb,
        };

        /* colour & depth -------------------------------------------------- */
        let (color, z) = color_and_z(&terrain, x, y);
        let tile_sprite = TileSprite { x, y };

        match terrain.sprite_entities[idx_sprite] {
            Some(entity) => {
                let transform = Transform {
                    translation: Vec3::new(
                        x as f32 * TILE_SIZE,
                        tile_to_world_y(terrain.height, y),
                        z,
                    ),
                    ..default()
                };
                let sprite = Sprite {
                    color,
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                };
                inserts.push((
                    entity,
                    (Visibility::Visible, sprite, transform, tile_sprite),
                ));
            }
            None => {
                let transform = Transform {
                    translation: Vec3::new(
                        x as f32 * TILE_SIZE,
                        tile_to_world_y(terrain.height, y),
                        z,
                    ),
                    ..default()
                };
                let sprite = Sprite {
                    color,
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                };

                if let Some(entity) = terrain.free_sprites.pop() {
                    inserts.push((
                        entity,
                        (Visibility::Visible, sprite, transform, tile_sprite),
                    ));
                    terrain.sprite_entities[idx_sprite] = Some(entity);
                } else {
                    spawns.push((sprite, transform, tile_sprite));
                }
            }
        }
    }

    /* flush command buffers ---------------------------------------------- */
    if !spawns.is_empty() {
        commands.spawn_batch(spawns);
    }
    if !inserts.is_empty() {
        commands.insert_or_spawn_batch(inserts);
    }
}

/* ===========================================================
   spawn_tile helper
   =========================================================== */
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
   digging_system (mouse circular dig)
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
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let (cam, cam_tf) = cam_q.single();
    let Ok(world) = cam.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };

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
                    TileKind::Grass
                        | TileKind::Dirt
                        | TileKind::Stone
                        | TileKind::Obsidian
                        | TileKind::Snow
                ) {
                    terrain.tiles[uy][ux].kind = TileKind::Air;
                    terrain.changed_tiles.push_back((ux, uy));
                }
            }
        }
    }
}

/* ===========================================================
   solid collision check
   =========================================================== */
#[inline]
pub fn solid(terrain: &Terrain, tx: i32, ty: i32) -> bool {
    if tx < 0
        || ty < 0
        || tx >= terrain.width as i32
        || ty >= terrain.height as i32
    {
        return true;
    }
    matches!(
        terrain.tiles[ty as usize][tx as usize].kind,
        TileKind::Grass
            | TileKind::Dirt
            | TileKind::Stone
            | TileKind::Obsidian
            | TileKind::Snow
    )
}

/* ===========================================================
   sync_tile_sprite_entities_system
   – writes freshly spawned TileSprite IDs back into the grid
   =========================================================== */
pub fn sync_tile_sprite_entities_system(
    mut terrain: ResMut<Terrain>,
    q: Query<(Entity, &TileSprite), Added<TileSprite>>,
) {
    for (entity, tile) in &q {
        if tile.y < terrain.height && tile.x < terrain.width {
            let idx = terrain.idx(tile.x, tile.y);
            terrain.sprite_entities[idx] = Some(entity);
        }
    }
}