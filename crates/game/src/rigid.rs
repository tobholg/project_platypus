//! Rigid pixel bodies from the sim (felled trees, severed branches): drawn by
//! mapping every world cell they cover back into the body, so they stay on
//! the cell grid at any angle; and they hurt what they land on.

use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use platypus_sim::Kind;

use crate::actors::{Health, Kinematics};
use crate::render::cell_rgba;
use crate::world::{SimWorld, TICK_HZ, TickSet};

pub struct RigidPlugin;

/// Between the background (-1) and the playfield (0): a falling tree passes
/// behind the player and behind the ground it crashes into.
const Z_BODIES: f32 = -0.5;
/// Falling trees are background things until they land; drawn like one.
const DIM: f32 = 0.85;
/// Fastest point (cells/tick) that still does no harm.
const HARMLESS_SPEED: f32 = 1.2;
const DAMAGE_PER_SPEED: f32 = 14.0;
const MAX_DAMAGE: f32 = 80.0;

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
            .add_systems(FixedUpdate, crush.after(TickSet::Cells));
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
            *tf = Transform::from_xyz(lo.x as f32 + n / 2.0, lo.y as f32 + n / 2.0, Z_BODIES);
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
            data[i..i + 4].copy_from_slice(&cell_rgba(mats, c, ambient, DIM, p.x as usize, p.y as usize));
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
/// it moves, knocked along with it. Once per body per creature.
fn crush(
    sim: Res<SimWorld>,
    mut hit: Local<HashSet<(u32, Entity)>>,
    mut creatures: Query<(Entity, &mut Kinematics, &mut Health)>,
) {
    let world = &sim.world;
    let mats = world.materials();
    let bodies = world.bodies();
    hit.retain(|(id, _)| bodies.iter().any(|b| b.id == *id));
    for body in bodies.iter().filter(|b| b.top_speed() > HARMLESS_SPEED) {
        let (lo, hi) = body.bounds();
        for (entity, mut k, mut health) in &mut creatures {
            let (pos, half) = (k.body.pos, k.body.half);
            if pos.x + half.x < lo.x as f32 || pos.x - half.x > hi.x as f32 + 1.0 || pos.y + half.y < lo.y as f32 || pos.y - half.y > hi.y as f32 + 1.0 {
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
            let v = Vec2::from(body.velocity_at([at.x, at.y]));
            let speed = v.length();
            if speed < HARMLESS_SPEED {
                continue;
            }
            hit.insert((body.id, entity));
            health.harm((speed * DAMAGE_PER_SPEED).min(MAX_DAMAGE), crate::actors::Harm::Physical);
            let k = &mut *k;
            let push = v * TICK_HZ as f32 * 0.7 + Vec2::new(0.0, 120.0);
            k.loco.knock(&mut k.body, push, 0.4);
        }
    }
}
