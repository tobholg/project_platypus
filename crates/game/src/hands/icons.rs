//! Item icons: 16 × 16 pixel art from `assets/data/icons.ron` (text grids,
//! a shape and a palette per item), and for blocks a little block of their
//! material. Hot-reloaded. An item with neither shows its colour.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use platypus_sim::{Cell, MaterialTable};
use serde::Deserialize;

use super::items::{ItemId, Items, Use};
use crate::data::{Watched, data_path, load_ron};
use crate::world::SimWorld;

/// Pixels across an icon.
pub const ICON: usize = 16;

#[derive(Clone, Debug, Deserialize)]
struct IconDef {
    shape: String,
    palette: HashMap<char, (u8, u8, u8)>,
}

#[derive(Clone, Debug, Deserialize)]
struct IconsFile {
    outline: (u8, u8, u8),
    shapes: HashMap<String, Vec<String>>,
    icons: HashMap<String, IconDef>,
}

/// Each item's icon, by id (`None`: show its colour).
#[derive(Resource)]
pub struct Icons {
    by_item: Vec<Option<Handle<Image>>>,
    watch: Watched,
}

impl Icons {
    pub fn get(&self, item: ItemId) -> Option<&Handle<Image>> {
        self.by_item.get(item.0 as usize)?.as_ref()
    }
}

/// An icon's pixels (RGBA, top row first) from its shape and palette.
fn draw(file: &IconsFile, name: &str, def: &IconDef) -> Result<Vec<u8>, String> {
    let rows = file.shapes.get(&def.shape).ok_or(format!("icon `{name}`: no shape `{}`", def.shape))?;
    if rows.len() != ICON || rows.iter().any(|r| r.chars().count() != ICON) {
        return Err(format!("shape `{}`: {ICON} rows of {ICON}", def.shape));
    }
    let mut data = Vec::with_capacity(ICON * ICON * 4);
    for c in rows.iter().flat_map(|r| r.chars()) {
        let rgba = match c {
            '.' => [0, 0, 0, 0],
            'o' => [file.outline.0, file.outline.1, file.outline.2, 255],
            c => {
                let &(r, g, b) = def.palette.get(&c).ok_or(format!("icon `{name}`: `{c}` isn't in its palette"))?;
                [r, g, b, 255]
            }
        };
        data.extend(rgba);
    }
    Ok(data)
}

/// A block's icon: a square of its material (each pixel a shade of it),
/// lit from the top left, with the outline.
fn block_icon(mats: &MaterialTable, cell: Cell, outline: (u8, u8, u8)) -> Vec<u8> {
    let mut data = vec![0u8; ICON * ICON * 4];
    for y in 1..ICON - 1 {
        for x in 1..ICON - 1 {
            let edge = x == 1 || y == 1 || x == ICON - 2 || y == ICON - 2;
            let rgba = if edge {
                [outline.0, outline.1, outline.2, 255]
            } else {
                let h = platypus_sim::rng::hash(&[x as u64, y as u64, cell.material.0 as u64]);
                let mut c = mats.color(Cell { shade: (h % 256) as u8, ..cell });
                let k = if x == 2 || y == 2 { 1.18 } else if x == ICON - 3 || y == ICON - 3 { 0.72 } else { 1.0 };
                for ch in &mut c[..3] {
                    *ch = (*ch as f32 * k).min(255.0) as u8;
                }
                c[3] = 255;
                c
            };
            let i = (y * ICON + x) * 4;
            data[i..i + 4].copy_from_slice(&rgba);
        }
    }
    data
}

fn image(data: Vec<u8>) -> Image {
    Image::new(
        Extent3d { width: ICON as u32, height: ICON as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

fn build(file: &IconsFile, items: &Items, mats: &MaterialTable, images: &mut Assets<Image>) -> Vec<Option<Handle<Image>>> {
    (0..items.len())
        .map(|i| {
            let id = ItemId(i as u16);
            let def = items.def(id);
            if let Use::Block(m) = def.use_ {
                return Some(images.add(image(block_icon(mats, Cell::new(m, 128), file.outline))));
            }
            let icon = file.icons.get(&def.id)?;
            match draw(file, &def.id, icon) {
                Ok(data) => Some(images.add(image(data))),
                Err(e) => {
                    warn!("icons.ron: {e}");
                    None
                }
            }
        })
        .collect()
}

/// Once the items exist: every icon.
pub fn make_icons(mut commands: Commands, items: Option<Res<Items>>, icons: Option<Res<Icons>>, sim: Res<SimWorld>, mut images: ResMut<Assets<Image>>) {
    let (Some(items), None) = (items, icons) else { return };
    let path = data_path("icons.ron");
    let file: IconsFile = load_ron(&path).unwrap_or_else(|e| panic!("{e}"));
    let by_item = build(&file, &items, sim.materials(), &mut images);
    commands.insert_resource(Icons { by_item, watch: Watched::new(path) });
}

pub fn reload_icons(icons: Option<ResMut<Icons>>, items: Option<Res<Items>>, sim: Res<SimWorld>, mut images: ResMut<Assets<Image>>) {
    let (Some(mut icons), Some(items)) = (icons, items) else { return };
    if !icons.watch.changed() {
        return;
    }
    match load_ron::<IconsFile>(icons.watch.path()) {
        Ok(file) => {
            icons.by_item = build(&file, &items, sim.materials(), &mut images);
            info!("icons reloaded");
        }
        Err(e) => warn!("icons not reloaded: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every shape is 16 × 16 and every icon's letters are in its palette.
    #[test]
    fn every_icon_draws() {
        let file: IconsFile = crate::data::parse_ron(include_str!("../../../../assets/data/icons.ron")).unwrap();
        for (name, def) in &file.icons {
            draw(&file, name, def).unwrap_or_else(|e| panic!("{e}"));
        }
        assert!(file.icons.contains_key("copper_pickaxe"));
    }
}
