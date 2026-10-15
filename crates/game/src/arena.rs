//! The arena's tools (`PLATYPUS_WORLD=arena`, or the dev menu anywhere:
//! `devmenu.rs`, which has their buttons): time (pause, a tick at a time,
//! slow motion), overlays (every body's box, its facing and hand), what `O`
//! spawns at the cursor, clearing the floor, the arena's floors, and the
//! game's tempo (`tempo.rs`). Keys, in the arena or with the dev menu
//! open: P pause · . one tick (paused) · , slower (1, 1/2, 1/4, 1/10) · Y
//! overlays · T the next tempo · E the art editor (`editor.rs`).
//!
//! Pausing pauses virtual time, so the sim, bodies, particles and
//! animations all stop; a step hands the fixed clock exactly one tick.

use bevy::prelude::*;

use crate::creatures::custom::dummy::Dummy;
use crate::creatures::player::LocalPlayer;
use crate::creatures::spawn::SpawnKind;
use crate::creatures::{Creature, Kinematics, Team};
use crate::creatures::body::animation::HandPos;
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
    /// A tempo preset (`tempo.ron`), or the next.
    Tempo(usize),
    NextTempo,
    /// Play a sound (the sound board).
    Sound(String),
    /// Start the readouts' fight afresh (`fight.rs`).
    NewFight,
    /// The bestiary, open or shut (`bestiary/`).
    Bestiary,
    /// The arena's floor made another way (`worldgen::arena::Layout`, by
    /// name): the world reset, the player kept.
    Layout(String),
    /// A fight recorded, stopped and saved, or the last played back
    /// (`replay.rs`).
    Record,
    Stop,
    ReplayLast,
}

/// The arena's layout now, by name (`PLATYPUS_ARENA`, or the panel's).
#[derive(Resource)]
pub struct CurrentLayout(pub String);

impl Default for CurrentLayout {
    fn default() -> Self {
        CurrentLayout(std::env::var("PLATYPUS_ARENA").unwrap_or_else(|_| "sandbox".into()))
    }
}

#[derive(Resource, Default)]
pub struct ArenaView {
    /// The arena's keys are live (the arena, or the dev menu open).
    pub open: bool,
    /// Bodies' boxes, facing and hands drawn over the world.
    pub overlays: bool,
}

impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ArenaAction>()
            .init_resource::<ArenaView>()
            .init_resource::<CurrentLayout>()
            .init_resource::<StepOwed>()
            .add_systems(PreUpdate, (keys, act).chain().after(bevy::ui::UiSystems::Focus))
            .add_systems(PreUpdate, step.after(act))
            .add_systems(Update, (overlays, spawn_list));
    }
}

/// `PLATYPUS_SPAWN=dread_knight` (or a list, `zombie*3,slime,mimic`): those
/// creatures put down this long after the arena's up (s), on the ground to
/// the player's right, this far off and this far apart (cells); for a
/// quick fight. (Packs too, by name.)
const SPAWN_AFTER: f32 = 1.5;
const SPAWN_OFF: (f32, f32) = (110.0, 40.0);

fn spawn_list(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<SimWorld>,
    creatures: Res<crate::creatures::def::Creatures>,
    player: Query<&Kinematics, With<crate::creatures::player::LocalPlayer>>,
    mut done: Local<bool>,
) {
    if *done || time.elapsed_secs() < SPAWN_AFTER {
        return;
    }
    let Ok(list) = std::env::var("PLATYPUS_SPAWN") else {
        *done = true;
        return;
    };
    let Ok(pk) = player.single() else { return };
    *done = true;
    let packs = crate::creatures::spawn::packs();
    let mut kinds: Vec<String> = Vec::new();
    for word in list.split(',').map(str::trim).filter(|w| !w.is_empty()) {
        let (name, n) = word.split_once('*').map_or((word, 1), |(a, b)| (a.trim(), b.trim().parse().unwrap_or(1)));
        match (creatures.get(name).is_some(), packs.get(name)) {
            (true, _) => kinds.extend(std::iter::repeat_n(name.to_string(), n)),
            (false, Some(pack)) => {
                for _ in 0..n {
                    for (kind, count) in pack {
                        kinds.extend(std::iter::repeat_n(kind.clone(), *count as usize));
                    }
                }
            }
            (false, None) => warn!("PLATYPUS_SPAWN: no creature or pack `{name}` (creatures: {})", creature_kinds().join(", ")),
        }
    }
    let feet = pk.body.pos.y - pk.body.half.y;
    for (i, kind) in kinds.iter().enumerate() {
        let x = pk.body.pos.x + SPAWN_OFF.0 + i as f32 * SPAWN_OFF.1;
        let y = crate::creatures::spawn::find_ground(&sim.world, x as i32, feet as i32 + 120, 400).map_or(feet, |y| y as f32);
        crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(x, y), |_| {});
    }
    if !kinds.is_empty() {
        info!("arena: spawned {}", kinds.join(", "));
    }
}

/// Every creature kind there's a file for (but the player).
pub fn creature_kinds() -> Vec<String> {
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




/// Every creature but the player's.
type Others = (With<Creature>, Without<LocalPlayer>);

/// A tick owed to a paused clock.
#[derive(Resource, Default)]
struct StepOwed(bool);

#[allow(clippy::too_many_arguments)]
fn act(
    mut commands: Commands,
    mut actions: MessageReader<ArenaAction>,
    mut view: ResMut<ArenaView>,
    mut virt: ResMut<Time<Virtual>>,
    mut kind: ResMut<SpawnKind>,
    mut editor: ResMut<crate::editor::Editor>,
    creatures: Query<(Entity, Option<&Dummy>), Others>,
    mut owed: ResMut<StepOwed>,
    mut tempo: ResMut<crate::tempo::Tempo>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut new_fight: MessageWriter<crate::fight::NewFight>,
    mut bestiary: ResMut<crate::bestiary::panel::Bestiary>,
    (mut sim, mut resets, mut current, mut tape): (ResMut<SimWorld>, MessageWriter<crate::reset::ResetNow>, ResMut<CurrentLayout>, MessageWriter<crate::replay::TapeAction>),
) {
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
            ArenaAction::NewFight => {
                new_fight.write(crate::fight::NewFight);
            }
            ArenaAction::Bestiary => bestiary.open = !bestiary.open,
            ArenaAction::Record => {
                tape.write(crate::replay::TapeAction::Record);
            }
            ArenaAction::Stop => {
                tape.write(crate::replay::TapeAction::Stop);
            }
            ArenaAction::ReplayLast => {
                tape.write(crate::replay::TapeAction::Replay(None));
            }
            ArenaAction::Layout(name) => {
                let layout = platypus_worldgen::arena::Layout::parse(name);
                info!("arena: the {} layout", layout.name());
                let arena = platypus_worldgen::ArenaGen::with_layout(sim.materials(), layout);
                sim.generator = std::sync::Arc::new(arena);
                current.0.clone_from(name);
                resets.write(crate::reset::ResetNow(crate::reset::Reset::World));
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
        gizmos.line_2d(p + Vec2::new(h.x * f, 0.0), p + Vec2::new((h.x + 4.5) * f, 0.0), color);
        gizmos.line_2d(p + Vec2::new(-h.x - 1.5, -h.y), p + Vec2::new(h.x + 1.5, -h.y), Color::WHITE);
        if let Some(HandPos { at: Some(at), .. }) = hand {
            gizmos.circle_2d(Isometry2d::from_translation(*at), 1.5, Color::srgb(1.0, 0.9, 0.2));
        }
    }
}
