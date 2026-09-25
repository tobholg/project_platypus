//! Per-kind update rules. Integer-only; all randomness from `Hood::rng`.
//!
//! A cell that cannot do anything writes nothing, so its region goes to sleep.

use crate::cell::{Cell, flags};
use crate::material::{Kind, MatPhys, MaterialId};
use crate::particles::{Landing, Particle};
use crate::step::Hood;

/// Fall-speed cap; a falling cell moves `1 + vy / 4` cells per tick (max 8).
/// Must stay well under `MAX_REACH`.
pub(crate) const MAX_FALL: i8 = 28;

const NEIGHBOURS: [(i32, i32); 4] = [(0, 1), (0, -1), (-1, 0), (1, 0)];
/// -1 or 1, leaning downwind (`wind` in -1..1).
#[inline]
fn downwind_sign(h: &mut Hood) -> i32 {
    if (h.rng.next_u8() as f32) < 128.0 + h.wind * 110.0 { 1 } else { -1 }
}

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
        Kind::Plant => plant(h, x, y),
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
            // Plants are crushed by whatever falls or flows into them.
            tp.kind == Kind::Plant || matches!(tp.kind, Kind::Liquid | Kind::Gas | Kind::Fire) && tp.density < mover.density
        }
    }
}

/// Move `c` from (x,y) to (tx,ty); whatever was there takes its place.
/// Something flammable moving into flames catches fire instead of pushing them away.
#[inline(always)]
fn swap_to(h: &mut Hood, x: i32, y: i32, tx: i32, ty: i32, mut c: Cell) {
    let mut displaced = h.get(tx, ty).expect("caller checked the target is loaded");
    if !displaced.is_air() && h.mats.phys(displaced.material).kind == Kind::Plant {
        displaced = Cell::AIR; // crushed
    }
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
    if t >= p.ignites_at as i32 && c.flags & flags::BURNING == 0 {
        // Warm enough to catch: a chance per tick set by flammability, certain
        // only well above the ignition point. (Certain ignition at the
        // threshold made heat carry every fire across every meadow.)
        if t >= p.ignites_at as i32 + SURE_IGNITION_MARGIN || h.rng.chance(p.flammability.max(1)) {
            ignite(h, x, y, p);
            return true;
        }
        h.wake(x, y);
    }
    let into = if t >= p.above_at as i32 {
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

/// Above its ignition point by this much (°C), a material always catches.
const SURE_IGNITION_MARGIN: i32 = 250;
/// Chance /256 per tick that a burning cell with air above throws an ember.
const EMBER_CHANCE: u8 = 3;
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

/// Heat rises: fire catches upward much more readily than sideways or down.
#[inline]
fn spread_chance(flammability: u8, dy: i32) -> u8 {
    match dy {
        1.. => flammability.saturating_mul(2),
        0 => flammability,
        _ => flammability / 2,
    }
}

/// Set a background cell burning (it stays in place and burns down).
pub(crate) fn ignite_bg(h: &mut Hood, x: i32, y: i32, p: &MatPhys) {
    let Some(mut b) = h.get_bg(x, y) else { return };
    if b.is_air() || b.flags & flags::BURNING != 0 {
        return;
    }
    b.flags |= flags::BURNING;
    b.life = p.burn_time;
    h.set_bg(x, y, b);
}

/// Fire in the playfield catching the background behind and beside it.
/// Flames are brief and move, so each lights what's behind it at a third of
/// the material's rate; otherwise rising flames race up a trunk ahead of the fire.
fn ignite_background_near(h: &mut Hood, x: i32, y: i32) {
    for (dx, dy) in [(0, 0), (0, 1), (-1, 0), (1, 0)] {
        let Some(b) = h.get_bg(x + dx, y + dy) else { continue };
        if b.is_air() || b.flags & flags::BURNING != 0 {
            continue;
        }
        let bp = *h.mats.phys(b.material);
        if bp.flammability > 0 && h.rng.chance(bp.flammability / 3 + 1) {
            ignite_bg(h, x + dx, y + dy, &bp);
        }
    }
}

/// One tick of a burning background cell: spreads through the background,
/// lights the playfield in front of it, puts flames into the air in front,
/// throws embers, and burns away (which may leave something hanging).
pub(crate) fn burn_background(h: &mut Hood, x: i32, y: i32, mut b: Cell) {
    let front = h.get(x, y);
    if let Some(f) = front
        && !f.is_air()
    {
        let fp = *h.mats.phys(f.material);
        if fp.kind == Kind::Liquid && fp.flammability == 0 && !fp.hot {
            // Water in front puts it out; charred by then, it's charcoal.
            let bp = h.mats.phys(b.material);
            if h.mats.is_charred(b) && bp.chars_into != MaterialId::AIR {
                let coal = spawn(h, bp.chars_into);
                h.set_bg(x, y, coal);
            } else {
                b.flags &= !flags::BURNING;
                h.set_bg(x, y, b);
            }
            return;
        }
        if fp.flammability > 0 && f.flags & flags::BURNING == 0 && h.rng.chance(fp.flammability) {
            ignite(h, x, y, &fp);
        }
    }
    for (dx, dy) in NEIGHBOURS8 {
        let Some(n) = h.get_bg(x + dx, y + dy) else { continue };
        if n.is_air() || n.flags & flags::BURNING != 0 {
            continue;
        }
        let np = *h.mats.phys(n.material);
        if np.flammability > 0 && h.rng.chance(spread_chance(np.flammability, dy)) {
            ignite_bg(h, x + dx, y + dy, &np);
        }
    }
    if front.is_some_and(|f| f.is_air()) && h.rng.chance(40) {
        let flame = spawn(h, h.mats.fire());
        h.set(x, y, flame);
    }
    if h.rng.chance(EMBER_CHANCE / 2 + 1) && h.get(x, y + 1).is_some_and(|a| a.is_air()) {
        let vx = (h.rng.next_u8() as f32 / 255.0 - 0.5) * 0.9 + h.wind * 0.3;
        let vy = 0.35 + h.rng.next_u8() as f32 / 255.0 * 0.5;
        let life = 40 + h.rng.next_u8() as u16 / 2;
        let ember = Particle { gravity: 0.06, ..Particle::new(h.centre(x, y + 1), [vx, vy], b, life, Landing::Ember) };
        h.emit(ember);
    }
    if h.tick.is_multiple_of(4) {
        if b.life == 0 {
            h.set_bg(x, y, Cell::AIR);
            h.note_broken_bg(x, y);
            // Some of what's left (charcoal from wood) drops out in front, a
            // third as often as in the playfield: a burnt forest leaves some,
            // not a carpet.
            let bp = *h.mats.phys(b.material);
            if bp.burns_into != MaterialId::AIR
                && h.rng.chance(bp.burns_into_chance / 3)
                && h.get(x, y).is_some_and(|f| f.is_air())
            {
                let mut left = spawn(h, bp.burns_into);
                left.heat = b.heat / 2;
                h.set(x, y, left);
            }
            return;
        }
        b.life -= 1;
        // Charred through: it stops holding things up, so check what it held.
        if b.life + 1 == h.mats.phys(b.material).charred_life {
            h.note_broken_bg(x, y);
        }
    }
    h.set_bg(x, y, b);
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
                // Put out after charring: what's left is charcoal.
                if h.mats.is_charred(*c) && p.chars_into != MaterialId::AIR {
                    let mut coal = spawn(h, p.chars_into);
                    coal.heat = c.heat.min(120);
                    h.set(x, y, coal);
                    return true;
                }
                c.flags &= !flags::BURNING;
                c.heat = c.heat.min(120);
                h.set(x, y, *c);
                return false;
            }
        }
    }
    // Spread, diagonals included (flames lick around corners), mostly upward.
    for (dx, dy) in NEIGHBOURS8 {
        let Some(n) = h.get(x + dx, y + dy) else { continue };
        if n.is_air() || n.flags & flags::BURNING != 0 {
            continue;
        }
        let np = *h.mats.phys(n.material);
        if np.flammability > 0 && h.rng.chance(spread_chance(np.flammability, dy)) {
            ignite(h, x + dx, y + dy, &np);
        }
    }
    ignite_background_near(h, x, y);
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
    // Embers: burning specks thrown up that can start fires where they land.
    if h.rng.chance(EMBER_CHANCE) && h.get(x, y + 1).is_some_and(|a| a.is_air()) {
        let vx = (h.rng.next_u8() as f32 / 255.0 - 0.5) * 0.9;
        let vy = 0.35 + h.rng.next_u8() as f32 / 255.0 * 0.5;
        let life = 40 + h.rng.next_u8() as u16 / 2;
        let ember = Particle { gravity: 0.06, ..Particle::new(h.centre(x, y + 1), [vx, vy], *c, life, Landing::Ember) };
        h.emit(ember);
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
        // Charred through: it stops holding things up, so check what it held.
        if c.life + 1 == p.charred_life && p.kind == Kind::Static {
            h.note_broken(x, y);
        }
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
            // Sideways only into air (or plants, which get washed away).
            // Layering (oil over water) happens by sinking; sideways swaps at
            // an interface would never end.
            if !h.get(x + dir * i, y).is_some_and(|t| t.is_air() || h.mats.phys(t.material).kind == Kind::Plant) {
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

/// A plant needs something under it: ground, or more plant. Otherwise it
/// withers (and the one above it will notice next tick).
fn plant(h: &mut Hood, x: i32, y: i32) {
    let supported = h.get(x, y - 1).is_none_or(|b| {
        !b.is_air() && matches!(h.mats.phys(b.material).kind, Kind::Static | Kind::Powder | Kind::Plant)
    });
    if !supported {
        h.set(x, y, Cell::AIR);
    }
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
    let d = downwind_sign(h);
    // Rise unevenly. If every cell of a cloud moved every tick, whole rows
    // would move in lockstep and a thick cloud would show as horizontal
    // stripes. Some ticks a cell hovers or drifts sideways instead (mostly
    // downwind), so a cloud churns and billows. Only while it can still rise:
    // gas with nowhere to go writes nothing and sleeps (sealed methane pockets).
    let roll = h.rng.next_u32() & 255;
    let rising = free(h, x, y + 1) || free(h, x - 1, y + 1) || free(h, x + 1, y + 1);
    if roll < 96 && rising {
        let dx = if roll < 72 { d } else { -d };
        if roll < 88 && free(h, x + dx, y) {
            return swap_to(h, x, y, x + dx, y, c);
        }
        // Hover, but stay awake.
        c.clock = h.clock;
        h.set(x, y, c);
        return;
    }
    // Billow: often drift up diagonally rather than rising in single file.
    let side = if h.rng.chance(170) { d } else { -d };
    if h.rng.chance(100) && free(h, x + side, y + 1) {
        return swap_to(h, x, y, x + side, y + 1, c);
    }
    if free(h, x, y + 1) {
        return swap_to(h, x, y, x, y + 1, c);
    }
    for dx in [side, -side] {
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
    } else if p.life_max != 0 {
        // Stuck but still ageing (it only ages every `decay_every` ticks):
        // stay awake, or steam under a cave roof never condenses.
        h.wake(x, y);
    }
}

fn fire(h: &mut Hood, x: i32, y: i32, mut c: Cell, p: &MatPhys) {
    if !age(h, x, y, &mut c, p) {
        return;
    }
    // Flames lick whatever is behind them.
    ignite_background_near(h, x, y);
    // Flicker upwards now and then, leaning downwind.
    if h.rng.chance(64) {
        let dx = downwind_sign(h) * (h.rng.coin() as i32);
        if h.get(x + dx, y + 1).is_some_and(|t| t.is_air()) {
            return swap_to(h, x, y, x + dx, y + 1, c);
        }
    }
    c.clock = h.clock;
    h.set(x, y, c);
}
