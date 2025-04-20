//! field‑of‑view & lighting (shadow‑casting) – radius‑bounded version

use bevy::prelude::*;
use std::collections::HashSet;

use crate::components::Player;
use crate::constants::TILE_SIZE;
use crate::terrain::{world_to_tile_y, ActiveRect, Terrain, TileKind};

/* ===========================================================
   Player‑tile resource
   =========================================================== */
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerTile {
    pub x: i32,
    pub y: i32,
}

/* ===========================================================
   Visible‑tiles resource
   =========================================================== */
#[derive(Resource, Default)]
pub struct VisibleTiles {
    pub set: HashSet<(usize, usize)>,
    scratch: HashSet<(usize, usize)>,
}

/* ===========================================================
   Tunables
   =========================================================== */
pub const FOV_RADIUS: i32 = 48;            // ← was 32
pub const LIGHT_BLEED_RADIUS: i32 = 1;
pub const ALWAYS_VISIBLE_DEPTH: usize = 4;

/* ===========================================================
   startup
   =========================================================== */
pub fn startup_fov_system(
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    terrain: Res<Terrain>,
) {
    let tf = player_q.single();
    let tx = (tf.translation.x / TILE_SIZE).floor() as i32;
    let ty = world_to_tile_y(terrain.height, tf.translation.y);

    commands.insert_resource(PlayerTile { x: tx, y: ty });
    commands.insert_resource(VisibleTiles::default());
}

/* ===========================================================
   track player tile – update only when the tile *changes*
   =========================================================== */
pub fn detect_player_tile_change_system(
    mut player_tile: ResMut<PlayerTile>,
    player_q: Query<&Transform, With<Player>>,
    terrain: Res<Terrain>,
) {
    let Ok(tf) = player_q.get_single() else { return };

    let nx = (tf.translation.x / TILE_SIZE).floor() as i32;
    let ny = world_to_tile_y(terrain.height, tf.translation.y);

    if player_tile.x == nx && player_tile.y == ny {
        return;
    }

    player_tile.x = nx;
    player_tile.y = ny;
}

/* ===========================================================
   recompute FOV – runs only when `PlayerTile` changed
   =========================================================== */
pub fn recompute_fov_system(
    mut terrain: ResMut<Terrain>,
    player_tile: Res<PlayerTile>,
    rect: Res<ActiveRect>,
    mut vis: ResMut<VisibleTiles>,
) {
    if !player_tile.is_changed() {
        return;
    }

    let (w, h) = (terrain.width as i32, terrain.height as i32);
    let (px, py) = (player_tile.x, player_tile.y);

    /* ---------- fresh visible set ---------- */
    let mut new_visible = std::mem::take(&mut vis.scratch);

    /* 8‑way shadow‑casting ---------------------------------------------- */
    const OCT: [(i32, i32, i32, i32); 8] = [
        (1, 0, 0, 1),
        (0, 1, 1, 0),
        (0, -1, 1, 0),
        (-1, 0, 0, 1),
        (-1, 0, 0, -1),
        (0, -1, -1, 0),
        (0, 1, -1, 0),
        (1, 0, 0, -1),
    ];
    for &(xx, xy, yx, yy) in &OCT {
        cast_light(
            &terrain,
            px,
            py,
            1,
            1.0,
            0.0,
            FOV_RADIUS,
            xx,
            xy,
            yx,
            yy,
            &mut new_visible,
        );
    }
    if (0..w).contains(&px) && (0..h).contains(&py) {
        new_visible.insert((px as usize, py as usize));
    }

    /* halo bleed --------------------------------------------------------- */
    if LIGHT_BLEED_RADIUS > 0 {
        let mut extra = Vec::<(usize, usize)>::new();
        for &(x, y) in &new_visible {
            for by in -LIGHT_BLEED_RADIUS..=LIGHT_BLEED_RADIUS {
                for bx in -LIGHT_BLEED_RADIUS..=LIGHT_BLEED_RADIUS {
                    let nx = x as i32 + bx;
                    let ny = y as i32 + by;
                    if (0..w).contains(&nx) && (0..h).contains(&ny) {
                        extra.push((nx as usize, ny as usize));
                    }
                }
            }
        }
        new_visible.extend(extra);
    }

    /* surface band – across whole active rect --------------------------- */
    for x in rect.min_x.max(0) as usize..=rect.max_x.min(w - 1) as usize {
        let ground = terrain.height_map[x];
        let max_y = (ground + ALWAYS_VISIBLE_DEPTH).min(h as usize - 1);
        for y in 0..=max_y {
            new_visible.insert((x, y));
        }
    }

    /* ---------------- diff old ↔ new ----------------------------------- */
    for &(ux, uy) in vis.set.difference(&new_visible) {
        terrain.tiles[uy][ux].visible = false;
        terrain.changed_tiles.push_back((ux, uy));
    }
    for &(ux, uy) in new_visible.difference(&vis.set) {
        let tile = &mut terrain.tiles[uy][ux];
        tile.visible = true;
        tile.explored = true;
        terrain.changed_tiles.push_back((ux, uy));
    }

    vis.set = new_visible;
    vis.scratch.clear();
}

/* ===========================================================
   recursive shadow‑casting
   =========================================================== */
fn cast_light(
    terrain: &Terrain,
    cx: i32,
    cy: i32,
    row: i32,
    mut start_slope: f32,
    end_slope: f32,
    radius: i32,
    xx: i32,
    xy: i32,
    yx: i32,
    yy: i32,
    out: &mut HashSet<(usize, usize)>,
) {
    if start_slope < end_slope {
        return;
    }
    let (w, h) = (terrain.width as i32, terrain.height as i32);
    let radius_sq = radius * radius;

    let mut blocked = false;
    let mut new_start = 0.0;

    for dist in row..=radius {
        let mut dx = -dist;
        let mut dy = -dist;

        while dx <= 0 {
            let l_slope = (dx as f32 - 0.5) / (dy as f32 + 0.5);
            let r_slope = (dx as f32 + 0.5) / (dy as f32 - 0.5);

            if r_slope > start_slope {
                dx += 1;
                continue;
            }
            if l_slope < end_slope {
                break;
            }

            let tx = cx + dx * xx + dy * xy;
            let ty = cy + dx * yx + dy * yy;

            if (0..w).contains(&tx) && (0..h).contains(&ty) {
                if dx * dx + dy * dy <= radius_sq {
                    out.insert((tx as usize, ty as usize));
                }

                let opaque = matches!(
                    terrain.tiles[ty as usize][tx as usize].kind,
                    TileKind::Dirt | TileKind::Stone | TileKind::Obsidian | TileKind::Grass | TileKind::Snow
                );

                if blocked {
                    if opaque {
                        new_start = r_slope;
                    } else {
                        blocked = false;
                        start_slope = new_start;
                    }
                } else if opaque {
                    blocked = true;
                    new_start = r_slope;
                    cast_light(
                        terrain,
                        cx,
                        cy,
                        dist + 1,
                        start_slope,
                        l_slope,
                        radius,
                        xx,
                        xy,
                        yx,
                        yy,
                        out,
                    );
                }
            }
            dx += 1;
        }
        if blocked {
            break;
        }
    }
}