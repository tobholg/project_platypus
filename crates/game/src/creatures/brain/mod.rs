//! Brains decide; bodies obey. A brain is a component holding its settings
//! (deserialised from the creature file's `brain.params`) plus systems in
//! `BrainSet` (in `TickSet::Intent`) that write `Controls`.
//!
//! Adding a behaviour:
//! ```ignore
//! #[derive(Component, Deserialize, Default)]
//! #[serde(default)]
//! struct Hopper { interval: f32 }
//!
//! app.register_brain::<Hopper>("hopper")
//!    .add_systems(FixedUpdate, hop.in_set(BrainSet));
//! ```
//! and any creature file can then say `brain: (kind: "hopper", params: (interval: 1.5))`.

pub mod ai;
pub mod critters;
pub mod monsters;
pub mod villager;

use std::collections::HashMap;

use bevy::prelude::*;
use serde::de::DeserializeOwned;

use super::def::BrainDef;

type Inserter = Box<dyn Fn(&mut EntityWorldMut, Option<&ron::Value>) -> Result<(), String> + Send + Sync>;

#[derive(Resource, Default)]
pub struct BrainRegistry(HashMap<String, Inserter>);

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
        inserter(entity, def.params.as_ref())
    }
}

pub trait RegisterBrain {
    fn register_brain<B: Component + DeserializeOwned + Default>(&mut self, name: &str) -> &mut Self;
}

impl RegisterBrain for App {
    fn register_brain<B: Component + DeserializeOwned + Default>(&mut self, name: &str) -> &mut Self {
        let name_owned = name.to_string();
        self.world_mut().get_resource_or_init::<BrainRegistry>().0.insert(
            name.to_string(),
            Box::new(move |entity, params| {
                let brain: B = match params {
                    None => B::default(),
                    Some(v) => v.clone().into_rust().map_err(|e| format!("brain `{name_owned}` params: {e}"))?,
                };
                entity.insert(brain);
                Ok(())
            }),
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
