//! The Fallen Templar: a great knight of an order long gone, risen in its
//! armour, a halberd in its fists, a torn tabard over its plate and a cape
//! behind (cloth). Its legs and arms are its rig's (two-bone IK, legs.rs):
//! its hands go where the halberd's grips are, so the halberd is posed and
//! the arms follow it.
//!
//! It keeps you at the length of its halberd, which is long:
//! - **Thrust**: it draws the halberd back level (the tell) and lunges, the
//!   spike driven at you.
//! - **Chop**: it raises the halberd high behind its head and holds it (the
//!   tell), then brings the blade down in a great arc ahead of it: what the
//!   blade meets is cleaved, the ground where it lands broken, and what's
//!   along the haft on the ground is knocked off its feet.
//! - **Hook**: far off, it reaches the halberd out over you, turned hook
//!   down (the tell), and drags it back: caught on the hook, you're hauled
//!   in, and the chop comes at once (a dash slips the haul).
//! - **Spin**: it takes the haft by its middle and spins it, walking at you,
//!   blade and butt both.
//! - Too close for its reach, it shoves you off with the haft.
//!
//! A blade that meets its halberd's head as it thrusts, chops, drags or
//! spins parries it: it staggers (an opening).

use std::sync::Arc;

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind};
use serde::Deserialize;

use super::{CustomCreature, CustomSet};
use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprites};
use crate::cloth::{ClothDef, Drapes};
use crate::creatures::body::animation::Animator;
use crate::creatures::body::legs::Legs;
use crate::creatures::{Controls, Harm, Kinematics, Team, Tugged};
use crate::world::{ChunkLoader, SimWorld};

#[derive(Component, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Templar {
    #[serde(skip)]
    doing: Doing,
    /// Its halberd as it holds it now (from its middle, facing right).
    #[serde(skip)]
    pose: Pose,
    /// Where an attack's swing started (its pose then).
    #[serde(skip)]
    from: Pose,
    /// Its attacks' waits: thrust, chop, hook, spin, shove.
    #[serde(skip)]
    ready: [f32; 5],
    /// Whom it's struck, and until when they're spared.
    #[serde(skip)]
    struck: Vec<(Entity, f32)>,
    /// Whom its hook caught.
    #[serde(skip)]
    hooked: Option<Entity>,
    #[serde(skip)]
    roared: bool,
    #[serde(skip)]
    started: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Doing {
    /// Walking at you, or standing, the halberd held.
    #[default]
    Carry,
    /// Drawn back (the thrust's tell); lunging; held out a moment.
    Draw(f32),
    Lunge(f32),
    Thrust(f32),
    /// Raised high (the chop's tell: shorter after a hook); coming down;
    /// where it came down, a moment.
    Raise(f32, f32),
    Chop(f32),
    Rest(f32),
    /// Reached out over you, hook down (the tell); dragged back; hauling in
    /// whom it caught.
    Reach(f32),
    Drag(f32),
    Haul(f32, Entity),
    /// Taking the haft by its middle (the spin's tell); spinning it, walking
    /// at you; getting its footing back.
    Wind(f32),
    Spin(f32),
    Settle(f32),
    /// Too close: the haft back (a moment's tell), then shoved out.
    Brace(f32),
    Shove(f32),
    /// Parried: staggered, a moment.
    Stagger(f32),
}

impl CustomCreature for Templar {
    const NAME: &'static str = "templar";

    fn build(app: &mut App) {
        app.init_resource::<HalberdCanvas>()
            .add_observer(on_spawn)
            .add_systems(FixedUpdate, think.in_set(CustomSet::Think))
            .add_systems(Update, draw.after(bevy::transform::TransformSystems::Propagate));
    }
}

impl Templar {
    /// What it's doing, in a word (a scenario's log).
    pub fn doing(&self) -> &'static str {
        match self.doing {
            Doing::Carry => "carry",
            Doing::Draw(_) => "draw",
            Doing::Lunge(_) => "lunge",
            Doing::Thrust(_) => "thrust",
            Doing::Raise(..) => "raise",
            Doing::Chop(_) => "chop",
            Doing::Rest(_) => "rest",
            Doing::Reach(_) => "reach",
            Doing::Drag(_) => "drag",
            Doing::Haul(..) => "haul",
            Doing::Wind(_) => "wind",
            Doing::Spin(_) => "spin",
            Doing::Settle(_) => "settle",
            Doing::Brace(_) => "brace",
            Doing::Shove(_) => "shove",
            Doing::Stagger(_) => "stagger",
        }
    }
}

/// The halberd held: the point of it at `grip_u` (cells along the haft
/// from its butt) is at `grip` (from its middle, facing right), the haft at
/// `angle` (radians, facing right), the blade on the `roll` side (1: ahead
/// of the haft when it's upright; -1: turned over, the hook there); its
/// hands at `near` and `far` along it (the far one may let go).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pose {
    grip: Vec2,
    grip_u: f32,
    angle: f32,
    roll: f32,
    near: f32,
    far: Option<f32>,
}

impl Default for Pose {
    fn default() -> Self {
        carry()
    }
}

impl Pose {
    fn dir(&self) -> Vec2 {
        Vec2::from_angle(self.angle)
    }

    /// The blade's side of the haft.
    fn side(&self) -> Vec2 {
        Vec2::new(self.angle.sin(), -self.angle.cos()) * self.roll
    }

    /// A point `u` along the haft and `v` off it (toward the blade).
    fn at(&self, u: f32, v: f32) -> Vec2 {
        self.grip + self.dir() * (u - self.grip_u) + self.side() * v
    }

    /// Partway to `to`.
    fn lerp(&self, to: &Pose, t: f32) -> Pose {
        let t = t.clamp(0.0, 1.0);
        let turn = (to.angle - self.angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        Pose {
            grip: self.grip.lerp(to.grip, t),
            grip_u: self.grip_u + (to.grip_u - self.grip_u) * t,
            angle: self.angle + turn * t,
            roll: if t < 0.5 { self.roll } else { to.roll },
            near: self.near + (to.near - self.near) * t,
            far: match (self.far, to.far) {
                (Some(a), Some(b)) => Some(a + (b - a) * t),
                (_, b) if t >= 0.5 => b,
                (a, _) => a,
            },
        }
    }
}

/// Held by its far hand at `hand`, `u` along, at `deg`; its near hand at
/// `near` along.
fn held(hand: (f32, f32), deg: f32, u: f32, near: f32, roll: f32) -> Pose {
    Pose { grip: Vec2::new(hand.0, hand.1), grip_u: u, angle: deg.to_radians(), roll, near, far: Some(u) }
}

/// Led by its near hand at `hand`, `u` along, at `deg`; its far hand at
/// `far` along (reaching ahead: its near arm's the one that reaches).
fn led(hand: (f32, f32), deg: f32, u: f32, far: f32, roll: f32) -> Pose {
    Pose { grip: Vec2::new(hand.0, hand.1), grip_u: u, angle: deg.to_radians(), roll, near: u, far: Some(far) }
}

// (Its poses, each within its arms' reach: shoulders at (1.5, 18.5) and
// (-6.5, 18.5) from its middle, arms 29 long. Its middle's 39 over its
// feet: what it strikes at, you, is lower.)

/// Walking: across its body, its head high ahead.
fn carry() -> Pose {
    held((14.0, 6.0), 70.0, 42.0, 26.0, 1.0)
}
/// Standing: upright, its butt on the ground by its front foot, one hand on it.
fn stand() -> Pose {
    Pose { grip: Vec2::new(10.0, -37.0), grip_u: 0.0, angle: 88f32.to_radians(), roll: 1.0, near: 52.0, far: None }
}
/// Drawn back for the thrust, and lunged: the spike down at your chest.
fn drawn() -> Pose {
    led((7.0, -2.0), -16.0, 62.0, 36.0, 1.0)
}
fn lunged() -> Pose {
    led((19.0, -3.0), -20.0, 62.0, 36.0, 1.0)
}
fn raised() -> Pose {
    held((0.0, 30.0), 115.0, 50.0, 34.0, 1.0)
}
/// Chopped down: the blade's edge a few cells off the ground ahead.
fn chopped() -> Pose {
    led((18.0, 0.0), -35.0, 50.0, 34.0, 1.0)
}
/// The hook: held by its butt, reached out high over you, turned over
/// (hook down); dipped down beyond you; dragged back along the ground.
fn reached() -> Pose {
    led((22.0, 8.0), 15.0, 14.0, 2.0, -1.0)
}
fn dipped() -> Pose {
    led((22.0, 8.0), -20.0, 14.0, 2.0, -1.0)
}
fn dragged() -> Pose {
    led((7.0, 2.0), -26.0, 14.0, 2.0, -1.0)
}
/// Spun about its middle, by both hands there, at `a` (radians).
fn spun(a: f32) -> Pose {
    Pose { grip: Vec2::new(14.0, 6.0), grip_u: SPIN_MID, angle: a, roll: 1.0, near: SPIN_MID - 5.0, far: Some(SPIN_MID + 5.0) }
}
fn braced() -> Pose {
    led((12.0, -4.0), -8.0, 50.0, 34.0, 1.0)
}
fn shoved() -> Pose {
    led((22.0, -2.0), -8.0, 50.0, 34.0, 1.0)
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
/// The halberd: how long (cells, butt to spike tip); where its head starts.
const LENGTH: f32 = 110.0;
const HEAD: f32 = 76.0;
const SPIN_MID: f32 = 55.0;
const THRUST_I: usize = 0;
const CHOP_I: usize = 1;
const HOOK_I: usize = 2;
const SPIN_I: usize = 3;
const SHOVE_I: usize = 4;
/// Hunting what's this near (cells).
const AGGRO: f32 = 420.0;
/// After any attack, nothing for at least this long.
const BREATHER: f32 = 0.5;
/// The thrust: drawn back this long, lunging in this long (it steps in this
/// fast), held out this long; how much, how hard (ahead, up); then it
/// waits; it thrusts at what's from this far to that (across), this near
/// up or down.
const DRAW: f32 = 0.5;
const LUNGE: (f32, f32) = (0.15, 260.0);
const THRUST_HOLD: f32 = 0.3;
const THRUST_HIT: (f32, f32, f32) = (30.0, 400.0, 140.0);
const THRUST_EVERY: f32 = 2.6;
const THRUST_RANGE: (f32, f32, f32) = (40.0, 80.0, 40.0);
/// The chop: raised this long (after a hook, this long), down in this long,
/// resting this long; the blade's blow (how much, how hard: ahead, up);
/// along the haft where it lands (how near, how much, how hard up); the
/// crater; then it waits; it chops what's from this far to that.
const RAISE: (f32, f32) = (0.7, 0.35);
const DOWN: f32 = 0.2;
const REST: f32 = 0.7;
const CHOP_HIT: (f32, f32, f32) = (40.0, 300.0, 350.0);
const QUAKE: (f32, f32, f32) = (12.0, 22.0, 380.0);
const CRATER: (i32, u8) = (4, 25);
const CHOP_EVERY: f32 = 3.6;
const CHOP_RANGE: (f32, f32, f32) = (25.0, 60.0, 50.0);
/// The hook: reached out this long, dragged back in this long, hauling this
/// long (toward it this fast; yanked this hard: across, up); then it waits;
/// it hooks at what's from this far to that.
const REACH: f32 = 0.55;
const DRAG: f32 = 0.36;
const HAUL: (f32, f32) = (0.45, 260.0);
const YANK: (f32, f32) = (420.0, 220.0);
const HOOK_EVERY: f32 = 8.0;
const HOOK_RANGE: (f32, f32, f32) = (72.0, 100.0, 40.0);
/// The spin: wound up this long; spun this long, turning this fast rising
/// to that (rad/s), walking at you this fast (of its run); settling after;
/// what the head does and the haft does, how hard; once a target in this
/// long; then it waits; it spins at what's from this far to that.
const WIND: f32 = 0.5;
const SPIN: (f32, f32, f32, f32) = (1.6, 6.0, 14.0, 0.8);
const SETTLE: f32 = 0.6;
const SPIN_HIT: (f32, f32, f32) = (24.0, 12.0, 380.0);
const SPIN_ONCE: f32 = 0.35;
const SPIN_EVERY: f32 = 6.5;
const SPIN_RANGE: (f32, f32, f32) = (30.0, 100.0, 50.0);
/// The shove: braced this long, shoving in this long; how much, how hard
/// (away, up); then it waits; it shoves what's this near.
const BRACE: f32 = 0.22;
const SHOVE: f32 = 0.14;
const SHOVE_HIT: (f32, f32, f32) = (8.0, 460.0, 160.0);
const SHOVE_EVERY: f32 = 2.0;
const SHOVE_NEAR: (f32, f32) = (30.0, 40.0);
/// Parried: it staggers this long.
const STAGGER: f32 = 1.3;

/// What part of the halberd a point is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Part {
    Haft,
    Blade,
    Spike,
    Hook,
}

/// Points over the halberd posed so (facing right, from its middle), each
/// how near it must come to touch and what part it is.
fn points(p: &Pose) -> Vec<(Vec2, f32, Part)> {
    let mut out = Vec::with_capacity(80);
    for n in 0..=(HEAD as i32 / 3) {
        out.push((p.at(n as f32 * 3.0, 0.0), 1.6, Part::Haft));
    }
    for u in (80..=96).step_by(3) {
        for v in [3.0, 7.0, 11.0] {
            out.push((p.at(u as f32, v), 2.2, Part::Blade));
        }
    }
    for u in (97..=LENGTH as i32).step_by(3) {
        out.push((p.at(u as f32, 0.0), 1.6, Part::Spike));
    }
    for v in [-3.0, -6.0, -9.0] {
        out.push((p.at(85.0 - v * 0.4, v), 2.2, Part::Hook));
    }
    out
}

/// The world from its middle, facing right.
fn world(m: Vec2, facing: f32, p: Vec2) -> Vec2 {
    m + Vec2::new(p.x * facing, p.y)
}

/// Its halberd's points in the world: at the pose, and between it and the
/// last (it swings fast).
fn swept(m: Vec2, facing: f32, was: &Pose, now: &Pose) -> Vec<(Vec2, f32, Part)> {
    (1..=3).flat_map(|i| points(&was.lerp(now, i as f32 / 3.0))).map(|(p, r, part)| (world(m, facing, p), r, part)).collect()
}

/// What's struck: the first of these points touching that box.
fn touching(pts: &[(Vec2, f32, Part)], parts: &[Part], pos: Vec2, half: Vec2) -> Option<(Vec2, Part)> {
    pts.iter().filter(|(_, _, part)| parts.contains(part)).find(|(p, r, _)| ((pos - *p).abs() - half).max(Vec2::ZERO).length() <= *r).map(|(p, _, part)| (*p, *part))
}

/// A blade (not its own) meeting the halberd's head: where.
fn parried(blades: &crate::combat::LiveBlades, pts: &[(Vec2, f32, Part)], me: Entity) -> Option<Vec2> {
    blades.0.iter().filter(|b| b.who != me).find_map(|b| b.cells.iter().find(|c| pts.iter().any(|(p, r, part)| *part != Part::Haft && p.distance(**c) <= r + 1.5)).copied())
}

fn solid(sim: &SimWorld, p: Vec2) -> bool {
    sim.world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| !c.is_air() && matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder))
}

fn clang() -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter {
        count: 0.0,
        life: (0.08, 0.25),
        colors: vec![(255, 255, 240), (255, 220, 130), (200, 120, 60)],
        speed: 240.0,
        spread: 1.2,
        gravity: 300.0,
        drag: 4.0,
        size: 1.0,
        jitter: 0.0,
        glow: true,
    }
}

fn dust() -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter {
        count: 0.0,
        life: (0.4, 1.0),
        colors: vec![(170, 160, 140), (120, 112, 100), (80, 74, 68)],
        speed: 180.0,
        spread: 1.2,
        gravity: 260.0,
        drag: 2.0,
        size: 1.5,
        jitter: 0.0,
        glow: false,
    }
}

/// Its tabard and its cape, hung as it comes.
fn on_spawn(add: On<Add, Templar>, mut commands: Commands) {
    let e = add.entity;
    let seed = e.to_bits();
    let mut drapes = Drapes::default();
    // A long cape off its shoulders, dark, its hem in rags.
    drapes.own(Arc::new(ClothDef { color: (40, 44, 58), lining: (92, 28, 32), cols: 8, rows: 19, gap: 2.8, pin: (-9.0, 1.0, 19.0), torn: 0.45, weight: 1.2, front: false, behind: 200.0, arc: (-15.0, 150.0) }), seed);
    // A tabard over its plate, the red of its order faded, torn.
    drapes.own(Arc::new(ClothDef { color: (122, 32, 36), lining: (70, 20, 24), cols: 4, rows: 13, gap: 2.4, pin: (-3.0, 6.0, 15.0), torn: 0.35, weight: 1.0, front: true, behind: 25.0, arc: (-20.0, 120.0) }), seed ^ 7);
    commands.entity(e).insert(drapes);
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn think(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    mut q: Query<(Entity, &mut Templar, &Kinematics, &mut Controls, &mut Animator, Option<&mut Legs>)>,
    hunted: Query<(Entity, &Kinematics, &Team), Without<Templar>>,
    mut dashed: MessageReader<crate::combat::Dashed>,
    (blades, time): (Res<crate::combat::LiveBlades>, Res<Time>),
    (mut hits, mut sounds, mut recoils): (MessageWriter<crate::combat::Hit>, MessageWriter<crate::sound::PlaySound>, MessageWriter<crate::combat::Recoil>),
    (mut sparks, mut trauma, mut flares): (ResMut<crate::vfx::Sparks>, ResMut<crate::fx::Trauma>, MessageWriter<crate::fx::Flare>),
) {
    let dashers: Vec<Entity> = dashed.read().map(|d| d.0).collect();
    let now = time.elapsed_secs();
    for (e, mut o, k, mut c, mut anim, legs) in &mut q {
        let (pos, facing) = (k.body.pos, k.loco.facing);
        let w = |p: Vec2| world(pos, facing, p);
        if !o.started {
            o.started = true;
            o.pose = carry();
        }
        for r in o.ready.iter_mut() {
            *r -= DT;
        }
        o.struck.retain(|(_, until)| *until > now);
        let target = hunted.iter().filter(|(_, _, t)| t.hunted()).map(|(e, tk, _)| (e, tk.body.pos)).filter(|(_, p)| p.distance(pos) < AGGRO).min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)));
        if target.is_some() && !o.roared {
            o.roared = true;
            sounds.write(crate::sound::PlaySound::at("groan", pos).pitch(0.45).volume(1.0));
            sounds.write(crate::sound::PlaySound::at("clank", pos).pitch(0.5).volume(0.7));
        }
        let was = o.pose;
        let doing = o.doing;
        let mut next = doing;
        // (What the halberd does to whom it meets, each once a while.)
        let strike = |o: &mut Templar, parts: &[Part], once: f32, hits: &mut MessageWriter<crate::combat::Hit>, blow: &dyn Fn(Vec2, Part) -> (f32, Vec2)| -> bool {
            let pts = swept(pos, facing, &was, &o.pose);
            let mut any = false;
            for (te, tk, tt) in &hunted {
                if !tt.hunted() || o.struck.iter().any(|(s, _)| *s == te) {
                    continue;
                }
                if let Some((at, part)) = touching(&pts, parts, tk.body.pos, tk.body.half) {
                    o.struck.push((te, now + once));
                    let (damage, knock) = blow(at, part);
                    hits.write(crate::combat::Hit { by: Some(e), target: te, damage, harm: if part == Part::Haft { Harm::Blunt } else { Harm::Slash }, knock, stun: 0.35, at, dir: knock.normalize_or(Vec2::X), weight: 2.2, crit: false });
                    any = true;
                }
            }
            any
        };
        match doing {
            Doing::Carry => {
                anim.force = None;
                // (Standing, its butt grounded; walking, carried.)
                let want = if k.body.vel.x.abs() > 8.0 { carry() } else { stand() };
                o.pose = o.pose.lerp(&want, 1.0 - (-DT * 9.0f32).exp());
                if let Some((_, tp)) = target
                    && k.loco.grounded()
                {
                    let off = (tp - pos).abs();
                    let within = |r: (f32, f32, f32)| (r.0..r.1).contains(&off.x) && off.y < r.2;
                    let start = if off.x < SHOVE_NEAR.0 && off.y < SHOVE_NEAR.1 && o.ready[SHOVE_I] <= 0.0 {
                        Some(Doing::Brace(0.0))
                    } else if within(HOOK_RANGE) && o.ready[HOOK_I] <= 0.0 {
                        Some(Doing::Reach(0.0))
                    } else if within(SPIN_RANGE) && o.ready[SPIN_I] <= 0.0 && o.ready[CHOP_I] > 1.0 {
                        Some(Doing::Wind(0.0))
                    } else if within(CHOP_RANGE) && o.ready[CHOP_I] <= 0.0 {
                        Some(Doing::Raise(0.0, RAISE.0))
                    } else if within(THRUST_RANGE) && o.ready[THRUST_I] <= 0.0 {
                        Some(Doing::Draw(0.0))
                    } else {
                        None
                    };
                    if let Some(s) = start {
                        o.from = o.pose;
                        o.struck.clear();
                        match s {
                            Doing::Raise(..) | Doing::Reach(_) => sounds.write(crate::sound::PlaySound::at("groan", pos).pitch(0.5).volume(0.9)),
                            _ => sounds.write(crate::sound::PlaySound::at("clank", pos).pitch(0.6).volume(0.6)),
                        };
                        next = s;
                    }
                }
            }
            Doing::Draw(t) => {
                anim.play("back");
                o.pose = o.from.lerp(&drawn(), (t / (DRAW * 0.6)).min(1.0));
                // (Held back, the spike trembling: the tell.)
                o.pose.grip.y += (t * 50.0).sin() * 0.4 * t / DRAW;
                next = if t + DT >= DRAW {
                    sounds.write(crate::sound::PlaySound::at("swing_heavy", w(o.pose.at(LENGTH, 0.0))).pitch(0.6).volume(1.0));
                    recoils.write(crate::combat::Recoil { who: e, add: Vec2::new(facing * LUNGE.1, 0.0), pogo: None, dive: None });
                    Doing::Lunge(0.0)
                } else {
                    Doing::Draw(t + DT)
                };
            }
            Doing::Lunge(t) | Doing::Thrust(t) => {
                anim.play("lean");
                let lunging = matches!(doing, Doing::Lunge(_));
                o.pose = if lunging { drawn().lerp(&lunged(), ((t + DT) / LUNGE.0).powf(0.7)) } else { lunged() };
                let pts = swept(pos, facing, &was, &o.pose);
                if let Some(at) = parried(&blades, &pts, e) {
                    next = stagger(&mut o, at, &mut sounds, &mut sparks, &mut trauma, &mut flares);
                } else {
                    strike(&mut o, &[Part::Spike, Part::Blade], 1.0, &mut hits, &|_, _| (THRUST_HIT.0, Vec2::new(facing * THRUST_HIT.1, THRUST_HIT.2)));
                    next = match doing {
                        Doing::Lunge(_) if t + DT >= LUNGE.0 => Doing::Thrust(0.0),
                        Doing::Lunge(_) => Doing::Lunge(t + DT),
                        _ if t + DT >= THRUST_HOLD => {
                            o.ready[THRUST_I] = THRUST_EVERY;
                            Doing::Carry
                        }
                        _ => Doing::Thrust(t + DT),
                    };
                }
            }
            Doing::Raise(t, hold) => {
                anim.play("back");
                o.pose = o.from.lerp(&raised(), (t / (hold * 0.5)).min(1.0));
                // (Held high, shaking harder as it holds it.)
                let shake = (t / hold) * 1.2;
                o.pose.angle += (t * 37.0).sin() * 0.02 * shake;
                next = if t + DT >= hold {
                    o.from = o.pose;
                    sounds.write(crate::sound::PlaySound::at("swing_heavy", w(o.pose.at(90.0, 0.0))).pitch(0.4).volume(1.0));
                    Doing::Chop(0.0)
                } else {
                    Doing::Raise(t + DT, hold)
                };
            }
            Doing::Chop(t) => {
                anim.play("lean");
                let u = ((t + DT) / DOWN).min(1.0);
                o.pose = o.from.lerp(&chopped(), u * u);
                // (Stepping into it.)
                c.0.move_x = facing * 0.6;
                let pts = swept(pos, facing, &was, &o.pose);
                if u > 0.3
                    && let Some(at) = parried(&blades, &pts, e)
                {
                    next = stagger(&mut o, at, &mut sounds, &mut sparks, &mut trauma, &mut flares);
                } else {
                    strike(&mut o, &[Part::Blade, Part::Spike, Part::Hook], 1.0, &mut hits, &|_, _| (CHOP_HIT.0, Vec2::new(facing * CHOP_HIT.1, CHOP_HIT.2)));
                    next = if u >= 1.0 {
                        // Down: the ground under the blade broken, what's
                        // along the haft on the ground knocked off its feet.
                        let edge = w(o.pose.at(88.0, 12.0));
                        let ground = (0..40).map(|n| edge - Vec2::Y * n as f32).find(|&p| solid(&sim, p)).unwrap_or(edge);
                        sim.queue(platypus_sim::WorldEdit::Explode { center: CellPos::from_world(ground.x, ground.y), radius: CRATER.0, power: CRATER.1 });
                        let (a, b) = (w(o.pose.at(o.pose.near, 0.0)), w(o.pose.at(LENGTH, 0.0)));
                        let (lo, hi) = (a.x.min(b.x), a.x.max(b.x));
                        let mut x = lo;
                        while x <= hi {
                            sparks.emit(&dust(), 6, Vec2::new(x, ground.y + 1.0), Vec2::Y, Vec2::ZERO);
                            x += 6.0;
                        }
                        sparks.emit(&clang(), 24, ground + Vec2::Y * 2.0, Vec2::Y, Vec2::ZERO);
                        for (te, tk, tt) in &hunted {
                            if !tt.hunted() || o.struck.iter().any(|(s, _)| *s == te) {
                                continue;
                            }
                            let (tp, th) = (tk.body.pos, tk.body.half);
                            let gx = ((tp.x - th.x) - hi).max(lo - (tp.x + th.x)).max(0.0);
                            let gy = ((tp.y - th.y) - ground.y).max(0.0);
                            if gx <= 2.0 && gy <= QUAKE.0 {
                                let knock = Vec2::new(facing * 120.0, QUAKE.2);
                                hits.write(crate::combat::Hit { by: Some(e), target: te, damage: QUAKE.1, harm: Harm::Blunt, knock, stun: 0.4, at: ground, dir: Vec2::Y, weight: 2.0, crit: false });
                            }
                        }
                        trauma.0 = (trauma.0 + 0.6).min(1.0);
                        sounds.write(crate::sound::PlaySound::at("slam", ground).volume(1.0));
                        sounds.write(crate::sound::PlaySound::at("clang", ground).pitch(0.55).volume(0.8));
                        Doing::Rest(0.0)
                    } else {
                        Doing::Chop(t + DT)
                    };
                }
            }
            Doing::Rest(t) => {
                anim.play("lean");
                next = if t + DT >= REST {
                    o.ready[CHOP_I] = CHOP_EVERY;
                    Doing::Carry
                } else {
                    Doing::Rest(t + DT)
                };
            }
            Doing::Reach(t) => {
                anim.play("lean");
                o.pose = o.from.lerp(&reached(), (t / (REACH * 0.6)).min(1.0));
                o.pose.grip.y += (t * 23.0).sin() * 0.5;
                next = if t + DT >= REACH {
                    o.from = o.pose;
                    sounds.write(crate::sound::PlaySound::at("swing_heavy", w(o.pose.at(85.0, 0.0))).pitch(0.7).volume(0.9));
                    Doing::Drag(0.0)
                } else {
                    Doing::Reach(t + DT)
                };
            }
            Doing::Drag(t) => {
                anim.play("back");
                // (Down beyond you first, then back along the ground.)
                let u = ((t + DT) / DRAG).min(1.0);
                o.pose = if u < 0.4 { o.from.lerp(&dipped(), u / 0.4) } else { dipped().lerp(&dragged(), (u - 0.4) / 0.6) };
                let pts = swept(pos, facing, &was, &o.pose);
                if let Some(at) = parried(&blades, &pts, e) {
                    next = stagger(&mut o, at, &mut sounds, &mut sparks, &mut trauma, &mut flares);
                } else {
                    // Caught on the hook (or the blade's back): hauled in.
                    let caught = hunted.iter().filter(|(_, _, tt)| tt.hunted()).find_map(|(te, tk, _)| touching(&pts, &[Part::Hook, Part::Blade, Part::Spike], tk.body.pos, tk.body.half).map(|(at, _)| (te, at, tk.body.vel)));
                    next = if let Some((who, at, vel)) = caught {
                        let to = (pos + Vec2::X * facing * 42.0 - at).normalize_or(-Vec2::X * facing);
                        let want = Vec2::new(to.x * YANK.0, YANK.1);
                        recoils.write(crate::combat::Recoil { who, add: want - vel, pogo: None, dive: None });
                        hits.write(crate::combat::Hit { by: Some(e), target: who, damage: 8.0, harm: Harm::Pierce, knock: Vec2::ZERO, stun: 0.3, at, dir: to, weight: 1.0, crit: false });
                        sounds.write(crate::sound::PlaySound::at("clink", at).pitch(0.7).volume(1.0));
                        sounds.write(crate::sound::PlaySound::at("clank", at).pitch(0.5).volume(0.9));
                        trauma.0 = (trauma.0 + 0.35).min(1.0);
                        o.hooked = Some(who);
                        Doing::Haul(0.0, who)
                    } else if u >= 1.0 {
                        o.ready[HOOK_I] = HOOK_EVERY;
                        Doing::Carry
                    } else {
                        Doing::Drag(t + DT)
                    };
                }
            }
            Doing::Haul(t, who) => {
                anim.play("back");
                o.pose = dragged();
                let slipped = dashers.contains(&who);
                let to = pos + Vec2::new(facing * 42.0, -k.body.half.y + 12.0);
                if slipped || t + DT >= HAUL.0 || hunted.get(who).is_err() {
                    if let Ok(mut ec) = commands.get_entity(who) {
                        ec.remove::<Tugged>();
                    }
                    o.hooked = None;
                    o.ready[HOOK_I] = HOOK_EVERY;
                    o.from = o.pose;
                    // Hauled in: the chop, at once (slipped: it comes on).
                    next = if slipped {
                        sounds.write(crate::sound::PlaySound::at("cast", to).pitch(1.4));
                        Doing::Carry
                    } else {
                        sounds.write(crate::sound::PlaySound::at("groan", pos).pitch(0.55).volume(0.9));
                        Doing::Raise(0.0, RAISE.1)
                    };
                } else {
                    commands.entity(who).insert(Tugged { to, speed: HAUL.1, left: 0.1 });
                    next = Doing::Haul(t + DT, who);
                }
            }
            Doing::Wind(t) => {
                anim.play("idle");
                let start = spun(70f32.to_radians());
                o.pose = o.from.lerp(&start, (t / (WIND * 0.6)).min(1.0));
                if t > WIND * 0.6 {
                    o.pose.angle -= ((t - WIND * 0.6) / (WIND * 0.4)).powi(2) * 0.8;
                }
                next = if t + DT >= WIND { Doing::Spin(0.0) } else { Doing::Wind(t + DT) };
            }
            Doing::Spin(t) => {
                anim.play("idle");
                let (len, w0, w1, walk) = SPIN;
                let speed = w0 + (w1 - w0) * (t / (len * 0.4)).min(1.0);
                o.pose = spun(o.pose.angle - speed * DT);
                if (o.pose.angle / std::f32::consts::PI).floor() != (was.angle / std::f32::consts::PI).floor() {
                    sounds.write(crate::sound::PlaySound::at("swing_heavy", w(o.pose.grip)).pitch(0.6 + speed / w1 * 0.3).volume(0.9));
                }
                c.0.move_x = facing * walk;
                let pts = swept(pos, facing, &was, &o.pose);
                if let Some(at) = parried(&blades, &pts, e) {
                    o.ready[SPIN_I] = SPIN_EVERY;
                    next = stagger(&mut o, at, &mut sounds, &mut sparks, &mut trauma, &mut flares);
                } else {
                    let g = w(o.pose.grip);
                    strike(&mut o, &[Part::Blade, Part::Spike, Part::Hook, Part::Haft], SPIN_ONCE, &mut hits, &|at, part| {
                        let dir = ((at - g).perp() * facing + Vec2::Y * 0.5 * (at - g).length()).normalize_or(Vec2::Y);
                        (if part == Part::Haft { SPIN_HIT.1 } else { SPIN_HIT.0 }, dir * SPIN_HIT.2)
                    });
                    // Its ends through the ground: sparks.
                    for u in [0.0, LENGTH] {
                        let p = w(o.pose.at(u, 0.0));
                        if solid(&sim, p) && (now * 30.0) as i32 % 2 == 0 {
                            sparks.emit(&clang(), 3, p + Vec2::Y * 2.0, Vec2::new(-facing, 0.6).normalize(), Vec2::ZERO);
                        }
                    }
                    next = if t + DT >= len {
                        o.ready[SPIN_I] = SPIN_EVERY;
                        o.from = o.pose;
                        Doing::Settle(0.0)
                    } else {
                        Doing::Spin(t + DT)
                    };
                }
            }
            Doing::Settle(t) => {
                anim.play("hurt");
                o.pose = o.from.lerp(&carry(), (t / (SETTLE * 0.6)).min(1.0));
                next = if t + DT >= SETTLE { Doing::Carry } else { Doing::Settle(t + DT) };
            }
            Doing::Brace(t) => {
                anim.play("back");
                o.pose = o.from.lerp(&braced(), (t / BRACE).min(1.0));
                next = if t + DT >= BRACE {
                    sounds.write(crate::sound::PlaySound::at("swing", pos).pitch(0.5).volume(0.9));
                    Doing::Shove(0.0)
                } else {
                    Doing::Brace(t + DT)
                };
            }
            Doing::Shove(t) => {
                anim.play("lean");
                o.pose = braced().lerp(&shoved(), ((t + DT) / SHOVE).min(1.0));
                if strike(&mut o, &[Part::Haft, Part::Blade, Part::Hook, Part::Spike], 1.0, &mut hits, &|_, _| (SHOVE_HIT.0, Vec2::new(facing * SHOVE_HIT.1, SHOVE_HIT.2))) {
                    sounds.write(crate::sound::PlaySound::at("kick", pos).pitch(0.6).volume(1.0));
                }
                // (Close up, its body shoves too.)
                for (te, tk, tt) in &hunted {
                    if tt.hunted() && !o.struck.iter().any(|(s, _)| *s == te) && (tk.body.pos.x - pos.x) * facing > 0.0 && (tk.body.pos - pos).abs().cmplt(Vec2::new(SHOVE_NEAR.0 + 4.0, SHOVE_NEAR.1)).all() {
                        o.struck.push((te, now + 1.0));
                        let knock = Vec2::new(facing * SHOVE_HIT.1, SHOVE_HIT.2);
                        hits.write(crate::combat::Hit { by: Some(e), target: te, damage: SHOVE_HIT.0, harm: Harm::Blunt, knock, stun: 0.3, at: tk.body.pos, dir: knock.normalize(), weight: 2.0, crit: false });
                    }
                }
                next = if t + DT >= SHOVE + 0.15 {
                    o.ready[SHOVE_I] = SHOVE_EVERY;
                    Doing::Carry
                } else {
                    Doing::Shove(t + DT)
                };
            }
            Doing::Stagger(t) => {
                anim.play("hurt");
                o.pose = o.from.lerp(&carry(), (t / STAGGER).min(1.0));
                o.pose.angle += (t * 20.0).sin() * 0.15 * (1.0 - t / STAGGER);
                next = if t + DT >= STAGGER { Doing::Carry } else { Doing::Stagger(t + DT) };
            }
        }
        // An attack done: a breather before the next.
        if next == Doing::Carry && doing != Doing::Carry {
            for r in o.ready.iter_mut() {
                *r = r.max(BREATHER);
            }
        }
        // Hauling broken off some other way (parried): let go.
        if !matches!(next, Doing::Haul(..))
            && let Some(who) = o.hooked.take()
            && let Ok(mut ec) = commands.get_entity(who)
        {
            ec.remove::<Tugged>();
        }
        o.doing = next;
        // (Busy, it stands; chopping or spinning, it steps in.)
        if !matches!(next, Doing::Carry | Doing::Chop(_) | Doing::Spin(_)) {
            c.0.move_x = 0.0;
        }
        if !matches!(next, Doing::Carry) {
            c.0.jump = false;
        }
        // Its hands where the halberd's grips are.
        if let Some(mut legs) = legs {
            let p = o.pose;
            let hand = |u: f32| {
                let at = p.at(u, 0.0);
                Vec2::new(at.x * facing, at.y)
            };
            legs.hold(0, Some(hand(p.near)));
            legs.hold(1, p.far.map(hand));
        }
    }
}

/// Parried at `at`: sparks, a ring, it staggers.
fn stagger(o: &mut Templar, at: Vec2, sounds: &mut MessageWriter<crate::sound::PlaySound>, sparks: &mut crate::vfx::Sparks, trauma: &mut crate::fx::Trauma, flares: &mut MessageWriter<crate::fx::Flare>) -> Doing {
    sounds.write(crate::sound::PlaySound::at("parry", at).volume(1.0));
    sounds.write(crate::sound::PlaySound::at("clang", at).pitch(0.6).volume(0.8));
    sparks.emit(&clang(), 40, at, Vec2::Y, Vec2::ZERO);
    flares.write(crate::fx::Flare { at, color: [1.6, 1.5, 1.2], life: 0.25 });
    trauma.0 = (trauma.0 + 0.4).min(1.0);
    o.from = o.pose;
    o.from.angle += 0.5;
    o.ready[THRUST_I] = o.ready[THRUST_I].max(1.5);
    o.ready[CHOP_I] = o.ready[CHOP_I].max(2.0);
    Doing::Stagger(0.0)
}

/// Its halberd, near pauldron and gauntlets: over its body, its arms and
/// its tabard.
#[derive(Resource)]
pub struct HalberdCanvas(Canvas);

impl Default for HalberdCanvas {
    fn default() -> Self {
        HalberdCanvas(Canvas::new("Halberds", 11.3))
    }
}

/// The halberd's colour at `u` along it and `v` off it (toward the blade).
fn halberd_pixel(u: f32, v: f32) -> Option<[u8; 4]> {
    let edge = [222, 228, 238, 255];
    let steel = [134, 138, 152, 255];
    let dark = [84, 88, 102, 255];
    // The spike.
    if (97.0..=LENGTH).contains(&u) && v.abs() <= 2.0 * (LENGTH - u) / 13.0 + 0.3 {
        return Some(if v > 0.3 { edge } else { steel });
    }
    // The axe blade: a beard below, a crescent edge, its top slanting up.
    if v >= 1.5 {
        let lower = 81.0 - (v - 1.5) * 0.29;
        let upper = 94.0 + (v - 1.5) * 0.35;
        let rim = 11.0 + 3.0 * (std::f32::consts::PI * ((u - 78.0) / 19.0).clamp(0.0, 1.0)).sin();
        if u >= lower && u <= upper && v <= rim {
            if (86.0..=88.5).contains(&u) && (5.0..=7.5).contains(&v) {
                return Some([30, 30, 36, 255]);
            }
            return Some(if v > rim - 1.4 { edge } else if u < lower + 1.5 || v < 2.6 { dark } else { steel });
        }
    }
    // The hook behind, curving up toward the head.
    if v <= -1.5 {
        let t = (-v - 1.5) / 7.5;
        if t <= 1.0 {
            let mid = 85.0 + t * 4.0;
            let half = 2.6 * (1.0 - t) + 0.4;
            if (u - mid).abs() <= half {
                return Some(if u > mid + half - 0.9 { edge } else { steel });
            }
        }
    }
    // The socket and its langets, riveted.
    if (HEAD - 2.0..=97.0).contains(&u) && v.abs() <= 2.2 {
        return Some(if ((u - 76.0).abs() < 0.8 || (u - 81.0).abs() < 0.8) && v.abs() < 0.8 { edge } else { dark });
    }
    // The haft: ash, bands of iron, a leather wrap; an iron butt.
    if (3.0..HEAD).contains(&u) && v.abs() <= 1.3 {
        if (20.0..21.5).contains(&u) || (44.0..45.5).contains(&u) || (66.0..67.5).contains(&u) {
            return Some(dark);
        }
        if (28.0..37.0).contains(&u) {
            return Some([62, 42, 30, 255]);
        }
        return Some(if v > 0.0 { [132, 96, 60, 255] } else { [92, 62, 40, 255] });
    }
    if (0.0..3.0).contains(&u) && v.abs() <= 2.0 {
        return Some([70, 72, 82, 255]);
    }
    None
}

/// A filled disc, darker at its rim, lit at its top.
fn disc(px: &mut crate::canvas::Pixels, c: Vec2, r: f32, fill: [u8; 4], lit: [u8; 4], rim: [u8; 4]) {
    let n = r.ceil() as i32 + 1;
    for y in -n..=n {
        for x in -n..=n {
            let o = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let d = Vec2::new(o.x, o.y * r / (r * 0.85));
            if d.length() <= r {
                let p = (c + o).floor().as_ivec2();
                px.put(p.x, p.y, if d.length() > r - 1.0 { rim } else if o.y > r * 0.35 { lit } else { fill });
            }
        }
    }
}

fn draw(mut commands: Commands, mut canvas: ResMut<HalberdCanvas>, mut images: ResMut<Assets<Image>>, camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>, q: Query<(&Templar, &GlobalTransform, &Kinematics)>, mut sprites: CanvasSprites) {
    let Ok((cam, loader)) = camera.single() else { return };
    let any = q.iter().any(|(o, ..)| o.started);
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, any) else { return };
    for (o, tf, k) in &q {
        if !o.started {
            continue;
        }
        // (Drawn where its body's drawn, as its arms are.)
        let (m, facing) = (tf.translation().truncate(), k.loco.facing);
        let p = o.pose;
        // Its near pauldron, over its arm.
        disc(&mut px, world(m, facing, Vec2::new(1.5, 20.0)), 5.5, [124, 128, 142, 255], [172, 178, 192, 255], [22, 20, 26, 255]);
        // The halberd: every cell near it, turned back to the haft's frame.
        let (dir, side) = (Vec2::new(p.dir().x * facing, p.dir().y), Vec2::new(p.side().x * facing, p.side().y));
        let (a, b) = (world(m, facing, p.at(0.0, 0.0)), world(m, facing, p.at(LENGTH, 0.0)));
        let lo = a.min(b) - Vec2::splat(16.0);
        let hi = a.max(b) + Vec2::splat(16.0);
        let base = world(m, facing, p.at(0.0, 0.0));
        for y in lo.y.floor() as i32..=hi.y.ceil() as i32 {
            for x in lo.x.floor() as i32..=hi.x.ceil() as i32 {
                let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - base;
                if let Some(col) = halberd_pixel(d.dot(dir), d.dot(side)) {
                    px.put(x, y, col);
                }
            }
        }
        // Its gauntlets, round the haft.
        for u in std::iter::once(p.near).chain(p.far) {
            disc(&mut px, world(m, facing, p.at(u, 0.0)), 2.8, [96, 100, 114, 255], [150, 156, 170, 255], [22, 20, 26, 255]);
        }
    }
}
