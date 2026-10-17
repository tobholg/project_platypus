//! Shields, Elden Ring's way: worn in the off hand (gear.ron `shield`),
//! raised while the right button is held (when what's in hand has no use of
//! its own for it: not a focus, not a vessel).
//!
//! - Raised, it costs stamina (`drain` a second; none comes back while it's
//!   up) and you walk slower.
//! - A blow from the side it faces (the side you face) is caught: it costs
//!   stamina by its weight (its damage × the shield's `stability`), and
//!   only a little of it gets through (`guard`: the share it stops).
//! - A blow that costs more stamina than you have left breaks your guard:
//!   what the stamina couldn't pay for gets through, in proportion (half
//!   its cost unpaid: half its damage), you stagger, and the shield's down
//!   a moment.
//! - From behind, it's no help.
//!
//! What's caught is what lands as a blow (`combat::Hit`: blades, claws,
//! arrows); spells and fire still find you.

use bevy::prelude::*;
use serde::Deserialize;

use crate::combat::Stamina;
use crate::creatures::Kinematics;
use crate::creatures::body::animation::HandPos;
use crate::creatures::player::LocalPlayer;
use crate::gear::{Equipment, OFF_HAND};
use crate::hands::items::{Inventory, Items, Use};

/// A shield's make (gear.ron, `gear.shield`).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct ShieldDef {
    /// The share of a caught blow it stops (the rest is chip).
    pub guard: f32,
    /// Stamina a caught blow costs, per point of its damage.
    pub stability: f32,
    /// Stamina a second while raised.
    pub drain: f32,
}

/// The player's guard: raised now; broken (seconds till it can rise again).
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct Guard {
    pub raised: bool,
    pub broken: f32,
}

/// Walking with a shield up: this share of the speed.
pub const GUARD_PACE: f32 = 0.5;
/// A broken guard: down this long, and you stagger this long.
const BROKEN: f32 = 1.2;
pub const BROKEN_STAGGER: f32 = 0.5;

/// The shield worn in the off hand, if any.
pub fn worn(items: &Items, eq: &Equipment) -> Option<ShieldDef> {
    eq.worn[OFF_HAND].and_then(|s| items.def(s.item).gear.as_ref()?.shield)
}

/// The right button raises it, while there's stamina and the guard isn't
/// broken; up, it drinks stamina.
#[allow(clippy::too_many_arguments)]
pub fn raise(
    mut commands: Commands,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    items: Option<Res<Items>>,
    hand: Res<crate::hands::Hand>,
    (open, over, dev): (Res<crate::hands::ui::InventoryOpen>, Res<crate::dev::PointerOverUi>, Res<crate::hands::DevTools>),
    mut player: Query<(Entity, &Equipment, &Inventory, Option<&mut Guard>, Option<&mut Stamina>), With<LocalPlayer>>,
) {
    let Some(items) = items else { return };
    let Ok((e, eq, inv, guard, stamina)) = player.single_mut() else { return };
    let Some(mut guard) = guard else {
        commands.entity(e).insert(Guard::default());
        return;
    };
    let dt = time.delta_secs();
    guard.broken = (guard.broken - dt).max(0.0);
    let shield = worn(&items, eq);
    // (What's in hand that wants the right button keeps it: a focus's
    // second spell, a vessel's scoop.)
    let busy = inv.slots.get(hand.active()).copied().flatten().is_some_and(|s| matches!(items.def(s.item).use_, Use::Focus { .. } | Use::Vessel { .. }));
    let Some(mut stamina) = stamina else { return };
    let want = shield.is_some() && mouse.pressed(MouseButton::Right) && !busy && !open.0 && !over.0 && !dev.0;
    guard.raised = want && guard.broken <= 0.0 && stamina.cur > 0.0;
    if let (true, Some(s)) = (guard.raised, shield) {
        stamina.spend(s.drain * dt);
    }
}

/// A blow against the guard: what gets through, the stamina it costs, and
/// whether it broke it. `front`: it came from the side the shield faces.
pub fn catch(shield: ShieldDef, stamina: f32, damage: f32) -> (f32, f32, bool) {
    let cost = damage * shield.stability;
    if cost <= stamina {
        // Caught: a little chips through.
        return (damage * (1.0 - shield.guard), cost, false);
    }
    // Broken: what stamina couldn't pay for gets through, in proportion.
    let unpaid = (cost - stamina) / cost.max(0.001);
    (damage * unpaid, stamina, true)
}

/// A broken guard: down for a moment.
pub fn break_guard(guard: &mut Guard) {
    guard.raised = false;
    guard.broken = BROKEN;
}

/// The shield shown on the player: lowered, at its off hand; raised, held
/// up in front of it, on the side it faces. Its item's picture.
#[derive(Component)]
pub struct ShieldSprite;

#[allow(clippy::type_complexity)]
pub fn draw(
    mut commands: Commands,
    items: Option<Res<Items>>,
    icons: Option<Res<crate::hands::icons::Icons>>,
    player: Query<(Entity, &Kinematics, &Equipment, Option<&Guard>, Option<&HandPos>), With<LocalPlayer>>,
    mut shown: Query<(Entity, &mut Transform, &mut Sprite, &ChildOf), With<ShieldSprite>>,
) {
    let (Some(items), Some(icons)) = (items, icons) else { return };
    let Ok((me, k, eq, guard, hand)) = player.single() else { return };
    let item = eq.worn[OFF_HAND].filter(|s| items.def(s.item).gear.as_ref().is_some_and(|g| g.shield.is_some()));
    let Some(item) = item else {
        for (e, ..) in &shown {
            commands.entity(e).despawn();
        }
        return;
    };
    let Some(picture) = icons.get(item.item) else { return };
    let facing = k.loco.facing;
    let raised = guard.is_some_and(|g| g.raised);
    // Raised: before it, chest high; lowered: at the off hand, behind.
    let at = if raised { Vec3::new(facing * (k.body.half.x * 0.7 + 1.0), k.body.half.y * 0.15, 0.06) } else { hand.and_then(|h| h.off).map_or(Vec3::new(-facing * 2.0, 0.0, -0.05), |o| o + Vec3::new(0.0, -1.0, -0.05)) };
    match shown.iter_mut().next() {
        Some((_, mut tf, mut sprite, _)) => {
            tf.translation = at;
            sprite.flip_x = facing < 0.0;
            if sprite.image != *picture {
                sprite.image = picture.clone();
            }
        }
        None => {
            commands.spawn((Name::new("Shield"), ShieldSprite, Sprite { image: picture.clone(), custom_size: Some(Vec2::splat(12.0)), ..default() }, Transform::from_translation(at), ChildOf(me)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WOOD: ShieldDef = ShieldDef { guard: 0.85, stability: 1.4, drain: 6.0 };

    /// Caught with stamina to spare: a little through, the cost paid. Short
    /// of it: broken, and through in proportion to what wasn't paid.
    #[test]
    fn a_blow_is_caught_or_breaks_the_guard_by_the_stamina_left() {
        let (through, cost, broke) = catch(WOOD, 50.0, 20.0);
        assert!(!broke && (cost - 28.0).abs() < 1e-3 && (through - 3.0).abs() < 1e-3, "{through} {cost}");
        // 28 to pay, 14 left: half unpaid, half the blow through.
        let (through, cost, broke) = catch(WOOD, 14.0, 20.0);
        assert!(broke && (cost - 14.0).abs() < 1e-3 && (through - 10.0).abs() < 1e-3, "{through} {cost}");
    }
}
