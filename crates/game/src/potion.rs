//! Potions (DESIGN §5): items drunk for an effect. An item whose `use` is
//! `Potion(heal, over, sickness)` is drunk by clicking it in the hand, or
//! the first one carried with H (quick heal, as Terraria's): it mends
//! `heal` health over `over` seconds (0: at once), then potion sickness for
//! `sickness` seconds, when no healing potion works (a status, top right).
//! Not at full health: it'd be wasted. While it mends, the screen's edges
//! glow green (`screen_fx.rs`: full at first, shrinking as it runs out);
//! the flask pops open with a fizz.

use bevy::prelude::*;

use crate::creatures::Health;
use crate::creatures::player::LocalPlayer;
use crate::hands::items::{Inventory, Items, Use};
use crate::world::{TICK_HZ, TickSet};

pub struct PotionPlugin;

impl Plugin for PotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Drink>()
            .add_systems(Update, quick_heal)
            .add_systems(FixedUpdate, (drink, mend, sicken).chain().in_set(TickSet::Intent));
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

/// H drinks the first healing potion carried (not in dev mode, where
/// letters are the dev tools').
fn quick_heal(keys: Res<ButtonInput<KeyCode>>, dev: Res<crate::hands::DevTools>, taken: Res<crate::dev::KeyboardTaken>, items: Option<Res<Items>>, player: Query<(Entity, &Inventory), With<LocalPlayer>>, mut out: MessageWriter<Drink>) {
    if !keys.just_pressed(KeyCode::KeyH) || dev.0 || taken.0 {
        return;
    }
    let (Some(items), Ok((me, inv))) = (items, player.single()) else { return };
    let first = inv.slots.iter().position(|s| s.is_some_and(|s| matches!(items.def(s.item).use_, Use::Potion { heal, .. } if heal > 0.0)));
    if let Some(slot) = first {
        out.write(Drink { who: me, slot });
    }
}

type Drinker<'a> = (&'a mut Inventory, &'a Health, Option<&'a PotionSickness>, &'a crate::creatures::Kinematics);

/// A potion drunk: one gone from its slot, the mending begun, sickness
/// after.
fn drink(mut commands: Commands, mut asks: MessageReader<Drink>, items: Option<Res<Items>>, mut drinkers: Query<Drinker>, mut sounds: MessageWriter<crate::sound::PlaySound>) {
    let Some(items) = items else { return };
    for ask in asks.read() {
        let Ok((mut inv, health, sick, k)) = drinkers.get_mut(ask.who) else { continue };
        let Some(stack) = inv.slots.get(ask.slot).copied().flatten() else { continue };
        let Use::Potion { heal, over, sickness } = items.def(stack.item).use_ else { continue };
        // (Sick, or nothing to mend: not drunk.)
        if sick.is_some() || health.hp >= health.max || health.hp <= 0.0 {
            sounds.write(crate::sound::PlaySound::here("potion_no"));
            continue;
        }
        inv.take(ask.slot, 1);
        let over = over.max(DT);
        commands.entity(ask.who).insert((Mending { rate: heal / over, left: heal, total: heal }, PotionSickness { left: sickness, total: sickness.max(0.01) }));
        sounds.write(crate::sound::PlaySound::at("drink", k.body.pos));
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
