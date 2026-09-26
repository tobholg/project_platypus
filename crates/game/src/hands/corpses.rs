//! Bodies (DESIGN §7c, §8): what a creature leaves when it dies. Its picture
//! as it fell, laid down (on its back, the way it was facing; a spider as it
//! was, from above), darkened: a body that falls, slides and is thrown by
//! blasts. It holds what the creature wore and held, what it carried
//! (`drops`) and a roll of its loot table (`loot`, at the depth it died,
//! with the player's luck): right-click it to open it, as a chest.
//!
//! A body stays as long as something's in it. Emptied (or never holding
//! anything) it lies a while, then fades. A creature with `corpse: false`
//! (an egg sac, a cocoon, a firefly) leaves none: what it had spills out.

use bevy::prelude::*;
use platypus_physics::{Body, Locomotion};
use platypus_sim::rng::Rng;

use super::chests::{Chests, Container, SLOTS};
use super::items::{Inventory, Items, Stack};
use super::spawn_drop;
use crate::actors::player::LocalPlayer;
use crate::actors::{Died, Kinematics};
use crate::fx::Explosion;
use crate::props::Thrown;
use crate::world::{SimWorld, TICK_HZ, TickSet};

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// An empty body lies this long (seconds), then fades over `FADE`.
const LINGER: f32 = 20.0;
const FADE: f32 = 3.0;
/// Bodies draw behind the living, over the world (chests are at 6).
const Z: f32 = 5.5;
/// A body's colours, darkened.
const DIM: f32 = 0.72;

/// A body in the world (its contents are kept by `Chests`, under `key`).
#[derive(Component)]
pub struct Corpse {
    pub key: u64,
    /// Seconds it's been empty.
    empty: f32,
}

/// The picture of a body (its child).
#[derive(Component)]
struct CorpseSprite;

pub struct CorpsesPlugin;

impl Plugin for CorpsesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (lay_out, (batter, rot).after(crate::props::fly)).in_set(TickSet::Bodies));
    }
}

/// What a dead creature had on it: what it wore and held (gear keeps its
/// roll), what it carried, and a roll of its loot table.
fn belongings(died: &Died, items: &Items, chests: &Chests, world: &platypus_sim::World, luck: f32) -> Inventory {
    let mut inv = Inventory::new(SLOTS);
    for s in &died.worn {
        inv.add(items, *s);
    }
    for (id, n) in &died.def.drops {
        if let Some(item) = items.id(id) {
            inv.add(items, Stack::new(item, n * items.unit(item)));
        }
    }
    if let Some(table) = &died.def.loot {
        let at = died.body.pos;
        let mut rng = Rng::seeded(&[world.seed(), world.tick(), at.x.to_bits() as u64, at.y.to_bits() as u64, 0xDEAD]);
        chests.roll_table(table, &mut inv, items, chests.level_at(world, at), luck, &mut rng);
    }
    inv
}

/// Every creature that died leaves its body (or spills what it had).
fn lay_out(
    mut commands: Commands,
    mut died: MessageReader<Died>,
    items: Option<Res<Items>>,
    sim: Res<SimWorld>,
    mut chests: ResMut<Chests>,
    player: Query<&crate::gear::Stats, With<LocalPlayer>>,
) {
    let Some(items) = items else { return };
    let luck = player.single().map_or(0.0, |s| s.get(crate::gear::Stat::Luck));
    for d in died.read() {
        let inv = belongings(d, &items, &chests, &sim.world, luck);
        let at = d.body.pos;
        if !d.def.corpse {
            for (k, s) in inv.slots.iter().flatten().enumerate() {
                spawn_drop(&mut commands, &items, at + Vec2::new(k as f32 * 0.7 - 2.0, 0.0), *s);
            }
            continue;
        }
        let key = chests.stash(at, inv);
        // Laid on its back, head the way it faced from; a spider (drawn
        // from above) as it was.
        let turn = if d.def.legs.is_some() { 0.0 } else { std::f32::consts::FRAC_PI_2 * d.facing };
        let rot = Quat::from_rotation_z(turn);
        let size = d.body.half * 2.0;
        let size = if turn == 0.0 { size } else { Vec2::new(size.y, size.x) };
        let mut body = Body::new(at, size.max(Vec2::splat(2.0)));
        body.vel = d.body.vel * 0.5;
        let mut e = commands.spawn((
            Name::new(format!("Body of {}", d.def.name)),
            Corpse { key, empty: 0.0 },
            Container { key, name: format!("{} (dead)", d.def.name) },
            Thrown { bounce: 0.05 },
            Kinematics { body, loco: Locomotion::default(), prev_pos: at },
            Transform::from_translation(at.extend(Z)),
            Visibility::default(),
        ));
        if let Some((sprite, offset)) = &d.sprite {
            let mut sprite = sprite.clone();
            sprite.color = Color::srgb(DIM, DIM, DIM);
            e.with_child((CorpseSprite, sprite, Transform::from_translation((rot * offset.with_z(0.0)).with_z(0.0)).with_rotation(rot)));
        }
    }
}

/// Blasts throw bodies about.
fn batter(mut blasts: MessageReader<Explosion>, mut q: Query<&mut Kinematics, With<Corpse>>) {
    for b in blasts.read() {
        for mut k in &mut q {
            let reach = b.radius * 1.6 + 6.0;
            let d = k.body.pos.distance(b.at);
            if d < reach {
                let away = (k.body.pos - b.at).normalize_or(Vec2::Y);
                k.body.vel += (away + Vec2::new(0.0, 0.6)) * b.power * 3.0 * (1.0 - d / reach);
            }
        }
    }
}

/// Empty bodies fade and go.
fn rot(mut commands: Commands, mut chests: ResMut<Chests>, mut q: Query<(Entity, &mut Corpse, Option<&Children>)>, mut sprites: Query<&mut Sprite, With<CorpseSprite>>) {
    for (e, mut c, children) in &mut q {
        if !chests.is_empty(c.key) {
            c.empty = 0.0;
            continue;
        }
        c.empty += DT;
        let fade = ((c.empty - LINGER) / FADE).clamp(0.0, 1.0);
        for child in children.into_iter().flat_map(|c| c.iter()) {
            if let Ok(mut s) = sprites.get_mut(child) {
                s.color = Color::srgba(DIM, DIM, DIM, 1.0 - fade);
            }
        }
        if fade >= 1.0 {
            chests.forget(c.key);
            commands.entity(e).despawn();
        }
    }
}
