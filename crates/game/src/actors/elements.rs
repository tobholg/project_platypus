//! The elements acting on creatures: heat, cold, corrosion, fire, and what
//! fluids leave on them. The sim says what a body is exposed to
//! (`World::exposure`, one rule from material data); this applies it and
//! keeps the statuses:
//!
//! - `Burning`: damage over time, flames above it, and it sets alight what it
//!   stands in (a burning orc running through a meadow lights the meadow).
//!   It burns out on its own; it can't relight itself from its own flames.
//! - `Coated`: what fluids have left on it (`coatings.ron`), Noita's way:
//!   each a share of it covered, several at once, never more than all of it
//!   together. Wet (water: 40 % puts a fire out, and it can't catch), oily
//!   (burns long and hard, catches from heat), acid (eats at it, less as it
//!   wears off)... In a fluid its share rises towards how deep in it you
//!   are; it pushes the others off to make room, so jumping in water washes
//!   acid off, and a puddle at your feet wets them but won't put you out.
//! - `Chilled`: touched something freezing; slowed (up to 60 %) for a moment.
//!   Hard frost puts a fire out.
//!
//! Each creature kind can resist (`resist` in its RON file).

use std::collections::HashMap;

use bevy::prelude::*;
use platypus_sim::{CellPos, WorldEdit};
use serde::Deserialize;

use super::animation::CreatureSprite;
use super::{Harm, Health, Kinematics};
use crate::data::{Watched, data_path, load_ron};
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// Seconds a creature burns once set alight (times its coating's `burn`).
/// Left alone, a fire burns down by `BURN_SLOW` a second all of it alight,
/// `BURN_SLOW + BURN_FAST` a second as it gutters out (in between, in
/// proportion): a big fire holds on, a little one dies quick. All of it
/// takes ~6 s to burn out (half of it ~2 s, a tenth ~0.3 s); oil's `burn`
/// stretches it.
const BURN_SLOW: f32 = 0.07;
const BURN_FAST: f32 = 0.25;
/// About how long all of it burns, left alone (tests).
#[cfg(test)]
pub const BURN_SECS: f32 = 6.1;
/// Damage a second all of it on fire (less the less of it; oil's `burn` more).
const FIRE_DAMAGE: f32 = 12.0;
/// Its fire rises towards this × the share of it touching flames...
const FIRE_SOAK: f32 = 5.0;
/// ... this fast (a share a second).
const FIRE_RATE: f32 = 5.0;
/// On fire, it spreads over the oily part of it this fast...
const OIL_SPREAD: f32 = 2.0;
/// ... and burns the oil away this fast where it burns (a share a second).
const OIL_BURN: f32 = 0.12;
/// Falling faster than this (cells/s), the rush of air beats a fire down...
const FALL_DOUSE: f32 = 140.0;
/// ... by this much more a second for every cell/s faster (falling flat
/// out, ~300: a full fire out in about a second).
const FALL_DOUSE_RATE: f32 = 0.006;
/// Where it burns it dries this fast (a share a second).
const FIRE_DRIES: f32 = 0.2;
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
    /// It clings: only a washing fluid (water) pushes it off; other fluids
    /// don't (venom, not washed away by the blood it draws).
    #[serde(default)]
    pub sticks: bool,
    /// It pushes even a clinging coating off (water).
    #[serde(default)]
    pub washes: bool,
    /// How readily it soaks in: its share rises towards this × the share of
    /// the body in it.
    #[serde(default = "soak")]
    pub soak: f32,
}

fn soak() -> f32 {
    1.6
}

/// Wet (fireproof coatings) at least this share: a fire goes out, and it
/// can't catch.
pub const DOUSED: f32 = 0.4;
/// Coated in something that `catches` at least this share: heat alone
/// sets it alight.
const CATCHES: f32 = 0.25;
/// In a fluid, a coating's share rises this fast (a share a second) towards
/// what it'll reach there.
const SOAK_RATE: f32 = 4.0;
/// Rain wets you all over, slowly.
const RAIN_RATE: f32 = 0.3;

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

/// On fire, as the coatings are on it: how much of it burns (0..1). It
/// catches as much as touches flames, spreads over what's oily, gives way
/// to what's wet (drying it as it burns) and burns out left alone; it
/// hurts, flames and lights as much as it burns.
#[derive(Component, Clone, Copy, Debug)]
pub struct Burning {
    pub share: f32,
    /// Damage and length multiplier now (oil burns hard and long).
    pub power: f32,
    spread: f32,
    /// Flames owed (`blaze`).
    owed: f32,
}

impl Burning {
    pub fn new(share: f32) -> Self {
        Burning { share: share.clamp(0.0, 1.0), power: 1.0, spread: 0.0, owed: 0.0 }
    }
}

/// A burning creature's firelight (a child of it), gone when it's out.
#[derive(Component)]
pub struct Blaze;

/// A burning creature's firelight: orange, flickering.
const BLAZE_LIGHT: [f32; 3] = [1.6, 0.8, 0.3];

/// Burning creatures blaze as much as they burn: flames (the torch's fire,
/// a little bigger; all of it on fire, as many as a torch's for every 8 × 8
/// cells of it, oil's hotter fire more) licking up from its feet, as far up
/// it as it burns (a few licks at its feet at 10 %, all over it at 100 %),
/// and a flickering firelight on it, as bright as it burns.
pub fn blaze(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<crate::light::LightSettings>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut burning: Query<(Entity, &Kinematics, &mut Burning, Option<&Children>)>,
    mut lights: Query<(Entity, &ChildOf, &mut crate::light::LightSource), With<Blaze>>,
) {
    let dt = time.delta_secs();
    let mut look = settings.fire.flame.clone();
    look.life = (look.life.0 * 1.3, look.life.1 * 1.5);
    look.speed *= 1.4;
    for (e, k, mut b, children) in &mut burning {
        let (pos, half) = (k.body.pos, k.body.half);
        let rate = look.count * (half.x * half.y * 4.0 / 64.0).max(0.5) * b.power.sqrt() * b.share;
        // (How far up it: from its feet.)
        let up = 0.3 + 0.7 * b.share;
        b.owed += rate * dt;
        let n = b.owed.floor();
        b.owed -= n;
        for i in 0..n as u64 {
            let h = platypus_sim::rng::hash(&[time.elapsed().as_micros() as u64, e.to_bits(), i]);
            let unit = |k: u64| ((h >> k) & 1023) as f32 / 1023.0;
            // (Anywhere across it, most from its lower half up.)
            let at = pos + Vec2::new((unit(0) * 2.0 - 1.0) * half.x * 0.9, half.y * (unit(10) * 1.7 * up - 1.0));
            sparks.emit(&look, 1, at, Vec2::Y, k.body.vel * 0.6);
        }
        let glow = BLAZE_LIGHT.map(|c| c * (0.2 + 0.8 * b.share));
        match children.and_then(|c| c.iter().find(|&c| lights.contains(c))) {
            Some(l) => {
                if let Ok((_, _, mut light)) = lights.get_mut(l) {
                    light.color = glow;
                }
            }
            None => {
                commands.entity(e).with_child((Name::new("Blaze"), Blaze, crate::light::LightSource { color: glow, flicker: 1.0 }, Transform::default()));
            }
        }
    }
    // Out: its light goes.
    for (l, parent, _) in &lights {
        if !burning.contains(parent.parent()) {
            commands.entity(l).despawn();
        }
    }
}

/// What's on it: each coating and its share of it covered (0..1), the
/// most first; together never more than 1.
#[derive(Component, Clone, Debug, Default)]
pub struct Coated {
    pub coats: Vec<(String, f32)>,
}

impl Coated {
    /// Coated in this much of `name`.
    #[cfg(test)]
    pub fn with(name: &str, share: f32) -> Self {
        Coated { coats: vec![(name.to_string(), share.clamp(0.0, 1.0))] }
    }

    /// How much of it is covered in `name`.
    pub fn share(&self, name: &str) -> f32 {
        self.coats.iter().find(|(n, _)| n == name).map_or(0.0, |(_, a)| *a)
    }

    /// The sum over its coatings of each's share × `f` of its rules.
    fn sum(&self, rules: &HashMap<String, Coating>, f: impl Fn(&Coating) -> f32) -> f32 {
        self.coats.iter().map(|(n, a)| rules.get(n).map_or(0.0, &f) * a).sum()
    }

    /// How wet: the share covered in coatings that put fires out.
    pub fn wet(&self, rules: &HashMap<String, Coating>) -> f32 {
        self.sum(rules, |c| if c.fireproof { 1.0 } else { 0.0 })
    }

    /// Too wet to burn.
    pub fn doused(&self, rules: &HashMap<String, Coating>) -> bool {
        self.wet(rules) >= DOUSED
    }

    pub fn heat_resist(&self, rules: &HashMap<String, Coating>) -> f32 {
        self.sum(rules, |c| c.heat_resist)
    }

    /// Set alight, it burns this many times as long and as hard.
    pub fn burn(&self, rules: &HashMap<String, Coating>) -> f32 {
        1.0 + self.sum(rules, |c| c.burn - 1.0)
    }

    /// How oily: the share covered in coatings that catch from heat.
    pub fn oily(&self, rules: &HashMap<String, Coating>) -> f32 {
        self.sum(rules, |c| if c.catches { 1.0 } else { 0.0 })
    }

    /// Heat alone sets it alight.
    pub fn catches(&self, rules: &HashMap<String, Coating>) -> bool {
        self.sum(rules, |c| if c.catches { 1.0 } else { 0.0 }) >= CATCHES
    }

    /// Damage a second from what's on it.
    pub fn damage(&self, rules: &HashMap<String, Coating>) -> f32 {
        self.sum(rules, |c| c.damage)
    }

    /// The coating covering most of it.
    pub fn most(&self) -> Option<&(String, f32)> {
        self.coats.first()
    }

    /// `name` up to `to` of it (never down), the others making room: all
    /// of them together never more than 1. A clinging one keeps its place
    /// unless `name` washes.
    pub fn soak(&mut self, name: &str, to: f32, rules: &HashMap<String, Coating>) {
        let washes = rules.get(name).is_some_and(|c| c.washes);
        let clings = |n: &str| n != name && !washes && rules.get(n).is_some_and(|c| c.sticks);
        let kept: f32 = self.coats.iter().filter(|(n, _)| clings(n)).map(|(_, a)| a).sum();
        let to = to.min(1.0 - kept);
        if to <= self.share(name) {
            return;
        }
        match self.coats.iter_mut().find(|(n, _)| n == name) {
            Some((_, a)) => *a = to,
            None => self.coats.push((name.to_string(), to)),
        }
        // (The rest pushed off, evenly, to fit.)
        let room = 1.0 - kept - to;
        let rest: f32 = self.coats.iter().filter(|(n, _)| n != name && !clings(n)).map(|(_, a)| a).sum();
        if rest > room {
            let k = room.max(0.0) / rest;
            for (n, a) in &mut self.coats {
                if n != name && !clings(n) {
                    *a *= k;
                }
            }
        }
        self.tidy();
    }

    /// The most first; none too little to matter.
    fn tidy(&mut self) {
        self.coats.retain(|(_, a)| *a > 0.005);
        self.coats.sort_by(|a, b| b.1.total_cmp(&a.1));
    }
}

/// Coat a creature in `share` of `name` (a stung spider's venom, a weapon's
/// poison), as touching it would.
pub fn stain(commands: &mut Commands, entity: Entity, name: &str, share: f32, coatings: &Coatings) {
    let (name, rules) = (name.to_string(), coatings.by_name.clone());
    commands.entity(entity).queue_silenced(move |mut e: EntityWorldMut| match e.get_mut::<Coated>() {
        Some(mut c) => c.soak(&name, share, &rules),
        None => {
            let mut c = Coated::default();
            c.soak(&name, share, &rules);
            e.insert(c);
        }
    });
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
    let fire_mat = sim.materials().fire();
    let rules = &coatings.by_name;
    for (entity, k, mut health, resist, burning, coated, chilled) in &mut q {
        let resist = resist.copied().unwrap_or_default();
        let (pos, half) = (k.body.pos, k.body.half);
        // The same cells collision uses.
        let (lo, hi) = k.body.cells_at(pos);
        let e = sim.world.exposure(CellPos::new(lo.x, lo.y), CellPos::new(hi.x, hi.y));

        // What's on it: each coating it's in rises towards how deep in it
        // it is (the rain wets it all over, slowly), pushing the others
        // off; the rest wear off.
        let mats = sim.materials();
        let mut soaking: Vec<(String, f32, f32)> = e
            .coats
            .iter()
            .flatten()
            .filter_map(|&(m, share)| {
                let name = mats.def(m).coats.clone()?;
                let soak = rules.get(&name).map_or(1.6, |c| c.soak);
                Some((name, (share * soak).min(1.0), SOAK_RATE))
            })
            .collect();
        if sim.world.rained_on(CellPos::new((lo.x + hi.x) / 2, hi.y + 1)) {
            soaking.push(("wet".to_string(), 1.0, RAIN_RATE));
        }
        let mut coat = coated.as_deref().cloned().unwrap_or_default();
        // (Wearing off, unless it's in enough of it to keep it.)
        for (n, a) in &mut coat.coats {
            if !soaking.iter().any(|(s, to, _)| s == n && *to >= *a) {
                *a -= DT / rules.get(n).map_or(1.0, |c| c.secs).max(0.1);
            }
        }
        coat.tidy();
        for (name, to, rate) in &soaking {
            let now = coat.share(name);
            if *to > now {
                coat.soak(name, (now + rate * DT).min(*to), rules);
            }
        }
        let heat_resist = (resist.heat + coat.heat_resist(rules)).min(1.0);
        // What's on it eats at it (less as it wears off); in the fluid, the
        // fluid's own corrosion counts (not both).
        let corrosion = e.corrosion.max(coat.damage(rules));
        health.harm(e.heat * (1.0 - heat_resist) * DT, Harm::Fire);
        health.harm(corrosion * (1.0 - resist.corrosion).max(0.0) * DT, Harm::Acid);

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

        // Fire, as the coatings: as much of it catches as touches flames
        // (embers underfoot: a little; standing in a fire: all of it; heat
        // alone lights what's oily), it spreads over the oily part, what's
        // wet takes its place (40 % wet puts it out) and dries under it;
        // left alone it burns out. (It only grows from contact or oil.)
        let was = burning.as_ref().map_or(0.0, |b| b.share);
        let power = coat.burn(rules);
        let mut fire = was;
        if resist.fireproof || cold > 0.5 || coat.doused(rules) {
            fire = 0.0;
        } else {
            let oily = coat.oily(rules);
            let heat_lit = coat.catches(rules) && e.heat > 0.0;
            let lit = (e.flames * FIRE_SOAK).min(1.0).max(if heat_lit { oily } else { 0.0 });
            let mut fed = false;
            if lit > fire {
                fire = (fire + FIRE_RATE * DT).min(lit);
                fed = true;
            }
            if fire > 0.0 && oily > fire {
                fire = (fire + OIL_SPREAD * DT).min(oily);
                fed = true;
            }
            if !fed {
                fire -= (BURN_SLOW + BURN_FAST * (1.0 - fire)) * DT / power.max(0.1);
            }
            // (Falling fast, the air beats it down, fed or not.)
            fire -= (-k.body.vel.y - FALL_DOUSE).max(0.0) * FALL_DOUSE_RATE * DT;
            fire = fire.clamp(0.0, 1.0 - coat.wet(rules));
        }
        if fire > 0.0 {
            // Where it burns: wet dries, oil burns away.
            for (n, a) in &mut coat.coats {
                let Some(c) = rules.get(n) else { continue };
                if c.fireproof {
                    *a -= fire * FIRE_DRIES * DT;
                } else if c.catches {
                    *a -= fire * OIL_BURN * DT;
                }
            }
            coat.tidy();
        }
        match coated {
            Some(mut c) if coat.coats.is_empty() => {
                c.coats.clear();
                commands.entity(entity).remove::<Coated>();
            }
            Some(mut c) => {
                if c.coats != coat.coats {
                    *c = coat.clone();
                }
            }
            None if !coat.coats.is_empty() => {
                commands.entity(entity).insert(coat.clone());
            }
            None => {}
        }

        match burning {
            Some(mut b) if fire > 0.005 => {
                b.share = fire;
                b.power = power;
                health.harm(FIRE_DAMAGE * fire * power * DT, Harm::Fire);
                b.spread -= DT;
                if b.spread <= 0.0 {
                    b.spread = SPREAD_EVERY;
                    // Flames above it (clear of its own body), and whatever
                    // it's standing in catches: as often as it burns.
                    let h = platypus_sim::rng::hash(&[tick, entity.to_bits()]);
                    if (h >> 20) % 1000 < (fire * 1000.0) as u64 {
                        let dx = (h % 1000) as f32 / 1000.0 * 2.0 - 1.0;
                        let at = pos + Vec2::new(dx * half.x, half.y + 3.0);
                        sim.queue(WorldEdit::Paint { center: CellPos::from_world(at.x, at.y), radius: 1, material: fire_mat, overwrite: false });
                        sim.queue(WorldEdit::Scorch { center: CellPos::from_world(pos.x, pos.y - half.y + 1.0), radius: half.x as i32 + 1 });
                    }
                }
            }
            Some(_) => {
                commands.entity(entity).remove::<Burning>();
            }
            None if fire > 0.005 => {
                commands.entity(entity).insert(Burning { share: fire, power, spread: 0.0, owed: 0.0 });
            }
            None => {}
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
/// The sky's lightning into water shocks harder.
const SKY_SHOCK_DAMAGE: f32 = 40.0;
const SHOCK_STUN: f32 = 0.6;

type Shockable<'a> = (Entity, &'a mut Health, &'a mut Kinematics, Option<&'a Resist>, Option<&'a Coated>);

/// Everything touching a charged cell (a pool lightning struck) is
/// shocked: hurt and stunned a moment.
fn shock(charged: &[CellPos], damage: f32, q: &mut Query<Shockable>) {
    if charged.is_empty() {
        return;
    }
    let charged: std::collections::HashSet<CellPos> = charged.iter().copied().collect();
    for (_, mut health, mut k, ..) in q.iter_mut() {
        let (lo, hi) = (k.body.pos - k.body.half, k.body.pos + k.body.half);
        let touches = (lo.y.floor() as i32 - 1..=hi.y.ceil() as i32).any(|y| (lo.x.floor() as i32 - 1..=hi.x.ceil() as i32).any(|x| charged.contains(&CellPos::new(x, y))));
        if touches {
            health.harm(damage, Harm::Storm);
            let k = &mut *k;
            let vel = k.body.vel * 0.3;
            k.loco.knock(&mut k.body, vel, SHOCK_STUN);
        }
    }
}

/// Lightning hurts whoever stands near where it strikes, and sets them
/// alight (unless coated in something that won't burn, or fireproof);
/// struck into water, it shocks everyone in it.
pub fn struck(
    mut commands: Commands,
    mut strikes: MessageReader<crate::fx::Lightning>,
    coatings: Res<Coatings>,
    mut q: Query<Shockable>,
) {
    for crate::fx::Lightning(s) in strikes.read() {
        shock(&s.charged, SKY_SHOCK_DAMAGE, &mut q);
        let at = Vec2::new(s.hit.x as f32 + 0.5, s.hit.y as f32 + 0.5);
        for (entity, mut health, k, resist, coated) in &mut q {
            let d = k.body.pos.distance(at);
            if d > LIGHTNING_REACH {
                continue;
            }
            health.harm(LIGHTNING_DAMAGE * (1.0 - d / LIGHTNING_REACH), Harm::Storm);
            catch_fire(&mut commands, entity, resist, coated, &coatings, 1.0);
        }
    }
}

/// Wand lightning (`fx::Zapped`): the sky's, smaller. It hurts what it ends
/// at and sets it alight; and whatever touches what it charged (the whole
/// pool it struck) is shocked: hurt and stunned, whoever cast it too.
pub fn zapped(mut commands: Commands, mut zaps: MessageReader<crate::fx::Zapped>, coatings: Res<Coatings>, mut q: Query<Shockable>) {
    for crate::fx::Zapped(z) in zaps.read() {
        shock(&z.charged, SHOCK_DAMAGE, &mut q);
        let at = Vec2::new(z.to.x as f32 + 0.5, z.to.y as f32 + 0.5);
        for (entity, mut health, k, resist, coated) in &mut q {
            // (From the body's edge: the bolt ends at its centre or a wall.)
            let d = (k.body.pos.distance(at) - k.body.half.min_element()).max(0.0);
            if d > ZAP_REACH {
                continue;
            }
            health.harm(ZAP_DAMAGE * (1.0 - d / ZAP_REACH), Harm::Storm);
            catch_fire(&mut commands, entity, resist, coated, &coatings, 0.6);
        }
    }
}

/// Chill a creature (a frost spell hit it): slowed by `cold` (0..1, less
/// what it resists) for `secs`, or longer or harder if it's colder already.
pub fn chill(commands: &mut Commands, entity: Entity, resist: Option<&Resist>, now: Option<&Chilled>, cold: f32, secs: f32) {
    let cold = cold * (1.0 - resist.map_or(0.0, |r| r.heat)).max(0.0);
    if cold <= 0.0 {
        return;
    }
    let (cold, left) = now.map_or((cold, secs), |c| (c.cold.max(cold), c.left.max(secs)));
    commands.entity(entity).insert(Chilled { left, cold });
}

/// Set `share` more of a creature alight (lightning all of it, a spark
/// some), unless it's wet enough or its kind won't burn.
pub fn catch_fire(commands: &mut Commands, entity: Entity, resist: Option<&Resist>, coated: Option<&Coated>, coatings: &Coatings, share: f32) {
    if coated.is_some_and(|c| c.doused(&coatings.by_name)) || resist.is_some_and(|r| r.fireproof) {
        return;
    }
    commands.entity(entity).queue_silenced(move |mut e: EntityWorldMut| match e.get_mut::<Burning>() {
        Some(mut b) => b.share = (b.share + share).min(1.0),
        None => {
            e.insert(Burning::new(share));
        }
    });
}

type Statuses<'a> = (&'a Children, Option<&'a Burning>, Option<&'a Coated>, Option<&'a Chilled>, Option<&'a mut super::hurt::Hurt>);

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
        } else if let Some(b) = burning {
            // (Scorched and lit from within, flickering fast; as much as it
            // burns.)
            let f = 0.5 + 0.5 * (t * 23.0).sin() * (t * 7.3).sin();
            Color::WHITE.mix(&Color::srgb(1.0, 0.45 + 0.3 * f, 0.2 + 0.2 * f), (0.3 + 0.7 * b.share).min(1.0))
        } else if let Some(c) = chilled {
            let k = 0.25 + 0.45 * c.cold;
            Color::srgb(1.0 - 0.6 * k, 1.0 - 0.25 * k, 1.0)
        } else if let Some((c, share)) = coated.and_then(|c| c.most()).and_then(|(n, a)| Some((coatings.by_name.get(n)?, *a))) {
            let (r, g, b) = c.color;
            Color::WHITE.mix(&Color::srgb_u8(r, g, b), 0.4 * share)
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
        app_region(material, 10..16, 10..20)
    }

    /// One chunk of air with `material` over `xs` × `ys`.
    fn app_region(material: &str, xs: std::ops::Range<i32>, ys: std::ops::Range<i32>) -> App {
        let mats = Arc::new(MaterialTable::from_ron(include_str!("../../../../assets/data/materials.ron")).unwrap());
        let mut world = SimCells::new(1, mats.clone());
        world.insert_chunk(Chunk::filled(ChunkPos::new(0, 0), Cell::AIR));
        let id = mats.expect_id(material);
        let mut rng = platypus_sim::rng::Rng::seeded(&[1]);
        for x in xs {
            for y in ys.clone() {
                world.set(CellPos::new(x, y), mats.spawn(id, &mut rng));
            }
        }
        let generator = Arc::new(FlatGen { width_chunks: 1, height_chunks: 1, floor: 1, stone: mats.expect_id("stone") });
        let mut app = App::new();
        app.insert_resource(SimWorld { world, generator, store: ChunkStore::default() });
        app.insert_resource(Coatings::from_ron(include_str!("../../../../assets/data/coatings.ron")).unwrap());
        app
    }

    /// What covers most of it.
    fn coat(app: &App, e: Entity) -> Option<String> {
        app.world().get::<Coated>(e).and_then(|c| c.most().map(|(n, _)| n.clone()))
    }

    fn share(app: &App, e: Entity, name: &str) -> f32 {
        app.world().get::<Coated>(e).map_or(0.0, |c| c.share(name))
    }

    fn creature(app: &mut App, at: Vec2, resist: Resist) -> Entity {
        let body = Body::new(at, Vec2::new(4.0, 8.0));
        app.world_mut()
            .spawn((Kinematics { body, loco: Locomotion::default(), prev_pos: at }, Health::new(100.0), resist))
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
        // Standing in it: all of it alight in a moment.
        tick(&mut app, 20);
        let b = app.world().get::<Burning>(e).unwrap().share;
        assert!(b > 0.95, "in the fire, all of it burns: {b}");
        // Out of the flames it keeps burning a while (all of it alight: past
        // half of it after 3 s), and hurts.
        app.world_mut().get_mut::<Kinematics>(e).unwrap().body.pos = OUT;
        let hp = app.world().get::<Health>(e).unwrap().hp;
        tick(&mut app, 180);
        let left = app.world().get::<Burning>(e).map_or(0.0, |b| b.share);
        assert!(left > 0.5, "a big fire holds on: {left} after 3 s");
        assert!(app.world().get::<Health>(e).unwrap().hp < hp - 20.0, "burning hurts");
        // ... then gutters out quick.
        tick(&mut app, 210);
        assert!(app.world().get::<Burning>(e).is_none(), "out in ~6 s all told");

        let mut wet = app_with("water");
        let e = creature(&mut wet, IN, Resist::default());
        wet.world_mut().entity_mut(e).insert(Burning::new(1.0));
        // (It soaks in over a moment: a tenth of a second to douse.)
        tick(&mut wet, 10);
        assert!(wet.world().get::<Burning>(e).is_none(), "water put it out");
        assert_eq!(coat(&wet, e).as_deref(), Some("wet"), "and it's wet");

        // Wet, then straight into flames: doesn't catch.
        let mut app = app_with("fire");
        let e = creature(&mut app, IN, Resist::default());
        app.world_mut().entity_mut(e).insert(Coated::with("wet", 1.0));
        tick(&mut app, 1);
        assert!(app.world().get::<Burning>(e).is_none(), "wet things don't catch");

        let mut app = app_with("fire");
        let e = creature(&mut app, IN, Resist { fireproof: true, ..default() });
        tick(&mut app, 30);
        assert!(app.world().get::<Burning>(e).is_none(), "fireproof");
    }

    #[test]
    fn falling_fast_beats_a_fire_out() {
        let mut app = app_with("stone");
        let (still, falling) = (creature(&mut app, OUT, Resist::default()), creature(&mut app, OUT + Vec2::new(20.0, 0.0), Resist::default()));
        for e in [still, falling] {
            app.world_mut().entity_mut(e).insert(Burning::new(1.0));
        }
        app.world_mut().get_mut::<Kinematics>(falling).unwrap().body.vel = Vec2::new(0.0, -300.0);
        tick(&mut app, 72);
        assert!(app.world().get::<Burning>(falling).is_none(), "falling flat out, out in a second or so");
        assert!(app.world().get::<Burning>(still).is_some_and(|b| b.share > 0.8), "standing still it burns on");
    }

    #[test]
    fn embers_underfoot_light_a_little_and_it_burns_out() {
        // A row of fire under its feet (the creature stands on it).
        let mut app = app_region("fire", 0..30, 9..10);
        let e = creature(&mut app, Vec2::new(13.0, 14.0), Resist::default());
        tick(&mut app, 30);
        let b = app.world().get::<Burning>(e).map_or(0.0, |b| b.share);
        assert!(b > 0.0 && b < 0.45, "a little of it alight: {b}");
        // Off them: out in a moment, having hurt a little.
        app.world_mut().get_mut::<Kinematics>(e).unwrap().body.pos = OUT;
        tick(&mut app, 90);
        assert!(app.world().get::<Burning>(e).is_none(), "burned out");
        let lost = 100.0 - app.world().get::<Health>(e).unwrap().hp;
        assert!(lost < 8.0, "a sting, not a burning: lost {lost}");
    }

    #[test]
    fn oil_spreads_a_spark_and_fire_dries_what_is_wet() {
        // 80 % oily, a spark on it, nowhere near flames: it spreads over
        // the oil and burns hot.
        let mut app = app_with("stone");
        let e = creature(&mut app, OUT, Resist::default());
        app.world_mut().entity_mut(e).insert((Coated::with("oily", 0.8), Burning::new(0.05)));
        tick(&mut app, 30);
        let b = *app.world().get::<Burning>(e).unwrap();
        assert!(b.share > 0.7, "over the oil: {}", b.share);
        assert!(b.power > 1.9, "hotter: ×{}", b.power);
        // A third wet (not enough to put it out), half on fire: the fire
        // dries it faster than it'd dry on its own.
        let mut app = app_with("stone");
        let e = creature(&mut app, OUT, Resist::default());
        app.world_mut().entity_mut(e).insert((Coated::with("wet", 0.3), Burning::new(0.6)));
        tick(&mut app, 60);
        let wet = share(&app, e, "wet");
        assert!(wet < 0.3 - 1.0 / 6.0 - 0.05, "dried: {wet}");
        let fire = app.world().get::<Burning>(e).map_or(0.0, |b| b.share);
        assert!(fire <= 1.0 - wet + 1e-4, "never more alight than it's dry: {fire} with {wet} wet");
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
    fn snow_underfoot_wets_your_feet_but_does_not_put_you_out() {
        let mut app = app_with("snow");
        // Standing on the snow (its box starts just above the pocket's top at 20).
        let e = creature(&mut app, Vec2::new(13.0, 24.0), Resist::default());
        app.world_mut().entity_mut(e).insert(Burning::new(1.0));
        tick(&mut app, 30);
        assert!(app.world().get::<Burning>(e).is_some(), "still burning");
        let wet = share(&app, e, "wet");
        assert!(wet > 0.0 && wet < DOUSED, "a little wet: {wet}");
    }

    #[test]
    fn a_puddle_wets_your_feet_but_wading_deep_puts_you_out() {
        // A row of water, the creature (4 × 8) standing in it (its feet at 10).
        let mut app = app_region("water", 0..30, 10..11);
        let e = creature(&mut app, Vec2::new(13.0, 14.0), Resist::default());
        app.world_mut().entity_mut(e).insert(Burning::new(1.0));
        tick(&mut app, 30);
        let wet = share(&app, e, "wet");
        assert!((0.1..DOUSED).contains(&wet), "its feet wet: {wet}");
        assert!(app.world().get::<Burning>(e).is_some(), "a puddle doesn't put a fire out");
        // Waist deep: out.
        let mut app = app_region("water", 0..30, 10..14);
        let e = creature(&mut app, Vec2::new(13.0, 14.0), Resist::default());
        app.world_mut().entity_mut(e).insert(Burning::new(1.0));
        tick(&mut app, 30);
        assert!(share(&app, e, "wet") > 0.7, "soaked: {}", share(&app, e, "wet"));
        assert!(app.world().get::<Burning>(e).is_none(), "wading deep puts it out");
    }

    #[test]
    fn a_step_in_acid_clings_and_eats_until_water_washes_it_off() {
        // A step into a shallow acid puddle, then out of it.
        let mut app = app_region("acid", 0..30, 10..11);
        let e = creature(&mut app, Vec2::new(13.0, 14.0), Resist::default());
        tick(&mut app, 15);
        let acid = share(&app, e, "acid");
        assert!(acid > 0.5, "a step coats you well: {acid}");
        app.world_mut().get_mut::<Kinematics>(e).unwrap().body.pos = Vec2::new(13.0, 40.0);
        let hp = app.world().get::<Health>(e).unwrap().hp;
        tick(&mut app, 60);
        let lost = hp - app.world().get::<Health>(e).unwrap().hp;
        assert!(lost > 2.0, "out of it, it still eats: lost {lost} in a second");
        assert!(share(&app, e, "acid") < acid, "and wears off");
        // Into water: washed off in a moment, and it stops.
        let mut water = app_with("water");
        let e = creature(&mut water, IN, Resist::default());
        water.world_mut().entity_mut(e).insert(Coated::with("acid", 0.6));
        tick(&mut water, 20);
        assert!(share(&water, e, "acid") < 0.05, "washed off: {}", share(&water, e, "acid"));
        let hp = water.world().get::<Health>(e).unwrap().hp;
        tick(&mut water, 60);
        assert!((hp - water.world().get::<Health>(e).unwrap().hp) < 0.5, "and it stopped eating");
    }

    #[test]
    fn coatings_share_you_and_venom_clings_but_water_washes_it() {
        let rules = Coatings::from_ron(include_str!("../../../../assets/data/coatings.ron")).unwrap().by_name;
        let mut c = Coated::default();
        c.soak("bloody", 0.7, &rules);
        c.soak("oily", 0.6, &rules);
        let total: f32 = c.coats.iter().map(|(_, a)| a).sum();
        assert!((total - 1.0).abs() < 1e-4 && (c.share("oily") - 0.6).abs() < 1e-4, "oil pushed some blood off: {:?}", c.coats);
        c.soak("venom", 0.5, &rules);
        c.soak("bloody", 1.0, &rules);
        assert!((c.share("venom") - 0.5).abs() < 1e-4, "blood doesn't push venom off: {:?}", c.coats);
        c.soak("wet", 1.0, &rules);
        assert_eq!(c.coats.len(), 1, "water washes everything off: {:?}", c.coats);
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
        hot.world_mut().entity_mut(e).insert(Coated::with("oily", 1.0));
        tick(&mut hot, 1);
        let b = *hot.world().get::<Burning>(e).expect("caught from heat");
        assert!(b.power > 2.0, "and burns hard and long (×{})", b.power);
        // Into water: washed off, and out.
        let mut water = app_with("water");
        let e = creature(&mut water, IN, Resist::default());
        water.world_mut().entity_mut(e).insert((Coated::with("oily", 1.0), b));
        tick(&mut water, 30);
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
    fn frost_chills_what_is_in_it_not_what_stands_on_it() {
        let mut app = app_with("stone");
        // Freeze the stone pocket hard, and stand a creature on it: ice
        // underfoot doesn't chill.
        app.world_mut().resource_mut::<SimWorld>().world.apply_edit(&WorldEdit::Heat { center: CellPos::new(13, 15), radius: 8, amount: -200 });
        let e = creature(&mut app, Vec2::new(13.0, 24.0), Resist::default());
        tick(&mut app, 1);
        assert!(app.world().get::<Chilled>(e).is_none(), "standing on the frozen stone chills nothing");
        // In it (buried in the frozen pocket), burning: chilled, and the
        // fire goes out.
        app.world_mut().get_mut::<Kinematics>(e).unwrap().body.pos = Vec2::new(13.0, 15.0);
        app.world_mut().entity_mut(e).insert(Burning::new(1.0));
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
