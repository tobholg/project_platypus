//! Axis-aligned bodies moving through a cell grid (SPEC §5).
//!
//! Every moving thing — player, enemy, dropped item, projectile — is a `Body`
//! moved by `move_and_collide`. Movement is swept one cell at a time, so
//! nothing tunnels, and contacts come back as data.

use glam::{IVec2, Vec2};

/// What a body experiences in a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Occupancy {
    Empty,
    Liquid,
    Solid,
    /// One-way: solid to a body landing on it from above (unless it's
    /// dropping through), open from below and the sides.
    Platform,
}

/// The world as bodies see it. Implemented by the game over the cell world
/// (unloaded = solid) and by tests over tiny hand-made grids.
pub trait Grid {
    fn occupancy(&self, x: i32, y: i32) -> Occupancy;

    /// How much grip a body standing on this cell gets (1: full; ice
    /// little).
    #[inline]
    fn grip(&self, _x: i32, _y: i32) -> f32 {
        1.0
    }

    #[inline]
    fn solid(&self, x: i32, y: i32) -> bool {
        self.occupancy(x, y) == Occupancy::Solid
    }
}

/// Keeps a box that ends exactly on a cell boundary from claiming the next cell.
const EPS: f32 = 1e-3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    /// Centre of the box, in cells.
    pub pos: Vec2,
    /// Cells per second.
    pub vel: Vec2,
    pub half: Vec2,
    /// Highest ledge (in cells) walked up without jumping.
    pub step_height: i32,
    /// Dropping through platforms (holding down).
    pub drop: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Contacts {
    pub ground: bool,
    pub ceiling: bool,
    pub wall_left: bool,
    pub wall_right: bool,
    /// Downward speed when landing this tick (0 if not landing).
    pub impact: f32,
    /// Cells climbed by step-up this tick.
    pub stepped: i32,
    /// Fraction of the box inside liquid, 0..=1.
    pub submerged: f32,
    /// Standing on something, how much grip it gives (the least under its
    /// feet: one foot on ice slips); 1 off the ground.
    pub grip: f32,
}

impl Body {
    pub fn new(pos: Vec2, size: Vec2) -> Self {
        Body { pos, vel: Vec2::ZERO, half: size / 2.0, step_height: 0, drop: false }
    }

    /// Inclusive cell rectangle covered at `pos`.
    #[inline]
    pub fn cells_at(&self, pos: Vec2) -> (IVec2, IVec2) {
        let min = (pos - self.half).floor().as_ivec2();
        let max = (pos + self.half - Vec2::splat(EPS)).floor().as_ivec2();
        (min, max)
    }

    pub fn bottom(&self) -> f32 {
        self.pos.y - self.half.y
    }
}

pub fn overlaps_solid(grid: &impl Grid, min: IVec2, max: IVec2) -> bool {
    (min.y..=max.y).any(|y| (min.x..=max.x).any(|x| grid.solid(x, y)))
}

fn blocked(grid: &impl Grid, body: &Body, pos: Vec2) -> bool {
    let (min, max) = body.cells_at(pos);
    overlaps_solid(grid, min, max)
}

/// Moving down from `from` to `to`, does a platform catch the body? Only
/// one whose top its feet were at or above (it came from above), and not
/// when it's dropping through.
fn caught(grid: &impl Grid, body: &Body, from: Vec2, to: Vec2) -> bool {
    if body.drop {
        return false;
    }
    let (min, max) = body.cells_at(to);
    let row = min.y;
    from.y - body.half.y >= row as f32 + 1.0 - EPS && (min.x..=max.x).any(|x| grid.occupancy(x, row) == Occupancy::Platform)
}

/// Still, and on something solid or a platform, out of any liquid: a body
/// at rest, with nothing to move it (a cheap check: the row under its feet
/// and its middle).
pub fn resting(grid: &impl Grid, body: &Body) -> bool {
    if body.vel != Vec2::ZERO {
        return false;
    }
    let (min, max) = body.cells_at(body.pos);
    let under = (min.x..=max.x).any(|x| matches!(grid.occupancy(x, min.y - 1), Occupancy::Solid | Occupancy::Platform));
    let dry = grid.occupancy(((min.x + max.x) / 2).max(min.x), min.y) != Occupancy::Liquid;
    under && dry && (body.pos.y - body.half.y - min.y as f32).abs() < 0.01
}

/// Standing on something (solid, or a platform it isn't dropping through)?
fn grounded(grid: &impl Grid, body: &Body) -> bool {
    let probe = body.pos - Vec2::new(0.0, 2.0 * EPS);
    blocked(grid, body, probe) || caught(grid, body, body.pos, probe)
}

/// Move `body` by `vel * dt`, stopping at solids, stepping up small ledges.
pub fn move_and_collide(grid: &impl Grid, body: &mut Body, dt: f32) -> Contacts {
    let mut c = Contacts::default();
    // Sand fell on us, or a wall was placed inside us: climb out first.
    depenetrate(grid, body);

    let was_grounded = grounded(grid, body);
    move_x(grid, body, body.vel.x * dt, was_grounded && body.vel.y <= 0.0, &mut c);
    move_y(grid, body, body.vel.y * dt, &mut c);
    if !c.ground && body.vel.y <= 0.0 {
        c.ground = grounded(grid, body);
    }
    c.submerged = submerged(grid, body);
    c.grip = if c.ground { grip_under(grid, body) } else { 1.0 };
    c
}

/// The least grip of the cells under a body's feet (solid ones).
fn grip_under(grid: &impl Grid, body: &Body) -> f32 {
    let (min, max) = body.cells_at(body.pos - Vec2::new(0.0, 2.0 * EPS));
    let (lo, _) = body.cells_at(body.pos);
    let row = if min.y < lo.y { min.y } else { lo.y - 1 };
    (min.x..=max.x).filter(|&x| grid.occupancy(x, row) != Occupancy::Empty).map(|x| grid.grip(x, row)).fold(1.0, f32::min)
}

fn move_x(grid: &impl Grid, body: &mut Body, dx: f32, may_step: bool, c: &mut Contacts) {
    let dir = dx.signum();
    let mut left = dx.abs();
    while left > 0.0 {
        let step = left.min(1.0);
        left -= step;
        let next = body.pos + Vec2::new(dir * step, 0.0);
        if !blocked(grid, body, next) {
            body.pos = next;
            continue;
        }
        // Walk up ledges no higher than step_height.
        if may_step
            && let Some(h) = (1..=body.step_height).find(|&h| !blocked(grid, body, next + Vec2::new(0.0, h as f32)))
        {
            // Settle onto the ledge exactly.
            body.pos = next + Vec2::new(0.0, h as f32);
            body.pos.y = (body.pos.y - body.half.y).floor() + body.half.y;
            c.stepped += h;
            continue;
        }
        // Flush against the blocking column.
        if dir > 0.0 {
            let col = (next.x + body.half.x - EPS).floor();
            body.pos.x = col - body.half.x;
            c.wall_right = true;
        } else {
            let col = (next.x - body.half.x).floor();
            body.pos.x = col + 1.0 + body.half.x;
            c.wall_left = true;
        }
        body.vel.x = 0.0;
        return;
    }
}

fn move_y(grid: &impl Grid, body: &mut Body, dy: f32, c: &mut Contacts) {
    let dir = dy.signum();
    let mut left = dy.abs();
    while left > 0.0 {
        let step = left.min(1.0);
        left -= step;
        let next = body.pos + Vec2::new(0.0, dir * step);
        if !blocked(grid, body, next) && !(dir < 0.0 && caught(grid, body, body.pos, next)) {
            body.pos = next;
            continue;
        }
        if dir < 0.0 {
            let row = (next.y - body.half.y).floor();
            body.pos.y = row + 1.0 + body.half.y;
            c.ground = true;
            c.impact = -body.vel.y;
        } else {
            let row = (next.y + body.half.y - EPS).floor();
            body.pos.y = row - body.half.y;
            c.ceiling = true;
        }
        body.vel.y = 0.0;
        return;
    }
}

/// If the body overlaps solids, move it to the nearest free spot:
/// up first (buried by sand), then sideways.
pub fn depenetrate(grid: &impl Grid, body: &mut Body) -> bool {
    if !blocked(grid, body, body.pos) {
        return false;
    }
    let reach = (body.half.y * 2.0).ceil() as i32 + 2;
    let candidates = (1..=reach)
        .map(|d| Vec2::new(0.0, d as f32))
        .chain((1..=reach / 2).flat_map(|d| [Vec2::new(-(d as f32), 0.0), Vec2::new(d as f32, 0.0)]))
        .chain((1..=reach / 2).map(|d| Vec2::new(0.0, -(d as f32))));
    for off in candidates {
        if !blocked(grid, body, body.pos + off) {
            body.pos += off;
            return true;
        }
    }
    false
}

pub fn submerged(grid: &impl Grid, body: &Body) -> f32 {
    let (min, max) = body.cells_at(body.pos);
    let mut wet = 0;
    let mut total = 0;
    for y in min.y..=max.y {
        for x in min.x..=max.x {
            total += 1;
            wet += (grid.occupancy(x, y) == Occupancy::Liquid) as i32;
        }
    }
    if total == 0 { 0.0 } else { wet as f32 / total as f32 }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// ASCII grids for tests: `#` solid, `=` ice (solid, slippery), `~`
    /// liquid, row 0 = bottom line.
    pub struct Ascii(pub Vec<Vec<u8>>);

    impl Ascii {
        pub fn new(rows: &[&str]) -> Self {
            Ascii(rows.iter().rev().map(|r| r.bytes().collect()).collect())
        }
    }

    impl Grid for Ascii {
        fn occupancy(&self, x: i32, y: i32) -> Occupancy {
            if x < 0 || y < 0 {
                return Occupancy::Solid;
            }
            match self.0.get(y as usize).and_then(|r| r.get(x as usize)) {
                Some(b'#') | Some(b'=') | None => Occupancy::Solid,
                Some(b'~') => Occupancy::Liquid,
                Some(b'-') => Occupancy::Platform,
                _ => Occupancy::Empty,
            }
        }

        fn grip(&self, x: i32, y: i32) -> f32 {
            if self.0.get(y as usize).and_then(|r| r.get(x as usize)) == Some(&b'=') { 0.1 } else { 1.0 }
        }
    }

    fn open_room() -> Ascii {
        let mut rows = vec!["#                              #"; 20];
        rows.push("################################");
        Ascii::new(&rows)
    }

    /// A room with a platform across it: row 11, so its top is 12.
    fn platform_room() -> Ascii {
        let mut rows = vec!["#                              #"; 20];
        rows[9] = "#          ----------          #";
        rows.push("################################");
        Ascii::new(&rows)
    }

    fn fall(g: &Ascii, b: &mut Body, ticks: usize) {
        for _ in 0..ticks {
            b.vel.y -= 600.0 / 60.0;
            if move_and_collide(g, b, 1.0 / 60.0).ground {
                b.vel.y = 0.0;
            }
        }
    }

    #[test]
    fn a_platform_holds_from_above_and_lets_through_from_below() {
        let g = platform_room();
        // Dropped from above: lands on it.
        let mut b = Body::new(Vec2::new(15.0, 18.0), Vec2::new(2.0, 4.0));
        fall(&g, &mut b, 120);
        assert_eq!(b.bottom(), 12.0, "standing on the platform");
        // Holding down: through it, to the floor.
        b.drop = true;
        fall(&g, &mut b, 120);
        assert_eq!(b.bottom(), 1.0, "dropped through");
        // Jumping up from below: through it, and it holds on the way down.
        b.drop = false;
        b.vel.y = 170.0;
        for _ in 0..8 {
            move_and_collide(&g, &mut b, 1.0 / 60.0);
        }
        assert!(b.bottom() > 12.0, "rose through the platform ({})", b.bottom());
        fall(&g, &mut b, 120);
        assert_eq!(b.bottom(), 12.0, "and landed on it");
        // Walking into it from the side at its height: no wall.
        let mut w = Body::new(Vec2::new(5.0, 12.0), Vec2::new(2.0, 4.0));
        w.vel.x = 60.0;
        let c = move_and_collide(&g, &mut w, 0.2);
        assert!(!c.wall_right && w.pos.x > 16.0, "walked through its end");
    }

    #[test]
    fn falls_and_lands_flush_on_the_floor() {
        let g = open_room();
        let mut b = Body::new(Vec2::new(10.0, 15.0), Vec2::new(2.0, 4.0));
        let mut landed = None;
        for _ in 0..120 {
            b.vel.y -= 600.0 / 60.0;
            let c = move_and_collide(&g, &mut b, 1.0 / 60.0);
            if c.ground && landed.is_none() {
                landed = Some(c.impact);
            }
        }
        assert_eq!(b.bottom(), 1.0, "resting exactly on row 0");
        assert!(landed.unwrap() > 0.0);
    }

    #[test]
    fn fast_bodies_do_not_tunnel_through_thin_walls() {
        let g = Ascii::new(&["     #     ", "     #     ", "     #     ", "###########"]);
        let mut b = Body::new(Vec2::new(2.0, 2.0), Vec2::new(1.0, 1.0));
        b.vel.x = 10_000.0;
        let c = move_and_collide(&g, &mut b, 1.0 / 60.0);
        assert!(c.wall_right);
        assert_eq!(b.pos.x + b.half.x, 5.0, "stopped flush at the wall");
    }

    #[test]
    fn steps_up_small_ledges_but_not_walls() {
        let g = Ascii::new(&[
            "#            #",
            "#         ####",
            "#      #######",
            "#    #########",
            "##############",
        ]);
        let mut b = Body::new(Vec2::new(2.0, 2.0), Vec2::new(1.0, 2.0));
        b.step_height = 1;
        for _ in 0..60 {
            b.vel = Vec2::new(30.0, -5.0);
            move_and_collide(&g, &mut b, 1.0 / 60.0);
        }
        // Climbed the steps at x=5 and x=7; the grid's ceiling stops it before x=10.
        assert!(b.pos.x > 9.0, "walked up the staircase, x = {}", b.pos.x);
        let mut wall = Body::new(Vec2::new(2.0, 2.0), Vec2::new(1.0, 2.0));
        wall.step_height = 0;
        for _ in 0..30 {
            wall.vel.x = 30.0;
            move_and_collide(&g, &mut wall, 1.0 / 60.0);
        }
        assert_eq!(wall.pos.x + wall.half.x, 5.0, "without step-up the first ledge is a wall");
    }

    #[test]
    fn buried_bodies_climb_out() {
        let g = Ascii::new(&["      ", "      ", "######", "######", "######"]);
        let mut b = Body::new(Vec2::new(3.0, 1.5), Vec2::new(1.0, 1.0));
        assert!(depenetrate(&g, &mut b));
        assert_eq!(b.bottom(), 3.0);
    }

    #[test]
    fn reports_submersion() {
        let g = Ascii::new(&["#    #", "#~~~~#", "#~~~~#", "######"]);
        let b = Body::new(Vec2::new(3.0, 2.0), Vec2::new(2.0, 2.0));
        assert_eq!(submerged(&g, &b), 1.0);
        let b = Body::new(Vec2::new(3.0, 3.0), Vec2::new(2.0, 2.0));
        assert_eq!(submerged(&g, &b), 0.5);
    }
}
