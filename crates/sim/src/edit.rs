//! Every gameplay change to cells is a `WorldEdit`. Applying edits at tick
//! boundaries is what keeps co-op, replays and undo possible.

use crate::coords::CellPos;
use crate::material::MaterialId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldEdit {
    /// Fill a disc with a material. With `overwrite: false` only air is filled.
    Paint { center: CellPos, radius: i32, material: MaterialId, overwrite: bool },
    /// Remove every cell in a disc whose hardness is at most `max_hardness`.
    Dig { center: CellPos, radius: i32, max_hardness: u8 },
    /// One tick of a pickaxe: every solid or powder cell in the disc takes
    /// `power` damage (less towards the rim) and breaks once its damage
    /// reaches its hardness. Cells harder than `max_hardness` are untouched;
    /// hardness 255 never breaks.
    Mine { center: CellPos, radius: i32, power: u8, max_hardness: u8 },
    /// Destroy what `power` can break (falling off towards the edge), shatter
    /// a rim into rubble (`crumbles_into`), ignite flammables, fill the crater
    /// with fire and smoke.
    Explode { center: CellPos, radius: i32, power: u8 },
    /// Set flammable cells alight; put flames in empty cells.
    Ignite { center: CellPos, radius: i32 },
    /// Add (or with a negative amount, remove) heat in °C to every non-air
    /// cell, less towards the rim. Melting, boiling and freezing follow.
    Heat { center: CellPos, radius: i32, amount: i16 },
    /// Lightning down the column at `x` from the clouds (or from `from_y` if
    /// there are none): it strikes the first thing in its way, sets it alight
    /// and scorches it (SPEC §3.13).
    Lightning { x: i32, from_y: i32 },
    /// Force a storm (`storm`) or clear sky over x ± `radius`; it fades back to
    /// the natural weather over minutes. Nothing without weather.
    Weather { x: i32, radius: i32, storm: bool },
}

/// What an edit changed. `removed` is sorted by material id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditReport {
    pub removed: Vec<(MaterialId, u32)>,
    pub placed: u32,
}

impl EditReport {
    pub(crate) fn add_removed(&mut self, id: MaterialId) {
        match self.removed.binary_search_by_key(&id, |&(m, _)| m) {
            Ok(i) => self.removed[i].1 += 1,
            Err(i) => self.removed.insert(i, (id, 1)),
        }
    }
}

/// Cells of a filled disc, in a fixed order.
pub fn disc(center: CellPos, radius: i32) -> impl Iterator<Item = CellPos> {
    let r = radius.max(0);
    let r2 = r * r + r; // slightly rounder than r*r for small radii
    (-r..=r).flat_map(move |dy| (-r..=r).filter(move |dx| dx * dx + dy * dy <= r2).map(move |dx| center.offset(dx, dy)))
}
