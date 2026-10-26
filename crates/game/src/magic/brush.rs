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
    let r = brush.radius;
    let mut set = false;
    for s in 0..=steps {
        let at = last.lerp(tip, s as f32 / steps as f32);
        let (cx, cy) = (at.x.floor() as i32, at.y.floor() as i32);
        let ri = r.ceil() as i32;
        let mut loose_here = false;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                if (dx * dx + dy * dy) as f32 > r * r + 0.5 {
                    continue;
                }
                let p = CellPos::new(cx + dx, cy + dy);
                if !open(world, p) || in_body(p) {
                    continue;
                }
                match held(world, painted, p) {
                    Some(h) if h <= brush.reach => {
                        if *mana < brush.per_cell {
                            return set;
                        }
                        *mana -= brush.per_cell;
                        let mut c: Cell = mats.spawn(solid, &mut rng);
                        c.heat = 0;
                        world.set(p, c);
                        painted.0.insert(p, h);
                        set = true;
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
        // In the open: its loose form, a flake or two falling from here.
        if loose_here && *mana >= brush.per_cell * 0.5 {
            *mana -= brush.per_cell * 0.5;
            world.spill([at.x, at.y], [0.0, -1.0], loose, 1, 0.15, 1.2);
        }
    }
    set
}
