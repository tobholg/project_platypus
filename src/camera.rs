use bevy::prelude::*;

use crate::components::Player;
use crate::constants::*;
use crate::terrain::Terrain;

/// simple camera follow with world clamping
pub fn camera_follow_system(
    mut cam_q:    Query<&mut Transform,(With<Camera>,Without<Player>)>,
    player_q:     Query<&Transform,With<Player>>,
    window_q:     Query<&Window>,
    terrain:      Res<Terrain>,
) {
    let Ok(mut cam_tf) = cam_q.get_single_mut() else { return; };
    let Ok(player_tf)  = player_q.get_single() else { return; };
    let window = window_q.single();

    let half_w = window.width()*0.5;
    let half_h = window.height()*0.5;
    let world_w = terrain.width  as f32 * TILE_SIZE;
    let world_h = terrain.height as f32 * TILE_SIZE;

    cam_tf.translation.x = player_tf.translation.x.clamp(half_w, world_w-half_w);
    cam_tf.translation.y = player_tf.translation.y.clamp(half_h, world_h-half_h);
}