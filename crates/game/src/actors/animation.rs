//! Picks an animation clip from movement state and plays it on the creature's
//! sprite child. Works for every creature; clips come from its definition.

use std::sync::Arc;

use bevy::prelude::*;
use platypus_physics::MoveState;

use super::Kinematics;
use super::creature::{CreatureArt, CreatureDef};

pub struct AnimationPlugin;

/// Marks the child entity that draws a creature.
#[derive(Component)]
pub struct CreatureSprite;

#[derive(Component)]
pub struct Animator {
    pub def: Arc<CreatureDef>,
    clip: String,
    frame: usize,
    timer: f32,
    /// Set by gameplay (e.g. an attack) to play a clip regardless of movement.
    pub force: Option<String>,
}

impl Animator {
    pub fn new(def: Arc<CreatureDef>) -> Self {
        Animator { def, clip: String::new(), frame: 0, timer: 0.0, force: None }
    }
}

/// Clip wanted for a movement state, with fallbacks so a creature only needs `idle`.
fn wanted(k: &Kinematics) -> &'static [&'static str] {
    let v = k.body.vel;
    match k.loco.state {
        MoveState::Dash => &["dash", "run", "idle"],
        MoveState::WallSlide => &["wall", "fall", "idle"],
        MoveState::Stunned => &["hurt", "fall", "idle"],
        MoveState::Air if v.y > 0.0 => &["jump", "fall", "idle"],
        MoveState::Air => &["fall", "jump", "idle"],
        MoveState::Ground if v.x.abs() > 4.0 => &["run", "idle"],
        MoveState::Ground => &["idle"],
    }
}

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, animate);
    }
}

fn animate(
    time: Res<Time>,
    assets: Res<AssetServer>,
    mut art: ResMut<CreatureArt>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut creatures: Query<(&Kinematics, &mut Animator, &Children)>,
    mut sprites: Query<(&mut Sprite, &mut Transform), With<CreatureSprite>>,
) {
    for (k, mut anim, children) in &mut creatures {
        let anim = &mut *anim;
        let def = anim.def.clone();
        let name = match &anim.force {
            Some(f) if def.animations.contains_key(f) => f.clone(),
            _ => wanted(k).iter().find(|n| def.animations.contains_key(**n)).map(|n| n.to_string()).unwrap_or_default(),
        };
        let Some(clip) = def.animations.get(&name) else { continue };
        let changed = name != anim.clip;
        if changed {
            anim.clip = name;
            anim.frame = 0;
            anim.timer = 0.0;
        } else {
            anim.timer += time.delta_secs();
            let step = 1.0 / clip.fps.max(0.01);
            while anim.timer >= step {
                anim.timer -= step;
                anim.frame = if anim.frame + 1 < clip.frames.len() {
                    anim.frame + 1
                } else if clip.looping {
                    0
                } else {
                    anim.frame
                };
            }
        }

        for child in children.iter() {
            let Ok((mut sprite, mut tf)) = sprites.get_mut(child) else { continue };
            if changed {
                sprite.image = art.image(&assets, &clip.image);
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.layout = art.layout(&mut layouts, def.sprite.frame, clip.columns, clip.rows);
                }
            }
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = clip.frames.get(anim.frame).copied().unwrap_or(0);
            }
            // Put the frame's `feet` pixel under the centre of the collision box.
            let facing = k.loco.facing;
            sprite.flip_x = facing < 0.0;
            let (fw, fh) = (def.sprite.frame.0 as f32, def.sprite.frame.1 as f32);
            let (ax, ay) = def.sprite.feet;
            let dx = fw / 2.0 - ax;
            let dy = ay - fh / 2.0;
            tf.translation = Vec3::new(dx * facing, -k.body.half.y + dy, 0.0);
        }
    }
}
