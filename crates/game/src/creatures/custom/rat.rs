//! Rats: the caves' scavengers, in packs, big as dogs. Their body and
//! their hunt are data (on the ground, a crawler's pounce, a bite on touch,
//! a pack's call and flank); what's theirs:
//!
//! - **Brave in numbers**: with fewer than three of them near together, a
//!   rat won't come at you: near, it scatters from you (squealing); further
//!   off, it hangs back and watches, waiting for more. Cut a pack down and
//!   the rest run. (Trapped against you, it still bites: its touch.)
//! - **Scavenging**: with no one to hunt, rats go to the nearest body and
//!   gnaw it, together; a body gnawed through is gone, what it had on it
//!   left lying where it was.
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
    /// Whether it was afraid last tick (it squeals as it turns tail).
    #[serde(skip)]
    afraid: bool,
    /// What it's doing, in a word (a scenario's log).
    #[serde(skip)]
    doing: &'static str,
    #[serde(skip)]
    seed: f32,
    #[serde(skip)]
    started: bool,
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
    /// What it's doing, in a word: `hunt`, `scatter`, `lurk`, `gnaw`,
    /// `scavenge` (going to a body), `idle`.
    pub fn doing(&self) -> &'static str {
        if self.doing.is_empty() { "idle" } else { self.doing }
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
/// Brave with this many near together (it among them), within this.
const COURAGE: usize = 3;
const TOGETHER: f32 = 100.0;
/// Afraid, it runs from what's this near, and hangs back (watching, not
/// coming on) from what's this near.
const SCATTER: f32 = 110.0;
const LURK: f32 = 180.0;
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
    let rats: Vec<Vec2> = q.iter().map(|(_, _, k, ..)| k.body.pos).collect();
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
        let near = rats.iter().filter(|&&r| r.distance(pos) < TOGETHER).count();
        let quarry = hunted.iter().filter(|(_, t)| t.hunted()).map(|(hk, _)| hk.body.pos).min_by(|a, b| a.distance(pos).total_cmp(&b.distance(pos)));
        let dist = quarry.map_or(f32::MAX, |q| q.distance(pos));
        let mut nibbling = false;
        let afraid = near < COURAGE && dist < SCATTER;
        if near < COURAGE && !afraid && dist < LURK {
            // Hanging back, watching them, not coming on.
            c.0.move_x = 0.0;
            c.0.jump = false;
            c.0.aim = quarry.unwrap_or(pos);
            o.doing = "lurk";
        } else if afraid {
            // Away from them, as fast as it goes (up walls as it comes to
            // them: it climbs).
            let q = quarry.expect("near enough to scatter from");
            c.0.move_x = if pos.x < q.x { -1.0 } else { 1.0 };
            c.0.aim = Vec2::ZERO;
            if !o.afraid {
                sounds.write(crate::sound::PlaySound::at("rat_squeak", pos).volume(1.0).pitch(1.3));
            }
            o.doing = "scatter";
        } else if dist < QUIET {
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
            o.doing = "idle";
        }
        o.afraid = afraid;
        if nibbling {
            if anim.force.as_deref() != Some("nibble") {
                anim.play("nibble");
            }
        } else if anim.force.as_deref() == Some("nibble") {
            anim.force = None;
        }
    }
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
