//! all player‑related systems: input, physics, animation, exhaust,
//! inventory selection, mining & shooting
//!
//! Works with **Bevy 0.15**, Rust 1.77.

use bevy::color::Alpha;               // ← brings set_alpha / with_alpha into scope
use bevy::input::ButtonInput;
use bevy::prelude::*;
use rand::Rng;

use crate::components::{
    AnimationIndices, AnimationTimer, Bullet, Debris, Enemy, Exhaust, HeldItem, Inventory, Player, Velocity
};
use crate::constants::*;
use crate::terrain::{solid, tile_to_world_y, world_to_tile_y, Terrain, TileKind};

/* -----------------------------------------------------------
   utility: approximate colour for debris particles
   ----------------------------------------------------------- */
#[inline]
fn tile_color(kind: TileKind) -> Color {
    match kind {
        TileKind::Dirt  => Color::srgb(0.55, 0.27, 0.07),
        TileKind::Stone => Color::srgb(0.50, 0.50, 0.50),
        _               => Color::WHITE,
    }
}

/* ===========================================================
   inventory hot‑keys (1 = pickaxe, 2 = gun)
   =========================================================== */
pub fn inventory_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<&mut Inventory, With<Player>>,
) {
    if let Ok(mut inv) = q.get_single_mut() {
        if keys.just_pressed(KeyCode::Digit1) {
            inv.selected = HeldItem::Pickaxe;
        }
        if keys.just_pressed(KeyCode::Digit2) {
            inv.selected = HeldItem::Gun;
        }
    }
}

/* ===========================================================
   horizontal movement & jump
   =========================================================== */
pub fn player_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<(&mut Velocity, &mut Transform, &Player)>,
) {
    if let Ok((mut vel, mut tf, ply)) = q.get_single_mut() {
        match (keys.pressed(KeyCode::KeyA), keys.pressed(KeyCode::KeyD)) {
            (true, false) => {
                vel.0.x = -WALK_SPEED;
                tf.scale.x = -tf.scale.x.abs();
            }
            (false, true) => {
                vel.0.x = WALK_SPEED;
                tf.scale.x = tf.scale.x.abs();
            }
            _ => vel.0.x = 0.0,
        }
        if keys.just_pressed(KeyCode::Space) && ply.grounded {
            vel.0.y = JUMP_SPEED;
        }
    }
}

/* ===========================================================
   physics, stepped collision & jet‑pack exhaust
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

    vel.0.y += GRAVITY * dt;
    if keys.pressed(KeyCode::Space) && !ply.grounded {
        vel.0.y += JET_ACCEL * dt;
    }

    let step_dt = dt / COLLISION_STEPS as f32;
    let half = Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT) / 2.0;
    ply.grounded = false;

    for _ in 0..COLLISION_STEPS {
        /* horizontal sweep */
        if vel.0.x != 0.0 {
            let new_x = tf.translation.x + vel.0.x * step_dt;
            let dir = vel.0.x.signum();
            let probe_x = new_x + dir * half.x;
            let tx = (probe_x / TILE_SIZE).floor() as i32;

            let y_top = world_to_tile_y(terrain.height, tf.translation.y + half.y - 0.1);
            let y_bot = world_to_tile_y(terrain.height, tf.translation.y - half.y + 0.1);
            let (y_min, y_max) = if y_top <= y_bot { (y_top, y_bot) } else { (y_bot, y_top) };

            if (y_min..=y_max).any(|ty| solid(&terrain, tx, ty)) {
                /* one‑tile auto‑step */
                if ply.grounded && vel.0.y <= 0.0 {
                    let lifted = tf.translation.y + MAX_STEP_HEIGHT;
                    let ty_top = world_to_tile_y(terrain.height, lifted + half.y - 0.1);
                    let ty_bot = world_to_tile_y(terrain.height, lifted - half.y + 0.1);
                    let (smin, smax) =
                        if ty_top <= ty_bot { (ty_top, ty_bot) } else { (ty_bot, ty_top) };

                    if !(smin..=smax).any(|ty| solid(&terrain, tx, ty)) {
                        tf.translation.y += MAX_STEP_HEIGHT;
                        tf.translation.x = new_x;
                        ply.grounded = true;
                    } else {
                        vel.0.x = 0.0;
                    }
                } else {
                    vel.0.x = 0.0;
                }
            } else {
                tf.translation.x = new_x;
            }
        }

        /* vertical sweep */
        if vel.0.y != 0.0 {
            let new_y = tf.translation.y + vel.0.y * step_dt;
            let dir = vel.0.y.signum();
            let probe_y = new_y + dir * half.y;
            let ty = world_to_tile_y(terrain.height, probe_y);

            let x_left  = ((tf.translation.x - half.x + 0.1) / TILE_SIZE).floor() as i32;
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

    /* jet‑pack exhaust */
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
   pickaxe mining (hold LMB)
   =========================================================== */
pub fn pickaxe_mining_system(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cam_q: Query<(&Camera, &GlobalTransform)>,
    mut terrain: ResMut<Terrain>,
    mut commands: Commands,
    inv_q: Query<&Inventory, With<Player>>,
) {
    let Ok(inv) = inv_q.get_single() else { return };
    if inv.selected != HeldItem::Pickaxe || !mouse.pressed(MouseButton::Left) {
        return;
    }

    let window = windows.single();
    let Some(cursor) = window.cursor_position() else { return };
    let (cam, cam_tf) = cam_q.single();
    let Ok(world) = cam.viewport_to_world_2d(cam_tf, cursor) else { return };

    let min_x = ((world.x - MINING_RADIUS) / TILE_SIZE).floor() as i32;
    let max_x = ((world.x + MINING_RADIUS) / TILE_SIZE).ceil()  as i32;

    let min_y_world = world.y - MINING_RADIUS;
    let max_y_world = world.y + MINING_RADIUS;
    let min_y = world_to_tile_y(terrain.height, max_y_world);
    let max_y = world_to_tile_y(terrain.height, min_y_world);

    let dt = 1.0 / 60.0;

    for ty in min_y..=max_y {
        for tx in min_x..=max_x {
            if tx < 0 || ty < 0 ||
               tx >= terrain.width as i32 || ty >= terrain.height as i32 {
                continue;
            }
            let dx = tx as f32 * TILE_SIZE - world.x;
            let dy = tile_to_world_y(terrain.height, ty as usize) - world.y;
            if dx * dx + dy * dy >= MINING_RADIUS * MINING_RADIUS {
                continue;
            }

            let (ux, uy) = (tx as usize, ty as usize);
            let tile = &mut terrain.tiles[uy][ux];
            if !matches!(tile.kind, TileKind::Dirt | TileKind::Stone) {
                continue;
            }

            tile.mine_time -= dt * PICKAXE_SPEED;
            if tile.mine_time <= 0.0 {
                tile.kind = TileKind::Air;
                terrain.changed_tiles.push_back((ux, uy));
                spawn_debris(&mut commands, &terrain, ux, uy);
            }
        }
    }
}

/* helper: debris particles */
fn spawn_debris(commands: &mut Commands, terrain: &Terrain, x: usize, y: usize) {
    let mut rng = rand::thread_rng();
    let color = tile_color(terrain.tiles[y][x].kind);
    let origin = Vec3::new(
        x as f32 * TILE_SIZE,
        tile_to_world_y(terrain.height, y),
        6.0,
    );

    for _ in 0..DEBRIS_RATE {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::splat(2.5)),
                    ..default()
                },
                transform: Transform::from_translation(origin),
                ..default()
            },
            Velocity(Vec2::new(
                rng.gen_range(DEBRIS_SPEED_X.clone()),
                rng.gen_range(DEBRIS_SPEED_Y.clone()),
            )),
            Debris { life: DEBRIS_LIFETIME },
        ));
    }
}

/* ===========================================================
   gun shooting (single bullet per click)
   =========================================================== */
pub fn gun_shoot_system(
    mouse: Res<ButtonInput<MouseButton>>,
    mut last_down: Local<bool>,
    windows: Query<&Window>,
    cam_q: Query<(&Camera, &GlobalTransform)>,
    inv_q: Query<&Inventory, With<Player>>,
    player_q: Query<&Transform, With<Player>>,
    mut commands: Commands,
) {
    let Ok(inv) = inv_q.get_single() else { return };
    if inv.selected != HeldItem::Gun {
        *last_down = mouse.pressed(MouseButton::Left);
        return;
    }

    let down = mouse.pressed(MouseButton::Left);
    if !down || *last_down {
        *last_down = down;
        return;
    }
    *last_down = down;

    let window = windows.single();
    let Some(cursor) = window.cursor_position() else { return };
    let (cam, cam_tf) = cam_q.single();
    let Ok(target) = cam.viewport_to_world_2d(cam_tf, cursor) else { return };

    let origin = player_q.single().translation.truncate();
    let dir = (target - origin).normalize_or_zero();
    if dir.length() == 0.0 {
        return;
    }

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(1.0, 0.0, 0.0),
                custom_size: Some(Vec2::splat(6.0)),
                ..default()
            },
            transform: Transform::from_translation(origin.extend(8.0)),
            ..default()
        },
        Velocity(dir * BULLET_SPEED),
        Bullet { damage: BULLET_DAMAGE, life: BULLET_LIFETIME },
    ));
}

/* ===========================================================
   bullet flight, damage, knock‑back & blood FX
   =========================================================== */
   pub fn bullet_update_system(
    time: Res<Time>,
    mut commands: Commands,

    /* bullets (have Bullet, never Enemy) */
    mut bullets: Query<
        (Entity, &mut Transform, &mut Velocity, &mut Bullet),
        Without<Enemy>,                 // ← proves disjointness
    >,

    /* ParamSet lets us borrow Enemy twice, but now each query
       also proves it never touches the bullet set */
    mut orcs: ParamSet<(
        /* read HP + position, despawn on death */
        Query<(Entity, &GlobalTransform, &mut Enemy), Without<Bullet>>,
        /* apply knock‑back impulse */
        Query<&mut Velocity, (With<Enemy>, Without<Bullet>)>,
    )>,

    terrain: Res<Terrain>,
) {
    let dt       = time.delta_secs();
    let half_orc = Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT) / 2.0;
    let mut knocks: Vec<(Entity, f32)> = Vec::new(); // (orc‑ID, ±1)

    /* ───────── 1. move bullets & process hits ───────── */
    for (b_ent, mut b_tf, mut b_vel, mut bullet) in &mut bullets {
        /* movement */
        b_vel.0.y += GRAVITY * dt * 0.2;
        b_tf.translation += (b_vel.0 * dt).extend(0.0);
        bullet.life -= dt;

        /* tile or timeout */
        if bullet.life <= 0.0
            || solid(
                &terrain,
                (b_tf.translation.x / TILE_SIZE).round() as i32,
                world_to_tile_y(terrain.height, b_tf.translation.y),
            )
        {
            commands.entity(b_ent).despawn();
            continue;
        }

        /* test vs. every orc */
        let b_pos = b_tf.translation.truncate();
        for (e_ent, e_gxf, mut enemy) in &mut orcs.p0() {
            let delta = (e_gxf.translation().truncate() - b_pos).abs();

            if delta.x <= half_orc.x && delta.y <= half_orc.y {
                /* hit */
                enemy.hp -= bullet.damage as i32;
                spawn_hit_blood(&mut commands, e_gxf.translation());
                knocks.push((e_ent, b_vel.0.x.signum()));
                commands.entity(b_ent).despawn();

                if enemy.hp <= 0 {
                    spawn_blood(&mut commands, e_gxf.translation() + Vec3::Z * 2.0);
                    commands.entity(e_ent).despawn();
                }
                break; // bullet gone
            }
        }
    }

    /* ───────── 2. knock‑back (separate Velocity borrow) ───────── */
    for (e_ent, dir_sign) in knocks {
        if let Ok(mut vel) = orcs.p1().get_mut(e_ent) {
            vel.0.x += dir_sign * HIT_KNOCKBACK;
        }
    }
}

/* ===========================================================
   debris fade‑out
   =========================================================== */
pub fn debris_update_system(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, &mut Sprite, &Velocity, &mut Debris)>,
) {
    let dt = time.delta_secs();
    for (e, mut tf, mut spr, vel, mut db) in &mut q {
        tf.translation += (vel.0 * dt).extend(0.0);
        db.life -= dt;

        spr.color.set_alpha(db.life / DEBRIS_LIFETIME);

        if db.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

/* ===========================================================
   exhaust particles decay
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

fn spawn_blood(commands: &mut Commands, pos: Vec3) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    for _ in 0..BLOOD_RATE {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: BLOOD_COLOR,
                    custom_size: Some(Vec2::splat(4.0)),
                    ..default()
                },
                transform: Transform::from_translation(pos),
                ..default()
            },
            Velocity(Vec2::new(
                rng.gen_range(BLOOD_SPEED_X.clone()),
                rng.gen_range(BLOOD_SPEED_Y.clone()),
            )),
            Debris { life: BLOOD_LIFETIME },        // we can reuse Debris
        ));
    }
}

fn spawn_hit_blood(commands: &mut Commands, pos: Vec3) {
    let mut rng = rand::thread_rng();
    for _ in 0..HIT_BLOOD_RATE {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: BLOOD_COLOR,
                    custom_size: Some(Vec2::splat(3.0)),
                    ..default()
                },
                transform: Transform::from_translation(pos),
                ..default()
            },
            Velocity(Vec2::new(
                rng.gen_range(-70.0..70.0),
                rng.gen_range(20.0..120.0),
            )),
            Debris { life: HIT_BLOOD_LIFE },
        ));
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