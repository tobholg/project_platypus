//! Sprites as text (DESIGN §10, arc A): a palette of characters, frames as
//! grids of them, clips as lists of frames. Written by hand, by the model or
//! by the in-game editor; compiled to RGBA frames and an atlas the game draws
//! from; checked and described for whoever is writing it.
//!
//! ```ron
//! (
//!     size: (10, 8),            // every frame, in pixels (1 px = 1 cell)
//!     feet: (5, 7),             // the pixel standing on the ground
//!     outline: (24, 18, 16),    // drawn round every frame (optional)
//!     palette: { 'b': (150, 110, 80), 'w': (230, 225, 215) },
//!     frames: {
//!         "sit": [".....bb...", ...],          // '.' is clear
//!     },
//!     derived: {
//!         // another frame, moved and/or drawn over ('.' keeps, '_' clears)
//!         "sit_blink": (from: "sit", shift: (0, 0), over: [...]),
//!     },
//!     clips: { "idle": (frames: ["sit", "sit", "sit_blink"], fps: 4) },
//!     anchors: { "mouth": { "sit": (8, 3) } },
//! )
//! ```
//!
//! **Rigs.** A character is drawn as parts (a head, a torso, arms, legs in
//! a few drawn variants), each with a pivot (its joint) and named points (a
//! hand); a pose puts parts together, pivot at a spot, in order, some
//! mirrored or shaded (an arm behind the body). Poses are frames like any
//! other (outlined as one shape), and a part's points become the pose's
//! anchors, so a weapon finds the hand in every frame by itself:
//!
//! ```ron
//!     parts: {
//!         "arm": (pivot: (1, 0), points: { "hand": (1, 5) }, rows: [...]),
//!     },
//!     poses: {
//!         "stand": [(part: "arm", at: (8, 9), shade: 0.7), (part: "torso", at: (9, 12)), ...],
//!     },
//! ```

pub mod dress;
pub mod edit;
pub mod rotate;

use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;

/// Clear in a frame; in an overlay, leave what's under it.
pub const CLEAR: char = '.';
/// In an overlay: clear what's under it.
pub const ERASE: char = '_';

#[derive(Clone, Debug, Deserialize)]
pub struct ArtFile {
    pub size: (u32, u32),
    pub feet: (f32, f32),
    #[serde(default)]
    pub outline: Option<(u8, u8, u8)>,
    pub palette: BTreeMap<char, Color>,
    #[serde(default)]
    pub frames: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub derived: BTreeMap<String, Derived>,
    #[serde(default)]
    pub clips: BTreeMap<String, Clip>,
    #[serde(default)]
    pub anchors: BTreeMap<String, BTreeMap<String, (i32, i32)>>,
    #[serde(default)]
    pub parts: BTreeMap<String, Part>,
    #[serde(default)]
    pub poses: BTreeMap<String, Vec<Layer>>,
    /// A limb drawn at angles (degrees from level, up positive, facing
    /// right), each a part: the game shows the one nearest where the
    /// creature aims, its pivot at the pose's anchor of the same name, over
    /// the pose drawn without its layer of that tag.
    #[serde(default)]
    pub fans: BTreeMap<String, Vec<FanArm>>,
    /// Parts drawn over a part wherever it's drawn (in poses and fans),
    /// placed as it is: its pivot on that part's pivot. What's worn goes
    /// here (`dress`).
    #[serde(default)]
    pub over: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FanArm {
    pub angle: f32,
    pub part: String,
    /// The part is drawn pointing at this angle: turn it to `angle` (one
    /// straight arm makes the whole fan). None: drawn at `angle` already.
    #[serde(default)]
    pub from: Option<f32>,
}

/// A body part: a grid of any size, its pivot (the joint it hangs from) and
/// named points (a hand's grip), both from its top-left.
#[derive(Clone, Debug, Deserialize)]
pub struct Part {
    #[serde(default)]
    pub pivot: (i32, i32),
    #[serde(default)]
    pub points: BTreeMap<String, (i32, i32)>,
    pub rows: Vec<String>,
}

/// A part in a pose: its pivot at `at`; mirrored about its pivot; its
/// colours scaled by `shade` (0.7: an arm behind the body); with `outline`,
/// edged in the outline colour where it lies over what's drawn already (an
/// arm in front of the body stands out from it).
#[derive(Clone, Debug, Deserialize)]
pub struct Layer {
    pub part: String,
    pub at: (i32, i32),
    #[serde(default)]
    pub flip: bool,
    #[serde(default = "one")]
    pub shade: f32,
    #[serde(default)]
    pub outline: bool,
    /// Degrees to turn the part about its pivot (+ counter-clockwise, as
    /// facing right: up for an arm held out), RotSprite-style; its points
    /// turn with it. A limb swings without being drawn again.
    #[serde(default)]
    pub turn: f32,
    /// A name for the layer (`front_arm`, `back_arm`): its pivot is the
    /// pose's `<tag>` anchor and its points are also `<tag>.<point>`
    /// (`back_arm.hand`); where a fan stands in for it, the pose is also
    /// drawn without it (`<pose>~<tag>`).
    #[serde(default)]
    pub tag: Option<String>,
}

fn one() -> f32 {
    1.0
}

/// A colour: RGB, or RGBA for glass and glows.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Color {
    Rgb(u8, u8, u8),
    Rgba(u8, u8, u8, u8),
}

impl Color {
    pub fn rgba(self) -> [u8; 4] {
        match self {
            Color::Rgb(r, g, b) => [r, g, b, 255],
            Color::Rgba(r, g, b, a) => [r, g, b, a],
        }
    }
}

/// A frame made from another: moved by `shift` (x right, y down), mirrored,
/// then drawn over.
#[derive(Clone, Debug, Deserialize)]
pub struct Derived {
    pub from: String,
    #[serde(default)]
    pub shift: (i32, i32),
    #[serde(default)]
    pub flip: bool,
    #[serde(default)]
    pub over: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Clip {
    pub frames: Vec<String>,
    pub fps: f32,
    #[serde(default = "yes")]
    pub looping: bool,
}

fn yes() -> bool {
    true
}

/// One frame's pixels: RGBA, `w × h`, top row first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pixels {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

impl Pixels {
    pub fn new(w: u32, h: u32) -> Self {
        Pixels { w, h, rgba: vec![0; (w * h * 4) as usize] }
    }

    fn i(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && (x as u32) < self.w && (y as u32) < self.h).then(|| ((y as u32 * self.w + x as u32) * 4) as usize)
    }

    pub fn get(&self, x: i32, y: i32) -> [u8; 4] {
        self.i(x, y).map_or([0; 4], |i| [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]])
    }

    pub fn set(&mut self, x: i32, y: i32, c: [u8; 4]) {
        if let Some(i) = self.i(x, y) {
            self.rgba[i..i + 4].copy_from_slice(&c);
        }
    }

    pub fn opaque(&self, x: i32, y: i32) -> bool {
        self.get(x, y)[3] > 0
    }
}

/// A compiled sprite: every frame (by name, in a fixed order), the clips as
/// indices into them, anchors by frame index.
#[derive(Clone, Debug)]
pub struct Art {
    pub size: (u32, u32),
    pub feet: (f32, f32),
    pub names: Vec<String>,
    pub frames: Vec<Pixels>,
    pub clips: BTreeMap<String, CompiledClip>,
    pub anchors: BTreeMap<String, HashMap<usize, (i32, i32)>>,
    /// (frame, tag) → the frame drawn without that tag's layer.
    pub without: HashMap<(usize, String), usize>,
    /// Fans: (angle, frame) by tag, the frames drawn with their pivot at
    /// `fan_pivot`.
    pub fans: BTreeMap<String, Vec<(f32, usize)>>,
    pub fan_pivot: (i32, i32),
}

impl Art {
    /// The fan frame nearest `angle` (degrees).
    pub fn fan_frame(&self, tag: &str, angle: f32) -> Option<usize> {
        self.fans.get(tag)?.iter().min_by(|a, b| (a.0 - angle).abs().total_cmp(&(b.0 - angle).abs())).map(|f| f.1)
    }
}

#[derive(Clone, Debug)]
pub struct CompiledClip {
    pub frames: Vec<usize>,
    pub fps: f32,
    pub looping: bool,
}

impl Art {
    pub fn index(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    /// Every frame packed in a grid `columns` wide: (atlas, columns, rows).
    pub fn atlas(&self) -> (Pixels, u32, u32) {
        let n = self.frames.len().max(1) as u32;
        let cols = n.min(8);
        let rows = n.div_ceil(cols);
        let (w, h) = self.size;
        let mut atlas = Pixels::new(w * cols, h * rows);
        for (k, f) in self.frames.iter().enumerate() {
            let (ox, oy) = ((k as u32 % cols) * w, (k as u32 / cols) * h);
            for y in 0..h {
                for x in 0..w {
                    atlas.set((ox + x) as i32, (oy + y) as i32, f.get(x as i32, y as i32));
                }
            }
        }
        (atlas, cols, rows)
    }
}

/// Parse a sprite file.
pub fn parse(text: &str) -> Result<ArtFile, String> {
    ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME).from_str(text).map_err(|e| e.to_string())
}

/// A grid to pixels (`overlay`: '.' keeps, '_' clears).
fn draw(into: &mut Pixels, rows: &[String], palette: &BTreeMap<char, Color>, overlay: bool, what: &str) -> Result<(), String> {
    if rows.len() != into.h as usize {
        return Err(format!("{what}: {} rows, the size says {}", rows.len(), into.h));
    }
    for (y, row) in rows.iter().enumerate() {
        let n = row.chars().count();
        if n != into.w as usize {
            return Err(format!("{what}: row {y} is {n} wide, the size says {}", into.w));
        }
        for (x, c) in row.chars().enumerate() {
            let px = match c {
                CLEAR if overlay => continue,
                CLEAR => [0; 4],
                ERASE if overlay => [0; 4],
                c => palette.get(&c).ok_or(format!("{what}: row {y}: '{c}' isn't in the palette"))?.rgba(),
            };
            into.set(x as i32, y as i32, px);
        }
    }
    Ok(())
}

/// Outline every opaque shape: transparent pixels touching one (sideways or
/// up and down) take the colour.
fn outline(p: &Pixels, c: (u8, u8, u8)) -> Pixels {
    let mut out = p.clone();
    for y in 0..p.h as i32 {
        for x in 0..p.w as i32 {
            if !p.opaque(x, y) && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| p.opaque(x + dx, y + dy)) {
                out.set(x, y, [c.0, c.1, c.2, 255]);
            }
        }
    }
    out
}

/// Named points in a frame (a part's or a pose's).
type Points = BTreeMap<String, (i32, i32)>;

/// Compile a sprite file: every frame drawn (derived ones from theirs),
/// outlined, and the clips and anchors resolved. Errors name what's wrong.
pub fn compile(file: &ArtFile) -> Result<Art, String> {
    let (w, h) = file.size;
    if w == 0 || h == 0 {
        return Err("size: zero".into());
    }
    for c in file.palette.keys() {
        if *c == CLEAR || *c == ERASE {
            return Err(format!("palette: '{c}' is reserved ('.' clear, '_' erase)"));
        }
    }
    let mut drawn: BTreeMap<String, Pixels> = BTreeMap::new();
    for (name, rows) in &file.frames {
        let mut p = Pixels::new(w, h);
        draw(&mut p, rows, &file.palette, false, &format!("frame `{name}`"))?;
        drawn.insert(name.clone(), p);
    }
    // Parts, then poses put together from them (and their points).
    let mut parts: BTreeMap<&str, Pixels> = BTreeMap::new();
    for (name, part) in &file.parts {
        let ph = part.rows.len() as u32;
        let pw = part.rows.first().map_or(0, |r| r.chars().count()) as u32;
        if pw == 0 {
            return Err(format!("part `{name}`: empty"));
        }
        let mut p = Pixels::new(pw, ph);
        draw(&mut p, &part.rows, &file.palette, false, &format!("part `{name}`"))?;
        parts.insert(name, p);
    }
    let mut pose_points: BTreeMap<String, Points> = BTreeMap::new();
    // A pose (or a fan arm) drawn: its layers in order, those tagged `skip`
    // left out; the points of what's drawn, and each tagged layer's pivot.
    let compose = |layers: &[Layer], skip: Option<&str>, what: &str| -> Result<(Pixels, Points), String> {
        let mut p = Pixels::new(w, h);
        let mut points = BTreeMap::new();
        for l in layers {
            if let Some(tag) = &l.tag {
                points.insert(tag.clone(), l.at);
                if skip == Some(tag.as_str()) {
                    continue;
                }
            }
            // The part, then whatever is worn over it, each placed the same.
            let over = file.over.get(&l.part).map(|v| v.as_slice()).unwrap_or(&[]);
            for (k, name) in std::iter::once(&l.part).chain(over).enumerate() {
                let def = file.parts.get(name).ok_or(format!("{what}: no part `{name}`"))?;
                // Turned: the picture (and its points) about its pivot, which
                // ends up in the middle of the turned picture.
                let turned;
                let (px, (pvx, pvy), part_points) = if l.turn != 0.0 {
                    turned = rotate::rotsprite(&parts[name.as_str()], def.pivot, l.turn);
                    let half = turned.w as i32 / 2;
                    let (s, c) = l.turn.to_radians().sin_cos();
                    let pts: Points = def
                        .points
                        .iter()
                        .map(|(n, &(x, y))| {
                            let (dx, dy) = ((x - def.pivot.0) as f32, (y - def.pivot.1) as f32);
                            (n.clone(), (half + (dx * c + dy * s).round() as i32, half + (-dx * s + dy * c).round() as i32))
                        })
                        .collect();
                    (&turned, (half, half), pts)
                } else {
                    (&parts[name.as_str()], def.pivot, def.points.clone())
                };
                let place = |x: i32, y: i32| {
                    let dx = if l.flip { pvx - x } else { x - pvx };
                    (l.at.0 + dx, l.at.1 + (y - pvy))
                };
                if l.outline
                    && let Some(oc) = file.outline
                {
                    for y in -1..=px.h as i32 {
                        for x in -1..=px.w as i32 {
                            if px.opaque(x, y) {
                                continue;
                            }
                            let near = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| px.opaque(x + dx, y + dy));
                            let (tx, ty) = place(x, y);
                            if near && p.opaque(tx, ty) {
                                p.set(tx, ty, [oc.0, oc.1, oc.2, 255]);
                            }
                        }
                    }
                }
                for y in 0..px.h as i32 {
                    for x in 0..px.w as i32 {
                        let c = px.get(x, y);
                        if c[3] == 0 {
                            continue;
                        }
                        let s = |v: u8| (v as f32 * l.shade).round().clamp(0.0, 255.0) as u8;
                        let (tx, ty) = place(x, y);
                        p.set(tx, ty, [s(c[0]), s(c[1]), s(c[2]), c[3]]);
                    }
                }
                // (The part's points only: what's worn over it has none.)
                for (point, &(x, y)) in part_points.iter().filter(|_| k == 0) {
                    points.insert(point.clone(), place(x, y));
                    // (A tagged layer's points under its tag too: `back_arm.hand`.)
                    if let Some(tag) = &l.tag {
                        points.insert(format!("{tag}.{point}"), place(x, y));
                    }
                }
            }
        }
        Ok((p, points))
    };
    // (pose, tag) → the name of the pose drawn without it.
    let mut without_names: Vec<(String, String, String)> = Vec::new();
    for (name, layers) in &file.poses {
        if drawn.contains_key(name) {
            return Err(format!("pose `{name}`: a frame has that name"));
        }
        let (p, points) = compose(layers, None, &format!("pose `{name}`"))?;
        for (point, at) in &points {
            pose_points.entry(point.clone()).or_default().insert(name.clone(), *at);
        }
        drawn.insert(name.clone(), p);
        // (Drawn without a tagged layer only where a fan stands in for it.)
        let tags: std::collections::BTreeSet<&String> = layers.iter().filter_map(|l| l.tag.as_ref()).filter(|t| file.fans.contains_key(*t)).collect();
        for tag in tags {
            let bare = format!("{name}~{tag}");
            let (p, points) = compose(layers, Some(tag), &format!("pose `{name}`"))?;
            for (point, at) in &points {
                pose_points.entry(point.clone()).or_default().insert(bare.clone(), *at);
            }
            drawn.insert(bare.clone(), p);
            without_names.push((name.clone(), tag.clone(), bare));
        }
    }
    // Fans: each arm alone, its pivot at the frame's middle.
    let fan_pivot = (w as i32 / 2, h as i32 / 2);
    let mut fan_names: Vec<(String, f32, String)> = Vec::new();
    for (tag, arms) in &file.fans {
        for a in arms {
            let name = format!("{tag}@{}", a.angle);
            let turn = a.from.map_or(0.0, |f| a.angle - f);
            let layer = Layer { part: a.part.clone(), at: fan_pivot, flip: false, shade: 1.0, outline: false, turn, tag: None };
            let (p, points) = compose(std::slice::from_ref(&layer), None, &format!("fan `{tag}`"))?;
            for (point, at) in &points {
                pose_points.entry(point.clone()).or_default().insert(name.clone(), *at);
            }
            drawn.insert(name.clone(), p);
            fan_names.push((tag.clone(), a.angle, name));
        }
    }
    // Derived frames, in whatever order their bases allow.
    let mut left: Vec<&String> = file.derived.keys().collect();
    while !left.is_empty() {
        let before = left.len();
        left.retain(|name| {
            let d = &file.derived[*name];
            let Some(base) = drawn.get(&d.from) else { return true };
            let mut p = Pixels::new(w, h);
            for y in 0..h as i32 {
                for x in 0..w as i32 {
                    let sx = if d.flip { w as i32 - 1 - x } else { x };
                    p.set(x + d.shift.0, y + d.shift.1, base.get(sx, y));
                }
            }
            if !d.over.is_empty()
                && let Err(e) = draw(&mut p, &d.over, &file.palette, true, &format!("derived `{name}`")) {
                    drawn.insert(format!("!{e}"), Pixels::new(1, 1));
                }
            drawn.insert((*name).clone(), p);
            false
        });
        if left.len() == before {
            let names: Vec<&str> = left.iter().map(|s| s.as_str()).collect();
            return Err(format!("derived {names:?}: made from a frame that doesn't exist (or from each other)"));
        }
    }
    if let Some(e) = drawn.keys().find(|k| k.starts_with('!')) {
        return Err(e[1..].to_string());
    }
    if file.frames.keys().any(|k| file.derived.contains_key(k)) {
        return Err("a frame and a derived frame share a name".into());
    }
    let names: Vec<String> = drawn.keys().cloned().collect();
    let frames: Vec<Pixels> = names.iter().map(|n| match file.outline {
        Some(c) => outline(&drawn[n], c),
        None => drawn[n].clone(),
    }).collect();
    let index = |n: &str| names.iter().position(|m| m == n);
    let mut clips = BTreeMap::new();
    for (name, c) in &file.clips {
        let frames = c.frames.iter().map(|f| index(f).ok_or(format!("clip `{name}`: no frame `{f}`"))).collect::<Result<Vec<_>, _>>()?;
        if frames.is_empty() {
            return Err(format!("clip `{name}`: no frames"));
        }
        clips.insert(name.clone(), CompiledClip { frames, fps: c.fps, looping: c.looping });
    }
    let mut anchors: BTreeMap<String, HashMap<usize, (i32, i32)>> = BTreeMap::new();
    for (point, by_pose) in &pose_points {
        let m = anchors.entry(point.clone()).or_default();
        for (pose, &at) in by_pose {
            m.insert(index(pose).expect("drawn"), at);
        }
    }
    for (name, by_frame) in &file.anchors {
        let m = anchors.entry(name.clone()).or_default();
        for (f, &at) in by_frame {
            m.insert(index(f).ok_or(format!("anchor `{name}`: no frame `{f}`"))?, at);
        }
    }
    let mut without = HashMap::new();
    for (pose, tag, bare) in without_names {
        without.insert((index(&pose).expect("drawn"), tag), index(&bare).expect("drawn"));
    }
    let mut fans: BTreeMap<String, Vec<(f32, usize)>> = BTreeMap::new();
    for (tag, angle, name) in fan_names {
        fans.entry(tag).or_default().push((angle, index(&name).expect("drawn")));
    }
    Ok(Art { size: file.size, feet: file.feet, names, frames, clips, anchors, without, fans, fan_pivot })
}

/// Things worth a look that aren't errors: lone pixels, colours never used,
/// frames no clip plays, the feet off the sprite, odd frame rates.
pub fn check(file: &ArtFile, art: &Art) -> Vec<String> {
    let mut warn = Vec::new();
    let (w, h) = art.size;
    if art.feet.0 < 0.0 || art.feet.1 < 0.0 || art.feet.0 > w as f32 || art.feet.1 > h as f32 {
        warn.push(format!("feet {:?} are off the {w}×{h} frame", art.feet));
    }
    for (name, p) in art.names.iter().zip(&art.frames) {
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let alone = p.opaque(x, y) && (-1..=1).all(|dy| (-1..=1).all(|dx| (dx, dy) == (0, 0) || !p.opaque(x + dx, y + dy)));
                if alone {
                    warn.push(format!("frame `{name}`: a lone pixel at ({x}, {y})"));
                }
            }
        }
        if (0..h as i32).all(|y| (0..w as i32).all(|x| !p.opaque(x, y))) {
            warn.push(format!("frame `{name}` is empty"));
        }
    }
    let used: std::collections::HashSet<char> = file.frames.values().chain(file.derived.values().map(|d| &d.over)).chain(file.parts.values().map(|p| &p.rows)).flatten().flat_map(|r| r.chars()).collect();
    for c in file.palette.keys() {
        if !used.contains(c) {
            warn.push(format!("palette '{c}' is never used"));
        }
    }
    let played: std::collections::HashSet<usize> = art.clips.values().flat_map(|c| c.frames.iter().copied()).collect();
    let spare: std::collections::HashSet<usize> = art.without.values().copied().chain(art.fans.values().flatten().map(|f| f.1)).collect();
    for (i, name) in art.names.iter().enumerate() {
        if !played.contains(&i) && !spare.contains(&i) {
            warn.push(format!("frame `{name}` isn't in any clip"));
        }
    }
    for (name, c) in &art.clips {
        if !(0.5..=60.0).contains(&c.fps) {
            warn.push(format!("clip `{name}` runs at {} fps", c.fps));
        }
    }
    warn
}

/// A sprite in words, for whoever can't look at it: its frames (what's drawn
/// where, how much of each colour), clips and anchors.
pub fn describe(file: &ArtFile, art: &Art) -> String {
    let (w, h) = art.size;
    let mut out = format!("{w}×{h} px, feet at {:?}, {} frames, {} clips\n", art.feet, art.frames.len(), art.clips.len());
    let colour_of: HashMap<[u8; 4], char> = file.palette.iter().map(|(c, col)| (col.rgba(), *c)).collect();
    for (name, p) in art.names.iter().zip(&art.frames) {
        let mut bounds = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        let mut counts: BTreeMap<char, u32> = BTreeMap::new();
        let mut mirror = 0;
        let mut opaque = 0;
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let px = p.get(x, y);
                if px[3] == 0 {
                    continue;
                }
                opaque += 1;
                bounds = (bounds.0.min(x), bounds.1.min(y), bounds.2.max(x), bounds.3.max(y));
                *counts.entry(colour_of.get(&px).copied().unwrap_or('#')).or_default() += 1;
                if p.opaque(w as i32 - 1 - x, y) {
                    mirror += 1;
                }
            }
        }
        if opaque == 0 {
            out += &format!("  frame `{name}`: empty\n");
            continue;
        }
        let colours: Vec<String> = counts.iter().map(|(c, n)| format!("'{c}'×{n}")).collect();
        out += &format!(
            "  frame `{name}`: drawn in x {}..{}, y {}..{} ({} px, {}% mirror-symmetric); {}\n",
            bounds.0,
            bounds.2,
            bounds.1,
            bounds.3,
            opaque,
            mirror * 100 / opaque,
            colours.join(" ")
        );
    }
    for (name, c) in &art.clips {
        let frames: Vec<&str> = c.frames.iter().map(|&i| art.names[i].as_str()).collect();
        out += &format!("  clip `{name}`: {} at {} fps{}\n", frames.join(" → "), c.fps, if c.looping { ", looping" } else { "" });
    }
    for (name, m) in &art.anchors {
        out += &format!("  anchor `{name}` in {} frames\n", m.len());
    }
    out
}

/// A contact sheet: one row per clip (its frames in order), then every frame
/// (a row of all of them), each scaled `scale` times on a checkerboard, the
/// feet marked red and anchors blue. Returns the picture and a legend.
pub fn sheet(art: &Art, scale: u32, marks: bool) -> (Pixels, String) {
    let (w, h) = art.size;
    let gap = 2;
    let all: Vec<usize> = (0..art.frames.len()).collect();
    let mut rows: Vec<(&str, &[usize])> = art.clips.iter().map(|(n, c)| (n.as_str(), c.frames.as_slice())).collect();
    rows.push(("(every frame)", &all));
    let cols = rows.iter().map(|r| r.1.len()).max().unwrap_or(1) as u32;
    let (cw, ch) = (w * scale + gap, h * scale + gap);
    let mut out = Pixels::new(cols * cw + gap, rows.len() as u32 * ch + gap);
    for y in 0..out.h as i32 {
        for x in 0..out.w as i32 {
            out.set(x, y, [40, 44, 52, 255]);
        }
    }
    let mut legend = String::new();
    for (r, (name, frames)) in rows.iter().enumerate() {
        let names: Vec<&str> = frames.iter().map(|&i| art.names[i].as_str()).collect();
        legend += &format!("row {r}: {name}: {}\n", names.join(", "));
        for (c, &f) in frames.iter().enumerate() {
            let (ox, oy) = (gap + c as u32 * cw, gap + r as u32 * ch);
            for y in 0..h * scale {
                for x in 0..w * scale {
                    let px = art.frames[f].get((x / scale) as i32, (y / scale) as i32);
                    let check = if ((x / (scale * 2).max(2)) + (y / (scale * 2).max(2))).is_multiple_of(2) { 70 } else { 90 };
                    let a = px[3] as u32;
                    let mix = |v: u8| ((v as u32 * a + check * (255 - a)) / 255) as u8;
                    out.set((ox + x) as i32, (oy + y) as i32, [mix(px[0]), mix(px[1]), mix(px[2]), 255]);
                }
            }
            if marks {
                let dot = |out: &mut Pixels, px: f32, py: f32, col: [u8; 4]| {
                    let (cx, cy) = (ox as f32 + px * scale as f32, oy as f32 + py * scale as f32);
                    for d in -(scale as i32)..=(scale as i32) {
                        out.set(cx as i32 + d, cy as i32, col);
                        out.set(cx as i32, cy as i32 + d, col);
                    }
                };
                dot(&mut out, art.feet.0, art.feet.1, [255, 60, 60, 255]);
                for m in art.anchors.values() {
                    if let Some(&(ax, ay)) = m.get(&f) {
                        dot(&mut out, ax as f32 + 0.5, ay as f32 + 0.5, [80, 150, 255, 255]);
                    }
                }
            }
        }
    }
    (out, legend)
}

/// Write pixels as a PNG.
pub fn write_png(p: &Pixels, path: &std::path::Path) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), p.w, p.h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&p.rgba).map_err(|e| e.to_string())
}

/// Read a PNG as pixels (for `import`).
pub fn read_png(path: &std::path::Path) -> Result<Pixels, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut dec = png::Decoder::new(std::io::BufReader::new(file));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16 | png::Transformations::ALPHA);
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("png: too big")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let (w, h) = (info.width, info.height);
    let channels = info.color_type.samples();
    let mut p = Pixels::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) as usize) * channels;
            let px = match channels {
                4 => [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]],
                2 => [buf[i], buf[i], buf[i], buf[i + 1]],
                3 => [buf[i], buf[i + 1], buf[i + 2], 255],
                _ => [buf[i], buf[i], buf[i], 255],
            };
            p.set(x as i32, y as i32, px);
        }
    }
    Ok(p)
}

/// A picture as a sprite file's frame and palette (for `import`): each
/// distinct opaque colour gets a character.
pub fn to_grid(p: &Pixels, x0: u32, y0: u32, w: u32, h: u32, palette: &mut Vec<([u8; 4], char)>) -> Vec<String> {
    const CHARS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789#$%&*+=?@";
    (0..h)
        .map(|y| {
            (0..w)
                .map(|x| {
                    let px = p.get((x0 + x) as i32, (y0 + y) as i32);
                    if px[3] == 0 {
                        return CLEAR;
                    }
                    if let Some(&(_, c)) = palette.iter().find(|(col, _)| *col == px) {
                        return c;
                    }
                    let c = CHARS.chars().nth(palette.len()).unwrap_or('?');
                    palette.push((px, c));
                    c
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"(
        size: (4, 3),
        feet: (2, 3),
        outline: (1, 2, 3),
        palette: { 'a': (200, 10, 10), 'b': (10, 200, 10, 128) },
        frames: {
            "one": ["....", ".ab.", "...."],
        },
        derived: {
            "two": (from: "one", shift: (1, 0), over: ["_...", "....", "a..."]),
            "three": (from: "two", flip: true),
        },
        clips: { "go": (frames: ["one", "two", "three"], fps: 6) },
        anchors: { "hand": { "two": (3, 1) } },
    )"#;

    #[test]
    fn frames_derive_outline_and_clip() {
        let file = parse(SAMPLE).unwrap();
        let art = compile(&file).unwrap();
        assert_eq!(art.names, vec!["one", "three", "two"]);
        let one = &art.frames[art.index("one").unwrap()];
        assert_eq!(one.get(1, 1), [200, 10, 10, 255]);
        assert_eq!(one.get(2, 1), [10, 200, 10, 128]);
        assert_eq!(one.get(1, 0), [1, 2, 3, 255], "outlined above");
        assert_eq!(one.get(0, 0), [0; 4], "not on the diagonal");
        let two = &art.frames[art.index("two").unwrap()];
        assert_eq!(two.get(2, 1), [200, 10, 10, 255], "shifted right");
        assert_eq!(two.get(0, 2), [200, 10, 10, 255], "drawn over");
        let three = &art.frames[art.index("three").unwrap()];
        assert_eq!(three.get(1, 1), [200, 10, 10, 255], "two, mirrored");
        assert_eq!(art.clips["go"].frames.len(), 3);
        assert_eq!(art.anchors["hand"][&art.index("two").unwrap()], (3, 1));
        let (atlas, cols, rows) = art.atlas();
        assert_eq!((atlas.w, atlas.h, cols, rows), (12, 3, 3, 1));
    }

    #[test]
    fn mistakes_are_named() {
        let bad = |s: &str| compile(&parse(s).unwrap()).unwrap_err();
        assert!(bad(r#"(size: (2, 1), feet: (1, 1), palette: {}, frames: { "x": ["z."] })"#).contains("'z' isn't in the palette"));
        assert!(bad(r#"(size: (2, 1), feet: (1, 1), palette: {}, frames: { "x": [".."] }, clips: { "c": (frames: ["y"], fps: 4) })"#).contains("no frame `y`"));
        assert!(bad(r#"(size: (2, 2), feet: (1, 1), palette: {}, frames: { "x": [".."] })"#).contains("1 rows"));
        assert!(bad(r#"(size: (2, 1), feet: (1, 1), palette: {}, derived: { "a": (from: "b"), "b": (from: "a") })"#).contains("doesn't exist"));
    }

    /// A turned layer: the part swung about its pivot, its points too.
    #[test]
    fn turned_layers_swing_parts_and_their_points() {
        let file = parse(r#"(
            size: (12, 12), feet: (6, 12),
            palette: { 'a': (200, 200, 200) },
            parts: { "arm": (pivot: (0, 0), points: { "hand": (4, 0) }, rows: ["aaaaa"]) },
            poses: {
                "out": [(part: "arm", at: (5, 6))],
                "up": [(part: "arm", at: (5, 6), turn: 90)],
            },
            fans: { "arm": [(angle: -90, part: "arm", from: 0)] },
            clips: { "idle": (frames: ["out", "up"], fps: 1) },
        )"#).unwrap();
        let art = compile(&file).unwrap();
        let (out, up) = (&art.frames[art.index("out").unwrap()], &art.frames[art.index("up").unwrap()]);
        assert!(out.opaque(9, 6) && !out.opaque(5, 2));
        assert!(up.opaque(5, 2) && !up.opaque(9, 6), "turned 90: straight up");
        assert_eq!(art.anchors["hand"][&art.index("up").unwrap()], (5, 2));
        let down = art.index("arm@-90").unwrap();
        assert!(art.frames[down].opaque(6, 10), "the fan arm made by turning: straight down");
    }

    #[test]
    fn poses_put_parts_together_and_carry_their_points() {
        let file = parse(r#"(
            size: (8, 6), feet: (4, 6),
            palette: { 'a': (10, 20, 30), 'b': (100, 100, 100) },
            parts: {
                "body": (pivot: (0, 0), rows: ["aa", "aa"]),
                "arm": (pivot: (0, 0), points: { "hand": (2, 1) }, rows: ["bbb", "..b"]),
            },
            poses: {
                "p": [(part: "arm", at: (4, 1), shade: 0.5), (part: "body", at: (3, 2))],
                "q": [(part: "arm", at: (4, 1), flip: true)],
            },
            clips: { "c": (frames: ["p", "q"], fps: 4) },
        )"#).unwrap();
        let art = compile(&file).unwrap();
        let p = &art.frames[art.index("p").unwrap()];
        assert_eq!(p.get(4, 1), [50, 50, 50, 255], "arm, shaded");
        assert_eq!(p.get(3, 2), [10, 20, 30, 255], "body over it");
        assert_eq!(art.anchors["hand"][&art.index("p").unwrap()], (6, 2));
        assert_eq!(art.anchors["hand"][&art.index("q").unwrap()], (2, 2), "mirrored about the pivot");
    }

    /// Every sprite in the game's assets compiles, with no warnings.
    #[test]
    fn every_asset_compiles_cleanly() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/art");
        let mut n = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "ron") {
                continue;
            }
            let file = parse(&std::fs::read_to_string(&path).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let art = compile(&file).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(check(&file, &art), Vec::<String>::new(), "{}", path.display());
            n += 1;
        }
        assert!(n >= 3, "the critters are there");
    }

    #[test]
    fn checks_and_descriptions_say_what_they_see() {
        let file = parse(r#"(size: (5, 3), feet: (2, 3), palette: { 'a': (1, 1, 1), 'z': (2, 2, 2) }, frames: { "x": ["a....", ".....", "..aa."] }, clips: {})"#).unwrap();
        let art = compile(&file).unwrap();
        let warn = check(&file, &art).join("\n");
        assert!(warn.contains("lone pixel at (0, 0)"), "{warn}");
        assert!(warn.contains("'z' is never used"), "{warn}");
        assert!(warn.contains("isn't in any clip"), "{warn}");
        let text = describe(&file, &art);
        assert!(text.contains("'a'×3"), "{text}");
    }
}
