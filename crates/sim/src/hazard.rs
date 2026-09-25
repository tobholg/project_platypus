//! What the world does to a body in it (SPEC §3.12). One rule for every
//! material, read from its data, so a new material hurts (or doesn't) with no
//! creature code. The game applies it to creatures each tick.

use crate::cell::flags;
use crate::coords::CellPos;
use crate::material::{Kind, MaterialId};
use crate::world::World;

/// Above this (°C), heat hurts.
pub const HARMFUL_HEAT: i32 = 60;
/// Damage per second per °C above `HARMFUL_HEAT`: lava (1 200 °C) ≈ 114/s,
/// scalding steam (~150 °C) ≈ 9/s.
const HEAT_DAMAGE: f32 = 0.1;
/// Below this (°C) it chills; fully chilled 60 °C further down.
pub const CHILLING_COLD: i32 = -10;
/// Below this (°C) cold also hurts, at `HEAT_DAMAGE` per °C.
pub const HARMFUL_COLD: i32 = -60;

/// What a body overlapping some cells is exposed to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Exposure {
    /// Damage per second from temperature: the hottest (or, below
    /// `HARMFUL_COLD`, coldest) cell it touches.
    pub heat: f32,
    /// Damage per second from corrosives (the worst it touches).
    pub corrosion: f32,
    /// Touching flames or something burning: it catches fire.
    pub ignites: bool,
    /// What it's getting coated in: the liquid it's in most of, else
    /// anything with a coating it touches (snow). See `MaterialDef::coats`.
    pub coat: Option<MaterialId>,
    /// Share of the body inside liquid.
    pub submerged: f32,
    /// 0..1: how chilled (slowed) the coldest cell it touches makes it.
    pub cold: f32,
}

impl World {
    /// Exposure of a body covering the cells from `min` to `max` (inclusive).
    /// It touches the ring of cells around that too (the floor under its
    /// feet, a wall beside it); being in water counts only inside. Worst
    /// cell wins, so a bigger body isn't hurt more.
    pub fn exposure(&self, min: CellPos, max: CellPos) -> Exposure {
        let mats = self.materials();
        let mut e = Exposure::default();
        let mut liquid = 0;
        // Most common coating liquid inside, and any coating touched.
        let mut soaking: Vec<(MaterialId, u32)> = Vec::new();
        let mut touched = None;
        for y in min.y - 1..=max.y + 1 {
            for x in min.x - 1..=max.x + 1 {
                let inside = x >= min.x && x <= max.x && y >= min.y && y <= max.y;
                let p = CellPos::new(x, y);
                let Some(c) = self.get(p) else { continue };
                if c.is_air() {
                    continue;
                }
                let ph = mats.phys(c.material);
                let burning = ph.kind == Kind::Fire || c.flags & flags::BURNING != 0;
                if burning {
                    // Flames set it alight; that does the harm, not their heat.
                    e.ignites = true;
                } else {
                    let t = self.climate().ambient(x, y) + c.heat as i32;
                    e.heat = e.heat.max((t - HARMFUL_HEAT).max(0) as f32 * HEAT_DAMAGE);
                    e.heat = e.heat.max((HARMFUL_COLD - t).max(0) as f32 * HEAT_DAMAGE);
                    e.cold = e.cold.max(((CHILLING_COLD - t) as f32 / 60.0).clamp(0.0, 1.0));
                }
                e.corrosion = e.corrosion.max(ph.corrosive as f32);
                let coats = mats.def(c.material).coats.is_some();
                if inside && ph.kind == Kind::Liquid {
                    liquid += 1;
                    if coats {
                        match inside_counts(&mut soaking, c.material) {
                            Some(n) => *n += 1,
                            None => soaking.push((c.material, 1)),
                        }
                    }
                } else if coats && touched.is_none() {
                    touched = Some(c.material);
                }
            }
        }
        let area = ((max.x - min.x + 1) * (max.y - min.y + 1)).max(1);
        e.submerged = liquid as f32 / area as f32;
        e.coat = soaking.iter().max_by_key(|(_, n)| *n).map(|(m, _)| *m).or(touched);
        e
    }
}

fn inside_counts(v: &mut [(MaterialId, u32)], m: MaterialId) -> Option<&mut u32> {
    v.iter_mut().find(|(k, _)| *k == m).map(|(_, n)| n)
}
