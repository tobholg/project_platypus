//! Dev tools: pickaxe, bomb, material spawner, igniter, eraser.
//! Tunables in `assets/data/tools.ron` (hot-reloaded).
//!
//! 1–6 pick a tool · LMB use · RMB erase · Q/E change spawner material ·
//! `[` `]` or Ctrl+wheel radius · Shift+LMB: spawner replaces solids, heat gun freezes.
//!
//! Input is sampled every frame but tools act on the fixed tick, so a pickaxe
//! digs at the same speed at 60 or 240 fps. Every change is a `WorldEdit`.

use std::collections::BTreeMap;

use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::math::Isometry2d;
use bevy::prelude::*;
use platypus_sim::{CellPos, EditReport, Kind, MaterialId, WorldEdit};
use serde::Deserialize;

use crate::actors::Kinematics;
use crate::actors::player::LocalPlayer;
use crate::camera::CursorWorld;
use crate::data::{Watched, data_path, load_ron};
use crate::props::spawn_bomb;
use crate::world::{SimWorld, TickSet};

pub struct ToolsPlugin;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Pickaxe,
    Bomb,
    Spawner,
    Igniter,
    Eraser,
    Heat,
}

impl Tool {
    pub const ALL: [Tool; 6] = [Tool::Pickaxe, Tool::Bomb, Tool::Spawner, Tool::Igniter, Tool::Eraser, Tool::Heat];
    const KEYS: [KeyCode; 6] =
        [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6];

    fn index(self) -> usize {
        Tool::ALL.iter().position(|t| *t == self).unwrap()
    }

    pub fn label(self) -> &'static str {
        match self {
            Tool::Pickaxe => "Pickaxe",
            Tool::Bomb => "Bomb",
            Tool::Spawner => "Spawn",
            Tool::Igniter => "Ignite",
            Tool::Eraser => "Erase",
            Tool::Heat => "Heat/Freeze",
        }
    }

    fn color(self) -> Color {
        match self {
            Tool::Pickaxe => Color::srgb(1.0, 0.85, 0.3),
            Tool::Bomb => Color::srgb(1.0, 0.3, 0.2),
            Tool::Spawner => Color::srgb(0.4, 0.8, 1.0),
            Tool::Igniter => Color::srgb(1.0, 0.55, 0.1),
            Tool::Eraser => Color::srgb(0.9, 0.9, 0.9),
            Tool::Heat => Color::srgb(1.0, 0.2, 0.6),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct PickaxeCfg {
    pub radius: i32,
    pub power: u8,
    pub max_hardness: u8,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BombCfg {
    pub radius: i32,
    pub power: u8,
    pub fuse: f32,
    pub throw_speed: f32,
    pub bounce: f32,
    pub damage: f32,
    pub knockback: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RadiusCfg {
    pub radius: i32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HeatCfg {
    pub radius: i32,
    /// °C added per tick at the centre.
    pub rate: i16,
    /// °C removed per tick with Shift.
    pub cool_rate: i16,
}

#[derive(Resource, Clone, Debug, Deserialize)]
pub struct ToolsConfig {
    pub pickaxe: PickaxeCfg,
    pub bomb: BombCfg,
    pub spawner: RadiusCfg,
    pub igniter: RadiusCfg,
    pub eraser: RadiusCfg,
    pub heat: HeatCfg,
}

impl ToolsConfig {
    fn radius(&self, t: Tool) -> i32 {
        match t {
            Tool::Pickaxe => self.pickaxe.radius,
            Tool::Bomb => self.bomb.radius,
            Tool::Spawner => self.spawner.radius,
            Tool::Igniter => self.igniter.radius,
            Tool::Eraser => self.eraser.radius,
            Tool::Heat => self.heat.radius,
        }
    }
}

/// What the local player is holding.
#[derive(Resource)]
pub struct Toolbelt {
    pub tool: Tool,
    /// Per-tool radius, starts from the config and changes with `[` `]`.
    pub radius: [i32; 6],
    /// Spawner material.
    pub material: MaterialId,
    /// Everything dug out so far, by material name (future inventory).
    pub mined: BTreeMap<String, u32>,
}

impl Toolbelt {
    pub fn radius(&self) -> i32 {
        self.radius[self.tool.index()]
    }
}

/// Mouse state, sampled per frame, consumed per tick.
#[derive(Resource, Default)]
struct ToolInput {
    primary: bool,
    secondary: bool,
    /// A primary click happened since the last tick (for one-shot tools).
    clicked: bool,
    overwrite: bool,
    cursor: Option<Vec2>,
}

#[derive(Resource)]
struct ConfigWatch(Watched);

#[derive(Component)]
struct Hotbar;

impl Plugin for ToolsPlugin {
    fn build(&self, app: &mut App) {
        let path = data_path("tools.ron");
        let cfg: ToolsConfig = load_ron(&path).unwrap_or_else(|e| panic!("{e}"));
        let radius = Tool::ALL.map(|t| cfg.radius(t));
        app.insert_resource(cfg)
            .insert_resource(ConfigWatch(Watched::new(path)))
            .insert_resource(Toolbelt { tool: Tool::Pickaxe, radius, material: MaterialId::AIR, mined: BTreeMap::new() })
            .init_resource::<ToolInput>()
            .add_systems(Startup, (spawn_hotbar, default_material))
            .add_systems(PreUpdate, sample_input.after(crate::camera::track_cursor))
            .add_systems(Update, (select, reload_config, preview, update_hotbar))
            .add_systems(FixedUpdate, use_tools.in_set(TickSet::Intent));
    }
}

fn default_material(sim: Res<SimWorld>, mut belt: ResMut<Toolbelt>) {
    belt.material = sim.materials().id("sand").unwrap_or(MaterialId(1));
}

fn sample_input(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Res<CursorWorld>,
    mut input: ResMut<ToolInput>,
) {
    input.primary = mouse.pressed(MouseButton::Left);
    input.secondary = mouse.pressed(MouseButton::Right);
    input.clicked |= mouse.just_pressed(MouseButton::Left);
    input.overwrite = keys.pressed(KeyCode::ShiftLeft);
    input.cursor = cursor.0;
}

fn select(
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<AccumulatedMouseScroll>,
    sim: Res<SimWorld>,
    mut belt: ResMut<Toolbelt>,
) {
    for (tool, key) in Tool::ALL.iter().zip(Tool::KEYS) {
        if keys.just_pressed(key) {
            belt.tool = *tool;
        }
    }
    // Cycle through every non-air material in materials.ron.
    let step = keys.just_pressed(KeyCode::KeyE) as i32 - keys.just_pressed(KeyCode::KeyQ) as i32;
    if step != 0 {
        let n = sim.materials().len() as i32 - 1;
        let cur = belt.material.0 as i32 - 1;
        belt.material = MaterialId(((cur + step).rem_euclid(n) + 1) as u16);
        belt.tool = Tool::Spawner;
    }
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let mut d = keys.just_pressed(KeyCode::BracketRight) as i32 - keys.just_pressed(KeyCode::BracketLeft) as i32;
    if ctrl && scroll.delta.y != 0.0 {
        d = scroll.delta.y.signum() as i32;
    }
    if d != 0 {
        let i = belt.tool.index();
        let r = belt.radius[i];
        belt.radius[i] = (r + d * (1 + r / 8)).clamp(0, 48);
    }
}

fn reload_config(mut watch: ResMut<ConfigWatch>, mut cfg: ResMut<ToolsConfig>, mut belt: ResMut<Toolbelt>) {
    if !watch.0.changed() {
        return;
    }
    match load_ron::<ToolsConfig>(watch.0.path()) {
        Ok(new) => {
            belt.radius = Tool::ALL.map(|t| new.radius(t));
            *cfg = new;
            info!("tools reloaded");
        }
        Err(e) => warn!("tools not reloaded: {e}"),
    }
}

fn use_tools(
    mut commands: Commands,
    mut input: ResMut<ToolInput>,
    cfg: Res<ToolsConfig>,
    mut belt: ResMut<Toolbelt>,
    mut sim: ResMut<SimWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
) {
    let clicked = std::mem::take(&mut input.clicked);
    let Some(at) = input.cursor else { return };
    let center = CellPos::from_world(at.x, at.y);
    let radius = belt.radius();

    let edit = if input.secondary {
        Some(WorldEdit::Dig { center, radius: belt.radius[Tool::Eraser.index()], max_hardness: 254 })
    } else {
        match belt.tool {
            Tool::Pickaxe if input.primary => {
                Some(WorldEdit::Mine { center, radius, power: cfg.pickaxe.power, max_hardness: cfg.pickaxe.max_hardness })
            }
            Tool::Spawner if input.primary => {
                Some(WorldEdit::Paint { center, radius, material: belt.material, overwrite: input.overwrite })
            }
            Tool::Igniter if input.primary => Some(WorldEdit::Ignite { center, radius }),
            Tool::Eraser if input.primary => Some(WorldEdit::Dig { center, radius, max_hardness: 254 }),
            Tool::Heat if input.primary => {
                let amount = if input.overwrite { -cfg.heat.cool_rate } else { cfg.heat.rate };
                Some(WorldEdit::Heat { center, radius, amount })
            }
            Tool::Bomb if clicked => {
                // Thrown from the player toward the cursor; dropped at the cursor in free-camera mode.
                let (from, vel) = match player.single() {
                    Ok(k) => {
                        let from = k.body.pos + Vec2::new(0.0, k.body.half.y * 0.5);
                        let dir = (at - from).normalize_or(Vec2::X);
                        let speed = cfg.bomb.throw_speed * ((at - from).length() / 120.0).clamp(0.35, 1.0);
                        (from, dir * speed + k.body.vel * 0.5)
                    }
                    Err(_) => (at, Vec2::ZERO),
                };
                spawn_bomb(&mut commands, from, vel, cfg.bomb.clone());
                None
            }
            _ => None,
        }
    };
    if let Some(edit) = edit {
        let report = sim.world.apply_edit(&edit);
        record_yield(&mut belt, &sim, &report);
    }
}

fn record_yield(belt: &mut Toolbelt, sim: &SimWorld, report: &EditReport) {
    for &(id, n) in &report.removed {
        let def = sim.materials().def(id);
        if matches!(def.kind, Kind::Static | Kind::Powder) {
            *belt.mined.entry(def.name.clone()).or_default() += n;
        }
    }
}

fn preview(input: Res<ToolInput>, belt: Res<Toolbelt>, mut gizmos: Gizmos) {
    let Some(at) = input.cursor else { return };
    let r = belt.radius() as f32 + 0.5;
    gizmos.circle_2d(Isometry2d::from_translation(at), r.max(1.0), belt.tool.color().with_alpha(0.8));
}

fn spawn_hotbar(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont { font_size: FontSize::Px(14.0), ..default() },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(8),
            left: px(8),
            padding: UiRect::all(px(6)),
            ..default()
        },
        Hotbar,
    ));
}

fn update_hotbar(belt: Res<Toolbelt>, sim: Res<SimWorld>, mut text: Single<&mut Text, With<Hotbar>>) {
    if !belt.is_changed() && !text.0.is_empty() {
        return;
    }
    let slots: Vec<String> = Tool::ALL
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let label = if *t == Tool::Spawner {
                format!("{}: {}", t.label(), sim.materials().def(belt.material).name)
            } else {
                t.label().to_string()
            };
            if *t == belt.tool { format!("[{} {label}]", i + 1) } else { format!(" {} {label} ", i + 1) }
        })
        .collect();
    let mined: Vec<String> = belt.mined.iter().map(|(k, v)| format!("{k} {v}")).collect();
    text.0 = format!(
        "{}   radius {}\nLMB use | RMB erase | Q/E material | Shift: overwrite / freeze | [ ] radius | Tab free camera\nmined: {}",
        slots.join(" "),
        belt.radius(),
        if mined.is_empty() { "-".into() } else { mined.join(", ") }
    );
}

/// Material and temperature under a world position, for the HUD.
pub fn material_at(sim: &SimWorld, at: Vec2) -> Option<(MaterialId, String)> {
    let p = CellPos::from_world(at.x, at.y);
    let cell = sim.world.get(p)?;
    let t = sim.world.temperature(p)?;
    Some((cell.material, format!("{} {t}C", sim.materials().def(cell.material).name)))
}
