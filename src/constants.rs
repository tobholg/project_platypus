use bevy::prelude::*;
use std::ops::Range;

/* ===========================================================
   WORLD SIZE — crank it up!
   =========================================================== */
/// size of one square tile, in world units
pub const TILE_SIZE: f32 = 8.0;
pub const RENDER_CHUNK: usize = 32;

/// single‑chunk dimensions (tiles)
pub const CHUNK_WIDTH:  usize = 160;   // ↑ from 120
pub const CHUNK_HEIGHT: usize = 120;   // ↑ from  90

/// number of chunks (world dimensions)
pub const NUM_CHUNKS_X: usize = 32;    // width  = 10 240 tiles
pub const NUM_CHUNKS_Y: usize = 8;    // height =  1 920 tiles

/* ===========================================================
   ACTIVE WINDOW (dynamic)
   =========================================================== */
/// overscan tiles beyond the viewport that stay alive
pub const ACTIVE_MARGIN: i32 = 10;

/* ===========================================================
   player physics and movement
   =========================================================== */
pub const PLAYER_WIDTH:  f32 = TILE_SIZE;
pub const PLAYER_HEIGHT: f32 = 18.0;
pub const GRAVITY:       f32 = -500.0;
pub const JUMP_SPEED:    f32 =  250.0;
pub const JET_ACCEL:     f32 = 1000.0;
pub const WALK_SPEED:    f32 =  200.0;
pub const COLLISION_STEPS: i32 = 4;
pub const MAX_STEP_HEIGHT: f32 = TILE_SIZE;

/* ===========================================================
   jet‑pack exhaust
   =========================================================== */
pub const EXHAUST_LIFETIME: f32 = 0.8;
pub const EXHAUST_RATE:    usize = 8;
pub const EXHAUST_SIZE:     f32 = 3.0;
pub const EXHAUST_COLOR: Color = Color::srgba(1.0, 0.6, 0.2, 1.0);
pub const EXHAUST_SPEED_Y: Range<f32> = -300.0..-120.0;
pub const EXHAUST_SPEED_X: Range<f32> =  -50.0..  50.0;

/* ===========================================================
   digging
   =========================================================== */
pub const DIG_RADIUS: f32 = 16.0;

/* ===========================================================
   enemy behaviour
   =========================================================== */
pub const AGGRO_RADIUS:    f32 = 32.0 * TILE_SIZE;
pub const ENEMY_SPEED:     f32 = WALK_SPEED * 0.8;
pub const ENEMY_KEEP_AWAY: f32 = 4.0 * TILE_SIZE;

/* ===========================================================
   colour variation (terrain tint)
   =========================================================== */
pub const COLOR_NOISE_SCALE: f64 = 0.05;
pub const COLOR_VARIATION_LEVELS: i32 = 4;
pub const COLOR_VARIATION_STRENGTH: f32 = 0.2;