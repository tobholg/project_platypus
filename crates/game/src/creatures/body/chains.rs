//! Chains (BE `limbs`, stage 2): tails, necks, a wraith's chains, a
//! scorpion's sting. A run of links from an anchor on the body, each
//! springing toward where it rests (a tail held out behind, a scorpion's
//! curled over its back), with weight: it lags and swings as the body goes,
//! sags if it's slack. One can aim: its end turning to point at what it's
//! after, and moves drive it (a move's pose: `coil` draws its end back,
//! `reach` throws its end out at the target: a scorpion's sting). Drawn
//! as the legs are (`legs.rs`: tapered, outlined), rings at its joints if
//! it's segmented, a sprite at its tip.
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
    /// ahead, 90 up, 180 behind), how much the second turns from it
    /// (degrees: anticlockwise positive, facing right), and how much more
    /// each next one turns than the last (`grow`: the curl tightening
    /// toward the tip): a tail held out (180, 0, 0); a scorpion's rising
    /// nearly straight up from its rear and curling forward over at its end,
    /// the sting poised high over its back (105, -5, -6).
    #[serde(default = "behind")]
    pub rest: f32,
    #[serde(default)]
    pub curl: f32,
    #[serde(default)]
    pub grow: f32,
    /// How many links at its end aim and strike (the rest hold their pose;
    /// else its last three quarters: a sting's base stays upright).
    #[serde(default)]
    pub jab: Option<usize>,
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

/// Link `i`'s way at rest (degrees, facing right).
fn angle(rest: f32, curl: f32, grow: f32, i: usize) -> f32 {
    let i = i as f32;
    rest + curl * i + grow * i * (i - 1.0) * 0.5
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
    /// How many links at its end aim and strike.
    pub fn jabs(&self) -> usize {
        self.jab.unwrap_or((self.links * 3).div_ceil(4)).clamp(1, self.links)
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
            let a = angle(def.rest, def.curl, def.grow, i).to_radians();
            let p = pts[i] + turn(Vec2::from_angle(a)) * def.length;
            pts.push(p);
        }
        Chain { prev: pts.clone(), pts, tip: None }
    }

    /// A frame: the anchor where the body has it now; every joint carried
    /// on by its swing, sagging, drawn toward its rest pose by its
    /// stiffness (from the joint before it, as it is now), each link its
    /// length again. Coiled (a move drawing it back, 0–1), it leans back
    /// (25°) and its end curls twice as tight. Aiming at `aim`, its end (its `jab` links)
    /// turns to point at it, more so toward the tip: the last link at it.
    pub fn step(&mut self, def: &ChainDef, anchor: Vec2, turn: impl Fn(Vec2) -> Vec2, aim: Option<Vec2>, coil: f32, dt: f32) {
        let n = def.links;
        if self.pts.len() != n + 1 {
            *self = Chain { tip: self.tip, ..Chain::rest(def, anchor, &turn) };
        }
        self.pts[0] = anchor;
        self.prev[0] = anchor;
        let coil = if aim.is_some() { coil.clamp(0.0, 1.5) } else { 0.0 };
        let (rest, curl, grow) = (def.rest + 25.0 * coil, def.curl, def.grow * (1.0 + 1.2 * coil));
        let stiff = if aim.is_some() { def.stiff.max(0.8) } else { def.stiff };
        let hold = (stiff * 22.0 * dt).min(1.0);
        let from = n - def.jabs();
        for i in 1..=n {
            // Its swing (a little damped) and its sag.
            let swing = (self.pts[i] - self.prev[i]) * 0.9;
            self.prev[i] = self.pts[i];
            self.pts[i] += swing + Vec2::NEG_Y * def.sag * dt * dt;
            // Toward its pose from the joint before it: at rest, or (its
            // end, aiming) turned toward the target.
            let mut way = turn(Vec2::from_angle(angle(rest, curl, grow, i - 1).to_radians()));
            if let Some(goal) = aim
                && i > from
            {
                let w = (i - from) as f32 / def.jabs() as f32;
                let at = (goal - self.pts[i - 1]).normalize_or(way);
                way = way.lerp(at, w * w).normalize_or(way);
            }
            let to = self.pts[i - 1] + way * def.length;
            self.pts[i] = self.pts[i].lerp(to, hold);
            // Its length again.
            let d = self.pts[i] - self.pts[i - 1];
            self.pts[i] = self.pts[i - 1] + d.normalize_or(turn(Vec2::NEG_X)) * def.length;
        }
    }
}

impl Chain {
    /// A move's say over an aiming chain, after its step: `reach` (0–1)
    /// swung at `aim` (a sting striking): the whole of it tipped forward
    /// from its base and its curl let out some (its shape kept: a whip,
    /// not a stretched rope), as far as brings its tip nearest the target
    /// while keeping over its anchor (not down through its own body): of
    /// up to 100° forward and its curl down to a tenth, the best; blended
    /// in by `reach`.
    pub fn strike(&mut self, def: &ChainDef, anchor: Vec2, turn: impl Fn(Vec2) -> Vec2, aim: Option<Vec2>, reach: f32) {
        let n = self.pts.len();
        let (Some(goal), true) = (aim, reach > 0.0 && n >= 2) else { return };
        let lay = |rest: f32, grow: f32| {
            let mut pts = vec![anchor];
            for i in 0..def.links {
                let a = angle(rest, def.curl, grow, i).to_radians();
                let p = pts[i] + turn(Vec2::from_angle(a)) * def.length;
                pts.push(p);
            }
            pts
        };
        // (Forward: toward 0° facing right, whichever side it rests.)
        let forward = if def.rest > 0.0 { -1.0 } else { 1.0 };
        let mut best = (f32::MAX, def.rest, def.grow);
        for i in 0..=20 {
            let rest = def.rest + forward * 5.0 * i as f32;
            for j in 1..=10 {
                let grow = def.grow * j as f32 / 10.0;
                let pts = lay(rest, grow);
                // Over its anchor all but its last two joints (a sting comes
                // down onto what's ahead, not through its own back).
                let under = pts[1..n - 2].iter().any(|p| p.y < anchor.y - 1.0);
                let tip = pts[n - 1];
                let score = tip.distance(goal) + if under { 1000.0 } else { 0.0 } + 0.02 * (i as f32 + (10 - j) as f32);
                if score < best.0 {
                    best = (score, rest, grow);
                }
            }
        }
        let r = reach.clamp(0.0, 1.0);
        let swung = lay(def.rest + (best.1 - def.rest) * r, def.grow + (best.2 - def.grow) * r);
        for (p, q) in self.pts.iter_mut().zip(&swung).skip(1) {
            *p = p.lerp(*q, r.sqrt());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tail() -> ChainDef {
        ChainDef { anchor: (0.0, 0.0), links: 6, length: 5.0, rest: 180.0, curl: 0.0, grow: 0.0, jab: None, stiff: 0.6, sag: 0.0, width: vec![4.0, 1.0], color: None, rings: false, tip: None, aims: None, far: false, behind: false }
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
    fn a_sting_poised_high_points_and_jabs_at_its_target() {
        let def = ChainDef { links: 8, rest: 105.0, curl: -5.0, grow: -6.0, aims: Some(80.0), ..tail() };
        let right = |v: Vec2| v;
        let mut c = Chain::rest(&def, Vec2::ZERO, right);
        // Poised: rising tall over its anchor, its end curled forward over
        // (the tip ahead of the top of the curve, pointing down and ahead).
        let top = c.pts.iter().map(|p| p.y).fold(f32::MIN, f32::max);
        assert!(top > 20.0, "tall: its top {top:.1} up");
        let (a, b) = (c.pts[7], c.pts[8]);
        assert!(b.x > a.x && b.y < a.y, "its sting pointing down and ahead: {a:?} to {b:?}");
        // Aiming at something ahead and low: its sting points at it, its
        // base as it was.
        let goal = Vec2::new(30.0, 0.0);
        for _ in 0..60 {
            c.step(&def, Vec2::ZERO, right, Some(goal), 0.0, 1.0 / 60.0);
        }
        let base = c.pts[2];
        let (a, b) = (c.pts[7], c.pts[8]);
        let point = (b - a).normalize().dot((goal - b).normalize());
        assert!(point > 0.8, "pointing at it ({point:.2}): {:?}", c.pts);
        // Struck: swung forward from its base, curl kept, its tip much
        // nearer; none of it down through its anchor's level but its end.
        let _ = base;
        let before = c.pts[8].distance(goal);
        c.strike(&def, Vec2::ZERO, right, Some(goal), 1.0);
        assert!(c.pts[8].distance(goal) < before * 0.5, "swung out: {:?} (was {before:.1} off)", c.pts[8]);
        assert!(c.pts[1..7].iter().all(|p| p.y > -1.0), "over its anchor: {:?}", c.pts);
        for w in c.pts.windows(2) {
            assert!(((w[1] - w[0]).length() - 5.0).abs() < 0.01);
        }
    }
}
