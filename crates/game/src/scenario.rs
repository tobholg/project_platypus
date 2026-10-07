//! Scripted runs for measuring the real game, not just the headless sim.
//!
//! `PLATYPUS_SCENARIO=pan cargo run -p platypus --release`
//!
//! - `idle`       camera still
//! - `pan`        camera sweeps right at 225 cells/s (legacy "walking" was 83 tiles/s at 3 px)
//! - `avalanche`  big blobs of sand, water, lava and oil every half second, slow pan
//! - `run`        the player runs right through real input: holds D, jumps, dashes
//! - `tools`      scripted cursor: pickaxe, bomb, pour water and oil, ignite, melt rock
//! - `tree`       builds a wooden tree beside the player and sets it on fire
//! - `blast`      three bombs dropped down one shaft beside the player, from t = 2 s
//! - `fell`       cuts through the trunk of the nearest tree to the right at t = 2 s
//! - `burn`       sets the base of that tree alight at t = 2 s instead
//! - `strike`     lightning onto that tree at t = 2 s instead
//! - `acid`       pours acid into a glass basin beside the player, boils it at 2 s, lights the fumes at 3.6 s
//! - `rain`       lights the forest beside the player at 2 s, a storm over it at 3 s (F5), lightning at 6 s (F7)
//! - `swim`       a pool beside the player; oil on the player, set alight, then it walks into the water
//! - `night`      the surface at 23:00 (`dusk`: 18:15)
//! - `flood`      a big block of water released in a dug-out hall beside the player
//! - `cave`       a chamber dug under the player (lava and acid pools), flashlight and torch on,
//!   a torch planted, two glow sticks thrown (`PLATYPUS_NOBEAM=1`: no flashlight)
//! - `chest`      places a chest with real input, opens it (RMB), puts bombs in, closes it,
//!   mines it: the bombs and the chest come back
//! - `hands`      the hands through real input: digs down and sideways with the pickaxe,
//!   builds a wall with what it dug, chops at a tree with auto tool (Ctrl),
//!   plants a torch; logs the inventory
//! - `chestfall`  a chest beside the player at 1 s, the ground under it dug
//!   out at 2 s: it falls; a blast beside it at 4 s throws it, one on it at 5 s
//!   breaks it
//! - `zoom`       types + twice and − once (as characters, as a Norwegian
//!   keyboard would), then scrolls the wheel two notches down; logs the zoom
//!   and the hotbar slot
//! - `shroom`     cuts through the stem of the nearest giant mushroom at 2 s
//!   (spawn in a fungal hollow): it falls as a body; logs the bodies
//! - `drop`       stands still until 3 s, then holds S: on a platform (e.g. a
//!   crypt's entrance, `PLATYPUS_SPAWN_X` at a ruin) it drops through; logs
//!   the feet before and after
//! - `magic`      three orcs well to the right at 1 s; then each starter wand
//!   through real input, held: sparks at the nearest orc (1.5 s), a fireball
//!   at the ground ahead (3 s), acid lobbed up and over (4.5 s), the flame
//!   wand at the orcs (5.5 s), lightning at them (7 s); logs mana, the orcs'
//!   health, spells in flight, blasts and zaps each second
//! - `shock`      a pool dug beside the player (flat world), two orcs in its
//!   far end at 3.5 s, lightning at them at 3.9 s: logs the zap and their
//!   health
//! - `well`       (flat world) two orcs to the right at 1 s; the gravity wand
//!   (hotbar 2) held on the ground ahead from 1.5 s, lifting it; swept over
//!   the orcs (2.6 s: it can carry two), carried up (3 s), swung hard up
//!   and right and let go mid-swing (3.6–3.75 s): thrown; logs what it
//!   holds, whom it carries, mana and the orcs (fall damage when they land)
//! - `force`      (flat world) a sand pile and two orcs to the right at 1 s;
//!   the force wand (hotbar 2) pushing toward them from 1.5 s, then pulling
//!   (right button) from 3 s, then pushing straight down (5–6 s: the
//!   recoil lifts the player); logs the orcs' distance, the player's height
//!   and mana
//! - `wellwater`  (flat world) a pool dug and filled beside the player; the
//!   gravity wand lifts it and lets go, twice; logs all the water there is
//!   (in cells, in flight, held) each half second: it should stay the same
//! - `splash`     (flat world) a pool beside the player, a pit of oil past
//!   it; a fireball lobbed into it (2 s: doused, a steam blast),
//!   one fired flat across it (3.5 s: skips, then doused), a spark bolt and
//!   an acid arrow into it (5, 6 s: plunge), a fireball lobbed onto the oil
//!   (7 s: alight), frost onto the pool (8.5 s: ice); logs water, steam,
//!   ice, oil and burning (`RUST_LOG=warn,platypus::magic=debug` traces the
//!   spells)
//! - `airjump`    (in cloud boots) lifts the player 240 cells at 1 s; it falls and air jumps
//!   (a cloud) just above the ground: no fall damage (`PLATYPUS_NOSAVE=1`:
//!   no air jump, it hurts; `PLATYPUS_NIGHT=1`: at night); logs height and
//!   health
//! - `critters`   (flat world) a rabbit, a bird and a frog placed 90–135 cells
//!   to the right at 1 s; the player walks at them from 2 s: they should hop
//!   and fly away; logs where the critters near the player are
//! - `editor`     (`PLATYPUS_WORLD=arena`, `PLATYPUS_EDIT_DIR` = a folder
//!   with copies of sprites: it WRITES them) opens the art editor, paints
//!   three pixels (9, 14–16) of the first thing in the first file in one stroke with
//!   the real pointer (a pose paints its first layer's part), logs what
//!   changed on disk; Ctrl+Z; logs whether the file is back as it was
//! - `webs`       (`PLATYPUS_WORLD=arena`) a walk right through open air, then
//!   through a thicket of cobweb; logs how far each got in 1.5 s
//! - `nest`       (a generated world, `small` is quickest) the player put in
//!   the spider nest nearest the start, a torch in hand; logs what's about
//!   after 3 s
//! - `underground` (`PLATYPUS_WORLD=arena`) each underground enemy in turn
//!   against the player (standing still): a spider, two slimes, two vampire
//!   bats, a skeleton, an egg sac; logs the health each phase cost and what
//!   they did; then the player on top of a column and a spider at its foot
//!   (logs how high it climbed)
//! - `life`       (`PLATYPUS_WORLD=arena`; try `PLATYPUS_HOUR=22`) fireflies
//!   over the floor, fish in the pool, bats in the air above; logs
//!   after 5 s whether each is still where it lives
//! - `crossing`   (`PLATYPUS_WORLD=arena`) a stream of sand poured 45 cells
//!   ahead, walked through (logs how far the player got); then a pit dug
//!   with water 18 cells below its rim, the player put in it, swimming up
//!   and jumping for the bank (logs whether it got out)
//! - `held`       (`PLATYPUS_WORLD=arena`) each tool in the hand in turn: the
//!   pickaxe into the floor (logs cells dug and swings), the torch, the
//!   spark wand at the first dummy, a bomb thrown, the axe swung
//! - `warband`    (`PLATYPUS_WORLD=arena`) O (the dev action) 180 cells off at
//!   1 s; logs what stands there at 3 s (a troll, three orcs, two archers)
//! - `archery`    (`PLATYPUS_WORLD=arena`) the bow (hotbar 2, slot 9): a full
//!   draw at the first dummy, a short one into the floor, one down through a
//!   lava puddle put behind (logs whether it burns); walks over the stuck arrows (logs arrows before and
//!   after), then an orc archer put 165 cells off shoots back (logs hp)
//! - `fight`      (`PLATYPUS_WORLD=arena`) the shortsword against an orc put
//!   40 cells off, held at it from 1.5 s; a troll put 70 cells off at 5 s;
//!   logs both sides' health, swings and staggers twice a second
//! - `melee`      (`PLATYPUS_WORLD=arena`) the shortsword (hotbar 2, slot 7)
//!   held down at the first dummy for 1.5 s, then the longsword (slot 8):
//!   logs hits, damage and stamina; a jump over the dummy striking down
//!   (logs the pogo); a blast on the player mid-dodge and one without (logs
//!   what each cost)
//! - `wands`      (`PLATYPUS_WORLD=arena`) the spark wand into the floor (logs
//!   the cells it broke) and at a sandbag put 60 cells off (logs how far
//!   it went), the
//!   flame wand at the dummies 90 and 180 cells off (logs what each took)
//! - `arena`      (`PLATYPUS_WORLD=arena`) overlays on; the spark wand at the
//!   first dummy, a fireball at the second; an orc picked and spawned with O;
//!   paused at 5 s and stepped three times (logs the sim ticks: 3), then a
//!   quarter speed from 6 s (logs ticks a second: ~15); logs the dummies'
//!   readouts
//! - `inventory`  opens the inventory screen (Esc) at 1 s, switches to the
//!   second hotbar (X) at 1.5 s, hovers the spark wand at 2 s (its tooltip;
//!   this moves the real mouse pointer), drags it to hotbar 3 at 2.6–3 s;
//!   logs where it ended up
//! - `gear`       (`PLATYPUS_WORLD=arena`) a blast beside the player with
//!   nothing on (logs what it cost), then a full iron set and a ring put on
//!   (logs its stats) and the same blast (logs what it cost now); the
//!   inventory opened, a leather jerkin in the pack hovered (its tooltip,
//!   against the chainmail worn) and picked up (the body slot lights up);
//!   an orc in leather and a skeleton in cloth
//!   stand by (brains off), to see gear on other humanoids
//! - `loot`       (`PLATYPUS_WORLD=arena`) an orc in a rare helm and a jerkin
//!   put beside the player (brain off) and struck dead at 1 s; logs its
//!   body and what's in it; the body right-clicked at 2 s (logs what
//!   opened)
//! - `fang`       (`PLATYPUS_WORLD=arena`) the Broodmother's Fang in hand
//!   (hotbar slot 1) against an orc put beside the player (brain off) from
//!   1 s; logs its health and whether it's envenomed (coated in acid)
//! - `foci`       (`PLATYPUS_WORLD=arena`, try `PLATYPUS_HOUR=22`) holds a
//!   focus to look at its aura: `PLATYPUS_SLOT` = hotbar slot 1–10, plus 10
//!   for the second hotbar (13: the gravity staff); held out from 1.5 s
//!   (`PLATYPUS_CAST=1`: casting, at the empty air ahead)
//! - `ice`        (`PLATYPUS_WORLD=arena`) the frost wand (hotbar 2, slot 6)
//!   across the pool's surface through real input (logs the ice it makes);
//!   a run onto the ice and let go (logs how far it slid, against the same
//!   on stone, and that walking on it didn't chill); a frost bolt at an orc
//!   put on the ice (logs that it's chilled)
//! - `chaos`      (`PLATYPUS_WORLD=arena`; a stress test) the player stands
//!   in the middle, untouchable, casting a salvo of spells every tick
//!   (fireball, lightning, acid, sparks, frost) at the crowd; every 3 s a
//!   bigger wave of mixed enemies (`PLATYPUS_CHAOS` a wave, times the wave's
//!   number; default 8), bombs, blobs of sand, water and lava dropped from
//!   the sky, a lightning strike; logs each second what's about (creatures,
//!   bodies, spells, drops, arrows, sparks, particles). With
//!   `--features spikes` and `PLATYPUS_PROFILE=1`: where the time goes.
//! - `voidlook`   the underground's void as you'd meet it: a natural cave
//!   `PLATYPUS_DEPTH` cells down (600), or in a zone (`PLATYPUS_ZONE` =
//!   fungal, crystal, toxic), the player on its floor holding a torch, a
//!   patch of back wall beside it axed away (logs where, and how much void
//!   shows); `PLATYPUS_NOTORCH=1` for the lantern alone, `PLATYPUS_SPAWN=`
//!   kinds (`spider,bat`) put out in the dark on either side
//! - `potion`     (`PLATYPUS_WORLD=arena`) health down to 22 (the edges
//!   pulse red), a blow at 2.5 s (a red flash), H at 4 s (a potion drunk:
//!   green, shrinking as it heals; logs the health 2 s and 4 s after), H
//!   again at 8.2 s (sick: not drunk; logs it), then the inventory opened
//! - `rocketswim` (`PLATYPUS_WORLD=arena`) a pit of water 240 deep, the
//!   player in it (far over the bottom) with its rocket boots empty: logs the charge
//!   after 3 s in the water, then how far a held jump rose it in 1.5 s
//!   (and whether the boots flamed: they shouldn't, under water)
//! - `spiderdeath` (`PLATYPUS_WORLD=arena`, try `PLATYPUS_HOUR=22`) a cave
//!   spider beside the player, struck twice by a longsword's blow at 1 s
//!   and 1.1 s (logs how fast each sends it: shrugged off, then
//!   staggered; it bleeds acid: logs the acid about it), killed at 1.5 s: its body keeps its legs, curled
//!   (`PLATYPUS_SCENARIO_SECS=3` for a screenshot)
//! - `forestfire` (a generated world) the land's wetness pinned
//!   (`PLATYPUS_WET`, 0..1, default 0.05: tinder-dry), then the nearest tree
//!   crown to the start set alight at 1 s; logs how much of the forest
//!   within 375 cells (leaves, needles, wood) is left after 25 s
//! - `reset`      (a generated world) a big hole dug where the player
//!   stands and an orc beside it; at 2 s the dev panel's "reset the world"
//!   clicked twice (`PLATYPUS_RESET=all`: "reset everything"); logs, before
//!   and after, whether the hole's there, the creatures about and where
//!   the player is
//! - `camplook`   (a generated world) the nearest miners' camp the world
//!   made (a mine cart, TNT, dynamite, a lantern), the player beside it with
//!   a torch; logs where
//! - `camp`       (`PLATYPUS_WORLD=arena`) a miners' camp 225 cells off (a
//!   lantern, a mine cart loaded with TNT, a TNT barrel, dynamite); at 2 s a
//!   spark lands on the cart (`PLATYPUS_CAMP=hit`: a blow instead, at once);
//!   logs what's left and the player's health at 1.5 s and every 0.8 s
//!   after 5 s (in the open, the rubble raining down hurts: take cover;
//!   `PLATYPUS_SCENARIO_SECS=4.4` for a screenshot as it goes off)
//! - `pickarea`   (`PLATYPUS_WORLD=arena`) the pickaxe's area mode: a dirt
//!   wall 18 wide beside the player and more dirt past a gap behind it; C
//!   pressed (the label says Area), the cursor aimed past the wall at the far
//!   dirt; the button held 1.2 s from 1.4 s (screenshot at 1.2 s with
//!   `PLATYPUS_SCENARIO_SECS=2.2`: the lit bite on the wall's face); logs
//!   what was dug from the wall and from the far dirt (only once it's
//!   through the wall: it can't reach through); then, on 75 cells of dirt, it digs straight down for 3 s
//!   (logs how far it dropped: it should fall down the shaft as it digs)
//! - `soak`       (`PLATYPUS_WORLD=arena`) what fluids leave on you: a step
//!   into a three-cell acid puddle (logs how much acid's on you and your
//!   health), out of it 1.4 s (it keeps eating), then set alight standing
//!   in a three-cell water puddle (a little wet, not put out: logs it at
//!   4.4 s; `PLATYPUS_SCENARIO_SECS=5.3` for a screenshot of it), then into
//!   deep water (the acid washed off, the fire out)
//! - `rocket`     (`PLATYPUS_WORLD=arena`, try `PLATYPUS_HOUR=22`) the player
//!   in rocket boots over an orc on a wooden floor (brain off), holding
//!   jump from 1 s: logs how high it gets, the fuel, what the exhaust does
//!   to the orc and the floor (fire, burning planks), and that landing
//!   refills it
//! - `spider`     (`PLATYPUS_WORLD=arena`) a cave spider put 180 cells from the
//!   player (standing still, healed each second): logs each attack it
//!   starts (spit, bite, sting), each hit the player takes, and venom
//!
//! Prints one line per second and a summary, then exits.
//! `PLATYPUS_SCREENSHOT=out.png` saves the window one second before the end
//! (`PLATYPUS_OFFSCREEN=1`: what the camera draws, without the window: works
//! with the screen locked).

use bevy::input::InputSystems;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use platypus_physics::MovementStats;
use platypus_sim::{CellPos, WorldEdit};

use crate::creatures::Kinematics;
use crate::creatures::player::LocalPlayer;
use crate::creatures::spawn::find_ground;
use crate::camera::{CursorOverride, MainCamera};
use crate::props::spawn_bomb;
use crate::tools::{Toolbelt, ToolsConfig};
use crate::debug::FrameStats;
use crate::world::{SimMetrics, SimWorld};

pub struct ScenarioPlugin;

#[derive(Resource)]
struct Scenario {
    name: String,
    elapsed: f32,
    duration: f32,
    next_report: f32,
    next_drop: f32,
    reports: Vec<(f32, f32, f32)>,
    screenshot: Option<String>,
    /// Frames being captured (`PLATYPUS_FRAMES`): the path, which, how many.
    frames: Option<(String, u32, u32)>,
}

type PlayerNotCamera = (With<LocalPlayer>, Without<MainCamera>);

/// The frame being captured (1-based; 0: none), for `frame_cams`.
static FRAME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Where the camera is for each captured frame (after it followed); with
/// `PLATYPUS_CAMLOG=1`, every frame (the time, the camera, the player).
fn frame_cams(time: Res<Time>, cam: Single<&Transform, With<MainCamera>>, player: Query<(&Kinematics, &Transform), PlayerNotCamera>, mut log: Local<Option<bool>>) {
    let i = FRAME.load(std::sync::atomic::Ordering::Relaxed);
    if i > 0 {
        println!("FRAME {:03} cam {} {}", i - 1, cam.translation.x, cam.translation.y);
    }
    if *log.get_or_insert_with(|| std::env::var("PLATYPUS_CAMLOG").is_ok())
        && let Ok((k, tf)) = player.single()
    {
        println!("CAM {} {} {} {} {} {}", time.elapsed_secs(), cam.translation.x, cam.translation.y, k.body.pos.y, k.loco.grounded(), tf.translation.y);
    }
}

impl Plugin for ScenarioPlugin {
    fn build(&self, app: &mut App) {
        let Ok(name) = std::env::var("PLATYPUS_SCENARIO") else { return };
        // (`sneak` is played: an hour.)
        let duration = std::env::var("PLATYPUS_SCENARIO_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(if name == "sneak" { 3600.0 } else { 20.0 });
        app.insert_resource(Scenario { name, elapsed: 0.0, duration, next_report: 2.0, next_drop: 1.0, reports: Vec::new(), screenshot: std::env::var("PLATYPUS_SCREENSHOT").ok(), frames: None })
            .add_systems(Update, run)
            .add_systems(PostUpdate, frame_cams.after(crate::camera::follow))
            // Inject input where real input arrives: after Bevy reads devices,
            // before anything reads the cursor or buttons.
            .add_systems(PreUpdate, tools_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, editor_script.after(InputSystems).before(crate::editor::capture))
            .add_systems(Update, (warband_script, life_script, underground_script, nest_script))
            .add_systems(PreUpdate, webs_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, crossing_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, held_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, gear_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, loot_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, fang_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, foci_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, ice_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, chaos_script)
            .add_systems(Update, (camp_script, spiderdeath_script, camplook_script, reset_script, forestfire_script, regrow_script, wildfire_script, refill_script, gold_script, goldheap_script, kick_script, boulder_script))
            .add_systems(PreUpdate, (logmagic_script, cast_script, trap_script, village_script, shop_script, climb_script, star_script, raid_script, quake_script, pedlar_script, troll_script, reactions_script, grab_script, bestiary_script, reload_script, layouts_script, record_script, course_script, dig_script, legs_script).after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, (rocket_script, rocketswim_script, soak_script, pickarea_script).after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, spider_script)
            .add_systems(PreUpdate, hook_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, pogo_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, backwall_script)
            .add_systems(Update, (overhang_script, shots_script, parts_script, safari_script))
            .add_systems(PreUpdate, stealth_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, sneak_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, sounds_script)
            .add_systems(Update, backdrop_script)
            .add_systems(Update, (underlook_script, voidlook_script))
            .add_systems(PreUpdate, potion_script.after(InputSystems))
            .add_systems(PreUpdate, surface_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, walk_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, tempo_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, beams_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, conjure_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, call_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, void_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, fall_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, save_script)
            .add_systems(PreUpdate, craft_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, (tree_script, blast_script, fell_script, acid_script, rain_script, swim_script, dark_script, flood_script))
            .add_systems(PreUpdate, (hands_script, chest_script, drop_script, chestfall_script, zoom_script, shroom_script, magic_script, shock_script, inventory_script, well_script, force_script, wellwater_script, splash_script, airjump_script, critters_script, arena_script, wands_script, melee_script, fight_script, archery_script).after(InputSystems).before(crate::camera::track_cursor));
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut s: ResMut<Scenario>,
    stats: Res<FrameStats>,
    metrics: Res<SimMetrics>,
    mut sim: ResMut<SimWorld>,
    mut cam: Single<&mut Transform, With<MainCamera>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
    offscreen: Option<Res<crate::camera::Offscreen>>,
) {
    let dt = time.delta_secs().min(0.1);
    s.elapsed += dt;
    let pan = match s.name.as_str() {
        "pan" => 225.0,
        "avalanche" => 30.0,
        _ => 0.0,
    };
    cam.translation.x += pan * dt;

    if s.name == "run" {
        // Real key presses through the input system, not a shortcut into the player.
        keys.press(KeyCode::KeyD);
        let phase = s.elapsed % 1.4;
        if phase < 0.35 { keys.press(KeyCode::Space) } else { keys.release(KeyCode::Space) }
        let dash = s.elapsed % 2.3;
        if (0.5..0.55).contains(&dash) { keys.press(KeyCode::ShiftLeft) } else { keys.release(KeyCode::ShiftLeft) }
    }

    if s.name == "avalanche" && s.elapsed >= s.next_drop {
        s.next_drop += 0.5;
        let names = ["sand", "water", "lava", "oil", "gravel"];
        let k = (s.elapsed * 2.0) as usize;
        for i in 0..4 {
            let name = names[(k + i) % names.len()];
            let Some(material) = sim.materials().id(name) else { continue };
            let center = CellPos::new((cam.translation.x - 360.0 + i as f32 * 240.0) as i32, (cam.translation.y + 180.0) as i32);
            sim.queue(WorldEdit::Paint { center, radius: 33, material, overwrite: false });
        }
    }

    // The first two seconds are startup (initial chunk loads); skip them.
    if s.elapsed >= s.next_report {
        s.next_report += 1.0;
        let tick_ms = metrics.tick_time_avg.as_secs_f32() * 1e3;
        s.reports.push((stats.avg_ms, stats.worst_ms, tick_ms));
        println!(
            "scenario={} t={:>4.1}s frame_avg={:.2}ms frame_worst={:.2}ms sim_tick={:.3}ms active_chunks={} loaded={} bodies={}",
            s.name,
            s.elapsed,
            stats.avg_ms,
            stats.worst_ms,
            tick_ms,
            metrics.last.active_chunks,
            sim.world.loaded_count(),
            sim.world.bodies().len()
        );
    }
    if s.elapsed >= s.duration - 1.0
        && let Some(path) = s.screenshot.take()
    {
        let shot = match &offscreen {
            Some(o) => Screenshot::image(o.0.clone()),
            None => Screenshot::primary_window(),
        };
        commands.spawn(shot).observe(save_to_disk(path.clone()));
        // PLATYPUS_FRAMES=n: the next n frames too, as <path>.<i>.png (to
        // look for flicker frame to frame; `frame_cams` logs where the
        // camera was for each).
        let n = std::env::var("PLATYPUS_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        if n > 0 {
            s.frames = Some((path, 0, n));
        }
    }
    if let Some((path, i, n)) = s.frames.clone() {
        let shot = match &offscreen {
            Some(o) => Screenshot::image(o.0.clone()),
            None => Screenshot::primary_window(),
        };
        commands.spawn(shot).observe(save_to_disk(format!("{path}.{i:03}.png")));
        s.frames = (i + 1 < n).then_some((path, i + 1, n));
        FRAME.store(i + 1, std::sync::atomic::Ordering::Relaxed);
    } else {
        FRAME.store(0, std::sync::atomic::Ordering::Relaxed);
    }
    if s.elapsed >= s.duration {
        let n = s.reports.len().max(1) as f32;
        let avg = s.reports.iter().map(|r| r.0).sum::<f32>() / n;
        let worst = s.reports.iter().map(|r| r.1).fold(0.0, f32::max);
        let tick = s.reports.iter().map(|r| r.2).sum::<f32>() / n;
        println!("SUMMARY scenario={} frame_avg={avg:.2}ms frame_worst={worst:.2}ms sim_tick_avg={tick:.3}ms", s.name);
        exit.write(AppExit::Success);
    }
}

/// Drives the toolbelt like a player would: real mouse buttons and number keys,
/// with the cursor pointed by script.
#[allow(clippy::too_many_arguments)]
fn tools_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut belt: ResMut<Toolbelt>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut started: Local<Option<f32>>,
    mut dev: ResMut<crate::hands::DevTools>,
) {
    if s.name != "tools" {
        return;
    }
    dev.0 = true;
    let Ok(p) = player.single() else { return };
    let t0 = *started.get_or_insert(s.elapsed);
    let t = s.elapsed - t0;
    // Aim relative to the ground surface dx cells away (dy > 0 above it, < 0 into it).
    let at = |dx: f32, dy: f32| {
        let x = p.body.pos.x + dx;
        let ground = find_ground(&sim.world, x as i32, p.body.pos.y as i32 + 90, 450)? as f32;
        Some(Vec2::new(x, ground + dy))
    };
    let hold = |mouse: &mut ButtonInput<MouseButton>, on: bool| {
        if on { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    };
    for k in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit6] {
        keys.release(k);
    }
    match t {
        t if t < 0.5 => {}
        t if t < 2.5 => {
            keys.press(KeyCode::Digit1);
            cursor.0 = at(45.0, -4.5);
            hold(&mut mouse, true);
        }
        t if t < 2.7 => hold(&mut mouse, false),
        t if t < 2.8 => {
            keys.press(KeyCode::Digit2);
            cursor.0 = at(105.0, 3.0);
            hold(&mut mouse, true);
        }
        t if t < 5.0 => hold(&mut mouse, false),
        t if t < 6.2 => {
            keys.press(KeyCode::Digit3);
            if let Some(w) = sim.materials().id("water") {
                belt.material = w;
            }
            cursor.0 = at(-90.0, 60.0);
            hold(&mut mouse, true);
        }
        t if t < 7.2 => {
            if let Some(o) = sim.materials().id("oil") {
                belt.material = o;
            }
            cursor.0 = at(-90.0, 60.0);
        }
        t if t < 8.2 => hold(&mut mouse, false),
        t if t < 8.8 => {
            keys.press(KeyCode::Digit4);
            cursor.0 = at(-90.0, 4.5);
            hold(&mut mouse, true);
        }
        t if t < 9.0 => hold(&mut mouse, false),
        t if t < 11.5 => {
            // Melt a hole into the rock next to the player.
            keys.press(KeyCode::Digit6);
            cursor.0 = at(37.5, -9.0);
            hold(&mut mouse, true);
        }
        _ => {
            hold(&mut mouse, false);
            cursor.0 = at(0.0, 105.0);
        }
    }
}

/// Build a branching wooden tree next to the player, then light its base.
fn tree_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut state: Local<(Option<f32>, u8, Vec2)>,
) {
    if s.name != "tree" {
        return;
    }
    let Ok(p) = player.single() else { return };
    let t0 = *state.0.get_or_insert(s.elapsed);
    let t = s.elapsed - t0;
    let Some(wood) = sim.materials().id("wood") else { return };
    if state.1 == 0 && t > 0.3 {
        let x = p.body.pos.x + 105.0;
        let Some(ground) = find_ground(&sim.world, x as i32, p.body.pos.y as i32 + 90, 450) else { return };
        let base = Vec2::new(x, ground as f32);
        state.2 = base;
        let mut line = |a: Vec2, b: Vec2, r: i32| {
            let n = a.distance(b).ceil() as i32;
            for i in 0..=n {
                let q = a.lerp(b, i as f32 / n.max(1) as f32);
                sim.queue(WorldEdit::Paint { center: CellPos::from_world(q.x, q.y), radius: r, material: wood, overwrite: true });
            }
        };
        let top = base + Vec2::new(0.0, 105.0);
        line(base, top, 3);
        for (from, dir) in [(60.0, -1.0), (78.0, 1.0), (93.0, -1.0), (45.0, 1.0)] {
            let start = base + Vec2::new(0.0, from);
            let end = start + Vec2::new(dir * 39.0, 18.0);
            line(start, end, 2);
            for k in 0..3 {
                let twig = start.lerp(end, 0.4 + k as f32 * 0.25);
                line(twig, twig + Vec2::new(dir * 4.5, 10.5), 0);
            }
        }
        state.1 = 1;
    }
    if state.1 == 1 && t > 1.2 {
        let base = state.2;
        sim.queue(WorldEdit::Ignite { center: CellPos::from_world(base.x, base.y + 6.0), radius: 4 });
        state.1 = 2;
    }
}

fn blast_script(
    mut commands: Commands,
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    tools: Res<ToolsConfig>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut shaft: Local<Option<(f32, i32)>>,
    mut dropped: Local<u32>,
) {
    // Three bombs dropped down the same shaft, each after the last went off:
    // with no chain reaction they dig instead of going off together.
    let due = 2.0 + *dropped as f32 * (tools.bomb.fuse + 0.4);
    if s.name != "blast" || *dropped >= 3 || s.elapsed < due {
        return;
    }
    let Ok(p) = player.single() else { return };
    let (x, top) = *shaft.get_or_insert_with(|| {
        let x = p.body.pos.x + 105.0;
        (x, find_ground(&sim.world, x as i32, p.body.pos.y as i32 + 90, 450).unwrap_or(p.body.pos.y as i32) + 18)
    });
    spawn_bomb(&mut commands, Vec2::new(x, top as f32), Vec2::ZERO, tools.bomb.clone());
    *dropped += 1;
}

fn fell_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut done: Local<bool>) {
    if !matches!(s.name.as_str(), "fell" | "burn" | "strike") || *done || s.elapsed < 2.0 {
        return;
    }
    let Ok(p) = player.single() else { return };
    let (px, py) = (p.body.pos.x as i32, p.body.pos.y as i32);
    // The first background wood 30 cells above the ground (clear of tall
    // grass, below the branches): a trunk.
    for x in px + 45..px + 390 {
        let Some(ground) = find_ground(&sim.world, x, py + 120, 450) else { continue };
        let at = CellPos::new(x, ground + 30);
        let wood = |x: i32| {
            let q = CellPos::new(x, at.y);
            sim.world.get_bg(q).is_some_and(|b| !b.is_air() && sim.world.materials().phys(b.material).kind == platypus_sim::Kind::Static)
        };
        if !wood(x) {
            continue;
        }
        // Both edges: the search may start inside a trunk.
        let left = x - (1..60).take_while(|&d| wood(x - d)).count() as i32;
        let width = (left..left + 90).take_while(|&x| wood(x)).count() as i32;
        let center = CellPos::new(left + width / 2, at.y);
        if s.name == "burn" {
            sim.queue(WorldEdit::Ignite { center: CellPos::new(center.x, ground + 4), radius: width / 2 + 3 });
        } else if s.name == "strike" {
            sim.queue(WorldEdit::Lightning { x: center.x, from_y: py + 450 });
        } else {
            // Twice: a dig clears the playfield first where anything stands in front.
            for _ in 0..2 {
                sim.queue(WorldEdit::Dig { center, radius: width / 2 + 4, max_hardness: 200 });
            }
        }
        info!("{}: trunk {width} wide at {center:?}, player at {px}", s.name);
        *done = true;
        return;
    }
}

fn acid_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut step: Local<u8>) {
    if s.name != "acid" {
        return;
    }
    let Ok(p) = player.single() else { return };
    let x = p.body.pos.x as i32 + 90;
    let Some(ground) = find_ground(&sim.world, x, p.body.pos.y as i32 + 90, 450) else { return };
    let at = CellPos::new(x, ground + 6);
    match *step {
        0 if s.elapsed > 1.0 => {
            // A glass basin (acid eats dirt and stone, not glass), then the acid.
            let (Some(glass), Some(acid)) = (sim.materials().id("glass"), sim.materials().id("acid")) else { return };
            sim.queue(WorldEdit::Paint { center: at.offset(0, -15), radius: 21, material: glass, overwrite: true });
            sim.queue(WorldEdit::Dig { center: at.offset(0, -3), radius: 14, max_hardness: 40 });
            sim.queue(WorldEdit::Paint { center: at.offset(0, 3), radius: 12, material: acid, overwrite: false });
            *step = 1;
        }
        1 if s.elapsed > 2.0 => {
            sim.queue(WorldEdit::Heat { center: at, radius: 18, amount: 250 });
            *step = 2;
        }
        2 if s.elapsed > 3.6 => {
            sim.queue(WorldEdit::Ignite { center: at.offset(0, 30), radius: 9 });
            *step = 3;
        }
        _ => {}
    }
}

fn rain_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut step: Local<u8>) {
    if s.name != "rain" {
        return;
    }
    let Ok(p) = player.single() else { return };
    let x = p.body.pos.x as i32;
    match *step {
        0 if s.elapsed > 2.0 => {
            if let Some(ground) = find_ground(&sim.world, x + 120, p.body.pos.y as i32 + 90, 450) {
                sim.queue(WorldEdit::Ignite { center: CellPos::new(x + 120, ground + 3), radius: 9 });
            }
            *step = 1;
        }
        // What F5 does.
        1 if s.elapsed > 3.0 => {
            sim.queue(WorldEdit::Weather { x, radius: 600, storm: true });
            *step = 2;
        }
        // What F7 does, onto a spot left of the player.
        2 if s.elapsed > 6.0 => {
            sim.queue(WorldEdit::Lightning { x: x - 90, from_y: p.body.pos.y as i32 + 300 });
            *step = 3;
        }
        _ => {}
    }
}

fn swim_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut pool: Local<Option<(i32, i32)>>,
    mut step: Local<u8>,
) {
    if s.name != "swim" {
        return;
    }
    let Ok(p) = player.single() else { return };
    let (px, py) = (p.body.pos.x as i32, p.body.pos.y as i32);
    let &mut (x, ground) = pool.get_or_insert_with(|| (px + 60, find_ground(&sim.world, px + 60, py + 90, 450).unwrap_or(py)));
    match *step {
        0 if s.elapsed > 0.5 => {
            let Some(glass) = sim.materials().id("glass") else { return };
            // A glass-lined pit, 42 wide and 45 deep, full of water.
            for dx in (-24..=24).step_by(3) {
                sim.queue(WorldEdit::Paint { center: CellPos::new(x + dx, ground - 27), radius: 24, material: glass, overwrite: true });
            }
            for dx in (-21..=21).step_by(3) {
                sim.queue(WorldEdit::Dig { center: CellPos::new(x + dx, ground - 21), radius: 21, max_hardness: 200 });
            }
            *step = 1;
        }
        1 if s.elapsed > 0.6 => {
            if let Some(water) = sim.materials().id("water") {
                for dx in (-18..=18).step_by(6) {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x + dx, ground - 18), radius: 18, material: water, overwrite: false });
                }
            }
            *step = 2;
        }
        2 if s.elapsed > 2.0 => {
            if let Some(oil) = sim.materials().id("oil") {
                sim.queue(WorldEdit::Paint { center: CellPos::new(px, py + 21), radius: 6, material: oil, overwrite: false });
            }
            *step = 3;
        }
        3 if s.elapsed > 2.6 => {
            sim.queue(WorldEdit::Ignite { center: CellPos::new(px, py), radius: 9 });
            *step = 4;
        }
        4 if s.elapsed > 3.2 => keys.press(KeyCode::KeyD),
        _ => {}
    }
    if *step == 4 && px > x {
        keys.release(KeyCode::KeyD);
    }
}

#[allow(clippy::too_many_arguments)]
fn dark_script(
    mut commands: Commands,
    lights: Res<crate::light::LightSettings>,
    torch_art: Option<Res<crate::light::TorchArt>>,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut day: ResMut<crate::light::Daylight>,
    mut toggles: ResMut<crate::light::LightToggles>,
    mut cursor: ResMut<CursorOverride>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut done: Local<bool>,
) {
    if !matches!(s.name.as_str(), "night" | "cave" | "dusk") || *done || s.elapsed < 1.0 {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    // 23:00, or 18:15 for dusk (the day starts at the configured hour; skip
    // the difference).
    let hour = if s.name == "dusk" { 18.25 } else { 23.0 };
    day.skipped = (hour - day.time * 24.0).rem_euclid(24.0);
    if s.name == "cave" {
        // A chamber 150 below the surface, a lava pool on one side, acid on the
        // other; the player on its floor with the flashlight on.
        let x = k.body.pos.x as i32;
        let c = CellPos::new(x, k.body.pos.y as i32 - 150);
        let (Some(lava), Some(acid), Some(glass)) = (sim.materials().id("lava"), sim.materials().id("acid"), sim.materials().id("glass")) else { return };
        for dx in (-75..=75).step_by(15) {
            sim.queue(WorldEdit::Dig { center: c.offset(dx, (dx.abs() / 6) - 6), radius: 33, max_hardness: 250 });
        }
        sim.queue(WorldEdit::Paint { center: c.offset(93, -39), radius: 14, material: lava, overwrite: true });
        sim.queue(WorldEdit::Paint { center: c.offset(-87, -45), radius: 15, material: glass, overwrite: true });
        sim.queue(WorldEdit::Dig { center: c.offset(-87, -39), radius: 11, max_hardness: 250 });
        sim.queue(WorldEdit::Paint { center: c.offset(-87, -38), radius: 9, material: acid, overwrite: false });
        k.body.pos = Vec2::new(c.x as f32, c.y as f32 - 12.0);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
        toggles.carry = if std::env::var("PLATYPUS_NOBEAM").is_err() { crate::light::Carry::BigBeam } else { crate::light::Carry::Torch };
        cursor.0 = Some(k.body.pos + Vec2::new(135.0, -15.0));
        // A torch planted to the left, glow sticks thrown both ways.
        if let Some(art) = torch_art.as_deref() {
            crate::light::plant_torch(&mut commands, k.body.pos + Vec2::new(-60.0, -k.body.half.y), &lights, art);
        }
        let s = lights.glowstick.strength;
        crate::props::spawn_glowstick(&mut commands, k.body.pos + Vec2::new(-30.0, 6.0), Vec2::new(-90.0, 60.0), [0.25 * s, s, 0.45 * s], 90.0, lights.glowstick.haze);
        crate::props::spawn_glowstick(&mut commands, k.body.pos + Vec2::new(45.0, 6.0), Vec2::new(90.0, 60.0), [0.2 * s, 0.55 * s, 1.1 * s], 90.0, lights.glowstick.haze);
    }
    *done = true;
}

fn flood_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut step: Local<u8>, mut at: Local<Option<CellPos>>) {
    if s.name != "flood" {
        return;
    }
    let Ok(p) = player.single() else { return };
    let c = *at.get_or_insert_with(|| CellPos::new(p.body.pos.x as i32 + 30, p.body.pos.y as i32 - 255));
    match *step {
        // A hall 300 wide, 105 high, with stone floor; then a block of water
        // 75 wide and 90 high at its left end.
        0 if s.elapsed > 0.5 => {
            let (Some(stone), Some(water)) = (sim.materials().id("stone"), sim.materials().id("water")) else { return };
            for x in (-165..=165).step_by(9) {
                for y in (-9..=114).step_by(9) {
                    sim.queue(WorldEdit::Dig { center: c.offset(x, y), radius: 8, max_hardness: 250 });
                }
            }
            for x in (-165..=165).step_by(6) {
                sim.queue(WorldEdit::Paint { center: c.offset(x, -15), radius: 6, material: stone, overwrite: true });
            }
            *step = 1;
            let _ = water;
        }
        1 if s.elapsed > 0.8 => {
            let Some(water) = sim.materials().id("water") else { return };
            for x in (-150..=-82).step_by(4) {
                for y in (-4..=90).step_by(4) {
                    sim.queue(WorldEdit::Paint { center: c.offset(x, y), radius: 3, material: water, overwrite: false });
                }
            }
            *step = 2;
        }
        _ => {}
    }
}

/// The hands through real input (keys, mouse, a scripted cursor).
#[allow(clippy::too_many_arguments)]
fn chestfall_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut commands: Commands,
    mut chests: ResMut<crate::hands::chests::Chests>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    found: Query<&Kinematics, With<crate::hands::chests::Chest>>,
    mut step: Local<u8>,
    mut feet: Local<Option<Vec2>>,
) {
    if s.name != "chestfall" {
        return;
    }
    let Ok(k) = player.single() else { return };
    let chest_y = || found.iter().map(|c| c.body.bottom()).next();
    match *step {
        0 if s.elapsed > 1.0 => {
            // Somewhere to the side with room for it.
            let spot = (30..180).step_by(9).flat_map(|dx| [dx, -dx]).find_map(|dx| (-45..45).step_by(9).find_map(|dy| crate::hands::chests::place_spot(&sim.world, k.body.pos + Vec2::new(dx as f32, dy as f32))));
            let Some(at) = spot else { return };
            chests.spawn_placed(&mut commands, at);
            *feet = Some(at);
            *step = 1;
        }
        1 if s.elapsed > 1.9 => {
            info!("chestfall: chest standing at {:?}", chest_y());
            let at = feet.expect("placed");
            // (Centred a little high, so the hole's mouth is wider than the
            // chest, 18 cells: a circle just under its feet opens only 13.)
            sim.queue(WorldEdit::Dig { center: CellPos::new(at.x as i32, at.y as i32 - 14), radius: 21, max_hardness: 255 });
            *step = 2;
        }
        2 if s.elapsed > 4.0 => {
            info!("chestfall: chest now at {:?}, centre {:?}", chest_y(), found.iter().next().map(|c| c.body.pos));
            // A small blast beside it: thrown, not broken; then a big one on it.
            let c = found.iter().next().map(|c| c.body.pos).expect("a chest");
            sim.queue(WorldEdit::Explode { center: CellPos::new(c.x as i32 + 18, c.y as i32), radius: 8, power: 60 });
            *step = 3;
        }
        3 if s.elapsed > 5.0 => {
            info!("chestfall: after a blast beside it: {:?}", found.iter().next().map(|c| c.body.pos));
            if let Some(c) = found.iter().next().map(|c| c.body.pos) {
                sim.queue(WorldEdit::Explode { center: CellPos::new(c.x as i32, c.y as i32), radius: 21, power: 100 });
            }
            *step = 4;
        }
        4 if s.elapsed > 6.0 => {
            info!("chestfall: after a blast on it: {} chests", found.iter().count());
            *step = 5;
        }
        _ => {}
    }
}

fn zoom_script(
    s: Res<Scenario>,
    mut chars: ResMut<ButtonInput<bevy::input::keyboard::Key>>,
    mut scroll: ResMut<bevy::input::mouse::AccumulatedMouseScroll>,
    zoom: Res<crate::camera::Zoom>,
    hand: Res<crate::hands::Hand>,
    mut step: Local<u8>,
) {
    use bevy::input::keyboard::Key;
    if s.name != "zoom" {
        return;
    }
    chars.release_all();
    let t = s.elapsed;
    let at = |k: u8| *step == k && t > 1.0 + k as f32 * 0.4;
    if at(0) {
        info!("zoom: starts at {} px/cell, slot {}", zoom.0, hand.slot);
        chars.press(Key::Character("+".into()));
        *step = 1;
    } else if at(1) {
        chars.press(Key::Character("+".into()));
        *step = 2;
    } else if at(2) {
        info!("zoom: after + +: {} px/cell", zoom.0);
        chars.press(Key::Character("-".into()));
        *step = 3;
    } else if at(3) {
        info!("zoom: after -: {} px/cell", zoom.0);
        scroll.delta.y = -2.0;
        *step = 4;
    } else if at(4) {
        info!("zoom: after two wheel notches down: slot {}", hand.slot);
        *step = 5;
    }
}

fn shroom_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut step: Local<u8>) {
    if s.name != "shroom" {
        return;
    }
    let Ok(k) = player.single() else { return };
    match *step {
        0 if s.elapsed > 2.0 => {
            let Some(stem) = sim.materials().id("mushroom_stem") else { return };
            // The nearest stem cell with open air in front, a little up it.
            let p = CellPos::from_world(k.body.pos.x, k.body.pos.y);
            let found = (0..300).flat_map(|r: i32| (-r..=r).flat_map(move |dx| [(dx, r), (dx, -r), (r, dx), (-r, dx)])).map(|(dx, dy)| p.offset(dx, dy)).find(|&q| {
                sim.world.get_bg(q).is_some_and(|c| c.material == stem) && sim.world.get_bg(q.offset(0, -9)).is_some_and(|c| c.material == stem) && sim.world.get(q).is_some_and(|c| c.is_air())
            });
            let Some(at) = found else {
                info!("shroom: no mushroom near");
                *step = 9;
                return;
            };
            info!("shroom: cutting the stem at {at:?} ({} bodies)", sim.world.bodies().len());
            for dx in -3..=3 {
                let block = CellPos::new((at.x + dx * platypus_sim::BLOCK).div_euclid(platypus_sim::BLOCK), at.y.div_euclid(platypus_sim::BLOCK));
                sim.world.apply_edit(&WorldEdit::MineBlock { block, power: 255, max_hardness: 254, back: true });
            }
            *step = 1;
        }
        1 if s.elapsed > 3.5 => {
            info!("shroom: after the cut, {} bodies", sim.world.bodies().len());
            *step = 2;
        }
        _ => {}
    }
}

fn drop_script(s: Res<Scenario>, player: Query<&Kinematics, With<LocalPlayer>>, mut keys: ResMut<ButtonInput<KeyCode>>, mut logged: Local<u8>) {
    if s.name != "drop" {
        return;
    }
    let Ok(k) = player.single() else { return };
    if s.elapsed > 3.0 { keys.press(KeyCode::KeyS) } else { keys.release(KeyCode::KeyS) }
    let feet = k.body.bottom();
    if *logged == 0 && s.elapsed > 2.9 {
        info!("drop: feet at {feet:.0} before holding S");
        *logged = 1;
    } else if *logged == 1 && s.elapsed > 5.5 {
        info!("drop: feet at {feet:.0} after");
        *logged = 2;
    }
}

#[allow(clippy::too_many_arguments)]
fn hands_script(
    s: Res<Scenario>,
    player: Query<(&Kinematics, Option<&crate::hands::items::Inventory>), With<LocalPlayer>>,
    items: Option<Res<crate::hands::items::Items>>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut logged: Local<u8>,
    mut build_slot: Local<Option<usize>>,
    mut start_y: Local<Option<f32>>,
) {
    use crate::hands::items::{BLOCK_CELLS, Use};
    if s.name != "hands" {
        return;
    }
    let Ok((k, inv)) = player.single() else { return };
    let (Some(inv), Some(items)) = (inv, items) else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    const DIGITS: [KeyCode; 10] = [
        KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5,
        KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9, KeyCode::Digit0,
    ];
    for key in DIGITS {
        keys.release(key);
    }
    let pick = |keys: &mut ButtonInput<KeyCode>, slot: usize| keys.press(DIGITS[slot]);
    // Straight down, held: the smart cursor digs a shaft the player drops into.
    if t < 0.8 {
        *start_y = Some(p.y);
    }
    let (aim, hold, ctrl) = match t {
        t if t < 0.8 => (None, false, false),
        t if t < 6.0 => {
            pick(&mut keys, 0);
            if (5.9..6.0).contains(&t) && *logged == 0 {
                *logged = 2;
                info!("hands: dug down {:.0} cells in 5.2 s", start_y.unwrap_or(p.y) - p.y);
            }
            (Some(p + Vec2::new(0.0, -45.0)), true, false)
        }
        // Into the shaft's wall.
        t if t < 7.0 => (Some(p + Vec2::new(21.0, -3.0)), true, false),
        // Build with a stack that has a whole block.
        t if t < 8.5 => {
            if build_slot.is_none() {
                *build_slot = (0..10).find(|&i| inv.slots[i].is_some_and(|st| matches!(items.def(st.item).use_, Use::Block(_)) && st.count >= BLOCK_CELLS));
            }
            if let Some(i) = *build_slot {
                pick(&mut keys, i);
            }
            (Some(p + Vec2::new(-15.0, 6.0)), true, false)
        }
        // Auto tool (Ctrl) at whatever's up and to the right.
        t if t < 10.0 => (Some(p + Vec2::new(30.0, 36.0)), true, true),
        // A torch on the floor.
        t if t < 10.3 => {
            if let Some(i) = items.id("torch").and_then(|torch| (0..10).find(|&i| inv.slots[i].is_some_and(|st| st.item == torch))) {
                pick(&mut keys, i);
            }
            (Some(p + Vec2::new(-9.0, -12.0)), false, false)
        }
        t if t < 10.4 => (Some(p + Vec2::new(-9.0, -12.0)), true, false),
        _ => (None, false, false),
    };
    cursor.0 = aim;
    if hold { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    if ctrl { keys.press(KeyCode::ControlLeft) } else { keys.release(KeyCode::ControlLeft) }
    if t > 11.0 && *logged < 3 {
        *logged = 3;
        let held: Vec<String> = inv.slots.iter().flatten().map(|st| format!("{} {}", items.def(st.item).name, st.count / items.unit(st.item))).collect();
        let torches = items.id("torch").map_or(0, |torch| inv.count(torch));
        info!("hands: inventory {} (built with slot {:?}; {torches} torches left)", held.join(", "), build_slot.map(|i| i + 1));
    }
}

/// A chest through real input: place, open, fill (directly), close, mine.
#[allow(clippy::too_many_arguments)]
fn chest_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut player: Query<(&Kinematics, Option<&mut crate::hands::items::Inventory>), With<LocalPlayer>>,
    items: Option<Res<crate::hands::items::Items>>,
    mut chests: ResMut<crate::hands::chests::Chests>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut step: Local<u8>,
    mut spot: Local<Option<Vec2>>,
) {
    use crate::hands::items::Stack;
    if s.name != "chest" {
        return;
    }
    let Ok((k, Some(mut inv))) = player.single_mut() else { return };
    let Some(items) = items else { return };
    let (Some(chest), Some(bomb)) = (items.id("chest"), items.id("bomb")) else { return };
    let p = k.body.pos;
    for key in [KeyCode::Digit1, KeyCode::Digit9, KeyCode::Escape] {
        keys.release(key);
    }
    mouse.release(MouseButton::Left);
    mouse.release(MouseButton::Right);
    let at = *spot.get_or_insert(p + Vec2::new(24.0, 0.0));
    match *step {
        0 if s.elapsed > 0.8 => {
            // A chest in slot 9.
            inv.slots[8] = Some(Stack::new(chest, 1));
            keys.press(KeyCode::Digit9);
            *step = 1;
        }
        1 if s.elapsed > 1.2 => {
            cursor.0 = Some(at);
            mouse.press(MouseButton::Left);
            *step = 2;
        }
        2 if s.elapsed > 1.6 => {
            cursor.0 = Some(at + Vec2::new(0.0, -6.0));
            mouse.press(MouseButton::Right);
            *step = 3;
        }
        3 if s.elapsed > 2.0 => {
            match chests.open {
                Some(c) => {
                    let taken = inv.slots.iter_mut().flatten().find(|st| st.item == bomb).map(|st| std::mem::replace(&mut st.count, 0)).unwrap_or(0);
                    inv.slots.iter_mut().for_each(|sl| if sl.is_some_and(|st| st.count == 0) { *sl = None });
                    chests.contents(c, &sim.world, &items).add(&items, Stack::new(bomb, taken));
                    info!("chest: placed and opened at {c:?}, {taken} bombs put in");
                }
                None => info!("chest: not open"),
            }
            keys.press(KeyCode::Escape);
            *step = 4;
        }
        4 if s.elapsed > 2.4 => {
            keys.press(KeyCode::Digit1);
            *step = 5;
        }
        5 if s.elapsed > 2.5 && s.elapsed < 5.0 => {
            cursor.0 = Some(at + Vec2::new(0.0, -6.0));
            mouse.press(MouseButton::Left);
        }
        5 if s.elapsed >= 6.0 => {
            let bombs = inv.count(bomb);
            let chests_back = inv.count(chest);
            info!("chest: after mining it, {bombs} bombs and {chests_back} chest in the pack");
            *step = 6;
        }
        _ => {}
    }
}

type Others = (With<crate::creatures::Creature>, Without<LocalPlayer>);

/// Every starter wand, through real input (see the module notes).
#[allow(clippy::too_many_arguments)]
fn magic_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut commands: Commands,
    mut player: Query<(&Kinematics, &mut crate::creatures::Health, Option<&mut crate::magic::Mana>), With<LocalPlayer>>,
    orcs: Query<(&Kinematics, &crate::creatures::Health), Others>,
    spells: Query<(), With<crate::magic::Spell>>,
    mut blasts: MessageReader<crate::fx::Explosion>,
    mut zaps: MessageReader<crate::fx::Zapped>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, u32, u32)>,
) {
    if s.name != "magic" {
        return;
    }
    let Ok((k, mut me, mut mana)) = player.single_mut() else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    // Full mana and health for each wand, so each is seen on its own
    // (what it costs, and what it does to its caster).
    if [1.5, 3.0, 4.5, 5.5, 7.0].iter().any(|&at| (at..at + 0.05).contains(&t)) {
        me.hp = me.max;
        if let Some(m) = mana.as_deref_mut() {
            m.cur = m.max;
        }
    }
    state.2 += blasts.read().count() as u32;
    state.3 += zaps.read().count() as u32;
    if state.0 == 0 && t > 1.0 {
        for dx in [135, 180, 225] {
            let x = p.x as i32 + dx;
            if let Some(y) = find_ground(&sim.world, x, p.y as i32 + 300, 600) {
                crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(x as f32, y as f32), |_| {});
            }
        }
        state.0 = 1;
    }
    const SLOTS: [KeyCode; 5] = [KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9, KeyCode::Digit0];
    for key in SLOTS {
        keys.release(key);
    }
    let nearest = orcs.iter().map(|(o, _)| o.body.pos).min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)));
    let at_orc = nearest.unwrap_or(p + Vec2::new(90.0, 0.0));
    let (slot, aim) = match t {
        t if t < 1.5 => (None, None),
        t if t < 3.0 => (Some(0), Some(at_orc)),
        t if t < 4.5 => (Some(1), Some(p + Vec2::new(75.0, -18.0))),
        t if t < 5.5 => (Some(2), Some(p + Vec2::new(60.0, 60.0))),
        t if t < 7.0 => (Some(3), Some(at_orc)),
        t if t < 8.5 => (Some(4), Some(at_orc)),
        _ => (None, None),
    };
    if let Some(i) = slot {
        keys.press(SLOTS[i]);
    }
    cursor.0 = aim.or(Some(p + Vec2::new(45.0, 0.0)));
    if aim.is_some() { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    if t >= state.1 && state.0 == 1 {
        state.1 = (t * 4.0).floor() / 4.0 + 0.25;
        let hp: Vec<String> = orcs.iter().map(|(_, h)| format!("{:.0}", h.hp)).collect();
        info!(
            "magic: t {:.0} slot {:?} hp {:.0} mana {:.0} orcs [{}] spells {} blasts {} zaps {}",
            t,
            slot.map(|i| i + 6),
            me.hp,
            mana.map_or(0.0, |m| m.cur),
            hp.join(", "),
            spells.iter().count(),
            state.2,
            state.3
        );
    }
}

/// Lightning into a pool with orcs in it (see the module notes).
#[allow(clippy::too_many_arguments)]
fn shock_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut commands: Commands,
    player: Query<&Kinematics, With<LocalPlayer>>,
    orcs: Query<(&Kinematics, &crate::creatures::Health), Others>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut step: Local<u8>,
    mut pool: Local<Option<(i32, i32)>>,
    mut zaps: MessageReader<crate::fx::Zapped>,
) {
    if s.name != "shock" {
        return;
    }
    for crate::fx::Zapped(z) in zaps.read() {
        let orcs_at: Vec<String> = orcs.iter().map(|(o, _)| format!("({:.0},{:.0})", o.body.pos.x, o.body.pos.y)).collect();
        info!("shock: zap {:?} -> {:?}, {} cells charged; orcs {}", z.from, z.to, z.charged.len(), orcs_at.join(" "));
    }
    let Ok(k) = player.single() else { return };
    let p = k.body.pos;
    let near = |x: i32, y: i32| orcs.iter().filter(|(o, _)| (o.body.pos.x as i32 - x).abs() < 75 && (o.body.pos.y as i32 - y).abs() < 45).map(|(_, h)| format!("{:.0}", h.hp)).collect::<Vec<_>>();
    match *step {
        0 if s.elapsed > 1.0 => {
            // A trough 120 wide, 21 deep, 60 to the right; then water in it.
            let Some(ground) = find_ground(&sim.world, p.x as i32 + 120, p.y as i32 + 60, 150) else { return };
            for x in (p.x as i32 + 60..p.x as i32 + 180).step_by(6) {
                sim.queue(WorldEdit::Dig { center: CellPos::new(x, ground - 12), radius: 12, max_hardness: 255 });
            }
            *pool = Some((p.x as i32 + 120, ground - 12));
            *step = 1;
        }
        1 if s.elapsed > 1.3 => {
            let (cx, cy) = pool.expect("dug");
            let water = sim.materials().expect_id("water");
            for x in (cx - 60..cx + 60).step_by(4) {
                for dy in [-6, 3] {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, cy + dy), radius: 10, material: water, overwrite: false });
                }
            }
            *step = 2;
        }
        // (In the far end, and struck before they wade out toward the player.)
        2 if s.elapsed > 3.5 => {
            let (cx, cy) = pool.expect("dug");
            for dx in [33, 51] {
                crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new((cx + dx) as f32, cy as f32 + 3.0), |_| {});
            }
            *step = 3;
        }
        3 if s.elapsed > 3.9 => {
            let (cx, cy) = pool.expect("dug");
            info!("shock: before, orcs at the pool [{}]", near(cx, cy).join(", "));
            keys.press(KeyCode::Digit0);
            cursor.0 = Some(Vec2::new(cx as f32, cy as f32));
            mouse.press(MouseButton::Left);
            *step = 4;
        }
        4 if s.elapsed > 4.1 => {
            keys.release(KeyCode::Digit0);
            mouse.release(MouseButton::Left);
            *step = 5;
        }
        5 if s.elapsed > 4.6 => {
            let (cx, cy) = pool.expect("dug");
            info!("shock: after, orcs at the pool [{}]", near(cx, cy).join(", "));
            *step = 6;
        }
        _ => {}
    }
}

/// The inventory screen through real input (see the module notes).
#[allow(clippy::too_many_arguments)]
fn inventory_script(
    s: Res<Scenario>,
    mut window: Single<&mut Window, With<bevy::window::PrimaryWindow>>,
    slots: Query<(&crate::hands::ui::SlotUi, &bevy::ui::UiGlobalTransform, &InheritedVisibility)>,
    items: Option<Res<crate::hands::items::Items>>,
    player: Query<&crate::hands::items::Inventory, With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut step: Local<u8>,
) {
    use crate::hands::ui::Holder;
    if s.name != "inventory" {
        return;
    }
    let t = s.elapsed;
    keys.release(KeyCode::Escape);
    keys.release(KeyCode::KeyX);
    let scale = window.scale_factor();
    // Where a slot of the inventory screen is, in window (logical) pixels.
    let at = |i: usize| slots.iter().find(|(sl, _, v)| sl.0 == Holder::Pack && sl.1 == i && v.get()).map(|(_, tf, _)| tf.translation / scale);
    match *step {
        0 if t > 1.0 => {
            keys.press(KeyCode::Escape);
            *step = 1;
        }
        1 if t > 1.5 => {
            keys.press(KeyCode::KeyX);
            *step = 2;
        }
        2 if t > 2.0 => {
            window.set_cursor_position(at(5));
            *step = 3;
        }
        3 if t > 2.6 => {
            mouse.press(MouseButton::Left);
            *step = 4;
        }
        4 if t > 2.8 => {
            window.set_cursor_position(at(20));
            *step = 5;
        }
        5 if t > 3.0 => {
            mouse.release(MouseButton::Left);
            *step = 6;
        }
        6 if t > 3.3 => {
            if let (Some(items), Ok(inv)) = (items, player.single()) {
                let wand = items.id("storm_wand");
                let found = inv.slots.iter().position(|s| s.is_some_and(|s| Some(s.item) == wand));
                info!("inventory: the storm wand is in slot {found:?} (was 5; hotbar 3 starts at 20)");
            }
            window.set_cursor_position(None);
            *step = 7;
        }
        _ => {}
    }
}

/// The gravity wand through real input (see the module notes).
#[allow(clippy::too_many_arguments)]
fn well_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut commands: Commands,
    player: Query<(&Kinematics, Option<&crate::magic::Mana>), With<LocalPlayer>>,
    orcs: Query<(&Kinematics, &crate::creatures::Health), Others>,
    wells: Query<&crate::magic::well::Well>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, Option<Vec2>)>,
) {
    if s.name != "well" {
        return;
    }
    let Ok((k, mana)) = player.single() else { return };
    let t = s.elapsed;
    keys.release(KeyCode::KeyX);
    keys.release(KeyCode::Digit2);
    let home = *state.2.get_or_insert(k.body.pos);
    if state.0 == 0 && t > 1.0 {
        for dx in [105, 128] {
            let x = home.x as i32 + dx;
            if let Some(y) = find_ground(&sim.world, x, home.y as i32 + 90, 300) {
                crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(x as f32, y as f32), |_| {});
            }
        }
        // The second hotbar, its second slot: the gravity wand.
        keys.press(KeyCode::KeyX);
        keys.press(KeyCode::Digit2);
        state.0 = 1;
    }
    let lerp = |a: Vec2, b: Vec2, f: f32| a.lerp(b, f.clamp(0.0, 1.0));
    let nearest = orcs.iter().map(|(o, _)| o.body.pos).filter(|o| o.x > home.x + 30.0).min_by(|a, b| a.x.total_cmp(&b.x)).unwrap_or(home + Vec2::new(105.0, 12.0));
    let (ground, up, throw) = (home + Vec2::new(52.0, -9.0), home + Vec2::new(60.0, 45.0), home + Vec2::new(240.0, 195.0));
    let aim = match t {
        t if t < 1.5 => None,
        t if t < 2.6 => Some(ground),
        t if t < 3.0 => Some(nearest),
        t if t < 3.6 => Some(lerp(nearest, up, (t - 3.0) / 0.4)),
        t if t < 3.75 => Some(lerp(up, throw, (t - 3.6) / 0.15)),
        _ => None,
    };
    cursor.0 = aim.or(Some(home + Vec2::new(45.0, 30.0)));
    if aim.is_some() { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    if t >= state.1 {
        state.1 = (t * 4.0).floor() / 4.0 + 0.25;
        let held: Vec<(usize, usize)> = wells.iter().map(|w| (w.holding(), w.carrying())).collect();
        let hp: Vec<String> = orcs.iter().filter(|(o, _)| o.body.pos.x > home.x + 30.0).map(|(o, h)| format!("{:.0}@{:.0},{:.0}", h.hp, o.body.pos.x - home.x, o.body.pos.y - home.y)).collect();
        info!("well: t {t:.1} (cells, bodies) {held:?} mana {:.0} orcs [{}] particles {}", mana.map_or(0.0, |m| m.cur), hp.join(" "), sim.world.particles().len());
    }
}

/// The force wand through real input (see the module notes).
#[allow(clippy::too_many_arguments)]
fn force_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut commands: Commands,
    player: Query<(&Kinematics, Option<&crate::magic::Mana>, &crate::creatures::Health), With<LocalPlayer>>,
    orcs: Query<(&Kinematics, &crate::creatures::Health), Others>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, Option<Vec2>)>,
) {
    if s.name != "force" {
        return;
    }
    let Ok((k, mana, me)) = player.single() else { return };
    let t = s.elapsed;
    keys.release(KeyCode::KeyX);
    keys.release(KeyCode::Digit3);
    let home = *state.2.get_or_insert(k.body.pos);
    if state.0 == 0 && t > 1.0 {
        let sand = sim.materials().expect_id("sand");
        for dx in [51, 60, 69] {
            sim.queue(WorldEdit::Paint { center: CellPos::new(home.x as i32 + dx, home.y as i32 + 3), radius: 9, material: sand, overwrite: false });
        }
        for dx in [75, 93] {
            let x = home.x as i32 + dx;
            if let Some(y) = find_ground(&sim.world, x, home.y as i32 + 90, 300) {
                crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(x as f32, y as f32), |_| {});
            }
        }
        // The second hotbar, its third slot: the force wand.
        keys.press(KeyCode::KeyX);
        keys.press(KeyCode::Digit3);
        state.0 = 1;
    }
    let down = t > 5.0 && t < 6.0;
    let (push, pull) = ((t > 1.5 && t < 2.0) || down, t > 3.0 && t < 4.4);
    cursor.0 = Some(if down { k.body.pos + Vec2::new(0.0, -60.0) } else { home + Vec2::new(90.0, 0.0) });
    if push { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    if pull { mouse.press(MouseButton::Right) } else { mouse.release(MouseButton::Right) }
    if t >= state.1 {
        state.1 = (t * 4.0).floor() / 4.0 + 0.25;
        let at: Vec<String> = orcs.iter().filter(|(o, _)| (o.body.pos.x - home.x).abs() < 375.0 && o.body.pos.x > home.x + 15.0).map(|(o, h)| format!("{:.0}@{:.0},{:.0}", h.hp, o.body.pos.x - home.x, o.body.pos.y - home.y)).collect();
        info!("force: t {t:.2} {} player {:+.0} hp {:.0} mana {:.0} orcs [{}] particles {}", if push { "push" } else if pull { "pull" } else { "-" }, k.body.pos.y - home.y, me.hp, mana.map_or(0.0, |m| m.cur), at.join(" "), sim.world.particles().len());
    }
}

/// Lift a pool and drop it, counting the water (see the module notes).
#[allow(clippy::too_many_arguments)]
fn wellwater_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    wells: Query<&crate::magic::well::Well>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, Option<Vec2>)>,
) {
    if s.name != "wellwater" {
        return;
    }
    let Ok(k) = player.single() else { return };
    let t = s.elapsed;
    keys.release(KeyCode::KeyX);
    keys.release(KeyCode::Digit2);
    keys.release(KeyCode::Digit4);
    // PLATYPUS_STAFF=1: the gravity staff (hotbar 2, slot 4), four times.
    let staff = std::env::var("PLATYPUS_STAFF").is_ok();
    let home = *state.2.get_or_insert(k.body.pos);
    let water = sim.materials().expect_id("water");
    if state.0 == 0 && t > 0.5 {
        let Some(ground) = find_ground(&sim.world, home.x as i32 + 90, home.y as i32 + 60, 150) else { return };
        for x in (home.x as i32 + 60..home.x as i32 + 120).step_by(6) {
            sim.queue(WorldEdit::Dig { center: CellPos::new(x, ground - 9), radius: 9, max_hardness: 255 });
        }
        state.0 = 1;
    }
    if state.0 == 1 && t > 0.8 {
        let Some(ground) = find_ground(&sim.world, home.x as i32 + 90, home.y as i32 + 60, 150) else { return };
        for x in (home.x as i32 + 63..home.x as i32 + 117).step_by(4) {
            sim.queue(WorldEdit::Paint { center: CellPos::new(x, ground - 4), radius: 6, material: water, overwrite: false });
        }
        keys.press(KeyCode::KeyX);
        keys.press(if staff { KeyCode::Digit4 } else { KeyCode::Digit2 });
        state.0 = 2;
    }
    let cycles = if staff { 4 } else { 2 };
    let phase = t - 1.8;
    let lift = phase > 0.0 && (phase as i32) < cycles * 2 && phase.rem_euclid(2.4) < 1.4 && phase < cycles as f32 * 2.4;
    let high = lift && phase.rem_euclid(2.4) > 0.8;
    let over = home + Vec2::new(90.0, if high { 60.0 } else { -3.0 });
    cursor.0 = Some(over);
    if lift { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    if t >= state.1 && state.0 == 2 {
        state.1 = (t * 2.0).floor() / 2.0 + 0.5;
        let cells: usize = sim.world.chunks().map(|c| c.cells().iter().filter(|c| c.material == water).count()).sum();
        let flying = sim.world.particles().iter().filter(|p| p.cell.material == water).count();
        let held: usize = wells.iter().map(|w| w.holding_of(water)).sum();
        let others: Vec<String> = ["blood", "acid", "steam", "ice"]
            .iter()
            .filter_map(|n| {
                let m = sim.materials().id(n)?;
                let c: usize = sim.world.chunks().map(|c| c.cells().iter().filter(|c| c.material == m).count()).sum::<usize>() + sim.world.particles().iter().filter(|p| p.cell.material == m).count();
                Some(format!("{n} {c}"))
            })
            .collect();
        let stuff: usize = sim.world.chunks().map(|c| c.cells().iter().filter(|c| !c.is_air()).count()).sum::<usize>() + sim.world.particles().len() + wells.iter().map(|w| w.holding()).sum::<usize>();
        info!("wellwater: t {t:.1} {} water: cells {cells} flying {flying} held {held} = {} | {} | everything {stuff}", if lift { "lift" } else { "-" }, cells + flying + held, others.join(" "));
    }
}

/// Spells into water (see the module notes).
#[allow(clippy::too_many_arguments)]
fn splash_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, Option<Vec2>, i32)>,
    mut commands: Commands,
    orcs: Query<Entity, Others>,
) {
    if s.name != "splash" {
        return;
    }
    // (No orcs wading in the way.)
    for e in &orcs {
        commands.entity(e).despawn();
    }
    let Ok(k) = player.single() else { return };
    let t = s.elapsed;
    for key in [KeyCode::KeyX, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8] {
        keys.release(key);
    }
    let home = *state.2.get_or_insert(k.body.pos);
    let (water, oil) = (sim.materials().expect_id("water"), sim.materials().expect_id("oil"));
    if state.0 == 0 && t > 0.5 {
        let Some(ground) = find_ground(&sim.world, home.x as i32 + 120, home.y as i32 + 60, 150) else { return };
        for x in (home.x as i32 + 60..home.x as i32 + 195).step_by(6) {
            sim.queue(WorldEdit::Dig { center: CellPos::new(x, ground - 10), radius: 10, max_hardness: 255 });
        }
        // A pit of its own for the oil (on the pool it spreads over all of it).
        for x in (home.x as i32 + 225..home.x as i32 + 262).step_by(6) {
            sim.queue(WorldEdit::Dig { center: CellPos::new(x, ground - 8), radius: 8, max_hardness: 255 });
        }
        state.3 = ground;
        state.0 = 1;
    }
    if state.0 == 1 && t > 0.8 {
        let ground = state.3;
        for x in (home.x as i32 + 63..home.x as i32 + 192).step_by(4) {
            for dy in [-10, -4] {
                sim.queue(WorldEdit::Paint { center: CellPos::new(x, ground + dy), radius: 6, material: water, overwrite: false });
            }
        }
        state.0 = 2;
    }
    if state.0 == 2 && t > 1.3 {
        let ground = state.3;
        for x in (home.x as i32 + 228..home.x as i32 + 260).step_by(4) {
            sim.queue(WorldEdit::Paint { center: CellPos::new(x, ground - 4), radius: 4, material: oil, overwrite: false });
        }
        state.0 = 3;
    }
    let ground = state.3 as f32;
    // (wand key, aim, from, to): the fireball is hotbar 1 slot 7; spark 6;
    // acid 8; frost hotbar 2 slot 6.
    let plan: [(KeyCode, bool, Vec2, f32, f32); 6] = [
        (KeyCode::Digit7, false, home + Vec2::new(19.5, 81.0), 2.0, 2.1),
        (KeyCode::Digit7, false, Vec2::new(home.x + 165.0, ground), 3.5, 3.6),
        (KeyCode::Digit6, false, Vec2::new(home.x + 112.0, ground - 9.0), 5.0, 5.1),
        (KeyCode::Digit8, false, Vec2::new(home.x + 112.0, ground + 12.0), 6.0, 6.1),
        (KeyCode::Digit7, false, home + Vec2::new(34.5, 66.0), 7.0, 7.1),
        (KeyCode::Digit6, true, Vec2::new(home.x + 112.0, ground - 9.0), 8.5, 8.6),
    ];
    let now = plan.iter().find(|p| t >= p.3 - 0.3 && t < p.4);
    if let Some(&(key, bar2, aim, from, _)) = now {
        if t < from {
            // Pick the wand first (the second hotbar for frost, and back).
            if bar2 != (state.1 > 0.5) {
                keys.press(KeyCode::KeyX);
                state.1 = if bar2 { 1.0 } else { 0.0 };
            }
            keys.press(key);
        }
        cursor.0 = Some(aim);
        if t >= from { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    } else {
        mouse.release(MouseButton::Left);
        cursor.0 = Some(home + Vec2::new(30.0, 15.0));
    }
    if (t * 2.0).floor() != ((t - 0.017) * 2.0).floor() && state.0 >= 2 {
        let count = |n: &str| sim.materials().id(n).map_or(0, |m| sim.world.chunks().map(|c| c.cells().iter().filter(|c| c.material == m).count()).sum::<usize>());
        let burning: usize = sim.world.chunks().map(|c| c.cells().iter().filter(|c| c.flags & platypus_sim::cell::flags::BURNING != 0).count()).sum();
        let flying = sim.world.particles().iter().filter(|p| p.cell.material == water).count();
        info!("splash: t {t:.1} water {} (+{flying} flying) steam {} ice {} oil {} burning {burning}", count("water"), count("steam"), count("ice"), count("oil"));
    }
}

/// A long fall, saved (or not) by an air jump just above the ground; and
/// air jumps chained up (see the module notes).
fn airjump_script(
    s: Res<Scenario>,
    items: Option<Res<crate::hands::items::Items>>,
    mut worn: Query<&mut crate::gear::Equipment, With<LocalPlayer>>,
    mut player: Query<(&mut Kinematics, &crate::creatures::Health), With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<(Option<f32>, f32, u8, f32)>,
    mut day: ResMut<crate::light::Daylight>,
) {
    if s.name != "airjump" {
        return;
    }
    // PLATYPUS_NIGHT=1: at 23:00 (the cloud's glow in the dark).
    if std::env::var("PLATYPUS_NIGHT").is_ok_and(|v| !v.is_empty()) && state.0.is_none() {
        day.skipped = (23.0 - day.time * 24.0).rem_euclid(24.0);
    }
    let Ok((mut k, h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let ground = *state.0.get_or_insert(k.body.pos.y);
    // PLATYPUS_NOSAVE=1: no air jump before landing (it should hurt).
    let nosave = std::env::var("PLATYPUS_NOSAVE").is_ok_and(|v| !v.is_empty());
    // In cloud boots (the air jumps are theirs).
    if t < 0.5
        && let (Some(items), Ok(mut eq)) = (items.as_ref(), worn.single_mut())
        && let Some(b) = items.id("cloud_boots")
        && eq.worn[4].is_none_or(|s| s.item != b)
    {
        eq.worn[4] = Some(crate::hands::items::Stack::new(b, 1));
    }
    // Lifted 240 cells up at 1 s: a fall well past the safe height.
    if state.2 == 0 && t > 1.0 {
        k.body.pos.y += 240.0;
        k.prev_pos = k.body.pos;
        k.body.vel = Vec2::ZERO;
        state.2 = 1;
    }
    let low = k.body.pos.y - ground < 21.0;
    let save = !nosave && state.2 == 1 && k.body.vel.y < 0.0 && low;
    if save {
        state.2 = 2;
        state.3 = t;
    }
    let hold = state.2 == 2 && t - state.3 < 0.2;
    if hold { keys.press(KeyCode::Space) } else { keys.release(KeyCode::Space) }
    if t >= state.1 {
        state.1 = (t * 4.0).floor() / 4.0 + 0.25;
        info!("airjump: t {t:.2} height {:+.0} hp {:.0}{}", k.body.pos.y - ground, h.hp, if state.2 == 2 { " (saved)" } else { "" });
    }
}

/// Walk at a rabbit (see the module notes).
fn critters_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut commands: Commands,
    player: Query<&Kinematics, With<LocalPlayer>>,
    critters: Query<(&crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<(u8, f32)>,
) {
    if s.name != "critters" {
        return;
    }
    let Ok(k) = player.single() else { return };
    let t = s.elapsed;
    if state.0 == 0 && t > 1.0 {
        for (dx, kind) in [(90, "rabbit"), (112, "bird"), (135, "frog")] {
            let x = k.body.pos.x as i32 + dx;
            if let Some(y) = find_ground(&sim.world, x, k.body.pos.y as i32 + 60, 150) {
                crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(x as f32 + 0.5, y as f32), |_| {});
            }
        }
        state.0 = 1;
    }
    if t > 2.0 && t < 4.5 { keys.press(KeyCode::KeyD) } else { keys.release(KeyCode::KeyD) }
    if t >= state.1 {
        state.1 = (t * 2.0).floor() / 2.0 + 0.5;
        let near: Vec<String> = critters.iter().filter(|(c, r)| c.kind != "orc" && r.body.pos.distance(k.body.pos) < 750.0).map(|(c, r)| format!("{} {:+.0},{:+.0}", c.kind, r.body.pos.x - k.body.pos.x, r.body.pos.y - k.body.pos.y)).collect();
        info!("critters: t {t:.1} [{}]", near.join("; "));
    }
}

/// The arena's tools through their own messages, the wands through real
/// keys and buttons.
#[allow(clippy::too_many_arguments)]
fn arena_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    dummies: Query<(&crate::creatures::Creature, &Kinematics, &crate::creatures::custom::dummy::Tally)>,
    mut arena: MessageWriter<crate::arena::ArenaAction>,
    mut dev: MessageWriter<crate::dev::DevAction>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, u64, f32)>,
    mut logged: Local<f32>,
) {
    use crate::arena::ArenaAction as A;
    if s.name != "arena" {
        return;
    }
    let Ok(k) = player.single() else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    let tick = sim.world.tick();
    // Each step once, in order.
    let beats: [(f32, u8); 9] = [(0.5, 1), (4.6, 2), (5.0, 3), (5.2, 4), (5.4, 5), (5.6, 6), (5.8, 7), (6.0, 8), (8.0, 9)];
    for (at, n) in beats {
        if t < at || state.0 >= n {
            continue;
        }
        state.0 = n;
        match n {
            1 => {
                arena.write(A::Overlays);
            }
            2 => {
                arena.write(A::Pick("orc".into()));
                dev.write(crate::dev::DevAction::Spawn(Some(p + Vec2::new(-120.0, 15.0))));
            }
            3 => {
                arena.write(A::Pause);
                state.1 = tick;
            }
            4..=6 => {
                arena.write(A::Step);
            }
            7 => {
                info!("arena: paused and stepped 3 times: {} ticks", tick - state.1);
                arena.write(A::Pause);
            }
            8 => {
                arena.write(A::Speed(0.25));
                state.1 = tick;
                state.2 = t;
            }
            _ => {
                info!("arena: at 1/4 speed: {:.1} ticks a second", (tick - state.1) as f32 / (t - state.2));
                arena.write(A::Speed(1.0));
            }
        }
    }
    const SLOTS: [KeyCode; 2] = [KeyCode::Digit6, KeyCode::Digit7];
    for key in SLOTS {
        keys.release(key);
    }
    let dummy = |x: f32| dummies.iter().map(|(_, dk, _)| dk.body.pos).find(|d| (d.x - x).abs() < 12.0);
    let aim = match t {
        t if (1.0..3.0).contains(&t) => {
            keys.press(SLOTS[0]);
            dummy(1050.0)
        }
        t if (3.0..4.4).contains(&t) => {
            keys.press(SLOTS[1]);
            dummy(1140.0)
        }
        _ => None,
    };
    cursor.0 = aim.or(Some(p + Vec2::new(45.0, 0.0)));
    if aim.is_some() { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    if t >= *logged {
        *logged = (t * 2.0).floor() / 2.0 + 0.5;
        let read: Vec<String> = dummies.iter().map(|(c, dk, tl)| format!("{} @{:.0}: {:.0} dps, {:.0} in {} hits", c.kind, dk.body.pos.x, tl.dps(), tl.total, tl.hits)).collect();
        info!("arena: t {t:.1} [{}]", read.join("; "));
    }
}

/// The art editor through the real pointer and keys, on a scratch copy.
#[allow(clippy::too_many_arguments)]
fn editor_script(
    s: Res<Scenario>,
    mut window: Single<&mut Window, With<bevy::window::PrimaryWindow>>,
    canvas: Query<(&bevy::ui::UiGlobalTransform, &bevy::ui::ComputedNode), With<crate::editor::Canvas>>,
    editor: Res<crate::editor::Editor>,
    mut arena: MessageWriter<crate::arena::ArenaAction>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, String)>,
) {
    if s.name != "editor" {
        return;
    }
    let t = s.elapsed;
    let shown = editor.canvas();
    let read = || shown.as_ref().and_then(|(_, f)| std::fs::read_to_string(f).ok()).unwrap_or_default();
    let scale = window.scale_factor();
    // Pixel (x, y) of the canvas, in window pixels.
    let at = |x: f32, y: f32| {
        let ((w, h), _) = shown.clone()?;
        canvas.single().ok().map(|(tf, node)| {
            let size = node.size() / scale;
            tf.translation / scale + Vec2::new((x + 0.5) / w as f32 - 0.5, (y + 0.5) / h as f32 - 0.5) * size
        })
    };
    let beats = [0.5, 1.0, 1.2, 1.3, 1.4, 1.5, 1.8, 2.0, 2.2, 2.6];
    let Some(n) = beats.iter().rposition(|&b| t >= b) else { return };
    let n = n as u8 + 1;
    if state.0 >= n {
        return;
    }
    state.0 = n;
    match n {
        1 => {
            arena.write(crate::arena::ArenaAction::Editor);
        }
        2 => {
            state.1 = read();
            window.set_cursor_position(at(9.0, 14.0));
        }
        3 => mouse.press(MouseButton::Left),
        4 => window.set_cursor_position(at(9.0, 15.0)),
        5 => window.set_cursor_position(at(9.0, 16.0)),
        6 => mouse.release(MouseButton::Left),
        7 => {
            let now = read();
            let changed = state.1.chars().zip(now.chars()).filter(|(a, b)| a != b).count();
            info!("editor: after one stroke, {changed} characters of the file changed ({} lines before, {} after)", state.1.lines().count(), now.lines().count());
        }
        8 => {
            keys.press(KeyCode::ControlLeft);
            keys.press(KeyCode::KeyZ);
        }
        9 => {
            keys.release(KeyCode::ControlLeft);
            keys.release(KeyCode::KeyZ);
        }
        _ => {
            info!("editor: after Ctrl+Z the file is as it was: {}", read() == state.1);
            window.set_cursor_position(at(7.0, 9.0));
        }
    }
}

/// The spark and flame wands against the arena's floor, sandbag and dummies.
#[allow(clippy::too_many_arguments)]
fn wands_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    dummies: Query<(&crate::creatures::Creature, &Kinematics, &crate::creatures::custom::dummy::Tally)>,
    mut commands: Commands,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, usize, f32)>,
) {
    if s.name != "wands" {
        return;
    }
    let Ok(k) = player.single() else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    // Solid cells of the floor's top 18 rows around x 1020.
    let solid = || (990..1050).flat_map(|x| (floor - 18..floor).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| !c.is_air())).count();
    // (The sandbag nearest the player: one is put 60 cells off at 2 s.)
    let bag = || dummies.iter().filter(|(c, ..)| c.kind == "sandbag").map(|(_, dk, _)| dk.body.pos).min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)));
    let dummy = |x: f32| dummies.iter().find(|(c, dk, _)| c.kind == "dummy" && (dk.body.pos.x - x).abs() < 12.0);
    for key in [KeyCode::Digit6, KeyCode::Digit9] {
        keys.release(key);
    }
    let aim = match t {
        t if (0.8..1.0).contains(&t) => {
            state.1 = solid();
            None
        }
        t if (1.0..2.0).contains(&t) => {
            keys.press(KeyCode::Digit6);
            Some(Vec2::new(1020.0, floor as f32 - 3.0))
        }
        t if (2.0..2.2).contains(&t) => {
            if state.0 == 0 {
                info!("wands: the spark broke {} floor cells in a second", state.1 as i64 - solid() as i64);
                crate::creatures::def::spawn_creature(&mut commands, "sandbag", Vec2::new(p.x + 60.0, floor as f32), |_| {});
                state.0 = 1;
            }
            None
        }
        t if (2.2..3.2).contains(&t) => {
            if state.2 == 0.0 {
                state.2 = bag().map_or(0.0, |b| b.x);
            }
            keys.press(KeyCode::Digit6);
            bag()
        }
        t if (3.2..3.4).contains(&t) => {
            if state.0 == 1 {
                info!("wands: the sandbag went from x {:.0} to {:?} (a second of sparks, {:?} hits)", state.2, bag().map(|b| b.x.round()), dummies.iter().filter(|(c, ..)| c.kind == "sandbag").map(|d| d.2.hits).max());
                state.0 = 2;
            }
            None
        }
        t if (3.4..5.0).contains(&t) => {
            keys.press(KeyCode::Digit9);
            dummy(1050.0).map(|d| d.1.body.pos)
        }
        t if (5.0..6.6).contains(&t) => {
            keys.press(KeyCode::Digit9);
            dummy(1140.0).map(|d| d.1.body.pos)
        }
        _ => {
            if state.0 == 2 {
                let took = |x: f32| dummy(x).map_or(0.0, |d| d.2.total);
                info!("wands: flames from x {:.0}: the dummy 90 off took {:.0}, the one 180 off {:.0}", p.x, took(1050.0), took(1140.0));
                state.0 = 3;
            }
            None
        }
    };
    cursor.0 = aim.or(Some(p + Vec2::new(45.0, 0.0)));
    if aim.is_some() { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
}

type Fighting<'a> = (&'a Kinematics, &'a crate::creatures::Health, Option<&'a crate::combat::Stamina>, Option<&'a crate::combat::Wielding>);

/// The swords through real keys and buttons, against the first dummy.
#[allow(clippy::too_many_arguments)]
fn melee_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    player: Query<Fighting, With<LocalPlayer>>,
    dummies: Query<(&crate::creatures::Creature, &Kinematics, &crate::creatures::custom::dummy::Tally), Without<LocalPlayer>>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, f32, f32, f32)>,
) {
    if s.name != "melee" {
        return;
    }
    if s.elapsed < 0.1 {
        state.4 = 100.0;
        return;
    }
    let Ok((k, h, stamina, wielding)) = player.single() else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    let Some((_, dk, tally)) = dummies.iter().find(|(c, dk, _)| c.kind == "dummy" && (dk.body.pos.x - 1050.0).abs() < 12.0) else { return };
    let d = dk.body.pos;
    // Each key held while wanted, so a press is one press.
    let mut want = std::collections::HashSet::new();
    let window = |a: f32, b: f32| t >= a && t < b;
    if window(0.3, 0.4) {
        want.insert(KeyCode::KeyX);
    }
    // (Hotbar 2: the shortsword at 8, the longsword at 9.)
    if window(0.5, 0.6) {
        want.insert(KeyCode::Digit8);
    }
    if window(2.5, 2.6) {
        want.insert(KeyCode::Digit9);
    }
    let stam = stamina.map_or(0.0, |s| s.cur);
    let held = wielding.and_then(|w| w.0.clone()).unwrap_or_default();
    let mut swing = None;
    if window(0.6, 2.4) {
        if d.x - p.x > 18.0 {
            want.insert(KeyCode::KeyD);
        }
        if t > 0.9 {
            swing = Some(d + Vec2::new(0.0, 3.0));
        }
        state.4 = state.4.min(stam);
    }
    if window(2.4, 2.5) && state.0 == 0 {
        info!("melee: {held}: {} hits, {:.0} damage in {:.1}s; stamina went down to {:.0}", tally.hits, tally.total, tally.last - tally.start, state.4);
        state.0 = 1;
        state.1 = tally.total;
        state.4 = 100.0;
    }
    if window(2.7, 4.2) {
        swing = Some(d + Vec2::new(0.0, 3.0));
        state.4 = state.4.min(stam);
    }
    if window(4.2, 4.6) {
        if state.0 == 1 {
            info!("melee: {held}: {:.0} more damage; stamina went down to {:.0}", tally.total - state.1, state.4);
            state.0 = 2;
        }
        want.insert(KeyCode::KeyA);
    }
    if window(4.6, 5.8) {
        if t < 4.9 {
            want.insert(KeyCode::Space);
        }
        if p.x < d.x - 3.0 {
            want.insert(KeyCode::KeyD);
        }
        // Over it: strike down; how fast it rises after is the pogo.
        if !k.loco.grounded() && p.y > d.y + 18.0 {
            swing = Some(d);
            state.3 = f32::max(state.3, tally.hits as f32);
        }
        if t > 5.0 {
            state.2 = state.2.max(k.body.vel.y);
        }
    }
    if window(5.8, 5.9) && state.0 == 2 {
        info!("melee: striking down from above: the fastest rise after 5 s {:.0} cells/s", state.2);
        state.0 = 3;
    }
    // A blast in the middle of a dodge, then the same standing.
    if window(6.2, 6.3) {
        want.insert(KeyCode::ShiftLeft);
    }
    if t > 6.26 && state.0 == 3 {
        state.2 = h.hp;
        sim.queue(WorldEdit::Explode { center: CellPos::new(p.x as i32 + 6, p.y as i32), radius: 6, power: 60 });
        state.0 = 4;
    }
    if t > 6.9 && state.0 == 4 {
        info!("melee: a blast mid-dodge cost {:.0} hp", state.2 - h.hp);
        state.2 = h.hp;
        sim.queue(WorldEdit::Explode { center: CellPos::new(p.x as i32 + 6, p.y as i32), radius: 6, power: 60 });
        state.0 = 5;
    }
    if t > 7.4 && state.0 == 5 {
        info!("melee: the same blast standing cost {:.0} hp", state.2 - h.hp);
        state.0 = 6;
    }
    for key in [KeyCode::KeyX, KeyCode::Digit8, KeyCode::Digit9, KeyCode::KeyD, KeyCode::KeyA, KeyCode::Space, KeyCode::ShiftLeft] {
        match (want.contains(&key), keys.pressed(key)) {
            (true, false) => keys.press(key),
            (false, true) => keys.release(key),
            _ => {}
        }
    }
    cursor.0 = swing.or(Some(p + Vec2::new(45.0, 0.0)));
    match (swing.is_some(), mouse.pressed(MouseButton::Left)) {
        (true, false) => mouse.press(MouseButton::Left),
        (false, true) => mouse.release(MouseButton::Left),
        _ => {}
    }
}

/// The player against an orc, then a troll, through real keys and buttons.
#[allow(clippy::too_many_arguments)]
fn fight_script(
    s: Res<Scenario>,
    mut commands: Commands,
    player: Query<(&Kinematics, &crate::creatures::Health), With<LocalPlayer>>,
    foes: Query<(&crate::creatures::Creature, &Kinematics, &crate::creatures::Health, Has<crate::combat::Swing>), Without<LocalPlayer>>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, u32, u32)>,
) {
    if s.name != "fight" {
        return;
    }
    let Ok((k, h)) = player.single() else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR as f32;
    let mut want = std::collections::HashSet::new();
    if (0.3..0.4).contains(&t) {
        want.insert(KeyCode::KeyX);
    }
    if (0.5..0.6).contains(&t) {
        want.insert(KeyCode::Digit8);
    }
    if state.0 == 0 && t > 0.8 {
        crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(p.x - 60.0, floor), |_| {});
        state.0 = 1;
    }
    if state.0 == 1 && t > 5.0 {
        crate::creatures::def::spawn_creature(&mut commands, "troll", Vec2::new(p.x - 105.0, floor), |_| {});
        state.0 = 2;
    }
    let near = foes.iter().filter(|(c, ..)| c.kind == "orc" || c.kind == "troll").min_by(|a, b| a.1.body.pos.distance(p).total_cmp(&b.1.body.pos.distance(p)));
    let swing = near.filter(|(_, fk, ..)| t > 1.5 && fk.body.pos.distance(p) < 45.0).map(|(_, fk, ..)| fk.body.pos);
    for (_, _, _, swinging) in &foes {
        if swinging {
            state.3 += 1;
        }
    }
    if t >= state.1 {
        state.1 = (t * 2.0).floor() / 2.0 + 0.5;
        let them: Vec<String> = foes.iter().filter(|(c, ..)| c.kind == "orc" || c.kind == "troll").map(|(c, fk, fh, sw)| format!("{} {:+.0} hp {:.0}{}", c.kind, fk.body.pos.x - p.x, fh.hp, if sw { " swinging" } else { "" })).collect();
        info!("fight: t {t:.1} player hp {:.0} [{}] ({} ticks of enemy swings so far)", h.hp, them.join("; "), state.3);
    }
    for key in [KeyCode::KeyX, KeyCode::Digit8] {
        match (want.contains(&key), keys.pressed(key)) {
            (true, false) => keys.press(key),
            (false, true) => keys.release(key),
            _ => {}
        }
    }
    cursor.0 = swing.or(Some(p + Vec2::new(-45.0, 0.0)));
    match (swing.is_some(), mouse.pressed(MouseButton::Left)) {
        (true, false) => mouse.press(MouseButton::Left),
        (false, true) => mouse.release(MouseButton::Left),
        _ => {}
    }
}

/// The bow through real keys and buttons; then an orc archer.
#[allow(clippy::too_many_arguments)]
fn archery_script(
    s: Res<Scenario>,
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    player: Query<(&Kinematics, &crate::creatures::Health, &crate::hands::items::Inventory), With<LocalPlayer>>,
    dummies: Query<(&crate::creatures::Creature, &Kinematics, &crate::creatures::custom::dummy::Tally)>,
    arrows: Query<&crate::archery::Arrow>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, u32)>,
) {
    if s.name != "archery" {
        return;
    }
    let (Ok((k, h, inv)), Some(items)) = (player.single(), items) else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR as f32;
    let quiver = items.id("arrow").map_or(0, |a| inv.count(a));
    let mut want = std::collections::HashSet::new();
    if (0.3..0.4).contains(&t) {
        want.insert(KeyCode::KeyX);
    }
    if (0.5..0.6).contains(&t) {
        want.insert(KeyCode::Digit9);
    }
    let dummy = dummies.iter().find(|(c, dk, _)| c.kind == "dummy" && (dk.body.pos.x - 1050.0).abs() < 12.0);
    let mut aim = None;
    if (1.0..1.8).contains(&t) {
        aim = dummy.map(|d| d.1.body.pos + Vec2::new(0.0, 6.0));
    }
    if (2.0..2.2).contains(&t) {
        if state.0 == 0 {
            info!("archery: a full draw at the dummy: it took {:.0} ({} hits); {quiver} arrows left", dummy.map_or(0.0, |d| d.2.total), dummy.map_or(0, |d| d.2.hits));
            state.0 = 1;
        }
        aim = Some(Vec2::new(p.x + 45.0, floor - 3.0));
    }
    // A puddle of lava to shoot down through.
    if (2.3..2.32).contains(&t) && state.0 == 1 {
        if let Some(lava) = sim.materials().id("lava") {
            sim.queue(WorldEdit::Paint { center: CellPos::new(p.x as i32 - 36, floor as i32 + 2), radius: 3, material: lava, overwrite: false });
        }
        state.0 = 2;
    }
    if (2.65..3.3).contains(&t) {
        aim = Some(Vec2::new(p.x - 37.5, floor - 3.0));
    }
    if (3.4..3.42).contains(&t) {
        info!("archery: through the fire: {} of {} arrows burning", arrows.iter().filter(|a| a.burning()).count(), arrows.iter().count());
    }
    if (3.5..3.6).contains(&t) && state.0 == 2 {
        let stuck = arrows.iter().count();
        info!("archery: {stuck} arrows about, {quiver} in the quiver");
        state.0 = 3;
        state.2 = quiver;
    }
    if (3.6..5.0).contains(&t) && p.x < 1050.0 - 18.0 {
        want.insert(KeyCode::KeyD);
    }
    if (5.2..5.3).contains(&t) && state.0 == 3 {
        info!("archery: walked over them: {quiver} in the quiver (was {}), {} arrows about", state.2, arrows.iter().count());
        crate::creatures::def::spawn_creature(&mut commands, "orc_archer", Vec2::new(p.x - 165.0, floor), |_| {});
        state.0 = 4;
        state.1 = h.hp;
    }
    if t > 9.5 && state.0 == 4 {
        info!("archery: 4 s of the orc archer cost {:.0} hp", state.1 - h.hp);
        state.0 = 5;
    }
    for key in [KeyCode::KeyX, KeyCode::Digit9, KeyCode::KeyD] {
        match (want.contains(&key), keys.pressed(key)) {
            (true, false) => keys.press(key),
            (false, true) => keys.release(key),
            _ => {}
        }
    }
    cursor.0 = aim.or(Some(p + Vec2::new(45.0, 0.0)));
    match (aim.is_some(), mouse.pressed(MouseButton::Left)) {
        (true, false) => mouse.press(MouseButton::Left),
        (false, true) => mouse.release(MouseButton::Left),
        _ => {}
    }
}

/// O spawns a pack.
fn warband_script(
    s: Res<Scenario>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    foes: Query<&crate::creatures::Creature, Without<LocalPlayer>>,
    mut dev: MessageWriter<crate::dev::DevAction>,
    mut state: Local<u8>,
) {
    if s.name != "warband" {
        return;
    }
    let Ok(k) = player.single() else { return };
    if *state == 0 && s.elapsed > 1.0 {
        dev.write(crate::dev::DevAction::Spawn(Some(k.body.pos + Vec2::new(180.0, 30.0))));
        *state = 1;
    }
    if *state == 1 && s.elapsed > 3.0 {
        let mut n = std::collections::BTreeMap::new();
        for c in &foes {
            *n.entry(c.kind.clone()).or_insert(0) += 1;
        }
        info!("warband: {n:?}");
        *state = 2;
    }
}

/// Each tool in the hand, through real keys and buttons.
#[allow(clippy::too_many_arguments)]
fn held_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    player: Query<(&Kinematics, Option<&crate::combat::Swing>, &crate::combat::Wielding), With<LocalPlayer>>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, usize, u32, bool)>,
) {
    if s.name != "held" {
        return;
    }
    let Ok((k, swing, wielding)) = player.single() else { return };
    let p = k.body.pos;
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let solid = || (960..1020).flat_map(|x| (floor - 24..floor).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| !c.is_air())).count();
    // Swings started (a swing seen after none).
    let swinging = swing.is_some();
    if swinging && !state.3 {
        state.2 += 1;
    }
    state.3 = swinging;
    let mut want = std::collections::HashSet::new();
    let press = |a: f32, want: &mut std::collections::HashSet<KeyCode>, k: KeyCode| {
        if (a..a + 0.1).contains(&t) {
            want.insert(k);
        }
    };
    press(0.3, &mut want, KeyCode::Digit1);
    press(2.2, &mut want, KeyCode::Digit3);
    press(3.0, &mut want, KeyCode::Digit6);
    press(4.0, &mut want, KeyCode::Digit4);
    press(4.8, &mut want, KeyCode::Digit2);
    if (0.4..0.5).contains(&t) {
        state.1 = solid();
    }
    let aim = match t {
        t if (0.5..2.0).contains(&t) => Some(Vec2::new(p.x + 21.0, floor as f32 - 4.5)),
        t if (3.2..3.8).contains(&t) => Some(Vec2::new(1050.0, floor as f32 + 15.0)),
        t if (4.3..4.35).contains(&t) => Some(p + Vec2::new(90.0, 60.0)),
        t if (5.0..5.6).contains(&t) => Some(p + Vec2::new(30.0, 7.5)),
        _ => None,
    };
    if (2.05..2.1).contains(&t) && state.0 == 0 {
        info!("held: the pickaxe ({:?}) dug {} cells in 1.5 s over {} swings", wielding.0, state.1 as i64 - solid() as i64, state.2);
        state.0 = 1;
    }
    for (at, name) in [(2.7, "torch"), (3.5, "wand"), (4.2, "bomb"), (5.3, "axe")] {
        if (at..at + 0.02).contains(&t) {
            info!("held: {name}: wielding {:?}, swinging {}", wielding.0, swinging);
        }
    }
    for key in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit6] {
        match (want.contains(&key), keys.pressed(key)) {
            (true, false) => keys.press(key),
            (false, true) => keys.release(key),
            _ => {}
        }
    }
    cursor.0 = aim.or(Some(p + Vec2::new(45.0, 0.0)));
    match (aim.is_some(), mouse.pressed(MouseButton::Left)) {
        (true, false) => mouse.press(MouseButton::Left),
        (false, true) => mouse.release(MouseButton::Left),
        _ => {}
    }
}

/// Through falling sand; out of a pit of water.
fn crossing_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<(u8, f32, f32)>,
) {
    if s.name != "crossing" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let mut want = std::collections::HashSet::new();
    if state.0 == 0 && t > 0.3 {
        state.1 = k.body.pos.x;
        state.0 = 1;
    }
    let x0 = state.1 as i32;
    // Sand pouring from high up, 45 cells ahead.
    if (0.3..3.0).contains(&t)
        && let Some(sand) = sim.materials().id("sand")
    {
        sim.queue(WorldEdit::Paint { center: CellPos::new(x0 + 45, floor + 105), radius: 2, material: sand, overwrite: false });
    }
    if (1.2..3.0).contains(&t) {
        want.insert(KeyCode::KeyD);
    }
    if state.0 == 1 && t > 3.0 {
        info!("crossing: walked {:.0} cells through a sand stream 45 ahead (past it: {})", k.body.pos.x - state.1, k.body.pos.x - state.1 > 54.0);
        // A pit: 45 wide, 60 deep, water to 18 below the rim.
        let px = x0 + 135;
        sim.queue(WorldEdit::Dig { center: CellPos::new(px, floor - 30), radius: 30, max_hardness: 250 });
        if let Some(water) = sim.materials().id("water") {
            for y in (floor - 60..floor - 18).step_by(6) {
                sim.queue(WorldEdit::Paint { center: CellPos::new(px, y), radius: 24, material: water, overwrite: false });
            }
        }
        state.0 = 2;
    }
    if state.0 == 2 && t > 3.6 {
        k.body.pos = Vec2::new((x0 + 135) as f32, (floor - 33) as f32);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
        state.0 = 3;
        state.2 = f32::MIN;
    }
    if state.0 == 3 {
        // Swim up (hold jump), then at the surface jump for the left bank.
        let wet = k.loco.contacts.submerged;
        if t < 7.5 {
            if wet > 0.0 || t < 4.0 {
                want.insert(KeyCode::Space);
            }
            if t > 4.5 {
                want.insert(KeyCode::KeyA);
            }
            // (Let go now and then, so a new press can come.)
            if (t * 3.0).fract() < 0.15 {
                want.remove(&KeyCode::Space);
            }
        }
        state.2 = state.2.max(k.body.bottom());
        if t > 7.5 {
            info!("crossing: out of the pit: feet at {:+.0} from the rim (highest {:+.0}), {}", k.body.bottom() - floor as f32, state.2 - floor as f32, if k.body.bottom() >= floor as f32 - 0.75 && k.loco.contacts.submerged < 0.1 { "out" } else { "still in" });
            state.0 = 4;
        }
    }
    for key in [KeyCode::KeyD, KeyCode::KeyA, KeyCode::Space] {
        match (want.contains(&key), keys.pressed(key)) {
            (true, false) => keys.press(key),
            (false, true) => keys.release(key),
            _ => {}
        }
    }
}

/// Fireflies, fish and bats put where they live, and checked on.
fn life_script(
    s: Res<Scenario>,
    mut commands: Commands,
    sim: Res<SimWorld>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    critters: Query<(&crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    mut state: Local<u8>,
) {
    if s.name != "life" {
        return;
    }
    let floor = platypus_worldgen::arena::FLOOR as f32;
    if *state == 0 && s.elapsed > 0.5 {
        // The player by the pool, to watch.
        if let Ok(mut k) = player.single_mut() {
            k.body.pos = Vec2::new(660.0, floor + 12.0);
            k.prev_pos = k.body.pos;
        }
        for i in 0..6 {
            let x = 690.0 + i as f32 * 18.0;
            crate::creatures::def::spawn_creature(&mut commands, "firefly", Vec2::new(x, floor + 12.0 + (i % 3) as f32 * 9.0), |_| {});
        }
        for i in 0..4 {
            crate::creatures::def::spawn_creature(&mut commands, "fish", Vec2::new(495.0 + i as f32 * 33.0, floor - 30.0 - (i % 2) as f32 * 18.0), |_| {});
        }
        for i in 0..4 {
            crate::creatures::def::spawn_creature(&mut commands, "bat", Vec2::new(750.0 + i as f32 * 21.0, floor + 60.0 + i as f32 * 12.0), |_| {});
        }
        *state = 1;
    }
    if *state == 1 && s.elapsed > 5.5 {
        let world = &sim.world;
        let wet = |p: Vec2| world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| world.materials().phys(c.material).kind == platypus_sim::Kind::Liquid);
        for kind in ["firefly", "fish", "bat"] {
            let all: Vec<&Kinematics> = critters.iter().filter(|(c, _)| c.kind == kind).map(|(_, k)| k).collect();
            let fine = all
                .iter()
                .filter(|k| match kind {
                    "fish" => wet(k.body.pos),
                    _ => !k.loco.grounded() && k.body.pos.y > floor + 3.0,
                })
                .count();
            let heights: Vec<String> = all.iter().map(|k| format!("{:.0}", k.body.pos.y - floor)).collect();
            info!("life: {kind}: {fine} of {} where they live (heights over the floor: {})", all.len(), heights.join(" "));
        }
        *state = 2;
    }
}

/// A phase: its name, what's put (kind, dx from the player), how long.
type Phase<'a> = (&'a str, &'a [(&'a str, f32)], f32);

/// Creatures but the player and the arena's dummies.
type Foes = (Without<LocalPlayer>, Without<crate::creatures::custom::dummy::Dummy>);

/// The underground enemies, one kind at a time, against a player who stands.
#[allow(clippy::too_many_arguments)]
fn underground_script(
    s: Res<Scenario>,
    mut commands: Commands,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    foes: Query<(Entity, &crate::creatures::Creature, &Kinematics), Foes>,
    deaths: Res<crate::creatures::PlayerDeaths>,
    mut state: Local<(usize, f32, f32, u32, f32)>,
) {
    if s.name != "underground" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR as f32;
    // (name, what: (kind, dx from the player), how long)
    let phases: [Phase; 6] = [
        ("spider", &[("spider", 60.0)], 3.5),
        ("slimes", &[("slime", 52.0), ("acid_slime", -52.0)], 3.5),
        ("vampire bats", &[("vampire_bat", 45.0), ("vampire_bat", -45.0)], 4.0),
        ("skeleton", &[("skeleton", 60.0)], 4.0),
        ("egg sac", &[("egg_sac", 42.0)], 3.0),
        ("climb", &[], 5.0),
    ];
    let start: Vec<f32> = phases.iter().scan(0.5, |acc, p| { let a = *acc; *acc += p.2; Some(a) }).collect();
    let (i, _) = (state.0, 0);
    if i >= phases.len() {
        return;
    }
    if t < start[i] {
        return;
    }
    // A phase begins: the floor cleared, the player healed, its foes put.
    if state.4 < start[i] + 0.001 && state.4 <= start[i] {
        for (e, ..) in &foes {
            commands.entity(e).despawn();
        }
        h.hp = h.max;
        state.1 = h.hp;
        state.3 = deaths.0;
        state.2 = 0.0;
        let (name, what, _) = phases[i];
        let at = if name == "climb" {
            // On top of the tall column, a spider at its foot.
            k.body.pos = Vec2::new(1662.0, floor + 210.0 + 12.0);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            crate::creatures::def::spawn_creature(&mut commands, "spider", Vec2::new(1628.0, floor), |_| {});
            k.body.pos
        } else {
            k.body.pos
        };
        for &(kind, dx) in what {
            crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(at.x + dx, floor + if kind == "vampire_bat" { 60.0 } else { 0.0 }), |_| {});
        }
        state.4 = start[i] + 0.002;
    }
    // Keep a record: the highest a foe got (the climb).
    for (_, c, fk) in &foes {
        if c.kind == "spider" {
            state.2 = state.2.max(fk.body.pos.y - floor);
        }
    }
    let end = start[i] + phases[i].2;
    if t >= end - 0.05 {
        let lost = state.1 - h.hp + 100.0 * (deaths.0 - state.3) as f32;
        let mut kinds = std::collections::BTreeMap::new();
        for (_, c, _) in &foes {
            *kinds.entry(c.kind.clone()).or_insert(0) += 1;
        }
        let extra = if phases[i].0 == "climb" { format!("; the spider got {:.0} cells up (the player is at {:.0})", state.2, k.body.pos.y - floor) } else { String::new() };
        info!("underground: {}: the player lost {lost:.0} hp in {:.1} s; about: {kinds:?}{extra}; the player ends at ({:.0}, {:+.0} from the floor)", phases[i].0, phases[i].2, k.body.pos.x, k.body.pos.y - k.body.half.y - floor);
        state.0 += 1;
        if state.0 < phases.len() {
            state.4 = 0.0;
        }
    }
}

/// Into the nearest spider nest, torch in hand.
fn nest_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut toggles: ResMut<crate::light::LightToggles>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    foes: Query<(&crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    mut state: Local<(u8, Option<Vec2>)>,
) {
    if s.name != "nest" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    if state.0 == 0 && s.elapsed > 0.3 {
        let home = sim.generator.spawn_point();
        let nest = sim
            .generator
            .landmarks()
            .into_iter()
            .filter(|(_, n)| n == "spider nest")
            .min_by_key(|(p, _)| (p.x - home.x).abs() + (p.y - home.y).abs());
        match nest {
            Some((p, _)) => {
                info!("nest: a spider nest at {},{} ({} cells down from the start)", p.x, p.y, home.y - p.y);
                let at = Vec2::new(p.x as f32 - 30.0, p.y as f32);
                k.body.pos = at;
                k.body.vel = Vec2::ZERO;
                k.prev_pos = at;
                state.1 = Some(at);
            }
            None => info!("nest: no spider nest in this world"),
        }
        toggles.carry = crate::light::Carry::Torch;
        state.0 = 1;
    }
    // (Held there while its chunks load, so it doesn't fall away first.)
    if state.0 == 1 && s.elapsed < 1.5
        && let Some(at) = state.1
    {
        k.body.pos = at;
        k.body.vel = Vec2::ZERO;
    }
    if state.0 == 1 && s.elapsed > 3.3 {
        let mut n = std::collections::BTreeMap::new();
        for (c, fk) in &foes {
            if fk.body.pos.distance(k.body.pos) < 225.0 {
                *n.entry(c.kind.clone()).or_insert(0) += 1;
            }
        }
        info!("nest: about the player: {n:?}");
        state.0 = 2;
    }
}

/// How far a walk gets through open air, then through web.
fn webs_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, mut player: Query<&mut Kinematics, With<LocalPlayer>>, mut keys: ResMut<ButtonInput<KeyCode>>, mut state: Local<(u8, f32)>) {
    if s.name != "webs" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let walking = (1.0..2.5).contains(&t) || (3.5..5.0).contains(&t);
    match (walking, keys.pressed(KeyCode::KeyD)) {
        (true, false) => keys.press(KeyCode::KeyD),
        (false, true) => keys.release(KeyCode::KeyD),
        _ => {}
    }
    if state.0 == 0 && t > 1.0 {
        state.1 = k.body.pos.x;
        state.0 = 1;
    }
    if state.0 == 1 && t > 2.5 {
        info!("webs: open air: {:.0} cells in 1.5 s", k.body.pos.x - state.1);
        // Back, and a thicket of web ahead.
        k.body.pos.x = 840.0;
        k.prev_pos = k.body.pos;
        if let Some(web) = sim.materials().id("cobweb") {
            for dx in (0..90).step_by(12) {
                sim.queue(WorldEdit::Paint { center: CellPos::new(862 + dx, floor + 12), radius: 12, material: web, overwrite: false });
            }
        }
        state.0 = 2;
    }
    if state.0 == 2 && t > 3.5 {
        state.1 = k.body.pos.x;
        state.0 = 3;
    }
    if state.0 == 3 && t > 5.0 {
        info!("webs: through web: {:.0} cells in 1.5 s", k.body.pos.x - state.1);
        state.0 = 4;
    }
}

type GearTester<'a> = (Entity, &'a mut Kinematics, &'a mut crate::creatures::Health, &'a mut crate::gear::Equipment, &'a crate::gear::Stats, &'a mut crate::hands::items::Inventory);

/// Armour against a blast, and the equipment screen.
#[allow(clippy::too_many_arguments)]
fn gear_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    rules: Res<crate::gear::GearRules>,
    mut window: Single<&mut Window, With<bevy::window::PrimaryWindow>>,
    slots: Query<(&crate::hands::ui::SlotUi, &bevy::ui::UiGlobalTransform, &InheritedVisibility)>,
    mut player: Query<GearTester, With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut grip: ResMut<crate::hands::ui::Held>,
    mut state: Local<(u8, f32)>,
) {
    use crate::hands::items::Stack;
    if s.name != "gear" {
        return;
    }
    let (Some(items), Ok((me, mut k, mut h, mut eq, stats, mut inv))) = (items, player.single_mut()) else { return };
    let t = s.elapsed;
    let p = k.body.pos;
    keys.release(KeyCode::Escape);
    let blast = |sim: &mut SimWorld| sim.queue(WorldEdit::Explode { center: CellPos::new(p.x as i32 + 6, p.y as i32), radius: 6, power: 60 });
    match state.0 {
        0 if t > 0.8 => {
            state.1 = h.hp;
            blast(&mut sim);
            state.0 = 1;
        }
        1 if t > 1.4 => {
            info!("gear: the blast with nothing on cost {:.0} hp", state.1 - h.hp);
            h.hp = h.max;
            for (i, id) in ["iron_helm", "chainmail", "iron_gauntlets", "iron_greaves", "iron_boots", "ring_of_vigour"].iter().enumerate() {
                eq.worn[i] = items.id(id).map(|it| Stack::new(it, 1));
            }
            if let Some(j) = items.id("leather_jerkin") {
                inv.add(&items, Stack::new(j, 1));
            }
            // One of each rarity, rolled at item level 30.
            for (rarity, id) in ["iron_helm", "leather_boots", "chainmail", "longsword", "ember_amulet"].iter().enumerate() {
                let Some(item) = items.id(id) else { continue };
                let roll = crate::hands::items::Roll { rarity: rarity as u8, level: 30, seed: 977 + rarity as u32 };
                let stack = Stack { roll, ..Stack::new(item, 1) };
                let bonuses: Vec<String> = rules.bonuses(&items, &stack).iter().map(|&(s, v)| crate::gear::stats::line(s, v)).collect();
                info!("gear: rolled {} ({}): {}", rules.name(&items, &stack), rules.rarity(&items, &stack).map_or("-", |r| r.name.as_str()), bonuses.join(", "));
                inv.add(&items, stack);
            }
            state.0 = 2;
        }
        2 if t > 1.8 => {
            let listed: Vec<String> = stats.nonzero().map(|(s, v)| crate::gear::stats::line(s, v)).collect();
            info!("gear: in iron: health {:.0}/{:.0}, {}", h.hp, h.max, listed.join(", "));
            state.1 = h.hp;
            blast(&mut sim);
            state.0 = 3;
        }
        3 if t > 2.4 => {
            info!("gear: the same blast in iron cost {:.0} hp", state.1 - h.hp);
            keys.press(KeyCode::Escape);
            // (Washed, and out of the blood the blasts left: it tints.)
            commands.entity(me).remove::<crate::creatures::body::elements::Coated>();
            k.body.pos.x -= 75.0;
            k.prev_pos = k.body.pos;
            let floor = platypus_worldgen::arena::FLOOR as f32;
            let x = k.body.pos.x;
            for (kind, dx, wear) in [
                ("orc", 39.0, ["leather_cap", "leather_jerkin", "leather_gloves", "leather_trousers", "leather_boots"]),
                ("skeleton", 66.0, ["cloth_hood", "apprentice_robe", "silk_gloves", "cloth_trousers", "soft_boots"]),
            ] {
                let pieces: Vec<Option<Stack>> = wear.iter().map(|id| items.id(id).map(|it| Stack::new(it, 1))).collect();
                crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(x + dx, floor), move |e| {
                    e.remove::<crate::creatures::brain::hunter::Hunter>();
                    let mut eq = crate::gear::Equipment::default();
                    for (i, piece) in pieces.into_iter().enumerate() {
                        eq.worn[i] = piece;
                    }
                    e.insert(eq);
                });
            }
            state.0 = 4;
        }
        4 if t > 2.8 => {
            let jerkin = items.id("leather_jerkin");
            let at = inv.slots.iter().position(|s| s.is_some_and(|s| Some(s.item) == jerkin));
            let scale = window.scale_factor();
            let pos = at.and_then(|i| slots.iter().find(|(sl, _, v)| sl.0 == crate::hands::ui::Holder::Pack && sl.1 == i && v.get()).map(|(_, tf, _)| tf.translation / scale));
            info!("gear: hovering the jerkin in slot {at:?} at {pos:?}");
            window.set_cursor_position(pos);
            state.0 = 5;
        }
        // Picked up (into the mouse's grip, as a click would with the
        // pointer there): the body slot it goes in lights up.
        5 if t > 3.0 => {
            let jerkin = items.id("leather_jerkin");
            if let Some(i) = inv.slots.iter().position(|s| s.is_some_and(|s| Some(s.item) == jerkin)) {
                grip.stack = inv.slots[i].take();
            }
            state.0 = 6;
        }
        _ => {}
    }
}

/// A body to loot.
#[allow(clippy::too_many_arguments)]
fn loot_script(
    mut commands: Commands,
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    rules: Res<crate::gear::GearRules>,
    mut chests: ResMut<crate::hands::chests::Chests>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut foes: Query<&mut crate::creatures::Health, (With<crate::creatures::Creature>, Without<LocalPlayer>)>,
    bodies: Query<(&crate::hands::corpses::Corpse, &Kinematics)>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<u8>,
) {
    use crate::hands::items::{Roll, Stack};
    if s.name != "loot" {
        return;
    }
    let (Some(items), Ok(k)) = (items, player.single()) else { return };
    let t = s.elapsed;
    mouse.release(MouseButton::Right);
    match *state {
        0 if t > 0.5 => {
            let helm = items.id("iron_helm").map(|i| Stack { roll: Roll { rarity: 2, level: 12, seed: 4242 }, ..Stack::new(i, 1) });
            let jerkin = items.id("leather_jerkin").map(|i| Stack::new(i, 1));
            let floor = platypus_worldgen::arena::FLOOR as f32;
            crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(k.body.pos.x + 36.0, floor), move |e| {
                e.remove::<crate::creatures::brain::hunter::Hunter>();
                let mut eq = crate::gear::Equipment::default();
                eq.worn[0] = helm;
                eq.worn[1] = jerkin;
                e.insert(eq);
            });
            *state = 1;
        }
        1 if t > 1.0 => {
            for mut h in &mut foes {
                h.hp = 0.0;
            }
            *state = 2;
        }
        2 if t > 1.6 => {
            for (c, bk) in &bodies {
                let key = c.key;
                let inside: Vec<String> = chests.contents(key, &sim.world, &items).slots.iter().flatten().map(|s| rules.name(&items, s)).collect();
                info!("loot: a body at ({:.0}, {:.0}) holding: {}", bk.body.pos.x, bk.body.pos.y, inside.join(", "));
                cursor.0 = Some(bk.body.pos);
            }
            *state = 3;
        }
        3 if t > 2.0 => {
            mouse.press(MouseButton::Right);
            *state = 4;
        }
        4 if t > 2.3 => {
            info!("loot: opened {:?} ({})", chests.open, chests.open_name);
            *state = 5;
        }
        _ => {}
    }
}

type Foe = (With<crate::creatures::Creature>, Without<LocalPlayer>, Without<crate::creatures::custom::dummy::Dummy>);

/// The Broodmother's Fang at an orc.
#[allow(clippy::too_many_arguments)]
fn fang_script(
    mut commands: Commands,
    s: Res<Scenario>,
    items: Option<Res<crate::hands::items::Items>>,
    mut player: Query<(&Kinematics, &mut crate::hands::items::Inventory), With<LocalPlayer>>,
    foes: Query<(&Kinematics, &crate::creatures::Health, Option<&crate::creatures::body::elements::Coated>), Foe>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32)>,
) {
    if s.name != "fang" {
        return;
    }
    let (Some(items), Ok((k, mut inv))) = (items, player.single_mut()) else { return };
    let t = s.elapsed;
    if state.0 == 0 && t > 0.3 {
        inv.slots[0] = items.id("broodmother_fang").map(|f| crate::hands::items::Stack { roll: crate::hands::items::Roll { rarity: 4, level: 1, seed: 1 }, ..crate::hands::items::Stack::new(f, 1) });
        let floor = platypus_worldgen::arena::FLOOR as f32;
        crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(k.body.pos.x + 18.0, floor), |e| {
            e.remove::<crate::creatures::brain::hunter::Hunter>();
        });
        state.0 = 1;
    }
    let orc = foes.iter().next();
    if let Some((ok, ..)) = orc {
        cursor.0 = Some(ok.body.pos);
    }
    let swing = (1.0..2.5).contains(&t);
    match (swing, mouse.pressed(MouseButton::Left)) {
        (true, false) => mouse.press(MouseButton::Left),
        (false, true) => mouse.release(MouseButton::Left),
        _ => {}
    }
    if t > state.1 + 0.5 && t > 1.0 && state.0 < 5 {
        state.1 = t;
        state.0 += 1;
        match orc {
            Some((_, h, coat)) => info!("fang: t {t:.1} the orc has {:.0} hp, coated {:?}", h.hp, coat.map(|c| &c.coats)),
            None => info!("fang: t {t:.1} the orc is dead"),
        }
    }
}

/// A focus held, to see its aura.
fn foci_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut hand: ResMut<crate::hands::Hand>,
    player: Query<(Entity, &Kinematics), With<LocalPlayer>>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
) {
    if s.name != "foci" {
        return;
    }
    let Ok((me, k)) = player.single() else { return };
    let slot: usize = std::env::var("PLATYPUS_SLOT").ok().and_then(|v| v.parse().ok()).unwrap_or(7);
    hand.bar = (slot - 1) / 10;
    hand.slot = (slot - 1) % 10;
    let at = k.body.pos + Vec2::new(45.0, 21.0);
    cursor.0 = Some(at);
    if s.elapsed > 1.5 {
        if std::env::var("PLATYPUS_CAST").is_ok() {
            if !mouse.pressed(MouseButton::Left) {
                mouse.press(MouseButton::Left);
            }
        } else {
            commands.entity(me).insert(crate::creatures::body::animation::Aiming { at, left: 0.2 });
        }
    }
}

type IceWalker<'a> = (&'a mut Kinematics, Option<&'a crate::creatures::body::elements::Chilled>);
type IceFoe = (With<crate::creatures::Creature>, Without<LocalPlayer>, Without<crate::creatures::custom::dummy::Dummy>);

/// Frost on the pool: ice to slide on, and a chilled orc.
#[allow(clippy::too_many_arguments)]
fn ice_script(
    mut commands: Commands,
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut hand: ResMut<crate::hands::Hand>,
    mut player: Query<IceWalker, With<LocalPlayer>>,
    foes: Query<(&Kinematics, Option<&crate::creatures::body::elements::Chilled>), IceFoe>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<(u8, f32, f32)>,
) {
    if s.name != "ice" {
        return;
    }
    let Ok((mut k, chilled)) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let ice = || {
        let Some(m) = sim.materials().id("ice") else { return 0 };
        (450..645).flat_map(|x| (floor - 18..floor + 3).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).count()
    };
    let mut want = std::collections::HashSet::new();
    let mut fire = None;
    // Frost along the pool, from its left bank.
    if t < 0.4 {
        k.body.pos = Vec2::new(444.0, floor as f32 + 12.0);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
        hand.bar = 1;
        hand.slot = 5;
    }
    if (0.5..3.5).contains(&t) {
        let x = 474.0 + ((t - 0.5) / 3.0) * 150.0;
        fire = Some(Vec2::new(x, floor as f32 - 3.0));
    }
    match state.0 {
        0 if t > 3.8 => {
            info!("ice: frost made {} cells of ice on the pool", ice());
            // Run from the stone onto the ice, then let go.
            k.body.pos = Vec2::new(435.0, floor as f32 + 12.0);
            k.prev_pos = k.body.pos;
            state.0 = 1;
        }
        1 if t > 4.0 => {
            state.0 = 2;
        }
        2 if t > 4.8 => {
            state.1 = k.body.pos.x;
            state.0 = 3;
        }
        3 if t > 6.0 => {
            info!("ice: let go on the ice at x {:.0}, slid {:.0} cells; chilled: {}", state.1, k.body.pos.x - state.1, chilled.is_some());
            // The same on stone, right of the pool.
            k.body.pos = Vec2::new(660.0, floor as f32 + 12.0);
            k.prev_pos = k.body.pos;
            state.0 = 4;
        }
        4 if t > 6.1 => state.0 = 5,
        5 if t > 6.9 => {
            state.1 = k.body.pos.x;
            state.0 = 6;
        }
        6 if t > 8.1 => {
            info!("ice: on stone it slid {:.0} cells", k.body.pos.x - state.1);
            k.body.pos = Vec2::new(450.0, floor as f32 + 12.0);
            k.prev_pos = k.body.pos;
            crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(525.0, floor as f32 + 1.5), |e| {
                e.remove::<crate::creatures::brain::hunter::Hunter>();
            });
            state.0 = 7;
        }
        7 if t > 8.6 => {
            state.0 = 8;
        }
        8 if t > 9.4 => {
            info!("ice: the orc hit by frost: chilled {:?}", foes.iter().next().map(|(_, c)| c.map(|c| (c.cold, c.left))));
            state.0 = 9;
        }
        _ => {}
    }
    if matches!(state.0, 2 | 5) {
        want.insert(KeyCode::KeyD);
    }
    if state.0 == 8 {
        fire = foes.iter().next().map(|(fk, _)| fk.body.pos);
    }
    for key in [KeyCode::KeyD] {
        match (want.contains(&key), keys.pressed(key)) {
            (true, false) => keys.press(key),
            (false, true) => keys.release(key),
            _ => {}
        }
    }
    cursor.0 = fire.or(Some(k.body.pos + Vec2::new(60.0, 0.0)));
    match (fire.is_some(), mouse.pressed(MouseButton::Left)) {
        (true, false) => mouse.press(MouseButton::Left),
        (false, true) => mouse.release(MouseButton::Left),
        _ => {}
    }
}

/// What's about, for the chaos report.
#[derive(bevy::ecs::system::SystemParam)]
struct Census<'w, 's> {
    creatures: Query<'w, 's, (), (With<crate::creatures::Creature>, Without<LocalPlayer>)>,
    bodies: Query<'w, 's, (), With<crate::hands::corpses::Corpse>>,
    spells: Query<'w, 's, (), With<crate::magic::Spell>>,
    drops: Query<'w, 's, (), With<crate::hands::Dropped>>,
    arrows: Query<'w, 's, (), With<crate::archery::Arrow>>,
    sparks: Res<'w, crate::vfx::Sparks>,
    meshes: MessageReader<'w, 's, AssetEvent<Mesh>>,
    images: MessageReader<'w, 's, AssetEvent<Image>>,
    mesh_count: Res<'w, Assets<Mesh>>,
    image_count: Res<'w, Assets<Image>>,
}

/// Waves of everything, to see what gives first.
#[allow(clippy::too_many_arguments)]
fn chaos_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    tools: Res<crate::tools::ToolsConfig>,
    book: Res<crate::magic::Spellbook>,
    mut player: Query<(Entity, &mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    foes: Query<&Kinematics, Foe>,
    mut census: Census,
    mut casts: MessageWriter<crate::magic::CastRequest>,
    mut state: Local<(u32, f32, u64)>,
    mut changed: Local<(u32, u32, u32, u32)>,
) {
    if s.name != "chaos" {
        return;
    }
    // Meshes and images changed (re-uploaded) and added, since the last report.
    for e in census.meshes.read() {
        match e {
            AssetEvent::Modified { .. } => changed.0 += 1,
            AssetEvent::Added { .. } => changed.1 += 1,
            _ => {}
        }
    }
    for e in census.images.read() {
        match e {
            AssetEvent::Modified { .. } => changed.2 += 1,
            AssetEvent::Added { .. } => changed.3 += 1,
            _ => {}
        }
    }
    let Ok((me, mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    // The arena's middle; in a generated world, where the player started
    // (with PLATYPUS_CHAOS_RUN=1 moving right at 90 cells/s: the fight
    // streams the world in as it goes).
    if state.2 == 0 {
        state.2 = if sim.generator.wild() { (k.body.pos.x.to_bits() as u64) << 32 | k.body.pos.y.to_bits() as u64 } else { 1 };
    }
    let start = if state.2 == 1 { Vec2::new(930.0, platypus_worldgen::arena::FLOOR as f32 + 12.0) } else { Vec2::new(f32::from_bits((state.2 >> 32) as u32), f32::from_bits(state.2 as u32)) };
    let run = if std::env::var("PLATYPUS_CHAOS_RUN").is_ok() { 90.0 * t } else { 0.0 };
    let home = start + Vec2::new(run, 0.0);
    let floor = find_ground(&sim.world, home.x as i32, home.y as i32 + 60, 300).unwrap_or(home.y as i32 - 12) as f32;
    let home = Vec2::new(home.x, floor + 12.0);
    // Untouchable, and kept in the middle of it.
    h.hp = h.max;
    if k.body.pos.distance(home) > 45.0 {
        k.body.pos = home;
        k.body.vel = Vec2::ZERO;
        k.prev_pos = home;
    }
    let base: u32 = std::env::var("PLATYPUS_CHAOS").ok().and_then(|v| v.parse().ok()).unwrap_or(8);
    let mut rng = platypus_sim::rng::Rng::seeded(&[sim.world.tick(), 0xC4A05]);
    let mut unit = || rng.next_u32() as f32 / u32::MAX as f32;
    // A wave every 3 s, bigger each time.
    if t > 1.0 + state.0 as f32 * 3.0 {
        state.0 += 1;
        let n = base * state.0;
        const KINDS: [(&str, f32); 8] = [("orc", 0.3), ("orc_archer", 0.15), ("skeleton", 0.15), ("spider", 0.1), ("slime", 0.1), ("troll", 0.05), ("vampire_bat", 0.1), ("spiderling", 0.05)];
        for _ in 0..n {
            let mut pick = unit();
            let kind = KINDS.iter().find(|(_, w)| {
                pick -= w;
                pick <= 0.0
            }).map_or("orc", |k| k.0);
            let side = if unit() < 0.5 { -1.0 } else { 1.0 };
            let x = home.x + side * (90.0 + unit() * 390.0);
            crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(x, floor + 90.0 + unit() * 60.0), |_| {});
        }
        // Bombs, blobs from the sky, a strike.
        for _ in 0..4 + state.0 {
            let at = Vec2::new(home.x + (unit() - 0.5) * 750.0, floor + 180.0);
            crate::props::spawn_bomb(&mut commands, at, Vec2::new((unit() - 0.5) * 150.0, 0.0), tools.bomb.clone());
        }
        let mats = sim.materials().clone();
        for (name, r) in [("sand", 15), ("water", 18), ("lava", 9)] {
            if let Some(m) = mats.id(name) {
                let at = CellPos::new((home.x + (unit() - 0.5) * 600.0) as i32, (floor + 150.0 + unit() * 90.0) as i32);
                sim.queue(WorldEdit::Paint { center: at, radius: r, material: m, overwrite: false });
            }
        }
        sim.queue(WorldEdit::Lightning { x: (home.x + (unit() - 0.5) * 450.0) as i32, from_y: (floor + 300.0) as i32 });
        info!("chaos: wave {} ({} spawned)", state.0, n);
    }
    // A salvo of spells at the crowd, every spell as fast as it goes.
    let target = foes.iter().min_by(|a, b| a.body.pos.distance(home).total_cmp(&b.body.pos.distance(home))).map(|f| f.body.pos);
    if let Some(at) = target {
        for id in ["fireball", "lightning", "acid_arrow", "spark_bolt", "frost_bolt"] {
            if let Some(spell) = book.spells.iter().position(|s| s.id == id) {
                casts.write(crate::magic::CastRequest { caster: me, spell, from: k.body.pos + Vec2::new(0.0, 9.0), toward: at, alt: false });
            }
        }
    }
    if t >= state.1 + 1.0 {
        state.1 = t;
        info!(
            "chaos: t {t:.0} wave {} | creatures {} bodies {} spells {} drops {} arrows {} sparks {} particles {}",
            state.0,
            census.creatures.iter().count(),
            census.bodies.iter().count(),
            census.spells.iter().count(),
            census.drops.iter().count(),
            census.arrows.iter().count(),
            census.sparks.count(),
            sim.world.particles().len(),
        );
        info!(
            "chaos: t {t:.0} assets: meshes {} ({} changed, {} added), images {} ({} changed, {} added) in the last second",
            census.mesh_count.len(),
            changed.0,
            changed.1,
            census.image_count.len(),
            changed.2,
            changed.3
        );
        let mut sizes: Vec<usize> = census.mesh_count.iter().map(|(_, m)| m.count_vertices()).collect();
        sizes.sort_unstable_by(|a, b| b.cmp(a));
        info!("chaos: t {t:.0} biggest meshes (vertices): {:?}", &sizes[..sizes.len().min(8)]);
        *changed = Default::default();
    }
}

type RocketFoe = (With<crate::creatures::Creature>, Without<LocalPlayer>, Without<crate::creatures::custom::dummy::Dummy>);

/// Rocket boots over an orc on planks.
#[allow(clippy::too_many_arguments)]
fn rocket_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    foes: Query<(&crate::creatures::Health, Has<crate::creatures::body::elements::Burning>), RocketFoe>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<(u8, f32, f32)>,
) {
    if s.name != "rocket" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let x = 930;
    let burning = |sim: &SimWorld| {
        let mats = sim.materials();
        (x - 60..x + 60).flat_map(|xx| (floor - 3..floor + 90).map(move |y| (xx, y))).filter(|&(xx, y)| sim.world.get(CellPos::new(xx, y)).is_some_and(|c| !c.is_air() && (mats.phys(c.material).kind == platypus_sim::Kind::Fire || c.flags & platypus_sim::cell::flags::BURNING != 0))).count()
    };
    match state.0 {
        0 if t > 0.3 => {
            // A wooden floor, and an orc on it.
            if let Some(planks) = sim.materials().id("planks") {
                for dx in (-54..=54).step_by(9) {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x + dx, floor + 2), radius: 4, material: planks, overwrite: false });
                }
            }
            crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(x as f32 + 6.0, floor as f32 + 8.0), |e| {
                e.remove::<crate::creatures::brain::hunter::Hunter>();
            });
            k.body.pos = Vec2::new(x as f32, floor as f32 + 60.0);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            // (Full boots: they refill over seconds now; clamped to what
            // they hold.)
            k.loco.rocket_left = f32::MAX;
            state.1 = k.body.pos.y;
            state.0 = 1;
        }
        1 if t > 1.0 => state.0 = 2,
        2 => {
            state.2 = state.2.max(k.body.pos.y);
            if t > 3.2 {
                let fuel = sim.world.tick();
                let _ = fuel;
                let (hp, lit) = foes.iter().next().map_or((0.0, false), |(h, b)| (h.hp, b));
                info!("rocket: held jump 2.2 s: rose {:.0} cells above where it started (a jump is 60); the orc under it: {hp:.0} hp, burning {lit}; fire on the planks: {} cells", state.2 - state.1, burning(&sim));
                state.0 = 3;
            }
        }
        3 if t > 6.0 => {
            info!("rocket: after landing: fire on the planks {} cells", burning(&sim));
            state.0 = 4;
        }
        _ => {}
    }
    let hold = state.0 == 2;
    match (hold, keys.pressed(KeyCode::Space)) {
        (true, false) => keys.press(KeyCode::Space),
        (false, true) => keys.release(KeyCode::Space),
        _ => {}
    }
}

/// Potions and the screen's edges (see the module docs).
fn potion_script(s: Res<Scenario>, items: Option<Res<crate::hands::items::Items>>, mut hand: ResMut<crate::hands::Hand>, mut keys: ResMut<ButtonInput<KeyCode>>, mut player: Query<(&mut crate::creatures::Health, &crate::hands::items::Inventory, Has<crate::potion::PotionSickness>), With<LocalPlayer>>, mut state: Local<(u8, f32)>) {
    if s.name != "potion" {
        return;
    }
    let Ok((mut h, inv, sick)) = player.single_mut() else { return };
    let t = s.elapsed;
    let potions = || items.as_ref().and_then(|i| i.id("small_health_potion")).map_or(0, |p| inv.count(p));
    keys.release(KeyCode::KeyH);
    match state.0 {
        0 if t > 0.5 => {
            h.hp = 22.0;
            // (The potion in hand: its flask shows.)
            if let Some(i) = items.as_ref().and_then(|it| it.id("small_health_potion")).and_then(|p| inv.slots.iter().position(|s| s.is_some_and(|s| s.item == p))) {
                hand.bar = i / crate::hands::items::HOTBAR;
                hand.slot = i % crate::hands::items::HOTBAR;
            }
            state.0 = 1;
        }
        1 if t > 2.5 => {
            h.hp -= 8.0;
            state.0 = 2;
        }
        2 if t > 4.0 => {
            info!("potion: before: {:.0} of {:.0} hp, {} potions", h.hp, h.max, potions());
            state.1 = h.hp;
            keys.press(KeyCode::KeyH);
            state.0 = 3;
        }
        3 if t > 6.0 => {
            info!("potion: 2 s after: {:.0} hp (+{:.0}), {} potions left, sick {sick}", h.hp, h.hp - state.1, potions());
            state.0 = 4;
        }
        4 if t > 8.2 => {
            info!("potion: 4 s after: {:.0} hp (+{:.0})", h.hp, h.hp - state.1);
            state.1 = h.hp;
            keys.press(KeyCode::KeyH);
            state.0 = 5;
        }
        5 if t > 8.7 => {
            info!("potion: H again while sick: {:.0} hp (+{:.0}), {} potions left", h.hp, h.hp - state.1, potions());
            keys.press(KeyCode::Escape);
            state.0 = 6;
        }
        6 => keys.release(KeyCode::Escape),
        _ => {}
    }
}

/// Rocket boots under water: they fill back up there, and fire (slower,
/// no flame).
/// `spiderdeath`: a spider bled and killed beside the player.
#[allow(clippy::too_many_arguments)]
fn spiderdeath_script(
    mut commands: Commands,
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut spiders: Query<(Entity, &crate::creatures::Creature, &Kinematics, &mut crate::creatures::Health), Without<LocalPlayer>>,
    mut state: Local<u8>,
) {
    if s.name != "spiderdeath" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR as f32;
    let x = 780.0;
    let acid = sim.materials().id("acid");
    match *state {
        0 if t > 0.3 => {
            k.body.pos = Vec2::new(x, floor + k.body.half.y);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            crate::creatures::def::spawn_creature(&mut commands, "spider", Vec2::new(x + 60.0, floor), |_| {});
            *state = 1;
        }
        // A longsword's blow (24, knocked 375 cells/s), twice (after its
        // grace): how fast each sends it.
        1 | 3 if t > if *state == 1 { 1.0 } else { 1.1 } => {
            for (e, c, sk, _) in &spiders {
                if c.kind == "spider" {
                    let dir = Vec2::new(1.0, 0.3).normalize();
                    hits.write(crate::combat::Hit { target: e, damage: 24.0, harm: crate::creatures::Harm::Slash, knock: dir * 375.0, stun: 0.3, at: sk.body.pos, dir, weight: 2.0, crit: false });
                }
            }
            *state += 1;
        }
        2 | 4 if t > if *state == 2 { 1.05 } else { 1.15 } => {
            for (_, c, sk, _) in &spiders {
                if c.kind == "spider" {
                    info!("spiderdeath: struck ({}): moving {:.0} cells/s", if *state == 2 { "first" } else { "second" }, sk.body.vel.length());
                }
            }
            *state += 1;
        }
        5 if t > 1.5 => {
            for (_, c, sk, mut h) in &mut spiders {
                if c.kind == "spider" {
                    let p = sk.body.pos;
                    let near = (-45..45).flat_map(|dx| (-30..30).map(move |dy| CellPos::from_world(p.x + dx as f32, p.y + dy as f32))).filter(|&q| sim.world.get(q).is_some_and(|cell| Some(cell.material) == acid)).count();
                    info!("spiderdeath: a spider of {:.0} hp (after two blows), {near} cells of acid about it", h.hp);
                    h.hp = 0.0;
                }
            }
            *state = 6;
        }
        _ => {}
    }
}

/// `forestfire`: a forest set alight, the land pinned at a wetness.
fn forestfire_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, mut clock: ResMut<crate::clock::WorldClock>, player: Query<&Kinematics, With<LocalPlayer>>, mut state: Local<(u8, Vec2, usize)>) {
    if s.name != "forestfire" {
        return;
    }
    let wet: f32 = std::env::var("PLATYPUS_WET").ok().and_then(|v| v.parse().ok()).unwrap_or(0.05);
    clock.pinned = Some(wet);
    let t = s.elapsed;
    let Ok(k) = player.single() else { return };
    let mats = sim.materials().clone();
    let forest = ["leaves", "needles", "dark_leaves", "wood"].map(|n| mats.id(n));
    let count = |sim: &SimWorld, at: Vec2| {
        let mut n = 0;
        for y in (at.y as i32 - 90)..(at.y as i32 + 390) {
            for x in (at.x as i32 - 375)..(at.x as i32 + 375) {
                if sim.world.get_bg(CellPos::new(x, y)).is_some_and(|c| forest.contains(&Some(c.material)) && c.flags & platypus_sim::cell::flags::BURNING == 0) {
                    n += 1;
                }
            }
        }
        n
    };
    match state.0 {
        0 if t > 1.0 => {
            // The nearest crown: leaves in the background above the start.
            let p = k.body.pos;
            let crown = (0..300).flat_map(|r| [(r, 0), (-r, 0)]).flat_map(|(dx, _)| (30..240).map(move |dy| Vec2::new(p.x + dx as f32, p.y + dy as f32))).find(|q| sim.world.get_bg(CellPos::from_world(q.x, q.y)).is_some_and(|c| Some(c.material) == forest[0] || Some(c.material) == forest[1]));
            let Some(crown) = crown else {
                info!("forestfire: no tree near the start");
                state.0 = 9;
                return;
            };
            state.1 = crown;
            state.2 = count(&sim, crown);
            sim.queue(WorldEdit::Ignite { center: CellPos::from_world(crown.x, crown.y), radius: 6 });
            info!("forestfire: the land {:.0} % wet; a crown at ({:.0}, {:.0}) set alight; {} cells of forest about it", wet * 100.0, crown.x, crown.y, state.2);
            state.0 = 1;
        }
        1 if t > 26.0 => {
            let left = count(&sim, state.1);
            info!("forestfire: 25 s later, {:.0} % wet: {left} of {} cells of forest left ({} burnt)", wet * 100.0, state.2, state.2.saturating_sub(left));
            state.0 = 2;
        }
        _ => {}
    }
}

/// `regrow`: a forest burnt (an hour passing, the player still there),
/// the player away for `PLATYPUS_DAYS` (default 3) while the world clock
/// runs, then back: how much has grown again.
fn regrow_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut clock: ResMut<crate::clock::WorldClock>,
    mut day: ResMut<crate::light::Daylight>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut state: Local<(u8, Vec2)>,
    mut stay: Local<Option<f32>>,
) {
    if s.name != "regrow" {
        return;
    }
    let days: f32 = std::env::var("PLATYPUS_DAYS").ok().and_then(|v| v.parse().ok()).unwrap_or(3.0);
    let t = s.elapsed;
    let Ok(mut k) = player.single_mut() else { return };
    let mats = sim.materials().clone();
    let id = |n: &str| mats.id(n);
    let tree = ["leaves", "needles", "dark_leaves", "wood"].map(id);
    let tally = |sim: &SimWorld, at: Vec2| {
        let (mut forest, mut grass, mut soil, mut ash) = (0, 0, 0, 0);
        for y in (at.y as i32 - 120)..(at.y as i32 + 390) {
            for x in (at.x as i32 - 450)..(at.x as i32 + 450) {
                let p = CellPos::new(x, y);
                if sim.world.get_bg(p).is_some_and(|c| tree.contains(&Some(c.material))) {
                    forest += 1;
                }
                let Some(c) = sim.world.get(p) else { continue };
                let m = Some(c.material);
                grass += (m == id("grass")) as u32;
                soil += (m == id("soil")) as u32;
                ash += (m == id("ash") || m == id("charcoal")) as u32;
            }
        }
        format!("{forest} cells of forest, {grass} of grass, {soil} of burnt soil, {ash} of ash and charcoal")
    };
    let put = |k: &mut Kinematics, at: Vec2| {
        k.body.pos = at;
        k.body.vel = Vec2::ZERO;
        k.prev_pos = at;
    };
    match state.0 {
        0 if t > 1.0 => {
            clock.pinned = Some(0.02);
            let p = k.body.pos;
            state.1 = p;
            info!("regrow: before: {}", tally(&sim, p));
            for dx in [-240, -120, 0, 120, 240] {
                sim.queue(WorldEdit::Ignite { center: CellPos::from_world(p.x + dx as f32, p.y + 90.0), radius: 9 });
                sim.queue(WorldEdit::Ignite { center: CellPos::from_world(p.x + dx as f32, p.y - 3.0), radius: 9 });
            }
            state.0 = 1;
        }
        // (An hour on, with the player still there: the clock has seen it.)
        1 if t > 40.0 => {
            info!("regrow: burnt: {}", tally(&sim, state.1));
            clock.pinned = None;
            day.skipped += 1.0;
            state.0 = 2;
        }
        // (PLATYPUS_STAY=1: the player stays, an hour passing every tenth
        // of a second, as it would in play.)
        2 if t > 42.0 && std::env::var("PLATYPUS_STAY").is_ok() => {
            let hours = (days * 24.0).round() as u32;
            let done = ((t - 42.0) * 10.0) as u32;
            if done >= hours {
                state.0 = 4;
                return;
            }
            let base = *stay.get_or_insert(day.skipped);
            day.skipped = base + done as f32;
        }
        2 if t > 42.0 => {
            put(&mut k, state.1 + Vec2::new(6_000.0, 600.0));
            state.0 = 7;
        }
        7 if t > 44.0 => {
            day.skipped += 24.0 * days;
            state.0 = 3;
        }
        3 if t > 46.0 => {
            put(&mut k, state.1 + Vec2::new(0.0, 30.0));
            state.0 = 4;
        }
        4 if t > 50.0 && (std::env::var("PLATYPUS_STAY").is_err() || t > 44.0 + days * 2.4 + 2.0) => {
            info!("regrow: {days} days later: {} ({} trees regrowing, {} chunks healing)", tally(&sim, state.1), clock.trees.len(), clock.land.len());
            state.0 = 5;
        }
        _ => {}
    }
}

/// `wildfire`: the world clock's lightning in a forest well away from the
/// start, `PLATYPUS_DAYS` (default 0.25) passing, then the player there:
/// the scar (and, days on, its healing).
fn wildfire_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut clock: ResMut<crate::clock::WorldClock>,
    mut day: ResMut<crate::light::Daylight>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut state: Local<(u8, i32)>,
) {
    if s.name != "wildfire" {
        return;
    }
    let days: f32 = std::env::var("PLATYPUS_DAYS").ok().and_then(|v| v.parse().ok()).unwrap_or(0.25);
    let t = s.elapsed;
    let Ok(mut k) = player.single_mut() else { return };
    let home = sim.generator.spawn_point();
    match state.0 {
        0 if t > 1.0 => {
            let Some(&x) = sim.generator.trees_between(home.x + 3_750, home.x + 12_000).first() else {
                info!("wildfire: no forest east of the start");
                state.0 = 9;
                return;
            };
            let took = clock.wildfire(&*sim.generator, x, 1.0);
            let f = clock.fires.last().copied();
            info!("wildfire: lightning at x {x}: {took} trees, the scar {:?}", f.map(|f| (f.x0, f.x1)));
            state.1 = f.map_or(x, |f| (f.x0 + f.x1) / 2);
            state.0 = 1;
        }
        1 if t > 2.0 => {
            day.skipped += 24.0 * days;
            state.0 = 2;
        }
        2 if t > 3.0 => {
            let x = state.1;
            let y = sim.generator.surface_hint(x).unwrap_or(home.y) + 45;
            k.body.pos = Vec2::new(x as f32, y as f32);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            state.0 = 3;
        }
        3 if t > 8.0 => {
            let mats = sim.materials();
            let (soil, ash, grass) = (mats.id("soil"), mats.id("ash"), mats.id("grass"));
            let tree = ["wood", "leaves", "needles", "dark_leaves"].map(|n| mats.id(n));
            let at = k.body.pos;
            let (mut n_soil, mut n_ash, mut n_grass, mut n_tree) = (0, 0, 0, 0);
            for y in (at.y as i32 - 120)..(at.y as i32 + 330) {
                for x in (at.x as i32 - 450)..(at.x as i32 + 450) {
                    let p = platypus_sim::CellPos::new(x, y);
                    let m = sim.world.get(p).map(|c| c.material);
                    n_soil += (m == soil) as u32;
                    n_ash += (m == ash) as u32;
                    n_grass += (m == grass) as u32;
                    n_tree += sim.world.get_bg(p).is_some_and(|c| tree.contains(&Some(c.material))) as u32;
                }
            }
            info!("wildfire: {days} days after: {n_tree} cells of forest, {n_grass} of grass, {n_soil} of burnt soil, {n_ash} of ash; {}", clock.describe(at));
            state.0 = 4;
        }
        _ => {}
    }
}

/// `refill`: the nearest spider nest, its keepers killed, the player away
/// for `PLATYPUS_DAYS` (default 4), back: the lair refilled.
fn refill_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    clock: Res<crate::clock::WorldClock>,
    mut day: ResMut<crate::light::Daylight>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut foes: Query<(&crate::creatures::Creature, &Kinematics, &mut crate::creatures::Health), Without<LocalPlayer>>,
    mut state: Local<(u8, Vec2)>,
) {
    if s.name != "refill" {
        return;
    }
    let days: f32 = std::env::var("PLATYPUS_DAYS").ok().and_then(|v| v.parse().ok()).unwrap_or(4.0);
    let t = s.elapsed;
    let Ok(mut k) = player.single_mut() else { return };
    let home = sim.generator.spawn_point();
    let put = |k: &mut Kinematics, at: Vec2| {
        k.body.pos = at;
        k.body.vel = Vec2::ZERO;
        k.prev_pos = at;
    };
    let about = |foes: &Query<(&crate::creatures::Creature, &Kinematics, &mut crate::creatures::Health), Without<LocalPlayer>>, at: Vec2| {
        let mut n = std::collections::BTreeMap::new();
        for (c, fk, h) in foes {
            if h.hp > 0.0 && fk.body.pos.distance(at) < 225.0 {
                *n.entry(c.kind.clone()).or_insert(0) += 1;
            }
        }
        n
    };
    match state.0 {
        0 if t > 0.3 => {
            let nest = sim.generator.landmarks().into_iter().filter(|(_, n)| n == "spider nest").min_by_key(|(p, _)| (p.x - home.x).abs() + (p.y - home.y).abs());
            let Some((p, _)) = nest else {
                info!("refill: no spider nest in this world");
                state.0 = 9;
                return;
            };
            state.1 = Vec2::new(p.x as f32 - 30.0, p.y as f32);
            put(&mut k, state.1);
            state.0 = 1;
        }
        1 if t < 1.5 => put(&mut k, state.1),
        1 if t > 3.0 => {
            let near = clock.keepers.iter().filter(|((x, y), _)| Vec2::new(*x as f32, *y as f32).distance(state.1) < 225.0).map(|(_, k)| k.kind.clone()).collect::<Vec<_>>();
            info!("refill: at the nest: {:?} about; the clock keeps {near:?}", about(&foes, state.1));
            for (c, fk, mut h) in &mut foes {
                if near.contains(&c.kind) && fk.body.pos.distance(state.1) < 225.0 {
                    h.hp = 0.0;
                }
            }
            state.0 = 2;
        }
        2 if t > 5.0 => {
            info!("refill: killed: {:?} about", about(&foes, state.1));
            let away = Vec2::new(home.x as f32 + 4_500.0, sim.generator.surface_hint(home.x + 4_500).unwrap_or(home.y) as f32 + 45.0);
            put(&mut k, away);
            state.0 = 3;
        }
        3 if t > 7.0 => {
            day.skipped += 24.0 * days;
            state.0 = 4;
        }
        4 if t > 9.0 => {
            put(&mut k, state.1);
            state.0 = 5;
        }
        5 if t < 10.5 => put(&mut k, state.1),
        5 if t > 13.0 => {
            info!("refill: {days} days later: {:?} about", about(&foes, state.1));
            state.0 = 6;
        }
        _ => {}
    }
}

/// `gold` (arena world): a warband struck dead (its gold bursts out as
/// dust; the player walks over and takes it), then gold thrown into lava,
/// an acid puddle and the water pool, and a blast in a heap. Logs each.
fn gold_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut queue: ResMut<crate::gold::GoldQueue>,
    mut player: Query<(&mut Kinematics, &crate::gold::Gold), With<LocalPlayer>>,
    mut foes: Query<(Entity, &crate::creatures::Creature, &mut crate::creatures::Health), Without<LocalPlayer>>,
    mut state: Local<u8>,
) {
    if s.name != "gold" {
        return;
    }
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let Ok((mut k, gold)) = player.single_mut() else { return };
    let put = |k: &mut Kinematics, x: f32| {
        k.body.pos = Vec2::new(x, floor as f32 + k.body.half.y);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
    };
    // Gold (and molten gold) between two x: cells, the lowest, the span.
    let (g, molten, set) = (sim.materials().id("gold"), sim.materials().id("molten_gold"), sim.materials().id("solid_gold"));
    let lying = |sim: &SimWorld, x0: i32, x1: i32| {
        let (mut n, mut m, mut low, mut lo, mut hi) = (0, 0, i32::MAX, i32::MAX, i32::MIN);
        for y in floor - 120..floor + 180 {
            for x in x0..x1 {
                let c = sim.world.get(CellPos::new(x, y)).map(|c| Some(c.material));
                if c == Some(g) {
                    n += 1;
                    (low, lo, hi) = (low.min(y), lo.min(x), hi.max(x));
                } else if c == Some(molten) || c == Some(set) {
                    m += 1;
                }
            }
        }
        (n, m, low, lo, hi)
    };
    match *state {
        0 if t > 0.5 => {
            put(&mut k, 840.0);
            for (i, kind) in ["orc", "orc", "orc_archer", "orc", "troll"].iter().enumerate() {
                crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(1050.0 + i as f32 * 24.0, floor as f32), |_| {});
            }
            *state = 1;
        }
        1 if t > 0.8 => {
            for (e, c, mut h) in &mut foes {
                if ["orc", "orc_archer", "troll"].contains(&c.kind.as_str()) {
                    // (The troll heals and won't go below 1 while it does:
                    // struck dead, it doesn't.)
                    commands.entity(e).remove::<crate::creatures::nature::Regenerates>();
                    h.hp = 0.0;
                }
            }
            *state = 2;
        }
        2 if t > 3.0 => {
            let (n, _, _, lo, hi) = lying(&sim, 900, 1350);
            info!("gold: the warband's gold burst out and settled: {n} gold from x {lo} to {hi}; the player has {}", gold.0);
            put(&mut k, 1050.0);
            *state = 3;
        }
        3 if t > 3.5 => {
            put(&mut k, 1140.0);
            *state = 4;
        }
        4 if t > 5.0 => {
            let (n, ..) = lying(&sim, 900, 1350);
            info!("gold: walked over it: the player has {}, {n} left lying", gold.0);
            put(&mut k, 900.0);
            let acid = sim.materials().id("acid");
            for x in 705..750 {
                for y in (floor - 6)..floor {
                    sim.queue(WorldEdit::Dig { center: CellPos::new(x, y), radius: 0, max_hardness: 250 });
                    if let Some(m) = acid {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: m, overwrite: true });
                    }
                }
            }
            *state = 5;
        }
        5 if t > 5.5 => {
            queue.burst.extend([(Vec2::new(1395.0, floor as f32 + 30.0), 100), (Vec2::new(727.0, floor as f32 + 9.0), 60), (Vec2::new(547.0, floor as f32 + 9.0), 60)]);
            *state = 6;
        }
        6 if t > 10.0 => {
            let (lava, acid, pool) = (lying(&sim, 1335, 1455), lying(&sim, 660, 780), lying(&sim, 450, 645));
            info!("gold: 100 into the lava: {} gold dust, {} molten or set solid; 60 into acid: {} gold (the lowest at y {}); 60 into the pool: {} gold, the lowest at y {} (its floor at {})", lava.0, lava.1, acid.0, acid.2, pool.0, pool.2, floor - 75);
            put(&mut k, 1515.0);
            queue.burst.push((Vec2::new(1650.0, floor as f32 + 6.0), 150));
            *state = 7;
        }
        7 if t > 12.0 => {
            let (n, _, _, lo, hi) = lying(&sim, 1560, 1950);
            info!("gold: a heap: {n} gold from x {lo} to {hi}; a blast in it");
            sim.queue(WorldEdit::Explode { center: CellPos::new(1650, floor + 3), radius: 15, power: 90 });
            *state = 8;
        }
        8 if t > 14.0 => {
            let (n, _, _, lo, hi) = lying(&sim, 1545, 2100);
            info!("gold: after the blast: {n} gold from x {lo} to {hi}");
            *state = 9;
        }
        9 if t > 24.0 => {
            let lava = lying(&sim, 1335, 1455);
            info!("gold: the lava pit, 18 s on: {} gold dust, {} molten or set solid", lava.0, lava.1);
            *state = 10;
        }
        _ => {}
    }
}

/// `goldheap` (a generated world): a hollow dug under the start, the
/// player in it, and `PLATYPUS_GOLD` (default 6 000) poured in from its
/// roof: how a hoard looks in the dark.
fn goldheap_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, mut queue: ResMut<crate::gold::GoldQueue>, mut player: Query<&mut Kinematics, With<LocalPlayer>>, mut state: Local<(u8, Vec2)>) {
    if s.name != "goldheap" {
        return;
    }
    let t = s.elapsed;
    let amount: u32 = std::env::var("PLATYPUS_GOLD").ok().and_then(|v| v.parse().ok()).unwrap_or(6_000);
    let Ok(mut k) = player.single_mut() else { return };
    match state.0 {
        0 if t > 0.5 => {
            let home = sim.generator.spawn_point();
            let c = Vec2::new(home.x as f32, home.y as f32 - 330.0);
            for dx in (-90..=90).step_by(12) {
                for dy in (-36..=36).step_by(12) {
                    sim.queue(WorldEdit::Dig { center: CellPos::new(c.x as i32 + dx, c.y as i32 + dy), radius: 14, max_hardness: 250 });
                }
            }
            state.1 = c;
            state.0 = 1;
        }
        1 if t > 1.0 => {
            let c = state.1;
            k.body.pos = Vec2::new(c.x - 60.0, c.y - 30.0);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            for dx in [-12.0, 21.0, 45.0] {
                queue.burst.push((Vec2::new(c.x + dx, c.y + 27.0), amount / 3));
            }
            info!("goldheap: {amount} gold poured into a hollow at ({:.0}, {:.0})", c.x, c.y);
            state.0 = 2;
        }
        // (Held away from the heap: it isn't taken.)
        2 => {
            let c = state.1;
            k.body.pos.x = c.x - 60.0;
        }
        _ => {}
    }
}

/// `kick` (arena world): a log of wood, a heap of sand and a TNT barrel,
/// each kicked in turn from its left; then a kick into the floor; then an
/// orc and a troll, each kicked. Logs where each went, and the health.
#[allow(clippy::too_many_arguments)]
fn kick_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut cursor: ResMut<CursorOverride>,
    mut kicks: MessageWriter<crate::kick::Kick>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    others: Query<(&Kinematics, Option<&crate::creatures::Creature>), Without<LocalPlayer>>,
    health: Query<(&crate::creatures::Creature, &Kinematics, &crate::creatures::Health), Without<LocalPlayer>>,
    mut state: Local<u8>,
) {
    if s.name != "kick" {
        return;
    }
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let Ok(mut k) = player.single_mut() else { return };
    let mut stand = |k: &mut Kinematics, x: f32| {
        k.body.pos = Vec2::new(x, floor as f32 + k.body.half.y);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
        cursor.0 = Some(Vec2::new(x + 90.0, floor as f32 + 12.0));
    };
    let mats = sim.materials().clone();
    let (wood, sand) = (mats.expect_id("wood"), mats.expect_id("sand"));
    let span = |sim: &SimWorld, m, x0: i32, x1: i32| {
        let xs: Vec<i32> = (floor - 15..floor + 90).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).map(|(x, _)| x).collect();
        (xs.len(), xs.iter().copied().min().unwrap_or(0), xs.iter().copied().max().unwrap_or(0))
    };
    match *state {
        0 if t > 0.5 => {
            // A log (45 × 8) on the floor at 960; sand at 1140; a barrel
            // at 1290.
            for x in 960..1005 {
                for y in floor..floor + 8 {
                    sim.world.set(CellPos::new(x, y), platypus_sim::Cell::new(wood, 128));
                }
            }
            sim.queue(WorldEdit::Paint { center: CellPos::new(1147, floor + 6), radius: 6, material: sand, overwrite: false });
            crate::creatures::def::spawn_creature(&mut commands, "tnt_barrel", Vec2::new(1302.0, floor as f32), |_| {});
            stand(&mut k, 945.0);
            *state = 1;
        }
        1 if t > 1.5 => {
            info!("kick: the log: {:?} (cells, from x, to x)", span(&sim, wood, 840, 1350));
            kicks.write(crate::kick::Kick);
            *state = 2;
        }
        2 if t > 3.5 => {
            info!("kick: the log after: {:?}", span(&sim, wood, 840, 1500));
            stand(&mut k, 1135.0);
            *state = 3;
        }
        3 if t > 4.0 => {
            info!("kick: the sand: {:?}", span(&sim, sand, 1050, 1350));
            kicks.write(crate::kick::Kick);
            *state = 4;
        }
        4 if t > 6.0 => {
            info!("kick: the sand after: {:?}", span(&sim, sand, 1050, 1500));
            stand(&mut k, 1284.0);
            *state = 5;
        }
        5 if t > 6.5 => {
            let barrel = others.iter().find(|(_, c)| c.is_some_and(|c| c.kind == "tnt_barrel")).map(|(k, ..)| k.body.pos.round());
            info!("kick: the barrel at {barrel:?}");
            kicks.write(crate::kick::Kick);
            *state = 6;
        }
        6 if t > 8.0 => {
            let barrel = others.iter().find(|(_, c)| c.is_some_and(|c| c.kind == "tnt_barrel")).map(|(k, ..)| k.body.pos.round());
            info!("kick: the barrel after: {barrel:?}");
            // (Into bare floor: nothing to move.)
            stand(&mut k, 870.0);
            *state = 7;
        }
        7 if t > 8.5 => {
            let (n, ..) = span(&sim, mats.expect_id("stone"), 840, 930);
            info!("kick: the floor: {n} cells of stone about");
            kicks.write(crate::kick::Kick);
            *state = 8;
        }
        8 if t > 9.5 => {
            let (n, ..) = span(&sim, mats.expect_id("stone"), 840, 930);
            info!("kick: the floor after: {n}");
            // Then creatures: an orc, then a troll, each kicked from its left.
            crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(1620.0, floor as f32), |_| {});
            crate::creatures::def::spawn_creature(&mut commands, "troll", Vec2::new(1770.0, floor as f32), |_| {});
            *state = 10;
        }
        10 | 12 if t > if *state == 10 { 10.0 } else { 11.6 } => {
            let kind = if *state == 10 { "orc" } else { "troll" };
            if let Some((_, ok, h)) = health.iter().find(|(c, ..)| c.kind == kind) {
                info!("kick: the {kind} at x {:.0}, health {:.0}; kicked", ok.body.pos.x, h.hp);
                stand(&mut k, ok.body.pos.x - ok.body.half.x - 10.0);
                kicks.write(crate::kick::Kick);
            }
            *state += 1;
        }
        11 | 13 if t > if *state == 11 { 10.6 } else { 12.2 } => {
            let kind = if *state == 11 { "orc" } else { "troll" };
            if let Some((_, ok, h)) = health.iter().find(|(c, ..)| c.kind == kind) {
                info!("kick: the {kind} after: at x {:.0}, health {:.0}", ok.body.pos.x, h.hp);
            }
            *state += 1;
        }
        _ => {}
    }
}

/// `logmagic` (flat world): a log (45 × 8 cells of wood) pushed by the force
/// wand, then lifted by the gravity wand, carried up and let go. Logs the
/// log (cells lying, bodies flying, where) every quarter second.
#[allow(clippy::too_many_arguments)]
fn logmagic_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    player: Query<(&Kinematics, Option<&crate::magic::Mana>, &crate::hands::items::Inventory), With<LocalPlayer>>,
    mut hand: ResMut<crate::hands::Hand>,
    items: Option<Res<crate::hands::items::Items>>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, Option<Vec2>)>,
    mut anchor: Local<Option<f32>>,
) {
    if s.name != "logmagic" {
        return;
    }
    let Ok((k, mana, inv)) = player.single() else { return };
    let t = s.elapsed;
    let home = *state.2.get_or_insert(k.body.pos);
    let wood = sim.materials().expect_id("wood");
    if state.0 == 0 && t > 0.5 {
        let Some(ground) = find_ground(&sim.world, home.x as i32 + 68, home.y as i32 + 60, 150) else { return };
        for x in home.x as i32 + 45..home.x as i32 + 90 {
            for y in ground..ground + 8 {
                sim.world.set(CellPos::new(x, y), platypus_sim::Cell::new(wood, 128));
            }
        }
        state.0 = 1;
    }
    // (Once the kit's in hand.) The wand in hand, wherever it is in the
    // hotbars: the force wand, then the gravity wand.
    let take = |id: &str, hand: &mut crate::hands::Hand| {
        let Some(i) = items.as_ref().and_then(|items| inv.slots.iter().position(|st| st.is_some_and(|st| items.def(st.item).id == id))) else { return };
        (hand.bar, hand.slot) = (i / crate::hands::items::HOTBAR, i % crate::hands::items::HOTBAR);
    };
    if state.0 == 1 && t > 1.0 {
        take("force_wand", &mut hand);
        state.0 = 5;
    }
    if state.0 == 5 && t > 3.0 {
        take("gravity_wand", &mut hand);
        state.0 = 2;
    }
    // Where the log is (its middle, lying or flying).
    let log_at = |sim: &SimWorld| {
        let xs: Vec<i32> = (home.y as i32 - 30..home.y as i32 + 180).flat_map(|y| (home.x as i32 - 75..home.x as i32 + 600).map(move |x| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == wood)).map(|(x, _)| x).collect();
        sim.world.bodies().first().map_or(xs.iter().sum::<i32>() as f32 / xs.len().max(1) as f32, |b| b.pos[0])
    };
    let log = |sim: &SimWorld| {
        let lying = (home.y as i32 - 30..home.y as i32 + 180).flat_map(|y| (home.x as i32 - 75..home.x as i32 + 600).map(move |x| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == wood)).count();
        let flying: Vec<String> = sim.world.bodies().iter().map(|b| format!("{:.0},{:.0}", b.pos[0] - home.x, b.pos[1] - home.y)).collect();
        (lying, flying)
    };
    let push = t > 1.5 && t < 2.0;
    let hold = t > 3.4 && t < 6.5;
    // The well: on the log, then up and over.
    // (Where the log lay as the hold began: the well goes there, then up
    // and slowly over, not after the log itself.)
    let from = if hold { *anchor.get_or_insert_with(|| log_at(&sim) - home.x) } else { log_at(&sim) - home.x };
    let lift_to = if t < 4.2 { Vec2::new(from, 9.0) } else { Vec2::new(from + (t - 4.2) * 30.0, 60.0) };
    cursor.0 = Some(home + if hold { lift_to } else { Vec2::new(90.0, 3.0) });
    if push || hold { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    if t >= state.1 {
        state.1 = (t * 4.0).floor() / 4.0 + 0.25;
        let (lying, flying) = log(&sim);
        let held = inv.slots[hand.active()].and_then(|st| items.as_ref().map(|i| i.def(st.item).id.clone()));
        info!("logmagic: t {t:.2} {} wood lying {lying}, flying {flying:?}, the log at x {:+.0} mana {:.0} holding {held:?}", if push { "push" } else if hold { "hold" } else { "-" }, log_at(&sim) - home.x, mana.map_or(0.0, |m| m.cur));
    }
}

/// `cast` (arena world): a mould of stone built on the floor (a plate with a
/// notch), a furnace set down; the ladle filled at the furnace from the copper
/// bars in the pack, poured into the mould; set, the mould's walls dug away
/// and the casting kicked. Logs each step.
#[allow(clippy::too_many_arguments)]
fn cast_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    crafting: Res<crate::craft::Crafting>,
    items: Option<Res<crate::hands::items::Items>>,
    mut hand: ResMut<crate::hands::Hand>,
    mut player: Query<(&mut Kinematics, &crate::hands::items::Inventory), With<LocalPlayer>>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut kicks: MessageWriter<crate::kick::Kick>,
    mut state: Local<u8>,
) {
    if s.name != "cast" {
        return;
    }
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let Ok((mut k, inv)) = player.single_mut() else { return };
    let Some(items) = items else { return };
    let mats = sim.materials().clone();
    let (stone, copper, molten) = (mats.expect_id("stone"), mats.expect_id("copper"), mats.expect_id("molten_copper"));
    let count = |sim: &SimWorld, m| (floor - 90..floor + 180).flat_map(|y| (840..2250).map(move |x| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).collect::<Vec<_>>();
    let span = |cells: &[(i32, i32)]| (cells.len(), cells.iter().map(|c| c.0).min().unwrap_or(0), cells.iter().map(|c| c.0).max().unwrap_or(0));
    let ladle = || inv.slots.iter().flatten().find(|st| items.def(st.item).id == "ladle").and_then(|st| st.fill.map(|(_, n)| n)).unwrap_or(0);
    // The mould: walls at 960..962 and 994..996, and a block in it (975..980,
    // 3 high): the casting's notch.
    let walls: Vec<CellPos> = (floor..floor + 12).flat_map(|y| [960, 961, 962, 994, 995, 996].map(|x| CellPos::new(x, y))).chain((975..981).flat_map(|x| (floor..floor + 3).map(move |y| CellPos::new(x, y)))).collect();
    match *state {
        0 if t > 0.5 => {
            for &p in &walls {
                sim.world.set(p, platypus_sim::Cell::new(stone, 0));
            }
            if let Some(kind) = crafting.station("furnace") {
                crafting.spawn(&mut commands, kind, Vec2::new(900.0, floor as f32));
            }
            k.body.pos = Vec2::new(930.0, floor as f32 + k.body.half.y);
            k.prev_pos = k.body.pos;
            *state = 1;
        }
        1 if t > 1.0 => {
            if let Some(i) = inv.slots.iter().position(|st| st.is_some_and(|st| items.def(st.item).id == "ladle")) {
                (hand.bar, hand.slot) = (i / crate::hands::items::HOTBAR, i % crate::hands::items::HOTBAR);
            }
            *state = 2;
        }
        2 if t > 1.3 => {
            // At the furnace: fill the ladle from the bars.
            cursor.0 = Some(Vec2::new(900.0, floor as f32 + 9.0));
            mouse.press(MouseButton::Right);
            if t > 1.6 {
                mouse.release(MouseButton::Right);
                let bars = inv.slots.iter().flatten().filter(|st| items.def(st.item).id == "copper_bar").map(|st| st.count).sum::<u32>();
                info!("cast: the ladle filled at the furnace: {} cells of molten copper; {} cells of copper bars left in the pack", ladle(), bars);
                *state = 3;
            }
        }
        3 => {
            // Pour into the mould.
            cursor.0 = Some(Vec2::new(978.0, floor as f32 + 6.0));
            mouse.press(MouseButton::Left);
            if ladle() == 0 && t > 2.0 {
                mouse.release(MouseButton::Left);
                info!("cast: poured at t {t:.1}");
                *state = 4;
            }
        }
        4 if t > 7.0 => {
            let (c, m) = (count(&sim, copper), count(&sim, molten));
            info!("cast: set: {:?} copper (cells, from x, to x), {} still molten", span(&c), m.len());
            for &p in &walls {
                sim.world.set(p, platypus_sim::Cell::AIR);
            }
            k.body.pos = Vec2::new(955.0, floor as f32 + k.body.half.y);
            k.prev_pos = k.body.pos;
            cursor.0 = Some(Vec2::new(1050.0, floor as f32 + 12.0));
            *state = 5;
        }
        5 if t > 7.5 => {
            kicks.write(crate::kick::Kick);
            *state = 6;
        }
        6 if t > 9.5 => {
            let flying: Vec<String> = sim.world.bodies().iter().map(|b| format!("{:.0},{:.0}", b.pos[0], b.pos[1])).collect();
            info!("cast: kicked: {:?} copper lying, bodies {flying:?}", span(&count(&sim, copper)));
            *state = 7;
        }
        _ => {}
    }
}

/// `boulder` (flat world): a round boulder (radius 10) let go at the top of a
/// ramp; it rolls down into a stone wall and breaks. Logs the pieces in
/// flight and the rock every quarter second.
fn boulder_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut state: Local<(u8, f32, Option<Vec2>)>) {
    if s.name != "boulder" {
        return;
    }
    let Ok(k) = player.single() else { return };
    let t = s.elapsed;
    let home = *state.2.get_or_insert(k.body.pos);
    let mats = sim.materials().clone();
    let (stone, boulder) = (mats.expect_id("stone"), mats.expect_id("boulder"));
    let Some(ground) = find_ground(&sim.world, home.x as i32, home.y as i32 + 60, 150) else { return };
    let (x0, y0) = (home.x as i32 + 45, ground);
    if state.0 == 0 && t > 0.5 {
        // A ramp 180 up, falling 1 in 1 to the right over 180 cells, and a
        // wall 150 further on.
        for dx in 0..180 {
            for y in y0..y0 + 180 - dx {
                sim.world.set(CellPos::new(x0 + dx, y), platypus_sim::Cell::new(stone, 0));
            }
        }
        for dx in 330..348 {
            for y in y0..y0 + 75 {
                sim.world.set(CellPos::new(x0 + dx, y), platypus_sim::Cell::new(stone, 0));
            }
        }
        let (cx, cy) = (x0 + 12, y0 + 180 + 12);
        for y in cy - 10..=cy + 10 {
            for x in cx - 10..=cx + 10 {
                if (x - cx) * (x - cx) + (y - cy) * (y - cy) <= 110 {
                    sim.world.set(CellPos::new(x, y), platypus_sim::Cell::new(boulder, 128));
                }
            }
        }
        sim.world.lift(CellPos::new(cx, cy));
        state.0 = 1;
    }
    if state.0 == 1 && t >= state.1 {
        state.1 = (t * 4.0).floor() / 4.0 + 0.25;
        let rock = (y0 - 8..y0 + 210).flat_map(|y| (x0 - 30..x0 + 390).map(move |x| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == boulder)).count();
        let flying: Vec<String> = sim.world.bodies().iter().map(|b| format!("{:.0},{:.0}", b.pos[0] - x0 as f32, b.pos[1] - y0 as f32)).collect();
        info!("boulder: t {t:.2} rock lying {rock}, bodies {flying:?}, rubble in flight {}", sim.world.particles().len());
    }
}

/// The `trap` scenario's steps: the step, when it's next logged, and where
/// the player starts and the rope's x.
type TrapWalk = (u8, f32, Option<(Vec2, i32)>);

/// `trap` (a generated world): to the boulder trap nearest the start
/// (found by generating the chunks round it), the player set down just
/// short of its wire (or plate) and walked in (D held). Logs the trap, the fall and the
/// player's health.
#[allow(clippy::too_many_arguments)]
fn trap_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut player: Query<(&mut Kinematics, &crate::creatures::Health), With<LocalPlayer>>,
    mut others: Query<(&Kinematics, &mut crate::creatures::Health), Others>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<TrapWalk>,
) {
    if s.name != "trap" {
        return;
    }
    let t = s.elapsed;
    let Ok((mut k, health)) = player.single_mut() else { return };
    let mats = sim.materials().clone();
    let (rope, boulder) = (mats.expect_id("rope"), mats.expect_id("boulder"));
    if state.0 == 0 && t > 0.3 {
        // Rings of chunks round the start, under the surface: the first rope
        // over a boulder.
        let home = sim.generator.spawn_point().chunk();
        let found = (0..90).find_map(|r: i32| {
            (-r..=r).flat_map(|dx| [dx]).flat_map(|dx| (-90..-6).map(move |dy| platypus_sim::ChunkPos::new(home.x + dx, home.y + dy))).filter(|p| (p.x - home.x).abs() == r).find_map(|p| {
                let c = sim.generator.generate(p);
                let o = p.origin();
                (0..platypus_sim::CHUNK).flat_map(|y| (0..platypus_sim::CHUNK).map(move |x| (x, y))).find(|&(x, y)| y > 0 && c.get(x as usize, y as usize).material == rope && c.get(x as usize, y as usize - 1).material == boulder).map(|(x, y)| (o.x + x, o.y + y))
            })
        });
        let Some((x, y)) = found else {
            info!("trap: no boulder trap near the start");
            state.0 = 9;
            return;
        };
        // The floor under it.
        let floor = (1..90).map(|d| y - d).find(|&fy| sim.generator.generate(CellPos::new(x, fy).chunk()).get((x - CellPos::new(x, fy).chunk().origin().x) as usize, (fy - CellPos::new(x, fy).chunk().origin().y) as usize).material != platypus_sim::MaterialId::AIR && fy < y - 18);
        info!("trap: a boulder trap, its rope at ({x}, {y}), the floor under it at {floor:?}");
        // (Set down at the wire's or plate's edge: the tunnel's floor is only
        // 21 wide, walled in either side.)
        // (Set down 16 short of it: the tunnel runs 21 either side.)
        state.2 = floor.map(|f| (Vec2::new(x as f32 - 16.0, f as f32 + 1.0), x));
        state.0 = 1;
    }
    let Some((start, x)) = state.2 else { return };
    match state.0 {
        1 => {
            // Held there while its chunks load.
            k.body.pos = start + Vec2::new(0.0, k.body.half.y);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            if t > 2.0 {
                // (The tunnel's own creatures, gone: to walk in undisturbed.)
                for (ok, mut h) in &mut others {
                    if ok.body.pos.distance(start) < 225.0 {
                        h.hp = 0.0;
                    }
                }
                state.0 = 2;
                state.1 = t;
            }
        }
        2 => {
            keys.press(KeyCode::KeyD);
            if k.body.pos.x > x as f32 + 30.0 || t > state.1 + 3.0 {
                keys.release(KeyCode::KeyD);
                state.0 = 3;
            }
            if t >= state.1 {
                state.1 = t + 0.25;
                info!("trap: t {t:.2} player at {:+.0} (the rope's x), health {:.0}, bodies {}", k.body.pos.x - x as f32, health.hp, sim.world.bodies().len());
            }
        }
        3 => {
            info!("trap: past it: health {:.0}, bodies {}", health.hp, sim.world.bodies().len());
            state.0 = 4;
        }
        _ => {}
    }
}

/// `village` (a generated world; try `PLATYPUS_HOUR=22`): from the start,
/// the village's people and stations logged, then a walk through the
/// village (D held) and a stop by each of them. Logs where each is and
/// what they're doing, every second.
fn village_script(
    s: Res<Scenario>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    people: Query<(&crate::creatures::Creature, &Kinematics, &crate::creatures::brain::villager::Routine, Option<&crate::creatures::brain::villager::Home>)>,
    stations: Query<&Kinematics, With<crate::craft::Station>>,
    day: Res<crate::light::Daylight>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<(u8, f32)>,
) {
    if s.name != "village" {
        return;
    }
    let t = s.elapsed;
    let Ok(k) = player.single() else { return };
    if state.0 == 0 && t > 2.0 {
        keys.press(KeyCode::KeyD);
        state.0 = 1;
    }
    // (Walk on till past the last of them, then stand.)
    let last = people.iter().map(|(_, pk, ..)| pk.body.pos.x).fold(f32::MIN, f32::max);
    if state.0 == 1 && k.body.pos.x > last + 45.0 {
        keys.release(KeyCode::KeyD);
        state.0 = 2;
    }
    if t >= state.1 {
        state.1 = t.floor() + 1.0;
        let who: Vec<String> = people
            .iter()
            .map(|(c, pk, r, home)| {
                let doing = if r.fleeing { "running" } else if r.talking { "talking" } else { "about" };
                format!("{} at {:+.0} (home {:+.0}) {doing}", c.kind, pk.body.pos.x - k.body.pos.x, home.map_or(0.0, |h| h.0.x - pk.body.pos.x))
            })
            .collect();
        info!("village: t {t:.0} ({}) player at x {:.0}; {} stations; {}", day.clock(), k.body.pos.x, stations.iter().count(), who.join("; "));
    }
}

/// `shop` (a generated world): the player, with 400 gold and hurt, walks
/// through the village (D held) and stops at the smith (buys a pickaxe and
/// ten firebricks), the healer (a heal, two potions) and the merchant
/// (sells it the pickaxe; buys ten torches, held over its slot to the
/// end, for the screenshot). Logs each trade: gold, health, pack.
#[allow(clippy::too_many_arguments)]
fn shop_script(
    s: Res<Scenario>,
    items: Option<Res<crate::hands::items::Items>>,
    mut player: Query<(&Kinematics, &mut crate::gold::Gold, &mut crate::creatures::Health, &mut crate::hands::items::Inventory), With<LocalPlayer>>,
    people: Query<(Entity, &crate::creatures::brain::villager::Villager, &crate::creatures::brain::villager::Routine)>,
    foes: Query<(&crate::creatures::Creature, &Kinematics, &crate::creatures::Team), Without<LocalPlayer>>,
    mut shop: ResMut<crate::talk::Shop>,
    mut open: ResMut<crate::hands::InventoryOpen>,
    mut held: ResMut<crate::hands::ui::Held>,
    mut trades: MessageWriter<crate::talk::Trade>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<(usize, u8, f32)>,
) {
    use crate::talk::Trade;
    if s.name != "shop" {
        return;
    }
    let t = s.elapsed;
    let (Some(items), Ok((pk, mut gold, mut health, mut inv))) = (items, player.single_mut()) else { return };
    let pk = pk.body.pos;
    const VISIT: [&str; 3] = ["smith", "healer", "merchant"];
    let (visit, step, at) = &mut *state;
    if *visit == 0 && *step == 0 {
        if t < 2.0 {
            return;
        }
        gold.0 = 400;
        health.hp = 30.0;
        keys.press(KeyCode::KeyD);
        *step = 1;
        return;
    }
    let Some(role) = VISIT.get(*visit) else { return };
    let log = |what: &str, gold: u32, hp: f32, inv: &crate::hands::items::Inventory| {
        let pack: Vec<String> = inv.slots.iter().flatten().map(|st| format!("{}×{}", items.def(st.item).name, st.count / items.unit(st.item))).collect();
        info!("shop: {role}: {what}: gold {gold}, health {hp:.0}, pack [{}]", pack.join(", "));
    };
    match *step {
        // Walking: stop when it turns to talk.
        1 => {
            if let Some((e, v, _)) = people.iter().find(|(_, v, r)| v.role == *role && r.talking) {
                keys.release(KeyCode::KeyD);
                *shop = crate::talk::open_for(e, v, &items, false);
                open.0 = true;
                *step = 2;
                *at = t;
                log("opened", gold.0, health.hp, &inv);
            }
        }
        2 if t > *at + 1.0 => {
            match *role {
                "smith" => {
                    trades.write(Trade::Take { offer: 0, n: 1 });
                    let last = shop.offers.len() - 1;
                    trades.write(Trade::Take { offer: last, n: 10 });
                }
                "healer" => {
                    trades.write(Trade::Take { offer: 0, n: 1 });
                    trades.write(Trade::Take { offer: 1, n: 2 });
                }
                _ => {
                    // The pickaxe, from the pack to the merchant.
                    let pick = items.id("iron_pickaxe");
                    if let Some(i) = inv.slots.iter().position(|st| st.is_some_and(|st| Some(st.item) == pick)) {
                        held.stack = inv.slots[i].take();
                        trades.write(Trade::Sell);
                    }
                    trades.write(Trade::Take { offer: 0, n: 10 });
                }
            }
            *step = 3;
            *at = t;
        }
        3 if t > *at + 0.5 => {
            let running = people.iter().any(|(_, v, r)| v.role == *role && r.fleeing);
            let near: Vec<String> = foes.iter().filter(|(_, fk, team)| **team == crate::creatures::Team::Enemy && fk.body.pos.distance(pk) < 300.0).map(|(c, fk, _)| format!("{} at {:+.0}", c.kind, fk.body.pos.x - pk.x)).collect();
            log(&if shop.with.is_some() { "traded".to_string() } else if running { format!("it ran ({})", near.join(", ")) } else { "shut".to_string() }, gold.0, health.hp, &inv);
            if *role == "merchant" {
                // (Torches in hand, over the slot: what they'd fetch.)
                let torch = items.id("torch");
                if let Some(i) = inv.slots.iter().position(|st| st.is_some_and(|st| Some(st.item) == torch)) {
                    held.stack = inv.slots[i].take();
                }
                *step = 5;
                return;
            }
            *step = 4;
            *at = t;
        }
        4 if t > *at + 1.5 => {
            open.0 = false;
            *visit += 1;
            *step = 1;
            keys.press(KeyCode::KeyD);
        }
        _ => {}
    }
}

/// `climb` (a generated world; seed 1's mountain is west of the start):
/// the player walks left (A held, jumping at a wall) up the mountain by
/// the village. Every second it logs where the player is and the cells
/// that changed, tick to tick, in a window round it (from → to, the
/// commonest first): what's moving on the slope.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn climb_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut free: ResMut<crate::camera::FreeCamera>,
    mut cam: Single<&mut Transform, With<MainCamera>>,
    mut glide_from: Local<Option<Vec2>>,
    mut state: Local<(f32, Option<(CellPos, Vec<platypus_sim::MaterialId>)>, std::collections::HashMap<(platypus_sim::MaterialId, platypus_sim::MaterialId), u32>, f32, u64)>,
) {
    const W: i32 = 300;
    const H: i32 = 210;
    if s.name != "climb" || s.elapsed < 1.0 {
        return;
    }
    let Ok(k) = player.single() else { return };
    let t = s.elapsed;
    // PLATYPUS_GLIDE=1: the camera, not the player, goes up the slope, at
    // a steady (-45, 30) cells a second from where the player starts:
    // camera motion alone, the same every run.
    if std::env::var("PLATYPUS_GLIDE").is_ok() {
        free.0 = true;
        let from = *glide_from.get_or_insert(k.body.pos);
        cam.translation.x = from.x - 45.0 * (t - 1.0);
        cam.translation.y = from.y + 30.0 * (t - 1.0);
        return;
    }
    let (next, snap, changes, jump_until, last_tick) = &mut *state;
    // (PLATYPUS_WAY=right: the other way.)
    let right = std::env::var("PLATYPUS_WAY").is_ok_and(|w| w == "right");
    keys.press(if right { KeyCode::KeyD } else { KeyCode::KeyA });
    let c = &k.loco.contacts;
    if (if right { c.wall_right } else { c.wall_left }) && k.loco.grounded() && t > *jump_until {
        *jump_until = t + 0.35;
    }
    if t < *jump_until {
        keys.press(KeyCode::Space);
    } else {
        keys.release(KeyCode::Space);
    }
    let tick = sim.world.tick();
    if tick != *last_tick {
        *last_tick = tick;
        if let Some((o, prev)) = snap.as_mut() {
            for y in 0..H {
                for x in 0..W {
                    let m = sim.world.get(CellPos::new(o.x + x, o.y + y)).map_or(platypus_sim::MaterialId::AIR, |c| c.material);
                    let i = (y * W + x) as usize;
                    if prev[i] != m {
                        *changes.entry((prev[i], m)).or_default() += 1;
                        prev[i] = m;
                    }
                }
            }
        }
    }
    if t >= *next || snap.is_none() {
        *next = t.floor() + 1.0;
        let mats = sim.world.materials();
        let mut top: Vec<_> = changes.drain().collect();
        top.sort_by_key(|x| std::cmp::Reverse(x.1));
        let total: u32 = top.iter().map(|x| x.1).sum();
        let list: Vec<String> = top.iter().take(6).map(|((a, b), n)| format!("{}→{} {n}", mats.def(*a).name, mats.def(*b).name)).collect();
        info!("climb: t {t:.0} at ({:.0}, {:.0}) {} changes: {}", k.body.pos.x, k.body.pos.y, total, list.join(", "));
        let o = CellPos::new(k.body.pos.x as i32 - W / 2, k.body.pos.y as i32 - H / 2);
        let cells = (0..H).flat_map(|y| (0..W).map(move |x| (x, y))).map(|(x, y)| sim.world.get(CellPos::new(o.x + x, o.y + y)).map_or(platypus_sim::MaterialId::AIR, |c| c.material)).collect();
        *snap = Some((o, cells));
    }
}

/// `star` (a generated world, try `PLATYPUS_HOUR=22`): a falling star
/// called just ahead of the player (the dev panel's; west, over the plain);
/// 4 s after it lands, what's in the crater (meteorite and mithril cells,
/// star wisps about). `PLATYPUS_STAR=away`: one fell 3 hours ago, 450 cells
/// west, where no one was; the player's taken there; the crater, and the
/// news.
#[allow(clippy::too_many_arguments)]
fn star_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut clock: ResMut<crate::clock::WorldClock>,
    news: Res<crate::events::News>,
    mut acts: MessageWriter<crate::dev::DevAction>,
    mut player: Query<(&mut Kinematics, &crate::creatures::Health), With<LocalPlayer>>,
    creatures: Query<(&crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    mut state: Local<(u8, f32, i32)>,
) {
    if s.name != "star" {
        return;
    }
    let t = s.elapsed;
    let Ok((mut k, health)) = player.single_mut() else { return };
    let away = std::env::var("PLATYPUS_STAR").is_ok_and(|v| v == "away");
    let (step, at, x) = &mut *state;
    match *step {
        0 if t > 2.0 => {
            // (West, over the plain: the village is east of the start.)
            *x = k.body.pos.x as i32 - if away { 450 } else { 210 };
            info!("star: the player at x {:.0}, health {:.0}", k.body.pos.x, health.hp);
            if away {
                let day = clock.now - 0.125;
                clock.events.push(crate::events::Happening { kind: crate::events::EventKind::Star, day, x: *x, stage: crate::events::Stage::Coming, from: 0 });
            } else {
                k.loco.facing = -1.0;
                acts.write(crate::dev::DevAction::Event);
            }
            *step = 1;
            *at = t;
        }
        1 if t > *at + 1.0 && away => {
            // (Over to it: just short of the crater's rim.)
            k.body.pos.x = *x as f32 + 60.0;
            k.prev_pos = k.body.pos;
            *step = 2;
            *at = t;
        }
        1 if t > *at + 6.0 => *step = 2,
        2 if t > *at + 1.0 => {
            let mats = sim.world.materials();
            let (meteorite, mithril) = (mats.id("meteorite"), mats.id("mithril_ore"));
            let ground = sim.generator.surface_hint(*x).unwrap_or(0);
            let (mut m, mut o) = (0, 0);
            for y in ground - 90..ground + 30 {
                for cx in *x - 60..*x + 60 {
                    let c = sim.world.get(CellPos::new(cx, y)).map(|c| c.material);
                    m += (c == meteorite) as u32;
                    o += (c == mithril) as u32;
                }
            }
            let wisps = creatures.iter().filter(|(c, ck)| c.kind == "star_wisp" && (ck.body.pos.x as i32 - *x).abs() < 180).count();
            let stage = clock.events.iter().find(|h| h.x == *x).map(|h| format!("{:?}", h.stage)).unwrap_or_default();
            info!("star: at x {x} ({stage}): {m} meteorite cells, {o} mithril ore, {wisps} star wisps; the player's health {:.0}; the news: {:?}", health.hp, news.0.first());
            *step = 3;
        }
        _ => {}
    }
}

type Raiders = (With<crate::creatures::brain::ai::Marching>, Without<LocalPlayer>);

/// `raid` (a generated world): a raid on the village from the east, the
/// player standing at the start; every 2 s, the raiders (alive, how far
/// from the village's middle), the villagers (alive, running) and the
/// player's health. `PLATYPUS_RAID=away`: one that came hours ago while no
/// one was there: the cells of the village missing; then the player 1050
/// cells west and 3 hours on, three times (mending); then the guide paid
/// and 3 hours on.
#[allow(clippy::too_many_arguments)]
fn raid_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut clock: ResMut<crate::clock::WorldClock>,
    mut village: ResMut<crate::events::Village>,
    news: Res<crate::events::News>,
    mut acts: MessageWriter<crate::dev::DevAction>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut player: Query<(&mut Kinematics, &crate::creatures::Health), With<LocalPlayer>>,
    raiders: Query<(&Kinematics, &crate::creatures::Health), Raiders>,
    folk: Query<(&crate::creatures::brain::villager::Villager, &crate::creatures::brain::villager::Routine, &crate::creatures::Health)>,
    mut state: Local<(u8, f32, f32)>,
) {
    if s.name != "raid" {
        return;
    }
    let t = s.elapsed;
    let Ok((mut k, health)) = player.single_mut() else { return };
    let Some((lo, hi)) = sim.generator.village() else { return };
    let mid = (lo.x + hi.x) / 2;
    let away = std::env::var("PLATYPUS_RAID").is_ok_and(|v| v == "away");
    let (step, at, next_log) = &mut *state;
    if *step == 0 && t > 2.0 {
        let day = if away { clock.now - 0.1 } else { clock.now };
        clock.events.push(crate::events::Happening { kind: crate::events::EventKind::Raid, day, x: mid, stage: crate::events::Stage::Coming, from: 1 });
        *step = 1;
        *at = t;
        return;
    }
    if !away {
        if *step == 1 && t >= *next_log {
            *next_log = t.floor() + 2.0;
            let r: Vec<String> = raiders.iter().map(|(rk, rh)| format!("{:+.0}{}", rk.body.pos.x - mid as f32, if rh.hp <= 0.0 { "†" } else { "" })).collect();
            let v: Vec<String> = folk.iter().map(|(v, r, h)| format!("{}{}", v.role, if h.hp <= 0.0 { " dead" } else if r.fleeing { " running" } else { "" })).collect();
            info!("raid: t {t:.0}: raiders at [{}] from the village's middle; villagers [{}]; the player's health {:.0}", r.join(" "), v.join(", "), health.hp);
        }
        return;
    }
    match *step {
        1 if t > *at + 1.0 => {
            info!("raid: away: {} cells of the village missing; the news: {:?}", village.missing, news.0.first());
            k.body.pos.x = mid as f32 - 1050.0;
            k.prev_pos = k.body.pos;
            *step = 2;
            *at = t;
        }
        2..=4 if t > *at + 1.5 => {
            acts.write(crate::dev::DevAction::Later);
            *step += 1;
            *at = t;
            info!("raid: away, {} hours on", (*step - 2) * 3);
        }
        5 if t > *at + 1.5 => {
            info!("raid: away, 9 hours on: the guide paid");
            village.paid = true;
            acts.write(crate::dev::DevAction::Later);
            *step = 6;
            *at = t;
        }
        6 if t > *at + 1.5 => {
            // (Back, walking in as anyone would: what's due is done as the
            // village loads, before it's in view.)
            k.body.pos.x = mid as f32 - 675.0;
            k.prev_pos = k.body.pos;
            keys.press(KeyCode::KeyD);
            *step = 7;
            *at = t;
        }
        7 if k.body.pos.x > mid as f32 - 90.0 || t > *at + 9.0 => {
            keys.release(KeyCode::KeyD);
            info!("raid: away, paid, walked back into the village (x {:+.0}): {} cells missing; village {}..{}", k.body.pos.x - mid as f32, village.missing, lo.x - mid, hi.x - mid);
            *step = 8;
        }
        _ => {}
    }
}

/// `quake` (a generated world, best with `PLATYPUS_SPAWN_Y` on a cave floor,
/// e.g. 4800): an earthquake felt where the player is; for its 5 s, the
/// rubble falling round the player each second (particles and bodies in
/// flight within 225 cells). `PLATYPUS_QUAKE=surface` (at the start): its
/// heart 300 cells west; the chasm there: how deep it's open below the
/// ground, how wide at the top.
fn quake_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut clock: ResMut<crate::clock::WorldClock>,
    news: Res<crate::events::News>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut state: Local<(u8, f32, i32, f32)>,
) {
    if s.name != "quake" {
        return;
    }
    let t = s.elapsed;
    let Ok(k) = player.single() else { return };
    let surface = std::env::var("PLATYPUS_QUAKE").is_ok_and(|v| v == "surface");
    let (step, at, x, next) = &mut *state;
    match *step {
        0 if t > 2.0 => {
            *x = k.body.pos.x as i32 - if surface { 300 } else { 0 };
            let day = clock.now;
            clock.events.push(crate::events::Happening { kind: crate::events::EventKind::Quake, day, x: *x, stage: crate::events::Stage::Coming, from: 0 });
            *step = 1;
            *at = t;
            *next = t + 1.0;
        }
        1 if t >= *next && t < *at + 6.0 => {
            *next += 1.0;
            let p = k.body.pos;
            let falling = sim.world.particles().iter().filter(|q| (Vec2::new(q.pos[0], q.pos[1]) - p).length() < 225.0).count();
            let bodies = sim.world.bodies().iter().filter(|b| (Vec2::new(b.pos[0], b.pos[1]) - p).length() < 225.0).count();
            info!("quake: t {:.0}: {falling} particles and {bodies} bodies in flight round the player", t - *at);
        }
        1 if t > *at + 6.0 => {
            if surface {
                let ground = sim.generator.surface_hint(*x).unwrap_or(0);
                let open = |cx: i32| (0..300).take_while(|d| sim.world.get(CellPos::new(cx, ground + 3 - d)).is_some_and(|c| c.is_air())).count();
                let deep = (*x - 9..=*x + 9).map(open).max().unwrap_or(0);
                let wide = (*x - 30..=*x + 30).filter(|&cx| sim.world.get(CellPos::new(cx, ground - 3)).is_some_and(|c| c.is_air())).count();
                info!("quake: the chasm at x {x}: open {deep} cells down, {wide} wide at the top");
            }
            info!("quake: the news: {:?}", news.0.first());
            *step = 2;
        }
        _ => {}
    }
}

/// `pedlar` (a generated world): a pedlar comes to the village (from the
/// east), the player at the start; every 2 s where it is from the
/// village's middle; at 14 s its wares (its shop opened); then a day ahead
/// (its stay over): it walks off, and is gone once out of sight.
#[allow(clippy::too_many_arguments)]
fn pedlar_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    mut clock: ResMut<crate::clock::WorldClock>,
    news: Res<crate::events::News>,
    mut acts: MessageWriter<crate::dev::DevAction>,
    folk: Query<(Entity, &crate::creatures::brain::villager::Villager, &Kinematics)>,
    mut state: Local<(u8, f32, f32)>,
) {
    if s.name != "pedlar" {
        return;
    }
    let t = s.elapsed;
    let Some((lo, hi)) = sim.generator.village() else { return };
    let mid = (lo.x + hi.x) as f32 / 2.0;
    let (step, at, next) = &mut *state;
    let pedlar = folk.iter().find(|(_, v, _)| v.role == "pedlar");
    if *step == 0 && t > 2.0 {
        let day = clock.now;
        clock.events.push(crate::events::Happening { kind: crate::events::EventKind::Pedlar, day, x: mid as i32, stage: crate::events::Stage::Coming, from: 1 });
        *step = 1;
        *at = t;
        *next = t + 2.0;
        return;
    }
    if *step >= 1 && t >= *next {
        *next += 2.0;
        info!("pedlar: t {:.0}: {}; the news: {:?}", t - *at, pedlar.map_or("none about".to_string(), |(_, _, k)| format!("at {:+.0} from the village's middle", k.body.pos.x - mid)), news.0.first());
    }
    if *step == 1 && t > *at + 12.0 {
        if let (Some((e, v, _)), Some(items)) = (pedlar, items) {
            let shop = crate::talk::open_for(e, v, &items, false);
            let wares: Vec<String> = shop.offers.iter().map(|o| match o {
                crate::talk::Offer::Buy { item, price } => format!("{} {price}", items.def(*item).name),
                other => format!("{other:?}"),
            }).collect();
            info!("pedlar: its wares: {}", wares.join(", "));
        }
        acts.write(crate::dev::DevAction::DayAhead);
        *step = 2;
    }
}

/// `troll` (arena world): a troll 90 cells off, struck by a blade (24, a
/// longsword's, four a second) for 8 s: its health each second (it can't
/// die while it heals); then set alight, still struck: when it dies.
#[allow(clippy::too_many_arguments)]
fn troll_script(
    mut commands: Commands,
    s: Res<Scenario>,
    coatings: Res<crate::creatures::body::elements::Coatings>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    trolls: Query<(Entity, &crate::creatures::Creature, &Kinematics, &crate::creatures::Health)>,
    burns: Query<&crate::creatures::body::elements::Burning>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, f32, f32, Option<Entity>)>,
) {
    if s.name != "troll" {
        return;
    }
    let t = s.elapsed;
    let Ok(pk) = player.single() else { return };
    let (step, next_hit, next_log, troll) = &mut *state;
    if *step == 0 && t > 1.0 {
        crate::creatures::def::spawn_creature(&mut commands, "troll", pk.body.pos + Vec2::new(90.0, -pk.body.half.y), |_| {});
        *step = 1;
        return;
    }
    if troll.is_none() {
        *troll = trolls.iter().find(|(_, c, ..)| c.kind == "troll").map(|(e, ..)| e);
        *next_hit = t;
        *next_log = t.floor() + 1.0;
        return;
    }
    let Ok((e, _, k, h)) = trolls.get(troll.unwrap()) else {
        if *step < 9 {
            info!("troll: dead at t {t:.1}");
            *step = 9;
        }
        return;
    };
    if t >= *next_hit {
        *next_hit += 0.25;
        hits.write(crate::combat::Hit { target: e, damage: 24.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at: k.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
    }
    if *step == 1 && t > 11.0 {
        // (Out of the pool of its own blood first, and wiped dry: soaked
        // in it, as anything 40 % wet, it wouldn't catch.)
        commands.entity(e).queue(|mut ew: EntityWorldMut| {
            if let Some(mut k) = ew.get_mut::<Kinematics>() {
                k.body.pos.x += 60.0;
                k.prev_pos = k.body.pos;
            }
            ew.remove::<crate::creatures::body::elements::Coated>();
        });
        crate::creatures::body::elements::catch_fire(&mut commands, e, &h.nature, None, &coatings, 1.0);
        info!("troll: set alight at t {t:.1} (health {:.0})", h.hp);
        *step = 2;
    }
    if t >= *next_log {
        *next_log += 1.0;
        let fire = burns.get(e).map_or(String::new(), |b| format!(", burning ({:.0} %)", b.share * 100.0));
        info!("troll: t {t:.0}: health {:.0} / {:.0}{fire}", h.hp, h.max);
    }
}

/// `reactions`: a skeleton standing still, struck lightly with each kind of
/// hurt in turn (DESIGN §14.3): how each lands (hurt, resisted, absorbed);
/// then slashes until it drips (under half) and falters (under a quarter);
/// the player is stung twice, and the skeleton clubbed to pieces. Then the
/// arena's readout (`fight.rs`) and what the player saw (`observe.rs`).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn reactions_script(
    mut commands: Commands,
    s: Res<Scenario>,
    player: Query<(Entity, &Kinematics, Option<&crate::progress::Progress>), With<LocalPlayer>>,
    foes: Query<(Entity, &crate::creatures::Creature, &crate::creatures::Health, Has<crate::creatures::body::hurt::Faltering>)>,
    mut reacted: MessageReader<crate::creatures::body::hurt::Reacted>,
    mut hits: MessageWriter<crate::combat::Hit>,
    fight: Res<crate::fight::Fight>,
    mut state: Local<(u8, f32, usize, bool, bool)>,
) {
    use crate::creatures::Harm;
    if s.name != "reactions" {
        return;
    }
    const KINDS: [Harm; 10] = [Harm::Slash, Harm::Pierce, Harm::Blunt, Harm::Fire, Harm::Frost, Harm::Storm, Harm::Acid, Harm::Poison, Harm::Radiant, Harm::Void];
    let t = s.elapsed;
    let Ok((me, pk, progress)) = player.single() else { return };
    let (step, next, n, dripping, faltering) = &mut *state;
    if *step == 0 && t > 0.5 {
        crate::creatures::def::spawn_creature(&mut commands, "skeleton", pk.body.pos + Vec2::new(90.0, -pk.body.half.y), |e| {
            e.remove::<crate::creatures::brain::hunter::Hunter>();
        });
        *step = 1;
        *next = 1.5;
        return;
    }
    let Some((e, _, h, falters)) = foes.iter().find(|(_, c, ..)| c.kind == "skeleton") else {
        if *step == 2 {
            *step = 3;
            *next = t + 1.0;
        }
        if *step == 3 && t > *next {
            *step = 4;
            info!("reactions: the readout:\n{}", *fight);
            if let Some(seen) = progress.and_then(|p| p.observed.get("skeleton")) {
                let took: Vec<String> = seen.took.iter().map(|(k, v)| format!("{k} {:?}", v.reaction())).collect();
                info!("reactions: seen of skeletons: met {}, felled by {:?}, took: {}", seen.met, seen.felled, took.join(", "));
            }
        }
        return;
    };
    for r in reacted.read().filter(|r| r.target == e) {
        info!("reactions: skeleton, {:?}: {:?} ({:+.0} % of its health), now {:.1} hp", r.harm, r.reaction, -r.share * 100.0, h.hp);
    }
    if *step == 1 && t >= *next {
        // Each kind in turn; then slashes.
        let harm = KINDS.get(*n).copied().unwrap_or(Harm::Slash);
        hits.write(crate::combat::Hit { target: e, damage: 3.0, harm, knock: Vec2::ZERO, stun: 0.0, at: pk.body.pos + Vec2::new(90.0, 0.0), dir: Vec2::X, weight: 1.0, crit: false });
        *n += 1;
        *next = t + 0.5;
    }
    if !*dripping && h.hp < h.max * 0.5 {
        *dripping = true;
        info!("reactions: under half its health ({:.1} of {:.0}): it drips", h.hp, h.max);
    }
    if !*faltering && falters {
        *faltering = true;
        info!("reactions: under a quarter ({:.1}): it falters", h.hp);
        *step = 2;
        *n = 0;
    }
    // The player stung twice, then the skeleton clubbed till it's dead.
    if *step == 2 && t >= *next {
        let (target, harm, at) = if *n < 2 { (me, Harm::Pierce, pk.body.pos) } else { (e, Harm::Blunt, pk.body.pos + Vec2::new(90.0, 0.0)) };
        hits.write(crate::combat::Hit { target, damage: 5.0, harm, knock: Vec2::ZERO, stun: 0.0, at, dir: Vec2::X, weight: 1.0, crit: false });
        *n += 1;
        *next = t + 0.5;
    }
}

/// `grab`: a troll beside a player standing still: it snatches the player
/// up, squeezes twice and hurls them; a second grab is broken by striking
/// the troll hard (stunned, it lets go). Then a skeleton given the
/// `fire_ray` move sweeps its ray at the player (each part from the same
/// place, healed). Logs each phase, where the
/// player is held, its health, the throw and the ray.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn grab_script(
    mut commands: Commands,
    s: Res<Scenario>,
    book: Res<crate::creatures::moves::MoveBook>,
    player: Query<(Entity, &Kinematics, &crate::creatures::Health, Option<&crate::creatures::moves::Held>, Has<crate::creatures::body::elements::Burning>), With<LocalPlayer>>,
    foes: Query<(Entity, &crate::creatures::Creature, &Kinematics, &crate::creatures::moves::Moves), Without<LocalPlayer>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, f32, String, bool, f32, Vec2, f32)>,
) {
    if s.name != "grab" {
        return;
    }
    let t = s.elapsed;
    let Ok((me, pk, ph, held, burning)) = player.single() else { return };
    let (step, next, last, was_held, hp0, home, x0) = &mut *state;
    let doing = |kind: &str| foes.iter().find(|f| f.1.kind == kind).and_then(|f| f.3.doing(&book).map(|(id, _, ph)| format!("{id} {ph}")));
    // Each part from the same place on the open floor, healed, nothing about.
    let fresh = |commands: &mut Commands, home: Vec2| {
        for (e, ..) in &foes {
            commands.entity(e).despawn();
        }
        commands.entity(me).queue(move |mut ew: EntityWorldMut| {
            if let Some(mut k) = ew.get_mut::<Kinematics>() {
                k.body.pos = home;
                k.body.vel = Vec2::ZERO;
                k.prev_pos = home;
            }
            if let Some(mut h) = ew.get_mut::<crate::creatures::Health>() {
                h.hp = h.max;
            }
        });
    };
    match *step {
        0 if t > 0.5 => {
            *home = pk.body.pos;
            crate::creatures::def::spawn_creature(&mut commands, "troll", pk.body.pos + Vec2::new(36.0, -pk.body.half.y), |_| {});
            *step = 1;
            *hp0 = ph.hp;
        }
        1 | 2 => {
            let now = doing("troll").unwrap_or_default();
            if now != *last {
                info!("grab: t {t:.2} troll: {}  (player hp {:.0})", if now.is_empty() { "-" } else { &now }, ph.hp);
                *last = now.clone();
            }
            if held.is_some() != *was_held {
                *was_held = held.is_some();
                if *was_held {
                    let tk = foes.iter().find(|f| f.1.kind == "troll").map(|f| f.2.body.pos).unwrap_or_default();
                    info!("grab: t {t:.2} the player is held, {:?} from the troll's middle", (pk.body.pos - tk).round());
                    if *step == 2
                        && let Some((te, ..)) = foes.iter().find(|f| f.1.kind == "troll")
                    {
                        // Struck hard enough to stagger it: it lets go.
                        hits.write(crate::combat::Hit { target: te, damage: 90.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.5, at: pk.body.pos, dir: Vec2::X, weight: 2.0, crit: false });
                        info!("grab: t {t:.2} the troll struck hard");
                    }
                } else {
                    info!("grab: t {t:.2} let go, moving {:?} (hp {:.0}, {:.0} before)", pk.body.vel.round(), ph.hp, *hp0);
                    *x0 = pk.body.pos.x;
                    *next = t + 1.5;
                }
            }
            if !*was_held && *next > 0.0 && t > *next {
                info!("grab: t {t:.2} landed {:.0} cells from where it was let go, hp {:.0}", pk.body.pos.x - *x0, ph.hp);
                *next = 0.0;
                *step += 1;
                fresh(&mut commands, *home);
                *hp0 = 100.0;
                if *step == 2 {
                    crate::creatures::def::spawn_creature(&mut commands, "troll", *home + Vec2::new(36.0, -pk.body.half.y), |_| {});
                } else {
                    // (West of the player: the dummies stand east, in the ray's way.)
                    crate::creatures::def::spawn_creature(&mut commands, "skeleton", *home + Vec2::new(-150.0, -pk.body.half.y), |e| {
                        e.remove::<crate::creatures::brain::hunter::Hunter>();
                        e.insert(crate::creatures::moves::Moves::new(vec!["fire_ray".into()]));
                    });
                    *next = t + 4.0;
                }
            }
        }
        3 => {
            let now = doing("skeleton").unwrap_or_default();
            if now != *last {
                info!("grab: t {t:.2} skeleton: {}  (player hp {:.0}{})", if now.is_empty() { "-" } else { &now }, ph.hp, if burning { ", burning" } else { "" });
                *last = now;
            }
            if t > *next {
                info!("grab: the ray: the player at hp {:.0}{}", ph.hp, if burning { ", burning" } else { "" });
                *step = 4;
            }
        }
        _ => {}
    }
}

/// `bestiary`: the bestiary opened, `PLATYPUS_PICK` (default the troll)
/// opened in it, `PLATYPUS_SEARCH` typed; logs the cards shown and the
/// opened one's details (for a screenshot: `PLATYPUS_SCREENSHOT`).
fn bestiary_script(s: Res<Scenario>, mut b: ResMut<crate::bestiary::panel::Bestiary>, stage: Option<Res<crate::bestiary::stage::Stage>>, mut done: Local<bool>, mut said: Local<String>) {
    if s.name != "bestiary" || s.elapsed < 1.0 {
        return;
    }
    // What the live stage shows, as it changes.
    if let Some(st) = &stage
        && st.caption != *said
    {
        info!("bestiary: t {:.2} stage: {}", s.elapsed, st.caption);
        said.clone_from(&st.caption);
    }
    if *done {
        return;
    }
    *done = true;
    b.open = true;
    b.search = std::env::var("PLATYPUS_SEARCH").unwrap_or_default();
    let pick = std::env::var("PLATYPUS_PICK").unwrap_or_else(|_| "troll".into());
    let shown: Vec<String> = b.shown().map(|e| e.id.clone()).collect();
    info!("bestiary: {} cards shown: {}", shown.len(), shown.join(", "));
    if let Some(e) = b.shown().find(|e| e.id == pick) {
        for (head, lines) in e.details() {
            info!("bestiary: {}: {head}: {}", e.id, lines.join(" | "));
        }
    }
    b.picked = Some(pick);
}

/// `reload`: a troll spawned, then its moves, brain and touch taken away
/// and its health cap lowered, as if its file had been different; the
/// bestiary's Reload (`ReloadCreatures`): it has them back, mid-fight.
#[allow(clippy::type_complexity)]
fn reload_script(
    mut commands: Commands,
    s: Res<Scenario>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut trolls: Query<(Entity, &crate::creatures::Creature, &mut crate::creatures::Health, Has<crate::creatures::moves::Moves>, Has<crate::creatures::brain::hunter::Hunter>, Has<crate::combat::Sturdy>)>,
    mut reload: MessageWriter<crate::creatures::def::ReloadCreatures>,
    mut step: Local<u8>,
) {
    if s.name != "reload" {
        return;
    }
    let t = s.elapsed;
    let troll = trolls.iter_mut().find(|q| q.1.kind == "troll");
    match (*step, troll) {
        (0, _) if t > 0.5 => {
            let Ok(pk) = player.single() else { return };
            crate::creatures::def::spawn_creature(&mut commands, "troll", pk.body.pos + Vec2::new(200.0, -pk.body.half.y), |_| {});
            *step = 1;
        }
        (1, Some((e, _, mut h, moves, brain, sturdy))) if t > 1.0 => {
            info!("reload: before: moves {moves}, brain {brain}, poise {sturdy}, health {:.0} / {:.0}", h.hp, h.max);
            commands.entity(e).remove::<(crate::creatures::moves::Moves, crate::creatures::brain::hunter::Hunter, crate::combat::Sturdy)>();
            h.max = 100.0;
            h.hp = 100.0;
            *step = 2;
        }
        (2, Some((_, _, h, moves, brain, sturdy))) if t > 1.5 => {
            info!("reload: changed: moves {moves}, brain {brain}, poise {sturdy}, health {:.0} / {:.0}", h.hp, h.max);
            reload.write(crate::creatures::def::ReloadCreatures);
            *step = 3;
        }
        (3, Some((_, _, h, moves, brain, sturdy))) if t > 2.0 => {
            info!("reload: after the reload: moves {moves}, brain {brain}, poise {sturdy}, health {:.0} / {:.0}", h.hp, h.max);
            *step = 4;
        }
        _ => {}
    }
}

/// `layouts`: the arena's floor made each way in turn from its panel (the
/// world reset, the player kept), 3 s each; logs where the player stands
/// and what's under it after each.
fn layouts_script(s: Res<Scenario>, sim: Res<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut acts: MessageWriter<crate::arena::ArenaAction>, mut step: Local<usize>) {
    if s.name != "layouts" {
        return;
    }
    const ALL: [&str; 6] = ["flat", "cave", "slopes", "stairs", "real", "sandbox"];
    let t = s.elapsed;
    if t < 1.0 + *step as f32 * 3.0 {
        return;
    }
    if *step > 0
        && let Ok(k) = player.single()
    {
        let feet = k.body.pos - Vec2::Y * (k.body.half.y + 1.0);
        let under = sim.world.get(CellPos::from_world(feet.x, feet.y)).map_or("nothing".to_string(), |c| sim.materials().def(c.material).name.clone());
        info!("layouts: {}: the player at {:?}, standing {} on {under}", ALL[*step - 1], k.body.pos.round(), k.loco.grounded());
    }
    if let Some(l) = ALL.get(*step) {
        acts.write(crate::arena::ArenaAction::Layout((*l).into()));
    }
    *step += 1;
}

/// `record`: a fight recorded from the arena panel (`replay.rs`): an orc
/// put down, the shortsword taken (hotbar 2, slot 8), the player stepping
/// in and swinging at it for 9 s, then stopped and saved
/// (`saves/fights/fight-N.ron`, or `PLATYPUS_SAVE_DIR`'s). Play it back
/// as a test with `PLATYPUS_REPLAY=` that file.
#[allow(clippy::too_many_arguments)]
fn record_script(
    s: Res<Scenario>,
    tape: Res<crate::replay::Tape>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    foes: Query<(&crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    mut acts: MessageWriter<crate::replay::TapeAction>,
    mut dev: MessageWriter<crate::dev::DevAction>,
    mut kind: ResMut<crate::creatures::spawn::SpawnKind>,
    (mut cursor, mut keys, mut mouse): (ResMut<CursorOverride>, ResMut<ButtonInput<KeyCode>>, ResMut<ButtonInput<MouseButton>>),
    mut state: Local<(u8, f32)>,
) {
    if s.name != "record" {
        return;
    }
    let t = s.elapsed;
    let (step, since) = &mut *state;
    if *step == 0 && t > 1.0 {
        acts.write(crate::replay::TapeAction::Record);
        *step = 1;
        return;
    }
    if *step == 1 && tape.recording() {
        *step = 2;
        *since = t;
    }
    if *step < 2 {
        return;
    }
    let r = t - *since;
    let Ok(pk) = player.single() else { return };
    let mut want = std::collections::HashSet::new();
    let window = |a: f32, b: f32| r >= a && r < b;
    if *step == 2 && r >= 0.6 {
        *step = 3;
        kind.0 = "orc".into();
        dev.write(crate::dev::DevAction::Spawn(Some(pk.body.pos + Vec2::new(120.0, 0.0))));
    }
    if window(0.8, 0.9) {
        want.insert(KeyCode::KeyX);
    }
    if window(1.0, 1.1) {
        want.insert(KeyCode::Digit8);
    }
    let orc = foes.iter().find(|(c, _)| c.kind == "orc").map(|(_, k)| k.body.pos);
    let mut swing = false;
    if let Some(o) = orc
        && window(1.3, 10.3)
    {
        if (o.x - pk.body.pos.x).abs() > 22.0 {
            want.insert(if o.x > pk.body.pos.x { KeyCode::KeyD } else { KeyCode::KeyA });
        }
        cursor.0 = Some(o);
        swing = true;
    }
    for k in [KeyCode::KeyA, KeyCode::KeyD, KeyCode::KeyX, KeyCode::Digit8] {
        match (want.contains(&k), keys.pressed(k)) {
            (true, false) => keys.press(k),
            (false, true) => keys.release(k),
            _ => {}
        }
    }
    match (swing, mouse.pressed(MouseButton::Left)) {
        (true, false) => mouse.press(MouseButton::Left),
        (false, true) => mouse.release(MouseButton::Left),
        _ => {}
    }
    if *step == 3 && r > 10.5 {
        cursor.0 = None;
        acts.write(crate::replay::TapeAction::Stop);
        info!("record: stopped at {r:.1} s; the orc {}", if orc.is_some() { "still standing" } else { "dead" });
        *step = 4;
    }
}

/// `course` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=course`): the player on
/// the course's tower, a creature (`PLATYPUS_KIND`, default the orc) put
/// down at the far left; logs where it is each second and how near it
/// is, and when it gets to the player (within 40 cells), or that it
/// didn't in `PLATYPUS_SCENARIO_SECS`. `PLATYPUS_NONAV=1`: with no way
/// found, to compare.
fn course_script(
    mut commands: Commands,
    s: Res<Scenario>,
    ways: Res<crate::creatures::brain::way::Ways>,
    player: Query<(&Kinematics, &crate::creatures::Health), With<LocalPlayer>>,
    foes: Query<(&crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    mut state: Local<(u8, f32, String)>,
) {
    if s.name != "course" {
        return;
    }
    let t = s.elapsed;
    let Ok((pk, _)) = player.single() else { return };
    let (step, next, kind) = &mut *state;
    if *step == 0 && t > 1.0 {
        *kind = std::env::var("PLATYPUS_KIND").unwrap_or_else(|_| "orc".into());
        // (The ground kept loaded round it, and it knows where the player is.)
        let aware = pk.body.pos;
        crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(150.0, platypus_worldgen::arena::FLOOR as f32), move |e| {
            e.insert((crate::world::ChunkLoader { half_extent: Vec2::new(200.0, 200.0) }, crate::creatures::brain::senses::Alert::hunting(aware)));
            if let Some(mut h) = e.get_mut::<crate::creatures::brain::hunter::Hunter>() {
                h.aggro = 4000.0;
            }
        });
        info!("course: the player at {:?}; a {kind} put down at the far left", pk.body.pos.round());
        *step = 1;
        *next = t + 1.0;
        return;
    }
    let Some((_, fk)) = foes.iter().find(|(c, _)| c.kind == *kind) else { return };
    let d = fk.body.pos.distance(pk.body.pos);
    if *step == 1 && d < 40.0 {
        info!("course: the {kind} got to the player at t {:.1} (it set out at 1.0); {} searches, {} nodes, {} µs", t, ways.searches, ways.looked, ways.micros);
        *step = 2;
    }
    if *step == 1 && t >= *next {
        *next += 1.0;
        info!("course: t {:.0}: the {kind} at {:?}, {:.0} from the player", t, fk.body.pos.round(), d);
    }
    if *step == 1 && t > s.duration - 1.2 {
        info!("course: the {kind} didn't get there; {:.0} away at the end", d);
        *step = 3;
    }
}

/// `legs` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=real`: real ground): a
/// legged walker (`PLATYPUS_KIND`, default the stilt stalker) put down 300
/// cells west of the player (over the hills west of seed 1's start;
/// `PLATYPUS_WAY=right`: east, into the village), the player moved once to
/// wait on the ground `PLATYPUS_AHEAD` cells beyond it (600; inside the
/// walls), standing still; the walker goes after it all the way, the
/// camera following the walker. Logs twice a
/// second where it is, the ground's slope under it, its body's tilt and
/// height over its feet, how many feet hold the ground and whether any
/// foot is in rock; at the end, how its tilt followed the slope.
/// `PLATYPUS_SHOTS=dir`: a picture every 2 s into it (`PLATYPUS_SHOT_EVERY`
/// s: another gap). `PLATYPUS_KILL=1`:
/// it's struck dead 3 s from the end (to see its body); `PLATYPUS_FEETLOG=1`:
/// each foot (from its box's bottom; `~`: holding nothing), and every half
/// second where the player is and the walker's speed, footing, grip and
/// what it wants (its controls);
/// `PLATYPUS_PRICK=1`: a 3-hp cut at 4 s (does it bleed to death?);
/// `PLATYPUS_TAILLOG=1`: its first chain's tip each frame, whether it aims.
/// Struck, the player walks away from it for `PLATYPUS_RETREAT` s (1.4),
/// so it walks after the player again before each attack.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn legs_script(
    mut commands: Commands,
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    offscreen: Option<Res<crate::camera::Offscreen>>,
    mut player: Query<(Entity, &mut Kinematics), With<LocalPlayer>>,
    foes: Query<(&crate::creatures::Creature, &Kinematics, Option<&crate::creatures::body::legs::Legs>), Without<LocalPlayer>>,
    intents: Query<(&crate::creatures::Creature, &crate::creatures::Controls)>,
    mut healths: Query<(Entity, &crate::creatures::Creature, &mut crate::creatures::Health), Without<LocalPlayer>>,
    mut took: MessageReader<crate::creatures::Took>,
    mut began: MessageReader<crate::creatures::moves::Began>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut retreat: Local<f32>,
    mut state: Local<(u8, f32, String, Vec<(f32, f32)>, f32, f32, (f32, u32, u32))>,
) {
    if s.name != "legs" {
        return;
    }
    // What hurts it (it should be nothing, walking); with
    // PLATYPUS_TAILLOG, what hurts the player too.
    let me_hurt = player.single().ok().map(|(e, _)| e);
    // With PLATYPUS_TAILLOG, each move it begins, and how far off.
    for b in began.read() {
        if std::env::var("PLATYPUS_TAILLOG").is_ok()
            && let (Ok((_, fk, _)), Ok((_, pk))) = (foes.get(b.who), player.single())
        {
            info!("legs: t {:.2}: it began {} {:.0} across, {:.0} up", s.elapsed, b.id, (pk.body.pos.x - fk.body.pos.x).abs(), fk.body.pos.y - pk.body.pos.y);
        }
    }
    for tk in took.read() {
        if healths.get(tk.target).is_ok_and(|(_, c, _)| c.kind == state.2) {
            // (Real hurts only: not a spider drinking its own acid, 0.0 a
            // tick.)
            if tk.dealt >= 0.5 || tk.killed {
                warn!("legs: t {:.1}: the {} took {:.1} {:?}{}", s.elapsed, state.2, tk.dealt, tk.harm, if tk.killed { ", and died" } else { "" });
            }
        } else if Some(tk.target) == me_hurt {
            // (Not venom's every tick.)
            if std::env::var("PLATYPUS_TAILLOG").is_ok() && tk.dealt >= 1.0 {
                info!("legs: t {:.2}: the player took {:.1} {:?}", s.elapsed, tk.dealt, tk.harm);
            }
            // Struck (not venom ticking): it backs off a while, so the
            // walker walks after it again before the next.
            if tk.dealt > 1.0 {
                *retreat = s.elapsed + std::env::var("PLATYPUS_RETREAT").ok().and_then(|v| v.parse().ok()).unwrap_or(1.4);
            }
        }
    }
    let t = s.elapsed;
    let Ok((me, mut pk)) = player.single_mut() else { return };
    let (step, next, kind, pairs, shot, dir, strain) = &mut *state;
    // The ground's top under x, near y.
    let ground = |x: f32, y: f32| -> Option<f32> {
        let x = x.floor() as i32;
        let solid = |y: i32| sim.world.get(CellPos::new(x, y)).is_some_and(|c| matches!(sim.world.materials().phys(c.material).kind, platypus_sim::Kind::Static | platypus_sim::Kind::Powder));
        let top = y as i32 + 120;
        (0..300).map(|d| top - d).find(|&y| solid(y) && !solid(y + 1)).map(|y| (y + 1) as f32)
    };
    if *step == 0 && t > 1.0 {
        *kind = std::env::var("PLATYPUS_KIND").unwrap_or_else(|_| "stalker".into());
        *dir = if std::env::var("PLATYPUS_WAY").is_ok_and(|w| w == "right") { 1.0 } else { -1.0 };
        let x = pk.body.pos.x + 300.0 * *dir;
        let Some(y) = ground(x, pk.body.pos.y) else { return error!("legs: no ground at {x}") };
        // The player waits further on, standing still; the camera goes
        // with the walker.
        let gap = std::env::var("PLATYPUS_AHEAD").ok().and_then(|v| v.parse().ok()).unwrap_or(600.0);
        // (Inside the arena's walls, there.)
        let px = x + gap * *dir;
        let px = if std::env::var("PLATYPUS_WORLD").is_ok_and(|w| w == "arena") { px.clamp(70.0, 1850.0) } else { px };
        // (Its chunks aren't loaded yet that far off: the ground as the
        // world was made there.)
        match ground(px, pk.body.pos.y).or_else(|| sim.generator.surface_hint(px as i32).map(|y| y as f32)) {
            Some(py) => {
                pk.body.pos = Vec2::new(px, py + pk.body.half.y + 0.5);
                pk.body.vel = Vec2::ZERO;
                info!("legs: the player moved to wait at {:?}", pk.body.pos.round());
            }
            None => warn!("legs: no ground at {px} for the player to wait on"),
        }
        commands.entity(me).remove::<crate::camera::CameraTarget>();
        let aware = pk.body.pos;
        crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(x, y), move |e| {
            e.insert((crate::world::ChunkLoader { half_extent: Vec2::new(200.0, 200.0) }, crate::camera::CameraTarget, crate::creatures::brain::senses::Alert::hunting(aware)));
            if let Some(mut h) = e.get_mut::<crate::creatures::brain::hunter::Hunter>() {
                h.aggro = 4000.0;
            }
        });
        info!("legs: a {kind} put down at ({x:.0}, {y:.0}), the player at {:?}", pk.body.pos.round());
        *step = 1;
        *next = t + 1.0;
        *shot = t + 2.0;
        return;
    }
    if *step == 1 && t > s.duration - 3.0 && std::env::var("PLATYPUS_KILL").is_ok() {
        for (_, c, mut h) in &mut healths {
            if c.kind == *kind {
                h.hp = 0.0;
                info!("legs: the {kind} struck dead at t {t:.1}");
            }
        }
        *step = 3;
    }
    let Some((_, fk, legs)) = foes.iter().find(|(c, _, _)| c.kind == *kind) else { return };
    let wants = intents.iter().find(|(c, _)| c.kind == *kind).map(|(_, c)| (c.0.move_x, c.0.move_y));
    // (Its legs' strain, every frame: the most, and how often past reach.)
    if *step == 1
        && let Some(l) = legs
    {
        if l.strain() > 1.3 && std::env::var("PLATYPUS_FEETLOG").is_ok() {
            let b = Vec2::new(fk.body.pos.x, fk.body.pos.y - fk.body.half.y);
            let fs: Vec<String> = l.feet().map(|(p, g)| format!("{:?}{}", (p - b).round(), if g { "" } else { "~" })).collect();
            info!("legs: t {t:.2}: strain {:.2} at {:?} vel {:?} grounded {}; feet from its box's bottom {}", l.strain(), fk.body.pos.round(), fk.body.vel.round(), fk.loco.grounded(), fs.join(" "));
        }
        if std::env::var("PLATYPUS_TAILLOG").is_ok() {
            let (tips, aiming, facing) = l.chain_state();
            if let Some(tip) = tips.first() {
                let r = *tip;
                info!("taillog t {t:.3} tip {:?} from body Vec2({:.1}, {:.1}) aiming {aiming} facing {facing} vel {:?} gap {:.0}", tip.round(), r.x, r.y, fk.body.vel.round(), (pk.body.pos.x - fk.body.pos.x).abs());
            }
        }
        strain.0 = strain.0.max(l.strain());
        strain.1 += (l.strain() > 1.0) as u32;
        strain.2 += 1;
    }
    if *step == 1 && std::env::var("PLATYPUS_PRICK").is_ok() && (4.0..4.02).contains(&t)
        && let Some((target, _, _)) = healths.iter().find(|(_, c, _)| c.kind == *kind)
    {
        hits.write(crate::combat::Hit { target, damage: 3.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at: fk.body.pos, dir: Vec2::X, weight: 0.2, crit: false });
    }
    // Backing off: walking away from it (not into the walls).
    let away = if pk.body.pos.x < fk.body.pos.x { KeyCode::KeyA } else { KeyCode::KeyD };
    let arena = std::env::var("PLATYPUS_WORLD").is_ok_and(|w| w == "arena");
    let room = !arena || if away == KeyCode::KeyA { pk.body.pos.x > 90.0 } else { pk.body.pos.x < 1830.0 };
    if *step == 1 && t < *retreat && room {
        keys.release(if away == KeyCode::KeyA { KeyCode::KeyD } else { KeyCode::KeyA });
        keys.press(away);
    } else {
        keys.release(KeyCode::KeyA);
        keys.release(KeyCode::KeyD);
    }
    if *step == 1 && t >= *next {
        *next += 0.5;
        let x = fk.body.pos.x;
        let f = if fk.loco.facing < 0.0 { -1.0 } else { 1.0 };
        let slope = match (ground(x + 16.0 * f, fk.body.pos.y), ground(x - 16.0 * f, fk.body.pos.y)) {
            (Some(a), Some(b)) => (a - b).atan2(32.0).to_degrees(),
            _ => 0.0,
        };
        match legs {
            Some(legs) => {
                let (tilt, ride, planted) = legs.pose(fk.body.pos.y - fk.body.half.y);
                let solid = |p: Vec2| sim.world.get(CellPos::from_world(p.x, p.y + 0.5)).is_some_and(|c| sim.world.materials().phys(c.material).kind == platypus_sim::Kind::Static);
                let sunk = legs.feet().filter(|(p, _)| solid(*p)).count();
                pairs.push((slope, tilt));
                if std::env::var("PLATYPUS_FEETLOG").is_ok() {
                    let b = Vec2::new(fk.body.pos.x, fk.body.pos.y - fk.body.half.y);
                    let fs: Vec<String> = legs.feet().map(|(p, g)| format!("{:?}{}", (p - b).round(), if g { "" } else { "~" })).collect();
                    info!("legs: feet from its box's bottom: {}; vel {:?} grounded {}", fs.join(" "), fk.body.vel.round(), fk.loco.grounded());
                }
                if std::env::var("PLATYPUS_FEETLOG").is_ok() {
                    info!("legs: t {t:.1}: the player at {:?}; it: vel {:?} grounded {} clinging {:?} wants x {:.1} y {:.1}", pk.body.pos.round(), fk.body.vel.round(), fk.loco.grounded(), fk.loco.clinging(), wants.map_or(0.0, |w| w.0), wants.map_or(0.0, |w| w.1));
                }
                // (Its turrets' heat, if it has any: venting marked.)
                let heat: Vec<String> = legs.turret_heat().iter().map(|(h, v)| format!("{h:.2}{}", if *v { " venting" } else { "" })).collect();
                let heat = if heat.is_empty() { String::new() } else { format!("; guns' heat {}", heat.join(", ")) };
                info!("legs: t {t:.1}: at {:?} slope {slope:+.0}° tilt {tilt:+.0}° rides {ride:.0} over its box's bottom, {planted} feet planted, {sunk} in rock{heat}", fk.body.pos.round());
            }
            None => info!("legs: t {t:.1}: the {kind} has no legs"),
        }
    }
    if let Ok(dir) = std::env::var("PLATYPUS_SHOTS")
        && t >= *shot
    {
        *shot += std::env::var("PLATYPUS_SHOT_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(2.0);
        let shot = match &offscreen {
            Some(o) => Screenshot::image(o.0.clone()),
            None => Screenshot::primary_window(),
        };
        commands.spawn(shot).observe(save_to_disk(format!("{dir}/legs_{:05.0}.png", t * 100.0)));
    }
    if *step == 1 && t > s.duration - 1.2 {
        *step = 2;
        // How well its tilt followed the ground: the same way, and how far
        // off on the slopes (|slope| over 5°).
        let slopes: Vec<&(f32, f32)> = pairs.iter().filter(|(s, _)| s.abs() > 5.0).collect();
        let same = slopes.iter().filter(|(s, t)| s.signum() == t.signum()).count();
        let off = slopes.iter().map(|(s, t)| (s - t).abs()).sum::<f32>() / slopes.len().max(1) as f32;
        info!("legs: over {} readings on slopes its tilt went the ground's way {} times, {off:.0}° off on average", slopes.len(), same);
        info!("legs: a foot at most {:.2} of its reach from its hip; past its reach {:.1} % of frames", strain.0, strain.1 as f32 * 100.0 / strain.2.max(1) as f32);
        if let Some((_, _, Some(l))) = foes.iter().find(|(c, _, _)| c.kind == *kind) {
            let (seen, piled) = l.piled();
            if seen > 0 {
                info!("legs: its body piled on itself {:.1} % of frames", piled as f32 * 100.0 / seen as f32);
            }
            let (n, at, shown) = l.snaps();
            info!("legs: feet snapping (faster than 9 reaches/s against the body): {:.2} % where the steps put them, {:.2} % as drawn", at as f32 * 100.0 / n.max(1) as f32, shown as f32 * 100.0 / n.max(1) as f32);
        }
    }
}

/// `dig` (arena): the player sealed in a shell of `PLATYPUS_WALL` (dirt by
/// default; stone, obsidian, glass) 24 thick round a pocket, a cave spider
/// put down 200 cells off; logs how near it is each second and when it's
/// in with the player (within 40), or that it isn't.
fn dig_script(mut commands: Commands, s: Res<Scenario>, mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, foes: Query<(&crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>, mut state: Local<(u8, f32, String)>) {
    if s.name != "dig" {
        return;
    }
    let t = s.elapsed;
    let Ok(pk) = player.single() else { return };
    let (step, next, wall) = &mut *state;
    if *step == 0 && t > 0.8 {
        *wall = std::env::var("PLATYPUS_WALL").unwrap_or_else(|_| "dirt".into());
        let Some(m) = sim.materials().id(wall) else { return error!("dig: no material `{wall}`") };
        let (px, floor) = (pk.body.pos.x as i32, (pk.body.pos.y - pk.body.half.y) as i32);
        for x in px - 64..px + 64 {
            for y in floor..floor + 84 {
                let inside = (px - 30..px + 30).contains(&x) && y < floor + 40;
                if !inside {
                    sim.world.set(CellPos::new(x, y), platypus_sim::Cell::new(m, ((x * 7 + y * 13) & 255) as u8));
                }
            }
        }
        crate::creatures::def::spawn_creature(&mut commands, "spider", Vec2::new(px as f32 - 200.0, floor as f32), |e| {
            if let Some(mut h) = e.get_mut::<crate::creatures::brain::hunter::Hunter>() {
                h.aggro = 1000.0;
            }
        });
        info!("dig: the player sealed in {wall}; a spider 200 off");
        *step = 1;
        *next = t + 2.0;
        return;
    }
    let Some((_, fk)) = foes.iter().find(|(c, _)| c.kind == "spider") else { return };
    let d = fk.body.pos.distance(pk.body.pos);
    if *step == 1 && d < 40.0 {
        info!("dig: the spider is in, through {wall}, at t {t:.1} (put down at 0.8)");
        *step = 2;
    }
    if *step == 1 && t >= *next {
        *next += 2.0;
        info!("dig: t {t:.0}: the spider at {:?}, {d:.0} from the player", fk.body.pos.round());
    }
    if *step == 1 && t > s.duration - 1.2 {
        info!("dig: the spider isn't in ({wall}); {d:.0} away at the end");
        *step = 3;
    }
}

/// `reset`: the world reset from the dev panel.
fn reset_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut acts: MessageWriter<crate::dev::DevAction>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    creatures: Query<&crate::creatures::Creature, Without<LocalPlayer>>,
    mut state: Local<(u8, Vec2)>,
) {
    if s.name != "reset" {
        return;
    }
    let t = s.elapsed;
    let pos = player.single().map(|k| k.body.pos).ok();
    let hole = |sim: &SimWorld, at: Vec2| (0..30).filter(|d| sim.world.get(CellPos::from_world(at.x, at.y - 15.0 - *d as f32)).is_some_and(|c| c.is_air())).count();
    match state.0 {
        0 if t > 1.0 => {
            let Some(p) = pos else { return };
            for d in 0..8 {
                sim.queue(WorldEdit::Dig { center: CellPos::from_world(p.x + 45.0, p.y - 15.0 - d as f32 * 6.0), radius: 12, max_hardness: 250 });
            }
            crate::creatures::def::spawn_creature(&mut commands, "orc", p + Vec2::new(-60.0, 0.0), |_| {});
            state.1 = Vec2::new(p.x + 45.0, p.y);
            state.0 = 1;
        }
        1 if t > 1.8 => {
            info!("reset: before: {} of 30 cells of the hole open, {} creatures about, the player at {:?}", hole(&sim, state.1), creatures.iter().count(), pos.map(|p| p.round()));
            state.0 = 2;
        }
        2 | 3 if t > 2.0 + (state.0 - 2) as f32 * 0.3 => {
            let all = std::env::var("PLATYPUS_RESET").is_ok_and(|v| v == "all");
            acts.write(if all { crate::dev::DevAction::ResetAll } else { crate::dev::DevAction::ResetWorld });
            state.0 += 1;
        }
        4 if t > 5.0 => {
            info!("reset: after: {} of 30 cells of the hole open, {} creatures about, the player at {:?}, {} chunks loaded", hole(&sim, state.1), creatures.iter().count(), pos.map(|p| p.round()), sim.world.loaded_count());
            state.0 = 5;
        }
        _ => {}
    }
}

/// `camplook`: to the nearest camp the world made.
fn camplook_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut toggles: ResMut<crate::light::LightToggles>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut state: Local<(u8, Vec2)>,
) {
    if s.name != "camplook" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    if state.0 == 1 {
        k.body.pos = state.1;
        k.body.vel = Vec2::ZERO;
        k.prev_pos = state.1;
    }
    if state.0 != 0 || s.elapsed < 0.3 {
        return;
    }
    state.0 = 2;
    let start = k.body.pos;
    let (cx0, cy0) = ((start.x as i32).div_euclid(platypus_sim::CHUNK), (start.y as i32).div_euclid(platypus_sim::CHUNK));
    let mut best: Option<(f32, CellPos)> = None;
    let mut props = 0;
    for dy in 3..60 {
        for dx in -36..=36 {
            let pos = platypus_sim::ChunkPos::new(cx0 + dx, cy0 - dy);
            for (at, what) in sim.generator.generate_with_spawns(pos).1 {
                props += matches!(what, platypus_worldgen::Spawn::Prop(_)) as usize;
                if what == platypus_worldgen::Spawn::Prop("mine_cart") {
                    let d = Vec2::new(at.x as f32, at.y as f32).distance(start);
                    if best.is_none_or(|(b, _)| d < b) {
                        best = Some((d, at));
                    }
                }
            }
        }
    }
    match best {
        Some((d, at)) => {
            info!("camplook: a camp's cart at ({}, {}), {d:.0} cells from the start", at.x, at.y);
            state.1 = Vec2::new(at.x as f32 - 21.0, at.y as f32 + k.body.half.y + 0.5);
            toggles.carry = crate::light::Carry::Torch;
            state.0 = 1;
        }
        None => info!("camplook: no camp near the start ({props} props in the chunks looked at)"),
    }
}

/// `camp`: a miners' camp set off from afar.
fn camp_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut player: Query<(&mut Kinematics, &crate::creatures::Health), With<LocalPlayer>>,
    mut props: Query<(Entity, &crate::creatures::Creature, &mut crate::creatures::Health), Without<LocalPlayer>>,
    mut state: Local<u8>,
) {
    if s.name != "camp" {
        return;
    }
    let Ok((mut k, hp)) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR as f32;
    let x = 780.0;
    let kinds = ["mine_lantern", "mine_cart", "tnt_barrel", "dynamite"];
    let left = |props: &Query<(Entity, &crate::creatures::Creature, &mut crate::creatures::Health), Without<LocalPlayer>>| kinds.iter().map(|&kind| format!("{kind} {}", props.iter().filter(|(_, c, _)| c.kind == kind).count())).collect::<Vec<_>>().join(", ");
    match *state {
        0 if t > 0.3 => {
            for (kind, dx) in kinds.iter().zip([195.0, 222.0, 248.0, 260.0]) {
                crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(x + dx, floor), |_| {});
            }
            k.body.pos = Vec2::new(x, floor + k.body.half.y);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            *state = 1;
        }
        1 if t > 1.5 => {
            info!("camp: set up: {}; the player {:.0} hp, 222 cells from the cart", left(&props), hp.hp);
            *state = 2;
        }
        2 if t > 2.0 => {
            for (e, c, mut h) in &mut props {
                if c.kind == "mine_cart" {
                    if std::env::var("PLATYPUS_CAMP").is_ok_and(|v| v == "hit") {
                        h.hp = 0.0;
                    } else {
                        commands.entity(e).insert(crate::creatures::body::elements::Burning::new(0.1));
                    }
                }
            }
            *state = 3;
        }
        3 if t > 5.0 => {
            info!("camp: 3 s after the spark: {}; the player {:.0} hp", left(&props), hp.hp);
            *state = 4;
        }
        4..=7 if t > 5.0 + (*state - 3) as f32 * 0.8 => {
            info!("camp: {:.1} s after the spark: the player {:.0} hp", t - 2.0, hp.hp);
            *state += 1;
        }
        _ => {}
    }
}

/// `pickarea`: area mode dug into a dirt wall, aimed past it.
#[allow(clippy::too_many_arguments)]
fn pickarea_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    hand: Res<crate::hands::Hand>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut state: Local<(u8, usize, usize, f32)>,
) {
    if s.name != "pickarea" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let x = 810;
    // The wall (x + 21 .. x + 39) and the far dirt (x + 45 .. x + 69).
    let (wall, far) = ((x + 21, x + 39), (x + 45, x + 69));
    let dirt = sim.materials().id("dirt");
    let count = |sim: &SimWorld, (x0, x1): (i32, i32)| (x0..x1).flat_map(|cx| (floor..floor + 45).map(move |cy| CellPos::new(cx, cy))).filter(|&p| sim.world.get(p).is_some_and(|c| Some(c.material) == dirt)).count();
    match state.0 {
        0 if t > 0.3 => {
            if let Some(d) = dirt {
                for (x0, x1) in [wall, far] {
                    for cx in x0..x1 {
                        for cy in floor..floor + 45 {
                            sim.queue(WorldEdit::Paint { center: CellPos::new(cx, cy), radius: 0, material: d, overwrite: true });
                        }
                    }
                }
            }
            k.body.pos = Vec2::new(x as f32, floor as f32 + k.body.half.y);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            state.0 = 1;
        }
        1 if t > 0.8 => {
            keys.press(KeyCode::KeyC);
            state.1 = count(&sim, wall);
            state.2 = count(&sim, far);
            state.0 = 2;
        }
        2 if t > 0.9 => {
            keys.release(KeyCode::KeyC);
            info!("pickarea: C pressed: area mode {}; the wall has {} dirt cells, the far dirt {}", hand.area, state.1, state.2);
            state.0 = 3;
        }
        3 if t > 2.6 => {
            let (w, f) = (count(&sim, wall), count(&sim, far));
            info!("pickarea: 1.2 s of swings aimed past the wall: {} cells dug from the wall, {} from the far dirt", state.1 - w, state.2 - f);
            // Then dirt 75 deep under a spot further on.
            if let Some(d) = dirt {
                for cx in x + 81..x + 129 {
                    for cy in floor - 75..floor {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(cx, cy), radius: 0, material: d, overwrite: true });
                    }
                }
            }
            state.0 = 4;
        }
        4 if t > 2.8 => {
            k.body.pos = Vec2::new((x + 105) as f32, floor as f32 + k.body.half.y);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            state.3 = floor as f32;
            state.0 = 5;
        }
        5 if t > 6.0 => {
            let feet = k.body.pos.y - k.body.half.y;
            info!("pickarea: 3 s digging straight down in dirt: dropped {:.0} cells (feet {:.0} below the surface), standing on something: {}", state.3 - feet, state.3 - feet, k.loco.grounded());
            state.0 = 6;
        }
        _ => {}
    }
    // Aimed past the wall, at the far dirt, level with the chest; then
    // straight down.
    cursor.0 = match state.0 {
        2..=3 => Some(Vec2::new((far.0 + 12) as f32, floor as f32 + 18.0)),
        5 => Some(k.body.pos - Vec2::new(0.0, 60.0)),
        _ => None,
    };
    let hold = (state.0 == 3 && (1.4..2.6).contains(&t)) || state.0 == 5;
    if hold { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
}

type Soaker<'a> = (Entity, &'a mut Kinematics, &'a crate::creatures::Health, Option<&'a crate::creatures::body::elements::Coated>, Option<&'a crate::creatures::body::elements::Burning>);

/// `soak`: a step in acid, washed off in water; alight in a puddle, then
/// in deep water.
fn soak_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<Soaker, With<LocalPlayer>>,
    mut state: Local<(u8, f32)>,
) {
    if s.name != "soak" {
        return;
    }
    let Ok((me, mut k, health, coated, burning)) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    // The acid puddle, the deep water, the water puddle: (x from, to, depth).
    let (acid, deep, puddle) = ((750, 795, 3), (840, 900, 36), (930, 975, 3));
    let put = |k: &mut Kinematics, x: i32, feet: i32| {
        k.body.pos = Vec2::new(x as f32, feet as f32 + k.body.half.y);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
    };
    let on = |c: Option<&crate::creatures::body::elements::Coated>| c.map_or(String::from("nothing"), |c| c.coats.iter().map(|(n, a)| format!("{n} {:.0}%", a * 100.0)).collect::<Vec<_>>().join(", "));
    match state.0 {
        0 if t > 0.3 => {
            for (x0, x1, depth) in [acid, deep, puddle] {
                for x in x0..=x1 {
                    for y in floor - depth..floor {
                        sim.queue(WorldEdit::Dig { center: CellPos::new(x, y), radius: 0, max_hardness: 250 });
                    }
                }
            }
            state.0 = 1;
        }
        1 if t > 0.5 => {
            for ((x0, x1, depth), name) in [(acid, "acid"), (deep, "water"), (puddle, "water")] {
                let Some(m) = sim.materials().id(name) else { continue };
                for x in x0..=x1 {
                    for y in floor - depth..floor {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: m, overwrite: true });
                    }
                }
            }
            put(&mut k, 772, floor - acid.2);
            state.0 = 2;
        }
        2 if t > 1.6 => {
            info!("soak: a second standing in the acid puddle: {}, health {:.0}", on(coated), health.hp);
            put(&mut k, 818, floor);
            state.1 = health.hp;
            state.0 = 3;
        }
        3 if t > 3.0 => {
            info!("soak: 1.4 s out of it: {}, health {:.0} (lost {:.1} since)", on(coated), health.hp, state.1 - health.hp);
            put(&mut k, 952, floor - puddle.2);
            commands.entity(me).insert(crate::creatures::body::elements::Burning::new(1.0));
            state.0 = 4;
        }
        4 if t > 4.4 => {
            info!("soak: alight, 1.4 s standing in a three-cell water puddle: {}, burning {:.0}%", on(coated), burning.map_or(0.0, |b| b.share * 100.0));
            put(&mut k, 870, floor - 24);
            state.1 = health.hp;
            state.0 = 5;
        }
        5 if t > 5.2 => {
            info!("soak: 0.8 s in deep water: {}, burning {:.0}%, health {:.0}", on(coated), burning.map_or(0.0, |b| b.share * 100.0), health.hp);
            state.0 = 6;
        }
        6 if t > 6.5 => {
            info!("soak: 1.3 s more in it: health {:.0} (lost {:.1} in the water)", health.hp, state.1 - health.hp);
            state.0 = 7;
        }
        _ => {}
    }
}

fn rocketswim_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(&mut Kinematics, &crate::creatures::MoveStats), With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut fired: MessageReader<crate::creatures::Rocketed>,
    mut state: Local<(u8, f32, f32, u32)>,
) {
    if s.name != "rocketswim" {
        return;
    }
    let Ok((mut k, stats)) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let x = 810;
    state.3 += fired.read().count() as u32;
    match state.0 {
        // A pit 60 wide and 260 deep under the floor, then water in it.
        0 if t > 0.3 => {
            for dx in (-30..=30).step_by(7) {
                for dy in (-258..=6).step_by(7) {
                    sim.queue(WorldEdit::Dig { center: CellPos::new(x + dx, floor + dy), radius: 6, max_hardness: 250 });
                }
            }
            state.0 = 1;
        }
        1 if t > 0.6 => {
            if let Some(water) = sim.materials().id("water") {
                for dx in (-30..=30).step_by(4) {
                    for dy in (-258..=-18).step_by(4) {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x + dx, floor + dy), radius: 3, material: water, overwrite: false });
                    }
                }
            }
            // (Mid-water, far over the bottom: no ground to refill from.)
            k.body.pos = Vec2::new(x as f32, (floor - 90) as f32);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            k.loco.rocket_left = 0.0;
            state.0 = 2;
        }
        2 if t > 3.8 => {
            info!("rocketswim: after 3 s in the water the boots hold {:.2} of {:.2} s", k.loco.rocket_left, stats.0.rocket_time);
            state.1 = k.body.pos.y;
            state.3 = 0;
            state.0 = 3;
        }
        3 => {
            state.2 = state.2.max(k.body.pos.y);
            if t > 5.3 {
                let wet = sim.world.get(CellPos::from_world(k.body.pos.x, k.body.pos.y)).is_some_and(|c| sim.materials().phys(c.material).kind == platypus_sim::Kind::Liquid);
                info!("rocketswim: jump held 1.5 s under water: rose {:.0} cells, fired {} ticks, {:.2} s of charge left, still in water {wet}", state.2 - state.1, state.3, k.loco.rocket_left);
                state.0 = 4;
            }
        }
        _ => {}
    }
    let hold = state.0 == 3;
    match (hold, keys.pressed(KeyCode::Space)) {
        (true, false) => keys.press(KeyCode::Space),
        (false, true) => keys.release(KeyCode::Space),
        _ => {}
    }
}

/// A spider's attacks against a player standing still.
#[allow(clippy::too_many_arguments)]
fn spider_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut player: Query<(Entity, &mut Kinematics, &mut crate::creatures::Health, Option<&crate::creatures::body::elements::Coated>), With<LocalPlayer>>,
    spiders: Query<(&Kinematics, &crate::creatures::moves::Moves), Without<LocalPlayer>>,
    book: Res<crate::creatures::moves::MoveBook>,
    mut hits: MessageReader<crate::combat::Hit>,
    mut state: Local<(u8, f32, Option<String>)>,
) {
    if s.name != "spider" {
        return;
    }
    let Ok((me, mut k, mut h, coat)) = player.single_mut() else { return };
    let t = s.elapsed;
    // On open floor, the spider 180 cells off.
    if state.0 == 0 && t > 0.5 {
        let floor = platypus_worldgen::arena::FLOOR as f32;
        k.body.pos = Vec2::new(840.0, floor + 12.0);
        k.prev_pos = k.body.pos;
        crate::creatures::def::spawn_creature(&mut commands, "spider", Vec2::new(1020.0, floor), |_| {});
        state.0 = 1;
    }
    for hit in hits.read() {
        if hit.target == me {
            info!("spider: t {t:.2} the player took {:.0} (knocked {:.0} cells/s)", hit.damage, hit.knock.length());
        }
    }
    for (sk, a) in &spiders {
        let now = a.doing(&book).map(|d| d.0.to_string());
        if now != state.2 {
            if let Some(what) = &now {
                info!("spider: t {t:.2} {what} from {:.0} cells", sk.body.pos.distance(k.body.pos));
            }
            state.2 = now;
        }
    }
    if t > state.1 + 1.0 {
        state.1 = t;
        for (sk, a) in &spiders {
            info!("spider: t {t:.1} at {:.0} cells ({:.0},{:.0} to the player's {:.0},{:.0}), vel ({:.0},{:.0}), grounded {} clinging {:?}, doing {:?}", sk.body.pos.distance(k.body.pos), sk.body.pos.x, sk.body.pos.y, k.body.pos.x, k.body.pos.y, sk.body.vel.x, sk.body.vel.y, sk.loco.grounded(), sk.loco.clinging(), a.doing(&book));
        }
        if coat.is_some_and(|c| c.share("venom") > 0.0) {
            info!("spider: t {t:.1} the player is envenomed");
        }
        if h.hp < h.max {
            info!("spider: t {t:.1} the player lost {:.0} hp this second", h.max - h.hp);
        }
        h.hp = h.max;
    }
}

/// The grappling hook: hooked on a wall above an overhang, from below and
/// out (pulled up round the overhang's corner, not stuck on it); hooked on the wall's top edge and mantled up
/// onto it; hooked on a beam overhead (pulled up, hanging), rope let out
/// (rappelling down), a swing pumped and let go of mid-swing; a kick off
/// the wall on the rope; a chest pulled in. Logs each.
/// The walk so far: start x, ticks seen, at full speed, dead stops,
/// airborne, last vx, (unused), start y, done.
type Walked = (f32, u32, u32, u32, u32, f32, bool, f32, bool);

/// A plain walk right across the generated surface (D held, nothing
/// else) for 8 s: how often it runs at full speed, the dead stops (a
/// bump too tall to step: speed to nothing), the steps up, the hops off
/// bumps (off the ground without jumping), how far it got.
fn walk_script(
    s: Res<Scenario>,
    tempo: Res<crate::tempo::Tempo>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<Walked>,
) {
    if s.name != "walk" || s.elapsed < 1.0 || state.8 {
        return;
    }
    let Ok(k) = player.single() else { return };
    if state.1 == 0 {
        state.0 = k.body.pos.x;
        state.7 = k.body.pos.y;
        keys.press(KeyCode::KeyD);
    }
    state.1 += 1;
    let (vx, top) = (k.body.vel.x, 0.95 * tempo.apply(&MovementStats::default(), true).run_speed.min(142.0));
    if vx >= top.min(90.0) {
        state.2 += 1;
    }
    if state.5 > 45.0 && vx < 7.5 {
        state.3 += 1;
    }
    if !k.loco.grounded() {
        state.4 += 1;
    }
    state.5 = vx;
    if s.elapsed > 9.0 {
        keys.release(KeyCode::KeyD);
        let n = state.1.max(1) as f32;
        info!(
            "walk: {}: {:.0} cells in 8 s ({:.0} cells/s), at full speed {:.0} % of the time, {} dead stops, off the ground {:.0} % (no jumps), {:.0} cells up/down",
            tempo.name(),
            k.body.pos.x - state.0,
            (k.body.pos.x - state.0) / 8.0,
            100.0 * state.2 as f32 / n,
            state.3,
            100.0 * state.4 as f32 / n,
            k.body.pos.y - state.7
        );
        state.8 = true;
    }
}

/// Each tempo preset in turn (`tempo.ron`), the same moves, measured: a
/// run from a standstill (to 95 % of top speed, and the top), letting go
/// (how far it slides), a turn (from full speed one way to 90 % the
/// other), a full jump held (its height in cells and body heights, its
/// time in the air), a tap (the hop), and a drop from 150 cells (the
/// fastest fall, the time down).
fn tempo_script(
    s: Res<Scenario>,
    mut tempo: ResMut<crate::tempo::Tempo>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    // (preset, step, step start, readings)
    mut state: Local<(usize, u8, f32, Vec<f32>)>,
) {
    if s.name != "tempo" || s.elapsed < 0.5 {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    // (The dev start's rocket boots would fire on a held jump.)
    k.loco.rocket_left = 0.0;
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let feet = k.body.pos.y - k.body.half.y - fl;
    let st = &mut *state;
    let dt = t - st.2;
    let v = k.body.vel;
    let mut want: Vec<KeyCode> = Vec::new();
    let put = |k: &mut Kinematics, x: f32, y: f32| {
        k.body.pos = Vec2::new(x, fl + y + k.body.half.y);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
    };
    let Some(preset) = tempo.presets.get(st.0).map(|p| p.name.clone()) else { return };
    let (which, step) = (st.0, st.1);
    let r = &mut st.3;
    if r.len() < 12 {
        r.resize(12, 0.0);
    }
    // readings: 0 time to 95 %, 1 top, 2 slide, 3 turn, 4 apex, 5 air, 6 hop, 7 fall max, 8 fall time, 9 x at let-go, 10 rising?, 11 left ground at
    let next;
    match step {
        0 => {
            tempo.active = which;
            put(&mut k, 675.0, 1.5);
            *r = vec![0.0; 12];
            next = dt > 0.4;
        }
        // Run from a standstill.
        1 => {
            want.push(KeyCode::KeyD);
            r[1] = r[1].max(v.x);
            next = dt > 1.2;
        }
        // (The time to 95 % of the top, now the top is known: run again.)
        2 => {
            if dt < 0.05 {
                put(&mut k, 675.0, 1.5);
            } else {
                want.push(KeyCode::KeyD);
                if r[0] == 0.0 && v.x >= 0.95 * r[1] {
                    r[0] = dt - 0.05;
                }
            }
            next = dt > 1.2;
            if next {
                r[9] = k.body.pos.x;
            }
        }
        // Let go: the slide.
        3 => {
            r[2] = k.body.pos.x - r[9];
            next = dt > 0.8;
        }
        // Full speed, then turn.
        4 => {
            if dt < 1.0 {
                want.push(KeyCode::KeyD);
            } else {
                want.push(KeyCode::KeyA);
                if r[3] == 0.0 && v.x <= -0.9 * r[1] {
                    r[3] = dt - 1.0;
                }
            }
            next = dt > 2.4;
        }
        // A full jump, held.
        5 => {
            if dt < 0.05 {
                put(&mut k, 780.0, 1.5);
            } else if dt < 0.3 {
                // (Settle first.)
            } else {
                want.push(KeyCode::Space);
                r[4] = r[4].max(feet);
                if feet > 0.75 && r[11] == 0.0 {
                    r[11] = dt;
                }
                if r[11] > 0.0 && feet <= 0.75 && r[5] == 0.0 && dt - r[11] > 0.1 {
                    r[5] = dt - r[11];
                }
            }
            next = dt > 2.0;
        }
        // A tap.
        6 => {
            if dt < 0.05 {
                put(&mut k, 780.0, 1.5);
            }
            if (0.3..0.34).contains(&dt) {
                want.push(KeyCode::Space);
            }
            if dt > 0.1 {
                r[6] = r[6].max(feet);
            }
            next = dt > 1.5;
        }
        // A drop from 150.
        7 => {
            if dt < 0.03 {
                put(&mut k, 990.0, 150.0);
                r[11] = 0.0;
            } else {
                r[7] = r[7].max(-v.y);
                if feet > 75.0 {
                    r[11] = 1.0;
                }
                if r[11] > 0.0 && feet <= 0.75 && r[8] == 0.0 {
                    r[8] = dt - 0.03;
                }
            }
            next = dt > 2.0;
        }
        _ => {
            let h = 23.0;
            info!(
                "tempo: {preset}: to full run {:.2}s, top {:.0} cells/s ({:.1} heights/s), slide {:.1} cells, turn {:.2}s, jump {:.0} cells ({:.1} heights) {:.2}s in the air, tap {:.0} cells, fall at most {:.0} cells/s ({:.0} heights/s), 150 cells down in {:.2}s",
                r[0], r[1], r[1] / h, r[2], r[3], r[4], r[4] / h, r[5], r[6], r[7], r[7] / h, r[8]
            );
            st.0 += 1;
            st.1 = 0;
            st.2 = t;
            return;
        }
    }
    for key in [KeyCode::KeyA, KeyCode::KeyD, KeyCode::Space] {
        match (want.contains(&key), keys.pressed(key)) {
            (true, false) => keys.press(key),
            (false, true) => keys.release(key),
            _ => {}
        }
    }
    if next {
        st.1 += 1;
        st.2 = t;
    }
}

/// The underground's look: a big cavern dug 390 cells down, its back walls
/// taken away over its left two thirds (the cave's far backdrop shows
/// there, where light reaches), the player on its floor with two glow
/// sticks thrown out.
fn underlook_script(mut commands: Commands, s: Res<Scenario>, mut sim: ResMut<SimWorld>, lights: Res<crate::light::LightSettings>, mut player: Query<&mut Kinematics, With<LocalPlayer>>, mut state: Local<(u8, Vec2)>) {
    if s.name != "underlook" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let t = s.elapsed;
    match state.0 {
        0 if t > 0.5 => {
            let c = k.body.pos - Vec2::new(0.0, 390.0);
            for dx in (-180..=180).step_by(30) {
                let at = CellPos::new(c.x as i32 + dx, c.y as i32 + (18.0 * (dx as f32 * 0.02).sin()) as i32);
                sim.queue(WorldEdit::Dig { center: at, radius: 63, max_hardness: 250 });
            }
            *state = (1, c);
        }
        1 if t > 0.8 => {
            k.body.pos = state.1 + Vec2::new(-30.0, -15.0);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            state.0 = 2;
        }
        // (Once the chunks down there are loaded: dug again, walls off.)
        2 if t > 1.6 => {
            let c = state.1;
            for dx in (-180..=180).step_by(30) {
                let at = CellPos::new(c.x as i32 + dx, c.y as i32 + (18.0 * (dx as f32 * 0.02).sin()) as i32);
                sim.queue(WorldEdit::Dig { center: at, radius: 63, max_hardness: 250 });
            }
            state.0 = 3;
        }
        3 if t > 1.8 => {
            let c = state.1;
            for dx in (-180..=60).step_by(24) {
                let at = CellPos::new(c.x as i32 + dx, c.y as i32 + (18.0 * (dx as f32 * 0.02).sin()) as i32);
                for _ in 0..3 {
                    sim.queue(WorldEdit::Mine { center: at, radius: 60, power: 255, max_hardness: 250, back: true });
                }
            }
            state.0 = 4;
        }
        4 if t > 2.4 => {
            let s = lights.glowstick.strength;
            crate::props::spawn_glowstick(&mut commands, k.body.pos + Vec2::new(-15.0, 6.0), Vec2::new(-135.0, 75.0), [0.25 * s, s, 0.45 * s], 90.0, lights.glowstick.haze);
            crate::props::spawn_glowstick(&mut commands, k.body.pos + Vec2::new(15.0, 6.0), Vec2::new(165.0, 90.0), [0.2 * s, 0.55 * s, 1.1 * s], 90.0, lights.glowstick.haze);
            state.0 = 5;
        }
        5 if t > 3.0 => {
            let c = state.1;
            let (mut open, mut walled) = (0, 0);
            for y in (c.y as i32 - 45)..(c.y as i32 + 45) {
                for x in (c.x as i32 - 165)..(c.x as i32 + 165) {
                    let p = CellPos::new(x, y);
                    if sim.world.get(p).is_some_and(|c| c.is_air()) {
                        if sim.world.get_bg(p).is_some_and(|c| c.is_air()) {
                            open += 1;
                        } else {
                            walled += 1;
                        }
                    }
                }
            }
            info!("underlook: the cavern: {open} cells open to nothing behind, {walled} with a back wall");
            state.0 = 6;
        }
        _ => {}
    }
}

/// The underground's void in a natural cave (see the module docs).
#[allow(clippy::too_many_arguments)]
fn voidlook_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut toggles: ResMut<crate::light::LightToggles>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut state: Local<(u8, Vec2)>,
) {
    if s.name != "voidlook" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let t = s.elapsed;
    // (Held where it's put until it's stood on a cave floor.)
    if (1..=2).contains(&state.0) {
        k.body.pos = state.1;
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
    }
    match state.0 {
        0 if t > 0.3 => {
            let x0 = k.body.pos.x as i32;
            let ground = |x: i32| sim.generator.surface_hint(x).unwrap_or(k.body.pos.y as i32);
            let target = match std::env::var("PLATYPUS_ZONE") {
                Ok(zone) => (0..300)
                    .flat_map(|i| [x0 + i * 60, x0 - i * 60])
                    .find_map(|x| (3..120).map(|j| ground(x) - j * 60).find(|&y| sim.generator.zone_at(x, y) == Some(zone.as_str())).map(|y| (x, y))),
                Err(_) => {
                    let depth: i32 = std::env::var("PLATYPUS_DEPTH").ok().and_then(|v| v.parse().ok()).unwrap_or(600);
                    Some((x0, ground(x0) - depth))
                }
            };
            let Some((x, y)) = target else {
                info!("voidlook: no such zone near");
                state.0 = 9;
                return;
            };
            *state = (1, Vec2::new(x as f32, y as f32));
        }
        // Chunks loaded: the nearest cave floor (open 24 up, 8 either side,
        // rock under it).
        1 if t > 2.0 => {
            let c = CellPos::from_world(state.1.x, state.1.y);
            let air = |x: i32, y: i32| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.is_air());
            let solid = |x: i32, y: i32| sim.world.is_solid(CellPos::new(x, y));
            let floor = (0..390).flat_map(|r| {
                let ring: Vec<(i32, i32)> = (-r..=r).flat_map(|d| [(c.x + d, c.y + r), (c.x + d, c.y - r), (c.x + r, c.y + d), (c.x - r, c.y + d)]).collect();
                ring
            }).find(|&(x, y)| {
                // (In the zone asked for, if one was.)
                let zone = std::env::var("PLATYPUS_ZONE").ok();
                zone.is_none_or(|z| sim.generator.zone_at(x, y) == Some(z.as_str()))
                    && solid(x, y - 1) && solid(x - 4, y - 1) && solid(x + 4, y - 1) && (0..24).all(|dy| (-8..=8).all(|dx| air(x + dx, y + dy)))
            });
            match floor {
                Some((x, y)) => {
                    state.1 = Vec2::new(x as f32, y as f32 + k.body.half.y + 0.5);
                    let (zone, band) = (sim.generator.zone_at(x, y), sim.generator.band_hint(y));
                    info!("voidlook: a cave floor at ({x}, {y}), {} down; band {band:?}, zone {zone:?}", sim.generator.surface_hint(x).unwrap_or(y) - y);
                }
                None => info!("voidlook: no cave floor near ({}, {})", c.x, c.y),
            }
            // (`PLATYPUS_NOTORCH=1`: nothing in hand, the lantern alone.)
            toggles.carry = if std::env::var("PLATYPUS_NOTORCH").is_ok() { crate::light::Carry::Nothing } else { crate::light::Carry::Torch };
            // (`PLATYPUS_SPAWN=spider,bat`: those out in the dark, 60 cells
            // on either side in turn, a little above the floor.)
            if let Ok(kinds) = std::env::var("PLATYPUS_SPAWN") {
                for (i, kind) in kinds.split(',').filter(|k| !k.is_empty()).enumerate() {
                    let side = if i % 2 == 0 { 1.0 } else { -1.0 };
                    let at = state.1 + Vec2::new(side * (60.0 + 21.0 * (i / 2) as f32), 3.0 + 12.0 * (i / 2) as f32 - k.body.half.y);
                    crate::creatures::def::spawn_creature(&mut commands, kind, at, |_| {});
                }
            }
            state.0 = 2;
        }
        // Stood there; a patch of back wall beside it axed away, over a few
        // frames (each pass wears it down), as a player would.
        2 if t > 2.2 => state.0 = 3,
        3..=6 if t > 2.2 + (state.0 - 3) as f32 * 0.1 => {
            let c = state.1 + Vec2::new(39.0, 21.0);
            for dy in (-24..=24).step_by(12) {
                for dx in (-51..=51).step_by(12) {
                    let at = CellPos::new(c.x as i32 + dx, c.y as i32 + dy);
                    sim.queue(WorldEdit::Mine { center: at, radius: 10, power: 255, max_hardness: 250, back: true });
                }
            }
            state.0 += 1;
        }
        7 if t > 3.5 => {
            let c = state.1;
            let (mut void, mut walled) = (0, 0);
            for y in c.y as i32 - 60..c.y as i32 + 90 {
                for x in c.x as i32 - 135..c.x as i32 + 135 {
                    let p = CellPos::new(x, y);
                    if sim.world.get(p).is_some_and(|c| c.is_air()) {
                        if sim.world.get_bg(p).is_some_and(|c| c.is_air()) { void += 1 } else { walled += 1 }
                    }
                }
            }
            info!("voidlook: around the player {void} open cells show the void, {walled} a back wall");
            state.0 = 8;
        }
        _ => {}
    }
}

/// A look at the backdrops: the player put on the surface in the middle of
/// a biome (`PLATYPUS_BIOME`, the nearest wide stretch of it; default where
/// it starts), the hour as `PLATYPUS_HOUR` has it; then it walks a little.
fn backdrop_script(s: Res<Scenario>, sim: Res<SimWorld>, mut player: Query<&mut Kinematics, With<LocalPlayer>>, mut keys: ResMut<ButtonInput<KeyCode>>, mut done: Local<bool>) {
    if s.name != "backdrop" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    if !*done && s.elapsed > 0.5 {
        *done = true;
        let x0 = k.body.pos.x as i32;
        if let Ok(want) = std::env::var("PLATYPUS_BIOME") {
            // The nearest column with 900 cells of it either side.
            let found = (0..200).flat_map(|i| [x0 + i * 300, x0 - i * 300]).find(|&x| (-3..=3).all(|d| sim.generator.biome_hint(x + d * 300) == Some(want.as_str())));
            match found {
                Some(x) => {
                    let y = sim.generator.surface_hint(x).unwrap_or(0) as f32 + k.body.half.y + 3.0;
                    k.body.pos = Vec2::new(x as f32, y);
                    k.body.vel = Vec2::ZERO;
                    k.prev_pos = k.body.pos;
                    info!("backdrop: {want} at x {x}");
                }
                None => info!("backdrop: no {want} found"),
            }
        }
    }
    // (A walk right from 4 s: the layers slide at their own speeds.)
    if s.elapsed > 4.0 && !keys.pressed(KeyCode::KeyD) {
        keys.press(KeyCode::KeyD);
    }
}

/// Down into the rock and back (F2): a pocket dug 300 cells under the
/// player, the player put in it; then F2 up to the surface. Logs where
/// and what the sound heard (underground or not) at each.
fn surface_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    heard: Res<crate::sound::ambience::Heard>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<(u8, Vec2)>,
) {
    if s.name != "surface" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let t = s.elapsed;
    let surface = sim.generator.surface_hint(k.body.pos.x as i32).unwrap_or(0) as f32;
    match state.0 {
        0 if t > 1.5 => {
            let at = k.body.pos - Vec2::new(0.0, 300.0);
            sim.queue(WorldEdit::Dig { center: CellPos::new(at.x as i32, at.y as i32), radius: 21, max_hardness: 255 });
            *state = (1, at);
        }
        1 if t > 1.7 => {
            k.body.pos = state.1;
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            state.0 = 2;
        }
        2 if t > 6.0 => {
            info!("surface: in a pocket {:.0} cells below the ground: underground {:.2}", surface - (k.body.pos.y - k.body.half.y), heard.under);
            keys.press(KeyCode::F2);
            state.0 = 3;
        }
        3 => {
            keys.release(KeyCode::F2);
            state.0 = 4;
        }
        4 if t > 11.0 => {
            info!("surface: after F2, feet {:.0} cells from the ground as generated: underground {:.2}", (k.body.pos.y - k.body.half.y) - surface, heard.under);
            state.0 = 5;
        }
        _ => {}
    }
}

/// The sound board, played: every one-shot in sounds.ron in turn, 0.8 s
/// apart, each named in the log, near the player; then the beds and the
/// music go on under whatever's around.
fn sounds_script(s: Res<Scenario>, bank: Res<crate::sound::SoundBank>, player: Query<&Kinematics, With<LocalPlayer>>, mut next: Local<usize>, mut out: MessageWriter<crate::sound::PlaySound>) {
    if s.name != "sounds" || bank.made.is_empty() {
        return;
    }
    let names: Vec<&String> = bank.defs.iter().filter(|(_, d)| d.loops <= 0.0).map(|(n, _)| n).collect();
    let due = ((s.elapsed - 3.0) / 0.8).floor();
    if due < 0.0 || *next >= names.len() || (*next as f32) > due {
        return;
    }
    let at = player.single().map_or(Vec2::ZERO, |k| k.body.pos);
    let name = names[*next].clone();
    info!("sounds: {} ({} of {})", name, *next + 1, names.len());
    out.write(crate::sound::PlaySound::at(name, at + Vec2::new(30.0, 0.0)));
    *next += 1;
}

/// What lives where (BE `behaviour` stage 1): the player taken to each
/// surface biome in turn (the first stretch of it well away from the
/// start), kept alive, `PLATYPUS_SAFARI_SECS` seconds there (12; with
/// `PLATYPUS_LIFE_RATE=20` to hurry life along), then what came round them
/// (within 700 cells) is logged, kind by kind; then the next.
#[allow(clippy::type_complexity)]
fn safari_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(&crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    // (which biome, when it arrived there, where it is)
    mut state: Local<(usize, f32, Option<Vec2>)>,
) {
    if s.name != "safari" {
        return;
    }
    const BIOMES: [&str; 7] = ["plains", "forest", "desert", "tundra", "jungle", "deep forest", "mountains"];
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    h.hp = h.max;
    let dwell = std::env::var("PLATYPUS_SAFARI_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(12.0);
    if state.0 >= BIOMES.len() || t < 0.5 {
        return;
    }
    let biome = BIOMES[state.0];
    match state.2 {
        None => {
            // The first stretch of it 3500 cells or more from the start,
            // either way.
            let start = sim.generator.spawn_point().x;
            let found = (0..2000).flat_map(|i| [start + 3500 + i * 40, start - 3500 - i * 40]).find(|&x| sim.generator.biome_hint(x) == Some(biome) && sim.generator.biome_hint(x + 300) == Some(biome));
            // (Its middle: what spawns just off screen either side is in it
            // too.)
            let middle = found.map(|x| {
                let (mut lo, mut hi) = (x, x);
                while lo > x - 20000 && sim.generator.biome_hint(lo - 10) == Some(biome) {
                    lo -= 10;
                }
                while hi < x + 20000 && sim.generator.biome_hint(hi + 10) == Some(biome) {
                    hi += 10;
                }
                ((lo + hi) / 2, hi - lo)
            });
            match middle.and_then(|(x, w)| sim.generator.surface_hint(x).map(|y| (Vec2::new(x as f32, y as f32 + 14.0), w))) {
                Some((at, wide)) => {
                    k.body.pos = at;
                    k.prev_pos = at;
                    k.body.vel = Vec2::ZERO;
                    *state = (state.0, t, Some(at));
                    info!("safari: t {t:.1}: to the {biome}, at {:?} (the middle of {wide} cells of it)", at.round());
                }
                None => {
                    warn!("safari: no {biome} found");
                    state.0 += 1;
                }
            }
        }
        Some(at) => {
            // (Kept there, not wandering off.)
            if k.body.pos.distance(at) > 60.0 {
                k.body.pos = at;
                k.prev_pos = at;
                k.body.vel = Vec2::ZERO;
            }
            if t - state.1 >= dwell {
                let mut kinds: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
                for (c, ck) in &them {
                    if ck.body.pos.distance(at) < 700.0 {
                        *kinds.entry(c.kind.clone()).or_default() += 1;
                    }
                }
                let list: Vec<String> = kinds.iter().map(|(k, n)| format!("{k} ×{n}")).collect();
                info!("safari: the {biome} after {dwell:.0} s: {}", if list.is_empty() { "nothing".to_string() } else { list.join(", ") });
                *state = (state.0 + 1, t, None);
            }
        }
    }
}

/// Hit areas per part (BE `limbs` stage 4): a cave centipede and an iron
/// strider put down in the flat arena with no brain (held still), then
/// arrows shot at the centipede's first and last segments (well clear of
/// its head's box) and at the strider's viewport slit (its weak spot), its
/// plain hull and a leg. Logs what each arrow did: a part's hit lands,
/// a leg takes less, the weak spot more.
#[allow(clippy::too_many_arguments)]
fn parts_script(
    mut commands: Commands,
    s: Res<Scenario>,
    weapons: Option<Res<crate::combat::Weapons>>,
    player: Query<Entity, With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::body::parts::Parts>), Without<LocalPlayer>>,
    mut felt: MessageReader<crate::combat::Felt>,
    mut state: Local<(u8, Vec<(Vec2, String)>)>,
) {
    if s.name != "parts" {
        return;
    }
    use crate::creatures::body::parts::PartKind;
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    if state.0 == 0 && t > 0.5 {
        for (kind, x) in [("centipede", 900.0), ("strider", 1250.0)] {
            crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(x, fl + 30.0), |e| {
                e.insert(crate::creatures::brain::Staged);
            });
        }
        state.0 = 1;
        return;
    }
    let (Some(weapons), Ok(me)) = (weapons, player.single()) else { return };
    if state.0 == 1 && t > 2.5 {
        // Where to shoot: each a point on a part, the arrow from up and to
        // the left of it, a short way off.
        let mut aims: Vec<(Vec2, String, Entity)> = Vec::new();
        for (e, c, k, parts) in &them {
            let Some(parts) = parts else { continue };
            let mid = |kind: PartKind, last: bool| {
                let mut it = parts.list.iter().filter(|p| p.kind == kind);
                let p = if last { it.next_back() } else { it.next() };
                p.map(|p| (p.a + p.b) * 0.5)
            };
            match c.kind.as_str() {
                "centipede" => {
                    if let Some(p) = mid(PartKind::Segment, true) {
                        aims.push((p, "its last segment".into(), e));
                    }
                    if let Some(p) = mid(PartKind::Segment, false) {
                        aims.push((p, "its first segment".into(), e));
                    }
                }
                "strider" => {
                    if let Some(p) = mid(PartKind::Weak, false) {
                        aims.push((p, "its viewport (weak spot)".into(), e));
                    }
                    aims.push((k.body.pos + Vec2::new(-14.0, 12.0), "its hull".into(), e));
                    if let Some(p) = parts.list.iter().filter(|p| p.kind == PartKind::Leg).nth(1).map(|p| p.a.lerp(p.b, 0.5)) {
                        aims.push((p, "a leg".into(), e));
                    }
                }
                _ => {}
            }
        }
        for (i, (at, what, e)) in aims.into_iter().enumerate() {
            let from = at + Vec2::new(-14.0, 10.0);
            let vel = (at - from).normalize() * 420.0;
            crate::archery::spawn_arrow(&mut commands, &weapons, from, vel, me, None, (12.0, false), 60.0, 0.0);
            info!("parts: arrow {i} at {what}, {:?}", at.round());
            let _ = e;
            state.1.push((at, what));
        }
        state.0 = 2;
    }
    // (Each blow told apart by where it struck: the aim nearest it.)
    for f in felt.read() {
        let Some((at, what)) = state.1.iter().min_by(|a, b| a.0.distance(f.at).total_cmp(&b.0.distance(f.at))) else { continue };
        let kind = them.get(f.target).map_or("?".to_string(), |(_, c, ..)| c.kind.clone());
        info!("parts: t {t:.2}: the {kind}, struck at {:?} ({what}, {:.0} from the aim), meant {:.1}, took {:.1}", f.at.round(), at.distance(f.at), f.meant, f.dealt);
    }
}

/// `PLATYPUS_SHOTS=dir` (any scenario but `legs`, which has its own): a
/// picture every `PLATYPUS_SHOT_EVERY` s (2) into it, `<scenario>_<t in
/// hundredths>.png`.
fn shots_script(mut commands: Commands, s: Res<Scenario>, offscreen: Option<Res<crate::camera::Offscreen>>, mut next: Local<f32>) {
    let Ok(dir) = std::env::var("PLATYPUS_SHOTS") else { return };
    if s.name == "legs" || s.elapsed < *next {
        return;
    }
    *next += std::env::var("PLATYPUS_SHOT_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(2.0);
    let image = match &offscreen {
        Some(o) => Screenshot::image(o.0.clone()),
        None => Screenshot::primary_window(),
    };
    commands.spawn(image).observe(save_to_disk(format!("{dir}/{}_{:05.0}.png", s.name, s.elapsed * 100.0)));
}

/// The sneak scenario's line of text at the top of the screen.
#[derive(Component)]
struct SneakHud;

/// `sneak` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`; played, not
/// watched: it runs an hour): get through a dark hall at night, roofed
/// over from x 300 to 1680, to the open air past its far end, unseen.
/// - The camp: two orcs by a fire (a torch) under a stone shelf, facing
///   the way you come; a ramp up onto the shelf, its far end a jump over a
///   gap behind them.
/// - The patrol: a walkway on, an orc wandering the floor under it, a gap
///   over its beat to jump (the jump is loud, a run's feet).
/// - The sentry: an orc in its torch's light at the door, facing in; a
///   step down off the walkway's end. A bomb (4) thrown back down the
///   hall makes it go and look (the whole hall hears a blast).
///
/// Ctrl creeps (silent), L a light (two flashlights, then a torch: you
/// see, and are seen). The
/// line at the top counts the times something started hunting you.
/// `PLATYPUS_SNEAK_BOT=creep|run|torch|lamp` walks the player in from the
/// door along the walkway to x 1320, back from its end (Ctrl held but for
/// the jumps' run-ups, or not, or with a torch or a flashlight), then sets off a
/// bomb back down the hall and, the sentry gone past under it to look,
/// goes down and out; it logs every
/// change of every guard's wariness.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn sneak_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut day: ResMut<crate::light::Daylight>,
    lights: Res<crate::light::LightSettings>,
    torch_art: Option<Res<crate::light::TorchArt>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut toggles: ResMut<crate::light::LightToggles>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::brain::senses::Alert>), Without<LocalPlayer>>,
    mut hud: Query<&mut Text, With<SneakHud>>,
    tools: Res<ToolsConfig>,
    // (step, times spotted, when out, each guard's last wariness, the bot's
    // jump until, when it set off its bomb)
    mut state: Local<(u8, u32, Option<f32>, std::collections::HashMap<Entity, crate::creatures::brain::senses::Wary>, f32, Option<f32>)>,
) {
    use crate::creatures::brain::senses::Wary;
    if s.name != "sneak" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let fl = floor as f32;
    let bot = std::env::var("PLATYPUS_SNEAK_BOT").ok();
    if state.0 == 0 {
        if t < 0.5 {
            return;
        }
        // 23:00.
        day.skipped = (23.0 - day.time * 24.0).rem_euclid(24.0);
        let Some(stone) = sim.materials().id("stone") else { return };
        let mut fill = |x0: i32, x1: i32, y0: i32, y1: i32| {
            for x in (x0 + 2..=x1 - 2).step_by(3) {
                for y in (y0 + 2..=y1 - 2).step_by(3) {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 2, material: stone, overwrite: true });
                }
            }
        };
        // The roof (head room over the shelf for a jump).
        fill(300, 1680, floor + 140, floor + 152);
        // The camp's shelf, its ramp up.
        fill(560, 900, floor + 56, floor + 64);
        for x in (500..560).step_by(3) {
            fill(x, x + 4, floor - 2, floor + (x - 500) + 4);
        }
        // The walkway, a gap over the patrol; a step down off its end, high
        // enough to walk under.
        fill(940, 1190, floor + 56, floor + 62);
        fill(1225, 1420, floor + 56, floor + 62);
        fill(1436, 1460, floor + 30, floor + 35);
        // The camp's fire, the sentry's torch.
        if let Some(art) = torch_art.as_deref() {
            for x in [750.0, 1590.0] {
                crate::light::plant_torch(&mut commands, Vec2::new(x, fl), &lights, art);
            }
        }
        // (No training dummies.)
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        // The guards: (where, facing, wandering).
        for (x, facing, wander) in [(720.0, -1.0, 0.0), (780.0, -1.0, 0.0), (1110.0, 1.0, 0.3), (1600.0, -1.0, 0.0)] {
            crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(x, fl), move |e| {
                if let Some(mut h) = e.get_mut::<crate::creatures::brain::hunter::Hunter>() {
                    h.wander.speed = wander;
                }
                if let Some(mut k) = e.get_mut::<Kinematics>() {
                    k.loco.facing = facing;
                }
            });
        }
        k.body.pos = Vec2::new(200.0, fl + k.body.half.y);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
        toggles.carry = match bot.as_deref() {
            Some("torch") => crate::light::Carry::Torch,
            Some("lamp") => crate::light::Carry::SmallBeam,
            _ => crate::light::Carry::Nothing,
        };
        commands.spawn((
            SneakHud,
            Text::new(""),
            TextFont { font_size: bevy::text::FontSize::Px(15.0), ..default() },
            TextColor(Color::srgb(0.95, 0.9, 0.75)),
            bevy::ui::widget::TextShadow::default(),
            Node { position_type: PositionType::Absolute, top: px(96), width: percent(100), justify_content: JustifyContent::Center, ..default() },
            TextLayout::justify(Justify::Center),
        ));
        state.0 = 1;
        return;
    }
    // Who's grown how wary.
    for (e, c, gk, a) in &them {
        let Some(a) = a else { continue };
        let was = state.3.insert(e, a.wary);
        if was.is_some_and(|w| w != a.wary) {
            if a.wary == Wary::Hunting {
                state.1 += 1;
            }
            info!("sneak: t {t:.2} the {} at x {:.0} is {:?} (the player at x {:.0}, {:.0} up)", c.kind, gk.body.pos.x, a.wary, k.body.pos.x, k.body.pos.y - fl - k.body.half.y);
        }
    }
    if state.2.is_none() && k.body.pos.x > 1700.0 {
        state.2 = Some(t);
        info!("sneak: out at t {t:.1}, spotted {} times", state.1);
    }
    if let Ok(mut text) = hud.single_mut() {
        text.0 = match state.2 {
            Some(out) => format!("Out, {:.0} s, {}", out, if state.1 == 0 { "unseen. A ghost.".to_string() } else { format!("spotted {} times.", state.1) }),
            None => format!("Get through the hall unseen.    Ctrl: creep    L: a light (flashlights, a torch)    4: bombs make noise    Spotted: {}", state.1),
        };
    }
    // The bot: in, right, to the walkway's end; a bomb; out.
    if let Some(mode) = bot.as_deref() {
        let x = k.body.pos.x;
        let sentry = them.iter().filter(|q| q.1.kind == "orc").map(|q| q.2.body.pos.x).fold(f32::MIN, f32::max);
        if x >= 1320.0 && state.5.is_none() {
            state.5 = Some(t);
        }
        if state.0 == 1 && state.5.is_some_and(|b| t > b + 1.0) {
            info!("sneak: t {t:.2} the bot's bomb, down the hall at x 1150 (the sentry at x {sentry:.0})");
            crate::props::spawn_bomb(&mut commands, Vec2::new(1150.0, fl + 6.0), Vec2::ZERO, tools.bomb.clone());
            state.0 = 2;
        }
        // (Out once the furthest orc is back past it, or never came.)
        let going = x < 1320.0 || state.5.is_some_and(|b| t > b + 3.0 && sentry < 1300.0) || state.5.is_some_and(|b| t > b + 12.0);
        if going {
            keys.press(KeyCode::KeyD);
        } else {
            keys.release(KeyCode::KeyD);
        }
        // (Full pace for the jumps over the gaps, from a run-up.)
        let edge = [(880.0, 900.0), (1170.0, 1190.0)].iter().any(|&(a, b)| x > a && x < b && k.loco.grounded());
        let run_up = [(830.0, 900.0), (1120.0, 1190.0)].iter().any(|&(a, b)| x > a && x < b);
        if edge {
            state.4 = t + 0.35;
            keys.press(KeyCode::Space);
        } else if t > state.4 {
            keys.release(KeyCode::Space);
        }
        if mode != "run" && going && t > state.4 && !run_up && k.loco.grounded() {
            keys.press(KeyCode::ControlLeft);
        } else {
            keys.release(KeyCode::ControlLeft);
        }
    }
}

/// A creature put down for `stealth`: standing its ground while idle
/// (it doesn't wander out of reach), facing the player to its left.
fn stand_facing_left(e: &mut EntityWorldMut) {
    if let Some(mut h) = e.get_mut::<crate::creatures::brain::hunter::Hunter>() {
        h.wander.speed = 0.0;
    }
    if let Some(mut k) = e.get_mut::<Kinematics>() {
        k.loco.facing = -1.0;
    }
}

type Sensing<'a> = (Entity, &'a crate::creatures::Creature, &'a Kinematics, Option<&'a crate::creatures::brain::senses::Alert>);

/// Senses (BE `behaviour` stage 2), in a tunnel roofed over in the flat
/// arena (x 780..1320, its roof 44 up: no daylight in it), the player
/// standing still in it, each in turn:
/// - in the dark: an orc (`PLATYPUS_KIND`) 230 cells off doesn't see the
///   player (3 s): idle (each creature here stands its ground, facing the
///   player);
/// - the player lights a torch: it sees, and hunts;
/// - a wall across the tunnel, the torch still lit, an orc 120 cells off
///   behind it: it doesn't see through it (2.5 s); then the player runs
///   at the wall: it hears the footsteps and comes to look;
/// - the torch out, a cave spider (it sees in the dark) behind the wall:
///   idle; then the player is wounded: it smells the blood through the
///   rock, and hunts;
/// - crouched behind a wall knee-high (x 807..813), the torch lit, an orc
///   120 cells off: hidden (2.5 s); the player stands up: it hunts.
///
/// Logs each part's outcome, and every change of how wary it is.
#[allow(clippy::too_many_arguments)]
fn stealth_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut toggles: ResMut<crate::light::LightToggles>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<Sensing, Without<LocalPlayer>>,
    // (part, its foe's kind, the most wary it got, when it first hunted,
    // what it was last)
    mut state: Local<(u8, String, u8, f32, Option<crate::creatures::brain::senses::Wary>)>,
) {
    use crate::creatures::brain::senses::Wary;
    if s.name != "stealth" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let floor = platypus_worldgen::arena::FLOOR;
    let orc = std::env::var("PLATYPUS_KIND").unwrap_or_else(|_| "orc".into());
    let put = |k: &mut Kinematics, x: f32| {
        k.body.pos = Vec2::new(x, fl + k.body.half.y);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
    };
    let foe = them.iter().find(|q| q.1.kind == state.1);
    // Its wariness, as it changes.
    if let Some((_, c, fk, Some(a))) = foe {
        if state.4 != Some(a.wary) {
            info!("stealth: t {t:.2} the {} is {:?} at {:.0} cells", c.kind, a.wary, fk.body.pos.distance(k.body.pos));
            state.4 = Some(a.wary);
        }
        let rank = match a.wary {
            Wary::Idle => 0,
            Wary::Suspicious | Wary::Searching => 1,
            Wary::Hunting => 2,
        };
        state.2 = state.2.max(rank);
        if rank == 2 && state.3 < 0.0 {
            state.3 = t;
        }
    }
    let wary = |n: u8| ["idle", "suspicious", "hunting"][n as usize];
    let next = |state: &mut (u8, String, u8, f32, Option<Wary>), kind: &str| {
        state.0 += 1;
        state.1 = kind.to_string();
        state.2 = 0;
        state.3 = -1.0;
        state.4 = None;
    };
    let rid = |commands: &mut Commands| {
        for (e, ..) in them.iter().filter(|q| q.1.kind != "dummy") {
            commands.entity(e).despawn();
        }
    };
    match state.0 {
        0 if t > 0.5 => {
            if let Some(stone) = sim.materials().id("stone") {
                for x in (780..=1320).step_by(3) {
                    for y in [floor + 47, floor + 52] {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 3, material: stone, overwrite: true });
                    }
                }
            }
            put(&mut k, 840.0);
            toggles.carry = crate::light::Carry::Nothing;
            next(&mut state, &orc);
        }
        1 if t > 1.0 => {
            crate::creatures::def::spawn_creature(&mut commands, &orc, Vec2::new(1070.0, fl), stand_facing_left);
            state.0 = 2;
        }
        2 if t > 4.0 => {
            info!("stealth: in the dark, the {orc} {:.0} cells off got {} (idle: unseen)", foe.map_or(0.0, |f| f.2.body.pos.distance(k.body.pos)), wary(state.2));
            toggles.carry = crate::light::Carry::Torch;
            state.3 = -1.0;
            state.0 = 3;
        }
        3 if t > 6.5 => {
            let at = if state.3 >= 0.0 { format!("{:.2} s after", state.3 - 4.0) } else { "never".into() };
            info!("stealth: the torch lit at t 4.0: the {orc} hunted {at}");
            rid(&mut commands);
            if let Some(stone) = sim.materials().id("stone") {
                for x in (997..=1003).step_by(3) {
                    for y in (floor + 2..floor + 50).step_by(3) {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 3, material: stone, overwrite: true });
                    }
                }
            }
            put(&mut k, 930.0);
            next(&mut state, &orc);
        }
        4 if t > 7.0 => {
            crate::creatures::def::spawn_creature(&mut commands, &orc, Vec2::new(1050.0, fl), stand_facing_left);
            state.0 = 5;
        }
        5 if t > 9.5 => {
            info!("stealth: a wall between, the torch lit, the {orc} {:.0} cells off got {} (idle: unseen)", foe.map_or(0.0, |f| f.2.body.pos.distance(k.body.pos)), wary(state.2));
            keys.press(KeyCode::KeyD);
            state.0 = 6;
        }
        6 if t > 10.0 => {
            keys.release(KeyCode::KeyD);
            state.0 = 7;
        }
        7 if t > 12.0 => {
            let goal = foe.and_then(|f| f.3).and_then(|a| a.goal());
            info!("stealth: after the player ran at the wall (now at x {:.0}), the {orc} got {}, going to look at {:?}", k.body.pos.x, wary(state.2), goal.map(|g| g.round()));
            rid(&mut commands);
            toggles.carry = crate::light::Carry::Nothing;
            put(&mut k, 960.0);
            next(&mut state, "spider");
        }
        8 if t > 12.5 => {
            crate::creatures::def::spawn_creature(&mut commands, "spider", Vec2::new(1060.0, fl), stand_facing_left);
            state.0 = 9;
        }
        9 if t > 14.5 => {
            info!("stealth: a spider behind the wall, the player unhurt, got {} (idle: no sight through rock, nothing to smell)", wary(state.2));
            h.hp = h.max * 0.5;
            state.3 = -1.0;
            state.0 = 10;
        }
        10 if t > 16.5 => {
            let at = if state.3 >= 0.0 { format!("{:.2} s after", state.3 - 14.5) } else { "never".into() };
            info!("stealth: the player wounded at t 14.5: the spider hunted {at} (smelled through the wall)");
            rid(&mut commands);
            h.hp = h.max;
            // Behind a wall knee-high, crouched, the torch lit.
            if let Some(stone) = sim.materials().id("stone") {
                for x in (807..=813).step_by(3) {
                    for y in (floor + 2..=floor + 7).step_by(2) {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 2, material: stone, overwrite: true });
                    }
                }
            }
            put(&mut k, 795.0);
            keys.press(KeyCode::ControlLeft);
            toggles.carry = crate::light::Carry::Torch;
            next(&mut state, &orc);
            state.0 = 11;
        }
        11 if t > 17.0 => {
            crate::creatures::def::spawn_creature(&mut commands, &orc, Vec2::new(915.0, fl), stand_facing_left);
            state.0 = 12;
        }
        12 if t > 19.5 => {
            info!("stealth: crouched behind a knee-high wall, the torch lit, the {orc} {:.0} cells off got {} (idle: hidden)", foe.map_or(0.0, |f| f.2.body.pos.distance(k.body.pos)), wary(state.2));
            keys.release(KeyCode::ControlLeft);
            state.3 = -1.0;
            state.0 = 13;
        }
        13 if t > 21.5 => {
            let at = if state.3 >= 0.0 { format!("{:.2} s after", state.3 - 19.5) } else { "never".into() };
            info!("stealth: the player stood up at t 19.5: the {orc} hunted {at}");
            state.0 = 14;
        }
        _ => {}
    }
    if !(9..11).contains(&state.0) {
        h.hp = h.max;
    }
}

/// A climber over an overhang (BE `limbs` stage 3): a rock like a
/// mushroom in the flat arena, a stem (x 1002..1032, 100 high) under a cap
/// (x 942..1092, 12 thick), the player waiting on the cap; a creature
/// (`PLATYPUS_KIND`, the spider) put down on the floor to the left. To get
/// at the player it climbs the stem's face, goes along under the cap
/// upside down, and over its edge onto it. Logs four times a second where
/// it is, what it holds and the surface its legs are on; then when it
/// first held a wall, the ceiling, and got on top.
fn overhang_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(Entity, &mut Kinematics), With<LocalPlayer>>,
    them: Query<(&crate::creatures::Creature, &Kinematics, Option<&crate::creatures::body::legs::Legs>), Without<LocalPlayer>>,
    // (phase, next log, first on a wall, on the ceiling, on top, the kind)
    mut state: Local<(u8, f32, f32, f32, f32, String)>,
) {
    if s.name != "overhang" {
        return;
    }

    let Ok((me, mut k)) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let fl = floor as f32;
    let b = platypus_sim::edit::BLOCK;
    if state.0 == 0 && t > 0.5 {
        if let Some(stone) = sim.materials().id("stone") {
            for bx in 1002 / b..1032 / b {
                for by in floor / b..(floor + 100) / b {
                    sim.queue(WorldEdit::PlaceBlock { block: CellPos::new(bx, by), material: stone, back: false });
                }
            }
            for bx in 942 / b..1092 / b {
                for by in (floor + 100) / b..(floor + 112) / b {
                    sim.queue(WorldEdit::PlaceBlock { block: CellPos::new(bx, by), material: stone, back: false });
                }
            }
        }
        k.body.pos = Vec2::new(1070.0, fl + 128.0);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
        let kind = std::env::var("PLATYPUS_KIND").unwrap_or_else(|_| "spider".into());
        // (The camera on it.)
        commands.entity(me).remove::<crate::camera::CameraTarget>();
        // (Already on to the player: it's told where, not left to find out.)
        let aware = k.body.pos;
        crate::creatures::def::spawn_creature(&mut commands, &kind, Vec2::new(860.0, fl + 12.0), move |e| {
            e.insert((crate::camera::CameraTarget, crate::creatures::brain::senses::Alert::hunting(aware)));
        });
        *state = (1, 1.0, -1.0, -1.0, -1.0, kind);
        return;
    }
    if state.0 != 1 {
        return;
    }
    let Some((_, ck, legs)) = them.iter().find(|(c, _, _)| c.kind == state.5) else { return };
    let (x, y) = (ck.body.pos.x, ck.body.pos.y - fl);
    let holds = ck.loco.clinging();
    if state.2 < 0.0 && holds.is_some_and(|d| d.x != 0.0) {
        state.2 = t;
        info!("overhang: t {t:.2}: it holds the wall at ({x:.0}, {y:.0})");
    }
    if state.3 < 0.0 && holds == Some(Vec2::Y) {
        state.3 = t;
        info!("overhang: t {t:.2}: it holds the ceiling at ({x:.0}, {y:.0})");
    }
    if state.4 < 0.0 && y > 112.0 && ck.loco.grounded() {
        state.4 = t;
        info!("overhang: t {t:.2}: it's on top, at ({x:.0}, {y:.0})");
    }
    if t >= state.1 {
        state.1 += 0.25;
        let (surface, back) = legs.map_or((0.0, false), |l| l.surface());
        let (tilt, _, planted) = legs.map_or((0.0, 0.0, 0), |l| l.pose(0.0));
        info!(
            "overhang: t {t:.2}: at ({x:.0}, {y:.0}) vel {:?} holds {holds:?} grounded {} surface {surface:.0}°{} tilt {tilt:.0}° feet planted {planted} strain {:.2}{}",
            ck.body.vel.round(),
            ck.loco.grounded(),
            if back { " (from above, on the wall behind)" } else { "" },
            legs.map_or(0.0, |l| l.strain()),
            // (Segments: how many, how many of their feet hold, the last's
            // place.)
            legs.map(|l| l.segments()).filter(|(at, _)| !at.is_empty()).map_or(String::new(), |(at, held)| format!("; {} segments, {held} feet holding, the last at ({:.0}, {:.0})", at.len(), at[at.len() - 1].x, at[at.len() - 1].y - fl))
        );
    }
    if t > s.duration - 0.6 {
        state.0 = 2;
        info!("overhang: first on a wall at {:.1} s, the ceiling at {:.1} s, on top at {:.1} s (-1: never)", state.2, state.3, state.4);
    }
}

/// Spiders on the wall behind: a wall of stone blocks put up in the
/// background (x 840..990, up to 180), the player on a ledge at its top,
/// a spider below on the floor; it climbs the wall at the player.
fn backwall_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    spiders: Query<(&crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    // (phase, highest it climbed, time it first held on behind, closest)
    mut state: Local<(u8, f32, f32, f32)>,
) {
    if s.name != "backwall" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let fl = floor as f32;
    match state.0 {
        0 if t > 0.5 => {
            if let Some(stone) = sim.materials().id("stone") {
                let b = platypus_sim::edit::BLOCK;
                for bx in 840 / b..990 / b {
                    for by in floor / b..(floor + 180) / b {
                        sim.queue(WorldEdit::PlaceBlock { block: CellPos::new(bx, by), material: stone, back: true });
                    }
                }
                // A ledge at the top for the player.
                for x in (900..=990).step_by(3) {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, floor + 182), radius: 3, material: stone, overwrite: true });
                }
            }
            k.body.pos = Vec2::new(960.0, fl + 198.0);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            // (It knows where the player is: this is its climbing, not its eyes.)
            let aware = k.body.pos;
            crate::creatures::def::spawn_creature(&mut commands, "spider", Vec2::new(870.0, fl + 9.0), move |e| {
                e.insert(crate::creatures::brain::senses::Alert::hunting(aware));
            });
            *state = (1, 0.0, -1.0, f32::MAX);
        }
        1 => {
            if let Some((_, sk)) = spiders.iter().find(|(c, _)| c.kind == "spider") {
                state.1 = state.1.max(sk.body.pos.y - fl);
                state.3 = state.3.min(sk.body.pos.distance(k.body.pos));
                if state.2 < 0.0 && sk.loco.clinging() == Some(Vec2::ZERO) {
                    state.2 = t;
                    info!("backwall: the spider holds on to the wall behind at ({:.0}, {:.0}) after {:.1} s", sk.body.pos.x, sk.body.pos.y - fl, t - 0.5);
                }
            }
            if t > 7.0 {
                info!("backwall: the spider climbed to {:.0} cells up the wall behind (the ledge is at 182), came within {:.0} cells of the player", state.1, state.3);
                state.0 = 2;
            }
        }
        _ => {}
    }
}

type Pogoer<'a> = (&'a mut Kinematics, &'a crate::creatures::Health, Option<&'a crate::combat::Swing>);

/// The down-strike (DESIGN §8): a shortsword's down slash chained across
/// the three dummies with S held and the cursor off level ahead; the
/// longsword's plunge onto a dummy (the dive, the bounce, bouncing again
/// while S is held), then off a spider's spit thrown up at it; a bounce
/// off the lava; a plunge onto the floor beside a dummy (the slam).
#[allow(clippy::too_many_arguments)]
fn pogo_script(
    s: Res<Scenario>,
    mut player: Query<Pogoer, With<LocalPlayer>>,
    dummies: Query<(Entity, &crate::creatures::Creature, &Kinematics, &crate::creatures::custom::dummy::Tally), Without<LocalPlayer>>,
    spells: Query<&crate::magic::Spell>,
    book: Res<crate::magic::Spellbook>,
    mut casts: MessageWriter<crate::magic::CastRequest>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    // (phase, its start, bounces, the fastest fall, last vel.y, hits then)
    mut state: Local<(u8, f32, u32, f32, f32, u32)>,
) {
    if s.name != "pogo" {
        return;
    }
    let Ok((mut k, h, swing)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let p = k.body.pos;
    let dt = t - state.1;
    // (A dummy within 30 of where it stood: they're 90 apart, and a slam
    // beside one shoves it.)
    let hits = |x: f32| dummies.iter().find(|(_, c, dk, _)| c.kind == "dummy" && (dk.body.pos.x - x).abs() < 30.0).map_or(0, |d| d.3.hits);
    let dummy_at = |x: f32| dummies.iter().find(|(_, c, dk, _)| c.kind == "dummy" && (dk.body.pos.x - x).abs() < 30.0).map(|d| d.2.body.pos);
    // A bounce: rising fast just after falling.
    if k.body.vel.y > 225.0 && state.4 < 90.0 {
        state.2 += 1;
    }
    state.3 = state.3.min(k.body.vel.y);
    state.4 = k.body.vel.y;
    let mut want: Vec<KeyCode> = Vec::new();
    let mut click = false;
    let put = |k: &mut Kinematics, at: Vec2| {
        k.body.pos = at;
        k.body.vel = Vec2::ZERO;
        k.prev_pos = at;
    };
    let next = |state: &mut (u8, f32, u32, f32, f32, u32)| *state = (state.0 + 1, t, 0, 0.0, 0.0, 0);
    let all = || [1050.0, 1140.0, 1230.0].iter().map(|&x| hits(x)).sum::<u32>();
    match state.0 {
        // The shortsword (hotbar 2, slot 7).
        0 => {
            if (0.3..0.4).contains(&t) {
                want.push(KeyCode::KeyX);
            }
            if (0.5..0.6).contains(&t) {
                want.push(KeyCode::Digit7);
            }
            if t > 0.8 {
                put(&mut k, Vec2::new(1041.0, fl + 90.0));
                next(&mut state);
            }
        }
        // S held, the cursor level ahead: over a dummy, falling, strike.
        1 => {
            want.push(KeyCode::KeyS);
            let goal = [1050.0, 1140.0, 1230.0].into_iter().find(|&x| hits(x) == 0);
            if let Some(gx) = goal.and_then(dummy_at).map(|d| d.x) {
                // (Along the row, left to right.)
                if p.x < 1245.0 {
                    want.push(KeyCode::KeyD);
                }
                click = (p.x - gx).abs() < 21.0 && !k.loco.grounded() && k.body.vel.y < 0.0 && p.y - fl < 68.0;
            }
            cursor.0 = Some(p + Vec2::new(60.0, 3.0));
            if goal.is_none() || dt > 3.5 {
                info!(
                    "pogo: shortsword down slash, S held, cursor level ahead: {} bounces, dummies hit {} / {} / {} in {:.1}s; hp {:.0}",
                    state.2,
                    hits(1050.0),
                    hits(1140.0),
                    hits(1230.0),
                    dt,
                    h.hp
                );
                next(&mut state);
            }
        }
        // The longsword (slot 8); up over the middle dummy.
        2 => {
            if dt < 0.1 {
                want.push(KeyCode::Digit8);
            }
            if dt > 0.3 {
                put(&mut k, Vec2::new(1140.0, fl + 165.0));
                next(&mut state);
                state.5 = all();
            }
        }
        // A plunge: S and one click; S held 2 s (bouncing on), then let go.
        3 => {
            cursor.0 = Some(p + Vec2::new(0.0, -45.0));
            if dt < 2.0 {
                want.push(KeyCode::KeyS);
            }
            click = (0.05..0.12).contains(&dt);
            if (0.4..0.42).contains(&dt) {
                info!("pogo: plunging: a down-strike {}, falling at {:.0} cells/s", swing.is_some_and(|s| s.downward()), -state.3);
            }
            if dt > 3.0 {
                info!("pogo: longsword plunge from 165 up: fastest fall {:.0} cells/s, {} bounces (S held 2 s, then let go), the dummy struck {} times", -state.3, state.2, all() - state.5);
                next(&mut state);
            }
        }
        // Off a spider's spit: thrown up at the player as it plunges, off
        // to the side of the dummies.
        4 => {
            if dt < 0.05 {
                put(&mut k, Vec2::new(930.0, fl + 180.0));
            }
            cursor.0 = Some(p + Vec2::new(0.0, -45.0));
            want.push(KeyCode::KeyS);
            click = (0.1..0.15).contains(&dt);
            if (0.12..0.14).contains(&dt)
                && let Some(spell) = book.spells.iter().position(|x| x.id == "spider_spit")
                && let Some((e, ..)) = dummies.iter().next()
            {
                casts.write(crate::magic::CastRequest { caster: e, spell, from: Vec2::new(p.x, fl + 9.0), toward: p, alt: false });
            }
            if dt > 1.4 {
                info!("pogo: plunge onto a spit thrown up at it: {} bounces, {} spells left in the air, hp {:.0}", state.2, spells.iter().count(), h.hp);
                next(&mut state);
            }
        }
        // The shortsword, over the lava: S, and strike above it.
        5 => {
            if dt < 0.1 {
                want.push(KeyCode::Digit7);
            }
            if (0.1..0.15).contains(&dt) {
                put(&mut k, Vec2::new(1395.0, fl + 45.0));
            }
            if dt > 0.15 {
                want.push(KeyCode::KeyS);
                click = k.body.vel.y < 0.0 && p.y - fl < 36.0;
                if state.2 > 0 {
                    want.push(KeyCode::KeyA);
                }
            }
            cursor.0 = Some(p + Vec2::new(0.0, -45.0));
            if dt > 2.5 || (state.2 > 0 && p.x < 1335.0 && k.loco.grounded()) {
                info!("pogo: over the lava: {} bounces, hp {:.0}, now at x {:.0} (the pit is 1350..1440)", state.2, h.hp, p.x);
                next(&mut state);
            }
        }
        // The longsword: a plunge onto the floor beside the last dummy.
        6 => {
            if dt < 0.1 {
                want.push(KeyCode::Digit8);
            }
            // (Once the last swing's done and the longsword's in hand.)
            if (0.5..0.55).contains(&dt) {
                put(&mut k, Vec2::new(1209.0, fl + 105.0));
                state.5 = hits(1230.0);
            }
            if dt > 0.55 {
                cursor.0 = Some(p + Vec2::new(0.0, -45.0));
                want.push(KeyCode::KeyS);
                click = dt < 0.65;
            }
            if dt > 2.0 {
                info!("pogo: plunge onto the floor 21 cells from a dummy: it was struck {} times; hp {:.0}", hits(1230.0).saturating_sub(state.5), h.hp);
                next(&mut state);
            }
        }
        _ => {}
    }
    for key in [KeyCode::KeyX, KeyCode::Digit7, KeyCode::Digit8, KeyCode::KeyA, KeyCode::KeyD, KeyCode::KeyS] {
        match (want.contains(&key), keys.pressed(key)) {
            (true, false) => keys.press(key),
            (false, true) => keys.release(key),
            _ => {}
        }
    }
    match (click, mouse.pressed(MouseButton::Left)) {
        (true, false) => mouse.press(MouseButton::Left),
        (false, true) => mouse.release(MouseButton::Left),
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn hook_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut chests: ResMut<crate::hands::chests::Chests>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut player: Query<(&mut Kinematics, Option<&crate::gear::hook::Rope>, &crate::creatures::Health), With<LocalPlayer>>,
    boxes: Query<&Kinematics, (With<crate::hands::chests::Chest>, Without<LocalPlayer>)>,
    mut state: Local<(u8, f32, f32, f32)>,
) {
    if s.name != "hook" {
        return;
    }
    let Ok((mut k, rope, health)) = player.single_mut() else { return };
    // (Reeled in fast against rock, the rope never hurts: health stays.)
    let hp = health.hp;
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let fl = floor as f32;
    let beam = floor + 165;
    let (what, wraps) = rope.map_or(("none", 0), |r| r.state());
    let height = k.body.pos.y - k.body.half.y - fl;
    let dt = t - state.1;
    let mut held: Vec<KeyCode> = Vec::new();
    let put = |k: &mut Kinematics, x: f32| {
        k.body.pos = Vec2::new(x, fl + 12.0);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
    };
    let next = |state: &mut (u8, f32, f32, f32)| *state = (state.0 + 1, t, 0.0, 0.0);
    match state.0 {
        0 if t > 0.5 => {
            // A stone beam overhead; a stone column (1049..1060, up to 120)
            // with a bump on its face at 60; a chest out on the floor.
            if let Some(stone) = sim.materials().id("stone") {
                for x in (720..=1170).step_by(6) {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, beam + 4), radius: 4, material: stone, overwrite: true });
                }
                for y in (floor..floor + 117).step_by(4) {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(1054, y), radius: 5, material: stone, overwrite: true });
                }
                // An overhang: a slab out of the column's face at 75.
                for x in 1035..=1050 {
                    for y in [floor + 74, floor + 75] {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 2, material: stone, overwrite: true });
                    }
                }
            }
            chests.spawn_placed(&mut commands, Vec2::new(840.0, fl));
            put(&mut k, 990.0);
            next(&mut state);
        }
        // Up the column's face, past the bump.
        1 if dt > 0.4 => {
            cursor.0 = Some(Vec2::new(1051.0, fl + 105.0));
            if dt < 0.5 {
                held.push(KeyCode::KeyE);
            }
            if dt > 1.8 {
                info!("hook: hooked on the column's face above an overhang, from below and out: the hook {what}, at ({:.0}, {height:.0}) (the overhang's at 72..77, 1033..1052; hooked at ~105), moving {:.0}, health {hp:.0}", k.body.pos.x, k.body.vel.length());
                next(&mut state);
            }
        }
        // Onto the column's top edge (from its right, out in the open); W:
        // up onto it.
        2 => {
            if dt < 0.05 {
                put(&mut k, 1092.0);
            }
            cursor.0 = Some(Vec2::new(1059.0, fl + 120.0));
            if (0.1..0.2).contains(&dt) {
                held.push(KeyCode::KeyE);
            }
            if dt > 1.0 {
                held.push(KeyCode::KeyW);
            }
            if dt > 1.7 {
                info!("hook: hooked on the column's top and W: the hook {what}, at ({:.0}, {height:.0}) (the top is at ~117, 1049..1059), health {hp:.0}", k.body.pos.x);
                put(&mut k, 900.0);
                next(&mut state);
            }
        }
        // The beam overhead: pulled up, hanging.
        3 => {
            cursor.0 = Some(Vec2::new(900.0, beam as f32));
            if dt < 0.1 {
                held.push(KeyCode::KeyE);
            }
            if dt > 1.2 {
                info!("hook: hooked on the beam: the hook {what}, {height:.0} cells up (the beam is at 165)");
                state.2 = height;
                next(&mut state);
            }
        }
        // S: rappel down.
        4 => {
            held.push(KeyCode::KeyS);
            if dt > 0.9 {
                info!("hook: 0.9 s of S: down to {height:.0} cells, the hook {what}");
                next(&mut state);
            }
        }
        // D: pump a swing; then jump off.
        5 => {
            state.2 = state.2.max(k.body.pos.x - 900.0);
            state.3 = state.3.min(k.body.pos.x - 900.0);
            // (With the swing, as a player pumps: the way it's going; D to
            // start it. Fast enough, a jump off it.)
            if dt > 1.5 && k.body.vel.length() > 225.0 {
                held.push(KeyCode::Space);
                info!("hook: pumped {dt:.1} s: swung from x {:.0} to {:.0} of the anchor; let go moving ({:.0}, {:.0}), the hook {what}", state.3, state.2, k.body.vel.x, k.body.vel.y);
                next(&mut state);
            } else {
                held.push(if k.body.vel.x < -7.5 { KeyCode::KeyA } else { KeyCode::KeyD });
            }
            if dt > 5.5 {
                info!("hook: never fast enough to let go (moving {:.0})", k.body.vel.length());
                next(&mut state);
            }
        }
        // By the column's left face: pushed against it, then away: a kick.
        6 if dt > 1.2 => {
            if dt < 1.3 {
                put(&mut k, 1035.0);
            }
            cursor.0 = Some(Vec2::new(1041.0, beam as f32));
            if (1.4..1.5).contains(&dt) {
                held.push(KeyCode::KeyE);
            }
            if (2.5..3.2).contains(&dt) {
                held.push(KeyCode::KeyS);
            }
            if (3.2..3.6).contains(&dt) {
                held.push(KeyCode::KeyD);
            }
            if (3.6..3.7).contains(&dt) {
                held.push(KeyCode::KeyA);
                state.2 = state.2.min(k.body.vel.x);
            }
            if dt > 3.7 {
                info!("hook: against the column on the rope, then away: kicked off at {:.0} cells/s, the hook {what} ({wraps} wraps), health {hp:.0}", -state.2);
                next(&mut state);
            }
        }
        // The chest, pulled in.
        7 => {
            if let Some(c) = boxes.iter().next() {
                cursor.0 = Some(c.body.pos);
                if dt < 0.1 {
                    if state.2 == 0.0 {
                        state.2 = 1.0;
                        info!("hook: the chest is {:.0} cells off; hooking it", c.body.pos.distance(k.body.pos));
                    }
                    held.push(KeyCode::KeyE);
                }
                if dt > 1.4 {
                    info!("hook: the hook {what}; the chest now {:.0} cells off", c.body.pos.distance(k.body.pos));
                    next(&mut state);
                }
            }
        }
        _ => {}
    }
    for key in [KeyCode::KeyE, KeyCode::KeyW, KeyCode::KeyS, KeyCode::KeyA, KeyCode::KeyD, KeyCode::Space] {
        let on = held.contains(&key);
        if on && !keys.pressed(key) {
            keys.press(key);
        } else if !on && keys.pressed(key) {
            keys.release(key);
        }
    }
}

type BeamTarget<'a> = (&'a Kinematics, Option<&'a crate::creatures::custom::dummy::Tally>, Has<crate::creatures::body::elements::Burning>);

/// Beams and light: the frost ray swept over the pool (ice) and the lava
/// pit (stone), the fire ray on a dummy, the vaporiser into the floor and
/// at the sandbag, a star bomb into the floor. Logs what each did.
#[allow(clippy::too_many_arguments)]
fn beams_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut hand: ResMut<crate::hands::Hand>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    dummies: Query<BeamTarget, (With<crate::creatures::custom::dummy::Dummy>, Without<LocalPlayer>)>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32)>,
) {
    if s.name != "beams" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let count = |name: &str, xs: std::ops::Range<i32>, ys: std::ops::Range<i32>| {
        let Some(m) = sim.materials().id(name) else { return 0 };
        xs.flat_map(|x| ys.clone().map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).count()
    };
    let solid = |xs: std::ops::Range<i32>, ys: std::ops::Range<i32>| xs.flat_map(|x| ys.clone().map(move |y| (x, y))).filter(|&(x, y)| sim.world.is_solid(CellPos::new(x, y))).count();
    let mut stand = |x: f32| {
        if k.body.pos.distance(Vec2::new(x, floor as f32 + 12.0)) > 3.0 {
            k.body.pos = Vec2::new(x, floor as f32 + 12.0);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
        }
    };
    let (mut left, mut right) = (false, false);
    // (Hotbar 3: the radiant, frost and blaze staffs.)
    hand.bar = 2;
    let sweep = |a: f32, b: f32, from: f32, secs: f32| a + (b - a) * ((t - from) / secs).clamp(0.0, 1.0);
    match state.0 {
        0 if t > 0.5 => {
            info!("beams: the pool's ice {} cells before", count("ice", 450..645, floor - 78..floor + 3));
            state.0 = 1;
        }
        // The frost ray, along the pool from its left bank.
        1 => {
            hand.slot = 8;
            stand(438.0);
            cursor.0 = Some(Vec2::new(sweep(472.0, 622.0, 0.6, 2.4), floor as f32 - 1.5));
            left = t > 0.6;
            if t > 3.0 {
                info!("beams: frost ray swept 2.4 s over the pool: ice {} cells", count("ice", 450..645, floor - 78..floor + 3));
                state.0 = 2;
            }
        }
        // ... over the lava pit.
        2 => {
            hand.slot = 8;
            stand(1326.0);
            cursor.0 = Some(Vec2::new(sweep(1356.0, 1434.0, 3.2, 2.0), floor as f32 - 7.5));
            left = t > 3.2;
            if t > 5.4 {
                info!("beams: frost ray over the lava 2 s: basalt {} cells, lava {} left", count("basalt", 1347..1443, floor - 39..floor), count("lava", 1347..1443, floor - 39..floor));
                state.0 = 3;
            }
        }
        // The fire ray on the first dummy.
        3 => {
            hand.slot = 9;
            stand(960.0);
            cursor.0 = Some(Vec2::new(1050.0, floor as f32 + 15.0));
            left = t > 5.6;
            if t > 7.0 {
                let (hurt, burning) = dummies.iter().min_by(|a, b| a.0.body.pos.x.total_cmp(&b.0.body.pos.x)).map_or((0.0, false), |(_, tl, b)| (tl.map_or(0.0, |x| x.total), b));
                info!("beams: fire ray on the dummy 1.4 s: it took {hurt:.0}, burning {burning}");
                state.0 = 4;
                state.1 = solid(900..1020, floor - 60..floor) as f32;
            }
        }
        // The vaporiser into the floor ahead, then at the sandbag.
        4 => {
            hand.slot = 7;
            stand(960.0);
            cursor.0 = Some(Vec2::new(sweep(990.0, 1012.0, 7.2, 1.6), floor as f32 - 45.0));
            left = t > 7.2;
            if t > 8.8 {
                info!("beams: vaporiser into the floor 1.6 s: {} solid cells gone", state.1 as usize - solid(900..1020, floor - 60..floor));
                state.0 = 5;
            }
        }
        5 => {
            hand.slot = 7;
            stand(1269.0);
            cursor.0 = Some(Vec2::new(1305.0, floor as f32 + 15.0));
            left = t > 9.0;
            if t > 10.0 {
                let hurt = dummies.iter().max_by(|a, b| a.0.body.pos.x.total_cmp(&b.0.body.pos.x)).map_or(0.0, |(_, tl, _)| tl.map_or(0.0, |x| x.total));
                info!("beams: vaporiser on the sandbag 1 s: it took {hurt:.0}");
                state.0 = 6;
                state.1 = solid(720..1020, floor - 60..floor) as f32;
            }
        }
        // A star bomb into the floor.
        6 => {
            hand.slot = 7;
            stand(780.0);
            cursor.0 = Some(Vec2::new(855.0, floor as f32 - 7.5));
            right = (10.8..11.2).contains(&t);
            if t > 12.8 {
                info!("beams: a star bomb into the floor: {} solid cells gone", state.1 as usize - solid(720..1020, floor - 60..floor));
                state.0 = 7;
            }
        }
        _ => {}
    }
    for (b, on) in [(MouseButton::Left, left), (MouseButton::Right, right)] {
        if on && !mouse.pressed(b) {
            mouse.press(b);
        } else if !on && mouse.pressed(b) {
            mouse.release(b);
        }
    }
}

type Conjurer<'a> = (&'a mut Kinematics, &'a mut crate::hands::items::Inventory, &'a mut crate::creatures::Health);
type ConjureFoe<'a> = (Entity, &'a Kinematics, &'a crate::creatures::Health, Has<crate::creatures::body::elements::Burning>);

/// Walls and clouds: an ice wall between the player and a charging orc; a
/// fire wall another orc walks through; a toxic cloud on the first, then
/// lit with the fire ray. Logs each.
#[allow(clippy::too_many_arguments)]
fn conjure_script(
    mut commands: Commands,
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    mut hand: ResMut<crate::hands::Hand>,
    mut player: Query<Conjurer, With<LocalPlayer>>,
    orcs: Query<ConjureFoe, IceFoe>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, Option<Entity>, Option<Entity>)>,
) {
    if s.name != "conjure" {
        return;
    }
    let Ok((mut k, mut inv, mut hp)) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let count = |name: &str, xs: std::ops::Range<i32>, ys: std::ops::Range<i32>| {
        let Some(m) = sim.materials().id(name) else { return 0 };
        xs.flat_map(|x| ys.clone().map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).count()
    };
    // (The player isn't what's being tested: kept standing and whole.)
    k.body.pos.x = 900.0;
    hp.hp = hp.max;
    hand.bar = 2;
    let orc = |e: Option<Entity>| e.and_then(|e| orcs.get(e).ok()).map(|(_, k, h, b)| (k.body.pos.x, h.hp, b));
    let (mut left, mut right) = (false, false);
    match state.0 {
        0 if t > 0.5 => {
            // The acid staff out of the pack, onto hotbar 3.
            if let Some(items) = &items
                && let Some(i) = inv.slots.iter().position(|s| s.is_some_and(|s| items.def(s.item).id == "acid_staff"))
            {
                inv.slots.swap(i, 25);
            }
            k.body.pos = Vec2::new(900.0, floor as f32 + 12.0);
            k.prev_pos = k.body.pos;
            crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(1170.0, floor as f32 + 15.0), |_| {});
            state.0 = 1;
        }
        1 => {
            state.1 = orcs.iter().next().map(|(e, ..)| e);
            if state.1.is_some() {
                state.0 = 2;
            }
        }
        // An ice wall in the right-hand orc's way.
        2 => {
            hand.slot = 8;
            cursor.0 = Some(Vec2::new(975.0, floor as f32 + 30.0));
            right = (0.8..0.9).contains(&t);
            if t > 3.0 {
                info!("conjure: ice wall at x 970..980: ice {} cells; the orc behind it at x {:?}", count("ice", 960..990, floor..floor + 60), orc(state.1).map(|o| o.0));
                state.0 = 3;
            }
        }
        // A fire wall the left-hand orc walks into.
        3 => {
            hand.slot = 9;
            cursor.0 = Some(Vec2::new(810.0, floor as f32 + 30.0));
            right = (3.1..3.2).contains(&t);
            // Another orc, from the left, once the wall is up.
            if t > 3.4 && state.2.is_none() {
                info!("conjure: fire wall: {} cells of flame", count("fire", 795..825, floor..floor + 60));
                crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(705.0, floor as f32 + 15.0), |_| {});
                state.2 = Some(Entity::PLACEHOLDER);
            }
            if state.2 == Some(Entity::PLACEHOLDER) {
                state.2 = orcs.iter().find(|(e, ..)| Some(*e) != state.1).map(|(e, ..)| e).or(state.2);
            }
            if t > 6.0 {
                info!("conjure: the orc that walked into the fire wall: at x {:?}, hp {:?}, burning {:?}", orc(state.2).map(|o| o.0), orc(state.2).map(|o| o.1 as i32), orc(state.2).map(|o| o.2));
                state.0 = 4;
            }
        }
        // A toxic cloud on the orc behind the ice.
        4 => {
            hand.slot = 5;
            let at = orc(state.1).map_or(1050.0, |o| o.0);
            cursor.0 = Some(Vec2::new(at, floor as f32 + 18.0));
            left = (6.1..6.2).contains(&t);
            if t > 9.0 {
                info!("conjure: 2.7 s in the cloud: the orc's hp {:?}; miasma {} cells (sunk: {} below floor + 15)", orc(state.1).map(|o| o.1 as i32), count("miasma", 900..1200, floor - 30..floor + 90), count("miasma", 900..1200, floor - 30..floor + 15));
                state.0 = 5;
            }
        }
        // Lit with the fire ray.
        5 => {
            hand.slot = 9;
            let at = orc(state.1).map_or(1050.0, |o| o.0);
            cursor.0 = Some(Vec2::new(at, floor as f32 + 9.0));
            left = (9.1..9.6).contains(&t);
            if t > 9.7 && t < 9.72 {
                let lit = (900..1200).flat_map(|x| (floor - 30..floor + 90).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.flags & platypus_sim::cell::flags::BURNING != 0)).count();
                info!("conjure: fire ray into the cloud 0.6 s: {lit} cells burning, miasma {}", count("miasma", 900..1200, floor - 30..floor + 90));
            }
            if t > 10.5 {
                info!("conjure: fire ray into the cloud: miasma {} cells left, fire {}, the orc's hp {:?}, burning {:?}", count("miasma", 900..1200, floor - 30..floor + 90), count("fire", 900..1200, floor - 30..floor + 90), orc(state.1).map(|o| o.1 as i32), orc(state.1).map(|o| o.2));
                state.0 = 6;
            }
        }
        _ => {}
    }
    for (b, on) in [(MouseButton::Left, left), (MouseButton::Right, right)] {
        if on && !mouse.pressed(b) {
            mouse.press(b);
        } else if !on && mouse.pressed(b) {
            mouse.release(b);
        }
    }
}

/// Called from the sky: lightning on an orc in the open, then on one under
/// a stone roof (no sky: it fizzles), then a meteor on a third. Logs each.
/// (The player stands between the roofed one and the others.)
#[allow(clippy::too_many_arguments)]
fn call_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut hand: ResMut<crate::hands::Hand>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    orcs: Query<ConjureFoe, IceFoe>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32)>,
) {
    if s.name != "call" {
        return;
    }
    let Ok((mut k, mut hp)) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    hp.hp = hp.max;
    let at = |x: f32| orcs.iter().min_by(|a, b| (a.1.body.pos.x - x).abs().total_cmp(&(b.1.body.pos.x - x).abs())).map(|(_, k, h, burning)| (k.body.pos, h.hp, burning));
    let mut right = false;
    let hps = || orcs.iter().map(|(_, k, h, _)| (k.body.pos.x as i32, h.hp as i32)).collect::<Vec<_>>();
    match state.0 {
        0 if t > 0.5 => {
            k.body.pos = Vec2::new(930.0, floor as f32 + 12.0);
            k.prev_pos = k.body.pos;
            // A stone roof over the right-hand one.
            if let Some(stone) = sim.materials().id("stone") {
                for x in (780..=870).step_by(6) {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, floor + 75), radius: 4, material: stone, overwrite: true });
                }
            }
            for x in [1110.0, 1200.0, 825.0] {
                crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(x, floor as f32 + 15.0), |e| {
                    e.remove::<crate::creatures::brain::hunter::Hunter>();
                });
            }
            state.0 = 1;
        }
        // Call lightning (the storm staff's right button) on the first.
        1 if t > 1.0 => {
            hand.bar = 0;
            hand.slot = 9;
            if let Some((p, ..)) = at(1110.0) {
                cursor.0 = Some(p);
            }
            right = t < 1.1;
            if t > 2.2 {
                info!("call: lightning on the orc at 1110: orcs (x, hp) {:?}", hps());
                state.0 = 2;
            }
        }
        // ... on the one under the roof.
        2 => {
            if let Some((p, ..)) = at(825.0).filter(|(p, ..)| p.x < 900.0) {
                cursor.0 = Some(p);
            }
            right = (2.5..2.6).contains(&t);
            if t > 3.8 {
                info!("call: lightning on the orc under a stone roof: orcs (x, hp) {:?}", hps());
                state.0 = 3;
            }
        }
        // A meteor (the fire staff's right button) on the middle one.
        3 => {
            hand.bar = 0;
            hand.slot = 8;
            if let Some((p, ..)) = at(1200.0) {
                cursor.0 = Some(p);
            }
            right = (4.0..4.1).contains(&t);
            if t > 4.0 && state.1 == 0.0 {
                state.1 = (1140..1260).flat_map(|x| (floor - 60..floor).map(move |y| (x, y))).filter(|&(x, y)| sim.world.is_solid(CellPos::new(x, y))).count() as f32;
            }
            if t > 6.5 {
                let solid = (1140..1260).flat_map(|x| (floor - 60..floor).map(move |y| (x, y))).filter(|&(x, y)| sim.world.is_solid(CellPos::new(x, y))).count();
                let lava = sim.materials().id("lava").map_or(0, |m| (1050..1350).flat_map(|x| (floor - 60..floor + 60).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).count());
                info!("call: a meteor on the orc at 1200: orcs (x, hp) {:?}; a crater of {} cells, {} of lava about", hps(), state.1 as usize - solid, lava);
                state.0 = 4;
            }
        }
        _ => {}
    }
    if right && !mouse.pressed(MouseButton::Right) {
        mouse.press(MouseButton::Right);
    } else if !right && mouse.pressed(MouseButton::Right) {
        mouse.release(MouseButton::Right);
    }
}

type VoidCaster<'a> = (&'a mut Kinematics, &'a mut crate::hands::items::Inventory, &'a mut crate::creatures::Health);

/// Void: a blink along the floor; a portal pair (one in the floor, one on
/// the left face of a stone column) and a chest, water and the player
/// dropped into the floor one; stasis on a walking orc and a fireball
/// thrown into it. Logs each.
#[allow(clippy::too_many_arguments)]
fn void_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    mut chests: ResMut<crate::hands::chests::Chests>,
    mut hand: ResMut<crate::hands::Hand>,
    mut player: Query<VoidCaster, With<LocalPlayer>>,
    boxes: Query<&Kinematics, (With<crate::hands::chests::Chest>, Without<LocalPlayer>)>,
    orcs: Query<ConjureFoe, IceFoe>,
    spells: Query<&crate::magic::Spell>,
    portals: Query<&crate::magic::void::Portal>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32, Vec2)>,
) {
    if s.name != "void" {
        return;
    }
    let Ok((mut k, mut inv, mut hp)) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let fl = floor as f32;
    hp.hp = hp.max;
    let (mut left, mut right) = (false, false);
    let put = |k: &mut Kinematics, at: Vec2| {
        k.body.pos = at;
        k.body.vel = Vec2::ZERO;
        k.prev_pos = at;
    };
    let water = |sim: &SimWorld, xs: std::ops::Range<i32>| sim.materials().id("water").map_or(0, |m| xs.flat_map(|x| (floor..floor + 180).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).count());
    match state.0 {
        0 if t > 0.5 => {
            // The void wand and staff out of the pack, onto hotbar 3.
            if let Some(items) = &items {
                for (id, to) in [("void_wand", 20), ("void_staff", 21)] {
                    if let Some(i) = inv.slots.iter().position(|s| s.is_some_and(|s| items.def(s.item).id == id)) {
                        inv.slots.swap(i, to);
                    }
                }
            }
            put(&mut k, Vec2::new(780.0, fl + 12.0));
            state.0 = 1;
        }
        // Blink along the floor.
        1 => {
            hand.bar = 2;
            hand.slot = 0;
            cursor.0 = Some(Vec2::new(960.0, fl - 3.0));
            left = (0.7..0.75).contains(&t);
            if t > 1.4 {
                info!("void: blinked from x 780 toward x 960: now at x {:.0}, y {:.0}", k.body.pos.x, k.body.pos.y - fl);
                put(&mut k, Vec2::new(930.0, fl + 12.0));
                state.0 = 2;
            }
        }
        // A portal in the floor at 870, then one on the column's left face.
        2 => {
            hand.bar = 2;
            hand.slot = 1;
            // (The second from nearer the column: a void bolt reaches ~500 cells.)
            if t > 2.0 {
                put(&mut k, Vec2::new(1462.0, fl + 12.0));
            }
            cursor.0 = Some(if t < 2.0 { Vec2::new(870.0, fl - 3.0) } else { Vec2::new(1650.0, fl + 135.0) });
            left = (1.6..1.65).contains(&t) || (2.2..2.25).contains(&t);
            if t > 2.8 {
                let at: Vec<String> = portals.iter().map(|p| format!("({:.0}, {:.0}) facing ({:.0}, {:.0})", p.at.x, p.at.y - fl, p.normal.x, p.normal.y)).collect();
                info!("void: portals open: {at:?}");
                put(&mut k, Vec2::new(780.0, fl + 12.0));
                chests.spawn_placed(&mut commands, Vec2::new(871.0, fl + 90.0));
                state.0 = 3;
                state.1 = t;
            }
        }
        // A chest dropped into the floor portal.
        3 => {
            if let Some(c) = boxes.iter().next()
                && state.2 == Vec2::ZERO
                && c.body.pos.x > 1500.0
            {
                state.2 = c.body.pos;
                info!("void: the chest dropped into the floor portal came out at ({:.0}, {:.0}) moving ({:.0}, {:.0})", c.body.pos.x, c.body.pos.y - fl, c.body.vel.x, c.body.vel.y);
            }
            if t - state.1 > 1.5 {
                let c = boxes.iter().next().map(|c| c.body.pos);
                info!("void: 1.5 s later the chest is at {:?}", c.map(|c| (c.x as i32, (c.y - fl) as i32)));
                // Water poured over the floor portal.
                if let Some(w) = sim.materials().id("water") {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(870, floor + 60), radius: 12, material: w, overwrite: false });
                }
                state.0 = 4;
                state.1 = t;
            }
        }
        4 => {
            if t - state.1 > 2.0 {
                let obsidian = sim.materials().id("obsidian").map_or(0, |m| (1335..1455).flat_map(|x| (floor - 45..floor + 15).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).count());
                info!("void: water poured over the floor portal: {} cells left by it, {} by the column (x 1440..1650), {} obsidian on the lava pit", water(&sim, 810..930), water(&sim, 1440..1650), obsidian);
                // The player dropped in (over the floor portal, wherever the
                // bolt opened it; from 64 up: from 60 the tick before it
                // lands is just out of the portal's reach, and it stands on it).
                let over = portals.iter().find(|p| p.normal.y > 0.5).map_or(870.0, |p| p.at.x);
                put(&mut k, Vec2::new(over, fl + 64.0));
                state.0 = 5;
                state.1 = t;
                state.2 = Vec2::ZERO;
            }
        }
        5 => {
            if state.2 == Vec2::ZERO && k.body.pos.x > 1500.0 {
                state.2 = k.body.pos;
                info!("void: the player, dropped into the floor portal, came out at ({:.0}, {:.0}) moving ({:.0}, {:.0})", k.body.pos.x, k.body.pos.y - fl, k.body.vel.x, k.body.vel.y);
            }
            if t - state.1 > 1.5 {
                info!("void: 1.5 s later the player is at ({:.0}, {:.0})", k.body.pos.x, k.body.pos.y - fl);
                put(&mut k, Vec2::new(930.0, fl + 12.0));
                crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(1140.0, fl + 15.0), |_| {});
                state.0 = 6;
                state.1 = t;
            }
        }
        // Stasis on the orc as it comes, then a fireball into the bubble.
        6 => {
            let orc = orcs.iter().next().map(|o| o.1.body.pos);
            let dt = t - state.1;
            if let Some(o) = orc {
                cursor.0 = Some(o);
            }
            hand.bar = 2;
            hand.slot = 1;
            right = (0.8..0.85).contains(&dt);
            if (1.3..1.35).contains(&dt) {
                state.2 = orc.unwrap_or_default();
            }
            if dt > 1.4 {
                hand.bar = 0;
                hand.slot = 6;
                left = (1.4..1.45).contains(&dt);
            }
            if dt > 3.0 && state.0 == 6 {
                info!("void: stasis on the orc: it was at x {:.0} 1.3 s in, x {:?} now (3 s in); spells in flight {}", state.2.x, orc.map(|o| o.x as i32), spells.iter().count());
                state.0 = 7;
            }
        }
        7 if t - state.1 > 8.5 => {
            info!("void: after the bubble burst: the orc at x {:?}, hp {:?}; spells in flight {}", orcs.iter().next().map(|o| o.1.body.pos.x as i32), orcs.iter().next().map(|o| o.2.hp as i32), spells.iter().count());
            state.0 = 8;
        }
        _ => {}
    }
    for (b, on) in [(MouseButton::Left, left), (MouseButton::Right, right)] {
        if on && !mouse.pressed(b) {
            mouse.press(b);
        } else if !on && mouse.pressed(b) {
            mouse.release(b);
        }
    }
}

/// Falls: rocket high, fall, brake with the boots near the ground and land
/// (no hurt); the same without braking (hurt); a triple jump in cloud boots
/// (no hurt). Logs each landing.
fn fall_script(
    s: Res<Scenario>,
    deaths: Res<crate::creatures::PlayerDeaths>,
    items: Option<Res<crate::hands::items::Items>>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health, &mut crate::gear::Equipment), With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<(u8, f32, f32, f32)>,
) {
    if s.name != "fall" {
        return;
    }
    let Ok((mut k, mut hp, mut eq)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let height = k.body.pos.y - k.body.half.y - fl;
    let grounded = k.loco.grounded();
    let mut jump = false;
    let dt = t - state.1;
    // (phase, its start, the highest it got, the fastest it fell)
    state.2 = state.2.max(height);
    state.3 = state.3.min(k.body.vel.y);
    let start = |state: &mut (u8, f32, f32, f32), k: &mut Kinematics, hp: &mut crate::creatures::Health, phase: u8| {
        *state = (phase, t, 0.0, 0.0);
        k.body.pos = Vec2::new(930.0, fl + 12.0);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
        hp.hp = hp.max;
        // (Each run on full boots: they refill over seconds now; clamped to
        // what they hold.)
        k.loco.rocket_left = f32::MAX;
    };
    match state.0 {
        0 if t > 0.8 => start(&mut state, &mut k, &mut hp, 1),
        // Rocket up 1.2 s, fall; brake from 165 cells up until nearly still.
        1 | 3 => {
            let brake = state.0 == 1;
            jump = dt < 1.2 || brake && dt > 1.5 && height < 165.0 && k.body.vel.y < -60.0;
            if dt > 1.6 && grounded {
                info!("fall: rocket to {:.0} cells, fell at up to {:.0} cells/s, {}: landed with {:.0} of {:.0} hp (deaths so far {})", state.2, -state.3, if brake { "braked near the ground" } else { "no braking" }, hp.hp, hp.max, deaths.0);
                let next = state.0 + 1;
                start(&mut state, &mut k, &mut hp, next);
            }
        }
        2 if dt > 0.5 => start(&mut state, &mut k, &mut hp, 3),
        // A triple jump in cloud boots.
        4 if dt > 0.5 => {
            if let Some(items) = &items
                && let Some(b) = items.id("cloud_boots")
            {
                eq.worn[4] = Some(crate::hands::items::Stack::new(b, 1));
            }
            start(&mut state, &mut k, &mut hp, 5);
        }
        5 => {
            // A full jump, then two more off thin air, each held.
            jump = dt < 0.35 || (0.4..0.75).contains(&dt) || (0.8..1.15).contains(&dt);
            if dt > 0.6 && grounded {
                info!("fall: a triple jump in cloud boots to {:.0} cells: landed with {:.0} of {:.0} hp", state.2, hp.hp, hp.max);
                state.0 = 6;
            }
        }
        _ => {}
    }
    if jump && !keys.pressed(KeyCode::Space) {
        keys.press(KeyCode::Space);
    } else if !jump && keys.pressed(KeyCode::Space) {
        keys.release(KeyCode::Space);
    }
}

type Saver<'a> = (&'a mut Kinematics, Option<&'a mut crate::hands::items::Inventory>, Option<&'a crate::progress::Progress>);

/// Saving, in two runs (`PLATYPUS_SAVE` the same for both): `save_a`
/// (with `PLATYPUS_FRESH=1`) digs a hole, lays bricks, fills a chest, sets
/// down a workbench, gives the player gold and moves it, and saves; `save_b`
/// loads it. Both log the same things, to compare.
#[allow(clippy::too_many_arguments)]
fn save_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    mut chests: ResMut<crate::hands::chests::Chests>,
    crafting: Res<crate::craft::Crafting>,
    mut player: Query<Saver, With<LocalPlayer>>,
    boxes: Query<&crate::hands::chests::Chest>,
    stations: Query<&crate::craft::Station>,
    creatures: Query<&crate::creatures::Creature, Without<LocalPlayer>>,
    day: Res<crate::light::Daylight>,
    mut now: MessageWriter<crate::save::SaveNow>,
    mut state: Local<u8>,
) {
    let a = s.name == "save_a";
    if !(a || s.name == "save_b") {
        return;
    }
    let Some(items) = items else { return };
    let Ok((mut k, mut inv, progress)) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let count = |sim: &SimWorld, m: Option<platypus_sim::MaterialId>, xs: std::ops::Range<i32>, ys: std::ops::Range<i32>| {
        xs.flat_map(|x| ys.clone().map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| Some(c.material) == m || m.is_none() && c.is_air())).count()
    };
    if a && *state == 0 && t > 1.5 {
        *state = 1;
        let mats = sim.materials().clone();
        sim.queue(WorldEdit::Paint { center: CellPos::new(1050, floor - 15), radius: 12, material: platypus_sim::MaterialId::AIR, overwrite: true });
        if let Some(brick) = mats.id("brick") {
            sim.queue(WorldEdit::Paint { center: CellPos::new(840, floor + 45), radius: 6, material: brick, overwrite: true });
        }
        chests.spawn_placed(&mut commands, Vec2::new(900.0, floor as f32));
        if let (Some(key), Some(iron)) = (chests.stashes().map(|(k, ..)| k).max(), items.id("iron_bar")) {
            let world = &sim.world;
            chests.contents(key, world, &items).add(&items, crate::hands::items::Stack::new(iron, 3));
        }
        if let Some(bench) = crafting.station("workbench") {
            crafting.spawn(&mut commands, bench, Vec2::new(960.0, floor as f32));
        }
        if let (Some(inv), Some(gold)) = (inv.as_mut(), items.id("gold_bar")) {
            inv.add(&items, crate::hands::items::Stack::new(gold, 7));
        }
        k.body.pos = Vec2::new(780.0, floor as f32 + 12.0);
        k.prev_pos = k.body.pos;
        return;
    }
    if a && *state == 1 && t > 3.0 {
        *state = 2;
        now.write(crate::save::SaveNow);
    }
    if *state < 3 && t > 3.5 {
        *state = 3;
        let brick = sim.materials().id("brick");
        let gold = items.id("gold_bar");
        let iron = items.id("iron_bar");
        let chest_iron: u32 = chests.stashes().filter_map(|(_, _, c)| c).map(|c| iron.map_or(0, |i| c.count(i))).sum();
        info!(
            "{}: the hole {} air cells, bricks {}; the player at ({:.0}, {:.0}) with {} gold bars; {} chests ({} iron bars in them), {} stations, {} creatures; tick {}, {}; seen {} items",
            s.name,
            count(&sim, None, 1038..1064, floor - 27..floor - 1),
            count(&sim, brick, 831..851, floor + 36..floor + 56),
            k.body.pos.x,
            k.body.pos.y - floor as f32,
            inv.as_ref().map_or(0, |i| gold.map_or(0, |g| i.count(g))),
            boxes.iter().count(),
            chest_iron,
            stations.iter().count(),
            creatures.iter().count(),
            sim.world.tick(),
            day.clock(),
            progress.map_or(0, |p| p.seen.len()),
        );
    }
}

type Crafter<'a> = (&'a mut Kinematics, &'a mut crate::hands::items::Inventory, &'a crate::progress::Progress);

/// Crafting up the ladder: wood, stone, ore, coal and amethyst given; planks
/// and a workbench made, and set down; a furnace made at it and set down;
/// bars smelted; an anvil made and set down; a grappling hook made at it.
/// The stations are set down with the hands, as a player does. Logs each
/// step, the milestones reached and the recipes shown.
#[allow(clippy::too_many_arguments)]
fn craft_script(
    s: Res<Scenario>,
    items: Option<Res<crate::hands::items::Items>>,
    crafting: Res<crate::craft::Crafting>,
    mut hand: ResMut<crate::hands::Hand>,
    mut open: ResMut<crate::hands::InventoryOpen>,
    mut player: Query<Crafter, With<LocalPlayer>>,
    stations: Query<&crate::craft::Station>,
    mut asks: MessageWriter<crate::craft::CraftRequest>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: Local<(u8, f32)>,
) {
    if s.name != "craft" {
        return;
    }
    let Some(items) = items else { return };
    let Ok((mut k, mut inv, progress)) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR as f32;
    let dt = t - state.1;
    let mut click = false;
    let mut ask = |makes: &str, n: usize| {
        let Some(r) = crafting.recipe(makes) else { return warn!("no recipe for {makes}") };
        for _ in 0..n {
            asks.write(crate::craft::CraftRequest(r));
        }
    };
    let have = |inv: &crate::hands::items::Inventory, id: &str| items.id(id).map_or(0, |i| inv.count(i) / items.unit(i));
    // A made station into hotbar 3's first slot, in hand.
    let mut hold = |inv: &mut crate::hands::items::Inventory, id: &str| {
        if let Some(i) = inv.slots.iter().position(|s| s.is_some_and(|s| items.def(s.item).id == id)) {
            inv.slots.swap(i, 20);
        }
        hand.bar = 2;
        hand.slot = 0;
    };
    let next = |state: &mut (u8, f32)| *state = (state.0 + 1, t);
    match state.0 {
        0 if t > 0.8 => {
            k.body.pos = Vec2::new(930.0, floor + 12.0);
            k.prev_pos = k.body.pos;
            for (id, n) in [("block:wood", 8), ("block:stone", 12), ("block:copper_ore", 6), ("block:iron_ore", 24), ("block:coal", 2), ("block:amethyst", 3)] {
                if let Some(item) = items.id(id) {
                    inv.add(&items, crate::hands::items::Stack::new(item, n * items.unit(item)));
                }
            }
            next(&mut state);
        }
        // By hand: planks, a workbench.
        1 if dt > 0.3 => {
            ask("block:planks", 6);
            ask("workbench", 1);
            next(&mut state);
        }
        // (Set down within the hand's reach: 36 cells.)
        2 if dt > 0.2 => {
            if dt < 0.22 {
                info!("craft: by hand: {} planks left, {} workbench", have(&inv, "block:planks"), have(&inv, "workbench"));
            }
            hold(&mut inv, "workbench");
            cursor.0 = Some(Vec2::new(936.0, floor + 3.0));
            click = dt < 0.4;
            if dt > 0.55 {
                next(&mut state);
            }
        }
        // At the workbench: a furnace; set down.
        3 => {
            if dt < 0.05 {
                ask("furnace", 1);
            }
            if dt > 0.2 {
                hold(&mut inv, "furnace");
                cursor.0 = Some(Vec2::new(909.0, floor + 3.0));
                click = dt < 0.45;
            }
            if dt > 0.6 {
                info!("craft: {} stations set down; a furnace made: {}", stations.iter().count(), stations.iter().count() >= 2);
                next(&mut state);
            }
        }
        // Smelting; an anvil at the workbench; set down; a hook at it.
        4 => {
            if dt < 0.05 {
                ask("copper_bar", 2);
                ask("iron_bar", 8);
            }
            if (0.2..0.25).contains(&dt) && have(&inv, "anvil") == 0 && stations.iter().count() < 3 {
                info!("craft: smelted: {} copper bars, {} iron bars", have(&inv, "copper_bar"), have(&inv, "iron_bar"));
                ask("anvil", 1);
            }
            if dt > 0.4 {
                hold(&mut inv, "anvil");
                cursor.0 = Some(Vec2::new(960.0, floor + 3.0));
                click = dt < 0.65;
            }
            if dt > 0.8 {
                ask("grappling_hook", 1);
                next(&mut state);
            }
        }
        5 if dt > 0.3 => {
            let shown = crafting.listed(&items, &inv, progress, &(0..crafting.stations.len()).collect::<Vec<_>>());
            let altar = crafting.recipe("arcane_altar").is_some_and(|r| shown.iter().any(|(i, _)| *i == r));
            info!(
                "craft: {} stations; a grappling hook: {}; {} iron bars left (3 from the Smith milestone); milestones {:?}; {} recipes shown, the arcane altar's among them: {altar}",
                stations.iter().count(),
                have(&inv, "grappling_hook"),
                have(&inv, "iron_bar"),
                progress.done,
                shown.len()
            );
            open.0 = true;
            next(&mut state);
        }
        _ => {}
    }
    if click && !mouse.pressed(MouseButton::Left) {
        mouse.press(MouseButton::Left);
    } else if !click && mouse.pressed(MouseButton::Left) {
        mouse.release(MouseButton::Left);
    }
}
