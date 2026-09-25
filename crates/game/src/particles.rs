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

#[derive(Component)]
struct ParticleMesh(Handle<Mesh>);

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_mesh).add_systems(PostUpdate, rebuild_mesh);
    }
}

fn empty_mesh() -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new());
    mesh.insert_indices(Indices::U32(Vec::new()));
    mesh
}

fn spawn_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>) {
    let handle = meshes.add(empty_mesh());
    commands.spawn((
        Name::new("Particles"),
        Mesh2d(handle.clone()),
        MeshMaterial2d(materials.add(ColorMaterial { alpha_mode: AlphaMode2d::Blend, ..default() })),
        Transform::from_xyz(0.0, 0.0, Z_PARTICLES),
        // The mesh changes every frame; its bounds would go stale.
        NoFrustumCulling,
        ParticleMesh(handle),
    ));
}

fn rebuild_mesh(sim: Res<SimWorld>, q: Single<&ParticleMesh>, mut meshes: ResMut<Assets<Mesh>>) {
    let Some(mut mesh) = meshes.get_mut(&q.0) else { return };
    let particles = sim.world.particles();
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
        let c = Color::srgba_u8(rgba[0], rgba[1], rgba[2], rgba[3]).to_linear().to_f32_array();
        let (x, y) = (p.pos[0].floor(), p.pos[1].floor());
        let base = (i * 4) as u32;
        pos.extend([[x, y, 0.0], [x + 1.0, y, 0.0], [x + 1.0, y + 1.0, 0.0], [x, y + 1.0, 0.0]]);
        col.extend([c; 4]);
        idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
    mesh.insert_indices(Indices::U32(idx));
}
