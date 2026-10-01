//! Villagers (DESIGN §13 item 6): the village's people, as data. Each is a
//! creature file (its look, on the player's rig: art `base: "player"`, its
//! team `Villager`) with the `villager` brain, whose params say who it is:
//! its `role` (guide, smith, healer, merchant), what it says (`lines`), and
//! what it does for you (`sells`, `buys`, `heals`: `talk.rs`). New people
//! are new files.
//!
//! Its day: it lives where it was first put (its home, `Home`). By day it
//! potters about within `wander` of it; from dusk to dawn it goes home and
//! stays there. A monster near (`flee_range`) and it runs home and hides
//! there (`Hiding`: monsters let it be) till the danger's gone. A player
//! near (`TALK_NEAR`) and it stops and turns to them.

use bevy::prelude::*;
use platypus_sim::rng::Rng;
use serde::Deserialize;

use crate::creatures::brain::RegisterBrain;
use crate::creatures::player::LocalPlayer;
use crate::creatures::{Controls, Health, Kinematics, Team};
use crate::world::SimWorld;

pub struct VillagerPlugin;

impl Plugin for VillagerPlugin {
    fn build(&self, app: &mut App) {
        app.register_brain::<Villager>("villager").add_systems(FixedUpdate, live.in_set(super::BrainSet));
    }
}

/// A player this near (cells) and a villager stops to talk.
pub const TALK_NEAR: f32 = 54.0;
/// Home from this hour to that (it's night).
const HOME_FROM: f32 = 20.5;
const HOME_UNTIL: f32 = 6.5;

/// Who a villager is and what it does (its creature file's brain params).
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Villager {
    /// guide, smith, healer, merchant (what the panel offers follows).
    pub role: String,
    /// What it says, in turn, when you're near.
    pub lines: Vec<String>,
    /// How far from home it potters by day (cells), how fast (a share of
    /// its run), and how near a monster sends it running.
    pub wander: f32,
    pub wander_speed: f32,
    pub flee_range: f32,
    /// What it sells: (item id, gold each).
    pub sells: Vec<(String, u32)>,
    /// Buys what you carry (half what it'd sell it for; a gold each for
    /// what it doesn't sell).
    pub buys: bool,
    /// Heals you whole, for this much gold.
    pub heals: Option<u32>,
}

impl Default for Villager {
    fn default() -> Self {
        Villager { role: String::new(), lines: Vec::new(), wander: 60.0, wander_speed: 0.35, flee_range: 165.0, sells: Vec::new(), buys: false, heals: None }
    }
}

/// Where a villager lives (where it was first put).
#[derive(Component, Clone, Copy, Debug)]
pub struct Home(pub Vec2);

/// Hiding at home from a monster near: monsters let it be (`ai.rs`,
/// `monsters.rs`).
#[derive(Component, Clone, Copy, Debug)]
pub struct Hiding;

/// Within this of home (cells across), it's home.
const AT_HOME: f32 = 6.0;

/// A villager's day: where it's heading, till when; and whether it's
/// talking to a player (near) or running.
#[derive(Component, Default, Debug)]
pub struct Routine {
    target: f32,
    until: u64,
    pub talking: bool,
    pub fleeing: bool,
}

type Living<'a> = (Entity, &'a Villager, &'a Kinematics, &'a mut Controls, Option<&'a Home>, Option<&'a mut Routine>, Has<Hiding>);

fn live(
    mut commands: Commands,
    sim: Res<SimWorld>,
    day: Res<crate::light::Daylight>,
    players: Query<&Kinematics, With<LocalPlayer>>,
    others: Query<(&Kinematics, &Team, &Health), Without<Villager>>,
    mut q: Query<Living>,
) {
    let tick = sim.world.tick();
    let hour = day.time * 24.0;
    let night = !(HOME_UNTIL..HOME_FROM).contains(&hour);
    for (e, v, k, mut controls, home, routine, hiding) in &mut q {
        let pos = k.body.pos;
        let (Some(home), Some(mut r)) = (home, routine) else {
            commands.entity(e).insert((Home(pos), Routine { target: pos.x, ..default() }));
            continue;
        };
        let threat = others.iter().filter(|(_, t, h)| **t == Team::Enemy && h.hp > 0.0).map(|(ok, ..)| ok.body.pos).filter(|p| p.distance(pos) < v.flee_range).min_by(|a, b| a.distance_squared(pos).total_cmp(&b.distance_squared(pos)));
        let player = players.iter().map(|pk| pk.body.pos).find(|p| p.distance(pos) < TALK_NEAR);
        r.fleeing = threat.is_some();
        r.talking = player.is_some() && !r.fleeing;
        controls.0.aim = Vec2::ZERO;
        // Home and hidden while there's danger; out again once it's gone.
        let home_now = threat.is_some() && (home.0.x - pos.x).abs() < AT_HOME;
        if home_now != hiding {
            if home_now {
                commands.entity(e).insert(Hiding);
            } else {
                commands.entity(e).remove::<Hiding>();
            }
        }
        let move_x = if threat.is_some() {
            // Home, full tilt.
            let d = home.0.x - pos.x;
            if d.abs() < AT_HOME { 0.0 } else { d.signum() }
        } else if let Some(p) = player {
            controls.0.aim = p + Vec2::new(0.0, 9.0);
            0.0
        } else {
            let target = if night {
                home.0.x
            } else {
                if tick >= r.until {
                    let mut rng = Rng::seeded(&[sim.world.seed(), tick, e.to_bits(), 0x411]);
                    let unit = |rng: &mut Rng| (rng.next_u32() % 10_000) as f32 / 10_000.0;
                    r.target = home.0.x + (unit(&mut rng) * 2.0 - 1.0) * v.wander;
                    r.until = tick + ((3.0 + unit(&mut rng) * 6.0) * 60.0) as u64;
                }
                r.target
            };
            let d = target - pos.x;
            if d.abs() < 4.5 { 0.0 } else { d.signum() * if night { 0.6 } else { v.wander_speed } }
        };
        // A step it can't take: jump it.
        let c = &k.loco.contacts;
        let blocked = (move_x > 0.0 && c.wall_right) || (move_x < 0.0 && c.wall_left);
        controls.0.move_x = move_x;
        controls.0.jump = blocked && k.loco.grounded() && !controls.0.jump;
    }
}
