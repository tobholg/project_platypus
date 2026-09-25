//! Everything that moves: bodies against the cell grid, and the locomotion
//! controller shared by the player and every creature. No Bevy. See SPEC §5.

pub mod body;
pub mod locomotion;

pub use body::{Body, Contacts, Grid, Occupancy, move_and_collide};
pub use locomotion::{Intent, Locomotion, MoveEvents, MoveState, MovementStats};
