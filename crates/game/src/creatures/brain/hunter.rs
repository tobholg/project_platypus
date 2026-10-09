//! `hunter`: the general fighting brain (DESIGN §14.1). Every creature that
//! hunts uses it, set in its file from a few choices:
//!
//! - `aggro`: it notices what it hunts (players and villagers, not one
//!   hiding at home) within this (cells).
//! - `close`: how it goes at what it's noticed:
//!   - `Walk(keep, jump_to_reach)`: on foot to `keep` cells off, jumping
//!     walls, and up at you when you're `jump_to_reach` higher and near
//!     (orcs, skeletons, the troll);
//!   - `Range(near, far)`: on foot, keeping between `near` and `far`
//!     (archers);
//!   - `Swoop(hover, dive_time, dive_every)`: flying in a band (cells over
//!     the ground) and diving at you now and then (bats, star wisps);
//!   - `Hop(every)`: a hop at you every so often (slimes);
//!   - `Crawl(pounce_range, pounce_every, keep)`: over floors, walls and
//!     ceilings (its movement's `cling`), stopping `keep` cells off,
//!     pouncing when near or right above you (spiders).
//! - `attack`: `Touch` (what it touches it hurts: its file's `touch`, and
//!   its own moves), `Swing(reach, every, combo)` (what it wields, `combo`
//!   moves in a row when you're within `reach`, then a wait of `every` s),
//!   `Shoot(draw, every, wobble)` (its bow: a `draw` of a full draw, leading
//!   you, a shot every `every` s, `wobble` degrees off).
//! - `wander`: with no one about, on foot or crawling: how fast (a share of
//!   its run) and how often it changes its mind (seconds).
//! - `leash`: one keeping a place (`clock::Keeps`: a star's crater) goes
//!   for you only within this of it, and drifts back when you've gone.
//! - On the march (`Marching`: a raid), with no one to fight it walks there
//!   instead of wandering.
//!
//! What a creature file said as a brain of its own before (`melee_walker`,
//! `archer`, `swooper`, `hopper`, `crawler`) is one of these settings now.

use bevy::prelude::*;
use platypus_sim::rng::Rng;
use serde::Deserialize;

use super::RegisterBrain;
use super::ai::Marching;
use crate::creatures::{Controls, Kinematics, Team};
use crate::world::SimWorld;

pub struct HunterPlugin;

impl Plugin for HunterPlugin {
    fn build(&self, app: &mut App) {
        app.register_brain::<Hunter>("hunter").add_systems(FixedUpdate, (super::senses::perceive, crate::creatures::moves::start, hunt, crate::creatures::moves::run).chain().in_set(super::BrainSet));
    }
}

const TICKS: f32 = crate::world::TICK_HZ as f32;

/// A swooper further than this from what it's after flies at it (nearer,
/// it flits in its band and dives).
const SWOOP_NEAR: f32 = 160.0;

/// The settings (its file's `brain.params`).
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Hunter {
    pub aggro: f32,
    pub close: Close,
    pub attack: Attack,
    pub wander: Wander,
    pub leash: f32,
    /// How it senses what it hunts (`senses.rs`: sight, in the dark,
    /// hearing, smell, memory).
    pub senses: super::senses::SensesDef,
    /// How it fights beyond going at you (`tactics.rs`: flee, ambush, shun
    /// the light, call, flank).
    pub tactics: super::tactics::TacticsDef,
}

impl Default for Hunter {
    fn default() -> Self {
        Hunter { aggro: 300.0, close: Close::default(), attack: Attack::Touch, wander: Wander::default(), leash: 0.0, senses: super::senses::SensesDef::default(), tactics: super::tactics::TacticsDef::default() }
    }
}

/// How it goes at what it's noticed.
#[derive(Deserialize, Clone, Debug)]
pub enum Close {
    Walk {
        #[serde(default = "keep")]
        keep: f32,
        #[serde(default = "jump_to_reach")]
        jump_to_reach: f32,
    },
    Range { near: f32, far: f32 },
    Swoop { hover: (f32, f32), dive_time: f32, dive_every: f32 },
    Hop { every: f32 },
    Crawl {
        pounce_range: f32,
        pounce_every: f32,
        /// How near it comes, across (cells, middle to middle): a big
        /// spider stays a leg's length off and strikes from there.
        #[serde(default = "crawl_keep")]
        keep: f32,
        /// Scurrying (an insect's way of going): darts of about `dart.0`
        /// seconds, each followed by a freeze of about `dart.1` (each ±30 %);
        /// none: steady.
        #[serde(default)]
        dart: Option<(f32, f32)>,
    },
}

impl Default for Close {
    fn default() -> Self {
        Close::Walk { keep: keep(), jump_to_reach: jump_to_reach() }
    }
}

fn crawl_keep() -> f32 {
    3.0
}

fn keep() -> f32 {
    15.0
}

fn jump_to_reach() -> f32 {
    27.0
}

/// How it attacks.
#[derive(Deserialize, Clone, Debug, Default)]
pub enum Attack {
    #[default]
    Touch,
    Swing { reach: f32, every: f32, combo: u8 },
    Shoot { draw: f32, every: f32, wobble: f32 },
}

/// With no one about: how fast it potters (a share of its run), how often
/// it changes its mind (seconds, ±50 %).
#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(default)]
pub struct Wander {
    pub speed: f32,
    pub every: f32,
}

impl Default for Wander {
    fn default() -> Self {
        Wander { speed: 0.4, every: 2.5 }
    }
}

/// What a hunter remembers (ticks).
#[derive(Component, Default)]
pub struct HunterMind {
    /// Where it's wandering, until when.
    wander: Vec2,
    wander_until: u64,
    /// When it may act next (pounce, hop, attack, shot), a dive's end.
    next: u64,
    dive_until: u64,
    /// A swing under way: moves still to ask for, whether one's on.
    swings_left: u8,
    attacking: bool,
    /// Drawing its bow since.
    drawing: Option<u64>,
    /// Scurrying: frozen till, the next freeze from.
    still_until: u64,
    dart_until: u64,
    /// In a pack that flanks: the side of you it fights from (−1 left, 1
    /// right; 0: not chosen yet), chosen once as the hunt begins.
    side: f32,
    /// Skirmishing: coming in now (since), waiting out till, and whether
    /// it was busy with a move last tick (a move done: back out).
    charging: bool,
    charge_since: u64,
    wait_until: u64,
    was_busy: bool,
    /// Headway: the nearest it's come to what it hunts (cells), when, and
    /// where it was then; burrowing (its way dug through if that's the
    /// way) till when.
    best: f32,
    best_at: u64,
    best_pos: Vec2,
    burrow_until: u64,
}

/// What hunters go after (a villager hiding at home is let be).
type Hunted<'w, 's> = Query<'w, 's, (&'static Kinematics, &'static Team), Without<super::villager::Hiding>>;

/// Its way (`way.rs`), what it is (its profile's kind), how wary, its pack.
type Finding<'a> = (&'a crate::creatures::Creature, Option<&'a mut super::way::Way>, Option<&'a mut super::senses::Alert>, Option<&'a super::tactics::Pack>);

type Hunting<'a> = (
    Entity,
    &'a Hunter,
    &'a Kinematics,
    &'a mut Controls,
    Option<&'a mut HunterMind>,
    Option<&'a Marching>,
    Option<&'a crate::clock::Keeps>,
    Has<crate::combat::Swing>,
    Option<&'a crate::combat::Wielding>,
    Option<&'a crate::creatures::moves::Moves>,
    Finding<'a>,
);

/// 0..1 from a roll seeded by the world, the tick, the creature and a salt.
fn unit(sim: &SimWorld, id: u64, salt: u64) -> f32 {
    let mut rng = Rng::seeded(&[sim.world.seed(), sim.world.tick(), id, salt]);
    (rng.next_u32() % 10_001) as f32 / 10_000.0
}

/// Seconds as ticks.
fn ticks(secs: f32) -> u64 {
    (secs.max(0.0) * TICKS) as u64
}

/// Which way a march goes from `x` (0: there, or not marching).
fn march(m: Option<&Marching>, x: f32) -> f32 {
    super::ai::march(m, x)
}

#[allow(clippy::too_many_arguments)]
fn hunt(
    mut commands: Commands,
    sim: Res<SimWorld>,
    weapons: Option<Res<crate::combat::Weapons>>,
    hunted: Hunted,
    mut q: Query<Hunting>,
    mut swings: MessageWriter<crate::combat::MeleeRequest>,
    mut draws: MessageWriter<crate::archery::DrawBow>,
    ids: Query<&crate::creatures::Stable>,
    (mut ways, creatures, tempo): (ResMut<super::way::Ways>, Res<crate::creatures::def::Creatures>, Res<crate::tempo::Tempo>),
    packs: Query<(&super::tactics::Pack, &Kinematics)>,
    // Skirmishers coming in, per pack: last tick's count, this tick's.
    mut charging: Local<(std::collections::HashMap<u64, u32>, std::collections::HashMap<u64, u32>)>,
) {
    let tick = sim.world.tick();
    let (charging_then, charging_now) = &mut *charging;
    std::mem::swap(charging_then, charging_now);
    charging_now.clear();
    // Each pack's leader: its first still living, and where it is.
    let mut leaders: std::collections::HashMap<u64, (u32, Vec2)> = std::collections::HashMap::new();
    for (p, pk) in &packs {
        let l = leaders.entry(p.id).or_insert((p.rank, pk.body.pos));
        if p.rank < l.0 {
            *l = (p.rank, pk.body.pos);
        }
    }
    for (e, h, k, mut c, mind, marching, keeps, swinging, wielding, moves, (kind, way, mut alert_mut, pack)) in &mut q {
        let alert = alert_mut.as_deref();
        let (Some(mut m), Some(mut way)) = (mind, way) else {
            commands.entity(e).insert((HunterMind::default(), super::way::Way::default()));
            continue;
        };
        let id = crate::creatures::stable(&ids, e);
        let pos = k.body.pos;
        // (On a leash: only what's near its place.)
        let home = keeps.filter(|_| h.leash > 0.0).map(|kp| Vec2::new(kp.0.0 as f32, kp.0.1 as f32));
        // What it goes for: what its senses tell it (`senses.rs`): its
        // quarry where it is when it senses it, else where it last knew it
        // to be (suspicious, searching); nothing, idle. (No senses yet: the
        // nearest quarry in its aggro, as before.)
        let target = match alert {
            Some(a) => a.goal().map(|g| match a.target.filter(|_| a.engaged()).and_then(|t| hunted.get(t).ok()) {
                Some((pk, _)) => (pk.body.pos, pk.body.vel, pk.body.half),
                None => (g, Vec2::ZERO, Vec2::new(4.0, 8.0)),
            }),
            None => hunted
                .iter()
                .filter(|(_, t)| t.hunted())
                .map(|(pk, _)| (pk.body.pos, pk.body.vel, pk.body.half))
                .filter(|(p, ..)| p.distance(pos) < h.aggro && home.is_none_or(|hm| p.distance(hm) < h.leash))
                .min_by(|a, b| a.0.distance_squared(pos).total_cmp(&b.0.distance_squared(pos))),
        };
        // (Only what it senses now does it strike at; looking, suspicious or
        // searching, it goes slower.)
        let blind = alert.is_some_and(|a| !a.engaged());
        let looking = alert.is_some_and(|a| a.looking());
        // Its tactics (`tactics.rs`): running for it; kept off by the light
        // on its quarry; its pack's leader (not itself), to follow.
        let fleeing = alert.is_some_and(|a| a.fleeing);
        let shy = h.tactics.shun_light > 0.0 && alert.is_some_and(|a| a.quarry_lit > SHUN_LIT) && target.is_some_and(|t| t.0.distance(pos) < h.tactics.shun_light);
        let leader = pack.and_then(|p| leaders.get(&p.id).filter(|l| l.0 != p.rank)).map(|l| l.1);
        // (Burrowing: a digger hunting what it can't get nearer to, for a
        // while, goes the way its planner finds, digging where that's
        // quicker than round: through what's in the way, a crack too small,
        // a tunnel you dug. Hunting: what it sees, or lost a moment ago.)
        let digger = creatures.get(&kind.kind).is_some_and(|d| d.dig.is_some());
        let hunting = alert.is_some_and(|a| a.wary == super::senses::Wary::Hunting && !a.fleeing) && !shy;
        let burrowing = match target {
            Some((t, ..)) if digger && hunting => {
                let d = t.distance(pos);
                // (Headway: nearer, or on the move: climbing a face under
                // what it hunts is getting there.)
                if d < m.best - HEADWAY || m.best_at == 0 || pos.distance(m.best_pos) > MOVED_ON {
                    m.best = if m.best_at == 0 { d } else { m.best.min(d) };
                    m.best_at = tick.max(1);
                    m.best_pos = pos;
                }
                if tick.saturating_sub(m.best_at) > ticks(NO_HEADWAY) && d > STUCK_NEAR && tick >= m.burrow_until {
                    m.burrow_until = tick + ticks(BURROW);
                    m.best = d;
                    m.best_at = tick;
                    m.best_pos = pos;
                    if std::env::var("PLATYPUS_ALERTLOG").is_ok() {
                        info!("burrow: {e:?} {} burrows ({d:.0} off, no nearer for {NO_HEADWAY} s)", kind.kind);
                    }
                }
                tick < m.burrow_until
            }
            _ => {
                m.best_at = 0;
                m.burrow_until = 0;
                false
            }
        };

        // The way to it, where it can't be gone at straight (`way.rs`):
        // what to press, if a way's known.
        let feet = pos - Vec2::Y * k.body.half.y;
        let route = |ways: &mut super::way::Ways, way: &mut super::way::Way, goal: Vec2| ways.steer(&sim.world, &creatures, &tempo, kind, k, way, goal, tick);
        let walkable = |t: Vec2, th: Vec2| super::way::straight(&sim.world, feet, t - Vec2::Y * th.y, k.body.half, k.body.step_height as f32);
        let mut steer: Option<super::way::Steer> = None;
        let contacts = k.loco.contacts;
        let grounded = k.loco.grounded();
        // (Stunned, or busy with a move of its own: no swing, no shot.)
        let stunned = k.loco.state == platypus_physics::MoveState::Stunned || moves.is_some_and(|m| m.busy()) || blind || shy;
        c.0.aim = target.map_or(Vec2::ZERO, |t| t.0);
        let mut jump = false;
        let mut move_y = 0.0;
        // (A flanker on its way round: no move started till it's there.)
        let mut en_route = false;
        let mut move_x: f32 = match (&h.close, target) {
            // On foot at it: up to `keep` off, its weapon's combo when near.
            (Close::Walk { keep, jump_to_reach }, Some((t, _, th))) => {
                let d = t - pos;
                // (Far across, or out of reach above or below, and not a
                // straight walk: the way there.)
                if (d.x.abs() > *keep || d.y.abs() > *jump_to_reach) && (!walkable(t, th) || burrowing) {
                    steer = route(&mut ways, &mut way, t - Vec2::Y * th.y);
                }
                // (Up at you when you're above and near; following a way,
                // the way says when to jump.)
                jump = steer.is_none() && grounded && d.y > *jump_to_reach && d.x.abs() < h.aggro * 0.25;
                if let Attack::Swing { reach, every, combo } = h.attack {
                    let near = d.x.abs() < reach && d.y.abs() < reach;
                    if m.attacking && !swinging && m.swings_left == 0 {
                        m.attacking = false;
                        m.next = tick + ticks(every * (0.7 + 0.6 * unit(&sim, id, 0xA77)));
                    }
                    if near && !stunned && reach > 0.0 && tick >= m.next && !m.attacking && !swinging {
                        m.swings_left = combo.max(1);
                        m.attacking = true;
                    }
                    if m.swings_left > 0 && !stunned {
                        swings.write(crate::combat::MeleeRequest { attacker: e, at: t });
                        // (A request during a swing queues the next move: one each.)
                        if !swinging || combo > 1 {
                            m.swings_left -= 1;
                        }
                    }
                }
                // (Flanking: each of a pack takes a side of you as the hunt
                // begins, and keeps it: the leader's and the even places' the
                // side the pack came from, the odd places' the far side,
                // gone round to over you if they must.)
                let flanks = h.tactics.flank && pack.is_some() && !blind;
                if !flanks {
                    m.side = 0.0;
                } else if m.side == 0.0 {
                    let came = (leader.unwrap_or(pos).x - t.x).signum();
                    let came = if came == 0.0 { 1.0 } else { came };
                    m.side = if pack.is_some_and(|p| p.rank % 2 == 1) { -came } else { came };
                }
                let far = flanks.then_some(m.side);
                // (Skirmishing: waiting `back` off, coming in when its turn
                // comes (a pack's `together` at a time), out again once its
                // move is done.)
                let hold = match &h.tactics.skirmish {
                    Some(sk) if !blind => {
                        let busy = moves.is_some_and(|mv| mv.busy());
                        if m.charging && (m.was_busy && !busy || tick > m.charge_since + ticks(CHARGE_MOST)) {
                            m.charging = false;
                            m.wait_until = tick + ticks(sk.wait * (0.6 + 0.8 * unit(&sim, id, 0x5C1)));
                        }
                        m.was_busy = busy;
                        // (Those in last tick, or gone in already this one.)
                        let room = pack.is_none_or(|p| charging_then.get(&p.id).copied().unwrap_or(0).max(charging_now.get(&p.id).copied().unwrap_or(0)) < sk.together);
                        if !m.charging && tick >= m.wait_until && room {
                            m.charging = true;
                            m.charge_since = tick;
                        }
                        if m.charging
                            && let Some(p) = pack
                        {
                            *charging_now.entry(p.id).or_default() += 1;
                        }
                        (!m.charging).then_some(sk.back)
                    }
                    _ => {
                        m.charging = false;
                        None
                    }
                };
                match (steer, far, hold) {
                    _ if swinging => 0.0,
                    (Some(s), ..) => s.move_x,
                    (None, Some(f), _) => {
                        let gx = t.x + f * hold.unwrap_or(keep + FLANK_PAST) - pos.x;
                        let wrong_side = (pos.x - t.x).signum() != f;
                        if wrong_side && d.x.abs() < VAULT && grounded {
                            jump = true;
                        }
                        en_route = wrong_side || hold.is_some() || gx.abs() > keep * 0.5 + 4.0;
                        if gx.abs() < 4.0 { 0.0 } else { gx.signum() }
                    }
                    (None, None, Some(back)) => {
                        let side = if pos.x >= t.x { 1.0 } else { -1.0 };
                        let gx = t.x + side * back - pos.x;
                        en_route = true;
                        if gx.abs() < 6.0 { 0.0 } else { gx.signum() }
                    }
                    (None, None, None) if d.x.abs() <= *keep => 0.0,
                    (None, None, None) => d.x.signum(),
                }
            }
            // On foot, at a distance: back off, close in, draw and loose.
            (Close::Range { near, far }, Some((t, tv, th))) => {
                let d = t - pos;
                // (Too far, and not straight there: the way round.)
                if d.x.abs() > *far && (!walkable(t, th) || burrowing) {
                    steer = route(&mut ways, &mut way, t - Vec2::Y * th.y);
                }
                let bow = match (&h.attack, &weapons) {
                    (Attack::Shoot { .. }, Some(w)) => wielding.and_then(|wd| wd.0.as_deref()).and_then(|id| w.bow_index(id)).map(|i| w.bow(i).clone()),
                    _ => None,
                };
                match (h.attack.clone(), bow, &weapons) {
                    (Attack::Shoot { draw, every, wobble }, Some(bow), Some(w)) => {
                        if let Some(since) = m.drawing {
                            // Drawing: keep at it, then let go.
                            let held = (tick - since) as f32 / TICKS;
                            if stunned || held >= bow.draw * draw {
                                m.drawing = None;
                                m.next = tick + ticks(every * (0.7 + 0.6 * unit(&sim, id, 0xB0E)));
                            } else {
                                // Where it'll be when the arrow gets there, and the drop.
                                let speed = bow.speed.0 + (bow.speed.1 - bow.speed.0) * draw.min(1.0);
                                let flight = d.length() / speed.max(1.0);
                                let g = w.arrow_def().map_or(0.0, |a| a.gravity);
                                let aim = t + tv * flight + Vec2::new(0.0, 0.5 * g * flight * flight);
                                // (A wobble per shot, not per tick.)
                                let mut rng = Rng::seeded(&[sim.world.seed(), since, id, 0xA1A]);
                                let off = ((rng.next_u32() % 2001) as f32 / 1000.0 - 1.0) * wobble.to_radians();
                                let aim = pos + Vec2::from_angle(off).rotate(aim - pos);
                                draws.write(crate::archery::DrawBow { archer: e, at: aim });
                            }
                            0.0
                        } else if !stunned && tick >= m.next && d.x.abs() <= far * 1.2 && grounded {
                            m.drawing = Some(tick);
                            draws.write(crate::archery::DrawBow { archer: e, at: t });
                            0.0
                        } else {
                            steer.map_or_else(|| range(d, *near, *far), |s| s.move_x)
                        }
                    }
                    _ => steer.map_or_else(|| range(d, *near, *far), |s| s.move_x),
                }
            }
            // Flying: a dive at it now and then, else hovering in its band.
            // Far off, or out of sight behind something: there (the way
            // round, flying, if it can't go straight).
            (Close::Swoop { .. }, Some((t, _, th))) if pos.distance(t) > SWOOP_NEAR || !crate::creatures::moves::clear(&sim, pos, t) => {
                let s = if crate::creatures::moves::clear(&sim, pos, t) {
                    let d = (t - pos).normalize_or_zero();
                    super::way::Steer { move_x: d.x, move_y: d.y, ..default() }
                } else {
                    steer = route(&mut ways, &mut way, t - Vec2::Y * th.y);
                    steer.unwrap_or_default()
                };
                move_y = s.move_y;
                s.move_x
            }
            (Close::Swoop { hover, dive_time, dive_every }, Some((t, ..))) => {
                let steer = if tick < m.dive_until {
                    (t - pos).normalize_or_zero()
                } else if tick >= m.next {
                    m.dive_until = tick + ticks(*dive_time);
                    m.next = m.dive_until + ticks(dive_every * (0.7 + 0.6 * unit(&sim, id, 7)));
                    (t - pos).normalize_or_zero()
                } else {
                    flit(&sim, id, tick, &mut m, pos, *hover, home, h.leash)
                };
                move_y = if steer.y == 0.0 { 0.15 } else { steer.y };
                steer.x
            }
            (Close::Swoop { hover, .. }, None) => {
                let steer = flit(&sim, id, tick, &mut m, pos, *hover, home, h.leash);
                move_y = if steer.y == 0.0 { 0.15 } else { steer.y };
                let go = march(marching, pos.x);
                if go != 0.0 { go } else { steer.x }
            }
            // Hopping: down, it waits; a hop at it every so often. Not a
            // straight way there: along the way as the way says.
            (Close::Hop { every }, target) => {
                if let Some((t, _, th)) = target
                    && (!walkable(t, th) || burrowing)
                {
                    steer = route(&mut ways, &mut way, t - Vec2::Y * th.y);
                }
                match steer {
                    // (Following a way it goes as the way says: a hop's
                    // landing is too rough to line up a jump from.)
                    Some(s) => {
                        jump = s.jump;
                        s.move_x
                    }
                    None => {
                        let mut mx = 0.0;
                        if grounded && tick >= m.next {
                            let dir = match (target, steer) {
                                (Some((t, ..)), _) => (t.x - pos.x).signum(),
                                // (Idle: a hop now and then, either way.)
                                (None, _) if unit(&sim, id, 4) < 0.3 => {
                                    if unit(&sim, id, 5) < 0.5 {
                                        -1.0
                                    } else {
                                        1.0
                                    }
                                }
                                (None, _) => 0.0,
                            };
                            if dir != 0.0 {
                                jump = true;
                                mx = dir;
                            }
                            let every = if target.is_some() { *every } else { every * 2.5 };
                            m.next = tick + ticks(every * (0.7 + 0.6 * unit(&sim, id, 6)));
                        }
                        // (Airborne, it keeps going the way it hopped.)
                        if grounded { mx } else { c.0.move_x }
                    }
                }
            }
            // Crawling: up walls toward it, along ceilings; a pounce when
            // near, or dropping on it from above.
            (Close::Crawl { pounce_range, pounce_every, keep, dart }, Some((t, _, th))) => {
                let d = t - pos;
                move_y = if d.y > 6.0 {
                    1.0
                } else if d.y < -6.0 {
                    -1.0
                } else {
                    0.0
                };
                // Out of sight, or burrowing: the way round, over walls and
                // ceilings (and through, a digger).
                if !crate::creatures::moves::clear(&sim, pos, t) || burrowing {
                    steer = route(&mut ways, &mut way, t - Vec2::Y * th.y);
                    if let Some(s) = steer {
                        move_y = s.move_y;
                    }
                }
                let clinging = k.loco.clinging();
                // (On a ceiling, near enough across that it would go no
                // nearer (its `keep`): it lets go, down at you.)
                let over_you = clinging == Some(Vec2::Y) && d.y < 0.0 && d.x.abs() < keep.max(12.0) + 4.0;
                let near = d.length() < *pounce_range;
                if tick >= m.next && (over_you || (near && (grounded || clinging.is_some()))) {
                    jump = true;
                    m.next = tick + ticks(pounce_every * (0.7 + 0.6 * unit(&sim, id, 1)));
                }
                let mx = match steer {
                    Some(s) => s.move_x,
                    None if d.x.abs() > *keep => d.x.signum(),
                    None => 0.0,
                };
                // (Scurrying: a dart, a freeze, a dart. Not up close: there
                // it strikes.)
                let mut mx = mx;
                if let Some((go, rest)) = *dart
                    && d.length() > keep * 1.5
                {
                    if tick >= m.dart_until && tick >= m.still_until {
                        m.still_until = tick + ticks(rest * (0.7 + 0.6 * unit(&sim, id, 7)));
                        m.dart_until = m.still_until + ticks(go * (0.7 + 0.6 * unit(&sim, id, 8)));
                    }
                    if tick < m.still_until {
                        mx = 0.0;
                        move_y = 0.0;
                    }
                }
                // (Holding a wall it's going into, with nowhere up or down to
                // go: it climbs it, over (its own acid's crater, a step too
                // high), not pressing on it for ever.)
                if move_y == 0.0
                    && let Some(wall) = clinging
                    && wall.x != 0.0
                    && mx * wall.x > 0.0
                {
                    move_y = 1.0;
                }
                // (Holding a wall, its way on along the ceiling just over it
                // (an overhang's corner), away from the wall: up into the
                // corner first, so it touches the ceiling and takes hold. The
                // way finder's grid may have it a little under the ceiling,
                // a little down: up all the same.)
                if move_y <= 0.0
                    && let Some(wall) = clinging
                    && wall.x != 0.0
                    && mx * wall.x < 0.0
                {
                    let top = pos.y + k.body.half.y;
                    let near = (1..=6).any(|d| [-0.5, 0.0, 0.5].iter().any(|s| sim.world.is_solid(platypus_sim::CellPos::from_world(pos.x + s * k.body.half.x, top + d as f32))));
                    if near {
                        move_y = 1.0;
                    }
                }
                // (On the ceiling, its way on up (round the ceiling's edge,
                // onto the face over it): along to the edge first, the side
                // that's open over it.)
                if clinging == Some(Vec2::Y) && move_y > 0.0 && mx == 0.0 {
                    let top = pos.y + k.body.half.y;
                    let open = |x: f32| !(2..=8).any(|d| sim.world.is_solid(platypus_sim::CellPos::from_world(x, top + d as f32)));
                    let (left, right) = (open(pos.x - k.body.half.x - 2.0), open(pos.x + k.body.half.x + 2.0));
                    mx = match (left, right) {
                        (true, false) => -1.0,
                        (false, true) => 1.0,
                        _ => 0.0,
                    };
                }
                // (An ambusher on a ceiling stays up there: still till what
                // it's after comes near under it, then along the ceiling over
                // it, and down on it. Not yet hunting: up to a ceiling. What
                // it's after above it, it goes at as any crawler.)
                if h.tactics.ambush && !jump && d.y < 0.0 {
                    if clinging == Some(Vec2::Y) {
                        move_y = 1.0;
                        mx = if blind || d.x.abs() > AMBUSH_REACH || d.x.abs() < 10.0 { 0.0 } else { d.x.signum() };
                    } else if blind {
                        (mx, move_y) = lurk(&sim, id, tick, &mut m, pos);
                    }
                }
                // (PLATYPUS_STEERLOG: how it steers, every 15 ticks.)
                if std::env::var("PLATYPUS_STEERLOG").is_ok() && tick.is_multiple_of(15) {
                    info!("steer: t {tick} at {:?} to {:?} clear {} steer {:?} clinging {:?} grounded {grounded} → x {mx} y {move_y}", pos.round(), t.round(), crate::creatures::moves::clear(&sim, pos, t), steer, clinging);
                }
                mx
            }
            // (An ambusher with no one about: up to a ceiling, and still
            // there.)
            (Close::Crawl { .. }, None) if h.tactics.ambush => {
                let mx;
                (mx, move_y) = if k.loco.clinging() == Some(Vec2::Y) { (0.0, 1.0) } else { lurk(&sim, id, tick, &mut m, pos) };
                mx
            }
            (Close::Crawl { .. }, None) => {
                if tick >= m.wander_until {
                    m.wander = Vec2::from_angle(unit(&sim, id, 2) * std::f32::consts::TAU);
                    m.wander_until = tick + ticks(1.0 + 2.0 * unit(&sim, id, 3));
                }
                move_y = m.wander.y.signum();
                m.wander.x.signum() * h.wander.speed
            }
            // On foot, no one about: on the march, or pottering.
            (Close::Walk { .. } | Close::Range { .. }, None) => {
                let go = march(marching, pos.x);
                if go != 0.0 {
                    go
                } else {
                    if tick >= m.wander_until {
                        let mut rng = Rng::seeded(&[sim.world.seed(), tick, id]);
                        m.wander.x = [-1.0, 0.0, 1.0][(rng.next_u32() % 3) as usize];
                        m.wander_until = tick + ticks(h.wander.every * (0.5 + (rng.next_u32() % 1000) as f32 / 1000.0));
                    }
                    // (One of a pack strayed from its leader: back to it.)
                    match leader {
                        Some(l) if (l.x - pos.x).abs() > super::tactics::FOLLOW => (l.x - pos.x).signum() * h.wander.speed.max(0.5),
                        _ => m.wander.x * h.wander.speed,
                    }
                }
            }
        };
        // Running for it: away from what it fought (up and away, flying).
        // Kept off by the light: back to the light's edge, and waiting.
        if let Some((t, ..)) = target {
            let away = (pos.x - t.x).signum();
            if fleeing {
                move_x = if away == 0.0 { 1.0 } else { away };
                steer = None;
                if matches!(h.close, Close::Swoop { .. }) {
                    move_y = 0.6;
                }
            } else if shy {
                move_x = if t.distance(pos) < h.tactics.shun_light * 0.85 { away } else { 0.0 };
                steer = None;
            }
        }
        // On foot or hopping, a wall it's walking into: jump it (unless a
        // way's being followed: it knows when to).
        if steer.is_none() && matches!(h.close, Close::Walk { .. } | Close::Range { .. }) && ((move_x > 0.0 && contacts.wall_right) || (move_x < 0.0 && contacts.wall_left)) {
            jump = grounded;
        }
        let hold = steer.is_some_and(|s| s.hold);
        if let Some(s) = steer
            && !matches!(h.close, Close::Hop { .. })
        {
            jump |= s.jump;
        }
        c.0.move_x = if looking && !fleeing { move_x * 0.55 } else { move_x };
        if let Some(a) = alert_mut.as_mut()
            && a.flanking != en_route
        {
            a.flanking = en_route;
        }
        c.0.move_y = move_y;
        // Brains hold buttons; release after a press so the next press
        // registers (a way's jump is held as its arc was).
        c.0.jump = if hold && c.0.jump { true } else { jump && !c.0.jump };
    }
}

/// Burrowing: a digger no nearer (by `HEADWAY` cells) to what it hunts in
/// `NO_HEADWAY` s, and further than `STUCK_NEAR`, goes its planned way,
/// digging, for `BURROW` s (then sees again).
const HEADWAY: f32 = 6.0;
/// Moved this far since it last made headway, it's making it (cells).
const MOVED_ON: f32 = 12.0;
const NO_HEADWAY: f32 = 1.5;
const STUCK_NEAR: f32 = 40.0;
const BURROW: f32 = 6.0;

/// A quarry lit more than this keeps one that shuns the light off (a
/// torch, a lamp, day; the moon's is about 0.1).
const SHUN_LIT: f32 = 0.5;

/// A flanker's place: this far past `keep` on your far side (cells); it
/// goes over you when it's within `VAULT` across on the wrong side.
const FLANK_PAST: f32 = 6.0;
const VAULT: f32 = 45.0;

/// A skirmisher coming in goes back out after this long, its move made or
/// not (s).
const CHARGE_MOST: f32 = 2.5;

/// An ambusher on a ceiling comes along it for what's within this across
/// (cells); further, it waits.
const AMBUSH_REACH: f32 = 120.0;

/// Lurking (an ambusher not yet up): up whatever it touches, along to the
/// nearer wall (within `LURK_LOOK` either way; neither, either way) to go
/// up to a ceiling (its steering: across, up).
fn lurk(sim: &SimWorld, id: u64, tick: u64, m: &mut HunterMind, pos: Vec2) -> (f32, f32) {
    if tick >= m.wander_until {
        let wall = |dir: f32| (1..LURK_LOOK / 4).map(|i| i as f32 * 4.0).find(|&d| sim.world.is_solid(platypus_sim::CellPos::from_world(pos.x + dir * d, pos.y)));
        let dir = match (wall(-1.0), wall(1.0)) {
            (Some(l), Some(r)) => if l < r { -1.0 } else { 1.0 },
            (Some(_), None) => -1.0,
            (None, Some(_)) => 1.0,
            (None, None) => if unit(sim, id, 0x1A) < 0.5 { -1.0 } else { 1.0 },
        };
        m.wander = Vec2::new(dir, 1.0);
        m.wander_until = tick + ticks(2.0 + 2.0 * unit(sim, id, 0x1B));
    }
    (m.wander.x, 1.0)
}

/// How far either way a lurker looks for a wall to go up (cells).
const LURK_LOOK: i32 = 300;

/// Keeping between `near` and `far` across.
fn range(d: Vec2, near: f32, far: f32) -> f32 {
    let dist = d.x.abs();
    if dist < near {
        -d.x.signum()
    } else if dist > far {
        d.x.signum()
    } else {
        0.0
    }
}

/// Flitting about in its band over the ground (and back toward its place,
/// strayed past half its leash).
#[allow(clippy::too_many_arguments)]
fn flit(sim: &SimWorld, id: u64, tick: u64, m: &mut HunterMind, pos: Vec2, hover: (f32, f32), home: Option<Vec2>, leash: f32) -> Vec2 {
    if tick >= m.wander_until {
        m.wander = Vec2::from_angle(unit(sim, id, 8) * std::f32::consts::TAU);
        m.wander_until = tick + ticks(0.3 + 0.4 * unit(sim, id, 9));
    }
    let mut steer = m.wander;
    if let Some(h) = home
        && pos.distance(h) > leash * 0.5
    {
        steer.x = (h.x - pos.x).signum();
    }
    let h = crate::creatures::brain::critters::height(&sim.world, pos, (hover.1 as i32) * 2);
    if h < hover.0 {
        steer.y = steer.y.abs().max(0.7);
    } else if h > hover.1 {
        steer.y = -steer.y.abs().max(0.5);
    }
    steer
}
