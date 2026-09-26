//! Rarity and rolled bonuses (DESIGN §7c). A piece of gear found in the
//! world is rolled: a rarity (commoner the likelier; luck and a deeper find
//! shift the odds), its item level (how deep, how dangerous where it was
//! found), and a seed. Its bonuses are worked out from those whenever
//! they're needed, from `bonuses` in gear.ron: a rare piece has two or three
//! of those that suit it, each scaled by its level, and is named for them
//! ("Sturdy iron helm of the Bear"). Uniques are legendary and fixed: their
//! stats are all their own.

use serde::Deserialize;

use super::{GearDef, Slot, Stat};
use crate::hands::items::{ItemDef, Roll, Use};
use platypus_sim::rng::Rng;

/// A rarity: its name, colour, how many bonuses it rolls, and how often it
/// comes up (against the others' weights).
#[derive(Clone, Debug, Deserialize)]
pub struct Rarity {
    pub name: String,
    pub color: (u8, u8, u8),
    pub bonuses: (u32, u32),
    pub weight: f32,
}

/// A bonus a piece can roll: a stat, `base + per_level × level` of it
/// (±25 %), on the kinds of gear in `on`; it names the piece with its
/// `prefix` or `suffix`.
#[derive(Clone, Debug, Deserialize)]
pub struct Bonus {
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub suffix: Option<String>,
    pub stat: Stat,
    pub base: f32,
    #[serde(default)]
    pub per_level: f32,
    /// head, body, hands, legs, feet, trinket, armour (any worn with a
    /// weight), melee, bow, focus; empty: anything.
    #[serde(default)]
    pub on: Vec<String>,
    #[serde(default = "ten")]
    pub weight: f32,
}

fn ten() -> f32 {
    10.0
}

/// The kinds of gear a piece is, for bonuses' `on`.
fn kinds(item: &ItemDef, g: &GearDef) -> Vec<&'static str> {
    let mut k = vec![match g.slot {
        Slot::Head => "head",
        Slot::Body => "body",
        Slot::Hands => "hands",
        Slot::Legs => "legs",
        Slot::Feet => "feet",
        Slot::Trinket => "trinket",
        Slot::Hook => "hook",
        Slot::Held => match item.use_ {
            Use::Melee(_) => "melee",
            Use::Bow(_) => "bow",
            _ => "focus",
        },
    }];
    if g.weight.is_some() {
        k.push("armour");
    }
    k
}

/// A piece's bonuses: which (by index in `bonuses`) and how much.
pub fn bonuses(rarities: &[Rarity], table: &[Bonus], item: &ItemDef, roll: &Roll) -> Vec<(usize, f32)> {
    let (Some(g), Some(r)) = (&item.gear, rarities.get(roll.rarity as usize)) else { return Vec::new() };
    if g.unique || r.bonuses.1 == 0 {
        return Vec::new();
    }
    let kinds = kinds(item, g);
    let mut open: Vec<usize> = (0..table.len()).filter(|&i| table[i].on.is_empty() || table[i].on.iter().any(|o| kinds.contains(&o.as_str()))).collect();
    let mut rng = Rng::seeded(&[roll.seed as u64, roll.level as u64, 0xB0_7A5]);
    let n = r.bonuses.0 + rng.next_u32() % (r.bonuses.1 - r.bonuses.0 + 1);
    let mut out = Vec::new();
    for _ in 0..n {
        let total: f32 = open.iter().map(|&i| table[i].weight).sum();
        if total <= 0.0 {
            break;
        }
        let mut pick = unit(&mut rng) * total;
        let Some(at) = open.iter().position(|&i| {
            pick -= table[i].weight;
            pick <= 0.0
        }) else {
            break;
        };
        let i = open.remove(at);
        let b = &table[i];
        let v = (b.base + b.per_level * roll.level as f32) * (0.75 + 0.5 * unit(&mut rng));
        let v = if v.abs() >= 3.0 { v.round() } else { (v * 10.0).round() / 10.0 };
        out.push((i, v));
    }
    out
}

fn unit(rng: &mut Rng) -> f32 {
    rng.next_u32() as f32 / u32::MAX as f32
}

/// A piece's name: its first prefix bonus, its name, its first suffix.
pub fn name(table: &[Bonus], item: &ItemDef, rolled: &[(usize, f32)]) -> String {
    let prefix = rolled.iter().find_map(|&(i, _)| table[i].prefix.as_deref());
    let suffix = rolled.iter().find_map(|&(i, _)| table[i].suffix.as_deref());
    let mut n = item.name.clone();
    if let Some(p) = prefix {
        n = format!("{p} {}", n.to_lowercase());
    }
    if let Some(s) = suffix {
        n = format!("{n} {s}");
    }
    n
}

/// Roll a piece found at item level `level` by someone with `luck`: its
/// rarity (a unique is always the last, legendary), level and seed.
/// Anything that isn't gear rolls nothing.
pub fn roll(rarities: &[Rarity], item: &ItemDef, level: u8, luck: f32, rng: &mut Rng) -> Roll {
    let Some(g) = &item.gear else { return Roll::default() };
    let seed = rng.next_u32() | 1;
    if g.unique {
        return Roll { rarity: rarities.len().saturating_sub(1) as u8, level, seed };
    }
    // Rarer tiers come up more often deeper, and luckier.
    let boost = 1.0 + level as f32 / 40.0 + luck / 50.0;
    let weights: Vec<f32> = rarities.iter().enumerate().map(|(i, r)| r.weight * boost.powi(i as i32)).collect();
    let total: f32 = weights.iter().sum();
    let mut pick = unit(rng) * total;
    let rarity = weights.iter().position(|w| {
        pick -= w;
        pick <= 0.0
    });
    Roll { rarity: rarity.unwrap_or(0) as u8, level, seed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gear::GearFile;

    fn file() -> GearFile {
        crate::data::parse_ron(include_str!("../../../../assets/data/gear.ron")).unwrap()
    }

    #[test]
    fn rolls_are_repeatable_named_and_suit_the_piece() {
        let f = file();
        let helm = f.items.iter().find(|i| i.id == "iron_helm").unwrap();
        let rare = f.rarities.iter().position(|r| r.name == "Rare").unwrap() as u8;
        let roll = Roll { rarity: rare, level: 20, seed: 12345 };
        let a = bonuses(&f.rarities, &f.bonuses, helm, &roll);
        assert_eq!(a, bonuses(&f.rarities, &f.bonuses, helm, &roll), "the same roll, the same bonuses");
        assert!((2..=3).contains(&a.len()), "a rare piece has two or three: {a:?}");
        for &(i, _) in &a {
            let on = &f.bonuses[i].on;
            assert!(on.is_empty() || on.iter().any(|o| o == "head" || o == "armour"), "{:?} doesn't go on a helm", f.bonuses[i].stat);
        }
        let n = name(&f.bonuses, helm, &a);
        assert!(n.len() > helm.name.len(), "named for its bonuses: {n}");
        let common = Roll { rarity: 0, level: 20, seed: 12345 };
        assert!(bonuses(&f.rarities, &f.bonuses, helm, &common).is_empty());
    }

    #[test]
    fn deeper_and_luckier_finds_are_rarer() {
        let f = file();
        let helm = f.items.iter().find(|i| i.id == "iron_helm").unwrap();
        let mean = |level: u8, luck: f32| {
            let mut rng = Rng::seeded(&[7]);
            (0..4000).map(|_| roll(&f.rarities, helm, level, luck, &mut rng).rarity as f32).sum::<f32>() / 4000.0
        };
        let (shallow, deep, lucky) = (mean(0, 0.0), mean(60, 0.0), mean(0, 50.0));
        assert!(deep > shallow * 1.3 && lucky > shallow * 1.3, "{shallow} {deep} {lucky}");
        assert!(shallow < 1.0, "most finds are common: {shallow}");
    }
}
