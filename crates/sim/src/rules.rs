//! Per-kind update rules. Integer-only; all randomness from `Hood::rng`.
//!
//! A cell that cannot do anything writes nothing, so its region goes to sleep.

use crate::cell::{Cell, flags};
use crate::material::{Kind, MatPhys, MaterialId};
use crate::step::Hood;

/// Fall-speed cap; a falling cell moves `1 + vy / 4` cells per tick (max 8).
/// Must stay well under `MAX_REACH`.
pub(crate) const MAX_FALL: i8 = 28;

const NEIGHBOURS: [(i32, i32); 4] = [(0, 1), (0, -1), (-1, 0), (1, 0)];
const NEIGHBOURS8: [(i32, i32); 8] = [(0, 1), (0, -1), (-1, 0), (1, 0), (-1, 1), (1, 1), (-1, -1), (1, -1)];

/// Heat moves this fraction (/1024) of a temperature difference per tick,
/// scaled by the lower conductivity of the pair (0..255).
const CONDUCTION_DIV: i32 = 1024;
/// Cooling toward ambient per tick, /1024 of the excess (at least 1 °C).
const RELAX: i32 = 6;
/// A cell held warm by a neighbour stops updating once its change is this small.
const EQUILIBRIUM_BAND: i32 = 2;
/// Beyond this, heat is clamped.
const HEAT_MIN: i16 = -300;
const HEAT_MAX: i16 = 4000;

pub(crate) fn update_cell(h: &mut Hood, x: i32, y: i32, mut c: Cell) {
    let p = *h.mats.phys(c.material);
    if (c.heat != 0 || p.climate_sensitive) && transition(h, x, y, c, &p) {
        return;
    }
    if c.heat != 0 || p.heat_source {
        conduct(h, x, y, &mut c, &p);
    }
    if c.flags & flags::BURNING != 0 && burn(h, x, y, &mut c, &p) {
        return;
    }
    if p.interacts && interact(h, x, y, c, &p) {
        return;
    }
    let kind = if p.kind == Kind::Static && c.flags & flags::LOOSE != 0 { Kind::Powder } else { p.kind };
    match kind {
        Kind::Powder => {
            let _ = fall(h, x, y, c, &p) || slide(h, x, y, c, &p) || settle(h, x, y, c);
        }
        Kind::Liquid => {
            let _ = fall(h, x, y, c, &p) || slide(h, x, y, c, &p) || flow(h, x, y, c, &p) || settle(h, x, y, c);
        }
        Kind::Gas => gas(h, x, y, c, &p),
        Kind::Fire => fire(h, x, y, c, &p),
        Kind::Empty | Kind::Static => {}
    }
}

/// Can a cell with physics `mover` move into `target`?
#[inline(always)]
fn passable(h: &Hood, mover: &MatPhys, target: Option<Cell>) -> bool {
    match target {
        None => false,
        Some(t) if t.is_air() => true,
        Some(t) => {
            let tp = h.mats.phys(t.material);
            matches!(tp.kind, Kind::Liquid | Kind::Gas | Kind::Fire) && tp.density < mover.density
        }
    }
}

/// Move `c` from (x,y) to (tx,ty); whatever was there takes its place.
/// Something flammable moving into flames catches fire instead of pushing them away.
#[inline(always)]
fn swap_to(h: &mut Hood, x: i32, y: i32, tx: i32, ty: i32, mut c: Cell) {
    let mut displaced = h.get(tx, ty).expect("caller checked the target is loaded");
    if !displaced.is_air() && h.mats.phys(displaced.material).kind == Kind::Fire {
        let p = *h.mats.phys(c.material);
        if p.flammability > 0 {
            ignite(h, x, y, &p);
            return;
        }
    }
    c.clock = h.clock;
    displaced.clock = h.clock;
    h.set(tx, ty, c);
    h.set(x, y, displaced);
}

#[inline(always)]
fn spawn(h: &mut Hood, id: MaterialId) -> Cell {
    let mut c = h.mats.spawn(id, &mut h.rng);
    c.clock = h.clock;
    c
}

/// Melt, boil, freeze or catch fire at the material's temperatures.
/// Returns true if the cell became something else.
fn transition(h: &mut Hood, x: i32, y: i32, c: Cell, p: &MatPhys) -> bool {
    let t = h.ambient(y) + c.heat as i32;
    let into = if t >= p.ignites_at as i32 && c.flags & flags::BURNING == 0 {
        ignite(h, x, y, p);
        return true;
    } else if t >= p.above_at as i32 {
        p.above_into
    } else if t <= p.below_at as i32 {
        p.below_into
    } else {
        return false;
    };
    let mut n = spawn(h, into);
    // Keep the heat (molten stone is hot), unless the new material pins its own.
    if !h.mats.phys(into).heat_source {
        n.heat = c.heat;
    }
    h.set(x, y, n);
    note_if_solid_lost(h, x, y, p, into);
    true
}

/// A solid turned into something that isn't: whatever it held up may now hang free.
#[inline]
fn note_if_solid_lost(h: &mut Hood, x: i32, y: i32, was: &MatPhys, now: MaterialId) {
    if was.kind == Kind::Static && h.mats.phys(now).kind != Kind::Static {
        h.note_broken(x, y);
    }
}

/// Heat a burning cell holds (it glows, and heats its neighbours).
const BURN_HEAT: i16 = 650;

/// Set a cell on fire. Gases flash into flame; everything else starts burning
/// in place (`flags::BURNING`). Explosive materials sometimes blow up too.
pub(crate) fn ignite(h: &mut Hood, x: i32, y: i32, p: &MatPhys) {
    if let Some(ex) = p.explodes
        && h.rng.chance(ex.chance)
    {
        h.request_explosion(x, y, ex);
    }
    let Some(mut c) = h.get(x, y) else { return };
    if matches!(p.kind, Kind::Gas | Kind::Fire) {
        let flame = spawn(h, h.mats.fire());
        h.set(x, y, flame);
        return;
    }
    if c.flags & flags::BURNING != 0 {
        return;
    }
    c.flags |= flags::BURNING;
    c.life = p.burn_time;
    c.heat = c.heat.max(BURN_HEAT);
    h.set(x, y, c);
}

/// One tick of a burning cell. Returns true if it burned out (replaced).
fn burn(h: &mut Hood, x: i32, y: i32, c: &mut Cell, p: &MatPhys) -> bool {
    // Doused: a non-flammable, non-hot liquid touching it (water, not oil or lava).
    for (dx, dy) in NEIGHBOURS {
        if let Some(n) = h.get(x + dx, y + dy)
            && !n.is_air()
        {
            let np = h.mats.phys(n.material);
            if np.kind == Kind::Liquid && np.flammability == 0 && !np.hot {
                c.flags &= !flags::BURNING;
                c.heat = c.heat.min(120);
                h.set(x, y, *c);
                return false;
            }
        }
    }
    // Spread, diagonals included: flames lick around corners.
    for (dx, dy) in NEIGHBOURS8 {
        let Some(n) = h.get(x + dx, y + dy) else { continue };
        if n.is_air() || n.flags & flags::BURNING != 0 {
            continue;
        }
        let np = *h.mats.phys(n.material);
        if np.flammability > 0 && h.rng.chance(np.flammability) {
            ignite(h, x + dx, y + dy, &np);
        }
    }
    // Flames and smoke into the air around it, mostly upward.
    let (dx, dy) = [(0, 1), (0, 1), (-1, 0), (1, 0)][(h.rng.next_u32() % 4) as usize];
    if h.get(x + dx, y + dy).is_some_and(|a| a.is_air()) {
        let roll = h.rng.next_u8();
        let puff = if roll < 70 { Some(h.mats.fire()) } else if roll < 82 { h.mats.id("smoke") } else { None };
        if let Some(m) = puff.filter(|m| *m != MaterialId::AIR) {
            let cell = spawn(h, m);
            h.set(x + dx, y + dy, cell);
        }
    }
    // Burn down.
    if h.tick.is_multiple_of(4) {
        if c.life == 0 {
            let into = if h.rng.chance(p.burns_into_chance) { p.burns_into } else { MaterialId::AIR };
            let mut left = spawn(h, into);
            left.heat = c.heat / 2;
            h.set(x, y, left);
            note_if_solid_lost(h, x, y, p, into);
            return true;
        }
        c.life -= 1;
    }
    c.heat = c.heat.max(BURN_HEAT);
    h.set(x, y, *c);
    false
}

/// Heat flow. Each warm cell *pulls* toward its neighbours' temperatures and
/// cools toward ambient, writing only itself, so a region at equilibrium stops
/// writing and sleeps. A neighbour still at ambient is seeded once so it
/// starts taking part.
fn conduct(h: &mut Hood, x: i32, y: i32, c: &mut Cell, p: &MatPhys) {
    let before = c.heat;
    if p.heat_source {
        c.heat = p.heat;
    }
    let mut delta = 0i32;
    let mut inflow = false;
    for (dx, dy) in NEIGHBOURS {
        let (nx, ny) = (x + dx, y + dy);
        let Some(mut n) = h.get(nx, ny) else { continue };
        if n.is_air() {
            continue; // air holds no heat; hot gases and fire carry it instead
        }
        let np = h.mats.phys(n.material);
        let k = p.conductivity.min(np.conductivity) as i32;
        if k == 0 {
            continue;
        }
        if n.heat == 0 && !np.heat_source {
            // Seed only if the neighbour will then keep pulling enough to stay
            // warm; a seed of 1 would cool straight back to 0 and be re-seeded forever.
            let seed = (c.heat as i32 * k / CONDUCTION_DIV) as i16;
            if seed.abs() >= 2 {
                n.heat = seed;
                h.set(nx, ny, n);
            }
            continue;
        }
        // A source always reads as its fixed temperature, whatever it stores.
        let nh = if np.heat_source { np.heat } else { n.heat };
        let d = (nh as i32 - c.heat as i32) * k / CONDUCTION_DIV;
        inflow |= d > 0 && c.heat >= 0 || d < 0 && c.heat < 0;
        delta += d;
    }
    if p.heat_source {
        if c.heat != before {
            h.set(x, y, *c); // store the pinned heat once
        }
        return;
    }
    let excess = c.heat as i32;
    let relax = ((excess.abs() * RELAX) >> 10).max(1).min(excess.abs());
    delta -= excess.signum() * relax;
    // Held up by a neighbour and barely moving: call it equilibrium. (Cells
    // update one after another, so near balance they wobble by a degree or two.)
    if inflow && delta.abs() <= EQUILIBRIUM_BAND {
        delta = 0;
    }
    let mut heat = (c.heat as i32 + delta).clamp(HEAT_MIN as i32, HEAT_MAX as i32);
    if heat.signum() != excess.signum() && relax > 0 && !inflow {
        heat = 0; // don't overshoot ambient
    }
    c.heat = heat as i16;
    if c.heat != before {
        h.set(x, y, *c);
    }
}

/// Reactions and contact ignition. Returns true if this cell was transformed.
fn interact(h: &mut Hood, x: i32, y: i32, c: Cell, p: &MatPhys) -> bool {
    let mut pending = false;
    for (dx, dy) in NEIGHBOURS {
        let Some(n) = h.get(x + dx, y + dy) else { continue };
        if n.is_air() {
            continue;
        }
        let reactions = h.mats.reactions(c.material);
        if let Some(r) = reactions.iter().find(|r| r.partner == n.material).copied() {
            if h.rng.chance(r.chance) {
                let (a, b) = (spawn(h, r.self_into), spawn(h, r.partner_into));
                h.set(x, y, a);
                h.set(x + dx, y + dy, b);
                let np = *h.mats.phys(n.material);
                note_if_solid_lost(h, x, y, p, r.self_into);
                note_if_solid_lost(h, x + dx, y + dy, &np, r.partner_into);
                return true;
            }
            pending = true;
        }
        if p.hot && n.flags & flags::BURNING == 0 {
            let np = *h.mats.phys(n.material);
            if np.flammability > 0 {
                if h.rng.chance(np.flammability) {
                    ignite(h, x + dx, y + dy, &np);
                } else {
                    pending = true;
                }
            }
        }
    }
    if pending {
        // Something may still happen here next tick; don't let the region sleep.
        h.wake(x, y);
    }
    false
}

fn fall(h: &mut Hood, x: i32, y: i32, mut c: Cell, p: &MatPhys) -> bool {
    let speed = 1 + c.vy.max(0) as i32 / 4;
    let mut ty = y;
    for _ in 0..speed {
        let t = h.get(x, ty - 1);
        if !passable(h, p, t) {
            break;
        }
        ty -= 1;
        if !t.is_some_and(|t| t.is_air()) {
            break; // sinking through a fluid: one cell per tick
        }
    }
    if ty == y {
        return false;
    }
    c.vy = (c.vy + 1).min(MAX_FALL);
    c.flags = flags::with_rest(c.flags, 0);
    swap_to(h, x, y, x, ty, c);
    true
}

fn slide(h: &mut Hood, x: i32, y: i32, mut c: Cell, p: &MatPhys) -> bool {
    let d = h.rng.sign();
    for dx in [d, -d] {
        if passable(h, p, h.get(x + dx, y - 1)) {
            c.vy /= 2;
            c.flags = flags::with_rest(c.flags, 0);
            swap_to(h, x, y, x + dx, y - 1, c);
            return true;
        }
    }
    false
}

fn flow(h: &mut Hood, x: i32, y: i32, mut c: Cell, p: &MatPhys) -> bool {
    let first = if c.flags & flags::FLOW_LEFT != 0 { -1 } else { 1 };
    let reach = p.dispersion.max(1) as i32;
    // Under a column of liquid: pressure pushes it sideways, no budget needed.
    let pressured = h.get(x, y + 1).is_some_and(|a| h.mats.phys(a.material).kind == Kind::Liquid);
    let rest = if pressured { 0 } else { flags::rest(c.flags) };
    for dir in [first, -first] {
        let mut best = 0;
        let mut purposeful = false;
        for i in 1..=reach {
            // Sideways only into air. Layering (oil over water) happens by
            // sinking; sideways swaps at an interface would never end.
            if !h.get(x + dir * i, y).is_some_and(|t| t.is_air()) {
                break;
            }
            best = i;
            if passable(h, p, h.get(x + dir * i, y - 1)) {
                // Found a drop: go there, fall next tick.
                purposeful = true;
                break;
            }
        }
        if best > 0 && (purposeful || rest < flags::REST_LIMIT) {
            if dir < 0 { c.flags |= flags::FLOW_LEFT } else { c.flags &= !flags::FLOW_LEFT }
            // Spreading one way is free; only reversing (sloshing) spends the budget.
            let spent = if purposeful || pressured { 0 } else if dir == first { rest } else { rest + 1 };
            c.flags = flags::with_rest(c.flags, spent);
            c.vy = 0;
            swap_to(h, x, y, x + dir * best, y, c);
            return true;
        }
    }
    false
}

/// Could not move: drop any fall speed. Writes only if something changed.
fn settle(h: &mut Hood, x: i32, y: i32, mut c: Cell) -> bool {
    if c.vy != 0 {
        c.vy = 0;
        c.clock = h.clock;
        h.set(x, y, c);
    }
    true
}

/// Advance life by one decay step. Returns false if the cell expired (already replaced).
fn age(h: &mut Hood, x: i32, y: i32, c: &mut Cell, p: &MatPhys) -> bool {
    if p.life_max == 0 || !h.tick.is_multiple_of(p.decay_every as u64) {
        return true;
    }
    if c.life == 0 {
        let id = if h.rng.chance(p.decays_into_chance) { p.decays_into } else { MaterialId::AIR };
        let into = spawn(h, id);
        h.set(x, y, into);
        return false;
    }
    c.life -= 1;
    true
}

fn gas(h: &mut Hood, x: i32, y: i32, mut c: Cell, p: &MatPhys) {
    let before = c;
    if !age(h, x, y, &mut c, p) {
        return;
    }
    let free = |h: &Hood, tx: i32, ty: i32| h.get(tx, ty).is_some_and(|t| t.is_air());
    let d = h.rng.sign();
    // Billow: often drift up diagonally rather than rising in single file.
    if h.rng.chance(100) && free(h, x + d, y + 1) {
        return swap_to(h, x, y, x + d, y + 1, c);
    }
    if free(h, x, y + 1) {
        return swap_to(h, x, y, x, y + 1, c);
    }
    for dx in [d, -d] {
        if free(h, x + dx, y + 1) {
            return swap_to(h, x, y, x + dx, y + 1, c);
        }
    }
    let reach = 1 + (h.rng.next_u32() % p.dispersion.max(1) as u32) as i32;
    let mut best = 0;
    for i in 1..=reach {
        if !free(h, x + d * i, y) {
            break;
        }
        best = i;
    }
    if best > 0 {
        return swap_to(h, x, y, x + d * best, y, c);
    }
    if c != before {
        c.clock = h.clock;
        h.set(x, y, c);
    }
}

fn fire(h: &mut Hood, x: i32, y: i32, mut c: Cell, p: &MatPhys) {
    if !age(h, x, y, &mut c, p) {
        return;
    }
    // Flicker upwards now and then.
    if h.rng.chance(64) {
        let dx = h.rng.sign() * (h.rng.coin() as i32);
        if h.get(x + dx, y + 1).is_some_and(|t| t.is_air()) {
            return swap_to(h, x, y, x + dx, y + 1, c);
        }
    }
    c.clock = h.clock;
    h.set(x, y, c);
}
