//! Tactics as data (BE `behaviour` stage 3): how a hunter fights beyond
//! going at you, set in its brain's `tactics`:
//!
//! - `flee`: badly hurt (under this share of its health) it runs from what
//!   it was fighting and attacks no more, unless it's cornered (a wall at
//!   its back, you near): then it fights.
//! - `ambush`: a crawler with no one about goes up to a ceiling and waits
//!   there, still; what comes near under it, it goes along the ceiling to
//!   and drops on.
//! - `shun_light`: it won't come within this of a quarry in the light (a
//!   torch, a lamp, day): it waits at the light's edge.
//! - `call`: when it starts hunting, others of its kind within this
//!   (cells) hunt too, coming to where it saw you (a pack's own always
//!   hear their own).
//! - `flank`: in a pack, half go round you to your far side.
//! - `skirmish`: it keeps `back` off you on its side, and comes in for a
//!   move (a bite) and out again, waiting `wait` s (±40 %) between; of a
//!   pack, only `together` at a time come in.
//!
//! A pack (`Pack`): what spawned together as a group (`life.ron`'s
//! `group`). With no one about, the rest follow the leader (its first);
//! one that starts hunting calls the rest, whatever its `call`.

use bevy::prelude::*;
use serde::Deserialize;

/// A hunter's tactics (its brain's `tactics`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct TacticsDef {
    pub flee: f32,
    pub ambush: bool,
    pub shun_light: f32,
    pub call: f32,
    pub flank: bool,
    pub skirmish: Option<SkirmishDef>,
}

/// Coming in and out (`skirmish`): how far off it waits, how long between
/// goes, how many of a pack come in at once.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct SkirmishDef {
    pub back: f32,
    pub wait: f32,
    pub together: u32,
}

impl Default for SkirmishDef {
    fn default() -> Self {
        SkirmishDef { back: 50.0, wait: 1.5, together: 1 }
    }
}

/// One of a pack: its pack, and its place in it (0: the leader).
#[derive(Component, Clone, Copy, Debug)]
pub struct Pack {
    pub id: u64,
    pub rank: u32,
}

/// How far a pack's own hear one of them call (cells).
pub const PACK_CALL: f32 = 600.0;

/// With no one about, a follower keeps within this of its leader (cells).
pub const FOLLOW: f32 = 60.0;
