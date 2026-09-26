//! Spells and foci (DESIGN §7b, §7c). A spell is its own thing, made of
//! runes (`assets/data/spells.ron`); wands and staffs are foci, held gear
//! that holds no spells: a spell of tier N needs a focus of tier N or more
//! in the hand (a wand is tier 1, a staff tier 2), and the focus's stats
//! (spell power, cast speed, an element's power: a fire wand's +40% fire)
//! make it stronger. Every spell has an element (or none: arcane); its
//! element's power and spell power scale what it does, and its hurt is of
//! its element (resisted as such).
//!
//! A caster knows spells (`Caster`) and has one ready: Q steps to the next
//! (Shift+Q back); taking up a focus of an element readies the strongest
//! spell of that element it can cast.

use std::sync::Arc;

use bevy::prelude::*;
use serde::Deserialize;

use super::runes::{Carrier, Cast, Payload};
use crate::actors::Harm;
use crate::gear::{Stat, Stats};

/// What a spell (or a focus) is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
pub enum Element {
    Fire,
    Frost,
    Storm,
    Acid,
    Force,
    Gravity,
}

impl Element {
    /// The stat that strengthens it.
    pub fn power(self) -> Stat {
        match self {
            Element::Fire => Stat::FirePower,
            Element::Frost => Stat::FrostPower,
            Element::Storm => Stat::StormPower,
            Element::Acid => Stat::AcidPower,
            Element::Force => Stat::ForcePower,
            Element::Gravity => Stat::GravityPower,
        }
    }

    /// The kind of hurt it does.
    pub fn harm(self) -> Harm {
        match self {
            Element::Fire => Harm::Fire,
            Element::Frost => Harm::Frost,
            Element::Storm => Harm::Storm,
            Element::Acid => Harm::Acid,
            Element::Force | Element::Gravity => Harm::Physical,
        }
    }
}

/// A spell as written.
#[derive(Clone, Debug, Deserialize)]
pub struct SpellDef {
    pub id: String,
    pub name: String,
    /// Its runes, read left to right (`runes.rs`).
    pub runes: Vec<String>,
    /// Seconds between its casts, and after the last of them.
    pub delay: f32,
    pub recharge: f32,
    /// The least focus it needs (1: a wand; 2: a staff).
    #[serde(default = "one")]
    pub tier: u8,
    #[serde(default)]
    pub element: Option<Element>,
    #[serde(default)]
    pub about: Option<String>,
}

fn one() -> u8 {
    1
}

#[derive(Clone, Debug, Deserialize)]
pub struct SpellsFile {
    pub spells: Vec<SpellDef>,
}

/// The spells a caster knows (by index in the spellbook) and the one it
/// has ready.
#[derive(Component, Clone, Debug, Default)]
pub struct Caster {
    pub known: Vec<usize>,
    pub ready: usize,
}

impl Caster {
    /// The spell ready (its index in the spellbook).
    pub fn spell(&self) -> Option<usize> {
        self.known.get(self.ready).copied()
    }
}

/// How much stronger a caster makes a spell: spell power, and its
/// element's power.
pub fn power(stats: &Stats, element: Option<Element>) -> f32 {
    stats.mult(Stat::SpellPower) * element.map_or(1.0, |e| stats.mult(e.power()))
}

/// A cast made stronger (or weaker) by `power`, hurting as its element
/// does: its damage, blasts, heat, knock and matter, a well's lift and
/// grip, force's push, a stream's flow; and what it sets off, the same.
pub fn empower(cast: &Cast, power: f32, harm: Harm) -> Arc<Cast> {
    let mut c = cast.clone();
    c.harm = harm;
    if (power - 1.0).abs() > 1e-3 {
        for p in &mut c.payloads {
            match p {
                Payload::Damage(d) => *d *= power,
                Payload::Blast { power: b, .. } => *b = (*b as f32 * power).round().clamp(1.0, 255.0) as u8,
                Payload::Heat { amount, .. } => *amount = (*amount as f32 * power).round().clamp(-3000.0, 3000.0) as i16,
                Payload::Knock(k) => *k *= power,
                Payload::Matter { cells, .. } => *cells = (*cells as f32 * power).round() as u32,
                Payload::Ignite { .. } | Payload::Shatter { .. } => {}
            }
        }
        match &mut c.carrier {
            Carrier::Well { lift, grip, .. } => {
                *lift *= power;
                *grip *= power;
            }
            Carrier::Force { power: push, .. } => *push *= power,
            Carrier::Stream { rate, .. } => *rate = (*rate as f32 * power).round().max(1.0) as u32,
            _ => {}
        }
    }
    c.then = cast.then.as_ref().map(|t| empower(t, power, harm));
    Arc::new(c)
}

/// Q readies the next spell a caster knows (Shift+Q the one before); taking
/// up a focus of an element readies the strongest spell of that element it
/// can cast.
pub fn choose(
    keys: Res<ButtonInput<KeyCode>>,
    items: Option<Res<crate::hands::items::Items>>,
    book: Res<super::Spellbook>,
    open: Res<crate::hands::InventoryOpen>,
    mut last: Local<Option<crate::hands::items::ItemId>>,
    mut q: Query<(&mut Caster, &crate::gear::Equipment), With<crate::actors::player::LocalPlayer>>,
) {
    let (Some(items), Ok((mut c, eq))) = (items, q.single_mut()) else { return };
    let n = c.known.len();
    if n == 0 {
        return;
    }
    if keys.just_pressed(KeyCode::KeyQ) && !open.0 {
        let back = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        c.ready = if back { (c.ready + n - 1) % n } else { (c.ready + 1) % n };
    }
    let held = eq.held.map(|s| s.item);
    if held != *last {
        *last = held;
        let focus = held.and_then(|i| match items.def(i).use_ {
            crate::hands::items::Use::Focus { tier, element: Some(e) } => Some((tier, e)),
            _ => None,
        });
        // The strongest it can cast of its element (the first of those).
        if let Some((tier, e)) = focus {
            let spell = |i: usize| book.spells.get(c.known[i]).filter(|s| s.element == Some(e) && s.tier <= tier);
            if let Some(best) = (0..n).filter_map(|i| spell(i).map(|s| (i, s.tier))).max_by_key(|&(i, t)| (t, std::cmp::Reverse(i))) {
                c.ready = best.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hands::items::Use;
    use crate::magic::runes::{self, Runes, RunesFile};

    #[test]
    fn every_spell_reads_and_some_focus_casts_it() {
        let r = Runes::new(crate::data::parse_ron::<RunesFile>(include_str!("../../../../assets/data/runes.ron")).unwrap()).unwrap();
        let file: SpellsFile = crate::data::parse_ron(include_str!("../../../../assets/data/spells.ron")).unwrap();
        let items = crate::hands::items::test_items();
        let foci: Vec<(u8, Option<Element>)> = (0..items.len())
            .filter_map(|i| match items.def(crate::hands::items::ItemId(i as u16)).use_ {
                Use::Focus { tier, element } => Some((tier, element)),
                _ => None,
            })
            .collect();
        for s in &file.spells {
            assert!(!runes::casts(&r, &s.runes).unwrap().is_empty(), "{} casts nothing", s.id);
            assert!(foci.iter().any(|&(t, e)| t >= s.tier && (e.is_none() || e == s.element)), "no focus casts {}", s.id);
        }
        // A fireball from a fire wand (+40% fire) hurts as fire, harder.
        let fireball = file.spells.iter().find(|s| s.id == "fireball").unwrap();
        let cast = &runes::casts(&r, &fireball.runes).unwrap()[0];
        let mut st = Stats::default();
        st.add(Stat::FirePower, 40.0);
        let strong = empower(cast, power(&st, fireball.element), Element::Fire.harm());
        assert_eq!(strong.harm, Harm::Fire);
        let blast = |c: &Cast| c.payloads.iter().find_map(|p| if let Payload::Blast { power, .. } = p { Some(*power) } else { None }).unwrap();
        assert!(blast(&strong) > blast(cast), "{} > {}", blast(&strong), blast(cast));
    }
}
