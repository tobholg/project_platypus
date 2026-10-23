//! The glutton: a slime (`goo`) that drinks what it's struck with and
//! takes it on. Seven things: fire, frost, lightning, acid, poison, light
//! (radiant) and the void. Each turns its goo (hot, cold, charged, acid,
//! venom, blinding, ink), its touch and its light, and gives it an attack
//! (`glut_strike`), each a third of its charge; spent, or left unused a
//! while, it's itself again. What it drinks doesn't hurt it.
//!
//! What undoes what it holds goes off with it: fire and frost explode,
//! light and the void implode (pulling everything in), fire burns poison
//! off in a flash. Water douses its fire, dilutes its acid; its lightning
//! runs into the water, its frost freezes it. Oil coats it, and fire on an
//! oiled glutton flares up.

use bevy::prelude::*;
use platypus_sim::cell::flags::HELD;
use platypus_sim::{Cell, CellPos, Kind};

use super::{Goo, Spiking};
use crate::creatures::{Harm, Health, Kinematics};
use crate::world::SimWorld;

/// What the glutton's taken on, and how much of it's left (draining away).
#[derive(Component, Default)]
pub struct Glutton {
    pub took: Option<Harm>,
    charge: f32,
    /// When it took it on.
    took_at: f32,
    /// Its attack with what it holds: gathering it (since when), then
    /// letting it go (shots fired); and when it may again.
    striking: Option<(f32, u32)>,
    ready: f32,
    /// What it let go of just now, and till when it doesn't take that on
    /// again (its own lightning through its goo, its lava round it).
    own: Option<(Harm, f32)>,
    /// Oiled till then (s): fire on it flares up; when it last did.
    oiled: f32,
    flared: Option<f32>,
    /// When it last looked at what it's in, and when it may next let what
    /// it holds into the water.
    soaked: f32,
    shed: f32,
}

impl Glutton {
    /// How much of what it holds is left (1 full, 0 spent; more, oiled
    /// fire).
    pub fn charge(&self) -> f32 {
        self.charge
    }

    /// Oiled now.
    pub fn oiled(&self, now: f32) -> bool {
        now < self.oiled
    }
}

/// What the glutton drinks.
const DRUNK: [Harm; 7] = [Harm::Fire, Harm::Frost, Harm::Storm, Harm::Acid, Harm::Poison, Harm::Radiant, Harm::Void];
/// What it takes on: a strike of it at least this much (a spell drunk, a
/// blow, a bolt; not the trickle of flames or cold it's in).
const STRIKE: f32 = 2.0;
/// What it holds drains away in this long (s), left unused.
const DRAIN: f32 = 20.0;
/// A spell landing this near its goo (cells) is drunk.
const DRINKS: f32 = 4.0;
/// Taking something on heals it this share of its health.
const FEEDS: f32 = 0.15;
/// Fire and frost go off this hard (radius, power).
const UNDONE: (f32, f32) = (34.0, 45.0);
/// Light and the void implode: what's this near is pulled in this hard
/// and hurt this much; it loses this share of its health.
const IMPLODE: (f32, f32, f32, f32) = (80.0, 420.0, 12.0, 0.25);
/// Fire burning poison off: a flash this big.
const BURNT_OFF: (f32, f32) = (26.0, 30.0);
/// What it's in (cells touching its goo), looked at this often (s); this
/// many of one thing and it's in it.
const BATH_EVERY: f32 = 0.1;
const BATH: u32 = 10;
/// What it's in doesn't change what it holds for this long after it took
/// it on (s).
const SETTLE: f32 = 1.0;
/// Its own lava, acid or miasma round it doesn't feed it again for this
/// long after it let it go (s).
const OWN_POOL: f32 = 6.0;
/// Oiled this long (s); fire on it flares up, and its fire's this strong.
const OILED: f32 = 30.0;
const OILED_CHARGE: f32 = 1.5;
/// It flares up no oftener than this (s).
const FLARE_EVERY: f32 = 10.0;
/// A flare-up: how far it burns (cells), how much.
const FLARE_UP: (f32, f32) = (46.0, 14.0);
/// In water, its lightning runs into it, its frost freezes it: so often
/// (s), each costing this much of its charge.
const SHED_EVERY: f32 = 1.4;
const SHED_COST: f32 = 0.12;

/// What's touching its goo, by kind (cells), and a cell of water it's in.
#[derive(Default)]
struct Bath {
    fire: u32,
    acid: u32,
    miasma: u32,
    water: u32,
    oil: u32,
}

fn bath(sim: &SimWorld, goo: &Goo) -> Bath {
    let mats = sim.world.materials();
    let (water, oil, miasma) = (mats.id("water"), mats.id("oil"), mats.id("miasma"));
    let mut b = Bath::default();
    for &p in &goo.placed {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let Some(c) = sim.world.get(p.offset(dx, dy)) else { continue };
            if c.is_air() || c.flags & HELD != 0 {
                continue;
            }
            let ph = mats.phys(c.material);
            if Some(c.material) == water {
                b.water += 1;
            } else if Some(c.material) == oil {
                b.oil += 1;
            } else if Some(c.material) == miasma {
                b.miasma += 1;
            } else if ph.kind == Kind::Fire || (ph.hot && ph.kind == Kind::Liquid) {
                b.fire += 1;
            } else if ph.corrosive > 0 && ph.kind == Kind::Liquid {
                b.acid += 1;
            }
        }
    }
    b
}

/// What it holds: its goo, its touch (kind, damage), its light.
fn look(took: Option<Harm>, own: &str) -> (&str, Harm, f32, Option<[f32; 3]>) {
    match took {
        Some(Harm::Fire) => ("ember_goo", Harm::Fire, 11.0, Some(glow_of(Harm::Fire))),
        Some(Harm::Frost) => ("frost_goo", Harm::Frost, 9.0, Some(glow_of(Harm::Frost))),
        Some(Harm::Storm) => ("spark_goo", Harm::Storm, 12.0, Some(glow_of(Harm::Storm))),
        Some(Harm::Acid) => ("acid_goo", Harm::Acid, 10.0, Some(glow_of(Harm::Acid))),
        Some(Harm::Poison) => ("venom_goo", Harm::Poison, 8.0, Some(glow_of(Harm::Poison))),
        Some(Harm::Radiant) => ("radiant_goo", Harm::Radiant, 12.0, Some(glow_of(Harm::Radiant))),
        Some(Harm::Void) => ("void_goo", Harm::Void, 12.0, Some(glow_of(Harm::Void))),
        _ => (own, Harm::Blunt, 7.0, None),
    }
}

/// The light of what it holds (light: enough to light a cave).
fn glow_of(harm: Harm) -> [f32; 3] {
    match harm {
        Harm::Fire => [1.4, 0.7, 0.25],
        Harm::Frost => [0.6, 0.9, 1.3],
        Harm::Acid => [0.5, 1.3, 0.35],
        Harm::Poison => [0.8, 0.45, 1.0],
        Harm::Radiant => [2.6, 2.4, 1.9],
        Harm::Void => [0.35, 0.12, 0.6],
        _ => [0.7, 1.1, 1.5],
    }
}

/// The glutton takes on what it's struck with (a spell drunk, a blow, a
/// bolt, what it's in), and what it holds meets what it's in.
#[allow(clippy::type_complexity)]
pub fn glut(
    mut commands: Commands,
    time: Res<Time>,
    mut sim: ResMut<SimWorld>,
    mut q: Query<(Entity, &mut Glutton, &mut Goo, &Kinematics, &crate::creatures::body::animation::Animator, Option<&mut crate::combat::Touch>, &mut Health)>,
    bodies: Query<(Entity, &Kinematics, Option<&crate::creatures::Team>), Without<Glutton>>,
    (mut booms, mut hits, mut sounds, mut flares): (MessageWriter<crate::fx::Explosion>, MessageWriter<crate::combat::Hit>, MessageWriter<crate::sound::PlaySound>, MessageWriter<crate::fx::Flare>),
    (mut took, mut landed): (MessageReader<crate::creatures::Took>, MessageReader<crate::magic::Landed>),
    (mut drinkers, mut sparks, mut trauma): (ResMut<crate::magic::Drinkers>, ResMut<crate::vfx::Sparks>, ResMut<crate::fx::Trauma>),
) {
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    let spells: Vec<crate::magic::Landed> = landed.read().copied().collect();
    drinkers.0.clear();
    // (What each took this tick, however it came: tallied, all of it.)
    let mut taken: std::collections::HashMap<Entity, [f32; 11]> = std::collections::HashMap::new();
    for t in took.read() {
        if q.contains(t.target) {
            taken.entry(t.target).or_insert([0.0; 11])[t.harm as usize] += t.meant;
        }
    }
    for (e, mut g, mut goo, k, anim, touch, mut health) in &mut q {
        let mut got = taken.get(&e).copied().unwrap_or([0.0; 11]);
        // (A spell of it come down on its goo: drunk (`magic::land`).)
        for l in spells.iter().filter(|l| l.hit == Some(e)) {
            got[l.harm as usize] += STRIKE;
        }
        let (lo, hi) = goo.extent.unwrap_or((k.body.pos - k.body.half, k.body.pos + k.body.half));
        drinkers.0.push(crate::magic::Drinker { who: e, lo: lo - DRINKS, hi: hi + DRINKS, harms: &DRUNK });
        // What it's in: fire or lava, acid, miasma; water, oil.
        let (mut water, mut oil) = (false, false);
        // (Not just after it took something on: a lightning bolt's sparks
        // on its goo aren't fire to it.)
        if now - g.soaked >= BATH_EVERY && (g.took.is_none() || now - g.took_at >= SETTLE) {
            g.soaked = now;
            let b = bath(&sim, &goo);
            // (Not its own lava, acid or miasma round it, a while.)
            let own_pool = g.own.filter(|o| now < o.1 + OWN_POOL).map(|o| o.0);
            // (Flames in water don't count: its own lightning's sparks.)
            let fire = if b.water >= BATH { 0 } else { b.fire };
            for (n, harm) in [(fire, Harm::Fire), (b.acid, Harm::Acid), (b.miasma, Harm::Poison)] {
                if n >= BATH && own_pool != Some(harm) {
                    got[harm as usize] += STRIKE;
                }
            }
            water = b.water >= BATH;
            oil = b.oil >= BATH;
        }
        let before = g.took;
        // (Oil on its fire burns: it isn't oiled, it flares up.)
        if oil && g.took != Some(Harm::Fire) {
            g.oiled = now + OILED;
        }
        // (Not what it holds already: its own hot or cold goo would keep it
        // topped up, and it'd never drain; nor what it just let go of.)
        let own = g.own.filter(|o| now < o.1).map(|o| o.0);
        let struck: Vec<Harm> = DRUNK.into_iter().filter(|h| got[*h as usize] >= STRIKE && g.took != Some(*h) && own != Some(*h)).collect();
        let pos = k.body.pos;
        for harm in struck {
            match (g.took, harm) {
                // Fire and frost together: they go off.
                (Some(Harm::Fire), Harm::Frost) | (Some(Harm::Frost), Harm::Fire) => {
                    g.took = None;
                    booms.write(crate::fx::Explosion { at: pos, radius: UNDONE.0, power: UNDONE.1 });
                    sim.queue(platypus_sim::WorldEdit::Explode { center: CellPos::from_world(pos.x, pos.y), radius: 6, power: 30 });
                    sounds.write(crate::sound::PlaySound::at("boom", pos).volume(0.8));
                }
                // Light and the void: they implode, pulling all in.
                (Some(Harm::Radiant), Harm::Void) | (Some(Harm::Void), Harm::Radiant) => {
                    g.took = None;
                    for (b, bk, _) in &bodies {
                        let d = pos - bk.body.pos;
                        if d.length() <= IMPLODE.0 {
                            let dir = d.normalize_or(Vec2::Y);
                            hits.write(crate::combat::Hit { by: None, target: b, damage: IMPLODE.2, harm: Harm::Void, knock: dir * IMPLODE.1, stun: 0.3, at: bk.body.pos, dir, weight: 1.5, crit: false });
                        }
                    }
                    health.hp -= health.max * IMPLODE.3;
                    let inward = implosion();
                    for i in 0..36 {
                        let dir = Vec2::from_angle(i as f32 / 36.0 * std::f32::consts::TAU);
                        sparks.emit(&inward, 5, pos + dir * IMPLODE.0 * 0.8, -dir, Vec2::ZERO);
                    }
                    flares.write(crate::fx::Flare { at: pos, color: [1.6, 1.2, 2.2], life: 0.7 });
                    trauma.0 = (trauma.0 + 0.5).min(1.0);
                    sounds.write(crate::sound::PlaySound::at("boom", pos).pitch(0.45).volume(0.9));
                }
                // Fire on poison: burnt off, in a flash.
                (Some(Harm::Poison), Harm::Fire) => {
                    g.took = None;
                    g.own = Some((Harm::Fire, now + OWN));
                    booms.write(crate::fx::Explosion { at: pos, radius: BURNT_OFF.0, power: BURNT_OFF.1 });
                    sim.queue(platypus_sim::WorldEdit::Ignite { center: CellPos::from_world(pos.x, pos.y), radius: 12 });
                    sounds.write(crate::sound::PlaySound::at("ignite", pos));
                }
                _ => {
                    g.took = Some(harm);
                    g.took_at = now;
                    g.charge = 1.0;
                    // (It feeds on it.)
                    health.hp = (health.hp + health.max * FEEDS).min(health.max);
                }
            }
        }
        // Oiled, with fire in it (or oil on its fire): it flares up (the
        // oil burnt off), and its fire's the stronger; once a while.
        if g.took == Some(Harm::Fire) && (now < g.oiled || oil) && g.flared.is_none_or(|f| now - f >= FLARE_EVERY) {
            g.oiled = 0.0;
            g.flared = Some(now);
            g.charge = g.charge.max(OILED_CHARGE);
            flare_up(&mut sim, pos, (lo, hi), &bodies, &mut hits);
            sparks.emit(&eruption_fountain(), 260, Vec2::new(pos.x, hi.y), Vec2::Y, Vec2::ZERO);
            flares.write(crate::fx::Flare { at: pos, color: [2.4, 1.2, 0.4], life: 0.8 });
            trauma.0 = (trauma.0 + 0.45).min(1.0);
            sounds.write(crate::sound::PlaySound::at("ignite", pos).volume(1.0));
            sounds.write(crate::sound::PlaySound::at("boom", pos).pitch(1.3).volume(0.6));
        }
        // In water: what it holds meets it.
        if water && now >= g.shed {
            match g.took {
                // Its fire put out: steam.
                Some(Harm::Fire) => {
                    g.took = None;
                    if let Some(steam) = sim.world.materials().id("steam") {
                        sim.queue(platypus_sim::WorldEdit::Paint { center: CellPos::from_world(pos.x, hi.y + 4.0), radius: 9, material: steam, overwrite: false });
                        sim.world.puff([pos.x, hi.y], Cell::new(steam, 0), 40, 2.2);
                    }
                    sounds.write(crate::sound::PlaySound::at("vent", pos));
                }
                // Its acid diluted away.
                Some(Harm::Acid) => {
                    g.took = None;
                    sounds.write(crate::sound::PlaySound::at("pour", pos).pitch(0.8));
                }
                // Its lightning runs into the water: all of it's charged
                // (and what's in it hurt).
                Some(Harm::Storm) => {
                    let from = CellPos::from_world(pos.x, hi.y + 6.0);
                    sim.world.zap(from, CellPos::from_world(pos.x, pos.y));
                    // (The flames its spark leaves aren't fire to feed on.)
                    g.own = Some((Harm::Fire, now + OWN));
                    sounds.write(crate::sound::PlaySound::at("zap", pos).pitch(0.7));
                    spend(&mut g, SHED_COST);
                    g.shed = now + SHED_EVERY;
                }
                // Its frost freezes the water round it.
                Some(Harm::Frost) => {
                    let r = ((hi.x - lo.x).max(hi.y - lo.y) * 0.5 + 8.0) as i32;
                    sim.queue(platypus_sim::WorldEdit::Heat { center: CellPos::from_world(pos.x, pos.y), radius: r, amount: -500 });
                    sounds.write(crate::sound::PlaySound::at("pour", pos).pitch(1.9));
                    spend(&mut g, SHED_COST);
                    g.shed = now + SHED_EVERY;
                }
                _ => {}
            }
        }
        // Oiled: drips of it.
        if now < g.oiled
            && (now * 7.0).fract() < dt * 7.0
            && let Some(m) = sim.world.materials().id("oil")
        {
            sim.world.puff([pos.x + (now * 13.0).sin() * (hi.x - lo.x) * 0.4, lo.y + 1.0], Cell::new(m, 0), 2, 0.3);
        }
        if g.took.is_some() {
            g.charge -= dt / DRAIN;
            if g.charge <= 0.0 {
                g.took = None;
            }
        }
        if g.took.is_none() {
            g.charge = 0.0;
        }
        if g.took == before {
            continue;
        }
        let (goo_name, harm, damage, light) = look(g.took, anim.def.goo.as_ref().map_or("prism_goo", |d| d.material.as_str()));
        if let Some(m) = sim.world.materials().id(goo_name) {
            goo.turn(m);
        }
        if let Some(mut t) = touch {
            t.harm = harm;
            t.damage = damage;
        }
        match light {
            Some(color) => {
                commands.entity(e).insert(crate::light::LightSource { color, flicker: 0.15 });
            }
            None => {
                commands.entity(e).remove::<crate::light::LightSource>();
            }
        }
        sounds.write(crate::sound::PlaySound::at("pour", pos).pitch(1.4));
    }
}

/// Spend `cost` of its charge (spent: it drains away the next moment, and
/// it's itself again).
fn spend(g: &mut Glutton, cost: f32) {
    g.charge = if g.charge - cost <= 0.02 { 1e-5 } else { g.charge - cost };
}

/// An implosion's look: motes rushing in.
fn implosion() -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter {
        count: 0.0,
        life: (0.18, 0.3),
        colors: vec![(255, 255, 255), (220, 200, 255), (150, 90, 230), (40, 10, 70)],
        speed: 300.0,
        spread: 0.15,
        gravity: 0.0,
        drag: 0.0,
        size: 1.4,
        jitter: 0.0,
        glow: true,
    }
}

/// Oil on it catching: flame thrown up and round, what's near burnt.
fn flare_up(sim: &mut SimWorld, pos: Vec2, reach: (Vec2, Vec2), bodies: &Query<(Entity, &Kinematics, Option<&crate::creatures::Team>), Without<Glutton>>, hits: &mut MessageWriter<crate::combat::Hit>) {
    if let Some(fire) = sim.world.materials().id("fire") {
        sim.world.spray([pos.x, reach.1.y + 2.0], [0.0, 1.0], fire, 220, 5.0, 1.5);
    }
    sim.queue(platypus_sim::WorldEdit::Ignite { center: CellPos::from_world(pos.x, pos.y), radius: 22 });
    burn_near(pos, reach, FLARE_UP, bodies, hits);
}

/// What's within `(reach, damage)` of its goo is burnt and thrown up and
/// back (nearer, harder).
fn burn_near(pos: Vec2, goo: (Vec2, Vec2), (reach, damage): (f32, f32), bodies: &Query<(Entity, &Kinematics, Option<&crate::creatures::Team>), Without<Glutton>>, hits: &mut MessageWriter<crate::combat::Hit>) {
    for (e, bk, team) in bodies.iter() {
        // (Not its own side.)
        if team == Some(&crate::creatures::Team::Enemy) {
            continue;
        }
        let near = ((bk.body.pos.clamp(goo.0, goo.1) - bk.body.pos).abs() - bk.body.half).max(Vec2::ZERO).length();
        if near > reach {
            continue;
        }
        let f = 1.0 - near / reach * 0.5;
        let dir = Vec2::new((bk.body.pos.x - pos.x).signum(), 1.2).normalize();
        hits.write(crate::combat::Hit { by: None, target: e, damage: damage * f, harm: Harm::Fire, knock: dir * 300.0, stun: 0.2, at: bk.body.pos, dir, weight: 1.5, crit: false });
    }
}

/// The glutton's attacks, with what it holds: each spends this much of its
/// charge (three, and it's spent); it gathers it this long first, glowing,
/// holding still (to be read); between them, this long.
const STRIKE_COST: f32 = 0.34;
const STRIKE_TELL: f32 = 0.6;
const STRIKE_EVERY: f32 = 2.6;
/// After an attack it doesn't take on what it let go of for so long (s).
const OWN: f32 = 1.5;
/// Frost and acid: shots this far apart (s), fanned this much (degrees).
const SHOTS: (u32, f32) = (3, 0.32);
const FAN: f32 = 7.0;
/// Fire: how far its eruption reaches (cells), its hurt; lava lobbed at
/// them, lava and flame thrown straight up.
const ERUPTION: (f32, f32) = (58.0, 20.0);
const ERUPTION_MATTER: (usize, usize, usize) = (95, 55, 160);
/// Light: its ray swept this long (s), from this far above them to as far
/// below (degrees).
const RAY: (f32, f32) = (0.9, 26.0);
/// The void: it blinks beside them, then springs at them this hard.
const POUNCE: (f32, f32) = (110.0, 170.0);

/// How near it attacks from (cells), by what it holds.
fn range_of(harm: Harm) -> f32 {
    match harm {
        Harm::Storm => 200.0,
        Harm::Frost => 190.0,
        Harm::Acid => 180.0,
        Harm::Poison => 150.0,
        Harm::Radiant => 210.0,
        Harm::Void => 260.0,
        _ => 110.0,
    }
}

/// What rises off it as it gathers it.
fn motes_of(harm: Harm) -> &'static str {
    match harm {
        Harm::Fire => "fire",
        Harm::Frost => "frost_goo",
        Harm::Acid => "acid_goo",
        Harm::Poison => "miasma",
        Harm::Radiant => "radiant_goo",
        Harm::Void => "void_goo",
        _ => "spark_goo",
    }
}

/// What an eruption looks like: a fountain of flame and molten drops up
/// out of it, embers thrown all round.
fn eruption_fountain() -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter {
        count: 0.0,
        life: (0.35, 1.1),
        colors: vec![(255, 250, 210), (255, 200, 80), (255, 120, 30), (190, 50, 20), (70, 40, 35)],
        speed: 440.0,
        spread: 0.6,
        gravity: 260.0,
        drag: 1.2,
        size: 1.8,
        jitter: 0.0,
        glow: true,
    }
}
fn eruption_ring() -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter { life: (0.2, 0.6), speed: 230.0, spread: std::f32::consts::PI, gravity: 120.0, drag: 3.0, size: 1.3, ..eruption_fountain() }
}
/// Where it blinks from and to: a puff of the void.
fn blink_puff() -> crate::magic::runes::Emitter {
    crate::magic::runes::Emitter {
        count: 0.0,
        life: (0.2, 0.55),
        colors: vec![(230, 210, 255), (150, 90, 230), (60, 20, 110), (20, 6, 34)],
        speed: 160.0,
        spread: std::f32::consts::PI,
        gravity: 0.0,
        drag: 4.0,
        size: 1.5,
        jitter: 40.0,
        glow: true,
    }
}

/// The glutton attacks with what it holds: lightning struck at you, ice
/// shards shot at you, fire erupting out of it, acid lobbed at you,
/// miasma belched at you, a ray of light swept across you, or the void:
/// it blinks beside you and springs.
#[allow(clippy::type_complexity)]
pub fn glut_strike(
    mut commands: Commands,
    time: Res<Time>,
    mut sim: ResMut<SimWorld>,
    book: Res<crate::magic::Spellbook>,
    mut q: Query<(Entity, &mut Glutton, &Goo, &mut Kinematics, &mut crate::creatures::Controls, Option<&crate::creatures::brain::senses::Alert>)>,
    bodies: Query<(Entity, &Kinematics, Option<&crate::creatures::Team>), Without<Glutton>>,
    mut casts: MessageWriter<crate::magic::CastRequest>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    (mut sparks, mut flares, mut trauma): (ResMut<crate::vfx::Sparks>, MessageWriter<crate::fx::Flare>, ResMut<crate::fx::Trauma>),
) {
    let now = time.elapsed_secs();
    let spell = |id: &str| book.spells.iter().position(|s| s.id == id);
    for (e, mut g, goo, mut k, mut controls, alert) in &mut q {
        let Some(harm) = g.took else {
            g.striking = None;
            continue;
        };
        let target = alert.filter(|a| a.engaged()).and_then(|a| a.target).filter(|&t| t != e);
        let tk = target.and_then(|t| bodies.get(t).ok()).map(|(_, tk, _)| tk);
        // (Its spells leave from just outside its goo, toward them: from
        // inside, they'd strike its own goo.)
        let reach = goo.extent.unwrap_or((k.body.pos - k.body.half, k.body.pos + k.body.half));
        let top = tk.map_or(k.body.pos + Vec2::Y * k.body.half.y, |t| outside(reach, k.body.pos, (t.body.pos - k.body.pos).normalize_or(Vec2::Y) + Vec2::Y * 0.5));
        match g.striking {
            None => {
                let Some(tk) = tk else { continue };
                // (Not spiking, not with you in it, on its feet.)
                if now >= g.ready && matches!(goo.spiking, Spiking::Rest) && k.loco.grounded() && tk.body.pos.distance(k.body.pos) <= range_of(harm) {
                    g.striking = Some((now, 0));
                    let pitch = match harm {
                        Harm::Fire => 0.5,
                        Harm::Void => 0.35,
                        Harm::Poison | Harm::Acid => 0.7,
                        Harm::Frost => 1.8,
                        _ => 2.2,
                    };
                    sounds.write(crate::sound::PlaySound::at("pour", k.body.pos).pitch(pitch));
                }
            }
            Some((since, shots)) => {
                let into = now - since;
                if into < STRIKE_TELL {
                    // Gathering it: its light swelling, motes of it rising
                    // off its goo.
                    let swell = 1.0 + 2.0 * into / STRIKE_TELL;
                    commands.entity(e).insert(crate::light::LightSource { color: glow_of(harm).map(|c| c * swell), flicker: 0.35 });
                    if let Some(m) = sim.world.materials().id(motes_of(harm)) {
                        let w = k.body.half.x * 0.8;
                        let x = top.x + ((now * 37.0).sin() * w);
                        sim.world.puff([x, top.y], Cell::new(m, 0), 3, 1.4);
                    }
                } else {
                    let at = tk.map(|t| t.body.pos);
                    let after = into - STRIKE_TELL;
                    let done = match harm {
                        Harm::Storm => {
                            if let (Some(i), Some(at)) = (spell("lightning"), at) {
                                casts.write(crate::magic::CastRequest { caster: e, spell: i, from: top, toward: at, alt: false });
                            }
                            sounds.write(crate::sound::PlaySound::at("zap", top));
                            true
                        }
                        // Shot after shot, a little fanned: one at them, one
                        // either side (acid lobbed: aimed above them by how
                        // far it falls on the way).
                        Harm::Frost | Harm::Acid => {
                            let (id, lobbed) = if harm == Harm::Acid { ("glutton_acid", true) } else { ("glutton_ice", false) };
                            if harm == Harm::Acid && shots == 0 {
                                sounds.write(crate::sound::PlaySound::at("spit", top).pitch(0.7));
                            }
                            let due = 1 + (after / SHOTS.1) as u32;
                            if shots < due.min(SHOTS.0) {
                                g.striking = Some((since, shots + 1));
                                if let (Some(i), Some(tk)) = (spell(id), tk) {
                                    let aim = match book.flight(i).filter(|_| lobbed) {
                                        Some((speed, fall)) => {
                                            let t = top.distance(tk.body.pos) / speed.max(1.0);
                                            tk.body.pos + Vec2::Y * 0.5 * fall * t * t
                                        }
                                        None => tk.body.pos,
                                    };
                                    let off = [0.0, 1.0, -1.0][shots as usize % 3] * FAN;
                                    casts.write(crate::magic::CastRequest { caster: e, spell: i, from: top, toward: top + (aim - top).rotate(Vec2::from_angle(off.to_radians())), alt: false });
                                }
                            }
                            shots + 1 >= SHOTS.0 && after >= SHOTS.1 * (SHOTS.0 - 1) as f32
                        }
                        Harm::Poison => {
                            if let (Some(i), Some(at)) = (spell("glutton_miasma"), at) {
                                casts.write(crate::magic::CastRequest { caster: e, spell: i, from: top, toward: at, alt: false });
                            }
                            sounds.write(crate::sound::PlaySound::at("vent", top).pitch(0.6));
                            true
                        }
                        Harm::Radiant => {
                            // Swept from above them down through them.
                            if shots == 0 {
                                g.striking = Some((since, 1));
                                sounds.write(crate::sound::PlaySound::at("laser", top));
                            }
                            let f = (after / RAY.0).min(1.0);
                            if let (Some(i), Some(at)) = (spell("glutton_ray"), at) {
                                let way = (at - top).normalize_or(Vec2::X);
                                let side = if way.x < 0.0 { -1.0 } else { 1.0 };
                                let turn = (RAY.1 - 2.0 * RAY.1 * f) * side;
                                casts.write(crate::magic::CastRequest { caster: e, spell: i, from: top, toward: top + way.rotate(Vec2::from_angle(turn.to_radians())) * 200.0, alt: false });
                            }
                            f >= 1.0
                        }
                        Harm::Void => {
                            if let Some(tk) = tk {
                                blink(&sim, &mut k, tk, &mut sparks, &mut sounds);
                            }
                            true
                        }
                        _ => {
                            erupt(&mut sim, &k, reach, at, &bodies, &mut hits);
                            // (Its look: a fountain of flame and molten
                            // drops, embers all round, a flare, a jolt.)
                            let mouth = Vec2::new(k.body.pos.x, reach.1.y);
                            sparks.emit(&eruption_fountain(), 340, mouth, Vec2::Y, Vec2::ZERO);
                            sparks.emit(&eruption_ring(), 180, k.body.pos, Vec2::Y, Vec2::ZERO);
                            flares.write(crate::fx::Flare { at: mouth, color: [2.6, 1.3, 0.5], life: 0.8 });
                            trauma.0 = (trauma.0 + 0.55).min(1.0);
                            sounds.write(crate::sound::PlaySound::at("boom", k.body.pos).volume(0.7).pitch(0.8));
                            true
                        }
                    };
                    if done {
                        commands.entity(e).insert(crate::light::LightSource { color: glow_of(harm), flicker: 0.15 });
                        g.own = Some((harm, now + OWN));
                        g.striking = None;
                        g.ready = now + STRIKE_EVERY;
                        spend(&mut g, STRIKE_COST);
                    }
                }
                // (Gathering and letting go, it holds still; a blink's
                // spring is its own.)
                controls.0.move_x = 0.0;
                controls.0.jump = false;
            }
        }
    }
}

/// The void glutton blinks to their far side (or their near side, if
/// that's walled), level with their feet, and springs at them.
fn blink(sim: &SimWorld, k: &mut Kinematics, tk: &Kinematics, sparks: &mut crate::vfx::Sparks, sounds: &mut MessageWriter<crate::sound::PlaySound>) {
    let grid = crate::creatures::WorldGrid(&sim.world);
    let solid = |x: f32, y: f32| platypus_physics::Grid::occupancy(&grid, x.floor() as i32, y.floor() as i32) == platypus_physics::Occupancy::Solid;
    let half = k.body.half;
    let free = |c: Vec2| {
        let (lo, hi) = (c - half, c + half);
        let mut y = lo.y;
        while y <= hi.y {
            let mut x = lo.x;
            while x <= hi.x {
                if solid(x, y) {
                    return false;
                }
                x += 3.0;
            }
            y += 3.0;
        }
        true
    };
    let side = if k.body.pos.x < tk.body.pos.x { 1.0 } else { -1.0 };
    let y = tk.body.pos.y - tk.body.half.y + half.y + 0.5;
    let gap = tk.body.half.x + half.x + 3.0;
    let Some(to) = [side, -side].into_iter().map(|s| Vec2::new(tk.body.pos.x + s * gap, y)).find(|&c| free(c)) else { return };
    let puff = blink_puff();
    sparks.emit(&puff, 90, k.body.pos, Vec2::Y, Vec2::ZERO);
    sparks.emit(&puff, 90, to, Vec2::Y, Vec2::ZERO);
    sounds.write(crate::sound::PlaySound::at("cast", k.body.pos).pitch(0.5));
    sounds.write(crate::sound::PlaySound::at("cast", to).pitch(0.4));
    k.body.pos = to;
    k.prev_pos = to;
    let toward = (tk.body.pos.x - to.x).signum();
    k.body.vel = Vec2::new(toward * POUNCE.0, POUNCE.1);
}

/// Where a ray from `from` going `dir` leaves the box `reach` (and a few
/// cells past it).
fn outside(reach: (Vec2, Vec2), from: Vec2, dir: Vec2) -> Vec2 {
    let (lo, hi) = (reach.0 - 3.0, reach.1 + 3.0);
    let dir = dir.normalize_or(Vec2::Y);
    let t = |p: f32, d: f32, lo: f32, hi: f32| if d > 1e-4 { (hi - p) / d } else if d < -1e-4 { (lo - p) / d } else { f32::INFINITY };
    let k = t(from.x, dir.x, lo.x, hi.x).min(t(from.y, dir.y, lo.y, hi.y)).max(0.0);
    from + dir * k
}

/// Fire erupting out of the glutton: lava lobbed at them (`at`) and
/// thrown up with flame, what's near burnt and thrown up and back.
fn erupt(sim: &mut SimWorld, k: &Kinematics, reach: (Vec2, Vec2), at: Option<Vec2>, bodies: &Query<(Entity, &Kinematics, Option<&crate::creatures::Team>), Without<Glutton>>, hits: &mut MessageWriter<crate::combat::Hit>) {
    let top = Vec2::new(k.body.pos.x, reach.1.y + 2.0);
    let mats = sim.world.materials().clone();
    if let Some(lava) = mats.id("magma") {
        // (Lobbed at 60°, hard enough to reach them; slower drops fall
        // between: a rain of it, from here to there.)
        let to = at.unwrap_or(top + Vec2::X * 40.0);
        let side = if to.x < top.x { -1.0 } else { 1.0 };
        let far = (to.x - top.x).abs().max(20.0) * 1.15;
        let speed = (far * platypus_sim::particles::GRAVITY / (120f32).to_radians().sin()).sqrt();
        let dir = Vec2::from_angle(60f32.to_radians());
        sim.world.spray([top.x, top.y], [dir.x * side, dir.y], lava, ERUPTION_MATTER.0, speed, 0.3);
        sim.world.spray([top.x, top.y], [0.0, 1.0], lava, ERUPTION_MATTER.1, 5.0, 0.6);
    }
    if let Some(fire) = mats.id("fire") {
        sim.world.spray([top.x, top.y], [0.0, 1.0], fire, ERUPTION_MATTER.2, 5.6, 1.3);
    }
    sim.queue(platypus_sim::WorldEdit::Ignite { center: CellPos::from_world(k.body.pos.x, k.body.pos.y), radius: 16 });
    burn_near(k.body.pos, reach, ERUPTION, bodies, hits);
}
