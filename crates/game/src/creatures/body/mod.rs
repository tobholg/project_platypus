//! A creature's body: how it looks and animates (`animation`), procedural
//! legs (`legs`) and chains (`chains`: tails, stings), its parts' hit
//! areas (`parts`), hurt's flash and blood (`hurt`), and what's on it:
//! burning, wet, chilled, coated (`elements`); its eyes in the dark only
//! where the player sees it (`sight`).

pub mod animation;
pub mod chains;
pub mod elements;
pub mod hurt;
pub mod legs;
pub mod parts;
pub mod sight;
