//! Hands (DESIGN §4, §5): mining and building in blocks with a smart cursor,
//! items, an inventory with a hotbar, items on the ground. Play mode; F1
//! switches to the dev tools (`tools.rs`) and back.
//!
//! 1–0 pick a hotbar slot · LMB use it · hold Ctrl: the right tool for the
//! target (auto tool) · Alt: smart cursor on/off · I: inventory · RMB: open a
//! chest (R takes all) · ` (or F1): dev tools.

pub mod chests;
pub mod items;
pub mod target;
mod ui;

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
use items::{BLOCK_CELLS, HOTBAR, Inventory, Items, ItemsFile, Stack, Throwable, Use};

pub use ui::InventoryOpen;

pub struct HandsPlugin;

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// Slots in a player's pack (the first `HOTBAR` are the hotbar).
pub const PACK: usize = 40;
/// Blocks placed per second while the button is held.
const PLACE_RATE: f32 = 8.0;
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
    pub slot: usize,
    /// Smart cursor (Alt): dig a tunnel the body fits toward the cursor,
    /// rather than the block under it.
    pub smart: bool,
    cooldown: f32,
    /// Glow sticks thrown (they alternate colours).
    thrown: u32,
}

impl Default for Hand {
    fn default() -> Self {
        Hand { slot: 0, smart: true, cooldown: 0.0, thrown: 0 }
    }
}

/// Mouse and keys, sampled per frame, consumed per tick.
#[derive(Resource, Default)]
struct HandInput {
    primary: bool,
    clicked: bool,
    auto: bool,
    cursor: Option<Vec2>,
}

/// Something lying on the ground.
#[derive(Component)]
pub struct Dropped {
    pub stack: Stack,
    age: f32,
}

impl Plugin for HandsPlugin {
    fn build(&self, app: &mut App) {
        let file: ItemsFile = load_ron(&data_path("items.ron")).unwrap_or_else(|e| panic!("{e}"));
        app.insert_resource(PendingItems(Some(file)))
            .init_resource::<DevTools>()
            .init_resource::<Hand>()
            .init_resource::<HandInput>()
            .add_systems(Startup, build_items)
            .add_systems(PreUpdate, sample_input.after(crate::camera::track_cursor))
            .add_systems(Update, (toggle_dev, select, give_start, outline.run_if(play)))
            .add_systems(FixedUpdate, use_hands.run_if(play).in_set(TickSet::Intent))
            .add_systems(FixedUpdate, collect.after(crate::props::fly).in_set(TickSet::Bodies))
            .add_plugins((ui::UiPlugin, chests::ChestsPlugin));
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

fn give_start(mut commands: Commands, items: Option<Res<Items>>, new: Query<Entity, (With<LocalPlayer>, Without<Inventory>)>) {
    let Some(items) = items else { return };
    for e in &new {
        let mut inv = Inventory::new(PACK);
        for &(item, n) in &items.start {
            inv.add(&items, Stack { item, count: n * items.unit(item) });
        }
        commands.entity(e).insert(inv);
    }
}

fn toggle_dev(keys: Res<ButtonInput<KeyCode>>, mut dev: ResMut<DevTools>, mut hand: ResMut<Hand>) {
    if keys.just_pressed(KeyCode::AltLeft) && !dev.0 {
        hand.smart = !hand.smart;
    }
    // (F1 needs fn on a Mac keyboard; the key left of 1 doesn't.)
    if keys.any_just_pressed([KeyCode::F1, KeyCode::Backquote, KeyCode::IntlBackslash]) {
        dev.0 = !dev.0;
        info!("{}", if dev.0 { "dev tools (F1: hands)" } else { "hands (F1: dev tools)" });
    }
}

fn sample_input(mouse: Res<ButtonInput<MouseButton>>, keys: Res<ButtonInput<KeyCode>>, cursor: Res<CursorWorld>, open: Res<InventoryOpen>, mut input: ResMut<HandInput>) {
    // With the inventory open the mouse is for the inventory.
    let free = !open.0;
    input.primary = free && mouse.pressed(MouseButton::Left);
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

fn select(keys: Res<ButtonInput<KeyCode>>, dev: Res<DevTools>, mut hand: ResMut<Hand>) {
    if dev.0 {
        return;
    }
    for (i, key) in SLOT_KEYS.iter().enumerate() {
        if keys.just_pressed(*key) {
            hand.slot = i;
        }
    }
}

/// Where swings come from: a little above the body's centre.
fn hand_at(k: &Kinematics) -> Vec2 {
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

/// With auto tool, the slot of the best tool for what's at the cursor.
fn auto_slot(world: &World, items: &Items, inv: &Inventory, body: &Body, hand: Vec2, cursor: Vec2) -> Option<usize> {
    let tools: Vec<(usize, bool, u8, u8, f32)> = inv.slots[..HOTBAR]
        .iter()
        .enumerate()
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

#[allow(clippy::too_many_arguments)]
fn use_hands(
    mut commands: Commands,
    mut input: ResMut<HandInput>,
    items: Option<Res<Items>>,
    tools: Res<ToolsConfig>,
    lights: Res<LightSettings>,
    mut hand: ResMut<Hand>,
    mut sim: ResMut<SimWorld>,
    mut chests: ResMut<chests::Chests>,
    mut player: Query<(&Kinematics, &mut Inventory), With<LocalPlayer>>,
    creatures: Query<&Kinematics, With<Creature>>,
) {
    let clicked = std::mem::take(&mut input.clicked);
    hand.cooldown = (hand.cooldown - DT).max(0.0);
    let (Some(items), Some(cursor)) = (items, input.cursor) else { return };
    let Ok((k, mut inv)) = player.single_mut() else { return };
    let from = hand_at(k);
    let slot = if input.auto { auto_slot(&sim.world, &items, &inv, &k.body, from, cursor).unwrap_or(hand.slot) } else { hand.slot };
    let Some(stack) = inv.slots[slot] else { return };
    match items.def(stack.item).use_.clone() {
        Use::Mine { back, power, tier, speed, reach } if input.primary && hand.cooldown == 0.0 => {
            let Some(block) = mine_at(&sim.world, &k.body, from, cursor, hand.smart, (back, tier, reach)) else { return };
            let struck = if back { Vec::new() } else { chests.in_block(&sim.world, block) };
            let report = sim.world.apply_edit(&WorldEdit::MineBlock { block, power, max_hardness: tier, back });
            // A chest hit through breaks, spilling what's in it.
            if report.removed.iter().any(|&(m, _)| Some(m) == sim.materials().id("chest")) {
                for corner in struck {
                    chests.smash(&mut commands, &mut sim.world, &items, corner);
                }
            }
            debug!("hit {block:?} removed {:?}", report.removed);
            hand.cooldown = 1.0 / speed.max(0.1);
            let centre = Vec2::new((block.x as f32 + 0.5) * BLOCK as f32, (block.y as f32 + 0.5) * BLOCK as f32);
            for &(material, n) in &report.removed {
                if let Some(item) = items.block(material) {
                    spawn_drop(&mut commands, &items, centre, Stack { item, count: n });
                }
            }
        }
        Use::Block(material) if input.primary && hand.cooldown == 0.0 && stack.count >= BLOCK_CELLS => {
            let bodies: Vec<Body> = creatures.iter().map(|k| k.body).collect();
            let world = &sim.world;
            let Some(block) = target::place_target(from, cursor, 6.0 * BLOCK as f32, |b| free(world, b, &bodies), |b| supported(world, b)) else { return };
            let report = sim.world.apply_edit(&WorldEdit::PlaceBlock { block, material, back: false });
            inv.take(slot, report.placed);
            hand.cooldown = 1.0 / PLACE_RATE;
        }
        Use::Throw(what) if clicked => {
            let dir = (cursor - from).normalize_or(Vec2::X);
            let speed = tools.bomb.throw_speed * ((cursor - from).length() / 120.0).clamp(0.35, 1.0);
            let vel = dir * speed + k.body.vel * 0.5;
            match what {
                Throwable::Bomb => spawn_bomb(&mut commands, from, vel, tools.bomb.clone()),
                Throwable::Glowstick => {
                    hand.thrown += 1;
                    let s = lights.glowstick.strength;
                    let color = if hand.thrown.is_multiple_of(2) { [0.25 * s, 1.0 * s, 0.45 * s] } else { [0.2 * s, 0.55 * s, 1.1 * s] };
                    spawn_glowstick(&mut commands, from, vel, color, lights.glowstick.secs);
                }
            }
            inv.take(slot, 1);
        }
        Use::Chest if clicked => {
            let Some(corner) = chests::place_spot(&sim.world, cursor) else { return };
            let centre = Vec2::new(corner.x as f32 + 4.0, corner.y as f32 + 4.0);
            let bodies: Vec<Body> = creatures.iter().map(|k| k.body).collect();
            let in_the_way = bodies.iter().any(|b| (b.pos - centre).abs().cmple(b.half + Vec2::splat(4.0)).all());
            let Some(chest) = sim.materials().id("chest") else { return };
            if centre.distance(from) > 6.0 * BLOCK as f32 || in_the_way {
                return;
            }
            let side = chests::SIDE;
            sim.world.apply_edit(&WorldEdit::Stamp { corner, w: side, h: side, material: chest });
            chests.placed(corner);
            inv.take(slot, 1);
        }
        Use::Torch if clicked => {
            let bodies: Vec<Body> = Vec::new();
            let world = &sim.world;
            let Some(block) = target::place_target(from, cursor, 6.0 * BLOCK as f32, |b| free(world, b, &bodies), |b| supported(world, b)) else { return };
            plant_torch(&mut commands, Vec2::new((block.x as f32 + 0.5) * BLOCK as f32, block.y as f32 * BLOCK as f32 + 3.0), &lights);
            inv.take(slot, 1);
        }
        _ => {}
    }
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
            if dist > MAGNET || !inv.has_room(&items, d.stack.item) {
                continue;
            }
            if dist < GRAB {
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

/// The block the hands would act on, outlined: yellow to mine, cyan to place.
fn outline(
    input: Res<HandInput>,
    items: Option<Res<Items>>,
    hand: Res<Hand>,
    sim: Res<SimWorld>,
    player: Query<(&Kinematics, &Inventory), With<LocalPlayer>>,
    creatures: Query<&Kinematics, With<Creature>>,
    mut gizmos: Gizmos,
) {
    let (Some(items), Some(cursor)) = (items, input.cursor) else { return };
    let Ok((k, inv)) = player.single() else { return };
    let from = hand_at(k);
    let slot = if input.auto { auto_slot(&sim.world, &items, inv, &k.body, from, cursor).unwrap_or(hand.slot) } else { hand.slot };
    let Some(stack) = inv.slots[slot] else { return };
    let world = &sim.world;
    let (block, color) = match items.def(stack.item).use_ {
        Use::Mine { back, tier, reach, .. } => (mine_at(world, &k.body, from, cursor, hand.smart, (back, tier, reach)), Color::srgba(1.0, 0.9, 0.3, 0.9)),
        Use::Chest => {
            if let Some(c) = chests::place_spot(world, cursor) {
                let centre = Vec2::new(c.x as f32 + 4.0, c.y as f32 + 4.0);
                gizmos.rect_2d(bevy::math::Isometry2d::from_translation(centre), Vec2::splat(chests::SIDE as f32), Color::srgba(0.4, 0.9, 1.0, 0.9));
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
