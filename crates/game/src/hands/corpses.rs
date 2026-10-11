//! Bodies (DESIGN §7c, §8): what a creature leaves when it dies. Its picture
//! as it fell, laid down (on its back, the way it was facing; a spider as it
//! was, from above), darkened: a body that falls, slides and is thrown by
//! blasts. It holds what the creature wore and held, what it carried
//! (`drops`) and a roll of its loot table (`loot`, at the depth it died,
//! with the player's luck): right-click it to open it, as a chest.
//!
//! A body stays as long as something's in it. Emptied (or never holding
//! anything) it lies a while, then fades. There are at most `MAX_BODIES`:
//! past that, the oldest go, with what's in them (a battle leaves a field
//! of bodies, not a thousand of them). A creature with `corpse: false`
//! (an egg sac, a cocoon, a firefly) leaves none: what it had spills out.
//! A dead spider keeps its legs, curled in over its body (`curled_legs`).

use bevy::prelude::*;
use platypus_physics::{Body, Locomotion};
use platypus_sim::rng::Rng;

use super::chests::{Chests, Container, SLOTS};
use super::items::{Inventory, Items, Stack};
use super::spawn_drop;
use crate::creatures::player::LocalPlayer;
use crate::creatures::body::ragdoll;
use crate::creatures::{Died, Kinematics};
use crate::fx::Explosion;
use crate::props::Thrown;
use crate::world::{SimWorld, TICK_HZ, TickSet};

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// An empty body lies this long (seconds), then fades over `FADE`.
const LINGER: f32 = 20.0;
const FADE: f32 = 3.0;
/// Bodies draw behind the living, over the world (chests are at 6).
const Z: f32 = 5.5;
/// A body's colours, darkened.
const DIM: f32 = 0.72;
/// Most bodies lying about at once.
pub const MAX_BODIES: usize = 150;

/// A body in the world (its contents are kept by `Chests`, under `key`).
#[derive(Component)]
pub struct Corpse {
    pub key: u64,
    /// Seconds it's been empty.
    empty: f32,
    /// When it fell (a count: the oldest go first).
    born: u64,
}

/// The picture of a body (its child).
#[derive(Component)]
struct CorpseSprite;

pub struct CorpsesPlugin;

impl Plugin for CorpsesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (lay_out, (batter, rot, slump, ragdoll::step_all.after(batter)).after(crate::props::fly)).in_set(TickSet::Bodies))
            .add_systems(PostUpdate, (ragdoll, ragdoll::draw).after(crate::creatures::interpolate).before(bevy::transform::TransformSystems::Propagate));
    }
}

/// What a dead creature had on it: what it wore and held (gear keeps its
/// roll), what it carried, and a roll of its loot table.
fn belongings(died: &Died, items: &Items, chests: &Chests, world: &platypus_sim::World, luck: f32) -> Inventory {
    let mut inv = Inventory::new(SLOTS);
    for s in &died.worn {
        inv.add(items, *s);
    }
    for (id, n) in &died.def.drops {
        if let Some(item) = items.id(id) {
            inv.add(items, Stack::new(item, n * items.unit(item)));
        }
    }
    if let Some(table) = &died.def.loot {
        let at = died.body.pos;
        let mut rng = Rng::seeded(&[world.seed(), world.tick(), at.x.to_bits() as u64, at.y.to_bits() as u64, 0xDEAD]);
        chests.roll_table(table, &mut inv, items, chests.level_at(world, at), luck, &mut rng);
    }
    inv
}

/// A body drawn in parts (its art's poses: a humanoid) as a ragdoll: the
/// parts of the frame it fell in, dressed in what it wore, moving as it
/// was. None for a body that isn't (a sprite, a walker on legs).
fn limp_of(d: &Died, items: &Items, blood: (u8, u8, u8)) -> Option<(ragdoll::Limp, Vec<(ragdoll::Look, f32)>)> {
    let file = d.def.art_file.as_ref()?;
    if file.poses.is_empty() {
        return None;
    }
    let rig = d.def.rig.as_ref()?;
    let (sprite, offset) = d.sprite.as_ref()?;
    let frame = rig.names.get(sprite.texture_atlas.as_ref()?.index)?;
    let mut skins = crate::gear::look::skins_of(items, &d.worn);
    // (Its wounds' marks on it still.)
    let marked = (!d.marks.is_empty()).then(|| crate::creatures::body::marks::skin(&d.marks, crate::creatures::body::marks::Tissue::of(d.def.kind.as_deref()), blood));
    if let Some(m) = &marked {
        skins.push(m);
    }
    let dressed = if skins.is_empty() { (**file).clone() } else { platypus_art::dress::dress(file, &skins).ok()? };
    // (What it had lost lies apart.)
    let cuts: Vec<ragdoll::Cut> = ragdoll::cut(&dressed, frame).into_iter().filter(|c| c.limb().is_none_or(|l| !d.severed.contains(&l))).collect();
    if cuts.len() < 2 {
        return None;
    }
    let centre = d.body.pos + offset.truncate();
    // Bones (what rises, and what's broken for good) fall apart: rubble
    // (with its legs, if they're drawn apart from its art: `legs::rig`).
    if d.def.rises.as_ref().is_some_and(|r| r.remains == crate::creatures::body::rise::Remains::Rubble) {
        let (mut limp, mut looks) = ragdoll::rubble(&cuts, d.def.sprite.frame, centre, d.facing, d.body.vel);
        if let Some(legs) = d.def.legs.as_ref() {
            ragdoll::loose_limbs(&mut limp, &mut looks, &d.limbs, limb_drawer(legs), d.body.vel);
        }
        let looks = looks.into_iter().enumerate().map(|(i, l)| (l, i as f32 * 0.0005)).collect();
        return Some((limp, looks));
    }
    let limp = ragdoll::assemble(&cuts, d.def.sprite.frame, centre, d.facing, d.body.vel, d.spin.omega);
    // (Its parts in their pose's order, mirrored as it faced.)
    let looks = cuts
        .into_iter()
        .enumerate()
        .map(|(i, c)| (ragdoll::Look::Image(c.image, d.facing < 0.0), i as f32 * 0.01))
        .collect();
    Some((limp, looks))
}

/// How a walker's limbs are drawn, laid along x from their middle (a
/// capsule `len` long), as they're drawn alive (`legs::draw`): outlined, a
/// knuckle at the joint; a mech's legs as plates, shaded under.
pub(crate) fn limb_drawer(legs: &crate::creatures::body::legs::LegsDef) -> impl Fn(&crate::creatures::body::parts::Part, f32) -> Vec<(IVec2, [u8; 4])> + '_ {
    use crate::creatures::body::legs::{LegStyle, plate, stroke};
    use crate::creatures::body::parts::PartKind;
    let rgb = |(r, g, b): (u8, u8, u8)| [r, g, b, 255];
    let dark = |(r, g, b): (u8, u8, u8)| ((r as f32 * 0.6) as u8, (g as f32 * 0.6) as u8, (b as f32 * 0.6) as u8);
    let joint = legs.joint.unwrap_or(legs.color);
    let segment = legs.segments.as_ref().map(|s| s.color);
    let chain = legs.chains.iter().find_map(|c| c.color);
    let line = legs.outline.unwrap_or_else(|| dark(dark(legs.color)));
    move |p: &crate::creatures::body::parts::Part, len: f32| {
        let (a, b) = (Vec2::new(-len / 2.0, 0.0), Vec2::new(len / 2.0, 0.0));
        let r = (p.r - 0.5).max(0.5);
        let color = match p.kind {
            PartKind::Segment => segment.unwrap_or(legs.color),
            PartKind::Chain => chain.unwrap_or(legs.color),
            _ => legs.color,
        };
        let mut out = Vec::new();
        if legs.style == LegStyle::Plate && p.kind == PartKind::Leg {
            plate(a, b, r + 1.0, rgb(line), None, &mut out, Vec2::Y);
            plate(a, b, r, rgb(color), None, &mut out, Vec2::Y);
            plate(a, b, r, rgb(color), Some(rgb(dark(color))), &mut out, Vec2::Y);
        } else {
            stroke(a, b, r + 1.0, r + 1.0, rgb(line), &mut out);
            stroke(a, b, r, r, rgb(color), &mut out);
        }
        if p.kind == PartKind::Leg || p.kind == PartKind::Arm {
            let k = (r * 0.5).max(0.8);
            stroke(a, a, k, k, rgb(joint), &mut out);
        }
        out
    }
}

/// A walker (a body on procedural limbs) as a ragdoll: its body's sprite
/// and each limb as it was. None without limbs.
fn walker_of(d: &Died) -> Option<(ragdoll::Limp, Vec<(ragdoll::Look, f32)>)> {
    let legs = d.def.legs.as_ref()?;
    let (sprite, offset) = d.sprite.as_ref()?;
    if d.limbs.is_empty() {
        return None;
    }
    let draw = limb_drawer(legs);
    let (limp, looks) = ragdoll::limbed(sprite.clone(), offset.truncate(), d.def.sprite.frame, d.body.pos, &d.limbs, draw, d.body.vel, d.spin.omega);
    // (The body over its limbs.)
    let looks = looks.into_iter().enumerate().map(|(i, l)| (l, if i == 0 { 0.05 } else { i as f32 * 0.0005 })).collect();
    Some((limp, looks))
}

/// How tall a walker on legs lies dead (cells): its body, not its legs.
const BELLY: f32 = 14.0;

/// A dead spider's legs (from above, its body pointing right): each thigh
/// out from its hip, shorter than in life, the shin hooked back in over the
/// body, as dead spiders curl; a little crooked each (from
/// `seed`). Returns the picture and where its middle sits from the body's.
fn curled_legs(def: &crate::creatures::body::legs::LegsDef, seed: u64) -> (Image, Vec2) {
    let (leg, joint) = (def.color, def.joint.unwrap_or(def.color));
    let per_side = (def.legs() / 2).max(1);
    let (thigh, shin) = (def.reach * def.upper * 0.5, def.reach * (1.0 - def.upper) * 0.55);
    let mut rng = Rng::seeded(&[seed, 0x1E65]);
    let mut jitter = |k: f32| (rng.next_u32() as f32 / u32::MAX as f32 * 2.0 - 1.0) * k;
    let mut px: Vec<(IVec2, (u8, u8, u8))> = Vec::new();
    let line = |a: Vec2, b: Vec2, thick: i32, c: (u8, u8, u8), px: &mut Vec<(IVec2, (u8, u8, u8))>| {
        let n = (b - a).abs().max_element().ceil().max(1.0) as i32;
        let d = b - a;
        let across = if d.x.abs() > d.y.abs() { IVec2::Y } else { IVec2::X };
        for i in 0..=n {
            let p = a + d * (i as f32 / n as f32);
            let p = IVec2::new(p.x.floor() as i32, p.y.floor() as i32);
            for w in 0..thick.max(1) {
                px.push((p + across * (w - (thick - 1) / 2), c));
            }
        }
    };
    for side in [1.0f32, -1.0] {
        for j in 0..per_side {
            let along = if per_side > 1 { j as f32 / (per_side - 1) as f32 - 0.5 } else { 0.0 };
            let hip = Vec2::new(def.hips * 0.4 - along * def.spread * 1.6, side * 4.5);
            // Front legs out and forward, back ones out and back...
            let out = (80.0 - along * 120.0 + jitter(10.0)).to_radians();
            let dir = Vec2::new(out.cos(), side * out.sin());
            let knee = hip + dir * thigh * (0.9 + jitter(0.15));
            // ... each shin hooked back in over the body, spindly.
            let fold = Vec2::from_angle(side * (125.0 + jitter(14.0)).to_radians()).rotate(dir);
            let foot = knee + fold * shin * (0.85 + jitter(0.2));
            line(hip, knee, (def.thick as i32 - 1).max(1), leg, &mut px);
            line(knee, foot, 1, leg, &mut px);
            px.push((IVec2::new(knee.x.floor() as i32, knee.y.floor() as i32), joint));
        }
    }
    picture(px)
}

/// A dead walker's legs seen from the side (facing right): each thigh down
/// and out from its hip, the shin folded back in under the body, as a dead
/// insect's do; the far ones darker; a little crooked each (from `seed`).
/// Returns the picture and where its middle sits from the body's.
fn folded_legs(def: &crate::creatures::body::legs::LegsDef, seed: u64) -> (Image, Vec2) {
    let dark = |(r, g, b): (u8, u8, u8)| ((r as f32 * 0.6) as u8, (g as f32 * 0.6) as u8, (b as f32 * 0.6) as u8);
    let (thigh, shin) = (def.reach * def.upper * 0.45, def.reach * (1.0 - def.upper) * 0.4);
    let mut rng = Rng::seeded(&[seed, 0xF01D]);
    let mut jitter = |k: f32| (rng.next_u32() as f32 / u32::MAX as f32 * 2.0 - 1.0) * k;
    let mut px: Vec<(IVec2, (u8, u8, u8))> = Vec::new();
    // (The far legs first: the near ones over them.)
    let mut legs: Vec<&crate::creatures::body::legs::LegDef> = def.each.iter().collect();
    legs.sort_by_key(|l| !l.far);
    for l in legs {
        let (leg, joint) = if l.far { (def.far.unwrap_or(dark(def.color)), dark(def.joint.unwrap_or(def.color))) } else { (def.color, def.joint.unwrap_or(def.color)) };
        let out = if l.lean != 0.0 { l.lean.signum() } else { l.hip.0.signum() };
        let hip = Vec2::new(l.hip.0, l.hip.1);
        let down = Vec2::from_angle((-55.0 + jitter(12.0)).to_radians());
        let knee = hip + Vec2::new(down.x * out, down.y) * thigh;
        let fold = Vec2::from_angle((150.0 + jitter(15.0)).to_radians());
        let foot = knee + Vec2::new(fold.x * out, fold.y) * shin;
        line_px(hip, knee, (def.thick as i32).max(1), leg, &mut px);
        line_px(knee, foot, (def.thick as i32 - 1).max(1), leg, &mut px);
        px.push((IVec2::new(knee.x.floor() as i32, knee.y.floor() as i32), joint));
    }
    picture(px)
}

/// A line of cells `thick` across.
fn line_px(a: Vec2, b: Vec2, thick: i32, c: (u8, u8, u8), px: &mut Vec<(IVec2, (u8, u8, u8))>) {
    let n = (b - a).abs().max_element().ceil().max(1.0) as i32;
    let d = b - a;
    let across = if d.x.abs() > d.y.abs() { IVec2::Y } else { IVec2::X };
    for i in 0..=n {
        let p = a + d * (i as f32 / n as f32);
        let p = IVec2::new(p.x.floor() as i32, p.y.floor() as i32);
        for w in 0..thick.max(1) {
            px.push((p + across * (w - (thick - 1) / 2), c));
        }
    }
}

/// Cells as a picture, and where its middle is.
fn picture(px: Vec<(IVec2, (u8, u8, u8))>) -> (Image, Vec2) {
    let lo = px.iter().fold(IVec2::MAX, |m, (p, _)| m.min(*p));
    let hi = px.iter().fold(IVec2::MIN, |m, (p, _)| m.max(*p));
    let (w, h) = ((hi.x - lo.x + 1) as u32, (hi.y - lo.y + 1) as u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for (p, (r, g, b)) in px {
        let i = (((hi.y - p.y) as u32 * w + (p.x - lo.x) as u32) * 4) as usize;
        rgba[i..i + 4].copy_from_slice(&[r, g, b, 255]);
    }
    let image = Image::new(
        bevy::render::render_resource::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        rgba,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    (image, Vec2::new((lo.x + hi.x + 1) as f32 / 2.0, (lo.y + hi.y + 1) as f32 / 2.0))
}

/// Every creature that died leaves its body (or spills what it had).
#[allow(clippy::too_many_arguments)]
fn lay_out(
    mut commands: Commands,
    mut died: MessageReader<Died>,
    items: Option<Res<Items>>,
    sim: Res<SimWorld>,
    mut chests: ResMut<Chests>,
    player: Query<&crate::gear::Stats, With<LocalPlayer>>,
    mut images: ResMut<Assets<Image>>,
    mut count: Local<u64>,
) {
    let Some(items) = items else { return };
    let luck = player.single().map_or(0.0, |s| s.get(crate::gear::Stat::Luck));
    for d in died.read() {
        // (Bones that'll get up hold what it wore and held: its loot is for
        // when it's dead for good.)
        let inv = if d.rising.is_some() {
            let mut inv = Inventory::new(SLOTS);
            for s in &d.worn {
                inv.add(&items, *s);
            }
            inv
        } else {
            belongings(d, &items, &chests, &sim.world, luck)
        };
        let at = d.body.pos;
        // Its gold bursts out (not looted: `gold.rs`).
        if let Some(table) = d.def.loot.as_ref().filter(|_| d.rising.is_none()) {
            let mut rng = Rng::seeded(&[sim.world.seed(), sim.world.tick(), at.x.to_bits() as u64, at.y.to_bits() as u64, 0x601D]);
            let gold = chests.gold_of(table, &sim.world, at, &mut rng);
            if gold > 0 {
                chests.gold_found.push((at, gold));
            }
        }
        if !d.def.corpse {
            for (k, s) in inv.slots.iter().flatten().enumerate() {
                spawn_drop(&mut commands, &items, at + Vec2::new(k as f32 * 1.05 - 3.0, 0.0), *s);
            }
            continue;
        }
        let key = chests.stash(at, inv);
        // A body drawn in parts (a humanoid), or on limbs (a walker): a ragdoll.
        let blood = sim.world.materials().id(&d.def.blood).map_or((140, 24, 26), |m| {
            let def = sim.world.materials().def(m);
            def.colors[def.colors.len() / 2]
        });
        if let Some((limp, looks)) = limp_of(d, &items, blood).or_else(|| walker_of(d)) {
            let root_at = limp.pieces[limp.root].pos;
            // (Its box, for opening it: lying down.)
            let body = Body::new(root_at, Vec2::new(d.body.half.y, d.body.half.x) * 2.0);
            let mut e = commands.spawn((
                Name::new(format!("Body of {}", d.def.name)),
                Corpse { key, empty: 0.0, born: { *count += 1; *count } },
                Container { key, name: if d.rising.is_some() { format!("{} (bones)", d.def.name) } else { format!("{} (dead)", d.def.name) } },
                Kinematics { body, loco: Locomotion::default(), prev_pos: root_at },
                Transform::from_translation(root_at.extend(Z)),
                Visibility::default(),
            ));
            for (i, (look, z)) in looks.into_iter().enumerate() {
                let mut s = match look {
                    ragdoll::Look::Image(image, flip) => Sprite { flip_x: flip, ..Sprite::from_image(images.add(image)) },
                    ragdoll::Look::Sprite(s) => s,
                };
                s.color = Color::srgb(DIM, DIM, DIM);
                e.with_child((CorpseSprite, ragdoll::LimpPiece(i), s, Transform::from_xyz(0.0, 0.0, z)));
            }
            e.insert(limp);
            if let Some(rise) = &d.rising {
                crate::creatures::body::rise::make_pile(&mut e, d, rise, sim.world.materials().id(&d.def.blood));
            }
            continue;
        }
        // Laid on its back, head the way it faced from; a spider (drawn
        // from above) as it was; a walker on legs down on its belly, its
        // box no more than its body (not its legs).
        let side = d.def.legs.as_ref().is_some_and(|l| l.view == crate::creatures::body::legs::View::Side);
        let turn = if d.def.legs.is_some() { 0.0 } else { std::f32::consts::FRAC_PI_2 * d.facing };
        let rot = Quat::from_rotation_z(turn);
        let size = d.body.half * 2.0;
        let size = if turn == 0.0 { size } else { Vec2::new(size.y, size.x) };
        let (at, size, lowered) = if side {
            let h = BELLY.min(size.y);
            (Vec2::new(at.x, at.y - d.body.half.y + h / 2.0), Vec2::new(size.x, h), d.body.half.y - h / 2.0)
        } else {
            (at, size, 0.0)
        };
        let mut body = Body::new(at, size.max(Vec2::splat(3.0)));
        // (Thrown on as it was going when it died: a ragdoll.)
        body.vel = d.body.vel;
        let mut e = commands.spawn((
            Name::new(format!("Body of {}", d.def.name)),
            Corpse { key, empty: 0.0, born: { *count += 1; *count } },
            Container { key, name: if d.rising.is_some() { format!("{} (bones)", d.def.name) } else { format!("{} (dead)", d.def.name) } },
            Thrown { bounce: RAG_BOUNCE, slide: RAG_SLIDE },
            // (Turned as it was as it died, from how it'll lie: it topples
            // over, or spins on as it flew.)
            Ragdoll { angle: crate::creatures::body::tumble::wrap(d.spin.angle - turn), omega: d.spin.omega, last_vy: d.body.vel.y, lie: 0.0, sink: 0.0 },
            Kinematics { body, loco: Locomotion::default(), prev_pos: at },
            Transform::from_translation(at.extend(Z)).with_rotation(Quat::from_rotation_z(d.spin.angle - turn)),
            Visibility::default(),
        ));
        if let Some(rise) = &d.rising {
            crate::creatures::body::rise::make_pile(&mut e, d, rise, sim.world.materials().id(&d.def.blood));
        }
        if let Some((sprite, offset)) = &d.sprite {
            let mut sprite = sprite.clone();
            sprite.color = Color::srgb(DIM, DIM, DIM);
            let flip = sprite.flip_x;
            let offset = *offset + Vec3::Y * lowered;
            e.with_child((CorpseSprite, sprite, Transform::from_translation((rot * offset.with_z(0.0)).with_z(0.0)).with_rotation(rot)));
            // Legs: kept, curled in over the body (folded under it, seen
            // from the side).
            if let Some(legs) = &d.def.legs {
                let (image, centre) = if side { folded_legs(legs, at.x.to_bits() as u64 ^ *count) } else { curled_legs(legs, at.x.to_bits() as u64 ^ *count) };
                let mut s = Sprite::from_image(images.add(image));
                s.flip_x = flip;
                s.color = Color::srgb(DIM, DIM, DIM);
                let centre = if flip { Vec2::new(-centre.x, centre.y) } else { centre };
                e.with_child((CorpseSprite, s, Transform::from_translation((offset.truncate() + centre).extend(0.01))));
            }
        }
    }
}

/// A body thrown about as it died: it bounces (`RAG_BOUNCE`) and slides
/// (`RAG_SLIDE`) further than a dropped thing, spins as it flies, and once
/// down rolls over to lie as it lies (`SETTLE`); landing hard it thuds and
/// kicks up dust, and its slide turns it.
#[derive(Component)]
pub struct Ragdoll {
    /// Turned from how it'll lie (radians), and how fast it's turning.
    angle: f32,
    omega: f32,
    /// Its fall last frame (cells/s), to tell a landing.
    last_vy: f32,
    /// The ground it rests on: its slope's angle (it lies along it), and
    /// how far its box stands off the slope's middle (drawn down onto it:
    /// a box rests on its uphill corner).
    lie: f32,
    sink: f32,
}

const RAG_BOUNCE: f32 = 0.2;
const RAG_SLIDE: f32 = 0.88;
/// Down, it rolls over to lie this fast (per second).
const SETTLE: f32 = 7.0;
/// Falling faster than this (cells/s) and stopped: a landing.
const THUD: f32 = 90.0;
/// Spin a slide gives it (radians/s per cell/s).
const ROLL: f32 = 0.02;

/// A body at rest on a slope: it lies along it, drawn down onto it, and
/// on a steep one slides down (picking up speed to `SLIDE_MOST`, stopping
/// where it levels out). The ground's height either side of it, as found
/// within a few cells of its bottom.
fn slump(sim: Res<SimWorld>, mut q: Query<(&mut Kinematics, &mut Ragdoll)>) {
    let grid = crate::creatures::WorldGrid(&sim.world);
    let world = &sim.world;
    let ground = |x: f32, from: f32| -> Option<f32> {
        let x = x.floor() as i32;
        let top = from as i32 + SLOPE_LOOK;
        (top - SLOPE_DOWN..=top).rev().find(|&y| world.is_solid(platypus_sim::CellPos::new(x, y)) && !world.is_solid(platypus_sim::CellPos::new(x, y + 1))).map(|y| y as f32 + 1.0)
    };
    for (mut k, mut r) in &mut q {
        // (At rest, or sliding; falling, it's `props::fly`'s.)
        let resting = platypus_physics::resting(&grid, &k.body);
        if !resting && k.body.vel.y < -40.0 {
            continue;
        }
        let (hx, bottom) = (k.body.half.x * 0.9, k.body.pos.y - k.body.half.y);
        let (Some(left), Some(right)) = (ground(k.body.pos.x - hx, bottom), ground(k.body.pos.x + hx, bottom)) else { continue };
        let slope = (right - left) / (2.0 * hx);
        r.lie = slope.atan().clamp(-LIE_MOST, LIE_MOST);
        r.sink = ((bottom - (left + right) / 2.0) * 0.8).max(0.0);
        if slope.abs() > SLIDE_FROM {
            let k = &mut *k;
            let downhill = -slope.signum();
            k.body.vel.x = (k.body.vel.x + downhill * SLIDE_ACCEL * DT).clamp(-SLIDE_MOST, SLIDE_MOST);
            // (At rest `fly` leaves it be: moved here.)
            if resting {
                k.prev_pos = k.body.pos;
                k.body.vel.y -= 900.0 * DT;
                platypus_physics::move_and_collide(&grid, &mut k.body, DT);
            }
        } else if resting && k.body.vel.x != 0.0 {
            k.body.vel.x = 0.0;
        }
    }
}

/// Slopes for a body at rest: the ground looked for this many cells over
/// its bottom and so many under (a box perched on its uphill corner has
/// the downhill ground far below it), lain along up to this angle, slid
/// down past this slope, speeding up so much a second, to so fast.
const SLOPE_LOOK: i32 = 6;
const SLOPE_DOWN: i32 = 66;
const LIE_MOST: f32 = 1.0;
const SLIDE_FROM: f32 = 0.6;
const SLIDE_ACCEL: f32 = 600.0;
const SLIDE_MOST: f32 = 110.0;

fn ragdoll(time: Res<Time>, mut sim: ResMut<SimWorld>, mut sounds: MessageWriter<crate::sound::PlaySound>, mut q: Query<(&Kinematics, &mut Ragdoll, &mut Transform)>) {
    let dt = time.delta_secs();
    for (k, mut r, mut tf) in &mut q {
        let vy = k.body.vel.y;
        let resting = platypus_physics::resting(&crate::creatures::WorldGrid(&sim.world), &k.body);
        if resting && r.angle == r.lie && r.omega == 0.0 {
            tf.rotation = Quat::from_rotation_z(r.angle);
            tf.translation.y -= r.sink;
            continue;
        }
        // Landed hard: a thud, dust, its spin knocked about.
        if r.last_vy < -THUD && vy > r.last_vy * 0.5 {
            let feet = [k.body.pos.x, k.body.pos.y - k.body.half.y];
            if let Some(cell) = sim.world.get(platypus_sim::CellPos::from_world(feet[0], feet[1] - 1.0))
                && !cell.is_air()
            {
                let n = ((-r.last_vy - THUD) / 40.0).clamp(3.0, 14.0) as usize;
                sim.world.puff(feet, cell, n, 60.0);
            }
            let mut thud = crate::sound::PlaySound::at("land", k.body.pos);
            thud.volume = ((-r.last_vy) / 400.0).clamp(0.3, 1.0);
            sounds.write(thud);
            r.omega = r.omega * 0.5 - k.body.vel.x * ROLL;
        }
        r.last_vy = vy;
        if !resting && vy.abs() > 2.0 {
            r.angle += r.omega * dt;
        } else {
            // (Down: rolling over to lie along the ground it's on.)
            r.omega = 0.0;
            let off = crate::creatures::body::tumble::wrap(r.angle - r.lie);
            r.angle = r.lie + off * (-dt * SETTLE).exp();
            if off.abs() < 0.005 {
                r.angle = r.lie;
            }
        }
        tf.rotation = Quat::from_rotation_z(r.angle);
        tf.translation.y -= r.sink;
    }
}

/// Blasts throw bodies about.
fn batter(mut blasts: MessageReader<Explosion>, mut q: Query<&mut Kinematics, With<Corpse>>) {
    for b in blasts.read() {
        for mut k in &mut q {
            let reach = b.radius * 1.6 + 9.0;
            let d = k.body.pos.distance(b.at);
            if d < reach {
                let away = (k.body.pos - b.at).normalize_or(Vec2::Y);
                k.body.vel += (away + Vec2::new(0.0, 0.6)) * b.power * 4.5 * (1.0 - d / reach);
            }
        }
    }
}

/// Empty bodies fade and go; past `MAX_BODIES`, the oldest go.
fn rot(mut commands: Commands, mut chests: ResMut<Chests>, mut q: Query<(Entity, &mut Corpse, Option<&Children>)>, mut sprites: Query<&mut Sprite, With<CorpseSprite>>) {
    let n = q.iter().count();
    if n > MAX_BODIES {
        let mut ages: Vec<(u64, Entity, u64)> = q.iter().map(|(e, c, _)| (c.born, e, c.key)).collect();
        ages.sort_unstable();
        for &(_, e, key) in &ages[..n - MAX_BODIES] {
            chests.forget(key);
            commands.entity(e).despawn();
        }
        // (The rest fade next tick.)
        return;
    }
    for (e, mut c, children) in &mut q {
        if !chests.is_empty(c.key) {
            c.empty = 0.0;
            continue;
        }
        c.empty += DT;
        let fade = ((c.empty - LINGER) / FADE).clamp(0.0, 1.0);
        for child in children.into_iter().flat_map(|c| c.iter()) {
            if let Ok(mut s) = sprites.get_mut(child) {
                s.color = Color::srgba(DIM, DIM, DIM, 1.0 - fade);
            }
        }
        if fade >= 1.0 {
            chests.forget(c.key);
            commands.entity(e).despawn();
        }
    }
}
