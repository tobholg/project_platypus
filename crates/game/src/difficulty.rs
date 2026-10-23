//! How hard the world hits you: three ways to play, each save its own.
//!
//! - **Pilgrim**: gentler (to explore and build at your own pace); and only
//!   here do you heal by yourself (`craft::rest`).
//! - **Wayfarer** (the default): you start weak and afraid: a skeleton's
//!   five blows kill you unarmoured, eight in leather, about twelve in iron.
//! - **Hardened**: every blow lands hard.
//!
//! Only what the player takes changes (`Ward::taken`); creatures hit as
//! hard and are as tough. `PLATYPUS_DIFFICULTY` picks; else the save's;
//! else Wayfarer. A new game asks (`title.rs`), the dev menu changes it.

use bevy::prelude::*;

use crate::creatures::Health;
use crate::creatures::player::LocalPlayer;

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Difficulty {
    Pilgrim,
    #[default]
    Wayfarer,
    Hardened,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Pilgrim, Difficulty::Wayfarer, Difficulty::Hardened];

    pub fn parse(s: &str) -> Option<Difficulty> {
        match s.trim().to_lowercase().as_str() {
            "pilgrim" => Some(Difficulty::Pilgrim),
            "wayfarer" => Some(Difficulty::Wayfarer),
            "hardened" => Some(Difficulty::Hardened),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Pilgrim => "Pilgrim",
            Difficulty::Wayfarer => "Wayfarer",
            Difficulty::Hardened => "Hardened",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            Difficulty::Pilgrim => "Gentler blows, and you heal by yourself",
            Difficulty::Wayfarer => "The way it's meant to be played",
            Difficulty::Hardened => "Every blow lands hard",
        }
    }

    /// How much of what goes on hurting the player it takes (burning, acid
    /// on it, poison, heat): as it always was; blows are `taken`'s.
    pub fn lingering(self) -> f32 {
        match self {
            Difficulty::Pilgrim => 0.35,
            Difficulty::Wayfarer => 0.6,
            Difficulty::Hardened => 1.0,
        }
    }

    /// How much of what hits the player it takes (times what the creature
    /// files say: tuned so a skeleton's blows kill an unarmoured Wayfarer in
    /// five, measured with the `gauntlet` scenario).
    pub fn taken(self) -> f32 {
        match self {
            Difficulty::Pilgrim => 1.5,
            Difficulty::Wayfarer => 2.7,
            Difficulty::Hardened => 3.6,
        }
    }

    pub fn from_env() -> Option<Difficulty> {
        std::env::var("PLATYPUS_DIFFICULTY").ok().and_then(|v| Difficulty::parse(&v))
    }
}

pub struct DifficultyPlugin;

impl Plugin for DifficultyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Difficulty>().add_systems(Update, soften.after(crate::gear::apply));
    }
}

/// The player takes its share of what hits it (its gear's ward is made
/// again when what it wears changes: this goes on top, every frame).
fn soften(difficulty: Res<Difficulty>, mut player: Query<&mut Health, With<LocalPlayer>>) {
    let (taken, lingers) = (difficulty.taken(), difficulty.lingering() / difficulty.taken());
    for mut h in &mut player {
        if h.ward.taken != taken || h.ward.lingers != lingers {
            h.ward.taken = taken;
            h.ward.lingers = lingers;
        }
    }
}
