# Project Platypus — SPEC

Terraria's openness (dig, build, craft), Noita's world (every material is a
simulated cell), Hollow Knight's body (tight movement, readable melee combat).
Co-op from the start.

This file is the contract. Code that disagrees with it is a bug in one of the
two; fix whichever is wrong, in the same change.

---

## 1. Units and coordinates — one convention, everywhere

- **1 world unit = 1 cell.** Transforms, physics, AI, worldgen all speak cells.
  Zoom is purely a camera concern (default: 3 screen px per cell).
- **y points up**, matching Bevy. Cell `(x, y)` covers `[x, x+1) × [y, y+1)`.
  There is no "row 0 = top" anywhere outside the texture upload.
- A world position maps to its cell with `floor`, never `round`. The only
  functions that convert are in `platypus_sim::coords`. (Legacy had rendering
  centred on `x*T` while collision used `floor(x/T)` — a half-tile offset.
  One conversion function makes that class of bug impossible.)
- Chunks are `CHUNK = 64` cells square. `ChunkPos = floor_div(cell, 64)`.

## 2. Crates

```
crates/
  sim/       platypus_sim     cells, materials, chunks, stepping, edits. NO Bevy.
  worldgen/  platypus_worldgen seeded generators: fn(seed, ChunkPos) -> cells. NO Bevy.
  physics/   platypus_physics bodies vs a solid-grid trait, pixel masks, sweeps. NO Bevy.
  game/      platypus         the Bevy app: plugins for rendering, input, actors, combat, UI.
  bench/     platypus_bench   headless scenarios with time budgets (exit != 0 on regression).
assets/data/                  RON content: materials, creatures, weapons, items. Hot-reloaded.
legacy/                       the April 2025 prototype, kept runnable for reference.
```

Rules:
- The simulation crates never depend on Bevy. They compile in seconds, are
  unit-tested headless, and can be benchmarked without a window.
- `game` is glue. A feature is a Bevy `Plugin` in its own module with a small
  public surface (components, messages, a `SystemSet`). Plugins talk through
  messages and shared components, not by reaching into each other's internals.

## 3. The cell world (`platypus_sim`)

### 3.1 Cell
10 bytes, `#[repr(C)]`, `Pod`:
`material: u16, heat: i16, clock: u8, shade: u8, life: u8, flags: u8, vx: i8, vy: i8`.
- `heat` is °C *relative to local ambient* (§3.6); 0 costs nothing.
- `life` is remaining life for gases/fire, mining damage for solids.
- `shade` is fixed at cell creation and travels with the cell (texture that
  moves with the sand).
- `clock` is the low byte of the tick the cell last moved (a cell updates at
  most once per tick; the byte wraps, so a match only means "look again next tick").
- `flags`: liquid flow direction, liquid rest budget, `LOOSE` (§3.7).

### 3.2 Materials are data
`assets/data/materials.ron` defines every material: name, `kind`
(`Empty | Static | Powder | Liquid | Gas | Fire`), density, colours, hardness,
dispersion, flammability, lifetime, what it burns/melts into, and pairwise
reactions. Adding a material is a data change. The sim reads a compact
hot table (`MatPhys`) built from the definitions.

"Every element falls" is realised as: every element is a simulated cell with
a behaviour class. `Static` cells (rock, dirt, brick) hold position until
something converts them (dug, burned, melted, blasted, unsupported). A world
where rock literally flows is not playable.

### 3.3 Stepping
- Fixed tick, 60 Hz. Integer-only rules.
- Only **dirty rects** are updated; a chunk whose rect is empty costs nothing.
  Cells that fail to move go to sleep; any change wakes a 1-cell border.
- **Parallel checkerboard:** 4 passes; in each, chunks of one parity are
  updated concurrently. A job may touch cells at most 32 cells (half a chunk)
  outside its chunk, so same-pass jobs are disjoint. Velocities are clamped
  so this always holds (debug builds assert it).
- **Deterministic by construction:** randomness comes from
  `rng(world_seed, tick, chunk)`, never from a thread-local RNG. Results are
  identical for any thread count. A test enforces this.
- Unloaded neighbours behave as solid walls.

### 3.4 Edits
Every gameplay change to cells (dig, place, explode, paint) is a `WorldEdit`
applied at a tick boundary. This is the seam for co-op, replays and undo.

### 3.5 Streaming and persistence
Chunks load around every player (co-op: the union). Missing chunks come from
the chunk store (previously modified, lz4-compressed) or from worldgen.
Modified chunks are written back to the store on unload.

### 3.6 Temperature
- Ambient comes from `Climate` (by height: colder up high, warmer deep down).
  Cells store heat relative to it, so the whole world at ambient is free.
- A warm cell *pulls* toward its neighbours' temperatures (scaled by the lower
  conductivity of the pair) and cools toward ambient, writing only itself.
  Air holds no heat; fire and gases carry it. Sources (lava, burning fire)
  always read as their fixed heat.
- Near balance a cell held warm by a neighbour stops updating (±2 °C band), and
  a neighbour is only seeded if it will stay warm. Together these make hot
  regions go to sleep; the `deep` bench scenario enforces it.
- Phase changes are data: `above`, `below`, `ignites_at`, `explodes`.
- Invariant: lava (1200 °C) must not melt stone (1400 °C), or lava grows without
  bound. Only hotter things melt rock.
- Explosions requested by burning explosives are merged per tick (at most a few,
  nearby requests folded together) and applied at the next tick boundary.

### 3.7 Loose fragments
After any destruction — edits (dig, mine, bomb, ignite) *and* the simulation
itself (burning, melting, acid) — nearby solid pieces that no longer rest on
ground become `LOOSE` and fall as rubble of their own material. Ground is
anything edge-connected to bedrock, to unloaded world, or to more than 3 000
solid cells; a piece hanging by a diagonal corner is not attached. When a piece
falls, everything touching it is re-checked, so hangers-on follow. Checks
triggered by the simulation are grouped in 16×16 tiles, a few per tick, and
share what they learned about ground within the tick. With rigid bodies, big
falling pieces will become bodies instead of rubble.

### 3.8 Fire
Solids, powders and liquids burn *in place* (`BURNING` flag): the cell keeps
its material and position, glows, heats and ignites neighbours (diagonals
included), puts flames and smoke into the air around it, and after
`burn_time` becomes `burns_into` (e.g. some wood leaves ash) or nothing.
Water puts it out. Gases flash into flame. Flammable things falling into flames
catch fire. Heat rises: fire catches upward at twice the rate, downward at half.
Being above `ignites_at` gives a per-tick chance (set by flammability) to
catch, certain only 250 °C above it — otherwise heat would carry every fire
across every meadow regardless of flammability.

**How far fire spreads is a material property, not a special rule.** The chance
a burning cell lights a neighbour before burning out comes from flammability ×
burn_time; above a tipping point (~50 %) fire sweeps everything, below it
fires die out. Grass is tuned near that point: one spark in a meadow burns
roughly half of it, sometimes fizzles, rarely takes everything (tested over 20
seeds). Burned cells never regrow, so every fire ends; firebreaks (bare patches,
rock, water), wind and later rain shape where. A lit wooden slab still burns
up on every seed; a tree takes a few seconds to catch and burns ~20 s.

### 3.9 Particles
Things in flight between cells live in the sim as a plain list (not
entities), step once per tick after the cells, and march one cell at a time so
nothing tunnels. Each carries a real `Cell` and a landing rule: `Settle`
(becomes its cell; solids land `LOOSE` — blast debris, blood, splashes),
`Vanish` (dust, sparks), `Ember` (ignites what it lands on if flammable).
They are deterministic (seeded), capped at 30 000, and die at the edge of the
loaded world. Sources: explosions (hot debris thrown up and out of the crater,
sparks), mining (dust), burning cells (embers — how fire jumps gaps), creature
deaths (blood). Rendered as one dynamic mesh.

### 3.10 Background layer, plants, wind
- Every chunk has a second grid behind the playfield (`Chunk::background`):
  cave walls, tree trunks, branches, leaves. Creatures pass in front of it and
  liquids ignore it. It is stored and checksummed with the playfield.
- Tools and edits reach the background where the playfield is empty (you chop
  a tree by mining its trunk); explosions hit both; fire spreads between the
  layers (burning background puts flames into the air in front of it).
- A background piece is held up where it rests against solid playfield (a
  trunk rooted in the ground, a wall behind rock). A detached piece drops into
  the playfield: wood as loose rubble (still burning if it was), leaves as a
  falling flurry. Rigid bodies will later make chopped trees topple instead.
- `Kind::Plant` (tall grass, leaves): doesn't block creatures, burns readily,
  is crushed by falling powder and flowing liquid, withers without support.
- Wind: seeded, smooth, computed without trig (bit-identical across
  platforms). It biases gas drift and flames, and pushes light particles;
  embers blowing through a canopy can light it.
- Plant sway is rendering only: plant pixels are drawn shifted by wind, a
  travelling wave, and springs that creatures excite as they move through
  (2×8-cell tiles, underdamped). Cells never move, so sway costs the
  simulation nothing and never keeps a region awake.

## 4. Rendering

- One texture + one sprite per loaded chunk (~100 entities on screen, not
  ~270 000). A chunk re-colours and re-uploads only when its cells changed.
- **Nothing proportional to the number of cells may run because the player
  moved.** Legacy recomputed FOV (~200k hash inserts) on every tile crossing,
  and tile crossings get more frequent as cells shrink: at 3 px, walking cost
  40 ms/frame against 13 ms standing still. Lighting and visibility are
  computed per chunk on a coarse grid when *the world* changes, and applied
  on the GPU.

## 5. Bodies — "anything that can move" (`platypus_physics`)

- One `Body` (position, velocity, half-extents, flags) and one
  `move_and_collide(grid, body, dt) -> Contacts` against any `SolidGrid`.
  Step-up, grounded, wall and ceiling contacts come from there. Player,
  enemies, dropped items and projectiles all use it.
- Brains write **intent** (`move_x`, `jump`, `dash`, `attack`); one locomotion
  system turns intent + `MovementStats` into velocity. The player's brain is
  the input device; an orc's brain is AI. Dash, knockback and coyote time
  therefore work for every creature.
- Particles (debris, blood, sparks) are plain arrays, not entities. When they
  come to rest they become cells, so blood pools and stains.

## 6. Combat

- Weapons: pivot, swing curve (angle over time), windup/active/recovery
  frames, damage, knockback, hit-stop, and a 1-bit pixel mask derived from
  the sprite's alpha.
- Hurtboxes: per-animation-frame pixel masks derived from sprite alpha.
- Hit test: the blade is swept between last and current angle in sub-steps
  (no tunnelling), rasterised, and tested mask-against-mask after an AABB
  broadphase. The same sweep interacts with cells (cut grass, splash liquid,
  clang off stone → recoil).
- Every hit becomes one `Hit` message; one system applies damage, i-frames,
  knockback, hit-stop and VFX.

## 7. Extensibility — adding things without touching the engine

| To add…              | You write…                                             |
|----------------------|--------------------------------------------------------|
| a material           | an entry in `materials.ron`                            |
| a reaction           | a line in the reactions list of `materials.ron`        |
| an enemy (existing AI)| a creature RON (sprites, stats, attacks, brain + params)|
| a new AI behaviour   | one module implementing a brain, registered by name    |
| a weapon             | a weapon RON + sprite                                  |
| an item / recipe     | RON entries                                            |

All RON under `assets/data/` hot-reloads while the game runs.

## 8. Co-op

Host-authoritative, deterministic-by-construction, checksum-and-repair:
- The host owns the world. Clients send inputs and `WorldEdit` requests.
- Because the cell sim is deterministic given (seed, edits, tick), clients
  simulate locally for smooth visuals. The host sends per-chunk checksums
  periodically; a mismatched chunk is re-sent whole (the same compressed
  format as the chunk store).
- Each player's own body is simulated by its owner (no input delay on
  movement); the host is authoritative for enemies and damage.
- Seams that must exist before networking is built: `WorldEdit`,
  deterministic stepping, `SimTick`, chunk checksum + serialisation,
  stable entity ids for networked actors.

## 9. Performance budgets (checked by `platypus_bench`)

Reference machine: Apple M2 Max. Target: 60 fps at 1080p, 3 px/cell,
on a mid-range machine, so the M2 Max budgets are set at roughly half of the frame.

| Scenario                                  | Budget per tick |
|-------------------------------------------|-----------------|
| settled world, 12×8 chunks loaded         | < 0.5 ms        |
| deep world with lava lakes, settled        | < 0.5 ms (and asleep) |
| avalanche: ~100k moving cells             | < 6 ms          |
| streaming a new column of chunks          | < 4 ms          |

## 10. Working rules

- Fixed timestep for all gameplay; no `1/60` literals, no per-frame random rolls.
- No `static mut`, no global state; resources and components only.
- Every bug fix to the sim comes with a headless test.
- Tunables live in RON, not `const`, once they are gameplay-facing.
