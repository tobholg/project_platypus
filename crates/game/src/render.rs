//! Two textures per loaded chunk: the background layer (dimmed, behind) and
//! the playfield (SPEC §4). A chunk re-colours only when its cells changed.
//!
//! Plants (tall grass, leaves) are drawn separately on top of a cached base
//! image, shifted sideways by wind and by `FoliageSprings` that creatures
//! excite as they move through. That sway is purely visual: the cells never
//! move, so it costs the simulation nothing.

use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use platypus_sim::cell::flags;
use platypus_sim::{CHUNK, Cell, ChunkPos, Climate, Kind, MaterialTable};

use crate::actors::Kinematics;
use crate::camera::MainCamera;
use crate::world::{ChunkLoader, SimWorld};

pub struct ChunkRenderPlugin;

pub const SKY_COLOR: Color = Color::srgb(0.42, 0.66, 0.92);
pub const Z_BACKGROUND: f32 = -1.0;
pub const Z_WORLD: f32 = 0.0;

const N: usize = CHUNK as usize;
/// Plant sway is recomposed at this rate (Hz).
const SWAY_HZ: f32 = 30.0;

/// Marks the sprites that show a chunk.
#[derive(Component)]
pub struct ChunkSprite;

/// One plant pixel: where it is in the chunk, its colour, and how far it is
/// above its root (more height, more sway).
#[derive(Clone, Copy)]
struct PlantPx {
    x: u8,
    y: u8,
    rgba: [u8; 4],
    lift: u8,
}

struct Layer {
    entity: Entity,
    image: Handle<Image>,
    /// RGBA without plants (texture row order: row 0 = top).
    base: Vec<u8>,
    plants: Vec<PlantPx>,
}

struct ChunkGfx {
    front: Layer,
    back: Layer,
}

#[derive(Resource, Default)]
struct ChunkGfxs(HashMap<ChunkPos, ChunkGfx>);

/// Springs that bend foliage, one per 2×8-cell tile, created where creatures
/// move and removed once they settle. Displacement is in cells.
#[derive(Resource, Default)]
pub struct FoliageSprings {
    springs: HashMap<(i32, i32), Spring>,
}

#[derive(Clone, Copy, Default)]
struct Spring {
    disp: f32,
    vel: f32,
}

const SPRING_COL_BITS: i32 = 1;
const SPRING_ROW_BITS: i32 = 3;

impl FoliageSprings {
    fn disp(&self, x: i32, y: i32) -> f32 {
        self.springs.get(&(x >> SPRING_COL_BITS, y >> SPRING_ROW_BITS)).map_or(0.0, |s| s.disp)
    }
}

#[derive(Resource)]
struct SwayClock(Timer);

impl Plugin for ChunkRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(SKY_COLOR))
            .init_resource::<ChunkGfxs>()
            .init_resource::<FoliageSprings>()
            .insert_resource(SwayClock(Timer::from_seconds(1.0 / SWAY_HZ, TimerMode::Repeating)))
            .add_systems(Update, excite_foliage)
            .add_systems(PostUpdate, sync_chunks);
    }
}

// ---- colours -----------------------------------------------------------------

fn is_plant(mats: &MaterialTable, c: Cell) -> bool {
    !c.is_air() && mats.phys(c.material).kind == Kind::Plant
}

fn cell_rgba(mats: &MaterialTable, c: Cell, ambient: i32, dim: f32, lx: usize, ly: usize) -> [u8; 4] {
    let mut rgba = mats.color(c);
    let ph = mats.phys(c.material);
    // Mining damage on solids shows as darkening cracks.
    if c.life > 0 && matches!(ph.kind, Kind::Static | Kind::Powder) && ph.hardness > 0 && c.flags & flags::BURNING == 0 {
        let k = 1.0 - 0.6 * (c.life as f32 / ph.hardness as f32).min(1.0);
        for ch in &mut rgba[..3] {
            *ch = (*ch as f32 * k) as u8;
        }
    }
    if dim < 1.0 {
        for ch in &mut rgba[..3] {
            *ch = (*ch as f32 * dim) as u8;
        }
    }
    if c.heat > 300 {
        glow(&mut rgba, ambient + c.heat as i32);
    }
    if c.flags & flags::BURNING != 0 {
        // Flicker: a per-cell mix toward flame colours that changes as it burns down.
        let n = ((lx as u32 * 73856093) ^ (ly as u32 * 19349663) ^ (c.life as u32 * 83492791)) % 100;
        let flame = if n < 45 { [255.0, 120.0, 20.0] } else if n < 80 { [255.0, 190.0, 60.0] } else { [200.0, 50.0, 10.0] };
        for (ch, f) in rgba.iter_mut().zip(flame) {
            *ch = (*ch as f32 * 0.35 + f * 0.65) as u8;
        }
    }
    rgba
}

/// Background brightness: trees and plants close behind, walls further back.
fn bg_dim(mats: &MaterialTable, c: Cell) -> f32 {
    let ph = mats.phys(c.material);
    if ph.kind == Kind::Plant || ph.flammability > 0 { 0.8 } else { 0.55 }
}

/// Incandescence: dull red from ~450 °C through orange to yellow-white.
fn glow(rgba: &mut [u8; 4], celsius: i32) {
    const START: f32 = 450.0;
    let t = celsius as f32;
    if t <= START {
        return;
    }
    let k = ((t - START) / 1100.0).clamp(0.0, 1.0);
    let ramp = |a: [f32; 3], b: [f32; 3], f: f32| [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f];
    let (red, orange, white) = ([150.0, 24.0, 12.0], [255.0, 118.0, 24.0], [255.0, 236.0, 170.0]);
    let hot = if k < 0.5 { ramp(red, orange, k * 2.0) } else { ramp(orange, white, (k - 0.5) * 2.0) };
    let mix = (0.35 + 0.65 * k) * ((t - START) / 150.0).min(1.0);
    for (ch, h) in rgba.iter_mut().zip(hot) {
        *ch = (*ch as f32 + (h - *ch as f32) * mix) as u8;
    }
}

/// Byte offset of a chunk cell in its texture (texture row 0 = top).
#[inline]
fn px(lx: usize, ly: usize) -> usize {
    ((N - 1 - ly) * N + lx) * 4
}

/// Rebuild a layer's base image and plant list from its cells.
fn rebuild(layer: &mut Layer, cells: &[Cell], mats: &MaterialTable, origin_y: i32, climate: &Climate, back: bool) {
    layer.base.fill(0);
    layer.plants.clear();
    for ly in 0..N {
        let ambient = climate.ambient(origin_y + ly as i32);
        for lx in 0..N {
            let c = cells[ly * N + lx];
            if c.is_air() {
                continue;
            }
            let dim = if back { bg_dim(mats, c) } else { 1.0 };
            let rgba = cell_rgba(mats, c, ambient, dim, lx, ly);
            if is_plant(mats, c) && c.flags & flags::BURNING == 0 {
                // Height above its root: plant cells below it in this column.
                let lift = (1..=15).take_while(|d| ly >= *d && is_plant(mats, cells[(ly - d) * N + lx])).count() as u8 + 1;
                layer.plants.push(PlantPx { x: lx as u8, y: ly as u8, rgba, lift });
            } else {
                layer.base[px(lx, ly)..px(lx, ly) + 4].copy_from_slice(&rgba);
            }
        }
    }
}

struct Sway {
    t: f32,
    wind: f32,
}

/// Base + plants shifted by wind and springs, into the image.
fn compose(layer: &Layer, data: &mut [u8], origin: (i32, i32), sway: &Sway, springs: &FoliageSprings, back: bool) {
    data.copy_from_slice(&layer.base);
    let gust = if back { 0.7 } else { 1.0 };
    for p in &layer.plants {
        let (wx, wy) = (origin.0 + p.x as i32, origin.1 + p.y as i32);
        let wave = (sway.t * 2.1 + wx as f32 * 0.11 + wy as f32 * 0.04).sin();
        let lean = sway.wind * 1.1 + wave * (0.3 + 0.6 * sway.wind.abs()) * gust;
        let bend = (lean + springs.disp(wx, wy)) * (p.lift as f32 / 6.0).min(1.6);
        let x = (p.x as i32 + bend.round() as i32).clamp(0, CHUNK - 1) as usize;
        let i = px(x, p.y as usize);
        // Leaves sway over the sky, not over the tree's own wood.
        if !back || data[i + 3] == 0 {
            data[i..i + 4].copy_from_slice(&p.rgba);
        }
    }
}

// ---- systems -----------------------------------------------------------------

fn new_layer(commands: &mut Commands, images: &mut Assets<Image>, pos: ChunkPos, z: f32) -> Layer {
    let image = Image::new_fill(
        Extent3d { width: N as u32, height: N as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let image = images.add(image);
    let o = pos.origin();
    let half = CHUNK as f32 / 2.0;
    let entity = commands
        .spawn((
            Sprite { image: image.clone(), custom_size: Some(Vec2::splat(CHUNK as f32)), ..default() },
            Transform::from_xyz(o.x as f32 + half, o.y as f32 + half, z),
            ChunkSprite,
        ))
        .id();
    Layer { entity, image, base: vec![0; N * N * 4], plants: Vec::new() }
}

#[allow(clippy::too_many_arguments)]
fn sync_chunks(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<SimWorld>,
    springs: Res<FoliageSprings>,
    mut clock: ResMut<SwayClock>,
    mut gfx: ResMut<ChunkGfxs>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>,
) {
    gfx.0.retain(|pos, g| {
        let keep = sim.world.is_loaded(*pos);
        if !keep {
            for layer in [&g.front, &g.back] {
                commands.entity(layer.entity).despawn();
                images.remove(layer.image.id());
            }
        }
        keep
    });

    let sway_now = clock.0.tick(time.delta()).just_finished();
    let sway = Sway { t: time.elapsed_secs(), wind: sim.world.wind() };
    // Only chunks on screen (plus a margin) animate.
    let view = camera.single().ok().map(|(tf, l)| {
        let c = tf.translation().truncate();
        let margin = Vec2::splat(CHUNK as f32);
        (c - l.half_extent - margin, c + l.half_extent + margin)
    });
    let mats = sim.materials();
    let climate = sim.world.climate();

    for chunk in sim.world.chunks() {
        let fresh = !gfx.0.contains_key(&chunk.pos);
        if fresh {
            let front = new_layer(&mut commands, &mut images, chunk.pos, Z_WORLD);
            let back = new_layer(&mut commands, &mut images, chunk.pos, Z_BACKGROUND);
            gfx.0.insert(chunk.pos, ChunkGfx { front, back });
        }
        let g = gfx.0.get_mut(&chunk.pos).expect("inserted above");
        let dirty = chunk.take_render_dirty() || fresh;
        let o = chunk.pos.origin();
        if dirty {
            rebuild(&mut g.front, chunk.cells(), mats, o.y, &climate, false);
            rebuild(&mut g.back, chunk.background(), mats, o.y, &climate, true);
        }
        let visible = view.is_none_or(|(lo, hi)| {
            let (x, y) = (o.x as f32, o.y as f32);
            x + CHUNK as f32 >= lo.x && x <= hi.x && y + CHUNK as f32 >= lo.y && y <= hi.y
        });
        for (layer, back) in [(&g.front, false), (&g.back, true)] {
            let animate = sway_now && visible && !layer.plants.is_empty();
            if !(dirty || animate) {
                continue;
            }
            let Some(mut image) = images.get_mut(&layer.image) else { continue };
            let Some(data) = image.data.as_mut() else { continue };
            compose(layer, data, (o.x, o.y), &sway, &springs, back);
        }
    }
}

/// Creatures moving through foliage push it: sideways away from their body
/// and along their direction of travel. Springs wobble back afterwards.
fn excite_foliage(time: Res<Time>, mut springs: ResMut<FoliageSprings>, bodies: Query<&Kinematics>) {
    let dt = time.delta_secs().min(0.05);
    for k in &bodies {
        let b = &k.body;
        let speed = b.vel.length();
        if speed < 8.0 {
            continue;
        }
        let (x0, x1) = ((b.pos.x - b.half.x - 3.0) as i32, (b.pos.x + b.half.x + 3.0) as i32);
        let (y0, y1) = ((b.pos.y - b.half.y) as i32, (b.pos.y + b.half.y) as i32);
        for col in (x0 >> SPRING_COL_BITS)..=(x1 >> SPRING_COL_BITS) {
            for row in (y0 >> SPRING_ROW_BITS)..=(y1 >> SPRING_ROW_BITS) {
                let cx = ((col << SPRING_COL_BITS) + 1) as f32;
                let away = (cx - b.pos.x).signum();
                let s = springs.springs.entry((col, row)).or_default();
                s.vel += (b.vel.x * 0.06 + away * speed.min(200.0) * 0.03) * dt * 8.0;
            }
        }
    }
    // Underdamped: a few wobbles, then rest.
    const STIFFNESS: f32 = 55.0;
    const DAMPING: f32 = 4.5;
    springs.springs.retain(|_, s| {
        s.vel += (-STIFFNESS * s.disp - DAMPING * s.vel) * dt;
        s.disp = (s.disp + s.vel * dt).clamp(-3.5, 3.5);
        s.disp.abs() > 0.02 || s.vel.abs() > 0.05
    });
}
