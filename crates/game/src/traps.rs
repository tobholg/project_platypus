//! Traps (DESIGN §13.1): worldgen hangs a boulder from a tunnel's ceiling on
//! a rope, over a tripwire across the floor or a pressure plate in it (all
//! materials: they're saved with the world and nothing else keeps them). A
//! creature touching the wire, or standing on the plate, springs it: the
//! wire snaps and the rope above (within `REACH`) is cut, and the boulder,
//! held by nothing now, falls whole. (Cut the rope yourself, or burn it,
//! and it falls too; that's the sim's.)

use bevy::prelude::*;
use platypus_sim::{Cell, CellPos, MaterialId};

use crate::creatures::Kinematics;
use crate::world::{SimWorld, TickSet};

pub struct TrapsPlugin;

impl Plugin for TrapsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, spring.after(TickSet::Bodies));
    }
}

/// How far from what springs it the rope may be: across, and up.
const REACH: i32 = 36;
const UP: i32 = 90;

/// A creature on a wire or a plate springs what's above it.
fn spring(mut sim: ResMut<SimWorld>, creatures: Query<(&Kinematics, Option<&crate::creatures::Creature>)>, mut sounds: MessageWriter<crate::sound::PlaySound>) {
    let mats = sim.materials().clone();
    let (Some(wire), Some(plate), Some(rope)) = (mats.id("tripwire"), mats.id("pressure_plate"), mats.id("rope")) else { return };
    let mut sprung: Vec<(CellPos, String)> = Vec::new();
    for (k, who) in &creatures {
        let (lo, hi) = k.body.cells_at(k.body.pos);
        let is = |p: CellPos, m: MaterialId| sim.world.get(p).is_some_and(|c| c.material == m);
        let on_wire = (lo.y..=hi.y).flat_map(|y| (lo.x..=hi.x).map(move |x| CellPos::new(x, y))).find(|&p| is(p, wire));
        // (Standing, a body's bottom sits a hair over the cell it's on.)
        let under = (k.body.pos.y - k.body.half.y - 0.5).floor() as i32;
        let on_plate = (lo.x..=hi.x).map(|x| CellPos::new(x, under)).find(|&p| is(p, plate));
        if let Some(p) = on_wire.or(on_plate) {
            sprung.push((p, who.map_or_else(|| "something".into(), |c| c.kind.clone())));
        }
    }
    for (at, who) in sprung {
        // The wire snaps (all of it along the floor there).
        for dx in -REACH..=REACH {
            let p = at.offset(dx, 0);
            if sim.world.get(p).is_some_and(|c| c.material == wire) {
                sim.world.set(p, Cell::AIR);
            }
        }
        // The rope above is cut; what it held falls.
        let mut cut = None;
        for dy in 0..UP {
            for dx in -REACH..=REACH {
                let p = at.offset(dx, dy);
                if sim.world.get(p).is_some_and(|c| c.material == rope) {
                    sim.world.set(p, Cell::AIR);
                    cut = Some(p);
                }
            }
        }
        if let Some(p) = cut {
            sim.world.loosen_fragments(p, 18);
            sounds.write(crate::sound::PlaySound::at("snap", Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5)));
            info!("traps: sprung at ({}, {}) by a {who}: the rope at ({}, {}) cut", at.x, at.y, p.x, p.y);
        }
    }
}
