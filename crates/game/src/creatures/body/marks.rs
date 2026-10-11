//! Wound marks (`wounds` stage 3): a blow leaves its mark where it landed,
//! of its kind, in what the body's made of. On a humanoid drawn in layers
//! (`ragdoll::cut`'s parts: its head, torso, arms), the mark goes on the
//! part the blow's pixel was in, at that spot of the part: drawn into its
//! look as what it wears is (`gear::look::dress`: an overlay clipped to the
//! part), so it moves as the part does, and stays on its body when it dies.
//!
//! What a mark looks like: its blow (a slash's cut along the blow, a point's
//! hole, a blunt bruise or crack, fire's char, acid's melt, frost's rime)
//! in its tissue (flesh bleeds its own blood's colour, bone cracks dark and
//! dry, metal scratches bright and dents, a shell cracks). At most `MOST`
//! a body, the oldest going; redrawn at most every `REDRAW`.

use bevy::prelude::*;
use platypus_art::dress::{Overlay, Skin};

use super::parts::Limb;
use super::ragdoll::Cut;
use crate::creatures::nature::Harm;
use crate::creatures::Kinematics;
use crate::world::SimWorld;

/// Most marks a body keeps.
const MOST: usize = 10;
/// A body's look redrawn at most this often (seconds).
const REDRAW: f32 = 0.25;
/// A blow under this (of its health) leaves no mark.
const SMALLEST: f32 = 0.04;

/// What a body's made of, under its skin (by its kind).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tissue {
    Flesh,
    Bone,
    Metal,
    Shell,
}

impl Tissue {
    pub fn of(kind: Option<&str>) -> Tissue {
        match kind {
            Some("undead") => Tissue::Bone,
            Some("construct") => Tissue::Metal,
            Some("insect") => Tissue::Shell,
            _ => Tissue::Flesh,
        }
    }
}

/// What a blow leaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkKind {
    Cut,
    Hole,
    Bruise,
    Crack,
    Char,
    Melt,
    Rime,
}

impl MarkKind {
    pub fn of(harm: Harm, tissue: Tissue) -> Option<MarkKind> {
        Some(match harm {
            Harm::Slash => MarkKind::Cut,
            Harm::Pierce => MarkKind::Hole,
            Harm::Blunt if tissue == Tissue::Flesh => MarkKind::Bruise,
            Harm::Blunt => MarkKind::Crack,
            Harm::Fire | Harm::Storm | Harm::Radiant | Harm::Void => MarkKind::Char,
            Harm::Acid => MarkKind::Melt,
            Harm::Frost => MarkKind::Rime,
            _ => return None,
        })
    }
}

/// A mark on a part: where on it (its pixels, from its top-left), which way
/// the blow went (the part's way: facing right).
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub kind: MarkKind,
    pub part: String,
    pub at: IVec2,
    pub dir: Vec2,
}

/// A body's marks.
#[derive(Component, Clone, Debug, Default)]
pub struct Marks {
    pub list: Vec<Mark>,
    /// Changed since its look was last drawn; and how long till it may be.
    pub fresh: bool,
    wait: f32,
}

impl Marks {
    fn add(&mut self, m: Mark) {
        if self.list.len() >= MOST {
            self.list.remove(0);
        }
        self.list.push(m);
        self.fresh = true;
    }
}

/// Blows on a body drawn in layers leave their mark on the part they hit.
#[allow(clippy::type_complexity)]
pub fn mark(
    mut commands: Commands,
    mut felt: MessageReader<crate::combat::Felt>,
    mut masks: ResMut<super::wounds::LayerMasks>,
    mut q: Query<(&Kinematics, &super::animation::Animator, &crate::creatures::Health, Option<&mut Marks>, Option<&super::parts::Severed>)>,
) {
    for f in felt.read() {
        if f.dealt <= 0.0 {
            continue;
        }
        let Ok((k, anim, health, marks, severed)) = q.get_mut(f.target) else { continue };
        if f.dealt / health.max.max(1.0) < SMALLEST {
            continue;
        }
        let Some(kind) = MarkKind::of(f.harm, Tissue::of(anim.def.kind.as_deref())) else { continue };
        let Some((part, origin, turned, limb, p)) = layer_at(anim, k, f.at, &mut masks) else { continue };
        if turned || limb.is_some_and(|l| severed.is_some_and(|s| s.has(l))) {
            continue;
        }
        let facing = k.loco.facing;
        let m = Mark { kind, part, at: p - origin, dir: Vec2::new(f.dir.x * facing, -f.dir.y).normalize_or(Vec2::X) };
        match marks {
            Some(mut ms) => ms.add(m),
            None => {
                let mut ms = Marks::default();
                ms.add(m);
                commands.entity(f.target).insert(ms);
            }
        }
    }
}

/// The layer of the frame it's showing a world point is on (frontmost): its
/// part, where the part's top-left is, whether it's drawn turned, the limb
/// it is; and that frame's pixel.
fn layer_at(anim: &super::animation::Animator, k: &Kinematics, at: Vec2, masks: &mut super::wounds::LayerMasks) -> Option<(String, IVec2, bool, Option<Limb>, IVec2)> {
    let file = anim.def.art_file.as_ref().filter(|f| !f.poses.is_empty())?;
    let frame = anim.def.rig.as_ref()?.names.get(anim.shown)?.clone();
    let cuts = masks.cuts(file, &frame);
    let (x, y) = super::animation::pixel_at(&anim.def, k, at);
    let p = IVec2::new(x, y);
    let near = |c: &&Cut| (-1..=1).any(|dy| (-1..=1).any(|dx| c.covers(p + IVec2::new(dx, dy))));
    let c = cuts.iter().rev().find(|c| c.covers(p)).or_else(|| cuts.iter().rev().find(near))?;
    Some((c.name.clone(), c.origin, c.turned, c.limb(), p))
}

/// A body's look is redrawn with its marks at most every `REDRAW`.
pub fn settle(time: Res<Time>, mut q: Query<&mut Marks>) {
    let dt = time.delta_secs();
    for mut m in &mut q {
        m.bypass_change_detection().wait -= dt;
        if m.fresh && m.wait <= 0.0 {
            m.fresh = false;
            m.wait = REDRAW;
            m.set_changed();
        }
    }
}

/// A body's marks as something it wears: each a small overlay on its part,
/// clipped to it, in its tissue's colours (`blood` its blood's).
pub fn skin(marks: &[Mark], tissue: Tissue, blood: (u8, u8, u8)) -> Skin {
    let dark = |(r, g, b): (u8, u8, u8), k: f32| ((r as f32 * k) as u8, (g as f32 * k) as u8, (b as f32 * k) as u8);
    let (wound, rim) = match tissue {
        Tissue::Flesh => (dark(blood, 0.45), blood),
        Tissue::Bone => ((52, 44, 40), (98, 90, 80)),
        Tissue::Metal => ((60, 62, 70), (220, 222, 230)),
        Tissue::Shell => (dark(blood, 0.5), blood),
    };
    let mut palette = std::collections::BTreeMap::new();
    let mut put = |c: char, (r, g, b): (u8, u8, u8)| {
        palette.insert(c, platypus_art::Color::Rgb(r, g, b));
    };
    put('w', wound);
    put('r', rim);
    put('u', (92, 58, 84));
    put('c', (30, 22, 20));
    put('h', (82, 54, 34));
    put('g', (150, 220, 70));
    put('i', (196, 232, 255));
    let over = marks.iter().map(|m| {
        let (rows, centre) = pattern(m.kind, m.dir);
        Overlay { parts: vec![m.part.clone()], at: (m.at.x - centre.x, m.at.y - centre.y), rows, clip: true }
    });
    Skin { palette, recolor: Vec::new(), over: over.collect() }
}

/// A mark's shape, as rows, and its middle.
fn pattern(kind: MarkKind, dir: Vec2) -> (Vec<String>, IVec2) {
    let grid = |cells: &[(i32, i32, char)]| {
        let n = 7;
        let mut rows = vec![vec!['.'; n]; n];
        for &(x, y, c) in cells {
            let (x, y) = (x + 3, y + 3);
            if (0..n as i32).contains(&x) && (0..n as i32).contains(&y) {
                rows[y as usize][x as usize] = c;
            }
        }
        (rows.into_iter().map(|r| r.into_iter().collect()).collect(), IVec2::new(3, 3))
    };
    // (Across the blow, a cut; rows y down, so the blow's y turns over.)
    let d = Vec2::new(dir.x, -dir.y).normalize_or(Vec2::X);
    let along = |t: f32| ((d.x * t).round() as i32, (d.y * t).round() as i32);
    match kind {
        MarkKind::Cut => {
            let side = IVec2::new((-d.y).round() as i32, d.x.round() as i32);
            let mut c = Vec::new();
            for t in [-2.0, -1.0, 0.0, 1.0, 2.0] {
                let (x, y) = along(t);
                c.push((x, y, 'w'));
                if t.abs() < 2.0 {
                    c.push((x + side.x, y + side.y, 'r'));
                }
            }
            grid(&c)
        }
        MarkKind::Hole => grid(&[(0, 0, 'w'), (1, 0, 'w'), (0, 1, 'w'), (1, 1, 'r'), (-1, 0, 'r'), (0, -1, 'r')]),
        MarkKind::Bruise => grid(&[(-1, -1, 'u'), (0, -1, 'u'), (1, 0, 'u'), (-1, 0, 'u'), (0, 0, 'u'), (0, 1, 'u'), (1, 1, 'u')]),
        MarkKind::Crack => {
            let (a, b) = (along(1.0), along(2.0));
            grid(&[(0, 0, 'w'), (a.0, a.1, 'w'), (b.0 + 1, b.1, 'w'), (-a.0, -a.1 + 1, 'w'), (-b.0, -b.1 + 1, 'w'), (1, -1, 'r')])
        }
        MarkKind::Char => grid(&[(-1, -1, 'h'), (0, -1, 'c'), (1, -1, 'h'), (-1, 0, 'c'), (0, 0, 'c'), (1, 0, 'c'), (2, 0, 'h'), (0, 1, 'c'), (1, 1, 'h'), (-2, 1, 'h')]),
        MarkKind::Melt => grid(&[(0, 0, 'g'), (1, 0, 'w'), (0, 1, 'g'), (0, 2, 'g'), (1, 1, 'w'), (-1, 0, 'w')]),
        MarkKind::Rime => grid(&[(0, 0, 'i'), (-1, 1, 'i'), (2, -1, 'i'), (1, 1, 'i'), (-2, -1, 'i')]),
    }
}

/// The colour its blood shows (its blood's middle shade), or a dark red.
pub fn blood_color(sim: &SimWorld, bleeds: Option<&super::hurt::Bleeds>) -> (u8, u8, u8) {
    bleeds.map_or((140, 24, 26), |b| {
        let def = sim.world.materials().def(b.0);
        def.colors[def.colors.len() / 2]
    })
}
