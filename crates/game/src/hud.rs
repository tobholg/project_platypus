//! The player's HUD: a health bar (with a death count while developing), a
//! mana bar, and
//! a round timer for each status (burning, chilled, the current coating),
//! filled by how much of it is left.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::actors::elements::{BURN_SECS, Burning, CHILL_SECS, Chilled, Coated, Coatings};
use crate::actors::player::LocalPlayer;
use crate::actors::{Health, PlayerDeaths};

pub struct HudPlugin;

const SLOTS: usize = 4;
/// Status icon size (pixels).
const ICON: u32 = 34;

#[derive(Component)]
struct HealthFill;
#[derive(Component)]
struct HealthText;
#[derive(Component)]
struct ManaFill;
#[derive(Component)]
struct StatusSlot(usize);
#[derive(Component)]
struct StatusLabel(usize);

#[derive(Resource)]
struct Icons([Handle<Image>; SLOTS]);

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hud).add_systems(Update, (update_health, update_mana, update_statuses));
    }
}

fn spawn_hud(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let icons: [Handle<Image>; SLOTS] = std::array::from_fn(|_| images.add(blank()));
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-160.0)),
            width: Val::Px(320.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|root| {
            // Health: a bar with the numbers on it.
            root.spawn((
                Node { width: Val::Percent(100.0), height: Val::Px(16.0), ..default() },
                BackgroundColor(Color::srgba(0.08, 0.05, 0.05, 0.75)),
                BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    HealthFill,
                    Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                    BackgroundColor(Color::srgb(0.78, 0.12, 0.14)),
                ));
                bar.spawn((
                    HealthText,
                    Text::new(""),
                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                    TextColor(Color::WHITE),
                    Node { position_type: PositionType::Absolute, left: Val::Px(6.0), top: Val::Px(0.0), ..default() },
                ));
            });
            // Mana: a thinner bar under it.
            root.spawn((
                Node { width: Val::Percent(100.0), height: Val::Px(7.0), margin: UiRect::top(Val::Px(-3.0)), ..default() },
                BackgroundColor(Color::srgba(0.04, 0.05, 0.1, 0.75)),
            ))
            .with_child((ManaFill, Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(Color::srgb(0.25, 0.45, 1.0))));
            // Statuses: round timers with their name under them.
            root.spawn(Node { column_gap: Val::Px(10.0), justify_content: JustifyContent::Center, ..default() }).with_children(|row| {
                for (i, icon) in icons.iter().enumerate() {
                    row.spawn((StatusSlot(i), Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() }, Visibility::Hidden))
                        .with_children(|slot| {
                            slot.spawn((ImageNode::new(icon.clone()), Node { width: Val::Px(ICON as f32), height: Val::Px(ICON as f32), ..default() }));
                            slot.spawn((StatusLabel(i), Text::new(""), TextFont { font_size: FontSize::Px(11.0), ..default() }, TextColor(Color::WHITE)));
                        });
                }
            });
        });
    commands.insert_resource(Icons(icons));
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

fn update_health(
    deaths: Res<PlayerDeaths>,
    player: Query<&Health, With<LocalPlayer>>,
    mut fill: Query<&mut Node, With<HealthFill>>,
    mut text: Query<&mut Text, With<HealthText>>,
) {
    let Ok(h) = player.single() else { return };
    let f = (h.hp / h.max).clamp(0.0, 1.0);
    for mut n in &mut fill {
        n.width = Val::Percent(f * 100.0);
    }
    for mut t in &mut text {
        let died = if deaths.0 > 0 { format!("    died {}×", deaths.0) } else { String::new() };
        t.0 = format!("{:.0} / {:.0}{died}", h.hp.max(0.0), h.max);
    }
}

fn update_mana(player: Query<&crate::magic::Mana, With<LocalPlayer>>, mut fill: Query<&mut Node, With<ManaFill>>) {
    let Ok(m) = player.single() else { return };
    for mut n in &mut fill {
        n.width = Val::Percent((m.cur / m.max).clamp(0.0, 1.0) * 100.0);
    }
}

type PlayerStatuses<'a> = (Option<&'a Burning>, Option<&'a Chilled>, Option<&'a Coated>);

fn update_statuses(
    coatings: Res<Coatings>,
    icons: Res<Icons>,
    mut images: ResMut<Assets<Image>>,
    player: Query<PlayerStatuses, With<LocalPlayer>>,
    mut slots: Query<(&StatusSlot, &mut Visibility)>,
    mut labels: Query<(&StatusLabel, &mut Text)>,
) {
    let Ok((burning, chilled, coated)) = player.single() else { return };
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
    for (slot, mut vis) in &mut slots {
        *vis = if slot.0 < shown.len() { Visibility::Inherited } else { Visibility::Hidden };
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
