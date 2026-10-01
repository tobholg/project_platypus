//! A template for a creature's own code (`custom/mod.rs` says when to
//! write one). Copy it to `custom/<name>.rs`, rename `Template`, add
//! `pub mod <name>;` and one `.register_custom::<...>()` line in
//! `CustomPlugin`, and name it in the creature's file:
//!
//! ```ron
//! custom: (name: "template", params: (every: 2.0)),
//! ```
//!
//! Keep only the hooks you use. (Compiled with the tests, so it stays
//! right; `custom/mod.rs` registers it in a test.)

use bevy::prelude::*;
use serde::Deserialize;

use super::{CustomCreature, CustomSet};
use crate::creatures::{Controls, Died, Health, Kinematics};
use crate::world::TickSet;

/// Its settings (from `params`) and what it remembers.
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Template {
    /// Seconds between its tricks.
    pub every: f32,
    #[serde(skip)]
    wait: f32,
}

impl Default for Template {
    fn default() -> Self {
        Template { every: 2.0, wait: 0.0 }
    }
}

impl CustomCreature for Template {
    const NAME: &'static str = "template";

    fn build(app: &mut App) {
        app.add_observer(on_spawn)
            .add_systems(FixedUpdate, think.in_set(CustomSet::Think))
            .add_systems(FixedUpdate, (on_hit, on_death).in_set(TickSet::Bodies));
    }
}

/// Spawn: once it's in the world (its creature file's components are on it).
fn on_spawn(add: On<Add, Template>, mut q: Query<&mut Template>) {
    if let Ok(mut t) = q.get_mut(add.entity) {
        t.wait = t.every;
    }
}

/// Think: each tick, after its brain. Leave `Controls` as the brain set
/// them, change them, or replace them.
fn think(mut q: Query<(&mut Template, &Kinematics, &mut Controls)>) {
    let dt = 1.0 / crate::world::TICK_HZ as f32;
    for (mut t, _k, mut _c) in &mut q {
        t.wait -= dt;
        if t.wait <= 0.0 {
            t.wait = t.every;
            // Its trick: request a move, spawn something, change the world.
        }
    }
}

/// Hit: what struck it this tick.
fn on_hit(mut hits: MessageReader<crate::combat::Hit>, q: Query<(&Template, &Health)>) {
    for h in hits.read() {
        if let Ok((_t, _health)) = q.get(h.target) {
            // React: flinch, call for help, change phase.
        }
    }
}

/// Death: its kind died (its body's still to come).
fn on_death(mut died: MessageReader<Died>) {
    for d in died.read() {
        if d.kind == "template" {
            // Burst into something, drop something.
        }
    }
}
