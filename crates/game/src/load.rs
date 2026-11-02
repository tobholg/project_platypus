//! How loaded the game is: a pressure level, 0 (plenty of room) to 3
//! (struggling), from what the fixed tick costs (eased over a few dozen
//! ticks) and how many hunters are about. Systems that can do less
//! without it showing read it and do less: far hunters think every few
//! ticks, the planner gets less time a tick, far corpses fewer physics
//! substeps. On slower hardware the same scene costs more, so it reaches
//! the higher levels sooner, on its own.
//!
//! `PLATYPUS_LOAD=0..3` pins it (to compare); `PLATYPUS_LOADLOG=1` logs
//! each change.

use std::time::Instant;

use bevy::prelude::*;

pub struct LoadPlugin;

#[derive(Resource, Default)]
pub struct Load {
    /// 0 … 3.
    pub level: u8,
    /// The tick's cost (ms), eased.
    pub tick_ms: f32,
    /// Hunters thinking last tick.
    pub hunters: u32,
    started: Option<Instant>,
}

/// Tick cost (ms) that raises the level to 1, 2, 3; it falls again only
/// below `EASE_DOWN` of that (not in and out at the edge).
const UP: [f32; 3] = [3.0, 5.0, 8.0];
const EASE_DOWN: f32 = 0.75;
/// This many hunters thinking: at least level 1, whatever the cost (a
/// crowd: each a little dumber doesn't show).
const CROWD: u32 = 25;
/// A tick's share of the eased cost.
const EASE: f32 = 0.05;

impl Plugin for LoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Load>().add_systems(FixedFirst, start).add_systems(FixedLast, settle);
    }
}

fn pinned() -> Option<u8> {
    static PIN: std::sync::LazyLock<Option<u8>> = std::sync::LazyLock::new(|| std::env::var("PLATYPUS_LOAD").ok().and_then(|v| v.parse::<u8>().ok()).map(|l| l.min(3)));
    *PIN
}

fn start(mut load: ResMut<Load>) {
    load.started = Some(Instant::now());
}

fn settle(mut load: ResMut<Load>) {
    let Some(t) = load.started.take() else { return };
    let ms = t.elapsed().as_secs_f32() * 1e3;
    load.tick_ms += (ms - load.tick_ms) * EASE;
    let was = load.level;
    let mut level = was;
    while level < 3 && load.tick_ms > UP[level as usize] {
        level += 1;
    }
    while level > 0 && load.tick_ms < UP[level as usize - 1] * EASE_DOWN {
        level -= 1;
    }
    if load.hunters >= CROWD {
        level = level.max(1);
    }
    load.level = pinned().unwrap_or(level);
    if load.level != was && std::env::var("PLATYPUS_LOADLOG").is_ok() {
        info!("load: {was} -> {} (tick {:.2} ms, {} hunters)", load.level, load.tick_ms, load.hunters);
    }
}

impl Load {
    /// Every how many ticks a hunter this far from the nearest quarry
    /// (cells) thinks. Those in the fight stay sharp.
    pub fn think_every(&self, far: f32) -> u64 {
        const NEAR: f32 = 160.0;
        match (self.level, far > NEAR) {
            (0, _) => 1,
            (1, true) => 2,
            (2, true) => 3,
            (3, true) => 4,
            (3, false) => 2,
            _ => 1,
        }
    }

    /// The planner's time a tick (µs).
    pub fn plan_micros(&self) -> u64 {
        [2000, 1200, 700, 400][self.level as usize]
    }

    /// Ragdoll substeps for a corpse this far from the nearest player
    /// (cells) at this level: in view never fewer than two (one wobbles).
    pub fn ragdoll_substeps(&self, far: f32) -> usize {
        const IN_VIEW: f32 = 500.0;
        if far < IN_VIEW { [3, 3, 2, 2][self.level as usize] } else { [2, 1, 1, 1][self.level as usize] }
    }
}
