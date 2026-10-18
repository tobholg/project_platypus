//! Gnashers: a floating mouth from the deep sea's dark, a watcher's cousin
//! that bites instead of burning: a bloated orb that's mostly jaw, tendrils
//! trailing under it. It hangs in the night air like a watcher (held up by
//! its code), keeping its distance (its brain: `Range`), a lure glowing
//! over its crown, its jaw working.
//!
//! - **Snap**: near you, its jaw drops (the tell, short) and it darts in
//!   and bites.
//! - **Lunge**: further off, it draws back, its jaws gaping wide, teeth
//!   chattering, turning after you, slowly, then holding still (the tell,
//!   long: get out of the line, or behind something); then it shoots along
//!   that line and snaps shut on what's in it. After, it drifts a while,
//!   spent: open to you.
//!
//! Its hurt is all teeth: pierce.

use bevy::prelude::*;
use serde::Deserialize;

use super::watcher::{clear, ground};
use super::{CustomCreature, CustomSet, frame_anchor};
use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprites};
use crate::cloth::Links;
use crate::creatures::body::animation::Animator;
use crate::creatures::{Controls, Harm, Kinematics, Team};
use crate::world::{ChunkLoader, SimWorld};

#[derive(Component, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Gnasher {
    #[serde(skip)]
    doing: Doing,
    /// Its waits: snap, lunge.
    #[serde(skip)]
    ready: [f32; 2],
    /// The way it lunges.
    #[serde(skip)]
    aim: Vec2,
    /// Its tendrils, trailing under it.
    #[serde(skip)]
    tendrils: Vec<Links>,
    #[serde(skip)]
    clock: f32,
    #[serde(skip)]
    started: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Doing {
    /// Hovering, keeping its distance, its jaw working.
    #[default]
    Hover,
    /// Its jaw dropped to snap (the tell), then darting in.
    Gape(f32),
    Dart(f32, bool),
    /// Drawn back, gaping, turning after you, then still (the lunge's tell).
    Draw(f32),
    /// Shooting along its line; whether it's bitten yet.
    Lunge(f32, bool),
    /// Spent: drifting.
    Spent(f32),
}

impl CustomCreature for Gnasher {
    const NAME: &'static str = "gnasher";

    fn build(app: &mut App) {
        app.init_resource::<GnasherCanvas>().add_systems(FixedUpdate, think.in_set(CustomSet::Think)).add_systems(Update, draw);
    }
}

impl Gnasher {
    /// What it's doing, in a word (a scenario's log).
    pub fn doing(&self) -> &'static str {
        match self.doing {
            Doing::Hover => "hover",
            Doing::Gape(_) => "gape",
            Doing::Dart(..) => "dart",
            Doing::Draw(_) => "draw",
            Doing::Lunge(..) => "lunge",
            Doing::Spent(_) => "spent",
        }
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
const SNAP_I: usize = 0;
const LUNGE_I: usize = 1;
/// Hunting what's this near (cells).
const AGGRO: f32 = 340.0;
/// Hovering this high over the ground (its feet, cells), bobbing this much.
const HOVER: (f32, f32) = (30.0, 4.0);
/// The snap: at what's this near; its tell; darting this fast this long;
/// its bite (damage, knock); then it waits.
const SNAP_RANGE: f32 = 48.0;
const GAPE: f32 = 0.3;
const DART: (f32, f32) = (300.0, 0.2);
const SNAP_BITE: (f32, f32) = (14.0, 260.0);
const SNAP_EVERY: f32 = 1.8;
/// The lunge: at what's from this near to this far; its tell this long,
/// drawing back this fast, turning after you at this (rad/s) till this
/// long before it goes (then still); shooting this fast, at most this long;
/// its bite (damage, knock, stun); spent this long after; then it waits.
const LUNGE_RANGE: (f32, f32) = (35.0, 230.0);
const DRAW: f32 = 0.95;
const DRAW_BACK: f32 = 35.0;
const FOLLOW: f32 = 2.2;
const LOCK: f32 = 0.3;
const LUNGE: (f32, f32) = (470.0, 0.5);
const LUNGE_BITE: (f32, f32, f32) = (24.0, 480.0, 0.35);
const SPENT: f32 = 0.85;
const LUNGE_EVERY: f32 = 3.5;

/// Where it goes for: their chest (so its body, hung under its maw, clears
/// the ground on the way).
const CHEST: f32 = 6.0;

/// Whether its maw's on them (their body, a little round).
fn on(maw: Vec2, k: &Kinematics) -> bool {
    ((maw - k.body.pos).abs() - k.body.half).max(Vec2::ZERO).length() <= 5.0
}

#[allow(clippy::type_complexity)]
fn think(
    sim: Res<SimWorld>,
    mut q: Query<(Entity, &mut Gnasher, &mut Kinematics, &mut Controls, &mut Animator)>,
    hunted: Query<(Entity, &Kinematics, &Team), Without<Gnasher>>,
    (mut hits, mut sounds, mut trauma): (MessageWriter<crate::combat::Hit>, MessageWriter<crate::sound::PlaySound>, ResMut<crate::fx::Trauma>),
) {
    for (_e, mut o, mut k, mut c, mut anim) in &mut q {
        let (pos, half) = (k.body.pos, k.body.half);
        if !o.started {
            o.started = true;
            o.ready = [1.0, 2.5];
            o.aim = Vec2::X;
            o.tendrils = vec![Links::new(8, 2.6), Links::new(10, 2.6), Links::new(7, 2.6)];
        }
        o.clock += DT;
        for r in o.ready.iter_mut() {
            *r -= DT;
        }
        let maw = frame_anchor(&anim, &k, "maw", "").unwrap_or(pos);
        let target = hunted.iter().filter(|(_, _, t)| t.hunted()).filter(|(_, tk, _)| tk.body.pos.distance(pos) < AGGRO).min_by(|a, b| a.1.body.pos.distance(pos).total_cmp(&b.1.body.pos.distance(pos)));
        let tp = target.map(|(_, tk, _)| tk.body.pos + Vec2::Y * CHEST);
        // Bites what its maw's on (once a dart, once a lunge).
        let mut bite = |o_aim: Vec2, (damage, knock, stun): (f32, f32, f32)| -> bool {
            let Some((who, tk, _)) = target.filter(|(_, tk, _)| on(maw, tk)) else { return false };
            hits.write(crate::combat::Hit { target: who, damage, harm: Harm::Pierce, knock: (o_aim + Vec2::Y * 0.3).normalize_or_zero() * knock, stun, at: maw.lerp(tk.body.pos, 0.5), dir: o_aim, weight: 0.8, crit: false });
            sounds.write(crate::sound::PlaySound::at("chomp", maw).volume(1.0));
            true
        };
        let doing = o.doing;
        let mut next = doing;
        match doing {
            Doing::Hover => {
                if let Some(tp) = tp {
                    let dist = tp.distance(maw);
                    let seen = clear(&sim, maw, tp);
                    if o.ready[SNAP_I] <= 0.0 && seen && dist < SNAP_RANGE {
                        sounds.write(crate::sound::PlaySound::at("chatter", maw).pitch(1.0).volume(0.7));
                        next = Doing::Gape(0.0);
                    } else if o.ready[LUNGE_I] <= 0.0 && seen && (LUNGE_RANGE.0..LUNGE_RANGE.1).contains(&dist) {
                        o.aim = (tp - maw).normalize_or(Vec2::X);
                        sounds.write(crate::sound::PlaySound::at("groan", maw).pitch(1.6).volume(0.8));
                        next = Doing::Draw(0.0);
                    }
                }
            }
            Doing::Gape(t) => {
                if let Some(tp) = tp {
                    o.aim = (tp - maw).normalize_or(o.aim);
                }
                next = if t + DT >= GAPE {
                    sounds.write(crate::sound::PlaySound::at("snap", maw).pitch(0.8));
                    Doing::Dart(0.0, false)
                } else {
                    Doing::Gape(t + DT)
                };
            }
            Doing::Dart(t, bit) => {
                let bit = bit || bite(o.aim, (SNAP_BITE.0, SNAP_BITE.1, 0.2));
                next = if bit || t + DT >= DART.1 {
                    o.ready[SNAP_I] = SNAP_EVERY;
                    o.ready[LUNGE_I] = o.ready[LUNGE_I].max(1.2);
                    if !bit {
                        sounds.write(crate::sound::PlaySound::at("chomp", maw).volume(0.5).pitch(1.2));
                    }
                    Doing::Hover
                } else {
                    Doing::Dart(t + DT, bit)
                };
            }
            Doing::Draw(t) => {
                // Turning after them, slowly, then still.
                if let Some(tp) = tp
                    && t < DRAW - LOCK
                {
                    let want = (tp - maw).normalize_or(o.aim);
                    let turn = (want.to_angle() - o.aim.to_angle() + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                    o.aim = Vec2::from_angle(o.aim.to_angle() + turn.clamp(-FOLLOW * DT, FOLLOW * DT));
                }
                // Its teeth chattering, faster as it goes.
                let every = 0.16 - 0.1 * (t / DRAW);
                if (t / every) as u32 != ((t + DT) / every) as u32 {
                    sounds.write(crate::sound::PlaySound::at("chatter", maw).pitch(0.9 + t * 0.5).volume(0.55));
                }
                next = if t + DT >= DRAW {
                    sounds.write(crate::sound::PlaySound::at("snap", maw).pitch(0.55).volume(1.0));
                    trauma.0 = (trauma.0 + 0.12).min(1.0);
                    Doing::Lunge(0.0, false)
                } else {
                    Doing::Draw(t + DT)
                };
            }
            Doing::Lunge(t, bit) => {
                let bit = bit || bite(o.aim, LUNGE_BITE);
                // (Stopped short: struck rock or the ground, turned off its
                // line by it: no skimming along the floor after.)
                let stopped = t > 0.03 && (k.body.vel - o.aim * LUNGE.0).length() > LUNGE.0 * 0.2;
                next = if bit || stopped || t + DT >= LUNGE.1 {
                    if !bit {
                        sounds.write(crate::sound::PlaySound::at("chomp", maw).volume(0.6).pitch(0.8));
                    }
                    Doing::Spent(0.0)
                } else {
                    Doing::Lunge(t + DT, bit)
                };
            }
            Doing::Spent(t) => {
                next = if t + DT >= SPENT {
                    o.ready[LUNGE_I] = LUNGE_EVERY;
                    o.ready[SNAP_I] = o.ready[SNAP_I].max(0.6);
                    Doing::Hover
                } else {
                    Doing::Spent(t + DT)
                };
            }
        }
        o.doing = next;
        // Its face: shut darting and lunging (snapped), gaping in its tells,
        // working its jaw hovering.
        let clip = match next {
            Doing::Gape(_) | Doing::Draw(_) => "gape",
            Doing::Dart(_, false) | Doing::Lunge(_, false) => "gape",
            Doing::Dart(_, true) | Doing::Lunge(_, true) => "snap",
            Doing::Spent(_) => "ajar",
            Doing::Hover => "idle",
        };
        if anim.force.as_deref() != Some(clip) {
            anim.play(clip);
        }
        // Floating: held high over the ground, bobbing; facing what it's
        // after; its tells and its lunge steered here.
        let feet = pos.y - half.y;
        let over = ground(&sim, Vec2::new(pos.x, feet + 2.0), 120).map(|g| feet - g);
        let height = HOVER.0 + (o.clock * 1.1).sin() * HOVER.1;
        c.0.move_y = over.map_or(-0.5, |h| ((height - h) * 0.08).clamp(-1.0, 1.0));
        c.0.jump = false;
        match next {
            // (Facing its prey, backing off or not: its mouth to them.)
            Doing::Hover => {
                if let Some(tp) = tp {
                    c.0.aim = tp;
                }
            }
            Doing::Gape(_) | Doing::Draw(_) => {
                // Still, or drawing back from its line (the tell).
                let back = if matches!(next, Doing::Draw(_)) { -o.aim * DRAW_BACK } else { Vec2::ZERO };
                k.body.vel = k.body.vel.lerp(back, 0.2);
                c.0.move_x = 0.0;
                c.0.move_y = 0.0;
                c.0.aim = maw + o.aim * 60.0;
                // (A tremble, its jaws gaping.)
                if let Doing::Draw(t) = next {
                    k.body.pos.x += ((o.clock * 70.0).sin() * 0.35) * (t / DRAW);
                }
            }
            Doing::Dart(t, false) => {
                k.body.vel = o.aim * DART.0 * (1.0 - t / DART.1 * 0.5);
                c.0.move_x = o.aim.x;
                c.0.move_y = o.aim.y;
                c.0.aim = maw + o.aim * 60.0;
            }
            Doing::Lunge(_, false) => {
                k.body.vel = o.aim * LUNGE.0;
                c.0.move_x = o.aim.x;
                c.0.move_y = o.aim.y;
                c.0.aim = maw + o.aim * 60.0;
            }
            Doing::Dart(_, true) | Doing::Lunge(_, true) | Doing::Spent(_) => {
                // Spent: slowing, sinking a little, drifting.
                k.body.vel *= 0.86;
                c.0.move_x = 0.0;
                c.0.move_y = c.0.move_y.min(-0.2);
            }
        }
        // Its tendrils trailing under it (lifting and streaming behind as
        // it shoots along).
        for (i, l) in o.tendrils.iter_mut().enumerate() {
            if let Some(top) = frame_anchor(&anim, &k, ["t0", "t1", "t2"][i], "") {
                l.swing(top, 260.0);
            }
        }
    }
}

/// Its tendrils.
#[derive(Resource)]
pub struct GnasherCanvas(Canvas);

impl Default for GnasherCanvas {
    fn default() -> Self {
        GnasherCanvas(Canvas::new("Gnashers", 10.3))
    }
}

fn draw(mut commands: Commands, mut canvas: ResMut<GnasherCanvas>, mut images: ResMut<Assets<Image>>, camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>, q: Query<&Gnasher>, mut sprites: CanvasSprites) {
    let Ok((cam, loader)) = camera.single() else { return };
    let any = q.iter().any(|o| o.started);
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, any) else { return };
    for o in q.iter().filter(|o| o.started) {
        // Dark and fleshy, two cells thick near the orb, a pale sea-green
        // speck at each tip.
        for l in &o.tendrils {
            let pts = l.points();
            for i in 1..pts.len() {
                let (a, b) = (pts[i - 1], pts[i]);
                let n = (a.distance(b) * 2.0).ceil() as i32;
                for s in 0..=n {
                    let p = a.lerp(b, s as f32 / n.max(1) as f32).floor().as_ivec2();
                    px.put(p.x, p.y, [62, 54, 80, 255]);
                    if i < 4 {
                        px.put(p.x + 1, p.y, [38, 33, 50, 255]);
                    }
                }
            }
            if let Some(tip) = pts.last() {
                let p = tip.floor().as_ivec2();
                px.put(p.x, p.y, [120, 230, 210, 255]);
            }
        }
    }
}
