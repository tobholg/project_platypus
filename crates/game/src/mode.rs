//! Play or dev (PLAN.md, the prologue arc: `play-mode`).
//!
//! - **Play**: a new game is a new player with the `play` kit (a copper pick
//!   and axe, a short sword, a few torches: `items.ron`), and none of the
//!   dev tools: no dev menu, no toolbelt, no dev keys, no bestiary.
//! - **Dev**: everything (the `dev` kit, the dev menu on the key left of 1
//!   or F1: `devmenu.rs`).
//!
//! `PLATYPUS_MODE=play|dev` picks; otherwise a save remembers its own
//! (`save.rs`), and anything else (the arena, scenarios, a world named by
//! `PLATYPUS_WORLD`) is dev.

use bevy::prelude::*;

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    Play,
    #[default]
    Dev,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Mode> {
        match s.trim() {
            "play" => Some(Mode::Play),
            "dev" => Some(Mode::Dev),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Play => "play",
            Mode::Dev => "dev",
        }
    }

    /// `PLATYPUS_MODE`, if it names one.
    pub fn from_env() -> Option<Mode> {
        std::env::var("PLATYPUS_MODE").ok().and_then(|v| Mode::parse(&v))
    }

    pub fn dev(self) -> bool {
        self == Mode::Dev
    }

    /// The kit a new player starts with (`items.ron`).
    pub fn kit(self) -> &'static str {
        self.name()
    }
}
