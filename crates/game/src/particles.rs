//! Draws every particle in flight on screen, as pixels of one image over the
//! camera's view (a particle is a cell: a pixel of it). The particles
//! themselves live in the simulation (`platypus_sim::particles`).
//!
//! (They were quads in a mesh; tens of thousands of them made rebuilding
//! the mesh every frame the costliest thing in a big fight. An image the
//! size of the view costs the same however many there are.)

use bevy::prelude::*;
use platypus_sim::cell::flags;
use platypus_sim::{Landing, MaterialId};

use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprites};
use crate::world::{ChunkLoader, SimWorld};

pub struct ParticlesPlugin;

/// Above the world texture, below creatures.
const Z_PARTICLES: f32 = 5.0;

#[derive(Resource)]
struct ParticleCanvas(Canvas);

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ParticleCanvas(Canvas::new("Particles", Z_PARTICLES))).add_systems(PostUpdate, draw.before(bevy::transform::TransformSystems::Propagate));
    }
}

fn draw(
    mut commands: Commands,
    sim: Res<SimWorld>,
    mut canvas: ResMut<ParticleCanvas>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>,
    mut sprites: CanvasSprites,
) {
    let Ok((cam, loader)) = camera.single() else { return };
    let particles = sim.world.particles();
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, !particles.is_empty()) else { return };
    let mats = sim.materials();
    let fire = mats.fire();
    for (i, p) in particles.iter().enumerate() {
        let mut rgba = mats.color(p.cell);
        let hot = p.cell.flags & flags::BURNING != 0 || p.landing == Landing::Ember || (fire != MaterialId::AIR && p.cell.material == fire);
        if hot {
            // Embers and sparks: bright, flickering from orange to yellow.
            let t = ((i as u32).wrapping_mul(2654435761) ^ (p.life as u32 * 40503)) % 3;
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
        let mut tall = 1;
        match p.landing {
            Landing::Rain => {
                // Fading in as it leaves the cloud (rain drops live 900 ticks).
                let age = 900u16.saturating_sub(p.life);
                rgba = [96, 142, 222, (190 * age.min(10) / 10) as u8];
                tall = (1.0 + p.vel[1].abs()).min(4.0) as i32;
            }
            Landing::Snow => rgba = [244, 246, 250, 230],
            _ => {}
        }
        let (x, y) = (p.pos[0].floor() as i32, p.pos[1].floor() as i32);
        for dy in 0..tall {
            px.put(x, y + dy, rgba);
        }
    }
}
