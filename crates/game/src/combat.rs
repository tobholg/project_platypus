//! Melee (SPEC 6.2): weapons held in the hand, swung by moves written as
//! data (`assets/data/weapons.ron`), hitting pixel against pixel.
//!
//! - A weapon is a sprite (`assets/art`, pointing right, a `grip` anchor),
//!   turned ahead of time to 64 angles RotSprite-style
//!   (`platypus_art::rotate`). Those pictures are drawn at the hand, and are
//!   what hits.
//! - Anything with a brain asks for a swing with a `MeleeRequest` (the
//!   player's hands on the left button; enemies' AI later); what it holds
//!   is its `Wielding`. A swing winds up, sweeps and recovers; pressing
//!   during it queues the next move of the combo.
//! - While it sweeps, the blade is stepped from last tick's angle to this
//!   one a turn at a time (no tunnelling), and every blade pixel is tested
//!   against every body near it: against the pixel of its frame there (a
//!   rigged creature), or its box. Each is hit once a swing: damage,
//!   knockback and stun, a moment of hit-stop, a shake, sparks; struck
//!   downward in the air, the swinger bounces up (a pogo). The blade cuts
//!   grass it passes through and rings off stone.
//! - Stamina pays for swings and the dodge (the dash), which makes you
//!   untouchable for a moment.

use std::collections::{BTreeMap, HashMap};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use platypus_art::Pixels;
use platypus_sim::{CellPos, Kind, WorldEdit};
use serde::Deserialize;

use crate::actors::animation::{Aiming, Animator, HandPos, pixel_at};
use crate::actors::{Health, Kinematics, Team};
use crate::data::{Watched, assets_dir, data_path, load_ron};
use crate::magic::runes::Emitter;
use crate::vfx::Sparks;
use crate::world::{SimWorld, TICK_HZ, TickSet};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MeleeRequest>()
            .add_message::<Hit>()
            .add_message::<Recoil>()
            .add_message::<Dashed>()
            .init_resource::<HitStop>()
            .add_systems(Startup, load)
            .add_systems(PreUpdate, hit_stop)
            .add_systems(FixedUpdate, (start_swings, dodge, stamina).chain().after(TickSet::Intent).before(TickSet::Bodies))
            .add_systems(FixedUpdate, (touch, swing, apply_hits).chain().after(TickSet::Bodies).before(TickSet::Cells))
            .add_systems(Update, (reload, draw).chain().after(crate::actors::animation::animate));
    }
}

const DT: f32 = 1.0 / TICK_HZ as f32;
/// Angles each weapon is drawn at, all the way round.
const TURNS: usize = 64;
const STEP: f32 = 360.0 / TURNS as f32;
/// A dodge's cost and how long it keeps you untouchable (s).
pub const DODGE_COST: f32 = 18.0;
const DODGE_SAFE: f32 = 0.25;
/// Hit-stop: seconds the world all but stops on a hit (heavier: longer).
const STOP: f32 = 0.055;
const STOP_SPEED: f32 = 0.03;

// ---- data ----

#[derive(Clone, Debug, Deserialize)]
struct WeaponsFile {
    combo_gap: f32,
    pogo: f32,
    hit: Emitter,
    clang: Emitter,
    trail: Emitter,
    weapons: Vec<WeaponDef>,
    #[serde(default)]
    bows: Vec<BowDef>,
    #[serde(default)]
    arrow: Option<ArrowDef>,
    #[serde(default)]
    held: BTreeMap<String, HeldDef>,
}

/// How an item of an icon shape is held (`held` in weapons.ron): `rest`
/// degrees idle, in its icon's colours with `recolor`, burning with `burns`,
/// swung like a blade whenever it's used with `swing`.
#[derive(Clone, Debug, Deserialize)]
pub struct HeldDef {
    pub rest: f32,
    #[serde(default)]
    pub recolor: bool,
    #[serde(default)]
    pub burns: bool,
    #[serde(default)]
    pub swing: Option<SwingDef>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SwingDef {
    pub damage: f32,
    pub knock: f32,
    pub stun: f32,
    pub moves: Vec<MoveDef>,
}

/// A bow: drawn fully in `draw` s; loosed, the arrow goes from the first
/// to the second of `speed`, `damage` and `knock` by how far it was drawn.
/// (It shows only while drawn: slung, it would hide the one holding it.)
#[derive(Clone, Debug, Deserialize)]
pub struct BowDef {
    pub id: String,
    pub art: String,
    pub draw: f32,
    pub speed: (f32, f32),
    pub damage: (f32, f32),
    pub knock: (f32, f32),
    pub stun: f32,
    pub stamina: f32,
}

/// Arrows: their sprite (a `grip` in the middle, a `tip`), how fast they
/// fall (cells/s²), how long one stays stuck, how near you pick it up, how
/// long a burning one burns.
#[derive(Clone, Debug, Deserialize)]
pub struct ArrowDef {
    pub art: String,
    pub gravity: f32,
    pub stuck: f32,
    pub pickup: f32,
    pub burn: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WeaponDef {
    pub id: String,
    pub art: String,
    pub damage: f32,
    pub knock: f32,
    pub stun: f32,
    pub rest: f32,
    pub moves: Vec<MoveDef>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MoveDef {
    pub name: String,
    pub from: f32,
    pub to: f32,
    #[serde(default)]
    pub thrust: f32,
    pub windup: f32,
    pub active: f32,
    pub recovery: f32,
    pub stamina: f32,
    #[serde(default)]
    pub lunge: f32,
    #[serde(default = "one")]
    pub damage: f32,
    #[serde(default = "one")]
    pub knock: f32,
}

fn one() -> f32 {
    1.0
}

impl MoveDef {
    fn length(&self) -> f32 {
        self.windup + self.active + self.recovery
    }

    /// The blade's angle from the aim (degrees) and how far it's thrust,
    /// `t` seconds in, starting from `start` (where it was held).
    fn pose(&self, t: f32, start: f32) -> (f32, f32) {
        let ease = |x: f32| x * x * (3.0 - 2.0 * x);
        if t < self.windup {
            let k = ease(t / self.windup.max(1e-3));
            (start + (self.from - start) * k, 0.0)
        } else if t < self.windup + self.active {
            let k = (t - self.windup) / self.active.max(1e-3);
            (self.from + (self.to - self.from) * ease(k), self.thrust * (k * std::f32::consts::PI).sin())
        } else {
            (self.to, 0.0)
        }
    }

    fn active_at(&self, t: f32) -> bool {
        t >= self.windup && t < self.windup + self.active
    }
}

/// A weapon's picture at every angle (squares with the grip in the middle
/// pixel), and the atlas they're drawn from.
pub(crate) struct Turned {
    frames: Vec<Pixels>,
    side: u32,
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

impl Turned {
    pub(crate) fn index(angle: f32) -> usize {
        (((angle + 180.0) / STEP).round() as i64).rem_euclid(TURNS as i64) as usize
    }

    /// The blade's pixels at `angle` (from level, facing right), as offsets
    /// from the grip in cells (y up; mirrored facing left).
    pub(crate) fn cells(&self, angle: f32, facing: f32) -> impl Iterator<Item = Vec2> + '_ {
        let f = &self.frames[Self::index(angle)];
        let half = (self.side / 2) as i32;
        (0..self.side as i32).flat_map(move |y| (0..self.side as i32).map(move |x| (x, y))).filter(|&(x, y)| f.opaque(x, y)).map(move |(x, y)| Vec2::new((x - half) as f32 * facing, -(y - half) as f32))
    }
}

/// A frame of a sprite (by name; the first if `None`) turned to every
/// angle about its `grip` anchor.
fn turn_art(art: &str, frame: Option<&str>) -> Result<Vec<Pixels>, String> {
    turn_compiled(&compile_art(art, None)?, art, frame)
}

/// A sprite compiled, its colours for these letters swapped for others (an
/// item's icon colours: one pickaxe drawing, every tier).
fn compile_art(art: &str, colors: Option<&HashMap<char, (u8, u8, u8)>>) -> Result<platypus_art::Art, String> {
    let path = assets_dir().join("art").join(format!("{art}.ron"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut file = platypus_art::parse(&text)?;
    for (c, &(r, g, b)) in colors.into_iter().flatten() {
        if let Some(slot) = file.palette.get_mut(c) {
            *slot = platypus_art::Color::Rgb(r, g, b);
        }
    }
    platypus_art::compile(&file)
}

fn turn_compiled(compiled: &platypus_art::Art, art: &str, frame: Option<&str>) -> Result<Vec<Pixels>, String> {
    let i = match frame {
        Some(name) => compiled.index(name).ok_or(format!("{art}: no frame `{name}`"))?,
        None => 0,
    };
    let grip = compiled.anchors.get("grip").and_then(|m| m.get(&i)).copied().ok_or(format!("{art}: no `grip` anchor"))?;
    Ok((0..TURNS).map(|k| platypus_art::rotate::rotsprite(&compiled.frames[i], grip, -180.0 + k as f32 * STEP)).collect())
}

/// What icons.ron says of an item: its shape and colours.
#[derive(Clone, Debug, Deserialize)]
struct IconLook {
    shape: String,
    palette: HashMap<char, (u8, u8, u8)>,
}

#[derive(Clone, Debug, Deserialize)]
struct IconsLook {
    icons: HashMap<String, IconLook>,
}

/// Held things that aren't blades or bows: pointed where they're used,
/// resting otherwise; a torch burns in the hand.
pub(crate) struct Pointer {
    pub id: String,
    pub rest: f32,
    pub burns: bool,
    /// The flame's place from the grip (cells, y up, pointing right).
    pub flame: Option<Vec2>,
}

/// Any sprite's first frame turned to every angle about its `grip` (a
/// spider's body seen from above); with `only`, just its pixels of that
/// colour (its eyes, to draw over the dark).
pub(crate) fn turned_art(art: &str, only: Option<[u8; 3]>, images: &mut Assets<Image>, layouts: &mut Assets<TextureAtlasLayout>) -> Result<Turned, String> {
    let mut compiled = compile_art(art, None)?;
    if let Some(c) = only {
        for f in &mut compiled.frames {
            for px in f.rgba.chunks_mut(4) {
                if px[..3] != c {
                    px.copy_from_slice(&[0, 0, 0, 0]);
                }
            }
        }
    }
    Ok(Turned::build(turn_compiled(&compiled, art, None)?, images, layouts))
}

/// A weapon's picture for its icon: a blade pointing up and to the right, a
/// bow as it's drawn.
pub fn icon(id: &str) -> Option<Pixels> {
    let file: WeaponsFile = load_ron(&data_path("weapons.ron")).ok()?;
    if let Some(def) = file.weapons.iter().find(|w| w.id == id) {
        return turn_art(&def.art, None).ok().map(|f| f[Turned::index(45.0)].clone());
    }
    let bow = file.bows.iter().find(|b| b.id == id)?;
    turn_art(&bow.art, Some("rest")).ok().map(|f| f[Turned::index(0.0)].clone())
}

impl Turned {
    fn build(frames: Vec<Pixels>, images: &mut Assets<Image>, layouts: &mut Assets<TextureAtlasLayout>) -> Turned {
        let side = frames[0].w;
        let cols = 8u32;
        let rows = (TURNS as u32).div_ceil(cols);
        let mut atlas = Pixels::new(side * cols, side * rows);
        for (k, f) in frames.iter().enumerate() {
            let (ox, oy) = ((k as u32 % cols) * side, (k as u32 / cols) * side);
            for y in 0..side {
                for x in 0..side {
                    atlas.set((ox + x) as i32, (oy + y) as i32, f.get(x as i32, y as i32));
                }
            }
        }
        let image = images.add(Image::new(
            Extent3d { width: atlas.w, height: atlas.h, depth_or_array_layers: 1 },
            TextureDimension::D2,
            atlas.rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        ));
        let layout = layouts.add(TextureAtlasLayout::from_grid(UVec2::splat(side), cols, rows, None, None));
        Turned { frames, side, image, layout }
    }

    /// The sprite (atlas frame) pointing at `angle`.
    pub(crate) fn sprite(&self, angle: f32) -> Sprite {
        Sprite::from_atlas_image(self.image.clone(), TextureAtlas { layout: self.layout.clone(), index: Self::index(angle) })
    }
}

/// Every weapon's pictures: blades; bows at rest and drawn; the arrow.
struct Built {
    blades: Vec<WeaponDef>,
    pointers: Vec<(Pointer, Turned)>,
    turned: Vec<Turned>,
    bows: Vec<[Turned; 2]>,
    arrow: Option<Turned>,
}

#[derive(Resource)]
pub struct Weapons {
    file: WeaponsFile,
    /// Blades: weapons.ron's own, then every tool that swings (by item id).
    blades: Vec<WeaponDef>,
    pub(crate) pointers: Vec<(Pointer, Turned)>,
    turned: Vec<Turned>,
    pub(crate) bows: Vec<[Turned; 2]>,
    pub(crate) arrow: Option<Turned>,
    watch: Watched,
    art_watch: Watched,
}

impl Weapons {
    pub fn index(&self, id: &str) -> Option<usize> {
        self.blades.iter().position(|w| w.id == id)
    }

    pub fn def(&self, i: usize) -> &WeaponDef {
        &self.blades[i]
    }

    pub fn pointer_index(&self, id: &str) -> Option<usize> {
        self.pointers.iter().position(|p| p.0.id == id)
    }

    /// Anything held by this id (a blade, a bow, a tool, a wand...).
    pub fn knows(&self, id: &str) -> bool {
        self.index(id).is_some() || self.bow_index(id).is_some() || self.pointer_index(id).is_some()
    }

    pub fn bow_index(&self, id: &str) -> Option<usize> {
        self.file.bows.iter().position(|b| b.id == id)
    }

    pub fn bow(&self, i: usize) -> &BowDef {
        &self.file.bows[i]
    }

    pub fn arrow_def(&self) -> Option<&ArrowDef> {
        self.file.arrow.as_ref()
    }

    fn build(file: &WeaponsFile, images: &mut Assets<Image>, layouts: &mut Assets<TextureAtlasLayout>) -> Result<Built, String> {
        let mut turned = Vec::new();
        let mut blades = file.weapons.clone();
        for w in &file.weapons {
            turned.push(Turned::build(turn_art(&w.art, None).map_err(|e| format!("weapon `{}`: {e}", w.id))?, images, layouts));
        }
        // Every item whose icon's shape is held: drawn from the sprite of
        // that name (in its colours), a blade if it swings.
        let icons: IconsLook = load_ron(&data_path("icons.ron"))?;
        let mut pointers = Vec::new();
        let mut items: Vec<(&String, &IconLook)> = icons.icons.iter().collect();
        items.sort_by_key(|(id, _)| id.as_str());
        for (id, look) in items {
            let Some(h) = file.held.get(&look.shape) else { continue };
            let art = compile_art(&look.shape, h.recolor.then_some(&look.palette)).map_err(|e| format!("held `{id}`: {e}"))?;
            let frames = turn_compiled(&art, &look.shape, None)?;
            match &h.swing {
                Some(sw) => {
                    blades.push(WeaponDef { id: id.clone(), art: look.shape.clone(), damage: sw.damage, knock: sw.knock, stun: sw.stun, rest: h.rest, moves: sw.moves.clone() });
                    turned.push(Turned::build(frames, images, layouts));
                }
                None => {
                    let at = |name: &str| art.anchors.get(name).and_then(|m| m.get(&0)).copied();
                    let flame = at("flame").zip(at("grip")).map(|((fx, fy), (gx, gy))| Vec2::new((fx - gx) as f32, (gy - fy) as f32));
                    pointers.push((Pointer { id: id.clone(), rest: h.rest, burns: h.burns, flame }, Turned::build(frames, images, layouts)));
                }
            }
        }
        let mut bows = Vec::new();
        for b in &file.bows {
            let pose = |f| turn_art(&b.art, Some(f)).map_err(|e| format!("bow `{}`: {e}", b.id));
            bows.push([Turned::build(pose("rest")?, images, layouts), Turned::build(pose("drawn")?, images, layouts)]);
        }
        let arrow = match &file.arrow {
            Some(a) => Some(Turned::build(turn_art(&a.art, None)?, images, layouts)),
            None => None,
        };
        Ok(Built { blades, pointers, turned, bows, arrow })
    }
}

fn load(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut layouts: ResMut<Assets<TextureAtlasLayout>>) {
    let path = data_path("weapons.ron");
    let file: WeaponsFile = load_ron(&path).unwrap_or_else(|e| panic!("{e}"));
    let b = Weapons::build(&file, &mut images, &mut layouts).unwrap_or_else(|e| panic!("{e}"));
    commands.insert_resource(Weapons {
        file,
        blades: b.blades,
        pointers: b.pointers,
        turned: b.turned,
        bows: b.bows,
        arrow: b.arrow,
        watch: Watched::new(path),
        art_watch: Watched::new(assets_dir().join("art")),
    });
}

/// Editing weapons.ron or a weapon's sprite takes effect at once.
fn reload(weapons: Option<ResMut<Weapons>>, mut images: ResMut<Assets<Image>>, mut layouts: ResMut<Assets<TextureAtlasLayout>>) {
    let Some(mut w) = weapons else { return };
    let (a, b) = (w.watch.changed(), w.art_watch.changed());
    if !a && !b {
        return;
    }
    let file = match load_ron::<WeaponsFile>(w.watch.path()) {
        Ok(f) => f,
        Err(e) => return warn!("weapons not reloaded: {e}"),
    };
    match Weapons::build(&file, &mut images, &mut layouts) {
        Ok(b) => {
            w.file = file;
            w.blades = b.blades;
            w.pointers = b.pointers;
            w.turned = b.turned;
            w.bows = b.bows;
            w.arrow = b.arrow;
            info!("weapons reloaded");
        }
        Err(e) => warn!("weapons not reloaded: {e}"),
    }
}

// ---- state ----

/// What a creature holds in its hand (a weapon id), if anything.
#[derive(Component, Default, Clone, Debug, PartialEq)]
pub struct Wielding(pub Option<String>);

/// Swing at `at` (a world point) with what's wielded; during a swing, the
/// next move of the combo is queued.
#[derive(Message, Clone, Copy, Debug)]
pub struct MeleeRequest {
    pub attacker: Entity,
    pub at: Vec2,
}

/// Something was struck: every hit is one of these, and one system
/// (`apply_hits`) does what hits do, whatever struck.
#[derive(Message, Clone, Copy, Debug)]
pub struct Hit {
    pub target: Entity,
    pub damage: f32,
    /// The knockback (cells/s) and how long it takes control away.
    pub knock: Vec2,
    pub stun: f32,
    /// Where it struck, and which way the blow was going.
    pub at: Vec2,
    pub dir: Vec2,
    /// How hard it lands, for the hit-stop (1: a shortsword's slash).
    pub weight: f32,
    /// A critical hit (the stats' crit chance): more sparks, a longer stop.
    pub crit: bool,
}

/// What a swing does to the swinger: a lunge forward, a pogo up (which
/// gives back its air jumps and dash).
#[derive(Message, Clone, Copy, Debug)]
pub struct Recoil {
    pub who: Entity,
    pub add: Vec2,
    pub pogo: Option<f32>,
}

/// Contact damage: what it touches of another side (anyone can touch the
/// neutral) is hit, then it rests `every` s.
#[derive(Component, Clone, Copy, Debug, Deserialize)]
pub struct Touch {
    pub damage: f32,
    #[serde(default)]
    pub knock: f32,
    #[serde(default)]
    pub stun: f32,
    #[serde(default = "touch_every")]
    pub every: f32,
    #[serde(skip)]
    rest: f32,
}

fn touch_every() -> f32 {
    0.6
}

type Toucher<'a> = (Entity, &'a mut Touch, &'a Kinematics, Option<&'a Team>);
type Touched<'a> = (Entity, &'a Kinematics, Option<&'a Team>, Has<Invulnerable>);

/// Things that hurt by touch hit what they touch.
fn touch(mut hits: MessageWriter<Hit>, mut touchers: Query<Toucher>, bodies: Query<Touched, With<Health>>) {
    for (me, mut t, k, team) in &mut touchers {
        t.rest -= DT;
        if t.rest > 0.0 {
            continue;
        }
        for (e, tk, tteam, safe) in &bodies {
            if e == me || safe {
                continue;
            }
            // (Not its own side; and a critter can't hurt a critter.)
            match (team, tteam) {
                (Some(a), Some(b)) if a == b => continue,
                (Some(Team::Neutral), _) => continue,
                _ => {}
            }
            let d = tk.body.pos - k.body.pos;
            if (d.abs() - (tk.body.half + k.body.half)).max_element() > 0.5 {
                continue;
            }
            let push = (Vec2::new(d.x, 0.0).normalize_or(Vec2::X * k.loco.facing) + Vec2::new(0.0, 0.4)).normalize() * t.knock;
            hits.write(Hit { target: e, damage: t.damage, knock: push, stun: t.stun, at: k.body.pos + d * 0.5, dir: d.normalize_or(Vec2::X), weight: t.damage / 12.0, crit: false });
            t.rest = t.every;
            break;
        }
    }
}

/// A body started a dash (a dodge).
#[derive(Message, Clone, Copy, Debug)]
pub struct Dashed(pub Entity);

/// A swing under way.
#[derive(Component, Debug)]
pub struct Swing {
    weapon: usize,
    mv: usize,
    t: f32,
    /// Where it's aimed, degrees from level (facing right), and the facing.
    aim: f32,
    facing: f32,
    /// The blade's angle from where it was held when the swing began.
    start: f32,
    /// The blade's angle last tick (world-relative to facing), and now.
    prev: Option<f32>,
    pub angle: f32,
    pub thrust: f32,
    hit: Vec<Entity>,
    queued: bool,
    clanged: bool,
    lunged: bool,
    /// Struck downward in the air: a hit bounces the swinger up.
    pogo: bool,
}

/// Between swings: which move of the combo comes next, and how long since
/// the last ended.
#[derive(Component, Default)]
pub struct Combo {
    next: usize,
    idle: f32,
}

/// What swings and dodges cost; it comes back after a pause.
#[derive(Component, Clone, Copy, Debug)]
pub struct Stamina {
    pub cur: f32,
    pub max: f32,
    /// A second, once it's been `wait` s since the last spend.
    pub regen: f32,
    pub wait: f32,
    since: f32,
}

impl Stamina {
    pub fn new(max: f32) -> Self {
        Stamina { cur: max, max, regen: 45.0, wait: 0.5, since: 1.0 }
    }

    pub(crate) fn spend(&mut self, n: f32) {
        self.cur = (self.cur - n).max(0.0);
        self.since = 0.0;
    }
}

/// How a creature takes hits: `poise` damage shrugged off (no stun, a
/// quarter of the knockback) before one staggers it (full knockback, stun,
/// its own swing broken off), coming back after a pause; knockback divided
/// by `heft`; `after_hit` s untouchable after being hit.
#[derive(Component, Clone, Copy, Debug)]
pub struct Sturdy {
    pub poise: f32,
    left: f32,
    pub heft: f32,
    pub after_hit: f32,
    since: f32,
}

impl Sturdy {
    pub fn new(poise: f32, heft: f32, after_hit: f32) -> Self {
        Sturdy { poise, left: poise, heft: heft.max(0.1), after_hit, since: 0.0 }
    }
}

/// Seconds without a hit before poise is whole again.
const POISE_BACK: f32 = 1.5;

/// Nothing hurts it for `left` s (a dodge): what it loses is given back.
#[derive(Component, Clone, Copy, Debug)]
pub struct Invulnerable {
    pub left: f32,
    hp: f32,
}

/// The world all but stops for a moment on a hit.
#[derive(Resource, Default)]
struct HitStop {
    left: f32,
    /// The speed to go back to (while stopped).
    saved: Option<f32>,
}

impl HitStop {
    fn hit(&mut self, secs: f32) {
        self.left = self.left.max(secs);
    }
}

fn hit_stop(real: Res<Time<Real>>, mut stop: ResMut<HitStop>, mut virt: ResMut<Time<Virtual>>) {
    if stop.left > 0.0 {
        if stop.saved.is_none() {
            stop.saved = Some(virt.relative_speed());
            virt.set_relative_speed(STOP_SPEED);
        }
        stop.left -= real.delta_secs();
    } else if let Some(s) = stop.saved.take() {
        virt.set_relative_speed(s);
    }
}

// ---- systems ----

type Fighter<'a> = (&'a Kinematics, &'a Wielding, Option<&'a mut Swing>, Option<&'a mut Combo>, Option<&'a mut Stamina>);
type Holder<'a> = (Entity, &'a Wielding, &'a Kinematics, Option<&'a HandPos>, Option<&'a Swing>, Option<&'a crate::archery::Nocked>, Option<&'a Aiming>, Option<&'a Children>);

/// Swings begin (or queue the next) when asked for.
fn start_swings(
    mut commands: Commands,
    mut asks: MessageReader<MeleeRequest>,
    weapons: Option<Res<Weapons>>,
    mut q: Query<Fighter>,
) {
    let Some(weapons) = weapons else { return };
    for ask in asks.read() {
        let Ok((k, wielding, swing, combo, stamina)) = q.get_mut(ask.attacker) else { continue };
        let Some(w) = wielding.0.as_deref().and_then(|id| weapons.index(id)) else { continue };
        if let Some(mut s) = swing {
            // During a swing, the next is queued.
            s.queued = true;
            continue;
        }
        let def = weapons.def(w);
        let mv_i = combo.as_ref().map_or(0, |c| if c.idle <= weapons.file.combo_gap { c.next % def.moves.len().max(1) } else { 0 });
        let Some(mv) = def.moves.get(mv_i) else { continue };
        if let Some(mut s) = stamina {
            if s.cur <= 0.0 {
                continue;
            }
            s.spend(mv.stamina);
        }
        let d = ask.at - k.body.pos;
        let facing = if d.x.abs() > 0.5 { d.x.signum() } else { k.loco.facing };
        let aim = d.y.atan2(d.x.abs()).to_degrees();
        commands.entity(ask.attacker).insert(Swing {
            weapon: w,
            mv: mv_i,
            t: 0.0,
            aim,
            facing,
            start: def.rest - aim,
            prev: None,
            angle: def.rest,
            thrust: 0.0,
            hit: Vec::new(),
            queued: false,
            clanged: false,
            lunged: false,
            pogo: !k.loco.grounded() && d.normalize_or_zero().y < -0.5,
        });
    }
}

/// A dash is a dodge: it costs stamina and nothing hurts you for a moment.
fn dodge(mut commands: Commands, mut dashed: MessageReader<Dashed>, mut q: Query<(&Health, Option<&mut Stamina>)>) {
    for &Dashed(e) in dashed.read() {
        let Ok((h, stamina)) = q.get_mut(e) else { continue };
        if let Some(mut s) = stamina {
            s.spend(DODGE_COST);
        }
        commands.entity(e).insert(Invulnerable { left: DODGE_SAFE, hp: h.hp });
    }
}

fn stamina(mut q: Query<&mut Stamina>, mut combos: Query<&mut Combo, Without<Swing>>, mut sturdy: Query<&mut Sturdy>) {
    for mut s in &mut sturdy {
        s.since += DT;
        if s.since >= POISE_BACK {
            s.left = s.poise;
        }
    }
    for mut s in &mut q {
        s.since += DT;
        if s.since >= s.wait {
            s.cur = (s.cur + s.regen * DT).min(s.max);
        }
    }
    for mut c in &mut combos {
        c.idle += DT;
    }
}

/// Just before damage is noticed: what the untouchable lost comes back.
pub fn guard(mut commands: Commands, mut q: Query<(Entity, &mut Invulnerable, &mut Health)>) {
    for (e, mut inv, mut h) in &mut q {
        h.hp = h.hp.max(inv.hp);
        inv.hp = h.hp;
        inv.left -= DT;
        if inv.left <= 0.0 {
            commands.entity(e).remove::<Invulnerable>();
        }
    }
}

type Swinger<'a> = (Entity, &'a mut Swing, &'a Kinematics, Option<&'a HandPos>, Option<&'a Team>, Option<&'a mut Stamina>, Option<&'a crate::gear::Stats>);
type Target<'a> = (Entity, &'a Kinematics, Option<&'a Team>, Option<&'a Animator>, Has<Invulnerable>);

/// Swings move on a tick; while they sweep, they hit.
#[allow(clippy::too_many_arguments)]
fn swing(
    mut commands: Commands,
    weapons: Option<Res<Weapons>>,
    mut sim: ResMut<SimWorld>,
    mut sparks: ResMut<Sparks>,
    mut hits: MessageWriter<Hit>,
    mut recoil: MessageWriter<Recoil>,
    mut swingers: Query<Swinger>,
    targets: Query<Target, With<Health>>,
) {
    let Some(weapons) = weapons else { return };
    let none = crate::gear::Stats::default();
    for (me, mut s, k, hand, team, mut stamina, stats) in &mut swingers {
        let stats = stats.unwrap_or(&none);
        let def = weapons.def(s.weapon).clone();
        let Some(mv) = def.moves.get(s.mv).cloned() else {
            commands.entity(me).remove::<Swing>();
            continue;
        };
        s.t += DT * stats.mult(crate::gear::Stat::AttackSpeed);
        // Done: the next of the combo if it was asked for (and there's the
        // stamina for it), else rest.
        if s.t >= mv.length() {
            let next = (s.mv + 1) % def.moves.len();
            let can = stamina.as_ref().is_none_or(|st| st.cur > 0.0);
            if s.queued && can {
                if let Some(st) = stamina.as_mut() {
                    st.spend(def.moves[next].stamina);
                }
                s.mv = next;
                s.t = 0.0;
                s.start = s.angle;
                s.prev = None;
                s.hit.clear();
                s.queued = false;
                s.clanged = false;
                s.lunged = false;
                continue;
            }
            commands.entity(me).remove::<Swing>().insert(Combo { next, idle: 0.0 });
            continue;
        }
        let (rel, thrust) = mv.pose(s.t, s.start);
        let angle = s.aim + rel;
        s.angle = angle;
        s.thrust = thrust;
        let facing = s.facing;
        let dir = |a: f32| Vec2::new(a.to_radians().cos() * facing, a.to_radians().sin());
        // The arm follows the blade.
        let hand_at = hand.and_then(|h| h.at).unwrap_or(k.body.pos);
        commands.entity(me).insert(Aiming { at: hand_at + dir(angle) * 30.0, left: 0.1 });
        if !mv.active_at(s.t) {
            s.prev = None;
            continue;
        }
        if !s.lunged && k.loco.grounded() && mv.lunge > 0.0 {
            s.lunged = true;
            recoil.write(Recoil { who: me, add: Vec2::new(dir(s.aim).x.signum() * mv.lunge, 0.0), pogo: None });
        }
        // From last tick's angle to this one, a turn at a time.
        let from = s.prev.unwrap_or(angle);
        s.prev = Some(angle);
        let steps = ((angle - from).abs() / STEP).ceil().max(1.0) as usize;
        let turned = &weapons.turned[s.weapon];
        let feet = k.body.pos.y - k.body.half.y;
        for i in 0..=steps {
            let a = from + (angle - from) * i as f32 / steps as f32;
            let reach = dir(a) * thrust;
            let cells: Vec<Vec2> = turned.cells(a, facing).map(|c| hand_at + reach + c).collect();
            // The smear: along the outer part of the blade.
            let far = cells.iter().map(|c| c.distance(hand_at)).fold(0.0, f32::max);
            let trail = &weapons.file.trail;
            for &c in cells.iter().filter(|c| c.distance(hand_at) > far * 0.6).step_by(4) {
                sparks.emit(trail, trail.count as usize, c, Vec2::ZERO, Vec2::ZERO);
            }
            // Grass and the like are cut; stone rings.
            for &c in &cells {
                let p = CellPos::from_world(c.x, c.y);
                let Some(cell) = sim.world.get(p) else { continue };
                let ph = *sim.world.materials().phys(cell.material);
                if ph.kind == Kind::Plant && ph.hardness <= 2 {
                    sim.world.apply_edit(&WorldEdit::Dig { center: p, radius: 0, max_hardness: 2 });
                } else if ph.kind == Kind::Static && ph.hardness >= 20 && c.y > feet + 1.0 && !s.clanged {
                    s.clanged = true;
                    sparks.emit(&weapons.file.clang, weapons.file.clang.count as usize, c, -dir(a), Vec2::ZERO);
                }
            }
            let (lo, hi) = cells.iter().fold((Vec2::MAX, Vec2::MIN), |(lo, hi), c| (lo.min(*c), hi.max(*c)));
            for (e, tk, tteam, anim, safe) in &targets {
                if e == me || safe || s.hit.contains(&e) {
                    continue;
                }
                // (No hitting your own side; anyone can hit the neutral.)
                if let (Some(a), Some(b)) = (team, tteam)
                    && a == b
                    && *a != Team::Neutral
                {
                    continue;
                }
                let (bmin, bmax) = (tk.body.pos - tk.body.half - 3.0, tk.body.pos + tk.body.half + 3.0);
                if hi.x < bmin.x || lo.x > bmax.x || hi.y < bmin.y || lo.y > bmax.y {
                    continue;
                }
                let touched = cells.iter().find(|&&c| match anim.and_then(|a| a.def.rig.as_ref().map(|r| (a, r))) {
                    Some((a, rig)) => {
                        let (px, py) = pixel_at(&a.def, tk, c);
                        rig.frames.get(a.shown).is_some_and(|f| f.opaque(px, py))
                    }
                    None => (c - tk.body.pos).abs().cmple(tk.body.half).all(),
                });
                let Some(&at) = touched else { continue };
                s.hit.push(e);
                let away = (tk.body.pos - k.body.pos).normalize_or(Vec2::X * facing);
                let roll = (platypus_sim::rng::hash(&[sim.world.tick(), me.to_bits(), e.to_bits()]) % 10_000) as f32 / 10_000.0;
                let (damage, knock, crit) = stats.strike(def.damage * mv.damage, def.knock * mv.knock, roll);
                let push = (Vec2::new(away.x, 0.0).normalize_or(Vec2::X * facing) + Vec2::new(0.0, 0.45)).normalize() * knock;
                hits.write(Hit { target: e, damage, knock: push, stun: def.stun, at, dir: dir(a), weight: damage / 12.0, crit });
                if s.pogo {
                    recoil.write(Recoil { who: me, add: Vec2::ZERO, pogo: Some(weapons.file.pogo) });
                }
            }
        }
    }
}

type Struck<'a> = (&'a mut Kinematics, &'a mut Health, Option<&'a mut Sturdy>, &'a crate::actors::MoveStats, Has<Invulnerable>);

/// What a hit does: damage, knockback and stun, sparks where it struck, a
/// moment of hit-stop (longer, harder hits), a shake.
#[allow(clippy::too_many_arguments)]
fn apply_hits(
    mut commands: Commands,
    mut recoils: MessageReader<Recoil>,
    mut hits: MessageReader<Hit>,
    weapons: Option<Res<Weapons>>,
    mut sparks: ResMut<Sparks>,
    mut stop: ResMut<HitStop>,
    mut trauma: ResMut<crate::fx::Trauma>,
    mut q: Query<Struck>,
) {
    let Some(weapons) = weapons else { return };
    for r in recoils.read() {
        let Ok((mut k, _, _, stats, _)) = q.get_mut(r.who) else { continue };
        k.body.vel += r.add;
        if let Some(up) = r.pogo {
            k.body.vel.y = up;
            k.loco.refresh_air(&stats.0);
        }
    }
    // (Given its grace this tick: the rest of this tick's hits miss too, or
    // a swarm's bites all land at once.)
    let mut graced = std::collections::HashSet::new();
    for h in hits.read() {
        let Ok((mut k, mut health, sturdy, _, safe)) = q.get_mut(h.target) else { continue };
        if safe || graced.contains(&h.target) {
            continue;
        }
        health.harm(h.damage, crate::actors::Harm::Physical);
        let (mut knock, mut stun) = (h.knock, h.stun);
        if let Some(mut s) = sturdy {
            knock /= s.heft;
            s.since = 0.0;
            s.left -= h.damage;
            if s.left > 0.0 {
                // Shrugged off.
                knock *= 0.25;
                stun = 0.0;
            } else {
                s.left = s.poise;
                commands.entity(h.target).remove::<Swing>();
            }
            if s.after_hit > 0.0 {
                commands.entity(h.target).insert(Invulnerable { left: s.after_hit, hp: health.hp });
                graced.insert(h.target);
            }
        }
        let k = &mut *k;
        if stun > 0.0 {
            k.loco.knock(&mut k.body, knock, stun);
        } else {
            k.body.vel += knock;
        }
        let e = &weapons.file.hit;
        let (n, heavier) = if h.crit { (e.count as usize * 3, 1.6) } else { (e.count as usize, 1.0) };
        sparks.emit(e, n, h.at, -h.dir, Vec2::ZERO);
        stop.hit(STOP * (0.7 + 0.3 * h.weight).min(2.0) * heavier);
        trauma.0 = (trauma.0 + 0.12).min(1.0);
    }
}

#[derive(Component)]
struct WeaponSprite;

/// What's in the hand: a blade or a bow (by index).
#[derive(Clone, Copy)]
enum Held {
    Blade(usize),
    Bow(usize),
    Pointer(usize),
}

/// The weapon in the hand: at rest, or where the swing has it; a bow while
/// drawn; anything else held pointed where it's used (a wand casting), and
/// a torch alight.
#[allow(clippy::too_many_arguments)]
fn draw(
    mut commands: Commands,
    weapons: Option<Res<Weapons>>,
    lights: Res<crate::light::LightSettings>,
    holders: Query<Holder>,
    mut sprites: Query<(&mut Sprite, &mut Transform, &mut Visibility, Option<&mut crate::light::torch::Flame>), With<WeaponSprite>>,
) {
    let Some(weapons) = weapons else { return };
    for (e, wielding, k, hand, swing, nocked, aiming, children) in &holders {
        let sprite = children.and_then(|c| c.iter().find(|c| sprites.contains(*c)));
        let id = wielding.0.as_deref();
        let w = id.and_then(|id| {
            weapons.index(id).map(Held::Blade).or_else(|| weapons.bow_index(id).map(Held::Bow)).or_else(|| weapons.pointer_index(id).map(Held::Pointer))
        });
        let (Some(sprite), Some(w)) = (sprite, w) else {
            if let (None, Some(_)) = (sprite, w) {
                commands.entity(e).with_child((WeaponSprite, Sprite::default(), Transform::default(), Visibility::Hidden));
            }
            if let Some(c) = sprite
                && let Ok((_, _, mut v, flame)) = sprites.get_mut(c)
            {
                *v = Visibility::Hidden;
                if flame.is_some() {
                    commands.entity(c).remove::<(crate::light::torch::Flame, crate::light::LightSource)>();
                }
            }
            continue;
        };
        let Ok((mut sp, mut tf, mut vis, flame)) = sprites.get_mut(sprite) else { continue };
        let Some(local) = hand.and_then(|h| h.local) else {
            *vis = Visibility::Hidden;
            continue;
        };
        // Where it points when it's being used.
        let aimed = |at: Vec2| {
            let d = at - hand.and_then(|h| h.at).unwrap_or(k.body.pos);
            d.y.atan2(d.x.abs()).to_degrees()
        };
        let mut burning = None;
        let (turned, facing, angle, thrust) = match w {
            Held::Blade(w) => {
                let (angle, thrust) = swing.map_or((weapons.def(w).rest, 0.0), |s| (s.angle, s.thrust));
                (&weapons.turned[w], swing.map_or(k.loco.facing, |s| s.facing), angle, thrust)
            }
            Held::Bow(b) => match nocked {
                Some(n) => {
                    let pose = if n.drawn(&weapons) > 0.35 { 1 } else { 0 };
                    (&weapons.bows[b][pose], k.loco.facing, aimed(n.at), 0.0)
                }
                None => {
                    *vis = Visibility::Hidden;
                    continue;
                }
            },
            Held::Pointer(i) => {
                let (p, turned) = &weapons.pointers[i];
                let angle = aiming.filter(|a| a.left > 0.0).map_or(p.rest, |a| aimed(a.at));
                if p.burns {
                    burning = Some(p.flame.unwrap_or(Vec2::ZERO));
                }
                (turned, k.loco.facing, angle, 0.0)
            }
        };
        let dir = Vec2::new(angle.to_radians().cos() * facing, angle.to_radians().sin());
        if sp.image != turned.image {
            *sp = Sprite::from_atlas_image(turned.image.clone(), TextureAtlas { layout: turned.layout.clone(), index: 0 });
        }
        if let Some(atlas) = sp.texture_atlas.as_mut() {
            atlas.index = Turned::index(angle);
        }
        sp.flip_x = facing < 0.0;
        tf.translation = local + (dir * thrust).extend(0.03);
        *vis = Visibility::Inherited;
        // A torch in the hand burns: its flame turned with it.
        match (burning, flame) {
            (Some(off), Some(mut f)) => {
                let r = Vec2::from_angle(angle.to_radians()).rotate(off);
                f.at = Vec2::new(r.x * facing, r.y) + Vec2::Y;
            }
            (Some(_), None) => {
                let torch = crate::light::rgb(lights.torch.color, lights.torch.strength);
                commands.entity(sprite).insert((crate::light::torch::Flame::at(Vec2::ZERO), crate::light::LightSource { color: torch, flicker: 1.0 }));
            }
            (None, Some(_)) => {
                commands.entity(sprite).remove::<(crate::light::torch::Flame, crate::light::LightSource)>();
            }
            (None, None) => {}
        }
    }
}
