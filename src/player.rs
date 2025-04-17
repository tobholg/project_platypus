//! all player‑related systems (input, physics, animation, exhaust)
use bevy::prelude::*;
use rand::Rng;

use crate::constants::*;
use crate::components::*;
use crate::terrain::{solid, Terrain, world_to_tile_y};

/// ------- input (WASD/Space) -------
pub fn player_input_system(
    keys: Res<Input<KeyCode>>,
    mut q:  Query<(&mut Velocity, &mut TextureAtlasSprite, &Player)>,
) {
    if let Ok((mut vel,mut sprite,ply)) = q.get_single_mut() {
        match (keys.pressed(KeyCode::A),keys.pressed(KeyCode::D)) {
            (true,false)  => { vel.0.x = -WALK_SPEED; sprite.flip_x = true; }
            (false,true)  => { vel.0.x =  WALK_SPEED; sprite.flip_x = false; }
            _             => { vel.0.x = 0.0; }
        }
        if keys.just_pressed(KeyCode::Space) && ply.grounded {
            vel.0.y = JUMP_SPEED;
        }
    }
}

/// ------- physics, collision & jet‑pack -------
pub fn physics_and_collision_system(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    mut q:  Query<(&mut Transform,&mut Velocity,&mut Player)>,
    terrain: Res<Terrain>,
) {
    let dt = time.delta_seconds();
    let Ok((mut tf,mut vel,mut ply)) = q.get_single_mut() else { return; };

    vel.0.y += GRAVITY*dt;
    if keys.pressed(KeyCode::Space) && !ply.grounded { vel.0.y += JET_ACCEL*dt; }

    let step_dt = dt / COLLISION_STEPS as f32;
    let half = Vec2::new(PLAYER_WIDTH,PLAYER_HEIGHT)/2.0;
    ply.grounded = false;

    for _ in 0..COLLISION_STEPS {
        // horizontal
        if vel.0.x != 0.0 {
            let new_x = tf.translation.x + vel.0.x*step_dt;
            let dir = vel.0.x.signum();
            let probe_x = new_x + dir*half.x;
            let tx = (probe_x / TILE_SIZE).floor() as i32;

            let y_top = world_to_tile_y(terrain.height, tf.translation.y+half.y-0.1);
            let y_bot = world_to_tile_y(terrain.height, tf.translation.y-half.y+0.1);
            let (y_min,y_max) = if y_top<=y_bot {(y_top,y_bot)} else {(y_bot,y_top)};
            if (y_min..=y_max).any(|ty| solid(&terrain,tx,ty)) {
                vel.0.x = 0.0;
            } else {
                tf.translation.x = new_x;
            }
        }
        // vertical
        if vel.0.y != 0.0 {
            let new_y = tf.translation.y + vel.0.y*step_dt;
            let dir   = vel.0.y.signum();
            let probe_y = new_y + dir*half.y;
            let ty = world_to_tile_y(terrain.height, probe_y);

            let x_left  = ((tf.translation.x-half.x+0.1)/TILE_SIZE).floor() as i32;
            let x_right = ((tf.translation.x+half.x-0.1)/TILE_SIZE).floor() as i32;

            if (x_left..=x_right).any(|tx| solid(&terrain,tx,ty)) {
                if vel.0.y < 0.0 { ply.grounded = true; }
                vel.0.y = 0.0;
            } else {
                tf.translation.y = new_y;
            }
        }
    }

    // jet‑pack exhaust
    if keys.pressed(KeyCode::Space) && !ply.grounded {
        let mut rng = rand::thread_rng();
        for _ in 0..EXHAUST_RATE {
            commands.spawn((
                SpriteBundle{
                    sprite: Sprite{
                        color: EXHAUST_COLOR,
                        custom_size: Some(Vec2::splat(EXHAUST_SIZE)),
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        tf.translation.x + rng.gen_range(-2.0..2.0),
                        tf.translation.y - half.y,
                        5.0),
                    ..default()
                },
                Velocity(Vec2::new(
                    rng.gen_range(EXHAUST_SPEED_X.clone()),
                    rng.gen_range(EXHAUST_SPEED_Y.clone()),
                )),
                Exhaust{ life: EXHAUST_LIFETIME },
            ));
        }
    }
}

/// ------- decay & movement of exhaust particles -------
pub fn exhaust_update_system(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity,&mut Transform,&mut Sprite,&Velocity,&mut Exhaust)>,
) {
    let dt = time.delta_seconds();
    for (e,mut tf,mut spr,vel,mut ex) in &mut q {
        tf.translation += (vel.0*dt).extend(0.0);
        ex.life -= dt;
        spr.color.set_a(ex.life/EXHAUST_LIFETIME);
        if ex.life <= 0.0 { commands.entity(e).despawn(); }
    }
}

/// ------- walking animation -------
pub fn animate_player_system(
    time:  Res<Time>,
    mut q: Query<(&AnimationIndices,&mut AnimationTimer,&mut TextureAtlasSprite),With<Player>>,
) {
    for (indices,mut timer,mut sprite) in &mut q {
        if timer.tick(time.delta()).just_finished() {
            sprite.index = if sprite.index==indices.last { indices.first } else { sprite.index+1 };
        }
    }
}