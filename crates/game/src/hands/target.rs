//! Smart cursor (DESIGN §4.2): which block a swing or a placement is aimed at,
//! chosen for you. Pure functions over "what's at this block", so they're
//! tested without a world.
//!
//! Mining with the smart cursor clears a tunnel the body fits (`tunnel_target`):
//! aim below and hold, and you dig straight down; aim sideways, a tunnel you
//! can walk. Aimed diagonally it takes the first block with something to mine
//! on the line toward the cursor (you dig the face you see). Without the smart
//! cursor, the block under the cursor. Placing takes the block under the cursor if
//! it's free and touches something to hold it, or else the last free block
//! before that line runs into something.

use bevy::math::Vec2;
use platypus_sim::{BLOCK, CellPos};

/// Blocks crossed by the segment `from`→`to` (world cells), in order, each once.
pub fn blocks_along(from: Vec2, to: Vec2) -> Vec<CellPos> {
    let b = BLOCK as f32;
    let (a, z) = (from / b, to / b);
    let mut cur = (a.x.floor() as i32, a.y.floor() as i32);
    let end = (z.x.floor() as i32, z.y.floor() as i32);
    let d = z - a;
    let step = (d.x.signum() as i32, d.y.signum() as i32);
    // Distance along the segment (0..1) to the next vertical / horizontal grid line.
    let t_next = |p: f32, dir: f32, cell: i32| {
        if dir > 0.0 { (cell as f32 + 1.0 - p) / dir } else if dir < 0.0 { (cell as f32 - p) / dir } else { f32::INFINITY }
    };
    let (mut tx, mut ty) = (t_next(a.x, d.x, cur.0), t_next(a.y, d.y, cur.1));
    let (dtx, dty) = (if d.x != 0.0 { 1.0 / d.x.abs() } else { f32::INFINITY }, if d.y != 0.0 { 1.0 / d.y.abs() } else { f32::INFINITY });
    let mut out = vec![CellPos::new(cur.0, cur.1)];
    for _ in 0..4096 {
        if cur == end {
            break;
        }
        if tx < ty {
            cur.0 += step.0;
            tx += dtx;
        } else {
            cur.1 += step.1;
            ty += dty;
        }
        out.push(CellPos::new(cur.0, cur.1));
    }
    out
}

/// The line from `hand` toward `cursor`, cut to `reach` cells.
fn reach_line(hand: Vec2, cursor: Vec2, reach: f32) -> Vec<CellPos> {
    let d = cursor - hand;
    let to = if d.length() > reach { hand + d.normalize() * reach } else { cursor };
    blocks_along(hand, to)
}

/// The block a swing hits: the first on the line toward the cursor that has
/// something `minable`.
pub fn mine_target(hand: Vec2, cursor: Vec2, reach: f32, minable: impl Fn(CellPos) -> bool) -> Option<CellPos> {
    reach_line(hand, cursor, reach).into_iter().find(|&b| minable(b))
}

/// The smart cursor for digging (Terraria's): aimed mostly down, up or
/// sideways, it clears a tunnel the size of the body, nearest first. Aim below
/// and hold: the whole row under your feet goes before the next one, so you
/// drop into it and keep going. Aim sideways: a face as tall as you, column by
/// column. Aimed diagonally it takes the line toward the cursor.
///
/// `lo`, `hi`: the body's box (cells). Only blocks within `reach` of `hand`.
pub fn tunnel_target(lo: Vec2, hi: Vec2, hand: Vec2, cursor: Vec2, reach: f32, minable: impl Fn(CellPos) -> bool) -> Option<CellPos> {
    let b = BLOCK as f32;
    let d = cursor - (lo + hi) / 2.0;
    let vertical = d.y.abs() > d.x.abs() * 1.7;
    let horizontal = d.x.abs() > d.y.abs() * 1.7;
    if !vertical && !horizontal {
        return mine_target(hand, cursor, reach, minable);
    }
    let block = |v: f32| (v / b).floor() as i32;
    let in_reach = |c: CellPos| Vec2::new((c.x as f32 + 0.5) * b, (c.y as f32 + 0.5) * b).distance(hand) <= reach;
    // The blocks the body spans across (for a shaft) or up (for a tunnel).
    let (cols, rows) = ((block(lo.x)..=block(hi.x - 0.01)).collect::<Vec<_>>(), (block(lo.y)..=block(hi.y - 0.01)).collect::<Vec<_>>());
    for step in 0..16 {
        let mut layer: Vec<CellPos> = if vertical {
            let row = if d.y < 0.0 { block(lo.y - 0.5) - step } else { block(hi.y + 0.5) + step };
            cols.iter().map(|&x| CellPos::new(x, row)).collect()
        } else {
            let col = if d.x > 0.0 { block(hi.x + 0.5) + step } else { block(lo.x - 0.5) - step };
            rows.iter().map(|&y| CellPos::new(col, y)).collect()
        };
        if !layer.iter().any(|&c| in_reach(c)) {
            return None;
        }
        // Within a layer, the block nearest the cursor first.
        let centre = |c: &CellPos| Vec2::new((c.x as f32 + 0.5) * b, (c.y as f32 + 0.5) * b);
        layer.sort_by(|a, c| centre(a).distance(cursor).total_cmp(&centre(c).distance(cursor)));
        if let Some(&hit) = layer.iter().find(|&&c| in_reach(c) && minable(c)) {
            return Some(hit);
        }
    }
    None
}

/// Without the smart cursor: the block under the cursor, if it's within reach
/// and has something to mine.
pub fn cursor_target(hand: Vec2, cursor: Vec2, reach: f32, minable: impl Fn(CellPos) -> bool) -> Option<CellPos> {
    let under = CellPos::new((cursor.x / BLOCK as f32).floor() as i32, (cursor.y / BLOCK as f32).floor() as i32);
    let centre = Vec2::new((under.x as f32 + 0.5) * BLOCK as f32, (under.y as f32 + 0.5) * BLOCK as f32);
    (centre.distance(hand) <= reach && minable(under)).then_some(under)
}

/// The block a placement fills: under the cursor if it's `free` and
/// `supported` (and within reach), else, if the line toward the cursor runs
/// into something, the last free, supported block before it.
pub fn place_target(hand: Vec2, cursor: Vec2, reach: f32, free: impl Fn(CellPos) -> bool, supported: impl Fn(CellPos) -> bool) -> Option<CellPos> {
    let under = CellPos::new((cursor.x / BLOCK as f32).floor() as i32, (cursor.y / BLOCK as f32).floor() as i32);
    let centre = |b: CellPos| Vec2::new((b.x as f32 + 0.5) * BLOCK as f32, (b.y as f32 + 0.5) * BLOCK as f32);
    if centre(under).distance(hand) <= reach && free(under) && supported(under) {
        return Some(under);
    }
    // Only against something: pointing into open air places nothing (not a
    // block at your feet).
    let mut last = None;
    for b in reach_line(hand, cursor, reach) {
        if !free(b) {
            return last;
        }
        if supported(b) {
            last = Some(b);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// A floor of solid blocks at block row 0, a wall at block column 6.
    fn world() -> HashSet<(i32, i32)> {
        let mut s: HashSet<(i32, i32)> = (-10..20).map(|x| (x, 0)).collect();
        s.extend((1..5).map(|y| (6, y)));
        s
    }

    fn at(x: f32, y: f32) -> Vec2 {
        Vec2::new(x, y)
    }

    #[test]
    fn the_line_visits_every_block_it_crosses_once() {
        let b = blocks_along(at(1.0, 1.0), at(13.0, 6.0));
        assert_eq!(b.first(), Some(&CellPos::new(0, 0)));
        assert_eq!(b.last(), Some(&CellPos::new(3, 1)));
        for w in b.windows(2) {
            assert_eq!((w[1].x - w[0].x).abs() + (w[1].y - w[0].y).abs(), 1, "4-connected steps: {b:?}");
        }
    }

    #[test]
    fn mining_hits_the_face_you_see_not_what_is_buried_under_the_cursor() {
        let solid = world();
        let minable = |b: CellPos| solid.contains(&(b.x, b.y));
        // Standing at x 2 blocks, hand at 1.5 blocks up; cursor deep in the
        // wall beyond reach of its face: the face is hit.
        let hand = at(10.0, 6.0);
        let t = mine_target(hand, at(40.0, 10.0), 24.0, minable);
        assert_eq!(t, Some(CellPos::new(6, 1)), "the wall's face");
        // Pointing down: the floor under you.
        assert_eq!(mine_target(hand, at(10.0, -6.0), 24.0, minable), Some(CellPos::new(2, 0)));
        // Out of reach: nothing.
        assert_eq!(mine_target(hand, at(10.0, 60.0), 24.0, minable), None);
    }

    /// Solid ground everywhere below block row 0.
    fn ground(dug: &HashSet<(i32, i32)>) -> impl Fn(CellPos) -> bool + '_ {
        move |b: CellPos| b.y < 0 && !dug.contains(&(b.x, b.y))
    }

    #[test]
    fn holding_down_digs_a_shaft_the_body_fits_row_by_row() {
        // A body 6 wide standing on the ground at y 0, from x 6 to 12:
        // across blocks 1, 2 (cells 4–11).
        let mut dug = HashSet::new();
        let mut order = Vec::new();
        for _ in 0..6 {
            let (lo, hi) = (at(6.0, 0.0), at(12.0, 15.0));
            let t = tunnel_target(lo, hi, at(9.0, 10.0), at(10.0, -30.0), 24.0, ground(&dug)).expect("something to dig");
            dug.insert((t.x, t.y));
            order.push((t.x, t.y));
        }
        // Row -1 whole (both blocks, nearest the cursor first), then row -2...
        assert_eq!(order[..4], [(2, -1), (1, -1), (2, -2), (1, -2)], "{order:?}");
    }

    #[test]
    fn aiming_sideways_digs_a_face_as_tall_as_the_body() {
        let dug = HashSet::new();
        // A wall of solid blocks from x 4 (cell 16) on.
        let wall = |b: CellPos| b.x >= 4 && b.y >= 0;
        let (lo, hi) = (at(6.0, 0.0), at(12.0, 15.0));
        let t = tunnel_target(lo, hi, at(9.0, 10.0), at(40.0, 8.0), 24.0, wall).unwrap();
        assert_eq!(t.x, 4, "the column just in front");
        assert!((0..=3).contains(&t.y), "within the body's height");
        // Diagonally it's the line toward the cursor, like before.
        assert_eq!(tunnel_target(lo, hi, at(9.0, 10.0), at(20.0, -2.0), 24.0, ground(&dug)), mine_target(at(9.0, 10.0), at(20.0, -2.0), 24.0, ground(&dug)));
    }

    #[test]
    fn placing_goes_under_the_cursor_or_against_what_the_line_meets() {
        let solid = world();
        let free = |b: CellPos| !solid.contains(&(b.x, b.y));
        let supported = |b: CellPos| [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| solid.contains(&(b.x + dx, b.y + dy)));
        let hand = at(10.0, 6.0);
        // On the floor, under the cursor.
        assert_eq!(place_target(hand, at(18.5, 5.0), 24.0, free, supported), Some(CellPos::new(4, 1)));
        // Cursor inside the wall: against the wall's face.
        assert_eq!(place_target(hand, at(26.0, 6.0), 24.0, free, supported), Some(CellPos::new(5, 1)));
        // Mid-air with nothing near: nothing to hold it.
        assert_eq!(place_target(hand, at(14.0, 18.0), 24.0, free, supported), None);
    }
}
