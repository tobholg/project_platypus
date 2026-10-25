//! Potions (DESIGN §5): items drunk for an effect. An item whose `use` is
//! `Potion(heal, over, sickness)` is drunk by clicking it in the hand, or
//! the first one carried with H (quick heal, as Terraria's): it mends
//! `heal` health over `over` seconds (0: at once), then potion sickness for
//! `sickness` seconds, when no healing potion works (a status, top right).
//! Not at full health: it'd be wasted. While it mends, the screen's edges
//! glow green (`screen_fx.rs`: full at first, shrinking as it runs out);
//! the flask pops open with a fizz.
//!
//! Something eaten (a raw herb, `eaten`) mends a little the same way:
//! potion sickness doesn't stop it, it brings none, and eating another
//! while it mends starts it over (never cutting short a potion's).
//!
//! A potion can do something else instead (or as well), its `effect`, for a
//! while (a status, top right; sickness doesn't stop them, drinking another
//! starts it over): a fire ward (fire and heat hurt less), regeneration
//! (health back slowly) or an antidote (venom washed off, poison not felt).

use bevy::prelude::*;
use serde::Deserialize;

use crate::creatures::Health;
use crate::creatures::player::LocalPlayer;
use crate::hands::items::{Inventory, Items, Use};
use crate::world::{TICK_HZ, TickSet};

pub struct PotionPlugin;

impl Plugin for PotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Drink>()
            .add_systems(Update, quick_heal)
            .add_systems(FixedUpdate, (drink, mend, sicken, last).chain().in_set(TickSet::Intent));
    }
}

/// Drink what's in this inventory slot (the hand's, or quick heal's).
#[derive(Message, Clone, Copy, Debug)]
pub struct Drink {
    pub who: Entity,
    pub slot: usize,
}

/// Health coming back, `rate` a second, `left` more to come of `total`.
#[derive(Component, Debug)]
pub struct Mending {
    pub rate: f32,
    pub left: f32,
    pub total: f32,
}

/// No healing potion works until it passes.
#[derive(Component, Debug)]
pub struct PotionSickness {
    pub left: f32,
    pub total: f32,
}

const DT: f32 = (1.0 / TICK_HZ) as f32;

/// What a potion does besides healing (items.ron `effect`), for `secs`.
#[derive(Clone, Copy, Debug, Deserialize)]
pub enum Effect {
    /// Fire and heat hurt `fire` (a share) less.
    FireWard { fire: f32, secs: f32 },
    /// `rate` health back a second.
    Regen { rate: f32, secs: f32 },
    /// Venom washed off and kept off; poison not felt.
    Cure { secs: f32 },
}

impl Effect {
    /// For its tooltip.
    pub fn about(&self) -> String {
        match *self {
            Effect::FireWard { fire, secs } => format!("Fire and heat hurt {:.0}% less for {}", fire * 100.0, span(secs)),
            Effect::Regen { rate, secs } => format!("Mends {rate} health a second for {}", span(secs)),
            Effect::Cure { secs } => format!("Washes venom off; poison can't hurt you for {}", span(secs)),
        }
    }
}

fn span(secs: f32) -> String {
    if secs >= 60.0 && (secs % 60.0) < 1.0 { format!("{:.0} min", secs / 60.0) } else { format!("{secs:.0} s") }
}

/// Potions' effects lasting: each, `left` of `total` seconds.
#[derive(Component, Debug)]
pub struct Lasting {
    pub effects: Vec<(Effect, f32, f32)>,
}

/// H drinks the first healing potion carried (not in dev mode, where
/// letters are the dev tools').
fn quick_heal(keys: Res<ButtonInput<KeyCode>>, dev: Res<crate::hands::DevTools>, taken: Res<crate::dev::KeyboardTaken>, items: Option<Res<Items>>, player: Query<(Entity, &Inventory), With<LocalPlayer>>, mut out: MessageWriter<Drink>) {
    if !keys.just_pressed(KeyCode::KeyH) || dev.0 || taken.0 {
        return;
    }
    let (Some(items), Ok((me, inv))) = (items, player.single()) else { return };
    // (A healing potion; none, something to eat.)
    let heals = |eats: bool| inv.slots.iter().position(|s| s.is_some_and(|s| matches!(items.def(s.item).use_, Use::Potion { heal, eaten, .. } if heal > 0.0 && eaten == eats)));
    let first = heals(false).or_else(|| heals(true));
    if let Some(slot) = first {
        out.write(Drink { who: me, slot });
    }
}

type Drinker<'a> = (&'a mut Inventory, &'a Health, Option<&'a PotionSickness>, &'a crate::creatures::Kinematics, Option<&'a mut Lasting>, Option<&'a Mending>);

/// A potion drunk: one gone from its slot, the mending begun, sickness
/// after.
fn drink(mut commands: Commands, mut asks: MessageReader<Drink>, items: Option<Res<Items>>, mut drinkers: Query<Drinker>, mut sounds: MessageWriter<crate::sound::PlaySound>) {
    let Some(items) = items else { return };
    for ask in asks.read() {
        let Ok((mut inv, health, sick, k, lasting, mending)) = drinkers.get_mut(ask.who) else { continue };
        let Some(stack) = inv.slots.get(ask.slot).copied().flatten() else { continue };
        let Use::Potion { heal, over, sickness, effect, eaten } = items.def(stack.item).use_ else { continue };
        let heals = heal > 0.0;
        // (Dead, a healing one with nothing to mend, a drunk one while sick,
        // an eaten one while more is mending: not taken.)
        let blocked = if eaten { mending.is_some_and(|m| m.left > heal) } else { sick.is_some() };
        if health.hp <= 0.0 || heals && (blocked || health.hp >= health.max) {
            sounds.write(crate::sound::PlaySound::here("potion_no"));
            continue;
        }
        inv.take(ask.slot, 1);
        if heals {
            let over = over.max(DT);
            commands.entity(ask.who).insert(Mending { rate: heal / over, left: heal, total: heal });
            if !eaten && sickness > 0.0 {
                commands.entity(ask.who).insert(PotionSickness { left: sickness, total: sickness });
            }
        }
        // (Its effect starts, or starts over.)
        if let Some(e) = effect {
            let secs = match e {
                Effect::FireWard { secs, .. } | Effect::Regen { secs, .. } | Effect::Cure { secs } => secs,
            };
            let now = (e, secs, secs);
            match lasting {
                Some(mut l) => {
                    l.effects.retain(|(had, ..)| std::mem::discriminant(had) != std::mem::discriminant(&e));
                    l.effects.push(now);
                }
                None => {
                    commands.entity(ask.who).insert(Lasting { effects: vec![now] });
                }
            }
        }
        sounds.write(crate::sound::PlaySound::at(if eaten { "eat" } else { "drink" }, k.body.pos));
    }
}

fn mend(mut commands: Commands, mut mending: Query<(Entity, &mut Mending, &mut Health)>) {
    for (e, mut m, mut h) in &mut mending {
        let step = (m.rate * DT).min(m.left);
        h.hp = (h.hp + step).min(h.max);
        m.left -= step;
        if m.left <= 0.0 || h.hp <= 0.0 {
            commands.entity(e).remove::<Mending>();
        }
    }
}

fn sicken(mut commands: Commands, mut sick: Query<(Entity, &mut PotionSickness)>) {
    for (e, mut s) in &mut sick {
        s.left -= DT;
        if s.left <= 0.0 {
            commands.entity(e).remove::<PotionSickness>();
        }
    }
}

type Lasts<'a> = (Entity, &'a mut Lasting, &'a mut Health, Option<&'a mut crate::creatures::body::elements::Coated>);

/// Potions' effects while they last: the ward on it, health back, venom
/// off; gone when they run out.
fn last(mut commands: Commands, coatings: Res<crate::creatures::body::elements::Coatings>, mut q: Query<Lasts>) {
    for (e, mut lasting, mut h, coated) in &mut q {
        for (_, left, _) in &mut lasting.effects {
            *left -= DT;
        }
        lasting.effects.retain(|(_, left, _)| *left > 0.0);
        let (mut fire, mut antidote) = (0.0, false);
        for &(effect, ..) in &lasting.effects {
            match effect {
                Effect::FireWard { fire: f, .. } => fire = f,
                Effect::Regen { rate, .. } if h.hp > 0.0 => h.hp = (h.hp + rate * DT).min(h.max),
                Effect::Regen { .. } => {}
                Effect::Cure { .. } => antidote = true,
            }
        }
        h.ward.drunk_fire = fire;
        h.ward.antidote = antidote;
        // (Venom, and anything else that poisons, washed off.)
        if antidote && let Some(mut c) = coated {
            c.coats.retain(|(n, _)| coatings.by_name.get(n).is_none_or(|r| r.harm != crate::creatures::Harm::Poison));
        }
        if lasting.effects.is_empty() {
            commands.entity(e).remove::<Lasting>();
        }
    }
}
