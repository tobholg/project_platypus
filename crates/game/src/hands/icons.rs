//! Item icons: 16 × 16 pixel art from `assets/data/icons.ron` (text grids,
//! a shape and a palette per item), and for blocks a little block of their
//! material; a weapon is its sprite, a piece of gear what it looks like worn
//! (`gear::look::icon`). Hot-reloaded. An item with none of these shows its
//! colour.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use platypus_sim::{Cell, MaterialTable};
use serde::Deserialize;

use super::items::{ItemId, Items, Use};
use crate::actors::creature::Creatures;
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

/// A picture in an icon: its drawn part, centred (cut to fit if bigger;
/// small ones, under half the icon, twice the size).
fn fit(p: &platypus_art::Pixels) -> Vec<u8> {
    let opaque = |x: i32, y: i32| p.opaque(x, y);
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for y in 0..p.h as i32 {
        for x in 0..p.w as i32 {
            if opaque(x, y) {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            }
        }
    }
    let mut data = vec![0u8; ICON * ICON * 4];
    if x1 < x0 {
        return data;
    }
    let k = if (x1 - x0).max(y1 - y0) < ICON as i32 / 2 { 2 } else { 1 };
    let (cx, cy) = ((x0 + x1 + 1) / 2, (y0 + y1 + 1) / 2);
    let h = ICON as i32 / 2;
    for y in 0..ICON as i32 {
        for x in 0..ICON as i32 {
            let c = p.get(cx + (x - h).div_euclid(k), cy + (y - h).div_euclid(k));
            let i = (y as usize * ICON + x as usize) * 4;
            data[i..i + 4].copy_from_slice(&c);
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

fn build(file: &IconsFile, items: &Items, mats: &MaterialTable, mannequin: Option<&platypus_art::ArtFile>, images: &mut Assets<Image>) -> Vec<Option<Handle<Image>>> {
    (0..items.len())
        .map(|i| {
            let id = ItemId(i as u16);
            let def = items.def(id);
            if let Use::Block(m) = def.use_ {
                return Some(images.add(image(block_icon(mats, Cell::new(m, 128), file.outline))));
            }
            // A weapon is its own sprite, turned to point up and forward.
            if let Use::Melee(w) | Use::Bow(w) = &def.use_
                && let Some(p) = crate::combat::icon(w)
            {
                return Some(images.add(image(fit(&p))));
            }
            // Gear: what it looks like on.
            if let (Some(look), Some(m)) = (def.gear.as_ref().and_then(|g| g.look.as_ref()), mannequin)
                && let Some(p) = crate::gear::look::icon(m, look)
            {
                return Some(images.add(image(fit(&p))));
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
pub fn make_icons(mut commands: Commands, items: Option<Res<Items>>, icons: Option<Res<Icons>>, sim: Res<SimWorld>, creatures: Res<Creatures>, mut images: ResMut<Assets<Image>>) {
    let (Some(items), None) = (items, icons) else { return };
    let path = data_path("icons.ron");
    let file: IconsFile = load_ron(&path).unwrap_or_else(|e| panic!("{e}"));
    let by_item = build(&file, &items, sim.materials(), mannequin(&creatures).as_deref(), &mut images);
    commands.insert_resource(Icons { by_item, watch: Watched::new(path) });
}

/// What gear is shown on: the player as drawn.
fn mannequin(creatures: &Creatures) -> Option<std::sync::Arc<platypus_art::ArtFile>> {
    creatures.get("player")?.art_file.clone()
}

pub fn reload_icons(icons: Option<ResMut<Icons>>, items: Option<Res<Items>>, sim: Res<SimWorld>, creatures: Res<Creatures>, mut images: ResMut<Assets<Image>>) {
    let (Some(mut icons), Some(items)) = (icons, items) else { return };
    if !icons.watch.changed() {
        return;
    }
    match load_ron::<IconsFile>(icons.watch.path()) {
        Ok(file) => {
            icons.by_item = build(&file, &items, sim.materials(), mannequin(&creatures).as_deref(), &mut images);
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
