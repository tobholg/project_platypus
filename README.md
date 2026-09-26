# Project Platypus

Terraria's openness, Noita's world (every material is a simulated cell),
Hollow Knight's movement and combat. Co-op. Rust + Bevy 0.19.

`SPEC.md` is the contract. `PLAN.md` is where we are. `legacy/` is the April 2025
prototype, kept for reference.

## Run

```sh
cargo run -p platypus --release                    # the game
PLATYPUS_WORLD=flat cargo run -p platypus --release  # an empty sandbox box
PLATYPUS_WORLD=arena cargo run -p platypus --release # the arena: dummies, pause/step/slow motion (P . ,), boxes (Y), O spawns what's picked in its panel (a pack, or one kind)
PLATYPUS_WORLD=small cargo run -p platypus --release # the small world preset (8192 × 4096; default large, 32768 × 16384)
PLATYPUS_SEED=42 cargo run -p platypus --release     # another world
PLATYPUS_SPAWN_X=2600 cargo run -p platypus --release  # start elsewhere (e.g. the tundra; worldview lists the biomes)
PLATYPUS_SPAWN_Y=3200 cargo run -p platypus --release  # start on a cave floor that deep (3200 = the deep band: mithril, rubies)
PLATYPUS_SPAWN_X=15700 PLATYPUS_SPAWN_Y=12000 cargo run -p platypus --release  # inside a crypt (worldview lists where they are)
```

Look at a whole generated world, or a region of it at 1:1, without starting the game:

```sh
cargo run -p platypus_worldview --release -- --out world.png                  # all of it, ~2048 px wide
cargo run -p platypus_worldview --release -- --region 16000,12100,1200,500    # x,y,w,h in cells (y up)
cargo run -p platypus_worldview --release -- --seed 7 --preset small --scale 4
```

A strip down the left edge marks the vertical bands (sky, peaks, surface,
underground, caverns, deep, underworld); the dashed cyan line is sea level; a
strip along the top shows the biomes. It also prints the biomes, lakes and sky
islands with their positions, to point `--region` at.

The toolchain is pinned in `rust-toolchain.toml`; rustup fetches it on first build.

**Play:** A/D move (you always face the cursor; moving away from it you backpedal) · in water, hold Space to swim toward W/A/S/D · Space jump (hold for height, tap for a hop, again in the air
for up to three air jumps (each resets a fall), against a wall to wall-jump) · S (or ↓) drops through a wooden
platform · Shift dash (the dodge: a moment untouchable, costs stamina) · + and − zoom (the keys that type them, on any layout; the
keypad's too)

**Hands** (the default; items in `assets/data/items.ron`, icons in `icons.ron`; what you hold shows in your hand and moves: tools swing as they mine, wands point as they cast, a held torch burns — `held` in `weapons.ron`): 1–0 or the mouse wheel pick a hotbar slot, X the next of three hotbars ·
LMB use it (hold it with the cursor below you to dig straight down, beside you for a
tunnel: the smart cursor digs a hole you fit; Alt toggles it) · hold Ctrl for the
right tool for what's at the cursor (auto tool) · the key left of 1 (or F1) switches
to the dev tools and back · swords and a bow: the shortsword, longsword and bow start on hotbar 2 (X, then 7, 8 or
9; 60 arrows beside them). Hold LMB with the bow to draw, let go to loose. Swords: hold LMB to swing at the cursor through the combo, strike down in the air to bounce off
what you hit (weapons and moves in `assets/data/weapons.ron`) ·
Esc (or I) opens the inventory: drag stacks between slots (or click one up and
click it down), right-click takes half, Shift-click moves across, hover for what
it is and does, click outside to throw it out. Mining and building work in 4 × 4-cell blocks;
the outlined block is the one you'll hit (the first minable block on the line from
your hand to the cursor, so you dig the face you see) or fill (under the cursor,
or against whatever the line meets). What you mine drops out and drifts to you;
blocks count in cells, shown as whole blocks. You start with a copper pickaxe and
axe, torches, bombs, platforms, five wands (6–0: spark, fireball, acid arrow,
flame, storm) and glow sticks (in the pack).

**Magic:** hold LMB with a wand to cast at the cursor; casting costs mana (the
blue bar, refilling). A wand is a list of runes (`assets/data/runes.ron`,
hot-reloaded): a carrier (bolt, orb, arrow, flame stream, lightning), what it
carries (spark, blast, ignite, heat, frost, acid, water) and modifiers (heavy,
fire trail, quick, trigger); change a wand's runes in `items.ron`. The gravity
wand (hotbar 2) holds a well at the cursor while you hold the button: it lifts
what's loose and tears out rock, spins it in a ball, carries creatures it can
lift; swing it (whip it and some flies off), let go to drop it all. Swing the well and let go to throw what it holds (an orc thrown hard takes fall
damage when it lands; rock dropped on something hurts it). The force wand is a
telekinetic shout from you toward the cursor: left flings everything in the cone
away, right drags it in. Pushed at the ground it throws you up (hold it to hover),
at a wall it kicks you off it; pulling at a ceiling hauls you up. Held spells drain mana and it doesn't come back while you hold them.
The staffs are level II of both: bigger, stronger. Spells work
through the cell sim: a fireball's blast is a bomb's, smaller, and can hurt
you; the storm wand's lightning is the sky's, from the wand, and charges any
water it strikes (everyone in the pool is shocked, you too). Acid eats most
things softer than hard rock, not glass or gold. Fire into water is doused in a
burst of steam; fireballs skip across water if they come in low; frost freezes
water into ice you can stand on (the frost wand, hotbar 2); bolts fizzle out
underwater; blasts in water throw it up. Chests sit in cave pockets underground
(right-click to open, R takes everything, loot by depth in `assets/data/loot.ron`);
mine one to take it with you.

**Dev tools** (the key left of 1, or F1, switches to them and back; a panel on the
right has a button for every dev action, with its key; tuning in `assets/data/tools.ron`, hot-reloaded):

| Key | Tool | LMB |
|---|---|---|
| 1 | Pickaxe | mine the playfield (rock, dirt, logs on the ground; not standing trees); each cell takes damage until it reaches its `hardness` (dirt fast, stone slow, obsidian slower, bedrock never). Cracks show as darkening |
| 2 | Bomb | throw toward the cursor; bounces, explodes after the fuse: crater, hot debris, flung sand/water, fire, smoke, screen shake + flash, damage + knockback. Bombs don't set each other off (`chain_reaction` in `tools.ron`), so a string of them digs a shaft |
| 3 | Spawner | pour the selected material; Q/E cycle through every material; Shift also replaces solids |
| 4 | Igniter | set flammable things alight (wood, oil, coal, grass) |
| 5 | Eraser | delete instantly |
| 6 | Heat / Freeze | heat what's under the cursor; Shift freezes. Rock glows then melts to lava (1400 °C), sand turns to glass, water boils or freezes, wood and oil ignite, methane explodes |
| 7 | Glow stick | throw one; it glows green or blue for 90 s |
| 8 | Axe | cut the background: standing trees, burnt trunks, cave walls, wherever the playfield in front is empty. Chop a trunk to fell a tree |

The pickaxe only digs the playfield, the axe only the background; the eraser and
the heat tool reach both. RMB erases with any tool · the wheel (or `[` `]`) radius · + and − zoom · Tab free camera
(WASD flies, Shift faster) ·
dev actions (letters in dev mode, F-keys always, or the panel): H performance HUD
(F3) · J chunk borders + dirty rects (F4) · V thunderstorm here (F5) · B clear skies (F6) ·
N lightning at the cursor (F7) · M +3 hours (F8) · K lighting off (F9) · L light (a small beam, a big one, a torch in the off hand, none) · G plant a torch · O spawn a warband (a troll, three orcs, two archers; packs in `assets/data/packs.ron`, or pick one kind in the arena panel) · the HUD at the top shows health and
status timers; dying just refills health (`PLATYPUS_RESPAWN=1` to respawn at the start)

Things to try: pour water then lava on it (obsidian + steam) · oil on water, then
ignite · a bomb in a wooden structure (paint wood with Q/E) · mine under a sand
pocket · melt a hole in rock with the heat gun · freeze a lake, watch it thaw ·
find a greenish methane pocket underground and set it off (bomb or heat gun) ·
set a forest alight: fire climbs the trunks, embers blow downwind into the next
canopy, burning branches fall · chop through a trunk with the axe (or a bomb):
the tree topples, sheds its crown and lands as a log you can mine · drop a tree on an orc ·
light a tree at the base: it chars black and snaps · call lightning onto a tree (F7): it runs down the trunk and the whole tree goes up at once · burnt wood leaves charcoal; put charred wood out with water for more ·
lava, fire, steam and acid hurt; flames set creatures alight (they spread fire as they run; jump in water) · boil acid (heat gun) into a corrosive cloud that rains acid, then light the cloud ·
pour oil on a lake and light it · freeze the ground under an orc (Shift + heat gun) to slow it, or to put yourself out ·
watch the clouds: fronts drift with the wind and rain where they're heavy; rain puts out forest fires and fills puddles; boiled water rises and rains back down ·
run and dash through tall grass and watch it part and wobble back ·
the HUD shows the material and temperature under the cursor ·
edit `materials.ron` or `creatures/player.ron` while the game runs.

## Check your work

```sh
cargo test --workspace --release        # sim, physics, worldgen behaviour (headless, seconds)
cargo run -p platypus_bench --release   # sim budgets; exits non-zero on regression
PLATYPUS_SCENARIO=run cargo run -p platypus --release   # scripted in-game run, prints frame stats
PLATYPUS_ZOOM=6 ...                                      # start zoomed in
```

Scenarios: `idle`, `pan`, `avalanche`, `run`, `tools`, `tree`, `blast`, `fell`, `burn`, `acid`, `rain`, `swim`, `night`, `dusk`, `cave`, `flood` (a block of water collapsing in a dug hall), `strike` (lightning onto the nearest tree), `hands` (digs a shaft, builds, chops with auto tool, plants a torch), `chest` (places, opens, fills and breaks a chest), `chestfall` (digs out a chest's floor: it falls), `drop` (holds S on a platform), `magic` (every starter wand at orcs; `PLATYPUS_WORLD=flat` for a clear view), `shock` (lightning into a pool with orcs in it, flat world), `inventory` (opens it, hovers a wand, drags it to hotbar 3; moves the real mouse pointer), `well` (the gravity wand lifts the ground and two orcs, swings, drops it; flat world), `force` (pushes a sand pile and orcs away, then pulls; flat world), `splash` (spells into a pool and an oil pit; flat world), `critters` (a rabbit, a bird and a frog walked at: they flee), `arena` (`PLATYPUS_WORLD=arena`: wands at the dummies, an orc spawned, pause and three steps, quarter speed; logs ticks and readouts), `life` (arena world, try `PLATYPUS_HOUR=22`: fireflies, fish, bats; logs whether each stays where it lives), `crossing` (arena world: through a falling sand stream; out of a pit of water by jumping at the surface), `held` (arena world: the pickaxe into the floor, the torch, a wand casting, a bomb thrown, the axe; logs what's wielded), `warband` (arena world: O spawns the default pack; logs what came), `archery` (arena world: the bow at a dummy, into the floor, through lava, the arrows picked up, an orc archer), `fight` (arena world: the shortsword against an orc, then a troll; logs both sides), `melee` (arena world: both swords at a dummy, a pogo, a blast mid-dodge; logs hits, stamina and hp), `wands` (arena world: the spark wand into the floor and at a sandbag, the flame wand at dummies 60 and 120 cells off), `editor` (arena world, `PLATYPUS_EDIT_DIR` = a folder of sprite COPIES: paints a stroke with the real pointer, undoes it, logs both). Bench scenarios: `settled`, `deep`, `avalanche`, `streaming`. Add `PLATYPUS_OFFSCREEN=1` for screenshots that work with the screen locked (the camera draws into an image; no HUD). Add `PLATYPUS_SCREENSHOT=out.png` to
capture the window, `PLATYPUS_SCENARIO_SECS=10` to change the length, `PLATYPUS_NOLIGHT=1` to start with lighting off.
Scenarios run without vsync; `PLATYPUS_VSYNC=1` runs them with it, to see the
frame pacing a player gets. For stutter, build with `--features spikes`: every
frame over `PLATYPUS_SPIKES` ms (default 12) is logged with its heaviest Bevy
systems (render world included), and every sim tick over 4 ms with its phases
(edits, cells, broken, particles, bodies, weather).

## Add things (no engine changes)

Everything under `assets/data/` hot-reloads while the game runs.

- **A material:** add an entry to `assets/data/materials.ron` (kind, density, colours, …).
- **A reaction:** add a line to `reactions` in the same file.
- **An enemy with an existing AI:** copy `assets/data/creatures/orc.ron` (or `troll.ron`),
  change its art, size, stats, `weapon` (weapons.ron), `poise`/`heft` and brain params
  (`reach`, `combo`, `attack_every`). Spawn it by its file name (the arena panel lists it).
- **A weapon:** a sprite pointing right with a `grip` anchor, and an entry in
  `assets/data/weapons.ron` (damage, knockback, its combo of moves).
- **A sprite:** write `assets/art/<name>.ron` (palette characters, frames as text
  grids, clips; see `crates/art/src/lib.rs`), look at it with
  `cargo run -p platypus_art --release -- sheet assets/art/<name>.ron` (also `check`,
  `describe`, `render`, `import`), and draw a creature with it: `art: "<name>"` in its
  creature file. Editing it reloads the game's live creatures. Or draw it in the game:
  `PLATYPUS_WORLD=arena`, E opens the art editor (it saves the same text files). From a
  script: `platypus-art get|set|paint <file> <path> ...`, e.g.
  `platypus-art paint assets/art/dummy.ron frames.stand 6,0=x 7,0=x`. A pose layer tagged
  `front_arm` plus a `fans` entry gives the creature an arm that aims where it casts.
- **Ambient life:** a kind in `assets/data/life.ron` (where it lives: surface, shore, water, caves; when: day or night; how many). A critter is a creature file with the `critter` brain: `hovers` for fliers that never land, `swims` for fish, `light` to glow.
- **A new AI behaviour:** in `crates/game/src/actors/ai.rs` (or a new module), a component
  with its settings plus one system writing `Controls`:

  ```rust
  #[derive(Component, Deserialize, Default)]
  #[serde(default)]
  struct Hopper { interval: f32 }

  app.register_brain::<Hopper>("hopper")
     .add_systems(FixedUpdate, hop.in_set(TickSet::Intent));
  ```

  then any creature file can use `brain: (kind: "hopper", params: (interval: 1.5))`.

## Layout

```
crates/sim/       the cell world: materials, chunks, parallel deterministic stepping (no Bevy)
crates/worldgen/  seeded generation: a WorldPlan, then chunk = f(plan, position) (no Bevy)
crates/worldview/ platypus-worldview: renders a generated world to PNG
crates/physics/   bodies vs the grid + the shared movement controller (no Bevy)
crates/game/      Bevy app: world, render, camera, actors, tools, debug, scenario
crates/bench/     headless performance budgets
assets/data/      materials.ron, creatures/*.ron
```
