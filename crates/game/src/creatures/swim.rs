//! Swimmers (an eel, the leviathan, an angler): they swim as real fish do,
//! burst and coast. Their brain or their own code says where and how
//! keenly (`Controls`: the way to go, its length the hurry); this strokes
//! them there. Each stroke is a push along the way its head points, then
//! the water's drag slows it (a glide: `MovementStats::stroke_drag`); in a
//! hurry it strokes harder and oftener. Its head comes round toward where
//! it's going no faster than it can turn (more in a stroke: it bends into
//! it), and the way it's going comes round after its head (it carves arcs,
//! not drifting sideways), so it swings through loops and S-bends. A
//! strike (`Swimmer::strike`) is a hard kick and a burst of quick strokes.
//!
//! Its body (`BodyDef`): plates along the way its head went, a wave
//! running down them from head to tail, swelling with each stroke and
//! dying away in the glide (the dance of it); fins on them (along its
//! back and belly, at its sides, its tail's fan), each a fan of bony rays
//! with a dim web between and lit tips, swaying with it; spots and lines
//! of light along its flanks pulsing in waves down it; tendrils trailing
//! from its tail (`TendrilDef`), wafting on the water, crackling with
//! lightning if they're charged: near one, anything alive is shocked
//! (not itself). Its plates take blows (`Parts`). Drawn on two canvases:
//! its body lit as the world is, its glow over the light (as glowfish
//! are).

use std::collections::VecDeque;
use std::f32::consts::TAU;

use bevy::prelude::*;
use serde::Deserialize;

use super::body::animation::{Animator, Hug};
use super::body::legs::stroke;
use super::body::parts::{Part, PartKind, Parts};
use super::{Controls, Kinematics};
use crate::canvas::{Canvas, CanvasSprites};
use platypus_sim::CellPos;
use crate::world::{ChunkLoader, TickSet};

pub struct SwimPlugin;

impl Plugin for SwimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BodyCanvas>()
            .init_resource::<GlowCanvas>()
            .add_systems(FixedUpdate, (grow, turn_back, strokes, burrow).chain().after(super::custom::CustomSet::Think).in_set(TickSet::Intent))
            .add_systems(FixedUpdate, shock.in_set(TickSet::Bodies).after(super::move_creatures))
            .add_systems(Update, (follow.before(super::body::animation::animate), draw.after(follow)));
    }
}

/// How it swims, as a creature file writes it (`swim`).
#[derive(Clone, Debug, Deserialize)]
pub struct SwimDef {
    /// A stroke's push along its heading (cells/s), in a hurry (cruising,
    /// a third of it).
    pub stroke: f32,
    /// Seconds between strokes: in a hurry, and cruising.
    pub every: (f32, f32),
    /// The water's drag on it (a share of its speed a second): its glide.
    pub drag: f32,
    /// How fast its head turns (rad/s): gliding, and at a stroke's height.
    pub turn: (f32, f32),
    /// How fast the way it's going comes round to where its head points
    /// (rad/s).
    pub carve: f32,
    /// A burst (a strike): its strokes this many times as hard, this
    /// share as far apart as its hurried ones.
    #[serde(default = "burst")]
    pub burst: (f32, f32),
    /// Its head tilts with the way it swims up to this far (radians): a
    /// round fish's hardly, an eel's all the way.
    #[serde(default = "upright")]
    pub tilt: f32,
    /// It burrows (a leviathan): into rock it swims on through it as through
    /// water, its head tearing out a tunnel this many cells round (times
    /// its scale); 0: rock stops it.
    #[serde(default)]
    pub burrow: f32,
    #[serde(default)]
    pub body: Option<BodyDef>,
}

fn burst() -> (f32, f32) {
    (1.4, 0.6)
}

fn upright() -> f32 {
    std::f32::consts::FRAC_PI_2
}

/// Its body of plates behind its head.
#[derive(Clone, Debug, Deserialize)]
pub struct BodyDef {
    pub count: usize,
    /// Cells from one plate to the next; the biggest plate's length and
    /// height.
    pub spacing: f32,
    pub size: (f32, f32),
    /// Thickest this share of the way down it (0: at its head), the last
    /// plate this share of that.
    #[serde(default)]
    pub swell: f32,
    #[serde(default = "taper")]
    pub taper: f32,
    /// Its plates' colour, lit along its back, its belly, its outline.
    pub color: (u8, u8, u8),
    #[serde(default)]
    pub lit: Option<(u8, u8, u8)>,
    #[serde(default)]
    pub belly: Option<(u8, u8, u8)>,
    #[serde(default)]
    pub outline: Option<(u8, u8, u8)>,
    /// The wave down it: its length (plates), how far its tail sways (cells)
    /// at a stroke's height, its head this share of that, and between
    /// strokes this share.
    pub wave: (f32, f32, f32, f32),
    #[serde(default)]
    pub fins: Vec<FinDef>,
    #[serde(default)]
    pub glow: Option<GlowDef>,
    #[serde(default)]
    pub tendrils: Option<TendrilDef>,
}

/// Tendrils from its tail: so many, each so many links this far apart
/// (cells), this wide at the root (tapering), glowing this colour; charged,
/// they crackle and shock what touches them (`zap`: damage, stun).
#[derive(Clone, Debug, Deserialize)]
pub struct TendrilDef {
    pub count: usize,
    pub links: usize,
    pub gap: f32,
    #[serde(default = "tendril_width")]
    pub width: f32,
    pub color: (u8, u8, u8),
    #[serde(default)]
    pub zap: Option<(f32, f32)>,
}

fn tendril_width() -> f32 {
    2.0
}

fn taper() -> f32 {
    0.3
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum FinKind {
    /// Along its back (its upper side), or its belly.
    Back,
    Belly,
    /// At its side, sweeping back and down, flapping with its strokes.
    Side,
    /// Its tail's fan, off its last plate.
    Tail,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FinDef {
    pub kind: FinKind,
    /// Where along it (a share of the way, head to tail); how long (cells,
    /// along it) and tall.
    pub at: f32,
    pub size: (f32, f32),
    pub color: (u8, u8, u8),
    /// Its edge glows this colour.
    #[serde(default)]
    pub glow: Option<(u8, u8, u8)>,
}

/// Light along its flanks: a spot on every so many plates (and a line
/// down its side between them, `line`), pulsing in waves running down it
/// (so many a second; 0: steady); and so many lights spread down it,
/// lighting the water round it (each this bright).
#[derive(Clone, Debug, Deserialize)]
pub struct GlowDef {
    pub color: (u8, u8, u8),
    pub every: usize,
    #[serde(default)]
    pub pulse: f32,
    #[serde(default)]
    pub line: bool,
    #[serde(default)]
    pub lights: usize,
    #[serde(default = "light_strength")]
    pub strength: f32,
}

fn light_strength() -> f32 {
    0.2
}

/// A swimmer's strokes and its body as it lies.
#[derive(Component, Debug)]
pub struct Swimmer {
    /// Where its head points (radians).
    pub heading: f32,
    /// Seconds to its next stroke.
    stroke_left: f32,
    /// The last stroke's swell (1 as it strokes, dying away in the glide).
    pub pulse: f32,
    /// Seconds of a burst left (a strike).
    pub burst: f32,
    /// The wave's phase down its body; its glow's.
    phase: f32,
    glow: f32,
    /// The way its head went (newest first), as long as its body.
    way: VecDeque<Vec2>,
    /// Its plates: where, and which way they point (toward its head).
    pub plates: Vec<(Vec2, Vec2)>,
    /// A flare of its glow (its own code's tells): how bright (0..1), and
    /// where down it (a share, head to tail; below 0: all of it).
    pub flare: (f32, f32),
    /// A ball of its glow gathering in its maw (its own code's: 0 none, 1
    /// full), lighting the water round it.
    pub charge: f32,
    charge_light: Option<Entity>,
    /// Its lights down its body, and at its eye.
    lights: Vec<Entity>,
    eye_light: Option<Entity>,
    /// Its tendrils: each link where it is and was (Verlet).
    pub tendrils: Vec<(Vec<Vec2>, Vec<Vec2>)>,
    /// What its tendrils have shocked lately (each a wait).
    shocked: Vec<(Entity, f32)>,
    /// Its stroke count, for jitter.
    strokes: u32,
}

impl Swimmer {
    fn new(heading: f32) -> Swimmer {
        Swimmer { heading, stroke_left: 0.0, pulse: 0.0, burst: 0.0, phase: 0.0, glow: 0.0, way: VecDeque::new(), plates: Vec::new(), flare: (0.0, 0.0), charge: 0.0, charge_light: None, lights: Vec::new(), eye_light: None, tendrils: Vec::new(), shocked: Vec::new(), strokes: 0 }
    }

    /// A strike: a hard kick along `dir` (to `speed`), its head turned
    /// there, and a burst of quick strokes after for `secs`.
    pub fn strike(&mut self, k: &mut Kinematics, dir: Vec2, speed: f32, secs: f32) {
        let dir = dir.normalize_or(Vec2::from_angle(self.heading));
        k.body.vel = k.body.vel * 0.3 + dir * speed;
        self.heading = dir.to_angle();
        self.pulse = 1.0;
        self.burst = secs;
        self.stroke_left = 0.12;
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
/// Its swell from a stroke dies away this fast (seconds).
const PULSE_FADES: f32 = 0.35;
/// Charged tendrils' lightning leaps this far (cells) to what's alive near
/// them (touching or not), at most once this often a body.
const LEAP: f32 = 46.0;
const SHOCK_EVERY: f32 = 0.9;
/// Lightning on charged tendrils: new arcs this many times a second, this
/// many crawling along each tendril at once.
const ARC_HZ: f32 = 18.0;
const ARCS: u64 = 2;
/// Its eye's light (with a body's glow and an `eye` in its art), and the
/// halo round it (cells across, how bright at its heart).
const EYE_LIGHT: f32 = 0.35;
const EYE_HALO: (f32, f32) = (9.0, 170.0);
/// The ball of light in its maw: this big (cells, empty to full: times its
/// scale), its light this bright full.
const CHARGE: (f32, f32, f32) = (2.0, 9.0, 0.9);
/// The wave runs down it this many times a second gliding, and this many
/// more at a stroke's height.
const WAVE_HZ: (f32, f32) = (0.6, 2.4);

/// The way from `b` round to `a` (radians, the short way).
fn round(a: f32, b: f32) -> f32 {
    (a - b + TAU * 1.5).rem_euclid(TAU) - TAU * 0.5
}

/// A swimmer's strokes (its glide's its movement's: `def.rs` sets
/// `stroke_drag` from `drag`).
fn grow(mut commands: Commands, q: Query<(Entity, &Animator, &Kinematics), Without<Swimmer>>) {
    for (e, anim, k) in &q {
        if anim.def.swim.is_none() {
            continue;
        }
        commands.entity(e).insert((Swimmer::new(if k.loco.facing < 0.0 { std::f32::consts::PI } else { 0.0 }), Hug::default()));
    }
}

/// Stroke by stroke where its controls say, as keenly as they say; its
/// head the way it swims (still, turned to what it aims at).
fn strokes(mut q: Query<(&mut Swimmer, &mut Controls, &mut Kinematics, &Animator, Has<super::Phasing>)>) {
    for (mut s, mut c, mut k, anim, phasing) in &mut q {
        let Some(sd) = anim.def.swim.as_ref() else { continue };
        s.pulse *= (-DT / PULSE_FADES).exp();
        s.burst -= DT;
        s.stroke_left -= DT;
        // (Burrowing through rock it swims as in water: the rock's drag its
        // own, its movement left be: `burrow`.)
        if phasing {
            k.body.vel *= (-sd.drag * DT).exp();
        } else if k.loco.contacts.submerged < 0.5 {
            continue;
        }
        let want = Vec2::new(c.0.move_x, c.0.move_y);
        let hurry = want.length().min(1.0);
        let bursting = s.burst > 0.0;
        // (Still, it turns to what it's aiming at; it faces the way its
        // head points.)
        if hurry <= 0.05 && c.0.aim != Vec2::ZERO {
            let most = sd.turn.0 * DT;
            let heading = s.heading;
            s.heading += round((c.0.aim - k.body.pos).to_angle(), heading).clamp(-most, most);
        }
        c.0.aim = k.body.pos + Vec2::from_angle(s.heading) * 60.0;
        if hurry > 0.05 {
            let most = (sd.turn.0 + (sd.turn.1 - sd.turn.0) * s.pulse) * DT * if bursting { 1.5 } else { 1.0 };
            let heading = s.heading;
            s.heading += round(want.to_angle(), heading).clamp(-most, most);
            if s.stroke_left <= 0.0 {
                let push = sd.stroke * (0.35 + 0.65 * hurry) * if bursting { sd.burst.0 } else { 1.0 };
                k.body.vel += Vec2::from_angle(s.heading) * push;
                s.pulse = 1.0;
                s.strokes += 1;
                // (A little uneven, as a living thing's strokes are.)
                let jitter = 0.85 + 0.3 * (platypus_sim::rng::hash(&[s.strokes as u64, 0x5717]) % 100) as f32 / 100.0;
                let every = sd.every.1 + (sd.every.0 - sd.every.1) * hurry;
                s.stroke_left = every * jitter * if bursting { sd.burst.1 } else { 1.0 };
            }
        }
        // (The way it's going comes round after its head: it carves.)
        let speed = k.body.vel.length();
        if speed > 1.0 {
            let going = k.body.vel.to_angle();
            let most = sd.carve * DT;
            let turned = going + round(s.heading, going).clamp(-most, most);
            k.body.vel = Vec2::from_angle(turned) * speed;
        }
    }
}

/// Its head's tilt (as `animate` turns its sprite): the way its head
/// points, up or down, from level, the way it faces.
fn tilt(s: &Swimmer, k: &Kinematics) -> f32 {
    let (sin, cos) = s.heading.sin_cos();
    sin.atan2(cos.abs()) * k.loco.facing
}

/// An anchor of its head's art, turned with its head (`Hug`): its maw,
/// where its body joins.
pub fn anchor(anim: &Animator, k: &Kinematics, turn: f32, name: &str) -> Option<Vec2> {
    super::custom::frame_anchor(anim, k, name, "").map(|g| k.body.pos + Vec2::from_angle(turn).rotate(g - k.body.pos))
}

/// A burrower at the edge of the loaded ground: turned back this tick.
#[derive(Component)]
struct TurnBack;

/// A burrower turned back from unloaded ground: its way reversed, its head
/// round.
fn turn_back(mut commands: Commands, mut q: Query<(Entity, &mut Swimmer, &mut Kinematics), With<TurnBack>>) {
    for (e, mut s, mut k) in &mut q {
        k.body.vel = -k.body.vel * 0.6;
        s.heading += std::f32::consts::PI;
        commands.entity(e).remove::<TurnBack>();
    }
}

/// A burrower in rock swims through it (`Phasing`: its movement left be),
/// its head tearing out a tunnel (not the world's floor: bedrock), rubble
/// flying; out in the open again, it moves as ever.
fn burrow(
    mut commands: Commands,
    mut sim: ResMut<crate::world::SimWorld>,
    q: Query<(Entity, &Swimmer, &Kinematics, &Animator, &Hug, Has<super::Phasing>)>,
    mut sparks: ResMut<crate::vfx::Sparks>,
) {
    let bedrock = sim.materials().id("bedrock");
    for (e, s, k, anim, hug, phasing) in &q {
        let Some(sd) = anim.def.swim.as_ref().filter(|sd| sd.burrow > 0.0) else { continue };
        let r = sd.burrow * anim.def.scale;
        let ahead = Vec2::from_angle(s.heading);
        let head = anchor(anim, k, hug.0, "maw").unwrap_or(k.body.pos + ahead * k.body.half.x);
        let solid = |p: Vec2| sim.world.is_solid(CellPos::from_world(p.x, p.y));
        // (Ground not loaded ahead, it turns back: there it'd be held still.)
        let far = head + ahead * (r + 40.0);
        if !sim.world.is_loaded(CellPos::from_world(far.x, far.y).chunk()) {
            commands.entity(e).insert(TurnBack);
        }
        // In rock (its box, or just ahead of its head): through it.
        let (pos, half) = (k.body.pos, k.body.half);
        let boxed = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0), (0.0, 0.0), (1.0, 0.0), (-1.0, 0.0)].iter().any(|&(x, y)| solid(pos + half * Vec2::new(x, y)));
        // (Rock close round it, as down a tunnel it's torn: it swims on
        // down it, not sinking to its floor.)
        let round = half.length() + r;
        let walled = (0..8).filter(|i| solid(pos + Vec2::from_angle(*i as f32 * std::f32::consts::FRAC_PI_4) * round)).count() >= 3;
        let in_rock = boxed || walled || solid(head + ahead * r);
        if in_rock && !phasing {
            commands.entity(e).insert(super::Phasing);
        } else if !in_rock && phasing {
            commands.entity(e).remove::<super::Phasing>();
        }
        if !in_rock {
            continue;
        }
        // Its head tears out the rock round it.
        let c = head + ahead * r * 0.5;
        let lo = (c - Vec2::splat(r)).floor().as_ivec2();
        let mut torn = 0;
        for y in lo.y..=lo.y + (r * 2.0) as i32 + 1 {
            for x in lo.x..=lo.x + (r * 2.0) as i32 + 1 {
                let p = CellPos::new(x, y);
                if Vec2::new(x as f32 + 0.5, y as f32 + 0.5).distance(c) > r || !sim.world.is_solid(p) || sim.world.get(p).is_some_and(|cell| Some(cell.material) == bedrock) {
                    continue;
                }
                sim.world.set(p, platypus_sim::Cell::AIR);
                torn += 1;
            }
        }
        if torn > 0 && (s.glow * 20.0) as u32 % 3 == 0 {
            sparks.emit(&RUBBLE, 4, c, ahead, k.body.vel * 0.3);
        }
    }
}

/// Rock torn out by a burrower's head: grey grit flying.
static RUBBLE: std::sync::LazyLock<crate::magic::runes::Emitter> = std::sync::LazyLock::new(|| crate::magic::runes::Emitter { count: 1.0, life: (0.3, 0.8), colors: vec![(150, 146, 140), (100, 98, 96), (60, 58, 58)], speed: 70.0, spread: 1.4, gravity: 60.0, drag: 3.0, size: 1.2, jitter: 0.0, glow: false });

/// Where its body joins its head: its art's `grip`, else its middle.
fn neck(anim: &Animator, k: &Kinematics, turn: f32) -> Vec2 {
    anchor(anim, k, turn, "grip").unwrap_or(k.body.pos)
}

/// A plate's size against the biggest (`swell`, `taper`).
fn scale(b: &BodyDef, t: f32) -> f32 {
    if t < b.swell {
        0.8 + 0.2 * t / b.swell.max(1e-3)
    } else {
        1.0 - (1.0 - b.taper) * ((t - b.swell) / (1.0 - b.swell).max(1e-3))
    }
}

/// Its plates along the way its head went, the wave on them; its plates
/// to take blows.
#[allow(clippy::type_complexity)]
fn follow(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Swimmer, &Kinematics, &Animator, Option<&mut Parts>, &mut Hug)>, mut lamps: Query<(&mut Transform, &mut crate::light::LightSource), Without<Swimmer>>) {
    let dt = time.delta_secs().min(0.05);
    for (e, mut s, k, anim, parts, mut hug) in &mut q {
        // (Its head turned the way it swims, eased.)
        let most = anim.def.swim.as_ref().map_or(0.0, |sd| sd.tilt);
        let want = tilt(&s, k).clamp(-most, most);
        hug.0 += round(want, hug.0) * (12.0 * dt).min(1.0);
        let Some(b) = anim.def.swim.as_ref().and_then(|sd| sd.body.as_ref()) else { continue };
        let n = b.count;
        if n == 0 {
            continue;
        }
        let head = neck(anim, k, hug.0);
        let pulse = s.pulse;
        s.phase += dt * TAU * (WAVE_HZ.0 + WAVE_HZ.1 * pulse);
        s.glow += dt;
        // (Each plate after the last its spacing on, as big as it is: the
        // small ones toward its tail close together, no gaps.)
        let gaps: Vec<f32> = (0..n).map(|j| b.spacing * scale(b, j as f32 / (n - 1).max(1) as f32).max(0.4)).collect();
        let total: f32 = gaps.iter().sum::<f32>() + b.spacing;
        if s.way.is_empty() {
            // (Laid out straight behind its head.)
            let back = -Vec2::from_angle(s.heading);
            let mut d = 0.0;
            while d <= total {
                s.way.push_back(head + back * d);
                d += 2.0;
            }
        }
        if s.way.front().is_none_or(|p| p.distance(head) >= 1.0) {
            s.way.push_front(head);
        }
        // (No more of the way than its body needs.)
        let mut gone = 0.0;
        let mut keep = s.way.len();
        for i in 1..s.way.len() {
            gone += s.way[i - 1].distance(s.way[i]);
            if gone > total + 4.0 {
                keep = i + 1;
                break;
            }
        }
        s.way.truncate(keep);
        // Each plate along the way, and swayed across it by the wave.
        let (length, sway, at_head, rest) = b.wave;
        let swell = rest + (1.0 - rest) * pulse;
        let mut plates = Vec::with_capacity(n);
        let (mut i, mut walked, mut want) = (0usize, 0.0, 0.0);
        for j in 0..n {
            want += gaps[j];
            while i + 1 < s.way.len() && walked + s.way[i].distance(s.way[i + 1]) < want {
                walked += s.way[i].distance(s.way[i + 1]);
                i += 1;
            }
            let (p, q) = (s.way[i], s.way[(i + 1).min(s.way.len() - 1)]);
            let seg = q.distance(p).max(1e-3);
            let on = p + (q - p) * ((want - walked) / seg).clamp(0.0, 1.0);
            let dir = (p - q).normalize_or(Vec2::from_angle(s.heading));
            let t = j as f32 / (n - 1).max(1) as f32;
            let amp = sway * (at_head + (1.0 - at_head) * t.powf(1.4)) * swell;
            let off = dir.perp() * amp * (s.phase - TAU * j as f32 / length.max(1.0)).sin();
            plates.push((on + off, dir));
        }
        // (Each plate pointing at the one ahead, as the wave bends it.)
        for j in 0..n {
            let ahead = if j == 0 { head } else { plates[j - 1].0 };
            plates[j].1 = (ahead - plates[j].0).normalize_or(plates[j].1);
        }
        let list: Vec<Part> = plates
            .iter()
            .enumerate()
            .map(|(j, &(at, dir))| {
                let sc = scale(b, j as f32 / (n - 1).max(1) as f32);
                let half = dir * b.size.0 * sc * 0.5;
                Part { a: at - half, b: at + half, r: b.size.1 * sc * 0.5 + 0.5, mult: 1.0, kind: PartKind::Segment, limb: None }
            })
            .collect();
        match parts {
            Some(mut p) => p.set(list),
            None => {
                let mut p = Parts::default();
                p.set(list);
                commands.entity(e).insert(p);
            }
        }
        s.plates = plates;
        // Its tendrils, trailing from its tail.
        if let Some(td) = &b.tendrils {
            let (end, dir) = s.plates[n - 1];
            let root = end - dir * b.size.0 * scale(b, 1.0) * 0.5;
            waft(&mut s, td, root, dir, dt);
        }
        // Its eye (its art's `eye`), lit: a light there, its glow's colour.
        if let (Some(g), Some(eye)) = (&b.glow, anchor(anim, k, hug.0, "eye")) {
            let l = *s.eye_light.get_or_insert_with(|| commands.spawn((Name::new("Swimmer's eye"), crate::light::LightSource { color: [0.0; 3], flicker: 0.0 }, Transform::default(), ChildOf(e))).id());
            if let Ok((mut tf, mut light)) = lamps.get_mut(l) {
                tf.translation = (eye - k.body.pos).extend(0.0);
                let bright = EYE_LIGHT * (1.0 + flare_at(s.flare, 0.0));
                light.color = [g.color.0 as f32 / 255.0 * bright, g.color.1 as f32 / 255.0 * bright, g.color.2 as f32 / 255.0 * bright];
            }
        }
        // The ball of light gathering in its maw: a light there as it grows.
        if let (Some(g), Some(maw)) = (&b.glow, anchor(anim, k, hug.0, "maw")) {
            let l = *s.charge_light.get_or_insert_with(|| commands.spawn((Name::new("Swimmer's charge"), crate::light::LightSource { color: [0.0; 3], flicker: 0.0 }, Transform::default(), ChildOf(e))).id());
            if let Ok((mut tf, mut light)) = lamps.get_mut(l) {
                tf.translation = (maw - k.body.pos).extend(0.0);
                let bright = CHARGE.2 * s.charge;
                light.color = [g.color.0 as f32 / 255.0 * bright, g.color.1 as f32 / 255.0 * bright, g.color.2 as f32 / 255.0 * bright];
            }
        }
        // Its lights down it (its glow's colour, flaring with it).
        if let Some(g) = &b.glow {
            if s.lights.len() != g.lights {
                for l in s.lights.drain(..) {
                    commands.entity(l).despawn();
                }
                s.lights = (0..g.lights).map(|_| commands.spawn((Name::new("Swimmer's light"), crate::light::LightSource { color: [0.0; 3], flicker: 0.0 }, Transform::default(), ChildOf(e))).id()).collect();
            }
            for (i, &l) in s.lights.iter().enumerate() {
                let t = (i as f32 + 0.5) / g.lights as f32;
                let j = ((t * (n - 1) as f32).round() as usize).min(n - 1);
                let Ok((mut tf, mut light)) = lamps.get_mut(l) else { continue };
                // (A child of it: where its plate is, from it.)
                let at = s.plates[j].0 - k.body.pos;
                tf.translation = at.extend(0.0);
                let bright = g.strength * (1.0 + 2.0 * flare_at(s.flare, t));
                light.color = [g.color.0 as f32 / 255.0 * bright, g.color.1 as f32 / 255.0 * bright, g.color.2 as f32 / 255.0 * bright];
            }
        }
    }
}

/// Its tendrils' links, from its tail's end: trailing on the water (a
/// Verlet chain, damped), wafting across their way (each its own beat).
fn waft(s: &mut Swimmer, td: &TendrilDef, root: Vec2, dir: Vec2, dt: f32) {
    // (Each its own: a little longer or shorter than the rest.)
    let own = |k: usize| (platypus_sim::rng::hash(&[k as u64, 0x7E4D]) % 1000) as f32 / 1000.0;
    let gap = |k: usize| td.gap * (0.7 + 0.6 * own(k));
    if s.tendrils.len() != td.count {
        s.tendrils = (0..td.count).map(|k| {
            let pts: Vec<Vec2> = (0..td.links).map(|i| root - dir * gap(k) * i as f32).collect();
            (pts.clone(), pts)
        }).collect();
    }
    let keep = (-dt * 2.5).exp();
    let across = dir.perp();
    let t = s.glow;
    for (k, (pts, prev)) in s.tendrils.iter_mut().enumerate() {
        // (All from the one point at its tail's tip, a tassel, each off
        // at its own angle.)
        let at = root;
        if pts[0].distance(at) > 400.0 {
            let way = Vec2::from_angle((k as f32 - (td.count as f32 - 1.0) * 0.5) * 0.25).rotate(-dir);
            *pts = (0..td.links).map(|i| at + way * td.gap * i as f32).collect();
            *prev = pts.clone();
        }
        // (Each its own beat; they fan apart toward their tips.)
        let (beat, hz) = (own(k) * TAU, 1.4 + 1.6 * own(k + 17));
        let fan = (k as f32 - (td.count as f32 - 1.0) * 0.5) / td.count.max(1) as f32;
        let gap = gap(k);
        for i in 1..pts.len() {
            let v = (pts[i] - prev[i]) * keep;
            prev[i] = pts[i];
            let far = i as f32 / pts.len() as f32;
            let sway = across * ((t * hz + beat + i as f32 * 0.3).sin() * 90.0 + fan * 140.0) * far;
            // (Drawn out behind its tail, as by a current: at rest they
            // hang back, not curling up in knots.)
            let trail = -dir * 160.0;
            pts[i] += v + (sway + trail + Vec2::NEG_Y * 8.0) * dt * dt;
        }
        for _ in 0..4 {
            pts[0] = at;
            for i in 1..pts.len() {
                let d = pts[i] - pts[i - 1];
                let len = d.length().max(1e-4);
                let fix = d * (1.0 - gap / len);
                if i == 1 {
                    pts[i] -= fix;
                } else {
                    pts[i - 1] += fix * 0.5;
                    pts[i] -= fix * 0.5;
                }
            }
        }
        pts[0] = at;
    }
}

/// Charged tendrils shock what's alive and touching them or near them (the
/// lightning leaps the gap: `LEAP`), never their own: a storm hit, a jolt,
/// a bolt from the tendril into it.
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn shock(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut swimmers: Query<(Entity, &mut Swimmer, &Animator)>,
    bodies: Query<(Entity, &Kinematics, &super::Health)>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut sparks: ResMut<crate::vfx::Sparks>,
) {
    for (e, mut s, anim) in &mut swimmers {
        let Some(td) = anim.def.swim.as_ref().and_then(|sd| sd.body.as_ref()).and_then(|b| b.tendrils.as_ref()) else { continue };
        let Some((damage, stun)) = td.zap else { continue };
        for w in s.shocked.iter_mut() {
            w.1 -= DT;
        }
        s.shocked.retain(|w| w.1 > 0.0);
        // (Crackling now and then along them.)
        let n = s.tendrils.len();
        if n > 0 && platypus_sim::rng::hash(&[e.to_bits(), (s.glow * 60.0) as u64]) % 4 == 0 {
            let (pts, _) = &s.tendrils[(s.glow * 37.0) as usize % n];
            if let Some(&at) = pts.get((s.glow * 91.0) as usize % pts.len().max(1)) {
                sparks.emit(&crackle(td.color), 2, at, Vec2::Y, Vec2::ZERO);
            }
        }
        for (who, k, h) in &bodies {
            if who == e || h.hp <= 0.0 || s.shocked.iter().any(|w| w.0 == who) {
                continue;
            }
            // (Touching them, or near enough that the lightning leaps the
            // gap to it.)
            let reach = k.body.half.length() + td.width + LEAP;
            let touch = s.tendrils.iter().flat_map(|(pts, _)| pts.iter().skip(1)).min_by(|a, b| a.distance(k.body.pos).total_cmp(&b.distance(k.body.pos))).filter(|p| p.distance(k.body.pos) < reach).copied();
            if let Some(at) = touch {
                let away = (k.body.pos - at).normalize_or(Vec2::Y);
                hits.write(crate::combat::Hit { by: Some(e), target: who, damage, harm: super::Harm::Storm, knock: away * 220.0, stun, at, dir: away, weight: 0.6, crit: false });
                sounds.write(crate::sound::PlaySound::at("zap", at).pitch(1.3).volume(0.9));
                sparks.emit(&crackle(td.color), 14, at, away, Vec2::ZERO);
                // (A bolt of it, from the tendril into them: as a wand's.)
                let path: Vec<CellPos> = jag(at, k.body.pos, who.to_bits() ^ (s.glow * 100.0) as u64, 5.0, 3)
                    .windows(2)
                    .flat_map(|w| {
                        let n = w[0].distance(w[1]).ceil().max(1.0) as i32;
                        (0..=n).map(move |i| { let p = w[0].lerp(w[1], i as f32 / n as f32); CellPos::from_world(p.x, p.y) })
                    })
                    .collect();
                let zap = platypus_sim::Zap { from: CellPos::from_world(at.x, at.y), to: CellPos::from_world(k.body.pos.x, k.body.pos.y), path, charged: Vec::new() };
                crate::fx::spawn_bolt(&mut commands, &mut images, &zap);
                s.shocked.push((who, SHOCK_EVERY));
            }
        }
    }
}

/// A jagged run of lightning from `a` to `b` (its middle knocked aside,
/// then each half's, and so on: `depth` times), `amp` cells at most.
fn jag(a: Vec2, b: Vec2, seed: u64, amp: f32, depth: u32) -> Vec<Vec2> {
    let mut pts = vec![a, b];
    let mut amp = amp;
    for d in 0..depth {
        let mut next = Vec::with_capacity(pts.len() * 2);
        for (i, w) in pts.windows(2).enumerate() {
            let (p, q) = (w[0], w[1]);
            let r = (platypus_sim::rng::hash(&[seed, d as u64, i as u64]) % 2001) as f32 / 1000.0 - 1.0;
            let mid = (p + q) * 0.5 + (q - p).perp().normalize_or_zero() * r * amp;
            next.push(p);
            next.push(mid);
        }
        next.push(*pts.last().expect("two at least"));
        pts = next;
        amp *= 0.55;
    }
    pts
}

/// Lightning drawn as a wand's is (`fx::zap_image`): a white-hot line a
/// cell wide, a pale blue glow either side.
fn arc(pts: &[Vec2], lights: &mut Vec<(IVec2, [u8; 4])>) {
    let mut cells = Vec::new();
    for w in pts.windows(2) {
        let n = w[0].distance(w[1]).ceil().max(1.0) as i32;
        for i in 0..=n {
            cells.push(w[0].lerp(w[1], i as f32 / n as f32).floor().as_ivec2());
        }
    }
    for &c in &cells {
        for d in [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y] {
            lights.push((c + d, [150, 170, 255, 110]));
        }
    }
    for &c in &cells {
        lights.push((c, [250, 250, 255, 255]));
    }
}

/// Lightning's crackle off a charged tendril: white-hot, jumping about.
fn crackle((r, g, b): (u8, u8, u8)) -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter { count: 1.0, life: (0.05, 0.18), colors: vec![(255, 255, 255), (r, g, b), (r / 3, g / 3, b / 2)], speed: 60.0, spread: 3.14, gravity: 0.0, drag: 6.0, size: 1.0, jitter: 90.0, glow: true }
}

/// How much of a flare is at `t` down its body (0..1).
fn flare_at((bright, at): (f32, f32), t: f32) -> f32 {
    if at < 0.0 { bright } else { bright * (-((t - at) / 0.12).powi(2)).exp() }
}

/// Swimmers' bodies, lit as the world is (behind their heads).
#[derive(Resource)]
struct BodyCanvas(Canvas);

impl Default for BodyCanvas {
    fn default() -> Self {
        BodyCanvas(Canvas::new("Swimmers", 9.4))
    }
}

/// Their glow, over the light (as glowfish's).
#[derive(Resource)]
struct GlowCanvas(Canvas);

impl Default for GlowCanvas {
    fn default() -> Self {
        GlowCanvas(Canvas::new("Swimmers' glow", 15.55))
    }
}

fn rgb((r, g, b): (u8, u8, u8)) -> [u8; 4] {
    [r, g, b, 255]
}

/// A glowing fin's membrane: its glow, faint.
const MEMBRANE: u8 = 110;

fn faint((r, g, b): (u8, u8, u8)) -> [u8; 4] {
    [r, g, b, MEMBRANE]
}

/// A fin's web between its rays: its colour, see-through; its rays paler.
fn web((r, g, b): (u8, u8, u8)) -> [u8; 4] {
    [r, g, b, 170]
}

fn ray((r, g, b): (u8, u8, u8)) -> [u8; 4] {
    [r.saturating_add(50), g.saturating_add(60), b.saturating_add(60), 255]
}

fn dark((r, g, b): (u8, u8, u8)) -> (u8, u8, u8) {
    ((r as f32 * 0.6) as u8, (g as f32 * 0.6) as u8, (b as f32 * 0.6) as u8)
}

/// A plate: a capsule from `a` to `b`, `r` round, each cell once: its
/// outline (a cell round it: `outline`), or lit along its upper side
/// (`up`), its belly under, its fill between. (Every plate's outline
/// first, then their fills: one body, not a string of beads.)
#[allow(clippy::too_many_arguments)]
fn capsule(a: Vec2, b: Vec2, r: f32, outline: bool, line: [u8; 4], fill: [u8; 4], lit: [u8; 4], belly: [u8; 4], up: Vec2, out: &mut Vec<(IVec2, [u8; 4])>) {
    let d = b - a;
    let len2 = d.length_squared().max(1e-4);
    let lo = (a.min(b) - Vec2::splat(r + 1.5)).floor().as_ivec2();
    let hi = (a.max(b) + Vec2::splat(r + 1.5)).ceil().as_ivec2();
    let across = d.perp().normalize_or(Vec2::Y);
    let across = if across.dot(up) < 0.0 { -across } else { across };
    for y in lo.y..=hi.y {
        for x in lo.x..=hi.x {
            let q = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let t = ((q - a).dot(d) / len2).clamp(0.0, 1.0);
            let off = q - (a + d * t);
            let dist = off.length();
            if dist > r + 1.0 || (outline && dist <= r) || (!outline && dist > r) {
                continue;
            }
            let side = off.dot(across);
            let c = if outline {
                line
            } else if side > r - 1.2 {
                lit
            } else if side < -(r - 1.2) {
                belly
            } else {
                fill
            };
            out.push((IVec2::new(x, y), c));
        }
    }
}

/// A fin of rays: bony spines from its base (`a` to `b`) out along `out`
/// (leaning back along `back`), tallest in the middle, a web between them
/// (`web`, see-through), each tip lit (`tip`, over the light).
#[allow(clippy::too_many_arguments)]
fn rays(a: Vec2, b: Vec2, out: Vec2, back: Vec2, tall: f32, lean: f32, ray: [u8; 4], web: [u8; 4], tip: Option<[u8; 4]>, cells: &mut Vec<(IVec2, [u8; 4])>, lights: &mut Vec<(IVec2, [u8; 4])>) {
    let k = ((a.distance(b) / 4.0).round() as usize).clamp(3, 9);
    let spine = |i: usize| {
        let t = (i as f32 + 0.5) / k as f32;
        let base = a + (b - a) * t;
        let h = tall * (std::f32::consts::PI * t).sin().powf(0.6) * (0.8 + 0.2 * ((i * 7) % 3) as f32 / 2.0);
        (base, base + out * h + back * h * lean)
    };
    for i in 0..k - 1 {
        let ((b0, t0), (b1, t1)) = (spine(i), spine(i + 1));
        fin(t0, t1, b0, web, cells);
        fin(b0, b1, t1, web, cells);
    }
    for i in 0..k {
        let (base, end) = spine(i);
        stroke(base, end, 0.8, 0.4, ray, cells);
        if let Some(g) = tip {
            stroke(end, end, 0.9, 0.9, g, lights);
            stroke(end - (end - base) * 0.25, end, 0.4, 0.4, [g[0], g[1], g[2], 150], lights);
        }
    }
}

/// A fin: a fan of strokes from its tip to each point along its base.
fn fin(a: Vec2, b: Vec2, tip: Vec2, c: [u8; 4], out: &mut Vec<(IVec2, [u8; 4])>) {
    let n = (a.distance(b) * 1.5).ceil().max(2.0) as usize;
    for i in 0..=n {
        stroke(a + (b - a) * (i as f32 / n as f32), tip, 0.6, 0.4, c, out);
    }
}

/// Its fins and plates (tail first, so the nearer its head the more on
/// top), its glow over them.
fn draw(
    mut commands: Commands,
    mut body: ResMut<BodyCanvas>,
    mut glow: ResMut<GlowCanvas>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<(&GlobalTransform, &ChunkLoader), With<crate::camera::MainCamera>>,
    q: Query<(&Swimmer, &Animator, &Kinematics, &Hug)>,
    mut sprites: CanvasSprites,
) {
    let Ok((cam, loader)) = camera.single() else { return };
    let (eye, half) = (cam.translation().truncate(), loader.half_extent);
    let mut cells: Vec<(IVec2, [u8; 4])> = Vec::new();
    let mut lights: Vec<(IVec2, [u8; 4])> = Vec::new();
    for (s, anim, k, hug) in &q {
        let Some(b) = anim.def.swim.as_ref().and_then(|sd| sd.body.as_ref()) else { continue };
        // The ball of light gathering in its maw: a white-hot heart, its
        // glow round it, flickering.
        if let (Some(g), Some(maw)) = (&b.glow, anchor(anim, k, hug.0, "maw"))
            && s.charge > 0.01
        {
            let (r, gg, bl) = g.color;
            let flick = 1.0 + 0.12 * (s.glow * 23.0).sin();
            let reach = (CHARGE.0 + (CHARGE.1 - CHARGE.0) * s.charge) * anim.def.scale * flick;
            let lo = (maw - Vec2::splat(reach * 1.8)).floor().as_ivec2();
            let span = (reach * 3.6) as i32 + 2;
            for y in lo.y..=lo.y + span {
                for x in lo.x..=lo.x + span {
                    let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5).distance(maw) / reach;
                    let c = if d < 0.45 {
                        [255, 255, 255, 255]
                    } else if d < 1.0 {
                        let t = (d - 0.45) / 0.55;
                        let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
                        [mix(255, r), mix(255, gg), mix(255, bl), 255]
                    } else if d < 1.8 {
                        [r, gg, bl, (150.0 * (1.0 - (d - 1.0) / 0.8).powi(2)) as u8]
                    } else {
                        continue;
                    };
                    lights.push((IVec2::new(x, y), c));
                }
            }
        }
        // A halo round its eye, its glow's colour, fading out from it.
        if let (Some(g), Some(eye)) = (&b.glow, anchor(anim, k, hug.0, "eye")) {
            let (r, gg, bl) = g.color;
            let reach = EYE_HALO.0 * anim.def.scale;
            let flare = 1.0 + flare_at(s.flare, 0.0);
            let lo = (eye - Vec2::splat(reach)).floor().as_ivec2();
            for y in lo.y..=lo.y + (reach * 2.0) as i32 + 1 {
                for x in lo.x..=lo.x + (reach * 2.0) as i32 + 1 {
                    let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5).distance(eye) / reach;
                    if d < 1.0 {
                        let a = (EYE_HALO.1 * (1.0 - d).powi(2) * flare).min(255.0) as u8;
                        if a > 8 {
                            lights.push((IVec2::new(x, y), [r, gg, bl, a]));
                        }
                    }
                }
            }
        }
        let n = s.plates.len();
        if n == 0 || !s.plates.iter().any(|(p, _)| (*p - eye).abs().cmple(half + 60.0).all()) {
            continue;
        }
        let line = rgb(b.outline.unwrap_or_else(|| dark(dark(b.color))));
        let (fill, lit, belly) = (rgb(b.color), rgb(b.lit.unwrap_or(b.color)), rgb(b.belly.unwrap_or_else(|| dark(b.color))));
        // (Its upper side: across its way, whichever way's up.)
        let up = |dir: Vec2| {
            let p = dir.perp();
            if p.y < 0.0 { -p } else { p }
        };
        let at_share = |t: f32| ((t * (n - 1) as f32).round() as usize).min(n - 1);
        let fins = |kind: FinKind| b.fins.iter().filter(move |f| f.kind == kind);
        // Fins behind the body: back, belly (fans of rays).
        for f in fins(FinKind::Back).chain(fins(FinKind::Belly)) {
            let j = at_share(f.at);
            let (at, dir) = s.plates[j];
            let sc = scale(b, f.at);
            let u = up(dir) * if f.kind == FinKind::Back { 1.0 } else { -1.0 };
            let r = b.size.1 * sc * 0.5;
            let lean = 0.45 + (s.phase - TAU * j as f32 / b.wave.0.max(1.0)).sin() * 0.15;
            let (a0, a1) = (at + dir * f.size.0 * 0.5 + u * r * 0.7, at - dir * f.size.0 * 0.5 + u * r * 0.7);
            let (rc, wc) = (ray(f.color), web(f.color));
            rays(a0, a1, u, -dir, f.size.1 * sc, lean, rc, wc, f.glow.map(rgb), &mut cells, &mut lights);
        }
        if let Some(f) = fins(FinKind::Tail).next() {
            // Its tail's fan: from the end of it out to two tips, swept
            // with the wave.
            let (at, dir) = s.plates[n - 1];
            let u = up(dir);
            let sweep = (s.phase - TAU * (n - 1) as f32 / b.wave.0.max(1.0)).sin() * 0.35;
            let base = at - dir * b.size.0 * scale(b, 1.0) * 0.4;
            let (ta, tb) = (base - dir * f.size.0 + u * f.size.1 * (1.0 + sweep), base - dir * f.size.0 * 0.85 - u * f.size.1 * (1.0 - sweep));
            fin(ta, tb, base, rgb(f.color), &mut cells);
            if let Some(g) = f.glow {
                fin(ta, tb, base, faint(g), &mut lights);
                stroke(base, ta, 0.5, 0.5, rgb(g), &mut lights);
                stroke(base, tb, 0.5, 0.5, rgb(g), &mut lights);
                stroke(ta, tb, 0.5, 0.5, rgb(g), &mut lights);
            }
        }
        // Its plates, tail first: all their outlines, then their fills.
        let shape = |j: usize| {
            let (at, dir) = s.plates[j];
            let sc = scale(b, j as f32 / (n - 1).max(1) as f32);
            let (len, r) = (b.size.0 * sc, b.size.1 * 0.5 * sc);
            (at - dir * len * 0.5, at + dir * len * 0.5, r, up(dir))
        };
        for j in (0..n).rev() {
            let (pa, pb, r, u) = shape(j);
            capsule(pa, pb, r, true, line, fill, lit, belly, u, &mut cells);
        }
        for j in (0..n).rev() {
            let (at, _) = s.plates[j];
            let (pa, pb, r, u) = shape(j);
            capsule(pa, pb, r, false, line, fill, lit, belly, u, &mut cells);
            // Its glow: a spot on every so many plates, a line between.
            if let Some(g) = &b.glow {
                let wave = if g.pulse > 0.0 { 0.55 + 0.45 * (s.glow * g.pulse * TAU - j as f32 * 0.45).sin() } else { 1.0 };
                let wave = (wave + flare_at(s.flare, j as f32 / (n - 1).max(1) as f32)).min(1.0);
                let (cr, cg, cb) = g.color;
                let c = [cr, cg, cb, (255.0 * wave) as u8];
                if j % g.every.max(1) == 0 {
                    let spot = at + u * r * 0.15;
                    stroke(spot, spot, (r * 0.3).max(0.8), (r * 0.3).max(0.8), c, &mut lights);
                } else if g.line {
                    let mid = at - u * r * 0.2;
                    lights.push((mid.floor().as_ivec2(), [cr, cg, cb, (150.0 * wave) as u8]));
                }
            }
        }
        // Its side fins, in front: fanned back and down, flapping with its
        // strokes.
        for f in fins(FinKind::Side) {
            let j = at_share(f.at);
            let (at, dir) = s.plates[j];
            let u = up(dir);
            let flap = 0.5 + 0.5 * s.pulse;
            let out = (-u * flap - dir * 0.9).normalize_or(-dir);
            let (a0, a1) = (at + dir * f.size.0 * 0.2, at - dir * f.size.0 * 0.15);
            rays(a0, a1, out, -dir, f.size.0, 0.2, ray(f.color), web(f.color), f.glow.map(rgb), &mut cells, &mut lights);
        }
        // Its tendrils: lit, flickering if they're charged.
        if let Some(td) = &b.tendrils {
            let (r, g, bl) = td.color;
            for (k, (pts, _)) in s.tendrils.iter().enumerate() {
                let m = pts.len();
                for i in 1..m {
                    let t = i as f32 / m as f32;
                    let w = td.width * (1.0 - t * 0.8);
                    let flick = if td.zap.is_some() { 0.55 + 0.45 * ((s.glow * 40.0 + (i * 13 + k * 7) as f32).sin() > 0.3) as u8 as f32 } else { 1.0 };
                    let c = [r, g, bl, (255.0 * flick * (1.0 - t * 0.5)) as u8];
                    stroke(pts[i - 1], pts[i], w * 0.5, w * 0.45, c, &mut lights);
                }
            }
            // Lightning living on them (charged): short jagged arcs crawling
            // along each, now and then leaping to the next or forking out
            // into the water, new ones every flicker.
            if td.zap.is_some() {
                let beat = (s.glow * ARC_HZ) as u64;
                let n = s.tendrils.len();
                for (k, (pts, _)) in s.tendrils.iter().enumerate() {
                    let m = pts.len();
                    if m < 4 {
                        continue;
                    }
                    let roll = |salt: u64| platypus_sim::rng::hash(&[beat, k as u64, salt]);
                    for a in 0..ARCS {
                        let from = (roll(a) % (m as u64 - 3)) as usize;
                        let to = (from + 2 + (roll(a + 10) % 5) as usize).min(m - 1);
                        arc(&jag(pts[from], pts[to], roll(a + 20), 3.5, 3), &mut lights);
                    }
                    if n > 1 && roll(30) % 3 == 0 {
                        let other = &s.tendrils[(k + 1) % n].0;
                        let i = (roll(31) % m as u64) as usize;
                        if let Some(&q) = other.get(i) {
                            arc(&jag(pts[i], q, roll(32), 4.0, 3), &mut lights);
                        }
                    }
                    if roll(40) % 5 == 0 {
                        let i = (roll(41) % m as u64) as usize;
                        let out = Vec2::from_angle((roll(42) % 628) as f32 / 100.0) * (8.0 + (roll(43) % 12) as f32);
                        arc(&jag(pts[i], pts[i] + out, roll(44), 3.0, 3), &mut lights);
                    }
                }
            }
        }
    }
    if let Some(mut px) = body.0.frame(&mut commands, &mut images, &mut sprites, eye, half, !cells.is_empty()) {
        for (p, c) in &cells {
            px.put(p.x, p.y, *c);
        }
    }
    if let Some(mut px) = glow.0.frame(&mut commands, &mut images, &mut sprites, eye, half, !lights.is_empty()) {
        for (p, c) in &lights {
            px.put(p.x, p.y, *c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The short way round, either way, across the wrap.
    #[test]
    fn round_is_the_short_way() {
        assert!((round(0.1, -0.1) - 0.2).abs() < 1e-5);
        assert!((round(-3.0, 3.0) - (TAU - 6.0)).abs() < 1e-5);
        assert!((round(3.0, -3.0) + (TAU - 6.0)).abs() < 1e-5);
    }

    /// Plates swell to the biggest at `swell` and taper to `taper` at the
    /// tail.
    #[test]
    fn plates_swell_then_taper() {
        let b: BodyDef = ron::from_str("(count: 10, spacing: 5, size: (9, 7), swell: 0.2, taper: 0.4, color: (1, 2, 3), wave: (8.0, 3.0, 0.1, 0.3))").unwrap();
        assert!((scale(&b, 0.0) - 0.8).abs() < 1e-5);
        assert!((scale(&b, 0.2) - 1.0).abs() < 1e-5);
        assert!((scale(&b, 1.0) - 0.4).abs() < 1e-5);
        assert!(scale(&b, 0.6) < 1.0 && scale(&b, 0.6) > 0.4);
    }
}
