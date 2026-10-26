//! Wounds (`wounds` stage 2): limbs that can be cut off. Each limb a blow
//! lands on (a leg, an arm, a tail: the part it struck, `Parts`) keeps what
//! it's taken; past its share of the creature's health it comes off. The
//! piece flies, a ragdoll of its own (`ragdoll::cut_off`: drawn as it was,
//! knee and all), bleeding from where it was cut, and lies a while before
//! it fades; the stump bleeds too, less and less. What's lost is lost:
//! a cut leg holds nothing and steps nowhere (`legs`), and the creature
//! goes slower for it (a biped on one leg hops, on none it drags itself);
//! a wall-walker with fewer than half its legs can't hold on and falls.

use bevy::prelude::*;
use platypus_physics::Body;

use super::parts::{Limb, Part, Parts, Severed};
use super::ragdoll::{self, Limp, LimpPiece};
use crate::creatures::{Health, Kinematics, MoveStats};
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// A creature's legs share this much of its health between them, each at
/// least `LEG_LEAST` and at most `LEG_MOST` of it; an arm, a chain, a head
/// their own share.
const LEGS_SHARE: f32 = 1.2;
const LEG_LEAST: f32 = 0.12;
const LEG_MOST: f32 = 0.25;
const ARM_SHARE: f32 = 0.3;
const CHAIN_SHARE: f32 = 0.35;
const HEAD_SHARE: f32 = 0.3;
/// A cut piece flies off this fast (cells a second): away along the blow,
/// and up.
const THROW: f32 = 80.0;
const THROW_UP: f32 = 55.0;
const THROW_SPIN: f32 = 9.0;
/// Blood at the cut, all at once: a spray.
const CUT_SPRAY: usize = 14;
/// How long a stump and a piece bleed (seconds), and how much at first
/// (cells a second, tapering).
const STUMP_BLEEDS: f32 = 3.5;
const PIECE_BLEEDS: f32 = 2.5;
const BLEED_RATE: f32 = 26.0;
/// A loose skull's health (only what breaks its bones hurts it).
const SKULL_HEALTH: f32 = 10.0;
/// What breaks a skull's bones does this much more to it than to its kind.
const SKULL_WEAK: f32 = 1.6;
/// A cut piece lies this long, then fades over `PIECE_FADE`.
const PIECE_LIFE: f32 = 30.0;
pub const PIECE_FADE: f32 = 3.0;
/// A biped's speed on one leg and on none, and its jump; anything with
/// more legs keeps this much at least, the rest by the share it has left.
const ONE_LEG: f32 = 0.45;
const NO_LEGS: f32 = 0.15;
const HOP: f32 = 0.45;
const MANY_LEAST: f32 = 0.3;

/// A body drawn by its own art on legs that only add legs (`legs::rig`),
/// with no legs left: drawn this much lower (its pelvis on the ground),
/// in a box a crawler's height.
#[derive(Component, Clone, Copy, Debug)]
pub struct Legless(pub f32);

/// The least a legless body's box is (cells high).
const CRAWL_LEAST: f32 = 8.0;

/// What each limb's taken.
#[derive(Component, Clone, Debug, Default)]
pub struct Wounds(Vec<(Limb, f32)>);

/// A limb comes off: `dir` the way the blow went.
#[derive(Message, Clone, Copy, Debug)]
pub struct Cut {
    pub target: Entity,
    pub limb: Limb,
    pub dir: Vec2,
}

/// A piece that came off: how long it has left.
#[derive(Component)]
pub struct CutPiece {
    pub left: f32,
}

/// Where something bleeds: a point on one of its ragdoll's pieces (that
/// piece's own frame), or on its body (from its middle, facing right).
#[derive(Clone, Copy, Debug)]
enum Spot {
    Piece(usize, Vec2),
    Body(Vec2),
}

/// Bleeding: each spot, how long it has left of how long.
#[derive(Component, Clone, Debug)]
pub struct Bleeding {
    blood: platypus_sim::MaterialId,
    spots: Vec<(Spot, f32, f32)>,
    clock: f32,
}

/// How much a limb takes before it comes off (a head, its kind's `neck`
/// if it says).
fn limb_max(limb: Limb, health: f32, def: &crate::creatures::def::CreatureDef) -> f32 {
    let legs = def.legs.as_ref();
    let share = match limb {
        Limb::Leg(_) => {
            let n = legs.map_or(2, |l| if l.view == super::legs::View::Side { l.each.len() } else { l.count }).max(1);
            (LEGS_SHARE / n as f32).clamp(LEG_LEAST, LEG_MOST)
        }
        Limb::Arm(_) => ARM_SHARE,
        Limb::Chain(_) => CHAIN_SHARE,
        Limb::Head => def.neck.unwrap_or(HEAD_SHARE),
    };
    health * share
}

/// Each drawn frame's layers (a humanoid rig's: its head, its arms), cut
/// once and kept: which a blow on its picture lands on.
#[derive(Resource, Default)]
pub struct LayerMasks(std::collections::HashMap<(usize, String), std::sync::Arc<Vec<ragdoll::Cut>>>);

impl LayerMasks {
    pub fn cuts(&mut self, file: &std::sync::Arc<platypus_art::ArtFile>, frame: &str) -> std::sync::Arc<Vec<ragdoll::Cut>> {
        let key = (std::sync::Arc::as_ptr(file) as usize, frame.to_string());
        self.0.entry(key).or_insert_with(|| std::sync::Arc::new(ragdoll::cut(file, frame))).clone()
    }
}

/// The limb of a body drawn in layers (a humanoid rig) a blow at `at`
/// lands on: the frontmost layer there that's a limb (its head, an arm).
pub(crate) fn rig_limb(anim: &super::animation::Animator, k: &Kinematics, at: Vec2, masks: &mut LayerMasks) -> Option<Limb> {
    let file = anim.def.art_file.as_ref().filter(|f| !f.poses.is_empty())?;
    let frame = anim.def.rig.as_ref()?.names.get(anim.shown)?.clone();
    let cuts = masks.cuts(file, &frame);
    let (x, y) = super::animation::pixel_at(&anim.def, k, at);
    let p = IVec2::new(x, y);
    // (Frontmost first; a pixel's slack round it: blades are thin.)
    let near = |c: &ragdoll::Cut| (-1..=1).any(|dy| (-1..=1).any(|dx| c.covers(p + IVec2::new(dx, dy))));
    cuts.iter().rev().find(|c| c.covers(p)).or_else(|| cuts.iter().rev().find(|c| near(c))).and_then(|c| c.limb())
}

type Struck<'a> = (&'a Health, &'a Kinematics, Option<&'a Parts>, Option<&'a Severed>, Option<&'a mut Wounds>, &'a super::animation::Animator);

/// Blows that land on a limb wear it down; past its share, it's cut off
/// (a head with `behead`, only now and then: a roll each blow).
/// (A limb of its parts, `Parts`: a leg, an arm, a tail; or of its drawn
/// layers: a humanoid's head and arms.)
/// (Not the player's: your limbs don't come off, and your head off wasn't
/// a death you could see coming.)
pub fn wound(mut commands: Commands, mut felt: MessageReader<crate::combat::Felt>, mut q: Query<Struck, Without<crate::creatures::player::LocalPlayer>>, mut cuts: MessageWriter<Cut>, mut masks: ResMut<LayerMasks>) {
    for f in felt.read() {
        if f.dealt <= 0.0 {
            continue;
        }
        let Ok((health, k, parts, severed, wounds, anim)) = q.get_mut(f.target) else { continue };
        let Some(limb) = parts.and_then(|p| p.touch(f.at, 1.0)).and_then(|p| p.limb).or_else(|| rig_limb(anim, k, f.at, &mut masks)) else { continue };
        if severed.is_some_and(|s| s.has(limb)) {
            continue;
        }
        let max = limb_max(limb, health.max, &anim.def);
        let took = match wounds {
            Some(mut w) => match w.0.iter_mut().find(|(l, _)| *l == limb) {
                Some((_, t)) => {
                    *t += f.dealt;
                    *t
                }
                None => {
                    w.0.push((limb, f.dealt));
                    f.dealt
                }
            },
            None => {
                commands.entity(f.target).insert(Wounds(vec![(limb, f.dealt)]));
                f.dealt
            }
        };
        // (Its head maybe not, even then: `behead`, a roll each blow.)
        let comes_off = match (limb, anim.def.behead) {
            (Limb::Head, Some(chance)) => (platypus_sim::rng::hash(&[f.target.to_bits(), took.to_bits() as u64, 0xBE4D]) % 10_000) as f32 / 10_000.0 < chance,
            _ => true,
        };
        if took >= max && comes_off {
            cuts.write(Cut { target: f.target, limb, dir: f.dir });
        }
    }
}

/// What losing its head does (a creature file's `headless`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub enum Headless {
    #[default]
    Dies,
    /// It fights on blind (it hears): bones.
    Blind,
}

/// Lost its head and lives on (bones): it sees nothing (it hears).
#[derive(Component)]
pub struct Blind;

type Cutting<'a> = (&'a mut Kinematics, Option<&'a Parts>, &'a super::animation::Animator, Option<&'a mut Severed>, Option<&'a super::hurt::Bleeds>, &'a mut MoveStats, Option<&'a mut Bleeding>);
type Losing<'a> = (&'a mut Health, Option<&'a mut crate::gear::Equipment>, Option<&'a mut crate::combat::Wielding>, Option<&'a super::rise::Rises>, (Option<&'a super::rise::Lineage>, Option<&'a crate::creatures::Stable>));

/// A limb comes off: the piece flies, bleeding; the stump bleeds; the
/// creature has lost it (legs: slower, maybe down on its pelvis; its
/// weapon arm: its weapon; its head: its life, or for bones its sight).
#[allow(clippy::too_many_arguments)]
pub fn sever(
    mut commands: Commands,
    mut cuts: MessageReader<Cut>,
    mut sim: ResMut<SimWorld>,
    mut images: ResMut<Assets<Image>>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    items: Option<Res<crate::hands::items::Items>>,
    mut q: Query<Cutting>,
    mut losing: Query<Losing>,
) {
    for c in cuts.read() {
        let Ok((mut k, parts, anim, severed, bleeds, mut moves, bleeding)) = q.get_mut(c.target) else { continue };
        if severed.as_ref().is_some_and(|s| s.has(c.limb)) {
            continue;
        }
        let limbs: Vec<Part> = parts.map(|p| p.list.iter().filter(|p| p.limb == Some(c.limb)).cloned().collect()).unwrap_or_default();
        let facing = k.loco.facing;
        // The piece: its capsules (a walker's), or its drawn layer (a
        // humanoid's head or arm, in what it wears), and where it was cut.
        let spin = if c.dir.x < 0.0 { THROW_SPIN } else { -THROW_SPIN };
        let lift = if c.limb == Limb::Head { THROW_UP * 1.5 } else { THROW_UP };
        let vel = k.body.vel + c.dir.normalize_or(Vec2::X) * THROW + Vec2::Y * lift;
        let built = match (anim.def.legs.as_ref(), limbs.is_empty()) {
            (Some(legs), false) => {
                let root = limbs[0].a;
                let (limp, looks) = ragdoll::cut_off(&limbs, crate::hands::corpses::limb_drawer(legs), vel, spin);
                Some((limp, looks, root))
            }
            _ => {
                let worn: Vec<crate::hands::items::Stack> = losing.get(c.target).ok().and_then(|l| l.1.map(|eq| eq.worn.iter().flatten().copied().collect())).unwrap_or_default();
                rig_piece(anim, &k, c.limb, items.as_deref(), &worn, vel, spin)
            }
        };
        let Some((limp, looks, root)) = built else { continue };
        // Lost.
        let mut lost = severed.map(|s| s.0.clone()).unwrap_or_default();
        lost.push(c.limb);
        commands.entity(c.target).insert(Severed(lost.clone()));
        if let Some(legs) = anim.def.legs.as_ref() {
            // (At once; and from what it wears too, as its gear is next
            // worked out: `gear::apply`.)
            moves.0 = crippled(&anim.def.movement, legs, &lost);
            if let Ok((_, Some(mut eq), ..)) = losing.get_mut(c.target) {
                eq.set_changed();
            }
            // Its own art's body with no legs left: down on its pelvis, a
            // crawler's box (its bottom where it was).
            let total = if legs.view == super::legs::View::Side { legs.each.len() } else { legs.count };
            if legs.rig && matches!(c.limb, Limb::Leg(_)) && lost.iter().filter(|l| matches!(l, Limb::Leg(_))).count() >= total {
                let hip = legs.each.iter().map(|l| l.hip.1).fold(f32::MIN, f32::max);
                let sunk = anim.def.size.1 / 2.0 + hip;
                let half = ((anim.def.size.1 - sunk).max(CRAWL_LEAST)) / 2.0;
                let drop = k.body.half.y - half;
                k.body.half.y = half;
                k.body.pos.y -= drop;
                k.prev_pos.y -= drop;
                commands.entity(c.target).insert(Legless(sunk));
            }
        }
        let mut bone = None;
        if let Ok((mut health, eq, wielding, rises, (lineage, stable))) = losing.get_mut(c.target) {
            // (One that rises: the piece is one of its bones; its head, if
            // that's what holds it together, its skull.)
            if let Some(r) = rises.filter(|r| r.0.remains == super::rise::Remains::Rubble) {
                bone = Some((super::rise::Bone { lineage: crate::creatures::lineage_of(Some((lineage, stable))), limb: c.limb }, (r.0.anchor && c.limb == Limb::Head).then(|| r.0.breaks.clone())));
            }
            match c.limb {
                // Its weapon hand: what it held falls; it fights with what
                // it has left (the other fist, or nothing).
                Limb::Arm(0) if anim.def.legs.as_ref().is_none_or(|l| l.rig) => {
                    if let (Some(mut eq), Some(items)) = (eq, items.as_deref())
                        && let Some(held) = eq.held.take()
                    {
                        crate::hands::spawn_drop(&mut commands, items, root, held);
                    }
                    if let Some(mut w) = wielding {
                        w.0 = if lost.contains(&Limb::Arm(1)) { anim.def.armless.clone() } else { anim.def.unarmed.clone() };
                    }
                }
                Limb::Arm(1) if lost.contains(&Limb::Arm(0)) => {
                    if let Some(mut w) = wielding {
                        w.0 = anim.def.armless.clone();
                    }
                }
                // Its head: dead; bones fight on blind.
                Limb::Head if anim.def.headless == Headless::Blind => {
                    commands.entity(c.target).insert(Blind);
                }
                Limb::Head => health.hp = health.hp.min(0.0),
                _ => {}
            }
        }
        // The piece, flying off along the blow.
        let first = &limp.pieces[0];
        let (s, co) = (-first.angle).sin_cos();
        let d = root - first.pos;
        let cut_end = Vec2::new(d.x * co - d.y * s, d.x * s + d.y * co);
        let at = limp.pieces[limp.root].pos;
        let mut e = commands.spawn((Name::new("Cut off"), CutPiece { left: PIECE_LIFE }, Kinematics { body: Body::new(at, Vec2::splat(3.0)), loco: Default::default(), prev_pos: at }, Transform::from_translation(at.extend(5.6)), Visibility::default()));
        // A bone of one that rises lies till it's gathered up (or its owner's
        // gone for good); a skull can be struck: anything hurts it as it
        // hurts its kind (fire, light, a blade), what breaks its bones more.
        if let Some((b, skull)) = bone {
            e.insert((b, CutPiece { left: f32::INFINITY }));
            if let Some(breaks) = skull {
                // (Its box big enough to strike.)
                e.insert((
                    Kinematics { body: Body::new(at, Vec2::splat(6.0)), loco: Default::default(), prev_pos: at },
                    Name::new("Skull"),
                    super::rise::Skull,
                    Health { nature: anim.def.nature.worse(&breaks, SKULL_WEAK), ..Health::new(SKULL_HEALTH) },
                    MoveStats(anim.def.movement.clone()),
                    anim.def.team,
                ));
            }
        }
        for (i, look) in looks.into_iter().enumerate() {
            let s = match look {
                ragdoll::Look::Image(image, flip) => Sprite { flip_x: flip, ..Sprite::from_image(images.add(image)) },
                ragdoll::Look::Sprite(s) => s,
            };
            e.with_child((LimpPiece(i), s, Transform::from_xyz(0.0, 0.0, i as f32 * 0.001)));
        }
        e.insert(limp);
        // Blood: a spray at the cut; the piece bleeds from its cut end, the
        // stump from where it was.
        if let Some(&super::hurt::Bleeds(blood, much)) = bleeds {
            sim.world.splash([root.x, root.y], blood, (CUT_SPRAY as f32 * much) as usize, 2.6);
            e.insert(Bleeding { blood, spots: vec![(Spot::Piece(0, cut_end), PIECE_BLEEDS, PIECE_BLEEDS)], clock: 0.0 });
            let off = root - k.body.pos;
            let stump = (Spot::Body(Vec2::new(off.x * facing, off.y)), STUMP_BLEEDS, STUMP_BLEEDS);
            match bleeding {
                Some(mut b) => b.spots.push(stump),
                None => {
                    commands.entity(c.target).insert(Bleeding { blood, spots: vec![stump], clock: 0.0 });
                }
            }
        }
        sounds.write(crate::sound::PlaySound::at("snap", root).pitch(if c.limb == Limb::Head { 0.6 } else { 0.8 }));
    }
}

/// A humanoid's head or arm cut off: its layer of the frame it's in, drawn
/// in what it wears, a piece of its own; and where it was cut (the world).
fn rig_piece(anim: &super::animation::Animator, k: &Kinematics, limb: Limb, items: Option<&crate::hands::items::Items>, worn: &[crate::hands::items::Stack], vel: Vec2, spin: f32) -> Option<(Limp, Vec<ragdoll::Look>, Vec2)> {
    let file = anim.def.art_file.as_ref().filter(|f| !f.poses.is_empty())?;
    let frame = anim.def.rig.as_ref()?.names.get(anim.shown)?.clone();
    let skins = items.map(|i| crate::gear::look::skins_of(i, worn)).unwrap_or_default();
    let dressed = if skins.is_empty() { (**file).clone() } else { platypus_art::dress::dress(file, &skins).ok()? };
    let mut cuts = ragdoll::cut(&dressed, &frame);
    let i = cuts.iter().position(|c| c.limb() == Some(limb))?;
    let piece = cuts.swap_remove(i);
    let frame_size = anim.def.sprite.frame;
    let centre = k.body.pos + super::animation::body_at(&anim.def, k);
    let joint = piece.joint(frame_size);
    let (fw, fh) = (frame_size.0 as f32, frame_size.1 as f32);
    let root = centre + Vec2::new((joint.x - fw / 2.0) * k.loco.facing, fh / 2.0 - joint.y);
    let mut limp = ragdoll::assemble(std::slice::from_ref(&piece), frame_size, centre, k.loco.facing, vel, spin);
    limp.pieces[0].pixels = piece.pixels_local(k.loco.facing);
    Some((limp, vec![ragdoll::Look::Image(piece.image, k.loco.facing < 0.0)], root))
}

/// How it moves with what it's lost: its legs (a biped hops on one and
/// drags itself on none; more legs, slower by the share gone; a
/// wall-walker holds on only with half or more).
pub fn crippled(base: &platypus_physics::MovementStats, legs: &super::legs::LegsDef, lost: &[Limb]) -> platypus_physics::MovementStats {
    let total = if legs.view == super::legs::View::Side { legs.each.len() } else { legs.count };
    let gone = lost.iter().filter(|l| matches!(l, Limb::Leg(_))).count().min(total);
    let left = total - gone;
    let mut m = base.clone();
    if total == 0 || gone == 0 {
        return m;
    }
    let (speed, jump) = if total <= 2 {
        match left {
            0 => (NO_LEGS, 0.0),
            _ => (ONE_LEG, HOP),
        }
    } else {
        let share = left as f32 / total as f32;
        (MANY_LEAST + (1.0 - MANY_LEAST) * share, share.max(0.3))
    };
    m.run_speed *= speed;
    m.jump_height *= jump;
    m.dash_speed *= speed;
    m.cling = base.cling && left * 2 >= total;
    m
}

/// A legless body is drawn down on its pelvis: its art's feet raised by
/// what its legs held it up (kept so through a change of look).
pub fn sit(mut q: Query<(&Legless, &mut super::animation::Animator)>) {
    for (l, mut anim) in &mut q {
        let base = anim.def.art_file.as_ref().map(|f| f.feet.1).unwrap_or(anim.def.sprite.feet.1);
        let want = base - l.0;
        if (anim.def.sprite.feet.1 - want).abs() > 0.01 {
            let mut def = (*anim.def).clone();
            def.sprite.feet.1 = want;
            anim.def = std::sync::Arc::new(def);
            anim.refresh();
        }
    }
}

/// Blood from each bleeding spot, less and less, till it stops.
pub fn bleed(mut commands: Commands, mut sim: ResMut<SimWorld>, mut q: Query<(Entity, &mut Bleeding, &Kinematics, Option<&Limp>)>) {
    for (e, mut b, k, limp) in &mut q {
        b.clock += DT;
        let blood = b.blood;
        let mut out = Vec::new();
        for (spot, left, of) in &mut b.spots {
            *left -= DT;
            let at = match *spot {
                Spot::Piece(i, local) => limp.and_then(|l| l.pieces.get(i)).map(|p| {
                    let (s, c) = p.angle.sin_cos();
                    p.pos + Vec2::new(local.x * c - local.y * s, local.x * s + local.y * c)
                }),
                Spot::Body(off) => Some(k.body.pos + Vec2::new(off.x * k.loco.facing, off.y)),
            };
            // (A share of a cell a tick, tapering: a drop now and then.)
            let rate = BLEED_RATE * (*left / *of).max(0.0);
            if let Some(at) = at {
                out.push((at, rate));
            }
        }
        let clock = b.clock;
        for (at, rate) in out {
            let every = (TICK_HZ as f32 / rate.max(0.01)).max(1.0);
            if (clock * TICK_HZ as f32) % every < 1.0 {
                sim.world.splash([at.x, at.y], blood, 1, 0.8);
            }
        }
        b.spots.retain(|s| s.1 > 0.0);
        if b.spots.is_empty() {
            commands.entity(e).remove::<Bleeding>();
        }
    }
}

/// Cut pieces lie a while, then fade and go.
pub fn fade(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut CutPiece, &Children)>, mut sprites: Query<&mut Sprite, With<LimpPiece>>) {
    let dt = time.delta_secs();
    for (e, mut p, children) in &mut q {
        p.left -= dt;
        if p.left <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        if p.left < PIECE_FADE {
            for &c in children {
                if let Ok(mut s) = sprites.get_mut(c) {
                    s.color.set_alpha(p.left / PIECE_FADE);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walker(legs: usize, cling: bool) -> (platypus_physics::MovementStats, super::super::legs::LegsDef) {
        let mut m = platypus_physics::MovementStats::default();
        m.run_speed = 100.0;
        m.jump_height = 40.0;
        m.cling = cling;
        let text = format!("(view: Above, count: {legs}, reach: 10, color: (1, 1, 1), body: \"x\")");
        (m, crate::data::parse_ron(&text).unwrap())
    }

    #[test]
    fn a_wall_walker_with_half_its_legs_gone_lets_go() {
        let (m, legs) = walker(8, true);
        assert!(crippled(&m, &legs, &[Limb::Leg(0), Limb::Leg(1), Limb::Leg(2)]).cling, "five of eight: it holds on");
        let lost: Vec<Limb> = (0..5).map(Limb::Leg).collect();
        let c = crippled(&m, &legs, &lost);
        assert!(!c.cling, "three of eight: it falls");
        assert!(c.run_speed < 100.0 && c.run_speed >= 100.0 * MANY_LEAST);
    }

    #[test]
    fn a_biped_hops_on_one_leg_and_drags_itself_on_none() {
        let (m, legs) = walker(2, false);
        let one = crippled(&m, &legs, &[Limb::Leg(0)]);
        assert_eq!(one.run_speed, 100.0 * ONE_LEG);
        let none = crippled(&m, &legs, &[Limb::Leg(0), Limb::Leg(1)]);
        assert_eq!(none.run_speed, 100.0 * NO_LEGS);
        assert_eq!(none.jump_height, 0.0);
    }
}
