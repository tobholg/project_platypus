//! Progression (DESIGN §7d): what the player has done and found, and the
//! milestones it adds up to (`assets/data/progression.ron`).
//!
//! - `Progress` (on the player, saved with it): every item it has held
//!   (recipes show once you've seen one of what goes in: `craft.rs`), the
//!   deepest it's been, what it has killed and crafted, the milestones it
//!   has reached and the recipes they unlocked.
//! - A milestone is a condition (holding an item, having crafted one, a
//!   depth reached, so many of a creature killed, another milestone; all or
//!   any of several) and what reaching it gives: items, and recipes that
//!   were locked. Reaching one shows a toast.
//!
//! The particulars (which milestones, what they give) are placeholders to
//! build on; the conditions and rewards are the system.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::actors::player::LocalPlayer;
use crate::actors::{Died, Kinematics};
use crate::data::{Watched, data_path, load_ron};
use crate::hands::items::{Inventory, Items, Stack};
use crate::world::SimWorld;

/// A killed creature counts for the player within this many cells of it.
const WITNESS: f32 = 300.0;
/// Seconds between looks at the milestones.
const CHECK_EVERY: f32 = 0.5;
/// Seconds a toast stays up.
const TOAST_SECS: f32 = 4.0;

/// What the player has done and found.
#[derive(Component, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    /// Every item it has held (by id).
    pub seen: BTreeSet<String>,
    /// The deepest it has been (cells below sea level).
    pub deepest: i32,
    /// Creatures killed near it, by kind.
    pub kills: BTreeMap<String, u32>,
    /// Items it has crafted, by id.
    pub crafted: BTreeMap<String, u32>,
    /// Milestones reached (by id), and recipes they unlocked (by the item
    /// they make).
    pub done: BTreeSet<String>,
    pub unlocked: BTreeSet<String>,
}

/// When a milestone is reached.
#[derive(Clone, Debug, Deserialize)]
pub enum When {
    /// Holding (or wearing) an item, now or ever.
    Holds(String),
    /// Having crafted an item.
    Crafted(String),
    /// Having been this many cells below sea level.
    Depth(i32),
    /// So many of a kind of creature killed.
    Killed(String, u32),
    /// Another milestone reached.
    Done(String),
    All(Vec<When>),
    Any(Vec<When>),
}

impl When {
    fn met(&self, p: &Progress) -> bool {
        match self {
            When::Holds(id) => p.seen.contains(id),
            When::Crafted(id) => p.crafted.contains_key(id),
            When::Depth(d) => p.deepest >= *d,
            When::Killed(kind, n) => p.kills.get(kind).copied().unwrap_or(0) >= *n,
            When::Done(id) => p.done.contains(id),
            When::All(all) => all.iter().all(|w| w.met(p)),
            When::Any(any) => any.iter().any(|w| w.met(p)),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub about: String,
    pub when: When,
    /// Items it gives (id, count in the item's units: blocks as blocks).
    #[serde(default)]
    pub gives: Vec<(String, u32)>,
    /// Locked recipes it unlocks (by the item they make).
    #[serde(default)]
    pub unlocks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProgressionFile {
    pub milestones: Vec<Milestone>,
}

#[derive(Resource)]
pub struct Milestones {
    pub list: Vec<Milestone>,
    watch: Watched,
}

/// A line for the player, shown a few seconds at the top of the screen
/// (a milestone reached, "Saved").
#[derive(Message, Clone, Debug)]
pub struct Toast(pub String);

pub struct ProgressPlugin;

impl Plugin for ProgressPlugin {
    fn build(&self, app: &mut App) {
        let path = data_path("progression.ron");
        let file: ProgressionFile = load_ron(&path).unwrap_or_else(|e| panic!("{e}"));
        app.insert_resource(Milestones { list: file.milestones, watch: Watched::new(path) })
            .add_message::<Toast>()
            .add_systems(Startup, spawn_toasts)
            .add_systems(Update, (give_progress, track, reach, reload, show_toasts));
    }
}

fn reload(mut m: ResMut<Milestones>) {
    if !m.bypass_change_detection().watch.changed() {
        return;
    }
    match load_ron::<ProgressionFile>(m.watch.path()) {
        Ok(f) => {
            m.list = f.milestones;
            info!("progression reloaded");
        }
        Err(e) => warn!("progression not reloaded: {e}"),
    }
}

/// A player without a record gets an empty one (a loaded player's own
/// replaces it: `save.rs`).
fn give_progress(mut commands: Commands, new: Query<Entity, (With<LocalPlayer>, Without<Progress>)>) {
    for e in &new {
        commands.entity(e).insert(Progress::default());
    }
}

/// Note what the player holds, how deep it is, what dies near it.
fn track(
    sim: Res<SimWorld>,
    items: Option<Res<Items>>,
    pending: Option<Res<crate::save::PendingPlayer>>,
    mut died: MessageReader<Died>,
    mut player: Query<(&Kinematics, Option<&Inventory>, &crate::gear::Equipment, &mut Progress), With<LocalPlayer>>,
) {
    let (None, Ok((k, inv, eq, mut p))) = (pending, player.single_mut()) else {
        died.clear();
        return;
    };
    let pos = k.body.pos;
    for d in died.read() {
        if !d.kind.is_empty() && d.body.pos.distance(pos) <= WITNESS {
            *p.bypass_change_detection().kills.entry(d.kind.clone()).or_default() += 1;
        }
    }
    let depth = sim.world.climate().sea_level - pos.y as i32;
    if depth > p.deepest {
        p.bypass_change_detection().deepest = depth;
    }
    let Some(items) = items else { return };
    let held = inv.map(|i| i.slots.as_slice()).unwrap_or(&[]).iter().chain(eq.worn.iter()).flatten();
    for s in held {
        let id = &items.def(s.item).id;
        if !p.seen.contains(id) {
            p.bypass_change_detection().seen.insert(id.clone());
        }
    }
}

/// Every so often: any milestone newly met is reached (a toast, its gifts,
/// its unlocks).
#[allow(clippy::too_many_arguments)]
fn reach(
    mut commands: Commands,
    time: Res<Time>,
    mut wait: Local<f32>,
    items: Option<Res<Items>>,
    milestones: Res<Milestones>,
    pending: Option<Res<crate::save::PendingPlayer>>,
    mut toasts: MessageWriter<Toast>,
    mut player: Query<(&Kinematics, &mut Progress, Option<&mut Inventory>), With<LocalPlayer>>,
) {
    *wait -= time.delta_secs();
    if *wait > 0.0 || pending.is_some() {
        return;
    }
    *wait = CHECK_EVERY;
    let (Some(items), Ok((k, mut p, mut inv))) = (items, player.single_mut()) else { return };
    for m in &milestones.list {
        if p.done.contains(&m.id) || !m.when.met(&p) {
            continue;
        }
        p.done.insert(m.id.clone());
        p.unlocked.extend(m.unlocks.iter().cloned());
        let mut line = format!("Milestone: {}", m.name);
        if !m.about.is_empty() {
            line = format!("{line}. {}", m.about);
        }
        for (id, n) in &m.gives {
            let Some(item) = items.id(id) else {
                warn!("progression.ron: `{}` gives no such item `{id}`", m.id);
                continue;
            };
            let stack = Stack::new(item, n * items.unit(item));
            let left = inv.as_mut().map_or(stack.count, |i| i.add(&items, stack));
            if left > 0 {
                crate::hands::spawn_drop(&mut commands, &items, k.body.pos, Stack { count: left, ..stack });
            }
            line = format!("{line}  +{n} {}", items.def(item).name);
        }
        if !m.unlocks.is_empty() {
            line = format!("{line}  (new recipes)");
        }
        info!("progress: {line}");
        toasts.write(Toast(line));
    }
}

#[derive(Component)]
struct ToastRoot;

/// One toast on screen, and how long it has left.
#[derive(Component)]
struct ToastLine(f32);

fn spawn_toasts(mut commands: Commands) {
    commands.spawn((
        ToastRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(60.0),
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(4.0),
            ..default()
        },
    ));
}

fn show_toasts(mut commands: Commands, time: Res<Time>, mut new: MessageReader<Toast>, root: Single<Entity, With<ToastRoot>>, mut lines: Query<(Entity, &mut ToastLine, &mut TextColor, &mut BackgroundColor)>) {
    for t in new.read() {
        commands.entity(*root).with_child((
            ToastLine(TOAST_SECS),
            Text::new(t.0.clone()),
            TextFont { font_size: FontSize::Px(15.0), ..default() },
            TextColor(Color::srgb(1.0, 0.92, 0.6)),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            Node { padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)), ..default() },
        ));
    }
    for (e, mut line, mut color, mut bg) in &mut lines {
        line.0 -= time.delta_secs();
        if line.0 <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        // (Fading out over its last second.)
        let a = line.0.min(1.0);
        color.0 = color.0.with_alpha(a);
        bg.0 = bg.0.with_alpha(0.6 * a);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milestones_read_and_their_conditions_hold() {
        let file: ProgressionFile = crate::data::parse_ron(include_str!("../../../assets/data/progression.ron")).unwrap();
        let items = crate::hands::items::test_items();
        for m in &file.milestones {
            for (id, _) in &m.gives {
                assert!(items.id(id).is_some(), "{} gives no such item `{id}`", m.id);
            }
        }
        let mut p = Progress::default();
        let w = When::All(vec![When::Holds("copper_bar".into()), When::Any(vec![When::Depth(100), When::Killed("orc".into(), 2)])]);
        assert!(!w.met(&p));
        p.seen.insert("copper_bar".into());
        p.kills.insert("orc".into(), 1);
        assert!(!w.met(&p));
        p.kills.insert("orc".into(), 2);
        assert!(w.met(&p));
    }
}
