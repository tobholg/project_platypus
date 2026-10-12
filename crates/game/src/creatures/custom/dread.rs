//! Dread knights: tall as two of you, crowned and hooded, a long torn
//! cloak and rags (cloth), chains at its belt and wrist, a great spiked
//! ball on a chain in its fist (the flail: a weight on a chain, swung).
//! It walks at you, never hurrying (its brain); shrieks when it first sees
//! you. Far, it whirls the ball over its head (the tell) and brings it down
//! where you stand: the ground cratered, what's near thrown. Near, it
//! draws the ball back and sweeps it along the ground (jump it). Between,
//! the ball drags behind it, and hurts what it hits going fast. Dead, its
//! cloak can be worn.

use std::sync::Arc;

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind};
use serde::Deserialize;

use super::{CustomCreature, CustomSet};
use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprites};
use crate::cloth::{ClothDef, Drapes, Links, draw_links};
use crate::creatures::{Controls, Died, Harm, Kinematics, Team};
use crate::world::{ChunkLoader, SimWorld};

/// What it's doing and what it carries.
#[derive(Component, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Dread {
    #[serde(skip)]
    doing: Doing,
    /// The ball, where it was last tick; the hand that holds its chain.
    #[serde(skip)]
    ball: Vec2,
    #[serde(skip)]
    ball_prev: Vec2,
    #[serde(skip)]
    hand: Vec2,
    #[serde(skip)]
    slam_ready: f32,
    #[serde(skip)]
    sweep_ready: f32,
    /// Whom the ball's struck this swing (once each).
    #[serde(skip)]
    struck: Vec<Entity>,
    #[serde(skip)]
    shrieked: bool,
    /// Chains hanging at its belt and wrist.
    #[serde(skip)]
    chains: Vec<Links>,
    #[serde(skip)]
    started: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Doing {
    /// Walking, the ball dragging behind.
    #[default]
    Carry,
    /// Whirling it overhead (the tell).
    Whirl(f32),
    /// Bringing it down, from there to there.
    Slam(f32, Vec2, Vec2),
    /// Where it came down, a moment.
    Rest(f32),
    /// Drawing it back low (the sweep's tell), from there.
    Draw(f32, Vec2),
    /// Sweeping it along the ground.
    Sweep(f32),
}

impl CustomCreature for Dread {
    const NAME: &'static str = "dread";

    fn build(app: &mut App) {
        app.init_resource::<FlailCanvas>()
            .add_observer(on_spawn)
            .add_systems(FixedUpdate, think.in_set(CustomSet::Think))
            .add_systems(FixedUpdate, on_death.in_set(crate::world::TickSet::Bodies))
            .add_systems(Update, draw);
    }
}

impl Dread {
    /// What it's doing, in a word (a scenario's log).
    pub fn doing(&self) -> &'static str {
        match self.doing {
            Doing::Carry => "carry",
            Doing::Whirl(_) => "whirl",
            Doing::Slam(..) => "slam",
            Doing::Rest(_) => "rest",
            Doing::Draw(..) => "draw",
            Doing::Sweep(_) => "sweep",
        }
    }
}

const DT: f32 = (1.0 / crate::world::TICK_HZ) as f32;
/// The flail: its chain's length, the ball's size, how hard it falls,
/// links in its chain.
const CHAIN: f32 = 46.0;
const BALL: f32 = 5.5;
const BALL_FALL: f32 = 1100.0;
const CHAIN_LINKS: usize = 12;
/// Hunting what's this near (cells).
const AGGRO: f32 = 360.0;
/// The slam: whirled this long, at most this far round its hand; down in
/// this long, this high an arc; resting there; then it waits.
const WHIRL: (f32, f32) = (0.9, 22.0);
const SLAM: (f32, f32) = (0.32, 30.0);
const REST: f32 = 0.7;
const SLAM_EVERY: f32 = 3.5;
/// It slams what's within its chain's reach and this (cells).
const SLAM_FROM: f32 = 8.0;
/// What the slam does: how far (cells from where it lands), how much, how
/// hard; the crater.
const QUAKE: (f32, f32, f32) = (18.0, 28.0, 380.0);
const CRATER: (i32, u8) = (4, 22);
/// The sweep: drawn back this long, swept round in this long (from, to:
/// degrees the way it faces), how much, how hard; then it waits.
const DRAW: f32 = 0.5;
const SWEEP: (f32, f32, f32) = (0.55, 200.0, 345.0);
const SWEEP_HIT: (f32, f32) = (22.0, 340.0);
const SWEEP_EVERY: f32 = 3.0;
/// Near enough to sweep (across, up or down).
const SWEEP_NEAR: (f32, f32) = (36.0, 24.0);
/// Swung fast, the ball hurts what it hits.
const FAST: (f32, f32) = (300.0, 14.0);

/// Where its flail hand is (from its middle, the way it faces).
fn hand_at(pos: Vec2, facing: f32, doing: Doing) -> Vec2 {
    let o = match doing {
        Doing::Whirl(_) | Doing::Slam(..) => Vec2::new(9.5, 24.5),
        Doing::Rest(_) => Vec2::new(10.0, -0.5),
        Doing::Draw(..) | Doing::Sweep(_) => Vec2::new(5.5, -2.5),
        Doing::Carry => Vec2::new(9.5, -4.5),
    };
    pos + Vec2::new(o.x * facing, o.y)
}

/// Its cloak and rags, its chains: hung as it comes.
fn on_spawn(add: On<Add, Dread>, mut commands: Commands) {
    let e = add.entity;
    let seed = e.to_bits();
    let mut drapes = Drapes::default();
    // A long cloak off its shoulders, torn to rags at the hem.
    // (Hung from the back of its shoulders and wider than it, so it streams
    // out behind it: a train of black rags.)
    drapes.own(Arc::new(ClothDef { color: (44, 44, 58), lining: (70, 26, 32), cols: 9, rows: 16, gap: 2.7, pin: (-17.0, -5.0, 12.0), torn: 0.6, weight: 1.2, front: false, behind: 230.0 }), seed);
    // Rags off each arm, past the gauntlets.
    for (n, x) in [(1u64, 10.0f32), (2, -11.0)] {
        drapes.own(Arc::new(ClothDef { color: (52, 50, 62), lining: (60, 34, 40), cols: 2, rows: 12, gap: 2.2, pin: (x - 1.5, x + 1.5, 6.0), torn: 0.65, weight: 0.8, front: false, behind: 90.0 }), seed ^ n);
    }
    commands.entity(e).insert(drapes);
}

fn solid(sim: &SimWorld, p: Vec2) -> bool {
    sim.world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| !c.is_air() && matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder))
}

/// The ball on its chain, a tick: it falls and swings, held at most the
/// chain's length from the hand, rests and drags on the ground.
fn hang_ball(sim: &SimWorld, d: &mut Dread, reach: f32) {
    let mut v = (d.ball - d.ball_prev) * 0.985;
    let on_ground = solid(sim, d.ball - Vec2::Y * (BALL + 1.0));
    if on_ground {
        v.x *= 0.85;
    }
    d.ball_prev = d.ball;
    d.ball += v - Vec2::Y * BALL_FALL * DT * DT;
    let off = d.ball - d.hand;
    if off.length() > reach {
        d.ball = d.hand + off.normalize() * reach;
    }
    for _ in 0..8 {
        if solid(sim, d.ball - Vec2::Y * BALL) {
            d.ball.y += 1.0;
        } else {
            break;
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn think(
    mut sim: ResMut<SimWorld>,
    mut q: Query<(Entity, &mut Dread, &Kinematics, &mut Controls, &mut crate::creatures::body::animation::Animator)>,
    hunted: Query<(Entity, &Kinematics, &Team), Without<Dread>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    (mut sparks, mut trauma): (ResMut<crate::vfx::Sparks>, ResMut<crate::fx::Trauma>),
) {
    for (_e, mut d, k, mut c, mut anim) in &mut q {
        let (pos, half, facing) = (k.body.pos, k.body.half, k.loco.facing);
        let feet = pos.y - half.y;
        d.hand = hand_at(pos, facing, d.doing);
        if !d.started {
            d.started = true;
            d.ball = d.hand + Vec2::new(-facing * 20.0, -20.0);
            d.ball_prev = d.ball;
            d.chains = vec![Links::new(6, 2.6), Links::new(5, 2.6), Links::new(7, 2.6)];
        }
        d.slam_ready -= DT;
        d.sweep_ready -= DT;
        let target = hunted.iter().filter(|(_, _, t)| t.hunted()).map(|(e, tk, _)| (e, tk.body.pos, tk.body.half)).filter(|(_, p, _)| p.distance(pos) < AGGRO).min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)));
        if target.is_some() && !d.shrieked {
            d.shrieked = true;
            sounds.write(crate::sound::PlaySound::at("snarl", pos).pitch(1.7).volume(1.0));
            sounds.write(crate::sound::PlaySound::at("groan", pos).pitch(1.5).volume(0.9));
        }
        let doing = d.doing;
        let mut next = doing;
        match doing {
            Doing::Carry => {
                anim.force = None;
                hang_ball(&sim, &mut d, CHAIN);
                if let Some((_, tp, _)) = target {
                    let off = tp - pos;
                    if off.x.abs() < SWEEP_NEAR.0 && off.y.abs() < SWEEP_NEAR.1 && d.sweep_ready <= 0.0 {
                        let from = d.ball;
                        next = Doing::Draw(0.0, from);
                        sounds.write(crate::sound::PlaySound::at("clank", d.ball).pitch(0.5));
                    } else if tp.distance(d.hand) < CHAIN + SLAM_FROM && d.slam_ready <= 0.0 && k.loco.grounded() {
                        next = Doing::Whirl(0.0);
                    }
                }
            }
            Doing::Whirl(t) => {
                anim.play("raise");
                let spin = t * (6.0 + 12.0 * t / WHIRL.0) * facing;
                d.ball_prev = d.ball;
                d.ball = d.hand + Vec2::from_angle(spin + std::f32::consts::FRAC_PI_2) * WHIRL.1 * (t / 0.25).min(1.0);
                let ticks = (t / 0.2) as u32;
                if ((t + DT) / 0.2) as u32 != ticks {
                    sounds.write(crate::sound::PlaySound::at("swing_heavy", d.ball).pitch(0.5 + t * 0.3));
                }
                next = if t + DT >= WHIRL.0 {
                    // Where it comes down: where they stand, no further than
                    // its chain reaches, on the ground there.
                    let mut to = target.map_or(d.hand + Vec2::new(facing * CHAIN, -CHAIN), |(_, tp, th)| tp - Vec2::Y * (th.y - BALL));
                    if to.distance(d.hand) > CHAIN {
                        to = d.hand + (to - d.hand).normalize() * CHAIN;
                    }
                    if let Some(drop) = (0..60).map(|n| to - Vec2::Y * n as f32).find(|&p| solid(&sim, p - Vec2::Y * BALL)) {
                        to = drop;
                    }
                    Doing::Slam(0.0, d.ball, to)
                } else {
                    Doing::Whirl(t + DT)
                };
            }
            Doing::Slam(t, from, to) => {
                anim.play("slam");
                let u = ((t + DT) / SLAM.0).min(1.0).powf(1.6);
                d.ball_prev = d.ball;
                d.ball = from.lerp(to, u) + Vec2::Y * SLAM.1 * 4.0 * u * (1.0 - u);
                next = if u >= 1.0 {
                    // It comes down: the ground's broken, what's near thrown.
                    for (e, tk, t) in &hunted {
                        if !t.hunted() {
                            continue;
                        }
                        let near = ((tk.body.pos - to).abs() - tk.body.half).max(Vec2::ZERO).length();
                        if near <= QUAKE.0 {
                            let away = Vec2::new((tk.body.pos.x - to.x).signum(), 0.9).normalize();
                            let f = 1.0 - near / QUAKE.0 * 0.5;
                            hits.write(crate::combat::Hit { target: e, damage: QUAKE.1 * f, harm: Harm::Blunt, knock: away * QUAKE.2, stun: 0.35, at: to, dir: away, weight: 2.2, crit: false });
                        }
                    }
                    sim.queue(platypus_sim::WorldEdit::Explode { center: CellPos::from_world(to.x, to.y - BALL), radius: CRATER.0, power: CRATER.1 });
                    let dust = crate::magic::runes::Emitter {
                        count: 0.0,
                        life: (0.4, 1.1),
                        colors: vec![(170, 160, 140), (120, 112, 100), (80, 74, 68)],
                        speed: 220.0,
                        spread: 1.3,
                        gravity: 260.0,
                        drag: 2.0,
                        size: 1.5,
                        jitter: 0.0,
                        glow: false,
                    };
                    sparks.emit(&dust, 120, to - Vec2::Y * BALL, Vec2::Y, Vec2::ZERO);
                    trauma.0 = (trauma.0 + 0.6).min(1.0);
                    sounds.write(crate::sound::PlaySound::at("slam", to).volume(1.0));
                    sounds.write(crate::sound::PlaySound::at("boom", to).pitch(0.55).volume(0.7));
                    Doing::Rest(0.0)
                } else {
                    Doing::Slam(t + DT, from, to)
                };
            }
            Doing::Rest(t) => {
                anim.play("slam");
                d.ball_prev = d.ball;
                next = if t + DT >= REST {
                    d.slam_ready = SLAM_EVERY;
                    Doing::Carry
                } else {
                    Doing::Rest(t + DT)
                };
            }
            Doing::Draw(t, from) => {
                anim.play("sweep");
                let back = Vec2::new(d.hand.x - facing * CHAIN * 0.8, feet + BALL);
                d.ball_prev = d.ball;
                d.ball = from.lerp(back, ((t + DT) / DRAW).min(1.0));
                next = if t + DT >= DRAW {
                    sounds.write(crate::sound::PlaySound::at("swing_heavy", d.hand).pitch(0.45));
                    d.struck.clear();
                    Doing::Sweep(0.0)
                } else {
                    Doing::Draw(t + DT, from)
                };
            }
            Doing::Sweep(t) => {
                anim.play("sweep");
                let u = ((t + DT) / SWEEP.0).min(1.0);
                let a = (SWEEP.1 + (SWEEP.2 - SWEEP.1) * u).to_radians();
                let mut at = d.hand + Vec2::new(a.cos() * facing, a.sin()) * CHAIN * 0.9;
                at.y = at.y.max(feet + BALL);
                d.ball_prev = d.ball;
                d.ball = at;
                for (e, tk, t) in &hunted {
                    if t.hunted() && !d.struck.contains(&e) && ((tk.body.pos - at).abs() - tk.body.half).max_element() <= BALL {
                        d.struck.push(e);
                        let dir = Vec2::new(facing, 0.35).normalize();
                        hits.write(crate::combat::Hit { target: e, damage: SWEEP_HIT.0, harm: Harm::Blunt, knock: dir * SWEEP_HIT.1, stun: 0.3, at, dir, weight: 1.8, crit: false });
                        sounds.write(crate::sound::PlaySound::at("clank", at).pitch(0.6).volume(1.0));
                    }
                }
                next = if u >= 1.0 {
                    d.sweep_ready = SWEEP_EVERY;
                    Doing::Carry
                } else {
                    Doing::Sweep(t + DT)
                };
            }
        }
        // Swung fast (dragged, flung), it hurts what it hits.
        if matches!(next, Doing::Carry) {
            let speed = (d.ball - d.ball_prev).length() / DT;
            if speed > FAST.0 {
                for (e, tk, t) in &hunted {
                    if t.hunted() && ((tk.body.pos - d.ball).abs() - tk.body.half).max_element() <= BALL {
                        let dir = (tk.body.pos - d.ball).normalize_or(Vec2::Y);
                        hits.write(crate::combat::Hit { target: e, damage: FAST.1, harm: Harm::Blunt, knock: dir * 220.0, stun: 0.2, at: d.ball, dir, weight: 1.2, crit: false });
                    }
                }
            }
        }
        d.doing = next;
        // (Busy with the flail, it stands.)
        if !matches!(next, Doing::Carry) {
            c.0.move_x = 0.0;
            c.0.jump = false;
        }
        // Its chains: at its belt (two) and its other wrist.
        let tops = [pos + Vec2::new(-4.0 * facing, -2.0), pos + Vec2::new(3.0 * facing, -2.0), pos + Vec2::new(-10.0 * facing, -7.0)];
        for (l, top) in d.chains.iter_mut().zip(tops) {
            l.swing(top, 300.0);
        }
    }
}

/// The flail and the chains, in front of the player, behind it.
#[derive(Resource)]
pub struct FlailCanvas(Canvas);

impl Default for FlailCanvas {
    fn default() -> Self {
        FlailCanvas(Canvas::new("Flails", 11.15))
    }
}

/// The ball (iron, a highlight, spikes), its chain from the hand (sagging
/// when slack), the chains it wears.
fn draw(mut commands: Commands, mut canvas: ResMut<FlailCanvas>, mut images: ResMut<Assets<Image>>, camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>, q: Query<&Dread>, mut sprites: CanvasSprites) {
    let Ok((cam, loader)) = camera.single() else { return };
    let any = q.iter().any(|d| d.started);
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, any) else { return };
    let (light, dark) = ([150, 152, 164, 255], [70, 72, 82, 255]);
    for d in &q {
        if !d.started {
            continue;
        }
        for l in &d.chains {
            draw_links(&mut px, l.points(), light, dark);
        }
        // The chain: sagging with the slack.
        let slack = (CHAIN - d.hand.distance(d.ball)).max(0.0);
        let mid = (d.hand + d.ball) / 2.0 - Vec2::Y * (slack * 0.6).min(18.0);
        let pts: Vec<Vec2> = (0..=CHAIN_LINKS).map(|i| {
            let t = i as f32 / CHAIN_LINKS as f32;
            d.hand.lerp(mid, t).lerp(mid.lerp(d.ball, t), t)
        }).collect();
        draw_links(&mut px, &pts, light, dark);
        // The ball: spikes, then iron, lit from above.
        let c = d.ball;
        for s in 0..10 {
            let a = s as f32 / 10.0 * std::f32::consts::TAU + 0.3;
            for r in 0..4 {
                let p = (c + Vec2::from_angle(a) * (BALL + r as f32)).floor().as_ivec2();
                px.put(p.x, p.y, if r < 2 { [110, 112, 124, 255] } else { [180, 182, 196, 255] });
            }
        }
        let r = BALL.ceil() as i32;
        for y in -r..=r {
            for x in -r..=r {
                let o = Vec2::new(x as f32, y as f32);
                if o.length() <= BALL {
                    let lit = (o - Vec2::new(-1.5, 2.0)).length() < BALL * 0.45;
                    let col = if lit { [120, 122, 136, 255] } else if o.length() > BALL - 1.0 { [30, 30, 36, 255] } else { [62, 64, 74, 255] };
                    let p = (c + o).floor().as_ivec2();
                    px.put(p.x, p.y, col);
                }
            }
        }
    }
}

/// Dead: its cloak, to wear.
fn on_death(mut commands: Commands, mut died: MessageReader<Died>, items: Option<Res<crate::hands::items::Items>>) {
    for d in died.read() {
        if d.kind != "dread_knight" {
            continue;
        }
        if let Some(items) = items.as_ref()
            && let Some(id) = items.id("dread_cloak")
        {
            crate::hands::spawn_drop(&mut commands, items, d.body.pos, crate::hands::items::Stack::new(id, 1));
        }
    }
}
