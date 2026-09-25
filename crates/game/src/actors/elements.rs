//! The elements acting on creatures: heat, corrosion, fire, water. The sim
//! says what a body is exposed to (`World::exposure`, one rule from material
//! data); this applies it, and keeps the two status effects:
//!
//! - `Burning`: damage over time, trails flames and sets alight what it
//!   touches (a burning orc running through a meadow lights the meadow).
//! - `Wet`: fresh out of water or out in the rain, can't catch fire.
//! - `Chilled`: touched something freezing; slowed (up to 60 %) for a moment.
//!   Strong cold puts a fire out.
//!
//! Each creature kind can resist (`resist` in its RON file).

use bevy::prelude::*;
use platypus_sim::{CellPos, WorldEdit};
use serde::Deserialize;

use super::animation::CreatureSprite;
use super::{Health, Kinematics};
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// Seconds a creature burns after it last touched flames.
const BURN_SECS: f32 = 4.0;
const BURN_DAMAGE: f32 = 7.0;
/// Seconds it stays wet after leaving water.
const WET_SECS: f32 = 3.0;
/// A burning creature lights what it touches this often (seconds).
const SPREAD_EVERY: f32 = 0.25;
/// Seconds it stays chilled after the cold contact ends.
const CHILL_SECS: f32 = 1.5;
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

#[derive(Component, Clone, Copy, Debug)]
pub struct Burning {
    pub left: f32,
    spread: f32,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Wet {
    pub left: f32,
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
    Option<&'a mut Wet>,
    Option<&'a mut Chilled>,
);

/// Runs after movement, before deaths.
pub fn expose(mut commands: Commands, mut sim: ResMut<SimWorld>, mut q: Query<Exposed>) {
    let tick = sim.world.tick();
    let fire = sim.materials().fire();
    for (entity, k, mut health, resist, burning, wet, chilled) in &mut q {
        let resist = resist.copied().unwrap_or_default();
        let (pos, half) = (k.body.pos, k.body.half);
        // The same cells collision uses.
        let (lo, hi) = k.body.cells_at(pos);
        let mut e = sim.world.exposure(CellPos::new(lo.x, lo.y), CellPos::new(hi.x, hi.y));
        // Out in the rain: soaked, as good as in water for fire.
        if sim.world.rained_on(CellPos::new((lo.x + hi.x) / 2, hi.y + 1)) {
            e.douses = true;
            e.ignites = false;
        }

        let harm = e.heat * (1.0 - resist.heat).max(0.0) + e.corrosion * (1.0 - resist.corrosion).max(0.0);
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
        // Hard frost snuffs a fire.
        let frozen_out = cold > 0.5;

        // Water puts it out and keeps it from catching for a while.
        if frozen_out && burning.is_some() && !e.douses {
            commands.entity(entity).remove::<Burning>();
            continue;
        }
        if e.douses {
            if burning.is_some() {
                commands.entity(entity).remove::<Burning>();
            }
            match wet {
                Some(mut w) => w.left = WET_SECS,
                None => {
                    commands.entity(entity).insert(Wet { left: WET_SECS });
                }
            }
            continue;
        }
        let wet_now = match wet {
            Some(mut w) => {
                w.left -= DT;
                if w.left <= 0.0 {
                    commands.entity(entity).remove::<Wet>();
                }
                w.left > 0.0
            }
            None => false,
        };

        match burning {
            Some(mut b) => {
                if e.ignites {
                    b.left = BURN_SECS;
                }
                b.left -= DT;
                health.hp -= BURN_DAMAGE * DT;
                b.spread -= DT;
                if b.spread <= 0.0 {
                    b.spread = SPREAD_EVERY;
                    // Flames licking off it, and whatever it's touching catches.
                    let h = platypus_sim::rng::hash(&[tick, entity.to_bits()]);
                    let dx = (h % 1000) as f32 / 1000.0 * 2.0 - 1.0;
                    // Just above it: flames inside its own box would keep it alight forever.
                    let at = pos + Vec2::new(dx * half.x, half.y + 1.5);
                    sim.queue(WorldEdit::Paint { center: CellPos::from_world(at.x, at.y), radius: 1, material: fire, overwrite: false });
                    sim.queue(WorldEdit::Ignite { center: CellPos::from_world(pos.x, pos.y - half.y + 1.0), radius: half.x as i32 + 1 });
                }
                if b.left <= 0.0 {
                    commands.entity(entity).remove::<Burning>();
                }
            }
            None => {
                if e.ignites && !resist.fireproof && !wet_now {
                    commands.entity(entity).insert(Burning { left: BURN_SECS, spread: 0.0 });
                }
            }
        }
    }
}

/// Reach of a lightning strike (cells) and the damage at its centre.
const LIGHTNING_REACH: f32 = 10.0;
const LIGHTNING_DAMAGE: f32 = 55.0;

type Strikable<'a> = (Entity, &'a mut Health, &'a Kinematics, Option<&'a Resist>, Has<Wet>);

/// Lightning hurts whoever stands near where it strikes, and sets them
/// alight (unless wet or fireproof).
pub fn struck(
    mut commands: Commands,
    mut strikes: MessageReader<crate::fx::Lightning>,
    mut q: Query<Strikable>,
) {
    for crate::fx::Lightning(s) in strikes.read() {
        let at = Vec2::new(s.hit.x as f32 + 0.5, s.hit.y as f32 + 0.5);
        for (entity, mut health, k, resist, wet) in &mut q {
            let d = k.body.pos.distance(at);
            if d > LIGHTNING_REACH {
                continue;
            }
            health.hp -= LIGHTNING_DAMAGE * (1.0 - d / LIGHTNING_REACH);
            if !wet && !resist.is_some_and(|r| r.fireproof) {
                commands.entity(entity).insert(Burning { left: BURN_SECS, spread: 0.0 });
            }
        }
    }
}

type Statuses<'a> = (&'a Children, Has<Burning>, Has<Wet>, Option<&'a Chilled>);

/// Burning creatures flicker orange; chilled ones go icy; wet ones look a
/// little blue.
pub fn tint(
    time: Res<Time>,
    creatures: Query<Statuses>,
    mut sprites: Query<&mut Sprite, With<CreatureSprite>>,
) {
    let t = time.elapsed_secs();
    for (children, burning, wet, chilled) in &creatures {
        let color = if burning {
            let f = 0.5 + 0.5 * (t * 23.0).sin();
            Color::srgb(1.0, 0.55 + 0.25 * f, 0.3 + 0.2 * f)
        } else if let Some(c) = chilled {
            let k = 0.25 + 0.45 * c.cold;
            Color::srgb(1.0 - 0.6 * k, 1.0 - 0.25 * k, 1.0)
        } else if wet {
            Color::srgb(0.75, 0.85, 1.0)
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
        app
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
        wet.world_mut().entity_mut(e).insert(Burning { left: BURN_SECS, spread: 1.0 });
        tick(&mut wet, 1);
        assert!(wet.world().get::<Burning>(e).is_none(), "water put it out");
        assert!(wet.world().get::<Wet>(e).is_some(), "and it's wet");

        // Wet, then straight into flames: doesn't catch.
        let mut app = app_with("fire");
        let e = creature(&mut app, IN, Resist::default());
        app.world_mut().entity_mut(e).insert(Wet { left: WET_SECS });
        tick(&mut app, 1);
        assert!(app.world().get::<Burning>(e).is_none(), "wet things don't catch");

        let mut app = app_with("fire");
        let e = creature(&mut app, IN, Resist { fireproof: true, ..default() });
        tick(&mut app, 30);
        assert!(app.world().get::<Burning>(e).is_none(), "fireproof");
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
        app.world_mut().entity_mut(e).insert(Burning { left: BURN_SECS, spread: 1.0 });
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
