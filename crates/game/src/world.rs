//! The cell world inside Bevy: owns the simulation, steps it on the fixed
//! tick, streams chunks around every `ChunkLoader`, and hot-reloads materials.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use platypus_sim::store::ChunkStore;
use platypus_sim::{CHUNK, ChunkPos, MaterialTable, StepStats, Weather, World, WorldEdit};
use platypus_worldgen::ChunkGenerator;
use rayon::prelude::*;

use crate::data::Watched;
use crate::fx::{Explosion, Lightning};

/// Ticks per second of the simulation and all gameplay (SPEC §3.3).
pub const TICK_HZ: f64 = 60.0;

pub struct WorldPlugin {
    pub seed: u64,
    pub materials: Arc<MaterialTable>,
    /// Watched for hot reload.
    pub materials_path: PathBuf,
    pub generator: Arc<dyn ChunkGenerator>,
}

/// Runs in `FixedUpdate`, in this order. Gameplay plugins put their systems
/// in the matching set instead of ordering against each other's internals.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TickSet {
    /// Read input / AI, produce intents and `WorldEdit`s.
    Intent,
    /// Move bodies against the grid.
    Bodies,
    /// Apply queued edits and step the cells.
    Cells,
}

#[derive(Resource)]
pub struct SimWorld {
    pub world: World,
    pub generator: Arc<dyn ChunkGenerator>,
    pub store: ChunkStore,
}

impl SimWorld {
    pub fn materials(&self) -> &Arc<MaterialTable> {
        self.world.materials()
    }

    pub fn queue(&mut self, edit: WorldEdit) {
        self.world.queue_edit(edit);
    }
}

/// Keeps the chunks under a rectangle loaded (camera, each co-op player).
#[derive(Component, Clone, Copy, Debug)]
pub struct ChunkLoader {
    /// Half-size of the area that must be loaded, in cells.
    pub half_extent: Vec2,
}

/// Timings for the debug HUD and the scenario runner.
#[derive(Resource, Default)]
pub struct SimMetrics {
    pub last: StepStats,
    pub tick_time: Duration,
    /// Exponential moving average of `tick_time`.
    pub tick_time_avg: Duration,
    pub loaded_this_frame: usize,
    pub stream_time: Duration,
}

#[derive(Resource)]
struct MaterialsSource(Watched);

/// Chunks beyond the loaded area that stay loaded, so turning around is free.
const UNLOAD_HYSTERESIS: i32 = 2;
/// Cap on chunks generated per frame, so a teleport doesn't stall a frame.
const MAX_LOADS_PER_FRAME: usize = 48;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        let (mats, generator) = (self.materials.clone(), self.generator.clone());
        let mut world = World::new(self.seed, mats);
        world.set_climate(generator.climate());
        if let Some((y0, height)) = generator.cloud_band() {
            let (lo, hi) = generator.bounds();
            let width = (hi.x - lo.x + 1) * CHUNK;
            world.set_weather(Weather::new(self.seed, width, y0, height));
        }
        app.insert_resource(Time::<Fixed>::from_hz(TICK_HZ))
            .insert_resource(SimWorld { world, generator, store: ChunkStore::default() })
            .insert_resource(MaterialsSource(Watched::new(self.materials_path.clone())))
            .init_resource::<SimMetrics>()
            .configure_sets(FixedUpdate, (TickSet::Intent, TickSet::Bodies, TickSet::Cells).chain())
            .add_systems(FixedUpdate, step_cells.in_set(TickSet::Cells))
            .add_systems(PreUpdate, stream_chunks)
            .add_systems(Update, hot_reload_materials);
    }
}

fn step_cells(mut sim: ResMut<SimWorld>, mut metrics: ResMut<SimMetrics>, mut fx: MessageWriter<Explosion>, mut bolts: MessageWriter<Lightning>) {
    let t = Instant::now();
    metrics.last = sim.world.step();
    for &s in &metrics.last.lightning {
        bolts.write(Lightning(s));
    }
    for &(p, radius) in &metrics.last.detonated {
        fx.write(Explosion { at: Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5), radius: radius as f32 });
    }
    metrics.tick_time = t.elapsed();
    #[cfg(feature = "spikes")]
    if metrics.tick_time.as_secs_f32() > 0.004 {
        let phases: Vec<String> = platypus_sim::PHASES.iter().zip(metrics.last.phases).map(|(n, d)| format!("{n} {:.1}", d.as_secs_f32() * 1000.0)).collect();
        info!("slow tick {:.1} ms: {} (active {}, particles {})", metrics.tick_time.as_secs_f32() * 1000.0, phases.join(", "), metrics.last.active_chunks, sim.world.particles().len());
    }
    metrics.tick_time_avg = metrics.tick_time_avg.mul_f64(0.95) + metrics.tick_time.mul_f64(0.05);
}

/// Load what any loader needs, unload what nobody needs (writing modified
/// chunks to the store). Co-op: the loaded set is the union over players.
fn stream_chunks(
    mut sim: ResMut<SimWorld>,
    mut metrics: ResMut<SimMetrics>,
    loaders: Query<(&GlobalTransform, &ChunkLoader)>,
) {
    let t = Instant::now();
    let rects: Vec<(IVec2, IVec2)> = loaders
        .iter()
        .map(|(tf, l)| {
            let c = tf.translation().truncate();
            let lo = ((c - l.half_extent) / CHUNK as f32).floor().as_ivec2() - IVec2::ONE;
            let hi = ((c + l.half_extent) / CHUNK as f32).floor().as_ivec2() + IVec2::ONE;
            (lo, hi)
        })
        .collect();
    if rects.is_empty() {
        return;
    }
    let wanted = |p: ChunkPos, pad: i32| {
        rects.iter().any(|(lo, hi)| p.x >= lo.x - pad && p.y >= lo.y - pad && p.x <= hi.x + pad && p.y <= hi.y + pad)
    };

    // Unload.
    let stale: Vec<ChunkPos> =
        sim.world.chunks().map(|c| c.pos).filter(|&p| !wanted(p, UNLOAD_HYSTERESIS)).collect();
    for pos in stale {
        if let Some(chunk) = sim.world.remove_chunk(pos)
            && chunk.is_modified()
        {
            sim.store.put(&chunk);
        }
    }

    // Load, nearest to a loader first.
    let mut missing: Vec<(i32, ChunkPos)> = Vec::new();
    for (lo, hi) in &rects {
        let mid = (*lo + *hi) / 2;
        for y in lo.y..=hi.y {
            for x in lo.x..=hi.x {
                let p = ChunkPos::new(x, y);
                if sim.generator.in_bounds(p) && !sim.world.is_loaded(p) {
                    missing.push(((IVec2::new(x, y) - mid).length_squared(), p));
                }
            }
        }
    }
    missing.sort_unstable();
    missing.dedup_by_key(|(_, p)| *p);
    missing.truncate(MAX_LOADS_PER_FRAME);
    metrics.loaded_this_frame = missing.len();

    let sim = &mut *sim;
    let from_store: Vec<_> = missing.iter().filter_map(|(_, p)| sim.store.take(*p)).collect();
    let generator = sim.generator.clone();
    let generated: Vec<_> = missing
        .par_iter()
        .filter(|(_, p)| !from_store.iter().any(|c| c.pos == *p))
        .map(|(_, p)| generator.generate(*p))
        .collect();
    for chunk in from_store.into_iter().chain(generated) {
        sim.world.insert_chunk(chunk);
    }
    metrics.stream_time = t.elapsed();
}

/// Edit `materials.ron` while the game runs; ids stay stable, errors are
/// logged and the old table is kept.
fn hot_reload_materials(mut src: ResMut<MaterialsSource>, mut sim: ResMut<SimWorld>) {
    if !src.0.changed() {
        return;
    }
    let text = match std::fs::read_to_string(src.0.path()) {
        Ok(t) => t,
        Err(e) => return warn!("materials: {e}"),
    };
    match sim.world.materials().reload_from_ron(&text) {
        Ok(table) => {
            info!("materials reloaded ({} materials)", table.len());
            sim.world.set_materials(Arc::new(table));
        }
        Err(e) => warn!("materials not reloaded: {e}"),
    }
}
