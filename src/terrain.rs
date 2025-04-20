//! world generation, streaming sprites, digging & helpers
//!
//! Fast‑path optimisations currently enabled
//! -----------------------------------------
//! 1. “Stripe differencing” – only the rows / columns that actually enter or
//!    leave the active window are touched (O(w+h) work per tile‑crossing).
//! 2. **Sprite pooling**   – tiles that scroll out of view are not despawned;
//!    their entities are pushed into `Terrain::free_sprites` and recycled for
//!    incoming tiles. This eliminates Bevy’s archetype churn and cuts CPU
//!    time by an order of magnitude.
//!
//! Compatible with **Bevy 0.15**

use bevy::input::ButtonInput;
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::Rng;
use std::collections::VecDeque;
use bevy::math::Mat2;          // 2×2 rotation matrix (Bevy re‑export)

use crate::components::*;
use crate::constants::*;

/* ===========================================================
   helpers (row‑0 = top)
   =========================================================== */
#[inline]
pub fn tile_to_world_y(terrain_h: usize, tile_y: usize) -> f32 {
    (terrain_h as f32 - 1. - tile_y as f32) * TILE_SIZE
}
#[inline]
pub fn world_to_tile_y(terrain_h: usize, world_y: f32) -> i32 {
    (terrain_h as f32 - 1. - (world_y / TILE_SIZE).floor()) as i32
}

/* ===========================================================
   tile data
   =========================================================== */
#[derive(Clone, Copy, PartialEq)]
pub enum TileKind {
    Air,
    Sky,
    Grass,   // NEW – surface layer
    Dirt,
    Stone,
    Obsidian,
    Snow,
}

#[derive(Clone, Copy)]
pub struct Tile {
    pub kind:     TileKind,
    pub visible:  bool,
    pub explored: bool,
    pub mine_time:  f32,
    pub base_rgb:  Vec3,
}

/* ===========================================================
   resources
   =========================================================== */
#[derive(Resource)]
pub struct Terrain {
    pub tiles:           Vec<Vec<Tile>>,
    pub sprite_entities: Vec<Option<Entity>>,
    pub changed_tiles:   VecDeque<(usize, usize)>,
    pub free_sprites:    Vec<Entity>,          // sprite pool
    pub width:           usize,
    pub height:          usize,
    pub height_map:      Vec<usize>,
    pub color_noise:     Perlin,
}

impl Terrain {
    #[inline(always)]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
}

/* sliding active rectangle ------------------------------------------------ */
#[derive(Resource, Copy, Clone, PartialEq, Eq, Debug)]
pub struct ActiveRect {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

/* rectangle from previous frame (for early‑out) --------------------------- */
#[derive(Resource, Default)]
pub struct LastRect(pub Option<ActiveRect>);

/* ===========================================================
   generation parameters & knobs
   =========================================================== */
const MIN_CAVE_DEPTH: usize = 8;
const BACKGROUND_BROWN: Vec3 = Vec3::new(0.20, 0.10, 0.05);
const EXPLORED_BRIGHTNESS: f32 = 0.25;

/* tweakables ------------------------------------------------------------- */
const OBSIDIAN_START_FRAC: f32 = 0.80;   // bottom 20 % of map is obsidian

/* rift (vertical chasm) parameters */
const RIFT_FREQ:   f64 = 0.018;
const RIFT_THRESH: f64 = 0.75;

/* layer‑leak probabilities */
const DIRT_TO_STONE:   f32 = 0.1;
const STONE_TO_OBSID:  f32 = 0.05;

/* surface grass ratio */
const GRASS_RATIO: f32 = 0.85;

/* ===========================================================
   generate world + player
   =========================================================== */
pub fn generate_world_and_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    /* --- sprite sheet ---------------------------------------------------- */
    let sheet = asset_server.load("textures/player_sheet.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::new(100, 100), 6, 1, None, None);
    let layout_handle = atlas_layouts.add(layout);

    /* --- dimensions ------------------------------------------------------ */
    let w = CHUNK_WIDTH  * NUM_CHUNKS_X;
    let h = CHUNK_HEIGHT * NUM_CHUNKS_Y;

    /* --- surface height map --------------------------------------------- */
    let mut height_map = vec![0usize; w];
    let noise_hills  = Perlin::new(rand::thread_rng().gen());
    let noise_cliffs = Perlin::new(rand::thread_rng().gen());

    let base = h as f32 * 0.35;
    let amp_low  =  5.0;
    let amp_high = 12.0;

    let cliff_freq      = 0.12;
    let cliff_thresh    = 0.85;
    let cliff_strength  = 18.0;

    for x in 0..w {
        let n = noise_hills.get([x as f64 * 0.01, 0.0]);
        let mut elev = if n >= 0.0 {
            base - n as f32 * amp_high
        } else {
            base - n as f32 * amp_low
        };

        let cliff_sample = noise_cliffs.get([x as f64 * cliff_freq, 100.0]);
        if cliff_sample.abs() > cliff_thresh {
            elev -= cliff_sample.signum() as f32 * cliff_strength;
        }
        height_map[x] = elev.clamp(4.0, (h - 10) as f32) as usize;
    }

    /* --- alloc tile grid ------------------------------------------------- */
    let mut tiles = vec![
        vec![
            Tile {
                kind:     TileKind::Air,
                visible:  false,
                explored: false,
                mine_time: 0.0,
                base_rgb:  BACKGROUND_BROWN,
            };
            w
        ];
        h
    ];
    let sprite_entities = vec![None; w * h];

    /* noises -------------------------------------------------------------- */
    let noise_rift = Perlin::new(rand::thread_rng().gen());
    let color_noise = Perlin::new(rand::thread_rng().gen());

    let mut rng = rand::thread_rng();

    /* ========== column‑wise generation ================================== */
    for x in 0..w {
        let surface = height_map[x];

        /* sky tiles ------------------------------------------------------- */
        for y in 0..surface {
            tiles[y][x].kind = TileKind::Sky;
            tiles[y][x].mine_time = 0.0;
        }

        /* pre‑compute rift value for column ------------------------------ */
        let rift_val = noise_rift.get([x as f64 * RIFT_FREQ, 0.0]);

        /* ground tiles ---------------------------------------------------- */
        for y in surface..h {
            let depth = y - surface;
            let mut kind = if depth < MIN_CAVE_DEPTH {
                if depth > h / 4 { TileKind::Stone } else { TileKind::Dirt }
            } else {
                // Keep the rift feature, but drop the old noise‑carve logic.
                if rift_val > RIFT_THRESH && depth > 3 {
                    TileKind::Air
                } else if y >= (h as f32 * OBSIDIAN_START_FRAC) as usize {
                    TileKind::Obsidian
                } else if depth > h / 4 {
                    TileKind::Stone
                } else {
                    TileKind::Dirt
                }
            };

            /* surface: mostly grass ------------------------------------ */
            if depth == 0 {
                kind = if rng.gen::<f32>() < GRASS_RATIO {
                    TileKind::Grass
                } else {
                    TileKind::Dirt
                };
            } else {
                /* probabilistic lower‑layer clusters -------------------- */
                match kind {
                    TileKind::Dirt if rng.gen::<f32>() < DIRT_TO_STONE =>
                        kind = TileKind::Stone,
                    TileKind::Stone if rng.gen::<f32>() < STONE_TO_OBSID =>
                        kind = TileKind::Obsidian,
                    _ => {}
                }
            }

            /* assign mine time ----------------------------------------- */
            let (kind, mine_time) = match kind {
                TileKind::Grass     => (TileKind::Grass,    0.10),
                TileKind::Snow     => (TileKind::Grass,    0.10),
                TileKind::Dirt      => (TileKind::Dirt,     0.25),
                TileKind::Stone     => (TileKind::Stone,    0.50),
                TileKind::Obsidian  => (TileKind::Obsidian, 1.00),
                TileKind::Air | TileKind::Sky => (kind, 0.0),
            };
            tiles[y][x].kind = kind;
            tiles[y][x].mine_time = mine_time;

            // ---------- per‑tile tint (discrete steps) ----------
            use crate::constants::{
                COLOR_NOISE_SCALE, COLOR_VARIATION_LEVELS, COLOR_VARIATION_STRENGTH,
            };

            let raw = color_noise.get([
                x as f64 * COLOR_NOISE_SCALE,
                y as f64 * COLOR_NOISE_SCALE,
            ]) as f32;

            let step = (((raw + 1.0) * 0.5) * COLOR_VARIATION_LEVELS as f32)
                .floor()
                .clamp(0.0, (COLOR_VARIATION_LEVELS - 1) as f32);
            let norm   = step / (COLOR_VARIATION_LEVELS as f32 - 1.0) * 2.0 - 1.0;
            let factor = 1.0 + norm * COLOR_VARIATION_STRENGTH;

            tiles[y][x].base_rgb = match kind {
                TileKind::Grass    => Vec3::new(0.13, 0.70, 0.08) * factor,
                TileKind::Snow     => Vec3::new(0.95, 0.95, 0.95) * factor,
                TileKind::Dirt     => Vec3::new(0.55, 0.27, 0.07) * factor,
                TileKind::Stone    => Vec3::new(0.50, 0.50, 0.50) * factor,
                TileKind::Obsidian => Vec3::new(0.20, 0.05, 0.35) * factor,
                TileKind::Air      => BACKGROUND_BROWN            * factor,
                TileKind::Sky      => Vec3::ZERO, // unused
            };
        }
    }

    generate_mountains(&mut tiles, &height_map, w, h, w / 2);

    /* ──────────────────── Sky islands (robust) ────────────────── */
    {
        /* tunables --------------------------------------------------------- */
        const ISLAND_MIN_RADIUS : usize = 80;
        const ISLAND_RADIUS_MAX : usize = 128;
        const ISLAND_Y_SCALE    : f32   = 0.50;   // shallower underside
        const ISLAND_SURF_WAVES : f64   = 0.06;   // grass‑line bumpiness
        const ISLAND_GAP        : i32   = 10;     // empty tiles between islands

        /* placement bookkeeping ------------------------------------------- */
        #[derive(Clone, Copy)]
        struct Rect { min_x: i32, max_x: i32, min_y: i32, max_y: i32 }
        let mut placed : Vec<Rect> = Vec::new();

        /* realistic island count for this map width ----------------------- */
        let min_footprint  = (ISLAND_MIN_RADIUS as i32 * 2 + ISLAND_GAP) as usize;
        let target_islands = (w / min_footprint).clamp(1, 32);

        const MAX_SEARCH: usize = 3_000;          // tries per island before giving up

        let mut rng       = rand::thread_rng();
        let surf_noise    = Perlin::new(rng.gen());
        let edge_noise    = Perlin::new(rng.gen());
        let cave_noise    = Perlin::new(rng.gen());

        /* ───────────── attempt to place up to `target_islands` islands ───────────── */
        'outer: for _ in 0..target_islands {
            /* (1) pick a centre that doesn’t overlap any previous island ---------- */
            let (cx, cy, rx, ry_bottom, ry_top, rect) = {
                let mut tries = 0;
                loop {
                    tries += 1;
                    if tries > MAX_SEARCH {
                        eprintln!(
                            "⚠️  sky‑island search aborted after {MAX_SEARCH} tries \
                            ({} islands placed so far)",
                            placed.len()
                        );
                        continue 'outer;                       // skip this island
                    }

                    let rx = rng.gen_range(ISLAND_MIN_RADIUS..=ISLAND_RADIUS_MAX) as f32;
                    let ry_bottom = rx * ISLAND_Y_SCALE;
                    let ry_top    = (rx * 0.30).max(8.0);

                    let cx = rng.gen_range(rx as i32 + 4 .. w as i32 - rx as i32 - 4);
                    let cy = rng.gen_range(3 .. height_map[cx as usize] as i32 / 2);

                    let rect = Rect {
                        min_x: (cx as f32 - rx - ISLAND_GAP as f32) as i32,
                        max_x: (cx as f32 + rx + ISLAND_GAP as f32) as i32,
                        min_y: cy - 2,
                        max_y: cy + ry_bottom as i32 + 2,
                    };
                    if placed.iter().all(|r|
                        rect.max_x < r.min_x || rect.min_x > r.max_x ||
                        rect.max_y < r.min_y || rect.min_y > r.max_y
                    ) {
                        break (cx, cy, rx, ry_bottom, ry_top, rect);
                    }
                }
            };
            placed.push(rect);

            /* (2) ───── carve the island (code identical to old version) ───── */

            /* 2‑a  solid ellipsoid with wavy grass line ---------------------- */
            for dx in -(rx as i32)..=(rx as i32) {
                let nx = dx as f32 / rx;
                if nx.abs() > 1.0 { continue; }

                let x  = cx + dx;
                if x < 0 || x >= w as i32 { continue; }
                let ux = x as usize;

                /* ±2‑tile surface undulation */
                let y_top = cy + (surf_noise.get([x as f64 * ISLAND_SURF_WAVES, 0.0]) * 2.0) as i32;

                /* grass + 2 dirt tiles */
                for (dy, kind) in [(0, TileKind::Grass), (1, TileKind::Dirt), (2, TileKind::Dirt)] {
                    let y = y_top + dy;
                    if y >= 0 && y < h as i32 {
                        let uy = y as usize;
                        tiles[uy][ux].kind      = kind;
                        tiles[uy][ux].mine_time = if kind == TileKind::Grass { 0.20 } else { 0.25 };
                    }
                }

                /* stone/dirt underside */
                let taper = (1.0 - nx * nx).sqrt();
                let depth = (ry_bottom * taper) as i32;
                for d in 3..=depth {
                    let y = y_top + d;
                    if y >= h as i32 { break; }
                    let uy = y as usize;

                    /* jitter edge without punching holes */
                    let e = edge_noise.get([x as f64 * 0.22, y as f64 * 0.22]) as f32;
                    if d == depth && e < -0.15 { continue; }

                    let kind = if d < 7 { TileKind::Dirt } else { TileKind::Stone };
                    let t    = if kind == TileKind::Dirt { 0.25 } else { 0.50 };
                    tiles[uy][ux].kind      = kind;
                    tiles[uy][ux].mine_time = t;
                }
            }

            /* 2‑b  branching cavern system (unchanged) ------------------------ */
            {
                /* walker parameters – unchanged -------------------------------- */
                const SKY_WALKERS_MIN:  u8  = 1;
                const SKY_WALKERS_MAX:  u8  = 2;
                const SKY_STEPS_MIN:    u16 = 150;
                const SKY_STEPS_MAX:    u16 = 260;
                const SKY_TURN_CHANCE:  f32 = 0.15;
                const SKY_TUNNEL_R_MIN: i32 = 1;
                const SKY_TUNNEL_R_MAX: i32 = 2;
                const SKY_ROOM_R_MIN:   i32 = 4;
                const SKY_ROOM_R_MAX:   i32 = 6;
                const SKY_ROOM_RATE:    f32 = 0.07;
                const SHELL_BUFFER:     i32 = 3;

                let mut walkers: Vec<(Vec2, Vec2)> = {
                    let mut v = Vec::new();
                    let seeds = rng.gen_range(SKY_WALKERS_MIN..=SKY_WALKERS_MAX);
                    for _ in 0..seeds {
                        let dx  = rng.gen_range(-(rx * 0.30) as i32 ..= (rx * 0.30) as i32);
                        let dy  = rng.gen_range(ry_top as i32 + 4 .. (ry_bottom as i32 - 6));
                        let pos = Vec2::new((cx + dx) as f32, (cy + dy) as f32);
                        let dir = Vec2::new(
                            rng.gen_range(-1.0..1.0),
                            rng.gen_range(-0.15..0.15),
                        ).normalize();
                        v.push((pos, dir));
                    }
                    v
                };

                for (mut pos, mut dir) in walkers.drain(..) {
                    let steps = rng.gen_range(SKY_STEPS_MIN..=SKY_STEPS_MAX);
                    for _ in 0..steps {
                        /* carve tunnel or room */
                        let r = if rng.gen::<f32>() < SKY_ROOM_RATE {
                            rng.gen_range(SKY_ROOM_R_MIN..=SKY_ROOM_R_MAX)
                        } else {
                            rng.gen_range(SKY_TUNNEL_R_MIN..=SKY_TUNNEL_R_MAX)
                        };
                        carve_disc(&mut tiles, w, h, pos.x as i32, pos.y as i32, r);

                        /* maybe turn */
                        if rng.gen::<f32>() < SKY_TURN_CHANCE {
                            let ang = rng.gen_range(-0.9..0.9);
                            dir = (Mat2::from_angle(ang) * dir).normalize();
                        }

                        /* step */
                        pos += dir;

                        /* bounce if we hit the shell */
                        let rel     = Vec2::new(pos.x - cx as f32, pos.y - cy as f32);
                        let ellipse = Vec2::new(rel.x / rx, rel.y / ry_bottom);
                        if ellipse.length_squared() > 0.80
                            || (pos.y - cy as f32) < SHELL_BUFFER as f32
                            || (cy as f32 + ry_bottom) - pos.y < SHELL_BUFFER as f32
                        {
                            dir = -dir;
                            pos += dir * 2.0;
                        }
                    }
                }
            }
        }
    }

    /* ──────────────────── Underground caverns (walker) ─────────────────── */
    carve_underground_caverns(&mut tiles, w, h, &height_map);

    /* --- spawn player ---------------------------------------------------- */
    let spawn_x  = w / 2;
    let surf_row = height_map[spawn_x];
    let spawn = Vec2::new(
        spawn_x as f32 * TILE_SIZE,
        tile_to_world_y(h, surf_row) + TILE_SIZE * 0.5 + PLAYER_HEIGHT * 0.5 + 4.0,
    );

    commands.spawn((
        Sprite::from_atlas_image(
            sheet,
            TextureAtlas { layout: layout_handle, index: 0 },
        ),
        Transform {
            translation: spawn.extend(10.0),
            scale: Vec3::splat(1.8),
            ..default()
        },
        Player { grounded: false },
        Velocity(Vec2::ZERO),
        Inventory { selected: HeldItem::Pickaxe },
        AnimationIndices { first: 0, last: 5 },
        AnimationTimer(Timer::from_seconds(0.12, TimerMode::Repeating)),
    ));

    /* --- insert resources ----------------------------------------------- */
    commands.insert_resource(Terrain {
        tiles,
        sprite_entities,
        changed_tiles: VecDeque::new(),
        free_sprites: Vec::new(),
        width: w,
        height: h,
        height_map,
        color_noise,
    });
    commands.insert_resource(LastRect::default());
}


/* ──────────────────── Mountains (new) ────────────────── */
fn generate_mountains(
    tiles: &mut [Vec<Tile>],
    height_map: &[usize],
    w: usize,
    h: usize,
    player_x: usize,
) {
    use rand::Rng;
    use noise::{NoiseFn, Perlin};

    const MOUNTAINS_PER_SIDE:     usize = 3;
    const MIN_DIST_FROM_PLAYER:   i32   = 200;
    const MIN_GAP_BETWEEN:        i32   = 120;
    const WIDTH_MIN:              usize = 256;
    const WIDTH_MAX:              usize = 768;
    const HEIGHT_MIN:             usize = 128;
    const HEIGHT_MAX:             usize = 256;
    const MAX_ATTEMPTS:           usize = 5_000;

    #[derive(Clone, Copy)]
    struct Band { l: i32, r: i32 }

    let mut rng         = rand::thread_rng();
    let ridge_noise     = Perlin::new(rng.gen());
    let mut placed: Vec<Band> = Vec::new();

    for side in [true, false] {                // true = left, false = right
        let mut attempts = 0usize;
        let mut made     = 0usize;

        while made < MOUNTAINS_PER_SIDE && attempts < MAX_ATTEMPTS {
            attempts += 1;

            /* --- choose footprint & reject if it overlaps ---------------- */
            let width  = rng.gen_range(WIDTH_MIN..=WIDTH_MAX) as i32;
            let half   = width / 2;
            let height = rng.gen_range(HEIGHT_MIN..=HEIGHT_MAX) as i32;

            let cx = if side {
                rng.gen_range(half .. player_x as i32 - MIN_DIST_FROM_PLAYER - half)
            } else {
                rng.gen_range(player_x as i32 + MIN_DIST_FROM_PLAYER + half .. w as i32 - half - 1)
            };

            let span = Band { l: cx - half - MIN_GAP_BETWEEN, r: cx + half + MIN_GAP_BETWEEN };
            if placed.iter().any(|b| b.r >= span.l && b.l <= span.r) { continue; }
            placed.push(Band { l: cx - half, r: cx + half });
            made += 1;

            /* --- sculpt every column in the ridge ----------------------- */
            for dx in -half..=half {
                let x  = cx + dx;
                if !(0..w as i32).contains(&x) { continue; }
                let ux = x as usize;

                let nx            = dx as f32 / half as f32;          // −1 … +1
                // Slightly flatter, less “needly” silhouette
                let base_profile  = (1.0 - nx.abs()).powf(1.3);
                // Reduce random bump amplitude for smoother ridges
                let bump          = ridge_noise.get([x as f64 * 0.06, cx as f64 * 0.002]) as f32;
                let height_factor = 1.0 + bump * 0.22;                // ±22 % variation
                let column_peak   = (height as f32 * base_profile * height_factor).round() as i32;

                let surface = height_map[ux] as i32;
                let top     = (surface - column_peak).max(0);

                /* 1 ─── build above‑ground part (snow → stone → dirt/grass) */
                for y in (top..=surface).rev() {
                    let above_ground = surface - y;
                    let kind = if above_ground <= 1 {
                        // Ground‑level & first slope tile: mix of stone and dirt, never grass
                        TileKind::Stone
                    } else {
                        let dist = y - top;
                        if dist <= 2 {
                            TileKind::Snow
                        } else if dist <= 6 {
                            if rng.gen::<f32>() < 0.4 { TileKind::Snow } else { TileKind::Stone }
                        } else {
                            TileKind::Stone
                        }
                    };

                    tiles[y as usize][ux].kind = kind;
                    tiles[y as usize][ux].mine_time = match kind {
                        TileKind::Grass => 0.20,
                        TileKind::Dirt  => 0.25,
                        TileKind::Stone => 0.50,
                        TileKind::Snow  => 0.15,
                        _               => 0.0,
                    };
                }

                /* 2 ─── extend roots beneath the original ground ---------- */
                let base_depth = ((column_peak as f32) * 0.5).ceil() as i32;
                for d in 1..=base_depth {
                    let y = surface + d;
                    if y >= h as i32 { break; }

                    // upper third stays stone, lower two‑thirds fade into dirt
                    let kind = if d <= base_depth / 3 { TileKind::Stone } else { TileKind::Dirt };

                    if matches!(
                        tiles[y as usize][ux].kind,
                        TileKind::Grass | TileKind::Dirt | TileKind::Stone
                    ) {
                        tiles[y as usize][ux].kind      = kind;
                        tiles[y as usize][ux].mine_time = if kind == TileKind::Stone { 0.50 } else { 0.25 };
                    }
                }
            }
        }

        if attempts == MAX_ATTEMPTS {
            eprintln!(
                "⚠️  mountain generation hit MAX_ATTEMPTS ({}) on the {} side; placed {}/{} mountains.",
                MAX_ATTEMPTS,
                if side { "left" } else { "right" },
                made,
                MOUNTAINS_PER_SIDE
            );
        }

        println!("Successfully generated mountains.");
    }
}


/* ===========================================================
   walker‑style underground caverns (larger & more elaborate)
   =========================================================== */
   fn carve_underground_caverns(
    tiles: &mut [Vec<Tile>],
    width: usize,
    height: usize,
    height_map: &[usize],
) {
    use rand::Rng;
    use bevy::math::{Vec2, Mat2};

    // Tunables
    const UNDER_STEPS_MIN:    u16 = 400;
    const UNDER_STEPS_MAX:    u16 = 700;
    const UNDER_TURN_CHANCE:  f32 = 0.25;
    const UNDER_TUNNEL_R_MIN: i32 = 2;
    const UNDER_TUNNEL_R_MAX: i32 = 4;
    const UNDER_ROOM_R_MIN:   i32 = 6;
    const UNDER_ROOM_R_MAX:   i32 = 10;

    let mut rng = rand::thread_rng();
    let walker_count = (width / 32).max(10);

    // Seed walkers a bit below the surface but above obsidian
    let mut walkers: Vec<(Vec2, Vec2)> = Vec::new();
    for _ in 0..walker_count {
        let x = rng.gen_range(4..width - 4) as i32;
        let surface = height_map[x as usize] as i32;
        let y_min = surface + MIN_CAVE_DEPTH as i32;
        let y_max = (height as f32 * OBSIDIAN_START_FRAC) as i32 - 4;
        if y_min >= y_max { continue; }
        let y = rng.gen_range(y_min..y_max);
        let pos = Vec2::new(x as f32, y as f32);
        let dir = Vec2::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-0.3..0.3),
        ).normalize();
        walkers.push((pos, dir));
    }

    // Walk and carve
    for (mut pos, mut dir) in walkers {
        let steps = rng.gen_range(UNDER_STEPS_MIN..=UNDER_STEPS_MAX);
        for _ in 0..steps {
            let radius = if rng.gen::<f32>() < 0.15 {
                rng.gen_range(UNDER_ROOM_R_MIN..=UNDER_ROOM_R_MAX)
            } else {
                rng.gen_range(UNDER_TUNNEL_R_MIN..=UNDER_TUNNEL_R_MAX)
            };
            carve_disc(tiles, width, height, pos.x as i32, pos.y as i32, radius);

            if rng.gen::<f32>() < UNDER_TURN_CHANCE {
                let ang = rng.gen_range(-1.0..1.0);
                dir = (Mat2::from_angle(ang) * dir).normalize();
            }
            pos += dir;

            // Stop if we wander outside the map
            if pos.x < 2.0 || pos.x > (width - 2) as f32
                || pos.y < 2.0 || pos.y > (height - 2) as f32 {
                break;
            }
        }
    }
}

#[inline(always)]
fn carve_disc(
    tiles: &mut [Vec<Tile>],
    w: usize,
    h: usize,
    cx: i32,
    cy: i32,
    r:  i32,
) {
    for dx in -r..=r {
        let nx    = dx as f32 / r as f32;
        let slice = ((1.0 - nx * nx).sqrt() * r as f32).round() as i32;

        for dy in -slice..=slice {
            let x = cx + dx;
            let y = cy + dy;
            if x < 0 || x >= w as i32 || y < 0 || y >= h as i32 { continue; }
            if matches!(tiles[y as usize][x as usize].kind, TileKind::Sky) { continue; }

            tiles[y as usize][x as usize].kind      = TileKind::Air;
            tiles[y as usize][x as usize].mine_time = 0.0;
        }
    }
}


/* ===========================================================
   helpers for streaming sprites
   =========================================================== */
#[inline]
fn color_and_z(terrain: &Terrain, x: usize, y: usize) -> (Color, f32) {
    let tile     = terrain.tiles[y][x];
    let base_rgb = tile.base_rgb * brightness(&tile);

    let color = Color::srgb(
        base_rgb.x.clamp(0.0, 1.0),
        base_rgb.y.clamp(0.0, 1.0),
        base_rgb.z.clamp(0.0, 1.0),
    );
    let z = if tile.kind == TileKind::Air { -1.0 } else { 0.0 };
    (color, z)
}

#[inline]
fn ensure_sprite(commands: &mut Commands, terrain: &mut Terrain, x: i32, y: i32) {
    if x < 0 || y < 0 || x >= terrain.width as i32 || y >= terrain.height as i32 {
        return;
    }
    let (ux, uy) = (x as usize, y as usize);
    let idx      = terrain.idx(ux, uy);
    if terrain.sprite_entities[idx].is_some() {
        return;
    }
    if !matches!(
        terrain.tiles[uy][ux].kind,
        TileKind::Grass | TileKind::Dirt | TileKind::Stone |
        TileKind::Obsidian | TileKind::Snow | TileKind::Air
    ) {
        return;                         // Sky never gets a sprite
    }

    let (color, z) = color_and_z(terrain, ux, uy);

    let entity = if let Some(e) = terrain.free_sprites.pop() {
        commands.entity(e).insert((
            Visibility::Visible,
            Sprite {
                color,
                custom_size: Some(Vec2::splat(TILE_SIZE)),
                ..default()
            },
            Transform::from_xyz(
                ux as f32 * TILE_SIZE,
                tile_to_world_y(terrain.height, uy),
                z,
            ),
            TileSprite { x: ux, y: uy },
        ));
        e
    } else {
        spawn_tile(commands, terrain, ux, uy)
    };
    terrain.sprite_entities[idx] = Some(entity);
}


/* ===========================================================
   stream_tiles_system – stripe differencing + pooling
   (unchanged from previous version)
   =========================================================== */
   pub fn stream_tiles_system(
    mut commands: Commands,
    mut terrain: ResMut<Terrain>,
    rect: Res<ActiveRect>,
    mut last_rect: ResMut<LastRect>,
) {
    let new = *rect;
    if last_rect.0 == Some(new) {
        return;
    }

    /* first populate ----------------------------------------------------- */
    let Some(prev) = last_rect.0 else {
        for y in new.min_y..=new.max_y {
            for x in new.min_x..=new.max_x {
                ensure_sprite(&mut commands, &mut terrain, x, y);
            }
        }
        last_rect.0 = Some(new);
        return;
    };

    /* stripes entering view ---------------------------------------------- */
    for x in new.min_x..=new.max_x {
        if x < prev.min_x || x > prev.max_x {
            for y in new.min_y..=new.max_y {
                ensure_sprite(&mut commands, &mut terrain, x, y);
            }
        }
    }
    for y in new.min_y..=new.max_y {
        if y < prev.min_y || y > prev.max_y {
            for x in new.min_x..=new.max_x {
                ensure_sprite(&mut commands, &mut terrain, x, y);
            }
        }
    }

    /* stripes leaving view (pool) ---------------------------------------- */
    for x in prev.min_x..=prev.max_x {
        if x < new.min_x || x > new.max_x {
            for y in prev.min_y..=prev.max_y {
                let (ux, uy) = (x as usize, y as usize);
                let idx = terrain.idx(ux, uy);
                if let Some(e) = terrain.sprite_entities[idx] {
                    commands.entity(e).insert(Visibility::Hidden);
                    terrain.free_sprites.push(e);
                    terrain.sprite_entities[idx] = None;
                }
            }
        }
    }
    for y in prev.min_y..=prev.max_y {
        if y < new.min_y || y > new.max_y {
            for x in prev.min_x..=prev.max_x {
                if x >= new.min_x && x <= new.max_x {
                    let (ux, uy) = (x as usize, y as usize);
                    let idx = terrain.idx(ux, uy);
                    if let Some(e) = terrain.sprite_entities[idx] {
                        commands.entity(e).insert(Visibility::Hidden);
                        terrain.free_sprites.push(e);
                        terrain.sprite_entities[idx] = None;
                    }
                }
            }
        }
    }

    last_rect.0 = Some(new);
}

/* ===========================================================
   update_active_rect_system (unchanged)
   =========================================================== */
pub fn update_active_rect_system(
    cam_q: Query<&Transform, With<Camera>>,
    window_q: Query<&Window>,
    terrain: Res<Terrain>,
    mut rect_res: Option<ResMut<ActiveRect>>,
    mut commands: Commands,
) {
    let cam_tf = match cam_q.get_single() {
        Ok(t) => t,
        Err(_) => return,
    };
    let window = window_q.single();

    let pad_x = ((window.width() * 0.5) / TILE_SIZE).ceil() as i32 + ACTIVE_MARGIN;
    let pad_y = ((window.height() * 0.5) / TILE_SIZE).ceil() as i32 + ACTIVE_MARGIN;

    let px = (cam_tf.translation.x / TILE_SIZE).round() as i32;
    let py = world_to_tile_y(terrain.height, cam_tf.translation.y);

    let new = ActiveRect {
        min_x: (px - pad_x).clamp(0, terrain.width as i32 - 1),
        max_x: (px + pad_x).clamp(0, terrain.width as i32 - 1),
        min_y: (py - pad_y).clamp(0, terrain.height as i32 - 1),
        max_y: (py + pad_y).clamp(0, terrain.height as i32 - 1),
    };

    match rect_res {
        Some(mut r) if *r != new => *r = new,
        None => commands.insert_resource(new),
        _ => {}
    }
}

/* ===========================================================
redraw_changed_tiles_system – with cached, stepped tint
=========================================================== */
pub fn redraw_changed_tiles_system(
    mut commands: Commands,
    mut terrain: ResMut<Terrain>,
) {
    use crate::constants::{
        COLOR_NOISE_SCALE,
        COLOR_VARIATION_LEVELS,
        COLOR_VARIATION_STRENGTH,
        TILE_SIZE,
    };

    let mut spawns:  Vec<(Sprite, Transform, TileSprite)> = Vec::new();
    let mut inserts: Vec<(Entity, (Visibility, Sprite, Transform, TileSprite))> = Vec::new();

    while let Some((x, y)) = terrain.changed_tiles.pop_front() {
        let idx_sprite = terrain.idx(x, y);
        let kind       = terrain.tiles[y][x].kind;

        /* SKY tiles: hide & recycle sprite */
        if kind == TileKind::Sky {
            if let Some(e) = terrain.sprite_entities[idx_sprite] {
                commands.entity(e).insert(Visibility::Hidden);
                terrain.free_sprites.push(e);
                terrain.sprite_entities[idx_sprite] = None;
            }
            continue;
        }

        /* re‑tint --------------------------------------------------------- */
        let raw = terrain.color_noise.get([
            x as f64 * COLOR_NOISE_SCALE,
            y as f64 * COLOR_NOISE_SCALE,
        ]) as f32;

        let step = (((raw + 1.0) * 0.5) * COLOR_VARIATION_LEVELS as f32)
            .floor()
            .clamp(0.0, (COLOR_VARIATION_LEVELS - 1) as f32);
        let norm   = step / (COLOR_VARIATION_LEVELS as f32 - 1.0) * 2.0 - 1.0;
        let factor = 1.0 + norm * COLOR_VARIATION_STRENGTH;

        terrain.tiles[y][x].base_rgb = match kind {
            TileKind::Grass    => Vec3::new(0.13, 0.70, 0.08) * factor,
            TileKind::Snow     => Vec3::new(0.95, 0.95, 0.95) * factor,
            TileKind::Dirt     => Vec3::new(0.55, 0.27, 0.07) * factor,
            TileKind::Stone    => Vec3::new(0.50, 0.50, 0.50) * factor,
            TileKind::Obsidian => Vec3::new(0.20, 0.05, 0.35) * factor,
            TileKind::Air      => Vec3::new(0.20, 0.10, 0.05) * factor, // BACKGROUND_BROWN
            _ => terrain.tiles[y][x].base_rgb,
        };

        /* colour & depth -------------------------------------------------- */
        let (color, z) = color_and_z(&terrain, x, y);
        let tile_sprite = TileSprite { x, y };

        match terrain.sprite_entities[idx_sprite] {
            Some(entity) => {
                let transform = Transform {
                    translation: Vec3::new(
                        x as f32 * TILE_SIZE,
                        tile_to_world_y(terrain.height, y),
                        z,
                    ),
                    ..default()
                };
                let sprite = Sprite {
                    color,
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                };
                inserts.push((entity, (Visibility::Visible, sprite, transform, tile_sprite)));
            }
            None => {
                let transform = Transform {
                    translation: Vec3::new(
                        x as f32 * TILE_SIZE,
                        tile_to_world_y(terrain.height, y),
                        z,
                    ),
                    ..default()
                };
                let sprite = Sprite {
                    color,
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                };

                if let Some(entity) = terrain.free_sprites.pop() {
                    inserts.push((entity, (Visibility::Visible, sprite, transform, tile_sprite)));
                    terrain.sprite_entities[idx_sprite] = Some(entity);
                } else {
                    spawns.push((sprite, transform, tile_sprite));
                }
            }
        }
    }

    /* flush the command buffers */
    if !spawns.is_empty() {
        commands.spawn_batch(spawns);
    }
    if !inserts.is_empty() {
        commands.insert_or_spawn_batch(inserts);
    }
}

/* ===========================================================
   spawn_tile helper
   =========================================================== */
#[inline]
fn brightness(tile: &Tile) -> f32 {
    if tile.visible { 1.0 } else if tile.explored { EXPLORED_BRIGHTNESS } else { 0.0 }
}

pub fn spawn_tile(
    commands: &mut Commands,
    terrain: &Terrain,
    x: usize,
    y: usize,
) -> Entity {
    let (color, z) = color_and_z(terrain, x, y);
    commands.spawn((
        Sprite {
            color,
            custom_size: Some(Vec2::splat(TILE_SIZE)),
            ..default()
        },
        Transform::from_xyz(
            x as f32 * TILE_SIZE,
            tile_to_world_y(terrain.height, y),
            z,
        ),
        TileSprite { x, y },
    )).id()
}

/* ===========================================================
   digging_system (mouse circular dig) – unchanged
   =========================================================== */
pub fn digging_system(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cam_q: Query<(&Camera, &GlobalTransform)>,
    mut terrain: ResMut<Terrain>,
) {
    if !mouse.pressed(MouseButton::Left) {
        return;
    }
    let window = windows.single();
    let Some(cursor) = window.cursor_position() else { return };
    let (cam, cam_tf) = cam_q.single();
    let Ok(world) = cam.viewport_to_world_2d(cam_tf, cursor) else { return };

    let min_x = ((world.x - DIG_RADIUS) / TILE_SIZE).floor() as i32;
    let max_x = ((world.x + DIG_RADIUS) / TILE_SIZE).ceil()  as i32;

    let min_y_world = world.y - DIG_RADIUS;
    let max_y_world = world.y + DIG_RADIUS;
    let min_y = world_to_tile_y(terrain.height, max_y_world);
    let max_y = world_to_tile_y(terrain.height, min_y_world);

    for ty in min_y..=max_y {
        for tx in min_x..=max_x {
            if tx < 0 || ty < 0 || tx >= terrain.width as i32 || ty >= terrain.height as i32 {
                continue;
            }
            let dx = tx as f32 * TILE_SIZE - world.x;
            let dy = tile_to_world_y(terrain.height, ty as usize) - world.y;
            if dx*dx + dy*dy < DIG_RADIUS * DIG_RADIUS {
                let (ux, uy) = (tx as usize, ty as usize);
                if matches!(
                    terrain.tiles[uy][ux].kind,
                    TileKind::Grass | TileKind::Dirt | TileKind::Stone | TileKind::Obsidian | TileKind::Snow
                ) {
                    terrain.tiles[uy][ux].kind = TileKind::Air;
                    terrain.changed_tiles.push_back((ux, uy));
                }
            }
        }
    }
}

/* ===========================================================
   solid collision check
   =========================================================== */
#[inline]
pub fn solid(terrain: &Terrain, tx: i32, ty: i32) -> bool {
    if tx < 0 || ty < 0 || tx >= terrain.width as i32 || ty >= terrain.height as i32 {
        return true;
    }
    matches!(
        terrain.tiles[ty as usize][tx as usize].kind,
        TileKind::Grass | TileKind::Dirt | TileKind::Stone | TileKind::Obsidian | TileKind::Snow
    )
}

/* ===========================================================
   sync_tile_sprite_entities_system
   – writes freshly spawned TileSprite entity IDs into the grid
   =========================================================== */
   pub fn sync_tile_sprite_entities_system(
    mut terrain: ResMut<Terrain>,
    q: Query<(Entity, &TileSprite), Added<TileSprite>>,
) {
    for (entity, tile) in &q {
        if tile.y < terrain.height && tile.x < terrain.width {
            let idx = terrain.idx(tile.x, tile.y);
            terrain.sprite_entities[idx] = Some(entity);
        }
    }
}