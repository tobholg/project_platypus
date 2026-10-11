//! The dead coming up out of the ground (a zombie: `emerges` in life.ron),
//! and burning in the sun (`sunburns` in its creature file).
//!
//! One that comes up out of the ground is put standing where it'll stand,
//! hidden and still (`Dormant`), and drawn sunk into the earth: only what
//! of it is above ground shows, rising over `EMERGE` s, its arms up (its
//! reach: a hand first, then the other, its head, its body heaved up in
//! jerks), the earth puffing and spilling round it, a groan; then it's
//! awake. (Several together come up one after another: `delay`.)
//!
//! In the sun (by day, nothing solid over its head), one that sunburns
//! smoulders and catches: it burns as anything burns (rain puts it out),
//! and its body, if it lies there to get up, burns away (`rise::kindle`).

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind};

use crate::creatures::{Dormant, Kinematics};
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// How long coming up takes (seconds), and the jerks in it (of the way).
const EMERGE: f32 = 2.6;
const JERK: f32 = 0.06;
/// How far up a sunlit one looks for something over its head (cells).
const SKY: i32 = 160;
/// The sun's up between these (of the day: 0 midnight, 0.5 noon).
const SUNRISE: f32 = 0.27;
const SUNSET: f32 = 0.73;
/// The least light it's drawn in (it isn't lit as the world is).
const DARKEST: f32 = 0.5;
/// How much it burns in the sun (at least).
const SUN_FIRE: f32 = 0.6;

/// Coming up out of the ground: how far along (below 0: still waiting
/// its turn), and its picture (arms up), made as it starts.
#[derive(Component)]
pub struct Emerging {
    t: f32,
    pixels: Vec<(Vec2, [f32; 4])>,
}

impl Emerging {
    /// Starting after `delay` s.
    pub fn after(delay: f32) -> Self {
        Emerging { t: -delay, pixels: Vec::new() }
    }
}

/// The sun's up now.
pub fn sun_up(day: &crate::light::Daylight) -> bool {
    (SUNRISE..SUNSET).contains(&day.time)
}

/// Open to the sun at `at` (by day, nothing solid over it).
pub fn sunlit(sim: &SimWorld, day: &crate::light::Daylight, at: Vec2) -> bool {
    sun_up(day) && sim.world.under_sky(CellPos::from_world(at.x, at.y), SKY)
}

/// How far up it is (of the way), and how far its picture's sunk (cells).
fn rise(em: &Emerging) -> (f32, f32) {
    let top = em.pixels.iter().fold(f32::MIN, |m, (p, _)| m.max(p.y));
    let bottom = em.pixels.iter().fold(f32::MAX, |m, (p, _)| m.min(p.y));
    let tall = (top - bottom).max(1.0);
    // (Slow to break the surface, heaving up in jerks.)
    let p = (em.t / EMERGE).clamp(0.0, 1.0);
    let ease = (p * p * (3.0 - 2.0 * p) + JERK * (p * 23.0).sin() * (1.0 - p)).clamp(0.0, 1.0);
    (p, (1.0 - ease) * (tall + 1.0))
}

/// What's above ground of one coming up shows; under it, the earth hides
/// it. (Drawn each frame, as it is now.)
/// (Drawn over the lighting, it's lit by hand: as lit as where it is,
/// never quite black.)
pub fn draw(
    sim: Res<SimWorld>,
    day: Res<crate::light::Daylight>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    q: Query<(&Emerging, &Kinematics)>,
    lights: Query<(&GlobalTransform, &crate::light::LightSource)>,
) {
    if q.is_empty() {
        return;
    }
    let lights: Vec<(Vec2, f32)> = lights.iter().map(|(t, l)| (t.translation().truncate(), l.color.iter().copied().fold(0.0f32, f32::max))).filter(|(_, b)| *b > 0.05).collect();
    for (em, k) in &q {
        if em.t < 0.0 || em.pixels.is_empty() {
            continue;
        }
        let (_, sink) = rise(em);
        let l = crate::creatures::brain::senses::lit(&sim, &day, &lights, k.body.pos).clamp(DARKEST, 1.0);
        let feet = k.body.pos - Vec2::Y * k.body.half.y;
        for &(at, c) in &em.pixels {
            let w = k.body.pos + at - Vec2::Y * sink;
            let open = sim.world.get(CellPos::from_world(w.x, w.y)).is_none_or(|cell| cell.is_air() || !matches!(sim.world.materials().phys(cell.material).kind, Kind::Static | Kind::Powder));
            if open && w.y >= feet.y - 0.5 {
                sparks.draw_now(w, 1.0, [c[0] * l, c[1] * l, c[2] * l, c[3]]);
            }
        }
    }
}

/// Coming up: its time runs, the earth puffs and spills; out, it's awake.
#[allow(clippy::type_complexity)]
pub fn emerge(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut q: Query<(Entity, &mut Emerging, &Kinematics, &super::animation::Animator, Option<&super::parts::Severed>)>,
) {
    for (e, mut em, k, anim, severed) in &mut q {
        em.t += DT;
        if em.t < 0.0 {
            continue;
        }
        let feet = k.body.pos - Vec2::Y * k.body.half.y;
        if em.pixels.is_empty() {
            let lost = severed.map(|s| s.0.clone()).unwrap_or_default();
            em.pixels = super::rise::picture(anim, k, "tell", &lost);
            sounds.write(crate::sound::PlaySound::at("mine_dirt", feet).pitch(0.6));
        }
        let (p, _) = rise(&em);
        // The earth it comes up through: puffs, and some spilled.
        let ground = sim.world.get(CellPos::from_world(feet.x, feet.y - 1.0));
        let tick = sim.world.tick();
        if let Some(cell) = ground.filter(|c| !c.is_air())
            && tick.is_multiple_of(4)
        {
            let dx = ((tick / 4) % 7) as f32 - 3.0;
            sim.world.puff([feet.x + dx, feet.y + 0.5], cell, 3, 22.0);
        }
        if (0.4..0.4 + DT).contains(&p) {
            sounds.write(crate::sound::PlaySound::at("groan", k.body.pos));
        }
        if p >= 1.0 {
            if let Some(cell) = ground.filter(|c| !c.is_air()) {
                sim.world.splash([feet.x, feet.y + 1.0], cell.material, 8, 1.4);
            }
            commands.entity(e).remove::<(Emerging, Dormant)>().insert(Visibility::Inherited);
        }
    }
}

/// In the sun, what sunburns burns (rain or water puts it out, as any
/// fire); a fresh fire on it at least `SUN_FIRE`.
#[allow(clippy::type_complexity)]
pub fn sunburn(
    mut commands: Commands,
    sim: Res<SimWorld>,
    day: Res<crate::light::Daylight>,
    mut clock: Local<f32>,
    mut q: Query<(Entity, &Kinematics, &super::animation::Animator, Option<&mut super::elements::Burning>), (Without<Dormant>, Without<Emerging>)>,
) {
    *clock += DT;
    if *clock < 0.25 || !sun_up(&day) {
        return;
    }
    *clock = 0.0;
    for (e, k, anim, burning) in &mut q {
        if !anim.def.sunburns || !sunlit(&sim, &day, k.body.pos + Vec2::Y * k.body.half.y) {
            continue;
        }
        match burning {
            Some(mut b) => b.share = b.share.max(SUN_FIRE),
            None => {
                commands.entity(e).insert(super::elements::Burning::new(SUN_FIRE));
            }
        }
    }
}
