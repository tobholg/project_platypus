//! Dev actions (weather, time, lighting, overlays, spawning) from keys or the
//! dev panel's buttons, so nothing depends on the keyboard layout: the panel
//! shows in dev mode (the key left of 1, or F1), letters work on any layout,
//! and F-keys (which on a Mac need fn) still do too.
//!
//! Dev mode: V storm · B clear sky · N lightning at the cursor · M +3 hours ·
//! K lighting on/off · H performance HUD · J chunk overlay. Always: L
//! flashlight · T carry a torch · G plant a torch · O spawn an orc (or what
//! the arena panel picked) at the cursor.

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
    Flashlight,
    Torch,
    PlantTorch(Option<Vec2>),
    /// The kind picked to spawn (an orc unless the arena picked another).
    Spawn(Option<Vec2>),
    PerfHud,
    Chunks,
    Radius(i32),
    Hands,
    /// The arena panel (time, overlays, spawning), anywhere.
    Arena,
}

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
            .add_systems(Startup, spawn_panel)
            .add_systems(PreUpdate, pointer_over_ui.after(bevy::ui::UiSystems::Focus))
            .add_systems(Update, (keys, buttons, show_panel));
    }
}

fn pointer_over_ui(ui: Query<&Interaction>, scripted: Res<crate::camera::CursorOverride>, mut over: ResMut<PointerOverUi>) {
    // (A scenario aiming its own cursor isn't pointing at the UI, wherever
    // the real mouse pointer happens to rest.)
    over.0 = scripted.0.is_none() && ui.iter().any(|i| *i != Interaction::None);
}

fn keys(keys: Res<ButtonInput<KeyCode>>, dev: Res<DevTools>, cursor: Res<CursorWorld>, mut out: MessageWriter<DevAction>) {
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
    ];
    for (f, letter, action) in pairs {
        if pressed(f) || (dev.0 && pressed(letter)) {
            out.write(action);
        }
    }
    for (k, action) in [
        (KeyCode::KeyL, DevAction::Flashlight),
        (KeyCode::KeyT, DevAction::Torch),
        (KeyCode::KeyG, DevAction::PlantTorch(at)),
        (KeyCode::KeyO, DevAction::Spawn(at)),
    ] {
        if pressed(k) {
            out.write(action);
        }
    }
}

fn buttons(clicks: Query<(&Interaction, &PanelButton), Changed<Interaction>>, mut out: MessageWriter<DevAction>) {
    for (i, b) in &clicks {
        if *i == Interaction::Pressed {
            out.write(b.0);
        }
    }
}

fn spawn_panel(mut commands: Commands) {
    let entries: [(&str, DevAction); 15] = [
        ("Storm here   V", DevAction::Storm),
        ("Clear sky   B", DevAction::ClearSky),
        ("Lightning   N", DevAction::Lightning(None)),
        ("+3 hours   M", DevAction::Later),
        ("Lighting on/off   K", DevAction::Lighting),
        ("Flashlight   L", DevAction::Flashlight),
        ("Carry a torch   T", DevAction::Torch),
        ("Plant a torch   G", DevAction::PlantTorch(None)),
        ("Spawn (an orc)   O", DevAction::Spawn(None)),
        ("Performance HUD   H", DevAction::PerfHud),
        ("Chunk overlay   J", DevAction::Chunks),
        ("Radius -   wheel", DevAction::Radius(-1)),
        ("Radius +   wheel", DevAction::Radius(1)),
        ("Arena tools", DevAction::Arena),
        ("Back to hands   key left of 1", DevAction::Hands),
    ];
    commands
        .spawn((
            Panel,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                top: px(8),
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
                    b.spawn((Text::new(label), TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(Color::WHITE)));
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
