//! Crafting (DESIGN §7d): stations and recipes as data
//! (`assets/data/crafting.ron`), Terraria's way.
//!
//! - A station is furniture (a workbench, a furnace, an anvil, an arcane
//!   altar): placed from its item like a chest, a body that stands on the
//!   ground, knocked back into its item with a pickaxe.
//! - A recipe makes an item from others (counted in their own units: blocks
//!   as blocks), by hand or at a station within reach.
//! - With the inventory open, the crafting panel lists what you can make
//!   (and, dimmed, what you're short of): a recipe shows once you've held
//!   anything that goes into it or comes out of it, and a `locked` one only
//!   once a milestone has unlocked it (`progress.rs`). Click one to make it.
//!
//! The recipes and stations written so far are a starting ladder to build
//! on: wood to planks to a workbench, stone and planks to a furnace, ore to
//! bars, bars to an anvil and on to tools, armour, hooks and foci.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use platypus_physics::{Body, Locomotion};
use serde::Deserialize;

use crate::actors::Kinematics;
use crate::actors::player::LocalPlayer;
use crate::data::{Watched, data_path, load_ron};
use crate::hands::items::{Inventory, Items, Stack};
use crate::progress::{Progress, Toast};
use crate::props::Thrown;

/// Stations draw behind creatures, beside chests.
const Z: f32 = 5.9;
/// Hit points: a few blows of a pickaxe.
const TOUGHNESS: f32 = 80.0;
/// Most recipes the panel lists.
const SHOWN: usize = 16;

#[derive(Clone, Debug, Deserialize)]
pub struct StationDef {
    pub id: String,
    pub name: String,
    /// Its picture, one character a cell (top row first; '.' clear).
    pub art: Vec<String>,
    pub palette: HashMap<char, (u8, u8, u8)>,
    /// A glow (a furnace's).
    #[serde(default)]
    pub light: Option<[f32; 3]>,
}

impl StationDef {
    pub fn size(&self) -> (i32, i32) {
        (self.art.first().map_or(1, |r| r.chars().count()) as i32, self.art.len() as i32)
    }
}

/// One recipe: what it makes (and how many), from what, where.
#[derive(Clone, Debug, Deserialize)]
pub struct Recipe {
    pub makes: String,
    #[serde(default = "one")]
    pub count: u32,
    /// (item, count in its units: blocks as blocks).
    pub needs: Vec<(String, u32)>,
    /// The station it's made at (none: by hand).
    #[serde(default)]
    pub at: Option<String>,
    /// Only once a milestone unlocks it.
    #[serde(default)]
    pub locked: bool,
}

fn one() -> u32 {
    1
}

#[derive(Clone, Debug, Deserialize)]
pub struct CraftingFile {
    /// How near a station must be (cells, to its middle).
    pub reach: f32,
    pub stations: Vec<StationDef>,
    pub recipes: Vec<Recipe>,
}

#[derive(Resource)]
pub struct Crafting {
    pub reach: f32,
    pub stations: Vec<StationDef>,
    pub recipes: Vec<Recipe>,
    art: Vec<Handle<Image>>,
    watch: Watched,
}

/// A station in the world: which, and how many more blows it takes.
#[derive(Component)]
pub struct Station {
    pub kind: usize,
    hp: f32,
}

impl Station {
    /// A pickaxe's blow; the last one breaks it.
    pub fn hit(&mut self, power: f32) -> bool {
        self.hp -= power;
        self.hp <= 0.0
    }
}

impl Crafting {
    pub fn station(&self, id: &str) -> Option<usize> {
        self.stations.iter().position(|s| s.id == id)
    }

    /// A station set down with its feet at `feet`.
    pub fn spawn(&self, commands: &mut Commands, kind: usize, feet: Vec2) {
        let def = &self.stations[kind];
        let (w, h) = def.size();
        let size = Vec2::new(w as f32, h as f32);
        let centre = feet + Vec2::new(0.0, size.y / 2.0);
        let mut e = commands.spawn((
            Name::new(def.name.clone()),
            Station { kind, hp: TOUGHNESS },
            Thrown { bounce: 0.0 },
            Kinematics { body: Body::new(centre, size), loco: Locomotion::default(), prev_pos: centre },
            Sprite::from_image(self.art[kind].clone()),
            Transform::from_translation(centre.extend(Z)),
        ));
        if let Some(c) = def.light {
            e.insert(crate::light::LightSource { color: c, flicker: 0.2 });
        }
    }

    /// Which recipes the player sees (by index), and whether each can be
    /// made now: what it has, what's near.
    pub fn listed(&self, items: &Items, inv: &Inventory, progress: &Progress, near: &[usize]) -> Vec<(usize, bool)> {
        let mut list: Vec<(usize, bool)> = self
            .recipes
            .iter()
            .enumerate()
            .filter(|(_, r)| shown(r, progress))
            .map(|(i, r)| (i, self.can_make(r, items, inv, near)))
            .collect();
        // (What can be made first.)
        list.sort_by_key(|&(i, can)| (!can, i));
        list
    }

    pub fn can_make(&self, r: &Recipe, items: &Items, inv: &Inventory, near: &[usize]) -> bool {
        let here = r.at.as_ref().is_none_or(|at| self.station(at).is_some_and(|s| near.contains(&s)));
        here && r.needs.iter().all(|(id, n)| items.id(id).is_some_and(|item| inv.count(item) >= n * items.unit(item)))
    }
}

/// Seen: anything that goes in or comes out has been held; and unlocked,
/// if it's locked.
pub fn shown(r: &Recipe, p: &Progress) -> bool {
    (!r.locked || p.unlocked.contains(&r.makes)) && (p.seen.contains(&r.makes) || r.needs.iter().any(|(id, _)| p.seen.contains(id)))
}

/// Make a recipe: what goes in comes out of the pack, and what it makes
/// (all of it, or none, if something's short).
pub fn make(r: &Recipe, items: &Items, inv: &mut Inventory) -> Option<Stack> {
    let product = items.id(&r.makes)?;
    let needs: Vec<_> = r.needs.iter().map(|(id, n)| Some((items.id(id)?, *n))).collect::<Option<_>>()?;
    if needs.iter().any(|&(item, n)| inv.count(item) < n * items.unit(item)) {
        return None;
    }
    for (item, n) in needs {
        inv.take_item(items, item, n);
    }
    Some(Stack::new(product, r.count * items.unit(product)))
}

/// A station's picture from its text art.
fn picture(def: &StationDef) -> Result<Image, String> {
    let (w, h) = def.size();
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for row in &def.art {
        if row.chars().count() as i32 != w {
            return Err(format!("station `{}`: rows of {w}", def.id));
        }
        for c in row.chars() {
            let rgba = match c {
                '.' => [0, 0, 0, 0],
                c => {
                    let &(r, g, b) = def.palette.get(&c).ok_or(format!("station `{}`: `{c}` isn't in its palette", def.id))?;
                    [r, g, b, 255]
                }
            };
            data.extend(rgba);
        }
    }
    Ok(Image::new(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD))
}

pub struct CraftPlugin;

impl Plugin for CraftPlugin {
    fn build(&self, app: &mut App) {
        let path = data_path("crafting.ron");
        let file: CraftingFile = load_ron(&path).unwrap_or_else(|e| panic!("{e}"));
        app.insert_resource(Crafting { reach: file.reach, stations: file.stations, recipes: file.recipes, art: Vec::new(), watch: Watched::new(path) })
            .add_systems(Startup, (make_art, spawn_panel))
            .add_message::<CraftRequest>()
            .add_systems(Update, (reload, show_panel, (click, craft).chain()));
    }
}

fn make_art(mut crafting: ResMut<Crafting>, mut images: ResMut<Assets<Image>>) {
    let art = crafting
        .stations
        .iter()
        .map(|d| {
            images.add(picture(d).unwrap_or_else(|e| {
                warn!("crafting.ron: {e}");
                Image::default()
            }))
        })
        .collect();
    crafting.art = art;
}

fn reload(mut crafting: ResMut<Crafting>, mut images: ResMut<Assets<Image>>) {
    if !crafting.bypass_change_detection().watch.changed() {
        return;
    }
    match load_ron::<CraftingFile>(crafting.watch.path()) {
        Ok(f) => {
            // (Stations keep their places: new ones go on the end.)
            crafting.reach = f.reach;
            crafting.recipes = f.recipes;
            for (i, d) in f.stations.into_iter().enumerate() {
                if let Ok(img) = picture(&d)
                    && let Some(h) = crafting.art.get(i).cloned()
                {
                    images.insert(&h, img).ok();
                }
                match crafting.stations.get_mut(i) {
                    Some(s) => *s = d,
                    None => crafting.stations.push(d),
                }
            }
            info!("crafting reloaded");
        }
        Err(e) => warn!("crafting not reloaded: {e}"),
    }
}

/// A station at the cursor (the blow of a pickaxe): which, and where.
pub fn station_at<'a>(at: Vec2, mut stations: impl Iterator<Item = (Entity, &'a Station, &'a Kinematics)>) -> Option<(Entity, usize, Vec2)> {
    stations.find(|(_, _, k)| ((k.body.pos - at).abs() - k.body.half).max_element() <= 2.0).map(|(e, s, k)| (e, s.kind, k.body.pos))
}

#[derive(Component)]
struct CraftRoot;

#[derive(Component)]
struct CraftList;

/// A recipe's row in the panel.
#[derive(Component)]
struct CraftRow(usize);

fn spawn_panel(mut commands: Commands) {
    commands
        .spawn((
            CraftRoot,
            Interaction::None,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                // (Under the status timers, top right.)
                top: px(180),
                right: px(8),
                width: px(300),
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                padding: UiRect::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.62)),
        ))
        .with_children(|p| {
            p.spawn((Text::new("Crafting  |  click: make one"), TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(Color::srgb(0.85, 0.85, 0.9))));
            p.spawn((CraftList, Node { flex_direction: FlexDirection::Column, row_gap: px(3), ..default() }));
        });
}

/// The stations within reach of the player.
fn near(crafting: &Crafting, at: Vec2, stations: &Query<(&Station, &Kinematics)>) -> Vec<usize> {
    stations.iter().filter(|(_, k)| k.body.pos.distance(at) <= crafting.reach).map(|(s, _)| s.kind).collect()
}

/// While the inventory is open: the recipes, rebuilt when what's listed
/// (or what can be made) changes.
#[allow(clippy::too_many_arguments)]
fn show_panel(
    mut commands: Commands,
    open: Res<crate::hands::ui::InventoryOpen>,
    crafting: Res<Crafting>,
    items: Option<Res<Items>>,
    icons: Option<Res<crate::hands::icons::Icons>>,
    player: Query<(&Kinematics, &Inventory, &Progress), With<LocalPlayer>>,
    stations: Query<(&Station, &Kinematics)>,
    mut root: Single<&mut Visibility, With<CraftRoot>>,
    list: Single<Entity, With<CraftList>>,
    mut last: Local<Vec<(usize, bool)>>,
) {
    let (Some(items), Ok((k, inv, progress))) = (items, player.single()) else { return };
    **root = if open.0 { Visibility::Visible } else { Visibility::Hidden };
    if !open.0 {
        return;
    }
    let near = near(&crafting, k.body.pos, &stations);
    let mut shown = crafting.listed(&items, inv, progress, &near);
    shown.truncate(SHOWN);
    if *last == shown && !crafting.is_changed() {
        return;
    }
    *last = shown.clone();
    commands.entity(*list).despawn_related::<Children>();
    commands.entity(*list).with_children(|l| {
        if shown.is_empty() {
            l.spawn((Text::new("(nothing yet: find wood, stone, ore)"), TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(Color::srgb(0.6, 0.6, 0.65))));
        }
        for &(i, can) in &shown {
            let r = &crafting.recipes[i];
            let Some(item) = items.id(&r.makes) else { continue };
            let name = if r.count > 1 { format!("{} x{}", items.def(item).name, r.count) } else { items.def(item).name.clone() };
            let needs: Vec<String> = r
                .needs
                .iter()
                .map(|(id, n)| {
                    let label = items.id(id).map_or(id.clone(), |it| items.def(it).name.clone());
                    format!("{n} {label}")
                })
                .collect();
            let at = r.at.as_ref().and_then(|a| crafting.station(a)).map(|s| format!("  at {}", crafting.stations[s].name)).unwrap_or_default();
            let (fg, bg) = if can { (Color::WHITE, Color::srgba(0.2, 0.3, 0.2, 0.7)) } else { (Color::srgb(0.55, 0.55, 0.6), Color::srgba(0.12, 0.12, 0.14, 0.7)) };
            l.spawn((Button, CraftRow(i), Node { column_gap: px(6), align_items: AlignItems::Center, padding: UiRect::all(px(3)), ..default() }, BackgroundColor(bg)))
                .with_children(|row| {
                    let icon = icons.as_ref().and_then(|ic| ic.get(item).cloned());
                    match icon {
                        Some(h) => row.spawn((ImageNode::new(h), Node { width: px(24), height: px(24), ..default() })),
                        None => {
                            let (r, g, b) = items.def(item).color;
                            row.spawn((Node { width: px(24), height: px(24), ..default() }, BackgroundColor(Color::srgb_u8(r, g, b))))
                        }
                    };
                    row.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|t| {
                        t.spawn((Text::new(name), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(fg)));
                        t.spawn((Text::new(format!("{}{at}", needs.join(", "))), TextFont { font_size: FontSize::Px(10.0), ..default() }, TextColor(fg.with_alpha(0.75))));
                    });
                });
        }
    });
}

/// Make a recipe (by its index), for the player: the panel's click, or a
/// script's.
#[derive(Message, Clone, Copy, Debug)]
pub struct CraftRequest(pub usize);

impl Crafting {
    /// The first recipe that makes an item.
    pub fn recipe(&self, makes: &str) -> Option<usize> {
        self.recipes.iter().position(|r| r.makes == makes)
    }
}

/// Click a recipe: ask to make it.
fn click(rows: Query<(&Interaction, &CraftRow), Changed<Interaction>>, mut asks: MessageWriter<CraftRequest>) {
    for (i, row) in &rows {
        if *i == Interaction::Pressed {
            asks.write(CraftRequest(row.0));
        }
    }
}

/// Make what's asked for, if it can be made here and now (into the pack;
/// what doesn't fit drops).
fn craft(
    mut commands: Commands,
    crafting: Res<Crafting>,
    items: Option<Res<Items>>,
    mut asks: MessageReader<CraftRequest>,
    mut player: Query<(&Kinematics, &mut Inventory, &mut Progress), With<LocalPlayer>>,
    stations: Query<(&Station, &Kinematics)>,
    mut toasts: MessageWriter<Toast>,
) {
    let Some(items) = items else { return };
    for ask in asks.read() {
        let Ok((k, mut inv, mut progress)) = player.single_mut() else { return };
        let Some(r) = crafting.recipes.get(ask.0) else { continue };
        let near = near(&crafting, k.body.pos, &stations);
        if !crafting.can_make(r, &items, &inv, &near) {
            continue;
        }
        let Some(made) = make(r, &items, &mut inv) else { continue };
        let left = inv.add(&items, made);
        if left > 0 {
            crate::hands::spawn_drop(&mut commands, &items, k.body.pos, Stack { count: left, ..made });
            toasts.write(Toast("No room: it's at your feet".into()));
        }
        *progress.crafted.entry(r.makes.clone()).or_default() += 1;
        info!("crafted {} x{}", r.makes, r.count);
    }
}

fn px(v: i32) -> Val {
    Val::Px(v as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Items, CraftingFile) {
        let items = crate::hands::items::test_items();
        let file: CraftingFile = crate::data::parse_ron(include_str!("../../../assets/data/crafting.ron")).unwrap();
        (items, file)
    }

    #[test]
    fn every_recipe_names_real_things_and_every_station_is_an_item() {
        let (items, file) = setup();
        for s in &file.stations {
            picture(s).unwrap();
            let item = items.id(&s.id).unwrap_or_else(|| panic!("no item for station `{}`", s.id));
            assert!(matches!(&items.def(item).use_, crate::hands::items::Use::Station(id) if *id == s.id), "`{}` places itself", s.id);
        }
        for r in &file.recipes {
            assert!(items.id(&r.makes).is_some(), "a recipe makes no such item `{}`", r.makes);
            for (id, _) in &r.needs {
                assert!(items.id(id).is_some(), "`{}` needs no such item `{id}`", r.makes);
            }
            if let Some(at) = &r.at {
                assert!(file.stations.iter().any(|s| &s.id == at), "`{}` is made at no such station `{at}`", r.makes);
            }
        }
    }

    #[test]
    fn making_takes_what_goes_in_all_or_nothing() {
        let (items, file) = setup();
        let bar = file.recipes.iter().find(|r| r.makes == "copper_bar").expect("a copper bar recipe");
        let ore = items.id("block:copper_ore").unwrap();
        let mut inv = Inventory::new(10);
        // Two blocks of ore: short.
        inv.add(&items, Stack::new(ore, 2 * items.unit(ore)));
        assert!(make(bar, &items, &mut inv).is_none());
        assert_eq!(inv.count(ore), 2 * items.unit(ore), "nothing taken");
        inv.add(&items, Stack::new(ore, 2 * items.unit(ore)));
        let made = make(bar, &items, &mut inv).expect("enough now");
        assert_eq!(items.def(made.item).id, "copper_bar");
        assert_eq!(inv.count(ore), items.unit(ore), "three blocks went in, one left");
        // It shows once some of it's been held; a locked one only unlocked.
        let mut p = Progress::default();
        assert!(!shown(bar, &p));
        p.seen.insert("block:copper_ore".into());
        assert!(shown(bar, &p));
        let locked = file.recipes.iter().find(|r| r.locked).expect("a locked recipe");
        p.seen.insert(locked.makes.clone());
        assert!(!shown(locked, &p));
        p.unlocked.insert(locked.makes.clone());
        assert!(shown(locked, &p));
    }
}
