//! Lights out of the hand: a torch, a flashlight, a lantern in the main
//! hand. A tap puts it down on a block face within reach (a torch planted,
//! a flashlight lying lit, shining the way you face, a lantern standing);
//! held, its arc shows (as a bomb's), and let go, it's thrown:
//!
//! - a **torch** flies spinning, burning: what it hits catches fire (a
//!   creature, a TNT barrel's fuse), firedamp it comes near goes up
//!   (`hazards`), what burns where it lands catches, and where it comes to
//!   rest it burns on, planted;
//! - a **flashlight** flies lit, its beam sweeping round as it spins, and
//!   lies where it lands, still shining;
//! - a **lantern** smashes where it lands, its oil out and alight: a
//!   firebomb. (A safety lamp's sturdier: it lands, and stays lit.)
//!
//! T throws what's in the off hand the same way (held: its arc; let go,
//! one of them thrown).
//!
//! Right-click one that's down, within reach, to take it back. They're
//! saved with the world.

use bevy::prelude::*;
use platypus_physics::{Body, Locomotion};
use platypus_sim::{CellPos, WorldEdit, cell::flags};

use super::items::{ItemId, Items, Stack};
use crate::creatures::Kinematics;
use crate::light::{Carry, LightKind, LightSettings, LightSource};
use crate::world::{SimWorld, TICK_HZ};

const DT: f32 = (1.0 / TICK_HZ) as f32;

/// What can come out of the hand this way (by the light it gives).
pub fn throwable(kind: Option<LightKind>) -> Option<LightKind> {
    kind.filter(|k| k.held())
}

/// How long the button's held before it's a throw (s): a tap puts it down.
pub const HOLD_TO_THROW: f32 = 0.2;

/// A light out of the hand: put down there, its beam (if any) that way; or
/// thrown, from there, so.
#[derive(Clone, Copy, Debug)]
pub enum Out {
    Place { at: Vec2, dir: Vec2 },
    Throw { from: Vec2, vel: Vec2 },
}

#[derive(Message, Clone, Copy, Debug)]
pub struct LightOut {
    pub by: Entity,
    pub item: ItemId,
    pub kind: LightKind,
    pub how: Out,
}

/// A light put down (or come to rest), to be taken back: what it was, and
/// which way its beam goes.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlacedLight {
    pub item: ItemId,
    /// Where it was put (a block face's top, its middle).
    pub at: Vec2,
    pub dir: Vec2,
}

/// Lights down in a save, to put back once their pictures are loaded: what,
/// where, which way.
#[derive(Resource, Default)]
pub struct PendingLights(pub Vec<(ItemId, Vec2, Vec2)>);

/// A flashlight's beam, from where this is, `dir` that way (the big one's
/// or the pocket one's: `LightSettings`).
#[derive(Component, Clone, Copy, Debug)]
pub struct BeamLight {
    pub dir: Vec2,
    pub big: bool,
}

/// A light in flight.
#[derive(Component)]
struct ThrownLight {
    by: Entity,
    item: ItemId,
    kind: LightKind,
    /// How it's turned (degrees), how fast it spins (degrees a second).
    angle: f32,
    spin: f32,
    /// How long it's lain still (s).
    still: f32,
    /// Its speed a tick ago (a sharp change: it hit something).
    was: Vec2,
    /// What it's set alight already.
    hit: Vec<Entity>,
    /// Its picture (a child), and its light's (a flashlight's beam, a
    /// torch's flame) offset.
    sprite: Entity,
}

pub struct LightsPlugin;

impl Plugin for LightsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<LightOut>()
            .init_resource::<PendingLights>()
            .add_systems(Update, (toss, out, pick_up, put_back).chain())
            .add_systems(FixedUpdate, fly.after(crate::world::TickSet::Bodies).before(crate::world::TickSet::Cells));
    }
}

/// How fast a thrown light spins (degrees a second at full speed), and
/// slows to still (cells a second); how long it lies still before it's down.
const SPIN: f32 = 720.0;
const STILL: f32 = 6.0;
const DOWN_AFTER: f32 = 0.3;
/// A hit: a speed change this sharp (a share of its speed, from at least
/// this fast).
const IMPACT: f32 = 0.45;
const IMPACT_FROM: f32 = 60.0;
/// What a thrown torch does to what it hits: this much fire harm, this much
/// of it set alight; what it lights where it lands (cells).
const TORCH_HIT: f32 = 3.0;
const TORCH_ALIGHT: f32 = 0.4;
const TORCH_IGNITES: i32 = 2;
/// A lantern smashed: so many cells of its oil out, alight, this fast.
const LANTERN_OIL: usize = 36;
const LANTERN_SPLASH: f32 = 2.2;
/// Taken back: right-clicked this near it, it this near you (cells).
const PICK_NEAR: f32 = 10.0;
const PICK_REACH: f32 = 56.0;

/// Its picture as a child of `e`, turned `angle` (mirrored facing left).
fn picture(commands: &mut Commands, art: &crate::light::torch::OffHandArt, kind: LightKind, angle: f32, left: bool) -> Option<(Entity, Vec2)> {
    let (mut sprite, tip) = art.look(kind.carry(), if left { 180.0 - angle } else { angle })?;
    sprite.flip_x = left;
    let tip = if left { Vec2::new(-tip.x, tip.y) } else { tip };
    Some((commands.spawn((sprite, Transform::from_xyz(0.0, 0.0, 0.01))).id(), tip))
}

/// Put down: what it is, at a block face's top (`at`, its middle), shining
/// `dir` if it's a flashlight.
pub fn place(commands: &mut Commands, settings: &LightSettings, art: &crate::light::torch::OffHandArt, torch_art: &crate::light::TorchArt, item: ItemId, kind: LightKind, at: Vec2, dir: Vec2) -> Entity {
    let placed = PlacedLight { item, at, dir };
    match kind.carry() {
        Carry::Torch => {
            let e = crate::light::plant_torch(commands, at, settings, torch_art);
            commands.entity(e).insert(placed);
            e
        }
        carry => {
            let left = dir.x < 0.0;
            let angle = dir.y.atan2(dir.x.abs()).to_degrees();
            let root = commands.spawn((Name::new("Placed light"), placed, Transform::from_translation((at + Vec2::Y * 3.0).extend(9.0)), Visibility::default())).id();
            if let Some((sprite, tip)) = picture(commands, art, kind, angle, left) {
                commands.entity(root).add_child(sprite);
                match carry {
                    Carry::SmallBeam | Carry::BigBeam => {
                        let beam = commands.spawn((BeamLight { dir, big: carry == Carry::BigBeam }, Transform::from_translation(tip.extend(0.0)))).id();
                        commands.entity(root).add_child(beam);
                    }
                    _ => {
                        let glow = commands.spawn((LightSource { color: crate::light::rgb(settings.lamp.color, settings.lamp.strength), flicker: 0.0 }, crate::light::Haze(settings.lamp.haze), Transform::from_translation(tip.extend(0.0)))).id();
                        commands.entity(root).add_child(glow);
                    }
                }
            }
            root
        }
    }
}

/// T: what's in the off hand readied (held: its arc), and thrown on letting
/// go: one of it, the rest kept there.
#[allow(clippy::too_many_arguments)]
fn toss(
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Res<crate::camera::CursorWorld>,
    items: Option<Res<Items>>,
    tools: Res<crate::tools::ToolsConfig>,
    (taken, open, dying): (Res<crate::dev::KeyboardTaken>, Res<super::InventoryOpen>, Res<crate::death::Dying>),
    mut hand: ResMut<super::Hand>,
    mut commands: Commands,
    mut player: Query<(Entity, &Kinematics, &mut crate::gear::Equipment), With<crate::creatures::player::LocalPlayer>>,
    mut outs: MessageWriter<LightOut>,
) {
    let (Some(items), Ok((me, k, mut eq))) = (items, player.single_mut()) else { return };
    let off = eq.worn[crate::gear::OFF_HAND];
    let light = off.and_then(|s| throwable(items.def(s.item).light).map(|kind| (s, kind)));
    if taken.0 || open.0 || dying.0.is_some() || light.is_none() {
        hand.tossing = false;
        return;
    }
    if keys.pressed(KeyCode::KeyT) {
        hand.tossing = true;
        let from = super::hand_at(k);
        commands.entity(me).insert(crate::creatures::body::animation::Aiming { at: from + Vec2::new(k.loco.facing * 20.0, 24.0), left: 0.15 });
        return;
    }
    if !std::mem::take(&mut hand.tossing) {
        return;
    }
    let (Some((stack, kind)), Some(at)) = (light, cursor.0) else { return };
    let from = super::hand_at(k);
    let vel = super::lob(from, at, tools.bomb.throw_speed) + k.body.vel * 0.5;
    commands.entity(me).insert(crate::creatures::body::animation::Aiming { at, left: 0.3 });
    outs.write(LightOut { by: me, item: stack.item, kind, how: Out::Throw { from, vel } });
    eq.worn[crate::gear::OFF_HAND] = (stack.count > 1).then(|| Stack { count: stack.count - 1, ..stack });
}

/// What the hands put out: put down, or thrown.
fn out(
    mut commands: Commands,
    mut outs: MessageReader<LightOut>,
    settings: Res<LightSettings>,
    art: Option<Res<crate::light::torch::OffHandArt>>,
    torch_art: Option<Res<crate::light::TorchArt>>,
) {
    let (Some(art), Some(torch_art)) = (art, torch_art) else {
        outs.clear();
        return;
    };
    for o in outs.read() {
        match o.how {
            Out::Place { at, dir } => {
                place(&mut commands, &settings, &art, &torch_art, o.item, o.kind, at, dir);
            }
            Out::Throw { from, vel } => {
                let left = vel.x < 0.0;
                let Some((sprite, tip)) = picture(&mut commands, &art, o.kind, 0.0, left) else { continue };
                let mut body = Body::new(from, Vec2::splat(2.5));
                body.vel = vel;
                let mut e = commands.spawn((
                    Name::new("Thrown light"),
                    crate::props::Thrown { bounce: 0.25, slide: 0.6 },
                    Kinematics { body, loco: Locomotion::default(), prev_pos: from },
                    Transform::from_translation(from.extend(12.0)),
                    Visibility::default(),
                ));
                e.add_child(sprite);
                match o.kind.carry() {
                    Carry::Torch => {
                        e.insert((LightSource { color: crate::light::rgb(settings.torch.color, settings.torch.strength), flicker: 1.0 }, crate::light::Haze(settings.torch.haze), crate::light::torch::Flame::at(tip)));
                    }
                    Carry::SmallBeam | Carry::BigBeam => {
                        e.insert(BeamLight { dir: Vec2::new(if left { -1.0 } else { 1.0 }, 0.0), big: o.kind.carry() == Carry::BigBeam });
                    }
                    _ => {
                        e.insert((LightSource { color: crate::light::rgb(settings.lamp.color, settings.lamp.strength), flicker: 0.0 }, crate::light::Haze(settings.lamp.haze)));
                    }
                }
                let spin = if left { SPIN } else { -SPIN };
                e.insert(ThrownLight { by: o.by, item: o.item, kind: o.kind, angle: 0.0, spin, still: 0.0, was: vel, hit: Vec::new(), sprite });
            }
        }
    }
}

/// Lights in flight: spinning; a torch setting alight what it hits, and
/// what burns where it strikes; a lantern smashing; down when still.
#[allow(clippy::type_complexity)]
fn fly(
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    settings: Res<LightSettings>,
    art: Option<Res<crate::light::torch::OffHandArt>>,
    torch_art: Option<Res<crate::light::TorchArt>>,
    mut thrown: Query<(Entity, &mut ThrownLight, &Kinematics, Option<&mut BeamLight>)>,
    bodies: Query<(Entity, &Kinematics, Has<crate::creatures::player::LocalPlayer>), (With<crate::creatures::Health>, Without<ThrownLight>)>,
    mut sprites: Query<&mut Sprite>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    let (Some(art), Some(torch_art)) = (art, torch_art) else { return };
    for (e, mut t, k, beam) in &mut thrown {
        let vel = k.body.vel;
        let speed = vel.length();
        let was = t.was.length();
        let struck = was > IMPACT_FROM && (vel - t.was).length() > IMPACT * was;
        t.was = vel;
        t.angle += t.spin * DT * (speed / 200.0).min(1.0);
        let left = t.spin > 0.0;
        if let Ok(mut s) = sprites.get_mut(t.sprite)
            && let Some((look, _)) = art.look(t.kind.carry(), if left { 180.0 - t.angle } else { t.angle })
        {
            s.texture_atlas = look.texture_atlas;
        }
        if let Some(mut b) = beam {
            b.dir = Vec2::from_angle(t.angle.to_radians());
        }
        // (What it's passing through or into.)
        let mut on: Option<Entity> = None;
        for (be, bk, is_player) in &bodies {
            if be == t.by || is_player || t.hit.contains(&be) {
                continue;
            }
            let d = (bk.body.pos - k.body.pos).abs();
            if d.x < bk.body.half.x + 3.0 && d.y < bk.body.half.y + 3.0 {
                on = Some(be);
                break;
            }
        }
        let at = CellPos::from_world(k.body.pos.x, k.body.pos.y);
        match t.kind.carry() {
            Carry::Torch => {
                if let Some(be) = on {
                    t.hit.push(be);
                    let dir = vel.normalize_or(Vec2::X);
                    hits.write(crate::combat::Hit { by: Some(t.by), target: be, damage: TORCH_HIT, harm: crate::creatures::Harm::Fire, knock: dir * 60.0, stun: 0.0, at: k.body.pos, dir, weight: 0.3, crit: false });
                    commands.entity(be).insert(crate::creatures::body::elements::Burning::new(TORCH_ALIGHT));
                }
                if struck {
                    sim.queue(WorldEdit::Ignite { center: at, radius: TORCH_IGNITES });
                }
            }
            Carry::Lantern if struck || on.is_some() => {
                // Smashed: its oil out, burning, and what it's in alight.
                let mats = sim.materials().clone();
                if let Some(oil) = mats.id("oil") {
                    let mut rng = platypus_sim::rng::Rng::seeded(&[sim.world.tick(), e.to_bits()]);
                    for i in 0..LANTERN_OIL {
                        let a = i as f32 / LANTERN_OIL as f32 * std::f32::consts::TAU;
                        let mut cell = mats.spawn(oil, &mut rng);
                        cell.flags |= flags::BURNING;
                        cell.life = mats.phys(oil).burn_time;
                        let s = LANTERN_SPLASH * (0.4 + 0.6 * (rng.next_u32() % 1000) as f32 / 1000.0);
                        sim.world.emit(platypus_sim::Particle::new([k.body.pos.x, k.body.pos.y + 1.0], [a.cos() * s, a.sin().abs() * s], cell, 180, platypus_sim::Landing::Settle));
                    }
                }
                sim.queue(WorldEdit::Ignite { center: at, radius: 3 });
                sounds.write(crate::sound::PlaySound::at("break", k.body.pos).pitch(1.4));
                commands.entity(e).despawn();
                continue;
            }
            _ => {}
        }
        // Lying still: down, as if put there.
        t.still = if speed < STILL { t.still + DT } else { 0.0 };
        if t.still >= DOWN_AFTER {
            let feet = k.body.pos - Vec2::Y * k.body.half.y;
            let dir = Vec2::new(if Vec2::from_angle(t.angle.to_radians()).x < 0.0 { -1.0 } else { 1.0 }, 0.08).normalize();
            if t.kind.carry() == Carry::Torch {
                sim.queue(WorldEdit::Ignite { center: CellPos::from_world(feet.x, feet.y - 1.0), radius: TORCH_IGNITES });
            }
            place(&mut commands, &settings, &art, &torch_art, t.item, t.kind, feet, dir);
            commands.entity(e).despawn();
        }
    }
}

/// A save's lights put back, once their pictures are there.
fn put_back(
    mut commands: Commands,
    mut pending: ResMut<PendingLights>,
    items: Option<Res<Items>>,
    settings: Res<LightSettings>,
    art: Option<Res<crate::light::torch::OffHandArt>>,
    torch_art: Option<Res<crate::light::TorchArt>>,
) {
    if pending.0.is_empty() {
        return;
    }
    let (Some(items), Some(art), Some(torch_art)) = (items, art, torch_art) else { return };
    for (item, at, dir) in std::mem::take(&mut pending.0) {
        if let Some(kind) = throwable(items.def(item).light) {
            place(&mut commands, &settings, &art, &torch_art, item, kind, at, dir);
        }
    }
}

/// Right-clicked, one that's down near the cursor and in reach: taken back.
#[allow(clippy::too_many_arguments)]
fn pick_up(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor: Res<crate::camera::CursorWorld>,
    items: Option<Res<Items>>,
    (open, over_ui, dying): (Res<super::InventoryOpen>, Res<crate::dev::PointerOverUi>, Res<crate::death::Dying>),
    mut player: Query<(&Kinematics, &mut super::items::Inventory), With<crate::creatures::player::LocalPlayer>>,
    placed: Query<(Entity, &PlacedLight, &GlobalTransform)>,
    mut sounds: MessageWriter<crate::sound::PlaySound>,
) {
    if !mouse.just_pressed(MouseButton::Right) || open.0 || over_ui.0 || dying.0.is_some() {
        return;
    }
    let (Some(items), Some(at), Ok((k, mut inv))) = (items, cursor.0, player.single_mut()) else { return };
    let near = placed
        .iter()
        .map(|(e, p, tf)| (e, p, tf.translation().truncate() + Vec2::Y * 3.0))
        .filter(|(_, _, pos)| pos.distance(at) < PICK_NEAR && pos.distance(k.body.pos) < PICK_REACH)
        .min_by(|a, b| a.2.distance(at).total_cmp(&b.2.distance(at)));
    let Some((e, p, pos)) = near else { return };
    let left = inv.add(&items, Stack::new(p.item, 1));
    if left > 0 {
        super::spawn_drop(&mut commands, &items, pos, Stack::new(p.item, left));
    }
    commands.entity(e).despawn();
    sounds.write(crate::sound::PlaySound::at("clink", pos).pitch(1.2).volume(0.5));
}
