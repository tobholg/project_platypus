//! F3: performance HUD. F4: chunk borders and dirty rects (what the sim is
//! actually updating). The overlay is the first tool to reach for when a
//! region "won't go to sleep".

use std::time::Duration;

use bevy::math::Isometry2d;
use bevy::prelude::*;
use platypus_sim::CHUNK;

use crate::camera::{CursorWorld, Zoom};
use crate::tools::{Toolbelt, material_at};
use crate::world::{SimMetrics, SimWorld};

pub struct DebugPlugin;

#[derive(Resource)]
struct DebugView {
    hud: bool,
    rects: bool,
    refresh: Timer,
}

/// Frame-time statistics over the last refresh window.
#[derive(Resource, Default)]
pub struct FrameStats {
    sum: Duration,
    frames: u32,
    worst: Duration,
    pub avg_ms: f32,
    pub worst_ms: f32,
}

impl FrameStats {
    fn roll(&mut self) {
        if self.frames > 0 {
            self.avg_ms = self.sum.as_secs_f32() * 1000.0 / self.frames as f32;
            self.worst_ms = self.worst.as_secs_f32() * 1000.0;
        }
        *self = FrameStats { avg_ms: self.avg_ms, worst_ms: self.worst_ms, ..default() };
    }
}

#[derive(Component)]
struct HudText;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugView { hud: true, rects: false, refresh: Timer::from_seconds(0.25, TimerMode::Repeating) })
            .init_resource::<FrameStats>()
            .add_systems(Startup, spawn_hud)
            .add_systems(Update, (toggle, record_frame, update_hud, draw_rects));
    }
}

fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont { font_size: FontSize::Px(13.0), ..default() },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        Node { position_type: PositionType::Absolute, top: px(6), left: px(6), padding: UiRect::all(px(6)), ..default() },
        HudText,
    ));
}

fn toggle(keys: Res<ButtonInput<KeyCode>>, mut view: ResMut<DebugView>, mut hud: Query<&mut Visibility, With<HudText>>) {
    if keys.just_pressed(KeyCode::F3) {
        view.hud = !view.hud;
        for mut v in &mut hud {
            *v = if view.hud { Visibility::Visible } else { Visibility::Hidden };
        }
    }
    if keys.just_pressed(KeyCode::F4) {
        view.rects = !view.rects;
    }
}

fn record_frame(time: Res<Time<Real>>, mut stats: ResMut<FrameStats>) {
    let d = time.delta();
    stats.sum += d;
    stats.frames += 1;
    stats.worst = stats.worst.max(d);
}

#[allow(clippy::too_many_arguments)]
fn update_hud(
    time: Res<Time<Real>>,
    mut view: ResMut<DebugView>,
    mut stats: ResMut<FrameStats>,
    metrics: Res<SimMetrics>,
    sim: Res<SimWorld>,
    belt: Res<Toolbelt>,
    cursor: Res<CursorWorld>,
    zoom: Res<Zoom>,
    entities: Query<Entity>,
    mut text: Single<&mut Text, With<HudText>>,
) {
    if !view.refresh.tick(time.delta()).just_finished() {
        return;
    }
    stats.roll();
    if !view.hud {
        return;
    }
    let under = cursor.0.and_then(|p| material_at(&sim, p).map(|(_, n)| format!("({:.0},{:.0}) {n}", p.x.floor(), p.y.floor())));
    text.0 = format!(
        "frame {:.2} ms avg, {:.2} worst ({:.0} fps)\n\
         sim tick {:.3} ms (avg {:.3}) | active chunks {} | visited {} cells\n\
         loaded {} chunks | stored {} ({} KB) | streamed {} ({:.2} ms)\n\
         entities {} | particles {} | bodies {} | tick {} | zoom {} px/cell\n\
         tool {} r{} | cursor {}\n\
         [F3] hud [F4] dirty rects | wheel zoom | O spawn orc",
        stats.avg_ms,
        stats.worst_ms,
        1000.0 / stats.avg_ms.max(0.001),
        metrics.tick_time.as_secs_f64() * 1e3,
        metrics.tick_time_avg.as_secs_f64() * 1e3,
        metrics.last.active_chunks,
        metrics.last.visited_cells,
        sim.world.loaded_count(),
        sim.store.len(),
        sim.store.bytes() / 1024,
        metrics.loaded_this_frame,
        metrics.stream_time.as_secs_f64() * 1e3,
        entities.iter().count(),
        sim.world.particles().len(),
        sim.world.bodies().len(),
        sim.world.tick(),
        zoom.0,
        belt.tool.label(),
        belt.radius(),
        under.unwrap_or_default(),
    );
}

fn draw_rects(view: Res<DebugView>, sim: Res<SimWorld>, mut gizmos: Gizmos) {
    if !view.rects {
        return;
    }
    let c = CHUNK as f32;
    for chunk in sim.world.chunks() {
        let o = chunk.pos.origin();
        let origin = Vec2::new(o.x as f32, o.y as f32);
        gizmos.rect_2d(Isometry2d::from_translation(origin + Vec2::splat(c / 2.0)), Vec2::splat(c), Color::srgba(1.0, 1.0, 1.0, 0.12));
        let r = chunk.dirty_rect().clamp_to_chunk();
        if !r.is_empty() {
            let min = origin + Vec2::new(r.min_x as f32, r.min_y as f32);
            let size = Vec2::new((r.max_x - r.min_x + 1) as f32, (r.max_y - r.min_y + 1) as f32);
            gizmos.rect_2d(Isometry2d::from_translation(min + size / 2.0), size, Color::srgb(1.0, 0.25, 0.2));
        }
    }
}
