//! The simplest brain, and marching orders.
//!
//! - `idle`: stands still (training dummies, props, art being tried out).
//! - `Marching`: given to creatures on a march (a raid on the village):
//!   with no one to fight, the `hunter` brain walks them there.

use bevy::prelude::*;
use serde::Deserialize;

use crate::creatures::brain::RegisterBrain;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.register_brain::<Idle>("idle");
    }
}

/// Stands still.
#[derive(Component, Deserialize, Default)]
pub struct Idle;

/// On the march (a raid: `events.rs`): with no one to fight, it walks to
/// this x rather than wandering, and stops there.
#[derive(Component, Clone, Copy, Debug)]
pub struct Marching(pub f32);

/// Near enough the march's end (cells): it's there.
const MARCHED: f32 = 36.0;

/// Which way a march goes from `x` (0 there, or not marching).
pub fn march(m: Option<&Marching>, x: f32) -> f32 {
    m.map_or(0.0, |m| if (m.0 - x).abs() < MARCHED { 0.0 } else { (m.0 - x).signum() })
}
