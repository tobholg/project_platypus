//! Gear (DESIGN §7c): things worn and wielded that change their wearer's
//! stats. Any item with a `gear` block is gear: armour and trinkets are
//! written in `assets/data/gear.ron`, weapons, tools and foci keep theirs in
//! `items.ron`. Every creature has `Equipment` (the humanoids wear it; any
//! holds what's in its hand) and the `Stats` it adds up to:
//!
//! - worn: head, body, hands, legs, feet and two trinkets;
//! - held: whatever's in the hand, while it's there (a sword's damage, a
//!   staff's spell power);
//! - each piece gives its own stats, and its armour's weight some more
//!   (heavy plate costs stamina and mana regen: `weights` in gear.ron).
//!
//! When what it wears changes (or the creature files do), `apply` puts the
//! stats into what they change: its health and ward, mana, stamina, poise
//! and movement. Combat and magic read the rest where they happen.

pub mod look;
pub mod roll;
pub mod stats;

use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::Deserialize;

use crate::actors::creature::Creatures;
use crate::actors::{Creature, Health, MoveStats, Ward};
use crate::combat::{Stamina, Sturdy};
use crate::data::{data_path, load_ron};
use crate::hands::items::{ItemDef, Items, Stack};
use crate::magic::Mana;
pub use stats::{Stat, Stats};

/// Where a piece of gear goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum Slot {
    Head,
    Body,
    Hands,
    Legs,
    Feet,
    /// Rings, amulets, charms: two may be worn.
    Trinket,
    /// In the hand (the hotbar slot in use): weapons, tools, foci.
    Held,
}

impl Slot {
    pub fn label(self) -> &'static str {
        match self {
            Slot::Head => "Head",
            Slot::Body => "Body",
            Slot::Hands => "Hands",
            Slot::Legs => "Legs",
            Slot::Feet => "Feet",
            Slot::Trinket => "Trinket",
            Slot::Held => "Held",
        }
    }
}

/// The worn slots, in order (what `Equipment::worn` holds).
pub const WORN: [Slot; 7] = [Slot::Head, Slot::Body, Slot::Hands, Slot::Legs, Slot::Feet, Slot::Trinket, Slot::Trinket];

/// How heavy a piece of armour is: what kind of fighter wears it. Its
/// weight's stats (gear.ron `weights`) come with every piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum Weight {
    /// Cloth: mages.
    Light,
    /// Leather: rangers.
    Medium,
    /// Mail and plate: warriors.
    Heavy,
}

/// What makes an item gear.
#[derive(Clone, Debug, Deserialize)]
pub struct GearDef {
    pub slot: Slot,
    /// Armour's weight (none: not armour).
    #[serde(default)]
    pub weight: Option<Weight>,
    /// What it gives (before any rolled bonuses).
    #[serde(default)]
    pub stats: BTreeMap<Stat, f32>,
    /// How it looks worn, on the humanoid rig (`look.rs`).
    #[serde(default)]
    pub look: Option<platypus_art::dress::Skin>,
    /// A named unique: always legendary, no random bonuses (its `stats`
    /// are all it has).
    #[serde(default)]
    pub unique: bool,
}

/// gear.ron: what armour's weight costs, the rarities and the bonuses
/// gear can roll, and the gear itself (items, as in items.ron).
#[derive(Clone, Debug, Deserialize)]
pub struct GearFile {
    #[serde(default)]
    pub weights: BTreeMap<Weight, BTreeMap<Stat, f32>>,
    pub rarities: Vec<roll::Rarity>,
    #[serde(default)]
    pub bonuses: Vec<roll::Bonus>,
    pub items: Vec<ItemDef>,
}

pub fn load() -> GearFile {
    load_ron(&data_path("gear.ron")).unwrap_or_else(|e| panic!("{e}"))
}

/// The rules gear follows (from gear.ron).
#[derive(Resource)]
pub struct GearRules {
    pub weights: BTreeMap<Weight, BTreeMap<Stat, f32>>,
    pub rarities: Vec<roll::Rarity>,
    pub bonuses: Vec<roll::Bonus>,
}

impl GearRules {
    /// A piece's rolled bonuses (stat, amount).
    pub fn bonuses(&self, items: &Items, s: &Stack) -> Vec<(Stat, f32)> {
        roll::bonuses(&self.rarities, &self.bonuses, items.def(s.item), &s.roll).into_iter().map(|(i, v)| (self.bonuses[i].stat, v)).collect()
    }

    /// What a stack is called: gear by its roll ("Sturdy iron helm of the
    /// Bear"), anything else by its name.
    pub fn name(&self, items: &Items, s: &Stack) -> String {
        let def = items.def(s.item);
        let rolled = roll::bonuses(&self.rarities, &self.bonuses, def, &s.roll);
        roll::name(&self.bonuses, def, &rolled)
    }

    /// A piece's rarity (none: not gear).
    pub fn rarity(&self, items: &Items, s: &Stack) -> Option<&roll::Rarity> {
        items.def(s.item).gear.as_ref()?;
        self.rarities.get(s.roll.rarity as usize)
    }
}

/// What a creature wears, and what's in its hand.
#[derive(Component, Clone, Debug, Default)]
pub struct Equipment {
    pub worn: [Option<Stack>; WORN.len()],
    pub held: Option<Stack>,
}

impl Equipment {
    /// Does this item go in worn slot `i`?
    pub fn fits(items: &Items, i: usize, stack: &Stack) -> bool {
        items.def(stack.item).gear.as_ref().is_some_and(|g| WORN.get(i) == Some(&g.slot))
    }

    /// The worn slot an item would go in: an empty one of its kind first.
    pub fn slot_for(&self, items: &Items, stack: &Stack) -> Option<usize> {
        let fits: Vec<usize> = (0..WORN.len()).filter(|&i| Self::fits(items, i, stack)).collect();
        fits.iter().copied().find(|&i| self.worn[i].is_none()).or(fits.first().copied())
    }

    /// Everything it has on it that counts: what it wears, and what it holds
    /// if that's held gear.
    pub fn pieces<'a>(&'a self, items: &'a Items) -> impl Iterator<Item = &'a Stack> + 'a {
        let held = self.held.iter().filter(|s| items.def(s.item).gear.as_ref().is_some_and(|g| g.slot == Slot::Held));
        self.worn.iter().flatten().chain(held)
    }

    /// Its stats, added up.
    pub fn total(&self, items: &Items, rules: &GearRules) -> Stats {
        let mut total = Stats::default();
        for s in self.pieces(items) {
            add_piece(&mut total, items, rules, s);
        }
        total
    }
}

/// One piece's stats (its own, its rolled bonuses and its weight's) into
/// `into`.
pub fn add_piece(into: &mut Stats, items: &Items, rules: &GearRules, s: &Stack) {
    let Some(g) = &items.def(s.item).gear else { return };
    for (&stat, &v) in &g.stats {
        into.add(stat, v);
    }
    for (stat, v) in rules.bonuses(items, s) {
        into.add(stat, v);
    }
    if let Some(w) = g.weight.and_then(|w| rules.weights.get(&w)) {
        for (&stat, &v) in w {
            into.add(stat, v);
        }
    }
}

/// One piece's stats alone (a tooltip's).
pub fn piece_stats(items: &Items, rules: &GearRules, s: &Stack) -> Stats {
    let mut st = Stats::default();
    add_piece(&mut st, items, rules, s);
    st
}

pub struct GearPlugin;

impl Plugin for GearPlugin {
    fn build(&self, app: &mut App) {
        let file = load();
        app.insert_resource(GearRules { weights: file.weights, rarities: file.rarities, bonuses: file.bonuses })
            .init_resource::<look::Wardrobe>()
            .add_systems(Update, (apply, look::dress).after(crate::actors::creature::hot_reload_creatures));
    }
}

/// Stamina regen with nothing on (`Stamina::new`'s).
const STAMINA_REGEN: f32 = 45.0;

type Wearer<'a> = (
    &'a Creature,
    Ref<'a, Equipment>,
    &'a mut Stats,
    &'a mut Health,
    &'a mut MoveStats,
    Option<&'a mut Stamina>,
    Option<&'a mut Mana>,
    Option<&'a mut Sturdy>,
);

/// What a creature wears changed (or its kind did, or it just got mana):
/// add its stats up again and put them where they go.
fn apply(items: Option<Res<Items>>, rules: Res<GearRules>, creatures: Res<Creatures>, mut q: Query<Wearer>) {
    let Some(items) = items else { return };
    let all = creatures.is_changed() || items.is_added();
    for (c, eq, mut stats, mut health, mut moves, stamina, mana, sturdy) in &mut q {
        let mana_new = mana.as_ref().is_some_and(|m| m.is_added());
        if !(all || eq.is_changed() || mana_new) {
            continue;
        }
        let Some(def) = creatures.get(&c.kind) else { continue };
        let total = eq.total(&items, &rules);
        let max = (def.health + total.get(Stat::Health)).max(1.0);
        if max > health.max {
            health.hp += max - health.max;
        }
        health.max = max;
        health.hp = health.hp.min(max);
        health.ward = Ward {
            armor: total.get(Stat::Armor),
            fire: total.share(Stat::FireResist, 0.9),
            frost: total.share(Stat::FrostResist, 0.9),
            storm: total.share(Stat::StormResist, 0.9),
            acid: total.share(Stat::AcidResist, 0.9),
            fall: total.share(Stat::FallResist, 0.9),
        };
        let mut m = def.movement.clone();
        m.run_speed *= total.mult(Stat::MoveSpeed);
        m.jump_height *= total.mult(Stat::JumpHeight);
        m.air_jumps = (m.air_jumps as f32 + total.get(Stat::AirJumps)).round().clamp(0.0, 9.0) as u8;
        moves.0 = m;
        if let (Some(mut st), Some(base)) = (stamina, def.stamina) {
            st.max = (base + total.get(Stat::Stamina)).max(1.0);
            st.cur = st.cur.min(st.max);
            st.regen = STAMINA_REGEN * total.mult(Stat::StaminaRegen);
        }
        if let Some(mut s) = sturdy {
            s.poise = def.poise + total.get(Stat::Poise);
        }
        // (A caster's mana, from `Mana::default`.)
        if let Some(mut mp) = mana {
            let base = Mana::default();
            mp.max = (base.max + total.get(Stat::Mana)).max(1.0);
            mp.cur = mp.cur.min(mp.max);
            mp.regen = base.regen * total.mult(Stat::ManaRegen);
        }
        *stats = total;
    }
}
