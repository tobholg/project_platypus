//! A creature's body: how it looks and animates (`animation`), procedural
//! legs (`legs`) and chains (`chains`: tails, stings), its parts' hit
//! areas (`parts`), hurt's flash and blood (`hurt`), and what's on it:
//! burning, wet, chilled, coated (`elements`), and on the player what's
//! hurting it (`signs`); a body that bends, drawn
//! along its spine (`spine`: a rat's); its eyes in the dark only
//! where the player sees it (`sight`); knocked about, it spins (`tumble`); killed, a skeleton's bones heap
//! and pull themselves back together (`rise`).

pub mod animation;
pub mod chains;
pub mod elements;
pub mod emerge;
pub mod goo;
pub mod hurt;
pub mod legs;
pub mod marks;
pub mod parts;
pub mod ragdoll;
pub mod rise;
pub mod sight;
pub mod signs;
pub mod spine;
pub mod tumble;
pub mod wounds;
