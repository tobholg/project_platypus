//! Items (DESIGN §5): what goes in inventories, lies on the ground and sits in
//! chests. Made things (tools, torches, bombs) are data in
//! `assets/data/items.ron`; every solid or powder material also has a block
//! item, made from the materials table, so anything you mine you can carry
//! and place again.

use std::collections::HashMap;

use bevy::prelude::*;
use platypus_sim::{BLOCK, Kind, MaterialId, MaterialTable};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(pub u16);

/// What using an item does.
#[derive(Clone, Debug, Deserialize)]
pub enum Use {
    /// Mines blocks: the playfield (a pickaxe) or, with `back`, the
    /// background (an axe: trees, walls). `power` damage a hit, `speed` hits a
    /// second, up to `tier` hardness, within `reach` blocks.
    Mine { back: bool, power: u8, tier: u8, speed: f32, reach: f32 },
    /// Throw one toward the cursor.
    Throw(Throwable),
    /// Plant one on a block face within reach: it lights the place up.
    Torch,
    /// A block of a material (made from the materials table, not written).
    #[serde(skip)]
    Block(MaterialId),
    /// Nothing (materials, loot).
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Throwable {
    Bomb,
    Glowstick,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ItemDef {
    pub id: String,
    pub name: String,
    /// Most in one slot (in whole items; blocks: whole blocks).
    #[serde(default = "one")]
    pub stack: u32,
    /// Icon colour until items have sprites.
    pub color: (u8, u8, u8),
    #[serde(rename = "use", default = "no_use")]
    pub use_: Use,
}

fn one() -> u32 {
    1
}

fn no_use() -> Use {
    Use::None
}

#[derive(Clone, Debug, Deserialize)]
pub struct ItemsFile {
    pub items: Vec<ItemDef>,
    /// What a new player carries: (item id, count).
    #[serde(default)]
    pub start: Vec<(String, u32)>,
}

/// Every item there is.
#[derive(Resource, Clone, Debug)]
pub struct Items {
    defs: Vec<ItemDef>,
    by_id: HashMap<String, ItemId>,
    blocks: HashMap<MaterialId, ItemId>,
    pub start: Vec<(ItemId, u32)>,
}

/// Blocks count in cells (so a half-mined block isn't lost): this many make a
/// whole block.
pub const BLOCK_CELLS: u32 = (BLOCK * BLOCK) as u32;

impl Items {
    pub fn new(file: ItemsFile, mats: &MaterialTable) -> Result<Items, String> {
        let mut defs = file.items;
        for (id, def) in mats.iter() {
            let ph = mats.phys(id);
            if matches!(def.kind, Kind::Static | Kind::Powder) && ph.hardness < u8::MAX {
                let (r, g, b) = def.colors[def.colors.len() / 2];
                let mut name = def.name.replace('_', " ");
                if let Some(first) = name.get_mut(0..1) {
                    first.make_ascii_uppercase();
                }
                defs.push(ItemDef { id: format!("block:{}", def.name), name, stack: 999, color: (r, g, b), use_: Use::Block(id) });
            }
        }
        let mut by_id = HashMap::new();
        let mut blocks = HashMap::new();
        for (i, d) in defs.iter().enumerate() {
            if by_id.insert(d.id.clone(), ItemId(i as u16)).is_some() {
                return Err(format!("item `{}` is defined twice", d.id));
            }
            if let Use::Block(m) = d.use_ {
                blocks.insert(m, ItemId(i as u16));
            }
        }
        let start = file
            .start
            .iter()
            .map(|(id, n)| by_id.get(id).map(|&i| (i, *n)).ok_or(format!("start: no item `{id}`")))
            .collect::<Result<_, _>>()?;
        Ok(Items { defs, by_id, blocks, start })
    }

    pub fn def(&self, id: ItemId) -> &ItemDef {
        &self.defs[id.0 as usize]
    }

    pub fn id(&self, name: &str) -> Option<ItemId> {
        self.by_id.get(name).copied()
    }

    /// The block item of a material, if it has one.
    pub fn block(&self, m: MaterialId) -> Option<ItemId> {
        self.blocks.get(&m).copied()
    }

    /// How many counted units make one whole item (blocks count in cells).
    pub fn unit(&self, id: ItemId) -> u32 {
        if matches!(self.def(id).use_, Use::Block(_)) { BLOCK_CELLS } else { 1 }
    }

    /// Most units in one slot.
    pub fn stack_units(&self, id: ItemId) -> u32 {
        self.def(id).stack * self.unit(id)
    }
}

/// Some of one item: `count` units (cells for blocks).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stack {
    pub item: ItemId,
    pub count: u32,
}

/// Slots of stacks: a creature's pack, a chest. The first `HOTBAR` of a
/// player's are the hotbar.
#[derive(Component, Clone, Debug)]
pub struct Inventory {
    pub slots: Vec<Option<Stack>>,
}

pub const HOTBAR: usize = 10;

impl Inventory {
    pub fn new(slots: usize) -> Inventory {
        Inventory { slots: vec![None; slots] }
    }

    /// Put in as much as fits (topping up stacks of the same item first);
    /// returns what didn't fit.
    pub fn add(&mut self, items: &Items, stack: Stack) -> u32 {
        let max = items.stack_units(stack.item);
        let mut left = stack.count;
        for s in self.slots.iter_mut().flatten().filter(|s| s.item == stack.item) {
            let n = left.min(max.saturating_sub(s.count));
            s.count += n;
            left -= n;
        }
        for slot in self.slots.iter_mut().filter(|s| s.is_none()) {
            if left == 0 {
                break;
            }
            let n = left.min(max);
            *slot = Some(Stack { item: stack.item, count: n });
            left -= n;
        }
        left
    }

    /// Would any of it fit?
    pub fn has_room(&self, items: &Items, item: ItemId) -> bool {
        let max = items.stack_units(item);
        self.slots.iter().any(|s| s.is_none_or(|s| s.item == item && s.count < max))
    }

    /// Take up to `n` units from a slot; returns how many.
    pub fn take(&mut self, slot: usize, n: u32) -> u32 {
        let Some(s) = &mut self.slots[slot] else { return 0 };
        let got = n.min(s.count);
        s.count -= got;
        if s.count == 0 {
            self.slots[slot] = None;
        }
        got
    }

    pub fn count(&self, item: ItemId) -> u32 {
        self.slots.iter().flatten().filter(|s| s.item == item).map(|s| s.count).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Items {
        let mats = MaterialTable::from_ron(include_str!("../../../../assets/data/materials.ron")).unwrap();
        let file: ItemsFile = crate::data::parse_ron(include_str!("../../../../assets/data/items.ron")).unwrap();
        Items::new(file, &mats).unwrap()
    }

    #[test]
    fn every_solid_has_a_block_item_counted_in_cells() {
        let it = items();
        let stone = it.id("block:stone").expect("stone blocks");
        assert_eq!(it.unit(stone), 16);
        assert!(it.id("block:bedrock").is_none(), "no unbreakable blocks");
        assert!(it.id("block:water").is_none(), "no liquid blocks");
        assert!(!it.start.is_empty(), "a new player carries something");
    }

    #[test]
    fn adding_tops_up_then_fills_empty_slots() {
        let it = items();
        let torch = it.id("torch").unwrap();
        let mut inv = Inventory::new(3);
        assert_eq!(inv.add(&it, Stack { item: torch, count: 150 }), 0);
        assert_eq!(inv.slots[0], Some(Stack { item: torch, count: 99 }));
        assert_eq!(inv.slots[1], Some(Stack { item: torch, count: 51 }));
        assert_eq!(inv.add(&it, Stack { item: torch, count: 200 }), 200 - 48 - 99, "tops up 48, one slot of 99, the rest is left");
        assert!(!inv.has_room(&it, torch));
        assert_eq!(inv.take(0, 10), 10);
        assert_eq!(inv.count(torch), 297 - 10);
    }
}
