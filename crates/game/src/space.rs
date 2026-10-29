//! Space (the space arc), the game's side: sun and shade. Up in the void
//! there's no air to warm or cool you, only the sun: in its light you
//! bake (slowly at first, then harder the longer you stay in it), out of
//! it (a rock's shadow, or night, the world between you and the sun) you
//! freeze (chilled, slowed). The edge of a shadow is where to be.

use bevy::prelude::*;
use platypus_sim::{CellPos, Kind};

use crate::creatures::body::elements::Chilled;
use crate::creatures::{Health, Kinematics};
use crate::world::{SimWorld, TickSet};

pub struct SpacePlugin;

impl Plugin for SpacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, sun_and_shade.after(crate::creatures::move_creatures).in_set(TickSet::Bodies));
    }
}

/// How far into space (`sky::space_at`) the sun and the shade begin to
/// tell.
const SPACE_FROM: f32 = 0.5;
/// Looking for what shades you: this far toward the sun (cells), this
/// many cells a step, again every this many ticks.
const SHADE_REACH: i32 = 900;
const SHADE_STEP: i32 = 2;
const LOOK_EVERY: u64 = 6;
/// In the sun, it takes this long (s) to bake through (heat 0 → 1), and
/// out of it this long to cool; baked, this much harm a second at full
/// heat (less below: the square of it).
const BAKE_SECS: f32 = 8.0;
const COOL_SECS: f32 = 4.0;
const SCORCH: f32 = 9.0;
/// In shade, chilled this hard (of a frost spell's worst), refreshed for
/// this long.
const SHADE_CHILL: f32 = 0.5;
const SHADE_CHILL_SECS: f32 = 0.5;

/// In space: in the sun or not, and how baked through (0 … 1).
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct Sunlit {
    pub lit: bool,
    pub heat: f32,
}

/// Is the sun up (by day: at night the world's between you and it)?
fn sun_up(time: f32) -> bool {
    -(time * std::f32::consts::TAU).cos() > 0.0
}

/// Nothing solid between `from` and the sun, `toward` it, as far as
/// `SHADE_REACH`?
fn in_the_sun(sim: &SimWorld, from: Vec2, toward: Vec2) -> bool {
    let mats = sim.world.materials();
    (1..SHADE_REACH / SHADE_STEP).all(|i| {
        let p = from + toward * (i * SHADE_STEP) as f32;
        sim.world.get(CellPos::from_world(p.x, p.y)).is_none_or(|c| c.is_air() || !matches!(mats.phys(c.material).kind, Kind::Static | Kind::Powder))
    })
}

/// Up in space: in the sun, baked; in shade, chilled.
fn sun_and_shade(mut commands: Commands, time: Res<Time>, sim: Res<SimWorld>, day: Res<crate::light::Daylight>, mut q: Query<(Entity, &Kinematics, &mut Health, Option<&mut Sunlit>, Option<&Chilled>)>) {
    let dt = time.delta_secs();
    let look = sim.world.tick() % LOOK_EVERY == 0;
    let up = sun_up(day.time);
    let d = crate::light::sun_dir(day.time);
    let toward = -Vec2::new(d[0], d[1]);
    for (e, k, mut health, sunlit, chilled) in &mut q {
        let space = crate::sky::space_at(&sim, k.body.pos.y);
        if space < SPACE_FROM {
            if sunlit.is_some() {
                commands.entity(e).remove::<Sunlit>();
            }
            continue;
        }
        let mut s = sunlit.as_deref().copied().unwrap_or_default();
        if look || sunlit.is_none() {
            s.lit = up && in_the_sun(&sim, k.body.pos, toward);
        }
        if s.lit {
            s.heat = (s.heat + dt / BAKE_SECS).min(1.0);
            health.linger(SCORCH * s.heat * s.heat * dt, crate::creatures::Harm::Fire, || crate::death::Cause::new("The sun", "baked"));
        } else {
            s.heat = (s.heat - dt / COOL_SECS).max(0.0);
            crate::creatures::body::elements::chill(&mut commands, e, &health.nature, chilled, SHADE_CHILL * space, SHADE_CHILL_SECS);
        }
        match sunlit {
            Some(mut now) => *now = s,
            None => {
                commands.entity(e).insert(s);
            }
        }
    }
}
