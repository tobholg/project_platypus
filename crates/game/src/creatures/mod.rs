//! Creatures (DESIGN §14): every creature is data, in layers (its body,
//! how it moves, its moves, its behaviour, its phases); a few also have code
//! of their own (`custom/`).
//!
//! So far: how each takes hurt (`nature`: ten kinds of damage, a profile
//! per kind of creature, healing from some, what it can't suffer), and
//! regeneration.

pub mod nature;

use bevy::prelude::*;

use crate::actors::Health;
use crate::world::TickSet;

pub struct CreaturesPlugin;

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, regenerate.in_set(TickSet::Bodies).before(crate::actors::deaths));
    }
}

/// It heals over time (`regen` in its file), stopped a while by some kinds
/// of hurt: health a second, the kinds (bits), how long, and how long it
/// still waits.
#[derive(Component, Clone, Debug)]
pub struct Regenerates {
    pub per_sec: f32,
    pub stopped_by: u16,
    pub pause: f32,
    pub waiting: f32,
    pub undying: bool,
}

impl Regenerates {
    pub fn new(def: &nature::RegenDef) -> Self {
        Regenerates { per_sec: def.per_sec, stopped_by: def.stopped_by.iter().fold(0, |m, h| m | h.bit()), pause: def.pause, waiting: 0.0, undying: def.undying }
    }
}

/// Undying, brought down by what doesn't stop its healing: it hangs on at
/// this much.
const HANGS_ON: f32 = 1.0;

/// Healing, unless something that stops it hurt it lately; an undying one
/// can't die while it heals.
fn regenerate(mut q: Query<(&mut Regenerates, &mut Health)>) {
    let dt = 1.0 / crate::world::TICK_HZ as f32;
    for (mut r, mut h) in &mut q {
        if h.felt & r.stopped_by != 0 {
            r.waiting = r.pause;
        }
        h.felt = 0;
        // (Healing still: what brought it down can't finish it.)
        if r.undying && r.waiting <= 0.0 && h.hp <= 0.0 {
            h.hp = HANGS_ON;
        }
        if r.waiting > 0.0 {
            r.waiting -= dt;
            continue;
        }
        if h.hp > 0.0 && h.hp < h.max {
            h.hp = (h.hp + r.per_sec * dt).min(h.max);
        }
    }
}
