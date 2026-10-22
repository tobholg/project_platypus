//! Rats: the caves' scavengers, in packs, big as dogs. Their body and
//! their hunt are data (on the ground, a crawler's pounce, a bite on touch,
//! a pack's call and flank); what's theirs:
//!
//! - **Always at you**: one alone comes at you as a pack does (its hunt:
//!   the brain's crawl, a pounce, its bite).
//! - **Scavenging**: with no one to hunt, rats go to the nearest body and
//!   gnaw it, together; a body gnawed through is gone, what it had on it
//!   left lying where it was.
//! - **Skittish**: idle, never still for long: a dart one way or the
//!   other, a freeze, a twitch, now and then up the nearest wall.
//! - Squeaks now and then.

use bevy::prelude::*;
use serde::Deserialize;

use super::{CustomCreature, CustomSet};
use crate::creatures::body::animation::Animator;
use crate::creatures::{Controls, Kinematics, Team};
use crate::hands::corpses::Corpse;
use crate::world::{SimWorld, TickSet};

#[derive(Component, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Rat {
    /// Till its next squeak, its next gnaw's sound.
    #[serde(skip)]
    squeak: f32,
    #[serde(skip)]
    chew: f32,
    /// What it's doing, in a word (a scenario's log).
    #[serde(skip)]
    doing: &'static str,
    #[serde(skip)]
    seed: f32,
    #[serde(skip)]
    started: bool,
    /// Idle: what it's doing (move x, move y) till when (its own clock).
    #[serde(skip)]
    fidget: (f32, f32),
    #[serde(skip)]
    fidget_left: f32,
    #[serde(skip)]
    fidgets: u32,
}

/// How long a body's been gnawed (rat-seconds, summed over the rats at it).
#[derive(Component, Default)]
pub struct Gnawed(pub f32);

impl CustomCreature for Rat {
    const NAME: &'static str = "rat";

    fn build(app: &mut App) {
        app.add_systems(FixedUpdate, think.in_set(CustomSet::Think)).add_systems(FixedUpdate, devour.in_set(TickSet::Bodies));
    }
}

impl Rat {
    /// What it's doing, in a word: `hunt`, `gnaw`,
    /// `scavenge` (going to a body), `idle`.
    pub fn doing(&self) -> &'static str {
        if self.doing.is_empty() { "idle" } else { self.doing }
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
/// With no one this near, it goes to a body this near; at it within this,
/// it gnaws.
const QUIET: f32 = 140.0;
const SMELL: f32 = 220.0;
const AT: f32 = 7.0;
/// A body's gone, gnawed this long.
const EATEN: f32 = 30.0;
/// Squeaking every so often (s, from..to); gnawing, a sound every so often.
const SQUEAK: (f32, f32) = (2.5, 7.0);
const CHEW: f32 = 0.22;

#[allow(clippy::type_complexity)]
fn think(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Rat, &Kinematics, &mut Controls, &mut Animator)>,
    hunted: Query<(&Kinematics, &Team), Without<Rat>>,
    mut bodies: Query<(Entity, &Kinematics, Option<&mut Gnawed>), (With<Corpse>, Without<Rat>)>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    for (e, mut o, k, mut c, mut anim) in &mut q {
        let pos = k.body.pos;
        if !o.started {
            o.started = true;
            o.seed = (e.to_bits() % 997) as f32 / 997.0;
            o.squeak = SQUEAK.0 + (SQUEAK.1 - SQUEAK.0) * o.seed;
        }
        o.squeak -= DT;
        if o.squeak <= 0.0 {
            o.squeak = SQUEAK.0 + (SQUEAK.1 - SQUEAK.0) * (o.seed * 7.7 + o.squeak.abs()).fract();
            sounds.write(crate::sound::PlaySound::at("rat_squeak", pos).volume(0.6).pitch(0.9 + o.seed * 0.3));
        }
        let quarry = hunted.iter().filter(|(_, t)| t.hunted()).map(|(hk, _)| hk.body.pos).min_by(|a, b| a.distance(pos).total_cmp(&b.distance(pos)));
        let dist = quarry.map_or(f32::MAX, |q| q.distance(pos));
        let mut nibbling = false;
        if dist < QUIET {
            o.doing = "hunt";
        } else if let Some((body, bk, gnawed)) = bodies.iter_mut().filter(|(_, bk, _)| bk.body.pos.distance(pos) < SMELL).min_by(|a, b| a.1.body.pos.distance(pos).total_cmp(&b.1.body.pos.distance(pos))) {
            let dx = bk.body.pos.x - pos.x;
            if dx.abs() > AT {
                c.0.move_x = dx.signum() * 0.8;
                o.doing = "scavenge";
            } else {
                // Gnawing it: still, head down, its teeth at it.
                c.0.move_x = 0.0;
                c.0.jump = false;
                nibbling = true;
                o.doing = "gnaw";
                match gnawed {
                    Some(mut g) => g.0 += DT,
                    None => {
                        commands.entity(body).insert(Gnawed(DT));
                    }
                }
                o.chew -= DT;
                if o.chew <= 0.0 {
                    o.chew = CHEW * (0.7 + 0.6 * o.seed);
                    sounds.write(crate::sound::PlaySound::at("gnaw", bk.body.pos).pitch(0.9 + o.seed * 0.3));
                }
            }
        } else {
            // Skittish: a dart, a freeze, a twitch, up a wall now and then.
            fidget(&mut o);
            c.0.move_x = o.fidget.0;
            c.0.move_y = o.fidget.1;
            c.0.jump = false;
            o.doing = "idle";
        }
        if nibbling {
            if anim.force.as_deref() != Some("nibble") {
                anim.play("nibble");
            }
        } else if anim.force.as_deref() == Some("nibble") {
            anim.force = None;
        }
    }
}

/// Its fidgeting on its own (idle): as it last chose, till that
/// runs out; then a freeze, a dart one way or the other, a twitch, or a
/// run up whatever wall it comes to.
fn fidget(o: &mut Rat) {
    o.fidget_left -= DT;
    if o.fidget_left > 0.0 {
        return;
    }
    o.fidgets += 1;
    let (n, seed) = (o.fidgets, o.seed);
    let roll = |salt: u32| (((n.wrapping_mul(2_654_435_761) ^ salt) as f32 * 0.618_034 + seed * 7.31).sin() * 43_758.547).fract().abs();
    let (r, side) = (roll(1), if roll(2) < 0.5 { -1.0 } else { 1.0 });
    (o.fidget, o.fidget_left) = if r < 0.35 {
        ((0.0, 0.0), 0.2 + 0.7 * roll(3))
    } else if r < 0.8 {
        ((side, 0.0), 0.15 + 0.35 * roll(4))
    } else if r < 0.9 {
        ((side * 0.4, 0.0), 0.08 + 0.1 * roll(5))
    } else {
        ((side, 1.0), 0.6 + 0.6 * roll(6))
    };
}

/// A body gnawed through: gone, what it had on it left lying there, a
/// little blood where it was.
fn devour(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    mut chests: ResMut<crate::hands::chests::Chests>,
    q: Query<(Entity, &Corpse, &Kinematics, &Gnawed)>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    let Some(items) = items else { return };
    for (e, corpse, k, g) in &q {
        if g.0 < EATEN {
            continue;
        }
        let at = k.body.pos;
        if let Some(inv) = chests.inventory(corpse.key) {
            for s in inv.slots.iter().flatten() {
                crate::hands::spawn_drop(&mut commands, &items, at, *s);
            }
        }
        chests.forget(corpse.key);
        commands.entity(e).despawn();
        if let Some(blood) = sim.materials().id("blood") {
            sim.queue(platypus_sim::WorldEdit::Paint { center: platypus_sim::CellPos::from_world(at.x, at.y - k.body.half.y + 1.0), radius: 2, material: blood, overwrite: false });
        }
        sounds.write(crate::sound::PlaySound::at("gnaw", at).volume(1.0).pitch(0.7));
    }
}
