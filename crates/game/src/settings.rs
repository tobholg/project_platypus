//! The player's own settings, not a world's: kept across sessions and
//! saves, in `saves/settings.ron`. Changed in the game menu (Esc, then
//! Settings: `pause.rs`).
//!
//! - `volume`: all the game's sound, in tenths (0 silent .. 10 full),
//!   over the mix sounds.ron sets.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub volume: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { volume: 10 }
    }
}

impl Settings {
    /// All the game's sound, 0..1.
    pub fn volume(&self) -> f32 {
        self.volume.min(10) as f32 / 10.0
    }
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(load()).add_systems(Update, keep.run_if(resource_changed::<Settings>));
    }
}

fn path() -> std::path::PathBuf {
    crate::save::saves_dir().join("settings.ron")
}

fn load() -> Settings {
    std::fs::read_to_string(path()).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default()
}

/// Written whenever it changes.
fn keep(settings: Res<Settings>, mut last: Local<Option<Settings>>) {
    if last.as_ref() == Some(&*settings) {
        return;
    }
    *last = Some(settings.clone());
    let text = ron::ser::to_string_pretty(&*settings, ron::ser::PrettyConfig::default()).unwrap_or_default();
    let _ = std::fs::create_dir_all(crate::save::saves_dir());
    if let Err(e) = std::fs::write(path(), text) {
        warn!("settings: {e}");
    }
}
