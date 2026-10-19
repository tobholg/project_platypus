//! Blocks that look built (materials.ron `tile`): a block of a building
//! material draws itself by what's round it, as a Terraria tile does. Where
//! nothing solid is beside it, an outline, lit inside its top and shaded
//! inside its bottom; where two of its sides are open, the corner rounded
//! off on an arc (the cells there left out); where it meets another
//! material, a seam; where it meets its own, nothing: the art runs on. The
//! art inside (bricks, cobble, boards, a beam's grain, roof tiles, plaster
//! in its timber frame) is laid by world position, each stone or board its
//! own tone, so a wall looks laid by hand.
//!
//! Pure: placing blocks (`World::stamp_block`) and worldgen's structures
//! both draw with it. A cell's shade picks its colour off the material's
//! `colors`, darkest first; a tile material's are written outline, gap,
//! shadow, face, light, highlight (or so).

use serde::Deserialize;

use crate::edit::BLOCK;
use crate::rng::hash;

/// How a building material draws itself (materials.ron `tile`).
#[derive(Clone, Debug, Deserialize)]
pub struct Tile {
    pub art: Art,
    /// Its open corners rounded off on an arc this many cells across (0:
    /// square: a wall behind).
    #[serde(default = "default_round")]
    pub round: f32,
    /// Its open edges outlined (a wall behind is only darkened there).
    #[serde(default = "yes")]
    pub outline: bool,
    /// Cracked (a weak wall looks it, a little).
    #[serde(default)]
    pub cracked: bool,
}

fn default_round() -> f32 {
    3.0
}

fn yes() -> bool {
    true
}

/// What's drawn inside a block.
#[derive(Clone, Debug, Deserialize)]
pub enum Art {
    /// Courses `course` cells tall of bricks `length` (from, to) long, each
    /// its own tone, lit on top, mortar under.
    Bricks { course: i32, length: (i32, i32) },
    /// Round stones about `size` across, set in dark gaps, each lit toward
    /// its top-left.
    Cobble { size: i32 },
    /// Boards laid along, `board` cells tall, `length` long, ends staggered,
    /// a seam under each, nail heads at their ends, knots.
    Boards { board: i32, length: i32 },
    /// Boards stood up, `width` cells wide (a plank wall).
    Slats { width: i32 },
    /// A beam: grain running up it.
    Grain,
    /// Roof tiles in staggered rows `row` cells tall, `width` wide, each lit
    /// at its top and dark in the row above's shadow.
    Shingles { row: i32, width: i32 },
    /// Plaster in a timber frame: beams round its edges (where it meets
    /// anything else), a post every `panel` blocks, braces across.
    Framed { panel: i32 },
    /// Mottled, nothing laid (plain stone, a wall of earth).
    Plain,
}

/// What's round a block: which of its sides (and corners, for the notch in
/// an inside corner) have nothing solid beside them, and which sides meet
/// another solid material.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Around {
    pub n: bool,
    pub s: bool,
    pub w: bool,
    pub e: bool,
    pub nw: bool,
    pub ne: bool,
    pub sw: bool,
    pub se: bool,
    /// North, south, west, east: another solid material there.
    pub other: [bool; 4],
}

impl Around {
    /// Shut in on every side (the art alone: a block in the middle of a wall).
    pub const SHUT: Around = Around { n: false, s: false, w: false, e: false, nw: false, ne: false, sw: false, se: false, other: [false; 4] };
}

/// Tones along a tile material's `colors` (0 darkest).
const OUTLINE: f32 = 0.0;
const GAP: f32 = 0.14;
const LIFT: f32 = 0.24;
const SINK: f32 = 0.22;

fn roll(parts: &[i64]) -> f32 {
    let v: Vec<u64> = parts.iter().map(|&p| p as u64).collect();
    (hash(&v) >> 11) as f32 / (1u64 << 53) as f32
}

/// A cell's shade (0..255) as drawn, or `None`: cut off (a rounded corner).
/// `x`, `y` are world cells (y up); `block` the block's own.
pub fn shade(tile: &Tile, x: i32, y: i32, a: Around) -> Option<u8> {
    let b = BLOCK;
    let (cx, cy) = (x.rem_euclid(b), y.rem_euclid(b));
    // (From the top, as art is read.)
    let top = b - 1 - cy;
    let (fx, fy) = (cx as f32 + 0.5, top as f32 + 0.5);
    let (n, s, w, e) = (a.n, a.s, a.w, a.e);
    let mut edge = false;
    // Rounded open corners: outside the arc, nothing; on it, the edge.
    let r = tile.round;
    if r > 0.0 {
        let bf = b as f32;
        for (open, kx, ky) in [(n && w, r, r), (n && e, bf - r, r), (s && w, r, bf - r), (s && e, bf - r, bf - r)] {
            if !open {
                continue;
            }
            let inside_x = if kx == r { fx < kx } else { fx > kx };
            let inside_y = if ky == r { fy < ky } else { fy > ky };
            if inside_x && inside_y {
                let d = ((fx - kx).powi(2) + (fy - ky).powi(2)).sqrt();
                if d > r {
                    return None;
                }
                if d > r - 1.0 {
                    edge = true;
                }
            }
        }
    }
    if (n && top == 0) || (s && top == b - 1) || (w && cx == 0) || (e && cx == b - 1) {
        edge = true;
    }
    // An inside corner (two solid sides round an open diagonal): a dark
    // notch where they meet.
    let notch = (!n && !w && a.nw && top == 0 && cx == 0) || (!n && !e && a.ne && top == 0 && cx == b - 1) || (!s && !w && a.sw && top == b - 1 && cx == 0) || (!s && !e && a.se && top == b - 1 && cx == b - 1);
    let mut t = if edge || notch {
        if tile.outline { OUTLINE } else { art(tile, x, y, a) - 0.3 }
    } else {
        let mut t = art(tile, x, y, a);
        // A bevel inside the outline: lit on top and a little on the left,
        // shaded under and on the right.
        if n && top == 1 {
            t += LIFT;
        }
        if w && cx == 1 {
            t += LIFT * 0.4;
        }
        if s && top == b - 2 {
            t -= SINK;
        }
        if e && cx == b - 2 {
            t -= SINK * 0.6;
        }
        // A seam where it meets another material.
        let [on, os, ow, oe] = a.other;
        if (on && top == 0) || (os && top == b - 1) || (ow && cx == 0) || (oe && cx == b - 1) {
            t = t.min(GAP + 0.08);
        }
        if tile.cracked && crack(x, y) {
            t = GAP;
        }
        t
    };
    t = t.clamp(0.0, 1.0);
    Some(((t * 15.0).round() as u8) * 16 + 8)
}

/// A crack through weak masonry: short dark zig-zags, here and there.
fn crack(x: i32, y: i32) -> bool {
    let b = BLOCK;
    let (bx, by) = (x.div_euclid(b), y.div_euclid(b));
    if roll(&[bx as i64, by as i64, 71]) > 0.55 {
        return false;
    }
    let (cx, cy) = (x.rem_euclid(b), y.rem_euclid(b));
    let start = (roll(&[bx as i64, by as i64, 72]) * (b - 2) as f32) as i32 + 1;
    let wiggle = if roll(&[bx as i64, by as i64, cy as i64, 73]) > 0.5 { 1 } else { 0 };
    cy >= 1 && cy < b - 1 && cx == (start + cy / 2 + wiggle).min(b - 2)
}

/// The art's tone at a cell (before edges): 0..1 along `colors`.
fn art(tile: &Tile, x: i32, y: i32, a: Around) -> f32 {
    let jitter = (roll(&[x as i64, y as i64, 9]) - 0.5) * 0.08;
    match tile.art {
        Art::Bricks { course, length: (lo, hi) } => {
            let row = y.div_euclid(course);
            let from_top = course - 1 - y.rem_euclid(course);
            let len = lo + (roll(&[row as i64, 31]) * (hi - lo + 1) as f32) as i32;
            let lx = x + (roll(&[row as i64, 32]) * len as f32) as i32;
            let brick = lx.div_euclid(len);
            let along = lx.rem_euclid(len);
            if from_top == course - 1 || along == 0 {
                return GAP;
            }
            let mut t = 0.4 + roll(&[brick as i64, row as i64, 33]) * 0.3;
            if from_top == 0 {
                t += 0.16;
            }
            if from_top == course - 2 {
                t -= 0.1;
            }
            if along == 1 {
                t += 0.06;
            }
            if along == len - 1 {
                t -= 0.06;
            }
            t + jitter
        }
        Art::Cobble { size } => {
            let (gx, gy) = (x.div_euclid(size), y.div_euclid(size));
            let mut best = [(f32::MAX, 0i32, 0i32, 0.0f32, 0.0f32); 2];
            for j in -1..=1 {
                for i in -1..=1 {
                    let (sx, sy) = (gx + i, gy + j);
                    let px = (sx * size) as f32 + 1.0 + roll(&[sx as i64, sy as i64, 41]) * (size - 2) as f32;
                    let py = (sy * size) as f32 + 1.0 + roll(&[sx as i64, sy as i64, 42]) * (size - 2) as f32;
                    let d = ((x as f32 + 0.5 - px).powi(2) + ((y as f32 + 0.5 - py) * 1.15).powi(2)).sqrt();
                    if d < best[0].0 {
                        best[1] = best[0];
                        best[0] = (d, sx, sy, px, py);
                    } else if d < best[1].0 {
                        best[1] = (d, sx, sy, px, py);
                    }
                }
            }
            let (d0, sx, sy, px, py) = best[0];
            if best[1].0 - d0 < 1.05 {
                return GAP;
            }
            // Lit toward its top-left (y up: above it, to its left).
            0.38 + roll(&[sx as i64, sy as i64, 43]) * 0.32 + ((px - x as f32) + (y as f32 - py)) * 0.04 + jitter
        }
        Art::Boards { board, length } => {
            let row = y.div_euclid(board);
            let from_top = board - 1 - y.rem_euclid(board);
            let lx = x + (roll(&[row as i64, 51]) * length as f32) as i32;
            let along = lx.rem_euclid(length);
            let plank = lx.div_euclid(length);
            if from_top == board - 1 {
                return GAP;
            }
            if along == 0 {
                return GAP + 0.08;
            }
            let mut t = 0.45 + roll(&[plank as i64, row as i64, 52]) * 0.25;
            if from_top == 0 {
                t += 0.14;
            }
            if (along == 1 || along == length - 1) && from_top == 1 {
                t -= 0.2; // a nail
            }
            if roll(&[(x / 3) as i64, y as i64, 53]) > 0.95 {
                t -= 0.2; // a knot
            }
            t + ((x as f32 * 0.55 + from_top as f32 * 1.7 + roll(&[plank as i64, 54]) * 9.0).sin()) * 0.04 + jitter
        }
        Art::Slats { width } => {
            let slat = x.div_euclid(width);
            if x.rem_euclid(width) == 0 {
                return GAP;
            }
            0.45 + roll(&[slat as i64, (y / 24) as i64, 55]) * 0.25 + (y as f32 * 0.4 + slat as f32).sin() * 0.04 + jitter
        }
        Art::Grain => {
            let cx = x.rem_euclid(BLOCK);
            let mut t = 0.48 + (y as f32 * 0.9 + cx as f32 * 0.5 + roll(&[x as i64, 56]) * 4.0).sin() * 0.08;
            if cx == 1 {
                t += 0.1;
            }
            if roll(&[(x / 2) as i64, (y / 3) as i64, 57]) > 0.93 {
                t -= 0.14;
            }
            t + jitter
        }
        Art::Shingles { row, width } => {
            let r = y.div_euclid(row);
            let from_top = row - 1 - y.rem_euclid(row);
            let lx = x + if r.rem_euclid(2) == 1 { width / 2 + 1 } else { 0 };
            let tile_x = lx.div_euclid(width);
            let along = lx.rem_euclid(width);
            if from_top == row - 1 && (along == 0 || along == width - 1) {
                return OUTLINE;
            }
            if along == 0 {
                return GAP;
            }
            let mut t = 0.42 + roll(&[tile_x as i64, r as i64, 58]) * 0.3;
            if from_top == 0 {
                t -= 0.18; // in the shadow of the row above
            }
            if from_top == 1 {
                t += 0.18;
            }
            if along == width - 1 {
                t -= 0.08;
            }
            t + jitter
        }
        Art::Framed { panel } => {
            let b = BLOCK;
            let (cx, cy) = (x.rem_euclid(b), y.rem_euclid(b));
            let bx = x.div_euclid(b);
            // Beams round the plaster where it meets anything else, a post
            // every `panel` blocks, braces across (long diagonals, one way
            // and the other by turns).
            let beam = (a.n || a.other[0]) && cy >= b - 2
                || (a.s || a.other[1]) && cy <= 1
                || (a.w || a.other[2]) && cx <= 1
                || (a.e || a.other[3]) && cx >= b - 2
                || bx.rem_euclid(panel) == 0 && cx <= 1;
            let span = panel * b;
            let pi = x.div_euclid(span);
            let brace = if pi.rem_euclid(2) == 0 { (x - y).rem_euclid(span) <= 1 } else { (x + y).rem_euclid(span) <= 1 };
            if beam || brace {
                // (A dark wood on the light plaster: the low end of `colors`.)
                return 0.12 + (y as f32 * 0.9 + cx as f32).sin().abs() * 0.08 + roll(&[x as i64, y as i64, 59]) * 0.04;
            }
            let mut t = 0.72 + roll(&[(x / 3) as i64, (y / 3) as i64, 60]) * 0.18 + jitter;
            if roll(&[(x / 9) as i64, (y / 6) as i64, 61]) > 0.9 {
                t -= 0.07; // a stain
            }
            t
        }
        Art::Plain => 0.5 + roll(&[(x / 2) as i64, (y / 2) as i64, 62]) * 0.25 + jitter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bricks() -> Tile {
        Tile { art: Art::Bricks { course: 4, length: (6, 10) }, round: 3.0, outline: true, cracked: false }
    }

    /// Alone (open all round): outlined, its corners rounded off; shut in:
    /// the art alone, nothing cut.
    #[test]
    fn a_lone_block_is_outlined_and_rounded_a_shut_in_one_is_whole() {
        let t = bricks();
        let alone = Around { n: true, s: true, w: true, e: true, nw: true, ne: true, sw: true, se: true, other: [false; 4] };
        let b = BLOCK;
        // The corner cell (top-left: y at the block's top) is cut.
        assert_eq!(shade(&t, 0, b - 1, alone), None, "the corner's rounded off");
        // An edge cell mid-side is the outline (the darkest).
        assert_eq!(shade(&t, b / 2, b - 1, alone), Some(8), "the top edge is outlined");
        let shut = Around::SHUT;
        for y in 0..b {
            for x in 0..b {
                assert!(shade(&t, x, y, shut).is_some(), "nothing cut shut in ({x}, {y})");
            }
        }
        assert_ne!(shade(&t, b / 2, b - 1, shut), Some(8), "no outline shut in");
    }

    /// Two of its own side by side: no edge between them.
    #[test]
    fn a_block_beside_its_own_runs_on() {
        let t = bricks();
        let b = BLOCK;
        let left = Around { n: true, s: true, w: true, e: false, nw: true, ne: true, sw: true, se: true, other: [false; 4] };
        // Its east column isn't outlined, its top-right corner isn't cut.
        assert!(shade(&t, b - 1, b - 1, left).is_some(), "not rounded toward its neighbour");
        assert_ne!(shade(&t, b - 1, b / 2, left), Some(8), "no outline toward its neighbour");
    }
}
