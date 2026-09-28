//! The screen's edges, telling how you are: a red flash fading in and out
//! when you're hurt (stronger the harder), a slow red breath while your
//! health is low (a little quicker and stronger the lower; calm), and a lush green glow while a
//! potion heals you: full as it's drunk, then dissolving in place as the
//! healing runs out (its soft inner reach fading first, the rim at the
//! screen's edge last), continuously. Each glow is two overlays over the
//! whole view: a soft band and a thin rim, their colour at the edges only,
//! fading to nothing towards the middle.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::actors::Health;
use crate::actors::player::LocalPlayer;

pub struct ScreenFxPlugin;

impl Plugin for ScreenFxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Glows>().add_systems(Startup, setup).add_systems(Update, show);
    }
}

#[derive(Component)]
struct Edge(Kind);

/// Which glow, and its layer.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Red,
    RedRim,
    Green,
    GreenRim,
}

/// How strong each is now (0..1), and where they're heading.
#[derive(Resource, Default)]
struct Glows {
    hurt: f32,
    hurt_shown: f32,
    heal: f32,
    heal_shown: f32,
    last_hp: Option<f32>,
    /// Where the low-health pulse is in its cycle (0..1): stepped on by its
    /// pace each frame, so a change of pace never jumps it.
    beat: f32,
    /// How strong the low-health pulse is, eased (no step as health moves).
    low: f32,
}

/// Low health: below this share of it the edges pulse.
const LOW: f32 = 0.3;
/// The low-health pulse: a slow breath, seconds a cycle (at the top of
/// `LOW`, and at nearly none left). Calm, never a flicker.
const PULSE_SLOW: f32 = 2.4;
const PULSE_FAST: f32 = 1.7;
/// How deep the soft band reaches in from every edge, and the rim (shares
/// of the screen's height).
const BAND: f32 = 0.15;
const RIM: f32 = 0.06;

/// The vignette with its band `band` deep (a share of the screen's
/// height): clear in the middle, rising towards the edges, as deep from
/// the top and bottom as from the sides (16:9), the corners a little
/// stronger; smooth (sampled linearly).
fn vignette(band: f32) -> Image {
    let (w, h) = (320u32, 180u32);
    let aspect = w as f32 / h as f32;
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = ((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32);
            // Distance to the nearest edge, in heights; a soft minimum of
            // the two (the corners fuller).
            let (ex, ey) = (fx.min(1.0 - fx) * aspect, fy.min(1.0 - fy));
            let d = (ex.powf(-3.0) + ey.powf(-3.0)).powf(-1.0 / 3.0);
            let t = (1.0 - d / band.max(1e-3)).clamp(0.0, 1.0);
            let a = t * t * (3.0 - 2.0 * t);
            px.extend([255, 255, 255, (a * 255.0) as u8]);
        }
    }
    let mut image = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, px, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    image.sampler = ImageSampler::linear();
    image
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let (soft, rim) = (images.add(vignette(BAND)), images.add(vignette(RIM)));
    for kind in [Kind::Red, Kind::RedRim, Kind::Green, Kind::GreenRim] {
        let image = if matches!(kind, Kind::Red | Kind::Green) { soft.clone() } else { rim.clone() };
        commands.spawn((
            Edge(kind),
            // (Stretched to the node: by default an image keeps its own
            // proportions, and a window taller than 16:9 left bars at the
            // top and bottom uncovered.)
            ImageNode::new(image).with_color(Color::NONE).with_mode(bevy::ui::widget::NodeImageMode::Stretch),
            // (Pinned to all four edges of the window.)
            Node { position_type: PositionType::Absolute, left: Val::Px(0.0), right: Val::Px(0.0), top: Val::Px(0.0), bottom: Val::Px(0.0), ..default() },
            // (Under the rest of the interface; never takes a click.)
            GlobalZIndex(-50),
            bevy::ui::FocusPolicy::Pass,
        ));
    }
}

fn show(time: Res<Time<Real>>, mut glows: ResMut<Glows>, player: Query<(&Health, Option<&crate::potion::Mending>), With<LocalPlayer>>, mut edges: Query<(&Edge, &mut ImageNode)>) {
    let dt = time.delta_secs().min(0.1);
    let Ok((h, mending)) = player.single() else { return };
    // Hurt: a flash by how much of your health went (a scratch faint, a
    // big blow strong).
    if let Some(was) = glows.last_hp
        && was - h.hp >= 0.5
        && h.hp > 0.0
    {
        glows.hurt = (glows.hurt + 0.35 + 2.0 * (was - h.hp) / h.max.max(1.0)).min(1.0);
    }
    glows.last_hp = Some(h.hp);
    // Healing: as much of it as is still to come.
    glows.heal = mending.map_or(0.0, |m| (m.left / m.total.max(0.01)).clamp(0.0, 1.0));
    // Fading in quick, out slower.
    let k = if glows.hurt > glows.hurt_shown { 1.0 - (-dt / 0.04).exp() } else { 1.0 - (-dt / 0.12).exp() };
    glows.hurt_shown += (glows.hurt - glows.hurt_shown) * k;
    glows.hurt *= (-dt / 0.35).exp();
    // (The green eases in over a moment, then follows the healing down
    // exactly, from the first frame it drops.)
    glows.heal_shown = if glows.heal > glows.heal_shown { glows.heal_shown + (glows.heal - glows.heal_shown) * (1.0 - (-dt / 0.05).exp()) } else { glows.heal };
    // Low health: a slow, smooth breath of red (a little quicker and
    // stronger the lower), never dropping to nothing between breaths.
    let share = h.hp / h.max.max(1.0);
    let k = if h.hp > 0.0 && share < LOW { (LOW - share) / LOW } else { 0.0 };
    let period = PULSE_SLOW + (PULSE_FAST - PULSE_SLOW) * k;
    glows.beat = (glows.beat + dt / period).fract();
    let want = if k > 0.0 { 0.16 + 0.22 * k } else { 0.0 };
    glows.low += (want - glows.low) * (1.0 - (-dt / 0.4).exp());
    let breath = 0.5 - 0.5 * (glows.beat * std::f32::consts::TAU).cos();
    let low = glows.low * (0.55 + 0.45 * breath);
    let red = (glows.hurt_shown * 0.55).max(low);
    // Healing: full as it's drunk, then dissolving in place: the soft band
    // fades quickest, the rim at the edge lingers, both smoothly to nothing.
    let k = glows.heal_shown.clamp(0.0, 1.0);
    let (green, green_rim) = (0.32 * k * k, 0.5 * (1.0 - (1.0 - k).powi(3)));
    for (edge, mut img) in &mut edges {
        img.color = match edge.0 {
            Kind::Red => Color::srgba(0.85, 0.05, 0.04, red * 0.7),
            Kind::RedRim => Color::srgba(0.9, 0.08, 0.06, red * 0.55),
            Kind::Green => Color::srgba(0.25, 0.95, 0.35, green),
            Kind::GreenRim => Color::srgba(0.35, 1.0, 0.45, green_rim),
        };
    }
}
