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
        out.write(PlaySound::at(if heavy { "hit_heavy" } else { "hit" }, h.at).volume(loud));
    }
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

/// The player's footsteps, by what's underfoot, a stride apart; a jump's
/// push off.
fn steps(sim: Res<crate::world::SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut walked: Local<f32>, mut was: Local<(bool, f32)>, mut out: MessageWriter<PlaySound>) {
    let Ok(k) = player.single() else { return };
    let feet = k.body.pos - Vec2::Y * (k.body.half.y + 0.5);
    let ground = k.loco.grounded();
    // A jump: off the ground, rising fast.
    if was.0 && !ground && k.body.vel.y > 120.0 && was.1 <= 20.0 {
        out.write(PlaySound::at("jump", feet).volume(0.7));
    }
    *was = (ground, k.body.vel.y);
    if !ground || k.body.vel.x.abs() < 15.0 {
        *walked = walked.min(10.0);
        return;
    }
    *walked += (k.body.pos - k.prev_pos).length();
    if *walked < 16.0 {
        return;
    }
    *walked = 0.0;
    if let Some(what) = underfoot(&sim.world, CellPos::from_world(feet.x, feet.y)) {
        let name = match what {
            "stone" => "step_stone",
            "dirt" => "step_dirt",
            "sand" => "step_sand",
            "wood" => "step_wood",
            "snow" => "step_snow",
            "water" => "step_water",
            _ => "step_grass",
        };
        out.write(PlaySound::at(name, feet).volume((k.body.vel.x.abs() / 90.0).clamp(0.5, 1.0)));
    }
}

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
