//! Gold, Noita's way (DESIGN §13 item 7): a count, not a thing in the pack.
//!
//! The player's gold is a number (`Gold`, saved with the player, shown in
//! the HUD). In the world it's a material, `gold` (materials.ron): dust, one
//! cell a coin, that acts as any other. It bursts out of the dying and out
//! of a chest the first time it's opened (the loot tables' `gold`, more the
//! deeper) as specks that land and pile, glittering and glowing; a great
//! heap lights a cave. Being a material, the rest is the simulation's: it's
//! saved with its chunk, forever; it sinks in water; acid can't eat it
//! (`inert`); blasts throw it; lava melts it into `molten_gold`, which sets
//! into `solid_gold` as it cools (a thing, like any metal: dug, it's coins
//! again); the vaporiser erases it.
//!
//! A player takes the gold near it: walked into, it streams in (glinting
//! specks), up to `TAKE_PER_TICK` cells a tick, so a heap drains in a gush.
//! Dug with a pickaxe, it's counted too (`Dug`: it's `counted`, never a
//! block item).

use bevy::prelude::*;
use platypus_sim::rng::{Rng, hash};
use platypus_sim::{Cell, CellPos, Landing, MaterialId, Particle};

use crate::actors::Kinematics;
use crate::actors::player::LocalPlayer;
use crate::magic::runes::Emitter;
use crate::world::{SimWorld, TickSet};

/// A player takes gold this many cells from its body (all round).
const REACH: i32 = 6;
/// Most gold taken a tick (a heap drains at ~2 900 a second).
const TAKE_PER_TICK: u32 = 48;
/// Most specks a burst throws a frame (the rest wait for the next).
const BURST_PER_FRAME: u32 = 600;

pub struct GoldPlugin;

impl Plugin for GoldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GoldQueue>()
            .add_message::<Dug>()
            .add_systems(Update, (spill_queued, count_dug))
            .add_systems(FixedUpdate, take.in_set(TickSet::Bodies));
    }
}

/// A player's gold.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Gold(pub u32);

/// Gold the local player dug with its hands (cells).
#[derive(Message, Clone, Copy, Debug)]
pub struct Dug(pub u32);

/// Gold to throw into the world: where, how much.
#[derive(Resource, Default)]
pub struct GoldQueue {
    pub burst: Vec<(Vec2, u32)>,
}

/// The HUD's nugget, as text art ('o' dark, 'g' gold, 'W' shine).
pub const ICON: &[&str] = &[".gWg.", "ggggg", "oggoo", ".oo.."];

/// The icon's colours.
pub fn art_color(c: char) -> Option<[u8; 4]> {
    match c {
        'o' => Some([150, 98, 22, 255]),
        'g' => Some([236, 186, 52, 255]),
        'W' => Some([255, 244, 190, 255]),
        _ => None,
    }
}

/// Gold bursting out at `at`: `amount` specks of gold flung up and about,
/// landing as dust.
fn burst(sim: &mut SimWorld, gold: MaterialId, at: Vec2, amount: u32, seed: u64) {
    let mut rng = Rng::seeded(&[seed, 0x601D]);
    let unit = |rng: &mut Rng| rng.next_u32() as f32 / u32::MAX as f32;
    let mats = sim.world.materials().clone();
    for _ in 0..amount {
        // (Cells a tick: up and out, a fountain.)
        let a = (55.0 + 70.0 * unit(&mut rng)).to_radians();
        let speed = 0.4 + 0.9 * unit(&mut rng);
        let vel = [a.cos() * speed, a.sin() * speed];
        let jiggle = [unit(&mut rng) * 3.0 - 1.5, unit(&mut rng) * 2.0];
        let cell: Cell = mats.spawn(gold, &mut rng);
        sim.world.emit(Particle::new([at.x + jiggle[0], at.y + 2.0 + jiggle[1]], vel, cell, 90, Landing::Settle));
    }
}

/// What the loot tables found (`Chests::gold_found`: the dying, chests
/// opened) and anything else queued, into the world (a few hundred specks a
/// frame at most: a hoard pours out over a moment).
fn spill_queued(mut queue: ResMut<GoldQueue>, mut chests: ResMut<crate::hands::chests::Chests>, mut sim: ResMut<SimWorld>) {
    queue.burst.append(&mut chests.gold_found);
    if queue.burst.is_empty() {
        return;
    }
    let Some(gold) = sim.world.materials().id("gold") else {
        queue.burst.clear();
        return;
    };
    let mut budget = BURST_PER_FRAME;
    let mut left = Vec::new();
    for (at, amount) in std::mem::take(&mut queue.burst) {
        let now = amount.min(budget);
        if now > 0 {
            let seed = hash(&[sim.world.tick(), at.x.to_bits() as u64, at.y.to_bits() as u64, amount as u64]);
            burst(&mut sim, gold, at, now, seed);
            budget -= now;
        }
        if amount > now {
            left.push((at, amount - now));
        }
    }
    queue.burst = left;
}

/// The local player takes the gold within `REACH` of it: the cells go, the
/// count goes up, specks stream in.
fn take(
    mut sim: ResMut<SimWorld>,
    mut players: Query<(&Kinematics, &mut Gold), With<LocalPlayer>>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut ding: Local<f32>,
) {
    let Some(gold) = sim.world.materials().id("gold") else { return };
    *ding -= 1.0 / crate::world::TICK_HZ as f32;
    for (k, mut purse) in &mut players {
        let (lo, hi) = k.body.cells_at(k.body.pos);
        let mut got = Vec::new();
        'look: for y in lo.y - REACH..=hi.y + REACH {
            for x in lo.x - REACH..=hi.x + REACH {
                let p = CellPos::new(x, y);
                if sim.world.get(p).is_some_and(|c| c.material == gold) {
                    got.push(p);
                    if got.len() as u32 >= TAKE_PER_TICK {
                        break 'look;
                    }
                }
            }
        }
        if got.is_empty() {
            continue;
        }
        for &p in &got {
            sim.world.set(p, Cell::AIR);
        }
        purse.0 = purse.0.saturating_add(got.len() as u32);
        // A few specks streaming in from what was taken.
        let e = streak();
        for p in got.iter().step_by(4) {
            let at = Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5);
            sparks.emit(&e, 1, at, k.body.pos - at, Vec2::ZERO);
        }
        if *ding <= 0.0 {
            sounds.write(crate::sound::PlaySound::here("gold").volume((0.5 + got.len() as f32 / 48.0).min(1.0)));
            *ding = 0.07;
        }
    }
}

/// A speck of gold flying in.
fn streak() -> Emitter {
    Emitter {
        count: 1.0,
        life: (0.08, 0.16),
        colors: vec![(255, 240, 170), (240, 190, 60)],
        speed: 90.0,
        spread: 0.2,
        gravity: 0.0,
        drag: 0.0,
        size: 1.0,
        jitter: 0.0,
        glow: true,
    }
}

/// Gold dug with the hands is counted.
fn count_dug(mut dug: MessageReader<Dug>, mut players: Query<&mut Gold, With<LocalPlayer>>) {
    let n: u32 = dug.read().map(|d| d.0).sum();
    if n > 0
        && let Ok(mut g) = players.single_mut()
    {
        g.0 = g.0.saturating_add(n);
    }
}
