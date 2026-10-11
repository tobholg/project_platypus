//! Ragdolls (`wounds` stage 1): a body as its parts, pinned at their joints,
//! each part colliding with the cells. A humanoid's head, torso, arms and
//! legs are cut from its own art as it fell (each drawn alone from its
//! pose, dressed in what it wore), each a rigid piece: its pixels, its
//! weight (how many), how it turns (where they lie about its middle), and
//! the points on its edge that meet the world. Each joint is where a part
//! touches the torso in that pose (the neck, the shoulders, the hips), and
//! bends only so far (`limit`): a head nods, an arm swings round, legs
//! fold.
//!
//! Stepped with position-based dynamics: each piece moves on as it was
//! moving (Verlet), falls, and is then put right a few times over: joints
//! pulled together, bends held to their limits, edge points pushed out of
//! the cells they've gone into (and slowed where they touch: friction).
//! Still a while, a ragdoll sleeps until something throws it (a blast).
//! The body it belongs to (a `Corpse`, for its contents) goes where its
//! torso is.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use platypus_physics::Grid;

use crate::creatures::{Kinematics, WorldGrid};
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;
const SUBSTEPS: usize = 3;
const ITERATIONS: usize = 4;
/// Velocity kept a substep in the air (a little air drag), and in contact;
/// the fastest a piece goes (cells/s: a body thrown hard flies, it doesn't
/// cross the map).
const AIR: f32 = 0.997;
const FASTEST: f32 = 220.0;
const GRIP: f32 = 0.55;
/// Turning kept a substep (joints wobble, then settle).
const SPIN_KEPT: f32 = 0.985;
/// Most edge points a piece tests against the world.
const SAMPLES_MOST: usize = 28;
/// Still: moving less than this (cells a tick) this many ticks, it sleeps.
const STILL: f32 = 0.015;
const STILL_TICKS: u32 = 40;
/// How far a joint may bend from how it was as the body fell (radians),
/// by part.
const BEND_HEAD: f32 = 0.9;
const BEND_ARM: f32 = 2.6;
const BEND_LEGS: f32 = 1.3;
const BEND_OTHER: f32 = 1.6;
/// What a ragdoll takes of a push on its body (a blast: `corpses::batter`),
/// and the most (cells/s).
const THROWN_SHARE: f32 = 0.35;
const THROWN_MOST: f32 = 260.0;
/// Dead, it goes limp: the torso tips back (cells a second, away from where
/// it faced), each limb a twist of its own (radians a second, either way),
/// or a body balanced on its legs would stand.
const SLUMP_BACK: f32 = 22.0;
const SLUMP_TWIST: f32 = 3.5;

/// One rigid piece of a ragdoll.
#[derive(Clone, Debug)]
pub struct Piece {
    /// Its middle (its weight's), now and a substep ago; its turn the same.
    pub pos: Vec2,
    prev: Vec2,
    pub angle: f32,
    prev_angle: f32,
    /// As it was at the start of this tick (drawn between ticks).
    tick_pos: Vec2,
    tick_angle: f32,
    inv_mass: f32,
    inv_inertia: f32,
    /// Its edge points, from its middle (unturned).
    samples: Vec<Vec2>,
    /// Where its picture's middle is from its weight's (unturned).
    pub draw_offset: Vec2,
    pub name: String,
    /// Its pixels, from its middle (unturned), each its colour: kept for
    /// rubble (bones that fly back together: `rise`); else none.
    pub pixels: Vec<(Vec2, [u8; 4])>,
}

/// Two pieces pinned together at a point (each's own, from its middle).
#[derive(Clone, Debug)]
pub struct Joint {
    a: usize,
    b: usize,
    la: Vec2,
    lb: Vec2,
    /// The bend between them as the body fell, and how far from it they go.
    rest: f32,
    limit: f32,
}

/// A body as pieces and joints.
#[derive(Component, Clone, Debug)]
pub struct Limp {
    pub pieces: Vec<Piece>,
    joints: Vec<Joint>,
    /// The piece the body goes with (its torso).
    pub root: usize,
    still: u32,
    pub asleep: bool,
}

/// A piece's picture (a child of the body).
#[derive(Component)]
pub struct LimpPiece(pub usize);

/// A piece cut out of the art: its picture, and where it was.
pub struct Cut {
    pub image: Image,
    pub name: String,
    /// Its layer's tag (`front_arm`, `back_arm`), if it has one.
    pub tag: Option<String>,
    /// Its pixels' middle, and its picture's middle, in the frame (pixels, y down).
    com: Vec2,
    centre: Vec2,
    pixels: Vec<IVec2>,
    colors: Vec<[u8; 4]>,
    edge: Vec<IVec2>,
}

impl Cut {
    /// Does it cover this pixel of its frame?
    pub fn covers(&self, p: IVec2) -> bool {
        self.pixels.contains(&p)
    }

    /// The limb it is, if it's one that can be cut off: a humanoid's front
    /// arm (`Arm(0)`, its weapon hand), back arm (`Arm(1)`), head.
    pub fn limb(&self) -> Option<super::parts::Limb> {
        use super::parts::Limb;
        match self.tag.as_deref() {
            Some("front_arm") => Some(Limb::Arm(0)),
            Some("back_arm") => Some(Limb::Arm(1)),
            _ if self.name.contains("head") || self.name == "blink" => Some(Limb::Head),
            _ => None,
        }
    }

    /// Its pixels from its middle (unturned, the world's way up, mirrored as
    /// it faces), each its colour: a piece's `pixels`.
    pub fn pixels_local(&self, facing: f32) -> Vec<(Vec2, [u8; 4])> {
        self.pixels.iter().zip(&self.colors).map(|(q, c)| {
            let d = q.as_vec2() + Vec2::splat(0.5) - self.com;
            (Vec2::new(d.x * facing, -d.y), *c)
        }).collect()
    }

    /// Where it meets what it was cut from (frame pixels): its pixels'
    /// middle, toward the frame's middle.
    pub fn joint(&self, frame: (u32, u32)) -> Vec2 {
        let mid = Vec2::new(frame.0 as f32 / 2.0, frame.1 as f32 / 2.0);
        let near = self.pixels.iter().min_by(|a, b| (a.as_vec2() - mid).length_squared().total_cmp(&(b.as_vec2() - mid).length_squared())).copied().unwrap_or(IVec2::ZERO);
        near.as_vec2() + Vec2::splat(0.5)
    }
}

/// The pose a shown frame was drawn from (a fan's bare pose is its pose; a
/// derived frame is its source, shifted).
fn pose_of<'a>(file: &'a platypus_art::ArtFile, frame: &str) -> Option<(&'a Vec<platypus_art::Layer>, (i32, i32))> {
    let base = frame.split('~').next().unwrap_or(frame);
    if let Some(p) = file.poses.get(base) {
        return Some((p, (0, 0)));
    }
    let d = file.derived.get(base)?;
    file.poses.get(&d.from).map(|p| (p, d.shift))
}

/// The pieces of a body drawn as `frame` of `file` (dressed): each layer
/// of its pose drawn alone (with what's over it), cut to its pixels.
pub fn cut(file: &platypus_art::ArtFile, frame: &str) -> Vec<Cut> {
    let Some((layers, shift)) = pose_of(file, frame).or_else(|| pose_of(file, "stand")) else { return Vec::new() };
    let mut out = Vec::new();
    for (i, layer) in layers.iter().enumerate() {
        let mut one = platypus_art::ArtFile {
            size: file.size,
            feet: file.feet,
            outline: file.outline,
            palette: file.palette.clone(),
            frames: Default::default(),
            derived: Default::default(),
            clips: Default::default(),
            anchors: Default::default(),
            parts: file.parts.clone(),
            poses: Default::default(),
            fans: Default::default(),
            over: file.over.clone(),
        };
        let mut l = layer.clone();
        l.at = (l.at.0 + shift.0, l.at.1 + shift.1);
        // (Alone it lies over nothing: no edging over what's under it.)
        l.outline = false;
        let name = format!("piece{i}");
        one.poses.insert(name.clone(), vec![l]);
        let Ok(art) = platypus_art::compile(&one) else { continue };
        let Some(p) = art.index(&name).and_then(|i| art.frames.get(i)) else { continue };
        let mut pixels = Vec::new();
        for y in 0..p.h as i32 {
            for x in 0..p.w as i32 {
                if p.opaque(x, y) {
                    pixels.push(IVec2::new(x, y));
                }
            }
        }
        if pixels.is_empty() {
            continue;
        }
        let lo = pixels.iter().fold(IVec2::MAX, |m, q| m.min(*q));
        let hi = pixels.iter().fold(IVec2::MIN, |m, q| m.max(*q));
        let (w, h) = ((hi.x - lo.x + 1) as u32, (hi.y - lo.y + 1) as u32);
        let mut data = vec![0u8; (w * h * 4) as usize];
        for q in &pixels {
            let c = p.get(q.x, q.y);
            let i = (((q.y - lo.y) as u32 * w + (q.x - lo.x) as u32) * 4) as usize;
            data[i..i + 4].copy_from_slice(&c);
        }
        let mut image = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
        image.sampler = bevy::image::ImageSampler::nearest();
        let set: std::collections::HashSet<IVec2> = pixels.iter().copied().collect();
        let edge: Vec<IVec2> = pixels.iter().copied().filter(|q| [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y].iter().any(|d| !set.contains(&(*q + *d)))).collect();
        let com = pixels.iter().fold(Vec2::ZERO, |s, q| s + q.as_vec2()) / pixels.len() as f32 + Vec2::splat(0.5);
        let centre = Vec2::new(lo.x as f32 + w as f32 / 2.0, lo.y as f32 + h as f32 / 2.0);
        let colors = pixels.iter().map(|q| p.get(q.x, q.y)).collect();
        out.push(Cut { image, name: layer.part.clone(), tag: layer.tag.clone(), com, centre, pixels, colors, edge });
    }
    out
}

/// How far a part may bend at its joint.
fn bend(name: &str) -> f32 {
    match name {
        n if n.contains("head") || n.contains("blink") => BEND_HEAD,
        n if n.contains("arm") || n.starts_with("aim") => BEND_ARM,
        n if n.contains("leg") || n.starts_with("run") || n == "tuck" || n == "dangle" => BEND_LEGS,
        _ => BEND_OTHER,
    }
}

/// A ragdoll of cut pieces, laid where the body was: `centre` is the
/// frame's middle in the world, `facing` its way; moving as it was
/// (`vel`, cells a second, turning `omega` a second about the torso).
pub fn assemble(cuts: &[Cut], frame: (u32, u32), centre: Vec2, facing: f32, vel: Vec2, omega: f32) -> Limp {
    let (fw, fh) = (frame.0 as f32, frame.1 as f32);
    // Frame pixels (y down) to the world, and offsets the same.
    let world = |p: Vec2| centre + Vec2::new((p.x - fw / 2.0) * facing, fh / 2.0 - p.y);
    let local = |d: Vec2| Vec2::new(d.x * facing, -d.y);
    let root = cuts.iter().position(|c| c.name.contains("torso")).unwrap_or_else(|| cuts.iter().enumerate().max_by_key(|(_, c)| c.pixels.len()).map_or(0, |(i, _)| i));
    let root_pos = cuts.get(root).map_or(centre, |c| world(c.com));
    let mut rng = platypus_sim::rng::Rng::seeded(&[centre.x.to_bits() as u64, centre.y.to_bits() as u64, 0x5106]);
    let mut pieces = Vec::new();
    for (i, c) in cuts.iter().enumerate() {
        let mass = c.pixels.len() as f32;
        let inertia = c.pixels.iter().map(|q| (q.as_vec2() + Vec2::splat(0.5) - c.com).length_squared()).sum::<f32>().max(1.0);
        let step = (c.edge.len() / SAMPLES_MOST).max(1);
        let samples = c.edge.iter().step_by(step).map(|q| local(q.as_vec2() + Vec2::splat(0.5) - c.com)).collect();
        let pos = world(c.com);
        // (Moving as the body was, and turning with it about its torso.)
        let r = pos - root_pos;
        // (Going limp: the torso back, the limbs each their own twist.)
        let unit = rng.next_u32() as f32 / u32::MAX as f32 * 2.0 - 1.0;
        let (slump, twist) = if i == root { (Vec2::new(-facing * SLUMP_BACK, 0.0), -facing * SLUMP_TWIST * 0.4) } else { (Vec2::ZERO, unit * SLUMP_TWIST) };
        let v = (vel + slump + Vec2::new(-r.y, r.x) * omega) * DT / SUBSTEPS as f32;
        let w = (omega + twist) * DT / SUBSTEPS as f32;
        // (Its pixels kept: a body that gets up flies back together, `rise`.)
        let pixels = c.pixels_local(facing);
        pieces.push(Piece { pos, prev: pos - v, angle: 0.0, prev_angle: -w, tick_pos: pos, tick_angle: 0.0, inv_mass: 1.0 / mass, inv_inertia: 1.0 / inertia, samples, draw_offset: local(c.centre - c.com), name: c.name.clone(), pixels });
    }
    // Each piece pinned to the torso where it touches it.
    let mut joints = Vec::new();
    if let Some(rc) = cuts.get(root) {
        let torso: std::collections::HashSet<IVec2> = rc.pixels.iter().copied().collect();
        for (i, c) in cuts.iter().enumerate() {
            if i == root {
                continue;
            }
            let touching: Vec<IVec2> = c.pixels.iter().copied().filter(|q| (-1..=1).any(|dy| (-1..=1).any(|dx| torso.contains(&(*q + IVec2::new(dx, dy)))))).collect();
            let at = if touching.is_empty() {
                // (Not touching: the nearest two pixels' middle.)
                let (mut best, mut d) = (c.com, f32::MAX);
                for q in &c.pixels {
                    for t in &rc.pixels {
                        let dd = q.distance_squared(*t) as f32;
                        if dd < d {
                            d = dd;
                            best = (q.as_vec2() + t.as_vec2()) / 2.0 + Vec2::splat(0.5);
                        }
                    }
                }
                best
            } else {
                touching.iter().fold(Vec2::ZERO, |s, q| s + q.as_vec2()) / touching.len() as f32 + Vec2::splat(0.5)
            };
            let jw = world(at);
            joints.push(Joint { a: root, b: i, la: jw - pieces[root].pos, lb: jw - pieces[i].pos, rest: 0.0, limit: bend(&c.name) });
        }
    }
    Limp { pieces, joints, root, still: 0, asleep: false }
}

/// How a piece looks: drawn for it (mirrored or not), or the body's own
/// sprite.
pub enum Look {
    Image(Image, bool),
    Sprite(Sprite),
}

/// How far a walker's limb bends at a joint with another limb (a knee, an
/// elbow, a tail's link), and where it meets the body (a hip, a shoulder).
const BEND_KNEE: f32 = 2.2;
const BEND_HIP: f32 = 1.8;
/// A walker's body piece: this share of its frame (sprites have margins).
const BODY_SHARE: f32 = 0.34;

/// A part's pixels by the chunk of rubble they fall in.
type Chunks = std::collections::BTreeMap<(i32, i32), Vec<(IVec2, [u8; 4])>>;

/// Rubble: chunks this many pixels across.
const CHUNK: i32 = 4;
/// Fewest pixels a chunk keeps (smaller crumbs join the one beside).
const CHUNK_LEAST: usize = 3;
/// How the chunks scatter as it falls apart (cells a second, radians a second).
const SCATTER: f32 = 34.0;
const SCATTER_SPIN: f32 = 9.0;

/// A body that falls apart (bones: what rises, and what's broken for good)
/// as rubble: each part cut into small chunks, loose (no joints), flung a
/// little apart and tumbling, to lie in a heap. Each chunk keeps its pixels
/// (for flying back together: `rise`).
pub fn rubble(cuts: &[Cut], frame: (u32, u32), centre: Vec2, facing: f32, vel: Vec2) -> (Limp, Vec<Look>) {
    let (fw, fh) = (frame.0 as f32, frame.1 as f32);
    let world = |p: Vec2| centre + Vec2::new((p.x - fw / 2.0) * facing, fh / 2.0 - p.y);
    let local = |d: Vec2| Vec2::new(d.x * facing, -d.y);
    let sub = DT / SUBSTEPS as f32;
    let mut rng = platypus_sim::rng::Rng::seeded(&[centre.x.to_bits() as u64, centre.y.to_bits() as u64, 0x8B0E]);
    let mut unit = move || rng.next_u32() as f32 / u32::MAX as f32 * 2.0 - 1.0;
    let mut pieces = Vec::new();
    let mut looks = Vec::new();
    for (ci, c) in cuts.iter().enumerate() {
        // (Each part's grid a little offset, so the breaks don't line up.)
        let off = IVec2::new(ci as i32 * 3 % CHUNK, ci as i32 * 5 % CHUNK);
        let mut chunks: Chunks = Default::default();
        for (q, col) in c.pixels.iter().zip(&c.colors) {
            let k = ((q.x + off.x).div_euclid(CHUNK), (q.y + off.y).div_euclid(CHUNK));
            chunks.entry(k).or_default().push((*q, *col));
        }
        // (Crumbs join a neighbour.)
        let keys: Vec<(i32, i32)> = chunks.keys().copied().collect();
        for k in keys {
            if chunks.get(&k).is_some_and(|v| v.len() < CHUNK_LEAST)
                && let Some(n) = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().map(|d| (k.0 + d.0, k.1 + d.1)).find(|n| chunks.get(n).is_some_and(|v| v.len() >= CHUNK_LEAST))
            {
                let crumbs = chunks.remove(&k).unwrap_or_default();
                chunks.entry(n).or_default().extend(crumbs);
            }
        }
        for px in chunks.into_values() {
            let com = px.iter().fold(Vec2::ZERO, |s, (q, _)| s + q.as_vec2()) / px.len() as f32 + Vec2::splat(0.5);
            let drawn: Vec<(IVec2, [u8; 4])> = px.iter().map(|(q, col)| (IVec2::new(q.x, -q.y), *col)).collect();
            let (image, middle) = picture(&drawn);
            // (`picture` draws y up: its middle back in frame pixels, y down;
            // a pixel's row `q` was drawn at `-q`, spanning up to `-q + 1`.)
            let middle = Vec2::new(middle.x, 1.0 - middle.y);
            let mass = px.len() as f32;
            let inertia = px.iter().map(|(q, _)| (q.as_vec2() + Vec2::splat(0.5) - com).length_squared()).sum::<f32>().max(1.0);
            let set: std::collections::HashSet<IVec2> = px.iter().map(|(q, _)| *q).collect();
            let samples = px.iter().filter(|(q, _)| [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y].iter().any(|d| !set.contains(&(*q + *d)))).map(|(q, _)| local(q.as_vec2() + Vec2::splat(0.5) - com)).collect();
            let pos = world(com);
            let fling = vel * 0.4 + Vec2::new(unit() * SCATTER, unit().abs() * SCATTER * 0.6);
            let spin = unit() * SCATTER_SPIN;
            let pixels = px.iter().map(|(q, col)| (local(q.as_vec2() + Vec2::splat(0.5) - com), *col)).collect();
            pieces.push(Piece { pos, prev: pos - fling * sub, angle: 0.0, prev_angle: -spin * sub, tick_pos: pos, tick_angle: 0.0, inv_mass: 1.0 / mass, inv_inertia: 1.0 / inertia, samples, draw_offset: local(middle - com), name: "bone".into(), pixels });
            looks.push(Look::Image(image, facing < 0.0));
        }
    }
    let root = pieces.iter().enumerate().max_by(|a, b| a.1.inv_mass.total_cmp(&b.1.inv_mass).reverse()).map_or(0, |(i, _)| i);
    (Limp { pieces, joints: Vec::new(), root, still: 0, asleep: false }, looks)
}

/// Limbs drawn apart from its art (a rig's legs: `legs::rig`) added to
/// rubble, each a loose bone (no joint), flung with the rest, keeping its
/// pixels.
pub fn loose_limbs(limp: &mut Limp, looks: &mut Vec<Look>, limbs: &[super::parts::Part], draw: impl Fn(&super::parts::Part, f32) -> Vec<(IVec2, [u8; 4])>, vel: Vec2) {
    let sub = DT / SUBSTEPS as f32;
    let mut rng = platypus_sim::rng::Rng::seeded(&[limbs.len() as u64, vel.x.to_bits() as u64, 0x1005]);
    let mut unit = move || rng.next_u32() as f32 / u32::MAX as f32 * 2.0 - 1.0;
    for p in limbs {
        let d = p.b - p.a;
        let len = d.length().max(1.0);
        let thick = (p.r * 2.0 - 1.0).max(1.0);
        let drawn = draw(p, len);
        if drawn.is_empty() {
            continue;
        }
        let (image, middle) = picture(&drawn);
        let mut samples = Vec::new();
        let mut x = -len / 2.0;
        while x <= len / 2.0 {
            samples.extend([Vec2::new(x, thick / 2.0), Vec2::new(x, -thick / 2.0)]);
            x += 2.0;
        }
        let mass = len * thick;
        let pos = (p.a + p.b) / 2.0;
        let angle = d.y.atan2(d.x);
        let fling = vel * 0.4 + Vec2::new(unit() * SCATTER, unit().abs() * SCATTER * 0.6);
        let spin = unit() * SCATTER_SPIN;
        let pixels = drawn.iter().map(|(q, c)| (q.as_vec2() + Vec2::splat(0.5), *c)).collect();
        limp.pieces.push(Piece { pos, prev: pos - fling * sub, angle, prev_angle: angle - spin * sub, tick_pos: pos, tick_angle: angle, inv_mass: 1.0 / mass, inv_inertia: 12.0 / (mass * (len * len + thick * thick)).max(1.0), samples, draw_offset: middle, name: "bone".into(), pixels });
        looks.push(Look::Image(image, false));
    }
}

impl Limp {
    /// Every pixel of its pieces where it is now (rubble's: what flies back
    /// together), each its colour.
    pub fn world_pixels(&self) -> Vec<(Vec2, [u8; 4])> {
        self.pieces.iter().flat_map(|p| p.pixels.iter().map(move |(l, c)| (p.pos + rot(*l, p.angle), *c))).collect()
    }
}

/// Cells drawn (each a colour, later ones over earlier) as a picture, and
/// where its middle is among them.
fn picture(px: &[(IVec2, [u8; 4])]) -> (Image, Vec2) {
    let lo = px.iter().fold(IVec2::MAX, |m, (p, _)| m.min(*p));
    let hi = px.iter().fold(IVec2::MIN, |m, (p, _)| m.max(*p));
    let (w, h) = ((hi.x - lo.x + 1) as u32, (hi.y - lo.y + 1) as u32);
    let mut data = vec![0u8; (w * h * 4) as usize];
    for (p, c) in px {
        let i = (((hi.y - p.y) as u32 * w + (p.x - lo.x) as u32) * 4) as usize;
        data[i..i + 4].copy_from_slice(c);
    }
    let mut image = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    image.sampler = bevy::image::ImageSampler::nearest();
    (image, (lo + hi + IVec2::ONE).as_vec2() / 2.0)
}

/// A walker (a body on procedural limbs) as a ragdoll: its body sprite one
/// piece (`frame` its size, at the body's middle plus `offset`), each limb
/// capsule (`limbs`: thigh, shin, arm bone, segment, tail link) another,
/// drawn by `draw` as it's drawn alive, laid along x from its middle; each
/// limb pinned to the one before it it meets (a shin to its thigh, a link
/// to the last), or else to the body where it's nearest.
#[allow(clippy::too_many_arguments)]
pub fn limbed(sprite: Sprite, offset: Vec2, frame: (u32, u32), body: Vec2, limbs: &[super::parts::Part], draw: impl Fn(&super::parts::Part, f32) -> Vec<(IVec2, [u8; 4])>, vel: Vec2, omega: f32) -> (Limp, Vec<Look>) {
    let sub = DT / SUBSTEPS as f32;
    let centre = body + offset;
    let half = Vec2::new(frame.0 as f32, frame.1 as f32) * BODY_SHARE;
    let mut rng = platypus_sim::rng::Rng::seeded(&[centre.x.to_bits() as u64, centre.y.to_bits() as u64, 0x11B5]);
    let edge = |half: Vec2| {
        let mut s = Vec::new();
        let n = 5;
        for i in 0..=n {
            let t = i as f32 / n as f32 * 2.0 - 1.0;
            s.extend([Vec2::new(t * half.x, -half.y), Vec2::new(t * half.x, half.y), Vec2::new(-half.x, t * half.y), Vec2::new(half.x, t * half.y)]);
        }
        s
    };
    let new = |pos: Vec2, angle: f32, mass: f32, inertia: f32, samples: Vec<Vec2>, name: &str, v: Vec2, w: f32| Piece {
        pos,
        prev: pos - v * sub,
        angle,
        prev_angle: angle - w * sub,
        tick_pos: pos,
        tick_angle: angle,
        inv_mass: 1.0 / mass.max(1.0),
        inv_inertia: 1.0 / inertia.max(1.0),
        samples,
        draw_offset: Vec2::ZERO,
        name: name.to_string(),
        pixels: Vec::new(),
    };
    let body_mass = (half.x * half.y * 4.0).max(4.0);
    let mut pieces = vec![new(centre, 0.0, body_mass, body_mass * half.length_squared() / 3.0, edge(half), "body", vel, omega)];
    let mut looks = vec![Look::Sprite(sprite)];
    let mut angles = vec![0.0f32];
    for p in limbs {
        let d = p.b - p.a;
        let len = d.length().max(1.0);
        let thick = (p.r * 2.0 - 1.0).max(1.0);
        let angle = d.y.atan2(d.x);
        let drawn = draw(p, len);
        let (image, middle) = if drawn.is_empty() { picture(&[(IVec2::ZERO, [255, 0, 255, 255])]) } else { picture(&drawn) };
        let mut samples = Vec::new();
        let mut x = -len / 2.0;
        while x <= len / 2.0 {
            samples.extend([Vec2::new(x, thick / 2.0), Vec2::new(x, -thick / 2.0)]);
            x += 2.0;
        }
        samples.extend([Vec2::new(-len / 2.0, 0.0), Vec2::new(len / 2.0, 0.0)]);
        let mass = len * thick;
        let twist = (rng.next_u32() as f32 / u32::MAX as f32 * 2.0 - 1.0) * SLUMP_TWIST;
        let pos = (p.a + p.b) / 2.0;
        let r = pos - centre;
        let mut piece = new(pos, angle, mass, mass * (len * len + thick * thick) / 12.0, samples, "limb", vel + Vec2::new(-r.y, r.x) * omega, omega + twist);
        piece.draw_offset = middle;
        // (Its pixels kept: a bone that flies back together, `rise`.)
        piece.pixels = drawn.iter().map(|(q, c)| (q.as_vec2() + Vec2::splat(0.5), *c)).collect();
        pieces.push(piece);
        looks.push(Look::Image(image, false));
        angles.push(angle);
    }
    // Each limb pinned to the one before it it meets, or to the body.
    let mut joints = Vec::new();
    for (i, p) in limbs.iter().enumerate() {
        let me = i + 1;
        let tol = (p.r * 2.0).max(2.5).max(0.6 * p.a.distance(p.b));
        let mut best: Option<(usize, Vec2, f32)> = None;
        for (j, q) in limbs[..i].iter().enumerate() {
            for (x, y) in [(p.a, q.b), (p.a, q.a), (p.b, q.b), (p.b, q.a)] {
                let dist = x.distance(y);
                if dist < tol && best.is_none_or(|b| dist < b.2) {
                    best = Some((j + 1, (x + y) / 2.0, dist));
                }
            }
        }
        let (other, at, limit) = match best {
            Some((j, at, _)) => (j, at, BEND_KNEE),
            None => (0, if p.a.distance(centre) <= p.b.distance(centre) { p.a } else { p.b }, BEND_HIP),
        };
        let (pa, pb) = (&pieces[other], &pieces[me]);
        joints.push(Joint { a: other, b: me, la: rot(at - pa.pos, -pa.angle), lb: rot(at - pb.pos, -pb.angle), rest: wrap(angles[me] - angles[other]), limit });
    }
    (Limp { pieces, joints, root: 0, still: 0, asleep: false }, looks)
}

/// A limb cut off (`wounds`): its capsules as `limbed`'s are, each pinned
/// to the one it meets, with no body (thrown at `vel`, turning `omega`).
pub fn cut_off(limbs: &[super::parts::Part], draw: impl Fn(&super::parts::Part, f32) -> Vec<(IVec2, [u8; 4])>, vel: Vec2, omega: f32) -> (Limp, Vec<Look>) {
    let at = limbs.first().map_or(Vec2::ZERO, |p| p.a);
    let (mut limp, mut looks) = limbed(Sprite::default(), Vec2::ZERO, (2, 2), at, limbs, draw, vel, omega);
    // (Built on a body, then the body taken away.)
    limp.pieces.remove(0);
    looks.remove(0);
    limp.joints.retain(|j| j.a != 0 && j.b != 0);
    for j in &mut limp.joints {
        j.a -= 1;
        j.b -= 1;
    }
    limp.root = 0;
    (limp, looks)
}

fn rot(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

fn cross(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

fn wrap(a: f32) -> f32 {
    (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// Move a piece so its point `r` (from its middle, turned) goes `by`.
fn push(p: &mut Piece, r: Vec2, by: Vec2, share: f32) {
    let len = by.length();
    if len < 1e-6 {
        return;
    }
    let n = by / len;
    let w = p.inv_mass + p.inv_inertia * cross(r, n).powi(2);
    let k = len * share / w.max(1e-9);
    p.pos += n * k * p.inv_mass;
    p.angle += p.inv_inertia * cross(r, n) * k;
}

impl Limp {
    /// A tick: moving on, falling, then put right (joints, bends, the
    /// world), `SUBSTEPS` times.
    pub fn step(&mut self, grid: &impl Grid, gravity: f32) {
        for p in &mut self.pieces {
            p.tick_pos = p.pos;
            p.tick_angle = p.angle;
        }
        if self.asleep {
            return;
        }
        let h = DT / SUBSTEPS as f32;
        for _ in 0..SUBSTEPS {
            for p in &mut self.pieces {
                let v = ((p.pos - p.prev) * AIR).clamp_length_max(FASTEST * h);
                let w = (p.angle - p.prev_angle) * SPIN_KEPT;
                p.prev = p.pos;
                p.prev_angle = p.angle;
                p.pos += v + Vec2::new(0.0, -gravity * h * h);
                p.angle += w;
            }
            let mut touching = vec![false; self.pieces.len()];
            for _ in 0..ITERATIONS {
                self.joints_together();
                self.bends_held();
                for (i, p) in self.pieces.iter_mut().enumerate() {
                    touching[i] |= out_of_cells(p, grid);
                }
            }
            // Where it touches, it drags (and doesn't bounce).
            for (p, &t) in self.pieces.iter_mut().zip(&touching) {
                if t {
                    p.prev = p.pos - (p.pos - p.prev) * GRIP;
                    p.prev_angle = p.angle - (p.angle - p.prev_angle) * GRIP;
                }
            }
        }
        let moved = self.pieces.iter().map(|p| p.pos.distance(p.tick_pos) + (p.angle - p.tick_angle).abs() * 4.0).fold(0.0f32, f32::max);
        self.still = if moved < STILL { self.still + 1 } else { 0 };
        if self.still >= STILL_TICKS {
            self.asleep = true;
            for p in &mut self.pieces {
                p.prev = p.pos;
                p.prev_angle = p.angle;
            }
        }
    }

    fn joints_together(&mut self) {
        for j in &self.joints {
            let (pa, pb) = (&self.pieces[j.a], &self.pieces[j.b]);
            let (ra, rb) = (rot(j.la, pa.angle), rot(j.lb, pb.angle));
            let gap = (pb.pos + rb) - (pa.pos + ra);
            let len = gap.length();
            if len < 1e-4 {
                continue;
            }
            let n = gap / len;
            let wa = pa.inv_mass + pa.inv_inertia * cross(ra, n).powi(2);
            let wb = pb.inv_mass + pb.inv_inertia * cross(rb, n).powi(2);
            let k = len / (wa + wb);
            let (ia, ib) = (pa.inv_inertia, pb.inv_inertia);
            let (ma, mb) = (pa.inv_mass, pb.inv_mass);
            let a = &mut self.pieces[j.a];
            a.pos += n * k * ma;
            a.angle += ia * cross(ra, n) * k;
            let b = &mut self.pieces[j.b];
            b.pos -= n * k * mb;
            b.angle -= ib * cross(rb, n) * k;
        }
    }

    fn bends_held(&mut self) {
        for j in &self.joints {
            let bent = wrap(self.pieces[j.b].angle - self.pieces[j.a].angle - j.rest);
            let over = bent.abs() - j.limit;
            if over <= 0.0 {
                continue;
            }
            let fix = over * bent.signum();
            let (ia, ib) = (self.pieces[j.a].inv_inertia, self.pieces[j.b].inv_inertia);
            let sum = (ia + ib).max(1e-9);
            self.pieces[j.a].angle += fix * ia / sum;
            self.pieces[j.b].angle -= fix * ib / sum;
        }
    }

    /// Thrown (a blast): every piece moves `vel` more (cells a second), and
    /// it wakes.
    pub fn throw(&mut self, vel: Vec2) {
        let v = vel * DT / SUBSTEPS as f32;
        for p in &mut self.pieces {
            p.prev -= v;
        }
        self.asleep = false;
        self.still = 0;
    }

    /// Where a piece is drawn, `t` of the way through the tick: its
    /// picture's middle and its turn.
    pub fn drawn(&self, i: usize, t: f32) -> (Vec2, f32) {
        let p = &self.pieces[i];
        let pos = p.tick_pos.lerp(p.pos, t);
        let angle = p.tick_angle + wrap(p.angle - p.tick_angle) * t;
        (pos + rot(p.draw_offset, angle), angle)
    }

    /// Moved bodily (every piece, as it is), awake.
    pub fn shift(&mut self, by: Vec2) {
        for p in &mut self.pieces {
            p.pos += by;
            p.prev += by;
            p.tick_pos += by;
        }
        self.asleep = false;
        self.still = 0;
    }

    pub fn root_pos(&self) -> (Vec2, Vec2) {
        let p = &self.pieces[self.root];
        (p.tick_pos, p.pos)
    }
}

/// A piece's edge points out of the cells they've gone into: each pushed
/// back the shortest way out (up first); true if any touched.
fn out_of_cells(p: &mut Piece, grid: &impl Grid) -> bool {
    let mut touched = false;
    for s in p.samples.clone() {
        let r = rot(s, p.angle);
        let w = p.pos + r;
        let (x, y) = (w.x.floor() as i32, w.y.floor() as i32);
        if !grid.solid(x, y) {
            continue;
        }
        touched = true;
        // The nearest way out: up, the sides, down, the corners, a cell
        // or a few away; to just past that cell's edge.
        let mut best: Option<Vec2> = None;
        'search: for k in 1..=4 {
            for d in [IVec2::Y, IVec2::X, IVec2::NEG_X, IVec2::NEG_Y, IVec2::new(1, 1), IVec2::new(-1, 1), IVec2::new(1, -1), IVec2::new(-1, -1)] {
                let c = IVec2::new(x, y) + d * k;
                if !grid.solid(c.x, c.y) {
                    // (To the near edge of the free cell.)
                    let target = Vec2::new(
                        if d.x > 0 { c.x as f32 + 0.01 } else if d.x < 0 { c.x as f32 + 0.99 } else { w.x },
                        if d.y > 0 { c.y as f32 + 0.01 } else if d.y < 0 { c.y as f32 + 0.99 } else { w.y },
                    );
                    best = Some(target - w);
                    break 'search;
                }
            }
        }
        if let Some(by) = best {
            push(p, r, by, 0.8);
        }
    }
    touched
}

/// Every ragdoll a tick on; its body where its torso is (and thrown with
/// it: what threw the body throws its pieces).
pub fn step_all(sim: Res<SimWorld>, mut q: Query<(&mut Limp, &mut Kinematics)>) {
    let grid = WorldGrid(&sim.world);
    for (mut limp, mut k) in &mut q {
        if k.body.vel != Vec2::ZERO {
            // (A share of what threw its body, and no more than so much: a
            // bomb's blast tumbles bodies about, not across the map.)
            let v = (k.body.vel * THROWN_SHARE).clamp_length_max(THROWN_MOST);
            limp.throw(v);
            k.body.vel = Vec2::ZERO;
        }
        limp.step(&grid, crate::props::GRAVITY);
        let (from, to) = limp.root_pos();
        k.prev_pos = from;
        k.body.pos = to;
    }
}

/// The pieces drawn where they are between ticks, under their body
/// (which `interpolate` has put where its torso is).
pub fn draw(time: Res<Time<Fixed>>, bodies: Query<(&Limp, &Transform), Without<LimpPiece>>, mut pieces: Query<(&LimpPiece, &ChildOf, &mut Transform)>) {
    let t = time.overstep_fraction();
    for (piece, parent, mut tf) in &mut pieces {
        let Ok((limp, body)) = bodies.get(parent.parent()) else { continue };
        if piece.0 >= limp.pieces.len() {
            continue;
        }
        let (at, angle) = limp.drawn(piece.0, t);
        let base = body.translation.truncate();
        tf.translation.x = at.x - base.x;
        tf.translation.y = at.y - base.y;
        tf.rotation = Quat::from_rotation_z(angle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platypus_physics::Occupancy;

    /// Solid below y 0.
    struct Floor;
    impl Grid for Floor {
        fn occupancy(&self, _x: i32, y: i32) -> Occupancy {
            if y < 0 { Occupancy::Solid } else { Occupancy::Empty }
        }
    }

    fn square(at: Vec2) -> Piece {
        let samples = vec![Vec2::new(-2.0, -2.0), Vec2::new(2.0, -2.0), Vec2::new(2.0, 2.0), Vec2::new(-2.0, 2.0)];
        Piece { pos: at, prev: at, angle: 0.0, prev_angle: 0.0, tick_pos: at, tick_angle: 0.0, inv_mass: 1.0 / 16.0, inv_inertia: 1.0 / 40.0, samples, draw_offset: Vec2::ZERO, name: "torso".into(), pixels: Vec::new() }
    }

    #[test]
    fn a_piece_falls_and_rests_on_the_floor_and_sleeps() {
        let mut limp = Limp { pieces: vec![square(Vec2::new(0.0, 20.0))], joints: Vec::new(), root: 0, still: 0, asleep: false };
        for _ in 0..600 {
            limp.step(&Floor, 1350.0);
        }
        let p = &limp.pieces[0];
        assert!((p.pos.y - 2.0).abs() < 0.6, "rests on the floor: {}", p.pos.y);
        assert!(limp.asleep, "and sleeps");
    }

    #[test]
    fn joined_pieces_stay_together() {
        let a = square(Vec2::new(0.0, 30.0));
        let b = square(Vec2::new(4.0, 30.0));
        let j = Joint { a: 0, b: 1, la: Vec2::new(2.0, 0.0), lb: Vec2::new(-2.0, 0.0), rest: 0.0, limit: 1.0 };
        let mut limp = Limp { pieces: vec![a, b], joints: vec![j], root: 0, still: 0, asleep: false };
        limp.throw(Vec2::new(0.0, 0.0));
        limp.pieces[1].prev -= Vec2::new(0.0, 0.5);
        for _ in 0..300 {
            limp.step(&Floor, 1350.0);
        }
        let (pa, pb) = (&limp.pieces[0], &limp.pieces[1]);
        let gap = (pb.pos + rot(Vec2::new(-2.0, 0.0), pb.angle)).distance(pa.pos + rot(Vec2::new(2.0, 0.0), pa.angle));
        assert!(gap < 0.3, "the joint holds: {gap}");
        assert!(pa.pos.y > 1.0 && pb.pos.y > 1.0, "both on the floor");
    }
}
