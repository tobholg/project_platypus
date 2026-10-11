//! Getting up again (a skeleton, `rises` in its creature file).
//!
//! Killed, it's gone, and its bones lie where it fell: a body (`corpses`),
//! holding only what it wore and held (its loot is rolled when it's dead
//! for good), with health of its own (`PILE_SHARE` of the creature's) and
//! its kind's weaknesses, made worse (`weak`: blunt and radiant, times
//! `PILE_WEAK`): a hammer breaks a heap in a blow or two, a sword in a few.
//! Broken, it dies as anything dies: a body with its loot, its gold, the
//! kill counted.
//!
//! Left alone `after` seconds, it gets up: the last `GLOW` of them a cold
//! light grows in the bones (a real light: a heap waking shows across a
//! dark cave), wisps rise and the ash about it is drawn in; then it stands
//! as a new creature of its kind, at `health` of its health, wearing and
//! holding what's still in the heap (take its sword and it gets up with
//! its bare fists, `unarmed`; give it a better one...), anything else in it
//! spilled. Its
//! bones fly together from the heap, feet first (`ASSEMBLE`), and its eyes
//! light last.

use bevy::prelude::*;
use platypus_physics::body::overlaps_solid;
use platypus_sim::CellPos;
use serde::Deserialize;

use crate::creatures::nature::Harm;
use crate::creatures::{Creature, Died, Dormant, Health, Kinematics, WorldGrid};
use crate::hands::chests::Chests;
use crate::hands::corpses::Corpse;
use crate::hands::items::{Items, Stack, Use};
use crate::magic::runes::Emitter;
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// A heap's health, of the creature's.
const PILE_SHARE: f32 = 0.4;
/// How much worse a heap's weaknesses are than the creature's.
const PILE_WEAK: f32 = 1.6;
/// The light grows this long before it stands (seconds).
const GLOW: f32 = 1.5;
/// The bones fly together this long: each in `FLIGHT` of it, the feet's
/// first, the skull's last.
const ASSEMBLE: f32 = 1.8;
const FLIGHT: f32 = 0.7;
/// How high a bone arcs on its way, and how far round it swirls (cells).
const ARC: f32 = 6.0;
const SWIRL: f32 = 4.0;
/// A body getting up jerks this much as it heaves (of the way up).
const JERK: f32 = 0.35;
/// Loose bones this near a heap as it gets up fly back to it (cells).
const GATHER: f32 = 48.0;
/// Headless, it puts its skull back on this near it (cells).
const REACH_SKULL: f32 = 8.0;
/// Standing where it can't (under a low roof), it tries again after this;
/// past `STUCK_MOST` of trying, it stands anyway. Room is looked for this
/// far up (on its ash) and to each side (cells).
const RETRY: f32 = 0.5;
const STUCK_MOST: f32 = 3.0;
const ROOM_UP: i32 = 8;
const ROOM_ASIDE: [f32; 9] = [0.0, 3.0, -3.0, 6.0, -6.0, 10.0, -10.0, 15.0, -15.0];
/// The waking light (`RiseDef::glow`) at its brightest, of its colour; the
/// light when there's none to go by.
const LIGHT_MOST: f32 = 1.6;
const LIGHT: [f32; 3] = [0.35, 1.0, 0.75];
/// A body's darkening (as `corpses` darkens one).
const DIM: f32 = 0.72;
/// Ash drawn back in: cells a tick at most, from this far.
const ASH_PER_TICK: usize = 2;
const ASH_REACH: i32 = 14;
/// Its eyes' light as they catch, and how fast it fades.
const EYES_FLASH: f32 = 2.2;
const EYES_FADE: f32 = 3.0;

/// How it gets up again (a creature file's `rises`).
#[derive(Clone, Debug, Deserialize)]
pub struct RiseDef {
    /// Seconds a heap before it stands.
    pub after: f32,
    /// Of its health it stands with.
    pub health: f32,
    /// The only kinds of hurt that break its bones (a heap, a loose skull):
    /// anything else glances off (a skeleton: blunt; a blade never kills it
    /// for good).
    #[serde(default, alias = "weak")]
    pub breaks: Vec<Harm>,
    /// Its head is what holds it together: cut off, it's a skull of its
    /// own, found again as it gets up (and put back on, gone to it if it's
    /// out of reach); broken (only by `breaks`), it never gets up again.
    #[serde(default)]
    pub anchor: bool,
    /// What it leaves as it falls: rubble (bones that come apart: a
    /// skeleton) or its body, whole (that gets up again: a zombie).
    #[serde(default)]
    pub remains: Remains,
    /// The light that grows in it as it wakes (and in its wisps, and the
    /// glow on what flies back together).
    #[serde(default = "glow")]
    pub glow: (u8, u8, u8),
    /// How many times it gets up (none: every time; a zombie: once).
    #[serde(default)]
    pub times: Option<u32>,
}

/// What one that rises leaves as it falls (`RiseDef::remains`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum Remains {
    #[default]
    Rubble,
    Body,
}

fn glow() -> (u8, u8, u8) {
    (90, 255, 190)
}

impl RiseDef {
    /// Its waking light, as light (0..1 a channel).
    pub fn light(&self) -> [f32; 3] {
        let (r, g, b) = self.glow;
        [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
    }
}

#[derive(Component, Clone)]
pub struct Rises(pub RiseDef);

/// Who a creature that rises is, through every time it falls and gets up
/// (its first number): what its loose bones belong to.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lineage(pub u64);

/// A loose bone of one that rises (a limb cut off it): whose, and which.
#[derive(Component, Clone, Copy, Debug)]
pub struct Bone {
    pub lineage: u64,
    pub limb: super::parts::Limb,
}

/// How many times it's got up (`RiseDef::times`).
#[derive(Component, Clone, Copy, Debug)]
pub struct Risen(pub u32);

/// Its skull: what holds it together (`RiseDef::anchor`).
#[derive(Component)]
pub struct Skull;

/// Headless, it goes to its skull to put it back on.
#[derive(Component)]
pub struct Seeking;

/// A heap of bones that will get up: what it was, how long it has left.
#[derive(Component)]
pub struct Pile {
    lineage: u64,
    /// What it had lost as it fell (found again as it gets up, if near).
    lost: Vec<super::parts::Limb>,
    kind: String,
    def: std::sync::Arc<crate::creatures::def::CreatureDef>,
    rise: RiseDef,
    left: f32,
    /// How long it's waited for room to stand.
    stuck: f32,
    facing: f32,
    sprite: Option<(Sprite, Vec3)>,
    /// Times it's got up before.
    risen: u32,
}

/// Bones flying together into a creature just stood up (it's hidden until
/// they're there).
#[derive(Component)]
pub struct Assembling {
    t: f32,
    /// Each pixel: from (in the heap), to (in it), its colour; offsets
    /// from its middle.
    pixels: Vec<(Vec2, Vec2, [f32; 4])>,
    eyes: [f32; 3],
    /// Its waking light (the glow on what flies together).
    light: [f32; 3],
    /// Where its heap's bones lay (the world), to fly from.
    from: Vec<Vec2>,
    /// A body getting up whole (`Remains::Body`): its pixels go from where
    /// they lie, its feet's end to its feet, along the line from there to
    /// its head (the world); no arc, no swirl, no glow.
    whole: Option<(Vec2, Vec2)>,
    /// What it gets up without (not in the picture that forms).
    lost: Vec<super::parts::Limb>,
}

/// Its eyes just caught: a light that fades.
#[derive(Component)]
pub struct EyesFlash(f32);

/// Stood up already dressed: not dressed again (`gear::outfit`).
#[derive(Component)]
pub struct Outfitted;

type Falling<'a> = (
    Entity,
    &'a mut Health,
    &'a Kinematics,
    &'a Rises,
    &'a Creature,
    &'a super::animation::Animator,
    Option<&'a crate::gear::Equipment>,
    Option<&'a Children>,
    Option<&'a super::hurt::Bleeds>,
    Option<&'a super::tumble::Tumble>,
    Option<&'a super::parts::Severed>,
    (Option<&'a Lineage>, Option<&'a crate::creatures::Stable>),
    Option<&'a super::marks::Marks>,
    Option<&'a crate::creatures::Gibbed>,
    (Option<&'a super::parts::Parts>, Option<&'a Risen>),
);

/// One that rises, killed, leaves its bones (a body that gets up:
/// `make_pile`) and goes, with no death of its own. Just before deaths.
pub fn collapse(mut commands: Commands, mut sim: ResMut<SimWorld>, mut died: MessageWriter<Died>, mut q: Query<Falling>, sprites: Query<(&Sprite, &Transform), With<super::animation::CreatureSprite>>) {
    for (e, mut h, k, rises, creature, anim, eq, children, bleeds, spin, severed, (lineage, stable), marks, gib, (parts, risen)) in &mut q {
        if h.hp > 0.0 {
            continue;
        }
        // (Not this time: its head off, if that kills it; a body blown
        // apart; or got up as often as it does. It dies as anything does.)
        let beheaded = anim.def.headless == super::wounds::Headless::Dies && severed.is_some_and(|s| s.has(super::parts::Limb::Head));
        let blown = rises.0.remains == Remains::Body && gib.is_some();
        if beheaded || blown || rises.0.times.is_some_and(|t| risen.map_or(0, |r| r.0) >= t) {
            continue;
        }
        // (Kept from deaths: it's gone now.)
        h.hp = 1.0;
        let sprite = children.and_then(|ch| ch.iter().find_map(|c| sprites.get(c).ok())).map(|(s, tf)| (s.clone(), tf.translation));
        let worn = eq.map(|eq| eq.worn.iter().flatten().chain(eq.held.iter()).copied().collect()).unwrap_or_default();
        let limbs = parts.map(|p| p.limbs().into_iter().filter(|l| l.limb.is_none_or(|x| !severed.is_some_and(|c| c.has(x)))).collect()).unwrap_or_default();
        died.write(Died { kind: creature.kind.clone(), def: anim.def.clone(), body: k.body, facing: k.loco.facing, sprite, worn, spin: spin.copied().unwrap_or_default(), rising: Some(rises.0.clone()), risen: risen.map_or(0, |r| r.0), limbs, severed: severed.map(|s| s.0.clone()).unwrap_or_default(), lineage: crate::creatures::lineage_of(Some((lineage, stable))), marks: marks.map(|m| m.list.clone()).unwrap_or_default(), gib: gib.map(|g| g.0) });
        if let Some(&super::hurt::Bleeds(dust)) = bleeds {
            sim.world.splash([k.body.pos.x, k.body.pos.y], dust, 14, 2.4);
        }
        commands.entity(e).despawn();
    }
}

/// The body of one that will get up (`corpses::lay_out`, for a `rising`
/// death): health of its own, its weaknesses worse, hit as a creature is.
pub fn make_pile(e: &mut EntityCommands, d: &Died, rise: &RiseDef, dust: Option<platypus_sim::MaterialId>) {
    // (Only what breaks bones hurts it, worse than it hurt the creature;
    // with nothing named, anything does, as it did: a body.)
    let nature = if rise.breaks.is_empty() { d.def.nature } else { crate::creatures::nature::Nature::only(&rise.breaks, |h| d.def.nature.of(h) * PILE_WEAK) };
    let max = (d.def.health * PILE_SHARE).max(1.0);
    e.insert((
        Pile { lineage: d.lineage, lost: d.severed.clone(), kind: d.kind.clone(), def: d.def.clone(), rise: rise.clone(), left: rise.after, stuck: 0.0, facing: d.facing, sprite: d.sprite.clone(), risen: d.risen },
        Health { nature, ..Health::new(max) },
        crate::creatures::MoveStats(d.def.movement.clone()),
        d.def.team,
        Name::new(format!("Bones of {}", d.def.name)),
    ));
    if let Some(m) = dust {
        e.insert(super::hurt::Bleeds(m));
    }
}

type Breaking<'a> = (Entity, &'a mut Health, &'a Kinematics, &'a Pile, &'a Corpse, Option<&'a super::hurt::Bleeds>);

/// A heap broken (its health gone): dead for good. Its body (with what's
/// still in it, its loot, its gold) and the kill, as any death; a burst of
/// bone. Just before deaths.
#[allow(clippy::too_many_arguments)]
pub fn shatter(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<Items>>,
    mut chests: ResMut<Chests>,
    mut died: MessageWriter<Died>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut q: Query<Breaking>,
) {
    let Some(items) = items else { return };
    for (e, mut h, k, pile, corpse, bleeds) in &mut q {
        if h.hp > 0.0 {
            continue;
        }
        h.hp = 1.0;
        let worn: Vec<Stack> = chests.contents(corpse.key, &sim.world, &items).slots.iter().flatten().copied().collect();
        chests.forget(corpse.key);
        died.write(Died { kind: pile.kind.clone(), def: pile.def.clone(), body: k.body, facing: pile.facing, sprite: pile.sprite.clone(), worn, spin: default(), rising: None, risen: pile.risen, limbs: Vec::new(), severed: pile.lost.clone(), lineage: pile.lineage, marks: Vec::new(), gib: None });
        // (Bones burst apart; a body bursts in what it bleeds.)
        let bones = pile.rise.remains == Remains::Rubble;
        if bones {
            let burst = bone_burst();
            sparks.emit(&burst, burst.count as usize, k.body.pos, Vec2::Y, Vec2::ZERO);
        }
        sounds.write(if bones { crate::sound::PlaySound::at("break", k.body.pos).pitch(1.3) } else { crate::sound::PlaySound::at("snap", k.body.pos).pitch(0.6) });
        if let Some(&super::hurt::Bleeds(dust)) = bleeds {
            sim.world.splash([k.body.pos.x, k.body.pos.y], dust, if bones { 18 } else { 32 }, 2.8);
        }
        commands.entity(e).despawn();
    }
}

/// Bone flying apart.
fn bone_burst() -> Emitter {
    Emitter { count: 26.0, life: (0.25, 0.6), colors: vec![(232, 226, 205), (190, 182, 160), (140, 134, 120)], speed: 110.0, spread: 1.4, gravity: 300.0, drag: 0.5, size: 1.0, jitter: 0.3, glow: false }
}

/// A wisp rising off what's waking, in its light.
fn wisp(light: [f32; 3]) -> Emitter {
    let c = |k: f32| ((light[0] * 255.0 * k) as u8, (light[1] * 255.0 * k) as u8, (light[2] * 255.0 * k) as u8);
    Emitter { count: 1.0, life: (0.4, 0.9), colors: vec![c(1.0), c(0.75)], speed: 22.0, spread: 0.5, gravity: -20.0, drag: 0.6, size: 1.0, jitter: 0.4, glow: true }
}

/// A grain of what it bled drawn in, in its light.
fn streak(light: [f32; 3]) -> Emitter {
    let c = ((light[0] * 255.0) as u8, (light[1] * 255.0) as u8, (light[2] * 255.0) as u8);
    Emitter { count: 1.0, life: (0.1, 0.2), colors: vec![(150, 145, 140), c], speed: 70.0, spread: 0.1, gravity: 0.0, drag: 0.0, size: 1.0, jitter: 0.0, glow: true }
}

type Waking<'a> = (Entity, &'a mut Pile, &'a Kinematics, &'a Corpse, Option<&'a mut crate::light::LightSource>, Option<&'a super::ragdoll::Limp>);

type Crushed<'a> = (Entity, &'a Bone, &'a Health, &'a Kinematics);
type Undone<'a> = (Entity, Option<&'a Lineage>, Option<&'a crate::creatures::Stable>, &'a mut Health);

/// A skull broken (only what breaks its bones can): its owner never gets up
/// again. Walking headless, it falls for good; a heap, it's broken; its
/// other loose bones fade. Just before deaths.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn crush(
    mut commands: Commands,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    skulls: Query<Crushed, With<Skull>>,
    mut owners: Query<Undone, (With<Rises>, Without<Pile>, Without<Skull>)>,
    mut piles: Query<(&Pile, &mut Health), (Without<Rises>, Without<Skull>)>,
    mut loose: Query<(&Bone, &mut super::wounds::CutPiece), Without<Skull>>,
) {
    for (e, bone, h, k) in &skulls {
        if h.hp > 0.0 {
            continue;
        }
        let burst = bone_burst();
        sparks.emit(&burst, burst.count as usize, k.body.pos, Vec2::Y, Vec2::ZERO);
        sounds.write(crate::sound::PlaySound::at("break", k.body.pos).pitch(1.5));
        commands.entity(e).despawn();
        for (o, l, s, mut oh) in &mut owners {
            if crate::creatures::lineage_of(Some((l, s))) == bone.lineage {
                commands.entity(o).remove::<Rises>();
                oh.hp = oh.hp.min(0.0);
            }
        }
        for (p, mut ph) in &mut piles {
            if p.lineage == bone.lineage {
                ph.hp = ph.hp.min(0.0);
            }
        }
        for (b, mut c) in &mut loose {
            if b.lineage == bone.lineage {
                c.left = c.left.min(super::wounds::PIECE_FADE);
            }
        }
    }
}

/// Headless, it goes to its skull and puts it back on: its head again, its
/// sight, its eyes catching light.
#[allow(clippy::type_complexity)]
pub fn seek(
    mut commands: Commands,
    mut seekers: Query<(Entity, &Kinematics, &Lineage, &mut crate::creatures::Controls, Option<&super::parts::Severed>, &super::animation::Animator), With<Seeking>>,
    skulls: Query<(Entity, &Bone, &super::ragdoll::Limp), With<Skull>>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    for (e, k, lineage, mut controls, severed, anim) in &mut seekers {
        let Some((s, _, limp)) = skulls.iter().find(|(_, b, _)| b.lineage == lineage.0) else {
            commands.entity(e).remove::<Seeking>();
            continue;
        };
        let at = limp.pieces[limp.root].pos;
        let d = at - k.body.pos;
        if d.length() <= REACH_SKULL + k.body.half.y {
            // Put back on.
            commands.entity(s).despawn();
            let kept: Vec<super::parts::Limb> = severed.map(|sv| sv.0.iter().copied().filter(|l| *l != super::parts::Limb::Head).collect()).unwrap_or_default();
            let eyes = anim.def.eyes.map(|(r, g, b)| [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]).or_else(|| anim.def.rises.as_ref().map(|r| r.light())).unwrap_or(LIGHT);
            commands.entity(e).remove::<(Seeking, super::wounds::Blind)>().insert((super::parts::Severed(kept), EyesFlash(1.0), crate::light::LightSource { color: eyes.map(|c| c * EYES_FLASH), flicker: 0.0 }));
            sounds.write(crate::sound::PlaySound::at("clank", k.body.pos).pitch(0.7));
            continue;
        }
        // (Its brain's steering overruled: to the skull, a jump if it's up.)
        controls.0.move_x = d.x.signum();
        controls.0.jump = d.y > 12.0 && d.x.abs() < 30.0;
    }
}

/// A heap's time runs: the light grows in it the last `GLOW`, wisps rise,
/// the ash about it is drawn in; then it stands (if there's room), a new
/// one of its kind dressed in what's left in it, and the heap goes.
#[allow(clippy::too_many_arguments)]
pub fn stand(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<Items>>,
    mut chests: ResMut<Chests>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut q: Query<Waking>,
    bones: Query<(Entity, &Bone, &super::ragdoll::Limp, Has<Skull>), Without<Pile>>,
) {
    let Some(items) = items else { return };
    for (e, mut pile, k, corpse, light, limp) in &mut q {
        pile.left -= DT;
        let pos = k.body.pos;
        if pile.left < GLOW {
            let glow = (1.0 - pile.left.max(0.0) / GLOW).powf(1.5);
            let color = pile.rise.light().map(|c| c * LIGHT_MOST * glow);
            match light {
                Some(mut l) => l.color = color,
                None => {
                    commands.entity(e).insert(crate::light::LightSource { color, flicker: 0.0 });
                }
            }
            if sim.world.tick().is_multiple_of(3) {
                let at = pos + Vec2::new((sim.world.tick() % 7) as f32 - 3.0, 0.0);
                sparks.emit(&wisp(pile.rise.light()), 1, at, Vec2::Y, Vec2::ZERO);
            }
            let blood = sim.world.materials().id(&pile.def.blood);
            draw_in(&mut sim, &mut sparks, pos, blood, pile.rise.light());
        }
        if pile.left > 0.0 {
            continue;
        }
        // Standing: room for its box on the ground under the heap, or a
        // little up (on its own ash) or to a side; none for long, it stands
        // anyway (and is pushed out of what it's in).
        let Some(feet) = room_to_stand(&sim, pos, pile.def.size).or_else(|| (pile.stuck >= STUCK_MOST).then(|| ground_under(&sim, pos))) else {
            pile.left = RETRY;
            pile.stuck += RETRY;
            continue;
        };
        // Its loose bones near enough fly back with the rest (its skull
        // last: its pixels go to the top); what's out of reach it gets up
        // without (headless, it goes to its skull).
        let mut from: Vec<(Vec2, [u8; 4])> = limp.map(|l| l.world_pixels()).unwrap_or_default();
        let mut found = Vec::new();
        let mut skull_far = false;
        for (b, bone, bl, skull) in &bones {
            if bone.lineage != pile.lineage {
                continue;
            }
            if bl.pieces[bl.root].pos.distance(pos) <= GATHER {
                from.extend(bl.world_pixels());
                found.push(bone.limb);
                commands.entity(b).despawn();
            } else if skull {
                skull_far = true;
            }
        }
        let missing: Vec<super::parts::Limb> = pile.lost.iter().copied().filter(|l| !found.contains(l)).collect();
        let headless = missing.contains(&super::parts::Limb::Head);
        // Dressed in what's left in it; the rest spills (and what its lost
        // arm held).
        let left: Vec<Stack> = chests.contents(corpse.key, &sim.world, &items).slots.iter().flatten().copied().collect();
        chests.forget(corpse.key);
        let mut eq = crate::gear::Equipment::default();
        let armless = missing.contains(&super::parts::Limb::Arm(0));
        for s in left {
            let def = items.def(s.item);
            let weapon = matches!(def.use_, Use::Melee(_) | Use::Bow(_)) && def.gear.is_some();
            if weapon && eq.held.is_none() && !armless {
                eq.held = Some(s);
            } else if let Some(i) = def.gear.as_ref().and(eq.slot_for(&items, &s)) {
                eq.worn[i] = Some(s);
            } else {
                crate::hands::spawn_drop(&mut commands, &items, pos, s);
            }
        }
        // (Nothing in its hand: what it fights with bare, its fists.)
        let wielding = eq
            .held
            .and_then(|s| match &items.def(s.item).use_ {
                Use::Melee(id) | Use::Bow(id) => Some(id.clone()),
                _ => None,
            })
            .or_else(|| pile.def.unarmed.clone())
            .filter(|_| !(armless && missing.contains(&super::parts::Limb::Arm(1))))
            .or_else(|| pile.def.armless.clone().filter(|_| armless && missing.contains(&super::parts::Limb::Arm(1))));
        let (share, facing) = (pile.rise.health, pile.facing);
        let (lineage, seek, risen) = (pile.lineage, headless && skull_far, pile.risen + 1);
        let light = pile.rise.light();
        let eyes = pile.def.eyes.map_or(light, |(r, g, b)| [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]);
        // (Its bones fly up from where they lie; a body gets up from how
        // it lies, its feet's end first.)
        let whole = (pile.rise.remains == Remains::Body).then(|| lying(limp, pos)).flatten();
        let from: Vec<Vec2> = from.into_iter().map(|(p, _)| p).collect();
        let missing_now = missing.clone();
        crate::creatures::def::spawn_creature(&mut commands, &pile.kind, feet, move |c| {
            c.insert((Lineage(lineage), Risen(risen)));
            if !missing.is_empty() {
                c.insert(super::parts::Severed(missing));
            }
            if headless {
                c.insert(super::wounds::Blind);
                if seek {
                    c.insert(Seeking);
                }
            }
            if let Some(mut h) = c.get_mut::<Health>() {
                h.hp = h.max * share;
            }
            if let Some(mut k) = c.get_mut::<Kinematics>() {
                k.loco.facing = facing;
            }
            let glow = crate::light::LightSource { color: light.map(|c| c * LIGHT_MOST), flicker: 0.0 };
            c.insert((eq, crate::combat::Wielding(wielding), Outfitted, Assembling { t: 0.0, pixels: Vec::new(), eyes, light, from, whole, lost: missing_now }, Dormant, Visibility::Hidden, glow));
        });
        commands.entity(e).despawn();
    }
}

/// How a body lies: its feet's end and the way to its head (the world),
/// from its pieces (its head's, its legs'), or along it, its lower end.
fn lying(limp: Option<&super::ragdoll::Limp>, at: Vec2) -> Option<(Vec2, Vec2)> {
    let limp = limp?;
    let head = limp.pieces.iter().find(|p| p.name.contains("head")).map(|p| p.pos);
    let legs: Vec<Vec2> = limp.pieces.iter().filter(|p| p.name == "limb").map(|p| p.pos).collect();
    let torso = limp.pieces[limp.root].pos;
    let feet = if legs.is_empty() { None } else { Some(legs.iter().sum::<Vec2>() / legs.len() as f32) };
    let (from, to) = match (feet, head) {
        (Some(f), Some(h)) => (f, h),
        (Some(f), None) => (f, torso),
        (None, Some(h)) => (torso - (h - torso), h),
        (None, None) => (at, at + Vec2::Y),
    };
    let axis = (to - from).normalize_or(Vec2::Y);
    Some((from, axis))
}

/// The ground under `at` (out of the ground first, if `at` is in it): where
/// feet would stand.
fn ground_under(sim: &SimWorld, at: Vec2) -> Vec2 {
    let grid = WorldGrid(&sim.world);
    let cell = CellPos::from_world(at.x, at.y);
    let mut y = cell.y;
    while y < cell.y + 12 && platypus_physics::Grid::solid(&grid, cell.x, y) {
        y += 1;
    }
    while y > cell.y - 40 && !platypus_physics::Grid::solid(&grid, cell.x, y - 1) {
        y -= 1;
    }
    Vec2::new(at.x, y as f32)
}

/// Where a body this big could stand up from a heap at `at`: on the ground
/// under it, or a little higher, or on the ground a little to a side;
/// the nearest that's clear.
fn room_to_stand(sim: &SimWorld, at: Vec2, (w, h): (f32, f32)) -> Option<Vec2> {
    let grid = WorldGrid(&sim.world);
    for dx in ROOM_ASIDE {
        let ground = ground_under(sim, at + Vec2::new(dx, 0.0));
        // (Not off a ledge: to the side only where the ground's near.)
        if (ground.y - at.y).abs() > h {
            continue;
        }
        for up in 0..=ROOM_UP {
            let feet = ground + Vec2::new(0.0, up as f32);
            let standing = platypus_physics::Body::new(feet + Vec2::new(0.0, h / 2.0 + 0.05), Vec2::new(w, h));
            let (lo, hi) = standing.cells_at(standing.pos);
            if !overlaps_solid(&grid, lo, hi) {
                return Some(feet);
            }
        }
    }
    None
}

/// What it bled (a skeleton's ash) near a waking heap drifts back into it,
/// a grain or two a tick.
fn draw_in(sim: &mut SimWorld, sparks: &mut crate::vfx::Sparks, at: Vec2, blood: Option<platypus_sim::MaterialId>, light: [f32; 3]) {
    let Some(ash) = blood else { return };
    let c = CellPos::from_world(at.x, at.y);
    let turn = sim.world.tick() as f32 * 0.37;
    let mut taken = 0;
    'look: for r in 2..=ASH_REACH {
        for i in 0..8 {
            let a = (i as f32 + turn) * std::f32::consts::TAU / 8.0;
            let p = CellPos::new(c.x + (a.cos() * r as f32) as i32, c.y + (a.sin() * r as f32 * 0.5) as i32);
            if sim.world.get(p).is_some_and(|cell| cell.material == ash) {
                sim.world.set(p, platypus_sim::Cell::AIR);
                let from = Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5);
                sparks.emit(&streak(light), 1, from, at - from, Vec2::ZERO);
                taken += 1;
                if taken >= ASH_PER_TICK {
                    break 'look;
                }
            }
        }
    }
}

type Gathering<'a> = (Entity, &'a mut Assembling, &'a Kinematics, &'a super::animation::Animator, Option<&'a super::hurt::Bleeds>);

/// Bones flying together: each pixel of its picture from somewhere in the
/// heap to its place, feet first, the skull last; then it's there (shown,
/// awake), its eyes catch light, a puff of ash.
#[allow(clippy::too_many_arguments)]
pub fn assemble(
    mut commands: Commands,
    time: Res<Time>,
    mut sim: ResMut<SimWorld>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut q: Query<Gathering>,
    mut flashes: Query<(Entity, &mut EyesFlash, &mut crate::light::LightSource), Without<Assembling>>,
) {
    let dt = time.delta_secs();
    for (e, mut a, k, anim, bleeds) in &mut q {
        if a.pixels.is_empty() {
            let from = std::mem::take(&mut a.from);
            a.pixels = pixels_of(anim, k, &from, a.whole, &a.lost);
            let sound = if a.whole.is_some() { crate::sound::PlaySound::at("snap", k.body.pos).pitch(0.5).volume(0.5) } else { crate::sound::PlaySound::at("clank", k.body.pos).pitch(0.55).volume(0.7) };
            sounds.write(sound);
        }
        // (A body: no arc, no swirl, unlit; all of it at once, nearly.)
        let (arc, swirl, lit, stagger) = if a.whole.is_some() { (0.0, 0.0, 0.0, 0.25) } else { (ARC, SWIRL, 1.0, 1.0) };
        a.t += dt;
        let (lo, hi) = a.pixels.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.1.y), hi.max(p.1.y)));
        let span = (hi - lo).max(1.0);
        // A body gets up whole: its standing picture, turned as it lay,
        // swung up about its feet (from where they lay to under it), in a
        // slow heave with a jerk or two in it.
        if let Some((feet, axis)) = a.whole {
            let p = (a.t / ASSEMBLE).clamp(0.0, 1.0);
            let ease = p * p * (3.0 - 2.0 * p) + JERK * (p * 19.0).sin() * p * (1.0 - p);
            let lay = -axis.x.atan2(axis.y);
            let (s, c) = (lay * (1.0 - ease)).sin_cos();
            let pivot = (feet - k.body.pos).lerp(Vec2::new(0.0, -k.body.half.y), ease);
            for &(_, to, rgba) in &a.pixels {
                let d = to - Vec2::new(0.0, -k.body.half.y);
                sparks.draw_now(k.body.pos + pivot + Vec2::new(d.x * c - d.y * s, d.x * s + d.y * c), 1.4, rgba);
            }
        }
        for (i, &(from, to, rgba)) in a.pixels.iter().enumerate().filter(|_| a.whole.is_none()) {
            // (Feet first: each starts later the higher it goes.)
            let delay = (to.y - lo) / span * (ASSEMBLE - FLIGHT) * stagger;
            let p = ((a.t - delay) / FLIGHT).clamp(0.0, 1.0);
            // (Slow to lift, quick in the middle, settling into place.)
            let ease = p * p * (3.0 - 2.0 * p);
            let hump = (ease * std::f32::consts::PI).sin();
            let side = if i % 2 == 0 { 1.0 } else { -1.0 };
            let swirl = Vec2::new(side * swirl * hump * (1.0 - ease), arc * hump);
            // (Still in the heap: dimmer; landing: lit by the glow.)
            let glow = (hump * 0.6).min(1.0) * lit;
            let l = a.light;
            let c = [rgba[0] + (l[0] - rgba[0]) * glow * 0.5, rgba[1] + (l[1] - rgba[1]) * glow * 0.5, rgba[2] + (l[2] - rgba[2]) * glow * 0.5, rgba[3]];
            sparks.draw_now(k.body.pos + from.lerp(to, ease) + swirl, 1.0, c);
        }
        if a.t < ASSEMBLE {
            continue;
        }
        let eyes = a.eyes;
        commands.entity(e).remove::<(Assembling, Dormant)>().insert((Visibility::Inherited, EyesFlash(1.0), crate::light::LightSource { color: eyes.map(|c| c * EYES_FLASH), flicker: 0.0 }));
        let mut burst = wisp(a.light);
        burst.colors = vec![((eyes[0] * 255.0) as u8, (eyes[1] * 255.0) as u8, (eyes[2] * 255.0) as u8)];
        burst.speed = 60.0;
        burst.spread = 3.2;
        sparks.emit(&burst, 12, k.body.pos + Vec2::new(0.0, k.body.half.y * 0.85), Vec2::Y, Vec2::ZERO);
        sounds.write(crate::sound::PlaySound::at("clank", k.body.pos).pitch(0.8));
        if let Some(&super::hurt::Bleeds(dust)) = bleeds {
            sim.world.splash([k.body.pos.x, k.body.pos.y - k.body.half.y * 0.6], dust, 10, 1.6);
        }
    }
    for (e, mut f, mut light) in &mut flashes {
        let was = f.0;
        f.0 -= dt * EYES_FADE;
        if f.0 <= 0.0 {
            commands.entity(e).remove::<(EyesFlash, crate::light::LightSource)>();
            continue;
        }
        light.color = light.color.map(|c| c * f.0 / was);
    }
}

/// The pixels of its standing picture (its first idle frame), each with a
/// place in the heap to fly from (one of its heap's bones, `from`, the
/// lowest to its feet; or anywhere low by it): offsets from its middle.
fn pixels_of(anim: &super::animation::Animator, k: &Kinematics, from: &[Vec2], whole: Option<(Vec2, Vec2)>, lost: &[super::parts::Limb]) -> Vec<(Vec2, Vec2, [f32; 4])> {
    let def = &anim.def;
    let Some(rig) = def.rig.as_ref() else { return vec![(Vec2::ZERO, Vec2::ZERO, [0.0; 4])] };
    let idx = rig.clips.get("idle").and_then(|c| c.frames.first().copied()).unwrap_or(0);
    let Some(frame) = rig.frames.get(idx) else { return vec![(Vec2::ZERO, Vec2::ZERO, [0.0; 4])] };
    let (fw, fh) = (def.sprite.frame.0 as f32, def.sprite.frame.1 as f32);
    let body = super::animation::body_at(def, k);
    let facing = k.loco.facing;
    let mut rng = platypus_sim::rng::Rng::seeded(&[k.body.pos.x.to_bits() as u64, 0xB0E5]);
    let mut unit = || rng.next_u32() as f32 / u32::MAX as f32;
    // (What it's lost isn't in it: the frontmost layer at a pixel a
    // limb it's lost, that pixel's not drawn.)
    let cuts = match (def.art_file.as_ref(), rig.names.get(idx)) {
        (Some(file), Some(name)) if !lost.is_empty() => super::ragdoll::cut(file, name),
        _ => Vec::new(),
    };
    let gone = |p: IVec2| cuts.iter().rev().find(|c| c.covers(p)).and_then(|c| c.limb()).is_some_and(|l| lost.contains(&l));
    let mut out = Vec::new();
    for py in 0..frame.h as i32 {
        for px in 0..frame.w as i32 {
            if !frame.opaque(px, py) || gone(IVec2::new(px, py)) {
                continue;
            }
            let c = frame.get(px, py);
            let to = body + Vec2::new((px as f32 - fw / 2.0 + 0.5) * facing, fh / 2.0 - 0.5 - py as f32);
            // (Lying in the heap: along the ground, low.)
            let from = Vec2::new((unit() - 0.5) * fh, -k.body.half.y + unit() * 4.0);
            out.push((from, to, [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 1.0]));
        }
    }
    // Legs drawn apart from its art (`legs::rig`): standing, straight down
    // from its hips to the ground under it, as they're drawn.
    if let Some(legs) = def.legs.as_ref().filter(|l| l.rig) {
        let rgb = |(r, g, b): (u8, u8, u8)| [r, g, b, 255];
        let line = legs.outline.unwrap_or((26, 20, 20));
        let w = |i: usize| legs.width.get(i).or(legs.width.last()).copied().unwrap_or(2.0) * 0.5;
        for (i, l) in legs.each.iter().enumerate() {
            if lost.contains(&super::parts::Limb::Leg(i as u8)) {
                continue;
            }
            let hip = Vec2::new(l.hip.0 * facing, l.hip.1);
            let foot = Vec2::new(hip.x, -k.body.half.y + 0.5);
            let color = if l.far { legs.far.unwrap_or((116, 112, 103)) } else { legs.color };
            let mut px = Vec::new();
            super::legs::stroke(hip, foot, w(0) + 1.0, w(1) + 1.0, rgb(line), &mut px);
            super::legs::stroke(hip, foot, w(0), w(1), rgb(color), &mut px);
            for (p, c) in px {
                out.push((Vec2::ZERO, p.as_vec2() + Vec2::splat(0.5), [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 1.0]));
            }
        }
    }
    if !from.is_empty() {
        // (Lowest bones to its feet, highest to its skull; a body's, from
        // its feet's end along it to its head.)
        let key = |p: &Vec2| whole.map_or(p.y, |(feet, axis)| (*p + k.body.pos - feet).dot(axis));
        let mut src: Vec<Vec2> = from.iter().map(|p| *p - k.body.pos).collect();
        src.sort_by(|a, b| key(a).total_cmp(&key(b)));
        let mut order: Vec<usize> = (0..out.len()).collect();
        order.sort_by(|&a, &b| out[a].1.y.total_cmp(&out[b].1.y));
        for (n, &i) in order.iter().enumerate() {
            out[i].0 = src[n * src.len() / order.len().max(1)];
        }
    }
    if out.is_empty() {
        out.push((Vec2::ZERO, Vec2::ZERO, [0.0; 4]));
    }
    out
}

/// A heap waking: its bones take on the light, pulsing faster as it nears.
pub fn tint_waking(piles: Query<(&Pile, &Children)>, mut sprites: Query<&mut Sprite>) {
    for (pile, children) in &piles {
        if pile.left >= GLOW {
            continue;
        }
        let k = (1.0 - pile.left.max(0.0) / GLOW).powf(1.5);
        let pulse = 0.5 + 0.5 * (pile.left * (6.0 + 14.0 * k)).sin();
        let t = k * (0.6 + 0.4 * pulse);
        for &c in children {
            if let Ok(mut s) = sprites.get_mut(c) {
                let l = pile.rise.light();
                s.color = Color::srgb(DIM + (l[0] - DIM) * t, DIM + (l[1] - DIM) * t, DIM + (l[2] - DIM) * t);
            }
        }
    }
}

/// Dormancy leaves one being put together alone (it wakes when it's whole).
pub type NotDown = Without<Assembling>;
