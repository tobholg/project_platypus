//! The game's tempo (`assets/data/tempo.ron`): presets of how fast and how
//! heavy movement is, to try side by side (the dev panel's Tempo row, or
//! `PLATYPUS_TEMPO=<name>`). A preset sets the player's movement fields it
//! names (over its creature file's; its run speed and jump height stay
//! scaled by what it wears), and paces every other creature: speeds × pace,
//! accelerations and gravity × pace², times ÷ pace (the same arcs, slower).
//!
//! Applied where movement is read (`Tempo::apply`), so a switch is live.

use std::collections::BTreeMap;

use bevy::prelude::*;
use platypus_physics::MovementStats;
use serde::Deserialize;

use crate::data::{Watched, data_path, load_ron};

pub struct TempoPlugin;

impl Plugin for TempoPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Tempo::load()).add_systems(Update, reload);
    }
}

#[derive(Deserialize, Clone, Debug)]
struct TempoFile {
    default: String,
    presets: Vec<Preset>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Preset {
    pub name: String,
    /// Every other creature's pace (1: as its file says).
    #[serde(default = "one")]
    pub pace: f32,
    /// The player's movement fields, by name.
    #[serde(default)]
    pub player: BTreeMap<String, f32>,
}

fn one() -> f32 {
    1.0
}

#[derive(Resource)]
pub struct Tempo {
    pub presets: Vec<Preset>,
    pub active: usize,
    /// The player's own run speed and jump height (its file's), so what it
    /// wears still scales them.
    base: (f32, f32),
    watch: Watched,
}

impl Tempo {
    fn load() -> Self {
        let path = data_path("tempo.ron");
        let file: TempoFile = load_ron(&path).unwrap_or_else(|e| {
            warn!("tempo.ron: {e}");
            TempoFile { default: String::new(), presets: Vec::new() }
        });
        let pick = std::env::var("PLATYPUS_TEMPO").unwrap_or(file.default.clone());
        let active = file.presets.iter().position(|p| p.name.eq_ignore_ascii_case(&pick)).unwrap_or(0);
        let base = load_ron::<crate::actors::creature::CreatureDef>(&data_path("creatures/player.ron")).map_or((95.0, 40.0), |d| (d.movement.run_speed, d.movement.jump_height));
        Tempo { presets: file.presets, active, base, watch: Watched::new(path) }
    }

    pub fn name(&self) -> &str {
        self.presets.get(self.active).map_or("", |p| &p.name)
    }

    /// A creature's movement at this tempo (`player`: it's the player's).
    pub fn apply(&self, s: &MovementStats, player: bool) -> MovementStats {
        let mut m = s.clone();
        let Some(p) = self.presets.get(self.active) else { return m };
        if player {
            for (field, &v) in &p.player {
                match field.as_str() {
                    "run_speed" => m.run_speed = v * s.run_speed / self.base.0.max(1.0),
                    "jump_height" => m.jump_height = v * s.jump_height / self.base.1.max(1.0),
                    "ground_accel" => m.ground_accel = v,
                    "ground_decel" => m.ground_decel = v,
                    "turn_accel" => m.turn_accel = v,
                    "air_accel" => m.air_accel = v,
                    "gravity" => m.gravity = v,
                    "fall_gravity" => m.fall_gravity = v,
                    "max_fall" => m.max_fall = v,
                    "jump_hold" => m.jump_hold = v,
                    "jump_cut" => m.jump_cut = v,
                    "wall_slide_speed" => m.wall_slide_speed = v,
                    "wall_jump_push" => m.wall_jump_push = v,
                    "dash_speed" => m.dash_speed = v,
                    "dash_time" => m.dash_time = v,
                    _ => {}
                }
            }
            return m;
        }
        let k = p.pace;
        if (k - 1.0).abs() < 1e-4 || k <= 0.0 {
            return m;
        }
        for v in [&mut m.run_speed, &mut m.max_fall, &mut m.dash_speed, &mut m.fly_speed, &mut m.swim_speed, &mut m.wall_slide_speed, &mut m.wall_jump_push, &mut m.rocket_speed] {
            *v *= k;
        }
        for v in [&mut m.ground_accel, &mut m.ground_decel, &mut m.turn_accel, &mut m.air_accel, &mut m.gravity, &mut m.fly_accel, &mut m.swim_accel, &mut m.rocket_thrust] {
            *v *= k * k;
        }
        for v in [&mut m.dash_time, &mut m.jump_hold] {
            *v /= k;
        }
        m
    }
}

/// tempo.ron edited: the presets again (the pick kept by name).
fn reload(mut tempo: ResMut<Tempo>) {
    if !tempo.bypass_change_detection().watch.changed() {
        return;
    }
    let name = tempo.name().to_string();
    let mut fresh = Tempo::load();
    if let Some(i) = fresh.presets.iter().position(|p| p.name == name) {
        fresh.active = i;
    }
    *tempo = fresh;
    info!("tempo.ron reloaded: {}", tempo.name());
}
