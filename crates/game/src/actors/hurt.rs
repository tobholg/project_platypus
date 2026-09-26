//! Being hurt, shown: whatever has `Health` flashes red when it loses some,
//! and the damage floats up off it as a number (hits close together add
//! up on one number for a moment, so a flame stream doesn't spray digits). Anything that
//! lowers `hp` shows, whatever did it: spells, blasts, fire, falls.

use bevy::prelude::*;
use platypus_sim::MaterialId;

use super::Health;
use super::Kinematics;
use super::player::LocalPlayer;
use crate::world::SimWorld;

/// What a creature bleeds (from its RON `blood`).
#[derive(Component, Clone, Copy)]
pub struct Bleeds(pub MaterialId);

/// Cells of blood a hit sprays for each point it takes (a bit much: it's
/// more fun, and blood boils, freezes, conducts and washes off like the
/// rest), at most this many a hit...
const BLEED_PER_HP: f32 = 1.2;
const BLEED_MOST: f32 = 70.0;
/// ... and what a death bursts out.
pub const DEATH_BLOOD: usize = 110;

/// Seconds a creature flashes after a hit.
const FLASH: f32 = 0.12;
/// Losing at least this in a tick flashes.
const FLASH_AT: f32 = 2.0;
/// Hits within this long of a number add to it.
const MERGE: f32 = 0.35;
/// How long a number floats, and how fast (cells/s).
const NUMBER_LIFE: f32 = 0.9;
const NUMBER_RISE: f32 = 18.0;
/// Height of the digits, in cells.
const NUMBER_SIZE: f32 = 7.0;

/// What `Health` was last tick, how long it still flashes, and the number
/// it's showing (to add the next hit to).
#[derive(Component)]
pub struct Hurt {
    last: f32,
    pub flash: f32,
    number: Option<Entity>,
}

#[derive(Component)]
pub struct DamageNumber {
    amount: f32,
    age: f32,
}

/// New bodies with health are watched.
pub fn watch(mut commands: Commands, new: Query<(Entity, &Health), Without<Hurt>>) {
    for (e, h) in &new {
        commands.entity(e).insert(Hurt { last: h.hp, flash: 0.0, number: None });
    }
}

type Wounded<'a> = (&'a Health, &'a Kinematics, &'a mut Hurt, Has<LocalPlayer>, Option<&'a Bleeds>);

/// Health that fell since last tick: flash, and a number (or more on the
/// last one). Runs just before deaths, so a killing blow shows too.
pub fn notice(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    mut hurt: Query<Wounded>,
    mut numbers: Query<(&mut DamageNumber, &mut Text2d)>,
    mut shown: Local<u32>,
) {
    for (health, k, mut h, player, bleeds) in &mut hurt {
        let lost = h.last - health.hp;
        h.last = health.hp;
        if lost < 0.01 {
            continue;
        }
        // A real hit flashes and bleeds; burning's trickle only counts up.
        if lost >= FLASH_AT {
            h.flash = FLASH;
            if let Some(&Bleeds(blood)) = bleeds {
                let n = (lost * BLEED_PER_HP).min(BLEED_MOST) as usize;
                sim.world.splash([k.body.pos.x, k.body.pos.y + k.body.half.y * 0.3], blood, n, 1.2 + (lost * 0.02).min(1.2));
            }
        }
        if let Some(e) = h.number
            && let Ok((mut n, mut text)) = numbers.get_mut(e)
            && n.age < MERGE
        {
            n.amount += lost;
            text.0 = digits(n.amount);
            continue;
        }
        let color = if player { Color::srgb(1.0, 0.35, 0.3) } else { Color::srgb(1.0, 0.92, 0.7) };
        // (Side by side, a little, so hits in a row don't print over each other.)
        *shown = shown.wrapping_add(1);
        let at = k.body.pos + Vec2::new((*shown % 5) as f32 * 2.0 - 4.0, k.body.half.y + 4.0);
        let e = commands
            .spawn((
                Name::new("Damage"),
                DamageNumber { amount: lost, age: 0.0 },
                Text2d::new(digits(lost)),
                TextFont { font_size: FontSize::Px(NUMBER_SIZE), ..default() },
                TextColor(color),
                Transform::from_translation(at.extend(20.0)),
            ))
            .id();
        h.number = Some(e);
    }
}

/// Whole points; less than one shows as 1.
fn digits(amount: f32) -> String {
    format!("{}", amount.round().max(1.0) as i32)
}

/// Numbers float up and fade.
pub fn float(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut DamageNumber, &mut Transform, &mut TextColor)>) {
    let dt = time.delta_secs();
    for (e, mut n, mut tf, mut color) in &mut q {
        n.age += dt;
        if n.age >= NUMBER_LIFE {
            commands.entity(e).despawn();
            continue;
        }
        tf.translation.y += NUMBER_RISE * dt * (1.0 - n.age / NUMBER_LIFE);
        color.0.set_alpha(((NUMBER_LIFE - n.age) / 0.3).min(1.0));
    }
}
