//! The arena's tools (`PLATYPUS_WORLD=arena`, or the dev panel anywhere):
//! time (pause, a tick at a time, slow motion), overlays (every body's box,
//! its facing and hand), what `O` spawns at the cursor, and clearing the
//! floor, and the game's tempo (`tempo.rs`). A panel on the left, and keys:
//! P pause · . one tick (paused) · , slower (1, 1/2, 1/4, 1/10) · Y
//! overlays · T the next tempo · E the art editor (`editor.rs`).
//!
//! The panel's sections fold (click a heading; which are open is
//! remembered, `saves/arena_panel.txt`), and it's no taller than the
//! window allows: the wheel scrolls it. It sits under the hotbar, and
//! steps away while the inventory's open.
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

/// Where the panel starts: under the hotbar.
const ARENA_TOP: f32 = 96.0;

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
    /// A tempo preset (`tempo.ron`), or the next.
    Tempo(usize),
    NextTempo,
    /// Play a sound (the sound board).
    Sound(String),
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

/// The panel's scrolling body.
#[derive(Component)]
struct PanelBody;

/// A section's heading (click: fold or unfold) and its contents.
#[derive(Component)]
struct SectionHead(usize);
#[derive(Component)]
struct SectionBody(usize);

/// The sections, in order, and which are open at first.
const SECTIONS: [(&str, bool); 6] = [("Time", true), ("Tempo: T the next (tempo.ron)", true), ("Look", true), ("Spawn at the cursor: O (packs first)", false), ("Make", true), ("Sounds: click to hear (sounds.ron), F11 mute", false)];

/// A group of the sound board: its name, and which sounds are in it.
type SoundGroup<'a> = (&'a str, &'a dyn Fn(&str) -> bool);

/// Which sections are open.
#[derive(Resource)]
struct Folds(Vec<bool>);

fn folds_path() -> std::path::PathBuf {
    crate::save::saves_dir().join("arena_panel.txt")
}

impl Default for Folds {
    fn default() -> Self {
        // (Remembered: a line per open section's first word.)
        let saved = std::fs::read_to_string(folds_path()).ok();
        let open = |i: usize| {
            let (name, open) = SECTIONS[i];
            saved.as_ref().map_or(open, |s| s.lines().any(|l| l == first_word(name)))
        };
        Folds((0..SECTIONS.len()).map(open).collect())
    }
}

fn first_word(s: &str) -> &str {
    s.split([' ', ':']).next().unwrap_or(s)
}

impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ArenaAction>()
            .init_resource::<ArenaView>()
            .init_resource::<StepOwed>()
            .init_resource::<Folds>()
            .add_systems(Startup, spawn_panel)
            .add_systems(PreUpdate, (keys, buttons, fold, scroll, act).chain().after(bevy::ui::UiSystems::Focus))
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
        (KeyCode::KeyT, ArenaAction::NextTempo),
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

/// A heading clicked: its section folds or unfolds (and it's remembered).
fn fold(heads: Query<(&Interaction, &SectionHead), Changed<Interaction>>, mut folds: ResMut<Folds>, mut bodies: Query<(&SectionBody, &mut Node)>, mut labels: Query<(&SectionHead, &mut Text)>) {
    let mut changed = false;
    for (i, h) in &heads {
        if *i == Interaction::Pressed {
            folds.0[h.0] = !folds.0[h.0];
            changed = true;
        }
    }
    if !changed && !folds.is_added() {
        return;
    }
    for (b, mut node) in &mut bodies {
        node.display = if folds.0[b.0] { Display::Flex } else { Display::None };
    }
    for (h, mut text) in &mut labels {
        text.0 = format!("{} {}", if folds.0[h.0] { "-" } else { "+" }, SECTIONS[h.0].0);
    }
    if changed {
        let open: Vec<&str> = SECTIONS.iter().zip(&folds.0).filter(|(_, o)| **o).map(|((n, _), _)| first_word(n)).collect();
        let _ = std::fs::create_dir_all(crate::save::saves_dir());
        let _ = std::fs::write(folds_path(), open.join("\n"));
    }
}

/// The wheel over the panel scrolls it.
fn scroll(wheel: Res<bevy::input::mouse::AccumulatedMouseScroll>, view: Res<ArenaView>, panel: Query<&bevy::ui::RelativeCursorPosition, With<Panel>>, mut body: Query<&mut ScrollPosition, With<PanelBody>>) {
    if !view.open || wheel.delta.y == 0.0 || !panel.iter().any(|p| p.cursor_over()) {
        return;
    }
    let dy = match wheel.unit {
        bevy::input::mouse::MouseScrollUnit::Line => wheel.delta.y * 24.0,
        bevy::input::mouse::MouseScrollUnit::Pixel => wheel.delta.y,
    };
    for mut s in &mut body {
        s.y = (s.y - dy).max(0.0);
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
    mut tempo: ResMut<crate::tempo::Tempo>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
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
            ArenaAction::Tempo(i) => tempo.active = (*i).min(tempo.presets.len().saturating_sub(1)),
            ArenaAction::NextTempo => tempo.active = (tempo.active + 1) % tempo.presets.len().max(1),
            ArenaAction::Sound(name) => {
                sounds.write(crate::sound::PlaySound::here(name.clone()));
            }
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

fn spawn_panel(mut commands: Commands, sim: Res<SimWorld>, mut view: ResMut<ArenaView>, tempo: Res<crate::tempo::Tempo>, bank: Res<crate::sound::SoundBank>) {
    let tempos: Vec<String> = tempo.presets.iter().map(|p| p.name.clone()).collect();
    // (The one-shots: the beds and music play themselves.)
    let sounds: Vec<String> = bank.defs.iter().filter(|(_, d)| d.loops <= 0.0).map(|(n, _)| n.clone()).collect();
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
    commands
        .spawn((
            Panel,
            Visibility::Hidden,
            // (Hovering it counts as over the UI: the wheel is its.)
            Interaction::default(),
            bevy::ui::RelativeCursorPosition::default(),
            Node {
                position_type: PositionType::Absolute,
                top: px(ARENA_TOP),
                left: px(6),
                // (Its most height: what the window leaves, `show_panel`.)
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                padding: UiRect::all(px(6)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        ))
        .with_children(|p| {
            p.spawn((PanelTitle, Text::new("ARENA"), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(Color::srgb(1.0, 0.85, 0.3))));
            p.spawn((
                PanelBody,
                ScrollPosition::default(),
                Node { flex_direction: FlexDirection::Column, row_gap: px(4), overflow: Overflow::scroll_y(), min_height: px(0), flex_shrink: 1.0, ..default() },
            ))
            .with_children(|p| {
                // Each section: a heading to click, then what's in it.
                let section = |p: &mut ChildSpawnerCommands, i: usize, f: &dyn Fn(&mut ChildSpawnerCommands)| {
                    p.spawn((Button, SectionHead(i), Node { padding: UiRect::axes(px(2), px(1)), ..default() }, BackgroundColor(Color::NONE)))
                        .with_children(|b| {
                            b.spawn((SectionHead(i), Text::new(SECTIONS[i].0), TextFont { font_size: FontSize::Px(11.0), ..default() }, TextColor(Color::srgb(0.75, 0.75, 0.8))));
                        });
                    p.spawn((SectionBody(i), Node { flex_direction: FlexDirection::Column, row_gap: px(3), ..default() })).with_children(|b| f(b));
                };
                let sub = |p: &mut ChildSpawnerCommands, text: &str| {
                    p.spawn((Text::new(text), TextFont { font_size: FontSize::Px(10.0), ..default() }, TextColor(Color::srgb(0.6, 0.6, 0.65))));
                };
                section(p, 0, &|p| {
                    row(p, &|r| {
                        label(r, "Pause  P", ArenaAction::Pause);
                        label(r, "Step  .", ArenaAction::Step);
                    });
                    row(p, &|r| {
                        for (s, t) in SPEEDS.iter().zip(["1x", "1/2", "1/4", "1/10"]) {
                            label(r, t, ArenaAction::Speed(*s));
                        }
                    });
                });
                section(p, 1, &|p| {
                    row(p, &|r| {
                        for (i, name) in tempos.iter().enumerate() {
                            label(r, name, ArenaAction::Tempo(i));
                        }
                    });
                });
                section(p, 2, &|p| row(p, &|r| label(r, "Boxes and hands  Y", ArenaAction::Overlays)));
                section(p, 3, &|p| {
                    row(p, &|r| {
                        for k in kinds() {
                            label(r, &k, ArenaAction::Pick(k.clone()));
                        }
                    });
                    row(p, &|r| label(r, "Clear the floor", ArenaAction::Clear));
                });
                section(p, 4, &|p| row(p, &|r| label(r, "Art editor  E", ArenaAction::Editor)));
                // Sounds, grouped: hits, steps and moving, hands, the rest.
                section(p, 5, &|p| {
                    let groups: [SoundGroup; 4] = [
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
                        sub(p, name);
                        row(p, &|r| {
                            for n in &these {
                                label(r, n, ArenaAction::Sound((*n).clone()));
                            }
                        });
                    }
                });
            });
        });
}

const IDLE: Color = Color::srgba(0.25, 0.25, 0.3, 0.8);
const PICKED: Color = Color::srgba(0.55, 0.45, 0.15, 0.9);

#[allow(clippy::too_many_arguments)]
fn show_panel(
    view: Res<ArenaView>,
    virt: Res<Time<Virtual>>,
    kind: Res<SpawnKind>,
    tempo: Res<crate::tempo::Tempo>,
    windows: Query<&Window>,
    inventory: Res<crate::hands::ui::InventoryOpen>,
    mut panel: Query<(&mut Visibility, &mut Node), With<Panel>>,
    mut title: Query<&mut Text, With<PanelTitle>>,
    mut buttons: Query<(&Interaction, &PanelButton, &mut BackgroundColor)>,
) {
    // No taller than the window leaves: its top at 150, the hotbar below.
    // (Under the hotbar, above the debug text; away while the inventory's
    // open, which is where it'd be.)
    let most = windows.iter().next().map_or(600.0, |w| (w.height() - ARENA_TOP - 150.0).max(120.0));
    for (mut v, mut node) in &mut panel {
        *v = if view.open && !inventory.0 { Visibility::Visible } else { Visibility::Hidden };
        if node.max_height != px(most) {
            node.max_height = px(most);
        }
    }
    let speed = virt.relative_speed();
    for mut t in &mut title {
        let state = if virt.is_paused() { "  paused".to_string() } else if speed < 1.0 { format!("  x{speed}") } else { String::new() };
        t.0 = format!("ARENA  {}{state}", tempo.name());
    }
    for (i, b, mut bg) in &mut buttons {
        let on = match &b.0 {
            ArenaAction::Speed(s) => (s - speed).abs() < 1e-3,
            ArenaAction::Pause => virt.is_paused(),
            ArenaAction::Overlays => view.overlays,
            ArenaAction::Pick(k) => *k == kind.0,
            ArenaAction::Tempo(i) => *i == tempo.active,
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
