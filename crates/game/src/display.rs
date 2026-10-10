//! What's drawn over the world besides the world: alert marks over
//! creatures and damage numbers, each an option (`assets/data/display.ron`,
//! read at the start; the dev panel switches them while playing).

use bevy::prelude::*;
use serde::Deserialize;

use crate::dev::DevAction;

pub struct DisplayPlugin;

impl Plugin for DisplayPlugin {
    fn build(&self, app: &mut App) {
        let display: Display = crate::data::load_ron(&crate::data::data_path("display.ron")).unwrap_or_else(|e| {
            warn!("{e}");
            Display::default()
        });
        app.insert_resource(display).add_systems(Update, switch);
    }
}

/// The options (both off by default).
#[derive(Resource, Deserialize, Default, Clone, Copy)]
#[serde(default)]
pub struct Display {
    /// "?" and "!" over creatures as they grow wary (`senses`).
    pub alert_marks: bool,
    /// The damage a hit does, floating up (`hurt`).
    pub damage_numbers: bool,
}

fn switch(mut actions: MessageReader<DevAction>, mut display: ResMut<Display>, mut toasts: MessageWriter<crate::progress::Toast>) {
    for a in actions.read() {
        let (name, on) = match a {
            DevAction::AlertMarks => {
                display.alert_marks = !display.alert_marks;
                ("Alert marks", display.alert_marks)
            }
            DevAction::DamageNumbers => {
                display.damage_numbers = !display.damage_numbers;
                ("Damage numbers", display.damage_numbers)
            }
            _ => continue,
        };
        toasts.write(crate::progress::Toast(format!("{name} {}", if on { "on" } else { "off" })));
    }
}
