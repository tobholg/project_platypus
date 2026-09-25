//! Ores and gems (DESIGN §3.2 step 6): where they lie. Each ore has a depth
//! window spanning parts of the bands, with a fade at its ends. Inside it
//! the ore forms noise blobs or veins stretched along the strata, and it's
//! likelier where a cave wall exposes it, so exploring finds ore. Gems grow
//! only in cave walls, in small glowing clusters.
//!
//! Hardness climbs with depth (materials.ron), so a pickaxe's tier decides
//! how deep you can mine for profit.

use platypus_sim::{MaterialId, MaterialTable};

use crate::plan::{Band, WorldPlan};

/// One ore's rule.
#[derive(Clone, Debug)]
pub struct Ore {
    pub material: MaterialId,
    /// Where it lies, as (lowest y, highest y).
    pub window: (i32, i32),
    /// Cells per noise unit across and down: wide and flat is a vein along
    /// the strata, round is a blob.
    pub scale: (f64, f64),
    /// Noise above this is ore (in the middle of the window).
    pub threshold: f64,
    /// Separates each ore's noise from the others.
    pub salt: f64,
}

/// One gem's rule: the band it grows in.
#[derive(Clone, Debug)]
pub struct Gem {
    pub material: MaterialId,
    pub band: Band,
    pub salt: f64,
}

/// How much lower the threshold is where a cave is near: ore shows on walls.
pub const EXPOSED_BONUS: f64 = 0.1;
/// How far (cells) to look for a cave when deciding "exposed".
pub const EXPOSED_REACH: i32 = 5;
/// Gem clusters: noise above this, in a wall within `GEM_REACH` of a cave,
/// in the stretches of wall where gems grow at all (a coarser noise above
/// `GEM_ZONE`, every `GEM_ZONE_SCALE` cells or so), so they're a find rather
/// than a lining.
pub const GEM_THRESHOLD: f64 = 0.4;
pub const GEM_ZONE: f64 = 0.55;
pub const GEM_ZONE_SCALE: f64 = 140.0;
pub const GEM_REACH: i32 = 4;
/// Cells per noise unit for gem clusters (a few cells across).
pub const GEM_SCALE: f64 = 7.0;

/// Every ore and gem, placed for this plan's bands.
pub fn rules(plan: &WorldPlan, mats: &MaterialTable) -> (Vec<Ore>, Vec<Gem>) {
    let span = |b: Band| plan.band_span(b);
    // A point `f` of the way down a band (0 = its top, 1 = its floor).
    let at = |b: Band, f: f64| {
        let (lo, hi) = span(b);
        hi - ((hi - lo) as f64 * f) as i32
    };
    let top = plan.height;
    let ore = |name: &str, window: (i32, i32), scale: (f64, f64), threshold: f64, salt: f64| {
        assert!(window.0 < window.1, "{name}: its window is upside down ({window:?})");
        Ore { material: mats.expect_id(name), window, scale, threshold, salt }
    };
    // (Windows are (lowest y, highest y): the deeper point first.)
    let ores = vec![
        // Coal: shallow seams, flat and wide, up into the mountains.
        ore("coal", (at(Band::Caverns, 0.5), top), (34.0, 13.0), 0.56, 11.9),
        // Copper: common veins from the mountains down through the underground.
        ore("copper_ore", (at(Band::Underground, 1.0), top), (44.0, 16.0), 0.5, 21.3),
        // Iron: veins from mid-underground to the upper caverns.
        ore("iron_ore", (at(Band::Caverns, 0.4), at(Band::Underground, 0.35)), (40.0, 15.0), 0.52, 33.7),
        // Silver: blobs through the caverns.
        ore("silver_ore", (at(Band::Caverns, 1.0), at(Band::Caverns, 0.1)), (20.0, 16.0), 0.55, 45.1),
        // Gold: smaller blobs, lower caverns and the upper deep.
        ore("gold_ore", (at(Band::Deep, 0.55), at(Band::Caverns, 0.5)), (17.0, 14.0), 0.57, 57.9),
        // Mithril: long rare seams in the deep's slate.
        ore("mithril_ore", (span(Band::Deep).0, at(Band::Deep, 0.2)), (34.0, 10.0), 0.68, 69.4),
    ];
    let gem = |name: &str, band: Band, salt: f64| Gem { material: mats.expect_id(name), band, salt };
    let gems = vec![gem("amethyst", Band::Underground, 81.7), gem("emerald", Band::Caverns, 93.1), gem("ruby", Band::Deep, 105.3)];
    (ores, gems)
}

impl Ore {
    /// The threshold at `y`: the rule's inside the window, rising toward its
    /// ends (over 15% of it, at most 400 cells) so ore thins out rather than
    /// stopping at a line. `None` outside.
    pub fn threshold_at(&self, y: i32) -> Option<f64> {
        let (lo, hi) = self.window;
        if y < lo || y > hi {
            return None;
        }
        let fade = ((hi - lo) as f64 * 0.15).clamp(1.0, 400.0);
        let edge = ((y - lo).min(hi - y) as f64 / fade).min(1.0);
        Some(self.threshold + (1.0 - edge) * 0.25)
    }
}
