//! The hotbar (always, in play mode), the inventory screen (Esc or I: the
//! three hotbars and the pack) and an open chest, Terraria-style:
//!
//! - drag a stack to another slot, or click it to pick it up and click
//!   where it goes (swapping, or merging the same item); right-click takes
//!   half a stack;
//! - Shift-click moves a stack between the hotbars and the pack, or with a
//!   chest open between the chest and the pack;
//! - click outside the panels holding a stack to throw it out;
//! - hover a slot for what's in it (a tooltip);
//! - X switches hotbar (also: click a hotbar's number).

use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use bevy::window::PrimaryWindow;

use super::chests::{Chests, SLOTS};
use super::icons::Icons;
use super::items::{BARS, HOTBAR, Inventory, ItemDef, Items, Stack, Use};
use super::{DevTools, Hand, PACK, spawn_drop};
use crate::actors::Kinematics;
use crate::actors::player::LocalPlayer;
use crate::magic::Spellbook;
use crate::world::SimWorld;

pub struct UiPlugin;

/// The inventory is open: the mouse works it, not the world.
#[derive(Resource, Default)]
pub struct InventoryOpen(pub bool);

/// A stack picked up with the mouse, on its way to another slot, and the
/// slot it was dragged from (while the button is still down).
#[derive(Resource, Default)]
struct Held {
    stack: Option<Stack>,
    from: Option<(Holder, usize)>,
}

/// Whose slot: the hotbar in use (bottom of the screen, by position), the
/// player's inventory (by slot), or the open chest.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Holder {
    Bar,
    Pack,
    Chest,
}

#[derive(Component)]
pub(crate) struct SlotUi(pub(crate) Holder, pub(crate) usize);

#[derive(Component)]
struct SlotIcon(Holder, usize);

#[derive(Component)]
struct SlotCount(Holder, usize);

/// A hotbar's number in the inventory screen (click: use that hotbar).
#[derive(Component)]
struct BarTag(usize);

#[derive(Component)]
struct ChestRoot;

#[derive(Component)]
struct HotbarRoot;

#[derive(Component)]
struct PackRoot;

#[derive(Component)]
struct ItemLabel;

#[derive(Component)]
struct HeldIcon;

#[derive(Component)]
struct HeldCount;

#[derive(Component)]
struct Tooltip;

const SLOT: f32 = 44.0;
const ICON_PX: f32 = 32.0;
const EMPTY: Color = Color::srgba(0.08, 0.08, 0.1, 0.72);
const EDGE: Color = Color::srgba(0.5, 0.5, 0.55, 0.8);
const CHOSEN: Color = Color::srgb(1.0, 0.85, 0.3);
const BAR_BG: Color = Color::srgba(0.1, 0.1, 0.16, 0.8);

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryOpen>()
            .init_resource::<Held>()
            .add_systems(Startup, spawn)
            .add_systems(Update, (toggle, press, release, show, tooltip).chain());
    }
}

fn slot(parent: &mut ChildSpawnerCommands, which: Holder, i: usize) {
    parent
        .spawn((
            Button,
            RelativeCursorPosition::default(),
            SlotUi(which, i),
            Node {
                width: px(SLOT),
                height: px(SLOT),
                border: UiRect::all(px(2)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(EDGE),
            BackgroundColor(EMPTY),
        ))
        .with_children(|s| {
            s.spawn((SlotIcon(which, i), ImageNode::default(), Node { width: px(ICON_PX), height: px(ICON_PX), ..default() }, BackgroundColor(Color::NONE)));
            s.spawn((
                SlotCount(which, i),
                Text::new(""),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(Color::WHITE),
                Node { position_type: PositionType::Absolute, right: px(3), bottom: px(1), ..default() },
            ));
        });
}

fn grid() -> Node {
    Node { display: Display::Grid, grid_template_columns: RepeatedGridTrack::px(HOTBAR as u16, SLOT), column_gap: px(3), row_gap: px(3), ..default() }
}

fn spawn(mut commands: Commands) {
    let small = |s: &str| (Text::new(s), TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(Color::srgb(0.85, 0.85, 0.9)));
    // The hotbar in use and its label, bottom centre.
    commands
        .spawn((
            HotbarRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(8),
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(4),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                ItemLabel,
                Text::new(""),
                TextFont { font_size: FontSize::Px(13.0), ..default() },
                TextColor(Color::WHITE),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
                Node { padding: UiRect::axes(px(6), px(2)), ..default() },
            ));
            root.spawn(Node { column_gap: px(3), ..default() }).with_children(|row| {
                for i in 0..HOTBAR {
                    slot(row, Holder::Bar, i);
                }
            });
        });
    // The inventory screen: the hotbars (numbered), then the pack.
    commands
        .spawn((
            PackRoot,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(8),
                width: percent(100),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Interaction::None,
                Node { flex_direction: FlexDirection::Column, row_gap: px(6), padding: UiRect::all(px(8)), ..default() },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.62)),
            ))
            .with_children(|panel| {
                panel.spawn(small("Inventory  |  drag or click: move  |  right-click: half  |  Shift-click: across  |  X: next hotbar"));
                for bar in 0..BARS {
                    panel.spawn(Node { column_gap: px(6), align_items: AlignItems::Center, ..default() }).with_children(|row| {
                        row.spawn((
                            Button,
                            BarTag(bar),
                            Node { width: px(20), height: px(SLOT), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                            BackgroundColor(BAR_BG),
                        ))
                        .with_child((Text::new(format!("{}", bar + 1)), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(Color::WHITE)));
                        row.spawn(grid()).with_children(|g| {
                            for i in bar * HOTBAR..(bar + 1) * HOTBAR {
                                slot(g, Holder::Pack, i);
                            }
                        });
                    });
                }
                panel.spawn(small("Pack"));
                panel.spawn(Node { padding: UiRect::left(px(26)), ..default() }).with_children(|row| {
                    row.spawn(grid()).with_children(|g| {
                        for i in BARS * HOTBAR..PACK {
                            slot(g, Holder::Pack, i);
                        }
                    });
                });
            });
        });
    // An open chest, above the inventory.
    commands
        .spawn((
            ChestRoot,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                bottom: px((SLOT + 3.0) * (BARS + 3) as f32 + 100.0),
                width: percent(100),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Interaction::None,
                Node { flex_direction: FlexDirection::Column, row_gap: px(6), padding: UiRect::all(px(8)), ..default() },
                BackgroundColor(Color::srgba(0.12, 0.08, 0.04, 0.7)),
            ))
            .with_children(|panel| {
                panel.spawn(small("Chest  |  Shift-click: to the pack  |  R: take all"));
                panel.spawn(grid()).with_children(|g| {
                    for i in 0..SLOTS {
                        slot(g, Holder::Chest, i);
                    }
                });
            });
        });
    // The stack in the mouse's grip.
    commands
        .spawn((HeldIcon, ImageNode::default(), Visibility::Hidden, Node { position_type: PositionType::Absolute, width: px(ICON_PX), height: px(ICON_PX), ..default() }, BackgroundColor(Color::NONE), GlobalZIndex(10)))
        .with_child((
            HeldCount,
            Text::new(""),
            TextFont { font_size: FontSize::Px(12.0), ..default() },
            TextColor(Color::WHITE),
            Node { position_type: PositionType::Absolute, right: px(-6), bottom: px(-4), ..default() },
        ));
    // What's under the mouse.
    commands.spawn((
        Tooltip,
        Visibility::Hidden,
        Text::new(""),
        TextFont { font_size: FontSize::Px(13.0), ..default() },
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, padding: UiRect::axes(px(8), px(5)), max_width: px(360), ..default() },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.92)),
        BorderColor::all(Color::srgba(0.6, 0.6, 0.7, 0.8)),
        GlobalZIndex(20),
    ));
}

#[allow(clippy::too_many_arguments)]
fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    items: Option<Res<Items>>,
    mut open: ResMut<InventoryOpen>,
    mut held: ResMut<Held>,
    mut chests: ResMut<Chests>,
    mut inv: Query<&mut Inventory, With<LocalPlayer>>,
    mut pack: Query<&mut Visibility, PackOnly>,
    mut chest_panel: Query<&mut Visibility, (With<ChestRoot>, Without<HotbarRoot>)>,
    mut hotbar: Query<&mut Visibility, With<HotbarRoot>>,
    dev: Res<DevTools>,
) {
    for mut v in &mut chest_panel {
        *v = if chests.open.is_some() { Visibility::Visible } else { Visibility::Hidden };
    }
    // (The inventory screen holds the hotbars, so the bottom one steps aside.)
    for mut v in &mut hotbar {
        *v = if dev.0 || open.0 { Visibility::Hidden } else { Visibility::Visible };
    }
    let want = if keys.any_just_pressed([KeyCode::KeyI, KeyCode::Escape]) { !open.0 } else { open.0 };
    if want == open.0 && pack.iter().next().is_some_and(|v| (*v == Visibility::Visible) == open.0) {
        return;
    }
    open.0 = want;
    if !open.0 {
        chests.open = None;
    }
    // Closing with something in hand puts it back.
    if !open.0
        && let (Some(stack), Some(items), Ok(mut inv)) = (held.stack.take(), items, inv.single_mut())
    {
        inv.add(&items, stack);
    }
    for mut v in &mut pack {
        *v = if open.0 { Visibility::Visible } else { Visibility::Hidden };
    }
}

type SlotQuery<'a> = (&'a RelativeCursorPosition, &'a SlotUi, &'a InheritedVisibility);

/// The slot under the mouse (shown ones only). (Not `Interaction`: while
/// a drag's button is down, the slot it started on stays `Pressed`.)
fn under(slots: &Query<SlotQuery>) -> Option<(Holder, usize)> {
    slots.iter().find(|(c, _, v)| c.cursor_over() && v.get()).map(|(_, s, _)| (s.0, s.1))
}

/// The inventory slot a UI slot shows.
fn index(hand: &Hand, which: Holder, i: usize) -> usize {
    if which == Holder::Bar { hand.bar * HOTBAR + i } else { i }
}

/// A slot's stack, wherever it is.
fn slot_mut<'a>(inv: &'a mut Inventory, chest: Option<&'a mut Inventory>, hand: &Hand, which: Holder, i: usize) -> Option<&'a mut Option<Stack>> {
    match which {
        Holder::Bar | Holder::Pack => inv.slots.get_mut(index(hand, which, i)),
        Holder::Chest => chest?.slots.get_mut(i),
    }
}

/// Mouse down on a slot: pick up, put down, take half, move across.
#[allow(clippy::too_many_arguments)]
fn press(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    items: Option<Res<Items>>,
    open: Res<InventoryOpen>,
    over_ui: Res<crate::dev::PointerOverUi>,
    sim: Res<SimWorld>,
    mut commands: Commands,
    mut chests: ResMut<Chests>,
    mut hand: ResMut<Hand>,
    mut held: ResMut<Held>,
    slots: Query<SlotQuery>,
    tags: Query<(&Interaction, &BarTag), Changed<Interaction>>,
    mut player: Query<(&Kinematics, &mut Inventory), With<LocalPlayer>>,
) {
    let (Some(items), Ok((k, mut inv))) = (items, player.single_mut()) else { return };
    for (interaction, tag) in &tags {
        if *interaction == Interaction::Pressed {
            hand.bar = tag.0;
        }
    }
    let (left, right) = (mouse.just_pressed(MouseButton::Left), mouse.just_pressed(MouseButton::Right));
    if !left && !right {
        return;
    }
    let Some((which, i)) = under(&slots) else {
        // Holding something, clicking outside the panels: throw it out.
        if left
            && open.0
            && !over_ui.0
            && let Some(stack) = held.stack.take()
        {
            spawn_drop(&mut commands, &items, k.body.pos + Vec2::new(0.0, k.body.half.y), stack);
        }
        return;
    };
    if !open.0 {
        // Closed: clicking the hotbar just picks the slot.
        if which == Holder::Bar && left {
            hand.slot = i;
        }
        return;
    }
    let hand = &*hand;
    let chest_key = chests.open;
    if which == Holder::Chest && chest_key.is_none() {
        return;
    }
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    if left && shift && held.stack.is_none() {
        // Across: chest → pack; pack → chest (open) or hotbars ↔ pack.
        let from = index(hand, which, i);
        let stack = match which {
            Holder::Chest => chest_key.and_then(|c| chests.contents(c, &sim.world, &items).slots[i].take()),
            _ => inv.slots[from].take(),
        };
        let Some(stack) = stack else { return };
        let left_over = match (which, chest_key) {
            (Holder::Chest, _) => inv.add(&items, stack),
            (_, Some(c)) => chests.contents(c, &sim.world, &items).add(&items, stack),
            _ => {
                let range = if from < BARS * HOTBAR { BARS * HOTBAR..PACK } else { 0..BARS * HOTBAR };
                let mut other = Inventory { slots: inv.slots[range.clone()].to_vec() };
                let n = other.add(&items, stack);
                inv.slots[range].copy_from_slice(&other.slots);
                n
            }
        };
        if left_over > 0 {
            let back = Some(Stack { item: stack.item, count: left_over });
            match which {
                Holder::Chest => {
                    if let Some(c) = chest_key {
                        chests.contents(c, &sim.world, &items).slots[i] = back;
                    }
                }
                _ => inv.slots[from] = back,
            }
        }
        return;
    }
    let chest = chest_key.map(|c| chests.contents(c, &sim.world, &items));
    let Some(slot) = slot_mut(&mut inv, chest, hand, which, i) else { return };
    if right {
        // Half of it (whole items: rounded up) into the hand, if the hand is free.
        if held.stack.is_none()
            && let Some(s) = *slot
        {
            let unit = items.unit(s.item);
            let half = (s.count / unit).div_ceil(2).max(1) * unit;
            let take = half.min(s.count);
            held.stack = Some(Stack { item: s.item, count: take });
            *slot = (s.count > take).then_some(Stack { item: s.item, count: s.count - take });
        }
        return;
    }
    if held.stack.is_none() {
        // Picked up: drag it somewhere, or let go here to carry it.
        held.stack = slot.take();
        held.from = held.stack.map(|_| (which, i));
    } else {
        put(&items, &mut held.stack, slot);
        held.from = None;
    }
}

/// Mouse up after a drag: into the slot it's over (back where it was, it's
/// just picked up; over nothing, it stays in hand to be clicked down).
#[allow(clippy::too_many_arguments)]
fn release(
    mouse: Res<ButtonInput<MouseButton>>,
    items: Option<Res<Items>>,
    open: Res<InventoryOpen>,
    sim: Res<SimWorld>,
    hand: Res<Hand>,
    mut chests: ResMut<Chests>,
    mut held: ResMut<Held>,
    slots: Query<SlotQuery>,
    mut inv: Query<&mut Inventory, With<LocalPlayer>>,
) {
    if !mouse.just_released(MouseButton::Left) || !open.0 {
        return;
    }
    let Some(from) = held.from.take() else { return };
    let (Some(items), Ok(mut inv)) = (items, inv.single_mut()) else { return };
    let Some((which, i)) = under(&slots) else { return };
    if (which, i) == from {
        return;
    }
    let chest = chests.open.map(|c| chests.contents(c, &sim.world, &items));
    let Some(slot) = slot_mut(&mut inv, chest, &hand, which, i) else { return };
    put(&items, &mut held.stack, slot);
}

/// Put the held stack in a slot: merge the same item as far as it goes,
/// otherwise swap.
fn put(items: &Items, held: &mut Option<Stack>, slot: &mut Option<Stack>) {
    match (*held, *slot) {
        (Some(h), Some(s)) if h.item == s.item => {
            let n = (items.stack_units(s.item) - s.count).min(h.count);
            *slot = Some(Stack { item: s.item, count: s.count + n });
            *held = (h.count > n).then_some(Stack { item: h.item, count: h.count - n });
        }
        (h, s) => {
            *slot = h;
            *held = s;
        }
    }
}

// (The UI queries touch the same components on different nodes; these say
// which.)
type PackOnly = (With<PackRoot>, Without<ChestRoot>, Without<HotbarRoot>);
type TagsOnly = (Without<SlotIcon>, Without<HeldIcon>);
type IconsOnly = (Without<HeldIcon>, Without<BarTag>);
type CountsOnly = (Without<ItemLabel>, Without<HeldCount>);
type LabelOnly = (With<ItemLabel>, Without<HeldCount>);
type Grip<'a> = (&'a mut Node, &'a mut ImageNode, &'a mut BackgroundColor, &'a mut Visibility);
type GripOnly = (With<HeldIcon>, Without<SlotIcon>);

/// Whole items in a stack (blocks count in cells).
fn whole(items: &Items, s: &Stack) -> u32 {
    s.count / items.unit(s.item)
}

#[allow(clippy::too_many_arguments)]
fn show(
    items: Option<Res<Items>>,
    icons: Option<Res<Icons>>,
    hand: Res<Hand>,
    held: Res<Held>,
    window: Single<&Window, With<PrimaryWindow>>,
    inv: Query<&Inventory, With<LocalPlayer>>,
    sim: Res<SimWorld>,
    mut chests: ResMut<Chests>,
    mut borders: Query<(&SlotUi, &mut BorderColor)>,
    mut tags: Query<(&BarTag, &mut BackgroundColor), TagsOnly>,
    mut slot_icons: Query<(&SlotIcon, &mut ImageNode, &mut BackgroundColor), IconsOnly>,
    mut counts: Query<(&SlotCount, &mut Text), CountsOnly>,
    mut label: Single<&mut Text, LabelOnly>,
    mut grip: Single<Grip, GripOnly>,
    mut grip_count: Single<&mut Text, With<HeldCount>>,
) {
    let (Some(items), Ok(inv)) = (items, inv.single()) else { return };
    // An icon, or else the item's colour.
    let look = |s: Option<&Stack>| -> (Handle<Image>, Color) {
        let Some(s) = s else { return (Handle::default(), Color::NONE) };
        match icons.as_ref().and_then(|ic| ic.get(s.item)) {
            Some(h) => (h.clone(), Color::NONE),
            None => {
                let (r, g, b) = items.def(s.item).color;
                (Handle::default(), Color::srgb_u8(r, g, b))
            }
        }
    };
    let stash: Vec<Option<Stack>> = match chests.open {
        Some(c) => chests.contents(c, &sim.world, &items).slots.clone(),
        None => vec![None; SLOTS],
    };
    let slot_of = |which: Holder, i: usize| if which == Holder::Chest { stash[i] } else { inv.slots[index(&hand, which, i)] };
    for (&SlotUi(which, i), mut border) in &mut borders {
        let chosen = which != Holder::Chest && index(&hand, which, i) == hand.active();
        *border = BorderColor::all(if chosen { CHOSEN } else { EDGE });
    }
    for (tag, mut bg) in &mut tags {
        bg.0 = if tag.0 == hand.bar { Color::srgba(0.55, 0.45, 0.12, 0.9) } else { BAR_BG };
    }
    for (&SlotIcon(which, i), mut image, mut bg) in &mut slot_icons {
        let (handle, color) = look(slot_of(which, i).as_ref());
        if image.image != handle {
            image.image = handle.clone();
        }
        image.color = if handle == Handle::default() { Color::NONE } else { Color::WHITE };
        bg.0 = color;
    }
    for (&SlotCount(which, i), mut text) in &mut counts {
        let n = slot_of(which, i).map_or(0, |s| whole(&items, &s));
        text.0 = if n > 1 { n.to_string() } else { String::new() };
    }
    let name = inv.slots[hand.active()].map_or(String::new(), |s| {
        let unit = items.unit(s.item);
        let spare = s.count % unit;
        if unit > 1 && spare > 0 { format!("{} ({} + {spare}/{unit})", items.def(s.item).name, s.count / unit) } else { items.def(s.item).name.clone() }
    });
    let smart = if hand.smart { "smart cursor" } else { "plain cursor" };
    label.0 = format!("{name}   |   hotbar {} of {BARS} [X]   |   {smart} [Alt]   |   inventory [Esc]   |   dev tools: key left of 1", hand.bar + 1);
    let (node, image, bg, vis) = &mut *grip;
    match (held.stack, window.cursor_position()) {
        (Some(s), Some(at)) => {
            **vis = Visibility::Visible;
            let (handle, color) = look(Some(&s));
            image.image = handle.clone();
            image.color = if handle == Handle::default() { Color::NONE } else { Color::WHITE };
            bg.0 = color;
            node.left = px(at.x + 6.0);
            node.top = px(at.y + 6.0);
            let n = whole(&items, &s);
            grip_count.0 = if n > 1 { n.to_string() } else { String::new() };
        }
        _ => **vis = Visibility::Hidden,
    }
}

/// Over a slot with something in it: what it is and what it does.
#[allow(clippy::too_many_arguments)]
fn tooltip(
    items: Option<Res<Items>>,
    book: Option<Res<Spellbook>>,
    weapons: Option<Res<crate::combat::Weapons>>,
    hand: Res<Hand>,
    held: Res<Held>,
    sim: Res<SimWorld>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut chests: ResMut<Chests>,
    slots: Query<SlotQuery>,
    inv: Query<&Inventory, With<LocalPlayer>>,
    mut tip: Single<(&mut Text, &mut Node, &mut Visibility), With<Tooltip>>,
) {
    let (text, node, vis) = &mut *tip;
    **vis = Visibility::Hidden;
    let (Some(items), Ok(inv), Some(at)) = (items, inv.single(), window.cursor_position()) else { return };
    if held.stack.is_some() {
        return;
    }
    let Some((which, i)) = under(&slots) else { return };
    let stack = match which {
        Holder::Chest => chests.open.and_then(|c| chests.contents(c, &sim.world, &items).slots[i]),
        _ => inv.slots[index(&hand, which, i)],
    };
    let Some(stack) = stack else { return };
    text.0 = describe(&items, book.as_deref(), weapons.as_deref(), &stack);
    node.left = px(at.x + 18.0);
    // (Above the cursor near the bottom, where the hotbars are.)
    node.top = px(if at.y > window.height() * 0.5 { at.y - 110.0 } else { at.y + 18.0 });
    **vis = Visibility::Visible;
}

/// A stack, in words: its name and count, what it does, its note.
fn describe(items: &Items, book: Option<&Spellbook>, weapons: Option<&crate::combat::Weapons>, s: &Stack) -> String {
    let def: &ItemDef = items.def(s.item);
    let n = whole(items, s);
    let mut lines = vec![if n > 1 { format!("{} ({n})", def.name) } else { def.name.clone() }];
    match &def.use_ {
        Use::Mine { back: false, power, tier, speed, reach } => {
            lines.push(format!("Pickaxe: mines up to hardness {tier}"));
            lines.push(format!("{power} a hit, {speed} hits a second, reach {reach} blocks"));
        }
        Use::Mine { back: true, power, tier, speed, reach } => {
            lines.push(format!("Axe: trees and walls, up to hardness {tier}"));
            lines.push(format!("{power} a hit, {speed} hits a second, reach {reach} blocks"));
        }
        Use::Cast { runes, delay, .. } => {
            lines.push("Wand: hold to cast at the cursor".into());
            if let Some(book) = book {
                let (names, mana) = book.describe(runes);
                lines.push(format!("Runes: {}", names.join(" + ")));
                lines.push(format!("{mana:.0} mana a cast, {:.1} casts a second", 1.0 / delay.max(0.01)));
            }
        }
        Use::Bow(id) => {
            lines.push("Bow: hold the left button to draw, let go to loose".into());
            if let Some(b) = weapons.and_then(|w| w.bow_index(id).map(|i| w.bow(i))) {
                lines.push(format!("{:.0} to {:.0} damage by how far it's drawn ({:.2} s full)", b.damage.0, b.damage.1, b.draw));
            }
        }
        Use::Melee(id) => {
            lines.push("Weapon: hold the left button to swing at the cursor".into());
            if let Some(w) = weapons.and_then(|w| w.index(id).map(|i| w.def(i))) {
                let names: Vec<&str> = w.moves.iter().map(|m| m.name.as_str()).collect();
                lines.push(format!("{:.0} damage, knockback {:.0}; {}", w.damage, w.knock, names.join(", then ")));
            }
        }
        Use::Throw(t) => lines.push(format!("{t:?}: click to throw it toward the cursor")),
        Use::Torch => lines.push("Click to plant it on a block: light".into()),
        Use::Chest => lines.push("Click to place it on the ground; right-click it to open".into()),
        Use::Block(_) => {
            let spare = s.count % items.unit(s.item);
            lines.push("Block: hold to build with it".into());
            if spare > 0 {
                lines.push(format!("and {spare} of {} cells toward another", items.unit(s.item)));
            }
        }
        Use::None => {}
    }
    if let Some(about) = &def.about {
        lines.push(about.clone());
    }
    lines.join("\n")
}
