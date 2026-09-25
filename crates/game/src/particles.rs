//! Draws every particle in flight as one dynamic mesh (one draw call).
//! The particles themselves live in the simulation (`platypus_sim::particles`).

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::sprite_render::AlphaMode2d;
use platypus_sim::cell::flags;
use platypus_sim::{Landing, MaterialId};

use crate::world::SimWorld;

pub struct ParticlesPlugin;

/// Above the world texture, below creatures.
const Z_PARTICLES: f32 = 5.0;

/// Particles per mesh. One huge mesh (30 000 particles, 120 000 vertices)
/// silently stopped drawing, so they are split across several.
const PER_MESH: usize = 4_096;

/// Meshes in draw order; more are made as needed.
#[derive(Resource, Default)]
struct ParticleMeshes {
    meshes: Vec<Handle<Mesh>>,
    material: Option<Handle<ColorMaterial>>,
}

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ParticleMeshes>().add_systems(PostUpdate, rebuild_meshes);
    }
}

fn empty_mesh() -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; 4]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0f32; 4]; 4]);
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

fn spawn_mesh(commands: &mut Commands, meshes: &mut Assets<Mesh>, material: Handle<ColorMaterial>) -> Handle<Mesh> {
    let handle = meshes.add(empty_mesh());
    commands.spawn((
        Name::new("Particles"),
        Mesh2d(handle.clone()),
        MeshMaterial2d(material),
        Transform::from_xyz(0.0, 0.0, Z_PARTICLES),
        // The mesh changes every frame; its bounds would go stale.
        NoFrustumCulling,
    ));
    handle
}

fn rebuild_meshes(
    mut commands: Commands,
    sim: Res<SimWorld>,
    mut set: ResMut<ParticleMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let particles = sim.world.particles();
    let batches = particles.len().div_ceil(PER_MESH).max(1);
    let set = &mut *set;
    let material = set.material.get_or_insert_with(|| materials.add(ColorMaterial { alpha_mode: AlphaMode2d::Blend, ..default() })).clone();
    while set.meshes.len() < batches {
        set.meshes.push(spawn_mesh(&mut commands, &mut meshes, material.clone()));
    }
    for (b, handle) in set.meshes.iter().enumerate() {
        let Some(mut mesh) = meshes.get_mut(handle) else { continue };
        let lo = (b * PER_MESH).min(particles.len());
        let hi = ((b + 1) * PER_MESH).min(particles.len());
        fill_mesh(&mut mesh, &particles[lo..hi], &sim);
    }
}

fn fill_mesh(mesh: &mut Mesh, particles: &[platypus_sim::Particle], sim: &SimWorld) {
    let mats = sim.materials();
    let fire = mats.fire();
    let mut pos = Vec::with_capacity(particles.len() * 4);
    let mut col = Vec::with_capacity(particles.len() * 4);
    let mut idx = Vec::with_capacity(particles.len() * 6);
    for (i, p) in particles.iter().enumerate() {
        let mut rgba = mats.color(p.cell);
        let hot = p.cell.flags & flags::BURNING != 0 || p.landing == Landing::Ember || (fire != MaterialId::AIR && p.cell.material == fire);
        if hot {
            // Embers and sparks: bright, flickering from orange to yellow.
            let t = ((i as u32 * 2654435761) ^ (p.life as u32 * 40503)) % 3;
            rgba = [[255, 150, 30, 255], [255, 210, 90, 255], [255, 110, 20, 255]][t as usize];
        } else if p.cell.heat > 350 {
            let k = ((p.cell.heat as f32 - 350.0) / 900.0).clamp(0.0, 1.0);
            let hot_col = [255.0, 120.0 + 100.0 * k, 30.0 + 60.0 * k];
            for (ch, h) in rgba.iter_mut().zip(hot_col) {
                *ch = (*ch as f32 + (h - *ch as f32) * (0.4 + 0.5 * k)) as u8;
            }
        }
        // Dust fades out over its short life.
        if p.landing == Landing::Vanish && !hot {
            rgba[3] = (p.life.min(20) * 12) as u8;
        }
        // Rain: pale streaks as long as the drop is fast. Snow: white flecks.
        let mut tall = 1.0;
        match p.landing {
            Landing::Rain => {
                // Fading in as it leaves the cloud (rain drops live 600 ticks).
                let age = 600u16.saturating_sub(p.life);
                rgba = [196, 214, 236, (120 * age.min(10) / 10) as u8];
                tall = (1.0 + p.vel[1].abs() * 0.6).min(5.0).floor();
            }
            Landing::Snow => rgba = [244, 246, 250, 230],
            _ => {}
        }
        let c = Color::srgba_u8(rgba[0], rgba[1], rgba[2], rgba[3]).to_linear().to_f32_array();
        let (x, y) = (p.pos[0].floor(), p.pos[1].floor());
        let base = (i * 4) as u32;
        pos.extend([[x, y, 0.0], [x + 1.0, y, 0.0], [x + 1.0, y + tall, 0.0], [x, y + tall, 0.0]]);
        col.extend([c; 4]);
        idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    if pos.is_empty() {
        // Bevy's mesh allocator mishandles meshes with no vertices; keep one
        // invisible quad instead.
        pos.extend([[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]);
        col.extend([[0.0; 4]; 4]);
        idx.extend([0, 1, 2, 0, 2, 3]);
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
    mesh.insert_indices(Indices::U32(idx));
}
