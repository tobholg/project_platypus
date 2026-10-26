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
//! - `pack`       (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat PLATYPUS_MODE=play`) the
//!   inventory screen: Esc, Quick sort, the crafting panel's tabs, a recipe made, Settings
//!   (the game menu); `PLATYPUS_SHOTS=dir`: a picture of each step
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
//!   starts (spit, bite, sting, web), each hit the player takes, and venom;
//!   its web, phase by phase and its strand's way (shot, caught, reeled
//!   in, missed, broken free of). `PLATYPUS_SPIDER_DODGE`: the player
//!   slips the strand (`dash`: through it as it comes, `jump`: over it as
//!   it's shot, `run`: away from the tell on) or breaks free of it caught
//!   (`breakdash`, `strike`: a longsword at the spider; `wall`: stone put
//!   up between them cuts it);
//!   `PLATYPUS_SPIDER_SHOTS=dir`: pictures of each web's tell, its flight,
//!   the player reeled in, the bite, a miss
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

mod ladder;

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
            .add_systems(PreUpdate, sleep_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, placing_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, chase_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, haunt_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, drips_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, cavein_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, firedamp_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, death_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, rest_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, wallhook_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, aimswing_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, gauntlet_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, switch_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, lightthrow_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, groundbuild_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, ladder::ladder_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, chaos_script)
            .add_systems(Update, (camp_script, spiderdeath_script, camplook_script, reset_script, forestfire_script, regrow_script, wildfire_script, refill_script, gold_script, goldheap_script, kick_script, boulder_script))
            .add_systems(PreUpdate, (bowshot_script, damp_script, lakes_script).after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, (logmagic_script, cast_script, trap_script, village_script, shop_script, climb_script, star_script, raid_script, quake_script, pedlar_script, troll_script, reactions_script, grab_script, bestiary_script, reload_script, layouts_script, record_script, course_script, dig_script, legs_script).after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, (rocket_script, rocketswim_script, soak_script, pickarea_script).after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, spider_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, hook_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, pogo_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, leap_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, slimecheck_script)
            .add_systems(Update, structure_script)
            .add_systems(Update, ores_script)
            .add_systems(PreUpdate, mimic_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, wraith_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, dread_script)
            .add_systems(Update, olmer_script)
            .add_systems(Update, heatglow_script)
            .add_systems(PreUpdate, play_script.after(InputSystems).before(crate::camera::track_cursor).before(crate::pause::key))
            .add_systems(PreUpdate, title_script.after(InputSystems).before(crate::title::capture))
            .add_systems(PreUpdate, chop_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, starter_script)
            .add_systems(Update, close_window)
            .add_systems(PreUpdate, lights_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, throw_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, specks_script)
            .add_systems(Update, hurt_script)
            .add_systems(Update, castle_script)
            .add_systems(PreUpdate, shield_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, ropejump_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, blade_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, ridge_script)
            .add_systems(PreUpdate, goldsmash_script.after(InputSystems).before(crate::magic::well::twist_input).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, devmenu_script.after(bevy::ui::UiSystems::Focus).before(crate::devmenu::clicks))
            .add_systems(PreUpdate, volume_script.after(bevy::ui::UiSystems::Focus).before(crate::pause::slide))
            .add_systems(PreUpdate, devmenu_place_script.after(InputSystems).before(crate::devmenu::place))
            .add_systems(PreUpdate, devmenu_pick_script.after(bevy::ui::UiSystems::Focus).before(crate::devmenu::clicks))
            .add_systems(PreUpdate, toad_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, necro_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, strikes_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, (watcher_script, gnasher_script, bats_script, headshot_script, keepsake_script, rats_script, worldmap_script, ropehook_script, loadouts_script, jumps_script, stormboots_script, layers_script, crimson_script, lineup_script, sprint_script, phase_script, build_script))
            .add_systems(PreUpdate, templar_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, backwall_script)
            .add_systems(Update, (overhang_script, shots_script, parts_script, safari_script, reach_script))
            .add_systems(PreUpdate, stealth_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, sneak_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, tactics_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(Update, factions_script)
            .add_systems(Update, burrow_script)
            .add_systems(Update, show_script)
            .add_systems(Update, ragdoll_script)
            .add_systems(Update, slope_script)
            .add_systems(Update, ledge_script)
            .add_systems(Update, rise_script)
            .add_systems(Update, limp_script)
            .add_systems(Update, sever_script)
            .add_systems(Update, zombie_script)
            .add_systems(Update, night_script)
            .add_systems(Update, slime_script)
            .add_systems(Update, skull_script)
            .add_systems(Update, knock_script)
            .add_systems(Update, marks_script)
            .add_systems(Update, pickups_script)
            .add_systems(PreUpdate, filter_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, cursor_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, optics_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PostUpdate, plunge_script.after(bevy::transform::TransformSystems::Propagate))
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
            .add_systems(Update, (ratbend_script, bladesplash_script, dryout_script, hearts_script, signs_script, brew_script, herbs_script, slimeswing_script, wardrobe_script, herbcut_script, zombiejump_script, crushcheck_script, batcount_script, airswing_script, dive_script, paint_script, firewall_script, fields_script))
            .add_systems(PreUpdate, craft_script.after(InputSystems).before(crate::camera::track_cursor))
            .add_systems(PreUpdate, pack_script.after(bevy::ui::UiSystems::Focus).before(crate::pause::key))
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
        // (`PLATYPUS_RAIN_ROOF=1`: a stone slab overhead with gaps in it,
        // to shade the sun: god rays through them.)
        0 if s.elapsed > 1.0 && std::env::var("PLATYPUS_RAIN_ROOF").is_ok() && s.elapsed < 1.1 => {
            if let Some(stone) = sim.materials().id("stone") {
                let y = p.body.pos.y as i32 + 110;
                for xx in (x - 260..x + 260).step_by(1) {
                    if (xx - x).rem_euclid(70) < 12 {
                        continue;
                    }
                    for dy in 0..10 {
                        sim.world.set(CellPos::new(xx, y + dy), platypus_sim::Cell::new(stone, 120));
                    }
                }
            }
        }
        0 if s.elapsed > 2.0 => {
            if let Some(ground) = find_ground(&sim.world, x + 120, p.body.pos.y as i32 + 90, 450)
                && std::env::var("PLATYPUS_RAIN_ROOF").is_err()
            {
                sim.queue(WorldEdit::Ignite { center: CellPos::new(x + 120, ground + 3), radius: 9 });
            }
            *step = 1;
        }
        // What F5 does (`PLATYPUS_RAIN_CLEAR=1`: F6's clear sky instead,
        // to compare).
        1 if s.elapsed > 3.0 => {
            sim.queue(WorldEdit::Weather { x, radius: 600, storm: std::env::var("PLATYPUS_RAIN_CLEAR").is_err() });
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
    day.skipped = (day.skipped + hour - day.time * 24.0).rem_euclid(24.0);
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
        day.skipped = (day.skipped + 23.0 - day.time * 24.0).rem_euclid(24.0);
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
    mut hand: ResMut<crate::hands::Hand>,
) {
    if s.name != "archery" {
        return;
    }
    let (Ok((k, h, inv)), Some(items)) = (player.single(), items) else { return };
    // (The bow, wherever the kit put it.)
    if t_before(&s, 0.6) {
        take_up(&mut hand, inv, Some(&items), "bow");
    }
    let p = k.body.pos;
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR as f32;
    let quiver = items.id("arrow").map_or(0, |a| inv.count(a));
    let mut want = std::collections::HashSet::new();
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

/// Fireflies and fish put where they live, and checked on.
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
        *state = 1;
    }
    if *state == 1 && s.elapsed > 5.5 {
        let world = &sim.world;
        let wet = |p: Vec2| world.get(CellPos::from_world(p.x, p.y)).is_some_and(|c| world.materials().phys(c.material).kind == platypus_sim::Kind::Liquid);
        for kind in ["firefly", "fish"] {
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

/// A focus held, to see its aura (`PLATYPUS_FOCUS=acid_staff`: that one,
/// wherever it is in the pack, swapped into the slot).
#[allow(clippy::too_many_arguments)]
fn foci_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut hand: ResMut<crate::hands::Hand>,
    mut player: Query<(Entity, &Kinematics, Option<&mut crate::hands::items::Inventory>), With<LocalPlayer>>,
    items: Option<Res<crate::hands::items::Items>>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
) {
    if s.name != "foci" {
        return;
    }
    let Ok((me, k, inv)) = player.single_mut() else { return };
    let slot: usize = std::env::var("PLATYPUS_SLOT").ok().and_then(|v| v.parse().ok()).unwrap_or(7);
    hand.bar = (slot - 1) / 10;
    hand.slot = (slot - 1) % 10;
    if let (Ok(id), Some(mut inv), Some(items)) = (std::env::var("PLATYPUS_FOCUS"), inv, items.as_deref())
        && let Some(item) = items.id(&id)
        && let Some(i) = inv.slots.iter().position(|st| st.is_some_and(|st| st.item == item))
    {
        let to = hand.active();
        if i != to {
            inv.slots.swap(i, to);
        }
    }
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
    (inv, items): (Query<&crate::hands::items::Inventory, With<LocalPlayer>>, Option<Res<crate::hands::items::Items>>),
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
        if let Ok(inv) = inv.single() {
            take_up(&mut hand, inv, items.as_deref(), "frost_wand");
        }
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
    // (A jump, let go, then held: the boots fire on a second press in the
    // air.)
    let hold = state.0 == 2 && !(1.15..1.25).contains(&t);
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
                    hits.write(crate::combat::Hit { by: None, target: e, damage: 24.0, harm: crate::creatures::Harm::Slash, knock: dir * 375.0, stun: 0.3, at: sk.body.pos, dir, weight: 2.0, crit: false });
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

/// `lakes` (a generated world or a save: `PLATYPUS_SAVE`): finds the
/// biggest bodies of water under the ground near the player (the world as
/// generated: chunks `PLATYPUS_LAKE_REACH` either way, 60 by default, round
/// the player or `PLATYPUS_LAKE_AT=x,y`; the
/// richest in water, near ones merged), then stands the player (kept
/// alive) just over each in turn, `PLATYPUS_LAKE_SECS` (8) each, logging
/// the frame rate there and the sim's tick. (With `--features spikes` and
/// `PLATYPUS_PROFILE`, where the time went.)
fn lakes_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    metrics: Res<crate::world::SimMetrics>,
    time: Res<Time<Real>>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    mut state: Local<(Vec<(CellPos, usize)>, usize, f32, u32, f32)>,
    mut sample: Local<f32>,
) {
    if s.name != "lakes" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let (lakes, at, since, frames, ticks) = &mut *state;
    let secs: f32 = std::env::var("PLATYPUS_LAKE_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(8.0);
    if lakes.is_empty() && t > 1.0 {
        let reach: i32 = std::env::var("PLATYPUS_LAKE_REACH").ok().and_then(|v| v.parse().ok()).unwrap_or(60);
        let water = sim.materials().id("water");
        // (`PLATYPUS_LAKE_AT=x,y`: round there instead of the player.)
        let around = std::env::var("PLATYPUS_LAKE_AT").ok().and_then(|v| {
            let (x, y) = v.split_once(',')?;
            Some(Vec2::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
        });
        let me = around.map_or_else(|| CellPos::from_world(k.body.pos.x, k.body.pos.y), |a| CellPos::from_world(a.x, a.y)).chunk();
        let (lo, hi) = sim.generator.bounds();
        let surface = sim.generator.spawn_point().y;
        let mut wet: Vec<(platypus_sim::ChunkPos, usize)> = Vec::new();
        for cy in (me.y - reach).max(lo.y)..=(me.y + reach).min(hi.y) {
            // (Under the ground only.)
            if cy * platypus_sim::CHUNK as i32 > surface - 200 {
                continue;
            }
            for cx in (me.x - reach).max(lo.x)..=(me.x + reach).min(hi.x) {
                let pos = platypus_sim::ChunkPos::new(cx, cy);
                let chunk = sim.generator.generate(pos);
                let n = chunk.cells().iter().filter(|c| Some(c.material) == water).count();
                if n > 400 {
                    wet.push((pos, n));
                }
            }
        }
        // The richest, near ones (within 6 chunks) counted as one lake.
        wet.sort_by_key(|w| std::cmp::Reverse(w.1));
        let mut found: Vec<(platypus_sim::ChunkPos, usize)> = Vec::new();
        for (pos, n) in &wet {
            if let Some(f) = found.iter_mut().find(|f| (f.0.x - pos.x).abs() <= 6 && (f.0.y - pos.y).abs() <= 6) {
                f.1 += n;
            } else {
                found.push((*pos, *n));
            }
        }
        found.sort_by_key(|f| std::cmp::Reverse(f.1));
        found.truncate(std::env::var("PLATYPUS_LAKES").ok().and_then(|v| v.parse().ok()).unwrap_or(4));
        info!("lakes: {} wet chunks; the biggest: {:?}", wet.len(), found.iter().map(|(p, n)| (p.x * platypus_sim::CHUNK as i32, p.y * platypus_sim::CHUNK as i32, n)).collect::<Vec<_>>());
        *lakes = found.into_iter().map(|(p, n)| (CellPos::new(p.x * platypus_sim::CHUNK as i32 + 32, p.y * platypus_sim::CHUNK as i32 + 32), n)).collect();
        *at = usize::MAX;
        *since = t;
        return;
    }
    if lakes.is_empty() {
        return;
    }
    if *at >= lakes.len() && *at != usize::MAX {
        return;
    }
    *frames += 1;
    *ticks += metrics.tick_time.as_secs_f32() * 1000.0;
    // Each second: how busy the cells are (chunks awake, cells visited, the
    // step's phases, ms).
    if t >= *sample {
        *sample = t + 1.0;
        let m = &metrics.last;
        let phases: Vec<String> = platypus_sim::PHASES.iter().zip(m.phases).map(|(n, d)| format!("{n} {:.1}", d.as_secs_f32() * 1000.0)).collect();
        info!("lakes: t {t:.0} tick {:.2} ms: {} chunks awake, {} cells visited; {}", metrics.tick_time.as_secs_f32() * 1000.0, m.active_chunks, m.visited_cells, phases.join(", "));
        // Round the lake's middle: its water, how much of it is off the
        // ambient heat, how much still sloshing (not at rest).
        if let Some(&(c, _)) = lakes.get(*at) {
            let mats = sim.materials();
            let (mut water, mut warm, mut restless, mut lowest) = (0, 0, 0, i32::MAX);
            for dy in (-200..200).step_by(2) {
                for dx in (-400..400).step_by(2) {
                    let p = c.offset(dx, dy);
                    if let Some(cell) = sim.world.get(p).filter(|cell| !cell.is_air() && mats.phys(cell.material).kind == platypus_sim::Kind::Liquid) {
                        water += 1;
                        warm += (cell.heat != 0) as usize;
                        restless += (platypus_sim::cell::flags::rest(cell.flags) < platypus_sim::cell::flags::REST_LIMIT) as usize;
                        lowest = lowest.min(p.y);
                    }
                }
            }
            info!("lakes:   its liquid (1 in 4 sampled): {water}, off ambient {warm}, not at rest {restless}");
        }
    }
    let next = if *at == usize::MAX { 0 } else { *at + 1 };
    if *at == usize::MAX || t - *since >= secs {
        if *at != usize::MAX {
            let span = t - *since;
            info!("lakes: #{} at ({}, {}) water {}: {:.0} fps, sim tick {:.2} ms, frame {:.1} ms, {} chunks loaded, {} particles", *at, lakes[*at].0.x, lakes[*at].0.y, lakes[*at].1, *frames as f32 / span, *ticks / *frames as f32, span * 1000.0 / *frames as f32, sim.world.loaded_count(), sim.world.particles().len());
        }
        if next >= lakes.len() {
            *at = next;
            return;
        }
        *at = next;
        *since = t;
        *frames = 0;
        *ticks = 0.0;
        // Just over it: up its column from its middle to open air over water.
        let c = lakes[next].0;
        let spot = (0..400).map(|d| c.offset(0, d)).find(|p| sim.world.get(*p).is_none_or(|cell| cell.is_air())).unwrap_or(c);
        k.body.pos = Vec2::new(spot.x as f32 + 0.5, spot.y as f32 + 12.0);
        k.body.vel = Vec2::ZERO;
        info!("lakes: to #{next} at ({}, {})", spot.x, spot.y);
    }
    let _ = time;
}

/// `damp` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat PLATYPUS_MODE=dev`):
/// firedamp all round the player (in the dev kit's rocket boots), what's in
/// its hand `PLATYPUS_HOLD` (an item id; bow by default); it jumps now and
/// then (never firing the boots; `PLATYPUS_ROCKET`: firing them). `hazards`
/// logs what lights it.
fn damp_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut hand: ResMut<crate::hands::Hand>,
    items: Option<Res<crate::hands::items::Items>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    player: Query<(&Kinematics, &crate::hands::items::Inventory), With<LocalPlayer>>,
    mut done: Local<bool>,
) {
    if s.name != "damp" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let Ok((k, inv)) = player.single() else { return };
    let hold = std::env::var("PLATYPUS_HOLD").unwrap_or_else(|_| "bow".into());
    if let Some(i) = items.as_ref().and_then(|items| inv.slots.iter().position(|st| st.is_some_and(|st| items.def(st.item).id == hold))) {
        (hand.bar, hand.slot) = (i / crate::hands::items::HOTBAR, i % crate::hands::items::HOTBAR);
    }
    if !*done && t > 1.0 {
        *done = true;
        if let Some(gas) = sim.materials().id("firedamp") {
            let c = CellPos::from_world(k.body.pos.x, k.body.pos.y);
            for dy in -12..40 {
                for dx in -40..40 {
                    let p = c.offset(dx, dy);
                    if sim.world.get(p).is_some_and(|cell| cell.is_air()) {
                        sim.world.set(p, platypus_sim::Cell::new(gas, 128));
                    }
                }
            }
        }
    }
    // A jump now and then, held through (one press: never the boots); with
    // `PLATYPUS_ROCKET`, pressed again in the air and held: the boots fire.
    let phase = t % 2.0;
    let rocket = std::env::var("PLATYPUS_ROCKET").is_ok();
    let down = t > 1.5 && (phase < 0.25 || (rocket && (0.35..1.2).contains(&phase)));
    if down { keys.press(KeyCode::Space) } else { keys.release(KeyCode::Space) }
}

/// `bowshot` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat PLATYPUS_MODE=dev`):
/// the bow in hand, a zombie held 70 to the right; the player fires full
/// draws (the left button held 0.8 s, let go 0.3 s): four at its torso, then
/// at its head (a fresh zombie). Logs its health after each.
/// `PLATYPUS_BOW_TARGET`: another creature in its place.
fn bowshot_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut hand: ResMut<crate::hands::Hand>,
    items: Option<Res<crate::hands::items::Items>>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    player: Query<(&Kinematics, &crate::hands::items::Inventory), With<LocalPlayer>>,
    mut them: Query<(Entity, &crate::creatures::Creature, &mut Kinematics, &crate::creatures::Health), Without<LocalPlayer>>,
    mut state: Local<(u8, f32, Option<Vec2>, u8, u8)>,
    mut felt: MessageReader<crate::combat::Felt>,
) {
    if s.name != "bowshot" {
        return;
    }
    for f in felt.read() {
        if let Ok((_, c, zk, _)) = them.get(f.target)
            && c.kind != "dummy"
        {
            info!("bowshot: struck {:.0} (meant {:.0}) at {:+.1},{:+.1} from its middle", f.dealt, f.meant, f.at.x - zk.body.pos.x, f.at.y - zk.body.pos.y);
        }
    }
    menu.open = false;
    let t = s.elapsed;
    let Ok((k, inv)) = player.single() else { return };
    let (step, next, spot, shots, logged) = &mut *state;
    let spot = *spot.get_or_insert(k.body.pos + Vec2::new(70.0, 0.0));
    // The bow in hand.
    if let Some(i) = items.as_ref().and_then(|items| inv.slots.iter().position(|st| st.is_some_and(|st| items.def(st.item).id == "bow"))) {
        (hand.bar, hand.slot) = (i / crate::hands::items::HOTBAR, i % crate::hands::items::HOTBAR);
    }
    if *step == 0 && t > 0.5 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        crate::creatures::def::spawn_creature(&mut commands, &std::env::var("PLATYPUS_BOW_TARGET").unwrap_or_else(|_| "zombie".into()), spot, |_| {});
        *step = 1;
        *next = 1.5;
        return;
    }
    // Held where it stands.
    let mut target = None;
    for (_, c, mut zk, h) in &mut them {
        if c.kind == "dummy" || c.kind == "sandbag" || h.hp <= 0.0 {
            continue;
        }
        let zk = &mut *zk;
        zk.body.pos.x = spot.x;
        zk.body.vel.x = 0.0;
        let fall = zk.body.vel.y;
        zk.loco.knock(&mut zk.body, Vec2::new(0.0, fall), 0.1);
        let head = *shots >= 4;
        let up = std::env::var("PLATYPUS_HEAD_UP").ok().and_then(|v| v.parse().ok()).unwrap_or(-2.0);
        target = Some(zk.body.pos + if head { Vec2::new(0.0, zk.body.half.y + up) } else { Vec2::new(0.0, 6.0) });
    }
    let Some(at) = target else {
        // (Dead: a fresh one, for the headshots.)
        if *step == 1 && t > *next {
            crate::creatures::def::spawn_creature(&mut commands, &std::env::var("PLATYPUS_BOW_TARGET").unwrap_or_else(|_| "zombie".into()), spot, |_| {});
            *next = t + 1.0;
        }
        mouse.release(MouseButton::Left);
        return;
    };
    cursor.0 = Some(at);
    if t < *next {
        return;
    }
    let phase = (t - *next) % 1.1;
    if phase < 0.8 {
        mouse.press(MouseButton::Left);
    } else {
        if mouse.pressed(MouseButton::Left) {
            *shots += 1;
        }
        mouse.release(MouseButton::Left);
    }
    if *shots >= 9 {
        *step = 2;
    }
    let hp: Vec<String> = them.iter().filter(|(_, c, ..)| c.kind != "dummy" && c.kind != "sandbag").map(|(_, _, _, h)| format!("{:.0}", h.hp)).collect();
    // (Half a second after each, what it did.)
    if *logged < *shots && phase > 0.4 {
        *logged = *shots;
        info!("bowshot: t {t:.1} shot {} at the {}: zombie hp [{}]", shots, if *shots > 4 { "head" } else { "torso" }, hp.join(", "));
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
                crafting.spawn(&mut commands, kind, Vec2::new(900.0, floor as f32), false);
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
        hits.write(crate::combat::Hit { by: None, target: e, damage: 24.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at: k.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
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
        crate::creatures::body::elements::catch_fire(&mut commands, e, &h, None, &coatings, 1.0);
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
        hits.write(crate::combat::Hit { by: None, target: e, damage: 3.0, harm, knock: Vec2::ZERO, stun: 0.0, at: pk.body.pos + Vec2::new(90.0, 0.0), dir: Vec2::X, weight: 1.0, crit: false });
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
        hits.write(crate::combat::Hit { by: None, target, damage: 5.0, harm, knock: Vec2::ZERO, stun: 0.0, at, dir: Vec2::X, weight: 1.0, crit: false });
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
                        hits.write(crate::combat::Hit { by: None, target: te, damage: 90.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.5, at: pk.body.pos, dir: Vec2::X, weight: 2.0, crit: false });
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
/// is, and when it gets to the player (within 40 cells; at night), or that it
/// didn't in `PLATYPUS_SCENARIO_SECS`. `PLATYPUS_NONAV=1`: with no way
/// found, to compare.
fn course_script(
    mut commands: Commands,
    s: Res<Scenario>,
    ways: Res<crate::creatures::brain::way::Ways>,
    player: Query<(&Kinematics, &crate::creatures::Health), With<LocalPlayer>>,
    foes: Query<(&crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    mut state: Local<(u8, f32, String)>,
    mut day: ResMut<crate::light::Daylight>,
) {
    if s.name != "course" {
        return;
    }
    let t = s.elapsed;
    // (At night: a way's what's tested, and a cave spider keeps off a
    // player in the day.)
    if state.0 == 0 {
        day.skipped = (day.skipped + 23.0 - day.time * 24.0).rem_euclid(24.0);
    }
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
    // (Pictures on after it's dead: its body.)
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
        hits.write(crate::combat::Hit { by: None, target, damage: 3.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at: fk.body.pos, dir: Vec2::X, weight: 0.2, crit: false });
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
        // (Told where the player is: this is its digging, not its senses;
        // sealed in, it couldn't see it.)
        let at = Vec2::new(px as f32, floor as f32 + 12.0);
        crate::creatures::def::spawn_creature(&mut commands, "spider", Vec2::new(px as f32 - 200.0, floor as f32), move |e| {
            e.insert(crate::creatures::brain::senses::Alert::hunting(at));
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
    mut hand: ResMut<crate::hands::Hand>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut state: Local<(u8, usize, usize, f32)>,
) {
    if s.name != "pickarea" {
        return;
    }
    // (From a block at a time, so C turns the round bite on.)
    if state.0 == 0 {
        hand.area = false;
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

/// What the spider scenario keeps of its web: how many it's begun, the
/// move's phase and the strand's way last seen (each since when), when
/// the tell, the shot and the catch were, the dodge pressed for (the shot
/// or catch it was for), the shot it slipped, where (and when) a miss
/// landed, the pictures taken, a tally (shot, caught, reeled in, missed,
/// broken free), and whether it's been summed up.
#[derive(Default)]
struct WebWatch {
    webs: u32,
    phase: Option<String>,
    phased: f32,
    hold: Option<&'static str>,
    since: f32,
    tell: f32,
    shot: f32,
    caught: f32,
    dodge: (f32, f32),
    slipped: f32,
    landed: Option<(Vec2, f32)>,
    pictures: std::collections::HashSet<(&'static str, u32)>,
    tally: [u32; 5],
    summed: bool,
}

/// A spider's attacks against a player standing still.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn spider_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(Entity, &mut Kinematics, &mut crate::creatures::Health, Option<&crate::creatures::body::elements::Coated>, Has<crate::combat::Invulnerable>), With<LocalPlayer>>,
    spiders: Query<(&Kinematics, &crate::creatures::moves::Moves, Option<&crate::creatures::brain::senses::Alert>, &crate::creatures::Creature, Entity), Without<LocalPlayer>>,
    book: Res<crate::creatures::moves::MoveBook>,
    mut hits: MessageReader<crate::combat::Hit>,
    mut state: Local<(u8, f32, Option<String>)>,
    (mut web, strands, offscreen): (Local<WebWatch>, Query<&crate::creatures::moves::web::Strand>, Option<Res<crate::camera::Offscreen>>),
    (mut keys, mut mouse, mut cursor): (ResMut<ButtonInput<KeyCode>>, ResMut<ButtonInput<MouseButton>>, ResMut<CursorOverride>),
    // (The nearest spider's facing last frame, and its turns: logged a
    // second at a time, to see it doesn't flip round and round.)
    (mut turns, rigs): (Local<(f32, u32, u32)>, Query<&crate::creatures::body::legs::Legs>),
) {
    if s.name != "spider" {
        return;
    }
    if let Some((_, .., e)) = spiders.iter().filter(|x| x.3.kind == "spider").min_by(|x, y| x.0.body.pos.distance_squared(player.single().map_or(Vec2::ZERO, |p| p.1.body.pos)).total_cmp(&y.0.body.pos.distance_squared(player.single().map_or(Vec2::ZERO, |p| p.1.body.pos))))
        && let Ok(l) = rigs.get(e)
    {
        if turns.0 != 0.0 && l.facing() != turns.0 {
            turns.1 += 1;
            turns.2 += 1;
        }
        turns.0 = l.facing();
    }
    if (s.elapsed + 1e-3).fract() < 0.02 && turns.1 > 0 {
        info!("spider: t {:.0} turned round {} times this second ({} in all)", s.elapsed, turns.1, turns.2);
        turns.1 = 0;
    }
    let Ok((me, mut k, mut h, coat, untouchable)) = player.single_mut() else { return };
    let t = s.elapsed;
    // On open floor, the spider 180 cells off. (In a generated world, on
    // the ground where the player is, the spider 140 cells on, facing it.)
    if state.0 == 0 && t > 0.5 {
        // (`PLATYPUS_SPIDER_LAIR=1`, a generated world: the player put on
        // the floor of the nearest spider nest's cave, `PLATYPUS_SPIDER_DX`
        // cells off its middle, its own spider there: a fight as the world
        // has it.)
        if std::env::var("PLATYPUS_SPIDER_LAIR").is_ok() {
            let home = sim.generator.spawn_point();
            let lair = sim.generator.landmarks().into_iter().filter(|(_, n)| n == "spider nest").min_by_key(|(p, _)| (p.x - home.x).abs());
            if let Some((p, _)) = lair {
                let dx: i32 = std::env::var("PLATYPUS_SPIDER_DX").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
                let x = p.x + dx;
                let floor = (0..300).map(|d| p.y - d).find(|&y| sim.world.is_solid(CellPos::new(x, y)) && !sim.world.is_solid(CellPos::new(x, y + 1)));
                k.body.pos = Vec2::new(x as f32, floor.unwrap_or(p.y) as f32 + 1.0 + k.body.half.y);
                k.prev_pos = k.body.pos;
                info!("spider: the player put by the spider nest at ({}, {})", p.x, p.y);
            }
            state.0 = 1;
            return;
        }
        if std::env::var("PLATYPUS_WORLD").is_ok_and(|w| w == "arena") {
            let floor = platypus_worldgen::arena::FLOOR as f32;
            k.body.pos = Vec2::new(840.0, floor + 12.0);
            k.prev_pos = k.body.pos;
            // (`PLATYPUS_SPIDER_TUNNEL=1`: a winding tunnel of stone between
            // them, 34 high, its floor rising and falling: a tight cave, to
            // see the spider doesn't flip round and round in it.)
            if std::env::var("PLATYPUS_SPIDER_TUNNEL").is_ok()
                && let Some(stone) = sim.materials().id("stone")
            {
                let f = floor as i32;
                for x in 780..1080 {
                    let wave = ((x as f32 - 780.0) / 300.0 * std::f32::consts::TAU * 1.5).sin() * 14.0;
                    let (lo, hi) = (f + wave as i32, f + wave as i32 + 34);
                    for y in f..f + 90 {
                        let c = if y >= lo && y < hi { platypus_sim::Cell::AIR } else { platypus_sim::Cell::new(stone, (x * 13 + y * 7) as u8) };
                        sim.world.set(CellPos::new(x, y), c);
                    }
                }
            }
            // (Facing the player: facing away, it sees a share as far, and
            // whether it ever turned round was up to its wander.)
            crate::creatures::def::spawn_creature(&mut commands, "spider", Vec2::new(1020.0, floor), |e| {
                if let Some(mut kk) = e.get_mut::<Kinematics>() {
                    kk.loco.facing = -1.0;
                }
            });
        } else {
            let y0 = k.body.pos.y;
            let top = |x: f32| (0..400).map(|d| y0 as i32 + 200 - d).find(|&y| sim.world.is_solid(CellPos::new(x as i32, y))).map_or(y0, |y| y as f32 + 1.0);
            let (px, sx) = (k.body.pos.x, k.body.pos.x + 140.0);
            let (py, sy) = (top(px), top(sx));
            k.body.pos = Vec2::new(px, py + k.body.half.y + 0.5);
            k.prev_pos = k.body.pos;
            let feet = Vec2::new(sx, sy + 1.0);
            crate::creatures::def::spawn_creature(&mut commands, "spider", feet, |e| {
                if let Some(mut kk) = e.get_mut::<Kinematics>() {
                    kk.loco.facing = -1.0;
                }
            });
        }
        state.0 = 1;
    }
    for hit in hits.read() {
        if hit.target == me {
            info!("spider: t {t:.2} the player took {:.0} (knocked {:.0} cells/s)", hit.damage, hit.knock.length());
        }
    }
    // (The one nearest the player: the world has spiders of its own.)
    let nearest = spiders.iter().filter(|x| x.3.kind == "spider").min_by(|x, y| x.0.body.pos.distance(k.body.pos).total_cmp(&y.0.body.pos.distance(k.body.pos)));
    if let Some((sk, a, _, _, _)) = nearest {
        let now = a.doing(&book).map(|d| d.0.to_string());
        if now != state.2 {
            if let Some(what) = &now {
                info!("spider: t {t:.2} {what} from {:.0} cells", sk.body.pos.distance(k.body.pos));
            }
            state.2 = now;
        }
    }
    // Its web: the move phase by phase, its strand's way; the dodge
    // (`PLATYPUS_SPIDER_DODGE`), the pictures (`PLATYPUS_SPIDER_SHOTS`).
    let dodge = std::env::var("PLATYPUS_SPIDER_DODGE").unwrap_or_default();
    let mut want = std::collections::HashSet::new();
    let mut swing = None;
    // (`strike`: the longsword in hand, hotbar 2's slot 9.)
    if dodge == "strike" {
        if (0.3..0.4).contains(&t) {
            want.insert(KeyCode::KeyX);
        }
        if (0.5..0.6).contains(&t) {
            want.insert(KeyCode::Digit9);
        }
    }
    let mut picture = None;
    if let Some((sk, a, _, _, se)) = nearest {
        let off = sk.body.pos.distance(k.body.pos);
        let toward = if sk.body.pos.x > k.body.pos.x { KeyCode::KeyD } else { KeyCode::KeyA };
        let away = if toward == KeyCode::KeyD { KeyCode::KeyA } else { KeyCode::KeyD };
        let phase = a.doing(&book).filter(|d| d.0 == "spider_web").map(|d| d.2.to_string());
        if phase != web.phase {
            if let Some(p) = &phase {
                info!("spider: t {t:.2} web: {p} ({off:.0} cells off)");
                web.webs += (p == "windup") as u32;
                if p == "hold" {
                    web.tell = t;
                }
            }
            web.phase = phase.clone();
            web.phased = t;
        }
        use crate::creatures::moves::web::Hold;
        let strand = strands.iter().find(|st| st.by() == se);
        let hold = strand.map(|st| match st.hold {
            Hold::Out => "out",
            Hold::Caught(_) => "caught",
            Hold::Reeled(_) => "reeled in",
            Hold::Spent { missed: true } => "missed",
            Hold::Spent { missed: false } => "let go",
        });
        if hold != web.hold
            && let (Some(now), Some(st)) = (hold, strand)
        {
            let tip = st.tip.distance(k.body.pos);
            match now {
                "out" => {
                    web.tally[0] += 1;
                    web.shot = t;
                    info!("spider: t {t:.2} web: shot at the player {off:.0} cells off ({:.2} s after the tell began)", t - web.tell);
                }
                "caught" => {
                    web.tally[1] += 1;
                    web.caught = t;
                    info!("spider: t {t:.2} web: the player caught {:.0} cells from the spider, {:.2} s after the shot", st.root.distance(k.body.pos), t - web.shot);
                }
                "reeled in" => {
                    web.tally[2] += 1;
                    info!("spider: t {t:.2} web: the player reeled in to its jaws in {:.2} s ({off:.0} cells off)", t - web.caught);
                }
                "missed" => {
                    web.tally[3] += 1;
                    info!("spider: t {t:.2} web: missed: the strand's tip {tip:.0} cells from the player, {:.0} from the spider", st.tip.distance(sk.body.pos));
                    web.landed = Some((st.tip, t));
                }
                _ if web.hold == Some("caught") => {
                    web.tally[4] += 1;
                    info!("spider: t {t:.2} web: broken free ({}) after {:.2} s, {off:.0} cells off", st.freed.unwrap_or("out of time"), t - web.caught);
                }
                _ => info!("spider: t {t:.2} web: the strand let go"),
            }
            web.hold = Some(now);
            web.since = t;
        } else if hold.is_none() {
            web.hold = None;
        }
        // (Its tip by the player, the player untouchable: slipped.)
        if let Some(st) = strand.filter(|st| st.hold == Hold::Out && untouchable && ((st.tip - k.body.pos).abs() - k.body.half).max_element() < 4.0)
            && web.slipped != web.shot
        {
            web.slipped = web.shot;
            info!("spider: t {t:.2} web: the strand's tip passed the player while untouchable (dashing), {:.0} cells from the spider", st.tip.distance(sk.body.pos));
        }
        // (Where a miss landed, a moment on: the cobweb it left.)
        if let Some((at, when)) = web.landed
            && t - when > 0.3
        {
            web.landed = None;
            let webbed = sim.materials().id("cobweb").map_or(0, |cobweb| {
                let c = CellPos::from_world(at.x, at.y);
                (-5..=5).flat_map(|dx| (-5..=5).map(move |dy| c.offset(dx, dy))).filter(|p| sim.world.get(*p).is_some_and(|cell| cell.material == cobweb)).count()
            });
            info!("spider: t {t:.2} web: {webbed} cells of cobweb where it landed");
        }
        // The dodge, once a shot (or a catch): `dash` through the strand
        // as it comes (toward the spider), `jump` as it's shot, `run` away
        // from the tell on; caught, `breakdash` or `strike` at the spider,
        // or (`wall`) a stone wall put up between them.
        let fresh = |w: &WebWatch, of: f32| w.dodge.1 != of;
        match (dodge.as_str(), hold) {
            ("wall", Some("caught")) if t - web.caught > 0.1 && fresh(&web, web.caught) => {
                web.dodge = (t, web.caught);
                let mid = strand.map_or(k.body.pos, |st| (st.root + k.body.pos) / 2.0);
                let at = CellPos::from_world(mid.x, mid.y);
                commands.queue(move |world: &mut World| {
                    let mut sim = world.resource_mut::<SimWorld>();
                    if let Some(stone) = sim.materials().id("stone") {
                        sim.queue(WorldEdit::Paint { center: at, radius: 7, material: stone, overwrite: false });
                    }
                });
                info!("spider: t {t:.2} web: a stone wall put up between them");
            }
            ("dash", Some("out")) if strand.is_some_and(|st| st.tip.distance(k.body.pos) < 40.0) && fresh(&web, web.shot) => web.dodge = (t + 0.12, web.shot),
            ("jump", Some("out")) if fresh(&web, web.shot) => web.dodge = (t + 0.3, web.shot),
            ("run", _) if web.phase.as_deref() == Some("hold") && fresh(&web, web.tell) => web.dodge = (t + 1.2, web.tell),
            ("breakdash", Some("caught")) if t - web.caught > 0.15 && fresh(&web, web.caught) => web.dodge = (t + 0.1, web.caught),
            ("strike", Some("caught")) => swing = Some(sk.body.pos),
            _ => {}
        }
        if t < web.dodge.0 {
            if web.dodge.1 == web.shot && dodge == "dash" {
                want.insert(KeyCode::ShiftLeft);
                want.insert(toward);
            }
            match dodge.as_str() {
                "jump" => {
                    want.insert(KeyCode::Space);
                }
                "run" => {
                    want.insert(away);
                }
                "breakdash" => {
                    want.insert(KeyCode::ShiftLeft);
                }
                _ => {}
            }
        }
        // The pictures: the tell, the strand halfway, the player reeled in,
        // a miss, the bite (each web's first of each).
        let halfway = strand.is_some_and(|st| st.hold == Hold::Out && st.tip.distance(k.body.pos) < st.root.distance(k.body.pos) * 0.55);
        picture = match (web.phase.as_deref(), hold) {
            (Some("hold"), _) if t - web.tell > 0.25 => Some("tell"),
            (_, Some("out")) if halfway => Some("flight"),
            (_, Some("caught")) if t - web.caught > 0.2 => Some("reeled"),
            (_, Some("missed")) if t - web.since > 0.08 => Some("missed"),
            (Some("bite"), _) if t - web.phased > 0.06 => Some("bite"),
            _ => None,
        };
    }
    let nth = web.webs;
    if let (Some(what), Ok(dir)) = (picture, std::env::var("PLATYPUS_SPIDER_SHOTS"))
        && web.pictures.insert((what, nth))
    {
        let image = match &offscreen {
            Some(o) => Screenshot::image(o.0.clone()),
            None => Screenshot::primary_window(),
        };
        info!("spider: t {t:.2} web: a picture ({what})");
        commands.spawn(image).observe(save_to_disk(format!("{dir}/web{nth}_{what}.png")));
    }
    if !dodge.is_empty() {
        for key in [KeyCode::KeyX, KeyCode::Digit9, KeyCode::KeyA, KeyCode::KeyD, KeyCode::Space, KeyCode::ShiftLeft] {
            match (want.contains(&key), keys.pressed(key)) {
                (true, false) => keys.press(key),
                (false, true) => keys.release(key),
                _ => {}
            }
        }
    }
    if dodge == "strike" {
        cursor.0 = swing.or(Some(k.body.pos + Vec2::new(45.0, 0.0)));
        match (swing.is_some(), mouse.pressed(MouseButton::Left)) {
            (true, false) => mouse.press(MouseButton::Left),
            (false, true) => mouse.release(MouseButton::Left),
            _ => {}
        }
    }
    if t > s.duration - 0.2 && !web.summed {
        web.summed = true;
        let [shot, caught, reeled, missed, broke] = web.tally;
        info!("spider: web: {shot} shot, {caught} caught, {reeled} reeled in, {missed} missed, {broke} broken free");
    }
    if t > state.1 + 1.0 {
        state.1 = t;
        if let Some((sk, a, al, _, _)) = nearest {
            let al = al.map_or("no alert".to_string(), |a| format!("{:?} senses {} engaged {} flanking {} cowering {} lit {:.2}", a.wary, a.senses_it, a.engaged(), a.flanking, a.cowering, a.quarry_lit));
            info!("spider: t {t:.1} at {:.0} cells ({:.0},{:.0} to the player's {:.0},{:.0}), vel ({:.0},{:.0}), grounded {} clinging {:?}, doing {:?}, line {}; {al}", sk.body.pos.distance(k.body.pos), sk.body.pos.x, sk.body.pos.y, k.body.pos.x, k.body.pos.y, sk.body.vel.x, sk.body.vel.y, sk.loco.grounded(), sk.loco.clinging(), a.doing(&book), crate::creatures::moves::clear(&sim, sk.body.pos, k.body.pos));
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

/// `ridge` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a stone mountain
/// (a peak 110 up, `PLATYPUS_RIDGE_HIGH`; each slope's run 140,
/// `PLATYPUS_RIDGE_WIDE`: about 38°), the player held still halfway up
/// its left side (healed each second), a cave spider put down at the foot
/// of its right; a dent in the right slope between them, below the peak
/// (`PLATYPUS_DENT=w,h`: its width along the slope and depth into it;
/// 14,10; `0,0`: none; `PLATYPUS_DENT_KIND=cave`: a hole into the slope
/// sideways, `w` high, `h` in, rock over it). Logs twice a second where the spider is and how
/// it holds on, each move it starts, and its alertness (it's after the
/// player from the start, told where it is; `PLATYPUS_UNTOLD=1`: as one
/// that's just seen it, its own senses on it from there); at the end how
/// long it took to come within 40 cells, if it did.
#[allow(clippy::type_complexity)]
fn ridge_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    spiders: Query<(&Kinematics, &crate::creatures::moves::Moves, Option<&crate::creatures::brain::senses::Alert>), Without<LocalPlayer>>,
    book: Res<crate::creatures::moves::MoveBook>,
    mut state: Local<(u8, f32, Option<String>, Option<f32>, u32, Vec2)>,
) {
    if s.name != "ridge" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR;
    // (`PLATYPUS_RIDGE_HIGH`: the peak's height, 110; `PLATYPUS_RIDGE_WIDE`:
    // each slope's run, 140: about 38° at 110 high.)
    let env = |name: &str, or: i32| std::env::var(name).ok().and_then(|v| v.parse::<i32>().ok()).unwrap_or(or);
    let (high, wide) = (env("PLATYPUS_RIDGE_HIGH", 110), env("PLATYPUS_RIDGE_WIDE", 140));
    let peak = (1000, high);
    let ground = |x: i32| fl + (peak.1 - (x - peak.0).abs() * peak.1 / wide).max(0);
    if state.0 == 0 && t > 0.3 {
        let stone = sim.materials().id("stone").unwrap();
        for x in peak.0 - 150..peak.0 + 150 {
            for y in fl..ground(x) {
                sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: stone, overwrite: true });
            }
        }
        // (The dent: a pocket into the right slope, two thirds of the way up.)
        let (w, d) = std::env::var("PLATYPUS_DENT").ok().and_then(|v| v.split_once(',').and_then(|(a, b)| Some((a.parse::<i32>().ok()?, b.parse::<i32>().ok()?)))).unwrap_or((14, 10));
        let at = peak.0 + wide * 5 / 14;
        // (`PLATYPUS_DENT_KIND=cave`: a hole into the slope sideways instead,
        // `w` high and `h` in, rock over it: past it, a climber goes in, up
        // its back and along its roof upside down, and out over its lip.)
        let cave = std::env::var("PLATYPUS_DENT_KIND").is_ok_and(|v| v == "cave");
        let air = |x: i32, y: i32| WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: platypus_sim::MaterialId::AIR, overwrite: true };
        if cave {
            let mouth = ground(at);
            for x in at - d..=at + w {
                for y in mouth - w..mouth {
                    sim.queue(air(x, y));
                }
            }
        } else {
            for x in at - w / 2..at + w / 2 {
                for y in ground(x) - d..ground(x) {
                    sim.queue(air(x, y));
                }
            }
        }
        state.0 = 1;
        return;
    }
    if state.0 == 1 && t > 0.6 {
        let px = peak.0 - wide / 2;
        k.body.pos = Vec2::new(px as f32, ground(px) as f32 + k.body.half.y + 1.0);
        k.prev_pos = k.body.pos;
        state.5 = k.body.pos;
        // (Already after the player: told where it is all along;
        // `PLATYPUS_UNTOLD=1`: as one that's just seen it, its own senses
        // keeping it on it from there.)
        let at = k.body.pos;
        let told = std::env::var("PLATYPUS_UNTOLD").is_err();
        crate::creatures::def::spawn_creature(&mut commands, "spider", Vec2::new((peak.0 + wide + 25) as f32, fl as f32), move |e| {
            e.insert(if told { crate::creatures::brain::senses::Alert::hunting(at) } else { crate::creatures::brain::senses::Alert::spotted(at) });
        });
        state.0 = 2;
        state.1 = t;
    }
    if state.0 < 2 {
        return;
    }
    // (The player held where it stands: knocked about, it slid off down the
    // far side.)
    k.body.pos = state.5;
    k.prev_pos = state.5;
    k.body.vel = Vec2::ZERO;
    // (The one nearest the player: the world has spiders of its own.)
    let nearest = spiders.iter().min_by(|x, y| x.0.body.pos.distance(k.body.pos).total_cmp(&y.0.body.pos.distance(k.body.pos)));
    if let Some((sk, a, _)) = nearest {
        let now = a.doing(&book).map(|d| d.0.to_string());
        if now != state.2 {
            if let Some(what) = &now {
                info!("ridge: t {t:.2} {what} from {:.0} cells", sk.body.pos.distance(k.body.pos));
                state.4 += (what == "spider_spit") as u32;
            }
            state.2 = now;
        }
        if state.3.is_none() && sk.body.pos.distance(k.body.pos) < 40.0 {
            state.3 = Some(t);
            info!("ridge: t {t:.1} the spider's within 40 cells");
        }
    }
    if t > state.1 + 0.5 {
        state.1 = t;
        for (sk, _, alert) in &spiders {
            let al = alert.map_or("no alert".to_string(), |a| format!("{:?} senses {} engaged {} flanking {}", a.wary, a.senses_it, a.engaged(), a.flanking));
            info!("ridge: t {t:.1} at ({:.0},{:.0}) {:.0} off, vel ({:.0},{:.0}), grounded {} clinging {:?}; {al}", sk.body.pos.x, sk.body.pos.y, sk.body.pos.distance(k.body.pos), sk.body.vel.x, sk.body.vel.y, sk.loco.grounded(), sk.loco.clinging());
        }
        h.hp = h.max;
    }
    if t > s.duration - 0.05 && state.0 == 2 {
        state.0 = 3;
        match state.3 {
            Some(at) => info!("ridge: came within 40 cells at t {at:.1}; {} spits", state.4),
            None => info!("ridge: never came within 40 cells; {} spits", state.4),
        }
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
        // `PLATYPUS_LAKE=1`: on the shore of the nearest lake (its water
        // mirroring the sky).
        if std::env::var("PLATYPUS_LAKE").is_ok() {
            let found = (0..4000).flat_map(|i| [x0 + i * 40, x0 - i * 40]).find(|&x| (0..5).all(|d| sim.generator.water_hint(x + d * 30).is_some()));
            match found {
                Some(x) => {
                    // (Back from its edge, on the land.)
                    let shore = (1..400).map(|d| x - d * 4).find(|&sx| sim.generator.water_hint(sx).is_none()).unwrap_or(x) - 30;
                    let y = sim.generator.surface_hint(shore).unwrap_or(0) as f32 + k.body.half.y + 3.0;
                    k.body.pos = Vec2::new(shore as f32, y);
                    k.body.vel = Vec2::ZERO;
                    k.prev_pos = k.body.pos;
                    info!("backdrop: a lake at x {x}, standing at {shore}");
                }
                None => info!("backdrop: no lake found"),
            }
        }
        // `PLATYPUS_BORDER=1`: a little before the nearest border between
        // biomes on the right (the walk crosses it: the looks change over).
        if std::env::var("PLATYPUS_BORDER").is_ok() {
            let here = sim.generator.biome_hint(x0);
            match (1..4000).map(|i| x0 + i * 10).find(|&x| sim.generator.biome_hint(x) != here) {
                Some(x) => {
                    let at = x - 40;
                    let y = sim.generator.surface_hint(at).unwrap_or(0) as f32 + k.body.half.y + 3.0;
                    k.body.pos = Vec2::new(at as f32, y);
                    k.body.vel = Vec2::ZERO;
                    k.prev_pos = k.body.pos;
                    info!("backdrop: border {:?} -> {:?} at x {x}", here, sim.generator.biome_hint(x));
                }
                None => info!("backdrop: no border found"),
            }
        }
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
    // (A walk right from 4 s: the layers slide at their own speeds. Not by
    // a lake: it'd walk in.)
    if s.elapsed > 4.0 && std::env::var("PLATYPUS_LAKE").is_err() && !keys.pressed(KeyCode::KeyD) {
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
    const BIOMES: [&str; 9] = ["plains", "forest", "desert", "tundra", "jungle", "swamp", "deep forest", "mountains", "ocean"];
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
            // (The ocean's at the world's ends: inland of its shore, where
            // what spawns just off screen is on the beach; not its "middle"
            // off the edge.)
            let middle = if biome == "ocean" { found.map(|x| (x + if x > start { -560 } else { 560 }, 600)) } else { None }.or_else(|| found.map(|x| {
                let (mut lo, mut hi) = (x, x);
                while lo > x - 20000 && sim.generator.biome_hint(lo - 10) == Some(biome) {
                    lo -= 10;
                }
                while hi < x + 20000 && sim.generator.biome_hint(hi + 10) == Some(biome) {
                    hi += 10;
                }
                ((lo + hi) / 2, hi - lo)
            }));
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
                // (Water life: in or by the seed's lakes and oceans, never
                // elsewhere.)
                let lake = |x: i32| sim.generator.water_hint(x);
                let (mut good, mut bad) = (0, Vec::new());
                for (c, ck) in them.iter().filter(|(_, ck)| ck.body.pos.distance(at) < 700.0) {
                    let (x, y) = (ck.body.pos.x as i32, ck.body.pos.y as i32);
                    let ok = match c.kind.as_str() {
                        "fish" => lake(x).is_some_and(|l| y < l + 4),
                        "crab" => (-60..=60).step_by(3).any(|dx| lake(x + dx).is_some_and(|l| (l - y).abs() < 50)),
                        _ => continue,
                    };
                    if ok {
                        good += 1;
                    } else {
                        bad.push(format!("{} at ({x}, {y})", c.kind));
                    }
                }
                info!("safari: the {biome}: water life at lakes and seas {good}, elsewhere {}: {bad:?}", bad.len());
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
        for (kind, x) in [("centipede", 900.0), ("strider", 1250.0), ("tyrant", 1600.0)] {
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
                "tyrant" => {
                    // (Its head: its picture's furthest out the way it
                    // faces, a little back; far past its box.)
                    if let Some(b) = &parts.body {
                        let f = if b.flip { -1.0 } else { 1.0 };
                        let snout = (0..120).rev().map(|i| i as f32).find(|&x| (-40..40).any(|y| b.covers(b.grip + Vec2::new(x * f, y as f32), 0.0)));
                        if let Some(x) = snout {
                            let y = (-40..40).map(|y| y as f32).filter(|&y| b.covers(b.grip + Vec2::new((x - 8.0) * f, y), 0.0)).sum::<f32>() / (-40..40).filter(|&y| b.covers(b.grip + Vec2::new((x - 8.0) * f, y as f32), 0.0)).count().max(1) as f32;
                            aims.push((b.grip + Vec2::new((x - 8.0) * f, y), "its head".into(), e));
                        }
                    }
                    if let Some(p) = mid(PartKind::Chain, false) {
                        aims.push((p, "its tail".into(), e));
                    }
                }
                _ => {}
            }
        }
        for (i, (at, what, e)) in aims.into_iter().enumerate() {
            let inside = them.get(e).is_ok_and(|(_, _, k, _)| ((at - k.body.pos).abs() - k.body.half).max_element() <= 0.0);
            let what = if inside { what } else { format!("{what} (outside its box)") };
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

/// `reach` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): every creature
/// that walks on legs (`legs.rs`) put down in a row, held still; at 3 s,
/// for each, how much of its body as drawn lies outside its box and every
/// limb: what no blow could land on before its picture was struck too.
fn reach_script(
    mut commands: Commands,
    s: Res<Scenario>,
    them: Query<(&crate::creatures::Creature, &Kinematics, Option<&crate::creatures::body::parts::Parts>), Without<LocalPlayer>>,
    mut state: Local<u8>,
) {
    if s.name != "reach" {
        return;
    }
    use crate::creatures::body::parts::PartKind;
    const KINDS: [&str; 13] = ["centipede", "crab", "fallen_templar", "skeleton", "scorpion", "raptor", "spiderling", "spider", "tyrant", "strider", "zombie_crimson", "stalker", "zombie"];
    let fl = platypus_worldgen::arena::FLOOR as f32;
    if *state == 0 && s.elapsed > 0.5 {
        for (i, kind) in KINDS.iter().enumerate() {
            crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(260.0 + i as f32 * 130.0, fl + 2.0), |e| {
                e.insert(crate::creatures::brain::Staged);
            });
        }
        *state = 1;
    }
    if *state == 1 && s.elapsed > 3.0 {
        *state = 2;
        for (c, k, parts) in &them {
            let Some(parts) = parts else { continue };
            let Some(b) = &parts.body else {
                info!("reach: {}: no body of its legs' (drawn by its own art)", c.kind);
                continue;
            };
            let (mut all, mut out) = (0, 0);
            let r = b.turned.reach() as i32;
            for y in -r..=r {
                for x in -r..=r {
                    let p = b.grip + Vec2::new(x as f32, y as f32);
                    if !b.covers(p, 0.0) {
                        continue;
                    }
                    all += 1;
                    let in_box = ((p - k.body.pos).abs() - k.body.half).max_element() <= 0.0;
                    let on_limb = parts.list.iter().any(|q| q.kind != PartKind::Body && q.gap(p) <= 0.0);
                    out += (!in_box && !on_limb) as u32;
                }
            }
            info!("reach: {}: {out} of its body's {all} pixels outside its box and limbs ({:.0}%)", c.kind, out as f32 * 100.0 / all.max(1) as f32);
        }
    }
}

/// `PLATYPUS_SHOTS=dir` (any scenario but `legs` and `pack`, which have their own): a
/// picture every `PLATYPUS_SHOT_EVERY` s (2) into it, `<scenario>_<t in
/// hundredths>.png`.
fn shots_script(mut commands: Commands, s: Res<Scenario>, offscreen: Option<Res<crate::camera::Offscreen>>, mut next: Local<f32>) {
    let Ok(dir) = std::env::var("PLATYPUS_SHOTS") else { return };
    if s.name == "legs" || s.name == "pack" || s.elapsed < *next {
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
///   over its beat to jump.
///
/// (Each gap 28 wide, a faint glow at its edges, a step down in it: a
/// missed jump lands there, and it's the way back up from the floor.)
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
        day.skipped = (day.skipped + 23.0 - day.time * 24.0).rem_euclid(24.0);
        let (Some(stone), Some(bedrock)) = (sim.materials().id("stone"), sim.materials().id("bedrock")) else { return };
        // Stone, a row of bedrock through its middle: a slab in the air
        // stays up when a blast shakes it (stone joined to bedrock is
        // ground; a slab on its own falls once anything near it breaks).
        let mut fill = |x0: i32, x1: i32, y0: i32, y1: i32| {
            for x in (x0 + 2..=x1 - 2).step_by(3) {
                for y in (y0 + 2..=y1 - 2).step_by(3) {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 2, material: stone, overwrite: true });
                }
            }
            for x in (x0 + 2..=x1 - 2).step_by(2) {
                sim.queue(WorldEdit::Paint { center: CellPos::new(x, (y0 + y1) / 2), radius: 1, material: bedrock, overwrite: true });
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
        fill(928, 1190, floor + 56, floor + 62);
        fill(1218, 1420, floor + 56, floor + 62);
        fill(1436, 1460, floor + 30, floor + 35);
        // A step down in each gap (a missed jump lands on it, and the way
        // back up from the floor), and a faint glow at each gap's edges:
        // too dim to give you away, enough to see where to jump.
        for (a, b) in [(900, 928), (1190, 1218)] {
            fill(a + 2, b - 2, floor + 28, floor + 33);
            for x in [a, b] {
                commands.spawn((Name::new("Edge glow"), crate::light::LightSource { color: [0.05, 0.13, 0.2], flicker: 0.0 }, Transform::from_xyz(x as f32, fl + 66.0, 0.0)));
            }
        }
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

/// `tactics` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`; BE `behaviour`
/// stage 3): each tactic in turn, at night, a room roofed over at the left
/// (x 300..700, its roof 60 up, walled at its right end):
/// - flee: an orc on the player, cut to 15 % of its health at 2 s, runs;
///   the player follows it to the arena's wall: cornered, it fights;
/// - ambush: a cave centipede in the room goes up to its ceiling and waits;
///   the player walks in at 24 s: it comes along the ceiling and drops;
/// - flank and skirmish: a pack of four raptors at the player held still:
///   where each is, twice a second (two each side, waiting 52 off, one at
///   a time coming in to bite);
/// - call: a raptor pack, its leader in sight of the player, the others
///   too far off to see: they come at its call;
/// - shun the light: in the room, the player with a torch, a cave spider:
///   it keeps off; the torch out at 53 s, it comes.
///
/// Logs each part's outcome and every change of how wary each is, of its
/// running, of its hold on a ceiling.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn tactics_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut day: ResMut<crate::light::Daylight>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut toggles: ResMut<crate::light::LightToggles>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    mut them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::brain::senses::Alert>, &mut crate::creatures::Health), Without<LocalPlayer>>,
    mut hits: MessageReader<crate::combat::Hit>,
    // (part, when it began, what each was last: wary, running, on a ceiling)
    mut state: Local<(u8, f32, std::collections::HashMap<Entity, (crate::creatures::brain::senses::Wary, bool, bool)>)>,
    mut next_log: Local<f32>,
) {
    use crate::creatures::brain::tactics::Pack;
    if s.name != "tactics" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let fl = floor as f32;
    let put = |k: &mut Kinematics, x: f32| {
        k.body.pos = Vec2::new(x, fl + k.body.half.y);
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
    };
    let me = k.body.pos;
    for hit in hits.read() {
        if them.get(hit.target).is_err() && hit.damage > 0.0 {
            info!("tactics: t {t:.2} the player was struck ({:.0})", hit.damage);
        }
    }
    // Each one's changes.
    for (e, c, ck, a, _) in &them {
        let Some(a) = a else { continue };
        let now = (a.wary, a.fleeing, ck.loco.clinging() == Some(Vec2::Y));
        if let Some(was) = state.2.insert(e, now)
            && was != now
        {
            let what = if was.0 != now.0 {
                format!("{:?}", now.0)
            } else if was.1 != now.1 {
                (if now.1 { "running" } else { "stops running" }).to_string()
            } else {
                (if now.2 { "on the ceiling" } else { "off the ceiling" }).to_string()
            };
            info!("tactics: t {t:.2} the {} at x {:.0} ({:.0} up): {what} (the player at x {:.0})", c.kind, ck.body.pos.x, ck.body.pos.y - fl - ck.body.half.y, me.x);
        }
    }
    let rid = |commands: &mut Commands, them: &Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::brain::senses::Alert>, &mut crate::creatures::Health), Without<LocalPlayer>>| {
        for (e, ..) in them.iter() {
            commands.entity(e).despawn();
        }
    };
    // (Each put down facing the player, to its left: what it sees ahead of
    // it, it sees.)
    let spawn = |commands: &mut Commands, kind: &str, x: f32, told: Option<Vec2>, pack: Option<(u64, u32)>| {
        crate::creatures::def::spawn_creature(commands, kind, Vec2::new(x, fl), move |e| {
            if let Some(mut k) = e.get_mut::<Kinematics>() {
                k.loco.facing = -1.0;
            }
            if let Some(at) = told {
                e.insert(crate::creatures::brain::senses::Alert::hunting(at));
            }
            if let Some((id, rank)) = pack {
                e.insert(Pack { id, rank });
            }
        });
    };
    let first = |kind: &str| them.iter().find(|q| q.1.kind == kind).map(|q| (q.2.body.pos, q.2.body.half));
    // (Once a second, or twice for the flank.)
    let every = if state.0 == 5 { 0.5 } else { 1.0 };
    let tell = t >= *next_log;
    if tell {
        *next_log = (t / every).floor() * every + every;
    }
    match state.0 {
        0 if t > 0.5 => {
            day.skipped = (day.skipped + 23.0 - day.time * 24.0).rem_euclid(24.0);
            if let (Some(stone), Some(bedrock)) = (sim.materials().id("stone"), sim.materials().id("bedrock")) {
                for x in (300..=700).step_by(3) {
                    for y in [floor + 62, floor + 67] {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 2, material: if y == floor + 62 { stone } else { bedrock }, overwrite: true });
                    }
                }
                for y in (floor + 1..floor + 66).step_by(3) {
                    for x in [701, 705] {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 2, material: stone, overwrite: true });
                    }
                }
            }
            rid(&mut commands, &them);
            put(&mut k, 1200.0);
            spawn(&mut commands, "orc", 1300.0, Some(Vec2::new(1200.0, fl + 12.0)), None);
            *state = (1, t, default());
        }
        // Flee.
        1 if t > 2.0 => {
            for (_, c, _, _, mut ch) in &mut them {
                if c.kind == "orc" {
                    ch.hp = ch.max * 0.15;
                }
            }
            info!("tactics: t {t:.2} the orc cut to 15 % of its health");
            state.0 = 2;
        }
        2 | 3 => {
            let orc = first("orc");
            if state.0 == 2 && t > 6.0 {
                state.0 = 3;
            }
            // (Following it, from 6 s, to 40 cells off.)
            if state.0 == 3 && orc.is_some_and(|(o, _)| o.x - me.x > 40.0) {
                keys.press(KeyCode::KeyD);
            } else {
                keys.release(KeyCode::KeyD);
            }
            if tell && let Some((o, _)) = orc {
                info!("tactics: t {t:.1} flee: the orc at x {:.0}, {:.0} from the player", o.x, o.x - me.x);
            }
            if t > 16.0 {
                keys.release(KeyCode::KeyD);
                rid(&mut commands, &them);
                put(&mut k, 150.0);
                spawn(&mut commands, "centipede", 600.0, None, None);
                state.0 = 4;
                state.1 = t;
            }
        }
        // Ambush: it goes up; the player walks in at 24 s.
        4 => {
            let walk = t > 24.0 && me.x < 640.0;
            if walk {
                keys.press(KeyCode::KeyD);
            } else {
                keys.release(KeyCode::KeyD);
            }
            if t > 32.0 {
                keys.release(KeyCode::KeyD);
                rid(&mut commands, &them);
                put(&mut k, 960.0);
                for r in 0..4 {
                    spawn(&mut commands, "raptor", 1250.0 + r as f32 * 24.0, None, Some((0xF1A4, r)));
                }
                state.0 = 5;
                state.1 = t;
            }
        }
        // Flank: who's which side (the player held where it stands: its
        // place, not how it's knocked about, is what's measured).
        5 => {
            put(&mut k, 960.0);
            if tell {
                let xs: Vec<f32> = them.iter().filter(|q| q.1.kind == "raptor").map(|q| q.2.body.pos.x - me.x).collect();
                let (l, r) = (xs.iter().filter(|x| **x < 0.0).count(), xs.iter().filter(|x| **x > 0.0).count());
                info!("tactics: t {t:.1} flank: {l} left of the player, {r} right ({:?}; the player at x {:.0})", xs.iter().map(|x| x.round() as i32).collect::<Vec<_>>(), me.x);
            }
            if t > 42.0 {
                rid(&mut commands, &them);
                put(&mut k, 960.0);
                // (The leader sees the player; the rest are too far off.)
                spawn(&mut commands, "raptor", 1220.0, None, Some((0xCA11, 0)));
                spawn(&mut commands, "raptor", 1560.0, None, Some((0xCA11, 1)));
                spawn(&mut commands, "raptor", 1590.0, None, Some((0xCA11, 2)));
                state.0 = 6;
                state.1 = t;
            }
        }
        // Call: the far two come.
        6 => {
            if tell {
                let xs: Vec<i32> = them.iter().filter(|q| q.1.kind == "raptor").map(|q| (q.2.body.pos.x - me.x).round() as i32).collect();
                info!("tactics: t {t:.1} call: the raptors from the player {xs:?} (the player at x {:.0})", me.x);
            }
            if t > 50.0 {
                rid(&mut commands, &them);
                put(&mut k, 400.0);
                toggles.carry = crate::light::Carry::Torch;
                spawn(&mut commands, "spider", 640.0, None, None);
                state.0 = 7;
                state.1 = t;
            }
        }
        // Shun the light: the torch out at 55 s.
        7 => {
            if t > 55.0 && toggles.carry == crate::light::Carry::Torch {
                toggles.carry = crate::light::Carry::Nothing;
                info!("tactics: t {t:.2} the torch out");
            }
            if tell
                && let Some((p, _)) = first("spider")
            {
                info!("tactics: t {t:.1} shun: the spider {:.0} from the player ({})", p.distance(me), if toggles.carry == crate::light::Carry::Torch { "torch lit" } else { "dark" });
            }
            if t > 60.0 {
                state.0 = 8;
            }
        }
        _ => {}
    }
}

/// `factions` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`; BE `behaviour`
/// stage 4): the player at x 900, watching unhunted (`Hiding`, as a
/// villager at home: nothing goes for it; kept whole), three fights in
/// turn just off to its right:
/// - war: three orcs and three skeletons, 280 cells apart, facing each
///   other (orcs and the dead hunt each other);
/// - prey, at night: a cave spider and two vampire bats (spiders take
///   bats; struck, a bat fights back);
/// - the hunt: a pack of three raptors and two orcs (raptors run down orcs;
///   orcs fight back).
///
/// Logs every blow between them as it lands (who took it, how much), each
/// death, and each fight's end: who's left, every two seconds who's alive.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn factions_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut day: ResMut<crate::light::Daylight>,
    mut player: Query<(Entity, &mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, &crate::creatures::Health), Without<LocalPlayer>>,
    mut felt: MessageReader<crate::combat::Felt>,
    // (fight, when it ends, who was alive last time)
    mut state: Local<(u8, f32, Vec<(Entity, String)>)>,
    // (when to log who's alive next; deaths not told till, while a fight's
    // cleared away)
    mut times: Local<(f32, f32)>,
) {
    use crate::creatures::brain::tactics::Pack;
    let (next_log, quiet) = &mut *times;
    if s.name != "factions" {
        return;
    }
    let Ok((me, mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    if state.0 == 0 {
        commands.entity(me).insert(crate::creatures::brain::villager::Hiding);
    }
    for f in felt.read() {
        if let Ok((_, c, ..)) = them.get(f.target)
            && f.dealt > 0.0
        {
            info!("factions: t {t:.2} the {} took {:.0}", c.kind, f.dealt);
        }
    }
    // Deaths: who was alive and isn't.
    let alive: Vec<(Entity, String)> = them.iter().filter(|q| q.1.kind != "dummy" && q.1.kind != "sandbag" && q.3.hp > 0.0).map(|q| (q.0, q.1.kind.clone())).collect();
    for (e, kind) in &state.2 {
        if !alive.iter().any(|a| a.0 == *e) && t > *quiet {
            info!("factions: t {t:.2} a {kind} is dead");
        }
    }
    state.2 = alive.clone();
    let count = |kind: &str| alive.iter().filter(|a| a.1 == kind).count();
    let spawn = |commands: &mut Commands, kind: &str, x: f32, facing: f32, pack: Option<(u64, u32)>| {
        crate::creatures::def::spawn_creature(commands, kind, Vec2::new(x, fl), move |e| {
            if let Some(mut k) = e.get_mut::<Kinematics>() {
                k.loco.facing = facing;
            }
            if let Some((id, rank)) = pack {
                e.insert(Pack { id, rank });
            }
        });
    };
    let rid = |commands: &mut Commands| {
        for (e, ..) in them.iter() {
            commands.entity(e).despawn();
        }
    };
    let tell = t >= *next_log;
    if tell {
        *next_log = t.floor() + 2.0;
    }
    // (Each fight: its kinds, and the time it may take.)
    const FIGHTS: [(&[&str], f32); 3] = [(&["orc", "skeleton"], 40.0), (&["spider", "vampire_bat"], 25.0), (&["raptor", "orc"], 30.0)];
    match state.0 {
        0 | 2 | 4 if t > 0.5 && t > state.1 => {
            rid(&mut commands);
            *quiet = t + 0.2;
            k.body.pos = Vec2::new(900.0, fl + k.body.half.y);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            match state.0 {
                0 => {
                    for i in 0..3 {
                        spawn(&mut commands, "orc", 1200.0 + i as f32 * 30.0, 1.0, None);
                        spawn(&mut commands, "skeleton", 1480.0 + i as f32 * 30.0, -1.0, None);
                    }
                }
                2 => {
                    // (At night: a cave spider keeps out of the day.)
                    day.skipped = (day.skipped + 23.0 - day.time * 24.0).rem_euclid(24.0);
                    spawn(&mut commands, "spider", 1100.0, 1.0, None);
                    spawn(&mut commands, "vampire_bat", 1250.0, -1.0, None);
                    spawn(&mut commands, "vampire_bat", 1290.0, -1.0, None);
                }
                _ => {
                    for i in 0..3 {
                        spawn(&mut commands, "raptor", 1550.0 + i as f32 * 26.0, -1.0, Some((0xFAC7, i)));
                    }
                    spawn(&mut commands, "orc", 1250.0, 1.0, None);
                    spawn(&mut commands, "orc", 1280.0, 1.0, None);
                }
            }
            let (kinds, secs) = FIGHTS[state.0 as usize / 2];
            info!("factions: t {t:.2} the fight begins: {}", kinds.join(" against "));
            state.1 = t + secs;
            state.0 += 1;
        }
        1 | 3 | 5 => {
            let (kinds, _) = FIGHTS[state.0 as usize / 2];
            let left: Vec<String> = kinds.iter().map(|k| format!("{} {}", count(k), k)).collect();
            if tell {
                info!("factions: t {t:.1} alive: {}", left.join(", "));
            }
            let over = kinds.iter().filter(|k| count(k) > 0).count() < 2;
            if over || t > state.1 {
                info!("factions: t {t:.2} the fight's over{}: {} left", if over { "" } else { " (time)" }, left.join(", "));
                state.1 = t + 1.0;
                state.0 += 1;
            }
        }
        _ => {}
    }
}

/// `burrow` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): the player in a
/// room inside a mound (x 1000..1240, 120 high; the room x 1120..1180, 30
/// high), seen through a slit (6 high, out through the mound's left side:
/// the player fits through nothing, a cave spider less); a cave spider put
/// down outside at x 850, told where the player is. It can't get nearer:
/// it burrows, its front legs at the face. The mound `PLATYPUS_WALL`: dirt
/// (claws), stone (acid), brick (fortified: it doesn't dig). Logs where
/// the spider is every second, how much of the mound's gone, and when (if)
/// it reaches the player. `PLATYPUS_LAMP=1`: the player's small flashlight
/// on, aimed out down the slit (the spider stops at its light, backs off and
/// rears). `PLATYPUS_BEAM=1`: no spider; at night, the
/// player outside the mound with its big flashlight aimed up across the
/// mound's face (how the beam's edges look on it).
#[allow(clippy::too_many_arguments)]
fn burrow_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    (mut day, mut toggles, mut cursor): (ResMut<crate::light::Daylight>, ResMut<crate::light::LightToggles>, ResMut<CursorOverride>),
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    mut state: Local<(u8, f32)>,
    mut hits: MessageReader<crate::combat::Hit>,
) {
    if s.name != "burrow" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let struck = hits.read().filter(|hit| hit.damage > 0.0).count();
    if struck > 0 && state.0 >= 2 {
        info!("burrow: t {:.2} the player was struck", s.elapsed);
    }
    h.hp = h.max;
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let fl = floor as f32;
    let wall = std::env::var("PLATYPUS_WALL").unwrap_or_else(|_| "dirt".into());
    let (x0, x1, top) = (1000, 1240, floor + 120);
    let mound = |sim: &SimWorld| {
        let Some(m) = sim.materials().id(&wall) else { return 0 };
        (x0..x1).flat_map(|x| (floor..top).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).count()
    };
    match state.0 {
        0 if t > 0.5 => {
            let Some(m) = sim.materials().id(&wall) else {
                warn!("burrow: no material `{wall}`");
                state.0 = 9;
                return;
            };
            for x in (x0..x1).step_by(3) {
                for y in (floor..top).step_by(3) {
                    let inside = (1120..1180).contains(&x) && (floor..floor + 30).contains(&y) || (x0..1120).contains(&x) && (floor + 18..floor + 24).contains(&y);
                    if !inside {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x + 1, y + 1), radius: 2, material: m, overwrite: true });
                    }
                }
            }
            // (The room and the slit kept open: painting round them spills.)
            for x in x0..1180 {
                for y in floor..floor + 30 {
                    let inside = (1120..1180).contains(&x) && y < floor + 30 || (floor + 18..floor + 24).contains(&y);
                    if inside {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: platypus_sim::MaterialId::AIR, overwrite: true });
                    }
                }
            }
            // (No training dummies.)
            for (e, c, _) in &them {
                if c.kind == "dummy" || c.kind == "sandbag" {
                    commands.entity(e).despawn();
                }
            }
            k.body.pos = Vec2::new(1160.0, fl + k.body.half.y);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            // (`PLATYPUS_BEAM=1`: no spider; at night, the big flashlight on,
            // aimed up across the mound's face: how its edges look.)
            if std::env::var("PLATYPUS_BEAM").is_ok() {
                day.skipped = (day.skipped + 23.0 - day.time * 24.0).rem_euclid(24.0);
                toggles.carry = crate::light::Carry::BigBeam;
                k.body.pos = Vec2::new(880.0, fl + k.body.half.y);
                k.prev_pos = k.body.pos;
                cursor.0 = Some(Vec2::new(1100.0, fl + 80.0));
                state.0 = 9;
                return;
            }
            // (`PLATYPUS_LAMP=1`: the player's small flashlight on, aimed
            // out down the slit: the spider keeps off its light.)
            if std::env::var("PLATYPUS_LAMP").is_ok() {
                toggles.carry = crate::light::Carry::SmallBeam;
                cursor.0 = Some(Vec2::new(900.0, fl + 20.0));
            }
            *state = (1, t);
        }
        1 if t > 1.2 => {
            info!("burrow: a {wall} mound of {} cells; the spider put down", mound(&sim));
            let at = k.body.pos;
            crate::creatures::def::spawn_creature(&mut commands, "spider", Vec2::new(850.0, fl), move |e| {
                e.insert(crate::creatures::brain::senses::Alert::hunting(at));
            });
            *state = (2, t);
        }
        2 => {
            k.body.pos = Vec2::new(1160.0, fl + k.body.half.y);
            k.body.vel = Vec2::ZERO;
            // (`PLATYPUS_LAMP_OFF=s`: the light out then.)
            if let Some(off) = std::env::var("PLATYPUS_LAMP_OFF").ok().and_then(|v| v.parse::<f32>().ok())
                && t >= off
                && toggles.carry != crate::light::Carry::Nothing
            {
                toggles.carry = crate::light::Carry::Nothing;
                info!("burrow: t {t:.1} the light out");
            }
            let spider = them.iter().find(|q| q.1.kind == "spider").map(|q| q.2.body.pos);
            if t >= state.1 + 1.0 {
                state.1 = t.floor() + 1.0;
                if let Some(p) = spider {
                    info!("burrow: t {t:.0} the spider at ({:.0}, {:.0}), {:.0} from the player; the mound {} cells", p.x, p.y - fl, p.distance(k.body.pos), mound(&sim));
                }
            }
            // (Reached: it's bitten the player, or it's within a cave
            // spider's `keep` across, a leg's length, where it strikes from,
            // allowing for a heap of its own dirt under it.)
            if struck > 0 || spider.is_some_and(|p| p.distance(k.body.pos) <= 40.0) {
                info!("burrow: t {t:.1} the spider reached the player");
                state.0 = 3;
            }
        }
        _ => {}
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
                    sim.queue(WorldEdit::PlaceBlock { block: CellPos::new(bx, by), material: stone, back: false, most: u32::MAX });
                }
            }
            for bx in 942 / b..1092 / b {
                for by in (floor + 100) / b..(floor + 112) / b {
                    sim.queue(WorldEdit::PlaceBlock { block: CellPos::new(bx, by), material: stone, back: false, most: u32::MAX });
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
                        sim.queue(WorldEdit::PlaceBlock { block: CellPos::new(bx, by), material: stone, back: true, most: u32::MAX });
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

/// `dread` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a dread knight
/// at 1120; the player (who can't die) stands at 900 (`PLATYPUS_DREAD=near`:
/// at 1085, for the sweep; `kill`: it's struck with light till it dies, and
/// leaves its cloak; `dash`: drawn in by its void, the player dashes;
/// `back`: while it pulls, the player walks away; `parry`: near it, the
/// player swings and swings at it).
/// Logs what it does, where the player is, the player's health lost.
fn dread_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, &crate::creatures::Health, Option<&crate::creatures::custom::dread::Dread>), Without<LocalPlayer>>,
    drops: Query<&crate::hands::Dropped>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, f32, f32, &'static str)>,
    (tugged, mut keys, player_e): (Query<(), (With<LocalPlayer>, With<crate::creatures::Tugged>)>, ResMut<ButtonInput<KeyCode>>, Query<Entity, With<LocalPlayer>>),
    (mut mouse, mut cursor): (ResMut<ButtonInput<MouseButton>>, ResMut<CursorOverride>),
) {
    if s.name != "dread" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let mode = std::env::var("PLATYPUS_DREAD").unwrap_or_default();
    state.2 += h.max - h.hp;
    h.hp = h.max;
    if state.0 == 0 && t > 0.5 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        k.body.pos = Vec2::new(if matches!(mode.as_str(), "near" | "parry") { 1075.0 } else if mode == "bat" { 600.0 } else { 900.0 }, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        crate::creatures::def::spawn_creature(&mut commands, "dread_knight", Vec2::new(1120.0, fl), |_| {});
        info!("dread: a dread knight at 1120 ({mode}); the player is {:?}", player_e.single().ok());
        state.0 = 1;
        return;
    }
    let Some((e, _, dk, dh, d)) = them.iter().find(|x| x.1.kind == "dread_knight") else {
        if state.0 == 1 && t > 2.0 {
            state.0 = 2;
            state.1 = t;
        }
        if state.0 == 2 && t > state.1 + 1.0 {
            info!("dread: t {t:.1} dead; things dropped about: {}", drops.iter().count());
            state.0 = 9;
        }
        return;
    };
    let doing = d.map_or("?", |d| d.doing());
    if doing != state.3 {
        info!("dread: t {t:.2} {doing} at {:.0}; the player at {:.0}, lost {:.0}", dk.body.pos.x, k.body.pos.x, state.2);
        state.3 = doing;
    }
    // `bat`: the ball, struck where it lies, batted along.
    if mode == "bat" {
        if let Some(d) = d {
            let b = d.ball();
            cursor.0 = Some(b);
            // (Put beside it, on the near side, once, at 2.5 s.)
            if (2.5..2.52).contains(&t) {
                k.body.pos.x = b.x - 22.0;
                k.prev_pos = k.body.pos;
            }
            if t > 2.6 && (t * 6.0) as i32 % 2 == 0 && (b.x - k.body.pos.x).abs() < 30.0 { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
            if (t * 2.0).fract() < 0.02 {
                info!("dread: t {t:.1} the ball at ({:.0}, {:.0}), the player at {:.0}", b.x, b.y - fl, k.body.pos.x);
            }
        }
    }
    // `parry`: the player (in its sweep's reach) swings and swings at it.
    if mode == "parry" {
        cursor.0 = Some(dk.body.pos - Vec2::Y * 20.0);
        if (t * 6.0) as i32 % 2 == 0 { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    }
    // `back`: while it pulls, the player walks away from it.
    if mode == "back" {
        if doing == "pull" || doing == "gather" { keys.press(KeyCode::KeyA) } else { keys.release(KeyCode::KeyA) }
    }
    // `dash`: drawn in by its void, the player dashes (free).
    if mode == "dash" {
        if !tugged.is_empty() && (t * 10.0) as i32 % 3 == 0 { keys.press(KeyCode::ShiftLeft) } else { keys.release(KeyCode::ShiftLeft) }
    }
    if mode == "kill" && t > 2.0 && t > state.1 {
        hits.write(crate::combat::Hit { by: None, target: e, damage: 60.0, harm: crate::creatures::Harm::Radiant, knock: Vec2::ZERO, stun: 0.0, at: dk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
        state.1 = t + 0.4;
    }
    let _ = dh;
}

/// `olmer` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): Olmer the Defiler
/// at 1120; the player (who can't die) stands at 900 (`PLATYPUS_OLMER=near`:
/// at 1075, for the slam; `parry`: near him, the player swings and swings).
/// Logs what he does, where the player is, the player's health lost.
fn olmer_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::custom::olmer::Olmer>), Without<LocalPlayer>>,
    mut state: Local<(u8, f32, &'static str)>,
    (mut mouse, mut cursor): (ResMut<ButtonInput<MouseButton>>, ResMut<CursorOverride>),
) {
    if s.name != "olmer" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let mode = std::env::var("PLATYPUS_OLMER").unwrap_or_default();
    state.1 += h.max - h.hp;
    h.hp = h.max;
    if state.0 == 0 && t > 0.5 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        k.body.pos = Vec2::new(if mode.is_empty() { 900.0 } else { 1075.0 }, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        crate::creatures::def::spawn_creature(&mut commands, "olmer", Vec2::new(1120.0, fl), |_| {});
        info!("olmer: Olmer at 1120 ({mode})");
        state.0 = 1;
        return;
    }
    let Some((_, _, ok, o)) = them.iter().find(|x| x.1.kind == "olmer") else { return };
    let doing = o.map_or("?", |o| o.doing());
    if doing != state.2 {
        info!("olmer: t {t:.2} {doing} at {:.0}; the player at {:.0} ({:.0} up), lost {:.0}", ok.body.pos.x, k.body.pos.x, k.body.pos.y - k.body.half.y - fl, state.1);
        state.2 = doing;
    }
    if mode == "parry" {
        cursor.0 = Some(ok.body.pos + Vec2::Y * 10.0);
        if (t * 6.0) as i32 % 2 == 0 { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    }
}

/// `templar` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a Fallen Templar
/// at 1120; the player (who can't die) stands at 900 (`PLATYPUS_TEMPLAR=`
/// `near`: at 1095, for the shove and chop; `mid`: at 1050, the thrust and
/// spin; `far`: at 990, the hook; `parry`: at 1050, swinging and swinging
/// at it; `dash`: at 990, dashing when hauled). Logs what it does, where
/// the player is, the player's health lost.
fn templar_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::custom::templar::Templar>), Without<LocalPlayer>>,
    mut state: Local<(u8, f32, &'static str)>,
    (mut mouse, mut cursor, mut keys): (ResMut<ButtonInput<MouseButton>>, ResMut<CursorOverride>, ResMut<ButtonInput<KeyCode>>),
    tugged: Query<(), (With<LocalPlayer>, With<crate::creatures::Tugged>)>,
) {
    if s.name != "templar" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let mode = std::env::var("PLATYPUS_TEMPLAR").unwrap_or_default();
    state.1 += h.max - h.hp;
    h.hp = h.max;
    if state.0 == 0 && t > 0.5 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        let x = match mode.as_str() {
            "near" => 1095.0,
            "mid" | "parry" => 1050.0,
            "far" | "dash" => 990.0,
            _ => 900.0,
        };
        k.body.pos = Vec2::new(x, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        crate::creatures::def::spawn_creature(&mut commands, "fallen_templar", Vec2::new(1120.0, fl), |_| {});
        info!("templar: a Fallen Templar at 1120 ({mode})");
        state.0 = 1;
        return;
    }
    let Some((_, _, tk, o)) = them.iter().find(|x| x.1.kind == "fallen_templar") else { return };
    let doing = o.map_or("?", |o| o.doing());
    if doing != state.2 {
        info!("templar: t {t:.2} {doing} at {:.0}; the player at {:.0} ({:.0} up), lost {:.0}", tk.body.pos.x, k.body.pos.x, k.body.pos.y - k.body.half.y - fl, state.1);
        state.2 = doing;
    }
    if mode == "parry" {
        cursor.0 = Some(tk.body.pos + Vec2::new(-30.0, 10.0));
        if (t * 6.0) as i32 % 2 == 0 { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    }
    if mode == "dash" {
        if !tugged.is_empty() && (t * 10.0) as i32 % 3 == 0 { keys.press(KeyCode::ShiftLeft) } else { keys.release(KeyCode::ShiftLeft) }
    }
}

/// `play` (`PLATYPUS_WORLD=arena PLATYPUS_MODE=play PLATYPUS_RESPAWN=1`):
/// what a new game starts with (logs the pack and what's worn), that the
/// key left of 1 doesn't bring up the dev tools, dying (the player, 120
/// cells off, killed at 2 s, wakes at the start: logs where), and Esc (at
/// 3 s and 3.5 s: logs the inventory open, then shut; the game menu's
/// `pack`'s).
#[allow(clippy::type_complexity)]
fn play_script(
    s: Res<Scenario>,
    mode: Res<crate::mode::Mode>,
    dev: Res<crate::hands::DevTools>,
    items: Option<Res<crate::hands::items::Items>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health, Option<&crate::hands::items::Inventory>, &crate::gear::Equipment), With<LocalPlayer>>,
    mut respawned: MessageReader<crate::respawn::Respawned>,
    (menu, virt, open): (Res<crate::pause::GameMenu>, Res<Time<Virtual>>, Res<crate::hands::InventoryOpen>),
    mut step: Local<u8>,
) {
    if s.name != "play" {
        return;
    }
    let t = s.elapsed;
    let Ok((mut k, mut h, inv, eq)) = player.single_mut() else { return };
    let Some(items) = items else { return };
    if *step == 0 && t > 1.0 {
        *step = 1;
        let carried: Vec<String> = inv.map(|i| i.slots.iter().flatten().map(|st| format!("{}x{}", items.def(st.item).id, st.count / items.unit(st.item))).collect()).unwrap_or_default();
        let worn = eq.worn.iter().flatten().count();
        info!("play: mode {}; carries {}; wears {worn}", mode.name(), carried.join(", "));
        keys.press(KeyCode::Backquote);
    } else if *step == 1 && t > 1.2 {
        *step = 2;
        keys.release(KeyCode::Backquote);
        info!("play: the key left of 1 pressed: dev tools {}", if dev.0 { "ON (wrong)" } else { "off" });
        k.body.pos.x += 120.0;
        k.prev_pos = k.body.pos;
    } else if *step == 2 && t > 2.0 {
        *step = 3;
        info!("play: killed at ({:.0}, {:.0})", k.body.pos.x, k.body.pos.y);
        h.hp = 0.0;
    } else if *step == 3 && respawned.read().count() > 0 {
        *step = 4;
        info!("play: woke at ({:.0}, {:.0}), life {:.0}/{:.0}", k.body.pos.x, k.body.pos.y, h.hp, h.max);
    } else if (*step == 4 && t > 3.0) || (*step == 6 && t > 3.5) {
        *step += 1;
        keys.press(KeyCode::Escape);
    } else if *step == 5 || *step == 7 {
        *step += 1;
        keys.release(KeyCode::Escape);
        info!("play: Esc: inventory {}, game menu {}, time {}", if open.0 { "open" } else { "shut" }, if menu.open { "up" } else { "shut" }, if virt.is_paused() { "paused" } else { "running" });
    }
}

/// `ropejump` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a jump from the
/// floor, held (logs how high it rose); then the hook thrown up at a stone
/// beam 160 overhead, hung from it, and a jump off it, held (logs how high
/// that rose over where it let go). Then the same off a wall: a column the
/// rope hangs against (logs how far out and up it went).
#[allow(clippy::too_many_arguments)]
fn ropejump_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut player: Query<(&mut Kinematics, Option<&crate::gear::hook::Rope>), With<LocalPlayer>>,
    mut st: Local<(u8, f32, f32, f32, f32)>,
) {
    if s.name != "ropejump" {
        return;
    }
    let Ok((mut k, rope)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let feet = k.body.pos.y - k.body.half.y;
    let (what, _) = rope.map_or(("none", 0), |r| r.state());
    let release = |keys: &mut ButtonInput<KeyCode>| {
        keys.release(KeyCode::Space);
        keys.release(KeyCode::KeyE);
    };
    match st.0 {
        0 if t > 0.5 => {
            if let Some(stone) = sim.materials().id("stone") {
                for x in (900..=1100).step_by(6) {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, fl as i32 + 165), radius: 4, material: stone, overwrite: true });
                }
            }
            k.body.pos = Vec2::new(1000.0, fl + k.body.half.y);
            k.prev_pos = k.body.pos;
            st.0 = 1;
            st.1 = t;
        }
        // A ground jump, held.
        1 if t > st.1 + 0.6 => {
            keys.press(KeyCode::Space);
            st.0 = 2;
            st.1 = t;
            st.2 = feet;
            st.3 = feet;
        }
        2 => {
            st.3 = st.3.max(feet);
            if t > st.1 + 0.9 {
                release(&mut keys);
                info!("ropejump: a ground jump, held, rose {:.0}", st.3 - st.2);
                st.0 = 3;
                st.1 = t;
            }
        }
        // The hook up at the beam; hang.
        3 if t > st.1 + 0.4 => {
            cursor.0 = Some(Vec2::new(k.body.pos.x, fl + 165.0));
            keys.press(KeyCode::KeyE);
            st.0 = 4;
            st.1 = t;
        }
        4 if t > st.1 + 0.1 => {
            keys.release(KeyCode::KeyE);
            if what == "hang" || t > st.1 + 2.5 {
                // (Rope let out: room under the beam to jump.)
                keys.press(KeyCode::KeyS);
                st.0 = 40;
                st.1 = t;
            }
        }
        40 if t > st.1 + 0.7 => {
            keys.release(KeyCode::KeyS);
            st.0 = 41;
            st.1 = t;
        }
        41 if t > st.1 + 0.5 => {
            info!("ropejump: hanging ({what}) at {:.0} over the floor", feet - fl);
            keys.press(KeyCode::Space);
            st.0 = 5;
            st.1 = t;
            st.2 = feet;
            st.3 = feet;
        }
        5 => {
            st.3 = st.3.max(feet);
            if t > st.1 + 0.9 {
                release(&mut keys);
                info!("ropejump: a jump off the rope, held, rose {:.0} over where it let go", st.3 - st.2);
                st.0 = 6;
            }
        }
        _ => {}
    }
}

/// `blade` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`, `PLATYPUS_WEAPON`:
/// longsword): what a blade does to the world. `PLATYPUS_BLADE=grass`: a
/// meadow of tall grass (20 high) to the right, slashed through at 1 s:
/// logs the grass before and after, the clippings flying. `water` /
/// `blue_goo`: a pit of it to the right, slashed into at 1 s: logs what flew
/// and that none's lost. `rope`: hung from a stone beam 120 up, S held and
/// a click aimed below: logs whether it struck down (it mustn't).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn blade_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    (mut hand, items): (ResMut<crate::hands::Hand>, Option<Res<crate::hands::items::Items>>),
    (mut mouse, mut cursor, mut keys): (ResMut<ButtonInput<MouseButton>>, ResMut<CursorOverride>, ResMut<ButtonInput<KeyCode>>),
    mut player: Query<(&mut Kinematics, &crate::hands::items::Inventory, Option<&crate::gear::hook::Rope>, Option<&crate::combat::Swing>), With<LocalPlayer>>,
    mut st: Local<(u8, f32, usize)>,
) {
    if s.name != "blade" {
        return;
    }
    let Ok((mut k, inv, rope, swing)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR;
    let mode = std::env::var("PLATYPUS_BLADE").unwrap_or_else(|_| "grass".into());
    let weapon = std::env::var("PLATYPUS_WEAPON").unwrap_or_else(|_| "longsword".into());
    if t < 0.5 {
        return;
    }
    take_up(&mut hand, inv, items.as_deref(), &weapon);
    let (liquid, grass) = (sim.materials().id(if mode == "grass" || mode == "rope" { "water" } else { &mode }), sim.materials().id("tall_grass"));
    let count = |sim: &SimWorld, m: Option<platypus_sim::MaterialId>, xs: std::ops::Range<i32>, ys: std::ops::Range<i32>| xs.flat_map(|x| ys.clone().map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| Some(c.material) == m)).count();
    let flying = |sim: &SimWorld, m: Option<platypus_sim::MaterialId>| sim.world.particles().iter().filter(|p| Some(p.cell.material) == m).count();
    match st.0 {
        0 => {
            match mode.as_str() {
                "grass" => {
                    for x in 1010..1070 {
                        for y in fl..fl + 20 {
                            sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: grass.unwrap(), overwrite: true });
                        }
                    }
                }
                "rope" => {
                    if let Some(stone) = sim.materials().id("stone") {
                        for x in (940..=1060).step_by(6) {
                            sim.queue(WorldEdit::Paint { center: CellPos::new(x, fl + 125), radius: 4, material: stone, overwrite: true });
                        }
                    }
                }
                _ => {
                    for x in 1008..1043 {
                        for y in fl - 10..fl {
                            sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: liquid.unwrap(), overwrite: true });
                        }
                    }
                }
            }
            k.body.pos = Vec2::new(1000.0, fl as f32 + k.body.half.y);
            k.prev_pos = k.body.pos;
            st.0 = 1;
        }
        1 if t > 1.0 => {
            match mode.as_str() {
                "grass" => {
                    st.2 = count(&sim, grass, 1005..1075, fl..fl + 22);
                    cursor.0 = Some(Vec2::new(1060.0, fl as f32 + 9.0));
                }
                "rope" => {
                    cursor.0 = Some(Vec2::new(k.body.pos.x, fl as f32 + 125.0));
                    keys.press(KeyCode::KeyE);
                }
                _ => {
                    st.2 = count(&sim, liquid, 1005..1046, fl - 12..fl + 4);
                    cursor.0 = Some(Vec2::new(1028.0, fl as f32 - 4.0));
                }
            }
            if mode != "rope" {
                mouse.press(MouseButton::Left);
            }
            st.0 = 2;
            st.1 = t;
        }
        2 if mode == "rope" => {
            keys.release(KeyCode::KeyE);
            let (what, _) = rope.map_or(("none", 0), |r| r.state());
            if what == "hang" || t > st.1 + 2.5 {
                keys.press(KeyCode::KeyS);
                cursor.0 = Some(k.body.pos - Vec2::new(0.0, 40.0));
                mouse.press(MouseButton::Left);
                st.0 = 3;
                st.1 = t;
            }
        }
        2 if t > st.1 + 0.25 => {
            mouse.release(MouseButton::Left);
            match mode.as_str() {
                "grass" => info!("blade: grass: {} clippings flying", flying(&sim, grass)),
                _ => info!("blade: {mode}: {} of {} cells of it in the air", flying(&sim, liquid), st.2),
            }
            st.0 = 3;
            st.1 = t;
        }
        3 if mode == "rope" && t > st.1 + 0.1 => {
            mouse.release(MouseButton::Left);
            keys.release(KeyCode::KeyS);
            let (what, _) = rope.map_or(("none", 0), |r| r.state());
            info!("blade: rope: hanging ({what}), S held, aimed below: swing {}, down-strike {}", swing.is_some(), swing.is_some_and(|s| s.downward()));
            st.0 = 4;
        }
        3 if mode != "rope" && t > st.1 + 2.5 => {
            match mode.as_str() {
                "grass" => info!("blade: grass: {} cells of it before the slash, {} after", st.2, count(&sim, grass, 1005..1075, fl..fl + 22)),
                _ => info!("blade: {mode}: {} cells before, {} landed and settled ({} still flying)", st.2, count(&sim, liquid, 960..1090, fl - 12..fl + 40), flying(&sim, liquid)),
            }
            st.0 = 4;
        }
        _ => {}
    }
}

/// `shield` (`PLATYPUS_WORLD=arena`): the wooden shield worn, raised (the
/// right button held, the cursor to the right); from 1 s a 20-point blow
/// from in front every 0.4 s till the guard breaks; then, lowered and
/// raised again, one from behind. Logs health, stamina and the guard.
#[allow(clippy::type_complexity)]
fn shield_script(
    s: Res<Scenario>,
    items: Option<Res<crate::hands::items::Items>>,
    mut player: Query<(Entity, &Kinematics, &crate::creatures::Health, &mut crate::gear::Equipment, Option<&crate::combat::Stamina>, Option<&crate::gear::shield::Guard>), With<LocalPlayer>>,
    (mut mouse, mut cursor): (ResMut<ButtonInput<MouseButton>>, ResMut<CursorOverride>),
    mut hits: MessageWriter<crate::combat::Hit>,
    mut st: Local<(u8, f32)>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
) {
    if s.name != "shield" {
        return;
    }
    // (The dev menu shut: no god mode.)
    menu.open = false;
    let Ok((me, k, h, mut eq, stamina, guard)) = player.single_mut() else { return };
    let Some(items) = items else { return };
    let t = s.elapsed;
    // (After the kit's on: it wears a flashlight there.)
    if st.0 == 0 && t > 0.5 {
        eq.worn[crate::gear::OFF_HAND] = items.id("wooden_shield").map(|i| crate::hands::items::Stack::new(i, 1));
        st.0 = 1;
        st.1 = 1.0;
    }
    if st.0 == 0 {
        return;
    }
    cursor.0 = Some(k.body.pos + Vec2::new(80.0, 0.0));
    if !mouse.pressed(MouseButton::Right) && st.0 != 3 {
        mouse.press(MouseButton::Right);
    }
    let sta = stamina.map_or(0.0, |s| s.cur);
    let (raised, broken) = guard.map_or((false, 0.0), |g| (g.raised, g.broken));
    let blow = |side: f32| crate::combat::Hit { by: None, target: me, damage: 20.0, harm: crate::creatures::Harm::Slash, knock: Vec2::new(-side * 200.0, 60.0), stun: 0.2, at: k.body.pos + Vec2::new(side * 5.0, 2.0), dir: Vec2::new(-side, 0.0), weight: 1.0, crit: false };
    if t >= st.1 {
        match st.0 {
            1 => {
                info!("shield: before a blow: life {:.0}, stamina {sta:.0}, raised {raised}", h.hp);
                hits.write(blow(1.0));
                st.1 = t + 0.4;
                if broken > 0.0 {
                    info!("shield: the guard broke (down {broken:.1} s)");
                    st.0 = 2;
                    st.1 = t + 2.5;
                }
            }
            2 => {
                info!("shield: up again: life {:.0}, stamina {sta:.0}, raised {raised}; a blow from behind", h.hp);
                hits.write(blow(-1.0));
                st.0 = 4;
                st.1 = t + 0.3;
            }
            4 => {
                info!("shield: after the blow from behind: life {:.0}, stamina {sta:.0}", h.hp);
                st.0 = 5;
            }
            _ => {}
        }
    }
}

/// `castle` (any world with castles): the player put by the nearest
/// castle (its landmark), kept alive; at 6 s, the candles about it (props,
/// lit) and any candle cells left (logs both).
fn castle_script(s: Res<Scenario>, sim: Res<SimWorld>, mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>, them: Query<(&crate::creatures::Creature, &Kinematics, Option<&crate::light::LightSource>), Without<LocalPlayer>>, mut st: Local<(u8, Vec2)>) {
    if s.name != "castle" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    if st.0 == 0 && s.elapsed > 1.0 {
        let me = k.body.pos;
        let Some((at, _)) = sim.generator.landmarks().into_iter().filter(|(_, n)| n == "castle").min_by_key(|(p, _)| (p.x as f32 - me.x).abs() as i64) else {
            st.0 = 9;
            return info!("castle: no castle in this world");
        };
        k.body.pos = Vec2::new(at.x as f32, at.y as f32 + 30.0);
        k.prev_pos = k.body.pos;
        st.0 = 1;
        st.1 = k.body.pos;
        info!("castle: to the castle at ({}, {})", at.x, at.y);
    }
    if st.0 == 1 && s.elapsed > 6.0 {
        st.0 = 2;
        let near = |p: Vec2| p.distance(st.1) < 600.0;
        let candles: Vec<_> = them.iter().filter(|(c, k, _)| c.kind == "candle" && near(k.body.pos)).collect();
        let lit = candles.iter().filter(|(.., l)| l.is_some()).count();
        let w = &sim.world;
        let cid = w.materials().expect_id("candle");
        let c = CellPos::from_world(st.1.x, st.1.y);
        let cells = (-600..600).flat_map(|dx| (-400..400).map(move |dy| c.offset(dx, dy))).filter(|p| w.get(*p).is_some_and(|x| x.material == cid)).count();
        info!("castle: {} candles about it ({lit} lit); candle cells left: {cells}", candles.len());
    }
}

/// `hurt` (`PLATYPUS_WORLD=arena`, `PLATYPUS_DIFFICULTY=` each): at 1 s a
/// 20-point sword blow to the player; logs what it took.
fn hurt_script(s: Res<Scenario>, difficulty: Res<crate::difficulty::Difficulty>, mut player: Query<&mut crate::creatures::Health, With<LocalPlayer>>, mut done: Local<bool>) {
    if s.name != "hurt" || *done || s.elapsed < 1.0 {
        return;
    }
    let Ok(mut h) = player.single_mut() else { return };
    *done = true;
    let before = h.hp;
    h.harm(20.0, crate::creatures::Harm::Slash);
    info!("hurt: {}: a 20 blow took {:.1} (armour stopping {:.0}%)", difficulty.name(), before - h.hp, (1.0 - h.ward.through(crate::creatures::Harm::Slash) / h.ward.taken) * 100.0);
}

/// `specks` (any world: `PLATYPUS_SAVE=` a copy of one played): at 4 s,
/// every lone solid near the player (a static cell, playfield or
/// background, with no solid next to it, up, down or across: a speck that
/// hangs), counted by material and layer, with a few places each.
fn specks_script(s: Res<Scenario>, sim: Res<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut done: Local<bool>) {
    if s.name != "specks" || *done || s.elapsed < 4.0 {
        return;
    }
    let Ok(pk) = player.single() else { return };
    *done = true;
    let w = &sim.world;
    let mats = w.materials();
    let solid = |c: Option<platypus_sim::Cell>| c.is_some_and(|c| !c.is_air() && matches!(mats.phys(c.material).kind, platypus_sim::Kind::Static | platypus_sim::Kind::Powder));
    let mut found: std::collections::BTreeMap<(String, &str), (u32, Vec<(i32, i32)>)> = Default::default();
    let me = CellPos::from_world(pk.body.pos.x, pk.body.pos.y);
    for dx in -700..700 {
        for dy in -500..500 {
            let p = me.offset(dx, dy);
            for (layer, get) in [("front", 0), ("back", 1)] {
                let at = |q: CellPos| if get == 0 { w.get(q) } else { w.get_bg(q) };
                let c = at(p);
                // (Loose is falling already: rubble.)
                if !c.is_some_and(|c| !c.is_air() && mats.phys(c.material).kind == platypus_sim::Kind::Static && c.flags & platypus_sim::cell::flags::LOOSE == 0) {
                    continue;
                }
                let lone = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().all(|&(ox, oy)| !solid(at(p.offset(ox, oy))));
                // (Background behind solid playfield is held: a wall.)
                if lone && !(get == 1 && solid(w.get(p))) {
                    let e = found.entry((mats.def(c.unwrap().material).name.clone(), layer)).or_default();
                    e.0 += 1;
                    if e.1.len() < 4 {
                        e.1.push((dx, dy));
                    }
                }
            }
        }
    }
    info!("specks: {} kinds of lone solid near the player", found.len());
    // (And how warm the rock is: over 60 °C hurt to touch, before; over 220, now.)
    let (mut warm, mut hot, mut deepest) = (0u32, 0u32, (0i32, 0i32, 0i32));
    for dx in (-700..700).step_by(2) {
        for dy in (-500..500).step_by(2) {
            let p = me.offset(dx, dy);
            let Some(c) = w.get(p) else { continue };
            if c.is_air() || mats.phys(c.material).kind != platypus_sim::Kind::Static {
                continue;
            }
            let t = w.climate().ambient(p.x, p.y) + c.heat as i32;
            warm += (t > 60) as u32;
            hot += (t > 220) as u32;
            if t > deepest.0 {
                deepest = (t, dx, dy);
            }
        }
    }
    info!("specks: rock over 60 °C: {warm} (of every other cell), over 220: {hot}; hottest {} °C at {:?}", deepest.0, (deepest.1, deepest.2));
    for ((m, layer), (n, at)) in &found {
        info!("specks: {n} {m} ({layer}), e.g. at {at:?} from the player");
        if let Some(&(dx, dy)) = at.first() {
            let p = me.offset(dx, dy);
            let c = w.get(p).unwrap_or(platypus_sim::Cell::AIR);
            let round: Vec<String> = [(-1, 1), (0, 1), (1, 1), (-1, 0), (1, 0), (-1, -1), (0, -1), (1, -1)].iter().map(|&(ox, oy)| w.get(p.offset(ox, oy)).map_or("?".into(), |n| if n.is_air() { ".".into() } else { mats.def(n.material).name.clone() })).collect();
            info!("specks:   flags {:#x} life {} bears {} vy {}; round it {round:?}; behind {:?}", c.flags, c.life, mats.bears_load(c), c.vy, w.get_bg(p).map(|b| mats.def(b.material).name.clone()));
        }
    }
}

/// `throw` (`PLATYPUS_WORLD=arena`): a bomb (the dev kit's, wherever it
/// is) readied at 1 s (the left button held: held up, its arc shown), let
/// go at 1.6 s toward a point 120 right and 60 up (`PLATYPUS_THROW_AT=x,y`
/// elsewhere); logs where every bomb is, ten times a second.
fn throw_script(
    s: Res<Scenario>,
    mut hand: ResMut<crate::hands::Hand>,
    player: Query<(&Kinematics, &crate::hands::items::Inventory), With<LocalPlayer>>,
    items: Option<Res<crate::hands::items::Items>>,
    bombs: Query<(&Kinematics, &Sprite, &Transform, &Visibility), With<crate::props::Bomb>>,
    (mut mouse, mut cursor): (ResMut<ButtonInput<MouseButton>>, ResMut<CursorOverride>),
    mut st: Local<(u8, f32)>,
) {
    if s.name != "throw" {
        return;
    }
    let Ok((pk, inv)) = player.single() else { return };
    let t = s.elapsed;
    // (The bombs, wherever they are.)
    take_up(&mut hand, inv, items.as_deref(), "bomb");
    // (`PLATYPUS_THROW_AT=x,y`: aimed there from the player instead.)
    let aim = std::env::var("PLATYPUS_THROW_AT").ok().and_then(|v| v.split_once(',').and_then(|(x, y)| Some(Vec2::new(x.parse().ok()?, y.parse().ok()?)))).unwrap_or(Vec2::new(120.0, 60.0));
    cursor.0 = Some(pk.body.pos + aim);
    // (Readied from 1 s, held up, let go at 1.6 s.)
    if st.0 == 0 && t > 1.0 {
        st.0 = 1;
        mouse.press(MouseButton::Left);
    } else if st.0 == 1 && t > 1.6 {
        st.0 = 2;
        mouse.release(MouseButton::Left);
        info!("throw: let go at 1.6 s");
    }
    if t >= st.1 {
        st.1 = t + 0.1;
        for (k, sprite, tf, vis) in &bombs {
            info!("throw: t {t:.1}: bomb at ({:+.0}, {:+.0}) vel ({:.0}, {:.0}) drawn at ({:+.0}, {:+.0}) z {:.1} {:?} size {:?}", k.body.pos.x - pk.body.pos.x, k.body.pos.y - pk.body.pos.y, k.body.vel.x, k.body.vel.y, tf.translation.x - pk.body.pos.x, tf.translation.y - pk.body.pos.y, tf.translation.z, vis, sprite.custom_size);
        }
    }
}

/// `lights` (`PLATYPUS_WORLD=arena PLATYPUS_HOUR=23`, with `PLATYPUS_SHOTS`
/// and `PLATYPUS_SHOT_EVERY=2`): what's worn for light, in the dark: a
/// spider right by the player, a zombie 150 out in front, the cursor on the
/// zombie. 0-3 s the flashlight (the dev kit's), 3-5 the lantern, 5-7 a
/// torch, 7-9 nothing held but a mining helmet. Logs what's carried and how
/// the two's eyes shine.
#[allow(clippy::type_complexity)]
fn lights_script(
    mut commands: Commands,
    s: Res<Scenario>,
    items: Option<Res<crate::hands::items::Items>>,
    toggles: Res<crate::light::LightToggles>,
    mut player: Query<(&Kinematics, &mut crate::gear::Equipment), With<LocalPlayer>>,
    them: Query<(&crate::creatures::Creature, &Kinematics, Option<&crate::light::Shine>), Without<LocalPlayer>>,
    mut cursor: ResMut<CursorOverride>,
    mut st: Local<(u8, f32, bool)>,
) {
    if s.name != "lights" {
        return;
    }
    let Ok((pk, mut eq)) = player.single_mut() else { return };
    let Some(items) = items else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    if !st.2 && t > 0.5 {
        st.2 = true;
        crate::creatures::def::spawn_creature(&mut commands, "spider", Vec2::new(pk.body.pos.x + 22.0, fl), |_| {});
        crate::creatures::def::spawn_creature(&mut commands, "zombie", Vec2::new(pk.body.pos.x + 150.0, fl), |_| {});
    }
    if let Some((_, zk, _)) = them.iter().find(|(c, ..)| c.kind == "zombie") {
        cursor.0 = Some(zk.body.pos + Vec2::Y * zk.body.half.y * 0.5);
    }
    let wear = |eq: &mut crate::gear::Equipment, i: usize, id: Option<&str>| eq.worn[i] = id.and_then(|id| items.id(id)).map(|it| crate::hands::items::Stack::new(it, 1));
    let step = match t {
        t if t < 3.0 => 1,
        t if t < 5.0 => 2,
        t if t < 7.0 => 3,
        _ => 4,
    };
    if step > st.0 {
        st.0 = step;
        match step {
            2 => wear(&mut eq, crate::gear::OFF_HAND, Some("lantern")),
            3 => wear(&mut eq, crate::gear::OFF_HAND, Some("torch")),
            4 => {
                wear(&mut eq, crate::gear::OFF_HAND, None);
                wear(&mut eq, 0, Some("mining_helmet"));
            }
            _ => {}
        }
    }
    if t >= st.1 {
        st.1 = t.floor() + 1.0;
        let shine = |kind: &str| them.iter().find(|(c, ..)| c.kind == kind).and_then(|(.., sh)| sh.map(|s| s.0)).unwrap_or(-1.0);
        info!("lights: t {t:.0}: carrying {:?}, headlamp {}; eye-shine: spider {:.2}, zombie {:.2}", toggles.carry, toggles.headlamp, shine("spider"), shine("zombie"));
    }
}

/// `goldsmash` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a lump of
/// solid gold (14 × 10 cells, with gold dust about it) on the floor 60 cells
/// right, a spider at 150; the gravity staff swapped into hand. From 1 s the
/// well holds the lump up over it; at 2.5 s three notches of the wheel turn
/// it; at 4 s the well swings fast onto the spider. Logs the bodies flying,
/// the held one's turn, the spider's health, and the mana.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn goldsmash_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    hand: Res<crate::hands::Hand>,
    mut player: Query<(&Kinematics, &mut crate::hands::items::Inventory, Option<&crate::magic::Mana>), With<LocalPlayer>>,
    them: Query<(&crate::creatures::Creature, &crate::creatures::Health, &Kinematics), Without<LocalPlayer>>,
    (mut mouse, mut cursor, mut scroll): (ResMut<ButtonInput<MouseButton>>, ResMut<CursorOverride>, ResMut<bevy::input::mouse::AccumulatedMouseScroll>),
    mut st: Local<(u8, f32, Vec2, Vec2)>,
) {
    if s.name != "goldsmash" {
        return;
    }
    let Ok((pk, mut inv, mana)) = player.single_mut() else { return };
    let Some(items) = items else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR;
    if st.0 == 0 && t > 0.5 {
        st.0 = 1;
        let x0 = pk.body.pos.x as i32 + 54;
        let gold = sim.world.materials().expect_id("solid_gold");
        let dust = sim.world.materials().expect_id("gold");
        for dx in 0..14 {
            for dy in 0..10 {
                sim.world.set(CellPos::new(x0 + dx, fl + dy), platypus_sim::Cell::new(gold, 0));
            }
        }
        for dx in -8..22 {
            sim.world.set(CellPos::new(x0 + dx, fl + 10 + (dx.rem_euclid(3) == 0) as i32), platypus_sim::Cell::new(dust, 0));
        }
        st.2 = Vec2::new(x0 as f32 + 7.0, fl as f32 + 5.0);
        crate::creatures::def::spawn_creature(&mut commands, "spider", Vec2::new(pk.body.pos.x + 150.0, fl as f32), |_| {});
        if let Some(staff) = items.id("gravity_staff")
            && let Some(i) = inv.slots.iter().position(|st| st.is_some_and(|st| st.item == staff))
        {
            let to = hand.active();
            inv.slots.swap(i, to);
        }
    }
    let spider = them.iter().find(|(c, ..)| c.kind == "spider");
    let flying = sim.world.bodies().len();
    let turn = sim.world.bodies().first().map_or(0.0, |b| b.rot[1].atan2(b.rot[0]).to_degrees());
    if t > 1.0 {
        // Hold it up over where it lay; then swing down onto the spider.
        let at = if t < 4.0 { st.2 + Vec2::new(0.0, 30.0) } else { spider.map_or(st.2, |(_, _, k)| k.body.pos) + Vec2::new(0.0, (5.0 - t).max(0.0) * 40.0) };
        cursor.0 = Some(at);
        mouse.press(MouseButton::Left);
    }
    if st.0 == 1 && t > 2.5 {
        st.0 = 2;
        scroll.delta.y = 3.0;
        scroll.unit = bevy::input::mouse::MouseScrollUnit::Line;
    }
    if t >= st.1 {
        st.1 = (t * 2.0).floor() / 2.0 + 0.5;
        info!("goldsmash: t {t:.1}: bodies flying {flying}, turned {turn:.0}°, spider {}, mana {:.0}", spider.map_or("gone".into(), |(_, h, _)| format!("{:.0}/{:.0}", h.hp, h.max)), mana.map_or(0.0, |m| m.cur));
    }
}

/// `PLATYPUS_CLOSE_AT=3` (any scenario): the window closed then, as a
/// player closing it does (what's done on the way out: saving).
fn close_window(mut commands: Commands, s: Res<Scenario>, windows: Query<Entity, With<bevy::window::PrimaryWindow>>, mut done: Local<bool>) {
    let Some(at) = std::env::var("PLATYPUS_CLOSE_AT").ok().and_then(|v| v.parse::<f32>().ok()) else { return };
    if *done || s.elapsed < at {
        return;
    }
    *done = true;
    for w in &windows {
        info!("scenario: closing the window");
        commands.entity(w).despawn();
    }
}

/// `starter` (`PLATYPUS_WORLD=small PLATYPUS_SAVE=off PLATYPUS_LIFE_RATE=20`,
/// `PLATYPUS_HOUR=22` for the night): what's about the start: every 5 s,
/// the creatures within 1500 cells by kind (the player kept alive).
fn starter_script(s: Res<Scenario>, sim: Res<SimWorld>, mut player: Query<(&Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>, them: Query<(&crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>, mut next: Local<f32>) {
    if s.name != "starter" {
        return;
    }
    let Ok((pk, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    if s.elapsed < *next {
        return;
    }
    *next = s.elapsed + 5.0;
    let (mut up, mut down): (std::collections::BTreeMap<String, u32>, std::collections::BTreeMap<String, u32>) = Default::default();
    for (c, k) in &them {
        if (k.body.pos.x - pk.body.pos.x).abs() < 1500.0 && (k.body.pos.y - pk.body.pos.y).abs() < 1500.0 {
            let ground = sim.generator.surface_hint(k.body.pos.x as i32).unwrap_or(0) as f32;
            let (map, dx) = if k.body.pos.y > ground - 40.0 { (&mut up, (k.body.pos.x - pk.body.pos.x) as i32) } else { (&mut down, (k.body.pos.x - pk.body.pos.x) as i32) };
            *map.entry(format!("{}@{}", c.kind, dx / 100 * 100)).or_default() += 1;
        }
    }
    info!("starter: t {:.0}: SURFACE {up:?}", s.elapsed);
    info!("starter: t {:.0}: UNDER {down:?}", s.elapsed);
}

/// `chop` (`PLATYPUS_WORLD=small PLATYPUS_SAVE=off`, play mode or dev):
/// the nearest tree to the start, cut a little above its foot (clear of the
/// ground, all its width in the open) with the copper axe until
/// it falls, then the log lying there bucked (the player put beside what's
/// left of it, the axe at its nearest wood); then a wall built of the wood:
/// four blocks up from the ground beside you, the cursor in the open air
/// above the top one each time. Logs each second: the wood carried (blocks),
/// the tree's wood standing (background) and lying (playfield) near it.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn chop_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    mut hand: ResMut<crate::hands::Hand>,
    mut player: Query<(&mut Kinematics, &mut crate::hands::items::Inventory), With<LocalPlayer>>,
    (mut mouse, mut cursor): (ResMut<ButtonInput<MouseButton>>, ResMut<CursorOverride>),
    mut st: Local<(u8, f32, Option<CellPos>, f32, i32)>,
) {
    if s.name != "chop" {
        return;
    }
    let Ok((mut k, mut inv)) = player.single_mut() else { return };
    let Some(items) = items else { return };
    let t = s.elapsed;
    let w = &sim.world;
    let wood = w.materials().expect_id("wood");
    let carried = items.id("block:wood").map_or(0.0, |i| inv.count(i) as f32 / 36.0);
    let near = |c: CellPos, bg: bool| (-160..=160).flat_map(|dx| (-40..=200).map(move |dy| c.offset(dx, dy))).filter(|p| if bg { w.get_bg(*p) } else { w.get(*p) }.is_some_and(|x| x.material == wood)).count();
    match st.0 {
        // The nearest trunk: wood behind open air, low, near the start.
        0 if t > 3.0 => {
            let p = CellPos::from_world(k.body.pos.x, k.body.pos.y);
            let foot = (0..600).flat_map(|d| [d, -d]).find_map(|dx| {
                let x = p.x + dx;
                (p.y - 120..p.y + 120).find(|&y| {
                    let c = CellPos::new(x, y);
                    w.get_bg(c).is_some_and(|b| b.material == wood) && w.get(c).is_some_and(|f| f.is_air()) && w.get_bg(c.offset(0, 30)).is_some_and(|b| b.material == wood)
                }).map(|y| CellPos::new(x, y + 14))
            });
            let Some(foot) = foot else {
                // (None here: on along the surface, and look again when it's loaded.)
                st.4 += 1;
                if st.4 > 10 {
                    st.0 = 9;
                    return info!("chop: no tree found");
                }
                let x = k.body.pos.x + 700.0;
                let y = sim.generator.surface_hint(x as i32).unwrap_or(p.y) as f32 + 40.0;
                k.body.pos = Vec2::new(x, y);
                k.prev_pos = k.body.pos;
                st.0 = 8;
                st.1 = t;
                return;
            };
            st.2 = Some(foot);
            st.0 = 1;
            k.body.pos = Vec2::new(foot.x as f32 - 14.0, foot.y as f32 + k.body.half.y);
            k.prev_pos = k.body.pos;
            take_up(&mut hand, &inv, Some(&items), "copper_axe");
            info!("chop: a tree at ({}, {}): standing {} cells", foot.x, foot.y, near(foot, true));
            // (PLATYPUS_CHOP=fire: the blaze staff's ray on its trunk instead.)
            if std::env::var("PLATYPUS_CHOP").is_ok_and(|v| v == "fire") {
                st.0 = 20;
                st.1 = t;
                k.body.pos.x -= 40.0;
                k.prev_pos = k.body.pos;
            }
        }
        20 => {
            let foot = st.2.expect("found");
            if let Some(blaze) = items.id("blaze_staff") {
                let to = hand.active();
                if let Some(i) = inv.slots.iter().position(|s| s.is_some_and(|s| s.item == blaze))
                    && i != to
                {
                    inv.slots.swap(i, to);
                }
            }
            cursor.0 = Some(Vec2::new(foot.x as f32 + 4.0, foot.y as f32 + 20.0));
            if t - st.1 < 3.0 {
                mouse.press(MouseButton::Left);
            } else {
                mouse.release(MouseButton::Left);
            }
            if t >= st.3 {
                st.3 = t + 2.0;
                let burning = (-80..=80).flat_map(|dx| (-20..=160).map(move |dy| foot.offset(dx, dy))).filter(|p| w.get(*p).is_some_and(|c| c.material == w.materials().expect_id("fire"))).count();
                info!("chop: fire: t {t:.0}: standing {}, flames about it {burning}", near(foot, true));
            }
        }
        1 | 2 => {
            let foot = st.2.expect("found");
            // Cut at its foot; once it's down, at the log's nearest wood.
            if st.0 == 1 && near(foot, false) > 200 {
                st.0 = 2;
                info!("chop: t {t:.1}: it's down: lying {} cells, standing {}", near(foot, false), near(foot, true));
            }
            let at = if st.0 == 1 {
                // (The trunk's nearest wood at its foot: deeper as it's cut.)
                (0..40).find_map(|dx| (0..6).map(|dy| CellPos::new(foot.x - 3 + dx, foot.y + dy)).find(|c| w.get_bg(*c).is_some_and(|b| b.material == wood) && w.get(*c).is_some_and(|f| f.is_air()))).map(|c| Vec2::new(c.x as f32 + 0.5, c.y as f32 + 0.5))
            } else {
                let me = CellPos::from_world(k.body.pos.x, k.body.pos.y);
                (0..200).find_map(|r| (-r..=r).flat_map(|dx| [CellPos::new(me.x + dx, me.y - r), CellPos::new(me.x + dx, me.y + r)]).chain((-r..=r).flat_map(|dy| [CellPos::new(me.x - r, me.y + dy), CellPos::new(me.x + r, me.y + dy)])).find(|c| w.get(*c).is_some_and(|x| x.material == wood))).map(|c| Vec2::new(c.x as f32 + 0.5, c.y as f32 + 0.5))
            };
            match at {
                Some(at) => {
                    // (Within the axe's reach: walk the body up to it.)
                    if at.distance(k.body.pos) > 30.0 {
                        let to = at - Vec2::new(at.x.signum() * 0.0 + if at.x > k.body.pos.x { 18.0 } else { -18.0 }, 0.0);
                        k.body.pos = Vec2::new(to.x, k.body.pos.y.max(at.y));
                        k.prev_pos = k.body.pos;
                    }
                    cursor.0 = Some(at);
                    mouse.press(MouseButton::Left);
                }
                None if st.0 == 2 => {
                    st.0 = 3;
                    st.1 = t;
                }
                // (Cut through: it's falling, a body in the air; then it lies.)
                None => {
                    mouse.release(MouseButton::Left);
                    if st.1 == 0.0 {
                        st.1 = t;
                        info!("chop: t {t:.1}: cut through; standing {}", near(foot, true));
                    } else if t - st.1 > 12.0 {
                        info!("chop: t {t:.1}: it never came down");
                        st.0 = 3;
                    }
                }
            }
            if t >= st.3 {
                st.3 = t + 1.0;
                info!("chop: t {t:.0}: carrying {carried:.1} blocks of wood; standing {}, lying {}", near(foot, true), near(foot, false));
            }
            if t > 40.0 && st.0 != 3 {
                st.0 = 3;
                st.1 = t;
            }
        }
        // Build: a wall four blocks up beside you, aiming into the air above.
        3 => {
            mouse.release(MouseButton::Left);
            take_up(&mut hand, &inv, Some(&items), "block:wood");
            st.0 = 4;
            st.1 = t;
            st.4 = 0;
            info!("chop: building with {carried:.1} blocks of wood");
        }
        4 => {
            let feet = k.body.pos.y - k.body.half.y;
            let col = ((k.body.pos.x + 16.0) / 6.0).floor() as i32;
            let ground = (feet / 6.0).floor() as i32;
            let woody = |r: i32| w.get(CellPos::new(col * 6 + 3, r * 6 + 3)).is_some_and(|c| c.material == wood);
            let n = (ground - 2..ground + 12).filter(|&r| woody(r)).count() as i32;
            let top = (ground - 2..ground + 12).filter(|&r| woody(r)).max().unwrap_or(ground);
            if t - st.1 > 0.15 {
                st.1 = t;
                // (The air above the top: a block over it.)
                cursor.0 = Some(Vec2::new(col as f32 * 6.0 + 3.0, (top + 1) as f32 * 6.0 + 9.5));
                mouse.press(MouseButton::Left);
                st.4 += 1;
            } else {
                mouse.release(MouseButton::Left);
            }
            if n >= 4 || st.4 > 30 {
                st.0 = 5;
                info!("chop: the wall stands {n} blocks high (wood left {carried:.1} blocks)");
                mouse.release(MouseButton::Left);
            }
        }
        8 if t > st.1 + 2.0 => st.0 = 0,
        _ => {}
    }
}

/// `title` (`PLATYPUS_TITLE=1`, best with `PLATYPUS_SAVE_DIR` a scratch
/// folder): the title over the world behind it; at 4 s Enter (Continue, or
/// for a new game's world, New game); logs whether the title's up, time
/// paused and whether there's a save, before and after (Ctrl+S at 6 s).
/// `PLATYPUS_TITLE_PICK=arena`: the Arena picked instead (the game starts
/// again with it: offscreen too, as this one is).
#[allow(clippy::too_many_arguments)]
fn title_script(s: Res<Scenario>, title: Res<crate::title::Title>, virt: Res<Time<Virtual>>, slot: Option<Res<crate::save::SaveSlot>>, mut keys: ResMut<ButtonInput<KeyCode>>, mut saves: MessageWriter<crate::save::SaveNow>, mut picks: MessageWriter<crate::title::Picked>, mut step: Local<u8>) {
    if s.name != "title" {
        return;
    }
    let t = s.elapsed;
    let saved = || slot.as_ref().is_some_and(|sl| sl.dir.join("world.ron").exists());
    let state = |w: &str| info!("title: {w}: up {}, paused {}, saved {}", title.up, virt.is_paused(), saved());
    match *step {
        0 if t > 3.5 => {
            *step = 1;
            state("before");
            saves.write(crate::save::SaveNow);
        }
        1 if t > 4.0 => {
            *step = 2;
            info!("title: a save asked for at the title: saved {}", saved());
            if std::env::var("PLATYPUS_TITLE_PICK").is_ok_and(|v| v == "arena") {
                picks.write(crate::title::Picked(crate::title::Choice::Arena));
            } else {
                keys.press(KeyCode::Enter);
            }
        }
        2 if t > 4.1 => {
            *step = 3;
            keys.release(KeyCode::Enter);
        }
        3 if t > 6.0 => {
            *step = 4;
            saves.write(crate::save::SaveNow);
        }
        4 if t > 7.0 => {
            *step = 5;
            state("after");
        }
        _ => {}
    }
}

/// `devmenu` (`PLATYPUS_WORLD=arena`, with `PLATYPUS_SHOTS` and
/// `PLATYPUS_SHOT_EVERY=1.5` to see it): the dev menu open at each tab in
/// turn (1.5 s each: Player, Items, Items searched for "sword", Creatures,
/// World, Tools, View, Time, Sounds; `PLATYPUS_SEARCH` another search);
/// on the way, a longsword and the
/// `miner` kit are clicked, and what the pack holds is logged before and after.
#[allow(clippy::type_complexity)]
fn devmenu_script(
    s: Res<Scenario>,
    items: Option<Res<crate::hands::items::Items>>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut buttons: Query<(&mut Interaction, Option<&crate::devmenu::ItemButton>, Option<&crate::devmenu::Act>)>,
    player: Query<&crate::hands::items::Inventory, With<LocalPlayer>>,
    mut step: Local<usize>,
) {
    use crate::devmenu::Tab;
    if s.name != "devmenu" {
        return;
    }
    let Some(items) = items else { return };
    let tabs = [Tab::Player, Tab::Items, Tab::Items, Tab::Creatures, Tab::World, Tab::Tools, Tab::View, Tab::Time, Tab::Sounds];
    let i = ((s.elapsed - 0.75) / 1.5).floor();
    if i < 0.0 || i as usize >= tabs.len() {
        return;
    }
    let i = i as usize;
    menu.open = true;
    menu.tab = tabs[i];
    let search = std::env::var("PLATYPUS_SEARCH").unwrap_or_else(|_| "sword".into());
    menu.set_search(if i == 2 { &search } else { "" });
    let count = |id: &str| player.single().ok().map_or(0, |inv| items.id(id).map_or(0, |it| inv.count(it) / items.unit(it)));
    if *step == 0 && i == 1 && s.elapsed > 2.6 {
        *step = 1;
        info!("devmenu: before: longsword {}, gold_pickaxe {}, torch {}", count("longsword"), count("gold_pickaxe"), count("torch"));
        let sword = items.id("longsword");
        for (mut int, b, _) in &mut buttons {
            if b.is_some_and(|b| Some(b.0) == sword) {
                *int = Interaction::Pressed;
            }
        }
    } else if *step == 1 && s.elapsed > 2.8 {
        *step = 2;
        for (mut int, _, act) in &mut buttons {
            if act.is_some_and(|a| *a == crate::devmenu::Act::Kit("miner".into())) {
                *int = Interaction::Pressed;
            }
        }
    } else if *step == 2 && s.elapsed > 3.0 {
        *step = 3;
        info!("devmenu: after: longsword {}, gold_pickaxe {}, torch {}", count("longsword"), count("gold_pickaxe"), count("torch"));
    }
}

/// `devmenu`, placing: the zombie's button pressed at 15 s.
fn devmenu_pick_script(s: Res<Scenario>, mut buttons: Query<(&mut Interaction, &crate::devmenu::Act)>, mut done: Local<bool>) {
    if s.name != "devmenu" || *done || s.elapsed < 15.0 {
        return;
    }
    for (mut i, a) in &mut buttons {
        if *a == crate::devmenu::Act::Arena(crate::arena::ArenaAction::Pick("zombie".into())) {
            *i = Interaction::Pressed;
            *done = true;
        }
    }
}

/// `devmenu`, placing: at 15 s the zombie picked in the Creatures tab, at
/// 15.5 s and 16 s a click in the world 80 and 120 cells to the right; logs
/// the zombies there before and after (`PLATYPUS_SCENARIO_SECS=18`).
#[allow(clippy::type_complexity)]
fn devmenu_place_script(
    s: Res<Scenario>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    (mut mouse, mut cursor): (ResMut<ButtonInput<MouseButton>>, ResMut<CursorOverride>),
    player: Query<&Kinematics, With<LocalPlayer>>,
    them: Query<&crate::creatures::Creature>,
    mut step: Local<u8>,
) {
    if s.name != "devmenu" {
        return;
    }
    let Ok(k) = player.single() else { return };
    let zombies = || them.iter().filter(|c| c.kind == "zombie").count();
    let t = s.elapsed;
    match *step {
        0 if t > 14.5 => {
            *step = 1;
            menu.open = true;
            menu.tab = crate::devmenu::Tab::Creatures;
        }
        // (The pick itself: `devmenu_pick_script`, after the UI's focus.)
        1 if t > 15.0 => {
            *step = 2;
            info!("devmenu: placing: zombies before {}", zombies());
        }
        2 | 4 if t > 15.0 + 0.5 * (*step / 2) as f32 => {
            cursor.0 = Some(k.body.pos + Vec2::new(if *step == 2 { 80.0 } else { 120.0 }, 0.0));
            mouse.press(MouseButton::Left);
            *step += 1;
        }
        3 | 5 => {
            mouse.release(MouseButton::Left);
            *step += 1;
        }
        6 if t > 17.0 => {
            *step = 7;
            info!("devmenu: placing: zombies after {}", zombies());
        }
        _ => {}
    }
}

/// `heatglow` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a heat gun's
/// worth of heat (radius 8, 60 °C a tick) held on the floor 70 right of the
/// player for 3 s (`PLATYPUS_HEAT_SECS`); logs every 2 s how far out the rock glows (480 °C+), the
/// hottest cell, and the magma and basalt there, till the rock's cold.
fn heatglow_script(s: Res<Scenario>, mut sim: ResMut<SimWorld>, player: Query<&Kinematics, With<LocalPlayer>>, mut at: Local<Option<CellPos>>, mut next: Local<f32>) {
    if s.name != "heatglow" {
        return;
    }
    let t = s.elapsed;
    if at.is_none() {
        let Ok(k) = player.single() else { return };
        if t < 1.0 {
            return;
        }
        let fl = platypus_worldgen::arena::FLOOR;
        *at = Some(CellPos::new(k.body.pos.x as i32 + 70, fl - 4));
        *next = 1.0;
    }
    let c = at.expect("set");
    let held: f32 = std::env::var("PLATYPUS_HEAT_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(3.0);
    if t < 1.0 + held {
        sim.queue(WorldEdit::Heat { center: c, radius: 8, amount: 60 });
    }
    if t >= *next {
        *next += 2.0;
        let w = &sim.world;
        let m = w.materials().clone();
        let glow = (0..40).rev().find(|d| w.temperature(c.offset(*d, -6)).is_some_and(|tc| tc >= 480)).unwrap_or(0);
        let near = |name: &str| m.id(name).map_or(0, |id| (-30..=30).flat_map(|dx| (-30..=10).map(move |dy| c.offset(dx, dy))).filter(|p| w.get(*p).is_some_and(|cell| cell.material == id)).count());
        let hottest = (-30..=30).flat_map(|dx| (-30..=10).map(move |dy| c.offset(dx, dy))).filter_map(|p| w.temperature(p)).max().unwrap_or(0);
        info!("heatglow: t {t:.0}: glows {glow} out (6 down); hottest {hottest} °C; magma {}, basalt {}", near("magma"), near("basalt"));
    }
}

/// `toad` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a bloat toad at
/// 1120; the player (who can't die) stands at 1020 (`PLATYPUS_TOAD=`: `dash`:
/// dashes when the tongue sticks; `cut`: swings at the tongue; `kill`: it's
/// struck till it dies (its methane), and at 2 s after, a torch thrown in:
/// the gas goes up). Logs what it does, where the player is, the health lost.
#[allow(clippy::too_many_arguments)]
fn toad_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::custom::toad::Toad>), Without<LocalPlayer>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, f32, f32, &'static str, Option<Vec2>)>,
    (mut mouse, mut cursor, mut keys): (ResMut<ButtonInput<MouseButton>>, ResMut<CursorOverride>, ResMut<ButtonInput<KeyCode>>),
    tugged: Query<(), (With<LocalPlayer>, With<crate::creatures::Tugged>)>,
) {
    if s.name != "toad" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let mode = std::env::var("PLATYPUS_TOAD").unwrap_or_default();
    state.2 += h.max - h.hp;
    h.hp = h.max;
    if state.0 == 0 && t > 0.5 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        k.body.pos = Vec2::new(1020.0, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        crate::creatures::def::spawn_creature(&mut commands, "bloat_toad", Vec2::new(1120.0, fl), |_| {});
        info!("toad: a bloat toad at 1120 ({mode})");
        state.0 = 1;
        return;
    }
    let methane = |sim: &SimWorld| sim.materials().id("swamp_gas").map_or(0, |m| (950..1250).flat_map(|x| (fl as i32 - 5..fl as i32 + 60).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).count());
    let Some((e, _, tk, o)) = them.iter().find(|x| x.1.kind == "bloat_toad") else {
        // Dead: its gas; a torch thrown into it 2 s on.
        if state.0 == 1 && t > 1.0 {
            state.0 = 2;
            state.1 = t;
            info!("toad: t {t:.1} dead; swamp gas {} cells about", methane(&sim));
        }
        if state.0 == 2 && t > state.1 + 2.0 {
            if let Some(at) = state.4 {
                sim.queue(WorldEdit::Ignite { center: CellPos::from_world(at.x, at.y), radius: 3 });
            }
            state.0 = 3;
            state.1 = t;
        }
        if state.0 == 3 && t > state.1 + 1.0 {
            info!("toad: t {t:.1} a torch in the gas: swamp gas {} cells left; the player lost {:.0}", methane(&sim), state.2);
            state.0 = 4;
        }
        return;
    };
    state.4 = Some(tk.body.pos);
    let doing = o.map_or("?", |o| o.doing());
    if doing != state.3 {
        info!("toad: t {t:.2} {doing} at {:.0}; the player at {:.0} ({:.0} up), lost {:.0}", tk.body.pos.x, k.body.pos.x, k.body.pos.y - k.body.half.y - fl, state.2);
        state.3 = doing;
    }
    if mode == "dash" {
        if !tugged.is_empty() && (t * 10.0) as i32 % 3 == 0 { keys.press(KeyCode::ShiftLeft) } else { keys.release(KeyCode::ShiftLeft) }
    }
    if mode == "cut" {
        cursor.0 = Some(Vec2::new(tk.body.pos.x - 50.0, k.body.pos.y));
        if (t * 6.0) as i32 % 2 == 0 { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    }
    if mode == "kill" && t > 2.0 && t > state.1 {
        hits.write(crate::combat::Hit { by: None, target: e, damage: 25.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at: tk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
        state.1 = t + 0.5;
    }
}

/// `necro` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a necromancer at
/// 1150; the player (who can't die) stands at 950 (`PLATYPUS_NECRO=`:
/// `soft`: the floor between them earth, so its skeletons claw up out of
/// it; `body`: an orc killed by it first, to be raised; `near`: the player
/// walks up to it (it blinks away); `kill`: struck till it dies at 9 s (what
/// it raised falls apart); `pack`: its zombies struck down at 7 s (it rests
/// before raising another pack)). Logs what it does, its zombies, where the
/// player is, the health lost.
#[allow(clippy::too_many_arguments)]
fn necro_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health, Has<crate::creatures::moves::Held>), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::custom::necro::Necro>), Without<LocalPlayer>>,
    raised: Query<(), With<crate::creatures::custom::necro::RaisedBy>>,
    bodies: Query<(), With<crate::hands::corpses::Corpse>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, f32, f32, &'static str, usize)>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
) {
    if s.name != "necro" {
        return;
    }
    let Ok((mut k, mut h, held)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR;
    let mode = std::env::var("PLATYPUS_NECRO").unwrap_or_default();
    state.2 += h.max - h.hp;
    h.hp = h.max;
    if state.0 == 0 && t > 0.5 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        if mode == "soft"
            && let (Some(dirt), Some(grass)) = (sim.materials().id("dirt"), sim.materials().id("grass"))
        {
            for x in 960..1150 {
                for y in fl - 24..fl {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: if y == fl - 1 { grass } else { dirt }, overwrite: true });
                }
            }
        }
        k.body.pos = Vec2::new(950.0, fl as f32 + k.body.half.y);
        k.prev_pos = k.body.pos;
        crate::creatures::def::spawn_creature(&mut commands, "necromancer", Vec2::new(1150.0, fl as f32), |_| {});
        if mode == "body" {
            crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(1110.0, fl as f32), |_| {});
        }
        info!("necro: a necromancer at 1150 ({mode})");
        state.0 = 1;
        return;
    }
    // `body`: the orc killed where it stands, at 1 s.
    if mode == "body" && state.0 == 1 && t > 1.0 {
        for (e, c, ok, _) in &them {
            if c.kind == "orc" {
                hits.write(crate::combat::Hit { by: None, target: e, damage: 999.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at: ok.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
            }
        }
        state.0 = 2;
    }
    let skeletons = them.iter().filter(|x| x.1.kind == "zombie").count();
    let up = raised.iter().count();
    if up != state.4 {
        info!("necro: t {t:.2} raised now {up} (zombies {skeletons}, bodies lying {})", bodies.iter().count());
        state.4 = up;
    }
    let Some((e, _, nk, o)) = them.iter().find(|x| x.1.kind == "necromancer") else {
        if state.0 < 9 && t > 2.0 {
            info!("necro: t {t:.1} the necromancer's dead; zombies {skeletons}, raised {up}");
            state.0 = 9;
            state.1 = t;
        }
        if state.0 == 9 && t > state.1 + 2.0 {
            info!("necro: t {t:.1} 2 s on: zombies standing {skeletons}, raised {up}");
            state.0 = 10;
        }
        return;
    };
    let doing = o.map_or("?", |o| o.doing());
    if doing != state.3 {
        info!("necro: t {t:.2} {doing} at ({:.0}, {:.0} up); skulls {}; the player at {:.0}{}, lost {:.0}", nk.body.pos.x, nk.body.pos.y - nk.body.half.y - fl as f32, o.map_or(0, |o| o.skulls()), k.body.pos.x, if held { " HELD" } else { "" }, state.2);
        state.3 = doing;
    }
    if mode == "near" {
        let toward = if nk.body.pos.x > k.body.pos.x { KeyCode::KeyD } else { KeyCode::KeyA };
        keys.release(KeyCode::KeyA);
        keys.release(KeyCode::KeyD);
        if t > 1.0 {
            keys.press(toward);
        }
    }
    // `pack`: its zombies struck down at 7 s (it rests before raising again).
    if mode == "pack" && (7.0..7.1).contains(&t) {
        for (ze, c, zk, _) in &them {
            if c.kind == "zombie" {
                hits.write(crate::combat::Hit { by: None, target: ze, damage: 999.0, harm: crate::creatures::Harm::Fire, knock: Vec2::ZERO, stun: 0.0, at: zk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
            }
        }
    }
    if mode == "kill" && t > 9.0 && t > state.1 {
        hits.write(crate::combat::Hit { by: None, target: e, damage: 40.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at: nk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
        state.1 = t + 0.3;
    }
}

/// `strikes` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): the pogo and the
/// dash attacks against the arena's dummies. `PLATYPUS_STRIKE=pogo`: the
/// shortsword; dropped from high over the middle dummy again and again,
/// striking down (S held, a fresh click) when its feet are 40, 32, 26, 20,
/// 14, 9, 5 cells over the dummy's top: which bounce. `PLATYPUS_STRIKE=`
/// `dash` (`PLATYPUS_WEAPON`: shortsword, longsword, fang, club): from
/// 70 cells off the first dummy, dash at it and strike as the dash goes;
/// three times. Logs the dummy's hurt, how high the player rose.
#[allow(clippy::too_many_arguments)]
fn strikes_script(
    s: Res<Scenario>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health, &crate::hands::items::Inventory), With<LocalPlayer>>,
    dummies: Query<(&Kinematics, &crate::creatures::custom::dummy::Tally), (With<crate::creatures::custom::dummy::Dummy>, Without<LocalPlayer>)>,
    (mut hand, items): (ResMut<crate::hands::Hand>, Option<Res<crate::hands::items::Items>>),
    (mut mouse, mut cursor, mut keys): (ResMut<ButtonInput<MouseButton>>, ResMut<CursorOverride>, ResMut<ButtonInput<KeyCode>>),
    mut state: Local<(u32, f32, f32, f32, bool, f32)>,
) {
    if s.name != "strikes" {
        return;
    }
    let Ok((mut k, mut h, inv)) = player.single_mut() else { return };
    h.hp = h.max;
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let mode = std::env::var("PLATYPUS_STRIKE").unwrap_or_else(|_| "pogo".into());
    let weapon = std::env::var("PLATYPUS_WEAPON").unwrap_or_else(|_| "shortsword".into());
    if t < 0.5 {
        return;
    }
    take_up(&mut hand, inv, items.as_deref(), if mode == "pogo" { "shortsword" } else { &weapon });
    let hurt = |x: f32| dummies.iter().find(|(dk, _)| (dk.body.pos.x - x).abs() < 20.0).map_or(0.0, |(_, tl)| tl.total);
    let put = |k: &mut Kinematics, at: Vec2| {
        k.body.pos = at;
        k.prev_pos = at;
        k.body.vel = Vec2::ZERO;
    };
    // (state: attempt, its start, the dummy's hurt then, highest the player
    // rose, struck yet, lowest it fell.)
    if mode == "pogo" {
        const AT: [f32; 7] = [40.0, 32.0, 26.0, 20.0, 14.0, 9.0, 5.0];
        let n = state.0 as usize;
        if n >= AT.len() {
            return;
        }
        let (dx, dtop) = dummies.iter().find(|(dk, _)| (dk.body.pos.x - 1140.0).abs() < 20.0).map_or((1140.0, fl + 20.0), |(dk, _)| (dk.body.pos.x, dk.body.pos.y + dk.body.half.y));
        if state.1 == 0.0 {
            let hy = k.body.half.y;
            put(&mut k, Vec2::new(dx, dtop + 70.0 + hy));
            state.1 = t;
            state.3 = f32::MIN;
            state.4 = false;
            return;
        }
        keys.press(KeyCode::KeyS);
        cursor.0 = Some(k.body.pos - Vec2::Y * 30.0);
        let feet = k.body.pos.y - k.body.half.y - dtop;
        let falling = k.body.vel.y < 0.0;
        if !state.4 && falling && feet <= AT[n] {
            state.4 = true;
            state.5 = t;
        }
        // (A click held a moment: a frame alone can fall between ticks.)
        if state.4 && t < state.5 + 0.05 {
            mouse.press(MouseButton::Left);
        } else {
            mouse.release(MouseButton::Left);
        }
        if state.4 && k.body.vel.y > 0.0 {
            state.3 = state.3.max(k.body.vel.y);
        }
        if state.4 && t > state.5 + 0.6 {
            info!("strikes: pogo struck at {:>2.0} over the dummy: {}", AT[n], if state.3 > 50.0 { "BOUNCED" } else { "missed" });
            state.0 += 1;
            state.1 = 0.0;
            keys.release(KeyCode::KeyS);
        }
        return;
    }
    // Dash attacks.
    let n = state.0;
    if n >= 3 {
        return;
    }
    if state.1 == 0.0 {
        let hy = k.body.half.y;
        put(&mut k, Vec2::new(980.0, fl + hy));
        state.1 = t;
        state.2 = hurt(1050.0);
        state.3 = 0.0;
        return;
    }
    let dt = t - state.1;
    cursor.0 = Some(Vec2::new(1060.0, fl + 14.0));
    if (0.3..0.45).contains(&dt) {
        keys.press(KeyCode::KeyD);
    } else {
        keys.release(KeyCode::KeyD);
    }
    if (0.32..0.36).contains(&dt) {
        keys.press(KeyCode::ShiftLeft);
    } else {
        keys.release(KeyCode::ShiftLeft);
    }
    if (0.38..0.42).contains(&dt) {
        mouse.press(MouseButton::Left);
    } else {
        mouse.release(MouseButton::Left);
    }
    state.3 = state.3.max(k.body.pos.y - k.body.half.y - fl);
    if dt > 1.6 {
        info!("strikes: {weapon} dash attack {}: the dummy took {:.0}, the player rose {:.0}", n + 1, hurt(1050.0) - state.2, state.3);
        state.0 += 1;
        state.1 = 0.0;
    }
}

/// `watcher` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a watcher at
/// 1150; the player (who can't die) stands at 950 (`PLATYPUS_WATCHER=`:
/// `flash`: a bright flash beside it at 5 s; `radiant`: a radiant blow at
/// 5 s (both blind it); `kill`: struck till it dies at 8 s (it bursts);
/// `lamp`: a light put beside it at 3 s (it squints, backs off, won't beam)).
/// Logs what it does, the player's health lost, the blight and its mist.
fn watcher_script(
    mut commands: Commands,
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::custom::watcher::Watcher>), Without<LocalPlayer>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut flares: MessageWriter<crate::fx::Flare>,
    mut state: Local<(u8, f32, f32, &'static str)>,
    mut second: Local<u32>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
) {
    if s.name != "watcher" {
        return;
    }
    // (The dev menu shut: no god mode. `PLATYPUS_KIND`: an elder.)
    menu.open = false;
    let kind = std::env::var("PLATYPUS_KIND").unwrap_or_else(|_| "watcher".into());
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let mode = std::env::var("PLATYPUS_WATCHER").unwrap_or_default();
    state.2 += h.max - h.hp;
    h.hp = h.max;
    let count = |name: &str| sim.materials().id(name).map_or(0, |m| (850..1300).flat_map(|x| (fl as i32 - 5..fl as i32 + 120).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).count());
    if state.0 == 0 && t > 0.5 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        k.body.pos = Vec2::new(950.0, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        crate::creatures::def::spawn_creature(&mut commands, &kind, Vec2::new(1150.0, fl), |_| {});
        info!("watcher: a {kind} at 1150 ({mode})");
        state.0 = 1;
        return;
    }
    let Some((e, _, wk, o)) = them.iter().find(|x| x.1.kind == kind) else {
        if state.0 == 1 && t > 1.0 {
            state.0 = 2;
            state.1 = t;
        }
        if state.0 == 2 && t > state.1 + 1.5 {
            info!("watcher: t {t:.1} dead: blight {} cells, its mist {}", count("blight"), count("blight_mist"));
            state.0 = 3;
        }
        return;
    };
    let doing = o.map_or("?", |o| o.doing());
    if t as u32 != *second {
        *second = t as u32;
        info!("watcher: t {t:.0} {:.0} up, {:.0} off", wk.body.pos.y - wk.body.half.y - fl, (wk.body.pos.x - k.body.pos.x).abs());
    }
    if doing != state.3 {
        info!("watcher: t {t:.2} {doing} at ({:.0}, {:.0} up); the player lost {:.0}; blight {}, mist {}, fog {}", wk.body.pos.x, wk.body.pos.y - wk.body.half.y - fl, state.2, count("blight"), count("blight_mist"), count("blight_fog"));
        state.3 = doing;
    }
    // `lamp`: a light put beside it at 3 s (it squints and backs off).
    if mode == "lamp" && state.0 == 1 && t > 3.0 {
        commands.spawn((Transform::from_translation((wk.body.pos + Vec2::new(-25.0, 0.0)).extend(0.0)), GlobalTransform::default(), crate::light::LightSource { color: [1.0, 0.85, 0.6], flicker: 0.0 }));
        info!("watcher: t {t:.1} a lamp beside it");
        state.0 = 4;
    }
    if (5.0..5.02).contains(&t) {
        match mode.as_str() {
            "flash" => {
                flares.write(crate::fx::Flare { at: wk.body.pos + Vec2::new(-20.0, 0.0), color: [2.0, 2.0, 1.8], life: 0.3 });
            }
            "radiant" => {
                hits.write(crate::combat::Hit { by: None, target: e, damage: 6.0, harm: crate::creatures::Harm::Radiant, knock: Vec2::ZERO, stun: 0.0, at: wk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
            }
            _ => {}
        }
    }
    if mode == "kill" && t > 8.0 && t > state.1 {
        hits.write(crate::combat::Hit { by: None, target: e, damage: 30.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at: wk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
        state.1 = t + 0.3;
    }
}

/// `gnasher` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a gnasher at
/// 1150; the player (who can't die) stands at 950 (`PLATYPUS_GNASHER=`:
/// `dodge`: they run back out of its line as its lunge locks; `near`: they
/// go at it (into snapping reach); `kill`: struck till it dies at 8 s). Logs what it does,
/// where, and the player's health lost.
fn gnasher_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::custom::gnasher::Gnasher>), Without<LocalPlayer>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, f32, f32, &'static str, f32, u32)>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    tugged: Query<(), (With<LocalPlayer>, With<crate::creatures::Tugged>)>,
) {
    if s.name != "gnasher" {
        return;
    }
    // (The dev menu shut: no god mode. `PLATYPUS_KIND`: an abyssal one.)
    menu.open = false;
    let kind = std::env::var("PLATYPUS_KIND").unwrap_or_else(|_| "gnasher".into());
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let mode = std::env::var("PLATYPUS_GNASHER").unwrap_or_default();
    let lost = h.max - h.hp;
    if lost > 0.0 {
        info!("gnasher: t {t:.2} bitten: {lost:.0}");
    }
    state.2 += lost;
    h.hp = h.max;
    if state.0 == 0 && t > 0.5 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        let x = if mode == "near" { 1110.0 } else { 950.0 };
        k.body.pos = Vec2::new(x, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        crate::creatures::def::spawn_creature(&mut commands, &kind, Vec2::new(1150.0, fl), |_| {});
        info!("gnasher: a {kind} at 1150 ({mode})");
        state.0 = 1;
        return;
    }
    let Some((e, _, gk, o)) = them.iter().find(|x| x.1.kind == kind) else {
        if state.0 == 1 && t > 1.0 {
            info!("gnasher: t {t:.1} dead; the player lost {:.0}", state.2);
            state.0 = 3;
        }
        return;
    };
    let doing = o.map_or("?", |o| o.doing());
    // (Its height, a second at a time: up out of reach, down now and then.)
    if t as u32 != state.5 {
        state.5 = t as u32;
        info!("gnasher: t {t:.0} {:.0} up, {:.0} off; the player at {:.0}{}", gk.body.pos.y - gk.body.half.y - fl, (gk.body.pos.x - k.body.pos.x).abs(), k.body.pos.x, if tugged.is_empty() { "" } else { ", drawn in" });
    }
    if doing != state.3 {
        info!("gnasher: t {t:.2} {doing} at ({:.0}, {:.0} up); the player at {:.0}, lost {:.0}", gk.body.pos.x, gk.body.pos.y - gk.body.half.y - fl, k.body.pos.x, state.2);
        if doing == "draw" {
            state.4 = t;
        }
        state.3 = doing;
    }
    // `dodge`: as its lunge locks, they run back, out of its line (it
    // slants down into the ground where they stood).
    if mode == "dodge" && doing == "draw" && t > state.4 + 0.7 && t < state.4 + 1.1 {
        k.body.pos.x -= 2.5;
    }
    // `near`: they go at it (into its snapping reach).
    if mode == "near" && doing == "hover" && (gk.body.pos.x - k.body.pos.x).abs() > 26.0 {
        k.body.pos.x += 1.6 * (gk.body.pos.x - k.body.pos.x).signum();
    }
    if mode == "kill" && t > 8.0 && t > state.1 {
        hits.write(crate::combat::Hit { by: None, target: e, damage: 30.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at: gk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
        state.1 = t + 0.3;
    }
}

/// `bats` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a stone roof over
/// the floor from 850 to 1350, 76 up; seven cave bats (`PLATYPUS_BAT_COUNT`)
/// put under it at
/// 1080..1200 (they fly up and roost); the player (who can't die) at 800,
/// out of their hearing. `PLATYPUS_BATS=`: (none) at 6 s the player walks
/// in under them (someone near wakes one; its screech the rest);
/// `mine`: the player stands at 1030 (near, not near enough) and at 5 s a
/// pick strikes stone there; `still`: nothing (they should sleep on);
/// `flash`: as (none), and at 12 s a flash among the swarm (they drop).
/// `PLATYPUS_BAT_ROOF`: the roof this high (80; 40 is a tunnel).
/// Logs, a second at a time, what the bats are doing and what the player's
/// lost.
fn bats_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::custom::cave_bat::CaveBat>), Without<LocalPlayer>>,
    (mut sounds, mut flares): (MessageWriter<crate::sound::PlaySound>, MessageWriter<crate::fx::Flare>),
    mut state: Local<(u8, f32, u32)>,
) {
    if s.name != "bats" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let mode = std::env::var("PLATYPUS_BATS").unwrap_or_default();
    state.1 += h.max - h.hp;
    h.hp = h.max;
    if state.0 == 0 && t > 0.3 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        let stone = sim.materials().expect_id("stone");
        let roof: i32 = std::env::var("PLATYPUS_BAT_ROOF").ok().and_then(|v| v.parse().ok()).unwrap_or(80);
        for x in (850..=1350).step_by(5) {
            sim.queue(platypus_sim::WorldEdit::Paint { center: CellPos::new(x, fl as i32 + roof), radius: 6, material: stone, overwrite: true });
        }
        state.0 = 1;
        return;
    }
    if state.0 == 1 && t > 0.6 {
        let x = if mode == "mine" { 1030.0 } else { 800.0 };
        k.body.pos = Vec2::new(x, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        let many: u32 = std::env::var("PLATYPUS_BAT_COUNT").ok().and_then(|v| v.parse().ok()).unwrap_or(7);
        let roof: f32 = std::env::var("PLATYPUS_BAT_ROOF").ok().and_then(|v| v.parse().ok()).unwrap_or(80.0);
        for n in 0..many {
            crate::creatures::def::spawn_creature(&mut commands, "cave_bat", Vec2::new(1080.0 + n as f32 * 20.0, fl + (roof - 35.0).clamp(12.0, 45.0)), |_| {});
        }
        info!("bats: seven cave bats under the roof ({mode})");
        state.0 = 2;
        return;
    }
    if state.0 < 2 {
        return;
    }
    // (The player walks in under them, a little at a time.)
    if (mode.is_empty() || mode == "flash") && t > 6.0 && k.body.pos.x < 1120.0 {
        k.body.pos.x += 1.2;
    }
    if mode == "mine" && (5.0..5.02).contains(&t) {
        sounds.write(crate::sound::PlaySound::at("mine_stone", k.body.pos + Vec2::new(8.0, 0.0)));
        info!("bats: t {t:.2} a pick strikes stone at {:.0}", k.body.pos.x);
    }
    let bats: Vec<_> = them.iter().filter(|x| x.1.kind == "cave_bat").collect();
    if mode == "flash" && (12.0..12.02).contains(&t) {
        let mid = bats.iter().map(|x| x.2.body.pos).sum::<Vec2>() / bats.len().max(1) as f32;
        flares.write(crate::fx::Flare { at: mid, color: [2.0, 2.0, 1.8], life: 0.3 });
        info!("bats: t {t:.2} a flash at ({:.0}, {:.0} up)", mid.x, mid.y - fl);
    }
    if t as u32 != state.2 {
        state.2 = t as u32;
        let mut tally: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
        for (_, _, bk, o) in &bats {
            let what = if bk.loco.state == platypus_physics::MoveState::Stunned { "senseless" } else { o.map_or("?", |o| o.doing()) };
            *tally.entry(what).or_default() += 1;
        }
        let list: Vec<String> = tally.iter().map(|(k, n)| format!("{k} {n}")).collect();
        let high = bats.iter().map(|x| x.2.body.pos.y - fl).fold(0.0f32, f32::max);
        // (Any in the rock, or pressed against it while swarming.)
        let grid = crate::creatures::WorldGrid(&sim.world);
        let inside = bats.iter().filter(|x| {
            let (lo, hi) = x.2.body.cells_at(x.2.body.pos);
            platypus_physics::body::overlaps_solid(&grid, lo, hi)
        }).count();
        let pressed = bats.iter().filter(|x| { let c = &x.2.loco.contacts; c.wall_left || c.wall_right || (c.ceiling && x.3.is_some_and(|o| !matches!(o.doing(), "roost" | "settle" | "stir"))) }).count();
        info!("bats: t {t:.0} {} (highest {high:.0} up); in rock {inside}, against it {pressed}; the player at {:.0}, lost {:.0}", list.join(", "), k.body.pos.x, state.1);
    }
}

/// `headshot` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a zombie, then
/// an orc (`PLATYPUS_HEADSHOT=orc`), put at 1000, held still; the player at
/// 880 casts a spark bolt every second, level, at the part of its head
/// drawn above its box (where only its picture is). Logs where it aimed,
/// how far above the box, and the health it lost.
fn headshot_script(
    mut commands: Commands,
    s: Res<Scenario>,
    book: Res<crate::magic::Spellbook>,
    mut player: Query<(Entity, &mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    mut them: Query<(Entity, &crate::creatures::Creature, &mut Kinematics, &crate::creatures::Health, Option<&crate::creatures::body::animation::Animator>), Without<LocalPlayer>>,
    mut casts: MessageWriter<crate::magic::CastRequest>,
    mut state: Local<(u8, f32, f32)>,
) {
    if s.name != "headshot" {
        return;
    }
    let Ok((me, mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let kind = std::env::var("PLATYPUS_HEADSHOT").unwrap_or_else(|_| "zombie".into());
    h.hp = h.max;
    if state.0 == 0 && t > 0.5 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        k.body.pos = Vec2::new(880.0, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        crate::creatures::def::spawn_creature(&mut commands, &kind, Vec2::new(1000.0, fl), |_| {});
        info!("headshot: a {kind} at 1000");
        state.0 = 1;
        return;
    }
    let Some((_, _, mut tk, th, anim)) = them.iter_mut().find(|x| x.1.kind == kind) else { return };
    // (Held still where it stands.)
    tk.body.pos.x = 1000.0;
    tk.body.vel = Vec2::ZERO;
    if state.0 == 1 {
        state.1 = th.hp;
        state.0 = 2;
    }
    if t > 2.0 && t > state.2 {
        state.2 = t + 1.0;
        let top = tk.body.pos.y + tk.body.half.y;
        // The highest point of its picture over its middle column, above
        // its box.
        let head = anim.and_then(|a| (1..30).rev().map(|dy| Vec2::new(tk.body.pos.x, top + dy as f32)).find(|&p| crate::creatures::body::animation::on_picture(a, &tk, p)));
        let Some(head) = head else {
            info!("headshot: t {t:.0} nothing of its picture above its box");
            return;
        };
        let aim = head - Vec2::Y * 1.0;
        let Some(i) = book.spells.iter().position(|s| s.id == "spark_bolt") else { return };
        casts.write(crate::magic::CastRequest { caster: me, spell: i, from: Vec2::new(930.0, aim.y), toward: Vec2::new(1100.0, aim.y), alt: false });
        info!("headshot: t {t:.0} a bolt at {:.0} up ({:.1} over its box); it's lost {:.0} so far", aim.y - fl, aim.y - top, state.1 - th.hp);
    }
}

/// `keepsake` (a save: `PLATYPUS_SAVE=...`): what the player has on them,
/// logged at 1, 3 and 6 s: the hotbar in hand, the first slots of the
/// pack, what's worn, gold, health, the discard slot. To see a save's
/// player come back as it was saved. `PLATYPUS_KEEPSAKE=hold`: at 2 s the
/// pack's first stack is picked up onto the cursor and its second thrown
/// in the discard slot, the hotbar's third slot taken in hand, and it's
/// saved (then run it again without, to see them come back).
#[allow(clippy::type_complexity)]
fn keepsake_script(
    s: Res<Scenario>,
    items: Option<Res<crate::hands::items::Items>>,
    (mut hand, mut held): (ResMut<crate::hands::Hand>, ResMut<crate::hands::ui::Held>),
    mut player: Query<(&Kinematics, &crate::creatures::Health, Option<&mut crate::hands::items::Inventory>, &crate::gear::Equipment, Option<&crate::gold::Gold>), With<LocalPlayer>>,
    mut saves: MessageWriter<crate::save::SaveNow>,
    mut next: Local<u8>,
    mut done: Local<bool>,
) {
    if s.name != "keepsake" {
        return;
    }
    if std::env::var("PLATYPUS_KEEPSAKE").as_deref() == Ok("hold")
        && s.elapsed > 2.0
        && !*done
        && let Ok((_, _, Some(mut pack), ..)) = player.single_mut()
    {
        *done = true;
        held.stack = pack.slots[0].take();
        held.set_trash(pack.slots[1].take());
        hand.slot = 2;
        saves.write(crate::save::SaveNow);
        info!("keepsake: t 2 picked up the first stack, threw the second away, the third slot in hand; saved");
    }
    let at = [1.0, 3.0, 6.0];
    if *next as usize >= at.len() || s.elapsed < at[*next as usize] {
        return;
    }
    *next += 1;
    let (Some(items), Ok((k, h, pack, eq, gold))) = (items, player.single()) else { return };
    let name = |st: &Option<crate::hands::items::Stack>| st.map_or("-".to_string(), |st| format!("{}×{}", items.def(st.item).id, st.count));
    let pack: Vec<String> = pack.map_or(vec![], |p| p.slots.iter().take(20).map(name).collect());
    let worn: Vec<String> = eq.worn.iter().map(name).collect();
    let (cursor, trash) = held.kept();
    info!("keepsake: t {:.0} at ({:.0}, {:.0}), hp {:.0}, gold {}, hand slot {} bar {}; cursor {}, discard {}", s.elapsed, k.body.pos.x, k.body.pos.y, h.hp, gold.map_or(0, |g| g.0), hand.slot, hand.bar, name(&cursor), name(&trash));
    info!("keepsake:   pack {}", pack.join(" "));
    info!("keepsake:   worn {}", worn.join(" "));
}

/// `rats` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): rats at 1100; the
/// player (who can't die) at 950. `PLATYPUS_RATS=`: (none) a pack of five
/// (they come at you), and from 7 s one struck dead a second; `alone`:
/// two (they come at you too); `body`: a
/// zombie killed at 1100, four rats at 1250, the player far off at 700
/// (they go to the body and gnaw it till it's gone, what it had left
/// lying); `idle`: three far from the player (skittish, on their own);
/// `swarm`: five right by the player (two at it at a time, the rest
/// circling).
/// On the ground whatever the arena (`PLATYPUS_ARENA=slopes`, `cave`).
/// Logs, a second at a time, what the rats are doing, how many are at the
/// player at once, the bodies and what's lying about, and what the player's
/// lost.
/// `PLATYPUS_RAT`: the kind (`rat_lithe`: the one that bends).
#[allow(clippy::type_complexity)]
fn rats_script(
    mut commands: Commands,
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::custom::rat::Rat>), Without<LocalPlayer>>,
    bodies: Query<(), With<crate::hands::corpses::Corpse>>,
    drops: Query<(), With<crate::hands::Dropped>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, f32, u32, f32)>,
    mut biting: Local<(usize, usize)>,
) {
    if s.name != "rats" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let mode = std::env::var("PLATYPUS_RATS").unwrap_or_default();
    let kind = std::env::var("PLATYPUS_RAT").unwrap_or_else(|_| "rat".into());
    state.1 += h.max - h.hp;
    h.hp = h.max;
    if state.0 == 0 && t > 0.5 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        // (On the ground there, whatever the arena: flat, slopes, a cave.)
        let ground = |x: f32| find_ground(&sim.world, x as i32, fl as i32 + 300, 600).map_or(fl, |y| y as f32);
        let x = match mode.as_str() { "body" => 700.0, "idle" => 900.0, _ => 950.0 };
        k.body.pos = Vec2::new(x, ground(x) + k.body.half.y);
        k.prev_pos = k.body.pos;
        let (n, at) = match mode.as_str() {
            "alone" => (2, 1100.0),
            "swarm" => (5, 990.0),
            "body" => (4, 1250.0),
            "idle" => (3, 1180.0),
            _ => (5, 1100.0),
        };
        for i in 0..n {
            let rx = at + i as f32 * 40.0;
            crate::creatures::def::spawn_creature(&mut commands, &kind, Vec2::new(rx, ground(rx) + 2.0), |_| {});
        }
        if mode == "body" {
            crate::creatures::def::spawn_creature(&mut commands, "zombie", Vec2::new(1100.0, fl), |_| {});
        }
        info!("rats: {n} rats at {at} ({mode})");
        state.0 = 1;
        return;
    }
    if state.0 < 1 {
        return;
    }
    // `body`: the zombie struck dead.
    if mode == "body" && state.0 == 1 && t > 1.0 {
        if let Some((e, _, zk, _)) = them.iter().find(|x| x.1.kind == "zombie") {
            hits.write(crate::combat::Hit { by: None, target: e, damage: 999.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at: zk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
            info!("rats: t {t:.1} the zombie struck dead");
            state.0 = 2;
        }
    }
    // (none): from 7 s, a rat struck dead a second, down to two.
    if mode.is_empty() && t > 7.0 && t > state.3 {
        let rats: Vec<_> = them.iter().filter(|x| x.1.kind == kind).collect();
        if rats.len() > 2 {
            let (e, _, rk, _) = rats[0];
            hits.write(crate::combat::Hit { by: None, target: e, damage: 99.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at: rk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
            state.3 = t + 1.0;
            info!("rats: t {t:.1} a rat struck dead ({} left)", rats.len() - 1);
        }
    }
    // (How many at it at once: within a bite of it, now and the most yet.)
    let at_it = them.iter().filter(|(_, c, rk, _)| c.kind == kind && rk.body.pos.distance(k.body.pos) < 18.0).count();
    biting.1 = biting.1.max(at_it);
    biting.0 = biting.0.max(at_it);
    if t as u32 != state.2 {
        state.2 = t as u32;
        info!("rats: t {t:.0} at the player at once: {} this second (most yet {})", biting.0, biting.1);
        biting.0 = 0;
        let mut tally: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
        let mut far = 0.0f32;
        for (_, c, rk, o) in &them {
            if c.kind == kind {
                *tally.entry(o.map_or("?", |o| o.doing())).or_default() += 1;
                far = far.max((rk.body.pos.x - k.body.pos.x).abs());
            }
        }
        let list: Vec<String> = tally.iter().map(|(k, n)| format!("{k} {n}")).collect();
        info!("rats: t {t:.0} {} (furthest {far:.0} off); bodies {}, lying {}; the player lost {:.0}", list.join(", "), bodies.iter().count(), drops.iter().count(), state.1);
    }
}

/// `volume` (`PLATYPUS_WORLD=arena`): the game menu opened, then its volume
/// slider pressed as a hand would: the sixth step (60%), − (50%), + twice
/// (70%). Logs the setting, the main bus's volume, what settings.ron says.
fn volume_script(
    s: Res<Scenario>,
    mut menu: ResMut<crate::pause::GameMenu>,
    mut virt: ResMut<Time<Virtual>>,
    mut buttons: Query<(&mut Interaction, &crate::pause::Volume)>,
    settings: Res<crate::settings::Settings>,
    mut done: Local<u8>,
) {
    if s.name != "volume" {
        return;
    }
    let t = s.elapsed;
    let steps = [(1.0, None), (1.5, Some(crate::pause::Volume::Step(6))), (2.0, Some(crate::pause::Volume::Less)), (2.5, Some(crate::pause::Volume::More)), (3.0, Some(crate::pause::Volume::More)), (3.5, None)];
    let Some(&(at, press)) = steps.get(*done as usize) else { return };
    if t < at {
        return;
    }
    *done += 1;
    match press {
        None if *done == 1 => {
            crate::pause::open(&mut menu, &mut virt);
            info!("volume: the menu open; volume {}%", settings.volume as u32 * 10);
        }
        None => {
            let file = std::fs::read_to_string(crate::save::saves_dir().join("settings.ron")).unwrap_or_default();
            info!("volume: at the end {}%; settings.ron: {}", settings.volume as u32 * 10, file.replace('\n', " "));
        }
        Some(which) => {
            info!("volume: {:?}% before pressing", settings.volume as u32 * 10);
            for (mut i, v) in &mut buttons {
                if *v == which {
                    *i = Interaction::Pressed;
                }
            }
        }
    }
}

/// `worldmap` (a generated world, by day): the player carried along the
/// surface from the start, 300 cells at a time, to chart it; then the map
/// opened on them: at 0.5 (7 s), 2 (8 s), 0.125 (9 s); the fog off (dev
/// view) from 10 s, at 0.125, then at 0.5 a little down (15 s), then at 2
/// (16 s). `PLATYPUS_SHOTS` with `PLATYPUS_SHOT_EVERY=0.5` to see each
/// (the half seconds: its labels come a frame after it opens). Logs the
/// tiles charted.
fn worldmap_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    mut screen: ResMut<crate::map::MapScreen>,
    map: Res<crate::map::WorldMap>,
    mut state: Local<(u32, Option<Vec2>)>,
) {
    if s.name != "worldmap" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let t = s.elapsed;
    let start = *state.1.get_or_insert(k.body.pos);
    // Along the surface, a step every 0.6 s.
    let step = ((t - 0.5) / 0.6).floor() as i32;
    if (0..10).contains(&step) && step as u32 >= state.0 {
        state.0 = step as u32 + 1;
        let x = start.x + (step - 3) as f32 * 300.0;
        let y = sim.generator.surface_hint(x as i32).unwrap_or(start.y as i32) as f32 + 30.0;
        k.body.pos = Vec2::new(x, y);
        k.prev_pos = k.body.pos;
        k.body.vel = Vec2::ZERO;
    }
    let at = |t0: f32| (t0..t0 + 0.02).contains(&t);
    if at(7.0) {
        screen.show(k.body.pos, 0.5);
        info!("worldmap: t 7 open at 0.5; {} tiles charted", map.tiles.len());
    }
    if at(8.0) {
        screen.show(k.body.pos, 2.0);
        info!("worldmap: t 8 at 2");
    }
    if at(9.0) {
        screen.show(k.body.pos, 0.125);
        info!("worldmap: t 9 at 0.125");
    }
    if at(10.0) {
        screen.fog_off = true;
        screen.show(k.body.pos, 0.125);
        info!("worldmap: t 10 the fog off");
    }
    if at(15.0) {
        screen.show(k.body.pos - Vec2::new(0.0, 300.0), 0.5);
        info!("worldmap: t 15 at 0.5, a little down");
    }
    if at(16.0) {
        screen.show(k.body.pos - Vec2::new(0.0, 300.0), 2.0);
        info!("worldmap: t 16 at 2");
        // Where a click would take you: in the rock 300 down, in the sky
        // 200 up, and out over the world's far edge.
        for (what, to) in [("300 down, in rock", k.body.pos - Vec2::new(0.0, 300.0)), ("200 up, in the sky", k.body.pos + Vec2::new(0.0, 200.0)), ("200 along", k.body.pos + Vec2::new(200.0, 0.0))] {
            let room = crate::map::standing_room(&sim, to, k.body.half, 400);
            info!("worldmap: a click {what} ({:.0}, {:.0}) takes you to {:?}", to.x, to.y, room.map(|r| (r.x.round(), r.y.round(), (r - to).length().round())));
        }
    }
}

/// `ropehook` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): the rope hook
/// worn. A stone ledge from 600 to 900, its top 200 up; a stone knob at
/// (1080, 100 up). The player on the ledge by its lip: hooks the lip,
/// walks off, rappels down (S) to the floor; climbs back up (W); then, on
/// the floor at 1000, tosses it at the knob (an arc that should land on
/// it), and straight up at 320 (out of reach: it rises, falls back).
/// Logs height a quarter second at a time, the rope's state, where the
/// tosses landed.
#[allow(clippy::too_many_arguments)]
fn ropehook_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    mut cursor: ResMut<CursorOverride>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut player: Query<(&mut Kinematics, &mut crate::gear::Equipment, Option<&crate::gear::hook::Rope>, &mut crate::creatures::Health), With<LocalPlayer>>,
    mut state: Local<(u8, f32, u32, Vec2)>,
) {
    if s.name != "ropehook" {
        return;
    }
    let Ok((mut k, mut eq, rope, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let (what, _) = rope.map_or(("none", 0), |r| r.state());
    let height = k.body.pos.y - k.body.half.y - fl;
    let dt = t - state.1;
    let mut held: Vec<KeyCode> = Vec::new();
    let put = |k: &mut Kinematics, at: Vec2| {
        k.body.pos = at + Vec2::Y * k.body.half.y;
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
    };
    let quarter = (dt * 4.0) as u32;
    let x = k.body.pos.x;
    let log = |label: &str, state: &mut (u8, f32, u32, Vec2)| {
        if quarter != state.2 {
            state.2 = quarter;
            info!("ropehook: {label} t {dt:.2}: {height:.0} up at {x:.0}, the rope {what}");
        }
    };
    match state.0 {
        0 if t > 0.5 => {
            if let Some(stone) = sim.materials().id("stone") {
                for x in (600..=896).step_by(4) {
                    for y in [fl as i32 + 190, fl as i32 + 196] {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 4, material: stone, overwrite: true });
                    }
                }
                sim.queue(WorldEdit::Paint { center: CellPos::new(1080, fl as i32 + 100), radius: 4, material: stone, overwrite: true });
            }
            if let (Some(items), Some(slot)) = (items.as_ref(), crate::gear::WORN.iter().position(|w| *w == crate::gear::Slot::Hook))
                && let Some(id) = items.id("rope_hook")
            {
                eq.worn[slot] = Some(crate::hands::items::Stack::new(id, 1));
            }
            *state = (1, t, 0, Vec2::ZERO);
        }
        // On the ledge by its lip; hook the lip below the feet.
        1 if dt > 0.4 => {
            if dt < 0.5 {
                put(&mut k, Vec2::new(880.0, fl + 201.0));
            }
            cursor.0 = Some(Vec2::new(898.0, fl + 198.0));
            if (0.7..0.8).contains(&dt) {
                held.push(KeyCode::KeyE);
            }
            if dt > 1.4 {
                info!("ropehook: hooked the lip: the rope {what}, {height:.0} up");
                *state = (2, t, 0, Vec2::ZERO);
            }
        }
        // Off the edge, then S: rappel down.
        2 => {
            if dt < 0.35 {
                held.push(KeyCode::KeyD);
            }
            if dt > 0.6 {
                held.push(KeyCode::KeyS);
            }
            log("rappel", &mut state);
            if dt > 3.4 {
                *state = (3, t, 0, Vec2::ZERO);
            }
        }
        // W: climb back up.
        3 => {
            held.push(KeyCode::KeyW);
            log("climb", &mut state);
            if dt > 3.4 {
                *state = (4, t, 0, Vec2::ZERO);
            }
        }
        // Let go (jump), on the floor at 1000; toss at the knob.
        4 => {
            if dt < 0.05 {
                held.push(KeyCode::Space);
            }
            if (0.3..0.35).contains(&dt) {
                put(&mut k, Vec2::new(1000.0, fl + 1.0));
            }
            cursor.0 = Some(Vec2::new(1080.0, fl + 100.0));
            // (Held a while, its arc shown, before it's let go.)
            if (0.8..1.6).contains(&dt) {
                held.push(KeyCode::KeyE);
            }
            if dt > 2.8 {
                let held = rope.and_then(|r| r.tether()).map(|(at, _)| (at.x.round(), (at.y - fl).round()));
                info!("ropehook: tossed at the knob (1080, 100 up) from (1000, the floor): the rope {what}, holding at {held:?}");
                *state = (5, t, 0, Vec2::ZERO);
            }
        }
        // Straight up at 320: out of reach.
        5 => {
            if dt < 0.05 {
                held.push(KeyCode::Space);
            }
            if (0.3..0.35).contains(&dt) {
                put(&mut k, Vec2::new(1500.0, fl + 1.0));
            }
            cursor.0 = Some(Vec2::new(1500.0, fl + 320.0));
            if (0.8..0.9).contains(&dt) {
                held.push(KeyCode::KeyE);
            }
            log("up", &mut state);
            if dt > 2.6 {
                let held = rope.and_then(|r| r.tether()).map(|(at, _)| (at.x.round(), (at.y - fl).round()));
                info!("ropehook: tossed straight up at 320: the rope {what}, holding at {held:?}");
                *state = (6, t, 0, Vec2::ZERO);
            }
        }
        _ => {}
    }
    for key in [KeyCode::KeyE, KeyCode::KeyD, KeyCode::KeyS, KeyCode::KeyW, KeyCode::Space] {
        match (held.contains(&key), keys.pressed(key)) {
            (true, false) => keys.press(key),
            (false, true) => keys.release(key),
            _ => {}
        }
    }
}

/// `loadouts` (any world): each loadout handed out in turn, a second
/// apart, as the dev menu does; logs the hotbar, the rest of the pack and
/// what's worn after each, and what there wasn't room for.
fn loadouts_script(
    s: Res<Scenario>,
    items: Option<Res<crate::hands::items::Items>>,
    mut player: Query<(&mut crate::hands::items::Inventory, &mut crate::gear::Equipment), With<LocalPlayer>>,
    mut next: Local<usize>,
) {
    if s.name != "loadouts" {
        return;
    }
    let (Some(items), Ok((mut inv, mut eq))) = (items, player.single_mut()) else { return };
    let loadouts: Vec<String> = items.kits.iter().filter(|k| k.loadout).map(|k| k.name.clone()).collect();
    if *next >= loadouts.len() || s.elapsed < 1.0 + *next as f32 {
        return;
    }
    let name = &loadouts[*next];
    *next += 1;
    let over = crate::hands::hand_out(&items, items.kit(name), &mut inv, &mut eq);
    let show = |st: &Option<crate::hands::items::Stack>| st.map(|st| format!("{}×{}", items.def(st.item).id, st.count));
    let bar: Vec<String> = inv.slots[..crate::hands::items::HOTBAR].iter().map(|st| show(st).unwrap_or("-".into())).collect();
    let pack: Vec<String> = inv.slots[crate::hands::items::HOTBAR..].iter().filter_map(show).collect();
    let worn: Vec<String> = eq.worn.iter().filter_map(show).collect();
    info!("loadouts: {name}: hotbar {}", bar.join(" "));
    info!("loadouts: {name}: pack {}", pack.join(" "));
    info!("loadouts: {name}: worn {}; no room for {}", worn.join(" "), over.len());
}

/// The jump test's start: its clock set on its first look (from 0.5 s).
fn i_first(state: &mut (usize, f32, f32, f32), t: f32) -> bool {
    if t < 0.5 {
        return true;
    }
    if state.1 == 0.0 {
        state.1 = t;
    }
    false
}

/// `jumps` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a full jump (jump
/// held), with no boots, feather boots, then Zephyr treads, each air jump
/// at the top of the rise before; logs the highest each reached.
fn jumps_script(
    s: Res<Scenario>,
    items: Option<Res<crate::hands::items::Items>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut player: Query<(&mut Kinematics, &mut crate::gear::Equipment, &crate::creatures::MoveStats), With<LocalPlayer>>,
    mut state: Local<(usize, f32, f32, f32)>,
) {
    if s.name != "jumps" {
        return;
    }
    let (Some(items), Ok((mut k, mut eq, stats))) = (items, player.single_mut()) else { return };
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let t = s.elapsed;
    let boots = [None, Some("feather_boots"), Some("cloud_boots")];
    let feet = crate::gear::WORN.iter().position(|w| *w == crate::gear::Slot::Feet).expect("feet");
    if i_first(&mut state, t) {
        return;
    }
    let (i, start) = (state.0, state.1);
    if i >= boots.len() {
        return;
    }
    let dt = t - start;
    if dt < 0.05 {
        eq.worn[feet] = boots[i].and_then(|b| items.id(b)).map(|id| crate::hands::items::Stack::new(id, 1));
        k.body.pos = Vec2::new(900.0 + i as f32 * 150.0, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        k.body.vel = Vec2::ZERO;
        state.2 = 0.0;
    }
    let height = k.body.pos.y - k.body.half.y - fl;
    state.2 = state.2.max(height);
    // Jump held from 0.6 s; let go at the top of each rise, pressed again
    // the next frame (an air jump).
    // (Let go a few frames: a tick must see it, or the next press isn't one.)
    let rising = k.body.vel.y > 0.0;
    if (0.6..3.0).contains(&dt) {
        if !keys.pressed(KeyCode::Space) {
            if t > state.3 {
                keys.press(KeyCode::Space);
            }
        } else if !rising && dt > 0.75 {
            keys.release(KeyCode::Space);
            state.3 = t + 0.06;
        }
    } else if keys.pressed(KeyCode::Space) {
        keys.release(KeyCode::Space);
    }
    if dt > 3.4 {
        info!("jumps: {}: highest {:.0} cells (jump height {:.0}, air jumps {})", boots[i].unwrap_or("no boots"), state.2, stats.0.jump_height, stats.0.air_jumps);
        *state = (i + 1, t, 0.0, 0.0);
    }
}

/// `stormboots` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`, by night):
/// storm boots worn; three zombies on the floor at 1000..1060 (held
/// still), the player at 960: at 1 s they jump, and at the top, a double
/// jump (its storm). `PLATYPUS_STORMBOOTS=empty`: no zombies (the bolts
/// go to ground). Logs the zombies' health after; then, landed, a dash
/// (its dust).
fn stormboots_script(
    mut commands: Commands,
    s: Res<Scenario>,
    items: Option<Res<crate::hands::items::Items>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut player: Query<(&mut Kinematics, &mut crate::gear::Equipment, &crate::creatures::MoveStats), With<LocalPlayer>>,
    mut them: Query<(Entity, &crate::creatures::Creature, &mut Kinematics, &crate::creatures::Health), Without<LocalPlayer>>,
    mut state: Local<(u8, bool, f32)>,
) {
    if s.name != "stormboots" {
        return;
    }
    let (Some(items), Ok((mut k, mut eq, stats))) = (items, player.single_mut()) else { return };
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let t = s.elapsed;
    let empty = std::env::var("PLATYPUS_STORMBOOTS").as_deref() == Ok("empty");
    if state.0 == 0 && t > 0.3 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        let feet = crate::gear::WORN.iter().position(|w| *w == crate::gear::Slot::Feet).expect("feet");
        eq.worn[feet] = items.id("storm_boots").map(|id| crate::hands::items::Stack::new(id, 1));
        k.body.pos = Vec2::new(960.0, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        if !empty {
            for x in [1000.0, 1030.0, 1060.0] {
                crate::creatures::def::spawn_creature(&mut commands, "zombie", Vec2::new(x, fl), |_| {});
            }
        }
        state.0 = 1;
    }
    // (Held still.)
    for (_, c, mut zk, _) in &mut them {
        if c.kind == "zombie" {
            zk.body.vel.x = 0.0;
        }
    }
    // Jump at 1 s; at the top, let go (a few frames: a tick must see it)
    // and jump again (the double jump).
    if state.0 == 1 && t > 1.0 {
        let rising = k.body.vel.y > 0.0;
        if t < 1.2 || (rising && !state.1) {
            keys.press(KeyCode::Space);
        } else if !state.1 {
            keys.release(KeyCode::Space);
            state.1 = true;
            state.2 = t;
        } else if t > state.2 + 0.06 && !keys.pressed(KeyCode::Space) {
            keys.press(KeyCode::Space);
            state.0 = 2;
            info!("stormboots: t {t:.2} the double jump, {:.0} up (jump height {:.0}, air jumps {})", k.body.pos.y - k.body.half.y - fl, stats.0.jump_height, stats.0.air_jumps);
        }
    }
    // (Landed: a dash to the right, its dust thrown back.)
    if state.0 == 3 && (3.6..3.9).contains(&t) {
        keys.press(KeyCode::KeyD);
        if t > 3.65 {
            keys.press(KeyCode::ShiftLeft);
        }
    } else if state.0 == 3 && t > 3.9 {
        keys.release(KeyCode::KeyD);
        keys.release(KeyCode::ShiftLeft);
    }
    if state.0 == 2 && t > 3.2 {
        keys.release(KeyCode::Space);
        let hp: Vec<String> = them.iter().filter(|x| x.1.kind == "zombie").map(|x| format!("{:.0}/{:.0} at {:.0}", x.3.hp, x.3.max, x.2.body.pos.x)).collect();
        info!("stormboots: the zombies after: {}", if hp.is_empty() { "none".into() } else { hp.join(", ") });
        state.0 = 3;
    }
}

/// `layers` (a generated world; `PLATYPUS_LIFE_RATE=30` to hurry life):
/// the player (who can't die) put in a cave in the middle of each depth
/// band below the ground in turn (`PLATYPUS_LAYER_SECS`, 10, in each);
/// logs what came round them in each (within 700 cells).
#[allow(clippy::type_complexity)]
fn layers_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(&crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    mut state: Local<(usize, f32, Option<Vec2>)>,
) {
    if s.name != "layers" {
        return;
    }
    const BANDS: [&str; 4] = ["underground", "caverns", "deep", "underworld"];
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let t = s.elapsed;
    let dwell = std::env::var("PLATYPUS_LAYER_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(10.0);
    if state.0 >= BANDS.len() || t < 0.5 {
        return;
    }
    let band = BANDS[state.0];
    match state.2 {
        None => {
            // Its middle (by the bands' names, down a column by the start),
            // then a cave's air there with room.
            let x = sim.generator.spawn_point().x + 400;
            let ys: Vec<i32> = (0..30000).step_by(8).map(|d| sim.generator.spawn_point().y - d).filter(|&y| sim.generator.band_hint(y) == Some(band)).collect();
            let Some(&mid) = ys.get(ys.len() / 2) else {
                warn!("layers: no {band}");
                *state = (state.0 + 1, t, None);
                return;
            };
            let air = |x: i32, y: i32| sim.generator.material_hint(x, y).is_some_and(|m| m == platypus_sim::MaterialId::AIR);
            let spot = (0..400).flat_map(|r| [(r * 7) as i32, -(r * 7) as i32]).flat_map(|dx| (-300..300).step_by(10).map(move |dy| (x + dx, mid + dy))).find(|&(x, y)| air(x, y) && air(x, y + 20) && air(x - 8, y) && air(x + 8, y));
            let Some((x, y)) = spot else {
                warn!("layers: no cave in the {band}");
                *state = (state.0 + 1, t, None);
                return;
            };
            let at = Vec2::new(x as f32, y as f32);
            k.body.pos = at;
            k.prev_pos = at;
            k.body.vel = Vec2::ZERO;
            *state = (state.0, t, Some(at));
            info!("layers: t {t:.1}: into the {band}, at ({x}, {y})");
        }
        Some(at) => {
            if k.body.pos.distance(at) > 80.0 {
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
                info!("layers: the {band} after {dwell:.0} s: {}", if list.is_empty() { "nothing".into() } else { list.join(", ") });
                *state = (state.0 + 1, t, None);
            }
        }
    }
}

/// `crimson` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`, by night): a
/// crimson head at 1100, the player (who can't die) at 1010; logs the
/// moves it starts, the player's health lost and coats, and the plague
/// blood (and miasma) on the floor, a second at a time. `PLATYPUS_FOE`:
/// another kind in its place (to see what it does to you).
fn crimson_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health, Option<&crate::creatures::body::elements::Coated>), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature), Without<LocalPlayer>>,
    mut began: MessageReader<crate::creatures::moves::Began>,
    mut state: Local<(u8, f32, u32)>,
    mut edits: ResMut<SimWorld>,
    chilled: Query<(), (With<LocalPlayer>, With<crate::creatures::body::elements::Chilled>)>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
) {
    if s.name != "crimson" {
        return;
    }
    // (The dev menu shut: no god mode.)
    menu.open = false;
    let mut pool: Vec<(platypus_sim::MaterialId, Vec2)> = Vec::new();
    let Ok((mut k, mut h, coat)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    state.1 += h.max - h.hp;
    h.hp = h.max;
    if state.0 == 0 && t > 0.4 {
        for (e, c) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        k.body.pos = Vec2::new(1010.0, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        let foe = std::env::var("PLATYPUS_FOE").unwrap_or_else(|_| "zombie_crimson".into());
        crate::creatures::def::spawn_creature(&mut commands, &foe, Vec2::new(1100.0, fl), |_| {});
        // (`PLATYPUS_POOL=1`: a pool of water between them.)
        if std::env::var("PLATYPUS_POOL").is_ok()
            && let Some(water) = edits.materials().id("water")
        {
            pool.push((water, Vec2::new(1060.0, fl + 4.0)));
        }
        state.0 = 1;
    }
    // (Kept where it stands, out of its grab's reach.)
    if state.0 == 1 {
        k.body.pos.x = 1010.0;
    }
    for b in began.read() {
        info!("crimson: t {t:.2} began {}", b.id);
    }
    if t as u32 != state.2 {
        state.2 = t as u32;
        let sim = &*edits;
        let count = |name: &str| sim.materials().id(name).map_or(0, |m| (900..1300).flat_map(|x| (fl as i32 - 4..fl as i32 + 60).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).count());
        let coats: Vec<String> = coat.map_or(vec![], |c| c.coats.iter().map(|(n, a)| format!("{n} {a:.2}")).collect());
        info!("crimson: t {t:.0} the player lost {:.0}; coats [{}]{}; plague blood {}, miasma {}, water {}, ice {}", state.1, coats.join(", "), if chilled.is_empty() { "" } else { ", chilled" }, count("plague_blood"), count("miasma"), count("water"), count("ice"));
    }
    for (m, at) in pool {
        for dx in (-20..=20).step_by(4) {
            edits.queue(platypus_sim::WorldEdit::Paint { center: CellPos::new(at.x as i32 + dx, at.y as i32), radius: 4, material: m, overwrite: false });
        }
    }
}

/// `sprint` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): running right,
/// a dash at 1 s with Shift held on to 4 s, then let go, still running;
/// logs its speed, whether it sprints, its stamina and the clip shown.
fn sprint_script(
    s: Res<Scenario>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut player: Query<(&mut Kinematics, &crate::combat::Stamina, &crate::creatures::body::animation::Animator), With<LocalPlayer>>,
    mut state: Local<(bool, u32)>,
) {
    if s.name != "sprint" {
        return;
    }
    menu.open = false;
    let Ok((mut k, stamina, anim)) = player.single_mut() else { return };
    let t = s.elapsed;
    if !state.0 && t > 0.3 {
        state.0 = true;
        let fl = platypus_worldgen::arena::FLOOR as f32;
        k.body.pos = Vec2::new(300.0, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
    }
    if t > 0.5 && !keys.pressed(KeyCode::KeyD) {
        keys.press(KeyCode::KeyD);
    }
    if (1.0..4.0).contains(&t) {
        if !keys.pressed(KeyCode::ShiftLeft) {
            keys.press(KeyCode::ShiftLeft);
        }
    } else if keys.pressed(KeyCode::ShiftLeft) {
        keys.release(KeyCode::ShiftLeft);
    }
    let tenth = (t * 4.0) as u32;
    if tenth != state.1 {
        state.1 = tenth;
        info!("sprint: t {t:.2} speed {:.0}, sprinting {}, stamina {:.0}, clip {}", k.body.vel.x, k.loco.sprinting, stamina.cur, anim.clip_name());
    }
}

/// `phase` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a phase spider
/// (`PLATYPUS_FOE`: another) 190 cells off; logs when it fades, goes
/// through and comes out (where), the moves it starts, what the player's
/// lost. `PLATYPUS_FLASH=1`: a flash on it whenever it's fading or through
/// (light snaps it out; a spiderling bursts).
#[allow(clippy::type_complexity)]
fn phase_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature), Without<LocalPlayer>>,
    spiders: Query<(&crate::creatures::custom::phase::Phase, &Kinematics, &crate::creatures::Health), Without<LocalPlayer>>,
    mut began: MessageReader<crate::creatures::moves::Began>,
    mut flares: MessageWriter<crate::fx::Flare>,
    mut state: Local<(u8, f32, &'static str, u32)>,
    sim: Res<SimWorld>,
) {
    if s.name != "phase" {
        return;
    }
    menu.open = false;
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    state.1 += h.max - h.hp;
    h.hp = h.max;
    if state.0 == 0 && t > 0.4 {
        for (e, c) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        k.body.pos = Vec2::new(1010.0, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        let foe = std::env::var("PLATYPUS_FOE").unwrap_or_else(|_| "phase_spider".into());
        crate::creatures::def::spawn_creature(&mut commands, &foe, Vec2::new(1200.0, fl + 10.0), |_| {});
        state.0 = 1;
    }
    for b in began.read() {
        info!("phase: t {t:.2} began {}", b.id);
    }
    if state.0 != 1 {
        return;
    }
    let me = k.body.pos;
    match spiders.iter().next() {
        Some((p, sk, sh)) => {
            if p.doing() != state.2 {
                state.2 = p.doing();
                info!("phase: t {t:.2} {} at ({:.0}, {:.0}), {:.0} from the player (who's lost {:.0}); it has {:.0} hp", p.doing(), sk.body.pos.x, sk.body.pos.y, sk.body.pos.distance(me), state.1, sh.hp);
            }
            if std::env::var("PLATYPUS_FLASH").is_ok() && p.doing() != "here" {
                flares.write(crate::fx::Flare { at: sk.body.pos + Vec2::Y * 10.0, color: [1.4, 1.4, 1.4], life: 0.2 });
            }
        }
        // (Once it's been seen: not the tick it's spawned.)
        None if !state.2.is_empty() => {
            info!("phase: t {t:.2} it's gone (dead); the player lost {:.0}", state.1);
            state.0 = 2;
        }
        None => {}
    }
    if t as u32 != state.3 {
        state.3 = t as u32;
        let far = spiders.iter().next().map_or(-1.0, |(_, sk, _)| sk.body.pos.distance(me));
        let web = sim.materials().id("cobweb").map_or(0, |m| (me.x as i32 - 60..me.x as i32 + 60).flat_map(|x| (fl as i32 - 2..fl as i32 + 50).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).count());
        info!("phase: t {t:.0} the player has lost {:.0}; it's {far:.0} off; cobweb by them {web}", state.1);
    }
}

/// `groundbuild` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): uneven dirt
/// laid over the floor (a few cells up and down, column by column, as
/// natural ground is), a puddle of water and one of blood; then bricks
/// set down through real input: a row along the ground, aimed at it as a
/// player would, then into each puddle, then a second row on the first;
/// then a block of dirt mined beside them (what's loose falls). Logs each
/// brick's block, whether its bottom touches what's under it, what's dug
/// out, the brick cells before and after the mining, and what's left of
/// the puddles.
fn groundbuild_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut player: Query<(&mut Kinematics, &mut crate::hands::items::Inventory), With<LocalPlayer>>,
    items: Option<Res<crate::hands::items::Items>>,
    mut hand: ResMut<crate::hands::Hand>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    them: Query<(Entity, &crate::creatures::Creature), Without<LocalPlayer>>,
    mut commands: Commands,
    mut st: Local<(u8, f32, usize, usize)>,
) {
    if s.name != "groundbuild" {
        return;
    }
    menu.open = false;
    mouse.release(MouseButton::Left);
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR;
    let b = platypus_sim::BLOCK;
    let Some(items) = items else { return };
    let Ok((mut k, mut inv)) = player.single_mut() else { return };
    let (step, next, i, before) = &mut *st;
    let top = |w: &platypus_sim::World, x: i32| (fl - 10..fl + 40).rev().find(|&y| w.get(CellPos::new(x, y)).is_some_and(|c| !c.is_air() && w.materials().phys(c.material).kind == platypus_sim::Kind::Static)).unwrap_or(fl - 1) + 1;
    // The row's blocks, along x 1040.., and the puddles' at 1136 and 1176.
    let row: Vec<i32> = (0..6).map(|n| 1040 / b + n).collect();
    let puddles = [(1136, "water"), (1176, "blood")];
    match *step {
        0 if t > 0.5 => {
            // (The arena's dummies out of the way: bodies a block won't go into.)
            for (e, c) in &them {
                if c.kind == "dummy" || c.kind == "sandbag" {
                    commands.entity(e).despawn();
                }
            }
            let mats = sim.materials().clone();
            let mut rng = platypus_sim::rng::Rng::seeded(&[5]);
            let (dirt, grass) = (mats.expect_id("dirt"), mats.expect_id("grass"));
            for x in 1000..1240 {
                let h = (3.0 + 2.5 * (x as f32 * 0.21).sin() + 1.5 * (x as f32 * 0.57).sin()).round() as i32;
                for y in fl..fl + h {
                    sim.world.set(CellPos::new(x, y), mats.spawn(if y == fl + h - 1 { grass } else { dirt }, &mut rng));
                }
            }
            for (px, what) in puddles {
                let id = mats.expect_id(what);
                for x in px..px + 16 {
                    for y in fl..fl + 10 {
                        sim.world.set(CellPos::new(x, y), platypus_sim::Cell::AIR);
                    }
                    for y in fl..fl + 5 {
                        sim.world.set(CellPos::new(x, y), mats.spawn(id, &mut rng));
                    }
                }
                for y in fl..fl + 10 {
                    sim.world.set(CellPos::new(px - 1, y), mats.spawn(dirt, &mut rng));
                    sim.world.set(CellPos::new(px + 16, y), mats.spawn(dirt, &mut rng));
                }
            }
            k.body.pos = Vec2::new(1010.0, (fl + 12) as f32 + k.body.half.y);
            k.prev_pos = k.body.pos;
            if let Some(brick) = items.id("block:brick") {
                inv.slots[8] = Some(crate::hands::items::Stack::new(brick, 40 * crate::hands::items::BLOCK_CELLS));
                (hand.bar, hand.slot) = (0, 8);
            }
            hand.smart = false;
            *step = 1;
            *next = t + 0.5;
        }
        // Each brick: aimed at the ground's top over the block's middle (a
        // player points at the ground), clicked.
        1 if t >= *next => {
            let aims: Vec<Vec2> = row.iter().map(|&bx| { let x = bx * b + b / 2; Vec2::new(x as f32, top(&sim.world, x) as f32 - 1.0) })
                .chain(puddles.iter().map(|&(px, _)| Vec2::new((px + 8) as f32, (fl + 3) as f32)))
                .chain(row.iter().map(|&bx| { let x = bx * b + b / 2; Vec2::new(x as f32, top(&sim.world, x) as f32 + 2.0) }))
                .collect();
            match aims.get(*i) {
                Some(&aim) => {
                    // (Stood within reach of it; the button held a moment,
                    // as a click is.)
                    if cursor.0 != Some(aim) {
                        k.body.pos.x = aim.x - 24.0;
                        k.prev_pos = k.body.pos;
                        cursor.0 = Some(aim);
                        *next = t + 0.12;
                    } else {
                        *i += 1;
                        *next = t + 0.25;
                    }
                    mouse.press(MouseButton::Left);
                }
                None => {
                    let brick = sim.materials().expect_id("block:brick".trim_start_matches("block:"));
                    *before = (980..1260).flat_map(|x| (fl - 4..fl + 60).map(move |y| CellPos::new(x, y))).filter(|&p| sim.world.get(p).is_some_and(|c| c.material == brick)).count();
                    // Each block of the first row: does its bottom touch?
                    for &bx in &row {
                        let (x0, x1) = (bx * b, bx * b + b);
                        let rows: Vec<i32> = (fl - 2..fl + 30).filter(|&y| (x0..x1).any(|x| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == brick))).collect();
                        let low = rows.first().copied().unwrap_or(0);
                        let touching = (x0..x1).any(|x| sim.world.get(CellPos::new(x, low)).is_some_and(|c| c.material == brick) && sim.world.get(CellPos::new(x, low - 1)).is_some_and(|c| !c.is_air() && sim.world.materials().phys(c.material).kind == platypus_sim::Kind::Static));
                        info!("groundbuild: block column {bx}: brick rows {:?}, its bottom touching what's under it: {touching}", rows.first().zip(rows.last()));
                    }
                    *step = 2;
                    *next = t + 0.5;
                }
            }
        }
        // Mine a block of dirt beside the row (loosening what's loose).
        2 if t >= *next => {
            let bx = row[0] - 2;
            let at = CellPos::new(bx, (top(&sim.world, bx * b + 4) - 2).div_euclid(b));
            sim.world.apply_edit(&WorldEdit::MineBlock { block: at, power: 255, max_hardness: 255, back: false });
            *step = 3;
            *next = t + 2.5;
        }
        3 if t >= *next => {
            let mats = sim.materials();
            let brick = mats.expect_id("brick");
            let after = (980..1260).flat_map(|x| (fl - 4..fl + 60).map(move |y| CellPos::new(x, y))).filter(|&p| sim.world.get(p).is_some_and(|c| c.material == brick)).count();
            let left: Vec<String> = puddles.iter().map(|&(px, what)| { let id = mats.expect_id(what); format!("{what} {}", (px - 4..px + 20).flat_map(|x| (fl..fl + 40).map(move |y| CellPos::new(x, y))).filter(|&p| sim.world.get(p).is_some_and(|c| c.material == id)).count()) }).collect();
            let dirt = items.id("block:dirt").map_or(0, |d| inv.count(d));
            info!("groundbuild: bricks placed {} cells, after mining beside them {after}; the puddles: {}; dirt in the pack {dirt} cells", *before, left.join(", "));
            *step = 4;
        }
        _ => {}
    }
}

/// `haunt` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`, try
/// `PLATYPUS_HOUR=23`): the player with a flashlight and a mining helmet's
/// lamp (aimed right), a torch planted and a candelabra stood by it, a pool
/// of water dug at 1060; a `PLATYPUS_FOE` (chain_wraith) put 260 off to the
/// right. Every 0.5 s: how haunted the player's spot is, its lamps' and a
/// flame's share of their light, the pool's water and ice.
fn haunt_script(
    s: Res<Scenario>,
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut toggles: ResMut<crate::light::LightToggles>,
    settings: Res<crate::light::LightSettings>,
    torch_art: Option<Res<crate::light::TorchArt>>,
    crafting: Res<crate::craft::Crafting>,
    haunts: Res<crate::creatures::haunt::Haunts>,
    mut cursor: ResMut<CursorOverride>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut st: Local<(u8, f32)>,
) {
    if s.name != "haunt" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR;
    let Ok(pk) = player.single() else { return };
    toggles.carry = crate::light::Carry::BigBeam;
    toggles.headlamp = true;
    cursor.0 = Some(pk.body.pos + Vec2::new(200.0, -10.0));
    let (step, next) = &mut *st;
    let pool = (1040..1080, fl - 12..fl);
    if *step == 0 && t > 0.5 {
        *step = 1;
        let mats = sim.materials().clone();
        let water = mats.expect_id("water");
        let mut rng = platypus_sim::rng::Rng::seeded(&[3]);
        for x in pool.0.clone() {
            for y in pool.1.clone() {
                sim.world.set(CellPos::new(x, y), mats.spawn(water, &mut rng));
            }
        }
        if let Some(art) = torch_art {
            crate::light::plant_torch(&mut commands, Vec2::new(pk.body.pos.x + 30.0, fl as f32), &settings, &art);
        }
        if let Some(kind) = crafting.station("candelabra") {
            crafting.spawn(&mut commands, kind, Vec2::new(pk.body.pos.x - 30.0, fl as f32), false);
        }
        let foe = std::env::var("PLATYPUS_FOE").unwrap_or_else(|_| "chain_wraith".into());
        crate::creatures::def::spawn_creature(&mut commands, &foe, Vec2::new(pk.body.pos.x + 260.0, fl as f32 + 40.0), |_| {});
        return;
    }
    if *step == 1 && t >= *next {
        *next = t + 0.5;
        let mats = sim.materials();
        let (water, ice) = (mats.expect_id("water"), mats.expect_id("ice"));
        let (mut w, mut i) = (0, 0);
        for x in pool.0.clone() {
            for y in pool.1.clone() {
                match sim.world.get(CellPos::new(x, y)).map(|c| c.material) {
                    Some(m) if m == water => w += 1,
                    Some(m) if m == ice => i += 1,
                    _ => {}
                }
            }
        }
        let p = pk.body.pos;
        info!("haunt: t {t:.1} haunted {:.2}, lamps {:.2}, a flame {:.2}; the pool: {w} water, {i} ice; ghosts at {:?}", haunts.at(p), haunts.electric(p, t, 2), haunts.flame(p, t, 1), haunts.at.iter().map(|(g, _)| g.round()).collect::<Vec<_>>());
    }
}

/// `drips` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a stone ceiling
/// over the player (40 up, 14 thick, 1000..1240) with a lens of water
/// (`PLATYPUS_LIQUID`: oil) sealed in it over 1080..1160, four cells of
/// stone under it. Every second to 8 s: the drops falling (water
/// particles) under the lens and anywhere else; at 8 s the roof under the
/// lens is mined out, and every second after: the lens's liquid left, what
/// came down onto the floor, the player's health. In a generated world
/// (`PLATYPUS_SPAWN_X`/`_Y` under a perched pocket): lamps on, aimed up,
/// and every second the drops falling within 300.
fn drips_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut toggles: ResMut<crate::light::LightToggles>,
    mut cursor: ResMut<CursorOverride>,
    player: Query<(&Kinematics, &crate::creatures::Health), With<LocalPlayer>>,
    mut st: Local<(u8, f32, u32, u32)>,
) {
    if s.name != "drips" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR;
    let Ok((pk, hp)) = player.single() else { return };
    if std::env::var("PLATYPUS_WORLD").is_err() {
        toggles.carry = crate::light::Carry::BigBeam;
        toggles.headlamp = true;
        cursor.0 = Some(pk.body.pos + Vec2::new(40.0, 80.0));
        let (_, next, seen, _) = &mut *st;
        let mats = sim.materials();
        *seen += sim.world.particles().iter().filter(|p| p.landing == platypus_sim::particles::Landing::Vanish && mats.phys(p.cell.material).kind == platypus_sim::Kind::Liquid && Vec2::from(p.pos).distance(pk.body.pos) < 300.0 && p.life >= 239).count() as u32;
        if t >= *next {
            *next = t + 1.0;
            info!("drips: t {t:.1} drops so far within 300: {seen}; the player at {:.0}", pk.body.pos);
        }
        return;
    }
    let liquid = std::env::var("PLATYPUS_LIQUID").unwrap_or_else(|_| "water".into());
    let (ceil, lens) = (fl + 40, (1080..1160, fl + 44..fl + 52));
    let (step, next, under, elsewhere) = &mut *st;
    let mats = sim.materials().clone();
    let wet = mats.expect_id(&liquid);
    if *step == 0 && t > 0.5 {
        *step = 1;
        *next = t + 1.0;
        let stone = mats.expect_id("stone");
        let mut rng = platypus_sim::rng::Rng::seeded(&[5]);
        for x in 1000..1240 {
            for y in ceil..ceil + 14 {
                let m = if lens.0.contains(&x) && lens.1.contains(&y) { wet } else { stone };
                sim.world.set(CellPos::new(x, y), mats.spawn(m, &mut rng));
            }
        }
        info!("drips: a ceiling at {ceil}, {liquid} in it over {:?}; the player at {:.0}", lens.0, pk.body.pos.x);
        return;
    }
    if *step == 0 {
        return;
    }
    // (Drops in the air under the ceiling, a frame at a time.)
    for p in sim.world.particles() {
        if p.cell.material == wet && p.pos[1] < ceil as f32 && p.pos[1] > fl as f32 {
            if lens.0.contains(&(p.pos[0] as i32)) { *under += 1 } else { *elsewhere += 1 }
        }
    }
    let count = |sim: &SimWorld, xs: std::ops::Range<i32>, ys: std::ops::Range<i32>| xs.flat_map(|x| ys.clone().map(move |y| CellPos::new(x, y))).filter(|&p| sim.world.get(p).is_some_and(|c| c.material == wet)).count();
    if *step == 1 && t >= *next {
        *next = t + 1.0;
        info!("drips: t {t:.1} drop-frames under the lens {under}, elsewhere {elsewhere}");
        if t >= 8.0 {
            *step = 2;
            for x in lens.0.clone() {
                for y in ceil..lens.1.start {
                    sim.world.set(CellPos::new(x, y), platypus_sim::Cell::AIR);
                }
            }
            info!("drips: t {t:.1} the roof under the lens mined out");
        }
    } else if *step == 2 && t >= *next {
        *next = t + 1.0;
        info!("drips: t {t:.1} {liquid} left in the lens {}, on the floor {}; the player's health {:.0}", count(&sim, lens.0.clone(), lens.1.clone()), count(&sim, 900..1340, fl..ceil), hp.hp);
    }
}

/// `cavein` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a stone ceiling
/// 60 up over 1000..1240, its underside 8 cells of loose sand
/// (`PLATYPUS_LOOSE`: loose_gravel) with a copper
/// nugget in it; the player at 960 (`PLATYPUS_UNDER=1`: under the middle).
/// Every second: grains seen falling, cave-ins under way, sand on the floor,
/// the player's health; at 5 s four cells of the loose roof dug out at
/// 1120 (`PLATYPUS_BLAST=1`: a shake instead, as a bomb's). In a generated
/// world (`PLATYPUS_SPAWN_X`/`_Y`, or `PLATYPUS_TP=x,y`, under a loose
/// roof): lamps on, the loose
/// roof nearest the player dug into at 5 s.
fn cavein_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    roofs: Res<crate::hazards::CaveIns>,
    mut trauma: ResMut<crate::fx::Trauma>,
    (mut toggles, mut cursor): (ResMut<crate::light::LightToggles>, ResMut<CursorOverride>),
    mut player: Query<(&mut Kinematics, &crate::creatures::Health), With<LocalPlayer>>,
    mut st: Local<(u8, f32, u32)>,
) {
    if s.name != "cavein" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR;
    let Ok((mut pk, hp)) = player.single_mut() else { return };
    if std::env::var("PLATYPUS_WORLD").is_err() {
        // A generated world: lamps on, and at 5 s the loose roof nearest
        // the player dug into.
        toggles.carry = crate::light::Carry::BigBeam;
        toggles.headlamp = true;
        cursor.0 = Some(pk.body.pos + Vec2::new(30.0, 90.0));
        let (step, next, placed) = &mut *st;
        // (`PLATYPUS_TP=x,y`: put there first, where the spawn's search
        // won't go: a lair's cavern.)
        if *placed == 0
            && let Ok(tp) = std::env::var("PLATYPUS_TP")
            && let Some((x, y)) = tp.split_once(',')
            && let (Ok(x), Ok(y)) = (x.parse::<f32>(), y.parse::<f32>())
        {
            pk.body.pos = Vec2::new(x, y);
            pk.body.vel = Vec2::ZERO;
            *placed = 1;
        }
        if t >= *next {
            *next = t + 1.0;
            info!("cavein: t {t:.1} cave-ins under way {}; the player's health {:.0} at {:.0}", roofs.under_way.len(), hp.hp, pk.body.pos);
        }
        if *step == 0 && t >= 5.0 {
            *step = 1;
            let mats = sim.materials().clone();
            let loose = [mats.expect_id("loose_sand"), mats.expect_id("loose_gravel")];
            let p = CellPos::from_world(pk.body.pos.x, pk.body.pos.y);
            let roof = (-100..=100).flat_map(|dx| (-40..=160).map(move |dy| p.offset(dx, dy))).filter(|&q| sim.world.get(q).is_some_and(|c| loose.contains(&c.material))).min_by_key(|q| (q.x - p.x).pow(2) + (q.y - p.y).pow(2));
            if let Some(r) = roof {
                for dx in 0..2 {
                    for dy in 0..2 {
                        sim.world.set(r.offset(dx, dy), platypus_sim::Cell::AIR);
                    }
                }
            }
            info!("cavein: t {t:.1} the loose roof over the player dug at {roof:?}");
        }
        return;
    }
    let loose = std::env::var("PLATYPUS_LOOSE").unwrap_or_else(|_| "loose_sand".into());
    let ceil = fl + 60;
    let (step, next, grains) = &mut *st;
    let mats = sim.materials().clone();
    let (lid, rubble) = (mats.expect_id(&loose), mats.phys(mats.expect_id(&loose)).crumbles_into);
    if *step == 0 && t > 0.5 {
        *step = 1;
        *next = t + 1.0;
        let (stone, copper) = (mats.expect_id("stone"), mats.expect_id("copper_ore"));
        let mut rng = platypus_sim::rng::Rng::seeded(&[6]);
        for x in 1000..1240 {
            for y in ceil - 8..ceil + 14 {
                let m = if y >= ceil { stone } else if (1150..1153).contains(&x) && (ceil - 4..ceil - 1).contains(&y) { copper } else { lid };
                sim.world.set(CellPos::new(x, y), mats.spawn(m, &mut rng));
            }
        }
        if std::env::var("PLATYPUS_UNDER").is_ok() {
            pk.body.pos.x = 1110.0;
        }
        info!("cavein: a {loose} roof at {}..{ceil} over 1000..1240; the player at {:.0}", ceil - 8, pk.body.pos.x);
        return;
    }
    if *step == 0 {
        return;
    }
    *grains += sim.world.particles().iter().filter(|p| p.cell.material == rubble && p.landing == platypus_sim::particles::Landing::Vanish && p.life == 149).count() as u32;
    let count = |sim: &SimWorld, m: platypus_sim::MaterialId| (980..1260).flat_map(|x| (fl - 2..ceil).map(move |y| CellPos::new(x, y))).filter(|&p| sim.world.get(p).is_some_and(|c| c.material == m)).count();
    if t >= *next {
        *next = t + 1.0;
        let copper = mats.expect_id("copper_ore");
        info!("cavein: t {t:.1} grains so far {grains}; cave-ins under way {}; {loose} left {}, rubble below the roof {}, copper down {}; the player's health {:.0}", roofs.under_way.len(), count(&sim, lid), count(&sim, rubble), (980..1260).flat_map(|x| (fl - 2..ceil - 10).map(move |y| CellPos::new(x, y))).filter(|&p| sim.world.get(p).is_some_and(|c| c.material == copper)).count(), hp.hp);
        if *step == 1 && t >= 5.0 {
            *step = 2;
            if std::env::var("PLATYPUS_BLAST").is_ok() {
                trauma.0 = 0.6;
                info!("cavein: t {t:.1} shaken");
            } else {
                for x in 1120..1122 {
                    for y in ceil - 8..ceil - 6 {
                        sim.world.set(CellPos::new(x, y), platypus_sim::Cell::AIR);
                    }
                }
                info!("cavein: t {t:.1} four cells of the roof dug out");
            }
        }
    }
}

/// `firedamp` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): an upturned
/// cup of stone over the player (a roof 56 up over 1000..1240, coal seams
/// in it, walls down to 30 up), firedamp filling it from 26 up; the player
/// under its middle carries a safety lamp, then at 4 s a torch (`PLATYPUS_CARRY`: what
/// instead: lantern, flashlight), and jumps at 4.5 s; a TNT barrel just
/// outside the cup, at 1262 (`PLATYPUS_NOBARREL=1`: none); `PLATYPUS_DARK=1`:
/// the light switched off (L) before the torch comes out, on again at 6 s,
/// a jump at 6.5 s.
/// Every half second:
/// the firedamp left, burning coal, the lamp's reading, the player's health.
/// In a generated world (`PLATYPUS_TP=x,y` in a chamber with firedamp):
/// the safety lamp, then at 3.5 s a torch, lifted at 4 s to just under the
/// gas and held there half a second.
fn firedamp_script(
    s: Res<Scenario>,
    mut commands: Commands,
    mut sim: ResMut<SimWorld>,
    barrels: Query<&Kinematics, (With<crate::creatures::custom::explosive::Explosive>, Without<LocalPlayer>)>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut toggles: ResMut<crate::light::LightToggles>,
    damp: Res<crate::hazards::Firedamp>,
    switched: Res<crate::light::switch::Switch>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut player: Query<(&mut Kinematics, &crate::creatures::Health), With<LocalPlayer>>,
    mut st: Local<(u8, f32, Vec2)>,
) {
    if s.name != "firedamp" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR;
    let Ok((mut pk, hp)) = player.single_mut() else { return };
    let (step, next, held) = &mut *st;
    let mats = sim.materials().clone();
    let (gas, coal) = (mats.expect_id("firedamp"), mats.expect_id("coal_seam"));
    if std::env::var("PLATYPUS_WORLD").is_err() {
        // A generated world (`PLATYPUS_TP=x,y` in a chamber with firedamp):
        // the safety lamp, then a torch, lifted to just under the gas (as if
        // climbing to it).
        let near = |sim: &SimWorld, c: CellPos, m: platypus_sim::MaterialId, r: i32| (-r..=r).step_by(2).flat_map(|dx| (-r..=r).step_by(2).map(move |dy| c.offset(dx, dy))).filter(|&p| sim.world.get(p).is_some_and(|cell| cell.material == m)).count();
        if *step == 0 {
            *step = 1;
            toggles.carry = crate::light::Carry::SafetyLamp;
            if let Ok(tp) = std::env::var("PLATYPUS_TP")
                && let Some((x, y)) = tp.split_once(',')
                && let (Ok(x), Ok(y)) = (x.parse::<f32>(), y.parse::<f32>())
            {
                pk.body.pos = Vec2::new(x, y);
                pk.body.vel = Vec2::ZERO;
            }
        }
        let c = CellPos::from_world(pk.body.pos.x, pk.body.pos.y);
        if *step == 1 && t >= 3.5 {
            toggles.carry = crate::light::Carry::Torch;
        }
        if *step == 1 && t >= 4.0 {
            *step = 2;
            if let Some(up) = (0..160).find(|&d| sim.world.get(c.offset(0, d)).is_some_and(|cell| cell.material == gas)) {
                pk.body.pos.y += up as f32 - 10.0;
                pk.body.vel = Vec2::ZERO;
            }
            *held = pk.body.pos;
            info!("firedamp: t {t:.1} a torch, lifted to {:.0}", pk.body.pos);
        }
        // (Held there half a second, as if on a ledge.)
        if *step == 2 && t < 4.5 {
            pk.body.pos = *held;
            pk.body.vel = Vec2::ZERO;
        }
        if t >= *next {
            *next = t + 0.5;
            info!("firedamp: t {t:.1} firedamp within 60: {}, coal {}; the lamp reads {:.2}; the player's health {:.0} at {:.0}", near(&sim, c, gas, 60), near(&sim, c, coal, 60), damp.near, hp.hp, pk.body.pos);
        }
        return;
    }
    let roof = fl + 56;
    if *step == 0 && t > 0.5 {
        *step = 1;
        let stone = mats.expect_id("stone");
        let mut rng = platypus_sim::rng::Rng::seeded(&[8]);
        for x in 992..1248 {
            for y in fl + 26..roof + 10 {
                let wall = !(1000..1240).contains(&x);
                let m = if y >= roof || wall && y >= fl + 30 {
                    if y < roof + 6 && (y - roof).rem_euclid(3) == 0 { coal } else { stone }
                } else if wall {
                    continue;
                } else {
                    gas
                };
                sim.world.set(CellPos::new(x, y), mats.spawn(m, &mut rng));
            }
        }
        toggles.carry = crate::light::Carry::SafetyLamp;
        pk.body.pos.x = 1120.0;
        if std::env::var("PLATYPUS_NOBARREL").is_err() {
            crate::creatures::def::spawn_creature(&mut commands, "tnt_barrel", Vec2::new(1262.0, fl as f32), |_| {});
        }
        info!("firedamp: a cup of firedamp over 1000..1240 from {} to {roof}; the player at {:.0} with a safety lamp", fl + 26, pk.body.pos.x);
        return;
    }
    if *step == 0 {
        return;
    }
    if *step == 1 && t >= 4.0 {
        *step = 2;
        toggles.carry = match std::env::var("PLATYPUS_CARRY").as_deref() {
            Ok("lantern") => crate::light::Carry::Lantern,
            Ok("flashlight") => crate::light::Carry::BigBeam,
            _ => crate::light::Carry::Torch,
        };
        info!("firedamp: t {t:.1} now carrying {:?}", toggles.carry);
    }
    let jump = (4.5..4.7).contains(&t) || std::env::var("PLATYPUS_DARK").is_ok() && (6.5..6.7).contains(&t);
    if jump { keys.press(KeyCode::Space) } else { keys.release(KeyCode::Space) }
    // (`PLATYPUS_DARK=1`: its light switched off, L, at 3.5 s, so the torch
    // comes out dark; switched on again, L, at 6 s, in the gas.)
    if std::env::var("PLATYPUS_DARK").is_ok() {
        if (3.5..3.53).contains(&t) || (6.0..6.03).contains(&t) {
            if !keys.pressed(KeyCode::KeyL) {
                keys.press(KeyCode::KeyL);
                info!("firedamp: t {t:.2} L pressed (the hand was {})", if switched.hand.on { "on" } else { "off" });
            }
        } else {
            keys.release(KeyCode::KeyL);
        }
    }
    if t >= *next {
        *next = t + 0.5;
        let count = |m: platypus_sim::MaterialId| (990..1250).flat_map(|x| (fl..roof + 10).map(move |y| CellPos::new(x, y))).filter(|&p| sim.world.get(p).is_some_and(|c| c.material == m)).count();
        let burning = (990..1250).flat_map(|x| (fl..roof + 10).map(move |y| CellPos::new(x, y))).filter(|&p| sim.world.get(p).is_some_and(|c| c.material == coal && c.flags & platypus_sim::cell::flags::BURNING != 0)).count();
        info!("firedamp: t {t:.1} firedamp {} cells, coal {} ({burning} burning); the lamp reads {:.2}; the player's health {:.0} at {:.0}; TNT barrels {}; the hand's light {} ({:.2})", count(gas), count(coal), damp.near, hp.hp, pk.body.pos, barrels.iter().count(), if switched.hand.on { "on" } else { "off" }, switched.hand_level(toggles.carry));
    }
}

/// `death` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat PLATYPUS_RESPAWN=1`):
/// the player with 40 gold, scorched at 1 s (8, by fire), left at 6 health
/// at 1.5 s with a `PLATYPUS_FOE` (rat) beside it: it dies. Every half
/// second: whether the death screen's up, what it says killed it, the
/// countdown, where the player is; `PLATYPUS_SPACE` (s after dying, 2):
/// Space pressed, to wake at once; `PLATYPUS_AGAIN=1`: then a second death;
/// `PLATYPUS_HOOK=1`: hooked into the floor as it dies.
fn death_script(
    s: Res<Scenario>,
    mut commands: Commands,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    dying: Res<crate::death::Dying>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut player: Query<(Entity, &Kinematics, &mut crate::creatures::Health, Option<&mut crate::gold::Gold>, Option<&crate::gear::hook::Rope>), With<LocalPlayer>>,
    mut cursor: ResMut<CursorOverride>,
    (vis, bodies, pieces): (Query<&Visibility>, Query<&crate::hands::corpses::Corpse>, Query<&crate::creatures::body::wounds::CutPiece>),
    mut st: Local<(u8, f32, Option<f32>)>,
) {
    if s.name != "death" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let Ok((me, pk, mut hp, gold, rope)) = player.single_mut() else { return };
    let (step, next, died) = &mut *st;
    keys.release(KeyCode::Space);
    // (`PLATYPUS_HOOK=1`: the hook thrown into the floor off to the right at
    // 1.2 s, held as it dies.)
    if std::env::var("PLATYPUS_HOOK").is_ok() {
        cursor.0 = Some(pk.body.pos + Vec2::new(70.0, -14.0));
        if (1.2..1.3).contains(&t) { keys.press(KeyCode::KeyE) } else { keys.release(KeyCode::KeyE) }
    }
    if *step == 0 && t > 1.0 {
        *step = 1;
        if let Some(mut g) = gold {
            g.0 = 40;
        }
        hp.harm_by(8.0, crate::creatures::Harm::Fire, || crate::death::Cause::new("Flames", "burned alive"));
    }
    if *step == 1 && t > 1.5 {
        *step = 2;
        hp.hp = 6.0;
        let foe = std::env::var("PLATYPUS_FOE").unwrap_or_else(|_| "rat".into());
        crate::creatures::def::spawn_creature(&mut commands, &foe, Vec2::new(pk.body.pos.x + 20.0, platypus_worldgen::arena::FLOOR as f32), |_| {});
    }
    if died.is_none() && dying.0.is_some() {
        *died = Some(t);
    }
    // (`PLATYPUS_AGAIN=1`: woken, left at 4 health 3 s on, with the rat
    // still about: a second death, to see the grudge and the new life.)
    if std::env::var("PLATYPUS_AGAIN").is_ok()
        && *step == 2
        && let Some(at) = *died
        && dying.0.is_none()
        && t > at + 3.0
    {
        *step = 3;
        *died = None;
        // (Out of its grace after a bite, or the grace gives it back.)
        commands.entity(me).remove::<crate::combat::Invulnerable>();
        hp.hp = 4.0;
        info!("death: t {t:.1} left at 4 health again");
    }
    let space: f32 = std::env::var("PLATYPUS_SPACE").ok().and_then(|v| v.parse().ok()).unwrap_or(2.0);
    if let Some(at) = *died
        && dying.0.is_some()
        && t >= at + space
        && t < at + space + 0.05
    {
        keys.press(KeyCode::Space);
    }
    if t >= *next {
        *next = t + 0.5;
        match dying.0.as_ref() {
            Some(_) if std::env::var("PLATYPUS_DEATHDBG").is_ok() => info!("death: dbg t {t:.1} player visibility {:?}; bodies {}; pieces cut off {}", vis.get(me).ok(), bodies.iter().count(), pieces.iter().count()),
            Some(d) => info!("death: t {t:.1} the screen's up: {} ({}) for {:.0}, {:.0} too many; grudge {}; moments {:?}; life {:.1} s, kills {}, gold left {}; \"{}\"; {:.1} s in", d.cause.title, d.cause.how, d.blow, d.over, d.grudge, d.moments, d.life, d.kills, d.gold, d.epitaph, d.since),
            None => info!("death: t {t:.1} alive, health {:.0} at {:.0}; the rope {:?}", hp.hp, pk.body.pos, rope.and_then(|r| r.tether()).map(|(p, l)| (p.round(), l.round()))),
        }
    }
}

/// `switch` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat PLATYPUS_HOUR=23`):
/// a flashlight in hand (`PLATYPUS_CARRY`: torch, lantern) and a helmet's
/// lamp, aimed right; L at 1 s (off) and at 2.5 s (on). Every 0.05 s from
/// each: how much of the hand's and the head's light is on, the filament's
/// share, the flare.
fn switch_script(
    s: Res<Scenario>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut toggles: ResMut<crate::light::LightToggles>,
    switched: Res<crate::light::switch::Switch>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut cursor: ResMut<CursorOverride>,
    player: Query<&Kinematics, With<LocalPlayer>>,
    mut next: Local<f32>,
) {
    if s.name != "switch" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let Ok(pk) = player.single() else { return };
    toggles.carry = match std::env::var("PLATYPUS_CARRY").as_deref() {
        Ok("torch") => crate::light::Carry::Torch,
        Ok("lantern") => crate::light::Carry::Lantern,
        _ => crate::light::Carry::BigBeam,
    };
    toggles.headlamp = true;
    cursor.0 = Some(pk.body.pos + Vec2::new(160.0, 10.0));
    let flip = (1.0..1.03).contains(&t) || (2.5..2.53).contains(&t);
    if flip && !keys.pressed(KeyCode::KeyL) { keys.press(KeyCode::KeyL) } else if !flip { keys.release(KeyCode::KeyL) }
    let near = (1.0..1.8).contains(&t) || (2.5..3.1).contains(&t);
    if near && t >= *next {
        *next = t + 0.05;
        let (on, warm, flare) = switched.hand.electric();
        info!("switch: t {t:.2} the hand {} ({:.2} of it; electric {on:.2}, filament {warm:.1}, flare {flare:.2}); the head {:.2}", if switched.hand.on { "on" } else { "off" }, switched.hand_level(toggles.carry), switched.head.electric().0);
    }
}

/// `lightthrow` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`, try
/// `PLATYPUS_HOUR=23`): torches, a flashlight and a lantern in the hotbar,
/// a TNT barrel 150 to the right. A torch tapped down 40 ahead (1 s); a
/// torch held and thrown at the barrel (2 s); the flashlight thrown 80 out
/// (4.5 s); the lantern thrown 60 out (6.5 s); the planted torch
/// right-clicked back (9 s). Every half second: what's down, what's in
/// flight, the barrel, fire about, the torches in the pack.
fn lightthrow_script(
    s: Res<Scenario>,
    mut commands: Commands,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    sim: Res<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    (mut hand, mut cursor, mut mouse): (ResMut<crate::hands::Hand>, ResMut<CursorOverride>, ResMut<ButtonInput<MouseButton>>),
    (mut player, mut keys, dots): (Query<(&Kinematics, &mut crate::hands::items::Inventory, &mut crate::gear::Equipment), With<LocalPlayer>>, ResMut<ButtonInput<KeyCode>>, Query<&Visibility, With<crate::cursor::ArcDot>>),
    placed: Query<(&crate::hands::lights::PlacedLight, Has<crate::hands::lights::BeamLight>, Option<&Children>)>,
    thrown: Query<&Name>,
    barrels: Query<&Kinematics, (With<crate::creatures::custom::explosive::Explosive>, Without<LocalPlayer>)>,
    mut st: Local<(u8, f32)>,
) {
    if s.name != "lightthrow" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let Some(items) = items else { return };
    let Ok((pk, mut inv, mut eq)) = player.single_mut() else { return };
    let (step, next) = &mut *st;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let x = pk.body.pos.x;
    let id = |n: &str| items.id(n).expect(n);
    // (`PLATYPUS_TOSS=1`: three torches in the off hand; T held 1..1.5 s
    // at a point 100 out, let go.)
    if std::env::var("PLATYPUS_TOSS").is_ok() {
        if *step == 0 && t > 0.5 {
            *step = 9;
            eq.worn[crate::gear::OFF_HAND] = Some(crate::hands::items::Stack::new(id("torch"), 3));
        }
        cursor.0 = Some(Vec2::new(x + 100.0, fl + 4.0));
        if (1.0..1.5).contains(&t) { keys.press(KeyCode::KeyT) } else { keys.release(KeyCode::KeyT) }
        if t >= *next && *step == 9 {
            *next = t + 0.25;
            let flying = thrown.iter().filter(|n| n.as_str() == "Thrown light").count();
            let arc = dots.iter().filter(|v| **v != Visibility::Hidden).count();
            info!("lightthrow: t {t:.2} off hand {:?}; arc dots shown {arc}; in flight {flying}; down {}", eq.worn[crate::gear::OFF_HAND].map(|s| s.count), placed.iter().count());
        }
        return;
    }
    if *step == 0 && t > 0.5 {
        *step = 1;
        inv.slots[0] = Some(crate::hands::items::Stack::new(id("torch"), 5));
        inv.slots[1] = Some(crate::hands::items::Stack::new(id("flashlight"), 1));
        inv.slots[2] = Some(crate::hands::items::Stack::new(id("lantern"), 1));
        crate::creatures::def::spawn_creature(&mut commands, "tnt_barrel", Vec2::new(x + 150.0, fl), |_| {});
    }
    // (Which slot, where the cursor is, and the left button held from..to.)
    let (slot, aim, press) = match t {
        _ if t < 2.0 => (0, Vec2::new(x + 40.0, fl + 2.0), (1.0, 1.05)),
        _ if t < 4.5 => (0, Vec2::new(x + 150.0, fl + 12.0), (2.0, 2.5)),
        _ if t < 6.5 => (1, Vec2::new(x + 80.0, fl + 4.0), (4.5, 5.0)),
        _ => (2, Vec2::new(x + 60.0, fl + 2.0), (6.5, 7.0)),
    };
    (hand.bar, hand.slot) = (0, slot);
    cursor.0 = Some(aim);
    if t >= press.0 && t < press.1 { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    // The planted torch taken back.
    if (9.0..9.05).contains(&t) {
        cursor.0 = Some(Vec2::new(x + 40.0, fl + 4.0));
        if !mouse.pressed(MouseButton::Right) {
            mouse.press(MouseButton::Right);
        }
    } else {
        mouse.release(MouseButton::Right);
    }
    if t >= *next && *step > 0 {
        *next = t + 0.5;
        let down: Vec<String> = placed.iter().map(|(p, beam, _)| format!("{} at {:.0}{}", items.def(p.item).id, p.at.x - x, if beam { " (beam)" } else { "" })).collect();
        let flying = thrown.iter().filter(|n| n.as_str() == "Thrown light").count();
        let fire = sim.materials().fire();
        let burning = ((x - 40.0) as i32..(x + 200.0) as i32).flat_map(|cx| (fl as i32..fl as i32 + 30).map(move |cy| CellPos::new(cx, cy))).filter(|&p| sim.world.get(p).is_some_and(|c| c.material == fire || c.flags & platypus_sim::cell::flags::BURNING != 0)).count();
        let torches = inv.slots.iter().flatten().filter(|s| s.item == id("torch")).map(|s| s.count).sum::<u32>();
        info!("lightthrow: t {t:.1} down: {down:?}; in flight {flying}; TNT barrels {}; burning cells {burning}; torches in the pack {torches}", barrels.iter().count());
    }
}

/// `rest` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`; `PLATYPUS_DIFFICULTY`
/// to try another): the player left at half health at 0.5 s; a campfire
/// set down beside it at 6 s. Every second: its health, how fast it heals.
fn rest_script(
    s: Res<Scenario>,
    mut commands: Commands,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    crafting: Res<crate::craft::Crafting>,
    mut player: Query<(Entity, &Kinematics, &mut crate::creatures::Health, &crate::creatures::nature::Regenerates), With<LocalPlayer>>,
    mut st: Local<(u8, f32)>,
) {
    if s.name != "rest" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let Ok((me, pk, mut hp, regen)) = player.single_mut() else { return };
    let (step, next) = &mut *st;
    if *step == 0 && t > 0.5 {
        *step = 1;
        commands.entity(me).remove::<crate::combat::Invulnerable>();
        hp.hp = 50.0;
    }
    if *step == 1 && t > 6.0 {
        *step = 2;
        if let Some(kind) = crafting.station("campfire") {
            // (`PLATYPUS_FIREAT`: this far to its right instead, 0: under it.)
            let off: f32 = std::env::var("PLATYPUS_FIREAT").ok().and_then(|v| v.parse().ok()).unwrap_or(24.0);
            crafting.spawn(&mut commands, kind, Vec2::new(pk.body.pos.x + off, platypus_worldgen::arena::FLOOR as f32), false);
        }
        info!("rest: t {t:.1} a campfire set down beside it");
    }
    if t >= *next {
        *next = t + 1.0;
        info!("rest: t {t:.1} health {:.1}, healing {:.2} a second", hp.hp, regen.per_sec);
    }
}

/// `gauntlet` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`; a difficulty
/// with `PLATYPUS_DIFFICULTY`): the player in `PLATYPUS_GEAR` (none,
/// leather, iron), standing; a `PLATYPUS_FOE` (skeleton; `PLATYPUS_COUNT`
/// of them) put 90 to its left, facing it. Each hit it takes (what got
/// through), and when it first dies: after how many, how long.
fn gauntlet_script(
    s: Res<Scenario>,
    mut commands: Commands,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    items: Option<Res<crate::hands::items::Items>>,
    deaths: Res<crate::creatures::PlayerDeaths>,
    mut player: Query<(&Kinematics, &crate::creatures::Health, &mut crate::gear::Equipment), With<LocalPlayer>>,
    mut st: Local<(u8, f32, Vec<f32>, u32, bool)>,
) {
    if s.name != "gauntlet" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let Some(items) = items else { return };
    let Ok((pk, hp, mut eq)) = player.single_mut() else { return };
    let (step, last, hits, died_before, done) = &mut *st;
    let foe = std::env::var("PLATYPUS_FOE").unwrap_or_else(|_| "skeleton".into());
    if *step == 0 && t > 0.3 {
        *step = 1;
        let set: &[&str] = match std::env::var("PLATYPUS_GEAR").as_deref() {
            Ok("leather") => &["leather_cap", "leather_jerkin", "leather_gloves", "leather_trousers", "leather_boots"],
            Ok("iron") => &["iron_helm", "chainmail", "iron_gauntlets", "iron_greaves", "iron_boots"],
            _ => &[],
        };
        for name in set {
            let stack = crate::hands::items::Stack::new(items.id(name).expect(name), 1);
            if let Some(i) = (0..eq.worn.len()).find(|&i| eq.worn[i].is_none() && crate::gear::Equipment::fits(&items, i, &stack)) {
                eq.worn[i] = Some(stack);
            }
        }
        *last = hp.hp;
        *died_before = deaths.0;
        return;
    }
    if *step == 1 && t > 0.6 {
        *step = 2;
        let n: usize = std::env::var("PLATYPUS_COUNT").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
        for i in 0..n {
            // (On the player from the start: told where it is, not left to
            // find it out.)
            let at = pk.body.pos;
            crate::creatures::def::spawn_creature(&mut commands, &foe, Vec2::new(pk.body.pos.x - 90.0 - 20.0 * i as f32, platypus_worldgen::arena::FLOOR as f32), move |e| {
                e.insert(crate::creatures::brain::senses::Alert::hunting(at));
            });
        }
        *last = hp.hp;
    }
    if *step < 2 || *done {
        return;
    }
    if std::env::var("PLATYPUS_GAUNTLETLOG").is_ok() && (t * 2.0).fract() < 0.02 {
        info!("gauntlet: dbg t {t:.1} hp {:.0} deaths {} (before {}) hits {}", hp.hp, deaths.0, died_before, hits.len());
    }
    if deaths.0 > *died_before {
        *done = true;
        hits.push(*last);
        info!("gauntlet: {foe} x{}, gear {}: dead after {} hits in {:.1} s: {:?}", std::env::var("PLATYPUS_COUNT").unwrap_or_else(|_| "1".into()), std::env::var("PLATYPUS_GEAR").unwrap_or_else(|_| "none".into()), hits.len(), t - 0.6, hits.iter().map(|h| h.round()).collect::<Vec<_>>());
        return;
    }
    if hp.hp < *last - 0.5 {
        hits.push(*last - hp.hp);
    }
    *last = hp.hp;
}

/// `aimswing` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a
/// `PLATYPUS_WEAPON` (shortsword) swung once each with the cursor level, 45
/// up, 80 up and 40 down, a second apart from 1 s (pictures with
/// `PLATYPUS_SHOTS`); logs each swing's aim and blade angle.
fn aimswing_script(
    s: Res<Scenario>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    items: Option<Res<crate::hands::items::Items>>,
    (mut hand, mut cursor, mut mouse): (ResMut<crate::hands::Hand>, ResMut<CursorOverride>, ResMut<ButtonInput<MouseButton>>),
    mut player: Query<(&Kinematics, &mut crate::hands::items::Inventory, Option<&crate::combat::Swing>), With<LocalPlayer>>,
    mut next: Local<f32>,
) {
    if s.name != "aimswing" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let (Some(items), Ok((pk, mut inv, swing))) = (items, player.single_mut()) else { return };
    let w = std::env::var("PLATYPUS_WEAPON").unwrap_or_else(|_| "shortsword".into());
    if let Some(id) = items.id(&w) {
        inv.slots[9] = Some(crate::hands::items::Stack::new(id, 1));
        (hand.bar, hand.slot) = (0, 9);
    }
    let angles = [0.0f32, 45.0, 80.0, -40.0];
    let i = ((t - 1.0).max(0.0)) as usize;
    let a = angles.get(i).copied().unwrap_or(0.0).to_radians();
    cursor.0 = Some(pk.body.pos + Vec2::new(a.cos(), a.sin()) * 60.0);
    let local = (t - 1.0).fract();
    if t > 1.0 && i < angles.len() && local < 0.05 { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    if let Some(sw) = swing
        && t >= *next
    {
        *next = t + 0.03;
        info!("aimswing: t {t:.2} aim {:.0} blade {:.0}", sw.aim, sw.angle);
    }
}

/// `wallhook` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a stone wall
/// 30 to the player's right, 200 tall; the hook thrown up into it at 0.6 s
/// (pulled up to it, hanging against it), Space at 2 s (`PLATYPUS_AWAY=1`:
/// holding A, away from the wall, as it jumps). Every 0.1 s from 1.8 s:
/// where the player is, how fast it goes.
fn wallhook_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut cursor: ResMut<CursorOverride>,
    player: Query<(&Kinematics, Option<&crate::gear::hook::Rope>), With<LocalPlayer>>,
    mut st: Local<(u8, f32, f32)>,
) {
    if s.name != "wallhook" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR;
    let Ok((pk, rope)) = player.single() else { return };
    let (step, next, wall_x) = &mut *st;
    if *step == 0 && t > 0.3 {
        *step = 1;
        *wall_x = pk.body.pos.x + 30.0;
        let mats = sim.materials().clone();
        let stone = mats.expect_id("stone");
        let mut rng = platypus_sim::rng::Rng::seeded(&[4]);
        for x in *wall_x as i32..*wall_x as i32 + 20 {
            for y in fl..fl + 200 {
                sim.world.set(CellPos::new(x, y), mats.spawn(stone, &mut rng));
            }
        }
    }
    cursor.0 = Some(Vec2::new(*wall_x + 1.0, fl as f32 + 110.0));
    if (0.6..0.65).contains(&t) { keys.press(KeyCode::KeyE) } else { keys.release(KeyCode::KeyE) }
    let away = std::env::var("PLATYPUS_AWAY").is_ok();
    if (2.0..2.05).contains(&t) { keys.press(KeyCode::Space) } else { keys.release(KeyCode::Space) }
    if away && (1.95..2.3).contains(&t) { keys.press(KeyCode::KeyA) } else { keys.release(KeyCode::KeyA) }
    if t >= 1.8 && t >= *next && t < 2.8 {
        *next = t + 0.1;
        info!("wallhook: t {t:.1} at {:.0} ({:.0} from the wall), going {:.0}; roped {}", pk.body.pos, *wall_x - pk.body.pos.x, pk.body.vel, rope.is_some_and(|r| r.roped()));
    }
}

/// `chase` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a `PLATYPUS_FOE`
/// (zombie) put `PLATYPUS_OFF` (220) cells to the player's right; the player
/// stands (`PLATYPUS_FLEE=1`: runs left, holding A, from 1.5 s;
/// `PLATYPUS_SWING=1`: swings its shortsword at it as it comes near). Every
/// 0.25 s: how far off it is, how fast it goes, the player's health; then
/// when it first got within 30, what it cost, how close it came.
fn chase_script(
    s: Res<Scenario>,
    mut commands: Commands,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut player: Query<(&Kinematics, &crate::creatures::Health, &mut crate::hands::items::Inventory), With<LocalPlayer>>,
    them: Query<(&crate::creatures::Creature, &Kinematics, Option<&crate::creatures::Health>), Without<LocalPlayer>>,
    (items, mut hand, mut cursor, mut mouse): (Option<Res<crate::hands::items::Items>>, ResMut<crate::hands::Hand>, ResMut<CursorOverride>, ResMut<ButtonInput<MouseButton>>),
    mut st: Local<(u8, f32, Option<f32>, f32, f32)>,
) {
    if s.name != "chase" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let foe = std::env::var("PLATYPUS_FOE").unwrap_or_else(|_| "zombie".into());
    let Ok((pk, hp, mut inv)) = player.single_mut() else { return };
    mouse.release(MouseButton::Left);
    let flee = std::env::var("PLATYPUS_FLEE").is_ok();
    if flee && t > 1.5 { keys.press(KeyCode::KeyA) } else { keys.release(KeyCode::KeyA) }
    let (step, next, caught, closest, start_hp) = &mut *st;
    if *step == 0 && t > 0.5 {
        let off: f32 = std::env::var("PLATYPUS_OFF").ok().and_then(|v| v.parse().ok()).unwrap_or(220.0);
        crate::creatures::def::spawn_creature(&mut commands, &foe, Vec2::new(pk.body.pos.x + off, platypus_worldgen::arena::FLOOR as f32), |_| {});
        *step = 1;
        *closest = f32::MAX;
        *start_hp = hp.hp;
        return;
    }
    let Some((_, fk, fh)) = them.iter().find(|(c, ..)| c.kind == foe) else { return };
    let d = fk.body.pos.distance(pk.body.pos);
    // (`PLATYPUS_SWING=1`: the shortsword swung at it as it comes near.)
    if std::env::var("PLATYPUS_SWING").is_ok()
        && let Some(items) = &items
        && let Some(sw) = items.id("shortsword")
    {
        inv.slots[9] = Some(crate::hands::items::Stack::new(sw, 1));
        (hand.bar, hand.slot) = (0, 9);
        cursor.0 = Some(fk.body.pos);
        if d < 34.0 {
            mouse.press(MouseButton::Left);
        }
    }
    *closest = closest.min(d);
    if caught.is_none() && d < 30.0 {
        *caught = Some(t);
    }
    if t >= *next {
        *next = t + std::env::var("PLATYPUS_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(0.25);
        info!("chase: t {t:.2} {foe} {d:.0} off, going {:.0} (the player {:.0}), hp {:.0}; its hp {:.0}", fk.body.vel.x.abs(), pk.body.vel.x.abs(), hp.hp, fh.map_or(0.0, |h| h.hp));
    }
    if *step == 1 && t > s.duration - 0.5 {
        *step = 2;
        info!("chase: END {foe}: within 30 at {}, closest {closest:.0}, the player lost {:.0}", caught.map_or("never".into(), |c| format!("{c:.1} s")), *start_hp - hp.hp);
    }
}

/// `placing` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a workbench set
/// down through real input on a lump of dirt (levelled, the dirt dug into
/// the pack) and tried on a lump of stone (refused: mine it flat); logs
/// each, shots of the outline (`PLATYPUS_SHOTS`).
fn placing_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::hands::items::Inventory), With<LocalPlayer>>,
    items: Option<Res<crate::hands::items::Items>>,
    stations: Query<&Kinematics, (With<crate::craft::Station>, Without<LocalPlayer>)>,
    mut hand: ResMut<crate::hands::Hand>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut toasts: MessageReader<crate::progress::Toast>,
    mut step: Local<u8>,
) {
    if s.name != "placing" {
        toasts.clear();
        return;
    }
    menu.open = false;
    mouse.release(MouseButton::Left);
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR;
    let (dirt_at, stone_at) = (Vec2::new(1040.0, fl as f32 + 4.0), Vec2::new(1100.0, fl as f32 + 4.0));
    for m in toasts.read() {
        info!("placing: told \"{}\"", m.0);
    }
    let Some(items) = items else { return };
    let Ok((mut k, mut inv)) = player.single_mut() else { return };
    let dirt = items.id("block:dirt").map_or(0, |d| inv.count(d) / items.unit(d));
    match *step {
        0 if t > 0.5 => {
            let mats = sim.materials().clone();
            let mut rng = platypus_sim::rng::Rng::seeded(&[7]);
            for (name, x0, w, h) in [("dirt", 1036, 10, 5), ("stone", 1097, 6, 3)] {
                let id = mats.expect_id(name);
                for x in x0..x0 + w {
                    for y in fl..fl + h {
                        sim.world.set(CellPos::new(x, y), mats.spawn(id, &mut rng));
                    }
                }
            }
            k.body.pos = Vec2::new(1070.0, fl as f32 + k.body.half.y + 8.0);
            k.prev_pos = k.body.pos;
            if let Some(w) = items.id("workbench") {
                inv.slots[8] = Some(crate::hands::items::Stack::new(w, 2));
                (hand.bar, hand.slot) = (0, 8);
            }
            *step = 1;
        }
        1 if t > 1.2 => {
            cursor.0 = Some(dirt_at);
            if t > 1.8 {
                info!("placing: on the dirt lump, {} dirt in the pack before", dirt);
                mouse.press(MouseButton::Left);
                *step = 2;
            }
        }
        2 if t > 2.4 => {
            info!("placing: after: {} stations down, {} dirt in the pack", stations.iter().count(), dirt);
            cursor.0 = Some(stone_at);
            *step = 3;
        }
        3 if t > 3.0 => {
            mouse.press(MouseButton::Left);
            *step = 4;
        }
        4 if t > 3.6 => {
            info!("placing: on the stone lump: {} stations down", stations.iter().count());
            *step = 5;
        }
        _ => {}
    }
}

/// `build` with `PLATYPUS_SLEEP=1`: once it's furnished, upstairs, a
/// right-click on the bed; the checkpoint's there after.
fn sleep_script(
    s: Res<Scenario>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut cursor: ResMut<CursorOverride>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    checkpoint: Res<crate::respawn::Checkpoint>,
    mut health: Query<&mut crate::creatures::Health, With<LocalPlayer>>,
    mut step: Local<u8>,
) {
    if s.name != "build" || std::env::var("PLATYPUS_SLEEP").is_err() {
        return;
    }
    let t = s.elapsed;
    let b = platypus_sim::BLOCK;
    let g = platypus_worldgen::arena::FLOOR.div_euclid(b);
    // (As `build_script` lays it out: the bed upstairs, on the right.)
    let (l, up) = (1000 / b, g + 8);
    let floor = ((up + 1) * b) as f32;
    let bed = Vec2::new((l as f32 + 21.9) * b as f32, floor + 6.0);
    match *step {
        0 if t > 2.0 => {
            if let Ok(mut k) = player.single_mut() {
                k.body.pos = Vec2::new((l + 17) as f32 * b as f32, floor + k.body.half.y + 0.5);
                k.prev_pos = k.body.pos;
            }
            info!("sleep: before the bed, checkpoint {:?}", checkpoint.0);
            *step = 1;
        }
        1 if t > 2.6 => {
            cursor.0 = Some(bed);
            mouse.press(MouseButton::Right);
            *step = 2;
        }
        2 if t > 2.8 => {
            mouse.release(MouseButton::Right);
            info!("sleep: after the bed, checkpoint {:?} (the bed at x {:.0}, its floor y {floor})", checkpoint.0, bed.x);
            *step = 3;
        }
        // Then, sent away and struck dead (`PLATYPUS_RESPAWN=1` to wake at
        // the checkpoint): back at the bed.
        3 if t > 3.0 => {
            if let (Ok(mut k), Ok(mut h)) = (player.single_mut(), health.single_mut()) {
                k.body.pos.x -= 120.0;
                k.prev_pos = k.body.pos;
                h.hp = 0.0;
            }
            *step = 4;
        }
        4 if t > 6.5 => {
            if let Ok(k) = player.single_mut() {
                info!("sleep: woke at {:?} (feet y {:.0})", k.body.pos.round(), k.body.pos.y - k.body.half.y);
            }
            *step = 5;
        }
        _ => {}
    }
}

/// `build` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a small house put up
/// block by block as a player would (`PlaceBlock`), from 1000 on the
/// floor: a cobble footing, stone-brick walls with a stone wall behind
/// downstairs, plank floors, timber posts with plaster behind upstairs, a
/// stepped tile roof, a door's gap; the player stood off to its left.
/// Screenshots show how blocks draw themselves (`platypus_sim::look`).
/// Then furnished (chairs facing the table), a mason's bench and a sawmill
/// outside (`PLATYPUS_TRADES=1`: the player taken over to them);
/// `PLATYPUS_SLEEP=1`: `sleep_script`.
fn build_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature), Without<LocalPlayer>>,
    crafting: Res<crate::craft::Crafting>,
    mut done: Local<u8>,
) {
    if s.name != "build" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let b = platypus_sim::BLOCK;
    let fl = platypus_worldgen::arena::FLOOR;
    let g = fl.div_euclid(b);
    let x0 = 1000 / b;
    // Sized to the player (drawn ~33 cells tall): a ground floor of seven
    // blocks, an upper of six, a door of five.
    let (l, r) = (x0, x0 + 24);
    let (up, ceil) = (g + 8, g + 15);
    let bare = std::env::var("PLATYPUS_BARE").is_ok();
    // Then the furniture, once the blocks are in (`PLATYPUS_BARE=1`: none).
    if *done == 1 && t > 1.0 {
        *done = 2;
        if bare {
            return;
        }
        let at = |bx: i32, by: i32| Vec2::new((bx * b) as f32, (by * b) as f32);
        let size = |id: &str| crafting.station(id).map_or((8, 8), |k| crafting.stations[k].size());
        // (`flip`: facing left.)
        let mut put = |id: &str, feet: Vec2, flip: bool| {
            if let Some(kind) = crafting.station(id) {
                crafting.spawn(&mut commands, kind, feet, flip);
            }
        };
        let (f1, f2) = (g + 1, up + 1);
        let half = Vec2::new(b as f32 / 2.0, 0.0);
        put("potted_plant", at(l + 2, f1), false);
        put("barrel", at(l + 4, f1), false);
        put("wall_shelf", at(l + 3, f1 + 3) + half, false);
        put("table", at(l + 11, f1), false);
        put("chair", at(l + 8, f1), false);
        put("chair", at(l + 14, f1) + half, true);
        put("crate", at(l + 16, f1), false);
        put("hearth", at(r - 4, f1), false);
        put("painting", at(l + 11, f1 + 4), false);
        // Upstairs, mirrored about the middle: a bookcase, a banner, a
        // window, the chandelier over the platform, a window, a banner, the
        // bed. (A station's feet are its middle.)
        let mid = |bx: f32, id: &str| Vec2::new((l as f32 + bx) * b as f32, (f2 * b) as f32) + Vec2::new(0.0, 0.0 * size(id).0 as f32);
        put("bookcase", mid(1.9, "bookcase"), false);
        let banner = size("banner_red").1 as f32;
        let hang = |bx: f32| Vec2::new((l as f32 + bx) * b as f32, (ceil * b) as f32 - 2.0 - banner);
        put("banner_red", hang(5.4), false);
        put("banner_green", hang(19.6), false);
        put("bed", mid(21.9, "bed"), true);
        // Hung from the ceilings (their feet their height under it).
        let (ch, la) = (size("chandelier").1 as f32, size("hanging_lantern").1 as f32);
        put("chandelier", Vec2::new((l as f32 + 12.5) * b as f32, (ceil * b) as f32 - ch), false);
        put("hanging_lantern", at(l + 5, up) - Vec2::new(0.0, la), false);
        put("hanging_lantern", at(r - 8, up) - Vec2::new(0.0, la), false);
        // Outside, the trades: a mason's bench, a sawmill.
        put("mason_bench", at(r + 5, f1), false);
        put("sawmill", at(r + 12, f1), true);
        return;
    }
    // `PLATYPUS_TRADES=1`: then over to them, to look.
    if *done == 2 && t > 1.6 && std::env::var("PLATYPUS_TRADES").is_ok() {
        *done = 3;
        if let Ok(mut k) = player.single_mut() {
            k.body.pos = Vec2::new(((r + 8) * b) as f32, ((g + 1) * b) as f32 + k.body.half.y + 0.5);
            k.prev_pos = k.body.pos;
        }
    }
    if *done != 0 || t < 0.4 {
        return;
    }
    *done = 1;
    for (e, c) in &them {
        if c.kind == "dummy" || c.kind == "sandbag" {
            commands.entity(e).despawn();
        }
    }
    let id = |n: &str| sim.materials().expect_id(n);
    let (cobble, brick, planks, timber, roof, stone_wall, plaster, wood_wall) =
        (id("cobblestone"), id("brick"), id("planks"), id("timber"), id("roof_tiles"), id("stone_wall"), id("plaster"), id("wood_wall"));
    let mut fg: Vec<(i32, i32, platypus_sim::MaterialId)> = Vec::new();
    let mut bg: Vec<(i32, i32, platypus_sim::MaterialId)> = Vec::new();
    for x in l - 1..=r + 1 {
        fg.push((x, g, cobble));
    }
    // Ground floor: stone-brick walls, a stone wall behind; a door of five
    // on the left (`PLATYPUS_BARE=1`: a gap), brick over it.
    let door_top = g + 5;
    for y in g + 1..up {
        if y > door_top {
            fg.push((l, y, brick));
        } else if !bare {
            fg.push((l, y, id("door")));
        }
        fg.push((r, y, brick));
        for x in l + 1..r {
            bg.push((x, y, stone_wall));
        }
    }
    // The upper floor, a platform in its middle to jump up through (and
    // drop down).
    let platform = id("platform");
    for x in l - 1..=r + 1 {
        fg.push((x, up, if (l + 11..l + 14).contains(&x) { platform } else { planks }));
    }
    // (A wall behind the platform: under its half block shows the room.)
    for x in l + 11..l + 14 {
        bg.push((x, up, stone_wall));
    }
    // Upstairs: timber posts, plaster behind (behind the posts too).
    for y in up + 1..ceil {
        fg.push((l - 1, y, timber));
        fg.push((r + 1, y, timber));
        for x in l - 1..=r + 1 {
            bg.push((x, y, plaster));
        }
    }
    for x in l - 1..=r + 1 {
        fg.push((x, ceil, planks));
    }
    // A roof of tiles, two thick, a plank wall behind.
    let (mut lo, mut hi, mut y) = (l - 3, r + 3, ceil + 1);
    while lo <= hi {
        for x in lo..=hi {
            if x - lo < 2 || hi - x < 2 || y == ceil + 1 {
                fg.push((x, y, roof));
            }
            bg.push((x, y, wood_wall));
        }
        lo += 1;
        hi -= 1;
        y += 1;
    }
    // Windows (`PLATYPUS_PLAIN=1`: none): leaded downstairs, stained
    // upstairs (three blocks square), a rose in the gable (four, on its
    // own grid).
    if std::env::var("PLATYPUS_PLAIN").is_err() {
        let (leaded, stained, rose) = (id("leaded_glass"), id("stained_glass"), id("rose_glass"));
        let rose_x = (l + 12).div_euclid(4) * 4 - 2;
        let rose_y = (ceil + 3).div_euclid(4) * 4;
        let rose_x = rose_x.div_euclid(4) * 4;
        for e in bg.iter_mut() {
            let (x, y) = (e.0, e.1);
            // (Both floors' mirrored about the house's middle, l + 12.5.)
            let downstairs = (g + 3..g + 6).contains(&y) && ((l + 5..l + 8).contains(&x) || (l + 17..l + 20).contains(&x));
            let upstairs = (up + 2..up + 5).contains(&y) && ((l + 7..l + 10).contains(&x) || (l + 15..l + 18).contains(&x));
            if downstairs {
                e.2 = leaded;
            }
            if upstairs {
                e.2 = stained;
            }
            if (rose_x..rose_x + 4).contains(&x) && (rose_y..rose_y + 4).contains(&y) {
                e.2 = rose;
            }
        }
    }
    for (x, y, m) in bg {
        sim.queue(platypus_sim::WorldEdit::PlaceBlock { block: CellPos::new(x, y), material: m, back: true, most: u32::MAX });
    }
    for (x, y, m) in fg {
        sim.queue(platypus_sim::WorldEdit::PlaceBlock { block: CellPos::new(x, y), material: m, back: false, most: u32::MAX });
    }
    // The roof hammered into slopes (`PLATYPUS_STEPPED=1`: left stepped):
    // each row's outer block a ramp (◢ on the left, ◣ on the right; the
    // wall behind it too), its inner one hung underneath (◤, ◥). (A
    // hammer's blows: half, ◢, ◣, ◥, ◤.)
    if std::env::var("PLATYPUS_STEPPED").is_err() {
        let (mut lo, mut hi, mut y) = (l - 3, r + 3, ceil + 1);
        let mut hammer = |x: i32, y: i32, blows: usize, back: bool| {
            for _ in 0..blows {
                sim.queue(platypus_sim::WorldEdit::ShapeBlock { block: CellPos::new(x, y), back });
            }
        };
        while hi - lo >= 1 {
            for back in [false, true] {
                hammer(lo, y, 2, back);
                hammer(hi, y, 3, back);
            }
            if y > ceil + 1 && hi - lo > 3 {
                hammer(lo + 1, y, 5, false);
                hammer(hi - 1, y, 4, false);
            }
            lo += 1;
            hi -= 1;
            y += 1;
        }
    }
    if let Ok(mut k) = player.single_mut() {
        k.body.pos = Vec2::new(((l - 8) * b) as f32, fl as f32 + k.body.half.y);
        k.prev_pos = k.body.pos;
    }
    // Lamps only in a bare house (`PLATYPUS_NOLAMPS=1`: none): furnished,
    // its lanterns and fire light it.
    if bare && std::env::var("PLATYPUS_NOLAMPS").is_err() {
        for (bx, by) in [(l + 6, g + 6), (l + 13, up + 5)] {
            let at = Vec2::new(((bx * b) + b / 2) as f32, (by * b) as f32);
            commands.spawn((Transform::from_translation(at.extend(0.0)), GlobalTransform::default(), crate::light::LightSource { color: [1.0, 0.78, 0.5], flicker: 0.1 }));
        }
    }
    info!("build: a house queued from block {l} to {r}");
}

/// `lineup` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): creatures put in a
/// row 70 cells apart from 1000, held where they stand (`PLATYPUS_LINEUP`,
/// comma-separated kinds; the deeper kin by default); logs each, its
/// health and how it takes slash, blunt, fire and frost.
fn lineup_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut them: Query<(Entity, &crate::creatures::Creature, &mut Kinematics, &crate::creatures::Health), Without<LocalPlayer>>,
    mut state: Local<u8>,
) {
    if s.name != "lineup" {
        return;
    }
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let kinds: Vec<String> = std::env::var("PLATYPUS_LINEUP").unwrap_or_else(|_| "plague_rat,venom_centipede,zombie_crimson,skeleton_archer,armored_skeleton,frost_wraith,ember_bat".into()).split(',').map(|k| k.trim().to_string()).collect();
    if *state == 0 && t > 0.4 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        for (i, kind) in kinds.iter().enumerate() {
            crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(1000.0 + i as f32 * 70.0, fl + 2.0), |_| {});
        }
        *state = 1;
    }
    // (The player held up over the middle of the row, out of reach, the
    // camera on them; each held where it was put.)
    if let Ok(mut k) = player.single_mut() {
        k.body.pos = Vec2::new(1000.0 + (kinds.len() as f32 - 1.0) * 35.0, fl + 70.0);
        k.body.vel = Vec2::ZERO;
    }
    for (_, c, mut k, _) in &mut them {
        if let Some(i) = kinds.iter().position(|x| *x == c.kind) {
            k.body.pos.x = 1000.0 + i as f32 * 70.0;
            k.body.vel.x = 0.0;
        }
    }
    if *state == 1 && t > 1.5 {
        *state = 2;
        for (_, c, _, h) in &them {
            if kinds.contains(&c.kind) {
                use crate::creatures::Harm;
                info!("lineup: {}: {:.0} health; slash ×{:.2}, blunt ×{:.2}, fire ×{:.2}, frost ×{:.2}", c.kind, h.max, h.nature.of(Harm::Slash), h.nature.of(Harm::Blunt), h.nature.of(Harm::Fire), h.nature.of(Harm::Frost));
            }
        }
    }
}

/// `wraith` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat PLATYPUS_HOUR=22`):
/// a chain wraith at 1150, 60 up; the player (who can't die) stands at 900.
/// `PLATYPUS_WRAITH`: (none) let be: it comes, lashes, pulls, rakes;
/// `water`: a pool from 960 to 1060 between them (it stops at it);
/// `dash`: hooked, the player dashes (free); `kill`: struck till it dies
/// (its chains drop); `rock`: a slab of stone at 1000..1060 it drifts
/// through. Logs what it does, where, its health, the player's lost.
fn wraith_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, &crate::creatures::Health, Option<&crate::creatures::custom::wraith::Wraith>), Without<LocalPlayer>>,
    drops: Query<&crate::hands::Dropped>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, f32, f32, &'static str)>,
    (inv, items): (Query<&crate::hands::items::Inventory, With<LocalPlayer>>, Option<Res<crate::hands::items::Items>>),
) {
    if s.name != "wraith" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let floor = platypus_worldgen::arena::FLOOR;
    let mode = std::env::var("PLATYPUS_WRAITH").unwrap_or_default();
    state.2 += h.max - h.hp;
    h.hp = h.max;
    let wraith = them.iter().find(|x| x.1.kind == "chain_wraith");
    if state.0 == 0 && t > 0.5 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        k.body.pos = Vec2::new(900.0, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        let paint = |sim: &mut SimWorld, name: &str, x0: i32, x1: i32, y0: i32, y1: i32| {
            if let Some(m) = sim.materials().id(name) {
                for x in x0..x1 {
                    for y in y0..y1 {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: m, overwrite: true });
                    }
                }
            }
        };
        if mode == "water" {
            paint(&mut sim, "water", 960, 1060, floor - 20, floor + 2);
        }
        if mode == "rock" {
            paint(&mut sim, "stone", 1000, 1060, floor, floor + 120);
        }
        crate::creatures::def::spawn_creature(&mut commands, "chain_wraith", Vec2::new(1150.0, fl + 60.0), |_| {});
        info!("wraith: a chain wraith at 1150 ({mode})");
        state.0 = 1;
        return;
    }
    let Some((e, _, wk, wh, w)) = wraith else {
        if state.0 == 1 && t > 1.5 {
            state.0 = 2;
            state.1 = t;
        }
        if state.0 == 2 && t > state.1 + 1.0 {
            let held = items.as_ref().and_then(|i| i.id("wraith_chain")).zip(inv.single().ok()).map_or(0, |(id, inv)| inv.count(id));
            info!("wraith: t {t:.1} gone; things dropped about: {}, wraith chains carried: {held}", drops.iter().count());
            state.0 = 9;
        }
        return;
    };
    let doing = w.map_or("?", |w| w.doing());
    if doing != state.3 {
        info!("wraith: t {t:.2} {} at ({:.0}, {:.0}); the player at {:.0}, lost {:.0}", doing, wk.body.pos.x, wk.body.pos.y - fl, k.body.pos.x, state.2);
        state.3 = doing;
    }
    if mode == "dash" {
        if doing == "pull" { keys.press(KeyCode::ShiftLeft) } else { keys.release(KeyCode::ShiftLeft) }
    }
    if mode == "kill" && t > 3.0 && t > state.1 {
        hits.write(crate::combat::Hit { by: None, target: e, damage: 40.0, harm: crate::creatures::Harm::Radiant, knock: Vec2::ZERO, stun: 0.0, at: wk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
        state.1 = t + 0.5;
    }
    if (t * 2.0).fract() < 0.02 && t > 1.0 {
        info!("wraith: t {t:.1} {doing} at ({:.0}, {:.0}) hp {:.0}; the player lost {:.0}", wk.body.pos.x, wk.body.pos.y - fl, wh.hp, state.2);
    }
}

/// `mimic` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a chest at 980
/// and a mimic at 1060 (it looks the same); the player (who can't die)
/// walks right from 900 from 2 s, past the chest, to the mimic; it wakes
/// and fights. `PLATYPUS_MIMIC=open`: the player stops short and right
/// clicks it (to open it); `kill`: struck till it dies (it leaves a
/// chest). Logs its state, the player's health lost.
fn mimic_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, &crate::creatures::Health, Has<crate::creatures::Disguised>), Without<LocalPlayer>>,
    chest_q: Query<&Kinematics, (With<crate::hands::chests::Chest>, Without<LocalPlayer>)>,
    mut chests: ResMut<crate::hands::chests::Chests>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut cursor: ResMut<CursorOverride>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, f32, f32)>,
) {
    if s.name != "mimic" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let mode = std::env::var("PLATYPUS_MIMIC").unwrap_or_default();
    state.2 += h.max - h.hp;
    h.hp = h.max;
    let mimic = them.iter().find(|x| x.1.kind == "mimic");
    match state.0 {
        0 if t > 0.5 => {
            for (e, c, ..) in &them {
                if c.kind == "dummy" || c.kind == "sandbag" {
                    commands.entity(e).despawn();
                }
            }
            k.body.pos = Vec2::new(900.0, fl + k.body.half.y);
            k.prev_pos = k.body.pos;
            chests.spawn_found(&mut commands, CellPos::new(980, platypus_worldgen::arena::FLOOR));
            crate::creatures::def::spawn_creature(&mut commands, "mimic", Vec2::new(1060.0, fl), |_| {});
            info!("mimic: a chest at 980, a mimic at 1060");
            state.0 = 1;
        }
        1 if t > 2.0 => {
            let stop = if mode == "open" { 1030.0 } else { 1200.0 };
            if k.body.pos.x < stop && mimic.is_some_and(|m| m.4) {
                keys.press(KeyCode::KeyD);
            } else {
                keys.release(KeyCode::KeyD);
                if mode == "open" && mimic.is_some_and(|m| m.4) {
                    if let Some(m) = mimic {
                        cursor.0 = Some(m.2.body.pos);
                    }
                    mouse.press(MouseButton::Right);
                }
            }
            if mimic.is_some_and(|m| !m.4) {
                keys.release(KeyCode::KeyD);
                mouse.release(MouseButton::Right);
                info!("mimic: t {t:.1} it woke, the player at {:.0}", k.body.pos.x);
                state.0 = 2;
                state.1 = t;
            }
            if t > 8.0 {
                info!("mimic: it never woke (the player at {:.0})", k.body.pos.x);
                state.0 = 9;
            }
        }
        2 if mode == "kill" && t > state.1 + 0.5 => {
            if let Some((e, _, mk, _, _)) = mimic {
                hits.write(crate::combat::Hit { by: None, target: e, damage: 40.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at: mk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
                state.1 = t;
            } else {
                info!("mimic: t {t:.1} dead; chests in the world: {}", chest_q.iter().count());
                state.0 = 9;
            }
        }
        _ => {}
    }
    if t > state.1 + 1.0 && state.0 >= 1 && state.0 < 9 && (t * 2.0).fract() < 0.02 {
        if let Some((_, _, mk, mh, shut)) = mimic {
            info!("mimic: t {t:.1} {} at {:.0} ({:.0} up), hp {:.0}; the player at {:.0}, lost {:.0}", if shut { "shut" } else { "awake" }, mk.body.pos.x, mk.body.pos.y - mk.body.half.y - fl, mh.hp, k.body.pos.x, state.2);
        }
    }
}

/// `structure` (the real world): the player dropped into the middle of
/// the nearest `PLATYPUS_STRUCTURE` (castle, the default; crypt) to the
/// spawn, kept alive (for a look: `PLATYPUS_ZOOM=1`, `PLATYPUS_SHOTS`).
fn structure_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    mut done: Local<bool>,
    mut told: Local<bool>,
    near: Query<(&Kinematics, Option<&crate::creatures::Creature>, Has<crate::hands::chests::Chest>), Without<LocalPlayer>>,
) {
    if s.name != "structure" {
        return;
    }
    let Ok((mut pk, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    // (A while after: the chests and mimics round it.)
    if *done && !*told && s.elapsed > 5.0 {
        *told = true;
        let close = |k: &Kinematics| k.body.pos.distance(pk.body.pos) < 500.0;
        let chests = near.iter().filter(|(k, _, c)| *c && close(k)).count();
        let mimics = near.iter().filter(|(k, c, _)| c.is_some_and(|c| c.kind == "mimic") && close(k)).count();
        info!("structure: within 500: {chests} chests, {mimics} mimics");
    }
    if *done || s.elapsed < 0.5 {
        return;
    }
    *done = true;
    let want = std::env::var("PLATYPUS_STRUCTURE").unwrap_or_else(|_| "castle".into());
    let home = sim.generator.spawn_point();
    let all: Vec<(CellPos, String)> = sim.generator.landmarks().into_iter().filter(|(_, n)| *n == want).collect();
    match all.iter().min_by_key(|(p, _)| (p.x - home.x).abs()) {
        Some((p, _)) => {
            pk.body.pos = Vec2::new(p.x as f32, p.y as f32);
            pk.prev_pos = pk.body.pos;
            pk.body.vel = Vec2::ZERO;
            info!("structure: the player in the {want} at ({}, {}), {} from the spawn ({} of them)", p.x, p.y, p.x - home.x, all.len());
        }
        None => info!("structure: no {want} in this world"),
    }
}

/// `slimecheck` (the real world; `PLATYPUS_LIFE_RATE=8` for more): the
/// player put in a cave `PLATYPUS_SLIMECHECK_DEPTH` (300) cells under the
/// spawn (the nearest open spot), kept alive; every 3 s each slime near:
/// how much of its box is in solid ground, the goo it holds, how far it's
/// moved since the last look.
fn slimecheck_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::body::goo::Goo>), Without<LocalPlayer>>,
    mut state: Local<(bool, f32, std::collections::HashMap<Entity, Vec2>)>,
) {
    if s.name != "slimecheck" {
        return;
    }
    let Ok((mut pk, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let world = &sim.world;
    let mats = world.materials();
    let solid = |x: i32, y: i32| world.get(CellPos::new(x, y)).is_some_and(|c| !c.is_air() && matches!(mats.phys(c.material).kind, platypus_sim::Kind::Static | platypus_sim::Kind::Powder));
    if !state.0 && s.elapsed > 1.0 {
        let depth: i32 = std::env::var("PLATYPUS_SLIMECHECK_DEPTH").ok().and_then(|v| v.parse().ok()).unwrap_or(300);
        let sp = sim.generator.spawn_point();
        let top = sim.generator.surface_hint(sp.x).unwrap_or(sp.y);
        // (The nearest open spot round there, roomy enough to stand in.)
        let spot = (0..400).step_by(4).flat_map(|r| [(r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, -r), (r, -r), (-r, r)]).map(|(dx, dy)| (sp.x + dx, top - depth + dy)).find(|&(x, y)| (-6..=6).all(|dx| (0..30).all(|dy| !solid(x + dx, y + dy))) && solid(x, y - 1));
        match spot {
            Some((x, y)) => {
                pk.body.pos = Vec2::new(x as f32 + 0.5, y as f32 + pk.body.half.y);
                pk.prev_pos = pk.body.pos;
                pk.body.vel = Vec2::ZERO;
                info!("slimecheck: the player in a cave at ({x}, {y}), {} under the surface", top - y);
            }
            None => info!("slimecheck: no cave found {depth} under the spawn"),
        }
        state.0 = true;
        state.1 = s.elapsed + 3.0;
        return;
    }
    if !state.0 || s.elapsed < state.1 {
        return;
    }
    state.1 = s.elapsed + 3.0;
    let mut lines = Vec::new();
    for (e, c, k, goo) in &them {
        if !(c.kind.contains("slime") || c.kind == "glutton") || k.body.pos.distance(pk.body.pos) > 900.0 {
            continue;
        }
        let (lo, hi) = (k.body.pos - k.body.half, k.body.pos + k.body.half);
        let (mut n, mut inside) = (0, 0);
        let mut y = lo.y.ceil() as i32;
        while (y as f32) < hi.y {
            let mut x = lo.x.ceil() as i32;
            while (x as f32) < hi.x {
                n += 1;
                if solid(x, y) {
                    inside += 1;
                }
                x += 2;
            }
            y += 2;
        }
        let was = state.2.insert(e, k.body.pos);
        let moved = was.map_or(-1.0, |w| w.distance(k.body.pos));
        // (PLATYPUS_SLIMECHECK_VISIT: the player beside the first that
        // hasn't moved, to see it.)
        if moved == 0.0 && std::env::var("PLATYPUS_SLIMECHECK_VISIT").is_ok() && !lines.iter().any(|l: &String| l.starts_with("visiting")) && s.elapsed > 12.0 && s.elapsed < 16.0 {
            pk.body.pos = k.body.pos + Vec2::new(-70.0, 30.0);
            pk.prev_pos = pk.body.pos;
            pk.body.vel = Vec2::ZERO;
            lines.push(format!("visiting the {} at ({:.0}, {:.0})", c.kind, k.body.pos.x, k.body.pos.y));
        }
        lines.push(format!("{} box {:.0}x{:.0} {}% in rock, goo {}, moved {:.0}, grounded {}", c.kind, k.body.half.x * 2.0, k.body.half.y * 2.0, inside * 100 / n.max(1), goo.map_or(0, |g| g.held()), moved, k.loco.grounded()));
    }
    info!("slimecheck: t {:.0}: {} slimes near{}", s.elapsed, lines.len(), lines.iter().map(|l| format!("\n  {l}")).collect::<String>());
}

/// `leap` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`, with any
/// `PLATYPUS_TEMPO`): how the player moves at this tempo. A full jump
/// (1 s: how high, how long in the air), a dash on the ground (3 s: how
/// far), then a long fall from high up (lifted back up on the way) with a
/// dash at once and another 0.9 s on (how many came: an air dash back in
/// the air, or not); then on its rocket boots, jump held and steering
/// right (how far, how fast sideways, and whether it's kept after).
fn leap_script(
    s: Res<Scenario>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut dashed: MessageReader<crate::combat::Dashed>,
    tempo: Res<crate::tempo::Tempo>,
    // (phase, the highest, when it left the ground, x where it dashed, dashes)
    mut state: Local<(u8, f32, f32, f32, u32)>,
) {
    if s.name != "leap" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let t = s.elapsed;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let feet = k.body.pos.y - k.body.half.y;
    let dashes = dashed.read().count() as u32;
    keys.release(KeyCode::ShiftLeft);
    let tap = |keys: &mut ButtonInput<KeyCode>| keys.press(KeyCode::ShiftLeft);
    match state.0 {
        0 if t > 0.8 => {
            k.body.pos = Vec2::new(900.0, fl + k.body.half.y);
            k.prev_pos = k.body.pos;
            k.body.vel = Vec2::ZERO;
            info!("leap: tempo {}", tempo.name());
            state.0 = 1;
        }
        1 if t > 1.0 => {
            keys.press(KeyCode::Space);
            state.1 = 0.0;
            state.2 = t;
            state.0 = 2;
        }
        2 => {
            if t - state.2 > 0.6 {
                keys.release(KeyCode::Space);
            }
            state.1 = state.1.max(feet - fl);
            if t - state.2 > 0.1 && k.loco.grounded() {
                info!("leap: a full jump rose {:.0} cells ({:.1} bodies), {:.2} s in the air", state.1, state.1 / 23.0, t - state.2);
                state.0 = 3;
            }
        }
        3 if t > 3.0 => {
            tap(&mut keys);
            state.3 = k.body.pos.x;
            state.2 = t;
            state.0 = 4;
        }
        4 if t - state.2 > 0.6 => {
            info!("leap: a dash on the ground went {:.0} cells", k.body.pos.x - state.3);
            k.body.pos = Vec2::new(900.0, fl + 380.0);
            k.prev_pos = k.body.pos;
            k.body.vel = Vec2::ZERO;
            state.4 = 0;
            state.2 = t;
            state.0 = 5;
        }
        5 if t - state.2 > 0.1 => {
            tap(&mut keys);
            info!("leap: dash at {:.0} up", feet - fl);
            state.0 = 6;
        }
        // (Lifted back up as it falls: still in the air, its air dash as
        // it was.)
        6 if t - state.2 > 0.6 && feet - fl < 250.0 && t - state.2 < 0.9 => {
            k.body.pos.y += 200.0;
            k.prev_pos = k.body.pos;
        }
        6 if t - state.2 > 1.0 => {
            tap(&mut keys);
            info!("leap: dash at {:.0} up ({} so far)", feet - fl, state.4);
            state.0 = 7;
        }
        7 if t - state.2 > 1.3 => {
            info!("leap: falling, two dashes 0.9 s apart: {} came ({})", state.4, if k.loco.grounded() { "it had landed" } else { "still in the air" });
            state.0 = 8;
        }
        // On rocket boots (what it wears): from 120 up, after 0.2 s, jump pressed and
        // held, steering right, 1.6 s; then let go of jump, still steering
        // (its speed carried?).
        8 if k.loco.grounded() && t - state.2 > 2.0 => {
            k.body.pos = Vec2::new(300.0, fl + 120.0);
            k.prev_pos = k.body.pos;
            k.body.vel = Vec2::ZERO;
            state.3 = 300.0;
            state.1 = 0.0;
            state.2 = t;
            state.0 = 9;
        }
        9 => {
            // (Pressed in the air and held: the boots.)
            let d = t - state.2;
            if d < 0.2 { keys.release(KeyCode::Space) } else { keys.press(KeyCode::Space) }
            keys.press(KeyCode::KeyD);
            state.1 = state.1.max(k.body.vel.x);
            if t - state.2 > 1.8 {
                keys.release(KeyCode::Space);
                info!("leap: t {t:.1} rocketing 1.6 s: {:.0} cells across, {:.0} up, at most {:.0} cells/s sideways", k.body.pos.x - state.3, feet - fl, state.1);
                state.0 = 10;
            }
        }
        10 => {
            keys.press(KeyCode::KeyD);
            if t - state.2 > 2.1 {
                info!("leap: 0.3 s after, still steering: {:.0} cells/s sideways", k.body.vel.x);
                keys.release(KeyCode::KeyD);
                state.0 = 11;
            }
        }
        _ => {}
    }
    if state.0 >= 5 {
        state.4 += dashes;
    }
}


/// The down-strike (DESIGN §8): a shortsword's down slash chained across
/// the three dummies with S held and the cursor off level ahead; the
/// longsword's plunges onto a dummy (the dive, the bounce, another plunge
/// on the way down again: a click each), then off a spider's spit thrown up
/// at it; a bounce off the lava (the sandbox's pit; a pool is made there in
/// a layout without it); a plunge onto the floor beside a dummy (the slam).
/// (A plunge is live for only so far of its fall, `PLUNGE_FALL`: each is
/// begun within that of what it's to strike.)
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
    mut sim: ResMut<SimWorld>,
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
    let dummy_top = |x: f32| dummies.iter().find(|(_, c, dk, _)| c.kind == "dummy" && (dk.body.pos.x - x).abs() < 30.0).map(|d| d.2.body.pos.y + d.2.body.half.y);
    // A bounce: rising fast just after falling.
    if k.body.vel.y > 225.0 && state.4 < 90.0 {
        state.2 += 1;
    }
    // (Diving now, and not a moment ago.)
    let dove = k.body.vel.y < -700.0 && state.3 >= -700.0;
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
        // The shortsword (hotbar 2, slot 8).
        0 => {
            if (0.3..0.4).contains(&t) {
                want.push(KeyCode::KeyX);
            }
            if (0.5..0.6).contains(&t) {
                want.push(KeyCode::Digit8);
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
        // The longsword (slot 9); up over the middle dummy.
        2 => {
            if dt < 0.1 {
                want.push(KeyCode::Digit9);
            }
            if dt > 0.3 {
                put(&mut k, Vec2::new(1140.0, fl + 165.0));
                next(&mut state);
                state.5 = all();
            }
        }
        // Plunges: dropped from 165 up, S held; a click as it falls within
        // reach of the dummy's top (its windup's fall and then 45 cells:
        // the plunge, live for 70, meets it), and again each time it falls
        // back after a bounce. At 2 s S let go and no more clicks: down.
        3 => {
            cursor.0 = Some(p + Vec2::new(0.0, -45.0));
            if dt < 2.0 {
                want.push(KeyCode::KeyS);
                if let Some(top) = dummy_top(1140.0) {
                    // (The windup, 0.08 s, falls on at this speed.)
                    let gap = p.y - k.body.half.y - top + k.body.vel.y * 0.08;
                    click = !k.loco.grounded() && k.body.vel.y < 0.0 && gap < 45.0;
                }
            }
            if dove {
                info!("pogo: plunging: a down-strike {}, falling at {:.0} cells/s, {:.0} above the floor", swing.is_some_and(|s| s.downward()), -k.body.vel.y, p.y - k.body.half.y - fl);
            }
            if dt > 3.0 {
                info!("pogo: longsword plunges, dropped from 165 up: fastest fall {:.0} cells/s, {} bounces (a click each, S held 2 s, then let go), the dummy struck {} times", -state.3, state.2, all() - state.5);
                next(&mut state);
            }
        }
        // Off a spider's spit: thrown up at the player as it plunges, off
        // to the side of the dummies (low enough that they meet while the
        // plunge is live).
        4 => {
            if dt < 0.05 {
                put(&mut k, Vec2::new(930.0, fl + 115.0));
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
        // The shortsword, over the lava: S, and strike above it. (A layout
        // without the pit, `flat`: a pool of lava sunk into the floor there.)
        5 => {
            if dt < 0.1 {
                want.push(KeyCode::Digit8);
                let lava = sim.materials().id("lava");
                let at = CellPos::new(1395, fl as i32 - 12);
                if let Some(lava) = lava
                    && sim.world.get(at).is_some_and(|c| c.material != lava)
                {
                    sim.queue(WorldEdit::Dig { center: CellPos::new(1395, fl as i32 - 9), radius: 15, max_hardness: u8::MAX });
                    sim.queue(WorldEdit::Paint { center: CellPos::new(1395, fl as i32 - 15), radius: 15, material: lava, overwrite: false });
                }
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
                want.push(KeyCode::Digit9);
            }
            // (Once the last swing's done and the longsword's in hand; low
            // enough to land within the plunge's live fall.)
            if (0.5..0.55).contains(&dt) {
                put(&mut k, Vec2::new(1209.0, fl + 70.0));
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
    for key in [KeyCode::KeyX, KeyCode::Digit8, KeyCode::Digit9, KeyCode::KeyA, KeyCode::KeyD, KeyCode::KeyS] {
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

/// Takes up the item `id` from wherever it is in the pack: its hotbar, its
/// slot. (By id, not by slot: the pack's start changes, and a slot's item
/// with it: scenarios that held a slot drifted onto boots and rings.)
fn t_before(s: &Scenario, t: f32) -> bool {
    s.elapsed < t
}

fn take_up(hand: &mut crate::hands::Hand, inv: &crate::hands::items::Inventory, items: Option<&crate::hands::items::Items>, id: &str) {
    let Some(item) = items.and_then(|i| i.id(id)) else { return };
    if let Some(i) = inv.slots.iter().position(|st| st.is_some_and(|st| st.item == item)) {
        hand.bar = i / crate::hands::items::HOTBAR;
        hand.slot = i % crate::hands::items::HOTBAR;
    }
}

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
    (inv, items): (Query<&crate::hands::items::Inventory, With<LocalPlayer>>, Option<Res<crate::hands::items::Items>>),
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
    let mut hold = |id: &str| {
        if let Ok(inv) = inv.single() {
            take_up(&mut hand, inv, items.as_deref(), id);
        }
    };
    let sweep = |a: f32, b: f32, from: f32, secs: f32| a + (b - a) * ((t - from) / secs).clamp(0.0, 1.0);
    match state.0 {
        0 if t > 0.5 => {
            info!("beams: the pool's ice {} cells before", count("ice", 450..645, floor - 78..floor + 3));
            state.0 = 1;
        }
        // The frost ray, along the pool from its left bank.
        1 => {
            hold("frost_staff");
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
            hold("frost_staff");
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
            hold("blaze_staff");
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
            hold("radiant_staff");
            stand(960.0);
            cursor.0 = Some(Vec2::new(sweep(990.0, 1012.0, 7.2, 1.6), floor as f32 - 45.0));
            left = t > 7.2;
            if t > 8.8 {
                info!("beams: vaporiser into the floor 1.6 s: {} solid cells gone", state.1 as usize - solid(900..1020, floor - 60..floor));
                state.0 = 5;
            }
        }
        5 => {
            hold("radiant_staff");
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
            hold("radiant_staff");
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
    let Ok((mut k, inv, mut hp)) = player.single_mut() else { return };
    let t = s.elapsed;
    let floor = platypus_worldgen::arena::FLOOR;
    let count = |name: &str, xs: std::ops::Range<i32>, ys: std::ops::Range<i32>| {
        let Some(m) = sim.materials().id(name) else { return 0 };
        xs.flat_map(|x| ys.clone().map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).count()
    };
    // (The player isn't what's being tested: kept standing and whole; past
    // the ice wall for the cloud and the fire ray, the orc behind it in the
    // cloud's reach, 225.)
    k.body.pos.x = if state.0 >= 4 { 1020.0 } else { 900.0 };
    hp.hp = hp.max;
    let orc = |e: Option<Entity>| e.and_then(|e| orcs.get(e).ok()).map(|(_, k, h, b)| (k.body.pos.x, h.hp, b));
    let (mut left, mut right) = (false, false);
    match state.0 {
        0 if t > 0.5 => {
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
            take_up(&mut hand, &inv, items.as_deref(), "frost_staff");
            cursor.0 = Some(Vec2::new(975.0, floor as f32 + 30.0));
            right = (0.8..0.9).contains(&t);
            if t > 3.0 {
                info!("conjure: ice wall at x 970..980: ice {} cells; the orc behind it at x {:?}", count("ice", 960..990, floor..floor + 60), orc(state.1).map(|o| o.0));
                state.0 = 3;
            }
        }
        // A fire wall the left-hand orc walks into.
        3 => {
            take_up(&mut hand, &inv, items.as_deref(), "blaze_staff");
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
            take_up(&mut hand, &inv, items.as_deref(), "acid_staff");
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
            take_up(&mut hand, &inv, items.as_deref(), "blaze_staff");
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
                let lava = sim.materials().id("magma").map_or(0, |m| (1050..1350).flat_map(|x| (floor - 60..floor + 60).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == m)).count());
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
            // (A jump, let go, then held: the boots fire on a second press.)
            jump = dt < 0.15 || (0.25..1.2).contains(&dt) || brake && dt > 1.5 && height < 165.0 && k.body.vel.y < -60.0;
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
            crafting.spawn(&mut commands, bench, Vec2::new(960.0, floor as f32), false);
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

/// `pack` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat PLATYPUS_MODE=play`;
/// `PLATYPUS_SHOTS=dir` for a picture of each step): the inventory screen,
/// Terraria's way. A messy pack (stacks split, gaps, ore, bars, a helm) and
/// a workbench, furnace and anvil beside the player; then, a step every
/// half second or so: Esc (the inventory opens), Quick sort (logs the pack
/// before and after), Crafting (the panel), an iron pickaxe pointed at, the
/// Tools tab, a copper pickaxe made (logs how many before and after), the
/// Materials tab and an iron bar pointed at, Esc (the panel shuts, the
/// inventory stays), Settings (the game menu up, time stopped), Esc (back
/// to play), Esc and I (open, shut). The buttons are pressed as the UI
/// would (no cursor offscreen). Logs each step.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn pack_script(
    mut commands: Commands,
    s: Res<Scenario>,
    items: Option<Res<crate::hands::items::Items>>,
    crafting: Res<crate::craft::Crafting>,
    (open, panel): (Res<crate::hands::InventoryOpen>, Res<crate::hands::crafting::CraftingPanel>),
    (menu, virt): (Res<crate::pause::GameMenu>, Res<Time<Virtual>>),
    offscreen: Option<Res<crate::camera::Offscreen>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut player: Query<(&Kinematics, &mut crate::hands::items::Inventory), With<LocalPlayer>>,
    mut ui: Query<(&mut Interaction, Option<&crate::hands::ui::PackButton>, Option<&crate::hands::crafting::Tab>, Option<&crate::hands::crafting::RecipeSlot>)>,
    mut state: Local<(u8, f32)>,
) {
    use crate::hands::crafting::Tab;
    use crate::hands::items::{Category, Stack};
    use crate::hands::ui::PackButton;
    if s.name != "pack" {
        return;
    }
    let (Some(items), Ok((k, mut inv))) = (items, player.single_mut()) else { return };
    let t = s.elapsed;
    keys.release(KeyCode::Escape);
    keys.release(KeyCode::KeyI);
    let pack = |inv: &crate::hands::items::Inventory| -> String {
        inv.slots[10..].iter().map(|s| s.map_or("-".to_string(), |s| format!("{}x{}", items.def(s.item).id, s.count / items.unit(s.item)))).collect::<Vec<_>>().join(" ")
    };
    let have = |inv: &crate::hands::items::Inventory, id: &str| items.id(id).map_or(0, |i| inv.count(i) / items.unit(i));
    let says = || format!("inventory {}, crafting {}, game menu {}, time {}", if open.0 { "open" } else { "shut" }, if panel.open && open.0 { "up" } else { "shut" }, if menu.open { "up" } else { "shut" }, if virt.is_paused() { "paused" } else { "running" });
    // A click on a button, as the UI makes one (and a recipe pointed at).
    let mut press = |which: &dyn Fn(Option<&PackButton>, Option<&Tab>, Option<usize>) -> bool, how: Interaction| {
        for (mut i, b, tab, r) in &mut ui {
            if which(b, tab, r.map(|r| r.0)) {
                *i = how;
            }
        }
    };
    let recipe = |makes: &str| crafting.recipe(makes);
    let mut shot = |name: &str| {
        let Ok(dir) = std::env::var("PLATYPUS_SHOTS") else { return };
        let image = match &offscreen {
            Some(o) => Screenshot::image(o.0.clone()),
            None => Screenshot::primary_window(),
        };
        commands.spawn(image).observe(save_to_disk(format!("{dir}/pack_{name}.png")));
    };
    if t < state.1 {
        return;
    }
    let next = |state: &mut (u8, f32), wait: f32| *state = (state.0 + 1, t + wait);
    match state.0 {
        0 if t > 1.0 => {
            let messy: [(usize, &str, u32); 16] = [
                (12, "torch", 20),
                (15, "block:stone", 30),
                (17, "iron_bar", 7),
                (21, "torch", 35),
                (23, "block:wood", 10),
                (26, "iron_helm", 1),
                (30, "block:copper_ore", 9),
                (33, "copper_bar", 4),
                (34, "iron_bar", 9),
                (38, "block:planks", 12),
                (41, "bomb", 3),
                (44, "small_health_potion", 1),
                (47, "block:stone", 40),
                (52, "block:coal", 5),
                (55, "copper_bar", 4),
                (58, "block:amethyst", 3),
            ];
            for (slot, id, n) in messy {
                if let Some(item) = items.id(id) {
                    inv.slots[slot] = Some(Stack::new(item, n * items.unit(item)));
                }
            }
            let floor = platypus_worldgen::arena::FLOOR as f32;
            for (id, dx) in [("workbench", 24.0), ("furnace", 46.0), ("anvil", -26.0)] {
                if let Some(kind) = crafting.station(id) {
                    crafting.spawn(&mut commands, kind, Vec2::new(k.body.pos.x + dx, floor), false);
                }
            }
            next(&mut state, 0.5);
        }
        1 => {
            keys.press(KeyCode::Escape);
            next(&mut state, 0.4);
        }
        2 => {
            info!("pack: Esc: {}", says());
            info!("pack: before quick sort: {}", pack(&inv));
            shot("1_open");
            next(&mut state, 0.3);
        }
        3 => {
            press(&|b, _, _| b == Some(&PackButton::Sort), Interaction::Pressed);
            next(&mut state, 0.3);
        }
        4 => {
            info!("pack: after quick sort: {}", pack(&inv));
            shot("2_sorted");
            next(&mut state, 0.3);
        }
        5 => {
            press(&|b, _, _| b == Some(&PackButton::Crafting), Interaction::Pressed);
            next(&mut state, 0.3);
        }
        6 => {
            let r = recipe("iron_pickaxe");
            press(&|_, _, slot| slot.is_some() && slot == r, Interaction::Hovered);
            next(&mut state, 0.3);
        }
        7 => {
            info!("pack: Crafting: {}; {} recipes listed, {} can be made; picked {:?}", says(), panel.list.len(), panel.list.iter().filter(|(_, c)| *c).count(), panel.picked.map(|i| &crafting.recipes[i].makes));
            shot("3_crafting");
            next(&mut state, 0.3);
        }
        8 => {
            press(&|_, tab, _| tab == Some(&Tab(Some(Category::Tools))), Interaction::Pressed);
            next(&mut state, 0.3);
        }
        9 => {
            let tools: Vec<&str> = panel.in_tab(&crafting, &items).iter().map(|(i, _)| crafting.recipes[*i].makes.as_str()).collect();
            info!("pack: the Tools tab: {}", tools.join(", "));
            shot("4_tools");
            next(&mut state, 0.3);
        }
        10 => {
            info!("pack: copper pickaxes before: {}", have(&inv, "copper_pickaxe"));
            let r = recipe("copper_pickaxe");
            press(&|_, _, slot| slot.is_some() && slot == r, Interaction::Pressed);
            next(&mut state, 0.3);
        }
        11 => {
            info!("pack: copper pickaxes after: {}; copper bars {}", have(&inv, "copper_pickaxe"), have(&inv, "copper_bar"));
            shot("5_made");
            next(&mut state, 0.3);
        }
        12 => {
            press(&|_, tab, _| tab == Some(&Tab(Some(Category::Materials))), Interaction::Pressed);
            next(&mut state, 0.2);
        }
        13 => {
            let r = recipe("iron_bar");
            press(&|_, _, slot| slot.is_some() && slot == r, Interaction::Hovered);
            next(&mut state, 0.3);
        }
        14 => {
            let shown: Vec<&str> = panel.in_tab(&crafting, &items).iter().map(|(i, _)| crafting.recipes[*i].makes.as_str()).collect();
            info!("pack: the Materials tab: {}", shown.join(", "));
            shot("6_materials");
            next(&mut state, 0.3);
        }
        15 => {
            keys.press(KeyCode::Escape);
            next(&mut state, 0.3);
        }
        16 => {
            info!("pack: Esc with the panel up: {}", says());
            press(&|b, _, _| b == Some(&PackButton::Settings), Interaction::Pressed);
            next(&mut state, 0.4);
        }
        17 => {
            info!("pack: Settings: {}", says());
            shot("7_menu");
            next(&mut state, 0.3);
        }
        18 => {
            keys.press(KeyCode::Escape);
            next(&mut state, 0.3);
        }
        19 => {
            info!("pack: Esc with the game menu up: {}", says());
            keys.press(KeyCode::Escape);
            next(&mut state, 0.3);
        }
        20 => {
            info!("pack: Esc again: {}", says());
            keys.press(KeyCode::KeyI);
            next(&mut state, 0.3);
        }
        21 => {
            info!("pack: I: {}", says());
            next(&mut state, 0.3);
        }
        _ => {}
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

/// Frames counted, their seconds, the slowest, and those over 12.5 ms
/// (when, how long): the `show` scenario's tally.
#[derive(Default)]
struct FrameTally(u32, f32, f32, Vec<(f32, f32)>);

/// `show` (the real world): the show (`show.rs`) as it's shown. Every 6 s
/// from 3 s "Show the next", and a second after, what's near: each
/// creature's kind, how far off and which side, standing or not, its
/// health. The player can't die (it watches). With `PLATYPUS_SHOW=1` no
/// acts: show mode, and every 5 s what life has come near (within 700).
#[allow(clippy::too_many_arguments)]
fn show_script(
    s: Res<Scenario>,
    mode: Res<crate::show::ShowMode>,
    mut dev: MessageWriter<crate::dev::DevAction>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    near: Query<(&crate::creatures::Creature, &Kinematics, &crate::creatures::Health), Without<LocalPlayer>>,
    mut next: Local<f32>,
    mut look: Local<f32>,
    mut hurt: Local<f32>,
    mut out: Local<bool>,
    sim: Res<SimWorld>,
    (time, mut frames): (Res<Time>, Local<FrameTally>),
) {
    if s.name != "show" {
        return;
    }
    let Ok((mut pk, mut ph)) = player.single_mut() else { return };
    // (Out of the village first, west: open ground, nothing in the way.)
    if !*out && s.elapsed > 1.0 {
        *out = true;
        let x = pk.body.pos.x - 600.0;
        if let Some(y) = sim.generator.surface_hint(x as i32).and_then(|s| crate::creatures::spawn::find_ground(&sim.world, x as i32, s + 80, 300)) {
            pk.body.pos = Vec2::new(x, y as f32 + pk.body.half.y);
            pk.prev_pos = pk.body.pos;
        }
    }
    *hurt += ph.max - ph.hp;
    ph.hp = ph.max;
    let p = pk.body.pos;
    // (`PLATYPUS_NOACTS=1`: no acts, only what's near and the frames, show
    // mode or not.)
    if mode.0 || std::env::var("PLATYPUS_NOACTS").is_ok() {
        frames.0 += 1;
        frames.1 += time.delta_secs();
        frames.2 = frames.2.max(time.delta_secs());
        if time.delta_secs() > 0.0125 {
            frames.3.push((s.elapsed, time.delta_secs() * 1000.0));
        }
        if s.elapsed >= *look + 5.0 {
            let list: Vec<String> = frames.3.iter().map(|(t, ms)| format!("{t:.1}s {ms:.0}")).collect();
            info!("show: spikes over 12.5 ms: {}", list.join(", "));
            *look = s.elapsed;
            let mut kinds: std::collections::BTreeMap<&str, usize> = Default::default();
            for (c, k, _) in &near {
                if k.body.pos.distance(p) < 700.0 {
                    *kinds.entry(c.kind.as_str()).or_default() += 1;
                }
            }
            info!("show: t {:.0} frame {:.2} ms avg, {:.1} worst; {} creatures in all; near: {kinds:?}", s.elapsed, frames.1 * 1000.0 / frames.0.max(1) as f32, frames.2 * 1000.0, near.iter().count());
            *frames = FrameTally::default();
        }
        return;
    }
    if s.elapsed >= 3.0 && s.elapsed >= *next {
        if s.elapsed > 4.0 {
            info!("show: t {:.0} the player took {:.0} since the last act", s.elapsed, *hurt);
        }
        *hurt = 0.0;
        *next = s.elapsed + 6.0;
        *look = s.elapsed + 1.0;
        dev.write(crate::dev::DevAction::ShowNext);
    }
    if *look > 0.0 && s.elapsed >= *look {
        *look = 0.0;
        let mut seen: Vec<String> = near
            .iter()
            .filter(|(_, k, _)| k.body.pos.distance(p) < 500.0)
            .map(|(c, k, h)| format!("{} {:+.0},{:+.0}{} {:.0}/{:.0}", c.kind, k.body.pos.x - p.x, k.body.pos.y - p.y, if k.loco.grounded() { "" } else { " (up)" }, h.hp, h.max))
            .collect();
        seen.sort();
        info!("show: t {:.0} near: {}", s.elapsed, seen.join(", "));
    }
}

/// `plunge` (the real world): the player lifted 500 cells up at 2 s and let
/// fall; every frame from then for 4 s: the frame's length, the fixed ticks
/// run in it, the player as drawn, the camera, and the player on screen
/// (drawn minus camera, in pixels at the zoom): a smooth fall draws the
/// player still on screen and the camera's steps even for even frames.
/// `PLATYPUS_LAMP=1`: at night, the big flashlight aimed ahead and down.
#[allow(clippy::too_many_arguments)]
fn plunge_script(
    s: Res<Scenario>,
    time: Res<Time>,
    fixed: Res<Time<Fixed>>,
    zoom: Res<crate::camera::Zoom>,
    mut player: Query<(&mut Kinematics, &GlobalTransform), With<LocalPlayer>>,
    cam: Query<&Transform, With<crate::camera::MainCamera>>,
    mut state: Local<(bool, f64, f32, f32)>,
    (mut day, mut toggles, mut cursor): (ResMut<crate::light::Daylight>, ResMut<crate::light::LightToggles>, ResMut<CursorOverride>),
) {
    if s.name != "plunge" {
        return;
    }
    let Ok((mut k, tf)) = player.single_mut() else { return };
    // (`PLATYPUS_LAMP=1`: at night, the big flashlight aimed ahead and
    // down as it falls: does its light keep up?)
    if std::env::var("PLATYPUS_LAMP").is_ok() {
        cursor.0 = Some(k.body.pos + Vec2::new(160.0, -80.0));
    }
    if !state.0 && s.elapsed > 2.0 {
        state.0 = true;
        if std::env::var("PLATYPUS_LAMP").is_ok() {
            day.skipped = (day.skipped + 23.0 - day.time * 24.0).rem_euclid(24.0);
            toggles.carry = crate::light::Carry::BigBeam;
        }
        k.body.pos.y += 500.0;
        k.body.vel = Vec2::ZERO;
        k.prev_pos = k.body.pos;
        return;
    }
    if !state.0 || s.elapsed > 6.0 {
        return;
    }
    let Ok(c) = cam.single() else { return };
    let ticks = ((fixed.elapsed_secs_f64() - state.1) * 60.0).round();
    state.1 = fixed.elapsed_secs_f64();
    let (py, cy) = (tf.translation().y, c.translation.y);
    let px = zoom.0;
    info!("plunge: t {:.3} dt {:.2} ms ticks {ticks} vy {:.0} player {py:.2} cam {cy:.2} (step {:.2}) on screen {:.1} px", s.elapsed, time.delta_secs() * 1000.0, k.body.vel.y, cy - state.2, (py - cy) * px);
    state.2 = cy;
}

/// `optics` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): things light
/// passes through, on the floor in a row: a glass prism (a triangle 60
/// wide, 52 high), a column of ruby, emerald and amethyst, a pool of water
/// (120 wide, 36 deep, dug into the floor), a block of ice, a block of
/// phosphor. `PLATYPUS_OPTICS`
/// picks the shot (or `spire`: the real world, by a crystal spire at
/// `PLATYPUS_SPIRE_X`, `PLATYPUS_OPTICS_HOUR`): `prism`, `gems`, `water`, `ice`, `rainbow` (a slit in a
/// stone wall before the prism: a pencil of light) (at night, the big
/// flashlight aimed at it from 130 cells off; `water` low across the
/// pool's surface) or `day` (9 am, the sun low, nothing carried, by the
/// pool). For before/after pictures (`PLATYPUS_SCREENSHOT`: held still,
/// the light aimed); without one, the player's free to walk and aim.
fn optics_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    (mut day, mut toggles, mut cursor): (ResMut<crate::light::Daylight>, ResMut<crate::light::LightToggles>, ResMut<CursorOverride>),
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature), Without<LocalPlayer>>,
    mut state: Local<u8>,
) {
    if s.name != "optics" {
        return;
    }
    let Ok(mut k) = player.single_mut() else { return };
    let floor = platypus_worldgen::arena::FLOOR;
    let fl = floor as f32;
    let shot = std::env::var("PLATYPUS_OPTICS").unwrap_or_else(|_| "prism".into());
    // (`spire`: in the real world, by a crystal spire (`PLATYPUS_SPIRE_X`),
    // the sun low (`PLATYPUS_OPTICS_HOUR`, 7.5 by default).)
    if shot == "spire" {
        if *state == 0 && s.elapsed > 0.5 {
            let x = std::env::var("PLATYPUS_SPIRE_X").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(104_350.0) - 70.0;
            let hour = std::env::var("PLATYPUS_OPTICS_HOUR").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(7.5);
            day.skipped = (day.skipped + hour - day.time * 24.0).rem_euclid(24.0);
            toggles.carry = crate::light::Carry::Nothing;
            if let Some(y) = sim.generator.surface_hint(x as i32) {
                k.body.pos = Vec2::new(x, y as f32 + 60.0);
                k.body.vel = Vec2::ZERO;
                k.prev_pos = k.body.pos;
            }
            info!("optics: by the spire at {:.0}, {hour} h", x + 70.0);
            *state = 9;
        }
        return;
    }
    match *state {
        0 if s.elapsed > 0.5 => {
            let id = |n: &str| sim.materials().id(n);
            let (Some(glass), Some(ruby), Some(emerald), Some(amethyst), Some(water), Some(ice), Some(stone), Some(phosphor)) = (id("glass"), id("ruby"), id("emerald"), id("amethyst"), id("water"), id("ice"), id("stone"), id("phosphor")) else {
                warn!("optics: a material's missing");
                *state = 9;
                return;
            };
            let put = |sim: &mut SimWorld, x: i32, y: i32, m: platypus_sim::MaterialId| sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: m, overwrite: true });
            // The prism: a triangle on the floor.
            for y in 0..52 {
                let half = (30.0 * (1.0 - y as f32 / 52.0)) as i32;
                for x in 930 - half..=930 + half {
                    put(&mut sim, x, floor + y, glass);
                }
            }
            // The gems, side by side.
            for (x0, m) in [(1010, ruby), (1024, emerald), (1038, amethyst)] {
                for x in x0..x0 + 14 {
                    for y in floor..floor + 60 {
                        put(&mut sim, x, y, m);
                    }
                }
            }
            // The pool, dug and filled.
            for x in 1100..1220 {
                for y in floor - 36..floor {
                    put(&mut sim, x, y, water);
                }
            }
            // The ice.
            for x in 1300..1340 {
                for y in floor..floor + 30 {
                    put(&mut sim, x, y, ice);
                }
            }
            // Phosphor, past the ice.
            for x in 1420..1460 {
                for y in floor..floor + 40 {
                    put(&mut sim, x, y, phosphor);
                }
            }
            // (`rainbow`: a stone wall before the prism, a slit in it
            // two texels high: a pencil of light, split into its colours.)
            if shot == "rainbow" {
                for x in 860..872 {
                    for y in floor..floor + 70 {
                        if !(floor + 20..floor + 28).contains(&y) {
                            put(&mut sim, x, y, stone);
                        }
                    }
                }
            }
            for (e, c) in &them {
                if c.kind == "dummy" || c.kind == "sandbag" {
                    commands.entity(e).despawn();
                }
            }
            let night = |day: &mut crate::light::Daylight, h: f32| day.skipped = (day.skipped + h - day.time * 24.0).rem_euclid(24.0);
            let target = match shot.as_str() {
                "gems" => Vec2::new(1031.0, fl + 30.0),
                "water" => Vec2::new(1200.0, fl - 2.0),
                "ice" => Vec2::new(1320.0, fl + 15.0),
                "day" => Vec2::new(1160.0, fl - 18.0),
                "rainbow" => Vec2::new(930.0, fl + 24.0),
                "phosphor" => Vec2::new(1440.0, fl + 20.0),
                _ => Vec2::new(930.0, fl + 18.0),
            };
            if shot == "day" {
                // (Morning: the sun low, so what bends shows.)
                night(&mut day, 9.0);
                toggles.carry = crate::light::Carry::Nothing;
            } else {
                night(&mut day, 23.0);
                toggles.carry = crate::light::Carry::BigBeam;
            }
            let stand = match shot.as_str() {
                "water" => target.x - 150.0,
                "rainbow" => 800.0,
                "ice" => 1250.0,
                "phosphor" => 1360.0,
                _ => target.x - 130.0,
            };
            k.body.pos = Vec2::new(stand, fl + k.body.half.y);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            cursor.0 = Some(target);
            info!("optics: the {shot} shot, the player at x {stand:.0}, aimed at ({:.0}, {:.0})", target.x, target.y - fl);
            *state = 1;
        }
        // (Held still and aimed only for pictures; played, the player
        // walks and aims the light as ever. `phosphor`: the light out at
        // 4 s, to watch it glow on.)
        1 if std::env::var("PLATYPUS_SCREENSHOT").is_ok() || std::env::var("PLATYPUS_SHOTS").is_ok() => {
            k.body.vel.x = 0.0;
            if shot == "phosphor" && s.elapsed > 4.0 && toggles.carry != crate::light::Carry::Nothing {
                toggles.carry = crate::light::Carry::Nothing;
                info!("optics: the light out at {:.1} s", s.elapsed);
            }
        }
        1 => {
            cursor.0 = None;
            *state = 2;
        }
        _ => {}
    }
}

/// `ragdoll` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): an orc, a
/// skeleton, a raptor, a spider and a troll in a row, the player off to
/// the side (it can't die); at 2 s a blast among them. Logs who died and
/// where the bodies came to rest; for pictures of how they fly.
fn ragdoll_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature), Without<LocalPlayer>>,
    bodies: Query<&Kinematics, (With<crate::hands::corpses::Corpse>, Without<LocalPlayer>)>,
    mut state: Local<u8>,

) {
    if s.name != "ragdoll" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    match *state {
        0 if s.elapsed > 0.5 => {
            for (e, c) in &them {
                if c.kind == "dummy" || c.kind == "sandbag" {
                    commands.entity(e).despawn();
                }
            }
            k.body.pos = Vec2::new(960.0, fl + k.body.half.y);
            k.prev_pos = k.body.pos;
            for (i, kind) in ["orc", "skeleton", "raptor", "spider", "troll"].into_iter().enumerate() {
                crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(1040.0 + i as f32 * 34.0, fl), |_| {});
            }
            *state = 1;
        }
        1 if s.elapsed > 2.0 => {
            sim.queue(WorldEdit::Explode { center: CellPos::new(1100, platypus_worldgen::arena::FLOOR + 4), radius: 30, power: 150 });
            info!("ragdoll: the blast");
            *state = 2;
        }
        2 if s.elapsed > 5.5 => {
            let alive: Vec<&str> = them.iter().filter(|(_, c)| c.kind != "dummy" && c.kind != "sandbag").map(|(_, c)| c.kind.as_str()).collect();
            let rest: Vec<String> = bodies.iter().map(|b| format!("{:.0}", b.body.pos.x)).collect();
            info!("ragdoll: alive {alive:?}; bodies at x {}", rest.join(", "));
            *state = 3;
        }
        _ => {}
    }
}

/// `slope` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a steep stone ramp
/// (rising 1.8 a cell), an iron strider set on it, a skeleton and an orc
/// on it struck dead where they stand at 1.6 s. Logs, every second, how far
/// the strider's feet are off the ramp, and where the bodies are (do they
/// lie along it and slide down?).
fn slope_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    bodies: Query<&Kinematics, (With<crate::hands::corpses::Corpse>, Without<LocalPlayer>)>,
    mut state: Local<(u8, f32)>,
) {
    if s.name != "slope" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let floor = platypus_worldgen::arena::FLOOR;
    let fl = floor as f32;
    let ramp = |x: f32| fl + ((x - 1060.0) * 1.8).clamp(0.0, 200.0);
    match state.0 {
        0 if s.elapsed > 0.5 => {
            let Some(stone) = sim.materials().id("stone") else { return };
            for x in 1060..1180 {
                for y in floor..ramp(x as f32) as i32 {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: stone, overwrite: true });
                }
            }
            for (e, c, _) in &them {
                if c.kind == "dummy" || c.kind == "sandbag" {
                    commands.entity(e).despawn();
                }
            }
            k.body.pos = Vec2::new(1000.0, fl + k.body.half.y);
            k.prev_pos = k.body.pos;
            *state = (1, s.elapsed);
        }
        1 if s.elapsed > 1.0 => {
            for (kind, x) in [("strider", 1120.0), ("skeleton", 1095.0), ("orc", 1150.0)] {
                crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(x, ramp(x) + 2.0), |_| {});
            }
            *state = (2, s.elapsed);
        }
        2 if s.elapsed > 1.6 => {
            // (Struck dead where they stand, on the ramp.)
            for (e, c, _) in &them {
                if c.kind == "skeleton" || c.kind == "orc" {
                    commands.entity(e).queue(|mut ew: EntityWorldMut| {
                        if let Some(mut h) = ew.get_mut::<crate::creatures::Health>() {
                            h.hp = 0.0;
                        }
                    });
                }
            }
            *state = (3, s.elapsed);
        }
        3 if s.elapsed >= state.1 + 1.0 => {
            state.1 = s.elapsed;
            for (_, c, kk) in &them {
                if c.kind == "strider" {
                    let feet = kk.body.pos.y - kk.body.half.y;
                    info!("slope: t {:.0} the strider at x {:.0}: feet {:.0} over the ramp", s.elapsed, kk.body.pos.x, feet - ramp(kk.body.pos.x));
                }
            }
            let rest: Vec<String> = bodies.iter().map(|b| format!("x {:.0} ({:.0} over the ramp, moving {:.0})", b.body.pos.x, b.body.pos.y - b.body.half.y - ramp(b.body.pos.x), b.body.vel.length())).collect();
            info!("slope: t {:.0} bodies: {}", s.elapsed, rest.join(", "));
        }
        _ => {}
    }
}

/// `cursor` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): the cursor in
/// each of its looks, a second each (pictures with `PLATYPUS_SHOTS`): the
/// pickaxe over dirt (1 s), over obsidian, too hard (2 s); a bomb, its arc
/// (3 s); over an orc's body, a hand (4 s); a fire wand just cast, its ring
/// (5 s); crouched, the eye (6 s).
#[allow(clippy::too_many_arguments)]
fn cursor_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut over: ResMut<crate::camera::CursorOverride>,
    mut hand: ResMut<crate::hands::Hand>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    bodies: Query<&Kinematics, (With<crate::hands::corpses::Corpse>, Without<LocalPlayer>)>,
    mut state: Local<u8>,
) {
    use platypus_sim::BLOCK;
    if s.name != "cursor" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let floor = platypus_worldgen::arena::FLOOR;
    let fl = floor as f32;
    let t = s.elapsed;
    // (A block of a material, `bx` blocks along, on the floor.)
    let block = |bx: i32| Vec2::new((bx as f32 + 0.5) * BLOCK as f32, fl + BLOCK as f32 * 0.5);
    let bx0 = 912 / BLOCK;
    if *state == 0 && t > 0.5 {
        for (e, c, _) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        k.body.pos = Vec2::new(900.0, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        for (bx, m) in [(bx0, "dirt"), (bx0 + 2, "obsidian")] {
            let Some(m) = sim.materials().id(m) else { continue };
            for y in floor..floor + BLOCK {
                for x in bx * BLOCK..(bx + 1) * BLOCK {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: m, overwrite: true });
                }
            }
        }
        crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(860.0, fl), |e| {
            e.remove::<crate::creatures::brain::hunter::Hunter>();
        });
        *state = 1;
    }
    if *state == 1 && t > 0.8 {
        for (e, c, _) in &them {
            if c.kind == "orc" {
                commands.entity(e).queue(|mut ew: EntityWorldMut| {
                    if let Some(mut h) = ew.get_mut::<crate::creatures::Health>() {
                        h.hp = 0.0;
                    }
                });
            }
        }
        *state = 2;
    }
    let body = bodies.iter().next().map(|b| b.body.pos);
    let (slot, at) = match t {
        t if t < 2.0 => (0, block(bx0)),
        t if t < 3.0 => (0, block(bx0 + 2)),
        t if t < 4.0 => (3, Vec2::new(880.0 - (t - 3.0) * 60.0, fl + 40.0)),
        t if t < 5.0 => (0, body.unwrap_or(Vec2::new(860.0, fl + 4.0))),
        _ => (6, Vec2::new(1010.0, fl + 20.0)),
    };
    hand.slot = slot;
    hand.bar = 0;
    over.0 = Some(at);
    if (5.0..5.1).contains(&t) {
        mouse.press(MouseButton::Left);
    } else {
        mouse.release(MouseButton::Left);
    }
    if t > 5.8 {
        keys.press(KeyCode::ControlLeft);
    }
    if *state == 2 && t > 1.5 {
        info!("cursor: pickaxe on dirt (1 s), obsidian (2 s), bomb (3 s), body (4 s), wand (5 s), crouched (6 s)");
        *state = 3;
    }
}

/// `filter` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): the pick-up
/// filter. Valuables chosen (V) at 0.5 s; dirt, copper ore, a torch, a
/// common helm and a rare one dropped on the player at 1 s: the dirt and
/// the common helm stay on the ground (faded). At 3 s the cursor over the
/// dirt (a hand), right-clicked: it's taken. At 4.5 s the pack opened (the
/// filter row under it). Logs what's in the pack and what's on the ground.
#[allow(clippy::too_many_arguments)]
fn filter_script(
    mut commands: Commands,
    s: Res<Scenario>,
    items: Option<Res<crate::hands::items::Items>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut over: ResMut<crate::camera::CursorOverride>,
    mut open: ResMut<crate::hands::InventoryOpen>,
    player: Query<(&Kinematics, &crate::hands::items::Inventory, Option<&crate::hands::filter::PickupFilter>), With<LocalPlayer>>,
    drops: Query<(&crate::hands::Dropped, &Kinematics)>,
    mut state: Local<u8>,
) {
    use crate::hands::items::{Roll, Stack};
    if s.name != "filter" {
        return;
    }
    let (Some(items), Ok((k, inv, f))) = (items, player.single()) else { return };
    let t = s.elapsed;
    let count = |id: &str| items.id(id).map_or(0, |i| inv.count(i));
    match *state {
        0 if t > 0.5 => {
            keys.press(KeyCode::KeyV);
            *state = 1;
        }
        1 => {
            keys.release(KeyCode::KeyV);
            info!("filter: chosen {:?}", f);
            *state = 2;
        }
        2 if t > 1.0 => {
            let at = k.body.pos + Vec2::new(18.0, 10.0);
            for (id, n, roll) in [("block:dirt", 4, Roll::default()), ("block:copper_ore", 3, Roll::default()), ("torch", 5, Roll::default()), ("iron_helm", 1, Roll { rarity: 0, level: 5, seed: 1 }), ("iron_helm", 1, Roll { rarity: 2, level: 5, seed: 2 })] {
                if let Some(i) = items.id(id) {
                    crate::hands::spawn_drop(&mut commands, &items, at, Stack { roll, ..Stack::new(i, n * items.unit(i)) });
                }
            }
            *state = 3;
        }
        3 if t > 2.6 => {
            let left: Vec<String> = drops.iter().map(|(d, _)| items.def(d.stack.item).name.clone()).collect();
            info!("filter: pack dirt {} copper {} torch {}; left on the ground {:?}", count("block:dirt"), count("block:copper_ore"), count("torch"), left);
            *state = 4;
        }
        4 if t > 3.0 => {
            let dirt = drops.iter().find(|(d, _)| items.def(d.stack.item).id == "block:dirt").map(|(_, dk)| dk.body.pos);
            over.0 = dirt;
            if t > 3.3 {
                mouse.press(MouseButton::Right);
                *state = 5;
            }
        }
        5 => {
            mouse.release(MouseButton::Right);
            *state = 6;
        }
        6 if t > 4.4 => {
            let left: Vec<String> = drops.iter().map(|(d, _)| items.def(d.stack.item).name.clone()).collect();
            info!("filter: after the right-click: pack dirt {}; left on the ground {:?}", count("block:dirt"), left);
            open.0 = true;
            *state = 7;
        }
        _ => {}
    }
}

/// `pickups` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): things dropped
/// on the player to see "Picked up" over its head: copper ore in two lots
/// (one line, counting up), dirt blocks and a bite of a block, a torch, a
/// rare helm, then a burst of gold. Logs the pack's copper and the gold.
fn pickups_script(
    mut commands: Commands,
    s: Res<Scenario>,
    items: Option<Res<crate::hands::items::Items>>,
    mut gold_queue: ResMut<crate::gold::GoldQueue>,
    player: Query<(&Kinematics, &crate::hands::items::Inventory, &crate::gold::Gold), With<LocalPlayer>>,
    mut state: Local<u8>,
) {
    use crate::hands::items::{BLOCK_CELLS, Roll, Stack};
    if s.name != "pickups" {
        return;
    }
    let (Some(items), Ok((k, inv, gold))) = (items, player.single()) else { return };
    let at = k.body.pos + Vec2::new(0.0, 14.0);
    let mut drop = |id: &str, n: u32, roll: Roll| {
        if let Some(i) = items.id(id) {
            crate::hands::spawn_drop(&mut commands, &items, at, Stack { roll, ..Stack::new(i, n * items.unit(i)) });
        }
    };
    match *state {
        0 if s.elapsed > 1.0 => {
            for _ in 0..3 {
                drop("block:copper_ore", 1, Roll::default());
            }
            *state = 1;
        }
        1 if s.elapsed > 1.6 => {
            for _ in 0..4 {
                drop("block:copper_ore", 2, Roll::default());
            }
            drop("block:dirt", 3, Roll::default());
            if let Some(dirt) = items.id("block:dirt") {
                crate::hands::spawn_drop(&mut commands, &items, at, Stack::new(dirt, BLOCK_CELLS / 2));
            }
            *state = 2;
        }
        2 if s.elapsed > 2.2 => {
            drop("torch", 5, Roll::default());
            drop("iron_helm", 1, Roll { rarity: 3, level: 12, seed: 4242 });
            gold_queue.burst.push((at + Vec2::new(20.0, 0.0), 60));
            *state = 3;
        }
        3 if s.elapsed > 3.4 => {
            let copper = items.id("block:copper_ore").map_or(0, |i| inv.count(i));
            info!("pickups: copper ore in the pack {} cells ({} blocks), gold {}", copper, copper / BLOCK_CELLS, gold.0);
            *state = 4;
        }
        _ => {}
    }
}

/// `marks` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): wounds' marks. An
/// orc (flesh) and a skeleton (bone), their armour off, struck every 0.3 s
/// somewhere on them with each kind of hurt in turn (slash, pierce, blunt,
/// fire, acid, frost; light blows), the orc killed at 6 s (its body keeps
/// them). Logs how many marks each has.
#[allow(clippy::type_complexity)]
fn marks_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::body::marks::Marks>), Without<LocalPlayer>>,
    mut eqs: Query<&mut crate::gear::Equipment, Without<LocalPlayer>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, f32, u32)>,
) {
    use crate::creatures::Harm;
    if s.name != "marks" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    if (1.2..1.4).contains(&s.elapsed) {
        for mut eq in &mut eqs {
            eq.worn = Default::default();
        }
    }
    match state.0 {
        0 if s.elapsed > 0.5 => {
            for (e, c, ..) in &them {
                if c.kind == "dummy" || c.kind == "sandbag" {
                    commands.entity(e).despawn();
                }
            }
            k.body.pos = Vec2::new(880.0, fl + k.body.half.y);
            k.prev_pos = k.body.pos;
            for (kind, x) in [("orc", 940.0), ("skeleton", 975.0)] {
                crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(x, fl), |e| {
                    e.remove::<crate::creatures::brain::hunter::Hunter>();
                });
            }
            state.0 = 1;
            state.1 = 1.8;
        }
        1 if s.elapsed > state.1 => {
            state.1 = s.elapsed + 0.3;
            state.2 += 1;
            let n = state.2;
            let harm = [Harm::Slash, Harm::Pierce, Harm::Blunt, Harm::Fire, Harm::Acid, Harm::Frost][(n as usize / 2) % 6];
            for (e, c, tk, _) in &them {
                if c.kind != "orc" && c.kind != "skeleton" {
                    continue;
                }
                // (Somewhere up and down it, a little either side.)
                let h = platypus_sim::rng::hash(&[n as u64, e.to_bits()]);
                let up = (h % 100) as f32 / 100.0;
                let side = ((h / 100) % 100) as f32 / 100.0 - 0.5;
                let at = Vec2::new(tk.body.pos.x + side * tk.body.half.x * 1.6, tk.body.pos.y - tk.body.half.y * 0.2 + up * tk.body.half.y * 1.5);
                hits.write(crate::combat::Hit { by: None, target: e, damage: 6.0, harm, knock: Vec2::ZERO, stun: 0.0, at, dir: Vec2::new(1.0, -0.2), weight: 0.5, crit: false });
            }
            if s.elapsed > 6.0 {
                for (e, c, tk, _) in &them {
                    if c.kind == "orc" {
                        hits.write(crate::combat::Hit { by: None, target: e, damage: 999.0, harm: Harm::Slash, knock: Vec2::new(40.0, 20.0), stun: 0.3, at: tk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
                    }
                }
                state.0 = 2;
            }
        }
        _ => {}
    }
    if state.0 == 1 && state.2.is_multiple_of(5) && state.2 > 0 {
        let counts: Vec<String> = them.iter().filter(|t| t.1.kind == "orc" || t.1.kind == "skeleton").map(|t| format!("{} {}", t.1.kind, t.3.map_or(0, |m| m.list.len()))).collect();
        info!("marks: {}", counts.join(", "));
    }
}

/// `knock` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): how far a blow
/// throws a creature (`PLATYPUS_KIND`, a skeleton): at 2 s a longsword's
/// cleave (24 slash, 375 knock, a little up), at 4 s another; logs how far
/// it went from where it stood each time, and how high.
fn knock_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, Vec2, f32, f32)>,
) {
    if s.name != "knock" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let kind = std::env::var("PLATYPUS_KIND").unwrap_or_else(|_| "skeleton".into());
    let it = them.iter().find(|t| t.1.kind == kind);
    match state.0 {
        0 if s.elapsed > 0.5 => {
            for (e, c, _) in &them {
                if c.kind == "dummy" || c.kind == "sandbag" {
                    commands.entity(e).despawn();
                }
            }
            k.body.pos = Vec2::new(860.0, fl + k.body.half.y);
            k.prev_pos = k.body.pos;
            crate::creatures::def::spawn_creature(&mut commands, &kind, Vec2::new(900.0, fl), |e| {
                e.remove::<crate::creatures::brain::hunter::Hunter>();
            });
            state.0 = 1;
        }
        1 | 3 if s.elapsed > if state.0 == 1 { 2.0 } else { 4.0 } => {
            if let Some((e, _, tk)) = it {
                let push = Vec2::new(1.0, 0.45).normalize() * 375.0;
                hits.write(crate::combat::Hit { by: None, target: e, damage: 24.0, harm: crate::creatures::Harm::Slash, knock: push, stun: 0.3, at: tk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
                state.1 = tk.body.pos;
                state.2 = 0.0;
                state.3 = 0.0;
            }
            state.0 += 1;
        }
        2 | 4 => {
            if let Some((_, _, tk)) = it {
                state.2 = state.2.max((tk.body.pos.x - state.1.x).abs());
                state.3 = state.3.max(tk.body.pos.y - state.1.y);
            }
            if s.elapsed > if state.0 == 2 { 3.8 } else { 5.8 } {
                info!("knock: the {kind} went {:.0} cells, {:.0} up", state.2, state.3);
                state.0 += 1;
            }
        }
        _ => {}
    }
}

/// `skull` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a skeleton's skull
/// is what holds it together. Its head off at 2 s (it fights on blind);
/// then `PLATYPUS_SKULL=near` (the default): killed at 3 s, it gets up again
/// whole, its skull flying back with its bones; `far`: its skull thrown 120
/// cells off first, it gets up headless and goes to it, and puts it back on;
/// `crush`: a hammer on its skull at 3 s, it falls for good. Logs each half
/// second what it's lost, whether it sees, and the skulls and heaps there.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn skull_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, &crate::creatures::Health, Option<&crate::creatures::body::parts::Severed>, Has<crate::creatures::body::wounds::Blind>, Has<crate::creatures::body::rise::Seeking>), Without<LocalPlayer>>,
    mut skulls: Query<(Entity, &mut crate::creatures::body::ragdoll::Limp, &crate::creatures::Health), (With<crate::creatures::body::rise::Skull>, Without<LocalPlayer>)>,
    piles: Query<(), With<crate::creatures::body::rise::Pile>>,
    mut eqs: Query<&mut crate::gear::Equipment, Without<LocalPlayer>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, f32)>,
) {
    if s.name != "skull" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let case = std::env::var("PLATYPUS_SKULL").unwrap_or_else(|_| "near".into());
    if (1.2..1.4).contains(&s.elapsed) {
        for mut eq in &mut eqs {
            eq.worn = Default::default();
        }
    }
    let skeleton = them.iter().find(|t| t.1.kind == "skeleton");
    match state.0 {
        0 if s.elapsed > 0.5 => {
            for (e, c, ..) in &them {
                if c.kind == "dummy" || c.kind == "sandbag" {
                    commands.entity(e).despawn();
                }
            }
            k.body.pos = Vec2::new(880.0, fl + k.body.half.y);
            k.prev_pos = k.body.pos;
            crate::creatures::def::spawn_creature(&mut commands, "skeleton", Vec2::new(960.0, fl), |e| {
                e.remove::<crate::creatures::brain::hunter::Hunter>();
            });
            state.0 = 1;
        }
        1 if s.elapsed > 2.0 => {
            if let Some((e, _, tk, ..)) = skeleton {
                let at = Vec2::new(tk.body.pos.x, tk.body.pos.y - tk.body.half.y + 34.0);
                hits.write(crate::combat::Hit { by: None, target: e, damage: 42.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at, dir: Vec2::new(1.0, 0.3), weight: 1.0, crit: false });
                info!("skull: t {:.1} its head off", s.elapsed);
            }
            state.0 = 2;
        }
        2 if s.elapsed > 2.6 && case == "far" => {
            for (_, mut limp, _) in &mut skulls {
                limp.shift(Vec2::new(120.0, 0.0));
                info!("skull: t {:.1} its skull thrown off", s.elapsed);
            }
            state.0 = 3;
        }
        2 | 3 if s.elapsed > 3.0 => {
            if case == "crush" {
                for (e, limp, _) in &skulls {
                    let at = limp.pieces[0].pos;
                    hits.write(crate::combat::Hit { by: None, target: e, damage: 30.0, harm: crate::creatures::Harm::Blunt, knock: Vec2::ZERO, stun: 0.0, at, dir: Vec2::NEG_Y, weight: 1.0, crit: false });
                    info!("skull: t {:.1} a hammer on its skull", s.elapsed);
                }
            } else if let Some((e, _, tk, ..)) = skeleton {
                hits.write(crate::combat::Hit { by: None, target: e, damage: 999.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at: tk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
                info!("skull: t {:.1} a sword kills it", s.elapsed);
            }
            state.0 = 4;
        }
        _ => {}
    }
    if state.0 >= 1 && s.elapsed >= state.1 {
        state.1 = s.elapsed + 0.5;
        let sk = skeleton.map(|(_, _, tk, th, sev, blind, seek)| format!("skeleton hp {:.0} at x {:.0} lost {:?}{}{}", th.hp, tk.body.pos.x, sev.map(|s| s.0.clone()).unwrap_or_default(), if blind { " blind" } else { "" }, if seek { " seeking" } else { "" })).unwrap_or("no skeleton".into());
        let sks: Vec<String> = skulls.iter().map(|(_, l, h)| format!("at x {:.0} hp {:.0}", l.pieces[0].pos.x, h.hp)).collect();
        info!("skull: t {:.1} {sk}; skulls {sks:?}; heaps {}", s.elapsed, piles.iter().count());
    }
}

/// `sever` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): limbs cut off.
/// `PLATYPUS_SEVER=spider` (the default): a stone roof 50 cells up, a cave
/// spider on its underside; from 2 s an arrow into one of its legs every
/// half second (each enough to cut it): it should hold on with five of its
/// eight, and fall with three. `PLATYPUS_SEVER=skeleton`: a skeleton by
/// the player (its armour off), a blade through its sword arm at 2 s (the
/// sword falls), its head at 3.5 s (it fights on blind), a leg at 5 s
/// (dead: rubble without what it had lost). Logs, each half second, what
/// it's lost, whether it holds on, where it is and how fast it can go.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sever_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Option<&crate::creatures::body::parts::Parts>, Option<&crate::creatures::body::parts::Severed>, &crate::creatures::MoveStats), Without<LocalPlayer>>,
    mut controls: Query<&mut crate::creatures::Controls, Without<LocalPlayer>>,
    mut eqs: Query<&mut crate::gear::Equipment, Without<LocalPlayer>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, f32, f32)>,
) {
    use crate::creatures::body::parts::Limb;
    if s.name != "sever" {
        return;
    }
    // (No armour on it: every blow the same, run to run.)
    if (1.2..1.4).contains(&s.elapsed) {
        for mut eq in &mut eqs {
            eq.worn = Default::default();
        }
    }
    // (The spider wants up: it holds on to the roof while it can.)
    for mut c in &mut controls {
        c.0.move_y = 1.0;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let floor = platypus_worldgen::arena::FLOOR;
    let fl = floor as f32;
    let spider = std::env::var("PLATYPUS_SEVER").map_or(true, |v| v != "skeleton");
    let kind = if spider { "spider" } else { "skeleton" };
    match state.0 {
        0 if s.elapsed > 0.5 => {
            for (e, c, ..) in &them {
                if c.kind == "dummy" || c.kind == "sandbag" {
                    commands.entity(e).despawn();
                }
            }
            if spider {
                let Some(stone) = sim.materials().id("stone") else { return };
                for x in 920..1120 {
                    for y in floor + 50..floor + 62 {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: stone, overwrite: true });
                    }
                }
            }
            k.body.pos = Vec2::new(if spider { 880.0 } else { 900.0 }, fl + k.body.half.y);
            k.prev_pos = k.body.pos;
            state.0 = 1;
        }
        1 if s.elapsed > 0.8 => {
            // (The spider against the roof's underside.)
            let at = if spider { Vec2::new(1020.0, fl + 50.0 - 18.0) } else { Vec2::new(960.0, fl) };
            crate::creatures::def::spawn_creature(&mut commands, kind, at, move |e| {
                e.remove::<crate::creatures::brain::hunter::Hunter>();
                // (Up into the roof: it catches hold.)
                if spider && let Some(mut k) = e.get_mut::<Kinematics>() {
                    k.body.vel.y = 90.0;
                }
            });
            state.0 = 2;
            state.1 = 2.0;
        }
        2 if !spider && s.elapsed > state.1 => {
            // A skeleton: its sword arm (2 s), its head (3.5 s), a leg (5 s),
            // each struck through where it's drawn.
            let Some((e, _, tk, parts, _, _)) = them.iter().find(|t| t.1.kind == kind) else { return };
            let f = tk.loco.facing;
            let feet = tk.body.pos.y - tk.body.half.y;
            let leg = parts.and_then(|p| p.list.iter().find(|p| matches!(p.limb, Some(Limb::Leg(_))))).map(|p| (p.a + p.b) / 2.0);
            let (at, what) = match s.elapsed {
                t if t < 3.0 => (Vec2::new(tk.body.pos.x + 3.0 * f, feet + 22.0), "its sword arm"),
                t if t < 4.5 => (Vec2::new(tk.body.pos.x, feet + 34.0), "its head"),
                _ => (leg.unwrap_or(Vec2::new(tk.body.pos.x, feet + 9.0)), "a leg"),
            };
            let damage = 42.0;
            hits.write(crate::combat::Hit { by: None, target: e, damage, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at, dir: Vec2::new(f, 0.3), weight: 1.0, crit: false });
            info!("sever: t {:.1} a blade through {what}", s.elapsed);
            state.1 = s.elapsed + 1.5;
            if s.elapsed > 4.5 {
                state.0 = 3;
            }
        }
        2 if s.elapsed > state.1 => {
            // The next leg it still has: struck through where it is.
            let Some((e, _, tk, Some(parts), cut, _)) = them.iter().find(|t| t.1.kind == kind) else { return };
            let next = parts.list.iter().find(|p| matches!(p.limb, Some(Limb::Leg(_))) && p.limb.is_none_or(|l| !cut.is_some_and(|c| c.has(l))));
            if let Some(p) = next {
                let at = (p.a + p.b) / 2.0;
                let harm = if spider { crate::creatures::Harm::Pierce } else { crate::creatures::Harm::Slash };
                hits.write(crate::combat::Hit { by: None, target: e, damage: if spider { 40.0 } else { 60.0 }, harm, knock: Vec2::ZERO, stun: 0.0, at, dir: (at - tk.body.pos).normalize_or(Vec2::X), weight: 1.0, crit: false });
                info!("sever: t {:.1} a blow through its leg {:?}", s.elapsed, p.limb);
            }
            state.1 = s.elapsed + if spider { 0.5 } else { 3.0 };
            if !spider && s.elapsed > 4.5 {
                state.0 = 3;
            }
        }
        _ => {}
    }
    if state.0 >= 2 && s.elapsed >= state.2 {
        state.2 = s.elapsed + 0.5;
        if let Some((_, _, tk, _, cut, moves)) = them.iter().find(|t| t.1.kind == kind) {
            let lost = cut.map_or(0, |c| c.0.len());
            info!("sever: t {:.1} {kind} lost {:?}: {} at ({:.0}, {:.0}) speed {:.0} cling {}", s.elapsed, cut.map(|c| c.0.clone()).unwrap_or_default(), if tk.loco.clinging().is_some() { "holding on" } else { "not holding" }, tk.body.pos.x, tk.body.pos.y - fl, moves.0.run_speed, moves.0.cling);
            let _ = lost;
        }
    }
}

/// `slime` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a green slime
/// hops at the player (who can't die). `PLATYPUS_SLIME`:
///   look (default): struck now and then (2 s, 4 s, 6 s): it lets go of
///     goo it can't hold.
///   burn: a spark of fire on its goo at 3 s.
///   kill: struck till it dies (from 2 s): its goo spills.
///   heal: two longsword blows (2 s, 2.6 s), then let be: it heals,
///     drawing its spilled goo back in.
///   split: a blade through it (3.5 s, 5 s): it loses goo, and no slime
///     is cut off it (slimes don't split).
///   merge: two half slimes side by side: they merge into one.
///   eat: a rabbit, two torches and copper ore put in its goo (1.5 s).
///   squeeze: a slab with a gap 10 high under it, between it and the
///     player: it squeezes through.
///   glut (with PLATYPUS_SLIME_KIND=glutton): struck with fire (4 s),
///     frost (6.5 s: the two go off), lightning (9 s).
///   dash (any kind, e.g. spider): the player runs at it and dashes
///     through it (2.5 s, back at 4.5 s): its touch should land.
///   wand (glutton): the player casts `PLATYPUS_GLUT_SPELL` (frost_bolt;
///     a list, `a,b`, in turn) at it every 1.5 s.
///   (PLATYPUS_GLUT_POOL=water|oil: a pit of it where it starts.)
///   attack (glutton): struck with `PLATYPUS_GLUT_HARM` (fire, frost,
///     acid, poison, radiant, void, storm: the default) at 2 s, then let be: it attacks with it till
///     it's spent.
///   (anything else, `none`: let be: it hops at the player, spikes them.)
/// Logs its health, the goo it holds and the goo loose near it.
#[allow(clippy::type_complexity)]
fn slime_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health, Option<&crate::creatures::body::elements::Coated>), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, &crate::creatures::Health, Option<&crate::creatures::body::goo::Goo>, Option<&crate::creatures::brain::senses::Alert>), Without<LocalPlayer>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, f32, f32)>,
    mut took: Local<f32>,
    mut felt: MessageReader<crate::combat::Felt>,
    gluttons: Query<(&crate::creatures::body::goo::Glutton, &crate::creatures::body::goo::Goo)>,
    (items, swallowed, mut keys): (Option<Res<crate::hands::items::Items>>, Query<(Entity, Has<crate::hands::Dropped>, Option<&crate::creatures::Creature>), With<crate::creatures::body::goo::Swallowed>>, ResMut<ButtonInput<KeyCode>>),
    drops: Query<&crate::hands::Dropped>,
    mut drive: Query<&mut crate::creatures::Controls, Without<LocalPlayer>>,
    (book, mut casts, player_e): (Res<crate::magic::Spellbook>, MessageWriter<crate::magic::CastRequest>, Query<Entity, With<LocalPlayer>>),
) {
    if s.name != "slime" {
        return;
    }
    let mode = std::env::var("PLATYPUS_SLIME").unwrap_or_else(|_| "look".into());
    // (PLATYPUS_SLIME_KIND: which slime: slime, blue_slime, acid_slime, glutton.)
    let kind = std::env::var("PLATYPUS_SLIME_KIND").unwrap_or_else(|_| "slime".into());
    let Ok((mut k, mut h, coated)) = player.single_mut() else { return };
    let slimy = coated.map_or(0.0, |c| c.share("slimy"));
    if h.max - h.hp > 15.0 {
        info!("slime: t {:.2} the player lost {:.0} at once (hp {:.1}, max {:.1})", s.elapsed, h.max - h.hp, h.hp, h.max);
    }
    *took += h.max - h.hp;
    h.hp = h.max;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let t = s.elapsed;
    for f in felt.read() {
        if f.dealt > 15.0 {
            info!("slime: t {t:.2} a blow of {:.0} ({:?}) on {:?}", f.dealt, f.harm, f.target);
        }
    }
    let slime = them.iter().find(|x| x.1.kind == kind);
    if state.0 == 0 && t > 0.5 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        k.body.pos = Vec2::new(900.0, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        // (PLATYPUS_SLIME_DIRT: earth under it, to see acid eat it.)
        if std::env::var("PLATYPUS_SLIME_DIRT").is_ok()
            && let Some(dirt) = sim.materials().id("dirt")
        {
            let floor = platypus_worldgen::arena::FLOOR;
            for x in 930..1060 {
                for y in floor - 30..floor {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: dirt, overwrite: true });
                }
            }
        }
        // (PLATYPUS_GLUT_POOL=water|oil: a pit of it where it starts.)
        if let Ok(pool) = std::env::var("PLATYPUS_GLUT_POOL")
            && let Some(m) = sim.materials().id(&pool)
        {
            let floor = platypus_worldgen::arena::FLOOR;
            for x in 940..1060 {
                for y in floor - 22..floor + 2 {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: m, overwrite: true });
                }
            }
        }
        if mode == "squeeze" {
            // (A slab with a gap 10 high under it, between it and the
            // player: it squeezes through.)
            if let Some(stone) = sim.materials().id("stone") {
                let floor = platypus_worldgen::arena::FLOOR;
                for x in 880..950 {
                    for y in floor + 10..floor + 60 {
                        sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: stone, overwrite: true });
                    }
                }
            }
            k.body.pos = Vec2::new(850.0, fl + k.body.half.y);
            k.prev_pos = k.body.pos;
        }
        if mode == "many" {
            // (Ten, spread out: what holding their goo costs.)
            for n in 0..10 {
                crate::creatures::def::spawn_creature(&mut commands, &kind, Vec2::new(700.0 + n as f32 * 90.0, fl), |_| {});
            }
        } else if mode == "merge" {
            // (Two halves, side by side, hopping at the player.)
            for x in [960.0, 1000.0] {
                crate::creatures::def::spawn_creature(&mut commands, &kind, Vec2::new(x, fl), |c| {
                    c.insert(crate::creatures::body::goo::Mass(0.5));
                    if let Some(mut h) = c.get_mut::<crate::creatures::Health>() {
                        h.max = 45.0;
                        h.hp = 45.0;
                    }
                });
            }
        } else {
            let driven = mode == "squeeze";
            crate::creatures::def::spawn_creature(&mut commands, &kind, Vec2::new(980.0, fl), move |c| {
                // (Squeezing: no mind of its own, driven at the gap.)
                if driven {
                    c.remove::<crate::creatures::brain::hunter::Hunter>();
                }
            });
        }
        info!("slime: a {kind} hops at the player ({mode})");
        state.0 = 1;
        state.1 = 2.0;
        return;
    }
    let Some((e, _, sk, sh, goo, alert)) = slime else {
        if state.0 >= 1 && t >= state.2 {
            state.2 = t + 0.5;
            info!("slime: t {t:.1} gone; goo loose near: {}", loose(&sim, Vec2::new(950.0, fl), "green_goo"));
        }
        return;
    };
    let strike = |hits: &mut MessageWriter<crate::combat::Hit>, harm: crate::creatures::Harm, damage: f32| {
        hits.write(crate::combat::Hit { by: None, target: e, damage, harm, knock: Vec2::new(40.0, 30.0), stun: 0.2, at: sk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
    };
    match mode.as_str() {
        "burn" if state.0 == 1 && t > 3.0 => {
            // (Flames on its goo, as a torch's or burning grass's.)
            if let Some(fire) = sim.materials().id("fire") {
                sim.queue(WorldEdit::Paint { center: CellPos::from_world(sk.body.pos.x, sk.body.pos.y + 7.0), radius: 2, material: fire, overwrite: false });
            }
            info!("slime: t {t:.1} flames on its goo");
            state.0 = 2;
        }
        "kill" if state.0 >= 1 && t > state.1 => {
            strike(&mut hits, crate::creatures::Harm::Slash, 8.0);
            state.1 = t + 0.7;
        }
        "eat" if state.0 == 1 && t > 1.5 => {
            // (A rabbit and some things, in its goo.)
            crate::creatures::def::spawn_creature(&mut commands, "rabbit", sk.body.pos + Vec2::new(4.0, 0.0), |_| {});
            if let Some(items) = items.as_ref() {
                for (name, dx) in [("torch", -6.0), ("copper_ore", 6.0), ("torch", 0.0)] {
                    if let Some(id) = items.id(name) {
                        crate::hands::spawn_drop(&mut commands, items, sk.body.pos + Vec2::new(dx, 2.0), crate::hands::items::Stack::new(id, 1));
                    }
                }
            }
            info!("slime: t {t:.1} a rabbit, two torches and copper ore in its goo");
            state.0 = 2;
        }
        "glut" if state.0 >= 1 && state.0 <= 3 && t > 1.5 + state.0 as f32 * 2.5 => {
            let harm = [crate::creatures::Harm::Fire, crate::creatures::Harm::Frost, crate::creatures::Harm::Storm][state.0 as usize - 1];
            strike(&mut hits, harm, 6.0);
            info!("slime: t {t:.1} struck with {harm:?}");
            state.0 += 1;
        }
        // `wand`: the player casts `PLATYPUS_GLUT_SPELL` at it every 1.5 s.
        "wand" if state.0 >= 1 && t > 2.0 + (state.0 - 1) as f32 * 1.5 && state.0 <= 5 => {
            // (A list, cast in turn: `spark_bolt,blink_bolt`.)
            let ids = std::env::var("PLATYPUS_GLUT_SPELL").unwrap_or_else(|_| "frost_bolt".into());
            let ids: Vec<&str> = ids.split(',').collect();
            let id = ids[(state.0 as usize - 1) % ids.len()].to_string();
            if let (Some(spell), Ok(me)) = (book.spells.iter().position(|x| x.id == id), player_e.single()) {
                casts.write(crate::magic::CastRequest { caster: me, spell, from: k.body.pos + Vec2::new(0.0, 9.0), toward: sk.body.pos, alt: false });
            }
            info!("slime: t {t:.1} {id} cast at it (hp {:.0})", sh.hp);
            state.0 += 1;
        }
        "attack" if state.0 == 1 && t > 2.0 => {
            let harm = match std::env::var("PLATYPUS_GLUT_HARM").as_deref() {
                Ok("fire") => crate::creatures::Harm::Fire,
                Ok("frost") => crate::creatures::Harm::Frost,
                Ok("acid") => crate::creatures::Harm::Acid,
                Ok("poison") => crate::creatures::Harm::Poison,
                Ok("radiant") => crate::creatures::Harm::Radiant,
                Ok("void") => crate::creatures::Harm::Void,
                _ => crate::creatures::Harm::Storm,
            };
            strike(&mut hits, harm, 6.0);
            info!("slime: t {t:.1} struck with {harm:?}");
            *took = 0.0;
            state.0 = 2;
        }
        "split" if state.0 >= 1 && state.0 <= 2 && t > 2.0 + state.0 as f32 * 1.5 => {
            strike(&mut hits, crate::creatures::Harm::Slash, 30.0);
            info!("slime: t {t:.1} a blade through it");
            state.0 += 1;
        }
        "heal" if state.0 == 1 && t > 2.0 => {
            strike(&mut hits, crate::creatures::Harm::Slash, 24.0);
            info!("slime: t {t:.1} a longsword's blow");
            state.0 = 2;
        }
        "heal" if state.0 == 2 && t > 2.6 => {
            strike(&mut hits, crate::creatures::Harm::Slash, 24.0);
            info!("slime: t {t:.1} another");
            state.0 = 3;
        }
        "look" if state.0 >= 1 && t > state.1 && state.1 <= 6.0 => {
            strike(&mut hits, crate::creatures::Harm::Slash, 6.0);
            info!("slime: t {t:.1} struck");
            state.1 += 2.0;
        }
        _ => {}
    }
    // `dash`: from 2.4 s the player runs at it and dashes (2.5 s), and
    // again back (4.4 s, 4.5 s): through it.
    if mode == "dash" {
        let toward = if sk.body.pos.x > k.body.pos.x { KeyCode::KeyD } else { KeyCode::KeyA };
        let away = if toward == KeyCode::KeyD { KeyCode::KeyA } else { KeyCode::KeyD };
        let run = (2.4..3.0).contains(&t) || (4.4..5.0).contains(&t);
        keys.release(away);
        if run { keys.press(toward) } else { keys.release(toward) }
        if (2.5..2.53).contains(&t) || (4.5..4.53).contains(&t) {
            keys.press(KeyCode::ShiftLeft);
        } else {
            keys.release(KeyCode::ShiftLeft);
        }
        if run {
            info!("slime: t {t:.2} dashing: the player at {:.0}, it at {:.0}; lost {:.0} in all", k.body.pos.x, sk.body.pos.x, *took);
        }
    }
    if mode == "squeeze"
        && let Ok(mut c) = drive.get_mut(e)
    {
        c.0.move_x = -1.0;
    }
    if t >= state.2 && mode == "squeeze" {
        info!("slime: t {t:.1} at x {:.0}, {:.0} high (the gap: 880..950)", sk.body.pos.x, sk.body.half.y * 2.0);
    }
    if t >= state.2 && mode == "wand" {
        if let Ok((g, _)) = gluttons.get(e) {
            info!("slime: t {t:.1} hp {:.0}/{:.0}, holds {:?} ({:.2}){}", sh.hp, sh.max, g.took, g.charge(), if g.oiled(t) { ", oiled" } else { "" });
        }
    }
    if t >= state.2 && mode == "attack" {
        if let Ok((g, _)) = gluttons.get(e) {
            info!("slime: t {t:.1} the glutton holds {:?} ({:.2} left); the player {:.0} away, has lost {:.0} in all", g.took, g.charge(), sk.body.pos.distance(k.body.pos), *took);
        }
    }
    if t >= state.2 && mode == "glut" {
        if let Ok((g, gg)) = gluttons.get(e) {
            info!("slime: t {t:.1} the glutton holds {:?}, its goo {}", g.took, gg.material.map_or("its own".to_string(), |m| sim.world.materials().def(m).name.clone()));
        }
    }
    if t >= state.2 && mode == "eat" {
        let held: Vec<String> = swallowed.iter().map(|(_, d, c)| if d { "an item".to_string() } else { c.map_or("?".to_string(), |c| c.kind.clone()) }).collect();
        info!("slime: t {t:.1} in it: [{}]; items about: {}", held.join(", "), drops.iter().count());
    }
    if t >= state.2 && matches!(mode.as_str(), "split" | "merge" | "many") {
        let all: Vec<String> = them.iter().filter(|x| x.1.kind == kind).map(|x| format!("{:.0}/{:.0} holds {} at {:.0}", x.3.hp, x.3.max, x.4.map_or(0, |g| g.held()), x.2.body.pos.x)).collect();
        info!("slime: t {t:.1} slimes: {}", all.join("; "));
    }
    if t >= state.2 {
        state.2 = t + 0.5;
        info!("slime: t {t:.1} hp {:.0}/{:.0}, holds {} goo, loose near {}, at x {:.0} ({:.0} up); the player's {:.0}% slimy, has taken {:.0}; {:?}; spikes {:?}", sh.hp, sh.max, goo.map_or(0, |g| g.held()), loose(&sim, sk.body.pos, "green_goo"), sk.body.pos.x, sk.body.pos.y - sk.body.half.y - fl, slimy * 100.0, *took, alert.map(|a| a.wary), goo.map(|g| g.spiking));
    }
}

/// Loose cells of a material (not held) near a point.
fn loose(sim: &SimWorld, at: Vec2, name: &str) -> usize {
    let Some(m) = sim.materials().id(name) else { return 0 };
    let c = CellPos::from_world(at.x, at.y);
    let mut n = 0;
    for y in c.y - 40..c.y + 40 {
        for x in c.x - 120..c.x + 120 {
            if sim.world.get(CellPos::new(x, y)).is_some_and(|cell| cell.material == m && cell.flags & platypus_sim::cell::flags::HELD == 0) {
                n += 1;
            }
        }
    }
    n
}

/// `night` (the world, with PLATYPUS_HOUR=22; PLATYPUS_LIFE_RATE to hurry
/// it): the player stands where they start (they can't die); every 2 s,
/// the zombies near, and how many are still coming up out of the ground.
#[allow(clippy::type_complexity)]
fn night_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(&crate::creatures::Creature, &Kinematics, Has<crate::creatures::body::emerge::Emerging>), Without<LocalPlayer>>,
    mut next: Local<f32>,
) {
    if s.name != "night" {
        return;
    }
    let Ok((mut pk, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    // (PLATYPUS_NIGHT_X: that far from the spawn instead, on the ground.)
    if let Some(dx) = std::env::var("PLATYPUS_NIGHT_X").ok().and_then(|v| v.parse::<f32>().ok())
        && s.elapsed < 1.0
    {
        let x = sim.generator.spawn_point().x as f32 + dx;
        if let Some(top) = sim.generator.surface_hint(x as i32) {
            pk.body.pos = Vec2::new(x, top as f32 + pk.body.half.y + 2.0);
            pk.prev_pos = pk.body.pos;
            pk.body.vel = Vec2::ZERO;
        }
    }
    if s.elapsed < *next {
        return;
    }
    *next = s.elapsed + 2.0;
    let near: Vec<_> = them.iter().filter(|(c, k, _)| c.kind.starts_with("zombie") && k.body.pos.distance(pk.body.pos) < 700.0).collect();
    let coming = near.iter().filter(|(_, _, e)| *e).count();
    let at: Vec<String> = near.iter().map(|(_, k, _)| format!("{:+.0}", k.body.pos.x - pk.body.pos.x)).collect();
    let slimes: Vec<String> = them.iter().filter(|(c, k, _)| (c.kind.contains("slime") || c.kind == "glutton") && k.body.pos.distance(pk.body.pos) < 700.0).map(|(c, ..)| c.kind.clone()).collect();
    if !slimes.is_empty() {
        info!("night: t {:.0} slimes near: {}", s.elapsed, slimes.join(", "));
    }
    let biome = sim.generator.biome_hint(pk.body.pos.x as i32).unwrap_or("?");
    info!("night: t {:.0} in {biome}: zombies near {} ({} coming up) at {}", s.elapsed, near.len(), coming, at.join(" "));
}

/// `zombie` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a zombie walks
/// at the player (who can't die). `PLATYPUS_ZOMBIE`:
///   rise (default): killed at 3 s, its body gets up again; killed again
///     at 15 s, it stays down (it gets up once).
///   limbs: its arms off (2 s, 3 s: it bites), its legs (4 s, 5 s: it
///     crawls on at the player), its head (10 s: dead for good).
///   break: killed at 3 s, its body broken at 5 s: it never gets up.
/// Logs what's there every half second.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn zombie_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health, Has<crate::creatures::moves::Held>), With<LocalPlayer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut began: MessageReader<crate::creatures::moves::Began>,
    (mut hand, mut over): (ResMut<crate::hands::Hand>, ResMut<crate::camera::CursorOverride>),
    alight: Query<&crate::creatures::Health, (With<crate::creatures::body::rise::Ablaze>, Without<LocalPlayer>)>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, &crate::combat::Wielding, Option<&crate::creatures::body::parts::Severed>, Has<crate::creatures::body::wounds::Legless>, &crate::creatures::Health), Without<LocalPlayer>>,
    piles: Query<(Entity, &Kinematics), (With<crate::creatures::body::rise::Pile>, Without<LocalPlayer>)>,
    bodies: Query<&crate::hands::corpses::Corpse, Without<crate::creatures::body::rise::Pile>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut cuts: MessageWriter<crate::creatures::body::wounds::Cut>,
    mut state: Local<(u8, f32)>,
    mut took: Local<f32>,
) {
    use crate::creatures::body::parts::Limb;
    if s.name != "zombie" {
        return;
    }
    let mode = std::env::var("PLATYPUS_ZOMBIE").unwrap_or_else(|_| "rise".into());
    let Ok((mut k, mut h, held)) = player.single_mut() else { return };
    // `shake`: held, it mashes jump to break free.
    if mode == "torch" && state.0 > 2 {
        if let Some((_, p)) = piles.iter().next() {
            over.0 = Some(p.body.pos);
        }
    }
    // `chase`: the player walks away, now and then, keeping ahead.
    if mode == "chase" && state.0 >= 1 && s.elapsed > 1.0 && (s.elapsed * 1.6) as i32 % 2 == 0 {
        keys.press(KeyCode::KeyA);
    } else if mode == "chase" {
        keys.release(KeyCode::KeyA);
    }
    // `touch`: the player walks into it, and keeps walking into it.
    if mode == "touch" && state.0 >= 1 {
        keys.press(KeyCode::KeyD);
    }
    if mode == "shake" {
        if held && (s.elapsed * 7.0) as i32 % 2 == 0 {
            keys.press(KeyCode::Space);
        } else {
            keys.release(KeyCode::Space);
        }
    }
    // (What the player took, kept up: it can't die.)
    *took += h.max - h.hp;
    h.hp = h.max;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let zombie = them.iter().find(|t| t.1.kind.starts_with("zombie"));
    let up = them.iter().filter(|t| t.1.kind.starts_with("zombie")).count();
    let kill = |hits: &mut MessageWriter<crate::combat::Hit>, e: Entity, at: Vec2| {
        hits.write(crate::combat::Hit { by: None, target: e, damage: 999.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at, dir: Vec2::X, weight: 1.0, crit: false });
    };
    let t = s.elapsed;
    for b in began.read() {
        info!("zombie: t {t:.1} a move: {}", b.id);
    }
    match state.0 {
        0 if t > 0.5 => {
            for (e, c, ..) in &them {
                if c.kind == "dummy" || c.kind == "sandbag" {
                    commands.entity(e).despawn();
                }
            }
            k.body.pos = Vec2::new(900.0, fl + k.body.half.y);
            k.prev_pos = k.body.pos;
            if mode == "touch" {
                // (A skeleton standing there, its brain gone: it only
                // touches.)
                crate::creatures::def::spawn_creature(&mut commands, "skeleton", Vec2::new(960.0, fl), |e| {
                    e.remove::<crate::creatures::brain::hunter::Hunter>();
                });
            } else if mode == "emerge" {
                // (Earth under grass where they come up: they dig through
                // soft ground only.)
                let floor = platypus_worldgen::arena::FLOOR;
                if let (Some(dirt), Some(grass)) = (sim.materials().id("dirt"), sim.materials().id("grass")) {
                    for x in 920..1060 {
                        for y in floor - 24..floor {
                            sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: if y == floor - 1 { grass } else { dirt }, overwrite: true });
                        }
                    }
                }
                // (Three coming up out of the ground, one after another.)
                for n in 0..3 {
                    crate::creatures::def::spawn_creature(&mut commands, "zombie", Vec2::new(960.0 + n as f32 * 26.0, fl), move |e| {
                        e.insert((crate::creatures::body::emerge::Emerging::after(0.5 + n as f32 * 0.9), crate::creatures::Dormant, Visibility::Hidden));
                    });
                }
            } else {
                crate::creatures::def::spawn_creature(&mut commands, "zombie", Vec2::new(1000.0, fl), |_| {});
            }
            info!("zombie: one walks at the player ({mode})");
            state.0 = 1;
        }
        1.. => {
            let at = zombie.map(|z| (z.0, z.2.body.pos));
            let cut = |cuts: &mut MessageWriter<crate::creatures::body::wounds::Cut>, limb: Limb| {
                if let Some((e, _)) = at {
                    cuts.write(crate::creatures::body::wounds::Cut { target: e, limb, dir: Vec2::new(1.0, 0.4) });
                    info!("zombie: t {t:.1} cut off: {limb:?}");
                }
            };
            let plan: &[(f32, &str)] = match mode.as_str() {
                "limbs" => &[(2.0, "arm0"), (3.0, "arm1"), (4.0, "leg0"), (5.0, "leg1"), (10.0, "head")],
                "break" => &[(3.0, "kill"), (5.0, "break")],
                "grab" | "shake" | "emerge" | "sun" | "touch" | "chase" => &[],
                "burn" => &[(3.0, "kill"), (5.0, "fire")],
                "torch" => &[(3.0, "kill"), (4.0, "torch")],
                _ => &[(3.0, "kill"), (15.0, "kill"), (27.0, "kill")],
            };
            let i = state.0 as usize - 1;
            if let Some(&(when, what)) = plan.get(i)
                && t > when
            {
                match what {
                    "arm0" => cut(&mut cuts, Limb::Arm(0)),
                    "arm1" => cut(&mut cuts, Limb::Arm(1)),
                    "leg0" => cut(&mut cuts, Limb::Leg(0)),
                    "leg1" => cut(&mut cuts, Limb::Leg(1)),
                    "head" => cut(&mut cuts, Limb::Head),
                    "kill" => {
                        if let Some((e, p)) = at {
                            kill(&mut hits, e, p);
                            info!("zombie: t {t:.1} killed");
                        }
                    }
                    "fire" => {
                        if let Some((e, p)) = piles.iter().next() {
                            hits.write(crate::combat::Hit { by: None, target: e, damage: 4.0, harm: crate::creatures::Harm::Fire, knock: Vec2::ZERO, stun: 0.0, at: p.body.pos, dir: Vec2::X, weight: 0.2, crit: false });
                            info!("zombie: t {t:.1} a spark of fire on its body");
                        }
                    }
                    "torch" => {
                        // (The player beside the body, a torch out, held to it.)
                        if let Some((_, p)) = piles.iter().next() {
                            k.body.pos = Vec2::new(p.body.pos.x - 12.0, fl + k.body.half.y);
                            k.prev_pos = k.body.pos;
                            hand.bar = 0;
                            hand.slot = 2;
                            over.0 = Some(p.body.pos);
                            info!("zombie: t {t:.1} a torch held to its body");
                        }
                    }
                    "break" => {
                        if let Some((e, p)) = piles.iter().next() {
                            kill(&mut hits, e, p.body.pos);
                            info!("zombie: t {t:.1} its body struck hard");
                        }
                    }
                    _ => {}
                }
                state.0 += 1;
            }
            if t >= state.1 {
                state.1 = t + 0.5;
                let what = match zombie {
                    Some((_, _, zk, w, sev, legless, zh)) => format!(
                        "up at x {:.0} ({:.0} up, {:.0} high), hp {:.0}, fights with {:?}, lost {:?}{}",
                        zk.body.pos.x,
                        zk.body.pos.y - zk.body.half.y - fl,
                        zk.body.half.y * 2.0,
                        zh.hp,
                        w.0,
                        sev.map(|s| s.0.clone()).unwrap_or_default(),
                        if legless { ", legless" } else { "" }
                    ),
                    None => "none up".into(),
                };
                let kind = zombie.map_or("", |z| z.1.kind.as_str());
                if mode == "chase"
                    && let Some(z) = zombie
                {
                    info!("zombie: t {t:.1} chase: {:.0} behind the player, going {:.0}", z.2.body.pos.x - k.body.pos.x, z.2.body.vel.x.abs());
                }
                if mode == "touch" {
                    info!("zombie: t {t:.1} touch: the player at x {:.0}, has taken {:.0}", k.body.pos.x, *took);
                }
                info!("zombie: t {t:.1} ({up}) {kind} {what}; bodies getting up {}{}, bodies {}; the player's taken {:.0}{}", piles.iter().count(), alight.iter().next().map_or(String::new(), |h| format!(" (alight, {:.0}/{:.0})", h.hp, h.max)), bodies.iter().count(), *took, if held { ", HELD" } else { "" });
            }
        }
        _ => {}
    }
}

/// `limp` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): ragdolls. A stone
/// ledge 24 cells up beside the player; at 1.5 s an orc on its edge dies
/// where it stands (it should slump over the edge), an orc below is knocked
/// into the ledge's face, and an orc high in the air dies falling. At 4 s a
/// blast among them throws the bodies. Logs where each body's pieces are.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn limp_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    limps: Query<&crate::creatures::body::ragdoll::Limp>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<u8>,
) {
    if s.name != "limp" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let floor = platypus_worldgen::arena::FLOOR;
    let fl = floor as f32;
    match *state {
        0 if s.elapsed > 0.5 => {
            for (e, c, _) in &them {
                if c.kind == "dummy" || c.kind == "sandbag" {
                    commands.entity(e).despawn();
                }
            }
            let Some(stone) = sim.materials().id("stone") else { return };
            for x in 960..1000 {
                for y in floor..floor + 24 {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: stone, overwrite: true });
                }
            }
            k.body.pos = Vec2::new(900.0, fl + k.body.half.y);
            k.prev_pos = k.body.pos;
            *state = 1;
        }
        1 if s.elapsed > 0.8 => {
            let still = |e: &mut EntityWorldMut| {
                e.remove::<crate::creatures::brain::hunter::Hunter>();
            };
            // `PLATYPUS_KINDS=raptor,spider,...`: those in a row past the ledge
            // instead, each struck dead where it stands.
            if let Ok(kinds) = std::env::var("PLATYPUS_KINDS") {
                for (i, kind) in kinds.split(',').enumerate() {
                    crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(1015.0 + i as f32 * 45.0, fl), still);
                }
                *state = 2;
                return;
            }
            crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(963.0, fl + 24.0), still);
            crate::creatures::def::spawn_creature(&mut commands, "orc", Vec2::new(935.0, fl), still);
            crate::creatures::def::spawn_creature(&mut commands, "skeleton", Vec2::new(925.0, fl + 70.0), still);
            *state = 2;
        }
        2 if s.elapsed > 1.5 && std::env::var("PLATYPUS_KINDS").is_ok() => {
            for (e, c, tk) in &them {
                if c.kind != "dummy" {
                    hits.write(crate::combat::Hit { by: None, target: e, damage: 9999.0, harm: crate::creatures::Harm::Slash, knock: Vec2::new(30.0, 0.0), stun: 0.5, at: tk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
                }
            }
            k.body.pos = Vec2::new(975.0, fl + 24.0 + k.body.half.y);
            k.prev_pos = k.body.pos;
            *state = 3;
        }
        2 if s.elapsed > 1.5 => {
            let mut all: Vec<(Entity, Vec2)> = them.iter().filter(|t| t.1.kind != "dummy").map(|t| (t.0, t.2.body.pos)).collect();
            all.sort_by(|a, b| b.1.y.total_cmp(&a.1.y));
            // (Highest first: the falling skeleton, the orc on the ledge, the orc below.)
            let blows = [(crate::creatures::Harm::Blunt, Vec2::new(0.0, 0.0)), (crate::creatures::Harm::Slash, Vec2::new(-10.0, 0.0)), (crate::creatures::Harm::Blunt, Vec2::new(260.0, 120.0))];
            for ((e, at), (harm, knock)) in all.into_iter().zip(blows) {
                hits.write(crate::combat::Hit { by: None, target: e, damage: 999.0, harm, knock, stun: 0.5, at, dir: knock.normalize_or(Vec2::X), weight: 1.0, crit: false });
            }
            *state = 3;
        }
        3 if s.elapsed > 4.0 => {
            sim.queue(WorldEdit::Explode { center: CellPos::new(950, floor + 4), radius: 14, power: 60 });
            *state = 4;
        }
        _ => {}
    }
    if *state >= 3 && ((s.elapsed * 10.0) as i32) % 10 == 0 {
        for l in &limps {
            let at: Vec<String> = l.pieces.iter().map(|p| format!("{} ({:.0}, {:.0})", p.name, p.pos.x, p.pos.y - fl)).collect();
            info!("limp: t {:.1} {}{}", s.elapsed, at.join(" "), if l.asleep { " asleep" } else { "" });
        }
    }
}

/// `rise` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a skeleton by the
/// player (who can't die). At 2 s a sword kills it: its bones lie in a heap
/// and it stands again 5 s on (7.6 s, bones flying together), at half its
/// health. At 9 s it's killed again, at 9.5 s its sword is taken from the
/// heap, at 10 s a sword chips the heap: it gets up without a sword. At
/// 16 s killed again, at 16.5 s a hammer on the heap: broken, dead for good
/// (a body). Logs, each half second, the skeleton, the heaps and the bodies.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn rise_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    mut chests: ResMut<crate::hands::chests::Chests>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, &crate::creatures::Health, &crate::combat::Wielding), Without<LocalPlayer>>,
    piles: Query<(Entity, &crate::creatures::Health, &crate::hands::corpses::Corpse), (With<crate::creatures::body::rise::Pile>, Without<LocalPlayer>)>,
    bodies: Query<&crate::hands::corpses::Corpse, Without<crate::creatures::body::rise::Pile>>,
    mut hits: MessageWriter<crate::combat::Hit>,
    (mut hand, mut over, mut mouse): (ResMut<crate::hands::Hand>, ResMut<crate::camera::CursorOverride>, ResMut<ButtonInput<MouseButton>>),
    mut state: Local<(u8, f32)>,
    mut next: Local<f32>,
    mut took: Local<f32>,
) {
    if s.name != "rise" {
        return;
    }
    // `PLATYPUS_RISE_MELEE=1`: no struck blows: from 2 s the player swings
    // its longsword at the skeleton (or its heap), as a player would.
    let melee = std::env::var("PLATYPUS_RISE_MELEE").is_ok();
    let (Some(items), Ok((mut k, mut h))) = (items, player.single_mut()) else { return };
    // (What the player took, kept up: it can't die.)
    *took += h.max - h.hp;
    h.hp = h.max;
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let skeleton = them.iter().find(|t| t.1.kind == "skeleton").map(|t| (t.0, t.2.body.pos));
    let pile = piles.iter().next().map(|p| (p.0, p.2.key));
    let mut strike = |at: Option<(Entity, Vec2)>, harm: crate::creatures::Harm, damage: f32| {
        if let Some((e, pos)) = at {
            hits.write(crate::combat::Hit { by: None, target: e, damage, harm, knock: Vec2::new(60.0, 40.0), stun: 0.3, at: pos, dir: Vec2::X, weight: 1.0, crit: false });
        }
    };
    let pile_at = pile.map(|(e, _)| (e, k.body.pos));
    if melee && state.0 >= 1 {
        hand.bar = 1;
        hand.slot = 8;
        let target = skeleton.map(|t| t.1).or(piles.iter().next().map(|_| k.body.pos + Vec2::new(30.0, 0.0)));
        over.0 = target;
        if s.elapsed > 2.0 && target.is_some_and(|t| t.distance(k.body.pos) < 40.0) {
            mouse.press(MouseButton::Left);
        } else {
            mouse.release(MouseButton::Left);
        }
    }
    match state.0 {
        _ if melee && state.0 >= 1 => {}
        0 if s.elapsed > 0.5 => {
            for (e, c, ..) in &them {
                if c.kind == "dummy" || c.kind == "sandbag" {
                    commands.entity(e).despawn();
                }
            }
            k.body.pos = Vec2::new(900.0, fl + k.body.half.y);
            k.prev_pos = k.body.pos;
            // `PLATYPUS_RISE_ROOF=n`: a stone roof n cells over the floor (a
            // cave's low tunnel).
            if let Some(n) = std::env::var("PLATYPUS_RISE_ROOF").ok().and_then(|v| v.parse::<i32>().ok()) {
                let floor = platypus_worldgen::arena::FLOOR;
                if let Some(stone) = sim.materials().id("stone") {
                    for x in 860..1120 {
                        for y in floor + n..floor + n + 10 {
                            sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: stone, overwrite: true });
                        }
                    }
                }
            }
            crate::creatures::def::spawn_creature(&mut commands, "skeleton", Vec2::new(990.0, fl), |_| {});
            *state = (1, s.elapsed);
        }
        1 if s.elapsed > 2.0 => {
            info!("rise: a sword kills it");
            strike(skeleton, crate::creatures::Harm::Slash, 999.0);
            *state = (2, s.elapsed);
        }
        2 if s.elapsed > 9.0 => {
            info!("rise: a sword kills it again");
            strike(skeleton, crate::creatures::Harm::Slash, 999.0);
            *state = (3, s.elapsed);
        }
        3 if s.elapsed > 9.5 => {
            if let Some((_, key)) = pile {
                let inv = chests.contents(key, &sim.world, &items);
                for slot in inv.slots.iter_mut() {
                    if slot.is_some_and(|st| matches!(items.def(st.item).use_, crate::hands::items::Use::Melee(_))) {
                        info!("rise: its {} taken from the heap", items.def(slot.unwrap().item).name);
                        *slot = None;
                    }
                }
            }
            *state = (4, s.elapsed);
        }
        4 if s.elapsed > 10.0 => {
            info!("rise: a sword chips the heap");
            strike(pile_at, crate::creatures::Harm::Slash, 8.0);
            *state = (5, s.elapsed);
        }
        5 if s.elapsed > 16.0 => {
            info!("rise: killed a third time");
            strike(skeleton, crate::creatures::Harm::Slash, 999.0);
            *state = (6, s.elapsed);
        }
        6 if s.elapsed > 16.5 => {
            info!("rise: a hammer on the heap");
            strike(pile_at, crate::creatures::Harm::Blunt, 12.0);
            *state = (7, s.elapsed);
        }
        _ => {}
    }
    if state.0 >= 1 && s.elapsed >= *next {
        *next = s.elapsed + 0.5;
        let sk = them.iter().find(|t| t.1.kind == "skeleton").map(|t| format!("skeleton hp {:.0}/{:.0} wielding {:?}", t.3.hp, t.3.max, t.4.0)).unwrap_or("no skeleton".into());
        let hp: Vec<String> = piles.iter().map(|p| format!("{:.0}/{:.0}", p.1.hp, p.1.max)).collect();
        info!("rise: t {:.1} {sk}; heaps {:?}; bodies {}; the player took {:.0}", s.elapsed, hp, bodies.iter().count(), *took);
    }
}

/// `ledge` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): the player on a
/// stone ledge `PLATYPUS_LEDGE` cells up (30 by default), 40 wide, a
/// `PLATYPUS_KIND` (a skeleton) put down below 70 cells off: does it get up
/// to the player? Logs where it is each second, and when it's on the ledge.
fn ledge_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    mut state: Local<(u8, f32)>,
    mut highest: Local<f32>,
) {
    if s.name != "ledge" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let floor = platypus_worldgen::arena::FLOOR;
    let fl = floor as f32;
    let high = std::env::var("PLATYPUS_LEDGE").ok().and_then(|v| v.parse::<i32>().ok()).unwrap_or(30);
    let kind = std::env::var("PLATYPUS_KIND").unwrap_or_else(|_| "skeleton".into());
    // (PLATYPUS_LEDGE_WIDE: how wide; a narrow one's a pillar, too high to climb.)
    let wide = std::env::var("PLATYPUS_LEDGE_WIDE").ok().and_then(|v| v.parse::<i32>().ok()).unwrap_or(40);
    let (x0, x1) = (1040 - wide, 1040);
    // (PLATYPUS_LEDGE_THICK: a slab that thick at the top, open under.)
    let thick = std::env::var("PLATYPUS_LEDGE_THICK").ok().and_then(|v| v.parse::<i32>().ok()).unwrap_or(high);
    match state.0 {
        0 if s.elapsed > 0.5 => {
            let Some(stone) = sim.materials().id("stone") else { return };
            for x in x0..x1 {
                for y in floor + high - thick..floor + high {
                    sim.queue(WorldEdit::Paint { center: CellPos::new(x, y), radius: 0, material: stone, overwrite: true });
                }
            }
            for (e, c, _) in &them {
                if c.kind == "dummy" || c.kind == "sandbag" {
                    commands.entity(e).despawn();
                }
            }
            *state = (1, s.elapsed);
        }
        1 if s.elapsed > 1.0 => {
            k.body.pos = Vec2::new((x0 + x1) as f32 / 2.0, fl + high as f32 + k.body.half.y + 1.0);
            k.body.vel = Vec2::ZERO;
            k.prev_pos = k.body.pos;
            crate::creatures::def::spawn_creature(&mut commands, &kind, Vec2::new(930.0, fl), |_| {});
            info!("ledge: a ledge {high} up, a {kind} below");
            *state = (2, s.elapsed);
        }
        2 => {
            k.body.pos.x = (x0 + x1) as f32 / 2.0;
            k.body.vel.x = 0.0;
            let Some((_, _, c)) = them.iter().find(|(_, c, _)| c.kind == kind) else { return };
            let feet = c.body.pos.y - c.body.half.y;
            if feet - fl > *highest + 2.0 {
                *highest = feet - fl;
                info!("ledge: t {:.1} the {kind}'s feet as high as {:.0}", s.elapsed, *highest);
            }
            if feet >= fl + high as f32 - 1.0 && (x0 as f32..x1 as f32).contains(&c.body.pos.x) {
                info!("ledge: t {:.1} the {kind} is up on the ledge ({high} up)", s.elapsed);
                state.0 = 3;
            } else if s.elapsed >= state.1 + 1.0 {
                state.1 = s.elapsed;
                info!("ledge: t {:.0} the {kind} at x {:.0}, {:.0} up, vel {:?}, grounded {}", s.elapsed, c.body.pos.x, feet - fl, c.body.vel.round(), c.loco.grounded());
            }
        }
        _ => {}
    }
}

/// `ores` (a generated world): a chamber 200 under the surface, its right
/// wall laid with a band of each ore and gem from the floor up (copper,
/// iron, silver, gold, mithril, meteorite, amethyst, emerald, ruby), each
/// 14 high, open to the chamber; the same again buried 12 cells into the
/// rock behind it. The player at its left, facing it. Pictures into
/// `PLATYPUS_ORES` (a dir): `ores_dark.png` (only its faint lantern),
/// `ores_torch.png` (a torch), `ores_beam.png` (a flashlight on the wall).
/// What's buried should never show; what's open, only in the light.
#[allow(clippy::type_complexity)]
fn ores_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    offscreen: Option<Res<crate::camera::Offscreen>>,
    mut day: ResMut<crate::light::Daylight>,
    mut toggles: ResMut<crate::light::LightToggles>,
    mut cursor: ResMut<CursorOverride>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    mut state: Local<(u8, Vec2)>,
) {
    if s.name != "ores" {
        return;
    }
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let t = s.elapsed;
    let shot = |commands: &mut Commands, name: &str| {
        if let Ok(dir) = std::env::var("PLATYPUS_ORES") {
            let image = match &offscreen {
                Some(o) => Screenshot::image(o.0.clone()),
                None => Screenshot::primary_window(),
            };
            commands.spawn(image).observe(save_to_disk(format!("{dir}/ores_{name}.png")));
        }
    };
    if state.0 == 0 && t > 0.5 {
        day.skipped = (day.skipped + 23.0 - day.time * 24.0).rem_euclid(24.0);
        let x = k.body.pos.x as i32;
        let top = sim.generator.surface_hint(x).unwrap_or(k.body.pos.y as i32);
        let floor = top - 200;
        let mats = sim.materials().clone();
        let Some(stone) = mats.id("stone") else { return };
        // The rock round it all stone, the chamber dug out of it.
        let (x0, x1) = (x - 160, x + 160);
        for y in floor - 30..floor + 170 {
            for xx in x0..=x1 {
                let p = CellPos::new(xx, y);
                let inside = xx > x0 + 20 && xx < x1 - 40 && y >= floor && y < floor + 130;
                let cell = if inside { platypus_sim::Cell::AIR } else { platypus_sim::Cell::new(stone, (xx * 31 + y * 17) as u8) };
                sim.world.set(p, cell);
            }
        }
        let wall = x1 - 40;
        let names = ["copper_ore", "iron_ore", "silver_ore", "gold_ore", "mithril_ore", "meteorite", "amethyst", "emerald", "ruby"];
        for (i, name) in names.iter().enumerate() {
            let Some(m) = mats.id(name) else { continue };
            let y0 = floor + i as i32 * 14;
            for y in y0..y0 + 13 {
                // Open to the chamber: 8 deep into its wall.
                for xx in wall..wall + 8 {
                    let mut c = mats.spawn(m, &mut platypus_sim::rng::Rng::seeded(&[xx as u64, y as u64]));
                    c.shade = mats.natural_shade(m, xx, y, c.shade);
                    sim.world.set(CellPos::new(xx, y), c);
                }
                // Buried: 12 further in, 8 wide.
                for xx in wall + 20..wall + 28 {
                    let mut c = mats.spawn(m, &mut platypus_sim::rng::Rng::seeded(&[xx as u64, y as u64]));
                    c.shade = mats.natural_shade(m, xx, y, c.shade);
                    sim.world.set(CellPos::new(xx, y), c);
                }
            }
        }
        let at = Vec2::new((x0 + 60) as f32, floor as f32 + 1.0 + k.body.half.y);
        k.body.pos = at;
        k.prev_pos = at;
        k.body.vel = Vec2::ZERO;
        k.loco.facing = 1.0;
        toggles.enabled = true;
        toggles.carry = crate::light::Carry::Nothing;
        *state = (1, Vec2::new(wall as f32, floor as f32 + 60.0));
        info!("ores: the chamber's wall at x {wall}, floor {floor}");
    }
    let (wall, mid) = (state.1.x, state.1.y);
    // Kept where it was put (it would wander off).
    if state.0 >= 1 {
        let p = k.body.pos;
        k.body.vel.x = 0.0;
        k.prev_pos = p;
    }
    match state.0 {
        1 if t > 3.5 => {
            shot(&mut commands, "dark");
            state.0 = 2;
        }
        2 if t > 4.0 => {
            toggles.carry = crate::light::Carry::Torch;
            // (Walked up to the wall with it.)
            k.body.pos.x = wall - 40.0;
            k.prev_pos = k.body.pos;
            cursor.0 = Some(Vec2::new(wall, mid));
            state.0 = 3;
        }
        3 if t > 6.5 => {
            shot(&mut commands, "torch");
            state.0 = 4;
        }
        4 if t > 7.0 => {
            toggles.carry = crate::light::Carry::BigBeam;
            k.body.pos.x = wall - 140.0;
            k.prev_pos = k.body.pos;
            cursor.0 = Some(Vec2::new(wall, mid));
            state.0 = 5;
        }
        5 if t > 9.5 => {
            shot(&mut commands, "beam");
            state.0 = 6;
        }
        _ => {}
    }
}

/// `ratbend` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=stairs`, try
/// `PLATYPUS_NOLIGHT=1 PLATYPUS_ZOOM=3`): the player (who can't die) on the
/// high ledge at the right, a rat (`PLATYPUS_RAT`: `rat`, `rat_lithe`) on
/// the floor before it, rubble strewn between (lumps of stone, big and
/// small): it runs over the rubble at the ledge, up its face, over its lip.
/// Every half second: where the rat is, how fast, on what.
fn ratbend_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    mut state: Local<(bool, f32)>,
) {
    if s.name != "ratbend" {
        return;
    }
    menu.open = false;
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    let t = s.elapsed;
    let kind = std::env::var("PLATYPUS_RAT").unwrap_or_else(|_| "rat".into());
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let ground = |x: f32| find_ground(&sim.world, x as i32, fl as i32 + 300, 600).map_or(fl, |y| y as f32);
    if !state.0 && t > 0.5 {
        for (e, c, _) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        k.body.pos = Vec2::new(1700.0, ground(1700.0) + k.body.half.y);
        k.prev_pos = k.body.pos;
        let stone = sim.world.materials().expect_id("stone");
        let rubble: Vec<(i32, i32, i32)> = [(1452, 6), (1466, 3), (1478, 8), (1497, 4), (1506, 5), (1521, 7), (1538, 3), (1552, 6), (1569, 4), (1584, 7), (1600, 3)].iter().map(|&(x, r)| (x, ground(x as f32) as i32, r)).collect();
        let start = Vec2::new(1400.0, ground(1400.0) + 2.0);
        for (x, y, r) in rubble {
            sim.queue(WorldEdit::Paint { center: CellPos::new(x, y - r / 3), radius: r, material: stone, overwrite: false });
        }
        crate::creatures::def::spawn_creature(&mut commands, &kind, start, |_| {});
        info!("ratbend: a {kind} at 1400, rubble 1450..1600, the player on the ledge at 1700");
        state.0 = true;
        return;
    }
    if state.0 && t >= state.1 {
        state.1 = t + 0.5;
        for (_, c, rk) in them.iter().filter(|x| x.1.kind == kind) {
            info!("ratbend: t {t:.1} the {} at ({:.0}, {:.0}), going ({:.0}, {:.0}), {:?}{}", c.kind, rk.body.pos.x, rk.body.pos.y, rk.body.vel.x, rk.body.vel.y, rk.loco.state, rk.loco.clinging().map_or(String::new(), |d| format!(", holding on {d}")));
        }
    }
}

/// `bladesplash` (`PLATYPUS_WORLD=arena`, the sandbox): the player at the
/// water pool's right edge with a longsword (`PLATYPUS_WEAPON`), swinging
/// down into the water at 1, 2 and 3 s. Every 0.1 s: how much water's
/// flying.
fn bladesplash_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    items: Option<Res<crate::hands::items::Items>>,
    (mut hand, mut cursor, mut mouse): (ResMut<crate::hands::Hand>, ResMut<CursorOverride>, ResMut<ButtonInput<MouseButton>>),
    mut player: Query<(&mut Kinematics, &mut crate::hands::items::Inventory), With<LocalPlayer>>,
    mut state: Local<(bool, f32, usize)>,
) {
    if s.name != "bladesplash" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let (Some(items), Ok((mut k, mut inv))) = (items, player.single_mut()) else { return };
    if !state.0 {
        let fl = platypus_worldgen::arena::FLOOR as f32;
        k.body.pos = Vec2::new(637.0, fl + k.body.half.y + 1.0);
        k.prev_pos = k.body.pos;
        let w = std::env::var("PLATYPUS_WEAPON").unwrap_or_else(|_| "longsword".into());
        if let Some(id) = items.id(&w) {
            inv.slots[9] = Some(crate::hands::items::Stack::new(id, 1));
            (hand.bar, hand.slot) = (0, 9);
        }
        state.0 = true;
    }
    cursor.0 = Some(k.body.pos + Vec2::new(-45.0, -22.0));
    let swing = [1.0f32, 2.0, 3.0].iter().any(|&at| (at..at + 0.05).contains(&t));
    if swing { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
    let water = sim.materials().id("water");
    let flying = sim.world.particles().iter().filter(|p| Some(p.cell.material) == water).count();
    state.2 = state.2.max(flying);
    if t >= state.1 {
        state.1 = t + 0.1;
        info!("bladesplash: t {t:.1} water flying {flying} (most {})", state.2);
    }
}

/// `dryout` (a generated world, `small` is quickest): a flood dumped on the
/// land by the start (water where the seed put none), an hour passing with
/// the player there, then the player away for `PLATYPUS_DAYS` (default 1)
/// while the world clock runs, then back: how much of the water is left.
/// (`PLATYPUS_DAYS=0.25`: a quarter of a day, part of it.)
/// `firewall` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`, best with
/// `PLATYPUS_HOUR=23`): the player (kept alive) paints a wall of fire 40
/// high, 30 to its right (0.5-2 s); then a skeleton, a zombie and a rat are
/// put 70 beyond it, hunting the player, the wall kept painted while they
/// come. Logs each half second which side of the wall each is on, and its
/// health.
fn firewall_script(
    mut commands: Commands,
    s: Res<Scenario>,
    book: Res<crate::magic::Spellbook>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut casts: MessageWriter<crate::magic::CastRequest>,
    mut player: Query<(Entity, &Kinematics, &mut crate::creatures::Health, Option<&mut crate::magic::Mana>), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, &crate::creatures::Health), Without<LocalPlayer>>,
    mut state: Local<(u8, f32)>,
) {
    if s.name != "firewall" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let Ok((me, k, mut h, mana)) = player.single_mut() else { return };
    h.hp = h.max;
    if let Some(mut m) = mana {
        m.cur = m.max;
    }
    let Some(spell) = book.spells.iter().position(|d| d.id == "fire_wall") else { return };
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let wall = k.body.pos.x + 30.0;
    let (step, next) = &mut *state;
    // The wall painted (and painted again every few seconds, to keep it).
    let phase = (t - 0.5) % 4.0;
    if t > 0.5 && phase < 1.5 {
        let toward = Vec2::new(wall, fl + 1.0 + 40.0 * (phase / 1.5));
        casts.write(crate::magic::CastRequest { caster: me, spell, from: k.body.pos + Vec2::new(0.0, 6.0), toward, alt: false });
    }
    if *step == 0 && t > 2.2 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        for (i, kind) in ["skeleton", "zombie", "rat_lithe"].iter().enumerate() {
            crate::creatures::def::spawn_creature(&mut commands, kind, Vec2::new(wall + 70.0 + i as f32 * 12.0, fl + 14.0), |_| {});
        }
        *step = 1;
    }
    if *step >= 1 && t >= *next {
        *next = t + 0.5;
        let line: Vec<String> = them
            .iter()
            .filter(|(_, c, ..)| ["skeleton", "zombie", "rat_lithe"].contains(&c.kind.as_str()))
            .map(|(_, c, ck, ch)| format!("{} {:+.0} ({:.0} hp)", c.kind, ck.body.pos.x - wall, ch.hp))
            .collect();
        info!("firewall: t {t:.1} {}", line.join(", "));
    }
}

/// `fields` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): the player
/// (kept alive) casts a field 30 to its right (`PLATYPUS_FIELD`), again as
/// it runs out: `repulse` (default): a bubble of force over the floor; a zombie, a
/// rat and a skeleton archer put 70 beyond it, hunting the player, and a
/// zombie's spark bolt cast at the player through it (4 s); `blast`: the
/// same three, a repulsor opened right on the zombie (3 s); `launch`: one
/// opened at the player's feet (1 s); `bounce`: the player's own spark bolt
/// shot into one (2 s), an arrow before it (1.5 s). `slow`: a slow
/// field over the floor; a zombie charges through it, sand is dropped into
/// it from above, a bolt cast through it. `lift`: a pit 24 deep dug there,
/// sand in its bottom half, water over it, a rat in it; three updrafts cast
/// up from its floor; at 7 s the player is put in it to
/// ride it. Logs each half second.
fn fields_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    book: Res<crate::magic::Spellbook>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut casts: MessageWriter<crate::magic::CastRequest>,
    mut player: Query<(Entity, &mut Kinematics, &mut crate::creatures::Health, Option<&mut crate::magic::Mana>), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, &crate::creatures::Health), Without<LocalPlayer>>,
    spells: Query<&crate::magic::Spell>,
    arrows: Query<(&crate::archery::Arrow, &Transform)>,
    mut state: Local<(u8, f32, f32)>,
    mut fired: Local<std::collections::HashSet<u32>>,
    weapons: Option<Res<crate::combat::Weapons>>,
) {
    if s.name != "fields" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let kind = std::env::var("PLATYPUS_FIELD").unwrap_or_else(|_| "repulse".into());
    let Ok((me, mut k, mut h, mana)) = player.single_mut() else { return };
    if h.hp < h.max {
        info!("fields: t {t:.2} the player was hurt {:.0} ({:?})", h.max - h.hp, h.recent.last().map(|r| r.cause.clone()));
    }
    h.hp = h.max;
    if let Some(mut m) = mana {
        m.cur = m.max;
    }
    let id = match kind.as_str() {
        "slow" => "slow_field",
        "lift" => "updraft",
        _ => "repulsor",
    };
    let Some(spell) = book.spells.iter().position(|d| d.id == id) else { return };
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let (step, next, home) = &mut *state;
    if *step == 0 {
        *home = k.body.pos.x;
    }
    let x = *home + 30.0;
    // (The updrafts cast from over the pit, straight down into it.)
    let from = if kind == "lift" { Vec2::new(x, fl + 30.0) } else { k.body.pos + Vec2::new(0.0, 6.0) };
    // Cast once at the start, and again as it runs out.
    let mut cast = |toward: Vec2, at: f32| {
        if t >= at && fired.insert((at * 10.0) as u32) {
            casts.write(crate::magic::CastRequest { caster: me, spell, from, toward, alt: false });
        }
    };
    match kind.as_str() {
        "slow" => {
            // (Its middle well off, so the player stands outside it.)
            cast(Vec2::new(x + 40.0, fl + 14.0), 0.5);
            cast(Vec2::new(x + 40.0, fl + 14.0), 8.0);
        }
        "lift" => {
            if *step == 0 && t > 0.3 {
                let (sand, water) = (sim.materials().id("sand"), sim.materials().id("water"));
                if let (Some(sand), Some(water)) = (sand, water) {
                    for xx in x as i32 - 15..x as i32 + 15 {
                        for yy in fl as i32 - 24..fl as i32 {
                            let m = if yy < fl as i32 - 12 { sand } else { water };
                            sim.world.set(CellPos::new(xx, yy), platypus_sim::Cell::new(m, 128));
                        }
                    }
                }
            }
            cast(Vec2::new(x, fl - 22.0), 0.6);
            cast(Vec2::new(x - 9.0, fl - 22.0), 1.0);
            cast(Vec2::new(x + 9.0, fl - 22.0), 1.4);
            if (7.0..7.1).contains(&t) {
                k.body.pos = Vec2::new(x, fl + 10.0);
                k.body.vel = Vec2::ZERO;
            }
        }
        // (`bounce`: a repulsor ahead, the player's own spark bolt shot
        // into it, 2 s.)
        "bounce" => {
            // (Its middle 70 off: the player stands clear of it.)
            cast(Vec2::new(x + 40.0, fl + 16.0), 0.5);
            if t >= 2.0 && fired.insert(20_000)
                && let Some(bolt) = book.spells.iter().position(|d| d.id == "spark_bolt")
            {
                casts.write(crate::magic::CastRequest { caster: me, spell: bolt, from, toward: Vec2::new(x + 100.0, fl + 16.0), alt: false });
            }
            // (And an arrow of its own, 1.5 s.)
            if t >= 1.5 && fired.insert(15_000)
                && let Some(w) = weapons.as_deref()
            {
                crate::archery::spawn_arrow(&mut commands, w, from + Vec2::new(4.0, 0.0), Vec2::new(420.0, 0.0), me, Some(crate::creatures::Team::Player), (12.0, false), 0.0, 0.0);
            }
        }
        // (`launch`: a repulsor opened at the player's feet, 1 s.)
        "launch" => cast(Vec2::new(k.body.pos.x, fl - 4.0), 1.0),
        // (`blast`: no wall; a repulsor opened right on the zombie, 3 s.)
        "blast" => {
            if let Some((_, _, zk, _)) = them.iter().find(|(_, c, ..)| c.kind == "zombie") {
                cast(zk.body.pos, 3.0);
            }
        }
        _ => {
            cast(Vec2::new(x, fl + 16.0), 0.5);
            cast(Vec2::new(x, fl + 16.0), 7.5);
        }
    }
    if *step == 0 && t > 2.2 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        let kinds: &[&str] = match kind.as_str() {
            "slow" => &["zombie"],
            "lift" => &["rat"],
            _ => &["zombie", "rat_lithe", "skeleton_archer"],
        };
        for (i, kind_) in kinds.iter().enumerate() {
            let at = if kind == "lift" { Vec2::new(x, fl - 8.0) } else { Vec2::new(x + 70.0 + i as f32 * 14.0, fl + 14.0) };
            crate::creatures::def::spawn_creature(&mut commands, kind_, at, |_| {});
        }
        *step = 1;
    }
    // A bolt cast at the player by the zombie, through the field.
    if *step == 1 && t > 4.0 && kind != "lift" {
        *step = 2;
        if let (Some(bolt), Some((z, _, zk, _))) = (book.spells.iter().position(|d| d.id == "spark_bolt"), them.iter().find(|(_, c, ..)| c.kind == "zombie")) {
            casts.write(crate::magic::CastRequest { caster: z, spell: bolt, from: zk.body.pos, toward: k.body.pos, alt: false });
        }
    }
    // Sand dropped into the slow field from above.
    if kind == "slow" && (3.0..3.05).contains(&t)
        && let Some(sand) = sim.materials().id("sand")
    {
        for xx in x as i32 - 4..x as i32 + 4 {
            for yy in fl as i32 + 60..fl as i32 + 66 {
                sim.world.set(CellPos::new(xx, yy), platypus_sim::Cell::new(sand, 128));
            }
        }
    }
    if t >= *next {
        *next = t + 0.5;
        let line: Vec<String> = them
            .iter()
            .filter(|(_, c, ..)| c.kind != "dummy" && c.kind != "sandbag")
            .map(|(_, c, ck, ch)| format!("{} {:+.0},{:+.0} v{:.0} ({:.0} hp)", c.kind, ck.body.pos.x - x, ck.body.pos.y - fl, ck.body.vel.length(), ch.hp))
            .collect();
        let bolts: Vec<String> = spells.iter().map(|b| format!("{:+.0}{}", b.at().x - x, if b.caster() == me { " mine" } else { "" })).collect();
        let shafts: Vec<String> = arrows.iter().filter(|(a, _)| a.flying()).map(|(a, tf)| format!("{:+.0}{}", tf.translation.x - x, if a.shooter() == me { " mine" } else { "" })).collect();
        let mats = sim.world.materials();
        let count = |name: &str, lo: i32, hi: i32| {
            let Some(m) = mats.id(name) else { return 0 };
            (x as i32 - 15..x as i32 + 15).flat_map(|xx| (lo..hi).map(move |yy| (xx, yy))).filter(|&(xx, yy)| sim.world.get(CellPos::new(xx, yy)).is_some_and(|c| c.material == m)).count()
        };
        let sand_low = mats.id("sand").and_then(|m| (fl as i32..fl as i32 + 70).find(|&yy| (x as i32 - 6..x as i32 + 6).any(|xx| sim.world.get(CellPos::new(xx, yy)).is_some_and(|c| c.material == m)))).map_or(-1, |yy| yy - fl as i32);
        info!(
            "fields: t {t:.1} [{}] bolts [{}] arrows [{}] pit sand {} water {} | sand above floor from {} | me {:+.0},{:+.0}",
            line.join(", "),
            bolts.join(" "),
            shafts.join(" "),
            count("sand", fl as i32 - 24, fl as i32),
            count("water", fl as i32 - 24, fl as i32),
            sand_low,
            k.body.pos.x - x,
            k.body.pos.y - fl
        );
    }
}

/// `paint` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): the player holds a
/// brush spell (`PLATYPUS_BRUSH`, ice_wall) and draws: a column up off the
/// floor 30 to its right (1-3 s), a bridge out from its top (3-5 s), then a
/// stroke in the open air high over the floor (5.5-6.5 s). Logs the cells
/// of what it set, of the loose, and its mana, each second (then every ten,
/// to see it melt). `PLATYPUS_PAINT=pool`: water under where it draws;
/// `wall`: a stone wall between it and where it draws; `face`: a plank wall
/// beside where the column's drawn (it runs up its face); `tall`: the
/// column 100 high; `long`: the bridge 120 long.
fn paint_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    book: Res<crate::magic::Spellbook>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut casts: MessageWriter<crate::magic::CastRequest>,
    player: Query<(Entity, &Kinematics, Option<&crate::magic::Mana>), With<LocalPlayer>>,
    mut next: Local<f32>,
    mut dug: Local<bool>,
) {
    if s.name != "paint" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let Ok((me, k, mana)) = player.single() else { return };
    let id = std::env::var("PLATYPUS_BRUSH").unwrap_or_else(|_| "ice_wall".into());
    let Some(spell) = book.spells.iter().position(|d| d.id == id) else { return };
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let x = k.body.pos.x + 30.0;
    let lerp = |a: Vec2, b: Vec2, f: f32| a + (b - a) * f.clamp(0.0, 1.0);
    // (`PLATYPUS_PAINT=pool`: a pool 12 deep and 60 wide dug where it
    // draws, full of water; the column's foot on the water.)
    let pool = std::env::var("PLATYPUS_PAINT").is_ok_and(|v| v == "pool");
    // (`PLATYPUS_PAINT=wall`: a stone wall 70 high between it and where it
    // draws: only a brush that needs no line of sight gets there.)
    if std::env::var("PLATYPUS_PAINT").is_ok_and(|v| v == "wall") && !*dug && t > 0.4 {
        *dug = true;
        if let Some(stone) = sim.materials().id("stone") {
            for xx in x as i32 - 14..x as i32 - 9 {
                for yy in fl as i32..fl as i32 + 70 {
                    sim.world.set(CellPos::new(xx, yy), platypus_sim::Cell::new(stone, 128));
                }
            }
        }
    }
    // (`PLATYPUS_PAINT=face`: a wall of planks 40 high just right of where
    // the column's drawn: the column runs up its face.)
    if std::env::var("PLATYPUS_PAINT").is_ok_and(|v| v == "face") && !*dug && t > 0.4 {
        *dug = true;
        if let Some(planks) = sim.materials().id("planks") {
            for xx in x as i32 + 3..x as i32 + 9 {
                for yy in fl as i32..fl as i32 + 40 {
                    sim.world.set(CellPos::new(xx, yy), platypus_sim::Cell::new(planks, 128));
                }
            }
        }
    }
    if pool && !*dug && t > 0.4 {
        *dug = true;
        if let Some(water) = sim.materials().id("water") {
            for xx in x as i32 - 20..x as i32 + 40 {
                for yy in fl as i32 - 12..fl as i32 {
                    sim.world.set(CellPos::new(xx, yy), platypus_sim::Cell::new(water, 128));
                }
            }
        }
    }
    let foot = if pool { fl - 6.0 } else { fl + 1.0 };
    // (`PLATYPUS_PAINT=tall`: the column 100 high, over the same two seconds.)
    let top = if std::env::var("PLATYPUS_PAINT").is_ok_and(|v| v == "tall") { fl + 100.0 } else { fl + 30.0 };
    // (`PLATYPUS_PAINT=long`: the bridge 120 long, past where the old limit
    // of 30 out would have turned it to snow.)
    let span = if std::env::var("PLATYPUS_PAINT").is_ok_and(|v| v == "long") { 120.0 } else { 40.0 };
    let at = if (1.0..3.0).contains(&t) {
        Some(lerp(Vec2::new(x, foot), Vec2::new(x, top), (t - 1.0) / 2.0))
    } else if (3.0..5.0).contains(&t) {
        Some(lerp(Vec2::new(x, top), Vec2::new(x + span, top), (t - 3.0) / 2.0))
    } else if (5.5..6.5).contains(&t) {
        Some(lerp(Vec2::new(x - 20.0, fl + 70.0), Vec2::new(x + 20.0, fl + 70.0), t - 5.5))
    } else {
        None
    };
    if let Some(toward) = at {
        let from = k.body.pos + Vec2::new(0.0, 6.0);
        casts.write(crate::magic::CastRequest { caster: me, spell, from, toward, alt: false });
    }
    if t >= *next {
        // (Each second while it paints, then every ten: how long it lasts.)
        *next = if t < 10.0 { t + 1.0 } else { t + 10.0 };
        let mats = sim.world.materials();
        let count = |name: &str| {
            let Some(m) = mats.id(name) else { return 0 };
            (k.body.pos.x as i32 - 40..k.body.pos.x as i32 + 120).flat_map(|xx| (fl as i32..fl as i32 + 140).map(move |yy| (xx, yy))).filter(|&(xx, yy)| sim.world.get(CellPos::new(xx, yy)).is_some_and(|c| c.material == m)).count()
        };
        // (The ground round the column's foot: how cold it is.)
        let cold = (x as i32 - 12..x as i32 + 12).flat_map(|xx| (fl as i32 - 4..fl as i32).map(move |yy| CellPos::new(xx, yy))).filter_map(|p| sim.world.temperature(p)).min().unwrap_or(0);
        // (How high what it painted reaches over the floor, by the column.)
        let high = |name: &str| {
            let Some(m) = mats.id(name) else { return 0 };
            (fl as i32..fl as i32 + 140).rev().find(|&yy| (x as i32 - 6..x as i32 + 6).any(|xx| sim.world.get(CellPos::new(xx, yy)).is_some_and(|c| c.material == m))).map_or(0, |yy| yy - fl as i32)
        };
        info!("paint: t {t:.0} ice {} snow {} fire {} planks {} mana {:.0}; up to {} high; the floor at its foot down to {cold} °C", count("ice"), count("snow"), count("fire"), count("planks"), mana.map_or(0.0, |m| m.cur), high("ice").max(high("fire")));
    }
}

/// `dive` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a pool 70 deep dug
/// in the floor, full of water, a torch stuck in its bottom; the player, a
/// torch in its off hand, dropped in (1 s): it sinks, its torch goes out,
/// its breath runs down and it drowns a while; pulled out (26 s), its torch
/// lights again and its breath comes back. Logs it each half second.
fn dive_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    items: Option<Res<crate::hands::items::Items>>,
    settings: Res<crate::light::LightSettings>,
    art: Option<Res<crate::light::TorchArt>>,
    switch: Res<crate::light::switch::Switch>,
    mut player: Query<(Entity, &mut Kinematics, &crate::creatures::Health, &mut crate::gear::Equipment, Option<&crate::breath::Breath>), With<LocalPlayer>>,
    torches: Query<Has<crate::light::torch::Snuffed>, With<crate::light::PlantedTorch>>,
    mut state: Local<(u8, f32)>,
) {
    if s.name != "dive" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let (Some(items), Ok((me, mut k, h, mut eq, breath))) = (items, player.single_mut()) else { return };
    let fl = platypus_worldgen::arena::FLOOR;
    let (x0, x1, deep) = (920, 1000, 70);
    let (step, next) = &mut *state;
    match *step {
        0 if t > 0.3 => {
            commands.entity(me).remove::<crate::combat::Invulnerable>();
            let Some(water) = sim.materials().id("water") else { return };
            for x in x0..x1 {
                for y in fl - deep..fl {
                    sim.world.set(CellPos::new(x, y), platypus_sim::Cell::new(water, 128));
                }
            }
            if let Some(art) = art.as_deref() {
                crate::light::plant_torch(&mut commands, Vec2::new((x0 + 10) as f32, (fl - deep) as f32), &settings, art);
            }
            let torch = crate::hands::items::Stack::new(items.id("torch").expect("torch"), 1);
            if let Some(i) = (0..eq.worn.len()).find(|&i| crate::gear::Equipment::fits(&items, i, &torch)) {
                eq.worn[i] = Some(torch);
            }
            *step = 1;
        }
        1 if t > 1.0 => {
            k.body.pos = Vec2::new(960.0, fl as f32 + 4.0 + k.body.half.y);
            k.prev_pos = k.body.pos;
            *step = 2;
        }
        2 if t > 26.0 => {
            k.body.pos = Vec2::new(880.0, fl as f32 + k.body.half.y);
            k.prev_pos = k.body.pos;
            k.body.vel = Vec2::ZERO;
            info!("dive: pulled out at {t:.1}");
            *step = 3;
        }
        _ => {}
    }
    if *step >= 2 && t >= *next {
        *next = t + 0.5;
        let snuffed = torches.iter().filter(|&s| s).count();
        info!(
            "dive: t {t:.1} depth {:.0} sinking {:+.0} torch {} breath {:?} health {:.0}; torches out {snuffed}",
            fl as f32 - (k.body.pos.y - k.body.half.y),
            k.body.vel.y,
            if switch.hand.on { "lit" } else { "out" },
            breath.map(|b| (b.left * 10.0).round() / 10.0),
            h.hp
        );
    }
}

/// `airswing` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): the player jumps
/// (1 s) and swings its sword in the air as fast as it can, aimed
/// `PLATYPUS_AIRSWING` degrees (0, level; `none`: no swings): logs its height
/// and how fast it's rising or falling every 0.05 s till it lands, and the
/// time in the air.
fn airswing_script(
    s: Res<Scenario>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    items: Option<Res<crate::hands::items::Items>>,
    (mut hand, mut cursor, mut mouse): (ResMut<crate::hands::Hand>, ResMut<CursorOverride>, ResMut<ButtonInput<MouseButton>>),
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut player: Query<(&Kinematics, &mut crate::hands::items::Inventory), With<LocalPlayer>>,
    mut state: Local<(u8, f32, f32, f32)>,
) {
    if s.name != "airswing" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let (Some(items), Ok((k, mut inv))) = (items, player.single_mut()) else { return };
    let aim = std::env::var("PLATYPUS_AIRSWING").unwrap_or_else(|_| "0".into());
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let (step, next, took_off, top) = &mut *state;
    match *step {
        0 if t > 0.5 => {
            if let Some(id) = items.id("copper_shortsword") {
                inv.slots[9] = Some(crate::hands::items::Stack::new(id, 1));
                (hand.bar, hand.slot) = (0, 9);
            }
            *step = 1;
        }
        1 if t > 1.0 => {
            keys.press(KeyCode::Space);
            *took_off = t;
            *step = 2;
        }
        2 => {
            if t > *took_off + 0.25 {
                keys.release(KeyCode::Space);
            }
            *top = top.max(k.body.pos.y - k.body.half.y - fl);
            if let Ok(deg) = aim.parse::<f32>() {
                let a = deg.to_radians();
                cursor.0 = Some(k.body.pos + Vec2::new(a.cos() * 40.0, a.sin() * 40.0));
                if ((t - *took_off) * 20.0) as u32 % 2 == 0 { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
            }
            if t >= *next {
                *next = t + 0.05;
                info!("airswing: t {:.2} height {:.1} rising {:+.0}", t - *took_off, k.body.pos.y - k.body.half.y - fl, k.body.vel.y);
            }
            if t > *took_off + 0.3 && k.loco.grounded() {
                mouse.release(MouseButton::Left);
                info!("airswing: landed after {:.2} s in the air, {:.1} high at most (aim {aim})", t - *took_off, *top);
                *step = 3;
            }
        }
        _ => {}
    }
}

/// `batcount` (a generated world; `PLATYPUS_SEED`): the player (kept alive)
/// put on the floor of the start valley's bat roost, there 40 s: logs every
/// 2 s how many cave bats are round it (and of them lairs' keepers).
fn batcount_script(
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut player: Query<(&mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    bats: Query<(&crate::creatures::Creature, &Kinematics, Has<crate::clock::Keeps>), Without<LocalPlayer>>,
    homes: Query<(&crate::creatures::Creature, &crate::clock::Keeps)>,
    mut state: Local<(bool, f32, usize)>,
) {
    if s.name != "batcount" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let Ok((mut k, mut h)) = player.single_mut() else { return };
    h.hp = h.max;
    if !state.0 && t > 1.0 {
        let home = sim.generator.spawn_point();
        let Some((p, _)) = sim.generator.landmarks().into_iter().filter(|(_, n)| n == "bat roost").min_by_key(|(p, _)| (p.x - home.x).abs()) else {
            warn!("batcount: no bat roost");
            state.0 = true;
            return;
        };
        let floor = (0..200).map(|d| p.y - d).find(|&y| sim.world.is_solid(CellPos::new(p.x, y)) && !sim.world.is_solid(CellPos::new(p.x, y + 1)));
        k.body.pos = Vec2::new(p.x as f32, floor.unwrap_or(p.y) as f32 + 1.0 + k.body.half.y);
        k.prev_pos = k.body.pos;
        info!("batcount: put in the bat roost at ({}, {})", p.x, p.y);
        state.0 = true;
        return;
    }
    if state.0 && t >= state.1 {
        state.1 = t + 2.0;
        let near: Vec<bool> = bats.iter().filter(|(c, bk, _)| c.kind == "cave_bat" && bk.body.pos.distance(k.body.pos) < 400.0).map(|(.., keeper)| keeper).collect();
        let most = near.len();
        state.2 = state.2.max(most);
        info!("batcount: t {t:.0} {} bats near ({} lairs'); the most so far {}", most, near.iter().filter(|&&x| x).count(), state.2);
        if t < 4.0 {
            let mut at: Vec<(i32, i32)> = homes.iter().filter(|(c, _)| c.kind == "cave_bat").map(|(_, k)| k.0).collect();
            at.sort();
            info!("batcount: the bats' lairs: {at:?}");
        }
    }
}

/// `crushcheck` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): what falling
/// things do to the player: a boulder dropped on it from 40 cells (1 s),
/// then a log up on two posts over it, its middle chopped out so each half
/// swings down onto it (4.5 s); then the same log onto a zombie (8.5 s).
/// Logs health after each.
fn crushcheck_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut player: Query<(Entity, &mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(&crate::creatures::Creature, &crate::creatures::Health), Without<LocalPlayer>>,
    mut step: Local<(u8, f32)>,
) {
    if s.name != "crushcheck" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let Ok((me, mut k, mut h)) = player.single_mut() else { return };
    let fl = platypus_worldgen::arena::FLOOR;
    let x = 950;
    let (Some(boulder), Some(wood), Some(stone)) = (sim.materials().id("boulder"), sim.materials().id("wood"), sim.materials().id("stone")) else { return };
    let (st, mark) = &mut *step;
    match *st {
        0 if t > 0.5 => {
            commands.entity(me).remove::<crate::combat::Invulnerable>();
            k.body.pos = Vec2::new(x as f32, fl as f32 + k.body.half.y);
            k.prev_pos = k.body.pos;
            *st = 1;
        }
        1 if t > 1.0 => {
            *mark = h.hp;
            for dx in -6..=6i32 {
                for dy in -6..=6i32 {
                    if dx * dx + dy * dy <= 36 {
                        sim.world.set(CellPos::new(x + dx, fl + 40 + dy), platypus_sim::Cell::new(boulder, 120));
                    }
                }
            }
            sim.world.lift(CellPos::new(x, fl + 40));
            *st = 2;
        }
        2 if t > 4.0 => {
            info!("crushcheck: a boulder from 40 cells: health {:.0} -> {:.0}", *mark, h.hp);
            h.hp = h.max;
            k.body.pos = Vec2::new(x as f32, fl as f32 + k.body.half.y);
            k.prev_pos = k.body.pos;
            // (Clear what the boulder left.)
            for xx in x - 30..x + 30 {
                for yy in fl..fl + 50 {
                    sim.world.set(CellPos::new(xx, yy), platypus_sim::Cell::AIR);
                }
            }
            *st = 3;
        }
        3 if t > 4.5 => {
            *mark = h.hp;
            // A log (70 × 7) up on two posts 18 high at its ends, the
            // player under its middle; its middle chopped out: each half
            // swings down about its post onto it.
            for px in [x - 34, x + 33] {
                for yy in fl..fl + 18 {
                    for xx in px - 1..px + 2 {
                        sim.world.set(CellPos::new(xx, yy), platypus_sim::Cell::new(stone, 120));
                    }
                }
            }
            for xx in x - 36..x + 36 {
                for yy in fl + 18..fl + 25 {
                    if (xx - x).abs() > 2 {
                        sim.world.set(CellPos::new(xx, yy), platypus_sim::Cell::new(wood, 120));
                    }
                }
            }
            sim.world.lift(CellPos::new(x - 20, fl + 21));
            sim.world.lift(CellPos::new(x + 20, fl + 21));
            *st = 4;
        }
        4 if t > 8.0 => {
            info!("crushcheck: a log's two halves swinging down onto it: health {:.0} -> {:.0}", *mark, h.hp);
            // (Then the same over a zombie, the player well away.)
            k.body.pos = Vec2::new(x as f32 - 120.0, fl as f32 + k.body.half.y);
            k.prev_pos = k.body.pos;
            for xx in x - 40..x + 40 {
                for yy in fl..fl + 50 {
                    sim.world.set(CellPos::new(xx, yy), platypus_sim::Cell::AIR);
                }
            }
            crate::creatures::def::spawn_creature(&mut commands, "zombie", Vec2::new(x as f32, fl as f32 + 11.0), |_| {});
            *st = 5;
        }
        5 if t > 8.5 => {
            for px in [x - 34, x + 33] {
                for yy in fl..fl + 18 {
                    for xx in px - 1..px + 2 {
                        sim.world.set(CellPos::new(xx, yy), platypus_sim::Cell::new(stone, 120));
                    }
                }
            }
            for xx in x - 36..x + 36 {
                for yy in fl + 18..fl + 25 {
                    if (xx - x).abs() > 2 {
                        sim.world.set(CellPos::new(xx, yy), platypus_sim::Cell::new(wood, 120));
                    }
                }
            }
            sim.world.lift(CellPos::new(x - 20, fl + 21));
            sim.world.lift(CellPos::new(x + 20, fl + 21));
            *st = 6;
        }
        6 if t > 12.0 => {
            for (c, ch) in &them {
                if c.kind == "zombie" {
                    info!("crushcheck: the same log onto a zombie: health {:.0} of {:.0}", ch.hp, ch.max);
                }
            }
            *st = 7;
        }
        _ => {}
    }
}

/// `zombiejump` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a zombie
/// charges the player (who can't die), who jumps clean over it as it comes:
/// it skids on, gets its bearings, then comes back. Logs it each 0.2 s.
fn zombiejump_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, Has<crate::creatures::Bearings>), Without<LocalPlayer>>,
    mut state: Local<(u8, f32, u8)>,
) {
    if s.name != "zombiejump" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let Ok(mut pk) = player.single_mut() else { return };
    let (step, next, jumps) = &mut *state;
    if *step == 0 && t > 0.5 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        crate::creatures::def::spawn_creature(&mut commands, "zombie", pk.body.pos + Vec2::new(150.0, 4.0), |_| {});
        *step = 1;
        return;
    }
    let Some((_, _, zk, lost)) = them.iter().find(|(_, c, ..)| c.kind == "zombie") else { return };
    let gap = zk.body.pos.x - pk.body.pos.x;
    // (Over it as it charges in: a jump high and long, the way it came from.)
    if *jumps < 2 && zk.body.vel.x.abs() > 110.0 && gap.abs() < 26.0 && pk.loco.grounded() && zk.body.vel.x * gap < 0.0 {
        pk.body.vel = Vec2::new(gap.signum() * 150.0, 330.0);
        *jumps += 1;
        info!("zombiejump: t {t:.2} the player jumps over it ({gap:.0} off, it at {:.0} cells/s)", zk.body.vel.x);
    }
    if t >= *next {
        *next = t + 0.2;
        info!("zombiejump: t {t:.1} zombie {:.0} (the player {gap:+.0}) speed {:+.0} facing {:+.0}{}", zk.body.pos.x, zk.body.vel.x, zk.loco.facing, if lost { " GETTING ITS BEARINGS" } else { "" });
    }
}

/// `herbcut` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a sprig of
/// healroot on the floor in front of the player, who swings its sword at
/// it (1.5 s): logs what's left of it and the healroot it carries after.
fn herbcut_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    items: Option<Res<crate::hands::items::Items>>,
    (mut hand, mut cursor, mut mouse): (ResMut<crate::hands::Hand>, ResMut<CursorOverride>, ResMut<ButtonInput<MouseButton>>),
    mut player: Query<(&mut Kinematics, &mut crate::hands::items::Inventory), With<LocalPlayer>>,
    mut step: Local<u8>,
) {
    if s.name != "herbcut" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let (Some(items), Ok((mut k, mut inv))) = (items, player.single_mut()) else { return };
    let Some(herb) = sim.materials().id("healroot") else { return };
    let fl = platypus_worldgen::arena::FLOOR;
    let x0 = 960;
    let count = |sim: &SimWorld| (x0 - 4..x0 + 5).flat_map(|x| (fl..fl + 14).map(move |y| (x, y))).filter(|&(x, y)| sim.world.get(CellPos::new(x, y)).is_some_and(|c| c.material == herb)).count();
    let carried = |inv: &crate::hands::items::Inventory| inv.slots.iter().flatten().filter(|st| items.def(st.item).id == "healroot").map(|st| st.count).sum::<u32>();
    match *step {
        0 if t > 0.5 => {
            k.body.pos = Vec2::new(x0 as f32 - 14.0, fl as f32 + k.body.half.y);
            k.prev_pos = k.body.pos;
            if let Some(id) = items.id("copper_shortsword") {
                inv.slots[9] = Some(crate::hands::items::Stack::new(id, 1));
                (hand.bar, hand.slot) = (0, 9);
            }
            // (Room for it: the kit fills the pack.)
            let n = inv.slots.len();
            inv.slots[n - 1] = None;
            for (dx, tall) in [(-2, 4), (-1, 7), (0, 9), (1, 7), (2, 4)] {
                for dy in 0..tall {
                    sim.world.set(CellPos::new(x0 + dx, fl + dy), platypus_sim::Cell::new(herb, if dy == tall - 1 { 238 } else { 90 }));
                }
            }
            info!("herbcut: a sprig of {} cells; healroot carried {}", count(&sim), carried(&inv));
            *step = 1;
        }
        1 if t > 1.5 => {
            cursor.0 = Some(Vec2::new(x0 as f32, fl as f32 + 5.0));
            mouse.press(MouseButton::Left);
            *step = 2;
        }
        2 if t > 1.6 => {
            mouse.release(MouseButton::Left);
            *step = 3;
        }
        3 if t > 3.5 => {
            info!("herbcut: after the swing: {} cells left; healroot carried {}", count(&sim), carried(&inv));
            *step = 4;
        }
        _ => {}
    }
}

/// `wardrobe` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): the player in
/// each metal's armour in turn, its longsword in hand (copper, iron,
/// silver, gold, mithril; 1.5 s each from 1 s): logs its armour and health.
fn wardrobe_script(
    s: Res<Scenario>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    items: Option<Res<crate::hands::items::Items>>,
    mut hand: ResMut<crate::hands::Hand>,
    mut player: Query<(&crate::creatures::Health, &mut crate::gear::Equipment, &mut crate::hands::items::Inventory), With<LocalPlayer>>,
    mut shown: Local<Option<usize>>,
) {
    if s.name != "wardrobe" {
        return;
    }
    menu.open = false;
    let (Some(items), Ok((h, mut eq, mut inv))) = (items, player.single_mut()) else { return };
    let metals = ["copper", "iron", "silver", "gold", "mithril"];
    let i = ((s.elapsed - 1.0) / 1.5).floor();
    if i < 0.0 || i as usize >= metals.len() {
        return;
    }
    let i = i as usize;
    if *shown == Some(i) {
        if (s.elapsed - 1.0 - i as f32 * 1.5) > 1.0 {
            info!("wardrobe: {}: armour {:.0}, health {:.0}", metals[i], h.ward.armor, h.max);
            *shown = Some(i + 100);
        }
        return;
    }
    if shown.is_some_and(|k| k == i + 100) {
        return;
    }
    let m = metals[i];
    let mail = if m == "iron" { "chainmail".to_string() } else { format!("{m}_mail") };
    let long = if m == "iron" { "longsword".to_string() } else { format!("{m}_longsword") };
    for w in eq.worn.iter_mut() {
        *w = None;
    }
    for name in [format!("{m}_helm"), mail, format!("{m}_gauntlets"), format!("{m}_greaves"), format!("{m}_boots")] {
        let stack = crate::hands::items::Stack::new(items.id(&name).unwrap_or_else(|| panic!("no {name}")), 1);
        if let Some(k) = (0..eq.worn.len()).find(|&k| eq.worn[k].is_none() && crate::gear::Equipment::fits(&items, k, &stack)) {
            eq.worn[k] = Some(stack);
        }
    }
    inv.slots[9] = Some(crate::hands::items::Stack::new(items.id(&long).expect("longsword"), 1));
    (hand.bar, hand.slot) = (0, 9);
    *shown = Some(i);
}

/// `slimeswing` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): a green
/// slime (`PLATYPUS_SLIME_KIND`) hops at the player, who swings a
/// `PLATYPUS_WEAPON` (shortsword) at it as fast as it can once it's near
/// (aimed `PLATYPUS_AIM_UP` cells above its middle):
/// logs each blow that lands and what's left of it, and how many it took.
fn slimeswing_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    items: Option<Res<crate::hands::items::Items>>,
    (mut hand, mut cursor, mut mouse): (ResMut<crate::hands::Hand>, ResMut<CursorOverride>, ResMut<ButtonInput<MouseButton>>),
    mut hits: MessageReader<crate::combat::Hit>,
    mut player: Query<(&mut Kinematics, &mut crate::hands::items::Inventory), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics, &crate::creatures::Health, Option<&crate::creatures::body::goo::Goo>), Without<LocalPlayer>>,
    mut state: Local<(u8, usize, f32)>,
) {
    if s.name != "slimeswing" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let (Some(items), Ok((mut pk, mut inv))) = (items, player.single_mut()) else { return };
    let kind = std::env::var("PLATYPUS_SLIME_KIND").unwrap_or_else(|_| "slime".into());
    let (step, landed, last_hp) = &mut *state;
    if *step == 0 && t > 0.5 {
        for (e, c, ..) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        let w = std::env::var("PLATYPUS_WEAPON").unwrap_or_else(|_| "shortsword".into());
        if let Some(id) = items.id(&w) {
            inv.slots[9] = Some(crate::hands::items::Stack::new(id, 1));
            (hand.bar, hand.slot) = (0, 9);
        }
        crate::creatures::def::spawn_creature(&mut commands, &kind, pk.body.pos + Vec2::new(70.0, 10.0), |_| {});
        *step = 1;
        return;
    }
    let Some((e, _, sk, sh, goo)) = them.iter().find(|(_, c, ..)| c.kind == kind) else {
        if *step == 1 && t > 2.0 {
            info!("slimeswing: dead after {} blows, at {t:.1} s", *landed);
            *step = 2;
        }
        mouse.release(MouseButton::Left);
        return;
    };
    for h in hits.read().filter(|h| h.target == e) {
        *landed += 1;
        info!("slimeswing: t {t:.2} blow {} for {:.1} at {:.0},{:.0} (its middle {:.0},{:.0})", *landed, h.damage, h.at.x, h.at.y, sk.body.pos.x, sk.body.pos.y);
    }
    if (sh.hp - *last_hp).abs() > 0.5 {
        info!("slimeswing: t {t:.2} hp {:.1} of {:.0}, goo {}", sh.hp, sh.max, goo.map_or(0, |g| g.held()));
        *last_hp = sh.hp;
    }
    // (`PLATYPUS_AIM_UP`: aimed this far above its middle: a zombie's head.)
    let up: f32 = std::env::var("PLATYPUS_AIM_UP").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    cursor.0 = Some(sk.body.pos + Vec2::Y * up);
    let near = (sk.body.pos.x - pk.body.pos.x).abs() < 34.0;
    // (One that won't come (a mimic, a chest till you're on it): the player
    // goes to it.)
    if !near && t > 3.0 && *landed == 0 {
        pk.body.pos.x = sk.body.pos.x - 24.0;
        pk.prev_pos = pk.body.pos;
    }
    if near && (t * 10.0) as u32 % 4 == 0 { mouse.press(MouseButton::Left) } else { mouse.release(MouseButton::Left) }
}

/// `herbs`: the player taken to the nearest healroot under the start (the
/// generator's, as the seed made it), to see it in its cave.
fn herbs_script(s: Res<Scenario>, sim: Res<SimWorld>, mut menu: ResMut<crate::devmenu::DevMenu>, mut player: Query<&mut Kinematics, With<LocalPlayer>>, mut done: Local<bool>) {
    if s.name != "herbs" || *done || s.elapsed < 1.0 {
        return;
    }
    menu.open = false;
    let Ok(mut k) = player.single_mut() else { return };
    let Some(herb) = sim.materials().id("healroot") else { return };
    let start = sim.generator.spawn_point().chunk();
    let mut best: Option<CellPos> = None;
    for r in 0..12 {
        for cx in -r..=r {
            for cy in [-r, r] {
                let pos = platypus_sim::ChunkPos::new(start.x + cx, start.y + cy);
                let c = sim.generator.generate(pos);
                for y in 0..platypus_sim::CHUNK {
                    for x in 0..platypus_sim::CHUNK {
                        if best.is_none() && c.get(x as usize, y as usize).material == herb {
                            best = Some(CellPos::new(pos.origin().x + x, pos.origin().y + y));
                        }
                    }
                }
            }
        }
        if best.is_some() {
            break;
        }
    }
    *done = true;
    let Some(at) = best else {
        warn!("herbs: none near the start");
        return;
    };
    info!("herbs: healroot at {at:?}");
    k.body.pos = Vec2::new(at.x as f32 + 30.0, at.y as f32 + k.body.half.y + 2.0);
    k.prev_pos = k.body.pos;
    k.body.vel = Vec2::ZERO;
}

/// `brew`: each potion drunk on the flat arena, and what it does logged:
/// the health potion (60 back in a second), regeneration (slow, on top of
/// potion sickness), the antidote (venom washed off, poison not felt), the
/// fire ward (heat off a hot floor, before and after: half), and a raw
/// healroot eaten while still potion sick.
fn brew_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    items: Option<Res<crate::hands::items::Items>>,
    coatings: Res<crate::creatures::body::elements::Coatings>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut drink: MessageWriter<crate::potion::Drink>,
    mut player: Query<(Entity, &mut Kinematics, &mut crate::creatures::Health, &mut crate::hands::items::Inventory, Option<&crate::creatures::body::elements::Coated>), With<LocalPlayer>>,
    mut state: Local<(u8, f32)>,
) {
    if s.name != "brew" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let (Some(items), Ok((me, mut k, mut h, mut inv, coated))) = (items, player.single_mut()) else { return };
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let feet = CellPos::from_world(950.0, fl + 1.0);
    let mut quaff = |inv: &crate::hands::items::Inventory, id: &str| {
        let slot = inv.slots.iter().position(|s| s.is_some_and(|s| items.def(s.item).id == id));
        match slot {
            Some(slot) => drink.write(crate::potion::Drink { who: me, slot }),
            None => panic!("brew: no {id} carried"),
        };
    };
    // (The floor searing hot from 10 s.)
    if t > 10.0
        && let Some(stone) = sim.materials().id("stone")
    {
        for x in feet.x - 8..=feet.x + 8 {
            for y in feet.y - 3..feet.y {
                sim.world.set(CellPos::new(x, y), platypus_sim::Cell { heat: 260, ..platypus_sim::Cell::new(stone, 128) });
            }
        }
    }
    let (step, mark) = &mut *state;
    let due = [1.0, 1.5, 2.7, 3.0, 7.0, 7.0, 7.5, 9.5, 10.2, 12.2, 12.4, 14.4, 15.0, 15.2, 17.4, 17.5];
    let Some(&at) = due.get(*step as usize) else { return };
    if t < at {
        return;
    }
    let coats = coated.map(|c| c.coats.clone());
    match *step {
        0 => {
            commands.entity(me).remove::<crate::combat::Invulnerable>();
            k.body.pos = Vec2::new(950.0, fl + k.body.half.y);
            k.prev_pos = k.body.pos;
            // (Into the last slots, whatever was there: the kit fills the pack.)
            let n = inv.slots.len();
            for (i, id) in ["small_health_potion", "regeneration_potion", "antidote", "fire_ward_potion", "healroot"].into_iter().enumerate() {
                let item = items.id(id).unwrap_or_else(|| panic!("brew: no item {id}"));
                inv.slots[n - 1 - i] = Some(crate::hands::items::Stack::new(item, 2));
            }
            h.hp = 20.0;
        }
        1 => {
            *mark = h.hp;
            quaff(&inv, "small_health_potion");
        }
        2 => info!("brew: health potion: {:.0} -> {:.0} in 1.2 s", *mark, h.hp),
        3 => {
            *mark = h.hp;
            quaff(&inv, "regeneration_potion");
        }
        4 => info!("brew: regeneration: {:.1} -> {:.1} in 4 s", *mark, h.hp),
        5 => crate::creatures::body::elements::stain(&mut commands, me, "venom", 1.0, &coatings),
        6 => {
            info!("brew: venomed: {coats:?}, hp {:.1}", h.hp);
            *mark = h.hp;
            quaff(&inv, "antidote");
        }
        7 => info!("brew: antidote: {coats:?}, hp {:.1} -> {:.1} in 2 s (regen +3 of it)", *mark, h.hp),
        8 => *mark = h.hp,
        9 => info!("brew: heat unwarded: {:.1} -> {:.1} in 2 s (regen +3)", *mark, h.hp),
        10 => {
            quaff(&inv, "fire_ward_potion");
            *mark = h.hp;
        }
        11 => info!("brew: heat warded: {:.1} -> {:.1} in 2 s (regen +3)", *mark, h.hp),
        12 => info!("brew: ward {:?}", (h.ward.drunk_fire, h.ward.antidote)),
        13 => {
            // (Still potion sick from the health potion: eaten all the same.)
            h.hp = 50.0;
            *mark = h.hp;
            quaff(&inv, "healroot");
        }
        14 => info!("brew: healroot eaten while potion sick: {:.1} -> {:.1} in 2.2 s (regen +3.3)", *mark, h.hp),
        _ => info!("brew: done"),
    }
    *step += 1;
}

type SignsSubject<'a> = (
    Entity,
    &'a mut Kinematics,
    &'a mut crate::creatures::Health,
    Option<&'a crate::creatures::body::elements::Burning>,
    Option<&'a crate::creatures::body::elements::Coated>,
    Option<&'a crate::creatures::body::signs::Signs>,
);

/// `signs`: what hurts the player, shown on it (`creatures::body::signs`),
/// each a few seconds in turn on the flat arena: venom, acid, oil (lava
/// near: its warning), searing heat (the floor under it hot), wet in
/// the heat (steam), a wet landing (spray), wet under a storm (sparks). Its
/// health is kept up.
fn signs_script(
    mut commands: Commands,
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    coatings: Res<crate::creatures::body::elements::Coatings>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    time: Res<Time>,
    mut player: Query<SignsSubject, With<LocalPlayer>>,
    mut step: Local<u8>,
) {
    if s.name != "signs" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let Ok((me, mut k, mut h, burning, coated, signs)) = player.single_mut() else { return };
    if h.hp < h.max * 0.6 {
        h.hp = h.max * 0.6;
    }
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let at = Vec2::new(950.0, fl + k.body.half.y);
    let feet = CellPos::from_world(at.x, fl + 1.0);
    let paint = |sim: &mut SimWorld, name: &str, dx: i32, dy: i32, r: i32| {
        let material = if name == "air" { Some(platypus_sim::MaterialId::AIR) } else { sim.materials().id(name) };
        if let Some(material) = material {
            sim.queue(WorldEdit::Paint { center: CellPos::new(feet.x + dx, feet.y + dy), radius: r, material, overwrite: true });
        }
    };
    let coat = |commands: &mut Commands, name: &str| {
        commands.entity(me).remove::<crate::creatures::body::elements::Coated>();
        if !name.is_empty() {
            crate::creatures::body::elements::stain(commands, me, name, 1.0, &coatings);
        }
    };
    // (Each second: how it is.)
    if (t * 2.0).floor() != ((t - time.delta_secs()) * 2.0).floor() {
        let storm = sim.world.weather().map(|w| (w.rain_at(feet.x), w.severe_at(feet.x)));
        info!("signs: t {t:.1} hp {:.0} burning {:?} coats {:?} heat {:.2} storm {storm:?}", h.hp, burning.map(|b| b.share), coated.map(|c| c.coats.clone()), signs.map_or(0.0, |s| s.heat()));
    }
    // (From 9 s to 14.5 s the floor under it is searing hot: a hot plate.)
    if (9.0..14.5).contains(&t)
        && let Some(stone) = sim.materials().id("stone")
    {
        for x in feet.x - 8..=feet.x + 8 {
            for y in feet.y - 3..feet.y {
                sim.world.set(CellPos::new(x, y), platypus_sim::Cell { heat: 260, ..platypus_sim::Cell::new(stone, 128) });
            }
        }
    }
    let next = [1.0, 1.5, 4.0, 6.5, 9.0, 12.0, 14.5, 16.5, 20.0];
    let Some(&due) = next.get(*step as usize) else { return };
    if t < due {
        return;
    }
    info!("signs: step {} at {t:.1}", *step);
    match *step {
        0 => {
            commands.entity(me).remove::<crate::combat::Invulnerable>();
            k.body.pos = at;
            k.prev_pos = at;
            // (A pit in the floor off to the side.)
            paint(&mut sim, "air", 11, -4, 4);
        }
        1 => coat(&mut commands, "venom"),
        2 => coat(&mut commands, "acid"),
        3 => {
            coat(&mut commands, "oily");
            paint(&mut sim, "lava", 11, -5, 2);
        }
        4 => {
            coat(&mut commands, "");
            paint(&mut sim, "air", 11, -4, 4);
        }
        5 => coat(&mut commands, "wet"),
        6 => {
            coat(&mut commands, "wet");
            k.body.pos = at + Vec2::Y * 30.0;
            k.prev_pos = k.body.pos;
            k.body.vel = Vec2::new(0.0, -80.0);
        }
        7 => {
            coat(&mut commands, "wet");
            sim.queue(WorldEdit::Weather { x: at.x as i32, radius: 600, storm: true });
        }
        _ => info!("signs: done"),
    }
    *step += 1;
}

fn dryout_script(
    s: Res<Scenario>,
    mut sim: ResMut<SimWorld>,
    mut day: ResMut<crate::light::Daylight>,
    mut player: Query<&mut Kinematics, With<LocalPlayer>>,
    mut state: Local<(u8, Vec2, usize)>,
) {
    if s.name != "dryout" {
        return;
    }
    let days: f32 = std::env::var("PLATYPUS_DAYS").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0);
    let t = s.elapsed;
    let Ok(mut k) = player.single_mut() else { return };
    let water = sim.materials().id("water");
    let tally = |sim: &SimWorld, at: Vec2| {
        let mut n = 0;
        for y in (at.y as i32 - 120)..(at.y as i32 + 200) {
            for x in (at.x as i32 - 300)..(at.x as i32 + 300) {
                n += sim.world.get(CellPos::new(x, y)).is_some_and(|c| Some(c.material) == water) as usize;
            }
        }
        n
    };
    let put = |k: &mut Kinematics, at: Vec2| {
        k.body.pos = at;
        k.body.vel = Vec2::ZERO;
        k.prev_pos = at;
    };
    match state.0 {
        0 if t > 1.0 => {
            let p = k.body.pos;
            state.1 = p;
            let Some(w) = water else { return };
            state.2 = tally(&sim, p);
            for dx in [-160, -80, 80, 160] {
                sim.queue(WorldEdit::Paint { center: CellPos::from_world(p.x + dx as f32, p.y + 30.0), radius: 22, material: w, overwrite: false });
            }
            info!("dryout: before: {} cells of water about the start; a flood dumped", state.2);
            state.0 = 1;
        }
        // (Settled, and an hour on with the player there: the clock has
        // seen it.)
        1 if t > 12.0 => {
            info!("dryout: flooded: {} cells of water", tally(&sim, state.1));
            day.skipped += 1.0;
            state.0 = 2;
        }
        2 if t > 14.0 => {
            put(&mut k, state.1 + Vec2::new(6_000.0, 600.0));
            state.0 = 3;
        }
        3 if t > 16.0 => {
            day.skipped += 24.0 * days;
            state.0 = 4;
        }
        4 if t > 18.0 => {
            put(&mut k, state.1 + Vec2::new(0.0, 30.0));
            state.0 = 5;
        }
        5 if t > 22.0 => {
            info!("dryout: {days} days later: {} cells of water (the seed's own: {})", tally(&sim, state.1), state.2);
            state.0 = 6;
        }
        _ => {}
    }
}

/// `hearts` (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=flat`): the player held at
/// `PLATYPUS_HP` of its health (default 0.15; nothing lowers it, hearts
/// raise it), `PLATYPUS_KILLS` (default 30) rats put in a row beside it
/// (from x `PLATYPUS_RATS_AT`, default 990: the player's at 950) at 1.5 s
/// and struck dead one every `PLATYPUS_KILL_EVERY` s (default 0.25) from
/// 2 s. Logs each kill's hearts (red,
/// gold), what the player took, its health; at the end, the tally.
fn hearts_script(
    mut commands: Commands,
    s: Res<Scenario>,
    sim: Res<SimWorld>,
    mut menu: ResMut<crate::devmenu::DevMenu>,
    mut player: Query<(Entity, &mut Kinematics, &mut crate::creatures::Health), With<LocalPlayer>>,
    them: Query<(Entity, &crate::creatures::Creature, &Kinematics), Without<LocalPlayer>>,
    hearts: Query<(Entity, &crate::hearts::Heart)>,
    mut hits: MessageWriter<crate::combat::Hit>,
    mut state: Local<(u8, f32, usize, f32, usize, usize, f32)>,
    mut seen: Local<std::collections::HashSet<Entity>>,
) {
    if s.name != "hearts" {
        return;
    }
    menu.open = false;
    let t = s.elapsed;
    let Ok((me, mut k, mut h)) = player.single_mut() else { return };
    let share: f32 = std::env::var("PLATYPUS_HP").ok().and_then(|v| v.parse().ok()).unwrap_or(0.15);
    let kills: usize = std::env::var("PLATYPUS_KILLS").ok().and_then(|v| v.parse().ok()).unwrap_or(30);
    let fl = platypus_worldgen::arena::FLOOR as f32;
    let (step, next, killed, floor, red, gold, healed) = &mut *state;
    if *step == 0 && t > 0.5 {
        for (e, c, _) in &them {
            if c.kind == "dummy" || c.kind == "sandbag" {
                commands.entity(e).despawn();
            }
        }
        k.body.pos = Vec2::new(950.0, fl + k.body.half.y);
        k.prev_pos = k.body.pos;
        *step = 1;
        return;
    }
    if *step == 1 && t > 1.5 {
        let from: f32 = std::env::var("PLATYPUS_RATS_AT").ok().and_then(|v| v.parse().ok()).unwrap_or(990.0);
        for i in 0..kills {
            crate::creatures::def::spawn_creature(&mut commands, "rat", Vec2::new(from + i as f32 * 9.0, fl + 2.0), |_| {});
        }
        // (Its grace off first: it gives back what's lost.)
        commands.entity(me).remove::<crate::combat::Invulnerable>();
        *floor = h.max * share;
        h.hp = *floor;
        *step = 2;
        *next = 2.0;
        info!("hearts: the player held at {:.0} of {:.0}; {kills} rats beside it", h.hp, h.max);
    }
    if *step < 2 {
        return;
    }
    // (Nothing lowers it: only hearts raise it.)
    if h.hp < *floor {
        h.hp = *floor;
    }
    if h.hp > *floor + 0.01 {
        info!("hearts: t {t:.2} health up {:.1} -> {:.1}", *floor, h.hp);
        *healed += h.hp - *floor;
        *floor = h.hp;
    }
    let _ = &sim;
    let every: f32 = std::env::var("PLATYPUS_KILL_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(0.25);
    if t >= *next && *killed < kills {
        *next = t + every;
        if let Some((e, _, rk)) = them.iter().filter(|x| x.1.kind == "rat").min_by(|a, b| a.2.body.pos.x.total_cmp(&b.2.body.pos.x)) {
            hits.write(crate::combat::Hit { by: None, target: e, damage: 999.0, harm: crate::creatures::Harm::Slash, knock: Vec2::ZERO, stun: 0.0, at: rk.body.pos, dir: Vec2::X, weight: 1.0, crit: false });
            *killed += 1;
        }
    }
    // (Each heart once: counted the frame it's first seen.)
    for (e, heart) in &hearts {
        if seen.insert(e) {
            if heart.gold { *gold += 1 } else { *red += 1 }
            info!("hearts: t {t:.2} a {} heart (kill {killed}; {:.0} health)", if heart.gold { "gold" } else { "red" }, h.hp, killed = *killed);
        }
    }
    if *killed == kills && t > *next + 3.0 && *step == 2 {
        *step = 3;
        info!("hearts: {kills} kills held at {:.0} % health: {} red, {} gold hearts; healed {:.0} in all (the player now {:.0} of {:.0})", share * 100.0, *red, *gold, *healed, h.hp, h.max);
    }
}
