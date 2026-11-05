//! Healing, shown: whatever puts health back on the player (a potion
//! mending, a heart taken, a regen draught, rest) sends green plusses
//! rising out of it, one for every couple of points, a few at a time:
//! each pops in (a little past full, settling), drifts up swaying, slows,
//! twinkles and fades, some of them lighting what's round them; soft green
//! motes rise with them. A heart (a lot at once) bursts a ring of green
//! light round it too. With damage numbers on, a green "+N" floats up over
//! it, counting up while the healing goes on (a potion's whole draught on
//! one number).

use bevy::prelude::*;

use crate::creatures::player::LocalPlayer;
use crate::creatures::{Health, Kinematics};
use crate::magic::runes::Emitter;

pub struct HealingPlugin;

impl Plugin for HealingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, make_art).add_systems(Update, (watch, rise, count).chain());
    }
}

/// Points of health a plus; at most this many owed (a big heal's plusses
/// come over a moment, not all at once); seconds between two.
const PER_PLUS: f32 = 2.0;
const OWED_MOST: f32 = 36.0;
const GAP: f32 = 0.05;
/// Seconds after the player turns up before its health rising is healing.
const SETTLE: f32 = 1.0;
/// A number says at least this much (points), gathered over gaps of at most
/// this long (s).
const TRICKLE: (f32, f32) = (3.0, 1.5);
/// A plus: seconds it lasts, how fast it rises (cells/s), how far it
/// sways (cells), its size (× its picture), the share of them lit.
const LIFE: (f32, f32) = (0.9, 1.35);
const RISE: (f32, f32) = (24.0, 44.0);
const SWAY: f32 = 2.2;
const SIZE: (f32, f32) = (0.9, 1.5);
const LIT_EVERY: u32 = 2;
/// This much at once (points) and it bursts a ring.
const BURST: f32 = 8.0;
/// The "+N": hits of healing this close (s) add to it; it floats so long,
/// this fast, this big.
const MERGE: f32 = 0.45;
const NUMBER_LIFE: f32 = 1.1;
const NUMBER_RISE: f32 = 22.0;
const NUMBER_SIZE: f32 = 10.5;

/// A plus, as text art ('o' its dark edge, 'G' it, 'W' its bright heart).
const ART: [&str; 7] = ["..ooo..", "..oGo..", "oooGooo", "oGGWGGo", "oooGooo", "..oGo..", "..ooo.."];

#[derive(Resource)]
struct PlusArt(Handle<Image>);

/// A plus rising: where it is from the player (it rises with them), how
/// fast, how old, how long it lasts, its size, its sway.
#[derive(Component)]
struct Plus {
    off: Vec2,
    vel: Vec2,
    age: f32,
    life: f32,
    size: f32,
    phase: f32,
}

/// The "+N" over the player.
#[derive(Component)]
struct HealNumber {
    amount: f32,
    age: f32,
}

fn make_art(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let (w, h) = (7u32, 7u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for (y, row) in ART.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            let c = match ch {
                'o' => [18, 84, 40, 220],
                'G' => [96, 236, 120, 255],
                'W' => [226, 255, 222, 255],
                _ => continue,
            };
            let i = (y * w as usize + x) * 4;
            rgba[i..i + 4].copy_from_slice(&c);
        }
    }
    let image = images.add(Image::new(
        bevy::render::render_resource::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        rgba,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    ));
    commands.insert_resource(PlusArt(image));
}

/// Soft green motes rising with the plusses.
static MOTES: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 1.0,
    life: (0.5, 1.0),
    colors: vec![(200, 255, 200), (110, 240, 130), (60, 200, 100)],
    speed: 18.0,
    spread: 0.7,
    gravity: -40.0,
    drag: 1.5,
    size: 1.0,
    jitter: 0.6,
    glow: true,
});

/// A lot healed at once: a ring of green light bursting out round it.
static RING: std::sync::LazyLock<Emitter> = std::sync::LazyLock::new(|| Emitter {
    count: 28.0,
    life: (0.3, 0.55),
    colors: vec![(220, 255, 220), (120, 245, 140)],
    speed: 95.0,
    spread: std::f32::consts::PI,
    gravity: -20.0,
    drag: 5.0,
    size: 1.0,
    jitter: 0.2,
    glow: true,
});

/// What the player had last frame; plusses owed, and till the next; the
/// "+N" showing; plusses made (for which are lit, and where).
#[derive(Default)]
struct Watch {
    last: Option<f32>,
    /// Seconds watched (the first second's health is the game setting it).
    seen: f32,
    /// Healed, not yet said (a trickle), and how long since the last of it.
    pend: f32,
    quiet: f32,
    owed: f32,
    next: f32,
    number: Option<Entity>,
    made: u32,
}

fn unit(seed: &[u64]) -> f32 {
    (platypus_sim::rng::hash(seed) % 10_000) as f32 / 10_000.0
}

/// Health that rose: plusses owed (and a ring for a lot at once), and the
/// number; owed plusses let out, a few at a time.
#[allow(clippy::too_many_arguments)]
fn watch(
    mut commands: Commands,
    time: Res<Time>,
    art: Option<Res<PlusArt>>,
    display: Res<crate::display::Display>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    player: Query<(&Kinematics, &Health), With<LocalPlayer>>,
    mut numbers: Query<(&mut HealNumber, &mut Text2d)>,
    mut w: Local<Watch>,
    cheats: Res<crate::devmenu::Cheats>,
) {
    let Some(art) = art else { return };
    let Ok((k, h)) = player.single() else {
        w.last = None;
        w.seen = 0.0;
        return;
    };
    let dt = time.delta_secs();
    let was = w.last.replace(h.hp);
    w.seen += dt;
    // (Not coming back from death: that's no healing to show.)
    // (Nor god mode topping it up.)
    let gained = match was {
        Some(was) if was > 0.0 && h.hp > was && w.seen > SETTLE && !cheats.god => h.hp - was,
        _ => 0.0,
    };
    // (A trickle, rest's or a regen's, adds up a while before it says so.)
    w.quiet += dt;
    if w.quiet > TRICKLE.1 {
        w.pend = 0.0;
    }
    if gained > 0.0 {
        w.owed = (w.owed + gained).min(OWED_MOST);
        if gained >= BURST {
            sparks.emit(&RING, RING.count as usize, k.body.pos, Vec2::Y, Vec2::ZERO);
            w.next = 0.0;
        }
        if display.damage_numbers {
            w.pend += gained;
            w.quiet = 0.0;
            let merged = w.number.and_then(|e| numbers.get_mut(e).ok()).filter(|(n, _)| n.age < MERGE);
            match merged {
                Some((mut n, mut text)) => {
                    n.amount += gained;
                    n.age = n.age.min(MERGE * 0.5);
                    text.0 = format!("+{}", n.amount.round().max(1.0) as i32);
                    w.pend = 0.0;
                }
                None if w.pend < TRICKLE.0 => {}
                None => {
                    let gained = std::mem::take(&mut w.pend);
                    let e = commands
                        .spawn((
                            Name::new("Healed"),
                            HealNumber { amount: gained, age: 0.0 },
                            Text2d::new(format!("+{}", gained.round().max(1.0) as i32)),
                            TextFont { font_size: FontSize::Px(NUMBER_SIZE), ..default() },
                            TextColor(Color::srgb(0.45, 1.0, 0.55)),
                            Transform::from_translation((k.body.pos + Vec2::new(0.0, k.body.half.y + 8.0)).extend(20.0)),
                        ))
                        .id();
                    w.number = Some(e);
                }
            }
        }
    }
    w.next -= dt;
    while w.owed >= PER_PLUS && w.next <= 0.0 {
        w.owed -= PER_PLUS;
        w.next += GAP;
        w.made = w.made.wrapping_add(1);
        let n = w.made as u64;
        let u = |s: u64| unit(&[n, s, 0x9E]);
        // (Out of its body: anywhere across it, low to high.)
        let off = Vec2::new((u(1) * 2.0 - 1.0) * (k.body.half.x + 6.0), (u(2) * 1.3 - 0.5) * k.body.half.y);
        let vel = Vec2::new(off.x.signum() * u(3) * 6.0, RISE.0 + (RISE.1 - RISE.0) * u(4));
        let size = SIZE.0 + (SIZE.1 - SIZE.0) * u(5) * u(5);
        let mut e = commands.spawn((
            Name::new("Plus"),
            Plus { off, vel, age: 0.0, life: LIFE.0 + (LIFE.1 - LIFE.0) * u(6), size, phase: u(7) * std::f32::consts::TAU },
            Sprite { color: Color::srgba(1.0, 1.0, 1.0, 0.0), ..Sprite::from_image(art.0.clone()) },
            Transform::from_translation((k.body.pos + off).extend(19.5)).with_scale(Vec3::ZERO),
        ));
        if w.made.is_multiple_of(LIT_EVERY) {
            e.insert(crate::light::LightSource { color: crate::light::rgb((90, 255, 120), 0.05), flicker: 0.0 });
        }
        sparks.emit(&MOTES, MOTES.count as usize, k.body.pos + off, Vec2::Y, Vec2::ZERO);
    }
    if w.owed < PER_PLUS {
        w.next = w.next.max(0.0);
    }
}

/// Plusses rising with the player, swaying, popping in, twinkling, fading.
fn rise(mut commands: Commands, time: Res<Time>, player: Query<&Kinematics, With<LocalPlayer>>, mut q: Query<(Entity, &mut Plus, &mut Transform, &mut Sprite, Option<&mut crate::light::LightSource>)>) {
    let dt = time.delta_secs();
    let you = player.single().ok().map(|k| k.body.pos);
    for (e, mut p, mut tf, mut sprite, light) in &mut q {
        p.age += dt;
        let f = p.age / p.life;
        if f >= 1.0 {
            commands.entity(e).despawn();
            continue;
        }
        let v = p.vel;
        p.off += v * dt;
        p.vel.y *= 1.0 - (1.2 * dt).min(1.0);
        p.vel.x *= 1.0 - (2.0 * dt).min(1.0);
        let sway = (p.age * 5.0 + p.phase).sin() * SWAY * f.sqrt();
        let at = you.unwrap_or(tf.translation.truncate() - p.off) + p.off + Vec2::X * sway;
        tf.translation = at.extend(19.5);
        // (In: from nothing to a little past full in a tenth of a second,
        // settling; out: shrinking a little as it fades.)
        let pop = (p.age / 0.14).min(1.0);
        let pop = if pop < 1.0 { pop * (1.0 + 0.45 * (pop * std::f32::consts::PI).sin()) } else { 1.0 };
        let out = if f > 0.6 { 1.0 - 0.35 * (f - 0.6) / 0.4 } else { 1.0 };
        tf.scale = Vec3::splat(p.size * pop * out);
        let fade = if f > 0.55 { (1.0 - (f - 0.55) / 0.45).powf(1.4) } else { 1.0 };
        let twinkle = 0.85 + 0.15 * (p.age * 17.0 + p.phase * 3.0).sin();
        sprite.color = Color::srgba(twinkle, 1.0, twinkle, fade);
        if let Some(mut l) = light {
            l.color = crate::light::rgb((90, 255, 120), 0.05 * fade);
        }
    }
}

/// The "+N" floats up and fades.
fn count(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut HealNumber, &mut Transform, &mut TextColor)>) {
    let dt = time.delta_secs();
    for (e, mut n, mut tf, mut color) in &mut q {
        n.age += dt;
        if n.age >= NUMBER_LIFE {
            commands.entity(e).despawn();
            continue;
        }
        tf.translation.y += NUMBER_RISE * dt * (1.0 - n.age / NUMBER_LIFE);
        let f = n.age / NUMBER_LIFE;
        color.0 = Color::srgba(0.45, 1.0, 0.55, if f > 0.6 { 1.0 - (f - 0.6) / 0.4 } else { 1.0 });
    }
}
