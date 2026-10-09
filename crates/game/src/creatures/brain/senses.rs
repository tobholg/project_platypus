//! Senses and alertness (BE `behaviour` stage 2): a hunter no longer knows
//! where you are from across the map, through rock, in the dark. It sees
//! (a clear line from its eye to you, out to its sight, less in the dark
//! unless it sees in the dark: daylight on you under open sky, a torch
//! or a flashlight you carry or a torch near you gives you away; less
//! behind it), hears (a
//! fight's blows, a blast, your footsteps when you run, your landings:
//! each a noise with a reach; crouched and creeping, Ctrl held, is under
//! what it hears, and lower to see over cover), and smells (a wounded quarry, within its smell, through rock).
//! Each hunter is `Idle` (wandering), `Suspicious` (it glimpsed or heard
//! something: it turns and comes to look, slowly; a glimpse held long
//! enough, or seen close, and it's hunting), `Hunting` (it knows where you
//! are and goes for you: only now does it attack), or `Searching` (it lost
//! you: where it last knew you were, a while, then it gives up; a
//! suspicious one, once it's been to look). Struck,
//! it hunts whatever's near enough to have done it. A "?" over its head
//! while it's suspicious or searching, a "!" as it starts hunting.

use bevy::prelude::*;
use serde::Deserialize;

use crate::creatures::{Health, Kinematics, Team};
use crate::world::SimWorld;

/// How a hunter senses (its brain's `senses`).
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct SensesDef {
    /// How far it sees in good light (cells; 0: its `aggro`).
    pub sight: f32,
    /// How well it sees in the dark (0: not at all past a quarter of its
    /// sight, 1: as by day: a spider, a machine's sensors).
    pub dark: f32,
    /// How well it hears (a noise's reach, times this).
    pub hearing: f32,
    /// How far it smells a wounded quarry (cells; 0: it doesn't).
    pub smell: f32,
    /// How long it goes on hunting what it no longer senses (s), before it
    /// searches.
    pub memory: f32,
    /// How far it sees behind it, against ahead, while it isn't hunting
    /// (hunting, it's turned to look).
    pub behind: f32,
}

impl Default for SensesDef {
    fn default() -> Self {
        SensesDef { sight: 0.0, dark: 0.25, hearing: 1.0, smell: 0.0, memory: 4.0, behind: 0.4 }
    }
}

/// How wary it is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Wary {
    #[default]
    Idle,
    Suspicious,
    Hunting,
    Searching,
}

/// A hunter's alertness: how wary, what it's after and where it last knew
/// that to be, whether it senses it now, seconds in this state and since
/// it last sensed it, how much it's glimpsed (to 1: hunting), its mark
/// over its head.
#[derive(Component, Debug, Default)]
pub struct Alert {
    pub wary: Wary,
    pub target: Option<Entity>,
    pub last: Vec2,
    pub senses_it: bool,
    since: f32,
    unseen: f32,
    glimpse: f32,
    mark: Option<Entity>,
    /// Seconds the "!" still shows.
    shout: f32,
    /// Told where its quarry is, always (a scenario's: as before senses,
    /// it knows).
    told: bool,
    /// Running for it: badly hurt (its tactics' `flee`), not cornered.
    pub fleeing: bool,
    /// How lit its quarry was when it last sensed it (0..1).
    pub quarry_lit: f32,
    /// Cornered and turned to fight: at bay till what it fights is well
    /// off again.
    at_bay: bool,
    /// On its way round to your far side (a flanker, `hunter.rs`): no move
    /// till it's there.
    pub flanking: bool,
}

impl Alert {
    /// Already hunting what's at `at`, and always told where it is (a
    /// scenario that wants it on you, not finding you out).
    pub fn hunting(at: Vec2) -> Alert {
        Alert { wary: Wary::Hunting, last: at, senses_it: true, told: true, ..default() }
    }

    /// Looking, not chasing: suspicious or searching (it goes slower).
    pub fn looking(&self) -> bool {
        matches!(self.wary, Wary::Suspicious | Wary::Searching)
    }

    /// It's hunting and senses its quarry now, and isn't running for it:
    /// it may attack.
    pub fn engaged(&self) -> bool {
        self.wary == Wary::Hunting && self.senses_it && !self.fleeing
    }

    /// Where it's going, if anywhere: its quarry's last known place
    /// (hunting, suspicious or searching).
    pub fn goal(&self) -> Option<Vec2> {
        (self.wary != Wary::Idle).then_some(self.last)
    }

    fn set(&mut self, wary: Wary) {
        if self.wary != wary {
            if wary == Wary::Hunting {
                self.shout = 0.9;
            }
            self.wary = wary;
            self.since = 0.0;
        }
    }
}

/// What `perceive` reads of each hunter.
type Perceiver<'a> = (
    Entity,
    &'a super::hunter::Hunter,
    &'a Kinematics,
    &'a Transform,
    Option<&'a mut Alert>,
    Option<&'a crate::clock::Keeps>,
    (Option<&'a Health>, Option<&'a super::tactics::Pack>, Option<&'a crate::creatures::Creature>),
);

/// One starting to hunt, calling: from where, how far its kind hear it
/// (its `call`; its pack always do), its pack, its kind, what it's after
/// and where.
struct Call {
    from: Entity,
    at: Vec2,
    reach: f32,
    pack: Option<u64>,
    kind: Option<String>,
    target: Option<Entity>,
    last: Vec2,
}

/// Cornered: a wall at its back and what it fights within this (cells).
const CORNERED: f32 = 60.0;

/// A noise this tick: where, how far it carries (cells), and how much
/// it makes a listener sure something's there (a share of the way to
/// hunting: a lump for a noise once, a second's worth for one going on).
struct Noise {
    at: Vec2,
    reach: f32,
    sure: f32,
}

/// How sure a noise makes it: a landing, once; running feet, each second
/// it goes on (less what fades, 0.4 a second: under a second of running
/// in its hearing and it hunts the sound). Blows and blasts only
/// make it come and look (a lure).
const SURE_LANDED: f32 = 0.3;
const SURE_RUNNING: f32 = 1.6;

/// How lit a spot is for seeing it (0 dark, 1 broad day): daylight under
/// open sky near the surface (no rock over it), and light sources near it
/// (a torch carried or planted, glowing things).
fn lit(sim: &SimWorld, day: &crate::light::Daylight, lights: &[(Vec2, f32)], at: Vec2) -> f32 {
    let sky = day.sky.iter().copied().fold(0.0f32, f32::max).clamp(0.0, 1.0);
    let open = sim.generator.surface_hint(at.x as i32).is_none_or(|s| at.y > s as f32 - 20.0) && crate::creatures::moves::clear(sim, at, at + Vec2::Y * SKY_ROOF);
    let near = lights.iter().map(|(p, b)| b * (1.0 - p.distance(at) / 70.0).max(0.0)).fold(0.0f32, f32::max);
    (if open { sky } else { 0.0 }).max(near.min(1.0))
}

const NOISE_FIGHT: f32 = 260.0;

/// Crouched, it's seen this share of its half height lower than its
/// middle (a crouch is about a third lower).
const CROUCH_LOW: f32 = 0.6;

/// Where its mark sits in depth: over the lighting.
const MARK_Z: f32 = 20.0;

/// How high over a spot rock still shades it from the sky (cells).
const SKY_ROOF: f32 = 120.0;

/// Each hunter senses what's about it and grows more or less wary.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn perceive(
    mut commands: Commands,
    sim: Res<SimWorld>,
    day: Res<crate::light::Daylight>,
    mut hunters: Query<Perceiver>,
    quarry: Query<(Entity, &Kinematics, &Team, &Health), Without<super::villager::Hiding>>,
    lights: Query<(&GlobalTransform, &crate::light::LightSource)>,
    toggles: Res<crate::light::LightToggles>,
    me: Query<Entity, With<crate::creatures::player::LocalPlayer>>,
    mut felt: MessageReader<crate::combat::Felt>,
    mut blasts: MessageReader<crate::fx::Explosion>,
    mut landed: MessageReader<crate::creatures::Landed>,
) {
    let dt = (1.0 / crate::world::TICK_HZ) as f32;
    let lights: Vec<(Vec2, f32)> = lights.iter().map(|(t, l)| (t.translation().truncate(), l.color.iter().copied().fold(0.0f32, f32::max))).filter(|(_, b)| *b > 0.05).collect();
    // The noises made this tick: blows, blasts, landings, and quarry on the
    // run (its feet).
    let mut noises: Vec<Noise> = Vec::new();
    let mut struck: Vec<Entity> = Vec::new();
    for f in felt.read() {
        noises.push(Noise { at: f.at, reach: NOISE_FIGHT, sure: 0.0 });
        struck.push(f.target);
    }
    for b in blasts.read() {
        noises.push(Noise { at: b.at, reach: 200.0 + b.radius * 12.0, sure: 0.0 });
    }
    for l in landed.read() {
        if let Ok((_, k, t, _)) = quarry.get(l.entity)
            && t.hunted()
        {
            noises.push(Noise { at: k.body.pos, reach: 40.0 + l.drop * 0.4, sure: SURE_LANDED });
        }
    }
    // (Each one seen at its middle; crouched, low, so cover a little under
    // its height hides it.)
    let hunted: Vec<(Entity, Vec2, f32, Vec2, bool)> = quarry
        .iter()
        .filter(|(_, _, t, _)| t.hunted())
        .map(|(e, k, _, h)| (e, k.body.pos - Vec2::Y * if k.loco.crouching { k.body.half.y * CROUCH_LOW } else { 0.0 }, h.hp / h.max.max(1.0), k.body.vel, k.loco.grounded()))
        .collect();
    // How lit each is (the same for every eye on it): a flashlight in
    // the player's hand as bright as a torch (it's a lamp, at you; a torch
    // in the hand is a light source of its own).
    let lamp = matches!(toggles.carry, crate::light::Carry::SmallBeam | crate::light::Carry::BigBeam);
    let light: Vec<f32> = hunted.iter().map(|(q, at, ..)| if lamp && me.contains(*q) { 1.0 } else { lit(&sim, &day, &lights, *at) }).collect();
    for (_, at, _, vel, grounded) in &hunted {
        let run = vel.x.abs();
        if *grounded && run > 40.0 {
            noises.push(Noise { at: *at, reach: 30.0 + run * 0.55, sure: SURE_RUNNING * dt });
        }
    }
    // Calls: one starting to hunt brings its pack, and its kind near
    // (its tactics' `call`).
    let mut calls: Vec<Call> = Vec::new();
    for (e, h, k, tf, alert, keeps, (health, pack, kind)) in &mut hunters {
        let Some(mut a) = alert else {
            commands.entity(e).insert(Alert::default());
            continue;
        };
        let s = &h.senses;
        let pos = k.body.pos;
        let eye = pos + Vec2::Y * k.body.half.y * 0.6;
        let sight = if s.sight > 0.0 { s.sight } else { h.aggro };
        let home = keeps.filter(|_| h.leash > 0.0).map(|kp| Vec2::new(kp.0.0 as f32, kp.0.1 as f32));
        // What it senses: seen (in sight, in the light or seeing in the
        // dark, nothing in the way) or smelled (wounded, near enough).
        let mut best: Option<(Entity, Vec2, f32, bool, f32)> = None;
        for (&(q, at, share, ..), &light) in hunted.iter().zip(&light) {
            if home.is_some_and(|hm| at.distance(hm) > h.leash) {
                continue;
            }
            let d = at.distance(pos);
            let back = a.wary != Wary::Hunting && (at.x - pos.x) * k.loco.facing < 0.0;
            let see = sight * (0.25 + 0.75 * (s.dark + (1.0 - s.dark) * light)) * if back { s.behind } else { 1.0 };
            let seen = a.told || d < see && crate::creatures::moves::clear(&sim, eye, at);
            let smelled = s.smell > 0.0 && d < s.smell && share < 0.75;
            if (seen || smelled) && best.is_none_or(|b| d < b.2) {
                // (Close in its sight, or smelled: no doubt about it.)
                best = Some((q, at, d, smelled || d < see * 0.45, light));
            }
        }
        let was = a.wary;
        a.since += dt;
        a.shout = (a.shout - dt).max(0.0);
        match best {
            Some((q, at, _, sure, light)) => {
                a.quarry_lit = light;
                a.target = Some(q);
                a.last = at;
                a.unseen = 0.0;
                a.senses_it = true;
                match a.wary {
                    Wary::Hunting => {}
                    Wary::Searching => a.set(Wary::Hunting),
                    Wary::Idle | Wary::Suspicious => {
                        a.glimpse += dt * if sure { 6.0 } else { 1.6 };
                        let now = if a.glimpse >= 1.0 { Wary::Hunting } else { Wary::Suspicious };
                        a.set(now);
                    }
                }
            }
            None => {
                a.senses_it = false;
                a.unseen += dt;
                a.glimpse = (a.glimpse - dt * 0.4).max(0.0);
                match a.wary {
                    Wary::Hunting if a.unseen > s.memory => a.set(Wary::Searching),
                    Wary::Searching if a.since > 5.0 => a.set(Wary::Idle),
                    // (Once it's had a look where it was: there, or long
                    // on the way.)
                    Wary::Suspicious if a.glimpse <= 0.0 && (a.since > 4.0 && a.last.distance(pos) < 40.0 || a.since > 10.0) => a.set(Wary::Idle),
                    _ => {}
                }
            }
        }
        // Heard: it comes to look (not when it's already on the hunt);
        // heard enough (feet running on), it hunts the sound.
        if matches!(a.wary, Wary::Idle | Wary::Suspicious | Wary::Searching)
            && !a.senses_it
            && let Some(n) = noises.iter().filter(|n| n.at.distance(pos) < n.reach * s.hearing).min_by(|x, y| x.at.distance(pos).total_cmp(&y.at.distance(pos)))
        {
            a.last = n.at;
            a.glimpse += n.sure;
            if a.glimpse >= 1.0 {
                a.unseen = 0.0;
                a.set(Wary::Hunting);
            } else if a.wary == Wary::Idle {
                a.set(Wary::Suspicious);
            }
            a.since = 0.0;
        }
        // Struck: it goes for whatever's near enough to have done it.
        if struck.contains(&e) {
            if let Some(&(q, at, ..)) = hunted.iter().filter(|(_, at, ..)| at.distance(pos) < sight * 1.5).min_by(|x, y| x.1.distance(pos).total_cmp(&y.1.distance(pos))) {
                a.target = Some(q);
                a.last = at;
                a.senses_it = true;
                a.unseen = 0.0;
                a.glimpse = 1.0;
            }
            a.set(Wary::Hunting);
        }
        // Badly hurt: it runs for it, unless it's cornered (a wall at its
        // back, what it fights near).
        let share = health.map_or(1.0, |hp| hp.hp / hp.max.max(1.0));
        let away = (pos.x - a.last.x).signum();
        let backed = if away < 0.0 { k.loco.contacts.wall_left } else { k.loco.contacts.wall_right };
        let near = a.last.distance(pos);
        a.at_bay = (a.at_bay || backed && near < CORNERED) && near < CORNERED * 1.5;
        let cornered = a.at_bay;
        let fleeing = h.tactics.flee > 0.0 && share < h.tactics.flee && a.wary == Wary::Hunting && !cornered;
        if fleeing != a.fleeing && std::env::var("PLATYPUS_ALERTLOG").is_ok() {
            info!("alert: {e:?} {} at {:.0}% health{}", if fleeing { "flees" } else { "stops fleeing" }, share * 100.0, if cornered { " (cornered)" } else { "" });
        }
        a.fleeing = fleeing;
        if was != Wary::Hunting && a.wary == Wary::Hunting && !a.told && (pack.is_some() || h.tactics.call > 0.0) {
            calls.push(Call { from: e, at: pos, reach: h.tactics.call, pack: pack.map(|p| p.id), kind: kind.map(|c| c.kind.clone()), target: a.target, last: a.last });
        }
        if was != a.wary && std::env::var("PLATYPUS_ALERTLOG").is_ok() {
            info!("alert: {e:?} {was:?} → {:?} at {:?}", a.wary, a.last.round());
        }
        // Its mark: "?" suspicious or searching, "!" just begun hunting.
        let text = match a.wary {
            Wary::Suspicious | Wary::Searching => Some("?"),
            Wary::Hunting if a.shout > 0.0 => Some("!"),
            _ => None,
        };
        // (Over the light, as damage numbers are: it reads in the dark.)
        let up = Vec3::new(0.0, k.body.half.y + 9.0, MARK_Z - tf.translation.z);
        match (a.mark, text) {
            (None, Some(t)) => {
                let mark = commands
                    .spawn((
                        Text2d::new(t),
                        TextFont { font_size: bevy::text::FontSize::Px(14.0), ..default() },
                        TextColor(mark_color(a.wary)),
                        bevy::sprite::Text2dShadow { offset: Vec2::new(1.0, -1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85) },
                        Transform::from_translation(up).with_scale(Vec3::splat(0.5)),
                    ))
                    .id();
                commands.entity(e).add_child(mark);
                a.mark = Some(mark);
            }
            (Some(m), t) => {
                let color = mark_color(a.wary);
                commands.entity(m).queue_silenced(move |mut ew: EntityWorldMut| {
                    if let Some(mut text) = ew.get_mut::<Text2d>() {
                        text.0 = t.unwrap_or("").to_string();
                    }
                    if let Some(mut c) = ew.get_mut::<TextColor>() {
                        c.0 = color;
                    }
                    if let Some(mut tf) = ew.get_mut::<Transform>() {
                        tf.translation = up;
                    }
                });
            }
            (None, None) => {}
        }
    }
    // The called: each not yet hunting that hears a call (its pack's, or
    // its kind's within the caller's `call`) hunts where the caller saw
    // its quarry, though it doesn't see it yet itself.
    if calls.is_empty() {
        return;
    }
    for (e, _, k, _, alert, _, (_, pack, kind)) in &mut hunters {
        let Some(mut a) = alert else { continue };
        if a.wary == Wary::Hunting {
            continue;
        }
        let heard = calls.iter().find(|c| {
            c.from != e
                && (pack.is_some_and(|p| c.pack == Some(p.id)) && c.at.distance(k.body.pos) < super::tactics::PACK_CALL
                    || c.reach > 0.0 && c.kind.is_some() && kind.map(|k| &k.kind) == c.kind.as_ref() && c.at.distance(k.body.pos) < c.reach)
        });
        if let Some(c) = heard {
            a.target = c.target;
            a.last = c.last;
            a.unseen = 0.0;
            a.glimpse = 1.0;
            a.set(Wary::Hunting);
            if std::env::var("PLATYPUS_ALERTLOG").is_ok() {
                info!("alert: {e:?} called by {:?}: hunting at {:?}", c.from, c.last.round());
            }
        }
    }
}

fn mark_color(w: Wary) -> Color {
    match w {
        Wary::Hunting => Color::srgb(1.0, 0.25, 0.2),
        _ => Color::srgb(1.0, 0.9, 0.35),
    }
}
