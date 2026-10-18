//! Esc, and the game menu. Esc opens the inventory (Terraria's), and shuts
//! it again (the crafting panel first, if it's up); the panels that have
//! the keyboard (the art editor, the bestiary, the title screen, the dev
//! menu) shut first: Esc is theirs. The game menu's opened from the
//! inventory's Settings button (`hands::ui`), and time stands still while
//! it's up. Resume (or Esc) · Save · Main menu (saved, then the title
//! screen over this save: `title.rs`) · Quit (saved). Resumed, it's back
//! to play.

use bevy::prelude::*;

use crate::dev::KeyboardTaken;

pub struct PausePlugin;

/// The game menu's up (and whether it paused time, to unpause it after).
#[derive(Resource, Default)]
pub struct GameMenu {
    pub open: bool,
    paused: bool,
}

#[derive(Component, Clone, Copy, PartialEq)]
enum Choice {
    Resume,
    Save,
    MainMenu,
    Quit,
}

#[derive(Component)]
struct Root;

/// The volume slider (`settings.rs`): its ten steps (1..=10, a tenth
/// each), the step down and up, the reading.
#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) enum Volume {
    Step(u8),
    Less,
    More,
}

#[derive(Component)]
struct VolumeReading;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameMenu>()
            .add_systems(Startup, spawn)
            // (First of whatever hears Esc: it's the menu's only when
            // nothing else is open to shut.)
            .add_systems(PreUpdate, key.after(bevy::input::InputSystems).before(crate::editor::capture).before(crate::bestiary::panel::capture).before(crate::title::capture))
            .add_systems(PreUpdate, (choose, slide).after(bevy::ui::UiSystems::Focus))
            .add_systems(Update, (look, show_volume));
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn key(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut menu: ResMut<GameMenu>,
    mut taken: ResMut<KeyboardTaken>,
    mut virt: ResMut<Time<Virtual>>,
    mut inventory: ResMut<crate::hands::ui::InventoryOpen>,
    mut crafting: ResMut<crate::hands::crafting::CraftingPanel>,
    dev: Res<crate::devmenu::DevMenu>,
    bestiary: Res<crate::bestiary::panel::Bestiary>,
    editor: Res<crate::editor::Editor>,
    title: Res<crate::title::Title>,
) {
    let esc = keys.just_pressed(KeyCode::Escape);
    if menu.open {
        if esc {
            close(&mut menu, &mut virt);
        }
        taken.0 = true;
        keys.reset_all();
        return;
    }
    // (Theirs: they have the keyboard, and shut on Esc themselves.)
    if !esc || editor.open || bestiary.open || title.up || taken.0 {
        return;
    }
    if inventory.0 && crafting.open {
        crafting.open = false;
    } else if inventory.0 {
        inventory.0 = false;
    } else if dev.open {
        // (The dev menu's own: `devmenu.rs`.)
        return;
    } else {
        inventory.0 = true;
    }
    keys.reset(KeyCode::Escape);
}

/// Up, and time stopped (the inventory's Settings button).
pub fn open(menu: &mut GameMenu, virt: &mut Time<Virtual>) {
    menu.open = true;
    menu.paused = !virt.is_paused();
    virt.pause();
}

fn close(menu: &mut GameMenu, virt: &mut Time<Virtual>) {
    menu.open = false;
    if menu.paused {
        virt.unpause();
    }
}

fn choose(
    clicks: Query<(&Interaction, &Choice), Changed<Interaction>>,
    mut menu: ResMut<GameMenu>,
    mut virt: ResMut<Time<Virtual>>,
    mut saves: MessageWriter<crate::save::SaveNow>,
    mut exit: MessageWriter<AppExit>,
) {
    for (i, c) in &clicks {
        if *i != Interaction::Pressed || !menu.open {
            continue;
        }
        match c {
            Choice::Resume => close(&mut menu, &mut virt),
            Choice::Save => {
                saves.write(crate::save::SaveNow);
            }
            // (Leaving, the save's written before the game goes; then the
            // game starts again at the title, over it.)
            // (An unsaved world, the arena: the title over the last save.)
            Choice::MainMenu => {
                crate::title::relaunch_to_title();
                exit.write(AppExit::Success);
            }
            Choice::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

const IDLE: Color = Color::srgba(0.18, 0.17, 0.22, 0.9);
const HOVER: Color = Color::srgba(0.42, 0.36, 0.2, 0.95);
/// A volume step on, off.
const ON: Color = Color::srgb(0.98, 0.86, 0.55);
const OFF: Color = Color::srgba(0.25, 0.24, 0.3, 0.9);

/// The volume slider: a step clicked (or dragged over with the button
/// down) sets it there; − and + a tenth down and up.
pub(crate) fn slide(menu: Res<GameMenu>, mouse: Res<ButtonInput<MouseButton>>, q: Query<(Ref<Interaction>, &Volume)>, mut settings: ResMut<crate::settings::Settings>) {
    if !menu.open {
        return;
    }
    for (i, v) in &q {
        let pressed = *i == Interaction::Pressed;
        // (− and +: once a press, as it goes down.)
        let pressing = pressed && i.is_changed();
        let want = match *v {
            Volume::Step(n) if pressed || (*i == Interaction::Hovered && mouse.pressed(MouseButton::Left)) => n,
            Volume::Less if pressing => settings.volume.saturating_sub(1),
            Volume::More if pressing => (settings.volume + 1).min(10),
            _ => continue,
        };
        if settings.volume != want {
            settings.volume = want;
        }
    }
}

/// The slider showing the volume: its steps lit up to it, the reading.
fn show_volume(settings: Res<crate::settings::Settings>, mut steps: Query<(&Volume, &Interaction, &mut BackgroundColor)>, mut reading: Query<&mut Text, With<VolumeReading>>) {
    for (v, i, mut bg) in &mut steps {
        let want = match *v {
            Volume::Step(n) if n <= settings.volume => ON,
            Volume::Step(_) => OFF,
            _ if *i == Interaction::None => IDLE,
            _ => HOVER,
        };
        if bg.0 != want {
            bg.0 = want;
        }
    }
    for mut t in &mut reading {
        let want = format!("{}%", settings.volume as u32 * 10);
        if t.0 != want {
            t.0 = want;
        }
    }
}

fn spawn(mut commands: Commands, info: Res<crate::title::GameInfo>, slot: Option<Res<crate::save::SaveSlot>>) {
    let saves = slot.is_some();
    commands
        .spawn((
            Root,
            Visibility::Hidden,
            GlobalZIndex(55),
            Interaction::default(),
            Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.02, 0.6)),
        ))
        .with_children(|p| {
            p.spawn((Node { flex_direction: FlexDirection::Column, row_gap: px(8), padding: UiRect::all(px(24)), align_items: AlignItems::Stretch, min_width: px(320), ..default() }, BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.92)))).with_children(|p| {
                p.spawn((Text::new(info.title.to_uppercase()), TextFont { font_size: FontSize::Px(34.0), ..default() }, TextColor(Color::srgb(0.98, 0.86, 0.55)), Node { margin: UiRect::bottom(px(12)), align_self: AlignSelf::Center, ..default() }));
                let mut choices = vec![("Resume   Esc", Choice::Resume)];
                if saves {
                    choices.push(("Save", Choice::Save));
                }
                choices.push((if saves { "Save and go to the main menu" } else { "Main menu" }, Choice::MainMenu));
                choices.push((if saves { "Save and quit" } else { "Quit" }, Choice::Quit));
                for (label, c) in choices {
                    p.spawn((Button, c, Node { padding: UiRect::axes(px(16), px(8)), justify_content: JustifyContent::Center, ..default() }, BackgroundColor(IDLE))).with_children(|b| {
                        b.spawn((Text::new(label), TextFont { font_size: FontSize::Px(20.0), ..default() }, TextColor(Color::srgb(0.93, 0.91, 0.86))));
                    });
                }
                // Volume: a step down, ten steps (click or drag along them),
                // a step up, the reading.
                p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(6), margin: UiRect::top(px(10)), ..default() }).with_children(|r| {
                    r.spawn((Text::new("Volume"), TextFont { font_size: FontSize::Px(18.0), ..default() }, TextColor(Color::srgb(0.93, 0.91, 0.86)), Node { width: px(72), ..default() }));
                    let small = |r: &mut ChildSpawnerCommands, label: &str, v: Volume| {
                        r.spawn((Button, v, Node { width: px(24), height: px(24), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, BackgroundColor(IDLE))).with_children(|b| {
                            b.spawn((Text::new(label), TextFont { font_size: FontSize::Px(18.0), ..default() }, TextColor(Color::srgb(0.93, 0.91, 0.86))));
                        });
                    };
                    small(r, "-", Volume::Less);
                    r.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(2), ..default() }).with_children(|bar| {
                        for i in 1..=10u8 {
                            bar.spawn((Button, Volume::Step(i), Node { width: px(12), height: px(20), ..default() }, BackgroundColor(OFF)));
                        }
                    });
                    small(r, "+", Volume::More);
                    r.spawn((VolumeReading, Text::new("100%"), TextFont { font_size: FontSize::Px(18.0), ..default() }, TextColor(Color::srgb(0.93, 0.91, 0.86)), Node { width: px(52), ..default() }));
                });
            });
        });
}

fn look(menu: Res<GameMenu>, mut root: Query<&mut Visibility, With<Root>>, mut buttons: Query<(&Interaction, &mut BackgroundColor), With<Choice>>) {
    for mut v in &mut root {
        let want = if menu.open { Visibility::Visible } else { Visibility::Hidden };
        if *v != want {
            *v = want;
        }
    }
    for (i, mut bg) in &mut buttons {
        let want = if *i == Interaction::None { IDLE } else { HOVER };
        if bg.0 != want {
            bg.0 = want;
        }
    }
}
