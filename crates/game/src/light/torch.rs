//! Torches: planted in the world (G, the torch item) or held in the off
//! hand (L cycles to it, and to the flashlights: held in the off hand
//! too, `flashlight.ron`, the beam leaving its lens; both pointed at the
//! cursor). Drawn from `assets/art/torch.ron`; alight with a
//! flame of rising motes, a wisp of smoke and the odd ember from its `flame`
//! anchor (`lighting.ron`'s `fire`: looks only, the sim never sees them, so
//! a torch sets nothing alight), and a light that flickers like fire.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde::Deserialize;

use super::{Carry, LightSettings, LightSource, LightToggles, PlantedTorch, rgb};
use crate::creatures::body::animation::HandPos;
use crate::creatures::player::LocalPlayer;
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
/// rises (planted; held, it's `OffHandArt`).
#[derive(Resource)]
pub struct TorchArt {
    image: Handle<Image>,
    size: Vec2,
    flame: Vec2,
}

/// Burning: fire rises from `at` (from the entity, cells) as it goes.
#[derive(Component, Default)]
pub struct Flame {
    pub at: Vec2,
    /// Motes owed (flame, smoke, embers).
    owed: [f32; 3],
    /// A smaller fire than a torch's (a candle's): this share of its motes.
    pub share: Option<f32>,
}

impl Flame {
    /// Fire rising from `at` (from the entity, cells).
    pub fn at(at: Vec2) -> Self {
        Flame { at, owed: [0.0; 3], share: None }
    }

    /// A smaller fire at `at`: `share` of a torch's motes.
    pub fn small(at: Vec2, share: f32) -> Self {
        Flame { at, owed: [0.0; 3], share: Some(share) }
    }
}

/// What's in the player's off hand for light (a torch, a flashlight).
#[derive(Component)]
pub struct HeldTorch;

/// A torch held in the off hand: upright, leaning this far ahead (degrees).
const TORCH_LEAN: f32 = -12.0;

/// A safety lamp's flame in firedamp.
const SAFETY_BLUE: (u8, u8, u8) = (110, 160, 255);

/// The held torch's fire (a child at its head, wherever it points).
#[derive(Component)]
pub struct HeldFlame;

/// The off hand's lights turned to every angle, and where on them (cells
/// from the grip, as drawn: the torch upright, the flashlight pointing
/// right) the flame and the lens are.
#[derive(Resource)]
pub struct OffHandArt {
    torch: crate::combat::Turned,
    flashlight: crate::combat::Turned,
    lantern: crate::combat::Turned,
    flame: Vec2,
    lens: Vec2,
    /// A lantern's flame, inside its glass.
    glow: Vec2,
}

/// A held flashlight's lens, from the player's middle (cells): the beam
/// leaves from there (`compute_light`).
#[derive(Resource, Default)]
pub struct OffLight {
    pub lens: Option<Vec2>,
}

/// An art's anchor from its `grip` (cells, y up), first frame.
fn from_grip(art: &str, anchor: &str) -> Vec2 {
    let path = assets_dir().join("art").join(format!("{art}.ron"));
    let compiled = std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| platypus_art::parse(&t)).and_then(|f| platypus_art::compile(&f));
    let Ok(a) = compiled else { return Vec2::ZERO };
    let at = |n: &str| a.anchors.get(n).and_then(|m| m.get(&0)).copied();
    match (at(anchor), at("grip")) {
        (Some((x, y)), Some((gx, gy))) => Vec2::new((x - gx) as f32, (gy - y) as f32),
        _ => Vec2::ZERO,
    }
}

pub fn load_off_hand(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut layouts: ResMut<Assets<TextureAtlasLayout>>) {
    let turned = |art: &str, images: &mut Assets<Image>, layouts: &mut Assets<TextureAtlasLayout>| crate::combat::turned_art(art, None, images, layouts).unwrap_or_else(|e| panic!("{art}: {e}"));
    let torch = turned("torch", &mut images, &mut layouts);
    let flashlight = turned("flashlight", &mut images, &mut layouts);
    let lantern = turned("lantern", &mut images, &mut layouts);
    commands.insert_resource(OffHandArt { torch, flashlight, lantern, flame: from_grip("torch", "flame"), lens: from_grip("flashlight", "lens"), glow: from_grip("lantern", "flame") });
}

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
    commands.insert_resource(TorchArt { image, size, flame: from_centre("flame") });
}

/// A torch stuck in the ground, its foot at `at`.
pub fn plant_torch(commands: &mut Commands, at: Vec2, settings: &LightSettings, art: &TorchArt) -> Entity {
    let centre = Vec2::new(0.0, art.size.y / 2.0 - 1.0);
    commands
        .spawn((
        Name::new("Torch"),
        PlantedTorch,
        LightSource { color: rgb(settings.torch.color, settings.torch.strength), flicker: 1.0 },
        super::Haze(settings.torch.haze),
        Flame { at: centre + art.flame + Vec2::Y, ..default() },
        Transform::from_translation(at.extend(9.0)),
        Visibility::default(),
        children![(Sprite::from_image(art.image.clone()), Transform::from_translation(centre.extend(0.0)))],
    ))
        .id()
}

impl OffHandArt {
    /// What's held in the off hand (a torch, a flashlight, a lantern) as
    /// its own sprite, turned to `angle` (degrees: 0 right, 90 up), and
    /// where on it its light comes from (from its grip, turned with it).
    pub fn look(&self, carry: Carry, angle: f32) -> Option<(Sprite, Vec2)> {
        let (turned, tip, angle) = match carry {
            Carry::Torch => (&self.torch, self.flame, angle - 90.0),
            Carry::SmallBeam | Carry::BigBeam => (&self.flashlight, self.lens, angle),
            Carry::Lantern | Carry::SafetyLamp => (&self.lantern, self.glow, 0.0),
            Carry::Nothing => return None,
        };
        let (image, layout) = turned.atlas();
        let index = crate::combat::Turned::index(angle);
        let tip = Vec2::from_angle(crate::combat::Turned::angle_of(index).to_radians()).rotate(tip);
        Some((Sprite::from_atlas_image(image, TextureAtlas { layout, index }), tip))
    }
}

/// Fire rising from whatever burns.
pub fn burn(time: Res<Time>, settings: Res<LightSettings>, mut sparks: ResMut<Sparks>, mut flames: Query<(&GlobalTransform, &mut Flame)>) {
    let dt = time.delta_secs();
    let look = &settings.fire;
    for (tf, mut f) in &mut flames {
        let at = tf.translation().truncate() + f.at;
        let share = f.share.unwrap_or(1.0);
        for (i, e) in [&look.flame, &look.smoke, &look.embers].into_iter().enumerate() {
            f.owed[i] += e.count * share * dt;
            let n = f.owed[i].floor();
            f.owed[i] -= n;
            if n > 0.0 {
                sparks.emit(e, n as usize, at, Vec2::Y, Vec2::ZERO);
            }
        }
    }
}

type HeldOnly = (With<HeldTorch>, Without<LocalPlayer>, Without<HeldFlame>);

/// What's worn in the off hand (`LightToggles::carry`): a torch, a
/// flashlight or a lantern, gripped at the back arm's hand, in front of the
/// body so it shows; a torch or a flashlight pointed at the cursor (the side
/// the player faces, up to straight up or down; the torch burns from its
/// head wherever it points, a flashlight's beam leaves its lens), a lantern
/// hanging upright, glowing from inside, steady.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn hold(
    mut commands: Commands,
    toggles: Res<LightToggles>,
    settings: Res<LightSettings>,
    art: Option<Res<OffHandArt>>,
    cursor: Res<crate::camera::CursorWorld>,
    mut off: ResMut<OffLight>,
    mut was: Local<Option<Carry>>,
    player: Query<(Entity, &crate::creatures::Kinematics, Option<&HandPos>), With<LocalPlayer>>,
    mut held: Query<(Entity, &mut Transform, &mut Sprite), HeldOnly>,
    mut flames: Query<(&mut Transform, &mut LightSource, Option<&mut Flame>), (With<HeldFlame>, Without<HeldTorch>)>,
    firedamp: Option<Res<crate::hazards::Firedamp>>,
    (switched, mut sparks, mut flipped): (Res<super::switch::Switch>, ResMut<Sparks>, Local<u32>),
) {
    off.lens = None;
    let (Some(art), Ok((me, k, hand))) = (art, player.single()) else { return };
    let carry = toggles.carry;
    // (Another thing in hand: the old one goes, the new one comes.)
    if *was != Some(carry) {
        *was = Some(carry);
        for (e, ..) in &held {
            commands.entity(e).despawn();
        }
        let turned = match carry {
            Carry::Torch => &art.torch,
            Carry::SmallBeam | Carry::BigBeam => &art.flashlight,
            Carry::Lantern | Carry::SafetyLamp => &art.lantern,
            Carry::Nothing => return,
        };
        let (image, layout) = turned.atlas();
        let mut e = commands.spawn((Name::new("Held light"), HeldTorch, Sprite::from_atlas_image(image, TextureAtlas { layout, index: 0 }), Transform::default(), ChildOf(me)));
        if matches!(carry, Carry::Lantern | Carry::SafetyLamp) {
            e.with_child((Name::new("Held lantern's light"), HeldFlame, LightSource { color: rgb(settings.lamp.color, settings.lamp.strength), flicker: 0.0 }, super::Haze(settings.lamp.haze), Transform::default(), Visibility::default()));
        }
        if carry == Carry::Torch {
            e.with_child((
                Name::new("Held torch's fire"),
                HeldFlame,
                LightSource { color: rgb(settings.torch.color, settings.torch.strength), flicker: 1.0 },
                super::Haze(settings.torch.haze),
                Flame { at: Vec2::Y, ..default() },
                Transform::default(),
                Visibility::default(),
            ));
        }
        return;
    }
    let Ok((_, mut tf, mut sprite)) = held.single_mut() else { return };
    let facing = k.loco.facing;
    let hand_at = hand.and_then(|h| h.off).unwrap_or(Vec3::ZERO);
    // At the cursor, on the side it faces (straight ahead with none).
    let d = cursor.0.map_or(Vec2::X, |c| c - (k.body.pos + hand_at.truncate()));
    let aim = d.y.atan2(d.x.abs()).to_degrees().clamp(-90.0, 90.0);
    // (The torch is drawn upright: turned a quarter less.)
    let (angle, tip) = match carry {
        // (A torch is held up, leaning a little ahead, wherever you look.)
        Carry::Torch => (TORCH_LEAN, art.flame),
        // (A lantern hangs: upright, whichever way you look.)
        Carry::Lantern | Carry::SafetyLamp => (0.0, art.glow),
        _ => (aim, art.lens),
    };
    if let Some(atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = crate::combat::Turned::index(angle);
    }
    sprite.flip_x = facing < 0.0;
    tf.translation = hand_at + Vec3::new(0.0, 0.0, 0.02);
    let r = Vec2::from_angle(angle.to_radians()).rotate(tip);
    let tip_at = Vec2::new(r.x * facing, r.y);
    if matches!(carry, Carry::Torch | Carry::Lantern | Carry::SafetyLamp) {
        let lamp = if carry == Carry::Torch { settings.torch } else { settings.lamp };
        // (A safety lamp near firedamp: its flame burns blue, and taller.)
        let near = if carry == Carry::SafetyLamp { firedamp.map_or(0.0, |f| f.near) } else { 0.0 };
        // (Switched off, L: a torch snuffed, a lantern's wick down;
        // `switch.rs`.)
        let on = switched.hand_level(carry);
        for (mut ftf, mut light, flame) in &mut flames {
            ftf.translation = tip_at.extend(0.0);
            let warm = rgb(lamp.color, lamp.strength * (1.0 + 0.6 * near) * on);
            let blue = rgb(SAFETY_BLUE, lamp.strength * (1.0 + 0.6 * near) * on);
            light.color = std::array::from_fn(|i| warm[i] + (blue[i] - warm[i]) * near);
            if let Some(mut f) = flame {
                f.share = Some(on);
            }
        }
        // A torch snuffed puffs smoke; lit again, a spark.
        if *flipped != switched.hand.flips() {
            *flipped = switched.hand.flips();
            if carry == Carry::Torch {
                let at = k.body.pos + hand_at.truncate() + tip_at;
                let (look, n) = if switched.hand.on { (&settings.fire.embers, 8) } else { (&settings.fire.smoke, 16) };
                sparks.emit(look, n, at, Vec2::Y, Vec2::ZERO);
            }
        }
    } else {
        off.lens = Some(hand_at.truncate() + tip_at);
    }
}
