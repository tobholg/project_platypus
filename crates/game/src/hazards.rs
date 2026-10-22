//! The caves' hazards, and what gives them away (survivable, if you heed
//! it): read off the world's cells, not the plan, so they hold for what
//! the player's dug, built and flooded as well as for what was generated.
//!
//! - **Drips**: under liquid held up by a few cells of rock (the caves'
//!   perched pockets, `worldgen::caves::Pocket`; a pool over a thin floor),
//!   drops fall from the ceiling now and then and splash below. Dig up
//!   there and it all comes down on you.

use bevy::prelude::*;
use platypus_sim::particles::{Landing, Particle};
use platypus_sim::rng::Rng;
use platypus_sim::{CellPos, Kind};

use crate::creatures::Kinematics;
use crate::creatures::player::LocalPlayer;
use crate::sound::PlaySound;
use crate::world::{SimWorld, TickSet};

pub struct HazardsPlugin;

impl Plugin for HazardsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, drips.after(TickSet::Bodies).before(TickSet::Cells));
    }
}

/// Where drips are looked for: this far round the player (cells, across
/// and up and down), this many cells tried a tick (a pocket drips about
/// once a second or two, a bigger one more).
const DRIP_REACH: (i32, i32) = (220, 130);
const DRIP_TRIES: usize = 5;
/// How deep a pocket's liquid may be looked down through, and how thick the
/// rock under it may be, for it to drip (cells).
const DRIP_DEEP: i32 = 30;
const DRIP_ROOF: i32 = 8;

/// Drips under perched liquid near the player, each splashing where it
/// lands (a drop's worth of sound, late by its fall).
fn drips(mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut rng: Local<Option<Rng>>, mut splashes: Local<Vec<(u32, Vec2)>>, mut sounds: MessageWriter<PlaySound>) {
    let rng = rng.get_or_insert_with(|| Rng::seeded(&[0xD219]));
    let unit = |rng: &mut Rng| rng.next_u32() as f32 / u32::MAX as f32;
    splashes.retain_mut(|(ticks, at)| {
        *ticks = ticks.saturating_sub(1);
        if *ticks == 0 {
            sounds.write(PlaySound::at("drip", *at).volume(0.8));
        }
        *ticks > 0
    });
    let Ok(k) = player.single() else { return };
    let c = CellPos::from_world(k.body.pos.x, k.body.pos.y);
    let mats = sim.materials().clone();
    let kind = |sim: &SimWorld, p: CellPos| sim.world.get(p).map(|cell| (cell, mats.phys(cell.material).kind));
    for _ in 0..DRIP_TRIES {
        let p = c.offset(((unit(rng) * 2.0 - 1.0) * DRIP_REACH.0 as f32) as i32, ((unit(rng) * 2.0 - 1.0) * DRIP_REACH.1 as f32) as i32);
        let Some((wet, Kind::Liquid)) = kind(&sim, p) else { continue };
        // Down through it to its bottom, down through what holds it up, to
        // open air under that.
        let Some(bottom) = (1..=DRIP_DEEP).find(|&d| kind(&sim, p.offset(0, -d)).is_none_or(|(_, k)| k != Kind::Liquid)) else { continue };
        let roof = (0..=DRIP_ROOF).find(|&d| kind(&sim, p.offset(0, -bottom - d)).is_none_or(|(_, k)| !matches!(k, Kind::Static | Kind::Powder)));
        let Some(roof) = roof.filter(|&r| r > 0) else { continue };
        let under = p.offset(0, -bottom - roof);
        if !kind(&sim, under).is_some_and(|(_, k)| matches!(k, Kind::Empty | Kind::Gas)) {
            continue;
        }
        let drop = mats.spawn(wet.material, rng);
        let at = [under.x as f32 + 0.2 + 0.6 * unit(rng), under.y as f32 + 0.9];
        sim.world.emit(Particle::new(at, [0.0, -0.05], drop, 240, Landing::Vanish));
        // Its splash, when it's fallen as far as the floor.
        let fall = (1..240).find(|&d| !kind(&sim, under.offset(0, -d)).is_some_and(|(_, k)| matches!(k, Kind::Empty | Kind::Gas))).unwrap_or(240);
        let ticks = (2.0 * fall as f32 / platypus_sim::particles::GRAVITY).sqrt() as u32 + 1;
        splashes.push((ticks, Vec2::new(at[0], (under.y - fall + 1) as f32)));
    }
}
