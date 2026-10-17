//! Creatures (DESIGN §14): the player, enemies, villagers, critters,
//! anything with a body and a brain. Every creature is data, in layers;
//! a few also have code of their own.
//!
//! Where to go to change each:
//! - what a creature *is*: a RON file in `assets/data/creatures/` (`def.rs`)
//! - how it takes *hurt*: its kind's profile (`nature.rs`, `assets/data/kinds.ron`)
//! - its *body*: how it looks and animates, its legs, its wounds and
//!   flashes, what's on it (burning, wet, chilled) (`body/`)
//! - how it *decides*: a brain, registered by name (`brain/`; the player's
//!   is the keyboard, `player.rs`)
//! - its *moves*: the attacks it makes (`moves/`)
//! - code of its *own*, for what data can't say (`custom/`)
//! - how it *moves*: shared by all, `platypus_physics::Locomotion` driven by
//!   `move_creatures` here
//! - where it *appears*: `spawn.rs`

pub mod body;
pub mod brain;
pub mod custom;
pub mod def;
pub mod factions;
pub mod moves;
pub mod nature;
pub mod player;
pub mod spawn;

use bevy::prelude::*;
use platypus_physics::{Body, Grid, Intent, Locomotion, MovementStats, Occupancy, move_and_collide};
use platypus_sim::{CellPos, Kind, World, WorldEdit};
use serde::Deserialize;

use crate::world::{SimWorld, TICK_HZ, TickSet};

pub struct CreaturesPlugin;

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Landed>()
            .add_message::<body::hurt::Reacted>()
            .add_message::<Took>()
            .add_message::<AirJumped>()
            .add_message::<Died>()
            .add_message::<Rocketed>()
            .add_plugins((def::CreaturePlugin, brain::BrainPlugin, spawn::SpawnPlugin, body::animation::AnimationPlugin, factions::FactionsPlugin))
            .add_plugins((player::PlayerPlugin, brain::ai::AiPlugin, brain::critters::CrittersPlugin, brain::hunter::HunterPlugin, body::legs::LegsPlugin, custom::CustomPlugin, brain::villager::VillagerPlugin, moves::MovesPlugin, brain::way::WayPlugin, body::sight::SightPlugin))
            .add_systems(FixedUpdate, (move_creatures, fall_damage, body::elements::expose, crate::combat::guard, body::hurt::notice, nature::regenerate, tally, crate::observe::observe, body::rise::crush, body::rise::collapse, body::rise::shatter, deaths, body::rise::stand).chain().in_set(TickSet::Bodies))
            .insert_resource(body::elements::Coatings::load())
            .init_resource::<PlayerDeaths>()
            .init_resource::<StableIds>()
            .add_systems(FixedUpdate, displace_liquid.after(move_creatures).in_set(TickSet::Bodies))
            // What a grab holds stays in its grip.
            .add_systems(FixedUpdate, moves::pin.after(move_creatures).before(deaths).in_set(TickSet::Bodies))
            .add_systems(FixedUpdate, moves::struggle.before(moves::pin).after(move_creatures).in_set(TickSet::Bodies))
            .add_systems(Update, (body::elements::tint, body::elements::blaze, body::elements::reload_coatings, body::hurt::watch, body::hurt::float))
            // Hits as they landed, shown (`hurt`); the badly hurt drip and falter.
            .add_systems(FixedUpdate, (body::hurt::react, body::hurt::wounded).chain().after(crate::combat::Hits))
            // Blows on limbs wear them down and cut them off (`wounds`).
            .add_message::<body::wounds::Cut>()
            .init_resource::<body::wounds::LayerMasks>()
            .add_systems(FixedUpdate, (body::wounds::wound, body::wounds::sever, body::wounds::bleed).chain().after(crate::combat::Hits).before(TickSet::Cells))
            .add_systems(Update, (body::wounds::fade, body::wounds::sit.before(body::animation::animate), body::marks::settle))
            .add_systems(FixedUpdate, body::marks::mark.after(crate::combat::Hits).before(TickSet::Cells))
            .add_systems(FixedUpdate, body::rise::seek.after(brain::BrainSet).before(move_creatures))
            .add_systems(FixedUpdate, (body::elements::struck, body::elements::zapped, blasted, pelted).after(TickSet::Cells))
            .add_systems(PostUpdate, interpolate.before(TransformSystems::Propagate))
            .add_systems(Update, (body::rise::assemble, body::rise::tint_waking))
            .add_systems(PreUpdate, dormancy)
            .add_systems(Update, body::tumble::spin.before(body::animation::animate))
            .add_systems(FixedUpdate, voices.in_set(TickSet::Bodies))
            .add_systems(FixedUpdate, spurts.after(brain::BrainSet).before(move_creatures))
            // A slime holds its goo round it.
            .add_systems(FixedUpdate, body::goo::hold.after(move_creatures).before(deaths).in_set(TickSet::Bodies))
            .add_systems(FixedUpdate, (body::goo::spike, body::goo::glut_strike).chain().after(brain::BrainSet).before(move_creatures))
            .add_systems(FixedUpdate, body::goo::merge.after(body::goo::hold).before(deaths).in_set(TickSet::Bodies))
            .add_systems(FixedUpdate, (body::goo::engulf, body::goo::digest).chain().after(body::goo::hold).before(deaths).in_set(TickSet::Bodies))
            .add_systems(FixedUpdate, (body::goo::glut.after(tally).before(deaths), body::goo::bounce.after(move_creatures)).in_set(TickSet::Bodies))
            // The dead come up out of the ground, and burn in the sun.
            .add_systems(FixedUpdate, (body::emerge::emerge, body::emerge::sunburn.before(body::elements::expose)).in_set(TickSet::Bodies))
            .add_systems(Update, body::emerge::draw)
            // A body that gets up, set alight, burns away for good.
            .add_systems(FixedUpdate, (body::rise::kindle, body::rise::pyre).chain().after(body::elements::expose).before(body::rise::shatter).in_set(TickSet::Bodies));
    }
}

/// Far from every player, or its ground not loaded: it rests. Its brain,
/// senses, legs, exposure and touch skip it (they ran for every creature in
/// the world, hundreds, most of them nowhere near: half the frame), as its
/// movement already did off the loaded ground. Looked at four times a
/// second; it wakes as a player comes within `DORMANT_FAR`.
#[derive(Component)]
pub struct Dormant;

const DORMANT_FAR: f32 = 1_400.0;

/// Passing for something it isn't (a mimic, a chest): no mind of its own
/// (`hunter`), no touch; its own code says when it's itself.
#[derive(Component)]
pub struct Disguised;

/// Goes through rock and everything (a wraith): it moves as its velocity
/// says, no gravity, nothing in the way (its own code steers it).
#[derive(Component)]
pub struct Phasing;

/// Drawn toward `to` at `speed` (cells/s) on top of how it moves (a dread
/// knight's void): it can run against it, slower; for `left` s.
#[derive(Component)]
pub struct Tugged {
    pub to: Vec2,
    pub speed: f32,
    pub left: f32,
}

/// Creatures, not the player.
type NotPlayer = (With<Creature>, Without<player::LocalPlayer>);

fn dormancy(
    mut commands: Commands,
    time: Res<Time>,
    mut clock: Local<f32>,
    sim: Res<SimWorld>,
    players: Query<&Kinematics, With<player::LocalPlayer>>,
    q: Query<(Entity, &Kinematics, Has<Dormant>), (NotPlayer, body::rise::NotDown)>,
) {
    *clock += time.delta_secs();
    if *clock < 0.25 {
        return;
    }
    *clock = 0.0;
    let near: Vec<Vec2> = players.iter().map(|k| k.body.pos).collect();
    for (e, k, dormant) in &q {
        let p = k.body.pos;
        let far = !sim.world.is_loaded(CellPos::from_world(p.x, p.y).chunk()) || near.iter().all(|n| n.distance(p) > DORMANT_FAR);
        if far && !dormant {
            commands.entity(e).insert(Dormant);
        } else if !far && dormant {
            commands.entity(e).remove::<Dormant>();
        }
    }
}

/// A creature's number, in the order creatures were made since the world
/// began (or was reset whole): what its rolls are seeded from, so the same
/// fight played again rolls the same (entity ids depend on everything
/// else ever made, menus too).
#[derive(Component, Clone, Copy, Debug)]
pub struct Stable(pub u64);

/// The next creature's number.
#[derive(Resource, Default)]
pub struct StableIds(pub u64);

/// Who a creature is through its risings: its lineage, else its number.
pub fn lineage_of(l: Option<(Option<&body::rise::Lineage>, Option<&Stable>)>) -> u64 {
    l.and_then(|(l, s)| l.map(|l| l.0).or(s.map(|s| s.0))).unwrap_or(0)
}

/// A creature's number for its rolls (an entity's bits, if it has none).
pub fn stable(ids: &Query<&Stable>, e: Entity) -> u64 {
    ids.get(e).map_or(e.to_bits(), |s| s.0)
}

/// Instance of a creature definition (file stem in `assets/data/creatures/`).
#[derive(Component, Clone, Debug)]
pub struct Creature {
    pub kind: String,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Team {
    Player,
    Enemy,
    Neutral,
    /// The village's people: on the player's side (its blows pass them by),
    /// and hunted as the player is.
    Villager,
}

impl Team {
    /// On the same side: blows between them don't land (anyone can hit the
    /// neutral).
    pub fn allied(self, other: Team) -> bool {
        match (self, other) {
            (Team::Neutral, _) | (_, Team::Neutral) => false,
            (Team::Player, Team::Villager) | (Team::Villager, Team::Player) => true,
            (a, b) => a == b,
        }
    }

    /// What monsters hunt: the player and the villagers.
    pub fn hunted(self) -> bool {
        matches!(self, Team::Player | Team::Villager)
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Health {
    pub hp: f32,
    pub max: f32,
    /// What its gear stops (`gear::stats`): every hurt goes through `harm`.
    pub ward: Ward,
    /// How its kind takes each kind of hurt (`creatures::nature`).
    pub nature: Nature,
    /// The kinds of hurt it's felt since this was last looked at (a bit
    /// each, `Harm::bit`): what stops a troll's healing.
    pub felt: u16,
    /// What it's taken since the last tick's `tally`, by kind (`Harm as
    /// usize`): what was meant, and what got through (below 0: drunk in).
    pub took: [(f32, f32); 11],
}

pub use nature::{Harm, Nature};

/// Armour and resistances (shares 0..1 of each kind stopped), and the
/// share of what gets through that's taken at all (the player's difficulty:
/// `difficulty.rs`).
#[derive(Clone, Copy, Debug)]
pub struct Ward {
    pub armor: f32,
    pub fire: f32,
    pub frost: f32,
    pub storm: f32,
    pub acid: f32,
    pub fall: f32,
    pub taken: f32,
}

impl Default for Ward {
    fn default() -> Self {
        Ward { armor: 0.0, fire: 0.0, frost: 0.0, storm: 0.0, acid: 0.0, fall: 0.0, taken: 1.0 }
    }
}

/// Armour stops `armor / (armor + ARMOR_HALF)` of a physical hurt: half at
/// this much, never all.
pub const ARMOR_HALF: f32 = 50.0;

impl Ward {
    /// The share of a kind of hurt that gets through.
    pub fn through(&self, kind: Harm) -> f32 {
        let stopped = match kind {
            Harm::Slash | Harm::Pierce | Harm::Blunt => self.armor.max(0.0) / (self.armor.max(0.0) + ARMOR_HALF),
            Harm::Fire => self.fire,
            Harm::Frost => self.frost,
            Harm::Storm => self.storm,
            Harm::Acid => self.acid,
            Harm::Fall => self.fall,
            Harm::Poison | Harm::Radiant | Harm::Void => 0.0,
        };
        (1.0 - stopped.clamp(0.0, 0.9)) * self.taken
    }
}

impl Health {
    pub fn new(max: f32) -> Health {
        Health { hp: max, max, ward: Ward::default(), nature: Nature::default(), felt: 0, took: [(0.0, 0.0); 11] }
    }

    /// Hurt it: `amount` of a kind, less what its gear stops, times how its
    /// kind takes it. Returns what got through; below 0, it drank it in
    /// (its nature heals it: up to its maximum).
    pub fn harm(&mut self, amount: f32, kind: Harm) -> f32 {
        if amount <= 0.0 {
            return 0.0;
        }
        self.felt |= kind.bit();
        let m = self.nature.of(kind);
        let got = if m < 0.0 {
            let heal = amount * -m;
            self.hp = (self.hp + heal).min(self.max);
            -heal
        } else {
            let got = amount * self.ward.through(kind) * m;
            self.hp -= got;
            got
        };
        let t = &mut self.took[kind as usize];
        t.0 += amount;
        t.1 += got;
        got
    }
}

/// Stepping up a bump snaps the body up (a tick), and walking off one
/// down; what's drawn (and the camera) follows smoothly instead: this many
/// cells still to go (below: to rise; above: to come down).
#[derive(Component, Default)]
pub struct StepEase(pub f32);

/// How fast what's drawn rises after a step up (cells/s).
const STEP_EASE: f32 = 105.0;

/// Physical state of a creature. `prev_pos` is for render interpolation.
#[derive(Component, Clone, Debug)]
pub struct Kinematics {
    pub body: Body,
    pub loco: Locomotion,
    pub prev_pos: Vec2,
}

#[derive(Component, Clone, Debug)]
pub struct MoveStats(pub MovementStats);

/// What the brain wants this tick. Brains write it in `TickSet::Intent`.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Controls(pub Intent);

/// A spurt under way (its speed's times this, for so long), and when it
/// may next (`CreatureDef::spurt`).
#[derive(Component, Default)]
pub struct Spurt {
    left: f32,
    times: f32,
    next: f32,
}

/// Hunting, one that spurts breaks into a run at its quarry now and then:
/// a snarl, suddenly fast for a couple of seconds, then as it was.
#[allow(clippy::type_complexity)]
fn spurts(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &body::animation::Animator, &Kinematics, &mut MoveStats, Option<&mut Spurt>, Option<&brain::senses::Alert>, Option<&Stable>), Without<Dormant>>,
    bodies: Query<&Kinematics>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    let now = time.elapsed_secs();
    let dt = time.delta_secs();
    for (e, anim, k, mut moves, spurt, alert, stable) in &mut q {
        let Some(def) = anim.def.spurt else { continue };
        let n = stable.map_or(e.to_bits(), |s| s.0);
        let roll = |salt: u64| ((n.wrapping_add(salt).wrapping_mul(0x9E37_79B9_7F4A_7C15)) >> 40) as f32 / (1u64 << 24) as f32;
        let wait = |at: f32| at + def.every.0 + (def.every.1 - def.every.0) * roll((at * 10.0) as u64);
        let Some(mut s) = spurt else {
            // (Its first any time: no safe first seconds.)
            commands.entity(e).insert(Spurt { left: 0.0, times: 1.0, next: now + def.every.1 * roll(7) });
            continue;
        };
        if s.left > 0.0 {
            s.left -= dt;
            if s.left <= 0.0 {
                moves.0.run_speed /= s.times;
                s.times = 1.0;
                s.next = wait(now);
            }
            continue;
        }
        // (At what it's hunting, near enough, not on top of it.)
        let quarry = alert.filter(|a| a.engaged()).and_then(|a| a.target).and_then(|t| bodies.get(t).ok());
        let Some(qk) = quarry else { continue };
        let d = qk.body.pos.distance(k.body.pos);
        if now < s.next || !(def.range.0..def.range.1).contains(&d) || !k.loco.grounded() {
            continue;
        }
        s.left = def.secs;
        s.times = def.speed;
        moves.0.run_speed *= def.speed;
        sounds.write(crate::sound::PlaySound::at("snarl", k.body.pos));
    }
}

/// When it next makes its sound (`CreatureDef::voice`).
#[derive(Component)]
pub struct Voice(f32);

/// Awake, each with a voice makes its sound now and then (a zombie's
/// groan), at its own times.
fn voices(mut commands: Commands, time: Res<Time>, q: Query<(Entity, &body::animation::Animator, &Kinematics, Option<&Voice>, Option<&Stable>), Without<Dormant>>, mut sounds: MessageWriter<crate::sound::PlaySound>) {
    let now = time.elapsed_secs();
    for (e, anim, k, voice, stable) in &q {
        let Some((sound, (lo, hi))) = anim.def.voice.as_ref() else { continue };
        // (Its own times: its number, and how often it's spoken.)
        let n = stable.map_or(e.to_bits(), |s| s.0);
        let u = |salt: u64| ((n.wrapping_add(salt).wrapping_mul(0x9E37_79B9_7F4A_7C15)) >> 40) as f32 / (1u64 << 24) as f32;
        match voice {
            None => {
                commands.entity(e).insert(Voice(now + lo + (hi - lo) * u(1)));
            }
            Some(v) if now >= v.0 => {
                sounds.write(crate::sound::PlaySound::at(sound.clone(), k.body.pos));
                commands.entity(e).insert(Voice(now + lo + (hi - lo) * u((now * 10.0) as u64)));
            }
            _ => {}
        }
    }
}

/// Its own pace (`CreatureDef::pace`): its run speed times this.
#[derive(Component, Clone, Copy, Debug)]
pub struct Pace(pub f32);

impl Pace {
    /// One of `spread` either way, picked by its number (the same each run).
    pub fn of(spread: f32, number: u64) -> Pace {
        let u = (number.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40) as f32 / (1u64 << 24) as f32;
        Pace(1.0 + spread * (2.0 * u - 1.0))
    }

    pub fn moved(self, m: &platypus_physics::MovementStats) -> platypus_physics::MovementStats {
        let mut m = m.clone();
        m.run_speed *= self.0;
        m
    }
}

/// Fall damage, by how far it fell (Terraria-style; speed saturates at
/// max fall within ~50 cells, so it can't tell a double jump from a cliff):
/// falling further than `safe_height` cells, from where the fall started
/// (the highest point since it left the ground or last came down gently:
/// `GENTLE_FALL`), hurts `per_cell` a cell over; slamming into a wall
/// or ceiling faster than `slam_speed` (flung by a spell, a blast) hurts
/// `per_speed` per cell/s over.
#[derive(Component, Clone, Copy, Debug, Deserialize)]
pub struct FallDamage {
    pub safe_height: f32,
    pub per_cell: f32,
    #[serde(default = "slam_speed")]
    pub slam_speed: f32,
    #[serde(default = "slam_per")]
    pub per_speed: f32,
}

fn slam_speed() -> f32 {
    675.0
}

fn slam_per() -> f32 {
    0.167
}

/// Coming down slower than this (cells/s), a fall hasn't started yet: it
/// counts from where it last was this slow (the top of a jump, rocket boots
/// braking near the ground, a wall slide), not from the highest point.
const GENTLE_FALL: f32 = 120.0;

/// Where a body's fall started: the highest point since it last stood on
/// something or came down gently (for fall damage).
#[derive(Component, Default)]
pub struct FallTrack {
    top: Option<f32>,
}

/// Hitting a wall or ceiling slower than this isn't worth reporting (walking
/// into a wall).
const SLAM_MIN: f32 = 225.0;

/// A creature's rocket boots fired this tick (`gear::boots`: the exhaust).
#[derive(Message, Clone, Copy, Debug)]
pub struct Rocketed {
    pub entity: Entity,
    /// Its feet, and how it's moving.
    pub at: Vec2,
    pub vel: Vec2,
}

/// A body jumped off thin air (a double jump) with its feet at `at`: a puff
/// of cloud there (`vfx`), a soft flash of light (`light`).
#[derive(Message, Clone, Copy, Debug)]
pub struct AirJumped {
    pub at: Vec2,
}

/// A body landed after falling `drop` cells, or slammed into a wall or
/// ceiling at `slam` cells/s: fall damage reads it.
#[derive(Message, Clone, Copy, Debug)]
pub struct Landed {
    pub entity: Entity,
    pub drop: f32,
    pub slam: f32,
}

/// Walks through webs freely (a spider: its creature file's `web_walker`).
#[derive(Component)]
pub struct WebWalker;

/// In something sticky, a body goes at this share of its speed.
const STUCK_SPEED: f32 = 0.3;

/// Whether any cell a body is in is sticky (cobweb).
fn stuck_in(world: &World, body: &Body) -> bool {
    let (lo, hi) = (body.pos - body.half, body.pos + body.half);
    (lo.y.floor() as i32..hi.y.ceil() as i32).any(|y| {
        (lo.x.floor() as i32..hi.x.ceil() as i32).any(|x| world.get(CellPos::new(x, y)).is_some_and(|c| !c.is_air() && world.materials().phys(c.material).sticky))
    })
}

/// The cell world as bodies (and the planner) see it: `platypus_nav`.
pub use platypus_nav::world::{WorldGrid, backed};

/// Where a box of `half` round `center` sits clear of solid ground: there,
/// or the nearest place within `reach` cells (up first, then aside, then
/// down), if any.
pub fn free_spot(world: &platypus_sim::World, center: Vec2, half: Vec2, reach: i32) -> Option<Vec2> {
    let grid = WorldGrid(world);
    let clear = |c: Vec2| {
        let (lo, hi) = (c - half + 0.25, c + half - 0.25);
        let mut y = lo.y;
        loop {
            let mut x = lo.x;
            loop {
                if platypus_physics::Grid::occupancy(&grid, x.floor() as i32, y.floor() as i32) == platypus_physics::Occupancy::Solid {
                    return false;
                }
                if x >= hi.x {
                    break;
                }
                x = (x + 2.0).min(hi.x);
            }
            if y >= hi.y {
                break;
            }
            y = (y + 2.0).min(hi.y);
        }
        true
    };
    if clear(center) {
        return Some(center);
    }
    (1..=reach / 2).map(|r| (r * 2) as f32).find_map(|r| [Vec2::new(0.0, r), Vec2::new(-r, 0.0), Vec2::new(r, 0.0), Vec2::new(-r, r), Vec2::new(r, r), Vec2::new(0.0, -r)].into_iter().map(|d| center + d).find(|&c| clear(c)))
}

const DT: f32 = (1.0 / TICK_HZ) as f32;

/// One movement code path for every creature.
type Movers<'a> = (Entity, &'a mut Kinematics, &'a MoveStats, &'a Controls, (Option<&'a body::elements::Chilled>, Option<&'a body::elements::Coated>, Has<body::goo::Goo>, Has<Phasing>, Option<&'a mut Tugged>), Option<&'a mut FallTrack>, Has<WebWalker>, Option<&'a crate::gear::hook::Rope>, Has<player::LocalPlayer>, Option<&'a mut StepEase>, Has<body::hurt::Faltering>);

pub(crate) fn move_creatures(
    sim: Res<SimWorld>,
    mut q: Query<Movers>,
    mut landed: MessageWriter<Landed>,
    mut air: MessageWriter<AirJumped>,
    mut rockets: MessageWriter<Rocketed>,
    mut dashed: MessageWriter<crate::combat::Dashed>,
    tempo: Res<crate::tempo::Tempo>,
    coatings: Res<body::elements::Coatings>,
) {
    let grid = WorldGrid(&sim.world);
    for (entity, mut k, stats, controls, (chilled, coated, gooey, phasing, tug), track, web_walker, rope, player, ease, faltering) in &mut q {
        // Frozen until the ground under it is loaded.
        if !sim.world.is_loaded(CellPos::from_world(k.body.pos.x, k.body.pos.y).chunk()) {
            continue;
        }
        let k = &mut *k;
        k.prev_pos = k.body.pos;
        // (Through everything, as it's steered.)
        if phasing {
            k.body.pos += k.body.vel * DT;
            continue;
        }
        let slowed;
        // At the game's tempo (`tempo.rs`).
        let paced = tempo.apply(&stats.0, player);
        // Chilled, wading through something sticky (cobweb), coated in
        // something that clogs (a slime's goo), or hurt badly
        // (`hurt::Faltering`): slowed.
        let webbed = !web_walker && stuck_in(&sim.world, &k.body);
        // (A body of goo isn't clogged by goo.)
        let clogged = if gooey { 1.0 } else { coated.map_or(1.0, |c| c.speed(&coatings.by_name)) };
        let speed = chilled.map_or(1.0, |c| c.speed()).min(clogged).min(if webbed { STUCK_SPEED } else { 1.0 }).min(if faltering { body::hurt::FALTER_SPEED } else { 1.0 });
        let stats = if speed < 1.0 {
            slowed = paced.slowed(speed);
            &slowed
        } else {
            &paced
        };
        // On a grappling hook's rope: it swings, held at the rope's length
        // (reeling in pulls it up: `gear::hook`).
        let tether = rope.and_then(|r| r.tether());
        k.loco.swinging = tether.is_some();
        // (A climber can hold on to the wall behind it.)
        k.loco.backed = stats.cling && backed(&sim.world, k.body.pos);
        let ev = k.loco.steer(stats, &controls.0, &mut k.body, DT);
        if let Some((at, len)) = tether {
            platypus_physics::tether(&mut k.body, at, len, DT);
        }
        if ev.dashed {
            dashed.write(crate::combat::Dashed(entity));
        }
        if ev.rocketed {
            rockets.write(Rocketed { entity, at: k.body.pos - Vec2::new(0.0, k.body.half.y), vel: k.body.vel });
        }
        if ev.air_jumped {
            air.write(AirJumped { at: k.body.pos - Vec2::new(0.0, k.body.half.y) });
        }
        // A jump off air or a wall starts the fall over (a double jump just
        // before landing saves you, as in Terraria).
        // (So does hanging from a hook.)
        let rejumped = ev.air_jumped || ev.wall_jumped || tether.is_some();
        let before = k.body.vel;
        // (Tugged: drawn that way too, this tick only; how it steers isn't.)
        let drift = tug.map_or(Vec2::ZERO, |mut t| {
            t.left -= DT;
            if t.left > 0.0 { (t.to - k.body.pos).normalize_or_zero() * t.speed } else { Vec2::ZERO }
        });
        k.body.vel += drift;
        let contacts = move_and_collide(&grid, &mut k.body, DT);
        // (What a wall stopped is stopped; the rest goes back as it steers.)
        let v = k.body.vel;
        k.body.vel = Vec2::new(if v.x == 0.0 { 0.0 } else { v.x - drift.x }, if v.y == 0.0 { 0.0 } else { v.y - drift.y });
        if let Some(mut e) = ease {
            let most = 2.0 * stats.step_height as f32;
            e.0 = (e.0 - contacts.stepped as f32 + contacts.snapped).clamp(-most, most);
        }
        // (A plunge's dive lasts the tick it was given for.)
        k.loco.dive = 0.0;
        // Slammed into a wall or a ceiling (flung by a spell, a blast): an
        // impact like a landing, by the speed it hit at. (Not on a hook's
        // rope: it reels you in fast, right up to the rock it bit.)
        let walled = if (contacts.wall_left && before.x < 0.0) || (contacts.wall_right && before.x > 0.0) { before.x.abs() } else { 0.0 };
        let roofed = if contacts.ceiling && before.y > 0.0 { before.y } else { 0.0 };
        let slam = if tether.is_some() { 0.0 } else { walled.max(roofed) };
        let y = k.body.pos.y;
        let drop = track.map_or(0.0, |mut t| {
            if rejumped {
                t.top = None;
            }
            let top = t.top.unwrap_or(y).max(y);
            // (Standing on something, or in water: the fall starts over.
            // Nearly still, or coming down gently (rocket boots braking, a
            // wall slide): it starts from here.)
            t.top = if contacts.ground || contacts.submerged > 0.5 {
                None
            } else if k.body.vel.y > -GENTLE_FALL {
                Some(y)
            } else {
                Some(top)
            };
            top - y
        });
        if k.loco.after_move(contacts).is_some() {
            landed.write(Landed { entity, drop, slam });
        } else if slam > SLAM_MIN {
            landed.write(Landed { entity, drop: 0.0, slam });
        }
    }
}

/// Bodies push liquid aside: whatever flowed into a creature's box moves up
/// its column (the level rises around it), splashing if it arrived fast.
fn displace_liquid(mut sim: ResMut<SimWorld>, q: Query<&Kinematics, Without<Dormant>>) {
    for k in &q {
        let (lo, hi) = k.body.cells_at(k.body.pos);
        let grid = WorldGrid(&sim.world);
        let wet = (lo.y..=hi.y).any(|y| (lo.x..=hi.x).any(|x| grid.occupancy(x, y) == Occupancy::Liquid));
        if !wet {
            continue;
        }
        // Cells per tick, in sixteenths (edits hold integers).
        let v = k.body.vel / TICK_HZ as f32 * 16.0;
        sim.queue(WorldEdit::Displace {
            min: CellPos::new(lo.x, lo.y),
            max: CellPos::new(hi.x, hi.y),
            vel: [v.x.round() as i16, v.y.round() as i16],
        });
    }
}

fn fall_damage(mut landed: MessageReader<Landed>, mut q: Query<(&FallDamage, &mut Health), Without<crate::magic::well::Carried>>) {
    for l in landed.read() {
        if let Ok((f, mut h)) = q.get_mut(l.entity) {
            let fell = (l.drop - f.safe_height).max(0.0) * f.per_cell;
            let slammed = (l.slam - f.slam_speed).max(0.0) * f.per_speed;
            h.harm(fell.max(slammed), Harm::Fall);
        }
    }
}

/// How many times the player has died (the HUD shows it).
#[derive(Resource, Default)]
pub struct PlayerDeaths(pub u32);

/// Dead creatures burst into blood particles that land as real cells (they
/// run and pool). The player wakes at its checkpoint (`respawn.rs`).
/// Particles of solid, powder or liquid (not rain, dust or embers) faster
/// than this (cells/s) hurt what they fly through...
const PELT_SAFE: f32 = 135.0;
/// (Liquids: faster than this: a death's burst of blood doesn't hurt what's
/// beside it; water flung from a well still does.)
const PELT_SAFE_LIQUID: f32 = 375.0;
/// ... by their weight (density against water's; liquids half) × how many
/// times faster × this, and are mostly stopped by it, shoving it.
const PELT: f32 = 0.8;

type Pelted<'a> = (&'a mut Kinematics, &'a mut Health, Option<&'a brain::way::DigFace>, Option<&'a body::hurt::Bleeds>);

/// Heavy things flying fast hurt what they hit: a ball of rock dropped
/// from a well, blast debris, a flung stream of sand (not a digger at its
/// face: what it digs rains on it).
/// (Not a heap of bones or a loose skull: its own dust, flung as it fell,
/// would wear it down; nor its own blood, spurting from a wound.)
fn pelted(mut sim: ResMut<SimWorld>, mut q: Query<Pelted, (Without<body::rise::Pile>, Without<body::rise::Skull>)>) {
    let boxes: Vec<(Vec2, Vec2)> = q.iter().map(|(k, ..)| (k.body.pos - k.body.half, k.body.pos + k.body.half)).collect();
    let bloods: Vec<Option<platypus_sim::MaterialId>> = q.iter().map(|(.., b)| b.map(|b| b.0)).collect();
    if boxes.is_empty() {
        return;
    }
    let mats = sim.world.materials().clone();
    let mut hurt = vec![0.0f32; boxes.len()];
    let mut shove = vec![Vec2::ZERO; boxes.len()];
    for p in sim.world.particles_mut() {
        if p.landing != platypus_sim::Landing::Settle {
            continue;
        }
        let v = Vec2::new(p.vel[0], p.vel[1]) * TICK_HZ as f32;
        let speed = v.length();
        let ph = mats.phys(p.cell.material);
        let safe = if ph.kind == Kind::Liquid { PELT_SAFE_LIQUID } else { PELT_SAFE };
        if speed < safe {
            continue;
        }
        let at = Vec2::new(p.pos[0], p.pos[1]);
        let Some(i) = boxes.iter().position(|(lo, hi)| at.cmpge(*lo).all() && at.cmple(*hi).all()) else { continue };
        if bloods[i] == Some(p.cell.material) {
            continue;
        }
        let weight = (ph.density as f32 / 1000.0).clamp(0.2, 5.0) * if ph.kind == Kind::Liquid { 0.5 } else { 1.0 };
        hurt[i] += weight * (speed - safe) / safe * PELT;
        shove[i] += v * weight * 0.02;
        p.vel = [p.vel[0] * 0.3, p.vel[1] * 0.3];
    }
    for (i, (mut k, mut h, dig, _)) in q.iter_mut().enumerate() {
        // (A digger shrugs off the dirt it's digging.)
        if hurt[i] > 0.0 && dig.is_none_or(|d| d.face.is_empty()) {
            h.harm(hurt[i], Harm::Blunt);
            k.body.vel += shove[i];
        }
    }
}

/// Every explosion (a bomb, a fireball, a gas pocket, lightning's burst)
/// hurts and throws the creatures near it, less the farther they are: out
/// to 1.6 times its radius, up to 0.65 × its power in damage and 3 × in
/// knockback (a bomb: 90 and 420).
/// What a blast throws: its body, health, spin, and whether it's the player.
type Blasted<'a> = (Entity, &'a mut Kinematics, &'a mut Health, Option<&'a mut body::tumble::Tumble>, Has<player::LocalPlayer>, Option<&'a crate::combat::Sturdy>);

fn blasted(mut commands: Commands, mut blasts: MessageReader<crate::fx::Explosion>, mut q: Query<Blasted>) {
    for b in blasts.read() {
        let reach = b.radius * 1.6;
        for (e, mut k, mut health, tumble, is_player, sturdy) in &mut q {
            let d = k.body.pos - b.at;
            let dist = d.length();
            if dist > reach {
                continue;
            }
            let f = 1.0 - dist / reach;
            let dealt = health.harm(b.power * 0.65 * f, Harm::Blunt);
            let gibs = !is_player && (health.hp <= 0.0 || dealt >= health.max * GIB_SHARE);
            let dir = (d.normalize_or(Vec2::Y) + Vec2::new(0.0, 0.6)).normalize();
            let k = &mut *k;
            let knock = dir * b.power * 4.5 * (0.4 + 0.6 * f);
            // (A creature takes a share of it, less the heavier it is, and no
            // more than so much (a blast throws harder than a blade); the
            // player all of it.)
            let knock = if is_player { knock } else { (knock * crate::combat::KNOCK_TAKEN / sturdy.map_or(1.0, |s| s.heft).max(1.0)).clamp_length_max(crate::combat::KNOCK_MOST * 1.5) };
            k.loco.knock(&mut k.body, knock, 0.35);
            if gibs {
                commands.entity(e).insert(Gibbed(dir * b.power * 1.2));
            }
            // (Thrown spinning: `tumble`; not the player.)
            if !is_player {
                let spin = body::tumble::spin_of(knock, sturdy.map_or(1.0, |s| s.heft));
                match tumble {
                    Some(mut t) => t.omega = (t.omega + spin).clamp(-30.0, 30.0),
                    None => {
                        commands.entity(e).insert(body::tumble::Tumble { angle: 0.0, omega: spin });
                    }
                }
            }
        }
    }
}

/// A creature died (not the player): what its body leaves (`hands::corpses`)
/// needs to know what it was, where, how it looked and what it had on.
#[derive(Message, Clone)]
pub struct Died {
    /// What kind it was (its creature file's name), and that kind.
    pub kind: String,
    pub def: std::sync::Arc<def::CreatureDef>,
    pub body: platypus_physics::Body,
    pub facing: f32,
    /// Its picture as it fell, and where it sat on the body.
    pub sprite: Option<(Sprite, Vec3)>,
    pub worn: Vec<crate::hands::items::Stack>,
    /// How it was turning as it died (`Tumble`): its body goes on so.
    pub spin: body::tumble::Tumble,
    /// It'll get up again (`body::rise`): its body is a heap of bones, with
    /// what it wore and held and no loot yet, and it isn't a kill.
    pub rising: Option<body::rise::RiseDef>,
    /// How many times it had got up already (`RiseDef::times`).
    pub risen: u32,
    /// Burnt away (a body that got up, set alight): it lies charred.
    pub charred: bool,
    /// Its limbs as they were (a walker's legs, arms, segments, tail: its
    /// `Parts`, weak spots left out): its ragdoll's pieces (`ragdoll`).
    pub limbs: Vec<body::parts::Part>,
    /// What it had lost (`wounds`): not drawn on its body.
    pub severed: Vec<body::parts::Limb>,
    /// Who it was, through every time it fell and got up (`rise::Lineage`).
    pub lineage: u64,
    /// Its wounds' marks (`marks`): its body keeps them.
    pub marks: Vec<body::marks::Mark>,
    /// Blown apart (a blast that killed it, or hurt it badly): the blast's
    /// push on its gibs (`corpses`).
    pub gib: Option<Vec2>,
}

/// A blast hurt it badly or killed it: if it dies of it, it's blown apart
/// (the push the blast gave it).
#[derive(Component, Clone, Copy, Debug)]
pub struct Gibbed(pub Vec2);

/// A blast that does this much of a creature's health, or kills it, blows
/// it apart if it dies.
const GIB_SHARE: f32 = 0.35;

/// What a body took in a tick, of one kind (`Health::took`, from every
/// source: blows, spells, burning, falls): what was meant, what got through
/// (below 0: drunk in), and whether it's dead.
/// Written just before deaths, so the killing blow is in it.
#[derive(Message, Clone, Copy, Debug)]
pub struct Took {
    pub target: Entity,
    pub harm: Harm,
    pub meant: f32,
    pub dealt: f32,
    pub killed: bool,
}

/// Each body's takings since the last tick, as `Took`s.
pub(crate) fn tally(mut took: MessageWriter<Took>, mut q: Query<(Entity, &mut Health)>) {
    for (e, mut h) in &mut q {
        if h.took.iter().all(|t| t.0 == 0.0) {
            continue;
        }
        let killed = h.hp <= 0.0;
        for (harm, t) in Harm::ALL.into_iter().zip(std::mem::take(&mut h.took)) {
            if t.0 > 0.0 {
                took.write(Took { target: e, harm, meant: t.0, dealt: t.1, killed });
            }
        }
    }
}

type Mortal<'a> = (
    Entity,
    &'a mut Health,
    &'a mut Kinematics,
    Has<player::LocalPlayer>,
    Option<&'a body::hurt::Bleeds>,
    Option<&'a body::animation::Animator>,
    Option<&'a Creature>,
    Option<&'a crate::gear::Equipment>,
    Option<&'a Children>,
);

#[allow(clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    (mut deaths, checkpoint, mut respawned): (ResMut<PlayerDeaths>, Res<crate::respawn::Checkpoint>, MessageWriter<crate::respawn::Respawned>),
    mut died: MessageWriter<Died>,
    sprites: Query<(&Sprite, &Transform, Has<body::legs::LegBody>), Or<(With<body::animation::CreatureSprite>, With<body::legs::LegBody>)>>,
    mut q: Query<Mortal>,
    tumbles: Query<&body::tumble::Tumble>,
    parts: Query<&body::parts::Parts>,
    severed: Query<&body::parts::Severed>,
    lineages: Query<(Option<&body::rise::Lineage>, Option<&Stable>)>,
    wounds: Query<&body::marks::Marks>,
    gibbed: Query<&Gibbed>,
) {
    for (entity, mut h, mut k, is_player, bleeds, anim, creature, eq, children) in &mut q {
        if h.hp > 0.0 {
            continue;
        }
        if let (Some(anim), true, false) = (anim, creature.is_some(), is_player) {
            // (A walker's body as its legs carried it, where they carried it,
            // level: its own sprite's hidden, somewhere else. Its tail and
            // legs are pinned on where they were.)
            let shown: Vec<_> = children.into_iter().flat_map(|ch| ch.iter()).filter_map(|e| sprites.get(e).ok()).collect();
            let sprite = shown.iter().find(|(.., legged)| *legged).or(shown.first()).map(|&(s, tf, legged)| {
                let mut s = s.clone();
                if let (true, Some(atlas)) = (legged, s.texture_atlas.as_mut()) {
                    atlas.index = crate::combat::Turned::index(0.0);
                }
                (s, tf.translation)
            });
            let worn = eq.map(|eq| eq.worn.iter().flatten().chain(eq.held.iter()).copied().collect()).unwrap_or_default();
            let kind = creature.map_or_else(String::new, |c| c.kind.clone());
            let spin = tumbles.get(entity).copied().unwrap_or_default();
            // (Its limbs, but what's been cut off already lies apart.)
            let cut = severed.get(entity).ok();
            let limbs = parts.get(entity).map(|p| p.limbs().into_iter().filter(|l| l.limb.is_none_or(|x| !cut.is_some_and(|c| c.has(x)))).collect()).unwrap_or_default();
            died.write(Died { kind, def: anim.def.clone(), body: k.body, facing: k.loco.facing, sprite, worn, spin, rising: None, risen: 0, charred: false, limbs, severed: cut.map(|c| c.0.clone()).unwrap_or_default(), lineage: lineage_of(lineages.get(entity).ok()), marks: wounds.get(entity).map(|m| m.list.clone()).unwrap_or_default(), gib: gibbed.get(entity).ok().map(|g| g.0) });
        }
        if let Some(&body::hurt::Bleeds(blood)) = bleeds {
            // A burst of real blood cells: they fly, land, run and pool.
            sim.world.splash([k.body.pos.x, k.body.pos.y], blood, body::hurt::DEATH_BLOOD, 3.9);
        }
        if is_player {
            deaths.0 += 1;
            commands.entity(entity).remove::<(body::elements::Burning, body::elements::Coated, body::elements::Chilled)>();
            h.hp = h.max;
            // (Back at the checkpoint, `respawn.rs`; a scenario or a
            // replay, where it stands, so the run goes on as written.)
            if !crate::respawn::heals_in_place() {
                let died_at = k.body.pos;
                k.body.pos = crate::respawn::respawn_at(&sim, &checkpoint, k.body.half);
                k.body.vel = Vec2::ZERO;
                k.prev_pos = k.body.pos;
                respawned.write(crate::respawn::Respawned { died_at });
            }
        } else {
            commands.entity(entity).despawn();
        }
    }
}

/// Render between the last two ticks so 120 Hz displays stay smooth.
/// (Paused, as the arena pauses to step a tick at a time: where it is now.)
pub(crate) fn interpolate(time: Res<Time<Fixed>>, virt: Res<Time<Virtual>>, mut q: Query<(&Kinematics, &mut Transform, Option<&mut StepEase>)>) {
    let a = if virt.is_paused() { 1.0 } else { time.overstep_fraction() };
    let dt = virt.delta_secs();
    for (k, mut tf, ease) in &mut q {
        let p = k.prev_pos.lerp(k.body.pos, a);
        tf.translation.x = p.x;
        tf.translation.y = p.y;
        // (A step: drawn going to it.)
        if let Some(mut e) = ease
            && e.0 != 0.0
        {
            let go = STEP_EASE * dt;
            e.0 = if e.0 < 0.0 { (e.0 + go).min(0.0) } else { (e.0 - go).max(0.0) };
            tf.translation.y += e.0;
        }
    }
}
