//! What's worn, drawn on the wearer (DESIGN §7c): a creature drawn from a
//! text sprite on the humanoid rig is drawn again with the skins of what it
//! wears (`platypus_art::dress`), feet up to head, and plays that instead.
//! Each outfit is made once per kind of creature (`Wardrobe`) and shared.
//!
//! A piece's icon is the same drawing: what changes on a bare mannequin
//! (the player's sprite) when it's put on.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use platypus_art::dress::Skin;

use super::{Equipment, Slot, WORN};
use crate::creatures::Creature;
use crate::creatures::body::animation::Animator;
use crate::creatures::def::{CreatureDef, Creatures, set_art};
use crate::creatures::body::parts::Limb;
use crate::hands::items::Items;

/// Outfits made so far: (creature kind, what's worn) → the creature drawn
/// in it.
#[derive(Resource, Default)]
pub struct Wardrobe(HashMap<String, Arc<CreatureDef>>);

/// The order skins go on: what's under first.
const LAYERS: [Slot; 6] = [Slot::Feet, Slot::Legs, Slot::Body, Slot::Hands, Slot::Head, Slot::Trinket];

/// The skins of what's worn, under to over, and a name for the outfit.
fn outfit<'a>(items: &'a Items, eq: &Equipment) -> (Vec<&'a Skin>, String) {
    let mut skins = Vec::new();
    let mut name = String::new();
    for layer in LAYERS {
        for (i, s) in eq.worn.iter().enumerate() {
            let Some(s) = s else { continue };
            if WORN[i] != layer {
                continue;
            }
            let def = items.def(s.item);
            if let Some(look) = def.gear.as_ref().and_then(|g| g.look.as_ref()) {
                skins.push(look);
                name.push('+');
                name.push_str(&def.id);
            }
        }
    }
    (skins, name)
}

/// The skins of these things (what a body wore), under to over, wherever
/// each is worn (what's held has none).
pub fn skins_of<'a>(items: &'a Items, stacks: &[crate::hands::items::Stack]) -> Vec<&'a Skin> {
    let mut out = Vec::new();
    for layer in LAYERS {
        for s in stacks {
            let def = items.def(s.item);
            if let Some(g) = def.gear.as_ref().filter(|g| g.slot == layer)
                && let Some(look) = g.look.as_ref()
            {
                out.push(look);
            }
        }
    }
    out
}

/// Its art without the limbs it's lost (`wounds`): the layers of its
/// weapon arm (`front_arm`, and its aim's fan), its other arm (`back_arm`),
/// its head.
fn without(file: &mut platypus_art::ArtFile, lost: &[Limb]) {
    let gone = |l: &platypus_art::Layer| {
        lost.iter().any(|x| match x {
            Limb::Arm(0) => l.tag.as_deref() == Some("front_arm"),
            Limb::Arm(1) => l.tag.as_deref() == Some("back_arm"),
            Limb::Head => l.part.contains("head") || l.part == "blink",
            _ => false,
        })
    };
    for layers in file.poses.values_mut() {
        layers.retain(|l| !gone(l));
    }
    if lost.contains(&Limb::Arm(0)) {
        file.fans.remove("front_arm");
    }
}

/// Draw a creature in skins (its art as written, dressed and compiled),
/// without what it's lost.
fn dressed(base: &CreatureDef, kind: &str, skins: &[&Skin], name: &str, lost: &[Limb]) -> Result<CreatureDef, String> {
    let file = base.art_file.as_ref().ok_or("not drawn as text")?;
    let mut file = platypus_art::dress::dress(file, skins)?;
    without(&mut file, lost);
    let art = platypus_art::compile(&file)?;
    let mut def = base.clone();
    set_art(&mut def, &format!("{kind}{name}"), art);
    Ok(def)
}

type Dressing<'a> = (&'a Creature, Ref<'a, Equipment>, &'a mut Animator, Option<Ref<'a, crate::creatures::body::parts::Severed>>);

/// Whoever's outfit changed (or whose kind was reloaded) is drawn in it
/// (without what it's lost).
pub fn dress(items: Option<Res<Items>>, creatures: Res<Creatures>, mut wardrobe: ResMut<Wardrobe>, mut q: Query<Dressing>) {
    let Some(items) = items else { return };
    let all = creatures.is_changed();
    if all {
        wardrobe.0.clear();
    }
    for (c, eq, mut anim, severed) in &mut q {
        if !(all || eq.is_changed() || severed.as_ref().is_some_and(|s| s.is_changed())) {
            continue;
        }
        // (Only what's drawn in its art's layers: a humanoid's arms, head.)
        let lost: Vec<Limb> = severed.map(|s| s.0.iter().copied().filter(|l| matches!(l, Limb::Arm(0) | Limb::Arm(1) | Limb::Head)).collect()).unwrap_or_default();
        let Some(base) = creatures.get(&c.kind) else {
            continue;
        };
        if base.art_file.is_none() {
            continue;
        }
        let (skins, mut name) = outfit(&items, &eq);
        for l in &lost {
            name.push_str(&format!("-{l:?}"));
        }
        let want = if skins.is_empty() && lost.is_empty() {
            base.clone()
        } else {
            let key = format!("{}{name}", c.kind);
            match wardrobe.0.get(&key) {
                Some(d) => d.clone(),
                None => match dressed(base, &c.kind, &skins, &name, &lost) {
                    Ok(d) => {
                        let d = Arc::new(d);
                        wardrobe.0.insert(key, d.clone());
                        d
                    }
                    Err(e) => {
                        warn!("{} in{name}: {e}", c.kind);
                        base.clone()
                    }
                },
            }
        };
        if !Arc::ptr_eq(&anim.def, &want) {
            anim.def = want;
            anim.refresh();
        }
    }
}

/// A piece's icon: what it changes on the mannequin (standing), outlined.
pub fn icon(mannequin: &platypus_art::ArtFile, skin: &Skin) -> Option<platypus_art::Pixels> {
    let bare = platypus_art::compile(mannequin).ok()?;
    let worn = platypus_art::compile(&platypus_art::dress::dress(mannequin, &[skin]).ok()?).ok()?;
    let f = bare.index("stand")?;
    let (a, b) = (&bare.frames[f], &worn.frames[f]);
    let mut out = platypus_art::Pixels::new(a.w, a.h);
    for y in 0..a.h as i32 {
        for x in 0..a.w as i32 {
            if a.get(x, y) != b.get(x, y) && b.opaque(x, y) {
                out.set(x, y, b.get(x, y));
            }
        }
    }
    let oc = mannequin.outline.unwrap_or((26, 20, 20));
    let mut edged = out.clone();
    for y in 0..a.h as i32 {
        for x in 0..a.w as i32 {
            if !out.opaque(x, y) && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| out.opaque(x + dx, y + dy)) {
                edged.set(x, y, [oc.0, oc.1, oc.2, 255]);
            }
        }
    }
    Some(edged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gear::GearFile;

    /// Every piece's look goes on every humanoid, and changes something
    /// (what's worn on the legs and feet, where it has legs drawn: a lanky
    /// skeleton's are its `legs`').
    #[test]
    fn every_look_dresses_every_humanoid() {
        let gear: GearFile = crate::data::parse_ron(include_str!("../../../../assets/data/gear.ron")).unwrap();
        for rig in [include_str!("../../../../assets/art/player.ron"), include_str!("../../../../assets/art/orc.ron"), include_str!("../../../../assets/art/skeleton.ron")] {
            let file = platypus_art::parse(rig).unwrap();
            let bare = platypus_art::compile(&file).unwrap();
            for item in &gear.items {
                let Some(look) = item.gear.as_ref().and_then(|g| g.look.as_ref()) else {
                    continue;
                };
                if !file.parts.contains_key("legs") && item.gear.as_ref().is_some_and(|g| matches!(g.slot, Slot::Legs | Slot::Feet)) {
                    continue;
                }
                let art = platypus_art::compile(&platypus_art::dress::dress(&file, &[look]).unwrap_or_else(|e| panic!("{}: {e}", item.id))).unwrap_or_else(|e| panic!("{}: {e}", item.id));
                let f = art.index("stand").unwrap();
                assert_ne!(art.frames[f].rgba, bare.frames[f].rgba, "{} changes nothing", item.id);
            }
        }
        let player = platypus_art::parse(include_str!("../../../../assets/art/player.ron")).unwrap();
        let helm = gear.items.iter().find(|i| i.id == "iron_helm").unwrap();
        let p = icon(&player, helm.gear.as_ref().unwrap().look.as_ref().unwrap()).unwrap();
        assert!(p.rgba.chunks(4).filter(|c| c[3] > 0).count() > 10, "the helm's icon shows the helm");
    }
}
