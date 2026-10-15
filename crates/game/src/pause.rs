//! The game menu: Esc, with nothing else open (the inventory, the dev menu,
//! the bestiary, the art editor shut first: Esc shuts them). Time stands
//! still while it's up. Resume (or Esc again) · Save · Main menu (saved,
//! then the title screen over this save: `title.rs`) · Quit (saved).

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

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameMenu>()
            .add_systems(Startup, spawn)
            // (First of whatever hears Esc: it's the menu's only when
            // nothing else is open to shut.)
            .add_systems(PreUpdate, key.after(bevy::input::InputSystems).before(crate::editor::capture).before(crate::bestiary::panel::capture).before(crate::title::capture))
            .add_systems(PreUpdate, choose.after(bevy::ui::UiSystems::Focus))
            .add_systems(Update, look);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn key(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut menu: ResMut<GameMenu>,
    mut taken: ResMut<KeyboardTaken>,
    mut virt: ResMut<Time<Virtual>>,
    inventory: Res<crate::hands::ui::InventoryOpen>,
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
    let busy = inventory.0 || dev.open || bestiary.open || editor.open || title.up;
    if esc && !busy {
        menu.open = true;
        menu.paused = !virt.is_paused();
        virt.pause();
        keys.reset_all();
    }
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
