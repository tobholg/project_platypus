//! all player‑related systems (input, physics, animation, exhaust)

use bevy::input::ButtonInput;
use bevy::prelude::*;
use rand::Rng;

use crate::components::*;
use crate::constants::*;
use crate::terrain::{solid, world_to_tile_y, Terrain};

/* ===========================================================
   input (WASD / Space)
   =========================================================== */
pub fn player_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<(&mut Velocity, &mut Transform, &Player)>,
) {
    if let Ok((mut vel, mut tf, ply)) = q.get_single_mut() {
        match (keys.pressed(KeyCode::KeyA), keys.pressed(KeyCode::KeyD)) {
            (true, false) => {
                vel.0.x = -WALK_SPEED;
                tf.scale.x = -tf.scale.x.abs(); // face left
            }
            (false, true) => {
                vel.0.x = WALK_SPEED;
                tf.scale.x = tf.scale.x.abs(); // face right
            }
            _ => vel.0.x = 0.0,
        }
        if keys.just_pressed(KeyCode::Space) && ply.grounded {
            vel.0.y = JUMP_SPEED;
        }
    }
}

/* ===========================================================
   physics, collision & jet‑pack
   =========================================================== */
pub fn physics_and_collision_system(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<(&mut Transform, &mut Velocity, &mut Player)>,
    terrain: Res<Terrain>,
) {
    let dt = time.delta_secs();
    let Ok((mut tf, mut vel, mut ply)) = q.get_single_mut() else { return };

    /* ---- apply gravity & jet‑pack ---- */
    vel.0.y += GRAVITY * dt;
    if keys.pressed(KeyCode::Space) && !ply.grounded {
        vel.0.y += JET_ACCEL * dt;
    }

    /* ---- stepped collision resolution ---- */
    let step_dt = dt / COLLISION_STEPS as f32;
    let half = Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT) / 2.0;
    ply.grounded = false;

    for _ in 0..COLLISION_STEPS {
        /* ---------- horizontal pass ---------- */
        if vel.0.x != 0.0 {
            let new_x = tf.translation.x + vel.0.x * step_dt;
            let dir = vel.0.x.signum();
            let probe_x = new_x + dir * half.x;
            let tx = (probe_x / TILE_SIZE).floor() as i32;

            let y_top = world_to_tile_y(terrain.height, tf.translation.y + half.y - 0.1);
            let y_bot = world_to_tile_y(terrain.height, tf.translation.y - half.y + 0.1);
            let (y_min, y_max) = if y_top <= y_bot { (y_top, y_bot) } else { (y_bot, y_top) };

            if (y_min..=y_max).any(|ty| solid(&terrain, tx, ty)) {
                /* --------------------------------------------------
                   attempt a one‑tile auto‑step before giving up
                   -------------------------------------------------- */
                if ply.grounded && vel.0.y <= 0.0 {
                    // pretend we climbed MAX_STEP_HEIGHT
                    let lifted_y = tf.translation.y + MAX_STEP_HEIGHT;

                    let top = lifted_y + half.y - 0.1;
                    let bot = lifted_y - half.y + 0.1;
                    let ty_top = world_to_tile_y(terrain.height, top);
                    let ty_bot = world_to_tile_y(terrain.height, bot);
                    let (smin, smax) =
                        if ty_top <= ty_bot { (ty_top, ty_bot) } else { (ty_bot, ty_top) };

                    // space above the obstacle clear?
                    if !(smin..=smax).any(|ty| solid(&terrain, tx, ty)) {
                        // ✓ climb: adjust position, keep horizontal motion
                        tf.translation.y += MAX_STEP_HEIGHT;
                        tf.translation.x = new_x;
                        ply.grounded = true; // remain grounded
                    } else {
                        vel.0.x = 0.0; // blocked
                    }
                } else {
                    vel.0.x = 0.0; // airborne or already climbing
                }
            } else {
                tf.translation.x = new_x;
            }
        }

        /* ---------- vertical pass ---------- */
        if vel.0.y != 0.0 {
            let new_y = tf.translation.y + vel.0.y * step_dt;
            let dir = vel.0.y.signum();
            let probe_y = new_y + dir * half.y;
            let ty = world_to_tile_y(terrain.height, probe_y);

            let x_left = ((tf.translation.x - half.x + 0.1) / TILE_SIZE).floor() as i32;
            let x_right = ((tf.translation.x + half.x - 0.1) / TILE_SIZE).floor() as i32;

            if (x_left..=x_right).any(|tx| solid(&terrain, tx, ty)) {
                if vel.0.y < 0.0 {
                    ply.grounded = true;
                }
                vel.0.y = 0.0;
            } else {
                tf.translation.y = new_y;
            }
        }
    }

    /* ---- jet‑pack exhaust particles ---- */
    if keys.pressed(KeyCode::Space) && !ply.grounded {
        let mut rng = rand::thread_rng();
        for _ in 0..EXHAUST_RATE {
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: EXHAUST_COLOR,
                        custom_size: Some(Vec2::splat(EXHAUST_SIZE)),
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        tf.translation.x + rng.gen_range(-2.0..2.0),
                        tf.translation.y - half.y,
                        5.0,
                    ),
                    ..default()
                },
                Velocity(Vec2::new(
                    rng.gen_range(EXHAUST_SPEED_X.clone()),
                    rng.gen_range(EXHAUST_SPEED_Y.clone()),
                )),
                Exhaust { life: EXHAUST_LIFETIME },
            ));
        }
    }
}

/* ===========================================================
   decay & movement of exhaust particles
   =========================================================== */
pub fn exhaust_update_system(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, &mut Sprite, &Velocity, &mut Exhaust)>,
) {
    let dt = time.delta_secs();
    for (e, mut tf, mut spr, vel, mut ex) in &mut q {
        tf.translation += (vel.0 * dt).extend(0.0);
        ex.life -= dt;
        spr.color.set_alpha(ex.life / EXHAUST_LIFETIME);
        if ex.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

/* ===========================================================
   simple walk‑cycle animation
   =========================================================== */
pub fn animate_player_system(
    time: Res<Time>,
    mut q: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite), With<Player>>,
) {
    for (indices, mut timer, mut sprite) in &mut q {
        if timer.tick(time.delta()).just_finished() {
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = if atlas.index == indices.last {
                    indices.first
                } else {
                    atlas.index + 1
                };
            }
        }
    }
}