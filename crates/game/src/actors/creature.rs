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

use super::animation::{Animator, CreatureSprite};
use super::brain::BrainRegistry;
use super::elements::Resist;
use super::{Controls, Creature, FallDamage, Health, Kinematics, MoveStats, Team};
use crate::data::{Watched, data_path, load_ron};

pub struct CreaturePlugin;

#[derive(Clone, Debug, Deserialize)]
pub struct CreatureDef {
    pub name: String,
    /// Collision box in cells (width, height).
    pub size: (f32, f32),
    pub health: f32,
    pub team: Team,
    #[serde(default)]
    pub movement: MovementStats,
    #[serde(default)]
    pub fall_damage: Option<FallDamage>,
    /// Elemental resistances (heat, corrosion, fireproof). Default: none.
    #[serde(default)]
    pub resist: Resist,
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
    pub brain: BrainDef,
    /// Draw order among creatures.
    #[serde(default = "default_z")]
    pub z: f32,
    /// Stamina (swings and dodges spend it); none: it never tires.
    #[serde(default)]
    pub stamina: Option<f32>,
    /// A weapon it holds from the start (`assets/data/weapons.ron`).
    #[serde(default)]
    pub weapon: Option<String>,
    /// Damage it shrugs off before a hit staggers it (it comes back after a
    /// pause); 0: every hit does.
    #[serde(default)]
    pub poise: f32,
    /// Knockback it takes is divided by this (a troll's 4).
    #[serde(default = "one_f")]
    pub heft: f32,
    /// Seconds nothing hurts it after a hit (the player's grace).
    #[serde(default)]
    pub after_hit: f32,
    /// It gives off light (a firefly): its colour, how bright, and a pulse
    /// (seconds a swell; 0: steady).
    #[serde(default)]
    pub light: Option<CreatureLight>,
    /// What it bleeds (a material; "" for nothing): it sprays when it's
    /// hurt and bursts out when it dies (`hurt.rs`).
    #[serde(default = "red_blood")]
    pub blood: String,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct CreatureLight {
    pub color: (u8, u8, u8),
    pub strength: f32,
    #[serde(default)]
    pub pulse: f32,
}

fn one_f() -> f32 {
    1.0
}

fn default_z() -> f32 {
    10.0
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
    #[serde(default)]
    pub params: Option<ron::Value>,
}

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

    fn load_all(dir: &std::path::Path) -> HashMap<String, Arc<CreatureDef>> {
        let mut defs = HashMap::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            warn!("no creature directory at {}", dir.display());
            return defs;
        };
        for path in entries.filter_map(|e| Some(e.ok()?.path())) {
            if path.extension().is_some_and(|e| e == "ron") {
                let kind = path.file_stem().unwrap().to_string_lossy().into_owned();
                match load_ron::<CreatureDef>(&path).and_then(with_art) {
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
    let art = platypus_art::parse(&text).and_then(|f| platypus_art::compile(&f)).map_err(|e| format!("{}: {e}", path.display()))?;
    let (atlas, columns, rows) = art.atlas();
    def.sprite = SpriteDef { frame: art.size, feet: art.feet };
    def.animations = art
        .clips
        .iter()
        .map(|(clip, c)| (clip.clone(), AnimDef { image: format!("art:{name}"), columns, rows, frames: c.frames.clone(), fps: c.fps, looping: c.looping }))
        .collect();
    def.atlas = Some(Arc::new(atlas));
    def.rig = Some(Arc::new(art));
    Ok(def)
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
        app.insert_resource(Creatures { defs: Creatures::load_all(&dir), watch: Watched::new(dir), art_watch: Watched::new(art) })
            .init_resource::<CreatureArt>()
            .add_systems(Update, hot_reload_creatures);
    }
}

fn hot_reload_creatures(mut creatures: ResMut<Creatures>, mut art: ResMut<CreatureArt>, mut q: Query<(&Creature, &mut MoveStats, &mut Health, &mut Animator, &mut Resist)>) {
    let (a, b) = (creatures.watch.changed(), creatures.art_watch.changed());
    if !a && !b {
        return;
    }
    art.forget_art();
    let defs = Creatures::load_all(creatures.watch.path());
    for (c, mut stats, mut health, mut anim, mut resist) in &mut q {
        if let Some(def) = defs.get(&c.kind) {
            stats.0 = def.movement.clone();
            *resist = def.resist;
            health.hp = health.hp.min(def.health);
            health.max = def.health;
            anim.def = def.clone();
            anim.refresh();
        }
    }
    info!("creatures reloaded ({} kinds)", defs.len());
    creatures.defs = defs;
}

/// Spawn a creature by id, standing with its feet at `feet`. Returns the entity
/// through `then`, so callers can add their own components (e.g. `LocalPlayer`).
pub fn spawn_creature(commands: &mut Commands, kind: &str, feet: Vec2, then: impl FnOnce(&mut EntityWorldMut) + Send + 'static) {
    let kind = kind.to_string();
    commands.queue(move |world: &mut World| {
        let Some(def) = world.resource::<Creatures>().get(&kind).cloned() else {
            return error!("unknown creature `{kind}`");
        };
        let (w, h) = def.size;
        let center = feet + Vec2::new(0.0, h / 2.0);
        let mut body = Body::new(center, Vec2::new(w, h));
        body.step_height = def.movement.step_height;

        let sprite = world.resource_scope(|world, mut art: Mut<CreatureArt>| {
            let first = def.animations.get("idle").or_else(|| def.animations.values().next())?;
            let image = world.resource_scope(|world, mut images: Mut<Assets<Image>>| art.image(world.resource::<AssetServer>(), &mut images, &first.image, def.atlas.as_deref()));
            let layout = art.layout(&mut world.resource_mut::<Assets<TextureAtlasLayout>>(), def.sprite.frame, first.columns, first.rows);
            Some(Sprite::from_atlas_image(image, TextureAtlas { layout, index: first.frames.first().copied().unwrap_or(0) }))
        });

        let blood = world.resource::<crate::world::SimWorld>().materials().id(&def.blood);
        let mut e = world.spawn((
            Name::new(def.name.clone()),
            Creature { kind: kind.clone() },
            def.team,
            Health { hp: def.health, max: def.health },
            Kinematics { body, loco: Locomotion::default(), prev_pos: center },
            MoveStats(def.movement.clone()),
            Controls::default(),
            Animator::new(def.clone()),
            Transform::from_translation(center.extend(def.z)),
            Visibility::default(),
        ));
        if let Some(f) = def.fall_damage {
            e.insert((f, super::FallTrack::default()));
        }
        e.insert((def.resist, crate::combat::Wielding(def.weapon.clone())));
        if let Some(s) = def.stamina {
            e.insert(crate::combat::Stamina::new(s));
        }
        e.insert(crate::combat::Sturdy::new(def.poise, def.heft, def.after_hit));
        if let Some(l) = def.light {
            let color = crate::light::rgb(l.color, l.strength);
            e.insert(crate::light::LightSource { color, flicker: 0.0 });
            if l.pulse > 0.0 {
                // (Each its own moment in the cycle.)
                let phase = (platypus_sim::rng::hash(&[feet.x.to_bits() as u64, feet.y.to_bits() as u64]) % 628) as f32 / 100.0;
                e.insert(crate::light::Glow { color, period: l.pulse, phase });
            }
        }
        if let Some(m) = blood {
            e.insert(super::hurt::Bleeds(m));
        }
        if let Some(sprite) = sprite {
            // Rigs with an arm that aims get a second sprite for it.
            if def.rig.as_ref().is_some_and(|r| r.fans.contains_key(super::animation::FRONT_ARM)) {
                e.insert(super::animation::HandPos::default());
                e.with_child((sprite.clone(), Transform::default(), Visibility::Hidden, super::animation::ArmSprite));
            }
            e.with_child((sprite, Transform::default(), CreatureSprite));
        }
        let id = e.id();

        // Brain last: it may depend on the components above.
        world.resource_scope(|world, registry: Mut<BrainRegistry>| {
            let mut e = world.entity_mut(id);
            if let Err(err) = registry.insert(&def.brain, &mut e) {
                error!("creature `{kind}`: {err}");
            }
        });
        then(&mut world.entity_mut(id));
    });
}
