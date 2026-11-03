//! Guns (the space arc's star-forged weapons): held in the hand and fired
//! at the cursor, each shot a spell (`spells.ron`: what flies, what it does
//! where it lands) paid for with a round of ammo, not mana: from the ammo
//! slots first, then the pack (`gear::Equipment::take_ammo`). A repeater
//! fires as long as it's held; a launcher once a press. Each shot kicks
//! its shooter back (`kick`): weightless, a held repeater drives you
//! slowly backwards, a rocket shoves you hard.
//!
//! Gunpowder and rockets carry their own fire: they work in the void.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;

use crate::hands::items::{Inventory, ItemId, Items, Use};
use crate::magic::runes::{Cast, Emitter};
use crate::world::{TICK_HZ, TickSet};

pub struct GunsPlugin;

impl Plugin for GunsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Shoot>().init_resource::<Ready>().add_systems(FixedUpdate, shoot.in_set(TickSet::Intent));
    }
}

/// The gun in hand fired (the trigger held): `fresh`, pressed just now.
#[derive(Message, Clone, Copy, Debug)]
pub struct Shoot {
    pub shooter: Entity,
    pub item: ItemId,
    pub from: Vec2,
    pub at: Vec2,
    pub fresh: bool,
}

/// Seconds till each shooter's next shot.
#[derive(Resource, Default)]
struct Ready(HashMap<Entity, f32>);

/// A gun firing: its rate kept, a round of ammo taken, its shot sent off
/// at the cursor (spread a little), its kick, its flash and its sound.
#[allow(clippy::too_many_arguments)]
fn shoot(
    mut shots: MessageReader<Shoot>,
    items: Option<Res<Items>>,
    book: Res<crate::magic::Spellbook>,
    mut ready: ResMut<Ready>,
    mut firing: ResMut<crate::magic::Firing>,
    mut shooters: Query<(&mut crate::gear::Equipment, Option<&mut Inventory>)>,
    mut recoils: MessageWriter<crate::drift::Recoil>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut casts: Local<HashMap<String, Option<Arc<Cast>>>>,
    mut tick: Local<u64>,
) {
    let dt = 1.0 / TICK_HZ as f32;
    for t in ready.0.values_mut() {
        *t -= dt;
    }
    ready.0.retain(|_, t| *t > -1.0);
    *tick += 1;
    let Some(items) = items else { return };
    for s in shots.read() {
        let Use::Gun { shot, ammo, rate, auto, spread, kick, sound } = &items.def(s.item).use_ else { continue };
        // (Held down: as fast as it fires; else a press a shot.)
        if !*auto && !s.fresh {
            continue;
        }
        if ready.0.get(&s.shooter).is_some_and(|t| *t > 0.0) {
            continue;
        }
        let Some(cast) = casts
            .entry(shot.clone())
            .or_insert_with(|| {
                let def = book.spells.iter().find(|d| d.id == *shot)?;
                crate::magic::runes::casts(&book.runes, &def.runes).ok()?.into_iter().next()
            })
            .clone()
        else {
            warn!("guns: no shot `{shot}` in spells.ron");
            continue;
        };
        let Ok((mut eq, inv)) = shooters.get_mut(s.shooter) else { continue };
        // Out of ammo: a dry click, now and then.
        if eq.take_ammo(inv.map(|i| i.into_inner()), &items, ammo).is_none() {
            if s.fresh {
                sounds.write(crate::sound::PlaySound::at("gun_dry", s.from).volume(0.5));
            }
            ready.0.insert(s.shooter, 0.25);
            continue;
        }
        ready.0.insert(s.shooter, 1.0 / rate.max(0.1));
        let aim = (s.at - s.from).normalize_or(Vec2::X);
        // (Spread: a little either way, the same for everyone on the tick.)
        let roll = platypus_sim::rng::hash(&[*tick, s.shooter.to_bits(), 0x6075]) as f32 / u64::MAX as f32 - 0.5;
        let dir = Vec2::from_angle((roll * 2.0 * spread).to_radians()).rotate(aim);
        let from = s.from + aim * MUZZLE;
        firing.0.push(crate::magic::Fire { cast, caster: s.shooter, from, dir, reach: s.at.distance(s.from), pace: 1.0 });
        if *kick > 0.0 {
            recoils.write(crate::drift::Recoil { who: s.shooter, from: s.from, push: -aim * *kick });
        }
        sparks.emit(&FLASH, FLASH.count as usize, from, aim, Vec2::ZERO);
        if let Some(name) = sound {
            sounds.write(crate::sound::PlaySound::at(name.clone(), from));
        }
    }
}

/// The muzzle: this far out along the aim from the hand (cells).
const MUZZLE: f32 = 14.0;

/// A shot's flash at the muzzle: gold and cyan sparks, out ahead.
static FLASH: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 6.0,
    life: (0.04, 0.12),
    colors: vec![(255, 250, 225), (255, 205, 110), (150, 225, 255)],
    speed: 160.0,
    spread: 0.5,
    gravity: 0.0,
    drag: 8.0,
    size: 1.0,
    jitter: 0.0,
    glow: true,
});
