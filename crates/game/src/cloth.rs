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
            .init_resource::<ClothFront>()
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
    /// Where it may hang from where it's pinned (degrees: 0 straight down,
    /// + behind its wearer, 90 straight out behind, 180 straight up): from
    /// `arc.0` (a little in front) to `arc.1` (up over its head, falling).
    /// Never round its front, however it's flung: it can't wind round.
    pub arc: (f32, f32),
}

impl Default for ClothDef {
    fn default() -> Self {
        ClothDef { color: (150, 38, 36), lining: (96, 26, 28), cols: 5, rows: 9, gap: 2.5, pin: (-4.0, 1.0, 5.0), torn: 0.0, weight: 1.0, front: false, behind: 0.0, arc: (-15.0, 165.0) }
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
/// At the edge of its arc, each column (from the back) this much less far
/// up than the one behind it (degrees).
const FAN: f32 = 10.0;
/// Flapping as its wearer rushes along: at this speed (cells/s) as hard
/// as this (cells/s²), more going faster.
const FLUTTER: (f32, f32) = (300.0, 2600.0);
/// How hard cloth falls (× its weight), how much of its swing it keeps a
/// tick, the breeze's push (cells/s²); ties pulled true this many times a
/// tick; pinned further than this from where it was (a teleport): hung
/// again from scratch.
const FALL: f32 = 700.0;
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
        // (Rushing through the air, it flaps: as hard as it goes.)
        let rush = (k.body.vel.length() / FLUTTER.0).min(1.5);
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
                    let up = (j as f32 / rows as f32) * rush * FLUTTER.1;
                    let flap = Vec2::new((t * 19.0 + j as f32 * 1.3 + i as f32 * 0.7).sin(), (t * 23.0 + j as f32 * 0.9).cos() * 0.5) * up;
                    d.pts[n] += v + (Vec2::new(breeze + away, -FALL * def.weight) + flap) * DT * DT;
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
            // Within its arc behind its wearer: a point swung past it (flung
            // over its head, round its front) is put back on its edge, its
            // swing that way gone.
            // (Fanned: the rearmost column may go furthest up, the rest a
            // little less each: at its edge it's still cloth, not a line.)
            let lo = def.arc.0.to_radians();
            for j in 1..rows {
                for i in 0..cols {
                    let hi = (def.arc.1 - (i as f32) * FAN).to_radians();
                    let n = idx(i, j);
                    let o = d.pts[idx(i, 0)];
                    let v = d.pts[n] - o;
                    let len = v.length();
                    if len < 1e-3 {
                        continue;
                    }
                    // (Its angle: from straight down toward behind.)
                    let a = (-v.x * facing).atan2(-v.y);
                    if a < lo || a > hi {
                        let a = a.clamp(lo, hi);
                        let p = o + Vec2::new(-a.sin() * facing, -a.cos()) * len;
                        d.pts[n] = p;
                        d.prev[n] = p;
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

/// Cloth drawn over its wearer (`front`): over the tallest creatures.
#[derive(Resource)]
pub struct ClothFront(Canvas);

impl Default for ClothFront {
    fn default() -> Self {
        ClothFront(Canvas::new("Cloth over", 11.25))
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
/// bunched and further down, its hem and sides edged; behind its wearer
/// (a cape) or in front (rags over a body).
#[allow(clippy::too_many_arguments)]
fn draw(
    mut commands: Commands,
    mut canvas: ResMut<ClothCanvas>,
    mut front: ResMut<ClothFront>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>,
    q: Query<&Drapes>,
    mut sprites: CanvasSprites,
) {
    let Ok((cam, loader)) = camera.single() else { return };
    let (centre, half) = (cam.translation().truncate(), loader.half_extent);
    for (canvas, in_front) in [(&mut canvas.0, false), (&mut front.0, true)] {
        let any = q.iter().any(|d| d.0.iter().any(|x| x.def.front == in_front));
        let Some(mut px) = canvas.frame(&mut commands, &mut images, &mut sprites, centre, half, any) else { continue };
        for drapes in &q {
            for d in drapes.0.iter().filter(|x| x.def.front == in_front) {
                draw_drape(&mut px, d);
            }
        }
    }
}

fn draw_drape(px: &mut crate::canvas::Pixels, d: &Drape) {
    let def = &d.def;
    let (cols, rows) = (def.cols, def.rows);
    if d.pts.len() != cols * rows {
        return;
    }
    let p = |i: usize, j: usize| d.pts[j * cols + i];
    let kept = |i: usize, j: usize| j + 1 < rows - d.tears[i].min(rows - 2);
    for j in 0..rows - 1 {
        for i in 0..cols - 1 {
            if !kept(i, j) {
                continue;
            }
            let (a, b, c, e) = (p(i, j), p(i + 1, j), p(i + 1, j + 1), p(i, j + 1));
            // (Bunched: narrower than it lies flat; turned: its inside
            // showing.)
            let wide = ((b.x - a.x).abs() + (c.x - e.x).abs()) * 0.5 / def.gap;
            let fold = 0.55 + 0.45 * wide.clamp(0.0, 1.0);
            let down = 1.0 - 0.18 * j as f32 / rows as f32;
            let inside = (b - a).perp_dot(e - a) * d.facing > 0.0;
            let col = shade(if inside { def.lining } else { def.color }, fold * down);
            tri(px, a, b, c, col);
            tri(px, a, c, e, col);
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
            line(px, p(0, j), p(0, j + 1));
        }
        if kept(cols - 2, j) {
            line(px, p(cols - 1, j), p(cols - 1, j + 1));
        }
    }
    for i in 0..cols - 1 {
        let j = (rows - 1 - d.tears[i].min(rows - 2)).max(1);
        line(px, p(i, j), p(i + 1, j));
    }
}

/// A hanging chain of links (`n`, `gap` apart), held at its top: it falls,
/// swings and trails what holds it. (Not cloth, but it hangs as cloth
/// does; a creature's code draws it with `draw_links`.)
#[derive(Clone, Debug, Default)]
pub struct Links {
    pts: Vec<Vec2>,
    prev: Vec<Vec2>,
    n: usize,
    gap: f32,
}

impl Links {
    pub fn new(n: usize, gap: f32) -> Links {
        Links { pts: Vec::new(), prev: Vec::new(), n, gap }
    }

    pub fn points(&self) -> &[Vec2] {
        &self.pts
    }

    /// A tick: held at `top`, falling `fall` cells/s².
    pub fn swing(&mut self, top: Vec2, fall: f32) {
        if self.pts.len() != self.n || self.pts[0].distance(top) > JUMP {
            self.pts = (0..self.n).map(|i| top - Vec2::Y * self.gap * i as f32).collect();
            self.prev = self.pts.clone();
        }
        for i in 1..self.n {
            let v = (self.pts[i] - self.prev[i]) * 0.97;
            self.prev[i] = self.pts[i];
            self.pts[i] += v - Vec2::Y * fall * DT * DT;
        }
        for _ in 0..6 {
            self.pts[0] = top;
            for i in 1..self.n {
                let d = self.pts[i] - self.pts[i - 1];
                let len = d.length().max(1e-4);
                let fix = d * (1.0 - self.gap / len);
                if i == 1 {
                    self.pts[i] -= fix;
                } else {
                    self.pts[i - 1] += fix * 0.5;
                    self.pts[i] -= fix * 0.5;
                }
            }
        }
        self.pts[0] = top;
    }
}

/// Links drawn through `pts`: light and dark by turns, two cells thick.
pub fn draw_links(px: &mut crate::canvas::Pixels, pts: &[Vec2], light: [u8; 4], dark: [u8; 4]) {
    let mut n = 0u32;
    let mut last = IVec2::MAX;
    for pair in pts.windows(2) {
        let steps = (pair[0].distance(pair[1]) * 1.5).ceil().max(1.0) as i32;
        for s in 0..=steps {
            let p = pair[0].lerp(pair[1], s as f32 / steps as f32).floor().as_ivec2();
            if p == last {
                continue;
            }
            last = p;
            n += 1;
            px.put(p.x, p.y, if (n / 2).is_multiple_of(2) { light } else { dark });
            px.put(p.x, p.y - 1, dark);
        }
    }
}
