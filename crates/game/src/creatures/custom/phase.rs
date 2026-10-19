//! Phase spiders (and their spiderlings): they slip out of the world. Every
//! so often while hunting (and now and then when struck hard) one
//! shimmers, is gone, ripples through the rock toward you (a faint trail
//! of motes shows where), and comes out beside or behind you, biting as
//! it lands (`ambush`: one of its moves). Light is what they fear: a
//! flashlight's beam, a flash, a bright lamp near snaps one out of phase,
//! senseless a moment; a phase spiderling caught in a beam or a flash
//! (`pops`) bursts.

use bevy::prelude::*;
use serde::Deserialize;

use super::watcher::{clear, solid};
use super::{CustomCreature, CustomSet};
use crate::creatures::moves::{Moves, Occupied};
use crate::creatures::{Health, Kinematics, Phasing, Team};
use crate::world::{SimWorld, TickSet};

/// Its settings (from `params`) and what it's doing.
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Phase {
    /// Seconds between its blinks while it hunts (from..to).
    pub every: (f32, f32),
    /// It comes out this far from you (cells, from..to).
    pub near: (f32, f32),
    /// Through the rock this fast (cells/s).
    pub speed: f32,
    /// What it does as it comes out (a move of its own; "": nothing).
    pub ambush: String,
    /// Struck, the chance it slips away.
    pub dodge: f32,
    /// Snapped out of phase by light: senseless this long (s).
    pub dazed: f32,
    /// Caught in a beam or a flash, it bursts (a spiderling).
    pub pops: bool,
    /// Its shimmer's colour.
    pub color: (u8, u8, u8),
    #[serde(skip)]
    doing: Doing,
    #[serde(skip)]
    wait: f32,
    /// Where it's going, through the rock, and at whom.
    #[serde(skip)]
    to: Vec2,
    #[serde(skip)]
    quarry: Option<Entity>,
    /// Struck: it slips away (blinks however near it is).
    #[serde(skip)]
    slip: bool,
}

impl Default for Phase {
    fn default() -> Self {
        Phase {
            every: (4.0, 7.0),
            near: (26.0, 40.0),
            speed: 260.0,
            ambush: String::new(),
            dodge: 0.3,
            dazed: 1.2,
            pops: false,
            color: (190, 140, 255),
            doing: Doing::Here,
            wait: 2.0,
            to: Vec2::ZERO,
            quarry: None,
            slip: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Doing {
    /// In the world.
    #[default]
    Here,
    /// Shimmering, going (s in).
    Fading(f32),
    /// Out of the world, going through rock to `to` (s in).
    Through(f32),
}

impl Phase {
    /// What it's doing, in a word (readouts, scenarios).
    pub fn doing(&self) -> &'static str {
        match self.doing {
            Doing::Here => "here",
            Doing::Fading(_) => "fading",
            Doing::Through(_) => "through",
        }
    }
}

impl CustomCreature for Phase {
    const NAME: &'static str = "phase";

    fn build(app: &mut App) {
        app.add_systems(FixedUpdate, think.in_set(CustomSet::Think)).add_systems(FixedUpdate, struck.in_set(TickSet::Bodies));
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
/// Shimmering this long before it's gone; through the rock this long at
/// most (then out the nearest open air past it).
const FADE: f32 = 0.35;
const THROUGH: f32 = 1.8;
/// It blinks at what it hunts within this far (cells).
const HUNT: f32 = 360.0;
/// It bites as it comes out if you're within this far.
const AMBUSH: f32 = 60.0;
/// A lamp this near (cells) and this bright (its brightest channel); a
/// flash this near and bright.
const LAMP: (f32, f32) = (34.0, 1.2);
const FLASH: (f32, f32) = (90.0, 1.0);

fn roll(e: Entity, t: f32, salt: u64) -> f32 {
    platypus_sim::rng::hash(&[e.to_bits(), (t * 1000.0) as u64, salt]) as f32 / u64::MAX as f32
}

/// Room for a body of `half` here: nothing solid at its middle or corners.
fn room(sim: &SimWorld, at: Vec2, half: Vec2) -> bool {
    [Vec2::ZERO, Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0), Vec2::new(-1.0, 1.0), Vec2::new(1.0, 1.0), Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0)]
        .iter()
        .all(|&o| !solid(sim, at + o * (half - 0.5)))
}

/// Where it comes out by its quarry at `q` (it's at `from`): behind them
/// first (on the far side from it), then above, then its own side; settled
/// onto the ground under the spot if there's ground near; open air, a
/// clear line to them.
fn landing(sim: &SimWorld, from: Vec2, q: Vec2, half: Vec2, d: f32) -> Option<Vec2> {
    let back = if from.x < q.x { 1.0 } else { -1.0 };
    let tries = [Vec2::new(back * d, 0.0), Vec2::new(back * d, 18.0), Vec2::new(back * d * 0.6, 30.0), Vec2::new(-back * d, 0.0), Vec2::new(-back * d, 18.0), Vec2::new(0.0, 36.0)];
    tries.iter().find_map(|&o| {
        let mut at = q + o;
        if !room(sim, at, half) {
            return None;
        }
        // (Down onto the ground under it, if it's near.)
        for _ in 0..40 {
            if solid(sim, at - Vec2::Y * (half.y + 1.0)) || !room(sim, at - Vec2::Y, half) {
                break;
            }
            at.y -= 1.0;
        }
        clear(sim, at, q).then_some(at)
    })
}

/// In bright light: a flashlight's beam on it, a flash near, a bright lamp.
fn bright(sim: &SimWorld, at: Vec2, cones: &crate::light::Cones, flashes: &[Vec2], lamps: &[Vec2]) -> bool {
    let beam = cones.0.iter().any(|&(from, dir, half, range, _)| {
        let d = at - from;
        d.length() < range && d.normalize_or_zero().angle_to(dir).abs() < half && clear(sim, from, at)
    });
    beam || flashes.iter().any(|&f| f.distance(at) < FLASH.0 && clear(sim, f, at)) || lamps.iter().any(|&l| l.distance(at) < LAMP.0)
}

/// Each tick: when to blink, where to, through the rock, and out again;
/// what light does to it.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn think(
    mut commands: Commands,
    sim: Res<SimWorld>,
    time: Res<Time>,
    cones: Res<crate::light::Cones>,
    mut flares: MessageReader<crate::fx::Flare>,
    lamps: Query<(&GlobalTransform, &crate::light::LightSource), (Without<crate::magic::Spell>, Without<Phase>)>,
    mut q: Query<(Entity, &mut Phase, &mut Kinematics, &mut Health, &mut Visibility, Option<&mut Moves>), Without<crate::creatures::Dormant>>,
    hunted: Query<(Entity, &Kinematics, &Team), Without<Phase>>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    let now = time.elapsed_secs();
    let flashes: Vec<Vec2> = flares.read().filter(|f| f.color.iter().copied().fold(0.0f32, f32::max) >= FLASH.1).map(|f| f.at).collect();
    let lamps: Vec<Vec2> = lamps.iter().filter(|(_, l)| l.color.iter().copied().fold(0.0f32, f32::max) >= LAMP.1).map(|(t, _)| t.translation().truncate()).collect();
    // (Its own flashes, sent after: what it reads are the world's.)
    let mut flash: Vec<crate::fx::Flare> = Vec::new();
    for (e, mut p, mut k, mut health, mut vis, moves) in &mut q {
        let pos = k.body.pos;
        let half = k.body.half;
        let shimmer = shimmer(p.color);
        // A spiderling in a beam or a flash: it bursts.
        if p.pops && health.hp > 0.0 {
            let beam = bright(&sim, pos, &cones, &flashes, &[]);
            if beam {
                debug!("{e:?} bursts in the light ({} beams, {} flashes)", cones.0.len(), flashes.len());
                sparks.emit(&shimmer, 40, pos, Vec2::Y, Vec2::ZERO);
                flash.push(crate::fx::Flare { at: pos, color: color_of(p.color, 0.8), life: 0.2 });
                sounds.write(crate::sound::PlaySound::at("snap", pos).pitch(1.6).volume(0.6));
                // (Out of the world or in it: gone.)
                commands.entity(e).try_remove::<crate::combat::Invulnerable>();
                *vis = Visibility::Inherited;
                health.hp = 0.0;
                continue;
            }
        }
        let quarry = hunted
            .iter()
            .filter(|(_, _, t)| t.hunted())
            .map(|(o, ok, _)| (o, ok.body.pos))
            .filter(|(_, at)| at.distance(pos) < HUNT)
            .min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)));
        match p.doing {
            Doing::Here => {
                p.wait -= DT;
                let busy = moves.as_ref().is_some_and(|m| m.busy());
                // (Not when it's beside them already, unless it's slipping
                // away from a blow.)
                if let Some((who, at)) = quarry
                    && p.wait <= 0.0
                    && (p.slip || at.distance(pos) > p.near.1 + 12.0)
                    && !busy
                    && k.loco.state != platypus_physics::MoveState::Stunned
                {
                    let d = p.near.0 + (p.near.1 - p.near.0) * roll(e, now, 1);
                    if let Some(to) = landing(&sim, pos, at, half, d) {
                        p.to = to;
                        p.quarry = Some(who);
                        p.doing = Doing::Fading(0.0);
                        p.slip = false;
                        commands.entity(e).try_insert(Occupied);
                        sounds.write(crate::sound::PlaySound::at("phase_out", pos));
                    } else {
                        p.wait = 0.8;
                    }
                }
            }
            Doing::Fading(t) => {
                // (Held still, shimmering.)
                k.body.vel = Vec2::ZERO;
                sparks.emit(&shimmer, 3, pos + Vec2::new((roll(e, now, 2) - 0.5) * half.x * 2.0, (roll(e, now, 3) - 0.5) * half.y * 2.0), Vec2::Y, Vec2::ZERO);
                if bright(&sim, pos, &cones, &flashes, &lamps) {
                    snap(&mut commands, e, &mut p, &mut k, &mut vis, &mut sparks, &mut sounds);
                    continue;
                }
                if t + DT >= FADE {
                    // Gone: an afterimage of motes where it was.
                    sparks.emit(&shimmer, 34, pos, Vec2::Y, Vec2::ZERO);
                    flash.push(crate::fx::Flare { at: pos, color: color_of(p.color, 0.6), life: 0.3 });
                    *vis = Visibility::Hidden;
                    commands.entity(e).try_insert(Phasing);
                    p.doing = Doing::Through(0.0);
                } else {
                    p.doing = Doing::Fading(t + DT);
                }
            }
            Doing::Through(t) => {
                // Untouchable out of the world.
                commands.entity(e).try_insert(crate::combat::Invulnerable { left: 0.15, hp: health.hp, dodge: false });
                // (After them, if they've gone far from where it meant to
                // come out.)
                if let Some((_, at)) = quarry.filter(|(w, _)| Some(*w) == p.quarry)
                    && at.distance(p.to) > p.near.1 + 30.0
                    && let Some(to) = landing(&sim, pos, at, half, p.near.0)
                {
                    p.to = to;
                }
                let d = p.to - pos;
                let step = p.speed * DT;
                k.body.vel = if d.length() > step { d.normalize() * p.speed } else { d / DT };
                // (A ripple of motes where it goes.)
                if roll(e, now, 4) < 0.6 {
                    sparks.emit(&shimmer, 1, pos, Vec2::Y, Vec2::ZERO);
                }
                let open = room(&sim, pos, half);
                if open && bright(&sim, pos, &cones, &flashes, &lamps) {
                    snap(&mut commands, e, &mut p, &mut k, &mut vis, &mut sparks, &mut sounds);
                    continue;
                }
                let there = d.length() <= step;
                if (there || t > THROUGH) && open {
                    // Out: a burst, and (in reach) it bites as it lands.
                    k.body.pos = if there { p.to } else { pos };
                    k.prev_pos = k.body.pos;
                    k.body.vel = Vec2::ZERO;
                    *vis = Visibility::Inherited;
                    commands.entity(e).try_remove::<Phasing>().try_remove::<Occupied>();
                    sparks.emit(&shimmer, 30, k.body.pos, Vec2::Y, Vec2::ZERO);
                    flash.push(crate::fx::Flare { at: k.body.pos, color: color_of(p.color, 0.9), life: 0.25 });
                    sounds.write(crate::sound::PlaySound::at("phase_in", k.body.pos));
                    if let (Some(mut m), Some((who, at))) = (moves, quarry)
                        && at.distance(k.body.pos) < AMBUSH
                        && let Some(i) = m.ids().iter().position(|id| *id == p.ambush)
                    {
                        k.loco.facing = if at.x < k.body.pos.x { -1.0 } else { 1.0 };
                        m.force(i, who, at - k.body.pos);
                    }
                    p.doing = Doing::Here;
                    p.wait = p.every.0 + (p.every.1 - p.every.0) * roll(e, now, 5);
                } else {
                    p.doing = Doing::Through(t + DT);
                }
            }
        }
    }
    if !flash.is_empty() {
        commands.queue(move |w: &mut World| {
            for f in flash {
                w.write_message(f);
            }
        });
    }
}

/// Snapped out of phase by light: back in the world where it is, senseless
/// a moment.
fn snap(commands: &mut Commands, e: Entity, p: &mut Phase, k: &mut Kinematics, vis: &mut Visibility, sparks: &mut crate::vfx::Sparks, sounds: &mut MessageWriter<crate::sound::PlaySound>) {
    let pos = k.body.pos;
    *vis = Visibility::Inherited;
    commands.entity(e).try_remove::<Phasing>().try_remove::<Occupied>();
    sparks.emit(&shimmer(p.color), 24, pos, Vec2::Y, Vec2::ZERO);
    sounds.write(crate::sound::PlaySound::at("phase_in", pos).pitch(0.6));
    let k = &mut *k;
    k.loco.knock(&mut k.body, Vec2::new(0.0, -30.0), p.dazed);
    p.doing = Doing::Here;
    p.wait = p.every.1;
}

/// Struck: now and then it slips away (out of reach: it blinks to the far
/// side of you, or off).
fn struck(mut hits: MessageReader<crate::combat::Hit>, mut q: Query<(Entity, &mut Phase)>, time: Res<Time>) {
    let now = time.elapsed_secs();
    for h in hits.read() {
        let Ok((e, mut p)) = q.get_mut(h.target) else { continue };
        if p.doing == Doing::Here && p.dodge > 0.0 && roll(e, now, 6) < p.dodge {
            p.wait = p.wait.min(0.05);
            p.slip = true;
        }
    }
}

/// Its own light, `k` (0..1) of its colour: never as bright as the flash
/// that snaps it (`FLASH`), or its own would.
fn color_of((r, g, b): (u8, u8, u8), k: f32) -> [f32; 3] {
    let k = k.min(0.95);
    [r as f32 / 255.0 * k, g as f32 / 255.0 * k, b as f32 / 255.0 * k]
}

/// Motes of it: its colour, lit, drifting up and fading.
fn shimmer((r, g, b): (u8, u8, u8)) -> crate::magic::runes::Emitter {
    let pale = |c: u8| c.saturating_add(70);
    crate::magic::runes::Emitter {
        count: 1.0,
        life: (0.3, 0.8),
        colors: vec![(pale(r), pale(g), pale(b)), (r, g, b), (r / 2, g / 2, b / 2)],
        speed: 40.0,
        spread: std::f32::consts::PI,
        gravity: -25.0,
        drag: 3.0,
        size: 1.2,
        jitter: 6.0,
        glow: true,
    }
}
