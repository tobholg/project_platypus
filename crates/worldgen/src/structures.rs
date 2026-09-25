//! Structures (DESIGN §3.3): crypts now, castles next. A structure is a
//! list of pieces, each a grid of glyphs at block resolution (4 × 4 cells,
//! on the mining grid) placed in the world. Rooms come from text files
//! (`assets/data/rooms/*.rooms`) and are assembled Spelunky-style: a path of
//! rooms from the entrance to a goal room on a coarse grid, side rooms off
//! it, a secret or two behind illusory walls, the doors nobody uses walled
//! up.
//!
//! Layouts are made once, when the world is planned (from the seed and the
//! site), and chunks rasterise whatever parts of them they touch.

use std::collections::HashMap;
use std::sync::OnceLock;

use platypus_sim::CHUNK;
use platypus_sim::rng::Rng;

/// Cells per block (the mining grid).
pub const BLOCK: i32 = 4;
/// A room slot, in blocks.
pub const SLOT_W: i32 = 16;
pub const SLOT_H: i32 = 10;
/// Side doors are these rows of a slot (from its bottom); holes in the top
/// or bottom are these columns.
const DOOR_ROWS: [i32; 5] = [1, 2, 3, 4, 5];
const HOLE_COLS: [i32; 4] = [6, 7, 8, 9];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Glyph {
    /// Leave the terrain as it is.
    Keep,
    Wall,
    /// Open, with the wall behind it.
    Open,
    /// Open to the sky (no wall behind).
    Sky,
    /// A chest's bottom-left block.
    Chest,
    Candle,
    Spawn,
    Boss,
    Water,
    Lava,
    Spikes,
    Illusory,
    Weak,
    Planks,
    Rubble,
}

impl Glyph {
    fn from_char(c: char) -> Option<Glyph> {
        Some(match c {
            ' ' => Glyph::Keep,
            '#' => Glyph::Wall,
            '.' => Glyph::Open,
            ',' => Glyph::Sky,
            'C' => Glyph::Chest,
            'T' => Glyph::Candle,
            'S' => Glyph::Spawn,
            'B' => Glyph::Boss,
            'W' => Glyph::Water,
            'L' => Glyph::Lava,
            '^' => Glyph::Spikes,
            '?' => Glyph::Illusory,
            '%' => Glyph::Weak,
            '=' => Glyph::Planks,
            '~' => Glyph::Rubble,
            _ => return None,
        })
    }

    /// Something you can pass (a door is open).
    pub fn passable(self) -> bool {
        !matches!(self, Glyph::Keep | Glyph::Wall | Glyph::Weak | Glyph::Planks | Glyph::Spikes)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Side {
    L,
    R,
    T,
    B,
}

impl Side {
    fn opposite(self) -> Side {
        match self {
            Side::L => Side::R,
            Side::R => Side::L,
            Side::T => Side::B,
            Side::B => Side::T,
        }
    }
}

/// A door in a room's edge: its side, and which slot along that side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Socket {
    pub side: Side,
    pub slot: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomKind {
    Room,
    Entrance,
    Goal,
    Secret,
    /// On the surface, over the way down.
    Ruin,
}

#[derive(Clone, Debug)]
pub struct Room {
    pub name: String,
    pub kind: RoomKind,
    /// Size in slots.
    pub slots: (i32, i32),
    /// Row 0 is the bottom.
    glyphs: Vec<Glyph>,
    /// The doors it has.
    pub sockets: Vec<Socket>,
}

impl Room {
    /// Size in blocks.
    pub fn size(&self) -> (i32, i32) {
        (self.slots.0 * SLOT_W, self.slots.1 * SLOT_H)
    }

    pub fn at(&self, bx: i32, by: i32) -> Glyph {
        self.glyphs[(by * self.size().0 + bx) as usize]
    }

    /// Every door position a room this size could have.
    fn all_sockets(slots: (i32, i32)) -> Vec<Socket> {
        let mut v = Vec::new();
        for slot in 0..slots.1 {
            v.push(Socket { side: Side::L, slot });
            v.push(Socket { side: Side::R, slot });
        }
        for slot in 0..slots.0 {
            v.push(Socket { side: Side::T, slot });
            v.push(Socket { side: Side::B, slot });
        }
        v
    }

    /// The blocks of a door.
    fn door(&self, s: Socket) -> Vec<(i32, i32)> {
        let (w, h) = self.size();
        match s.side {
            Side::L => DOOR_ROWS.iter().map(|r| (0, s.slot * SLOT_H + r)).collect(),
            Side::R => DOOR_ROWS.iter().map(|r| (w - 1, s.slot * SLOT_H + r)).collect(),
            Side::B => HOLE_COLS.iter().map(|c| (s.slot * SLOT_W + c, 0)).collect(),
            Side::T => HOLE_COLS.iter().map(|c| (s.slot * SLOT_W + c, h - 1)).collect(),
        }
    }
}

/// Rooms from a `.rooms` file.
pub fn parse(src: &str) -> Result<Vec<Room>, String> {
    let mut rooms = Vec::new();
    let mut current: Option<(String, RoomKind, Vec<String>)> = None;
    let finish = |(name, kind, mut rows): (String, RoomKind, Vec<String>), rooms: &mut Vec<Room>| -> Result<(), String> {
        while rows.last().is_some_and(|r| r.trim().is_empty()) {
            rows.pop();
        }
        let width = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) as i32;
        let slots = ((width + SLOT_W - 1) / SLOT_W, rows.len() as i32 / SLOT_H);
        if slots.0 == 0 || rows.len() as i32 != slots.1 * SLOT_H || slots.1 == 0 {
            return Err(format!("room `{name}`: {} rows of up to {width}: not whole {SLOT_W} × {SLOT_H} slots", rows.len()));
        }
        let (w, h) = (slots.0 * SLOT_W, slots.1 * SLOT_H);
        let mut glyphs = vec![Glyph::Keep; (w * h) as usize];
        for (i, row) in rows.iter().enumerate() {
            let by = h - 1 - i as i32;
            for (bx, c) in row.chars().enumerate() {
                let g = Glyph::from_char(c).ok_or(format!("room `{name}`: unknown `{c}`"))?;
                glyphs[(by * w + bx as i32) as usize] = g;
            }
        }
        let mut room = Room { name: name.clone(), kind, slots, glyphs, sockets: Vec::new() };
        for s in Room::all_sockets(slots) {
            let door = room.door(s);
            let open = door.iter().filter(|&&(x, y)| room.at(x, y).passable()).count();
            if open == door.len() {
                room.sockets.push(s);
            } else if open > 0 {
                return Err(format!("room `{name}`: door {s:?} is half open"));
            }
        }
        if kind == RoomKind::Ruin {
            if !room.sockets.contains(&Socket { side: Side::B, slot: 0 }) {
                return Err(format!("ruin `{name}` has no way down (a hole in its floor)"));
            }
        } else {
            // A room's edge is wall except its doors, so walling up a door
            // closes it.
            let doors: Vec<(i32, i32)> = room.sockets.iter().flat_map(|&s| room.door(s)).collect();
            for bx in 0..w {
                for by in 0..h {
                    let edge = bx == 0 || by == 0 || bx == w - 1 || by == h - 1;
                    if edge && !doors.contains(&(bx, by)) && room.at(bx, by) != Glyph::Wall {
                        return Err(format!("room `{name}`: its edge at ({bx}, {by}) is open outside a door"));
                    }
                }
            }
        }
        rooms.push(room);
        Ok(())
    };
    for line in src.lines() {
        if let Some(rest) = line.strip_prefix("room ") {
            if let Some(r) = current.take() {
                finish(r, &mut rooms)?;
            }
            let mut words = rest.split_whitespace();
            let name = words.next().ok_or("a room without a name")?.to_string();
            let kind = match words.next().unwrap_or("room") {
                "room" => RoomKind::Room,
                "entrance" => RoomKind::Entrance,
                "goal" => RoomKind::Goal,
                "secret" => RoomKind::Secret,
                "ruin" => RoomKind::Ruin,
                k => return Err(format!("room `{name}`: unknown kind `{k}`")),
            };
            current = Some((name, kind, Vec::new()));
        } else if let Some((_, _, rows)) = &mut current {
            rows.push(line.to_string());
        }
        // (Before the first room: comments.)
    }
    if let Some(r) = current.take() {
        finish(r, &mut rooms)?;
    }
    Ok(rooms)
}

pub fn crypt_rooms() -> &'static [Room] {
    static ROOMS: OnceLock<Vec<Room>> = OnceLock::new();
    ROOMS.get_or_init(|| parse(include_str!("../../../assets/data/rooms/crypt.rooms")).unwrap_or_else(|e| panic!("crypt.rooms: {e}")))
}

/// A grid of glyphs placed in the world.
#[derive(Clone, Debug)]
pub struct Piece {
    /// Bottom-left cell (on the block grid).
    pub x: i32,
    pub y: i32,
    /// Size in blocks.
    pub w: i32,
    pub h: i32,
    glyphs: Vec<Glyph>,
}

impl Piece {
    fn new(bx: i32, by: i32, w: i32, h: i32, glyphs: Vec<Glyph>) -> Piece {
        Piece { x: bx * BLOCK, y: by * BLOCK, w, h, glyphs }
    }

    /// A room at block (bx, by), its unused doors walled up and `secret`
    /// doors made illusory walls.
    fn room(room: &Room, bx: i32, by: i32, used: &[Socket], secret: &[Socket]) -> Piece {
        let mut glyphs = room.glyphs.clone();
        let w = room.size().0;
        for &s in &room.sockets {
            let fill = if secret.contains(&s) {
                Glyph::Illusory
            } else if used.contains(&s) {
                continue;
            } else {
                Glyph::Wall
            };
            for (x, y) in room.door(s) {
                glyphs[(y * w + x) as usize] = fill;
            }
        }
        let (w, h) = room.size();
        Piece::new(bx, by, w, h, glyphs)
    }

    /// The glyph at a world cell, if the piece covers it.
    pub fn glyph(&self, x: i32, y: i32) -> Option<Glyph> {
        let (bx, by) = ((x - self.x).div_euclid(BLOCK), (y - self.y).div_euclid(BLOCK));
        (bx >= 0 && by >= 0 && bx < self.w && by < self.h).then(|| self.glyphs[(by * self.w + bx) as usize])
    }

    /// Blocks holding `g`, as their bottom-left cells.
    pub fn blocks_of(&self, g: Glyph) -> impl Iterator<Item = (i32, i32)> + '_ {
        (0..self.h).flat_map(move |by| (0..self.w).map(move |bx| (bx, by))).filter(move |&(bx, by)| self.glyphs[(by * self.w + bx) as usize] == g).map(|(bx, by)| (self.x + bx * BLOCK, self.y + by * BLOCK))
    }

    /// (x0, y0, x1, y1) in cells, inclusive.
    pub fn bbox(&self) -> (i32, i32, i32, i32) {
        (self.x, self.y, self.x + self.w * BLOCK - 1, self.y + self.h * BLOCK - 1)
    }

    pub fn fingerprint(&self) -> u64 {
        self.glyphs.iter().fold(hash_i(&[self.x, self.y, self.w, self.h]), |a, &g| a.wrapping_mul(31).wrapping_add(g as u64))
    }
}

fn hash_i(v: &[i32]) -> u64 {
    platypus_sim::rng::hash(&v.iter().map(|&x| x as u64).collect::<Vec<_>>())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureKind {
    Crypt,
}

impl StructureKind {
    pub fn name(self) -> &'static str {
        match self {
            StructureKind::Crypt => "crypt",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Structure {
    pub kind: StructureKind,
    /// Where it was sited (a crypt: its ruin's middle, on the surface).
    pub site: (i32, i32),
    /// Rooms in its grid, and the grid's size in slots.
    pub rooms: usize,
    pub grid: (i32, i32),
    pub pieces: Vec<Piece>,
}

impl Structure {
    /// (x0, y0, x1, y1) in cells, inclusive.
    pub fn bbox(&self) -> (i32, i32, i32, i32) {
        self.pieces.iter().map(Piece::bbox).fold((i32::MAX, i32::MAX, i32::MIN, i32::MIN), |a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))
    }
}

/// Every structure, and which pieces touch each chunk.
#[derive(Default)]
pub struct Structures {
    pub list: Vec<Structure>,
    bins: HashMap<(i32, i32), Vec<(u16, u16)>>,
}

impl Structures {
    pub fn new(list: Vec<Structure>) -> Structures {
        let mut bins: HashMap<(i32, i32), Vec<(u16, u16)>> = HashMap::new();
        for (si, s) in list.iter().enumerate() {
            for (pi, p) in s.pieces.iter().enumerate() {
                let (x0, y0, x1, y1) = p.bbox();
                for cy in y0.div_euclid(CHUNK)..=y1.div_euclid(CHUNK) {
                    for cx in x0.div_euclid(CHUNK)..=x1.div_euclid(CHUNK) {
                        bins.entry((cx, cy)).or_default().push((si as u16, pi as u16));
                    }
                }
            }
        }
        Structures { list, bins }
    }

    /// The pieces touching a chunk.
    pub fn pieces_in(&self, cx: i32, cy: i32) -> impl Iterator<Item = (&Structure, &Piece)> {
        self.bins.get(&(cx, cy)).into_iter().flatten().map(|&(s, p)| {
            let s = &self.list[s as usize];
            (s, &s.pieces[p as usize])
        })
    }

    /// What a structure puts at a cell (the first piece there that doesn't
    /// leave it to the terrain).
    pub fn glyph_at(&self, x: i32, y: i32) -> Option<(Glyph, StructureKind)> {
        self.pieces_in(x.div_euclid(CHUNK), y.div_euclid(CHUNK)).find_map(|(s, p)| p.glyph(x, y).filter(|&g| g != Glyph::Keep).map(|g| (g, s.kind)))
    }

    /// Anything within `margin` cells of column x?
    pub fn near_column(&self, x: i32, margin: i32) -> bool {
        self.list.iter().any(|s| {
            let (x0, _, x1, _) = s.bbox();
            x >= x0 - margin && x <= x1 + margin
        })
    }

    pub fn checksum(&self) -> u64 {
        self.list.iter().flat_map(|s| &s.pieces).fold(0u64, |a, p| a.wrapping_mul(31).wrapping_add(p.fingerprint()))
    }
}

/// A room being laid out on the grid.
struct Node {
    cx: i32,
    cy: i32,
    /// Width in slots.
    w: i32,
    kind: RoomKind,
    used: Vec<Socket>,
    secret: Vec<Socket>,
}

impl Node {
    fn covers(&self, cx: i32, cy: i32) -> bool {
        cy == self.cy && cx >= self.cx && cx < self.cx + self.w
    }

    /// The socket at grid cell (cx, cy) on `side`.
    fn socket(&self, cx: i32, side: Side) -> Socket {
        match side {
            Side::L | Side::R => Socket { side, slot: 0 },
            Side::T | Side::B => Socket { side, slot: cx - self.cx },
        }
    }
}

fn step(cx: i32, cy: i32, side: Side) -> (i32, i32) {
    match side {
        Side::L => (cx - 1, cy),
        Side::R => (cx + 1, cy),
        Side::T => (cx, cy - 1),
        Side::B => (cx, cy + 1),
    }
}

/// Join the rooms at grid cells a and b (b is `side` of a).
fn link(nodes: &mut [Node], a: (i32, i32), side: Side, secret: bool) {
    let b = step(a.0, a.1, side);
    let ia = nodes.iter().position(|n| n.covers(a.0, a.1)).expect("a is laid out");
    let ib = nodes.iter().position(|n| n.covers(b.0, b.1)).expect("b is laid out");
    let sa = nodes[ia].socket(a.0, side);
    let sb = nodes[ib].socket(b.0, side.opposite());
    nodes[ia].used.push(sa);
    nodes[ib].used.push(sb);
    if secret {
        nodes[ib].secret.push(sb);
    }
}

fn pick<'a>(rooms: &'a [Room], rng: &mut Rng, kind: RoomKind, w: i32, need: &[Socket]) -> Option<&'a Room> {
    let fits: Vec<&Room> = rooms.iter().filter(|r| r.kind == kind && r.slots == (w, 1) && need.iter().all(|s| r.sockets.contains(s))).collect();
    (!fits.is_empty()).then(|| fits[rng.next_u32() as usize % fits.len()])
}

/// A crypt: a ruin on the surface at block (bx, by) (its floor's bottom-left
/// block), a shaft down `depth` blocks, then a `grid` of room slots with the
/// entrance under the shaft, a path of rooms down to the goal, side rooms
/// and secrets.
pub fn crypt(rooms: &[Room], rng: &mut Rng, site: (i32, i32), grid: (i32, i32), depth: i32) -> Structure {
    let (gw, gh) = grid;
    // The ruin's floor: the block at the surface under the site's middle.
    let (rx, ry) = (site.0.div_euclid(BLOCK) - SLOT_W / 2, (site.1 - 1).div_euclid(BLOCK));
    let entrance = (rng.next_u32() % gw as u32) as i32;
    let gx = rx - entrance * SLOT_W;
    let top = ry - depth;
    let slot_at = |cx: i32, cy: i32| (gx + cx * SLOT_W, top - (cy + 1) * SLOT_H);

    // The path: across, now and then down, until the bottom row.
    let mut path = vec![(entrance, 0)];
    let mut dir = if rng.chance(128) { 1 } else { -1 };
    let mut on_bottom = 0;
    loop {
        let (cx, cy) = *path.last().expect("starts with the entrance");
        let free = |x: i32, path: &[(i32, i32)]| x >= 0 && x < gw && !path.contains(&(x, cy));
        if cy == gh - 1 {
            if on_bottom > 0 && (rng.chance(110) || !free(cx + dir, &path)) {
                break;
            }
            on_bottom += 1;
        }
        let down = cy + 1 < gh && (rng.chance(80) || (!free(cx + dir, &path) && !free(cx - dir, &path)));
        if down {
            path.push((cx, cy + 1));
            continue;
        }
        if !free(cx + dir, &path) {
            dir = -dir;
        }
        if free(cx + dir, &path) {
            path.push((cx + dir, cy));
        } else if cy + 1 < gh {
            path.push((cx, cy + 1));
        } else {
            break;
        }
    }

    let mut nodes: Vec<Node> = Vec::new();
    let node = |cx, cy, w, kind| Node { cx, cy, w, kind, used: Vec::new(), secret: Vec::new() };
    for (i, &(cx, cy)) in path.iter().enumerate().take(path.len() - 1) {
        nodes.push(node(cx, cy, 1, if i == 0 { RoomKind::Entrance } else { RoomKind::Room }));
    }
    // The goal: two slots wide if there's room beside the path's end.
    let (ex, ey) = *path.last().expect("a path");
    let taken = |x: i32, nodes: &[Node]| nodes.iter().any(|n| n.covers(x, ey));
    let wide = [dir, -dir].into_iter().find(|&d| ex + d >= 0 && ex + d < gw && !taken(ex + d, &nodes));
    match wide {
        Some(d) => nodes.push(node(ex.min(ex + d), ey, 2, RoomKind::Goal)),
        None => nodes.push(node(ex, ey, 1, RoomKind::Goal)),
    }
    if path.len() == 1 {
        // (A one-room crypt: the goal is the entrance.)
        nodes[0].kind = RoomKind::Goal;
    }
    for w in path.windows(2) {
        let side = match (w[1].0 - w[0].0, w[1].1 - w[0].1) {
            (1, 0) => Side::R,
            (-1, 0) => Side::L,
            _ => Side::B,
        };
        link(&mut nodes, w[0], side, false);
    }

    // Side rooms off the path (not the goal), a third of them secrets; at
    // least one secret if there's anywhere to put it.
    let path_rooms = path.len() - 1;
    let mut spots = Vec::new();
    for &(cx, cy) in &path[..path_rooms] {
        for side in [Side::L, Side::R, Side::B] {
            let (nx, ny) = step(cx, cy, side);
            if nx >= 0 && nx < gw && ny < gh && !nodes.iter().any(|n| n.covers(nx, ny)) {
                spots.push(((cx, cy), side));
            }
        }
    }
    let mut secrets = 0;
    for (k, &(from, side)) in spots.iter().enumerate() {
        let (nx, ny) = step(from.0, from.1, side);
        if nodes.iter().any(|n| n.covers(nx, ny)) {
            continue;
        }
        let last_chance = secrets == 0 && k + 1 == spots.len();
        if !last_chance && !rng.chance(90) {
            continue;
        }
        let secret = last_chance || rng.chance(85);
        nodes.push(node(nx, ny, 1, if secret { RoomKind::Secret } else { RoomKind::Room }));
        link(&mut nodes, from, side, secret);
        secrets += secret as i32;
    }

    // The ruin, the shaft, the rooms.
    let ruin = pick(rooms, rng, RoomKind::Ruin, 1, &[]).expect("a ruin room");
    let mut pieces = vec![Piece::room(ruin, rx, ry, &[Socket { side: Side::B, slot: 0 }], &[])];
    pieces.push(shaft(rx, top, ry - top));
    for n in &nodes {
        let mut need = n.used.clone();
        if n.kind == RoomKind::Entrance || (n.kind == RoomKind::Goal && path.len() == 1) {
            need.push(Socket { side: Side::T, slot: 0 });
        }
        let room = pick(rooms, rng, n.kind, n.w, &need)
            .or_else(|| pick(rooms, rng, RoomKind::Room, n.w, &need))
            .unwrap_or_else(|| panic!("no {:?} room {} wide with doors {need:?}", n.kind, n.w));
        let (bx, by) = slot_at(n.cx, n.cy);
        pieces.push(Piece::room(room, bx, by, &need, &n.secret));
    }
    Structure { kind: StructureKind::Crypt, site, rooms: nodes.len(), grid, pieces }
}

/// The way down from a ruin to its crypt: a shaft `h` blocks tall whose
/// bottom is block row `bottom`, lined with wall, with ledges to climb
/// back up and a candle now and then.
fn shaft(rx: i32, bottom: i32, h: i32) -> Piece {
    let w = SLOT_W;
    let mut glyphs = vec![Glyph::Keep; (w * h) as usize];
    for y in 0..h {
        let row = &mut glyphs[(y * w) as usize..((y + 1) * w) as usize];
        row[4] = Glyph::Wall;
        row[11] = Glyph::Wall;
        for g in &mut row[5..=10] {
            *g = Glyph::Open;
        }
        // Ledges every 7 blocks, left and right in turn.
        if y % 7 == 4 {
            let at = if (y / 7) % 2 == 0 { 5 } else { 9 };
            row[at] = Glyph::Planks;
            row[at + 1] = Glyph::Planks;
        }
        if y % 14 == 10 {
            row[if (y / 14) % 2 == 0 { 10 } else { 5 }] = Glyph::Candle;
        }
    }
    Piece::new(rx, bottom, w, h, glyphs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn the_crypt_rooms_parse_with_doors_all_round() {
        let rooms = crypt_rooms();
        let all = |r: &Room| Room::all_sockets(r.slots).iter().all(|s| r.sockets.contains(s));
        for kind in [RoomKind::Entrance, RoomKind::Room, RoomKind::Goal, RoomKind::Secret] {
            // Any combination of doors can be made by walling some up.
            assert!(rooms.iter().any(|r| r.kind == kind && r.slots == (1, 1) && all(r)), "a {kind:?} with every door");
        }
        assert!(rooms.iter().any(|r| r.kind == RoomKind::Goal && r.slots == (2, 1) && all(r)), "a wide goal");
        assert!(rooms.iter().any(|r| r.kind == RoomKind::Ruin));
        assert!(parse("room bad\n#..#").is_err(), "a room must be whole slots");
    }

    /// Blocks you can reach from the ruin's inside, through what's passable
    /// (illusory walls too), without leaving the structure.
    fn reachable(s: &Structure) -> HashSet<(i32, i32)> {
        let glyph = |bx: i32, by: i32| s.pieces.iter().find_map(|p| p.glyph(bx * BLOCK, by * BLOCK).filter(|&g| g != Glyph::Keep));
        let ruin = &s.pieces[0];
        let start = (ruin.x / BLOCK + 7, ruin.y / BLOCK + 1);
        assert!(glyph(start.0, start.1).is_some_and(Glyph::passable), "the ruin's middle is open");
        let mut seen = HashSet::from([start]);
        let mut stack = vec![start];
        while let Some((x, y)) = stack.pop() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let q = (x + dx, y + dy);
                if !seen.contains(&q) && glyph(q.0, q.1).is_some_and(Glyph::passable) {
                    seen.insert(q);
                    stack.push(q);
                }
            }
        }
        seen
    }

    #[test]
    fn every_chest_in_a_crypt_can_be_reached_from_its_ruin() {
        let rooms = crypt_rooms();
        let mut secrets = 0;
        for seed in 0..300u64 {
            let mut rng = Rng::seeded(&[seed, 5]);
            let grid = (3 + (seed % 4) as i32, 3 + (seed / 4 % 3) as i32);
            let s = crypt(rooms, &mut rng, (10_000, 8_000), grid, 40);
            let reach = reachable(&s);
            let chests: Vec<(i32, i32)> = s.pieces.iter().flat_map(|p| p.blocks_of(Glyph::Chest)).map(|(x, y)| (x / BLOCK, y / BLOCK)).collect();
            assert!(chests.len() >= 2, "seed {seed}: a goal's worth of chests ({})", chests.len());
            for c in &chests {
                assert!(reach.contains(c), "seed {seed} {grid:?}: the chest at block {c:?} can't be reached");
            }
            secrets += s.pieces.iter().any(|p| p.blocks_of(Glyph::Illusory).next().is_some()) as i32;
        }
        assert!(secrets > 250, "most crypts hide a secret ({secrets}/300)");
    }
}
