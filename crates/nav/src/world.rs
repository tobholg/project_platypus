//! The cell world as bodies see it (`Grid`: the game's collisions go
//! through this too) and as the planner does (`NavWorld`). Unloaded chunks
//! are solid.

use glam::Vec2;
use platypus_physics::{Grid, Occupancy};
use platypus_sim::{CellPos, Kind, World};

use crate::NavWorld;

/// The cell world, as bodies and the planner see it.
pub struct WorldGrid<'a>(pub &'a World);

/// A wall behind this point (the background layer: a cave's wall, a built
/// one, a trunk) to climb on.
pub fn backed(world: &World, at: Vec2) -> bool {
    world.get_bg(CellPos::from_world(at.x, at.y)).is_some_and(|c| world.materials().phys(c.material).kind != Kind::Empty)
}

/// Grip a body gets on something slippery (ice): it slides.
const SLIPPERY_GRIP: f32 = 0.1;

impl Grid for WorldGrid<'_> {
    #[inline]
    fn grip(&self, x: i32, y: i32) -> f32 {
        match self.0.get(CellPos::new(x, y)) {
            Some(c) if self.0.materials().phys(c.material).slippery => SLIPPERY_GRIP,
            _ => 1.0,
        }
    }

    #[inline]
    fn occupancy(&self, x: i32, y: i32) -> Occupancy {
        match self.0.get(CellPos::new(x, y)) {
            None => Occupancy::Solid,
            // (Sand still falling, a stream of it: you walk through.)
            Some(c) => self.class(c),
        }
    }
}

impl WorldGrid<'_> {
    #[inline]
    fn class(&self, c: platypus_sim::Cell) -> Occupancy {
        match self.0.materials().phys(c.material) {
            p if p.platform => Occupancy::Platform,
            p => match p.kind {
                Kind::Powder if c.vy > 0 => Occupancy::Empty,
                Kind::Static | Kind::Powder => Occupancy::Solid,
                Kind::Liquid => Occupancy::Liquid,
                Kind::Empty | Kind::Gas | Kind::Fire | Kind::Plant => Occupancy::Empty,
            },
        }
    }
}

impl NavWorld for WorldGrid<'_> {
    fn solid_column(&self, x: i32, y0: i32, out: &mut [Option<(u8, bool)>]) {
        let chunk = platypus_sim::CHUNK;
        let mut i = 0;
        while i < out.len() {
            let y = y0 + i as i32;
            let p = CellPos::new(x, y);
            let run = ((chunk - y.rem_euclid(chunk)) as usize).min(out.len() - i);
            match self.0.chunk(p.chunk()) {
                None => out[i..i + run].fill(Some((255, true))),
                Some(c) => {
                    let (lx, ly) = p.local();
                    for k in 0..run {
                        let cell = c.get(lx, ly + k);
                        out[i + k] = (self.class(cell) == Occupancy::Solid).then(|| {
                            let ph = self.0.materials().phys(cell.material);
                            (ph.hardness, ph.inert)
                        });
                    }
                }
            }
            i += run;
        }
    }

    fn solid_cell(&self, x: i32, y: i32) -> Option<(u8, bool)> {
        match self.0.get(CellPos::new(x, y)) {
            None => Some((255, true)),
            Some(c) if self.class(c) == Occupancy::Solid => {
                let p = self.0.materials().phys(c.material);
                Some((p.hardness, p.inert))
            }
            Some(_) => None,
        }
    }

    fn column(&self, x: i32, y0: i32, out: &mut [Occupancy]) {
        let chunk = platypus_sim::CHUNK;
        let mut i = 0;
        while i < out.len() {
            let y = y0 + i as i32;
            let p = CellPos::new(x, y);
            let run = ((chunk - y.rem_euclid(chunk)) as usize).min(out.len() - i);
            match self.0.chunk(p.chunk()) {
                None => out[i..i + run].fill(Occupancy::Solid),
                Some(c) => {
                    let (lx, ly) = p.local();
                    for k in 0..run {
                        out[i + k] = self.class(c.get(lx, ly + k));
                    }
                }
            }
            i += run;
        }
    }

    #[inline]
    fn backed(&self, x: i32, y: i32) -> bool {
        self.0.get_bg(CellPos::new(x, y)).is_some_and(|c| self.0.materials().phys(c.material).kind != Kind::Empty)
    }
}
