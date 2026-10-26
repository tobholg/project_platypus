//! The distortion of a channelled spell: a screen-space pass (Bevy's
//! fullscreen material, `assets/shaders/warp.wgsl`). Round (`cone` 0), a
//! gravity well: it swirls and pinches what's drawn around it, darkens its
//! heart and rings it with a faint bright edge, like light bending round
//! something heavy. A cone (force), from the caster toward the cursor:
//! sharp wavefronts racing out along it (a push, stretching the picture
//! outward) or in (a pull, squeezing it), brightest at the fronts, fading at
//! the cone's edges. One at a time (the strongest); strength 0 is off.
//!
//! The same pass bends the light through painted fields (`fields`, see
//! `FieldWarp`). (One pass for both: Bevy keeps one fullscreen pipeline a
//! camera, so a second material would draw with the first's.)

use bevy::core_pipeline::fullscreen_material::FullscreenMaterial;
use bevy::core_pipeline::{Core2d, Core2dSystems};
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::BoxedSystem;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;

/// Where the well is on screen and how hard it bends things.
#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default, Debug)]
pub struct Warp {
    /// Its middle, in screen UV (0..1 from the top left).
    pub center: Vec2,
    /// How far it reaches, as a share of the screen's height.
    pub radius: f32,
    /// 0 = off … 1 = strongest.
    pub strength: f32,
    /// Screen width over height.
    pub aspect: f32,
    /// Seconds, for the shimmer.
    pub time: f32,
    /// 1: a push (outward); 0: drawing in.
    pub push: f32,
    /// Half the cone's angle (radians); 0: all round.
    pub cone: f32,
    /// Which way the cone points, on screen (x right, y down), in units of
    /// the screen's height.
    pub dir: Vec2,
    /// Painted fields in view (`field::show`).
    pub fields: FieldWarp,
}

impl FullscreenMaterial for Warp {
    fn fragment_shader() -> ShaderRef {
        "shaders/warp.wgsl".into()
    }

    fn schedule() -> impl bevy::ecs::schedule::ScheduleLabel + Clone {
        Core2d
    }

    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        system.in_set(Core2dSystems::PostProcess)
    }
}

/// Painted fields (`field.rs`) in view, bending the light each its own
/// way (`assets/shaders/warp.wgsl`, after the well's bend): a repulsor
/// magnifies like glass, rippling where it's struck; a slow field wavers,
/// smeared and greyed to violet; an updraft streams upward. No borders:
/// each fades out toward its edge.
#[derive(Clone, Copy, ShaderType, Debug, Default)]
pub struct FieldWarp {
    pub time: f32,
    /// Screen width over height.
    pub aspect: f32,
    /// Cells a screen's height (patterns are in cells).
    pub scale: f32,
    /// How many of `shapes` are fields (0: none in view).
    pub count: f32,
    /// Two a field: (middle or foot in screen UV, its size in screen
    /// heights, its kind: 1 repulsor, 2 slow, 3 updraft), (a column's top
    /// in screen UV, its strength, a seed, -).
    pub shapes: [Vec4; 16],
    /// Blows on repulsors: screen UV, age (s), how hard (0..1).
    pub ripples: [Vec4; 8],
}
