//! Bodies (DESIGN §7c, §8): what a creature leaves when it dies. Its picture
//! as it fell, laid down (on its back, the way it was facing; a spider as it
//! was, from above), darkened: a body that falls, slides and is thrown by
//! blasts. It holds what the creature wore and held, what it carried
//! (`drops`) and a roll of its loot table (`loot`, at the depth it died,
//! with the player's luck): right-click it to open it, as a chest.
//!
//! A body stays as long as something's in it. Emptied (or never holding
//! anything) it lies a while, then fades. There are at most `MAX_BODIES`:
//! past that, the oldest go, with what's in them (a battle leaves a field
//! of bodies, not a thousand of them). A creature with `corpse: false`
//! (an egg sac, a cocoon, a firefly) leaves none: what it had spills out.
//! A dead spider keeps its legs, curled in over its body (`curled_legs`).

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
/// Most bodies lying about at once.
pub const MAX_BODIES: usize = 150;

/// A body in the world (its contents are kept by `Chests`, under `key`).
#[derive(Component)]
pub struct Corpse {
    pub key: u64,
    /// Seconds it's been empty.
    empty: f32,
    /// When it fell (a count: the oldest go first).
    born: u64,
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

/// A dead spider's legs (from above, its body pointing right): each thigh
/// out from its hip, shorter than in life, the shin hooked back in over the
/// body, as dead spiders curl; a little crooked each (from
/// `seed`). Returns the picture and where its middle sits from the body's.
fn curled_legs(def: &crate::actors::legs::LegsDef, seed: u64) -> (Image, Vec2) {
    let (leg, joint) = (def.color, def.joint.unwrap_or(def.color));
    let per_side = (def.count / 2).max(1);
    let (thigh, shin) = (def.reach * def.upper * 0.5, def.reach * (1.0 - def.upper) * 0.55);
    let mut rng = Rng::seeded(&[seed, 0x1E65]);
    let mut jitter = |k: f32| (rng.next_u32() as f32 / u32::MAX as f32 * 2.0 - 1.0) * k;
    let mut px: Vec<(IVec2, (u8, u8, u8))> = Vec::new();
    let line = |a: Vec2, b: Vec2, thick: i32, c: (u8, u8, u8), px: &mut Vec<(IVec2, (u8, u8, u8))>| {
        let n = (b - a).abs().max_element().ceil().max(1.0) as i32;
        let d = b - a;
        let across = if d.x.abs() > d.y.abs() { IVec2::Y } else { IVec2::X };
        for i in 0..=n {
            let p = a + d * (i as f32 / n as f32);
            let p = IVec2::new(p.x.floor() as i32, p.y.floor() as i32);
            for w in 0..thick.max(1) {
                px.push((p + across * (w - (thick - 1) / 2), c));
            }
        }
    };
    for side in [1.0f32, -1.0] {
        for j in 0..per_side {
            let along = if per_side > 1 { j as f32 / (per_side - 1) as f32 - 0.5 } else { 0.0 };
            let hip = Vec2::new(def.hips * 0.4 - along * def.spread * 1.6, side * 3.0);
            // Front legs out and forward, back ones out and back...
            let out = (80.0 - along * 120.0 + jitter(10.0)).to_radians();
            let dir = Vec2::new(out.cos(), side * out.sin());
            let knee = hip + dir * thigh * (0.9 + jitter(0.15));
            // ... each shin hooked back in over the body, spindly.
            let fold = Vec2::from_angle(side * (125.0 + jitter(14.0)).to_radians()).rotate(dir);
            let foot = knee + fold * shin * (0.85 + jitter(0.2));
            line(hip, knee, (def.thick as i32 - 1).max(1), leg, &mut px);
            line(knee, foot, 1, leg, &mut px);
            px.push((IVec2::new(knee.x.floor() as i32, knee.y.floor() as i32), joint));
        }
    }
    let lo = px.iter().fold(IVec2::MAX, |m, (p, _)| m.min(*p));
    let hi = px.iter().fold(IVec2::MIN, |m, (p, _)| m.max(*p));
    let (w, h) = ((hi.x - lo.x + 1) as u32, (hi.y - lo.y + 1) as u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for (p, (r, g, b)) in px {
        let i = (((hi.y - p.y) as u32 * w + (p.x - lo.x) as u32) * 4) as usize;
        rgba[i..i + 4].copy_from_slice(&[r, g, b, 255]);
    }
    let image = Image::new(
        bevy::render::render_resource::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        rgba,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    (image, Vec2::new((lo.x + hi.x + 1) as f32 / 2.0, (lo.y + hi.y + 1) as f32 / 2.0))
}

/// Every creature that died leaves its body (or spills what it had).
#[allow(clippy::too_many_arguments)]
fn lay_out(
    mut commands: Commands,
    mut died: MessageReader<Died>,
    items: Option<Res<Items>>,
    sim: Res<SimWorld>,
    mut chests: ResMut<Chests>,
    player: Query<&crate::gear::Stats, With<LocalPlayer>>,
    mut images: ResMut<Assets<Image>>,
    mut count: Local<u64>,
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
            Corpse { key, empty: 0.0, born: { *count += 1; *count } },
            Container { key, name: format!("{} (dead)", d.def.name) },
            Thrown { bounce: 0.05 },
            Kinematics { body, loco: Locomotion::default(), prev_pos: at },
            Transform::from_translation(at.extend(Z)),
            Visibility::default(),
        ));
        if let Some((sprite, offset)) = &d.sprite {
            let mut sprite = sprite.clone();
            sprite.color = Color::srgb(DIM, DIM, DIM);
            let flip = sprite.flip_x;
            e.with_child((CorpseSprite, sprite, Transform::from_translation((rot * offset.with_z(0.0)).with_z(0.0)).with_rotation(rot)));
            // Legs: kept, curled in over the body.
            if let Some(legs) = &d.def.legs {
                let (image, centre) = curled_legs(legs, at.x.to_bits() as u64 ^ *count);
                let mut s = Sprite::from_image(images.add(image));
                s.flip_x = flip;
                s.color = Color::srgb(DIM, DIM, DIM);
                let centre = if flip { Vec2::new(-centre.x, centre.y) } else { centre };
                e.with_child((CorpseSprite, s, Transform::from_translation((offset.truncate() + centre).extend(0.01))));
            }
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

/// Empty bodies fade and go; past `MAX_BODIES`, the oldest go.
fn rot(mut commands: Commands, mut chests: ResMut<Chests>, mut q: Query<(Entity, &mut Corpse, Option<&Children>)>, mut sprites: Query<&mut Sprite, With<CorpseSprite>>) {
    let n = q.iter().count();
    if n > MAX_BODIES {
        let mut ages: Vec<(u64, Entity, u64)> = q.iter().map(|(e, c, _)| (c.born, e, c.key)).collect();
        ages.sort_unstable();
        for &(_, e, key) in &ages[..n - MAX_BODIES] {
            chests.forget(key);
            commands.entity(e).despawn();
        }
        // (The rest fade next tick.)
        return;
    }
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
