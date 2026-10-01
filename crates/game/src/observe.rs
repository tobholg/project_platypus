//! What the player has seen each kind of creature do (DESIGN §14.7, the
//! player's bestiary, later): recorded from now, kept with its progress
//! (`Progress::observed`, saved). Only what happens within sight
//! (`progress::WITNESS`) counts, from anyone's blows, spells and fires:
//! how many it has met, the moves it saw them make and what they carried,
//! how they took each kind of hurt (what was meant and what got through:
//! hurt, shrugged off, drunk in), what finished the ones that died, and
//! whether it saw one heal and what stopped that.
//!
//! The facts, not the words: the bestiary writes "It drank in the acid",
//! "Arrows did little", "The flames stopped its wounds closing" from these.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::creatures::moves::Began;
use crate::creatures::nature::Regenerates;
use crate::creatures::player::LocalPlayer;
use crate::creatures::{Creature, Health, Kinematics, Took};
use crate::progress::{Progress, WITNESS};

/// What the player has seen of one kind.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Seen {
    /// How many it has met (each one once).
    pub met: u32,
    /// The moves it saw them begin (by id), how often.
    pub moves: BTreeMap<String, u32>,
    /// What it saw them carry (weapons by id).
    pub wields: BTreeSet<String>,
    /// How they took each kind of hurt it saw land (by its name).
    pub took: BTreeMap<String, Taken>,
    /// What finished the ones it saw die: the kind that did the most in the
    /// last moment, by its name, how often.
    pub felled: BTreeMap<String, u32>,
    /// It saw one close its wounds; the kinds of hurt it saw stop that.
    pub healed: bool,
    pub stopped: BTreeSet<String>,
}

/// One kind of hurt as it landed on them, added up: what was meant and
/// what got through (below 0: drunk in).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Taken {
    pub meant: f32,
    pub dealt: f32,
}

impl Taken {
    /// How it seems to take it, as the hurt is shown (`hurt::reaction`).
    pub fn reaction(&self) -> crate::creatures::body::hurt::Reaction {
        crate::creatures::body::hurt::reaction(self.meant, self.dealt)
    }
}

/// Already counted as met.
#[derive(Component)]
pub struct Noticed;

type Onlooker<'a> = (&'a Kinematics, &'a mut Progress);
type Watched<'a> = (Entity, &'a Creature, &'a Kinematics, &'a Health, Option<&'a crate::combat::Wielding>, Option<&'a Regenerates>, Has<Noticed>);

/// Note what the player sees: run between the tally and deaths (the dying
/// are still there to be seen).
pub(crate) fn observe(
    mut commands: Commands,
    pending: Option<Res<crate::save::PendingPlayer>>,
    mut began: MessageReader<Began>,
    mut took: MessageReader<Took>,
    mut player: Query<Onlooker, With<LocalPlayer>>,
    creatures: Query<Watched, Without<LocalPlayer>>,
) {
    let (None, Ok((k, mut p))) = (pending, player.single_mut()) else {
        began.clear();
        took.clear();
        return;
    };
    let me = k.body.pos;
    let near = |c: &Kinematics| c.body.pos.distance(me) <= WITNESS;
    // (Written to without a change: nothing reacts to it.)
    let seen = &mut p.bypass_change_detection().observed;
    for b in began.read() {
        if let Ok((_, c, ck, ..)) = creatures.get(b.who)
            && near(ck)
        {
            *seen.entry(c.kind.clone()).or_default().moves.entry(b.id.clone()).or_default() += 1;
        }
    }
    let mut last: Vec<(Entity, crate::creatures::Harm, f32)> = Vec::new();
    for t in took.read() {
        let Ok((_, c, ck, _, _, regen, _)) = creatures.get(t.target) else { continue };
        if !near(ck) {
            continue;
        }
        let s = seen.entry(c.kind.clone()).or_default();
        let taken = s.took.entry(t.harm.name().to_string()).or_default();
        taken.meant += t.meant;
        taken.dealt += t.dealt;
        if regen.is_some_and(|r| r.stopped_by & t.harm.bit() != 0) {
            s.stopped.insert(t.harm.name().to_string());
        }
        if t.killed {
            match last.iter_mut().find(|l| l.0 == t.target) {
                Some(l) if l.2 < t.dealt => *l = (t.target, t.harm, t.dealt),
                Some(_) => {}
                None => last.push((t.target, t.harm, t.dealt)),
            }
        }
    }
    for (e, harm, _) in last {
        if let Ok((_, c, ..)) = creatures.get(e) {
            *seen.entry(c.kind.clone()).or_default().felled.entry(harm.name().to_string()).or_default() += 1;
        }
    }
    for (e, c, ck, h, wielding, regen, noticed) in &creatures {
        if !near(ck) {
            continue;
        }
        let s = seen.entry(c.kind.clone()).or_default();
        if !noticed {
            s.met += 1;
            commands.entity(e).insert(Noticed);
        }
        if let Some(id) = wielding.and_then(|w| w.0.as_ref())
            && !s.wields.contains(id)
        {
            s.wields.insert(id.clone());
        }
        if !s.healed && regen.is_some_and(|r| r.waiting <= 0.0) && h.hp > 0.0 && h.hp < h.max {
            s.healed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seen_reads_back_as_written_and_tells_how_it_took_it() {
        use crate::creatures::body::hurt::Reaction;
        let mut s = Seen { met: 2, healed: true, ..default() };
        s.moves.insert("spider_bite".into(), 3);
        s.took.insert("acid".into(), Taken { meant: 10.0, dealt: -4.0 });
        s.took.insert("pierce".into(), Taken { meant: 10.0, dealt: 3.0 });
        s.took.insert("fire".into(), Taken { meant: 10.0, dealt: 15.0 });
        s.felled.insert("fire".into(), 1);
        s.stopped.insert("fire".into());
        let text = ron::to_string(&s).expect("writes");
        let back: Seen = ron::from_str(&text).expect("reads");
        assert_eq!(back, s);
        assert_eq!(s.took["acid"].reaction(), Reaction::Absorbed);
        assert_eq!(s.took["pierce"].reaction(), Reaction::Resisted);
        assert_eq!(s.took["fire"].reaction(), Reaction::Hurt);
        // (An old save, with none of it, reads too.)
        let old: Progress = ron::from_str("(seen: [], deepest: 3)").expect("an old progress reads");
        assert!(old.observed.is_empty());
    }
}
