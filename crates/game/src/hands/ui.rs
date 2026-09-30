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
//! - X switches hotbar (also: click a hotbar's number);
//! - gear goes in the equipment slots beside the pack (drag it there, or
//!   Shift-click it: on, and off again; while it's held, the slots it goes
//!   in light up); what it all adds up to is listed beside them.

use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use bevy::window::PrimaryWindow;

use super::chests::{Chests, SLOTS};
use super::icons::Icons;
use super::items::{BARS, HOTBAR, Inventory, ItemDef, Items, Stack, Use};
use super::{DevTools, Hand, PACK, PACK_ROWS, spawn_drop};
use crate::actors::Kinematics;
use crate::actors::player::LocalPlayer;
use crate::gear::{Equipment, GearRules, Stats, WORN};
use crate::magic::Spellbook;
use crate::world::SimWorld;

pub struct UiPlugin;

/// The inventory is open: the mouse works it, not the world.
#[derive(Resource, Default)]
pub struct InventoryOpen(pub bool);

/// A stack picked up with the mouse, on its way to another slot, and the
/// slot it was dragged from (while the button is still down).
#[derive(Resource, Default)]
pub(crate) struct Held {
    pub(crate) stack: Option<Stack>,
    from: Option<(Holder, usize)>,
    /// The discard slot's stack: the last thing thrown in, to take back
    /// until the next goes in (and this one's gone for good).
    trash: Option<Stack>,
}

/// Whose slot: the hotbar in use (bottom of the screen, by position), the
/// player's inventory (by slot), the open chest, or what the player wears
/// (by `gear::WORN` slot), or the discard slot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Holder {
    Bar,
    Pack,
    Chest,
    Equip,
    Trash,
}

#[derive(Component)]
pub(crate) struct SlotUi(pub(crate) Holder, pub(crate) usize);

#[derive(Component)]
struct SlotIcon(Holder, usize);

#[derive(Component)]
struct SlotCount(Holder, usize);

/// A hotbar's number in the inventory screen (click: use that hotbar).
/// The keys' line, bottom centre.
#[derive(Component)]
struct HintLabel;

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

/// The tooltip's lines under its title.
#[derive(Component)]
struct TooltipBody;

/// An empty equipment slot's name.
#[derive(Component)]
struct SlotName(usize);

/// What the player's gear adds up to.
#[derive(Component)]
struct StatsText;

/// What the open chest (or body) is called.
#[derive(Component)]
struct ChestTitle;

/// A slot's size, and the hotbar's chosen one (bigger, gold: Terraria's).
const SLOT: f32 = 44.0;
const SLOT_BIG: f32 = 52.0;
const ICON_PX: f32 = 32.0;
/// Slots: rounded, deep blue, a darker edge.
const EMPTY: Color = Color::srgba(0.16, 0.2, 0.42, 0.78);
const EDGE: Color = Color::srgba(0.07, 0.08, 0.2, 0.9);
const CHOSEN: Color = Color::srgb(1.0, 0.88, 0.35);
const CHOSEN_BG: Color = Color::srgba(0.95, 0.76, 0.2, 0.92);
const ROUND: f32 = 7.0;
/// An equipment slot the gear in the mouse's grip would go in (white: no
/// rarity has it).
const FITS: Color = Color::WHITE;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryOpen>()
            .init_resource::<Held>()
            .add_systems(Startup, spawn)
            .add_systems(Update, (toggle, press, release, show, show_gear, title, tooltip, hints).chain());
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
                border_radius: BorderRadius::all(px(ROUND)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(EDGE),
            BackgroundColor(EMPTY),
        ))
        .with_children(|s| {
            s.spawn((SlotIcon(which, i), ImageNode::default(), Node { width: px(ICON_PX), height: px(ICON_PX), ..default() }, BackgroundColor(Color::NONE)));
            // The hotbar's slots: their key, top left.
            if which == Holder::Bar {
                s.spawn((
                    Text::new(format!("{}", (i + 1) % 10)),
                    TextFont { font_size: FontSize::Px(11.0), ..default() },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
                    Node { position_type: PositionType::Absolute, left: px(4), top: px(1), ..default() },
                ));
            }
            if which == Holder::Equip {
                s.spawn((
                    SlotName(i),
                    Text::new(WORN[i].label()),
                    TextFont { font_size: FontSize::Px(10.0), ..default() },
                    TextColor(Color::srgba(0.7, 0.7, 0.8, 0.55)),
                    Node { position_type: PositionType::Absolute, ..default() },
                ));
            }
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
    Node { display: Display::Grid, grid_template_columns: RepeatedGridTrack::px(HOTBAR as u16, SLOT), column_gap: px(4), row_gap: px(4), ..default() }
}

/// A tight dark shadow under text over the world (readable on sky or
/// rock).
fn shadow() -> TextShadow {
    TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85) }
}

/// The held item's name over the hotbar: at most this many letters.
const LABEL_CHARS: usize = 34;

/// Where the inventory starts: under the hotbar.
const PACK_TOP: f32 = 86.0;

fn spawn(mut commands: Commands) {
    let small = |s: &str| (Text::new(s), TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(Color::srgb(0.85, 0.85, 0.9)));
    // The hotbar in use, top left (Terraria's): what's held named over it.
    // Open, the inventory hangs below it.
    commands
        .spawn((
            HotbarRoot,
            Node {
                position_type: PositionType::Absolute,
                top: px(4),
                left: px(10),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(3),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                ItemLabel,
                Text::new(""),
                TextFont { font_size: FontSize::Px(15.0), ..default() },
                TextColor(Color::WHITE),
                shadow(),
                // (One line; a long name is cut short with an ellipsis, so
                // it never widens the column and pushes the hotbar out.)
                TextLayout::default().with_no_wrap(),
                Node { min_height: px(18), ..default() },
            ));
            root.spawn(Node { column_gap: px(4), align_items: AlignItems::Center, ..default() }).with_children(|row| {
                for i in 0..HOTBAR {
                    slot(row, Holder::Bar, i);
                }
            });
        });
    // What the keys do, small, bottom centre.
    commands.spawn((
        Node { position_type: PositionType::Absolute, bottom: px(6), width: percent(100), justify_content: JustifyContent::Center, ..default() },
    )).with_child((
        HintLabel,
        Text::new(""),
        TextFont { font_size: FontSize::Px(12.0), ..default() },
        TextColor(Color::srgba(0.85, 0.85, 0.9, 0.8)),
        shadow(),
    ));
    // The inventory screen: the hotbars (numbered), then the pack.
    commands
        .spawn((
            PackRoot,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                top: px(PACK_TOP),
                left: px(10),
                align_items: AlignItems::Start,
                column_gap: px(6),
                ..default()
            },
        ))
        .with_children(|root| {
            // (No box behind the pack, as Terraria's: just the slots.)
            root.spawn((Interaction::None, Node { flex_direction: FlexDirection::Column, row_gap: px(6), ..default() }))
            .with_children(|panel| {
                panel.spawn((small("Inventory: drag or click to move, right-click half, Shift-click across, Ctrl-click to discard"), shadow()));
                panel.spawn(Node { column_gap: px(10), align_items: AlignItems::End, ..default() }).with_children(|row| {
                    row.spawn(grid()).with_children(|g| {
                        for i in BARS * HOTBAR..PACK {
                            slot(g, Holder::Pack, i);
                        }
                    });
                    // The discard slot: what goes in is gone once the next
                    // thing does.
                    row.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: px(3), ..default() }).with_children(|c| {
                        c.spawn((small("Discard"), shadow()));
                        slot(c, Holder::Trash, 0);
                    });
                });
            });
            // The gear worn, and what it adds up to, right of the pack.
            root.spawn((
                Interaction::None,
                Node { flex_direction: FlexDirection::Column, row_gap: px(4), padding: UiRect::all(px(8)), width: px(200), ..default() },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.62)),
            ))
            .with_children(|panel| {
                panel.spawn(small("Stats"));
                panel.spawn((StatsText, Text::new(""), TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(Color::srgb(0.9, 0.9, 0.95))));
            });
            root.spawn((
                Interaction::None,
                Node { flex_direction: FlexDirection::Column, row_gap: px(3), padding: UiRect::all(px(8)), align_items: AlignItems::Center, ..default() },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.62)),
            ))
            .with_children(|panel| {
                panel.spawn(small("Worn"));
                for i in 0..WORN.len() {
                    slot(panel, Holder::Equip, i);
                }
            });
        });
    // An open chest, above the inventory.
    commands
        .spawn((
            ChestRoot,
            Visibility::Hidden,
            // (Under the pack, top left.)
            Node {
                position_type: PositionType::Absolute,
                top: px(PACK_TOP + (SLOT + 3.0) * PACK_ROWS as f32 + 50.0),
                left: px(10),
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
                panel.spawn((ChestTitle, small("Chest  |  Shift-click: to the pack  |  R: take all")));
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
    // (Its title in the colour of its rarity, the rest under it.)
    commands
        .spawn((
            Tooltip,
            Visibility::Hidden,
            Text::new(""),
            TextFont { font_size: FontSize::Px(14.0), ..default() },
            TextColor(Color::WHITE),
            Node { position_type: PositionType::Absolute, padding: UiRect::axes(px(8), px(5)), border: UiRect::all(px(1)), max_width: px(360), ..default() },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.92)),
            BorderColor::all(Color::srgba(0.6, 0.6, 0.7, 0.8)),
            GlobalZIndex(20),
        ))
        .with_child((TooltipBody, TextSpan::new(""), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(Color::srgb(0.9, 0.9, 0.94))));
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
    // (The hotbar is the inventory's top row: it stays, but in dev mode.)
    for mut v in &mut hotbar {
        *v = if dev.0 { Visibility::Hidden } else { Visibility::Visible };
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
fn slot_mut<'a>(inv: &'a mut Inventory, chest: Option<&'a mut Inventory>, eq: &'a mut Equipment, hand: &Hand, which: Holder, i: usize) -> Option<&'a mut Option<Stack>> {
    match which {
        Holder::Bar | Holder::Pack => inv.slots.get_mut(index(hand, which, i)),
        Holder::Chest => chest?.slots.get_mut(i),
        Holder::Equip => eq.worn.get_mut(i),
        // (The discard slot is its own: `press`, `release`.)
        Holder::Trash => None,
    }
}

/// The keys' line: which hotbar, the cursor, the inventory, dev tools.
fn hints(hand: Res<Hand>, items: Option<Res<Items>>, player: Query<&Inventory, With<LocalPlayer>>, mut text: Single<&mut Text, With<HintLabel>>) {
    let smart = if hand.smart { "smart cursor" } else { "plain cursor" };
    // (A pickaxe in hand: its mode.)
    let pick = items.as_ref().zip(player.single().ok()).and_then(|(items, inv)| inv.slots[hand.active()].map(|s| items.def(s.item).use_.clone())).is_some_and(|u| matches!(u, Use::Mine { back: false, area, .. } if area > 0.0));
    let mode = if pick { format!("   |   {} [C]", if hand.area { "area" } else { "precise" }) } else { String::new() };
    let want = format!("hotbar {} of {BARS} [X]   |   {smart} [Alt]{mode}   |   inventory [Esc]   |   potion [H]   |   dev tools: key left of 1", hand.bar + 1);
    if text.0 != want {
        text.0 = want;
    }
}

/// Only gear of its kind goes in an equipment slot.
fn allowed(items: &Items, which: Holder, i: usize, stack: Option<&Stack>) -> bool {
    which != Holder::Equip || stack.is_none_or(|s| Equipment::fits(items, i, s))
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
    mut player: Query<(&Kinematics, &mut Inventory, &mut Equipment), With<LocalPlayer>>,
) {
    let (Some(items), Ok((k, mut inv, mut eq))) = (items, player.single_mut()) else { return };
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
    // The discard slot: something in hand goes in (what was there is gone);
    // an empty hand takes back what's there.
    if which == Holder::Trash {
        let h = &mut *held;
        if left {
            if h.stack.is_some() {
                h.trash = h.stack.take();
            } else {
                h.stack = h.trash.take();
                h.from = h.stack.map(|_| (which, i));
            }
        }
        return;
    }
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    // Ctrl-click: straight into the discard slot (the pack and hotbars').
    if left && keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight, KeyCode::SuperLeft, KeyCode::SuperRight]) && held.stack.is_none() && matches!(which, Holder::Bar | Holder::Pack) {
        let j = index(hand, which, i);
        if let Some(s) = inv.slots[j].take() {
            held.trash = Some(s);
        }
        return;
    }
    if left && shift && held.stack.is_none() && which == Holder::Equip {
        // Off: into the pack.
        if let Some(s) = eq.worn[i].take()
            && inv.add(&items, s) > 0
        {
            eq.worn[i] = Some(s);
        }
        return;
    }
    // Gear, with no chest open: on (what was worn goes where it was).
    if left
        && shift
        && held.stack.is_none()
        && chest_key.is_none()
        && which != Holder::Chest
        && let Some(s) = inv.slots[index(hand, which, i)]
        && let Some(j) = eq.slot_for(&items, &s)
    {
        inv.slots[index(hand, which, i)] = eq.worn[j].replace(s);
        return;
    }
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
            let back = Some(Stack { count: left_over, ..stack });
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
    if !allowed(&items, which, i, held.stack.as_ref()) {
        return;
    }
    let Some(slot) = slot_mut(&mut inv, chest, &mut eq, hand, which, i) else { return };
    if right {
        // Half of it (whole items: rounded up) into the hand, if the hand is free.
        if held.stack.is_none()
            && let Some(s) = *slot
        {
            let unit = items.unit(s.item);
            let half = (s.count / unit).div_ceil(2).max(1) * unit;
            let take = half.min(s.count);
            held.stack = Some(Stack { count: take, ..s });
            *slot = (s.count > take).then_some(Stack { count: s.count - take, ..s });
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
    mut player: Query<(&mut Inventory, &mut Equipment), With<LocalPlayer>>,
) {
    if !mouse.just_released(MouseButton::Left) || !open.0 {
        return;
    }
    let Some(from) = held.from.take() else { return };
    let (Some(items), Ok((mut inv, mut eq))) = (items, player.single_mut()) else { return };
    let Some((which, i)) = under(&slots) else { return };
    if (which, i) == from || !allowed(&items, which, i, held.stack.as_ref()) {
        return;
    }
    // Dragged onto the discard slot: in it goes.
    if which == Holder::Trash {
        let h = &mut *held;
        h.trash = h.stack.take();
        return;
    }
    let chest = chests.open.map(|c| chests.contents(c, &sim.world, &items));
    let Some(slot) = slot_mut(&mut inv, chest, &mut eq, &hand, which, i) else { return };
    put(&items, &mut held.stack, slot);
}

/// Put the held stack in a slot: merge the same item as far as it goes,
/// otherwise swap.
fn put(items: &Items, held: &mut Option<Stack>, slot: &mut Option<Stack>) {
    match (*held, *slot) {
        (Some(h), Some(s)) if h.same(&s) => {
            let n = (items.stack_units(s.item) - s.count).min(h.count);
            *slot = Some(Stack { count: s.count + n, ..s });
            *held = (h.count > n).then_some(Stack { count: h.count - n, ..h });
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
type SlotsOnly = (Without<SlotIcon>, Without<HeldIcon>);
type IconsOnly = (Without<HeldIcon>, Without<SlotUi>);
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
    rules: Res<GearRules>,
    icons: Option<Res<Icons>>,
    hand: Res<Hand>,
    held: Res<Held>,
    window: Single<&Window, With<PrimaryWindow>>,
    inv: Query<(&Inventory, &Equipment), With<LocalPlayer>>,
    sim: Res<SimWorld>,
    mut chests: ResMut<Chests>,
    mut borders: Query<(&SlotUi, &mut BorderColor, &mut Node, &mut BackgroundColor), SlotsOnly>,
    mut slot_icons: Query<(&SlotIcon, &mut ImageNode, &mut BackgroundColor), IconsOnly>,
    mut counts: Query<(&SlotCount, &mut Text), CountsOnly>,
    mut label: Single<&mut Text, LabelOnly>,
    mut grip: Single<Grip, GripOnly>,
    mut grip_count: Single<&mut Text, With<HeldCount>>,
) {
    let (Some(items), Ok((inv, eq))) = (items, inv.single()) else { return };
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
    let slot_of = |which: Holder, i: usize| match which {
        Holder::Chest => stash[i],
        Holder::Equip => eq.worn[i],
        Holder::Trash => held.trash,
        _ => inv.slots[index(&hand, which, i)],
    };
    for (&SlotUi(which, i), mut border, mut node, mut bg) in &mut borders {
        let chosen = matches!(which, Holder::Bar | Holder::Pack) && index(&hand, which, i) == hand.active();
        // (The hotbar's chosen slot: bigger, gold.)
        let big = which == Holder::Bar && chosen;
        let size = px(if big { SLOT_BIG } else { SLOT });
        if node.width != size {
            node.width = size;
            node.height = size;
        }
        bg.0 = if big { CHOSEN_BG } else { EMPTY };
        // (Gear finer than common is edged in its rarity's colour.)
        let rare = slot_of(which, i).filter(|s| s.roll.rarity > 0).and_then(|s| rules.rarity(&items, &s)).map(|r| Color::srgb_u8(r.color.0, r.color.1, r.color.2));
        // Holding gear: where it can go lights up.
        let fits = which == Holder::Equip && held.stack.is_some_and(|h| Equipment::fits(&items, i, &h));
        *border = BorderColor::all(if fits {
            FITS
        } else if chosen {
            CHOSEN
        } else {
            rare.unwrap_or(EDGE)
        });
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
        if unit > 1 && spare > 0 { format!("{} ({} + {spare}/{unit})", items.def(s.item).name, s.count / unit) } else { rules.name(&items, &s) }
    });
    // (Cut to LABEL_CHARS, well short of the hotbar's width.)
    let name = if name.chars().count() > LABEL_CHARS { format!("{}...", name.chars().take(LABEL_CHARS - 3).collect::<String>().trim_end()) } else { name };
    // (A pickaxe: its mode, C.)
    let pick = inv.slots[hand.active()].is_some_and(|s| matches!(items.def(s.item).use_, Use::Mine { back: false, area, .. } if area > 0.0));
    let name = if pick { format!("{name} ({})", if hand.area { "Area" } else { "Precise" }) } else { name };
    if label.0 != name {
        label.0 = name;
    }
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

/// Empty equipment slots say what goes there; the stats panel lists what
/// the player's gear adds up to.
fn show_gear(
    open: Res<InventoryOpen>,
    player: Query<(&Equipment, &Stats, &crate::actors::Health), With<LocalPlayer>>,
    mut names: Query<(&SlotName, &mut Visibility)>,
    mut text: Single<&mut Text, With<StatsText>>,
) {
    let Ok((eq, stats, health)) = player.single() else { return };
    if !open.0 {
        return;
    }
    for (n, mut v) in &mut names {
        let want = if eq.worn[n.0].is_some() { Visibility::Hidden } else { Visibility::Inherited };
        if *v != want {
            *v = want;
        }
    }
    let armor = stats.get(crate::gear::Stat::Armor);
    let mut lines = vec![
        format!("Health {:.0} / {:.0}", health.hp.max(0.0), health.max),
        format!("Armour {armor:.0} (stops {:.0}%)", (1.0 - health.ward.through(crate::actors::Harm::Physical)) * 100.0),
    ];
    lines.extend(stats.nonzero().filter(|(s, _)| *s != crate::gear::Stat::Armor).map(|(s, v)| crate::gear::stats::line(s, v)));
    let t = lines.join("\n");
    if text.0 != t {
        text.0 = t;
    }
}

/// With a focus in hand: its spells, which button casts each, what they
/// cost.
/// A spell in a line: the button, its name, element and mana.
fn spell_line(book: &Spellbook, i: usize, button: &str) -> String {
    let s = &book.spells[i];
    let (_, mana) = book.describe(&s.runes);
    let of = s.element.map_or("arcane".to_string(), |e| format!("{e:?}").to_lowercase());
    format!("{button}: {} ({of}, {mana:.0} mana)", s.name)
}

/// The open chest's (or body's) name over it.
fn title(chests: Res<Chests>, mut t: Single<&mut Text, With<ChestTitle>>) {
    let want = format!("{}  |  Shift-click: to the pack  |  R: take all", if chests.open_name.is_empty() { "Chest" } else { &chests.open_name });
    if t.0 != want {
        t.0 = want;
    }
}

/// Over a slot with something in it: what it is and what it does.
#[allow(clippy::too_many_arguments)]
fn tooltip(
    items: Option<Res<Items>>,
    rules: Res<GearRules>,
    book: Option<Res<Spellbook>>,
    weapons: Option<Res<crate::combat::Weapons>>,
    hand: Res<Hand>,
    held: Res<Held>,
    sim: Res<SimWorld>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut chests: ResMut<Chests>,
    slots: Query<SlotQuery>,
    inv: Query<(&Inventory, &Equipment), With<LocalPlayer>>,
    mut tip: Single<(&mut Text, &mut TextColor, &mut BorderColor, &mut Node, &mut Visibility), With<Tooltip>>,
    mut body: Single<&mut TextSpan, With<TooltipBody>>,
) {
    let (text, color, border, node, vis) = &mut *tip;
    **vis = Visibility::Hidden;
    let (Some(items), Ok((inv, eq)), Some(at)) = (items, inv.single(), window.cursor_position()) else { return };
    if held.stack.is_some() {
        return;
    }
    let Some((which, i)) = under(&slots) else { return };
    let stack = match which {
        Holder::Chest => chests.open.and_then(|c| chests.contents(c, &sim.world, &items).slots[i]),
        Holder::Equip => eq.worn[i],
        Holder::Trash => held.trash,
        _ => inv.slots[index(&hand, which, i)],
    };
    let Some(stack) = stack else { return };
    // (Worn, it's not compared with itself.)
    let against = (which != Holder::Equip).then_some(eq);
    let words = describe(&items, book.as_deref(), weapons.as_deref(), Some((&rules, against)), &stack);
    let (title, rest) = words.split_once('\n').unwrap_or((&words, ""));
    text.0 = title.to_string();
    body.0 = format!("\n{rest}");
    let rarity = rules.rarity(&items, &stack).map_or(Color::WHITE, |r| Color::srgb_u8(r.color.0, r.color.1, r.color.2));
    color.0 = rarity;
    **border = BorderColor::all(rarity.with_alpha(0.8));
    node.left = px(at.x + 18.0);
    // (Above the cursor near the bottom, where the hotbars are.)
    node.top = px(if at.y > window.height() * 0.5 { at.y - 110.0 } else { at.y + 18.0 });
    **vis = Visibility::Visible;
}

/// A stack, in words: its name and count, what it does, its note.
fn describe(items: &Items, book: Option<&Spellbook>, weapons: Option<&crate::combat::Weapons>, gear: Option<(&GearRules, Option<&Equipment>)>, s: &Stack) -> String {
    let def: &ItemDef = items.def(s.item);
    let n = whole(items, s);
    let name = gear.map_or(def.name.clone(), |(rules, _)| rules.name(items, s));
    let mut lines = vec![if n > 1 { format!("{name} ({n})") } else { name }];
    if let (Some(g), Some((rules, worn))) = (&def.gear, gear) {
        let weight = g.weight.map_or(String::new(), |w| format!(", {w:?} armour"));
        let rarity = rules.rarity(items, s).map_or("", |r| r.name.as_str());
        let level = if s.roll.level > 0 { format!("  (item level {})", s.roll.level) } else { String::new() };
        lines.push(format!("{rarity} {}{weight}{level}", g.slot.label().to_lowercase()));
        let mine = crate::gear::piece_stats(items, rules, s);
        lines.extend(mine.nonzero().map(|(st, v)| crate::gear::stats::line(st, v)));
        // Against what's worn in its place (the first of its kind).
        if let Some(eq) = worn
            && let Some(j) = WORN.iter().position(|w| *w == g.slot)
        {
            let theirs = eq.worn[j].map(|w| crate::gear::piece_stats(items, rules, &w)).unwrap_or_default();
            let mut diff = mine.clone();
            for (st, v) in theirs.nonzero() {
                diff.add(st, -v);
            }
            let changes: Vec<String> = diff.nonzero().map(|(st, v)| crate::gear::stats::line(st, v)).collect();
            if !changes.is_empty() {
                let what = if eq.worn[j].is_some() { "Instead of what you wear" } else { "Put on" };
                lines.push(format!("{what}: {}", changes.join(", ")));
            }
        }
    }
    match &def.use_ {
        Use::Mine { back: false, power, tier, speed, reach, area } => {
            lines.push(format!("Pickaxe: mines up to hardness {tier}"));
            lines.push(format!("{power} a hit, {speed} hits a second, reach {reach} blocks"));
            if *area > 0.0 {
                lines.push(format!("C: a block at a time, or a round bite {:.0} cells across", area * 2.0));
            }
        }
        Use::Mine { back: true, power, tier, speed, reach, .. } => {
            lines.push(format!("Axe: trees and walls, up to hardness {tier}"));
            lines.push(format!("{power} a hit, {speed} hits a second, reach {reach} blocks"));
        }
        Use::Focus { tier, element, spells } => {
            let what = if *tier >= 2 { "Staff" } else { "Wand" };
            let of = element.map_or(String::new(), |e| format!(" of {}", format!("{e:?}").to_lowercase()));
            lines.push(format!("{what}{of}: hold a button to cast at the cursor"));
            if let Some(book) = book {
                for (k, i) in crate::magic::spells::focus_spells(&book.spells, *tier, *element, spells, &s.roll).into_iter().enumerate() {
                    lines.push(spell_line(book, i, if k == 0 { "LMB" } else { "RMB" }));
                    if let Some(about) = &book.spells[i].about {
                        lines.push(format!("  {about}"));
                    }
                }
            }
        }
        Use::Station(_) => lines.push("Crafting station: click to set it down; a pickaxe takes it back".into()),
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
        Use::Vessel { holds, hot, acidproof } => {
            let takes = match (hot, acidproof) {
                (true, _) => "any liquid, molten metal too",
                (false, true) => "any cool liquid, acid too",
                _ => "a cool liquid (not acid)",
            };
            lines.push(format!("Holds {holds} cells of {takes}"));
            lines.push("RMB: fill it (at a furnace: melt bars and ore from your pack into it); LMB: pour".into());
            match s.fill.and_then(|(m, n)| items.material_name(m).map(|name| (name.replace('_', " "), n))) {
                Some((name, n)) => lines.push(format!("{n} cells of {name} in it")),
                None => lines.push("Empty".into()),
            }
        }
        Use::Potion { heal, over, sickness } => {
            let when = if *over > 0.0 { format!(" over {over:.0} s") } else { String::new() };
            lines.push(format!("Heals {heal:.0}{when}; click (or H) to drink"));
            lines.push(format!("Potion sickness {sickness:.0} s after"));
        }
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
