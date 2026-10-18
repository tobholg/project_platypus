//! The crafting panel (`craft.rs` has the recipes): opened from the
//! inventory's Crafting button, and shut again by it, its X, Esc (before
//! the inventory) or the inventory shutting. Beside the pack:
//!
//! - tabs down its left, one a sort of thing made (`Category`, from the
//!   item it makes: tools, weapons, armour...), each with how many of its
//!   recipes you can see, All first;
//! - the recipes you can see (`Crafting::listed`: held once, or unlocked by
//!   a milestone), a slot each: bright if it can be made here and now,
//!   dimmed if you're short of something or no station it needs is near;
//! - beside them, the one pointed at last: what it makes and does, what
//!   goes in (what you have of each, of what it takes) and where it's made.
//!
//! Click a recipe (or Craft) to make one.

use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use super::icons::Icons;
use super::items::{Category, Inventory, ItemId, Items, Stack};
use super::ui::{BUTTON, BUTTON_HOVER, CHOSEN, EDGE, EMPTY, InventoryOpen, PACK_RIGHT, PACK_TOP, ROUND, describe, shadow};
use crate::craft::{CraftRequest, Crafting, Station};
use crate::creatures::Kinematics;
use crate::creatures::player::LocalPlayer;
use crate::gear::{Equipment, GearRules};
use crate::progress::Progress;

pub struct CraftingPlugin;

/// The panel: up or not, its tab (none: All), the recipe pointed at last,
/// and what it lists (each recipe you can see, whether it can be made now)
/// with the stations near enough.
#[derive(Resource, Default)]
pub struct CraftingPanel {
    pub open: bool,
    pub tab: Option<Category>,
    pub picked: Option<usize>,
    pub(crate) list: Vec<(usize, bool)>,
    near: Vec<usize>,
}

impl CraftingPanel {
    /// What the tab shows of the list.
    pub(crate) fn in_tab(&self, crafting: &Crafting, items: &Items) -> Vec<(usize, bool)> {
        self.list.iter().copied().filter(|&(i, _)| self.tab.is_none_or(|t| made(crafting, items, i).is_some_and(|m| items.category(m) == t))).collect()
    }
}

/// What a recipe makes.
fn made(crafting: &Crafting, items: &Items, i: usize) -> Option<ItemId> {
    crafting.recipes.get(i).and_then(|r| items.id(&r.makes))
}

#[derive(Component)]
struct PanelRoot;

/// A tab (none: All), and its count.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Tab(pub(crate) Option<Category>);

#[derive(Component)]
struct TabCount(Option<Category>);

/// The recipes' grid, and the window it scrolls in.
#[derive(Component)]
struct Grid;

#[derive(Component)]
struct GridView;

/// A recipe's slot.
#[derive(Component)]
pub(crate) struct RecipeSlot(pub(crate) usize);

/// The picked recipe's side.
#[derive(Component)]
struct Details;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanelButton {
    Craft,
    Close,
}

/// Which stations are near, in the header.
#[derive(Component)]
struct NearText;

/// A recipe's slot, and its icon.
const CELL: f32 = 52.0;
const CELL_ICON: f32 = 36.0;
const GAP: f32 = 6.0;
const COLUMNS: u16 = 7;
const ROWS: f32 = 8.0;
const TABS_WIDTH: f32 = 140.0;
const DETAILS_WIDTH: f32 = 260.0;
/// The panel: deep blue-black, a soft blue rim; the side it details on a
/// shade lighter.
const PANEL: Color = Color::srgb(0.05, 0.06, 0.13);
const RIM: Color = Color::srgb(0.3, 0.35, 0.62);
const SIDE: Color = Color::srgb(0.1, 0.12, 0.24);
/// A recipe you can't make yet: a darker slot, its icon faded.
const SHORT: Color = Color::srgb(0.11, 0.12, 0.24);
const FADED: f32 = 0.32;
/// A recipe you can make: a green rim.
const READY: Color = Color::srgb(0.45, 0.82, 0.45);
const TITLE: Color = Color::srgb(0.98, 0.86, 0.55);
const TEXT: Color = Color::srgb(0.92, 0.92, 0.96);
const DIM: Color = Color::srgb(0.62, 0.63, 0.7);
const ENOUGH: Color = Color::srgb(0.55, 0.9, 0.5);
const SHORT_TEXT: Color = Color::srgb(1.0, 0.48, 0.42);

impl Plugin for CraftingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CraftingPanel>().add_systems(Startup, spawn).add_systems(Update, (clicks, list, grid, tabs, details, scroll).chain());
    }
}

fn text(s: impl Into<String>, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (Text::new(s), TextFont { font_size: FontSize::Px(size), ..default() }, TextColor(color))
}

fn spawn(mut commands: Commands) {
    commands
        .spawn((
            PanelRoot,
            Interaction::None,
            Visibility::Hidden,
            GlobalZIndex(5),
            Node {
                position_type: PositionType::Absolute,
                top: px(PACK_TOP),
                left: px(PACK_RIGHT),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                padding: UiRect::all(px(12)),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(RIM),
        ))
        .with_children(|p| {
            // The title, what's near, the X.
            p.spawn(Node { align_items: AlignItems::Center, column_gap: px(14), ..default() }).with_children(|top| {
                top.spawn((text("Crafting", 22.0, TITLE), shadow()));
                top.spawn((NearText, text("", 13.0, DIM), Node { flex_grow: 1.0, ..default() }));
                top.spawn((
                    Button,
                    PanelButton::Close,
                    Node { width: px(28), height: px(28), border: UiRect::all(px(2)), border_radius: BorderRadius::all(px(ROUND)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                    BorderColor::all(EDGE),
                    BackgroundColor(BUTTON),
                ))
                .with_child(text("X", 15.0, TEXT));
            });
            p.spawn(Node { column_gap: px(12), align_items: AlignItems::Stretch, ..default() }).with_children(|body| {
                // The tabs (those with no recipes at all hidden: `tabs`).
                body.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(3), width: px(TABS_WIDTH), ..default() }).with_children(|t| {
                    for c in std::iter::once(None).chain(Category::ALL.into_iter().map(Some)) {
                        t.spawn((
                            Button,
                            Tab(c),
                            Node { padding: UiRect::axes(px(10), px(5)), border: UiRect::all(px(2)), border_radius: BorderRadius::all(px(6)), justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Center, ..default() },
                            BorderColor::all(Color::NONE),
                            BackgroundColor(Color::NONE),
                        ))
                        .with_children(|b| {
                            b.spawn(text(c.map_or("All", |c| c.label()), 14.0, TEXT));
                            b.spawn((TabCount(c), text("", 12.0, DIM)));
                        });
                    }
                });
                // The recipes, in a window that scrolls if they're many.
                let width = COLUMNS as f32 * CELL + (COLUMNS - 1) as f32 * GAP;
                body.spawn((
                    GridView,
                    RelativeCursorPosition::default(),
                    ScrollPosition::default(),
                    Node { width: px(width + 4.0), height: px(ROWS * (CELL + GAP)), overflow: Overflow::scroll_y(), padding: UiRect::all(px(2)), ..default() },
                ))
                .with_child((
                    Grid,
                    Node { display: Display::Grid, grid_template_columns: RepeatedGridTrack::px(COLUMNS, CELL), grid_auto_rows: vec![GridTrack::px(CELL)], column_gap: px(GAP), row_gap: px(GAP), align_content: AlignContent::Start, ..default() },
                ));
                body.spawn((
                    Details,
                    Node { flex_direction: FlexDirection::Column, row_gap: px(6), width: px(DETAILS_WIDTH), padding: UiRect::all(px(10)), border_radius: BorderRadius::all(px(8)), ..default() },
                    BackgroundColor(SIDE),
                ));
            });
            p.spawn(text("Click a recipe to make one   |   bright: it can be made now   |   dimmed: short of something, or its station isn't near", 12.0, DIM));
        });
}

/// The X, Craft, a tab, a recipe: clicked. A recipe pointed at is picked
/// (what the side shows); clicked, one's made.
#[allow(clippy::type_complexity)]
fn clicks(
    open: Res<InventoryOpen>,
    mut panel: ResMut<CraftingPanel>,
    buttons: Query<(&Interaction, &PanelButton), Changed<Interaction>>,
    tabs: Query<(&Interaction, &Tab), Changed<Interaction>>,
    slots: Query<(&Interaction, &RecipeSlot), Changed<Interaction>>,
    mut view: Query<&mut ScrollPosition, With<GridView>>,
    mut asks: MessageWriter<CraftRequest>,
) {
    if !(open.0 && panel.open) {
        return;
    }
    for (i, b) in &buttons {
        if *i != Interaction::Pressed {
            continue;
        }
        match b {
            PanelButton::Close => panel.open = false,
            PanelButton::Craft => {
                if let Some(r) = panel.picked {
                    asks.write(CraftRequest(r));
                }
            }
        }
    }
    for (i, t) in &tabs {
        if *i == Interaction::Pressed && panel.tab != t.0 {
            panel.tab = t.0;
            for mut s in &mut view {
                s.0 = Vec2::ZERO;
            }
        }
    }
    for (i, s) in &slots {
        match i {
            Interaction::Pressed => {
                panel.picked = Some(s.0);
                asks.write(CraftRequest(s.0));
            }
            Interaction::Hovered => panel.picked = Some(s.0),
            Interaction::None => {}
        }
    }
}

/// What it lists: worked out while it's up (what's carried and what's near
/// change under it); and up only with the inventory.
fn list(
    open: Res<InventoryOpen>,
    crafting: Res<Crafting>,
    items: Option<Res<Items>>,
    mut panel: ResMut<CraftingPanel>,
    player: Query<(&Kinematics, &Inventory, &Progress), With<LocalPlayer>>,
    stations: Query<(&Station, &Kinematics)>,
    mut root: Single<&mut Visibility, With<PanelRoot>>,
    mut near_text: Single<&mut Text, With<NearText>>,
) {
    let up = open.0 && panel.open;
    let want = if up { Visibility::Visible } else { Visibility::Hidden };
    if **root != want {
        **root = want;
    }
    let (Some(items), Ok((k, inv, progress)), true) = (items, player.single(), up) else { return };
    let near = crate::craft::near(&crafting, k.body.pos, &stations);
    let list = crafting.listed(&items, inv, progress, &near);
    if panel.list != list {
        panel.list = list;
    }
    let mut names: Vec<&str> = near.iter().map(|&s| crafting.stations[s].name.as_str()).collect();
    names.sort();
    names.dedup();
    let want = if names.is_empty() { "No station near: what's made by hand".to_string() } else { format!("Near: {}", names.join(", ")) };
    if near_text.0 != want {
        near_text.0 = want;
    }
    if panel.near != near {
        panel.near = near;
    }
}

/// The tab's recipes, rebuilt when what it shows changes; each slot's rim:
/// gold picked, white under the mouse, green if it can be made.
#[allow(clippy::too_many_arguments)]
fn grid(
    mut commands: Commands,
    crafting: Res<Crafting>,
    items: Option<Res<Items>>,
    icons: Option<Res<Icons>>,
    panel: Res<CraftingPanel>,
    grid: Single<Entity, With<Grid>>,
    mut slots: Query<(&RecipeSlot, &Interaction, &mut BorderColor)>,
    mut last: Local<(Option<Category>, Vec<(usize, bool)>, bool)>,
) {
    let Some(items) = items else { return };
    if !panel.open {
        return;
    }
    let shown = panel.in_tab(&crafting, &items);
    let have_icons = icons.is_some();
    if last.0 != panel.tab || last.1 != shown || last.2 != have_icons || crafting.is_changed() {
        *last = (panel.tab, shown.clone(), have_icons);
        commands.entity(*grid).despawn_related::<Children>();
        commands.entity(*grid).with_children(|g| {
            if shown.is_empty() {
                let what = if panel.list.is_empty() { "Nothing to make yet: find wood, stone, ore".to_string() } else { format!("Nothing in {} yet", panel.tab.map_or("All", |t| t.label())) };
                g.spawn((text(what, 13.0, DIM), Node { grid_column: GridPlacement::span(COLUMNS), ..default() }));
            }
            for &(i, can) in &shown {
                let Some(item) = made(&crafting, &items, i) else { continue };
                let r = &crafting.recipes[i];
                g.spawn((
                    Button,
                    RecipeSlot(i),
                    Node { width: px(CELL), height: px(CELL), border: UiRect::all(px(2)), border_radius: BorderRadius::all(px(ROUND)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                    BorderColor::all(EDGE),
                    BackgroundColor(if can { EMPTY } else { SHORT }),
                ))
                .with_children(|s| {
                    icon(s, &items, icons.as_deref(), item, CELL_ICON, if can { 1.0 } else { FADED });
                    if r.count > 1 {
                        s.spawn((text(r.count.to_string(), 12.0, Color::WHITE), shadow(), Node { position_type: PositionType::Absolute, right: px(4), bottom: px(1), ..default() }));
                    }
                });
            }
        });
    }
    let can = |i: usize| shown.iter().any(|&(j, c)| j == i && c);
    for (s, i, mut rim) in &mut slots {
        let want = if panel.picked == Some(s.0) {
            CHOSEN
        } else if *i != Interaction::None {
            Color::WHITE
        } else if can(s.0) {
            READY
        } else {
            EDGE
        };
        *rim = BorderColor::all(want);
    }
}

/// An item's icon, or a square of its colour; `alpha` fades it.
fn icon(parent: &mut ChildSpawnerCommands, items: &Items, icons: Option<&Icons>, item: ItemId, size: f32, alpha: f32) {
    let node = Node { width: px(size), height: px(size), ..default() };
    match icons.and_then(|ic| ic.get(item)) {
        Some(h) => {
            parent.spawn((ImageNode::new(h.clone()).with_color(Color::srgba(1.0, 1.0, 1.0, alpha)), node));
        }
        None => {
            let (r, g, b) = items.def(item).color;
            parent.spawn((node, BackgroundColor(Color::srgba_u8(r, g, b, (alpha * 255.0) as u8))));
        }
    }
}

/// The tabs: how many recipes each has that you can see (dimmed, none;
/// hidden, if no recipe at all makes its sort of thing); the one picked
/// gold-edged.
fn tabs(
    crafting: Res<Crafting>,
    items: Option<Res<Items>>,
    panel: Res<CraftingPanel>,
    mut tabs: Query<(&Tab, &Interaction, &mut Node, &mut BackgroundColor, &mut BorderColor, &Children)>,
    mut counts: Query<(&TabCount, &mut Text)>,
    mut labels: Query<&mut TextColor, Without<TabCount>>,
    mut buttons: Query<(&PanelButton, &Interaction, &mut BackgroundColor, &mut BorderColor), Without<Tab>>,
) {
    let Some(items) = items else { return };
    if !panel.open {
        return;
    }
    let sort = |i: usize| made(&crafting, &items, i).map(|m| items.category(m));
    let seen = |c: Option<Category>| panel.list.iter().filter(|&&(i, _)| c.is_none() || sort(i) == c).count();
    for (t, i, mut node, mut bg, mut rim, children) in &mut tabs {
        let any = t.0.is_none() || (0..crafting.recipes.len()).any(|i| sort(i) == t.0);
        let display = if any { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
        let picked = panel.tab == t.0;
        bg.0 = if picked {
            Color::srgba(0.95, 0.76, 0.2, 0.22)
        } else if *i != Interaction::None {
            BUTTON_HOVER
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.04)
        };
        *rim = BorderColor::all(if picked { CHOSEN } else { Color::NONE });
        let lit = seen(t.0) > 0 || picked;
        for c in children.iter() {
            if let Ok(mut color) = labels.get_mut(c) {
                color.0 = if lit { TEXT } else { DIM.with_alpha(0.6) };
            }
        }
    }
    // (The X: lighter under the mouse; Craft: a white rim.)
    let can = panel.picked.is_some_and(|p| panel.list.iter().any(|&(j, c)| j == p && c));
    for (b, i, mut bg, mut rim) in &mut buttons {
        match b {
            PanelButton::Close => bg.0 = if *i == Interaction::None { BUTTON } else { BUTTON_HOVER },
            PanelButton::Craft => *rim = BorderColor::all(if *i != Interaction::None { Color::WHITE } else if can { READY } else { EDGE }),
        }
    }
    for (c, mut text) in &mut counts {
        let n = seen(c.0);
        let want = if n > 0 { n.to_string() } else { String::new() };
        if text.0 != want {
            text.0 = want;
        }
    }
}

/// The picked recipe: its item big, named, what it is and does; what goes
/// in (what you have of what it takes, green enough, red short); where it's
/// made; Craft. Rebuilt when any of that changes.
#[allow(clippy::too_many_arguments)]
fn details(
    mut commands: Commands,
    crafting: Res<Crafting>,
    items: Option<Res<Items>>,
    icons: Option<Res<Icons>>,
    rules: Res<GearRules>,
    book: Option<Res<crate::magic::Spellbook>>,
    weapons: Option<Res<crate::combat::Weapons>>,
    panel: Res<CraftingPanel>,
    player: Query<(&Inventory, &Equipment), With<LocalPlayer>>,
    side: Single<Entity, With<Details>>,
    mut last: Local<Option<(Option<usize>, Vec<u32>, bool, bool)>>,
) {
    let (Some(items), Ok((inv, eq))) = (items, player.single()) else { return };
    if !panel.open {
        return;
    }
    let picked = panel.picked.filter(|i| panel.list.iter().any(|(j, _)| j == i));
    let r = picked.and_then(|i| crafting.recipes.get(i));
    let can = picked.is_some_and(|i| panel.list.iter().any(|&(j, c)| j == i && c));
    let at = r.and_then(|r| r.at.as_ref()).and_then(|a| crafting.station(a));
    let here = at.is_none_or(|s| panel.near.contains(&s));
    let have: Vec<u32> = r.map_or(Vec::new(), |r| r.needs.iter().map(|(id, _)| items.id(id).map_or(0, |it| inv.count(it) / items.unit(it))).collect());
    let now = (picked, have.clone(), can, here);
    if last.as_ref() == Some(&now) && !crafting.is_changed() && !icons.as_ref().is_some_and(|i| i.is_changed()) {
        return;
    }
    *last = Some(now);
    commands.entity(*side).despawn_related::<Children>();
    let (Some(r), Some(item)) = (r, picked.and_then(|i| made(&crafting, &items, i))) else {
        commands.entity(*side).with_child(text("Point at a recipe: what it makes, what goes in, where it's made", 13.0, DIM));
        return;
    };
    let def = items.def(item);
    let stack = Stack::new(item, r.count * items.unit(item));
    let words = describe(&items, book.as_deref(), weapons.as_deref(), def.gear.as_ref().map(|_| (&*rules, Some(eq))), &stack);
    let about: Vec<&str> = words.lines().skip(1).collect();
    commands.entity(*side).with_children(|d| {
        // The item, big, beside its name and sort.
        d.spawn(Node { column_gap: px(10), align_items: AlignItems::Center, ..default() }).with_children(|top| {
            top.spawn((
                Node { width: px(64), height: px(64), border: UiRect::all(px(2)), border_radius: BorderRadius::all(px(ROUND)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                BorderColor::all(if can { READY } else { EDGE }),
                BackgroundColor(EMPTY),
            ))
            .with_children(|s| icon(s, &items, icons.as_deref(), item, 48.0, 1.0));
            top.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(2), flex_shrink: 1.0, ..default() }).with_children(|t| {
                t.spawn(text(def.name.clone(), 18.0, Color::WHITE));
                let makes = if r.count > 1 { format!("{}, makes {}", items.category(item).label(), r.count) } else { items.category(item).label().to_string() };
                t.spawn(text(makes, 12.0, DIM));
            });
        });
        d.spawn((text("Needs", 13.0, TITLE), Node { margin: UiRect::top(px(4)), ..default() }));
        for ((id, n), got) in r.needs.iter().zip(&have) {
            let Some(it) = items.id(id) else { continue };
            d.spawn(Node { column_gap: px(8), align_items: AlignItems::Center, ..default() }).with_children(|row| {
                icon(row, &items, icons.as_deref(), it, 24.0, 1.0);
                row.spawn((text(items.def(it).name.clone(), 14.0, TEXT), Node { flex_grow: 1.0, ..default() }));
                row.spawn(text(format!("{got} / {n}"), 14.0, if got >= n { ENOUGH } else { SHORT_TEXT }));
            });
        }
        let (place, color) = match at {
            None => ("Made by hand".to_string(), DIM),
            Some(s) if here => (format!("Made at the {} (one's near)", crafting.stations[s].name.to_lowercase()), ENOUGH),
            Some(s) => (format!("Made at the {} (none near)", crafting.stations[s].name.to_lowercase()), SHORT_TEXT),
        };
        d.spawn((text(place, 13.0, color), Node { margin: UiRect::vertical(px(4)), ..default() }));
        if !about.is_empty() {
            d.spawn(text(about.join("\n"), 12.0, Color::srgb(0.8, 0.8, 0.86)));
        }
        d.spawn(Node { flex_grow: 1.0, ..default() });
        d.spawn((
            Button,
            PanelButton::Craft,
            Node { padding: UiRect::axes(px(16), px(8)), border: UiRect::all(px(2)), border_radius: BorderRadius::all(px(ROUND)), justify_content: JustifyContent::Center, ..default() },
            BorderColor::all(if can { READY } else { EDGE }),
            BackgroundColor(if can { Color::srgba(0.2, 0.42, 0.22, 0.95) } else { SHORT }),
        ))
        .with_child(text(if can { "Craft" } else { "Can't make it yet" }, 16.0, if can { Color::WHITE } else { DIM }));
    });
}

/// The wheel over the recipes scrolls them (when they're more than fit).
fn scroll(wheel: Res<AccumulatedMouseScroll>, panel: Res<CraftingPanel>, mut view: Query<(&RelativeCursorPosition, &mut ScrollPosition), With<GridView>>) {
    if !panel.open || wheel.delta.y == 0.0 {
        return;
    }
    let dy = match wheel.unit {
        MouseScrollUnit::Line => wheel.delta.y * (CELL + GAP),
        MouseScrollUnit::Pixel => wheel.delta.y,
    };
    for (over, mut s) in &mut view {
        if over.cursor_over() {
            s.0.y = (s.0.y - dy).max(0.0);
        }
    }
}
