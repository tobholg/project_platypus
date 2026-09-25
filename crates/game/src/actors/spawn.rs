//! Where creatures appear. Spawns wait until the ground under them is loaded,
//! then stand on the first surface found.
//!
//! A freshly generated chunk brings the creatures the world put there (a
//! crypt's guards), once each: an unmodified chunk is generated again when
//! it comes back into view, so the ones already spawned are remembered.
//!
//! Debug: `O` spawns an orc at the cursor.

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind, World};

use super::creature::spawn_creature;
use super::player::LocalPlayer;
use crate::camera::CameraTarget;
use super::Kinematics;
use crate::world::{ChunkLoader, FreshChunks, SimWorld};
use platypus_worldgen::Spawn;

pub struct SpawnPlugin;

pub struct PendingSpawn {
    pub kind: String,
    pub x: i32,
    /// Search for ground below this height.
    pub from_y: i32,
    pub local_player: bool,
}

#[derive(Resource, Default)]
pub struct SpawnQueue(pub Vec<PendingSpawn>);

/// World spawns already made (by where they stand).
#[derive(Resource, Default)]
pub struct Spawned(pub std::collections::HashSet<CellPos>);

/// How many enemies to scatter around the start (tunable later via RON).
const START_ENEMIES: [(&str, i32); 8] =
    [("orc", -520), ("orc", -300), ("orc", -170), ("orc", 160), ("orc", 280), ("orc", 450), ("orc", 700), ("orc", -800)];

impl Plugin for SpawnPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnQueue>()
            .init_resource::<Spawned>()
            .add_systems(Startup, queue_start)
            .add_systems(Update, (process_queue, debug_spawn, world_spawns));
    }
}

fn queue_start(sim: Res<SimWorld>, mut queue: ResMut<SpawnQueue>) {
    let s = sim.generator.spawn_point();
    queue.0.push(PendingSpawn { kind: "player".into(), x: s.x, from_y: s.y + 120, local_player: true });
    for (kind, dx) in START_ENEMIES {
        queue.0.push(PendingSpawn { kind: kind.into(), x: s.x + dx, from_y: s.y + 250, local_player: false });
    }
}

/// First standable spot scanning down from `from_y`: air above, solid below.
pub fn find_ground(world: &World, x: i32, from_y: i32, depth: i32) -> Option<i32> {
    let solid = |y| {
        world.get(CellPos::new(x, y)).map(|c| matches!(world.materials().phys(c.material).kind, Kind::Static | Kind::Powder))
    };
    let mut above_free = !solid(from_y)?;
    for y in (from_y - depth..from_y).rev() {
        let s = solid(y)?;
        if s && above_free {
            return Some(y + 1);
        }
        above_free = !s;
    }
    None
}

fn process_queue(mut commands: Commands, sim: Res<SimWorld>, mut queue: ResMut<SpawnQueue>) {
    queue.0.retain(|p| {
        let Some(ground) = find_ground(&sim.world, p.x, p.from_y, 600) else {
            return true; // not loaded yet
        };
        let feet = Vec2::new(p.x as f32 + 0.5, ground as f32);
        if p.local_player {
            // Each player keeps the world around it simulated (co-op: every player).
            spawn_creature(&mut commands, &p.kind, feet, |e| {
                e.insert((LocalPlayer, CameraTarget, ChunkLoader { half_extent: Vec2::new(420.0, 260.0) }));
            });
        } else {
            spawn_creature(&mut commands, &p.kind, feet, |_| {});
        }
        false
    });
}

fn world_spawns(mut commands: Commands, fresh: Res<FreshChunks>, mut spawned: ResMut<Spawned>, mut chests: ResMut<crate::hands::chests::Chests>) {
    for &(at, what) in &fresh.0 {
        if !spawned.0.insert(at) {
            continue;
        }
        match what {
            Spawn::Creature(kind) => spawn_creature(&mut commands, kind, Vec2::new(at.x as f32, at.y as f32), |_| {}),
            Spawn::Chest => chests.spawn_found(&mut commands, at),
        }
    }
}

fn debug_spawn(mut commands: Commands, mut actions: MessageReader<crate::dev::DevAction>, player: Query<&Kinematics, With<LocalPlayer>>) {
    for a in actions.read() {
        if let crate::dev::DevAction::SpawnOrc(at) = *a
            // From the panel: a little way off from the player.
            && let Some(at) = at.or_else(|| player.single().ok().map(|k| k.body.pos + Vec2::new(40.0, 10.0)))
        {
            spawn_creature(&mut commands, "orc", at, |_| {});
        }
    }
}
