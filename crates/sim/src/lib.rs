//! The Platypus cell world: every material is a simulated cell.
//!
//! No Bevy in here. See SPEC.md §3.

pub mod cell;
pub mod chunk;
pub mod climate;
pub mod coords;
pub mod edit;
pub mod material;
pub mod particles;
pub mod rng;
mod rules;
mod step;
pub mod store;
mod world;

pub use cell::Cell;
pub use chunk::Chunk;
pub use climate::Climate;
pub use coords::{CHUNK, CHUNK_AREA, CellPos, ChunkPos, Rect};
pub use edit::{EditReport, WorldEdit};
pub use material::{Kind, MaterialId, MaterialTable};
pub use particles::{Landing, Particle};
pub use step::StepStats;
pub use world::World;
