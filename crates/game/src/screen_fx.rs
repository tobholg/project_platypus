//! The screen's edges, telling how you are: a red flash fading in and out
//! when you're hurt (stronger the harder), a red pulse while your health
//! is low (quicker and stronger the lower), and a lush green glow while a
//! potion heals you: full as it starts, drawing back into the edges as the
//! healing runs out, from the moment it's drunk. Two overlays over the whole view (red, green), their colour
//! at the edges only, fading to nothing towards the middle.

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

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Red,
    Green,
}

/// How strong each is now (0..1), and where they're heading.
#[derive(Resource, Default)]
struct Glows {
    hurt: f32,
    hurt_shown: f32,
    heal: f32,
    heal_shown: f32,
    last_hp: Option<f32>,
}

/// Low health: below this share of it the edges pulse.
const LOW: f32 = 0.3;
/// How deep the glow reaches in from every edge (a share of the screen's
/// height).
const BAND: f32 = 0.26;

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

/// The green glow's vignettes, thinnest first: as a potion's healing runs
/// out, the glow draws back into the edges.
#[derive(Resource)]
struct Shrinking(Vec<Handle<Image>>);

const STEPS: usize = 24;

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let full = images.add(vignette(BAND));
    let steps: Vec<Handle<Image>> = (1..=STEPS).map(|k| images.add(vignette(BAND * k as f32 / STEPS as f32))).collect();
    for kind in [Kind::Red, Kind::Green] {
        commands.spawn((
            Edge(kind),
            // (Stretched to the node: by default an image keeps its own
            // proportions, and a window taller than 16:9 left bars at the
            // top and bottom uncovered.)
            ImageNode::new(if kind == Kind::Red { full.clone() } else { steps[STEPS - 1].clone() }).with_color(Color::NONE).with_mode(bevy::ui::widget::NodeImageMode::Stretch),
            // (Pinned to all four edges of the window.)
            Node { position_type: PositionType::Absolute, left: Val::Px(0.0), right: Val::Px(0.0), top: Val::Px(0.0), bottom: Val::Px(0.0), ..default() },
            // (Under the rest of the interface; never takes a click.)
            GlobalZIndex(-50),
            bevy::ui::FocusPolicy::Pass,
        ));
    }
    commands.insert_resource(Shrinking(steps));
}

fn show(time: Res<Time<Real>>, mut glows: ResMut<Glows>, shrinking: Res<Shrinking>, player: Query<(&Health, Option<&crate::potion::Mending>), With<LocalPlayer>>, mut edges: Query<(&Edge, &mut ImageNode)>) {
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
    // (The green follows the healing at once: its band's depth is how
    // much is left, from the first frame.)
    glows.heal_shown = glows.heal;
    // Low health: a pulse, a heartbeat's pace, quicker and stronger the
    // lower.
    let share = h.hp / h.max.max(1.0);
    let low = if h.hp > 0.0 && share < LOW {
        let k = (LOW - share) / LOW;
        let beat = (time.elapsed_secs() * std::f32::consts::TAU * (0.8 + 0.7 * k)).sin() * 0.5 + 0.5;
        (0.18 + 0.32 * k) * (0.35 + 0.65 * beat * beat)
    } else {
        0.0
    };
    let red = (glows.hurt_shown * 0.65).max(low);
    // Healing: the green's band as deep as what's left of it, drawing back
    // into the edges; a touch fainter as it goes.
    let left = glows.heal_shown;
    let green = if left > 0.0 { 0.35 + 0.15 * left } else { 0.0 };
    let step = ((left * STEPS as f32).ceil() as usize).clamp(1, STEPS) - 1;
    for (edge, mut img) in &mut edges {
        match edge.0 {
            Kind::Red => img.color = Color::srgba(0.85, 0.05, 0.04, red),
            Kind::Green => {
                img.color = Color::srgba(0.25, 0.95, 0.35, green);
                if img.image != shrinking.0[step] {
                    img.image = shrinking.0[step].clone();
                }
            }
        }
    }
}
