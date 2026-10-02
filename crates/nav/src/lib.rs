//! Finding the way (DESIGN §14.4): one planner for every creature, its
//! abilities as data. No Bevy; the game hands it the world (`NavWorld`) and
//! each creature's `Profile`.
//!
//! - **The grid** (`tile.rs`): a node is `NODE` × `NODE` cells, a tile 16 ×
//!   16 nodes (one 64-cell chunk). For a body's size, a tile says of each
//!   node where it can stand (the floor's height in it, to the cell), where
//!   it fits in the open, where it's in liquid, where a climber can hold on;
//!   all from one pass of column clearances, through the same occupancy the
//!   bodies collide with. Tiles are made when a search first needs them and
//!   forgotten when their chunk's cells change (`Nav::forget`).
//! - **Moves** (`profile.rs`, `moves.rs`): what a body can do from a node, as
//!   offsets with a cost in seconds: walk (and step up), drop (no further
//!   than is safe), jump (arcs recorded once per profile by running the real
//!   physics with the creature's own movement), climb, fly, swim.
//! - **Searches** (`search.rs`): A* from a creature toward a goal, with a
//!   budget of nodes (out of it: the way toward the nearest it got); and a
//!   map spreading out from a target (`Field`) that every creature of a
//!   profile chasing it walks downhill on (fifty orcs cost one search).

pub mod moves;
pub mod profile;
pub mod search;
pub mod tile;
pub mod world;

pub use moves::{Kind, Move};
pub use profile::{Jump, Profile};
pub use search::{Field, Path, field, find, find_until};
pub use tile::{NODE, Nav, NodePos, TILE, View, node_feet, node_of, tile_of};

use platypus_physics::Grid;

/// The world as the planner sees it: the cells' occupancy as bodies see it
/// (`Grid`), and what climbers and diggers need besides.
pub trait NavWorld: Grid {
    /// A wall behind this cell to climb on (a cave's, a built wall, a trunk).
    fn backed(&self, _x: i32, _y: i32) -> bool {
        false
    }

    /// A column's occupancy from `y0` up, into `out` (a world reads it a
    /// chunk at a time: a tile is made from columns).
    fn column(&self, x: i32, y0: i32, out: &mut [platypus_physics::Occupancy]) {
        for (i, o) in out.iter_mut().enumerate() {
            *o = self.occupancy(x, y0 + i as i32);
        }
    }
}
