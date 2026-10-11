//! "Picked up": what the player takes shows over its head a while, a line
//! a thing (its icon, its name in its colour, how many: "Copper ore (9)"),
//! Terraria's way.
//! More of the same within `KEEP` counts up on its line instead of making
//! another (the line pops); gold too, so sweeping a heap counts it in. Gear
//! is named by its roll, in its rarity's colour; the first of anything the
//! player ever holds says so (`new`). Its pack full, a red "Inventory full"
//! instead. Newest at the bottom, nearest the head; at most `LINES`.
//!
//! `hands::collect` and `gold` say what was taken (`PickedUp`).

use bevy::prelude::*;

use crate::camera::MainCamera;
use crate::creatures::Kinematics;
use crate::creatures::player::LocalPlayer;
use crate::gear::GearRules;
use crate::hands::icons::Icons;
use crate::hands::items::{Items, Stack};

/// A line stays this long after its last pick-up (seconds), fading at the end.
const KEEP: f32 = 3.0;
const FADE: f32 = 0.6;
/// Most lines at once (the oldest goes).
const LINES: usize = 5;
/// The pop of a line counting up: how big, how fast it settles.
const POP: f32 = 0.22;
const POP_SETTLE: f32 = 7.0;
/// Over the head (world cells).
const ABOVE: f32 = 9.0;
const FONT: f32 = 13.0;
const ICON_PX: f32 = 16.0;
/// The line's box (its middle is over the head).
const WIDTH: f32 = 360.0;
/// Gold's colour, the "new" mark's, the full pack's.
const GOLD: (u8, u8, u8) = (255, 208, 72);
const NEW: (u8, u8, u8) = (255, 244, 200);
const FULL: (u8, u8, u8) = (255, 90, 80);
/// Names are never darker than this (coal on a dark cave stays readable).
const LIGHTEST_DARK: f32 = 0.55;

/// What the local player just took.
#[derive(Message, Clone, Copy, Debug)]
pub enum PickedUp {
    /// An item; `new`: the first of it the player has ever held.
    Item { stack: Stack, new: bool },
    /// Gold (cells: coins).
    Gold(u32),
    /// Something it walked up to didn't fit.
    Full,
}

pub struct PickupsPlugin;

impl Plugin for PickupsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PickedUp>()
            .add_systems(Startup, spawn_root)
            .add_systems(Update, (take_in, age).chain())
            .add_systems(PostUpdate, place.after(crate::camera::follow).before(bevy::ui::UiSystems::Layout));
    }
}

/// The box the lines stand in, over the player's head.
#[derive(Component)]
struct Root;

/// What a line counts: an item (one stack's kind: item, roll, contents), gold, or the full pack.
#[derive(Clone, Copy, PartialEq)]
enum What {
    Item(Stack),
    Gold,
    Full,
}

impl What {
    fn is(&self, other: &What) -> bool {
        match (self, other) {
            (What::Item(a), What::Item(b)) => a.same(b),
            (What::Gold, What::Gold) | (What::Full, What::Full) => true,
            _ => false,
        }
    }
}

#[derive(Component)]
struct Line {
    what: What,
    /// How many (units: cells for blocks and gold).
    units: u32,
    age: f32,
    pop: f32,
    /// Its order (the oldest goes first).
    born: u32,
    text: Entity,
    icon: Entity,
}

fn spawn_root(mut commands: Commands) {
    commands.spawn((
        Root,
        Name::new("Picked up"),
        Node {
            position_type: PositionType::Absolute,
            width: px(WIDTH),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::FlexEnd,
            row_gap: px(1.0),
            ..default()
        },
        Pickable::IGNORE,
        GlobalZIndex(5),
    ));
}

fn shadow() -> TextShadow {
    TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85) }
}

/// A colour lifted toward white until it reads on the dark.
fn readable((r, g, b): (u8, u8, u8)) -> Color {
    let c = Vec3::new(r as f32, g as f32, b as f32) / 255.0;
    let lum = c.dot(Vec3::new(0.299, 0.587, 0.114));
    let k = if lum < LIGHTEST_DARK { (LIGHTEST_DARK - lum) / (1.0 - lum).max(0.01) } else { 0.0 };
    let c = c.lerp(Vec3::ONE, k);
    Color::srgb(c.x, c.y, c.z)
}

/// How many, as the pack shows them: whole blocks (less than one, a bit).
fn amount(units: u32, unit: u32) -> String {
    match units / unit.max(1) {
        0 => "a bit".into(),
        n => format!("{n}"),
    }
}

/// A line's words and colour.
fn words(what: &What, units: u32, items: &Items, rules: Option<&GearRules>) -> (String, Color) {
    match what {
        What::Gold => (format!("{units} gold"), readable(GOLD)),
        What::Full => ("Inventory full".into(), readable(FULL)),
        What::Item(s) => {
            let def = items.def(s.item);
            let rarity = rules.and_then(|r| r.rarity(items, s));
            let name = match (rules, rarity) {
                (Some(r), Some(_)) => r.name(items, s),
                _ => def.name.clone(),
            };
            let color = readable(rarity.map_or(def.color, |r| r.color));
            let unit = items.unit(s.item);
            if units == unit { (name, color) } else { (format!("{name} ({})", amount(units, unit)), color) }
        }
    }
}

/// Gold's nugget (`gold::ICON`) as an image.
fn gold_icon(images: &mut Assets<Image>) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let (w, h) = (crate::gold::ICON[0].len(), crate::gold::ICON.len());
    let data = crate::gold::ICON.iter().flat_map(|r| r.chars()).flat_map(|c| crate::gold::art_color(c).unwrap_or([0; 4])).collect();
    let mut image = Image::new(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default());
    image.sampler = bevy::image::ImageSampler::nearest();
    images.add(image)
}

type LineParts<'a> = (&'a mut Text, &'a mut TextColor);

/// New pick-ups: counted onto their line if it's still up, else a line of their own.
#[allow(clippy::too_many_arguments)]
fn take_in(
    mut commands: Commands,
    mut picked: MessageReader<PickedUp>,
    items: Option<Res<Items>>,
    rules: Option<Res<GearRules>>,
    icons: Option<Res<Icons>>,
    mut images: ResMut<Assets<Image>>,
    mut gold: Local<Option<Handle<Image>>>,
    mut born: Local<u32>,
    root: Single<Entity, With<Root>>,
    mut lines: Query<(Entity, &mut Line)>,
    mut texts: Query<LineParts>,
) {
    let Some(items) = items else {
        picked.clear();
        return;
    };
    // (What came this frame put together first: three ores at once are one
    // line of three.)
    let mut got: Vec<(What, u32, bool)> = Vec::new();
    for p in picked.read() {
        let (what, units, new) = match *p {
            PickedUp::Item { stack, new } => (What::Item(stack), stack.count, new),
            PickedUp::Gold(n) => (What::Gold, n, false),
            PickedUp::Full => (What::Full, 0, false),
        };
        match got.iter_mut().find(|g| g.0.is(&what)) {
            Some(g) => {
                g.1 = g.1.saturating_add(units);
                g.2 |= new;
            }
            None => got.push((what, units, new)),
        }
    }
    let mut gone: Vec<Entity> = Vec::new();
    let mut spawned = 0;
    for (what, units, new) in got {
        // (A gone line still fading is counted onto: it comes back.)
        if let Some((_, mut line)) = lines.iter_mut().find(|(e, l)| l.what.is(&what) && !gone.contains(e)) {
            line.units = line.units.saturating_add(units);
            line.age = 0.0;
            line.pop = if what == What::Full { line.pop } else { 1.0 };
            if let Ok((mut text, mut color)) = texts.get_mut(line.text) {
                let (w, c) = words(&line.what, line.units, &items, rules.as_deref());
                text.0 = w;
                color.0 = c;
            }
            continue;
        }
        // The oldest goes to make room.
        if lines.iter().filter(|(e, _)| !gone.contains(e)).count() + spawned >= LINES
            && let Some((e, _)) = lines.iter().filter(|(e, _)| !gone.contains(e)).min_by_key(|(_, l)| l.born)
        {
            commands.entity(e).despawn();
            gone.push(e);
        }
        let (w, c) = words(&what, units, &items, rules.as_deref());
        let image = match what {
            What::Item(s) => icons.as_ref().and_then(|i| i.get(s.item).cloned()),
            What::Gold => Some(gold.get_or_insert_with(|| gold_icon(&mut images)).clone()),
            What::Full => None,
        };
        let swatch = match what {
            What::Item(s) if image.is_none() => {
                let (r, g, b) = items.def(s.item).color;
                Color::srgb_u8(r, g, b)
            }
            _ => Color::NONE,
        };
        let size = if what == What::Gold { Vec2::new(10.0, 8.0) } else if image.is_some() { Vec2::splat(ICON_PX) } else { Vec2::splat(8.0) };
        let shown = what != What::Full;
        let icon = commands
            .spawn((
                ImageNode { image: image.unwrap_or_default(), color: if shown { Color::WHITE } else { Color::NONE }, ..default() },
                BackgroundColor(swatch),
                Node { width: px(size.x), height: px(size.y), display: if shown { Display::Flex } else { Display::None }, ..default() },
            ))
            .id();
        let text = commands.spawn((Text::new(w), TextFont { font_size: FontSize::Px(FONT), ..default() }, TextColor(c), shadow())).id();
        let row = commands
            .spawn((
                Line { what, units, age: 0.0, pop: 0.6, born: *born, text, icon },
                Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(4.0), ..default() },
                UiTransform::IDENTITY,
            ))
            .add_children(&[icon, text])
            .id();
        if new {
            let mark = commands.spawn((Text::new("new"), TextFont { font_size: FontSize::Px(FONT - 3.0), ..default() }, TextColor(readable(NEW)), shadow())).id();
            commands.entity(row).add_child(mark);
        }
        commands.entity(*root).add_child(row);
        *born = born.wrapping_add(1);
        spawned += 1;
    }
}

type Fading<'a> = (Entity, &'a mut Line, &'a mut UiTransform, &'a Children);

/// Lines settle from their pop, fade at the end of their time, and go.
fn age(mut commands: Commands, time: Res<Time>, mut lines: Query<Fading>, mut colors: Query<&mut TextColor>, mut icons: Query<(&mut ImageNode, &mut BackgroundColor)>) {
    let dt = time.delta_secs();
    for (e, mut line, mut tf, children) in &mut lines {
        line.age += dt;
        if line.age >= KEEP {
            commands.entity(e).despawn();
            continue;
        }
        line.pop = (line.pop - POP_SETTLE * dt * line.pop.max(0.15)).max(0.0);
        tf.scale = Vec2::splat(1.0 + POP * line.pop);
        let alpha = ((KEEP - line.age) / FADE).min(1.0);
        for &c in children {
            if let Ok(mut color) = colors.get_mut(c) {
                color.0.set_alpha(alpha);
            }
        }
        if let Ok((mut image, mut bg)) = icons.get_mut(line.icon) {
            if image.color.alpha() > 0.0 {
                image.color.set_alpha(alpha);
            }
            if bg.0.alpha() > 0.0 {
                bg.0.set_alpha(alpha);
            }
        }
    }
}

/// The box over the player's head, where the camera now shows it.
fn place(
    player: Query<(&Transform, &Kinematics), With<LocalPlayer>>,
    camera: Single<(&Camera, &Transform), With<MainCamera>>,
    mut root: Single<(&mut Node, &mut Visibility), With<Root>>,
) {
    let (cam, cam_tf) = *camera;
    let (node, vis) = &mut *root;
    let at = player.single().ok().and_then(|(t, k)| {
        let head = t.translation.truncate() + Vec2::new(0.0, k.body.half.y + ABOVE);
        let size = cam.logical_viewport_size()?;
        let p = cam.world_to_viewport(&GlobalTransform::from(*cam_tf), head.extend(0.0)).ok()?;
        Some((p, size))
    });
    let Some((p, size)) = at else {
        **vis = Visibility::Hidden;
        return;
    };
    **vis = Visibility::Inherited;
    node.left = px(p.x - WIDTH / 2.0);
    node.bottom = px(size.y - p.y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_count_in_whole_blocks_as_the_pack_shows_them() {
        assert_eq!(amount(3, 1), "3");
        assert_eq!(amount(72, 36), "2");
        assert_eq!(amount(54, 36), "1");
        assert_eq!(amount(18, 36), "a bit");
    }

    #[test]
    fn dark_names_are_lifted_bright_ones_kept() {
        let lum = |c: Color| {
            let s = c.to_srgba();
            0.299 * s.red + 0.587 * s.green + 0.114 * s.blue
        };
        assert!(lum(readable((30, 30, 34))) >= LIGHTEST_DARK - 0.01);
        assert_eq!(readable((255, 208, 72)).to_srgba(), Color::srgb_u8(255, 208, 72).to_srgba());
    }
}
