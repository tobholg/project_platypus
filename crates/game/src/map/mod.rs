//! The world map (M): the world as you've seen it, charted as you go,
//! shared by everyone in the world and saved with it.
//!
//! - **Charting**: a few times a second, what's lit in view (the light's
//!   last solve: `light::Lit`), and what's open to the sky above the
//!   ground as the world was made, is put on the map as it is now: a map
//!   pixel a 4×4 of cells, what's there (its commonest material, or the
//!   wall behind its air). Dark caves stay unknown: you chart them by
//!   torchlight. What changes when you're away isn't seen till you're
//!   back.
//! - **Tiles**: 256×256 map pixels (1024×1024 cells), only those charted
//!   kept, saved as `map.bin` beside the world (packed; pins and where
//!   you last died too).
//! - **The screen** (`screen.rs`): M opens it full screen, the world going
//!   on behind; fog over the uncharted. In dev mode it can show the world
//!   unseen too (from the seed), and a click there takes you there.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind, MaterialId};

use crate::world::SimWorld;

mod screen;
pub use screen::MapScreen;

/// Cells a map pixel covers (a side).
pub const CELLS: i32 = 4;
/// Map pixels a tile's side.
pub const TILE: i32 = 256;
const TILE_PX: usize = (TILE * TILE) as usize;
/// Charted every so often (s): what's lit at least this much (or open to
/// the sky above the ground as made).
const CHART_EVERY: f32 = 0.25;
const LIT: f32 = 0.05;
/// A map pixel's what: its material's id, with this bit when it's the
/// wall behind open air.
pub const WALL: u16 = 1 << 15;
/// Saved every so often (s), as the world is.
const SAVE_EVERY: f32 = 60.0;

/// A tile of the map: what's at each pixel (row 0 the lowest), and which
/// have been seen.
pub struct Tile {
    pub px: Vec<u16>,
    seen: Vec<u64>,
    /// Changed since the screen last drew it.
    pub dirty: bool,
}

impl Tile {
    fn new() -> Self {
        Tile { px: vec![0; TILE_PX], seen: vec![0; TILE_PX / 64], dirty: true }
    }

    pub fn seen(&self, i: usize) -> bool {
        (self.seen[i >> 6] >> (i & 63)) & 1 == 1
    }

    fn see(&mut self, i: usize) {
        self.seen[i >> 6] |= 1 << (i & 63);
    }
}

/// The map, as charted.
#[derive(Resource, Default)]
pub struct WorldMap {
    pub tiles: HashMap<IVec2, Tile>,
    /// Pins dropped (world points).
    pub pins: Vec<Vec2>,
    /// Where you last died.
    pub death: Option<Vec2>,
    /// Round a wrapping world, its width in tiles: tiles are kept as
    /// stored (x in 0 … that), whichever lap they were charted on.
    pub laps: Option<i32>,
}

impl WorldMap {
    /// The tile a map pixel's in (as stored), and its index there.
    pub fn locate(&self, m: IVec2) -> (IVec2, usize) {
        let t = IVec2::new(m.x.div_euclid(TILE), m.y.div_euclid(TILE));
        (self.key(t), (m.y.rem_euclid(TILE) * TILE + m.x.rem_euclid(TILE)) as usize)
    }

    /// A tile as stored: round a wrapping world, x in 0 … its width.
    pub fn key(&self, t: IVec2) -> IVec2 {
        self.laps.map_or(t, |n| IVec2::new(t.x.rem_euclid(n), t.y))
    }

    /// The tile at `t`, wherever round the world that is, if charted.
    pub fn tile(&self, t: IVec2) -> Option<&Tile> {
        self.tiles.get(&self.key(t))
    }

    /// How far tile `a` is from `b` (round the world the shorter way).
    pub fn apart(&self, a: IVec2, b: IVec2) -> IVec2 {
        let d = a - b;
        self.laps.map_or(d, |n| IVec2::new((d.x + n / 2).rem_euclid(n) - n / 2, d.y))
    }

    /// The map pixel a world point's in.
    pub fn pixel(p: Vec2) -> IVec2 {
        IVec2::new((p.x / CELLS as f32).floor() as i32, (p.y / CELLS as f32).floor() as i32)
    }

    /// What's at a map pixel, if it's been seen.
    pub fn what(&self, m: IVec2) -> Option<u16> {
        let (t, i) = self.locate(m);
        self.tiles.get(&t).filter(|t| t.seen(i)).map(|t| t.px[i])
    }

    /// Whether a world point's been seen.
    pub fn seen_at(&self, p: Vec2) -> bool {
        self.what(Self::pixel(p)).is_some()
    }

    fn chart(&mut self, m: IVec2, what: u16) {
        let (t, i) = self.locate(m);
        let tile = self.tiles.entry(t).or_insert_with(Tile::new);
        if !tile.seen(i) || tile.px[i] != what {
            tile.px[i] = what;
            tile.see(i);
            tile.dirty = true;
        }
    }
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        // (Saved last in the frame, as the world is: leaving, after the
        // window's closing is heard.)
        app.init_resource::<WorldMap>().add_systems(Startup, load).add_systems(Update, (chart, died)).add_systems(Last, save.after(bevy::window::ExitSystems));
        screen::build(app);
    }
}

/// What a map pixel shows of the world now: the commonest of what's at
/// four cells of it (air, gas and fire counting for nothing), or with
/// nothing there, the wall behind it (`WALL`), or open air. None: not
/// loaded.
fn what_at(sim: &SimWorld, m: IVec2) -> Option<u16> {
    let world = &sim.world;
    let mats = world.materials();
    let (cx, cy) = (m.x * CELLS, m.y * CELLS);
    let mut found: [(u16, u8); 4] = [(0, 0); 4];
    let mut n = 0;
    for (dx, dy) in [(1, 1), (3, 1), (1, 3), (3, 3)] {
        let c = world.get(CellPos::new(cx + dx, cy + dy))?;
        if c.is_air() || matches!(mats.phys(c.material).kind, Kind::Gas | Kind::Fire) {
            continue;
        }
        let id = c.material.0;
        match found[..n].iter_mut().find(|(m, _)| *m == id) {
            Some(f) => f.1 += 1,
            None => {
                found[n] = (id, 1);
                n += 1;
            }
        }
    }
    if let Some(&(id, _)) = found[..n].iter().max_by_key(|f| f.1) {
        return Some(id);
    }
    let wall = world.get_bg(CellPos::new(cx + 2, cy + 2)).filter(|c| !c.is_air());
    Some(wall.map_or(MaterialId::AIR.0, |c| c.material.0 | WALL))
}

/// Chart what's lit in view (and what's open to the sky).
fn chart(time: Res<Time>, sim: Res<SimWorld>, mut lit: ResMut<crate::light::Lit>, mut map: ResMut<WorldMap>, mut clock: Local<f32>) {
    let laps = sim.world.wraps().map(|w| w / (TILE * CELLS));
    if map.laps != laps {
        map.laps = laps;
    }
    *clock -= time.delta_secs();
    if *clock <= 0.0 && lit.view.is_none() {
        lit.want = true;
        *clock = CHART_EVERY;
    }
    let Some(view) = lit.view.take() else { return };
    let t = view.texel;
    let (x0, y0) = (view.origin.x, view.origin.y);
    let (x1, y1) = (x0 + view.w as i32 * t, y0 + view.h as i32 * t);
    let (m0, m1) = (IVec2::new(x0.div_euclid(CELLS) + 1, y0.div_euclid(CELLS) + 1), IVec2::new(x1.div_euclid(CELLS) - 1, y1.div_euclid(CELLS) - 1));
    for mx in m0.x..=m1.x {
        let cx = mx * CELLS + 2;
        let ground = sim.generator.surface_hint(cx);
        for my in m0.y..=m1.y {
            let cy = my * CELLS + 2;
            // (Open to the sky: above the ground as it was made. Else lit:
            // here or a pixel off, so a cave's walls show by the light of
            // its air.)
            let sky = ground.is_some_and(|g| cy > g + 2);
            let lit_here = [(0, 0), (CELLS, 0), (-CELLS, 0), (0, CELLS), (0, -CELLS)].iter().filter_map(|&(dx, dy)| view.at(cx + dx, cy + dy)).fold(0.0f32, f32::max);
            if !sky && lit_here < LIT {
                continue;
            }
            if let Some(w) = what_at(&sim, IVec2::new(mx, my)) {
                map.chart(IVec2::new(mx, my), w);
            }
        }
    }
}

/// Where you last died, marked.
fn died(mut respawned: MessageReader<crate::respawn::Respawned>, mut map: ResMut<WorldMap>) {
    for r in respawned.read() {
        map.death = Some(r.died_at);
    }
}

/// Somewhere a body of `half` can stand nearest `at` (its middle): open
/// for it, ground under it; from the world as it is where it's loaded,
/// the seed's ground where not. In the open, straight down onto what's
/// under it; in rock, the nearest room, out to `reach` cells.
pub fn standing_room(sim: &SimWorld, at: Vec2, half: Vec2, reach: i32) -> Option<Vec2> {
    let mats = sim.world.materials();
    let solid = |x: i32, y: i32| match sim.world.get(CellPos::new(x, y)) {
        Some(c) => !c.is_air() && matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder),
        None => sim.generator.material_hint(x, y).is_some_and(|m| m != MaterialId::AIR && matches!(mats.phys(m).kind, Kind::Static | Kind::Powder)),
    };
    let (hw, hh) = (half.x.ceil() as i32, (half.y * 2.0).ceil() as i32);
    let fits = |x: i32, feet: i32| (-hw..=hw).step_by(2).all(|dx| (0..hh).step_by(2).all(|dy| !solid(x + dx, feet + dy))) && (-hw / 2..=hw / 2).any(|dx| solid(x + dx, feet - 1));
    let (ax, ay) = (at.x.round() as i32, at.y.round() as i32);
    // In the open: straight down, onto what's under it.
    if !solid(ax, ay) {
        for dy in 0..reach * 2 {
            if fits(ax, ay - dy) {
                return Some(Vec2::new(ax as f32 + 0.5, (ay - dy) as f32 + half.y));
            }
            if solid(ax, ay - dy) {
                break;
            }
        }
    }
    // In rock: the nearest room, rings out.
    for r in (0..=reach).step_by(2) {
        for dx in [-r, r] {
            for dy in (-r..=r).step_by(2) {
                if fits(ax + dx, ay + dy) {
                    return Some(Vec2::new((ax + dx) as f32 + 0.5, (ay + dy) as f32 + half.y));
                }
            }
        }
        for dy in [-r, r] {
            for dx in (-r..=r).step_by(2) {
                if fits(ax + dx, ay + dy) {
                    return Some(Vec2::new((ax + dx) as f32 + 0.5, (ay + dy) as f32 + half.y));
                }
            }
        }
    }
    None
}

// --- Saving: `map.bin` beside the world's files.

const MAGIC: &[u8; 4] = b"PMAP";
const VERSION: u32 = 1;

fn write_map(path: &Path, names: &[String], map: &WorldMap) -> std::io::Result<()> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(names.len() as u32).to_le_bytes());
    for n in names {
        out.extend_from_slice(&(n.len() as u16).to_le_bytes());
        out.extend_from_slice(n.as_bytes());
    }
    out.extend_from_slice(&(map.pins.len() as u32).to_le_bytes());
    for p in &map.pins {
        out.extend_from_slice(&p.x.to_le_bytes());
        out.extend_from_slice(&p.y.to_le_bytes());
    }
    let death = map.death.unwrap_or(Vec2::NAN);
    out.extend_from_slice(&death.x.to_le_bytes());
    out.extend_from_slice(&death.y.to_le_bytes());
    out.extend_from_slice(&(map.tiles.len() as u32).to_le_bytes());
    for (k, t) in &map.tiles {
        let mut raw = Vec::with_capacity(TILE_PX / 8 + TILE_PX * 2);
        for w in &t.seen {
            raw.extend_from_slice(&w.to_le_bytes());
        }
        for p in &t.px {
            raw.extend_from_slice(&p.to_le_bytes());
        }
        let packed = lz4_flex::compress_prepend_size(&raw);
        out.extend_from_slice(&k.x.to_le_bytes());
        out.extend_from_slice(&k.y.to_le_bytes());
        out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        out.extend_from_slice(&packed);
    }
    let tmp = path.with_extension("bin.tmp");
    std::fs::write(&tmp, out)?;
    std::fs::rename(tmp, path)
}

fn read_map(path: &Path, sim: &SimWorld) -> Result<WorldMap, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut at = 0usize;
    let mut take = |n: usize| -> Result<&[u8], String> {
        let s = bytes.get(at..at + n).ok_or("map.bin: cut short")?;
        at += n;
        Ok(s)
    };
    if take(4)? != MAGIC {
        return Err("map.bin: not a map".into());
    }
    let u32_ = |b: &[u8]| u32::from_le_bytes(b.try_into().expect("4 bytes"));
    let f32_ = |b: &[u8]| f32::from_le_bytes(b.try_into().expect("4 bytes"));
    if u32_(take(4)?) != VERSION {
        return Err("map.bin: another version".into());
    }
    // (Its materials by name, to the world's ids now.)
    let mats = sim.materials();
    let n = u32_(take(4)?) as usize;
    let mut remap = Vec::with_capacity(n);
    for _ in 0..n {
        let len = u16::from_le_bytes(take(2)?.try_into().expect("2 bytes")) as usize;
        let name = String::from_utf8_lossy(take(len)?).to_string();
        remap.push(mats.id(&name).map_or(MaterialId::AIR.0, |m| m.0));
    }
    let mut map = WorldMap::default();
    for _ in 0..u32_(take(4)?) {
        let (x, y) = (f32_(take(4)?), f32_(take(4)?));
        map.pins.push(Vec2::new(x, y));
    }
    let death = Vec2::new(f32_(take(4)?), f32_(take(4)?));
    map.death = (!death.x.is_nan()).then_some(death);
    for _ in 0..u32_(take(4)?) {
        let (x, y) = (i32::from_le_bytes(take(4)?.try_into().expect("4 bytes")), i32::from_le_bytes(take(4)?.try_into().expect("4 bytes")));
        let len = u32_(take(4)?) as usize;
        let raw = lz4_flex::decompress_size_prepended(take(len)?).map_err(|e| e.to_string())?;
        if raw.len() != TILE_PX / 8 + TILE_PX * 2 {
            return Err("map.bin: a tile the wrong size".into());
        }
        let mut t = Tile::new();
        for (i, w) in raw[..TILE_PX / 8].chunks_exact(8).enumerate() {
            t.seen[i] = u64::from_le_bytes(w.try_into().expect("8 bytes"));
        }
        for (i, p) in raw[TILE_PX / 8..].chunks_exact(2).enumerate() {
            let v = u16::from_le_bytes(p.try_into().expect("2 bytes"));
            let id = remap.get((v & !WALL) as usize).copied().unwrap_or(0);
            t.px[i] = id | (v & WALL);
        }
        map.tiles.insert(IVec2::new(x, y), t);
    }
    Ok(map)
}

/// The world's map, loaded with it.
fn load(slot: Option<Res<crate::save::SaveSlot>>, sim: Res<SimWorld>, mut map: ResMut<WorldMap>) {
    let Some(slot) = slot else { return };
    let path = slot.dir.join("map.bin");
    if !path.exists() {
        return;
    }
    match read_map(&path, &sim) {
        Ok(m) => {
            info!("map: {} tiles charted, {} pins", m.tiles.len(), m.pins.len());
            *map = m;
        }
        Err(e) => warn!("{e}"),
    }
}

/// Saved with the world: when it's saved (asked, leaving), and every
/// minute; written on a thread of its own (leaving, before the game goes).
#[allow(clippy::too_many_arguments)]
fn save(
    time: Res<Time<Real>>,
    slot: Option<Res<crate::save::SaveSlot>>,
    sim: Res<SimWorld>,
    map: Res<WorldMap>,
    mut now: MessageReader<crate::save::SaveNow>,
    mut exit: MessageReader<AppExit>,
    mut next: Local<Option<f32>>,
    mut writing: Local<Option<std::thread::JoinHandle<()>>>,
) {
    let next = next.get_or_insert(SAVE_EVERY);
    *next -= time.delta_secs();
    let asked = now.read().count() > 0;
    let leaving = exit.read().count() > 0;
    if !(asked || leaving || *next <= 0.0) {
        return;
    }
    *next = SAVE_EVERY;
    let Some(slot) = slot else { return };
    if slot.hold || map.tiles.is_empty() {
        return;
    }
    if let Some(w) = writing.take() {
        let _ = w.join();
    }
    let mats = sim.materials();
    let names: Vec<String> = (0..mats.len()).map(|i| mats.def(MaterialId(i as u16)).name.clone()).collect();
    // (A copy to write from: the map goes on being charted meanwhile.)
    let copy = WorldMap { tiles: map.tiles.iter().map(|(k, t)| (*k, Tile { px: t.px.clone(), seen: t.seen.clone(), dirty: false })).collect(), pins: map.pins.clone(), death: map.death, laps: map.laps };
    let path = slot.dir.join("map.bin");
    let handle = std::thread::spawn(move || {
        if let Err(e) = write_map(&path, &names, &copy) {
            error!("map: {e}");
        }
    });
    if leaving {
        let _ = handle.join();
    } else {
        *writing = Some(handle);
    }
}
