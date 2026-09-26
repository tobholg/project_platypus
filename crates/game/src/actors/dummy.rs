//! Training dummies (the arena's): planted where they stand (or not: a
//! sandbag goes flying), never die, and show above them what they've taken
//! over the current fight: from its first hit until it's left alone for
//! `reset_after` seconds, the damage per second, the total, how long.
//! Everything that hurts counts (weapons, spells, blasts, fire, falls),
//! since it reads `Health` like the damage numbers do.

use bevy::prelude::*;
use serde::Deserialize;

use super::animation::Animator;
use super::{Health, Kinematics};

/// A dummy's brain (its creature file's `brain.params`).
#[derive(Component, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Dummy {
    /// Stays where it first stood, whatever hits it.
    pub anchored: bool,
    /// Seconds without a hit that end a fight (the next hit starts another).
    pub reset_after: f32,
}

impl Default for Dummy {
    fn default() -> Self {
        Dummy { anchored: true, reset_after: 3.0 }
    }
}

/// The current (or last) fight: when it began, the last hit, the damage.
#[derive(Component, Default, Debug)]
pub struct Tally {
    home: Option<Vec2>,
    pub start: f32,
    pub last: f32,
    pub total: f32,
    pub hits: u32,
}

impl Tally {
    /// Damage per second over the fight (a single hit: over a second).
    pub fn dps(&self) -> f32 {
        self.total / (self.last - self.start).max(1.0)
    }
}

#[derive(Component)]
pub struct TallyLabel;

type Dummies<'a> = (Entity, &'a Dummy, &'a mut Health, &'a mut Kinematics, Option<&'a mut Tally>, Option<&'a mut Animator>);

/// After the damage numbers have seen a hit, before deaths: count it, heal
/// it back, and flinch; planted dummies go back where they stood.
pub fn tally(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<Dummies>,
) {
    let now = time.elapsed_secs();
    for (e, d, mut h, mut k, tally, anim) in &mut q {
        let Some(mut t) = tally else {
            commands.entity(e).insert(Tally::default()).with_child((
                TallyLabel,
                Text2d::new(""),
                TextFont { font_size: FontSize::Px(6.0), ..default() },
                TextColor(Color::srgba(1.0, 0.95, 0.8, 0.0)),
                Transform::from_xyz(0.0, k.body.half.y + 10.0, 30.0),
            ));
            continue;
        };
        let lost = h.max - h.hp;
        h.hp = h.max;
        if lost > 0.01 {
            if t.total == 0.0 || now - t.last > d.reset_after {
                *t = Tally { home: t.home, start: now, last: now, total: 0.0, hits: 0 };
            }
            t.total += lost;
            t.hits += 1;
            t.last = now;
            if let Some(mut a) = anim {
                a.play("hit");
            }
        }
        if d.anchored {
            if t.home.is_none() && k.loco.grounded() {
                t.home = Some(k.body.pos);
            }
            if let Some(home) = t.home {
                k.body.pos = home;
                k.body.vel = Vec2::ZERO;
            }
        }
    }
}

/// The readout over each dummy: bright during a fight, dim after it.
pub fn show(
    time: Res<Time<Fixed>>,
    dummies: Query<(&Dummy, &Tally, &Children)>,
    mut labels: Query<(&mut Text2d, &mut TextColor), With<TallyLabel>>,
    mut anims: Query<&mut Animator, With<Dummy>>,
) {
    let now = time.elapsed_secs();
    for (d, t, children) in &dummies {
        for &c in children {
            let Ok((mut text, mut color)) = labels.get_mut(c) else { continue };
            if t.hits == 0 {
                color.0.set_alpha(0.0);
                continue;
            }
            text.0 = format!("{:.0} dps\n{:.0} in {:.1}s", t.dps(), t.total, (t.last - t.start).max(0.0));
            color.0.set_alpha(if now - t.last > d.reset_after { 0.45 } else { 1.0 });
        }
    }
    // The flinch is played once; after it, back to standing.
    for mut a in &mut anims {
        if a.force.is_some() && a.finished() {
            a.force = None;
        }
    }
}
