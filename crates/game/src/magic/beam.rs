//! Beams (`Carrier::Beam`): an instant line from the hand, held. Each tick
//! it's held it reaches from the hand to the first thing in its way (a
//! solid, a liquid, a plant: anything but air, gas and flame; or a body),
//! and what it carries goes off there: a fire ray heats and lights, a frost
//! ray freezes (water to an ice bridge, lava to stone), the vaporiser eats
//! the cells it plays on, hardest last.
//!
//! Drawn a cell at a time on a canvas above the light overlay (a beam is
//! light: it doesn't darken in the dark), with a light where it ends.

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind, World};

use super::runes::Cast;
use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprite, CanvasSprites};
use crate::light::LightSource;
use crate::world::{ChunkLoader, SimWorld};

/// Above the light overlay (15), below glowing eyes.
const Z_BEAMS: f32 = 15.8;

/// Where a beam from `from` along `dir` stops, out to `range`: at the
/// first cell in its way, or the first body (not `caster`'s) it meets.
/// The tip, and the body there.
pub fn trace(world: &World, bodies: impl Iterator<Item = (Entity, Vec2, Vec2)>, caster: Entity, from: Vec2, dir: Vec2, range: f32) -> (Vec2, Option<Entity>) {
    let mats = world.materials();
    // Only bodies near the line are worth looking at, each step.
    let near: Vec<(Entity, Vec2, Vec2)> = bodies
        .filter(|&(e, pos, half)| {
            let t = (pos - from).dot(dir).clamp(0.0, range);
            e != caster && (from + dir * t).distance(pos) <= half.max_element() + 1.0
        })
        .collect();
    let steps = (range * 2.0) as i32;
    for i in 1..=steps {
        let p = from + dir * (i as f32 * 0.5);
        if let Some(&(e, ..)) = near.iter().find(|(_, pos, half)| ((p - *pos).abs() - *half).max_element() < 0.5) {
            return (p, Some(e));
        }
        let open = world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| c.is_air() || matches!(mats.phys(c.material).kind, Kind::Gas | Kind::Fire | Kind::Empty));
        if !open {
            return (p, None);
        }
    }
    (from + dir * range, None)
}

/// A beam drawn this tick: from, to, cells across, colour, and the tick.
pub struct Line {
    pub from: Vec2,
    pub to: Vec2,
    pub width: f32,
    pub color: (u8, u8, u8),
    pub tick: u64,
}

/// The beams held now (each tick's), for drawing.
#[derive(Resource, Default)]
pub struct Beams {
    pub lines: Vec<Line>,
}

impl Beams {
    pub fn add(&mut self, cast: &Cast, from: Vec2, to: Vec2, width: f32, tick: u64) {
        self.lines.push(Line { from, to, width, color: cast.color, tick });
    }
}

#[derive(Resource)]
pub struct BeamCanvas(Canvas);

impl Default for BeamCanvas {
    fn default() -> Self {
        BeamCanvas(Canvas::new("Beams", Z_BEAMS))
    }
}

/// The lights at beams' tips (one each, kept for the next).
#[derive(Resource, Default)]
pub struct BeamLights(Vec<Entity>);

/// Draw the beams held this tick or the last (a frame between two ticks
/// still shows it); forget older ones.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    mut commands: Commands,
    sim: Res<SimWorld>,
    time: Res<Time>,
    mut beams: ResMut<Beams>,
    mut canvas: ResMut<BeamCanvas>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>,
    mut sprites: CanvasSprites,
    mut lights: ResMut<BeamLights>,
    mut glow: Query<(&mut LightSource, &mut Transform), Without<CanvasSprite>>,
) {
    let tick = sim.world.tick();
    beams.lines.retain(|l| l.tick + 1 >= tick);
    // (Of each caster's beam, the latest: two ticks in a frame drew two.)
    let mut lines: Vec<&Line> = Vec::new();
    for l in beams.lines.iter().rev() {
        if !lines.iter().any(|m| m.from.distance(l.from) < 6.0 && m.color == l.color) {
            lines.push(l);
        }
    }
    // A light at each tip.
    while lights.0.len() < lines.len() {
        lights.0.push(commands.spawn((Name::new("Beam light"), LightSource { color: [0.0; 3], flicker: 0.2 }, Transform::default())).id());
    }
    for (i, &e) in lights.0.iter().enumerate() {
        let Ok((mut light, mut tf)) = glow.get_mut(e) else { continue };
        match lines.get(i) {
            Some(l) => {
                let (r, g, b) = l.color;
                light.color = [r as f32 / 255.0 * 2.2, g as f32 / 255.0 * 2.2, b as f32 / 255.0 * 2.2];
                tf.translation = l.to.extend(0.0);
            }
            None => light.color = [0.0; 3],
        }
    }
    let Ok((cam, loader)) = camera.single() else { return };
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, !lines.is_empty()) else { return };
    // (It shimmers: each cell's brightness changes every frame.)
    let frame = (time.elapsed_secs() * 60.0) as u32;
    for l in &lines {
        let d = l.to - l.from;
        let len = d.length();
        if len < 0.5 {
            continue;
        }
        let dir = d / len;
        let side = dir.perp();
        let (r, g, b) = l.color;
        let core = [(r as u16 + 510) / 3, (g as u16 + 510) / 3, (b as u16 + 510) / 3].map(|c| c.min(255) as u8);
        let half = l.width / 2.0;
        let steps = (len * 2.0) as i32;
        for s in 0..=steps {
            let at = l.from + dir * (s as f32 * 0.5);
            let mut o = -half;
            while o <= half + 0.01 {
                let p = (at + side * o).floor().as_ivec2();
                let edge = o.abs() / half.max(0.5);
                let h = (p.x as u32).wrapping_mul(73856093) ^ (p.y as u32).wrapping_mul(19349663) ^ frame.wrapping_mul(83492791);
                let flicker = 170 + (h % 86) as u8;
                let c = if edge < 0.45 { [core[0], core[1], core[2], 255] } else { [r, g, b, flicker] };
                px.put(p.x, p.y, c);
                o += 0.5;
            }
        }
    }
}
