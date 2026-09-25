//! Things that move but don't think: bombs now; dropped items and
//! projectiles later. Same `Body` and `move_and_collide` as creatures.

use bevy::prelude::*;
use platypus_physics::{Body, Locomotion, move_and_collide};
use platypus_sim::{CellPos, WorldEdit};

use crate::actors::{Health, Kinematics, WorldGrid};
use crate::fx::Explosion;
use crate::tools::BombCfg;
use crate::world::{SimWorld, TICK_HZ, TickSet};

pub struct PropsPlugin;

const DT: f32 = (1.0 / TICK_HZ) as f32;
const GRAVITY: f32 = 900.0;

#[derive(Component)]
pub struct Bomb {
    fuse: f32,
    cfg: BombCfg,
}

impl Plugin for PropsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (fly_bombs, explode_bombs).chain().in_set(TickSet::Bodies))
            .add_systems(Update, blink);
    }
}

pub fn spawn_bomb(commands: &mut Commands, at: Vec2, vel: Vec2, cfg: BombCfg) {
    let mut body = Body::new(at, Vec2::splat(3.0));
    body.vel = vel;
    commands.spawn((
        Name::new("Bomb"),
        Bomb { fuse: cfg.fuse, cfg },
        Kinematics { body, loco: Locomotion::default(), prev_pos: at },
        Sprite::from_color(Color::srgb(0.12, 0.12, 0.14), Vec2::splat(4.0)),
        Transform::from_translation(at.extend(12.0)),
    ));
}

/// Ballistic flight with bounces.
fn fly_bombs(sim: Res<SimWorld>, mut q: Query<(&mut Kinematics, &Bomb)>) {
    let grid = WorldGrid(&sim.world);
    for (mut k, bomb) in &mut q {
        let k = &mut *k;
        k.prev_pos = k.body.pos;
        k.body.vel.y -= GRAVITY * DT;
        let before = k.body.vel;
        let c = move_and_collide(&grid, &mut k.body, DT);
        let e = bomb.cfg.bounce;
        if c.ground || c.ceiling {
            k.body.vel.y = if before.y.abs() > 40.0 { -before.y * e } else { 0.0 };
            k.body.vel.x *= if c.ground { 0.75 } else { 1.0 };
        }
        if c.wall_left || c.wall_right {
            k.body.vel.x = -before.x * e;
        }
        if c.submerged > 0.5 {
            k.body.vel *= 0.9;
        }
    }
}

fn explode_bombs(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    mut bombs: Query<(Entity, &mut Bomb, &Kinematics)>,
    mut creatures: Query<(&mut Kinematics, &mut Health), Without<Bomb>>,
    mut fx: MessageWriter<Explosion>,
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
        sim.world.apply_edit(&WorldEdit::Explode { center: CellPos::from_world(at.x, at.y), radius: cfg.radius, power: cfg.power });
        fx.write(Explosion { at, radius: cfg.radius as f32 });

        // Creatures: damage and knockback, falling off with distance.
        let reach = cfg.radius as f32 * 1.6;
        for (mut k, mut health) in &mut creatures {
            let d = k.body.pos - at;
            let dist = d.length();
            if dist > reach {
                continue;
            }
            let f = 1.0 - dist / reach;
            health.hp -= cfg.damage * f;
            let dir = (d.normalize_or(Vec2::Y) + Vec2::new(0.0, 0.6)).normalize();
            let k = &mut *k;
            k.loco.knock(&mut k.body, dir * cfg.knockback * (0.4 + 0.6 * f), 0.35);
        }
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
fn blink(mut q: Query<(&Bomb, &mut Sprite)>) {
    for (bomb, mut sprite) in &mut q {
        let rate = if bomb.fuse < 0.5 { 16.0 } else { 5.0 };
        let on = (bomb.fuse * rate).fract() < 0.5;
        sprite.color = if on { Color::srgb(1.0, 0.2, 0.1) } else { Color::srgb(0.12, 0.12, 0.14) };
    }
}
