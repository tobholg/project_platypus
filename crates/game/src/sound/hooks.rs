//! The game's own messages, heard: hits, hurts, deaths, footsteps and
//! landings by what's underfoot, jumps, dashes, blasts, thunder, casts,
//! crafting and milestones. (Swings, clangs, bounces and mining ask for
//! their sounds where they happen: `combat.rs`, `hands`.)

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind, World};

use super::PlaySound;
use crate::actors::player::LocalPlayer;
use crate::actors::{Died, Health, Kinematics, Landed};

pub struct HooksPlugin;

impl Plugin for HooksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (hits, hurts, deaths, steps, landings, world_sounds, casts, crafts).before(super::play));
    }
}

/// What a cell sounds like to walk on or dig: stone, dirt, sand, wood,
/// snow, grass (or none: air).
pub fn underfoot(world: &World, p: CellPos) -> Option<&'static str> {
    let cell = world.get(p)?;
    let mats = world.materials();
    let ph = mats.phys(cell.material);
    let name = mats.def(cell.material).name.as_str();
    let has = |s: &str| name.contains(s);
    Some(match ph.kind {
        Kind::Empty | Kind::Gas | Kind::Fire => return None,
        Kind::Liquid => "water",
        Kind::Plant => "grass",
        Kind::Powder if has("snow") => "snow",
        Kind::Powder => "sand",
        Kind::Static if has("wood") || has("plank") || has("log") || has("platform") => "wood",
        Kind::Static if has("snow") || has("ice") => "snow",
        Kind::Static if has("dirt") || has("mud") || has("clay") || has("grass") || has("moss") => "dirt",
        Kind::Static => "stone",
    })
}

/// Every blow lands with a sound where it struck (a crit, louder).
fn hits(mut hits: MessageReader<crate::combat::Hit>, mut out: MessageWriter<PlaySound>) {
    for h in hits.read() {
        let loud = (0.6 + h.weight * 0.3).min(1.3) * if h.crit { 1.3 } else { 1.0 };
        let heavy = h.weight >= 1.6 || h.crit;
        out.write(PlaySound::at(if heavy { "hit_heavy" } else { hit_name() }, h.at).volume(loud));
    }
}

/// Which "hit" a blow plays: `PLATYPUS_HIT`=b or c for the other takes
/// (to compare by ear), `old` for the one before.
fn hit_name() -> &'static str {
    static NAME: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    NAME.get_or_init(|| match std::env::var("PLATYPUS_HIT").as_deref() {
        Ok("b") => "hit_b",
        Ok("c") => "hit_c",
        Ok("old") => "hit_old",
        _ => "hit",
    })
}

/// The player hurt, whatever hurt them (a blow, fire, a fall).
fn hurts(player: Query<(&Health, &Kinematics), With<LocalPlayer>>, mut last: Local<Option<f32>>, mut out: MessageWriter<PlaySound>) {
    let Ok((h, k)) = player.single() else { return };
    if let Some(was) = *last
        && was - h.hp >= 2.0
        && h.hp > 0.0
    {
        out.write(PlaySound::at("hurt", k.body.pos).volume((0.6 + (was - h.hp) / 40.0).min(1.2)));
    }
    *last = Some(h.hp);
}

fn deaths(mut died: MessageReader<Died>, mut out: MessageWriter<PlaySound>) {
    for d in died.read() {
        // (Bigger, lower.)
        let size = (d.body.half.y / 8.0).clamp(0.5, 2.5);
        out.write(PlaySound::at("death", d.body.pos).pitch(1.0 / size.sqrt()).volume(0.7 + 0.2 * size));
    }
}

/// The player's footsteps: a steady beat while running on the ground (a
/// little quicker the faster), by what's underfoot; a jump's push off.
fn steps(sim: Res<crate::world::SimWorld>, time: Res<Time>, player: Query<&Kinematics, With<LocalPlayer>>, mut beat: Local<f32>, mut was: Local<(bool, f32)>, mut out: MessageWriter<PlaySound>) {
    let Ok(k) = player.single() else { return };
    let feet = k.body.pos - Vec2::Y * (k.body.half.y + 0.5);
    let ground = k.loco.grounded();
    // A jump: off the ground, rising fast.
    if was.0 && !ground && k.body.vel.y > 120.0 && was.1 <= 20.0 {
        out.write(PlaySound::at("jump", feet).volume(0.7));
    }
    *was = (ground, k.body.vel.y);
    let speed = k.body.vel.x.abs();
    if !ground || speed < 20.0 {
        // (The first step comes soon after starting.)
        *beat = beat.min(0.1);
        return;
    }
    *beat -= time.delta_secs();
    if *beat > 0.0 {
        return;
    }
    *beat = STEP_BEAT * (70.0 / speed).clamp(0.8, 1.3);
    // (What it stands on anywhere under its feet: on a bump's corner the
    // middle can be over air.)
    let under = [0.0, -0.7, 0.7, -1.0, 1.0]
        .iter()
        .flat_map(|&dx| [0.0, 1.0].map(|dy| Vec2::new(feet.x + dx * (k.body.half.x - 0.5), feet.y - dy)))
        .find_map(|p| underfoot(&sim.world, CellPos::from_world(p.x, p.y)));
    if let Some(what) = under {
        let name = match what {
            "stone" => "step_stone",
            "dirt" => "step_dirt",
            "sand" => "step_sand",
            "wood" => "step_wood",
            "snow" => "step_snow",
            "water" => "step_water",
            _ => "step_grass",
        };
        out.write(PlaySound::at(name, feet));
    }
}

/// Seconds between footsteps at a run (~70 cells/s).
const STEP_BEAT: f32 = 0.3;

/// Coming down: a thump by how far (small hops say nothing).
fn landings(mut landed: MessageReader<Landed>, bodies: Query<(&Kinematics, Has<LocalPlayer>)>, mut out: MessageWriter<PlaySound>) {
    for l in landed.read() {
        let Ok((k, player)) = bodies.get(l.entity) else { continue };
        if l.drop < 6.0 && l.slam < 150.0 {
            continue;
        }
        let hard = ((l.drop / 80.0).max(l.slam / 500.0)).clamp(0.2, 1.3);
        let at = k.body.pos - Vec2::Y * k.body.half.y;
        out.write(PlaySound::at("land", at).volume(hard * if player { 1.0 } else { 0.7 }).pitch((1.0 - hard * 0.2).max(0.7)));
    }
}

/// Blasts, thunder, wand lightning, double jumps and dodges.
fn world_sounds(
    mut blasts: MessageReader<crate::fx::Explosion>,
    mut strikes: MessageReader<crate::fx::Lightning>,
    mut zaps: MessageReader<crate::fx::Zapped>,
    mut air: MessageReader<crate::actors::AirJumped>,
    mut dashes: MessageReader<crate::combat::Dashed>,
    bodies: Query<&Kinematics>,
    mut out: MessageWriter<PlaySound>,
) {
    for b in blasts.read() {
        let big = (b.radius / 20.0).clamp(0.3, 2.0);
        out.write(PlaySound::at("boom", b.at).volume((0.5 + big * 0.5).min(1.4)).pitch(1.0 / big.sqrt()));
    }
    for s in strikes.read() {
        out.write(PlaySound::at("thunder", Vec2::new(s.0.hit.x as f32, s.0.hit.y as f32)));
    }
    for z in zaps.read() {
        out.write(PlaySound::at("zap", Vec2::new(z.0.to.x as f32, z.0.to.y as f32)).volume(0.8));
    }
    for a in air.read() {
        out.write(PlaySound::at("air_jump", a.at).volume(0.6));
    }
    for d in dashes.read() {
        if let Ok(k) = bodies.get(d.0) {
            out.write(PlaySound::at("dash", k.body.pos).volume(0.7));
        }
    }
}

/// A spell cast (a creature's spit too), from where it leaves.
fn casts(mut casts: MessageReader<crate::magic::CastRequest>, book: Res<crate::magic::Spellbook>, mut out: MessageWriter<PlaySound>) {
    for c in casts.read() {
        let beast = book.spells.get(c.spell).is_some_and(|s| s.beast);
        out.write(PlaySound::at(if beast { "spit" } else { "cast" }, c.from).volume(0.7));
    }
}

/// Something made; a milestone reached.
fn crafts(mut made: MessageReader<crate::craft::CraftRequest>, mut toasts: MessageReader<crate::progress::Toast>, mut out: MessageWriter<PlaySound>) {
    for _ in made.read() {
        out.write(PlaySound::here("craft"));
    }
    for _ in toasts.read() {
        out.write(PlaySound::here("toast").volume(0.8));
    }
}
