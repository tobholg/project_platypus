//! Void magic (DESIGN §7b): space itself.
//!
//! - Blink (`Payload::Blink`): the caster is where the spell lands (beside
//!   the wall, if it hit one), moving as they were.
//! - Portals (`Payload::Portal`): each cast opens one on what the spell hit,
//!   facing out of it: the first of a caster's pair, then the second, then
//!   the older is replaced. Whatever goes into one comes out of the other,
//!   its speed turned to the exit's facing (fall into a floor portal, fly
//!   out of a wall's): bodies (you, creatures, chests, bodies, bombs),
//!   spells in flight, and the liquids and sand that press into its mouth
//!   (drain a lake onto a crypt). A portal closes after a minute.
//! - Stasis (`Payload::Stasis`): a bubble where it lands, for a few
//!   seconds: bodies and spells inside stop where they are, and go on as
//!   they were when it bursts (freeze a fireball, walk round it).

use std::collections::HashMap;

use bevy::prelude::*;
use platypus_physics::{Grid, Occupancy};
use platypus_sim::rng::Rng;
use platypus_sim::{CellPos, Kind, Particle};

use super::Spell;
use crate::actors::{Kinematics, WorldGrid};
use crate::camera::MainCamera;
use crate::canvas::{Canvas, CanvasSprite, CanvasSprites};
use crate::light::LightSource;
use crate::vfx::Sparks;
use crate::world::{ChunkLoader, SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// Half a portal's length along the surface (cells).
const MOUTH: f32 = 9.0;
/// Seconds a portal stays open.
const PORTAL_LIFE: f32 = 60.0;
/// Seconds before something that went through can go through again.
const COOLDOWN: f32 = 0.25;
/// Cells of liquid or sand a portal takes in a tick, at most.
const POUR: usize = 28;
/// How fast what pours out leaves (cells a tick).
const POUR_SPEED: f32 = 1.6;
/// How far back from where a blink lands it looks for room (cells).
const BLINK_BACK: i32 = 40;
/// Portal colours: the first of a pair, the second.
const COLORS: [(u8, u8, u8); 2] = [(90, 170, 255), (255, 150, 50)];
/// Above the light overlay (as beams are).
const Z_VOID: f32 = 15.7;
/// How far a portal's oval stands out of its surface (cells).
const DEPTH: i32 = 6;

/// What void spells did this tick (from `fly`), carried out after it.
pub enum Act {
    Blink { caster: Entity, at: Vec2, dir: Vec2 },
    Portal { caster: Entity, at: Vec2, normal: Vec2 },
    Stasis { at: Vec2, radius: f32, secs: f32 },
}

#[derive(Resource, Default)]
pub struct Acts(pub Vec<Act>);

/// A portal: where, which way it faces, whose (and which of their pair).
#[derive(Component)]
pub struct Portal {
    pub owner: Entity,
    pub at: Vec2,
    pub normal: Vec2,
    pub which: usize,
    left: f32,
}

impl Portal {
    fn tangent(&self) -> Vec2 {
        self.normal.perp()
    }

    /// Where a point is, from this portal: along its mouth, and out from it.
    fn local(&self, p: Vec2) -> (f32, f32) {
        let d = p - self.at;
        (d.dot(self.tangent()), d.dot(self.normal))
    }
}

/// Each caster's pair: the portals open (by which), and which opens next.
#[derive(Resource, Default)]
pub struct Pairs(HashMap<Entity, ([Option<Entity>; 2], usize)>);

/// A stasis bubble.
#[derive(Component)]
pub struct Stasis {
    pub at: Vec2,
    pub radius: f32,
    left: f32,
}

/// A body held in stasis: where it's held, and how it was moving.
#[derive(Component)]
pub struct Held {
    pos: Vec2,
    vel: Vec2,
}

/// Things that just went through a portal (seconds before they can again).
#[derive(Resource, Default)]
pub struct Recent(HashMap<Entity, f32>);

/// Room for a box of half size `half` at `at`: nothing solid in it.
fn fits(grid: &WorldGrid, at: Vec2, half: Vec2) -> bool {
    let (lo, hi) = ((at - half).floor().as_ivec2(), (at + half - Vec2::splat(0.01)).floor().as_ivec2());
    (lo.y..=hi.y).all(|y| (lo.x..=hi.x).all(|x| matches!(grid.occupancy(x, y), Occupancy::Empty | Occupancy::Liquid)))
}

/// Carry out the void spells that landed this tick.
#[allow(clippy::too_many_arguments)]
pub fn act(
    mut commands: Commands,
    sim: Res<SimWorld>,
    mut acts: ResMut<Acts>,
    mut pairs: ResMut<Pairs>,
    mut sparks: ResMut<Sparks>,
    mut bodies: Query<&mut Kinematics>,
    portals: Query<&Portal>,
) {
    let grid = WorldGrid(&sim.world);
    for a in std::mem::take(&mut acts.0) {
        match a {
            Act::Blink { caster, at, dir } => {
                let Ok(mut k) = bodies.get_mut(caster) else { continue };
                let half = k.body.half;
                // Back along the way it came until there's room (standing up
                // off what it landed on, if need be).
                let back = dir.normalize_or(Vec2::X);
                let lift = (half.y * 2.0 + 2.0) as i32;
                let Some(to) = (0..BLINK_BACK).flat_map(|i| (0..=lift).map(move |l| at - back * (half.max_element() * 0.5 + i as f32) + Vec2::Y * l as f32)).find(|&p| fits(&grid, p, half)) else { continue };
                sparks.emit(&BLINK, BLINK.count as usize, k.body.pos, Vec2::Y, Vec2::ZERO);
                sparks.emit(&BLINK, BLINK.count as usize, to, Vec2::Y, Vec2::ZERO);
                k.body.pos = to;
                k.prev_pos = to;
            }
            Act::Portal { caster, at, normal } => {
                let (open, next) = pairs.0.entry(caster).or_default();
                let which = *next;
                if let Some(old) = open[which].take()
                    && portals.get(old).is_ok()
                {
                    commands.entity(old).despawn();
                }
                let (r, g, b) = COLORS[which];
                let e = commands
                    .spawn((
                        Name::new("Portal"),
                        Portal { owner: caster, at: at + normal * 0.5, normal, which, left: PORTAL_LIFE },
                        LightSource { color: [r as f32 / 255.0 * 1.6, g as f32 / 255.0 * 1.6, b as f32 / 255.0 * 1.6], flicker: 0.1 },
                        Transform::from_translation((at + normal * 3.0).extend(0.0)),
                    ))
                    .id();
                open[which] = Some(e);
                *next = 1 - which;
                sparks.emit(&BLINK, BLINK.count as usize, at, normal, Vec2::ZERO);
            }
            Act::Stasis { at, radius, secs } => {
                commands.spawn((Name::new("Stasis"), Stasis { at, radius, left: secs }, LightSource { color: [0.5, 0.35, 0.9], flicker: 0.05 }, Transform::from_translation(at.extend(0.0))));
            }
        }
    }
}

/// Portals age and close; a pair whose portal closed opens that one next.
pub fn age(mut commands: Commands, mut pairs: ResMut<Pairs>, mut portals: Query<(Entity, &mut Portal)>, mut recent: ResMut<Recent>) {
    for (e, mut p) in &mut portals {
        p.left -= DT;
        if p.left <= 0.0 {
            commands.entity(e).despawn();
            if let Some((open, next)) = pairs.0.get_mut(&p.owner) {
                open[p.which] = None;
                *next = p.which;
            }
        }
    }
    recent.0.retain(|_, t| {
        *t -= DT;
        *t > 0.0
    });
}

/// The other of a portal's pair, if it's open.
fn twin<'a>(pairs: &Pairs, portals: &'a Query<(Entity, &Portal)>, p: &Portal) -> Option<&'a Portal> {
    let (open, _) = pairs.0.get(&p.owner)?;
    open[1 - p.which].and_then(|e| portals.get(e).ok()).map(|(_, q)| q)
}

/// Through a portal: from `from`'s mouth (along, out) with velocity `v`, to
/// `to`'s: where, and moving how (into `from` becomes out of `to`).
fn carry(from: &Portal, to: &Portal, along: f32, v: Vec2, clearance: f32) -> (Vec2, Vec2) {
    let speed_in = -v.dot(from.normal);
    let slide = v.dot(from.tangent());
    let pos = to.at - to.tangent() * along + to.normal * clearance;
    let vel = to.normal * speed_in - to.tangent() * slide;
    (pos, vel)
}

type Traveller<'a> = (Entity, &'a mut Kinematics, Has<Held>);

/// Bodies, spells, and liquids and sand pressing into a portal come out of
/// its twin.
#[allow(clippy::too_many_arguments)]
pub fn through(
    mut sim: ResMut<SimWorld>,
    pairs: Res<Pairs>,
    mut recent: ResMut<Recent>,
    mut sparks: ResMut<Sparks>,
    portals: Query<(Entity, &Portal)>,
    mut bodies: Query<Traveller>,
    mut spells: Query<(Entity, &mut Spell)>,
) {
    if portals.is_empty() {
        return;
    }
    let pairs = &*pairs;
    // (What goes through, and where it comes out: moved after looking, so a
    // body can't go through twice in a tick.)
    let mut moves: Vec<(Entity, Vec2, Vec2)> = Vec::new();
    {
        let grid = WorldGrid(&sim.world);
        for (_, a) in &portals {
            let Some(b) = twin(pairs, &portals, a) else { continue };
            for (e, k, held) in &bodies {
                if held || recent.0.contains_key(&e) {
                    continue;
                }
                let (along, out) = a.local(k.body.pos);
                let half = k.body.half;
                let depth = (half.x * a.normal.x).abs() + (half.y * a.normal.y).abs();
                if along.abs() > MOUTH || out > depth + 2.5 || out < -2.0 || k.body.vel.dot(a.normal) >= 0.0 {
                    continue;
                }
                let clearance = (half.x * b.normal.x).abs() + (half.y * b.normal.y).abs() + 1.5;
                let (pos, vel) = carry(a, b, along.clamp(-MOUTH + 2.0, MOUTH - 2.0), k.body.vel, clearance);
                if fits(&grid, pos, half) {
                    moves.push((e, pos, vel));
                }
            }
        }
    }
    for (e, pos, vel) in moves {
        let Ok((_, mut k, _)) = bodies.get_mut(e) else { continue };
        sparks.emit(&BLINK, 12, k.body.pos, Vec2::Y, Vec2::ZERO);
        k.body.pos = pos;
        k.prev_pos = pos;
        k.body.vel = vel;
        recent.0.insert(e, COOLDOWN);
    }
    // Spells in flight.
    for (e, mut s) in &mut spells {
        if recent.0.contains_key(&e) {
            continue;
        }
        for (_, a) in &portals {
            let Some(b) = twin(pairs, &portals, a) else { continue };
            let (along, out) = a.local(s.pos);
            if along.abs() > MOUTH || !(-1.0..3.0).contains(&out) || s.vel.dot(a.normal) >= 0.0 {
                continue;
            }
            let (pos, vel) = carry(a, b, along, s.vel, 2.5);
            s.pos = pos;
            s.prev = pos;
            s.vel = vel;
            recent.0.insert(e, COOLDOWN);
            break;
        }
    }
    // Liquids and sand pressing into a mouth pour out of the twin.
    let world = &mut sim.world;
    let mats = world.materials().clone();
    for (pe, a) in &portals {
        let Some(b) = twin(pairs, &portals, a) else { continue };
        let mut rng = Rng::seeded(&[world.tick(), pe.to_bits(), 0x9087]);
        let mut poured = 0;
        'mouth: for i in -(MOUTH as i32)..=(MOUTH as i32) {
            for o in [0.5f32, 1.5, 2.5] {
                let p = a.at + a.tangent() * i as f32 + a.normal * o;
                let q = CellPos::from_world(p.x, p.y);
                let Some(c) = world.get(q) else { continue };
                if c.is_air() || !matches!(mats.phys(c.material).kind, Kind::Liquid | Kind::Powder) {
                    continue;
                }
                let Some(c) = world.pluck(q) else { continue };
                let jiggle = (rng.next_u32() as f32 / u32::MAX as f32 - 0.5) * 0.4;
                let at = b.at - b.tangent() * i as f32 + b.normal * 1.5;
                let v = b.normal * POUR_SPEED + b.tangent() * jiggle;
                world.emit(Particle::new([at.x, at.y], [v.x, v.y], c, 120, platypus_sim::Landing::Settle));
                poured += 1;
                if poured >= POUR {
                    break 'mouth;
                }
            }
        }
    }
}

/// Stasis: what's inside a bubble is held where it is (spells stop too:
/// `fly` skips them); when the bubble bursts it goes on as it was.
#[allow(clippy::too_many_arguments)]
pub fn stasis(
    mut commands: Commands,
    mut bubbles: Query<(Entity, &mut Stasis)>,
    mut bodies: Query<(Entity, &mut Kinematics, Option<&Held>)>,
    mut spells: Query<&mut Spell>,
    mut sparks: ResMut<Sparks>,
) {
    let mut live = Vec::new();
    for (e, mut s) in &mut bubbles {
        s.left -= DT;
        if s.left <= 0.0 {
            commands.entity(e).despawn();
            sparks.emit(&BLINK, 40, s.at, Vec2::Y, Vec2::ZERO);
        } else {
            live.push((s.at, s.radius));
        }
    }
    let inside = |p: Vec2| live.iter().any(|(at, r)| p.distance(*at) <= *r);
    for (e, mut k, held) in &mut bodies {
        match (held, inside(k.body.pos)) {
            (None, true) => {
                commands.entity(e).try_insert(Held { pos: k.body.pos, vel: k.body.vel });
                k.body.vel = Vec2::ZERO;
            }
            (Some(h), true) => {
                let k = &mut *k;
                k.body.pos = h.pos;
                k.prev_pos = h.pos;
                k.body.vel = Vec2::ZERO;
                // (No control while held: a creature can't walk off.)
                k.loco.knock(&mut k.body, Vec2::ZERO, DT * 2.0);
            }
            (Some(h), false) => {
                // Let go: as it was.
                k.body.vel = h.vel;
                commands.entity(e).remove::<Held>();
            }
            (None, false) => {}
        }
    }
    for mut s in &mut spells {
        s.held = inside(s.pos);
    }
}

/// Pin held bodies after everything has moved this tick (movement ran on
/// them anyway).
pub fn pin(mut bodies: Query<(&mut Kinematics, &Held)>) {
    for (mut k, h) in &mut bodies {
        k.body.pos = h.pos;
        k.prev_pos = h.pos;
        k.body.vel = Vec2::ZERO;
    }
}

#[derive(Resource)]
pub struct VoidCanvas(Canvas);

impl Default for VoidCanvas {
    fn default() -> Self {
        VoidCanvas(Canvas::new("Void", Z_VOID))
    }
}

/// Portals (a swirling ring along the surface) and stasis bubbles, a cell
/// at a time.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    mut commands: Commands,
    time: Res<Time>,
    mut canvas: ResMut<VoidCanvas>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<(&GlobalTransform, &ChunkLoader), With<MainCamera>>,
    portals: Query<&Portal>,
    bubbles: Query<&Stasis>,
    mut sprites: CanvasSprites,
    held: Query<&Transform, (With<Held>, Without<CanvasSprite>)>,
) {
    let Ok((cam, loader)) = camera.single() else { return };
    let any = !portals.is_empty() || !bubbles.is_empty();
    let Some(mut px) = canvas.0.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, any) else { return };
    let t = time.elapsed_secs();
    for p in &portals {
        let (r, g, b) = COLORS[p.which];
        let (tan, n) = (p.tangent(), p.normal);
        // An oval standing out of the surface: a bright rim, a dark heart
        // with its colour turning in it.
        let pale = [r, g, b].map(|c| ((c as u16 + 255) / 2) as u8);
        for i in -(MOUTH as i32 + 1)..=(MOUTH as i32 + 1) {
            for o in 0..=DEPTH {
                let (u, v) = (i as f32 / (MOUTH + 1.0), (o as f32 - DEPTH as f32 / 2.0) / (DEPTH as f32 / 2.0 + 0.5));
                let d = u * u + v * v;
                if d > 1.0 {
                    continue;
                }
                let at = p.at + tan * i as f32 + n * (o as f32 - 0.5);
                let c = at.floor().as_ivec2();
                let swirl = ((u * 5.0 - t * 4.0 + v * 3.0).sin() * 0.5 + 0.5) * (1.0 - d * 0.6);
                let col = if d > 0.55 {
                    // The rim: pale, flickering to its colour.
                    if (u * 9.0 + t * 7.0).sin() > 0.3 { [pale[0], pale[1], pale[2], 255] } else { [r, g, b, 255] }
                } else {
                    [(r as f32 * swirl) as u8 / 2 + 12, (g as f32 * swirl) as u8 / 2 + 8, (b as f32 * swirl) as u8 / 2 + 24, 235]
                };
                px.put(c.x, c.y, col);
            }
        }
    }
    for s in &bubbles {
        // A thin shimmering shell and a faint violet haze.
        let n = (s.radius * 6.0) as i32;
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU + t * 0.6;
            let wobble = (a * 5.0 + t * 3.0).sin() * 0.6;
            let c = (s.at + Vec2::from_angle(a) * (s.radius + wobble)).floor().as_ivec2();
            let bright = (((a * 3.0 - t * 2.0).sin() * 0.5 + 0.5) * 120.0) as u8;
            px.put(c.x, c.y, [150 + bright / 2, 110 + bright / 3, 255, 200]);
        }
        let r = s.radius as i32;
        for y in -r..=r {
            for x in -r..=r {
                if !((x * 7 + y * 13) as u32).wrapping_mul(2654435761).is_multiple_of(9) || x * x + y * y > r * r {
                    continue;
                }
                let c = (s.at + Vec2::new(x as f32, y as f32)).floor().as_ivec2();
                px.put(c.x, c.y, [120, 90, 220, 60]);
            }
        }
    }
    // What's held glints.
    for tf in &held {
        let c = tf.translation.truncate().floor().as_ivec2();
        px.put(c.x, c.y + 9, [200, 180, 255, 200]);
    }
}

/// A blink's (and a portal's) flash of violet.
static BLINK: std::sync::LazyLock<super::runes::Emitter> = std::sync::LazyLock::new(|| super::runes::Emitter {
    count: 30.0,
    life: (0.2, 0.6),
    colors: vec![(240, 220, 255), (170, 120, 255), (80, 40, 160)],
    speed: 70.0,
    spread: std::f32::consts::PI,
    gravity: 0.0,
    drag: 3.0,
    size: 1.2,
    jitter: 0.0,
    glow: true,
});

#[cfg(test)]
mod tests {
    use super::*;

    fn portal(at: Vec2, normal: Vec2, which: usize) -> Portal {
        Portal { owner: Entity::PLACEHOLDER, at, normal, which, left: PORTAL_LIFE }
    }

    /// Falling into a floor portal, out of a wall's: the fall's speed
    /// becomes speed out of the wall; sliding along one is sliding along
    /// the other.
    #[test]
    fn a_portal_turns_speed_to_face_out() {
        let floor = portal(Vec2::new(0.0, 0.0), Vec2::Y, 0);
        let wall = portal(Vec2::new(100.0, 50.0), Vec2::NEG_X, 1);
        let (pos, vel) = carry(&floor, &wall, 0.0, Vec2::new(0.0, -300.0), 4.0);
        assert!((pos - Vec2::new(96.0, 50.0)).length() < 1e-3, "out in front of the wall: {pos}");
        assert!((vel - Vec2::new(-300.0, 0.0)).length() < 1e-3, "out of the wall at the fall's speed: {vel}");
        // Speed is kept, whatever the angle.
        let v = Vec2::new(120.0, -250.0);
        let (_, out) = carry(&floor, &wall, 2.0, v, 4.0);
        assert!((out.length() - v.length()).abs() < 1e-3);
        assert!(out.dot(wall.normal) > 0.0, "it leaves the wall");
        // Two floor portals: down into one is up out of the other.
        let other = portal(Vec2::new(200.0, 0.0), Vec2::Y, 1);
        let (_, up) = carry(&floor, &other, 0.0, Vec2::new(0.0, -200.0), 4.0);
        assert!((up - Vec2::new(0.0, 200.0)).length() < 1e-3);
    }
}
