//! Silk: a move's `Web` act. A strand shot from the creature's spinnerets
//! (a point on its body: the spider turns its back and rears its abdomen
//! at you, `Rear::away`) at where its target will be, a little ahead of
//! them; it flies straight, out to its reach, only along a clear line
//! (the move's `line`). It sticks to its target and nothing else (a dash's
//! untouchable moment slips it), yanks them off their feet, and they're
//! reeled in to its jaws (`Tugged`: they can pull against it) for the
//! move's next phase, its bite. A dash, a blade across the strand or into
//! the spider, rock between them, or the spider struck senseless breaks
//! it. Missed, it lands where it hits (a little cobweb where it sticks)
//! or droops from the end of its reach.
//!
//! A strand is a thing of its own, its spider's (`by`): the move only
//! shoots it and reads how it's doing (`Hold`), and lets go of it as its
//! phase ends; let go, it's cut loose from the spinnerets, droops a
//! moment, fading, and is gone.

use bevy::prelude::*;
use serde::Deserialize;

use super::Quarry;
use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprite, CanvasSprites};
use crate::creatures::body::legs::Legs;
use crate::creatures::{Kinematics, Tugged};
use crate::world::{ChunkLoader, SimWorld};

const DT: f32 = super::DT;
/// Aimed this share of the way to where they'll be (ahead of them a little:
/// keep going and it falls behind, stop or turn and it's on you).
const LEAD: f32 = 0.5;
/// Sticks to what its tip passes this near (cells, from their box).
const STICK: f32 = 2.0;
/// Reeled in no slower than this (cells/s), however near. Caught, the
/// first yank: toward its jaws and up off their feet (cells/s); held up a
/// moment (a step, a rise), a hop up over it.
const REEL_LEAST: f32 = 140.0;
const YANK: (f32, f32) = (130.0, 120.0);
const HOP: f32 = 170.0;
/// Let go, a strand still in the air falls this fast (cells/s²), and it's
/// gone after this long (s).
const FALL: f32 = 260.0;
const FADE: f32 = 0.7;
/// A little cobweb where a strand sticks to the rock (cells across).
const COBWEB: i32 = 2;

/// A `Web` act, as moves.ron writes it.
#[derive(Clone, Debug, Deserialize)]
pub struct Web {
    /// Shot from here on its body (cells from its grip, facing right, y
    /// up: its spinnerets), `speed` cells/s, out to `reach` cells.
    pub from: (f32, f32),
    pub speed: f32,
    pub reach: f32,
    /// Caught, they're reeled in to a point `at` cells toward them (its
    /// jaws) in about `reel` s, till they're within `near` of it.
    pub at: f32,
    pub reel: f32,
    #[serde(default = "near")]
    pub near: f32,
}

fn near() -> f32 {
    6.0
}

/// A strand of silk out: whose, at whom, where it's from and to (the
/// world), and how it's doing.
#[derive(Component, Clone, Debug)]
pub struct Strand {
    def: Web,
    by: Entity,
    target: Entity,
    pub root: Vec2,
    pub tip: Vec2,
    /// Its tip last tick (drawn between), its flight, how far it's flown.
    was: Vec2,
    vel: Vec2,
    flown: f32,
    /// How fast it reels them in (set as it catches them), the nearest
    /// it's had them to its jaws, and how long they've come no nearer.
    reel: f32,
    closest: f32,
    stall: f32,
    pub hold: Hold,
    /// What broke its catch free, if anything did (a readout).
    pub freed: Option<&'static str>,
    /// Let go: on something (it doesn't fall), and how long ago.
    landed: bool,
    gone: f32,
}

/// How a strand's doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hold {
    /// Flying.
    Out,
    /// Stuck to them, reeling them in.
    Caught(Entity),
    /// Reeled in to its jaws (its bite's next).
    Reeled(Entity),
    /// Missed (it landed, or flew its reach), broken free of, or let go:
    /// slack, fading.
    Spent { missed: bool },
}

impl Strand {
    /// Let go (the move's done with it).
    pub fn let_go(&mut self) {
        if !matches!(self.hold, Hold::Spent { .. }) {
            self.hold = Hold::Spent { missed: false };
            self.vel = Vec2::ZERO;
        }
    }

    /// Whose it is.
    pub fn by(&self) -> Entity {
        self.by
    }
}

/// Where its spinnerets are: on its body as its legs carry it, or (no
/// legs, none drawn yet) beside its middle as it faces.
fn spinnerets(def: &Web, k: &Kinematics, legs: Option<&Legs>) -> Vec2 {
    let plain = k.body.pos + Vec2::new(def.from.0 * k.loco.facing, def.from.1);
    legs.map(|l| l.on_body(Vec2::new(def.from.0, def.from.1))).filter(|p| p.distance(k.body.pos) < 80.0).unwrap_or(plain)
}

/// A strand shot by `by` at `at`, from its spinnerets: toward where
/// they'll be when it gets there, a share of the way (`LEAD`).
pub(super) fn shoot(def: &Web, by: Entity, k: &Kinematics, legs: Option<&Legs>, at: &Quarry) -> Strand {
    let root = spinnerets(def, k, legs);
    let flight = root.distance(at.pos) / def.speed.max(1.0);
    let aim = (at.pos + at.vel * flight * LEAD - root).normalize_or(Vec2::X * k.loco.facing);
    Strand { def: def.clone(), by, target: at.e, root, tip: root, was: root, vel: aim * def.speed, flown: 0.0, reel: 0.0, closest: f32::MAX, stall: 0.0, hold: Hold::Out, freed: None, landed: false, gone: 0.0 }
}

/// Each strand, a tick: flown on (stuck to its target, or landed), its
/// catch reeled in (or broken free of), or, let go, falling and fading.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fly(
    mut commands: Commands,
    sim: Res<SimWorld>,
    blades: Res<crate::combat::LiveBlades>,
    mut dashed: MessageReader<crate::combat::Dashed>,
    mut strands: Query<(Entity, &mut Strand)>,
    bodies: Query<(&Kinematics, Option<&Legs>, Has<crate::combat::Invulnerable>)>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut recoils: MessageWriter<crate::combat::Recoil>,
) {
    let dashers: Vec<Entity> = dashed.read().map(|d| d.0).collect();
    let solid = |p: Vec2| sim.world.is_solid(platypus_sim::CellPos::from_world(p.x, p.y));
    for (me, mut s) in &mut strands {
        let s = &mut *s;
        s.was = s.tip;
        // (From its spinnerets, till it's let go: then it's cut loose
        // there. Its spider gone, it's let go.)
        let spider = bodies.get(s.by).ok();
        match spider {
            _ if matches!(s.hold, Hold::Spent { .. }) => {}
            Some((k, legs, _)) => s.root = spinnerets(&s.def, k, legs),
            None => s.let_go(),
        }
        match s.hold {
            Hold::Out => {
                let step = s.vel * DT;
                let n = (step.length() / 1.5).ceil().max(1.0) as i32;
                let quarry = bodies.get(s.target).ok().filter(|(_, _, safe)| !safe).map(|(k, ..)| (k.body.pos, k.body.half));
                let (start, mut next) = (s.tip, None);
                for i in 1..=n {
                    let p = start + step * (i as f32 / n as f32);
                    // (Only its target: it passes over anything else.)
                    if let Some((tp, th)) = quarry
                        && ((tp - p).abs() - th).max(Vec2::ZERO).length() <= STICK
                    {
                        s.tip = p;
                        next = Some(Hold::Caught(s.target));
                        break;
                    }
                    if solid(p) {
                        next = Some(Hold::Spent { missed: true });
                        break;
                    }
                    s.tip = p;
                }
                s.flown += step.length();
                match next {
                    Some(Hold::Caught(who)) => {
                        // (About `reel` s in, from where it caught them;
                        // yanked off their feet.)
                        let jaws = jaws(s, spider.map(|b| b.0), s.tip);
                        s.reel = (s.tip.distance(jaws) / s.def.reel.max(0.05)).max(REEL_LEAST);
                        s.hold = Hold::Caught(who);
                        if let Ok((tk, ..)) = bodies.get(who) {
                            let to = (jaws.x - tk.body.pos.x).signum();
                            recoils.write(crate::combat::Recoil { who, add: Vec2::new(to * YANK.0, YANK.1) - tk.body.vel, pogo: None, dive: None });
                        }
                        sounds.write(crate::sound::PlaySound::at("spit", s.tip).pitch(0.6).volume(0.8));
                    }
                    Some(_) => {
                        // Landed on rock: a little cobweb where it sticks.
                        s.hold = Hold::Spent { missed: true };
                        s.landed = true;
                        s.vel = Vec2::ZERO;
                        if let Some(web) = sim.world.materials().id("cobweb") {
                            let at = platypus_sim::CellPos::from_world(s.tip.x, s.tip.y);
                            commands.queue(move |world: &mut World| {
                                world.resource_mut::<SimWorld>().queue(platypus_sim::WorldEdit::Paint { center: at, radius: COBWEB, material: web, overwrite: false });
                            });
                        }
                        sounds.write(crate::sound::PlaySound::at("step_dirt", s.tip).pitch(1.6).volume(0.4));
                    }
                    None if s.flown >= s.def.reach => {
                        // (Its reach flown: it droops.)
                        s.hold = Hold::Spent { missed: true };
                        s.vel *= 0.15;
                    }
                    None => {}
                }
            }
            Hold::Caught(who) => {
                let Ok((tk, ..)) = bodies.get(who) else {
                    s.let_go();
                    continue;
                };
                let (tp, th) = (tk.body.pos, tk.body.half);
                s.tip = tp;
                let spider = spider.map(|b| b.0);
                let stunned = spider.is_some_and(|k| k.loco.state == platypus_physics::MoveState::Stunned);
                // Broken free: a dash, a blade across it or into the spider
                // (not its own), rock between, the spider senseless.
                let struck = spider.is_some_and(|k| blades.0.iter().filter(|b| b.who != s.by).any(|b| b.cells.iter().any(|c| ((*c - k.body.pos).abs() - k.body.half).max_element() <= 1.0)));
                let free = if dashers.contains(&who) {
                    Some("a dash")
                } else if stunned {
                    Some("the spider senseless")
                } else if struck || blades.across(s.root, tp, s.by).is_some() {
                    Some("a blade")
                } else if !super::clear(&sim, s.root, tp) {
                    Some("rock between")
                } else {
                    None
                };
                let jaws = jaws(s, spider, tp);
                if free.is_some() {
                    s.let_go();
                    s.freed = free;
                    commands.entity(who).try_remove::<Tugged>();
                    sounds.write(crate::sound::PlaySound::at("snap", tp).pitch(1.8).volume(0.7));
                } else if ((jaws - tp).abs() - th).max_element() <= s.def.near {
                    s.hold = Hold::Reeled(who);
                    commands.entity(who).try_remove::<Tugged>();
                } else {
                    commands.entity(who).try_insert(Tugged { to: jaws, speed: s.reel, left: 0.1 });
                    // (Held up a moment, not rising: hopped up over it.)
                    let gap = jaws.distance(tp);
                    s.stall = if gap > s.closest - 0.3 { s.stall + DT } else { 0.0 };
                    s.closest = s.closest.min(gap);
                    if s.stall > 0.08 && tk.body.vel.y <= 0.0 {
                        s.stall = 0.0;
                        recoils.write(crate::combat::Recoil { who, add: Vec2::Y * (HOP - tk.body.vel.y), pogo: None, dive: None });
                    }
                }
            }
            Hold::Reeled(who) => {
                if let Ok((tk, ..)) = bodies.get(who) {
                    s.tip = tk.body.pos;
                }
            }
            Hold::Spent { .. } => {
                s.gone += DT;
                if s.gone >= FADE {
                    commands.entity(me).despawn();
                    continue;
                }
                // (Still in the air: it falls till it's on something, its
                // cut end too.)
                if !s.landed {
                    s.vel = s.vel * (1.0 - 3.0 * DT) - Vec2::Y * FALL * DT;
                    let to = s.tip + s.vel * DT;
                    if solid(to) {
                        s.landed = true;
                    } else {
                        s.tip = to;
                    }
                }
                let to = s.root - Vec2::Y * FALL * s.gone * DT;
                if !solid(to) {
                    s.root = to;
                }
            }
        }
    }
}

/// Where it reels its catch to: `at` cells from its middle toward them.
fn jaws(s: &Strand, spider: Option<&Kinematics>, them: Vec2) -> Vec2 {
    spider.map_or(s.root, |k| k.body.pos + (them - k.body.pos).normalize_or(Vec2::X * k.loco.facing) * s.def.at)
}

/// The strands' canvas, over the bodies.
#[derive(Resource)]
pub struct SilkCanvas(Canvas);

impl Default for SilkCanvas {
    fn default() -> Self {
        SilkCanvas(Canvas::new("Silk", 10.75))
    }
}

/// Each strand, a cell at a time, from its spinnerets: taut and straight
/// in flight (a brighter bead at its tip); held, a little sag; let go,
/// slack and sagging more as it fades.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw(
    mut commands: Commands,
    mut canvas: ResMut<SilkCanvas>,
    mut images: ResMut<Assets<Image>>,
    time: Res<Time<Fixed>>,
    camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>,
    strands: Query<&Strand>,
    spiders: Query<(&Kinematics, Option<&Legs>)>,
    bodies: Query<&Transform, Without<CanvasSprite>>,
    mut sprites: CanvasSprites,
) {
    let Ok((cam, loader)) = camera.single() else { return };
    let any = !strands.is_empty();
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, any) else { return };
    let a = time.overstep_fraction();
    for s in &strands {
        // (From the spinnerets as they're drawn this frame; let go, from
        // where it was cut loose.)
        let root = match s.hold {
            Hold::Spent { .. } => s.root,
            _ => spiders.get(s.by).map_or(s.root, |(k, legs)| spinnerets(&s.def, k, legs)),
        };
        let (tip, sag, fade) = match s.hold {
            Hold::Out => (s.was.lerp(s.tip, a), 0.0, 1.0),
            Hold::Caught(who) | Hold::Reeled(who) => (bodies.get(who).map_or(s.tip, |t| t.translation.truncate()), 2.0 + root.distance(s.tip) * 0.02, 1.0),
            Hold::Spent { .. } => (s.was.lerp(s.tip, a), 3.0 + s.gone * 18.0, 1.0 - s.gone / FADE),
        };
        let alpha = (fade.clamp(0.0, 1.0) * 235.0) as u8;
        let (silk, shade) = ([236, 234, 226, alpha], [168, 166, 160, alpha / 2]);
        let mid = (root + tip) / 2.0 - Vec2::Y * sag;
        let steps = (root.distance(tip) * 1.5).ceil().max(1.0) as i32 + (sag * 2.0) as i32;
        let mut last = IVec2::MAX;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let c = root.lerp(mid, t).lerp(mid.lerp(tip, t), t).floor().as_ivec2();
            if c == last {
                continue;
            }
            last = c;
            // (A thread, a fainter one under it: it reads on dark rock
            // and on pale.)
            px.put(c.x, c.y, silk);
            px.put(c.x, c.y - 1, shade);
        }
        if s.hold == Hold::Out {
            let b = tip.floor().as_ivec2();
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1), (-1, 0), (0, -1)] {
                px.put(b.x + dx, b.y + dy, [250, 250, 244, 255]);
            }
        }
    }
}
