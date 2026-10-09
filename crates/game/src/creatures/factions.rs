//! Factions (BE `behaviour` stage 4): who hunts whom among the monsters.
//! A creature file's `faction` (orcs, undead, spiders, bats, raptors,
//! tyrants) puts it in one; `factions.ron` says what each hunts. A hunter
//! hunts what its faction hunts as it hunts you (`senses.rs`: seen, heard,
//! smelled, called); struck by anything hostile it fights back, hunted or
//! hunter. Between two hostile factions blows land though both are the
//! monsters' side (`Team::Enemy`): an orc's cleaver cuts a skeleton, a
//! spider's bite takes a bat.
//!
//! Each creature carries its faction as bits (`Faction`): its own, what it
//! hunts, and its foes (what it hunts and what hunts it), so a blow asks
//! "hostile?" without a lookup.

use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use crate::data::{data_path, load_ron};

/// `factions.ron`: what each faction hunts.
#[derive(Deserialize)]
struct FactionsFile {
    hunts: HashMap<String, Vec<String>>,
}

/// Every faction by name (its bit: its place), and what each hunts.
#[derive(Resource, Default)]
pub struct Factions {
    names: Vec<String>,
    hunts: Vec<u32>,
}

impl Factions {
    pub fn load() -> Factions {
        let file: FactionsFile = load_ron(&data_path("factions.ron")).unwrap_or_else(|e| panic!("{e}"));
        let mut names: Vec<String> = file.hunts.iter().flat_map(|(k, v)| std::iter::once(k).chain(v)).cloned().collect();
        names.sort();
        names.dedup();
        assert!(names.len() <= 32, "factions.ron: at most 32 factions");
        let bit = |n: &String| 1u32 << names.iter().position(|m| m == n).unwrap_or(0);
        let hunts = names.iter().map(|n| file.hunts.get(n).map_or(0, |v| v.iter().fold(0, |m, h| m | bit(h)))).collect();
        Factions { names, hunts }
    }

    /// The faction of this name, as a creature carries it.
    pub fn get(&self, name: &str) -> Option<Faction> {
        let i = self.names.iter().position(|n| n == name)?;
        let bit = 1u32 << i;
        let hunted_by = self.hunts.iter().enumerate().filter(|(_, h)| *h & bit != 0).fold(0, |m, (j, _)| m | (1u32 << j));
        Some(Faction { bit, hunts: self.hunts[i], foes: self.hunts[i] | hunted_by })
    }
}

/// A creature's faction: its bit, what it hunts, its foes (hunted or
/// hunting it).
#[derive(Component, Clone, Copy, Debug)]
pub struct Faction {
    pub bit: u32,
    pub hunts: u32,
    pub foes: u32,
}

impl Faction {
    /// It hunts what's of that faction.
    pub fn hunts(&self, other: &Faction) -> bool {
        self.hunts & other.bit != 0
    }
}

/// Two creatures at war (a blow between them lands, struck it fights back).
pub fn hostile(a: Option<&Faction>, b: Option<&Faction>) -> bool {
    matches!((a, b), (Some(a), Some(b)) if a.foes & b.bit != 0)
}

pub struct FactionsPlugin;

impl Plugin for FactionsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Factions::load());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn who_hunts_whom_and_who_fights_back() {
        let file = FactionsFile { hunts: HashMap::from([("orcs".into(), vec!["undead".into()]), ("raptors".into(), vec!["orcs".into()]), ("undead".into(), vec!["orcs".into()])]) };
        let mut names: Vec<String> = file.hunts.iter().flat_map(|(k, v)| std::iter::once(k).chain(v)).cloned().collect();
        names.sort();
        names.dedup();
        let bit = |n: &String| 1u32 << names.iter().position(|m| m == n).unwrap();
        let hunts = names.iter().map(|n| file.hunts.get(n).map_or(0, |v| v.iter().fold(0, |m, h| m | bit(h)))).collect();
        let f = Factions { names, hunts };
        let (orc, raptor, skeleton) = (f.get("orcs").unwrap(), f.get("raptors").unwrap(), f.get("undead").unwrap());
        assert!(raptor.hunts(&orc) && !orc.hunts(&raptor), "the raptor's the hunter");
        assert!(hostile(Some(&orc), Some(&raptor)), "but they're at war: struck, the orc fights back");
        assert!(hostile(Some(&orc), Some(&skeleton)) && hostile(Some(&skeleton), Some(&orc)));
        assert!(!hostile(Some(&raptor), Some(&skeleton)), "neither hunts the other");
        assert!(!hostile(Some(&orc), None), "no faction: nobody's foe");
    }
}
