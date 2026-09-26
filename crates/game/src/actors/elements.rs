//! The elements acting on creatures: heat, cold, corrosion, fire, and what
//! fluids leave on them. The sim says what a body is exposed to
//! (`World::exposure`, one rule from material data); this applies it and
//! keeps the statuses:
//!
//! - `Burning`: damage over time, flames above it, and it sets alight what it
//!   stands in (a burning orc running through a meadow lights the meadow).
//!   It burns out on its own; it can't relight itself from its own flames.
//! - `Coated`: what the last fluid it touched left on it (`coatings.ron`):
//!   wet (water, snow: puts fires out, can't catch), oily (burns long and
//!   hard, catches from heat), acid (eats at it)... One at a time: a new fluid
//!   replaces the old one, so jumping in water washes off oil.
//! - `Chilled`: touched something freezing; slowed (up to 60 %) for a moment.
//!   Hard frost puts a fire out.
//!
//! Each creature kind can resist (`resist` in its RON file).

use std::collections::HashMap;

use bevy::prelude::*;
use platypus_sim::{CellPos, WorldEdit};
use serde::Deserialize;

use super::animation::CreatureSprite;
use super::{Health, Kinematics};
use crate::data::{Watched, data_path, load_ron};
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// Seconds a creature burns once set alight (times its coating's `burn`).
pub const BURN_SECS: f32 = 4.0;
const BURN_DAMAGE: f32 = 7.0;
/// A burning creature lights what it touches this often (seconds).
const SPREAD_EVERY: f32 = 0.25;
/// Seconds it stays chilled after the cold contact ends.
pub const CHILL_SECS: f32 = 1.5;
/// Fully chilled, it moves at this share of its speed.
const CHILL_SLOW: f32 = 0.4;

/// Shares (0..1) of elemental damage a creature ignores, and whether it can
/// catch fire at all. Default: none, and it can.
#[derive(Component, Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Resist {
    pub heat: f32,
    pub corrosion: f32,
    pub fireproof: bool,
}

/// One coating's rules (see `assets/data/coatings.ron`).
#[derive(Clone, Debug, Deserialize)]
pub struct Coating {
    pub label: String,
    pub secs: f32,
    pub color: (u8, u8, u8),
    #[serde(default)]
    pub fireproof: bool,
    #[serde(default)]
    pub heat_resist: f32,
    #[serde(default = "one")]
    pub burn: f32,
    #[serde(default)]
    pub catches: bool,
    #[serde(default)]
    pub damage: f32,
}

fn one() -> f32 {
    1.0
}

/// Every coating by name, hot-reloaded.
#[derive(Resource)]
pub struct Coatings {
    pub by_name: HashMap<String, Coating>,
    watch: Option<Watched>,
}

impl Coatings {
    #[cfg(test)]
    pub fn from_ron(text: &str) -> Result<Self, String> {
        Ok(Coatings { by_name: crate::data::parse_ron(text)?, watch: None })
    }

    pub fn load() -> Self {
        let path = data_path("coatings.ron");
        let by_name = load_ron(&path).unwrap_or_else(|e| panic!("{e}"));
        Coatings { by_name, watch: Some(Watched::new(path)) }
    }
}

pub fn reload_coatings(mut c: ResMut<Coatings>) {
    let Some(w) = c.watch.as_mut() else { return };
    if !w.changed() {
        return;
    }
    match load_ron(w.path()) {
        Ok(new) => {
            c.by_name = new;
            info!("coatings reloaded");
        }
        Err(e) => warn!("coatings not reloaded: {e}"),
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Burning {
    pub left: f32,
    /// What it started with (for the timer).
    pub total: f32,
    /// Damage multiplier (oil burns hard).
    pub power: f32,
    spread: f32,
}

impl Burning {
    pub fn new(secs: f32, power: f32) -> Self {
        Burning { left: secs, total: secs, power, spread: 0.0 }
    }
}

#[derive(Component, Clone, Debug)]
pub struct Coated {
    pub name: String,
    pub left: f32,
    pub total: f32,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Chilled {
    pub left: f32,
    /// 0..1, how hard.
    pub cold: f32,
}

impl Chilled {
    /// Share of normal speed it moves at.
    pub fn speed(&self) -> f32 {
        1.0 - (1.0 - CHILL_SLOW) * self.cold
    }
}

type Exposed<'a> = (
    Entity,
    &'a Kinematics,
    &'a mut Health,
    Option<&'a Resist>,
    Option<&'a mut Burning>,
    Option<&'a mut Coated>,
    Option<&'a mut Chilled>,
);

/// Runs after movement, before deaths.
pub fn expose(mut commands: Commands, mut sim: ResMut<SimWorld>, coatings: Res<Coatings>, mut q: Query<Exposed>) {
    let tick = sim.world.tick();
    let fire = sim.materials().fire();
    for (entity, k, mut health, resist, burning, coated, chilled) in &mut q {
        let resist = resist.copied().unwrap_or_default();
        let (pos, half) = (k.body.pos, k.body.half);
        // The same cells collision uses.
        let (lo, hi) = k.body.cells_at(pos);
        let e = sim.world.exposure(CellPos::new(lo.x, lo.y), CellPos::new(hi.x, hi.y));

        // What it's coated in: the fluid it touches now (or the rain), else
        // what it had, wearing off.
        let touching = e
            .coat
            .and_then(|m| sim.materials().def(m).coats.clone())
            .or_else(|| sim.world.rained_on(CellPos::new((lo.x + hi.x) / 2, hi.y + 1)).then(|| "wet".to_string()));
        let coat_name = match (touching, coated) {
            (Some(name), Some(mut c)) => {
                let secs = coatings.by_name.get(&name).map_or(0.0, |c| c.secs);
                if c.name != name {
                    c.name = name.clone();
                }
                c.left = secs;
                c.total = secs;
                Some(name)
            }
            (Some(name), None) => {
                let secs = coatings.by_name.get(&name).map_or(0.0, |c| c.secs);
                commands.entity(entity).insert(Coated { name: name.clone(), left: secs, total: secs });
                Some(name)
            }
            (None, Some(mut c)) => {
                c.left -= DT;
                if c.left <= 0.0 {
                    commands.entity(entity).remove::<Coated>();
                    None
                } else {
                    Some(c.name.clone())
                }
            }
            (None, None) => None,
        };
        let coat = coat_name.as_ref().and_then(|n| coatings.by_name.get(n));

        let heat_resist = (resist.heat + coat.map_or(0.0, |c| c.heat_resist)).min(1.0);
        // A coating's damage is what lingers after leaving the fluid; in it,
        // the fluid's own corrosion counts (not both).
        let corrosion = e.corrosion.max(coat.map_or(0.0, |c| c.damage));
        let harm = e.heat * (1.0 - heat_resist) + corrosion * (1.0 - resist.corrosion).max(0.0);
        health.hp -= harm * DT;

        // Cold: slowed while touching it and a moment after; resisted like heat.
        let cold = e.cold * (1.0 - resist.heat).max(0.0);
        match chilled {
            Some(mut c) if cold > 0.0 => {
                c.left = CHILL_SECS;
                c.cold = c.cold.max(cold);
            }
            Some(mut c) => {
                c.left -= DT;
                if c.left <= 0.0 {
                    commands.entity(entity).remove::<Chilled>();
                }
            }
            None if cold > 0.0 => {
                commands.entity(entity).insert(Chilled { left: CHILL_SECS, cold });
            }
            None => {}
        }

        let fireproof = resist.fireproof || coat.is_some_and(|c| c.fireproof);
        match burning {
            Some(mut b) => {
                // Water, snow or hard frost puts it out.
                if fireproof || cold > 0.5 {
                    commands.entity(entity).remove::<Burning>();
                    continue;
                }
                // It burns out on its own: its own flames don't relight it.
                b.left -= DT;
                health.hp -= BURN_DAMAGE * b.power * DT;
                b.spread -= DT;
                if b.spread <= 0.0 {
                    b.spread = SPREAD_EVERY;
                    // Flames above it (clear of its own body), and whatever
                    // it's standing in catches.
                    let h = platypus_sim::rng::hash(&[tick, entity.to_bits()]);
                    let dx = (h % 1000) as f32 / 1000.0 * 2.0 - 1.0;
                    let at = pos + Vec2::new(dx * half.x, half.y + 3.0);
                    sim.queue(WorldEdit::Paint { center: CellPos::from_world(at.x, at.y), radius: 1, material: fire, overwrite: false });
                    sim.queue(WorldEdit::Scorch { center: CellPos::from_world(pos.x, pos.y - half.y + 1.0), radius: half.x as i32 + 1 });
                }
                if b.left <= 0.0 {
                    commands.entity(entity).remove::<Burning>();
                }
            }
            None => {
                let heat_lit = coat.is_some_and(|c| c.catches) && e.heat > 0.0;
                if (e.ignites || heat_lit) && !fireproof {
                    let burn = coat.map_or(1.0, |c| c.burn);
                    commands.entity(entity).insert(Burning::new(BURN_SECS * burn, burn));
                }
            }
        }
    }
}

/// Reach of a lightning strike (cells) and the damage at its centre.
const LIGHTNING_REACH: f32 = 10.0;
const LIGHTNING_DAMAGE: f32 = 55.0;
/// Wand lightning: hurts out to this many cells from where it ends (tight:
/// it's aimed, and whoever cast it is usually near)...
const ZAP_REACH: f32 = 3.0;
/// ... this much, at most.
const ZAP_DAMAGE: f32 = 30.0;
/// Touching what a zap charged (in the pool it struck): this much, and
/// stunned this long.
const SHOCK_DAMAGE: f32 = 25.0;
const SHOCK_STUN: f32 = 0.6;

type Strikable<'a> = (Entity, &'a mut Health, &'a Kinematics, Option<&'a Resist>, Option<&'a Coated>);
type Shockable<'a> = (Entity, &'a mut Health, &'a mut Kinematics, Option<&'a Resist>, Option<&'a Coated>);

/// Lightning hurts whoever stands near where it strikes, and sets them
/// alight (unless coated in something that won't burn, or fireproof).
pub fn struck(
    mut commands: Commands,
    mut strikes: MessageReader<crate::fx::Lightning>,
    coatings: Res<Coatings>,
    mut q: Query<Strikable>,
) {
    for crate::fx::Lightning(s) in strikes.read() {
        let at = Vec2::new(s.hit.x as f32 + 0.5, s.hit.y as f32 + 0.5);
        for (entity, mut health, k, resist, coated) in &mut q {
            let d = k.body.pos.distance(at);
            if d > LIGHTNING_REACH {
                continue;
            }
            health.hp -= LIGHTNING_DAMAGE * (1.0 - d / LIGHTNING_REACH);
            catch_fire(&mut commands, entity, resist, coated, &coatings);
        }
    }
}

/// Wand lightning (`fx::Zapped`): the sky's, smaller. It hurts what it ends
/// at and sets it alight; and whatever touches what it charged (the whole
/// pool it struck) is shocked: hurt and stunned, whoever cast it too.
pub fn zapped(mut commands: Commands, mut zaps: MessageReader<crate::fx::Zapped>, coatings: Res<Coatings>, mut q: Query<Shockable>) {
    for crate::fx::Zapped(z) in zaps.read() {
        if !z.charged.is_empty() {
            let charged: std::collections::HashSet<CellPos> = z.charged.iter().copied().collect();
            for (_, mut health, mut k, ..) in &mut q {
                let (lo, hi) = (k.body.pos - k.body.half, k.body.pos + k.body.half);
                let touches = (lo.y.floor() as i32 - 1..=hi.y.ceil() as i32)
                    .any(|y| (lo.x.floor() as i32 - 1..=hi.x.ceil() as i32).any(|x| charged.contains(&CellPos::new(x, y))));
                if touches {
                    health.hp -= SHOCK_DAMAGE;
                    let k = &mut *k;
                    let vel = k.body.vel * 0.3;
                    k.loco.knock(&mut k.body, vel, SHOCK_STUN);
                }
            }
        }
        let at = Vec2::new(z.to.x as f32 + 0.5, z.to.y as f32 + 0.5);
        for (entity, mut health, k, resist, coated) in &mut q {
            // (From the body's edge: the bolt ends at its centre or a wall.)
            let d = (k.body.pos.distance(at) - k.body.half.min_element()).max(0.0);
            if d > ZAP_REACH {
                continue;
            }
            health.hp -= ZAP_DAMAGE * (1.0 - d / ZAP_REACH);
            catch_fire(&mut commands, entity, resist, coated, &coatings);
        }
    }
}

/// Set a creature alight, unless its coating (wet) or its kind won't burn.
/// An oily one burns longer and harder.
pub fn catch_fire(commands: &mut Commands, entity: Entity, resist: Option<&Resist>, coated: Option<&Coated>, coatings: &Coatings) {
    let coat = coated.and_then(|c| coatings.by_name.get(&c.name));
    if !coat.is_some_and(|c| c.fireproof) && !resist.is_some_and(|r| r.fireproof) {
        let burn = coat.map_or(1.0, |c| c.burn);
        commands.entity(entity).insert(Burning::new(BURN_SECS * burn, burn));
    }
}

type Statuses<'a> = (&'a Children, Has<Burning>, Option<&'a Coated>, Option<&'a Chilled>, Option<&'a mut super::hurt::Hurt>);

/// Just hit: a red flash. Otherwise burning creatures flicker orange;
/// chilled ones go icy; coated ones take a little of their coating's colour.
pub fn tint(
    time: Res<Time>,
    coatings: Res<Coatings>,
    mut creatures: Query<Statuses>,
    mut sprites: Query<&mut Sprite, With<CreatureSprite>>,
) {
    let t = time.elapsed_secs();
    for (children, burning, coated, chilled, hurt) in &mut creatures {
        let flash = hurt.is_some_and(|mut h| {
            h.flash = (h.flash - time.delta_secs()).max(0.0);
            h.flash > 0.0
        });
        let color = if flash {
            Color::srgb(1.0, 0.3, 0.28)
        } else if burning {
            let f = 0.5 + 0.5 * (t * 23.0).sin();
            Color::srgb(1.0, 0.55 + 0.25 * f, 0.3 + 0.2 * f)
        } else if let Some(c) = chilled {
            let k = 0.25 + 0.45 * c.cold;
            Color::srgb(1.0 - 0.6 * k, 1.0 - 0.25 * k, 1.0)
        } else if let Some(c) = coated.and_then(|c| coatings.by_name.get(&c.name)) {
            let (r, g, b) = c.color;
            Color::srgb_u8(r, g, b).mix(&Color::WHITE, 0.6)
        } else {
            Color::WHITE
        };
        for child in children.iter() {
            if let Ok(mut s) = sprites.get_mut(child) {
                s.color = color;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bevy::ecs::system::RunSystemOnce;
    use platypus_physics::{Body, Locomotion};
    use platypus_sim::store::ChunkStore;
    use platypus_sim::{Cell, Chunk, ChunkPos, MaterialTable, World as SimCells};
    use platypus_worldgen::FlatGen;

    use super::*;

    /// One chunk of air with a 6×10 pocket of `material` at x 10..16.
    fn app_with(material: &str) -> App {
        let mats = Arc::new(MaterialTable::from_ron(include_str!("../../../../assets/data/materials.ron")).unwrap());
        let mut world = SimCells::new(1, mats.clone());
        world.insert_chunk(Chunk::filled(ChunkPos::new(0, 0), Cell::AIR));
        let id = mats.expect_id(material);
        let mut rng = platypus_sim::rng::Rng::seeded(&[1]);
        for x in 10..16 {
            for y in 10..20 {
                world.set(CellPos::new(x, y), mats.spawn(id, &mut rng));
            }
        }
        let generator = Arc::new(FlatGen { width_chunks: 1, height_chunks: 1, floor: 1, stone: mats.expect_id("stone") });
        let mut app = App::new();
        app.insert_resource(SimWorld { world, generator, store: ChunkStore::default() });
        app.insert_resource(Coatings::from_ron(include_str!("../../../../assets/data/coatings.ron")).unwrap());
        app
    }

    fn coat(app: &App, e: Entity) -> Option<String> {
        app.world().get::<Coated>(e).map(|c| c.name.clone())
    }

    fn creature(app: &mut App, at: Vec2, resist: Resist) -> Entity {
        let body = Body::new(at, Vec2::new(4.0, 8.0));
        app.world_mut()
            .spawn((Kinematics { body, loco: Locomotion::default(), prev_pos: at }, Health { hp: 100.0, max: 100.0 }, resist))
            .id()
    }

    fn tick(app: &mut App, n: usize) {
        for _ in 0..n {
            app.world_mut().run_system_once(expose).unwrap();
        }
    }

    const IN: Vec2 = Vec2::new(13.0, 15.0);
    const OUT: Vec2 = Vec2::new(40.0, 15.0);

    #[test]
    fn lava_burns_acid_eats_air_is_harmless() {
        for (material, lo, hi) in [("lava", 50.0f32, 120.0f32), ("acid", 20.0, 40.0), ("air", 0.0, 0.0)] {
            let mut app = app_with(if material == "air" { "stone" } else { material });
            let at = if material == "air" { OUT } else { IN };
            let e = creature(&mut app, at, Resist::default());
            tick(&mut app, 60);
            let lost = 100.0 - app.world().get::<Health>(e).unwrap().hp;
            assert!((lo..=hi).contains(&lost), "{material}: lost {lost} in a second");
        }
    }

    #[test]
    fn fire_sets_you_alight_water_puts_it_out_and_wet_things_dont_catch() {
        let mut app = app_with("fire");
        let e = creature(&mut app, IN, Resist::default());
        tick(&mut app, 1);
        assert!(app.world().get::<Burning>(e).is_some(), "caught fire");
        // Out of the flames it keeps burning a while, and hurts.
        app.world_mut().get_mut::<Kinematics>(e).unwrap().body.pos = OUT;
        let hp = app.world().get::<Health>(e).unwrap().hp;
        tick(&mut app, 60);
        assert!(app.world().get::<Burning>(e).is_some(), "still burning");
        assert!(app.world().get::<Health>(e).unwrap().hp < hp - 5.0, "burning hurts");

        let mut wet = app_with("water");
        let e = creature(&mut wet, IN, Resist::default());
        wet.world_mut().entity_mut(e).insert(Burning::new(BURN_SECS, 1.0));
        tick(&mut wet, 1);
        assert!(wet.world().get::<Burning>(e).is_none(), "water put it out");
        assert_eq!(coat(&wet, e).as_deref(), Some("wet"), "and it's wet");

        // Wet, then straight into flames: doesn't catch.
        let mut app = app_with("fire");
        let e = creature(&mut app, IN, Resist::default());
        app.world_mut().entity_mut(e).insert(Coated { name: "wet".into(), left: 6.0, total: 6.0 });
        tick(&mut app, 1);
        assert!(app.world().get::<Burning>(e).is_none(), "wet things don't catch");

        let mut app = app_with("fire");
        let e = creature(&mut app, IN, Resist { fireproof: true, ..default() });
        tick(&mut app, 30);
        assert!(app.world().get::<Burning>(e).is_none(), "fireproof");
    }

    #[test]
    fn a_brush_with_fire_burns_out_and_does_not_kill() {
        let mut app = app_with("fire");
        let e = creature(&mut app, IN, Resist::default());
        tick(&mut app, 1);
        app.world_mut().get_mut::<Kinematics>(e).unwrap().body.pos = OUT;
        // Step the sim too: its own flames are painted above it.
        for _ in 0..(BURN_SECS * 60.0) as usize + 30 {
            app.world_mut().resource_mut::<SimWorld>().world.step();
            tick(&mut app, 1);
        }
        assert!(app.world().get::<Burning>(e).is_none(), "burned out");
        let hp = app.world().get::<Health>(e).unwrap().hp;
        assert!(hp > 60.0, "it hurt, it didn't kill ({hp} left)");
    }

    #[test]
    fn snow_puts_a_fire_out() {
        let mut app = app_with("snow");
        // Standing on the snow (its box starts just above the pocket's top at 20).
        let e = creature(&mut app, Vec2::new(13.0, 24.0), Resist::default());
        app.world_mut().entity_mut(e).insert(Burning::new(BURN_SECS, 1.0));
        tick(&mut app, 1);
        assert!(app.world().get::<Burning>(e).is_none(), "the snow put it out");
        assert_eq!(coat(&app, e).as_deref(), Some("wet"));
    }

    #[test]
    fn oil_burns_long_catches_from_heat_and_water_washes_it_off() {
        let mut app = app_with("oil");
        let e = creature(&mut app, IN, Resist::default());
        tick(&mut app, 1);
        assert_eq!(coat(&app, e).as_deref(), Some("oily"));
        // Out of the oil, onto hot stone: an oily creature catches from heat alone.
        let mut hot = app_with("stone");
        hot.world_mut().resource_mut::<SimWorld>().world.apply_edit(&WorldEdit::Heat { center: CellPos::new(13, 15), radius: 8, amount: 200 });
        let e = creature(&mut hot, Vec2::new(13.0, 24.0), Resist::default());
        hot.world_mut().entity_mut(e).insert(Coated { name: "oily".into(), left: 12.0, total: 12.0 });
        tick(&mut hot, 1);
        let b = *hot.world().get::<Burning>(e).expect("caught from heat");
        assert!(b.left > BURN_SECS * 2.0, "and burns long ({} s)", b.left);
        // Into water: washed off, and out.
        let mut water = app_with("water");
        let e = creature(&mut water, IN, Resist::default());
        water.world_mut().entity_mut(e).insert((Coated { name: "oily".into(), left: 12.0, total: 12.0 }, b));
        tick(&mut water, 1);
        assert_eq!(coat(&water, e).as_deref(), Some("wet"), "the water replaced the oil");
        assert!(water.world().get::<Burning>(e).is_none());
    }

    #[test]
    fn resistance_scales_the_harm() {
        let mut app = app_with("acid");
        let e = creature(&mut app, IN, Resist { corrosion: 0.75, ..default() });
        tick(&mut app, 60);
        let lost = 100.0 - app.world().get::<Health>(e).unwrap().hp;
        assert!((6.0..9.0).contains(&lost), "a quarter of acid's 30/s: lost {lost}");
    }

    #[test]
    fn frost_chills_slows_and_snuffs_fire() {
        let mut app = app_with("stone");
        // Freeze the stone pocket hard, and stand a burning creature on it.
        app.world_mut().resource_mut::<SimWorld>().world.apply_edit(&WorldEdit::Heat { center: CellPos::new(13, 15), radius: 8, amount: -200 });
        let e = creature(&mut app, Vec2::new(13.0, 24.0), Resist::default());
        app.world_mut().entity_mut(e).insert(Burning::new(BURN_SECS, 1.0));
        tick(&mut app, 1);
        let chilled = *app.world().get::<Chilled>(e).expect("chilled");
        assert!(chilled.speed() < 0.5, "slowed: {}", chilled.speed());
        assert!(app.world().get::<Burning>(e).is_none(), "the frost put the fire out");
        // Off the ice it wears off.
        app.world_mut().get_mut::<Kinematics>(e).unwrap().body.pos = OUT;
        tick(&mut app, (CHILL_SECS * 60.0) as usize + 2);
        assert!(app.world().get::<Chilled>(e).is_none(), "thawed");
    }
}
