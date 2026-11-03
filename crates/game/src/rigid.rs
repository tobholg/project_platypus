//! Rigid pixel bodies from the sim (felled trees, severed branches): drawn by
//! mapping every world cell they cover back into the body, so they stay on
//! the cell grid at any angle; and they hurt what they land on. A felled
//! tree cracks into logs as it tips (the sim: `World::crack`); a log is
//! loot: walk into it and it's yours, and one come to rest drops as wood
//! to pick up (`logs`). Logs don't hurt the player.

use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use platypus_sim::Kind;

use crate::creatures::{Health, Kinematics};
use crate::render::cell_rgba;
use crate::world::{SimWorld, TICK_HZ, TickSet};

pub struct RigidPlugin;

/// Between the background (-1) and the playfield (0): a falling tree passes
/// behind the player and behind the ground it crashes into.
const Z_BODIES: f32 = -0.5;
/// Falling trees are background things until they land; drawn like one.
const DIM: f32 = 0.85;
/// Fastest point (cells/tick) that still does no harm.
const HARMLESS_SPEED: f32 = 1.8;
const DAMAGE_PER_SPEED: f32 = 9.33;
/// A log cut free (not a felled tree) hurts the player only falling at least
/// this fast (cells/tick): a real drop, not the shifting and short falls of
/// one being chopped up.
const LOG_HARMLESS: f32 = 3.0;
const MAX_DAMAGE: f32 = 80.0;
/// Logs are drawn among the playfield's things (in front of the ground,
/// as loot is), at full light.
const Z_LOGS: f32 = 0.4;

struct Drawn {
    entity: Entity,
    image: Handle<Image>,
    side: u32,
    tick: u64,
}

#[derive(Resource, Default)]
struct DrawnBodies(HashMap<u32, Drawn>);

impl Plugin for RigidPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawnBodies>()
            .add_systems(Update, draw_bodies)
            .add_systems(FixedUpdate, (crush, logs).chain().after(TickSet::Cells));
    }
}

fn draw_bodies(
    mut commands: Commands,
    sim: Res<SimWorld>,
    mut drawn: ResMut<DrawnBodies>,
    mut images: ResMut<Assets<Image>>,
    mut sprites: Query<(&mut Transform, &mut Sprite)>,
) {
    let world = &sim.world;
    let mats = world.materials();
    let climate = world.climate();
    let tick = world.tick();
    let alive: HashSet<u32> = world.bodies().iter().map(|b| b.id).collect();
    drawn.0.retain(|id, d| {
        let keep = alive.contains(id);
        if !keep {
            commands.entity(d.entity).despawn();
        }
        keep
    });
    for body in world.bodies() {
        let (lo, hi) = body.bounds();
        let side = (hi.x - lo.x + 1) as u32;
        let d = drawn.0.entry(body.id).or_insert_with(|| {
            let image = images.add(blank(side));
            let entity = commands.spawn((Sprite::from_image(image.clone()), Transform::default())).id();
            Drawn { entity, image, side, tick: u64::MAX }
        });
        if d.tick == tick {
            continue;
        }
        d.tick = tick;
        if d.side != side {
            // Shed its crown: smaller now.
            d.image = images.add(blank(side));
            d.side = side;
        }
        if let Ok((mut tf, mut sprite)) = sprites.get_mut(d.entity) {
            let n = side as f32;
            *tf = Transform::from_xyz(lo.x as f32 + n / 2.0, lo.y as f32 + n / 2.0, if body.log { Z_LOGS } else { Z_BODIES });
            sprite.image = d.image.clone();
            sprite.custom_size = Some(Vec2::splat(n));
        }
        let Some(mut image) = images.get_mut(&d.image) else { continue };
        let Some(data) = image.data.as_mut() else { continue };
        data.fill(0);
        let ambient = climate.ambient(body.pos[0] as i32, body.pos[1] as i32);
        for (p, c) in body.world_cells() {
            let (tx, ty) = ((p.x - lo.x) as usize, (hi.y - p.y) as usize);
            let i = (ty * side as usize + tx) * 4;
            data[i..i + 4].copy_from_slice(&cell_rgba(mats, c, ambient, if body.log { 1.0 } else { DIM }, p.x as usize, p.y as usize));
        }
    }
}

fn blank(side: u32) -> Image {
    Image::new_fill(
        Extent3d { width: side, height: side, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

/// A falling trunk hits creatures in its way: damage by how fast that part of
/// it moves (a felled tree's), or how fast it goes (anything loose: a
/// boulder, a cut log), knocked along with it. Once per body per creature.
/// A log the player's chopping up doesn't batter it as it shifts and drops
/// (`LOG_HARMLESS`); dropped on anything else, it hurts as ever.
fn crush(
    sim: Res<SimWorld>,
    mut hit: Local<HashSet<(u32, Entity)>>,
    mut creatures: Query<(Entity, &mut Kinematics, &mut Health, Has<crate::creatures::player::LocalPlayer>)>,
) {
    let world = &sim.world;
    let mats = world.materials();
    let bodies = world.bodies();
    hit.retain(|(id, _)| bodies.iter().any(|b| b.id == *id));
    for body in bodies.iter().filter(|b| if b.felled() { b.top_speed() } else { Vec2::from(b.vel).length() } > HARMLESS_SPEED) {
        let (lo, hi) = body.bounds();
        // (Mostly wood: a log.)
        let woody = {
            let (mut wood, mut n) = (0u32, 0u32);
            for (_, c) in body.world_cells() {
                if mats.phys(c.material).kind != Kind::Plant {
                    n += 1;
                    wood += u32::from(mats.def(c.material).loot == platypus_sim::Loot::Wood);
                }
            }
            wood * 2 > n
        };
        for (entity, mut k, mut health, player) in &mut creatures {
            // (A felled tree's, cracking into logs, or a log: loot, not a
            // hazard, to the player.)
            if player && body.logging() {
                continue;
            }
            let (pos, half) = (k.body.pos, k.body.half);
            if pos.x + half.x < lo.x as f32 || pos.x - half.x > hi.x as f32 + 1.5 || pos.y + half.y < lo.y as f32 || pos.y - half.y > hi.y as f32 + 1.5 {
                continue;
            }
            if hit.contains(&(body.id, entity)) {
                continue;
            }
            // A few points over the creature's box.
            let struck = (-1..=1).flat_map(|i| (-1..=1).map(move |j| pos + Vec2::new(i as f32, j as f32) * half * 0.8)).find(|p| {
                body.sample([p.x, p.y]).is_some_and(|c| mats.phys(c.material).kind != Kind::Plant)
            });
            let Some(at) = struck else { continue };
            // (A felled tree hits as fast as the part of it that strikes:
            // its crown swinging down. Anything loose (a boulder, a log cut
            // free, a dropped thing) as fast as it goes itself: a log
            // tipping over as it's chopped up barely moves, however its
            // end swings.)
            let v = Vec2::from(if body.felled() { body.velocity_at([at.x, at.y]) } else { body.vel });
            let speed = v.length();
            let harmless = if player && woody && !body.felled() { LOG_HARMLESS } else { HARMLESS_SPEED };
            if speed < harmless {
                continue;
            }
            hit.insert((body.id, entity));
            let share = if player { crate::creatures::PLAYER_DEBRIS } else { 1.0 };
            health.harm_by((speed * DAMAGE_PER_SPEED).min(MAX_DAMAGE) * share, crate::creatures::Harm::Blunt, || crate::death::Cause::new("Something heavy", "crushed"));
            let k = &mut *k;
            let push = v * TICK_HZ as f32 * 0.7 + Vec2::new(0.0, 180.0);
            k.loco.knock(&mut k.body, push, 0.4);
        }
    }
}

/// Logs: one the player walks into is taken up (as wood, dropped on it:
/// the pickup takes it in); one come to rest drops as wood where it lay.
/// A tree starting to fall creaks; each log cracking off, a crack.
fn logs(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    player: Query<&Kinematics, With<crate::creatures::player::LocalPlayer>>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut heard: Local<HashSet<u32>>,
) {
    let Some(items) = items else { return };
    let world = &mut sim.world;
    // (Sounds: a tree as it starts to go, a log as it comes off.)
    heard.retain(|id| world.bodies().iter().any(|b| b.id == *id));
    for b in world.bodies() {
        if heard.insert(b.id) && b.logging() {
            let at = Vec2::from(b.pos);
            sounds.write(if b.log { crate::sound::PlaySound::at("log_crack", at) } else { crate::sound::PlaySound::at("timber", at) });
        }
    }
    let mut done = world.take_logged();
    if let Ok(k) = player.single() {
        let (lo, hi) = (k.body.pos - k.body.half - 1.0, k.body.pos + k.body.half + 1.0);
        let touched: Vec<u32> = world
            .bodies()
            .iter()
            .filter(|b| b.log)
            .filter(|b| {
                let (blo, bhi) = b.bounds();
                bhi.x as f32 + 1.0 >= lo.x && blo.x as f32 <= hi.x && bhi.y as f32 + 1.0 >= lo.y && blo.y as f32 <= hi.y
            })
            .filter(|b| (0..=4).flat_map(|i| (0..=4).map(move |j| Vec2::new(i as f32, j as f32) / 4.0)).any(|t| b.sample((lo + (hi - lo) * t).into()).is_some()))
            .map(|b| b.id)
            .collect();
        for id in touched {
            if let Some(mut l) = world.collect_log(id) {
                l.at = k.body.pos.into();
                done.push(l);
            }
        }
    }
    for l in done {
        for (material, n) in l.loot {
            if let Some(item) = items.block(material) {
                crate::hands::spawn_drop(&mut commands, &items, Vec2::from(l.at), crate::hands::items::Stack::new(item, n));
            }
        }
    }
}
