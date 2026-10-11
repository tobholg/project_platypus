//! Getting up again (a skeleton, `rises` in its creature file). Killed, its
//! bones fall in a heap instead of a body; a while later (`after`) the heap
//! shakes, pulls itself back together and it stands, at `health` of its
//! health. A blow of one of `by`'s kinds breaks it for good: on the heap
//! (a hammer, holy light), or as the killing blow (blows only: the knock
//! into the ground, a fall, burning don't). Then it dies as anything
//! dies: its body, its loot, the kill counted (`deaths`).
//!
//! The heap is the creature itself, put down: it does nothing (`Dormant`),
//! its box shrinks to a heap's, it falls and is thrown as anything is, and
//! other blows clatter off it (its health is kept up). What's drawn is its
//! picture as it fell, laid on its back (`Heap`), in place of its own.

use bevy::prelude::*;
use platypus_physics::body::overlaps_solid;
use serde::Deserialize;

use super::animation::CreatureSprite;
use crate::creatures::nature::Harm;
use crate::combat::Felt;
use crate::creatures::{Controls, Dormant, Health, Kinematics, WorldGrid};
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// A heap's height (cells).
const HEAP_HALF: f32 = 4.5;
/// The heap shakes this long before it stands (seconds): a warning.
const SHAKE: f32 = 1.0;
/// Standing up where it can't (under a low roof), it tries again after this.
const RETRY: f32 = 0.5;
/// The heap's colours, darkened (as a body's).
const DIM: f32 = 0.78;
/// Health a heap is kept at, so blows that can't break it don't kill it.
const KEPT: f32 = 1.0;
/// Its dust as it falls and as it stands (cells of its blood: ash).
const DUST_FALL: usize = 14;
const DUST_RISE: usize = 10;

/// How it gets up again (a creature file's `rises`).
#[derive(Clone, Debug, Deserialize)]
pub struct RiseDef {
    /// Seconds a heap before it stands.
    pub after: f32,
    /// Of its health it stands with.
    pub health: f32,
    /// Kinds of hurt that break it for good.
    pub by: Vec<Harm>,
}

#[derive(Component, Clone)]
pub struct Rises(pub RiseDef);

/// Down in a heap: how long before it stands, its standing box, its picture.
#[derive(Component)]
pub struct Collapsed {
    left: f32,
    half: Vec2,
    heap: Entity,
}

/// A heap's picture (its own entity, where its creature is).
#[derive(Component)]
pub struct Heap {
    of: Entity,
}

type Rising<'a> = (Entity, &'a mut Health, &'a mut Kinematics, &'a Rises, Option<&'a mut Collapsed>, &'a mut Controls, Option<&'a Children>, Option<&'a super::hurt::Bleeds>);

/// Just before deaths: one that would die falls in a heap instead (unless
/// the blow was one that breaks it); a heap struck by one that breaks it
/// dies; one struck by anything else is kept from dying.
#[allow(clippy::too_many_arguments)]
pub fn collapse(
    mut commands: Commands,
    mut felt: MessageReader<Felt>,
    mut sim: ResMut<SimWorld>,
    mut q: Query<Rising>,
    sprites: Query<(&Sprite, &Transform), With<CreatureSprite>>,
    defs: Query<&super::animation::Animator>,
) {
    // (Only blows break it, a hammer's, holy light's: not the knock into
    // the ground a killing blow throws it into, nor a fall.)
    let blows: Vec<Felt> = felt.read().copied().collect();
    for (e, mut h, mut k, rises, collapsed, mut controls, children, bleeds) in &mut q {
        let breaks = blows.iter().any(|b| b.target == e && b.dealt > 0.0 && rises.0.by.contains(&b.harm));
        match collapsed {
            // A heap: broken (it dies now, `deaths`), or kept.
            Some(_) => {
                if breaks {
                    h.hp = h.hp.min(0.0);
                } else {
                    h.hp = h.hp.max(KEPT);
                }
            }
            None if h.hp <= 0.0 && !breaks => {
                h.hp = KEPT;
                let half = k.body.half;
                // Its box shrinks to a heap's, on the same ground.
                let drop = (half.y - HEAP_HALF).max(0.0);
                k.body.pos.y -= drop;
                k.prev_pos.y -= drop;
                k.body.half.y = half.y.min(HEAP_HALF);
                controls.0 = default();
                // Its picture as it fell, laid on its back, head the way it faced from.
                let facing = k.loco.facing;
                let turn = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2 * facing);
                let shown = children.and_then(|ch| ch.iter().find_map(|c| sprites.get(c).ok()));
                let z = defs.get(e).map_or(10.0, |a| a.def.z) - 0.5;
                let mut heap = commands.spawn((Name::new("Heap of bones"), Transform::from_translation(k.body.pos.extend(z)).with_rotation(turn), Visibility::default()));
                if let Some((sprite, tf)) = shown {
                    let mut sprite = sprite.clone();
                    sprite.color = Color::srgb(DIM, DIM, DIM);
                    // (Turned with the heap, about the body's middle.)
                    heap.with_child((sprite, Transform::from_translation(tf.translation.with_z(0.0))));
                }
                let heap = heap.insert(Heap { of: e }).id();
                commands.entity(e).insert((Collapsed { left: rises.0.after, half, heap }, Dormant, Visibility::Hidden));
                if let Some(&super::hurt::Bleeds(dust)) = bleeds {
                    sim.world.splash([k.body.pos.x, k.body.pos.y], dust, DUST_FALL, 2.4);
                }
            }
            None => {}
        }
    }
}

type Down<'a> = (Entity, &'a mut Collapsed, &'a mut Health, &'a mut Kinematics, &'a Rises, Option<&'a super::hurt::Bleeds>);

/// A heap whose time is up stands (if there's room), at `health` of its health.
pub fn stand(mut commands: Commands, mut sim: ResMut<SimWorld>, mut q: Query<Down>, mut sounds: MessageWriter<crate::sound::PlaySound>) {
    for (e, mut c, mut h, mut k, rises, bleeds) in &mut q {
        c.left -= DT;
        if c.left > 0.0 {
            continue;
        }
        // Standing: its box back up from the same ground, if that's clear.
        let rise = c.half.y - k.body.half.y;
        let pos = k.body.pos + Vec2::new(0.0, rise);
        let mut standing = k.body;
        standing.half = c.half;
        let (lo, hi) = standing.cells_at(pos);
        if overlaps_solid(&WorldGrid(&sim.world), lo, hi) {
            c.left = RETRY;
            continue;
        }
        k.body.half = c.half;
        k.body.pos = pos;
        k.prev_pos.y += rise;
        k.body.vel = Vec2::ZERO;
        h.hp = h.max * rises.0.health;
        commands.entity(c.heap).despawn();
        commands.entity(e).remove::<(Collapsed, Dormant)>().insert(Visibility::Inherited);
        sounds.write(crate::sound::PlaySound::at("clank", pos).pitch(0.7));
        if let Some(&super::hurt::Bleeds(dust)) = bleeds {
            sim.world.splash([pos.x, pos.y - c.half.y * 0.5], dust, DUST_RISE, 1.6);
        }
    }
}

/// A heap's picture goes where its creature is (shaking as it's about to
/// stand); its creature gone (broken, dead), it goes.
pub fn draw_heaps(mut commands: Commands, time: Res<Time>, mut heaps: Query<(Entity, &Heap, &mut Transform)>, of: Query<(&Transform, Option<&Collapsed>), Without<Heap>>) {
    for (e, heap, mut tf) in &mut heaps {
        let Ok((at, Some(c))) = of.get(heap.of) else {
            commands.entity(e).despawn();
            continue;
        };
        let shake = if c.left < SHAKE {
            let t = time.elapsed_secs() * 40.0;
            let k = 1.0 - c.left.max(0.0) / SHAKE;
            Vec2::new(t.sin(), (t * 1.3).cos() * 0.5) * (0.4 + 0.8 * k)
        } else {
            Vec2::ZERO
        };
        tf.translation.x = at.translation.x + shake.x;
        tf.translation.y = at.translation.y + shake.y;
    }
}

/// Dormancy leaves a heap down (it wakes it by standing).
pub type NotDown = Without<Collapsed>;
