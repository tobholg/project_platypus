//! Watchers: a floating eye the size of you, nerves trailing under it,
//! hanging high in the dark (it flies, held up by its code) and keeping its
//! distance (its brain: `Range`), its iris following you wherever you go.
//!
//! - **Spit**: it squints (the tell) and lobs a glob of blight at you:
//!   glowing purple sludge that poisons, withers what grows, and gives
//!   itself up as a toxic mist.
//! - **Beam**: its iris kindles and a thin line reaches out from it,
//!   following you, slowly, then holding still (the tell, long: get out of
//!   the line, or behind something); then the beam, along that line, as long
//!   as it lasts, burning what it meets, scorching what it ends on.
//! - **Light**: a lamp near it (a torch, a glowstick) and it squints and
//!   backs off, and won't beam; a flash (a blast, a radiant spell, a flare)
//!   or a radiant blow blinds it: its eye shut, it reels, a while, open to
//!   you.
//! - Dead, it bursts: blight.

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind};
use serde::Deserialize;

use super::{CustomCreature, CustomSet, frame_anchor};
use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprites};
use crate::cloth::Links;
use crate::creatures::body::animation::Animator;
use crate::creatures::{Controls, Died, Harm, Kinematics, Team};
use crate::world::{ChunkLoader, SimWorld};

#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Watcher {
    /// Its beams a volley (one after another, a short charge between) and
    /// how far each sweeps across you as it burns (degrees; by turns one
    /// way, then back); what each does, every tenth of a second, to what's
    /// in it. An elder's: three, sweeping.
    pub beams: u32,
    pub sweep: f32,
    pub beam_hit: f32,
    /// Every this many spits, instead a cloud (a spell: `cloud`) lobbed at
    /// you, that lingers (0: none).
    pub cloud_every: u32,
    pub cloud: String,
    /// Its iris, at its pupil (darker out to its rim).
    pub iris: (u8, u8, u8),
    #[serde(skip)]
    doing: Doing,
    /// Drawn this many times its picture's size (its creature file's
    /// `scale`): its iris, its pupil, its nerves with it.
    #[serde(skip)]
    scale: f32,
    /// Beams left in this volley; the way the one now sweeps, and where
    /// it's centred.
    #[serde(skip)]
    left: u32,
    #[serde(skip)]
    turn: f32,
    #[serde(skip)]
    base: Vec2,
    /// Spits since its last cloud.
    #[serde(skip)]
    spits: u32,
    /// Its waits: spit, beam.
    #[serde(skip)]
    ready: [f32; 2],
    /// Its eye's middle (the world), and where its iris looks (from it).
    #[serde(skip)]
    eye: Vec2,
    #[serde(skip)]
    look: Vec2,
    /// How open its lids are (1 wide, 0 shut), for its iris.
    #[serde(skip)]
    lids: f32,
    /// Its beam's way, and where it ends.
    #[serde(skip)]
    aim: Vec2,
    #[serde(skip)]
    end: Vec2,
    /// Blinded for this long more; squinting from a lamp near.
    #[serde(skip)]
    blind: f32,
    #[serde(skip)]
    glare: Option<Vec2>,
    /// Its next blink, and a blink under way.
    #[serde(skip)]
    blink: (f32, f32),
    #[serde(skip)]
    nerves: Vec<Links>,
    #[serde(skip)]
    clock: f32,
    #[serde(skip)]
    started: bool,
}

impl Default for Watcher {
    fn default() -> Self {
        Watcher {
            beams: 1,
            sweep: 0.0,
            beam_hit: BEAM_HIT.0,
            cloud_every: 0,
            cloud: String::new(),
            iris: (210, 150, 40),
            doing: Doing::Hover,
            scale: 1.0,
            left: 0,
            turn: 1.0,
            base: Vec2::NEG_Y,
            spits: 0,
            ready: [0.0; 2],
            eye: Vec2::ZERO,
            look: Vec2::ZERO,
            lids: 0.0,
            aim: Vec2::ZERO,
            end: Vec2::ZERO,
            blind: 0.0,
            glare: None,
            blink: (0.0, 0.0),
            nerves: Vec::new(),
            clock: 0.0,
            started: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Doing {
    /// Hovering, keeping its distance, watching.
    #[default]
    Hover,
    /// Squinting to spit (the tell).
    Spit(f32),
    /// Its line reaching out, following, then still (the beam's tell).
    Charge(f32),
    /// The beam.
    Beam(f32),
}

impl CustomCreature for Watcher {
    const NAME: &'static str = "watcher";

    fn build(app: &mut App) {
        app.init_resource::<WatcherCanvas>()
            .add_systems(FixedUpdate, (dazzle, think).chain().in_set(CustomSet::Think))
            .add_systems(FixedUpdate, burst.in_set(crate::world::TickSet::Bodies))
            .add_systems(Update, draw);
    }
}

impl Watcher {
    /// What it's doing, in a word (a scenario's log).
    pub fn doing(&self) -> &'static str {
        if self.blind > 0.0 {
            return "blind";
        }
        match self.doing {
            Doing::Hover if self.glare.is_some() => "glare",
            Doing::Hover => "hover",
            Doing::Spit(_) => "spit",
            Doing::Charge(_) => "charge",
            Doing::Beam(_) => "beam",
        }
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
const SPIT_I: usize = 0;
const BEAM_I: usize = 1;
/// Hunting what's this near (cells).
const AGGRO: f32 = 330.0;
/// Bobbing this much as it floats (cells; `float_height`).
const BOB: f32 = 3.0;
/// Its eye's white (cells across, half), its iris and pupil.
const WHITE: f32 = 14.5;
const IRIS: f32 = 6.0;
const PUPIL: f32 = 2.6;
/// The spit: squinting this long; at what's this near; then it waits.
const SPIT: f32 = 0.35;
const SPIT_RANGE: f32 = 260.0;
const SPIT_EVERY: f32 = 2.8;
/// The beam: its tell this long, following at this turn (rad/s) till this
/// long before it fires (then still); lasting this long, reaching this far
/// (cells), what it does to what's in it every so often; the heat it lays
/// where it ends (°C a tick, cells across); at what's from this near to
/// this far; then it waits.
const CHARGE: f32 = 1.4;
/// A volley's later beams charge only this long (the line's already out).
const RECHARGE: f32 = 0.6;
const FOLLOW: f32 = 1.0;
const LOCK: f32 = 0.4;
const BEAM: (f32, f32) = (0.9, 340.0);
const BEAM_HIT: (f32, f32) = (12.0, 0.1);
const SCORCH: (i16, i32) = (45, 3);
const BEAM_RANGE: (f32, f32) = (60.0, 320.0);
const BEAM_EVERY: f32 = 7.0;
/// A lamp this near (cells) and this bright (not a spell in flight: its own
/// spit glows) and it squints and backs off; a
/// flash this near and this bright blinds it this long; a radiant blow,
/// this long.
const GLARE: (f32, f32) = (46.0, 0.3);
const FLASH: (f32, f32, f32) = (90.0, 1.0, 2.6);
const RADIANT_BLIND: f32 = 3.2;

pub(super) fn solid(sim: &SimWorld, p: Vec2) -> bool {
    sim.world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| !c.is_air() && matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder))
}

pub(super) fn clear(sim: &SimWorld, a: Vec2, b: Vec2) -> bool {
    let n = (a.distance(b) / 2.0).ceil() as i32;
    (1..n).all(|i| !solid(sim, a.lerp(b, i as f32 / n as f32)))
}

/// Where a line from `from` along `dir` first meets rock, within `reach`.
fn trace(sim: &SimWorld, from: Vec2, dir: Vec2, reach: f32) -> Vec2 {
    let mut d = 0.0;
    while d < reach {
        let p = from + dir * d;
        if solid(sim, p) {
            return p;
        }
        d += 1.0;
    }
    from + dir * reach
}

/// Floating as a Terraria eye does: mostly high, out of reach (it shoots
/// from up there), and now and then sweeping down within it, beside you,
/// a while (to bite; to be hit), then up again. Its height over the ground
/// now (cells) and whether it's coming down or down (to close in on you
/// sideways). `seed` (0..1) staggers one from another.
pub(super) fn float_height(clock: f32, seed: f32) -> (f32, bool) {
    let period = FLOAT_PERIOD + FLOAT_PERIOD * 0.35 * seed;
    let t = (clock + seed * 31.0).rem_euclid(period);
    // (Down for the last stretch of its round, eased in and out.)
    let into = t - (period - FLOAT_DOWN);
    let ease = |x: f32| x.clamp(0.0, 1.0) * x.clamp(0.0, 1.0) * (3.0 - 2.0 * x.clamp(0.0, 1.0));
    let down = ease(into / FLOAT_EASE).min(ease((FLOAT_DOWN - into) / FLOAT_EASE));
    (FLOAT_HIGH + (FLOAT_LOW - FLOAT_HIGH) * down, into > 0.0 && into < FLOAT_DOWN - FLOAT_EASE)
}

/// Its float (`float_height`): up this high, down this low (cells over the
/// ground), round every so often (s, and up to a third longer), down this
/// long of it, easing this long each way; down, this far to your side.
const FLOAT_HIGH: f32 = 74.0;
const FLOAT_LOW: f32 = 14.0;
const FLOAT_PERIOD: f32 = 9.0;
const FLOAT_DOWN: f32 = 4.2;
const FLOAT_EASE: f32 = 0.9;
pub(super) const FLOAT_BESIDE: f32 = 24.0;

/// A creature's stagger for its float, from its entity.
pub(super) fn float_seed(e: Entity) -> f32 {
    (e.to_bits() % 97) as f32 / 97.0
}

pub(super) fn ground(sim: &SimWorld, at: Vec2, reach: i32) -> Option<f32> {
    (0..reach).map(|n| at - Vec2::Y * n as f32).find(|&p| solid(sim, p - Vec2::Y)).map(|p| p.y.floor())
}

fn sparkle() -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter {
        count: 0.0,
        life: (0.15, 0.4),
        colors: vec![(250, 230, 255), (200, 120, 255), (120, 40, 180)],
        speed: 90.0,
        spread: 1.2,
        gravity: 0.0,
        drag: 3.0,
        size: 1.2,
        jitter: 0.0,
        glow: true,
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn think(
    mut sim: ResMut<SimWorld>,
    book: Res<crate::magic::Spellbook>,
    mut q: Query<(Entity, &mut Watcher, &Kinematics, &mut Controls, &mut Animator)>,
    hunted: Query<(Entity, &Kinematics, &Team), Without<Watcher>>,
    lamps: Query<(&GlobalTransform, &crate::light::LightSource), Without<crate::magic::Spell>>,
    (mut hits, mut casts, mut sounds): (MessageWriter<crate::combat::Hit>, MessageWriter<crate::magic::CastRequest>, MessageWriter<crate::sound::PlaySound>),
    (mut sparks, mut flares, mut trauma): (ResMut<crate::vfx::Sparks>, MessageWriter<crate::fx::Flare>, ResMut<crate::fx::Trauma>),
) {
    let lamps: Vec<(Vec2, f32)> = lamps.iter().map(|(t, l)| (t.translation().truncate(), l.color.iter().copied().fold(0.0f32, f32::max))).filter(|(_, b)| *b >= GLARE.1).collect();
    for (e, mut o, k, mut c, mut anim) in &mut q {
        let (pos, half) = (k.body.pos, k.body.half);
        o.eye = frame_anchor(&anim, k, "eye", "").unwrap_or(pos);
        let eye = o.eye;
        if !o.started {
            o.started = true;
            o.scale = anim.def.scale;
            o.ready = [1.5, 3.0];
            let link = 2.6 * o.scale;
            o.nerves = vec![Links::new(6, link), Links::new(8, link), Links::new(6, link)];
            o.blink = (3.0, 0.0);
            o.lids = 1.0;
            o.aim = Vec2::NEG_Y;
        }
        o.clock += DT;
        let (white, iris) = (WHITE * o.scale, IRIS * o.scale);
        for r in o.ready.iter_mut() {
            *r -= DT;
        }
        // Blinded (`dazzle`): what it was doing's over.
        if o.blind > 0.0 && o.doing != Doing::Hover {
            o.doing = Doing::Hover;
        }
        o.blind = (o.blind - DT).max(0.0);
        // A lamp near: it squints, backs off from it, won't beam.
        o.glare = lamps.iter().filter(|(at, _)| at.distance(eye) < GLARE.0).map(|(at, _)| *at).next();
        let target = hunted.iter().filter(|(_, _, t)| t.hunted()).map(|(e, tk, _)| (e, tk.body.pos)).filter(|(_, p)| p.distance(pos) < AGGRO).min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)));
        // Its iris follows you (or wanders, with no one to watch).
        let want = match target {
            Some((_, tp)) if o.blind <= 0.0 => (tp - eye).normalize_or_zero() * (white - iris - 1.5),
            _ => Vec2::new((o.clock * 0.7).sin(), (o.clock * 0.53).cos() * 0.6) * (white - iris - 3.0),
        };
        o.look = o.look.lerp(want, 1.0 - (-DT * 9.0f32).exp());
        // Blinking now and then.
        o.blink.0 -= DT;
        if o.blink.0 <= 0.0 {
            o.blink = (3.0 + (o.clock * 7.3).sin().abs() * 3.0, 0.14);
        }
        o.blink.1 = (o.blink.1 - DT).max(0.0);
        let doing = o.doing;
        let mut next = doing;
        if o.blind > 0.0 {
            next = Doing::Hover;
        } else {
            match doing {
                Doing::Hover => {
                    if let Some((_, tp)) = target {
                        let dist = tp.distance(eye);
                        let seen = clear(&sim, eye, tp);
                        if o.glare.is_none() && o.ready[BEAM_I] <= 0.0 && seen && (BEAM_RANGE.0..BEAM_RANGE.1).contains(&dist) {
                            o.aim = (tp - eye).normalize_or(Vec2::NEG_Y);
                            o.left = o.beams.max(1);
                            sounds.write(crate::sound::PlaySound::at("cast", eye).pitch(0.35).volume(0.9));
                            next = Doing::Charge(0.0);
                        } else if o.ready[SPIT_I] <= 0.0 && seen && dist < SPIT_RANGE {
                            sounds.write(crate::sound::PlaySound::at("pour", eye).pitch(0.7).volume(0.7));
                            next = Doing::Spit(0.0);
                        }
                    }
                }
                Doing::Spit(t) => {
                    next = if t + DT >= SPIT {
                        // Lobbed: aimed above them by how far they are;
                        // every so often a cloud instead (an elder's), on
                        // them.
                        o.spits += 1;
                        let clouds = o.cloud_every > 0 && o.spits >= o.cloud_every;
                        let (spell, lob) = if clouds { (o.cloud.clone(), 0.0) } else { ("blight_spit".to_string(), 0.22) };
                        if clouds {
                            o.spits = 0;
                        }
                        if let (Some((_, tp)), Some(i)) = (target, book.spells.iter().position(|s| s.id == spell)) {
                            let to = tp + Vec2::Y * (tp.distance(eye) * lob);
                            casts.write(crate::magic::CastRequest { caster: e, spell: i, from: eye + (to - eye).normalize_or(Vec2::NEG_Y) * (white + 2.0), toward: to, alt: false });
                            sounds.write(crate::sound::PlaySound::at("spit", eye).pitch(if clouds { 0.4 } else { 0.6 }).volume(1.0));
                        }
                        o.ready[SPIT_I] = SPIT_EVERY;
                        Doing::Hover
                    } else {
                        Doing::Spit(t + DT)
                    };
                }
                Doing::Charge(t) => {
                    // Following, slowly, then still.
                    if let Some((_, tp)) = target
                        && t < CHARGE - LOCK
                    {
                        let want = (tp - eye).normalize_or(o.aim);
                        let turn = (want.to_angle() - o.aim.to_angle() + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                        o.aim = Vec2::from_angle(o.aim.to_angle() + turn.clamp(-FOLLOW * DT, FOLLOW * DT));
                    }
                    o.end = trace(&sim, eye, o.aim, BEAM.1);
                    if ((t + DT) / 0.2) as u32 != (t / 0.2) as u32 {
                        flares.write(crate::fx::Flare { at: eye, color: [0.6 + t / CHARGE, 0.2, 0.9 + t / CHARGE], life: 0.22 });
                        sounds.write(crate::sound::PlaySound::at("cast", eye).pitch(0.35 + t * 0.5).volume(0.5));
                    }
                    next = if t + DT >= CHARGE {
                        sounds.write(crate::sound::PlaySound::at("laser", eye).pitch(0.6).volume(1.0));
                        trauma.0 = (trauma.0 + 0.25).min(1.0);
                        // (Sweeping: from one side of where it locked to
                        // the other, the other way each beam.)
                        o.base = o.aim;
                        o.turn = -o.turn;
                        Doing::Beam(0.0)
                    } else {
                        Doing::Charge(t + DT)
                    };
                }
                Doing::Beam(t) => {
                    if o.sweep != 0.0 {
                        let across = (t / BEAM.0 - 0.5) * o.sweep.to_radians() * o.turn;
                        o.aim = Vec2::from_angle(across).rotate(o.base);
                    }
                    o.end = trace(&sim, eye, o.aim, BEAM.1);
                    let (a, b) = (eye + o.aim * (white + 1.0), o.end);
                    // What's in it, every so often: burnt.
                    if (t / BEAM_HIT.1) as u32 != ((t + DT) / BEAM_HIT.1) as u32 {
                        for (who, tk, tt) in &hunted {
                            if !tt.hunted() {
                                continue;
                            }
                            let ab = b - a;
                            let s = ((tk.body.pos - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
                            let near = a + ab * s;
                            if ((tk.body.pos - near).abs() - tk.body.half).max(Vec2::ZERO).length() <= 2.5 {
                                hits.write(crate::combat::Hit { target: who, damage: o.beam_hit, harm: Harm::Fire, knock: o.aim * 60.0, stun: 0.05, at: near, dir: o.aim, weight: 0.4, crit: false });
                            }
                        }
                    }
                    // Where it ends, scorched: heat (the rock glows, grass burns).
                    sim.queue(platypus_sim::WorldEdit::Heat { center: CellPos::from_world(b.x, b.y), radius: SCORCH.1, amount: SCORCH.0 });
                    sparks.emit(&sparkle(), 3, b, -o.aim, Vec2::ZERO);
                    if ((t + DT) / 0.1) as u32 != (t / 0.1) as u32 {
                        flares.write(crate::fx::Flare { at: b, color: [1.2, 0.5, 1.6], life: 0.12 });
                    }
                    next = if t + DT >= BEAM.0 && o.left > 1 && o.glare.is_none() {
                        // The volley's next: its line out again at once,
                        // after them (a short charge).
                        o.left -= 1;
                        if let Some((_, tp)) = target {
                            o.aim = (tp - eye).normalize_or(o.aim);
                        }
                        sounds.write(crate::sound::PlaySound::at("cast", eye).pitch(0.5).volume(0.8));
                        Doing::Charge(CHARGE - RECHARGE)
                    } else if t + DT >= BEAM.0 {
                        o.ready[BEAM_I] = BEAM_EVERY;
                        o.ready[SPIT_I] = o.ready[SPIT_I].max(1.0);
                        Doing::Hover
                    } else {
                        Doing::Beam(t + DT)
                    };
                }
            }
        }
        o.doing = next;
        // Its lids: shut blinded or blinking; squinting at a lamp or to
        // spit; wide charging and beaming.
        let (clip, lids) = if o.blind > 0.0 || o.blink.1 > 0.0 {
            ("shut", 0.0)
        } else {
            match next {
                Doing::Spit(_) => ("squint", 0.35),
                Doing::Charge(_) | Doing::Beam(_) => ("idle", 1.0),
                Doing::Hover if o.glare.is_some() => ("squint", 0.35),
                Doing::Hover => ("idle", 1.0),
            }
        };
        if anim.force.as_deref() != Some(clip) {
            anim.play(clip);
        }
        o.lids = lids;
        // Floating: held high over the ground, bobbing; blinded, it sinks
        // and reels; at a lamp, it backs off from it; busy, it holds still.
        let feet = pos.y - half.y;
        let over = ground(&sim, Vec2::new(pos.x, feet + 2.0), 120).map(|g| feet - g);
        // (Mostly up out of reach, now and then down beside you: Terraria's
        // eyes. Blinded, it sinks low, reeling.)
        let (float, down) = float_height(o.clock, float_seed(e));
        let height = if o.blind > 0.0 { FLOAT_LOW } else { float } + (o.clock * 1.3).sin() * BOB;
        c.0.move_y = over.map_or(-0.5, |h| ((height - h) * 0.08).clamp(-1.0, 1.0));
        c.0.jump = false;
        if o.blind > 0.0 {
            c.0.move_x = (o.clock * 2.3).sin() * 0.6;
        } else if let Some(at) = o.glare {
            c.0.move_x = if at.x < pos.x { 1.0 } else { -1.0 };
        } else if !matches!(next, Doing::Hover) {
            c.0.move_x = 0.0;
        } else if let (true, Some((_, tp))) = (down, target) {
            let to = tp.x + FLOAT_BESIDE * (pos.x - tp.x).signum() - pos.x;
            c.0.move_x = (to * 0.03).clamp(-1.0, 1.0);
        }
        // Its nerves trailing under it.
        let scale = o.scale;
        for (i, l) in o.nerves.iter_mut().enumerate() {
            let x = (i as f32 - 1.0) * 6.0 * scale;
            l.swing(eye + Vec2::new(x, -white + 1.5 + (i == 1) as u8 as f32 * -0.5), 300.0);
        }
    }
}

/// A flash near (a blast, a radiant spell, a flare) or a radiant blow
/// blinds it.
fn dazzle(sim: Res<SimWorld>, mut q: Query<(Entity, &mut Watcher)>, mut flashes: MessageReader<crate::fx::Flare>, mut struck: MessageReader<crate::combat::Hit>, mut sounds: MessageWriter<crate::sound::PlaySound>) {
    let flashes: Vec<(Vec2, f32)> = flashes.read().map(|f| (f.at, f.color.iter().copied().fold(0.0f32, f32::max))).filter(|(_, b)| *b >= FLASH.1).collect();
    let radiant: Vec<Entity> = struck.read().filter(|h| h.harm == Harm::Radiant).map(|h| h.target).collect();
    for (e, mut o) in &mut q {
        if !o.started {
            continue;
        }
        let eye = o.eye;
        // (Its own light doesn't blind it: its eye kindling, its beam's end.)
        let busy = matches!(o.doing, Doing::Charge(_) | Doing::Beam(_));
        let mine = |at: Vec2| busy && (at.distance(eye) < 3.0 || at.distance(o.end) < 3.0);
        let flashed = flashes.iter().any(|(at, _)| !mine(*at) && at.distance(eye) < FLASH.0 && clear(&sim, *at, eye));
        let hit = radiant.contains(&e);
        if flashed || hit {
            if o.blind <= 0.0 {
                sounds.write(crate::sound::PlaySound::at("groan", eye).pitch(1.9).volume(0.9));
            }
            o.blind = o.blind.max(if hit { RADIANT_BLIND } else { FLASH.2 });
        }
    }
}

/// Dead, it bursts: blight.
fn burst(mut died: MessageReader<Died>, mut sim: ResMut<SimWorld>, mut sounds: MessageWriter<crate::sound::PlaySound>, mut sparks: ResMut<crate::vfx::Sparks>) {
    for d in died.read() {
        if d.def.custom.as_ref().is_none_or(|c| c.name != Watcher::NAME) {
            continue;
        }
        if let Some(blight) = sim.materials().id("blight") {
            // (An elder, more.)
            let radius = (4.0 * d.def.scale).round() as i32;
            sim.queue(platypus_sim::WorldEdit::Paint { center: CellPos::from_world(d.body.pos.x, d.body.pos.y), radius, material: blight, overwrite: false });
        }
        sparks.emit(&sparkle(), 40, d.body.pos, Vec2::Y, Vec2::ZERO);
        sounds.write(crate::sound::PlaySound::at("spit", d.body.pos).pitch(0.4).volume(1.0));
        sounds.write(crate::sound::PlaySound::at("groan", d.body.pos).pitch(2.2).volume(0.8));
    }
}

/// Its iris and pupil, its nerves, its line and its beam.
#[derive(Resource)]
pub struct WatcherCanvas(Canvas);

impl Default for WatcherCanvas {
    fn default() -> Self {
        WatcherCanvas(Canvas::new("Watchers", 10.4))
    }
}

fn draw(mut commands: Commands, mut canvas: ResMut<WatcherCanvas>, mut images: ResMut<Assets<Image>>, camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>, q: Query<&Watcher>, time: Res<Time>, mut sprites: CanvasSprites) {
    let Ok((cam, loader)) = camera.single() else { return };
    let any = q.iter().any(|o| o.started);
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, any) else { return };
    let now = time.elapsed_secs();
    for o in q.iter().filter(|o| o.started) {
        // Its nerves: fleshy, thinning to their ends.
        for l in &o.nerves {
            let pts = l.points();
            for i in 1..pts.len() {
                let (a, b) = (pts[i - 1], pts[i]);
                let w = if i < 3 { 1 } else { 0 };
                let n = (a.distance(b) * 2.0).ceil() as i32;
                for s in 0..=n {
                    let p = a.lerp(b, s as f32 / n.max(1) as f32).floor().as_ivec2();
                    px.put(p.x, p.y, [110, 48, 62, 255]);
                    if w > 0 {
                        px.put(p.x + 1, p.y, [80, 30, 44, 255]);
                    }
                }
            }
        }
        // Its iris and pupil, where it looks, inside its lids.
        let charge = match o.doing {
            Doing::Charge(t) => t / CHARGE,
            Doing::Beam(_) => 1.0,
            _ => 0.0,
        };
        let (white, iris_r) = (WHITE * o.scale, IRIS * o.scale);
        if o.lids > 0.0 {
            let c = o.eye + o.look;
            let n = iris_r.ceil() as i32 + 1;
            let pupil = PUPIL * o.scale * (1.0 - charge * 0.55);
            let (ir, ig, ib) = o.iris;
            let (inner, outer) = (Vec3::new(ir as f32, ig as f32, ib as f32), Vec3::new(ir as f32 * 0.7, ig as f32 * 0.6, ib as f32 * 0.5));
            for y in -n..=n {
                for x in -n..=n {
                    let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    if d.length() > iris_r {
                        continue;
                    }
                    let p = (c + d).floor().as_ivec2();
                    // (Inside its white, and its lids' almond.)
                    let e = Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5) - o.eye;
                    if e.length() > white - 0.5 {
                        continue;
                    }
                    let band = if o.lids >= 0.9 { white } else { o.lids * white * (1.0 - (e.x / (white + 0.5)).powi(2)).max(0.0).sqrt() };
                    if e.y.abs() > band {
                        continue;
                    }
                    let r = d.length();
                    let mix = |a: Vec3, b: Vec3, f: f32| a.lerp(b, f.clamp(0.0, 1.0));
                    let col = if r <= pupil {
                        [10, 6, 8, 255]
                    } else if r > iris_r - 1.0 {
                        [60, 30, 16, 255]
                    } else {
                        // (Amber, darker out to its rim, in streaks; kindling
                        // violet-white as it charges.)
                        let streak = ((r * 2.3 + (d.y.atan2(d.x) * 3.0).sin()) as i32 % 2) as f32 * 0.12;
                        let c = mix(mix(inner, outer, r / iris_r * 0.7 + streak), Vec3::new(240.0, 200.0, 255.0), charge);
                        [c.x as u8, c.y as u8, c.z as u8, 255]
                    };
                    px.put(p.x, p.y, col);
                }
            }
            // A glint.
            let g = (c + Vec2::new(-2.0, 2.0)).floor().as_ivec2();
            px.put(g.x, g.y, [255, 255, 255, 255]);
        }
        // The line reaching out (the beam's tell): flickering, then still
        // and bright as it locks.
        if let Doing::Charge(t) = o.doing {
            let locked = t >= CHARGE - LOCK;
            let (a, b) = (o.eye + o.aim * (white + 1.0), o.end);
            let n = a.distance(b) as i32;
            for i in 0..n {
                if !locked && ((i as f32 * 0.7 + now * 30.0).sin() < 0.2) {
                    continue;
                }
                let p = a.lerp(b, i as f32 / n.max(1) as f32).floor().as_ivec2();
                px.put(p.x, p.y, if locked { [255, 200, 255, 255] } else { [190, 90, 230, 255] });
            }
        }
        // The beam: a white core in a violet glow, flickering.
        if let Doing::Beam(_) = o.doing {
            let (a, b) = (o.eye + o.aim * (white - 1.0), o.end);
            let side = o.aim.perp();
            let n = (a.distance(b) * 1.5) as i32;
            let swell = 1.0 + (now * 40.0).sin() * 0.4;
            for i in 0..n {
                let p = a.lerp(b, i as f32 / n.max(1) as f32);
                for w in -4..=4 {
                    let off = w as f32 * 0.8;
                    if off.abs() > 3.0 * swell {
                        continue;
                    }
                    let q = (p + side * off).floor().as_ivec2();
                    let col = if off.abs() < 1.0 { [255, 245, 255, 255] } else if off.abs() < 2.0 { [220, 150, 255, 255] } else { [150, 60, 210, 255] };
                    px.put(q.x, q.y, col);
                }
            }
        }
    }
}
