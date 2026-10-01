//! Conjured things (`Carrier::Wall`): walls raised out of the ground where
//! a spell is aimed. A wall of something solid (ice, stone) is left to the
//! world once it's made (ice melts, fire thaws it, a pickaxe digs it); a
//! wall of flame is kept burning for a while (`Held`), flames dying and
//! coming back, burning what walks through.

use bevy::prelude::*;
use platypus_sim::rng::Rng;
use platypus_sim::{CellPos, Kind, World};

use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// How far below where it's aimed a wall looks for ground to stand on.
const GROUND_BELOW: i32 = 120;
/// A held wall refills an open cell with this chance (/256) a tick.
const REFILL: u8 = 90;

/// Is this cell open to a conjured one (air, gas, flame)?
fn open(world: &World, p: CellPos) -> bool {
    let mats = world.materials();
    world.get(p).is_some_and(|c| c.is_air() || matches!(mats.phys(c.material).kind, Kind::Gas | Kind::Fire | Kind::Empty))
}

/// Raise a wall of `material` (`width` × `height`) on the first ground
/// below `at` (or at `at`, with none near), in the open cells; a solid
/// one not where a body (centre, half size) is. The wall's corners, if
/// the material exists.
pub fn wall(world: &mut World, material: &str, at: Vec2, width: i32, height: i32, bodies: &[(Vec2, Vec2)]) -> Option<(CellPos, CellPos)> {
    let mats = world.materials().clone();
    let m = mats.id(material)?;
    let blocks = matches!(mats.phys(m).kind, Kind::Static | Kind::Powder);
    let (x, y) = (at.x.floor() as i32, at.y.floor() as i32);
    let base = (0..GROUND_BELOW).map(|d| y - d).find(|&yy| world.is_solid(CellPos::new(x, yy - 1))).unwrap_or(y);
    let min = CellPos::new(x - width / 2, base);
    let max = CellPos::new(min.x + width - 1, base + height - 1);
    let mut rng = Rng::seeded(&[world.tick(), x as u64, base as u64, 0xC0_4A11]);
    for yy in min.y..=max.y {
        for xx in min.x..=max.x {
            let p = CellPos::new(xx, yy);
            let c = Vec2::new(xx as f32 + 0.5, yy as f32 + 0.5);
            if !open(world, p) || blocks && bodies.iter().any(|(pos, half)| ((c - *pos).abs() - *half).max_element() < 0.5) {
                continue;
            }
            world.set(p, mats.spawn(m, &mut rng));
        }
    }
    Some((min, max))
}

/// A wall kept filled (`left` seconds more): flames that die come back.
#[derive(Component)]
pub struct Held {
    pub min: CellPos,
    pub max: CellPos,
    pub material: String,
    pub left: f32,
}

/// Keep held walls filled, until their time is up.
pub fn hold(mut commands: Commands, mut sim: ResMut<SimWorld>, mut walls: Query<(Entity, &mut Held)>) {
    for (e, mut w) in &mut walls {
        w.left -= DT;
        if w.left <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let world = &mut sim.world;
        let mats = world.materials().clone();
        let Some(m) = mats.id(&w.material) else { continue };
        let mut rng = Rng::seeded(&[world.tick(), e.to_bits(), 0x4E1D]);
        for y in w.min.y..=w.max.y {
            for x in w.min.x..=w.max.x {
                let p = CellPos::new(x, y);
                if rng.chance(REFILL) && open(world, p) && world.get(p).is_some_and(|c| c.material != m) {
                    world.set(p, mats.spawn(m, &mut rng));
                }
            }
        }
    }
}
