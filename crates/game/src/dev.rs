//! Dev actions (weather, time, lighting, overlays, spawning) from keys or the
//! dev menu's buttons (`devmenu.rs`), so nothing depends on the keyboard
//! layout. Only in dev mode (`mode.rs`; playing, only F3 the performance
//! HUD).
//!
//! With the dev toolbelt in hand: U (or F2) to the start · V storm · B clear
//! sky · N lightning at the cursor · M +3 hours · K lighting on/off · H
//! performance HUD · J chunk overlay; the F-keys (F2-F9) always. And: G
//! plant a torch · O spawn what the dev menu picked (a
//! warband: `packs.ron`) at the cursor · P the show's next act (`show.rs`;
//! not with the arena's keys live, where it pauses).

use bevy::prelude::*;

use crate::camera::CursorWorld;
use crate::hands::DevTools;

pub struct DevPlugin;

/// Something a dev key or button asks for. Where a place matters, `None`
/// means "by the player" (a button: the mouse is on the panel).
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub enum DevAction {
    Storm,
    ClearSky,
    Lightning(Option<Vec2>),
    Later,
    Lighting,
    PlantTorch(Option<Vec2>),
    /// The kind picked to spawn (an orc unless the arena picked another).
    Spawn(Option<Vec2>),
    PerfHud,
    Chunks,
    Radius(i32),
    Hands,
    /// The dev menu at its Time tab (`devmenu.rs`).
    Arena,
    /// To the start: the surface where the world began (the village).
    Surface,
    /// A day ahead (the world clock catches up: `clock.rs`).
    DayAhead,
    /// An event now, near you (`events.rs`).
    Event,
    /// Reset the world (`reset.rs`; twice to be sure): the land and what's
    /// in it, or everything (you too).
    ResetWorld,
    ResetAll,
    /// The show's next act (`show.rs`), and show mode on or off.
    ShowNext,
    ShowMode,
    /// Alert marks and damage numbers on or off (`display.rs`).
    AlertMarks,
    DamageNumbers,
}

/// A screen has the keyboard (the art editor): the player and the keys
/// that act on the world leave it alone.
#[derive(Resource, Default)]
pub struct KeyboardTaken(pub bool);

/// The pointer is over a UI element: clicks are for it, not the world.
#[derive(Resource, Default)]
pub struct PointerOverUi(pub bool);

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DevAction>()
            .init_resource::<PointerOverUi>()
            .init_resource::<KeyboardTaken>()
            .add_systems(PreUpdate, pointer_over_ui.after(bevy::ui::UiSystems::Focus))
            .add_systems(Update, (keys, to_surface));
    }
}

fn pointer_over_ui(ui: Query<&Interaction>, scripted: Res<crate::camera::CursorOverride>, mut over: ResMut<PointerOverUi>) {
    // (A scenario aiming its own cursor isn't pointing at the UI, wherever
    // the real mouse pointer happens to rest.)
    over.0 = scripted.0.is_none() && ui.iter().any(|i| *i != Interaction::None);
}

fn keys(keys: Res<ButtonInput<KeyCode>>, mode: Res<crate::mode::Mode>, dev: Res<DevTools>, cursor: Res<CursorWorld>, arena: Res<crate::arena::ArenaView>, taken: Res<KeyboardTaken>, mut out: MessageWriter<DevAction>) {
    let at = cursor.0;
    let pressed = |k: KeyCode| keys.just_pressed(k);
    // (Playing: only the performance HUD.)
    if !mode.dev() {
        if pressed(KeyCode::F3) {
            out.write(DevAction::PerfHud);
        }
        return;
    }
    // F-keys always; the same things on letters in dev mode.
    let pairs = [
        (KeyCode::F5, KeyCode::KeyV, DevAction::Storm),
        (KeyCode::F6, KeyCode::KeyB, DevAction::ClearSky),
        (KeyCode::F7, KeyCode::KeyN, DevAction::Lightning(at)),
        // (Later: F8 only; M is the world map's.)
        (KeyCode::F8, KeyCode::F8, DevAction::Later),
        (KeyCode::F9, KeyCode::KeyK, DevAction::Lighting),
        (KeyCode::F3, KeyCode::KeyH, DevAction::PerfHud),
        (KeyCode::F4, KeyCode::KeyJ, DevAction::Chunks),
        (KeyCode::F2, KeyCode::KeyU, DevAction::Surface),
    ];
    for (f, letter, action) in pairs {
        if pressed(f) || (dev.0 && pressed(letter)) {
            out.write(action);
        }
    }
    for (k, action) in [
        (KeyCode::KeyG, DevAction::PlantTorch(at)),
        (KeyCode::KeyO, DevAction::Spawn(at)),
    ] {
        if pressed(k) {
            out.write(action);
        }
    }
    if pressed(KeyCode::KeyP) && !arena.open && !taken.0 {
        out.write(DevAction::ShowNext);
    }
}

/// F2 (U in dev mode): the player to the start, the village, standing on
/// the ground there (as the world first put it down: scanning down from
/// over the spawn). Not loaded yet there: on the ground as generated, and
/// the body climbs out of anything it's in once it loads.
fn to_surface(mut acts: MessageReader<DevAction>, sim: Res<crate::world::SimWorld>, mut player: Query<&mut crate::creatures::Kinematics, With<crate::creatures::player::LocalPlayer>>) {
    if !acts.read().any(|a| *a == DevAction::Surface) {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let s = sim.generator.spawn_point();
    let ground = crate::creatures::spawn::find_ground(&sim.world, s.x, s.y + 180, 900).or_else(|| sim.generator.surface_hint(s.x).map(|y| y + 1)).unwrap_or(s.y);
    let (x, y) = (s.x as f32 + 0.5, ground as f32 + k.body.half.y);
    k.body.pos = Vec2::new(x, y);
    k.body.vel = Vec2::ZERO;
    k.prev_pos = k.body.pos;
    info!("dev: to the start at ({x:.0}, {y:.0})");
}
