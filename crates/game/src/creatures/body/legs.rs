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
//!   never two neighbours at once; going, sooner (and when left behind),
//!   quicker, up to half at once, planting ahead of where it's going. With
//!   nothing in reach a leg hangs curled.
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
        app.init_resource::<Footfalls>()
            // (The bestiary's stage view: canvases may draw for it.)
            .init_resource::<crate::canvas::StageView>()
            .init_resource::<BodyArt>()
            .add_systems(Update, (turn_over, grow_legs, aims, walk, turrets, parts, footfalls, draw).chain().after(crate::creatures::body::animation::animate).after(TransformSystems::Propagate));
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
    /// Its name, for a move to drive it (`LimbPose`) or strike from it.
    #[serde(default)]
    pub name: Option<String>,
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
struct Footfalls(Vec<(Vec2, Footfall)>, Vec<(Vec2, bool)>);

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
    /// Its name, for a move to drive it (`LimbPose`: reach, open) or strike
    /// from it.
    #[serde(default)]
    pub name: Option<String>,
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
    /// Which legs strike (a move's `paw`: raised high ahead, then down
    /// hard); else, from above, its front pair.
    #[serde(default)]
    pub strikers: Vec<usize>,
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
    /// Another way to be seen, on the wall behind (a spider seen from the
    /// side on the ground, walls and ceilings; from above on the wall
    /// behind): it turns over to it when it holds on there, and back.
    #[serde(default)]
    pub back: Option<Box<LegsDef>>,
    /// Side: a body of segments trailing the head along the way it came
    /// (a centipede: up walls, along ceilings, round corners as it went),
    /// each a plate with a pair of legs stepping in a wave down it.
    #[serde(default)]
    pub segments: Option<SegmentsDef>,
    /// How its parts take blows (`parts.rs`): legs, arms, segments,
    /// chains, and weak spots on its body.
    #[serde(default)]
    pub parts: super::parts::PartsDef,
    /// Side: its turrets (aimed limbs: a strider's guns).
    #[serde(default)]
    pub turrets: Vec<TurretDef>,
}

/// A turret: a `barrel` sprite (pointing right, its `grip` the pivot)
/// pivoting at `at` on the body (cells from its grip, facing right, y up),
/// swinging toward its target at `turn` degrees a second (within `range`
/// cells; else it rests pointing ahead), its muzzle `length` along it; a
/// move fires from it (a `Cast`'s `from`) along where it points, not
/// straight at the target: outrun its swing. Each shot heats it (`heat`).
#[derive(Clone, Debug, Deserialize)]
pub struct TurretDef {
    pub name: String,
    pub at: (f32, f32),
    pub barrel: String,
    pub length: f32,
    #[serde(default = "turret_turn")]
    pub turn: f32,
    #[serde(default = "turret_range")]
    pub range: f32,
    #[serde(default)]
    pub far: bool,
    #[serde(default)]
    pub heat: HeatDef,
}

fn turret_turn() -> f32 {
    120.0
}
fn turret_range() -> f32 {
    320.0
}

/// How a turret heats: each shot adds `shot` (1: as hot as it goes), it
/// cools `cool` a second; at 1 it overheats and vents for `vent` seconds
/// (steam, a hiss; its fire moves wait: the player's opening), cooling
/// faster meanwhile. Drawn dull red, then orange, then white, glowing.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct HeatDef {
    pub shot: f32,
    pub cool: f32,
    pub vent: f32,
}

impl Default for HeatDef {
    fn default() -> Self {
        HeatDef { shot: 0.22, cool: 0.3, vent: 1.8 }
    }
}

/// A turret as it is now: where it points (the world, radians), how hot,
/// seconds of venting left, its sprites (barrel, its heat's glow, a sight
/// line, a charge's glow at the muzzle), its pivot and muzzle.
struct Turret {
    aim: f32,
    heat: f32,
    venting: f32,
    barrel: Option<Entity>,
    glow: Option<Entity>,
    sight: Entity,
    charge: Entity,
    pivot: Vec2,
    muzzle: Vec2,
    /// It's just overheated: a hiss to play.
    hiss: bool,
}

#[derive(Component)]
struct LegTurret;

#[derive(Component)]
struct LegTell;

/// A body of segments (`segments`): drawn as plates (outlined, lit along
/// the back, a belly under), each with a near and a far leg.
#[derive(Clone, Debug, Deserialize)]
pub struct SegmentsDef {
    pub count: usize,
    /// Cells from one segment to the next along its way; a segment's
    /// length and height.
    pub spacing: f32,
    pub size: (f32, f32),
    /// Its plates' colour, lit along its back, and its belly.
    pub color: (u8, u8, u8),
    #[serde(default)]
    pub lit: Option<(u8, u8, u8)>,
    #[serde(default)]
    pub belly: Option<(u8, u8, u8)>,
    /// A leg's reach (cells), and where its foot rests ahead of its hip
    /// (the near one's; the far one's half as far behind).
    pub reach: f32,
    #[serde(default = "seg_lean")]
    pub lean: f32,
    /// How far behind the segment ahead each one's legs step (a share of
    /// a step's cycle): the wave running down it.
    #[serde(default = "seg_wave")]
    pub wave: f32,
    /// The last segment's trailing feelers (cells long).
    #[serde(default)]
    pub tail: f32,
    /// The last segments smaller, down to this share at the end.
    #[serde(default = "full")]
    pub taper: f32,
}

fn seg_lean() -> f32 {
    3.0
}
fn seg_wave() -> f32 {
    0.12
}
fn full() -> f32 {
    1.0
}

/// A segment as it is now: where it is, which way the one ahead of it is
/// (along its way), the surface it's on (as the head was there), which way
/// it faces on it, its two feet (near, far).
#[derive(Clone, Copy, Debug)]
struct Seg {
    at: Vec2,
    /// Drawn this far from `at`, dying away (laid out afresh after a fall:
    /// it eases there, not jumps).
    ease: Vec2,
    dir: Vec2,
    surface: f32,
    facing: f32,
    feet: [Foot; 2],
}

impl SegmentsDef {
    /// Segment `s`'s size, against the first's (the end tapers).
    fn scale(&self, s: usize) -> f32 {
        let tail = 3.0f32.min(self.count as f32);
        let from = self.count as f32 - tail;
        if (s as f32) < from { 1.0 } else { 1.0 - (1.0 - self.taper) * ((s as f32 - from + 1.0) / tail) }
    }
}

/// How a legged body is held: a move's pose (`moves/`, a phase's `pose`,
/// eased into), the same thing: raised `lift` cells off what it holds,
/// drawn `back` cells behind its heading (a crouch), its stinger curled
/// `curl` of the way over its back and past its head.
#[derive(Component, Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Rear {
    pub lift: f32,
    pub back: f32,
    pub curl: f32,
    /// Its aiming chains: drawn back, coiled (0–1), and thrown out at the
    /// target (0–1: a sting striking).
    pub coil: f32,
    pub reach: f32,
    /// Its striking legs (`strikers`) raised ahead and up (0–1.2): a
    /// stamp's, a bite's threat.
    pub paw: f32,
    /// Side: its body tipped nose down (degrees; up: negative): a bite's
    /// head dipping, a rearing back.
    pub pitch: f32,
    /// Its named limbs (a leg's, an arm's `name`), each as a move holds it.
    pub limbs: Vec<LimbPose>,
    /// Where the move's target is (set as it runs): what arms reach for.
    #[serde(skip)]
    pub target: Option<Vec2>,
    /// What the move's phase shows (set as it runs: `moves`' `tells`),
    /// each how far into the phase (0–1).
    #[serde(skip)]
    pub tells: Vec<(Tell, f32)>,
}

/// A tell a move's phase shows (you see it coming): a turret's sight line
/// (a thin red line from its muzzle along where it points), its charge (a
/// glow at the muzzle, growing through the phase), a lock-on (a mark over
/// the target).
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub enum Tell {
    Sight(String),
    Charge(String),
    Lock,
}

/// A named limb as a move holds it: a leg `raise`d off the ground, ahead
/// and up (0–1.2: a stomp's wind-up; dropped back fast, it slams down); an
/// arm reached out at the target (0: at rest, 1: as far as it goes toward
/// it); a claw `open` (0: shut, 1: wide).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct LimbPose {
    pub name: String,
    pub raise: f32,
    pub reach: f32,
    pub open: f32,
}

impl Rear {
    /// From one pose toward another, `w` of the way (0–1), trembling
    /// `tremble` (at `t` seconds in: curl, coil ×4, paw ×3, a raised leg ×3
    /// and a claw ×3 shiver; a held stance).
    pub fn blend(from: &Rear, to: &Rear, w: f32, tremble: f32, t: f32) -> Rear {
        let mix = |a: f32, b: f32| a + (b - a) * w;
        let shake = tremble * (t * 40.0).sin();
        let mut names: Vec<&str> = from.limbs.iter().chain(&to.limbs).map(|l| l.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        let limbs = names
            .into_iter()
            .map(|name| {
                let (a, b) = (from.limb(name).cloned().unwrap_or_default(), to.limb(name).cloned().unwrap_or_default());
                LimbPose { name: name.to_string(), raise: mix(a.raise, b.raise) + shake * 3.0, reach: mix(a.reach, b.reach), open: mix(a.open, b.open) + shake * 3.0 }
            })
            .collect();
        Rear {
            lift: mix(from.lift, to.lift),
            back: mix(from.back, to.back),
            curl: mix(from.curl, to.curl) + shake,
            coil: mix(from.coil, to.coil) + shake * 4.0,
            reach: mix(from.reach, to.reach),
            paw: mix(from.paw, to.paw) + shake * 3.0,
            pitch: mix(from.pitch, to.pitch),
            limbs,
            target: from.target,
            tells: from.tells.clone(),
        }
    }

    /// Its named limb's pose, if the move holds it.
    pub fn limb(&self, name: &str) -> Option<&LimbPose> {
        self.limbs.iter().find(|l| l.name == name)
    }
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
    /// The legs that strike.
    pub fn strikers(&self) -> Vec<usize> {
        match (self.strikers.is_empty(), self.view) {
            (false, _) => self.strikers.clone(),
            (true, View::Above) => vec![0, 1],
            (true, View::Side) => Vec::new(),
        }
    }

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
    /// Where the foot is drawn, and how fast it's going there: it has
    /// weight, so it follows `at` (where the steps put it) as a damped
    /// spring, never faster than a leg can swing (`foot_speed`): no snap
    /// from one place to another in a frame, however the steps jump.
    shown: Vec2,
    speed: Vec2,
}

impl Foot {
    fn new(at: Vec2, grips: bool) -> Self {
        Foot { at, from: at, to: at, t: 1.0, grips, retry: 0.0, shown: at, speed: Vec2::ZERO }
    }
}

/// How long a drawn foot takes to catch up with where it should be
/// (seconds, about), and how fast a leg can swing (cells/s, for each cell
/// of its reach; on top of the body's own speed).
const FOOT_LAG: f32 = 0.06;
const FOOT_SPEED: f32 = 9.0;

/// `FOOT_LAG`, or `PLATYPUS_FOOTLAG` (0: none, to compare).
fn foot_lag() -> f32 {
    static LAG: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *LAG.get_or_init(|| std::env::var("PLATYPUS_FOOTLAG").ok().and_then(|v| v.parse().ok()).unwrap_or(FOOT_LAG))
}

/// `from` toward `to` as a critically damped spring taking about `lag`
/// seconds, no faster than `most` (Game Programming Gems 4's smooth damp:
/// steady for any frame time).
fn smooth_damp(from: Vec2, to: Vec2, speed: &mut Vec2, lag: f32, most: f32, dt: f32) -> Vec2 {
    let omega = 2.0 / lag.max(1e-4);
    let x = omega * dt;
    let decay = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
    let change = (from - to).clamp_length_max(most * lag);
    let target = from - change;
    let temp = (*speed + omega * change) * dt;
    *speed = (*speed - omega * temp) * decay;
    let mut out = target + (change + temp) * decay;
    // (Not past where it's going.)
    if (to - from).dot(out - to) > 0.0 {
        out = to;
        *speed = Vec2::ZERO;
    }
    out
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
    ride: Option<Vec2>,
    /// Side: the surface it's on, as a turn from the ground (radians: a
    /// quarter turn, a wall on its right; a half, the ceiling), eased
    /// round as it goes over onto it. Its body, legs and steps are all in
    /// that frame: its ground is under it whichever way that is.
    surface: f32,
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
    /// A readout of snaps: foot-frames seen, and of those, a foot that
    /// moved faster than a leg swings (`FOOT_SPEED` reaches a second,
    /// against the body): where the steps put it, and where it's drawn.
    /// (Last frame's of each, and of the body.)
    snaps: (u32, u32, u32),
    before: Vec<(Vec2, Vec2)>,
    before_body: Option<Vec2>,
    /// Seen as its def's `back` (on the wall behind), and how long it's
    /// been where the other view belongs (it turns over after a moment).
    on_back: bool,
    other: f32,
    /// Its segments (`segments`), the way its head came (where its body
    /// rode, the surface it was on there; newest first), and their steps'
    /// clock.
    segs: Vec<Seg>,
    trail: std::collections::VecDeque<(Vec2, f32)>,
    seg_phase: f32,
    /// Seconds its head has held something since it was put down (till
    /// it's settled, 0.3 s, its body is laid out afresh each frame: it
    /// doesn't trail its fall, or its body easing down onto its feet).
    settled: f32,
    /// Seconds it's been backing up onto its own way (taken back in, not
    /// doubled back on, till 0.4 s: then it turns), and a readout: frames
    /// seen, and of those, its body piled on itself (two segments two or
    /// more apart nearer than 0.6 of their spacing: doubled back).
    backing: f32,
    piled: (u32, u32),
    /// Seconds its head's held nothing (a fall: landing after one, its body
    /// is laid out behind it again).
    aloft: f32,
    /// Seconds it's stood with its body dangling.
    dangle: f32,
    /// Where its body's grip was this frame (the world: `parts`).
    grip: Vec2,
    /// Its turrets as they are now, and its lock-on mark (a reticle over
    /// its target while a move's phase shows `Tell::Lock`).
    turrets: Vec<Turret>,
    lock: Option<Entity>,
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
        (self.tilt.to_degrees(), self.ride.map_or(0.0, |r| r.y - bottom), self.feet.iter().filter(|f| f.grips && f.t >= 1.0).count())
    }

    /// How far its furthest foot is from its hip, against its reach (over
    /// 1: further than the leg can reach; it's drawn at full length).
    pub fn strain(&self) -> f32 {
        self.strain
    }

    /// Its segments' places (head first) and how many of their feet hold
    /// something.
    /// Its named turret's muzzle and the way it points (the world).
    pub fn muzzle(&self, name: &str) -> Option<(Vec2, Vec2)> {
        let j = self.def.turrets.iter().position(|t| t.name == name)?;
        self.turrets.get(j).map(|t| (t.muzzle, Vec2::from_angle(t.aim)))
    }

    /// Its named turret is venting (overheated: it can't fire).
    pub fn venting(&self, name: &str) -> bool {
        self.def.turrets.iter().position(|t| t.name == name).and_then(|j| self.turrets.get(j)).is_some_and(|t| t.venting > 0.0)
    }

    /// Its named turret fired: hotter; at its limit, it vents.
    pub fn fired(&mut self, name: &str) {
        let Some(j) = self.def.turrets.iter().position(|t| t.name == name) else { return };
        let h = self.def.turrets[j].heat.clone();
        if let Some(t) = self.turrets.get_mut(j) {
            t.heat += h.shot;
            if t.heat >= 1.0 && t.venting <= 0.0 {
                t.venting = h.vent;
                t.hiss = true;
            }
        }
    }

    /// Its turrets' heat (0–1 and on) and whether each is venting, in order
    /// (a readout).
    pub fn turret_heat(&self) -> Vec<(f32, bool)> {
        self.turrets.iter().map(|t| (t.heat, t.venting > 0.0)).collect()
    }

    /// Where its named limb ends (the world): a leg's foot, an arm's wrist.
    pub fn limb_end(&self, name: &str) -> Option<Vec2> {
        // (Where the foot is, not where it's drawn: that lags a fast slam.)
        let leg = self.def.each.iter().position(|l| l.name.as_deref() == Some(name)).and_then(|i| self.feet.get(i)).map(|f| f.at);
        leg.or_else(|| self.def.arms.iter().position(|a| a.name.as_deref() == Some(name)).and_then(|j| self.arms.get(j)).map(|a| a.2))
    }

    /// Frames its segments were seen, and how many of them its body was
    /// piled on itself.
    pub fn piled(&self) -> (u32, u32) {
        self.piled
    }

    pub fn segments(&self) -> (Vec<Vec2>, usize) {
        (self.segs.iter().map(|s| s.at).collect(), self.segs.iter().flat_map(|s| s.feet).filter(|f| f.grips && f.t >= 1.0).count())
    }

    /// The surface it's on (degrees from the ground: 90 a wall on its
    /// right, 180 the ceiling), and whether it's seen as its `back`.
    pub fn surface(&self) -> (f32, bool) {
        (self.surface.to_degrees(), self.on_back)
    }

    /// Foot-frames seen, and how many snapped (faster than a leg swings,
    /// against the body): where the steps put them, where they're drawn.
    pub fn snaps(&self) -> (u32, u32, u32) {
        self.snaps
    }

    /// Its chains' tips from their anchors, whether it's aiming, which way
    /// it faces.
    pub fn chain_state(&self) -> (Vec<Vec2>, bool, f32) {
        (self.chains.iter().filter_map(|c| Some(*c.pts.last()? - *c.pts.first()?)).collect(), self.aim.is_some(), self.facing)
    }

    /// Where its feet are (the world), and whether each holds something.
    pub fn feet(&self) -> impl Iterator<Item = (Vec2, bool)> + '_ {
        self.feet.iter().map(|f| (f.at, f.grips))
    }

    /// The body's frame to the world's, as a function (to hand on while
    /// its chains are borrowed): from the side, mirrored to its facing and
    /// tilted; from above, turned to its heading.
    fn frame(&self) -> impl Fn(Vec2) -> Vec2 + use<> {
        let (side, f, tilt, heading, surface) = (self.def.view == View::Side, self.facing, self.tilt, self.heading, self.surface);
        move |v: Vec2| if side { Vec2::from_angle(surface + tilt * f).rotate(Vec2::new(v.x * f, v.y)) } else { Vec2::from_angle(heading).rotate(v) }
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
    /// it's turned now (Side: mirrored to its facing, tilted, turned to
    /// the surface it's on).
    fn turn(&self, p: Vec2) -> Vec2 {
        Vec2::from_angle(self.surface + self.tilt * self.facing).rotate(Vec2::new(p.x * self.facing, p.y))
    }

    /// Side: the surface's frame (not the body's tilt): along it (the
    /// world's right, on the ground) and up off it.
    fn along(&self) -> Vec2 {
        Vec2::from_angle(self.surface)
    }
    fn off(&self) -> Vec2 {
        Vec2::from_angle(self.surface).perp()
    }

    /// Side: where leg `i`'s foot goes, wanting to be `ahead` cells along
    /// the surface from its hip: the surface under there, in the frame.
    fn ground_under(&self, sim: &SimWorld, i: usize, c: Vec2, ahead: f32, reach: f32) -> Option<Vec2> {
        ground_under(sim, self.along(), self.hip(i, c), ahead, reach)
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

/// Side: where a foot goes, wanting to be `ahead` cells along the
/// surface (`along`: its way, the world's right on the ground) from its
/// hip: the surface there if it's in reach, else nearer the hip (down a
/// steep slope the ground ahead is past its reach; nearer, it isn't).
fn ground_under(sim: &SimWorld, along: Vec2, hip: Vec2, ahead: f32, reach: f32) -> Option<Vec2> {
    (0..5).find_map(|k| ground_at(sim, along, hip, ahead * (1.0 - k as f32 / 5.0), reach))
}

/// The surface under the point `ahead` along it from the hip, within
/// reach: down (off the surface's way: `along` turned back a quarter)
/// from the hip's level (up out of rock first, a little: a foot uphill)
/// to the first solid cell; the foot on its face (on a cell's edge, the
/// surface square to the world's; else the last open point).
fn ground_at(sim: &SimWorld, along: Vec2, hip: Vec2, ahead: f32, reach: f32) -> Option<Vec2> {
    let down = -along.perp();
    let mut p = hip + along * ahead;
    let mut lifted = 0.0;
    while solid(sim, p) {
        p -= down;
        lifted += 1.0;
        if lifted > reach * 0.3 {
            return None;
        }
    }
    let mut gone = 0.0;
    while gone < reach * 1.05 + lifted {
        if solid(sim, p + down) {
            let foot = if down.y.abs() > 0.999 {
                Vec2::new(p.x, if down.y < 0.0 { p.y.floor() } else { p.y.floor() + 1.0 })
            } else if down.x.abs() > 0.999 {
                Vec2::new(if down.x < 0.0 { p.x.floor() } else { p.x.floor() + 1.0 }, p.y)
            } else {
                p
            };
            return ((foot - hip).length() <= reach * 1.02).then_some(foot);
        }
        p += down;
        gone += 1.0;
    }
    None
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

/// A body seen two ways (`back`) turns over when it goes onto the wall
/// behind or off it (after a moment there: not back and forth at an edge):
/// its legs and their sprites go, and grow again as the other.
fn turn_over(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Legs, &Kinematics, &Animator)>) {
    for (e, mut legs, k, anim) in &mut q {
        if anim.def.legs.as_ref().is_none_or(|d| d.back.is_none()) {
            continue;
        }
        let back = k.loco.clinging() == Some(Vec2::ZERO);
        if back == legs.on_back {
            legs.other = 0.0;
            continue;
        }
        legs.other += time.delta_secs();
        if legs.other < 0.12 {
            continue;
        }
        let parts = [Some(legs.body), legs.eyes, legs.stinger]
            .into_iter()
            .chain(legs.claws.iter().copied())
            .chain(legs.chains.iter().map(|c| c.tip))
            .chain(legs.turrets.iter().flat_map(|t| [t.barrel, t.glow, Some(t.sight), Some(t.charge)]))
            .chain([legs.lock]);
        for part in parts.flatten() {
            commands.entity(part).despawn();
        }
        commands.entity(e).remove::<Legs>();
    }
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
        // (On the wall behind: its view from there, if it has one.)
        let on_back = def.back.is_some() && k.loco.clinging() == Some(Vec2::ZERO);
        let def = match (on_back, def.back.clone()) {
            (true, Some(back)) => *back,
            _ => def,
        };
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
        let mut legs = Legs { feet: Vec::new(), heading: 0.0, body, eyes, stinger, offset: Vec2::ZERO, facing, tilt: 0.0, ride: None, surface: 0.0, slope: 0.0, arms: Vec::new(), claws: Vec::new(), air: 0.0, strain: 0.0, last: None, phase: 0.0, chains: Vec::new(), aim: None, snaps: (0, 0, 0), before: Vec::new(), before_body: None, on_back, other: 0.0, segs: Vec::new(), trail: std::collections::VecDeque::new(), seg_phase: 0.0, settled: 0.0, backing: 0.0, piled: (0, 0), aloft: 0.0, dangle: 0.0, grip: Vec2::ZERO, turrets: Vec::new(), lock: None, def };
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
        // Its turrets: a turned barrel each, its heat's glow over it (lit
        // over the dark, as eyes are), a sight line, a charge's glow.
        for td in legs.def.turrets.clone() {
            if !art.0.contains_key(&td.barrel) {
                match crate::combat::turned_art(&td.barrel, None, &mut images, &mut layouts) {
                    Ok(t) => {
                        art.0.insert(td.barrel.clone(), t);
                    }
                    Err(err) => warn!("legs: turret `{}`: {err}", td.barrel),
                }
            }
            let z = if td.far { Z_FAR_CLAW } else { Z_NEAR_CLAW } - root_z;
            let barrel = art.0.get(&td.barrel).map(|t| commands.spawn((LegTurret, t.sprite(0.0), Transform::from_xyz(0.0, 0.0, z))).id());
            let glow = art.0.get(&td.barrel).map(|t| {
                let mut s = t.sprite(0.0);
                s.color = Color::NONE;
                commands.spawn((LegTurret, s, Transform::from_xyz(0.0, 0.0, Z_EYES - root_z))).id()
            });
            let tell = |commands: &mut Commands, anchor: bevy::sprite::Anchor| commands.spawn((LegTell, Sprite::from_color(Color::NONE, Vec2::ONE), anchor, Transform::from_xyz(0.0, 0.0, Z_EYES - root_z + 0.01), Visibility::Hidden)).id();
            let (sight, charge) = (tell(&mut commands, bevy::sprite::Anchor::CENTER_LEFT), tell(&mut commands, bevy::sprite::Anchor::CENTER));
            for part in [barrel, glow, Some(sight), Some(charge)].into_iter().flatten() {
                commands.entity(e).add_child(part);
            }
            legs.turrets.push(Turret { aim: if facing < 0.0 { std::f32::consts::PI } else { 0.0 }, heat: 0.0, venting: 0.0, barrel, glow, sight, charge, pivot: k.body.pos, muzzle: k.body.pos, hiss: false });
        }
        // (With turrets, a lock-on mark too.)
        if !legs.def.turrets.is_empty() {
            let mark = commands.spawn((LegTell, Sprite::from_image(images.add(reticle())), Transform::from_xyz(0.0, 0.0, Z_EYES - root_z + 0.02), Visibility::Hidden)).id();
            commands.entity(e).add_child(mark);
            legs.lock = Some(mark);
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
                View::Side => legs.ground_under(&sim, i, c, legs.def.each[i].lean * facing, legs.def.full_reach()),
            };
            let at = held.unwrap_or(hip + Vec2::from_angle(legs.way(i)) * legs.def.full_reach() * 0.5);
            legs.feet.push(Foot::new(at, true));
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
        // (Further than a leg reaches and than it was going: a fast fall or
        // lunge in a long frame isn't being put down somewhere.)
        let jumped = legs.last.is_none_or(|l| l.distance(middle) > legs.def.full_reach() + k.body.vel.length() * dt * 2.0);
        if jumped {
            legs.ride = None;
            legs.tilt = 0.0;
            legs.slope = 0.0;
        }
        legs.last = Some(middle);
        let rear = rear.cloned().unwrap_or_default();
        let v = k.body.vel;
        // Where the body is: from above, raised off what it holds and
        // drawn back from its heading (an attack); from the side, riding
        // its feet, tilted with them.
        // (From the side, turned round: its body flips at once, so its
        // feet are planted afresh on the other side too.)
        // The surface it's on (from the side): the ground, or the wall or
        // ceiling it holds, eased round to as it goes over onto it.
        if side {
            let want = match k.loco.clinging() {
                Some(d) if d != Vec2::ZERO => d.y.atan2(d.x) + std::f32::consts::FRAC_PI_2,
                _ => 0.0,
            };
            let d = (want - legs.surface + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            legs.surface = if jumped { want } else { legs.surface + d.clamp(-10.0 * dt, 10.0 * dt) };
        }
        let (along, off) = (legs.along(), legs.off());
        let going = v.dot(along);
        // (Which way it faces: on the ground, its own; on a wall or the
        // ceiling, the way it goes along it.)
        // (A body of segments backing up onto its own way faces on the
        // way it was going, till it turns.)
        let backs = legs.def.segments.is_some() && legs.backing < 0.4 && legs.trail.len() > 2 && v.dot(legs.trail[0].0 - legs.trail[1].0) < -5.0;
        let facing = if backs {
            legs.facing
        } else if legs.surface.abs() < 0.3 {
            if k.loco.facing < 0.0 { -1.0 } else { 1.0 }
        } else if going.abs() > 5.0 {
            going.signum()
        } else {
            legs.facing
        };
        let turned = side && (facing < 0.0) != (legs.facing < 0.0);
        // (Put down somewhere else: drawn there at once. Turned round:
        // the feet planted afresh, but drawn swinging over.)
        let put = jumped;
        let jumped = jumped || turned;
        let (c, up) = if side {
            legs.facing = facing;
            let f = facing;
            // (All in the surface's frame: along it, and up off it, from
            // its middle.)
            let local = |p: Vec2| {
                let d = p - middle;
                Vec2::new(d.dot(along), d.dot(off))
            };
            let extent = off.x.abs() * k.body.half.x + off.y.abs() * k.body.half.y;
            let base = -extent;
            let planted: Vec<Vec2> = legs.feet.iter().filter(|foot| foot.grips).map(|foot| foot.to).collect();
            let ground = if planted.is_empty() { base } else { planted.iter().map(|p| local(*p).y).sum::<f32>() / planted.len() as f32 };
            // (Up while a foot's in the air: a walker on two legs bobs.)
            let up = legs.feet.iter().filter(|foot| foot.t < 1.0 && foot.grips).map(|foot| (std::f32::consts::PI * foot.t).sin()).fold(0.0, f32::max);
            let want = (ground + legs.def.ride + legs.def.bob * up).clamp(base + legs.def.ride * 0.5, extent);
            // (Eased: where it rode stays where it was in the world as its
            // box jolts.)
            let y = legs.ride.map_or(want, |r| {
                let y = local(r).y;
                y + (want - y) * (dt * 12.0).min(1.0)
            });
            let at = middle + off * y;
            legs.ride = Some(at);
            // Nose up as the ground its feet stand on rises ahead: the
            // slope of the line through its planted feet (along the way it
            // faces), once they're spread enough to say.
            let want = {
                let pts: Vec<Vec2> = planted
                    .iter()
                    .map(|p| {
                        let l = local(*p);
                        Vec2::new(l.x * f, l.y)
                    })
                    .collect();
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
            let want = want - (legs.def.pitch * (going.abs() / 100.0).min(1.5)).to_radians();
            let most = legs.def.tilt.to_radians();
            // (A move's pitch on top: a bite's head dipping, rearing back.)
            let want = want.clamp(-most, most) - rear.pitch.to_radians();
            legs.tilt += (want - legs.tilt) * (dt * 8.0).min(1.0);
            (at + off * rear.lift - along * f * rear.back, off)
        } else {
            let up = {
                let held: Vec<Vec2> = legs.feet.iter().filter(|f| f.grips).map(|f| f.at).collect();
                if held.is_empty() { Vec2::Y } else { (middle - held.iter().sum::<Vec2>() / held.len() as f32).normalize_or(Vec2::Y) }
            };
            let fwd = Vec2::from_angle(legs.heading);
            (middle + up * rear.lift - fwd * rear.back, up)
        };
        legs.offset = c - middle;
        legs.grip = c;
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
                    View::Side => legs.ground_under(&sim, i, c, legs.def.each[i].lean * legs.facing, legs.def.full_reach()),
                };
                let at = held.unwrap_or(hip + Vec2::from_angle(way) * legs.def.full_reach() * 0.5);
                legs.feet[i] = Foot::new(at, held.is_some());
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
        let (index, flip) = if side { (crate::combat::Turned::index((legs.tilt + legs.facing * legs.surface).to_degrees()), legs.facing < 0.0) } else { (crate::combat::Turned::index(legs.heading.to_degrees()), false) };
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
        // (From above, how fast it's going, 0–1 at 120 cells/s and on: the
        // faster, the sooner a leg steps, the quicker, the more at once, the
        // further ahead it plants: a running spider's legs keep up.)
        let pace = if side { 0.0 } else { (v.length() / 120.0).min(1.0) };
        let step_time = legs.def.step * (1.0 - 0.35 * pace);
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
            let speed = going.abs();
            let groups = legs.def.each.iter().map(|l| l.gait).max().unwrap_or(0) as f32 + 1.0;
            let duty = 0.68 - 0.26 * (speed / 150.0).min(1.0);
            let cycle = (legs.def.stride / duty).max(1.0);
            let dir = if speed > 5.0 { going.signum() } else { legs.facing };
            // (Where along the surface from its hip each foot's wanted.)
            let target = |legs: &Legs, i: usize| {
                let hip = legs.hip(i, c);
                (hip, legs.def.each[i].lean * legs.facing + dir * legs.def.stride * 0.5)
            };
            let behind = (0..n).any(|i| {
                let (hip, ahead) = target(&legs, i);
                legs.feet[i].t >= 1.0 && ((legs.feet[i].at - hip).dot(along) - ahead).abs() > legs.def.stride * 0.9
            });
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
                        && let Some(p) = ground_under(&sim, along, hip, x, reach)
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
                    match ground_under(&sim, along, hip, x, reach) {
                        Some(to) => {
                            foot.to = to;
                            foot.grips = true;
                        }
                        None => {
                            foot.to = open_toward(&sim, hip, hip - off * reach * 0.7);
                            foot.grips = false;
                        }
                    }
                    foot.t = s.min(0.999);
                    let e = s * s * (3.0 - 2.0 * s);
                    foot.at = foot.from.lerp(foot.to, e) + off * lift * (std::f32::consts::PI * s).sin();
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
            let rest = side.then(|| legs.def.each[i].lean * legs.facing + going * step_time);
            let off = f.at - hip;
            let due = match rest {
                Some(x) => !f.grips && f.retry <= 0.0 || f.grips && ((off.dot(along) - x).abs() > legs.def.stride || off.length() > reach * 0.98),
                None => {
                    let twisted = off.length() > 0.75 && Vec2::from_angle(way).dot(off.normalize()) < 0.2;
                    let stretched = off.length() > reach * (0.98 - 0.35 * pace);
                    let crowded = off.length() < reach * 0.25;
                    // (Going, a foot left well behind its hip steps on.)
                    let behind = pace > 0.2 && off.dot(v.normalize_or_zero()) < -reach * 0.35;
                    if f.grips { stretched || crowded || twisted || behind } else { f.retry <= 0.0 }
                }
            };
            // (From above, not with a neighbour, nor more than a third at
            // once; from the side, not while another gait group steps.)
            let busy = if side {
                let g = legs.def.each[i].gait;
                legs.feet.iter().zip(&legs.def.each).any(|(o, l)| o.t < 1.0 && l.gait != g)
            } else {
                let neighbours = [(i + n - 2) % n, (i + 2) % n];
                let most = if pace > 0.3 { n / 2 } else { n.div_ceil(3) };
                neighbours.iter().any(|&j| legs.feet[j].t < 1.0) || stepping >= most
            };
            legs.feet[i].retry -= dt;
            // (A foot pulled past its reach lets go now, its gait or its
            // neighbours or not: else, from above, a leg whose neighbours
            // keep taking the turns is dragged on and on.)
            let torn = f.grips && off.length() > reach;
            if !due || busy && !torn {
                continue;
            }
            let to = match rest {
                Some(x) => ground_under(&sim, along, hip, x, reach),
                // (From where its hip will be when the foot comes down, and a
                // little on: going, it plants ahead.)
                None => foothold(&sim, hip + v * step_time * 1.8, way, reach, back),
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
        // Striking legs (a move's `paw`): raised high ahead, over its head;
        // as `paw` drops back they come down ahead, hard, onto the ground
        // there (they let go: the next step plants them again).
        // (And a named leg a move raises: a stomp's.)
        let raised: Vec<(usize, f32)> = legs
            .def
            .strikers()
            .into_iter()
            .map(|i| (i, rear.paw))
            .chain(legs.def.each.iter().enumerate().filter_map(|(i, l)| l.name.as_deref().and_then(|name| rear.limb(name)).map(|p| (i, p.raise))))
            .filter(|&(i, p)| p > 0.01 && i < n)
            .collect();
        if !raised.is_empty() {
            let ahead = if side { along * legs.facing } else { fwd };
            for (i, p) in raised {
                let hip = legs.hip(i, c);
                let p = p.clamp(0.0, 1.2);
                // (Within its reach, so its knee bends; from above, the pair
                // spread apart, one to each side.)
                let spread = if side { Vec2::ZERO } else { ahead.perp() * if i.is_multiple_of(2) { 1.0 } else { -1.0 } * reach * 0.22 * p };
                let high = hip + ahead * reach * (0.55 - 0.1 * p) + up * reach * (0.5 * p - 0.2) + spread;
                // (From the side, coming down onto the ground ahead as it's
                // let down: raised past 0.6, held high; under that, lower
                // and lower to its spot there, so a stomp lands under it.)
                let low = side.then(|| ground_under(&sim, along, hip, ahead.dot(along) * reach * 0.45, reach)).flatten();
                // (From the side, lifted straight up off its spot ahead:
                // three quarters of its hip's height, or a third of its
                // reach for a long-legged low body (a spider's, up over
                // its head); not kicked up past its hip.)
                let at = match low {
                    Some(g) => {
                        let height = ((hip - g).dot(up) * 0.75).max(reach * 0.35);
                        g + up * height * p + ahead * reach * 0.1 * p
                    }
                    None => high,
                };
                let foot = &mut legs.feet[i];
                foot.at = open_toward(&sim, hip, at);
                foot.to = foot.at;
                foot.t = 1.0;
                foot.grips = false;
                foot.retry = 0.05;
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
            // (A move reaching it out at its target: as far toward it as the
            // arm goes; a claw opened, turned up off the forearm.)
            let pose = arm.name.as_deref().and_then(|name| rear.limb(name));
            let hand = match (pose, rear.target) {
                (Some(p), Some(t)) if p.reach > 0.01 => {
                    let most = shoulder + (t - shoulder).clamp_length_max(arm.bones.0 + arm.bones.1 - 0.5);
                    hand.lerp(most, p.reach.clamp(0.0, 1.0))
                }
                _ => hand,
            };
            let open = pose.map_or(0.0, |p| p.open.clamp(-0.3, 1.3));
            let (elbow, wrist) = knee(shoulder, hand, arm.bones.0, arm.bones.1, legs.bend(arm.elbow, 1.0), |p| solid(&sim, p));
            legs.arms.push((shoulder, elbow, wrist));
            if let Some(Ok((mut s, mut t))) = legs.claws.get(j).copied().flatten().map(|e| claws.get_mut(e)) {
                let d = wrist - elbow;
                let local = d.y.atan2(d.x * legs.facing) + open * 35f32.to_radians();
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
        // Its segments (a centipede's body): along the way its head came.
        if let Some(sd) = legs.def.segments.clone() {
            // (After where the head goes, not its pose: a bite's rearing
            // back and thrusting on isn't the way it came.)
            let base = c - (up * rear.lift - along * legs.facing * rear.back);
            segments(&mut legs, &sd, &sim, base, c - base, put, grounded, v, dt);
        }
        // The drawn feet after where the steps put them, with weight.
        let most = v.length() + reach * FOOT_SPEED;
        let lag = foot_lag();
        let l = &mut *legs;
        for f in l.feet.iter_mut().chain(l.segs.iter_mut().flat_map(|s| s.feet.iter_mut())) {
            if put || lag <= 0.0 {
                f.shown = f.at;
                f.speed = Vec2::ZERO;
            } else {
                f.shown = smooth_damp(f.shown, f.at, &mut f.speed, lag, most, dt);
            }
        }
        // (A readout: feet that went faster than a leg swings, against
        // the body: 9 reaches a second.)
        let moved = legs.before_body.map(|b| c - b);
        if !put
            && let Some(body) = moved
            && legs.before.len() == n
            && dt > 0.0
        {
            for i in 0..n {
                let (a, b) = legs.before[i];
                let most = reach * FOOT_SPEED * dt;
                legs.snaps.0 += 1;
                legs.snaps.1 += ((legs.feet[i].at - a - body).length() > most) as u32;
                legs.snaps.2 += ((legs.feet[i].shown - b - body).length() > most) as u32;
            }
        }
        legs.before = legs.feet.iter().map(|f| (f.at, f.shown)).collect();
        legs.before_body = Some(c);
    }
}

/// A segmented body after its head: the way the head came kept (where its
/// body rode, a point a cell, and the surface it was on), each segment
/// `spacing` on from the last along it, turned to the one ahead, on the
/// surface the head was on there (round a corner as it went round it);
/// each segment's two legs step on a clock that goes round as the body
/// goes, each segment's a `wave` of a cycle behind the one ahead (the
/// near and far legs half a cycle apart), planted under where they rest
/// on that surface, swinging there in an arc off it; with nothing in
/// reach a foot hangs. Put down somewhere: laid out straight behind it.
/// (`c`: where the head goes, not posed; `pose`: how far a move's pose
/// has it from there.)
#[allow(clippy::too_many_arguments)]
fn segments(legs: &mut Legs, sd: &SegmentsDef, sim: &SimWorld, c: Vec2, pose: Vec2, put: bool, holds: bool, v: Vec2, dt: f32) {
    let count = sd.count;
    let total = sd.spacing * (count as f32 + 1.0) + 8.0;
    let (along, surface, facing) = (legs.along(), legs.surface, legs.facing);
    if put {
        legs.settled = 0.0;
    }
    // (Down after a fall, a quarter second or more: laid out behind it
    // again, along the ground, each segment easing there from where it
    // fell: a body flopping down after its head, not a heap where it
    // landed.)
    // (Its body dangling: segments with no rock near them, any way round.)
    let near_rock = |p: Vec2| {
        let hold = legs.def.ride as i32 + 10;
        [Vec2::NEG_Y, Vec2::Y, Vec2::X, Vec2::NEG_X, Vec2::new(0.7, 0.7), Vec2::new(-0.7, 0.7), Vec2::new(0.7, -0.7), Vec2::new(-0.7, -0.7)].iter().any(|d| (1..=hold).any(|k| solid(sim, p + *d * k as f32)))
    };
    let dangling = legs.segs.iter().filter(|s| !near_rock(s.at)).count();
    // Down after a fall with some of its body dangling; or standing with
    // much of it dangling a while (draped off a ledge it leapt from, a
    // column it dropped down): laid out again.
    legs.dangle = if holds && v.length() < 20.0 && dangling >= 4 { legs.dangle + dt } else { 0.0 };
    let landed = holds && (legs.aloft > 0.1 && dangling >= 3 || legs.dangle > 0.5) && legs.settled >= 0.3 && !legs.segs.is_empty();
    if landed {
        legs.dangle = 0.0;
    }
    legs.aloft = if holds { 0.0 } else { legs.aloft + dt };
    let fell: Option<Vec<Vec2>> = landed.then(|| legs.segs.iter().map(|s| s.at + s.ease).collect());
    let decay = (-dt / 0.12).exp();
    for seg in &mut legs.segs {
        seg.ease *= decay;
    }
    if legs.settled < 0.3 || legs.trail.is_empty() || landed {
        if !landed {
            legs.settled = if holds { legs.settled + dt } else { 0.0 };
        }
        // (Laid along the surface under it, riding as high off it as the
        // head does: not in the air where it was put down.)
        legs.trail.clear();
        let off = along.perp();
        let ride = legs.def.ride;
        // (Behind the head; after a fall, on the side the body came down
        // on, not swung over the head to the other.)
        // (Its body's side of it; above it (a column it dropped down), the
        // side it came from: behind the way it was going.)
        let back = match &fell {
            Some(f) if !f.is_empty() => {
                let x = (f.iter().sum::<Vec2>() / f.len() as f32 - c).dot(along);
                let going = v.dot(along);
                along * if x.abs() > 4.0 { x.signum() } else if going.abs() > 20.0 { -going.signum() } else { -facing }
            }
            _ => -along * facing,
        };
        let mut d = 1.0;
        while d <= total {
            let p = c + back * d;
            let p = ground_at(sim, along, p + off * ride, 0.0, ride * 2.0 + 40.0).map_or(p, |g| g + off * ride);
            legs.trail.push_back((p, surface));
            d += 1.0;
        }
        if !landed {
            legs.segs.clear();
        }
    }
    // Backing up onto its own way (a lunge braked, a step back, knocked
    // back): the way taken back in at the head, the body sliding back after
    // it and its tail going on straight the way it lay; not doubled back on
    // (its body piled up on itself). Backing on past 0.4 s, it turns: the
    // way doubles back.
    let mut backed = false;
    while legs.backing < 0.4 && legs.trail.len() > 2 {
        let (f0, f1) = (legs.trail[0].0, legs.trail[1].0);
        let fwd = (f0 - f1).normalize_or_zero();
        // (Back the way it came, not across it: going round a corner it's
        // partly back toward the face, and that's no backing up.)
        if (c - f0).dot(fwd) >= -0.5 * c.distance(f0) {
            break;
        }
        legs.trail.pop_front();
        let gone = f0.distance(legs.trail[0].0);
        let n = legs.trail.len();
        let ((e0, s0), (e1, _)) = (legs.trail[n - 1], legs.trail[n - 2]);
        legs.trail.push_back((e0 + (e0 - e1).normalize_or(-fwd) * gone, s0));
        backed = true;
    }
    // (The way only grows on ahead: creeping back, it isn't laid behind
    // the head, unless it's turning.)
    let ahead = legs.trail.len() < 2 || (c - legs.trail[0].0).dot((legs.trail[0].0 - legs.trail[1].0).normalize_or_zero()) >= -0.5 * c.distance(legs.trail[0].0);
    if legs.trail.front().is_none_or(|(p, _)| p.distance(c) >= 1.0) && (ahead || legs.backing >= 0.4) {
        legs.trail.push_front((c, surface));
        if !backed {
            legs.backing = 0.0;
        }
    }
    if backed || !ahead {
        legs.backing += dt;
    }
    // (No hooks in it near the head: a point the way turns back at by more
    // than ~110° dropped (a hop come down a little behind where it went up,
    // a jolt back): a body following a hook piles on itself. Corners onto
    // walls and ceilings, a quarter turn, stay.)
    let mut i = 1;
    while i + 1 < legs.trail.len().min(24) {
        let (a, b, n) = (legs.trail[i - 1].0, legs.trail[i].0, legs.trail[i + 1].0);
        if (a - b).normalize_or_zero().dot((b - n).normalize_or_zero()) < -0.35 {
            legs.trail.remove(i);
            i = i.saturating_sub(1).max(1);
        } else {
            i += 1;
        }
    }
    // (No more of the way than the body needs.)
    let mut gone = c.distance(legs.trail[0].0);
    let mut keep = legs.trail.len();
    for i in 1..legs.trail.len() {
        gone += legs.trail[i - 1].0.distance(legs.trail[i].0);
        if gone > total {
            keep = i + 1;
            break;
        }
    }
    legs.trail.truncate(keep);
    // (What of it hangs in the open falls: a point with no rock near it
    // comes down, 220 cells/s, upright, till it's on something:
    // a body leapt off a ledge, or dropped from the ceiling, falls after
    // its head and doesn't hang in the air along the way it went.)
    // (Held: rock beside it or over it within its ride and 14 (round a
    // corner it's going over, the rock's diagonal to it, further than its
    // ride); under it, within its ride and four, and it settles down onto
    // that, to its ride, gently; else it falls.)
    let ride = legs.def.ride;
    let far = ride as i32 + 14;
    let near = ride as i32 + 4;
    let beside = [Vec2::Y, Vec2::X, Vec2::NEG_X, Vec2::new(0.7, 0.7), Vec2::new(-0.7, 0.7)];
    let under = [Vec2::new(0.7, -0.7), Vec2::new(-0.7, -0.7)];
    for i in 1..legs.trail.len() {
        let (p, s0) = legs.trail[i];
        if let Some(k) = (1..=near).find(|k| solid(sim, p - Vec2::Y * *k as f32)) {
            let over = k as f32 - 1.0 - ride;
            if over > 0.5 {
                legs.trail[i] = (p - Vec2::Y * over.min(60.0 * dt), s0);
            }
            continue;
        }
        if beside.iter().any(|d| (1..=far).any(|k| solid(sim, p + *d * k as f32))) || under.iter().any(|d| (1..=near).any(|k| solid(sim, p + *d * k as f32))) {
            continue;
        }
        legs.trail[i] = (p - Vec2::Y * (220.0 * dt).min(2.0), 0.0);
    }
    // (No long gaps in it: points a cell apart where it's stretched (a
    // point left on a ledge, the rest fallen), so what's between falls too:
    // the body drapes down off the ledge, not a pole through the air.)
    let mut i = 0;
    while i + 1 < legs.trail.len() && legs.trail.len() < 400 {
        let ((a, sa), (b, _)) = (legs.trail[i], legs.trail[i + 1]);
        if a.distance(b) > 2.0 {
            legs.trail.insert(i + 1, (a + (b - a).normalize_or_zero(), sa));
        }
        i += 1;
    }
    // (Never shorter than the body: a fall bunches the way up; its tail end
    // goes on along the surface behind the head, and falls in its turn.)
    let needed = sd.spacing * (count as f32 + 1.0);
    let mut len = legs.trail.front().map_or(0.0, |(p, _)| c.distance(*p)) + legs.trail.iter().zip(legs.trail.iter().skip(1)).map(|(a, b)| a.0.distance(b.0)).sum::<f32>();
    while len < needed && legs.trail.len() >= 2 {
        let n = legs.trail.len();
        let ((e0, s0), (e1, _)) = (legs.trail[n - 1], legs.trail[n - 2]);
        let _ = e1;
        // (Along the surface behind the head, not on up a column it fell
        // down: that would grow back as fast as it falls.)
        let p = e0 - along * facing;
        legs.trail.push_back((p, s0));
        len += e0.distance(p);
    }
    // A point `d` along the way back from the head, and the surface there.
    let pts: Vec<(Vec2, f32)> = std::iter::once((c, surface)).chain(legs.trail.iter().copied()).collect();
    let place = |d: f32| -> (Vec2, f32) {
        let mut left = d;
        for w in pts.windows(2) {
            let ((a, sa), (b, sb)) = (w[0], w[1]);
            let len = a.distance(b);
            if left <= len && len > 0.0 {
                let t = left / len;
                let turn = (sb - sa + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                return (a.lerp(b, t), sa + turn * t);
            }
            left -= len;
        }
        let (end, s) = *pts.last().unwrap_or(&(c, surface));
        (end - along * facing * left, s)
    };
    let fresh = legs.segs.len() != count;
    if fresh {
        legs.segs = (0..count).map(|_| Seg { at: c, ease: Vec2::ZERO, dir: along * facing, surface, facing, feet: [Foot::new(c, false); 2] }).collect();
    }
    // (The head's pose, a bite's rearing back, carried down its first few
    // segments, less and less: its front draws back with it.)
    let mut ahead = c + pose;
    for s in 0..count {
        let (at, surf) = place(sd.spacing * (s as f32 + 1.0));
        let at = at + pose * (1.0 - s as f32 / 4.0).max(0.0);
        let seg = &mut legs.segs[s];
        seg.dir = (ahead - at).normalize_or(seg.dir);
        seg.at = at;
        seg.surface = surf;
        // (Which way it faces on its surface: the way the one ahead is.)
        let x = seg.dir.dot(Vec2::from_angle(surf));
        if x.abs() > 0.2 {
            seg.facing = x.signum();
        }
        ahead = at;
    }
    if let Some(fell) = fell {
        for (seg, was) in legs.segs.iter_mut().zip(fell) {
            seg.ease = was - seg.at;
        }
    }
    // (A readout: is it piled on itself?)
    let piled = (0..count).any(|a| (a + 2..count).any(|b| legs.segs[a].at.distance(legs.segs[b].at) < sd.spacing * 0.6));
    legs.piled.0 += 1;
    legs.piled.1 += piled as u32;
    // Their steps: a clock going round as the body goes (a cycle every
    // stride a planted foot covers, over the share it's planted).
    let speed = v.length();
    let going = speed > 5.0;
    let duty = 0.68 - 0.26 * (speed / 150.0).min(1.0);
    let stride = sd.spacing * 1.1;
    legs.seg_phase = (legs.seg_phase + speed / (stride / duty).max(1.0) * dt).fract();
    let (phase, reach, lift) = (legs.seg_phase, sd.reach, legs.def.lift.min(sd.reach * 0.4));
    for s in 0..count {
        let scale = sd.scale(s);
        let seg = &mut legs.segs[s];
        let (a, o, f) = (Vec2::from_angle(seg.surface), Vec2::from_angle(seg.surface).perp(), seg.facing);
        let hip = seg.at - o * sd.size.1 * 0.3 * scale;
        for k in 0..2 {
            let lean = if k == 0 { sd.lean } else { -sd.lean * 0.5 } * f;
            let want = lean + if going { f * stride * 0.5 } else { 0.0 };
            let foot = &mut seg.feet[k];
            if fresh {
                let at = ground_under(sim, a, hip, lean, reach);
                *foot = Foot::new(at.unwrap_or(hip - o * reach * 0.6), at.is_some());
                continue;
            }
            let p = (phase + s as f32 * sd.wave + k as f32 * 0.5).fract();
            if p < duty || !going {
                // Planted (where its swing was going); slipped past its
                // reach, or holding nothing: put down again.
                if foot.t < 1.0 {
                    foot.t = 1.0;
                    foot.at = foot.to;
                }
                if !foot.grips || (foot.at - hip).length() > reach {
                    match ground_under(sim, a, hip, lean, reach) {
                        Some(to) => *foot = Foot { at: to, from: foot.at, to, t: 1.0, grips: true, ..*foot },
                        None => {
                            foot.grips = false;
                            foot.to = open_toward(sim, hip, hip - o * reach * 0.6 + a * lean * 0.5);
                            foot.at = foot.at.lerp(foot.to, (dt * 8.0).min(1.0));
                        }
                    }
                }
            } else {
                // Swinging, an arc up off its surface to where it'll land.
                if foot.t >= 1.0 {
                    foot.from = foot.at;
                }
                let sw = (p - duty) / (1.0 - duty);
                match ground_under(sim, a, hip, want, reach) {
                    Some(to) => {
                        foot.to = to;
                        foot.grips = true;
                    }
                    None => {
                        foot.to = open_toward(sim, hip, hip - o * reach * 0.6 + a * lean * 0.5);
                        foot.grips = false;
                    }
                }
                foot.t = sw.min(0.999);
                let e = sw * sw * (3.0 - 2.0 * sw);
                foot.at = foot.from.lerp(foot.to, e) + o * lift * (std::f32::consts::PI * sw).sin();
            }
            // (A foot is never further than its leg reaches.)
            if (foot.at - hip).length() > reach {
                foot.at = hip + (foot.at - hip).normalize_or(-o) * reach;
            }
        }
    }
}

/// A lock-on mark: a ring with four ticks pointing in, white (tinted when
/// drawn), 17 cells across.
fn reticle() -> Image {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let n = 17u32;
    let c = (n as f32 - 1.0) * 0.5;
    let mut px = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (x as f32 - c, y as f32 - c);
            let r = (dx * dx + dy * dy).sqrt();
            // (The ring, broken at the four ticks; the ticks in toward the
            // middle.)
            let tick = (dx.abs() < 0.5 || dy.abs() < 0.5) && (3.5..7.5).contains(&r);
            let ring = (6.5..7.6).contains(&r) && !(dx.abs() < 1.5 || dy.abs() < 1.5);
            let a = if tick || ring { 255 } else { 0 };
            px.extend([255, 255, 255, a]);
        }
    }
    Image::new(Extent3d { width: n, height: n, depth_or_array_layers: 1 }, TextureDimension::D2, px, TextureFormat::Rgba8UnormSrgb, bevy::asset::RenderAssetUsages::RENDER_WORLD)
}

/// How hot a turret looks: dull red, then orange, then white, more and
/// more opaque (0 at rest, a glow as it heats).
fn heat_color(h: f32) -> Color {
    let h = h.clamp(0.0, 1.2);
    let (r, g, b) = if h < 0.4 {
        (0.75, 0.12, 0.05)
    } else if h < 0.75 {
        (1.0, 0.45, 0.1)
    } else {
        (1.0, 0.92, 0.75)
    };
    Color::srgba(r, g, b, ((h - 0.05) * 1.1).clamp(0.0, 0.92))
}

/// Turrets: each swings toward its target at its turn rate (its rest, ahead,
/// with none in range), cools (faster venting), steams while it vents;
/// drawn: the barrel turned to where it points, its heat's glow over it,
/// and what the move's phase shows (`Rear::tells`): a sight line from the
/// muzzle along where it points to what's in the way, a glow growing at the
/// muzzle as it charges.
#[allow(clippy::type_complexity)]
fn turrets(
    time: Res<Time>,
    sim: Res<SimWorld>,
    mut q: Query<(&mut Legs, Option<&Rear>)>,
    mut sprites: Query<(&mut Sprite, &mut Transform, &mut Visibility), Or<(With<LegTurret>, With<LegTell>)>>,
    mut falls: ResMut<Footfalls>,
) {
    let dt = time.delta_secs().min(0.05);
    let now = time.elapsed_secs();
    for (mut legs, rear) in &mut q {
        if legs.turrets.is_empty() {
            continue;
        }
        let rear = rear.cloned().unwrap_or_default();
        let (c, middle, facing) = (legs.grip, legs.grip - legs.offset, legs.facing);
        let rest = legs.turn(Vec2::X).to_angle();
        let (aim_at, defs) = (legs.aim, legs.def.turrets.clone());
        for (j, td) in defs.iter().enumerate() {
            let pivot = c + legs.turn(Vec2::new(td.at.0, td.at.1));
            let want = match aim_at {
                Some(t) if t.distance(pivot) <= td.range => (t - pivot).to_angle(),
                _ => rest,
            };
            let t = &mut legs.turrets[j];
            let d = (want - t.aim + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            let most = td.turn.to_radians() * dt;
            t.aim += d.clamp(-most, most);
            t.pivot = pivot;
            t.muzzle = pivot + Vec2::from_angle(t.aim) * td.length;
            let cool = td.heat.cool * if t.venting > 0.0 { 3.0 } else { 1.0 };
            t.heat = (t.heat - cool * dt).max(0.0);
            if t.venting > 0.0 {
                t.venting -= dt;
                // (Steam off it, a puff every few frames.)
                if (now * 30.0).fract() < 0.5 {
                    falls.1.push((t.muzzle, false));
                }
            }
            if std::mem::take(&mut t.hiss) {
                falls.1.push((t.muzzle, true));
            }
        }
        for (j, td) in defs.iter().enumerate() {
            let t = &legs.turrets[j];
            let dir = Vec2::from_angle(t.aim);
            let local = dir.y.atan2(dir.x * facing);
            let index = crate::combat::Turned::index(local.to_degrees());
            let at = t.pivot - middle;
            for (e, glow) in [(t.barrel, false), (t.glow, true)] {
                if let Some(Ok((mut s, mut tr, _))) = e.map(|e| sprites.get_mut(e)) {
                    if let Some(atlas) = s.texture_atlas.as_mut() {
                        atlas.index = index;
                    }
                    s.flip_x = facing < 0.0;
                    if glow {
                        s.color = heat_color(t.heat);
                    }
                    tr.translation = at.extend(tr.translation.z);
                }
            }
            // Its sight line: to the first solid on the way, or its range.
            let sight = rear.tells.iter().find(|(tell, _)| *tell == Tell::Sight(td.name.clone()));
            if let Ok((mut s, mut tr, mut vis)) = sprites.get_mut(t.sight) {
                match sight {
                    Some(_) => {
                        let len = (1..td.range as i32).find(|k| solid(&sim, t.muzzle + dir * *k as f32)).map_or(td.range, |k| k as f32);
                        let flicker = 0.5 + 0.5 * (now * 37.0).sin();
                        s.custom_size = Some(Vec2::new(len, 0.6));
                        s.color = Color::srgba(1.0, 0.12, 0.08, 0.3 + 0.25 * flicker);
                        tr.translation = (t.muzzle - middle).extend(tr.translation.z);
                        tr.rotation = Quat::from_rotation_z(t.aim);
                        *vis = Visibility::Inherited;
                    }
                    None => *vis = Visibility::Hidden,
                }
            }
            let charge = rear.tells.iter().find(|(tell, _)| *tell == Tell::Charge(td.name.clone())).map(|(_, f)| *f);
            if j == 0
                && let Some(mark) = legs.lock
                && let Ok((mut s, mut tr, mut vis)) = sprites.get_mut(mark)
            {
                // Its lock-on mark: over the target, closing in and turning
                // as the lock completes, blinking.
                match (rear.tells.iter().find(|(tell, _)| *tell == Tell::Lock), rear.target) {
                    (Some((_, f)), Some(at)) => {
                        let blink = if (now * 9.0).fract() < 0.6 { 1.0 } else { 0.35 };
                        s.color = Color::srgba(1.0, 0.15, 0.1, 0.9 * blink);
                        tr.translation = (at - middle).extend(tr.translation.z);
                        tr.rotation = Quat::from_rotation_z(now * 2.5);
                        tr.scale = Vec3::splat(1.8 - 0.8 * f.min(1.0));
                        *vis = Visibility::Inherited;
                    }
                    _ => *vis = Visibility::Hidden,
                }
            }
            if let Ok((mut s, mut tr, mut vis)) = sprites.get_mut(t.charge) {
                match charge {
                    Some(f) => {
                        let size = 1.0 + 3.5 * f + 0.5 * (now * 50.0).sin().abs();
                        s.custom_size = Some(Vec2::splat(size));
                        s.color = Color::srgba(1.0, 0.35 + 0.6 * f, 0.25 + 0.6 * f, 0.45 + 0.5 * f);
                        tr.translation = (t.muzzle - middle).extend(tr.translation.z);
                        tr.rotation = Quat::from_rotation_z(t.aim + now * 6.0);
                        *vis = Visibility::Inherited;
                    }
                    None => *vis = Visibility::Hidden,
                }
            }
        }
    }
}

/// Its parts' hit areas as they are now (`parts.rs`): each leg hip to
/// knee to foot, each arm, each segment, each chain's links, each weak
/// spot; a share of a blow to its body each (its `parts`).
fn parts(mut commands: Commands, mut q: Query<(Entity, &Legs, Option<&mut super::parts::Parts>)>) {
    use super::parts::{Part, PartKind};
    for (e, legs, mine) in &mut q {
        let def = &legs.def;
        let pd = &def.parts;
        let c = legs.grip;
        let side = def.view == View::Side;
        let mut list = Vec::new();
        let w = |i: usize| def.width.get(i).or(def.width.last()).copied().unwrap_or(def.thick as f32) * 0.5 + 0.5;
        let (a, b) = (def.reach * def.upper, def.reach * (1.0 - def.upper));
        for (i, f) in legs.feet.iter().enumerate() {
            let hip = legs.hip(i, c);
            let bend = if side {
                let l = &def.each[i];
                legs.bend(l.knee, if l.lean != 0.0 { l.lean } else { l.hip.0 })
            } else {
                Vec2::Y
            };
            let (k, foot) = knee(hip, f.at, a, b, bend, |_| false);
            list.push(Part { a: hip, b: k, r: w(0), mult: pd.legs, kind: PartKind::Leg });
            list.push(Part { a: k, b: foot, r: w(1), mult: pd.legs, kind: PartKind::Leg });
        }
        for (arm, &(shoulder, elbow, wrist)) in def.arms.iter().zip(&legs.arms) {
            let r = arm.width.first().copied().unwrap_or(2.0) * 0.5 + 0.5;
            list.push(Part { a: shoulder, b: elbow, r, mult: pd.arms, kind: PartKind::Arm });
            list.push(Part { a: elbow, b: wrist, r, mult: pd.arms, kind: PartKind::Arm });
        }
        if let Some(sd) = &def.segments {
            for (s, seg) in legs.segs.iter().enumerate() {
                let scale = sd.scale(s);
                let half = seg.dir * sd.size.0 * scale * 0.5;
                list.push(Part { a: seg.at - half, b: seg.at + half, r: sd.size.1 * scale * 0.5 + 0.5, mult: pd.segments, kind: PartKind::Segment });
            }
        }
        for (cd, ch) in def.chains.iter().zip(&legs.chains) {
            let n = ch.pts.len();
            for i in 1..n {
                let r = cd.width_at(i as f32 / (n - 1).max(1) as f32) * 0.5 + 0.5;
                list.push(Part { a: ch.pts[i - 1], b: ch.pts[i], r, mult: pd.chains, kind: PartKind::Chain });
            }
        }
        for wd in &pd.weak {
            let at = c + legs.turn(Vec2::new(wd.at.0, wd.at.1));
            list.push(Part { a: at, b: at, r: wd.r, mult: wd.mult, kind: PartKind::Weak });
        }
        match mine {
            Some(mut p) => p.set(list),
            None => {
                let mut p = super::parts::Parts::default();
                p.set(list);
                commands.entity(e).insert(p);
            }
        }
    }
}

/// Where a creature's aiming chains and turrets reach for: the nearest
/// thing it hunts within the furthest of their ranges (its middle), or
/// nothing.
fn aims(mut legs: Query<(&mut Legs, &Kinematics)>, hunted: Query<(&Kinematics, &crate::creatures::Team)>) {
    for (mut l, k) in &mut legs {
        let Some(range) = l.def.chains.iter().filter_map(|c| c.aims).chain(l.def.turrets.iter().map(|t| t.range)).reduce(f32::max) else {
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
    // (A turret venting: steam off it; just overheated, a hiss.)
    let steam = sim.world.materials().id("steam");
    for (at, hiss) in std::mem::take(&mut falls.1) {
        if hiss {
            sounds.write(crate::sound::PlaySound::at("vent", at));
        }
        if let Some(steam) = steam {
            sim.world.puff([at.x, at.y], platypus_sim::Cell::new(steam, 0), if hiss { 10 } else { 2 }, 26.0);
        }
    }
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
/// ends); with `under`, only its underside's band in that colour (shaded:
/// the side away from `up`).
#[allow(clippy::too_many_arguments)]
fn plate(a: Vec2, b: Vec2, r: f32, c: [u8; 4], under: Option<[u8; 4]>, out: &mut Vec<(IVec2, [u8; 4])>, up: Vec2) {
    let d = b - a;
    let len = d.length().max(0.01);
    let dir = d / len;
    let n = dir.perp();
    let down = if n.dot(up) > 0.0 { -n } else { n };
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

/// A side-view creature's own limb images (behind its body, in front),
/// children of it: its legs, arms and chains drawn in its own frame, so
/// they move with it smoothly.
#[derive(Component, Default)]
struct LimbImages([Layer; 2]);

/// One of them: its sprite, its image, which of the creature's cells its
/// bottom-left is, its size (cells).
#[derive(Default)]
struct Layer {
    sprite: Option<Entity>,
    image: Handle<Image>,
    origin: IVec2,
    size: UVec2,
}

#[derive(Component)]
struct LimbLayer;

/// The limb images' sprites.
type LayerSprites<'w, 's> = Query<'w, 's, (&'static mut Sprite, &'static mut Transform), (With<LimbLayer>, Without<crate::canvas::CanvasSprite>)>;

impl Layer {
    /// This frame's cells (the creature's own: from where it's drawn) onto
    /// the image: made bigger when they don't fit (a margin round them,
    /// the size in steps of 16: not made again every frame), placed so its
    /// cells sit where they're meant to.
    #[allow(clippy::too_many_arguments)]
    fn paint(&mut self, commands: &mut Commands, images: &mut Assets<Image>, sprites: &mut LayerSprites, parent: Entity, cells: &[(IVec2, [u8; 4])], z: f32) {
        let lo = cells.iter().fold(IVec2::MAX, |m, (p, _)| m.min(*p));
        let hi = cells.iter().fold(IVec2::MIN, |m, (p, _)| m.max(*p));
        if !cells.is_empty() {
            let need = (hi - lo + IVec2::ONE).as_uvec2();
            let fits = self.sprite.is_some() && lo.cmpge(self.origin).all() && (hi - self.origin).as_uvec2().cmplt(self.size).all();
            if !fits {
                if need.cmpgt(self.size).any() || self.sprite.is_none() {
                    self.size = ((need + UVec2::splat(8)) / 16 + UVec2::ONE) * 16;
                    self.image = images.add(Image::new(
                        bevy::render::render_resource::Extent3d { width: self.size.x, height: self.size.y, depth_or_array_layers: 1 },
                        bevy::render::render_resource::TextureDimension::D2,
                        vec![0; (self.size.x * self.size.y * 4) as usize],
                        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
                    ));
                }
                // (Its cells in the middle of it.)
                self.origin = (lo + hi) / 2 - (self.size / 2).as_ivec2();
            }
        }
        let at = (self.origin.as_vec2() + self.size.as_vec2() / 2.0).extend(z);
        match self.sprite.and_then(|s| sprites.get_mut(s).ok()) {
            Some((mut s, mut t)) => {
                if s.image != self.image {
                    s.image = self.image.clone();
                    s.custom_size = Some(self.size.as_vec2());
                }
                t.translation = at;
            }
            None if !cells.is_empty() => {
                let s = commands.spawn((LimbLayer, Sprite { image: self.image.clone(), custom_size: Some(self.size.as_vec2()), ..default() }, Transform::from_translation(at))).id();
                commands.entity(parent).add_child(s);
                self.sprite = Some(s);
            }
            None => {}
        }
        let Some(mut img) = images.get_mut(&self.image) else { return };
        let (w, h) = (self.size.x as i32, self.size.y as i32);
        let Some(data) = img.data.as_mut() else { return };
        data.fill(0);
        for (p, c) in cells {
            let (x, y) = (p.x - self.origin.x, p.y - self.origin.y);
            if x < 0 || y < 0 || x >= w || y >= h {
                continue;
            }
            let i = (((h - 1 - y) * w + x) * 4) as usize;
            data[i..i + 4].copy_from_slice(c);
        }
    }
}

/// In front of a creature's body (10.02), under its eyes.
const Z_NEAR_LEGS: f32 = 10.06;


/// Every leg, a cell at a time, onto the canvases.
#[allow(clippy::too_many_arguments)]
fn draw(
    mut commands: Commands,
    sim: Res<SimWorld>,
    mut images: ResMut<Assets<Image>>,
    mut q: Query<(Entity, &Legs, &GlobalTransform, Option<&mut LimbImages>)>,
    mut layers: LayerSprites,
) {
    // (Behind, in front.)
    let mut own: [Vec<(IVec2, [u8; 4])>; 2] = [Vec::new(), Vec::new()];
    let mut line = Vec::new();
    for (e, legs, tf, mine) in &mut q {
        // Drawn in its own frame (`o`: where the creature's drawn), onto its
        // own images that move with it: its limbs glide as its body does,
        // not a cell at a time on the world's grid.
        let side = legs.def.view == View::Side;
        let o = tf.translation().truncate();
        own[0].clear();
        own[1].clear();
        let quads = &mut own;
        // (The body where it's drawn: rearing lifts the hips.)
        let c = tf.translation().truncate() + legs.offset - o;
        let rgb = |(r, g, b): (u8, u8, u8)| [r, g, b, 255];
        let dark = |(r, g, b): (u8, u8, u8)| ((r as f32 * 0.6) as u8, (g as f32 * 0.6) as u8, (b as f32 * 0.6) as u8);
        let (a, b) = (legs.def.reach * legs.def.upper, legs.def.reach * (1.0 - legs.def.upper));
        let held: Vec<Vec2> = legs.feet.iter().filter(|f| f.grips).map(|f| f.shown - o).collect();
        let up = if side {
            legs.turn(Vec2::Y)
        } else if held.is_empty() {
            Vec2::Y
        } else {
            (c - held.iter().sum::<Vec2>() / held.len() as f32).normalize_or(Vec2::Y)
        };
        for (i, f) in legs.feet.iter().enumerate() {
            let hip = legs.hip(i, c);
            let f = Foot { at: f.shown - o, ..*f };
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
            // (Along the surface it's on, and up off it.)
            let (ahead, lift) = (legs.along() * legs.facing, legs.off());
            let ankle = (legs.def.ankle > 0.0).then(|| f.at + (-ahead * legs.def.heel.to_radians().cos() + lift * legs.def.heel.to_radians().sin()) * legs.def.ankle);
            // (Drawn as solved: past its reach the leg is at full length
            // and the foot comes with it, never stretched.)
            let (k, end) = knee(hip, ankle.unwrap_or(f.at), a, b, bend, |p| solid(&sim, p + o));
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
                    bones.extend([(foot, foot + ahead * toes, fw, 1.0), (foot, foot - ahead * toes * 0.45 + lift * 0.5, fw, 1.0)]);
                }
                if legs.def.style == LegStyle::Plate {
                    // Metal: plates, shaded under; a piston; bolts; a flat
                    // foot plate.
                    let shade = rgb(dark(if far { legs.def.far.unwrap_or(dark(legs.def.color)) } else { legs.def.color }));
                    let n = if ankle.is_some() { 3 } else { 2 };
                    for (pass, grow) in [(rgb(line), 1.0), (leg, 0.0)] {
                        for &(a, b, wa, _) in &bones[..n] {
                            plate(a, b, wa * 0.5 + grow, pass, None, to, lift);
                        }
                    }
                    for &(a, b, wa, _) in &bones[..n] {
                        plate(a, b, wa * 0.5, leg, Some(shade), to, lift);
                    }
                    let up = legs.turn(Vec2::Y);
                    let (pa, pb) = (hip.lerp(k, 0.45), bones[1].0.lerp(bones[1].1, 0.55));
                    let side = (pb - pa).perp().normalize_or(up) * (w(1) * 0.5 + 0.8);
                    stroke(pa + side, pb + side, 0.9, 0.9, rgb(line), to);
                    stroke(pa + side, pb + side, 0.5, 0.5, joint, to);
                    let toes = legs.def.toes;
                    if toes > 0.0 {
                        let (a, b) = (foot - ahead * toes * 0.45, foot + ahead * toes);
                        plate(a + lift * 0.8, b + lift * 0.8, 1.6, rgb(line), None, to, lift);
                        plate(a + lift * 0.8, b + lift * 0.8, 1.0, leg, None, to, lift);
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
        // Segments, from the tail up (the nearer the head, the more on
        // top): the far leg behind, the plate (rounded, outlined, lit along
        // its back, its belly darker), the near leg in front; the last one's
        // feelers trailing.
        if let Some(sd) = &legs.def.segments {
            let line = rgb(legs.def.outline.unwrap_or_else(|| dark(dark(sd.color))));
            let (fill, lit, belly) = (rgb(sd.color), rgb(sd.lit.unwrap_or(sd.color)), rgb(sd.belly.unwrap_or_else(|| dark(sd.color))));
            let (near, far) = (rgb(legs.def.color), rgb(legs.def.far.unwrap_or(dark(legs.def.color))));
            let knob = rgb(legs.def.joint.unwrap_or(legs.def.color));
            let bones = (sd.reach * 0.5, sd.reach * 0.5);
            let [behind, front] = quads;
            for (s, seg) in legs.segs.iter().enumerate().rev() {
                let scale = sd.scale(s);
                let (a, up) = (Vec2::from_angle(seg.surface), Vec2::from_angle(seg.surface).perp());
                let at = seg.at + seg.ease - o;
                let hip = at - up * sd.size.1 * 0.3 * scale;
                let leg = |k: usize, to: &mut Vec<(IVec2, [u8; 4])>| {
                    let foot = seg.feet[k].shown - o;
                    let lean = if k == 0 { 1.0 } else { -1.0 } * seg.facing;
                    let bend = (up + a * lean * 0.5).normalize_or(up);
                    let (kn, end) = knee(hip, foot, bones.0, bones.1, bend, |p| solid(&sim, p + o));
                    let c = if k == 0 { near } else { far };
                    stroke(hip, kn, 0.8, 0.65, c, to);
                    stroke(kn, end, 0.65, 0.45, c, to);
                    to.push((IVec2::new(kn.x.floor() as i32, kn.y.floor() as i32), if k == 0 { knob } else { c }));
                };
                leg(1, behind);
                let (len, r) = (sd.size.0 * scale, sd.size.1 * 0.5 * scale);
                let (pa, pb) = (at - seg.dir * len * 0.5, at + seg.dir * len * 0.5);
                stroke(pa, pb, r + 1.0, r + 1.0, line, behind);
                stroke(pa, pb, r, r, fill, behind);
                let inset = seg.dir * r * 0.6;
                plate(pa + inset, pb - inset, r, fill, Some(belly), behind, up);
                plate(pa + inset, pb - inset, r, fill, Some(lit), behind, -up);
                if s + 1 == legs.segs.len() && sd.tail > 0.0 {
                    for lift in [0.3, -0.1] {
                        let tip = pa - seg.dir * sd.tail + up * sd.tail * lift;
                        stroke(pa, tip, 0.7, 0.4, near, behind);
                    }
                }
                leg(0, front);
            }
        }
        // Arms, as the legs are drawn (where they are now: `walk`).
        for (arm, &(shoulder, elbow, wrist)) in legs.def.arms.iter().zip(&legs.arms) {
            let (shoulder, elbow, wrist) = (shoulder - o, elbow - o, wrist - o);
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
            let pts: Vec<Vec2> = ch.pts.iter().map(|p| *p - o).collect();
            let base = def.color.unwrap_or(legs.def.color);
            let fill = rgb(if def.far { dark(base) } else { base });
            let line = rgb(legs.def.outline.unwrap_or_else(|| dark(dark(base))));
            let to = &mut quads[(!def.far && !def.behind && side) as usize];
            let w = |i: usize| def.width_at(i as f32 / (n - 1) as f32);
            for (pass, grow) in [(line, 1.0), (fill, 0.0)] {
                for i in 1..n {
                    stroke(pts[i - 1], pts[i], w(i - 1) * 0.5 + grow, w(i) * 0.5 + grow, pass, to);
                }
            }
            if def.rings {
                for i in 1..n - 1 {
                    let across = (pts[i + 1] - pts[i - 1]).perp().normalize_or(Vec2::Y) * (w(i) * 0.5);
                    stroke(pts[i] - across, pts[i] + across, 0.5, 0.5, line, to);
                }
            }
        }
        {
            let z = tf.translation().z;
            match mine {
                Some(mut mine) => {
                    for (layer, (cells, at)) in mine.0.iter_mut().zip([(&own[0], 9.5), (&own[1], Z_NEAR_LEGS)]) {
                        layer.paint(&mut commands, &mut images, &mut layers, e, cells, at - z);
                    }
                }
                None => {
                    commands.entity(e).insert(LimbImages::default());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drawn_foot_has_weight_it_follows_never_faster_than_a_leg_swings_and_never_past() {
        // Its foot put 40 cells off in one go (a step jumping): drawn, it
        // goes there over frames, never faster than it may, and stops there.
        let (mut at, to, mut speed) = (Vec2::ZERO, Vec2::new(40.0, 0.0), Vec2::ZERO);
        let dt = 1.0 / 60.0;
        let most = 300.0;
        let mut frames = 0;
        while at.distance(to) > 0.05 && frames < 120 {
            let next = smooth_damp(at, to, &mut speed, FOOT_LAG, most, dt);
            assert!(next.distance(at) <= most * dt * 1.01, "{} cells in a frame", next.distance(at));
            assert!(next.x <= to.x + 1e-3, "past where it goes: {next}");
            at = next;
            frames += 1;
        }
        assert!((8..60).contains(&frames), "caught up in {frames} frames");
        // (Held still, it stays.)
        speed = Vec2::ZERO;
        let still = smooth_damp(to, to, &mut speed, FOOT_LAG, most, dt);
        assert_eq!(still, to);
    }
}
