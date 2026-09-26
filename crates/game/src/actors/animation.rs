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

/// The fan (and pose tag) of an arm that aims.
pub const FRONT_ARM: &str = "front_arm";

/// The child entity drawing a rig's aiming arm (hidden unless aiming).
#[derive(Component)]
pub struct ArmSprite;

/// Aiming at `at` (a world point) for `left` more seconds: a rig points its
/// front arm there (casting; weapons later).
#[derive(Component, Clone, Copy, Debug)]
pub struct Aiming {
    pub at: Vec2,
    pub left: f32,
}

/// Where the creature's (front) hand is: in the world (spells leave from
/// it), and from its centre as drawn (a held weapon sits there). From the
/// aiming arm while it aims, else from the pose's `hand` anchor.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct HandPos {
    pub at: Option<Vec2>,
    pub local: Option<Vec3>,
    /// The off (back) hand, from its centre as drawn: a torch sits there.
    pub off: Option<Vec3>,
}

#[derive(Component)]
pub struct Animator {
    pub def: Arc<CreatureDef>,
    clip: String,
    frame: usize,
    timer: f32,
    /// Set by gameplay (e.g. an attack) to play a clip regardless of movement.
    pub force: Option<String>,
    /// Seconds off the ground (stepping up a bump lifts a body for a tick:
    /// not a jump), and whether it's running (with some give either way).
    air: f32,
    running: bool,
    /// A clip that doesn't loop has played to its end.
    done: bool,
    /// The body frame drawn now (what a blade hits: `pixel_at`).
    pub shown: usize,
}

impl Animator {
    pub fn new(def: Arc<CreatureDef>) -> Self {
        Animator { def, clip: String::new(), frame: 0, timer: 0.0, force: None, air: 0.0, running: false, done: false, shown: 0 }
    }

    /// Pick the clip (and its image) again next frame (the art changed).
    pub fn refresh(&mut self) {
        self.clip.clear();
    }

    /// Play `clip` over whatever movement would pick, from its start (again,
    /// if it's already playing), until `force` is cleared.
    pub fn play(&mut self, clip: &str) {
        self.force = Some(clip.into());
        self.clip.clear();
    }

    /// The clip playing doesn't loop and has reached its end.
    pub fn finished(&self) -> bool {
        self.done
    }
}

/// Off the ground this long before it looks airborne (unless it jumped).
const AIR_GRACE: f32 = 0.12;
/// Starts running above this speed, stops below the lower one (cells/s).
const RUN_ON: f32 = 8.0;
const RUN_OFF: f32 = 3.0;

/// Clip wanted for a movement state, with fallbacks so a creature only needs
/// `idle`. Stepping up a bump (a tick off the ground) doesn't count as air;
/// running starts and stops with some give, so it doesn't flicker.
fn wanted(k: &Kinematics, anim: &mut Animator, dt: f32) -> &'static [&'static str] {
    let v = k.body.vel;
    anim.air = if k.loco.state == MoveState::Air { anim.air + dt } else { 0.0 };
    let airborne = anim.air > AIR_GRACE || v.y > 60.0;
    anim.running = if anim.running { v.x.abs() > RUN_OFF } else { v.x.abs() > RUN_ON };
    match k.loco.state {
        MoveState::Dash => &["dash", "run", "idle"],
        MoveState::WallSlide => &["wall", "fall", "idle"],
        MoveState::Stunned => &["hurt", "fall", "idle"],
        MoveState::Air if airborne && v.y > 0.0 => &["jump", "fall", "idle"],
        MoveState::Air if airborne => &["fall", "jump", "idle"],
        _ if anim.running => &["run", "idle"],
        _ => &["idle"],
    }
}

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, animate);
    }
}

type Animated<'a> = (&'a Kinematics, &'a mut Animator, &'a Children, Option<&'a mut Aiming>, Option<&'a mut HandPos>);
type BodySprites = (With<CreatureSprite>, Without<ArmSprite>);
type Arms<'a> = (&'a mut Sprite, &'a mut Transform, &'a mut Visibility);

#[allow(clippy::too_many_arguments)]
pub fn animate(
    time: Res<Time>,
    assets: Res<AssetServer>,
    mut art: ResMut<CreatureArt>,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut creatures: Query<Animated>,
    mut sprites: Query<(&mut Sprite, &mut Transform), BodySprites>,
    mut arms: Query<Arms, With<ArmSprite>>,
) {
    for (k, mut anim, children, mut aiming, hand) in &mut creatures {
        let anim = &mut *anim;
        let def = anim.def.clone();
        let dt = time.delta_secs();
        let want = wanted(k, anim, dt);
        let name = match &anim.force {
            Some(f) if def.animations.contains_key(f) => f.clone(),
            _ => want.iter().find(|n| def.animations.contains_key(**n)).map(|n| n.to_string()).unwrap_or_default(),
        };
        let Some(clip) = def.animations.get(&name) else { continue };
        let changed = name != anim.clip;
        if changed {
            debug!("clip {} -> {} (state {:?}, vel {:.0},{:.0})", anim.clip, name, k.loco.state, k.body.vel.x, k.body.vel.y);
        }
        if changed {
            anim.clip = name;
            anim.frame = 0;
            anim.timer = 0.0;
            anim.done = false;
        } else {
            // A run goes at the pace it's running (half to one and a half
            // times its rate), and backwards when it moves away from where
            // it faces (backing off while aiming).
            let mut rate = 1.0;
            let mut back = false;
            if name == "run" {
                let run = def.movement.run_speed.max(1.0);
                rate = (k.body.vel.x.abs() / run).clamp(0.5, 1.5);
                back = k.body.vel.x * k.loco.facing < 0.0;
            }
            anim.timer += dt * rate;
            let step = 1.0 / clip.fps.max(0.01);
            let n = clip.frames.len();
            while anim.timer >= step {
                anim.timer -= step;
                anim.frame = if back {
                    (anim.frame + n - 1) % n
                } else if anim.frame + 1 < n {
                    anim.frame + 1
                } else if clip.looping {
                    0
                } else {
                    anim.done = true;
                    anim.frame
                };
            }
        }

        let facing = k.loco.facing;
        let (fw, fh) = (def.sprite.frame.0 as f32, def.sprite.frame.1 as f32);
        // The frame's `feet` pixel under the centre of the collision box.
        let body_at = body_at(&def, k).extend(0.0);
        let mut index = clip.frames.get(anim.frame).copied().unwrap_or(0);
        // Aiming: the pose without its front arm, and the arm drawn at the
        // angle nearest the aim, its pivot at the pose's shoulder.
        if let Some(a) = aiming.as_deref_mut() {
            a.left -= dt;
        }
        let mut arm: Option<(usize, Vec3)> = None;
        let mut hand_at: Option<Vec3> = None;
        // (A frame pixel's place relative to the creature's centre.)
        let local = |px: f32, py: f32| body_at + Vec3::new((px + 0.5 - fw / 2.0) * facing, -(py + 0.5 - fh / 2.0), 0.0);
        if let (Some(a), Some(rig)) = (aiming.as_deref(), def.rig.as_ref())
            && a.left > 0.0
            && let Some(&bare) = rig.without.get(&(index, FRONT_ARM.to_string()))
            && let Some(&(sx, sy)) = rig.anchors.get(FRONT_ARM).and_then(|m| m.get(&index))
        {
            let shoulder = k.body.pos + local(sx as f32, sy as f32).truncate();
            let d = a.at - shoulder;
            let angle = d.y.atan2(d.x.abs()).to_degrees();
            if let Some(f) = rig.fan_frame(FRONT_ARM, angle) {
                let (px, py) = rig.fan_pivot;
                let offset = Vec3::new((sx - px) as f32 * facing, -((sy - py) as f32), 0.01);
                arm = Some((f, body_at + offset));
                index = bare;
                if let Some(&(hx, hy)) = rig.anchors.get("hand").and_then(|m| m.get(&f)) {
                    hand_at = Some(local(hx as f32, hy as f32) + offset);
                }
            }
        }
        // Not aiming: the pose's own hand.
        if arm.is_none()
            && let Some(rig) = def.rig.as_ref()
            && let Some(&(hx, hy)) = rig.anchors.get("hand").and_then(|m| m.get(&index))
        {
            hand_at = Some(local(hx as f32, hy as f32));
        }
        anim.shown = index;
        let off = def.rig.as_ref().and_then(|r| r.anchors.get("back_arm.hand")).and_then(|m| m.get(&index)).map(|&(x, y)| local(x as f32, y as f32));
        if let Some(mut h) = hand {
            h.off = off;
            h.at = hand_at.map(|l| k.body.pos + l.truncate());
            h.local = hand_at;
        }

        for child in children.iter() {
            if let Ok((mut sprite, mut tf, mut vis)) = arms.get_mut(child) {
                *vis = if arm.is_some() { Visibility::Inherited } else { Visibility::Hidden };
                if let Some((f, at)) = arm {
                    sprite.image = art.image(&assets, &mut images, &clip.image, def.atlas.as_deref());
                    if let Some(atlas) = sprite.texture_atlas.as_mut() {
                        atlas.layout = art.layout(&mut layouts, def.sprite.frame, clip.columns, clip.rows);
                        atlas.index = f;
                    }
                    sprite.flip_x = facing < 0.0;
                    tf.translation = at;
                }
                continue;
            }
            let Ok((mut sprite, mut tf)) = sprites.get_mut(child) else { continue };
            if changed {
                sprite.image = art.image(&assets, &mut images, &clip.image, def.atlas.as_deref());
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.layout = art.layout(&mut layouts, def.sprite.frame, clip.columns, clip.rows);
                }
            }
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = index;
            }
            sprite.flip_x = facing < 0.0;
            tf.translation = body_at;
        }
    }
}

/// Where a creature's sprite is drawn, from its centre (the frame's feet
/// pixel under the middle of its box).
pub fn body_at(def: &CreatureDef, k: &Kinematics) -> Vec2 {
    let (fw, fh) = (def.sprite.frame.0 as f32, def.sprite.frame.1 as f32);
    let (ax, ay) = def.sprite.feet;
    Vec2::new((fw / 2.0 - ax) * k.loco.facing, -k.body.half.y + ay - fh / 2.0)
}

/// The pixel of its frame a world point falls on (it may be off the frame).
pub fn pixel_at(def: &CreatureDef, k: &Kinematics, world: Vec2) -> (i32, i32) {
    let (fw, fh) = (def.sprite.frame.0 as f32, def.sprite.frame.1 as f32);
    let d = world - k.body.pos - body_at(def, k);
    (((d.x * k.loco.facing) + fw / 2.0 - 0.5).round() as i32, (fh / 2.0 - 0.5 - d.y).round() as i32)
}
