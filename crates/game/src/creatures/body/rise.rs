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
/// The bones fly together this long.
const ASSEMBLE: f32 = 0.6;
/// Standing where it can't (under a low roof), it tries again after this.
const RETRY: f32 = 0.5;
/// The light in the bones (cold, sickly), at its brightest.
const LIGHT: [f32; 3] = [0.35, 1.0, 0.75];
const LIGHT_MOST: f32 = 1.6;
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
    /// Kinds of hurt a heap is weaker still to.
    #[serde(default)]
    pub weak: Vec<Harm>,
}

#[derive(Component, Clone)]
pub struct Rises(pub RiseDef);

/// A heap of bones that will get up: what it was, how long it has left.
#[derive(Component)]
pub struct Pile {
    kind: String,
    def: std::sync::Arc<crate::creatures::def::CreatureDef>,
    rise: RiseDef,
    left: f32,
    facing: f32,
    sprite: Option<(Sprite, Vec3)>,
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
);

/// One that rises, killed, leaves its bones (a body that gets up:
/// `make_pile`) and goes, with no death of its own. Just before deaths.
pub fn collapse(mut commands: Commands, mut sim: ResMut<SimWorld>, mut died: MessageWriter<Died>, mut q: Query<Falling>, sprites: Query<(&Sprite, &Transform), With<super::animation::CreatureSprite>>) {
    for (e, mut h, k, rises, creature, anim, eq, children, bleeds, spin) in &mut q {
        if h.hp > 0.0 {
            continue;
        }
        // (Kept from deaths: it's gone now.)
        h.hp = 1.0;
        let sprite = children.and_then(|ch| ch.iter().find_map(|c| sprites.get(c).ok())).map(|(s, tf)| (s.clone(), tf.translation));
        let worn = eq.map(|eq| eq.worn.iter().flatten().chain(eq.held.iter()).copied().collect()).unwrap_or_default();
        died.write(Died { kind: creature.kind.clone(), def: anim.def.clone(), body: k.body, facing: k.loco.facing, sprite, worn, spin: spin.copied().unwrap_or_default(), rising: Some(rises.0.clone()), limbs: Vec::new() });
        if let Some(&super::hurt::Bleeds(dust)) = bleeds {
            sim.world.splash([k.body.pos.x, k.body.pos.y], dust, 14, 2.4);
        }
        commands.entity(e).despawn();
    }
}

/// The body of one that will get up (`corpses::lay_out`, for a `rising`
/// death): health of its own, its weaknesses worse, hit as a creature is.
pub fn make_pile(e: &mut EntityCommands, d: &Died, rise: &RiseDef, dust: Option<platypus_sim::MaterialId>) {
    let mut nature = d.def.nature;
    for &w in &rise.weak {
        nature = nature.weaker(w, PILE_WEAK);
    }
    let max = (d.def.health * PILE_SHARE).max(1.0);
    e.insert((
        Pile { kind: d.kind.clone(), def: d.def.clone(), rise: rise.clone(), left: rise.after, facing: d.facing, sprite: d.sprite.clone() },
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
        died.write(Died { kind: pile.kind.clone(), def: pile.def.clone(), body: k.body, facing: pile.facing, sprite: pile.sprite.clone(), worn, spin: default(), rising: None, limbs: Vec::new() });
        let burst = bone_burst();
        sparks.emit(&burst, burst.count as usize, k.body.pos, Vec2::Y, Vec2::ZERO);
        sounds.write(crate::sound::PlaySound::at("break", k.body.pos).pitch(1.3));
        if let Some(&super::hurt::Bleeds(dust)) = bleeds {
            sim.world.splash([k.body.pos.x, k.body.pos.y], dust, 18, 2.8);
        }
        commands.entity(e).despawn();
    }
}

/// Bone flying apart.
fn bone_burst() -> Emitter {
    Emitter { count: 26.0, life: (0.25, 0.6), colors: vec![(232, 226, 205), (190, 182, 160), (140, 134, 120)], speed: 110.0, spread: 1.4, gravity: 300.0, drag: 0.5, size: 1.0, jitter: 0.3, glow: false }
}

/// A wisp rising off waking bones.
fn wisp() -> Emitter {
    Emitter { count: 1.0, life: (0.4, 0.9), colors: vec![(120, 255, 200), (70, 200, 170)], speed: 22.0, spread: 0.5, gravity: -20.0, drag: 0.6, size: 1.0, jitter: 0.4, glow: true }
}

/// A grain of ash drawn in.
fn streak() -> Emitter {
    Emitter { count: 1.0, life: (0.1, 0.2), colors: vec![(150, 145, 140), (120, 255, 200)], speed: 70.0, spread: 0.1, gravity: 0.0, drag: 0.0, size: 1.0, jitter: 0.0, glow: true }
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
    mut q: Query<(Entity, &mut Pile, &Kinematics, &Corpse, Option<&mut crate::light::LightSource>)>,
) {
    let Some(items) = items else { return };
    for (e, mut pile, k, corpse, light) in &mut q {
        pile.left -= DT;
        let pos = k.body.pos;
        if pile.left < GLOW {
            let glow = (1.0 - pile.left.max(0.0) / GLOW).powf(1.5);
            let color = LIGHT.map(|c| c * LIGHT_MOST * glow);
            match light {
                Some(mut l) => l.color = color,
                None => {
                    commands.entity(e).insert(crate::light::LightSource { color, flicker: 0.0 });
                }
            }
            if sim.world.tick().is_multiple_of(3) {
                let at = pos + Vec2::new((sim.world.tick() % 7) as f32 - 3.0, 0.0);
                sparks.emit(&wisp(), 1, at, Vec2::Y, Vec2::ZERO);
            }
            draw_in_ash(&mut sim, &mut sparks, pos);
        }
        if pile.left > 0.0 {
            continue;
        }
        // Standing: its box, from the heap's ground, must be clear.
        let (w, h) = pile.def.size;
        let feet = Vec2::new(pos.x, pos.y - k.body.half.y);
        let standing = platypus_physics::Body::new(feet + Vec2::new(0.0, h / 2.0 + 0.5), Vec2::new(w, h));
        let (lo, hi) = standing.cells_at(standing.pos);
        if overlaps_solid(&WorldGrid(&sim.world), lo, hi) {
            pile.left = RETRY;
            continue;
        }
        // Dressed in what's left in it; the rest spills.
        let left: Vec<Stack> = chests.contents(corpse.key, &sim.world, &items).slots.iter().flatten().copied().collect();
        chests.forget(corpse.key);
        let mut eq = crate::gear::Equipment::default();
        for s in left {
            let def = items.def(s.item);
            let weapon = matches!(def.use_, Use::Melee(_) | Use::Bow(_)) && def.gear.is_some();
            if weapon && eq.held.is_none() {
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
            .or_else(|| pile.def.unarmed.clone());
        let (share, facing) = (pile.rise.health, pile.facing);
        let eyes = pile.def.eyes.map_or(LIGHT, |(r, g, b)| [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]);
        crate::creatures::def::spawn_creature(&mut commands, &pile.kind, feet, move |c| {
            if let Some(mut h) = c.get_mut::<Health>() {
                h.hp = h.max * share;
            }
            if let Some(mut k) = c.get_mut::<Kinematics>() {
                k.loco.facing = facing;
            }
            c.insert((eq, crate::combat::Wielding(wielding), Outfitted, Assembling { t: 0.0, pixels: Vec::new(), eyes }, Dormant, Visibility::Hidden));
        });
        commands.entity(e).despawn();
    }
}

/// Ash near a waking heap drifts back into it, a grain or two a tick.
fn draw_in_ash(sim: &mut SimWorld, sparks: &mut crate::vfx::Sparks, at: Vec2) {
    let Some(ash) = sim.world.materials().id("ash") else { return };
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
                sparks.emit(&streak(), 1, from, at - from, Vec2::ZERO);
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
            a.pixels = pixels_of(anim, k);
            sounds.write(crate::sound::PlaySound::at("clank", k.body.pos).pitch(0.55).volume(0.7));
        }
        a.t += dt;
        let (lo, hi) = a.pixels.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.1.y), hi.max(p.1.y)));
        let span = (hi - lo).max(1.0);
        for &(from, to, rgba) in &a.pixels {
            // (Feet first: each starts later the higher it goes.)
            let delay = (to.y - lo) / span * ASSEMBLE * 0.5;
            let p = ((a.t - delay) / (ASSEMBLE * 0.5)).clamp(0.0, 1.0);
            let ease = 1.0 - (1.0 - p).powi(3);
            let lift = (ease * std::f32::consts::PI).sin() * 3.0;
            sparks.draw_now(k.body.pos + from.lerp(to, ease) + Vec2::new(0.0, lift), 1.0, rgba);
        }
        if a.t < ASSEMBLE {
            continue;
        }
        let eyes = a.eyes;
        commands.entity(e).remove::<(Assembling, Dormant)>().insert((Visibility::Inherited, EyesFlash(1.0), crate::light::LightSource { color: eyes.map(|c| c * EYES_FLASH), flicker: 0.0 }));
        let mut burst = wisp();
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
/// place in the heap to fly from: offsets from its middle.
fn pixels_of(anim: &super::animation::Animator, k: &Kinematics) -> Vec<(Vec2, Vec2, [f32; 4])> {
    let def = &anim.def;
    let Some(rig) = def.rig.as_ref() else { return vec![(Vec2::ZERO, Vec2::ZERO, [0.0; 4])] };
    let idx = rig.clips.get("idle").and_then(|c| c.frames.first().copied()).unwrap_or(0);
    let Some(frame) = rig.frames.get(idx) else { return vec![(Vec2::ZERO, Vec2::ZERO, [0.0; 4])] };
    let (fw, fh) = (def.sprite.frame.0 as f32, def.sprite.frame.1 as f32);
    let body = super::animation::body_at(def, k);
    let facing = k.loco.facing;
    let mut rng = platypus_sim::rng::Rng::seeded(&[k.body.pos.x.to_bits() as u64, 0xB0E5]);
    let mut unit = || rng.next_u32() as f32 / u32::MAX as f32;
    let mut out = Vec::new();
    for py in 0..frame.h as i32 {
        for px in 0..frame.w as i32 {
            if !frame.opaque(px, py) {
                continue;
            }
            let c = frame.get(px, py);
            let to = body + Vec2::new((px as f32 - fw / 2.0 + 0.5) * facing, fh / 2.0 - 0.5 - py as f32);
            // (Lying in the heap: along the ground, low.)
            let from = Vec2::new((unit() - 0.5) * fh, -k.body.half.y + unit() * 4.0);
            out.push((from, to, [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 1.0]));
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
                s.color = Color::srgb(DIM + (LIGHT[0] - DIM) * t, DIM + (LIGHT[1] - DIM) * t, DIM + (LIGHT[2] - DIM) * t);
            }
        }
    }
}

/// Dormancy leaves one being put together alone (it wakes when it's whole).
pub type NotDown = Without<Assembling>;
