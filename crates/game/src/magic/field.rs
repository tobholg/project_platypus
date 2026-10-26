//! Fields (`Carrier::Field`): cast where the cursor is (up to the rock in
//! the way, unless it needs no line of sight), a field is a shape of bent
//! space lasting `lasts` seconds: a bubble (`radius`), or an updraft's
//! column (`radius` either side), rising from there to the rock above
//! (`LIFT_MOST` at most). It swells into being (`GROW`), and fades out in
//! its last second. A caster keeps `most` of a kind; another replaces the
//! oldest. No borders: each kind only bends the light, its own way, in the
//! warp pass (`warp::FieldWarp`, `assets/shaders/warp.wgsl`):
//!
//! - A repulsor (Force) is a bubble that throws out what walks into it
//!   (but the player and the player's side), as hard as it came and
//!   harder; an enemy's arrow or spell turns back the way it came, yours
//!   now. Each blow ripples it (thoom) and wears it down (`WEAR`). Like
//!   glass, it magnifies what's behind it.
//! - A slow field (Void): inside it, everything runs at an eighth of its
//!   speed (`SLOW`), you too: bodies, blows and swings, spells and arrows,
//!   what falls and flows, sparks of it in flight, the sounds it makes. It
//!   wavers like heat haze, smeared and greyed toward violet.
//! - An updraft (Gravity): it lifts what's in it (`LIFT_UP`: bodies, you to
//!   ride it), and carries sand, water and rubble up through it (`RISE`
//!   cells a tick) and out of its top, spilled to the side: a cave cleared.
//!   The picture streams upward in it.

use std::collections::HashMap;

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind, World};
use serde::Deserialize;

use crate::creatures::player::LocalPlayer;
use crate::creatures::{Kinematics, MoveStats, Team};
use crate::magic::runes::Emitter;
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;

/// Which field.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldKind {
    Repulse,
    Slow,
    Lift,
}

impl FieldKind {
    /// Its number in the warp pass.
    fn code(self) -> f32 {
        match self {
            FieldKind::Repulse => 1.0,
            FieldKind::Slow => 2.0,
            FieldKind::Lift => 3.0,
        }
    }
}

/// A field: which, where (a bubble's middle, a column's foot), how big (a
/// bubble's radius, half a column's width), how high a column reaches, how
/// long more and how old (s), whose.
#[derive(Clone, Copy, Debug)]
pub struct Field {
    pub kind: FieldKind,
    pub at: Vec2,
    pub radius: f32,
    pub top: f32,
    pub left: f32,
    pub age: f32,
    pub by: Entity,
}

impl Field {
    /// How far it's swollen into being (0..1).
    fn grown(&self) -> f32 {
        let k = (self.age / GROW).min(1.0);
        1.0 - (1.0 - k) * (1.0 - k)
    }

    fn size(&self) -> f32 {
        self.radius * self.grown()
    }

    /// A column's top, as far as it's shot up yet.
    fn reach(&self) -> f32 {
        self.at.y + (self.top - self.at.y) * self.grown()
    }

    /// How strongly it bends (fading out in its last second).
    fn strength(&self) -> f32 {
        (self.left / FADE).min(1.0) * self.grown()
    }

    fn contains(&self, p: Vec2) -> bool {
        match self.kind {
            FieldKind::Lift => (p.x - self.at.x).abs() <= self.size() && p.y >= self.at.y - 1.0 && p.y <= self.reach(),
            _ => p.distance(self.at) <= self.size(),
        }
    }

    /// Whether a box touches it.
    fn touches(&self, pos: Vec2, half: Vec2) -> bool {
        match self.kind {
            FieldKind::Lift => (pos.x - self.at.x).abs() <= self.size() + half.x && pos.y + half.y >= self.at.y - 1.0 && pos.y - half.y <= self.reach(),
            _ => ((pos - self.at).abs() - half).max(Vec2::ZERO).length() <= self.size(),
        }
    }

    /// The cells it covers, a column's top first.
    fn cells(&self) -> impl Iterator<Item = CellPos> + '_ {
        let r = self.size();
        let (x0, x1) = ((self.at.x - r).floor() as i32, (self.at.x + r).floor() as i32);
        let (y0, y1) = match self.kind {
            FieldKind::Lift => (self.at.y.floor() as i32, self.reach().floor() as i32),
            _ => ((self.at.y - r).floor() as i32, (self.at.y + r).floor() as i32),
        };
        (x0..=x1).flat_map(move |x| (y0..=y1).rev().map(move |y| CellPos::new(x, y))).filter(|p| self.contains(Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5)))
    }
}

/// The fields there are, and the ripples on repulsors (where, how old: s,
/// how hard).
#[derive(Resource, Default)]
pub struct Fields {
    fields: Vec<Field>,
    ripples: Vec<(Vec2, f32, f32)>,
}

impl Fields {
    fn kind_at(&self, p: Vec2) -> Option<FieldKind> {
        self.fields.iter().rev().find(|f| f.contains(p)).map(|f| f.kind)
    }

    fn any(&self, kind: FieldKind) -> bool {
        self.fields.iter().any(|f| f.kind == kind)
    }

    /// Whether a point is in a slow field (sounds made there drop).
    pub fn slow_at(&self, p: Vec2) -> bool {
        self.kind_at(p) == Some(FieldKind::Slow)
    }

    fn touching(&self, pos: Vec2, half: Vec2, kind: FieldKind) -> Option<usize> {
        self.fields.iter().position(|f| f.kind == kind && f.touches(pos, half))
    }

    /// The first repulsor along a path (a shot's this tick: a fast one
    /// would skip over its edge), which, and where.
    fn first_repulse(&self, from: Vec2, to: Vec2) -> Option<(usize, Vec2)> {
        let steps = from.distance(to).ceil().max(1.0) as usize;
        (0..=steps).find_map(|i| {
            let p = from.lerp(to, i as f32 / steps as f32);
            self.fields.iter().position(|f| f.kind == FieldKind::Repulse && f.contains(p)).map(|i| (i, p))
        })
    }

    /// A blow on repulsor `i` at `at`: it ripples, and wears down.
    fn struck(&mut self, i: usize, at: Vec2, hard: f32, wear: f32) {
        self.fields[i].left -= wear;
        if self.ripples.len() >= RIPPLES {
            self.ripples.remove(0);
        }
        self.ripples.push((at, 0.0, hard.clamp(0.3, 1.0)));
    }
}

/// Everything in a slow field runs at this share of its speed.
pub const SLOW: f32 = 0.125;
/// A field swells into being over this long (s); in its last second it
/// fades out.
const GROW: f32 = 0.3;
const FADE: f32 = 1.0;
/// An updraft lifts bodies toward this (cells/s), gripping this hard (a
/// share of the difference, a tick); a column reaches this high at most
/// (cells).
const LIFT_UP: f32 = 125.0;
const LIFT_GRIP: f32 = 0.16;
const LIFT_MOST: i32 = 160;
/// What's loose in an updraft rises this many cells a tick; out of its top
/// it's thrown aside this fast (cells a tick: across, up).
const RISE: usize = 2;
const SPILL: (f32, f32) = (1.1, 0.7);
/// Particles in an updraft are pushed up this much a tick (cells/tick²),
/// to this fast at most; rubble (the world's bodies) too.
const LIFT_PUSH: f32 = 0.55;
const LIFT_TOP: f32 = 1.6;
/// A repulsor throws what walks in back as hard as it came, and kicks it
/// this hard more (cells/s, over the root of its heft) and up; it can't
/// steer that long (s), and isn't thrown again sooner than this (s).
const KICK: f32 = 150.0;
const KICK_UP: f32 = 70.0;
const REELING: f32 = 0.35;
const AGAIN: f32 = 0.2;
/// Caught inside one (it opened on it): blasted out, this much faster
/// (cells/s, over the fourth root of its heft: a heavy thing flies too;
/// from a share at the rim to all of it in the middle), and up (a share of
/// that: it flies), reeling this long (s).
const BLAST: f32 = 700.0;
const BLASTED: f32 = 0.7;
const BLAST_UP: f32 = 0.55;
/// Each blow takes this long off a repulsor (s): a shot, something
/// thrown out of it.
const WEAR: (f32, f32) = (0.4, 0.15);
/// Ripples on fields at once at most, and how long one lasts (s).
const RIPPLES: usize = 8;
pub const RIPPLE_LIFE: f32 = 0.8;
/// Fields in the warp at once at most (the nearest the view's middle).
const SHOWN: usize = 8;

/// Something in a slow field: its time runs at this share (`SLOW`).
#[derive(Component, Clone, Copy, Debug)]
pub struct Slowed(pub f32);

/// The share of time something runs at (1: as ever).
pub fn share(slowed: Option<&Slowed>) -> f32 {
    slowed.map_or(1.0, |s| s.0)
}

/// What a field spell is (`Carrier::Field`).
pub struct Cast {
    pub kind: FieldKind,
    pub radius: f32,
    pub range: f32,
    pub lasts: f32,
    pub most: usize,
    pub through: bool,
}

/// Rock: what an updraft stops under and a cast stops at.
fn rock(world: &World, p: CellPos) -> bool {
    world.get(p).is_none_or(|c| !c.is_air() && world.materials().phys(c.material).kind == Kind::Static && c.flags & platypus_sim::cell::flags::LOOSE == 0)
}

/// A field cast by `caster` from `from` toward its aim (`dir`, `aim` away):
/// where it opens.
#[allow(clippy::too_many_arguments)]
pub fn open(world: &World, fields: &mut Fields, sparks: &mut crate::vfx::Sparks, caster: Entity, from: Vec2, dir: Vec2, aim: f32, cast: &Cast) -> Vec2 {
    let reach = aim.min(cast.range);
    let mut at = from + dir * reach;
    if !cast.through {
        for i in 1..=(reach * 2.0) as i32 {
            let p = from + dir * (i as f32 * 0.5);
            if rock(world, CellPos::from_world(p.x, p.y)) {
                at = p - dir * 0.6;
                break;
            }
        }
    }
    let top = if cast.kind == FieldKind::Lift {
        let foot = CellPos::from_world(at.x, at.y);
        (1..=LIFT_MOST).find(|&k| rock(world, foot.offset(0, k))).map_or(at.y + LIFT_MOST as f32, |k| foot.y as f32 + k as f32)
    } else {
        at.y
    };
    // (A caster's oldest of the kind goes, past how many it may keep.)
    let mine: Vec<usize> = fields.fields.iter().enumerate().filter(|(_, f)| f.by == caster && f.kind == cast.kind).map(|(i, _)| i).collect();
    if mine.len() >= cast.most.max(1) {
        fields.fields.remove(mine[0]);
    }
    fields.fields.push(Field { kind: cast.kind, at, radius: cast.radius, top, left: cast.lasts, age: 0.0, by: caster });
    let e = match cast.kind {
        FieldKind::Repulse => &*GLINT,
        FieldKind::Slow => &*HAZE,
        FieldKind::Lift => &*DRAFT,
    };
    sparks.emit(e, 24, at, Vec2::Y, Vec2::ZERO);
    at
}

/// Fields age, and go.
pub fn age(time: Res<Time>, mut fields: ResMut<Fields>) {
    let dt = time.delta_secs();
    fields.fields.retain_mut(|f| {
        f.age += dt;
        f.left -= dt;
        f.left > 0.0
    });
}

type Body<'a> = (Entity, &'a mut Kinematics, Option<&'a MoveStats>, Option<&'a Slowed>);

/// Before bodies move: what's in a slow field is slowed (its time's share,
/// `Slowed`), its spells and arrows too; what's in an updraft is lifted.
pub fn mark(mut commands: Commands, fields: Res<Fields>, mut bodies: Query<Body>, mut spells: Query<&mut super::Spell>, mut arrows: Query<(&mut crate::archery::Arrow, &Transform)>) {
    let (slow, lift) = (fields.any(FieldKind::Slow), fields.any(FieldKind::Lift));
    for (e, mut k, stats, slowed) in &mut bodies {
        let inside = slow && fields.touching(k.body.pos, k.body.half, FieldKind::Slow).is_some();
        match (inside, slowed.is_some()) {
            (true, false) => {
                commands.entity(e).try_insert(Slowed(SLOW));
            }
            (false, true) => {
                commands.entity(e).try_remove::<Slowed>();
            }
            _ => {}
        }
        if lift && fields.touching(k.body.pos, k.body.half, FieldKind::Lift).is_some() {
            // (Up toward the draft's speed, its weight held off: in a slow
            // field too, an eighth as fast.)
            let fall = stats.map_or(crate::props::GRAVITY, |s| s.0.gravity);
            let s = share(slowed);
            k.body.vel.y += ((LIFT_UP - k.body.vel.y) * LIFT_GRIP + fall * DT) * s;
        }
    }
    for mut s in &mut spells {
        let want = if slow && fields.slow_at(s.pos) { SLOW } else { 1.0 };
        if s.slow != want {
            s.slow = want;
        }
    }
    for (mut a, tf) in &mut arrows {
        let want = if slow && fields.slow_at(tf.translation.truncate()) { SLOW } else { 1.0 };
        if a.slow != want {
            a.slow = want;
        }
    }
}

type Walker<'a> = (Entity, &'a mut Kinematics, &'a crate::creatures::body::animation::Animator, Option<&'a Team>);

/// After bodies move: what walked into a repulsor is thrown back out, and
/// what was shot at you through one turns back, yours.
#[allow(clippy::too_many_arguments)]
pub fn repel(
    mut fields: ResMut<Fields>,
    mut walkers: Query<Walker, Without<LocalPlayer>>,
    casters: Query<&Kinematics, With<LocalPlayer>>,
    mut spells: Query<&mut super::Spell>,
    mut arrows: Query<(&mut crate::archery::Arrow, &mut Transform)>,
    teams: Query<&Team>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    time: Res<Time>,
    mut since: Local<HashMap<Entity, f32>>,
) {
    let dt = time.delta_secs();
    for t in since.values_mut() {
        *t += dt;
    }
    since.retain(|_, t| *t < AGAIN * 4.0);
    if !fields.any(FieldKind::Repulse) {
        return;
    }
    let ours = |t: Option<&Team>| t.is_some_and(|t| t.allied(Team::Player));
    for (e, mut k, anim, team) in &mut walkers {
        if ours(team) {
            continue;
        }
        let Some(i) = fields.touching(k.body.pos, k.body.half, FieldKind::Repulse) else { continue };
        let k = &mut *k;
        // (Stepped in this tick: back where it was. Already in it: it was
        // caught as it opened, and is blasted out.)
        let caught = fields.fields[i].touches(k.prev_pos, k.body.half);
        if !caught {
            k.body.pos = k.prev_pos;
        }
        if since.get(&e).is_some_and(|t| *t < AGAIN) {
            continue;
        }
        since.insert(e, 0.0);
        // Out, away from the bubble's middle (and its caster: one caught in
        // the middle goes away from them): what it came in with turned
        // back, and a kick.
        let f = fields.fields[i];
        let from_caster = casters.get(f.by).map_or(Vec2::ZERO, |c| (k.body.pos - c.body.pos).normalize_or_zero());
        let away = (k.body.pos - f.at + from_caster * f.size() * 0.3).normalize_or(-k.body.vel.normalize_or(Vec2::NEG_Y));
        let at = f.at + away * f.size();
        let v = k.body.vel;
        let back = if v.dot(away) < 0.0 { v - 2.0 * v.dot(away) * away } else { v };
        let heft = anim.def.heft.max(0.5).sqrt();
        let deep = (1.0 - k.body.pos.distance(f.at) / f.radius.max(1.0)).clamp(0.0, 1.0);
        let blast = if caught { BLAST * (0.4 + 0.6 * deep) } else { 0.0 };
        let flung = (away + Vec2::Y * BLAST_UP) * blast / heft.sqrt();
        k.loco.knock(&mut k.body, back + (away * KICK + Vec2::Y * KICK_UP) / heft + flung, if caught { BLASTED } else { REELING });
        let hard = (v.length() + KICK + blast) / 300.0;
        fields.struck(i, at, hard, WEAR.1);
        sparks.emit(&RIPPLE, RIPPLE.count as usize, at, away, Vec2::ZERO);
        sounds.write(crate::sound::PlaySound::at("thoom", at).volume(0.5 + 0.5 * hard.min(1.0)).pitch((1.0 / heft).clamp(0.6, 1.3)));
    }
    // An enemy's spell: back the way it came, its caster now the field's.
    for mut s in &mut spells {
        let Some((i, at)) = fields.first_repulse(s.prev, s.pos) else { continue };
        let by = fields.fields[i].by;
        if s.caster == by || ours(teams.get(s.caster).ok()) {
            continue;
        }
        s.pos = s.prev;
        s.vel = -s.vel;
        s.caster = by;
        fields.struck(i, at, 0.6, WEAR.0);
        sparks.emit(&RIPPLE, RIPPLE.count as usize / 2, at, s.vel.normalize_or(Vec2::Y), Vec2::ZERO);
        sounds.write(crate::sound::PlaySound::at("thoom", at).volume(0.6).pitch(1.3));
    }
    for (mut a, mut tf) in &mut arrows {
        if !a.flying() {
            continue;
        }
        let now = tf.translation.truncate();
        let Some((i, at)) = fields.first_repulse(now - a.vel * DT * a.slow, now) else { continue };
        let by = fields.fields[i].by;
        if a.shooter() == by || ours(a.team().as_ref()) {
            continue;
        }
        let back = a.turn_back(by);
        // (Back out of the bubble, where it met it.)
        tf.translation = (at + back.normalize_or_zero() * 2.0).extend(tf.translation.z);
        fields.struck(i, at, 0.5, WEAR.0);
        sparks.emit(&RIPPLE, RIPPLE.count as usize / 2, at, back.normalize_or(Vec2::Y), Vec2::ZERO);
        sounds.write(crate::sound::PlaySound::at("thoom", at).volume(0.5).pitch(1.4));
    }
}

/// Before the cells step: what's loose in an updraft rises (and spills out
/// of its top), and what's in a slow field moves only one tick in eight.
pub fn cells(mut sim: ResMut<SimWorld>, fields: Res<Fields>) {
    if fields.fields.iter().all(|f| f.kind == FieldKind::Repulse) {
        return;
    }
    let world = &mut sim.world;
    let mats = world.materials().clone();
    // (Marked as moved this coming tick: the step leaves them be.)
    let next = (world.tick() + 1) as u8;
    let loose = |c: &platypus_sim::Cell| {
        !c.is_air() && c.flags & platypus_sim::cell::flags::HELD == 0 && (matches!(mats.phys(c.material).kind, Kind::Powder | Kind::Liquid) || c.flags & platypus_sim::cell::flags::LOOSE != 0)
    };
    let open = |c: &platypus_sim::Cell| c.is_air() || mats.phys(c.material).kind == Kind::Gas;
    let in_lift = |p: CellPos| fields.fields.iter().any(|f| f.kind == FieldKind::Lift && f.contains(Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5)));
    let mut rng = platypus_sim::rng::Rng::seeded(&[world.tick(), 0x11F7]);
    for f in fields.fields.iter().filter(|f| f.kind == FieldKind::Lift) {
        // Top first, so a column of sand rises together.
        for p in f.cells() {
            let Some(c) = world.get(p).filter(|c| loose(c) && c.clock != next) else { continue };
            let mut at = p;
            for _ in 0..RISE {
                let up = at.offset(0, 1);
                let Some(above) = world.get(up).filter(open) else { break };
                if in_lift(up) {
                    let mut moved = c;
                    moved.clock = next;
                    moved.vy = 0;
                    world.set(up, moved);
                    world.set(at, above);
                    at = up;
                } else {
                    // Out of the top: thrown aside, landing where it may.
                    let side = if rng.chance(128) { 1.0 } else { -1.0 };
                    let spread = 0.6 + rng.next_u32() as f32 / u32::MAX as f32 * 0.8;
                    world.set(at, above);
                    world.emit(platypus_sim::Particle::new([up.x as f32 + 0.5, up.y as f32 + 0.5], [side * SPILL.0 * spread, SPILL.1], c, 120, platypus_sim::Landing::Settle));
                    break;
                }
            }
        }
    }
    // Rubble in an updraft (the world's bodies) rises too.
    if fields.any(FieldKind::Lift) {
        for b in world.bodies_mut() {
            if in_lift(CellPos::from_world(b.pos[0], b.pos[1])) {
                b.vel[1] = (b.vel[1] + LIFT_PUSH).min(LIFT_TOP);
            }
        }
    }
    // Slow: seven ticks in eight, what moves there waits.
    if !world.tick().is_multiple_of(8) {
        for f in fields.fields.iter().filter(|f| f.kind == FieldKind::Slow) {
            for p in f.cells() {
                let Some(mut c) = world.get(p) else { continue };
                if c.clock == next || (!mats.phys(c.material).active && c.flags & (platypus_sim::cell::flags::LOOSE | platypus_sim::cell::flags::BURNING) == 0 && c.heat == 0) {
                    continue;
                }
                c.clock = next;
                world.set(p, c);
            }
        }
    }
}

/// After the cells step: what flies in a slow field went an eighth as far
/// (and fell an eighth as much, and lives eight times as long); in an
/// updraft, it's carried up.
pub fn motes(mut sim: ResMut<SimWorld>, fields: Res<Fields>) {
    if fields.fields.iter().all(|f| f.kind == FieldKind::Repulse) {
        return;
    }
    let world = &mut sim.world;
    let tick = world.tick();
    for m in world.particles_mut() {
        match fields.kind_at(Vec2::new(m.pos[0], m.pos[1])) {
            Some(FieldKind::Slow) => {
                let undo = 1.0 - SLOW;
                m.pos[0] -= m.vel[0] * undo;
                m.pos[1] -= m.vel[1] * undo;
                m.vel[1] += platypus_sim::particles::GRAVITY * m.gravity * undo;
                if !tick.is_multiple_of(8) {
                    m.life = m.life.saturating_add(1);
                }
            }
            Some(FieldKind::Lift) => m.vel[1] = (m.vel[1] + LIFT_PUSH).min(LIFT_TOP),
            _ => {}
        }
    }
}

/// The fields in the warp the camera wears: the nearest the view's middle,
/// in screen terms, and the ripples on them.
pub fn show(time: Res<Time>, mut fields: ResMut<Fields>, camera: Single<(&Camera, &GlobalTransform, &mut super::warp::Warp), With<crate::camera::MainCamera>>) {
    let dt = time.delta_secs();
    for r in &mut fields.ripples {
        r.1 += dt;
    }
    fields.ripples.retain(|r| r.1 < RIPPLE_LIFE);
    let (cam, cam_tf, mut warp) = camera.into_inner();
    let warp = &mut warp.fields;
    if fields.fields.is_empty() {
        if warp.count != 0.0 {
            warp.count = 0.0;
        }
        return;
    }
    let size = cam.logical_viewport_size().unwrap_or(Vec2::ONE);
    let mid = cam_tf.translation().truncate();
    let uv = |p: Vec2| cam.world_to_viewport(cam_tf, p.extend(0.0)).ok().map(|v| v / size);
    // (Cells a screen's height: sizes in it, and patterns in cells.)
    let (Some(a), Some(b)) = (uv(mid), uv(mid + Vec2::Y)) else { return };
    let per_cell = (a.y - b.y).abs().max(1e-6);
    let mut shown: Vec<&Field> = fields.fields.iter().collect();
    shown.sort_by(|x, y| x.at.distance(mid).total_cmp(&y.at.distance(mid)));
    warp.shapes = [Vec4::ZERO; 2 * SHOWN];
    let mut n = 0;
    for f in shown.into_iter().take(SHOWN) {
        let (Some(at), Some(top)) = (uv(f.at), uv(Vec2::new(f.at.x, f.reach()))) else { continue };
        warp.shapes[2 * n] = Vec4::new(at.x, at.y, f.size() * per_cell, f.kind.code());
        warp.shapes[2 * n + 1] = Vec4::new(top.y, f.strength(), (f.at.x * 0.37 + f.at.y * 0.11).fract(), 0.0);
        n += 1;
    }
    warp.count = n as f32;
    warp.time = time.elapsed_secs();
    warp.aspect = size.x / size.y;
    warp.scale = 1.0 / per_cell;
    for (i, r) in warp.ripples.iter_mut().enumerate() {
        *r = fields.ripples.get(i).and_then(|&(at, age, hard)| uv(at).map(|p| Vec4::new(p.x, p.y, age, hard))).unwrap_or(Vec4::ZERO);
    }
}

/// A repulsor struck: blue-white sparks off the blow.
static RIPPLE: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 14.0,
    life: (0.15, 0.4),
    colors: vec![(240, 250, 255), (150, 205, 255), (90, 150, 255)],
    speed: 70.0,
    spread: 1.2,
    gravity: 0.0,
    drag: 4.0,
    size: 1.0,
    jitter: 0.0,
    glow: true,
});

/// A repulsor opening: glints.
static GLINT: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.2, 0.5),
    colors: vec![(220, 240, 255), (140, 195, 255)],
    speed: 60.0,
    spread: 3.14,
    gravity: 0.0,
    drag: 4.0,
    size: 1.0,
    jitter: 0.0,
    glow: true,
});

/// A slow field opening: violet motes, barely drifting.
static HAZE: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.8, 1.6),
    colors: vec![(200, 170, 255), (140, 100, 230)],
    speed: 30.0,
    spread: 3.14,
    gravity: 0.0,
    drag: 3.0,
    size: 1.0,
    jitter: 1.0,
    glow: true,
});

/// An updraft opening: pale motes streaming up.
static DRAFT: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.4, 0.9),
    colors: vec![(210, 255, 240), (120, 230, 210)],
    speed: 120.0,
    spread: 0.25,
    gravity: -40.0,
    drag: 0.5,
    size: 1.0,
    jitter: 0.0,
    glow: true,
});
