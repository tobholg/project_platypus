//! Bloat toads: the size of a pony, swollen, warty, venting gas. They hop
//! after you (their brain: `Hop`), landing heavily, and swell as they're
//! hurt (their sitting picture: fat, then huge), till they burst.
//!
//! - **Tongue**: its throat swells and its mouth gapes (the tell), then its
//!   tongue flies at you, out to `TONGUE.1`: stuck, you're reeled in to its
//!   mouth, chomped and spat out. A dash slips it; a blade cuts it (the
//!   toad recoils, and it's slow to try again).
//! - **Belch**: now and then it swells and belches a cloud of swamp gas at
//!   you (swamp gas: it hangs a while): a torch or a fire spell there, and
//!   it goes up.
//! - **Burst**: dead, it's blown apart into a cloud of swamp gas (and
//!   blood): lit
//!   beside a crowd, it's a bomb; dead in a fire, it goes up at once.

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind};
use serde::Deserialize;

use super::{CustomCreature, CustomSet, frame_anchor};
use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprites};
use crate::creatures::body::animation::Animator;
use crate::creatures::{Controls, Died, Harm, Health, Kinematics, Team, Tugged};
use crate::world::{ChunkLoader, SimWorld};

#[derive(Component, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Toad {
    #[serde(skip)]
    doing: Doing,
    /// Its mouth (where its tongue comes from) and the tongue's tip.
    #[serde(skip)]
    mouth: Vec2,
    #[serde(skip)]
    tip: Vec2,
    /// Which way the tongue flies.
    #[serde(skip)]
    aim: Vec2,
    /// Its tongue's and its belch's waits.
    #[serde(skip)]
    ready: [f32; 2],
    /// Off the ground last tick (its landing's thud).
    #[serde(skip)]
    aloft: bool,
    #[serde(skip)]
    started: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Doing {
    /// Sitting, hopping (its brain).
    #[default]
    Sit,
    /// Its throat swelling, its mouth agape (the tongue's tell).
    Gape(f32),
    /// The tongue flying out.
    Shoot(f32),
    /// Stuck to whom, reeling them in.
    Stuck(f32, Entity),
    /// Chomping whom it reeled in, then spitting them out.
    Chomp(f32),
    /// Drawing back its tongue, empty.
    Reel,
    /// Its tongue cut: recoiling.
    Cut(f32),
    /// Swelling to belch (the tell); belching.
    Swell(f32),
}

impl CustomCreature for Toad {
    const NAME: &'static str = "toad";

    fn build(app: &mut App) {
        app.init_resource::<TongueCanvas>()
            .add_systems(FixedUpdate, think.in_set(CustomSet::Think))
            .add_systems(FixedUpdate, burst.in_set(crate::world::TickSet::Bodies))
            .add_systems(Update, draw);
    }
}

impl Toad {
    /// What it's doing, in a word (a scenario's log).
    pub fn doing(&self) -> &'static str {
        match self.doing {
            Doing::Sit => "sit",
            Doing::Gape(_) => "gape",
            Doing::Shoot(_) => "shoot",
            Doing::Stuck(..) => "stuck",
            Doing::Chomp(_) => "chomp",
            Doing::Reel => "reel",
            Doing::Cut(_) => "cut",
            Doing::Swell(_) => "swell",
        }
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
const TONGUE_I: usize = 0;
const BELCH_I: usize = 1;
/// Hunting what's this near (cells).
const AGGRO: f32 = 320.0;
/// The tongue: from this near to this far (across; up or down this
/// much); gaping this long first; flying this fast, reeled back this fast
/// (cells/s), what it reels in this fast, for at most this long, stuck at
/// least this long before it chomps (a dash, a cut, has time); how near its
/// tip must come to stick; then it waits this long.
const TONGUE: (f32, f32, f32) = (28.0, 125.0, 70.0);
const GAPE: f32 = 0.42;
const FLY: f32 = 760.0;
const BACK: f32 = 520.0;
const REEL: (f32, f32, f32) = (220.0, 1.5, 0.3);
const STICK: f32 = 3.5;
const TONGUE_EVERY: f32 = 3.4;
/// Stuck, the first yank (toward its mouth, up); and reeled in this near,
/// it chomps: how much, spat out how hard (ahead, up); chomping this long.
const YANK: (f32, f32) = (170.0, 130.0);
const CHOMP_NEAR: f32 = 16.0;
const CHOMP: (f32, f32, f32, f32) = (20.0, 330.0, 220.0, 0.45);
/// Cut: recoiling this long, then waiting this long to try again.
const CUT: (f32, f32) = (1.0, 4.5);
/// The belch: swelling this long; its cloud (cells across, ahead of it);
/// then it waits; it belches at what's this near.
const SWELL: f32 = 0.6;
const BELCH: (i32, f32) = (6, 16.0);
const BELCH_EVERY: f32 = 9.0;
const BELCH_NEAR: f32 = 150.0;
/// Its burst: the swamp gas (cells across), the blood.
const BURST: (i32, i32) = (13, 4);
/// Landing from a hop this fast (down, cells/s) or faster: a thud.
const THUD: f32 = 160.0;

fn solid(sim: &SimWorld, p: Vec2) -> bool {
    sim.world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| !c.is_air() && matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder))
}

/// Nothing solid between `a` and `b`.
fn clear(sim: &SimWorld, a: Vec2, b: Vec2) -> bool {
    let n = (a.distance(b) / 2.0).ceil() as i32;
    (1..n).all(|i| !solid(sim, a.lerp(b, i as f32 / n as f32)))
}

fn gore() -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter {
        count: 0.0,
        life: (0.2, 0.6),
        colors: vec![(150, 30, 34), (110, 20, 24), (190, 60, 60)],
        speed: 140.0,
        spread: 1.2,
        gravity: 400.0,
        drag: 1.5,
        size: 1.3,
        jitter: 0.0,
        glow: false,
    }
}

fn puff() -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter {
        count: 0.0,
        life: (0.4, 1.0),
        colors: vec![(140, 200, 120), (110, 170, 100), (170, 220, 150)],
        speed: 60.0,
        spread: 1.0,
        gravity: -20.0,
        drag: 2.0,
        size: 1.8,
        jitter: 0.0,
        glow: false,
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn think(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    mut q: Query<(Entity, &mut Toad, &Kinematics, &mut Controls, &mut Animator, &Health)>,
    hunted: Query<(Entity, &Kinematics, &Team), Without<Toad>>,
    mut dashed: MessageReader<crate::combat::Dashed>,
    blades: Res<crate::combat::LiveBlades>,
    (mut hits, mut sounds, mut recoils): (MessageWriter<crate::combat::Hit>, MessageWriter<crate::sound::PlaySound>, MessageWriter<crate::combat::Recoil>),
    (mut sparks, mut trauma): (ResMut<crate::vfx::Sparks>, ResMut<crate::fx::Trauma>),
) {
    let dashers: Vec<Entity> = dashed.read().map(|d| d.0).collect();
    for (e, mut o, k, mut c, mut anim, health) in &mut q {
        let (pos, facing) = (k.body.pos, k.loco.facing);
        o.mouth = frame_anchor(&anim, k, "tongue", "").unwrap_or(pos + Vec2::new(facing * 18.0, 4.0));
        if !o.started {
            o.started = true;
            o.tip = o.mouth;
            o.ready = [1.0, 4.0];
        }
        for r in o.ready.iter_mut() {
            *r -= DT;
        }
        // Down from a hop: a thud, dust.
        let grounded = k.loco.grounded();
        if grounded && o.aloft && k.prev_pos.y - pos.y > THUD * DT {
            sounds.write(crate::sound::PlaySound::at("stomp", pos).pitch(0.55).volume(0.9));
            trauma.0 = (trauma.0 + 0.12).min(1.0);
        }
        o.aloft = !grounded;
        let target = hunted.iter().filter(|(_, _, t)| t.hunted()).map(|(e, tk, _)| (e, tk.body.pos)).filter(|(_, p)| p.distance(pos) < AGGRO).min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)));
        let mouth = o.mouth;
        let doing = o.doing;
        let mut next = doing;
        match doing {
            Doing::Sit => {
                o.tip = mouth;
                // Sitting, its picture swells with its hurt.
                let hurt = 1.0 - health.hp / health.max.max(1.0);
                let sit = if hurt > 0.66 { "idle_huge" } else if hurt > 0.33 { "idle_fat" } else { "idle" };
                if grounded && k.body.vel.x.abs() < 6.0 {
                    if anim.force.as_deref() != Some(sit) {
                        anim.play(sit);
                    }
                } else {
                    anim.force = None;
                }
                if let Some((_, tp)) = target
                    && grounded
                {
                    let off = tp - mouth;
                    let ahead = off.x * facing;
                    if (TONGUE.0..TONGUE.1).contains(&ahead) && off.y.abs() < TONGUE.2 && o.ready[TONGUE_I] <= 0.0 && clear(&sim, mouth, tp) {
                        sounds.write(crate::sound::PlaySound::at("groan", mouth).pitch(2.1).volume(0.8));
                        next = Doing::Gape(0.0);
                    } else if off.length() < BELCH_NEAR && ahead > 0.0 && o.ready[BELCH_I] <= 0.0 {
                        sounds.write(crate::sound::PlaySound::at("groan", mouth).pitch(1.5).volume(0.7));
                        next = Doing::Swell(0.0);
                    }
                }
            }
            Doing::Gape(t) => {
                anim.play("gape");
                o.tip = mouth;
                next = if t + DT >= GAPE {
                    // At where they'll be, a little.
                    let lead = target.map_or(mouth + Vec2::X * facing * TONGUE.1, |(who, tp)| tp + hunted.get(who).map_or(Vec2::ZERO, |h| h.1.body.vel * 0.08));
                    o.aim = (lead - mouth).normalize_or(Vec2::X * facing);
                    sounds.write(crate::sound::PlaySound::at("spit", mouth).pitch(0.6).volume(1.0));
                    Doing::Shoot(0.0)
                } else {
                    Doing::Gape(t + DT)
                };
            }
            Doing::Shoot(t) => {
                anim.play("gape");
                let was = o.tip;
                let reach = ((t + DT) * FLY).min(TONGUE.1);
                o.tip = mouth + o.aim * reach;
                next = if let Some(at) = blades.across(mouth, o.tip, e) {
                    cut_off(&mut o, at, &mut sounds, &mut sparks)
                } else if let Some((who, _, _)) = hunted.iter().filter(|(_, _, t)| t.hunted()).find(|(_, tk, _)| {
                    // (Swept from where its tip was: it flies fast.)
                    (0..=4).any(|i| {
                        let p = was.lerp(o.tip, i as f32 / 4.0);
                        ((tk.body.pos - p).abs() - tk.body.half).max(Vec2::ZERO).length() <= STICK
                    })
                }) {
                    // Stuck: yanked toward its mouth, and up off their feet.
                    let (tp, vel) = hunted.get(who).map_or((o.tip, Vec2::ZERO), |h| (h.1.body.pos, h.1.body.vel));
                    let to = (mouth - tp).normalize_or(-Vec2::X * facing);
                    recoils.write(crate::combat::Recoil { who, add: Vec2::new(to.x * YANK.0, YANK.1) - vel, pogo: None, dive: None });
                    sounds.write(crate::sound::PlaySound::at("snap", o.tip).pitch(0.7).volume(0.9));
                    Doing::Stuck(0.0, who)
                } else if reach >= TONGUE.1 || solid(&sim, o.tip) {
                    Doing::Reel
                } else {
                    Doing::Shoot(t + DT)
                };
            }
            Doing::Stuck(t, who) => {
                anim.play("gape");
                let theirs = hunted.get(who).ok().map(|h| h.1.body.pos);
                if let Some(tp) = theirs {
                    o.tip = tp;
                }
                next = if let Some(at) = blades.across(mouth, o.tip, e) {
                    cut_off(&mut o, at, &mut sounds, &mut sparks)
                } else if dashers.contains(&who) || theirs.is_none() || t + DT >= REEL.1 {
                    Doing::Reel
                } else if t >= REEL.2 && theirs.is_some_and(|tp| tp.distance(mouth) < CHOMP_NEAR) {
                    // Reeled in: chomped, spat out.
                    let knock = Vec2::new(facing * CHOMP.1, CHOMP.2);
                    hits.write(crate::combat::Hit { by: Some(e), target: who, damage: CHOMP.0, harm: Harm::Pierce, knock, stun: 0.4, at: mouth, dir: knock.normalize(), weight: 1.6, crit: false });
                    sounds.write(crate::sound::PlaySound::at("kick", mouth).pitch(0.5).volume(1.0));
                    sparks.emit(&gore(), 16, mouth, Vec2::new(facing, 0.3), Vec2::ZERO);
                    Doing::Chomp(0.0)
                } else {
                    commands.entity(who).insert(Tugged { to: mouth, speed: REEL.0, left: 0.1 });
                    Doing::Stuck(t + DT, who)
                };
                if !matches!(next, Doing::Stuck(..))
                    && let Ok(mut ec) = commands.get_entity(who)
                {
                    ec.remove::<Tugged>();
                }
            }
            Doing::Chomp(t) => {
                anim.play("swell");
                o.tip = mouth;
                next = if t + DT >= CHOMP.3 {
                    o.ready[TONGUE_I] = TONGUE_EVERY;
                    Doing::Sit
                } else {
                    Doing::Chomp(t + DT)
                };
            }
            Doing::Reel => {
                anim.play("gape");
                let back = o.tip - mouth;
                let step = BACK * DT;
                next = if back.length() <= step {
                    o.tip = mouth;
                    o.ready[TONGUE_I] = TONGUE_EVERY;
                    Doing::Sit
                } else {
                    o.tip = mouth + back * (1.0 - step / back.length());
                    Doing::Reel
                };
            }
            Doing::Cut(t) => {
                anim.play("hurt");
                o.tip = o.tip.lerp(mouth, 0.25);
                next = if t + DT >= CUT.0 {
                    o.tip = mouth;
                    o.ready[TONGUE_I] = CUT.1;
                    Doing::Sit
                } else {
                    Doing::Cut(t + DT)
                };
            }
            Doing::Swell(t) => {
                anim.play("swell");
                o.tip = mouth;
                next = if t + DT >= SWELL {
                    // A belch: a cloud of swamp gas, ahead of it.
                    let at = mouth + Vec2::X * facing * BELCH.1;
                    if let Some(gas) = sim.materials().id("swamp_gas") {
                        sim.queue(platypus_sim::WorldEdit::Paint { center: CellPos::from_world(at.x, at.y), radius: BELCH.0, material: gas, overwrite: false });
                    }
                    sparks.emit(&puff(), 24, at, Vec2::X * facing, Vec2::ZERO);
                    sounds.write(crate::sound::PlaySound::at("groan", mouth).pitch(0.8).volume(1.0));
                    o.ready[BELCH_I] = BELCH_EVERY;
                    Doing::Sit
                } else {
                    Doing::Swell(t + DT)
                };
            }
        }
        o.doing = next;
        // (Busy, it sits still.)
        if !matches!(next, Doing::Sit) {
            c.0.move_x = 0.0;
            c.0.jump = false;
        }
    }
}

/// Its tongue cut (at `at`): blood, a squeal; it recoils.
fn cut_off(o: &mut Toad, at: Vec2, sounds: &mut MessageWriter<crate::sound::PlaySound>, sparks: &mut crate::vfx::Sparks) -> Doing {
    sounds.write(crate::sound::PlaySound::at("groan", at).pitch(2.4).volume(1.0));
    sounds.write(crate::sound::PlaySound::at("hit", at).volume(0.8));
    sparks.emit(&gore(), 24, at, Vec2::Y, Vec2::ZERO);
    let _ = o;
    Doing::Cut(0.0)
}

/// Dead: it bursts, a cloud of swamp gas and blood. (A fire near it, it goes
/// up: the gas catches.)
fn burst(mut died: MessageReader<Died>, mut sim: ResMut<SimWorld>, mut sounds: MessageWriter<crate::sound::PlaySound>, mut sparks: ResMut<crate::vfx::Sparks>, mut trauma: ResMut<crate::fx::Trauma>) {
    for d in died.read() {
        if d.def.custom.as_ref().is_none_or(|c| c.name != Toad::NAME) {
            continue;
        }
        let at = d.body.pos;
        let c = CellPos::from_world(at.x, at.y);
        let m = sim.materials().clone();
        if let Some(gas) = m.id("swamp_gas") {
            sim.queue(platypus_sim::WorldEdit::Paint { center: c, radius: BURST.0, material: gas, overwrite: false });
        }
        if let Some(blood) = m.id("blood") {
            sim.queue(platypus_sim::WorldEdit::Paint { center: c.offset(0, 4), radius: BURST.1, material: blood, overwrite: false });
        }
        sparks.emit(&gore(), 60, at, Vec2::Y, Vec2::ZERO);
        sparks.emit(&puff(), 50, at, Vec2::Y, Vec2::ZERO);
        sounds.write(crate::sound::PlaySound::at("boom", at).pitch(1.6).volume(0.7));
        sounds.write(crate::sound::PlaySound::at("spit", at).pitch(0.5).volume(1.0));
        trauma.0 = (trauma.0 + 0.25).min(1.0);
    }
}

/// Its tongue, over it.
#[derive(Resource)]
pub struct TongueCanvas(Canvas);

impl Default for TongueCanvas {
    fn default() -> Self {
        TongueCanvas(Canvas::new("Tongues", 11.12))
    }
}

fn draw(mut commands: Commands, mut canvas: ResMut<TongueCanvas>, mut images: ResMut<Assets<Image>>, camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>, q: Query<&Toad>, mut sprites: CanvasSprites) {
    let Ok((cam, loader)) = camera.single() else { return };
    let out = |o: &Toad| o.started && o.tip.distance(o.mouth) > 2.0;
    let any = q.iter().any(out);
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, any) else { return };
    for o in q.iter().filter(|o| out(o)) {
        let (a, b) = (o.mouth, o.tip);
        let side = (b - a).perp().normalize_or(Vec2::Y);
        let n = (a.distance(b) * 2.0).ceil() as i32;
        for i in 0..=n {
            let p = a.lerp(b, i as f32 / n.max(1) as f32);
            // (Thicker toward its root; pink, a darker underside.)
            let w = 1.6 - 0.5 * i as f32 / n.max(1) as f32;
            for s in [-1.0f32, -0.5, 0.0, 0.5, 1.0] {
                let q = (p + side * s * w).floor().as_ivec2();
                px.put(q.x, q.y, if s < 0.0 { [168, 70, 90, 255] } else { [222, 112, 132, 255] });
            }
        }
        // The sticky tip.
        for y in -3..=3 {
            for x in -3..=3 {
                let d = Vec2::new(x as f32, y as f32);
                if d.length() <= 3.0 {
                    let q = (b + d).floor().as_ivec2();
                    px.put(q.x, q.y, if d.y > 1.0 { [250, 176, 190, 255] } else if d.length() > 2.2 { [176, 76, 98, 255] } else { [236, 132, 152, 255] });
                }
            }
        }
    }
}
