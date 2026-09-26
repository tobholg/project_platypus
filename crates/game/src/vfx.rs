//! Sparks: visual-only effects (spell trails, impact bursts, crackle), not
//! part of the simulation, drawn as one dynamic mesh like the sim's
//! particles. What a spell looks like is data (`runes.ron`, `Look`); this
//! only moves and draws the sparks it asks for. Also the soft halo image
//! spells glow with.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite_render::AlphaMode2d;
use platypus_sim::rng::Rng;

use crate::magic::runes::Emitter;

pub struct VfxPlugin;

/// Above creatures and spells, below the light's flashes.
const Z_SPARKS: f32 = 13.0;
/// Most sparks alive (the oldest go first).
const MAX_SPARKS: usize = 8_000;
/// Sparks per mesh (see `particles::PER_MESH`).
const PER_MESH: usize = 4_096;
/// A glowing spark's halo, times its size, and how opaque.
const HALO: f32 = 3.0;
const HALO_ALPHA: f32 = 0.22;

struct Spark {
    pos: Vec2,
    vel: Vec2,
    age: f32,
    life: f32,
    colors: Vec<[f32; 3]>,
    gravity: f32,
    drag: f32,
    size: f32,
    jitter: f32,
    glow: bool,
}

/// Sparks alive.
#[derive(Resource)]
pub struct Sparks {
    live: Vec<Spark>,
    /// Drawn this frame only (a well's held cells): where, how big, colour.
    now: Vec<([f32; 2], f32, [f32; 4])>,
    rng: Rng,
}

impl Default for Sparks {
    fn default() -> Self {
        Sparks { live: Vec::new(), now: Vec::new(), rng: Rng::seeded(&[0x5EA4]) }
    }
}

impl Sparks {
    /// A square drawn this frame only.
    pub fn draw_now(&mut self, at: Vec2, size: f32, rgba: [f32; 4]) {
        self.now.push(([at.x, at.y], size, rgba));
    }

    /// `n` sparks of an emitter at `at`, heading `dir` (± its spread),
    /// moving with `carry` besides (a trail keeps some of its spell's speed).
    pub fn emit(&mut self, e: &Emitter, n: usize, at: Vec2, dir: Vec2, carry: Vec2) {
        let colors: Vec<[f32; 3]> = e.colors.iter().map(|&(r, g, b)| [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]).collect();
        if colors.is_empty() {
            return;
        }
        let rng = &mut self.rng;
        let unit = |rng: &mut Rng| rng.next_u32() as f32 / u32::MAX as f32;
        for _ in 0..n {
            let a = (unit(rng) * 2.0 - 1.0) * e.spread;
            let speed = e.speed * (0.3 + 0.7 * unit(rng));
            let jiggle = Vec2::new(unit(rng) - 0.5, unit(rng) - 0.5);
            self.live.push(Spark {
                pos: at + jiggle,
                vel: Vec2::from_angle(a).rotate(dir.normalize_or(Vec2::Y)) * speed + carry,
                age: 0.0,
                life: e.life.0 + (e.life.1 - e.life.0) * unit(rng),
                colors: colors.clone(),
                gravity: e.gravity,
                drag: e.drag,
                size: e.size,
                jitter: e.jitter,
                glow: e.glow,
            });
        }
        if self.live.len() > MAX_SPARKS {
            let excess = self.live.len() - MAX_SPARKS;
            self.live.drain(..excess);
        }
    }

    /// A trail's sparks for `cells` flown: `count` a cell, the fraction by
    /// chance.
    pub fn trail(&mut self, e: &Emitter, cells: f32, at: Vec2, back: Vec2, carry: Vec2) {
        let want = e.count * cells;
        let whole = want.floor();
        let extra = (self.rng.next_u32() as f32 / u32::MAX as f32) < want - whole;
        self.emit(e, whole as usize + extra as usize, at, back, carry);
    }
}

/// The soft round glow spells (and anything else) can wear: white, fading
/// to nothing at the edge; tint it with the sprite's colour.
#[derive(Resource)]
pub struct Halo(pub Handle<Image>);

#[derive(Resource, Default)]
struct SparkMeshes {
    meshes: Vec<Handle<Mesh>>,
    material: Option<Handle<ColorMaterial>>,
}

impl Plugin for VfxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Sparks>()
            .init_resource::<SparkMeshes>()
            .add_systems(Startup, make_halo)
            .add_systems(Update, (step_sparks, air_puffs))
            .add_systems(PostUpdate, draw_sparks);
    }
}

fn make_halo(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    const N: u32 = 32;
    let mut data = Vec::with_capacity((N * N * 4) as usize);
    for y in 0..N {
        for x in 0..N {
            let d = Vec2::new(x as f32 + 0.5 - N as f32 / 2.0, y as f32 + 0.5 - N as f32 / 2.0).length() / (N as f32 / 2.0);
            let a = (1.0 - d).clamp(0.0, 1.0).powf(1.8);
            data.extend([255, 255, 255, (a * 255.0) as u8]);
        }
    }
    let image = Image::new(
        Extent3d { width: N, height: N, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    commands.insert_resource(Halo(images.add(image)));
}

fn step_sparks(time: Res<Time>, mut sparks: ResMut<Sparks>) {
    let dt = time.delta_secs().min(0.05);
    let sparks = &mut *sparks;
    let rng = &mut sparks.rng;
    sparks.live.retain_mut(|s| {
        s.age += dt;
        if s.age >= s.life {
            return false;
        }
        s.vel.y -= s.gravity * dt;
        s.vel *= (1.0 - s.drag * dt).max(0.0);
        if s.jitter > 0.0 {
            let j = Vec2::new(rng.next_u32() as f32 / u32::MAX as f32 - 0.5, rng.next_u32() as f32 / u32::MAX as f32 - 0.5);
            s.pos += j * s.jitter * dt;
        }
        s.pos += s.vel * dt;
        true
    });
}

/// Its colour now: along its colours over its life, fading out over the
/// last third.
fn color_of(s: &Spark) -> [f32; 4] {
    let f = (s.age / s.life).clamp(0.0, 1.0);
    let n = s.colors.len();
    let rgb = if n == 1 {
        s.colors[0]
    } else {
        let x = f * (n - 1) as f32;
        let i = (x.floor() as usize).min(n - 2);
        let k = x - i as f32;
        let (a, b) = (s.colors[i], s.colors[i + 1]);
        [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k]
    };
    let alpha = ((1.0 - f) * 3.0).min(1.0);
    [rgb[0], rgb[1], rgb[2], alpha]
}

fn draw_sparks(mut commands: Commands, mut sparks: ResMut<Sparks>, mut set: ResMut<SparkMeshes>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>) {
    // A glowing spark is two quads (its halo, then it); this frame's own
    // squares go first.
    let mut quads: Vec<([f32; 2], f32, [f32; 4])> = std::mem::take(&mut sparks.now);
    quads.reserve(sparks.live.len() * 2);
    for s in &sparks.live {
        let c = color_of(s);
        if s.glow {
            quads.push(([s.pos.x, s.pos.y], s.size * HALO, [c[0], c[1], c[2], c[3] * HALO_ALPHA]));
        }
        quads.push(([s.pos.x, s.pos.y], s.size, c));
    }
    let batches = quads.len().div_ceil(PER_MESH).max(1);
    let set = &mut *set;
    let material = set.material.get_or_insert_with(|| materials.add(ColorMaterial { alpha_mode: AlphaMode2d::Blend, ..default() })).clone();
    while set.meshes.len() < batches {
        let handle = meshes.add(empty_mesh());
        commands.spawn((Name::new("Sparks"), Mesh2d(handle.clone()), MeshMaterial2d(material.clone()), Transform::from_xyz(0.0, 0.0, Z_SPARKS), NoFrustumCulling));
        set.meshes.push(handle);
    }
    for (b, handle) in set.meshes.iter().enumerate() {
        let Some(mut mesh) = meshes.get_mut(handle) else { continue };
        let lo = (b * PER_MESH).min(quads.len());
        let hi = ((b + 1) * PER_MESH).min(quads.len());
        fill(&mut mesh, &quads[lo..hi]);
    }
}

fn empty_mesh() -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; 4]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0f32; 4]; 4]);
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

fn fill(mesh: &mut Mesh, quads: &[([f32; 2], f32, [f32; 4])]) {
    let mut pos = Vec::with_capacity(quads.len() * 4 + 4);
    let mut col = Vec::with_capacity(quads.len() * 4 + 4);
    let mut idx = Vec::with_capacity(quads.len() * 6 + 6);
    for (i, &([x, y], size, c)) in quads.iter().enumerate() {
        let h = size / 2.0;
        let c = Color::srgba(c[0], c[1], c[2], c[3]).to_linear().to_f32_array();
        let base = (i * 4) as u32;
        pos.extend([[x - h, y - h, 0.0], [x + h, y - h, 0.0], [x + h, y + h, 0.0], [x - h, y + h, 0.0]]);
        col.extend([c; 4]);
        idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    if pos.is_empty() {
        // (An empty mesh is mishandled; one invisible quad instead.)
        pos.extend([[0.0, 0.0, 0.0]; 4]);
        col.extend([[0.0; 4]; 4]);
        idx.extend([0, 1, 2, 0, 2, 3]);
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
    mesh.insert_indices(Indices::U32(idx));
}

/// A double jump's cloud: a soft puff spreading out and down under the
/// feet, and a ring of glowing motes (it lights the dark a moment, `light`).
fn air_puffs(mut jumps: MessageReader<crate::actors::AirJumped>, mut sparks: ResMut<Sparks>) {
    for j in jumps.read() {
        sparks.emit(&PUFF, PUFF.count as usize, j.at, Vec2::NEG_Y, Vec2::ZERO);
        sparks.emit(&PUFF_RING, PUFF_RING.count as usize, j.at, Vec2::X, Vec2::ZERO);
        sparks.emit(&PUFF_RING, PUFF_RING.count as usize, j.at, Vec2::NEG_X, Vec2::ZERO);
    }
}

static PUFF: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 30.0,
    life: (0.3, 0.75),
    colors: vec![(255, 255, 255), (220, 232, 255), (160, 180, 225)],
    speed: 95.0,
    spread: 1.45,
    gravity: -30.0,
    drag: 5.0,
    size: 1.5,
    jitter: 8.0,
    glow: false,
});

static PUFF_RING: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 8.0,
    life: (0.25, 0.5),
    colors: vec![(235, 245, 255), (150, 190, 255)],
    speed: 130.0,
    spread: 0.3,
    gravity: -10.0,
    drag: 6.0,
    size: 1.0,
    jitter: 0.0,
    glow: true,
});
