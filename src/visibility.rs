//! Field‑of‑view + visibility systems (fast shadow‑casting, wall‑stop bug fixed)

use bevy::prelude::*;
use std::collections::HashSet;

use crate::components::Player;
use crate::constants::TILE_SIZE;
use crate::terrain::{world_to_tile_y, Terrain, TileKind};

/* ===========================================================
   Player‑tile resource (updated every frame)
   =========================================================== */
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTile {
    pub x: i32,
    pub y: i32,
}

/* ===========================================================
   Visible‑tiles resource (current frame) with scratch space
   =========================================================== */
#[derive(Resource)]
pub struct VisibleTiles {
    pub set: HashSet<(usize, usize)>,
    scratch: HashSet<(usize, usize)>,
}

impl Default for VisibleTiles {
    fn default() -> Self {
        // R = 32 → πR² ≃ 3300 tiles … 4096 for round power‑of‑two
        let cap = 4096;
        Self {
            set: HashSet::with_capacity(cap),
            scratch: HashSet::with_capacity(cap),
        }
    }
}

/* ===========================================================
   Tunables
   =========================================================== */
pub const FOV_RADIUS: i32 = 48;            // vision range underground
pub const LIGHT_BLEED_RADIUS: i32 = 1;     // soft edge
pub const ALWAYS_VISIBLE_DEPTH: usize = 2; // rows below surface kept lit

/* ===========================================================
   Startup – initialise PlayerTile & VisibleTiles
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
   Track when the player steps onto a new tile
   =========================================================== */
pub fn detect_player_tile_change_system(
    mut player_tile: Option<ResMut<PlayerTile>>,
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    terrain: Res<Terrain>,
) {
    let Ok(tf) = player_q.get_single() else { return };
    let nx = (tf.translation.x / TILE_SIZE).floor() as i32;
    let ny = world_to_tile_y(terrain.height, tf.translation.y);

    if let Some(mut pt) = player_tile {
        if pt.x != nx || pt.y != ny {
            pt.x = nx;
            pt.y = ny;
        }
    } else {
        commands.insert_resource(PlayerTile { x: nx, y: ny });
    }
}

/* ===========================================================
   Recompute FOV when PlayerTile changes
   =========================================================== */
pub fn recompute_fov_system(
    mut terrain: ResMut<Terrain>,
    player_tile: Res<PlayerTile>,
    mut vis_set: ResMut<VisibleTiles>,
) {
    if !player_tile.is_changed() {
        return; // player is still on the same tile
    }

    let (w, h) = (terrain.width as i32, terrain.height as i32);
    let (px, py) = (player_tile.x, player_tile.y);

    /* ---------- phase 0: build NEW visible set ---------- */
    let mut new_visible = std::mem::take(&mut vis_set.scratch); // empty scratch
    debug_assert!(new_visible.is_empty());

    /* ---- cast into the 8 octants ---- */
    const OCT: [(i32, i32, i32, i32); 8] = [
        ( 1,  0,  0,  1), // E‑SE
        ( 0,  1,  1,  0), // SE‑S
        ( 0, -1,  1,  0), // SW‑S
        (-1,  0,  0,  1), // W‑SW
        (-1,  0,  0, -1), // W‑NW
        ( 0, -1, -1,  0), // NW‑N
        ( 0,  1, -1,  0), // NE‑N
        ( 1,  0,  0, -1), // E‑NE
    ];
    for &(xx, xy, yx, yy) in &OCT {
        cast_light(
            &terrain,
            px,
            py,
            1,          // start at row 1 (adjacent tiles)
            1.0,        // start slope
            0.0,        // end slope
            FOV_RADIUS,
            xx, xy, yx, yy,
            &mut new_visible,
        );
    }

    /* ---- always include the player’s own tile ---- */
    if px >= 0 && py >= 0 && px < w && py < h {
        new_visible.insert((px as usize, py as usize));
    }

    /* ---- halo bleed (soft edges) ---- */
    if LIGHT_BLEED_RADIUS > 0 {
        let mut extra = Vec::<(usize, usize)>::new();
        for &(x, y) in &new_visible {
            for by in -LIGHT_BLEED_RADIUS..=LIGHT_BLEED_RADIUS {
                for bx in -LIGHT_BLEED_RADIUS..=LIGHT_BLEED_RADIUS {
                    let nx = x as i32 + bx;
                    let ny = y as i32 + by;
                    if nx >= 0 && ny >= 0 && nx < w && ny < h {
                        extra.push((nx as usize, ny as usize));
                    }
                }
            }
        }
        new_visible.extend(extra);
    }

    /* ---- surface band is always visible ---- */
    for x in 0..w as usize {
        let ground = terrain.height_map[x];
        let max_y = (ground + ALWAYS_VISIBLE_DEPTH).min(h as usize - 1);
        for y in 0..=max_y {
            new_visible.insert((x, y));
        }
    }

    /* ---------- phase 1: diff old ↔ new ---------- */
    for &(ux, uy) in vis_set.set.difference(&new_visible) {
        terrain.tiles[uy][ux].visible = false;
        terrain.changed_tiles.push_back((ux, uy));
    }
    for &(ux, uy) in new_visible.difference(&vis_set.set) {
        let tile = &mut terrain.tiles[uy][ux];
        tile.visible = true;
        tile.explored = true;
        terrain.changed_tiles.push_back((ux, uy));
    }

    /* ---------- phase 2: store for next frame ---------- */
    vis_set.set = new_visible;
    vis_set.scratch.clear();          // reclaim buffer
}

/* ===========================================================
   Symmetrical recursive shadow‑casting  (wall‑blocking correct)
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

            if tx >= 0 && ty >= 0 && tx < w && ty < h {
                // distance check
                if dx * dx + dy * dy <= radius_sq {
                    out.insert((tx as usize, ty as usize));
                }

                let opaque = matches!(
                    terrain.tiles[ty as usize][tx as usize].kind,
                    TileKind::Dirt | TileKind::Stone
                );

                if blocked {
                    if opaque {
                        // still in shadow, update leading edge
                        new_start = r_slope;
                    } else {
                        // stepped out of shadow → recurse for the lit gap
                        blocked = false;
                        start_slope = new_start;
                    }
                } else if opaque {
                    // hit an opaque tile – recurse for area beyond it,
                    // then mark the remainder of this row as shadowed
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
            // the rest of the octant is in shadow
            break;
        }
    }
}