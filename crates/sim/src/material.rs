//! Materials are data (`assets/data/materials.ron`). The simulation reads a
//! compact per-material table (`MatPhys`) built from the definitions.

use bytemuck::{Pod, Zeroable};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::cell::Cell;
use crate::rng::Rng;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Pod, Zeroable)]
pub struct MaterialId(pub u16);

impl MaterialId {
    /// Index 0 is always air.
    pub const AIR: MaterialId = MaterialId(0);
}

/// Behaviour class. Everything is simulated; the class decides how.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Empty,
    /// Holds position until converted (dug, burned, melted, blasted).
    Static,
    Powder,
    Liquid,
    Gas,
    Fire,
    /// Tall grass, leaves: stays put, doesn't block creatures, burns readily,
    /// is crushed by falling powder and flowing liquid, withers without support.
    Plant,
}

impl Kind {
    /// Can things of other kinds flow through / displace this?
    #[inline]
    pub const fn is_fluid(self) -> bool {
        matches!(self, Kind::Empty | Kind::Liquid | Kind::Gas | Kind::Fire)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct MaterialDef {
    pub name: String,
    pub kind: Kind,
    /// Heavier fluids and powders sink through lighter fluids.
    #[serde(default)]
    pub density: u16,
    /// Gradient stops (sRGB). Each cell picks a point on it via `Cell::shade`.
    pub colors: Vec<(u8, u8, u8)>,
    /// 0 = dug instantly … 255 = indestructible.
    #[serde(default)]
    pub hardness: u8,
    /// Liquids: cells travelled sideways per tick. Gases: sideways jitter.
    #[serde(default)]
    pub dispersion: u8,
    /// Chance (/256 per tick) to catch fire next to something hot.
    #[serde(default)]
    pub flammability: u8,
    /// Sets flammable neighbours alight (fire, lava).
    #[serde(default)]
    pub hot: bool,
    /// Gases and fire: life range in decay steps.
    #[serde(default)]
    pub lifetime: Option<(u8, u8)>,
    /// Ticks per decay step (life can then last up to 255 × this).
    #[serde(default = "one")]
    pub decay_every: u8,
    /// What a cell becomes when its life runs out (default: air).
    #[serde(default)]
    pub decays_into: Option<String>,
    /// Chance /256 that `decays_into` is left rather than nothing. Steam that
    /// always condensed would drip back onto hot rock and cycle forever.
    #[serde(default = "always")]
    pub decays_into_chance: u8,
    /// What is left when it has burned out (default: nothing).
    #[serde(default)]
    pub burns_into: Option<String>,
    /// Chance /256 that `burns_into` is left rather than nothing (ash from wood).
    #[serde(default = "always")]
    pub burns_into_chance: u8,
    /// How long it burns, in steps of 4 ticks (default 45 = 3 s).
    #[serde(default)]
    pub burn_time: Option<u8>,
    /// Share of its burn after which a burning solid is charred and no longer
    /// carries weight (default 0.5; 1 = holds until it's gone). A trunk burning
    /// at the base snaps before it has burned through.
    #[serde(default)]
    pub chars_at: Option<f32>,
    /// Damage per second it does to a body touching it (acid 30).
    #[serde(default)]
    pub corrosive: u8,
    /// Chance /4096 per tick that it catches from a burning neighbour (twice
    /// that from below, half from above). Default: flammability × 16, i.e.
    /// flammability /256. Together with how long a material burns this sets
    /// whether fire runs through it or dies out (SPEC §3.8).
    #[serde(default)]
    pub spread: Option<u16>,
    /// Chance /4096 per tick that a burning cell with fewer than two burning
    /// neighbours goes out: a spark on a log can fizzle, a blaze can't.
    #[serde(default)]
    pub fizzles: u16,
    /// Water vapour: when it fades into the air it feeds the clouds above
    /// (weather), so boiled water comes back as rain.
    #[serde(default)]
    pub vapour: bool,
    /// What a charred cell becomes when its fire is put out (wood: charcoal).
    /// Default: it just stops burning.
    #[serde(default)]
    pub chars_into: Option<String>,
    /// What a solid breaks into when shattered but not destroyed (e.g. by a
    /// blast's rim): loose rubble that then falls. Default: nothing.
    #[serde(default)]
    pub crumbles_into: Option<String>,

    // ---- temperature (°C). See SPEC §3.6. --------------------------------
    /// Heat a fresh cell starts with, relative to ambient (lava 1200, ice -30).
    #[serde(default)]
    pub heat: i16,
    /// Holds `heat` forever instead of cooling (lava, fire while it burns).
    #[serde(default)]
    pub heat_source: bool,
    /// 0..255, how fast heat moves through it. Default depends on kind.
    #[serde(default)]
    pub conductivity: Option<u8>,
    /// Becomes another material at or above an absolute temperature (melt, boil).
    #[serde(default)]
    pub above: Option<(i16, String)>,
    /// Becomes another material at or below an absolute temperature (freeze).
    #[serde(default)]
    pub below: Option<(i16, String)>,
    /// Catches fire (becomes `burns_into`) at or above this absolute temperature.
    #[serde(default)]
    pub ignites_at: Option<i16>,
    /// When it catches fire, sometimes explodes instead (chained, merged per tick).
    #[serde(default)]
    pub explodes: Option<ExplosionDef>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub struct ExplosionDef {
    pub radius: i32,
    pub power: u8,
    /// Chance /256 that an igniting cell explodes rather than just burning.
    pub chance: u8,
}

fn one() -> u8 {
    1
}

fn always() -> u8 {
    255
}

#[derive(Clone, Debug, Deserialize)]
pub struct ReactionDef {
    pub a: String,
    pub b: String,
    pub a_into: String,
    pub b_into: String,
    /// Chance per tick (/256) while touching.
    pub chance: u8,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MaterialsFile {
    pub materials: Vec<MaterialDef>,
    #[serde(default)]
    pub reactions: Vec<ReactionDef>,
}

/// Hot per-material data read by the stepping rules.
#[derive(Clone, Copy, Debug)]
pub struct MatPhys {
    pub kind: Kind,
    pub density: u16,
    pub hardness: u8,
    pub dispersion: u8,
    pub flammability: u8,
    pub hot: bool,
    pub life_min: u8,
    pub life_max: u8,
    pub decay_every: u8,
    pub decays_into: MaterialId,
    pub decays_into_chance: u8,
    /// Left behind after burning out (`AIR` = nothing).
    pub burns_into: MaterialId,
    pub burns_into_chance: u8,
    /// Burn duration in steps of 4 ticks.
    pub burn_time: u8,
    /// A burning cell with less `life` than this is charred: it no longer
    /// carries weight (0 = never).
    pub charred_life: u8,
    /// `AIR` = stays itself when put out.
    pub chars_into: MaterialId,
    /// Damage per second to a body touching it.
    pub corrosive: u8,
    pub spread: u16,
    pub fizzles: u16,
    pub vapour: bool,
    /// `AIR` when the material doesn't crumble.
    pub crumbles_into: MaterialId,
    pub heat: i16,
    pub heat_source: bool,
    pub conductivity: u8,
    /// Absolute °C; `i16::MAX` = never.
    pub above_at: i16,
    pub above_into: MaterialId,
    /// Absolute °C; `i16::MIN` = never.
    pub below_at: i16,
    pub below_into: MaterialId,
    /// Absolute °C; `i16::MAX` = never.
    pub ignites_at: i16,
    pub explodes: Option<ExplosionDef>,
    /// Has a temperature transition that ordinary climates can reach
    /// (ice, snow, water), so it must be checked even at ambient.
    pub climate_sensitive: bool,
    /// Needs the neighbour check (hot or part of a reaction).
    pub interacts: bool,
    /// Can ever change on its own; `false` lets the stepper skip the cell.
    pub active: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Reaction {
    pub partner: MaterialId,
    pub self_into: MaterialId,
    pub partner_into: MaterialId,
    pub chance: u8,
}

/// Number of colours precomputed per material.
pub const PALETTE_SIZE: usize = 16;

#[derive(Debug)]
pub enum MaterialError {
    Parse(String),
    Invalid(String),
}

impl std::fmt::Display for MaterialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaterialError::Parse(e) => write!(f, "materials: parse error: {e}"),
            MaterialError::Invalid(e) => write!(f, "materials: {e}"),
        }
    }
}

impl std::error::Error for MaterialError {}

pub struct MaterialTable {
    /// The flame material (`fire`), or air if the content has none.
    fire: MaterialId,
    defs: Vec<MaterialDef>,
    phys: Vec<MatPhys>,
    reactions: Vec<Vec<Reaction>>,
    palette: Vec<[u8; 4]>,
    by_name: FxHashMap<String, MaterialId>,
}

impl MaterialTable {
    pub fn from_ron(src: &str) -> Result<Self, MaterialError> {
        let file = parse(src)?;
        Self::build(file.materials, file.reactions)
    }

    /// Re-read definitions while keeping every existing name on its old id,
    /// so cells already in the world keep their meaning. New names are appended.
    pub fn reload_from_ron(&self, src: &str) -> Result<Self, MaterialError> {
        let file = parse(src)?;
        let mut incoming: FxHashMap<String, MaterialDef> =
            file.materials.into_iter().map(|d| (d.name.clone(), d)).collect();
        let mut ordered = Vec::with_capacity(self.defs.len() + incoming.len());
        for old in &self.defs {
            // A removed material keeps its slot (with its old definition) so ids stay stable.
            ordered.push(incoming.remove(&old.name).unwrap_or_else(|| old.clone()));
        }
        let mut new: Vec<_> = incoming.into_values().collect();
        new.sort_by(|a, b| a.name.cmp(&b.name));
        ordered.extend(new);
        Self::build(ordered, file.reactions)
    }

    fn build(defs: Vec<MaterialDef>, reaction_defs: Vec<ReactionDef>) -> Result<Self, MaterialError> {
        if defs.first().map(|d| (d.name.as_str(), d.kind)) != Some(("air", Kind::Empty)) {
            return Err(MaterialError::Invalid("the first material must be `air` with kind Empty".into()));
        }
        if defs.len() > u16::MAX as usize {
            return Err(MaterialError::Invalid("too many materials".into()));
        }
        let mut by_name = FxHashMap::default();
        for (i, d) in defs.iter().enumerate() {
            if d.colors.is_empty() && d.kind != Kind::Empty {
                return Err(MaterialError::Invalid(format!("`{}` has no colors", d.name)));
            }
            if by_name.insert(d.name.clone(), MaterialId(i as u16)).is_some() {
                return Err(MaterialError::Invalid(format!("duplicate material `{}`", d.name)));
            }
        }
        let lookup = |name: &str, ctx: &str| {
            by_name
                .get(name)
                .copied()
                .ok_or_else(|| MaterialError::Invalid(format!("{ctx}: unknown material `{name}`")))
        };

        let fire = by_name.get("fire").copied();
        let mut phys = Vec::with_capacity(defs.len());
        for d in &defs {
            let decays_into = match &d.decays_into {
                Some(n) => lookup(n, &format!("{}.decays_into", d.name))?,
                None => MaterialId::AIR,
            };
            let burns_into = match &d.burns_into {
                Some(n) => lookup(n, &format!("{}.burns_into", d.name))?,
                None => MaterialId::AIR,
            };
            let chars_into = match &d.chars_into {
                Some(n) => lookup(n, &format!("{}.chars_into", d.name))?,
                None => MaterialId::AIR,
            };
            let crumbles_into = match &d.crumbles_into {
                Some(n) => lookup(n, &format!("{}.crumbles_into", d.name))?,
                None => MaterialId::AIR,
            };
            let (above_at, above_into) = match &d.above {
                Some((t, n)) => (*t, lookup(n, &format!("{}.above", d.name))?),
                None => (i16::MAX, MaterialId::AIR),
            };
            let (below_at, below_into) = match &d.below {
                Some((t, n)) => (*t, lookup(n, &format!("{}.below", d.name))?),
                None => (i16::MIN, MaterialId::AIR),
            };
            let conductivity = d.conductivity.unwrap_or(match d.kind {
                Kind::Empty => 0,
                Kind::Static => 40,
                Kind::Powder => 30,
                Kind::Liquid => 50,
                Kind::Gas => 8,
                Kind::Fire => 60,
                Kind::Plant => 10,
            });
            let climate_sensitive = (-150..=150).contains(&above_at) || (-150..=150).contains(&below_at);
            let (life_min, life_max) = d.lifetime.unwrap_or((0, 0));
            phys.push(MatPhys {
                kind: d.kind,
                density: d.density,
                hardness: d.hardness,
                dispersion: d.dispersion,
                flammability: d.flammability,
                hot: d.hot,
                life_min,
                life_max: life_max.max(life_min),
                decay_every: d.decay_every.max(1),
                decays_into,
                decays_into_chance: d.decays_into_chance,
                burns_into,
                burns_into_chance: d.burns_into_chance,
                burn_time: d.burn_time.unwrap_or(45),
                charred_life: {
                    let burn = d.burn_time.unwrap_or(45) as f32;
                    (burn * (1.0 - d.chars_at.unwrap_or(0.5).clamp(0.0, 1.0))).round() as u8
                },
                chars_into,
                corrosive: d.corrosive,
                spread: d.spread.unwrap_or(d.flammability as u16 * 16),
                fizzles: d.fizzles,
                vapour: d.vapour,
                crumbles_into,
                heat: d.heat,
                heat_source: d.heat_source,
                conductivity: if d.kind == Kind::Empty { 0 } else { conductivity },
                above_at,
                above_into,
                below_at,
                below_into,
                ignites_at: d.ignites_at.unwrap_or(i16::MAX),
                explodes: d.explodes,
                climate_sensitive,
                interacts: d.hot,
                active: false,
            });
        }

        let mut reactions = vec![Vec::new(); defs.len()];
        for r in &reaction_defs {
            let ctx = format!("reaction {}+{}", r.a, r.b);
            let (a, b) = (lookup(&r.a, &ctx)?, lookup(&r.b, &ctx)?);
            let (ai, bi) = (lookup(&r.a_into, &ctx)?, lookup(&r.b_into, &ctx)?);
            reactions[a.0 as usize].push(Reaction { partner: b, self_into: ai, partner_into: bi, chance: r.chance });
            if a != b {
                reactions[b.0 as usize].push(Reaction { partner: a, self_into: bi, partner_into: ai, chance: r.chance });
            }
        }
        for (p, rs) in phys.iter_mut().zip(&reactions) {
            p.interacts |= !rs.is_empty();
            p.active = p.interacts
                || p.climate_sensitive
                || p.heat_source
                || matches!(p.kind, Kind::Powder | Kind::Liquid | Kind::Gas | Kind::Fire | Kind::Plant);
        }

        let mut palette = Vec::with_capacity(defs.len() * PALETTE_SIZE);
        for d in &defs {
            for i in 0..PALETTE_SIZE {
                palette.push(gradient(&d.colors, i as f32 / (PALETTE_SIZE - 1) as f32, d.kind == Kind::Empty));
            }
        }

        Ok(MaterialTable { fire: fire.unwrap_or(MaterialId::AIR), defs, phys, reactions, palette, by_name })
    }

    /// Burned far enough that it no longer carries weight (SPEC §3.7).
    #[inline]
    pub fn is_charred(&self, c: Cell) -> bool {
        c.flags & crate::cell::flags::BURNING != 0 && c.life < self.phys(c.material).charred_life
    }

    /// Does this cell hold up what rests on it? Every ground check asks this.
    #[inline]
    pub fn bears_load(&self, c: Cell) -> bool {
        !self.is_charred(c)
    }

    #[inline]
    pub fn phys(&self, id: MaterialId) -> &MatPhys {
        &self.phys[id.0 as usize]
    }

    /// The flame material: what gases turn into when lit, and what burning
    /// things put out into the air around them.
    #[inline]
    pub fn fire(&self) -> MaterialId {
        self.fire
    }

    #[inline]
    pub fn reactions(&self, id: MaterialId) -> &[Reaction] {
        &self.reactions[id.0 as usize]
    }

    pub fn def(&self, id: MaterialId) -> &MaterialDef {
        &self.defs[id.0 as usize]
    }

    pub fn id(&self, name: &str) -> Option<MaterialId> {
        self.by_name.get(name).copied()
    }

    /// For code paths where a missing material is a content bug.
    pub fn expect_id(&self, name: &str) -> MaterialId {
        self.id(name).unwrap_or_else(|| panic!("material `{name}` is not defined in materials.ron"))
    }

    pub fn len(&self) -> usize {
        self.defs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (MaterialId, &MaterialDef)> {
        self.defs.iter().enumerate().map(|(i, d)| (MaterialId(i as u16), d))
    }

    /// sRGBA colour of a cell.
    #[inline]
    pub fn color(&self, cell: Cell) -> [u8; 4] {
        self.palette[cell.material.0 as usize * PALETTE_SIZE + (cell.shade as usize >> 4)]
    }

    /// A fresh cell of `id` with random shade and, for gases/fire, a random life.
    #[inline]
    pub fn spawn(&self, id: MaterialId, rng: &mut Rng) -> Cell {
        let p = self.phys(id);
        let mut c = Cell::new(id, rng.next_u8());
        c.heat = p.heat;
        if p.life_max > 0 {
            c.life = rng.range_u8(p.life_min, p.life_max);
        }
        c
    }
}

/// Content files may write `lifetime: (1, 2)` rather than `lifetime: Some((1, 2))`.
fn parse(src: &str) -> Result<MaterialsFile, MaterialError> {
    ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
        .from_str(src)
        .map_err(|e| MaterialError::Parse(e.to_string()))
}

fn gradient(stops: &[(u8, u8, u8)], t: f32, empty: bool) -> [u8; 4] {
    if empty || stops.is_empty() {
        return [0, 0, 0, 0];
    }
    if stops.len() == 1 {
        let (r, g, b) = stops[0];
        return [r, g, b, 255];
    }
    let f = t * (stops.len() - 1) as f32;
    let i = (f as usize).min(stops.len() - 2);
    let k = f - i as f32;
    let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * k).round() as u8;
    let (a, b) = (stops[i], stops[i + 1]);
    [lerp(a.0, b.0), lerp(a.1, b.1), lerp(a.2, b.2), 255]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const TEST_MATERIALS: &str = r#"(
        materials: [
            (name: "air",   kind: Empty, colors: []),
            (name: "stone", kind: Static, density: 3000, hardness: 60, colors: [(120,120,120)]),
            (name: "sand",  kind: Powder, density: 1600, colors: [(200,170,100), (220,190,120)]),
            (name: "water", kind: Liquid, density: 1000, dispersion: 5, colors: [(40,90,200)]),
            (name: "oil",   kind: Liquid, density: 800, dispersion: 3, flammability: 80, colors: [(60,40,20)]),
            (name: "lava",  kind: Liquid, density: 2500, dispersion: 1, hot: true, colors: [(255,90,0)]),
            (name: "steam", kind: Gas, density: 1, dispersion: 2, lifetime: (60, 120), colors: [(220,220,230)]),
            (name: "smoke", kind: Gas, density: 2, dispersion: 1, lifetime: (20, 60), colors: [(60,60,60)]),
            (name: "fire",  kind: Fire, hot: true, lifetime: (10, 30), decays_into: "smoke", colors: [(255,200,0)]),
            (name: "wood",  kind: Static, density: 700, flammability: 40, colors: [(110,70,30)]),
        ],
        reactions: [
            (a: "lava", b: "water", a_into: "stone", b_into: "steam", chance: 255),
            (a: "fire", b: "water", a_into: "smoke", b_into: "water", chance: 200),
        ],
    )"#;

    #[test]
    fn loads_and_links_names() {
        let t = MaterialTable::from_ron(TEST_MATERIALS).unwrap();
        let fire = t.expect_id("fire");
        assert_eq!(t.fire(), fire);
        assert_eq!(t.phys(t.expect_id("wood")).burns_into, MaterialId::AIR, "burns to nothing by default");
        assert_eq!(t.phys(fire).decays_into, t.expect_id("smoke"));
        assert!(t.phys(t.expect_id("water")).interacts, "water reacts with lava");
        assert!(!t.phys(t.expect_id("stone")).active);
        assert_eq!(t.reactions(t.expect_id("water")).len(), 2);
    }

    #[test]
    fn reload_keeps_ids_stable() {
        let t = MaterialTable::from_ron(TEST_MATERIALS).unwrap();
        let water = t.expect_id("water");
        // Reordered, one new material, water now more dispersive.
        let edited = TEST_MATERIALS
            .replace(r#"(name: "sand","#, r#"(name: "gravel", kind: Powder, density: 1800, colors: [(90,90,90)]),
            (name: "sand","#)
            .replace("dispersion: 5", "dispersion: 9");
        let t2 = t.reload_from_ron(&edited).unwrap();
        assert_eq!(t2.expect_id("water"), water);
        assert_eq!(t2.phys(water).dispersion, 9);
        assert_eq!(t2.expect_id("gravel").0 as usize, t.len());
    }

    #[test]
    fn rejects_unknown_names() {
        let bad = TEST_MATERIALS.replace(r#"decays_into: "smoke""#, r#"decays_into: "smok""#);
        assert!(MaterialTable::from_ron(&bad).is_err());
    }
}
