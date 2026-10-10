//! Dev actions (weather, time, lighting, overlays, spawning) from keys or the
//! dev panel's buttons, so nothing depends on the keyboard layout: the panel
//! shows in dev mode (the key left of 1, or F1), letters work on any layout,
//! and F-keys (which on a Mac need fn) still do too.
//!
//! Dev mode: U (or F2) to the start · V storm · B clear sky · N lightning at the cursor · M +3 hours ·
//! K lighting on/off · H performance HUD · J chunk overlay. Always: L
//! what you carry for light (nothing, a small beam, a big one, a torch in
//! the off hand) · G plant a torch · O spawn a warband (a troll, orcs, archers:
//! `packs.ron`; or what
//! the arena panel picked) at the cursor · P the show's next act
//! (`show.rs`; not with the arena panel up, where it pauses).

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
    /// What the player carries for light: none, a small beam, a big one, a torch.
    Flashlight,
    PlantTorch(Option<Vec2>),
    /// The kind picked to spawn (an orc unless the arena picked another).
    Spawn(Option<Vec2>),
    PerfHud,
    Chunks,
    Radius(i32),
    Hands,
    /// The arena panel (time, overlays, spawning), anywhere.
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
}

/// A screen has the keyboard (the art editor): the player and the keys
/// that act on the world leave it alone.
#[derive(Resource, Default)]
pub struct KeyboardTaken(pub bool);

/// The pointer is over a UI element: clicks are for it, not the world.
#[derive(Resource, Default)]
pub struct PointerOverUi(pub bool);

#[derive(Component)]
struct Panel;

#[derive(Component)]
struct PanelButton(DevAction);

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DevAction>()
            .init_resource::<PointerOverUi>()
            .init_resource::<KeyboardTaken>()
            .add_systems(Startup, spawn_panel)
            .add_systems(PreUpdate, pointer_over_ui.after(bevy::ui::UiSystems::Focus))
            .add_systems(Update, (keys, buttons, show_panel, to_surface));
    }
}

fn pointer_over_ui(ui: Query<&Interaction>, scripted: Res<crate::camera::CursorOverride>, mut over: ResMut<PointerOverUi>) {
    // (A scenario aiming its own cursor isn't pointing at the UI, wherever
    // the real mouse pointer happens to rest.)
    over.0 = scripted.0.is_none() && ui.iter().any(|i| *i != Interaction::None);
}

fn keys(keys: Res<ButtonInput<KeyCode>>, dev: Res<DevTools>, cursor: Res<CursorWorld>, arena: Res<crate::arena::ArenaView>, taken: Res<KeyboardTaken>, mut out: MessageWriter<DevAction>) {
    let at = cursor.0;
    let pressed = |k: KeyCode| keys.just_pressed(k);
    // F-keys always; the same things on letters in dev mode.
    let pairs = [
        (KeyCode::F5, KeyCode::KeyV, DevAction::Storm),
        (KeyCode::F6, KeyCode::KeyB, DevAction::ClearSky),
        (KeyCode::F7, KeyCode::KeyN, DevAction::Lightning(at)),
        (KeyCode::F8, KeyCode::KeyM, DevAction::Later),
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
        (KeyCode::KeyL, DevAction::Flashlight),
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

fn buttons(clicks: Query<(&Interaction, &PanelButton), Changed<Interaction>>, mut out: MessageWriter<DevAction>) {
    for (i, b) in &clicks {
        if *i == Interaction::Pressed {
            out.write(b.0);
        }
    }
}

fn spawn_panel(mut commands: Commands) {
    let entries: [(&str, DevAction); 21] = [
        ("Show the next creature   P", DevAction::ShowNext),
        ("Show mode: a lively world", DevAction::ShowMode),
        ("To the start   F2 / U", DevAction::Surface),
        ("Storm here   V", DevAction::Storm),
        ("Clear sky   B", DevAction::ClearSky),
        ("Lightning   N", DevAction::Lightning(None)),
        ("+3 hours   M", DevAction::Later),
        ("A day ahead", DevAction::DayAhead),
        ("An event (star, raid, quake, pedlar)", DevAction::Event),
        ("Lighting on/off   K", DevAction::Lighting),
        ("Light: beam, big, torch   L", DevAction::Flashlight),
        ("Plant a torch   G", DevAction::PlantTorch(None)),
        ("Spawn a warband   O", DevAction::Spawn(None)),
        ("Performance HUD   H", DevAction::PerfHud),
        ("Chunk overlay   J", DevAction::Chunks),
        ("Radius -   wheel", DevAction::Radius(-1)),
        ("Radius +   wheel", DevAction::Radius(1)),
        ("Arena tools", DevAction::Arena),
        ("Back to hands   key left of 1", DevAction::Hands),
        ("Reset the world (twice)", DevAction::ResetWorld),
        ("Reset everything (twice)", DevAction::ResetAll),
    ];
    commands
        .spawn((
            Panel,
            Visibility::Hidden,
            Node {
                // (Under the player's hearts, stars and bolts: `hud.rs`.)
                position_type: PositionType::Absolute,
                top: px(170),
                right: px(8),
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                padding: UiRect::all(px(6)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        ))
        .with_children(|p| {
            p.spawn((Text::new("DEV"), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(Color::srgb(1.0, 0.85, 0.3))));
            for (label, action) in entries {
                p.spawn((
                    Button,
                    PanelButton(action),
                    Node { padding: UiRect::axes(px(8), px(3)), ..default() },
                    BackgroundColor(Color::srgba(0.25, 0.25, 0.3, 0.8)),
                ))
                .with_children(|b| {
                    let mut t = b.spawn((Text::new(label), TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(Color::WHITE)));
                    // (The resets ask to be sure: `reset.rs` relabels them.)
                    match action {
                        DevAction::ResetWorld => {
                            t.insert(crate::reset::ResetLabel(crate::reset::Reset::World));
                        }
                        DevAction::ResetAll => {
                            t.insert(crate::reset::ResetLabel(crate::reset::Reset::Everything));
                        }
                        _ => {}
                    }
                });
            }
        });
}

fn show_panel(
    dev: Res<DevTools>,
    mut panel: Query<&mut Visibility, With<Panel>>,
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<PanelButton>>,
) {
    for mut v in &mut panel {
        *v = if dev.0 { Visibility::Visible } else { Visibility::Hidden };
    }
    for (i, mut bg) in &mut buttons {
        bg.0 = match i {
            Interaction::Pressed => Color::srgba(0.6, 0.5, 0.2, 0.9),
            Interaction::Hovered => Color::srgba(0.35, 0.35, 0.42, 0.9),
            Interaction::None => Color::srgba(0.25, 0.25, 0.3, 0.8),
        };
    }
}
