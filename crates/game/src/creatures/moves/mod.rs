//! Moves as data (DESIGN §14.1, PLAN BE 2): what creatures do to you, from
//! `assets/data/moves.ron`, named in a creature's file (`moves: [..]`, in
//! the order it tries them).
//!
//! A move is phases (a wind-up to read, the strike, the recovery), each
//! some seconds long, each easing the body's pose (`legs::Rear`: how high it
//! rears, how far it crouches back, how its tail curls) toward its own and
//! doing what it says: a lunge, a strike (what's within reach of a point
//! ahead of it is hit, once a move), a spell cast (aimed to lead a moving
//! target, lobbed if it falls), a slam round its feet, a summons. A move
//! starts when its target is in `range`, it's ready (`every` seconds after
//! the last), and its needs are met (`footing`: standing or clinging;
//! `line`: nothing solid between). While a move is under way the creature
//! stands (its brain's steering is put aside).
//!
//! Weapons' swings (`weapons.ron`), bows and touch are moves of their own
//! kinds already; this is for what a creature does with its own body.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use serde::Deserialize;

use crate::combat::Hit;
use crate::creatures::body::animation::Animator;
use crate::creatures::body::elements::Coatings;
use crate::creatures::body::legs::Rear;
use crate::creatures::{Controls, Harm, Kinematics, Team};
use crate::data::{Watched, data_path, load_ron};
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// After any move, this long before the next (a breath between).
const BREATH: f32 = 0.4;

pub struct MovesPlugin;

impl Plugin for MovesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MoveBook::load()).add_message::<Began>().add_systems(Update, reload);
    }
}

#[derive(Deserialize)]
struct MovesFile {
    moves: Vec<MoveDef>,
}

/// One move.
#[derive(Clone, Debug, Deserialize)]
pub struct MoveDef {
    pub id: String,
    /// When it may start.
    #[serde(default)]
    pub when: When,
    /// Seconds after it ends before it may come again.
    pub every: f32,
    pub phases: Vec<Phase>,
}

/// What a move needs to start.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct When {
    /// The target this far off (cells, from its middle to theirs).
    pub range: (f32, f32),
    /// Standing on something or clinging to it.
    pub footing: bool,
    /// Nothing solid between them.
    pub line: bool,
}

impl Default for When {
    fn default() -> Self {
        When { range: (0.0, 1e9), footing: false, line: false }
    }
}

/// A stretch of a move.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Phase {
    /// What it is, for readouts and the bestiary: `windup`, `strike`,
    /// `recover` (or anything).
    pub name: String,
    pub secs: f32,
    /// The pose it eases to over this phase (the one before held if none).
    pub pose: Option<Pose>,
    pub ease: Ease,
    /// The curl trembling, this much, while it lasts.
    pub tremble: f32,
    /// An animation clip of its art played through it (a rig's pose: the
    /// troll's `reach`), till another phase's or the move's end.
    pub clip: Option<String>,
    pub acts: Vec<Act>,
}

impl Default for Phase {
    fn default() -> Self {
        Phase { name: String::new(), secs: 0.0, pose: None, ease: Ease::Smooth, tremble: 0.0, clip: None, acts: Vec::new() }
    }
}

/// How it's held (`legs::Rear`).
#[derive(Clone, Copy, Debug, Deserialize, Default)]
#[serde(default)]
pub struct Pose {
    pub lift: f32,
    pub back: f32,
    pub curl: f32,
    /// An aiming chain (a scorpion's tail): how far it's drawn back, coiled
    /// tighter (0–1), and how far it's thrown out at the target (0: its
    /// arch, 1: stretched straight at it).
    pub coil: f32,
    pub reach: f32,
    /// Its striking legs (a spider's front pair) raised high ahead (0–1);
    /// dropped back to 0 fast, they come down hard (a stamp).
    pub paw: f32,
}

/// How a phase moves into its pose.
#[derive(Clone, Copy, Debug, Deserialize, Default, PartialEq)]
pub enum Ease {
    /// At once.
    Snap,
    /// Evenly.
    Linear,
    /// Slow, fast, slow.
    #[default]
    Smooth,
}

/// What happens in a phase. Lunge, cast, slam, summon and sound go off as
/// it starts; a strike is live through it (from `from` of the way in) until
/// it lands.
#[derive(Clone, Debug, Deserialize)]
pub enum Act {
    /// Off at the target: `speed` cells/s, and `up` cells/s more upward.
    Lunge { speed: f32, #[serde(default)] up: f32 },
    /// What's within `reach` of a point `at` cells toward the target is hit.
    Strike(Strike),
    /// A spell (spells.ref) cast from `at` cells toward the target and `up`
    /// above: aimed where they'll be, lobbed by as far as it falls.
    Cast { spell: String, #[serde(default)] at: f32, #[serde(default)] up: f32 },
    /// What's within `radius` of its feet is hit.
    Slam(Slam),
    /// `count` of a creature, `spread` cells either side.
    Summon { kind: String, count: u32, #[serde(default)] spread: f32 },
    /// A sound (sounds.ron) where it is.
    Sound(String),
    /// Live through the phase: what's within `reach` of a point `at` cells
    /// toward the target is caught (from `from` of the way in) and held
    /// there, helpless, until the move ends, a `Throw`, or the grabber is
    /// stunned (it lets go). Strikes in later phases land on what's held.
    /// A grab that catches nothing goes straight to the move's last phase.
    Grab(Grab),
    /// What's held let go, flung `speed` cells/s along the way it faces and
    /// `up` more upward (stunned `stun` s).
    Throw { speed: f32, #[serde(default)] up: f32, #[serde(default = "throw_stun")] stun: f32 },
    /// Live through the phase: a held spell (a ray: spells.ron) cast every
    /// tick from `at` cells toward the target, its aim swinging after them
    /// at most `turn` radians a second (outrun it, or get under it).
    Beam { spell: String, #[serde(default)] at: f32, #[serde(default = "beam_turn")] turn: f32 },
}

#[derive(Clone, Debug, Deserialize)]
pub struct Grab {
    pub at: f32,
    pub reach: f32,
    #[serde(default)]
    pub from: f32,
}

fn throw_stun() -> f32 {
    0.4
}

fn beam_turn() -> f32 {
    1.5
}

/// Caught by a grab: held at `at` (where the grabber holds it) while it
/// lasts, its body pinned there (`pin`), no control of its own.
#[derive(Component, Clone, Copy, Debug)]
pub struct Held {
    pub by: Entity,
    pub at: Vec2,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Strike {
    pub at: f32,
    pub reach: f32,
    pub damage: f32,
    #[serde(default = "pierce")]
    pub harm: Harm,
    /// Knockback (cells/s) along the way it faces, and `up` of that upward.
    pub knock: f32,
    #[serde(default)]
    pub up: f32,
    pub stun: f32,
    /// Live from this share of the phase on.
    #[serde(default)]
    pub from: f32,
    /// A coating left on what it hits (coatings.ron: venom).
    #[serde(default)]
    pub coat: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Slam {
    pub radius: f32,
    pub damage: f32,
    #[serde(default = "blunt")]
    pub harm: Harm,
    pub knock: f32,
    pub stun: f32,
}

fn pierce() -> Harm {
    Harm::Pierce
}

fn blunt() -> Harm {
    Harm::Blunt
}

/// Every move, by id.
#[derive(Resource)]
pub struct MoveBook {
    moves: Vec<Arc<MoveDef>>,
    by_id: HashMap<String, usize>,
    watch: Watched,
}

impl MoveBook {
    fn read(path: &std::path::Path) -> Result<Vec<MoveDef>, String> {
        let file: MovesFile = load_ron(path)?;
        for m in &file.moves {
            if m.phases.is_empty() {
                return Err(format!("move `{}` has no phases", m.id));
            }
        }
        Ok(file.moves)
    }

    pub(crate) fn load() -> Self {
        let path = data_path("moves.ron");
        let moves = Self::read(&path).unwrap_or_else(|e| panic!("moves.ron: {e}"));
        let mut book = MoveBook { moves: Vec::new(), by_id: HashMap::new(), watch: Watched::new(path) };
        book.set(moves);
        book
    }

    fn set(&mut self, moves: Vec<MoveDef>) {
        self.by_id = moves.iter().enumerate().map(|(i, m)| (m.id.clone(), i)).collect();
        self.moves = moves.into_iter().map(Arc::new).collect();
    }

    pub fn get(&self, id: &str) -> Option<&Arc<MoveDef>> {
        self.by_id.get(id).map(|&i| &self.moves[i])
    }

    pub fn has(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }
}

fn reload(mut book: ResMut<MoveBook>, mut asked: MessageReader<crate::creatures::def::ReloadCreatures>) {
    // (Polled without marking it changed: the bestiary rereads on a change.)
    let changed = book.bypass_change_detection().watch.changed();
    if !changed && asked.read().count() == 0 {
        return;
    }
    asked.clear();
    match MoveBook::read(book.watch.path()) {
        Ok(moves) => {
            book.set(moves);
            info!("moves.ron reloaded");
        }
        Err(e) => error!("moves.ron not reloaded: {e}"),
    }
}

/// A creature's moves (its file's `moves`, in the order it tries them),
/// when each may come again, and the one under way.
/// A creature began a move (by id): what onlookers see (`observe`).
#[derive(Message, Clone, Debug)]
pub struct Began {
    pub who: Entity,
    pub id: String,
}

#[derive(Component, Debug)]
pub struct Moves {
    ids: Vec<String>,
    ready: Vec<f32>,
    doing: Option<Doing>,
}

#[derive(Debug)]
struct Doing {
    /// Which of its moves, how far in (s), and the phase it's in.
    which: usize,
    t: f32,
    phase: usize,
    target: Entity,
    /// Toward the target as it started.
    dir: Vec2,
    struck: bool,
    /// What its grab holds, and where (cells toward the target).
    held: Option<(Entity, f32)>,
    /// Where a beam points (it swings after the target).
    aim: Vec2,
    /// When the phase began (s into the move).
    start: f32,
    /// The pose the phase eases from.
    from: Pose,
}

impl Moves {
    pub fn new(ids: Vec<String>) -> Self {
        let ready = vec![0.0; ids.len()];
        Moves { ids, ready, doing: None }
    }

    /// Its moves (ids), in the order it tries them.
    pub fn ids(&self) -> &[String] {
        &self.ids
    }

    /// Start move `which` at `target` now, whatever its range and wait (the
    /// bestiary's stage).
    pub fn force(&mut self, which: usize, target: Entity, dir: Vec2) {
        if which < self.ids.len() {
            let dir = dir.normalize_or(Vec2::X);
            self.doing = Some(Doing { which, t: 0.0, phase: usize::MAX, start: 0.0, target, dir, struck: false, held: None, aim: dir, from: Pose::default() });
        }
    }

    /// What its move can hit, from where it stands (`pos`): each a box's
    /// middle, its half size, what it is (`strike`, `grab`, `slam`) and
    /// whether it's live now (else coming: this phase's before it's live,
    /// or the next phase's while this one only winds up).
    pub fn shapes(&self, book: &MoveBook, pos: Vec2, half: Vec2) -> Vec<(Vec2, f32, &'static str, bool)> {
        let Some(d) = &self.doing else { return Vec::new() };
        let Some(m) = book.get(&self.ids[d.which]) else { return Vec::new() };
        let Some(p) = m.phases.get(d.phase) else { return Vec::new() };
        let f = if p.secs > 0.0 { ((d.t - d.start) / p.secs).clamp(0.0, 1.0) } else { 1.0 };
        let of = |acts: &[Act], now: bool| -> Vec<(Vec2, f32, &'static str, bool)> {
            acts.iter()
                .filter_map(|a| match a {
                    Act::Strike(s) => Some((pos + d.dir * s.at, s.reach, "strike", now && f >= s.from && !d.struck)),
                    Act::Grab(g) => Some((pos + d.dir * g.at, g.reach, "grab", now && f >= g.from && d.held.is_none())),
                    Act::Slam(s) => Some((pos - Vec2::Y * half.y, s.radius, "slam", now && !d.struck)),
                    _ => None,
                })
                .collect()
        };
        let here = of(&p.acts, true);
        if !here.is_empty() {
            return here;
        }
        m.phases.get(d.phase + 1).map_or_else(Vec::new, |next| of(&next.acts, false))
    }

    /// Busy with a move (its weapon waits: `hunter`).
    pub fn busy(&self) -> bool {
        self.doing.is_some()
    }

    /// The move under way, if any: its id, how far in (seconds) and the
    /// phase's name.
    pub fn doing<'a>(&'a self, book: &'a MoveBook) -> Option<(&'a str, f32, &'a str)> {
        let d = self.doing.as_ref()?;
        let m = book.get(&self.ids[d.which])?;
        Some((m.id.as_str(), d.t, m.phases.get(d.phase).map_or("", |p| p.name.as_str())))
    }
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Nothing solid on the line from `a` to `b` (a look every two cells).
pub(crate) fn clear(sim: &SimWorld, a: Vec2, b: Vec2) -> bool {
    let n = (a.distance(b) / 2.0).ceil() as i32;
    (1..n).all(|i| {
        let p = a.lerp(b, i as f32 / n as f32);
        !sim.world.is_solid(platypus_sim::CellPos::from_world(p.x, p.y))
    })
}

type Mover<'a> = (Entity, &'a mut Moves, &'a mut Kinematics, &'a mut Controls, Option<&'a mut Rear>, Option<&'a mut Animator>);
type Prey<'a> = (Entity, &'a Kinematics, &'a Team);

/// What a move goes for: where it is, how it's moving, its half size.
#[derive(Clone, Copy)]
struct Quarry {
    e: Entity,
    pos: Vec2,
    vel: Vec2,
    half: Vec2,
}

/// Start a move when one's in reach and ready (before the brain, so it
/// knows: its weapon waits, `Moves::busy`); not mid-swing of what it
/// wields.
pub fn start(
    time: Res<Time>,
    sim: Res<SimWorld>,
    book: Res<MoveBook>,
    mut began: MessageWriter<Began>,
    mut movers: Query<(Entity, &mut Moves, &Kinematics, Has<crate::combat::Swing>), Without<crate::creatures::brain::Staged>>,
    prey: Query<Prey, Without<crate::creatures::brain::villager::Hiding>>,
) {
    let now = time.elapsed_secs();
    for (e, mut moves, k, swinging) in &mut movers {
        if moves.doing.is_some() || swinging {
            continue;
        }
        let pos = k.body.pos;
        let holds = k.loco.grounded() || k.loco.clinging().is_some();
        let Some((pe, pk, _)) = prey.iter().filter(|(p, _, t)| *p != e && t.hunted()).min_by(|x, y| x.1.body.pos.distance(pos).total_cmp(&y.1.body.pos.distance(pos))) else { continue };
        let to = pk.body.pos - pos;
        let dist = to.length();
        let start = (0..moves.ids.len()).find(|&i| {
            let Some(m) = book.get(&moves.ids[i]) else { return false };
            now >= moves.ready[i] && (m.when.range.0..m.when.range.1).contains(&dist) && (!m.when.footing || holds) && (!m.when.line || clear(&sim, pos, pk.body.pos))
        });
        if let Some(which) = start {
            // (`phase` past the end: the first phase's start is still to come.)
            let dir = to.normalize_or(Vec2::X);
            moves.doing = Some(Doing { which, t: 0.0, phase: usize::MAX, start: 0.0, target: pe, dir, struck: false, held: None, aim: dir, from: Pose::default() });
            began.write(Began { who: e, id: moves.ids[which].clone() });
        }
    }
}

/// Carry on the move under way (after the brain: a creature mid-move
/// stands).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn run(
    mut commands: Commands,
    time: Res<Time>,
    book: Res<MoveBook>,
    spells: Res<crate::magic::Spellbook>,
    coatings: Res<Coatings>,
    mut hits: MessageWriter<Hit>,
    mut casts: MessageWriter<crate::magic::CastRequest>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut q: ParamSet<(Query<Mover>, Query<Prey, Without<crate::creatures::brain::villager::Hiding>>)>,
) {
    let now = time.elapsed_secs();
    // (What's hunted, as it stands before any of this tick's moves.)
    let prey: Vec<Quarry> = q.p1().iter().filter(|(_, _, t)| t.hunted()).map(|(e, k, _)| Quarry { e, pos: k.body.pos, vel: k.body.vel, half: k.body.half }).collect();
    let mut movers = q.p0();
    for (e, mut moves, mut k, mut c, rear, mut anim) in &mut movers {
        let Some(mut rear) = rear else {
            commands.entity(e).insert(Rear::default());
            continue;
        };
        if moves.doing.is_none() {
            continue;
        }
        let pos = k.body.pos;
        let id = moves.doing.as_ref().map(|d| moves.ids[d.which].clone()).expect("a move under way");
        let Some(m) = book.get(&id).cloned() else {
            moves.doing = None;
            continue;
        };
        // It stands while it moves.
        c.0.move_x = 0.0;
        c.0.move_y = 0.0;
        c.0.jump = false;
        let d = moves.doing.as_mut().expect("a move under way");
        let target = prey.iter().find(|p| p.e == d.target).copied();
        if let Some(pk) = &target {
            c.0.aim = pk.pos;
        }
        let dir = d.dir;
        let at = |along: f32| pos + dir * along;
        // Holding something: it's where it's held, unless it's gone, or
        // the grabber's been struck senseless (it lets go).
        let stunned = k.loco.state == platypus_physics::MoveState::Stunned;
        if let Some((h, along)) = d.held {
            if stunned || target.is_none() {
                commands.entity(h).try_remove::<Held>();
                d.held = None;
            } else {
                commands.entity(h).try_insert(Held { by: e, at: at(along) });
            }
        }
        // Into each phase whose time has come (one of no length goes by at
        // once, its pose taken and what it does done).
        let mut done = false;
        loop {
            let next = if d.phase == usize::MAX {
                0
            } else if d.t >= d.start + m.phases[d.phase].secs {
                d.start += m.phases[d.phase].secs;
                // (A grab that caught nothing: straight to the last phase.)
                let missed = d.held.is_none() && m.phases[d.phase].acts.iter().any(|a| matches!(a, Act::Grab(_)));
                if missed { (d.phase + 1).max(m.phases.len() - 1) } else { d.phase + 1 }
            } else {
                break;
            };
            let Some(p) = m.phases.get(next) else {
                done = true;
                break;
            };
            d.phase = next;
            d.struck = false;
            if let (Some(clip), Some(a)) = (&p.clip, anim.as_deref_mut()) {
                a.play(clip);
            }
            d.from = Pose { lift: rear.lift, back: rear.back, curl: rear.curl, coil: rear.coil, reach: rear.reach, paw: rear.paw };
            if let Some(pose) = p.pose
                && (p.ease == Ease::Snap || p.secs == 0.0)
            {
                set(&mut rear, pose);
            }
            let mut vel = k.body.vel;
            begin(p, e, &k, dir, target.as_ref(), &spells, &mut vel, at, &mut casts, &mut sounds, &mut commands);
            k.body.vel = vel;
            for act in &p.acts {
                if let Act::Throw { speed, up, stun } = act
                    && let Some((h, _)) = d.held.take()
                {
                    let fling = dir * *speed + Vec2::Y * *up;
                    let stun = *stun;
                    commands.entity(h).try_remove::<Held>().queue_silenced(move |mut ew: EntityWorldMut| {
                        if let Some(mut hk) = ew.get_mut::<Kinematics>() {
                            let hk = &mut *hk;
                            hk.loco.knock(&mut hk.body, fling, stun);
                        }
                    });
                }
            }
        }
        if stunned && d.held.is_none() && m.phases.get(d.phase).is_some_and(|p| p.acts.iter().any(|a| matches!(a, Act::Grab(_) | Act::Beam { .. }))) {
            // (Struck senseless mid-grab or mid-beam: the move's off.)
            done = true;
        }
        if done {
            if let Some((h, _)) = d.held.take() {
                commands.entity(h).try_remove::<Held>();
            }
            if let Some(a) = anim.as_deref_mut()
                && m.phases.iter().any(|p| p.clip.is_some())
            {
                a.force = None;
            }
            let which = d.which;
            moves.ready[which] = now + m.every;
            for r in &mut moves.ready {
                *r = r.max(now + BREATH);
            }
            moves.doing = None;
            *rear = Rear::default();
            continue;
        }
        let (i, start) = (d.phase, d.start);
        let p = &m.phases[i];
        let f = if p.secs > 0.0 { ((d.t - start) / p.secs).clamp(0.0, 1.0) } else { 1.0 };
        if let Some(to) = p.pose {
            let w = match p.ease {
                Ease::Snap => 1.0,
                Ease::Linear => f,
                Ease::Smooth => smooth(f),
            };
            rear.lift = d.from.lift + (to.lift - d.from.lift) * w;
            rear.back = d.from.back + (to.back - d.from.back) * w;
            rear.curl = d.from.curl + (to.curl - d.from.curl) * w + p.tremble * (d.t * 40.0).sin();
            rear.coil = d.from.coil + (to.coil - d.from.coil) * w + p.tremble * 4.0 * (d.t * 40.0).sin();
            rear.reach = d.from.reach + (to.reach - d.from.reach) * w;
            rear.paw = d.from.paw + (to.paw - d.from.paw) * w + p.tremble * 3.0 * (d.t * 40.0).sin();
        }
        // A strike, live.
        for act in &p.acts {
            match act {
                Act::Strike(s) if !d.struck && f >= s.from => {
                    if let Some(pk) = &target {
                        let head = at(s.at);
                        if ((pk.pos - head).abs() - pk.half).max_element() <= s.reach {
                            d.struck = true;
                            hits.write(Hit { target: d.target, damage: s.damage, harm: s.harm, knock: (dir + Vec2::Y * s.up).normalize() * s.knock, stun: s.stun, at: head, dir, weight: s.damage / 12.0, crit: false });
                            if let Some(coat) = &s.coat
                                && coatings.by_name.contains_key(coat)
                            {
                                crate::creatures::body::elements::stain(&mut commands, d.target, coat, 1.0, &coatings);
                            }
                        }
                    }
                }
                Act::Grab(g) if d.held.is_none() && f >= g.from => {
                    if let Some(pk) = &target {
                        let hand = at(g.at);
                        if ((pk.pos - hand).abs() - pk.half).max_element() <= g.reach {
                            d.held = Some((pk.e, g.at));
                            commands.entity(pk.e).try_insert(Held { by: e, at: hand });
                        }
                    }
                }
                Act::Beam { spell, at: along, turn } => {
                    if let (Some(pk), Some(i)) = (&target, spells.spells.iter().position(|s| &s.id == spell)) {
                        let from = at(*along);
                        // (Swinging after them, no faster than it can turn.)
                        let want = (pk.pos - from).normalize_or(d.aim);
                        let swing = d.aim.angle_to(want).clamp(-turn * DT, turn * DT);
                        d.aim = Vec2::from_angle(swing).rotate(d.aim);
                        c.0.aim = from + d.aim * 60.0;
                        casts.write(crate::magic::CastRequest { caster: e, spell: i, from, toward: from + d.aim * 60.0, alt: false });
                    }
                }
                Act::Slam(s) if !d.struck && f >= 0.0 => {
                    d.struck = true;
                    let feet = pos - Vec2::Y * k.body.half.y;
                    for pk in prey.iter().filter(|p| p.e != e) {
                        if ((pk.pos - feet).abs() - pk.half).max_element() <= s.radius {
                            let away = Vec2::new((pk.pos.x - pos.x).signum(), 0.6).normalize();
                            hits.write(Hit { target: pk.e, damage: s.damage, harm: s.harm, knock: away * s.knock, stun: s.stun, at: feet, dir: away, weight: s.damage / 12.0, crit: false });
                        }
                    }
                }
                _ => {}
            }
        }
        d.t += DT;
    }
}

/// What's held stays where it's held: no fall, no control (stunned a
/// moment at a time while it lasts). Let go if the grabber's gone.
pub(crate) fn pin(mut commands: Commands, mut q: Query<(Entity, &Held, &mut Kinematics)>, grabbers: Query<(), With<Moves>>) {
    for (e, h, mut k) in &mut q {
        if grabbers.get(h.by).is_err() {
            commands.entity(e).remove::<Held>();
            continue;
        }
        let k = &mut *k;
        k.body.pos = h.at;
        k.loco.knock(&mut k.body, Vec2::ZERO, 2.0 * DT);
    }
}

fn set(rear: &mut Rear, pose: Pose) {
    rear.lift = pose.lift;
    rear.back = pose.back;
    rear.curl = pose.curl;
    rear.coil = pose.coil;
    rear.reach = pose.reach;
    rear.paw = pose.paw;
}

/// What a phase does as it starts.
#[allow(clippy::too_many_arguments)]
fn begin(
    p: &Phase,
    e: Entity,
    k: &Kinematics,
    dir: Vec2,
    target: Option<&Quarry>,
    spells: &crate::magic::Spellbook,
    vel: &mut Vec2,
    at: impl Fn(f32) -> Vec2,
    casts: &mut MessageWriter<crate::magic::CastRequest>,
    sounds: &mut MessageWriter<crate::sound::PlaySound>,
    commands: &mut Commands,
) {
    for act in &p.acts {
        match act {
            Act::Lunge { speed, up } => *vel = dir * *speed + Vec2::Y * *up,
            Act::Cast { spell, at: along, up } => {
                let (Some(pk), Some(i)) = (target, spells.spells.iter().position(|s| &s.id == spell)) else { continue };
                let from = at(*along) + Vec2::Y * *up;
                // Where they'll be when it gets there, and above that by as
                // far as it falls on the way.
                let toward = match spells.flight(i) {
                    Some((speed, fall)) => {
                        let flight = from.distance(pk.pos) / speed.max(1.0);
                        pk.pos + pk.vel * flight + Vec2::Y * 0.5 * fall * flight * flight
                    }
                    None => pk.pos,
                };
                casts.write(crate::magic::CastRequest { caster: e, spell: i, from, toward, alt: false });
            }
            Act::Summon { kind, count, spread } => {
                for n in 0..*count {
                    let dx = if *count > 1 { (n as f32 / (*count - 1) as f32 - 0.5) * 2.0 * spread } else { 0.0 };
                    crate::creatures::def::spawn_creature(commands, kind, k.body.pos + Vec2::new(dx, 1.5 - k.body.half.y), |_| {});
                }
            }
            Act::Sound(name) => {
                sounds.write(crate::sound::PlaySound::at(name.clone(), k.body.pos));
            }
            Act::Strike(_) | Act::Slam(_) | Act::Grab(_) | Act::Throw { .. } | Act::Beam { .. } => {}
        }
    }
}
