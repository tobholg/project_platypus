//! Olmer the Defiler: a giant musclehead (blond, parted down the middle,
//! glasses, a tank top, arms like hams) with a giant barbell, three blue
//! plates on each end. He walks at you, the bar across his shoulders, and
//! when he stands about he does his reps (overhead and back down).
//!
//! - Near, he presses the bar overhead and holds it there, shaking (the
//!   tell), then drops into a squat and slams it down flat in front of
//!   him: both plates crater the ground, and anything along the bar's
//!   length, or near either end, is thrown (jump it, or be well away).
//! - A little further, he takes the bar in one fist and twirls it like a
//!   staff, walking at you, the plates battering what they meet. A blade
//!   that meets a plate mid-twirl parries it: he staggers (an opening).
//!
//! The bar's middle follows the pose (the art's `bar` anchor); it's drawn
//! behind him on his shoulders, in front of him when he swings it.

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind};
use serde::Deserialize;

use super::{CustomCreature, CustomSet};
use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprites};
use crate::creatures::body::animation::{Animator, body_at};
use crate::creatures::{Controls, Harm, Kinematics, Team};
use crate::world::{ChunkLoader, SimWorld};

#[derive(Component, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Olmer {
    #[serde(skip)]
    doing: Doing,
    /// The bar: its middle, which way it lies (radians), whether it's in
    /// front of him.
    #[serde(skip)]
    bar: Vec2,
    #[serde(skip)]
    angle: f32,
    #[serde(skip)]
    front: bool,
    /// Where the bar came down from (the slam).
    #[serde(skip)]
    from: Vec2,
    /// Its attacks' waits: slam, twirl.
    #[serde(skip)]
    ready: [f32; 2],
    /// Whom the plates have struck, and until when they're spared.
    #[serde(skip)]
    struck: Vec<(Entity, f32)>,
    #[serde(skip)]
    shouted: bool,
    #[serde(skip)]
    started: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Doing {
    /// Walking or standing about (his reps), the bar on his shoulders.
    #[default]
    Carry,
    /// The bar pressed overhead, shaking (the slam's tell).
    Raise(f32),
    /// Bringing it down.
    Slam(f32),
    /// Squatting where it came down, a moment.
    Rest(f32),
    /// Taking it in one fist, starting it turning (the twirl's tell).
    Wind(f32),
    /// Twirling it, walking at you.
    Twirl(f32),
    /// Getting his breath back after a twirl.
    Puff(f32),
    /// Parried: staggered, a moment.
    Stagger(f32),
}

impl CustomCreature for Olmer {
    const NAME: &'static str = "olmer";

    fn build(app: &mut App) {
        app.init_resource::<BarBehind>()
            .init_resource::<BarFront>()
            .add_systems(FixedUpdate, think.in_set(CustomSet::Think))
            .add_systems(Update, (draw_behind, draw_front));
    }
}

impl Olmer {
    /// What he's doing, in a word (a scenario's log).
    pub fn doing(&self) -> &'static str {
        match self.doing {
            Doing::Carry => "carry",
            Doing::Raise(_) => "raise",
            Doing::Slam(_) => "slam",
            Doing::Rest(_) => "rest",
            Doing::Wind(_) => "wind",
            Doing::Twirl(_) => "twirl",
            Doing::Puff(_) => "puff",
            Doing::Stagger(_) => "stagger",
        }
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
/// The bar: half its length; where its plates sit along it (from the
/// middle: the stack's near and far faces), how tall a plate is (half).
const BAR: f32 = 57.0;
const PLATES: (f32, f32) = (34.0, 46.5);
const PLATE: f32 = 15.0;
/// A plate stack's middle along the bar, and how near it must come to hit.
const STACK: f32 = 40.0;
const STACK_R: f32 = 14.0;
const SLAM_I: usize = 0;
const TWIRL_I: usize = 1;
/// Hunting what's this near (cells).
const AGGRO: f32 = 400.0;
/// The slam: held overhead this long, brought down in this long, squatting
/// this long after; then he waits. He slams what's this near (across, up).
const RAISE: f32 = 0.8;
const DOWN: f32 = 0.16;
const REST: f32 = 0.9;
const SLAM_EVERY: f32 = 3.4;
const SLAM_NEAR: (f32, f32) = (70.0, 50.0);
/// What the slam does: how far from the bar (cells), how much, how hard
/// (across, up); each plate's crater.
const QUAKE: (f32, f32, f32, f32) = (22.0, 42.0, 300.0, 420.0);
const CRATER: (i32, u8) = (6, 30);
/// The twirl: wound up this long, twirled this long (turning this fast,
/// rising to that: rad/s), walking at you this fast (of his run); puffing
/// after. How much a plate does, how hard; once a target in this long.
/// He twirls at what's this near.
const WIND: f32 = 0.5;
const TWIRL: (f32, f32, f32, f32) = (1.8, 7.0, 15.0, 0.9);
const PUFF: f32 = 0.6;
const TWIRL_HIT: (f32, f32, f32) = (24.0, 380.0, 0.35);
const TWIRL_EVERY: f32 = 4.0;
const TWIRL_NEAR: (f32, f32) = (150.0, 60.0);
/// Parried: he staggers this long.
const STAGGER: f32 = 1.2;

/// Where a frame's anchor (`name`, in its picture) is in the world.
fn anchor(anim: &Animator, k: &Kinematics, name: &str, frame: &str) -> Option<Vec2> {
    let def = &anim.def;
    let rig = def.rig.as_ref()?;
    let index = if frame.is_empty() { anim.shown } else { rig.index(frame)? };
    let &(px, py) = rig.anchors.get(name)?.get(&index)?;
    let (fw, fh) = (def.sprite.frame.0 as f32, def.sprite.frame.1 as f32);
    let local = Vec2::new((px as f32 + 0.5 - fw / 2.0) * k.loco.facing, -(py as f32 + 0.5 - fh / 2.0)) * def.scale;
    Some(k.body.pos + body_at(def, k) + local)
}

/// Points along the bar (and how near each must come to hit: a plate's
/// half, or the bar's), at its angle now and at three angles between it and
/// the last (it turns fast).
fn swept(bar: Vec2, was: f32, now: f32) -> impl Iterator<Item = (Vec2, f32)> {
    (1..=4).flat_map(move |i| {
        let along = Vec2::from_angle(was + (now - was) * i as f32 / 4.0);
        (-28..=28).map(move |n| {
            let u = n as f32 * 2.0;
            let r = if (PLATES.0..=PLATES.1).contains(&u.abs()) { PLATE } else { 2.0 };
            (bar + along * u, r)
        })
    })
}

/// Its two plate stacks' middles.
fn stacks(bar: Vec2, angle: f32) -> [Vec2; 2] {
    let along = Vec2::from_angle(angle);
    [bar - along * STACK, bar + along * STACK]
}

fn solid(sim: &SimWorld, p: Vec2) -> bool {
    sim.world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| !c.is_air() && matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder))
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

fn dust() -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter {
        count: 0.0,
        life: (0.4, 1.1),
        colors: vec![(170, 160, 140), (120, 112, 100), (80, 74, 68)],
        speed: 220.0,
        spread: 1.3,
        gravity: 260.0,
        drag: 2.0,
        size: 1.6,
        jitter: 0.0,
        glow: false,
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn think(
    mut sim: ResMut<SimWorld>,
    mut q: Query<(Entity, &mut Olmer, &Kinematics, &mut Controls, &mut Animator)>,
    hunted: Query<(Entity, &Kinematics, &Team), Without<Olmer>>,
    (blades, time): (Res<crate::combat::LiveBlades>, Res<Time>),
    (mut hits, mut sounds): (MessageWriter<crate::combat::Hit>, MessageWriter<crate::sound::PlaySound>),
    (mut sparks, mut trauma, mut flares): (ResMut<crate::vfx::Sparks>, ResMut<crate::fx::Trauma>, MessageWriter<crate::fx::Flare>),
) {
    let now = time.elapsed_secs();
    for (e, mut o, k, mut c, mut anim) in &mut q {
        let (pos, facing) = (k.body.pos, k.loco.facing);
        let held = anchor(&anim, k, "bar", "").unwrap_or(pos + Vec2::Y * 20.0);
        if !o.started {
            o.started = true;
            o.bar = held;
        }
        for r in o.ready.iter_mut() {
            *r -= DT;
        }
        o.struck.retain(|(_, until)| *until > now);
        let target = hunted.iter().filter(|(_, _, t)| t.hunted()).map(|(e, tk, _)| (e, tk.body.pos)).filter(|(_, p)| p.distance(pos) < AGGRO).min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)));
        if target.is_some() && !o.shouted {
            o.shouted = true;
            sounds.write(crate::sound::PlaySound::at("snarl", pos).pitch(0.55).volume(1.0));
        }
        let doing = o.doing;
        let mut next = doing;
        match doing {
            Doing::Carry => {
                anim.force = None;
                o.bar = held;
                o.angle = 0.0;
                if let Some((_, tp)) = target {
                    let off = (tp - pos).abs();
                    if off.x < SLAM_NEAR.0 && off.y < SLAM_NEAR.1 && o.ready[SLAM_I] <= 0.0 && k.loco.grounded() {
                        sounds.write(crate::sound::PlaySound::at("groan", pos).pitch(0.55).volume(1.0));
                        next = Doing::Raise(0.0);
                    } else if off.x < TWIRL_NEAR.0 && off.y < TWIRL_NEAR.1 && o.ready[TWIRL_I] <= 0.0 && k.loco.grounded() {
                        sounds.write(crate::sound::PlaySound::at("snarl", pos).pitch(0.7).volume(0.9));
                        next = Doing::Wind(0.0);
                    }
                }
            }
            Doing::Raise(t) => {
                anim.play("raise");
                // Overhead, shaking harder as he holds it (the tell).
                let up = anchor(&anim, k, "bar", "raise").unwrap_or(held);
                let shake = (t / RAISE) * 1.5;
                o.bar = up + Vec2::new((t * 61.0).sin() * shake, (t * 47.0).cos() * shake * 0.5);
                o.angle = (t * 29.0).sin() * 0.03 * t / RAISE;
                next = if t + DT >= RAISE {
                    o.from = up;
                    sounds.write(crate::sound::PlaySound::at("swing_heavy", up).pitch(0.4).volume(1.0));
                    Doing::Slam(0.0)
                } else {
                    Doing::Raise(t + DT)
                };
            }
            Doing::Slam(t) => {
                anim.play("slam");
                let down = anchor(&anim, k, "bar", "slam").unwrap_or(pos);
                let u = ((t + DT) / DOWN).min(1.0);
                o.bar = o.from.lerp(down, u * u);
                o.angle = 0.0;
                next = if u >= 1.0 {
                    // It comes down: each plate on the ground, cratering it;
                    // what's along the bar or near its ends, thrown.
                    let ends = stacks(o.bar, 0.0).map(|p| {
                        let bottom = p - Vec2::Y * PLATE;
                        (0..30).map(|n| bottom - Vec2::Y * n as f32).find(|&q| solid(&sim, q)).unwrap_or(bottom)
                    });
                    for &g in &ends {
                        sim.queue(platypus_sim::WorldEdit::Explode { center: CellPos::from_world(g.x, g.y), radius: CRATER.0, power: CRATER.1 });
                        sparks.emit(&dust(), 120, g + Vec2::Y * 2.0, Vec2::Y, Vec2::ZERO);
                        sparks.emit(&clang(), 20, g + Vec2::Y * 4.0, Vec2::Y, Vec2::ZERO);
                    }
                    let (lo, hi) = (ends[0].x.min(ends[1].x) - STACK_R, ends[0].x.max(ends[1].x) + STACK_R);
                    let ground = (ends[0].y + ends[1].y) / 2.0;
                    for (te, tk, tt) in &hunted {
                        if !tt.hunted() {
                            continue;
                        }
                        let (tp, th) = (tk.body.pos, tk.body.half);
                        let gap_x = ((tp.x - th.x) - hi).max(lo - (tp.x + th.x)).max(0.0);
                        let gap_y = ((tp.y - th.y) - ground).max(0.0);
                        let near = Vec2::new(gap_x, gap_y).length();
                        if near <= QUAKE.0 {
                            let side = if tp.x == o.bar.x { facing } else { (tp.x - o.bar.x).signum() };
                            let f = 1.0 - near / QUAKE.0 * 0.5;
                            let knock = Vec2::new(side * QUAKE.2, QUAKE.3);
                            hits.write(crate::combat::Hit { target: te, damage: QUAKE.1 * f, harm: Harm::Blunt, knock, stun: 0.4, at: o.bar, dir: knock.normalize(), weight: 2.6, crit: false });
                        }
                    }
                    trauma.0 = (trauma.0 + 0.8).min(1.0);
                    sounds.write(crate::sound::PlaySound::at("slam", o.bar).volume(1.0));
                    sounds.write(crate::sound::PlaySound::at("boom", o.bar).pitch(0.5).volume(0.8));
                    sounds.write(crate::sound::PlaySound::at("clang", o.bar).pitch(0.5).volume(0.7));
                    Doing::Rest(0.0)
                } else {
                    Doing::Slam(t + DT)
                };
            }
            Doing::Rest(t) => {
                anim.play("slam");
                next = if t + DT >= REST {
                    o.ready[SLAM_I] = SLAM_EVERY;
                    // (Not straight into a twirl.)
                    o.ready[TWIRL_I] = o.ready[TWIRL_I].max(1.0);
                    Doing::Carry
                } else {
                    Doing::Rest(t + DT)
                };
            }
            Doing::Wind(t) => {
                anim.play("spin");
                o.bar = anchor(&anim, k, "bar", "spin").unwrap_or(held);
                // Starting it turning, slow, top first toward you.
                o.angle = -facing * (t / WIND).powi(2) * 1.2;
                next = if t + DT >= WIND {
                    o.struck.clear();
                    Doing::Twirl(0.0)
                } else {
                    Doing::Wind(t + DT)
                };
            }
            Doing::Twirl(t) => {
                anim.play("spin");
                o.bar = anchor(&anim, k, "bar", "spin").unwrap_or(held);
                let (len, w0, w1, walk) = TWIRL;
                let w = w0 + (w1 - w0) * (t / (len * 0.4)).min(1.0);
                let was = o.angle;
                o.angle -= facing * w * DT;
                // A whoosh each half turn.
                if (o.angle / std::f32::consts::PI).floor() != (was / std::f32::consts::PI).floor() {
                    sounds.write(crate::sound::PlaySound::at("swing_heavy", o.bar).pitch(0.5 + w / w1 * 0.3).volume(0.9));
                }
                c.0.move_x = facing * walk;
                let ends = stacks(o.bar, o.angle);
                let ends_was = stacks(o.bar, was);
                // A blade meeting a plate: parried.
                let parry = blades.0.iter().filter(|b| b.who != e).find_map(|b| b.cells.iter().find(|cell| ends.iter().chain(ends_was.iter()).any(|p| p.distance(**cell) <= STACK_R + 1.0)).copied());
                if let Some(at) = parry {
                    sounds.write(crate::sound::PlaySound::at("parry", at).volume(1.0));
                    sounds.write(crate::sound::PlaySound::at("clang", at).pitch(0.6).volume(0.8));
                    sparks.emit(&clang(), 40, at, (at - o.bar).normalize_or(Vec2::Y), Vec2::ZERO);
                    flares.write(crate::fx::Flare { at, color: [1.6, 1.5, 1.2], life: 0.25 });
                    trauma.0 = (trauma.0 + 0.4).min(1.0);
                    o.ready[TWIRL_I] = TWIRL_EVERY;
                    next = Doing::Stagger(0.0);
                } else {
                    // What the bar meets, swept from where it was (plates
                    // fat, the bar thin), struck once in a while.
                    for (te, tk, tt) in &hunted {
                        if !tt.hunted() || o.struck.iter().any(|(s, _)| *s == te) {
                            continue;
                        }
                        let Some(p) = swept(o.bar, was, o.angle).find(|&(p, r)| ((tk.body.pos - p).abs() - tk.body.half).max(Vec2::ZERO).length() <= r).map(|(p, _)| p) else { continue };
                        o.struck.push((te, now + TWIRL_HIT.2));
                        // Thrown the way the bar's going there, and up.
                        let spin = (p - o.bar).perp() * -facing;
                        let dir = (spin.normalize_or(Vec2::X * facing) + Vec2::Y * 0.6).normalize();
                        hits.write(crate::combat::Hit { target: te, damage: TWIRL_HIT.0, harm: Harm::Blunt, knock: dir * TWIRL_HIT.1, stun: 0.3, at: p, dir, weight: 2.0, crit: false });
                        sounds.write(crate::sound::PlaySound::at("clank", p).pitch(0.5).volume(1.0));
                    }
                    // A plate through the ground: sparks off it.
                    for p in ends {
                        let low = p - Vec2::Y * PLATE * 0.6;
                        if solid(&sim, low) && (now * 30.0) as i32 % 2 == 0 {
                            sparks.emit(&clang(), 3, low + Vec2::Y * 2.0, Vec2::new(-facing, 0.6).normalize(), Vec2::ZERO);
                        }
                    }
                    next = if t + DT >= len {
                        o.ready[TWIRL_I] = TWIRL_EVERY;
                        o.ready[SLAM_I] = o.ready[SLAM_I].max(0.8);
                        Doing::Puff(0.0)
                    } else {
                        Doing::Twirl(t + DT)
                    };
                }
            }
            Doing::Puff(t) => {
                anim.play("hurt");
                o.bar = held;
                o.angle = 0.0;
                next = if t + DT >= PUFF { Doing::Carry } else { Doing::Puff(t + DT) };
            }
            Doing::Stagger(t) => {
                anim.play("hurt");
                o.bar = held + Vec2::new((t * 40.0).sin() * 1.5, 0.0);
                o.angle = (t * 9.0).sin() * 0.25 * (1.0 - t / STAGGER);
                next = if t + DT >= STAGGER { Doing::Carry } else { Doing::Stagger(t + DT) };
            }
        }
        o.doing = next;
        o.front = matches!(next, Doing::Slam(_) | Doing::Rest(_) | Doing::Wind(_) | Doing::Twirl(_));
        // (Busy, he stands; twirling, he walks at you.)
        if !matches!(next, Doing::Carry | Doing::Twirl(_)) {
            c.0.move_x = 0.0;
        }
        if !matches!(next, Doing::Carry) {
            c.0.jump = false;
        }
    }
}

/// The bar on his shoulders (behind him), and swung (in front of him).
#[derive(Resource)]
pub struct BarBehind(Canvas);

#[derive(Resource)]
pub struct BarFront(Canvas);

impl Default for BarBehind {
    fn default() -> Self {
        BarBehind(Canvas::new("Barbells behind", 11.05))
    }
}

impl Default for BarFront {
    fn default() -> Self {
        BarFront(Canvas::new("Barbells", 11.17))
    }
}

/// The bar's colour at a point (`u` along it from its middle, `v` across).
fn bar_pixel(u: f32, v: f32) -> Option<[u8; 4]> {
    let (u, v) = (u.abs(), v);
    let va = v.abs();
    if u > BAR {
        return None;
    }
    // Three plates, a cell apart: blue, lit along the top and the outer face.
    for n in 0..3 {
        let a = PLATES.0 + n as f32 * 4.2;
        let b = a + 3.3;
        let round = if va > PLATE - 1.5 { 0.6 } else { 0.0 };
        if u >= a + round && u <= b - round && va <= PLATE {
            return Some(if v > PLATE - 1.5 || u > b - 0.8 {
                [96, 156, 236, 255]
            } else if v < -PLATE + 1.5 {
                [18, 44, 110, 255]
            } else if va < 1.6 {
                [26, 62, 150, 255]
            } else {
                [36, 88, 196, 255]
            });
        }
    }
    // The collar, outside the plates.
    if (PLATES.1 + 0.6..PLATES.1 + 2.2).contains(&u) && va <= 2.6 {
        return Some(if v > 1.2 { [140, 142, 150, 255] } else { [70, 72, 80, 255] });
    }
    // The sleeves (thicker), then the shaft (knurled near his hands).
    let thick = if u >= PLATES.0 - 1.5 { 1.9 } else { 1.3 };
    if va <= thick {
        return Some(if v > 0.3 {
            [214, 216, 224, 255]
        } else if u < PLATES.0 - 1.5 && (u * 2.0) as i32 % 3 == 0 {
            [96, 98, 108, 255]
        } else {
            [150, 152, 162, 255]
        });
    }
    None
}

/// The bars that are (or aren't) in front, onto that canvas.
#[allow(clippy::too_many_arguments)]
fn draw_bars(canvas: &mut Canvas, front: bool, commands: &mut Commands, images: &mut Assets<Image>, sprites: &mut CanvasSprites, camera: &Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>, q: &Query<&Olmer>) {
    let Ok((cam, loader)) = camera.single() else { return };
    let any = q.iter().any(|o| o.started && o.front == front);
    let Some(mut px) = canvas.frame(commands, images, sprites, cam.translation().truncate(), loader.half_extent, any) else { return };
    for o in q.iter().filter(|o| o.started && o.front == front) {
        let (along, across) = (Vec2::from_angle(o.angle), Vec2::from_angle(o.angle).perp());
        let n = (BAR + 2.0) as i32;
        let c = o.bar.floor().as_ivec2();
        for y in -n..=n {
            for x in -n..=n {
                let p = Vec2::new((c.x + x) as f32 + 0.5, (c.y + y) as f32 + 0.5) - o.bar;
                if let Some(col) = bar_pixel(p.dot(along), p.dot(across)) {
                    px.put(c.x + x, c.y + y, col);
                }
            }
        }
    }
}

fn draw_behind(mut commands: Commands, mut canvas: ResMut<BarBehind>, mut images: ResMut<Assets<Image>>, camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>, q: Query<&Olmer>, mut sprites: CanvasSprites) {
    draw_bars(&mut canvas.0, false, &mut commands, &mut images, &mut sprites, &camera, &q);
}

fn draw_front(mut commands: Commands, mut canvas: ResMut<BarFront>, mut images: ResMut<Assets<Image>>, camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>, q: Query<&Olmer>, mut sprites: CanvasSprites) {
    draw_bars(&mut canvas.0, true, &mut commands, &mut images, &mut sprites, &camera, &q);
}
