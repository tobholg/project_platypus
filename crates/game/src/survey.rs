//! Performance survey (any scenario): every frame's time logged from 3 s
//! on, with the fixed ticks it ran and what they cost; a `SURVEY` line at
//! exit with percentiles and how many frames missed 240/200/120/60 Hz.
//!
//! - `PLATYPUS_SURVEY_AT` = `island` | `space` | `ocean` | `seabed` |
//!   `flooded` | `lava` | `deep` | `cave`: the player put there at 0.5 s
//!   (held till 2.5 s while the chunks load), god mode on.
//! - `PLATYPUS_SURVEY_MOVE=run`: D held from 2.5 s, jumping now and then
//!   (streams the world in); `fly`: as run, the player's height held.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use platypus_sim::CellPos;

use crate::creatures::Kinematics;
use crate::creatures::player::LocalPlayer;
use crate::world::{SimMetrics, SimWorld};

pub struct SurveyPlugin;

/// (frame ms, ticks run in it, those ticks' ms, the cell sim's ms in them)
static LOG: Mutex<Vec<(f32, u8, f32, f32)>> = Mutex::new(Vec::new());

#[derive(Resource, Default)]
struct Frame {
    ticks: u8,
    fixed: Duration,
    cells: Duration,
    tick_start: Option<Instant>,
    elapsed: f32,
}

impl Plugin for SurveyPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var("PLATYPUS_SCENARIO").is_err() {
            return;
        }
        app.init_resource::<Frame>()
            .add_systems(FixedFirst, tick_start)
            .add_systems(FixedLast, tick_end)
            .add_systems(Last, (record, report).chain())
            .add_systems(Update, place)
            .add_systems(PreUpdate, drive.after(bevy::input::InputSystems));
    }
}

fn tick_start(mut f: ResMut<Frame>) {
    f.tick_start = Some(Instant::now());
}

fn tick_end(mut f: ResMut<Frame>, metrics: Res<SimMetrics>) {
    if let Some(t) = f.tick_start.take() {
        f.fixed += t.elapsed();
        f.ticks += 1;
        f.cells += metrics.tick_time;
    }
}

fn record(time: Res<Time<Real>>, mut f: ResMut<Frame>) {
    f.elapsed += time.delta_secs();
    let warm = std::env::var("PLATYPUS_SURVEY_WARM").ok().and_then(|s| s.parse().ok()).unwrap_or(6.0);
    if f.elapsed > warm {
        LOG.lock().unwrap().push((time.delta_secs() * 1e3, f.ticks, f.fixed.as_secs_f32() * 1e3, f.cells.as_secs_f32() * 1e3));
    }
    f.ticks = 0;
    f.fixed = Duration::ZERO;
    f.cells = Duration::ZERO;
}

fn pct(v: &mut [f32], p: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((v.len() - 1) as f32 * p).round() as usize]
}

fn report(mut exits: MessageReader<AppExit>) {
    if exits.read().next().is_none() {
        return;
    }
    let log = LOG.lock().unwrap();
    if log.is_empty() {
        return;
    }
    let n = log.len() as f32;
    let mut ft: Vec<f32> = log.iter().map(|r| r.0).collect();
    let avg = ft.iter().sum::<f32>() / n;
    let over = |ms: f32| log.iter().filter(|r| r.0 > ms).count();
    let mut with: Vec<f32> = log.iter().filter(|r| r.1 > 0).map(|r| r.0).collect();
    let mut without: Vec<f32> = log.iter().filter(|r| r.1 == 0).map(|r| r.0).collect();
    let mut tick: Vec<f32> = log.iter().filter(|r| r.1 > 0).map(|r| r.2 / r.1 as f32).collect();
    let mut cells: Vec<f32> = log.iter().filter(|r| r.1 > 0).map(|r| r.3 / r.1 as f32).collect();
    let multi = log.iter().filter(|r| r.1 > 1).count();
    let mean = |v: &[f32]| if v.is_empty() { 0.0 } else { v.iter().sum::<f32>() / v.len() as f32 };
    println!(
        "SURVEY frames={} fps={:.0} avg={:.2} p50={:.2} p90={:.2} p99={:.2} p999={:.2} max={:.2} | over4.2={:.1}% over5={:.1}% over8.3={:.2}% over16.7={} | tick-frames p50={:.2} p99={:.2} other p50={:.2} p99={:.2} | tick avg={:.2} p99={:.2} max={:.2} cells avg={:.2} p99={:.2} | 2+ticks={}",
        log.len(),
        1000.0 / avg,
        avg,
        pct(&mut ft, 0.5),
        pct(&mut ft, 0.9),
        pct(&mut ft, 0.99),
        pct(&mut ft, 0.999),
        pct(&mut ft, 1.0),
        over(4.17) as f32 * 100.0 / n,
        over(5.0) as f32 * 100.0 / n,
        over(8.33) as f32 * 100.0 / n,
        over(16.7),
        pct(&mut with, 0.5),
        pct(&mut with, 0.99),
        pct(&mut without, 0.5),
        pct(&mut without, 0.99),
        mean(&tick),
        pct(&mut tick, 0.99),
        pct(&mut tick, 1.0),
        mean(&cells),
        pct(&mut cells, 0.99),
        multi,
    );
}

/// Where `PLATYPUS_SURVEY_AT` asks, near the start.
fn target(sim: &SimWorld) -> Option<Vec2> {
    let at = std::env::var("PLATYPUS_SURVEY_AT").ok()?;
    let g = &sim.generator;
    let s = g.spawn_point();
    let mats = sim.materials();
    let id = |n: &str| mats.id(n);
    let ground = |x: i32| g.surface_hint(x).unwrap_or(s.y);
    // The nearest column (either way) with `what` between `depth.0` and
    // `depth.1` under the ground, scanning down.
    let find = |what: &dyn Fn(i32, i32) -> bool, depth: (i32, i32)| -> Option<(i32, i32)> {
        (0..250).flat_map(|i| [s.x + i * 90, s.x - i * 90]).find_map(|x| (depth.0..depth.1).step_by(12).map(|d| ground(x) - d).find(|&y| what(x, y)).map(|y| (x, y)))
    };
    let is = |n: &str| {
        let m = id(n);
        move |x: i32, y: i32| m.is_some() && g.material_hint(x, y) == m
    };
    let p = match at.as_str() {
        "island" => {
            let isl = g.sky_islands();
            let (lo, hi) = isl.iter().min_by_key(|(lo, hi)| ((lo.x + hi.x) / 2 - s.x).abs())?;
            ((lo.x + hi.x) / 2, hi.y + 30)
        }
        "space" => {
            let a = g.asteroids();
            let (c, r, _) = a.iter().min_by_key(|(c, _, _)| (c.x - s.x).abs())?;
            (c.x, c.y + *r as i32 + 40)
        }
        "ocean" => {
            let x = 2500;
            (x, g.water_hint(x).unwrap_or(ground(x)) + 20)
        }
        "seabed" => {
            let x = 2500;
            let water = is("water");
            let y = (0..4000).step_by(4).map(|d| g.water_hint(x).unwrap_or(0) - d).take_while(|&y| water(x, y)).last()?;
            (x, y + 30)
        }
        "flooded" => {
            let water = is("water");
            let (x, y) = find(&|x, y| water(x, y) && water(x, y - 30), (600, 30000))?;
            (x, y - 30)
        }
        "lava" => {
            let lava = is("lava");
            let (x, y) = find(&lava, (2000, 20000))?;
            (x, y + 120)
        }
        "deep" | "cave" => {
            let air = is("air");
            let d = if at == "deep" { (4000, 20000) } else { (500, 3000) };
            let (x, y) = find(&|x, y| air(x, y) && air(x, y + 30) && !air(x, y - 6), d)?;
            (x, y + 12)
        }
        _ => return None,
    };
    info!("survey: {at} at {:?}", CellPos::new(p.0, p.1));
    Some(Vec2::new(p.0 as f32, p.1 as f32))
}

fn place(time: Res<Time<Real>>, sim: Res<SimWorld>, mut cheats: ResMut<crate::devmenu::Cheats>, mut player: Query<&mut Kinematics, With<LocalPlayer>>, mut st: Local<(f32, Option<Option<Vec2>>, f32)>) {
    st.0 += time.delta_secs();
    if std::env::var("PLATYPUS_SURVEY_AT").is_err() || st.0 < 0.5 {
        return;
    }
    cheats.god = true;
    let at = *st.1.get_or_insert_with(|| target(&sim));
    let Ok(mut k) = player.single_mut() else { return };
    if let Some(at) = at
        && st.0 < 2.5
    {
        k.body.pos = at;
        k.prev_pos = at;
        k.body.vel = Vec2::ZERO;
        st.2 = at.y;
    }
    if std::env::var("PLATYPUS_SURVEY_MOVE").as_deref() == Ok("fly") && st.0 >= 2.5 {
        k.body.pos.y = st.2;
        k.body.vel.y = 0.0;
    }
}

fn drive(time: Res<Time<Real>>, mut keys: ResMut<ButtonInput<KeyCode>>, mut t: Local<f32>) {
    *t += time.delta_secs();
    let Ok(m) = std::env::var("PLATYPUS_SURVEY_MOVE") else { return };
    if *t < 2.5 || !(m == "run" || m == "fly") {
        return;
    }
    keys.press(KeyCode::KeyD);
    if (*t % 1.4) < 0.35 { keys.press(KeyCode::Space) } else { keys.release(KeyCode::Space) }
}
