//! Chests (DESIGN §5): 8 × 8 cells of the `chest` material, bound to what's
//! in them. The world makes them (in cave pockets) and players place them;
//! the game keeps their contents, rolled from `loot.ron` the first time one is
//! opened (from the world seed and the chest's place, so the same chest always
//! holds the same things). Break a chest (mine it, blow it up, burn it) and
//! what was in it spills out, with the chest itself.
//!
//! Right-click a chest within reach to open it; Shift-click moves a stack
//! between the chest and the pack; R takes everything.

use std::collections::HashMap;

use bevy::prelude::*;
use platypus_sim::rng::Rng;
use platypus_sim::{CellPos, MaterialId, World, WorldEdit};
use serde::Deserialize;

use super::items::{Inventory, Items, Stack};
use super::{DevTools, spawn_drop};
use crate::actors::Kinematics;
use crate::actors::player::LocalPlayer;
use crate::camera::CursorWorld;
use crate::data::{data_path, load_ron};
use crate::world::SimWorld;

/// A chest's side, in cells.
pub const SIDE: i32 = 8;
/// Slots in a chest.
pub const SLOTS: usize = 20;
/// How far from the hand a chest can be opened (cells).
const REACH: f32 = 28.0;

#[derive(Clone, Debug, Deserialize)]
struct LootFile {
    tables: Vec<LootTable>,
}

#[derive(Clone, Debug, Deserialize)]
struct LootTable {
    #[allow(dead_code)]
    name: String,
    /// Cells below sea level (large world) from which this table applies.
    deeper_than: i32,
    rolls: (u32, u32),
    entries: Vec<LootEntry>,
}

#[derive(Clone, Debug, Deserialize)]
struct LootEntry {
    item: String,
    weight: u32,
    count: (u32, u32),
}

/// Known chests (by corner) and what's in them (`None`: not opened yet).
#[derive(Resource, Default)]
pub struct Chests {
    known: HashMap<CellPos, Option<Inventory>>,
    /// The chest whose panel is open.
    pub open: Option<CellPos>,
    tables: Vec<LootTable>,
    /// Loot depths are written for the large world; others scale them.
    depth_scale: f32,
}

pub struct ChestsPlugin;

impl Plugin for ChestsPlugin {
    fn build(&self, app: &mut App) {
        let file: LootFile = load_ron(&data_path("loot.ron")).unwrap_or_else(|e| panic!("{e}"));
        let mut tables = file.tables;
        tables.sort_by_key(|t| t.deeper_than);
        app.insert_resource(Chests { tables, depth_scale: 1.0, ..default() })
            .add_systems(Startup, scale_depths)
            .add_systems(Update, (open_chest, take_all, close_far))
            .add_systems(FixedUpdate, break_ruined.in_set(crate::world::TickSet::Bodies));
    }
}

fn scale_depths(sim: Res<SimWorld>, mut chests: ResMut<Chests>) {
    let (lo, hi) = sim.generator.bounds();
    chests.depth_scale = ((hi.y - lo.y + 1) * platypus_sim::CHUNK) as f32 / 16_384.0;
}

fn chest_material(world: &World) -> Option<MaterialId> {
    world.materials().id("chest")
}

fn is_chest(world: &World, p: CellPos) -> bool {
    chest_material(world).is_some_and(|m| world.get(p).is_some_and(|c| c.material == m))
}

fn contains(corner: CellPos, p: CellPos) -> bool {
    (corner.x..corner.x + SIDE).contains(&p.x) && (corner.y..corner.y + SIDE).contains(&p.y)
}

impl Chests {
    /// The chest a cell belongs to, if any (registering one the world made).
    pub fn find(&mut self, world: &World, p: CellPos) -> Option<CellPos> {
        if let Some(&c) = self.known.keys().find(|&&c| contains(c, p)) {
            return Some(c);
        }
        if !is_chest(world, p) {
            return None;
        }
        // A chest the world made: its corner is where its cells stop, left
        // and down.
        let mut x = p.x;
        while x > p.x - (SIDE - 1) && is_chest(world, CellPos::new(x - 1, p.y)) {
            x -= 1;
        }
        let mut y = p.y;
        while y > p.y - (SIDE - 1) && is_chest(world, CellPos::new(x, y - 1)) {
            y -= 1;
        }
        let corner = CellPos::new(x, y);
        self.known.insert(corner, None);
        Some(corner)
    }

    /// A chest's contents, rolling them the first time.
    pub fn contents(&mut self, corner: CellPos, world: &World, items: &Items) -> &mut Inventory {
        let depth = ((world.climate().sea_level - corner.y) as f32 / self.depth_scale) as i32;
        let table = self.tables.iter().rev().find(|t| depth >= t.deeper_than).cloned();
        let seed = world.seed();
        self.known.entry(corner).or_default().get_or_insert_with(|| {
            let mut inv = Inventory::new(SLOTS);
            if let Some(t) = table {
                roll(&mut inv, &t, items, &mut Rng::seeded(&[seed, 0xC4E57, corner.x as u64, corner.y as u64]));
            }
            inv
        })
    }

    /// A chest the player just placed (empty).
    pub fn placed(&mut self, corner: CellPos) {
        self.known.insert(corner, Some(Inventory::new(SLOTS)));
    }

    /// Break a chest: take its cells out, spill what's in it and the chest.
    pub fn smash(&mut self, commands: &mut Commands, world: &mut World, items: &Items, corner: CellPos) {
        let inside = match self.known.get(&corner) {
            Some(Some(inv)) => inv.slots.iter().flatten().copied().collect::<Vec<_>>(),
            // Never opened: roll it now, so breaking one isn't a way to lose it.
            _ => self.contents(corner, world, items).slots.iter().flatten().copied().collect(),
        };
        self.known.remove(&corner);
        if self.open == Some(corner) {
            self.open = None;
        }
        if let Some(m) = chest_material(world) {
            world.apply_edit(&WorldEdit::Remove { min: corner, max: corner.offset(SIDE - 1, SIDE - 1), material: m });
        }
        let at = Vec2::new(corner.x as f32 + SIDE as f32 / 2.0, corner.y as f32 + SIDE as f32 / 2.0);
        for (k, stack) in inside.into_iter().enumerate() {
            spawn_drop(commands, items, at + Vec2::new(k as f32 * 0.7 - 3.0, 0.0), stack);
        }
        if let Some(chest) = items.id("chest") {
            spawn_drop(commands, items, at, Stack { item: chest, count: 1 });
        }
    }

    /// Chests with a cell in `block`'s 4 × 4 cells.
    pub fn in_block(&mut self, world: &World, block: CellPos) -> Vec<CellPos> {
        let mut out = Vec::new();
        for p in platypus_sim::block_cells(block) {
            if is_chest(world, p)
                && let Some(c) = self.find(world, p)
                && !out.contains(&c)
            {
                out.push(c);
            }
        }
        out
    }
}

fn roll(inv: &mut Inventory, t: &LootTable, items: &Items, rng: &mut Rng) {
    let total: u32 = t.entries.iter().map(|e| e.weight).sum();
    if total == 0 {
        return;
    }
    let draws = t.rolls.0 + rng.next_u32() % (t.rolls.1 - t.rolls.0 + 1);
    for _ in 0..draws {
        let mut pick = rng.next_u32() % total;
        let Some(e) = t.entries.iter().find(|e| {
            if pick < e.weight {
                true
            } else {
                pick -= e.weight;
                false
            }
        }) else {
            continue;
        };
        let Some(item) = items.id(&e.item) else {
            warn!("loot.ron: no item `{}`", e.item);
            continue;
        };
        let n = e.count.0 + rng.next_u32() % (e.count.1 - e.count.0 + 1);
        inv.add(items, Stack { item, count: n * items.unit(item) });
    }
}

/// Right-click a chest within reach: open it (with the pack).
#[allow(clippy::too_many_arguments)]
fn open_chest(
    mouse: Res<ButtonInput<MouseButton>>,
    dev: Res<DevTools>,
    cursor: Res<CursorWorld>,
    items: Option<Res<Items>>,
    sim: Res<SimWorld>,
    mut chests: ResMut<Chests>,
    mut open: ResMut<super::InventoryOpen>,
    player: Query<&Kinematics, With<LocalPlayer>>,
) {
    if dev.0 || !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let (Some(at), Some(items), Ok(k)) = (cursor.0, items, player.single()) else { return };
    if at.distance(k.body.pos) > REACH {
        return;
    }
    // Forgiving: a click on the chest or just beside it.
    let c = CellPos::from_world(at.x, at.y);
    let Some(corner) = (-2..=2).flat_map(|dy| (-2..=2).map(move |dx| c.offset(dx, dy))).find_map(|p| chests.find(&sim.world, p)) else { return };
    chests.contents(corner, &sim.world, &items);
    chests.open = Some(corner);
    open.0 = true;
}

/// R: everything in the open chest into the pack.
fn take_all(keys: Res<ButtonInput<KeyCode>>, items: Option<Res<Items>>, sim: Res<SimWorld>, mut chests: ResMut<Chests>, mut inv: Query<&mut Inventory, With<LocalPlayer>>) {
    let (Some(corner), Some(items), Ok(mut inv)) = (chests.open, items, inv.single_mut()) else { return };
    if !keys.just_pressed(KeyCode::KeyR) {
        return;
    }
    let chest = chests.contents(corner, &sim.world, &items);
    for slot in chest.slots.iter_mut() {
        if let Some(s) = *slot {
            let left = inv.add(&items, s);
            *slot = (left > 0).then_some(Stack { item: s.item, count: left });
        }
    }
}

/// Walking away closes the chest.
fn close_far(mut chests: ResMut<Chests>, player: Query<&Kinematics, With<LocalPlayer>>) {
    let (Some(c), Ok(k)) = (chests.open, player.single()) else { return };
    let centre = Vec2::new(c.x as f32 + SIDE as f32 / 2.0, c.y as f32 + SIDE as f32 / 2.0);
    if centre.distance(k.body.pos) > REACH * 1.5 {
        chests.open = None;
    }
}

/// Chests blown up or burned away (most of their cells gone) break too.
fn break_ruined(mut commands: Commands, mut tick: Local<u32>, items: Option<Res<Items>>, mut sim: ResMut<SimWorld>, mut chests: ResMut<Chests>) {
    *tick += 1;
    let Some(items) = items else { return };
    if !(*tick).is_multiple_of(30) {
        return;
    }
    let world = &sim.world;
    let ruined: Vec<CellPos> = chests
        .known
        .keys()
        .copied()
        .filter(|&c| world.get(c).is_some()) // loaded
        .filter(|&c| (0..SIDE).flat_map(|dy| (0..SIDE).map(move |dx| c.offset(dx, dy))).filter(|&p| is_chest(world, p)).count() < (SIDE * SIDE / 2) as usize)
        .collect();
    for c in ruined {
        chests.smash(&mut commands, &mut sim.world, &items, c);
    }
}

/// Where a chest would go for a cursor: on the highest ground under it
/// (searched a little way down; natural ground isn't flat, so three of its
/// eight columns resting on something will do), its box empty. Returns the
/// corner.
pub fn place_spot(world: &World, cursor: Vec2) -> Option<CellPos> {
    let mats = world.materials();
    let solid = |p: CellPos| world.get(p).is_some_and(|c| matches!(mats.phys(c.material).kind, platypus_sim::Kind::Static | platypus_sim::Kind::Powder));
    let x0 = cursor.x.floor() as i32 - SIDE / 2;
    let y = cursor.y.floor() as i32 + 4;
    // Each column's ground: the first solid cell going down.
    let tops: Vec<Option<i32>> = (0..SIDE).map(|dx| (0..20).map(|d| y - d).find(|&yy| solid(CellPos::new(x0 + dx, yy)))).collect();
    let floor = tops.iter().flatten().max()? + 1;
    let resting = tops.iter().filter(|t| **t == Some(floor - 1)).count();
    let corner = CellPos::new(x0, floor);
    // (Tall grass and smoke don't count: a chest goes over them.)
    let room = |c: platypus_sim::Cell| c.is_air() || matches!(mats.phys(c.material).kind, platypus_sim::Kind::Plant | platypus_sim::Kind::Gas | platypus_sim::Kind::Fire);
    let empty = (0..SIDE).all(|dy| (0..SIDE).all(|dx| world.get(corner.offset(dx, dy)).is_some_and(room)));
    (resting >= 3 && empty).then_some(corner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hands::items::ItemsFile;
    use platypus_sim::MaterialTable;

    fn setup() -> (Items, Vec<LootTable>) {
        let mats = MaterialTable::from_ron(include_str!("../../../../assets/data/materials.ron")).unwrap();
        let file: ItemsFile = crate::data::parse_ron(include_str!("../../../../assets/data/items.ron")).unwrap();
        let loot: LootFile = crate::data::parse_ron(include_str!("../../../../assets/data/loot.ron")).unwrap();
        (Items::new(file, &mats).unwrap(), loot.tables)
    }

    #[test]
    fn every_loot_item_exists_and_a_roll_is_the_same_every_time() {
        let (items, tables) = setup();
        for t in &tables {
            for e in &t.entries {
                assert!(items.id(&e.item).is_some(), "loot.ron `{}`: no item `{}`", t.name, e.item);
            }
            let fill = |seed: u64| {
                let mut inv = Inventory::new(SLOTS);
                roll(&mut inv, t, &items, &mut Rng::seeded(&[seed]));
                inv.slots
            };
            assert_eq!(fill(7), fill(7), "same seed, same chest");
            assert!(fill(7).iter().flatten().count() >= t.rolls.0 as usize / 2, "`{}` puts something in", t.name);
        }
    }
}
