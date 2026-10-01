//! How a creature takes hurt (DESIGN §14.2): ten kinds of damage, and for
//! each creature a multiplier per kind and what it can't suffer.
//!
//! A creature file names its `kind` (`assets/data/kinds.ron`: beast,
//! humanoid, insect, undead, spirit, ooze, construct, starfire, ...), which
//! gives a profile that makes sense for what it is, and may adjust it
//! (`profile`, `cant`). A multiplier of 1 takes it as anyone would; 0
//! shrugs it off; 2 is a weakness; below 0 it heals (it drinks it in). The
//! profile lives in `Health` (its one copy), so every hurt, from any source,
//! goes through it (`Health::harm`), after what gear stops (`Ward`).

use std::collections::HashMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::Health;

/// Kinds of hurt, each with sources in the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Harm {
    /// Swords, cleavers, claws.
    Slash,
    /// Arrows, thrusts, stingers, bites, shot.
    Pierce,
    /// Clubs, kicks, falling rock, boulders, blasts, crushing.
    Blunt,
    Fire,
    Frost,
    /// Lightning, charged water.
    Storm,
    Acid,
    /// Venom, blight.
    Poison,
    /// Light.
    Radiant,
    /// The void wand, necromancy.
    Void,
    /// A fall (only fall resistance helps; not an attack).
    Fall,
}

impl Harm {
    /// Every kind, in order (`Harm as usize` is its place).
    pub const ALL: [Harm; 11] = [
        Harm::Slash,
        Harm::Pierce,
        Harm::Blunt,
        Harm::Fire,
        Harm::Frost,
        Harm::Storm,
        Harm::Acid,
        Harm::Poison,
        Harm::Radiant,
        Harm::Void,
        Harm::Fall,
    ];

    /// Its name, as files write it.
    pub fn name(self) -> &'static str {
        ["slash", "pierce", "blunt", "fire", "frost", "storm", "acid", "poison", "radiant", "void", "fall"][self as usize]
    }

    /// Its bit (for sets of kinds: `Health::felt`).
    pub fn bit(self) -> u16 {
        1 << self as u16
    }
}

/// What a creature can't suffer, whatever hurts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Cant {
    /// Catch fire.
    Burn,
    /// Be chilled (slowed by cold).
    Chill,
    /// Be poisoned (venom does nothing).
    Poison,
    /// Be held in webs.
    Web,
    /// Be staggered (knocked about by hits).
    Stagger,
}

/// How a creature takes each kind of hurt, and what it can't suffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nature {
    by: [f32; 11],
    cant: u8,
}

impl Default for Nature {
    fn default() -> Self {
        Nature { by: [1.0; 11], cant: 0 }
    }
}

impl Nature {
    /// The multiplier for a kind of hurt (1: as anyone; 0: none; below 0:
    /// it heals).
    pub fn of(&self, kind: Harm) -> f32 {
        self.by[kind as usize]
    }

    pub fn cant(&self, what: Cant) -> bool {
        self.cant & (1 << what as u8) != 0
    }

    /// A kind's profile, then a creature's own changes to it.
    pub fn of_kind(kinds: &Kinds, kind: Option<&str>, profile: &HashMap<Harm, f32>, cant: &[Cant]) -> Result<Nature, String> {
        let mut n = Nature::default();
        if let Some(k) = kind {
            let def = kinds.0.get(k).ok_or_else(|| {
                let mut known: Vec<_> = kinds.0.keys().cloned().collect();
                known.sort();
                format!("unknown kind `{k}` (kinds.ron: {})", known.join(", "))
            })?;
            n.apply(&def.profile, &def.cant);
        }
        n.apply(profile, cant);
        Ok(n)
    }

    fn apply(&mut self, profile: &HashMap<Harm, f32>, cant: &[Cant]) {
        for (&k, &m) in profile {
            self.by[k as usize] = m;
        }
        for &c in cant {
            self.cant |= 1 << c as u8;
        }
    }

    /// A nature with these multipliers (tests).
    #[cfg(test)]
    pub fn with(mut self, kind: Harm, m: f32) -> Self {
        self.by[kind as usize] = m;
        self
    }

    #[cfg(test)]
    pub fn without(mut self, what: Cant) -> Self {
        self.cant |= 1 << what as u8;
        self
    }
}

/// One kind's profile (`kinds.ron`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct KindDef {
    pub profile: HashMap<Harm, f32>,
    pub cant: Vec<Cant>,
    /// What it's called in the bestiary (beast, the undead, ...).
    pub label: String,
}

/// The kinds of creature (`assets/data/kinds.ron`), by name.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(transparent)]
pub struct Kinds(pub HashMap<String, KindDef>);

impl Kinds {
    pub fn load() -> Result<Kinds, String> {
        crate::data::load_ron(&crate::data::data_path("kinds.ron"))
    }
}

/// Healing over time, stopped a while by some kinds of hurt (a troll's,
/// stopped by fire and acid): health a second, the kinds, how long.
#[derive(Clone, Debug, Deserialize)]
pub struct RegenDef {
    pub per_sec: f32,
    #[serde(default)]
    pub stopped_by: Vec<Harm>,
    #[serde(default = "five")]
    pub pause: f32,
    /// It can't die while it's healing: brought down by anything else, it
    /// hangs on at the last of its health and heals (a troll: only fire or
    /// acid, then any blow, finishes it).
    #[serde(default)]
    pub undying: bool,
}

fn five() -> f32 {
    5.0
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
    pub fn new(def: &RegenDef) -> Self {
        Regenerates { per_sec: def.per_sec, stopped_by: def.stopped_by.iter().fold(0, |m, h| m | h.bit()), pause: def.pause, waiting: 0.0, undying: def.undying }
    }
}

/// Undying, brought down by what doesn't stop its healing: it hangs on at
/// this much.
const HANGS_ON: f32 = 1.0;

/// Healing, unless something that stops it hurt it lately; an undying one
/// can't die while it heals.
pub(crate) fn regenerate(mut q: Query<(&mut Regenerates, &mut Health)>) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_hurt_is_kept_by_kind_until_tallied() {
        for (i, h) in Harm::ALL.into_iter().enumerate() {
            assert_eq!(h as usize, i, "{h:?} in its place");
        }
        let mut nature = Nature::default();
        nature.by[Harm::Void as usize] = -0.5;
        let mut h = Health { nature, ..Health::new(50.0) };
        h.hp = 40.0;
        h.harm(10.0, Harm::Slash);
        h.harm(5.0, Harm::Slash);
        h.harm(4.0, Harm::Void);
        assert_eq!(h.took[Harm::Slash as usize], (15.0, 15.0));
        assert_eq!(h.took[Harm::Void as usize], (4.0, -2.0));
        assert_eq!(h.took[Harm::Fire as usize], (0.0, 0.0));
    }

    #[test]
    fn every_kind_reads_and_makes_sense() {
        let kinds = Kinds::load().expect("kinds.ron");
        for name in ["beast", "humanoid", "insect", "undead", "spirit", "ooze", "construct", "starfire"] {
            assert!(kinds.0.contains_key(name), "kinds.ron has {name}");
        }
        let undead = Nature::of_kind(&kinds, Some("undead"), &HashMap::new(), &[]).unwrap();
        assert!(undead.of(Harm::Blunt) > 1.0 && undead.of(Harm::Pierce) < 1.0, "bones: blunt breaks them, points pass through");
        assert!(undead.of(Harm::Radiant) > 1.0 && undead.of(Harm::Void) < 0.0, "light hurts the dead; the void feeds them");
        assert!(undead.cant(Cant::Poison));
        let ooze = Nature::of_kind(&kinds, Some("ooze"), &HashMap::from([(Harm::Acid, -0.5)]), &[]).unwrap();
        assert!(ooze.of(Harm::Acid) < 0.0 && ooze.of(Harm::Blunt) < 0.5, "an acid slime drinks acid; blows sink in");
        assert!(Nature::of_kind(&kinds, Some("nonsense"), &HashMap::new(), &[]).is_err());
        // Every kind's multipliers within reason, no kind immune to all.
        for (name, k) in &kinds.0 {
            let n = Nature::of_kind(&kinds, Some(name), &HashMap::new(), &[]).unwrap();
            let all = [Harm::Slash, Harm::Pierce, Harm::Blunt, Harm::Fire, Harm::Frost, Harm::Storm, Harm::Acid, Harm::Poison, Harm::Radiant, Harm::Void, Harm::Fall];
            assert!(all.iter().all(|&h| (-1.0..=3.0).contains(&n.of(h))), "{name}");
            assert!(all.iter().any(|&h| n.of(h) >= 1.0), "{name} can be hurt somehow");
            assert!(!k.label.is_empty(), "{name} has a label");
        }
    }
}
