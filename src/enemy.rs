//! orc‑spawn, AI, and physics (enemies “sleep” when outside ActiveRect)

use bevy::prelude::*;
use rand::Rng;

use crate::{
    components::*,
    constants::*,
    terrain::{
        solid, tile_to_world_y, world_to_tile_y, ActiveRect, Terrain,
    },
};

/* ===========================================================
   start‑up: drop orcs on the surface
   =========================================================== */
pub fn spawn_enemies(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    terrain: Res<Terrain>,
) {
    let sheet = asset_server.load("textures/orc_sheet.png");
    let layout =
        TextureAtlasLayout::from_grid(UVec2::new(100, 100), 6, 1, None, None);
    let layout_handle = atlas_layouts.add(layout);

    let mut rng = rand::thread_rng();
    for _ in 0..64 {
        let x_tile = rng.gen_range(0..terrain.width);
        let y_tile = terrain.height_map[x_tile];

        let pos = Vec2::new(
            x_tile as f32 * TILE_SIZE,
            tile_to_world_y(terrain.height, y_tile)
                + TILE_SIZE * 0.5
                + PLAYER_HEIGHT * 0.5,
        );

        commands.spawn((
            Sprite::from_atlas_image(
                sheet.clone(),
                TextureAtlas {
                    layout: layout_handle.clone(),
                    index: 0,
                },
            ),
            Transform {
                translation: pos.extend(10.0),
                scale: Vec3::splat(1.8),
                ..default()
            },
            Enemy { grounded: false, hp: 100 },
            Velocity(Vec2::ZERO),
            AnimationIndices { first: 0, last: 5 },
            AnimationTimer(Timer::from_seconds(
                0.12,
                TimerMode::Repeating,
            )),
        ));
    }
}

/* ===========================================================
   tag / un‑tag enemies based on ActiveRect
   =========================================================== */
pub fn update_active_tag_system(
    rect_res: Res<ActiveRect>,
    terrain: Res<Terrain>,
    mut q: Query<(Entity, &Transform, Option<&Active>), With<Enemy>>,
    mut commands: Commands,
) {
    let rect = *rect_res; // copy to avoid repeated deref

    for (e, tf, has_tag) in &mut q {
        let tx = (tf.translation.x / TILE_SIZE).floor() as i32;
        let ty = world_to_tile_y(terrain.height, tf.translation.y);

        let inside = tx >= rect.min_x
            && tx <= rect.max_x
            && ty >= rect.min_y
            && ty <= rect.max_y;

        match (inside, has_tag.is_some()) {
            (true, false) => {
                commands.entity(e).insert(Active);
            }
            (false, true) => {
                commands.entity(e).remove::<Active>();
            }
            _ => {}
        }
    }
}

/* ===========================================================
   AI (runs only for Active enemies)
   =========================================================== */
pub fn enemy_ai_system(
    mut enemies: Query<
        (&mut Velocity, &mut Transform, &Enemy),
        (With<Active>, Without<Player>),
    >,
    player_q: Query<&Transform, With<Player>>,
) {
    let Ok(player_tf) = player_q.get_single() else { return };
    let player_pos = player_tf.translation.truncate();
    let mut rng = rand::thread_rng();

    for (mut vel, mut tf, enemy) in &mut enemies {
        let pos = tf.translation.truncate();
        let to_player = player_pos - pos;
        let dist = to_player.length();

        /* ---- aggro zone ---- */
        if dist < AGGRO_RADIUS {
            let dx = to_player.x;

            if dx.abs() > ENEMY_KEEP_AWAY {
                vel.0.x = ENEMY_SPEED * dx.signum();
                tf.scale.x = dx.signum() * tf.scale.x.abs();
            } else {
                vel.0.x = 0.0;
            }

            if enemy.grounded
                && to_player.y > TILE_SIZE * 0.5
                && rng.gen_bool(0.15)
            {
                vel.0.y = JUMP_SPEED;
            }
            continue;
        }

        /* ---- idle wandering ---- */
        if rng.gen_bool(0.02) {
            vel.0.x = if rng.gen_bool(0.5) {
                -ENEMY_SPEED
            } else {
                ENEMY_SPEED
            };
            tf.scale.x = vel.0.x.signum() * tf.scale.x.abs();
        }
        if enemy.grounded && rng.gen_bool(0.005) {
            vel.0.y = JUMP_SPEED;
        }
    }
}

/* ===========================================================
   physics (gravity + tile collision) only for Active enemies
   =========================================================== */
pub fn enemy_physics_system(
    time: Res<Time>,
    mut q: Query<
        (&mut Transform, &mut Velocity, &mut Enemy),
        With<Active>,
    >,
    terrain: Res<Terrain>,
) {
    let dt = time.delta_secs();
    let half = Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT) / 2.0;

    for (mut tf, mut vel, mut enemy) in &mut q {
        vel.0.y += GRAVITY * dt;
        let step_dt = dt / COLLISION_STEPS as f32;
        enemy.grounded = false;

        for _ in 0..COLLISION_STEPS {
            /* --- horizontal --- */
            if vel.0.x != 0.0 {
                let new_x = tf.translation.x + vel.0.x * step_dt;
                let dir = vel.0.x.signum();
                let probe_x = new_x + dir * half.x;
                let tx = (probe_x / TILE_SIZE).floor() as i32;

                let y_top = world_to_tile_y(
                    terrain.height,
                    tf.translation.y + half.y - 0.1,
                );
                let y_bot = world_to_tile_y(
                    terrain.height,
                    tf.translation.y - half.y + 0.1,
                );
                let (y_min, y_max) =
                    if y_top <= y_bot { (y_top, y_bot) } else { (y_bot, y_top) };

                if (y_min..=y_max).any(|ty| solid(&terrain, tx, ty)) {
                    vel.0.x = 0.0;
                } else {
                    tf.translation.x = new_x;
                }
            }

            /* --- vertical --- */
            if vel.0.y != 0.0 {
                let new_y = tf.translation.y + vel.0.y * step_dt;
                let dir = vel.0.y.signum();
                let probe_y = new_y + dir * half.y;
                let ty = world_to_tile_y(terrain.height, probe_y);

                let x_left =
                    ((tf.translation.x - half.x + 0.1) / TILE_SIZE).floor() as i32;
                let x_right =
                    ((tf.translation.x + half.x - 0.1) / TILE_SIZE).floor() as i32;

                if (x_left..=x_right).any(|tx| solid(&terrain, tx, ty)) {
                    if vel.0.y < 0.0 {
                        enemy.grounded = true;
                    }
                    vel.0.y = 0.0;
                } else {
                    tf.translation.y = new_y;
                }
            }
        }
    }
}

/* ===========================================================
   reuse player animation code
   =========================================================== */
pub fn animate_enemy_system(
    time: Res<Time>,
    mut q: Query<
        (&AnimationIndices, &mut AnimationTimer, &mut Sprite),
        (With<Enemy>, With<Active>),
    >,
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