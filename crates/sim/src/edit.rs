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
    /// One tick of a pickaxe (`back: false`, the playfield) or an axe
    /// (`back: true`, the background where the playfield is open: trees,
    /// walls): every cell in the disc takes `power` damage (less towards the
    /// rim) and breaks once its damage reaches its hardness. Cells harder
    /// than `max_hardness` are untouched; hardness 255 never breaks.
    Mine { center: CellPos, radius: i32, power: u8, max_hardness: u8, back: bool },
    /// One hit on a block (`BLOCK` × `BLOCK` cells, block coordinates): the
    /// block's minable cells (in the playfield, or with `back` the background
    /// where the playfield in front is open) take `power` damage, and when it
    /// reaches the hardest of them they all break at once. Cells harder than
    /// `max_hardness` stay (ore beyond a tool's tier).
    MineBlock { block: CellPos, power: u8, max_hardness: u8, back: bool },
    /// Fill the empty cells of a block (air, or tall grass, smoke, flames:
    /// what building pushes aside) with a material (its pattern, if it has
    /// one, gives the shades). `EditReport::placed` says how many.
    PlaceBlock { block: CellPos, material: MaterialId, back: bool },
    /// Fill the empty cells (as `PlaceBlock`) of a `w` × `h` box with a material, shaded by its pattern
    /// anchored at the box's corner (furniture: a chest's picture sits on it
    /// wherever it stands). `EditReport::placed` says how many cells.
    Stamp { corner: CellPos, w: i32, h: i32, material: MaterialId },
    /// Take every cell of one material out of a rectangle (inclusive):
    /// what's left of a broken chest, door or other furniture.
    Remove { min: CellPos, max: CellPos, material: MaterialId },
    /// Destroy what `power` can break (falling off towards the edge), shatter
    /// a rim into rubble (`crumbles_into`), ignite flammables, fill the crater
    /// with fire and smoke.
    Explode { center: CellPos, radius: i32, power: u8 },
    /// A sharp hit (a spark bolt): the solids in a disc no harder than
    /// `max_hardness` break and fly off as rubble (what they crumble into,
    /// or themselves), thrown away from `from`. Liquids, gas and the
    /// background are left alone.
    Shatter { center: CellPos, from: CellPos, radius: i32, max_hardness: u8 },
    /// Set flammable cells alight; put flames in empty cells.
    Ignite { center: CellPos, radius: i32 },
    /// Set flammable cells alight, without flames in the empty ones (a
    /// burning creature brushing past: its own flames would relight it).
    Scorch { center: CellPos, radius: i32 },
    /// A body moving through liquid (`min..=max`, moving by `vel`: 1/16
    /// cells per tick, integers so edits compare exactly and replay the same
    /// on every machine) trades places with it: the liquid it moves into goes
    /// where it just was. Fast, some splashes out.
    Displace { min: CellPos, max: CellPos, vel: [i16; 2] },
    /// Add (or with a negative amount, remove) heat in °C to every non-air
    /// cell, less towards the rim. Melting, boiling and freezing follow.
    Heat { center: CellPos, radius: i32, amount: i16 },
    /// Lightning from a wand: a smaller bolt from `from` toward `to`,
    /// jagged, stopped by the first solid thing in its way; what burns along
    /// it catches, and where it lands it bursts, heats and lights.
    Zap { from: CellPos, to: CellPos },
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

/// Blocks: what hands mine and build in (DESIGN D2), `BLOCK` × `BLOCK`
/// cells on a fixed grid. The world stays cells.
pub const BLOCK: i32 = 4;

/// The block a cell is in.
pub fn block_of(p: CellPos) -> CellPos {
    CellPos::new(p.x.div_euclid(BLOCK), p.y.div_euclid(BLOCK))
}

/// The cells of a block, in a fixed order.
pub fn block_cells(block: CellPos) -> impl Iterator<Item = CellPos> {
    let (x0, y0) = (block.x * BLOCK, block.y * BLOCK);
    (0..BLOCK).flat_map(move |dy| (0..BLOCK).map(move |dx| CellPos::new(x0 + dx, y0 + dy)))
}

/// Cells of a filled disc, in a fixed order.
pub fn disc(center: CellPos, radius: i32) -> impl Iterator<Item = CellPos> {
    let r = radius.max(0);
    let r2 = r * r + r; // slightly rounder than r*r for small radii
    (-r..=r).flat_map(move |dy| (-r..=r).filter(move |dx| dx * dx + dy * dy <= r2).map(move |dx| center.offset(dx, dy)))
}
