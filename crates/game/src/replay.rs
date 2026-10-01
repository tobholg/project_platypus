//! Fights recorded and played back (DESIGN §14.7, arena v2): a fight
//! becomes a test.
//!
//! Recording (the arena panel's "Record", "Stop and save"): everything is
//! reset (the world as the layout makes it, a new player, tick 0, the
//! creature numbers from 1: `creatures::Stable`), then each frame is kept:
//! how long it was, the time's speed (slow motion, paused), the keys and
//! mouse buttons held (play's keys only: not the arena's and dev's), and
//! where the cursor pointed in the world; and each creature put down (O,
//! the bestiary's Place) as where and what. Saved, with how it ended (the
//! player's health and deaths, who's left and their health, what the Fight
//! readout added up), to `saves/fights/fight-N.ron`.
//!
//! Playing back (the panel's "Replay the last", or `PLATYPUS_REPLAY=file`
//! headless as a test: exits 0 if it ended as recorded, 1 if not): the same
//! reset, the same layout, then each frame as it was: the same length (the
//! clock stepped by hand, so the sim's ticks fall on the same frames), the
//! same keys and buttons, the cursor pointed the same, the same creatures
//! put down; then how it ended, held against the recording (the same dead,
//! health within `CLOSE` of a whole health, the readout's totals within
//! `CLOSE` of theirs).

use std::time::Duration;

use bevy::input::InputSystems;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use serde::{Deserialize, Serialize};

use crate::creatures::player::LocalPlayer;
use crate::creatures::{Creature, Health};

/// How near a replay's ending must come to the recording's (a share).
const CLOSE: f32 = 0.15;

/// Play's keys, as recorded (by their place here): not the arena's (P . ,
/// Y T O), dev's or the function keys.
const KEYS: [KeyCode; 46] = [
    KeyCode::KeyA,
    KeyCode::KeyB,
    KeyCode::KeyC,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::KeyH,
    KeyCode::KeyI,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::KeyN,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyS,
    KeyCode::KeyU,
    KeyCode::KeyV,
    KeyCode::KeyW,
    KeyCode::KeyX,
    KeyCode::KeyZ,
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
    KeyCode::Space,
    KeyCode::ShiftLeft,
    KeyCode::ShiftRight,
    KeyCode::ControlLeft,
    KeyCode::ControlRight,
    KeyCode::AltLeft,
    KeyCode::AltRight,
    KeyCode::SuperLeft,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::Tab,
    KeyCode::Minus,
];
const BUTTONS: [MouseButton; 3] = [MouseButton::Left, MouseButton::Right, MouseButton::Middle];

/// One frame, as it was.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Frame {
    /// Its length (real seconds), the time's speed, paused or not.
    pub dt: f32,
    pub speed: f32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub paused: bool,
    /// Keys held (their places in `KEYS`), buttons held (bits), the cursor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<u8>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub buttons: u8,
    pub cursor: Option<(f32, f32)>,
}

fn is_zero(v: &u8) -> bool {
    *v == 0
}

/// How a fight ended.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Outcome {
    pub player_hp: f32,
    pub player_deaths: u32,
    /// Who's left: kind, health, its most; sorted.
    pub left: Vec<(String, f32, f32)>,
    /// The Fight readout's totals: to them, to you.
    pub to_them: f32,
    pub to_you: f32,
}

impl Outcome {
    /// Where a replay's ending differs from the recording's (empty: it
    /// matched).
    pub fn differences(&self, recorded: &Outcome) -> Vec<String> {
        let mut out = Vec::new();
        if self.player_deaths != recorded.player_deaths {
            out.push(format!("the player died {} times, not {}", self.player_deaths, recorded.player_deaths));
        }
        let kinds = |o: &Outcome| o.left.iter().map(|(k, ..)| k.clone()).collect::<Vec<_>>();
        if kinds(self) != kinds(recorded) {
            out.push(format!("left standing: {:?}, not {:?}", kinds(self), kinds(recorded)));
        } else {
            for ((k, hp, max), (_, was, _)) in self.left.iter().zip(&recorded.left) {
                if (hp - was).abs() > CLOSE * max {
                    out.push(format!("{k} at {hp:.0}, not {was:.0}"));
                }
            }
        }
        if (self.player_hp - recorded.player_hp).abs() > CLOSE * 100.0 {
            out.push(format!("the player at {:.0}, not {:.0}", self.player_hp, recorded.player_hp));
        }
        let near = |a: f32, b: f32| (a - b).abs() <= (CLOSE * b.abs()).max(10.0);
        if !near(self.to_them, recorded.to_them) || !near(self.to_you, recorded.to_you) {
            out.push(format!("dealt {:.0} and took {:.0}, not {:.0} and {:.0}", self.to_them, self.to_you, recorded.to_them, recorded.to_you));
        }
        out
    }
}

/// A fight on file.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FightFile {
    /// The arena's layout (by name: `worldgen::arena::Layout::parse`).
    pub layout: String,
    /// The fixed clock's leftover at the first frame (s).
    pub overstep: f32,
    pub frames: Vec<Frame>,
    /// Creatures put down: the frame, what, where (feet).
    #[serde(default)]
    pub spawns: Vec<(u32, String, (f32, f32))>,
    #[serde(default)]
    pub outcome: Outcome,
}

/// What the tape is doing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Idle,
    /// Asked for: waiting for the reset to happen first.
    Arming { play: bool },
    Recording,
    Playing,
    /// Asked to stop: how it ended to be written down (or compared).
    Ending { play: bool },
}

#[derive(Resource, Default)]
pub struct Tape {
    pub mode: Mode,
    pub file: FightFile,
    /// The frame it's at, and what it says (the Fight section shows it).
    pub frame: usize,
    pub status: String,
    /// Where it's kept (and, as a test, whether to exit when it's done).
    path: Option<std::path::PathBuf>,
    exit: bool,
}

impl Tape {
    pub fn recording(&self) -> bool {
        self.mode == Mode::Recording
    }

    pub fn playing(&self) -> bool {
        self.mode == Mode::Playing
    }

    /// The creatures recorded as put down this frame (playing back).
    pub fn spawns_now(&self) -> Vec<(String, Vec2)> {
        self.file.spawns.iter().filter(|s| s.0 as usize + 1 == self.frame).map(|(_, k, (x, y))| (k.clone(), Vec2::new(*x, *y))).collect()
    }

    /// A creature put down, recording.
    pub fn note_spawn(&mut self, kind: &str, at: Vec2) {
        let frame = self.frame.saturating_sub(1) as u32;
        self.file.spawns.push((frame, kind.to_string(), (at.x, at.y)));
    }
}

/// Ask the tape for something (the arena panel).
#[derive(Message, Clone, Debug)]
pub enum TapeAction {
    Record,
    Stop,
    /// Play this file back (none: the last recorded).
    Replay(Option<std::path::PathBuf>),
}

pub struct ReplayPlugin;

impl Plugin for ReplayPlugin {
    fn build(&self, app: &mut App) {
        let mut tape = Tape::default();
        // As a test: played back headless, then out with how it went.
        if let Ok(path) = std::env::var("PLATYPUS_REPLAY") {
            tape.path = Some(path.into());
            tape.exit = true;
        }
        app.insert_resource(tape)
            .add_message::<TapeAction>()
            .add_systems(PreUpdate, frames.after(InputSystems).after(crate::camera::track_cursor).after(crate::editor::capture).before(crate::creatures::player::sample_keys))
            .add_systems(Update, (asked, start_on_reset, end).chain())
            .add_systems(Last, next_frame);
    }
}

fn fights_dir() -> std::path::PathBuf {
    crate::save::saves_dir().join("fights")
}

/// The last fight recorded.
fn last_fight() -> Option<std::path::PathBuf> {
    let mut all: Vec<_> = std::fs::read_dir(fights_dir()).ok()?.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "ron")).collect();
    all.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
    all.pop()
}

/// What's asked (the panel; or, as a test, a moment after it starts).
#[allow(clippy::too_many_arguments)]
fn asked(
    time: Res<Time<Real>>,
    mut tape: ResMut<Tape>,
    mut acts: MessageReader<TapeAction>,
    mut resets: MessageWriter<crate::reset::ResetNow>,
    mut sim: ResMut<crate::world::SimWorld>,
    mut layout: ResMut<crate::arena::CurrentLayout>,
    player: Query<(), With<LocalPlayer>>,
    mut started: Local<bool>,
) {
    let mut wanted: Vec<TapeAction> = acts.read().cloned().collect();
    if tape.exit && !*started && time.elapsed_secs() > 1.0 && !player.is_empty() {
        *started = true;
        wanted.push(TapeAction::Replay(tape.path.clone()));
    }
    for a in wanted {
        match a {
            TapeAction::Record if tape.mode == Mode::Idle => {
                tape.file = FightFile { layout: layout.0.clone(), ..default() };
                tape.mode = Mode::Arming { play: false };
                tape.status = "Recording: starting afresh".into();
                resets.write(crate::reset::ResetNow(crate::reset::Reset::Everything));
            }
            TapeAction::Stop if tape.mode == Mode::Recording => tape.mode = Mode::Ending { play: false },
            TapeAction::Stop if tape.mode == Mode::Playing => tape.mode = Mode::Ending { play: true },
            TapeAction::Replay(path) if tape.mode == Mode::Idle => {
                let Some(path) = path.or_else(last_fight) else {
                    tape.status = "No fight recorded yet".into();
                    continue;
                };
                let file: FightFile = match crate::data::load_ron(&path) {
                    Ok(f) => f,
                    Err(e) => {
                        tape.status = format!("Couldn't read it: {e}");
                        error!("replay: {e}");
                        continue;
                    }
                };
                // Its layout, then the same fresh start.
                if file.layout != layout.0 {
                    let arena = platypus_worldgen::ArenaGen::with_layout(sim.materials(), platypus_worldgen::arena::Layout::parse(&file.layout));
                    sim.generator = std::sync::Arc::new(arena);
                    layout.0.clone_from(&file.layout);
                }
                info!("replay: {} ({} frames, {} creatures put down)", path.display(), file.frames.len(), file.spawns.len());
                tape.status = format!("Replaying {}", path.file_stem().unwrap_or_default().to_string_lossy());
                tape.file = file;
                tape.path = Some(path);
                tape.mode = Mode::Arming { play: true };
                resets.write(crate::reset::ResetNow(crate::reset::Reset::Everything));
            }
            _ => {}
        }
    }
}

/// The reset done: frame 0 is the next.
fn start_on_reset(mut tape: ResMut<Tape>, mut done: MessageReader<crate::reset::WasReset>, mut fight: MessageWriter<crate::fight::NewFight>) {
    // (A fight starts from everything made afresh.)
    if !done.read().any(|r| r.0 == crate::reset::Reset::Everything) {
        return;
    }
    if let Mode::Arming { play } = tape.mode {
        tape.mode = if play { Mode::Playing } else { Mode::Recording };
        tape.frame = 0;
        fight.write(crate::fight::NewFight);
    }
}

/// Each frame, kept (recording) or given back (playing).
#[allow(clippy::too_many_arguments)]
fn frames(
    mut tape: ResMut<Tape>,
    real: Res<Time<Real>>,
    virt: Res<Time<Virtual>>,
    mut fixed: ResMut<Time<Fixed>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    cursor: Res<crate::camera::CursorWorld>,
    mut aim: ResMut<crate::camera::CursorOverride>,
    over_ui: Res<crate::dev::PointerOverUi>,
    taken: Res<crate::dev::KeyboardTaken>,
) {
    match tape.mode {
        Mode::Recording => {
            // (Keys a screen has, and clicks on the panels, did nothing in
            // play: none.)
            let held_keys = if taken.0 { Vec::new() } else { KEYS.iter().enumerate().filter(|(_, k)| keys.pressed(**k)).map(|(i, _)| i as u8).collect() };
            let buttons = if over_ui.0 { 0 } else { BUTTONS.iter().enumerate().filter(|(_, b)| mouse.pressed(**b)).fold(0, |m, (i, _)| m | 1 << i) };
            if tape.frame == 0 {
                tape.file.overstep = fixed.overstep().as_secs_f32();
            }
            let f = Frame { dt: real.delta_secs(), speed: virt.relative_speed(), paused: virt.is_paused(), keys: held_keys, buttons, cursor: cursor.0.map(|c| (c.x, c.y)) };
            tape.file.frames.push(f);
            tape.frame += 1;
            tape.status = format!("Recording  {:.1} s", tape.file.frames.iter().map(|f| f.dt).sum::<f32>());
        }
        Mode::Playing => {
            let Some(f) = tape.file.frames.get(tape.frame).cloned() else {
                tape.mode = Mode::Ending { play: true };
                return;
            };
            if tape.frame == 0 {
                let left = fixed.overstep();
                fixed.discard_overstep(left);
                fixed.accumulate_overstep(Duration::from_secs_f32(tape.file.overstep));
            }
            for (i, k) in KEYS.iter().enumerate() {
                match (f.keys.contains(&(i as u8)), keys.pressed(*k)) {
                    (true, false) => keys.press(*k),
                    (false, true) => keys.release(*k),
                    _ => {}
                }
            }
            for (i, b) in BUTTONS.iter().enumerate() {
                match (f.buttons & (1 << i) != 0, mouse.pressed(*b)) {
                    (true, false) => mouse.press(*b),
                    (false, true) => mouse.release(*b),
                    _ => {}
                }
            }
            aim.0 = f.cursor.map(|(x, y)| Vec2::new(x, y));
            tape.frame += 1;
            let n = tape.file.frames.len().max(1);
            tape.status = format!("Replaying  {} %", tape.frame * 100 / n);
        }
        _ => {}
    }
}

/// The next frame's length and speed, set before it starts (playing).
fn next_frame(tape: Res<Tape>, mut strategy: ResMut<TimeUpdateStrategy>, mut virt: ResMut<Time<Virtual>>) {
    let next = match tape.mode {
        Mode::Playing => tape.file.frames.get(tape.frame),
        _ => None,
    };
    match next {
        Some(f) => {
            *strategy = TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(f.dt));
            if (virt.relative_speed() - f.speed).abs() > 1e-4 {
                virt.set_relative_speed(f.speed);
            }
            match (f.paused, virt.is_paused()) {
                (true, false) => virt.pause(),
                (false, true) => virt.unpause(),
                _ => {}
            }
        }
        None => {
            if !matches!(*strategy, TimeUpdateStrategy::Automatic) {
                *strategy = TimeUpdateStrategy::Automatic;
            }
        }
    }
}

type Others = (Without<LocalPlayer>, Without<crate::creatures::brain::Staged>);

/// How it ended: written down (recording), or held against the recording
/// (playing).
#[allow(clippy::too_many_arguments)]
fn end(
    mut tape: ResMut<Tape>,
    player: Query<&Health, With<LocalPlayer>>,
    creatures: Query<(&Creature, &Health), Others>,
    deaths: Res<crate::creatures::PlayerDeaths>,
    fight: Res<crate::fight::Fight>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut aim: ResMut<crate::camera::CursorOverride>,
    mut exit: MessageWriter<AppExit>,
) {
    let Mode::Ending { play } = tape.mode else { return };
    let mut left: Vec<(String, f32, f32)> = creatures.iter().map(|(c, h)| (c.kind.clone(), h.hp.max(0.0), h.max)).collect();
    left.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let outcome = Outcome { player_hp: player.single().map_or(0.0, |h| h.hp), player_deaths: deaths.0, left, to_them: fight.to_them.total(), to_you: fight.to_you.total() };
    tape.mode = Mode::Idle;
    if !play {
        tape.file.outcome = outcome;
        let dir = fights_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = (1..).map(|n| dir.join(format!("fight-{n}.ron"))).find(|p| !p.exists()).expect("a free name");
        let text = ron::ser::to_string_pretty(&tape.file, ron::ser::PrettyConfig::default().compact_arrays(true)).unwrap_or_default();
        match std::fs::write(&path, text) {
            Ok(()) => {
                info!("replay: recorded {} ({} frames)", path.display(), tape.file.frames.len());
                tape.status = format!("Saved {}", path.file_name().unwrap_or_default().to_string_lossy());
            }
            Err(e) => tape.status = format!("Couldn't save it: {e}"),
        }
        return;
    }
    // Played back: hands off the keys and the cursor again.
    keys.release_all();
    mouse.release_all();
    aim.0 = None;
    let differences = outcome.differences(&tape.file.outcome);
    if differences.is_empty() {
        info!("replay: PASS: it ended as recorded ({outcome:?})");
        tape.status = "Replayed: it ended as recorded".into();
    } else {
        warn!("replay: FAIL: {}", differences.join("; "));
        info!("replay: recorded {:?}", tape.file.outcome);
        info!("replay: replayed {outcome:?}");
        tape.status = format!("Replayed: {}", differences.join("; "));
    }
    if tape.exit {
        exit.write(if differences.is_empty() { AppExit::Success } else { AppExit::from_code(1) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fight_reads_back_and_an_ending_is_held_against_it() {
        let recorded = Outcome { player_hp: 80.0, player_deaths: 0, left: vec![("orc".into(), 30.0, 70.0)], to_them: 100.0, to_you: 20.0 };
        let file = FightFile { layout: "flat".into(), overstep: 0.004, frames: vec![Frame { dt: 0.016, speed: 1.0, keys: vec![3, 32], buttons: 1, cursor: Some((1.0, 2.0)), ..default() }], spawns: vec![(4, "orc".into(), (10.0, 20.0))], outcome: recorded.clone() };
        let text = ron::to_string(&file).expect("writes");
        let back: FightFile = ron::from_str(&text).expect("reads");
        assert_eq!(back.outcome, recorded);
        assert_eq!(back.frames[0].keys, vec![3, 32]);
        assert_eq!(KEYS[3], KeyCode::KeyD);
        // Near enough: the same.
        let close = Outcome { player_hp: 76.0, to_them: 108.0, left: vec![("orc".into(), 36.0, 70.0)], ..recorded.clone() };
        assert!(close.differences(&recorded).is_empty());
        // Another ending: said how.
        let dead = Outcome { left: vec![], player_deaths: 1, ..recorded.clone() };
        let d = dead.differences(&recorded);
        assert_eq!(d.len(), 2, "{d:?}");
        assert!(Outcome { player_hp: 40.0, ..recorded.clone() }.differences(&recorded)[0].contains("player at 40"));
    }
}
