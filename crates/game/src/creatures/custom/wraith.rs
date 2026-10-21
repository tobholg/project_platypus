//! Chain wraiths: shades that drift through rock (`Phasing`), chains
//! hanging from iron cuffs on their wrists (each a chain of links, swung
//! by its motion; drawn here, on a canvas of their own). A wraith keeps a
//! chain's length off whoever it hunts; spins a chain overhead (a rising
//! rattle: the tell) and lashes it out at them; hooked, they're dragged in
//! and raked with cold claws, unless they dash or strike at the chain to
//! break free. Too near, it whirls both chains round itself. It can't go
//! into water or over it; inside rock it can't be hurt (and can't attack);
//! out of the deep dark, it fades at dawn. Dead, it leaves its chains.

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind};
use serde::Deserialize;

use super::{CustomCreature, CustomSet, frame_anchor};
use crate::creatures::body::animation::Animator;
use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprite, CanvasSprites};
use crate::creatures::{Controls, Died, Harm, Health, Kinematics, Phasing, Team};
use crate::world::{ChunkLoader, SimWorld};

/// Its settings (`params`) and what it's doing.
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Wraith {
    /// How far its chain lashes (cells), how far off it keeps, how near
    /// it whirls.
    pub reach: f32,
    pub keep: f32,
    pub sweep: f32,
    /// Its rake: what it does (damage), how cold it leaves you (0..1 of a
    /// chill) and how long (a frost wraith: colder, longer).
    pub rake: f32,
    pub cold: f32,
    pub cold_secs: f32,
    /// A cold round it this far (cells; 0: none): it cools what's near
    /// (water freezes where it drifts over), and chills what comes near.
    pub aura: f32,
    #[serde(skip)]
    aura_tick: f32,
    #[serde(skip)]
    doing: Doing,
    #[serde(skip)]
    chains: [Chain; 2],
    #[serde(skip)]
    whip_ready: f32,
    #[serde(skip)]
    sweep_ready: f32,
    /// How much of it shows (drawn; inside rock it's faint).
    #[serde(skip)]
    alpha: f32,
    /// In the deep dark (it stays at dawn), decided where it first was.
    #[serde(skip)]
    deep: Option<bool>,
}

impl Wraith {
    /// What it's doing, in a word (for a scenario's log).
    pub fn doing(&self) -> &'static str {
        match self.doing {
            Doing::Drift => "drift",
            Doing::Spin(_) => "spin",
            Doing::Lash(..) => "lash",
            Doing::Pull(..) => "pull",
            Doing::Rake(..) => "rake",
            Doing::Whirl(_) => "whirl",
            Doing::Sweep(..) => "sweep",
            Doing::Fade(_) => "fade",
        }
    }
}

impl Default for Wraith {
    fn default() -> Self {
        Wraith { reach: 120.0, keep: 85.0, sweep: 40.0, rake: RAKE_HIT, cold: 0.6, cold_secs: 2.0, aura: 0.0, aura_tick: 0.0, doing: Doing::Drift, chains: Default::default(), whip_ready: 0.0, sweep_ready: 0.0, alpha: 0.0, deep: None }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Doing {
    #[default]
    Drift,
    /// The front chain spun overhead (the tell).
    Spin(f32),
    /// Lashed out that way.
    Lash(f32, Vec2),
    /// Hooked them: dragging them in.
    Pull(f32, Entity),
    /// Up to it: raked (whether it's struck yet).
    Rake(f32, Entity, bool),
    /// Both chains out (the tell), then whirled round (whom they've struck).
    Whirl(f32),
    Sweep(f32, bool),
    /// Dawn: fading away.
    Fade(f32),
}

/// A chain: its links, where they were last tick (it swings).
#[derive(Clone, Debug, Default)]
struct Chain {
    pts: Vec<Vec2>,
    prev: Vec<Vec2>,
}

impl CustomCreature for Wraith {
    const NAME: &'static str = "wraith";

    fn build(app: &mut App) {
        app.init_resource::<ChainCanvas>()
            .add_observer(on_spawn)
            .add_systems(FixedUpdate, think.in_set(CustomSet::Think))
            .add_systems(FixedUpdate, on_death.in_set(crate::world::TickSet::Bodies))
            .add_systems(Update, (fade_in, draw).after(crate::creatures::body::elements::tint));
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
/// Links in a chain, cells apart; how hard a hanging chain falls.
const LINKS: usize = 14;
const LINK: f32 = 3.0;
const CHAIN_FALL: f32 = 260.0;
/// Hunting what's this near (cells); drifting this fast (half that in
/// rock), hovering this high over them.
const AGGRO: f32 = 420.0;
const DRIFT: f32 = 70.0;
const HOVER: f32 = 20.0;
/// The whip: spun this long (s), lashed out in this long; a hook drags in
/// this fast, for at most this long; the rake; then it waits this long.
const SPIN: f32 = 0.7;
const LASH: f32 = 0.22;
const PULL: (f32, f32) = (340.0, 0.9);
const RAKE: f32 = 0.45;
const WHIP_EVERY: f32 = 3.2;
/// The whirl: its tell, its sweep (turns, the chains' reach), then a wait.
const WHIRL: f32 = 0.45;
const SWEEP: (f32, f32, f32) = (0.9, 2.0, 40.0);
const SWEEP_EVERY: f32 = 4.0;
/// Its blows: the rake (frost), the sweep.
const RAKE_HIT: f32 = 18.0;
/// Its cold (`aura`), every so often (s): the cells round it cooled this
/// much (°C), what's near chilled this much, this long.
const AURA_EVERY: f32 = 0.25;
const AURA_COLD: i16 = -45;
const AURA_CHILL: (f32, f32) = (0.35, 0.6);

/// Frost motes drifting off a frost wraith.
static RIME: std::sync::LazyLock<crate::magic::runes::Emitter> = std::sync::LazyLock::new(|| crate::magic::runes::Emitter {
    count: 5.0,
    life: (0.4, 1.0),
    colors: vec![(235, 245, 255), (190, 225, 255), (150, 200, 240)],
    speed: 30.0,
    spread: 3.1,
    gravity: 30.0,
    drag: 2.0,
    size: 1.0,
    jitter: 10.0,
    glow: true,
});
/// Frost in its palms (its breath, gathered and breathed).
static PALM: std::sync::LazyLock<crate::magic::runes::Emitter> = std::sync::LazyLock::new(|| crate::magic::runes::Emitter {
    count: 1.0,
    life: (0.2, 0.45),
    colors: vec![(255, 255, 255), (205, 238, 255), (140, 200, 250)],
    speed: 18.0,
    spread: 0.9,
    gravity: -20.0,
    drag: 3.0,
    size: 1.0,
    jitter: 0.0,
    glow: true,
});
/// The clips it plays itself (a move's it leaves be).
const OWN_CLIPS: [&str; 4] = ["raise", "lash", "claw", "whirl"];
const SWEEP_HIT: f32 = 12.0;
/// Fading at dawn this long (s); deeper than this (cells under the
/// surface) it's in the deep dark and doesn't.
const FADE: f32 = 2.0;
const DEEP: i32 = 200;
/// How much of it shows: in the open, attacking, in rock.
const SHOWS: (f32, f32, f32) = (0.72, 0.92, 0.3);

/// Spawned: it phases.
fn on_spawn(add: On<Add, Wraith>, mut commands: Commands) {
    commands.entity(add.entity).insert(Phasing);
}

/// Where its wrists are (front, back), as its picture has them
/// (`wrist_f`, `wrist_b` anchors: the gaunt design), else as the round
/// design's were, by what it's doing (`wrists`).
fn hands_at(anim: &Animator, k: &Kinematics, doing: Doing) -> [Vec2; 2] {
    match (frame_anchor(anim, k, "wrist_f", ""), frame_anchor(anim, k, "wrist_b", "")) {
        (Some(f), Some(b)) => [f, b],
        _ => wrists(k.body.pos, if k.loco.facing < 0.0 { -1.0 } else { 1.0 }, doing),
    }
}

/// Where its wrists are (front, back), by what it's doing.
fn wrists(pos: Vec2, facing: f32, doing: Doing) -> [Vec2; 2] {
    let (front, back) = match doing {
        Doing::Spin(_) => (Vec2::new(9.0, 15.0), Vec2::new(-9.0, 0.0)),
        Doing::Lash(..) | Doing::Pull(..) | Doing::Rake(..) => (Vec2::new(10.0, 4.0), Vec2::new(-9.0, 0.0)),
        Doing::Whirl(_) | Doing::Sweep(..) => (Vec2::new(11.0, 6.0), Vec2::new(-11.0, 6.0)),
        _ => (Vec2::new(9.0, 0.0), Vec2::new(-9.0, 0.0)),
    };
    [pos + Vec2::new(front.x * facing, front.y), pos + Vec2::new(back.x * facing, back.y)]
}

fn solid_at(sim: &SimWorld, p: Vec2) -> bool {
    sim.world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| !c.is_air() && matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder))
}

/// Water in the box round `pos`, or under it (the first thing below that
/// isn't air): a wraith can't go there.
fn watery(sim: &SimWorld, pos: Vec2, half: Vec2) -> bool {
    let Some(water) = sim.world.materials().id("water") else { return false };
    let wet = |p: Vec2| sim.world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| c.material == water);
    let (lo, hi) = (pos - half, pos + half);
    let mut y = lo.y;
    while y <= hi.y {
        let mut x = lo.x;
        while x <= hi.x {
            if wet(Vec2::new(x, y)) {
                return true;
            }
            x += 3.0;
        }
        y += 3.0;
    }
    // (Over it: what's under its middle and edges.)
    [lo.x, pos.x, hi.x].into_iter().any(|x| {
        (0..150).map(|d| Vec2::new(x, lo.y - d as f32)).find(|&p| sim.world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| !c.is_air())).is_some_and(wet)
    })
}

/// Which of `pts` is in the box round `pos` (a little more than `half`).
fn touches(pts: &[Vec2], pos: Vec2, half: Vec2) -> bool {
    pts.iter().any(|p| ((*p - pos).abs() - half).max_element() <= 1.5)
}

/// A hanging chain swings: links fall and trail, held at the wrist.
fn swing(c: &mut Chain, wrist: Vec2) {
    if c.pts.len() != LINKS {
        c.pts = (0..LINKS).map(|i| wrist - Vec2::Y * LINK * i as f32).collect();
        c.prev = c.pts.clone();
    }
    for i in 1..LINKS {
        let v = (c.pts[i] - c.prev[i]) * 0.97;
        c.prev[i] = c.pts[i];
        c.pts[i] += v - Vec2::Y * CHAIN_FALL * DT * DT;
    }
    for _ in 0..6 {
        c.pts[0] = wrist;
        for i in 1..LINKS {
            let d = c.pts[i] - c.pts[i - 1];
            let len = d.length().max(1e-4);
            let fix = d * (1.0 - LINK / len);
            if i == 1 {
                c.pts[i] -= fix;
            } else {
                c.pts[i - 1] += fix * 0.5;
                c.pts[i] -= fix * 0.5;
            }
        }
    }
    c.pts[0] = wrist;
}

/// A chain laid out as `at` says (link i of `LINKS`), its swing kept.
fn lay(c: &mut Chain, at: impl Fn(usize) -> Vec2) {
    if c.pts.len() != LINKS {
        c.pts = (0..LINKS).map(&at).collect();
        c.prev = c.pts.clone();
        return;
    }
    for i in 0..LINKS {
        c.prev[i] = c.pts[i];
        c.pts[i] = at(i);
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn think(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    day: Res<crate::light::Daylight>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut wraiths: Query<(Entity, &mut Wraith, &mut crate::creatures::body::animation::Animator, &Health, Option<&crate::creatures::moves::Moves>)>,
    mut bodies: Query<(Entity, &mut Kinematics, Option<&Team>, Option<&Controls>, Has<Wraith>)>,
    natures: Query<&Health, Without<Wraith>>,
    mut swings: MessageReader<crate::combat::MeleeRequest>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut flares: MessageWriter<crate::fx::Flare>,
) {
    let struck_at: Vec<Entity> = swings.read().map(|s| s.attacker).collect();
    let night = !(0.22..0.8).contains(&day.time);
    for (e, mut w, mut anim, health, moves) in &mut wraiths {
        let Ok((_, k, ..)) = bodies.get(e) else { continue };
        // (Its own, kept: what it holds is moved below.)
        let me = k.clone();
        let (pos, half, facing) = (me.body.pos, me.body.half, me.loco.facing);
        let deep = *w.deep.get_or_insert_with(|| sim.generator.surface_hint(pos.x as i32).is_some_and(|s| s - pos.y as i32 > DEEP));
        let in_rock = solid_at(&sim, pos);
        // Whom it hunts: the nearest of the living near.
        let target = bodies
            .iter()
            .filter(|(o, _, t, _, wraith)| *o != e && !wraith && t.is_some_and(|t| t.hunted()))
            .map(|(o, ok, ..)| (o, ok.body.pos, ok.body.half))
            .filter(|(_, p, _)| p.distance(pos) < AGGRO)
            .min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)));
        // Its cold (a frost wraith's): the cells round it cooled (water
        // freezes), what's near chilled, frost motes drifting off it.
        w.aura_tick -= DT;
        if w.aura > 0.0 && w.aura_tick <= 0.0 && !in_rock {
            w.aura_tick = AURA_EVERY;
            sim.queue(platypus_sim::WorldEdit::Heat { center: platypus_sim::CellPos::from_world(pos.x, pos.y), radius: w.aura as i32, amount: AURA_COLD });
            let near: Vec<Entity> = bodies.iter().filter(|(o, ok, t, _, wraith)| *o != e && !wraith && t.is_some_and(|t| t.hunted()) && ok.body.pos.distance(pos) < w.aura).map(|(o, ..)| o).collect();
            for who in near {
                if let Ok(h) = natures.get(who) {
                    crate::creatures::body::elements::chill(&mut commands, who, &h.nature, None, AURA_CHILL.0, AURA_CHILL.1);
                }
            }
            sparks.emit(&RIME, RIME.count as usize, pos, Vec2::NEG_Y, Vec2::ZERO);
        }
        // Its breath (a frost wraith's: `frost_breath`): frost gathering in
        // its raised palms, then blazing out of them as it breathes, a cold
        // light on what's near.
        let breath = match anim.force.as_deref() {
            Some("gather") => Some(false),
            Some("breathe") => Some(true),
            _ => None,
        };
        if let Some(blazing) = breath {
            let f = if facing < 0.0 { -1.0 } else { 1.0 };
            // (Its palms as its picture has them, the gaunt design's; else
            // where the round design's were.)
            let drawn = frame_anchor(&anim, &me, "palm_f", "").zip(frame_anchor(&anim, &me, "palm_b", "")).map(|(a, b)| [a - pos, b - pos]);
            let palms = drawn.unwrap_or(if blazing { [Vec2::new(13.0 * f, 8.5), Vec2::new(13.0 * f, 5.5)] } else { [Vec2::new(-11.5 * f, 18.5), Vec2::new(11.5 * f, 18.5)] });
            let way = if blazing { Vec2::X * f } else { Vec2::Y };
            for (i, p) in palms.into_iter().enumerate() {
                // (Every other tick or so, each.)
                if platypus_sim::rng::hash(&[e.to_bits(), (day.time * 1.0e7) as u64, i as u64]) & 1 == 0 {
                    sparks.emit(&PALM, 1, pos + p, way, Vec2::ZERO);
                }
            }
            let color = if blazing { [0.75, 1.05, 1.6] } else { [0.3, 0.5, 0.85] };
            flares.write(crate::fx::Flare { at: pos + (palms[0] + palms[1]) / 2.0, color, life: 0.06 });
        }
        // Dawn, out of the deep dark: it fades.
        if !deep && !night && !matches!(w.doing, Doing::Fade(_)) {
            w.doing = Doing::Fade(0.0);
        }
        // In rock it can't be hurt.
        if in_rock {
            commands.entity(e).insert(crate::combat::Invulnerable { left: 0.15, hp: health.hp, dodge: false });
        }
        let mut vel = Vec2::ZERO;
        let mut face = facing;
        if let Some((_, tp, _)) = target {
            face = if tp.x < pos.x { -1.0 } else { 1.0 };
        }
        w.whip_ready -= DT;
        w.sweep_ready -= DT;
        let doing = w.doing;
        let mut next = doing;
        match doing {
            Doing::Drift => {
                // (Its own clips let go; a move's, its breath, held.)
                if anim.force.as_deref().is_some_and(|c| OWN_CLIPS.contains(&c)) {
                    anim.force = None;
                }
                match target {
                    Some((_, tp, _)) => {
                        // A chain's length off, on its side of them, over them.
                        let side = if pos.x < tp.x { -1.0 } else { 1.0 };
                        let want = tp + Vec2::new(side * w.keep, HOVER + (day.time * 900.0).sin() * 6.0);
                        let d = want - pos;
                        vel = d.normalize_or_zero() * DRIFT * (d.length() / 20.0).min(1.0);
                        let dist = tp.distance(pos);
                        // (Not mid-move, its breath: its arms are busy.)
                        if moves.is_some_and(|m| m.busy()) {
                        } else if !in_rock && dist <= w.sweep && w.sweep_ready <= 0.0 {
                            next = Doing::Whirl(0.0);
                            sounds.write(crate::sound::PlaySound::at("clank", pos).pitch(0.55));
                        } else if !in_rock && dist <= w.reach && w.whip_ready <= 0.0 {
                            next = Doing::Spin(0.0);
                        }
                    }
                    None => vel = Vec2::new((day.time * 400.0).sin() * 20.0, (day.time * 700.0).cos() * 8.0),
                }
            }
            Doing::Spin(t) => {
                anim.play("raise");
                // (A rising rattle.)
                let ticks = (t / 0.09) as u32;
                if ((t + DT) / 0.09) as u32 != ticks {
                    sounds.write(crate::sound::PlaySound::at("clank", pos).pitch(0.8 + t).volume(0.5));
                }
                next = if t + DT >= SPIN {
                    let aim = target.map_or(Vec2::X * face, |(_, tp, _)| (tp - pos).normalize_or(Vec2::X * face));
                    sounds.write(crate::sound::PlaySound::at("swing_heavy", pos).pitch(0.7));
                    Doing::Lash(0.0, aim)
                } else {
                    Doing::Spin(t + DT)
                };
            }
            Doing::Lash(t, aim) => {
                anim.play("lash");
                next = if t + DT >= LASH { Doing::Drift } else { Doing::Lash(t + DT, aim) };
                // Hooked: whoever it reaches.
                if let Some((who, tp, th)) = target
                    && touches(&w.chains[0].pts, tp, th)
                {
                    // (It hooks; the rake hurts: a blow now would grace them
                    // past it.)
                    sounds.write(crate::sound::PlaySound::at("clank", tp).pitch(0.6).volume(1.0));
                    next = Doing::Pull(0.0, who);
                } else if t + DT >= LASH {
                    w.whip_ready = WHIP_EVERY;
                }
            }
            Doing::Pull(t, who) => {
                anim.play("lash");
                let wrist = hands_at(&anim, &me, doing)[0];
                let free = bodies.get(who).ok().is_none_or(|(_, _, _, c, _)| c.is_some_and(|c| c.0.dash)) || struck_at.contains(&who);
                match bodies.get_mut(who) {
                    Ok((_, mut tk, ..)) if !free && t < PULL.1 => {
                        let to = wrist - tk.body.pos;
                        if to.length() < half.x + tk.body.half.x + 6.0 {
                            next = Doing::Rake(0.0, who, false);
                        } else {
                            let tk = &mut *tk;
                            tk.loco.knock(&mut tk.body, to.normalize_or_zero() * PULL.0, 0.12);
                            next = Doing::Pull(t + DT, who);
                        }
                    }
                    _ => {
                        // (Broken free, or it let go.)
                        if free {
                            sounds.write(crate::sound::PlaySound::at("clang", wrist).pitch(0.8));
                        }
                        w.whip_ready = WHIP_EVERY;
                        next = Doing::Drift;
                    }
                }
            }
            Doing::Rake(t, who, struck) => {
                anim.play("claw");
                let mut now = struck;
                if !struck && t >= RAKE * 0.35 {
                    now = true;
                    if let Ok((_, tk, ..)) = bodies.get(who) {
                        let away = Vec2::new((tk.body.pos.x - pos.x).signum(), 0.5).normalize();
                        hits.write(crate::combat::Hit { target: who, damage: w.rake, harm: Harm::Frost, knock: away * 240.0, stun: 0.25, at: tk.body.pos, dir: away, weight: 1.2, crit: false });
                        if let Ok(h) = natures.get(who) {
                            crate::creatures::body::elements::chill(&mut commands, who, &h.nature, None, w.cold, w.cold_secs);
                        }
                        sounds.write(crate::sound::PlaySound::at("hit", tk.body.pos).pitch(0.7));
                    }
                }
                next = if t + DT >= RAKE {
                    w.whip_ready = WHIP_EVERY;
                    Doing::Drift
                } else {
                    Doing::Rake(t + DT, who, now)
                };
            }
            Doing::Whirl(t) => {
                anim.play("whirl");
                next = if t + DT >= WHIRL {
                    sounds.write(crate::sound::PlaySound::at("swing_heavy", pos).pitch(0.6));
                    Doing::Sweep(0.0, false)
                } else {
                    Doing::Whirl(t + DT)
                };
            }
            Doing::Sweep(t, struck) => {
                anim.play("whirl");
                let mut now = struck;
                if !struck
                    && let Some((who, tp, th)) = target
                    && (touches(&w.chains[0].pts, tp, th) || touches(&w.chains[1].pts, tp, th))
                {
                    now = true;
                    let away = Vec2::new((tp.x - pos.x).signum(), 0.6).normalize();
                    hits.write(crate::combat::Hit { target: who, damage: SWEEP_HIT, harm: Harm::Blunt, knock: away * 260.0, stun: 0.2, at: tp, dir: away, weight: 1.0, crit: false });
                    sounds.write(crate::sound::PlaySound::at("clank", tp).pitch(0.7).volume(1.0));
                }
                next = if t + DT >= SWEEP.0 {
                    w.sweep_ready = SWEEP_EVERY;
                    Doing::Drift
                } else {
                    Doing::Sweep(t + DT, now)
                };
            }
            Doing::Fade(t) => {
                anim.force = None;
                vel = Vec2::Y * 12.0;
                if t >= FADE {
                    commands.entity(e).despawn();
                    continue;
                }
                next = Doing::Fade(t + DT);
            }
        }
        // (Its own attacks hold its moves off: one thing at a time.)
        if matches!(next, Doing::Drift) != matches!(doing, Doing::Drift) {
            if matches!(next, Doing::Drift) {
                commands.entity(e).try_remove::<crate::creatures::moves::Occupied>();
            } else {
                commands.entity(e).try_insert(crate::creatures::moves::Occupied);
            }
        }
        w.doing = next;
        // In rock, slowed; never into water, nor over it.
        if in_rock {
            vel *= 0.5;
        }
        let shows = if in_rock {
            SHOWS.2
        } else if matches!(next, Doing::Drift) {
            SHOWS.0
        } else {
            SHOWS.1
        };
        w.alpha = match next {
            Doing::Fade(t) => SHOWS.0 * (1.0 - t / FADE).max(0.0),
            _ => w.alpha + (shows - w.alpha) * (DT * 6.0).min(1.0),
        };
        let wet_now = watery(&sim, pos, half);
        if !wet_now {
            if watery(&sim, pos + Vec2::new(vel.x * DT, 0.0), half) {
                vel.x = 0.0;
            }
            if watery(&sim, pos + Vec2::new(0.0, vel.y * DT), half) {
                vel.y = 0.0;
            }
        }
        // Its chains: hanging, spun, lashed, held, whirled.
        let [front, back] = hands_at(&anim, &me, next);
        match next {
            Doing::Spin(t) => {
                let a = t * (7.0 + 16.0 * t / SPIN) + std::f32::consts::FRAC_PI_2;
                lay(&mut w.chains[0], |i| front + Vec2::from_angle(a - i as f32 * 0.05) * (i as f32 / (LINKS - 1) as f32) * 30.0);
                swing(&mut w.chains[1], back);
            }
            Doing::Lash(t, aim) => {
                let f = (t / LASH).min(1.0);
                let reach = w.reach * (1.0 - (1.0 - f) * (1.0 - f));
                let side = aim.perp();
                lay(&mut w.chains[0], |i| {
                    let u = i as f32 / (LINKS - 1) as f32;
                    front + aim * reach * u + side * (u * 9.0 - t * 40.0).sin() * 3.0 * (1.0 - f)
                });
                swing(&mut w.chains[1], back);
            }
            Doing::Pull(_, who) | Doing::Rake(_, who, _) => {
                let to = bodies.get(who).map_or(front, |(_, tk, ..)| tk.body.pos);
                lay(&mut w.chains[0], |i| front.lerp(to, i as f32 / (LINKS - 1) as f32));
                swing(&mut w.chains[1], back);
            }
            Doing::Whirl(t) => {
                let out = (t / WHIRL).min(1.0) * SWEEP.2;
                lay(&mut w.chains[0], |i| front + Vec2::new(face, -0.6).normalize() * out * i as f32 / (LINKS - 1) as f32);
                lay(&mut w.chains[1], |i| back + Vec2::new(-face, -0.6).normalize() * out * i as f32 / (LINKS - 1) as f32);
            }
            Doing::Sweep(t, _) => {
                let a = t / SWEEP.0 * SWEEP.1 * std::f32::consts::TAU * face;
                for (n, at) in [(0, front), (1, back)] {
                    let a = a + n as f32 * std::f32::consts::PI;
                    lay(&mut w.chains[n], |i| at + Vec2::from_angle(a - i as f32 * 0.06 * face) * SWEEP.2 * i as f32 / (LINKS - 1) as f32);
                }
            }
            _ => {
                swing(&mut w.chains[0], front);
                swing(&mut w.chains[1], back);
            }
        }
        if let Ok((_, mut k, ..)) = bodies.get_mut(e) {
            k.body.vel = vel;
            k.loco.facing = face;
        }
    }
}

/// How much of it shows, on its picture (after the tint, which sets its
/// colour).
fn fade_in(wraiths: Query<(&Wraith, &Children)>, mut sprites: Query<&mut Sprite, Without<CanvasSprite>>) {
    for (w, children) in &wraiths {
        for c in children.iter() {
            if let Ok(mut s) = sprites.get_mut(c) {
                s.color.set_alpha(w.alpha);
            }
        }
    }
}

/// The wraiths' chains, over the bodies.
#[derive(Resource)]
pub struct ChainCanvas(Canvas);

impl Default for ChainCanvas {
    fn default() -> Self {
        ChainCanvas(Canvas::new("Wraith chains", 10.7))
    }
}

/// Each chain, link by link (light and dark by turns), from its cuff.
fn draw(
    mut commands: Commands,
    mut canvas: ResMut<ChainCanvas>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>,
    wraiths: Query<&Wraith>,
    mut sprites: CanvasSprites,
) {
    let Ok((cam, loader)) = camera.single() else { return };
    let any = wraiths.iter().any(|w| w.chains[0].pts.len() == LINKS);
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, any) else { return };
    for w in &wraiths {
        // (The iron's more there than the shade.)
        let a = ((w.alpha * 1.25).clamp(0.0, 1.0) * 255.0) as u8;
        let (light, dark, cuff) = ([186, 194, 214, a], [92, 98, 120, a], [210, 212, 222, a]);
        for c in &w.chains {
            if c.pts.len() != LINKS {
                continue;
            }
            let mut n = 0u32;
            let mut last = IVec2::MAX;
            for pair in c.pts.windows(2) {
                let steps = (pair[0].distance(pair[1]) * 1.5).ceil().max(1.0) as i32;
                for s in 0..=steps {
                    let p = pair[0].lerp(pair[1], s as f32 / steps as f32).floor().as_ivec2();
                    if p == last {
                        continue;
                    }
                    last = p;
                    n += 1;
                    // (Two cells thick: a link lit on top, in shadow under.)
                    let lit = (n / 2).is_multiple_of(2);
                    px.put(p.x, p.y, if lit { light } else { dark });
                    px.put(p.x, p.y - 1, dark);
                }
            }
            let w0 = c.pts[0].floor().as_ivec2();
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1), (-1, 0), (-1, 1)] {
                px.put(w0.x + dx, w0.y + dy, cuff);
            }
        }
    }
}

/// Dead: it comes apart in mist, and leaves its chains.
fn on_death(mut commands: Commands, mut died: MessageReader<Died>, items: Option<Res<crate::hands::items::Items>>, mut sparks: ResMut<crate::vfx::Sparks>, mut sounds: MessageWriter<crate::sound::PlaySound>) {
    for d in died.read() {
        if d.kind != "chain_wraith" {
            continue;
        }
        let mist = crate::magic::runes::Emitter {
            count: 0.0,
            life: (0.5, 1.4),
            colors: vec![(220, 236, 255), (140, 170, 210), (70, 84, 120)],
            speed: 60.0,
            spread: std::f32::consts::PI,
            gravity: -30.0,
            drag: 1.5,
            size: 1.6,
            jitter: 10.0,
            glow: true,
        };
        sparks.emit(&mist, 160, d.body.pos, Vec2::Y, Vec2::ZERO);
        sounds.write(crate::sound::PlaySound::at("clank", d.body.pos).pitch(0.45));
        if let Some(items) = items.as_ref()
            && let Some(id) = items.id("wraith_chain")
        {
            crate::hands::spawn_drop(&mut commands, items, d.body.pos, crate::hands::items::Stack::new(id, 2));
        }
    }
}
