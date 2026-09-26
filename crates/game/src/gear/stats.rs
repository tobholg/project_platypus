//! Stats (DESIGN §7c): every number gear can change, named once here. A
//! creature's `Stats` are the sum of what it wears and holds (each piece's
//! own stats and its rolled bonuses); the systems that care read them:
//! armour and resistances where it's hurt (`Health::harm`), damage, speed
//! and crits where it swings or looses, spell power where it casts, the
//! rest into its health, mana, stamina, poise and movement (`gear::apply`).
//!
//! Every stat adds up. Some count in points (armour, health, mana), the rest
//! in percent (`+15 %` spell power: 1.15 times).

use serde::Deserialize;

macro_rules! stats {
    ($($(#[$doc:meta])* $name:ident: $label:literal, $percent:literal;)*) => {
        /// A number gear can change.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
        pub enum Stat {
            $($(#[$doc])* $name),*
        }

        impl Stat {
            pub const ALL: &[Stat] = &[$(Stat::$name),*];

            /// What it's called in a tooltip.
            pub fn label(self) -> &'static str {
                match self {
                    $(Stat::$name => $label),*
                }
            }

            /// Counted in percent (else points).
            pub fn percent(self) -> bool {
                match self {
                    $(Stat::$name => $percent),*
                }
            }
        }
    };
}

stats! {
    /// Stops physical hurt: `armor / (armor + 50)` of it (half at 50).
    Armor: "armour", false;
    Health: "health", false;
    /// Damage shrugged off before a blow staggers.
    Poise: "poise", false;
    FireResist: "fire resistance", true;
    FrostResist: "frost resistance", true;
    StormResist: "storm resistance", true;
    AcidResist: "acid resistance", true;
    /// Less hurt from falls.
    FallResist: "fall resistance", true;
    /// Blows and arrows.
    Damage: "damage", true;
    AttackSpeed: "attack speed", true;
    /// A blow or arrow that crits does 1.5 × (plus crit damage).
    Crit: "critical chance", true;
    CritDamage: "critical damage", true;
    Knockback: "knockback", true;
    Stamina: "stamina", false;
    StaminaRegen: "stamina regen", true;
    Mana: "mana", false;
    ManaRegen: "mana regen", true;
    /// Every spell's damage, blasts and heat.
    SpellPower: "spell power", true;
    /// Casts come sooner (a focus's delay and recharge).
    CastSpeed: "cast speed", true;
    FirePower: "fire power", true;
    FrostPower: "frost power", true;
    StormPower: "storm power", true;
    AcidPower: "acid power", true;
    ForcePower: "force power", true;
    GravityPower: "gravity power", true;
    MoveSpeed: "move speed", true;
    JumpHeight: "jump height", true;
    AirJumps: "air jumps", false;
    /// Better finds: rarer gear from what it loots.
    Luck: "luck", false;
}

/// What gear adds up to, on a creature (none worn: all zero).
#[derive(bevy::prelude::Component, Clone, Debug, PartialEq)]
pub struct Stats([f32; Stat::ALL.len()]);

impl Default for Stats {
    fn default() -> Self {
        Stats([0.0; Stat::ALL.len()])
    }
}

impl Stats {
    pub fn get(&self, s: Stat) -> f32 {
        self.0[s as usize]
    }

    pub fn add(&mut self, s: Stat, v: f32) {
        self.0[s as usize] += v;
    }

    /// A percent stat as a multiplier (+15 → 1.15; never below a tenth).
    pub fn mult(&self, s: Stat) -> f32 {
        (1.0 + self.get(s) / 100.0).max(0.1)
    }

    /// A percent stat as a share (+15 → 0.15), capped.
    pub fn share(&self, s: Stat, cap: f32) -> f32 {
        (self.get(s) / 100.0).clamp(0.0, cap)
    }

    /// The stats that aren't zero, in order.
    pub fn nonzero(&self) -> impl Iterator<Item = (Stat, f32)> + '_ {
        Stat::ALL.iter().map(|&s| (s, self.get(s))).filter(|(_, v)| v.abs() > 1e-3)
    }

    /// A blow's (or arrow's) damage and knockback with these stats, and
    /// whether it crit (`roll`: 0..1, from the caller's seed).
    pub fn strike(&self, damage: f32, knock: f32, roll: f32) -> (f32, f32, bool) {
        let crit = roll < self.share(Stat::Crit, 1.0);
        let times = if crit { 1.5 + self.get(Stat::CritDamage) / 100.0 } else { 1.0 };
        (damage * self.mult(Stat::Damage) * times, knock * self.mult(Stat::Knockback), crit)
    }
}

/// A stat and its amount, in words: "+12 armour", "+15% spell power".
pub fn line(s: Stat, v: f32) -> String {
    let sign = if v < 0.0 { "-" } else { "+" };
    let n = v.abs();
    let n = if (n - n.round()).abs() < 0.05 { format!("{:.0}", n) } else { format!("{:.1}", n) };
    if s.percent() { format!("{sign}{n}% {}", s.label()) } else { format!("{sign}{n} {}", s.label()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_add_up_and_read_as_multipliers() {
        let mut s = Stats::default();
        s.add(Stat::Damage, 20.0);
        s.add(Stat::Damage, 5.0);
        assert_eq!(s.get(Stat::Damage), 25.0);
        assert!((s.mult(Stat::Damage) - 1.25).abs() < 1e-6);
        assert_eq!(s.nonzero().count(), 1);
        let (d, k, crit) = s.strike(10.0, 100.0, 0.5);
        assert!((d - 12.5).abs() < 1e-4 && (k - 100.0).abs() < 1e-4 && !crit);
        s.add(Stat::Crit, 60.0);
        assert!(s.strike(10.0, 100.0, 0.5).2, "a roll under the crit chance crits");
        assert_eq!(line(Stat::Armor, 12.0), "+12 armour");
        assert_eq!(line(Stat::SpellPower, 15.0), "+15% spell power");
    }
}
