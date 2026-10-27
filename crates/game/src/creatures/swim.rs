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
//! back and belly, at its sides, its tail's fan) swaying with it; spots
//! and lines of light along its flanks pulsing in waves down it. Its plates
//! take blows (`Parts`). Drawn on two canvases: its body lit as the world
//! is, its glow over the light (as glowfish are).

use std::collections::VecDeque;
use std::f32::consts::TAU;

use bevy::prelude::*;
use serde::Deserialize;

use super::body::animation::{Animator, Hug};
use super::body::legs::{plate, stroke};
use super::body::parts::{Part, PartKind, Parts};
use super::{Controls, Kinematics, MoveStats};
use crate::canvas::{Canvas, CanvasSprites};
use crate::world::{ChunkLoader, TickSet};

pub struct SwimPlugin;

impl Plugin for SwimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BodyCanvas>()
            .init_resource::<GlowCanvas>()
            .add_systems(FixedUpdate, (grow, strokes).chain().after(super::custom::CustomSet::Think).in_set(TickSet::Intent))
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
/// (so many a second; 0: steady).
#[derive(Clone, Debug, Deserialize)]
pub struct GlowDef {
    pub color: (u8, u8, u8),
    pub every: usize,
    #[serde(default)]
    pub pulse: f32,
    #[serde(default)]
    pub line: bool,
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
    /// Its stroke count, for jitter.
    strokes: u32,
}

impl Swimmer {
    fn new(heading: f32) -> Swimmer {
        Swimmer { heading, stroke_left: 0.0, pulse: 0.0, burst: 0.0, phase: 0.0, glow: 0.0, way: VecDeque::new(), plates: Vec::new(), strokes: 0 }
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
/// The wave runs down it this many times a second gliding, and this many
/// more at a stroke's height.
const WAVE_HZ: (f32, f32) = (0.6, 2.4);

/// The way from `b` round to `a` (radians, the short way).
fn round(a: f32, b: f32) -> f32 {
    (a - b + TAU * 1.5).rem_euclid(TAU) - TAU * 0.5
}

/// A swimmer's strokes and glide; its drag to its movement.
fn grow(mut commands: Commands, mut q: Query<(Entity, &Animator, &Kinematics, Option<&mut MoveStats>), Without<Swimmer>>) {
    for (e, anim, k, stats) in &mut q {
        let Some(sd) = anim.def.swim.as_ref() else { continue };
        if let Some(mut s) = stats {
            s.0.stroke_drag = sd.drag;
        }
        commands.entity(e).insert((Swimmer::new(if k.loco.facing < 0.0 { std::f32::consts::PI } else { 0.0 }), Hug::default()));
    }
}

/// Stroke by stroke where its controls say, as keenly as they say; its
/// head the way it swims (still, turned to what it aims at).
fn strokes(mut q: Query<(&mut Swimmer, &mut Controls, &mut Kinematics, &Animator)>) {
    for (mut s, mut c, mut k, anim) in &mut q {
        let Some(sd) = anim.def.swim.as_ref() else { continue };
        s.pulse *= (-DT / PULSE_FADES).exp();
        s.burst -= DT;
        s.stroke_left -= DT;
        if k.loco.contacts.submerged < 0.5 {
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

/// Where its body joins its head: its art's `grip` (turned with its head),
/// else its middle.
fn neck(anim: &Animator, k: &Kinematics, turn: f32) -> Vec2 {
    super::custom::frame_anchor(anim, k, "grip", "").map_or(k.body.pos, |g| k.body.pos + Vec2::from_angle(turn).rotate(g - k.body.pos))
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
fn follow(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Swimmer, &Kinematics, &Animator, Option<&mut Parts>, &mut Hug)>) {
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
        let total = b.spacing * (n as f32 + 1.0);
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
        let (mut i, mut walked) = (0usize, 0.0);
        for j in 0..n {
            let want = b.spacing * (j as f32 + 1.0);
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
    }
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

fn dark((r, g, b): (u8, u8, u8)) -> (u8, u8, u8) {
    ((r as f32 * 0.6) as u8, (g as f32 * 0.6) as u8, (b as f32 * 0.6) as u8)
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
    q: Query<(&Swimmer, &Animator)>,
    mut sprites: CanvasSprites,
) {
    let Ok((cam, loader)) = camera.single() else { return };
    let (eye, half) = (cam.translation().truncate(), loader.half_extent);
    let mut cells: Vec<(IVec2, [u8; 4])> = Vec::new();
    let mut lights: Vec<(IVec2, [u8; 4])> = Vec::new();
    for (s, anim) in &q {
        let Some(b) = anim.def.swim.as_ref().and_then(|sd| sd.body.as_ref()) else { continue };
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
        // Fins behind the body: back, belly, the far side's.
        for f in fins(FinKind::Back).chain(fins(FinKind::Belly)) {
            let j = at_share(f.at);
            let (at, dir) = s.plates[j];
            let sc = scale(b, f.at);
            let u = up(dir) * if f.kind == FinKind::Back { 1.0 } else { -1.0 };
            let r = b.size.1 * sc * 0.5;
            let lean = (s.phase - TAU * j as f32 / b.wave.0.max(1.0)).sin() * 0.25;
            let (a0, a1) = (at + dir * f.size.0 * 0.5 + u * r * 0.6, at - dir * f.size.0 * 0.5 + u * r * 0.6);
            let tip = at - dir * f.size.0 * (0.35 + lean) + u * (r + f.size.1 * sc);
            fin(a0, a1, tip, rgb(f.color), &mut cells);
            if let Some(g) = f.glow {
                stroke(a0, tip, 0.5, 0.5, rgb(g), &mut lights);
                stroke(tip, a1, 0.5, 0.5, rgb(g), &mut lights);
            }
        }
        if let Some(f) = fins(FinKind::Tail).next() {
            let (at, dir) = s.plates[n - 1];
            let u = up(dir);
            let sweep = (s.phase - TAU * (n - 1) as f32 / b.wave.0.max(1.0)).sin() * 0.35;
            let base = at - dir * b.size.0 * scale(b, 1.0) * 0.4;
            for side in [1.0f32, -1.0] {
                let tip = base - dir * f.size.0 + u * (f.size.1 * side + f.size.1 * sweep);
                fin(base + u * side * 1.5, base - u * side * 0.5, tip, rgb(f.color), &mut cells);
                if let Some(g) = f.glow {
                    stroke(base, tip, 0.5, 0.5, rgb(g), &mut lights);
                }
            }
        }
        // Its plates, tail first.
        for j in (0..n).rev() {
            let (at, dir) = s.plates[j];
            let sc = scale(b, j as f32 / (n - 1).max(1) as f32);
            let (len, r) = (b.size.0 * sc, b.size.1 * 0.5 * sc);
            let (pa, pb) = (at - dir * len * 0.5, at + dir * len * 0.5);
            let u = up(dir);
            stroke(pa, pb, r + 1.0, r + 1.0, line, &mut cells);
            stroke(pa, pb, r, r, fill, &mut cells);
            let inset = dir * r * 0.6;
            plate(pa + inset, pb - inset, r, fill, Some(belly), &mut cells, u);
            plate(pa + inset, pb - inset, r, fill, Some(lit), &mut cells, -u);
            // Its glow: a spot on every so many plates, a line between.
            if let Some(g) = &b.glow {
                let wave = if g.pulse > 0.0 { 0.55 + 0.45 * (s.glow * g.pulse * TAU - j as f32 * 0.45).sin() } else { 1.0 };
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
        // Its side fins, in front.
        for f in fins(FinKind::Side) {
            let j = at_share(f.at);
            let (at, dir) = s.plates[j];
            let u = up(dir);
            let flap = 0.4 + 0.6 * s.pulse;
            let tip = at - dir * f.size.0 - u * f.size.1 * flap;
            fin(at + dir * 2.0, at - dir * 2.0, tip, rgb(f.color), &mut cells);
            if let Some(g) = f.glow {
                stroke(at, tip, 0.5, 0.5, rgb(g), &mut lights);
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
