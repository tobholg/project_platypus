//! Where creatures appear. Spawns wait until the ground under them is loaded,
//! then stand on the first surface found.
//!
//! A freshly generated chunk brings the creatures the world put there (a
//! crypt's guards), once each: an unmodified chunk is generated again when
//! it comes back into view, so the ones already spawned are remembered.
//!
//! Debug: `O` spawns the picked kind (`SpawnKind`: an orc, unless the arena
//! panel picked another) at the cursor.

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind, World};

use super::def::spawn_creature;
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

/// What `O` spawns: a creature, or a pack of them (`assets/data/packs.ron`)
/// (the arena panel picks it).
#[derive(Resource)]
pub struct SpawnKind(pub String);

impl Default for SpawnKind {
    fn default() -> Self {
        SpawnKind("warband".into())
    }
}

/// Every pack by name: its members and how many of each.
pub fn packs() -> std::collections::BTreeMap<String, Vec<(String, u32)>> {
    crate::data::load_ron(&crate::data::data_path("packs.ron")).unwrap_or_else(|e| {
        warn!("{e}");
        Default::default()
    })
}

/// Cells between a pack's members as they stand in line.
const PACK_GAP: f32 = 21.0;

#[derive(Resource, Default)]
pub struct SpawnQueue(pub Vec<PendingSpawn>);

/// World spawns already made (by where they stand).
#[derive(Resource, Default)]
pub struct Spawned(pub std::collections::HashSet<CellPos>);

/// The orcs about the start: two camps out past the start valley's plain
/// (3000 cells either side of the start), one each way, so the valley's
/// own trouble is slimes by day and the dead by night (`life.ron`).
const START_ENEMIES: [(&str, i32); 6] = [("orc", -3500), ("orc_archer", -3440), ("orc", -3380), ("orc", 3380), ("orc_archer", 3440), ("orc", 3500)];

impl Plugin for SpawnPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnQueue>()
            .init_resource::<Spawned>()
            .init_resource::<SpawnKind>()
            .add_systems(Startup, queue_start)
            .add_systems(Update, (process_queue, debug_spawn, world_spawns, old_candles));
    }
}

fn queue_start(sim: Res<SimWorld>, mut queue: ResMut<SpawnQueue>) {
    let s = sim.generator.spawn_point();
    queue.0.push(PendingSpawn { kind: "player".into(), x: s.x, from_y: s.y + 180, local_player: true });
    queue_start_enemies(&sim, &mut queue);
}

/// The enemies about the start (a new world, a reset).
pub fn queue_start_enemies(sim: &SimWorld, queue: &mut SpawnQueue) {
    if !sim.generator.wild() {
        return;
    }
    let s = sim.generator.spawn_point();
    for (kind, dx) in START_ENEMIES {
        queue.0.push(PendingSpawn { kind: kind.into(), x: s.x + dx, from_y: s.y + 375, local_player: false });
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

fn process_queue(mut commands: Commands, sim: Res<SimWorld>, saved: Option<Res<crate::save::PendingPlayer>>, mut queue: ResMut<SpawnQueue>) {
    queue.0.retain(|p| {
        // A saved player is put back where it was (`save::apply_player`), so
        // it needn't find ground here first: it mightn't (a shaft dug down
        // from the spawn, open past the loaded world, and the player never
        // came).
        let ground = match find_ground(&sim.world, p.x, p.from_y, 900) {
            Some(g) => g,
            None if p.local_player && saved.is_some() => p.from_y,
            None => return true, // not loaded yet
        };
        let feet = Vec2::new(p.x as f32 + 0.5, ground as f32);
        if p.local_player {
            // Each player keeps the world around it simulated (co-op: every player).
            spawn_creature(&mut commands, &p.kind, feet, |e| {
                e.insert((LocalPlayer, CameraTarget, ChunkLoader { half_extent: Vec2::new(630.0, 390.0) }, crate::gold::Gold::default()));
            });
        } else {
            spawn_creature(&mut commands, &p.kind, feet, |_| {});
        }
        false
    });
}

/// A chest this deep or deeper (cells under the surface), or in a castle
/// (within this of its middle), is a mimic one time in so many (by where
/// it is: the same every time).
const MIMIC_DEPTH: i32 = 150;
const MIMIC_CASTLE: i32 = 400;
const MIMIC_ONE_IN: u64 = 6;

fn mimic_at(sim: &SimWorld, at: CellPos) -> bool {
    if !platypus_sim::rng::hash(&[sim.world.seed(), 0x313C, at.x as u64, at.y as u64]).is_multiple_of(MIMIC_ONE_IN) {
        return false;
    }
    let deep = sim.generator.surface_hint(at.x).is_some_and(|s| s - at.y >= MIMIC_DEPTH);
    deep || sim.generator.landmarks().iter().any(|(p, n)| n == "castle" && (p.x - at.x).abs() < MIMIC_CASTLE && (p.y - at.y).abs() < MIMIC_CASTLE)
}

fn world_spawns(
    mut commands: Commands,
    fresh: Res<FreshChunks>,
    mut spawned: ResMut<Spawned>,
    mut chests: ResMut<crate::hands::chests::Chests>,
    mut clock: ResMut<crate::clock::WorldClock>,
    crafting: Res<crate::craft::Crafting>,
    sim: Res<SimWorld>,
) {
    for &(at, what) in &fresh.0 {
        if !spawned.0.insert(at) {
            continue;
        }
        // (Known by where it was made, as stored; put where it's seen:
        // round a wrapping world, the copy nearest the players.)
        let seen = CellPos::new(sim.world.near(at.x), at.y);
        let feet = Vec2::new(seen.x as f32, seen.y as f32);
        match what {
            Spawn::Creature(kind) | Spawn::Prop(kind) => spawn_creature(&mut commands, kind, feet, |_| {}),
            // (Deep down, a chest is now and then a mimic.)
            Spawn::Chest if mimic_at(&sim, at) => spawn_creature(&mut commands, "mimic", feet, |_| {}),
            Spawn::Chest => chests.spawn_found(&mut commands, at, seen),
            // (A village's anvil, furnace, workbench.)
            Spawn::Station(id) => match crafting.station(id) {
                Some(kind) => crafting.spawn(&mut commands, kind, feet, false),
                None => warn!("no crafting station `{id}`"),
            },
            // (The world clock refills its lair: see `clock::lairs`.)
            Spawn::Keeper(kind) => {
                spawn_creature(&mut commands, kind, feet, move |e| {
                    e.insert(crate::clock::Keeps((at.x, at.y)));
                });
                let seen = clock.now;
                clock.keepers.insert((at.x, at.y), crate::clock::Keeper { kind: kind.to_string(), seen });
            }
        }
    }
}

/// Candles were cells once (a castle's, a crypt's: solid, barely glowing,
/// in the way); now they're things (`candle` props, spawned with the
/// world). A chunk from an older save, as it loads: its candle cells
/// taken away, a lit candle put where each stood. (Looked at once a chunk,
/// each time it's loaded: none left after, it costs a glance.)
fn old_candles(mut commands: Commands, mut sim: ResMut<SimWorld>, mut seen: Local<std::collections::HashSet<platypus_sim::ChunkPos>>, mut frame: Local<u32>) {
    *frame = frame.wrapping_add(1);
    if !frame.is_multiple_of(10) {
        return;
    }
    let Some(candle) = sim.materials().id("candle") else { return };
    let loaded: std::collections::HashSet<platypus_sim::ChunkPos> = sim.world.chunks().map(|c| c.pos).collect();
    let new: Vec<platypus_sim::ChunkPos> = loaded.iter().filter(|p| !seen.contains(p)).copied().collect();
    *seen = loaded;
    let chunk = platypus_sim::CHUNK;
    for pos in new {
        // (Looked at where it's seen: round a wrapping world, the copy
        // nearest the players.)
        let at = sim.world.near_chunk(pos);
        let (x0, y0) = (at.x * chunk, at.y * chunk);
        let is = |w: &World, x: i32, y: i32| w.get(CellPos::new(x, y)).is_some_and(|c| c.material == candle);
        // (A candle standing on the chunk below, not loaded yet: its foot's
        // there; this chunk waits for it, looked at again next time.)
        if (x0..x0 + chunk).any(|x| is(&sim.world, x, y0) && sim.world.get(CellPos::new(x, y0 - 1)).is_none()) {
            seen.remove(&pos);
            continue;
        }
        let mut feet: Vec<(i32, i32)> = Vec::new();
        for y in y0..y0 + chunk {
            for x in x0..x0 + chunk {
                // (A candle's bottom left: candle here, none under it or left of it.)
                if is(&sim.world, x, y) && !is(&sim.world, x, y - 1) && !is(&sim.world, x - 1, y) {
                    feet.push((x, y));
                }
            }
        }
        if !feet.is_empty() {
            info!("candles: {} in chunk {pos:?} made things (from an older save's cells)", feet.len());
        }
        for (x, y) in feet {
            for dy in 0..6 {
                for dx in 0..2 {
                    if is(&sim.world, x + dx, y + dy) {
                        sim.world.set(CellPos::new(x + dx, y + dy), platypus_sim::Cell::AIR);
                    }
                }
            }
            spawn_creature(&mut commands, "candle", Vec2::new(x as f32 + 1.0, y as f32), |_| {});
        }
    }
}

fn debug_spawn(
    mut commands: Commands,
    mut actions: MessageReader<crate::dev::DevAction>,
    kind: Res<SpawnKind>,
    mut queue: ResMut<SpawnQueue>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut tape: ResMut<crate::replay::Tape>,
) {
    // Played back: what was put down then, as it was (asks now ignored).
    let asked: Vec<(String, Vec2)> = if tape.playing() {
        actions.clear();
        tape.spawns_now()
    } else {
        actions
            .read()
            .filter_map(|a| if let crate::dev::DevAction::Spawn(at) = *a { Some(at) } else { None })
            // From the panel: a way off from the player.
            .filter_map(|at| at.or_else(|| player.single().ok().map(|k| k.body.pos + Vec2::new(90.0, 15.0))))
            .map(|at| (kind.0.clone(), at))
            .collect()
    };
    for (kind, at) in asked {
        if tape.recording() {
            tape.note_spawn(&kind, at);
        }
        let Some(pack) = packs().remove(&kind) else {
            spawn_creature(&mut commands, &kind, at, |_| {});
            continue;
        };
        // In a line across the cursor, each on the ground under its place.
        let members: Vec<&String> = pack.iter().flat_map(|(k, n)| std::iter::repeat_n(k, *n as usize)).collect();
        let half = (members.len() as f32 - 1.0) / 2.0;
        for (i, k) in members.into_iter().enumerate() {
            let x = at.x + (i as f32 - half) * PACK_GAP;
            queue.0.push(PendingSpawn { kind: k.clone(), x: x as i32, from_y: at.y as i32 + 60, local_player: false });
        }
    }
}
