//! Particles: things in flight between cells (SPEC §3.9).
//!
//! A particle carries a real `Cell` (material, shade, heat, burning state) and
//! a behaviour for when its flight ends. They live in a plain list, step once
//! per tick after the cells, and move one cell at a time so nothing passes
//! through walls. Randomness comes from the world's seeded generators.

use crate::cell::{Cell, flags};
use crate::coords::CellPos;
use crate::material::{Kind, MaterialTable};

/// What happens when a particle's flight ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Landing {
    /// Becomes its cell where it stops (debris, drops of liquid, blood).
    /// Solids land `LOOSE`, so they pile as rubble.
    Settle,
    /// Purely visual: dust, sparks.
    Vanish,
    /// A burning speck: sets what it lands on alight if that can burn.
    Ember,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    /// World position in cells.
    pub pos: [f32; 2],
    /// Cells per tick.
    pub vel: [f32; 2],
    pub cell: Cell,
    /// Ticks left before it lands (or vanishes) wherever it is.
    pub life: u16,
    pub landing: Landing,
    /// Multiplies gravity: 1 for debris, negative for embers that rise.
    pub gravity: f32,
}

/// Gravity in cells per tick² (≈ 900 cells/s² at 60 Hz).
pub const GRAVITY: f32 = 0.25;
/// Velocity kept per tick (air drag).
const DRAG: f32 = 0.985;
/// Fastest a particle may travel per tick.
const MAX_SPEED: f32 = 8.0;
/// Most particles alive at once; the oldest are dropped beyond this.
pub const MAX_PARTICLES: usize = 30_000;

impl Particle {
    pub fn new(pos: [f32; 2], vel: [f32; 2], cell: Cell, life: u16, landing: Landing) -> Self {
        Particle { pos, vel, cell, life, landing, gravity: 1.0 }
    }

    pub fn cell_pos(&self) -> CellPos {
        CellPos::from_world(self.pos[0], self.pos[1])
    }
}

/// What the stepper needs from the world, so this module stays testable.
pub(crate) trait ParticleWorld {
    fn get(&self, p: CellPos) -> Option<Cell>;
    fn set(&mut self, p: CellPos, cell: Cell) -> bool;
    fn mats(&self) -> &MaterialTable;
    /// Set a flammable cell alight (same rules as everywhere else).
    fn ignite_at(&mut self, p: CellPos);
}

/// Can a particle fly through this cell?
fn open(mats: &MaterialTable, c: Cell) -> bool {
    c.is_air() || matches!(mats.phys(c.material).kind, Kind::Gas | Kind::Fire)
}

/// Advance every particle one tick. Landed and vanished particles are removed.
pub(crate) fn step(particles: &mut Vec<Particle>, world: &mut impl ParticleWorld) {
    if particles.len() > MAX_PARTICLES {
        let excess = particles.len() - MAX_PARTICLES;
        particles.drain(..excess);
    }
    let mut i = 0;
    while i < particles.len() {
        if step_one(&mut particles[i], world) {
            i += 1;
        } else {
            particles.swap_remove(i);
        }
    }
}

/// Returns false when the particle is done.
fn step_one(p: &mut Particle, world: &mut impl ParticleWorld) -> bool {
    p.vel[1] -= GRAVITY * p.gravity;
    p.vel[0] *= DRAG;
    p.vel[1] *= DRAG;
    let speed = p.vel[0].abs().max(p.vel[1].abs());
    if speed > MAX_SPEED {
        p.vel[0] *= MAX_SPEED / speed;
        p.vel[1] *= MAX_SPEED / speed;
    }

    // March one cell at a time so nothing tunnels.
    let steps = p.vel[0].abs().max(p.vel[1].abs()).ceil().max(1.0) as i32;
    let (dx, dy) = (p.vel[0] / steps as f32, p.vel[1] / steps as f32);
    for _ in 0..steps {
        let next = [p.pos[0] + dx, p.pos[1] + dy];
        let at = CellPos::from_world(next[0], next[1]);
        match world.get(at) {
            None => return false, // left the loaded world
            Some(c) if open(world.mats(), c) => p.pos = next,
            Some(_) => {
                land(p, at, world);
                return false;
            }
        }
    }

    p.life = p.life.saturating_sub(1);
    if p.life == 0 {
        let here = p.cell_pos();
        land(p, here, world);
        return false;
    }
    true
}

/// The flight ended at the particle's position, against `hit` (which may be
/// the cell it is in, when its life ran out mid-air).
fn land(p: &Particle, hit: CellPos, world: &mut impl ParticleWorld) {
    let here = p.cell_pos();
    match p.landing {
        Landing::Vanish => {}
        Landing::Ember => {
            if let Some(c) = world.get(hit)
                && !c.is_air()
                && world.mats().phys(c.material).flammability > 0
            {
                world.ignite_at(hit);
            }
        }
        Landing::Settle => {
            // Into the free cell it stopped in, or the first free one just above.
            for up in 0..4 {
                let spot = here.offset(0, up);
                if world.get(spot).is_some_and(|c| open(world.mats(), c)) {
                    let mut cell = p.cell;
                    if world.mats().phys(cell.material).kind == Kind::Static {
                        cell.flags |= flags::LOOSE;
                    }
                    // Keep falling at the speed it arrived with (rules: 1 + vy/4 cells/tick).
                    cell.vy = (-p.vel[1] * 4.0).clamp(0.0, 28.0) as i8;
                    world.set(spot, cell);
                    return;
                }
            }
        }
    }
}
