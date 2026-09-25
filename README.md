# Project Platypus

Terraria's openness, Noita's world (every material is a simulated cell),
Hollow Knight's movement and combat. Co-op. Rust + Bevy 0.19.

`SPEC.md` is the contract. `PLAN.md` is where we are. `legacy/` is the April 2025
prototype, kept for reference.

## Run

```sh
cargo run -p platypus --release                    # the game
PLATYPUS_WORLD=flat cargo run -p platypus --release  # an empty sandbox box
PLATYPUS_SEED=42 cargo run -p platypus --release     # another world
```

The toolchain is pinned in `rust-toolchain.toml`; rustup fetches it on first build.

**Play:** A/D move · Space jump (hold for height, tap for a hop, again in the air
for a double jump, against a wall to wall-jump) · Shift dash

**Dev tools** (tuning in `assets/data/tools.ron`, hot-reloaded):

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
light a tree at the base: it chars black and snaps · burnt wood leaves charcoal; put charred wood out with water for more ·
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

Scenarios: `idle`, `pan`, `avalanche`, `run`, `tools`, `tree`, `blast`, `fell`, `burn`, `acid`, `rain`, `swim`, `night`, `dusk`, `cave`, `flood` (a block of water collapsing in a dug hall). Bench scenarios: `settled`, `deep`, `avalanche`, `streaming`. Add `PLATYPUS_SCREENSHOT=out.png` to
capture the window, `PLATYPUS_SCENARIO_SECS=10` to change the length, `PLATYPUS_NOLIGHT=1` to start with lighting off.

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
crates/worldgen/  seeded generation, chunk = f(seed, position) (no Bevy)
crates/physics/   bodies vs the grid + the shared movement controller (no Bevy)
crates/game/      Bevy app: world, render, camera, actors, tools, debug, scenario
crates/bench/     headless performance budgets
assets/data/      materials.ron, creatures/*.ron
```
