//! Mimics: a chest (to the last nail) until a player comes right up to it,
//! tries to open it, or something hurts it. Till then it's `Disguised`: no
//! mind of its own, no touch, its picture the shut chest's. Then it
//! rattles open and comes at you (its brain, its moves). Killed, it's a
//! chest again, there where it died (and what's in it is a chest's from
//! that deep: `hands::chests`). Some chests are mimics: `creatures::spawn`.

use bevy::prelude::*;
use serde::Deserialize;

use super::{CustomCreature, CustomSet};
use crate::creatures::body::animation::Animator;
use crate::creatures::{Controls, Died, Disguised, Health, Kinematics, Team};

/// How near a player wakes it (cells, from its box); what it remembers.
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Mimic {
    pub wake: f32,
    /// Someone tried to open it (a right click on it, within reach).
    #[serde(skip)]
    asked: bool,
    /// Awake since (s; its waking clip plays this long).
    #[serde(skip)]
    woke: Option<f32>,
}

impl Default for Mimic {
    fn default() -> Self {
        Mimic { wake: 14.0, asked: false, woke: None }
    }
}

impl CustomCreature for Mimic {
    const NAME: &'static str = "mimic";

    fn build(app: &mut App) {
        app.add_observer(on_spawn)
            .add_systems(Update, open_it)
            .add_systems(FixedUpdate, think.in_set(CustomSet::Think))
            .add_systems(FixedUpdate, on_death.in_set(crate::world::TickSet::Bodies));
    }
}

/// Its waking clip plays this long (s); it springs this hard at you.
const WAKING: f32 = 0.5;
const SPRING: (f32, f32) = (140.0, 230.0);

/// Spawned: a chest, shut.
fn on_spawn(add: On<Add, Mimic>, mut commands: Commands, mut q: Query<&mut Animator>) {
    commands.entity(add.entity).insert(Disguised);
    if let Ok(mut anim) = q.get_mut(add.entity) {
        anim.play("shut");
    }
}

/// A right click on it, as on a chest (`chests::open_chest`): it's asked.
fn open_it(mouse: Res<ButtonInput<MouseButton>>, cursor: Res<crate::camera::CursorWorld>, player: Query<&Kinematics, With<crate::creatures::player::LocalPlayer>>, mut q: Query<(&mut Mimic, &Kinematics), With<Disguised>>) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let (Some(at), Ok(pk)) = (cursor.0, player.single()) else { return };
    for (mut m, k) in &mut q {
        if ((k.body.pos - at).abs() - k.body.half).max_element() <= 4.5 && k.body.pos.distance(pk.body.pos) <= crate::hands::chests::REACH {
            m.asked = true;
        }
    }
}

/// Shut, it's still; someone near, asking or hurting it: it wakes, springs
/// at them, and is itself.
#[allow(clippy::type_complexity)]
fn think(
    mut commands: Commands,
    time: Res<Time>,
    hunted: Query<(&Kinematics, &Team), (Without<Mimic>, Without<crate::creatures::brain::villager::Hiding>)>,
    mut q: Query<(Entity, &mut Mimic, &mut Kinematics, &mut Controls, &mut Animator, &Health, Has<Disguised>)>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    let now = time.elapsed_secs();
    for (e, mut m, mut k, mut c, mut anim, health, shut) in &mut q {
        if !shut {
            if m.woke.is_some_and(|w| now - w > WAKING) && anim.force.as_deref() == Some("wake") {
                anim.force = None;
            }
            continue;
        }
        c.0.move_x = 0.0;
        c.0.jump = false;
        let near = hunted.iter().filter(|(_, t)| t.hunted()).map(|(hk, _)| (((hk.body.pos - k.body.pos).abs() - k.body.half - hk.body.half).max(Vec2::ZERO).length(), hk.body.pos)).min_by(|a, b| a.0.total_cmp(&b.0));
        let hurt = health.hp < health.max;
        let Some((gap, at)) = near else { continue };
        if !(gap <= m.wake || m.asked || hurt) {
            continue;
        }
        commands.entity(e).remove::<Disguised>();
        m.woke = Some(now);
        m.asked = false;
        anim.play("wake");
        let side = if at.x < k.body.pos.x { -1.0 } else { 1.0 };
        k.loco.facing = side;
        k.body.vel = Vec2::new(side * SPRING.0, SPRING.1);
        sounds.write(crate::sound::PlaySound::at("clank", k.body.pos).pitch(0.7).volume(0.9));
        sounds.write(crate::sound::PlaySound::at("snarl", k.body.pos).pitch(0.8));
    }
}

/// Dead: a chest again, where it fell (open it for what's in it).
fn on_death(mut commands: Commands, mut died: MessageReader<Died>, mut chests: ResMut<crate::hands::chests::Chests>, mut sounds: MessageWriter<crate::sound::PlaySound>) {
    for d in died.read() {
        if d.kind != "mimic" {
            continue;
        }
        let feet = d.body.pos - Vec2::Y * d.body.half.y;
        chests.spawn_found(&mut commands, platypus_sim::CellPos::from_world(feet.x, feet.y + 0.5));
        sounds.write(crate::sound::PlaySound::at("clank", d.body.pos).pitch(0.5));
    }
}
