//! The kick (DESIGN §13.1, Noita's): F kicks what's in front of your feet.
//! Things take it by their weight: an object (a log, a casting: `object`
//! materials, lifted out whole as a body) is sent along, the heavier the
//! slower; loose powder and rubble fly (gold with them); items, bodies,
//! barrels and carts are sent flying (by their size); a small creature is
//! shoved. Ground takes nothing: a kick into rock is just a kick.
//!
//! It shows: the player's `kick` clip (the knee up, the leg out, back), the
//! blow landing as the leg is out (`LANDS` after the key), a puff of dust
//! at the foot and, when it moved something, a small jolt of the camera.

use bevy::prelude::*;
use platypus_sim::CellPos;

use crate::actors::player::LocalPlayer;
use crate::actors::{Creature, Kinematics};
use crate::props::Thrown;
use crate::world::SimWorld;

pub struct KickPlugin;

impl Plugin for KickPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Kick>().add_systems(Update, ((kick_key, kick).chain().run_if(not(crate::hands::dev_tools)), kick_done));
    }
}

/// Seconds between kicks, and from the key to the blow (the clip's second
/// frame: the leg out).
const COOLDOWN: f32 = 0.45;
const LANDS: f32 = 1.0 / 16.0;
/// The camera's jolt when a kick moves something.
const JOLT: f32 = 0.12;
/// How far in front of the feet the kick reaches, and how high (cells).
const REACH: f32 = 8.0;
const HEIGHT: f32 = 7.0;
/// A kick's impulse into the world (mass in cells × cells a tick: a
/// 300-cell log leaves at a cell a tick, ~60 cells a second).
const IMPULSE: f32 = 300.0;
/// What things on the move take (cells a second), before their size.
const THROW: Vec2 = Vec2::new(170.0, 110.0);
/// Creatures smaller than this (cells of box) are shoved; bigger ones stand.
const SHOVE_BELOW: f32 = 60.0;

/// A kick, by the local player (a key, or a scenario).
#[derive(Message, Clone, Copy, Debug)]
pub struct Kick;

fn kick_key(keys: Res<ButtonInput<KeyCode>>, open: Res<crate::hands::InventoryOpen>, mut kicks: MessageWriter<Kick>) {
    if keys.just_pressed(KeyCode::KeyF) && !open.0 {
        kicks.write(Kick);
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn kick(
    time: Res<Time>,
    mut kicks: MessageReader<Kick>,
    mut sim: ResMut<SimWorld>,
    mut timing: Local<(f32, Option<f32>)>,
    mut player: Query<(&Kinematics, Option<&mut crate::actors::animation::Animator>), With<LocalPlayer>>,
    mut things: Query<(&mut Kinematics, Option<&Creature>, Has<Thrown>, Has<crate::actors::explosive::Explosive>), Without<LocalPlayer>>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut trauma: ResMut<crate::fx::Trauma>,
) {
    let dt = time.delta_secs();
    let (ready, lands) = &mut *timing;
    *ready -= dt;
    let Ok((me, anim)) = player.single_mut() else { return };
    // The key: the leg comes up and out; the blow lands as it's out.
    if kicks.read().count() > 0 && *ready <= 0.0 {
        *ready = COOLDOWN;
        *lands = Some(LANDS);
        if let Some(mut anim) = anim
            && anim.def.animations.contains_key("kick")
        {
            anim.play("kick");
        }
    }
    let Some(left) = lands.as_mut() else { return };
    *left -= dt;
    if *left > 0.0 {
        return;
    }
    *lands = None;
    let facing = if me.loco.facing < 0.0 { -1.0 } else { 1.0 };
    let feet = me.body.pos - Vec2::new(0.0, me.body.half.y);
    // The box in front of the feet.
    let near = feet.x + facing * (me.body.half.x - 1.0);
    let far = feet.x + facing * (me.body.half.x + REACH);
    let (x0, x1) = (near.min(far), near.max(far));
    let (y0, y1) = (feet.y - 1.0, feet.y + HEIGHT);
    let (moved, specks) = sim.world.kick(CellPos::from_world(x0, y0), CellPos::from_world(x1, y1), [facing * 1.6, 0.4], IMPULSE);
    let mut hit = moved + specks;
    for (mut k, creature, thrown, prop) in &mut things {
        let p = k.body.pos;
        let (lo, hi) = (p - k.body.half, p + k.body.half);
        if hi.x < x0 || lo.x > x1 || hi.y < y0 || lo.y > y1 {
            continue;
        }
        // (Size: a small thing flies, a big one barely moves.)
        let area = (k.body.half.x * k.body.half.y * 4.0).max(1.0);
        let push = Vec2::new(facing * THROW.x, THROW.y) * (12.0 / area).sqrt().clamp(0.15, 1.6);
        if thrown {
            k.body.vel += push;
            hit += 1;
        } else if prop {
            // A barrel, a cart: no mind to resist, sent by its weight.
            let k = &mut *k;
            k.loco.knock(&mut k.body, Vec2::new(facing * THROW.x, THROW.y) * 0.9, 0.8);
            hit += 1;
        } else if creature.is_some() && area < SHOVE_BELOW {
            let k = &mut *k;
            k.loco.knock(&mut k.body, push, 0.35);
            hit += 1;
        }
    }
    let at = Vec2::new((x0 + x1) / 2.0, feet.y + 2.0);
    sounds.write(crate::sound::PlaySound::at(if hit > 0 { "kick" } else { "kick_miss" }, at));
    // Dust at the foot (more when it met something), and a jolt.
    let foot = Vec2::new(feet.x + facing * (me.body.half.x + 4.0), feet.y + 5.0);
    sparks.emit(&dust(), if hit > 0 { 10 } else { 4 }, foot, Vec2::new(facing, 0.6), Vec2::ZERO);
    if hit > 0 {
        trauma.0 = (trauma.0 + JOLT).min(1.0);
    }
}

/// The kick's clip, let go once it's played.
fn kick_done(mut q: Query<&mut crate::actors::animation::Animator, With<LocalPlayer>>) {
    for mut anim in &mut q {
        if anim.force.as_deref() == Some("kick") && anim.finished() {
            anim.force = None;
        }
    }
}

/// A kick's puff of dust.
fn dust() -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter {
        count: 1.0,
        life: (0.2, 0.45),
        colors: vec![(176, 166, 150), (128, 120, 108)],
        speed: 45.0,
        spread: 1.1,
        gravity: 30.0,
        drag: 3.0,
        size: 1.0,
        jitter: 0.0,
        glow: false,
    }
}
