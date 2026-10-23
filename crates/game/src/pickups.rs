//! "Picked up": what the player takes shows over its head a while, a line
//! a thing (its icon, its name in its colour, how many: "Copper ore (9)"),
//! Terraria's way.
//! More of the same within `KEEP` counts up on its line instead of making
//! another (the line pops); gold too, so sweeping a heap counts it in. Gear
//! is named by its roll, in its rarity's colour; the first of anything the
//! player ever holds says so (`new`). Its pack full, a red "Inventory full"
//! instead. Newest at the bottom, nearest the head; at most `LINES`. A line
//! rises into place from below as it fades in, and when its time is up (or
//! a new one comes with the lines full, pushing the oldest out) it rises on
//! out of the top as it fades.
//!
//! `hands::collect` and `gold` say what was taken (`PickedUp`).

use bevy::prelude::*;

use crate::camera::MainCamera;
use crate::creatures::Kinematics;
use crate::creatures::player::LocalPlayer;
use crate::gear::GearRules;
use crate::hands::icons::Icons;
use crate::hands::items::{Items, Stack};

/// A line stays this long after its last pick-up (seconds).
const KEEP: f32 = 2.4;
/// It fades in this long, and out (rising as it fades) this long.
const FADE_IN: f32 = 0.2;
const FADE_OUT: f32 = 0.3;
/// Most lines at once (a new one pushes the oldest out).
const LINES: usize = 3;
/// A line's height, how far below its place it comes in from, how far up
/// it goes out (screen pixels), and how quickly it gets to where it's going.
const LINE_H: f32 = 18.0;
const SLIDE_IN: f32 = 10.0;
const SLIDE_OUT: f32 = 14.0;
const GLIDE: f32 = 14.0;
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
#[derive(Message, Clone, Debug)]
pub enum PickedUp {
    /// An item; `new`: the first of it the player has ever held.
    Item { stack: Stack, new: bool },
    /// Gold (cells: coins).
    Gold(u32),
    /// Something it walked up to didn't fit.
    Full,
    /// Gold lost (left where it died): a red count down.
    Lost(u32),
    /// A word of its own over the head ("Pick up: Valuables"): one at a
    /// time, the next replacing it.
    Note(String),
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
    Note,
    Lost,
}

impl What {
    fn is(&self, other: &What) -> bool {
        match (self, other) {
            (What::Item(a), What::Item(b)) => a.same(b),
            (What::Gold, What::Gold) | (What::Full, What::Full) | (What::Note, What::Note) | (What::Lost, What::Lost) => true,
            _ => false,
        }
    }
}

#[derive(Component)]
struct Line {
    what: What,
    /// How many (units: cells for blocks and gold).
    units: u32,
    /// Since its last pick-up, and since it came.
    age: f32,
    shown: f32,
    pop: f32,
    /// Its order (the oldest goes first).
    born: u32,
    /// How high it is (screen pixels over the bottom of the box).
    y: f32,
    /// Going: how long it's been going, from how high.
    going: Option<(f32, f32)>,
    text: Entity,
    icon: Entity,
}

fn spawn_root(mut commands: Commands) {
    commands.spawn((
        Root,
        Name::new("Picked up"),
        Node { position_type: PositionType::Absolute, width: px(WIDTH), height: px(LINE_H * (LINES as f32 + 1.0)), ..default() },
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

/// A line's words and colour (a note's are its own).
fn words(what: &What, units: u32, items: &Items, rules: Option<&GearRules>) -> (String, Color) {
    match what {
        What::Note => (String::new(), Color::srgb(0.85, 0.92, 1.0)),
        What::Gold => (format!("{units} gold"), readable(GOLD)),
        What::Full => ("Inventory full".into(), readable(FULL)),
        What::Lost => (format!("-{units} gold"), readable(FULL)),
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
pub(crate) fn gold_icon(images: &mut Assets<Image>) -> Handle<Image> {
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
    let mut note: Option<String> = None;
    for p in picked.read() {
        let (what, units, new) = match p {
            PickedUp::Item { stack, new } => (What::Item(*stack), stack.count, *new),
            PickedUp::Gold(n) => (What::Gold, *n, false),
            PickedUp::Full => (What::Full, 0, false),
            PickedUp::Lost(n) => (What::Lost, *n, false),
            PickedUp::Note(s) => {
                note = Some(s.clone());
                (What::Note, 0, false)
            }
        };
        match got.iter_mut().find(|g| g.0.is(&what)) {
            Some(g) => {
                g.1 = g.1.saturating_add(units);
                g.2 |= new;
            }
            None => got.push((what, units, new)),
        }
    }
    // (A note's words are its own: the last one said.)
    let say = |what: &What, units: u32| {
        let (w, c) = words(what, units, &items, rules.as_deref());
        if *what == What::Note { (note.clone().unwrap_or_default(), c) } else { (w, c) }
    };
    let mut spawned = 0;
    for (what, units, new) in got {
        if let Some((_, mut line)) = lines.iter_mut().find(|(_, l)| l.what.is(&what) && l.going.is_none()) {
            line.units = line.units.saturating_add(units);
            line.age = 0.0;
            line.pop = if matches!(what, What::Full | What::Note) { line.pop } else { 1.0 };
            if let Ok((mut text, mut color)) = texts.get_mut(line.text) {
                let (w, c) = say(&line.what, line.units);
                text.0 = w;
                color.0 = c;
            }
            continue;
        }
        // The lines full: the oldest is pushed out.
        if lines.iter().filter(|(_, l)| l.going.is_none()).count() + spawned >= LINES
            && let Some((_, mut oldest)) = lines.iter_mut().filter(|(_, l)| l.going.is_none()).min_by_key(|(_, l)| l.born)
        {
            oldest.going = Some((0.0, oldest.y));
        }
        let (w, c) = say(&what, units);
        let image = match what {
            What::Item(s) => icons.as_ref().and_then(|i| i.get(s.item).cloned()),
            What::Gold | What::Lost => Some(gold.get_or_insert_with(|| gold_icon(&mut images)).clone()),
            What::Full | What::Note => None,
        };
        let swatch = match what {
            What::Item(s) if image.is_none() => {
                let (r, g, b) = items.def(s.item).color;
                Color::srgb_u8(r, g, b)
            }
            _ => Color::NONE,
        };
        let size = if what == What::Gold { Vec2::new(10.0, 8.0) } else if image.is_some() { Vec2::splat(ICON_PX) } else { Vec2::splat(8.0) };
        let shown = !matches!(what, What::Full | What::Note);
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
                Line { what, units, age: 0.0, shown: 0.0, pop: 0.0, born: *born, y: -SLIDE_IN, going: None, text, icon },
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.0),
                    width: percent(100.0),
                    bottom: px(-SLIDE_IN),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: px(4.0),
                    ..default()
                },
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

type Fading<'a> = (Entity, &'a mut Line, &'a mut Node, &'a mut UiTransform, &'a Children);

/// Lines glide to their places (newest lowest), fade in as they come and
/// settle from their pop; their time up, they go: up and out, fading.
fn age(mut commands: Commands, time: Res<Time>, mut lines: Query<Fading>, mut colors: Query<&mut TextColor>, mut icons: Query<(&mut ImageNode, &mut BackgroundColor)>) {
    let dt = time.delta_secs();
    // (Places: the staying, newest first.)
    let mut staying: Vec<(u32, Entity)> = lines.iter().filter(|l| l.1.going.is_none()).map(|l| (l.1.born, l.0)).collect();
    staying.sort_unstable_by_key(|s| std::cmp::Reverse(s.0));
    let glide = 1.0 - (-GLIDE * dt).exp();
    for (e, mut line, mut node, mut tf, children) in &mut lines {
        line.age += dt;
        line.shown += dt;
        if line.going.is_none() && line.age >= KEEP {
            line.going = Some((0.0, line.y));
        }
        let alpha = match &mut line.going {
            Some((t, from)) => {
                *t += dt;
                if *t >= FADE_OUT {
                    commands.entity(e).despawn();
                    continue;
                }
                // (Up and out over the top of the lines, clear of those
                // staying, in the order they were in: rising and fading
                // together.)
                let k = *t / FADE_OUT;
                let to = (LINES as f32 - 1.0) * LINE_H + SLIDE_OUT + *from * 0.5;
                let from = *from;
                line.y = from + (to - from) * k * (2.0 - k);
                1.0 - k
            }
            None => {
                let slot = staying.iter().position(|s| s.1 == e).unwrap_or(0);
                line.y += (slot as f32 * LINE_H - line.y) * glide;
                (line.shown / FADE_IN).min(1.0)
            }
        };
        node.bottom = px(line.y);
        line.pop = (line.pop - POP_SETTLE * dt * line.pop.max(0.15)).max(0.0);
        tf.scale = Vec2::splat(1.0 + POP * line.pop);
        for &c in children {
            if let Ok(mut color) = colors.get_mut(c) {
                color.0.set_alpha(alpha);
            }
        }
        // (An icon or swatch that's there at all: never quite clear, so it
        // stays told apart from one that isn't.)
        if let Ok((mut image, mut bg)) = icons.get_mut(line.icon) {
            if image.color != Color::NONE {
                image.color.set_alpha(alpha.max(0.001));
            }
            if bg.0 != Color::NONE {
                bg.0.set_alpha(alpha.max(0.001));
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
