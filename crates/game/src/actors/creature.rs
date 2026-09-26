//! Creature definitions: one RON file per creature in `assets/data/creatures/`.
//! The file stem is the creature's id (`orc.ron` → "orc").
//!
//! Editing a file while the game runs updates every live creature of that kind
//! (movement, health max, animations). Adding a file adds a creature.

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
    pub sprite: SpriteDef,
    /// Clip name → clip. Standard names: idle, run, jump, fall, dash, wall.
    pub animations: HashMap<String, AnimDef>,
    pub brain: BrainDef,
    /// Draw order among creatures.
    #[serde(default = "default_z")]
    pub z: f32,
    /// What it bleeds (a material; "" for nothing): it sprays when it's
    /// hurt and bursts out when it dies (`hurt.rs`).
    #[serde(default = "red_blood")]
    pub blood: String,
}

fn default_z() -> f32 {
    10.0
}

fn red_blood() -> String {
    "blood".into()
}

#[derive(Clone, Debug, Deserialize)]
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
                match load_ron::<CreatureDef>(&path) {
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

/// Loaded images and atlas layouts, shared by all creatures.
#[derive(Resource, Default)]
pub struct CreatureArt {
    images: HashMap<String, Handle<Image>>,
    layouts: HashMap<(u32, u32, u32, u32), Handle<TextureAtlasLayout>>,
}

impl CreatureArt {
    pub fn image(&mut self, assets: &AssetServer, path: &str) -> Handle<Image> {
        self.images.entry(path.to_string()).or_insert_with(|| assets.load(path.to_string())).clone()
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
        app.insert_resource(Creatures { defs: Creatures::load_all(&dir), watch: Watched::new(dir) })
            .init_resource::<CreatureArt>()
            .add_systems(Update, hot_reload_creatures);
    }
}

fn hot_reload_creatures(mut creatures: ResMut<Creatures>, mut q: Query<(&Creature, &mut MoveStats, &mut Health, &mut Animator, &mut Resist)>) {
    if !creatures.watch.changed() {
        return;
    }
    let defs = Creatures::load_all(creatures.watch.path());
    for (c, mut stats, mut health, mut anim, mut resist) in &mut q {
        if let Some(def) = defs.get(&c.kind) {
            stats.0 = def.movement.clone();
            *resist = def.resist;
            health.hp = health.hp.min(def.health);
            health.max = def.health;
            anim.def = def.clone();
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
            let image = art.image(world.resource::<AssetServer>(), &first.image);
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
        e.insert(def.resist);
        if let Some(m) = blood {
            e.insert(super::hurt::Bleeds(m));
        }
        if let Some(sprite) = sprite {
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
