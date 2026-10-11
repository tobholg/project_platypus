//! Being hurt, shown (DESIGN §14.3: no health bars). A hit is one of three
//! (`reaction`): hurt (a red flash and blood, both as much as the share of
//! its health it took: a big hit on a small thing is a big reaction, the
//! same on a colossus small), resisted (it took half or less of what was
//! meant: dull sparks and a clang, no flash, no blood) or absorbed (it
//! healed from it: a glow of the hurt's colour, a draught, its dripping
//! stops). Hurt badly, a creature shows it: under half its health it drips
//! (its blood, or dust), under a quarter it falters (slower).
//!
//! The damage floats up off it as a number (hits close together add up on
//! one number for a moment, so a flame stream doesn't spray digits).
//! Anything that lowers `hp` shows, whatever did it: spells, blasts, fire,
//! falls (what isn't a hit flashes and bleeds by how much it took).

use bevy::prelude::*;
use platypus_sim::MaterialId;

use crate::combat::Felt;
use crate::creatures::player::LocalPlayer;
use crate::creatures::{Harm, Health, Kinematics};
use crate::magic::runes::Emitter;
use crate::vfx::Sparks;
use crate::world::SimWorld;

/// How a hit landed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reaction {
    Hurt,
    /// It took no more than `RESISTED` of what was meant.
    Resisted,
    /// It healed from it.
    Absorbed,
}

/// A hit doing this share of what it meant or less is resisted.
const RESISTED: f32 = 0.5;

/// How a hit meant to do `meant` that did `dealt` landed.
pub fn reaction(meant: f32, dealt: f32) -> Reaction {
    if dealt < 0.0 {
        Reaction::Absorbed
    } else if dealt <= meant * RESISTED {
        Reaction::Resisted
    } else {
        Reaction::Hurt
    }
}

/// A hit, shown: what it was to its target (for logs and, later, the
/// player's bestiary: what they saw it shrug off).
#[derive(Message, Clone, Copy, Debug)]
pub struct Reacted {
    pub target: Entity,
    pub reaction: Reaction,
    pub harm: Harm,
    pub share: f32,
}

/// What a creature bleeds (from its RON `blood`).
#[derive(Component, Clone, Copy)]
pub struct Bleeds(pub MaterialId);

/// Cells of blood a hit sprays for each share of its health it takes (a
/// tenth: 30; a bit much: it's more fun, and blood boils, freezes, conducts
/// and washes off like the rest), at most this many a hit...
const BLEED_PER_SHARE: f32 = 300.0;
const BLEED_MOST: f32 = 158.0;
/// ... and what a death bursts out.
pub const DEATH_BLOOD: usize = 248;

/// Seconds a creature flashes after a hit: this much, and this much more
/// for each share of its health the hit took (at most `FLASH_MOST`).
const FLASH: f32 = 0.08;
const FLASH_PER_SHARE: f32 = 0.6;
const FLASH_MOST: f32 = 0.3;
/// Seconds a creature glows that drank a hit, and won't drip after.
pub(crate) const GLOW: f32 = 0.35;
const DRY: f32 = 3.0;
/// Under this share of its health it drips (more often the less it has
/// left: up to `DRIPS` a second); under the other it falters, this slow.
const DRIPPING: f32 = 0.5;
const DRIPS: f32 = 5.0;
const FALTERING: f32 = 0.25;
pub const FALTER_SPEED: f32 = 0.75;
/// Losing at least this in a tick flashes.
const FLASH_AT: f32 = 2.0;
/// Hits within this long of a number add to it.
const MERGE: f32 = 0.35;
/// How long a number floats, and how fast (cells/s).
const NUMBER_LIFE: f32 = 0.9;
const NUMBER_RISE: f32 = 27.0;
/// Height of the digits, in cells.
const NUMBER_SIZE: f32 = 10.5;

/// What `Health` was last tick, how long it still flashes (or glows, and
/// in what colour), the number it's showing (to add the next hit to), what
/// of its loss hits have already shown, and its dripping.
#[derive(Component)]
pub struct Hurt {
    last: f32,
    pub flash: f32,
    pub glow: f32,
    pub glow_color: (u8, u8, u8),
    number: Option<Entity>,
    shown: f32,
    dry: f32,
    drip: f32,
}

/// Hurt badly (under a quarter of its health): slower (`FALTER_SPEED`).
#[derive(Component)]
pub struct Faltering;

#[derive(Component)]
pub struct DamageNumber {
    amount: f32,
    age: f32,
}

/// New bodies with health are watched.
pub fn watch(mut commands: Commands, new: Query<(Entity, &Health), Without<Hurt>>) {
    for (e, h) in &new {
        commands.entity(e).insert(Hurt { last: h.hp, flash: 0.0, glow: 0.0, glow_color: (255, 255, 255), number: None, shown: 0.0, dry: 0.0, drip: 0.0 });
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
    display: Res<crate::display::Display>,
) {
    for (health, k, mut h, player, bleeds) in &mut hurt {
        let lost = h.last - health.hp;
        h.last = health.hp;
        // (What hits took is shown already: `react`.)
        let unshown = lost - h.shown;
        h.shown = 0.0;
        if lost < 0.01 {
            continue;
        }
        // A real loss flashes and bleeds; burning's trickle only counts up.
        if unshown >= FLASH_AT {
            let share = unshown / health.max.max(1.0);
            h.flash = h.flash.max((FLASH + share * FLASH_PER_SHARE).min(FLASH_MOST));
            if let Some(&Bleeds(blood)) = bleeds {
                bleed(&mut sim, k, blood, share);
            }
        }
        // (Numbers only with the option on: `display.rs`.)
        if !display.damage_numbers {
            continue;
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
        let at = k.body.pos + Vec2::new((*shown % 5) as f32 * 3.0 - 6.0, k.body.half.y + 6.0);
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

/// A spray of its blood for a hit of `share` of its health.
fn bleed(sim: &mut SimWorld, k: &Kinematics, blood: platypus_sim::MaterialId, share: f32) {
    let n = (share * BLEED_PER_SHARE).min(BLEED_MOST) as usize;
    if n > 0 {
        sim.world.splash([k.body.pos.x, k.body.pos.y + k.body.half.y * 0.3], blood, n, 1.8 + (share * 6.0).min(1.8));
    }
}

/// Each hit as it landed, shown on what it struck (`reaction`).
pub fn react(mut felt: MessageReader<Felt>, mut sim: ResMut<SimWorld>, mut sparks: ResMut<Sparks>, mut reacted: MessageWriter<Reacted>, mut q: Query<(&Kinematics, &mut Hurt, Option<&Bleeds>)>) {
    for f in felt.read() {
        let Ok((k, mut h, bleeds)) = q.get_mut(f.target) else { continue };
        let reaction = reaction(f.meant, f.dealt);
        match reaction {
            Reaction::Hurt => {
                h.flash = h.flash.max((FLASH + f.share * FLASH_PER_SHARE).min(FLASH_MOST));
                if let Some(&Bleeds(blood)) = bleeds {
                    bleed(&mut sim, k, blood, f.share);
                }
            }
            Reaction::Resisted => sparks.emit(&DULL, DULL.count as usize, f.at, -f.dir, Vec2::ZERO),
            Reaction::Absorbed => {
                h.glow = GLOW;
                h.glow_color = color_of(f.harm);
                h.dry = DRY;
                let mut e = DRANK.clone();
                e.colors = vec![(255, 255, 255), color_of(f.harm)];
                sparks.emit(&e, e.count as usize, k.body.pos, Vec2::Y, Vec2::ZERO);
            }
        }
        h.shown += f.dealt.max(0.0);
        reacted.write(Reacted { target: f.target, reaction, harm: f.harm, share: f.share });
    }
}

/// A kind of hurt's colour (an absorbed hit glows it).
pub(crate) fn color_of(harm: Harm) -> (u8, u8, u8) {
    match harm {
        Harm::Slash => (235, 235, 240),
        Harm::Pierce => (215, 195, 150),
        Harm::Blunt => (165, 150, 140),
        Harm::Fall => (120, 110, 105),
        Harm::Fire => (255, 160, 60),
        Harm::Frost => (170, 220, 255),
        Harm::Storm => (200, 220, 255),
        Harm::Acid => (150, 240, 80),
        Harm::Poison => (160, 210, 80),
        Harm::Radiant => (255, 244, 200),
        Harm::Void => (180, 130, 255),
    }
}

type Wounds<'a> = (Entity, &'a Health, &'a Kinematics, &'a mut Hurt, Option<&'a Bleeds>, Has<Faltering>);

/// Hurt badly, it shows: dripping under half its health (its blood, or
/// dust), faltering under a quarter. (Not the player: the hearts say it.)
pub fn wounded(mut commands: Commands, time: Res<Time<Fixed>>, mut sim: ResMut<SimWorld>, mut sparks: ResMut<Sparks>, mut q: Query<Wounds, Without<LocalPlayer>>) {
    let dt = time.delta_secs();
    for (e, health, k, mut h, bleeds, faltering) in &mut q {
        h.dry = (h.dry - dt).max(0.0);
        let left = (health.hp / health.max.max(1.0)).max(0.0);
        let falters = health.hp > 0.0 && left < FALTERING;
        if falters != faltering {
            if falters {
                commands.entity(e).insert(Faltering);
            } else {
                commands.entity(e).remove::<Faltering>();
            }
        }
        if health.hp <= 0.0 || left >= DRIPPING || h.dry > 0.0 {
            continue;
        }
        h.drip += dt * DRIPS * (DRIPPING - left) / DRIPPING;
        if h.drip < 1.0 {
            continue;
        }
        h.drip -= 1.0;
        // From somewhere about its middle and below.
        let x = k.body.pos.x + (((sim.world.tick() * 7919) % 100) as f32 / 100.0 - 0.5) * k.body.half.x;
        let at = Vec2::new(x, k.body.pos.y - k.body.half.y * 0.2);
        match bleeds {
            Some(&Bleeds(blood)) => sim.world.splash([at.x, at.y], blood, 2, 0.3),
            None => sparks.emit(&DUST, DUST.count as usize, at, Vec2::NEG_Y, Vec2::ZERO),
        }
    }
}

/// Sparks off a resisted hit: dull, few, quick.
static DULL: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 6.0,
    life: (0.06, 0.16),
    colors: vec![(190, 186, 176), (110, 106, 100)],
    speed: 120.0,
    spread: 1.0,
    gravity: 300.0,
    drag: 5.0,
    size: 1.0,
    jitter: 0.0,
    glow: false,
});

/// A drunk hit: motes of its colour rising off it.
static DRANK: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 14.0,
    life: (0.3, 0.7),
    colors: vec![(255, 255, 255)],
    speed: 40.0,
    spread: 1.2,
    gravity: -60.0,
    drag: 2.0,
    size: 1.0,
    jitter: 0.0,
    glow: true,
});

/// What falls off a bloodless thing hurt badly.
static DUST: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 2.0,
    life: (0.3, 0.6),
    colors: vec![(170, 164, 150), (110, 106, 98)],
    speed: 10.0,
    spread: 0.4,
    gravity: 120.0,
    drag: 1.0,
    size: 1.0,
    jitter: 0.0,
    glow: false,
});

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
