//! Saving and loading (DESIGN §3.5). A save is a folder (`saves/<name>`):
//!
//! - `world.ron`: the format's version, the world (its kind and seed: the
//!   rest of it is regenerated from them), the tick (so the time of day), the
//!   materials by name in the order the chunks number them, the places whose
//!   spawns have been spawned, and the count of keys made for chests;
//! - `chunks.bin`: every chunk that's been changed (the sim's own bytes,
//!   lz4), loaded or not; the rest regenerate;
//! - `player.ron`: where you are, health and mana, the pack, what you wear,
//!   the hotbar in hand;
//! - `things.ron`: chests (and what's in every chest known, opened or not),
//!   crafting stations, items lying about, creatures; a body's belongings are kept as items
//!   lying where it lay.
//!
//! Items and materials are kept by name, so a save still reads after
//! items.ron or materials.ron change order (a name that's gone is dropped,
//! with a warning). Saved: every minute, on quitting, and with Ctrl+S.
//!
//! `PLATYPUS_SAVE=<name>` picks the save (default: the world and seed, as
//! `large-1`; scenarios and the arena and flat sandboxes save only when
//! it's given), `PLATYPUS_SAVE=off`
//! turns saving off, `PLATYPUS_FRESH=1` starts over (the next save
//! overwrites the old one).

use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use platypus_sim::{CellPos, ChunkPos, MaterialId, store};
use serde::{Deserialize, Serialize};

use crate::creatures::player::LocalPlayer;
use crate::creatures::spawn::Spawned;
use crate::creatures::{Creature, Health, Kinematics};
use crate::gear::Equipment;
use crate::hands::chests::{Chest, Chests};
use crate::hands::corpses::Corpse;
use crate::hands::items::{Inventory, Items, Roll, Stack};
use crate::hands::{Dropped, Hand, spawn_drop};
use crate::magic::Mana;
use crate::world::SimWorld;

/// The save format's version: a save of another is refused (and kept, moved
/// aside: `<name>.v<version>`). 2: the large world became 8× as big
/// (DESIGN §13).
const VERSION: u32 = 2;
/// Seconds between saves while playing.
const AUTOSAVE: f32 = 60.0;
/// What starts `chunks.bin`.
const MAGIC: &[u8; 4] = b"PLCH";

pub struct SavePlugin {
    /// The save's folder name (none: no saving).
    pub name: Option<String>,
    /// The world it's of (as `PLATYPUS_WORLD` names it) and its seed.
    pub kind: String,
    pub seed: u64,
}

/// Save now (Ctrl+S, a scenario).
#[derive(Message, Clone, Copy, Debug)]
pub struct SaveNow;

/// The save in use: its folder, and when it's next saved.
#[derive(Resource)]
pub struct SaveSlot {
    pub dir: PathBuf,
    kind: String,
    seed: u64,
    next: f32,
    /// A save to load, found at startup.
    load: bool,
    /// The last save's files being written (on a thread of its own).
    writing: Option<std::thread::JoinHandle<()>>,
}

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SaveNow>();
        let Some(name) = &self.name else { return };
        let dir = saves_dir().join(name);
        let fresh = std::env::var("PLATYPUS_FRESH").is_ok_and(|v| !v.is_empty());
        let load = !fresh && dir.join("world.ron").exists();
        info!("save: {} ({})", dir.display(), if load { "loading it" } else { "a new one" });
        app.insert_resource(SaveSlot { dir, kind: self.kind.clone(), seed: self.seed, next: AUTOSAVE, load, writing: None })
            .add_systems(PostStartup, load_world)
            .add_systems(Update, (keys, apply_player.after(crate::hands::give_start)))
            .add_systems(Update, settle_hp.after(crate::gear::apply))
            .add_systems(Last, save);
    }
}

/// Where saves go: `PLATYPUS_SAVE_DIR`, or `saves/` beside the assets.
pub fn saves_dir() -> PathBuf {
    std::env::var("PLATYPUS_SAVE_DIR").map(PathBuf::from).unwrap_or_else(|_| crate::data::assets_dir().join("..").join("saves"))
}

/// A stack as saved: its item by name, and its roll.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedStack {
    pub id: String,
    pub count: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rarity: u8,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub level: u8,
    #[serde(default, skip_serializing_if = "is_zero32")]
    pub seed: u32,
    /// A vessel's contents: the liquid by name, and cells.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<(String, u32)>,
}

fn is_zero(v: &u8) -> bool {
    *v == 0
}

fn is_zero32(v: &u32) -> bool {
    *v == 0
}

pub fn stack_out(items: &Items, s: &Stack) -> SavedStack {
    let fill = s.fill.and_then(|(m, n)| items.material_name(m).map(|name| (name.to_string(), n)));
    SavedStack { id: items.def(s.item).id.clone(), count: s.count, rarity: s.roll.rarity, level: s.roll.level, seed: s.roll.seed, fill }
}

/// A saved stack as a stack, if its item still exists.
pub fn stack_in(items: &Items, s: &SavedStack) -> Option<Stack> {
    let Some(item) = items.id(&s.id) else {
        warn!("save: no item `{}` any more; dropped", s.id);
        return None;
    };
    let fill = s.fill.as_ref().and_then(|(name, n)| items.material(name).map(|m| (m, *n)));
    Some(Stack { item, count: s.count, roll: Roll { rarity: s.rarity, level: s.level, seed: s.seed }, fill })
}

fn slots_out(items: &Items, slots: &[Option<Stack>]) -> Vec<Option<SavedStack>> {
    slots.iter().map(|s| s.as_ref().map(|s| stack_out(items, s))).collect()
}

fn slots_in(items: &Items, slots: &[Option<SavedStack>]) -> Vec<Option<Stack>> {
    slots.iter().map(|s| s.as_ref().and_then(|s| stack_in(items, s))).collect()
}

#[derive(Serialize, Deserialize)]
struct WorldFile {
    version: u32,
    kind: String,
    seed: u64,
    tick: u64,
    materials: Vec<String>,
    spawned: Vec<(i32, i32)>,
    keys: u64,
    /// Hours skipped (the dev key's), so the day and the clock carry on.
    #[serde(default)]
    skipped: f32,
    /// The world clock (`clock.rs`).
    #[serde(default)]
    clock: crate::clock::ClockFile,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerFile {
    pos: (f32, f32),
    hp: f32,
    #[serde(default)]
    mana: Option<f32>,
    bar: usize,
    slot: usize,
    pack: Vec<Option<SavedStack>>,
    worn: Vec<Option<SavedStack>>,
    /// What it has done and found (`progress.rs`).
    #[serde(default)]
    pub progress: crate::progress::Progress,
    /// Its gold (`gold.rs`).
    #[serde(default)]
    gold: u32,
}

#[derive(Serialize, Deserialize)]
struct SavedStash {
    key: u64,
    origin: (i32, i32),
    contents: Option<Vec<Option<SavedStack>>>,
}

#[derive(Serialize, Deserialize)]
struct SavedCreature {
    kind: String,
    feet: (f32, f32),
    hp: f32,
    /// A lair's keeper: where it was put (`clock::Keeps`).
    #[serde(default)]
    keeps: Option<(i32, i32)>,
}

#[derive(Serialize, Deserialize, Default)]
struct ThingsFile {
    chests: Vec<(u64, (f32, f32))>,
    /// Crafting stations (by id), where their feet are.
    #[serde(default)]
    stations: Vec<(String, (f32, f32))>,
    stashes: Vec<SavedStash>,
    drops: Vec<((f32, f32), SavedStack)>,
    creatures: Vec<SavedCreature>,
}

fn write_ron<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let text = ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default()).map_err(|e| e.to_string())?;
    write_file(path, text.as_bytes())
}

/// Written beside, then moved over the old one (a crash mid-save leaves
/// the last save whole).
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    std::fs::File::create(&tmp).and_then(|mut f| f.write_all(bytes)).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

fn read_ron<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    crate::data::load_ron(path)
}

/// The changed chunks: count, then each chunk's position, length and bytes.
fn write_chunks(path: &Path, chunks: &[(ChunkPos, std::sync::Arc<[u8]>)]) -> Result<(), String> {
    let mut out = Vec::with_capacity(12 + chunks.iter().map(|(_, b)| b.len() + 12).sum::<usize>());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(chunks.len() as u32).to_le_bytes());
    for (p, b) in chunks {
        out.extend_from_slice(&p.x.to_le_bytes());
        out.extend_from_slice(&p.y.to_le_bytes());
        out.extend_from_slice(&(b.len() as u32).to_le_bytes());
        out.extend_from_slice(b);
    }
    write_file(path, &out)
}

fn read_chunks(path: &Path) -> Result<Vec<(ChunkPos, Vec<u8>)>, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path).and_then(|mut f| f.read_to_end(&mut bytes)).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut at = 0usize;
    let mut take = |n: usize| -> Result<&[u8], String> {
        let s = bytes.get(at..at + n).ok_or("chunks.bin is cut short")?;
        at += n;
        Ok(s)
    };
    if take(4)? != MAGIC {
        return Err("chunks.bin isn't a chunk file".into());
    }
    let word = |b: &[u8]| u32::from_le_bytes(b.try_into().expect("four bytes"));
    let version = word(take(4)?);
    if version != VERSION {
        return Err(format!("chunks.bin is version {version}, this reads {VERSION}"));
    }
    let n = word(take(4)?) as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let x = word(take(4)?) as i32;
        let y = word(take(4)?) as i32;
        let len = word(take(4)?) as usize;
        out.push((ChunkPos::new(x, y), take(len)?.to_vec()));
    }
    Ok(out)
}

/// Ctrl+S saves.
fn keys(keys: Res<ButtonInput<KeyCode>>, mut now: MessageWriter<SaveNow>) {
    if keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight, KeyCode::SuperLeft, KeyCode::SuperRight]) && keys.just_pressed(KeyCode::KeyS) {
        now.write(SaveNow);
    }
}

type SavedPlayer<'a> = (&'a Kinematics, &'a Health, Option<&'a Mana>, Option<&'a Inventory>, &'a Equipment, Option<&'a crate::progress::Progress>, Option<&'a crate::gold::Gold>);
/// (More for `save`, past Bevy's count of parameters.)
type SavedAlso<'w, 's> = (Res<'w, crate::craft::Crafting>, Query<'w, 's, (&'static crate::craft::Station, &'static Kinematics)>, Res<'w, crate::light::Daylight>, Res<'w, crate::clock::WorldClock>);
type SavedCreatures<'a> = (&'a Creature, &'a Kinematics, &'a Health, Option<&'a crate::clock::Keeps>);

/// Save, when it's time, when asked, and on the way out.
#[allow(clippy::too_many_arguments)]
fn save(
    time: Res<Time<Real>>,
    mut slot: ResMut<SaveSlot>,
    mut now: MessageReader<SaveNow>,
    mut exit: MessageReader<AppExit>,
    sim: Res<SimWorld>,
    items: Option<Res<Items>>,
    chests: Res<Chests>,
    spawned: Res<Spawned>,
    hand: Res<Hand>,
    player: Query<SavedPlayer, With<LocalPlayer>>,
    boxes: Query<(&Chest, &Kinematics)>,
    drops: Query<(&Dropped, &Kinematics)>,
    bodies: Query<(&Corpse, &Kinematics)>,
    creatures: Query<SavedCreatures, Without<LocalPlayer>>,
    (crafting, stations, day, clock): SavedAlso,
    mut toasts: MessageWriter<crate::progress::Toast>,
) {
    slot.next -= time.delta_secs();
    let asked = now.read().count() > 0;
    let leaving = exit.read().count() > 0;
    if !(asked || leaving || slot.next <= 0.0) {
        return;
    }
    slot.next = AUTOSAVE;
    let Some(items) = items else { return };
    // (Nothing to save until there's a player, loaded or new.)
    let Ok((k, health, mana, pack, eq, progress, gold)) = player.single() else { return };
    let started = std::time::Instant::now();
    let dir = slot.dir.clone();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return error!("save: {}: {e}", dir.display());
    }
    let world = &sim.world;
    let mats = world.materials();
    let file = WorldFile {
        version: VERSION,
        kind: slot.kind.clone(),
        seed: slot.seed,
        tick: world.tick(),
        materials: (0..mats.len()).map(|i| mats.def(MaterialId(i as u16)).name.clone()).collect(),
        spawned: spawned.0.iter().map(|p| (p.x, p.y)).collect(),
        keys: chests.placed(),
        skipped: day.skipped,
        clock: clock.save(),
    };
    let mut chunks: Vec<(ChunkPos, std::sync::Arc<[u8]>)> = world.chunks().filter(|c| c.is_modified()).map(|c| (c.pos, store::encode(c).into())).collect();
    chunks.extend(sim.store.iter().map(|(p, b)| (p, b.clone())));
    let me = PlayerFile {
        pos: (k.body.pos.x, k.body.pos.y),
        hp: health.hp,
        mana: mana.map(|m| m.cur),
        bar: hand.bar,
        slot: hand.slot,
        pack: pack.map_or_else(Vec::new, |p| slots_out(&items, &p.slots)),
        worn: slots_out(&items, &eq.worn),
        progress: progress.cloned().unwrap_or_default(),
        gold: gold.map_or(0, |g| g.0),
    };
    let mut things = ThingsFile {
        chests: boxes.iter().map(|(c, k)| (c.key, (k.body.pos.x, k.body.pos.y - k.body.half.y))).collect(),
        stations: stations.iter().filter_map(|(s, k)| Some((crafting.stations.get(s.kind)?.id.clone(), (k.body.pos.x, k.body.pos.y - k.body.half.y)))).collect(),
        stashes: chests
            .stashes()
            // (A body's belongings are kept as items where it lies.)
            .filter(|(key, ..)| !bodies.iter().any(|(b, _)| b.key == *key))
            .map(|(key, origin, contents)| SavedStash { key, origin: (origin.x, origin.y), contents: contents.map(|i| slots_out(&items, &i.slots)) })
            .collect(),
        drops: drops.iter().map(|(d, k)| ((k.body.pos.x, k.body.pos.y), stack_out(&items, &d.stack))).collect(),
        creatures: creatures.iter().map(|(c, k, h, keeps)| SavedCreature { kind: c.kind.clone(), feet: (k.body.pos.x, k.body.pos.y - k.body.half.y), hp: h.hp, keeps: keeps.map(|k| k.0) }).collect(),
    };
    for (b, k) in &bodies {
        if let Some(inv) = chests.inventory(b.key) {
            things.drops.extend(inv.slots.iter().flatten().map(|s| ((k.body.pos.x, k.body.pos.y), stack_out(&items, s))));
        }
    }
    // The files written on a thread of their own: the frame pays only for
    // gathering what's saved (writing it all froze the game a tenth of a
    // second or more every minute). The last save's writing finishes
    // first; leaving, it's written before the game goes.
    if let Some(last) = slot.writing.take() {
        let _ = last.join();
    }
    let gathered = started.elapsed().as_secs_f32() * 1e3;
    let write = move || {
        let result = write_ron(&dir.join("world.ron"), &file)
            .and_then(|_| write_chunks(&dir.join("chunks.bin"), &chunks))
            .and_then(|_| write_ron(&dir.join("player.ron"), &me))
            .and_then(|_| write_ron(&dir.join("things.ron"), &things));
        match result {
            Ok(()) => info!("save: saved {} chunks, {} chests, {} creatures in {:.1} ms ({gathered:.1} ms in the frame)", chunks.len(), things.chests.len(), things.creatures.len(), started.elapsed().as_secs_f32() * 1e3),
            Err(e) => error!("save: {e}"),
        }
    };
    if leaving {
        write();
    } else {
        slot.writing = Some(std::thread::spawn(write));
    }
    if asked {
        toasts.write(crate::progress::Toast("Saved".into()));
    }
}

/// The player as saved, put back once it exists (after its start kit).
/// (While it waits, progress isn't tracked: the start kit isn't the save.)
#[derive(Resource)]
pub(crate) struct PendingPlayer(PlayerFile);

/// A loaded player's health, put back once its gear has been added up.
#[derive(Component)]
struct PendingHp(f32);

/// Read a save, if there is one: the world's changed chunks go where
/// unloaded chunks wait to load, the tick is set, chests, items and
/// creatures come back, and the player waits to be put back.
#[allow(clippy::too_many_arguments)]
fn load_world(
    mut commands: Commands,
    slot: Res<SaveSlot>,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<Items>>,
    mut chests: ResMut<Chests>,
    mut spawned: ResMut<Spawned>,
    crafting: Res<crate::craft::Crafting>,
    (mut day, mut clock): (ResMut<crate::light::Daylight>, ResMut<crate::clock::WorldClock>),
) {
    if !slot.load {
        return;
    }
    let dir = &slot.dir;
    let file: WorldFile = match read_ron(&dir.join("world.ron")) {
        Ok(f) => f,
        Err(e) => return error!("save: {e}; starting a new world"),
    };
    if file.version != VERSION || file.kind != slot.kind || file.seed != slot.seed {
        // (Never lost: moved aside, and a new world begun in its place.)
        let aside = (0..).map(|n| dir.with_extension(if n == 0 { format!("v{}", file.version) } else { format!("v{}-{n}", file.version) })).find(|p| !p.exists()).expect("a free name");
        let kept = std::fs::rename(dir, &aside).map_or_else(|e| format!("could not move it aside: {e}"), |_| format!("kept as {}", aside.display()));
        return error!("save: {} is of {} (seed {}, version {}), not {} (seed {}, version {VERSION}); {kept}; starting a new world", dir.display(), file.kind, file.seed, file.version, slot.kind, slot.seed);
    }
    // Materials by name: a save made with them in another order still reads.
    let mats = sim.world.materials().clone();
    let map: Vec<MaterialId> = file
        .materials
        .iter()
        .map(|n| {
            mats.id(n).unwrap_or_else(|| {
                warn!("save: no material `{n}` any more; it's air now");
                MaterialId::AIR
            })
        })
        .collect();
    let same = map.iter().enumerate().all(|(i, m)| m.0 as usize == i);
    let chunks = match read_chunks(&dir.join("chunks.bin")) {
        Ok(c) => c,
        Err(e) => return error!("save: {e}; starting a new world"),
    };
    let n = chunks.len();
    for (pos, bytes) in chunks {
        match store::decode_remapped(pos, &bytes, (!same).then_some(map.as_slice())) {
            Ok(chunk) => sim.store.put(&chunk),
            Err(e) => warn!("save: {e}"),
        }
    }
    sim.world.set_tick(file.tick);
    day.skipped = file.skipped;
    clock.load(&file.clock);
    spawned.0 = file.spawned.iter().map(|&(x, y)| CellPos::new(x, y)).collect::<HashSet<_>>();
    let Some(items) = items else { return error!("save: items aren't loaded") };
    let things: ThingsFile = read_ron(&dir.join("things.ron")).unwrap_or_else(|e| {
        warn!("save: {e}");
        ThingsFile::default()
    });
    let stashes = things
        .stashes
        .iter()
        .map(|s| (s.key, CellPos::new(s.origin.0, s.origin.1), s.contents.as_ref().map(|c| Inventory { slots: slots_in(&items, c) })))
        .collect();
    chests.restore(stashes, file.keys);
    for &(key, (x, y)) in &things.chests {
        chests.respawn(&mut commands, key, Vec2::new(x, y));
    }
    for (id, (x, y)) in &things.stations {
        match crafting.station(id) {
            Some(kind) => crafting.spawn(&mut commands, kind, Vec2::new(*x, *y)),
            None => warn!("save: no station `{id}` any more"),
        }
    }
    for ((x, y), s) in &things.drops {
        if let Some(stack) = stack_in(&items, s) {
            spawn_drop(&mut commands, &items, Vec2::new(*x, *y), stack);
        }
    }
    for c in &things.creatures {
        let (hp, keeps) = (c.hp, c.keeps);
        crate::creatures::def::spawn_creature(&mut commands, &c.kind, Vec2::new(c.feet.0, c.feet.1), move |e| {
            if let Some(mut h) = e.get_mut::<Health>() {
                h.hp = hp.min(h.max);
            }
            if let Some(k) = keeps {
                e.insert(crate::clock::Keeps(k));
            }
        });
    }
    match read_ron::<PlayerFile>(&dir.join("player.ron")) {
        Ok(p) => commands.insert_resource(PendingPlayer(p)),
        Err(e) => warn!("save: {e}; a new player"),
    }
    info!("save: loaded {n} chunks, {} chests, {} creatures, tick {}", things.chests.len(), things.creatures.len(), file.tick);
}

/// Put the saved player back, once it's there with its pack and mana.
type Restored<'a> = (Entity, &'a mut Kinematics, Option<&'a mut Mana>, Option<&'a mut Inventory>, &'a mut Equipment);

fn apply_player(mut commands: Commands, pending: Option<Res<PendingPlayer>>, items: Option<Res<Items>>, mut hand: ResMut<Hand>, mut player: Query<Restored, With<LocalPlayer>>) {
    let (Some(pending), Some(items)) = (pending, items) else { return };
    let Ok((e, mut k, mana, pack, mut eq)) = player.single_mut() else { return };
    let (Some(mut mana), Some(mut pack)) = (mana, pack) else { return };
    let p = &pending.0;
    k.body.pos = Vec2::new(p.pos.0, p.pos.1);
    k.body.vel = Vec2::ZERO;
    k.prev_pos = k.body.pos;
    if let Some(m) = p.mana {
        mana.cur = m.min(mana.max);
    }
    let slots = slots_in(&items, &p.pack);
    pack.slots = (0..pack.slots.len().max(slots.len())).map(|i| slots.get(i).copied().flatten()).collect();
    let worn = slots_in(&items, &p.worn);
    for (i, w) in eq.worn.iter_mut().enumerate() {
        *w = worn.get(i).copied().flatten();
    }
    hand.bar = p.bar;
    hand.slot = p.slot;
    commands.entity(e).insert((PendingHp(p.hp), p.progress.clone(), crate::gold::Gold(p.gold)));
    commands.remove_resource::<PendingPlayer>();
    info!("save: the player is back at ({:.0}, {:.0})", p.pos.0, p.pos.1);
}

fn settle_hp(mut commands: Commands, mut q: Query<(Entity, &mut Health, &PendingHp)>) {
    for (e, mut h, p) in &mut q {
        h.hp = p.0.min(h.max);
        commands.entity(e).remove::<PendingHp>();
    }
}
