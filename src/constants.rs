use bevy::prelude::*;
use std::ops::Range;

/* ===========================================================
   world geometry
   =========================================================== */
/// size of one square tile, in world‑space units
pub const TILE_SIZE: f32 = 8.0;

/// map dimensions (tiles)
pub const CHUNK_WIDTH:  usize = 120;
pub const CHUNK_HEIGHT: usize =  90;
pub const NUM_CHUNKS_X: usize =   3;
pub const NUM_CHUNKS_Y: usize =   2;

/* ===========================================================
   streaming “active window”
   =========================================================== */
/// half‑width/height (in tiles) of the rectangle that is kept
/// fully active (sprites spawned, enemies awake, etc.) around
/// the camera each frame
pub const ACTIVE_PAD_X: i32 = 40;   // ≈ 6 screen‑widths
pub const ACTIVE_PAD_Y: i32 = 25;   // ≈ 3 screen‑heights
pub const ACTIVE_MARGIN: i32 = 10;

/* ===========================================================
   digging
   =========================================================== */
pub const DIG_RADIUS: f32 = 16.0;

/* ===========================================================
   enemy behaviour
   =========================================================== */
pub const AGGRO_RADIUS: f32      = 32.0 * TILE_SIZE;
pub const ENEMY_SPEED: f32       = WALK_SPEED * 0.8; // 80 % of player speed
pub const ENEMY_KEEP_AWAY: f32   = 4.0 * TILE_SIZE;

/* ===========================================================
   player physics and movement
   =========================================================== */
pub const PLAYER_WIDTH:  f32 = TILE_SIZE;  // 1‑tile wide
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
pub const EXHAUST_COLOR: Color = Color::rgba(1.0, 0.6, 0.2, 1.0);
pub const EXHAUST_SPEED_Y: Range<f32> = -300.0..-120.0;
pub const EXHAUST_SPEED_X: Range<f32> =  -50.0..  50.0;

/* ===========================================================
   colour variation (terrain tint)
   =========================================================== */
pub const COLOR_NOISE_SCALE: f64 = 0.05;
pub const COLOR_VARIATION_LEVELS: i32 = 4;
pub const COLOR_VARIATION_STRENGTH: f32 = 0.2;