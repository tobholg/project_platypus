//! Lairs: cave chambers something has made its home (a spider nest). What
//! lairs there are is data (the game's `assets/data/lairs.ron`), given to
//! the generator with `TerrainGen::with_lairs`; which chambers they take is
//! decided from the seed with the rest of the plan, so every peer agrees.
//!
//! A lair lines its chamber: `lining` (cobweb) on the walls, `density` of
//! the wall's cells; threads of it hang from the roof. Its `keepers` stand
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
    /// Share of the chambers it could take that it does.
    pub chance: f32,
    /// What lines its walls, and how thickly (0..1).
    #[serde(default = "cobweb")]
    pub lining: String,
    #[serde(default = "half")]
    pub density: f32,
    /// Who lives there: (creature, how many).
    pub keepers: Vec<(String, u32)>,
    /// Only chambers at least this wide (half width, cells).
    #[serde(default = "roomy")]
    pub min_size: f32,
}

fn cobweb() -> String {
    "cobweb".into()
}

fn half() -> f32 {
    0.5
}

fn roomy() -> f32 {
    14.0
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
    pub keepers: Vec<&'static str>,
}

/// Which chamber each lair takes: `(chamber, lair)`, from the seed.
pub fn place(plan: &WorldPlan, caves: &Caves, defs: &[LairDef]) -> Vec<Option<u16>> {
    caves
        .chambers
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if c.pool.is_some() {
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
                    let roll = (hash(&[plan.seed, 0x1A12, i as u64, *k as u64]) % 10_000) as f32 / 10_000.0;
                    (d.depth.0..d.depth.1).contains(&depth)
                        && (d.zones.is_empty() || d.zones.iter().any(|z| z == zone))
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
