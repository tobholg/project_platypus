//! Procedural legs: legs that reach out and grip the world, Noita-style,
//! and the body they carry. Two ways of seeing it (`view`):
//!
//! - **Above** (spiders): a body seen from above, turned to where it's
//!   going, its legs splayed all round it. A creature file's `legs` gives
//!   it: `count` legs spread round its body
//!   (each with a way it prefers to reach, fanned front to back on both
//!   sides), `reach` (a leg's full length: thigh `upper` of it, the rest
//!   shin), how long a step takes and how high a foot lifts, colours, and
//!   the `body` sprite (from above, pointing right, a `grip` anchor where
//!   the legs meet), turned with RotSprite to where the body heads.
//!
//!   Each foot holds on to a real solid cell: found by casting from the hip
//!   along the leg's way, then swept round it (±90°) until something solid
//!   is met within reach. A foot stays put while the body moves; once it's
//!   stretched too far, crowded or twisted too far from its way it lifts and
//!   steps to a new hold (an arc away from what it holds), a few at a time,
//!   never two neighbours at once. With nothing in reach a leg hangs curled.
//!   The knees (two-bone IK) bend away from what the feet hold. Legs are
//!   drawn a cell at a time (Bresenham), so they stay pixel art, behind the
//!   body.
//! - **Side** (walkers on legs: the stilt stalker, crabs): a body seen
//!   from the side (`body`, pointing right, its `grip` where the hips are
//!   measured from), each leg in `each` with its hip on it, where its foot
//!   rests (`lean`: ahead of the hip, or behind), its gait (legs of one
//!   group step together, never while another group's are stepping) and
//!   whether it's the far side's (drawn behind the body, darker; the near
//!   ones in front). A foot plants on the ground under where it rests,
//!   a little ahead as it goes, and steps when it's left a `stride`
//!   behind; the body rides `ride` cells over its planted feet and tilts
//!   with them (front feet higher: nose up), so it follows the ground.
//!
//! Looks only: the body's movement is its own box's (`cling` to climb);
//! its box takes in the legs.

use std::collections::HashMap;

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind};
use serde::Deserialize;

use crate::creatures::body::animation::{Animator, CreatureSprite};
use crate::creatures::Kinematics;
use crate::world::SimWorld;

pub struct LegsPlugin;

impl Plugin for LegsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LegCanvas>()
            .init_resource::<Footfalls>()
            .init_resource::<crate::canvas::StageView>()
            .init_resource::<BodyArt>()
            .add_systems(Update, (grow_legs, aims, walk, footfalls, draw).chain().after(crate::creatures::body::animation::animate).after(TransformSystems::Propagate));
    }
}

/// Which way a legged body is seen.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
pub enum View {
    /// From above (a spider): turned to where it's going.
    #[default]
    Above,
    /// From the side: facing where it goes, tilted with the ground.
    Side,
}

/// One leg of a body seen from the side.
#[derive(Clone, Debug, Deserialize)]
pub struct LegDef {
    /// Its hip, from the body's `grip` (cells, facing right, y up).
    pub hip: (f32, f32),
    /// Where its foot rests, ahead of its hip (cells; behind: negative).
    #[serde(default)]
    pub lean: f32,
    /// Its gait group: a group's legs step together.
    #[serde(default)]
    pub gait: u8,
    /// On the far side: behind the body, darker.
    #[serde(default)]
    pub far: bool,
    /// Which way its knee bends.
    #[serde(default)]
    pub knee: Knee,
}

/// How a leg is drawn.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
pub enum LegStyle {
    /// Flesh: tapered, round (a raptor's, a tyrant's, a crab's).
    #[default]
    Flesh,
    /// Metal: straight plates as wide as `width` says at their start,
    /// shaded under, bolts at the joints, a piston from thigh to shank,
    /// flat foot plates (`toes`: their length) (a walker's).
    Plate,
}

/// Which way a side-view leg's knee bends.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
pub enum Knee {
    /// Up and out from the body: a front leg's forward, a back one's back
    /// (insects, crabs: the knee high).
    #[default]
    Out,
    /// Forward, whichever leg (a person's).
    Forward,
    /// Back (a bird's or a raptor's: what bends is its ankle).
    Back,
    /// Up, or down (an arm's elbow: a crab's claw held up from below).
    Up,
    Down,
}

/// What a foot coming down does (a heavy one): a sound of its own (and
/// the ground's footstep under it), a puff of dust the colour of the
/// ground, the screen shaken (`shake`: trauma at its foot, less further
/// off, none past 400 cells).
#[derive(Clone, Debug, Deserialize)]
pub struct Footfall {
    #[serde(default)]
    pub sound: Option<String>,
    #[serde(default = "one_f")]
    pub volume: f32,
    #[serde(default = "yes")]
    pub ground: bool,
    #[serde(default)]
    pub dust: u32,
    #[serde(default)]
    pub shake: f32,
}

fn one_f() -> f32 {
    1.0
}
fn yes() -> bool {
    true
}

/// Feet that came down this frame (`walk` notes them, `footfalls` sounds
/// them: it changes the world).
#[derive(Resource, Default)]
struct Footfalls(Vec<(Vec2, Footfall)>);

/// An arm: a limb that doesn't walk, held out from the body (a crab's
/// claws, a raptor's little arms), its hand resting where it says,
/// swaying a little; a `claw` sprite at its end (pointing right, its
/// `grip` at the wrist), turned to the forearm.
#[derive(Clone, Debug, Deserialize)]
pub struct ArmDef {
    /// Its shoulder and where its hand rests, from the body's `grip`
    /// (cells, facing right, y up).
    pub shoulder: (f32, f32),
    pub hand: (f32, f32),
    /// Its upper arm and forearm (cells).
    pub bones: (f32, f32),
    /// How thick at the shoulder, the elbow and the wrist (cells).
    #[serde(default)]
    pub width: Vec<f32>,
    #[serde(default = "elbow")]
    pub elbow: Knee,
    #[serde(default)]
    pub claw: Option<String>,
    /// On the far side: behind the body, darker.
    #[serde(default)]
    pub far: bool,
    /// How far the hand sways at rest (cells).
    #[serde(default = "sway")]
    pub sway: f32,
}

fn elbow() -> Knee {
    Knee::Down
}
fn sway() -> f32 {
    1.0
}

/// Legs, as a creature file writes them.
#[derive(Clone, Debug, Deserialize)]
pub struct LegsDef {
    #[serde(default)]
    pub view: View,
    /// Side: each leg.
    #[serde(default)]
    pub each: Vec<LegDef>,
    /// Side: how high the body's grip rides over its planted feet, how far
    /// back a foot gets before it steps (cells), and how far the body
    /// tilts at most (degrees).
    #[serde(default)]
    pub ride: f32,
    #[serde(default = "stride")]
    pub stride: f32,
    #[serde(default = "tilt")]
    pub tilt: f32,
    /// Side: the far legs' colour (else the near ones', darker).
    #[serde(default)]
    pub far: Option<(u8, u8, u8)>,
    /// Side: how far the body rises while a foot's in the air (cells: a
    /// two-legged walker bobs), and how far it leans nose down as it
    /// goes (degrees at 100 cells/s: a runner).
    #[serde(default)]
    pub bob: f32,
    #[serde(default)]
    pub pitch: f32,
    /// How thick a leg is at its hip, its knee and its foot (cells; with an
    /// `ankle`, at its ankle too, before the foot): drawn
    /// tapered and outlined, a limb with some flesh to it (a raptor's
    /// thighs, a crab's armour). Without it, lines `thick` across (a
    /// spider's).
    #[serde(default)]
    pub width: Vec<f32>,
    /// Side: a third bone, the foot's (cells; a bird's, a raptor's: the
    /// long bone from its ankle down to its toes), held `heel` degrees up
    /// from the ground behind the foot. `width` then has four: the hip,
    /// the knee, the ankle, the foot.
    #[serde(default)]
    pub ankle: f32,
    #[serde(default = "heel")]
    pub heel: f32,
    /// Side: its arms.
    #[serde(default)]
    pub arms: Vec<ArmDef>,
    /// Side: what its feet do coming down.
    #[serde(default)]
    pub footfall: Option<Footfall>,
    /// How its legs are drawn.
    #[serde(default)]
    pub style: LegStyle,
    /// Its chains (`chains.rs`: tails, necks, a sting).
    #[serde(default)]
    pub chains: Vec<super::chains::ChainDef>,
    /// Toes (cells long): a foot along the ground ahead, a claw behind.
    #[serde(default)]
    pub toes: f32,
    /// The outline's colour (else the leg's, much darker).
    #[serde(default)]
    pub outline: Option<(u8, u8, u8)>,
    #[serde(default = "eight")]
    pub count: usize,
    /// A leg's full length (cells); the thigh is `upper` of it.
    pub reach: f32,
    #[serde(default = "thigh")]
    pub upper: f32,
    /// Seconds a step takes; how high a foot lifts (cells).
    #[serde(default = "step_time")]
    pub step: f32,
    #[serde(default = "lift")]
    pub lift: f32,
    /// Where the legs meet, ahead of the body's middle (cells, along its
    /// heading), and how far apart their hips are along it.
    #[serde(default)]
    pub hips: f32,
    #[serde(default = "spread")]
    pub spread: f32,
    /// Colours: the leg, its joints; thighs this many cells thick (shins
    /// one less).
    pub color: (u8, u8, u8),
    #[serde(default)]
    pub joint: Option<(u8, u8, u8)>,
    #[serde(default = "one")]
    pub thick: u8,
    /// The body's sprite (seen from above, pointing right).
    pub body: String,
    /// Its pixels of this colour glow (eyes): drawn over the dark, at full
    /// brightness however dark it is.
    #[serde(default)]
    pub eyes: Option<(u8, u8, u8)>,
    /// A stinger (its sprite, from above, pointing right): shown curling
    /// over the body while it stings (`Rear::curl`).
    #[serde(default)]
    pub stinger: Option<String>,
}

/// How a legged body is held (a move: `moves/`): raised `lift` cells
/// off what it holds, drawn `back` cells behind its heading (a crouch), its
/// stinger curled `curl` of the way over its back and past its head.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Rear {
    pub lift: f32,
    pub back: f32,
    pub curl: f32,
    /// Its aiming chains: drawn back, coiled (0–1), and thrown out at the
    /// target (0–1: a sting striking).
    pub coil: f32,
    pub reach: f32,
}

fn eight() -> usize {
    8
}
fn heel() -> f32 {
    60.0
}
fn stride() -> f32 {
    10.0
}
fn tilt() -> f32 {
    30.0
}

impl LegsDef {
    /// How far a leg reaches, hip to foot (its ankle's bone too).
    pub fn full_reach(&self) -> f32 {
        self.reach + self.ankle
    }

    /// How many legs.
    pub fn legs(&self) -> usize {
        match self.view {
            View::Above => self.count,
            View::Side => self.each.len(),
        }
    }
}
fn thigh() -> f32 {
    0.55
}
fn step_time() -> f32 {
    0.12
}
fn lift() -> f32 {
    4.5
}
fn spread() -> f32 {
    4.5
}
fn one() -> u8 {
    1
}

/// A foot: where it is, where a step goes from and to, how far along
/// (1: planted), whether it holds anything.
#[derive(Clone, Copy, Debug)]
struct Foot {
    at: Vec2,
    from: Vec2,
    to: Vec2,
    t: f32,
    grips: bool,
    /// Seconds before a leg holding nothing looks again.
    retry: f32,
}

/// A creature's legs as they are now.
#[derive(Component)]
pub struct Legs {
    def: LegsDef,
    feet: Vec<Foot>,
    /// Where the body points (radians), and the drawn body.
    heading: f32,
    body: Entity,
    eyes: Option<Entity>,
    stinger: Option<Entity>,
    /// Where the body is drawn from its middle (rearing, crouching).
    offset: Vec2,
    /// Side: which way it faces (1 right, -1 left), its tilt (radians,
    /// nose up), and its grip's height (world y) as it rides its feet.
    facing: f32,
    tilt: f32,
    ride: Option<f32>,
    /// Side: the ground's slope under its feet (radians, up ahead), as
    /// last read.
    slope: f32,
    /// Its arms as they are now (the world: shoulder, elbow, wrist), and
    /// their claws' sprites.
    arms: Vec<(Vec2, Vec2, Vec2)>,
    claws: Vec<Option<Entity>>,
    /// Side: seconds its body's been off the ground; how far its furthest
    /// foot is from its hip, against its reach (a readout).
    air: f32,
    strain: f32,
    /// Where its body was last frame (a jump further than a leg reaches:
    /// put down, through a portal: its feet are planted afresh).
    last: Option<Vec2>,
    /// Side: its gait's clock (0..1 a cycle: every leg has stepped once).
    phase: f32,
    /// Its chains as they are now, and where an aiming one reaches for.
    chains: Vec<super::chains::Chain>,
    aim: Option<Vec2>,
}

#[derive(Component)]
struct LegClaw;

/// Over a creature's body and its near legs (10.06); a far claw behind its
/// far legs (9.5).
const Z_NEAR_CLAW: f32 = 10.07;
const Z_FAR_CLAW: f32 = 9.45;

#[derive(Component)]
struct LegBody;

/// Turned body sprites by name (and their eyes alone, by name + colour).
#[derive(Resource, Default)]
struct BodyArt(HashMap<String, crate::combat::Turned>);

use crate::creatures::body::animation::Z_EYES;

#[derive(Component)]
struct LegEyes;

#[derive(Component)]
struct LegStinger;

impl Legs {
    /// Leg `i`'s preferred way, from the heading (radians): fanned front to
    /// back, alternating sides. (Side: down to where its foot rests.)
    fn way(&self, i: usize) -> f32 {
        if self.def.view == View::Side {
            let l = &self.def.each[i];
            let rest = self.turn(Vec2::new(l.lean, -self.def.ride));
            return rest.y.atan2(rest.x);
        }
        let n = self.def.count.max(2);
        let per_side = n.div_ceil(2);
        let k = i / 2;
        let side = if i.is_multiple_of(2) { 1.0 } else { -1.0 };
        // From 25° off the nose to 155° (off the tail), evenly.
        let a = 25.0 + 130.0 * k as f32 / (per_side - 1).max(1) as f32;
        self.heading + side * a.to_radians()
    }

    /// How it's carried now (for readouts): its tilt (degrees, nose up),
    /// how high its grip rides over the bottom of its box (cells), and
    /// how many of its feet hold something.
    pub fn pose(&self, bottom: f32) -> (f32, f32, usize) {
        (self.tilt.to_degrees(), self.ride.map_or(0.0, |y| y - bottom), self.feet.iter().filter(|f| f.grips && f.t >= 1.0).count())
    }

    /// How far its furthest foot is from its hip, against its reach (over
    /// 1: further than the leg can reach; it's drawn at full length).
    pub fn strain(&self) -> f32 {
        self.strain
    }

    /// Its chains' tips (the world), whether it's aiming, which way it faces.
    pub fn chain_state(&self) -> (Vec<Vec2>, bool, f32) {
        (self.chains.iter().filter_map(|c| c.pts.last().copied()).collect(), self.aim.is_some(), self.facing)
    }

    /// Where its feet are (the world), and whether each holds something.
    pub fn feet(&self) -> impl Iterator<Item = (Vec2, bool)> + '_ {
        self.feet.iter().map(|f| (f.at, f.grips))
    }

    /// The body's frame to the world's, as a function (to hand on while
    /// its chains are borrowed): from the side, mirrored to its facing and
    /// tilted; from above, turned to its heading.
    fn frame(&self) -> impl Fn(Vec2) -> Vec2 + use<> {
        let (side, f, tilt, heading) = (self.def.view == View::Side, self.facing, self.tilt, self.heading);
        move |v: Vec2| if side { Vec2::from_angle(tilt * f).rotate(Vec2::new(v.x * f, v.y)) } else { Vec2::from_angle(heading).rotate(v) }
    }

    /// Which way a side-view joint bends (`ahead`: which way the limb's
    /// out from the body, along it), as a direction in the world.
    fn bend(&self, knee: Knee, ahead: f32) -> Vec2 {
        let up = self.turn(Vec2::Y);
        match knee {
            Knee::Out => (up + self.turn(Vec2::X * ahead.signum()) * 0.6).normalize_or(up),
            Knee::Forward => (self.turn(Vec2::X) + up * 0.3).normalize_or(up),
            Knee::Back => (self.turn(-Vec2::X) + up * 0.3).normalize_or(up),
            Knee::Up => up,
            Knee::Down => -up,
        }
    }

    /// A point on the body (cells from its grip, facing right, y up) as
    /// it's turned now (Side: mirrored to its facing, tilted).
    fn turn(&self, p: Vec2) -> Vec2 {
        Vec2::from_angle(self.tilt * self.facing).rotate(Vec2::new(p.x * self.facing, p.y))
    }

    /// Leg `i`'s hip, in the world, the body at `c`.
    fn hip(&self, i: usize, c: Vec2) -> Vec2 {
        if self.def.view == View::Side {
            let (x, y) = self.def.each[i].hip;
            return c + self.turn(Vec2::new(x, y));
        }
        let n = self.def.count.max(2);
        let per_side = n.div_ceil(2);
        let k = (i / 2) as f32 / (per_side - 1).max(1) as f32;
        let along = self.def.hips + self.def.spread * (0.5 - k);
        c + Vec2::from_angle(self.heading) * along
    }
}

fn solid(sim: &SimWorld, p: Vec2) -> bool {
    sim.world.get(CellPos::from_world(p.x, p.y)).is_none_or(|c| matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder))
}

/// A hold within reach of `hip`, about its way: rays swept round it
/// (±90°), each to the first solid; of those, the one nearest 70 % of its
/// reach (a leg stretched along a wall, not bunched against it), and least
/// turned from its way. The last open point before the solid one.
fn foothold(sim: &SimWorld, hip: Vec2, way: f32, reach: f32, back: bool) -> Option<Vec2> {
    // On the wall behind: anywhere along its way on that wall (rock in
    // reach still first).
    let behind = hip + Vec2::from_angle(way) * reach * 0.7;
    let back = (back && crate::creatures::backed(&sim.world, behind) && !solid(sim, behind)).then_some(behind);
    let mut best: Option<(f32, Vec2)> = None;
    for sweep in [0.0f32, 15.0, -15.0, 30.0, -30.0, 45.0, -45.0, 60.0, -60.0, 75.0, -75.0, 90.0, -90.0] {
        let d = Vec2::from_angle(way + sweep.to_radians());
        let mut last = hip;
        let mut s = 1.0;
        while s <= reach {
            let p = hip + d * s;
            if solid(sim, p) {
                // (Not from inside rock: a hip in a wall grips nothing.)
                if s > 1.0 {
                    let score = (s - reach * 0.7).abs() / reach + sweep.abs() / 180.0 * 0.6;
                    if best.is_none_or(|(b, _)| score < b) {
                        best = Some((score, last));
                    }
                }
                break;
            }
            last = p;
            s += 0.7;
        }
    }
    best.map(|(_, p)| p).or(back)
}

/// Side: where a foot goes, wanting to be under `x`: the ground there if
/// it's in reach, else nearer the hip (down a steep slope the ground ahead
/// is past its reach; nearer, it isn't).
fn ground_under(sim: &SimWorld, hip: Vec2, x: f32, reach: f32) -> Option<Vec2> {
    (0..5).find_map(|k| ground_at(sim, hip, x + (hip.x - x) * k as f32 / 5.0, reach))
}

/// The ground under `x` within reach of the hip: down from the hip's
/// height (up out of rock first, a little: a foot uphill) to the first
/// solid cell; the foot on its top.
fn ground_at(sim: &SimWorld, hip: Vec2, x: f32, reach: f32) -> Option<Vec2> {
    let mut y = hip.y;
    while solid(sim, Vec2::new(x, y)) {
        y += 1.0;
        if y > hip.y + reach * 0.3 {
            return None;
        }
    }
    let lowest = hip.y - reach * 1.05;
    while y > lowest {
        if solid(sim, Vec2::new(x, y - 1.0)) {
            let foot = Vec2::new(x, y.floor());
            return ((foot - hip).length() <= reach * 1.02).then_some(foot);
        }
        y -= 1.0;
    }
    None
}

/// Straight down, most of a leg's reach (where a foot with nothing under
/// it hangs).
fn legs_turn_down(reach: f32) -> Vec2 {
    Vec2::NEG_Y * reach * 0.7
}

/// From `a` toward `b`, as far as it's open (a free leg never reaches into
/// rock).
fn open_toward(sim: &SimWorld, a: Vec2, b: Vec2) -> Vec2 {
    let d = b - a;
    let n = (d.length() / 0.7).ceil().max(1.0) as i32;
    let mut last = a;
    for k in 1..=n {
        let p = a + d * (k as f32 / n as f32);
        if solid(sim, p) {
            return last;
        }
        last = p;
    }
    b
}

/// Give legged creatures their legs and their turned body.
#[allow(clippy::too_many_arguments)]
fn grow_legs(
    mut commands: Commands,
    mut art: ResMut<BodyArt>,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    sim: Res<SimWorld>,
    new: Query<(Entity, &Animator, &Kinematics), Without<Legs>>,
) {
    for (e, anim, k) in &new {
        let Some(def) = anim.def.legs.clone() else { continue };
        let eyes_key = def.eyes.map(|(r, g, b)| format!("{}#{r},{g},{b}", def.body));
        for (key, only) in [(Some(def.body.clone()), None), (eyes_key.clone(), def.eyes.map(|(r, g, b)| [r, g, b]))] {
            let Some(key) = key else { continue };
            if art.0.contains_key(&key) {
                continue;
            }
            match crate::combat::turned_art(&def.body, only, &mut images, &mut layouts) {
                Ok(t) => {
                    art.0.insert(key, t);
                }
                Err(err) => warn!("legs: body `{}`: {err}", def.body),
            }
        }
        let Some(turned) = art.0.get(&def.body) else { continue };
        let body = commands.spawn((LegBody, turned.sprite(0.0), Transform::from_xyz(0.0, 0.0, 0.02))).id();
        commands.entity(e).add_child(body);
        // The eyes: over the dark (a child of the root, lifted above the light).
        let root_z = anim.def.z;
        let eyes = eyes_key.and_then(|key| art.0.get(&key)).map(|t| commands.spawn((LegEyes, crate::creatures::body::animation::Blinks, t.sprite(0.0), Transform::from_xyz(0.0, 0.0, Z_EYES - root_z))).id());
        if let Some(eyes) = eyes {
            commands.entity(e).add_child(eyes);
        }
        let stinger = def.stinger.clone().and_then(|name| {
            if !art.0.contains_key(&name) {
                match crate::combat::turned_art(&name, None, &mut images, &mut layouts) {
                    Ok(t) => {
                        art.0.insert(name.clone(), t);
                    }
                    Err(err) => warn!("legs: stinger `{name}`: {err}"),
                }
            }
            art.0.get(&name).map(|t| commands.spawn((LegStinger, t.sprite(0.0), Transform::from_xyz(0.0, 0.0, 0.05), Visibility::Hidden)).id())
        });
        if let Some(s) = stinger {
            commands.entity(e).add_child(s);
        }
        let facing = if k.loco.facing < 0.0 { -1.0 } else { 1.0 };
        let mut legs = Legs { feet: Vec::new(), heading: 0.0, body, eyes, stinger, offset: Vec2::ZERO, facing, tilt: 0.0, ride: None, slope: 0.0, arms: Vec::new(), claws: Vec::new(), air: 0.0, strain: 0.0, last: None, phase: 0.0, chains: Vec::new(), aim: None, def };
        // Its arms' claws: turned sprites at their wrists.
        for arm in legs.def.arms.clone() {
            let claw = arm.claw.and_then(|name| {
                if !art.0.contains_key(&name) {
                    match crate::combat::turned_art(&name, None, &mut images, &mut layouts) {
                        Ok(t) => {
                            art.0.insert(name.clone(), t);
                        }
                        Err(err) => warn!("legs: claw `{name}`: {err}"),
                    }
                }
                let z = if arm.far { Z_FAR_CLAW } else { Z_NEAR_CLAW } - root_z;
                art.0.get(&name).map(|t| commands.spawn((LegClaw, t.sprite(0.0), Transform::from_xyz(0.0, 0.0, z))).id())
            });
            if let Some(c) = claw {
                commands.entity(e).add_child(c);
            }
            legs.claws.push(claw);
        }
        // Its chains' tips: turned sprites at their ends (a sting).
        for chain in legs.def.chains.clone() {
            let tip = chain.tip.and_then(|name| {
                if !art.0.contains_key(&name) {
                    match crate::combat::turned_art(&name, None, &mut images, &mut layouts) {
                        Ok(t) => {
                            art.0.insert(name.clone(), t);
                        }
                        Err(err) => warn!("legs: chain tip `{name}`: {err}"),
                    }
                }
                let z = if chain.far { Z_FAR_CLAW } else { Z_NEAR_CLAW } - root_z;
                art.0.get(&name).map(|t| commands.spawn((LegClaw, t.sprite(0.0), Transform::from_xyz(0.0, 0.0, z))).id())
            });
            if let Some(t) = tip {
                commands.entity(e).add_child(t);
            }
            legs.chains.push(super::chains::Chain::with_tip(tip));
        }
        let c = match legs.def.view {
            View::Above => k.body.pos,
            View::Side => Vec2::new(k.body.pos.x, k.body.pos.y - k.body.half.y + legs.def.ride),
        };
        for i in 0..legs.def.legs() {
            let hip = legs.hip(i, c);
            let held = match legs.def.view {
                View::Above => foothold(&sim, hip, legs.way(i), legs.def.reach, false),
                View::Side => ground_under(&sim, hip, hip.x + legs.def.each[i].lean * facing, legs.def.full_reach()),
            };
            let at = held.unwrap_or(hip + Vec2::from_angle(legs.way(i)) * legs.def.full_reach() * 0.5);
            legs.feet.push(Foot { at, from: at, to: at, t: 1.0, grips: true, retry: 0.0 });
        }
        commands.entity(e).insert(legs);
    }
}

/// Feet stay put, then step; the body turns to where it's going (from
/// above) or rides its feet (from the side).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn walk(
    time: Res<Time>,
    sim: Res<SimWorld>,
    mut q: Query<(&mut Legs, &Kinematics, &GlobalTransform, &Children, Option<&Rear>)>,
    mut sprites: Query<(&mut Sprite, &mut Visibility), (With<CreatureSprite>, Without<LegBody>, Without<LegStinger>)>,
    mut bodies: Query<(&mut Sprite, &mut Transform), (Or<(With<LegBody>, With<LegEyes>)>, Without<CreatureSprite>, Without<LegStinger>, Without<LegClaw>)>,
    mut stingers: Query<(&mut Sprite, &mut Transform, &mut Visibility), (With<LegStinger>, Without<CreatureSprite>, Without<LegBody>, Without<LegEyes>, Without<LegClaw>)>,
    mut claws: Query<(&mut Sprite, &mut Transform), (With<LegClaw>, Without<CreatureSprite>, Without<LegBody>, Without<LegEyes>, Without<LegStinger>)>,
    mut falls: ResMut<Footfalls>,
) {
    let dt = time.delta_secs().min(0.05);
    let now = time.elapsed_secs();
    for (mut legs, k, tf, children, rear) in &mut q {
        // (Where it's drawn; its body's place if that's far off: on its
        // first frame its transform hasn't caught up yet.)
        let drawn = tf.translation().truncate();
        let middle = if drawn.distance(k.body.pos) > legs.def.full_reach() { k.body.pos } else { drawn };
        let side = legs.def.view == View::Side;
        // (Moved further than a leg reaches since last frame: put down, or
        // through a portal. Its ride and tilt start afresh; its feet are
        // planted afresh, below.)
        let jumped = legs.last.is_none_or(|l| l.distance(middle) > legs.def.full_reach());
        if jumped {
            legs.ride = None;
            legs.tilt = 0.0;
            legs.slope = 0.0;
        }
        legs.last = Some(middle);
        let rear = rear.copied().unwrap_or_default();
        let v = k.body.vel;
        // Where the body is: from above, raised off what it holds and
        // drawn back from its heading (an attack); from the side, riding
        // its feet, tilted with them.
        // (From the side, turned round: its body flips at once, so its
        // feet are planted afresh on the other side too.)
        let turned = side && (k.loco.facing < 0.0) != (legs.facing < 0.0);
        let jumped = jumped || turned;
        let (c, up) = if side {
            legs.facing = if k.loco.facing < 0.0 { -1.0 } else { 1.0 };
            let f = legs.facing;
            let base = middle.y - k.body.half.y;
            let planted: Vec<Vec2> = legs.feet.iter().filter(|foot| foot.grips).map(|foot| foot.to).collect();
            let ground = if planted.is_empty() { base } else { planted.iter().map(|p| p.y).sum::<f32>() / planted.len() as f32 };
            // (Up while a foot's in the air: a walker on two legs bobs.)
            let up = legs.feet.iter().filter(|foot| foot.t < 1.0 && foot.grips).map(|foot| (std::f32::consts::PI * foot.t).sin()).fold(0.0, f32::max);
            let want = (ground + legs.def.ride + legs.def.bob * up).clamp(base + legs.def.ride * 0.5, middle.y + k.body.half.y);
            let y = legs.ride.map_or(want, |y| y + (want - y) * (dt * 12.0).min(1.0));
            legs.ride = Some(y);
            // Nose up as the ground its feet stand on rises ahead: the
            // slope of the line through its planted feet (along the way it
            // faces), once they're spread enough to say.
            let want = {
                let pts: Vec<Vec2> = planted.iter().map(|p| Vec2::new((p.x - middle.x) * f, p.y)).collect();
                let n = pts.len() as f32;
                let mean = pts.iter().sum::<Vec2>() / n.max(1.0);
                let (sxy, sxx) = pts.iter().fold((0.0, 0.0), |(sxy, sxx), p| (sxy + (p.x - mean.x) * (p.y - mean.y), sxx + (p.x - mean.x).powi(2)));
                if pts.len() >= 2 && sxx / n > 9.0 {
                    legs.slope = (sxy / sxx).atan();
                } else {
                    // (Feet together, nothing to read: level, slowly.)
                    legs.slope *= 1.0 - (dt * 2.0).min(1.0);
                }
                legs.slope
            };
            // (Nose down as it goes, a runner's lean.)
            let want = want - (legs.def.pitch * (v.x.abs() / 100.0).min(1.5)).to_radians();
            let most = legs.def.tilt.to_radians();
            legs.tilt += (want.clamp(-most, most) - legs.tilt) * (dt * 8.0).min(1.0);
            (Vec2::new(middle.x, y) + Vec2::Y * rear.lift - Vec2::X * f * rear.back, Vec2::Y)
        } else {
            let up = {
                let held: Vec<Vec2> = legs.feet.iter().filter(|f| f.grips).map(|f| f.at).collect();
                if held.is_empty() { Vec2::Y } else { (middle - held.iter().sum::<Vec2>() / held.len() as f32).normalize_or(Vec2::Y) }
            };
            let fwd = Vec2::from_angle(legs.heading);
            (middle + up * rear.lift - fwd * rear.back, up)
        };
        legs.offset = c - middle;
        let fwd = Vec2::from_angle(legs.heading);
        // (Jumped: every foot planted afresh, no steps across; its chains
        // at rest.)
        if jumped {
            for ch in &mut legs.chains {
                ch.pts.clear();
            }
            for i in 0..legs.feet.len() {
                let hip = legs.hip(i, c);
                let way = legs.way(i);
                let held = match legs.def.view {
                    View::Above => foothold(&sim, hip, way, legs.def.full_reach(), false),
                    View::Side => ground_under(&sim, hip, hip.x + legs.def.each[i].lean * legs.facing, legs.def.full_reach()),
                };
                let at = held.unwrap_or(hip + Vec2::from_angle(way) * legs.def.full_reach() * 0.5);
                legs.feet[i] = Foot { at, from: at, to: at, t: 1.0, grips: held.is_some(), retry: 0.0 };
            }
        }
        if !side {
            // Heading: where it goes (or, still, where it aims), turning steadily.
            let want = if v.length() > 9.0 { v.y.atan2(v.x) } else { legs.heading };
            let mut d = (want - legs.heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            d = d.clamp(-8.0 * dt, 8.0 * dt);
            legs.heading += d;
        }
        // The drawn body: the creature's own sprite hidden (its colour
        // borrowed: hurt flashes, tints), the turned one shown.
        let mut tint = None;
        for child in children.iter() {
            if let Ok((s, mut vis)) = sprites.get_mut(child) {
                tint = Some(s.color);
                *vis = Visibility::Hidden;
            }
        }
        // (From the side: tilted, mirrored to face left.)
        let (index, flip) = if side { (crate::combat::Turned::index(legs.tilt.to_degrees()), legs.facing < 0.0) } else { (crate::combat::Turned::index(legs.heading.to_degrees()), false) };
        let offset = legs.offset;
        if let Ok((mut s, mut t)) = bodies.get_mut(legs.body) {
            if let Some(atlas) = s.texture_atlas.as_mut() {
                atlas.index = index;
            }
            s.flip_x = flip;
            if let Some(t) = tint {
                s.color = t;
            }
            t.translation = offset.extend(t.translation.z);
        }
        if let Some(Ok((mut s, mut t))) = legs.eyes.map(|e| bodies.get_mut(e)) {
            if let Some(atlas) = s.texture_atlas.as_mut() {
                atlas.index = index;
            }
            s.flip_x = flip;
            t.translation = offset.extend(t.translation.z);
        }
        // The stinger: from over the abdomen, up over the back, down past
        // the head, turning from pointing back to pointing where it strikes.
        if let Some(Ok((mut s, mut t, mut vis))) = legs.stinger.map(|e| stingers.get_mut(e)) {
            let curl = rear.curl.clamp(0.0, 1.0);
            *vis = if curl > 0.05 { Visibility::Inherited } else { Visibility::Hidden };
            let along = -21.0 + 48.0 * curl;
            let at = offset + fwd * along + up * (std::f32::consts::PI * curl).sin() * 24.0;
            let angle = legs.heading + std::f32::consts::PI * (1.0 - curl);
            if let Some(atlas) = s.texture_atlas.as_mut() {
                atlas.index = crate::combat::Turned::index(angle.to_degrees());
            }
            t.translation = at.extend(t.translation.z);
        }
        // Steps.
        let (reach, n) = (legs.def.full_reach(), legs.feet.len());
        // (On the wall behind: its feet can hold on to it anywhere.)
        let back = k.loco.clinging() == Some(Vec2::ZERO);
        let stepping = legs.feet.iter().filter(|f| f.t < 1.0).count();
        let step_time = legs.def.step;
        // (From the side, off the ground a moment: its feet let go, tucked
        // up under it till it lands.)
        let grounded = k.loco.grounded() || k.loco.clinging().is_some();
        legs.air = if grounded { 0.0 } else { legs.air + dt };
        let aloft = side && legs.air > 0.08;
        legs.strain = (0..n).map(|i| (legs.feet[i].at - legs.hip(i, c)).length() / reach).fold(0.0, f32::max);
        // From the side, on the ground: its gait's clock. It goes round as
        // the body goes (a cycle every `stride` of ground a planted foot
        // covers, over the share of the cycle it's planted), each gait group
        // its share of a cycle on from the last; a foot is planted for the
        // first part of its turn (more of it walking, less running: a runner
        // has both feet off the ground a moment), then swings forward to
        // the ground under where it'll be needed: ahead of where it rests
        // by half a stride. Standing, the clock stops, unless a step's to
        // finish or a foot's been left behind.
        if side && !aloft {
            let speed = v.x.abs();
            let groups = legs.def.each.iter().map(|l| l.gait).max().unwrap_or(0) as f32 + 1.0;
            let duty = 0.68 - 0.26 * (speed / 150.0).min(1.0);
            let cycle = (legs.def.stride / duty).max(1.0);
            let dir = if speed > 5.0 { v.x.signum() } else { legs.facing };
            let target = |legs: &Legs, i: usize| {
                let hip = legs.hip(i, c);
                (hip, hip.x + legs.def.each[i].lean * legs.facing + dir * legs.def.stride * 0.5)
            };
            let behind = (0..n).any(|i| legs.feet[i].t >= 1.0 && (legs.feet[i].at.x - target(&legs, i).1).abs() > legs.def.stride * 0.9);
            let swinging = legs.feet.iter().any(|f| f.t < 1.0);
            let mut rate = speed / cycle;
            if swinging || behind {
                rate = rate.max((1.0 - duty) / step_time.max(0.01));
            }
            legs.phase = (legs.phase + rate * dt).fract();
            let lift = legs.def.lift;
            let footfall = legs.def.footfall.clone();
            for i in 0..n {
                let p = (legs.phase + legs.def.each[i].gait as f32 / groups).fract();
                let (hip, x) = target(&legs, i);
                let foot = &mut legs.feet[i];
                if p < duty {
                    // Planted (it comes down where its swing was going).
                    if foot.t < 1.0 {
                        foot.t = 1.0;
                        foot.at = foot.to;
                        if foot.grips
                            && let Some(ff) = &footfall
                        {
                            falls.0.push((foot.at, ff.clone()));
                        }
                    }
                    // (Left past its reach, a slip: put down again now.)
                    if foot.grips
                        && (foot.at - hip).length() > reach
                        && let Some(p) = ground_under(&sim, hip, x, reach)
                    {
                        foot.at = p;
                        foot.to = p;
                    }
                } else {
                    // Swinging: from where it lifted to where it'll land,
                    // that kept up to date as the body goes, an arc up.
                    if foot.t >= 1.0 {
                        foot.from = foot.at;
                    }
                    let s = (p - duty) / (1.0 - duty);
                    match ground_under(&sim, hip, x, reach) {
                        Some(to) => {
                            foot.to = to;
                            foot.grips = true;
                        }
                        None => {
                            foot.to = open_toward(&sim, hip, hip + legs_turn_down(reach));
                            foot.grips = false;
                        }
                    }
                    foot.t = s.min(0.999);
                    let e = s * s * (3.0 - 2.0 * s);
                    foot.at = foot.from.lerp(foot.to, e) + Vec2::Y * lift * (std::f32::consts::PI * s).sin();
                }
            }
        }
        for i in 0..n {
            if side && !aloft {
                break;
            }
            if aloft {
                let foot = &mut legs.feet[i];
                foot.grips = false;
                foot.t = 1.0;
                foot.retry = 0.0;
                continue;
            }
            let hip = legs.hip(i, c);
            let way = legs.way(i);
            let f = legs.feet[i];
            if f.t < 1.0 {
                continue;
            }
            // Where it would put its foot now (from the side: under where it
            // rests, ahead by as far as the body goes in a step).
            let rest = side.then(|| hip.x + legs.def.each[i].lean * legs.facing + v.x * step_time);
            let off = f.at - hip;
            let due = match rest {
                Some(x) => !f.grips && f.retry <= 0.0 || f.grips && ((f.at.x - x).abs() > legs.def.stride || off.length() > reach * 0.98),
                None => {
                    let twisted = off.length() > 0.75 && Vec2::from_angle(way).dot(off.normalize()) < 0.2;
                    let stretched = off.length() > reach * 0.98;
                    let crowded = off.length() < reach * 0.25;
                    if f.grips { stretched || crowded || twisted } else { f.retry <= 0.0 }
                }
            };
            // (From above, not with a neighbour, nor more than a third at
            // once; from the side, not while another gait group steps.)
            let busy = if side {
                let g = legs.def.each[i].gait;
                legs.feet.iter().zip(&legs.def.each).any(|(o, l)| o.t < 1.0 && l.gait != g)
            } else {
                let neighbours = [(i + n - 2) % n, (i + 2) % n];
                neighbours.iter().any(|&j| legs.feet[j].t < 1.0) || stepping >= n.div_ceil(3)
            };
            legs.feet[i].retry -= dt;
            // (A foot pulled past its reach lets go now, its gait or not.)
            let torn = side && f.grips && off.length() > reach;
            if !due || busy && !torn {
                continue;
            }
            let to = match rest {
                Some(x) => ground_under(&sim, hip, x, reach),
                None => foothold(&sim, hip, way, reach, back),
            };
            let foot = &mut legs.feet[i];
            foot.from = foot.at;
            match to {
                Some(p) => {
                    foot.to = p;
                    foot.grips = true;
                }
                None => {
                    // Nothing to hold: it reaches out, feeling the air.
                    foot.to = open_toward(&sim, hip, hip + Vec2::from_angle(way) * reach * 0.7);
                    foot.grips = false;
                    foot.retry = 0.2;
                }
            }
            foot.t = 0.0;
        }
        // Feet on their way: an arc off what they held (from the side: up).
        let away = if side {
            Vec2::Y
        } else {
            let held: Vec<Vec2> = legs.feet.iter().filter(|f| f.grips).map(|f| f.at).collect();
            if held.is_empty() { Vec2::Y } else { (c - held.iter().sum::<Vec2>() / held.len() as f32).normalize_or(Vec2::Y) }
        };
        let lift = legs.def.lift;
        for f in &mut legs.feet {
            // (From the side, the gait's clock moves them.)
            if f.t < 1.0 && !side {
                f.t = (f.t + dt / step_time.max(0.01)).min(1.0);
                let s = f.t * f.t * (3.0 - 2.0 * f.t);
                f.at = f.from.lerp(f.to, s) + away * lift * (std::f32::consts::PI * f.t).sin();
            }
        }
        // Arms: each hand where it rests, swaying (each its own beat); the
        // elbow as it bends; a claw at the wrist turned to the forearm.
        let arms: Vec<ArmDef> = legs.def.arms.clone();
        legs.arms.clear();
        for (j, arm) in arms.iter().enumerate() {
            let shoulder = c + legs.turn(Vec2::new(arm.shoulder.0, arm.shoulder.1));
            let phase = now * (1.3 + j as f32 * 0.31) + j as f32 * 2.1;
            let hand = c + legs.turn(Vec2::new(arm.hand.0, arm.hand.1) + Vec2::new(phase.sin(), (phase * 1.7).cos()) * arm.sway);
            let (elbow, wrist) = knee(shoulder, hand, arm.bones.0, arm.bones.1, legs.bend(arm.elbow, 1.0), |p| solid(&sim, p));
            legs.arms.push((shoulder, elbow, wrist));
            if let Some(Ok((mut s, mut t))) = legs.claws.get(j).copied().flatten().map(|e| claws.get_mut(e)) {
                let d = wrist - elbow;
                let local = d.y.atan2(d.x * legs.facing);
                if let Some(atlas) = s.texture_atlas.as_mut() {
                    atlas.index = crate::combat::Turned::index(local.to_degrees());
                }
                s.flip_x = legs.facing < 0.0;
                if let Some(t) = tint {
                    s.color = t;
                }
                t.translation = (wrist - middle).extend(t.translation.z);
            }
        }
        // Chains: each from its anchor, swinging after the body, reaching
        // for what it aims at; a sprite at its tip.
        let turn = legs.frame();
        let (aim, facing) = (legs.aim, legs.facing);
        let defs = legs.def.chains.clone();
        for (j, def) in defs.iter().enumerate() {
            let anchor = c + turn(Vec2::new(def.anchor.0, def.anchor.1));
            let Some(ch) = legs.chains.get_mut(j) else { continue };
            let aims = aim.filter(|_| def.aims.is_some());
            ch.step(def, anchor, &turn, aims, rear.coil, dt);
            ch.strike(def, anchor, &turn, aims, rear.reach);
            if let (Some(tip), [.., a, b]) = (ch.tip, ch.pts.as_slice())
                && let Ok((mut s, mut t)) = claws.get_mut(tip)
            {
                let d = *b - *a;
                let local = if side { d.y.atan2(d.x * facing) } else { d.y.atan2(d.x) };
                if let Some(atlas) = s.texture_atlas.as_mut() {
                    atlas.index = crate::combat::Turned::index(local.to_degrees());
                }
                s.flip_x = side && facing < 0.0;
                if let Some(t) = tint {
                    s.color = t;
                }
                t.translation = (*b - middle).extend(t.translation.z);
            }
        }
        // Legs holding nothing reach out from the body as it goes, and
        // twitch, each out of step.
        for i in 0..n {
            if legs.feet[i].grips || legs.feet[i].t < 1.0 {
                continue;
            }
            let wiggle = (now * (2.3 + i as f32 * 0.37) + i as f32 * 1.7).sin() * 0.35;
            let hip = legs.hip(i, c);
            // (From the side: tucked up under it, a little ahead or behind.)
            let reach_out = if side {
                hip + legs.turn(Vec2::new(legs.def.each[i].lean * 0.3, -reach * 0.5))
            } else {
                hip + Vec2::from_angle(legs.way(i) + wiggle) * reach * 0.7
            };
            let to = open_toward(&sim, hip, reach_out);
            let f = &mut legs.feet[i];
            f.to = to;
            f.at = f.at.lerp(to, (dt * 8.0).min(1.0));
            // (A free foot is never further than its leg reaches.)
            if (f.at - hip).length() > reach {
                f.at = hip + (f.at - hip).normalize_or(Vec2::NEG_Y) * reach;
            }
        }
    }
}

/// Where a creature's aiming chains reach for: the nearest thing it
/// hunts within the furthest of their ranges (its middle), or nothing.
fn aims(mut legs: Query<(&mut Legs, &Kinematics)>, hunted: Query<(&Kinematics, &crate::creatures::Team)>) {
    for (mut l, k) in &mut legs {
        let Some(range) = l.def.chains.iter().filter_map(|c| c.aims).reduce(f32::max) else {
            l.aim = None;
            continue;
        };
        l.aim = hunted
            .iter()
            .filter(|(_, t)| t.hunted())
            .map(|(hk, _)| hk.body.pos)
            .filter(|p| p.distance(k.body.pos) <= range)
            .min_by(|a, b| a.distance(k.body.pos).total_cmp(&b.distance(k.body.pos)));
    }
}

/// Feet that came down: their thud (and the ground's footstep), dust,
/// a shake of the screen if near.
fn footfalls(mut falls: ResMut<Footfalls>, mut sim: ResMut<SimWorld>, mut sounds: MessageWriter<crate::sound::PlaySound>, mut trauma: ResMut<crate::fx::Trauma>, camera: Query<&GlobalTransform, With<crate::camera::MainCamera>>) {
    let eye = camera.single().map(|c| c.translation().truncate()).ok();
    for (at, ff) in falls.0.drain(..) {
        if let Some(name) = &ff.sound {
            sounds.write(crate::sound::PlaySound::at(name.clone(), at).volume(ff.volume));
        }
        let under = CellPos::from_world(at.x, at.y - 0.5);
        if ff.ground
            && let Some(what) = crate::sound::hooks::underfoot(&sim.world, under)
        {
            let name = match what {
                "stone" => "step_stone",
                "dirt" => "step_dirt",
                "sand" => "step_sand",
                "wood" => "step_wood",
                "snow" => "step_snow",
                "water" => "step_water",
                _ => "step_grass",
            };
            sounds.write(crate::sound::PlaySound::at(name, at).volume(ff.volume));
        }
        if ff.dust > 0
            && let Some(cell) = sim.world.get(under).filter(|c| c.material != platypus_sim::MaterialId::AIR)
        {
            sim.world.puff([at.x, at.y + 0.5], cell, ff.dust as usize, 1.6);
        }
        if ff.shake > 0.0
            && let Some(eye) = eye
        {
            let near = (1.0 - eye.distance(at) / 400.0).max(0.0);
            trauma.0 = (trauma.0 + ff.shake * near).min(1.0);
        }
    }
}

/// Where the knee is: two bones of `a` and `b` from `hip` to `foot`, the
/// knee on the `up` side, unless that's in rock and the other side isn't
/// (a knee never bends into the ground).
fn knee(hip: Vec2, foot: Vec2, a: f32, b: f32, up: Vec2, rock: impl Fn(Vec2) -> bool) -> (Vec2, Vec2) {
    let d = foot - hip;
    let len = d.length().clamp((a - b).abs() + 0.01, a + b - 0.01);
    let dir = d.normalize_or(Vec2::X);
    let foot = hip + dir * len;
    // Law of cosines: along the hip–foot line, then out.
    let x = (a * a - b * b + len * len) / (2.0 * len);
    let h = (a * a - x * x).max(0.0).sqrt();
    let side = Vec2::new(-dir.y, dir.x);
    let k1 = hip + dir * x + side * h;
    let k2 = hip + dir * x - side * h;
    let (first, second) = if (k1 - hip).dot(up) >= (k2 - hip).dot(up) { (k1, k2) } else { (k2, k1) };
    let k = if rock(first) && !rock(second) { second } else { first };
    (k, foot)
}

/// A limb from `a` to `b`, `ra` cells round at `a` tapering to `rb` at `b`:
/// discs stamped along it, each cell once per disc (later strokes over
/// earlier ones).
fn stroke(a: Vec2, b: Vec2, ra: f32, rb: f32, c: [u8; 4], out: &mut Vec<(IVec2, [u8; 4])>) {
    let n = ((b - a).length() / 0.5).ceil().max(1.0) as i32;
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let p = a.lerp(b, t);
        let r = (ra + (rb - ra) * t).max(0.5);
        let ri = r.ceil() as i32;
        let (cx, cy) = (p.x.floor() as i32, p.y.floor() as i32);
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let q = Vec2::new((cx + dx) as f32 + 0.5, (cy + dy) as f32 + 0.5);
                if q.distance_squared(p) <= r * r {
                    out.push((IVec2::new(cx + dx, cy + dy), c));
                }
            }
        }
    }
}

/// A plate from `a` to `b`, `r` cells to each side of the line (square
/// ends); with `under`, only its underside's band in that colour (shaded).
fn plate(a: Vec2, b: Vec2, r: f32, c: [u8; 4], under: Option<[u8; 4]>, out: &mut Vec<(IVec2, [u8; 4])>) {
    let d = b - a;
    let len = d.length().max(0.01);
    let dir = d / len;
    let n = dir.perp();
    // (Under: the side away from up.)
    let down = if n.y > 0.0 { -n } else { n };
    let lo = a.min(b) - Vec2::splat(r + 1.0);
    let hi = a.max(b) + Vec2::splat(r + 1.0);
    for y in lo.y.floor() as i32..=hi.y.ceil() as i32 {
        for x in lo.x.floor() as i32..=hi.x.ceil() as i32 {
            let q = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - a;
            let along = q.dot(dir);
            let across = q.dot(n);
            if along < -0.5 || along > len + 0.5 || across.abs() > r {
                continue;
            }
            match under {
                Some(shade) => {
                    if q.dot(down) > r - 1.2 {
                        out.push((IVec2::new(x, y), shade));
                    }
                }
                None => out.push((IVec2::new(x, y), c)),
            }
        }
    }
}

/// A line of cells from `a` to `b` (Bresenham), each once.
fn cells(a: Vec2, b: Vec2, out: &mut Vec<IVec2>) {
    let (mut x0, mut y0) = (a.x.floor() as i32, a.y.floor() as i32);
    let (x1, y1) = (b.x.floor() as i32, b.y.floor() as i32);
    let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
    let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
    let mut err = dx + dy;
    loop {
        out.push(IVec2::new(x0, y0));
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// The legs' canvases (`canvas.rs`): behind the bodies (from above, and the
/// far legs from the side), and in front (the near legs from the side);
/// each again for the bestiary's stage.
#[derive(Resource)]
struct LegCanvas([crate::canvas::Canvas; 4]);

/// In front of a creature's body (10.02), under its eyes.
const Z_NEAR_LEGS: f32 = 10.06;

impl Default for LegCanvas {
    fn default() -> Self {
        use crate::canvas::Canvas;
        LegCanvas([Canvas::new("Legs", 9.5), Canvas::new("Near legs", Z_NEAR_LEGS), Canvas::new("Legs on the stage", 9.5), Canvas::new("Near legs on the stage", Z_NEAR_LEGS)])
    }
}

/// Every leg, a cell at a time, onto the canvases.
#[allow(clippy::too_many_arguments)]
fn draw(
    mut commands: Commands,
    sim: Res<SimWorld>,
    mut canvas: ResMut<LegCanvas>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<(&GlobalTransform, &crate::world::ChunkLoader), With<crate::camera::MainCamera>>,
    mut sprites: crate::canvas::CanvasSprites,
    stage: Res<crate::canvas::StageView>,
    q: Query<(&Legs, &GlobalTransform)>,
) {
    // (Behind, in front.)
    let mut quads: [Vec<(IVec2, [u8; 4])>; 2] = [Vec::new(), Vec::new()];
    let mut line = Vec::new();
    for (legs, tf) in &q {
        // (The body where it's drawn: rearing lifts the hips.)
        let c = tf.translation().truncate() + legs.offset;
        let side = legs.def.view == View::Side;
        let rgb = |(r, g, b): (u8, u8, u8)| [r, g, b, 255];
        let dark = |(r, g, b): (u8, u8, u8)| ((r as f32 * 0.6) as u8, (g as f32 * 0.6) as u8, (b as f32 * 0.6) as u8);
        let (a, b) = (legs.def.reach * legs.def.upper, legs.def.reach * (1.0 - legs.def.upper));
        let held: Vec<Vec2> = legs.feet.iter().filter(|f| f.grips).map(|f| f.at).collect();
        let up = if side {
            legs.turn(Vec2::Y)
        } else if held.is_empty() {
            Vec2::Y
        } else {
            (c - held.iter().sum::<Vec2>() / held.len() as f32).normalize_or(Vec2::Y)
        };
        for (i, f) in legs.feet.iter().enumerate() {
            let hip = legs.hip(i, c);
            // (Knees also splay out from the body a little: from the side,
            // a front leg's forward, a back one's back.)
            let bend = if side {
                let l = &legs.def.each[i];
                legs.bend(l.knee, if l.lean != 0.0 { l.lean } else { l.hip.0 })
            } else {
                (up + Vec2::from_angle(legs.way(i)) * 0.6).normalize_or(up)
            };
            // (With an ankle's bone: from the foot back up to the ankle, at
            // its heel's angle; the knee between the hip and the ankle.)
            let ahead = Vec2::X * legs.facing;
            let ankle = (legs.def.ankle > 0.0).then(|| f.at + (-ahead * legs.def.heel.to_radians().cos() + Vec2::Y * legs.def.heel.to_radians().sin()) * legs.def.ankle);
            // (Drawn as solved: past its reach the leg is at full length
            // and the foot comes with it, never stretched.)
            let (k, end) = knee(hip, ankle.unwrap_or(f.at), a, b, bend, |p| solid(&sim, p));
            let (ankle, foot) = match ankle {
                Some(at) => (Some(end), f.at + (end - at)),
                None => (None, end),
            };
            let far = side && legs.def.each[i].far;
            let (leg, joint) = if far {
                let c = legs.def.far.unwrap_or(dark(legs.def.color));
                (rgb(c), rgb(legs.def.joint.map_or(c, dark)))
            } else {
                (rgb(legs.def.color), rgb(legs.def.joint.unwrap_or(legs.def.color)))
            };
            let to = &mut quads[(side && !far) as usize];
            if !legs.def.width.is_empty() {
                // Tapered, outlined: the outline first (a cell wider all
                // round), then the flesh over it; toes along the ground.
                let w = |i: usize| legs.def.width.get(i).or(legs.def.width.last()).copied().unwrap_or(1.0);
                let line = legs.def.outline.unwrap_or_else(|| dark(dark(legs.def.color)));
                let toes = legs.def.toes;
                let mut bones = vec![(hip, k, w(0), w(1))];
                match ankle {
                    Some(at) => bones.extend([(k, at, w(1), w(2)), (at, foot, w(2), w(3))]),
                    None => bones.push((k, foot, w(1), w(2))),
                }
                let fw = w(if ankle.is_some() { 3 } else { 2 });
                if toes > 0.0 {
                    bones.extend([(foot, foot + ahead * toes, fw, 1.0), (foot, foot - ahead * toes * 0.45 + Vec2::Y * 0.5, fw, 1.0)]);
                }
                if legs.def.style == LegStyle::Plate {
                    // Metal: plates, shaded under; a piston; bolts; a flat
                    // foot plate.
                    let shade = rgb(dark(if far { legs.def.far.unwrap_or(dark(legs.def.color)) } else { legs.def.color }));
                    let n = if ankle.is_some() { 3 } else { 2 };
                    for (pass, grow) in [(rgb(line), 1.0), (leg, 0.0)] {
                        for &(a, b, wa, _) in &bones[..n] {
                            plate(a, b, wa * 0.5 + grow, pass, None, to);
                        }
                    }
                    for &(a, b, wa, _) in &bones[..n] {
                        plate(a, b, wa * 0.5, leg, Some(shade), to);
                    }
                    let up = legs.turn(Vec2::Y);
                    let (pa, pb) = (hip.lerp(k, 0.45), bones[1].0.lerp(bones[1].1, 0.55));
                    let side = (pb - pa).perp().normalize_or(up) * (w(1) * 0.5 + 0.8);
                    stroke(pa + side, pb + side, 0.9, 0.9, rgb(line), to);
                    stroke(pa + side, pb + side, 0.5, 0.5, joint, to);
                    let toes = legs.def.toes;
                    if toes > 0.0 {
                        let (a, b) = (foot - ahead * toes * 0.45, foot + ahead * toes);
                        plate(a + Vec2::Y * 0.8, b + Vec2::Y * 0.8, 1.6, rgb(line), None, to);
                        plate(a + Vec2::Y * 0.8, b + Vec2::Y * 0.8, 1.0, leg, None, to);
                    }
                    for (at, wk) in [(Some(hip), w(0)), (Some(k), w(1)), (ankle, w(2))] {
                        if let Some(at) = at {
                            let r = (wk * 0.3).max(1.0);
                            stroke(at, at, r + 0.6, r + 0.6, rgb(line), to);
                            stroke(at, at, r, r, joint, to);
                        }
                    }
                    continue;
                }
                for (pass, grow) in [(rgb(line), 1.0), (leg, 0.0)] {
                    for &(a, b, wa, wb) in &bones {
                        stroke(a, b, wa * 0.5 + grow, wb * 0.5 + grow, pass, to);
                    }
                }
                // (Knuckles at the knee and the ankle.)
                for (at, wk) in [(Some(k), w(1)), (ankle, w(2))] {
                    if let Some(at) = at {
                        stroke(at, at, (wk * 0.25).max(0.8), (wk * 0.25).max(0.8), joint, to);
                    }
                }
                continue;
            }
            // Thick lines: the cells beside the line, across it.
            let mut thick = |a: Vec2, b: Vec2, t: u8, to: &mut Vec<(IVec2, [u8; 4])>| {
                line.clear();
                cells(a, b, &mut line);
                let d = b - a;
                let across = if d.x.abs() > d.y.abs() { IVec2::new(0, 1) } else { IVec2::new(1, 0) };
                for &p in &line {
                    for w in 0..t.max(1) as i32 {
                        to.push((p + across * (w - (t as i32 - 1) / 2), leg));
                    }
                }
            };
            thick(hip, k, legs.def.thick, to);
            thick(k, foot, legs.def.thick.saturating_sub(1), to);
            to.push((IVec2::new(k.x.floor() as i32, k.y.floor() as i32), joint));
        }
        // Arms, as the legs are drawn (where they are now: `walk`).
        for (arm, &(shoulder, elbow, wrist)) in legs.def.arms.iter().zip(&legs.arms) {
            let w = |i: usize| arm.width.get(i).or(arm.width.last()).copied().unwrap_or(2.0);
            let line = legs.def.outline.unwrap_or_else(|| dark(dark(legs.def.color)));
            let (fill, knob) = if arm.far {
                let c = legs.def.far.unwrap_or(dark(legs.def.color));
                (rgb(c), rgb(legs.def.joint.map_or(c, dark)))
            } else {
                (rgb(legs.def.color), rgb(legs.def.joint.unwrap_or(legs.def.color)))
            };
            let to = &mut quads[(!arm.far) as usize];
            for (pass, grow) in [(rgb(line), 1.0), (fill, 0.0)] {
                stroke(shoulder, elbow, w(0) * 0.5 + grow, w(1) * 0.5 + grow, pass, to);
                stroke(elbow, wrist, w(1) * 0.5 + grow, w(2) * 0.5 + grow, pass, to);
            }
            stroke(elbow, elbow, (w(1) * 0.25).max(0.8), (w(1) * 0.25).max(0.8), knob, to);
        }
        // Chains: tapered and outlined as the legs, rings across their
        // joints if they're segmented.
        for (def, ch) in legs.def.chains.iter().zip(&legs.chains) {
            let n = ch.pts.len();
            if n < 2 {
                continue;
            }
            let base = def.color.unwrap_or(legs.def.color);
            let fill = rgb(if def.far { dark(base) } else { base });
            let line = rgb(legs.def.outline.unwrap_or_else(|| dark(dark(base))));
            let to = &mut quads[(!def.far && !def.behind && side) as usize];
            let w = |i: usize| def.width_at(i as f32 / (n - 1) as f32);
            for (pass, grow) in [(line, 1.0), (fill, 0.0)] {
                for i in 1..n {
                    stroke(ch.pts[i - 1], ch.pts[i], w(i - 1) * 0.5 + grow, w(i) * 0.5 + grow, pass, to);
                }
            }
            if def.rings {
                for i in 1..n - 1 {
                    let across = (ch.pts[i + 1] - ch.pts[i - 1]).perp().normalize_or(Vec2::Y) * (w(i) * 0.5);
                    stroke(ch.pts[i] - across, ch.pts[i] + across, 0.5, 0.5, line, to);
                }
            }
        }
    }
    let [back, front, stage_back, stage_front] = &mut canvas.0;
    if let Some((centre, half)) = stage.0 {
        for (canvas, quads) in [(stage_back, &quads[0]), (stage_front, &quads[1])] {
            if let Some(mut px) = canvas.frame(&mut commands, &mut images, &mut sprites, centre, half, !quads.is_empty()) {
                for (p, c) in quads {
                    px.put(p.x, p.y, *c);
                }
            }
        }
    }
    let Ok((cam, loader)) = camera.single() else { return };
    for (canvas, quads) in [(back, &quads[0]), (front, &quads[1])] {
        if let Some(mut px) = canvas.frame(&mut commands, &mut images, &mut sprites, cam.translation().truncate(), loader.half_extent, !quads.is_empty()) {
            for (p, c) in quads {
                px.put(p.x, p.y, *c);
            }
        }
    }
}
