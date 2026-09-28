//! Explosives (TNT barrels, dynamite, a mine cart loaded with them): props
//! the miners left in the caves, creatures with no mind (their creature
//! file's brain `explosive`), so whatever hurts a creature sets them off:
//! a blade, an arrow, a spell, another blast (they go off in a chain).
//! Broken, one goes off at once; caught alight (a spark, embers, a fire
//! bolt), its fuse fizzes for `fuse` seconds first. Either way the blast is
//! a bomb's (`WorldEdit::Explode`: it digs, burns and throws; creatures
//! feel it through `fx::Explosion`), `radius` and `power` its own.
//!
//! A lantern on a post (brain `idle`, a light) stands with them.

use bevy::prelude::*;
use platypus_sim::{CellPos, WorldEdit};
use serde::Deserialize;

use super::brain::RegisterBrain;
use super::elements::Burning;
use super::{Health, Kinematics};
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;

pub struct ExplosivePlugin;

impl Plugin for ExplosivePlugin {
    fn build(&self, app: &mut App) {
        app.register_brain::<Explosive>("explosive").add_systems(Update, fizz);
    }
}

/// How it goes off: the blast's radius (cells) and power (a bomb's are 38
/// and 140), and the fuse once it's alight (seconds).
#[derive(Component, Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Explosive {
    pub radius: i32,
    pub power: u8,
    pub fuse: f32,
    /// Alight: seconds left on the fuse.
    #[serde(skip)]
    pub lit: Option<f32>,
}

impl Default for Explosive {
    fn default() -> Self {
        Explosive { radius: 30, power: 140, fuse: 0.8, lit: None }
    }
}

/// Broken: it goes off now; alight: when its fuse runs down. (Before
/// `deaths`, which then takes it away: it leaves no body.)
pub fn detonate(mut sim: ResMut<SimWorld>, mut q: Query<(&mut Explosive, &Kinematics, &mut Health, Has<Burning>)>) {
    for (mut x, k, mut health, burning) in &mut q {
        if burning && x.lit.is_none() && health.hp > 0.0 {
            x.lit = Some(x.fuse);
        }
        if let Some(left) = x.lit.as_mut() {
            *left -= DT;
        }
        let spent = x.lit.is_some_and(|l| l <= 0.0);
        if health.hp > 0.0 && !spent {
            continue;
        }
        // (Gone once: marked spent with no health.)
        if x.power == 0 {
            continue;
        }
        let at = k.body.pos;
        sim.world.apply_edit(&WorldEdit::Explode { center: CellPos::from_world(at.x, at.y), radius: x.radius, power: x.power });
        x.power = 0;
        health.hp = 0.0;
    }
}

/// A lit fuse fizzes: sparks from its top, faster as it runs down.
fn fizz(time: Res<Time>, mut sparks: ResMut<crate::vfx::Sparks>, settings: Res<crate::light::LightSettings>, q: Query<(&Explosive, &Kinematics)>) {
    let dt = time.delta_secs();
    for (x, k) in &q {
        let Some(left) = x.lit else { continue };
        let rate = 30.0 + 90.0 * (1.0 - left / x.fuse.max(0.01)).clamp(0.0, 1.0);
        let n = (rate * dt + (time.elapsed_secs() * 7.3 + k.body.pos.x).fract()).floor() as usize;
        if n > 0 {
            sparks.emit(&settings.fire.embers, n, k.body.pos + Vec2::new(0.0, k.body.half.y), Vec2::Y, Vec2::ZERO);
        }
    }
}
