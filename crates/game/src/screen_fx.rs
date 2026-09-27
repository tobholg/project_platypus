//! The screen's edges, telling how you are: a red flash fading in and out
//! when you're hurt (stronger the harder), a red pulse while your health
//! is low (quicker and stronger the lower), and a lush green glow when a
//! potion heals you. Two overlays over the whole view (red, green), their
//! colour at the edges only, fading to nothing towards the middle.

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

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // The vignette: clear in the middle, rising towards the edges (an
    // ellipse, the corners strongest), smooth (sampled linearly).
    let (w, h) = (160u32, 90u32);
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let (u, v) = ((x as f32 + 0.5) / w as f32 * 2.0 - 1.0, (y as f32 + 0.5) / h as f32 * 2.0 - 1.0);
            // (Rounded-rectangle distance: the edges, not a circle.)
            let d = (u.abs().powf(4.0) + v.abs().powf(4.0)).powf(0.25);
            let t = ((d - 0.62) / 0.4).clamp(0.0, 1.0);
            let a = t * t * (3.0 - 2.0 * t);
            px.extend([255, 255, 255, (a * 255.0) as u8]);
        }
    }
    let mut image = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, px, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    image.sampler = ImageSampler::linear();
    let image = images.add(image);
    for kind in [Kind::Red, Kind::Green] {
        commands.spawn((
            Edge(kind),
            ImageNode::new(image.clone()).with_color(Color::NONE),
            Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            // (Under the rest of the interface; never takes a click.)
            GlobalZIndex(-50),
            bevy::ui::FocusPolicy::Pass,
        ));
    }
}

fn show(time: Res<Time<Real>>, mut glows: ResMut<Glows>, mut drank: MessageReader<crate::potion::Drank>, player: Query<(Entity, &Health), With<LocalPlayer>>, mut edges: Query<(&Edge, &mut ImageNode)>) {
    let dt = time.delta_secs().min(0.1);
    let Ok((me, h)) = player.single() else { return };
    // Hurt: a flash by how much of your health went (a scratch faint, a
    // big blow strong).
    if let Some(was) = glows.last_hp
        && was - h.hp >= 0.5
        && h.hp > 0.0
    {
        glows.hurt = (glows.hurt + 0.35 + 2.0 * (was - h.hp) / h.max.max(1.0)).min(1.0);
    }
    glows.last_hp = Some(h.hp);
    for d in drank.read() {
        if d.who == me {
            // (A bigger heal, a stronger glow.)
            glows.heal = (0.6 + d.heal / h.max.max(1.0)).min(1.0);
        }
    }
    // Fading in quick, out slower.
    let ease = |shown: f32, target: f32, up: f32, down: f32| {
        let k = if target > shown { 1.0 - (-dt / up).exp() } else { 1.0 - (-dt / down).exp() };
        shown + (target - shown) * k
    };
    glows.hurt_shown = ease(glows.hurt_shown, glows.hurt, 0.04, 0.12);
    glows.hurt *= (-dt / 0.35).exp();
    glows.heal_shown = ease(glows.heal_shown, glows.heal, 0.12, 0.25);
    glows.heal *= (-dt / 0.7).exp();
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
    let green = glows.heal_shown * 0.45;
    for (edge, mut img) in &mut edges {
        img.color = match edge.0 {
            Kind::Red => Color::srgba(0.85, 0.05, 0.04, red),
            Kind::Green => Color::srgba(0.25, 0.95, 0.35, green),
        };
    }
}
