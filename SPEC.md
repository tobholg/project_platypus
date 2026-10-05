# Project Platypus — SPEC

Terraria's openness (dig, build, craft), Noita's world (every material is a
simulated cell), Hollow Knight's body (tight movement, readable melee combat).
Co-op from the start.

This file is the contract. Code that disagrees with it is a bug in one of the
two; fix whichever is wrong, in the same change.

---

## 1. Units and coordinates — one convention, everywhere

- **1 world unit = 1 cell.** Transforms, physics, AI, worldgen all speak cells.
  Zoom is purely a camera concern (default: 2 screen px per cell).
- **y points up**, matching Bevy. Cell `(x, y)` covers `[x, x+1) × [y, y+1)`.
  There is no "row 0 = top" anywhere outside the texture upload.
- A world position maps to its cell with `floor`, never `round`. The only
  functions that convert are in `platypus_sim::coords`. (Legacy had rendering
  centred on `x*T` while collision used `floor(x/T)` — a half-tile offset.
  One conversion function makes that class of bug impossible.)
- Chunks are `CHUNK = 64` cells square. `ChunkPos = floor_div(cell, 64)`.
- The scale (agreed 2026-10-01, after a spike): everything was made 1.5×
  bigger in cells and is seen at 2 px a cell instead of 3, so it's the
  same size on screen with 2.25× the pixels in every thing. The rule:
  lengths, distances, speeds and accelerations ×1.5; areas and cell counts
  ×2.25; per-cell decay (light's falloff) to the 1/1.5 power; durations,
  damage, hit points, hardness, chances and counts kept. A block is 6
  cells (was 4). The backdrop's ranges are generated 1.5× as fine (a cell
  a pixel), and rocket boots fire a jet from each boot (the leg parts mark
  `foot_near`/`foot_far`, `animation::Soles`). `tools/scale/ron_scale.py`
  holds the rules for data.

## 2. Crates

```
crates/
  sim/       platypus_sim     cells, materials, chunks, stepping, edits. NO Bevy.
  worldgen/  platypus_worldgen seeded generators: WorldPlan, then fn(plan, ChunkPos) -> cells. NO Bevy.
  worldview/ platypus_worldview renders a generated world (or a region) to PNG.
  physics/   platypus_physics bodies vs a solid-grid trait, pixel masks, sweeps. NO Bevy.
  nav/       platypus_nav     the path planner: a coarse grid, moves as data, searches. NO Bevy.
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
tick, and look up to 192 cells along the surface for lower ground (moving
toward it costs nothing); aimless sloshing is budgeted (8 reversals) so
lakes sleep. A liquid is pushed by the column above it: a cell with its own
kind on top passes *through* its own kind (up to 72 cells) to the first open
cell, reaching `dispersion + depth above` cells, scaled down by viscosity. So
a block of water collapses like a dam break, bottom first (a 64-tall block
is ~25 tall after 1 s, 14 after 4 s), instead of eroding from its face while
only the top trickles out. Surface cells beside an open step doze rather than
sleep (they wake 1 tick in 16 to re-check), so a pressure-flattened pool
never freezes into stairs. `viscosity` (0 water … 255) makes a liquid move on fewer ticks,
pour slower and give up sooner: measured in a basin, water settles flat in
~4 s, oil ~6 s, blood and acid ~8–13 s, lava ~16 s in mounds.

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

- Presets (2026-09-29): `large` (196 608 × 49 152 cells, 3 072 × 768
  chunks: the game's default), `medium` (49 152 × 24 576: the reference
  world the plan's numbers are written for; the bench's;
  `PLATYPUS_WORLD=medium`) and `small` (12 288 × 6 144: the reference scaled
  down, for looking and testing; `PLATYPUS_WORLD=small`). Everything is
  sized from the preset (`plan::Scale`): sizes (a mountain, a biome's
  breadth, a lake) are the reference's times the world's share of it,
  never more than 1, so a bigger world has more of each, not bigger ones;
  counts (mountains, lakes, crypts, castles, islands, chasms, cave mouths,
  underground zones, and one of each special biome per reference width:
  tundras and jungles on their sides, apart from their own kind) are the
  reference's times its width's share. Tests: the same density at every
  size, the same peak heights, the bands. Lone massifs keep off the
  tundra (lifted, its pine forest was a treeless waste); a lake is never
  above its own banks; a giant mushroom never grows into its chamber's
  ceiling.
- Sea level sits a quarter of the way down in the reference world. Bands
  relative to it there: sky above +3 750, peaks +1 200, surface −300,
  underground −3 750, caverns −10 500, deep −16 500, underworld below. A
  smaller world scales them down; a taller one shares out the extra
  height: a fifth to the sky, the rest below (the underground 12 %, the
  caverns and the deep 44 % each, the underworld as it is). Depth as the
  reference world measures it (`reference_depth`, band by band) sets
  loot.
- Climate (`Climate`, in the sim): 15 °C at sea level; 1 °C colder per 90
  cells up (so a temperate peak is below freezing from about +1 350) until the
  sky band, where the air warms again (1 °C per 27 cells, back to 15 °C: the
  sky islands are mild, the summits the coldest place); warmer with depth, 85 °C
  over sea level at the bottom (cavern lakes stay liquid, the underworld is
  hot). A 192-entry table adds each biome's warmth across the world, blended
  at the borders; a chunk has one entry, so the hot path is a shift and a
  lookup. Snow, ice and bare rock follow from it; nothing is painted by biome.
- Biomes (stage 2) are laid out like Terraria's: oceans at both ends, a forest
  at the spawn, a tundra towards one edge and a jungle towards the other, a
  mountain range between the tundra and the spawn, a deep forest somewhere
  away from the spawn (each of those two takes a neighbouring region too, so
  they run 9–18 k cells), a desert, temperate forest, plains and swamp
  filling the rest, never the same twice in a row.
  - The mountain range: a long ridge (1 050–1 500 high) with a peak
    every 1 350–2 100 cells (2 100–3 300 high) on it, cold (−6 °C); above
    the peaks band the ground rises ever more slowly rather than being cut
    flat.
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
  beach to −570; about seven mountain massifs (large world), lopsided, each a
  broad shoulder under a concave peak plus sub-peaks, a wandering ridge line
  and 105-cell terraces, 1 350–3 600 cells high, joined into ranges where
  they meet, none near the spawn. Steep faces wander sideways (overhangs, ledges);
  crests and gentle slopes stay put. Soil thins with slope (bare rock on
  cliffs); snow lies where the ground is below 0 °C and not steep (deeper the
  colder, measured across the slope so steep faces get a crust; very cold it
  clings to steeper faces: below −8 °C slopes to 3, below −15 °C to 5), so
  ledges hold it and cold peaks are white.
- Water: local basins (rims looked for 1 800 cells either side) are filled to
  their lowest rim, levelled flat per lake and capped by the biome's depth, so
  every lake is held and asleep on load; three big bowls always hold one,
  other hollows by chance (swamps mostly, deserts an odd oasis); none in the
  notches between crags. Lakes where it freezes are ice. Caves keep 90 cells
  clear of a lake bed. Fewer caves and no liquid pockets above sea level.
- Sky islands (legacy port): a dozen, high in the sky band, never sharing a
  column; each rasterised once at plan time into a stamp (grass, dirt, stone,
  a walker-carved cave), so chunks stay pure; trees on them from a second
  forest plan.
- Caves are planned, not noise (`caves.rs`, v2): noise caves came in every
  width, so many were just too narrow for the player. Now, like Terraria's
  tile runners:
  - chambers: ragged ellipses, kept apart; small in the underground and
    inside mountains (half sizes 39–90 × 27–51: the first layer is easy),
    bigger in the caverns (68–210 × 42–120) and the deep (83–255 ×
    53–150);
    about 30 % hold a pool (water, some oil, lava in the deep), kept below
    where any tunnel comes in so it doesn't spill; then, by a roll of its
    own (hashed from the chamber, so the rest of the plan is as it was), 18
    % of the dry ones get a pool of oil or acid and 45 % of the water pools
    are oil or acid instead. Every acid pool sits in a lining of acid-proof
    `toxic_crust` (`caves::Open::Lining`: the rock within 4.5 cells of the
    pool, up to 3 over its level; chambers are binned wide enough for it),
    so it doesn't eat its way out and drain;
  - miners' leavings (`TerrainGen::mine_camp`, spawned as `Spawn::Prop`
    creatures): per chunk in the underground, caverns and deep, a try at a
    camp (60–80 %: a mine cart on a floor 39 wide with 23 of air over it;
    beside it on the nearest floor a TNT barrel and dynamite one way, a
    lantern on its post the other) and one at a lone TNT barrel or bundle
    of dynamite (21–25 %). A small world has ~20 camps and ~70 loose
    explosives. `camplook` scenario: to the nearest camp;
  - tunnels: each chamber to its nearest few, a spanning tree of those so
    every chamber connects (tested: 95 %+ in one network) plus more for
    loops; wandering lines 33–66 cells wide (the player is 23 tall); steep
    ones get alternating rock ledges every 45 cells to climb back up;
  - crevices: a fifth of the extra links (never the tree) are cracks 4.5–10.5
    wide, too thin to pass: throw a glow stick in;
  - mouths: ~40 tunnels down from dry land (away from the spawn) into the
    nearest chamber;
  - noise only roughens the walls; a chunk asks only the shapes binned to it.
    In the caverns every cave below the water table is flooded, so where
    tunnels meet the flooded chambers the water is already level.
  - Tests: tunnels fit the player (a 9 × 23 box along 160 sampled tunnels,
    ≤ 1 % blocked), the network connects without crevices, crevices are few.
    Planned in ~70 ms with the rest of the plan (the medium world; the
    large ~650 ms).
- Underground biomes (regions, two each in a large world, one in a small;
  everything else keeps its plain rock, and the underground layer stays
  plain and easy): elliptical areas ~1 350–2 250 × 675–1 125 half size in
  the caverns (toxic grottos in the deep too).
  - Fungal hollows, teal and violet: rock within three cells of open space
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
  - The dressing is a pass over each generated chunk (with a three-cell margin
    from the chunks around, asked once); spikes and mushrooms are planned
    with the chambers. `platypus-worldview` lists the areas.
- Underground (stage 3), by band:
  - underground: the planned caves, sand and gravel pockets, coal;
  - caverns: huge chambers (wider than tall) with stalactites and pillars
    (vertically streaked noise), fading in over the band's top 450 cells;
    chambers below the regional water table (one per 3 072 columns) are
    flooded: underground lakes;
  - deep: slate (with obsidian seams), chambers, lava pools;
  - underworld: basalt; a vault with a ragged, dripping roof, basalt islands
    hanging in it, and a lava sea at one flat level (asleep on load, the
    bench's 265 000 lava cells settle in 118 ticks); obsidian crusts where
    rock meets the lava.
  - Chasms: about five shafts from the lowland surface (away from the spawn,
    lakes and mountains; no trees at their lips) down into the deep, 165–360
    wide, wandering ±375, narrowing and widening (ledges), funnel-shaped at
    the top: the long descent.
  - Rock and walls follow the band (stone, slate, basalt), dithered at the
    borders. Slate melts at 1 500 °C and basalt at 1 650 °C, above what a lava
    sea heats them to.
  - Cost: a caverns chunk takes ~1.9× a surface chunk to generate (12-chunk
    column 3.2 ms vs 1.7 ms, bench `stream_deep`).
- Not yet: waterfalls, jungle and swamp materials (mud, vines), creatures of
  the underground biomes, spore gas, more biomes.

### 3.4c Hands: mining, building, items, chests
DESIGN.md §4–5, stage 4 of the world arc.

- Blocks: 6 × 6 cells on a fixed grid (`BLOCK`); the world stays cells.
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
  a player with room (72 cells) and are picked up (9).
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
  with unit tests. Ctrl picks the best tool for the target (auto tool). What
  a mining tool will take is lit as Terraria's smart cursor lights it: a
  see-through warm yellow over its cells, a little stronger at their edge,
  breathing gently, drawn over the dark (a sprite of one pixel a cell, z
  16.1); a placement is outlined in cyan.
- A pickaxe's two modes (C switches; the label says Precise or Area, the
  hint line `precise [C]`): precise, a block at a time as above; area (its
  `area` in items.ron, a radius in cells: copper 10.5, iron 11.25, gold 12,
  mithril 13.5: dug straight down, a shaft ~20 cells wide, twice the player's
  width, so you fall down it as you dig), a round bite. The bite's disc sits where the line from the
  hand toward the cursor first meets solid (with the smart cursor it goes
  on to full reach past the cursor), else at the cursor: a pick can't aim
  past rock. It takes the disc's cells the pick can get at: no more solid
  cells between the hand and them than the radius (`World::within_reach`),
  so it bites into the face nearest you, never what's on the other side of
  a wall. Each takes 3/4 of the pick's power (30 % less at the rim: round
  holes, but the whole width goes together) and breaks at its own hardness
  (`WorldEdit::MineReach`); what it leaves unsupported falls. The
  `pickarea` scenario: aimed past an 18-wide dirt wall, it digs the wall
  (the dirt behind only once it's through); dug straight down with a
  copper pick, 109 cells in 3 s (75 of dirt, then stone), falling as it
  goes.
- Pace: a copper pickaxe (power 35, 6 hits/s) takes dirt in one hit, stone in
  two; an iron one (power 60, 7/s) stone in one. The player's box is 9 × 23 cells, so it drops into a 2-block
  shaft and walks a 4-block tunnel.
- Chests are furniture: entities, not cells (`hands/chests.rs`), 18 × 15
  cells (`worldgen::CHEST_SIZE`), drawn from a text picture. A chest is a body
  (the same falling and collision as a dropped item): it falls when its floor
  goes, blasts throw it (and hurt it: 60 hit points, a bomb beside it breaks
  it), fire, lava and acid wear it down; breaking spills what's in it.
  Mining one takes its hit points off per hit (two hits of a copper
  pickaxe) and the last spills its contents and the chest.
- Worldgen doesn't draw chests: generating a chunk also reports what it
  starts with besides cells (`ChunkGenerator::generate_with_spawns`:
  `Spawn::Chest` or `Spawn::Creature` at their feet): a cave chest on the
  first cave floor with room for one in some underground chunks (~one in 75),
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
  over 15% of it, at most 600 cells), and noise stretched along the strata
  (veins) or round (blobs). Where a cave is within 8 cells the threshold is
  0.1 lower, so ore shows on cave walls: about 5% of the rock is ore, 10% of
  the rock at cave walls.
- Gems grow only in cave walls (within 6 cells of open space), in clusters,
  and only along some stretches of wall (a coarse noise), so they are a
  find, not a lining: amethyst in the underground (60), emerald in the
  caverns (80), ruby in the deep (100). They glow faintly, which lights the
  caves around them.
- Cost: a deep column of chunks streams in ~3.2 ms, under the 4 ms budget.

### 3.4f Lairs (`worldgen/src/lairs.rs`, `assets/data/lairs.ron`)
- Cave chambers something has made its home: a spider nest, a slime pit
  in the toxic grottos, a bone pit deep down. Each lair kind is data: its
  `depth` under the surface, the underground biomes (`zones`: `fungal`,
  `crystal`, `toxic`, `none`), a `chance` (the share of the chambers it
  could take that it does), its `lining` and `density`, its `keepers`
  (creature, how many) and a `min_size`. `TerrainGen::with_lairs` takes
  them; which chambers they take (never a pool's) is decided from the seed
  with the plan, so every peer agrees.
- A lair's chamber is lined as its chunks generate: the lining on air cells
  within three of rock, in clumps (`density`), and threads of it hanging from
  the roof. Its keepers are reported (as `Spawn::Keeper`) by the chunk of
  the chamber's middle, across it; they fall to its floor. Each carries
  where it was put (`clock::Keeps`, saved with it); the world clock refills
  a lair (§3.14). Ambient life's comings and goings (`life.ron`) leave
  keepers alone: a lair's bats stay when you walk away.
- `ChunkGenerator::landmarks` names them (tools, tests: the `nest`
  scenario goes to the nearest spider nest), and `zone_at` gives the
  underground biome at a cell. The worldview draws them (it loads the
  game's lairs).
- Crypts are kept by skeletons now; castles by orcs.

### 3.4g Gold (`game/src/gold.rs`, the `gold` material)
Noita's gold (DESIGN §13 item 7): a count, not a thing in the pack.
- The player's gold is a number (`Gold` on the player, saved in player.ron),
  shown in the HUD under the stamina bolts: a nugget and the count.
- In the world gold is a material, `gold`: a heavy powder (density 19 300),
  one cell a coin, glittering (`shimmer` 255) and glowing warm (110, 76, 14):
  a heap lights the cave round it. Being a material, everything else is the
  simulation's: it's saved with its chunk, forever; it sinks through water;
  acid can't eat it (`inert`); a blast throws it whole instead of destroying
  it (`flung`, a new material flag: the blast's liquid path, without the
  boiling); lava melts it into `molten_gold` (at 900 °C: lava here is never
  below 950, and a pit's cold floor keeps gold under real gold's 1 064),
  which sets into `solid_gold` below 850 (a thing, like the metals: dug, it's
  coins; SPEC §3.4h); the vaporiser erases it.
  `counted` (a new material flag): never a block item; what the hands dig
  of it is counted (`gold::Dug`).
- Where it comes from: the loot tables' `gold: (min, max)` (loot.ron),
  times one more for every 3 750 cells below sea level. A creature's bursts
  out as it dies (not looted: orc 4–12, archer 3–10, troll 25–50, skeleton
  5–14, spider 2–8, bat 1–4, slime 1–5); a chest's as it's first opened or
  broken (high 20–50, underground 25–60, caverns 40–90, deep 60–140).
  `Chests::gold_found` collects both; `gold.rs` throws each as that many
  specks (sim particles, `Landing::Settle`) up and out in a fountain,
  landing as dust, at most 600 a frame (a hoard pours out over a moment).
- Taking it: the local player takes every gold cell within 9 of its body,
  108 a tick at most (a heap drains at ~6 500 a second), with golden specks
  streaming in (every fourth cell) and a two-note ting.
- `gold` scenario (arena): a warband struck dead: ~32 gold bursts out over 90
  cells, walking through it takes 5–10. 100 thrown into the lava pit: after
  18 s, all 100 molten, nothing lost; 60 into an acid puddle: 58 lie in it,
  uneaten; 60 into the pool: all 60 at its bottom (y 85, the floor). A
  blast in a heap of ~40: ~40 after it, thrown over 45–70 cells. `goldheap`
  (a generated world): 6 000 poured into a hollow dug under the start (it
  opens into a cave now) heap and spill down it, ~15 chunks still awake
  after 45 s (the sim at ~1.2 ms a tick), a glittering heap lighting the
  cave. A sim test: a blast in a pile of gold destroys none of it and every
  cell lands.
- Not yet: rarer and deeper loot tables giving more, a merchant to spend it
  with (the village), molten gold cast into bars (matter, §13.1).

### 3.4h Metals (materials.ron; DESIGN §13.1)
- Each metal (copper, iron, silver, mithril) is two materials: solid (an
  `object`: a casting is kicked, thrown, lifted whole; carries heat and
  lightning) and molten (a glowing liquid that lights and burns what it
  touches). Heat moves between them as between ice and water: solid past its
  melting point runs (copper 1 085 °C, silver 962, iron 1 538, mithril
  1 750), molten sets again 60–70 °C under it.
- A bar *is* metal: a material's `item` names its block item, so the bar
  items (`copper_bar` …) are the metals' blocks, 36 cells each. Mining a
  casting gives bars; a bar laid down is 36 cells of metal. Recipes and
  rewards count bars as before (units: a bar is 36 cells).
- Ore melts into its molten metal ~200 °C over the metal's point (a
  furnace's heat: lava at 1 200 melts silver ore only).
- Molten metal is poured hot (copper at 1 300, iron 1 750), gives its heat up
  slowly (conductivity 25), and lingers past its freezing point (latent
  1 200: it sets with chance (d / 1 200)² a tick, d degrees under), so it runs
  for a second or two before it sets, as metal does. Stone holds copper,
  silver and gold (it gives way at 1 400); iron and mithril want firebrick.
- `firebrick` (furnace: a brick and a block of sand): heat-proof to 2 100,
  a poor conductor.
- Gold: molten gold sets into `solid_gold` (a thing, glowing, `counted`: dug,
  it's coins), which melts back at 900.
- Sim test: molten copper poured into a stone cup sets into 40 cells of
  copper, every one, in ~100 ticks, and the cup holds.
- **Casting.** Vessels (`Use::Vessel { holds, hot, acidproof }`; a stack
  carries its `fill`, saved by the liquid's name): the ladle (144 cells,
  molten metal too; anvil, 4 iron bars), the bucket (144 cool cells; anvil, 3
  iron bars), the flask (36, acid too; furnace, glass). RMB fills: scoops the
  liquid round the cursor (14 cells a tick, one kind), or, at a furnace, melts
  what the pack holds that melts into it (a metal's bars, 36 cells each; its
  ore, half: the rest is slag). LMB pours two cells a tick, thrown so it lands
  at the cursor (under gravity), hot as fresh. A mould is anything built of
  what stands the heat; the casting sets in its shape, and once the mould is
  mined away it's a thing: kicked, thrown, lifted, mined back into bars.
  `cast` scenario (arena): the ladle filled at a furnace from 4 copper bars
  (144 cells), poured into a stone mould with a notch, set in its shape (144
  cells), the mould dug away, kicked 31 cells, every cell kept.
- A kick sends a body at most 1.3 × its direction (the player's: 2.4 cells a
  tick across; a 64-cell casting kicked
  at 300 flew 290 cells into the lava). A body settling nearly square
  (within ~11° of a quarter turn) is squared up first, so it maps cell for
  cell (resampled at a slight tilt a log came back a cell short); a cell
  that meets something solid looks up to 15 above for room.
- Not yet: slag as a material, a furnace spout that pours on its own.

### 3.4e Structures: crypts and castles
DESIGN.md §3.3, stage 6 of the world arc (`worldgen/src/structures.rs`,
rooms in `assets/data/rooms/*.rooms`).
- A room is a text grid at block resolution (one character per 6 × 6
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
  under it, at most 450 cells), about 3 on a large world (2 by seed,
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
  rooms (crypts and castles) gather cobwebs, a ragged triangle up to eleven
  cells out. Moss and cobwebs are still plants; cobwebs `hang`: held from
  above by ground or the web they hang from (and only by that, so a web
  adrift in the air comes apart), and they burn in a flash.
- Secrets: illusory walls hide side rooms (above); some rooms have a niche
  sealed by a weak wall with a chest in it (`sealed_tomb`, `storeroom`);
  every lake 60+ cells deep keeps a chest at its deepest, on the bed. Chests
  above the underground (castles, lakes) roll the `high` loot table (ore,
  gems, the better pickaxes).
- Not yet: keys and locked doors (with the RPG arc), rooms behind
  waterfalls (no waterfalls yet), bosses (a pack of orcs for now).

### 3.4i The start and the village (`worldgen/src/plan.rs`, `structures.rs`, `assets/data/village.buildings`)
DESIGN §13 items 5–6.
- The start is a plain: the ground ±450 cells (× the preset's width
  scale) round the spawn is set to its median height, keeping a twentieth
  of its roll (`PLAIN_ROLL`), and eased back into the land over the next
  225. Trees still grow on it (a foliage cluster is capped at 45 cells from
  its wood, so a crown can't sprawl over a flattened edge).
- The village is a structure (`StructureKind::Village`: planks, brick, no
  cobwebs) laid out once with the plan, from 8 blocks right of the spawn:
  the buildings in `village.buildings`, in file order, 2 blocks apart, each
  on the ground under its middle with brick foundations down into the
  land. A building is a text grid at block resolution in the rooms' legend
  (`#` planks, `%` brick, `W` water, `,` a window: no back wall, the sky
  shows through, `.` open) and a letter a
  person (a keeper: `Spawn::Keeper`) or a station on the row over the floor: `g` guide, `s` smith, `h`
  healer, `m` merchant, `A` anvil, `F` furnace, `B` workbench). Doors are
  open five blocks high at both ends. Today: a guide's house, a well, a
  smithy (anvil, furnace), a healer's, a merchant's (workbench).
- The structure carries its people and stations (`Structure.spawns`,
  `Spawn::Creature`, `Spawn::Station`), reported by the chunk that holds
  them and spawned once like any structure's guards.
- The orcs round a new world's start stand 1 200–1 875 cells out, beyond the
  plain and out of the village's sight.
- Tests: the start is flat (a plain round the spawn); a village stands by
  the spawn, its buildings on the ground, its people and stations reported.

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
anything edge-connected to bedrock, to unloaded world, or to more than 6 750
solid cells; a piece hanging by a diagonal corner is not attached. When a piece
falls, everything touching it is re-checked, so hangers-on follow. Checks
triggered by the simulation are grouped in 16×16 tiles, a few per tick, and
share what they learned about ground within the tick. (Background pieces with
144+ wood cells fall as rigid bodies instead, §3.11.)

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
burned-out background drops a third as much into the playfield, and a
flammable background scrap beside it left with at most one neighbour comes
apart with it (so on along the scrap, within 6 cells): fire can leave the last
cells of a wall unlit, and a built wall never falls. Water puts it
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
seeds). Burned cells don't regrow while a fire lasts, so every fire ends (the
world clock heals them over days, §3.14); firebreaks (bare patches,
rock, water), wind and later rain shape where. A lit wooden slab still burns
up on every seed; a tree takes a few seconds to catch and burns ~20 s.

### 3.9 Particles
Things in flight between cells live in the sim as a plain list (not
entities), step once per tick after the cells, and march one cell at a time so
nothing tunnels. Each carries a real `Cell` and a landing rule: `Settle`
(becomes its cell; solids land `LOOSE` — blast debris, blood, splashes),
`Vanish` (dust, sparks), `Ember` (may ignite what it lands on or brushes).
They are deterministic (seeded), capped at 67 500, and die at the edge of the
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
- Where the background is empty more than 24 cells below the generated
  surface, the renderer draws a dark rock backdrop (earthy near the top,
  colder with depth, faint strata) instead of letting the sky show through.
  A stopgap until the parallax far background.
- A background piece is held up where it rests against solid playfield (a
  trunk rooted in the ground, a wall behind rock). Only growths
  (`grows: true`: wood, mushroom stems) carry anything and come down;
  background that doesn't grow (the rock behind a cave) neither falls nor
  holds a growth up, so a mushroom in a cave is held by its foot alone. Only wood (anything not a
  plant) carries weight: leaves hang on wood within `LEAF_REACH` (108 cells,
  through leaves), so a felled tree is never held up by its neighbour's
  crown. Worldgen keeps every leaf inside that reach and every background
  cell edge-connected (tested by felling generated trees).
- A detached piece with at least 144 wood cells comes away whole as a rigid
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
  Measured over 8 seeds: a burning crown sets the next tree 58 cells off
  alight 3 times, one 223 cells off never (4 and 2 when the first leaf an
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
- **Objects** (`object` materials: wood for now; metals and boulders next):
  a thing, not ground. At rest an object is ordinary cells (it costs nothing,
  is saved with its chunk, is stood on and flowed round like any cell); it's
  a thing because of its material. `World::lift` takes the connected cells of
  its material (9 to `OBJECT_MAX` 5 625: bigger is ground) out as a body. A
  playfield piece that loses its hold falls whole as a body if any of it is
  an object (a casting with rock stuck to it comes away together), unless
  it's lying on something (powder, liquid, a solid, not rubble): lifted it
  would only land and settle again. Settling, a cell that meets something
  solid goes just above it (up to 15 cells), so an object keeps every cell.
- **The kick** (`World::kick(lo, hi, dir, power)`, the game's F): in the box
  before the feet, objects lift out and every body there takes the impulse
  through its centre of mass (at an end it would only lever the thing up on
  its other end), loose powder and rubble fly as specks (gold too), and
  particles in flight are pushed; ground takes nothing. The game's impulse
  is 675 (a 675-cell log leaves at 2.4 cells a tick). A creature in the box
  takes a blow (`combat::Hit`, as a blade's): 8 damage, knocked back 225
  cells/s (before its heft) and stunned 0.3 s unless its poise shrugs it
  off; barrels and carts are only sent rolling (a kick doesn't set them
  off). `kick` scenario: an orc 70 → 63 (its armour), a troll 420 → 417,
  both shrugging off the knock. It shows: the player's
  `kick` clip (player.ron: `kick_knee`, then `kick_leg` straight out at the
  hip, leaning back, the arms swung against it; 16 fps, once through), the
  blow landing a sixteenth of a second after the key (the leg out), a puff
  of dust at the foot and, when it moved something, a small jolt of the
  camera. `kick` scenario (arena):
  a 360-cell log goes 31 cells and lands whole, a sand heap flies, a TNT
  barrel goes 43 cells, the floor takes nothing. A sim test: a kicked log
  lands whole and further on, a log on a post falls whole when the post's
  foot is dug away, the stone floor doesn't lift.
- **Magic moves objects** (`game/src/magic/well.rs`): force and the well lift
  an object whole, whatever its hardness (they'd tear its cells out one by
  one otherwise). Force shoves every body in its cone to its speed at once,
  as it does creatures (slower past 270 cells: by weight); a well holds the
  bodies within its reach on a spring to its heart (6/s, a fifth of the way
  a tick) against gravity, each weighing 0.7 a cell against what's left of
  its lift that tick. `logmagic` scenario (flat world): the force wand throws
  a 360-cell log 24 cells; the gravity wand lifts it ~60 cells, carries it 75
  over and drops it, and it lands whole.
- The unloaded world is a wall to a body, as to cells (a log flung off the
  loaded area used to fall out of the world). Sim test.
- **Boulders and fracture.** `boulder` is rock in one piece: an object, and
  `brittle` (a new flag). A round one rolls (the solver's friction and spin
  do it). A brittle body (most of its cells) breaks when one knock takes
  more than its hardness / 13.3 cells a tick out of it (a boulder: 4.5, a
  fall of ~27 cells; not a kick, not a roll on the flat), at least 6 ticks old
  and 27 cells big: in two along a line through its middle (one of four
  directions, from its id and age), each half flying off the other at
  0.52 cells a tick; a half under 9 cells is rubble. Rock is never lost.
  A sim test: a radius-8 boulder dropped from 225 breaks (197 cells of rock
  before and after), from 18 lands whole, on a 1-in-2 slope rolls 216
  cells. `boulder` scenario (flat world): one rolls down a steep ramp,
  crushing an orc on the way, and breaks in two at its foot.
- **Traps** (worldgen `TerrainGen::trap_at`, game `traps.rs`): all
  materials, so they're saved with the world and nothing else keeps them.
  A round boulder (radius 6) sits in a niche cut into a tunnel's ceiling, a
  cell clear of any rock all round, held by one cell of `rope` (weak,
  flammable, load-bearing) from rock at the niche's top; under it a
  `tripwire` (a plant: walked through) across the floor or a
  `pressure_plate` in it. A creature touching the wire, or standing on the
  plate (the cell half a cell under its feet), springs it: the wire snaps,
  the rope within 36 across and 90 up is cut, and the boulder, held by
  nothing, falls whole (and, from the tunnel's height, may break). Cut or
  burn the rope and it falls too. Placement is judged in world coordinates
  on the plan's ground, so a trap spans chunks: spots on a 60-cell grid,
  60/256 of them tried (half again in the caverns and the deep), wanting a
  floor 60 % rock over 42 cells, 29 to 52 cells of headroom, a player's
  height of walking room within 14 either side, and rock for the rope: 85
  in the medium world (34 plates). Streaming a column in the caverns costs
  ~0.2 ms more (the judging). Worldgen test: so many, each written as it
  should be. `trap` scenario (medium world): the nearest one found, the
  player walks onto its plate, the boulder drops (and breaks in two), the
  player through before it lands.
- Not yet: bodies aren't saved with the world (in flight: at rest they're
  cells) and don't collide with each other.

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
  - `Burning`: a share of it on fire (0–100 %, on the HUD as a percentage),
    as the coatings are on it. It catches as much as touches flames
    (`Exposure::flames`, the share of its cells in them, one just outside
    counting 0.35): its fire rises at 5 a second towards 5 × that, so embers
    underfoot light ~15 % of a player, standing in a fire all of it;
    heat alone lights what's oily. Spells, lightning and exhaust add a share
    (`catch_fire`: lightning 100 %, a fireball's blast 80 %, a zap 60 %, the
    flame jet 35 % a catch, rocket exhaust 30 %). It only grows from contact
    or oil: on fire, it spreads over the oily part of it (2 a second) and
    burns that oil away (0.12 a second where it burns). Left alone it burns
    down on a curve, slow while it's big and quick as it gutters out: by
    0.07 a second all of it alight, 0.32 a second nearly out (in between in
    proportion; times its oil's `burn`), so all of it lasts ~6 s (over half
    of it still after 3 s), a tenth ~0.3 s.
    What's wet (fireproof coatings: water, blood) takes its place: never
    more of it alight than it's dry, and 40 % wet or hard frost puts it
    out, so a puddle at your feet knocks it down and wading knee deep puts
    it out; falling faster than 210 cells/s the air beats it down too,
    0.004 a second more for every cell/s over (flat out, ~450: a full fire
    out in about a second); where it burns it dries (0.2 a second). It hurts 12 a second all
    of it alight (was 7, all or nothing), less the less of it, times its
    oil's `burn`. It shows as much as it burns: flames (the torch's fire; all
    of it alight, as many as a torch's for every 12 × 12 cells of it) licking
    up from its feet as far up it as it burns, an orange tint, a flickering
    firelight as bright as it burns; and it lights what it stands in (grass,
    a meadow) as often as it burns.
  - `Coated`: what fluids have left on it, Noita's way, from
    `assets/data/coatings.ron` via each material's `coats` (water/snow/ice:
    wet, oil: oily, blood: bloody, acid: acid). Each is a share of it
    covered (0–100 %, shown as a percentage), several at once, never more
    than 100 % together. In a fluid (or touching snow: a cell outside the
    body counts 0.35 of one inside) a coating's share rises at 4 a second
    towards `soak` (1.6; acid 5) × the share of the body in it: a three-cell
    puddle wets a player's feet ~22 %, knee deep ~40 %, waist deep ~80 %, a
    step in acid ~45 %. Rising, it pushes the others off to make room, so
    jumping in water washes acid off; a clinging one (`sticks`: venom) only
    a washing one (`washes`: water) pushes off. Out of it, each wears off
    over its `secs` from full. Its effects scale with its share: heat
    resistance, a fire's length and power, damage (acid 6/s all over, eating
    until it's worn off or washed off; in the acid, the acid's own 30/s
    counts instead); 40 % wet can't catch fire; 25 % oily catches from heat
    alone. Standing in the rain wets you all over, slowly.
  - `Chilled`: slowed down to 40 % while touching the cold and 1.5 s after
    (`MovementStats::slowed`); hard frost puts a fire out.
  What heat, acid, venom and cold do to a creature is its nature (§6.4):
  heat hurts as fire, acid as acid, venom as poison (none to what `cant:
  [poison]`), cold slows by its frost (none to what `cant: [chill]`); what
  `cant: [burn]` never catches fire. What drinks acid (a negative
  multiplier) heals in it.
- Bodies in liquid: water is thick (strong drag) and you're nearly buoyant
  (a tenth of gravity, sinking at ≤ 38 cells/s). Swimming: jump is a stroke
  toward where you steer (W/S/A/D or the arrows; up if nowhere), one on the
  press and another every 0.35 s while it's held (glide between: the
  drag); held, you tread water (no sinking), so ~27 cells/s up or down and
  level sideways. With your head out (under 85 % of you under water) a jump
  is a real jump: out of the water and onto the bank. Powder still falling
  (its fall speed `vy` > 0: a stream of sand) doesn't stop bodies; settled
  powder is solid ground. A body trades places with liquid the way sand does
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
  that ends in or within 3 cells of it charges all of it that's connected
  (up to 13 500 cells, `World::charge`, reported as `Zap::charged`); every
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
- Clouds are a coarse moisture field (6×6-cell texels) over the whole world
  width, in a band of sky above the surface (`ChunkGenerator::cloud_band`),
  not cells: a sky of drifting gas cells would keep every chunk up there
  awake. Stepped every 4 ticks from seed, tick and wind only (plus vapour fed
  from below), with `+ − × ÷` only, so it's deterministic for co-op.
- Each column relaxes toward a cloud shape (flat base, heaped top) set by
  seeded humidity fronts pinned to the moving air; the pattern drifts with
  the wind (~4.5 cells/s at full wind). Fronts come and go over tens of
  minutes, a storm lasts minutes. Above 0.9 moisture a texel rains out.
- Rain and snow are particles, started only over loaded ground: rain (snow
  where the cloud is below 0 °C, melting into rain in air above 1 °C) puts
  out flames and burning cells it passes or lands on, front and back; one
  drop in 100 (one flake in 7) lands as a cell, so downpours make puddles,
  not floods. New drops stop above 40 500 particles in flight, shared evenly,
  so the particle cap never evicts drops mid-fall.
- Steam that fades (rather than condensing on the spot) feeds the clouds
  above it (`vapour: true` in the material): boiled water comes back as rain.
- Creatures under an open raining sky are soaked (`Wet`, fire goes out).
- Rendering: the band is painted in air coordinates (world x minus how far
  the air has drifted) into a texture placed to the screen pixel, repainted
  every 30 ticks or when the view leaves it, so drift is smooth (the field
  itself moves in whole 6-cell texels); a bright rim, light
  body and shadowed belly per cloud; the sky greys as it clouds over.
- Storms: the wettest parts of big fronts rain hard; a column raining more
  than `STORM_RAIN` throws lightning now and then (one in 12 000 weather steps
  per column: a storm over a screen strikes every ~10 s). Lightning comes down
  the column from the cloud base and strikes the first solid, liquid, plant or
  tree. It bursts there (a small blast that shreds leaves, not wood, and
  craters soil) and heats the crown round it. Through a tree it runs on down
  the trunk (following the wood) to the ground: everything flammable within two
  cells of its path flashes alight at 1200 °C, the most a background cell
  holds, spitting flames and embers, so the tree burns from crown to foot at
  once. It blows the top of the trunk off. Where it earths the ground reaches
  1500 °C (sand fuses to glass) and catches. Measured: a struck tree has 26
  of 240 wood cells left after 20 s, one lit at its foot 138. Creatures
  within 15 cells take up to 55 damage and catch fire (unless wet). The game
  draws a forked bolt down to where it earthed, flashes the sky and shakes
  the camera.
- `WorldEdit::Weather` forces a storm or a clear sky over an area (fading back
  over ~2.5 minutes) and `WorldEdit::Lightning` strikes a column: dev V, B
  and N (or F5-F7, or the dev panel), and later spells or events.
- The field steps each column every 4 ticks, a quarter of the columns each
  tick, so its cost (~0.35 ms for the window round the players, ~3 ms for
  the medium world's whole width) is spread evenly rather than landing on
  every fourth tick. The ground under a raining column is looked up once
  every 10 s.
- The band sits 225 cells above sea level, over the tallest trees. It is often
  above the loaded area (zoomed in): rain, snow and lightning enter the world at
  the top of what's loaded below the cloud. Rain or snow is decided by the
  temperature of the ground it will land on. Drops fall at ~3 cells a tick and
  douse a 3-cell strip as they fall (skipped where the chunk is asleep:
  fire keeps its cells awake), until one meets background fire hotter
  than 800 °C: that boils it off (the cell loses 60 °C) and it's gone. So
  rain puts out a spreading fire's cooler edges at once and wears a blaze's
  heart (or a struck trunk) down from the top. Measured in a storm over four
  trees: the struck one burns through, its neighbours 60 cells off don't
  catch, and nothing is alight after 30 s.
- Not yet: weather isn't saved (it restarts from the seed), wind gusts from
  storms, lightning conducting through water and metal.

### 3.14 The world clock (`game/src/clock.rs`)
A slow simulation of the whole world on the game's own time (`Daylight::days`:
the sky's days, skipped hours included). Each process steps at its own pace
and catches up whole steps when time jumps (at most 400 a frame); each has an
abstract face (numbers, anywhere) and a live one (cells, where someone is).
Saved with the world (`WorldFile::clock`; older saves load with none of it).
- **Moisture**, every 10 game minutes, per column across the world (512 of
  them, `Climate::wet`): rain soaks the land (rate 8 a day), otherwise it
  dries toward its humidity (1.2 a day). The humidity is the biome's (desert
  5 %, plains 35 %, forest 50 %, jungle 85 %, swamp and ocean 90 %; the plan
  blurs it across borders) times a dry spell, a noise drifting over six days
  between 0.35 and 1.25. The rain is `Weather::rain_outlook(x, tick)`, the
  cloud the fronts want at x (the live clouds follow the same fronts), so it
  rains on land no one is on. In the sim, `living` materials (grass, tall
  grass, leaves, needles, dark leaves, moss, wood) catch the wetter they are:
  ignition 60 °C higher soaked, the chance of catching each moment 85 % lower
  and of flames reaching them 60 % lower. Land nobody set is dry (fire as
  tuned). `forestfire` (`PLATYPUS_WET` pins it): 25 s after one crown is lit,
  of ~15 000 cells of forest about it, 5 % wet 4 200–9 900 burnt, 20 %
  4 900–6 700, 35 % ~4 400, 50 % 3 800–5 300, 80 % ~3 800–4 000: wet land
  slows a forest fire, it doesn't stop one (so before the 1.5× rescale too:
  45 % of the forest dry, 35 % at 80 % wet).
- **Regrowth**, hourly and as chunks load: nature heals toward the pristine
  chunk (`ChunkGenerator::heal`, the chunk made again from the seed). Soil
  where there was grass or moss turns back between 0.1 and 1 day after it
  was hurt (each cell at its own moment); tall grass on bare grass between
  0.6 and 1.6; ash and charcoal lying where there was air or grass go between
  0.3 and 2 (in the background, 0.5 to 3). A tree as the seed made it, hurt
  a little, mends between day 1 and day 3. One mostly gone (at least 27 of
  its cells in a chunk, under 60 % of them standing) is lost: what's left of
  it is cleared, and the clock notes when (`TreeRecord`). A new tree of its
  kind (its look from `(seed, x, time)`) is a sapling a day later (8 % grown)
  and full grown by day five. Only nature's cells are touched: in the
  foreground those rules, in the background air, tree cells, ash and
  charcoal; never what was built.
- When: every chunk loaded from the store heals at once, catching up however
  long it was away (a chunk's healing counts from when it was put away if
  the clock hadn't noticed its hurt before); one made anew heals if a lost
  tree is near. A chunk away an hour or more comes back with its fires out
  (flames and smoke gone, burning cells put out, heat as it was made), so a
  fire frozen in the store can't burn the regrown forest. Every game hour
  the loaded chunks that were changed, are healing or have a lost tree near
  heal, four a frame (~0.2 ms each). Growth is gradual, so it happens in
  view too; clearing a lost tree's remains waits until it's out of view.
  Trees are judged (found lost) only on the hourly pass when the hour before
  was healed too: after a jump in time a regrowing tree would look hurt
  before it's drawn at its new size.
- Records: `trees` (x → when lost, which time), `land` (chunk → since when,
  cells left); a reset forgets both. `regrow` scenario (a forest set alight
  at 2 % wet, an hour, away for `PLATYPUS_DAYS`, back): forest cells before,
  burnt, after 0/1/5 days: 15 207, 7 000–11 000, 3 300–6 800/~5 000/
  19 000–34 000; ash and charcoal ~2 000/~1 250/~350; burnt soil ~130/1/3.
  `PLATYPUS_STAY=1`: the player stays, an hour passing every tenth of a
  second: after 5 days 31 000–37 000 forest cells, grass 480–560 of 576
  (healing in view, hour by hour).
- **Snags**: a lost tree's trunk stands charred (charcoal, a third to two
  thirds of its height, its shade halved) until the one in its place is
  30 % grown; then it goes as any charcoal does. Drawn, like clearing a
  lost tree's remains, only where no one is looking.
- **Wildfires**, hourly (catching up): lightning in dry forest where no one
  is (no player within 3 000 cells). Each column of land drier than 30 %
  has a chance an hour of 0.0004 per 384 cells of width at tinder-dry,
  falling off linearly to none at 30 %. The fire walks out from the strike
  through the trees (as the seed made them, `trees_between`) while they
  stand within 120 cells of each other, up to 120 cells (damp) to 900
  (tinder-dry) either side; young trees don't carry it. The trees it takes
  are lost (as above); the land from the first to the last is scorched for
  five days (`Wildfire { x0, x1, at }`, `Healing::scorched`): grass as soil,
  tall grass gone, ash on half the cells over grass, each until its moment
  to heal as a burn would from the fire's day, so the scar is the same
  however often it's healed and comes back on the same schedule. Logged.
  `wildfire` scenario (a strike 3 750+ cells east of the start, the player
  there `PLATYPUS_DAYS` later): 1 tree taken (the land there 44 % wet); at
  0.25 days a charred snag over 135 cells of burnt soil and 70 of ash; at 2
  a young forest, the grass back; at 5 grown.
- **Lairs refill**, hourly: a keeper alive anywhere keeps its record's day;
  one gone three days is back where it was put (the spot out of view).
  `refill` scenario (the nearest spider nest's keepers killed, away four
  days, back): exactly the seven that were killed come back; the other
  lairs, their keepers alive, are left as they are.
- The dev readout says, for the cursor, how wet the land is, a wildfire's
  scar there, whether its chunk is healing (cells, days in) and how far a
  lost tree near it has grown. "A day ahead" in the dev panel skips 24 hours.
- Not yet: lairs spreading into the caves beside them, the weather's fronts
  saved, rain on regions no one's in putting out their fires.

### 3.15 The world's events (`game/src/events.rs`)
DESIGN §13 item 4, PLAN L1 `world-events`.
- A timetable rolled from the seed and the day (`events::roll`): each whole
  day, each kind's chance, and when and where it happens. Rolled a day
  ahead; a new world from its first day. So what happens is the same
  however the time passes, watched or skipped.
- When an event's time comes: someone within 1 050 cells (across) and it
  happens **live**, through the simulation; no one there, or its time
  passed in a skip (more than 0.05 days ago), and it happens **away**: it's
  put into the land when its chunks load (beyond the screen's edge), quietly.
  What happened is kept 10 days (`WorldClock::events`, saved with the clock).
- **News**: what happened in the last two days, newest first (`News`); the
  villager talking to you says the latest first, then its own lines ("A star
  fell last night, a short walk west of here. Something keeps it."). Where:
  west or east of the village, and how far a walk (~5 400 cells a minute).
  Near you, a line on screen too (the progress toasts).
- **Falling stars**: 35 % of nights, between 21:00 and 05:00, 1 350–6 000
  cells from the spawn either way. Live: a streak in from high across the
  sky over 1.4 s (a glowing head that lights the land, a trail of blue
  sparks), then a blast where it lands (radius 24, power 150: the flash, the
  boom, the shake, flying debris that can hurt from far off). Away: the hole
  dug and its rim scorched, no blast. Either way: a meteorite (radius 6, 14
  cells down) and mithril ore beside and under it, the crater heated to
  700 °C (it glows; grass round it may catch), and two star wisps hovering
  over it. Seen from the surface by night, a star falling far off streaks
  across the sky toward it and is gone ("A star falls, far to the west").
- `meteorite`: dark, glassy, flecked with light, a cold blue glow, hardness
  95 (an iron pick).
- Star wisps (`star_wisp.ron`): small white-blue stars, fireproof, on the
  `hunter` brain, `Swoop`, with a `leash` (120 cells round their crater, kept as their
  `clock::Keeps`, so a save keeps it too): they dive at you only near it and
  drift back when you go. They don't come back once killed. 20–45 gold.
- **Raids**: 30 % of evenings (17:00–21:00), on the village, from the west
  or east. Live (someone within 1 050 cells of the village): the `raiders`
  pack (packs.ron: three orcs, an archer) comes in 495 cells beyond the
  middle of the view on its side, out of sight, `ai::Marching` on the
  village's middle (with no one to fight, a marcher walks there instead of
  wandering, and stops); "A warband is coming from the east!". Away: five
  holes (radius 6) knocked in the houses as the village's chunks load
  (what they held up comes down too). News: "Orcs came at us from the east
  this evening. We're mending what they broke."
- The village mends toward what the seed made (`events::mend`): its
  walls, roofs and floors (planks, brick, platforms, front and back) where
  there's nothing now (or ash, charcoal, rubble), from the bottom up, out of
  view (420 × 255 cells round a player), chunk by chunk as each is in
  (hourly, just after a raid, and as one loads: coming back, what's due is
  done before it's in view). An allowance of 90 cells a game hour accrues
  whether it's loaded or not (a raid starts it again); paid for, all of it.
  What's been built over stays. The guide offers "Mend the village by
  morning" for 60 gold while there's mending owed (`talk.rs`).
- Villagers are keepers (`Spawn::Keeper`): one killed comes back three days
  later, out of sight. A villager with a monster near runs home and hides
  there (`villager::Hiding`): monsters let it be (their targeting skips it)
  till the danger's gone.
- **Earthquakes**: 12 % of days, any hour, their heart 1 200–9 000 cells
  from the spawn either way; felt (live) within 4 500 cells. Felt: the
  screen shakes for 5 s (camera trauma 0.55, easing off over the last
  second) with a low rumble ("The ground shakes!"), and every 0.12 s a spot
  round each player (255 across, 195 down) is tried: open air under a cave
  ceiling, underground, and that ceiling comes down as rubble (a `Shatter`
  thrown down, radius 3–5). At its heart, felt or not, a chasm opens in the
  surface as its chunks load: a jagged crack 90–165 cells down, 12 wide at
  the top narrowing to 2 (from the seed and the day). News: "Did you feel
  the ground shake today? They say it split open, …".
- **A travelling pedlar**: 25 % of mornings (08:00–11:00), staying a day.
  While it's staying and someone is within 1 050 cells of the village with
  no pedlar about, one is made: at the village's middle if that's out of
  view, else walking in from 495 cells beyond the middle of the view on its
  side (its `Home` the village's middle). Its stay over, it walks off that
  way and is gone once out of view. A villager (`pedlar.ron`) with things
  from far off: rocket and cloud boots, a grappling hook, charms, a mithril
  pickaxe and bars; it doesn't buy. News: "A pedlar came to the village
  this morning. Off again tomorrow, he says."
- Seasons: dropped (the biomes carry the climate: the tundra snows, its
  lakes are frozen).
- The world generator gives the village's bounds (`ChunkGenerator::village`).
- Dev panel: "An event (star, raid, quake, pedlar)": each in turn, now (a
  star 210 cells ahead; a raid on the village from the side you face; a
  quake whose heart is 300 cells ahead; a pedlar). Scenarios `star`,
  `raid`, `quake`, `pedlar`.

## 4. Rendering

- One texture + one sprite per loaded chunk (~150 entities on screen, not
  ~600 000). A chunk re-colours and re-uploads only when its cells changed.
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
- **Air jumps** (`creatures::AirJumped`) puff a small cloud under the feet
  (sparks) and flash a soft blue light, so they show in the dark. The player
  has 3 while developing; later gear gives them (a cloud in a bottle).
- **Hurt** (`creatures/body/hurt.rs`): anything with `Health` that loses 2 or more
  in a tick flashes red for 0.12 s and, if it bleeds (`blood` in its RON,
  default `blood`), sprays 2.7 cells of it a point lost (at most 158); a
  death bursts out 248. Real cells: it pools, runs, boils, freezes,
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
  pays only for reading the world (~1.5–2 ms at 2 px/cell).
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
  opacity), walls behind rock fully. The rays lie on a lattice fixed in the
  world (from a corner every 64 texels), not on the light grid that follows
  the camera, so a lit edge stays put as you move (on the grid, climbing or
  walking made the sun's edge on slopes and crowns flicker).
- On the ground the camera's height follows the player's through a
  critically damped spring (0.1 s, never more than 9 cells behind): stepping
  up a hill snaps the body up a cell or two a tick and pauses, and a camera
  on it lurched with every step. In the air it moves with the player from
  the first frame (a spring there sat still for frames as rocket boots took
  off), what it was behind closing in 0.05 s. Across it follows exactly.
- The camera snaps to whole screen pixels a quarter pixel off the halves: a
  body at rest stands half a cell up, a tie at 3 pixels a cell, and float
  noise flipped it (the world hopping a pixel against the sky).
- Light sources: anything with a `LightSource` (colour, flicker) lights its
  surroundings: planted torches (G, the torch item), thrown glow sticks (tool
  7, green and blue in turn, 90 s, fading), later lanterns and glowing eyes.
  Fire (`flicker` 1) sways, flutters and jitters (about 0.75–1) and reddens
  as it dims (green by f^1.5, blue by f²). The player carries a lantern
  always, faint (0.08: underground it shows you and the rock beside you,
  little more; you need a light), and L steps through nothing, a small flashlight, the big one and a
  torch in the off hand (at the back arm's hand, `back_arm.hand`).
- Torches (`light/torch.rs`, `assets/art/torch.ron`: its `flame` and
  `grip` anchors) burn with a look only (`lighting.ron` `fire`: flames of
  rising motes shrinking from white to red, a wisp of smoke, an ember now
  and then; a second's worth each), so a torch sets nothing alight. Its
  light is a warm white (255, 208, 152) × 2.4, as bright as the first
  orange (255, 158, 70) × 3.0 (the same luminance) without its sepia cast,
  hazing only 0.15 of it over the dark (was 0.35: glare); it flickers as
  before.
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
  frame by itself. The player (`assets/art/player.ron`, ~33 px tall on its
  9 × 23 body, the head may overhang it) is drawn this way: a head (and a
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
  above 12 cells/s and off below 4.5), so steps and slopes don't flicker
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

### 5.2 Critters (`creatures/brain/critters.rs`)

- The `critter` brain (params: `flee_range`, `calm_after`, `hops`, `flies`,
  `wander_every`, `rest`, `wander_speed`, `hovers`, `swims`): rests and
  wanders when calm; flees a player within `flee_range`, a blast within 4 ×
  its radius, or being hurt, for `calm_after` seconds, away from it.
  Hoppers move in hops (pausing between them when ambling); flyers fly up
  and away when scared and glide back down to land when calm. Walled, it
  turns round. `hovers: (low, high)`: a flier that never lands, wandering
  in two dimensions and kept between those heights above the ground or
  water (fireflies; bats, with a `wander_every` of 0.35 s: flitting).
  `swims`: wanders in two dimensions in water, turning back at its edges
  and staying under its surface; stranded, it flops. (Optional brain params
  like `hovers` use `data::some`: brain params are read from a RON value,
  where `Some` can't be left out.)
- Movement for them: `fly_speed`/`fly_accel` (fliers), `swim_speed`/
  `swim_accel` (swimmers: steered and weightless under water).
- A creature file's `light: (color, strength, pulse)` makes it glow: a
  `LightSource`, with `pulse` a `Glow` (swelling and fading, each out of
  step), and glowing creatures seed the light grid's `emit` as glowing
  cells do (light and a haze over the dark).
- Ambient life (`assets/data/life.ron`, hot-reloaded): for each kind, where
  it lives (`Surface`: open ground; `Shore`: within 45 cells of water;
  `Water`: in cool liquid with room round it, anywhere near; `Cave`: the
  open air of a cave, a roof above, 45+ under the surface), `when` (`Any`,
  `Day`, `Night`: out of its hours it leaves once 390+ cells off), `above`
  (spawned that high over its spot), at most how many within 675 cells of
  the player, and a chance a second: a surface one turns up 495–660 cells
  to either side (just off the screen) on the first solid ground under open
  air near the surface; water and cave ones anywhere near (16 tries); any
  further than 1 200 cells is gone.
- Rabbits and birds (by day), frogs (by water), fireflies (at night over
  the ground, pulsing), fish (in lakes), bats (in caves).
- Enemies come and go the same way: a haunt's `depth` (under the surface)
  and `zone` (underground biome) keep slimes in the upper caves, acid
  slimes in the toxic grottos, spiders, skeletons and vampire bats deeper;
  a cave spawn is never nearer than ~270 cells (no popping in). Only kinds
  in life.ron are taken away when far.
- Pelting: a liquid hurts only above 375 cells/s (not 135), so a death's
  burst of blood doesn't hurt what's beside it (a burst egg sac hurt its
  own spiderlings).

### 5.3 The arena (`PLATYPUS_WORLD=arena`; `worldgen/src/arena.rs`, `game/src/arena.rs`)

- A walled sandbox 1920 × 960 cells, open to the sky, one floor at y 160
  with, left to right: stairs (7–8-cell steps), a ledge and a ramp; a water
  pool (165 × 75); two one-way platforms; the open floor where the player
  starts (x 960) with three training dummies and a sandbag; a lava pit; a
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
- **Fight** (the panel's second section; `fight.rs`): the fight as it
  goes, from every `Took` (blows, spells, burning, falls; the dummies'
  too): its length, what's been done **to them** (everyone but the
  player) and **to you**, each with damage a second (a single blow: over a
  second) and by kind, most first, as it landed (`slash 120`, `pierce 4
  of 12`: shrugged off, `void drank 3`); and a timeline of the last 20 s,
  a pixel a tenth of a second, to them above the line and to you below,
  each kind its colour (`hurt::color_of`), a tick a second. A fight runs
  from its first hurt until 5 s pass with none (the next starts afresh);
  "New fight" starts one now. Which sections are open is remembered by
  name, shut ones too (`-Name`), so a new section starts as it's meant to.
- **Layouts** (`worldgen::arena::Layout`; `PLATYPUS_ARENA`, or the panel's
  Layout section: the generator swapped, the world reset at once,
  `reset::ResetNow`, the player kept): `sandbox` (the floor above), `flat`,
  `cave` (a tunnel 70 cells high, its roof and floor wavering, three
  pillars), `slopes` (rolling hills, ±58 cells), `stairs` (8-cell steps up
  from 300 to 780 and down from 1020 to 1500, a ledge 96 up, platforms),
  `real` (the real world's chunks, copied whole from a seed and place,
  `real:SEED:X:Y`, by default seed 1's start: that place is the start;
  "fight it here"). Every layout has the same walls and the bestiary's
  stage; dummies stand on the ground (none in the real world's). Tested:
  each walled, staged, a start to stand on with room above.
- **Recorded fights** (`replay.rs`; the Fight section's Record, "Stop and
  save", "Replay the last"): recording resets everything first (the
  layout's world, a new player, tick 0, creatures numbered from 1), then
  keeps every frame (its length, the time's speed and pause, play's keys
  and mouse buttons held, not the arena's, dev's or a screen's, none while
  the pointer's on a panel, the cursor's world point) and each creature
  put down (O, the bestiary's Place); saved with how it ended (the
  player's health and deaths, who's left with their health, the readout's
  totals) to `saves/fights/fight-N.ron`. Played back (`PLATYPUS_REPLAY=`
  file, headless: exit 0 if it ended as recorded, 1 with how not): the
  same layout and reset, each frame the same length
  (`TimeUpdateStrategy::ManualDuration`, the fixed clock's leftover as it
  was), the same input and cursor, the same creatures put down; matched if
  the same are dead and health and the totals are within 15 %. A
  creature's rolls come from its number (`creatures::Stable`), not its
  entity id (which depends on everything made before, menus too). The
  `record` scenario's fight played back twice: the player at 89.4 hp both
  times, the orc dead; a doctored ending fails with exit 1. Don't step
  (.) while recording (a step isn't kept).

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

### 5.5 Villagers and talking (`creatures/brain/villager.rs`, `talk.rs`, `assets/data/creatures/{guide,smith,healer,merchant}.ron`)
DESIGN §13 item 6.
- A villager is a creature file: its look on the player's rig (art `base:
  "player"` with its own palette: `parse_based`), team `Villager` (the
  player's ally: neither hurts the other; monsters hunt both), and the
  `villager` brain, whose params say who it is: `role`, `lines`, `wander`
  (cells from home), `wander_speed`, `flee_range`, `sells` (item id, gold
  each), `buys`, `heals` (gold for a full heal). New people are new files.
- Its day: home is where it was first put. By day it potters within
  `wander` of it, a new spot every 3–9 s; from 20:30 to 06:30 it walks home
  and stays. A monster within `flee_range` (165) and it runs home and hides
  there (monsters let it be) till it's gone. Health 140, quick (102). A
  player within 54 cells and it stops and faces them (`Routine.talking`).
- The talking villager nearest the player says its lines, one every 4.5 s,
  in a bubble over its head (world text, shadowed, wrapped); walk off and
  back and it goes on to the next.
- Right-click one within 54 cells: its panel opens under the pack (and the
  pack with it). Rows of what it sells (icon, name, price; red when out of
  reach): click buys one, Shift-click ten (as many as the gold and the pack
  allow; blocks by the block). The healer's first row heals you whole. The
  merchant's slot takes a stack dropped on it (let go over it, or clicked
  down on it): half what anyone in the village sells one for (at least 1,
  × (1 + rarity) for gear), a gold apiece for what no one sells, nothing
  for plain blocks; held over it, the slot says what it would fetch. The
  guide's panel lists its tips. The panel shuts when the pack does, when
  you're 81 cells off, or when it runs. Clicks are `Trade` messages.
- Today: the guide (tips), the smith (pickaxes, swords, armour, ladle,
  bucket, firebrick), the healer (a heal for 15, potions), the merchant
  (torches, glow sticks, bombs, arrows, chests, planks, platforms, flasks;
  buys).
- Scenarios: `village` (a walk through by day or night, `PLATYPUS_HOUR`),
  `shop` (a trade with the smith, the healer and the merchant).

### 5.6 The bestiary (`game/src/bestiary/`, DESIGN §14.7)

- **The catalogue** (`bestiary/mod.rs`): every creature file but the
  player's, read from what the game has (the files, `kinds.ron`,
  `moves.ron`, the art), nothing written twice: name, kind, part (foe:
  enemies; villager; critter: the `critter` brain; thing: the rest), box,
  health, speed, poise and heft, brain and code, weapon, touch, moves
  (range, how often, phases, what they do), the kinds of hurt it doesn't
  take as anyone would, what it can't suffer, healing, loot. Its picture:
  its idle clip's first frame, trimmed. Tested: every creature has a card,
  a picture and its file.
- **The panel** (F12; the arena panel's "Bestiary"; `panel.rs`): over
  everything, with the keyboard while it's open. Search (typed), part and
  kind filters, a card each (picture, name, kind, health, "code" if it has
  its own); one opened on the right: its picture (or the live stage),
  Place (90 cells in front of the player), Fight (placed, the Fight
  readout afresh), Open file (`open -t`), Reload (`ReloadCreatures`:
  creature and move files read again, changed or not), Back; its details
  (Body, Hurt, Attacks, Carries). Esc clears the search, then the card,
  then closes.
- **The live stage** (`stage.rs`; worlds with `ChunkGenerator::stage`:
  the arena's, a room 330 × 156 sealed in bedrock at its top right, a
  stone floor with a 9-cell step): the opened creature on the real floor
  with a stand-in (a dummy on the hunted side, never dying), seen by its
  own camera into the card (2 px a cell, 660 × 312; its room kept loaded
  while it's on; the legs' canvas drawn for its view too). Its brain
  taken off (`BrainRegistry::remove`, `Staged`: not cleared as a far
  critter, moves started only by the stage), it's driven round and round:
  standing 1.2 s, walking 1 s there and back, two swings or a shot of
  what it wields, then each of its moves (`Moves::force`; the stand-in put
  at the middle of the move's range; never mid-swing). What a move can hit
  is drawn over it (`Moves::shapes`: strikes red, grabs violet, slams
  orange; faint while coming, bright while live; everyone's with the
  arena's overlays on). The caption says what's showing. The room is made
  afresh for each creature; one taken away by a reset is put back.
- **Live reload**: a creature file changed (or Reload) reaches the
  creatures of it already about: stats, health and profile, art, healing,
  poise, touch, weapon, moves (if the list changed) and brain (put on
  again from the new settings); not the player's, nor a staged one's. The
  `reload` scenario takes a troll's moves, brain and poise away and lowers
  its health cap; after Reload it has them all back.
- **`platypus-bestiary [dir] [creature ...]`** (`cli.rs`, the launcher in
  `src/bin/`): the game run headless in the arena (`PLATYPUS_BESTIARY`),
  the clock a tick a frame; for each creature its card opened: its
  details into `bestiary.md`, the panel as `<id>_card.png`, the stage
  caught 0.05 s into each thing it shows, put together as
  `<id>_strip.png` (half size, four to a row, numbered in the markdown).
  Every creature (29) in about 2 minutes.

### 5.7 Finding the way, and digging (`platypus_nav`; `creatures/brain/way.rs`; DESIGN §14.4)

- **One planner for every creature**, its abilities as data
  (`Profile`): its box, run speed, step, the furthest it'll drop (its
  fall damage's safe height; 150 without), gravity, whether it climbs
  (`cling`), flies, swims, digs; made from the creature's movement at
  the game's tempo (again when the tempo or its file changes).
- **The grid** (`tile.rs`): a node is 4 × 4 cells, a tile 16 × 16 nodes
  (a chunk). For one size of body a tile says, per node, the floor's
  height in it (to the cell), whether it fits in the open, is in liquid,
  can hold on (a wall beside or a wall behind, `backed`), and per cell how
  much room there is upward; all from one pass of column reads through
  the same occupancy the bodies collide with. Tiles are made when a
  search first needs them (about 65 µs) and forgotten when their chunk's
  cells change (the sim's `nav_dirty`; at most every 15 ticks a chunk, all
  at once).
- **Moves** (`moves.rs`): offsets in nodes with a cost in seconds: walk
  (up what it steps), drop (no further than is safe), jump, climb (round a
  corner by its open side), fly, swim (everyone; non-swimmers slowly),
  dig. **Jumps are the real physics**, run once per profile in an empty
  room: standing and at a run, the key held three ways (to its
  `jump_hold`, half, a tap); each way's whole arc kept (`Arc`), and on it
  where it can come down (quickest and highest to each) and, for a
  climber, what it can catch hold of. From a node each arc is played over
  the cells once a way, a tick at a time: stopped across by a wall it goes
  on up; the first floor it comes down onto is where it lands (not onto
  an edge: the ground goes on a node past it); a climber catches what it
  passes. A jump costs 0.2 s more than its time (0.5 at a run): walking
  where it can. Each node's moves are worked out once and kept (by
  profile), forgotten with the tiles round them; moves a node rules out
  (no jumps from inside the ground or from the air) aren't tried.
- **Searches** (`search.rs`): A* (`find`; `Search` can be put down at a
  deadline and taken up next tick: the same way as in one go), with a
  budget of nodes; out of it, the way to the nearest node it got to. A
  `field` (reverse Dijkstra from a target) for many of one profile.
- **Following** (`Ways::steer`, from the hunter's walk, range, hop and
  crawl when the straight line won't do, swoopers when far or out of
  sight): the next step turned into key presses: walk toward it; at a
  jump's take-off (the node's middle; a standing jump stops first) jump,
  the key held as the arc was; climbers press into their wall, steer by
  the next few steps and go over a lip; flyers make for the nearest step.
  Planned again when what it's after moves 3+ nodes, the way's off its
  route, it's made no headway for 75 ticks (not while digging), or every
  60 ticks if the way doesn't get there; a way that doesn't get there is
  cut before a drop it couldn't come back up from. A tick spends no more
  than 2 ms planning (everyone's); a search cut short goes on next tick,
  the old way followed meanwhile. `PLATYPUS_NONAV=1`: no planning (as
  before, to compare); `PLATYPUS_WAYLOG=1`: each way and jump logged; the
  arena's overlays (Y) draw every way (walking white, jumps gold, drops
  blue, climbing green, flying and swimming cyan).
- **Digging** (a creature file's `dig`: `claws: (hardness, rate)`, `acid:
  (hardness, rate, every, material)`): `Digging::secs` is one cell's time
  (claws for what's no harder than they take, `hardness/20/rate` s; acid
  for the rest no harder than it, nothing `inert`, `max(hardness,10)/20/
  rate` s), the same for the planner and the digger. A dig move goes into
  a node it doesn't fit, its cost the cells of its room there not already
  dug from where it comes (a tunnel goes on a slice at a time: per-column
  sums kept per tile); out of its tunnel into the open costs nothing. The
  `dig` system: a creature whose next step is a dig takes the cells of
  its room there (a cell to spare round it), nearest first, each as long
  as `secs` says; claws throw a pinch of what they scrape (as what it
  crumbles into) with a scratch (`mine_dirt`/`mine_stone`); acid spits a
  few drops of its material at the face every `every` s, hissing
  (`pour`). At most 400 cells a tick, everyone's. The cave spider: claws
  (25, 30), acid (90, 40, every 1.6).
- **The course** (`Layout::Course`, `PLATYPUS_ARENA=course`): left to
  right a wall 27 high, a pit 40 deep, a step 22 high into a tunnel, a
  tower with a passage under it and stairs down from it; the player on the
  tower. The `course` scenario (`PLATYPUS_KIND`, default the orc):
  an orc (24 s), a skeleton (26 s) and the cave spider (22 s, up the tower's face) get to the player, a vampire bat (9 s) and a star wisp (14 s) fly to it; a slime (its 32-cell step is past its 33-cell jump) and the troll (the 27-cell wall past its 24) can't and wait as near as they can; with `PLATYPUS_NONAV=1` the walkers stop at the first wall. The `dig` scenario: the player sealed in a shell (`PLATYPUS_WALL`, 34 cells thick at the sides) round a pocket, a cave spider 200 cells off: through dirt it's in at 29 s, through stone at 60 s (acid), never through obsidian (harder than its acid) or glass (inert).

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
  bounces the swinger up by `pogo` (a share of its own jump's height) and
  gives back its air jumps and air dash.
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
- **Enemies fight** with the same swings: the `hunter` brain's `Swing`:
  `reach` (swing at a player that close), `combo` (moves of its weapon in
  a row) and `every` (the wait after an attack ends, ±30 %); it
  faces its target, stands its ground while swinging and doesn't swing
  while stunned. The orc (humanoid rig, a cleaver: hack, backhack; windups
  0.26 / 0.18 s; poise 16) and the troll (a rig of turned limbs, 23 × 51,
  420 hp, a club: smash, sweep, 30 damage, 0.5 / 0.4 s windups to read and
  dodge; poise 80, heft 4). In the `fight` scenario the orc lands a hit,
  the shortsword kills it in ~4 s; the troll's smash takes 29 and throws you.
  The troll heals (§6.4 `regen`): steel brings it to 1 hp and no further;
  fire or acid stop its healing, and then it dies (the `troll` scenario:
  held at 2 hp by the sword, dead 0.1 s after it's set alight).
- **Everything held shows** (`weapons.ron` `held`, by icon shape): an item
  whose icon's shape is listed is drawn from the sprite of that name
  (`assets/art/pickaxe.ron`, `axe`, `wand`, `staff`, `torch`, `bomb`,
  `glowstick`: pointing right, a `grip`), in the item's icon colours with
  `recolor` (one pickaxe drawing, four tiers). With a `swing` it's a blade
  by its item id: a pickaxe or axe swings (queued, no stamina) whenever
  it's used, can hit what's in the way, and mines as before. Otherwise it
  rests at `rest` degrees and points where it's used (`Aiming`: a wand
  casting, a bomb thrown, a torch planted); `burns`: a held torch has its
  flame (turned with it) and light. The player wields whatever its hand's
  slot holds if `weapons` knows it (a weapon by its weapon id, anything
  else by its own). Swing targets must have `Health` (a pickaxe digging
  down once pogoed off the drops of what it dug).
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
  through lava burns, and the archer costs ~17 hp in 4 s (measured before
  the rescale: since, the scenario's bow looses nothing, a known bug).
- The shortsword (12 damage: slash, backslash, thrust) and the longsword
  (24: cleave, sweep, drive; slower, heavier on stamina). In the `melee`
  scenario: the shortsword lands 4 hits (53) in 0.7 s; the longsword
  empties the stamina bar in 1.5 s; striking down from above bounces
  (389 cells/s); a blast mid-dodge costs 0 hp against ~12 standing.

### 6.3 Underground enemies (`creatures/brain/hunter.rs`, `assets/data/creatures/`)

Each is a creature file (art, stats, brain and its params) on shared
parts, so another is a new file, not new code:
- **`touch`** (`combat::Touch`: damage, knock, stun, `every`): hurts what
  it touches of another side (not its own; critters touch nothing), then
  rests. Every hit is a `Hit`; a target given its `after_hit` grace ignores
  the rest of that tick's hits too (four spiderlings' bites don't land at
  once).
- **`cling`** (movement): a climber touching a wall or ceiling holds on
  (pressing into it), goes along it where it steers, and lets go on a
  jump; the sprite is turned to the surface (upside down on a ceiling).
- All fight with the `hunter` brain (§6.4), closing with `Crawl` (at you
  over any surface, a pounce when near, a drop from a ceiling above you),
  `Hop` (a hop at you every `every`), `Swoop` (flits in a band above the
  ground; dives at you for `dive_time`, back up every `dive_every`) or
  `Walk` with a `Swing` for anything that wields a weapon. The egg sac is
  its own code (`custom/hatchery.rs`: still until you come within `range`
  or hit it, then bursts into `count` of `brood`).
- **Procedural legs** (`creatures/body/legs.rs`, a creature file's `legs`): a body
  seen from above (its sprite, pointing right, a `grip` where the legs
  meet) turned with RotSprite to where it heads, and `count` legs fanned
  front to back on both sides. Each foot holds a real solid cell: rays
  from the hip swept ±90° round the leg's way, the hit nearest 70 % of its
  `reach` preferred (legs stretch along walls, not bunched on them). A
  foot stays put while the body moves and steps (an arc off what it holds,
  `step` s, `lift` high) once stretched, crowded or twisted, never beside
  a stepping neighbour, a third at most at once; with nothing in reach a
  leg reaches out into open air and twitches. Two-bone IK: knees bend away
  from what the feet hold (never into rock when the other bend is open).
  Legs are drawn a cell at a time (Bresenham; thighs `thick`, shins one
  less) under the body; `eyes`: the body's pixels of that colour drawn
  again over the darkness (full bright in the dark). Looks only: the
  body's movement is its own (`cling`).
  **From the side** (`view: Side`; BE `limbs`): a body seen from the side
  (pointing right, mirrored to face left), each leg in `each` with its
  `hip` (cells from the body's `grip`, y up), where its foot rests
  (`lean`: ahead of the hip, behind if negative), its `gait` group and
  whether it's `far`. Its feet go by a gait clock: a cycle as the body
  covers `stride` cells over the share of the cycle a foot's planted
  (68 % walking, down to 42 % at 150 cells/s: a runner has both feet up
  a moment), each gait group its share of a cycle after the last; a foot
  is planted for that share, then swings forward (an arc `lift` high) to
  the ground under where it rests, half a stride ahead (nearer the hip
  if that's past its reach: down a steep slope), kept up to date as the
  body goes. Standing, the clock stops unless a step's to finish or a
  foot's been left behind. A foot left past its reach is put down again
  at once; off the ground a moment its feet let go and tuck under it; a
  free foot is never further than its leg reaches; legs are drawn as
  solved (past its reach, at full length, never stretched). Moved
  further than a leg reaches in a frame (put down, a portal), its feet
  are planted afresh and its ride starts over; its first frame reads its
  body's place, not its transform (still at the origin then). The body rides
  `ride` cells over its planted feet (no lower than half that over its
  box, no higher than the box's top) and tilts with them, up to `tilt`°:
  the slope of a line through its planted feet (along the way it faces;
  feet together, nothing to read: it eases level). `bob`: it rises that
  many cells while a foot's in the air (two legs); `pitch`: it leans
  that many degrees nose down at 100 cells/s (a runner). A leg's `knee`:
  `Out` (up and out: a front leg's forward, a back one's back; crabs,
  insects), `Forward`, `Back` (a bird's: what bends is its ankle).
  `width: (hip, knee, foot)`: a leg drawn as a limb with some flesh to it,
  tapered from its hip to its foot (discs stamped along each bone), a
  cell of `outline` round it (else its colour much darker) and a knuckle
  at the knee; `toes`: a foot along the ground ahead and a claw behind
  (the raptor's thighs 8 to shanks 3, toes 7; the crab's armour 4.5 to a
  point; the stalker's stilts 3 to 1.2). Without `width`, lines `thick`
  across (the spider's). `ankle`: a third bone (a bird's: from its ankle
  down to its toes), held `heel`° up from the ground behind the foot, the
  knee between hip and ankle (the raptor: thigh forward, shank back, foot
  forward; `width` then four: hip, knee, ankle, foot). `Up`/`Down` bends
  too. `arms`: limbs that don't walk (a crab's claws, a raptor's little
  arms): a `shoulder` and where the `hand` rests (from the grip), two
  `bones`, a `width`, an `elbow` bend, swaying `sway` cells at rest, a
  `claw` sprite (pointing right, its grip at the wrist) turned to the
  forearm; near ones in front, far ones behind. The crag crab is seen from
  the side: eight legs, its pincers held out ahead. `footfall`: what a
  foot does coming down (from the side): a `sound` of its own (`stomp`,
  `clank`) and the ground's footstep (`ground`), a `dust` puff the
  colour of the ground (`World::puff`: dust that fades as it flies,
  nothing lands), screen shake (`shake` trauma at its foot, less further
  from the camera, none past 400 cells). Knuckles a quarter of the limb.
  The ridge tyrant (tyrant.ron): a T-rex at a T-rex's size (150 cells
  nose to tail, hips 50 up: twice a person), its art made by a script
  (`tools/tyrant_art.py`: shapes, shaded from above, written as text
  art); legs 46 + a foot bone 14, thighs 20 thick; it walks at 56
  cells/s, each foot down most of a slow stride; stomp, dust, shake.
  `style: Plate`: metal legs (straight plates as wide as `width` at their
  start, shaded under; bolts at hip, knee and ankle; a piston from thigh
  to shank; a flat foot plate `toes` long, landing level). The iron
  strider (strider.ron, art by `tools/strider_art.py`): a hull on two
  plated legs bending back at the knee, its hull 40 over its feet with no
  bob, high quick steps, each foot clanking down.
- **Chains** (`creatures/body/chains.rs`, a creature file's `legs`'
  `chains`; BE `limbs` stage 2): tails, necks, a sting. A run of `links`
  `length` long from an `anchor` on the body; at rest each link turns
  `curl`° from the last, the first `rest`° (facing right: 0 ahead, 90 up,
  180 behind); it springs toward that pose by `stiff` (0 a rope, 1 rigid),
  carries its swing (it lags and sways as the body goes), `sag`s; drawn
  tapered (`width` base to tip) and outlined as the legs, `rings` across
  its joints, a `tip` sprite turned to its last link; `far` (behind,
  darker) or `behind` (behind, as it is: a tail from the body's back).
  `aims`: within that range of the nearest thing it hunts, its end (its
  `jab` links, else its last three quarters) turns to point at it, more
  so toward the tip, the last link at it. `grow`: each link turns that
  much more than the last (a curl tightening toward the tip: a scorpion's
  tail rises nearly straight and curls over at its end, not a semicircle
  over its body). Moves drive it (a move's pose: `coil`, `reach`; the
  first of BE `limbs` stage 4): coiled, it leans back 25° and its end
  curls twice as tight (drawn back); reaching, the whole tail swings
  forward from its base, its curl let out some (of up to 100° forward and
  its curl down to a tenth, the shape that brings its tip nearest the
  target, keeping over its anchor but for its end: a whip, not a
  stretched rope). The raptor's tail and the tyrant's (cut from their
  sprites) are chains; the cave scorpion (scorpion.ron: eight legs,
  pincers, its tail 8 × 5 rising from its rear and curling over at its
  end, ringed, a sting) points its sting at you within 70 cells and,
  within 40, stings (`scorpion_sting`: coiled and trembling 0.48 s, the
  tail whipped over in 0.07 s with a small lunge, the hit 20 ahead where
  the tip lands, venom, snapped back in 0.18 s); it keeps 18 off. Seen at
  50 frames a second: poised in a tall ?, the tail whips over its back,
  the sting lands 25 cells ahead on the player (who flashes hurt), it
  swings back up (the scorpion standing at the player: its tip still
  on most frames, at most 2.2 cells, its body's bob). The `legs` scenario
  leaves the player standing `PLATYPUS_AHEAD` cells beyond the walker
  (600) and the camera follows the walker (it used to jerk the player
  ahead of it, which jerked a sting's target too). The near legs are drawn in front of the
  body (z 10.06), the far ones behind, in `far` (else the near colour
  darker). Its box takes in its legs (the stilt stalker: 18 × 58). Dead,
  it lies on its belly (a box 14 high) on its legs folded under it. The
  `legs` scenario (`PLATYPUS_WORLD=arena PLATYPUS_ARENA=real`): a walker
  led 600 cells west over the hills from seed 1's start, its tilt against
  the ground's slope: the stilt stalker (four legs, 18 × 58) 16 of 16
  readings on slopes the ground's way, 6° off on average; the crag crab
  (as first drawn: six legs from the front) 17 of 18, 11° off; the ridge
  raptor (two legs, bobbing, leaning into its run at 150 cells/s; legs
  since halved: its hip a quarter of its length up, as a real raptor's) 8 of 8,
  16° off; no foot of any in rock. Both since redrawn (arms, ankles):
  their looks judged on the arena's flat layout at 6 px a cell. With the
  gait clock: stalker 16 of 16 (6° off), crab 16 of 17 (10°), raptor 7
  of 8 (16°; its run's lean takes its nose down on gentle rises); a foot
  at most 1.02 of its reach from its hip (the crab 1.40 on its first
  frame), past it 0.6–1.7 % of frames (drawn at full length). Its run
  looked at a frame every 0.05 s (`PLATYPUS_SHOT_EVERY`).
- **Webs snare:** a material can be `sticky` (cobweb): a body in any of
  its cells moves at 30 % (with chill, the slower of the two), unless its
  creature is a `web_walker` (spiders). Blades cut webs (plants). In the
  `webs` scenario a walk covers 150 cells in 1.5 s in the open, ~53 in web.
- A creature's `light` can `haze` (glow like glowing cells: a steady
  `Glow`). Eyes in the dark aren't a glow: a creature's `eyes` (a colour
  in its text sprite; spiders' `legs` have their own) are drawn alone over
  the darkness (z 16.2, following the body's frame, flip and turn), crisp
  points of red, blinking now and then (shut 0.14 s every 2.5–6 s, each
  its own beat); their light is only the faintest red on what's right
  beside them (bats 0.03, no haze). `drops`: items that fall out when it
  dies.
- The cave spider (24 × 18, 130 hp: a body from above with eight glowing
  red eyes, legs 72 cells long, 5 thick; climbs, pounces; bites, spits
  and stings: its moves, §6.4;
  poise 60 and heft 4, a troll's weight: it shrugs off most blows;
  bleeds acid, and acid doesn't hurt it; its spit a big glob, 338 cells of
  acid (`acid_glob`): it drenches you; dead, its body keeps its legs curled in over it:
  `corpses::curled_legs`), spiderlings (the same, small, acid too), egg sacs (burst into four
  spiderlings), cocoons (hung from a nest's roof by their thread: negative
  gravity takes them up; cut open: blood and a victim's things), the slime
  (hops; full of glowing `slime`), the acid slime (full of acid, which it
  resists; glows), explosives (`creatures/custom/explosive.rs`, its own code `explosive`:
  a TNT barrel, 15 hp, a blast of 81 / 245; dynamite, 8 hp, 66 / 230; a
  mine cart loaded with both, 40 hp, 108 / 255 (craters; their rubble
  rains down far around: take cover); broken they go off at once,
  alight after their fuse, 0.6–1.3 s, sparks fizzing: anything that hurts
  a creature sets them off, another blast too, so they chain; they leave
  no body), a miner's lantern on its post (a warm light, 5 hp), the
  skeleton (the humanoid rig in bone, a rusty sword;
  crumbles to ash), the vampire bat (dives, bites). Packs `spiders` and
  `underground` spawn them together. In the `underground` scenario each
  hurts a player standing still (a spider ~90–100 hp in 3.5 s, spiderlings
  nothing in 3 s), and a spider climbs a 150-cell column to the player on top.

### 6.4 Creatures as data (`game/src/creatures/`, `assets/data/kinds.ron`, DESIGN §14)

Everything creature lives under `creatures/`: the core (`mod.rs`: health,
deaths, what hurts bodies), `def.rs` (creature files, spawning, hot
reload), `nature.rs` (kinds of hurt), `player.rs`, `spawn.rs`; `body/`
(animation, legs, hurt, elements); `brain/` (the shared brains); `moves/`
(moves as data, `assets/data/moves.ron`, and their one runner);
`custom/` (one creature's own code). Beside them: `observe.rs` (what the
player has seen creatures do) and `fight.rs` (the arena's readouts).

- **Ten kinds of hurt** (`nature::Harm`): slash, pierce, blunt, fire,
  frost, storm, acid, poison, radiant, void (and fall). Every `Hit` has
  one: a weapon's `harm` (default slash; a move's own `harm` over it), a
  `touch`'s (default pierce), a spell's element (no element: blunt), a
  blast's or a thrown thing's blunt, a coating's (acid; venom poison); a
  fall is its own.
- **Health::harm**: armour (`Ward`) takes its share of slash, pierce and
  blunt only; then the creature's multiplier for that kind. A negative
  one heals. It remembers which kinds hurt it this tick (`felt`), for
  what reacts to them, and keeps a ledger by kind (`took`: what was
  meant, what got through, below 0 drunk in), from every source (blows,
  spells, burning, acid, falls, blasts). Once a tick, just before deaths
  (so a killing blow is in it), `tally` turns each body's ledger into
  `Took` messages (target, kind, meant, dealt, dead or not) and clears it:
  what observations and readouts are made of.
- **Kinds** (`kinds.ron`): `humanoid`, `beast` (fire 1.5), `insect`
  (poison 0.3, fire and frost 1.5), `undead` (pierce 0.5, slash 0.75,
  blunt 1.5, fire 1.25, radiant 2, void −0.5; can't be poisoned),
  `spirit` (steel and blows 0.25, radiant 2), `ooze` (blunt 0.25, pierce
  0.5, fire 1.5), `construct` (blades and points 0.25, blunt 1.25, acid
  1.5; can't burn), `starfire` (drinks fire, frost 2), `thing` (props and
  dummies: plain). A creature file says its `kind`, and may change any
  multiplier (`profile: {acid: -0.25}`: the spider drinks acid) or add to
  what it can't suffer (`cant: [burn, chill, poison, web, stagger]`).
  Every file must name a known kind (a test loads them all).
- **`regen: (per_sec, stopped_by, pause, undying)`**: heals `per_sec`
  while hurt, stopped for `pause` s (default 5) by any of `stopped_by`;
  `undying`: while healing it can't be brought below 1 hp (the troll:
  10/s, stopped by fire and acid).
- **`brain: (kind, params)`**: the shared brains (`brain/`), `params` kept
  as written and read when it's spawned (so choices like `Walk(keep: 17)`
  survive). `hunter` (`brain/hunter.rs`) is every fighter: `aggro`
  (cells), `close` (`Walk(keep, jump_to_reach)`, `Range(near, far)`,
  `Swoop(hover, dive_time, dive_every)`, `Hop(every)`,
  `Crawl(pounce_range, pounce_every)`), `attack` (`Touch`, `Swing(reach,
  every, combo)`, `Shoot(draw, every, wobble)`), `wander: (speed, every)`,
  `leash`. Also `critter`,
  `villager`, `idle` (the default), `keyboard` (the player).
- **`custom: (name, params)`**: a creature's own code (`custom/`, one file
  each, registered in `CustomPlugin`): a component read from `params`
  and the hooks it needs (spawn, think after the brains in
  `CustomSet::Think`, hit, death); `_template.rs` stubs each. Now:
  `dummy` (the tallies), `explosive` (barrels, dynamite, the mine cart),
  `hatchery` (the egg sac). A file naming a brain or module that doesn't
  exist is an error at start.
- **Moves** (`moves/`, `assets/data/moves.ron`, hot-reloaded; DESIGN
  §14.1): what a creature does with its own body, as data. A creature
  file names its moves (`moves: ["spider_sting", "spider_bite",
  "spider_spit"]`, tried in that order; an unknown one is reported at
  start and fails a test). A move: `when` (`range: (near, far)` cells to
  its target, `footing`: standing or clinging, `line`: nothing solid
  between), `every` (seconds after it ends before it may come again; any
  move is followed by 0.4 s before the next), and `phases`, each named
  (windup, hold, strike, recover: the tell is what the windup shows),
  `secs` long, easing the body's pose (`legs::Rear`: `lift`, `back`,
  `curl`; `ease` Smooth, Linear or Snap; `tremble`) and doing its `acts`
  as it starts: `Lunge(speed, up)`; `Strike((at, reach, damage, harm,
  knock, up, stun, from, coat))`, live through the phase from `from` (a
  share of it): what's within `reach` of the point `at` cells toward the
  target is hit, once, and the coating left on it; `Cast(spell, at, up)`,
  aimed where the target will be and lobbed by the spell's own fall
  (`Spellbook::flight`); `Slam((radius, damage, harm, knock, stun))` round
  its feet; `Summon(kind, count, spread)`; `Sound(name)`;
  `Grab((at, reach, from))`, live: what's within `reach` of the point is
  caught and held there (`Held`, pinned by `moves::pin` after the bodies
  move: no fall, no control) until the move ends, a `Throw(speed, up,
  stun)`, or the grabber is stunned (it lets go, the move's off); strikes
  in later phases land on what's held (a strike lands once a phase); a
  grab that catches nothing goes straight to the last phase;
  `Beam(spell, at, turn)`, live: a ray spell cast every tick, its aim
  swinging after the target at most `turn` radians a second. A phase may
  play a `clip` of the creature's art (a rig's pose) through it. A move
  starts (`moves::start`, `Began`) before the brain thinks, when one's in
  range, ready, its needs are met and the creature isn't mid-swing; while
  it runs the brain's weapon waits (`Moves::busy`) and the creature stands
  (`moves::run`, after the brain). Weapons stay weapons: `hunter`'s
  `Swing` and `Shoot` use what's wielded (`weapons.ron`, the player's
  too), `Touch` the creature's `touch`. The troll's `troll_grab` (every
  9 s within 40 cells): its free arm drawn back (0.6 s, the tell), the
  snatch, two squeezes of 10, a hurl (520 cells/s, 330 up). `fire_ray`
  (a 0.9 s trembling windup, a 1.6 s sweep) waits for a creature that
  burns. The `grab` scenario: caught, squeezed (19 after armour), hurled
  235 cells; a second grab broken by a staggering blow; a skeleton given
  `fire_ray` sets the player alight.
  The spider's are all moves: in the `spider` scenario the spit from 180
  cells at 0.57 s, the sting's 30 at ~4.06 s with its venom, the bite's 16.
- **Seeing that it hurts** (`body/hurt.rs`, DESIGN §14.3; no health
  bars): every `Hit` as it lands is `combat::Felt` (what it meant, what it
  did, as a share of the target's health), shown as one of three
  (`hurt::reaction`, `Reacted`): **hurt** (a red flash and blood, both as
  much as the share: 0.08 s + 0.6 × share, at most 0.3; blood 300 cells a
  whole health, at most 158; the hit's pitch lower the more it took);
  **resisted** (half or less of what was meant got through: dull sparks, a
  clang, no flash, no blood, a third of the hit-stop); **absorbed** (it
  healed: a glow of the hurt's colour, sparks of it, a draught; its
  dripping stops for 3 s). Under half its health a creature drips (its
  blood, or dust: 5 a second); under a quarter it falters (`Faltering`:
  75 % speed). The player shows neither (the hearts say it). In the
  `reactions` scenario (arena), a skeleton struck with 3 of each kind:
  slash, blunt, fire, frost, storm, acid, radiant hurt; pierce and poison
  resisted; void absorbed (+2 %); it drips under half, falters under a
  quarter.
- **What the player has seen** (`observe.rs`, kept in `Progress::observed`
  by kind, saved with the player; the player's bestiary is made from it
  later, DESIGN §14.7): within `progress::WITNESS` (450 cells) of the
  player, from anyone's doing: how many of a kind it has met (each once),
  the moves it saw them begin, what they carried, how they took each kind
  of hurt (meant and got through, added up: `Taken::reaction` says hurt,
  shrugged off or drunk in), what finished the ones that died (the kind
  that did most in the last tick), whether it saw one heal and which
  kinds it saw stop that. Facts, not words: "It drank in the acid" is
  written from them later. The `reactions` scenario logs the skeleton's:
  met 1, felled by blunt, every kind as above.

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
  horizontal, over 165 cells/s) skips off it, up to 3 times, like a stone
  (a burning one steaming). Otherwise: fire (ignite, heat, a burning or fire
  trail) into water (anything that isn't flammable) is doused at the
  surface: it goes off there, its blast a steam blast, lighting nothing,
  its heat flashing the water around to steam, its fire sparks swapped for
  a hiss of steam; fire onto oil lights it; frost freezes the water it
  lands on or beside into ice (bridges); anything else plunges in, keeping
  0.93 of its speed a cell and ageing 3× as fast, fizzling below 75 cells/s
  (a bolt dies within ~30–45 cells). Where it meets the surface it throws
  real cells of it up.
- **Landing** applies the payloads through the sim's own edits: a blast is
  `WorldEdit::Explode` (so a fireball digs, throws debris and bodies, and
  hurts like a small bomb), heat `WorldEdit::Heat`, ignite
  `WorldEdit::Ignite` plus setting alight creatures in the radius, matter a
  `splash` of real cells, damage the body hit (with knockback), knock
  throws the body hit (`Knock(390)`: along the flight, a little up),
  shatter (`WorldEdit::Shatter`) breaks the solids and powders it lands on
  up to its hardness in a radius and throws them off as rubble, away from
  where it came (not when it hit a body). The spark wand is bolt + spark +
  knock + shatter (radius 5, hardness 60: dirt and stone, not slate).
- **Streams** spray, from 9 cells ahead of the hand, flames that last
  `life` s (± a third; the flame wand: 450 cells/s × 0.33 s, about 150
  cells) and become real fire cells where they stop (they rise, flicker and light what they touch)
  and one in six burning cells of their material; what the stream plays on
  is heated (+12 °C a cast, radius 5: wood catches, ice melts); what stands
  in it is scalded (2 a cast) and may catch.
- **Lightning** picks up to `targets` creatures within `range` toward the aim
  (within 0.6 rad of it, or 45 cells of the cursor), nearest the line first,
  or else aims at the cursor (up to `range`), and strikes each with
  `World::zap`: a jagged walk (pulled back to the line, gathered in at both
  ends) through open cells, stopped by the first solid, liquid or plant; two
  forks off it through open air; what burns along it catches and some air
  flares; where it ends it bursts (radius 3), heats and ignites. The sim
  reports each zap with its path (`StepStats::zaps`); the game draws
  exactly that path and hurts what's within 4.5 cells of the end (30 at most)
  and sets it alight (`elements::zapped`), as the sky's lightning does. It
  doesn't flare the air in its first 12 cells (the caster's hand). Into water
  (or anything that `charges`) it charges the pool (§3.12).
- **Channelled spells** (`magic/well.rs`): held open while the wand is
  held, paying `drain` mana a second, and the caster's mana doesn't come
  back meanwhile; out of mana, or let go, and it lets go.
  - **A gravity well** (`Carrier::Well`) sits at the cursor, following it
    on a spring (top speed 1 050 cells/s, most acceleration 13 500: it can be
    swung, and what it holds keeps its speed when you let go: thrown). It
    tries `pull` random cells in its
    reach each tick: powder, liquid and plants easily (more so nearer),
    solids up to its `strength` in hardness harder the harder they are
    (`World::pluck`, then `loosen_fragments` so what they held up falls), and
    catches particles in flight (`World::take_particles`), up to what it can
    `lift`: a cell weighs its density against water's (stone ≈ 2.6), a body
    its size in cells (an orc 264). Held cells each steer toward a place in
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
    the 6 cells at the hand); the left button flings everything in it away,
    the right drags it in (bodies till they're 15 cells off). Each tick it
    walks the cone farthest first (nearest for a pull) and flings up to
    `pull` cells (loose ones, solids up to `strength`) that have somewhere
    to go (straight on, else mirrored upward, else flat to the side: a push
    into the ground splashes), so the ones in front make way and a pile
    blows apart; particles in flight are shoved; bodies (not the caster) are
    launched at up to `power` cells/s (a push lifts a little) and stunned,
    and hurt by the blow: 0.027 per cell/s it changed their speed by (the
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
- **What flies hurts** (`creatures::pelted`): a particle of solid, powder or
  liquid (not rain, dust or embers) faster than 135 cells/s passing through a
  body deals its weight (density against water's; liquids half) × how many
  times faster × 0.8, and is mostly stopped (30 % of its speed left),
  shoving the body: a ball of rock dropped from a well, blast debris, a
  flung stream of sand.
- **Fall damage** (`FallDamage`, per creature RON) is by distance, Terraria
  style (speed saturates at max fall within ~75 cells, so it couldn't tell
  a double jump from a cliff): falling further than `safe_height` cells
  from where the fall started hurts `per_cell` a cell over (player 180
  and 0.4: a double or triple jump never hurts; orc 105 and 0.53). The fall
  starts at the highest point since it last stood on something, was in
  water, jumped off air or a wall (a double jump just before landing saves
  you), hung from a rope, or came down gently (slower than 120 cells/s:
  rocket boots braking near the ground, a wall slide: it counts from
  there); slamming into a wall or ceiling faster
  than `slam_speed` (675 cells/s: flung, not walking or dashing) hurts
  `per_speed` (0.17) per cell/s over.
- **Every explosion hurts** (`creatures::blasted`, from `StepStats::detonated`):
  bodies within 1.6 × its radius take up to 0.65 × its power and are thrown
  at up to 4.5 × its power, falling off with distance. Magic can hurt its
  caster: a fireball at point blank does.

## 7. Extensibility — adding things without touching the engine

| To add…              | You write…                                             |
|----------------------|--------------------------------------------------------|
| a material           | an entry in `materials.ron`                            |
| a reaction           | a line in the reactions list of `materials.ron`        |
| an enemy (existing AI)| a creature RON (sprites, stats, kind, attacks, brain + params)|
| a new way of fighting | a choice in `hunter` (`brain/hunter.rs`), or a new brain registered by name |
| one creature's trick | a module in `creatures/custom/`, one line in `CustomPlugin` |
| a kind of creature   | an entry in `kinds.ron`                                 |
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

Reference machine: Apple M2 Max. Target: 60 fps at 1080p, 2 px/cell,
on a mid-range machine, so the M2 Max budgets are set at roughly half of the frame.

| Scenario                                  | Budget per tick |
|-------------------------------------------|-----------------|
| settled world, 18×12 chunks loaded        | < 0.5 ms        |
| deep world with lava lakes, settled        | < 0.5 ms (and asleep) |
| avalanche: ~100k moving cells             | < 6 ms          |
| streaming a new column of chunks          | < 4 ms          |
| `nav`: a search, its moves known          | < 0.3 ms        |
| `nav_dig`: a tick's planning, diggers at work | < 2.5 ms    |

## 10. Working rules

- Fixed timestep for all gameplay; no `1/60` literals, no per-frame random rolls.
- No `static mut`, no global state; resources and components only.
- Every bug fix to the sim comes with a headless test.
- Tunables live in RON, not `const`, once they are gameplay-facing.
