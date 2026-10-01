//! Code of a creature's own, for what data can't say (DESIGN §14.1).
//!
//! Every creature is data first: its body, how it moves, its moves, its
//! brain (`brain/`) and its kind (`nature`). When a creature needs
//! something that can't be said in data (a mimic that's a chest until you
//! touch it, a wyrm tunnelling through the cells), it gets a module here,
//! named in its file:
//!
//! ```ron
//! custom: (name: "hatchery", params: (brood: "spiderling", count: 4)),
//! ```
//!
//! A module is one file: a component holding its settings (deserialised
//! from `params`), and the hooks it needs, all optional (see
//! `_template.rs`, which stubs each and says when it runs):
//! - **spawn**: once it's in the world (an observer on its component);
//! - **think**: each tick, after its brain (`CustomSet::Think`): it may
//!   leave the brain's decision as it is, change it, or replace it;
//! - **moves**: its own kinds of attack (until moves are data: systems that
//!   act when it decides to);
//! - **hit**, **death**: reading `combat::Hit` (what hit it) and `Died`;
//! - **phase**: with bosses' phases (PLAN BE 10).
//!
//! It uses the engine, never goes round it: its hurt goes through
//! `Health::harm`, its blows are `combat::Hit`s, its body is its creature
//! file's, so the arena's tools work on it as on anything.
//!
//! When a second creature needs the same trick, it moves out of here into
//! the shared vocabulary: what stays here is truly one of a kind.
//!
//! Registered in `CustomPlugin` (one line each). A file naming a module
//! (or a brain) that doesn't exist is reported at start, loudly.

pub mod dummy;
pub mod explosive;
pub mod hatchery;
// (The template: compiled with the tests, so it stays right.)
#[cfg(test)]
mod _template;

use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::brain::BrainRegistry;
use super::def::Creatures;
use crate::world::TickSet;

pub struct CustomPlugin;

impl Plugin for CustomPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CustomRegistry>()
            .configure_sets(FixedUpdate, CustomSet::Think.in_set(TickSet::Intent).after(super::brain::BrainSet))
            .add_systems(PostStartup, check_names)
            // One line each.
            .register_custom::<dummy::Dummy>()
            .register_custom::<explosive::Explosive>()
            .register_custom::<hatchery::Hatchery>();
    }
}

/// When creatures' own code runs.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CustomSet {
    /// Each tick, after the brains have decided.
    Think,
}

/// A creature's own code: a component (its settings, from `params`) and
/// what it adds to the game (its hooks: systems, observers).
pub trait CustomCreature: Component + DeserializeOwned + Default {
    /// What creature files call it (`custom: (name: ...)`).
    const NAME: &'static str;
    /// Its systems and observers (the hooks it needs).
    fn build(app: &mut App);
}

/// What a creature file says of its own code: the module's name, and its
/// settings.
#[derive(Clone, Debug, Deserialize)]
pub struct CustomDef {
    pub name: String,
    #[serde(default)]
    pub params: Option<ron::Value>,
}

type Inserter = Box<dyn Fn(&mut EntityWorldMut, Option<&ron::Value>) -> Result<(), String> + Send + Sync>;

/// The modules, by name.
#[derive(Resource, Default)]
pub struct CustomRegistry(HashMap<String, Inserter>);

impl CustomRegistry {
    pub fn insert(&self, def: &CustomDef, entity: &mut EntityWorldMut) -> Result<(), String> {
        let inserter = self.0.get(&def.name).ok_or_else(|| format!("no custom module `{}` (registered: {})", def.name, self.names().join(", ")))?;
        inserter(entity, def.params.as_ref())
    }

    pub fn has(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }

    fn names(&self) -> Vec<String> {
        let mut n: Vec<_> = self.0.keys().cloned().collect();
        n.sort();
        n
    }
}

pub trait RegisterCustom {
    fn register_custom<C: CustomCreature>(&mut self) -> &mut Self;
}

impl RegisterCustom for App {
    fn register_custom<C: CustomCreature>(&mut self) -> &mut Self {
        self.world_mut().get_resource_or_init::<CustomRegistry>().0.insert(
            C::NAME.to_string(),
            Box::new(|entity, params| {
                let c: C = match params {
                    None => C::default(),
                    Some(v) => v.clone().into_rust().map_err(|e| format!("custom `{}` params: {e}", C::NAME))?,
                };
                entity.insert(c);
                Ok(())
            }),
        );
        C::build(self);
        self
    }
}

/// Every creature file's brain and own code exist (else it's said, loudly,
/// at start: not when one first walks into view).
fn check_names(creatures: Res<Creatures>, brains: Res<BrainRegistry>, custom: Res<CustomRegistry>) {
    let mut bad = Vec::new();
    for (id, def) in creatures.all() {
        if !brains.has(&def.brain.kind) {
            bad.push(format!("{id}: no brain `{}`", def.brain.kind));
        }
        if let Some(c) = &def.custom
            && !custom.has(&c.name)
        {
            bad.push(format!("{id}: no custom module `{}` (registered: {})", c.name, custom.names().join(", ")));
        }
    }
    for b in &bad {
        error!("creature file: {b}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_module_registers_and_its_settings_read() {
        let mut app = App::new();
        app.init_resource::<CustomRegistry>().register_custom::<_template::Template>();
        let reg = app.world().resource::<CustomRegistry>();
        assert!(reg.has("template"));
        let def: CustomDef = ron::from_str("(name: \"template\", params: Some((every: 3.5)))").unwrap();
        let e = app.world_mut().spawn_empty().id();
        app.world_mut().resource_scope(|world, reg: Mut<CustomRegistry>| reg.insert(&def, &mut world.entity_mut(e))).unwrap();
        assert!(app.world().get::<_template::Template>(e).is_some());
        let bad: CustomDef = ron::from_str("(name: \"nothing\")").unwrap();
        let e2 = app.world_mut().spawn_empty().id();
        let err = app.world_mut().resource_scope(|world, reg: Mut<CustomRegistry>| reg.insert(&bad, &mut world.entity_mut(e2))).unwrap_err();
        assert!(err.contains("no custom module `nothing`"), "{err}");
    }
}
