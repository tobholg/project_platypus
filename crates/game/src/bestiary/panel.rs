//! The bestiary panel: F12 (or the arena panel's "Bestiary"), over
//! everything. Along the top: a search (type to search, Backspace, Esc
//! clears it, then closes), what part they play (foe, critter, villager,
//! thing) and their kinds. Below: a card each (its picture, name, kind and
//! health). Click one to open it on the right: its picture big, its body,
//! how it takes each kind of hurt, its attacks and moves, what it carries;
//! and Place (in front of you), Fight (placed, and the arena's Fight
//! readout starts afresh), Open its file, Reload (creature and move files
//! read again). While it's open it has the keyboard.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::input::InputSystems;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::{Entry, Role, catalogue, portrait};
use crate::creatures::def::Creatures;
use crate::creatures::moves::MoveBook;
use crate::dev::KeyboardTaken;

pub struct BestiaryPlugin;

impl Plugin for BestiaryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Bestiary>()
            .add_systems(PreUpdate, capture.after(InputSystems).after(crate::editor::capture))
            .add_systems(Update, (refresh, clicks, build, caption).chain());
    }
}

/// What the panel shows.
#[derive(Resource, Default)]
pub struct Bestiary {
    pub open: bool,
    pub search: String,
    pub role: Option<Role>,
    pub kind: Option<String>,
    /// The card opened (by id).
    pub picked: Option<String>,
    entries: Vec<Entry>,
    pictures: HashMap<String, (Handle<Image>, UVec2)>,
    /// What the panel was last built from (rebuilt when it changes).
    shape: String,
    stale: bool,
}

impl Bestiary {
    /// The cards the filters and search let through.
    pub fn shown(&self) -> impl Iterator<Item = &Entry> {
        self.entries
            .iter()
            .filter(|e| self.role.is_none_or(|r| r == e.role) && self.kind.as_ref().is_none_or(|k| *k == e.kind) && e.matches(&self.search))
    }
}

/// A button in the panel.
#[derive(Component, Clone, Debug, PartialEq)]
enum Do {
    Role(Option<Role>),
    Kind(Option<String>),
    Pick(String),
    Back,
    Place,
    Fight,
    Open,
    Reload,
    Close,
}

#[derive(Component)]
struct Root;

/// What the stage is showing, under its picture.
#[derive(Component)]
struct Caption;

fn caption(stage: Option<Res<super::stage::Stage>>, mut q: Query<&mut Text, With<Caption>>) {
    let Some(stage) = stage else { return };
    for mut t in &mut q {
        if t.0 != stage.caption {
            t.0.clone_from(&stage.caption);
        }
    }
}

/// While it's open: F12 or Esc (search cleared first, then a card, then
/// the panel) and typing are its; nothing else hears the keyboard.
fn capture(mut b: ResMut<Bestiary>, mut keys: ResMut<ButtonInput<KeyCode>>, mut typed: MessageReader<KeyboardInput>, mut taken: ResMut<KeyboardTaken>) {
    if keys.just_pressed(KeyCode::F12) {
        b.open = !b.open;
    }
    if !b.open {
        typed.clear();
        return;
    }
    taken.0 = true;
    if keys.just_pressed(KeyCode::Escape) {
        if !b.search.is_empty() {
            b.search.clear();
        } else if b.picked.is_some() {
            b.picked = None;
        } else {
            b.open = false;
        }
    }
    for k in typed.read().filter(|k| k.state.is_pressed()) {
        match &k.logical_key {
            Key::Backspace => {
                b.search.pop();
            }
            Key::Character(c) if c.chars().all(|c| c.is_alphanumeric() || c == '_' || c == ' ') => b.search.push_str(c),
            Key::Space => b.search.push(' '),
            _ => {}
        }
    }
    keys.reset_all();
}

/// The catalogue read again when the creatures or moves change (and the
/// pictures with it).
fn refresh(mut b: ResMut<Bestiary>, creatures: Res<Creatures>, book: Res<MoveBook>, mut images: ResMut<Assets<Image>>) {
    if !(b.entries.is_empty() || creatures.is_changed() || book.is_changed()) {
        return;
    }
    let entries = catalogue(&creatures, &book);
    let mut pictures = HashMap::new();
    for e in &entries {
        if let Some(p) = portrait(&e.def) {
            let size = UVec2::new(p.w, p.h);
            let image = Image::new(Extent3d { width: p.w, height: p.h, depth_or_array_layers: 1 }, TextureDimension::D2, p.rgba, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
            pictures.insert(e.id.clone(), (images.add(image), size));
        }
    }
    b.entries = entries;
    b.pictures = pictures;
    b.stale = true;
}

#[allow(clippy::too_many_arguments)]
fn clicks(
    mut b: ResMut<Bestiary>,
    buttons: Query<(&Interaction, &Do), Changed<Interaction>>,
    mut kind: ResMut<crate::creatures::spawn::SpawnKind>,
    mut dev: MessageWriter<crate::dev::DevAction>,
    mut fight: MessageWriter<crate::fight::NewFight>,
    mut reload: MessageWriter<crate::creatures::def::ReloadCreatures>,
) {
    for (i, d) in &buttons {
        if *i != Interaction::Pressed {
            continue;
        }
        let picked = b.picked.clone();
        match d.clone() {
            Do::Role(r) => b.role = r,
            Do::Kind(k) => b.kind = k,
            Do::Pick(id) => b.picked = Some(id),
            Do::Back => b.picked = None,
            Do::Close => b.open = false,
            Do::Place | Do::Fight => {
                let Some(id) = picked else { continue };
                // (A way in front of the player, as the panel's spawns are.)
                kind.0 = id;
                dev.write(crate::dev::DevAction::Spawn(None));
                if *d == Do::Fight {
                    fight.write(crate::fight::NewFight);
                }
                b.open = false;
            }
            Do::Open => {
                if let Some(e) = picked.and_then(|id| b.entries.iter().find(|e| e.id == id)) {
                    let opened = std::process::Command::new("open").arg("-t").arg(&e.file).spawn();
                    if let Err(err) = opened {
                        warn!("bestiary: couldn't open {}: {err}", e.file.display());
                    }
                }
            }
            Do::Reload => {
                reload.write(crate::creatures::def::ReloadCreatures);
            }
        }
    }
}

const BG: Color = Color::srgba(0.04, 0.04, 0.06, 0.985);
const CARD: Color = Color::srgba(0.13, 0.13, 0.17, 1.0);
const CARD_ON: Color = Color::srgba(0.32, 0.27, 0.12, 1.0);
const CHIP: Color = Color::srgba(0.2, 0.2, 0.26, 1.0);
const CHIP_ON: Color = Color::srgba(0.55, 0.45, 0.15, 1.0);
const DIM: Color = Color::srgb(0.6, 0.6, 0.66);
const GOLD: Color = Color::srgb(1.0, 0.85, 0.3);
/// A card's picture fits this box (px).
const CARD_PIC: (f32, f32) = (96.0, 72.0);
const BIG_PIC: (f32, f32) = (300.0, 220.0);

fn font(size: f32) -> TextFont {
    TextFont { font_size: FontSize::Px(size), ..default() }
}

/// A picture as big as fits `room` at a whole number of screen pixels a
/// cell (or less, when it won't fit at one).
fn fit(size: UVec2, room: (f32, f32)) -> (f32, f32) {
    let s = (room.0 / size.x as f32).min(room.1 / size.y as f32);
    let s = if s >= 1.0 { s.floor() } else { s };
    (size.x as f32 * s, size.y as f32 * s)
}

/// Built again whenever what it shows changes.
fn build(mut commands: Commands, mut b: ResMut<Bestiary>, stage: Option<Res<super::stage::Stage>>, roots: Query<Entity, With<Root>>, mut buttons: Query<(&Interaction, &Do, &mut BackgroundColor)>) {
    let shape = if b.open { format!("{}|{:?}|{:?}|{:?}|{}", b.search, b.role, b.kind, b.picked, b.entries.len()) } else { String::new() };
    if shape == b.shape && !b.stale {
        // Hover.
        for (i, d, mut bg) in &mut buttons {
            let on = match d {
                Do::Role(r) => *r == b.role,
                Do::Kind(k) => *k == b.kind,
                Do::Pick(id) => b.picked.as_ref() == Some(id),
                _ => false,
            };
            let base = if matches!(d, Do::Pick(_)) { if on { CARD_ON } else { CARD } } else if on { CHIP_ON } else { CHIP };
            bg.0 = if *i == Interaction::Hovered { base.lighter(0.08) } else { base };
        }
        return;
    }
    b.shape = shape;
    b.stale = false;
    for r in &roots {
        commands.entity(r).despawn();
    }
    if !b.open {
        return;
    }
    let shown: Vec<&Entry> = b.shown().collect();
    let mut kinds: Vec<String> = b.entries.iter().map(|e| e.kind.clone()).collect();
    kinds.sort();
    kinds.dedup();
    let chip = |p: &mut ChildSpawnerCommands, text: &str, d: Do, on: bool| {
        p.spawn((Button, d, Node { padding: UiRect::axes(px(8), px(3)), ..default() }, BackgroundColor(if on { CHIP_ON } else { CHIP })))
            .with_children(|c| {
                c.spawn((Text::new(text), font(12.0), TextColor(Color::WHITE)));
            });
    };
    let picture = |p: &mut ChildSpawnerCommands, id: &str, room: (f32, f32)| {
        p.spawn(Node { width: px(room.0), height: px(room.1), justify_content: JustifyContent::Center, align_items: AlignItems::End, ..default() }).with_children(|c| {
            if let Some((h, size)) = b.pictures.get(id) {
                let (w, hh) = fit(*size, room);
                c.spawn((ImageNode::new(h.clone()), Node { width: px(w), height: px(hh), ..default() }));
            }
        });
    };
    commands
        .spawn((Root, GlobalZIndex(100), Node { position_type: PositionType::Absolute, left: px(0), right: px(0), top: px(0), bottom: px(0), flex_direction: FlexDirection::Column, padding: UiRect::all(px(14)), row_gap: px(8), ..default() }, BackgroundColor(BG)))
        .with_children(|root| {
            // The top: title, search, filters.
            root.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(14), align_items: AlignItems::Center, ..default() }).with_children(|top| {
                top.spawn((Text::new(format!("BESTIARY  {} of {}", shown.len(), b.entries.len())), font(16.0), TextColor(GOLD)));
                let search = if b.search.is_empty() { "type to search".to_string() } else { format!("{}_", b.search) };
                top.spawn((Text::new(format!("Search: {search}")), font(13.0), TextColor(if b.search.is_empty() { DIM } else { Color::WHITE })));
                top.spawn(Node { flex_grow: 1.0, ..default() });
                chip(top, "Close  F12 / Esc", Do::Close, false);
            });
            root.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(4), row_gap: px(4), align_items: AlignItems::Center, ..default() }).with_children(|row| {
                chip(row, "all", Do::Role(None), b.role.is_none());
                for r in Role::ALL {
                    chip(row, r.name(), Do::Role(Some(r)), b.role == Some(r));
                }
                row.spawn(Node { width: px(16), ..default() });
                chip(row, "every kind", Do::Kind(None), b.kind.is_none());
                for k in &kinds {
                    chip(row, k, Do::Kind(Some(k.clone())), b.kind.as_ref() == Some(k));
                }
            });
            // The cards, and the one opened beside them.
            root.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(12), flex_grow: 1.0, min_height: px(0), ..default() }).with_children(|main| {
                main.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, align_content: AlignContent::FlexStart, column_gap: px(6), row_gap: px(6), flex_grow: 1.0, overflow: Overflow::scroll_y(), ..default() }).with_children(|grid| {
                    for e in &shown {
                        let on = b.picked.as_deref() == Some(e.id.as_str());
                        grid.spawn((Button, Do::Pick(e.id.clone()), Node { width: px(112), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, padding: UiRect::all(px(6)), row_gap: px(3), ..default() }, BackgroundColor(if on { CARD_ON } else { CARD })))
                            .with_children(|card| {
                                picture(card, &e.id, CARD_PIC);
                                card.spawn((Text::new(e.name.clone()), font(12.0), TextColor(Color::WHITE)));
                                let code = if e.code.is_some() { ", code" } else { "" };
                                card.spawn((Text::new(format!("{}, {:.0} hp{code}", e.kind, e.health)), font(10.0), TextColor(DIM)));
                            });
                    }
                    if shown.is_empty() {
                        grid.spawn((Text::new("Nothing matches."), font(13.0), TextColor(DIM)));
                    }
                });
                let Some(e) = b.picked.as_ref().and_then(|id| b.entries.iter().find(|e| &e.id == id)) else { return };
                // (Wider with the live stage in it.)
                let live = stage.as_ref().map(|s| s.image.clone());
                let width = if live.is_some() { 560.0 } else { 380.0 };
                main.spawn((Node { width: px(width), flex_direction: FlexDirection::Column, row_gap: px(6), padding: UiRect::all(px(10)), overflow: Overflow::scroll_y(), ..default() }, BackgroundColor(CARD))).with_children(|side| {
                    side.spawn((Text::new(e.name.clone()), font(18.0), TextColor(GOLD)));
                    side.spawn((Text::new(format!("{}.ron", e.id)), font(10.0), TextColor(DIM)));
                    match &live {
                        // The live stage: the real thing, doing what it does.
                        Some(image) => {
                            side.spawn((ImageNode::new(image.clone()), Node { width: px(540), height: px(540.0 * 312.0 / 660.0), ..default() }));
                            side.spawn((Caption, Text::new(""), font(11.0), TextColor(DIM)));
                        }
                        None => picture(side, &e.id, BIG_PIC),
                    }
                    side.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(4), row_gap: px(4), ..default() }).with_children(|row| {
                        chip(row, "Place", Do::Place, false);
                        chip(row, "Fight", Do::Fight, false);
                        chip(row, "Open file", Do::Open, false);
                        chip(row, "Reload", Do::Reload, false);
                        chip(row, "Back", Do::Back, false);
                    });
                    for (head, lines) in e.details() {
                        side.spawn((Text::new(head), font(12.0), TextColor(GOLD)));
                        side.spawn((Text::new(lines.join("\n")), font(11.0), TextColor(Color::WHITE), Node { max_width: px(width - 20.0), ..default() }));
                    }
                });
            });
        });
}
