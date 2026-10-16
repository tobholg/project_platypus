//! Things that move but don't think: bombs and glow sticks now; dropped items
//! and projectiles later. Same `Body` and `move_and_collide` as creatures.

use bevy::prelude::*;
use platypus_physics::{Body, Locomotion, move_and_collide};
use platypus_sim::{CellPos, WorldEdit};

use crate::creatures::{Kinematics, WorldGrid};
use crate::light::LightSource;
use crate::tools::BombCfg;
use crate::world::{SimWorld, TICK_HZ, TickSet};

pub struct PropsPlugin;

const DT: f32 = (1.0 / TICK_HZ) as f32;
pub(crate) const GRAVITY: f32 = 1350.0;

/// Flies ballistically and bounces (anything thrown).
#[derive(Component)]
pub struct Thrown {
    pub bounce: f32,
    /// Speed kept sliding along the ground, a tick (0.75: soon stopped; a
    /// body thrown by a blow slides on).
    pub slide: f32,
}

/// A thrown light: glows for `life` seconds, fading at the end.
#[derive(Component)]
pub struct Glowstick {
    age: f32,
    life: f32,
    color: [f32; 3],
}

#[derive(Component)]
pub struct Bomb {
    fuse: f32,
    cfg: BombCfg,
}

impl Plugin for PropsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (fly, explode_bombs, age_glowsticks).chain().in_set(TickSet::Bodies))
            .add_systems(Update, blink);
    }
}

pub fn spawn_bomb(commands: &mut Commands, at: Vec2, vel: Vec2, cfg: BombCfg) {
    let mut body = Body::new(at, Vec2::splat(4.5));
    body.vel = vel;
    commands.spawn((
        Name::new("Bomb"),
        Thrown { bounce: cfg.bounce, slide: 0.75 },
        Bomb { fuse: cfg.fuse, cfg },
        Kinematics { body, loco: Locomotion::default(), prev_pos: at },
        // (Drawn as the bomb in hand is, its item's picture: `hands::dress_bombs`.)
        Sprite::from_color(Color::srgb(0.12, 0.12, 0.14), Vec2::splat(9.0)),
        // Its fuse: a spark you can follow in the dark.
        LightSource { color: FUSE, flicker: 1.0 },
        Transform::from_translation(at.extend(12.0)),
    ));
}

/// A lit fuse's light.
const FUSE: [f32; 3] = [0.9, 0.42, 0.1];

/// Throw a glow stick (green or blue) that lights its surroundings.
pub fn spawn_glowstick(commands: &mut Commands, at: Vec2, vel: Vec2, color: [f32; 3], life: f32, haze: f32) {
    let mut body = Body::new(at, Vec2::new(3.0, 3.0));
    body.vel = vel;
    commands.spawn((
        Name::new("Glowstick"),
        Thrown { bounce: 0.3, slide: 0.75 },
        Glowstick { age: 0.0, life, color },
        LightSource { color, flicker: 0.0 },
        crate::light::Haze(haze),
        Kinematics { body, loco: Locomotion::default(), prev_pos: at },
        Sprite::from_color(Color::srgb(color[0].min(1.0), color[1].min(1.0), color[2].min(1.0)), Vec2::new(1.5, 4.5)),
        Transform::from_translation(at.extend(12.0)),
    ));
}

/// Glow sticks fade over their last quarter and are gone.
fn age_glowsticks(mut commands: Commands, mut q: Query<(Entity, &mut Glowstick, &mut LightSource, &mut Sprite)>) {
    for (e, mut g, mut light, mut sprite) in &mut q {
        g.age += DT;
        let left = 1.0 - g.age / g.life;
        if left <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let k = (left * 4.0).min(1.0);
        light.color = g.color.map(|c| c * k);
        sprite.color = Color::srgb((g.color[0] * k).min(1.0), (g.color[1] * k).min(1.0), (g.color[2] * k).min(1.0));
    }
}

/// Ballistic flight with bounces.
pub fn fly(sim: Res<SimWorld>, mut q: Query<(&mut Kinematics, &Thrown)>) {
    let grid = WorldGrid(&sim.world);
    for (mut k, thrown) in &mut q {
        // At rest on the ground: nothing to do until something pushes it or
        // what it lies on goes (a battlefield of bodies costs next to
        // nothing).
        if platypus_physics::resting(&grid, &k.body) {
            if k.prev_pos != k.body.pos {
                k.prev_pos = k.body.pos;
            }
            continue;
        }
        let k = &mut *k;
        k.prev_pos = k.body.pos;
        k.body.vel.y -= GRAVITY * DT;
        let before = k.body.vel;
        let c = move_and_collide(&grid, &mut k.body, DT);
        let e = thrown.bounce;
        if c.ground || c.ceiling {
            k.body.vel.y = if before.y.abs() > 60.0 { -before.y * e } else { 0.0 };
            k.body.vel.x *= if c.ground { thrown.slide } else { 1.0 };
        }
        if c.wall_left || c.wall_right {
            k.body.vel.x = -before.x * e;
        }
        if c.submerged > 0.5 {
            k.body.vel *= 0.9;
        }
        // Barely sliding on the ground: stopped.
        if c.ground && k.body.vel.y == 0.0 && k.body.vel.x.abs() < 3.0 {
            k.body.vel.x = 0.0;
        }
    }
}

fn explode_bombs(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    mut bombs: Query<(Entity, &mut Bomb, &Kinematics)>,
) {
    let mut blasts = Vec::new();
    for (entity, mut bomb, k) in &mut bombs {
        bomb.fuse -= DT;
        if bomb.fuse <= 0.0 {
            blasts.push((k.body.pos, bomb.cfg.clone()));
            commands.entity(entity).despawn();
        }
    }
    for (at, cfg) in blasts {
        // (The sim reports it with the rest, `StepStats::detonated`, and
        // creatures feel it from there: `creatures::blasted`.)
        sim.world.apply_edit(&WorldEdit::Explode { center: CellPos::from_world(at.x, at.y), radius: cfg.radius, power: cfg.power });
        let reach = cfg.radius as f32 * 1.6;
        // Off by default, so a string of bombs digs a shaft instead of
        // going off together.
        if cfg.chain_reaction {
            for (_, mut other, ok) in &mut bombs {
                if ok.body.pos.distance(at) < reach {
                    other.fuse = other.fuse.min(0.12);
                }
            }
        }
    }
}

/// Blink faster as the fuse runs down.
/// A bomb about to go blinks red (faster at the end), its fuse flaring.
fn blink(mut q: Query<(&Bomb, &mut Sprite, &mut LightSource)>) {
    for (bomb, mut sprite, mut light) in &mut q {
        let rate = if bomb.fuse < 0.5 { 16.0 } else { 5.0 };
        let on = (bomb.fuse * rate).fract() < 0.5;
        // (Its picture tinted; a plain square, coloured.)
        let pictured = sprite.image != Handle::default();
        sprite.color = match (on, pictured) {
            (true, true) => Color::srgb(1.0, 0.45, 0.35),
            (false, true) => Color::WHITE,
            (true, false) => Color::srgb(1.0, 0.2, 0.1),
            (false, false) => Color::srgb(0.12, 0.12, 0.14),
        };
        light.color = FUSE.map(|c| c * if on { 1.6 } else { 0.8 });
    }
}
