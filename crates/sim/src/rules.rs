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
        // The chance grows with how far above it is: warm wood beside a
        // fire rarely catches from heat alone; wood in a furnace does.
        let excess = (t - p.ignites_at as i32) as u32;
        let chance = (p.flammability.max(1) as u32 * 16 * excess.min(HEAT_RAMP) / HEAT_RAMP).max(1);
        if t >= p.ignites_at as i32 + SURE_IGNITION_MARGIN || h.rng.chance4096(chance) {
            ignite(h, x, y, p);
            return true;
        }
        h.wake(x, y);
    }
    let (into, past) = if t >= p.above_at as i32 {
        (p.above_into, t - p.above_at as i32)
    } else if t <= p.below_at as i32 {
        (p.below_into, p.below_at as i32 - t)
    } else {
        return false;
    };
    // Latent heat: the further past, the sooner (and it waits awake).
    if p.latent > 0 {
        let d = (past as u64 + 1).min(1 << 16);
        let l = p.latent as u64;
        let chance = (d * d * 4096 / (l * l)).min(4096) as u32;
        if !h.rng.chance4096(chance) {
            h.wake(x, y);
            return false;
        }
    }
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

/// Over this many °C above its ignition point the chance to catch from heat
/// ramps up to the full rate (flammability /256 per tick).
const HEAT_RAMP: u32 = 60;
/// Above its ignition point by this much (°C), a material always catches.
const SURE_IGNITION_MARGIN: i32 = 250;
/// Flaming neighbours a flame needs to keep going for sure.
const COMPANY: u32 = 2;
/// Flaming neighbours a burning cell needs before it throws embers.
const BLAZE: u32 = 4;
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

/// Chance /4096 per tick that a burning cell lights a neighbour of material
/// `p` at vertical offset `dy`. Heat rises: fire catches upward much more
/// readily than sideways or down.
#[inline]
fn spread_chance(p: &MatPhys, dy: i32) -> u32 {
    let s = p.spread as u32;
    match dy {
        1.. => s * 2,
        0 => s,
        _ => s / 2,
    }
}

/// Flaming neighbours of a burning cell (8 around, same layer; smouldering,
/// charred ones don't count). How big the fire is right here: below
/// `COMPANY` a flame is on its own and may fizzle (`MatPhys::fizzles`);
/// from `BLAZE` it throws embers.
fn flaming_around(h: &Hood, x: i32, y: i32, back: bool) -> u32 {
    NEIGHBOURS8
        .iter()
        .filter(|&&(dx, dy)| {
            let n = if back { h.get_bg(x + dx, y + dy) } else { h.get(x + dx, y + dy) };
            n.is_some_and(|n| n.flags & flags::BURNING != 0 && !h.mats.is_charred(n))
        })
        .count() as u32
}

/// Fire in the playfield heats the background behind and beside it.
fn ignite_background_near(h: &mut Hood, x: i32, y: i32) {
    for (dx, dy) in [(0, 0), (0, 1), (-1, 0), (1, 0)] {
        heat_bg(h, x + dx, y + dy, FLAME_HEAT);
    }
}

/// Background fire is heat-driven (SPEC §3.8). Background cells hold heat
/// (relative to ambient) but don't conduct it; fire moves by radiating it.
/// A hot cell cools back toward ambient, 1/`BG_COOL` of its heat a tick.
const BG_COOL: i32 = 32;
/// A burning cell keeps itself this hot a tick (combustion); alone, it
/// settles at ~`SELF_HEAT` × `BG_COOL` (144 °C over ambient): below
/// `SUSTAIN`, so a lone flame may go out.
const SELF_HEAT: i32 = 3;
/// Below this (°C over ambient) a flame may go out (`MatPhys::fizzles`).
const SUSTAIN: i32 = 250;
/// Above this, it's a blaze: embers.
const BLAZE_HEAT: i32 = 420;
/// Heat flames in the playfield give the background behind and beside them.
const FLAME_HEAT: i16 = 5;
const BG_MAX_HEAT: i32 = 1200;
const RAD_BASE: i32 = 1;
const RAD_DIV: i32 = 3;

/// Heat a burning cell of `p` radiates to a neighbour `dy` above (per tick):
/// hotter-burning materials more, twice upward, half downward.
#[inline]
fn radiated(p: &MatPhys, charred: bool, dy: i32) -> i32 {
    let base = RAD_BASE + p.flammability as i32 / RAD_DIV; // wood 3, leaves 7
    let base = if charred { base / 2 } else { base };
    match dy {
        1.. => base * 2,
        0 => base,
        _ => base / 2,
    }
}

/// Add heat to a background cell (fire in front, a heat gun, an ember).
pub(crate) fn heat_bg(h: &mut Hood, x: i32, y: i32, amount: i16) {
    let Some(mut b) = h.get_bg(x, y) else { return };
    if b.is_air() {
        return;
    }
    b.heat = (b.heat as i32 + amount as i32).clamp(-300, BG_MAX_HEAT) as i16;
    h.set_bg(x, y, b);
}

/// One tick of a hot or burning background cell.
pub(crate) fn background(h: &mut Hood, x: i32, y: i32, mut b: Cell) {
    let bp = *h.mats.phys(b.material);
    let front = h.get(x, y);
    let burning = b.flags & flags::BURNING != 0;
    // Water in front puts it out and cools it.
    if let Some(f) = front
        && !f.is_air()
    {
        let fp = *h.mats.phys(f.material);
        if fp.kind == Kind::Liquid && fp.flammability == 0 && !fp.hot {
            b.flags &= !flags::BURNING;
            b.heat = b.heat.min(20);
            h.set_bg(x, y, b);
            return;
        }
        if burning && fp.flammability > 0 && f.flags & flags::BURNING == 0 && h.rng.chance(fp.flammability) {
            ignite(h, x, y, &fp);
        }
    }
    // Cooling toward ambient.
    let mut heat = b.heat as i32;
    heat -= if heat > 0 { (heat / BG_COOL).max(1) } else if heat < 0 { (heat / BG_COOL).min(-1) } else { 0 };
    if !burning {
        b.heat = heat as i16;
        let t = h.ambient(y) + heat;
        if bp.flammability > 0 && t >= bp.ignites_at as i32 {
            let excess = (t - bp.ignites_at as i32) as u32;
            let chance = (bp.flammability.max(1) as u32 * 16 * excess.min(HEAT_RAMP) / HEAT_RAMP).max(1);
            if t >= bp.ignites_at as i32 + SURE_IGNITION_MARGIN || h.rng.chance4096(chance) {
                b.flags |= flags::BURNING;
                b.life = bp.burn_time;
            }
        }
        h.set_bg(x, y, b);
        return;
    }
    let charred = h.mats.is_charred(b);
    heat = (heat + SELF_HEAT).min(BG_MAX_HEAT);
    // A flame that isn't kept hot may go out (smouldering wood glows on).
    if !charred && heat < SUSTAIN && bp.fizzles > 0 && h.rng.chance4096(bp.fizzles as u32) {
        b.flags &= !flags::BURNING;
        b.heat = heat as i16;
        h.set_bg(x, y, b);
        // It carries weight again: check what's around.
        h.note_broken_bg(x, y);
        return;
    }
    // Radiate into the neighbours.
    for (dx, dy) in NEIGHBOURS8 {
        let Some(n) = h.get_bg(x + dx, y + dy) else { continue };
        if n.is_air() {
            continue;
        }
        heat_bg(h, x + dx, y + dy, radiated(&bp, charred, dy) as i16);
    }
    b.heat = heat as i16;
    // Flames into the air in front, more the hotter it is here.
    if !charred && front.is_some_and(|f| f.is_air()) && h.rng.chance((heat / 12).clamp(4, 60) as u8) {
        let flame = spawn(h, h.mats.fire());
        h.set(x, y, flame);
    }
    // Only a blaze throws embers.
    if !charred && heat > BLAZE_HEAT && h.rng.chance(EMBER_CHANCE / 2 + 1) && h.get(x, y + 1).is_some_and(|a| a.is_air()) {
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
            if bp.burns_into != MaterialId::AIR
                && h.rng.chance(bp.burns_into_chance / 3)
                && h.get(x, y).is_some_and(|f| f.is_air())
            {
                let mut left = spawn(h, bp.burns_into);
                left.heat = (heat / 2) as i16;
                h.set(x, y, left);
            }
            return;
        }
        b.life -= 1;
        // Charred through: it stops holding things up, so check what it held.
        if b.life + 1 == bp.charred_life {
            h.note_broken_bg(x, y);
        }
    }
    h.set_bg(x, y, b);
}

/// One tick of a burning cell. Returns true if it burned out (replaced).
fn burn(h: &mut Hood, x: i32, y: i32, c: &mut Cell, p: &MatPhys) -> bool {
    // Doused: a non-flammable, non-hot liquid touching it (water, not oil or
    // lava). Not from below if it's a liquid itself: burning oil floats on
    // the water it burns on.
    for (dx, dy) in NEIGHBOURS {
        if dy < 0 && p.kind == Kind::Liquid {
            continue;
        }
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
    // A lone flame on a log may just go out (charred wood left as charcoal).
    if p.fizzles > 0 && h.rng.chance4096(p.fizzles as u32) && flaming_around(h, x, y, false) < COMPANY {
        if h.mats.is_charred(*c) && p.chars_into != MaterialId::AIR {
            let mut coal = spawn(h, p.chars_into);
            coal.heat = c.heat.min(150);
            h.set(x, y, coal);
            h.note_broken(x, y);
            return true;
        }
        c.flags &= !flags::BURNING;
        c.heat = c.heat.min(150);
        h.set(x, y, *c);
        h.note_broken(x, y);
        return false;
    }
    // Charred, it smoulders: no more flames, no more spreading.
    let flaming = !h.mats.is_charred(*c);
    // Spread, diagonals included (flames lick around corners), mostly upward.
    for (dx, dy) in NEIGHBOURS8 {
        if !flaming {
            break;
        }
        let Some(n) = h.get(x + dx, y + dy) else { continue };
        if n.is_air() || n.flags & flags::BURNING != 0 {
            continue;
        }
        let np = *h.mats.phys(n.material);
        if np.flammability > 0 && h.rng.chance4096(spread_chance(&np, dy)) {
            ignite(h, x + dx, y + dy, &np);
        }
    }
    if flaming {
        ignite_background_near(h, x, y);
    }
    // Flames and smoke into the air around it, mostly upward.
    let (dx, dy) = [(0, 1), (0, 1), (-1, 0), (1, 0)][(h.rng.next_u32() % 4) as usize];
    if flaming && h.get(x + dx, y + dy).is_some_and(|a| a.is_air()) {
        let roll = h.rng.next_u8();
        let puff = if roll < 70 { Some(h.mats.fire()) } else if roll < 82 { h.mats.id("smoke") } else { None };
        if let Some(m) = puff.filter(|m| *m != MaterialId::AIR) {
            let cell = spawn(h, m);
            h.set(x + dx, y + dy, cell);
        }
    }
    // Embers: burning specks thrown up that can start fires where they land.
    // Only a blaze throws embers, not a lone flame.
    if flaming && h.rng.chance(EMBER_CHANCE) && h.get(x, y + 1).is_some_and(|a| a.is_air()) && flaming_around(h, x, y, false) >= BLAZE {
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
        if id == MaterialId::AIR && p.vapour {
            // Faded, not gone: it rises to the clouds.
            h.note_vapour(x);
        }
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
