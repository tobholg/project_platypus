//! Spells and foci (DESIGN §7b, §7c). A spell is its own thing, made of
//! runes (`assets/data/spells.ron`, recipes any focus can hold); wands and
//! staffs are foci, held gear that holds spells up to its tier (a wand is
//! tier 1, a staff tier 2), and the focus's stats
//! (spell power, cast speed, an element's power: a fire wand's +40% fire)
//! make it stronger. Every spell has an element (or none: arcane); its
//! element's power and spell power scale what it does, and its hurt is of
//! its element (resisted as such).
//!
//! A focus holds its spells (changed 2026-09-26 from a spellbook with a
//! spell ready): a wand one, a staff two (left and right button); written in
//! items.ron, or, for a focus found in the world, rolled from what suits it.

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

/// The spells a focus holds (by index in the spellbook): as written, or,
/// for one found in the world (a rolled seed), picked from what its element
/// and tier allow (a wand one, a staff two, different ones where it can).
pub fn focus_spells(spells: &[SpellDef], tier: u8, element: Option<Element>, written: &[String], roll: &crate::hands::items::Roll) -> Vec<usize> {
    let fixed: Vec<usize> = written.iter().filter_map(|id| spells.iter().position(|s| &s.id == id)).collect();
    let pool: Vec<usize> = (0..spells.len()).filter(|&i| spells[i].tier <= tier && (element.is_none() || spells[i].element == element)).collect();
    if roll.seed == 0 || pool.is_empty() {
        return fixed;
    }
    let mut rng = platypus_sim::rng::Rng::seeded(&[roll.seed as u64, 0x5_9E11]);
    let mut pool = pool;
    let n = if tier >= 2 { 2 } else { 1 };
    let mut out = Vec::new();
    for _ in 0..n {
        if pool.is_empty() {
            break;
        }
        out.push(pool.remove(rng.next_u32() as usize % pool.len()));
    }
    // (A staff's best spell first: its left button.)
    out.sort_by_key(|&i| std::cmp::Reverse(spells[i].tier));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hands::items::Use;
    use crate::magic::runes::{self, Runes, RunesFile};

    #[test]
    fn every_spell_reads_and_every_focus_holds_what_it_can_cast() {
        let r = Runes::new(crate::data::parse_ron::<RunesFile>(include_str!("../../../../assets/data/runes.ron")).unwrap()).unwrap();
        let file: SpellsFile = crate::data::parse_ron(include_str!("../../../../assets/data/spells.ron")).unwrap();
        let items = crate::hands::items::test_items();
        for s in &file.spells {
            assert!(!runes::casts(&r, &s.runes).unwrap().is_empty(), "{} casts nothing", s.id);
        }
        // Every focus holds spells it can cast (a wand one, a staff two); a
        // found one rolls as many of its element.
        for i in 0..items.len() {
            let def = items.def(crate::hands::items::ItemId(i as u16));
            let Use::Focus { tier, element, spells } = &def.use_ else { continue };
            let want = if *tier >= 2 { 2 } else { 1 };
            let written = focus_spells(&file.spells, *tier, *element, spells, &Default::default());
            assert_eq!(written.len(), want, "{} holds {} spells", def.id, written.len());
            assert!(written.iter().all(|&k| file.spells[k].tier <= *tier), "{} holds a spell beyond it", def.id);
            for seed in 1..20 {
                let found = focus_spells(&file.spells, *tier, *element, spells, &crate::hands::items::Roll { rarity: 1, level: 5, seed });
                assert!(!found.is_empty() && found.len() <= want, "{} found holds {found:?}", def.id);
                assert!(found.iter().all(|&k| file.spells[k].tier <= *tier && (element.is_none() || file.spells[k].element == *element)), "{}: {found:?}", def.id);
                if found.len() == 2 {
                    assert_ne!(found[0], found[1]);
                }
            }
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
