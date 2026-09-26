//! Chests (DESIGN §5): furniture, not cells. A chest is a body that falls
//! when what it stands on goes, gets thrown by blasts, and breaks when mined,
//! blown up or burned, spilling what's in it. Its contents live here, under a
//! key that stays put when the chest moves: a chest the world made is keyed
//! by where it was made, and rolls its loot from `loot.ron` the first time
//! it's opened (from the world seed and that place, so the same chest always
//! holds the same things); a placed one gets a fresh key and starts empty.
//!
//! Right-click a chest within reach to open it; Shift-click moves a stack
//! between the chest and the pack; R takes everything. Anything else that
//! holds things (a body: `corpses.rs`) is a `Container` kept here too, and
//! opens the same way.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use platypus_physics::{Body, Locomotion};
use platypus_sim::rng::{Rng, hash};
use platypus_sim::{CellPos, World};
use platypus_worldgen::CHEST_SIZE;
use serde::Deserialize;

use super::items::{Inventory, Items, Stack};
use super::{DevTools, spawn_drop};
use crate::actors::Kinematics;
use crate::actors::player::LocalPlayer;
use crate::camera::CursorWorld;
use crate::data::{data_path, load_ron};
use crate::fx::Explosion;
use crate::props::Thrown;
use crate::world::SimWorld;

/// Slots in a chest.
pub const SLOTS: usize = 20;
/// How far from the player a chest can be opened (cells).
const REACH: f32 = 28.0;
/// Hit points: two hits of a copper pickaxe, a bomb beside it, a few
/// seconds in a fire.
const TOUGHNESS: f32 = 60.0;
/// Chests draw behind creatures, in front of the world.
const Z: f32 = 6.0;

/// The picture, one character a cell (top row first).
const ART: [&str; 10] = [
    "..oooooooo..",
    ".oLLLLLLLLo.",
    "oLllllllllLo",
    "oGGGGGGGGGGo",
    "oLlllGYGlllo",
    "olllLGYGlllo",
    "oLllllllllLo",
    "oddddddddddo",
    "oGGGGGGGGGGo",
    "oooooooooooo",
];

fn art_color(c: char) -> [u8; 4] {
    match c {
        'o' => [30, 18, 10, 255],
        'L' => [168, 118, 66, 255],
        'l' => [134, 90, 48, 255],
        'd' => [104, 68, 36, 255],
        'G' => [214, 176, 72, 255],
        'Y' => [252, 230, 130, 255],
        _ => [0, 0, 0, 0],
    }
}

#[derive(Clone, Debug, Deserialize)]
struct LootFile {
    tables: Vec<LootTable>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LootTable {
    name: String,
    /// A chest's: cells below sea level (large world) from which this table
    /// applies. None: a creature's (by name, its `loot`).
    #[serde(default)]
    deeper_than: Option<i32>,
    rolls: (u32, u32),
    entries: Vec<LootEntry>,
}

#[derive(Clone, Debug, Deserialize)]
struct LootEntry {
    item: String,
    weight: u32,
    count: (u32, u32),
}

/// Something in the world that holds things (a chest, a body): the key to
/// what's in it, and what it's called in its window.
#[derive(Component)]
pub struct Container {
    pub key: u64,
    pub name: String,
}

/// A chest in the world: the key to what's in it, and how much more it
/// takes to break it.
#[derive(Component)]
pub struct Chest {
    pub key: u64,
    hp: f32,
}

/// What's in a chest (`None`: never opened, not rolled yet), and where it
/// was made (its loot's seed and depth).
struct Stash {
    origin: CellPos,
    contents: Option<Inventory>,
}

/// Every chest's contents, by key.
#[derive(Resource, Default)]
pub struct Chests {
    known: HashMap<u64, Stash>,
    /// The chest (or body) whose panel is open, and what it's called.
    pub open: Option<u64>,
    pub open_name: String,
    tables: Vec<LootTable>,
    /// Loot depths are written for the large world; others scale them.
    depth_scale: f32,
    placed: u64,
    art: Handle<Image>,
    /// Gear found in chests is rolled (`gear::roll`), with the luck of
    /// whoever opens it first.
    rarities: Vec<crate::gear::roll::Rarity>,
    luck: f32,
}

pub struct ChestsPlugin;

impl Plugin for ChestsPlugin {
    fn build(&self, app: &mut App) {
        let file: LootFile = load_ron(&data_path("loot.ron")).unwrap_or_else(|e| panic!("{e}"));
        let mut tables = file.tables;
        tables.sort_by_key(|t| t.deeper_than.unwrap_or(i32::MIN));
        app.insert_resource(Chests { tables, depth_scale: 1.0, ..default() })
            .add_systems(Startup, setup)
            .add_systems(Update, (open_chest, take_all, close_far))
            .add_systems(FixedUpdate, batter.in_set(crate::world::TickSet::Bodies).after(crate::props::fly));
    }
}

fn setup(sim: Res<SimWorld>, rules: Res<crate::gear::GearRules>, mut chests: ResMut<Chests>, mut images: ResMut<Assets<Image>>) {
    chests.rarities = rules.rarities.clone();
    let (lo, hi) = sim.generator.bounds();
    chests.depth_scale = ((hi.y - lo.y + 1) * platypus_sim::CHUNK) as f32 / 16_384.0;
    let (w, h) = CHEST_SIZE;
    let data: Vec<u8> = ART.iter().flat_map(|row| row.chars().flat_map(art_color)).collect();
    assert_eq!(data.len(), (w * h * 4) as usize, "the chest's picture is {w} × {h}");
    chests.art = images.add(Image::new(
        Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ));
}

fn size() -> Vec2 {
    Vec2::new(CHEST_SIZE.0 as f32, CHEST_SIZE.1 as f32)
}

impl Chests {
    /// A chest the world made at `feet` (once: `Spawned` remembers).
    pub fn spawn_found(&mut self, commands: &mut Commands, feet: CellPos) {
        let key = hash(&[0xC4E57, feet.x as u64, feet.y as u64]) & !(1 << 63);
        self.known.entry(key).or_insert(Stash { origin: feet, contents: None });
        self.spawn(commands, key, Vec2::new(feet.x as f32, feet.y as f32));
    }

    /// A chest the player just placed: empty, with a key of its own.
    pub fn spawn_placed(&mut self, commands: &mut Commands, feet: Vec2) {
        self.placed += 1;
        let key = (1 << 63) | self.placed;
        self.known.insert(key, Stash { origin: CellPos::from_world(feet.x, feet.y), contents: Some(Inventory::new(SLOTS)) });
        self.spawn(commands, key, feet);
    }

    fn spawn(&self, commands: &mut Commands, key: u64, feet: Vec2) {
        let centre = feet + Vec2::new(0.0, size().y / 2.0);
        commands.spawn((
            Name::new("Chest"),
            Chest { key, hp: TOUGHNESS },
            Container { key, name: "Chest".into() },
            Thrown { bounce: 0.0 },
            Kinematics { body: Body::new(centre, size()), loco: Locomotion::default(), prev_pos: centre },
            Sprite::from_image(self.art.clone()),
            Transform::from_translation(centre.extend(Z)),
        ));
    }

    /// A chest's contents, rolling them the first time.
    pub fn contents(&mut self, key: u64, world: &World, items: &Items) -> &mut Inventory {
        let stash = self.known.entry(key).or_insert(Stash { origin: CellPos::new(0, 0), contents: None });
        let depth = ((world.climate().sea_level - stash.origin.y) as f32 / self.depth_scale) as i32;
        let table = self.tables.iter().rev().find(|t| t.deeper_than.is_some_and(|d| depth >= d));
        let origin = stash.origin;
        let found = Found { rarities: &self.rarities, level: item_level(depth), luck: self.luck };
        stash.contents.get_or_insert_with(|| {
            let mut inv = Inventory::new(SLOTS);
            if let Some(t) = table {
                roll(&mut inv, t, items, &found, &mut Rng::seeded(&[world.seed(), 0xC4E57, origin.x as u64, origin.y as u64]));
            }
            inv
        })
    }

    /// Keep what's in something new (a body): its key.
    pub fn stash(&mut self, at: Vec2, contents: Inventory) -> u64 {
        self.placed += 1;
        let key = (1 << 62) | self.placed;
        self.known.insert(key, Stash { origin: CellPos::from_world(at.x, at.y), contents: Some(contents) });
        key
    }

    /// Forget what was kept under a key (the thing holding it is gone).
    pub fn forget(&mut self, key: u64) {
        self.known.remove(&key);
        if self.open == Some(key) {
            self.open = None;
        }
    }

    /// Is what's kept under a key empty (or unknown)?
    pub fn is_empty(&self, key: u64) -> bool {
        self.known.get(&key).is_none_or(|s| s.contents.as_ref().is_some_and(|i| i.slots.iter().all(|s| s.is_none())))
    }

    /// The item level of what's found at `at` (the depth, as a chest's).
    pub fn level_at(&self, world: &World, at: Vec2) -> u8 {
        item_level(((world.climate().sea_level as f32 - at.y) / self.depth_scale) as i32)
    }

    /// Roll a creature's loot table (by name) into `inv`.
    pub fn roll_table(&self, name: &str, inv: &mut Inventory, items: &Items, level: u8, luck: f32, rng: &mut Rng) {
        match self.tables.iter().find(|t| t.name == name) {
            Some(t) => roll(inv, t, items, &Found { rarities: &self.rarities, level, luck }, rng),
            None => warn!("loot.ron: no table `{name}`"),
        }
    }

    /// Break a chest: what's in it spills out at `at` (a never-opened one is
    /// rolled first, so breaking it isn't a way to lose it), and, mined, the
    /// chest itself.
    pub fn smash(&mut self, commands: &mut Commands, world: &World, items: &Items, key: u64, at: Vec2, whole: bool) {
        let inside: Vec<Stack> = self.contents(key, world, items).slots.iter().flatten().copied().collect();
        self.known.remove(&key);
        if self.open == Some(key) {
            self.open = None;
        }
        for (k, stack) in inside.into_iter().enumerate() {
            spawn_drop(commands, items, at + Vec2::new(k as f32 * 0.7 - 3.0, 0.0), stack);
        }
        if whole && let Some(chest) = items.id("chest") {
            spawn_drop(commands, items, at, Stack::new(chest, 1));
        }
    }
}

/// The item level of what's found this deep (cells below sea level, large
/// world): 1 at the surface, a level every 150 cells, at most 60.
pub fn item_level(depth: i32) -> u8 {
    (1 + depth.max(0) / 150).min(60) as u8
}

/// How gear found is rolled: the rarities, the item level where it's
/// found, the finder's luck.
pub struct Found<'a> {
    pub rarities: &'a [crate::gear::roll::Rarity],
    pub level: u8,
    pub luck: f32,
}

fn roll(inv: &mut Inventory, t: &LootTable, items: &Items, found: &Found, rng: &mut Rng) {
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
        if e.item == "nothing" {
            continue;
        }
        let Some(item) = items.id(&e.item) else {
            warn!("loot.ron: no item `{}`", e.item);
            continue;
        };
        let n = e.count.0 + rng.next_u32() % (e.count.1 - e.count.0 + 1);
        if items.def(item).gear.is_some() {
            // (Each piece rolled on its own.)
            for _ in 0..n {
                let roll = crate::gear::roll::roll(found.rarities, items.def(item), found.level, found.luck, rng);
                inv.add(items, Stack { roll, ..Stack::new(item, 1) });
            }
            continue;
        }
        inv.add(items, Stack::new(item, n * items.unit(item)));
    }
}

/// The chest under `at` (a little forgiving), if any: its entity, key and
/// centre.
pub fn chest_at<'a>(at: Vec2, mut chests: impl Iterator<Item = (Entity, &'a Chest, &'a Kinematics)>) -> Option<(Entity, u64, Vec2)> {
    chests.find(|(_, _, k)| ((k.body.pos - at).abs() - k.body.half).max_element() <= 2.0).map(|(e, c, k)| (e, c.key, k.body.pos))
}

impl Chest {
    /// A pickaxe's hit; the last one breaks it (spilling it, and the chest).
    pub fn hit(&mut self, power: f32) -> bool {
        self.hp -= power;
        self.hp <= 0.0
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
    player: Query<(&Kinematics, Option<&crate::gear::Stats>), With<LocalPlayer>>,
    found: Query<(&Container, &Kinematics)>,
) {
    if dev.0 || !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let (Some(at), Some(items), Ok((k, stats))) = (cursor.0, items, player.single()) else { return };
    // (A little forgiving: bodies are thin.)
    let Some((c, ck)) = found.iter().filter(|(_, ck)| ((ck.body.pos - at).abs() - ck.body.half).max_element() <= 3.0).min_by(|a, b| a.1.body.pos.distance(at).total_cmp(&b.1.body.pos.distance(at))) else { return };
    let (key, pos) = (c.key, ck.body.pos);
    if pos.distance(k.body.pos) > REACH {
        return;
    }
    chests.open_name = c.name.clone();
    chests.luck = stats.map_or(0.0, |s| s.get(crate::gear::Stat::Luck));
    chests.contents(key, &sim.world, &items);
    chests.open = Some(key);
    open.0 = true;
}

/// R: everything in the open chest into the pack.
fn take_all(keys: Res<ButtonInput<KeyCode>>, items: Option<Res<Items>>, sim: Res<SimWorld>, mut chests: ResMut<Chests>, mut inv: Query<&mut Inventory, With<LocalPlayer>>) {
    let (Some(key), Some(items), Ok(mut inv)) = (chests.open, items, inv.single_mut()) else { return };
    if !keys.just_pressed(KeyCode::KeyR) {
        return;
    }
    let chest = chests.contents(key, &sim.world, &items);
    for slot in chest.slots.iter_mut() {
        if let Some(s) = *slot {
            let left = inv.add(&items, s);
            *slot = (left > 0).then_some(Stack { count: left, ..s });
        }
    }
}

/// Walking away (or the chest going) closes it.
fn close_far(mut chests: ResMut<Chests>, player: Query<&Kinematics, With<LocalPlayer>>, found: Query<(&Container, &Kinematics)>) {
    let (Some(key), Ok(k)) = (chests.open, player.single()) else { return };
    let near = found.iter().find(|(c, _)| c.key == key).is_some_and(|(_, ck)| ck.body.pos.distance(k.body.pos) <= REACH * 1.5);
    if !near {
        chests.open = None;
    }
}

/// Blasts throw chests and knock pieces off them; fire, lava and acid eat
/// them. One that's had enough breaks, spilling what's in it.
fn batter(
    mut commands: Commands,
    mut blasts: MessageReader<Explosion>,
    mut tick: Local<u32>,
    sim: Res<SimWorld>,
    items: Option<Res<Items>>,
    mut chests: ResMut<Chests>,
    mut q: Query<(Entity, &mut Chest, &mut Kinematics)>,
) {
    let blasts: Vec<Explosion> = blasts.read().copied().collect();
    *tick += 1;
    let Some(items) = items else { return };
    // Fire and the like, checked a few times a second.
    const EVERY: u32 = 10;
    let dt = EVERY as f32 / crate::world::TICK_HZ as f32;
    for (entity, mut chest, mut k) in &mut q {
        for b in &blasts {
            let reach = b.radius * 1.6 + 6.0;
            let d = k.body.pos.distance(b.at);
            if d < reach {
                let f = 1.0 - d / reach;
                chest.hp -= 120.0 * f;
                let away = (k.body.pos - b.at).normalize_or(Vec2::Y);
                k.body.vel += (away + Vec2::new(0.0, 0.6)) * 260.0 * f;
            }
        }
        if (*tick).is_multiple_of(EVERY) {
            let (lo, hi) = k.body.cells_at(k.body.pos);
            let e = sim.world.exposure(CellPos::new(lo.x, lo.y), CellPos::new(hi.x, hi.y));
            let burning = if e.ignites { 15.0 } else { 0.0 };
            chest.hp -= (burning + e.heat + e.corrosion) * dt;
        }
        if chest.hp <= 0.0 {
            chests.smash(&mut commands, &sim.world, &items, chest.key, k.body.pos, false);
            commands.entity(entity).despawn();
        }
    }
}

/// Where a chest would go for a cursor: on the highest ground under it
/// (searched a little way down; natural ground isn't flat, so three
/// quarters of its columns resting on something will do), with room for it.
/// Returns its feet.
pub fn place_spot(world: &World, cursor: Vec2) -> Option<Vec2> {
    let (w, h) = CHEST_SIZE;
    let mats = world.materials();
    let solid = |p: CellPos| world.get(p).is_some_and(|c| matches!(mats.phys(c.material).kind, platypus_sim::Kind::Static | platypus_sim::Kind::Powder));
    let x0 = cursor.x.floor() as i32 - w / 2;
    let y = cursor.y.floor() as i32 + 4;
    // Each column's ground: the first solid cell going down.
    let tops: Vec<Option<i32>> = (0..w).map(|dx| (0..20).map(|d| y - d).find(|&yy| solid(CellPos::new(x0 + dx, yy)))).collect();
    let floor = tops.iter().flatten().max()? + 1;
    let resting = tops.iter().filter(|t| **t == Some(floor - 1)).count();
    // (Tall grass and smoke don't count: a chest goes over them.)
    let room = |c: platypus_sim::Cell| c.is_air() || matches!(mats.phys(c.material).kind, platypus_sim::Kind::Plant | platypus_sim::Kind::Gas | platypus_sim::Kind::Fire);
    let empty = (0..h).all(|dy| (0..w).all(|dx| world.get(CellPos::new(x0 + dx, floor + dy)).is_some_and(room)));
    (resting * 4 >= w as usize * 3 && empty).then_some(Vec2::new((x0 + w / 2) as f32, floor as f32))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (Items, Vec<LootTable>) {
        let loot: LootFile = crate::data::parse_ron(include_str!("../../../../assets/data/loot.ron")).unwrap();
        (crate::hands::items::test_items(), loot.tables)
    }

    #[test]
    fn every_loot_item_exists_and_a_roll_is_the_same_every_time() {
        let (items, tables) = setup();
        for t in &tables {
            for e in &t.entries {
                assert!(e.item == "nothing" || items.id(&e.item).is_some(), "loot.ron `{}`: no item `{}`", t.name, e.item);
            }
            let gear: crate::gear::GearFile = crate::data::parse_ron(include_str!("../../../../assets/data/gear.ron")).unwrap();
            let found = Found { rarities: &gear.rarities, level: 10, luck: 0.0 };
            let fill = |seed: u64| {
                let mut inv = Inventory::new(SLOTS);
                roll(&mut inv, t, &items, &found, &mut Rng::seeded(&[seed]));
                inv.slots
            };
            assert_eq!(fill(7), fill(7), "same seed, same chest");
            assert!(fill(7).iter().flatten().count() >= t.rolls.0 as usize / 2, "`{}` puts something in", t.name);
        }
    }

    #[test]
    fn the_picture_is_the_chests_size() {
        let (w, h) = CHEST_SIZE;
        assert_eq!(ART.len(), h as usize);
        assert!(ART.iter().all(|r| r.chars().count() == w as usize));
    }
}
