//! The bestiary (DESIGN §14.7): every creature there's a file for, as the
//! game knows it, in one place. The catalogue here is read from what the
//! game already has (the creature files, their kinds, `moves.ron`, their
//! art), nothing written twice; the panel (`panel.rs`, F12 or the arena
//! panel's "Bestiary") shows it as cards to filter and search, and a card
//! opened: its picture, its numbers, how it takes each kind of hurt, its
//! moves, what it carries, and buttons to place it, fight it, open its file
//! and reload.

pub mod cli;
pub mod panel;
pub mod stage;

use std::path::PathBuf;
use std::sync::Arc;

use crate::creatures::Team;
use crate::creatures::def::CreatureDef;
use crate::creatures::moves::{Act, MoveBook, MoveDef};
use crate::creatures::nature::{Cant, Harm};

/// What part a creature plays: the filter beside its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// It fights you.
    Foe,
    /// Wild life: it flees, or minds its own business.
    Critter,
    /// The village's people.
    Villager,
    /// Props, dummies, explosives: things with health.
    Thing,
}

impl Role {
    pub const ALL: [Role; 4] = [Role::Foe, Role::Critter, Role::Villager, Role::Thing];

    pub fn name(self) -> &'static str {
        match self {
            Role::Foe => "foe",
            Role::Critter => "critter",
            Role::Villager => "villager",
            Role::Thing => "thing",
        }
    }
}

/// One move, as the card lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct MoveLine {
    pub id: String,
    /// "0–36 cells, every 1.5 s"
    pub when: String,
    /// "windup 0.30 · strike 0.22 · recover 0.45"
    pub phases: String,
    /// "lunge, strike 16 pierce"
    pub does: String,
}

/// A creature as the bestiary shows it.
#[derive(Clone, Debug)]
pub struct Entry {
    /// Its file's name (what spawns it).
    pub id: String,
    pub name: String,
    /// Its kind (`kinds.ron`).
    pub kind: String,
    pub role: Role,
    /// Its box (cells), health, top speed (cells/s).
    pub size: (f32, f32),
    pub health: f32,
    pub speed: f32,
    pub poise: f32,
    pub heft: f32,
    /// Code of its own (`custom/`), and its brain.
    pub code: Option<String>,
    pub brain: String,
    pub weapon: Option<String>,
    /// Contact damage: "12 pierce, every 0.8 s".
    pub touch: Option<String>,
    pub moves: Vec<MoveLine>,
    /// Every kind of hurt it doesn't take as anyone would (×).
    pub profile: Vec<(Harm, f32)>,
    pub cant: Vec<&'static str>,
    /// "10/s, stopped 5 s by fire, acid; undying"
    pub regen: Option<String>,
    pub loot: Option<String>,
    pub drops: Vec<(String, u32)>,
    pub file: PathBuf,
    pub def: Arc<CreatureDef>,
}

impl Entry {
    /// Read from a creature's definition and the moves it names.
    pub fn of(id: &str, def: &Arc<CreatureDef>, book: &MoveBook) -> Entry {
        let role = match def.team {
            Team::Enemy => Role::Foe,
            Team::Villager => Role::Villager,
            _ if def.brain.kind == "critter" => Role::Critter,
            _ => Role::Thing,
        };
        let n = &def.nature;
        let profile = Harm::ALL.into_iter().filter(|h| *h != Harm::Fall && (n.of(*h) - 1.0).abs() > 1e-3).map(|h| (h, n.of(h))).collect();
        let cant = [(Cant::Burn, "burn"), (Cant::Chill, "be chilled"), (Cant::Poison, "be poisoned"), (Cant::Web, "be webbed"), (Cant::Stagger, "be staggered")]
            .into_iter()
            .filter(|(c, _)| n.cant(*c))
            .map(|(_, w)| w)
            .collect();
        let regen = def.regen.as_ref().map(|r| {
            let by: Vec<&str> = r.stopped_by.iter().map(|h| h.name()).collect();
            let stopped = if by.is_empty() { String::new() } else { format!(", stopped {:.0} s by {}", r.pause, by.join(", ")) };
            format!("{:.0}/s{stopped}{}", r.per_sec, if r.undying { "; can't die while it heals" } else { "" })
        });
        Entry {
            id: id.to_string(),
            name: def.name.clone(),
            kind: def.kind.clone().unwrap_or_else(|| "-".into()),
            role,
            size: def.size,
            health: def.health,
            speed: def.movement.run_speed,
            poise: def.poise,
            heft: def.heft,
            code: def.custom.as_ref().map(|c| c.name.clone()),
            brain: def.brain.kind.clone(),
            weapon: def.weapon.clone(),
            touch: def.touch.map(|t| format!("{:.0} {}, every {:.1} s", t.damage, t.harm.name(), t.every)),
            moves: def.moves.iter().map(|m| book.get(m).map_or_else(|| MoveLine { id: m.clone(), when: "(no such move)".into(), phases: String::new(), does: String::new() }, |d| move_line(d))).collect(),
            profile,
            cant,
            regen,
            loot: def.loot.clone(),
            drops: def.drops.clone(),
            file: crate::data::data_path("creatures").join(format!("{id}.ron")),
            def: def.clone(),
        }
    }

    /// Its card's details, a heading and lines each: what the expanded card
    /// and the command line show.
    pub fn details(&self) -> Vec<(&'static str, Vec<String>)> {
        let mut out = Vec::new();
        let mut body = vec![
            format!("{}, {}, {:.0} x {:.0} cells", self.kind, self.role.name(), self.size.0, self.size.1),
            format!("health {:.0}, speed {:.0} cells/s", self.health, self.speed),
        ];
        if self.poise > 0.0 || self.heft != 1.0 {
            body.push(format!("poise {:.0}, heft {:.1}", self.poise, self.heft));
        }
        let mut mind = format!("brain {}", self.brain);
        if let Some(c) = &self.code {
            mind += &format!(", code: {c}");
        }
        body.push(mind);
        out.push(("Body", body));
        let mut hurt: Vec<String> = Vec::new();
        if self.profile.is_empty() {
            hurt.push("takes every kind as anyone would".into());
        } else {
            let say = |m: f32| match m {
                m if m < 0.0 => format!("drinks it (heals {:.0} %)", -m * 100.0),
                0.0 => "shrugs it off".to_string(),
                m => format!("x{m:.2}"),
            };
            hurt.extend(self.profile.iter().map(|(h, m)| format!("{} {}", h.name(), say(*m))));
        }
        if !self.cant.is_empty() {
            hurt.push(format!("can't {}", self.cant.join(", ")));
        }
        if let Some(r) = &self.regen {
            hurt.push(format!("heals {r}"));
        }
        out.push(("Hurt", hurt));
        let mut attacks = Vec::new();
        if let Some(w) = &self.weapon {
            attacks.push(format!("wields {w}"));
        }
        if let Some(t) = &self.touch {
            attacks.push(format!("touch: {t}"));
        }
        for m in &self.moves {
            attacks.push(format!("{}: {}", m.id, m.when));
            if !m.phases.is_empty() {
                attacks.push(format!("  {}", m.phases));
            }
            if !m.does.is_empty() {
                attacks.push(format!("  {}", m.does));
            }
        }
        if attacks.is_empty() {
            attacks.push("none".into());
        }
        out.push(("Attacks", attacks));
        let mut carries = Vec::new();
        if let Some(l) = &self.loot {
            carries.push(format!("loot: {l}"));
        }
        carries.extend(self.drops.iter().map(|(i, n)| format!("{n} x {i}")));
        if !carries.is_empty() {
            out.push(("Carries", carries));
        }
        out
    }

    /// Whether a search finds it: in its id, name, kind, role or moves.
    pub fn matches(&self, search: &str) -> bool {
        let s = search.trim().to_lowercase();
        s.is_empty()
            || [self.id.as_str(), self.name.as_str(), self.kind.as_str(), self.role.name()].iter().any(|t| t.to_lowercase().contains(&s))
            || self.moves.iter().any(|m| m.id.contains(&s))
    }
}

fn move_line(m: &MoveDef) -> MoveLine {
    let (near, far) = m.when.range;
    let range = if far >= 1e8 { format!("from {near:.0} cells") } else { format!("{near:.0}-{far:.0} cells") };
    let mut needs = vec![range, format!("every {:.1} s", m.every)];
    if m.when.footing {
        needs.push("standing".into());
    }
    if m.when.line {
        needs.push("in sight".into());
    }
    let phases = m.phases.iter().map(|p| format!("{} {:.2}", if p.name.is_empty() { "-" } else { &p.name }, p.secs)).collect::<Vec<_>>().join(" > ");
    let does = m
        .phases
        .iter()
        .flat_map(|p| &p.acts)
        .filter_map(|a| match a {
            Act::Lunge { .. } => Some("lunge".to_string()),
            Act::Strike(s) => Some(format!("strike {:.0} {}{}", s.damage, s.harm.name(), s.coat.as_ref().map_or(String::new(), |c| format!(" + {c}")))),
            Act::Cast { spell, .. } => Some(format!("cast {spell}")),
            Act::Slam(s) => Some(format!("slam {:.0} {} round it", s.damage, s.harm.name())),
            Act::Summon { kind, count, .. } => Some(format!("summon {count} {kind}")),
            Act::Grab(_) => Some("grab".into()),
            Act::Throw { .. } => Some("throw".into()),
            Act::Beam { spell, .. } => Some(format!("beam {spell}")),
            Act::Sound(_) => None,
        })
        .collect::<Vec<_>>()
        .join(", ");
    MoveLine { id: m.id.clone(), when: needs.join(", "), phases, does }
}

/// Every creature but the player, by role, then name.
pub fn catalogue(creatures: &crate::creatures::def::Creatures, book: &MoveBook) -> Vec<Entry> {
    let mut all: Vec<Entry> = creatures.all().filter(|(id, _)| id.as_str() != "player").map(|(id, def)| Entry::of(id, def, book)).collect();
    all.sort_by(|a, b| (a.role, &a.name).cmp(&(b.role, &b.name)));
    all
}

/// A creature's portrait: the first frame of its idle (or any) clip,
/// trimmed to what's drawn.
pub fn portrait(def: &CreatureDef) -> Option<platypus_art::Pixels> {
    let art = def.rig.as_ref()?;
    let clip = art.clips.get("idle").or_else(|| art.clips.values().next());
    let frame = &art.frames[clip.and_then(|c| c.frames.first().copied()).unwrap_or(0)];
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, -1, -1);
    for y in 0..frame.h as i32 {
        for x in 0..frame.w as i32 {
            if frame.opaque(x, y) {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            }
        }
    }
    if x1 < 0 {
        return None;
    }
    let mut out = platypus_art::Pixels::new((x1 - x0 + 1) as u32, (y1 - y0 + 1) as u32);
    for y in y0..=y1 {
        for x in x0..=x1 {
            out.set(x - x0, y - y0, frame.get(x, y));
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_creature_has_a_card_with_its_numbers_and_a_picture() {
        let creatures = crate::creatures::def::Creatures::load_for_tests();
        let book = MoveBook::load();
        let all = catalogue(&creatures, &book);
        assert!(all.len() >= 25, "{} creatures", all.len());
        assert!(all.iter().all(|e| e.id != "player"));
        for e in &all {
            assert!(portrait(&e.def).is_some(), "{} has a picture", e.id);
            assert!(e.file.exists(), "{} has its file", e.id);
            assert!(!e.details().is_empty());
        }
        let troll = all.iter().find(|e| e.id == "troll").expect("the troll");
        assert_eq!(troll.role, Role::Foe);
        assert!(troll.regen.as_deref().is_some_and(|r| r.contains("fire")), "{:?}", troll.regen);
        assert!(troll.moves.iter().any(|m| m.id == "troll_grab" && m.does.contains("grab")));
        let spider = all.iter().find(|e| e.id == "spider").expect("the spider");
        assert!(spider.profile.iter().any(|(h, m)| *h == Harm::Acid && *m < 0.0), "the spider drinks acid");
        assert!(spider.matches("bite") && spider.matches("Insect") && !spider.matches("troll"));
        assert!(all.iter().any(|e| e.role == Role::Critter) && all.iter().any(|e| e.role == Role::Villager) && all.iter().any(|e| e.role == Role::Thing));
    }
}
