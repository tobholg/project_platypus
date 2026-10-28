//! The high air (the sky arc), over the sky islands: the jet stream, a
//! band of wind blowing the way the weather's wind does, carrying what's
//! in the air through it (creatures, drifters: `drift`), streaks of it
//! showing where it runs. (The air thinning, `breath`; gravity fading,
//! `creatures::move_creatures`.)

use bevy::prelude::*;

use crate::magic::runes::Emitter;
use crate::world::{ChunkLoader, SimWorld};

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, streaks);
    }
}

/// The jet stream's speed in its middle (cells/s). (What's in it steers
/// through the air, and the air's going this way: `move_creatures`.)
pub const JET: f32 = 160.0;
/// Streaks in view in its middle, a second, at most.
const STREAKS: f32 = 70.0;

/// Which way the jet stream blows (the weather's wind's way), and how hard
/// at `y`: its velocity there.
pub fn jet_at(sim: &SimWorld, y: f32) -> f32 {
    let Some(sky) = sim.generator.sky_layers() else { return 0.0 };
    let way = if sim.world.wind() < 0.0 { -1.0 } else { 1.0 };
    way * JET * sky.jet(y)
}

/// Streaks of wind racing through the jet stream where it's in view.
fn streaks(time: Res<Time>, sim: Res<SimWorld>, camera: Query<(&GlobalTransform, &ChunkLoader), With<crate::camera::MainCamera>>, mut sparks: ResMut<crate::vfx::Sparks>, mut owed: Local<f32>, mut seed: Local<u64>) {
    let Some(sky) = sim.generator.sky_layers() else { return };
    let Ok((tf, l)) = camera.single() else { return };
    let c = tf.translation().truncate();
    let (lo, hi) = ((c.y - l.half_extent.y).max(sky.jet.0 as f32), (c.y + l.half_extent.y).min(sky.jet.1 as f32));
    if hi <= lo {
        *owed = 0.0;
        return;
    }
    let share = (hi - lo) / (sky.jet.1 - sky.jet.0) as f32;
    *owed += time.delta_secs() * STREAKS * share;
    let mut roll = || {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (*seed >> 33) as f32 / (1u64 << 31) as f32
    };
    while *owed >= 1.0 {
        *owed -= 1.0;
        let at = Vec2::new(c.x + (roll() - 0.5) * 2.0 * l.half_extent.x, lo + roll() * (hi - lo));
        let jet = jet_at(&sim, at.y);
        if jet.abs() < 20.0 {
            continue;
        }
        sparks.emit(&STREAK, 1, at, Vec2::new(jet.signum(), 0.0), Vec2::new(jet, 0.0));
    }
}

/// A streak of the jet stream: pale, racing along with it.
static STREAK: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.8, 1.6),
    colors: vec![(236, 244, 255), (210, 226, 246)],
    speed: 30.0,
    spread: 0.04,
    gravity: 0.0,
    drag: 0.0,
    size: 1.0,
    jitter: 0.0,
    glow: false,
});
