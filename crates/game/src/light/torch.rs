//! Torches: planted in the world (G, the torch item) or held in the off
//! hand (L cycles to it). Drawn from `assets/art/torch.ron`; alight with a
//! flame of rising motes, a wisp of smoke and the odd ember from its `flame`
//! anchor (`lighting.ron`'s `fire`: looks only, the sim never sees them, so
//! a torch sets nothing alight), and a light that flickers like fire.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde::Deserialize;

use super::{Carry, LightSettings, LightSource, LightToggles, PlantedTorch, rgb};
use crate::actors::animation::HandPos;
use crate::actors::player::LocalPlayer;
use crate::data::assets_dir;
use crate::magic::runes::Emitter;
use crate::vfx::Sparks;

/// What a torch's fire looks like: each emitter's `count` is a second's
/// worth.
#[derive(Clone, Debug, Deserialize)]
pub struct FireLook {
    pub flame: Emitter,
    pub smoke: Emitter,
    pub embers: Emitter,
}

/// The torch's picture, and where on it (from its centre, cells) the flame
/// rises and the hand grips it.
#[derive(Resource)]
pub struct TorchArt {
    image: Handle<Image>,
    size: Vec2,
    flame: Vec2,
    grip: Vec2,
}

/// Burning: fire rises from `at` (from the entity, cells) as it goes.
#[derive(Component, Default)]
pub struct Flame {
    pub at: Vec2,
    /// Motes owed (flame, smoke, embers).
    owed: [f32; 3],
}

/// The torch in the player's off hand.
#[derive(Component)]
pub struct HeldTorch;

pub fn load_art(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let path = assets_dir().join("art").join("torch.ron");
    let art = std::fs::read_to_string(&path)
        .map_err(|e| e.to_string())
        .and_then(|t| platypus_art::parse(&t))
        .and_then(|f| platypus_art::compile(&f))
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let f = &art.frames[0];
    let size = Vec2::new(f.w as f32, f.h as f32);
    // A frame pixel's centre, from the picture's centre (y up).
    let from_centre = |name: &str| {
        art.anchors.get(name).and_then(|m| m.get(&0)).map_or(Vec2::ZERO, |&(x, y)| Vec2::new(x as f32 + 0.5 - size.x / 2.0, size.y / 2.0 - y as f32 - 0.5))
    };
    let image = images.add(Image::new(
        Extent3d { width: f.w, height: f.h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        f.rgba.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ));
    commands.insert_resource(TorchArt { image, size, flame: from_centre("flame"), grip: from_centre("grip") });
}

/// A torch stuck in the ground, its foot at `at`.
pub fn plant_torch(commands: &mut Commands, at: Vec2, settings: &LightSettings, art: &TorchArt) {
    let centre = Vec2::new(0.0, art.size.y / 2.0 - 1.0);
    commands.spawn((
        Name::new("Torch"),
        PlantedTorch,
        LightSource { color: rgb(settings.torch.color, settings.torch.strength), flicker: 1.0 },
        Flame { at: centre + art.flame + Vec2::Y, ..default() },
        Transform::from_translation(at.extend(9.0)),
        Visibility::default(),
        children![(Sprite::from_image(art.image.clone()), Transform::from_translation(centre.extend(0.0)))],
    ));
}

/// Fire rising from whatever burns.
pub fn burn(time: Res<Time>, settings: Res<LightSettings>, mut sparks: ResMut<Sparks>, mut flames: Query<(&GlobalTransform, &mut Flame)>) {
    let dt = time.delta_secs();
    let look = &settings.fire;
    for (tf, mut f) in &mut flames {
        let at = tf.translation().truncate() + f.at;
        for (i, e) in [&look.flame, &look.smoke, &look.embers].into_iter().enumerate() {
            f.owed[i] += e.count * dt;
            let n = f.owed[i].floor();
            f.owed[i] -= n;
            if n > 0.0 {
                sparks.emit(e, n as usize, at, Vec2::Y, Vec2::ZERO);
            }
        }
    }
}

type HeldOnly = (With<HeldTorch>, Without<LocalPlayer>);

/// With the torch picked (L), it's in the off hand: gripped at the back
/// arm's hand, in front of the body so it shows.
pub fn hold(
    mut commands: Commands,
    toggles: Res<LightToggles>,
    settings: Res<LightSettings>,
    art: Option<Res<TorchArt>>,
    player: Query<(Entity, &Transform, Option<&HandPos>), With<LocalPlayer>>,
    mut held: Query<(Entity, &mut Transform, &mut LightSource), HeldOnly>,
) {
    let (Some(art), Ok((me, _, hand))) = (art, player.single()) else { return };
    let want = toggles.carry == Carry::Torch;
    let at = hand.and_then(|h| h.off).unwrap_or(Vec3::ZERO) - art.grip.extend(0.0) + Vec3::new(0.0, 0.0, 0.02);
    match (want, held.single_mut()) {
        (true, Ok((_, mut tf, mut light))) => {
            tf.translation = at;
            light.color = rgb(settings.torch.color, settings.torch.strength);
        }
        (true, Err(_)) => {
            commands.entity(me).with_child((
                Name::new("Held torch"),
                HeldTorch,
                LightSource { color: rgb(settings.torch.color, settings.torch.strength), flicker: 1.0 },
                Flame { at: art.flame + Vec2::Y, ..default() },
                Sprite::from_image(art.image.clone()),
                Transform::from_translation(at),
            ));
        }
        (false, Ok((e, ..))) => commands.entity(e).despawn(),
        (false, Err(_)) => {}
    }
}
