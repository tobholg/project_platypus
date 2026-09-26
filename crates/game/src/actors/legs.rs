//! Procedural legs (spiders): a body seen from above, turned to where it's
//! going, and legs that reach out and grip the world, Noita-style.
//!
//! A creature file's `legs` gives it: `count` legs spread round its body
//! (each with a way it prefers to reach, fanned front to back on both
//! sides), `reach` (a leg's full length: thigh `upper` of it, the rest
//! shin), how long a step takes and how high a foot lifts, colours, and
//! the `body` sprite (from above, pointing right, a `grip` anchor where
//! the legs meet), turned with RotSprite to where the body heads.
//!
//! Each foot holds on to a real solid cell: found by casting from the hip
//! along the leg's way, then swept round it (±90°) until something solid
//! is met within reach. A foot stays put while the body moves; once it's
//! stretched too far, crowded or twisted too far from its way it lifts and
//! steps to a new hold (an arc away from what it holds), a few at a time,
//! never two neighbours at once. With nothing in reach a leg hangs curled.
//! The knees (two-bone IK) bend away from what the feet hold. Legs are
//! drawn a cell at a time (Bresenham), so they stay pixel art, behind the
//! body. Looks only: the body's movement is its own (`cling` to climb).

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::sprite_render::AlphaMode2d;
use platypus_sim::{CellPos, Kind};
use serde::Deserialize;

use super::animation::{Animator, CreatureSprite};
use super::Kinematics;
use crate::world::SimWorld;

pub struct LegsPlugin;

impl Plugin for LegsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LegMesh>()
            .init_resource::<BodyArt>()
            .add_systems(Update, (grow_legs, walk, draw).chain().after(super::animation::animate).after(TransformSystems::Propagate));
    }
}

/// Legs, as a creature file writes them.
#[derive(Clone, Debug, Deserialize)]
pub struct LegsDef {
    #[serde(default = "eight")]
    pub count: usize,
    /// A leg's full length (cells); the thigh is `upper` of it.
    pub reach: f32,
    #[serde(default = "thigh")]
    pub upper: f32,
    /// Seconds a step takes; how high a foot lifts (cells).
    #[serde(default = "step_time")]
    pub step: f32,
    #[serde(default = "lift")]
    pub lift: f32,
    /// Where the legs meet, ahead of the body's middle (cells, along its
    /// heading), and how far apart their hips are along it.
    #[serde(default)]
    pub hips: f32,
    #[serde(default = "spread")]
    pub spread: f32,
    /// Colours: the leg, its joints; thighs this many cells thick (shins
    /// one less).
    pub color: (u8, u8, u8),
    #[serde(default)]
    pub joint: Option<(u8, u8, u8)>,
    #[serde(default = "one")]
    pub thick: u8,
    /// The body's sprite (seen from above, pointing right).
    pub body: String,
    /// Its pixels of this colour glow (eyes): drawn over the dark, at full
    /// brightness however dark it is.
    #[serde(default)]
    pub eyes: Option<(u8, u8, u8)>,
}

fn eight() -> usize {
    8
}
fn thigh() -> f32 {
    0.55
}
fn step_time() -> f32 {
    0.12
}
fn lift() -> f32 {
    3.0
}
fn spread() -> f32 {
    3.0
}
fn one() -> u8 {
    1
}

/// A foot: where it is, where a step goes from and to, how far along
/// (1: planted), whether it holds anything.
#[derive(Clone, Copy, Debug)]
struct Foot {
    at: Vec2,
    from: Vec2,
    to: Vec2,
    t: f32,
    grips: bool,
    /// Seconds before a leg holding nothing looks again.
    retry: f32,
}

/// A creature's legs as they are now.
#[derive(Component)]
pub struct Legs {
    def: LegsDef,
    feet: Vec<Foot>,
    /// Where the body points (radians), and the drawn body.
    heading: f32,
    body: Entity,
    eyes: Option<Entity>,
}

#[derive(Component)]
struct LegBody;

/// Turned body sprites by name (and their eyes alone, by name + colour).
#[derive(Resource, Default)]
struct BodyArt(HashMap<String, crate::combat::Turned>);

/// Over the darkness (the light overlay is at 15–15.5).
const Z_EYES: f32 = 16.2;

#[derive(Component)]
struct LegEyes;

impl Legs {
    /// Leg `i`'s preferred way, from the heading (radians): fanned front to
    /// back, alternating sides.
    fn way(&self, i: usize) -> f32 {
        let n = self.def.count.max(2);
        let per_side = n.div_ceil(2);
        let k = i / 2;
        let side = if i.is_multiple_of(2) { 1.0 } else { -1.0 };
        // From 25° off the nose to 155° (off the tail), evenly.
        let a = 25.0 + 130.0 * k as f32 / (per_side - 1).max(1) as f32;
        self.heading + side * a.to_radians()
    }

    /// Leg `i`'s hip, in the world, the body at `c`.
    fn hip(&self, i: usize, c: Vec2) -> Vec2 {
        let n = self.def.count.max(2);
        let per_side = n.div_ceil(2);
        let k = (i / 2) as f32 / (per_side - 1).max(1) as f32;
        let along = self.def.hips + self.def.spread * (0.5 - k);
        c + Vec2::from_angle(self.heading) * along
    }
}

fn solid(sim: &SimWorld, p: Vec2) -> bool {
    sim.world.get(CellPos::from_world(p.x, p.y)).is_none_or(|c| matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder))
}

/// A hold within reach of `hip`, about its way: rays swept round it
/// (±90°), each to the first solid; of those, the one nearest 70 % of its
/// reach (a leg stretched along a wall, not bunched against it), and least
/// turned from its way. The last open point before the solid one.
fn foothold(sim: &SimWorld, hip: Vec2, way: f32, reach: f32) -> Option<Vec2> {
    let mut best: Option<(f32, Vec2)> = None;
    for sweep in [0.0f32, 15.0, -15.0, 30.0, -30.0, 45.0, -45.0, 60.0, -60.0, 75.0, -75.0, 90.0, -90.0] {
        let d = Vec2::from_angle(way + sweep.to_radians());
        let mut last = hip;
        let mut s = 1.0;
        while s <= reach {
            let p = hip + d * s;
            if solid(sim, p) {
                // (Not from inside rock: a hip in a wall grips nothing.)
                if s > 1.0 {
                    let score = (s - reach * 0.7).abs() / reach + sweep.abs() / 180.0 * 0.6;
                    if best.is_none_or(|(b, _)| score < b) {
                        best = Some((score, last));
                    }
                }
                break;
            }
            last = p;
            s += 0.7;
        }
    }
    best.map(|(_, p)| p)
}

/// From `a` toward `b`, as far as it's open (a free leg never reaches into
/// rock).
fn open_toward(sim: &SimWorld, a: Vec2, b: Vec2) -> Vec2 {
    let d = b - a;
    let n = (d.length() / 0.7).ceil().max(1.0) as i32;
    let mut last = a;
    for k in 1..=n {
        let p = a + d * (k as f32 / n as f32);
        if solid(sim, p) {
            return last;
        }
        last = p;
    }
    b
}

/// Give legged creatures their legs and their turned body.
#[allow(clippy::too_many_arguments)]
fn grow_legs(
    mut commands: Commands,
    mut art: ResMut<BodyArt>,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    sim: Res<SimWorld>,
    new: Query<(Entity, &Animator, &Kinematics), Without<Legs>>,
) {
    for (e, anim, k) in &new {
        let Some(def) = anim.def.legs.clone() else { continue };
        let eyes_key = def.eyes.map(|(r, g, b)| format!("{}#{r},{g},{b}", def.body));
        for (key, only) in [(Some(def.body.clone()), None), (eyes_key.clone(), def.eyes.map(|(r, g, b)| [r, g, b]))] {
            let Some(key) = key else { continue };
            if art.0.contains_key(&key) {
                continue;
            }
            match crate::combat::turned_art(&def.body, only, &mut images, &mut layouts) {
                Ok(t) => {
                    art.0.insert(key, t);
                }
                Err(err) => warn!("legs: body `{}`: {err}", def.body),
            }
        }
        let Some(turned) = art.0.get(&def.body) else { continue };
        let body = commands.spawn((LegBody, turned.sprite(0.0), Transform::from_xyz(0.0, 0.0, 0.02))).id();
        commands.entity(e).add_child(body);
        // The eyes: over the dark (a child of the root, lifted above the light).
        let root_z = anim.def.z;
        let eyes = eyes_key.and_then(|key| art.0.get(&key)).map(|t| commands.spawn((LegEyes, t.sprite(0.0), Transform::from_xyz(0.0, 0.0, Z_EYES - root_z))).id());
        if let Some(eyes) = eyes {
            commands.entity(e).add_child(eyes);
        }
        let mut legs = Legs { feet: Vec::new(), heading: 0.0, body, eyes, def };
        let c = k.body.pos;
        for i in 0..legs.def.count {
            let hip = legs.hip(i, c);
            let at = foothold(&sim, hip, legs.way(i), legs.def.reach).unwrap_or(hip + Vec2::from_angle(legs.way(i)) * legs.def.reach * 0.5);
            legs.feet.push(Foot { at, from: at, to: at, t: 1.0, grips: true, retry: 0.0 });
        }
        commands.entity(e).insert(legs);
    }
}

/// Feet stay put, then step; the body turns to where it's going.
#[allow(clippy::type_complexity)]
fn walk(
    time: Res<Time>,
    sim: Res<SimWorld>,
    mut q: Query<(&mut Legs, &Kinematics, &GlobalTransform, &Children)>,
    mut sprites: Query<(&mut Sprite, &mut Visibility), (With<CreatureSprite>, Without<LegBody>)>,
    mut bodies: Query<&mut Sprite, (Or<(With<LegBody>, With<LegEyes>)>, Without<CreatureSprite>)>,
) {
    let dt = time.delta_secs().min(0.05);
    for (mut legs, k, tf, children) in &mut q {
        let c = tf.translation().truncate();
        // Heading: where it goes (or, still, where it aims), turning steadily.
        let v = k.body.vel;
        let want = if v.length() > 6.0 { v.y.atan2(v.x) } else { legs.heading };
        let mut d = (want - legs.heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        d = d.clamp(-8.0 * dt, 8.0 * dt);
        legs.heading += d;
        // The drawn body: the creature's own sprite hidden (its colour
        // borrowed: hurt flashes, tints), the turned one shown.
        let mut tint = None;
        for child in children.iter() {
            if let Ok((s, mut vis)) = sprites.get_mut(child) {
                tint = Some(s.color);
                *vis = Visibility::Hidden;
            }
        }
        let index = crate::combat::Turned::index(legs.heading.to_degrees());
        if let Ok(mut s) = bodies.get_mut(legs.body) {
            if let Some(atlas) = s.texture_atlas.as_mut() {
                atlas.index = index;
            }
            if let Some(t) = tint {
                s.color = t;
            }
        }
        if let Some(Ok(mut s)) = legs.eyes.map(|e| bodies.get_mut(e))
            && let Some(atlas) = s.texture_atlas.as_mut()
        {
            atlas.index = index;
        }
        // Steps.
        let (reach, n) = (legs.def.reach, legs.feet.len());
        let stepping = legs.feet.iter().filter(|f| f.t < 1.0).count();
        let step_time = legs.def.step;
        for i in 0..n {
            let hip = legs.hip(i, c);
            let way = legs.way(i);
            let f = legs.feet[i];
            if f.t < 1.0 {
                continue;
            }
            let off = f.at - hip;
            let twisted = off.length() > 0.5 && Vec2::from_angle(way).dot(off.normalize()) < 0.2;
            let stretched = off.length() > reach * 0.98;
            let crowded = off.length() < reach * 0.25;
            let dangling = !f.grips;
            let neighbours = [(i + n - 2) % n, (i + 2) % n];
            let busy = neighbours.iter().any(|&j| legs.feet[j].t < 1.0) || stepping >= n.div_ceil(3);
            let due = if dangling { legs.feet[i].retry <= 0.0 } else { stretched || crowded || twisted };
            legs.feet[i].retry -= dt;
            if !due || busy {
                continue;
            }
            let to = foothold(&sim, hip, way, reach);
            let foot = &mut legs.feet[i];
            foot.from = foot.at;
            match to {
                Some(p) => {
                    foot.to = p;
                    foot.grips = true;
                }
                None => {
                    // Nothing to hold: it reaches out, feeling the air.
                    foot.to = open_toward(&sim, hip, hip + Vec2::from_angle(way) * reach * 0.7);
                    foot.grips = false;
                    foot.retry = 0.2;
                }
            }
            foot.t = 0.0;
        }
        // Feet on their way: an arc off what they held.
        let away = {
            let held: Vec<Vec2> = legs.feet.iter().filter(|f| f.grips).map(|f| f.at).collect();
            if held.is_empty() { Vec2::Y } else { (c - held.iter().sum::<Vec2>() / held.len() as f32).normalize_or(Vec2::Y) }
        };
        let lift = legs.def.lift;
        for f in &mut legs.feet {
            if f.t < 1.0 {
                f.t = (f.t + dt / step_time.max(0.01)).min(1.0);
                let s = f.t * f.t * (3.0 - 2.0 * f.t);
                f.at = f.from.lerp(f.to, s) + away * lift * (std::f32::consts::PI * f.t).sin();
            }
        }
        // Legs holding nothing reach out from the body as it goes, and
        // twitch, each out of step.
        let now = time.elapsed_secs();
        for i in 0..n {
            if legs.feet[i].grips || legs.feet[i].t < 1.0 {
                continue;
            }
            let wiggle = (now * (2.3 + i as f32 * 0.37) + i as f32 * 1.7).sin() * 0.35;
            let hip = legs.hip(i, c);
            let to = open_toward(&sim, hip, hip + Vec2::from_angle(legs.way(i) + wiggle) * reach * 0.7);
            let f = &mut legs.feet[i];
            f.to = to;
            f.at = f.at.lerp(to, (dt * 8.0).min(1.0));
        }
    }
}

/// Where the knee is: two bones of `a` and `b` from `hip` to `foot`, the
/// knee on the `up` side, unless that's in rock and the other side isn't
/// (a knee never bends into the ground).
fn knee(hip: Vec2, foot: Vec2, a: f32, b: f32, up: Vec2, rock: impl Fn(Vec2) -> bool) -> (Vec2, Vec2) {
    let d = foot - hip;
    let len = d.length().clamp((a - b).abs() + 0.01, a + b - 0.01);
    let dir = d.normalize_or(Vec2::X);
    let foot = hip + dir * len;
    // Law of cosines: along the hip–foot line, then out.
    let x = (a * a - b * b + len * len) / (2.0 * len);
    let h = (a * a - x * x).max(0.0).sqrt();
    let side = Vec2::new(-dir.y, dir.x);
    let k1 = hip + dir * x + side * h;
    let k2 = hip + dir * x - side * h;
    let (first, second) = if (k1 - hip).dot(up) >= (k2 - hip).dot(up) { (k1, k2) } else { (k2, k1) };
    let k = if rock(first) && !rock(second) { second } else { first };
    (k, foot)
}

/// A line of cells from `a` to `b` (Bresenham), each once.
fn cells(a: Vec2, b: Vec2, out: &mut Vec<IVec2>) {
    let (mut x0, mut y0) = (a.x.floor() as i32, a.y.floor() as i32);
    let (x1, y1) = (b.x.floor() as i32, b.y.floor() as i32);
    let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
    let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
    let mut err = dx + dy;
    loop {
        out.push(IVec2::new(x0, y0));
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

#[derive(Resource, Default)]
struct LegMesh {
    mesh: Option<Handle<Mesh>>,
}

/// Every leg, a cell at a time, into one mesh (under the bodies).
fn draw(
    mut commands: Commands,
    sim: Res<SimWorld>,
    mut set: ResMut<LegMesh>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    q: Query<(&Legs, &GlobalTransform)>,
) {
    let mut quads: Vec<(IVec2, [f32; 4])> = Vec::new();
    let mut line = Vec::new();
    for (legs, tf) in &q {
        let c = tf.translation().truncate();
        let rgb = |(r, g, b): (u8, u8, u8)| Color::srgb_u8(r, g, b).to_linear().to_f32_array();
        let (leg, joint) = (rgb(legs.def.color), rgb(legs.def.joint.unwrap_or(legs.def.color)));
        let (a, b) = (legs.def.reach * legs.def.upper, legs.def.reach * (1.0 - legs.def.upper));
        let held: Vec<Vec2> = legs.feet.iter().filter(|f| f.grips).map(|f| f.at).collect();
        let up = if held.is_empty() { Vec2::Y } else { (c - held.iter().sum::<Vec2>() / held.len() as f32).normalize_or(Vec2::Y) };
        for (i, f) in legs.feet.iter().enumerate() {
            let hip = legs.hip(i, c);
            // (Knees also splay out from the body a little.)
            let out = Vec2::from_angle(legs.way(i));
            let (k, foot) = knee(hip, f.at, a, b, (up + out * 0.6).normalize_or(up), |p| solid(&sim, p));
            // Thick lines: the cells beside the line, across it.
            let mut thick = |a: Vec2, b: Vec2, t: u8, quads: &mut Vec<(IVec2, [f32; 4])>| {
                line.clear();
                cells(a, b, &mut line);
                let d = b - a;
                let across = if d.x.abs() > d.y.abs() { IVec2::new(0, 1) } else { IVec2::new(1, 0) };
                for &p in &line {
                    for w in 0..t.max(1) as i32 {
                        quads.push((p + across * (w - (t as i32 - 1) / 2), leg));
                    }
                }
            };
            thick(hip, k, legs.def.thick, &mut quads);
            thick(k, foot, legs.def.thick.saturating_sub(1), &mut quads);
            quads.push((IVec2::new(k.x.floor() as i32, k.y.floor() as i32), joint));
        }
    }
    let handle = match &set.mesh {
        Some(h) => h.clone(),
        None => {
            let material = materials.add(ColorMaterial { alpha_mode: AlphaMode2d::Blend, ..default() });
            let h = meshes.add(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD));
            commands.spawn((Name::new("Legs"), Mesh2d(h.clone()), MeshMaterial2d(material), Transform::from_xyz(0.0, 0.0, 9.5), NoFrustumCulling));
            set.mesh = Some(h.clone());
            h
        }
    };
    let Some(mut mesh) = meshes.get_mut(&handle) else { return };
    let mut pos = Vec::with_capacity(quads.len() * 4 + 4);
    let mut col = Vec::with_capacity(quads.len() * 4 + 4);
    let mut idx = Vec::with_capacity(quads.len() * 6 + 6);
    for (i, (p, c)) in quads.iter().enumerate() {
        let (x, y) = (p.x as f32, p.y as f32);
        let base = (i * 4) as u32;
        pos.extend([[x, y, 0.0], [x + 1.0, y, 0.0], [x + 1.0, y + 1.0, 0.0], [x, y + 1.0, 0.0]]);
        col.extend([*c; 4]);
        idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    if pos.is_empty() {
        pos.extend([[0.0, 0.0, 0.0]; 4]);
        col.extend([[0.0; 4]; 4]);
        idx.extend([0, 1, 2, 0, 2, 3]);
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
    mesh.insert_indices(Indices::U32(idx));
}
