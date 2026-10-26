//! Lairs: cave chambers something has made its home (a spider nest). What
//! lairs there are is data (the game's `assets/data/lairs.ron`), given to
//! the generator with `TerrainGen::with_lairs`; which chambers they take is
//! decided from the seed with the rest of the plan, so every peer agrees.
//!
//! A lair lines its chamber: `lining` (cobweb) on the walls, `density` of
//! the wall's cells; threads of it hang from the roof; `litter` (guano)
//! lies heaped on its floor, `litter_density` of it. Its `keepers` stand
//! across its middle (they fall to its floor), reported by the chunk the
//! chamber's middle is in.

use platypus_sim::rng::hash;
use platypus_sim::{MaterialId, MaterialTable};
use serde::Deserialize;

use crate::caves::{Caves, Zone};
use crate::plan::WorldPlan;

/// One kind of lair, as written.
#[derive(Clone, Debug, Deserialize)]
pub struct LairDef {
    pub name: String,
    /// How deep under the surface its chamber is (cells, from..to).
    pub depth: (f32, f32),
    /// Underground biomes it's found in (`fungal`, `crystal`, `toxic`,
    /// `none` for plain rock); empty: any.
    #[serde(default)]
    pub zones: Vec<String>,
    /// Depth bands it's found in (`underground`, `caverns`, `deep`...: as
    /// deep as each is in this world's size); empty: any.
    #[serde(default)]
    pub bands: Vec<String>,
    /// Share of the chambers it could take that it does.
    pub chance: f32,
    /// What lines its walls, and how thickly (0..1).
    #[serde(default = "cobweb")]
    pub lining: String,
    #[serde(default = "half")]
    pub density: f32,
    /// What lies heaped on its floor (guano, under a roost), and how much
    /// of the floor (0..1).
    #[serde(default)]
    pub litter: Option<String>,
    #[serde(default)]
    pub litter_density: f32,
    /// Who lives there: (creature, how many).
    pub keepers: Vec<(String, u32)>,
    /// Only chambers at least this wide (half width, cells).
    #[serde(default = "roomy")]
    pub min_size: f32,
    /// One of the start valley's caverns' (and only those): on either side,
    /// the dark wood's, or the light forest's.
    #[serde(default)]
    pub start: Option<Side>,
    /// A chest on its floor.
    #[serde(default)]
    pub chest: bool,
}

/// Which side of the start valley a start lair is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Side {
    Any,
    Dark,
    Light,
}

fn cobweb() -> String {
    "cobweb".into()
}

fn half() -> f32 {
    0.5
}

fn roomy() -> f32 {
    21.0
}

/// Lairs as written (`assets/data/lairs.ron`; `Some` may be left out).
pub fn parse(text: &str) -> Result<Vec<LairDef>, String> {
    ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME).from_str(text).map_err(|e| e.to_string())
}

/// A lair ready to lay out: its lining's material, its keepers by name
/// (for `Spawn`, which names creatures for good).
pub struct Lair {
    pub name: String,
    pub lining: MaterialId,
    pub density: f32,
    pub litter: Option<(MaterialId, f32)>,
    pub keepers: Vec<&'static str>,
    pub chest: bool,
}

/// Which chamber each lair takes: `(chamber, lair)`, from the seed. The
/// start valley's caverns each take a start lair of their side (in turn
/// along it, so they differ); the rest, the others, by chance.
pub fn place(plan: &WorldPlan, caves: &Caves, defs: &[LairDef]) -> Vec<Option<u16>> {
    let starts: Vec<usize> = (0..defs.len()).filter(|&k| defs[k].start.is_some()).collect();
    let mid = plan.width / 2;
    let mut turn = [0usize; 2];
    caves
        .chambers
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if c.start {
                let dark = ((c.x as i32 - mid).signum()) == plan.start.dark_side();
                let side = if dark { Side::Dark } else { Side::Light };
                let fits: Vec<usize> = starts.iter().copied().filter(|&k| matches!(defs[k].start, Some(Side::Any)) || defs[k].start == Some(side)).collect();
                if fits.is_empty() {
                    return None;
                }
                let t = &mut turn[dark as usize];
                let k = fits[(*t + (hash(&[plan.seed, 0x5A1E, dark as u64]) as usize % fits.len())) % fits.len()];
                *t += 1;
                return Some(k as u16);
            }
            // (Not in a pool's chamber, nor one the caverns' water floods to
            // its middle: no nest or roost under water.)
            if c.pool.is_some() || (plan.band_at(c.y as i32) == crate::plan::Band::Caverns && (c.y as i32) < plan.water_table(c.x as i32)) {
                return None;
            }
            let depth = plan.surface_at(c.x as i32) as f32 - c.y;
            let zone = match c.zone {
                None => "none",
                Some(Zone::Fungal) => "fungal",
                Some(Zone::Crystal) => "crystal",
                Some(Zone::Toxic) => "toxic",
            };
            defs.iter()
                .enumerate()
                .find(|(k, d)| {
                    if d.start.is_some() {
                        return false;
                    }
                    let roll = (hash(&[plan.seed, 0x1A12, i as u64, *k as u64]) % 10_000) as f32 / 10_000.0;
                    (d.depth.0..d.depth.1).contains(&depth)
                        && (d.zones.is_empty() || d.zones.iter().any(|z| z == zone))
                        && (d.bands.is_empty() || d.bands.iter().any(|b| b == plan.band_at(c.y as i32).name()))
                        && c.rx >= d.min_size
                        && roll < d.chance
                })
                .map(|(k, _)| k as u16)
        })
        .collect()
}

/// A lair's materials and names, ready (names made to last: a handful).
pub fn ready(defs: &[LairDef], mats: &MaterialTable) -> Vec<Lair> {
    defs.iter()
        .map(|d| Lair {
            name: d.name.clone(),
            lining: mats.id(&d.lining).unwrap_or_else(|| mats.expect_id("cobweb")),
            density: d.density,
            litter: d.litter.as_deref().and_then(|m| mats.id(m)).map(|m| (m, d.litter_density)),
            chest: d.chest,
            keepers: d
                .keepers
                .iter()
                .flat_map(|(k, n)| {
                    let name: &'static str = Box::leak(k.clone().into_boxed_str());
                    std::iter::repeat_n(name, *n as usize)
                })
                .collect(),
        })
        .collect()
}
