//! The bestiary's live preview: the card opened, in a world with a stage
//! (the arena's: `ChunkGenerator::stage`, a room sealed in bedrock), the
//! real creature is put on the stage's real floor with a stand-in to go
//! for, and shown through a camera of its own drawing into an image (the
//! card shows it). Its brain is taken off (`Staged`): the stage drives it,
//! over and over: standing, walking there and back, a few swings or shots
//! of what it wields, then each of its moves in turn (the stand-in put
//! where the move reaches), with what each can hit drawn over it while
//! it's coming and while it's live (`Moves::shapes`: strikes red, grabs
//! violet, slams orange; faint until they can hit).

use bevy::prelude::*;

use crate::creatures::brain::{BrainRegistry, BrainSet, Staged};
use crate::creatures::def::Creatures;
use crate::creatures::moves::{MoveBook, Moves};
use crate::creatures::{Controls, Creature, Health, Kinematics, Team};
use platypus_sim::CellPos;

use crate::world::SimWorld;

/// The image (px): the room at 2 screen pixels a cell.
const IMAGE: (u32, u32) = (660, 312);
/// The stand-in: what's struck at.
const STAND_IN: &str = "dummy";

pub struct StagePlugin;

impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StageCast>()
            .add_systems(Startup, setup)
            .add_systems(Update, (cast, shapes))
            .add_systems(FixedUpdate, direct.in_set(BrainSet).before(crate::creatures::moves::start));
    }
}

/// One stretch of the show.
#[derive(Clone, Debug, PartialEq)]
enum Beat {
    Stand(f32),
    /// Walk this way (-1, 1) this long.
    Walk(f32, f32),
    /// Swing what it wields, this many times.
    Swing(u32),
    /// Draw its bow and loose.
    Shoot,
    /// Its move (by its place in its list).
    Move(usize),
}

/// The stage, if this world has one.
#[derive(Resource)]
pub struct Stage {
    pub image: Handle<Image>,
    camera: Entity,
    /// Its inside: the floor's left end, and the ceiling's right.
    lo: Vec2,
    hi: Vec2,
    /// Who's on it, the stand-in, and the show.
    pub showing: Option<String>,
    actor: Option<Entity>,
    stand_in: Option<Entity>,
    beats: Vec<Beat>,
    beat: usize,
    t: f32,
    /// What's being shown, said under the picture.
    pub caption: String,
}

impl Stage {
    /// Which stretch of the show it's at (0 again: round once).
    pub fn beat(&self) -> usize {
        self.beat
    }
}

fn setup(mut commands: Commands, sim: Res<SimWorld>, mut images: ResMut<Assets<Image>>) {
    let Some((lo, hi)) = sim.generator.stage() else { return };
    let (lo, hi) = (Vec2::new(lo.x as f32, lo.y as f32), Vec2::new(hi.x as f32, hi.y as f32));
    let image = images.add(Image::new_target_texture(IMAGE.0, IMAGE.1, bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb, None));
    // (The floor a little way up from the bottom: what it stands on shows.)
    let middle = Vec2::new((lo.x + hi.x) / 2.0, lo.y - 20.0 + IMAGE.1 as f32 / 4.0);
    let camera = commands
        .spawn((
            Name::new("Bestiary stage camera"),
            Camera2d,
            Camera { order: -1, is_active: false, clear_color: ClearColorConfig::Custom(Color::srgb(0.34, 0.38, 0.46)), ..default() },
            bevy::camera::RenderTarget::Image(image.clone().into()),
            Projection::Orthographic(OrthographicProjection { scale: 0.5, ..OrthographicProjection::default_2d() }),
            Transform::from_translation(middle.extend(100.0)),
            // (Its room kept loaded while it's on.)
            crate::world::ChunkLoader { half_extent: (hi - lo) / 2.0 },
        ))
        .id();
    commands.insert_resource(Stage { image, camera, lo, hi, showing: None, actor: None, stand_in: None, beats: Vec::new(), beat: 0, t: 0.0, caption: String::new() });
}

/// Who's on the stage follows the card opened: the old one off, the new
/// one put on (its brain taken off) with the stand-in; the camera on only
/// while there's something to show.
#[allow(clippy::too_many_arguments)]
fn cast(
    mut commands: Commands,
    stage: Option<ResMut<Stage>>,
    bestiary: Res<super::panel::Bestiary>,
    creatures: Res<Creatures>,
    weapons: Option<Res<crate::combat::Weapons>>,
    mut cams: Query<(&mut Camera, &Transform)>,
    mut view: ResMut<crate::canvas::StageView>,
    staged: Query<(), With<Staged>>,
    mut sim: ResMut<SimWorld>,
) {
    let Some(mut stage) = stage else { return };
    // (Taken away, the world reset: put on again.)
    if [stage.actor, stage.stand_in].iter().flatten().any(|e| !staged.contains(*e)) {
        stage.showing = None;
    }
    let wanted = bestiary.picked.clone().filter(|_| bestiary.open);
    if let Ok((mut c, tf)) = cams.get_mut(stage.camera) {
        c.is_active = wanted.is_some();
        // (Canvases, the spider's legs, drawn for its view too.)
        let half = Vec2::new(IMAGE.0 as f32, IMAGE.1 as f32) / 4.0;
        view.0 = wanted.is_some().then_some((tf.translation.truncate(), half));
    }
    if wanted == stage.showing {
        return;
    }
    for e in [stage.actor.take(), stage.stand_in.take()].into_iter().flatten() {
        commands.entity(e).try_despawn();
    }
    stage.showing = wanted.clone();
    stage.caption.clear();
    let Some(id) = wanted else { return };
    clean(&mut sim, stage.lo, stage.hi);
    let Some(def) = creatures.get(&id).cloned() else { return };
    let floor = stage.lo.y;
    let at = Vec2::new(stage.lo.x + 60.0 + def.size.0 / 2.0, floor + 1.0);
    let brain = def.brain.kind.clone();
    // (Put on through the creature's own spawning, its brain then taken off.)
    crate::creatures::def::spawn_creature(&mut commands, &id, at, move |e| {
        e.insert(Staged);
        let me = e.id();
        e.world_scope(|world| {
            world.resource_scope(|world, reg: Mut<BrainRegistry>| {
                if let Ok(mut ent) = world.get_entity_mut(me) {
                    reg.remove(&brain, &mut ent);
                }
            });
            world.resource_mut::<StageCast>().0 = Some(me);
        });
    });
    crate::creatures::def::spawn_creature(&mut commands, STAND_IN, Vec2::new(stage.lo.x + 200.0, floor + 1.0), |e| {
        // On the hunted side, so moves go for it.
        e.insert((Staged, Team::Villager));
        let me = e.id();
        e.world_scope(|world| world.resource_mut::<StageCast>().1 = Some(me));
    });
    // The show: stand, walk, its weapon, its moves.
    let mut beats = vec![Beat::Stand(1.2), Beat::Walk(1.0, 1.0), Beat::Walk(-1.0, 1.0), Beat::Stand(0.4)];
    if let Some(w) = &def.weapon {
        let bow = weapons.as_ref().and_then(|ws| ws.bow_index(w)).is_some();
        beats.push(if bow { Beat::Shoot } else { Beat::Swing(2) });
    }
    beats.extend((0..def.moves.len()).map(Beat::Move));
    stage.beats = beats;
    stage.beat = 0;
    stage.t = 0.0;
}

/// The room as it was made: what the last left there (acid, blood, holes)
/// gone, its floor and step stone again (as the arena makes them).
fn clean(sim: &mut SimWorld, lo: Vec2, hi: Vec2) {
    let Some(stone) = sim.materials().id("stone") else { return };
    let (x0, f, x1, y1) = (lo.x as i32, lo.y as i32, hi.x as i32, hi.y as i32);
    // (The few chunks it's in, as made.)
    let mut made: std::collections::HashMap<platypus_sim::ChunkPos, platypus_sim::Chunk> = Default::default();
    for x in x0..x1 {
        // (Its step: as high as the ground the generator says is there.)
        let ground = (f..f + 24)
            .find(|&y| {
                let at = CellPos::new(x, y).chunk();
                let chunk = made.entry(at).or_insert_with(|| sim.generator.generate(at));
                chunk.cells()[cell_index(x, y)].material == platypus_sim::MaterialId::AIR
            })
            .unwrap_or(f);
        for y in f - 6..y1 {
            let cell = if y < ground { platypus_sim::Cell::new(stone, ((x * 7 + y * 13) & 255) as u8) } else { platypus_sim::Cell::AIR };
            sim.world.set(CellPos::new(x, y), cell);
        }
    }
}

/// A cell's place in its chunk's cells.
fn cell_index(x: i32, y: i32) -> usize {
    let c = platypus_sim::CHUNK;
    (y.rem_euclid(c) * c + x.rem_euclid(c)) as usize
}

/// Who was put on (filled in as they're spawned).
#[derive(Resource, Default)]
pub struct StageCast(Option<Entity>, Option<Entity>);

type Actor<'a> = (&'a mut Kinematics, &'a mut Controls, Option<&'a mut Moves>, Has<crate::combat::Swing>);

/// The show, a tick at a time (as a brain would: it writes `Controls`).
#[allow(clippy::too_many_arguments)]
fn direct(
    mut commands: Commands,
    stage: Option<ResMut<Stage>>,
    mut cast: ResMut<StageCast>,
    book: Res<MoveBook>,
    mut actors: Query<Actor, With<Staged>>,
    mut health: Query<&mut Health, With<Staged>>,
    mut swings: MessageWriter<crate::combat::MeleeRequest>,
    mut draws: MessageWriter<crate::archery::DrawBow>,
) {
    let Some(mut stage) = stage else { return };
    if let Some(e) = cast.0.take() {
        stage.actor = Some(e);
    }
    if let Some(e) = cast.1.take() {
        stage.stand_in = Some(e);
        // (A dummy that isn't one: no tally, no going back where it stood;
        // taken off now it's all there.)
        commands.entity(e).remove::<(crate::creatures::custom::dummy::Dummy, crate::creatures::custom::dummy::Tally)>();
    }
    let (Some(actor), Some(stand_in)) = (stage.actor, stage.stand_in) else { return };
    const DT: f32 = 1.0 / crate::world::TICK_HZ as f32;
    // Neither dies on the stage.
    for e in [actor, stand_in] {
        if let Ok(mut h) = health.get_mut(e) {
            h.hp = h.max;
        }
    }
    let Ok([(mut ak, mut ac, moves, swinging), (mut sk, ..)]) = actors.get_many_mut([actor, stand_in]) else { return };
    // Off the stage somehow (flung, flown): back on.
    let (lo, hi) = (stage.lo, stage.hi);
    for k in [&mut *ak, &mut *sk] {
        if !(lo.x..hi.x).contains(&k.body.pos.x) || !(lo.y..hi.y).contains(&k.body.pos.y) {
            k.body.pos = Vec2::new((lo.x + hi.x) / 2.0, lo.y + k.body.half.y + 2.0);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
        }
    }
    let Some(beat) = stage.beats.get(stage.beat).cloned() else { return };
    let mut moves = moves;
    ac.0.move_x = 0.0;
    ac.0.move_y = 0.0;
    ac.0.jump = false;
    ac.0.aim = sk.body.pos;
    let first = stage.t == 0.0;

    stage.t += DT;
    let t = stage.t;
    let done = match beat {
        Beat::Stand(secs) => {
            stage.caption = "standing".into();
            t >= secs
        }
        Beat::Walk(dir, secs) => {
            stage.caption = "walking".into();
            // (Not into the walls.)
            let room = if dir > 0.0 { hi.x - 40.0 - ak.body.pos.x } else { ak.body.pos.x - lo.x - 40.0 };
            ac.0.move_x = if room > 0.0 { dir } else { 0.0 };
            t >= secs
        }
        Beat::Swing(n) => {
            stage.caption = "its weapon".into();
            // (The stand-in in reach.)
            if first {
                sk.body.pos.x = ak.body.pos.x + ak.body.half.x + 24.0;
            }
            if (t / 0.9).floor() > ((t - DT) / 0.9).floor() || first {
                swings.write(crate::combat::MeleeRequest { attacker: actor, at: sk.body.pos });
            }
            t >= 0.9 * n as f32 + 0.6
        }
        Beat::Shoot => {
            stage.caption = "its bow".into();
            if first {
                sk.body.pos.x = (ak.body.pos.x + 220.0).min(hi.x - 30.0);
            }
            if t < 1.2 {
                draws.write(crate::archery::DrawBow { archer: actor, at: sk.body.pos });
            }
            t >= 2.4
        }
        Beat::Move(i) => {
            let Some(moves) = moves.as_deref_mut() else {
                stage.beat += 1;
                return;
            };
            let id = moves.ids().get(i).cloned().unwrap_or_default();
            // (Not while a swing of what it wields is still going.)
            if first && swinging {
                stage.t = 0.0;
                return;
            }
            if first {
                // The stand-in where the move reaches (its range's middle,
                // as the room allows), on the side it faces.
                let (near, far) = book.get(&id).map_or((0.0, 60.0), |m| m.when.range);
                let want = ((near + far.min(near + 120.0)) / 2.0).max(near + 4.0);
                let x = (ak.body.pos.x + want).min(hi.x - 20.0);
                sk.body.pos = Vec2::new(x, lo.y + sk.body.half.y + 1.0);
                sk.body.vel = Vec2::ZERO;
                sk.prev_pos = sk.body.pos;
                let dir = sk.body.pos - ak.body.pos;
                moves.force(i, stand_in, dir);
            }
            stage.caption = match moves.doing(&book) {
                Some((id, _, phase)) if !phase.is_empty() => format!("{id}: {phase}"),
                _ => id.clone(),
            };
            // On once it's over, a moment later (or if it never got going).
            !moves.busy() && t > 0.6 || t > 8.0
        }
    };
    if done {
        stage.beat = (stage.beat + 1) % stage.beats.len().max(1);
        stage.t = 0.0;
    }
}

/// What a staged move can hit, drawn over it while it's live (and, with the
/// arena's overlays on, everyone's).
fn shapes(
    book: Res<MoveBook>,
    view: Res<crate::arena::ArenaView>,
    mut gizmos: Gizmos,
    q: Query<(&Kinematics, &Moves, Has<Staged>), With<Creature>>,
) {
    for (k, m, staged) in &q {
        if !(staged || view.overlays) {
            continue;
        }
        for (at, reach, what, live) in m.shapes(&book, k.body.pos, k.body.half) {
            let color = match what {
                "grab" => Color::srgb(0.75, 0.45, 1.0),
                "slam" => Color::srgb(1.0, 0.6, 0.2),
                _ => Color::srgb(1.0, 0.25, 0.25),
            };
            // (Faint while it's coming, bright while it can hit.)
            gizmos.rect_2d(Isometry2d::from_translation(at), Vec2::splat(reach * 2.0), if live { color } else { color.with_alpha(0.35) });
        }
    }
}
