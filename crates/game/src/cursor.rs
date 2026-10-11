//! The game's own cursor, in place of the system's: one picture that says
//! what a click does where it is, and a little of what's there.
//!
//! - An arrow by default (and over the pack, the panels, the dev tools).
//! - A crosshair with a weapon, a bow, a focus or something to throw; a
//!   ring round it fills as a bow draws or a focus's spell comes back, red
//!   while there isn't the mana for it.
//! - Brackets with a mining tool; a word under it for what's there, and in
//!   red what it would take when it's too hard ("needs a gold pickaxe"; the
//!   block is lit red: `hands::mark`).
//! - A hand over what a right-click opens, near enough: a chest, a body,
//!   one of the village folk (with its name, or the body's), or something
//!   on the ground the pick-up filter leaves ("right-click to take").
//! - Something to throw: where it will fly, dotted, to where it lands.
//! - An eye beside it, out of the light: how plain you are to see and hear
//!   (shut: dark and quiet; half; open: lit or loud). Never anything about
//!   creatures: that waits for the bestiary.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::{CursorOptions, PrimaryWindow};
use platypus_physics::Body;
use platypus_sim::{BLOCK, CellPos};

use crate::camera::CursorWorld;
use crate::creatures::Kinematics;
use crate::creatures::brain::senses::Exposure;
use crate::creatures::player::LocalPlayer;
use crate::hands::items::{Inventory, Items, Use};
use crate::hands::{DevTools, Hand, InventoryOpen};
use crate::world::{SimWorld, TICK_HZ};

/// Screen pixels to a picture's pixel.
const SCALE: f32 = 2.0;
const RING_STEPS: usize = 24;
const RING_SIZE: usize = 21;
const FONT: f32 = 12.0;
/// Where the words sit from the cursor's point (screen pixels).
const LABEL_AT: Vec2 = Vec2::new(16.0, 14.0);
/// Where the eye sits from the cursor's point.
const EYE_AT: Vec2 = Vec2::new(-26.0, 10.0);
/// The eye shows below this much light, or crouched.
const EYE_UNDER: f32 = 0.85;
/// The throw's flight, followed this long at most (seconds), a dot every
/// `DOT_EVERY` ticks.
const ARC_SECS: f32 = 2.0;
const DOT_EVERY: usize = 3;
const DOTS: usize = 40;
/// Over the dark and the mining mark.
const Z_ARC: f32 = 16.2;
const DT: f32 = (1.0 / TICK_HZ) as f32;

const OUTLINE: [u8; 4] = [16, 16, 20, 255];
const WHITE: [u8; 4] = [240, 240, 232, 255];
const PUPIL: [u8; 4] = [40, 40, 52, 255];

const ARROW: &[&str] = &[
    "o........", "oo.......", "owo......", "owwo.....", "owwwo....", "owwwwo...", "owwwwwo..", "owwwwwwo.", "owwwwoooo", "owoowo...", "oo..owo..", ".....oo..",
];
const CROSS: &[&str] = &[
    ".....ooo.....", ".....owo.....", ".....owo.....", ".....ooo.....", ".............", "oooo..o..oooo", "owwo.owo.owwo", "oooo..o..oooo", ".............", ".....ooo.....", ".....owo.....", ".....owo.....", ".....ooo.....",
];
const BRACKETS: &[&str] = &[
    "oooo...oooo", "owwo...owwo", "owoo...oowo", "oo.......oo", "...........", ".....o.....", "...........", "oo.......oo", "owoo...oowo", "owwo...owwo", "oooo...oooo",
];
const HAND: &[&str] = &[
    "....oo......", "...owwo.....", "...owwo.....", "...owwooo...", "...owwowwoo.", "...owwowwowo", "ooooowwwwwwo", "owwowwwwwwwo", "owwwwwwwwwwo", ".owwwwwwwwwo", "..owwwwwwwo.", "...owwwwwwo.", "....oooooo..",
];
const EYE_SHUT: &[&str] = &[".........", ".........", "ooooooooo", ".o.o.o.o.", "........."];
const EYE_HALF: &[&str] = &[".........", ".ooooooo.", "owwwpwwwo", ".oowwwoo.", "...ooo..."];
const EYE_OPEN: &[&str] = &["...ooo...", ".oowwwoo.", "owwwpwwwo", ".oowwwoo.", "...ooo..."];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shape {
    Arrow,
    Cross,
    Brackets,
    Hand,
}

impl Shape {
    /// Where its point is, in its pixels.
    fn hotspot(self) -> Vec2 {
        match self {
            Shape::Arrow => Vec2::ZERO,
            Shape::Cross => Vec2::new(6.5, 6.5),
            Shape::Brackets => Vec2::new(5.5, 5.5),
            Shape::Hand => Vec2::new(4.5, 0.5),
        }
    }
}

#[derive(Resource)]
struct Art {
    shapes: [(Handle<Image>, Vec2); 4],
    rings: Vec<Handle<Image>>,
    eyes: [(Handle<Image>, Vec2); 3],
}

#[derive(Component)]
struct Pointer;
#[derive(Component)]
struct Ring;
#[derive(Component)]
struct Label;
#[derive(Component)]
struct Eye;
#[derive(Component)]
struct ArcDot(usize);

pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (make_art, spawn).chain()).add_systems(Update, (hide_system_cursor, point, throw_arc));
    }
}

/// A picture from text rows: 'o' outline, 'w' white, 'p' a pupil.
fn draw(rows: &[&str], images: &mut Assets<Image>) -> (Handle<Image>, Vec2) {
    let (w, h) = (rows[0].len(), rows.len());
    let data = rows
        .iter()
        .flat_map(|r| r.chars())
        .flat_map(|c| match c {
            'o' => OUTLINE,
            'w' => WHITE,
            'p' => PUPIL,
            _ => [0; 4],
        })
        .collect();
    (images.add(image(w, h, data)), Vec2::new(w as f32, h as f32))
}

fn image(w: usize, h: usize, data: Vec<u8>) -> Image {
    let mut image = Image::new(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default());
    image.sampler = bevy::image::ImageSampler::nearest();
    image
}

/// A ring `done` of the way round (clockwise from the top): white, with a
/// dark outline, over a faint whole ring.
fn ring(done: f32) -> Image {
    let n = RING_SIZE;
    let c = (n as f32 - 1.0) / 2.0;
    let r = c - 1.5;
    let mut data = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let d = Vec2::new(x as f32 - c, c - y as f32);
            let off = (d.length() - r).abs();
            // (Clockwise from the top, 0..1.)
            let turn = (d.x.atan2(d.y) / std::f32::consts::TAU).rem_euclid(1.0);
            let lit = turn <= done || done >= 1.0;
            let px = if off < 0.75 {
                if lit { WHITE } else { [255, 255, 255, 60] }
            } else if off < 1.6 && lit {
                OUTLINE
            } else {
                [0; 4]
            };
            data[(y * n + x) * 4..(y * n + x) * 4 + 4].copy_from_slice(&px);
        }
    }
    image(n, n, data)
}

fn make_art(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let shapes = [draw(ARROW, &mut images), draw(CROSS, &mut images), draw(BRACKETS, &mut images), draw(HAND, &mut images)];
    let rings = (0..=RING_STEPS).map(|i| images.add(ring(i as f32 / RING_STEPS as f32))).collect();
    let eyes = [draw(EYE_SHUT, &mut images), draw(EYE_HALF, &mut images), draw(EYE_OPEN, &mut images)];
    commands.insert_resource(Art { shapes, rings, eyes });
}

fn spawn(mut commands: Commands, art: Res<Art>) {
    let abs = |size: Vec2| Node { position_type: PositionType::Absolute, width: px(size.x * SCALE), height: px(size.y * SCALE), ..default() };
    let (arrow, size) = art.shapes[0].clone();
    commands.spawn((Name::new("Cursor"), Pointer, ImageNode::new(arrow), abs(size), GlobalZIndex(1000), Pickable::IGNORE));
    let ring = RING_SIZE as f32;
    commands.spawn((Name::new("Cursor ring"), Ring, ImageNode::new(art.rings[0].clone()), abs(Vec2::splat(ring)), GlobalZIndex(999), Visibility::Hidden, Pickable::IGNORE));
    let (eye, size) = art.eyes[0].clone();
    commands.spawn((Name::new("Cursor eye"), Eye, ImageNode::new(eye), abs(size), GlobalZIndex(999), Visibility::Hidden, Pickable::IGNORE));
    commands.spawn((
        Name::new("Cursor words"),
        Label,
        Text::new(""),
        TextFont { font_size: FontSize::Px(FONT), ..default() },
        TextColor(Color::WHITE),
        TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85) },
        Node { position_type: PositionType::Absolute, ..default() },
        GlobalZIndex(999),
        Pickable::IGNORE,
    ));
    for i in 0..DOTS {
        commands.spawn((Name::new("Throw arc"), ArcDot(i), Sprite::from_color(Color::WHITE, Vec2::splat(2.0)), Transform::from_xyz(0.0, 0.0, Z_ARC), Visibility::Hidden));
    }
}

/// The system's cursor goes (ours is drawn in its place).
fn hide_system_cursor(mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    for mut c in &mut cursor {
        if c.visible {
            c.visible = false;
        }
    }
}

/// The pickaxe that first takes something this hard, named ("a gold
/// pickaxe"), or none.
fn needs(items: &Items, hardness: u8) -> Option<String> {
    (0..items.len())
        .map(|i| items.def(crate::hands::items::ItemId(i as u16)))
        .filter_map(|d| match d.use_ {
            Use::Mine { back: false, tier, .. } if tier >= hardness => Some((tier, d.name.to_lowercase())),
            _ => None,
        })
        .min_by_key(|(t, _)| *t)
        .map(|(_, n)| format!("needs {} {n}", if n.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" }))
}

/// A material's name as shown ("iron_ore": "Iron ore").
fn shown_name(name: &str) -> String {
    let mut s = name.replace('_', " ");
    if let Some(f) = s.get_mut(0..1) {
        f.make_ascii_uppercase();
    }
    s
}

type Placed<'a> = (&'a mut Node, &'a mut ImageNode, &'a mut Visibility);
type Words<'a> = (&'a mut Node, &'a mut Text, &'a mut TextColor, &'a mut Visibility);

/// The cursor, where the mouse is, as what a click there does.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn point(
    (window, camera): (Query<&Window, With<PrimaryWindow>>, Query<(&Camera, &GlobalTransform), With<crate::camera::MainCamera>>),
    art: Res<Art>,
    cursor: Res<CursorWorld>,
    items: Option<Res<Items>>,
    hand: Res<Hand>,
    (dev, open): (Res<DevTools>, Res<InventoryOpen>),
    sim: Res<SimWorld>,
    exposure: Res<Exposure>,
    (wands, book, weapons): (Res<crate::magic::Wands>, Res<crate::magic::Spellbook>, Option<Res<crate::combat::Weapons>>),
    player: Query<(Entity, &Kinematics, &Inventory, Option<&crate::archery::Nocked>, Option<&crate::magic::Mana>), With<LocalPlayer>>,
    (found, folk, drops, filter): (
        Query<(&crate::hands::chests::Container, &Kinematics), Without<LocalPlayer>>,
        Query<(&crate::creatures::brain::villager::Villager, &Kinematics), Without<LocalPlayer>>,
        Query<(Entity, &crate::hands::Dropped, &Kinematics), Without<LocalPlayer>>,
        Query<&crate::hands::filter::PickupFilter, With<LocalPlayer>>,
    ),
    (mut pointer, mut ring, mut eye, mut label): (
        Query<Placed, (With<Pointer>, Without<Ring>, Without<Eye>, Without<Label>)>,
        Query<Placed, (With<Ring>, Without<Pointer>, Without<Eye>, Without<Label>)>,
        Query<Placed, (With<Eye>, Without<Pointer>, Without<Ring>, Without<Label>)>,
        Query<Words, (With<Label>, Without<Pointer>, Without<Ring>, Without<Eye>)>,
    ),
) {
    let (Ok(mut p), Ok(mut r), Ok(mut e), Ok(mut l)) = (pointer.single_mut(), ring.single_mut(), eye.single_mut(), label.single_mut()) else { return };
    // (No mouse over the window, as in a scripted run: where the cursor is
    // put in the world, shown.)
    let screen = window.single().ok().and_then(|w| w.cursor_position()).or_else(|| {
        let (cam, tf) = camera.single().ok()?;
        cam.world_to_viewport(tf, cursor.0?.extend(0.0)).ok()
    });
    let Some(screen) = screen else {
        for v in [&mut *p.2, &mut *r.2, &mut *e.2, &mut *l.3] {
            *v = Visibility::Hidden;
        }
        return;
    };
    let mut shape = Shape::Arrow;
    let mut fill: Option<(f32, Color)> = None;
    let mut words: Option<(String, Color)> = None;
    let menus = open.0 || dev.0;
    if let (false, Some(items), Some(at), Ok((me, k, inv, nocked, mana))) = (menus, items.as_ref(), cursor.0, player.single()) {
        let held = inv.slots.get(hand.active()).copied().flatten();
        let from = crate::hands::hand_at(k);
        match held.map(|s| (s, items.def(s.item).use_.clone())) {
            Some((_, Use::Melee(_) | Use::Throw(_))) => shape = Shape::Cross,
            Some((_, Use::Bow(_))) => {
                shape = Shape::Cross;
                if let (Some(n), Some(w)) = (nocked, weapons.as_ref()) {
                    fill = Some((n.drawn(w), Color::WHITE));
                }
            }
            Some((s, Use::Focus { tier, element, ref spells })) => {
                shape = Shape::Cross;
                let held = crate::magic::spells::focus_spells(&book.spells, tier, element, spells, &s.roll);
                if let Some(&spell) = held.first() {
                    let ready = wands.ready(me, spell);
                    let short = mana.is_some_and(|m| m.cur < book.describe(&book.spells[spell].runes).1);
                    if short {
                        fill = Some((1.0, Color::srgb(1.0, 0.35, 0.3)));
                    } else if ready < 1.0 {
                        fill = Some((ready, Color::WHITE));
                    }
                }
            }
            Some((_, Use::Mine { back, tier, reach, .. })) => {
                shape = Shape::Brackets;
                let block = platypus_sim::block_of(CellPos::from_world(at.x, at.y));
                let centre = Vec2::new((block.x as f32 + 0.5) * BLOCK as f32, (block.y as f32 + 0.5) * BLOCK as f32);
                if !back && centre.distance(from) <= reach * BLOCK as f32 {
                    let mats = sim.world.materials();
                    if let Some(m) = crate::hands::too_hard(&sim.world, block, tier) {
                        let hard = mats.phys(m).hardness;
                        let why = if hard == u8::MAX { "can't be mined".to_string() } else { needs(items, hard).unwrap_or_else(|| "too hard to mine".into()) };
                        words = Some((format!("{}: {why}", shown_name(&mats.def(m).name)), Color::srgb(1.0, 0.45, 0.4)));
                    } else if let Some(c) = sim.world.get(CellPos::from_world(at.x, at.y)).filter(|c| !c.is_air() && matches!(mats.phys(c.material).kind, platypus_sim::Kind::Static | platypus_sim::Kind::Powder)) {
                        words = Some((shown_name(&mats.def(c.material).name), Color::srgba(1.0, 1.0, 1.0, 0.8)));
                    }
                }
            }
            _ => {}
        }
        // What a right-click opens, near enough: a hand (over anything held).
        let over = |ck: &Kinematics| ((ck.body.pos - at).abs() - ck.body.half).max_element() <= 4.5;
        if let Some((c, _)) = found.iter().find(|(_, ck)| over(ck) && ck.body.pos.distance(k.body.pos) <= crate::hands::chests::REACH) {
            shape = Shape::Hand;
            fill = None;
            words = Some((c.name.clone(), Color::WHITE));
        } else if let Some((v, _)) = folk.iter().find(|(_, vk)| over(vk) && vk.body.pos.distance(k.body.pos) <= crate::creatures::brain::villager::TALK_NEAR) {
            shape = Shape::Hand;
            fill = None;
            words = Some((shown_name(&v.role), Color::WHITE));
        } else if let Some((_, s)) = filter.single().ok().and_then(|f| crate::hands::filter::left_under(at, k.body.pos, items, f, drops.iter())) {
            // Something its filter leaves: it can be taken anyway.
            shape = Shape::Hand;
            fill = None;
            words = Some((format!("{}: right-click to take", items.def(s.item).name), Color::srgba(1.0, 1.0, 1.0, 0.85)));
        }
        // The eye, out of the light (or crouched).
        if exposure.light < EYE_UNDER || exposure.crouched {
            let open = exposure.light > 0.5 || exposure.noise > 0.0;
            let half = exposure.light > 0.2 || !exposure.crouched;
            let (which, tint) = if open { (2, Color::srgb(1.0, 0.82, 0.35)) } else if half { (1, Color::srgb(0.85, 0.85, 0.8)) } else { (0, Color::srgba(0.7, 0.7, 0.75, 0.8)) };
            let (img, size) = &art.eyes[which];
            e.1.image = img.clone();
            e.1.color = tint;
            e.0.width = px(size.x * SCALE);
            e.0.height = px(size.y * SCALE);
            e.0.left = px(screen.x + EYE_AT.x);
            e.0.top = px(screen.y + EYE_AT.y);
            *e.2 = Visibility::Inherited;
        } else {
            *e.2 = Visibility::Hidden;
        }
    } else {
        *e.2 = Visibility::Hidden;
    }
    // The pointer.
    let (img, size) = &art.shapes[shape as usize];
    if p.1.image != *img {
        p.1.image = img.clone();
        p.0.width = px(size.x * SCALE);
        p.0.height = px(size.y * SCALE);
    }
    let top_left = screen - shape.hotspot() * SCALE;
    p.0.left = px(top_left.x);
    p.0.top = px(top_left.y);
    *p.2 = Visibility::Inherited;
    // The ring, round the point.
    match fill {
        Some((done, color)) => {
            let i = ((done.clamp(0.0, 1.0) * RING_STEPS as f32).round() as usize).min(RING_STEPS);
            r.1.image = art.rings[i].clone();
            r.1.color = color;
            let half = RING_SIZE as f32 * SCALE / 2.0;
            r.0.left = px(screen.x - half);
            r.0.top = px(screen.y - half);
            *r.2 = Visibility::Inherited;
        }
        None => *r.2 = Visibility::Hidden,
    }
    // The words, under and right of it.
    match words {
        Some((w, color)) => {
            if l.1.0 != w {
                l.1.0 = w;
            }
            l.2.0 = color;
            l.0.left = px(screen.x + LABEL_AT.x);
            l.0.top = px(screen.y + LABEL_AT.y);
            *l.3 = Visibility::Inherited;
        }
        None => *l.3 = Visibility::Hidden,
    }
}

/// Holding something to throw: where it would fly, a dot every few ticks of
/// its flight, to where it first strikes something (a bigger dot).
#[allow(clippy::too_many_arguments)]
fn throw_arc(
    time: Res<Time>,
    cursor: Res<CursorWorld>,
    items: Option<Res<Items>>,
    hand: Res<Hand>,
    dev: Res<DevTools>,
    open: Res<InventoryOpen>,
    tools: Res<crate::tools::ToolsConfig>,
    sim: Res<SimWorld>,
    player: Query<(&Kinematics, &Inventory), With<LocalPlayer>>,
    mut dots: Query<(&ArcDot, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let path = (|| {
        let (items, at, (k, inv)) = (items.as_ref()?, cursor.0?, player.single().ok()?);
        if open.0 || dev.0 {
            return None;
        }
        let held = inv.slots.get(hand.active()).copied().flatten()?;
        let Use::Throw(_) = items.def(held.item).use_ else { return None };
        // (As the hands throw it: `use_hands`.)
        let from = crate::hands::hand_at(k);
        let dir = (at - from).normalize_or(Vec2::X);
        let speed = tools.bomb.throw_speed * ((at - from).length() / 180.0).clamp(0.35, 1.0);
        let mut body = Body::new(from, Vec2::splat(4.5));
        body.vel = dir * speed + k.body.vel * 0.5;
        let grid = crate::creatures::WorldGrid(&sim.world);
        let mut out = Vec::new();
        for t in 0..(ARC_SECS / DT) as usize {
            body.vel.y -= crate::props::GRAVITY * DT;
            let c = platypus_physics::move_and_collide(&grid, &mut body, DT);
            if c.ground || c.ceiling || c.wall_left || c.wall_right {
                out.push((body.pos, true));
                break;
            }
            if t % DOT_EVERY == DOT_EVERY - 1 {
                out.push((body.pos, false));
            }
            if out.len() >= DOTS {
                break;
            }
        }
        Some(out)
    })()
    .unwrap_or_default();
    // (The dots drift along it, gently, so it reads as a path.)
    let pulse = time.elapsed_secs() * 2.0;
    let n = path.len();
    for (d, mut tf, mut sprite, mut vis) in &mut dots {
        let Some(&(at, end)) = path.get(d.0) else {
            *vis = Visibility::Hidden;
            continue;
        };
        tf.translation.x = at.x;
        tf.translation.y = at.y;
        let fade = 1.0 - d.0 as f32 / n.max(1) as f32 * 0.5;
        let wave = 0.6 + 0.4 * ((d.0 as f32 * 0.6 - pulse * 3.0).sin() * 0.5 + 0.5);
        sprite.custom_size = Some(Vec2::splat(if end { 3.5 } else { 2.0 }));
        sprite.color = if end { Color::srgba(1.0, 0.85, 0.4, 0.95) } else { Color::srgba(1.0, 1.0, 1.0, 0.85 * fade * wave) };
        *vis = Visibility::Visible;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_are_rectangles() {
        for rows in [ARROW, CROSS, BRACKETS, HAND, EYE_SHUT, EYE_HALF, EYE_OPEN] {
            assert!(rows.iter().all(|r| r.len() == rows[0].len()), "{rows:?}");
        }
    }

    #[test]
    fn the_point_is_inside_each_picture() {
        for (s, rows) in [(Shape::Arrow, ARROW), (Shape::Cross, CROSS), (Shape::Brackets, BRACKETS), (Shape::Hand, HAND)] {
            let h = s.hotspot();
            assert!(h.x < rows[0].len() as f32 && h.y < rows.len() as f32);
        }
    }
}
