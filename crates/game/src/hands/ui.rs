//! The hotbar (always, in play mode), the rest of the pack (I) and an open
//! chest. Click a slot to pick its stack up and click another to put it down
//! (swapping, or merging the same item); Shift-click moves a stack between the
//! hotbar and the pack, or with a chest open between the chest and the pack.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::chests::{Chests, SLOTS};
use super::items::{HOTBAR, Inventory, Items, Stack};
use super::{DevTools, Hand, PACK};
use crate::actors::player::LocalPlayer;
use crate::world::SimWorld;

pub struct UiPlugin;

/// The pack is open: the mouse works the inventory, not the world.
#[derive(Resource, Default)]
pub struct InventoryOpen(pub bool);

/// A stack picked up with the mouse, on its way to another slot.
#[derive(Resource, Default)]
struct Held(Option<Stack>);

/// Whose slot: the player's pack or the open chest.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Holder {
    Pack,
    Chest,
}

#[derive(Component)]
struct SlotUi(Holder, usize);

#[derive(Component)]
struct SlotIcon(Holder, usize);

#[derive(Component)]
struct SlotCount(Holder, usize);

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

const SLOT: f32 = 34.0;
const EMPTY: Color = Color::srgba(0.08, 0.08, 0.1, 0.72);
const EDGE: Color = Color::srgba(0.5, 0.5, 0.55, 0.8);
const CHOSEN: Color = Color::srgb(1.0, 0.85, 0.3);

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryOpen>()
            .init_resource::<Held>()
            .add_systems(Startup, spawn)
            .add_systems(Update, (toggle, click, show).chain());
    }
}

fn slot(parent: &mut ChildSpawnerCommands, which: Holder, i: usize) {
    parent
        .spawn((
            Button,
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
            s.spawn((SlotIcon(which, i), Node { width: px(16), height: px(16), ..default() }, BackgroundColor(Color::NONE)));
            s.spawn((
                SlotCount(which, i),
                Text::new(""),
                TextFont { font_size: FontSize::Px(11.0), ..default() },
                TextColor(Color::WHITE),
                Node { position_type: PositionType::Absolute, right: px(2), bottom: px(0), ..default() },
            ));
        });
}

fn spawn(mut commands: Commands) {
    // Hotbar and its label, bottom centre.
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
                    slot(row, Holder::Pack, i);
                }
            });
        });
    // The rest of the pack, above the hotbar, when open.
    commands
        .spawn((
            PackRoot,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(SLOT + 40.0),
                width: percent(100),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Node { display: Display::Grid, grid_template_columns: RepeatedGridTrack::px(HOTBAR as u16, SLOT), column_gap: px(3), row_gap: px(3), padding: UiRect::all(px(6)), ..default() },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            ))
            .with_children(|grid| {
                for i in HOTBAR..PACK {
                    slot(grid, Holder::Pack, i);
                }
            });
        });
    // An open chest, above the pack.
    commands
        .spawn((
            ChestRoot,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(SLOT * 4.0 + 72.0),
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Chest  (Shift-click: move · R: take all)"),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(Color::WHITE),
            ));
            root.spawn((
                Node { display: Display::Grid, grid_template_columns: RepeatedGridTrack::px(HOTBAR as u16, SLOT), column_gap: px(3), row_gap: px(3), padding: UiRect::all(px(6)), ..default() },
                BackgroundColor(Color::srgba(0.12, 0.08, 0.04, 0.6)),
            ))
            .with_children(|grid| {
                for i in 0..SLOTS {
                    slot(grid, Holder::Chest, i);
                }
            });
        });
    // The stack in the mouse's grip.
    commands.spawn((
        HeldIcon,
        Visibility::Hidden,
        Node { position_type: PositionType::Absolute, width: px(14), height: px(14), ..default() },
        BackgroundColor(Color::NONE),
        GlobalZIndex(10),
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
    mut pack: Query<&mut Visibility, (With<PackRoot>, Without<ChestRoot>)>,
    mut chest_panel: Query<&mut Visibility, With<ChestRoot>>,
) {
    for mut v in &mut chest_panel {
        *v = if chests.open.is_some() { Visibility::Visible } else { Visibility::Hidden };
    }
    // A chest opened (or walked away from) moves the pack with it.
    let want = if keys.just_pressed(KeyCode::KeyI) || keys.just_pressed(KeyCode::Escape) {
        if open.0 { false } else { keys.just_pressed(KeyCode::KeyI) }
    } else {
        open.0
    };
    if want == open.0 && pack.iter().next().is_some_and(|v| (*v == Visibility::Visible) == open.0) {
        return;
    }
    open.0 = want;
    if !open.0 {
        chests.open = None;
    }
    // Closing with something in hand puts it back.
    if !open.0
        && let (Some(stack), Some(items), Ok(mut inv)) = (held.0.take(), items, inv.single_mut())
    {
        inv.add(&items, stack);
    }
    for mut v in &mut pack {
        *v = if open.0 { Visibility::Visible } else { Visibility::Hidden };
    }
}

#[allow(clippy::too_many_arguments)]
fn click(
    keys: Res<ButtonInput<KeyCode>>,
    items: Option<Res<Items>>,
    open: Res<InventoryOpen>,
    sim: Res<SimWorld>,
    mut chests: ResMut<Chests>,
    mut hand: ResMut<Hand>,
    mut held: ResMut<Held>,
    slots: Query<(&Interaction, &SlotUi), Changed<Interaction>>,
    mut inv: Query<&mut Inventory, With<LocalPlayer>>,
) {
    let (Some(items), Ok(mut inv)) = (items, inv.single_mut()) else { return };
    let chest = chests.open;
    for (interaction, &SlotUi(which, i)) in &slots {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if !open.0 {
            // Closed: clicking the hotbar just picks the slot.
            hand.slot = i;
            continue;
        }
        let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        match (which, chest) {
            (Holder::Chest, None) => continue,
            (Holder::Chest, Some(c)) => {
                let stash = chests.contents(c, &sim.world, &items);
                if shift {
                    // Into the pack.
                    if let Some(st) = stash.slots[i].take() {
                        let left = inv.add(&items, st);
                        stash.slots[i] = (left > 0).then_some(Stack { item: st.item, count: left });
                    }
                } else {
                    swap(&items, &mut held.0, &mut stash.slots[i]);
                }
            }
            (Holder::Pack, _) if shift => {
                let Some(stack) = inv.slots[i].take() else { continue };
                let left = if let Some(c) = chest {
                    chests.contents(c, &sim.world, &items).add(&items, stack)
                } else {
                    // Hotbar ↔ pack.
                    let range = if i < HOTBAR { HOTBAR..PACK } else { 0..HOTBAR };
                    let mut other = Inventory { slots: inv.slots[range.clone()].to_vec() };
                    let left = other.add(&items, stack);
                    inv.slots[range].copy_from_slice(&other.slots);
                    left
                };
                inv.slots[i] = (left > 0).then_some(Stack { item: stack.item, count: left });
            }
            (Holder::Pack, _) => swap(&items, &mut held.0, &mut inv.slots[i]),
        }
    }
}

/// Put the held stack in a slot: merge the same item as far as it goes,
/// otherwise swap.
fn swap(items: &Items, held: &mut Option<Stack>, slot: &mut Option<Stack>) {
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

#[allow(clippy::too_many_arguments)]
fn show(
    items: Option<Res<Items>>,
    dev: Res<DevTools>,
    hand: Res<Hand>,
    held: Res<Held>,
    window: Single<&Window, With<PrimaryWindow>>,
    inv: Query<&Inventory, With<LocalPlayer>>,
    sim: Res<SimWorld>,
    mut chests: ResMut<Chests>,
    mut roots: Query<&mut Visibility, (With<HotbarRoot>, Without<HeldIcon>)>,
    mut borders: Query<(&SlotUi, &mut BorderColor)>,
    mut icons: Query<(&SlotIcon, &mut BackgroundColor), Without<HeldIcon>>,
    mut counts: Query<(&SlotCount, &mut Text), Without<ItemLabel>>,
    mut label: Single<&mut Text, With<ItemLabel>>,
    mut grip: Single<(&mut Node, &mut BackgroundColor, &mut Visibility), With<HeldIcon>>,
) {
    for mut v in &mut roots {
        *v = if dev.0 { Visibility::Hidden } else { Visibility::Visible };
    }
    let (Some(items), Ok(inv)) = (items, inv.single()) else { return };
    let color = |s: &Stack| {
        let (r, g, b) = items.def(s.item).color;
        Color::srgb_u8(r, g, b)
    };
    let stash: Vec<Option<Stack>> = match chests.open {
        Some(c) => chests.contents(c, &sim.world, &items).slots.clone(),
        None => vec![None; SLOTS],
    };
    let slot_of = |which: Holder, i: usize| if which == Holder::Pack { inv.slots[i] } else { stash[i] };
    for (&SlotUi(which, i), mut border) in &mut borders {
        *border = BorderColor::all(if which == Holder::Pack && i == hand.slot { CHOSEN } else { EDGE });
    }
    for (&SlotIcon(which, i), mut bg) in &mut icons {
        bg.0 = slot_of(which, i).as_ref().map_or(Color::NONE, color);
    }
    for (&SlotCount(which, i), mut text) in &mut counts {
        let n = slot_of(which, i).map_or(0, |s| s.count / items.unit(s.item));
        text.0 = if n > 1 { n.to_string() } else { String::new() };
    }
    let name = inv.slots[hand.slot].map_or(String::new(), |s| {
        let unit = items.unit(s.item);
        let spare = s.count % unit;
        if unit > 1 && spare > 0 { format!("{} ({} + {spare}/{unit})", items.def(s.item).name, s.count / unit) } else { items.def(s.item).name.clone() }
    });
    let smart = if hand.smart { "smart cursor" } else { "plain cursor" };
    label.0 = format!("{name}   |   {smart} [Alt]  |  pack [I]  |  dev tools: key left of 1");
    let (node, bg, vis) = &mut *grip;
    match (held.0, window.cursor_position()) {
        (Some(s), Some(at)) => {
            **vis = Visibility::Visible;
            bg.0 = color(&s);
            node.left = px(at.x + 8.0);
            node.top = px(at.y + 8.0);
        }
        _ => **vis = Visibility::Hidden,
    }
}
