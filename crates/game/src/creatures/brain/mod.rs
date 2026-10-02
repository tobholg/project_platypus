//! Brains decide; bodies obey. A brain is a component holding its settings
//! (deserialised from the creature file's `brain.params`) plus systems in
//! `BrainSet` (in `TickSet::Intent`) that write `Controls`.
//!
//! The vocabulary: `hunter` (everything that fights: `hunter.rs` lists its
//! choices), `critter` (flees), `villager`, `idle`. A new way of fighting
//! is first a choice in `hunter` (a `Close` or an `Attack`); one creature's
//! trick is its own code (`custom/`) until a second needs it.
//!
//! Adding a brain, when no setting of these will do:
//! ```ignore
//! #[derive(Component, Deserialize, Default)]
//! #[serde(default)]
//! struct Shepherd { flock: f32 }
//!
//! app.register_brain::<Shepherd>("shepherd")
//!    .add_systems(FixedUpdate, herd.in_set(BrainSet));
//! ```
//! and any creature file can then say `brain: (kind: "shepherd", params: (flock: 40))`.

pub mod ai;
pub mod critters;
pub mod hunter;
pub mod villager;
pub mod way;

use std::collections::HashMap;

use bevy::prelude::*;
use serde::de::DeserializeOwned;

use super::def::BrainDef;

type Inserter = Box<dyn Fn(&mut EntityWorldMut, Option<&ron::value::RawValue>) -> Result<(), String> + Send + Sync>;
type Remover = Box<dyn Fn(&mut EntityWorldMut) + Send + Sync>;

/// Each brain by name: how it's put on a creature, and taken off (the
/// bestiary's stage drives a creature itself).
#[derive(Resource, Default)]
pub struct BrainRegistry(HashMap<String, (Inserter, Remover)>);

/// When brains decide: each tick, in `TickSet::Intent` (creatures' own
/// code thinks after them: `custom::CustomSet::Think`).
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrainSet;

impl BrainRegistry {
    pub fn has(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }

    pub fn insert(&self, def: &BrainDef, entity: &mut EntityWorldMut) -> Result<(), String> {
        let inserter = self.0.get(&def.kind).ok_or_else(|| {
            let mut known: Vec<_> = self.0.keys().cloned().collect();
            known.sort();
            format!("unknown brain `{}` (registered: {})", def.kind, known.join(", "))
        })?;
        (inserter.0)(entity, def.params.as_deref())
    }

    /// Take a brain off (by its name): the creature stands until something
    /// else writes its `Controls`.
    pub fn remove(&self, kind: &str, entity: &mut EntityWorldMut) {
        if let Some((_, remove)) = self.0.get(kind) {
            remove(entity);
        }
    }
}

/// A creature on the bestiary's stage: no brain, driven by the stage; its
/// moves started by the stage only; never cleared away as a far critter.
#[derive(Component)]
pub struct Staged;

pub trait RegisterBrain {
    fn register_brain<B: Component + DeserializeOwned + Default>(&mut self, name: &str) -> &mut Self;
}

impl RegisterBrain for App {
    fn register_brain<B: Component + DeserializeOwned + Default>(&mut self, name: &str) -> &mut Self {
        let name_owned = name.to_string();
        self.world_mut().get_resource_or_init::<BrainRegistry>().0.insert(
            name.to_string(),
            (
                Box::new(move |entity, params| {
                    let brain: B = match params {
                        None => B::default(),
                        Some(v) => crate::data::parse_ron(v.get_ron()).map_err(|e| format!("brain `{name_owned}` params: {e}"))?,
                    };
                    entity.insert(brain);
                    Ok(())
                }),
                Box::new(|entity| {
                    entity.remove::<B>();
                }),
            ),
        );
        self
    }
}

pub struct BrainPlugin;

impl Plugin for BrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BrainRegistry>().configure_sets(FixedUpdate, BrainSet.in_set(crate::world::TickSet::Intent));
    }
}
