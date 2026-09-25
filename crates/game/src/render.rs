//! One texture and one sprite per loaded chunk (SPEC §4). A chunk is
//! re-coloured only when its cells changed; nothing here scales with how far
//! or how fast anyone moves.

use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use platypus_sim::cell::flags;
use platypus_sim::{CHUNK, CellPos, ChunkPos, Kind};
use platypus_worldgen::Backdrop;

use crate::world::SimWorld;

pub struct ChunkRenderPlugin;

/// Marks the sprite that shows a chunk.
#[derive(Component)]
pub struct ChunkSprite;

#[derive(Resource, Default)]
struct ChunkSprites(HashMap<ChunkPos, (Entity, Handle<Image>)>);

/// Colour behind empty cells.
const SKY: [u8; 4] = [0, 0, 0, 0];
const CAVE: [u8; 4] = [26, 20, 18, 255];
pub const SKY_COLOR: Color = Color::srgb(0.42, 0.66, 0.92);

/// World layer depths.
pub const Z_WORLD: f32 = 0.0;

impl Plugin for ChunkRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(SKY_COLOR))
            .init_resource::<ChunkSprites>()
            .add_systems(PostUpdate, sync_chunk_sprites);
    }
}

fn sync_chunk_sprites(
    mut commands: Commands,
    sim: Res<SimWorld>,
    mut sprites: ResMut<ChunkSprites>,
    mut images: ResMut<Assets<Image>>,
) {
    // Despawn sprites whose chunk was unloaded.
    sprites.0.retain(|pos, (entity, handle)| {
        let keep = sim.world.is_loaded(*pos);
        if !keep {
            commands.entity(*entity).despawn();
            images.remove(handle.id());
        }
        keep
    });

    let mats = sim.materials();
    let climate = sim.world.climate();
    for chunk in sim.world.chunks() {
        let fresh = !sprites.0.contains_key(&chunk.pos);
        if !chunk.take_render_dirty() && !fresh {
            continue;
        }
        let handle = match sprites.0.get(&chunk.pos) {
            Some((_, h)) => h.clone(),
            None => {
                let image = Image::new_fill(
                    Extent3d { width: CHUNK as u32, height: CHUNK as u32, depth_or_array_layers: 1 },
                    TextureDimension::D2,
                    &SKY,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                );
                let handle = images.add(image);
                let origin = chunk.pos.origin();
                let half = CHUNK as f32 / 2.0;
                let entity = commands
                    .spawn((
                        Sprite { image: handle.clone(), custom_size: Some(Vec2::splat(CHUNK as f32)), ..default() },
                        Transform::from_xyz(origin.x as f32 + half, origin.y as f32 + half, Z_WORLD),
                        ChunkSprite,
                    ))
                    .id();
                sprites.0.insert(chunk.pos, (entity, handle.clone()));
                handle
            }
        };
        let Some(mut image) = images.get_mut(&handle) else { continue };
        let Some(data) = image.data.as_mut() else { continue };
        let origin = chunk.pos.origin();
        let cells = chunk.cells();
        for ly in 0..CHUNK as usize {
            // Texture row 0 is the top; chunk row 0 is the bottom.
            let row = (CHUNK as usize - 1 - ly) * CHUNK as usize * 4;
            for lx in 0..CHUNK as usize {
                let cell = cells[ly * CHUNK as usize + lx];
                let rgba = if cell.is_air() {
                    match sim.generator.backdrop(CellPos::new(origin.x + lx as i32, origin.y + ly as i32)) {
                        Backdrop::Sky => SKY,
                        Backdrop::Cave => CAVE,
                    }
                } else {
                    let mut rgba = mats.color(cell);
                    let ph = mats.phys(cell.material);
                    // Mining damage on solids shows as darkening cracks.
                    if cell.life > 0 && matches!(ph.kind, Kind::Static | Kind::Powder) && ph.hardness > 0 {
                        let k = 1.0 - 0.6 * (cell.life as f32 / ph.hardness as f32).min(1.0);
                        for ch in &mut rgba[..3] {
                            *ch = (*ch as f32 * k) as u8;
                        }
                    }
                    if cell.heat > 300 {
                        glow(&mut rgba, climate.ambient(origin.y + ly as i32) + cell.heat as i32);
                    }
                    if cell.flags & flags::BURNING != 0 {
                        // Flicker: a per-cell mix toward flame colours that changes as it burns down.
                        let n = ((lx as u32 * 73856093) ^ (ly as u32 * 19349663) ^ (cell.life as u32 * 83492791)) % 100;
                        let flame = if n < 45 { [255.0, 120.0, 20.0] } else if n < 80 { [255.0, 190.0, 60.0] } else { [200.0, 50.0, 10.0] };
                        for (ch, f) in rgba.iter_mut().zip(flame) {
                            *ch = (*ch as f32 * 0.35 + f * 0.65) as u8;
                        }
                    }
                    rgba
                };
                data[row + lx * 4..row + lx * 4 + 4].copy_from_slice(&rgba);
            }
        }
    }
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
