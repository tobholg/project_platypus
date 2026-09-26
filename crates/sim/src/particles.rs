//! Particles: things in flight between cells (SPEC §3.9).
//!
//! A particle carries a real `Cell` (material, shade, heat, burning state) and
//! a behaviour for when its flight ends. They live in a plain list, step once
//! per tick after the cells, and move one cell at a time so nothing passes
//! through walls. Randomness comes from the world's seeded generators.
//!
//! A tick flies them all at once (in parallel: flight only reads the world,
//! `ParticleView`), then applies what they did (land, douse a fire, light
//! the background: `Effect`), one after another in list order, so the
//! result is the same on any number of threads.

use crate::cell::{Cell, flags};
use crate::coords::CellPos;
use crate::material::{Kind, MaterialTable};
use rayon::prelude::*;

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
    /// A raindrop: puts out flames and burning cells it passes or hits;
    /// one in `RAIN_KEEP` lands as a cell (the rest are just rain to look at).
    Rain,
    /// A snowflake: drifts, melts into a raindrop in air above 1 °C; one in
    /// `SNOW_KEEP` settles as its cell.
    Snow,
}

/// One raindrop in this many lands as water: enough for puddles and rising
/// lakes in a downpour (~3 cells a minute), not a flood.
pub const RAIN_KEEP: u64 = 150;
/// One flake in this many settles.
pub const SNOW_KEEP: u64 = 10;

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

/// What applying particles' effects needs from the world (one at a time).
pub(crate) trait ParticleWorld {
    fn get(&self, p: CellPos) -> Option<Cell>;
    fn set(&mut self, p: CellPos, cell: Cell) -> bool;
    /// A new particle (flying from the next tick).
    fn emit(&mut self, p: Particle);
    fn mats(&self) -> &MaterialTable;
    /// Set a flammable cell alight (same rules as everywhere else).
    fn ignite_at(&mut self, p: CellPos);
    /// Set the background cell here alight (an ember caught it).
    fn light_bg(&mut self, p: CellPos);
    /// Water arriving at `p`: flames there go out, burning cells (in front
    /// or behind) stop burning, or, burning hotter than a drop can put out,
    /// lose some heat as it boils off. True if the water was used up.
    fn douse(&mut self, p: CellPos) -> bool;
    /// `douse` a drop's 3-cell strip centred on `p`; true if it was used up.
    fn douse_strip(&mut self, p: CellPos) -> bool {
        (-1..=1).any(|dx| self.douse(p.offset(dx, 0)))
    }
}

/// How strongly wind pushes each kind of particle (cells/tick² at full wind).
fn wind_push(landing: Landing) -> f32 {
    match landing {
        Landing::Ember | Landing::Vanish | Landing::Snow => 0.035,
        Landing::Rain => 0.012,
        Landing::Settle => 0.004,
    }
}

/// Can a particle fly through this cell?
fn open(mats: &MaterialTable, c: Cell) -> bool {
    c.is_air() || matches!(mats.phys(c.material).kind, Kind::Gas | Kind::Fire | Kind::Plant)
}

/// What flight needs to read of the world (shared by every thread).
pub(crate) trait ParticleView: Sync {
    fn get(&self, p: CellPos) -> Option<Cell>;
    fn mats(&self) -> &MaterialTable;
    fn wind(&self) -> f32;
    fn ambient(&self, x: i32, y: i32) -> i32;
    fn water(&self) -> Option<Cell>;
    /// Rain passing here: is there fire to douse in the strip (the cell and
    /// its neighbours across), and would the drop be spent on it (burning
    /// background hotter than rain can put out)?
    fn rain_at(&self, p: CellPos) -> (bool, bool);
    /// An ember over the background here: does it touch it (and is spent),
    /// and does that light it?
    fn ember_at(&self, p: CellPos, catch: u8) -> (bool, bool);
}

/// What a particle does to the world this tick, applied after flight.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Effect {
    /// Its flight ended (at `hit`: the cell it struck, or where it was).
    Land(Particle, CellPos),
    /// Rain over fire: douse the strip.
    Douse(CellPos),
    /// An ember caught the background here.
    LightBg(CellPos),
}

/// A particle's tick: whether it's still flying, and what it did.
struct Flight {
    alive: bool,
    effect: Option<Effect>,
    douse: Option<CellPos>,
}

/// Below this many, flying them in parallel isn't worth it.
const PARALLEL: usize = 1_024;

/// Fly every particle a tick (in parallel), dropping those whose flight
/// ended; what they did, in list order.
pub(crate) fn fly(particles: &mut Vec<Particle>, view: &impl ParticleView) -> Vec<Effect> {
    if particles.len() > MAX_PARTICLES {
        // Oldest first: they're nearest the end of their life anyway.
        let excess = particles.len() - MAX_PARTICLES;
        particles.drain(..excess);
    }
    let flights: Vec<Flight> = if particles.len() >= PARALLEL {
        particles.par_iter_mut().map(|p| fly_one(p, view)).collect()
    } else {
        particles.iter_mut().map(|p| fly_one(p, view)).collect()
    };
    let mut effects = Vec::new();
    for f in &flights {
        effects.extend(f.douse.map(Effect::Douse));
        effects.extend(f.effect);
    }
    let mut k = 0;
    particles.retain(|_| {
        k += 1;
        flights[k - 1].alive
    });
    effects
}

/// Apply what the particles did, in order.
pub(crate) fn apply(effects: &[Effect], world: &mut impl ParticleWorld) {
    for e in effects {
        match *e {
            Effect::Land(p, hit) => land(&p, hit, world),
            Effect::Douse(at) => {
                world.douse_strip(at);
            }
            Effect::LightBg(at) => world.light_bg(at),
        }
    }
}

/// An ember's chance (out of 256) to set what it touches alight, falling as it cools.
fn ember_catch(life: u16) -> u8 {
    (EMBER_CATCH as u32 * life.min(EMBER_HOT) as u32 / EMBER_HOT as u32) as u8
}

/// Chance (out of 256) that a fresh ember touching something flammable lights it.
const EMBER_CATCH: u8 = 64;
/// Ticks an ember stays at full heat before its chance to light starts falling.
const EMBER_HOT: u16 = 80;
/// Embers wink out at random: one in this many each tick.
const EMBER_FADE: u64 = 60;

/// Advance one particle a tick, reading the world only.
fn fly_one(p: &mut Particle, world: &impl ParticleView) -> Flight {
    let dead = |effect: Option<Effect>, douse: Option<CellPos>| Flight { alive: false, effect, douse };
    let mut douse = None;
    if p.landing == Landing::Ember && crate::rng::hash(&[p.pos[0].to_bits() as u64, p.pos[1].to_bits() as u64, p.life as u64]).is_multiple_of(EMBER_FADE) {
        return dead(None, None);
    }
    p.vel[1] -= GRAVITY * p.gravity;
    p.vel[0] += world.wind() * wind_push(p.landing);
    p.vel[0] *= DRAG;
    p.vel[1] *= DRAG;
    let speed = p.vel[0].abs().max(p.vel[1].abs());
    if speed > MAX_SPEED {
        p.vel[0] *= MAX_SPEED / speed;
        p.vel[1] *= MAX_SPEED / speed;
    }

    // Move a cell at a time so fast particles don't tunnel through walls.
    let steps = p.vel[0].abs().max(p.vel[1].abs()).ceil().max(1.0) as i32;
    let (dx, dy) = (p.vel[0] / steps as f32, p.vel[1] / steps as f32);
    for _ in 0..steps {
        let next = [p.pos[0] + dx, p.pos[1] + dy];
        let at = CellPos::from_world(next[0], next[1]);
        match world.get(at) {
            None => return dead(None, douse), // left the loaded world
            Some(c) if open(world.mats(), c) => {
                p.pos = next;
                match p.landing {
                    // An ember passing over flammable background may light
                    // it; either way it's spent on it.
                    Landing::Ember => {
                        let (spent, lights) = world.ember_at(at, ember_catch(p.life));
                        if spent {
                            return dead(lights.then_some(Effect::LightBg(at)), douse);
                        }
                    }
                    // Rain puts out what it falls through (a drop is spent on
                    // a fierce bg fire; a small one only dampens it).
                    Landing::Rain => {
                        let (any, spent) = world.rain_at(at);
                        if any && douse.is_none() {
                            douse = Some(at);
                        }
                        if spent {
                            return dead(None, douse);
                        }
                    }
                    // Snow melts falling through air above freezing.
                    Landing::Snow if world.ambient(at.x, at.y) > 1 => {
                        if let Some(water) = world.water() {
                            p.landing = Landing::Rain;
                            p.cell = water;
                            p.gravity = 1.0;
                        }
                    }
                    _ => {}
                }
            }
            Some(_) => return dead(Some(Effect::Land(*p, at)), douse),
        }
    }

    p.life = p.life.saturating_sub(1);
    if p.life == 0 {
        let here = p.cell_pos();
        return dead(Some(Effect::Land(*p, here)), douse);
    }
    Flight { alive: true, effect: None, douse }
}

/// The flight ended at the particle's position, against `hit` (which may be
/// the cell it is in, when its life ran out mid-air).
fn land(p: &Particle, hit: CellPos, world: &mut impl ParticleWorld) {
    let here = p.cell_pos();
    // Which drops and flakes land as cells: from where and when they fell, so
    // it's the same on every machine.
    let keep = |one_in: u64| {
        crate::rng::hash(&[p.pos[0].to_bits() as u64, p.pos[1].to_bits() as u64, p.life as u64]).is_multiple_of(one_in)
    };
    match p.landing {
        Landing::Vanish => {}
        Landing::Rain => {
            world.douse(hit);
            if keep(RAIN_KEEP) {
                settle(p, here, world);
            }
        }
        Landing::Snow => {
            world.douse(hit);
            if keep(SNOW_KEEP) {
                settle(p, here, world);
            }
        }
        Landing::Ember => {
            if let Some(c) = world.get(hit)
                && !c.is_air()
                && world.mats().phys(c.material).flammability > 0
                && (crate::rng::hash(&[p.pos[0].to_bits() as u64, p.pos[1].to_bits() as u64, 0xE3]) % 256) < ember_catch(p.life) as u64
            {
                world.ignite_at(hit);
            }
        }
        Landing::Settle => settle(p, here, world),
    }
}

/// Into the free cell it stopped in, or the first free one just above; or,
/// stopped in a liquid (a splash landing under water, a pool flowing over
/// it), into that cell, the liquid it displaced thrown up from the surface
/// above (a particle, so a lot of them spread instead of stacking), so
/// nothing is lost.
fn settle(p: &Particle, here: CellPos, world: &mut impl ParticleWorld) {
    let mut cell = p.cell;
    if world.mats().phys(cell.material).kind == Kind::Static {
        cell.flags |= flags::LOOSE;
    }
    // Keep falling at the speed it arrived with (rules: 1 + vy/4 cells/tick).
    cell.vy = (-p.vel[1] * 4.0).clamp(0.0, 28.0) as i8;
    for up in 0..4 {
        let spot = here.offset(0, up);
        if world.get(spot).is_some_and(|c| open(world.mats(), c)) {
            world.set(spot, cell);
            return;
        }
    }
    let liquid = |c: Cell| world.mats().phys(c.material).kind == Kind::Liquid;
    let Some(displaced) = world.get(here).filter(|&c| liquid(c)) else { return };
    // The top of this column of liquid, or of one beside it (a whole splash
    // lands in one cell; one column fills up to a ceiling).
    for dx in (0..=DISPLACE_SIDE).flat_map(|d| [d, -d]).skip(1) {
        let column = here.offset(dx, 0);
        if !world.get(column).is_some_and(liquid) {
            continue;
        }
        for up in 1..DISPLACE_REACH {
            let spot = column.offset(0, up);
            match world.get(spot) {
                Some(c) if open(world.mats(), c) => {
                    world.set(here, cell);
                    let h = crate::rng::hash(&[here.x as u64, here.y as u64, p.life as u64, dx as u64]);
                    let side = (h % 1000) as f32 / 1000.0 - 0.5;
                    world.emit(Particle::new([spot.x as f32 + 0.5, spot.y as f32 + 0.5], [side * 1.4, 0.5], displaced, 120, Landing::Settle));
                    return;
                }
                Some(c) if liquid(c) => continue,
                _ => break,
            }
        }
    }
}

/// How far up through a liquid a landing cell looks for the surface to put
/// what it displaced, and how many columns either side it tries.
const DISPLACE_REACH: i32 = 96;
const DISPLACE_SIDE: i32 = 12;
