//! Dread knights: nearly three times your height, crowned and hooded, hung
//! all over with rags (a great torn cloak behind, rags off its shoulders
//! and arms, strips over its body: cloth), chains at its belt and wrist,
//! and a great spiked ball on a long chain in its fist (the flail: a
//! weight on a chain, swung). It walks at you, never hurrying (its brain),
//! and shrieks when it first sees you.
//!
//! - In its chain's reach it whirls the ball over its head (the tell) and
//!   brings it down where you stand: the ground cratered, what's near
//!   thrown.
//! - Near, it draws the ball back and sweeps it along the ground (jump it).
//! - Further, it holds its free hand out at you and the void gathers there
//!   (motes drawn in, a low hum: the tell), then yanks you toward it in
//!   three hard tugs, each warned by a throb in its hand: thrown toward it
//!   and up off your feet (walking back won't hold you; a dash slips one),
//!   and then, at once, its flail.
//! - Before any of these, if the ball's lying away from it, it hauls it
//!   back in first, the chain taut.
//! - Or a sphere of void swells in its outstretched palm, crackling (the
//!   tell), and three orbs fly from it at you, fanned (weave between them),
//!   leaving void smoke, bursting where they strike.
//!
//! Between, the ball drags behind it, and hurts what it hits going fast.
//!
//! A blade that meets the ball as it comes down or sweeps parries it: the
//! ball's knocked back, the knight staggers (an opening). Struck lying on
//! the ground, the ball's batted along, the harder the heavier the blow.
//! A volley may come in up to three rounds, each re-charged (shorter) and
//! re-aimed. Dead, its cloak can be worn.

use std::sync::Arc;

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind};
use serde::Deserialize;

use super::{CustomCreature, CustomSet};
use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprites};
use crate::cloth::{ClothDef, Drapes, Links, draw_links};
use crate::creatures::{Controls, Died, Harm, Kinematics, Team, Tugged};
use crate::world::{ChunkLoader, SimWorld};

/// What it's doing and what it carries.
#[derive(Component, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Dread {
    #[serde(skip)]
    doing: Doing,
    /// The ball, where it was last tick; the hand that holds its chain; its
    /// other hand (the void's).
    #[serde(skip)]
    ball: Vec2,
    #[serde(skip)]
    ball_prev: Vec2,
    #[serde(skip)]
    hand: Vec2,
    #[serde(skip)]
    cast: Vec2,
    /// Which way it aims its void.
    #[serde(skip)]
    aim: Vec2,
    #[serde(skip)]
    ready: [f32; 4],
    /// Whom the ball's struck this swing (once each).
    #[serde(skip)]
    struck: Vec<Entity>,
    #[serde(skip)]
    shrieked: bool,
    /// Chains hanging at its belt and wrist.
    #[serde(skip)]
    chains: Vec<Links>,
    #[serde(skip)]
    started: bool,
    /// When it may next be batted along.
    #[serde(skip)]
    batted: f32,
}

/// Its attacks' waits: slam, sweep, pull, volley.
const SLAM_I: usize = 0;
const SWEEP_I: usize = 1;
const PULL_I: usize = 2;
const VOLLEY_I: usize = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Doing {
    /// Walking, the ball dragging behind.
    #[default]
    Carry,
    /// Whirling it overhead (the tell).
    Whirl(f32),
    /// Bringing it down, from there to there.
    Slam(f32, Vec2, Vec2),
    /// Where it came down, a moment.
    Rest(f32),
    /// Drawing it back low (the sweep's tell), from there.
    Draw(f32, Vec2),
    /// Sweeping it along the ground.
    Sweep(f32),
    /// The void gathering in its hand (the pull's tell).
    Gather(f32),
    /// Drawing whom it hunts toward it: in tugs (how many done), each a
    /// throb (its tell), a tug, a rest.
    Pull(f32, Entity, u32),
    /// The void swelling in its palm (the volley's tell): this round of
    /// so many.
    Ready(f32, u32, u32),
    /// Parried: staggered, a moment.
    Stagger(f32),
    /// Hauling the ball back in, then that.
    Haul(f32, Next),
}

/// What it hauls the ball in for.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Next {
    Whirl,
    Draw,
    Gather,
    Ready,
}

impl CustomCreature for Dread {
    const NAME: &'static str = "dread";

    fn build(app: &mut App) {
        app.init_resource::<FlailCanvas>()
            .add_observer(on_spawn)
            .add_systems(FixedUpdate, think.in_set(CustomSet::Think))
            .add_systems(FixedUpdate, on_death.in_set(crate::world::TickSet::Bodies))
            .add_systems(Update, draw);
    }
}

impl Dread {
    /// Where its ball is (a scenario's log).
    pub fn ball(&self) -> Vec2 {
        self.ball
    }

    /// What it's doing, in a word (a scenario's log).
    pub fn doing(&self) -> &'static str {
        match self.doing {
            Doing::Carry => "carry",
            Doing::Whirl(_) => "whirl",
            Doing::Slam(..) => "slam",
            Doing::Rest(_) => "rest",
            Doing::Draw(..) => "draw",
            Doing::Sweep(_) => "sweep",
            Doing::Gather(_) => "gather",
            Doing::Pull(..) => "pull",
            Doing::Ready(..) => "ready",
            Doing::Stagger(_) => "stagger",
            Doing::Haul(..) => "haul",
        }
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
/// The flail: its chain's length, the ball's size, how hard it falls,
/// links in its chain.
const CHAIN: f32 = 72.0;
/// Thrown (the slam), it pays out this much chain; reeled back in after at
/// up to this speed (cells/s).
const THROW: f32 = 110.0;
const REEL: f32 = 420.0;
const BALL: f32 = 8.5;
const BALL_FALL: f32 = 1100.0;
/// On the ground, the ball keeps this much of its roll a tick.
const GRIP: f32 = 0.93;
const CHAIN_LINKS: usize = 16;
/// Hunting what's this near (cells).
const AGGRO: f32 = 420.0;
/// The slam: whirled this long, this far round its hand; down in this
/// long, this high an arc; resting there; then it waits. It slams what's
/// within its chain's reach and this.
const WHIRL: (f32, f32) = (1.0, 34.0);
const SLAM: (f32, f32) = (0.36, 44.0);
const REST: f32 = 0.8;
const SLAM_EVERY: f32 = 3.6;
const SLAM_FROM: f32 = 10.0;
/// What the slam does: how far (cells from where it lands), how much, how
/// hard; the crater.
const QUAKE: (f32, f32, f32) = (26.0, 36.0, 440.0);
const CRATER: (i32, u8) = (6, 30);
/// The sweep: drawn back this long, swept round in this long (from, to:
/// degrees the way it faces), how much, how hard; then it waits; near
/// enough to sweep (across, up or down).
const DRAW: f32 = 0.55;
const SWEEP: (f32, f32, f32) = (0.6, 200.0, 345.0);
const SWEEP_HIT: (f32, f32) = (28.0, 400.0);
const SWEEP_EVERY: f32 = 3.2;
const SWEEP_NEAR: (f32, f32) = (52.0, 30.0);
/// The pull: the void gathers this long; then it draws you in so many
/// tugs, each a throb (s), a tug (s, cells/s) and a rest (s); from this
/// near to this far; then it waits.
const GATHER: f32 = 0.85;
const TUGS: u32 = 3;
const TUG: (f32, f32, f32, f32) = (0.3, 0.45, 210.0, 0.3);
/// Each tug's yank: toward its hand this fast, and up this fast (as it
/// starts; then dragged at `TUG.2` through it).
const YANK: (f32, f32) = (430.0, 300.0);
/// It hauls the ball in when it's further than this from its hand (cells),
/// at up to this speed, for at most this long.
const HAUL: (f32, f32, f32) = (22.0, 520.0, 0.6);
const PULL_RANGE: (f32, f32) = (90.0, 240.0);
const PULL_EVERY: f32 = 9.0;
/// The volley: the void swells in its palm this long; three, this far apart (degrees);
/// from this near to this far; then it waits.
const READY: f32 = 1.1;
/// Up to this many rounds a volley; each after the first charged this long.
const VOLLEY_ROUNDS: u32 = 3;
const READY_AGAIN: f32 = 0.7;
/// Parried, it staggers this long; the ball knocked back this fast (and up).
const STAGGER: f32 = 1.3;
const PARRIED: (f32, f32) = (520.0, 270.0);
/// Batted along the ground: a blow's knockback × this (cells/s), at most
/// once in this long.
const BAT: (f32, f32) = (0.65, 0.2);
const FAN: f32 = 16.0;
const VOLLEY_RANGE: (f32, f32) = (100.0, 320.0);
const VOLLEY_EVERY: f32 = 6.0;
/// Swung fast, the ball hurts what it hits.
const FAST: (f32, f32) = (320.0, 18.0);

/// Where its flail hand is, and its other hand (from its middle, the way
/// it faces).
fn hands_at(pos: Vec2, facing: f32, doing: Doing) -> (Vec2, Vec2) {
    let o = match doing {
        Doing::Whirl(_) | Doing::Slam(..) => Vec2::new(10.5, 36.0),
        Doing::Rest(_) => Vec2::new(12.5, 4.5),
        Doing::Draw(..) | Doing::Sweep(_) => Vec2::new(4.5, -4.0),
        _ => Vec2::new(11.5, -8.0),
    };
    // (Its free hand held out at you, palm open.)
    let cast = Vec2::new(13.0, 15.5);
    (pos + Vec2::new(o.x * facing, o.y), pos + Vec2::new(cast.x * facing, cast.y))
}

/// A blade (not its own) meeting the ball, going from `a` to `b` this
/// tick: where, and how hard that blade pushes.
fn parried(blades: &crate::combat::LiveBlades, a: Vec2, b: Vec2, me: Entity) -> Option<(Vec2, f32)> {
    let ab = b - a;
    let near = |p: Vec2| {
        let k = if ab.length_squared() > 1e-6 { ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0) } else { 0.0 };
        p.distance(a + ab * k)
    };
    blades.0.iter().filter(|bl| bl.who != me).find_map(|bl| bl.cells.iter().find(|c| near(**c) <= BALL + 1.5).map(|c| (*c, bl.knock)))
}

/// Sparks off iron struck.
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

/// Parried: the ball knocked back off the blade (at `at`), it staggers.
fn knocked_back(d: &mut Dread, at: Vec2, sounds: &mut MessageWriter<crate::sound::PlaySound>, sparks: &mut crate::vfx::Sparks, trauma: &mut crate::fx::Trauma, flares: &mut MessageWriter<crate::fx::Flare>) -> Doing {
    let off = (d.ball - at).normalize_or(Vec2::Y);
    let v = Vec2::new(off.x * PARRIED.0, off.y.max(0.0) * PARRIED.0 + PARRIED.1);
    d.ball_prev = d.ball - v * DT;
    sounds.write(crate::sound::PlaySound::at("parry", at).volume(1.0));
    sounds.write(crate::sound::PlaySound::at("clang", at).pitch(0.7).volume(0.8));
    sparks.emit(&clang(), 40, at, off, Vec2::ZERO);
    flares.write(crate::fx::Flare { at, color: [1.6, 1.5, 1.2], life: 0.25 });
    trauma.0 = (trauma.0 + 0.45).min(1.0);
    d.ready[SLAM_I] = SLAM_EVERY;
    d.ready[SWEEP_I] = SWEEP_EVERY;
    Doing::Stagger(0.0)
}

/// That attack, its first moment (and its sound).
fn start(d: &Dread, then: Next, sounds: &mut MessageWriter<crate::sound::PlaySound>) -> Doing {
    match then {
        Next::Whirl => Doing::Whirl(0.0),
        Next::Draw => {
            sounds.write(crate::sound::PlaySound::at("clank", d.ball).pitch(0.45));
            Doing::Draw(0.0, d.ball)
        }
        Next::Gather => {
            sounds.write(crate::sound::PlaySound::at("pour", d.cast).pitch(0.22).volume(1.0));
            Doing::Gather(0.0)
        }
        Next::Ready => {
            sounds.write(crate::sound::PlaySound::at("pour", d.cast).pitch(0.35).volume(0.9));
            // (One round, two or three: as it likes.)
            let rounds = 1 + (d.ball.x.to_bits() ^ d.ball.y.to_bits()) % VOLLEY_ROUNDS;
            Doing::Ready(0.0, 0, rounds)
        }
    }
}

/// That attack: the ball hauled in first, if it's lying away.
fn haul_then(d: &Dread, then: Next, sounds: &mut MessageWriter<crate::sound::PlaySound>) -> Doing {
    if d.ball.distance(d.hand) > HAUL.0 {
        sounds.write(crate::sound::PlaySound::at("clank", d.ball).pitch(0.6).volume(0.8));
        Doing::Haul(0.0, then)
    } else {
        start(d, then, sounds)
    }
}

/// Its cloak and rags, hung as it comes.
fn on_spawn(add: On<Add, Dread>, mut commands: Commands) {
    let e = add.entity;
    let seed = e.to_bits();
    let mut drapes = Drapes::default();
    let rag = |color: (u8, u8, u8), cols, rows, pin, torn, front, behind| Arc::new(ClothDef { color, lining: (64, 30, 38), cols, rows, gap: 2.4, pin, torn, weight: 0.9, front, behind, arc: (-25.0, 150.0) });
    // A great cloak off its shoulders, wider than it, torn to rags: it
    // streams behind it.
    drapes.own(Arc::new(ClothDef { color: (42, 42, 56), lining: (72, 26, 32), cols: 10, rows: 22, gap: 2.8, pin: (-20.0, -6.0, 17.0), torn: 0.6, weight: 1.2, front: false, behind: 220.0, arc: (-15.0, 150.0) }), seed);
    // Rags off its shoulders and arms, long and torn.
    for (n, (x0, x1, y, rows)) in [(-12.0, -9.0, 16.0, 17), (-14.0, -12.0, 4.0, 13), (9.0, 12.0, 16.0, 16), (11.5, 13.5, 2.0, 12)].into_iter().enumerate() {
        drapes.own(rag((50, 48, 60), 2, rows, (x0, x1, y), 0.65, false, 70.0), seed ^ (n as u64 + 1));
    }
    // Strips over its body: from its chest and its belt.
    for (n, (x0, x1, y, rows)) in [(-5.0, -2.0, 12.0, 14), (1.0, 4.0, 6.0, 12), (-2.0, 1.0, -3.0, 10)].into_iter().enumerate() {
        drapes.own(rag((38, 36, 48), 2, rows, (x0, x1, y), 0.7, true, 25.0), seed ^ (n as u64 + 11));
    }
    commands.entity(e).insert(drapes);
}

fn solid(sim: &SimWorld, p: Vec2) -> bool {
    sim.world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| !c.is_air() && matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder))
}

/// The ball on its chain, a tick: it falls and swings, held at most the
/// chain's length from the hand, rests and drags on the ground.
fn hang_ball(sim: &SimWorld, d: &mut Dread) {
    let mut v = (d.ball - d.ball_prev) * 0.985;
    if solid(sim, d.ball - Vec2::Y * (BALL + 1.0)) {
        v.x *= GRIP;
    }
    d.ball_prev = d.ball;
    d.ball += v - Vec2::Y * BALL_FALL * DT * DT;
    // (Paid out further than it drags: reeled back in, not snapped.)
    let off = d.ball - d.hand;
    let len = off.length();
    if len > CHAIN {
        let reach = (len - REEL * DT).max(CHAIN);
        d.ball = d.hand + off.normalize() * reach;
        if reach > CHAIN {
            d.ball_prev = d.ball;
        }
    }
    for _ in 0..10 {
        if solid(sim, d.ball - Vec2::Y * BALL) {
            d.ball.y += 1.0;
        } else {
            break;
        }
    }
}

/// The void's motes: drawn in to its hand, or streaming from you to it.
fn void_motes() -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter {
        count: 0.0,
        life: (0.25, 0.45),
        colors: vec![(230, 210, 255), (140, 80, 230), (60, 24, 110), (18, 6, 30)],
        speed: 120.0,
        spread: 0.2,
        gravity: 0.0,
        drag: 0.0,
        size: 1.3,
        jitter: 0.0,
        glow: true,
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn think(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    book: Res<crate::magic::Spellbook>,
    mut q: Query<(Entity, &mut Dread, &Kinematics, &mut Controls, &mut crate::creatures::body::animation::Animator)>,
    hunted: Query<(Entity, &Kinematics, &Team), Without<Dread>>,
    mut dashed: MessageReader<crate::combat::Dashed>,
    (blades, time): (Res<crate::combat::LiveBlades>, Res<Time>),
    (mut hits, mut casts, mut sounds, mut recoils): (MessageWriter<crate::combat::Hit>, MessageWriter<crate::magic::CastRequest>, MessageWriter<crate::sound::PlaySound>, MessageWriter<crate::combat::Recoil>),
    (mut sparks, mut trauma, mut flares): (ResMut<crate::vfx::Sparks>, ResMut<crate::fx::Trauma>, MessageWriter<crate::fx::Flare>),
) {
    let dashers: Vec<Entity> = dashed.read().map(|d| d.0).collect();
    let now = time.elapsed_secs();
    let motes = void_motes();
    for (e, mut d, k, mut c, mut anim) in &mut q {
        let (pos, half, facing) = (k.body.pos, k.body.half, k.loco.facing);
        let feet = pos.y - half.y;
        (d.hand, d.cast) = hands_at(pos, facing, d.doing);
        if !d.started {
            d.started = true;
            d.ball = d.hand + Vec2::new(-facing * 30.0, -10.0);
            d.ball_prev = d.ball;
            d.chains = vec![Links::new(8, 2.6), Links::new(6, 2.6), Links::new(9, 2.6)];
        }
        for r in d.ready.iter_mut() {
            *r -= DT;
        }
        let target = hunted.iter().filter(|(_, _, t)| t.hunted()).map(|(e, tk, _)| (e, tk.body.pos, tk.body.half)).filter(|(_, p, _)| p.distance(pos) < AGGRO).min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)));
        if target.is_some() && !d.shrieked {
            d.shrieked = true;
            sounds.write(crate::sound::PlaySound::at("snarl", pos).pitch(1.7).volume(1.0));
            sounds.write(crate::sound::PlaySound::at("groan", pos).pitch(1.5).volume(0.9));
        }
        let doing = d.doing;
        let mut next = doing;
        match doing {
            Doing::Carry => {
                anim.force = None;
                hang_ball(&sim, &mut d);
                if let Some((_, tp, _)) = target {
                    let off = tp - pos;
                    let dist = off.length();
                    let want = if off.x.abs() < SWEEP_NEAR.0 && off.y.abs() < SWEEP_NEAR.1 && d.ready[SWEEP_I] <= 0.0 {
                        Some(Next::Draw)
                    } else if tp.distance(d.hand) < THROW + SLAM_FROM && d.ready[SLAM_I] <= 0.0 && k.loco.grounded() {
                        Some(Next::Whirl)
                    } else if (VOLLEY_RANGE.0..VOLLEY_RANGE.1).contains(&dist) && d.ready[VOLLEY_I] <= 0.0 && d.ready[PULL_I] > 2.0 {
                        Some(Next::Ready)
                    } else if (PULL_RANGE.0..PULL_RANGE.1).contains(&dist) && d.ready[PULL_I] <= 0.0 {
                        Some(Next::Gather)
                    } else {
                        None
                    };
                    if let Some(w) = want {
                        next = haul_then(&d, w, &mut sounds);
                    }
                }
            }
            Doing::Whirl(t) => {
                anim.play("raise");
                let spin = t * (6.0 + 12.0 * t / WHIRL.0) * facing;
                d.ball_prev = d.ball;
                d.ball = d.hand + Vec2::from_angle(spin + std::f32::consts::FRAC_PI_2) * WHIRL.1 * (t / 0.25).min(1.0);
                if ((t + DT) / 0.2) as u32 != (t / 0.2) as u32 {
                    sounds.write(crate::sound::PlaySound::at("swing_heavy", d.ball).pitch(0.45 + t * 0.3));
                }
                next = if t + DT >= WHIRL.0 {
                    // Where it comes down: where they stand, no further than
                    // its chain reaches, on the ground there.
                    let mut to = target.map_or(d.hand + Vec2::new(facing * CHAIN, -CHAIN), |(_, tp, th)| tp - Vec2::Y * (th.y - BALL));
                    if to.distance(d.hand) > THROW {
                        to = d.hand + (to - d.hand).normalize() * THROW;
                    }
                    if let Some(drop) = (0..80).map(|n| to - Vec2::Y * n as f32).find(|&p| solid(&sim, p - Vec2::Y * BALL)) {
                        to = drop;
                    }
                    Doing::Slam(0.0, d.ball, to)
                } else {
                    Doing::Whirl(t + DT)
                };
            }
            Doing::Slam(t, from, to) => {
                anim.play("slam");
                let u = ((t + DT) / SLAM.0).min(1.0).powf(1.6);
                d.ball_prev = d.ball;
                d.ball = from.lerp(to, u) + Vec2::Y * SLAM.1 * 4.0 * u * (1.0 - u);
                let parry = if u > 0.3 { parried(&blades, d.ball_prev, d.ball, e) } else { None };
                next = if let Some((at, _)) = parry {
                    knocked_back(&mut d, at, &mut sounds, &mut sparks, &mut trauma, &mut flares)
                } else if u >= 1.0 {
                    // It comes down: the ground's broken, what's near thrown.
                    for (te, tk, t) in &hunted {
                        if !t.hunted() {
                            continue;
                        }
                        let near = ((tk.body.pos - to).abs() - tk.body.half).max(Vec2::ZERO).length();
                        if near <= QUAKE.0 {
                            let away = Vec2::new((tk.body.pos.x - to.x).signum(), 0.9).normalize();
                            let f = 1.0 - near / QUAKE.0 * 0.5;
                            hits.write(crate::combat::Hit { by: Some(e), target: te, damage: QUAKE.1 * f, harm: Harm::Blunt, knock: away * QUAKE.2, stun: 0.4, at: to, dir: away, weight: 2.6, crit: false });
                        }
                    }
                    sim.queue(platypus_sim::WorldEdit::Explode { center: CellPos::from_world(to.x, to.y - BALL), radius: CRATER.0, power: CRATER.1 });
                    let dust = crate::magic::runes::Emitter {
                        count: 0.0,
                        life: (0.4, 1.2),
                        colors: vec![(170, 160, 140), (120, 112, 100), (80, 74, 68)],
                        speed: 260.0,
                        spread: 1.4,
                        gravity: 260.0,
                        drag: 2.0,
                        size: 1.7,
                        jitter: 0.0,
                        glow: false,
                    };
                    sparks.emit(&dust, 200, to - Vec2::Y * BALL, Vec2::Y, Vec2::ZERO);
                    trauma.0 = (trauma.0 + 0.75).min(1.0);
                    sounds.write(crate::sound::PlaySound::at("slam", to).volume(1.0));
                    sounds.write(crate::sound::PlaySound::at("boom", to).pitch(0.5).volume(0.8));
                    Doing::Rest(0.0)
                } else {
                    Doing::Slam(t + DT, from, to)
                };
            }
            Doing::Rest(t) => {
                anim.play("slam");
                d.ball_prev = d.ball;
                next = if t + DT >= REST {
                    d.ready[SLAM_I] = SLAM_EVERY;
                    Doing::Carry
                } else {
                    Doing::Rest(t + DT)
                };
            }
            Doing::Draw(t, from) => {
                anim.play("sweep");
                let back = Vec2::new(d.hand.x - facing * CHAIN * 0.8, feet + BALL);
                d.ball_prev = d.ball;
                d.ball = from.lerp(back, ((t + DT) / DRAW).min(1.0));
                next = if t + DT >= DRAW {
                    sounds.write(crate::sound::PlaySound::at("swing_heavy", d.hand).pitch(0.4));
                    d.struck.clear();
                    Doing::Sweep(0.0)
                } else {
                    Doing::Draw(t + DT, from)
                };
            }
            Doing::Sweep(t) => {
                anim.play("sweep");
                let u = ((t + DT) / SWEEP.0).min(1.0);
                let a = (SWEEP.1 + (SWEEP.2 - SWEEP.1) * u).to_radians();
                let mut at = d.hand + Vec2::new(a.cos() * facing, a.sin()) * CHAIN * 0.9;
                at.y = at.y.max(feet + BALL);
                let was = d.ball;
                d.ball_prev = d.ball;
                d.ball = at;
                if let Some((bat, _)) = parried(&blades, was, at, e) {
                    next = knocked_back(&mut d, bat, &mut sounds, &mut sparks, &mut trauma, &mut flares);
                    d.doing = next;
                    continue;
                }
                for (te, tk, t) in &hunted {
                    if t.hunted() && !d.struck.contains(&te) && ((tk.body.pos - at).abs() - tk.body.half).max_element() <= BALL {
                        d.struck.push(te);
                        let dir = Vec2::new(facing, 0.35).normalize();
                        hits.write(crate::combat::Hit { by: Some(e), target: te, damage: SWEEP_HIT.0, harm: Harm::Blunt, knock: dir * SWEEP_HIT.1, stun: 0.35, at, dir, weight: 2.0, crit: false });
                        sounds.write(crate::sound::PlaySound::at("clank", at).pitch(0.55).volume(1.0));
                    }
                }
                next = if u >= 1.0 {
                    d.ready[SWEEP_I] = SWEEP_EVERY;
                    Doing::Carry
                } else {
                    Doing::Sweep(t + DT)
                };
            }
            Doing::Gather(t) => {
                anim.play("reach");
                hang_ball(&sim, &mut d);
                // Motes drawn in to its hand, more as it gathers; a hum.
                let n = 2 + (t / GATHER * 6.0) as usize;
                for i in 0..n {
                    let dir = Vec2::from_angle(t * 7.0 + i as f32 * 2.399);
                    sparks.emit(&motes, 1, d.cast + dir * 34.0, -dir, Vec2::ZERO);
                }
                if ((t + DT) / 0.35) as u32 != (t / 0.35) as u32 {
                    sounds.write(crate::sound::PlaySound::at("cast", d.cast).pitch(0.3 + t * 0.2).volume(0.7));
                }
                next = if t + DT >= GATHER {
                    flares.write(crate::fx::Flare { at: d.cast, color: [0.7, 0.3, 1.4], life: 0.5 });
                    sounds.write(crate::sound::PlaySound::at("boom", d.cast).pitch(0.35).volume(0.6));
                    match target {
                        Some((who, ..)) => Doing::Pull(0.0, who, 0),
                        None => {
                            d.ready[PULL_I] = PULL_EVERY;
                            Doing::Carry
                        }
                    }
                } else {
                    Doing::Gather(t + DT)
                };
            }
            Doing::Pull(t, who, done) => {
                anim.play("reach");
                hang_ball(&sim, &mut d);
                let near = hunted.get(who).ok().map(|(_, tk, _)| tk.body.pos);
                let moving = hunted.get(who).ok().map_or(Vec2::ZERO, |(_, tk, _)| tk.body.vel);
                let close = near.is_some_and(|p| (p.x - pos.x).abs() < SWEEP_NEAR.0 * 0.8);
                let (throb, tug, speed, rest) = TUG;
                let tugging = t >= throb && t < throb + tug;
                if dashers.contains(&who) && tugging && let Ok(mut ec) = commands.get_entity(who) {
                    // (Slipped this one; the next will come.)
                    ec.remove::<Tugged>();
                    sounds.write(crate::sound::PlaySound::at("cast", d.cast).pitch(1.4));
                }
                if close || near.is_none() || done >= TUGS {
                    if let Ok(mut ec) = commands.get_entity(who) {
                        ec.remove::<Tugged>();
                    }
                    d.ready[PULL_I] = PULL_EVERY;
                    // Pulled in: its flail, at once (near: swept; in reach:
                    // brought down on them; still out of reach: it comes on).
                    let swept = near.is_some_and(|p| (p.x - pos.x).abs() < SWEEP_NEAR.0);
                    let reached = near.is_some_and(|p| p.distance(d.hand) < THROW + SLAM_FROM);
                    next = if swept {
                        haul_then(&d, Next::Draw, &mut sounds)
                    } else if reached {
                        haul_then(&d, Next::Whirl, &mut sounds)
                    } else {
                        Doing::Carry
                    };
                } else {
                    let p = near.expect("near");
                    // Its throb: the void in its hand swelling (the tell).
                    if t < DT {
                        flares.write(crate::fx::Flare { at: d.cast, color: [0.6, 0.25, 1.2], life: throb });
                        sounds.write(crate::sound::PlaySound::at("cast", d.cast).pitch(0.35 + done as f32 * 0.1).volume(0.8));
                    }
                    if t < throb {
                        let dir = Vec2::from_angle(t * 11.0 + done as f32);
                        sparks.emit(&motes, 3, d.cast + dir * 20.0, -dir, Vec2::ZERO);
                    }
                    // The tug: hard, toward its hand.
                    if tugging && !dashers.contains(&who) {
                        if t - DT < throb {
                            // The yank: toward its hand and up off their feet,
                            // whatever way they were going.
                            let dir = (d.cast - p).normalize_or_zero();
                            let want = Vec2::new(dir.x * YANK.0, (dir.y * YANK.0).max(0.0) + YANK.1);
                            recoils.write(crate::combat::Recoil { who, add: want - moving, pogo: None, dive: None });
                            sounds.write(crate::sound::PlaySound::at("boom", d.cast).pitch(0.3).volume(0.7));
                            trauma.0 = (trauma.0 + 0.3).min(1.0);
                        }
                        commands.entity(who).insert(Tugged { to: d.cast, speed, left: 0.1 });
                        let to = (d.cast - p).normalize_or_zero();
                        for _ in 0..2 {
                            sparks.emit(&motes, 2, p + to.perp() * ((t * 31.0).sin() * 7.0), to, Vec2::ZERO);
                        }
                    }
                    next = if t + DT >= throb + tug + rest { Doing::Pull(0.0, who, done + 1) } else { Doing::Pull(t + DT, who, done) };
                }
            }
            Doing::Stagger(t) => {
                anim.play("hurt");
                hang_ball(&sim, &mut d);
                next = if t + DT >= STAGGER { Doing::Carry } else { Doing::Stagger(t + DT) };
            }
            Doing::Haul(t, then) => {
                anim.play("sweep");
                // In along its chain, faster and faster, off the ground.
                let to = d.hand - d.ball;
                let speed = HAUL.1 * (0.4 + t / HAUL.2).min(1.0);
                d.ball_prev = d.ball;
                d.ball += to.normalize_or_zero() * (speed * DT).min(to.length());
                next = if d.ball.distance(d.hand) <= HAUL.0 || t + DT >= HAUL.2 { start(&d, then, &mut sounds) } else { Doing::Haul(t + DT, then) };
            }
            Doing::Ready(t, round, rounds) => {
                anim.play("reach");
                let charge = if round == 0 { READY } else { READY_AGAIN };
                hang_ball(&sim, &mut d);
                if let Some((_, tp, _)) = target {
                    d.aim = (tp - d.cast).normalize_or(Vec2::X * facing);
                }
                // The void swelling in its palm: motes spiralling in, more and
                // faster; its light throbbing.
                let f = t / charge;
                for i in 0..(2 + (f * 8.0) as usize) {
                    let dir = Vec2::from_angle(t * (6.0 + f * 10.0) + i as f32 * 1.7);
                    sparks.emit(&motes, 1, d.cast + dir * (26.0 - f * 10.0), (-dir + dir.perp() * 0.6).normalize(), Vec2::ZERO);
                }
                if ((t + DT) / 0.15) as u32 != (t / 0.15) as u32 {
                    flares.write(crate::fx::Flare { at: d.cast, color: [0.4 + f * 0.6, 0.15, 0.8 + f * 0.8], life: 0.2 });
                }
                if ((t + DT) / 0.3) as u32 != (t / 0.3) as u32 {
                    sounds.write(crate::sound::PlaySound::at("cast", d.cast).pitch(0.5 + t * 0.5).volume(0.6));
                }
                next = if t + DT >= charge {
                    // Three at once, fanned.
                    if let Some(i) = book.spells.iter().position(|s| s.id == "dread_void") {
                        for k in [-1.0f32, 0.0, 1.0] {
                            let way = d.aim.rotate(Vec2::from_angle((k * FAN).to_radians()));
                            let from = d.cast + way * 5.0;
                            casts.write(crate::magic::CastRequest { caster: e, spell: i, from, toward: from + way * 200.0, alt: false });
                        }
                    }
                    sounds.write(crate::sound::PlaySound::at("cast", d.cast).pitch(0.4).volume(1.0));
                    sounds.write(crate::sound::PlaySound::at("boom", d.cast).pitch(0.6).volume(0.5));
                    flares.write(crate::fx::Flare { at: d.cast, color: [1.2, 0.5, 2.0], life: 0.35 });
                    if round + 1 < rounds {
                        Doing::Ready(0.0, round + 1, rounds)
                    } else {
                        d.ready[VOLLEY_I] = VOLLEY_EVERY;
                        Doing::Carry
                    }
                } else {
                    Doing::Ready(t + DT, round, rounds)
                };
            }
        }
        // Struck where it lies (or hangs): batted along, the harder the
        // heavier the blow.
        if matches!(next, Doing::Carry | Doing::Gather(_) | Doing::Pull(..) | Doing::Ready(..) | Doing::Stagger(_)) && now >= d.batted {
            if let Some((at, knock)) = parried(&blades, d.ball, d.ball, e) {
                let from = blades.0.iter().find(|b| b.knock == knock).and_then(|b| hunted.get(b.who).ok()).map_or(at, |(_, wk, _)| wk.body.pos);
                let mut dir = (d.ball - from).normalize_or(Vec2::X);
                dir.y = dir.y.max(0.5);
                let v = (d.ball - d.ball_prev) / DT + dir.normalize() * knock * BAT.0;
                d.ball_prev = d.ball - v * DT;
                d.batted = now + BAT.1;
                // (A clink, louder for a heavier blow.)
                sounds.write(crate::sound::PlaySound::at("clink", d.ball).volume((0.5 + knock / 600.0).min(1.0)));
                sparks.emit(&clang(), 12, d.ball, -dir, Vec2::ZERO);
            }
        }
        // Swung fast (dragged, flung), it hurts what it hits.
        if matches!(next, Doing::Carry | Doing::Gather(_) | Doing::Pull(..) | Doing::Ready(..)) {
            let speed = (d.ball - d.ball_prev).length() / DT;
            if speed > FAST.0 {
                for (te, tk, t) in &hunted {
                    if t.hunted() && ((tk.body.pos - d.ball).abs() - tk.body.half).max_element() <= BALL {
                        let dir = (tk.body.pos - d.ball).normalize_or(Vec2::Y);
                        hits.write(crate::combat::Hit { by: Some(e), target: te, damage: FAST.1, harm: Harm::Blunt, knock: dir * 240.0, stun: 0.2, at: d.ball, dir, weight: 1.3, crit: false });
                    }
                }
            }
        }
        d.doing = next;
        // (Busy, it stands.)
        if !matches!(next, Doing::Carry) {
            c.0.move_x = 0.0;
            c.0.jump = false;
        }
        // Its chains: at its belt (two) and its other wrist.
        let tops = [pos + Vec2::new(-4.0 * facing, -3.0), pos + Vec2::new(3.0 * facing, -3.0), pos + Vec2::new(-12.5 * facing, -9.0)];
        for (l, top) in d.chains.iter_mut().zip(tops) {
            l.swing(top, 300.0);
        }
    }
}

/// The flail, its chains and its void, in front of the player, behind it.
#[derive(Resource)]
pub struct FlailCanvas(Canvas);

impl Default for FlailCanvas {
    fn default() -> Self {
        FlailCanvas(Canvas::new("Flails", 11.15))
    }
}

/// A filled disc of `col` (`r` cells) at `c`, darker at its rim.
fn disc(px: &mut crate::canvas::Pixels, c: Vec2, r: f32, col: [u8; 4], rim: [u8; 4]) {
    let n = r.ceil() as i32;
    for y in -n..=n {
        for x in -n..=n {
            let o = Vec2::new(x as f32, y as f32);
            if o.length() <= r {
                let p = (c + o).floor().as_ivec2();
                px.put(p.x, p.y, if o.length() > r - 1.0 { rim } else { col });
            }
        }
    }
}

/// The ball (iron, lit from above, spikes), its chain from the hand
/// (sagging when slack), the chains it wears; its orbs swelling.
fn draw(mut commands: Commands, mut canvas: ResMut<FlailCanvas>, mut images: ResMut<Assets<Image>>, camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>, q: Query<&Dread>, mut sprites: CanvasSprites) {
    let Ok((cam, loader)) = camera.single() else { return };
    let any = q.iter().any(|d| d.started);
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, any) else { return };
    let (light, dark) = ([150, 152, 164, 255], [70, 72, 82, 255]);
    for d in &q {
        if !d.started {
            continue;
        }
        for l in &d.chains {
            draw_links(&mut px, l.points(), light, dark);
        }
        // The chain: sagging with the slack.
        let slack = (CHAIN - d.hand.distance(d.ball)).max(0.0);
        let mid = (d.hand + d.ball) / 2.0 - Vec2::Y * (slack * 0.6).min(26.0);
        let pts: Vec<Vec2> = (0..=CHAIN_LINKS)
            .map(|i| {
                let t = i as f32 / CHAIN_LINKS as f32;
                d.hand.lerp(mid, t).lerp(mid.lerp(d.ball, t), t)
            })
            .collect();
        draw_links(&mut px, &pts, light, dark);
        // The ball: spikes, then iron, lit from above.
        let c = d.ball;
        for s in 0..12 {
            let a = s as f32 / 12.0 * std::f32::consts::TAU + 0.3;
            for r in 0..5 {
                let p = (c + Vec2::from_angle(a) * (BALL + r as f32)).floor().as_ivec2();
                px.put(p.x, p.y, if r < 3 { [110, 112, 124, 255] } else { [184, 186, 200, 255] });
            }
        }
        disc(&mut px, c, BALL, [62, 64, 74, 255], [28, 28, 34, 255]);
        disc(&mut px, c + Vec2::new(-2.5, 3.0), BALL * 0.38, [124, 126, 140, 255], [96, 98, 110, 255]);
        // The void swelling in its palm: a dark sphere, a ring about it,
        // lightning crackling off it.
        if let Doing::Ready(t, round, _) = d.doing {
            let f = (t / if round == 0 { READY } else { READY_AGAIN }).min(1.0);
            let r = 2.0 + f * 6.0;
            disc(&mut px, d.cast, r + 1.5, [120, 70, 210, 255], [200, 160, 255, 255]);
            disc(&mut px, d.cast, r, [14, 4, 26, 255], [60, 24, 110, 255]);
            let ring = r + 4.0 + (t * 14.0).sin() * 1.5;
            for i in 0..24 {
                let p = (d.cast + Vec2::from_angle(i as f32 / 24.0 * std::f32::consts::TAU + t * 3.0) * ring).floor().as_ivec2();
                px.put(p.x, p.y, [180, 130, 255, 255]);
            }
            // (Crackling: jagged rays, new every few frames.)
            let seed = (t * 20.0) as u32;
            for k in 0..(2 + (f * 4.0) as u32) {
                let mut a = (seed.wrapping_mul(2654435761).wrapping_add(k * 977) % 628) as f32 / 100.0;
                let mut p = d.cast + Vec2::from_angle(a) * r;
                for step in 0..(4 + (f * 5.0) as u32) {
                    a += (((seed + k * 31 + step * 7) % 11) as f32 - 5.0) * 0.12;
                    let q = p + Vec2::from_angle(a) * 2.5;
                    for s in 0..=3 {
                        let c = p.lerp(q, s as f32 / 3.0).floor().as_ivec2();
                        px.put(c.x, c.y, [235, 220, 255, 255]);
                    }
                    p = q;
                }
            }
        }
        // The void in its hand, gathering or drawing.
        if let Doing::Gather(t) | Doing::Pull(t, ..) = d.doing {
            let f = match d.doing {
                Doing::Gather(_) => (t / GATHER).min(1.0),
                // (Each tug's throb: swelling, then spent.)
                _ => 0.6 + 0.4 * (t / TUG.0).min(1.0),
            };
            disc(&mut px, d.cast, 2.0 + f * 5.0, [16, 6, 28, 255], [150, 90, 240, 255]);
        }
    }
}

/// Dead: its cloak, to wear.
fn on_death(mut commands: Commands, mut died: MessageReader<Died>, items: Option<Res<crate::hands::items::Items>>) {
    for d in died.read() {
        if d.kind != "dread_knight" {
            continue;
        }
        if let Some(items) = items.as_ref()
            && let Some(id) = items.id("dread_cloak")
        {
            crate::hands::spawn_drop(&mut commands, items, d.body.pos, crate::hands::items::Stack::new(id, 1));
        }
    }
}
