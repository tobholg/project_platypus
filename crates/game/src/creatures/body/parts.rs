//! Hit areas per part (BE `limbs` stage 4): a body of limbs is more than
//! its box. Each legged creature keeps its parts' shapes as they are this
//! frame (`legs.rs` fills them in after it walks): its legs, arms,
//! segments and chains, each a capsule (a line with a radius), and its
//! weak spots (circles on its body). What tests a blow against a creature
//! (a swing, an arrow, a spell) takes a part as a hit too, so a blade across
//! a centipede's tail or a spider's leg lands; and every hit's damage is
//! scaled by what it struck (`mult_at`: legs and arms take less, a weak
//! spot more).

use bevy::prelude::*;
use serde::Deserialize;

/// What a part is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartKind {
    Leg,
    Arm,
    Segment,
    Chain,
    Weak,
}

/// A part as it is now (the world): a capsule from `a` to `b`, `r` round,
/// its hurt scaled by `mult`.
#[derive(Clone, Debug)]
pub struct Part {
    pub a: Vec2,
    pub b: Vec2,
    pub r: f32,
    pub mult: f32,
    pub kind: PartKind,
}

impl Part {
    /// How far `p` is outside it (0 or less: inside).
    pub fn gap(&self, p: Vec2) -> f32 {
        let d = self.b - self.a;
        let t = if d.length_squared() > 0.0 { ((p - self.a).dot(d) / d.length_squared()).clamp(0.0, 1.0) } else { 0.0 };
        p.distance(self.a + d * t) - self.r
    }
}

/// A creature's parts this frame, and the box round them all.
#[derive(Component, Clone, Debug, Default)]
pub struct Parts {
    pub list: Vec<Part>,
    pub lo: Vec2,
    pub hi: Vec2,
}

impl Parts {
    /// Laid out afresh: the parts and the box round them.
    pub fn set(&mut self, list: Vec<Part>) {
        let (lo, hi) = list.iter().fold((Vec2::MAX, Vec2::MIN), |(lo, hi), p| (lo.min(p.a.min(p.b) - p.r), hi.max(p.a.max(p.b) + p.r)));
        self.list = list;
        (self.lo, self.hi) = (lo, hi);
    }

    /// Anywhere near them at all (within `pad` of their box).
    pub fn near(&self, p: Vec2, pad: f32) -> bool {
        !self.list.is_empty() && p.cmpge(self.lo - pad).all() && p.cmple(self.hi + pad).all()
    }

    /// The part `p` is in (within `pad` of), if any: a weak spot first.
    pub fn touch(&self, p: Vec2, pad: f32) -> Option<&Part> {
        if !self.near(p, pad) {
            return None;
        }
        let mut hit = self.list.iter().filter(|q| q.gap(p) <= pad);
        let first = hit.next()?;
        Some(std::iter::once(first).chain(hit).find(|q| q.kind == PartKind::Weak).unwrap_or(first))
    }

    /// How much a blow at `p` hurts, against a blow to its body: a weak
    /// spot's (or a limb's) share; 1 elsewhere.
    pub fn mult_at(&self, p: Vec2) -> f32 {
        self.touch(p, 1.0).map_or(1.0, |q| q.mult)
    }
}

/// How its parts take blows (a creature file's `legs`' `parts`): legs,
/// arms, segments and chains a share of a blow to its body each; weak
/// spots on its body.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct PartsDef {
    pub legs: f32,
    pub arms: f32,
    pub segments: f32,
    pub chains: f32,
    pub weak: Vec<WeakDef>,
}

impl Default for PartsDef {
    fn default() -> Self {
        PartsDef { legs: 0.6, arms: 0.8, segments: 1.0, chains: 0.8, weak: Vec::new() }
    }
}

/// A weak spot: a circle `r` round at `at` on its body (cells from its
/// grip, facing right, y up), a blow there hurting `mult` times as much.
#[derive(Clone, Debug, Deserialize)]
pub struct WeakDef {
    pub at: (f32, f32),
    pub r: f32,
    pub mult: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blow_lands_on_the_part_it_strikes_a_weak_spot_first() {
        let mut parts = Parts::default();
        parts.set(vec![
            Part { a: Vec2::new(0.0, 0.0), b: Vec2::new(40.0, 0.0), r: 3.0, mult: 1.0, kind: PartKind::Segment },
            Part { a: Vec2::new(10.0, 0.0), b: Vec2::new(10.0, -12.0), r: 1.0, mult: 0.6, kind: PartKind::Leg },
            Part { a: Vec2::new(35.0, 1.0), b: Vec2::new(35.0, 1.0), r: 2.0, mult: 2.5, kind: PartKind::Weak },
        ]);
        assert_eq!(parts.touch(Vec2::new(20.0, 2.0), 0.0).map(|p| p.kind), Some(PartKind::Segment));
        assert_eq!(parts.touch(Vec2::new(10.0, -10.0), 0.0).map(|p| p.kind), Some(PartKind::Leg));
        assert_eq!(parts.mult_at(Vec2::new(35.0, 0.5)), 2.5, "the weak spot, over the segment it's on");
        assert!(parts.touch(Vec2::new(20.0, 10.0), 0.0).is_none(), "clear of it");
        assert_eq!(parts.mult_at(Vec2::new(100.0, 0.0)), 1.0);
    }
}
