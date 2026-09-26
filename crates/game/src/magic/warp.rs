//! The distortion of a channelled spell: a screen-space pass (Bevy's
//! fullscreen material, `assets/shaders/warp.wgsl`). Round (`cone` 0), a
//! gravity well: it swirls and pinches what's drawn around it, darkens its
//! heart and rings it with a faint bright edge, like light bending round
//! something heavy. A cone (force), from the caster toward the cursor:
//! sharp wavefronts racing out along it (a push, stretching the picture
//! outward) or in (a pull, squeezing it), brightest at the fronts, fading at
//! the cone's edges. One at a time (the strongest); strength 0 is off.

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
