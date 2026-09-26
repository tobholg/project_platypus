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
  worldgen/  platypus_worldgen seeded generators: WorldPlan, then fn(plan, ChunkPos) -> cells. NO Bevy.
  worldview/ platypus_worldview renders a generated world (or a region) to PNG.
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

Liquids fall, slide and flow sideways into air, up to `dispersion` cells a
tick, and look up to 128 cells along the surface for lower ground (moving
toward it costs nothing); aimless sloshing is budgeted (8 reversals) so
lakes sleep. A liquid is pushed by the column above it: a cell with its own
kind on top passes *through* its own kind (up to 48 cells) to the first open
cell, reaching `dispersion + depth above` cells, scaled down by viscosity. So
a block of water collapses like a dam break, bottom first (a 64-tall block
is 30 tall after 1 s, 14 after 4 s), instead of eroding from its face while
only the top trickles out. Surface cells beside an open step doze rather than
sleep (they wake 1 tick in 16 to re-check), so a pressure-flattened pool
never freezes into stairs. `viscosity` (0 water … 255) makes a liquid move on fewer ticks,
pour slower and give up sooner: measured in a basin, water settles flat in
~3 s, oil ~4.5 s, blood and acid ~6 s, lava ~20 s in mounds.

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
`Mine` names its layer: the pickaxe digs the playfield only, the axe the
background only (standing trees, cave walls), and only where the playfield
in front is open, so you can't axe through a rock wall.

### 3.4b World generation
Plan in DESIGN.md §3, built stage by stage on `world-arc`. A `WorldPlan` is
computed once from the seed (milliseconds): size, sea level, vertical bands,
the surface per column, the snow line, the climate, the forests. Every chunk
is then a pure function of the plan and its position, never of another chunk;
a test generates chunks in two orders on four threads and compares
checksums.

- Presets: `large` (32 768 × 16 384 cells, the game's default) and `small`
  (8 192 × 4 096, for looking and testing; `PLATYPUS_WORLD=small`).
- Sea level sits a quarter of the way down. Bands relative to it in the large
  world (others scale): sky above +2 500, peaks +800, surface −200, underground
  −2 500, caverns −7 000, deep −11 000, underworld below.
- Climate (`Climate`, in the sim): 15 °C at sea level; 1 °C colder per 60
  cells up (so a temperate peak is below freezing from about +900) until the
  sky band, where the air warms again (1 °C per 18 cells, back to 15 °C: the
  sky islands are mild, the summits the coldest place); warmer with depth, 85 °C
  over sea level at the bottom (cavern lakes stay liquid, the underworld is
  hot). A 128-entry table adds each biome's warmth across the world, blended
  at the borders; a chunk has one entry, so the hot path is a shift and a
  lookup. Snow, ice and bare rock follow from it; nothing is painted by biome.
- Biomes (stage 2) are laid out like Terraria's: oceans at both ends, a forest
  at the spawn, a tundra towards one edge and a jungle towards the other, a
  mountain range between the tundra and the spawn, a deep forest somewhere
  away from the spawn (each of those two takes a neighbouring region too, so
  they run 6–12 k cells), a desert, temperate forest, plains and swamp
  filling the rest, never the same twice in a row.
  - The mountain range: a long ridge (700–1 000 high) with a peak every
    900–1 400 cells (1 400–2 200 high) on it, cold (−6 °C); above the peaks
    band the ground rises ever more slowly rather than being cut flat.
    Lone massifs (three) stay out of the range and the deep forest.
  - The deep forest: denser, bigger trees toward its heart (to 1.45×), old
    elders at the heart with dark leaves that let little light through.
  - Tree species: broadleaf (temperate), conifer (below the 4 °C tree line,
    down to −18 °C: the taiga, the mountainsides; tiers of needles on a thin
    trunk line, snow along each tier's top where it freezes, from the white
    end of the needles' colour ramp), elder (the deep forest). Each sets warmth, land height, hills, tree density
  (`lushness` shifts the forest planner toward woods or meadow) and lake
  chance.
- Relief: rolling hills and cliff steps per biome; oceans shelve down from the
  beach to −380; about seven mountain massifs (large world), lopsided, each a
  broad shoulder under a concave peak plus sub-peaks, a wandering ridge line
  and 70-cell terraces, 900–2 400 cells high, joined into ranges where they
  meet, none near the spawn. Steep faces wander sideways (overhangs, ledges);
  crests and gentle slopes stay put. Soil thins with slope (bare rock on
  cliffs); snow lies where the ground is below 0 °C and not steep (deeper the
  colder, measured across the slope so steep faces get a crust; very cold it
  clings to steeper faces: below −8 °C slopes to 3, below −15 °C to 5), so
  ledges hold it and cold peaks are white.
- Water: local basins (rims looked for 1 200 cells either side) are filled to
  their lowest rim, levelled flat per lake and capped by the biome's depth, so
  every lake is held and asleep on load; three big bowls always hold one,
  other hollows by chance (swamps mostly, deserts an odd oasis); none in the
  notches between crags. Lakes where it freezes are ice. Caves keep 60 cells
  clear of a lake bed. Fewer caves and no liquid pockets above sea level.
- Sky islands (legacy port): a dozen, high in the sky band, never sharing a
  column; each rasterised once at plan time into a stamp (grass, dirt, stone,
  a walker-carved cave), so chunks stay pure; trees on them from a second
  forest plan.
- Caves are planned, not noise (`caves.rs`, v2): noise caves came in every
  width, so many were just too narrow for the player. Now, like Terraria's
  tile runners:
  - chambers: ragged ellipses, kept apart; small in the underground and
    inside mountains (half sizes 26–60 × 18–34: the first layer is easy),
    bigger in the caverns (45–140 × 28–80) and the deep (55–170 × 35–100);
    about 30 % hold a pool (water, some oil, lava in the deep), kept below
    where any tunnel comes in so it doesn't spill;
  - tunnels: each chamber to its nearest few, a spanning tree of those so
    every chamber connects (tested: 95 %+ in one network) plus more for
    loops; wandering lines 22–44 cells wide (the player is 15 tall); steep
    ones get alternating rock ledges every 30 cells to climb back up;
  - crevices: a fifth of the extra links (never the tree) are cracks 3–7
    wide, too thin to pass: throw a glow stick in;
  - mouths: ~40 tunnels down from dry land (away from the spawn) into the
    nearest chamber;
  - noise only roughens the walls; a chunk asks only the shapes binned to it.
    In the caverns every cave below the water table is flooded, so where
    tunnels meet the flooded chambers the water is already level.
  - Tests: tunnels fit the player (a 6 × 15 box along 160 sampled tunnels,
    ≤ 1 % blocked), the network connects without crevices, crevices are few.
    Planned in ~50 ms with the rest of the plan.
- Underground biomes (regions, two each in a large world, one in a small;
  everything else keeps its plain rock, and the underground layer stays
  plain and easy): elliptical areas ~1 000–1 500 × 450–750 half size in the
  caverns (toxic grottos in the deep too).
  - Fungal hollows, teal and violet: rock within two cells of open space
    becomes fungal soil; glowing sprouts on the floors, glowing vines (a
    hanging plant) from the ceilings; giant mushrooms in the chambers'
    background (background materials glow now too, where nothing's in
    front): parasols (a wide flat spotted cap, glowing violet gills under
    it, strands hanging from its rim, a leaning stem with a ring) and
    clusters of lantern stalks (a glowing bulb on a thin stalk); bracket
    fungi up the walls, left and right in turn a jump apart, are one-way
    platforms: a way up. Mushrooms stand on the floor they grow from (their
    feet found against the real, ragged floor; a lean that would take one
    into the wall stands it straight; parasols keep their caps apart), come
    down like trees when their stems are cut, take more heat than wood to
    catch (360–420 °C) and shrivel to ash at 450–500 °C.
  - Crystal caves: glowing crystals hang straight down from the chambers'
    ceilings (a quarter to half the way down), a few short ones stand on
    the floors (not where a tunnel comes in); crystal studs the walls.
  - Toxic grottos: every chamber holds acid (a pool below where tunnels come
    in); a glowing crust over the rock at the open space, and over anything
    touching the acid (ore, gravel): the crust is inert, so the pools stay.
  - Glows that live (rendering only, `MaterialDef`): `pulse` makes a glow
    breathe (each 16-cell patch on its own 3–6 s cycle, in the light grid);
    `shimmer` makes it glimmer (8-cell patches each slowly swelling from
    dark to bright and back every 5–12 s: crystals, gems, mithril);
    `motes` sheds glowing spores that drift up (the fungi; at most 160 at
    once, on screen only).
  - Nothing floats. Chambers' ragged edges are measured around their
    outline (open all the way from the middle to the edge), tunnels' walls
    wobble along their length; what grows from a wall (crystals, shelves,
    steep tunnels' ledges) and giant mushrooms (from their floor) carry a
    root cell and are drawn only if it's rock once every cave is carved. The
    caverns' big chambers hang stalactites from their real ceilings only
    (each column its own length, the ceiling's distance estimated from the
    noise, checked, and nothing carved in between), floors clear. And a
    solid piece lying wholly inside a chunk, touching no edge and nothing
    else, becomes what it floats in. Tested: in eight windows of 7 × 7
    chunks, at most two small pieces float (big masses of rock between caves
    may stand free).
  - The dressing is a pass over each generated chunk (with a two-cell margin
    from the chunks around, asked once); spikes and mushrooms are planned
    with the chambers. `platypus-worldview` lists the areas.
- Underground (stage 3), by band:
  - underground: the planned caves, sand and gravel pockets, coal;
  - caverns: huge chambers (wider than tall) with stalactites and pillars
    (vertically streaked noise), fading in over the band's top 300 cells;
    chambers below the regional water table (one per 2 048 columns) are
    flooded: underground lakes;
  - deep: slate (with obsidian seams), chambers, lava pools;
  - underworld: basalt; a vault with a ragged, dripping roof, basalt islands
    hanging in it, and a lava sea at one flat level (asleep on load, the
    bench's 80 000 lava cells settle in 122 ticks); obsidian crusts where
    rock meets the lava.
  - Chasms: about five shafts from the lowland surface (away from the spawn,
    lakes and mountains; no trees at their lips) down into the deep, 110–240
    wide, wandering ±250, narrowing and widening (ledges), funnel-shaped at
    the top: the long descent.
  - Rock and walls follow the band (stone, slate, basalt), dithered at the
    borders. Slate melts at 1 500 °C and basalt at 1 650 °C, above what a lava
    sea heats them to.
  - Cost: a caverns chunk takes ~2.6× a surface chunk to generate (8-chunk
    column 1.8 ms vs 0.7 ms, bench `stream_deep`).
- Not yet: waterfalls, jungle and swamp materials (mud, vines), creatures of
  the underground biomes, spore gas, more biomes.

### 3.4c Hands: mining, building, items, chests
DESIGN.md §4–5, stage 4 of the world arc.

- Blocks: 4 × 4 cells on a fixed grid (`BLOCK`); the world stays cells.
  `WorldEdit::MineBlock` damages a block's minable cells (the playfield, or
  with `back` the background where the front is open); at the hardest one's
  hardness they all break at once, and cells harder than the tool's tier stay.
  Damage lives in the cells' `life`, so the renderer's crack darkening shows
  it. `PlaceBlock` fills a block's room (air, tall grass, smoke, flames);
  `Stamp` fills a box anchored at its corner (furniture); `Remove` takes one
  material out of a rectangle.
- Patterns: a material may carry a world-anchored pattern of shades (hex
  rows); placed and generated cells take their shade from it, so `brick` and
  `planks` tile.
- Items (game, `hands/`): `items.ron` for made things plus a block item per
  solid or powder material (counted in cells, shown in whole blocks: nothing
  lost to rounding). `Inventory` is a component any creature can carry; the
  player's is 60 slots: three hotbars of 10 (1–0 picks a slot, X the next
  hotbar), then three rows of pack. Mined blocks drop as items that drift to
  a player with room (48 cells) and are picked up (6).
- The inventory screen (Esc or I, `hands/ui.rs`): the three hotbars
  (numbered; click one to use it) and the pack, Terraria-style: drag a stack
  to a slot, or click it up and click it down (merging the same item,
  otherwise swapping); right-click takes half; Shift-click moves it across
  (hotbars and pack, or pack and an open chest); clicking outside the panels
  with a stack in hand throws it out. Hovering a slot shows a tooltip (what
  it is, what it does: a pickaxe's hardness and speed, a wand's runes and
  mana, a note from `about`). Slots are hit-tested by cursor position, not
  `Interaction` (a drag's first slot stays `Pressed`).
- Icons (`icons.ron`, hot-reloaded): 16 × 16 text art, a shape in palette
  letters plus a palette per item (so tiers and elements share one drawing);
  blocks get a little block of their material; anything else its colour.
- Smart cursor (on by default, Alt toggles): aimed mostly down, up or
  sideways, a pickaxe clears a tunnel the body fits, nearest first: hold the
  button with the cursor below and the whole row under your feet goes before
  the next, so you drop and keep digging; sideways, a face as tall as you.
  Aimed diagonally (and for the axe) it hits the first minable block on the
  line from the hand toward the cursor (the face you see). Plain cursor: the
  block under it. A placement goes
  under the cursor if free and supported (a solid neighbour or a wall behind),
  else against whatever the line meets, never into open air. Pure functions
  with unit tests. Ctrl picks the best tool for the target (auto tool). The
  target is outlined.
- Pace: a copper pickaxe (power 35, 6 hits/s) takes dirt in one hit, stone in
  two; an iron one (power 60, 7/s) stone in one. The player's box is 6 × 15 cells, so it drops into a 2-block
  shaft and walks a 4-block tunnel.
- Chests are furniture: entities, not cells (`hands/chests.rs`), 12 × 10
  cells (`worldgen::CHEST_SIZE`), drawn from a text picture. A chest is a body
  (the same falling and collision as a dropped item): it falls when its floor
  goes, blasts throw it (and hurt it: 60 hit points, a bomb beside it breaks
  it), fire, lava and acid wear it down; breaking spills what's in it.
  Mining one takes its hit points off per hit (two hits of a copper
  pickaxe) and the last spills its contents and the chest.
- Worldgen doesn't draw chests: generating a chunk also reports what it
  starts with besides cells (`ChunkGenerator::generate_with_spawns`:
  `Spawn::Chest` or `Spawn::Creature` at their feet): a cave chest on the
  first cave floor with room for one in some underground chunks (~one in 30),
  a structure's chests and guards. The game makes each once (remembered by
  place: an unmodified chunk is generated again when it comes back).
- Contents live in the `Chests` resource under a key that stays when the
  chest moves: a world chest's is from where it was made, and its loot is
  rolled from `loot.ron` (tables by depth) from the seed and that place the
  first time it's opened; a placed chest gets a fresh key and starts empty.
  Right-click opens one within reach (Shift-click moves stacks, R takes all).
- Scenarios: `chest` (place, open, fill, mine: the contents and the chest
  come back), `chestfall` (the ground dug out under a chest: it falls).
- Play mode is the default; the key left of 1 (backquote; F1 needs fn on a
  Mac) switches to the dev tools and back. `PLATYPUS_SPAWN_X` starts
  elsewhere, to try a biome; `PLATYPUS_SPAWN_Y` on the nearest cave floor at
  or below a height, to look at the deep bands.

### 3.4d Ores and gems
DESIGN.md §3.2 step 6, stage 5 of the world arc (`worldgen/src/minerals.rs`).
- Ores are rock materials, harder the deeper they lie, and the pickaxe tiers
  climb with them (items.ron):

  | Ore | Hardness | Lies | First pickaxe |
  |---|---|---|---|
  | coal (powder, burns) | 10 | mountains to mid-caverns, flat seams | any |
  | copper | 50 | mountains down through the underground, veins | copper (70) |
  | iron | 65 | lower underground, upper caverns, veins | copper |
  | silver | 75 | the caverns, blobs | iron (100) |
  | gold | 85 | lower caverns, upper deep, small blobs | iron |
  | mithril | 110 | the deep, long rare seams, faint teal glow | gold (115) |

  Obsidian (120) takes the mithril pickaxe (150). The better pickaxes come
  from chests for now (deeper tables), until crafting.
- Each ore has a depth window with a fade at its ends (the threshold rises
  over 15% of it, at most 400 cells), and noise stretched along the strata
  (veins) or round (blobs). Where a cave is within 5 cells the threshold is
  0.1 lower, so ore shows on cave walls: about 5% of the rock is ore, 9% of
  the rock at cave walls.
- Gems grow only in cave walls (within 4 cells of open space), in clusters,
  and only along some stretches of wall (a coarse noise), so they are a
  find, not a lining: amethyst in the underground (60), emerald in the
  caverns (80), ruby in the deep (100). They glow faintly, which lights the
  caves around them.
- Cost: a deep column of chunks streams in ~2.2 ms (was 1.8 ms), under the
  4 ms budget.

### 3.4e Structures: crypts and castles
DESIGN.md §3.3, stage 6 of the world arc (`worldgen/src/structures.rs`,
rooms in `assets/data/rooms/*.rooms`).
- A room is a text grid at block resolution (one character per 4 × 4
  cells, on the mining grid), 16 × 10 blocks per slot; the legend is at the
  top of `crypt.rooms` (wall, open, open to the sky, keep the terrain,
  chest, candle, creature, boss, water, lava, spikes, illusory wall, weak
  wall, planks, rubble). Kinds: room, entrance, goal, secret, ruin.
- Doors are fixed places on a slot's edge (a side door is rows 1–5 from the
  bottom, a hole in the top or bottom is columns 6–9), read from the grid,
  so a room's doors are never declared twice. Every room has doors all
  round; assembly walls up the ones it doesn't use and makes a secret room's
  door an illusory wall. A room's edge must be wall outside its doors
  (checked when parsed).
- A crypt: a ruin on the surface (flat, dry lowland, away from the spawn,
  the chasms and other crypts; no tree grows in it), a shaft down 30–70
  blocks (walled, with ledges to climb back up and candles), then a grid of
  3–6 × 3–5 slots: a path from the entrance under the shaft, across and
  down to a goal room (two slots wide if there's room: chests and a boss),
  side rooms off it, about a third of them secrets behind an illusory wall
  (at least one when there's room). About 10 on a large world (3–8 by seed).
- Laid out once, when the world is planned; a chunk rasterises the pieces
  it touches (binned by chunk). Structures win over the terrain (and over
  caves and ore) wherever they aren't "keep the terrain".
- Crypt stone (hardness 90) needs an iron pickaxe, so the way through is
  the rooms; cracked stone breaks in one hit; an illusory wall is a still
  plant drawn like crypt stone: creatures walk through it. Spikes hurt on
  touch (corrosive 40). Candles glow faintly: crypts are dark.
- Guards: the generator reports a fresh chunk's creatures (`spawns`), the
  game spawns each once (remembered by place, since an unmodified chunk is
  generated again when it comes back into view). A boss is a pack of three
  orcs until there are bosses.
- Tests: every chest in 300 random crypts is reachable from the ruin
  through open blocks (illusory walls included); crypts sit on the ground
  with no tree in the ruin; chests are drawn whole across chunk borders;
  each guard is reported by exactly one chunk.
- A castle (`castle.rooms`, ashlar: hardness 100): on the mountains, where
  it's high but not too steep (height less three times the ground's fall
  under it, at most 300 cells), about 3 on a large world (2 by seed,
  spaced). A keep 2–3 slots wide and 2–3 tall between two towers a slot or
  two taller, every slot a room (no holes). The same layout walk, climbing:
  the gate at the foot of the tower on the side where the ground outside is
  nearest the floor, the goal at the top. Battlements over every column;
  foundations of wall from the floor down into the ground; to the gate a
  flying stair down the mountainside (three blocks thick, piers every eight)
  or, where the ground rises, a tunnel, either up to 60 blocks. Windows are
  holes in the back wall: the sky shows through.
- Tests also: every chest in 200 random castles is reachable from the gate,
  on falling and rising ground. Reachability moves a player-sized body (2 × 4
  blocks) and a chest counts when it's within mining reach, so a gap the
  player barely can't pass fails the test; rooms also must keep the way
  through a door clear (nothing solid within two blocks inside a door, and
  nothing under a hole in the ceiling all the way down), checked when parsed.
- Wooden platforms (`-` in rooms; the `platform` material, `platform: true`):
  one-way for creatures (the physics' `Occupancy::Platform`): a body landing
  from above stands on one, from below and the sides it passes through, and
  holding down (S or ↓, `Intent::down`) drops through. To the cell sim it's
  wood. Rooms' ledges and the shaft's are platforms; a placed platform block
  fills its block's top half. A new player carries 40.
- Age, from the place alone (a block's hash, a cell's): a crypt's floors
  grow moss in patches, some of its ceiling blocks have fallen in (gravel:
  it drops into a pile once the chunk is live), and the top corners of
  rooms (crypts and castles) gather cobwebs, a ragged triangle up to seven
  cells out. Moss and cobwebs are still plants; cobwebs `hang`: held from
  above by ground or the web they hang from (and only by that, so a web
  adrift in the air comes apart), and they burn in a flash.
- Secrets: illusory walls hide side rooms (above); some rooms have a niche
  sealed by a weak wall with a chest in it (`sealed_tomb`, `storeroom`);
  every lake 40+ cells deep keeps a chest at its deepest, on the bed. Chests
  above the underground (castles, lakes) roll the `high` loot table (ore,
  gems, the better pickaxes).
- Not yet: keys and locked doors (with the RPG arc), rooms behind
  waterfalls (no waterfalls yet), bosses (a pack of orcs for now).

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
  `StepStats::detonated` reports the ones applied, so the game can shake the
  camera and flash for sim-caused blasts the same as for its own bombs.
- A blast shatters everything breakable in its radius (some flies as hot
  debris), crumbles a rim around it, and flings loose material (powders,
  liquids, rubble) a few cells further out as particles that land again.

### 3.7 Loose fragments
After any destruction — edits (dig, mine, bomb, ignite) *and* the simulation
itself (burning, melting, acid) — nearby solid pieces that no longer rest on
ground become `LOOSE` and fall as rubble of their own material. Ground is
anything edge-connected to bedrock, to unloaded world, or to more than 3 000
solid cells; a piece hanging by a diagonal corner is not attached. When a piece
falls, everything touching it is re-checked, so hangers-on follow. Checks
triggered by the simulation are grouped in 16×16 tiles, a few per tick, and
share what they learned about ground within the tick. (Background pieces with
64+ wood cells fall as rigid bodies instead, §3.11.)

**What carries weight is one rule, `MaterialTable::bears_load`, used by every
ground check.** Today: a burning solid past its `chars_at` share of its burn
(default half) is charred and carries nothing; it notifies the ground check
when it crosses that line, so a trunk burning at the base snaps while most of
it is still there (about twice as soon as waiting for it to burn through).
Other weakening (mining cracks, metal softening near its melting point) belongs
in the same rule.

### 3.8 Fire
Solids, powders and liquids burn *in place* (`BURNING` flag): the cell keeps
its material and position, glows, heats and ignites neighbours (diagonals
included), puts flames and smoke into the air around it, and after
`burn_time` becomes `burns_into` (a quarter of wood leaves charcoal) or nothing;
burned-out background drops a third as much into the playfield. Water puts it
out; a charred cell put out becomes `chars_into` (wood: charcoal). Charcoal has
flammability 0 (flames don't catch on it) and lights only above 700 °C, hotter
than burning wood, so it survives the fire that made it. Gases flash into flame. Flammable things falling into flames
catch fire. Heat rises: fire catches upward at twice the rate, downward at half.
Being above `ignites_at` gives a per-tick chance (set by flammability) to
catch, certain only 250 °C above it — otherwise heat would carry every fire
across every meadow regardless of flammability.

**The background (standing trees) burns by heat.** Background cells hold
heat (relative to ambient) but don't conduct it; a hot one cools by 1/32 a
tick. A burning cell heats itself (+3 a tick) and radiates into its eight
neighbours (1 + flammability/3: wood 3, leaves 7; twice upward, half
downward; half when charred). A cell catches when its temperature passes its
ignition point, by the playfield's ramp. A flame below 250 °C over ambient
may go out (`fizzles`), so a lone spark dies, a fire big enough to heat
itself and what's above it climbs, and a front spreads sideways. Flames in the
playfield, the heat gun, lava and lightning add heat; water in front and rain
put it out and cool it. Measured on generated trees: a one-cell spark always
goes out within seconds; a small fire takes 40-90 % of a tree. The hotter it
burns the faster it eats its fuel: above 1100 °C twice as fast.

**Reactions keep heat.** What a reaction makes keeps the heat of what went
into it (unless it pins its own): lava quenched by water is glowing-hot
obsidian that boils off the water landing on it next, so water on lava makes
part crust, part steam.

**Phase changes take time (latent heat).** Past a material's `above`/`below`
threshold by d °C it changes with chance (d / `latent`)² a tick: ice in a
15 °C room melts over ~20 s, snow ~7 s, water at -5 °C freezes over ~30 s,
and a heat gun or lava does it at once.

**In the playfield, wood is hard to get going; grass and leaves aren't.** Per material:
`spread` (chance /4096 per tick to catch from a burning neighbour, ×2 from
below, ÷2 from above; default flammability × 16), `fizzles` (chance /4096 per
tick that a flame with fewer than two flaming neighbours goes out) and
`chars_at` (after that share of its burn it smoulders: no flames, no
spreading, no embers, and it stops carrying weight). Only a blaze (4+
flaming neighbours) throws embers, and flames licking into the air grow with
the fire. Heat alone lights a material with a chance that ramps up over the
first 60 °C above its ignition point. So a spark on a trunk sometimes goes
out, usually scorches it and burns the crown, leaving the trunk standing; a
proper fire burns most of a wooden slab (tested over 20 seeds each). Rain
puts fires out.

**How far fire spreads is a material property, not a special rule.** The chance
a burning cell lights a neighbour before burning out comes from flammability ×
burn_time; above a tipping point (~50 %) fire sweeps everything, below it
fires die out. Grass is tuned near that point: one spark in a meadow burns
roughly half of it, sometimes fizzles, rarely takes everything (tested over 40
seeds). Burned cells never regrow, so every fire ends; firebreaks (bare patches,
rock, water), wind and later rain shape where. A lit wooden slab still burns
up on every seed; a tree takes a few seconds to catch and burns ~20 s.

### 3.9 Particles
Things in flight between cells live in the sim as a plain list (not
entities), step once per tick after the cells, and march one cell at a time so
nothing tunnels. Each carries a real `Cell` and a landing rule: `Settle`
(becomes its cell; solids land `LOOSE` — blast debris, blood, splashes),
`Vanish` (dust, sparks), `Ember` (may ignite what it lands on or brushes).
They are deterministic (seeded), capped at 30 000, and die at the edge of the
loaded world. Sources: explosions (hot debris thrown up and out of the crater,
sparks), mining (dust), burning cells (embers — how fire jumps gaps), creature
deaths (blood). Rendered as one dynamic mesh.

### 3.10 Background layer, plants, wind
- Every chunk has a second grid behind the playfield (`Chunk::background`):
  cave walls, tree trunks, branches, leaves. Creatures pass in front of it and
  liquids ignore it. It is stored and checksummed with the playfield.
- Tools and edits reach the background where the playfield is empty (you chop
  a tree by mining its trunk); explosions blow away the background's trees,
  plants and wooden walls but not stone or earth walls (those come off with a
  tool, so a blasted tunnel keeps its back wall); fire spreads between the
  layers (burning background puts flames into the air in front of it).
- Where the background is empty more than 16 cells below the generated
  surface, the renderer draws a dark rock backdrop (earthy near the top,
  colder with depth, faint strata) instead of letting the sky show through.
  A stopgap until the parallax far background.
- A background piece is held up where it rests against solid playfield (a
  trunk rooted in the ground, a wall behind rock). Only growths
  (`grows: true`: wood, mushroom stems) carry anything and come down;
  background that doesn't grow (the rock behind a cave) neither falls nor
  holds a growth up, so a mushroom in a cave is held by its foot alone. Only wood (anything not a
  plant) carries weight: leaves hang on wood within `LEAF_REACH` (72 cells,
  through leaves), so a felled tree is never held up by its neighbour's
  crown. Worldgen keeps every leaf inside that reach and every background
  cell edge-connected (tested by felling generated trees).
- A detached piece with at least 64 wood cells comes away whole as a rigid
  body (§3.11), taking the leaves nearer its wood than any other wood (a
  shared canopy splits down the middle); leaves left with no wood in reach
  fall as a flurry. Smaller pieces drop into the playfield: wood as loose
  rubble (still burning if it was), leaves as a flurry.
- `Kind::Plant` (tall grass, leaves): doesn't block creatures, burns readily,
  is crushed by falling powder and flowing liquid, withers without support.
- Wind: seeded, smooth, computed without trig (bit-identical across
  platforms). It biases gas drift and flames, and pushes light particles;
  embers blowing through a canopy can light it. An ember cools as it flies:
  it lights what it touches with chance 1/4 while fresh, falling off over its
  last 80 ticks, and goes out in flight 1 tick in 60. In a canopy it touches
  one leaf (more often the more flammable) and is spent, lit or not.
  Measured over 8 seeds: a burning crown sets the next tree 28 cells off
  alight 2 times, one 108 cells off never (4 and 2 when the first leaf an
  ember brushed caught).
- Grass sway is rendering only: grass pixels are drawn shifted by wind, a
  travelling wave, and springs that creatures excite as they move through
  (2×8-cell tiles, underdamped). Cells never move, so sway costs the
  simulation nothing and never keeps a region awake. Crowns don't sway yet:
  that needs per-tree identity.

### 3.11 Rigid bodies
- A body is a local grid of cells with a pose: centre of mass, orientation as
  a unit complex number (turned by a Taylor series, no libm, so stepping is
  bit-identical across machines), velocity and spin. It lives in the sim and
  steps after particles, with substeps so no boundary cell moves more than
  0.6 cells per substep.
- Contacts are boundary wood cells inside solid playfield, or inside solid
  background around its hinge (the stump it broke from, so a tree pivots
  over its cut; it passes through other trees). Sequential impulses with
  friction; penetration corrected in proportion to depth.
- Leaves don't collide; when the crown touches the ground they shed as a
  flurry.
- At rest (or after 10 s) it becomes cells again: in the playfield (a log,
  mineable, still burning if it was) if it came down on the ground, in the
  background otherwise; the usual ground check then runs on it.
- The game draws a body by mapping each world cell it covers back into it,
  so it stays on the cell grid at any angle, and a fast-moving trunk damages
  and knocks back creatures it hits (once per body per creature).
- Not yet: bodies aren't saved with the world, don't collide with each other
  or with creatures, and playfield pieces still fall as rubble.

### 3.12 Elements on bodies
- One rule, `World::exposure(min, max)`, says what the cells a body covers
  (and the ring it touches: the floor under its feet, a wall beside it) do to
  it, from material data only: heat (any non-burning cell above 60 °C,
  0.1 damage/s per °C over: lava ~114/s, steam scalds, glowing rock burns
  feet), cold (below −10 °C it chills, fully 60 °C further down; below
  −60 °C it also hurts), corrosion (the
  material's `corrosive`, damage/s: acid 30, acid fumes 8), flames or
  burning cells (it catches fire), and being mostly under a liquid that puts
  fires out. The worst cell counts, not the sum, so size doesn't matter.
- The game keeps three statuses, each shown on the HUD as a round timer:
  - `Burning`: 7 damage/s for 4 s (times the coating's `burn`), flames
    painted above it and grass it stands in lit (a burning orc running through
    a meadow lights the meadow). It burns out: its own flames never relight
    it. Water, snow, a fireproof coating or hard frost put it out.
  - `Coated`: what the last fluid it touched left on it, from
    `assets/data/coatings.ron` via each material's `coats` (water/snow/ice:
    wet, oil: oily, blood: bloody, acid: acid). One coating at a time: a new
    fluid replaces the old (jump in water to wash off oil); the same one
    refreshes it. A coating can be fireproof, resist heat, make fire burn
    longer and harder, catch from heat alone, or do damage. Standing in the
    rain wets.
  - `Chilled`: slowed down to 40 % while touching the cold and 1.5 s after
    (`MovementStats::slowed`); hard frost puts a fire out.
  Creatures resist per kind in their RON (`resist: (heat, corrosion, fireproof)`).
- Bodies in liquid: water is thick (strong drag) and you're nearly buoyant
  (a tenth of gravity, sinking at ≤ 25 cells/s). Swimming: jump is a stroke
  toward where you steer (W/S/A/D or the arrows; up if nowhere), one on the
  press and another every 0.35 s while it's held (glide between: the
  drag); held, you tread water (no sinking), so ~27 cells/s up or down and
  level sideways. A body trades places with liquid the way sand does
  (`WorldEdit::Displace`): what's in the cells it moves into goes to the
  open cells it just left (at a surface; under water it's liquid all round
  and nothing moves), splashing out at speed. (It used to push the water in
  its box onto the surface beside it every tick: a body in water pumped up a
  wall of it.)
- A particle that lands where there's already liquid (a splash from under
  water, a pool flowing over it) takes that cell, and the liquid it
  displaced is thrown up from the surface above (a particle, so many of them
  spread instead of stacking); a splash started inside a liquid starts from
  its surface. Nothing is lost.
- Blood washes out in water: where they touch it dissolves (adding no water,
  so a pool doesn't swell with every wound; it once turned into water, and
  a well grinding orcs over a pool made it overflow).
- A blast throws liquid instead of destroying it: inside its radius every
  liquid cell is flung up and out (a geyser; oil goes up burning), and
  near its heart some flashes to its vapour (water to steam). Test.
- Water dilutes acid where they touch (`chance_4096: 4`: reactions can be
  given per 4096 for slow ones): a lone cell under water lasts ~2 s, a
  puddle on a pool's floor ~half a minute, till it's all water.
- Acid eats by hardness (`eats` in `materials.ron`, one rule, not a list of
  pairs): solids, powders and plants up to hardness 90 (dirt, wood, sand,
  stone, brick, most ores), softer ones faster, nothing `inert` (glass,
  gold, toxic crust); each cell of it bites 6 times before it's spent into
  smoke, so a little acid digs a pit bigger than itself.
- What `charges` (water, acid, blood, metal ores) carries lightning: a zap
  that ends in or within 2 cells of it charges all of it that's connected
  (up to 6000 cells, `World::charge`, reported as `Zap::charged`); every
  body touching a charged cell is shocked (25, stunned 0.6 s), its caster
  too if it's standing in the pool. The pool crackles blue and lights up.
  The sky's lightning does the same where it strikes water (or earths
  next to it), shocking harder (40): `Strike::charged`.
- Acid boils at 110 °C into acid fumes: corrosive (they eat what acid eats,
  weaker, used up doing it), condense into acid rain downwind, and flammable
  (a spark flashes the cloud into fire, with the odd small pop), which boils
  more acid. Blood boils into blood steam and freezes, like water.
- A burning liquid isn't put out by what it floats on: an oil slick burns on
  the water under it. Oil conducts heat poorly, so the lake survives.

### 3.13 Weather
- Clouds are a coarse moisture field (4×4-cell texels) over the whole world
  width, in a band of sky above the surface (`ChunkGenerator::cloud_band`),
  not cells: a sky of drifting gas cells would keep every chunk up there
  awake. Stepped every 4 ticks from seed, tick and wind only (plus vapour fed
  from below), with `+ − × ÷` only, so it's deterministic for co-op.
- Each column relaxes toward a cloud shape (flat base, heaped top) set by
  seeded humidity fronts pinned to the moving air; the pattern drifts with
  the wind (~3 cells/s at full wind). Fronts come and go over tens of minutes,
  a storm lasts minutes. Above 0.9 moisture a texel rains out.
- Rain and snow are particles, started only over loaded ground: rain (snow
  where the cloud is below 0 °C, melting into rain in air above 1 °C) puts
  out flames and burning cells it passes or lands on, front and back; one
  drop in 150 (one flake in 10) lands as a cell, so downpours make puddles,
  not floods. New drops stop above 18 000 particles in flight, shared evenly,
  so the particle cap never evicts drops mid-fall.
- Steam that fades (rather than condensing on the spot) feeds the clouds
  above it (`vapour: true` in the material): boiled water comes back as rain.
- Creatures under an open raining sky are soaked (`Wet`, fire goes out).
- Rendering: the band is painted in air coordinates (world x minus how far
  the air has drifted) into a texture placed to the screen pixel, repainted
  every 30 ticks or when the view leaves it, so drift is smooth (the field
  itself moves in whole 4-cell texels); a bright rim, light
  body and shadowed belly per cloud; the sky greys as it clouds over.
- Storms: the wettest parts of big fronts rain hard; a column raining more
  than `STORM_RAIN` throws lightning now and then (one in 12 000 weather steps
  per column: a storm over a screen strikes every ~10 s). Lightning comes down
  the column from the cloud base and strikes the first solid, liquid, plant or
  tree. It bursts there (a small blast that shreds leaves, not wood, and
  craters soil) and heats the crown round it. Through a tree it runs on down
  the trunk (following the wood) to the ground: everything flammable within a
  cell of its path flashes alight at 1200 °C, the most a background cell
  holds, spitting flames and embers, so the tree burns from crown to foot at
  once. It blows the top of the trunk off. Where it earths the ground reaches
  1500 °C (sand fuses to glass) and catches. Measured: a struck tree has 47
  of 240 wood cells left after 20 s, one lit at its foot 146. Creatures
  within 10 cells take up to 55 damage and catch fire (unless wet). The game
  draws a forked bolt down to where it earthed, flashes the sky and shakes
  the camera.
- `WorldEdit::Weather` forces a storm or a clear sky over an area (fading back
  over ~2.5 minutes) and `WorldEdit::Lightning` strikes a column: dev V, B
  and N (or F5-F7, or the dev panel), and later spells or events.
- The field steps each column every 4 ticks, a quarter of the columns each
  tick, so its cost (~1.2 ms for the world's width) is spread evenly rather
  than landing on every fourth tick. The ground under a raining column is
  looked up once every 10 s.
- The band sits 150 cells above sea level, over the tallest trees. It is often
  above the loaded area (zoomed in): rain, snow and lightning enter the world at
  the top of what's loaded below the cloud. Rain or snow is decided by the
  temperature of the ground it will land on. Drops fall at ~2 cells a tick and
  douse a 3-cell strip as they fall (skipped where the chunk is asleep:
  fire keeps its cells awake), until one meets background fire hotter
  than 800 °C: that boils it off (the cell loses 60 °C) and it's gone. So
  rain puts out a spreading fire's cooler edges at once and wears a blaze's
  heart (or a struck trunk) down from the top. Measured in a storm over four
  trees: the struck one burns through, its neighbours 60 cells off don't
  catch, and nothing is alight after 30 s.
- Not yet: weather isn't saved (it restarts from the seed), wind gusts from
  storms, lightning conducting through water and metal.

## 4. Rendering

- One texture + one sprite per loaded chunk (~100 entities on screen, not
  ~270 000). A chunk re-colours and re-uploads only when its cells changed.
- **Nothing proportional to the number of cells may run because the player
  moved.** Legacy recomputed FOV (~200k hash inserts) on every tile crossing,
  and tile crossings get more frequent as cells shrink: at 3 px, walking cost
  40 ms/frame against 13 ms standing still. Lighting and visibility are
  computed per chunk on a coarse grid when *the world* changes, and applied
  on the GPU.
- Fire is drawn by its life, not its shade: white-hot when new, then
  yellow, orange, red, nearly out, jittered a step either way each tick (a
  fire flickers from hot to dull), and it licks upward: a pixel or two of
  flame drawn into the air above it (drawing only; the sim's fire rules are
  unchanged).
- **Sparks** (`vfx.rs`): visual-only effects, not in the sim, one dynamic
  mesh like the particles (at most 8000, oldest first): position,
  velocity, gravity, drag, jitter, colours over life, fading over the last
  third, an optional faint halo 3× their size. What spells look like is
  data (6.1, `look`). A soft round `Halo` image (tinted by the sprite) is
  what spells glow in.
- **Air jumps** (`actors::AirJumped`) puff a small cloud under the feet
  (sparks) and flash a soft blue light, so they show in the dark. The player
  has 3 while developing; later gear gives them (a cloud in a bottle).
- **Hurt** (`actors/hurt.rs`): anything with `Health` that loses 2 or more
  in a tick flashes red for 0.12 s and, if it bleeds (`blood` in its RON,
  default `blood`), sprays 1.2 cells of it a point lost (at most 70); a
  death bursts out 110. Real cells: it pools, runs, boils, freezes,
  conducts lightning and coats what it touches; what it loses floats up as a number
  (hits in the first 0.35 s add to it; the player's red, others pale),
  noticed just before deaths so a killing blow shows.


### 4.1 Lighting and the day
- Materials say how light treats them (`materials.ron`): `glow` (the light
  they give off: lava, fire, acid) and `opacity` (how much they stop per cell;
  by default solids and powders most, liquids some, gases and plants little).
  Hot cells glow by temperature and burning cells flicker, whatever the data
  says; flying embers, blasts and lightning light up too.
- Every frame a light grid covers the view plus a margin (1 texel = 1, 2, 4 or
  8 cells by zoom, anchored to the world). It is filled from the cells, lit by
  the sky, the player's lantern and flashlight,
  then spread: every texel takes the best of what leaves its neighbours,
  straight and diagonal, so pools of light are round. Walls take light on
  their face and pass almost nothing on; a separate rim pass shows lit rock a
  few cells deep without letting light through walls. The flashlight is traced
  as rays into a direct buffer (hard shadows) of which a third scatters off
  what it lands on.
- The solve runs on the async pool and is shown the next frame; the frame
  pays only for reading the world (~1 ms at 3 px/cell).
- Light eases between frames (in over ~20 ms, out over ~50 ms), so moving
  smoke, flames and embers make fire glow and flicker rather than strobe.
- Gases barely dim light (smoke and steam cast no real shadows); trees shade
  the ground only slightly (a forest by day is bright); walls behind rock
  block the sky.
- It is drawn twice over the world and everything in it: multiplied (what
  isn't lit is dark; 0 ambient = Noita-dark, tunable) and added (a haze
  around what glows). Rendering only; the sim never reads it.
- The sky lights the world as directions, not straight down: the sun (or
  moon) crosses from east to west (never lower than ~12°) carrying the full
  sky light, and the rest of the sky comes in from 35° either side of
  vertical at 55 %, so shade is soft and nothing casts a hard shadow straight
  down. Each enters at the tops of columns open to the sky and down the side
  it comes from, dimmed by what it passes: tree crowns barely (1 % of their
  opacity), walls behind rock fully.
- Light sources: anything with a `LightSource` (colour, flicker) lights its
  surroundings: planted torches (G, the torch item), thrown glow sticks (tool
  7, green and blue in turn, 90 s, fading), later lanterns and glowing eyes.
  Fire (`flicker` 1) sways, flutters and jitters (about 0.75–1) and reddens
  as it dims (green by f^1.5, blue by f²). The player carries a lantern
  always, and L steps through nothing, a small flashlight, the big one and a
  torch in the off hand (at the back arm's hand, `back_arm.hand`).
- Torches (`light/torch.rs`, `assets/art/torch.ron`: its `flame` and
  `grip` anchors) burn with a look only (`lighting.ron` `fire`: flames of
  rising motes shrinking from white to red, a wisp of smoke, an ember now
  and then; a second's worth each), so a torch sets nothing alight.
- Heat on the background: where the playfield is open, the heat tool warms
  background cells, which catch fire by the playfield's rule (a heat gun on
  a tree lights it). Background cells hold heat but don't conduct it.
- Day and night: time of day from the tick (20-minute day by default), sky
  light white by day, golden at dawn and dusk, dim blue moonlight at night,
  greyer under cloud, flashed by lightning. `lighting.ron` sets all of it and
  hot-reloads. Keys: L what you carry (beam, big beam, torch, nothing), F8 +3
  hours, F9 lighting off.

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
- **Flyers** (`MovementStats::fly_speed` > 0): in the air, or steering up off
  the ground, a body flies: its velocity steers toward (move_x, move_y) ×
  `fly_speed` at `fly_accel`, sinking at 0.15 of it when steering neither
  up nor down (a glide). Birds, later bats.

### 5.1 Art as text (`platypus_art`, `assets/art/*.ron`)

- A sprite is text: a size, the feet (the pixel standing on the ground), an
  optional outline colour (drawn round every shape, sideways and up/down), a
  palette of characters ('.' clear; '_' in overlays: erase), frames as grids
  of them, derived frames (another frame moved by `shift`, mirrored by
  `flip`, drawn `over`), clips (frames, fps, looping) and anchors (named
  points per frame, for hands and weapons later). Written by hand, by the
  model, or (later) by the in-game editor.
- **Rigs:** `parts` (grids of any size, each with a pivot, its joint, and
  named points such as a hand's grip) and `poses` (a frame put together from
  parts: each part's pivot at a spot, in order, optionally mirrored about
  its pivot, shaded (0.7: the arm behind the body), or `outline`d where it
  lies over what's already drawn, so an arm in front of the body stands out).
  Poses are frames like any other (outlined as one shape) and a part's
  points become the pose's anchors: a weapon will find the hand in every
  frame by itself. The player (`assets/art/player.ron`, ~20 px tall on its
  6 × 15 body, the head may overhang it) is drawn this way: a head (and a
  blinking one), a torso, arms hanging, forward, back, lifted and reaching,
  legs standing, four run strides, tucked and dangling; poses for standing,
  breathing, blinking, four run steps (arms swinging against the legs, a
  bob on the passing steps), rising, falling, dashing, wall-sliding, hurt.
- **Tags and fans:** a pose layer may carry a `tag` (the player's arms:
  `front_arm`, `back_arm`); its points are also the pose's `<tag>.<point>`
  anchors (`back_arm.hand`: where the off hand is). For a tag with a fan,
  every pose also gets a `<pose>~<tag>` frame drawn without that layer, and `fans: { "<tag>": [angles] }` draws the tagged part alone
  at each angle (`<tag>@<angle>`, its pivot at the frame's middle, the
  angle's part named `aim<angle>`, `aimm<angle>` below the horizon), its
  points carried. Neither needs a clip.
- **Turned layers:** a layer's `turn` (degrees, + counter-clockwise as
  facing right) swings its part about its pivot with RotSprite, its points
  with it, so a limb is drawn once and posed by turning it; a fan arm's
  `from` (the angle its part is drawn at) makes the whole fan from one part.
  The troll is built this way: one arm, one leg, a torso and a head.
- **Casting arm:** a caster (`Aiming`, set for a moment by each cast) is
  drawn as its pose without the front arm, with a separate arm sprite, the
  fan frame nearest the angle from the shoulder anchor to the cursor, laid
  at the shoulder; the fan frame's `hand` point is where the spell leaves
  from (`HandPos`). Rigs without the fan simply don't aim.
- **Facing and pace:** a creature faces the point it aims at (the player:
  the cursor) whatever way it moves; running away from it plays the run
  backwards. The run clip's rate follows speed (0.5–1.5 ×). Clips change
  with hysteresis (airborne only after 0.12 s off the ground, running on
  above 8 cells/s and off below 3), so steps and slopes don't flicker
  between clips and restart them.
- `compile` draws every frame (derived ones in dependency order, cycles
  named), outlines, resolves clips and anchors, and packs an atlas (8
  frames a row). Mistakes are errors that say where.
- `check` warns about lone pixels, colours never used, frames no clip
  plays, feet off the frame, odd frame rates; a test compiles every asset
  with no warnings.
- The CLI `platypus-art` is how the model sees its work: `sheet` (a contact
  sheet, a row per clip and one of every frame, feet red, anchors blue, with
  a legend of which row is what), `render` (one frame), `check`, `describe`
  (a sprite in words: where each frame is drawn, its colours, symmetry,
  clips), `import` (a picture of frames to a sprite file).
- A creature with `art: "<name>"` is drawn from it: size, feet and clips
  come from the art (clip images are `art:<name>`, made from the compiled
  atlas once); editing the art file reloads every live creature drawn from
  it.

### 5.2 Critters (`actors/critters.rs`)

- The `critter` brain (params: `flee_range`, `calm_after`, `hops`, `flies`,
  `wander_every`, `rest`, `wander_speed`): rests and wanders when calm; flees
  a player within `flee_range`, a blast within 4 × its radius, or being hurt,
  for `calm_after` seconds, away from it. Hoppers move in hops (pausing
  between them when ambling); flyers fly up and away when scared and glide
  back down to land when calm. Walled, it turns round.
- Ambient life (`assets/data/life.ron`, hot-reloaded): for each kind, where
  it lives (`Surface`: open ground; `Shore`: within 30 cells of water), at
  most how many within 450 cells of the player, and a chance a second: a
  new one turns up 330–440 cells to either side (just off the screen), on
  the first solid ground under open air near the surface; any further than
  800 cells is gone.
- Rabbits (hop, bolt), birds (hop and peck, fly off, glide back), frogs (by
  water, leap).

### 5.3 The arena (`PLATYPUS_WORLD=arena`; `worldgen/src/arena.rs`, `game/src/arena.rs`)

- A walled sandbox 1280 × 640 cells, open to the sky, one floor at y 160
  with, left to right: stairs (5-cell steps), a ledge and a ramp; a water
  pool (110 × 50); two one-way platforms; the open floor where the player
  starts (x 640) with three training dummies and a sandbag; a lava pit; a
  sand heap; two stone columns to wall-jump between; a block of planks.
  It isn't `wild` (a generator flag): no enemies about the start and no
  ambient critters, only what's put there.
- **Dummies** (`dummy` brain: `anchored`, `reset_after`): never die (what
  they lose is counted, then healed, after the damage numbers see it and
  before deaths), flinch (`hit` clip), and show over them the fight's
  damage per second (a single hit: over a second), total and length; a
  fight ends after `reset_after` s untouched (the readout dims). A dummy is
  planted where it first stood; a sandbag isn't, so it flies (and leaks
  sand). Everything that lowers health counts.
- **The panel** (open from the start in the arena; anywhere from the dev
  panel's "Arena tools"): pause (P) and step one tick (.) — virtual time
  paused, the fixed clock handed exactly one tick, bodies drawn where they
  are, not interpolated; slow motion 1, ½, ¼, 1/10 (, cycles); overlays
  (Y): every body's box (player blue, enemies red, the rest green), its
  facing, its feet, and its hand while aiming; what `O` spawns at the
  cursor: a pack (`assets/data/packs.ron`: members and how many, in a line
  across the cursor, each on the ground under its place; `warband` — a
  troll, three orcs, two archers — by default, everywhere) or any creature
  file but the player's; clear the floor (all but
  the player and planted dummies).

### 5.4 The art editor (`game/src/editor.rs`, `art/src/edit.rs`)

- In the game, from the arena panel (E): the sprite files in `assets/art`
  (or `PLATYPUS_EDIT_DIR`), everything in one (poses, frames, derived
  frames, parts), a canvas, the palette, a pose's layers, a clip playing.
- It edits the files as text: `edit` finds a value by path (`parts.arm.
  rows`, `palette.'a'`, `poses.stand.0.at`) in a small RON span parse and
  rewrites it, or puts in a missing entry (and the maps on the way to it)
  laid out like its neighbours; every other byte stays. A stroke is saved
  when it ends (one undo for the whole stroke), so creatures in the world
  reload as you draw; a change that wouldn't compile isn't made; the file
  changed by someone else is taken up (undoably).
- Canvas: a frame or part shows its own grid; a pose shows itself, and
  painting it paints the picked layer's part where it lies (mirrored if the
  layer is). Onion skin: the frame before in its clip at 30 %. Marks: a
  part's pivot (cyan) and points, a frame's anchors (yellow), the feet
  (red), the picked layer's box.
- Tools: pencil, eraser, fill, pick (and the right button), point (the
  named point at the click: a part's pivot or points, a frame's anchors).
  Colours: nudge R G B by 8 (Shift: 1), a new colour (a free letter).
  Layers: arrows move the picked one's `at`. Ctrl/Cmd+Z, +Y; Esc or E
  closes. While it's open it has the keyboard (`KeyboardTaken`) and the
  wheel.
- The same edits from the command line, for the model and scripts:
  `platypus-art get|set|paint` (set and paint write only if the file still
  compiles).

## 6. Combat

### 6.2 Melee (`game::combat`, `assets/data/weapons.ron`)

- **A weapon** is a sprite (`assets/art/<art>.ron`: pointing right, a
  `grip` anchor) turned at load to 64 angles with RotSprite
  (`platypus_art::rotate`: Scale2x ×3, nearest turn at 8×, each pixel from
  its centre; `platypus-art turns` shows them). Those pictures are drawn at
  the hand (`HandPos.local`: the aiming arm's hand while swinging, the
  pose's `hand` anchor otherwise) and are the blade's hit mask. Its item is
  `Use::Melee(id)`; its icon is the sprite pointing up and forward.
- **Moves as data:** per weapon `damage`, `knock`, `stun`, a `rest` angle
  and a combo of moves, each `from`/`to` degrees from the aim (mirrored
  facing left), `windup`/`active`/`recovery` seconds (ease in-out),
  `thrust` (cells forward at mid-sweep), `stamina`, `lunge` (cells/s
  forward on the ground), `damage`/`knock` multipliers. Anything asks with
  a `MeleeRequest` (the player: the left button, held to keep swinging);
  it wields its `Wielding` (the player: the item in hand; a creature: its
  file's `weapon`). Asking again after the windup queues the next move; a
  swing that ends unqueued leaves the combo there for `combo_gap` s. Every
  move (queued ones too) costs its stamina; none left, no swing. The arm
  follows the blade (`Aiming`).
- **Hit test:** while it sweeps, the blade steps from last tick's angle to
  this one a turn (5.6°) at a time; each step's blade pixels are tested
  against every body near it after a box check: against the pixel of its
  shown frame there (`Animator::shown`, `pixel_at`) for a rigged creature,
  its box otherwise. Each body is hit once a swing; not your own team
  (anyone can hit the neutral), not the untouchable. The sweep cuts plants
  (hardness ≤ 2) and sparks off stone (hardness ≥ 20) above your feet, once
  a swing. A trail of motes smears the outer blade.
- **Every hit is one `Hit` message**; `apply_hits` does what hits do:
  damage, knockback (away, and up a little) and stun, sparks, hit-stop
  (virtual time at 3 % for ~55 ms, longer for heavier hits), a shake.
  Striking downward in the air (aimed more than 30° below level), a hit
  bounces the swinger up at `pogo` (300 cells/s) and gives back its air
  jumps and air dash.
- **Stamina** (a creature file's `stamina`; the player 100, a green bar
  under mana) comes back at 45/s half a second after it was last spent.
  **The dash is the dodge:** it costs 18 (none left: no dash) and makes
  you `Invulnerable` for 0.25 s (what you lose meanwhile is given back,
  just before damage is noticed).
- **Taking hits** (a creature file's `poise`, `heft`, `after_hit`): hits
  within `poise` damage (whole again 1.5 s after the last hit) are shrugged
  off (no stun, a quarter of the knockback); the one that breaks it
  staggers (full knockback and stun, its own swing broken off). Knockback
  is divided by `heft`. `after_hit` s untouchable after a hit (the player
  0.6, so a crowd can't juggle you).
- **Enemies fight** with the same swings: the `melee_walker` brain's
  `reach` (swing at a player that close), `combo` (moves of its weapon in
  a row) and `attack_every` (the wait after an attack ends, ±30 %); it
  faces its target, stands its ground while swinging and doesn't swing
  while stunned. The orc (humanoid rig, a cleaver: hack, backhack; windups
  0.26 / 0.18 s; poise 16) and the troll (a rig of turned limbs, 15 × 34,
  420 hp, a club: smash, sweep, 30 damage, 0.5 / 0.4 s windups to read and
  dodge; poise 80, heft 4). In the `fight` scenario the orc lands a hit,
  the shortsword kills it in ~2 s; the troll's smash takes 30 and throws you.
- **Bows** (`archery.rs`; `weapons.ron` `bows`, `arrow`): what wields one
  draws it while it asks (`DrawBow` each tick: the player holding the left
  button with arrows in the pack; an archer's brain) and looses when it
  stops asking; drawn fully in `draw` s, the arrow's speed, damage and
  knockback go from the first to the second of each pair by how far it was
  drawn (under a tenth: nothing). The player's pack pays an `arrow` item a
  shot. The bow shows (turned to the aim, drawn past a third) only while
  drawn.
- **Arrows** fly a cell at a time, turned to where they go, falling at
  `gravity`; their tip strikes a body pixel against its frame (a `Hit`) or
  sticks in stone and earth for `stuck` s (falling if what held it goes);
  within `pickup` cells of the player a stuck one comes back as an item.
  Through fire or lava one catches (burning `burn` s: a flame and a light
  at it; water puts it out and slows it) and sets alight where it strikes.
- **The orc archer** (`archer` brain: `near`, `far`, `draw`, `every`,
  `wobble`): backs off within `near`, closes beyond `far`, stands to draw,
  and looses at where you'll be (leading you and allowing for the drop),
  its aim off by up to `wobble` degrees a shot. In the `archery` scenario a
  full draw does 20 to a dummy, the stuck arrows come back, one shot down
  through lava burns, and the archer costs ~17 hp in 4 s.
- The shortsword (12 damage: slash, backslash, thrust) and the longsword
  (24: cleave, sweep, drive; slower, heavier on stamina). In the `melee`
  scenario: the shortsword lands 5 hits (65) in 0.9 s; the longsword
  empties the stamina bar in 1.5 s; striking down from above bounces
  (300 cells/s); a blast mid-dodge costs 3 hp against ~15 standing.

### 6.1 Magic (`game::magic`, DESIGN §7b)

- **Runes** (`assets/data/runes.ron`, hot-reloaded) are one of three kinds:
  a *carrier* (how a spell travels: `Bolt`, `Orb`, `Stream`, `Lightning`), a
  *payload* (what it does where it lands: `Damage`, `Blast`, `Heat`,
  `Ignite`, `Matter`, `Knock`, `Shatter`) or a *modifier* (how it behaves: `Gravity`, `Trail`,
  `Speed`, `Trigger`). Each costs mana and has a colour.
- **A wand** is an item (`Use::Cast { runes, delay, recharge }` in
  `items.ron`). Its runes read left to right into casts (`runes::casts`):
  modifiers gather until a carrier; the payloads after the carrier ride it;
  a `Trigger` makes the rest of the wand the cast set off where it lands,
  otherwise the rest is the wand's next cast. A cast looks like its last
  payload (acid is green) or its carrier.
- **Casting.** Holding a wand sends a `CastRequest` every tick; the wand
  (per caster) keeps its own time: `delay` after a cast, `recharge` after
  its last. The caster pays the cast's mana (and what it triggers) up front
  from `Mana` (the player: 100, +30/s, a bar under health); without enough,
  nothing happens.
- **Bolts and orbs** are `Spell` entities stepped a cell at a time: through
  open cells and liquids, stopped by the first solid or powder cell (an orb
  bounces off its first `bounces` solids, losing 30 %) or body with
  `Health` (their caster after 0.3 s: a fireball can come back), or where
  they are when their `life` runs out. Gravity: an orb falls at 0.3 of a
  thrown thing's, plus any `Gravity` rune. Trails shed their material as
  embers every other cell; a `Shed` rune splashes cells of its material
  (burning oil, for a fireball) wherever it bounces.
- **Meeting a liquid.** A fast orb coming in shallow (vertical under 0.6 ×
  horizontal, over 110 cells/s) skips off it, up to 3 times, like a stone
  (a burning one steaming). Otherwise: fire (ignite, heat, a burning or fire
  trail) into water (anything that isn't flammable) is doused at the
  surface: it goes off there, its blast a steam blast, lighting nothing,
  its heat flashing the water around to steam, its fire sparks swapped for
  a hiss of steam; fire onto oil lights it; frost freezes the water it
  lands on or beside into ice (bridges); anything else plunges in, keeping
  0.9 of its speed a cell and ageing 3× as fast, fizzling below 50 cells/s
  (a bolt dies within ~20–30 cells). Where it meets the surface it throws
  real cells of it up.
- **Landing** applies the payloads through the sim's own edits: a blast is
  `WorldEdit::Explode` (so a fireball digs, throws debris and bodies, and
  hurts like a small bomb), heat `WorldEdit::Heat`, ignite
  `WorldEdit::Ignite` plus setting alight creatures in the radius, matter a
  `splash` of real cells, damage the body hit (with knockback), knock
  throws the body hit (`Knock(260)`: along the flight, a little up),
  shatter (`WorldEdit::Shatter`) breaks the solids and powders it lands on
  up to its hardness in a radius and throws them off as rubble, away from
  where it came (not when it hit a body). The spark wand is bolt + spark +
  knock + shatter (radius 3, hardness 60: dirt and stone, not slate).
- **Streams** spray, from 6 cells ahead of the hand, flames that last
  `life` s (± a third; the flame wand: 300 cells/s × 0.33 s, about 100
  cells) and become real fire cells where they stop (they rise, flicker and light what they touch)
  and one in six burning cells of their material; what the stream plays on
  is heated (+12 °C a cast, radius 3: wood catches, ice melts); what stands
  in it is scalded (2 a cast) and may catch.
- **Lightning** picks up to `targets` creatures within `range` toward the aim
  (within 0.6 rad of it, or 30 cells of the cursor), nearest the line first,
  or else aims at the cursor (up to `range`), and strikes each with
  `World::zap`: a jagged walk (pulled back to the line, gathered in at both
  ends) through open cells, stopped by the first solid, liquid or plant; two
  forks off it through open air; what burns along it catches and some air
  flares; where it ends it bursts (radius 2), heats and ignites. The sim
  reports each zap with its path (`StepStats::zaps`); the game draws
  exactly that path and hurts what's within 3 cells of the end (30 at most)
  and sets it alight (`elements::zapped`), as the sky's lightning does. It
  doesn't flare the air in its first 8 cells (the caster's hand). Into water
  (or anything that `charges`) it charges the pool (§3.12).
- **Channelled spells** (`magic/well.rs`): held open while the wand is
  held, paying `drain` mana a second, and the caster's mana doesn't come
  back meanwhile; out of mana, or let go, and it lets go.
  - **A gravity well** (`Carrier::Well`) sits at the cursor, following it
    on a spring (top speed 700 cells/s, most acceleration 9000: it can be
    swung, and what it holds keeps its speed when you let go: thrown). It
    tries `pull` random cells in its
    reach each tick: powder, liquid and plants easily (more so nearer),
    solids up to its `strength` in hardness harder the harder they are
    (`World::pluck`, then `loosen_fragments` so what they held up falls), and
    catches particles in flight (`World::take_particles`), up to what it can
    `lift`: a cell weighs its density against water's (stone ≈ 2.6), a body
    its size in cells (an orc 168). Held cells each steer toward a place in
    a spinning ball with a limited `grip` (cells/s²): whip the cursor and
    the outer ones can't follow; past 1.4 × the reach they fly off as real
    cells with the speed they had. Bodies (not the caster) within what's
    left of the lift are carried in its heart (their own fall gravity
    cancelled, stunned; safe while carried: not ground, no fall damage,
    `Carried`, till they're let go),
    heavier ones only tugged; held rock grinds any body it's inside (by its
    speed through it). Released, it all drops keeping its momentum.
  - **Force** (`Carrier::Force`) comes from the caster, a telekinetic shout:
    a cone from the hand toward the cursor (±0.7 rad, out to `radius`, not
    the 4 cells at the hand); the left button flings everything in it away,
    the right drags it in (bodies till they're 10 cells off). Each tick it
    walks the cone farthest first (nearest for a pull) and flings up to
    `pull` cells (loose ones, solids up to `strength`) that have somewhere
    to go (straight on, else mirrored upward, else flat to the side: a push
    into the ground splashes), so the ones in front make way and a pile
    blows apart; particles in flight are shoved; bodies (not the caster) are
    launched at up to `power` cells/s (a push lifts a little) and stunned,
    and hurt by the blow: 0.04 per cell/s it changed their speed by (the
    first of a held push hurts, not every tick of it: a body already flying
    off isn't changed).
    What it can't move pushes back: the caster is driven the other way (a
    pull: toward it) at up to 1.4 × `power` × the share of the cone that's
    solid, reached half the way each tick: pushing straight down throws the
    caster up (held, a hover that settles where the cone reaches less
    ground), at a wall away from it; pulling at a ceiling hauls the caster
    up to it.
  - Levels are runes: `gravity_well` / `gravity_well_ii`, `force` /
    `force_ii` (reach, lift or power, strength); wands carry level I,
    staffs level II.
- **The distortion** (`magic/warp.rs`, `shaders/warp.wgsl`): a screen-space
  pass (Bevy's fullscreen material, in `Core2d` post-processing) around the
  strongest field. A well swirls and pinches inside 1.3 × its reach (harder
  the more it holds) with a dark heart and a faint bright ring. Force is a
  cone from the caster toward the cursor (1.25 × its reach): sharp
  wavefronts racing out along it (a push, stretching the picture at each
  front) or in (a pull, squeezing it), brightest at the fronts with the
  colours split a little, a faint haze filling the cone. Strength 0 leaves
  the picture alone.
- **Looks** (visual only): each rune may have a `look`: a `trail` (sparks a
  cell flown; a stream's come out of the wand with it) and a `burst`
  (sparks where it lands, off the surface it hit, or back along its way).
  A cast shows the looks of all its runes, so a composed spell looks
  composed (fire trail + acid: flames and green drips). A bolt or orb is a
  pale core in a halo of its colour.
- **What flies hurts** (`actors::pelted`): a particle of solid, powder or
  liquid (not rain, dust or embers) faster than 90 cells/s passing through a
  body deals its weight (density against water's; liquids half) × how many
  times faster × 0.8, and is mostly stopped (30 % of its speed left),
  shoving the body: a ball of rock dropped from a well, blast debris, a
  flung stream of sand.
- **Fall damage** (`FallDamage`, per creature RON) is by distance, Terraria
  style (speed saturates at max fall within ~50 cells, so it couldn't tell
  a double jump from a cliff): falling further than `safe_height` cells
  from the highest point since it last stood on something (or was in
  water, or jumped off air or a wall: a double jump just before landing
  saves you) hurts `per_cell` a cell over (player 100 and 0.6: a double jump
  never hurts; orc 70 and 0.8); slamming into a wall or ceiling faster
  than `slam_speed` (450 cells/s: flung, not walking or dashing) hurts
  `per_speed` (0.25) per cell/s over.
- **Every explosion hurts** (`actors::blasted`, from `StepStats::detonated`):
  bodies within 1.6 × its radius take up to 0.65 × its power and are thrown
  at up to 3 × its power, falling off with distance. Magic can hurt its
  caster: a fireball at point blank does.

## 7. Extensibility — adding things without touching the engine

| To add…              | You write…                                             |
|----------------------|--------------------------------------------------------|
| a material           | an entry in `materials.ron`                            |
| a reaction           | a line in the reactions list of `materials.ron`        |
| an enemy (existing AI)| a creature RON (sprites, stats, attacks, brain + params)|
| a new AI behaviour   | one module implementing a brain, registered by name    |
| a weapon             | a weapon RON + sprite                                  |
| a rune / a wand      | an entry in `runes.ron` / a `Cast` item in `items.ron` |
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
