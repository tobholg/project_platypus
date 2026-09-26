//! Built-in AI brains. Each is a component (its tunables, from the creature
//! file) plus one system writing `Controls`. Copy one to make a new behaviour.

use bevy::prelude::*;
use platypus_sim::rng::Rng;
use serde::Deserialize;

use super::brain::RegisterBrain;
use super::{Controls, Kinematics, Team};
use crate::world::{SimWorld, TickSet};

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.register_brain::<MeleeWalker>("melee_walker")
            .register_brain::<Idle>("idle")
            .register_brain::<Archer>("archer")
            .add_systems(FixedUpdate, (melee_walker, archer).in_set(TickSet::Intent));
    }
}

/// Stands still. Useful for training dummies and for testing new art.
#[derive(Component, Deserialize, Default)]
pub struct Idle;

/// Walks toward the nearest player, jumps over obstacles and up to ledges,
/// wanders when nobody is near; within `reach`, swings what it wields
/// (`combo` moves in a row, then waits `attack_every` s), standing its
/// ground while it swings.
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct MeleeWalker {
    /// Notices players within this distance (cells).
    pub aggro_range: f32,
    /// Stops approaching at this horizontal distance.
    pub keep_distance: f32,
    /// Fraction of run speed used while wandering.
    pub wander_speed: f32,
    /// Seconds between wander decisions (randomised ±50%).
    pub wander_every: f32,
    /// Jump when a player is this much higher (cells) and close horizontally.
    pub jump_to_reach: f32,
    /// Swings at a player this close (cells; 0: never).
    pub reach: f32,
    /// Seconds it waits after an attack before the next (randomised ±30 %).
    pub attack_every: f32,
    /// Moves of its weapon's combo it swings in one attack.
    pub combo: u8,
}

impl Default for MeleeWalker {
    fn default() -> Self {
        MeleeWalker { aggro_range: 220.0, keep_distance: 10.0, wander_speed: 0.4, wander_every: 2.5, jump_to_reach: 18.0, reach: 0.0, attack_every: 1.5, combo: 1 }
    }
}

/// Per-creature memory for simple brains.
#[derive(Component, Default)]
pub struct WanderState {
    until_tick: u64,
    dir: f32,
    /// When it may attack next, swings still to ask for in this one, and
    /// whether one is under way (the wait starts when it ends).
    next_attack: u64,
    swings_left: u8,
    attacking: bool,
}

type Walker<'a> = (Entity, &'a MeleeWalker, &'a Kinematics, &'a mut Controls, Option<&'a mut WanderState>, Has<crate::combat::Swing>);

fn melee_walker(
    mut commands: Commands,
    sim: Res<SimWorld>,
    players: Query<(&Kinematics, &Team), Without<MeleeWalker>>,
    mut q: Query<Walker>,
    mut swings: MessageWriter<crate::combat::MeleeRequest>,
) {
    let tick = sim.world.tick();
    for (entity, brain, k, mut controls, wander, swinging) in &mut q {
        let Some(mut w) = wander else {
            commands.entity(entity).insert(WanderState::default());
            continue;
        };
        let pos = k.body.pos;
        let target = players
            .iter()
            .filter(|(_, t)| **t == Team::Player)
            .map(|(pk, _)| pk.body.pos)
            .filter(|p| p.distance(pos) < brain.aggro_range)
            .min_by(|a, b| a.distance_squared(pos).total_cmp(&b.distance_squared(pos)));

        let mut want_jump = false;
        let c = &k.loco.contacts;
        controls.0.aim = target.unwrap_or(Vec2::ZERO);
        let move_x = match target {
            Some(t) => {
                let d = t - pos;
                want_jump = k.loco.grounded() && d.y > brain.jump_to_reach && d.x.abs() < brain.aggro_range * 0.25;
                // In reach: an attack (its combo asked for swing by swing).
                let stunned = k.loco.state == platypus_physics::MoveState::Stunned;
                let near = d.x.abs() < brain.reach && d.y.abs() < brain.reach;
                if w.attacking && !swinging && w.swings_left == 0 {
                    w.attacking = false;
                    let mut rng = Rng::seeded(&[sim.world.seed(), tick, entity.to_bits(), 0xA77]);
                    let secs = brain.attack_every * (0.7 + 0.6 * (rng.next_u32() % 1000) as f32 / 1000.0);
                    w.next_attack = tick + (secs * 60.0) as u64;
                }
                if near && !stunned && brain.reach > 0.0 && tick >= w.next_attack && !w.attacking && !swinging {
                    w.swings_left = brain.combo.max(1);
                    w.attacking = true;
                }
                if w.swings_left > 0 && !stunned {
                    swings.write(crate::combat::MeleeRequest { attacker: entity, at: t });
                    // (A request during a swing queues the next move: one each.)
                    if !swinging || brain.combo > 1 {
                        w.swings_left -= 1;
                    }
                }
                if swinging || d.x.abs() <= brain.keep_distance { 0.0 } else { d.x.signum() }
            }
            None => {
                if tick >= w.until_tick {
                    let mut rng = Rng::seeded(&[sim.world.seed(), tick, entity.to_bits()]);
                    w.dir = [-1.0, 0.0, 1.0][(rng.next_u32() % 3) as usize];
                    let secs = brain.wander_every * (0.5 + (rng.next_u32() % 1000) as f32 / 1000.0);
                    w.until_tick = tick + (secs * 60.0) as u64;
                }
                w.dir * brain.wander_speed
            }
        };
        // Blocked by a wall we're walking into: jump it.
        if (move_x > 0.0 && c.wall_right) || (move_x < 0.0 && c.wall_left) {
            want_jump = k.loco.grounded();
        }
        controls.0.move_x = move_x;
        // Brains hold buttons; release after a press so the next press registers.
        controls.0.jump = want_jump && !controls.0.jump;
    }
}

/// Keeps its distance and shoots what it wields (a bow): backs off from a
/// player nearer than `near`, closes on one further than `far`; draws for
/// `draw` of the bow's full draw, stands still meanwhile, and looses at
/// where the player will be (leading it, allowing for the arrow's drop);
/// then waits `every` s (±30 %). Its aim wanders up to `wobble` degrees.
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Archer {
    pub aggro_range: f32,
    pub near: f32,
    pub far: f32,
    pub draw: f32,
    pub every: f32,
    pub wander_speed: f32,
    pub wobble: f32,
}

impl Default for Archer {
    fn default() -> Self {
        Archer { aggro_range: 260.0, near: 50.0, far: 140.0, draw: 0.9, every: 1.6, wander_speed: 0.4, wobble: 4.0 }
    }
}

#[derive(Component, Default)]
pub struct ArcherState {
    next_shot: u64,
    drawing: Option<u64>,
}

type Bowman<'a> = (Entity, &'a Archer, &'a Kinematics, &'a mut Controls, Option<&'a mut ArcherState>, &'a crate::combat::Wielding);

fn archer(
    mut commands: Commands,
    sim: Res<SimWorld>,
    weapons: Option<Res<crate::combat::Weapons>>,
    players: Query<(&Kinematics, &Team), Without<Archer>>,
    mut q: Query<Bowman>,
    mut draws: MessageWriter<crate::archery::DrawBow>,
) {
    let Some(weapons) = weapons else { return };
    let tick = sim.world.tick();
    for (e, brain, k, mut controls, state, wielding) in &mut q {
        let Some(mut st) = state else {
            commands.entity(e).insert(ArcherState::default());
            continue;
        };
        let pos = k.body.pos;
        let target = players
            .iter()
            .filter(|(_, t)| **t == Team::Player)
            .map(|(pk, _)| (pk.body.pos, pk.body.vel))
            .filter(|(p, _)| p.distance(pos) < brain.aggro_range)
            .min_by(|a, b| a.0.distance_squared(pos).total_cmp(&b.0.distance_squared(pos)));
        let bow = wielding.0.as_deref().and_then(|id| weapons.bow_index(id)).map(|i| weapons.bow(i).clone());
        let (Some((t, tv)), Some(bow)) = (target, bow) else {
            controls.0.move_x = 0.0;
            controls.0.aim = Vec2::ZERO;
            continue;
        };
        let d = t - pos;
        controls.0.aim = t;
        let stunned = k.loco.state == platypus_physics::MoveState::Stunned;
        // Drawing: keep at it, then let go.
        if let Some(since) = st.drawing {
            let held = (tick - since) as f32 / 60.0;
            if stunned || held >= bow.draw * brain.draw {
                st.drawing = None;
                let mut rng = Rng::seeded(&[sim.world.seed(), tick, e.to_bits(), 0xB0E]);
                st.next_shot = tick + (brain.every * (0.7 + 0.6 * (rng.next_u32() % 1000) as f32 / 1000.0) * 60.0) as u64;
            } else {
                // Where it'll be when the arrow gets there, and the drop.
                let speed = bow.speed.0 + (bow.speed.1 - bow.speed.0) * brain.draw.min(1.0);
                let flight = d.length() / speed.max(1.0);
                let g = weapons.arrow_def().map_or(0.0, |a| a.gravity);
                let aim = t + tv * flight + Vec2::new(0.0, 0.5 * g * flight * flight);
                // (A wobble per shot, not per tick.)
                let mut rng = Rng::seeded(&[sim.world.seed(), since, e.to_bits(), 0xA1A]);
                let off = ((rng.next_u32() % 2001) as f32 / 1000.0 - 1.0) * brain.wobble.to_radians();
                let aim = pos + Vec2::from_angle(off).rotate(aim - pos);
                draws.write(crate::archery::DrawBow { archer: e, at: aim });
            }
            controls.0.move_x = 0.0;
            continue;
        }
        let dist = d.x.abs();
        if !stunned && tick >= st.next_shot && dist <= brain.far * 1.2 && k.loco.grounded() {
            st.drawing = Some(tick);
            draws.write(crate::archery::DrawBow { archer: e, at: t });
            controls.0.move_x = 0.0;
            continue;
        }
        controls.0.move_x = if dist < brain.near { -d.x.signum() } else if dist > brain.far { d.x.signum() } else { 0.0 };
        let c = &k.loco.contacts;
        let walled = (controls.0.move_x > 0.0 && c.wall_right) || (controls.0.move_x < 0.0 && c.wall_left);
        controls.0.jump = walled && k.loco.grounded() && !controls.0.jump;
    }
}
