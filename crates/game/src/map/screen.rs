//! The world map's screen (M): full screen, the world going on behind it.
//! Drag to pan, the wheel to zoom (about the pointer), right-click to drop
//! a pin (or pick one up), M or Esc to close.
//!
//! - **The look**: the world as charted, in its materials' colours, with
//!   relief (what faces up catches the light, overhangs shade), water's
//!   surface lighter, what glows glowing, rock darker the deeper it is;
//!   open sky shading up from pale to deep, caves dark. The uncharted is a
//!   slow drifting fog, the charted's edges feathered into it.
//! - **On it**: you, where you last died, where you wake, the village, the
//!   places you've found (lairs and the like), your pins; the surface's
//!   biomes named along it, the depths named down its side.
//! - **Dev mode**: F shows the world unseen too, drawn from the seed in
//!   the background (a little dimmer than what you've charted); a click
//!   on the map takes you there (the nearest standing room).

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{IoTaskPool, Task, block_on, poll_once};
use bevy::window::PrimaryWindow;
use platypus_sim::{Kind, MaterialId};

use super::{CELLS, TILE, TILE_PX, WALL, WorldMap};
use crate::creatures::Kinematics;
use crate::creatures::player::LocalPlayer;
use crate::world::SimWorld;

/// Zoom steps: screen pixels a map pixel (a map pixel is 4 cells; at 4 a
/// cell's a pixel, the game's furthest out).
const SCALES: [f32; 7] = [0.0625, 0.125, 0.25, 0.5, 1.0, 2.0, 4.0];
/// Opened at this.
const OPEN_SCALE: f32 = 0.5;
/// Below this the tiles' small pictures are drawn (smoothed), above it the
/// full ones (crisp).
const SMALL_BELOW: f32 = 0.75;
/// Tiles drawn a frame at most, and no more once this long's gone on
/// them (ms; the rest the next).
const DRAW_BUDGET: usize = 6;
const DRAW_MS: f32 = 3.0;
/// The seed's tiles made at once at most (dev view).
const MAKING: usize = 6;
/// A pin picked up within this of the pointer (screen px).
const PIN_REACH: f32 = 10.0;
/// A press that moves less than this is a click, not a drag (screen px).
const CLICK: f32 = 4.0;
/// Without a window (offscreen), the screen's taken to be this big.
const SCREEN: Vec2 = Vec2::new(1512.0, 917.0);

#[derive(Resource)]
pub struct MapScreen {
    pub open: bool,
    scale: f32,
    /// The map pixel at the screen's middle.
    center: Vec2,
    /// A drag under way: where the pointer was, the middle then, whether
    /// it's moved (else a click).
    drag: Option<(Vec2, Vec2, bool)>,
    /// Dev mode: the world unseen shown too.
    pub fog_off: bool,
    shown: HashMap<IVec2, Shown>,
    /// The seed's world, tile by tile (dev view), and those being made.
    seeds: HashMap<IVec2, Vec<u16>>,
    making: HashMap<IVec2, Task<Vec<u16>>>,
    /// Its labels, made when it opens (and when pins or the like change).
    labels_made: Option<(usize, Option<Vec2>, Option<Vec2>, bool)>,
    clock: f32,
}

impl Default for MapScreen {
    fn default() -> Self {
        MapScreen { open: false, scale: OPEN_SCALE, center: Vec2::ZERO, drag: None, fog_off: false, shown: HashMap::new(), seeds: HashMap::new(), making: HashMap::new(), labels_made: None, clock: 0.0 }
    }
}

impl MapScreen {
    /// Open on a world point, at a zoom (a scenario's: as M and the wheel
    /// would).
    pub fn show(&mut self, at: Vec2, scale: f32) {
        self.open = true;
        self.center = at / CELLS as f32;
        self.scale = scale;
        self.labels_made = None;
    }
}

/// A tile on the screen: its node, its pictures (full, small), whether
/// they're up to date.
struct Shown {
    node: Entity,
    /// Its full picture, made only once it's shown close in (across the
    /// whole world, hundreds of them, a quarter of a megabyte each).
    full: Option<Handle<Image>>,
    small: Handle<Image>,
    drawn: bool,
    /// Drawn once: shown (its old picture till it's drawn again).
    ever: bool,
}

#[derive(Component)]
struct MapRoot;
#[derive(Component)]
struct MapFog;
#[derive(Component)]
struct MapTiles;
#[derive(Component)]
struct MapLabels;
#[derive(Component)]
struct MapHelp;

/// A label or marker on the map: where it is in the world, from what zoom
/// it shows, whether only once that spot's been seen; `you`: it follows
/// you.
#[derive(Component)]
struct MapLabel {
    at: Vec2,
    from_scale: f32,
    seen_only: bool,
    you: bool,
    /// A name down the side (a depth band): kept at the screen's left edge.
    side: bool,
    /// With a dot on the spot (a place, a marker); else a name centred on
    /// it.
    dot: bool,
}

/// Each material's colour on the map, its kind, whether it glows.
#[derive(Resource)]
struct Palette(Vec<([f32; 3], Kind, bool)>);

pub(super) fn build(app: &mut App) {
    app.init_resource::<MapScreen>()
        .add_systems(Startup, spawn)
        .add_systems(PreUpdate, keys.after(bevy::input::InputSystems).before(crate::pause::key))
        .add_systems(Update, (palette, mouse, seeds, (place, labels, draw).chain()).chain());
}

fn screen_size(window: &Query<&Window, With<PrimaryWindow>>) -> Vec2 {
    window.single().map_or(SCREEN, |w| Vec2::new(w.width(), w.height()))
}

// --- Setting up.

fn spawn(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let fog = images.add(fog_texture());
    commands
        .spawn((
            Name::new("World map"),
            MapRoot,
            Visibility::Hidden,
            GlobalZIndex(58),
            Interaction::default(),
            Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), overflow: Overflow::clip(), ..default() },
            BackgroundColor(Color::srgb(0.03, 0.035, 0.06)),
        ))
        .with_children(|p| {
            p.spawn((MapFog, ImageNode::new(fog).with_mode(bevy::ui::widget::NodeImageMode::Tiled { tile_x: true, tile_y: true, stretch_value: 1.0 }), Node { position_type: PositionType::Absolute, width: px(SCREEN.x + 512.0), height: px(SCREEN.y + 512.0), ..default() }));
            p.spawn((MapTiles, Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() }));
            p.spawn((MapLabels, Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() }));
            // The frame, the title, the help.
            p.spawn((Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), border: UiRect::all(px(3)), ..default() }, BorderColor::all(Color::srgba(0.72, 0.6, 0.36, 0.85))));
            p.spawn((Text::new("WORLD MAP"), TextFont { font_size: FontSize::Px(22.0), ..default() }, TextColor(Color::srgb(0.98, 0.86, 0.55)), TextShadow::default(), Node { position_type: PositionType::Absolute, top: px(12), left: percent(50), margin: UiRect::left(px(-62)), ..default() }));
            p.spawn((MapHelp, Text::new(""), TextFont { font_size: FontSize::Px(14.0), ..default() }, TextColor(Color::srgba(0.9, 0.88, 0.82, 0.85)), TextShadow::default(), Node { position_type: PositionType::Absolute, bottom: px(12), left: px(16), ..default() }));
        });
}

/// A tileable fog: soft dark clouds, slate and blue-grey.
fn fog_texture() -> Image {
    const N: usize = 256;
    let noise = |x: usize, y: usize, cell: usize, seed: u64| -> f32 {
        let g = N / cell;
        let v = |i: usize, j: usize| (platypus_sim::rng::hash(&[(i % g) as u64, (j % g) as u64, seed]) % 1000) as f32 / 1000.0;
        let (fx, fy) = (x as f32 / cell as f32, y as f32 / cell as f32);
        let (i, j) = (fx.floor() as usize, fy.floor() as usize);
        let (u, w) = (fx.fract(), fy.fract());
        let (u, w) = (u * u * (3.0 - 2.0 * u), w * w * (3.0 - 2.0 * w));
        let a = v(i, j) + (v(i + 1, j) - v(i, j)) * u;
        let b = v(i, j + 1) + (v(i + 1, j + 1) - v(i, j + 1)) * u;
        a + (b - a) * w
    };
    let mut data = Vec::with_capacity(N * N * 4);
    for y in 0..N {
        for x in 0..N {
            let n = noise(x, y, 64, 1) * 0.55 + noise(x, y, 32, 2) * 0.28 + noise(x, y, 16, 3) * 0.17;
            let k = (n - 0.25).clamp(0.0, 1.0);
            let c = [12.0 + 26.0 * k, 14.0 + 28.0 * k, 22.0 + 36.0 * k];
            data.extend_from_slice(&[c[0] as u8, c[1] as u8, c[2] as u8, 255]);
        }
    }
    let mut image = Image::new(Extent3d { width: N as u32, height: N as u32, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
    image.sampler = bevy::image::ImageSampler::linear();
    image
}

/// The materials' colours on the map (their colours, averaged), made once.
fn palette(mut commands: Commands, sim: Res<SimWorld>, have: Option<Res<Palette>>) {
    if have.is_some() {
        return;
    }
    let mats = sim.materials();
    let p = (0..mats.len())
        .map(|i| {
            let id = MaterialId(i as u16);
            let def = mats.def(id);
            let n = def.colors.len().max(1) as f32;
            let sum = def.colors.iter().fold([0.0f32; 3], |a, &(r, g, b)| [a[0] + r as f32, a[1] + g as f32, a[2] + b as f32]);
            let ph = mats.phys(id);
            ([sum[0] / n, sum[1] / n, sum[2] / n], ph.kind, ph.glow != [0; 3])
        })
        .collect();
    commands.insert_resource(Palette(p));
}

// --- Input.

/// M opens and shuts it, Esc shuts it; F (dev mode) the fog. Open, it has
/// the keyboard: the world goes on, you don't.
fn keys(mut keys: ResMut<ButtonInput<KeyCode>>, mut screen: ResMut<MapScreen>, mode: Res<crate::mode::Mode>, taken: Res<crate::dev::KeyboardTaken>, title: Res<crate::title::Title>, menu: Res<crate::pause::GameMenu>, player: Query<&Kinematics, With<LocalPlayer>>) {
    if screen.open {
        if keys.just_pressed(KeyCode::KeyM) || keys.just_pressed(KeyCode::Escape) {
            screen.open = false;
            screen.drag = None;
        } else if keys.just_pressed(KeyCode::KeyF) && mode.dev() {
            screen.fog_off = !screen.fog_off;
            for s in screen.shown.values_mut() {
                s.drawn = false;
            }
            screen.labels_made = None;
        }
        keys.reset_all();
        return;
    }
    if keys.just_pressed(KeyCode::KeyM) && !taken.0 && !title.up && !menu.open {
        screen.open = true;
        screen.scale = OPEN_SCALE;
        if let Ok(k) = player.single() {
            screen.center = k.body.pos / CELLS as f32;
        }
        screen.labels_made = None;
        keys.reset(KeyCode::KeyM);
    }
}

/// The wheel zooms about the pointer, a drag pans, a right-click drops a
/// pin (or picks one up); a click (dev mode) takes you there.
#[allow(clippy::too_many_arguments)]
fn mouse(
    mut screen: ResMut<MapScreen>,
    mut map: ResMut<WorldMap>,
    mode: Res<crate::mode::Mode>,
    buttons: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    window: Query<&Window, With<PrimaryWindow>>,
    sim: Res<SimWorld>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut wheel: Local<f32>,
) {
    if !screen.open {
        *wheel = 0.0;
        return;
    }
    let size = screen_size(&window);
    let Some(cursor) = window.single().ok().and_then(|w| w.cursor_position()) else { return };
    let to_map = |s: &MapScreen, c: Vec2| Vec2::new(s.center.x + (c.x - size.x / 2.0) / s.scale, s.center.y - (c.y - size.y / 2.0) / s.scale);
    // Zoom, keeping the map under the pointer where it is.
    *wheel += match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
    };
    let notches = wheel.trunc();
    if notches != 0.0 {
        *wheel -= notches;
        let i = SCALES.iter().position(|&s| s >= screen.scale - 1e-4).unwrap_or(SCALES.len() - 1) as i32;
        let j = (i + notches as i32).clamp(0, SCALES.len() as i32 - 1) as usize;
        let under = to_map(&screen, cursor);
        screen.scale = SCALES[j];
        let now = to_map(&screen, cursor);
        screen.center += under - now;
    }
    // Drag to pan; a click that didn't move (dev mode) takes you there.
    if buttons.just_pressed(MouseButton::Left) {
        screen.drag = Some((cursor, screen.center, false));
    }
    if let Some((from, center, moved)) = screen.drag {
        let d = cursor - from;
        let moved = moved || d.length() > CLICK;
        screen.center = if moved { center + Vec2::new(-d.x, d.y) / screen.scale } else { center };
        screen.drag = Some((from, center, moved));
        if buttons.just_released(MouseButton::Left) {
            screen.drag = None;
            if !moved && mode.dev() {
                let to = to_map(&screen, cursor) * CELLS as f32;
                if let Ok(mut k) = player.single_mut()
                    && let Some(at) = super::standing_room(&sim, to, k.body.half, 400)
                {
                    k.body.pos = at;
                    k.prev_pos = at;
                    k.body.vel = Vec2::ZERO;
                    screen.center = at / CELLS as f32;
                    screen.open = false;
                    info!("map: to ({:.0}, {:.0})", at.x, at.y);
                }
            }
        }
    }
    // A pin: picked up if there's one under the pointer, else dropped.
    if buttons.just_pressed(MouseButton::Right) {
        let s = &*screen;
        let near = map.pins.iter().position(|p| {
            let m = *p / CELLS as f32;
            Vec2::new((m.x - s.center.x) * s.scale + size.x / 2.0, (s.center.y - m.y) * s.scale + size.y / 2.0).distance(cursor) < PIN_REACH
        });
        match near {
            Some(i) => {
                map.pins.remove(i);
            }
            None => {
                let at = to_map(s, cursor) * CELLS as f32;
                map.pins.push(at);
            }
        }
        screen.labels_made = None;
    }
}

// --- The seed's world (dev view).

/// With the fog off, the seed's world made for the tiles in view, in the
/// background, a few at a time.
fn seeds(mut screen: ResMut<MapScreen>, sim: Res<SimWorld>, window: Query<&Window, With<PrimaryWindow>>) {
    // Those made: kept, their tiles (and those beside, for their edges)
    // drawn again.
    let mut done = Vec::new();
    for (k, t) in screen.making.iter_mut() {
        if let Some(v) = block_on(poll_once(t)) {
            done.push((*k, v));
        }
    }
    // (Round a wrapping world any x is in it, its tiles made as stored.)
    let laps = sim.world.wraps().map(|w| w / (TILE * CELLS));
    let key = |k: IVec2| laps.map_or(k, |n| IVec2::new(k.x.rem_euclid(n), k.y));
    for (k, v) in done {
        screen.making.remove(&k);
        screen.seeds.insert(k, v);
        // (Its picture and those beside it, wherever round the world.)
        for (sk, sh) in screen.shown.iter_mut() {
            let d = key(*sk - k + IVec2::new(1, 1));
            if (0..=2).contains(&d.x) && (-1..=1).contains(&(sk.y - k.y)) {
                sh.drawn = false;
            }
        }
    }
    if !screen.open || !screen.fog_off {
        return;
    }
    let (lo, hi) = view_tiles(&screen, screen_size(&window));
    let (b0, b1) = sim.generator.bounds();
    let (w0, w1) = (IVec2::new(b0.x * 64, b0.y * 64) / (TILE * CELLS), IVec2::new(b1.x * 64 + 63, b1.y * 64 + 63) / (TILE * CELLS));
    let (x0, x1) = if laps.is_some() { (lo.x, hi.x) } else { (lo.x.max(w0.x), hi.x.min(w1.x)) };
    let mid = (screen.center / TILE as f32).floor().as_ivec2();
    let mut want: Vec<(IVec2, IVec2)> = (lo.y.max(w0.y)..=hi.y.min(w1.y)).flat_map(|y| (x0..=x1).map(move |x| IVec2::new(x, y))).map(|k| (k, key(k))).filter(|(_, k)| !screen.seeds.contains_key(k) && !screen.making.contains_key(k)).collect();
    want.sort_by_key(|(k, _)| (*k - mid).length_squared());
    want.dedup_by_key(|(_, k)| *k);
    let want = want.into_iter().map(|(_, k)| k);
    // (On the IO pool: the light's solve has the compute pool, and the
    // seed's tiles, a tenth of a second each, held it up.)
    let pool = IoTaskPool::get();
    for k in want {
        if screen.making.len() >= MAKING {
            break;
        }
        let generator = sim.generator.clone();
        let task = pool.spawn(async move {
            let mut px = vec![0u16; TILE_PX];
            for ly in 0..TILE {
                for lx in 0..TILE {
                    let (x, y) = ((k.x * TILE + lx) * CELLS + 2, (k.y * TILE + ly) * CELLS + 2);
                    px[(ly * TILE + lx) as usize] = generator.material_hint(x, y).map_or(0, |m| m.0);
                }
            }
            px
        });
        screen.making.insert(k, task);
    }
}

/// The tiles the screen shows (lowest, highest).
fn view_tiles(screen: &MapScreen, size: Vec2) -> (IVec2, IVec2) {
    let half = size / (2.0 * screen.scale);
    let lo = ((screen.center - half) / TILE as f32).floor().as_ivec2();
    let hi = ((screen.center + half) / TILE as f32).floor().as_ivec2();
    (lo, hi)
}

// --- Showing it.

/// The tiles in view placed (and made, and hidden out of view); the root
/// shown or not; the fog drifting.
#[allow(clippy::too_many_arguments)]
fn place(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut screen: ResMut<MapScreen>,
    map: Res<WorldMap>,
    mut images: ResMut<Assets<Image>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut root: Query<&mut Visibility, With<MapRoot>>,
    layer: Query<Entity, With<MapTiles>>,
    mut fog: Query<&mut Node, (With<MapFog>, Without<MapTiles>)>,
    mut nodes: Query<(&mut Node, &mut ImageNode, &mut Visibility), (Without<MapRoot>, Without<MapFog>)>,
) {
    let want = if screen.open { Visibility::Visible } else { Visibility::Hidden };
    for mut v in &mut root {
        if *v != want {
            *v = want;
        }
    }
    if !screen.open {
        return;
    }
    screen.clock += time.delta_secs();
    let size = screen_size(&window);
    for mut n in &mut fog {
        n.left = px(-(screen.clock * 7.0).rem_euclid(256.0).round());
        n.top = px(-(screen.clock * 3.0).rem_euclid(256.0).round());
        let (w, h) = (px(size.x + 512.0), px(size.y + 512.0));
        if n.width != w {
            n.width = w;
            n.height = h;
        }
    }
    let Ok(layer) = layer.single() else { return };
    let (lo, hi) = view_tiles(&screen, size);
    let fog_off = screen.fog_off;
    let small = screen.scale < SMALL_BELOW;
    let (scale, center) = (screen.scale, screen.center);
    let mut in_view = Vec::new();
    for ty in lo.y..=hi.y {
        for tx in lo.x..=hi.x {
            let k = IVec2::new(tx, ty);
            if !(map.tile(k).is_some() || (fog_off && screen.seeds.contains_key(&map.key(k)))) {
                continue;
            }
            in_view.push(k);
            if !screen.shown.contains_key(&k) {
                let blank = |n: u32, smooth: bool| {
                    let mut im = Image::new(Extent3d { width: n, height: n, depth_or_array_layers: 1 }, TextureDimension::D2, vec![0; (n * n * 4) as usize], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
                    im.sampler = if smooth { bevy::image::ImageSampler::linear() } else { bevy::image::ImageSampler::nearest() };
                    im
                };
                let smallim = images.add(blank(TILE as u32 / 4, true));
                let node = commands.spawn((ImageNode::new(smallim.clone()), Node { position_type: PositionType::Absolute, ..default() }, Visibility::Hidden)).id();
                commands.entity(layer).add_child(node);
                screen.shown.insert(k, Shown { node, full: None, small: smallim, drawn: false, ever: false });
            }
            // (Close in: its full picture, made now and drawn.)
            if !small
                && let Some(sh) = screen.shown.get_mut(&k)
                && sh.full.is_none()
            {
                let mut im = Image::new(Extent3d { width: TILE as u32, height: TILE as u32, depth_or_array_layers: 1 }, TextureDimension::D2, vec![0; TILE_PX * 4], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
                im.sampler = bevy::image::ImageSampler::nearest();
                sh.full = Some(images.add(im));
                sh.drawn = false;
            }
        }
    }
    for (k, s) in &screen.shown {
        let Ok((mut n, mut im, mut v)) = nodes.get_mut(s.node) else { continue };
        let visible = in_view.contains(k) && s.ever;
        let want = if visible { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want {
            *v = want;
        }
        if !visible {
            continue;
        }
        let (left, top) = ((k.x * TILE) as f32 - center.x, center.y - ((k.y + 1) * TILE) as f32);
        // (A pixel over each way, so tiles meet without a seam. Set only
        // when it moved: a node touched is laid out again, and far out
        // there are a thousand of them.)
        let (l, t, side) = (px((left * scale + size.x / 2.0).floor()), px((top * scale + size.y / 2.0).floor()), px((TILE as f32 * scale).ceil() + 1.0));
        if n.left != l || n.top != t || n.width != side {
            n.left = l;
            n.top = t;
            n.width = side;
            n.height = side;
        }
        // (Close in, its full picture once it's drawn; till then, and far
        // out, its small one.)
        let pic = match &s.full {
            Some(f) if !small && s.drawn => f,
            _ => &s.small,
        };
        if im.image != *pic {
            im.image = pic.clone();
        }
    }
}

/// The tiles in view that changed (or were never drawn), drawn: a few a
/// frame, the nearest the middle first.
fn draw(mut screen: ResMut<MapScreen>, mut map: ResMut<WorldMap>, palette: Option<Res<Palette>>, sim: Res<SimWorld>, mut images: ResMut<Assets<Image>>, window: Query<&Window, With<PrimaryWindow>>) {
    let Some(palette) = palette else { return };
    if !screen.open {
        return;
    }
    // (Charted since: drawn again.)
    let changed: Vec<IVec2> = map.tiles.iter().filter(|(_, t)| t.dirty).map(|(k, _)| *k).collect();
    for k in changed {
        if let Some(t) = map.tiles.get_mut(&k) {
            t.dirty = false;
        }
        // (Its picture, and those beside it for their edges: wherever
        // round the world they're shown.)
        for (sk, s) in screen.shown.iter_mut() {
            let d = map.apart(*sk, k);
            if d.x.abs() <= 1 && d.y.abs() <= 1 {
                s.drawn = false;
            }
        }
    }
    let (lo, hi) = view_tiles(&screen, screen_size(&window));
    let mid = (screen.center / TILE as f32).floor().as_ivec2();
    let mut todo: Vec<IVec2> = screen.shown.iter().filter(|(k, s)| !s.drawn && k.x >= lo.x && k.x <= hi.x && k.y >= lo.y && k.y <= hi.y).map(|(k, _)| *k).collect();
    todo.sort_by_key(|k| (*k - mid).length_squared());
    // (A few milliseconds' worth a frame, one at least.)
    let started = std::time::Instant::now();
    for k in todo.into_iter().take(DRAW_BUDGET) {
        if started.elapsed().as_secs_f32() * 1e3 > DRAW_MS {
            break;
        }
        let (full, small) = picture(k, &map, &screen, &palette, &sim);
        let s = screen.shown.get_mut(&k).expect("shown");
        if let Some(h) = &s.full
            && let Some(mut im) = images.get_mut(h)
        {
            im.data = Some(full);
        }
        if let Some(mut im) = images.get_mut(&s.small) {
            im.data = Some(small);
        }
        s.drawn = true;
        s.ever = true;
    }
}

/// A tile's pictures: full (a pixel a map pixel) and small (a quarter).
fn picture(k: IVec2, map: &WorldMap, screen: &MapScreen, palette: &Palette, sim: &SimWorld) -> (Vec<u8>, Vec<u8>) {
    const PAD: i32 = 2;
    let p = (TILE + 2 * PAD) as usize;
    const NONE: u32 = u32::MAX;
    // What's at each pixel of it and two round it: charted, or (the fog
    // off) the seed's; and whether it's been charted.
    let mut what = vec![NONE; p * p];
    let mut charted = vec![false; p * p];
    let seed_at = |m: IVec2| -> Option<u16> {
        if !screen.fog_off {
            return None;
        }
        let (t, i) = map.locate(m);
        screen.seeds.get(&t).map(|v| v[i])
    };
    // (Its own pixels straight from it; only its border looked up.)
    let (own, own_seed) = (map.tile(k), if screen.fog_off { screen.seeds.get(&map.key(k)) } else { None });
    for ly in -PAD..TILE + PAD {
        for lx in -PAD..TILE + PAD {
            let i = ((ly + PAD) as usize) * p + (lx + PAD) as usize;
            if (0..TILE).contains(&lx) && (0..TILE).contains(&ly) {
                let j = (ly * TILE + lx) as usize;
                if let Some(t) = own.filter(|t| t.seen(j)) {
                    what[i] = t.px[j] as u32;
                    charted[i] = true;
                } else if let Some(sd) = own_seed {
                    what[i] = sd[j] as u32;
                }
                continue;
            }
            let m = IVec2::new(k.x * TILE + lx, k.y * TILE + ly);
            match map.what(m) {
                Some(w) => {
                    what[i] = w as u32;
                    charted[i] = true;
                }
                None => {
                    if let Some(w) = seed_at(m) {
                        what[i] = w as u32;
                    }
                }
            }
        }
    }
    let open = |w: u32| w == NONE || (w as u16 & WALL) != 0 || w as u16 == MaterialId::AIR.0 || matches!(palette.0.get(w as usize).map(|c| c.1), Some(Kind::Gas | Kind::Fire));
    let ground: Vec<Option<i32>> = (0..TILE).map(|lx| sim.generator.surface_hint((k.x * TILE + lx) * CELLS + 2)).collect();
    let sky_from = sim.generator.spawn_point().y;
    let mut full = vec![0u8; TILE_PX * 4];
    for ly in 0..TILE {
        for lx in 0..TILE {
            let i = ((ly + PAD) as usize) * p + (lx + PAD) as usize;
            let w = what[i];
            if w == NONE {
                continue;
            }
            let cy = (k.y * TILE + ly) * CELLS + 2;
            let g = ground[lx as usize].unwrap_or(cy);
            let id = (w as u16 & !WALL) as usize;
            let (base, kind, glows) = palette.0.get(id).copied().unwrap_or(([0.0; 3], Kind::Static, false));
            let under_ground = cy < g - 3;
            let mut c = if w as u16 & WALL != 0 {
                // The wall behind open air: its colour, dim.
                [base[0] * 0.38 + 6.0, base[1] * 0.38 + 5.0, base[2] * 0.38 + 7.0]
            } else if id == MaterialId::AIR.0 as usize || matches!(kind, Kind::Gas | Kind::Fire) {
                if under_ground {
                    [16.0, 13.0, 19.0]
                } else {
                    // Sky: pale low down, deeper higher up (by height, not
                    // over its own ground: over the mountains it streaked).
                    let t = ((cy - sky_from) as f32 / 1600.0).clamp(0.0, 1.0);
                    [170.0 + (66.0 - 170.0) * t, 200.0 + (100.0 - 200.0) * t, 232.0 + (166.0 - 232.0) * t]
                }
            } else {
                let up = what[i + p];
                let down = what[i - p];
                let mut k = 1.0;
                match kind {
                    Kind::Liquid => {
                        if open(up) {
                            k = 1.28;
                        }
                    }
                    _ => {
                        // Relief: lit from above, overhangs in shade, the
                        // sides a little dim; deeper rock darker.
                        if open(up) {
                            k *= 1.24;
                        } else if open(what[i + 2 * p]) {
                            k *= 1.08;
                        }
                        if open(down) {
                            k *= 0.7;
                        }
                        if open(what[i - 1]) || open(what[i + 1]) {
                            k *= 0.92;
                        }
                        if under_ground {
                            k *= 1.0 - ((g - cy) as f32 / 5000.0).clamp(0.0, 0.35);
                        }
                    }
                }
                if glows {
                    k = k.max(1.0) * 1.35;
                }
                [base[0] * k, base[1] * k, base[2] * k]
            };
            // (The seed's world, never charted (dev view): a little dimmer.)
            if !charted[i] {
                c = [c[0] * 0.72, c[1] * 0.72, c[2] * 0.75];
            }
            // The fog's edge, feathered: the more of what's round it is
            // uncharted, the more the fog shows through.
            let mut alpha = 255.0;
            if !screen.fog_off {
                let mut blank = 0;
                for dy in -PAD..=PAD {
                    for dx in -PAD..=PAD {
                        if what[(i as i32 + dy * p as i32 + dx) as usize] == NONE {
                            blank += 1;
                        }
                    }
                }
                alpha = (255.0 * (1.0 - blank as f32 / 25.0 * 0.95)).max(40.0);
            }
            let o = (((TILE - 1 - ly) * TILE + lx) * 4) as usize;
            full[o..o + 4].copy_from_slice(&[c[0].clamp(0.0, 255.0) as u8, c[1].clamp(0.0, 255.0) as u8, c[2].clamp(0.0, 255.0) as u8, alpha as u8]);
        }
    }
    // The small picture: four by four, averaged (what's see-through
    // counting for less).
    let s = (TILE / 4) as usize;
    let mut small = vec![0u8; s * s * 4];
    for y in 0..s {
        for x in 0..s {
            let mut acc = [0u32; 4];
            for dy in 0..4 {
                for dx in 0..4 {
                    let o = ((y * 4 + dy) * TILE as usize + x * 4 + dx) * 4;
                    let a = full[o + 3] as u32;
                    for ch in 0..3 {
                        acc[ch] += full[o + ch] as u32 * a;
                    }
                    acc[3] += a;
                }
            }
            let o = (y * s + x) * 4;
            if acc[3] > 0 {
                for ch in 0..3 {
                    small[o + ch] = (acc[ch] / acc[3]) as u8;
                }
                small[o + 3] = (acc[3] / 16) as u8;
            }
        }
    }
    (full, small)
}

/// Labels and markers: made when the map opens (or what they show
/// changes), placed each frame (shown from their zoom, only once seen).
#[allow(clippy::too_many_arguments)]
fn labels(
    mut commands: Commands,
    mut screen: ResMut<MapScreen>,
    map: Res<WorldMap>,
    sim: Res<SimWorld>,
    mode: Res<crate::mode::Mode>,
    checkpoint: Option<Res<crate::respawn::Checkpoint>>,
    window: Query<&Window, With<PrimaryWindow>>,
    layer: Query<Entity, With<MapLabels>>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut placed: Query<(&MapLabel, &mut Node, &mut Visibility, &ComputedNode)>,
    mut help: Query<&mut Text, With<MapHelp>>,
) {
    if !screen.open {
        return;
    }
    let Ok(layer) = layer.single() else { return };
    let wake = checkpoint.as_ref().and_then(|c| c.0);
    let key = (map.pins.len(), map.death, wake, screen.fog_off);
    if screen.labels_made != Some(key) {
        screen.labels_made = Some(key);
        commands.entity(layer).despawn_children();
        let g = &sim.generator;
        let mut put = |at: Vec2, text: String, color: Color, size: f32, dot: bool, from_scale: f32, seen_only: bool, you: bool, side: bool| {
            let e = commands
                .spawn((MapLabel { at, from_scale, seen_only, you, side, dot }, Node { position_type: PositionType::Absolute, flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(4), ..default() }, Visibility::Hidden))
                .with_children(|p| {
                    if dot {
                        p.spawn((Node { width: px(9), height: px(9), border: UiRect::all(px(2)), ..default() }, BackgroundColor(color), BorderColor::all(Color::srgb(0.05, 0.04, 0.06))));
                    }
                    if !text.is_empty() {
                        p.spawn((Text::new(text), TextFont { font_size: FontSize::Px(size), ..default() }, TextColor(if dot { Color::srgb(0.95, 0.93, 0.88) } else { color }), TextShadow::default()));
                    }
                })
                .id();
            commands.entity(layer).add_child(e);
        };
        let title = |s: &str| s.split(' ').map(|w| w[..1].to_uppercase() + &w[1..]).collect::<Vec<_>>().join(" ");
        // The biomes named along the surface (a run of one, at its middle,
        // over its ground).
        let (b0, b1) = g.bounds();
        let (x0, x1) = (b0.x * 64, b1.x * 64 + 63);
        let mut runs: Vec<(i32, i32, &'static str)> = Vec::new();
        for x in (x0..=x1).step_by(64) {
            let Some(b) = g.biome_hint(x) else { continue };
            match runs.last_mut() {
                Some(r) if r.2 == b => r.1 = x,
                _ => runs.push((x, x, b)),
            }
        }
        for (a, b, name) in runs {
            let mid = (a + b) / 2;
            if b - a < 600 {
                continue;
            }
            let y = g.surface_hint(mid).unwrap_or(0) + 160;
            put(Vec2::new(mid as f32, y as f32), title(name), Color::srgba(0.98, 0.94, 0.82, 0.9), 18.0, false, 0.0, true, false, false);
        }
        // The depths named down the side, at the middle of each band.
        let (y0, y1) = (b0.y * 64, b1.y * 64 + 63);
        let mut bands: Vec<(i32, i32, &'static str)> = Vec::new();
        for y in (y0..=y1).step_by(16) {
            let Some(b) = g.band_hint(y) else { continue };
            match bands.last_mut() {
                Some(r) if r.2 == b => r.1 = y,
                _ => bands.push((y, y, b)),
            }
        }
        for (a, b, name) in bands {
            let label = match name {
                "peaks" => "The Peaks",
                "deep" => "The Deep",
                other => &title(other),
            };
            put(Vec2::new(0.0, ((a + b) / 2) as f32), label.to_uppercase(), Color::srgba(0.85, 0.75, 0.52, 0.75), 13.0, false, 0.0, false, false, true);
        }
        // Places: the village, what's been found.
        if let Some((lo, hi)) = g.village() {
            put(Vec2::new((lo.x + hi.x) as f32 / 2.0, hi.y as f32), "Village".into(), Color::srgb(0.98, 0.8, 0.35), 15.0, true, 0.0, true, false, false);
        }
        for (at, name) in g.landmarks() {
            // (Close in only: across the world there are hundreds.)
            put(Vec2::new(at.x as f32, at.y as f32), title(&name), Color::srgb(0.72, 0.7, 0.66), 13.0, true, 1.0, true, false, false);
        }
        // Yours: where you wake, where you died, your pins, you.
        if let Some(at) = wake {
            put(at, "Respawn".into(), Color::srgb(0.45, 0.85, 0.95), 14.0, true, 0.0, false, false, false);
        }
        if let Some(at) = map.death {
            put(at, "Died here".into(), Color::srgb(0.95, 0.25, 0.22), 14.0, true, 0.0, false, false, false);
        }
        for &at in &map.pins {
            put(at, String::new(), Color::srgb(1.0, 0.55, 0.15), 14.0, true, 0.0, false, false, false);
        }
        put(Vec2::ZERO, "You".into(), Color::WHITE, 15.0, true, 0.0, false, true, false);
        let mut h = "Drag to pan   |   wheel to zoom   |   right-click: pin   |   M or Esc: close".to_string();
        if mode.dev() {
            h += &format!("   |   F: fog {}   |   click: go there", if screen.fog_off { "off" } else { "on" });
        }
        for mut t in &mut help {
            t.0 = h.clone();
        }
    }
    let size = screen_size(&window);
    let you = player.single().map(|k| k.body.pos).unwrap_or_default();
    for (l, mut n, mut v, computed) in &mut placed {
        let at = if l.you { you } else { l.at };
        let m = at / CELLS as f32;
        let mut x = (m.x - screen.center.x) * screen.scale + size.x / 2.0;
        let y = (screen.center.y - m.y) * screen.scale + size.y / 2.0;
        if l.side {
            x = 14.0;
        }
        let shown = screen.scale >= l.from_scale && (!l.seen_only || screen.fog_off || map.seen_at(at)) && x > -200.0 && x < size.x + 200.0 && y > -40.0 && y < size.y + 40.0;
        let want = if shown { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want {
            *v = want;
        }
        // (Its dot on the spot; a name without one centred on it. Set only
        // when it moved.)
        let w = computed.size().x * computed.inverse_scale_factor();
        let (left, top) = (px((if l.side { x } else if l.dot { x - 4.5 } else { x - w / 2.0 }).round()), px((y - 8.0).round()));
        if n.left != left || n.top != top {
            n.left = left;
            n.top = top;
        }
    }
}
