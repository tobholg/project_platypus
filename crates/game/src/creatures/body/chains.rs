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
}

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
        Chain { prev: pts.clone(), pts, tip: None }
    }

    /// A frame: the anchor where the body has it now; every joint carried
    /// on by its swing, sagging, drawn toward its pose by its stiffness
    /// (from the joint before it, as it is now), each link its length
    /// again. Its pose is its rest pose, or, reaching for `aim`, the arch
    /// most like it whose last link points at it (`aim_pose`).
    pub fn step(&mut self, def: &ChainDef, anchor: Vec2, turn: impl Fn(Vec2) -> Vec2, aim: Option<Vec2>, dt: f32) {
        let n = def.links;
        if self.pts.len() != n + 1 {
            *self = Chain { tip: self.tip, ..Chain::rest(def, anchor, &turn) };
        }
        self.pts[0] = anchor;
        self.prev[0] = anchor;
        let (rest, curl) = match aim {
            Some(goal) => aim_pose(def, anchor, &turn, goal),
            None => (def.rest, def.curl),
        };
        // (Aiming, it holds its pose harder: a sting poised.)
        let stiff = if aim.is_some() { def.stiff.max(0.8) } else { def.stiff };
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

/// The arch a chain reaching for `goal` takes: of its rest pose's way up
/// (±40°) and curl (from straighter to half as tight again), the one whose
/// last link points most nearly at the goal, its tip nearest it, keeping
/// over its anchor as its rest pose is (a sting stays over the back), and
/// least changed from rest. (A handful of links, 81 poses: cheap.)
pub fn aim_pose(def: &ChainDef, anchor: Vec2, turn: impl Fn(Vec2) -> Vec2, goal: Vec2) -> (f32, f32) {
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
            let score = point * 2.0 + near + change * 0.3;
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
            c.step(&def, at, right, None, 1.0 / 60.0);
        }
        assert!(c.pts[6].y < at.y - 2.0, "the tip lags: {:?} under {:?}", c.pts[6], at);
        for _ in 0..240 {
            c.step(&def, at, right, None, 1.0 / 60.0);
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
            c.step(&def, Vec2::ZERO, right, Some(goal), 1.0 / 60.0);
        }
        let (a, b) = (c.pts[5], c.pts[6]);
        assert!(b.y > 0.0, "the tip over its anchor: {b:?}");
        let point = (b - a).normalize().dot((goal - b).normalize());
        assert!(point > 0.8, "pointing at it ({point:.2}): {:?}", c.pts);
    }
}
