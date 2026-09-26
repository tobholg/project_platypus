//! Bows and arrows (SPEC 6.3). Anything wielding a bow draws it while it
//! asks (`DrawBow` each tick: the player holding the left button, an
//! archer's brain), and looses when it stops asking: an arrow at the speed,
//! damage and knockback of how far it was drawn (`weapons.ron`).
//!
//! Arrows are bodies of their own: they fly and fall a cell at a time,
//! turned to where they're going; strike a body (pixel against its frame,
//! as a blade does: one `Hit`) or stick in stone and earth, where they stay
//! until you walk near and take them back. Through fire or lava one catches
//! and burns (it lights its way and sets alight what it strikes); water
//! puts it out and slows it.

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind, WorldEdit};

use crate::actors::animation::{Aiming, Animator, HandPos, pixel_at};
use crate::actors::player::LocalPlayer;
use crate::actors::{Health, Kinematics, Team};
use crate::combat::{Hit, Invulnerable, Stamina, Turned, Weapons, Wielding};
use crate::hands::items::{Inventory, Items};
use crate::light::LightSource;
use crate::light::torch::Flame;
use crate::world::{SimWorld, TICK_HZ, TickSet};

pub struct ArcheryPlugin;

impl Plugin for ArcheryPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DrawBow>()
            .add_systems(FixedUpdate, nock.after(TickSet::Intent).before(TickSet::Bodies))
            .add_systems(FixedUpdate, fly.after(TickSet::Bodies).before(TickSet::Cells))
            .add_systems(Update, (take_back, show).after(crate::actors::animation::animate));
    }
}

const DT: f32 = 1.0 / TICK_HZ as f32;
/// An arrow in flight this long without striking anything is gone.
const FLIGHT: f32 = 8.0;
/// Speed kept per cell of liquid an arrow goes through.
const WADE: f32 = 0.93;
/// Too slow to fly on: it drops.
const SPENT: f32 = 25.0;

/// Draw (or keep drawing) the bow, aimed at `at`. Stop asking to loose.
#[derive(Message, Clone, Copy, Debug)]
pub struct DrawBow {
    pub archer: Entity,
    pub at: Vec2,
}

/// A bow being drawn: which, for how long, at what.
#[derive(Component, Debug)]
pub struct Nocked {
    pub bow: usize,
    pub t: f32,
    pub at: Vec2,
    asked: bool,
}

impl Arrow {
    pub fn burning(&self) -> bool {
        self.burning > 0.0
    }
}

impl Nocked {
    /// How far it's drawn, 0..1.
    pub fn drawn(&self, weapons: &Weapons) -> f32 {
        (self.t / weapons.bow(self.bow).draw.max(0.01)).min(1.0)
    }
}

/// An arrow: flying (`stuck` none) or stuck in something for `stuck` s.
#[derive(Component, Debug)]
pub struct Arrow {
    pub vel: Vec2,
    shooter: Entity,
    team: Option<Team>,
    damage: f32,
    crit: bool,
    knock: f32,
    stun: f32,
    /// Seconds it still burns.
    burning: f32,
    age: f32,
    stuck: Option<f32>,
    /// Which way it points (stuck, it keeps it).
    angle: f32,
}

type Bowman<'a> = (Entity, &'a Wielding, &'a Kinematics, Option<&'a HandPos>, Option<&'a Team>, Option<&'a mut Nocked>, Option<&'a mut Stamina>, Option<&'a mut Inventory>, Option<&'a crate::gear::Stats>);

/// Drawing while asked; loosing when not.
#[allow(clippy::too_many_arguments)]
fn nock(
    mut commands: Commands,
    mut asks: MessageReader<DrawBow>,
    weapons: Option<Res<Weapons>>,
    items: Option<Res<Items>>,
    sim: Res<crate::world::SimWorld>,
    mut archers: Query<Bowman>,
) {
    let Some(weapons) = weapons else { return };
    for ask in asks.read() {
        let Ok((_, wielding, _, _, _, nocked, _, _, stats)) = archers.get_mut(ask.archer) else { continue };
        let Some(bow) = wielding.0.as_deref().and_then(|id| weapons.bow_index(id)) else { continue };
        let speed = stats.map_or(1.0, |s| s.mult(crate::gear::Stat::AttackSpeed));
        match nocked {
            Some(mut n) => {
                n.t += DT * speed;
                n.at = ask.at;
                n.asked = true;
            }
            None => {
                commands.entity(ask.archer).insert(Nocked { bow, t: 0.0, at: ask.at, asked: true });
            }
        }
    }
    let arrow_item = items.as_ref().and_then(|i| i.id("arrow"));
    let tick = sim.world.tick();
    for (e, _, k, hand, team, nocked, stamina, inv, stats) in &mut archers {
        let Some(mut n) = nocked else { continue };
        if n.asked {
            n.asked = false;
            commands.entity(e).insert(Aiming { at: n.at, left: 0.1 });
            continue;
        }
        // Let go: loose, if there's an arrow to (the player's pack; others
        // don't run out).
        commands.entity(e).remove::<Nocked>();
        if let (Some(mut inv), Some(items), Some(item)) = (inv, items.as_ref(), arrow_item)
            && !inv.take_item(items, item, 1)
        {
            continue;
        }
        let def = weapons.bow(n.bow);
        let d = n.drawn(&weapons);
        if d < 0.1 {
            continue;
        }
        if let Some(mut s) = stamina {
            s.spend(def.stamina);
        }
        let lerp = |(a, b): (f32, f32)| a + (b - a) * d;
        let from = hand.and_then(|h| h.at).unwrap_or(k.body.pos);
        let dir = (n.at - from).normalize_or(Vec2::X * k.loco.facing);
        let roll = (platypus_sim::rng::hash(&[tick, e.to_bits(), n.t.to_bits() as u64]) % 10_000) as f32 / 10_000.0;
        let (damage, knock, crit) = stats.cloned().unwrap_or_default().strike(lerp(def.damage), lerp(def.knock), roll);
        spawn_arrow(&mut commands, &weapons, from + dir * 3.0, dir * lerp(def.speed), e, team.copied(), (damage, crit), knock, def.stun);
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_arrow(commands: &mut Commands, weapons: &Weapons, at: Vec2, vel: Vec2, shooter: Entity, team: Option<Team>, (damage, crit): (f32, bool), knock: f32, stun: f32) {
    let Some(turned) = weapons.arrow.as_ref() else { return };
    let angle = vel.y.atan2(vel.x).to_degrees();
    commands.spawn((
        Name::new("Arrow"),
        Arrow { vel, shooter, team, damage, crit, knock, stun, burning: 0.0, age: 0.0, stuck: None, angle },
        turned.sprite(angle),
        Transform::from_translation(at.extend(12.0)),
    ));
}

type Target<'a> = (Entity, &'a Kinematics, Option<&'a Team>, Option<&'a Animator>, Has<Invulnerable>);

/// Arrows fly a cell at a time: fall, catch fire, wade, stick, strike.
#[allow(clippy::too_many_arguments)]
fn fly(
    mut commands: Commands,
    weapons: Option<Res<Weapons>>,
    mut sim: ResMut<SimWorld>,
    mut hits: MessageWriter<Hit>,
    mut arrows: Query<(Entity, &mut Arrow, &mut Transform)>,
    targets: Query<Target, With<Health>>,
) {
    let Some(def) = weapons.as_ref().and_then(|w| w.arrow_def().cloned()) else { return };
    for (e, mut a, mut tf) in &mut arrows {
        let was_burning = a.burning > 0.0;
        a.burning = (a.burning - DT).max(0.0);
        if let Some(left) = a.stuck {
            let left = left - DT;
            a.stuck = Some(left);
            let tip = tf.translation.truncate() + Vec2::from_angle(a.angle.to_radians()) * 4.0;
            let held = sim.world.get(CellPos::from_world(tip.x, tip.y)).is_some_and(|c| matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder));
            if left <= 0.0 {
                commands.entity(e).despawn();
            } else if !held {
                // What held it is gone: it falls.
                a.stuck = None;
                a.vel = Vec2::ZERO;
            }
            if was_burning != (a.burning > 0.0) {
                commands.entity(e).remove::<(Flame, LightSource)>();
            }
            continue;
        }
        a.age += DT;
        if a.age > FLIGHT {
            commands.entity(e).despawn();
            continue;
        }
        a.vel.y -= def.gravity * DT;
        let step = a.vel * DT;
        let n = step.length().ceil().max(1.0) as usize;
        let mut pos = tf.translation.truncate();
        let dir = a.vel.normalize_or(Vec2::X);
        a.angle = dir.y.atan2(dir.x).to_degrees();
        'flight: for _ in 0..n {
            pos += step / n as f32;
            let tip = pos + dir * 4.0;
            let p = CellPos::from_world(tip.x, tip.y);
            if let Some(c) = sim.world.get(p) {
                let ph = *sim.world.materials().phys(c.material);
                if ph.kind == Kind::Fire || (ph.hot && ph.kind == Kind::Liquid) {
                    a.burning = def.burn;
                } else if ph.kind == Kind::Liquid {
                    a.vel *= WADE;
                    if ph.flammability == 0 {
                        a.burning = 0.0;
                    }
                }
                if matches!(ph.kind, Kind::Static | Kind::Powder) {
                    a.stuck = Some(def.stuck);
                    if a.burning > 0.0 {
                        sim.world.apply_edit(&WorldEdit::Ignite { center: p, radius: 1 });
                    }
                    break 'flight;
                }
            } else {
                // Off the loaded world.
                commands.entity(e).despawn();
                break 'flight;
            }
            for (te, tk, tteam, anim, safe) in &targets {
                if te == a.shooter || safe {
                    continue;
                }
                if let (Some(x), Some(y)) = (a.team, tteam)
                    && x == *y
                    && x != Team::Neutral
                {
                    continue;
                }
                if (tip - tk.body.pos).abs().cmpgt(tk.body.half + 2.0).any() {
                    continue;
                }
                let struck = match anim.and_then(|an| an.def.rig.as_ref().map(|r| (an, r))) {
                    Some((an, rig)) => {
                        let (px, py) = pixel_at(&an.def, tk, tip);
                        rig.frames.get(an.shown).is_some_and(|f| f.opaque(px, py))
                    }
                    None => (tip - tk.body.pos).abs().cmple(tk.body.half).all(),
                };
                if !struck {
                    continue;
                }
                let push = (Vec2::new(dir.x, 0.0).normalize_or(Vec2::X) + Vec2::new(0.0, 0.3)).normalize() * a.knock;
                hits.write(Hit { target: te, damage: a.damage, knock: push, stun: a.stun, at: tip, dir, weight: a.damage / 12.0, crit: a.crit });
                if a.burning > 0.0 {
                    sim.world.apply_edit(&WorldEdit::Ignite { center: p, radius: 1 });
                }
                commands.entity(e).despawn();
                break 'flight;
            }
        }
        if a.stuck.is_none() && a.vel.length() < SPENT && a.age > 0.3 {
            a.vel = Vec2::new(a.vel.x * 0.5, a.vel.y.min(0.0));
        }
        tf.translation = pos.extend(tf.translation.z);
        // Burning: fire at its tip, and light.
        match (was_burning, a.burning > 0.0) {
            (false, true) => {
                commands.entity(e).try_insert((Flame::at(Vec2::ZERO), LightSource { color: [1.0, 0.55, 0.2], flicker: 1.0 }));
            }
            (true, false) => {
                commands.entity(e).remove::<(Flame, LightSource)>();
            }
            _ => {}
        }
    }
}

/// A stuck arrow near the player comes back to it (as an item, picked up
/// like anything dropped).
fn take_back(
    mut commands: Commands,
    weapons: Option<Res<Weapons>>,
    items: Option<Res<Items>>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    arrows: Query<(Entity, &Arrow, &Transform)>,
) {
    let (Some(weapons), Some(items), Ok(k)) = (weapons, items, player.single()) else { return };
    let (Some(def), Some(item)) = (weapons.arrow_def(), items.id("arrow")) else { return };
    for (e, a, tf) in &arrows {
        let at = tf.translation.truncate();
        if a.stuck.is_some() && at.distance(k.body.pos) < def.pickup {
            commands.entity(e).despawn();
            crate::hands::spawn_drop(&mut commands, &items, at, crate::hands::items::Stack::new(item, 1));
        }
    }
}

/// Arrows turned to where they point.
fn show(weapons: Option<Res<Weapons>>, mut arrows: Query<(&Arrow, &mut Sprite)>) {
    let Some(weapons) = weapons else { return };
    if weapons.arrow.is_none() {
        return;
    }
    for (a, mut sp) in &mut arrows {
        if let Some(atlas) = sp.texture_atlas.as_mut() {
            atlas.index = Turned::index(a.angle);
        }
    }
}
