//! The title screen (PLAN.md, the prologue arc: `play-mode`). Started
//! plainly (no `PLATYPUS_WORLD`, scenario or save named), the game opens
//! the last save there is (or, with none, a new game's world) paused
//! behind the title, and asks:
//!
//! - **Continue**: that save, as it was (the title steps away).
//! - **New game**: a new world (a new seed, the large preset) in a new
//!   play save (`play-1`, `play-2`...), the `play` kit (`mode.rs`).
//! - **Load**: any save there is, the last saved first.
//! - **Dev world**: the dev save (`large-1`) in dev mode.
//! - **Arena**: the sandbox.
//! - **Quit**.
//!
//! Another world than the one behind the title starts the game again with
//! it (the same program, `PLATYPUS_WORLD`, `PLATYPUS_SEED`, `PLATYPUS_SAVE`
//! and `PLATYPUS_MODE` set) and this one ends, unsaved. The game's name is
//! `assets/data/game.ron`. `PLATYPUS_TITLE=1` shows the title anyway (over
//! whatever world), `0` never.

use bevy::prelude::*;
use serde::Deserialize;

use crate::dev::KeyboardTaken;
use crate::mode::Mode;
use crate::save::SlotInfo;

/// The game's name and its line under it (`game.ron`).
#[derive(Resource, Clone, Debug, Deserialize)]
pub struct GameInfo {
    pub title: String,
    #[serde(default)]
    pub tagline: String,
}

impl GameInfo {
    pub fn load() -> GameInfo {
        crate::data::load_ron(&crate::data::data_path("game.ron")).unwrap_or_else(|e| {
            warn!("{e}");
            GameInfo { title: "Project Platypus".into(), tagline: String::new() }
        })
    }
}

/// A world to start: its kind, seed, save and mode.
#[derive(Clone, Debug, PartialEq)]
pub struct Launch {
    pub kind: String,
    pub seed: u64,
    pub save: Option<String>,
    pub mode: Mode,
}

impl Launch {
    /// The game again with this world, once this one's ended (and saved,
    /// if it saves).
    fn start(&self) {
        let save = self.save.clone().unwrap_or_else(|| "off".into());
        relaunch(vec![("PLATYPUS_WORLD", Some(self.kind.clone())), ("PLATYPUS_SEED", Some(self.seed.to_string())), ("PLATYPUS_MODE", Some(self.mode.name().into())), ("PLATYPUS_SAVE", Some(save)), ("PLATYPUS_TITLE", None), ("PLATYPUS_FRESH", None)]);
        info!("title: next, {} (seed {}, save {:?}, {})", self.kind, self.seed, self.save, self.mode.name());
    }

    fn of(slot: &SlotInfo) -> Launch {
        Launch { kind: slot.kind.clone(), seed: slot.seed, save: Some(slot.name.clone()), mode: slot.mode }
    }
}

/// What the game is started again with when this one ends: env set (or,
/// `None`, unset). `main` starts it, after the app's done (and saved).
static RELAUNCH: std::sync::Mutex<Option<Vec<(&'static str, Option<String>)>>> = std::sync::Mutex::new(None);

pub fn relaunch(env: Vec<(&'static str, Option<String>)>) {
    if let Ok(mut r) = RELAUNCH.lock() {
        *r = Some(env);
    }
}

/// The title screen again, over the save just left (a plain start).
pub fn relaunch_to_title() {
    relaunch(["PLATYPUS_WORLD", "PLATYPUS_SEED", "PLATYPUS_SAVE", "PLATYPUS_MODE", "PLATYPUS_FRESH", "PLATYPUS_TITLE"].into_iter().map(|k| (k, None)).collect());
}

/// After the app: start the game again, if it's been asked for.
pub fn relaunch_if_asked() {
    let Some(env) = RELAUNCH.lock().ok().and_then(|mut r| r.take()) else { return };
    let Ok(exe) = std::env::current_exe() else { return eprintln!("title: where's the game?") };
    let mut cmd = std::process::Command::new(exe);
    for (k, v) in env {
        match v {
            Some(v) => cmd.env(k, v),
            None => cmd.env_remove(k),
        };
    }
    if let Err(e) = cmd.spawn() {
        eprintln!("title: {e}");
    }
}

/// What a plain start opens behind the title: the last save, or a new
/// game's world (and whether it's that: new, never saved).
pub fn behind_the_title() -> (Launch, bool) {
    match crate::save::slots().first() {
        Some(last) => (Launch::of(last), false),
        None => (new_game(), true),
    }
}

/// A new game: a new seed, the large world, the first free play save.
fn new_game() -> Launch {
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(7, |d| d.as_nanos() as u64 % 1_000_000_007);
    let name = (1..).map(|n| format!("play-{n}")).find(|n| !crate::save::saves_dir().join(n).exists()).expect("a free name");
    Launch { kind: "large".into(), seed, save: Some(name), mode: Mode::Play }
}

pub struct TitlePlugin {
    /// Show it.
    pub show: bool,
    /// The world behind it, and whether that's a new game's (unsaved).
    pub behind: Launch,
    pub fresh: bool,
}

/// The title's up (the world behind it paused).
#[derive(Resource)]
pub struct Title {
    pub up: bool,
    behind: Launch,
    fresh: bool,
    /// The saves, listed (Load).
    listing: bool,
}

#[derive(Component, Clone, Debug, PartialEq)]
pub(crate) enum Choice {
    Continue,
    NewGame,
    Load,
    Slot(usize),
    Dev,
    Arena,
    Quit,
}

#[derive(Component)]
struct Root;
#[derive(Component)]
struct SlotList;

/// The saves when the title came up.
#[derive(Resource)]
struct Slots(Vec<SlotInfo>);

impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Picked>()
            .insert_resource(GameInfo::load())
            .insert_resource(Title { up: self.show, behind: self.behind.clone(), fresh: self.fresh, listing: false })
            .insert_resource(Slots(if self.show { crate::save::slots() } else { Vec::new() }))
            .add_systems(Startup, spawn)
            .add_systems(PreUpdate, capture.after(bevy::input::InputSystems).after(crate::editor::capture))
            .add_systems(PreUpdate, choose.after(bevy::ui::UiSystems::Focus))
            .add_systems(Update, look);
    }
}

/// While it's up: time stands still behind it, and the keyboard is its
/// (Enter: the first choice).
pub(crate) fn capture(mut title: ResMut<Title>, mut keys: ResMut<ButtonInput<KeyCode>>, mut taken: ResMut<KeyboardTaken>, mut virt: ResMut<Time<Virtual>>, mut first: Local<bool>, mut picks: MessageWriter<Picked>) {
    if !title.up {
        return;
    }
    if !*first {
        *first = true;
        virt.pause();
    }
    taken.0 = true;
    if keys.just_pressed(KeyCode::Enter) {
        picks.write(Picked(if title.fresh { Choice::NewGame } else { Choice::Continue }));
    }
    if keys.just_pressed(KeyCode::Escape) {
        title.listing = false;
    }
    keys.reset_all();
}

/// A choice made (a click, or Enter).
#[derive(Message, Clone, Debug)]
pub(crate) struct Picked(pub(crate) Choice);

fn ago(t: std::time::SystemTime) -> String {
    let s = std::time::SystemTime::now().duration_since(t).map_or(0, |d| d.as_secs());
    match s {
        0..60 => "just now".into(),
        60..3600 => format!("{} min ago", s / 60),
        3600..86400 => format!("{} h ago", s / 3600),
        _ => format!("{} days ago", s / 86400),
    }
}

fn label(slot: &SlotInfo) -> String {
    format!("{}   {} {}, seed {}, {}", slot.name, slot.mode.name(), slot.kind, slot.seed, ago(slot.saved))
}

const IDLE: Color = Color::srgba(0.1, 0.09, 0.12, 0.0);
const HOVER: Color = Color::srgba(0.95, 0.8, 0.45, 0.16);
const GOLD: Color = Color::srgb(0.98, 0.86, 0.55);

fn spawn(mut commands: Commands, title: Res<Title>, info: Res<GameInfo>, slots: Res<Slots>) {
    if !title.up {
        return;
    }
    let others = slots.0.iter().filter(|s| Some(&s.name) != title.behind.save.as_ref()).count();
    let mut choices: Vec<(String, Choice)> = Vec::new();
    if !title.fresh {
        let what = slots.0.first().map_or_else(String::new, |s| format!("   {}, {}", s.name, ago(s.saved)));
        choices.push((format!("Continue{what}"), Choice::Continue));
    }
    choices.push(("New game".into(), Choice::NewGame));
    if others > 0 {
        choices.push((format!("Load a save   ({others} more)"), Choice::Load));
    }
    choices.push(("Dev world".into(), Choice::Dev));
    choices.push(("Arena".into(), Choice::Arena));
    choices.push(("Quit".into(), Choice::Quit));
    commands
        .spawn((
            Root,
            GlobalZIndex(60),
            // (Over the HUD: the pointer on it is the title's.)
            Interaction::default(),
            Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), flex_direction: FlexDirection::Column, justify_content: JustifyContent::Center, padding: UiRect::left(px(120)), row_gap: px(6), ..default() },
            BackgroundGradient::from(LinearGradient::to_right(vec![ColorStop::auto(Color::srgba(0.02, 0.02, 0.05, 0.92)), ColorStop::percent(Color::srgba(0.02, 0.02, 0.05, 0.55), 45.0), ColorStop::auto(Color::srgba(0.02, 0.02, 0.05, 0.0))])),
        ))
        .with_children(|p| {
            p.spawn((Text::new(info.title.to_uppercase()), TextFont { font_size: FontSize::Px(96.0), ..default() }, TextColor(GOLD), TextShadow { offset: Vec2::new(4.0, 5.0), color: Color::srgba(0.0, 0.0, 0.0, 0.8) }));
            if !info.tagline.is_empty() {
                p.spawn((Text::new(info.tagline.clone()), TextFont { font_size: FontSize::Px(18.0), ..default() }, TextColor(Color::srgba(0.85, 0.82, 0.75, 0.75)), Node { margin: UiRect { left: px(6), bottom: px(40), ..default() }, ..default() }));
            }
            for (text, choice) in choices {
                p.spawn((Button, choice, Node { padding: UiRect::axes(px(14), px(6)), width: px(560), ..default() }, BackgroundColor(IDLE))).with_children(|b| {
                    b.spawn((Text::new(text), TextFont { font_size: FontSize::Px(26.0), ..default() }, TextColor(Color::srgb(0.93, 0.91, 0.86))));
                });
            }
            p.spawn((SlotList, Node { flex_direction: FlexDirection::Column, row_gap: px(2), margin: UiRect::top(px(16)), display: Display::None, ..default() })).with_children(|l| {
                for (i, s) in slots.0.iter().enumerate() {
                    if Some(&s.name) == title.behind.save.as_ref() {
                        continue;
                    }
                    l.spawn((Button, Choice::Slot(i), Node { padding: UiRect::axes(px(14), px(4)), width: px(560), ..default() }, BackgroundColor(IDLE))).with_children(|b| {
                        b.spawn((Text::new(label(s)), TextFont { font_size: FontSize::Px(17.0), ..default() }, TextColor(Color::srgb(0.85, 0.85, 0.82))));
                    });
                }
            });
            p.spawn((Text::new("Enter: the first   |   the key left of 1, in a dev world: the dev menu"), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(Color::srgba(0.8, 0.8, 0.8, 0.5)), Node { margin: UiRect::top(px(30)), ..default() }));
        });
}

#[allow(clippy::too_many_arguments)]
fn choose(
    mut commands: Commands,
    clicks: Query<(&Interaction, &Choice), Changed<Interaction>>,
    mut picked: MessageReader<Picked>,
    mut title: ResMut<Title>,
    slots: Res<Slots>,
    mut virt: ResMut<Time<Virtual>>,
    mut save: Option<ResMut<crate::save::SaveSlot>>,
    root: Query<Entity, With<Root>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !title.up {
        return;
    }
    // (Nothing's saved while the title's up: a world only looked at isn't
    // kept; nor the one left for another.)
    if let Some(s) = save.as_mut() {
        s.hold = true;
    }
    let mut chosen: Vec<Choice> = clicks.iter().filter(|(i, _)| **i == Interaction::Pressed).map(|(_, c)| c.clone()).collect();
    chosen.extend(picked.read().map(|p| p.0.clone()));
    let Some(choice) = chosen.into_iter().next() else { return };
    let dev = Launch { kind: "large".into(), seed: 1, save: Some("large-1".into()), mode: Mode::Dev };
    let arena = Launch { kind: "arena".into(), seed: 1, save: None, mode: Mode::Dev };
    let go = match choice {
        Choice::Continue => None,
        Choice::NewGame if title.fresh => None,
        Choice::NewGame => Some(new_game()),
        Choice::Load => {
            title.listing = !title.listing;
            return;
        }
        Choice::Slot(i) => slots.0.get(i).map(Launch::of),
        Choice::Dev if title.behind == dev => None,
        Choice::Dev => Some(dev),
        Choice::Arena => Some(arena),
        Choice::Quit => {
            exit.write(AppExit::Success);
            return;
        }
    };
    match go {
        // This world: the title steps away, time goes on, saving too.
        None => {
            title.up = false;
            virt.unpause();
            if let Some(s) = save.as_mut() {
                s.hold = false;
            }
            for e in &root {
                commands.entity(e).despawn();
            }
            info!("title: playing {} ({})", title.behind.save.as_deref().unwrap_or("unsaved"), title.behind.mode.name());
        }
        Some(launch) => {
            launch.start();
            exit.write(AppExit::Success);
        }
    }
}

fn look(title: Res<Title>, mut buttons: Query<(&Interaction, &mut BackgroundColor), With<Choice>>, mut list: Query<&mut Node, With<SlotList>>) {
    if !title.up {
        return;
    }
    for (i, mut bg) in &mut buttons {
        let want = if *i == Interaction::None { IDLE } else { HOVER };
        if bg.0 != want {
            bg.0 = want;
        }
    }
    for mut n in &mut list {
        let want = if title.listing { Display::Flex } else { Display::None };
        if n.display != want {
            n.display = want;
        }
    }
}
