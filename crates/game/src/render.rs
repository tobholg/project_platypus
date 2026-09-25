//! Two textures per loaded chunk: the background layer (dimmed, behind) and
//! the playfield (SPEC §4). A chunk re-colours only when its cells changed.
//!
//! Grass is drawn separately on top of a cached base image, shifted sideways
//! by wind and by `FoliageSprings` that creatures excite as they move through.
//! That sway is purely visual: the cells never move, so it costs the
//! simulation nothing.
//!
//! Leaves (background) don't sway for now. A crown has to move as one piece
//! or it tears, and one offset for every crown on screen looks mechanical;
//! doing it well needs to know which tree a leaf belongs to, which trees get
//! when they become bodies that can be felled.

use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use platypus_sim::cell::flags;
use platypus_sim::{CHUNK, Cell, CellPos, ChunkPos, Climate, Kind, MaterialTable};

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
    /// Cells that sparkle or shed motes (`MaterialDef::glint`, `motes`).
    glints: Vec<Glint>,
}

/// A cell that sparkles (crystals, gems) or sheds glowing motes (spores).
#[derive(Clone, Copy)]
struct Glint {
    x: u8,
    y: u8,
    rate: u8,
    motes: bool,
    rgb: [u8; 3],
}

/// A sparkle or a mote: a little light over the dark, for a moment.
#[derive(Component)]
struct Sparkle {
    age: f32,
    life: f32,
    vel: Vec2,
    rgb: [f32; 3],
}

/// Sparkles and motes draw over the light (they're lights themselves).
const Z_SPARKLES: f32 = 16.0;
/// At most this many at once.
const MAX_SPARKLES: usize = 160;

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
            .add_systems(Update, (excite_foliage, sparkle))
            .add_systems(PostUpdate, sync_chunks);
    }
}

// ---- colours -----------------------------------------------------------------

/// Sways in the wind (drawn over the base image, see `compose`).
fn is_plant(mats: &MaterialTable, c: Cell) -> bool {
    !c.is_air() && mats.phys(c.material).kind == Kind::Plant && !mats.def(c.material).still
}

pub(crate) fn cell_rgba(mats: &MaterialTable, c: Cell, ambient: i32, dim: f32, lx: usize, ly: usize) -> [u8; 4] {
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
        if mats.is_charred(c) {
            // Charred: black with glowing embers, so you can see where a
            // trunk is about to give way.
            let ember = n < 30;
            let char_rgb = [34.0, 24.0, 20.0];
            for (ch, (f, k)) in rgba.iter_mut().zip(flame.into_iter().zip(char_rgb)) {
                *ch = if ember { (k * 0.3 + f * 0.7) as u8 } else { (*ch as f32 * 0.15 + k * 0.85) as u8 };
            }
        } else {
            // Blackening as it burns down.
            let burnt = 1.0 - c.life as f32 / ph.burn_time.max(1) as f32;
            for (ch, f) in rgba.iter_mut().zip(flame) {
                *ch = (*ch as f32 * 0.35 * (1.0 - 0.6 * burnt) + f * 0.65) as u8;
            }
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

/// Cells below the ground as generated before a hole in the background shows
/// rock rather than sky (so a crater at the surface still opens to the sky).
const BACKDROP_BELOW: i32 = 16;

/// What shows through a hole in the background underground, until there is a
/// real far background (DESIGN: parallax layers per band): dark rock, earthy
/// near the top and colder with depth, in faint strata.
fn backdrop(x: i32, y: i32, depth: i32) -> [u8; 4] {
    let (earth, rock, deep) = ([46.0, 33.0, 24.0], [40.0, 40.0, 46.0], [27.0, 27.0, 35.0]);
    let mix = |a: [f32; 3], b: [f32; 3], t: f32| [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t.clamp(0.0, 1.0));
    let base = if depth < 120 { mix(earth, rock, (depth - BACKDROP_BELOW) as f32 / 100.0) } else { mix(rock, deep, (depth - 120) as f32 / 1500.0) };
    // Strata: bands a few cells thick that wander a little, plus grain.
    let band = platypus_sim::rng::hash(&[((y + (x >> 5) % 3) >> 2) as u64]) % 9;
    let grain = platypus_sim::rng::hash(&[x as u64, y as u64]) % 5;
    let k = 0.9 + 0.025 * band as f32 + 0.02 * grain as f32;
    let [r, g, b] = base.map(|c| (c * k).min(255.0) as u8);
    [r, g, b, 255]
}

/// Rebuild a layer's base image and plant list from its cells. `ground`: the
/// generated surface height of each of the chunk's columns (background layers
/// of generated worlds), for the backdrop.
fn rebuild(layer: &mut Layer, cells: &[Cell], mats: &MaterialTable, origin: CellPos, climate: &Climate, back: bool, ground: Option<&[i32]>) {
    layer.base.fill(0);
    layer.plants.clear();
    layer.glints.clear();
    for ly in 0..N {
        // (A chunk has one entry in the across-the-world table.)
        let ambient = climate.ambient(origin.x, origin.y + ly as i32);
        for lx in 0..N {
            let c = cells[ly * N + lx];
            if c.is_air() {
                if let Some(ground) = ground {
                    let (x, y) = (origin.x + lx as i32, origin.y + ly as i32);
                    let depth = ground[lx] - y;
                    if depth >= BACKDROP_BELOW {
                        layer.base[px(lx, ly)..px(lx, ly) + 4].copy_from_slice(&backdrop(x, y, depth));
                    }
                }
                continue;
            }
            let dim = if back { bg_dim(mats, c) } else { 1.0 };
            let rgba = cell_rgba(mats, c, ambient, dim, lx, ly);
            let def = mats.def(c.material);
            if (def.glint > 0 && !back) || def.motes {
                let c = mats.color(c);
                layer.glints.push(Glint { x: lx as u8, y: ly as u8, rate: if back { 0 } else { def.glint }, motes: def.motes, rgb: [c[0], c[1], c[2]] });
            }
            if !back && is_plant(mats, c) && c.flags & flags::BURNING == 0 {
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

/// Base + grass shifted by wind and springs, into the image. Each blade bends
/// with its height above the root, so the tip moves most.
fn compose(layer: &Layer, data: &mut [u8], origin: (i32, i32), sway: &Sway, springs: &FoliageSprings) {
    data.copy_from_slice(&layer.base);
    for p in &layer.plants {
        let (wx, wy) = (origin.0 + p.x as i32, origin.1 + p.y as i32);
        let wave = (sway.t * 2.4 + wx as f32 * 0.11).sin();
        let lean = sway.wind * 1.2 + wave * (0.35 + 0.7 * sway.wind.abs());
        let offset = (lean + springs.disp(wx, wy)) * (p.lift as f32 / 5.0).min(1.6);
        let x = (p.x as i32 + offset.round().clamp(-4.0, 4.0) as i32).clamp(0, CHUNK - 1) as usize;
        let i = px(x, p.y as usize);
        data[i..i + 4].copy_from_slice(&p.rgba);
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
    Layer { entity, image, base: vec![0; N * N * 4], plants: Vec::new(), glints: Vec::new() }
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
            rebuild(&mut g.front, chunk.cells(), mats, o, &climate, false, None);
            let ground: Option<Vec<i32>> = (0..N as i32).map(|lx| sim.generator.surface_hint(o.x + lx)).collect();
            rebuild(&mut g.back, chunk.background(), mats, o, &climate, true, ground.as_deref());
        }
        let visible = view.is_none_or(|(lo, hi)| {
            let (x, y) = (o.x as f32, o.y as f32);
            x + CHUNK as f32 >= lo.x && x <= hi.x && y + CHUNK as f32 >= lo.y && y <= hi.y
        });
        for layer in [&g.front, &g.back] {
            let animate = sway_now && visible && !layer.plants.is_empty();
            if !(dirty || animate) {
                continue;
            }
            let Some(mut image) = images.get_mut(&layer.image) else { continue };
            let Some(data) = image.data.as_mut() else { continue };
            compose(layer, data, (o.x, o.y), &sway, &springs);
        }
    }
}

/// Creatures in foliage part it: every tile near a body is pulled towards a
/// pose bent away from the body (strongest right next to it) and along its
/// direction of travel. When the body leaves, the springs wobble back.
/// Crystals and gems glint; glowing fungi shed motes that drift up. Only
/// on screen; each cell by chance, so a big crystal twinkles all over.
fn sparkle(
    mut commands: Commands,
    time: Res<Time>,
    gfx: Res<ChunkGfxs>,
    camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>,
    mut live: Query<(Entity, &mut Sparkle, &mut Sprite, &mut Transform)>,
    mut seed: Local<u64>,
) {
    let dt = time.delta_secs();
    let mut count = 0;
    for (e, mut s, mut sprite, mut tf) in &mut live {
        s.age += dt;
        if s.age >= s.life {
            commands.entity(e).despawn();
            continue;
        }
        count += 1;
        tf.translation += (s.vel * dt).extend(0.0);
        let a = (std::f32::consts::PI * s.age / s.life).sin();
        sprite.color = Color::srgba(s.rgb[0], s.rgb[1], s.rgb[2], a);
    }
    let Ok((tf, loader)) = camera.single() else { return };
    let c = tf.translation().truncate();
    let (lo, hi) = (c - loader.half_extent, c + loader.half_extent);
    let mut roll = || {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (*seed >> 33) as f32 / (1u64 << 31) as f32
    };
    for (pos, g) in &gfx.0 {
        let o = pos.origin();
        if (o.x as f32) > hi.x || (o.x + CHUNK) as f32 <= lo.x || (o.y as f32) > hi.y || (o.y + CHUNK) as f32 <= lo.y {
            continue;
        }
        for layer in [&g.front, &g.back] {
            for gl in &layer.glints {
                if count >= MAX_SPARKLES {
                    return;
                }
                let at = Vec2::new(o.x as f32 + gl.x as f32 + 0.5, o.y as f32 + gl.y as f32 + 0.5);
                let whiten = |c: u8| (c as f32 / 255.0 * 0.4 + 0.6).min(1.0);
                if gl.rate > 0 && roll() < gl.rate as f32 / 255.0 * dt / 10.0 {
                    // A glint: a little cross of light.
                    let rgb = gl.rgb.map(whiten);
                    let life = 0.25 + roll() * 0.3;
                    for size in [Vec2::new(3.0, 1.0), Vec2::new(1.0, 3.0)] {
                        commands.spawn((Sparkle { age: 0.0, life, vel: Vec2::ZERO, rgb }, Sprite::from_color(Color::NONE, size), Transform::from_translation(at.extend(Z_SPARKLES))));
                    }
                    count += 2;
                }
                if gl.motes && roll() < dt / 60.0 {
                    let rgb = gl.rgb.map(|c| (c as f32 / 255.0 * 0.7 + 0.3).min(1.0));
                    let vel = Vec2::new((roll() - 0.5) * 4.0, 2.0 + roll() * 4.0);
                    commands.spawn((Sparkle { age: 0.0, life: 2.0 + roll() * 2.0, vel, rgb }, Sprite::from_color(Color::NONE, Vec2::ONE), Transform::from_translation(at.extend(Z_SPARKLES))));
                    count += 1;
                }
            }
        }
    }
}

fn excite_foliage(time: Res<Time>, mut springs: ResMut<FoliageSprings>, bodies: Query<&Kinematics>) {
    let dt = time.delta_secs().min(0.05);
    for k in &bodies {
        springs.push(&k.body, dt);
    }
    springs.relax(dt);
}

impl FoliageSprings {
    /// How far (cells beyond its half-width) a body parts the foliage.
    const REACH: f32 = 9.0;

    fn push(&mut self, b: &platypus_physics::Body, dt: f32) {
        let speed = b.vel.length();
        let reach = b.half.x + Self::REACH;
        let (x0, x1) = ((b.pos.x - reach) as i32, (b.pos.x + reach) as i32);
        let (y0, y1) = ((b.pos.y - b.half.y - 2.0) as i32, (b.pos.y + b.half.y) as i32);
        // A body in the grass holds it parted; moving through, parts it further
        // and drags it along the direction of travel.
        let effort = (0.75 + speed / 400.0).min(1.2);
        for col in (x0 >> SPRING_COL_BITS)..=(x1 >> SPRING_COL_BITS) {
            let cx = ((col << SPRING_COL_BITS) + 1) as f32;
            let dx = cx - b.pos.x;
            let edge = 1.0 - dx.abs() / reach;
            if edge <= 0.0 {
                continue;
            }
            let close = edge.powf(0.6);
            let target = dx.signum() * 4.0 * close * effort + b.vel.x * 0.01;
            for row in (y0 >> SPRING_ROW_BITS)..=(y1 >> SPRING_ROW_BITS) {
                let s = self.springs.entry((col, row)).or_default();
                // Strong enough to win against the grass's own springiness.
                s.vel += (target - s.disp) * 450.0 * close * dt;
            }
        }
    }

    /// Underdamped: a few wobbles, then rest (settled springs are dropped).
    fn relax(&mut self, dt: f32) {
        const STIFFNESS: f32 = 55.0;
        const DAMPING: f32 = 5.0;
        self.springs.retain(|_, s| {
            s.vel += (-STIFFNESS * s.disp - DAMPING * s.vel) * dt;
            s.disp = (s.disp + s.vel * dt).clamp(-4.0, 4.0);
            s.disp.abs() > 0.02 || s.vel.abs() > 0.05
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platypus_physics::Body;

    const DT: f32 = 1.0 / 120.0;

    #[test]
    fn grass_parts_visibly_around_someone_in_it_and_settles_after() {
        let mut springs = FoliageSprings::default();
        let player = Body::new(Vec2::new(100.0, 20.0), Vec2::new(8.0, 16.0));
        for _ in 0..30 {
            springs.push(&player, DT);
            springs.relax(DT);
        }
        // Grass a few cells to the right of the player (~blade height 5).
        let right = springs.disp(106, 14);
        let left = springs.disp(93, 14);
        assert!(right > 1.5, "grass beside the player bends away visibly: {right}");
        assert!(left < -1.5, "…on both sides: {left}");
        for _ in 0..360 {
            springs.relax(DT);
        }
        assert!(springs.springs.is_empty(), "and it all settles within 3 s once they leave");
    }

    #[test]
    fn running_past_swings_the_grass_and_it_wobbles_back() {
        let mut springs = FoliageSprings::default();
        let mut runner = Body::new(Vec2::new(100.0, 20.0), Vec2::new(8.0, 16.0));
        runner.vel = Vec2::new(95.0, 0.0);
        // Follow one patch of grass the runner passes at x = 130.
        // A swing = clearly bent (>0.1 cell) the opposite way from the last clear bend.
        let (mut peak, mut last_side, mut swings) = (0.0f32, 0.0f32, 0);
        for frame in 0..360 {
            if frame < 90 {
                runner.pos.x += runner.vel.x * DT;
                springs.push(&runner, DT);
            }
            springs.relax(DT);
            let d = springs.disp(130, 14);
            peak = peak.max(d.abs());
            let gone = runner.pos.x - runner.half.x - FoliageSprings::REACH > 131.0;
            if d.abs() > 0.1 {
                if gone && last_side != 0.0 && d.signum() != last_side {
                    swings += 1;
                }
                last_side = d.signum();
            }
        }
        assert!(peak > 2.0, "the grass swings well over a cell as the runner passes: {peak}");
        assert!(swings >= 2, "and wobbles back and forth after ({swings} swings)");
    }
}
