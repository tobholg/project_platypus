//! The show: the bestiary's creatures brought on to show someone, in the
//! world as it is.
//!
//! - "Show the next" (P, or the dev panel): the next act of
//!   `assets/data/show.ron`, on the side of you the mouse is on, each on the
//!   ground there (a pack as one; an act's `against` a way past it, so two
//!   factions fall on each other), with a toast saying what to watch.
//! - Show mode (the dev panel, or `PLATYPUS_SHOW=1` from the start): a
//!   lively world. Life comes ten times as often, the big ones (strider,
//!   tyrant, raptors) anywhere, not just far from the start or in their
//!   biomes, and what lives deep in caves a quarter as deep (`critters.rs`).

use bevy::prelude::*;
use serde::Deserialize;

use crate::creatures::Kinematics;
use crate::creatures::player::LocalPlayer;
use crate::dev::DevAction;
use crate::world::SimWorld;

pub struct ShowPlugin;

impl Plugin for ShowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShowMode>().init_resource::<NextAct>().add_systems(Update, show);
    }
}

/// Show mode: the world lively (`critters.rs` reads it).
#[derive(Resource)]
pub struct ShowMode(pub bool);

impl Default for ShowMode {
    fn default() -> Self {
        ShowMode(std::env::var("PLATYPUS_SHOW").is_ok_and(|v| v != "0"))
    }
}

/// In show mode, life comes this many times as often.
pub const SHOW_RATE: f32 = 10.0;

/// One act of the show.
#[derive(Deserialize, Clone, Debug)]
struct Act {
    name: String,
    note: String,
    spawn: Vec<(String, u32)>,
    #[serde(default)]
    against: Vec<(String, u32)>,
    /// How far off the first of it stands (cells).
    #[serde(default = "default_at")]
    at: f32,
}

fn default_at() -> f32 {
    150.0
}

/// The act "Show the next" brings on.
#[derive(Resource, Default)]
struct NextAct(usize);

/// An act's members this far apart in line (cells); its `against` this much
/// further past them.
const GAP: f32 = 26.0;
const AGAINST: f32 = 130.0;
/// Where an act starts: within this of its `at`, never nearer than
/// `MIN_OFF`, on ground within `LEVEL` of the player's feet (cells).
const SHIFT: f32 = 120.0;
const MIN_OFF: f32 = 80.0;
const LEVEL: f32 = 30.0;

#[allow(clippy::too_many_arguments)]
fn show(
    mut commands: Commands,
    mut actions: MessageReader<DevAction>,
    mut mode: ResMut<ShowMode>,
    mut next: ResMut<NextAct>,
    sim: Res<SimWorld>,
    cursor: Res<crate::camera::CursorWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut toasts: MessageWriter<crate::progress::Toast>,
) {
    for a in actions.read() {
        match a {
            DevAction::ShowMode => {
                mode.0 = !mode.0;
                toasts.write(crate::progress::Toast(if mode.0 { "Show mode: the world comes alive (the big ones too)".into() } else { "Show mode off".into() }));
            }
            DevAction::ShowNext => {
                let Ok(pk) = player.single() else { continue };
                let acts: Vec<Act> = crate::data::load_ron(&crate::data::data_path("show.ron")).unwrap_or_else(|e| {
                    warn!("{e}");
                    Vec::new()
                });
                let Some(act) = acts.get(next.0 % acts.len().max(1)).cloned() else { continue };
                next.0 = (next.0 + 1) % acts.len();
                // (The side the mouse is on; a button's press, the mouse on
                // the panel at the right: there.)
                let p = pk.body.pos;
                let dir = cursor.0.map_or(1.0, |c| if c.x < p.x { -1.0 } else { 1.0 });
                let past = act.at + act.spawn.iter().map(|(_, n)| *n).sum::<u32>() as f32 * GAP + AGAINST;
                put(&mut commands, &sim, p, dir, act.at, &act.spawn, sim.world.tick());
                put(&mut commands, &sim, p, dir, past, &act.against, sim.world.tick() ^ 0xA6A1);
                toasts.write(crate::progress::Toast(act.name.clone()));
                toasts.write(crate::progress::Toast(act.note.clone()));
                info!("show: {} ({} of {})", act.name, (next.0 + acts.len() - 1) % acts.len() + 1, acts.len());
            }
            _ => {}
        }
    }
}

/// `who` in a line from `from` cells off on side `dir`, each on the ground
/// there (scanning down from a little over the player: under a cave's
/// roof, its floor), together as a pack. The line starts where the ground
/// is near the player's height and in plain sight of it, if that's
/// anywhere within `SHIFT` of `from` (put on a hilltop over the player,
/// the iron strider couldn't see it past the crest, and only wondered).
fn put(commands: &mut Commands, sim: &SimWorld, p: Vec2, dir: f32, from: f32, who: &[(String, u32)], id: u64) {
    let members: Vec<&String> = who.iter().flat_map(|(k, n)| std::iter::repeat_n(k, *n as usize)).collect();
    let pack = members.len() > 1;
    let ground = |x: i32| [40, 120, 240].into_iter().find_map(|up| crate::creatures::spawn::find_ground(&sim.world, x, p.y as i32 + up, 400));
    let feet = p.y - 12.0;
    let fair = |off: f32| {
        let x = (p.x + dir * off) as i32;
        ground(x).is_some_and(|y| (y as f32 - feet).abs() < LEVEL && crate::creatures::moves::clear(sim, p, Vec2::new(x as f32 + 0.5, y as f32 + 12.0)))
    };
    let from = (0..=SHIFT as i32 / 8).flat_map(|i| [from + i as f32 * 8.0, from - i as f32 * 8.0]).filter(|o| *o >= MIN_OFF).find(|o| fair(*o)).unwrap_or(from);
    for (i, kind) in members.into_iter().enumerate() {
        let x = (p.x + dir * (from + i as f32 * GAP)) as i32;
        let Some(y) = ground(x) else {
            warn!("show: no ground for a {kind} at x {x}");
            continue;
        };
        let rank = i as u32;
        // (Facing the player: what it hunts behind it, it barely sees,
        // and put down facing away it wandered off.)
        crate::creatures::def::spawn_creature(commands, kind, Vec2::new(x as f32 + 0.5, y as f32), move |e| {
            if let Some(mut k) = e.get_mut::<Kinematics>() {
                k.loco.facing = -dir;
            }
            if pack {
                e.insert(crate::creatures::brain::tactics::Pack { id, rank });
            }
        });
    }
}
