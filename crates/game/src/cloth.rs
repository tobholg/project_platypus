//! Cloth: capes, cloaks, rags. A drape is a grid of points (`cols` ×
//! `rows`, `gap` cells apart) tied to their neighbours (across, down,
//! corner to corner, and every other one down, stiffer), its top row
//! pinned to its wearer (a span across its shoulders, in front-facing
//! terms: it swings to the other side as it turns). It falls, trails what
//! it hangs from, is pushed by a breeze, rests on rock. Drawn on a canvas
//! just behind the player (and in front of other creatures): each little
//! quad filled, darker where it's bunched into a fold, its lining where it
//! shows its inside, a darker edge; a torn hem (`torn`) ragged.
//!
//! Worn: gear in the `Back` slot with a `cape` (gear.ron). Its own: a
//! creature's code can hang more on it (`Drapes::own`).

use std::sync::Arc;

use bevy::prelude::*;
use platypus_sim::rng::hash;
use platypus_sim::{CellPos, Kind};
use serde::Deserialize;

use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprites};
use crate::creatures::Kinematics;
use crate::gear::Equipment;
use crate::hands::items::Items;
use crate::world::{ChunkLoader, SimWorld, TickSet};

pub struct ClothPlugin;

impl Plugin for ClothPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClothCanvas>()
            .add_systems(Update, (wear, draw))
            .add_systems(FixedUpdate, hang.in_set(TickSet::Bodies).after(crate::creatures::move_creatures));
    }
}

/// A piece of cloth, as written (gear.ron `cape`, or a creature's code).
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ClothDef {
    /// Its outside and its lining.
    pub color: (u8, u8, u8),
    pub lining: (u8, u8, u8),
    /// Points across and down, and how far apart (cells).
    pub cols: usize,
    pub rows: usize,
    pub gap: f32,
    /// Where its top is pinned, from its wearer's middle (cells; + is the
    /// way it faces, and up): across from `pin.0` to `pin.1`, at `pin.2`.
    pub pin: (f32, f32, f32),
    /// How torn its hem is (0: whole; 1: up to its whole length in rags).
    pub torn: f32,
    /// How heavy (1: a cape's; more falls harder, swings less).
    pub weight: f32,
    /// Drawn in front of its wearer (a cloak), not behind (a cape).
    pub front: bool,
    /// How hard it falls away behind its wearer (cells/s²: a cape off the
    /// shoulders, not straight down its back).
    pub behind: f32,
}

impl Default for ClothDef {
    fn default() -> Self {
        ClothDef { color: (150, 38, 36), lining: (96, 26, 28), cols: 5, rows: 9, gap: 2.5, pin: (-4.0, 1.0, 5.0), torn: 0.0, weight: 1.0, front: false, behind: 0.0 }
    }
}

/// One drape hanging on something: its points and where they were.
#[derive(Clone, Debug)]
pub struct Drape {
    pub def: Arc<ClothDef>,
    /// Worn (from its gear: replaced as its gear changes) or its own.
    worn: bool,
    pts: Vec<Vec2>,
    prev: Vec<Vec2>,
    /// Each column's tear (quads off its hem); its seed.
    tears: Vec<usize>,
    /// Facing when last hung (it swings across as it turns).
    facing: f32,
}

impl Drape {
    pub fn new(def: Arc<ClothDef>, seed: u64) -> Drape {
        let tears = (0..def.cols).map(|i| ((hash(&[seed, i as u64, 0xC107]) % 1000) as f32 / 1000.0 * def.torn * (def.rows - 1) as f32) as usize).collect();
        Drape { def, worn: false, pts: Vec::new(), prev: Vec::new(), tears, facing: 1.0 }
    }

    /// Its points (for a creature's code: where its hem is).
    pub fn points(&self) -> &[Vec2] {
        &self.pts
    }
}

/// What's hanging on something.
#[derive(Component, Default)]
pub struct Drapes(pub Vec<Drape>);

impl Drapes {
    /// Hang `def` on it, its own (a creature's code).
    pub fn own(&mut self, def: Arc<ClothDef>, seed: u64) {
        self.0.push(Drape::new(def, seed));
    }
}

/// Its gear changed: what it wears on its back hung on it (and what it
/// took off taken down).
fn wear(mut commands: Commands, items: Option<Res<Items>>, mut q: Query<(Entity, &Equipment, Option<&mut Drapes>), Changed<Equipment>>) {
    let Some(items) = items else { return };
    for (e, eq, drapes) in &mut q {
        let capes: Vec<Arc<ClothDef>> = eq.pieces(&items).filter_map(|s| items.def(s.item).gear.as_ref()?.cape.clone()).map(Arc::new).collect();
        let seed = e.to_bits();
        match drapes {
            Some(mut d) => {
                d.0.retain(|x| !x.worn);
                for c in capes {
                    let mut x = Drape::new(c, seed);
                    x.worn = true;
                    d.0.push(x);
                }
            }
            None if !capes.is_empty() => {
                let list = capes
                    .into_iter()
                    .map(|c| {
                        let mut x = Drape::new(c, seed);
                        x.worn = true;
                        x
                    })
                    .collect();
                commands.entity(e).insert(Drapes(list));
            }
            None => {}
        }
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
/// How hard cloth falls (× its weight), how much of its swing it keeps a
/// tick, the breeze's push (cells/s²); ties pulled true this many times a
/// tick; pinned further than this from where it was (a teleport): hung
/// again from scratch.
const FALL: f32 = 420.0;
const KEEP: f32 = 0.982;
const BREEZE: f32 = 40.0;
const PASSES: usize = 8;
const JUMP: f32 = 40.0;

fn pin_at(def: &ClothDef, pos: Vec2, half: Vec2, facing: f32, i: usize) -> Vec2 {
    let u = if def.cols > 1 { i as f32 / (def.cols - 1) as f32 } else { 0.5 };
    let x = def.pin.0 + (def.pin.1 - def.pin.0) * u;
    let _ = half;
    pos + Vec2::new(x * facing, def.pin.2)
}

/// Each drape falls, swings and settles, pinned to its wearer.
fn hang(sim: Res<SimWorld>, time: Res<Time>, mut q: Query<(&mut Drapes, &Kinematics)>) {
    let t = time.elapsed_secs();
    let solid = |p: Vec2| sim.world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| !c.is_air() && matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder));
    for (mut drapes, k) in &mut q {
        let (pos, half, facing) = (k.body.pos, k.body.half, k.loco.facing);
        for d in drapes.0.iter_mut() {
            let def = d.def.clone();
            let (cols, rows, gap) = (def.cols, def.rows, def.gap);
            let idx = |i: usize, j: usize| j * cols + i;
            let fresh = d.pts.len() != cols * rows || d.pts.first().is_some_and(|p| p.distance(pin_at(&def, pos, half, facing, 0)) > JUMP);
            if fresh {
                d.pts = (0..rows).flat_map(|j| (0..cols).map(move |i| (i, j))).map(|(i, j)| pin_at(&def, pos, half, facing, i) - Vec2::Y * gap * j as f32).collect();
                d.prev = d.pts.clone();
            }
            d.facing = facing;
            // Fall, trail, the breeze.
            for j in 1..rows {
                for i in 0..cols {
                    let n = idx(i, j);
                    let v = (d.pts[n] - d.prev[n]) * KEEP;
                    let breeze = ((t * 1.7 + i as f32 * 0.6 + j as f32 * 0.35).sin() * 0.6 + (t * 0.53).sin() * 0.4) * BREEZE * (j as f32 / rows as f32);
                    d.prev[n] = d.pts[n];
                    let away = -facing * def.behind * (j as f32 / rows as f32);
                    d.pts[n] += v + Vec2::new(breeze + away, -FALL * def.weight) * DT * DT;
                }
            }
            // Ties pulled true; the top row held where it's pinned.
            let ties = |a: usize, b: usize, rest: f32, stiff: f32, pts: &mut Vec<Vec2>, top: usize| {
                let dlt = pts[b] - pts[a];
                let len = dlt.length().max(1e-4);
                let fix = dlt * ((1.0 - rest / len) * stiff);
                match (a < top, b < top) {
                    (true, true) => {}
                    (true, false) => pts[b] -= fix,
                    (false, true) => pts[a] += fix,
                    (false, false) => {
                        pts[a] += fix * 0.5;
                        pts[b] -= fix * 0.5;
                    }
                }
            };
            for _ in 0..PASSES {
                for i in 0..cols {
                    d.pts[idx(i, 0)] = pin_at(&def, pos, half, facing, i);
                }
                for j in 0..rows {
                    for i in 0..cols {
                        if i + 1 < cols {
                            ties(idx(i, j), idx(i + 1, j), gap, 1.0, &mut d.pts, cols);
                        }
                        if j + 1 < rows {
                            ties(idx(i, j), idx(i, j + 1), gap, 1.0, &mut d.pts, cols);
                            if i + 1 < cols {
                                ties(idx(i, j), idx(i + 1, j + 1), gap * std::f32::consts::SQRT_2, 0.5, &mut d.pts, cols);
                                ties(idx(i + 1, j), idx(i, j + 1), gap * std::f32::consts::SQRT_2, 0.5, &mut d.pts, cols);
                            }
                        }
                        if j + 2 < rows {
                            ties(idx(i, j), idx(i, j + 2), gap * 2.0, 0.3, &mut d.pts, cols);
                        }
                    }
                }
            }
            // Resting on rock: not into it.
            for n in cols..cols * rows {
                if solid(d.pts[n]) {
                    let back = d.prev[n];
                    d.pts[n] = if solid(Vec2::new(d.pts[n].x, back.y)) { Vec2::new(back.x, d.pts[n].y.max(back.y)) } else { Vec2::new(d.pts[n].x, back.y) };
                    if solid(d.pts[n]) {
                        d.pts[n] = back;
                    }
                }
            }
        }
    }
}

/// Cloth's canvas: just behind the player (z 11), in front of the rest.
#[derive(Resource)]
pub struct ClothCanvas(Canvas);

impl Default for ClothCanvas {
    fn default() -> Self {
        ClothCanvas(Canvas::new("Cloth", 10.95))
    }
}

fn shade((r, g, b): (u8, u8, u8), k: f32) -> [u8; 4] {
    [(r as f32 * k).min(255.0) as u8, (g as f32 * k).min(255.0) as u8, (b as f32 * k).min(255.0) as u8, 255]
}

/// Fill the triangle `a b c` with `col`.
fn tri(px: &mut crate::canvas::Pixels, a: Vec2, b: Vec2, c: Vec2, col: [u8; 4]) {
    let lo = a.min(b).min(c).floor().as_ivec2();
    let hi = a.max(b).max(c).ceil().as_ivec2();
    let area = (b - a).perp_dot(c - a);
    if area.abs() < 1e-3 {
        return;
    }
    for y in lo.y..=hi.y {
        for x in lo.x..=hi.x {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let w0 = (b - p).perp_dot(c - p) / area;
            let w1 = (c - p).perp_dot(a - p) / area;
            let w2 = 1.0 - w0 - w1;
            if w0 >= -0.02 && w1 >= -0.02 && w2 >= -0.02 {
                px.put(x, y, col);
            }
        }
    }
}

/// Every drape, quad by quad: its outside or its lining, darker where it's
/// bunched and further down, its hem and sides edged.
fn draw(mut commands: Commands, mut canvas: ResMut<ClothCanvas>, mut images: ResMut<Assets<Image>>, camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>, q: Query<&Drapes>, mut sprites: CanvasSprites) {
    let Ok((cam, loader)) = camera.single() else { return };
    let any = q.iter().any(|d| !d.0.is_empty());
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, any) else { return };
    for drapes in &q {
        for d in &drapes.0 {
            let def = &d.def;
            let (cols, rows) = (def.cols, def.rows);
            if d.pts.len() != cols * rows {
                continue;
            }
            let p = |i: usize, j: usize| d.pts[j * cols + i];
            let kept = |i: usize, j: usize| j + 1 < rows - d.tears[i].min(rows - 2);
            for j in 0..rows - 1 {
                for i in 0..cols - 1 {
                    if !kept(i, j) {
                        continue;
                    }
                    let (a, b, c, e) = (p(i, j), p(i + 1, j), p(i + 1, j + 1), p(i, j + 1));
                    // (Bunched: narrower than it lies flat; turned: its
                    // inside showing.)
                    let wide = ((b.x - a.x).abs() + (c.x - e.x).abs()) * 0.5 / def.gap;
                    let fold = 0.55 + 0.45 * wide.clamp(0.0, 1.0);
                    let down = 1.0 - 0.18 * j as f32 / rows as f32;
                    let inside = (b - a).perp_dot(e - a) * d.facing > 0.0;
                    let col = shade(if inside { def.lining } else { def.color }, fold * down);
                    tri(&mut px, a, b, c, col);
                    tri(&mut px, a, c, e, col);
                }
            }
            // Its edges: the sides, and the hem (each column's own).
            let edge = shade(def.color, 0.45);
            let line = |px: &mut crate::canvas::Pixels, a: Vec2, b: Vec2| {
                let steps = (a.distance(b) * 1.5).ceil().max(1.0) as i32;
                for s in 0..=steps {
                    let q = a.lerp(b, s as f32 / steps as f32).floor().as_ivec2();
                    px.put(q.x, q.y, edge);
                }
            };
            for j in 0..rows - 1 {
                if kept(0, j) {
                    line(&mut px, p(0, j), p(0, j + 1));
                }
                if kept(cols - 2, j) {
                    line(&mut px, p(cols - 1, j), p(cols - 1, j + 1));
                }
            }
            for i in 0..cols - 1 {
                let j = (rows - 1 - d.tears[i].min(rows - 2)).max(1);
                line(&mut px, p(i, j), p(i + 1, j));
            }
        }
    }
}
