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

/* ===========================================================
   loaded window (4×3 chunk grid)
   =========================================================== */
#[derive(Resource, Copy, Clone, Debug, PartialEq, Eq)]
pub struct LoadedWindow {
    pub origin_cx: i32, // left‑most loaded chunk column
    pub origin_cy: i32, // top‑most  loaded chunk row
}
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
   chunk helpers (groups of CHUNK_WIDTH × CHUNK_HEIGHT tiles)
   =========================================================== */
   #[inline]
   fn ensure_chunk(
       commands: &mut Commands,
       terrain:  &mut Terrain,
       cx: i32,
       cy: i32,
   ) {
       let min_x = cx * CHUNK_WIDTH  as i32;
       let max_x = ((cx + 1) * CHUNK_WIDTH  as i32 - 1).min(terrain.width  as i32 - 1);
       let min_y = cy * CHUNK_HEIGHT as i32;
       let max_y = ((cy + 1) * CHUNK_HEIGHT as i32 - 1).min(terrain.height as i32 - 1);
   
       for y in min_y..=max_y {
           for x in min_x..=max_x {
               ensure_sprite(commands, terrain, x, y);
           }
       }
   }
   
   #[inline]
   fn hide_chunk(
       commands: &mut Commands,
       terrain:  &mut Terrain,
       cx: i32,
       cy: i32,
   ) {
       let min_x = cx * CHUNK_WIDTH  as i32;
       let max_x = ((cx + 1) * CHUNK_WIDTH  as i32 - 1).min(terrain.width  as i32 - 1);
       let min_y = cy * CHUNK_HEIGHT as i32;
       let max_y = ((cy + 1) * CHUNK_HEIGHT as i32 - 1).min(terrain.height as i32 - 1);
   
       for y in min_y..=max_y {
           for x in min_x..=max_x {
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

/* ===========================================================
   stream_tiles_system – stripe differencing + pooling
   =========================================================== */
pub fn stream_tiles_system(
    mut commands: Commands,
    mut terrain: ResMut<Terrain>,
    loaded: Res<LoadedWindow>,
) {
    /* -----------------------------------------------------------
   chunk‑level differencing
    ----------------------------------------------------------- */
    let new_min_cx = loaded.origin_cx;
    let new_max_cx = loaded.origin_cx + LOADED_CHUNK_COLS - 1;
    let new_min_cy = loaded.origin_cy;
    let new_max_cy = loaded.origin_cy + LOADED_CHUNK_ROWS - 1;

    #[derive(Copy, Clone, PartialEq)]
    struct ChunkRect { min_cx: i32, max_cx: i32, min_cy: i32, max_cy: i32 }
    static mut PREV: Option<ChunkRect> = None;

    let new_rect = ChunkRect { min_cx: new_min_cx, max_cx: new_max_cx,
                            min_cy: new_min_cy, max_cy: new_max_cy };

    let prev = unsafe { PREV };

    if prev.is_none() {
        // first frame: fill everything
        for cy in new_min_cy..=new_max_cy {
            for cx in new_min_cx..=new_max_cx {
                ensure_chunk(&mut commands, &mut terrain, cx, cy);
            }
        }
        unsafe { PREV = Some(new_rect) };
        return;
    }

    let prev = prev.unwrap();
    if prev == new_rect {
        return;     // camera still inside same chunk window
    }

    /* ---------- entering chunks ---------- */
    for cx in new_min_cx..=new_max_cx {
        if cx < prev.min_cx || cx > prev.max_cx {
            for cy in new_min_cy..=new_max_cy {
                ensure_chunk(&mut commands, &mut terrain, cx, cy);
            }
        }
    }
    for cy in new_min_cy..=new_max_cy {
        if cy < prev.min_cy || cy > prev.max_cy {
            for cx in new_min_cx..=new_max_cx {
                ensure_chunk(&mut commands, &mut terrain, cx, cy);
            }
        }
    }

    /* ---------- leaving chunks ----------- */
    for cx in prev.min_cx..=prev.max_cx {
        if cx < new_min_cx || cx > new_max_cx {
            for cy in prev.min_cy..=prev.max_cy {
                hide_chunk(&mut commands, &mut terrain, cx, cy);
            }
        }
    }
    for cy in prev.min_cy..=prev.max_cy {
        if cy < new_min_cy || cy > new_max_cy {
            for cx in prev.min_cx..=prev.max_cx {
                if cx >= new_min_cx && cx <= new_max_cx {
                    hide_chunk(&mut commands, &mut terrain, cx, cy);
                }
            }
        }
    }

    unsafe { PREV = Some(new_rect) };
}

/* ===========================================================
   shift_loaded_window_system
   – keeps a 4×3 chunk window centred on the player and
     moves it whenever they step into an edge chunk
   =========================================================== */
pub fn shift_loaded_window_system(
    cam_q: Query<&Transform, With<Camera>>,
    terrain: Res<Terrain>,
    mut window_res: Option<ResMut<LoadedWindow>>,
    mut commands: Commands,
) {
    let cam_tf = match cam_q.get_single() {
        Ok(t) => t,
        Err(_) => return,
    };

    // Player position in chunk space
    let px = (cam_tf.translation.x / TILE_SIZE).round() as i32;
    let py = world_to_tile_y(terrain.height, cam_tf.translation.y);
    let player_cx = px / CHUNK_WIDTH as i32;
    let player_cy = py / CHUNK_HEIGHT as i32;

    match window_res {
        Some(mut win) => {
            let mut moved = false;

            // Re‑position the loaded‑chunk window in a single step so the player
            // is guaranteed to be inside it even if they crossed multiple chunks
            // in one frame (e.g. during fast falls or dashes).
 
            let max_cx = (terrain.width as i32 / CHUNK_WIDTH as i32) - LOADED_CHUNK_COLS;
            let max_cy = (terrain.height as i32 / CHUNK_HEIGHT as i32) - LOADED_CHUNK_ROWS;
 
            let new_origin_cx = player_cx
                .saturating_sub(LOADED_CHUNK_COLS / 2)
                .clamp(0, max_cx);
            let new_origin_cy = player_cy
                .saturating_sub(LOADED_CHUNK_ROWS / 2)
                .clamp(0, max_cy);
 
            if new_origin_cx != win.origin_cx || new_origin_cy != win.origin_cy {
                win.origin_cx = new_origin_cx;
                win.origin_cy = new_origin_cy;
                moved = true;
            }

            if moved {
                // mark as changed so dependent systems run
                *win = *win;
            }
        }
        None => {
            // first run – centre the window on the player's chunk
            let origin_cx = player_cx - LOADED_CHUNK_COLS / 2;
            let origin_cy = player_cy - LOADED_CHUNK_ROWS / 2;
            commands.insert_resource(LoadedWindow { origin_cx, origin_cy });
        }
    }
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