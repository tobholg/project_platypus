//! Pixel canvases: an image over the camera's view that something draws a
//! cell at a time each frame (particles, spider legs). A thing made of
//! single cells is cheaper as pixels of one image than as quads of a mesh:
//! a mesh of tens of thousands of quads rebuilt every frame was the
//! costliest thing in a big fight; an image the size of the view costs the
//! same however much is in it.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Cells drawn past each edge of the view.
const MARGIN: f32 = 8.0;

/// Marks a canvas's sprite.
#[derive(Component)]
pub struct CanvasSprite;

pub type CanvasSprites<'w, 's> = Query<'w, 's, (&'static mut Sprite, &'static mut Transform), With<CanvasSprite>>;

/// One canvas: its image, its sprite, its size, whether it had anything on
/// it last frame (an empty one isn't redrawn), its depth.
pub struct Canvas {
    image: Handle<Image>,
    entity: Option<Entity>,
    size: UVec2,
    drawn: bool,
    z: f32,
    name: &'static str,
}

/// A frame's drawing on a canvas: cells in world coordinates.
pub struct Pixels<'a> {
    image: bevy::asset::AssetMut<'a, Image>,
    origin: IVec2,
    w: i32,
    h: i32,
}

impl Pixels<'_> {
    /// Colour the cell at (x, y) (sRGB, alpha), if it's on the canvas.
    #[inline]
    pub fn put(&mut self, x: i32, y: i32, c: [u8; 4]) {
        let (px, py) = (x - self.origin.x, y - self.origin.y);
        if px < 0 || py < 0 || px >= self.w || py >= self.h {
            return;
        }
        // (Image rows run top down.)
        let i = (((self.h - 1 - py) * self.w + px) * 4) as usize;
        if let Some(data) = self.image.data.as_mut() {
            data[i..i + 4].copy_from_slice(&c);
        }
    }
}

fn blank(size: UVec2) -> Image {
    Image::new(
        Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
        TextureDimension::D2,
        vec![0; (size.x * size.y * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

impl Canvas {
    pub fn new(name: &'static str, z: f32) -> Self {
        Canvas { image: Handle::default(), entity: None, size: UVec2::ZERO, drawn: false, z, name }
    }

    /// Start this frame's drawing over the view (centred at `centre`,
    /// `half` across): cleared, placed. None when there's nothing to draw
    /// and it's empty already.
    pub fn frame<'a>(&mut self, commands: &mut Commands, images: &'a mut Assets<Image>, sprites: &mut CanvasSprites, centre: Vec2, half: Vec2, anything: bool) -> Option<Pixels<'a>> {
        if !anything && !self.drawn {
            return None;
        }
        self.drawn = anything;
        let half = half + MARGIN;
        let size = (half * 2.0).ceil().as_uvec2().max(UVec2::ONE);
        let origin = (centre - half).floor();
        if size != self.size || self.entity.is_none() {
            let image = images.add(blank(size));
            match self.entity.and_then(|e| sprites.get_mut(e).ok()) {
                Some((mut s, _)) => {
                    s.image = image.clone();
                    s.custom_size = Some(size.as_vec2());
                }
                None => {
                    let sprite = Sprite { image: image.clone(), custom_size: Some(size.as_vec2()), ..default() };
                    self.entity = Some(commands.spawn((Name::new(self.name), CanvasSprite, sprite, Transform::from_xyz(0.0, 0.0, self.z))).id());
                }
            }
            self.image = image;
            self.size = size;
        }
        if let Some((_, mut tf)) = self.entity.and_then(|e| sprites.get_mut(e).ok()) {
            tf.translation = (origin + size.as_vec2() / 2.0).extend(self.z);
        }
        let mut image = images.get_mut(&self.image)?;
        image.data.as_mut()?.fill(0);
        Some(Pixels { image, origin: origin.as_ivec2(), w: size.x as i32, h: size.y as i32 })
    }
}
