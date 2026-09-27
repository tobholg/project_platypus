//! The player's HUD, top right (as Terraria's): life as hearts (20 a heart,
//! ten to a row) with the numbers over them, mana as blue stars and
//! stamina as green bolts (20 each), each icon filling from the left as
//! it comes back; and under them a round timer for each status (burning,
//! chilled, the current coating), filled by how much of it is left, and
//! while rocket boots are worn, always, their charge (the rightmost).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::actors::elements::{BURN_SECS, Burning, CHILL_SECS, Chilled, Coated, Coatings};
use crate::actors::player::LocalPlayer;
use crate::actors::{Health, PlayerDeaths};

pub struct HudPlugin;

const SLOTS: usize = 5;
/// Status icon size, and a status's width with its label under it
/// (pixels; even, so the icon sits on whole pixels).
const ICON: u32 = 34;
const SLOT_WIDTH: f32 = 76.0;
/// Life, mana or stamina an icon stands for.
const PER: f32 = 20.0;
/// Most icons of a kind shown (and hearts to a row).
const MOST: usize = 20;
const ROW: usize = 10;
/// Steps an icon fills in, and the pixels a cell of its art is drawn.
const LEVELS: usize = 8;
const SCALE: u32 = 2;

/// The icons, as text art: a heart, a star, a bolt ('o' the outline, 'W' a
/// highlight, anything else the fill).
const HEART: [&str; 10] = [
    "..ooo.ooo..",
    ".orrrorrro.",
    "orrWrrrrrro",
    "orWrrrrrrro",
    "orrrrrrrrro",
    ".orrrrrrro.",
    "..orrrrro..",
    "...orrro...",
    "....oro....",
    ".....o.....",
];
const STAR: [&str; 11] = [
    ".....o.....",
    "....obo....",
    "....obo....",
    "oooobbboooo",
    "obbbbWbbbbo",
    ".obbbbbbbo.",
    "..obbbbbo..",
    "..obbobbo..",
    ".obbo.obbo.",
    ".obo...obo.",
    ".oo.....oo.",
];
const BOLT: [&str; 11] = [
    "......ooo..",
    ".....oWgo..",
    "....oWgo...",
    "...oggo....",
    "..oggoooo..",
    ".oggggggo..",
    "..oooogo...",
    "....oggo...",
    "...oggo....",
    "..ogo......",
    "..oo.......",
];

/// A kind of vital: its art, its colours full (fill, highlight) and spent.
struct Vital {
    art: &'static [&'static str],
    fill: [u8; 3],
    shine: [u8; 3],
    spent: [u8; 3],
}

const VITALS: [Vital; 3] = [
    Vital { art: &HEART, fill: [228, 44, 58], shine: [255, 196, 206], spent: [64, 28, 34] },
    Vital { art: &STAR, fill: [70, 140, 255], shine: [220, 238, 255], spent: [30, 38, 70] },
    Vital { art: &BOLT, fill: [130, 222, 70], shine: [220, 255, 170], spent: [36, 58, 30] },
];

#[derive(Component)]
struct LifeText;
/// One icon: which vital, and which of them.
#[derive(Component)]
struct Pip(usize, usize);

#[derive(Component)]
struct StatusSlot(usize);
#[derive(Component)]
struct StatusLabel(usize);

#[derive(Resource)]
struct Icons([Handle<Image>; SLOTS]);

/// Each vital's icon at each fill (0..=LEVELS).
#[derive(Resource)]
struct PipImages(Vec<Vec<Handle<Image>>>);

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hud).add_systems(Update, (update_life, update_pips, update_statuses));
    }
}

/// A vital's icon filled `share` from the left (the rest spent), drawn
/// `SCALE` pixels a cell.
fn pip_image(v: &Vital, share: f32) -> Image {
    let (w, h) = (v.art[0].len() as u32, v.art.len() as u32);
    let mut data = vec![0u8; (w * h * SCALE * SCALE * 4) as usize];
    let lit = (share * w as f32).round() as u32;
    for (y, row) in v.art.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            let full = (x as u32) < lit;
            let rgba = match ch {
                '.' => continue,
                'o' => [20, 10, 14, 255],
                'W' if full => [v.shine[0], v.shine[1], v.shine[2], 255],
                _ if full => {
                    // (Darker toward the bottom.)
                    let k = 1.0 - 0.3 * y as f32 / h as f32;
                    [(v.fill[0] as f32 * k) as u8, (v.fill[1] as f32 * k) as u8, (v.fill[2] as f32 * k) as u8, 255]
                }
                _ => [v.spent[0], v.spent[1], v.spent[2], 230],
            };
            for sy in 0..SCALE {
                for sx in 0..SCALE {
                    let (px, py) = (x as u32 * SCALE + sx, y as u32 * SCALE + sy);
                    let i = ((py * w * SCALE + px) * 4) as usize;
                    data[i..i + 4].copy_from_slice(&rgba);
                }
            }
        }
    }
    Image::new(Extent3d { width: w * SCALE, height: h * SCALE, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD)
}

fn spawn_hud(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let icons: [Handle<Image>; SLOTS] = std::array::from_fn(|_| images.add(blank()));
    let pips: Vec<Vec<Handle<Image>>> = VITALS.iter().map(|v| (0..=LEVELS).map(|l| images.add(pip_image(v, l as f32 / LEVELS as f32))).collect()).collect();
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(12.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
            row_gap: Val::Px(3.0),
            ..default()
        })
        .with_children(|root| {
            root.spawn((
                LifeText,
                Text::new(""),
                TextFont { font_size: FontSize::Px(13.0), ..default() },
                TextColor(Color::WHITE),
            ));
            for (k, v) in VITALS.iter().enumerate() {
                let (w, h) = (v.art[0].len() as f32 * SCALE as f32, v.art.len() as f32 * SCALE as f32);
                // (Hearts wrap ten to a row; the others stay one row.)
                let most = if k == 0 { MOST } else { ROW };
                root.spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::FlexEnd,
                    max_width: Val::Px((w + 2.0) * ROW as f32),
                    column_gap: Val::Px(2.0),
                    row_gap: Val::Px(2.0),
                    ..default()
                })
                .with_children(|row| {
                    for i in 0..most {
                        row.spawn((Pip(k, i), ImageNode::new(pips[k][LEVELS].clone()), Node { width: Val::Px(w), height: Val::Px(h), display: Display::None, ..default() }));
                    }
                });
            }
            // Statuses: round timers with their name under them.
            root.spawn(Node { column_gap: Val::Px(10.0), margin: UiRect::top(Val::Px(4.0)), ..default() }).with_children(|row| {
                for (i, icon) in icons.iter().enumerate() {
                    // (A fixed width: a label changing never moves the row.)
                    row.spawn((StatusSlot(i), Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, width: Val::Px(SLOT_WIDTH), display: Display::None, ..default() }))
                        .with_children(|slot| {
                            slot.spawn((ImageNode::new(icon.clone()), Node { width: Val::Px(ICON as f32), height: Val::Px(ICON as f32), ..default() }));
                            slot.spawn((StatusLabel(i), Text::new(""), TextFont { font_size: FontSize::Px(11.0), ..default() }, TextColor(Color::WHITE)));
                        });
                }
            });
        });
    commands.insert_resource(Icons(icons));
    commands.insert_resource(PipImages(pips));
}

fn blank() -> Image {
    Image::new_fill(
        Extent3d { width: ICON, height: ICON, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

fn update_life(deaths: Res<PlayerDeaths>, player: Query<&Health, With<LocalPlayer>>, mut text: Query<&mut Text, With<LifeText>>) {
    let Ok(h) = player.single() else { return };
    let died = if deaths.0 > 0 { format!("    died {}x", deaths.0) } else { String::new() };
    let want = format!("Life {:.0}/{:.0}{died}", h.hp.max(0.0).ceil(), h.max);
    for mut t in &mut text {
        if t.0 != want {
            t.0.clone_from(&want);
        }
    }
}

type Vitals<'a> = (&'a Health, Option<&'a crate::magic::Mana>, Option<&'a crate::combat::Stamina>);

/// Each icon shows its share of what it stands for: full, spent, or part
/// way; as many as the most there is (a heart for every 20).
fn update_pips(
    pips: Res<PipImages>,
    player: Query<Vitals, With<LocalPlayer>>,
    mut icons: Query<(&Pip, &mut ImageNode, &mut Node)>,
) {
    let Ok((h, mana, stamina)) = player.single() else { return };
    let vitals = [Some((h.hp.max(0.0), h.max)), mana.map(|m| (m.cur, m.max)), stamina.map(|s| (s.cur, s.max))];
    for (&Pip(k, i), mut image, mut node) in &mut icons {
        // (Out of the layout, not just hidden: the rows stay against the edge.)
        let count = vitals[k].map_or(0, |(_, max)| (max / PER).ceil() as usize);
        let display = if i < count { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
        let Some((cur, max)) = vitals[k] else { continue };
        // (The last icon of an odd maximum stands for what's left of it.)
        let worth = (max - i as f32 * PER).min(PER);
        let share = ((cur - i as f32 * PER) / worth.max(1.0)).clamp(0.0, 1.0);
        let level = (share * LEVELS as f32).round() as usize;
        if image.image != pips.0[k][level] {
            image.image = pips.0[k][level].clone();
        }
    }
}

type PlayerStatuses<'a> = (Option<&'a Burning>, Option<&'a Chilled>, Option<&'a Coated>, &'a crate::actors::Kinematics, Option<&'a crate::actors::MoveStats>);

fn update_statuses(
    coatings: Res<Coatings>,
    icons: Res<Icons>,
    mut images: ResMut<Assets<Image>>,
    player: Query<PlayerStatuses, With<LocalPlayer>>,
    mut slots: Query<(&StatusSlot, &mut Node)>,
    mut labels: Query<(&StatusLabel, &mut Text)>,
    mut fuel: Local<(f32, bool)>,
) {
    let Ok((burning, chilled, coated, k, stats)) = player.single() else { return };
    // (label, colour, share left)
    let mut shown: Vec<(String, [u8; 3], f32)> = Vec::new();
    if let Some(b) = burning {
        shown.push(("Burning".into(), [255, 130, 30], b.left / b.total.max(BURN_SECS * 0.1)));
    }
    if let Some(c) = chilled {
        shown.push(("Chilled".into(), [150, 215, 255], c.left / CHILL_SECS));
    }
    if let Some(c) = coated
        && let Some(def) = coatings.by_name.get(&c.name)
    {
        let (r, g, b) = def.color;
        shown.push((def.label.clone(), [r, g, b], c.left / c.total.max(0.01)));
    }
    // Rocket boots worn: their charge, always, rightmost.
    if let Some(time) = stats.map(|s| s.0.rocket_time).filter(|&t| t > 0.0) {
        let left = k.loco.rocket_left;
        // (Filling or not from how it last moved: the sim ticks slower
        // than frames, so it's unchanged every other frame; judged frame by
        // frame the label flickered, and the row with it.)
        if left > fuel.0 + 1e-5 {
            fuel.1 = true;
        } else if left < fuel.0 - 1e-5 || left >= time {
            fuel.1 = false;
        }
        fuel.0 = left;
        let label = if left <= 0.0 && !fuel.1 {
            "Empty"
        } else if fuel.1 {
            "Recharging"
        } else {
            "Rockets"
        };
        shown.push((label.into(), [255, 176, 70], left / time));
    }
    // (Out of the layout when unused, not just hidden: the row stays
    // against the right edge, as the hearts do.)
    for (slot, mut node) in &mut slots {
        let display = if slot.0 < shown.len() { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
    for (label, mut text) in &mut labels {
        if let Some((name, ..)) = shown.get(label.0) {
            text.0.clone_from(name);
        }
    }
    for (i, (_, color, left)) in shown.iter().enumerate() {
        if let Some(mut image) = images.get_mut(&icons.0[i])
            && let Some(data) = image.data.as_mut()
        {
            paint_timer(data, *color, left.clamp(0.0, 1.0));
        }
    }
}

/// A round timer: a dark disc, the share `left` filled clockwise from the
/// top in `color`, and a ring.
fn paint_timer(data: &mut [u8], color: [u8; 3], left: f32) {
    let c = (ICON as f32 - 1.0) / 2.0;
    let r = c - 1.0;
    for y in 0..ICON {
        for x in 0..ICON {
            let (dx, dy) = (x as f32 - c, y as f32 - c);
            let d = (dx * dx + dy * dy).sqrt();
            let i = ((y * ICON + x) * 4) as usize;
            if d > r + 0.5 {
                data[i..i + 4].copy_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            // Angle from 12 o'clock, clockwise, 0..1 (image rows go down).
            let a = (dx.atan2(-dy) / std::f32::consts::TAU).rem_euclid(1.0);
            let px = if d > r - 2.0 {
                [color[0], color[1], color[2], 255]
            } else if a <= left {
                [color[0], color[1], color[2], 230]
            } else {
                [20, 20, 24, 170]
            };
            data[i..i + 4].copy_from_slice(&px);
        }
    }
}
