//! A body that bends (a rat's): drawn by its spine, not as one picture.
//! Its box and its own sprite are its head's (the sprite hidden: what's
//! drawn is this, its eyes still its own). Behind its head its body is cut
//! in slices from its art's columns (`SpineDef::from` to `head`), each laid
//! along the way its head went and turned to it, no more than `bend`
//! against the slice ahead: round a corner it went round, up a wall, over a
//! bump. Laid straight, it's the picture as drawn. Its tail (`TailDef`) is
//! a chain off its rump: holding its shape a little, sagging, lagging
//! behind as it goes, lying on the ground it drags over.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use bevy::prelude::*;
use bevy::sprite::Anchor;
use serde::Deserialize;

use super::animation::{Animator, CreatureSprite};
use crate::world::SimWorld;

pub struct SpinePlugin;

impl Plugin for SpinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Pieces>().add_systems(Update, (grow, follow).chain().after(super::animation::animate));
    }
}

/// A body drawn along its spine, as a creature file writes it.
#[derive(Clone, Debug, Deserialize)]
pub struct SpineDef {
    /// Its body's columns in its art (pixels): from its rump to where its
    /// head begins (the head, everything from there on, drawn whole).
    pub from: f32,
    pub head: f32,
    /// How many slices its body is cut in, and the row through their
    /// middles (pixels from the top): where its spine runs.
    pub slices: usize,
    pub row: f32,
    /// The most a slice turns against the one ahead (degrees; the first,
    /// against the head, half of it).
    #[serde(default = "bend")]
    pub bend: f32,
    #[serde(default)]
    pub tail: Option<TailDef>,
}

fn bend() -> f32 {
    12.0
}

/// A tail: a chain off the body.
#[derive(Clone, Debug, Deserialize)]
pub struct TailDef {
    /// Where it leaves the body (art pixels), how long it is (cells) in so
    /// many links, how thick at its root and at its tip (cells).
    pub root: (f32, f32),
    pub length: f32,
    #[serde(default = "links")]
    pub links: usize,
    pub width: (f32, f32),
    /// Its colour, and (if any) its underside's.
    pub color: (u8, u8, u8),
    #[serde(default)]
    pub shade: Option<(u8, u8, u8)>,
    /// How much it holds its shape (0: limp, 1: stiff), and how heavily it
    /// hangs (1: as a rope).
    #[serde(default = "stiff")]
    pub stiff: f32,
    #[serde(default = "sag")]
    pub sag: f32,
}

fn links() -> usize {
    8
}
fn stiff() -> f32 {
    0.35
}
fn sag() -> f32 {
    0.4
}

/// A kind's art cut up: its head (the frame with its body left out) and
/// each slice (the frame with only its columns), every frame.
struct Cut {
    head: Vec<Handle<Image>>,
    slices: Vec<Vec<Handle<Image>>>,
    size: Vec2,
}

impl Cut {
    fn new(art: &platypus_art::Art, sd: &SpineDef, images: &mut Assets<Image>) -> Self {
        let (w, h) = art.size;
        let mut image = |keep: &dyn Fn(u32) -> bool, f: &platypus_art::Pixels| {
            let mut rgba = vec![0u8; (w * h * 4) as usize];
            for y in 0..h {
                for x in (0..w).filter(|x| keep(*x)) {
                    let i = ((y * w + x) * 4) as usize;
                    rgba[i..i + 4].copy_from_slice(&f.get(x as i32, y as i32));
                }
            }
            images.add(Image::new(
                bevy::render::render_resource::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                bevy::render::render_resource::TextureDimension::D2,
                rgba,
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
            ))
        };
        let head = sd.head as u32;
        let heads = art.frames.iter().map(|f| image(&|x| x >= head, f)).collect();
        // (Each two columns on toward the head, under the slice ahead: no
        // gap opens between two as the body bends, and nothing's drawn
        // twice.)
        let slices = (0..sd.slices)
            .map(|s| {
                let (a, b) = sd.columns(s);
                let (a, b) = (a.round().max(sd.from) as u32, (b.round() as u32 + 2).min(head + 2));
                art.frames.iter().map(|f| image(&|x| (a..b).contains(&x), f)).collect()
            })
            .collect();
        Cut { head: heads, slices, size: Vec2::new(w as f32, h as f32) }
    }
}

impl SpineDef {
    /// Slice `s`'s columns (the first is next to the head).
    fn columns(&self, s: usize) -> (f32, f32) {
        let w = (self.head - self.from) / self.slices as f32;
        (self.head - w * (s + 1) as f32, self.head - w * s as f32)
    }
}

/// How near rock a point of its body holds on (cells): further from any,
/// it falls.
const HOLD: i32 = 7;
/// How far above and below its joints a body looks for what it stands on
/// (cells, past its ride), and how fast it settles onto it.
const REACH: i32 = 4;
const SETTLE: f32 = 22.0;
/// The most a tail's link bends against the one before it (degrees).
const TAIL_BEND: f32 = 28.0;

/// Every kind's art, cut up once.
#[derive(Resource, Default)]
struct Pieces(HashMap<String, Arc<Cut>>);

/// What a spine draws with (children of the creature).
#[derive(Component)]
struct SpinePart;

/// A body drawn along its spine: where its head went (the way, its head
/// end first), what draws it, its tail's links.
#[derive(Component)]
pub struct Spine {
    cut: Arc<Cut>,
    way: VecDeque<Vec2>,
    head: Entity,
    slices: Vec<Entity>,
    /// Each link's outline and its fill.
    links: Vec<(Entity, Entity)>,
    /// Where one slice meets the next (its neck first), set down on what
    /// it stands on.
    joints: Vec<Vec2>,
    tail: Vec<Vec2>,
    tail_was: Vec<Vec2>,
    /// Mirrored (facing left) last frame; where its neck was.
    mirrored: bool,
    neck: Option<Vec2>,
}

fn grow(mut commands: Commands, mut pieces: ResMut<Pieces>, mut images: ResMut<Assets<Image>>, q: Query<(Entity, &Animator, &crate::creatures::Creature), Without<Spine>>) {
    for (e, anim, c) in &q {
        let (Some(sd), Some(rig)) = (anim.def.spine.as_ref(), anim.def.rig.as_ref()) else { continue };
        let cut = pieces.0.entry(c.kind.clone()).or_insert_with(|| Arc::new(Cut::new(rig, sd, &mut images))).clone();
        let head = commands.spawn((Sprite::from_image(cut.head[0].clone()), Transform::default(), SpinePart, ChildOf(e))).id();
        let slices = (0..sd.slices).map(|s| commands.spawn((Sprite::from_image(cut.slices[s][0].clone()), Anchor::CENTER, Transform::default(), SpinePart, ChildOf(e))).id()).collect();
        let n = sd.tail.as_ref().map_or(0, |t| t.links);
        let links = (0..n)
            .map(|_| {
                let line = commands.spawn((Sprite::from_color(Color::BLACK, Vec2::ONE), Transform::default(), SpinePart, ChildOf(e))).id();
                let fill = commands.spawn((Sprite::from_color(Color::WHITE, Vec2::ONE), Transform::default(), SpinePart, ChildOf(e))).id();
                (line, fill)
            })
            .collect();
        commands.entity(e).insert(Spine { cut, way: VecDeque::new(), head, slices, links, joints: Vec::new(), tail: Vec::new(), tail_was: Vec::new(), mirrored: false, neck: None });
    }
}

/// Turn `v` by `a` radians.
fn turned(v: Vec2, a: f32) -> Vec2 {
    Vec2::from_angle(a).rotate(v)
}

/// A point `d` back along a way (from `from`), and which way it runs there
/// (toward the head).
fn along(from: Vec2, way: &VecDeque<Vec2>, d: f32) -> Vec2 {
    let mut left = d;
    let mut a = from;
    for &b in way {
        let len = a.distance(b);
        if left <= len && len > 0.0 {
            return a.lerp(b, left / len);
        }
        left -= len;
        a = b;
    }
    a
}

#[allow(clippy::type_complexity)]
fn follow(
    time: Res<Time>,
    sim: Res<SimWorld>,
    mut q: Query<(&mut Spine, &Animator, &crate::creatures::Kinematics, &GlobalTransform, &Children), Without<crate::creatures::Dormant>>,
    mut mains: Query<(&Sprite, &Transform, &mut Visibility), (With<CreatureSprite>, Without<SpinePart>)>,
    mut parts: Query<(&mut Sprite, &mut Transform, Option<&mut Anchor>), (With<SpinePart>, Without<CreatureSprite>)>,
) {
    let dt = time.delta_secs().min(0.05);
    for (mut spine, anim, k, gt, children) in &mut q {
        let Some(sd) = anim.def.spine.as_ref() else { continue };
        let Some(child) = children.iter().find(|c| mains.contains(*c)) else { continue };
        let Ok((main, tf, mut vis)) = mains.get_mut(child) else { continue };
        *vis = Visibility::Hidden;
        let (color, frame) = (main.color, anim.shown);
        let o = gt.translation().truncate();
        // The head drawn as its sprite was to be (mirrored by its scale, so
        // a slice's off-centre pivot mirrors with it).
        let m = if main.flip_x { -1.0 } else { 1.0 };
        let sc = tf.scale.x.abs();
        let rot = tf.rotation;
        let size = spine.cut.size;
        // A pixel of its frame, drawn as its head is, in the world.
        let at = |px: f32, py: f32| o + tf.translation.truncate() + (rot * Vec3::new((px - size.x / 2.0) * sc * m, (size.y / 2.0 - py) * sc, 0.0)).truncate();
        let forward = (rot * Vec3::new(m, 0.0, 0.0)).truncate();
        let neck = at(sd.head, sd.row);
        let long = (sd.head - sd.from) * sc;
        // The way its head went: begun afresh (put down, or far from where
        // it was), mirrored with it as it turns round, its neck added on as
        // it goes; never ahead of its neck (backing up, the way's taken
        // back in); as long as its body.
        if spine.way.is_empty() || spine.neck.is_none_or(|n| n.distance(neck) > 40.0) {
            spine.way = (1..=(long as i32 + 4)).map(|k| neck - forward * k as f32).collect();
        } else if spine.mirrored != (m < 0.0) {
            // (Its body, where it's settled, and its tail with it: else they'd
            // swing across through its head to the other side.)
            let (mid, across) = (o + tf.translation.truncate(), (rot * Vec3::X).truncate());
            let sp = &mut *spine;
            for p in sp.way.iter_mut().chain(sp.joints.iter_mut()).chain(sp.tail.iter_mut()).chain(sp.tail_was.iter_mut()) {
                *p -= across * 2.0 * (*p - mid).dot(across);
            }
        }
        spine.mirrored = m < 0.0;
        spine.neck = Some(neck);
        while spine.way.len() > 2 && (spine.way[0] - neck).dot(forward) > -0.5 {
            spine.way.pop_front();
        }
        if spine.way.front().is_none_or(|p| p.distance(neck) >= 1.0) {
            spine.way.push_front(neck);
        }
        let mut gone = 0.0;
        let mut keep = spine.way.len();
        let mut prev = neck;
        for (i, p) in spine.way.iter().enumerate() {
            gone += prev.distance(*p);
            prev = *p;
            if gone > long + 3.0 {
                keep = i + 1;
                break;
            }
        }
        spine.way.truncate(keep);
        while gone < long + 3.0 {
            let n = spine.way.len();
            let end = spine.way[n - 1];
            let back = if n > 1 { (end - spine.way[n - 2]).normalize_or(-forward) } else { -forward };
            spine.way.push_back(end + back);
            gone += 1.0;
        }
        // In the air (a pounce, a fall), its body straightens out behind
        // its head; on the ground, what of it has nothing near it falls,
        // onto what's under it (a body draped off a ledge, a leap come
        // down): not left hanging along the way its head went.
        let solid = |p: Vec2| sim.world.is_solid(platypus_sim::CellPos::from_world(p.x, p.y));
        let aloft = !k.loco.grounded() && k.loco.clinging().is_none();
        let straighten = 1.0 - (-dt * 10.0).exp();
        let mut d = 0.0;
        let mut prev = neck;
        for i in 0..spine.way.len() {
            let p = spine.way[i];
            d += prev.distance(p);
            prev = p;
            let p = if aloft { p.lerp(neck - forward * d, straighten) } else { p };
            let held = [Vec2::NEG_Y, Vec2::Y, Vec2::X, Vec2::NEG_X, Vec2::new(0.7, -0.7), Vec2::new(-0.7, -0.7)].iter().any(|dir| (1..=HOLD).any(|c| solid(p + *dir * c as f32)));
            spine.way[i] = if held || aloft { p } else { p - Vec2::Y * (220.0 * dt).min(2.0) };
        }
        // Its joints (where one slice meets the next) along the way its head
        // went; on what it stands on (the ground, a wall it holds), each set
        // down on what's under its belly there, as high off it as its neck
        // rides: its body lies over the bumps and down in the dips, along
        // the slope where it is, not the line its head drew. (Eased there:
        // it settles, it doesn't jitter.)
        let n = sd.slices;
        let width = (sd.head - sd.from) / n as f32;
        let w = width * sc;
        let ride = (anim.def.sprite.feet.1 - sd.row) * sc;
        let mut want: Vec<Vec2> = (0..=n).map(|j| along(neck, &spine.way, j as f32 * w)).collect();
        if !aloft {
            for (j, p) in want.iter_mut().enumerate().skip(1) {
                let d = j as f32 * w;
                let t = (along(neck, &spine.way, (d - 1.5).max(0.0)) - along(neck, &spine.way, d + 1.5)).normalize_or(forward);
                let up = t.perp() * m;
                if let Some(c) = (-REACH..=(ride as i32 + REACH)).find(|c| solid(*p - up * *c as f32)) {
                    *p = *p - up * c as f32 + up * (ride + 1.0);
                }
            }
        }
        if spine.joints.len() != n + 1 || spine.joints[1].distance(want[1]) > 30.0 {
            spine.joints = want.clone();
        }
        let settle = (dt * SETTLE).min(1.0);
        for (j, p) in want.iter().enumerate() {
            spine.joints[j] = if j == 0 { *p } else { spine.joints[j].lerp(*p, settle) };
        }
        // Its slices: each turned from one joint to the next (no more than
        // `bend` against the one ahead), its front where the last one's
        // back is.
        let most = sd.bend.to_radians();
        let mut joint = neck;
        let mut ahead = forward;
        let mut last = (neck, forward, sd.head);
        for s in 0..n {
            let (a, b) = sd.columns(s);
            let mid = (a + b) / 2.0;
            let half = w * 0.5;
            let want = (spine.joints[s] - spine.joints[s + 1]).normalize_or(ahead);
            let most = if s == 0 { most * 0.5 } else { most };
            let turn = ahead.angle_to(want).clamp(-most, most);
            let dir = turned(ahead, turn);
            let centre = joint - dir * half;
            joint -= dir * half * 2.0;
            ahead = dir;
            last = (centre, dir, mid);
            if let Ok((mut sprite, mut stf, anchor)) = parts.get_mut(spine.slices[s]) {
                sprite.image = spine.cut.slices[s].get(frame).cloned().unwrap_or_default();
                sprite.color = color;
                if let Some(mut anchor) = anchor {
                    anchor.0 = Vec2::new((mid - size.x / 2.0) / size.x, (size.y / 2.0 - sd.row) / size.y);
                }
                let f = dir * m;
                stf.translation = (centre - o).extend(-0.002 * (s + 1) as f32);
                stf.rotation = Quat::from_rotation_z(f.y.atan2(f.x));
                stf.scale = Vec3::new(sc * m, sc, 1.0);
            }
        }
        if let Ok((mut sprite, mut stf, _)) = parts.get_mut(spine.head) {
            sprite.image = spine.cut.head.get(frame).cloned().unwrap_or_default();
            sprite.color = color;
            *stf = Transform { translation: tf.translation, rotation: rot, scale: Vec3::new(sc * m, sc, 1.0) };
        }
        // Its tail, off its last slice.
        let Some(td) = sd.tail.as_ref() else { continue };
        let (centre, dir, mid) = last;
        let up = dir.perp() * m;
        let root = centre + dir * (td.root.0 - mid) * sc + up * (sd.row - td.root.1) * sc;
        let n = td.links;
        let link = td.length / n as f32;
        if spine.tail.len() != n + 1 || spine.tail[0].distance(root) > 40.0 {
            spine.tail = (0..=n).map(|i| root - dir * link * i as f32).collect();
            spine.tail_was = spine.tail.clone();
        }
        spine.tail[0] = root;
        spine.tail_was[0] = root;
        // Moving with what it had (damped), sagging; held toward the shape
        // it keeps (back from the rump, curling a little down), less and
        // less toward its tip; each link its length from the last.
        let ease = (dt * 12.0).min(1.0);
        let mut rest = root;
        for i in 1..=n {
            let k = i as f32 / n as f32;
            let p = spine.tail[i];
            let v = (p - spine.tail_was[i]) * 0.8;
            spine.tail_was[i] = p;
            let mut p = p + v + Vec2::NEG_Y * 260.0 * td.sag * dt * dt;
            rest += turned(-dir, -0.18 * k * m) * link;
            p += (rest - p) * td.stiff * (1.0 - k * 0.7) * ease;
            spine.tail[i] = p;
        }
        // (Each link its length on from the last, bent no more than
        // `TAIL_BEND` against it: a tail curves, it doesn't kink or loop.)
        let most = TAIL_BEND.to_radians();
        for _ in 0..3 {
            let mut ahead = -dir;
            for i in 1..=n {
                let (a, b) = (spine.tail[i - 1], spine.tail[i]);
                let want = (b - a).normalize_or(ahead);
                let way = turned(ahead, ahead.angle_to(want).clamp(-most, most));
                ahead = way;
                let mut p = a + way * link;
                // (Lying on what it drags over, not through it.)
                let mut lift = 0;
                while lift < 4 && sim.world.is_solid(platypus_sim::CellPos::from_world(p.x, p.y)) {
                    p.y += 1.0;
                    lift += 1;
                }
                spine.tail[i] = p;
            }
        }
        let outline = anim.def.art_file.as_ref().and_then(|f| f.outline).unwrap_or((22, 16, 14));
        let rgb = |(r, g, b): (u8, u8, u8)| Color::srgb_u8(r, g, b);
        for i in 0..n {
            let (a, b) = (spine.tail[i], spine.tail[i + 1]);
            let k = (i as f32 + 0.5) / n as f32;
            let w = td.width.0 + (td.width.1 - td.width.0) * k;
            let mid = (a + b) / 2.0 - o;
            let angle = (b - a).y.atan2((b - a).x);
            let len = a.distance(b) + 0.8;
            let (line, fill) = spine.links[i];
            let shade = if i % 2 == 1 { td.shade.unwrap_or(td.color) } else { td.color };
            for (part, (wide, long, c, z)) in [(line, (w + 2.0, len + 1.0, rgb(outline), -0.05)), (fill, (w, len, rgb(shade), -0.04))] {
                if let Ok((mut sprite, mut stf, _)) = parts.get_mut(part) {
                    sprite.custom_size = Some(Vec2::new(long, wide));
                    sprite.color = c.mix(&color, 0.0);
                    stf.translation = mid.extend(z);
                    stf.rotation = Quat::from_rotation_z(angle);
                    stf.scale = Vec3::ONE;
                }
            }
        }
    }
}
