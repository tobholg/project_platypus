//! Hands (DESIGN §4, §5): mining and building in blocks with a smart cursor,
//! items, an inventory with a hotbar, items on the ground. Play mode; F1
//! switches to the dev tools (`tools.rs`) and back.
//!
//! 1–0 pick a hotbar slot · X the next hotbar · LMB use it · hold Ctrl: the
//! right tool for the target (auto tool) · Alt: smart cursor on/off · C: a
//! pickaxe's mode (a block at a time, or a round bite: `area_at`) · Esc or
//! I: inventory · RMB: open a chest or a body (R takes all) · ` (or F1): dev tools.

pub mod chests;
pub mod corpses;
pub mod icons;
pub mod items;
pub mod target;
pub(crate) mod ui;

use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use platypus_physics::{Body, Locomotion};
use platypus_sim::{BLOCK, CellPos, Kind, World, WorldEdit, block_cells};

use crate::actors::player::LocalPlayer;
use crate::actors::{Creature, Kinematics};
use crate::camera::CursorWorld;
use crate::data::{data_path, load_ron};
use crate::light::{LightSettings, plant_torch};
use crate::props::{Thrown, spawn_bomb, spawn_glowstick};
use crate::tools::ToolsConfig;
use crate::world::{SimWorld, TICK_HZ, TickSet};
use items::{BARS, BLOCK_CELLS, HOTBAR, Inventory, Items, ItemsFile, Stack, Throwable, Use};

pub use ui::InventoryOpen;

pub struct HandsPlugin;

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// Slots in a player's pack: `BARS` hotbars, then five rows of pack.
pub const PACK: usize = HOTBAR * (BARS + PACK_ROWS);
pub const PACK_ROWS: usize = 5;
/// The arm keeps pointing this long after a cast (seconds).
const AIM_HOLD: f32 = 0.4;
/// Blocks placed per second while the button is held.
const PLACE_RATE: f32 = 8.0;
/// A vessel reaches this far (cells), scoops within this many cells of the
/// cursor, this many a tick, and pours this many a tick.
const VESSEL_REACH: f32 = 40.0;
const VESSEL_SCOOP: i32 = 3;
const VESSEL_RATE: u32 = 6;
const VESSEL_POUR: u32 = 1;
/// Items on the ground drift to a player within this many cells...
const MAGNET: f32 = 48.0;
/// ... and are picked up within this many.
const GRAB: f32 = 6.0;

/// Dev tools on (F1): the old toolbelt instead of the hands.
#[derive(Resource, Default)]
pub struct DevTools(pub bool);

pub fn dev_tools(dev: Res<DevTools>) -> bool {
    dev.0
}

fn play(dev: Res<DevTools>) -> bool {
    !dev.0
}

/// The local player's hands: the hotbar slot in use, when it can swing again.
#[derive(Resource)]
pub struct Hand {
    /// The slot in the hotbar in use (0..HOTBAR), and which hotbar.
    pub slot: usize,
    pub bar: usize,
    /// Smart cursor (Alt): dig a tunnel the body fits toward the cursor,
    /// rather than the block under it.
    pub smart: bool,
    /// A pickaxe's area mode (C): a round bite out of the rock nearest you,
    /// not a block.
    pub area: bool,
    cooldown: f32,
    /// Glow sticks thrown (they alternate colours).
    thrown: u32,
}

impl Default for Hand {
    fn default() -> Self {
        Hand { slot: 0, bar: 0, smart: true, area: false, cooldown: 0.0, thrown: 0 }
    }
}

impl Hand {
    /// The inventory slot in use.
    pub fn active(&self) -> usize {
        self.bar * HOTBAR + self.slot
    }

    /// The inventory slots of the hotbar in use.
    pub fn bar_slots(&self) -> std::ops::Range<usize> {
        self.bar * HOTBAR..(self.bar + 1) * HOTBAR
    }
}

/// Mouse and keys, sampled per frame, consumed per tick.
#[derive(Resource, Default)]
struct HandInput {
    primary: bool,
    /// The right button held (force: pull).
    secondary: bool,
    clicked: bool,
    auto: bool,
    cursor: Option<Vec2>,
}

/// Something lying on the ground.
#[derive(Component)]
pub struct Dropped {
    pub stack: Stack,
    pub(crate) age: f32,
}

impl Plugin for HandsPlugin {
    fn build(&self, app: &mut App) {
        let mut file: ItemsFile = load_ron(&data_path("items.ron")).unwrap_or_else(|e| panic!("{e}"));
        // (Armour and trinkets: gear.ron.)
        file.items.extend(crate::gear::load().items);
        app.insert_resource(PendingItems(Some(file)))
            .init_resource::<DevTools>()
            .init_resource::<Hand>()
            .init_resource::<HandInput>()
            .add_systems(Startup, build_items)
            .add_systems(PreUpdate, sample_input.after(crate::camera::track_cursor).after(bevy::ui::UiSystems::Focus))
            .add_systems(Update, (toggle_dev, select, wield, give_start, outline.run_if(play), mark, icons::make_icons, icons::reload_icons))
            .add_systems(FixedUpdate, use_hands.run_if(play).in_set(TickSet::Intent))
            .add_systems(FixedUpdate, collect.after(crate::props::fly).in_set(TickSet::Bodies))
            .add_plugins((ui::UiPlugin, chests::ChestsPlugin, corpses::CorpsesPlugin));
    }
}

/// Items need the materials table (every solid has a block item).
#[derive(Resource)]
struct PendingItems(Option<ItemsFile>);

fn build_items(mut commands: Commands, mut pending: ResMut<PendingItems>, sim: Res<SimWorld>) {
    let file = pending.0.take().expect("built once");
    let items = Items::new(file, sim.materials()).unwrap_or_else(|e| panic!("items.ron: {e}"));
    commands.insert_resource(items);
}

type NewPlayer = (With<LocalPlayer>, Without<Inventory>);

pub(crate) fn give_start(mut commands: Commands, items: Option<Res<Items>>, mut new: Query<(Entity, &mut crate::gear::Equipment), NewPlayer>) {
    let Some(items) = items else { return };
    for (e, mut eq) in &mut new {
        let mut inv = Inventory::new(PACK);
        for &(item, n) in &items.start {
            inv.add(&items, Stack::new(item, n * items.unit(item)));
        }
        // What it starts wearing.
        for &item in &items.wear {
            let s = Stack::new(item, 1);
            if let Some(i) = eq.slot_for(&items, &s) {
                eq.worn[i] = Some(s);
            }
        }
        commands.entity(e).insert(inv);
    }
}

fn toggle_dev(keys: Res<ButtonInput<KeyCode>>, mut actions: MessageReader<crate::dev::DevAction>, mut dev: ResMut<DevTools>, mut hand: ResMut<Hand>) {
    if actions.read().any(|a| *a == crate::dev::DevAction::Hands) {
        dev.0 = false;
    }
    if keys.just_pressed(KeyCode::AltLeft) && !dev.0 {
        hand.smart = !hand.smart;
    }
    // (F1 needs fn on a Mac keyboard. The key left of 1 is Backquote on a US
    // layout and IntlBackslash on an ISO Mac, like a Norwegian one's §,
    // whose < key by Z then reads as Backquote.)
    if keys.any_just_pressed([KeyCode::F1, KeyCode::Backquote, KeyCode::IntlBackslash]) {
        dev.0 = !dev.0;
        info!("{}", if dev.0 { "dev tools (key left of 1: hands)" } else { "hands (key left of 1: dev tools)" });
    }
}

fn sample_input(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Res<CursorWorld>,
    open: Res<InventoryOpen>,
    ui: Res<crate::dev::PointerOverUi>,
    mut input: ResMut<HandInput>,
) {
    // With the inventory open (or the pointer on a panel) the mouse is for the UI.
    let free = !open.0 && !ui.0;
    input.primary = free && mouse.pressed(MouseButton::Left);
    input.secondary = free && mouse.pressed(MouseButton::Right);
    input.clicked |= free && mouse.just_pressed(MouseButton::Left);
    input.auto = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    input.cursor = cursor.0;
}

const SLOT_KEYS: [KeyCode; HOTBAR] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
    KeyCode::Digit0,
];

/// Number keys pick a hotbar slot; the wheel steps through them (a notch a
/// slot; a trackpad's scroll is counted in lines' worth of pixels).
fn select(keys: Res<ButtonInput<KeyCode>>, scroll: Res<AccumulatedMouseScroll>, dev: Res<DevTools>, open: Res<InventoryOpen>, over_ui: Res<crate::dev::PointerOverUi>, mut hand: ResMut<Hand>, mut wheel: Local<f32>) {
    if dev.0 {
        return;
    }
    for (i, key) in SLOT_KEYS.iter().enumerate() {
        if keys.just_pressed(*key) {
            hand.slot = i;
        }
    }
    if keys.just_pressed(KeyCode::KeyX) {
        hand.bar = (hand.bar + 1) % BARS;
    }
    if keys.just_pressed(KeyCode::KeyC) {
        hand.area = !hand.area;
    }
    const PIXELS_A_NOTCH: f32 = 40.0;
    *wheel += match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / PIXELS_A_NOTCH,
    };
    // (Over a panel the wheel scrolls it, not the hotbar.)
    if open.0 || over_ui.0 {
        *wheel = 0.0;
    }
    while wheel.abs() >= 1.0 {
        // Up is the slot before, as in Terraria.
        let step = if *wheel > 0.0 { HOTBAR - 1 } else { 1 };
        hand.slot = (hand.slot + step) % HOTBAR;
        *wheel -= wheel.signum();
    }
}

/// Where swings (and spells) come from: a little above the body's centre.
pub fn hand_at(k: &Kinematics) -> Vec2 {
    k.body.pos + Vec2::new(0.0, k.body.half.y * 0.4)
}

/// Does a block hold anything this tool can take?
pub fn minable(world: &World, block: CellPos, back: bool, tier: u8) -> bool {
    let mats = world.materials();
    block_cells(block).any(|p| {
        let Some(front) = world.get(p) else { return false };
        let c = if back {
            if !front.is_air() {
                return false;
            }
            world.get_bg(p).unwrap_or(front)
        } else {
            front
        };
        let ph = mats.phys(c.material);
        // A pickaxe goes through tall grass; an axe takes leaves too.
        let kinds = if back { matches!(ph.kind, Kind::Static | Kind::Powder | Kind::Plant) } else { matches!(ph.kind, Kind::Static | Kind::Powder) };
        !c.is_air() && kinds && ph.hardness <= tier && ph.hardness < u8::MAX
    })
}

/// Can a block be built into: nothing solid in it, no creature in the way.
fn free(world: &World, block: CellPos, bodies: &[Body]) -> bool {
    let mats = world.materials();
    let solid = block_cells(block).any(|p| world.get(p).is_none_or(|c| matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder)));
    let (lo, hi) = (Vec2::new((block.x * BLOCK) as f32, (block.y * BLOCK) as f32), Vec2::new(((block.x + 1) * BLOCK) as f32, ((block.y + 1) * BLOCK) as f32));
    let blocked = bodies.iter().any(|b| b.pos.x + b.half.x > lo.x && b.pos.x - b.half.x < hi.x && b.pos.y + b.half.y > lo.y && b.pos.y - b.half.y < hi.y);
    !solid && !blocked
}

/// Is there something for a block to rest on: a solid next to it, or a wall
/// behind it.
fn supported(world: &World, block: CellPos) -> bool {
    let mats = world.materials();
    let solid = |b: CellPos| block_cells(b).any(|p| world.get(p).is_some_and(|c| matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder)));
    let wall = block_cells(block).any(|p| world.get_bg(p).is_some_and(|c| !c.is_air()));
    wall || [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| solid(CellPos::new(block.x + dx, block.y + dy)))
}

/// The block a mining tool would hit: with the smart cursor a tunnel the body
/// fits (a pickaxe) or the line toward the cursor (an axe, at trees);
/// without it the block under the cursor.
fn mine_at(world: &World, body: &Body, hand: Vec2, cursor: Vec2, smart: bool, (back, tier, reach): (bool, u8, f32)) -> Option<CellPos> {
    let reach = reach * BLOCK as f32;
    let can = |b: CellPos| minable(world, b, back, tier);
    match (smart, back) {
        (true, false) => target::tunnel_target(body.pos - body.half, body.pos + body.half, hand, cursor, reach, can),
        (true, true) => target::mine_target(hand, cursor, reach, can),
        (false, _) => target::cursor_target(hand, cursor, reach, can),
    }
}

/// A pickaxe's area mode keeps this share of its power (it takes many
/// cells a swing).
const AREA_POWER: f32 = 0.75;

/// A pickaxe's swing in area mode: its disc (centre, radius) and the cells
/// it takes. The disc sits where the line from the hand toward the cursor
/// first meets rock (a pick can't aim past it), else at the cursor (within
/// reach; with the smart cursor the line goes on to full reach past it).
/// What it takes is what it can get at from the hand: no more solid cells
/// between than the disc's radius (`World::within_reach`).
fn area_at(world: &World, hand: Vec2, cursor: Vec2, smart: bool, tier: u8, reach: f32, radius: f32) -> Option<(CellPos, i32, Vec<CellPos>)> {
    let reach = reach * BLOCK as f32;
    let to = cursor - hand;
    let dir = to.normalize_or(Vec2::X);
    let far = if smart { reach } else { to.length().min(reach) };
    let mats = world.materials();
    let solid = |p: Vec2| world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder));
    let steps = (far * 2.0).ceil() as i32;
    let hit = (1..=steps).map(|i| hand + dir * (i as f32 * 0.5).min(far)).find(|&p| solid(p));
    let at = hit.unwrap_or(hand + dir * far);
    let (centre, radius) = (CellPos::from_world(at.x, at.y), radius.round() as i32);
    let cells = world.within_reach(centre, radius, CellPos::from_world(hand.x, hand.y), radius, tier);
    (!cells.is_empty()).then_some((centre, radius, cells))
}

/// With auto tool, the slot of the best tool for what's at the cursor.
fn auto_slot(world: &World, items: &Items, inv: &Inventory, bar: std::ops::Range<usize>, body: &Body, hand: Vec2, cursor: Vec2) -> Option<usize> {
    let first = bar.start;
    let tools: Vec<(usize, bool, u8, u8, f32)> = inv.slots[bar]
        .iter()
        .enumerate()
        .map(|(i, s)| (first + i, s))
        .filter_map(|(i, s)| match items.def(s.as_ref()?.item).use_ {
            Use::Mine { back, power, tier, reach, .. } => Some((i, back, tier, power, reach)),
            _ => None,
        })
        .collect();
    // The strongest tool of each kind that can reach something.
    let best = |want_back: bool| {
        tools
            .iter()
            .filter(|t| t.1 == want_back)
            .filter(|t| mine_at(world, body, hand, cursor, true, (want_back, t.2, t.4)).is_some())
            .max_by_key(|t| (t.2, t.3))
            .map(|t| t.0)
    };
    // What's under the cursor decides: something in the playfield wants a
    // pickaxe, a tree or wall with open air in front an axe.
    let at = CellPos::from_world(cursor.x, cursor.y);
    let front_solid = world.get(at).is_some_and(|c| matches!(world.materials().phys(c.material).kind, Kind::Static | Kind::Powder));
    if front_solid { best(false).or_else(|| best(true)) } else { best(true).or_else(|| best(false)) }
}

/// The player holds the weapon in its hand's slot (none in dev mode), and
/// what it holds counts toward its stats (held gear).
fn wield(
    items: Option<Res<Items>>,
    weapons: Option<Res<crate::combat::Weapons>>,
    hand: Res<Hand>,
    dev: Res<DevTools>,
    mut player: Query<(&Inventory, &mut crate::combat::Wielding, &mut crate::gear::Equipment), With<LocalPlayer>>,
) {
    let (Some(items), Some(weapons), Ok((inv, mut w, mut eq))) = (items, weapons, player.single_mut()) else { return };
    let held = inv.slots.get(hand.active()).copied().flatten().filter(|_| !dev.0);
    let in_hand = held.filter(|s| items.def(s.item).gear.is_some());
    if eq.held != in_hand {
        eq.held = in_hand;
    }
    // A weapon by its weapon id; anything else by its own, if it's held
    // (tools, wands, torches: `held` in weapons.ron).
    let want = held
        .map(|s| {
            let def = items.def(s.item);
            match &def.use_ {
                Use::Melee(id) | Use::Bow(id) => id.clone(),
                _ => def.id.clone(),
            }
        })
        .filter(|id| weapons.knows(id));
    if w.0 != want {
        w.0 = want;
    }
}

type User<'a> = (Entity, &'a Kinematics, &'a mut Inventory, Option<&'a crate::actors::animation::HandPos>);
type StationHit<'a> = (Entity, &'a mut crate::craft::Station, &'a Kinematics);

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn use_hands(
    mut commands: Commands,
    mut input: ResMut<HandInput>,
    items: Option<Res<Items>>,
    tools: Res<ToolsConfig>,
    lights: Res<LightSettings>,
    torch_art: Option<Res<crate::light::TorchArt>>,
    mut hand: ResMut<Hand>,
    mut sim: ResMut<SimWorld>,
    mut chests: ResMut<chests::Chests>,
    mut player: Query<User, With<LocalPlayer>>,
    creatures: Query<&Kinematics, With<Creature>>,
    mut found: Query<(Entity, &mut chests::Chest, &Kinematics), Without<LocalPlayer>>,
    (crafting, mut stations): (Res<crate::craft::Crafting>, Query<StationHit, (Without<LocalPlayer>, Without<chests::Chest>)>),
    book: Res<crate::magic::Spellbook>,
    (mut casts, mut swings, mut draws, mut sounds, mut drinks, mut dug): (MessageWriter<crate::magic::CastRequest>, MessageWriter<crate::combat::MeleeRequest>, MessageWriter<crate::archery::DrawBow>, MessageWriter<crate::sound::PlaySound>, MessageWriter<crate::potion::Drink>, MessageWriter<crate::gold::Dug>),
) {
    let clicked = std::mem::take(&mut input.clicked);
    hand.cooldown = (hand.cooldown - DT).max(0.0);
    let (Some(items), Some(cursor)) = (items, input.cursor) else { return };
    let Ok((me, k, mut inv, hand_pos)) = player.single_mut() else { return };
    let from = hand_at(k);
    let slot = if input.auto { auto_slot(&sim.world, &items, &inv, hand.bar_slots(), &k.body, from, cursor).unwrap_or(hand.active()) } else { hand.active() };
    let Some(stack) = inv.slots[slot] else { return };
    match items.def(stack.item).use_.clone() {
        // (A pickaxe or axe swings while it's used: `combat`, by its id.)
        Use::Mine { back, power, tier, speed, reach, area } if input.primary && hand.cooldown == 0.0 => {
            swings.write(crate::combat::MeleeRequest { attacker: me, at: cursor });
            // A chest at the cursor, within reach, takes the hit (the last
            // one breaks it, spilling what's in it, and the chest).
            if !back
                && let Some((e, key, pos)) = chests::chest_at(cursor, found.iter())
                && pos.distance(from) <= reach * BLOCK as f32
            {
                hand.cooldown = 1.0 / speed.max(0.1);
                if let Ok((_, mut chest, _)) = found.get_mut(e)
                    && chest.hit(power as f32)
                {
                    chests.smash(&mut commands, &sim.world, &items, key, pos, true);
                    commands.entity(e).despawn();
                }
                return;
            }
            // So does a crafting station: the last blow knocks it back into
            // its item.
            if !back
                && let Some((e, kind, pos)) = crate::craft::station_at(cursor, stations.iter())
                && pos.distance(from) <= reach * BLOCK as f32
            {
                hand.cooldown = 1.0 / speed.max(0.1);
                if let Ok((_, mut station, _)) = stations.get_mut(e)
                    && station.hit(power as f32)
                    && let Some(item) = crafting.stations.get(kind).and_then(|d| items.id(&d.id))
                {
                    spawn_drop(&mut commands, &items, pos, Stack::new(item, 1));
                    commands.entity(e).despawn();
                }
                return;
            }
            // Area mode: a round bite out of the rock nearest you.
            if !back && hand.area && area > 0.0 {
                let Some((centre, radius, _)) = area_at(&sim.world, from, cursor, hand.smart, tier, reach, area) else { return };
                let at = Vec2::new(centre.x as f32 + 0.5, centre.y as f32 + 0.5);
                let what = crate::sound::hooks::underfoot(&sim.world, centre);
                let power = ((power as f32 * AREA_POWER).round() as u8).max(1);
                let report = sim.world.apply_edit(&WorldEdit::MineReach { center: centre, radius, from: CellPos::from_world(from.x, from.y), bite: radius, power, max_hardness: tier });
                let hit = match what {
                    Some("stone") => "mine_stone",
                    Some("sand") | Some("snow") => "mine_sand",
                    _ => "mine_dirt",
                };
                sounds.write(crate::sound::PlaySound::at(hit, at));
                if !report.removed.is_empty() {
                    sounds.write(crate::sound::PlaySound::at("break", at).volume(0.8));
                }
                hand.cooldown = 1.0 / speed.max(0.1);
                for &(material, n) in &report.removed {
                    if let Some(item) = items.block(material) {
                        spawn_drop(&mut commands, &items, at, Stack::new(item, n));
                    } else if sim.world.materials().def(material).counted {
                        dug.write(crate::gold::Dug(n));
                    }
                }
                return;
            }
            let Some(block) = mine_at(&sim.world, &k.body, from, cursor, hand.smart, (back, tier, reach)) else { return };
            // (What it sounds like: what's there, before the blow.)
            let centre = Vec2::new((block.x as f32 + 0.5) * BLOCK as f32, (block.y as f32 + 0.5) * BLOCK as f32);
            let what = if back { Some("wood") } else { crate::sound::hooks::underfoot(&sim.world, CellPos::from_world(centre.x, centre.y)) };
            let report = sim.world.apply_edit(&WorldEdit::MineBlock { block, power, max_hardness: tier, back });
            let hit = match what {
                Some("stone") => "mine_stone",
                Some("sand") | Some("snow") => "mine_sand",
                Some("wood") => "mine_wood",
                _ => "mine_dirt",
            };
            sounds.write(crate::sound::PlaySound::at(hit, centre));
            if !report.removed.is_empty() {
                sounds.write(crate::sound::PlaySound::at("break", centre).volume(0.8));
            }
            debug!("hit {block:?} removed {:?}", report.removed);
            hand.cooldown = 1.0 / speed.max(0.1);
            let centre = Vec2::new((block.x as f32 + 0.5) * BLOCK as f32, (block.y as f32 + 0.5) * BLOCK as f32);
            for &(material, n) in &report.removed {
                if let Some(item) = items.block(material) {
                    spawn_drop(&mut commands, &items, centre, Stack::new(item, n));
                } else if sim.world.materials().def(material).counted {
                    dug.write(crate::gold::Dug(n));
                }
            }
        }
        Use::Vessel { holds, hot, acidproof } if input.primary || input.secondary => {
            let mats = sim.world.materials().clone();
            let mut st = stack;
            if input.secondary {
                let furnace = crate::craft::station_at(cursor, stations.iter())
                    .filter(|&(_, kind, pos)| crafting.stations.get(kind).is_some_and(|d| d.id == "furnace") && pos.distance(from) <= VESSEL_REACH);
                if furnace.is_some() {
                    // At a furnace: what the pack holds that melts, into it.
                    if hot && hand.cooldown == 0.0 && melt_into(&mut inv, &mut st, &items, &mats, holds) {
                        sounds.write(crate::sound::PlaySound::at("pour", cursor));
                        hand.cooldown = 0.4;
                    }
                } else if cursor.distance(from) <= VESSEL_REACH {
                    // Scoop the liquid at the cursor (one kind at a time).
                    let c = CellPos::from_world(cursor.x, cursor.y);
                    let mut taken = 0;
                    'scoop: for dy in -VESSEL_SCOOP..=VESSEL_SCOOP {
                        for dx in -VESSEL_SCOOP..=VESSEL_SCOOP {
                            let (have, n) = st.fill.map_or((None, 0), |(m, n)| (Some(m), n));
                            if n >= holds || taken >= VESSEL_RATE {
                                break 'scoop;
                            }
                            let p = c.offset(dx, dy);
                            let Some(cell) = sim.world.get(p) else { continue };
                            let ph = mats.phys(cell.material);
                            let fits = ph.kind == Kind::Liquid && (hot || !ph.hot) && (acidproof || ph.corrosive == 0) && have.is_none_or(|m| m == cell.material);
                            if fits && sim.world.pluck(p).is_some() {
                                st.fill = Some((cell.material, n + 1));
                                taken += 1;
                            }
                        }
                    }
                    if taken > 0 {
                        sounds.write(crate::sound::PlaySound::at("drip", cursor).volume(0.5));
                    }
                }
            } else if let Some((m, n)) = st.fill {
                // Pour: a stream from the hand that lands at the cursor (thrown
                // so, under gravity: a flight of `ticks`).
                let start = from + (cursor - from).normalize_or(Vec2::X) * 3.0;
                let to = cursor - start;
                let ticks = (to.length() / 1.5).clamp(6.0, 40.0);
                let g = platypus_sim::particles::GRAVITY;
                let aim = Vec2::new(to.x / ticks, to.y / ticks + 0.5 * g * ticks);
                let k = n.min(VESSEL_POUR);
                let mut rng = platypus_sim::rng::Rng::seeded(&[sim.world.tick(), n as u64, 0x7057]);
                for _ in 0..k {
                    let cell = mats.spawn(m, &mut rng);
                    let jiggle = (rng.next_u32() as f32 / u32::MAX as f32 - 0.5) * 0.06;
                    let v = aim * (1.0 + jiggle);
                    sim.world.emit(platypus_sim::Particle::new([start.x, start.y], [v.x, v.y], cell, 200, platypus_sim::Landing::Settle));
                }
                st.fill = (n > k).then_some((m, n - k));
            }
            inv.slots[slot] = Some(st);
        }
        Use::Block(material) if input.primary && hand.cooldown == 0.0 && stack.count >= BLOCK_CELLS => {
            let bodies: Vec<Body> = creatures.iter().map(|k| k.body).collect();
            let world = &sim.world;
            let Some(block) = target::place_target(from, cursor, 6.0 * BLOCK as f32, |b| free(world, b, &bodies), |b| supported(world, b)) else { return };
            let report = sim.world.apply_edit(&WorldEdit::PlaceBlock { block, material, back: false });
            if report.placed > 0 {
                sounds.write(crate::sound::PlaySound::at("place", Vec2::new((block.x as f32 + 0.5) * BLOCK as f32, (block.y as f32 + 0.5) * BLOCK as f32)));
            }
            inv.take(slot, report.placed);
            hand.cooldown = 1.0 / PLACE_RATE;
        }
        Use::Throw(what) if clicked => {
            commands.entity(me).insert(crate::actors::animation::Aiming { at: cursor, left: AIM_HOLD });
            let dir = (cursor - from).normalize_or(Vec2::X);
            let speed = tools.bomb.throw_speed * ((cursor - from).length() / 120.0).clamp(0.35, 1.0);
            let vel = dir * speed + k.body.vel * 0.5;
            match what {
                Throwable::Bomb => spawn_bomb(&mut commands, from, vel, tools.bomb.clone()),
                Throwable::Glowstick => {
                    hand.thrown += 1;
                    let s = lights.glowstick.strength;
                    let color = if hand.thrown.is_multiple_of(2) { [0.25 * s, 1.0 * s, 0.45 * s] } else { [0.2 * s, 0.55 * s, 1.1 * s] };
                    spawn_glowstick(&mut commands, from, vel, color, lights.glowstick.secs, lights.glowstick.haze);
                }
            }
            inv.take(slot, 1);
        }
        // A focus casts its spell: the left button its first, the right
        // its second (or the first again, "alt"). The spell keeps its own
        // time (`magic::request`). The arm points at the cursor while
        // casting, and the spell leaves from its hand.
        Use::Focus { tier, element, ref spells } if input.primary || input.secondary => {
            let held = crate::magic::spells::focus_spells(&book.spells, tier, element, spells, &stack.roll);
            let pick = if input.primary { held.first() } else { held.get(1).or(held.first()) };
            let Some(&spell) = pick else { return };
            commands.entity(me).insert(crate::actors::animation::Aiming { at: cursor, left: AIM_HOLD });
            let from = hand_pos.and_then(|h| h.at).unwrap_or(from);
            casts.write(crate::magic::CastRequest { caster: me, spell, from, toward: cursor, alt: !input.primary });
        }
        Use::Melee(_) if input.primary => {
            swings.write(crate::combat::MeleeRequest { attacker: me, at: cursor });
        }
        // (Drawn while held, loosed on letting go: `archery::nock`.)
        Use::Bow(_) if input.primary && items.id("arrow").is_some_and(|a| inv.count(a) > 0) => {
            draws.write(crate::archery::DrawBow { archer: me, at: cursor });
        }
        // (Drunk, healed, the sickness after: `potion::drink`.)
        Use::Potion { .. } if clicked => {
            drinks.write(crate::potion::Drink { who: me, slot });
        }
        Use::Chest if clicked => {
            let Some(feet) = chests::place_spot(&sim.world, cursor) else { return };
            if feet.distance(from) > 6.0 * BLOCK as f32 {
                return;
            }
            chests.spawn_placed(&mut commands, feet);
            inv.take(slot, 1);
        }
        Use::Station(ref id) if clicked => {
            let Some(kind) = crafting.station(id) else { return warn!("crafting.ron: no station `{id}`") };
            let Some(feet) = chests::place_spot_sized(&sim.world, cursor, crafting.stations[kind].size()) else { return };
            if feet.distance(from) > 6.0 * BLOCK as f32 {
                return;
            }
            crafting.spawn(&mut commands, kind, feet);
            inv.take(slot, 1);
        }
        Use::Torch if clicked => {
            let bodies: Vec<Body> = Vec::new();
            let world = &sim.world;
            let Some(block) = target::place_target(from, cursor, 6.0 * BLOCK as f32, |b| free(world, b, &bodies), |b| supported(world, b)) else { return };
            let Some(art) = torch_art.as_deref() else { return };
            let at = Vec2::new((block.x as f32 + 0.5) * BLOCK as f32, block.y as f32 * BLOCK as f32);
            commands.entity(me).insert(crate::actors::animation::Aiming { at, left: AIM_HOLD });
            plant_torch(&mut commands, at, &lights, art);
            inv.take(slot, 1);
        }
        _ => {}
    }
}

/// A furnace melts what the pack holds that melts (a metal's bars, its ore:
/// blocks whose material melts into a hot liquid) into the vessel `st`, up to
/// what it `holds`, one liquid at a time: a bar gives its 16 cells, ore half
/// (the rest is slag). Whether anything went in.
fn melt_into(inv: &mut Inventory, st: &mut Stack, items: &Items, mats: &platypus_sim::MaterialTable, holds: u32) -> bool {
    let (have, mut n) = st.fill.map_or((None, 0), |(m, n)| (Some(m), n));
    let mut into = have;
    for slot in inv.slots.iter_mut() {
        let Some(s) = slot.as_mut() else { continue };
        let Use::Block(m) = items.def(s.item).use_ else { continue };
        let molten = mats.phys(m).above_into;
        if molten == platypus_sim::MaterialId::AIR || mats.phys(molten).kind != Kind::Liquid || !mats.phys(molten).hot || into.is_some_and(|i| i != molten) {
            continue;
        }
        // (Cells of it per cell of metal: a bar's all metal, ore half.)
        let per = if mats.def(m).item.is_some() { 1 } else { 2 };
        let take = ((holds - n) * per).min(s.count);
        if take < per {
            continue;
        }
        let take = take - take % per;
        s.count -= take;
        n += take / per;
        into = Some(molten);
        if s.count == 0 {
            *slot = None;
        }
        if n >= holds {
            break;
        }
    }
    let changed = into.is_some() && (have.is_none() || n > st.fill.map_or(0, |f| f.1));
    if let Some(m) = into {
        st.fill = Some((m, n));
    }
    changed
}

/// Put a stack on the ground at `at`, popping out a little.
pub fn spawn_drop(commands: &mut Commands, items: &Items, at: Vec2, stack: Stack) {
    let h = platypus_sim::rng::hash(&[at.x.to_bits() as u64, at.y.to_bits() as u64, stack.item.0 as u64]);
    let jitter = (h % 1000) as f32 / 1000.0 - 0.5;
    let mut body = Body::new(at, Vec2::splat(1.5));
    body.vel = Vec2::new(jitter * 60.0, 70.0);
    let (r, g, b) = items.def(stack.item).color;
    commands.spawn((
        Name::new("Dropped"),
        Dropped { stack, age: 0.0 },
        Thrown { bounce: 0.2 },
        Kinematics { body, loco: Locomotion::default(), prev_pos: at },
        Sprite::from_color(Color::srgb_u8(r, g, b), Vec2::splat(3.0)),
        Transform::from_translation(at.extend(12.5)),
    ));
}

/// Items on the ground drift to a nearby player with room, and are picked up.
fn collect(
    mut commands: Commands,
    items: Option<Res<Items>>,
    mut drops: Query<(Entity, &mut Dropped, &mut Kinematics), Without<LocalPlayer>>,
    mut players: Query<(&Kinematics, &mut Inventory), With<LocalPlayer>>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    let Some(items) = items else { return };
    for (e, mut d, mut k) in &mut drops {
        d.age += DT;
        if d.age < 0.4 {
            continue;
        }
        for (pk, mut inv) in &mut players {
            let to = pk.body.pos - k.body.pos;
            let dist = to.length();
            if dist > MAGNET || !inv.has_room(&items, &d.stack) {
                continue;
            }
            if dist < GRAB {
                sounds.write(crate::sound::PlaySound::here("pickup"));
                let left = inv.add(&items, d.stack);
                if left == 0 {
                    commands.entity(e).despawn();
                } else {
                    d.stack.count = left;
                }
                break;
            }
            // Pulled in, through whatever is in the way.
            let speed = 60.0 + (MAGNET - dist) * 6.0;
            k.body.vel = Vec2::ZERO;
            k.body.pos += to / dist * speed * DT;
            break;
        }
    }
}

/// What a mining tool would take, lit (`mark`).
#[derive(Component)]
struct Marked;

/// Over the dark (the light overlay is at 15–15.5), under creatures' eyes.
const Z_MARK: f32 = 16.1;

/// The cells the mining tool in hand would take next, lit as Terraria's
/// smart cursor lights them: a see-through warm yellow over them (a
/// block, or area mode's bite), a little stronger at their edge, breathing
/// gently; over the dark, so it shows in it.
#[allow(clippy::too_many_arguments)]
fn mark(
    mut commands: Commands,
    time: Res<Time>,
    input: Res<HandInput>,
    items: Option<Res<Items>>,
    hand: Res<Hand>,
    dev: Res<DevTools>,
    sim: Res<SimWorld>,
    player: Query<(&Kinematics, &Inventory), With<LocalPlayer>>,
    mut images: ResMut<Assets<Image>>,
    mut marked: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<Marked>>,
    mut last: Local<Vec<CellPos>>,
) {
    let cells = (|| {
        let (items, cursor, (k, inv)) = (items.as_ref()?, input.cursor?, player.single().ok()?);
        if dev.0 {
            return None;
        }
        let from = hand_at(k);
        let world = &sim.world;
        let slot = if input.auto { auto_slot(world, items, inv, hand.bar_slots(), &k.body, from, cursor).unwrap_or(hand.active()) } else { hand.active() };
        let &Use::Mine { back, tier, reach, area, .. } = &items.def(inv.slots[slot]?.item).use_ else { return None };
        if !back && hand.area && area > 0.0 {
            return area_at(world, from, cursor, hand.smart, tier, reach, area).map(|(_, _, cells)| cells);
        }
        mine_at(world, &k.body, from, cursor, hand.smart, (back, tier, reach)).map(|b| block_cells(b).collect())
    })()
    .unwrap_or_default();
    let Ok((mut sprite, mut tf, mut vis)) = marked.single_mut() else {
        commands.spawn((Name::new("Mining mark"), Marked, Sprite::default(), Transform::from_xyz(0.0, 0.0, Z_MARK), Visibility::Hidden));
        return;
    };
    if cells.is_empty() {
        *vis = Visibility::Hidden;
        last.clear();
        return;
    }
    *vis = Visibility::Visible;
    if *last != cells {
        let lo = cells.iter().fold(IVec2::MAX, |m, p| m.min(IVec2::new(p.x, p.y)));
        let hi = cells.iter().fold(IVec2::MIN, |m, p| m.max(IVec2::new(p.x, p.y)));
        let (w, h) = ((hi.x - lo.x + 1) as u32, (hi.y - lo.y + 1) as u32);
        let set: std::collections::HashSet<CellPos> = cells.iter().copied().collect();
        let mut px = vec![0u8; (w * h * 4) as usize];
        for p in &cells {
            let edge = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| !set.contains(&CellPos::new(p.x + dx, p.y + dy)));
            let i = (((hi.y - p.y) as u32 * w + (p.x - lo.x) as u32) * 4) as usize;
            px[i..i + 4].copy_from_slice(&[255, 214, 90, if edge { 150 } else { 88 }]);
        }
        let mut image = Image::new(
            bevy::render::render_resource::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            bevy::render::render_resource::TextureDimension::D2,
            px,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = bevy::image::ImageSampler::nearest();
        sprite.image = images.add(image);
        sprite.custom_size = Some(Vec2::new(w as f32, h as f32));
        tf.translation = Vec3::new(lo.x as f32 + w as f32 / 2.0, lo.y as f32 + h as f32 / 2.0, Z_MARK);
        *last = cells;
    }
    let breath = 0.85 + 0.15 * (time.elapsed_secs() * 3.5).sin();
    sprite.color = Color::srgba(1.0, 1.0, 1.0, breath);
}

/// The spot the hands would build on or set something down at, outlined in
/// cyan (what a mining tool would take is lit: `mark`).
#[allow(clippy::too_many_arguments)]
fn outline(
    input: Res<HandInput>,
    items: Option<Res<Items>>,
    hand: Res<Hand>,
    sim: Res<SimWorld>,
    crafting: Res<crate::craft::Crafting>,
    player: Query<(&Kinematics, &Inventory), With<LocalPlayer>>,
    creatures: Query<&Kinematics, With<Creature>>,
    mut gizmos: Gizmos,
) {
    let (Some(items), Some(cursor)) = (items, input.cursor) else { return };
    let Ok((k, inv)) = player.single() else { return };
    let from = hand_at(k);
    let slot = if input.auto { auto_slot(&sim.world, &items, inv, hand.bar_slots(), &k.body, from, cursor).unwrap_or(hand.active()) } else { hand.active() };
    let Some(stack) = inv.slots[slot] else { return };
    let world = &sim.world;
    let (block, color) = match &items.def(stack.item).use_ {
        Use::Station(id) => {
            if let Some(kind) = crafting.station(id) {
                let (w, h) = crafting.stations[kind].size();
                if let Some(feet) = chests::place_spot_sized(world, cursor, (w, h)) {
                    let size = Vec2::new(w as f32, h as f32);
                    gizmos.rect_2d(bevy::math::Isometry2d::from_translation(feet + Vec2::new(0.0, size.y / 2.0)), size, Color::srgba(0.4, 0.9, 1.0, 0.9));
                }
            }
            (None, Color::NONE)
        }
        Use::Chest => {
            if let Some(feet) = chests::place_spot(world, cursor) {
                let (w, h) = platypus_worldgen::CHEST_SIZE;
                let size = Vec2::new(w as f32, h as f32);
                gizmos.rect_2d(bevy::math::Isometry2d::from_translation(feet + Vec2::new(0.0, size.y / 2.0)), size, Color::srgba(0.4, 0.9, 1.0, 0.9));
            }
            (None, Color::NONE)
        }
        Use::Block(_) | Use::Torch => {
            let bodies: Vec<Body> = creatures.iter().map(|k| k.body).collect();
            (target::place_target(from, cursor, 6.0 * BLOCK as f32, |b| free(world, b, &bodies), |b| supported(world, b)), Color::srgba(0.4, 0.9, 1.0, 0.9))
        }
        _ => (None, Color::NONE),
    };
    if let Some(b) = block {
        let c = Vec2::new((b.x as f32 + 0.5) * BLOCK as f32, (b.y as f32 + 0.5) * BLOCK as f32);
        gizmos.rect_2d(bevy::math::Isometry2d::from_translation(c), Vec2::splat(BLOCK as f32), color);
    }
}
