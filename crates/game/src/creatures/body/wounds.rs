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
const LEG_MOST: f32 = 0.4;
const ARM_SHARE: f32 = 0.3;
const CHAIN_SHARE: f32 = 0.35;
const HEAD_SHARE: f32 = 0.4;
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
/// A cut piece lies this long, then fades over `PIECE_FADE`.
const PIECE_LIFE: f32 = 30.0;
const PIECE_FADE: f32 = 3.0;
/// A biped's speed on one leg and on none, and its jump; anything with
/// more legs keeps this much at least, the rest by the share it has left.
const ONE_LEG: f32 = 0.45;
const NO_LEGS: f32 = 0.15;
const HOP: f32 = 0.45;
const MANY_LEAST: f32 = 0.3;

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
    left: f32,
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

/// How much a limb takes before it comes off.
fn limb_max(limb: Limb, health: f32, legs: Option<&super::legs::LegsDef>) -> f32 {
    let share = match limb {
        Limb::Leg(_) => {
            let n = legs.map_or(2, |l| if l.view == super::legs::View::Side { l.each.len() } else { l.count }).max(1);
            (LEGS_SHARE / n as f32).clamp(LEG_LEAST, LEG_MOST)
        }
        Limb::Arm(_) => ARM_SHARE,
        Limb::Chain(_) => CHAIN_SHARE,
        Limb::Head => HEAD_SHARE,
    };
    health * share
}

type Struck<'a> = (&'a Health, &'a Parts, Option<&'a Severed>, Option<&'a mut Wounds>, &'a super::animation::Animator);

/// Blows that land on a limb wear it down; past its share, it's cut off.
pub fn wound(mut commands: Commands, mut felt: MessageReader<crate::combat::Felt>, mut q: Query<Struck>, mut cuts: MessageWriter<Cut>) {
    for f in felt.read() {
        if f.dealt <= 0.0 {
            continue;
        }
        let Ok((health, parts, severed, wounds, anim)) = q.get_mut(f.target) else { continue };
        let Some(limb) = parts.touch(f.at, 1.0).and_then(|p| p.limb) else { continue };
        if severed.is_some_and(|s| s.has(limb)) {
            continue;
        }
        let max = limb_max(limb, health.max, anim.def.legs.as_ref());
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
        if took >= max {
            cuts.write(Cut { target: f.target, limb, dir: f.dir });
        }
    }
}

type Cutting<'a> = (&'a Kinematics, &'a Parts, &'a super::animation::Animator, Option<&'a mut Severed>, Option<&'a super::hurt::Bleeds>, &'a mut MoveStats, Option<&'a mut Bleeding>);

/// A limb comes off: the piece flies, bleeding; the stump bleeds; the
/// creature has lost it.
#[allow(clippy::too_many_arguments)]
pub fn sever(
    mut commands: Commands,
    mut cuts: MessageReader<Cut>,
    mut sim: ResMut<SimWorld>,
    mut images: ResMut<Assets<Image>>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut q: Query<Cutting>,
) {
    for c in cuts.read() {
        let Ok((k, parts, anim, severed, bleeds, mut moves, bleeding)) = q.get_mut(c.target) else { continue };
        if severed.as_ref().is_some_and(|s| s.has(c.limb)) {
            continue;
        }
        let limbs: Vec<Part> = parts.list.iter().filter(|p| p.limb == Some(c.limb)).cloned().collect();
        let Some(legs) = anim.def.legs.as_ref() else { continue };
        if limbs.is_empty() {
            continue;
        }
        // Lost.
        let mut lost = severed.map(|s| s.0.clone()).unwrap_or_default();
        lost.push(c.limb);
        commands.entity(c.target).insert(Severed(lost.clone()));
        moves.0 = crippled(&anim.def.movement, legs, &lost);
        // The piece, flying off along the blow.
        let root = limbs[0].a;
        let spin = if c.dir.x < 0.0 { THROW_SPIN } else { -THROW_SPIN };
        let vel = k.body.vel + c.dir.normalize_or(Vec2::X) * THROW + Vec2::Y * THROW_UP;
        let (limp, looks) = ragdoll::cut_off(&limbs, crate::hands::corpses::limb_drawer(legs), vel, spin);
        let first = &limp.pieces[0];
        let (s, co) = (-first.angle).sin_cos();
        let d = root - first.pos;
        let cut_end = Vec2::new(d.x * co - d.y * s, d.x * s + d.y * co);
        let at = limp.pieces[limp.root].pos;
        let mut e = commands.spawn((Name::new("Cut off"), CutPiece { left: PIECE_LIFE }, Kinematics { body: Body::new(at, Vec2::splat(3.0)), loco: Default::default(), prev_pos: at }, Transform::from_translation(at.extend(5.6)), Visibility::default()));
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
        if let Some(&super::hurt::Bleeds(blood)) = bleeds {
            sim.world.splash([root.x, root.y], blood, CUT_SPRAY, 2.6);
            e.insert(Bleeding { blood, spots: vec![(Spot::Piece(0, cut_end), PIECE_BLEEDS, PIECE_BLEEDS)], clock: 0.0 });
            let off = root - k.body.pos;
            let stump = (Spot::Body(Vec2::new(off.x * k.loco.facing, off.y)), STUMP_BLEEDS, STUMP_BLEEDS);
            match bleeding {
                Some(mut b) => b.spots.push(stump),
                None => {
                    commands.entity(c.target).insert(Bleeding { blood, spots: vec![stump], clock: 0.0 });
                }
            }
        }
        sounds.write(crate::sound::PlaySound::at("snap", root).pitch(0.8));
    }
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
        let text = format!("(view: Top, count: {legs}, reach: 10, color: (1, 1, 1), body: \"x\")");
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
