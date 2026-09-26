//! Dressing a rig: what's worn, drawn on it. A `Skin` is written once and
//! fits every creature on the same rig (they share part names and what
//! their palette letters stand for: `t` the tunic, `p` the trousers, `B` the
//! boots, `s` the skin...):
//!
//! - `recolor`: in the parts named (all, if none), a palette letter drawn in
//!   another colour: mail turns the tunic and sleeves steel, gloves turn the
//!   skin of the hands leather;
//! - `over`: rows drawn over the parts named, wherever they're drawn (every
//!   pose, every angle of an aiming arm), its top-left at `at` in the part's
//!   own pixels: a helm over the head.
//!
//! Part names may end in `*` (`arm*`, `aim*`: every arm, every aiming arm).

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{ArtFile, Color, Part};

/// How a piece of gear looks worn.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Skin {
    /// The colours its overlays are drawn in (its own letters: they don't
    /// meet the wearer's).
    #[serde(default)]
    pub palette: BTreeMap<char, Color>,
    #[serde(default)]
    pub recolor: Vec<Recolor>,
    #[serde(default)]
    pub over: Vec<Overlay>,
}

/// The wearer's letters, in new colours, in some parts.
#[derive(Clone, Debug, Deserialize)]
pub struct Recolor {
    #[serde(default)]
    pub parts: Vec<String>,
    pub colors: BTreeMap<char, Color>,
}

/// Rows drawn over some parts.
#[derive(Clone, Debug, Deserialize)]
pub struct Overlay {
    pub parts: Vec<String>,
    #[serde(default)]
    pub at: (i32, i32),
    pub rows: Vec<String>,
    /// Only where the part itself is drawn (a texture that follows any
    /// torso's shape: mail's rings, a jerkin's laces).
    #[serde(default)]
    pub clip: bool,
}

/// Does a part name match a pattern (`arm*`: any starting `arm`)?
fn matches(pattern: &str, name: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => pattern == name,
    }
}

fn picks(patterns: &[String], name: &str) -> bool {
    patterns.is_empty() || patterns.iter().any(|p| matches(p, name))
}

/// A sprite with skins on, in order (later ones over earlier ones). Their
/// colours get letters of their own (from the private-use range), so
/// nothing meets the wearer's.
pub fn dress(file: &ArtFile, skins: &[&Skin]) -> Result<ArtFile, String> {
    let mut out = file.clone();
    let mut next = 0xE000u32;
    let mut fresh = |palette: &mut BTreeMap<char, Color>, c: Color| -> Result<char, String> {
        loop {
            let ch = char::from_u32(next).ok_or("out of letters")?;
            next += 1;
            if let std::collections::btree_map::Entry::Vacant(e) = palette.entry(ch) {
                e.insert(c);
                return Ok(ch);
            }
        }
    };
    let names: Vec<String> = file.parts.keys().cloned().collect();
    for (n, skin) in skins.iter().enumerate() {
        for r in &skin.recolor {
            let mut to: BTreeMap<char, char> = BTreeMap::new();
            for (&from, &c) in &r.colors {
                to.insert(from, fresh(&mut out.palette, c)?);
            }
            for name in names.iter().filter(|name| picks(&r.parts, name)) {
                let part = out.parts.get_mut(name).expect("listed");
                for row in &mut part.rows {
                    *row = row.chars().map(|c| to.get(&c).copied().unwrap_or(c)).collect();
                }
            }
        }
        let mut own: BTreeMap<char, char> = BTreeMap::new();
        for (&c, &col) in &skin.palette {
            own.insert(c, fresh(&mut out.palette, col)?);
        }
        for (k, o) in skin.over.iter().enumerate() {
            let rows: Vec<String> = o.rows.iter().map(|row| row.chars().map(|c| if c == crate::CLEAR { c } else { own.get(&c).copied().unwrap_or(c) }).collect::<String>()).collect();
            if let Some((y, bad)) = rows.iter().enumerate().find_map(|(y, r)| r.chars().find(|c| *c != crate::CLEAR && !out.palette.contains_key(c)).map(|c| (y, c))) {
                return Err(format!("skin overlay {k}: row {y}: '{bad}' isn't in its palette"));
            }
            for name in names.iter().filter(|name| o.parts.iter().any(|p| matches(p, name))) {
                let base = &file.parts[name];
                let under =
                    |x: i32, y: i32| usize::try_from(y).ok().and_then(|y| base.rows.get(y)).and_then(|r| usize::try_from(x).ok().and_then(|x| r.chars().nth(x))).is_some_and(|c| c != crate::CLEAR);
                let rows: Vec<String> = if o.clip {
                    rows.iter().enumerate().map(|(y, r)| r.chars().enumerate().map(|(x, c)| if under(x as i32 + o.at.0, y as i32 + o.at.1) { c } else { crate::CLEAR }).collect()).collect()
                } else {
                    rows.clone()
                };
                let id = format!("{name}+{n}.{k}");
                out.parts.insert(id.clone(), Part { pivot: (base.pivot.0 - o.at.0, base.pivot.1 - o.at.1), points: BTreeMap::new(), rows });
                out.over.entry(name.clone()).or_default().push(id);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, parse};

    const RIG: &str = r#"(
        size: (8, 8), feet: (4, 8),
        palette: { 't': (0, 200, 0), 's': (200, 150, 120) },
        parts: {
            "head": (rows: ["ss", "ss"]),
            "arm": (pivot: (0, 0), rows: ["t", "s"]),
            "aim0": (pivot: (0, 0), rows: ["ts"]),
        },
        poses: { "stand": [ (part: "head", at: (2, 1)), (part: "arm", at: (4, 3), tag: "front_arm") ] },
        fans: { "front_arm": [ (angle: 0, part: "aim0") ] },
        clips: { "idle": (frames: ["stand"], fps: 1) },
    )"#;

    #[test]
    fn a_skin_recolours_its_parts_and_draws_over_them_everywhere() {
        let file = parse(RIG).unwrap();
        let skin: Skin = ron::from_str(
            r#"(
            palette: { 'h': (90, 90, 100) },
            recolor: [ (parts: ["arm*", "aim*"], colors: { 's': (80, 50, 30) }), (colors: { 't': (150, 150, 160) }) ],
            over: [ (parts: ["head"], at: (0, -1), rows: ["hh"]), (parts: ["head"], at: (-1, 0), rows: ["hhhh"], clip: true) ],
        )"#,
        )
        .unwrap();
        let art = compile(&dress(&file, &[&skin]).unwrap()).unwrap();
        let stand = &art.frames[art.index("stand").unwrap()];
        // The helm sits a row above the head's top-left (2, 1).
        assert_eq!(stand.get(2, 0), [90, 90, 100, 255]);
        assert_eq!(stand.get(2, 1), [90, 90, 100, 255], "a clipped band over the head's first row");
        assert_eq!(stand.get(1, 1), [0; 4], "but not past its side");
        assert_eq!(stand.get(2, 2), [200, 150, 120, 255], "the face stays skin");
        // The sleeve is mail; the hand is gloved.
        assert_eq!(stand.get(4, 3), [150, 150, 160, 255]);
        assert_eq!(stand.get(4, 4), [80, 50, 30, 255]);
        // And the aiming arm too.
        let aim = &art.frames[art.index("front_arm@0").unwrap()];
        let (px, py) = art.fan_pivot;
        assert_eq!(aim.get(px + 1, py), [80, 50, 30, 255]);
    }
}
