//! Brushes (`Carrier::Brush`): a spell held like a beam that paints. Each
//! tick it's held, a stream runs from the hand to the cursor (stopped by
//! what's in the way: it doesn't paint through rock), and the stroke since
//! the last tick is painted, `radius` round: each open cell it covers sets
//! solid (`solid`: ice) if it's held up (touching real ground, or a cell
//! this brush painted that's held up, no more than `reach` cells out from
//! the ground: walls, ledges, bridges, arches), and is otherwise its loose
//! form (`loose`: snow), falling. Each cell set costs `per_cell` mana (a
//! loose one half); out of mana, it stops. Never in a body.
//!
//! What it passes over freezes: a liquid that freezes (water) turns to ice
//! (`solid`) and holds what's painted on it (a pool frozen over, a wall
//! built off it); painted ice is set deeply cold (`COLD`), so it lasts in a
//! warm place; and everything round the stroke is chilled (`CHILL`): the
//! ground under it frosts over (`render`'s rime), water near it freezes.
//!
//! It isn't a ruler: the brush wobbles side to side along a stroke
//! (`WOBBLE`), swells and thins as it goes (`LUMPS`: smooth noise over
//! the world), and its rim sets only some of its cells each pass, so edges
//! come out ragged; under an overhang, ice now and then grows an icicle.
//!
//! As a cell sets, its element shows it (`Brush::look`: frost glitters and
//! tinkles), the stream's motes running down it to the cursor.

use bevy::prelude::*;
use platypus_sim::{Cell, CellPos, Kind, World};
use std::collections::HashMap;

use crate::magic::runes::Emitter;

/// Where each caster's stroke was last tick: the cursor's end, the tick.
#[derive(Resource, Default)]
pub struct Strokes(HashMap<Entity, (Vec2, u64)>);

/// The cells brushes have set solid, each how many cells out from real
/// ground it's held (0: touching it).
#[derive(Resource, Default)]
pub struct Painted(HashMap<CellPos, u8>);

/// Steps along a stroke (cells): a cell at a time, so a quick drag paints a
/// line, not dots.
const STEP: f32 = 1.0;
/// The stream's motes a tick, and the setting cells' glitter (one in this
/// many cells set gets some).
const STREAM_MOTES: usize = 3;
const GLITTER_ONE_IN: u64 = 3;
/// Setting cells sound at most every this many ticks.
pub const SOUND_TICKS: u64 = 7;
/// The painted cells are looked over (melted ones forgotten) this often.
const TIDY_EVERY: u64 = 120;
/// The wobble: this far either side (cells), its sway this long (cells
/// along the stroke).
const WOBBLE: f32 = 1.1;
const SWAY: f32 = 9.0;
/// The brush swells and thins this much (a share of its radius, either
/// way), over lumps this big (cells).
const LUMPS: (f32, f32) = (0.35, 4.0);
/// Its rim (past this share of its reach): each pass, a cell there sets
/// only this often.
const RIM: f32 = 0.65;
const RIM_SETS: f32 = 0.45;
/// A cell set with open air under it grows an icicle this often (one in
/// so many), this long at most (cells).
const ICICLE_ONE_IN: u64 = 14;
const ICICLE_MOST: i32 = 4;
/// Painted ice is set this much colder than round it (°C), so it lasts.
const COLD: i16 = -70;
/// Round each step of a stroke, this much heat taken (°C, less to the rim),
/// this far past the brush (cells), down to this cold at most (°C): what's
/// under it frosts, and no colder.
const CHILL: i32 = 6;
const CHILL_PAST: i32 = 4;
const CHILL_TO: i32 = -20;

/// What's round `at`, `r` cells, chilled (`CHILL`, less to the rim), never
/// below `CHILL_TO`; what freezes, freezing as the world has it.
fn chill(world: &mut World, at: CellPos, r: i32) {
    let mats = world.materials().clone();
    for dy in -r..=r {
        for dx in -r..=r {
            let d2 = dx * dx + dy * dy;
            if d2 > r * r {
                continue;
            }
            let p = at.offset(dx, dy);
            let (Some(c), Some(t)) = (world.get(p), world.temperature(p)) else { continue };
            if c.is_air() || !matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder | Kind::Liquid) || t <= CHILL_TO {
                continue;
            }
            let k = 1.0 - (d2 as f32).sqrt() / r as f32;
            let take = ((CHILL as f32 * (0.3 + 0.7 * k)) as i32).max(1).min(t - CHILL_TO);
            let mut cold = c;
            cold.heat = cold.heat.saturating_sub(take as i16);
            world.set(p, cold);
        }
    }
}

/// Smooth value noise (0..1) over the world, lumps `size` cells across.
fn lumps(x: f32, y: f32, size: f32, salt: u64) -> f32 {
    let (fx, fy) = (x / size, y / size);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let v = |i: f32, j: f32| (platypus_sim::rng::hash(&[i as i64 as u64, j as i64 as u64, salt]) % 10_000) as f32 / 10_000.0;
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(tx), s(ty));
    let a = v(x0, y0) + (v(x0 + 1.0, y0) - v(x0, y0)) * sx;
    let b = v(x0, y0 + 1.0) + (v(x0 + 1.0, y0 + 1.0) - v(x0, y0 + 1.0)) * sx;
    a + (b - a) * sy
}

/// What a brush is (`Carrier::Brush`).
pub struct Brush<'a> {
    pub solid: &'a str,
    pub loose: &'a str,
    pub radius: f32,
    pub range: f32,
    pub reach: u8,
    pub per_cell: f32,
    /// Off a cell as it sets; along the stream.
    pub look: Option<&'a Emitter>,
    pub stream: Option<&'a Emitter>,
}

/// One tick of a brush held: `caster` painting from `from` toward its aim
/// (`dir`, `aim` away), with `mana` to spend. Bodies' boxes it won't paint
/// in. Whether it set anything (for its sound).
#[allow(clippy::too_many_arguments)]
pub fn paint(
    world: &mut World,
    strokes: &mut Strokes,
    painted: &mut Painted,
    sparks: &mut crate::vfx::Sparks,
    caster: Entity,
    from: Vec2,
    dir: Vec2,
    aim: f32,
    brush: &Brush,
    mana: &mut f32,
    boxes: &[(Vec2, Vec2)],
) -> bool {
    let mats = world.materials().clone();
    let (Some(solid), Some(loose)) = (mats.id(brush.solid), mats.id(brush.loose)) else { return false };
    let tick = world.tick();
    // (Forget what's melted or been dug, now and then.)
    if tick.is_multiple_of(TIDY_EVERY) {
        painted.0.retain(|p, _| world.get(*p).is_some_and(|c| c.material == solid));
    }
    // To the cursor, or as near as the way's open: through what it's
    // painted itself (or it grows back along its own stroke toward the
    // hand), not through rock.
    let reach = aim.min(brush.range);
    let mut tip = from + dir * reach;
    let steps = (reach * 2.0) as i32;
    for i in 1..=steps {
        let p = from + dir * (i as f32 * 0.5);
        let c = CellPos::from_world(p.x, p.y);
        let clear = world.get(c).is_some_and(|cell| cell.is_air() || matches!(mats.phys(cell.material).kind, Kind::Gas | Kind::Fire | Kind::Plant) || (cell.material == solid && painted.0.contains_key(&c)));
        if !clear {
            // (Short of what stopped it: painting against a wall lays on it.)
            tip = p - dir * 0.6;
            break;
        }
    }
    // The stroke: from last tick's end (held on), else just here.
    let last = strokes.0.get(&caster).filter(|(_, t)| tick.saturating_sub(*t) <= 2).map_or(tip, |(p, _)| *p);
    strokes.0.insert(caster, (tip, tick));
    // The stream's motes, running out to the cursor.
    if let Some(e) = brush.stream {
        for i in 0..STREAM_MOTES {
            let k = ((tick * 7 + i as u64 * 13) % 17) as f32 / 17.0;
            sparks.emit(e, 1, from.lerp(tip, k), (tip - from).normalize_or(Vec2::X), Vec2::ZERO);
        }
    }
    let open = |w: &World, p: CellPos| w.get(p).is_some_and(|c| c.is_air() || matches!(mats.phys(c.material).kind, Kind::Gas | Kind::Plant));
    let in_body = |p: CellPos| {
        let c = Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5);
        boxes.iter().any(|(pos, half)| ((c - *pos).abs() - *half).max_element() < 0.5)
    };
    // Held up by what's round it: real ground (0), or a painted cell that
    // is (one further out).
    let held = |w: &World, painted: &Painted, p: CellPos| -> Option<u8> {
        let mut best: Option<u8> = None;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)] {
            let q = p.offset(dx, dy);
            let here = match painted.0.get(&q) {
                Some(&s) if w.get(q).is_some_and(|c| c.material == solid) => Some(s.saturating_add(1)),
                Some(_) => None,
                None => w.get(q).filter(|c| !c.is_air() && matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder)).map(|_| 0),
            };
            if let Some(h) = here {
                best = Some(best.map_or(h, |b: u8| b.min(h)));
            }
        }
        best
    };
    let mut rng = platypus_sim::rng::Rng::seeded(&[tick, caster.to_bits(), 0xB2A5]);
    let steps = (last.distance(tip) / STEP).ceil().max(1.0) as usize;
    let along = (tip - last).normalize_or(Vec2::X);
    let across = Vec2::new(-along.y, along.x);
    let r = brush.radius;
    let mut set = false;
    for s in 0..=steps {
        let mut at = last.lerp(tip, s as f32 / steps as f32);
        // (A wobble across the stroke, slow, by where along the world it is.)
        let sway = lumps(at.x + at.y, 0.0, SWAY, 0x5EA7) * 2.0 - 1.0;
        at += across * sway * WOBBLE;
        let (cx, cy) = (at.x.floor() as i32, at.y.floor() as i32);
        let ri = (r * (1.0 + LUMPS.0)).ceil() as i32;
        let mut loose_here = false;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let p = CellPos::new(cx + dx, cy + dy);
                // (Its reach here: swelling and thinning over the world.)
                let reach = r * (1.0 + LUMPS.0 * (lumps(p.x as f32, p.y as f32, LUMPS.1, 0x1B5) * 2.0 - 1.0));
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                if d > reach + 0.3 {
                    continue;
                }
                // (The rim: ragged, a cell now and then a pass.)
                if d > reach * RIM && !rng.chance((RIM_SETS * 255.0) as u8) {
                    continue;
                }
                if in_body(p) {
                    continue;
                }
                // (A liquid that freezes: frozen, ground to build on.)
                if let Some(c) = world.get(p).filter(|c| !c.is_air() && mats.phys(c.material).kind == Kind::Liquid && mats.phys(c.material).below_into == solid) {
                    let _ = c;
                    if *mana < brush.per_cell {
                        return set;
                    }
                    *mana -= brush.per_cell;
                    let mut ice: Cell = mats.spawn(solid, &mut rng);
                    ice.heat = COLD;
                    world.set(p, ice);
                    painted.0.insert(p, 0);
                    set = true;
                    continue;
                }
                if !open(world, p) {
                    continue;
                }
                match held(world, painted, p) {
                    Some(h) if h <= brush.reach => {
                        if *mana < brush.per_cell {
                            return set;
                        }
                        *mana -= brush.per_cell;
                        let mut c: Cell = mats.spawn(solid, &mut rng);
                        c.heat = COLD;
                        world.set(p, c);
                        painted.0.insert(p, h);
                        set = true;
                        // (Open under it: now and then an icicle.)
                        if brush.solid == "ice" && platypus_sim::rng::hash(&[p.x as u64, p.y as u64, 0x1C1C]).is_multiple_of(ICICLE_ONE_IN) {
                            let long = 1 + (platypus_sim::rng::hash(&[p.x as u64, p.y as u64, 0x1C1D]) % ICICLE_MOST as u64) as i32;
                            for k in 1..=long {
                                let q = p.offset(0, -k);
                                if !open(world, q) || in_body(q) || *mana < brush.per_cell {
                                    break;
                                }
                                *mana -= brush.per_cell;
                                let mut ic: Cell = mats.spawn(solid, &mut rng);
                                ic.heat = COLD;
                                world.set(q, ic);
                                painted.0.insert(q, h.saturating_add(k as u8));
                            }
                        }
                        if let Some(e) = brush.look
                            && platypus_sim::rng::hash(&[p.x as u64, p.y as u64, tick]).is_multiple_of(GLITTER_ONE_IN)
                        {
                            sparks.emit(e, e.count as usize, Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5), Vec2::Y, Vec2::ZERO);
                        }
                    }
                    _ => loose_here = true,
                }
            }
        }
        // (Round it, chilled, down to `CHILL_TO`: the ground under it
        // frosts, water near it freezes.)
        if brush.solid == "ice" && s % 2 == 0 {
            chill(world, CellPos::new(cx, cy), ri + CHILL_PAST);
        }
        // In the open: its loose form, a flake or two falling from here.
        if loose_here && *mana >= brush.per_cell * 0.5 {
            *mana -= brush.per_cell * 0.5;
            world.spill_heated([at.x, at.y], [0.0, -1.0], loose, if brush.solid == "ice" { COLD } else { 0 }, 1, 0.15, 1.2);
        }
    }
    set
}
