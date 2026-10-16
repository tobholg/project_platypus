//! The dev menu (dev mode only: `mode.rs`). The key left of 1 (or F1) opens
//! it, again (or Esc) shuts it. One panel on the left, tabs along its top:
//!
//! - **Player**: god mode (always, while the menu's open), endless mana and stamina, heal, die, to the
//!   start, teleport (to where you next click in the world), the light
//!   carried, a torch, the checkpoint (`respawn.rs`).
//! - **Items**: the kits (`items.ron`), and every item by kind or by name
//!   (click the search, type): click one into the pack, Shift-click for a
//!   stack, right-click to hold it (the hotbar slot in hand).
//! - **Creatures**: packs, then kinds, or by name: pick one and each click in
//!   the world places it (right-click stops; O spawns it at the cursor too),
//!   one in front of you, clearing the floor, the bestiary, the show.
//! - **World**: the time of day, weather, events, the arena's floors, resets.
//! - **Tools**: the dev toolbelt in place of the hands (`tools.rs`), its
//!   radius, and what the spawner pours.
//! - **View**: lighting, the performance HUD, overlays, marks, the art editor.
//! - **Time**: pause, a tick at a time, slow motion, the tempo, the fight
//!   readout and its replays (`fight.rs`, `replay.rs`).
//! - **Sounds**: the sound board.
//!
//! The tab open is remembered (`saves/dev_menu.txt`). It's no taller than
//! the window leaves (the wheel scrolls it), and steps away while the
//! inventory's open. A button says a `DevAction` or an `ArenaAction` (the
//! keys say the same: `dev.rs`, `arena.rs`) or something of the menu's own.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use platypus_sim::{Kind, MaterialId};

use crate::arena::{ArenaAction, ArenaView, CurrentLayout};
use crate::camera::CursorWorld;
use crate::creatures::player::LocalPlayer;
use crate::creatures::spawn::SpawnKind;
use crate::creatures::{Health, Kinematics};
use crate::dev::{DevAction, KeyboardTaken, PointerOverUi};
use crate::hands::icons::Icons;
use crate::hands::items::{Inventory, ItemDef, ItemId, Items, Stack, Use};
use crate::hands::{DevTools, Hand};
use crate::mode::Mode;
use crate::tools::{Tool, Toolbelt};
use crate::world::SimWorld;

pub struct DevMenuPlugin;

/// Where the panel's top is: under the hotbar.
const TOP: f32 = 96.0;
/// Its width.
const WIDE: f32 = 300.0;
/// The speeds slow motion goes through.
const SPEEDS: [(f32, &str); 4] = [(1.0, "1x"), (0.5, "1/2"), (0.25, "1/4"), (0.1, "1/10")];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Player,
    Items,
    Creatures,
    World,
    Tools,
    View,
    Time,
    Sounds,
}

impl Tab {
    const ALL: [Tab; 8] = [Tab::Player, Tab::Items, Tab::Creatures, Tab::World, Tab::Tools, Tab::View, Tab::Time, Tab::Sounds];

    fn name(self) -> &'static str {
        match self {
            Tab::Player => "Player",
            Tab::Items => "Items",
            Tab::Creatures => "Creatures",
            Tab::World => "World",
            Tab::Tools => "Tools",
            Tab::View => "View",
            Tab::Time => "Time",
            Tab::Sounds => "Sounds",
        }
    }
}

/// The items' kinds (the Items tab's filter).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cat {
    Tools,
    Weapons,
    Magic,
    Gear,
    Potions,
    Furniture,
    Blocks,
    Materials,
}

impl Cat {
    const ALL: [Cat; 8] = [Cat::Tools, Cat::Weapons, Cat::Magic, Cat::Gear, Cat::Potions, Cat::Furniture, Cat::Blocks, Cat::Materials];

    fn name(self) -> &'static str {
        match self {
            Cat::Tools => "tools",
            Cat::Weapons => "weapons",
            Cat::Magic => "magic",
            Cat::Gear => "armour & trinkets",
            Cat::Potions => "potions",
            Cat::Furniture => "furniture",
            Cat::Blocks => "blocks",
            Cat::Materials => "ore, bars & loot",
        }
    }

    pub fn of(def: &ItemDef) -> Cat {
        match &def.use_ {
            Use::Melee(_) | Use::Bow(_) => Cat::Weapons,
            _ if def.id.contains("arrow") => Cat::Weapons,
            Use::Focus { .. } => Cat::Magic,
            _ if def.gear.is_some() => Cat::Gear,
            Use::Mine { .. } | Use::Throw(_) | Use::Torch | Use::Vessel { .. } => Cat::Tools,
            Use::Potion { .. } => Cat::Potions,
            Use::Chest | Use::Station(_) => Cat::Furniture,
            Use::Block(_) if def.id.starts_with("block:") && matches!(def.loot, platypus_sim::Loot::Block | platypus_sim::Loot::Wood) => Cat::Blocks,
            _ => Cat::Materials,
        }
    }
}

/// What the menu shows.
#[derive(Resource)]
pub struct DevMenu {
    pub open: bool,
    pub tab: Tab,
    cat: Option<Cat>,
    /// The search (items or creatures, as the tab is), and whether it's
    /// being typed into.
    search: String,
    typing: bool,
    /// The next click in the world teleports you there.
    teleport: bool,
    /// Placing what's picked in the Creatures tab: each click in the world
    /// puts one there (right-click stops).
    placing: bool,
    /// What the lists were last built from (rebuilt when it changes).
    built: String,
}

/// What the cheats have on.
#[derive(Resource, Default)]
pub struct Cheats {
    /// Nothing brings you below full health.
    pub god: bool,
    /// Mana and stamina always full.
    pub endless: bool,
}

/// A button's meaning.
#[derive(Component, Clone, Debug, PartialEq)]
pub enum Act {
    Dev(DevAction),
    Arena(ArenaAction),
    Tab(Tab),
    Cat(Option<Cat>),
    Search,
    Kit(String),
    Empty,
    God,
    Endless,
    Heal,
    Die,
    Teleport,
    Checkpoint(bool),
    SpawnHere,
    Hands,
    Tool(Tool),
    Material(MaterialId),
    Radius(i32),
    Hour(f32),
}

/// An item in the Items tab's grid.
#[derive(Component)]
pub struct ItemButton(pub ItemId);

#[derive(Component)]
struct Panel;
#[derive(Component)]
struct Title;
#[derive(Component)]
struct Body;
#[derive(Component)]
struct TabBody(Tab);
/// The search field's text, and the line saying what's under the pointer.
#[derive(Component)]
struct SearchText;
#[derive(Component)]
struct Caption;
/// The lists rebuilt as the search and filters change.
#[derive(Component)]
struct ItemGrid;
#[derive(Component)]
struct CreatureList;
#[derive(Component)]
struct RadiusText;

impl DevMenu {
    /// The search, as if typed (scenarios).
    pub fn set_search(&mut self, s: &str) {
        if self.search != s {
            self.search = s.to_string();
        }
    }
}

fn tab_path() -> std::path::PathBuf {
    crate::save::saves_dir().join("dev_menu.txt")
}

impl Default for DevMenu {
    fn default() -> Self {
        let saved = std::fs::read_to_string(tab_path()).unwrap_or_default();
        let tab = Tab::ALL.into_iter().find(|t| t.name() == saved.trim()).unwrap_or(Tab::Player);
        DevMenu { open: false, tab, cat: None, search: String::new(), typing: false, teleport: false, placing: false, built: String::new() }
    }
}

impl Plugin for DevMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DevMenu>()
            .init_resource::<Cheats>()
            .add_systems(Startup, spawn_panel.after(crate::fight::make_timeline))
            // (Before the UI reads the mouse: a teleport's click is the menu's.)
            .add_systems(PreUpdate, (teleport, place).after(bevy::input::InputSystems).before(bevy::ui::UiSystems::Focus))
            .add_systems(PreUpdate, (toggle, typing).chain().after(bevy::input::InputSystems).after(crate::editor::capture).after(crate::bestiary::panel::capture).after(crate::title::capture))
            .add_systems(PreUpdate, (clicks, hold, scroll).chain().after(bevy::ui::UiSystems::Focus))
            .add_systems(Update, (rebuild, show, caption))
            .add_systems(Update, endless)
            .add_systems(FixedUpdate, god.after(crate::creatures::body::rise::shatter).before(crate::creatures::deaths).in_set(crate::world::TickSet::Bodies));
    }
}

/// The key left of 1 (or F1; F1 needs fn on a Mac. That key is Backquote
/// on a US layout and IntlBackslash on an ISO Mac, like a Norwegian one's
/// §, whose < key by Z then reads as Backquote): open, shut. Esc shuts it.
/// In the arena it's open from the start. The arena's keys (pause, step...)
/// work while it's open, and always in the arena.
fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mode: Res<Mode>,
    sim: Res<SimWorld>,
    taken: Res<KeyboardTaken>,
    mut acts: MessageReader<DevAction>,
    mut menu: ResMut<DevMenu>,
    mut view: ResMut<ArenaView>,
    mut first: Local<bool>,
) {
    if !mode.dev() {
        menu.open = false;
        view.open = false;
        return;
    }
    if !*first {
        *first = true;
        menu.open = !sim.generator.wild();
    }
    if !taken.0 || menu.typing {
        if keys.any_just_pressed([KeyCode::F1, KeyCode::Backquote, KeyCode::IntlBackslash]) {
            menu.open = !menu.open;
            menu.typing = false;
        }
        if keys.just_pressed(KeyCode::Escape) && menu.open && !menu.typing {
            menu.open = false;
        }
    }
    // (The old arena button: the menu at its Time tab.)
    for a in acts.read() {
        if *a == DevAction::Arena {
            menu.open = true;
            menu.tab = Tab::Time;
        }
    }
    view.open = menu.open || !sim.generator.wild();
}

/// While the search is being typed into, the keyboard is its: letters,
/// Backspace, Esc (clears it, then stops typing), Enter (stops typing).
fn typing(mut menu: ResMut<DevMenu>, mut keys: ResMut<ButtonInput<KeyCode>>, mut typed: MessageReader<KeyboardInput>, mut taken: ResMut<KeyboardTaken>) {
    if !(menu.open && menu.typing) {
        menu.typing = false;
        typed.clear();
        return;
    }
    taken.0 = true;
    if keys.just_pressed(KeyCode::Escape) {
        if menu.search.is_empty() {
            menu.typing = false;
        }
        menu.search.clear();
    }
    if keys.just_pressed(KeyCode::Enter) {
        menu.typing = false;
    }
    for k in typed.read().filter(|k| k.state.is_pressed()) {
        match &k.logical_key {
            Key::Backspace => {
                menu.search.pop();
            }
            Key::Character(c) if c.chars().all(|c| c.is_alphanumeric() || c == '_' || c == ' ' || c == ':') => menu.search.push_str(&c.to_lowercase()),
            Key::Space => menu.search.push(' '),
            _ => {}
        }
    }
    // (The menu's key still shuts it; nothing else hears the keys.)
    let shut = keys.any_just_pressed([KeyCode::F1]);
    keys.reset_all();
    if shut {
        menu.typing = false;
        menu.open = false;
    }
}

/// Armed, the next click in the world (not on a panel) puts you there.
fn teleport(
    mut menu: ResMut<DevMenu>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    over: Res<PointerOverUi>,
    cursor: Res<CursorWorld>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
) {
    if !menu.teleport || over.0 || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let (Some(at), Ok(mut k)) = (cursor.0, player.single_mut()) else { return };
    k.body.pos = at;
    k.body.vel = Vec2::ZERO;
    k.prev_pos = at;
    menu.teleport = false;
    // (The click was the teleport's, not the hands'.)
    mouse.reset(MouseButton::Left);
    info!("dev: teleported to ({:.0}, {:.0})", at.x, at.y);
}

/// Placing: a click in the world (not on a panel) puts the picked creature
/// (or pack) there; a right-click, or shutting the menu, stops it.
pub(crate) fn place(mut menu: ResMut<DevMenu>, mut mouse: ResMut<ButtonInput<MouseButton>>, over: Res<PointerOverUi>, cursor: Res<CursorWorld>, kind: Res<SpawnKind>, mut out: MessageWriter<DevAction>) {
    if !menu.placing {
        return;
    }
    if !menu.open {
        menu.placing = false;
        return;
    }
    if over.0 {
        return;
    }
    if mouse.just_pressed(MouseButton::Right) {
        menu.placing = false;
        mouse.reset(MouseButton::Right);
        return;
    }
    if mouse.just_pressed(MouseButton::Left)
        && let Some(at) = cursor.0
    {
        out.write(DevAction::Spawn(Some(at)));
        // (The click was the placing's, not the hands'.)
        mouse.reset(MouseButton::Left);
        info!("dev: placed {} at ({:.0}, {:.0})", kind.0, at.x, at.y);
    }
}

type Me<'a> = (&'a mut Health, Option<&'a mut Inventory>, &'a mut crate::gear::Equipment, &'a Kinematics);

#[allow(clippy::too_many_arguments)]
pub(crate) fn clicks(
    mut commands: Commands,
    mut toasts: MessageWriter<crate::progress::Toast>,
    mut said: MessageWriter<crate::pickups::PickedUp>,
    q: Query<(&Interaction, &Act), Changed<Interaction>>,
    items_q: Query<(&Interaction, &ItemButton), Changed<Interaction>>,
    keys: Res<ButtonInput<KeyCode>>,
    items: Option<Res<Items>>,
    mut menu: ResMut<DevMenu>,
    mut cheats: ResMut<Cheats>,
    (mut dev_tools, mut belt, mut checkpoint, mut day, picked): (ResMut<DevTools>, ResMut<Toolbelt>, ResMut<crate::respawn::Checkpoint>, ResMut<crate::light::Daylight>, Res<SpawnKind>),
    mut dev: MessageWriter<DevAction>,
    mut arena: MessageWriter<ArenaAction>,
    mut player: Query<Me, With<LocalPlayer>>,
) {
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let mut me = player.single_mut().ok();
    // What the pack had no room for: at your feet, to pick up as room comes.
    let mut over: Vec<Stack> = Vec::new();
    for (i, act) in &q {
        if *i != Interaction::Pressed {
            continue;
        }
        if *act != Act::Search {
            menu.typing = false;
        }
        match act {
            Act::Dev(a) => {
                dev.write(*a);
            }
            Act::Arena(a) => {
                // (A creature picked: placing it, until a right-click; the
                // same one again stops.)
                if let ArenaAction::Pick(k) = a {
                    menu.placing = !(menu.placing && *k == picked.0);
                    menu.teleport = false;
                }
                arena.write(a.clone());
            }
            Act::Tab(t) => {
                menu.tab = *t;
                menu.search.clear();
                let _ = std::fs::create_dir_all(crate::save::saves_dir());
                let _ = std::fs::write(tab_path(), t.name());
            }
            Act::Cat(c) => menu.cat = *c,
            Act::Search => menu.typing = !menu.typing,
            Act::God => cheats.god = !cheats.god,
            Act::Endless => cheats.endless = !cheats.endless,
            Act::Teleport => {
                menu.teleport = !menu.teleport;
                menu.placing = false;
            }
            Act::Hands => dev_tools.0 = false,
            Act::Tool(t) => {
                belt.tool = *t;
                dev_tools.0 = true;
            }
            Act::Material(m) => {
                belt.material = *m;
                belt.tool = Tool::Spawner;
                dev_tools.0 = true;
            }
            Act::Radius(d) => {
                let i = Tool::ALL.iter().position(|t| *t == belt.tool).unwrap_or(0);
                let r = belt.radius[i];
                belt.radius[i] = (r + d * (1 + r / 8)).clamp(0, 72);
            }
            // (Ahead to that hour: the world clock catches up, as it does.)
            Act::Hour(h) => day.skipped += ((h / 24.0) - day.time).rem_euclid(1.0) * 24.0,
            Act::SpawnHere => {
                let at = me.as_ref().map(|(.., k)| k.body.pos + Vec2::new(k.loco.facing * 40.0, 0.0));
                dev.write(DevAction::Spawn(at));
            }
            Act::Heal => {
                if let Some((h, ..)) = me.as_mut() {
                    h.hp = h.max;
                }
            }
            Act::Die => {
                cheats.god = false;
                menu.open = false;
                if let Some((h, ..)) = me.as_mut() {
                    h.hp = 0.0;
                }
            }
            Act::Checkpoint(set) => {
                checkpoint.0 = if *set { me.as_ref().map(|(.., k)| k.body.pos - Vec2::Y * k.body.half.y) } else { None };
                info!("dev: checkpoint {:?}", checkpoint.0);
            }
            Act::Kit(name) => {
                if let (Some(items), Some((_, Some(inv), eq, _))) = (items.as_deref(), me.as_mut()) {
                    over.extend(crate::hands::put_on(items, items.kit(name), &mut *inv, &mut *eq));
                }
            }
            Act::Empty => {
                if let Some((_, Some(inv), ..)) = me.as_mut() {
                    inv.slots.iter_mut().for_each(|s| *s = None);
                }
            }
        }
    }
    // An item: one into the pack, or (Shift) a stack.
    let Some(items) = items else { return };
    for (i, b) in &items_q {
        if *i != Interaction::Pressed {
            continue;
        }
        menu.typing = false;
        if let Some((_, Some(inv), ..)) = me.as_mut() {
            let n = if shift { items.stack_units(b.0) } else { items.unit(b.0) };
            let left = inv.add(&items, Stack::new(b.0, n));
            if left > 0 {
                over.push(Stack::new(b.0, left));
            }
            // (Said over your head, as anything picked up is.)
            if left < n {
                said.write(crate::pickups::PickedUp::Item { stack: Stack::new(b.0, n - left), new: false });
            }
        }
    }
    drop_over(&mut commands, &mut toasts, &items, me.as_ref().map(|(.., k)| k.body.pos), over);
}

/// What didn't fit in the pack, dropped at your feet (the pack takes it as
/// room comes; the magnet holds off while it's full).
fn drop_over(commands: &mut Commands, toasts: &mut MessageWriter<crate::progress::Toast>, items: &Items, at: Option<Vec2>, over: Vec<Stack>) {
    let Some(at) = at else { return };
    if over.is_empty() {
        return;
    }
    for st in over {
        crate::hands::spawn_drop(commands, items, at, st);
    }
    toasts.write(crate::progress::Toast("Pack full: dropped at your feet".into()));
}

/// Right-click an item: it goes in the hand (the hotbar slot in use; what
/// was there goes in the pack).
fn hold(mouse: Res<ButtonInput<MouseButton>>, items: Option<Res<Items>>, hand: Res<Hand>, buttons: Query<(&Interaction, &ItemButton)>, mut player: Query<&mut Inventory, With<LocalPlayer>>) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let (Some(items), Ok(mut inv)) = (items, player.single_mut()) else { return };
    let Some((_, b)) = buttons.iter().find(|(i, _)| **i == Interaction::Hovered) else { return };
    let slot = hand.active();
    let was = inv.slots.get_mut(slot).and_then(|s| s.replace(Stack::new(b.0, items.stack_units(b.0))));
    if let Some(was) = was {
        inv.add(&items, was);
    }
}

/// The wheel over the panel scrolls it.
fn scroll(wheel: Res<AccumulatedMouseScroll>, menu: Res<DevMenu>, panel: Query<&bevy::ui::RelativeCursorPosition, With<Panel>>, mut body: Query<&mut ScrollPosition, With<Body>>) {
    if !menu.open || wheel.delta.y == 0.0 || !panel.iter().any(|p| p.cursor_over()) {
        return;
    }
    let dy = match wheel.unit {
        MouseScrollUnit::Line => wheel.delta.y * 24.0,
        MouseScrollUnit::Pixel => wheel.delta.y,
    };
    for mut s in &mut body {
        s.y = (s.y - dy).max(0.0);
    }
}

/// God mode: on with the button, and always while the menu's open (no dying
/// while you pick things).
fn god(cheats: Res<Cheats>, menu: Res<DevMenu>, mut q: Query<&mut Health, With<LocalPlayer>>) {
    if cheats.god || menu.open {
        for mut h in &mut q {
            h.hp = h.max;
        }
    }
}

fn endless(cheats: Res<Cheats>, mut q: Query<(Option<&mut crate::magic::Mana>, Option<&mut crate::combat::Stamina>), With<LocalPlayer>>) {
    if !cheats.endless {
        return;
    }
    for (mana, stamina) in &mut q {
        if let Some(mut m) = mana {
            m.cur = m.max;
        }
        if let Some(mut s) = stamina {
            s.cur = s.max;
        }
    }
}

const IDLE: Color = Color::srgba(0.25, 0.25, 0.3, 0.8);
const HOVER: Color = Color::srgba(0.35, 0.35, 0.42, 0.9);
const PRESS: Color = Color::srgba(0.6, 0.5, 0.2, 0.9);
const PICKED: Color = Color::srgba(0.55, 0.45, 0.15, 0.9);
const DIM: Color = Color::srgb(0.6, 0.6, 0.65);

fn text(s: &str, size: f32, color: Color) -> impl Bundle {
    (Text::new(s), TextFont { font_size: FontSize::Px(size), ..default() }, TextColor(color))
}

fn button(p: &mut ChildSpawnerCommands, label: &str, act: Act) {
    let reset = match &act {
        Act::Dev(DevAction::ResetWorld) => Some(crate::reset::Reset::World),
        Act::Dev(DevAction::ResetAll) => Some(crate::reset::Reset::Everything),
        _ => None,
    };
    p.spawn((Button, act, Node { padding: UiRect::axes(px(7), px(2)), ..default() }, BackgroundColor(IDLE))).with_children(|b| {
        let mut t = b.spawn(text(label, 12.0, Color::WHITE));
        // (The resets ask to be sure: `reset.rs` relabels them.)
        if let Some(r) = reset {
            t.insert(crate::reset::ResetLabel(r));
        }
    });
}

fn row(p: &mut ChildSpawnerCommands, f: impl FnOnce(&mut ChildSpawnerCommands)) {
    p.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(3), row_gap: px(3), max_width: px(WIDE), ..default() }).with_children(f);
}

fn heading(p: &mut ChildSpawnerCommands, s: &str) {
    p.spawn((text(s, 10.0, DIM), Node { margin: UiRect::top(px(4)), ..default() }));
}

#[allow(clippy::too_many_arguments)]
fn spawn_panel(
    mut commands: Commands,
    sim: Res<SimWorld>,
    tempo: Res<crate::tempo::Tempo>,
    bank: Res<crate::sound::SoundBank>,
    timeline: Res<crate::fight::Timeline>,
) {
    let tempos: Vec<String> = tempo.presets.iter().map(|p| p.name.clone()).collect();
    // (The one-shots: the beds and music play themselves.)
    let mut sounds: Vec<String> = bank.defs.iter().filter(|(_, d)| d.loops <= 0.0).map(|(n, _)| n.clone()).collect();
    sounds.sort();
    let arena = !sim.generator.wild();
    // (The materials by kind: solids, powders, liquids, gases, the rest.)
    let kinds = [(Kind::Static, "Solids"), (Kind::Powder, "Powders"), (Kind::Liquid, "Liquids"), (Kind::Gas, "Gases"), (Kind::Plant, "Plants"), (Kind::Fire, "Fire")];
    let mats: Vec<(&str, Vec<(MaterialId, String)>)> = kinds.iter().map(|(k, name)| (*name, sim.materials().iter().filter(|(id, d)| *id != MaterialId::AIR && d.kind == *k).map(|(id, d)| (id, d.name.clone())).collect())).collect();
    commands
        .spawn((
            Panel,
            Visibility::Hidden,
            // (Hovering it counts as over the UI: clicks and the wheel are its.)
            Interaction::default(),
            bevy::ui::RelativeCursorPosition::default(),
            Node { position_type: PositionType::Absolute, top: px(TOP), left: px(6), width: px(WIDE + 14.0), flex_direction: FlexDirection::Column, row_gap: px(4), padding: UiRect::all(px(6)), ..default() },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.62)),
        ))
        .with_children(|p| {
            p.spawn((Title, text("DEV", 13.0, Color::srgb(1.0, 0.85, 0.3))));
            row(p, |r| {
                for t in Tab::ALL {
                    button(r, t.name(), Act::Tab(t));
                }
            });
            p.spawn((Body, ScrollPosition::default(), Node { flex_direction: FlexDirection::Column, row_gap: px(3), overflow: Overflow::scroll_y(), min_height: px(0), flex_shrink: 1.0, ..default() })).with_children(|p| {
                let tab = |p: &mut ChildSpawnerCommands, t: Tab, f: &dyn Fn(&mut ChildSpawnerCommands)| {
                    p.spawn((TabBody(t), Node { flex_direction: FlexDirection::Column, row_gap: px(3), display: Display::None, ..default() })).with_children(|b| f(b));
                };
                tab(p, Tab::Player, &|p| {
                    heading(p, "Cheats");
                    row(p, |r| {
                        button(r, "God mode (on while this is open)", Act::God);
                        button(r, "Endless mana & stamina", Act::Endless);
                        button(r, "Heal", Act::Heal);
                        button(r, "Die", Act::Die);
                    });
                    heading(p, "Go");
                    row(p, |r| {
                        button(r, "To the start  F2", Act::Dev(DevAction::Surface));
                        button(r, "Teleport: click the world", Act::Teleport);
                    });
                    heading(p, "Checkpoint: where you wake, dying in play");
                    row(p, |r| {
                        button(r, "Here", Act::Checkpoint(true));
                        button(r, "The start", Act::Checkpoint(false));
                    });
                    heading(p, "Light");
                    row(p, |r| {
                        button(r, "Carried: beam, big, torch  L", Act::Dev(DevAction::Flashlight));
                        button(r, "Plant a torch  G", Act::Dev(DevAction::PlantTorch(None)));
                    });
                });
                tab(p, Tab::Items, &|p| {
                    heading(p, "Kits (items.ron): into the pack, onto you");
                    p.spawn((Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(3), row_gap: px(3), max_width: px(WIDE), ..default() }, KitRow));
                    heading(p, "Click: one | Shift: a stack | right-click: in hand");
                    row(p, |r| {
                        button(r, "all", Act::Cat(None));
                        for c in Cat::ALL {
                            button(r, c.name(), Act::Cat(Some(c)));
                        }
                    });
                    search(p);
                    p.spawn((Caption, text(" ", 11.0, Color::srgb(0.9, 0.9, 0.92)), Node { max_width: px(WIDE), ..default() }));
                    p.spawn((ItemGrid, Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(2), row_gap: px(2), max_width: px(WIDE), ..default() }));
                    row(p, |r| button(r, "Empty the pack", Act::Empty));
                });
                tab(p, Tab::Creatures, &|p| {
                    row(p, |r| {
                        button(r, "Spawn at the cursor  O", Act::Dev(DevAction::Spawn(None)));
                        button(r, "In front of you", Act::SpawnHere);
                        button(r, "Clear the floor", Act::Arena(ArenaAction::Clear));
                    });
                    row(p, |r| {
                        button(r, "Bestiary  F12", Act::Arena(ArenaAction::Bestiary));
                        button(r, "Show the next  P", Act::Dev(DevAction::ShowNext));
                        button(r, "Show mode", Act::Dev(DevAction::ShowMode));
                    });
                    heading(p, "Pick one, then click the world to place it (right-click stops)");
                    search(p);
                    p.spawn((CreatureList, Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(3), row_gap: px(3), max_width: px(WIDE), ..default() }));
                });
                tab(p, Tab::World, &|p| {
                    heading(p, "Time of day (ahead to it)");
                    row(p, |r| {
                        for (h, l) in [(6.0, "Dawn"), (12.0, "Noon"), (18.0, "Dusk"), (0.0, "Midnight")] {
                            button(r, l, Act::Hour(h));
                        }
                        button(r, "+3 hours  M", Act::Dev(DevAction::Later));
                        button(r, "A day ahead", Act::Dev(DevAction::DayAhead));
                    });
                    heading(p, "Weather");
                    row(p, |r| {
                        button(r, "Storm  V", Act::Dev(DevAction::Storm));
                        button(r, "Clear sky  B", Act::Dev(DevAction::ClearSky));
                        button(r, "Lightning  N", Act::Dev(DevAction::Lightning(None)));
                    });
                    heading(p, "Events");
                    row(p, |r| button(r, "An event (star, raid, quake, pedlar)", Act::Dev(DevAction::Event)));
                    if arena {
                        heading(p, "The arena's floor (the world reset, you kept)");
                        row(p, |r| {
                            for l in ["sandbox", "flat", "cave", "slopes", "stairs", "real"] {
                                button(r, l, Act::Arena(ArenaAction::Layout(l.into())));
                            }
                        });
                    }
                    heading(p, "Start over");
                    row(p, |r| {
                        button(r, "Reset the world (twice)", Act::Dev(DevAction::ResetWorld));
                        button(r, "Reset everything (twice)", Act::Dev(DevAction::ResetAll));
                    });
                });
                tab(p, Tab::Tools, &|p| {
                    heading(p, "In hand: 1-8 pick a tool, the wheel or [ ] its radius");
                    row(p, |r| {
                        button(r, "Hands", Act::Hands);
                        for t in Tool::ALL {
                            button(r, t.label(), Act::Tool(t));
                        }
                    });
                    row(p, |r| {
                        button(r, "Radius -", Act::Radius(-1));
                        button(r, "Radius +", Act::Radius(1));
                        r.spawn((RadiusText, text("", 12.0, Color::WHITE)));
                    });
                    heading(p, "The spawner pours (Q/E step; Shift: through solids)");
                    for (kind, list) in &mats {
                        if list.is_empty() {
                            continue;
                        }
                        heading(p, kind);
                        row(p, |r| {
                            for (id, name) in list {
                                button(r, name, Act::Material(*id));
                            }
                        });
                    }
                });
                tab(p, Tab::View, &|p| {
                    row(p, |r| {
                        button(r, "Lighting  K", Act::Dev(DevAction::Lighting));
                        button(r, "Performance HUD  H", Act::Dev(DevAction::PerfHud));
                        button(r, "Chunks  J", Act::Dev(DevAction::Chunks));
                        button(r, "Boxes and hands  Y", Act::Arena(ArenaAction::Overlays));
                        button(r, "Alert marks", Act::Dev(DevAction::AlertMarks));
                        button(r, "Damage numbers", Act::Dev(DevAction::DamageNumbers));
                    });
                    heading(p, "Light radius (the old toolbelt's)");
                    row(p, |r| {
                        button(r, "Radius -", Act::Dev(DevAction::Radius(-1)));
                        button(r, "Radius +", Act::Dev(DevAction::Radius(1)));
                    });
                    heading(p, "Make");
                    row(p, |r| {
                        button(r, "Art editor  E", Act::Arena(ArenaAction::Editor));
                        button(r, "Bestiary  F12", Act::Arena(ArenaAction::Bestiary));
                    });
                });
                tab(p, Tab::Time, &|p| {
                    row(p, |r| {
                        button(r, "Pause  P", Act::Arena(ArenaAction::Pause));
                        button(r, "Step  .", Act::Arena(ArenaAction::Step));
                        for (s, l) in SPEEDS {
                            button(r, l, Act::Arena(ArenaAction::Speed(s)));
                        }
                    });
                    heading(p, "Tempo (tempo.ron): T the next");
                    row(p, |r| {
                        for (i, name) in tempos.iter().enumerate() {
                            button(r, name, Act::Arena(ArenaAction::Tempo(i)));
                        }
                    });
                    heading(p, "The fight: to them above, to you below");
                    p.spawn((crate::fight::FightText, text("", 10.0, Color::srgb(0.9, 0.9, 0.92)), Node { max_width: px(WIDE), ..default() }));
                    p.spawn((ImageNode::new(timeline.0.clone()), Node { width: px(200), height: px(41), ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6))));
                    row(p, |r| {
                        button(r, "New fight", Act::Arena(ArenaAction::NewFight));
                        button(r, "Record", Act::Arena(ArenaAction::Record));
                        button(r, "Stop and save", Act::Arena(ArenaAction::Stop));
                        button(r, "Replay the last", Act::Arena(ArenaAction::ReplayLast));
                    });
                });
                // Sounds, grouped: hits, steps and moving, hands, the rest.
                tab(p, Tab::Sounds, &|p| {
                    heading(p, "Click to hear (sounds.ron); F11 mutes");
                    type Group<'a> = (&'a str, &'a dyn Fn(&str) -> bool);
                    let groups: [Group; 4] = [
                        ("Hits", &|n| n.starts_with("hit") || n.starts_with("hurt") || n.starts_with("swing") || n.starts_with("clang")),
                        ("Steps and moving", &|n| n.starts_with("step_") || matches!(n, "land" | "jump" | "air_jump" | "dash")),
                        ("Hands", &|n| n.starts_with("mine_") || matches!(n, "break" | "place" | "craft" | "pickup")),
                        ("The rest", &|_| true),
                    ];
                    let mut left: Vec<&String> = sounds.iter().collect();
                    for (name, pick) in groups {
                        let these: Vec<&String> = left.iter().copied().filter(|n| pick(n)).collect();
                        left.retain(|n| !pick(n));
                        if these.is_empty() {
                            continue;
                        }
                        heading(p, name);
                        row(p, |r| {
                            for n in &these {
                                button(r, n, Act::Arena(ArenaAction::Sound((*n).clone())));
                            }
                        });
                    }
                });
            });
        });
}

/// The kits' row (filled once the items are read).
#[derive(Component)]
struct KitRow;

fn search(p: &mut ChildSpawnerCommands) {
    p.spawn((Button, Act::Search, Node { padding: UiRect::axes(px(7), px(3)), width: px(WIDE), ..default() }, BackgroundColor(Color::srgba(0.12, 0.12, 0.16, 0.9)))).with_children(|b| {
        b.spawn((SearchText, text("Search: click, type", 12.0, DIM)));
    });
}

/// The lists, built again when what they're built from changes: the items
/// (by kind and search), the creatures (by search), the kits.
#[allow(clippy::too_many_arguments)]
fn rebuild(
    mut commands: Commands,
    mut menu: ResMut<DevMenu>,
    items: Option<Res<Items>>,
    icons: Option<Res<Icons>>,
    grid: Query<Entity, With<ItemGrid>>,
    list: Query<Entity, With<CreatureList>>,
    kits: Query<Entity, With<KitRow>>,
) {
    let (Some(items), Some(icons)) = (items, icons) else { return };
    let shape = format!("{:?}|{}|{}", menu.cat, menu.search, menu.tab.name());
    if shape == menu.built && !items.is_changed() && !icons.is_changed() {
        return;
    }
    let first = menu.built.is_empty();
    menu.built = shape;
    let words: Vec<&str> = menu.search.split_whitespace().collect();
    let matches = |s: &str| words.iter().all(|w| s.to_lowercase().contains(w));
    if first {
        for e in &kits {
            commands.entity(e).with_children(|r| {
                for k in &items.kits {
                    button(r, &k.name, Act::Kit(k.name.clone()));
                }
            });
        }
    }
    if menu.tab == Tab::Items {
        for e in &grid {
            commands.entity(e).despawn_related::<Children>();
            commands.entity(e).with_children(|g| {
                for i in 0..items.len() {
                    let id = ItemId(i as u16);
                    let def = items.def(id);
                    if menu.cat.is_some_and(|c| Cat::of(def) != c) || !(matches(&def.name) || matches(&def.id)) {
                        continue;
                    }
                    g.spawn((Button, ItemButton(id), Node { width: px(28), height: px(28), padding: UiRect::all(px(3)), ..default() }, BackgroundColor(IDLE))).with_children(|b| match icons.get(id) {
                        Some(h) => {
                            b.spawn((ImageNode::new(h.clone()), Node { width: px(22), height: px(22), ..default() }));
                        }
                        None => {
                            let (r, gr, bl) = def.color;
                            b.spawn((Node { width: px(22), height: px(22), ..default() }, BackgroundColor(Color::srgb_u8(r, gr, bl))));
                        }
                    });
                }
            });
        }
    }
    if menu.tab == Tab::Creatures {
        for e in &list {
            commands.entity(e).despawn_related::<Children>();
            let packs: Vec<String> = crate::creatures::spawn::packs().into_keys().filter(|k| matches(k)).collect();
            let kinds: Vec<String> = crate::arena::creature_kinds().into_iter().filter(|k| matches(k)).collect();
            commands.entity(e).with_children(|p| {
                for (name, list) in [("Packs (packs.ron)", packs), ("Kinds", kinds)] {
                    if list.is_empty() {
                        continue;
                    }
                    p.spawn((text(name, 10.0, DIM), Node { width: percent(100), margin: UiRect::top(px(3)), ..default() }));
                    for k in list {
                        button(p, &k, Act::Arena(ArenaAction::Pick(k.clone())));
                    }
                }
            });
        }
    }
}

/// What's under the pointer in the item grid, and the search as typed.
fn caption(menu: Res<DevMenu>, items: Option<Res<Items>>, buttons: Query<(&Interaction, &ItemButton)>, kits: Query<(&Interaction, &Act)>, mut cap: Query<&mut Text, (With<Caption>, Without<SearchText>)>, mut field: Query<(&mut Text, &mut TextColor), With<SearchText>>) {
    let Some(items) = items else { return };
    let kit = kits.iter().find_map(|(i, a)| match a {
        Act::Kit(k) if *i != Interaction::None => Some(items.kit(k)),
        _ => None,
    });
    let want = buttons.iter().find(|(i, _)| **i != Interaction::None).map_or_else(
        || kit.map_or_else(|| " ".to_string(), |k| format!("The {} kit: {}", k.name, k.about)),
        |(_, b)| {
            let d = items.def(b.0);
            match &d.about {
                Some(a) => format!("{}: {a}", d.name),
                None => format!("{}  ({})", d.name, d.id),
            }
        },
    );
    for mut t in &mut cap {
        if t.0 != want {
            t.0.clone_from(&want);
        }
    }
    let (s, c) = match (menu.typing, menu.search.is_empty()) {
        (true, _) => (format!("Search: {}_", menu.search), Color::WHITE),
        (false, true) => ("Search: click, type".to_string(), DIM),
        (false, false) => (format!("Search: {}", menu.search), Color::WHITE),
    };
    for (mut t, mut col) in &mut field {
        if t.0 != s {
            t.0.clone_from(&s);
        }
        col.0 = c;
    }
}

#[allow(clippy::too_many_arguments)]
fn show(
    menu: Res<DevMenu>,
    view: Res<ArenaView>,
    cheats: Res<Cheats>,
    (virt, kind, tempo, layout): (Res<Time<Virtual>>, Res<SpawnKind>, Res<crate::tempo::Tempo>, Res<CurrentLayout>),
    (dev_tools, belt, toggles): (Res<DevTools>, Res<Toolbelt>, Res<crate::light::LightToggles>),
    windows: Query<&Window>,
    inventory: Res<crate::hands::ui::InventoryOpen>,
    mut panel: Query<(&mut Visibility, &mut Node), With<Panel>>,
    mut bodies: Query<(&TabBody, &mut Node), Without<Panel>>,
    mut title: Query<&mut Text, (With<Title>, Without<RadiusText>)>,
    mut radius: Query<&mut Text, With<RadiusText>>,
    mut buttons: Query<(&Interaction, &Act, &mut BackgroundColor), Without<ItemButton>>,
    mut item_buttons: Query<(&Interaction, &mut BackgroundColor), With<ItemButton>>,
) {
    // No taller than the window leaves (the hotbar above, the debug text below).
    let most = windows.iter().next().map_or(600.0, |w| (w.height() - TOP - 150.0).max(160.0));
    for (mut v, mut node) in &mut panel {
        *v = if menu.open && !inventory.0 { Visibility::Visible } else { Visibility::Hidden };
        if node.max_height != px(most) {
            node.max_height = px(most);
        }
    }
    if !menu.open {
        return;
    }
    for (b, mut node) in &mut bodies {
        let want = if b.0 == menu.tab { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
    }
    let speed = virt.relative_speed();
    let time = if virt.is_paused() { "  paused".to_string() } else if speed < 1.0 { format!("  x{speed}") } else { String::new() };
    let hand = if dev_tools.0 { format!("  | {}", belt.tool.label()) } else { String::new() };
    let teleport = if menu.teleport {
        "  | click to teleport".to_string()
    } else if menu.placing {
        format!("  | placing {}: click the world, right-click stops", kind.0)
    } else {
        String::new()
    };
    for mut t in &mut title {
        let want = format!("DEV  {}{time}{hand}{teleport}", tempo.name());
        if t.0 != want {
            t.0 = want;
        }
    }
    for mut t in &mut radius {
        let want = format!("{}: {}", belt.tool.label(), belt.radius());
        if t.0 != want {
            t.0 = want;
        }
    }
    for (i, act, mut bg) in &mut buttons {
        let on = match act {
            Act::Tab(t) => *t == menu.tab,
            Act::Cat(c) => *c == menu.cat,
            Act::God => cheats.god,
            Act::Endless => cheats.endless,
            Act::Teleport => menu.teleport,
            Act::Search => menu.typing,
            Act::Hands => !dev_tools.0,
            Act::Tool(t) => dev_tools.0 && *t == belt.tool,
            Act::Material(m) => *m == belt.material,
            Act::Dev(DevAction::Lighting) => toggles.enabled,
            Act::Arena(ArenaAction::Speed(s)) => (s - speed).abs() < 1e-3,
            Act::Arena(ArenaAction::Pause) => virt.is_paused(),
            Act::Arena(ArenaAction::Overlays) => view.overlays,
            Act::Arena(ArenaAction::Pick(k)) => *k == kind.0,
            Act::Arena(ArenaAction::Tempo(i)) => *i == tempo.active,
            Act::Arena(ArenaAction::Layout(l)) => *l == layout.0,
            _ => false,
        };
        let want = match i {
            Interaction::Pressed => PRESS,
            Interaction::Hovered => HOVER,
            Interaction::None if on => PICKED,
            Interaction::None if *act == Act::Search => Color::srgba(0.12, 0.12, 0.16, 0.9),
            Interaction::None => IDLE,
        };
        if bg.0 != want {
            bg.0 = want;
        }
    }
    for (i, mut bg) in &mut item_buttons {
        let want = match i {
            Interaction::Pressed => PRESS,
            Interaction::Hovered => HOVER,
            Interaction::None => IDLE,
        };
        if bg.0 != want {
            bg.0 = want;
        }
    }
}
