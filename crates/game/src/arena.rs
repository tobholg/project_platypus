//! The arena's tools (`PLATYPUS_WORLD=arena`, or the dev panel anywhere):
//! time (pause, a tick at a time, slow motion), overlays (every body's box,
//! its facing and hand), what `O` spawns at the cursor, and clearing the
//! floor. A panel on the left, and keys: P pause · . one tick (paused) ·
//! , slower (1, 1/2, 1/4, 1/10) · Y overlays · E the art editor (`editor.rs`).
//!
//! Pausing pauses virtual time, so the sim, bodies, particles and
//! animations all stop; a step hands the fixed clock exactly one tick.

use bevy::prelude::*;

use crate::actors::dummy::Dummy;
use crate::actors::player::LocalPlayer;
use crate::actors::spawn::SpawnKind;
use crate::actors::{Creature, Kinematics, Team};
use crate::actors::animation::HandPos;
use crate::data::data_path;
use crate::world::SimWorld;

pub struct ArenaPlugin;

/// The speeds slow motion goes through.
const SPEEDS: [f32; 4] = [1.0, 0.5, 0.25, 0.1];

/// Something the arena panel or keys ask for.
#[derive(Message, Clone, Debug, PartialEq)]
pub enum ArenaAction {
    Pause,
    Step,
    Speed(f32),
    Slower,
    Overlays,
    Pick(String),
    Clear,
    /// The art editor, open or shut.
    Editor,
}

#[derive(Resource, Default)]
pub struct ArenaView {
    /// The panel is showing.
    pub open: bool,
    /// Bodies' boxes, facing and hands drawn over the world.
    pub overlays: bool,
}

#[derive(Component)]
struct Panel;

#[derive(Component)]
struct PanelTitle;

#[derive(Component)]
struct PanelButton(ArenaAction);

impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ArenaAction>()
            .init_resource::<ArenaView>()
            .init_resource::<StepOwed>()
            .add_systems(Startup, spawn_panel)
            .add_systems(PreUpdate, (keys, buttons, act).chain().after(bevy::ui::UiSystems::Focus))
            .add_systems(PreUpdate, step.after(act))
            .add_systems(Update, (show_panel, overlays));
    }
}

/// Every pack (`packs.ron`), then every creature kind there's a file for
/// (but the player).
fn kinds() -> Vec<String> {
    let mut v: Vec<String> = crate::actors::spawn::packs().into_keys().collect();
    v.extend(creature_kinds());
    v
}

fn creature_kinds() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(data_path("creatures"))
        .map(|d| {
            d.filter_map(|e| e.ok())
                .filter_map(|e| e.path().file_stem().map(|s| s.to_string_lossy().into_owned()))
                .filter(|k| k != "player")
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn keys(keys: Res<ButtonInput<KeyCode>>, view: Res<ArenaView>, taken: Res<crate::dev::KeyboardTaken>, mut out: MessageWriter<ArenaAction>) {
    if !view.open || taken.0 {
        return;
    }
    for (k, a) in [
        (KeyCode::KeyP, ArenaAction::Pause),
        (KeyCode::Period, ArenaAction::Step),
        (KeyCode::Comma, ArenaAction::Slower),
        (KeyCode::KeyY, ArenaAction::Overlays),
        (KeyCode::KeyE, ArenaAction::Editor),
    ] {
        if keys.just_pressed(k) {
            out.write(a);
        }
    }
}

fn buttons(clicks: Query<(&Interaction, &PanelButton), Changed<Interaction>>, mut out: MessageWriter<ArenaAction>) {
    for (i, b) in &clicks {
        if *i == Interaction::Pressed {
            out.write(b.0.clone());
        }
    }
}

/// Every creature but the player's.
type Others = (With<Creature>, Without<LocalPlayer>);

/// A tick owed to a paused clock.
#[derive(Resource, Default)]
struct StepOwed(bool);

#[allow(clippy::too_many_arguments)]
fn act(
    mut commands: Commands,
    mut actions: MessageReader<ArenaAction>,
    mut dev: MessageReader<crate::dev::DevAction>,
    mut view: ResMut<ArenaView>,
    mut virt: ResMut<Time<Virtual>>,
    mut kind: ResMut<SpawnKind>,
    mut editor: ResMut<crate::editor::Editor>,
    creatures: Query<(Entity, Option<&Dummy>), Others>,
    mut owed: ResMut<StepOwed>,
) {
    for a in dev.read() {
        if *a == crate::dev::DevAction::Arena {
            view.open = !view.open;
        }
    }
    for a in actions.read() {
        match a {
            ArenaAction::Pause => {
                if virt.is_paused() {
                    virt.unpause();
                } else {
                    virt.pause();
                }
            }
            ArenaAction::Step => {
                virt.pause();
                owed.0 = true;
            }
            ArenaAction::Speed(s) => virt.set_relative_speed(*s),
            ArenaAction::Slower => {
                let now = virt.relative_speed();
                let i = SPEEDS.iter().position(|&s| (s - now).abs() < 1e-3).unwrap_or(0);
                virt.set_relative_speed(SPEEDS[(i + 1) % SPEEDS.len()]);
            }
            ArenaAction::Overlays => view.overlays = !view.overlays,
            ArenaAction::Pick(k) => kind.0 = k.clone(),
            ArenaAction::Editor => editor.open = !editor.open,
            // Everything but the player and the planted dummies.
            ArenaAction::Clear => {
                for (e, d) in &creatures {
                    if !d.is_some_and(|d| d.anchored) {
                        commands.entity(e).despawn();
                    }
                }
            }
        }
    }
}

/// A step: one tick's worth on the fixed clock, which runs it though
/// virtual time stands still.
fn step(mut owed: ResMut<StepOwed>, mut fixed: ResMut<Time<Fixed>>) {
    if owed.0 {
        owed.0 = false;
        let dt = fixed.timestep();
        fixed.accumulate_overstep(dt);
    }
}

fn spawn_panel(mut commands: Commands, sim: Res<SimWorld>, mut view: ResMut<ArenaView>) {
    // Open from the start in the arena itself.
    view.open = !sim.generator.wild();
    let label = |p: &mut ChildSpawnerCommands, text: &str, action: ArenaAction| {
        p.spawn((Button, PanelButton(action), Node { padding: UiRect::axes(px(7), px(2)), ..default() }, BackgroundColor(IDLE)))
            .with_children(|b| {
                b.spawn((Text::new(text), TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(Color::WHITE)));
            });
    };
    let row = |p: &mut ChildSpawnerCommands, f: &dyn Fn(&mut ChildSpawnerCommands)| {
        p.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(3), row_gap: px(3), max_width: px(230), ..default() })
            .with_children(|r| f(r));
    };
    let heading = |p: &mut ChildSpawnerCommands, text: &str| {
        p.spawn((Text::new(text), TextFont { font_size: FontSize::Px(11.0), ..default() }, TextColor(Color::srgb(0.75, 0.75, 0.8))));
    };
    commands
        .spawn((
            Panel,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                top: px(150),
                left: px(6),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                padding: UiRect::all(px(6)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        ))
        .with_children(|p| {
            p.spawn((PanelTitle, Text::new("ARENA"), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(Color::srgb(1.0, 0.85, 0.3))));
            heading(p, "Time");
            row(p, &|r| {
                label(r, "Pause  P", ArenaAction::Pause);
                label(r, "Step  .", ArenaAction::Step);
            });
            row(p, &|r| {
                for (s, t) in SPEEDS.iter().zip(["1x", "1/2", "1/4", "1/10"]) {
                    label(r, t, ArenaAction::Speed(*s));
                }
            });
            heading(p, "Look");
            row(p, &|r| label(r, "Boxes and hands  Y", ArenaAction::Overlays));
            heading(p, "Spawn at the cursor: O (packs first)");
            row(p, &|r| {
                for k in kinds() {
                    label(r, &k, ArenaAction::Pick(k.clone()));
                }
            });
            row(p, &|r| label(r, "Clear the floor", ArenaAction::Clear));
            heading(p, "Make");
            row(p, &|r| label(r, "Art editor  E", ArenaAction::Editor));
        });
}

const IDLE: Color = Color::srgba(0.25, 0.25, 0.3, 0.8);
const PICKED: Color = Color::srgba(0.55, 0.45, 0.15, 0.9);

fn show_panel(
    view: Res<ArenaView>,
    virt: Res<Time<Virtual>>,
    kind: Res<SpawnKind>,
    mut panel: Query<&mut Visibility, With<Panel>>,
    mut title: Query<&mut Text, With<PanelTitle>>,
    mut buttons: Query<(&Interaction, &PanelButton, &mut BackgroundColor)>,
) {
    for mut v in &mut panel {
        *v = if view.open { Visibility::Visible } else { Visibility::Hidden };
    }
    let speed = virt.relative_speed();
    for mut t in &mut title {
        t.0 = if virt.is_paused() { "ARENA  paused".into() } else if speed < 1.0 { format!("ARENA  x{speed}") } else { "ARENA".into() };
    }
    for (i, b, mut bg) in &mut buttons {
        let on = match &b.0 {
            ArenaAction::Speed(s) => (s - speed).abs() < 1e-3,
            ArenaAction::Pause => virt.is_paused(),
            ArenaAction::Overlays => view.overlays,
            ArenaAction::Pick(k) => *k == kind.0,
            _ => false,
        };
        bg.0 = match i {
            Interaction::Pressed => Color::srgba(0.6, 0.5, 0.2, 0.9),
            Interaction::Hovered => Color::srgba(0.35, 0.35, 0.42, 0.9),
            Interaction::None if on => PICKED,
            Interaction::None => IDLE,
        };
    }
}

/// Each body's box (the player's blue, enemies' red, the rest green), a
/// tick at its front for facing, its feet, and its hand while it aims.
fn overlays(view: Res<ArenaView>, mut gizmos: Gizmos, q: Query<(&Kinematics, &Transform, Option<&Team>, Option<&HandPos>)>) {
    if !view.overlays {
        return;
    }
    for (k, tf, team, hand) in &q {
        let color = match team {
            Some(Team::Player) => Color::srgb(0.3, 0.6, 1.0),
            Some(Team::Enemy) => Color::srgb(1.0, 0.3, 0.25),
            _ => Color::srgb(0.4, 1.0, 0.4),
        };
        let (p, h) = (tf.translation.truncate(), k.body.half);
        gizmos.rect_2d(Isometry2d::from_translation(p), h * 2.0, color);
        let f = k.loco.facing;
        gizmos.line_2d(p + Vec2::new(h.x * f, 0.0), p + Vec2::new((h.x + 3.0) * f, 0.0), color);
        gizmos.line_2d(p + Vec2::new(-h.x - 1.0, -h.y), p + Vec2::new(h.x + 1.0, -h.y), Color::WHITE);
        if let Some(HandPos { at: Some(at), .. }) = hand {
            gizmos.circle_2d(Isometry2d::from_translation(*at), 1.0, Color::srgb(1.0, 0.9, 0.2));
        }
    }
}
