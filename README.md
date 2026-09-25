# Project Platypus

Terraria's openness, Noita's world (every material is a simulated cell),
Hollow Knight's movement and combat. Co-op. Rust + Bevy 0.19.

`SPEC.md` is the contract. `PLAN.md` is where we are. `legacy/` is the April 2025
prototype, kept for reference.

## Run

```sh
cargo run -p platypus --release                    # the game
PLATYPUS_WORLD=flat cargo run -p platypus --release  # an empty sandbox box
PLATYPUS_WORLD=small cargo run -p platypus --release # the small world preset (8192 × 4096; default large, 32768 × 16384)
PLATYPUS_SEED=42 cargo run -p platypus --release     # another world
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

**Play:** A/D move · Space jump (hold for height, tap for a hop, again in the air
for a double jump, against a wall to wall-jump) · Shift dash

**Hands** (the default; items in `assets/data/items.ron`): 1–0 pick a hotbar slot ·
LMB use it · hold Ctrl for the right tool for what's at the cursor (auto tool) ·
I opens the pack (click to pick up and put down a stack, Shift-click to move it
between the hotbar and the pack). Mining and building work in 4 × 4-cell blocks;
the outlined block is the one you'll hit (the first minable block on the line from
your hand to the cursor, so you dig the face you see) or fill (under the cursor,
or against whatever the line meets). What you mine drops out and drifts to you;
blocks count in cells, shown as whole blocks. You start with a copper pickaxe and
axe, torches, bombs and glow sticks.

**Dev tools** (F1 switches to them and back; tuning in `assets/data/tools.ron`, hot-reloaded):

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
the heat tool reach both. RMB erases with any tool · `[` `]` or Ctrl+wheel radius · wheel zoom · Tab free camera
(WASD flies, Shift faster) · O spawn an orc at the cursor · F3 performance HUD ·
F4 chunk borders + dirty rects · L flashlight · T carry a torch · G plant a torch · 7 throw glow sticks · F8 +3 hours · F9 lighting off · the HUD at the top shows health and status timers; dying just refills health (`PLATYPUS_RESPAWN=1` to respawn at the start) · F5 thunderstorm here · F6 clear skies · F7 lightning at the cursor

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

Scenarios: `idle`, `pan`, `avalanche`, `run`, `tools`, `tree`, `blast`, `fell`, `burn`, `acid`, `rain`, `swim`, `night`, `dusk`, `cave`, `flood` (a block of water collapsing in a dug hall), `strike` (lightning onto the nearest tree), `hands` (digs a shaft, builds, chops with auto tool, plants a torch). Bench scenarios: `settled`, `deep`, `avalanche`, `streaming`. Add `PLATYPUS_SCREENSHOT=out.png` to
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
- **An enemy with an existing AI:** copy `assets/data/creatures/orc.ron`, change sprite,
  size, stats, brain params. Spawn it by its file name.
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
