//! Hearts: what a kill leaves now and then, a little life to take (hearts.ron).
//!
//! A creature that hunts (team `Enemy`) dying near the player sometimes drops
//! one: red (heals 20), now and then gold (40). The odds are its kind's (or
//! by how tough it was), times how hurt the player is (`odds`): none at full
//! health, steeply more the nearer death, never certain; and nearer death a
//! larger share are gold. A heart pops out, falls, lies bobbing and glowing
//! faintly; near the player it drifts to it (from further and faster the
//! lower its health) and is taken on touch, no potion sickness; left, it
//! fades. Dead, the player takes none (it waits, till it fades).

use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use crate::creatures::player::LocalPlayer;
use crate::creatures::{Died, Health, Kinematics, Team};
use crate::magic::runes::Emitter;
use crate::world::SimWorld;

pub struct HeartsPlugin;

impl Plugin for HeartsPlugin {
    fn build(&self, app: &mut App) {
        let rules: HeartRules = crate::data::load_ron(&crate::data::data_path("hearts.ron")).unwrap_or_else(|e| panic!("{e}"));
        app.insert_resource(rules).add_systems(Startup, make_art).add_systems(Update, (drop, fly).chain());
    }
}

/// How hearts drop and behave (hearts.ron).
#[derive(Resource, Deserialize, Clone, Debug)]
pub struct HeartRules {
    pub chance: HashMap<String, f32>,
    pub tough: (f32, f32, f32),
    pub above: f32,
    pub below: (f32, f32, f32),
    pub brink: f32,
    pub most: f32,
    pub gold: (f32, f32),
    pub gold_tough: (f32, f32),
    pub heal: (f32, f32),
    pub life: f32,
    pub reach: (f32, f32),
    pub speed: (f32, f32),
    pub near: f32,
}

impl HeartRules {
    /// How near death the player is, as hearts take it: 0 at half its
    /// health or more, 1 at the brink and beyond.
    fn low(&self, health: f32) -> f32 {
        ((0.5 - health) / (0.5 - self.brink).max(0.01)).clamp(0.0, 1.0)
    }

    /// A kill's odds of a heart, its kind's own being `base`, the player at
    /// `health` (a share of its most).
    pub fn odds(&self, base: f32, health: f32) -> f32 {
        let times = if health >= 0.5 { (1.0 - health).max(0.0) * self.above } else { self.below.0 + self.below.1 * self.low(health).powf(self.below.2) };
        (base * times).clamp(0.0, self.most)
    }

    /// Of the hearts dropped then, the share that are gold.
    fn gold_share(&self, health: f32, its_health: f32) -> f32 {
        let share = self.gold.0 + (self.gold.1 - self.gold.0) * self.low(health);
        if its_health >= self.gold_tough.0 { share.max(self.gold_tough.1) } else { share }
    }

    /// A kind's own odds: written, or by its health.
    fn base(&self, kind: &str, its_health: f32) -> f32 {
        self.chance.get(kind).copied().unwrap_or_else(|| (its_health * self.tough.0).clamp(self.tough.1, self.tough.2))
    }
}

/// A heart lying (or falling, or drifting to the player).
#[derive(Component)]
pub struct Heart {
    pub heal: f32,
    pub gold: bool,
    age: f32,
    vel: Vec2,
    at: Vec2,
}

/// The two hearts' pictures.
#[derive(Resource)]
struct HeartArt {
    red: Handle<Image>,
    gold: Handle<Image>,
}

/// A heart, as text art ('o' its edge, 'H' it, 'h' its shade, 'W' a shine).
const ART: [&str; 8] = [
    ".oo.oo.", //
    "oWHoHHo", "oHHHHho", "oHHHHho", ".oHHho.", "..oho..", "...o...", ".......",
];

fn make_art(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut paint = |fill: [u8; 3], shade: [u8; 3]| {
        let (w, h) = (7u32, 7u32);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for (y, row) in ART.iter().take(h as usize).enumerate() {
            for (x, ch) in row.chars().enumerate() {
                let c = match ch {
                    'o' => [40, 12, 16, 255],
                    'H' => [fill[0], fill[1], fill[2], 255],
                    'h' => [shade[0], shade[1], shade[2], 255],
                    'W' => [255, 240, 230, 255],
                    _ => continue,
                };
                let i = (y * w as usize + x) * 4;
                rgba[i..i + 4].copy_from_slice(&c);
            }
        }
        images.add(Image::new(
            bevy::render::render_resource::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            bevy::render::render_resource::TextureDimension::D2,
            rgba,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
        ))
    };
    let red = paint([226, 46, 62], [160, 26, 44]);
    let gold = paint([255, 206, 70], [206, 146, 34]);
    commands.insert_resource(HeartArt { red, gold });
}

/// Over a heart's head as it's taken: a few sparks of its colour.
fn glint(gold: bool) -> Emitter {
    Emitter {
        count: 10.0,
        life: (0.2, 0.45),
        colors: if gold { vec![(255, 250, 210), (255, 206, 70)] } else { vec![(255, 220, 220), (230, 60, 80)] },
        speed: 60.0,
        spread: 3.14,
        gravity: -30.0,
        drag: 3.0,
        size: 1.0,
        jitter: 0.0,
        glow: true,
    }
}

/// Hearts out of the dying: what hunts, near the player, by the odds.
fn drop(
    mut commands: Commands,
    rules: Res<HeartRules>,
    art: Option<Res<HeartArt>>,
    sim: Res<SimWorld>,
    mut died: MessageReader<Died>,
    player: Query<(&Kinematics, &Health), With<LocalPlayer>>,
) {
    let Some(art) = art else { return };
    let Ok((pk, h)) = player.single() else { return };
    let health = (h.hp / h.max.max(1.0)).clamp(0.0, 1.0);
    for (n, d) in died.read().enumerate() {
        if d.empty || d.rising.is_some() || d.def.team != Team::Enemy || d.body.pos.distance(pk.body.pos) > rules.near {
            continue;
        }
        let roll = |salt: u64| (platypus_sim::rng::hash(&[sim.world.tick(), n as u64, d.body.pos.x.to_bits() as u64, d.body.pos.y.to_bits() as u64, salt]) % 100_000) as f32 / 100_000.0;
        if roll(1) >= rules.odds(rules.base(&d.kind, d.def.health), health) {
            continue;
        }
        let gold = roll(2) < rules.gold_share(health, d.def.health);
        let at = d.body.pos;
        let vel = Vec2::new((roll(3) - 0.5) * 60.0, 90.0);
        let (image, light) = if gold { (art.gold.clone(), (255, 206, 90)) } else { (art.red.clone(), (240, 60, 80)) };
        commands.spawn((
            Name::new("Heart"),
            Heart { heal: if gold { rules.heal.1 } else { rules.heal.0 }, gold, age: 0.0, vel, at },
            Sprite::from_image(image),
            Transform::from_translation(at.extend(5.8)),
            Visibility::default(),
            crate::light::LightSource { color: crate::light::rgb(light, 0.04), flicker: 0.0 },
        ));
    }
}

/// Hearts falling, lying, drifting to the player and taken; fading.
#[allow(clippy::too_many_arguments)]
fn fly(
    mut commands: Commands,
    time: Res<Time>,
    rules: Res<HeartRules>,
    sim: Res<SimWorld>,
    dying: Res<crate::death::Dying>,
    mut sparks: ResMut<crate::vfx::Sparks>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
    mut player: Query<(&Kinematics, &mut Health), With<LocalPlayer>>,
    mut hearts: Query<(Entity, &mut Heart, &mut Transform, &mut Visibility)>,
) {
    let dt = time.delta_secs().min(0.05);
    let Ok((pk, mut h)) = player.single_mut() else { return };
    let low = rules.low((h.hp / h.max.max(1.0)).clamp(0.0, 1.0));
    let (reach, speed) = (rules.reach.0 + (rules.reach.1 - rules.reach.0) * low, rules.speed.0 + (rules.speed.1 - rules.speed.0) * low);
    let alive = dying.0.is_none();
    let solid = |p: Vec2| sim.world.is_solid(platypus_sim::CellPos::from_world(p.x, p.y));
    for (e, mut heart, mut tf, mut vis) in &mut hearts {
        heart.age += dt;
        if heart.age > rules.life {
            commands.entity(e).despawn();
            continue;
        }
        let to = pk.body.pos - heart.at;
        if alive && to.length() < reach {
            // Drifting to the player, through whatever's between.
            let want = to.normalize_or_zero() * speed;
            heart.vel = heart.vel.lerp(want, (dt * 8.0).min(1.0));
            let v = heart.vel;
            heart.at += v * dt;
        } else {
            // Falling; lying on what's under it.
            heart.vel.y -= 300.0 * dt;
            heart.vel.x *= 1.0 - (dt * 3.0).min(1.0);
            let next = heart.at + heart.vel * dt;
            if solid(next - Vec2::Y * 3.5) {
                heart.vel = Vec2::ZERO;
            } else {
                heart.at = next;
            }
        }
        // Taken.
        if alive && pk.body.pos.distance(heart.at) < 9.0 {
            h.hp = (h.hp + heart.heal).min(h.max);
            sparks.emit(&glint(heart.gold), 10, heart.at, Vec2::Y, Vec2::ZERO);
            sounds.write(crate::sound::PlaySound::at(if heart.gold { "heart_gold" } else { "heart" }, heart.at));
            commands.entity(e).despawn();
            continue;
        }
        // Bobbing; blinking its last three seconds.
        let bob = (heart.age * 3.0).sin() * 1.2;
        tf.translation = (heart.at + Vec2::Y * (bob + 2.0)).extend(5.8);
        let blink = rules.life - heart.age < 3.0 && (heart.age * 8.0).fract() < 0.4;
        let want = if blink { Visibility::Hidden } else { Visibility::Inherited };
        if *vis != want {
            *vis = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> HeartRules {
        crate::data::parse_ron(include_str!("../../../assets/data/hearts.ron")).unwrap()
    }

    /// The odds as agreed: none at full health, a rat's 6 % times 1.25 at
    /// half, times about 4 at a fifth, 6 at the brink; never past 70 %.
    #[test]
    fn odds_rise_as_health_falls_and_are_never_certain() {
        let r = rules();
        let rat = |f: f32| r.odds(0.06, f);
        assert_eq!(rat(1.0), 0.0);
        assert!((rat(0.5) - 0.075).abs() < 0.002, "{}", rat(0.5));
        assert!((rat(0.2) - 0.24).abs() < 0.01, "{}", rat(0.2));
        assert!((rat(0.05) - 0.36).abs() < 0.005, "{}", rat(0.05));
        assert!(rat(0.75) < rat(0.5) && rat(0.3) > rat(0.4) && rat(0.1) > rat(0.2));
        assert_eq!(r.odds(0.2, 0.05), 0.7, "a skeleton near the brink: the ceiling");
        assert!(r.gold_share(0.05, 70.0) > r.gold_share(0.8, 70.0), "more gold near death");
        assert_eq!(r.base("ogre", 1000.0), 0.4, "tough unknowns: by their health, at most");
    }
}
