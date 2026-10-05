//! Chains (BE `limbs`, stage 2): tails, necks, a wraith's chains, a
//! scorpion's sting. A run of links from an anchor on the body, each
//! springing toward where it rests (a tail held out behind, a scorpion's
//! curled over its back), with weight: it lags and swings as the body goes,
//! sags if it's slack. One can aim: its tip reaching for what it's after
//! (FABRIK over the links). Drawn as the legs are (`legs.rs`: tapered,
//! outlined), rings at its joints if it's segmented, a sprite at its tip.
//!
//! The state lives with the legs (`Legs::chains`): a chain is part of the
//! same rig, seen the same way (its rest pose turns and mirrors with the
//! body).

use bevy::prelude::*;
use serde::Deserialize;

/// A chain, as a creature file's `legs` writes it (`chains`).
#[derive(Clone, Debug, Deserialize)]
pub struct ChainDef {
    /// Where it hangs from, from the body's `grip` (cells, facing right,
    /// y up).
    pub anchor: (f32, f32),
    /// How many links, and how long each (cells).
    pub links: usize,
    pub length: f32,
    /// Which way its first link goes at rest (degrees, facing right: 0
    /// ahead, 90 up, 180 behind), and how much each next link turns from
    /// the last (degrees: anticlockwise positive, facing right): a tail
    /// held out (180, 0), a scorpion's up and back from its rear and arching
    /// forward over its back (140, -28).
    #[serde(default = "behind")]
    pub rest: f32,
    #[serde(default)]
    pub curl: f32,
    /// How hard it holds its rest pose (0: a rope, 1: rigid), and how much
    /// it sags (cells/s² of its own: slack chains, heavy tails).
    #[serde(default = "half")]
    pub stiff: f32,
    #[serde(default)]
    pub sag: f32,
    /// How thick at its base and its tip (cells; between them, tapering).
    #[serde(default)]
    pub width: Vec<f32>,
    /// Its colour (else the legs').
    #[serde(default)]
    pub color: Option<(u8, u8, u8)>,
    /// Rings drawn across its joints (a scorpion's tail, a centipede's).
    #[serde(default)]
    pub rings: bool,
    /// A sprite at its tip (pointing right, its `grip` where it joins),
    /// turned to the last link.
    #[serde(default)]
    pub tip: Option<String>,
    /// It reaches its tip for the nearest thing it hunts within this many
    /// cells (a scorpion's sting).
    #[serde(default)]
    pub aims: Option<f32>,
    /// On the far side: behind the body, darker. `behind`: behind the body
    /// as it is (a tail from its back).
    #[serde(default)]
    pub far: bool,
    #[serde(default)]
    pub behind: bool,
}

fn behind() -> f32 {
    180.0
}
fn half() -> f32 {
    0.5
}

/// A chain as it is now: its joints (the world: the anchor first), where
/// they were last frame (their swing), its tip's sprite.
#[derive(Clone, Debug, Default)]
pub struct Chain {
    pub pts: Vec<Vec2>,
    prev: Vec<Vec2>,
    pub tip: Option<Entity>,
    /// The arch it holds aiming (its way up, its curl: degrees), eased
    /// toward the best one rather than jumping between them.
    pose: Option<(f32, f32)>,
}

/// How far of its length a striking chain throws its tip (the rest arcs).
const STRIKE_REACH: f32 = 0.8;

/// How fast an aiming chain's arch changes (degrees a second).
const ARCH_RATE: f32 = 140.0;

impl ChainDef {
    /// How far it reaches, base to tip.
    pub fn reach(&self) -> f32 {
        self.links as f32 * self.length
    }

    /// Its thickness `t` of the way from base (0) to tip (1).
    pub fn width_at(&self, t: f32) -> f32 {
        match self.width.as_slice() {
            [] => 2.0,
            [w] => *w,
            ws => {
                let f = t.clamp(0.0, 1.0) * (ws.len() - 1) as f32;
                let i = (f.floor() as usize).min(ws.len() - 2);
                ws[i] + (ws[i + 1] - ws[i]) * (f - i as f32)
            }
        }
    }
}

impl Chain {
    /// One with a sprite at its tip (its joints laid out on its first step).
    pub fn with_tip(tip: Option<Entity>) -> Chain {
        Chain { tip, ..Default::default() }
    }

    /// At rest: each link where its rest pose puts it (`turn`: the body's
    /// frame to the world's: mirrored to its facing, tilted).
    pub fn rest(def: &ChainDef, anchor: Vec2, turn: impl Fn(Vec2) -> Vec2) -> Chain {
        let mut pts = vec![anchor];
        for i in 0..def.links {
            let a = (def.rest + def.curl * i as f32).to_radians();
            let p = pts[i] + turn(Vec2::from_angle(a)) * def.length;
            pts.push(p);
        }
        Chain { prev: pts.clone(), pts, tip: None, pose: None }
    }

    /// A frame: the anchor where the body has it now; every joint carried
    /// on by its swing, sagging, drawn toward its pose by its stiffness
    /// (from the joint before it, as it is now), each link its length
    /// again. Its pose is its rest pose, or, reaching for `aim`, the arch
    /// most like it whose last link points at it (`aim_pose`).
    pub fn step(&mut self, def: &ChainDef, anchor: Vec2, turn: impl Fn(Vec2) -> Vec2, aim: Option<Vec2>, coil: f32, dt: f32) {
        let n = def.links;
        if self.pts.len() != n + 1 {
            *self = Chain { tip: self.tip, ..Chain::rest(def, anchor, &turn) };
        }
        self.pts[0] = anchor;
        self.prev[0] = anchor;
        let (rest, curl) = match aim {
            Some(goal) => {
                let now = self.pose.unwrap_or((def.rest, def.curl));
                let best = aim_pose(def, anchor, &turn, goal, now);
                let ease = |a: f32, b: f32| a + (b - a).clamp(-ARCH_RATE * dt, ARCH_RATE * dt);
                let pose = (ease(now.0, best.0), ease(now.1, best.1));
                self.pose = Some(pose);
                pose
            }
            None => {
                self.pose = None;
                (def.rest, def.curl)
            }
        };
        // Coiled (a move drawing it back): its arch more upright and tighter.
        let coil = if aim.is_some() { coil.clamp(0.0, 1.5) } else { 0.0 };
        let toward_up = if (def.rest - 90.0).abs() < 1.0 { 0.0 } else { (90.0 - def.rest).signum() };
        let (rest, curl) = (rest + toward_up * 30.0 * coil, curl * (1.0 + 0.5 * coil));
        // (Aiming, it holds its pose harder: a sting poised.)
        let stiff = if aim.is_some() { def.stiff.max(0.8) } else { def.stiff };
        let stiff = if coil > 0.0 { 1.0 } else { stiff };
        let hold = (stiff * 22.0 * dt).min(1.0);
        for i in 1..=n {
            // Its swing (a little damped) and its sag.
            let swing = (self.pts[i] - self.prev[i]) * 0.9;
            self.prev[i] = self.pts[i];
            self.pts[i] += swing + Vec2::NEG_Y * def.sag * dt * dt;
            // Toward its pose from the joint before it.
            let a = (rest + curl * (i - 1) as f32).to_radians();
            let to = self.pts[i - 1] + turn(Vec2::from_angle(a)) * def.length;
            self.pts[i] = self.pts[i].lerp(to, hold);
            // Its length again.
            let d = self.pts[i] - self.pts[i - 1];
            self.pts[i] = self.pts[i - 1] + d.normalize_or(turn(Vec2::NEG_X)) * def.length;
        }
    }
}

impl Chain {
    /// A move's say over an aiming chain, after its step: `reach` (0–1)
    /// thrown out from its arch to its tip at `aim` (no further than 80 % of
    /// its length), along an arc bulging up as long as it is (over its
    /// back: a sting comes down onto what it strikes).
    pub fn strike(&mut self, def: &ChainDef, anchor: Vec2, aim: Option<Vec2>, reach: f32) {
        let n = self.pts.len();
        let (Some(goal), true) = (aim, reach > 0.0 && n >= 2) else { return };
        // (No further than most of its length: slack left to arc over; the
        // tip comes down short of a target past it.)
        let goal = anchor + (goal - anchor).clamp_length_max(def.reach() * STRIKE_REACH);
        let arc = arc_to(anchor, goal, def.links, def.length);
        let r = reach.clamp(0.0, 1.0);
        for i in 1..n {
            self.pts[i] = self.pts[i].lerp(arc[i], r);
        }
        self.relink(def);
        // (Struck out, it doesn't swing back on its own: the move draws it.)
        self.prev.clone_from(&self.pts);
    }

    /// Each link its length again, from the anchor out.
    fn relink(&mut self, def: &ChainDef) {
        for i in 1..self.pts.len() {
            let d = self.pts[i] - self.pts[i - 1];
            self.pts[i] = self.pts[i - 1] + d.normalize_or(Vec2::Y) * def.length;
        }
    }
}

/// `links` joints `length` apart from `from` to `to` along a circle's arc
/// bulging up (the arc's length theirs): its chord `to - from`; if that's
/// longer than they reach, straight at it as far as they go.
pub fn arc_to(from: Vec2, to: Vec2, links: usize, length: f32) -> Vec<Vec2> {
    let total = links as f32 * length;
    let chord = to - from;
    let d = chord.length();
    if d >= total * 0.999 || d < 0.01 {
        let dir = chord.normalize_or(Vec2::Y);
        return (0..=links).map(|i| from + dir * length * i as f32).collect();
    }
    // The arc's angle: total / d = θ / (2 sin(θ/2)), by bisection on 0..2π.
    let ratio = total / d;
    let (mut lo, mut hi) = (1e-4f32, std::f32::consts::TAU - 1e-3);
    for _ in 0..40 {
        let mid = (lo + hi) * 0.5;
        if mid / (2.0 * (mid * 0.5).sin()) < ratio { lo = mid } else { hi = mid }
    }
    let theta = (lo + hi) * 0.5;
    let radius = total / theta;
    // The centre: from the chord's middle, away from the bulge (up).
    let mut n = chord.perp().normalize();
    if n.y < 0.0 {
        n = -n;
    }
    let centre = from + chord * 0.5 - n * radius * (theta * 0.5).cos();
    let a0 = (from - centre).to_angle();
    let a1 = (to - centre).to_angle();
    // (The way round that passes the bulge: over the top.)
    let mut sweep = (a1 - a0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    let top = centre + n * radius;
    let mid_angle = a0 + sweep * 0.5;
    if (centre + Vec2::from_angle(mid_angle) * radius).distance(top) > radius * 0.5 {
        sweep -= sweep.signum() * std::f32::consts::TAU;
    }
    (0..=links).map(|i| centre + Vec2::from_angle(a0 + sweep * i as f32 / links as f32) * radius).collect()
}

/// The arch a chain reaching for `goal` takes: of its rest pose's way up
/// (±40°) and curl (from straighter to half as tight again), the one whose
/// last link points most nearly at the goal, its tip nearest it, keeping
/// over its anchor as its rest pose is (a sting stays over the back), and
/// least changed from rest; and from the arch it holds `now` (a better one
/// must be better by more than the change: it doesn't flick between two
/// nearly as good). (A handful of links, 81 poses: cheap.)
pub fn aim_pose(def: &ChainDef, anchor: Vec2, turn: impl Fn(Vec2) -> Vec2, goal: Vec2, now: (f32, f32)) -> (f32, f32) {
    let rest_tip = Chain::rest(def, anchor, &turn).pts.last().copied().unwrap_or(anchor);
    let over = rest_tip.y > anchor.y;
    let mut best = (f32::MAX, def.rest, def.curl);
    for i in 0..9 {
        let rest = def.rest - 40.0 + 10.0 * i as f32;
        for j in 0..9 {
            let curl = def.curl * (0.2 + 0.165 * j as f32);
            let mut p = anchor;
            let mut last = Vec2::X;
            for k in 0..def.links {
                last = turn(Vec2::from_angle((rest + curl * k as f32).to_radians()));
                p += last * def.length;
            }
            if over && p.y < anchor.y {
                continue;
            }
            let point = 1.0 - last.dot((goal - p).normalize_or(last));
            let near = p.distance(goal) / def.reach().max(1.0);
            let change = ((rest - def.rest).abs() + (curl - def.curl).abs()) / 180.0;
            let moved = ((rest - now.0).abs() + (curl - now.1).abs()) / 180.0;
            let score = point * 2.0 + near + change * 0.3 + moved * 0.8;
            if score < best.0 {
                best = (score, rest, curl);
            }
        }
    }
    (best.1, best.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tail() -> ChainDef {
        ChainDef { anchor: (0.0, 0.0), links: 6, length: 5.0, rest: 180.0, curl: 0.0, stiff: 0.6, sag: 0.0, width: vec![4.0, 1.0], color: None, rings: false, tip: None, aims: None, far: false, behind: false }
    }

    #[test]
    fn a_tail_holds_out_behind_and_swings_after_the_body() {
        let def = tail();
        let right = |v: Vec2| v;
        let mut c = Chain::rest(&def, Vec2::ZERO, right);
        // At rest: straight out behind, each link its length.
        assert!((c.pts[6] - Vec2::new(-30.0, 0.0)).length() < 0.01);
        // The body goes up fast: the tail lags below, then comes back level.
        let mut at = Vec2::ZERO;
        for _ in 0..10 {
            at += Vec2::new(0.0, 3.0);
            c.step(&def, at, right, None, 0.0, 1.0 / 60.0);
        }
        assert!(c.pts[6].y < at.y - 2.0, "the tip lags: {:?} under {:?}", c.pts[6], at);
        for _ in 0..240 {
            c.step(&def, at, right, None, 0.0, 1.0 / 60.0);
        }
        assert!((c.pts[6].y - at.y).abs() < 1.0, "back out level: {:?}", c.pts[6]);
        for w in c.pts.windows(2) {
            assert!(((w[1] - w[0]).length() - 5.0).abs() < 0.01);
        }
    }

    #[test]
    fn a_sting_keeps_over_its_back_and_points_at_its_target() {
        let def = ChainDef { rest: 140.0, curl: -28.0, aims: Some(80.0), ..tail() };
        let right = |v: Vec2| v;
        let mut c = Chain::rest(&def, Vec2::ZERO, right);
        // Ahead and low (where a scorpion's prey is): the sting arched over,
        // its last link pointing at it.
        let goal = Vec2::new(30.0, -4.0);
        for _ in 0..60 {
            c.step(&def, Vec2::ZERO, right, Some(goal), 0.0, 1.0 / 60.0);
        }
        let (a, b) = (c.pts[5], c.pts[6]);
        assert!(b.y > 0.0, "the tip over its anchor: {b:?}");
        let point = (b - a).normalize().dot((goal - b).normalize());
        assert!(point > 0.8, "pointing at it ({point:.2}): {:?}", c.pts);
        // Its target edging about (someone shifting their feet): the sting
        // holds steady, no flicking from one arch to another.
        let mut most: f32 = 0.0;
        for k in 0..120 {
            let wobble = goal + Vec2::new((k as f32 * 0.7).sin() * 3.0, 0.0);
            let before = c.pts[6];
            c.step(&def, Vec2::ZERO, right, Some(wobble), 0.0, 1.0 / 60.0);
            most = most.max(c.pts[6].distance(before));
        }
        assert!(most < 1.0, "its tip moved at most {most:.2} a frame");
        // Struck at something level with it and in reach: over the top in
        // an arc, coming down onto it; each link its length.
        let near = Vec2::new(20.0, 0.0);
        c.strike(&def, Vec2::ZERO, Some(near), 1.0);
        assert!(c.pts[6].distance(near) < 1.5, "struck home: {:?}", c.pts[6]);
        assert!(c.pts[3].y > 6.0, "over the top: {:?}", c.pts);
        for w in c.pts.windows(2) {
            assert!(((w[1] - w[0]).length() - 5.0).abs() < 0.01);
        }
    }
}
