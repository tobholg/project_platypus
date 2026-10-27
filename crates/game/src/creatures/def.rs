//! Creature definitions: one RON file per creature in `assets/data/creatures/`.
//! The file stem is the creature's id (`orc.ron` → "orc").
//!
//! Editing a file while the game runs updates every live creature of that kind
//! (movement, health max, animations). Adding a file adds a creature.
//!
//! Its look is either a picture sheet (`sprite` + `animations`) or, with
//! `art: "rabbit"`, a sprite written as text (`assets/art/rabbit.ron`,
//! `platypus_art`): compiled to an atlas at load, its clips the animations;
//! editing the art file reloads it too.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use platypus_physics::{Body, Locomotion, MovementStats};
use serde::Deserialize;

use super::body::animation::{Animator, CreatureSprite};
use super::brain::BrainRegistry;
use super::{Controls, Creature, FallDamage, Health, Kinematics, MoveStats, Team};
use crate::data::{Watched, data_path, load_ron};

pub struct CreaturePlugin;

/// How a creature digs (DESIGN §14.4): its claws (`(hardness, rate)`: cells
/// no harder, at `rate` cells a second of dirt), and acid (what its claws
/// can't take, no harder than `hardness`, nothing inert, at `rate` cells
/// a second of dirt; a few drops of `material` spat at the face every
/// `every` s to show it). The planner counts digging as the diggers dig.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct DigDef {
    #[serde(default)]
    pub claws: Option<(u8, f32)>,
    #[serde(default)]
    pub acid: Option<AcidDig>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct AcidDig {
    #[serde(default = "acid")]
    pub material: String,
    pub hardness: u8,
    pub rate: f32,
    pub every: f32,
}

fn acid() -> String {
    "acid".into()
}

impl DigDef {
    /// As the planner counts it.
    pub fn digging(&self) -> platypus_nav::Digging {
        let (claws, claw_rate) = self.claws.unwrap_or((0, 0.0));
        let (acid, acid_rate) = self.acid.as_ref().map_or((0, 0.0), |a| (a.hardness, a.rate));
        platypus_nav::Digging { claws, claw_rate, acid, acid_rate }
    }
}

/// A spurt (`CreatureDef::spurt`): its run speed times `speed` for `secs`,
/// at a quarry from `range` cells off, from every so often (seconds, from
/// .. to: each its own).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct SpurtDef {
    pub speed: f32,
    pub secs: f32,
    pub every: (f32, f32),
    pub range: (f32, f32),
}

/// How a humanoid takes blows where they land (`CreatureDef::zones`).
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default)]
pub struct Zones {
    pub head: f32,
    pub body: f32,
    pub arms: f32,
}

impl Default for Zones {
    fn default() -> Self {
        Zones { head: 1.0, body: 1.0, arms: 1.0 }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreatureDef {
    pub name: String,
    /// Collision box in cells (width, height).
    pub size: (f32, f32),
    pub health: f32,
    pub team: Team,
    /// Its faction (`factions.ron`: orcs, undead, spiders, ...): what it
    /// hunts besides you, and what hunts it.
    #[serde(default)]
    pub faction: Option<String>,
    #[serde(default)]
    pub movement: MovementStats,
    #[serde(default)]
    pub fall_damage: Option<FallDamage>,
    /// What kind of creature it is (`kinds.ron`: beast, humanoid, insect,
    /// undead, spirit, ooze, construct, ...): how it takes each kind of hurt.
    #[serde(default)]
    pub kind: Option<String>,
    /// Its own changes to its kind's: kind of hurt → multiplier (1 as
    /// anyone, 0 none, below 0 it heals), and what it can't suffer.
    #[serde(default)]
    pub profile: HashMap<crate::creatures::nature::Harm, f32>,
    #[serde(default)]
    pub cant: Vec<crate::creatures::nature::Cant>,
    /// It heals over time, stopped a while by some kinds of hurt (a troll).
    #[serde(default)]
    pub regen: Option<crate::creatures::nature::RegenDef>,
    /// Killed, it falls in a heap that gets up again, unless broken
    /// (`body/rise.rs`: a skeleton).
    #[serde(default)]
    pub rises: Option<super::body::rise::RiseDef>,
    /// Its head cut off (`wounds`): it dies, or fights on blind (bones).
    #[serde(default)]
    pub headless: super::body::wounds::Headless,
    /// Its head comes off past this share of its health in blows to it
    /// (`wounds`; none: the usual 0.3): a tough neck on a light body (a
    /// zombie's, whose head off kills it).
    #[serde(default)]
    pub neck: Option<f32>,
    /// Past that, the chance each blow to its head takes it off (none: the
    /// first does): a zombie's head is never sure to come off.
    #[serde(default)]
    pub behead: Option<f32>,
    /// Its kind and its changes, put together (at load).
    #[serde(skip)]
    pub nature: crate::creatures::nature::Nature,
    /// A sprite written as text (`assets/art/<name>.ron`): sets `sprite` and
    /// `animations` from its size, feet and clips.
    #[serde(default)]
    pub art: Option<String>,
    #[serde(default)]
    pub sprite: SpriteDef,
    /// Clip name → clip. Standard names: idle, run, jump, fall, dash, wall.
    #[serde(default)]
    pub animations: HashMap<String, AnimDef>,
    /// The compiled art's atlas (from `art`).
    #[serde(skip)]
    pub atlas: Option<Arc<platypus_art::Pixels>>,
    /// The compiled art (anchors, arms at angles, frames without an arm).
    #[serde(skip)]
    pub rig: Option<Arc<platypus_art::Art>>,
    /// The art as written (what's worn is drawn onto it: `gear::look`).
    #[serde(skip)]
    pub art_file: Option<Arc<platypus_art::ArtFile>>,
    /// How it decides (`brain/`): none said, it stands still (`idle`).
    #[serde(default = "idle")]
    pub brain: BrainDef,
    /// Code of its own, for what data can't say (`custom/`).
    #[serde(default)]
    pub custom: Option<super::custom::CustomDef>,
    /// Its own moves (moves.ron), in the order it tries them.
    #[serde(default)]
    pub moves: Vec<String>,
    /// How it digs its way through (`brain/way.rs`): claws, acid.
    #[serde(default)]
    pub dig: Option<DigDef>,
    /// Draw order among creatures.
    #[serde(default = "default_z")]
    pub z: f32,
    /// Drawn this many times its picture's size (anchored at its feet);
    /// its `size` and its moves' reach are its own, written for it.
    #[serde(default = "one_f")]
    pub scale: f32,
    /// Stamina (swings and dodges spend it); none: it never tires.
    #[serde(default)]
    pub stamina: Option<f32>,
    /// A weapon it holds from the start (`assets/data/weapons.ron`).
    #[serde(default)]
    pub weapon: Option<String>,
    /// What it fights with when it holds nothing (a skeleton whose sword
    /// was taken from its heap: bony fists).
    #[serde(default)]
    pub unarmed: Option<String>,
    /// What it fights with when it has no arms left (a zombie: its teeth).
    #[serde(default)]
    pub armless: Option<String>,
    /// Damage it shrugs off before a hit staggers it (it comes back after a
    /// pause); 0: every hit does.
    #[serde(default)]
    pub poise: f32,
    /// Knockback it takes is divided by this (a troll's 4).
    #[serde(default = "one_f")]
    pub heft: f32,
    /// How readily it catches fire (`Nature::kindle`): 1 as anything, 0.5
    /// half as readily and it burns out sooner, 0 never.
    #[serde(default = "one_f")]
    pub catches: f32,
    /// Mindless: it walks into a painted wall of fire (`brush::wary`) where
    /// others stop at its edge (a zombie, a rat, a slime).
    #[serde(default)]
    pub heedless: bool,
    /// Each one its own pace: its run speed this much (a share) faster or
    /// slower, at random (a pack of zombies closes in unevenly).
    #[serde(default)]
    pub pace: f32,
    /// How a body drawn in layers (a humanoid rig) takes a blow where it
    /// lands: on its head, its body, an arm (its legs are its `legs`'
    /// parts'). Each 1 if not given.
    #[serde(default)]
    pub zones: Zones,
    /// Seconds nothing hurts it after a hit (the player's grace).
    #[serde(default)]
    pub after_hit: f32,
    /// It hurts what it touches of another side (a spider's bite, a slime):
    /// damage, knockback (cells/s), stun, and a rest between touches.
    #[serde(default)]
    pub touch: Option<crate::combat::Touch>,
    /// A foe that doesn't hurt by touch (else every foe does, by its
    /// build: `Touch::of`).
    #[serde(default)]
    pub touchless: bool,
    /// Walks through webs freely (spiders); others are held back in them.
    #[serde(default)]
    pub web_walker: bool,
    /// What's on it when it dies, besides what it wears (items, how many):
    /// a cocoon's victim's things.
    #[serde(default)]
    pub drops: Vec<(String, u32)>,
    /// What it may wear (gear ids, and the chance of each): rolled when it
    /// comes into the world, as gear found where it stands.
    #[serde(default)]
    pub wears: Vec<(String, f32)>,
    /// Its loot table (`loot.ron`): rolled into its body when it dies.
    #[serde(default)]
    pub loot: Option<String>,
    /// Whether it leaves a body to loot (else what it had spills out).
    #[serde(default = "yes")]
    pub corpse: bool,
    /// Procedural legs and a body seen from above (spiders: `legs.rs`).
    #[serde(default)]
    pub legs: Option<super::body::legs::LegsDef>,
    /// A body that bends, drawn along its spine (a rat's: `spine.rs`).
    #[serde(default)]
    pub spine: Option<super::body::spine::SpineDef>,
    /// It swims stroke by stroke, a body of plates and fins behind its head
    /// (an eel, the leviathan: `swim.rs`).
    #[serde(default)]
    pub swim: Option<super::swim::SwimDef>,
    /// It gives off light (a firefly): its colour, how bright, and a pulse
    /// (seconds a swell; 0: steady).
    #[serde(default)]
    pub light: Option<CreatureLight>,
    /// Its pixels of this colour are eyes (a text sprite's): drawn over the
    /// dark, crisp, so they stand out in it as eyes do; they blink now and
    /// then. (Spiders' are their `legs`' `eyes`.)
    #[serde(default)]
    pub eyes: Option<(u8, u8, u8)>,
    /// Now and then, hunting, it breaks into a run at its quarry (a
    /// zombie's spurt: suddenly fast, for a couple of seconds).
    #[serde(default)]
    pub spurt: Option<SpurtDef>,
    /// It bounces when it lands: back up at this share of how hard it came
    /// down (a blue slime).
    #[serde(default)]
    pub bounce: f32,
    /// It takes on what it's struck with (fire, frost, lightning): the
    /// glutton (`body/goo.rs`, Glutton).
    #[serde(default)]
    pub glutton: bool,
    /// How big each one comes (against its kind: `body::goo::Mass`), from ..
    /// to, most near the smaller (a slime: some much bigger).
    #[serde(default)]
    pub sizes: Option<(f32, f32)>,
    /// Its body lies along the ground under it, tilted with a slope (a
    /// rat: `body::animation::hug`), not level over it.
    #[serde(default)]
    pub hug: bool,
    /// A ghost: the cold and the failing lights round it (`haunt.rs`).
    #[serde(default)]
    pub haunt: Option<super::haunt::HauntDef>,
    /// Its body is goo, cells of the world held round it (a slime:
    /// `body/goo.rs`).
    #[serde(default)]
    pub goo: Option<super::body::goo::GooDef>,
    /// It burns in the sun (by day under the open sky): the dead.
    #[serde(default)]
    pub sunburns: bool,
    /// A sound it makes now and then, awake (a zombie's groan): the
    /// sound, and from how many to how many seconds apart.
    #[serde(default)]
    pub voice: Option<(String, (f32, f32))>,
    /// What it bleeds (a material; "" for nothing): it sprays when it's
    /// hurt and bursts out when it dies (`hurt.rs`).
    #[serde(default = "red_blood")]
    pub blood: String,
    /// How much of it (1: as much as anything; a spider 1.4, a spiderling
    /// half): every spray, drip, burst and stump scaled so.
    #[serde(default = "whole")]
    pub bleeds: f32,
    /// Dead, it bursts into this gas, this far round (cells): a plague
    /// rat's miasma.
    #[serde(default)]
    pub death_gas: Option<(String, i32)>,
}

impl CreatureDef {
    /// How it hurts by touch: its file's, or (a foe) by its build.
    pub fn touch_now(&self) -> Option<crate::combat::Touch> {
        self.touch.or_else(|| (self.team == Team::Enemy && !self.touchless).then(|| crate::combat::Touch::of(self.health, self.heft)))
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct CreatureLight {
    pub color: (u8, u8, u8),
    pub strength: f32,
    #[serde(default)]
    pub pulse: f32,
    /// It glows as glowing cells do (a haze over the dark): eyes in a cave.
    #[serde(default)]
    pub haze: bool,
    /// It wavers like fire, 0..1 (a candle's flame).
    #[serde(default)]
    pub flicker: f32,
    /// A flame burns at this point (from its middle, cells, facing right),
    /// giving off this share of a torch's motes: a candle.
    #[serde(default)]
    pub flame: Option<(f32, f32, f32)>,
}

fn one_f() -> f32 {
    1.0
}

fn default_z() -> f32 {
    10.0
}

fn idle() -> BrainDef {
    BrainDef { kind: "idle".into(), params: None }
}

fn whole() -> f32 {
    1.0
}

fn red_blood() -> String {
    "blood".into()
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SpriteDef {
    /// Frame size in pixels (1 pixel = 1 cell).
    pub frame: (u32, u32),
    /// Pixel in the frame (from its top-left) that stands on the ground,
    /// centred under the collision box.
    pub feet: (f32, f32),
}

#[derive(Clone, Debug, Deserialize)]
pub struct AnimDef {
    /// Path under `assets/`.
    pub image: String,
    pub columns: u32,
    #[serde(default = "one")]
    pub rows: u32,
    pub frames: Vec<usize>,
    pub fps: f32,
    #[serde(default = "yes")]
    pub looping: bool,
}

fn one() -> u32 {
    1
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize)]
pub struct BrainDef {
    /// Name a brain was registered under (`App::register_brain`).
    pub kind: String,
    /// Brain-specific settings, deserialised into the brain component.
    /// (Kept as written: enums with fields, `close: Walk(keep: 11)`, read
    /// as they're meant.)
    #[serde(default)]
    pub params: Option<Box<ron::value::RawValue>>,
}

/// Read the creature files again now (the bestiary's Reload), changed or
/// not; `moves.ron` too.
#[derive(Message, Clone, Copy, Debug)]
pub struct ReloadCreatures;

#[derive(Resource)]
pub struct Creatures {
    defs: HashMap<String, Arc<CreatureDef>>,
    watch: Watched,
    /// `assets/art`: text sprites (reload the creatures drawn from them).
    art_watch: Watched,
}

impl Creatures {
    pub fn get(&self, kind: &str) -> Option<&Arc<CreatureDef>> {
        self.defs.get(kind)
    }

    /// Every creature file as loaded at start (the tests' and tools').
    #[cfg(test)]
    pub fn load_for_tests() -> Self {
        let dir = data_path("creatures");
        let art = data_path("").parent().expect("assets/data").join("art");
        Creatures { defs: Creatures::load_all(&dir), watch: Watched::new(dir), art_watch: Watched::new(art) }
    }

    /// Every creature file, by id.
    pub fn all(&self) -> impl Iterator<Item = (&String, &Arc<CreatureDef>)> {
        self.defs.iter()
    }

    fn load_all(dir: &std::path::Path) -> HashMap<String, Arc<CreatureDef>> {
        let mut defs = HashMap::new();
        let kinds = crate::creatures::nature::Kinds::load().unwrap_or_else(|e| {
            error!("kinds not loaded: {e}");
            Default::default()
        });
        let with_nature = |mut def: CreatureDef| -> Result<CreatureDef, String> {
            def.nature = crate::creatures::nature::Nature::of_kind(&kinds, def.kind.as_deref(), &def.profile, &def.cant).map_err(|e| format!("{}: {e}", def.name))?;
            def.nature.kindle = def.catches;
            Ok(def)
        };
        let Ok(entries) = std::fs::read_dir(dir) else {
            warn!("no creature directory at {}", dir.display());
            return defs;
        };
        for path in entries.filter_map(|e| Some(e.ok()?.path())) {
            if path.extension().is_some_and(|e| e == "ron") {
                let kind = path.file_stem().unwrap().to_string_lossy().into_owned();
                match load_ron::<CreatureDef>(&path).and_then(with_art).and_then(with_nature) {
                    Ok(def) => {
                        defs.insert(kind, Arc::new(def));
                    }
                    Err(e) => error!("creature not loaded: {e}"),
                }
            }
        }
        defs
    }
}

/// A creature drawn from a text sprite: compile it, take its size, feet and
/// clips (their image is `art:<name>`, the atlas kept on the definition).
fn with_art(mut def: CreatureDef) -> Result<CreatureDef, String> {
    let Some(name) = def.art.clone() else { return Ok(def) };
    let path = data_path("").parent().expect("assets/data").join("art").join(format!("{name}.ron"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let dir = path.parent().expect("assets/art").to_path_buf();
    let read = |base: &str| std::fs::read_to_string(dir.join(format!("{base}.ron"))).map_err(|e| format!("{base}.ron: {e}"));
    let file = platypus_art::parse_based(&text, &read).map_err(|e| format!("{}: {e}", path.display()))?;
    let art = platypus_art::compile(&file).map_err(|e| format!("{}: {e}", path.display()))?;
    def.art_file = Some(Arc::new(file));
    set_art(&mut def, &name, art);
    Ok(def)
}

/// Draw a creature from compiled art, its atlas image named `art:<name>`.
pub fn set_art(def: &mut CreatureDef, name: &str, art: platypus_art::Art) {
    let (atlas, columns, rows) = art.atlas();
    def.sprite = SpriteDef { frame: art.size, feet: art.feet };
    def.animations = art
        .clips
        .iter()
        .map(|(clip, c)| (clip.clone(), AnimDef { image: format!("art:{name}"), columns, rows, frames: c.frames.clone(), fps: c.fps, looping: c.looping }))
        .collect();
    def.atlas = Some(Arc::new(atlas));
    def.rig = Some(Arc::new(art));
}

/// Loaded images and atlas layouts, shared by all creatures.
#[derive(Resource, Default)]
pub struct CreatureArt {
    images: HashMap<String, Handle<Image>>,
    layouts: HashMap<(u32, u32, u32, u32), Handle<TextureAtlasLayout>>,
}

impl CreatureArt {
    /// A picture under `assets/`, or `art:<name>`: a compiled text sprite's
    /// atlas (`atlas`), made into an image once.
    pub fn image(&mut self, assets: &AssetServer, images: &mut Assets<Image>, path: &str, atlas: Option<&platypus_art::Pixels>) -> Handle<Image> {
        if let Some(h) = self.images.get(path) {
            return h.clone();
        }
        let handle = match (path.strip_prefix("art:"), atlas) {
            (Some(_), Some(a)) => images.add(Image::new(
                bevy::render::render_resource::Extent3d { width: a.w, height: a.h, depth_or_array_layers: 1 },
                bevy::render::render_resource::TextureDimension::D2,
                a.rgba.clone(),
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
            )),
            _ => assets.load(path.to_string()),
        };
        self.images.insert(path.to_string(), handle.clone());
        handle
    }

    /// A compiled text sprite's atlas with only its pixels of `color` (its
    /// eyes), made once.
    pub fn eyes(&mut self, images: &mut Assets<Image>, path: &str, atlas: &platypus_art::Pixels, color: (u8, u8, u8)) -> Handle<Image> {
        let key = format!("{path}#eyes");
        if let Some(h) = self.images.get(&key) {
            return h.clone();
        }
        let mut rgba = atlas.rgba.clone();
        for p in rgba.chunks_mut(4) {
            if (p[0], p[1], p[2]) != color {
                p[3] = 0;
            }
        }
        let handle = images.add(Image::new(
            bevy::render::render_resource::Extent3d { width: atlas.w, height: atlas.h, depth_or_array_layers: 1 },
            bevy::render::render_resource::TextureDimension::D2,
            rgba,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
        ));
        self.images.insert(key, handle.clone());
        handle
    }

    /// Forget compiled art (it changed): made again when next asked for.
    pub fn forget_art(&mut self) {
        self.images.retain(|k, _| !k.starts_with("art:"));
    }

    pub fn layout(&mut self, layouts: &mut Assets<TextureAtlasLayout>, frame: (u32, u32), cols: u32, rows: u32) -> Handle<TextureAtlasLayout> {
        self.layouts
            .entry((frame.0, frame.1, cols, rows))
            .or_insert_with(|| layouts.add(TextureAtlasLayout::from_grid(UVec2::new(frame.0, frame.1), cols, rows, None, None)))
            .clone()
    }
}

impl Plugin for CreaturePlugin {
    fn build(&self, app: &mut App) {
        let dir = data_path("creatures");
        let art = data_path("").parent().expect("assets/data").join("art");
        app.add_message::<ReloadCreatures>()
            .insert_resource(Creatures { defs: Creatures::load_all(&dir), watch: Watched::new(dir), art_watch: Watched::new(art) })
            .init_resource::<CreatureArt>()
            .add_systems(Update, hot_reload_creatures);
    }
}

type Reloaded<'a> = (Entity, &'a Creature, &'a mut MoveStats, &'a mut Health, &'a mut Animator, Option<&'a super::moves::Moves>, Has<super::brain::Staged>, Has<super::player::LocalPlayer>);

/// A creature file changed (or the bestiary asked): every creature of it
/// takes the new numbers where it stands, mid-fight: stats, health and
/// its profile, art, healing, poise, touch, weapon, its moves (if the
/// list changed) and its brain (put on again from the new settings; not
/// on the player, nor one on the bestiary's stage).
pub(crate) fn hot_reload_creatures(mut commands: Commands, mut creatures: ResMut<Creatures>, mut art: ResMut<CreatureArt>, mut asked: MessageReader<ReloadCreatures>, mut q: Query<Reloaded>) {
    // (Polled without marking it changed: what reads `is_changed` redresses
    // and restats every creature.)
    let polled = creatures.bypass_change_detection();
    let (a, b) = (polled.watch.changed(), polled.art_watch.changed());
    if !a && !b && asked.read().count() == 0 {
        return;
    }
    asked.clear();
    art.forget_art();
    let defs = Creatures::load_all(creatures.watch.path());
    for (e, c, mut stats, mut health, mut anim, moves, staged, player) in &mut q {
        let Some(def) = defs.get(&c.kind) else { continue };
        let old = anim.def.brain.kind.clone();
        stats.0 = def.movement.clone();
        health.nature = def.nature;
        health.hp = health.hp.min(def.health);
        health.max = def.health;
        anim.def = def.clone();
        anim.refresh();
        if player {
            continue;
        }
        let mut ec = commands.entity(e);
        match &def.regen {
            Some(r) => ec.insert(crate::creatures::nature::Regenerates::new(r)),
            None => ec.remove::<crate::creatures::nature::Regenerates>(),
        };
        ec.insert((crate::combat::Sturdy::new(def.poise, def.heft, def.after_hit), crate::combat::Wielding(def.weapon.clone())));
        match def.touch_now() {
            Some(t) => ec.insert(t),
            None => ec.remove::<crate::combat::Touch>(),
        };
        if moves.map_or(&[][..], |m| m.ids()) != def.moves.as_slice() {
            if def.moves.is_empty() {
                ec.remove::<super::moves::Moves>();
            } else {
                ec.insert(super::moves::Moves::new(def.moves.clone()));
            }
        }
        if !staged {
            let def = def.clone();
            ec.queue_silenced(move |mut ew: EntityWorldMut| {
                ew.world_scope(|world| {
                    world.resource_scope(|world, reg: Mut<super::brain::BrainRegistry>| {
                        if let Ok(mut ew) = world.get_entity_mut(e) {
                            reg.remove(&old, &mut ew);
                            if let Err(err) = reg.insert(&def.brain, &mut ew) {
                                error!("{}: {err}", def.name);
                            }
                        }
                    })
                });
            });
        }
    }
    info!("creatures reloaded ({} kinds)", defs.len());
    creatures.defs = defs;
}

/// Spawn a creature by id, standing with its feet at `feet`. Returns the entity
/// through `then`, so callers can add their own components (e.g. `LocalPlayer`).
/// A walker on legs wider than this (cells) stands on this share of its
/// width (`Body::stance`).
const STANCE_FROM: f32 = 14.0;
const STANCE_SHARE: f32 = 0.4;

/// A creature spawned in the rock is moved out to where it fits, up to
/// this far (cells).
const SPAWN_ROOM: i32 = 40;

/// How sizes lean toward the smaller end (`CreatureDef::sizes`): the
/// share up the range is a roll to this power.
const SIZE_SKEW: f32 = 3.5;

pub fn spawn_creature(commands: &mut Commands, kind: &str, feet: Vec2, then: impl FnOnce(&mut EntityWorldMut) + Send + 'static) {
    let kind = kind.to_string();
    commands.queue(move |world: &mut World| {
        let Some(def) = world.resource::<Creatures>().get(&kind).cloned() else {
            return error!("unknown creature `{kind}`");
        };
        let (w, h) = def.size;
        let mut center = feet + Vec2::new(0.0, h / 2.0);
        // (Not in the rock: where its box fits, near, if not just there.)
        if let Some(c) = super::free_spot(&world.resource::<crate::world::SimWorld>().world, center, Vec2::new(w, h) * 0.5, SPAWN_ROOM) {
            center = c;
        }
        let mut body = Body::new(center, Vec2::new(w, h));
        body.step_height = def.movement.step_height;
        // (A wide walker on legs seen from the side stands on its middle:
        // its legs reach the ground either side; `Body::stance`.)
        // (Not a climber: on a wall its box is what holds on, and its way's
        // planned for its whole width.)
        if w > STANCE_FROM && !def.movement.cling && def.legs.as_ref().is_some_and(|l| l.view == crate::creatures::body::legs::View::Side) {
            body.stance = Some((w * STANCE_SHARE).max(STANCE_FROM) / 2.0);
        }

        let (sprite, eyes) = world
            .resource_scope(|world, mut art: Mut<CreatureArt>| {
                let first = def.animations.get("idle").or_else(|| def.animations.values().next())?;
                let image = world.resource_scope(|world, mut images: Mut<Assets<Image>>| art.image(world.resource::<AssetServer>(), &mut images, &first.image, def.atlas.as_deref()));
                let layout = art.layout(&mut world.resource_mut::<Assets<TextureAtlasLayout>>(), def.sprite.frame, first.columns, first.rows);
                let atlas = TextureAtlas { layout, index: first.frames.first().copied().unwrap_or(0) };
                // (Its eyes: the same frames, only their pixels.)
                let eyes = def.eyes.zip(def.atlas.as_deref()).map(|(color, pixels)| {
                    let image = art.eyes(&mut world.resource_mut::<Assets<Image>>(), &first.image, pixels, color);
                    Sprite::from_atlas_image(image, atlas.clone())
                });
                Some((Some(Sprite::from_atlas_image(image, atlas)), eyes))
            })
            .unwrap_or((None, None));

        let blood = world.resource::<crate::world::SimWorld>().materials().id(&def.blood);
        let gas_id = def.death_gas.as_ref().and_then(|(g, _)| world.resource::<crate::world::SimWorld>().materials().id(g));
        let number = {
            let mut ids = world.resource_mut::<super::StableIds>();
            ids.0 += 1;
            ids.0
        };
        let faction = def.faction.as_deref().and_then(|n| {
            let f = world.resource::<super::factions::Factions>().get(n);
            if f.is_none() {
                warn!("{kind}: no faction `{n}` in factions.ron");
            }
            f
        });
        let mut e = world.spawn((
            Name::new(def.name.clone()),
            Creature { kind: kind.clone() },
            def.team,
            Health { nature: def.nature, ..Health::new(def.health) },
            Kinematics { body, loco: Locomotion::default(), prev_pos: center },
            MoveStats(super::Pace::of(def.pace, number).moved(&def.movement)),
            super::StepEase::default(),
            Controls::default(),
            Animator::new(def.clone()),
            Transform::from_translation(center.extend(def.z)),
            Visibility::default(),
        ));
        e.insert(super::Stable(number));
        if def.pace > 0.0 {
            e.insert(super::Pace::of(def.pace, number));
        }
        if let Some(f) = faction {
            e.insert(f);
        }
        if let Some(f) = def.fall_damage {
            e.insert((f, super::FallTrack::default()));
        }
        if let Some(r) = &def.regen {
            e.insert(crate::creatures::nature::Regenerates::new(r));
        }
        if def.goo.is_some() {
            e.insert(super::body::goo::Goo::default());
        }
        // (Each its own size, picked by its number; most near the smaller.)
        if let Some((lo, hi)) = def.sizes {
            let u = (number.wrapping_mul(0xD1B5_4A32_D192_ED03) >> 40) as f32 / (1u64 << 24) as f32;
            e.insert(super::body::goo::Mass(lo + (hi - lo) * u.powf(SIZE_SKEW)));
        }
        if def.glutton {
            e.insert(super::body::goo::Glutton::default());
        }
        if let Some(r) = &def.rises {
            e.insert(super::body::rise::Rises(r.clone()));
        }
        e.insert((crate::combat::Wielding(def.weapon.clone().or_else(|| def.unarmed.clone())), crate::gear::Equipment::default(), crate::gear::Stats::default()));
        if let Some(s) = def.stamina {
            e.insert(crate::combat::Stamina::new(s));
        }
        e.insert(crate::combat::Sturdy::new(def.poise, def.heft, def.after_hit));
        if let Some(t) = def.touch_now() {
            e.insert(t);
        }
        if def.web_walker {
            e.insert(super::WebWalker);
        }
        if let Some(l) = def.light {
            let color = crate::light::rgb(l.color, l.strength);
            e.insert(crate::light::LightSource { color, flicker: l.flicker });
            if let Some((x, y, share)) = l.flame {
                e.insert(crate::light::torch::Flame::small(Vec2::new(x, y), share));
            }
            if l.pulse > 0.0 || l.haze {
                // (Each its own moment in the cycle.)
                let phase = (platypus_sim::rng::hash(&[feet.x.to_bits() as u64, feet.y.to_bits() as u64]) % 628) as f32 / 100.0;
                e.insert(crate::light::Glow { color, period: l.pulse, phase });
            }
        }
        if let Some(m) = blood {
            e.insert(super::body::hurt::Bleeds(m, def.bleeds));
        }
        if let Some((gas, r)) = &def.death_gas
            && let Some(m) = gas_id
        {
            e.insert(super::body::hurt::DeathGas(m, *r));
            let _ = gas;
        }
        if def.rig.as_ref().is_some_and(|r| r.anchors.contains_key("foot_near")) {
            e.insert(super::body::animation::Soles::default());
        }
        if let Some(sprite) = sprite {
            // Rigs with an arm that aims get a second sprite for it.
            if def.rig.as_ref().is_some_and(|r| r.fans.contains_key(super::body::animation::FRONT_ARM)) {
                e.insert(super::body::animation::HandPos::default());
                e.with_child((sprite.clone(), Transform::default(), Visibility::Hidden, super::body::animation::ArmSprite));
            }
            e.with_child((sprite, Transform::default(), CreatureSprite));
            if let Some(eyes) = eyes {
                e.with_child((eyes, Transform::from_xyz(0.0, 0.0, super::body::animation::Z_EYES - def.z), super::body::animation::CreatureEyes, super::body::animation::Blinks));
            }
        }
        let id = e.id();

        // Brain last: it may depend on the components above.
        world.resource_scope(|world, registry: Mut<BrainRegistry>| {
            let mut e = world.entity_mut(id);
            if let Err(err) = registry.insert(&def.brain, &mut e) {
                error!("creature `{kind}`: {err}");
            }
        });
        if !def.moves.is_empty() {
            world.entity_mut(id).insert(super::moves::Moves::new(def.moves.clone()));
        }
        // Then its own code, if it has any.
        if let Some(c) = &def.custom {
            world.resource_scope(|world, registry: Mut<super::custom::CustomRegistry>| {
                let mut e = world.entity_mut(id);
                if let Err(err) = registry.insert(c, &mut e) {
                    error!("creature `{kind}`: {err}");
                }
            });
        }
        then(&mut world.entity_mut(id));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creatures::nature::Harm;

    #[test]
    fn every_creature_file_loads_with_its_kind() {
        let dir = data_path("creatures");
        let files = std::fs::read_dir(&dir).unwrap().filter(|e| e.as_ref().unwrap().path().extension().is_some_and(|x| x == "ron")).count();
        let defs = Creatures::load_all(&dir);
        assert_eq!(defs.len(), files, "a creature file didn't load (see the log)");
        let moves = super::super::moves::MoveBook::load();
        for (id, def) in &defs {
            assert!(def.kind.is_some(), "{id} names its kind");
            for m in &def.moves {
                assert!(moves.has(m), "{id}: no move `{m}` in moves.ron");
            }
        }
        assert_eq!(defs["spider"].moves, ["spider_stamp", "spider_bite", "spider_web", "spider_spit"]);
        // Their natures as designed.
        let of = |id: &str, h: Harm| defs[id].nature.of(h);
        assert!(of("skeleton", Harm::Blunt) > 1.0 && of("skeleton", Harm::Pierce) < 1.0);
        assert!(of("spider", Harm::Acid) < 0.0, "spiders drink acid");
        assert!(of("star_wisp", Harm::Fire) < 0.0 && of("star_wisp", Harm::Frost) > 1.0);
        assert!(defs["troll"].regen.as_ref().is_some_and(|r| r.stopped_by.contains(&Harm::Fire)));
        assert_eq!(of("dummy", Harm::Slash), 1.0, "practice dummies take hits as anyone (readouts stay honest)");
        // What bleeds acid isn't hurt by it: else a scratch bleeds onto
        // it, the acid eats it, it bleeds more, and it melts away.
        for (id, def) in &defs {
            if def.blood == "acid" {
                assert!(def.nature.of(Harm::Acid) <= 0.0, "{id} bleeds acid but acid hurts it");
            }
        }
    }
}
