//! The dead coming up out of the ground (a zombie: `emerges` in life.ron),
//! and burning in the sun (`sunburns` in its creature file).
//!
//! One that comes up out of the ground is put standing where it'll stand
//! (on the rim of where it comes up), hidden and still (`Dormant`), and
//! comes up beside it, behind (`BACK`), in its arms-up picture, only what
//! of it's above the earth showing:
//!   - a hand punches up through the ground (a little hole, earth flung up
//!     and about as real cells), then the other (`SECOND`); they claw at
//!     the air a moment;
//!   - it digs itself out: a pit its size (`PIT`: a scoop, wide at the top)
//!     opens from the top down, the earth flying up and away from where
//!     it'll stand, landing loose round the pit; its head, its shoulders
//!     come up;
//!   - it crawls out onto the rim, leaning in, heaving in jerks; then it's
//!     awake. The pit stays.
//! Only through soft ground (earth, grass, sand, snow: `soft`), never rock.
//! (Several together come up one after another: `Emerging::after`.)
//!
//! In the sun (by day, nothing solid over its head), one that sunburns
//! smoulders and catches: it burns as anything burns (rain puts it out),
//! and its body, if it lies there to get up, burns away (`rise::kindle`).

use bevy::prelude::*;
use platypus_sim::{Cell, CellPos, Kind, MaterialId};

use crate::creatures::{Dormant, Kinematics};
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;
/// When each part of coming up starts, and when it's out (seconds): the
/// first hand, the second, digging out, crawling out.
const SECOND: f32 = 0.35;
const DIG: f32 = 0.95;
const CRAWL: f32 = 1.9;
const OUT: f32 = 3.1;
/// The pit it comes up out of: half as wide as it's deep at the top
/// (cells), as deep; its middle this far behind where it'll stand.
const PIT: (f32, f32) = (7.0, 15.0);
const BACK: f32 = 9.0;
/// The hardest ground it comes up through (earth's 20; rock's 60).
const SOFT: u8 = 25;
/// A hand's hole (cells across, half), and the earth it throws.
const PUNCH: f32 = 1.6;
const PUNCH_SPRAY: usize = 10;
/// The earth flung out of the pit: how fast (cells a tick), how much of
/// it flies as cells (the rest just goes).
const FLING: f32 = 1.9;
const FLUNG: f32 = 0.6;
/// How far it leans in, crawling out (radians), and the jerks in it.
const LEAN: f32 = 0.75;
const JERK: f32 = 0.06;
/// How far up a sunlit one looks for something over its head (cells).
const SKY: i32 = 160;
/// The sun's up between these (of the day: 0 midnight, 0.5 noon).
const SUNRISE: f32 = 0.27;
const SUNSET: f32 = 0.73;
/// Its picture's canvas (cells), and where its feet are on it.
const CANVAS: (u32, u32) = (96, 72);
const ON_CANVAS: (f32, f32) = (48.0, 64.0);
/// How much it burns in the sun (at least).
const SUN_FIRE: f32 = 0.6;

/// Coming up out of the ground: how far along (below 0: still waiting
/// its turn); its picture (arms up, its feet at 0), made as it starts;
/// where the pit's middle is at the surface (the world); its hands'
/// places across; the pit's cells still to dig, in order.
#[derive(Component)]
pub struct Emerging {
    t: f32,
    pixels: Vec<(Vec2, [f32; 4])>,
    pit: Vec2,
    hands: [f32; 2],
    dig: Vec<(f32, CellPos)>,
    punched: u8,
    /// Its picture by the cell (from its feet), and the sprite it's shown
    /// in (an image of its own, drawn each frame).
    cells: std::collections::HashMap<IVec2, [u8; 4]>,
    canvas: Option<(Entity, Handle<Image>)>,
}

/// The sprite one coming up is shown in (whose).
#[derive(Component)]
pub struct Rising(Entity);

impl Emerging {
    /// Starting after `delay` s.
    pub fn after(delay: f32) -> Self {
        Emerging { t: -delay, pixels: Vec::new(), pit: Vec2::ZERO, hands: [0.0; 2], dig: Vec::new(), punched: 0, cells: Default::default(), canvas: None }
    }
}

/// Ground it can come up through.
pub fn soft(ph: &platypus_sim::material::MatPhys) -> bool {
    matches!(ph.kind, Kind::Static | Kind::Powder) && ph.hardness <= SOFT
}

fn solid(sim: &SimWorld, p: CellPos) -> bool {
    sim.world.get(p).is_some_and(|c| !c.is_air() && matches!(sim.world.materials().phys(c.material).kind, Kind::Static | Kind::Powder))
}

/// Where its picture's feet are and how it's turned, now: in the pit,
/// sunk (only its hands up, then its head and shoulders), then crawling
/// out onto the rim where it stands, leaning in.
fn pose(em: &Emerging, k: &Kinematics) -> (Vec2, f32) {
    let feet = k.body.pos - Vec2::Y * k.body.half.y;
    let tall = em.pixels.iter().fold(0.0f32, |m, (p, _)| m.max(p.y));
    let smooth = |x: f32| x * x * (3.0 - 2.0 * x);
    let t = em.t;
    // (How far its hands reach up out of the ground, clawing.)
    let claw = 3.0 + 1.5 * (t * 13.0).sin().abs();
    let (up, out) = if t < DIG {
        (claw, 0.0)
    } else if t < CRAWL {
        let p = smooth((t - DIG) / (CRAWL - DIG));
        (claw + (tall * 0.55 - claw) * p + JERK * tall * (p * 17.0).sin() * (1.0 - p), 0.0)
    } else {
        let p = smooth(((t - CRAWL) / (OUT - CRAWL)).min(1.0));
        (tall * 0.55 + (tall - tall * 0.55) * p + JERK * tall * (p * 13.0).sin() * (1.0 - p), p)
    };
    let side = (feet.x - em.pit.x).signum();
    let at = Vec2::new(em.pit.x + (feet.x - em.pit.x) * out, em.pit.y - tall + up + (feet.y - em.pit.y) * out);
    // (Leaning in as it comes over the rim, upright as it stands.)
    let lean = -side * LEAN * (out * std::f32::consts::PI).sin() * if t >= CRAWL { 1.0 } else { 0.0 };
    (at, lean)
}

/// What's above the earth of one coming up shows; under it, the earth
/// hides it: drawn into an image of its own each frame (turned as it
/// leans), shown as a sprite where it is, so it's lit as the world is.
#[allow(clippy::type_complexity)]
pub fn draw(
    mut commands: Commands,
    sim: Res<SimWorld>,
    mut images: ResMut<Assets<Image>>,
    mut q: Query<(Entity, &mut Emerging, &Kinematics, &super::animation::Animator)>,
    mut canvases: Query<(Entity, &Rising, &mut Transform)>,
) {
    // (One whose owner's up, or gone: gone.)
    for (c, r, _) in &canvases {
        if q.get(r.0).is_err() {
            commands.entity(c).despawn();
        }
    }
    for (e, mut em, k, anim) in &mut q {
        if em.t < 0.0 || em.pixels.is_empty() {
            continue;
        }
        if em.cells.is_empty() {
            em.cells = em.pixels.iter().map(|(p, c)| (p.floor().as_ivec2(), [(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, 255])).collect();
        }
        let (w, h) = CANVAS;
        let handle = match &em.canvas {
            Some((_, h)) => h.clone(),
            None => {
                let mut image = Image::new_fill(bevy::render::render_resource::Extent3d { width: w, height: h, depth_or_array_layers: 1 }, bevy::render::render_resource::TextureDimension::D2, &[0, 0, 0, 0], bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb, bevy::asset::RenderAssetUsages::default());
                image.sampler = bevy::image::ImageSampler::nearest();
                let handle = images.add(image);
                let c = commands.spawn((Name::new("Coming up"), Rising(e), Sprite::from_image(handle.clone()), Transform::from_translation(k.body.pos.extend(anim.def.z)), Visibility::default())).id();
                em.canvas = Some((c, handle.clone()));
                handle
            }
        };
        let (at, lean) = pose(&em, k);
        let base = at.round();
        let (s, c) = (-lean).sin_cos();
        let mut data = vec![0u8; (w * h * 4) as usize];
        for iy in 0..h {
            for ix in 0..w {
                let world = base + Vec2::new(ix as f32 + 0.5 - ON_CANVAS.0, ON_CANVAS.1 - iy as f32 - 0.5);
                let d = world - at;
                let local = Vec2::new(d.x * c - d.y * s, d.x * s + d.y * c);
                let Some(px) = em.cells.get(&local.floor().as_ivec2()) else { continue };
                if solid(&sim, CellPos::from_world(world.x, world.y)) {
                    continue;
                }
                let i = ((iy * w + ix) * 4) as usize;
                data[i..i + 4].copy_from_slice(px);
            }
        }
        if let Some(mut img) = images.get_mut(&handle) {
            img.data = Some(data);
        }
        if let Some((cv, _)) = em.canvas
            && let Ok((_, _, mut tf)) = canvases.get_mut(cv)
        {
            let centre = base + Vec2::new(w as f32 / 2.0 - ON_CANVAS.0, ON_CANVAS.1 - h as f32 / 2.0);
            tf.translation = centre.extend(anim.def.z);
        }
    }
}

/// Coming up: a hand punches up through the earth, then the other; it
/// digs its pit, the earth flying; it crawls out; then it's awake.
#[allow(clippy::type_complexity)]
pub fn emerge(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut trauma: ResMut<crate::fx::Trauma>,
    mut q: Query<(Entity, &mut Emerging, &mut Kinematics, &super::animation::Animator, Option<&super::parts::Severed>)>,
    players: Query<&Kinematics, (With<crate::creatures::player::LocalPlayer>, Without<Emerging>)>,
) {
    for (e, mut em, mut k, anim, severed) in &mut q {
        em.t += DT;
        if em.t < 0.0 {
            continue;
        }
        let feet = k.body.pos - Vec2::Y * k.body.half.y;
        if em.pixels.is_empty() {
            start(&mut em, &mut k, anim, severed, &sim, players.iter().next().map(|p| p.body.pos));
        }
        let near = players.iter().any(|p| p.body.pos.distance(feet) < 160.0);
        // A hand punches up through the earth, then the other.
        for (i, at) in [(0u8, 0.0), (1, SECOND)] {
            if em.punched == i && em.t >= at {
                em.punched += 1;
                let x = em.hands[i as usize];
                punch(&mut sim, Vec2::new(x, em.pit.y));
                sounds.write(crate::sound::PlaySound::at("break", Vec2::new(x, em.pit.y)).pitch(0.55 + 0.1 * i as f32));
                if near {
                    trauma.0 = (trauma.0 + 0.12).min(1.0);
                }
            }
        }
        // Digging out: the pit, from the top down, its earth flung up and
        // away from the rim it'll stand on.
        if em.t >= DIG {
            let p = ((em.t - DIG) / (CRAWL - DIG)).clamp(0.0, 1.0);
            let side = (feet.x - em.pit.x).signum();
            let pit = em.pit;
            while em.dig.last().is_some_and(|(o, _)| *o <= p) {
                let Some((_, c)) = em.dig.pop() else { break };
                let Some(cell) = sim.world.get(c).filter(|c| !c.is_air()) else { continue };
                let ph = *sim.world.materials().phys(cell.material);
                if !soft(&ph) {
                    continue;
                }
                sim.world.set(c, Cell::AIR);
                if platypus_sim::rng::hash(&[c.x as u64, c.y as u64, 0xD16]) % 100 < (FLUNG * 100.0) as u64 {
                    let loose = if ph.crumbles_into == MaterialId::AIR { cell.material } else { ph.crumbles_into };
                    let from = Vec2::new(c.x as f32 + 0.5, pit.y + 0.5);
                    let away = Vec2::new((c.x as f32 - pit.x) / PIT.0 * 0.5 - side * 0.45, 1.0).normalize();
                    sim.world.spill([from.x, from.y], [away.x, away.y], loose, 1, FLING, 0.35);
                }
            }
            if sim.world.tick().is_multiple_of(9) && p < 1.0 {
                sounds.write(crate::sound::PlaySound::at("mine_dirt", pit).pitch(0.7).volume(0.6));
            }
        }
        if (CRAWL..CRAWL + DT).contains(&em.t) {
            sounds.write(crate::sound::PlaySound::at("groan", k.body.pos));
        }
        if em.t >= OUT {
            commands.entity(e).remove::<(Emerging, Dormant)>().insert(Visibility::Inherited);
        }
    }
}

/// Its picture, which way it faces (to the player), where it comes up
/// (behind it), its hands' places, and the pit's cells in the order
/// they're dug (top down, a little ragged).
fn start(em: &mut Emerging, k: &mut Kinematics, anim: &super::animation::Animator, severed: Option<&super::parts::Severed>, sim: &SimWorld, player: Option<Vec2>) {
    if let Some(p) = player {
        k.loco.facing = if p.x >= k.body.pos.x { 1.0 } else { -1.0 };
    }
    let facing = k.loco.facing;
    let feet = k.body.pos - Vec2::Y * k.body.half.y;
    let lost = severed.map(|s| s.0.clone()).unwrap_or_default();
    // (Its picture with its feet at 0.)
    em.pixels = super::rise::picture(anim, k, "climb", &lost).into_iter().map(|(p, c)| (p + Vec2::Y * k.body.half.y, c)).collect();
    // The pit behind it, at the ground's top there.
    let x = feet.x - facing * BACK;
    let cx = x.floor() as i32;
    let fy = feet.y.round() as i32;
    let top = (fy - 8..=fy + 8).rev().find(|&y| solid(sim, CellPos::new(cx, y - 1)) && !solid(sim, CellPos::new(cx, y))).unwrap_or(fy);
    em.pit = Vec2::new(x, top as f32);
    // Its hands: the highest of it either side of its middle.
    let high = |left: bool| em.pixels.iter().filter(|(p, _)| (p.x < 0.0) == left).max_by(|a, b| a.0.y.total_cmp(&b.0.y)).map_or(0.0, |(p, _)| p.x);
    em.hands = [x + high(facing < 0.0), x + high(facing > 0.0)];
    // The pit's cells: a scoop, wide at the top.
    let mut dig = Vec::new();
    let (hw, depth) = PIT;
    for dy in 1..=depth as i32 {
        let y = top - dy;
        let d = dy as f32 / depth;
        let half = hw * (1.0 - d * d).max(0.0).sqrt();
        for dx in -(half.ceil() as i32)..=half.ceil() as i32 {
            if (dx as f32).abs() > half + 0.3 {
                continue;
            }
            let c = CellPos::new(cx + dx, y);
            let jitter = (platypus_sim::rng::hash(&[c.x as u64, c.y as u64, 0x917]) % 1000) as f32 / 1000.0 * 0.15;
            dig.push(((d * 0.85 + jitter).min(1.0), c));
        }
    }
    // (Last first: popped from the end.)
    dig.sort_by(|a, b| b.0.total_cmp(&a.0));
    em.dig = dig;
}

/// A hand punching up through the earth at `at` (its surface): a little
/// hole, and what was there flung up and about.
fn punch(sim: &mut SimWorld, at: Vec2) {
    let r = PUNCH.ceil() as i32;
    let (cx, cy) = (at.x.floor() as i32, at.y.round() as i32);
    let mut thrown = Vec::new();
    for dy in -r * 2..=0 {
        for dx in -r..=r {
            if (dx as f32).abs() > PUNCH + 0.2 {
                continue;
            }
            let c = CellPos::new(cx + dx, cy + dy - 1);
            let Some(cell) = sim.world.get(c).filter(|c| !c.is_air()) else { continue };
            let ph = *sim.world.materials().phys(cell.material);
            if soft(&ph) {
                sim.world.set(c, Cell::AIR);
                thrown.push(if ph.crumbles_into == MaterialId::AIR { cell.material } else { ph.crumbles_into });
            }
        }
    }
    for m in thrown.into_iter().take(PUNCH_SPRAY) {
        sim.world.spill([at.x, at.y + 0.5], [0.0, 1.0], m, 1, FLING * 1.1, 0.7);
    }
}

/// The sun's up now.
pub fn sun_up(day: &crate::light::Daylight) -> bool {
    (SUNRISE..SUNSET).contains(&day.time)
}

/// Open to the sun at `at` (by day, nothing solid over it).
pub fn sunlit(sim: &SimWorld, day: &crate::light::Daylight, at: Vec2) -> bool {
    sun_up(day) && sim.world.under_sky(CellPos::from_world(at.x, at.y), SKY)
}

/// In the sun, what sunburns burns (rain or water puts it out, as any
/// fire); a fresh fire on it at least `SUN_FIRE`.
#[allow(clippy::type_complexity)]
pub fn sunburn(
    mut commands: Commands,
    sim: Res<SimWorld>,
    day: Res<crate::light::Daylight>,
    mut clock: Local<f32>,
    mut q: Query<(Entity, &Kinematics, &super::animation::Animator, Option<&mut super::elements::Burning>), (Without<Dormant>, Without<Emerging>)>,
) {
    *clock += DT;
    if *clock < 0.25 || !sun_up(&day) {
        return;
    }
    *clock = 0.0;
    for (e, k, anim, burning) in &mut q {
        if !anim.def.sunburns || !sunlit(&sim, &day, k.body.pos + Vec2::Y * k.body.half.y) {
            continue;
        }
        match burning {
            Some(mut b) => b.share = b.share.max(SUN_FIRE),
            None => {
                commands.entity(e).insert(super::elements::Burning::new(SUN_FIRE));
            }
        }
    }
}
