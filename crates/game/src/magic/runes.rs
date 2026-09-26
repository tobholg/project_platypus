//! Runes and how a wand's runes make casts (DESIGN §7b). A rune is a
//! carrier (how a spell travels), a payload (what it does where it lands)
//! or a modifier (how it behaves). A wand holds runes in order; a cast reads
//! them left to right:
//!
//! - modifiers gather until a carrier, which they shape;
//! - the payloads after a carrier are what it carries;
//! - a `Trigger` among its modifiers makes the rest of the wand the cast it
//!   sets off where it lands; otherwise the rest is the wand's next cast.
//!
//! So `[trail fire, orb, blast, ignite]` is a fireball, and
//! `[trigger, bolt, spark, orb, blast]` a spark bolt that bursts into a
//! bouncing bomb where it hits.

use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;

/// How a spell travels.
#[derive(Clone, Debug, Deserialize)]
pub enum Carrier {
    /// Fast and straight (cells/s), gone after `life` seconds.
    Bolt { speed: f32, life: f32 },
    /// Slower, falling, bouncing off what it hits `bounces` times.
    Orb { speed: f32, life: f32, bounces: u8 },
    /// A spray of cells of a material from the wand (a flamethrower): `rate`
    /// a cast, at `speed`, spread `spread` radians; `burning` lights them.
    Stream { material: String, rate: u32, speed: f32, spread: f32, burning: bool },
    /// The sky's lightning, from the wand: a bolt to up to `targets`
    /// creatures within `range` of the aim (or the aim itself).
    Lightning { range: f32, targets: u8 },
    /// A gravity well at the cursor while the wand is held (`well.rs`):
    /// pulls in what's loose within `radius` and tears out solids up to
    /// `strength` hardness (`pull` tries a tick), holds what it can `lift`
    /// (a cell weighs its density against water's, a body its size in
    /// cells: creatures too) in a spinning ball with `grip` (cells/s², how
    /// hard it can swing them), costs `drain` mana a second.
    Well { radius: f32, strength: u8, lift: f32, pull: u32, grip: f32, drain: f32 },
    /// Force at the cursor while the wand is held: the left button pushes
    /// everything within `radius` away (up to `power` cells/s at the
    /// heart), the right pulls it in; tears out solids up to `strength`
    /// (`pull` tries a tick); `drain` mana a second.
    Force { radius: f32, power: f32, strength: u8, pull: u32, drain: f32 },
}

/// What a spell does where it lands (or what it hits).
#[derive(Clone, Debug, Deserialize)]
pub enum Payload {
    /// Hurt what it hits (hit points).
    Damage(f32),
    /// Blow up (the sim's blast: cells, bodies, fire).
    Blast { radius: i32, power: u8 },
    /// Heat (°C, − to freeze) in a radius.
    Heat { radius: i32, amount: i16 },
    /// Set what burns alight.
    Ignite { radius: i32 },
    /// Splash cells of a material.
    Matter { material: String, cells: u32 },
}

/// How a carrier behaves.
#[derive(Clone, Debug, Deserialize)]
pub enum Modifier {
    /// Falls (gravity, relative to a thrown thing's).
    Gravity(f32),
    /// Leaves cells of a material behind it as it goes, burning or not.
    Trail { material: String, burning: bool },
    /// Sheds a splash of `cells` of a material (burning or not) wherever it
    /// bounces.
    Shed { material: String, cells: u32, burning: bool },
    /// Faster (a multiplier).
    Speed(f32),
    /// What's left of the wand is cast where this lands.
    Trigger,
}

/// What a rune looks like (visual only: `vfx`): sparks it trails as it
/// flies, and sparks it bursts into where it lands. A cast shows the looks
/// of all its runes (a fire trail on a bolt carrying acid: flames and
/// drips).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Look {
    pub trail: Option<Emitter>,
    pub burst: Option<Emitter>,
}

/// Sparks: `count` per cell flown (a trail) or all at once (a burst).
#[derive(Clone, Debug, Deserialize)]
pub struct Emitter {
    pub count: f32,
    /// Seconds, (shortest, longest).
    pub life: (f32, f32),
    /// Its colour over its life, first to last (it fades out at the end).
    pub colors: Vec<(u8, u8, u8)>,
    /// Cells/s, fastest (each gets 30–100 % of it).
    #[serde(default)]
    pub speed: f32,
    /// Radians either side of its direction (a trail: backwards; a burst:
    /// off the surface it hit). 3.14 = all round.
    #[serde(default = "all_round")]
    pub spread: f32,
    /// Cells/s² down (negative rises).
    #[serde(default)]
    pub gravity: f32,
    /// Share of its speed lost a second.
    #[serde(default)]
    pub drag: f32,
    /// Cells across.
    #[serde(default = "one_cell")]
    pub size: f32,
    /// Jumps sideways at random (cells/s): electric crackle.
    #[serde(default)]
    pub jitter: f32,
    /// A faint halo around it.
    #[serde(default)]
    pub glow: bool,
}

fn all_round() -> f32 {
    std::f32::consts::PI
}

fn one_cell() -> f32 {
    1.0
}

#[derive(Clone, Debug, Deserialize)]
pub enum RuneKind {
    Carrier(Carrier),
    Payload(Payload),
    Modifier(Modifier),
}

#[derive(Clone, Debug, Deserialize)]
pub struct RuneDef {
    pub id: String,
    pub name: String,
    /// Mana it costs, each cast.
    pub mana: f32,
    /// What colour it glows (the projectile and its light).
    #[serde(default = "white")]
    pub color: (u8, u8, u8),
    pub kind: RuneKind,
    #[serde(default)]
    pub look: Look,
}

fn white() -> (u8, u8, u8) {
    (230, 230, 255)
}

#[derive(Clone, Debug, Deserialize)]
pub struct RunesFile {
    pub runes: Vec<RuneDef>,
}

/// Every rune there is.
#[derive(Clone, Debug, Default)]
pub struct Runes {
    defs: Vec<RuneDef>,
    by_id: HashMap<String, usize>,
}

impl Runes {
    pub fn new(file: RunesFile) -> Result<Runes, String> {
        let mut by_id = HashMap::new();
        for (i, d) in file.runes.iter().enumerate() {
            if by_id.insert(d.id.clone(), i).is_some() {
                return Err(format!("rune `{}` is defined twice", d.id));
            }
        }
        Ok(Runes { defs: file.runes, by_id })
    }

    pub fn get(&self, id: &str) -> Option<&RuneDef> {
        self.by_id.get(id).map(|&i| &self.defs[i])
    }
}

/// One spell, ready to go: a carrier with what shapes it and what it
/// carries, and what it sets off where it lands.
#[derive(Clone, Debug)]
pub struct Cast {
    pub carrier: Carrier,
    pub modifiers: Vec<Modifier>,
    pub payloads: Vec<Payload>,
    pub then: Option<Arc<Cast>>,
    pub mana: f32,
    pub color: (u8, u8, u8),
    /// The looks of all its runes: trails while it flies, bursts where it
    /// lands.
    pub trails: Vec<Emitter>,
    pub bursts: Vec<Emitter>,
}

impl Cast {
    pub fn gravity(&self) -> f32 {
        self.modifiers.iter().map(|m| if let Modifier::Gravity(g) = m { *g } else { 0.0 }).sum()
    }

    pub fn speed_scale(&self) -> f32 {
        self.modifiers.iter().map(|m| if let Modifier::Speed(k) = m { *k } else { 1.0 }).product()
    }

    pub fn trail(&self) -> Option<(&str, bool)> {
        self.modifiers.iter().find_map(|m| if let Modifier::Trail { material, burning } = m { Some((material.as_str(), *burning)) } else { None })
    }

    pub fn shed(&self) -> Option<(&str, u32, bool)> {
        self.modifiers.iter().find_map(|m| if let Modifier::Shed { material, cells, burning } = m { Some((material.as_str(), *cells, *burning)) } else { None })
    }

    /// Mana for this cast and what it sets off.
    pub fn total_mana(&self) -> f32 {
        self.mana + self.then.as_ref().map_or(0.0, |t| t.total_mana())
    }
}

/// A wand's runes as its casts, in order (see the module notes). Runes that
/// shape nothing (no carrier after them) are dropped.
pub fn casts(runes: &Runes, ids: &[String]) -> Result<Vec<Arc<Cast>>, String> {
    let defs: Vec<&RuneDef> = ids.iter().map(|id| runes.get(id).ok_or(format!("no rune `{id}`"))).collect::<Result<_, _>>()?;
    let mut out = Vec::new();
    let mut rest: &[&RuneDef] = &defs;
    while let Some((cast, left)) = one(rest) {
        out.push(Arc::new(cast));
        rest = left;
    }
    Ok(out)
}

/// The first cast in `runes`, and what's left after it.
fn one<'a, 'b>(runes: &'b [&'a RuneDef]) -> Option<(Cast, &'b [&'a RuneDef])> {
    let mut modifiers = Vec::new();
    let mut mana = 0.0;
    let mut i = 0;
    let (mut trails, mut bursts) = (Vec::new(), Vec::new());
    let mut look = |r: &RuneDef| {
        trails.extend(r.look.trail.clone());
        bursts.extend(r.look.burst.clone());
    };
    let (carrier, color) = loop {
        let r = runes.get(i)?;
        i += 1;
        mana += r.mana;
        look(r);
        match &r.kind {
            RuneKind::Modifier(m) => modifiers.push(m.clone()),
            RuneKind::Carrier(c) => break (c.clone(), r.color),
            // (A payload before any carrier has nothing to ride.)
            RuneKind::Payload(_) => {}
        }
    };
    // It looks like what it carries (acid is green), or like its carrier.
    let mut color = color;
    let mut payloads = Vec::new();
    while let Some(RuneKind::Payload(p)) = runes.get(i).map(|r| &r.kind) {
        mana += runes[i].mana;
        color = runes[i].color;
        look(runes[i]);
        payloads.push(p.clone());
        i += 1;
    }
    let trigger = modifiers.iter().any(|m| matches!(m, Modifier::Trigger));
    let (then, left) = if trigger {
        match one(&runes[i..]) {
            Some((next, left)) => (Some(Arc::new(next)), left),
            None => (None, &runes[i..]),
        }
    } else {
        (None, &runes[i..])
    };
    Some((Cast { carrier, modifiers, payloads, then, mana, color, trails, bursts }, left))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runes() -> Runes {
        let file: RunesFile = crate::data::parse_ron(include_str!("../../../../assets/data/runes.ron")).unwrap();
        Runes::new(file).unwrap()
    }

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_wand_reads_left_to_right() {
        let r = runes();
        // A fireball: a trail and an orb carrying a blast and fire.
        let c = casts(&r, &ids(&["trail_fire", "orb", "blast", "ignite"])).unwrap();
        assert_eq!(c.len(), 1);
        assert!(matches!(c[0].carrier, Carrier::Orb { .. }));
        assert_eq!(c[0].payloads.len(), 2);
        assert!(c[0].trail().is_some());
        // Two casts in turn.
        assert_eq!(casts(&r, &ids(&["bolt", "spark", "gravity", "bolt", "acid"])).unwrap().len(), 2);
        // A trigger: the rest is cast where it lands.
        let c = casts(&r, &ids(&["trigger", "bolt", "spark", "orb", "blast"])).unwrap();
        assert_eq!(c.len(), 1);
        let then = c[0].then.as_ref().expect("sets off an orb");
        assert!(matches!(then.carrier, Carrier::Orb { .. }));
        assert!(c[0].total_mana() > c[0].mana);
        assert!(casts(&r, &ids(&["nonsense"])).is_err());
    }
}
