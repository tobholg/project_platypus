//! Dev: reset the world (the dev panel, twice to be sure). Two ways:
//!
//! - **The world**: every changed chunk forgotten (they come back as the
//!   seed made them, as they stream in), and everything in it taken away
//!   (creatures, chests and stations, items lying about, bodies, thrown
//!   things, planted torches, what's in flight), the spawns made again as
//!   they come into view, the enemies about the start put back; you and
//!   what you carry stay, put back at the start.
//! - **Everything**: the world, and you too: a new player at the start with
//!   what one starts with, no progress, the first morning again.
//!
//! Then it's saved, so the save is the fresh world.

use bevy::prelude::*;

use crate::creatures::player::LocalPlayer;
use crate::creatures::spawn::{PendingSpawn, SpawnQueue, Spawned};
use crate::creatures::{Creature, Kinematics};
use crate::dev::DevAction;

pub struct ResetPlugin;

impl Plugin for ResetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Armed>().init_resource::<Pending>().add_systems(Update, (arm, label, reset.run_if(|p: Res<Pending>| p.0.is_some())).chain());
    }
}

/// What a reset takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reset {
    World,
    Everything,
}

/// Clicked once: this reset, if clicked again before the time's up.
#[derive(Resource, Default)]
struct Armed(Option<(Reset, f32)>);

#[derive(Resource, Default)]
struct Pending(Option<Reset>);

/// A reset button's label (it asks to be sure while armed).
#[derive(Component)]
pub struct ResetLabel(pub Reset);

/// Seconds a first click waits for the second.
const SURE: f32 = 3.0;

fn arm(time: Res<Time<Real>>, mut acts: MessageReader<DevAction>, mut armed: ResMut<Armed>, mut pending: ResMut<Pending>) {
    let now = time.elapsed_secs();
    if armed.0.is_some_and(|(_, until)| now > until) {
        armed.0 = None;
    }
    for a in acts.read() {
        let which = match a {
            DevAction::ResetWorld => Reset::World,
            DevAction::ResetAll => Reset::Everything,
            _ => continue,
        };
        if armed.0.is_some_and(|(w, _)| w == which) {
            armed.0 = None;
            pending.0 = Some(which);
        } else {
            armed.0 = Some((which, now + SURE));
        }
    }
}

fn label(armed: Res<Armed>, mut labels: Query<(&ResetLabel, &mut Text)>) {
    for (l, mut text) in &mut labels {
        let want = match (l.0, armed.0.map(|(w, _)| w)) {
            (Reset::World, Some(Reset::World)) => "Sure? Click again: reset the world",
            (Reset::Everything, Some(Reset::Everything)) => "Sure? Click again: reset everything",
            (Reset::World, _) => "Reset the world (twice)",
            (Reset::Everything, _) => "Reset everything (twice)",
        };
        if text.0 != want {
            text.0 = want.to_string();
        }
    }
}

/// Anything in the world that isn't the land itself.
type WorldThing = Or<(
    (With<Creature>, Without<LocalPlayer>),
    With<crate::hands::chests::Chest>,
    With<crate::craft::Station>,
    With<crate::hands::Dropped>,
    With<crate::hands::corpses::Corpse>,
    With<crate::props::Thrown>,
    With<crate::archery::Arrow>,
    With<crate::light::PlantedTorch>,
)>;

fn reset(world: &mut World) {
    let Some(which) = world.resource_mut::<Pending>().0.take() else { return };
    // The land: forgotten, and let go (it streams back in as the seed made it).
    let chunks = {
        let mut sim = world.resource_mut::<crate::world::SimWorld>();
        let loaded: Vec<_> = sim.world.chunks().map(|c| c.pos).collect();
        for pos in &loaded {
            sim.world.remove_chunk(*pos);
        }
        sim.world.clear_loose();
        let stored = sim.store.len();
        sim.store = Default::default();
        loaded.len() + stored
    };
    // What was in it.
    let things: Vec<Entity> = world.query_filtered::<Entity, WorldThing>().iter(world).collect();
    let n = things.len();
    for e in things {
        world.entity_mut(e).despawn();
    }
    world.resource_mut::<crate::hands::chests::Chests>().forget_all();
    world.resource_mut::<crate::clock::WorldClock>().forget();
    world.resource_mut::<Spawned>().0.clear();
    world.resource_mut::<crate::world::FreshChunks>().0.clear();
    // You: back at the start (or, everything, a new you).
    let start = world.resource::<crate::world::SimWorld>().generator.spawn_point();
    match which {
        Reset::World => {
            let mut q = world.query_filtered::<&mut Kinematics, With<LocalPlayer>>();
            for mut k in q.iter_mut(world) {
                // (Over the start, frozen until the ground under it is back.)
                k.body.pos = Vec2::new(start.x as f32 + 0.5, start.y as f32 + 60.0);
                k.body.vel = Vec2::ZERO;
                k.prev_pos = k.body.pos;
            }
        }
        Reset::Everything => {
            let players: Vec<Entity> = world.query_filtered::<Entity, With<LocalPlayer>>().iter(world).collect();
            for e in players {
                world.entity_mut(e).despawn();
            }
            world.resource_mut::<SpawnQueue>().0.push(PendingSpawn { kind: "player".into(), x: start.x, from_y: start.y + 180, local_player: true });
            world.resource_mut::<crate::world::SimWorld>().world.set_tick(0);
            world.resource_mut::<crate::light::Daylight>().skipped = 0.0;
            world.resource_mut::<crate::creatures::PlayerDeaths>().0 = 0;
        }
    }
    // (The enemies about the start, as a new world has them.)
    world.resource_scope(|world, mut queue: Mut<SpawnQueue>| crate::creatures::spawn::queue_start_enemies(world.resource::<crate::world::SimWorld>(), &mut queue));
    world.write_message(crate::save::SaveNow);
    info!("dev: reset {} ({chunks} chunks let go, {n} things taken away)", if which == Reset::World { "the world" } else { "everything" });
}
