//! Breath: the player's head under liquid, it holds its breath (a status,
//! top right, running down: 18 s, Hardened 12; a diving helmet's `Breath`
//! stat makes it last longer); bubbles stream off it,
//! faster as it runs low, and the screen's edges darken blue (`screen_fx`).
//! Out of breath, it drowns: a share of its health a second, whatever it
//! wears. Its head out, it's back in a couple of seconds (a gasp, if it
//! was low). Wading, swimming at the top: none of that. A bladder pod
//! (a cluster of them on the rock under water) bursts as you swim into it,
//! a gulp of air: a third of a full breath back.

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind};

use crate::creatures::player::LocalPlayer;
use crate::creatures::{Harm, Health, Kinematics};
use crate::difficulty::Difficulty;
use crate::magic::runes::Emitter;
use crate::world::SimWorld;

pub struct BreathPlugin;

impl Plugin for BreathPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, breathe);
    }
}

/// How much breath is left (seconds), of how much, and whether its head's
/// under.
#[derive(Component, Debug)]
pub struct Breath {
    pub left: f32,
    pub most: f32,
    pub under: bool,
}

impl Breath {
    /// What's left of it (0..1).
    pub fn share(&self) -> f32 {
        (self.left / self.most.max(0.01)).clamp(0.0, 1.0)
    }
}

/// Breath held (seconds): on Hardened, and otherwise.
const HELD_HARD: f32 = 12.0;
const HELD: f32 = 18.0;
/// All of it back this fast, its head out (seconds).
const REFILL: f32 = 2.0;
/// Out of breath, this share of its health a second.
const DROWNING: f32 = 0.05;
/// Its head: this far below the top of its body (cells).
const HEAD_DOWN: f32 = 3.0;
/// Bubbles off it a second: with all its breath, and with none.
const BUBBLES: (f32, f32) = (1.5, 9.0);
/// Surfacing with less than this share left, it gasps.
const GASP_BELOW: f32 = 0.5;
/// A bladder pod's gulp: this share of a full breath.
const POD_AIR: f32 = 0.34;
/// Air this thin (0 … 1) still breathes.
const THIN_BREATHES: f32 = 0.05;

fn breathe(
    mut commands: Commands,
    time: Res<Time>,
    mut sim: ResMut<SimWorld>,
    difficulty: Res<Difficulty>,
    dying: Res<crate::death::Dying>,
    mut player: Query<(Entity, &Kinematics, &mut Health, Option<&mut Breath>, Option<&crate::gear::Stats>), With<LocalPlayer>>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut owed: Local<f32>,
) {
    let Ok((me, k, mut h, breath, stats)) = player.single_mut() else { return };
    let most = if *difficulty == Difficulty::Hardened { HELD_HARD } else { HELD } * stats.map_or(1.0, |s| s.mult(crate::gear::Stat::Breath));
    let Some(mut b) = breath else {
        commands.entity(me).insert(Breath { left: most, most, under: false });
        return;
    };
    let dt = time.delta_secs().min(0.05);
    b.most = most;
    // (A helmet taken off: no more held than it can be.)
    b.left = b.left.min(most);
    let head = k.body.pos + Vec2::new(0.0, k.body.half.y - HEAD_DOWN);
    let under = dying.0.is_none()
        && sim.world.get(CellPos::from_world(head.x, head.y)).is_some_and(|c| !c.is_air() && sim.world.materials().phys(c.material).kind == Kind::Liquid);
    if b.under && !under && b.share() < GASP_BELOW {
        sounds.write(crate::sound::PlaySound::at("gasp", head));
    }
    b.under = under;
    // High in the sky the air thins: breath runs down (faster the higher:
    // none to breathe at the world's top), and out of it you suffocate.
    let thin = if under || dying.0.is_some() { 0.0 } else { sim.generator.sky_layers().map_or(0.0, |s| s.thin(head.y)) };
    if !under && thin > THIN_BREATHES {
        b.left = (b.left - dt * thin).max(0.0);
        if b.left <= 0.0 && h.hp > 0.0 {
            let dealt = DROWNING * h.max * dt * thin;
            h.hp -= dealt;
            if h.records {
                h.recent.push(crate::death::Hurt { cause: crate::death::Cause::new("Thin air", "suffocated"), kind: Harm::Blunt, dealt, age: 0.0 });
            }
        }
        return;
    }
    if !under {
        b.left = (b.left + dt * most / REFILL).min(most);
        return;
    }
    // A bladder pod touched: it bursts, a gulp of air.
    if let Some(pod) = sim.materials().id("bladder_pod") {
        let (lo, hi) = k.body.cells_at(k.body.pos);
        let touched: Vec<CellPos> = (lo.y..=hi.y).flat_map(|y| (lo.x..=hi.x).map(move |x| CellPos::new(x, y))).filter(|&p| sim.world.get(p).is_some_and(|c| c.material == pod)).collect();
        if !touched.is_empty() {
            let water = sim.materials().id("water");
            for p in &touched {
                // (The whole cluster with it.)
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let q = p.offset(dx, dy);
                        if sim.world.get(q).is_some_and(|c| c.material == pod)
                            && let Some(w) = water
                        {
                            sim.world.set(q, platypus_sim::Cell::new(w, 128));
                        }
                    }
                }
            }
            let at = Vec2::new(touched[0].x as f32 + 0.5, touched[0].y as f32 + 0.5);
            b.left = (b.left + most * POD_AIR).min(most);
            sparks.emit(&BUBBLE, 14, at, Vec2::Y, Vec2::ZERO);
            sounds.write(crate::sound::PlaySound::at("pod_pop", at));
        }
    }
    b.left = (b.left - dt).max(0.0);
    // Bubbles off its mouth, faster the less is left.
    let share = b.share();
    *owed += dt * (BUBBLES.1 + (BUBBLES.0 - BUBBLES.1) * share);
    while *owed >= 1.0 {
        *owed -= 1.0;
        sparks.emit(&BUBBLE, 1, head + Vec2::new(k.loco.facing * 2.0, 0.0), Vec2::Y, Vec2::ZERO);
    }
    // Out of it: drowning (armour or no).
    if b.left <= 0.0 && h.hp > 0.0 {
        let dealt = DROWNING * h.max * dt;
        h.hp -= dealt;
        if h.records {
            h.recent.push(crate::death::Hurt { cause: crate::death::Cause::new("Water", "drowned"), kind: Harm::Blunt, dealt, age: 0.0 });
        }
    }
}

/// A bubble off a held breath: wobbling up.
static BUBBLE: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.6, 1.2),
    colors: vec![(230, 245, 255), (170, 210, 245)],
    speed: 12.0,
    spread: 0.4,
    gravity: -55.0,
    drag: 1.5,
    size: 1.0,
    jitter: 6.0,
    glow: false,
});
