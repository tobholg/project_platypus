use bevy::prelude::*;
use std::ops::Range;

/// -------- tiles & world size --------
pub const TILE_SIZE: f32 = 8.0;
pub const CHUNK_WIDTH: usize  = 160;
pub const CHUNK_HEIGHT: usize =  90;
pub const NUM_CHUNKS_X: usize =   3;
pub const NUM_CHUNKS_Y: usize =   2;

/// digging
pub const DIG_RADIUS: f32 = 16.0;

/// -------- player phys --------
pub const PLAYER_WIDTH: f32  =  8.0;
pub const PLAYER_HEIGHT: f32 = 18.0;
pub const GRAVITY: f32       = -400.0;
pub const JUMP_SPEED: f32    =  230.0;
pub const JET_ACCEL: f32     =  800.0;
pub const WALK_SPEED: f32    =  160.0;
pub const COLLISION_STEPS: i32 = 4;

/// -------- jet‑pack exhaust --------
pub const EXHAUST_LIFETIME: f32 = 0.6;
pub const EXHAUST_RATE: usize   = 6;
pub const EXHAUST_SIZE: f32     = 3.0;
pub const EXHAUST_COLOR: Color  = Color::rgba(1.0, 0.6, 0.2, 1.0);
pub const EXHAUST_SPEED_Y: Range<f32> = -300.0..-120.0;
pub const EXHAUST_SPEED_X: Range<f32> =  -50.0..  50.0;

/// -------- colour‑variation --------
pub const COLOR_NOISE_SCALE: f64   = 0.05;
pub const COLOR_VARIATION_LEVELS: i32 = 4;
pub const COLOR_VARIATION_STRENGTH: f32 = 0.2;