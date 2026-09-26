//! Turning pixel art without ruining it (RotSprite, roughly): the sprite is
//! scaled up 8× with Scale2x three times (which rounds diagonals instead of
//! blowing up stairs), turned there with nearest sampling, and each output
//! pixel takes the colour at its centre. Held weapons are drawn this way at
//! every angle they can point, ahead of time, and the same pictures are
//! their hit masks.

use crate::Pixels;

/// Scale2x (EPX): twice the size, diagonal edges kept smooth.
pub fn scale2x(p: &Pixels) -> Pixels {
    let mut out = Pixels::new(p.w * 2, p.h * 2);
    for y in 0..p.h as i32 {
        for x in 0..p.w as i32 {
            let e = p.get(x, y);
            let (a, b, c, d) = (p.get(x, y - 1), p.get(x + 1, y), p.get(x - 1, y), p.get(x, y + 1));
            let (mut e0, mut e1, mut e2, mut e3) = (e, e, e, e);
            if a != d && c != b {
                if c == a {
                    e0 = c;
                }
                if a == b {
                    e1 = b;
                }
                if c == d {
                    e2 = c;
                }
                if d == b {
                    e3 = b;
                }
            }
            out.set(x * 2, y * 2, e0);
            out.set(x * 2 + 1, y * 2, e1);
            out.set(x * 2, y * 2 + 1, e2);
            out.set(x * 2 + 1, y * 2 + 1, e3);
        }
    }
    out
}

/// A sprite turned `degrees` (counter-clockwise, as angles go up) about
/// `pivot` (a pixel's centre, from the top-left). The result is a square
/// with the pivot in the middle pixel: `side / 2` is where it sits.
pub fn rotsprite(p: &Pixels, pivot: (i32, i32), degrees: f32) -> Pixels {
    // Far enough out to hold every pixel at any angle.
    let mut reach = 0.0f32;
    for y in 0..p.h as i32 {
        for x in 0..p.w as i32 {
            if p.opaque(x, y) {
                let (dx, dy) = ((x - pivot.0) as f32, (y - pivot.1) as f32);
                reach = reach.max((dx.abs() + 0.5).hypot(dy.abs() + 0.5));
            }
        }
    }
    let half = reach.ceil() as i32;
    let side = (half * 2 + 1) as u32;
    let big = scale2x(&scale2x(&scale2x(p)));
    let (s, c) = degrees.to_radians().sin_cos();
    let mut out = Pixels::new(side, side);
    for oy in 0..side as i32 {
        for ox in 0..side as i32 {
            // The output pixel's centre, from the pivot, y down: turn it back.
            let (dx, dy) = ((ox - half) as f32, (oy - half) as f32);
            // (Screen y is down, so a counter-clockwise turn is clockwise here.)
            let sx = dx * c - dy * s;
            let sy = dx * s + dy * c;
            let (bx, by) = (((pivot.0 as f32 + 0.5 + sx) * 8.0).floor() as i32, ((pivot.1 as f32 + 0.5 + sy) * 8.0).floor() as i32);
            out.set(ox, oy, big.get(bx, by));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar() -> Pixels {
        // A 6 × 1 bar, its pivot at its left end.
        let mut p = Pixels::new(6, 1);
        for x in 0..6 {
            p.set(x, 0, [200, 200, 200, 255]);
        }
        p
    }

    #[test]
    fn turning_keeps_the_pixels_and_points_where_it_says() {
        let p = bar();
        let flat = rotsprite(&p, (0, 0), 0.0);
        let half = flat.w as i32 / 2;
        let count = |q: &Pixels| (0..q.h as i32).flat_map(|y| (0..q.w as i32).map(move |x| (x, y))).filter(|&(x, y)| q.opaque(x, y)).count();
        assert_eq!(count(&flat), 6);
        assert!(flat.opaque(half + 5, half), "level: to the right");
        let up = rotsprite(&p, (0, 0), 90.0);
        assert!(up.opaque(half, half - 5), "90: straight up");
        let down = rotsprite(&p, (0, 0), -90.0);
        assert!(down.opaque(half, half + 5), "-90: straight down");
        let diag = rotsprite(&p, (0, 0), 45.0);
        assert!(diag.opaque(half + 3, half - 3), "45: up and right");
        assert!((4..=7).contains(&count(&diag)), "a diagonal (6 long: 4 to 5 steps): {}", count(&diag));
    }
}
