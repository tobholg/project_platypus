//! What the player picks up (`PickupFilter`, saved with it): so its pack
//! doesn't fill with what it doesn't want. Things fall in seven groups, as
//! a player thinks of loot:
//!
//! - **Gold** (the count, `gold.rs`);
//! - **Gear**: weapons, armour, foci, tools, from a rarity up (every one,
//!   Uncommon and better, Rare..., none);
//! - **Supplies**: potions, arrows, bombs, torches, glowsticks, whatever
//!   else is made;
//! - **Ores & bars**, **Gems & crystals**, **Wood**, and plain **Blocks**
//!   (dirt, stone, sand, brick...): a block's group is its material's
//!   `loot` (materials.ron).
//!
//! In the pack (Esc), a row under it: a chip a group (click: on or off; the
//! gear chip steps through the rarities, right-click back, its edge the
//! rarity's colour), hover one for what it is; before them a button that
//! steps through the presets (Everything, Valuables, Builder; Custom when
//! it's none of them). V steps through the presets while playing ("Pick
//! up: Valuables" over the head).
//!
//! What it won't pick up isn't drawn to it and lies faded (right-click it,
//! near enough, to take it anyway: the cursor shows a hand over it). A
//! block it won't pick up, dug with its own hands, crumbles to dust rather
//! than drop (a tunnel would leave a floor of them).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::items::{Items, Stack, Use};
use super::{Dropped, InventoryOpen};
use crate::camera::CursorWorld;
use crate::creatures::Kinematics;
use crate::creatures::player::LocalPlayer;
use crate::gear::GearRules;

/// What the player picks up.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PickupFilter {
    pub gold: bool,
    /// Gear from this rarity up (`gear.ron`'s order); none: no gear.
    pub gear: Option<u8>,
    pub supplies: bool,
    pub ores: bool,
    pub gems: bool,
    pub wood: bool,
    pub blocks: bool,
}

impl Default for PickupFilter {
    fn default() -> Self {
        EVERYTHING
    }
}

const EVERYTHING: PickupFilter = PickupFilter { gold: true, gear: Some(0), supplies: true, ores: true, gems: true, wood: true, blocks: true };
const PRESETS: [(&str, PickupFilter); 3] = [
    ("Everything", EVERYTHING),
    // Worth carrying home: no wood, no plain blocks, gear Rare and up.
    ("Valuables", PickupFilter { gold: true, gear: Some(2), supplies: true, ores: true, gems: true, wood: false, blocks: false }),
    // Building: every block, gear only Uncommon and up.
    ("Builder", PickupFilter { gold: true, gear: Some(1), supplies: true, ores: true, gems: true, wood: true, blocks: true }),
];

/// What group a thing falls in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Gold,
    Gear,
    Supplies,
    Ores,
    Gems,
    Wood,
    Blocks,
}

impl Group {
    pub const ALL: [Group; 7] = [Group::Gold, Group::Gear, Group::Supplies, Group::Ores, Group::Gems, Group::Wood, Group::Blocks];

    fn label(self) -> &'static str {
        match self {
            Group::Gold => "Gold",
            Group::Gear => "Gear",
            Group::Supplies => "Supplies",
            Group::Ores => "Ores & bars",
            Group::Gems => "Gems & crystals",
            Group::Wood => "Wood",
            Group::Blocks => "Blocks",
        }
    }

    /// The item whose icon is the chip's (gold has its own).
    fn icon(self) -> Option<&'static str> {
        match self {
            Group::Gold => None,
            Group::Gear => Some("iron_helm"),
            Group::Supplies => Some("small_health_potion"),
            Group::Ores => Some("block:copper_ore"),
            Group::Gems => Some("block:amethyst"),
            Group::Wood => Some("block:wood"),
            Group::Blocks => Some("block:stone"),
        }
    }
}

/// A thing's group: gear (and foci, tools) as gear; a block or bar by its
/// material's `loot`; the rest supplies.
pub fn group_of(items: &Items, s: &Stack) -> Group {
    let def = items.def(s.item);
    match def.use_ {
        Use::Block(_) => match def.loot {
            platypus_sim::Loot::Ore => Group::Ores,
            platypus_sim::Loot::Gem => Group::Gems,
            platypus_sim::Loot::Wood => Group::Wood,
            platypus_sim::Loot::Block => Group::Blocks,
        },
        Use::Focus { .. } | Use::Mine { .. } => Group::Gear,
        _ if def.gear.is_some() => Group::Gear,
        _ => Group::Supplies,
    }
}

impl PickupFilter {
    /// Does it pick this up?
    pub fn takes(&self, items: &Items, s: &Stack) -> bool {
        match group_of(items, s) {
            Group::Gear => self.gear.is_some_and(|from| s.roll.rarity >= from),
            g => self.takes_group(g),
        }
    }

    fn takes_group(&self, g: Group) -> bool {
        match g {
            Group::Gold => self.gold,
            Group::Gear => self.gear.is_some(),
            Group::Supplies => self.supplies,
            Group::Ores => self.ores,
            Group::Gems => self.gems,
            Group::Wood => self.wood,
            Group::Blocks => self.blocks,
        }
    }

    fn toggle(&mut self, g: Group) {
        let flag = match g {
            Group::Gold => &mut self.gold,
            Group::Supplies => &mut self.supplies,
            Group::Ores => &mut self.ores,
            Group::Gems => &mut self.gems,
            Group::Wood => &mut self.wood,
            Group::Blocks => &mut self.blocks,
            Group::Gear => {
                self.gear = if self.gear.is_some() { None } else { Some(0) };
                return;
            }
        };
        *flag = !*flag;
    }

    /// Gear's rarity a step on (`dir` 1) or back: every one, ... `most`
    /// and better, none, every one again.
    fn step_gear(&mut self, dir: i32, most: u8) {
        let n = most as i32 + 2;
        let at = self.gear.map_or(most as i32 + 1, |r| r as i32);
        let next = (at + dir).rem_euclid(n);
        self.gear = (next <= most as i32).then_some(next as u8);
    }

    /// The preset it is, if it's one.
    fn preset(&self) -> Option<usize> {
        PRESETS.iter().position(|(_, p)| p == self)
    }
}

pub struct FilterPlugin;

impl Plugin for FilterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_row).add_systems(Update, (give, cycle, chips, show_row, fill_icons, fade, take_anyway));
    }
}

type NewPlayer = (With<LocalPlayer>, Without<PickupFilter>);

fn give(mut commands: Commands, new: Query<Entity, NewPlayer>) {
    for e in &new {
        commands.entity(e).insert(PickupFilter::default());
    }
}

/// The most a gear chip steps to (the last rarity found in the world:
/// `gear.ron`'s weights; Legendary is only made).
fn most_rarity(rules: Option<&GearRules>) -> u8 {
    rules.map_or(3, |r| r.rarities.iter().rposition(|x| x.weight > 0.0).unwrap_or(0) as u8)
}

/// V: the next preset, said over the head.
fn cycle(keys: Res<ButtonInput<KeyCode>>, taken: Res<crate::dev::KeyboardTaken>, mut player: Query<&mut PickupFilter, With<LocalPlayer>>, mut said: MessageWriter<crate::pickups::PickedUp>) {
    if taken.0 || !keys.just_pressed(KeyCode::KeyV) {
        return;
    }
    let Ok(mut f) = player.single_mut() else { return };
    let next = f.preset().map_or(0, |i| (i + 1) % PRESETS.len());
    *f = PRESETS[next].1;
    said.write(crate::pickups::PickedUp::Note(format!("Pick up: {}", PRESETS[next].0)));
}

#[derive(Component)]
struct Row;
#[derive(Component)]
struct Chip(Group);
#[derive(Component)]
struct ChipIcon(Group);
#[derive(Component)]
struct PresetButton;
#[derive(Component)]
struct PresetLabel;
#[derive(Component)]
struct Hint;

const CHIP: f32 = 30.0;
const CHIP_ICON: f32 = 22.0;
const ON_BG: Color = Color::srgba(0.16, 0.2, 0.42, 0.85);
const OFF_BG: Color = Color::srgba(0.06, 0.06, 0.1, 0.8);
const ON_EDGE: Color = Color::srgba(0.55, 0.62, 0.9, 0.9);
const OFF_EDGE: Color = Color::srgba(0.2, 0.2, 0.26, 0.9);
/// Where the row sits: under the pack, over any open chest.
const TOP: f32 = super::ui::UNDER_PACK - super::ui::FILTER_ROW - 12.0;

fn text(s: &str, size: f32) -> (Text, TextFont, TextColor, TextShadow) {
    (Text::new(s), TextFont { font_size: FontSize::Px(size), ..default() }, TextColor(Color::srgb(0.88, 0.88, 0.93)), TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85) })
}

fn spawn_row(mut commands: Commands) {
    commands
        .spawn((
            Row,
            Interaction::None,
            Visibility::Hidden,
            Node { position_type: PositionType::Absolute, top: px(TOP), left: px(10), column_gap: px(5), align_items: AlignItems::Center, padding: UiRect::axes(px(8), px(4)), border_radius: BorderRadius::all(px(7)), ..default() },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        ))
        .with_children(|row| {
            row.spawn(text("Pick up", 12.0));
            row.spawn((
                PresetButton,
                Button,
                Node { padding: UiRect::axes(px(8), px(4)), border: UiRect::all(px(2)), border_radius: BorderRadius::all(px(6)), min_width: px(92), justify_content: JustifyContent::Center, ..default() },
                BorderColor::all(ON_EDGE),
                BackgroundColor(ON_BG),
            ))
            .with_child((PresetLabel, text("Everything", 12.0)));
            for g in Group::ALL {
                row.spawn((
                    Chip(g),
                    Button,
                    Node { width: px(CHIP), height: px(CHIP), border: UiRect::all(px(2)), border_radius: BorderRadius::all(px(6)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                    BorderColor::all(ON_EDGE),
                    BackgroundColor(ON_BG),
                ))
                .with_child((ChipIcon(g), ImageNode::default(), Node { width: px(CHIP_ICON), height: px(CHIP_ICON), ..default() }));
            }
            row.spawn((Hint, text("", 12.0), Node { margin: UiRect::left(px(6)), ..default() }));
        });
}

/// The chips' pictures, once there are icons (gold's its own nugget).
fn fill_icons(items: Option<Res<Items>>, icons: Option<Res<super::icons::Icons>>, mut images: ResMut<Assets<Image>>, mut q: Query<(&ChipIcon, &mut ImageNode)>, mut done: Local<bool>) {
    let (Some(items), Some(icons)) = (items, icons) else { return };
    if *done && !icons.is_changed() {
        return;
    }
    *done = true;
    for (c, mut img) in &mut q {
        let handle = match c.0.icon() {
            None => Some(crate::pickups::gold_icon(&mut images)),
            Some(id) => items.id(id).and_then(|i| icons.get(i).cloned()),
        };
        if let Some(h) = handle {
            img.image = h;
        }
    }
}

/// The row shows with the pack.
fn show_row(open: Res<InventoryOpen>, mut row: Query<&mut Visibility, With<Row>>) {
    for mut v in &mut row {
        let want = if open.0 { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want {
            *v = want;
        }
    }
}

/// Clicks on the chips and the preset button; how each looks; what the
/// hovered one is.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn chips(
    open: Res<InventoryOpen>,
    mouse: Res<ButtonInput<MouseButton>>,
    rules: Option<Res<GearRules>>,
    mut player: Query<&mut PickupFilter, With<LocalPlayer>>,
    mut chips: Query<(&Chip, &Interaction, &mut BackgroundColor, &mut BorderColor), Without<PresetButton>>,
    preset: Query<&Interaction, (With<PresetButton>, Changed<Interaction>)>,
    mut icons: Query<(&ChipIcon, &mut ImageNode)>,
    mut label: Query<&mut Text, (With<PresetLabel>, Without<Hint>)>,
    mut hint: Query<&mut Text, (With<Hint>, Without<PresetLabel>)>,
) {
    let Ok(mut f) = player.single_mut() else { return };
    if !open.0 {
        return;
    }
    let most = most_rarity(rules.as_deref());
    let rarity = |r: u8| rules.as_ref().and_then(|x| x.rarities.get(r as usize));
    if preset.iter().any(|i| *i == Interaction::Pressed) {
        let next = f.preset().map_or(0, |i| (i + 1) % PRESETS.len());
        *f = PRESETS[next].1;
    }
    let mut hovered = None;
    for (c, i, _, _) in &chips {
        if *i == Interaction::Pressed && mouse.just_pressed(MouseButton::Left) {
            match c.0 {
                Group::Gear => f.step_gear(1, most),
                g => f.toggle(g),
            }
        }
        if matches!(i, Interaction::Hovered | Interaction::Pressed) {
            hovered = Some(c.0);
            if c.0 == Group::Gear && mouse.just_pressed(MouseButton::Right) {
                f.step_gear(-1, most);
            }
        }
    }
    // How each looks: on bright, off dark and dim; gear's edge its rarity's colour.
    for (c, _, mut bg, mut edge) in &mut chips {
        let on = f.takes_group(c.0);
        bg.0 = if on { ON_BG } else { OFF_BG };
        let color = match (c.0, f.gear) {
            (Group::Gear, Some(r)) => rarity(r).map_or(ON_EDGE, |x| Color::srgb_u8(x.color.0, x.color.1, x.color.2)),
            _ if on => ON_EDGE,
            _ => OFF_EDGE,
        };
        *edge = BorderColor::all(color);
    }
    for (c, mut img) in &mut icons {
        img.color = if f.takes_group(c.0) { Color::WHITE } else { Color::srgba(1.0, 1.0, 1.0, 0.28) };
    }
    if let Ok(mut t) = label.single_mut() {
        let want = f.preset().map_or("Custom", |i| PRESETS[i].0);
        if t.0 != want {
            t.0 = want.to_string();
        }
    }
    if let Ok(mut t) = hint.single_mut() {
        let want = match hovered {
            Some(Group::Gear) => match f.gear {
                Some(0) => "Gear: all of it (click: fewer, right-click: more)".to_string(),
                Some(r) => format!("Gear: {} and better", rarity(r).map_or("?", |x| x.name.as_str())),
                None => "Gear: none".to_string(),
            },
            Some(g) => format!("{}: {}", g.label(), if f.takes_group(g) { "picked up" } else { "left on the ground" }),
            None => String::new(),
        };
        if t.0 != want {
            t.0 = want;
        }
    }
}

/// What it won't pick up lies faded.
fn fade(items: Option<Res<Items>>, player: Query<&PickupFilter, With<LocalPlayer>>, mut drops: Query<(&Dropped, &mut Sprite)>) {
    let (Some(items), Ok(f)) = (items, player.single()) else { return };
    for (d, mut s) in &mut drops {
        let a = if d.forced || f.takes(&items, &d.stack) { 1.0 } else { 0.4 };
        if s.color.alpha() != a {
            s.color.set_alpha(a);
        }
    }
}

/// How near the cursor a thing on the ground is right-clicked (cells), and
/// how near the player it must be.
const CLICK_NEAR: f32 = 6.0;
const TAKE_NEAR: f32 = 42.0;

/// The thing on the ground under the cursor that the player won't pick up
/// but could take (near enough), if any.
pub fn left_under<'a>(at: Vec2, me: Vec2, items: &Items, f: &PickupFilter, drops: impl Iterator<Item = (Entity, &'a Dropped, &'a Kinematics)>) -> Option<(Entity, Stack)> {
    drops
        .filter(|(_, d, k)| !d.forced && !f.takes(items, &d.stack) && k.body.pos.distance(at) <= CLICK_NEAR && k.body.pos.distance(me) <= TAKE_NEAR)
        .min_by(|a, b| a.2.body.pos.distance(at).total_cmp(&b.2.body.pos.distance(at)))
        .map(|(e, d, _)| (e, d.stack))
}

/// Right-click on something it won't pick up, near enough: it's taken
/// anyway (drawn in as anything is).
#[allow(clippy::type_complexity)]
fn take_anyway(
    mouse: Res<ButtonInput<MouseButton>>,
    open: Res<InventoryOpen>,
    cursor: Res<CursorWorld>,
    items: Option<Res<Items>>,
    player: Query<(&Kinematics, &PickupFilter), With<LocalPlayer>>,
    mut drops: Query<(Entity, &mut Dropped, &Kinematics), Without<LocalPlayer>>,
) {
    if open.0 || !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let (Some(items), Some(at), Ok((pk, f))) = (items, cursor.0, player.single()) else { return };
    let Some((e, _)) = left_under(at, pk.body.pos, &items, f, drops.iter()) else { return };
    if let Ok((_, mut d, _)) = drops.get_mut(e) {
        d.forced = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gear_steps_through_the_rarities_and_none() {
        let mut f = PickupFilter::default();
        let seen: Vec<Option<u8>> = (0..5).map(|_| { f.step_gear(1, 3); f.gear }).collect();
        assert_eq!(seen, vec![Some(1), Some(2), Some(3), None, Some(0)]);
        f.step_gear(-1, 3);
        assert_eq!(f.gear, None);
    }

    #[test]
    fn presets_are_told_apart() {
        assert_eq!(PickupFilter::default().preset(), Some(0));
        let mut f = PRESETS[1].1;
        assert_eq!(f.preset(), Some(1));
        f.toggle(Group::Wood);
        assert_eq!(f.preset(), None);
    }

    #[test]
    fn blocks_fall_in_their_materials_groups() {
        let items = super::super::items::test_items();
        let s = |id: &str| Stack::new(items.id(id).unwrap(), 1);
        assert_eq!(group_of(&items, &s("block:copper_ore")), Group::Ores);
        assert_eq!(group_of(&items, &s("block:amethyst")), Group::Gems);
        assert_eq!(group_of(&items, &s("block:wood")), Group::Wood);
        assert_eq!(group_of(&items, &s("block:dirt")), Group::Blocks);
        assert_eq!(group_of(&items, &s("torch")), Group::Supplies);
        assert_eq!(group_of(&items, &s("copper_pickaxe")), Group::Gear);
    }
}
