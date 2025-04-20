//! all player‑related systems: input, physics, animation, exhaust,
//! inventory selection, mining & shooting
//!
//! Works with **Bevy 0.15**, Rust 1.77.

use bevy::color::Alpha;               // ← brings set_alpha / with_alpha into scope
use bevy::input::ButtonInput;
use bevy::prelude::*;
use rand::Rng;

use crate::components::{
    AnimationIndices, AnimationTimer, Bullet, Debris, Enemy, 
    Exhaust, HeldItem, Inventory, Player, Velocity, Highlight,
    Health, Dashing,
};
use crate::constants::*;
use crate::terrain::{solid, tile_to_world_y, world_to_tile_y, Terrain, TileKind};

/// seconds between bullets when the gun is held down (≈12.5 rps)
const GUN_FIRE_INTERVAL: f32 = 0.12;

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
        if keys.just_pressed(KeyCode::Digit3) {
            inv.selected = HeldItem::StoneBlock;
        }
    }
}

/* ===========================================================
   horizontal movement & jump
   =========================================================== */
   pub fn player_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<(&mut Velocity, &mut Transform, &Player, Option<&Dashing>)>,
) {
    if let Ok((mut vel, mut tf, ply, dash)) = q.get_single_mut() {
        /* ignore A/D while dashing */
        if dash.is_none() {
            match (keys.pressed(KeyCode::KeyA), keys.pressed(KeyCode::KeyD)) {
                (true,  false) => {
                    vel.0.x = -WALK_SPEED;
                    tf.scale.x = -tf.scale.x.abs();
                }
                (false, true) => {
                    vel.0.x = WALK_SPEED;
                    tf.scale.x =  tf.scale.x.abs();
                }
                _ => vel.0.x = 0.0,
            }
        }

        /* jump still works while dashing */
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
    mut q: Query<(&mut Transform, &mut Velocity, &mut Player, &mut Health)>,
    terrain: Res<Terrain>,
) {
    let dt = time.delta_secs();
    let Ok((mut tf, mut vel, mut ply, mut health)) = q.get_single_mut() else { return };

    vel.0.y += GRAVITY * dt;
    if keys.pressed(KeyCode::Space) && !ply.grounded {
        vel.0.y += JET_ACCEL * dt;
    }

    let step_dt = dt / COLLISION_STEPS as f32;
    let half = Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT) / 2.0;
    ply.grounded = false;
    let mut landing_speed: Option<f32> = None;

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
                    landing_speed = Some(-vel.0.y);
                }
                vel.0.y = 0.0;
            } else {
                tf.translation.y = new_y;
            }
        }
    }

    /* after the collision loop, before the jet‑pack code */
    if let Some(v) = landing_speed {
        if v > SAFE_FALL_SPEED {
            let dmg = (v - SAFE_FALL_SPEED) * FALL_DMG_FACTOR;
            health.current = (health.current - dmg).max(0.0);
            health.last_damage = 0.0;

            // optional VFX / death check:
            // if health.current == 0.0 { commands.entity(entity).despawn(); }
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
   dash start (Shift)                                          */
   pub fn dash_start_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<(Entity, &mut Velocity, &Transform), (With<Player>, Without<Dashing>)>,
) {
    if !(keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight)) {
        return;
    }

    if let Ok((entity, mut vel, tf)) = q.get_single_mut() {
        let dir = if tf.scale.x >= 0.0 { 1.0 } else { -1.0 };
        vel.0.x = DASH_SPEED * dir;
        vel.0.y += DASH_UPWARD_BOOST;          // little upward kick
        /* white puff particles opposite to dash direction */
        {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            for _ in 0..DASH_PUFF_RATE {
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgba(0.9, 0.9, 0.9, 1.0),
                            custom_size: Some(Vec2::splat(DASH_PUFF_SIZE)),
                            ..default()
                        },
                        transform: Transform::from_xyz(
                            tf.translation.x - dir * PLAYER_WIDTH * 0.6
                                + rng.gen_range(-2.0..2.0),
                            tf.translation.y - PLAYER_HEIGHT * 0.2
                                + rng.gen_range(-2.0..2.0),
                            5.0,
                        ),
                        ..default()
                    },
                    Velocity(Vec2::new(
                        -dir * rng.gen_range(80.0..140.0),
                        rng.gen_range(-20.0..40.0),
                    )),
                    Exhaust { life: DASH_PUFF_LIFETIME },
                ));
            }
        }
        commands.entity(entity).insert(Dashing {
            remaining: DASH_DURATION,
            dir,
        });
    }
}

/* ===========================================================
   dash update & decay                                         */
pub fn dash_update_system(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Velocity, &mut Dashing)>,
) {
    let dt = time.delta_secs();
    for (entity, mut vel, mut dash) in &mut q {
        if dash.remaining > 0.0 {
            // launch phase: maintain full dash speed
            dash.remaining -= dt;
            vel.0.x = DASH_SPEED * dash.dir;
        } else {
            // decay phase: ease back toward normal movement
            vel.0.x -= dash.dir * DASH_DECEL * dt;

            // stop when we've slowed to (or below) walk speed or reversed
            if vel.0.x.signum() != dash.dir || vel.0.x.abs() <= WALK_SPEED {
                commands.entity(entity).remove::<Dashing>();
            }
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
            if !matches!(tile.kind, TileKind::Dirt | TileKind::Stone | TileKind::Obsidian | TileKind::Grass | TileKind::Snow) {
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

/* ===========================================================
   cursor‑based red/green highlight
   =========================================================== */
   pub fn cursor_highlight_system(
    mut commands: Commands,
    windows: Query<&Window>,
    cam_q: Query<(&Camera, &GlobalTransform)>,
    inv_q: Query<&Inventory, With<Player>>,
    terrain: Res<Terrain>,
    old: Query<Entity, With<Highlight>>,   // clear previous frame
) {
    // despawn previous highlights
    for e in &old {
        commands.entity(e).despawn();
    }

    let Ok(inv) = inv_q.get_single()            else { return };
    let window  =        windows.single();
    let Some(cursor) = window.cursor_position() else { return };
    let (cam, cam_tf)    = cam_q.single();
    let Ok(world) = cam.viewport_to_world_2d(cam_tf, cursor) else { return };

    match inv.selected {
        /* ---------- pickaxe: opaque‑red squares in mining radius ---------- */
        HeldItem::Pickaxe => {
            let min_x = ((world.x - MINING_RADIUS) / TILE_SIZE).floor() as i32;
            let max_x = ((world.x + MINING_RADIUS) / TILE_SIZE).ceil()  as i32;
            let min_y_world = world.y - MINING_RADIUS;
            let max_y_world = world.y + MINING_RADIUS;
            let min_y = world_to_tile_y(terrain.height, max_y_world);
            let max_y = world_to_tile_y(terrain.height, min_y_world);

            for ty in min_y..=max_y {
                for tx in min_x..=max_x {
                    if tx < 0 || ty < 0 ||
                       tx >= terrain.width as i32 || ty >= terrain.height as i32 {
                        continue;
                    }
                    let dx = tx as f32 * TILE_SIZE - world.x;
                    let dy = tile_to_world_y(terrain.height, ty as usize) - world.y;
                    if dx*dx + dy*dy >= MINING_RADIUS*MINING_RADIUS { continue; }

                    let (ux, uy) = (tx as usize, ty as usize);
                    if matches!(terrain.tiles[uy][ux].kind,
                        TileKind::Grass | TileKind::Dirt | TileKind::Stone | TileKind::Obsidian | TileKind::Snow)
                    {
                        commands.spawn((
                            Sprite {
                                color: Color::rgba(1.0, 0.0, 0.0, 0.4),
                                custom_size: Some(Vec2::splat(TILE_SIZE)),
                                ..default()
                            },
                            Transform::from_xyz(
                                ux as f32 * TILE_SIZE,
                                tile_to_world_y(terrain.height, uy),
                                20.0,
                            ),
                            Highlight,
                        ));
                    }
                }
            }
        }

        /* ---------- building: single green square if placeable ----------- */
        HeldItem::StoneBlock => {
            let tx = (world.x / TILE_SIZE).floor() as i32;
            let ty = world_to_tile_y(terrain.height, world.y);
            if tx < 0 || ty < 0 ||
               tx >= terrain.width as i32 || ty >= terrain.height as i32 {
                return;
            }
            let (ux, uy) = (tx as usize, ty as usize);
            if !matches!(terrain.tiles[uy][ux].kind, TileKind::Air | TileKind::Sky) {
                return; // occupied
            }
            if ![(-1,0),(1,0),(0,-1),(0,1)].iter()
                .any(|(dx,dy)| solid(&terrain, tx+dx, ty+dy))
            {
                return; // no solid neighbour
            }
            commands.spawn((
                Sprite {
                    color: Color::rgba(0.0, 1.0, 0.0, 0.4),
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
                Transform::from_xyz(
                    ux as f32 * TILE_SIZE,
                    tile_to_world_y(terrain.height, uy),
                    20.0,
                ),
                Highlight,
            ));
        }
        _ => {}
    }
}

/* ===========================================================
   place Stone block (HeldItem::StoneBlock)
   =========================================================== */
   pub fn place_stone_system(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cam_q: Query<(&Camera, &GlobalTransform)>,
    inv_q: Query<&Inventory, With<Player>>,
    mut terrain: ResMut<Terrain>,
) {
    let Ok(inv) = inv_q.get_single()                         else { return };
    if inv.selected != HeldItem::StoneBlock
        || !mouse.just_pressed(MouseButton::Left) { return; }

    let window  =        windows.single();
    let Some(cursor) = window.cursor_position()              else { return };
    let (cam, cam_tf)    = cam_q.single();
    let Ok(world) = cam.viewport_to_world_2d(cam_tf, cursor)  else { return };

    let tx = (world.x / TILE_SIZE).floor() as i32;
    let ty = world_to_tile_y(terrain.height, world.y);
    if tx < 0 || ty < 0 ||
       tx >= terrain.width as i32 || ty >= terrain.height as i32 { return; }

    let (ux, uy) = (tx as usize, ty as usize);
    if !matches!(terrain.tiles[uy][ux].kind, TileKind::Air | TileKind::Sky) { return; }
    if ![(-1,0),(1,0),(0,-1),(0,1)].iter()
        .any(|(dx,dy)| solid(&terrain, tx+dx, ty+dy)) { return; }

    terrain.tiles[uy][ux].kind = TileKind::Stone;
    terrain.tiles[uy][ux].mine_time = 0.50;
    terrain.changed_tiles.push_back((ux, uy));
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
   gun shooting – continuous fire while LMB held
   =========================================================== */
pub fn gun_shoot_system(
    mouse: Res<ButtonInput<MouseButton>>,      // read LMB state
    time:  Res<Time>,                          // delta‑time
    mut cooldown: Local<f32>,                  // time until next shot
    windows: Query<&Window>,
    cam_q:  Query<(&Camera, &GlobalTransform)>,
    inv_q:  Query<&Inventory, With<Player>>,
    player_q: Query<&Transform, With<Player>>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    *cooldown -= dt;

    let Ok(inv) = inv_q.get_single() else { return };
    if inv.selected != HeldItem::Gun || !mouse.pressed(MouseButton::Left) {
        return; // not in gun mode or button not held
    }
    if *cooldown > 0.0 {
        return; // still cooling down
    }
    *cooldown = GUN_FIRE_INTERVAL; // reset timer

    /* ---------- spawn a bullet ---------- */
    let window  =        windows.single();
    let Some(cursor) = window.cursor_position()              else { return };
    let (cam, cam_tf)    = cam_q.single();
    let Ok(target) = cam.viewport_to_world_2d(cam_tf, cursor) else { return };

    let origin = player_q.single().translation.truncate();
    let dir = (target - origin).normalize_or_zero();
    if dir.length() == 0.0 {
        return;
    }

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(1.0, 0.75, 0.0),
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
        b_vel.0.y += GRAVITY * dt * 0.5;
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
                enemy.recoil = RECOIL_TIME;          // start the stun timer
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
            vel.0.x = dir_sign * HIT_KNOCKBACK;        // horizontal shove
            if vel.0.y < HIT_KNOCKBACK_UP {            // only boost upward, never drag down
                vel.0.y = HIT_KNOCKBACK_UP;            // vertical pop
            }
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

/* ===========================================================
   passive health regeneration
   =========================================================== */
pub fn health_regen_system(
    time: Res<Time>,
    mut q: Query<&mut Health, With<Player>>,
) {
    let dt = time.delta_secs();
    if let Ok(mut health) = q.get_single_mut() {
        if health.current < health.max {
            health.last_damage += dt;
            if health.last_damage >= 5.0 {
                health.current = (health.current + dt).min(health.max);
            }
        } else {
            health.last_damage = 0.0; // reset when full
        }
    }
}