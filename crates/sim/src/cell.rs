use bytemuck::{Pod, Zeroable};

use crate::material::MaterialId;

/// One cell of the world. 10 bytes; a 64×64 chunk is 40 KiB.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct Cell {
    pub material: MaterialId,
    /// Temperature in °C *relative to the local ambient* (`Climate`). 0 is
    /// "normal" and costs nothing; only cells off ambient take part in heat flow.
    pub heat: i16,
    /// Low byte of the tick this cell last moved on; a cell updates at most once per tick.
    pub clock: u8,
    /// Colour variation, fixed at creation, travels with the cell.
    pub shade: u8,
    /// Gases and fire: remaining life (see `MaterialDef::decay_every`).
    /// Solids and powders: mining damage taken so far (breaks at `hardness`).
    pub life: u8,
    pub flags: u8,
    /// Reserved for ballistic motion; unused by the phase-1 rules.
    pub vx: i8,
    /// Accumulated fall speed (see `rules::MAX_FALL`).
    pub vy: i8,
}

pub mod flags {
    /// A `Static` cell knocked loose (no longer attached to anything big):
    /// it falls and piles like a powder while keeping its own material.
    pub const LOOSE: u8 = 1 << 5;
    /// On fire: the cell keeps its material and place, glows, spreads fire,
    /// puts out flames and burns down over `MaterialDef::burn_time`.
    pub const BURNING: u8 = 1 << 6;

    /// Liquids remember which way they were flowing, which stops left/right jitter.
    pub const FLOW_LEFT: u8 = 1 << 0;
    /// Liquids: direction reversals since last falling (bits 1..=4). A liquid
    /// that has sloshed back `REST_LIMIT` times with nowhere to drop stops, so
    /// lakes sleep. Spreading in one direction is never limited.
    pub const REST_SHIFT: u8 = 1;
    pub const REST_MASK: u8 = 0b1111 << REST_SHIFT;
    pub const REST_LIMIT: u8 = 8;

    #[inline]
    pub const fn rest(f: u8) -> u8 {
        (f & REST_MASK) >> REST_SHIFT
    }

    #[inline]
    pub const fn with_rest(f: u8, n: u8) -> u8 {
        (f & !REST_MASK) | ((n << REST_SHIFT) & REST_MASK)
    }
}

impl Cell {
    pub const AIR: Cell = Cell { material: MaterialId::AIR, heat: 0, clock: 0, shade: 0, life: 0, flags: 0, vx: 0, vy: 0 };

    #[inline]
    pub const fn new(material: MaterialId, shade: u8) -> Self {
        Cell { material, heat: 0, clock: 0, shade, life: 0, flags: 0, vx: 0, vy: 0 }
    }

    #[inline]
    pub const fn is_air(&self) -> bool {
        self.material.0 == 0
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cell_is_ten_bytes() {
        assert_eq!(std::mem::size_of::<super::Cell>(), 10);
    }
}
