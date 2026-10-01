# Design: the world, hands, RPG, and making things

Agreed 2026-09-25: D1–D4 and D6 as recommended, D5 left to mock-ups. Built on
the `world-arc` branch (off `v2`), so we can go back and try another path. Each
part moves into SPEC.md as its arc lands. Numbers are starting points.

## 0. Decisions to make

| # | Question | Decision |
|---|---|---|
| D1 | How big is the world? | 196 608 × 49 152 cells (3 072 × 768 chunks; grown 8× on 2026-09-29, §13), sized from the preset; `medium` (49 152 × 24 576, the first size) is the reference, `small` for tests. |
| D2 | What unit do mining and building work in? | Blocks of 6 × 6 cells on a fixed grid. The world stays cells. |
| D3 | How are humanoids drawn? | Terraria-style: layered frame sheets on one shared frame layout per body size. Gear is a "skin" painted onto body regions. Only held items rotate. |
| D4 | How do you loot the dead? | The corpse becomes a physical cell body that holds the loot. Interact to loot it. Destroy the corpse and the loot scatters. |
| D5 | How big are characters? | Keep 1 art pixel = 1 cell. Decide 27 px vs ~36 px humanoids from rendered mock-ups when the RPG arc starts. The 6 × 6 block works for both. |
| D7 | What tempo does movement have? | Open (2026-09-27): Hollow Knight's (fast, snappy, heavy: where we are) or Terraria's (a slower run with a run-up and a skid, a held rise to the jump, ~0.85 s in the air, a slow fall), or between. Leaning: Terraria's tempo with Hollow Knight's control (coyote time, buffering, variable height, the dash, the pogo), so movement gear has room to make you faster. Presets to try live in `tempo.ron` (the arena panel's Tempo row). |
| D6 | In what order? | World plan and viewer → terrain → hands (mining, items, chests) → ores → structures → saving → art tool and arena → RPG. |

## 1. Principles (carried over)

- **Cells are the truth.** Everything physical is cells: terrain, built blocks, corpses. (Furniture is the exception: entities with bodies.) Special cases become material data plus general rules.
- **Data over code.** Creatures, items, moves, loot tables, rooms and structures are RON or text files, hot-reloaded.
- **Anyone can use anything.** A weapon, a spell or armour works the same for the player and for any creature with the body to use it. The player is a creature with a keyboard brain (already true).
- **Deterministic and chunk-pure.** A chunk is a pure function of (seed, position, plan). Co-op and saving depend on it.
- **Everything can be looked at.** Every generator and asset has a headless render the model can open as an image, and a sandbox to try it in.

## 2. Scale

At 1 cell = 1 art pixel, the player is 9 × 23 cells and runs 143 cells/s.

| | Cells | In player heights | Terraria large (in player heights) |
|---|---|---|---|
| Width | 196 608 (was 49 152) | ~8 500 | ~2 800 |
| Height | 49 152 (was 24 576) | ~2 100 | ~800 |

Crossing the world on foot takes about 24 minutes (it was 6). Vertical bands, in the reference world (`medium`; the large world's sky, caverns and deep are taller: SPEC, presets) (sea level at about 25 % from the top):

| Band | Height (cells, relative to sea level) | What's there |
|---|---|---|
| Sky | +3 750 … +6 000 | Sky islands, shrines on them, the cloud band, wyverns later |
| Peaks | +1 200 … +3 750 | Mountains above the snow line (the climate makes snow, it isn't painted on), castles, cliffs |
| Surface | −300 … +1 200 | Biomes, forests, lakes, ruins, crypt entrances |
| Underground | −300 … −3 750 | Caves, mines, ores (copper, iron), crypts |
| Caverns | −3 750 … −10 500 | Huge chambers, underground lakes, crystal and mushroom caves, silver and gold |
| Deep | −10 500 … −16 500 | Chasms, the deepest ores, old ruins |
| Underworld | −16 500 … bottom | A lava sea, obsidian, heat |

Costs that grow with the world:
- **Weather** spans the width, simulated only over a window round the players (2 304 cells beyond the loaded chunks); the rest waits, until the world clock's coarse weather (§13) takes it over.
- **The plan** is per-column arrays plus fields at 1/24 resolution (2 048 × 1 024), a few MB.
- **Saving** stores only modified chunks (lz4).
- **Streaming** is unchanged: only what's around players is loaded.

## 3. World generation

### 3.1 Two levels

1. **`WorldPlan`**: computed once from the seed, in seconds, on worker threads. It holds:
   - the biome map, the surface height per column, mountain ranges, the snow line;
   - lake basins with their water levels;
   - cave-layer parameters, chasms, walker tunnels;
   - structure sites with their generated layouts;
   - ore-field parameters and chest sites.
2. **Rasterising.** Each chunk reads the plan, plus any structure layouts whose bounding boxes touch it, and fills its cells. It stays chunk-pure: a chunk never reads another chunk.

### 3.2 Plan stages

1. **Macro.** Oceans at both edges. A biome sequence across x from temperature × moisture (forest, plains, desert, snow, jungle, swamp), with blended borders.
2. **Relief.** Hills (current), mountain ranges with ridges and cliffs, valleys, overhangs, and sky islands (port legacy). Peaks rise above the snow line, so snow, ice lakes and colder weather follow from `Climate`.
3. **Water.** Basins carved and filled to their spill level: large lakes, mountain tarns, underground lakes, waterfalls where a basin spills over a cliff. Water is generated already levelled, so it's asleep on load.
4. **Underground.**
   - Worm tunnels near the surface.
   - Cavern-layer noise with large chambers, stalactites and pillars.
   - Chasms: huge vertical shafts linking surface to deep. This is the Elden Ring-style descent.
   - Walker tunnels (legacy), which guarantee the layers connect.
   - A lava sea at the bottom.
5. **Structures** (§3.3).
6. **Resources.**
   - Ores are materials with hardness tiers. They sit in depth bands as noise blobs and veins along the strata, more often exposed on cave walls.
   - Gems glow, which the lighting already supports.
   - Chest sites: in structures, and hidden in cave pockets.
7. **Secrets.**
   - Illusory walls: a material drawn like brick that dissolves when struck.
   - Weak walls that break in one hit.
   - Rooms behind waterfalls, and treasure at the bottom of lakes.
   - Keys and locked doors (after items exist).

### 3.3 Structures: castles, crypts, ruins

- **Rooms are text files.** Each is a grid at block resolution (one character per 6 × 6 cells), with a legend mapping characters to materials and markers:
  - `D` door socket, `C` chest, `S` spawn group, `T` torch, `L` ladder or platform;
  - `?` illusory wall, `W` water, `B` boss spawn.
  Sockets on the edges say which sides connect.
- **Assembly.** A layout grammar per structure type: a room graph on a coarse grid, filled by picking room templates whose sockets match (Spelunky / Dead Cells style).
  - Every layout has a critical path to a goal room, side branches, and secrets hanging off it.
- **Crypt.** A surface ruin, then a stair shaft, then a room graph going down, then a boss or treasure room. A key on a side branch opens a locked shortcut.
- **Castle.** Sited on a mountain top or cliff. The outer shape comes from rules (keep, towers, curtain walls, bridges over chasms), with foundations reaching down to the rock. The interior uses the same room assembly, and towers make it vertical.
- **Ageing pass.** Cell-level noise adds cracks, moss, cobwebs, rubble and collapsed sections, so no two copies of a room look stamped.
- **Materials look right on their own.** A material can carry a world-anchored pattern (brick, planks, cobble), so castle walls and player-built walls tile cleanly (§4.4).
- **Layouts are computed at plan time** from (seed, site id). The plan stores them; chunks rasterise the parts they touch.

### 3.4 Tools for generation

- **`platypus-worldview`** renders the plan (or rasterised regions) to PNG:
  - the whole world at 1:24;
  - any region at 1:1;
  - overlays for biomes, bands, structures, ores and chests.

  The model reads these images to check its own changes. This is the main loop for tuning generation.
- **A determinism test.** Generate chunks in different orders and on different threads, then compare checksums.
- **World presets.** `small` (12 288 × 6 144) for tests and scenarios, `large` for play.

### 3.5 Saving

A save is a directory:
- `world.ron`: format version, seed, preset, tick, weather state;
- the chunk store: modified chunks, lz4, in region files of 32 × 32 chunks;
- `entities.ron`: creatures, items on the ground, chest contents, bodies in flight;
- per-player files: inventory, equipment, position.

The plan isn't saved; it's regenerated from the seed. A version bump migrates or refuses to load.

**Built 2026-09-27 (`save.rs`, branch `progression-arc`)**, as a first
version to build on: `saves/<world>-<seed>/` holds `world.ron` (version,
kind, seed, tick: so the time of day, the materials by name in the order the
chunks number them, the spawned places, the chest key counter), `chunks.bin`
(every changed chunk, loaded or not, in the sim's own lz4 bytes; one file,
not regions yet), `player.ron` (position, health, mana, pack, what's worn,
the hotbar in hand, progress) and `things.ron` (chests and every chest's
contents known, crafting stations, items lying about, creatures; a body's
belongings are kept as items where it lay). Items and materials are kept by
name, so a save survives reordered data files (a missing name is dropped
with a warning). Saved every minute, on quitting and with Ctrl+S; loaded at
start. Not yet: the weather, bodies as bodies, falling sand and bodies in
flight, open portals and burning cells' timers (the cells themselves are
saved), regions, choosing a world at start.

## 4. Hands: mining and building

The current per-cell radius pickaxe removes an uneven blob and gives nothing back. Terraria feels good because every hit has a clear target, the result is predictable, the feedback is immediate, and the progress adds up.

### 4.1 Blocks

- The grid is fixed at 6 × 6 cells (D2): block `(x.div_euclid(6), y.div_euclid(6))`. The player is 2 blocks wide and 4 tall. Terraria's is 2 × 3.
- **Mining a block.** Each hit adds damage to the block; cracks show. When it breaks, every cell in the block that the tool can mine is removed at once. Harder cells, such as ore in a weaker tool's hit, stay.
- **Generated terrain stays cell-detailed.** A block at a cave edge may be half air. Mining it yields what was there.
- **Yield.** The inventory counts materials in cells and shows whole blocks (36 cells = 1 block). Partial blocks add up, and nothing is lost to rounding.
- **Placing a block.** It fills the empty cells of one grid block, using 36 cells of material.
  - Placed powder falls and placed liquid flows (sand, and water later from a bucket).
  - Placed stone is static. It needs support like any terrain (the existing fragment checks).
- **Tools and tiers.**
  - The pickaxe works on the playfield, the axe on trees and background, a hammer on background walls.
  - A tool's tier sets the hardest material it can mine, which drives ore progression.
  - Speed sets hits per second.
- **Particles.** Each hit throws dust and a few cells of debris, as now.

### 4.2 Smart cursor

The target is chosen for you and outlined:
- **Mining:** the first solid block on the line from the player's hand to the cursor, within reach (about 5 blocks). You dig the face you see, not a buried block under the cursor.
- **Placing:** the block under the cursor if it's empty, touches a solid block or background wall, and doesn't overlap a creature. Otherwise the nearest valid block along the same line.
- **Auto tool** (a held key): picks the right tool for the target, such as an axe for a trunk or a pickaxe for stone.
- **Fine tools stay** for dev work and later for spells: wands that dig by cells, bombs, the heat gun.

### 4.2b As built (stage 4)

- The player's box became 9 × 23 cells (was 12 × 24): exactly two blocks wide,
  it never fit a 2-wide shaft unless perfectly aligned with the grid.
- Chests became furniture entities (2026-09-25): as cells they hung in the
  air when their floor was mined out, and a rigid body of chest cells would
  land tilted and lose its identity (contents were keyed by the corner).
- Building pushes aside tall grass, smoke and flames.

### 4.3 Items on the ground

Mined material and drops are item entities. They fly out a little, settle, and drift to a nearby player (a magnet). They're also physical: they burn, sink and float by their material. A scroll burns; an iron sword doesn't.

### 4.4 Patterns

A material can have a `pattern`: a small tile of shade indices anchored to the world grid (for example 8 × 4 for bricks). Cells of that material draw their shade from the pattern instead of at random. Built walls, castles, planks and cobble then look like Terraria tiles without being tiles.

## 4.3b Backdrops and the night sky (built 2026-09-27, branch `backdrop-arc`)

- **Surface:** Noita-like ranges (`crates/backdrop/src/peaks.rs`), after
  two tries the user turned down (per-biome Noita scenes: too busy, and
  6–50 % parallax made you seasick; Terraria-like cliffs: not it). Tall
  sharp peaks cut into flat faces (wedges from the apex, lit on the left,
  shade on the right), concave flanks with shoulders, snow on the tops
  reaching down the gullies; range behind range fading into the sky's
  haze, mist at their feet, and below the feet the ranges run on down
  behind the ground (flanks steepening into shoulders, the gaps filled
  with the range's body: no band under them); big cloud heaps rising
  behind the ranges (faded bottoms hidden behind them), drifting. All of
  it behind the weather's clouds (z -2) and the back walls.
  A look per biome (`for_biome`): `alpine` for mountains and tundra,
  `noita` everywhere else; `violet`, `misty`, `needles`, `broad` wait for
  places of their own. Looks crossfade as the biomes around the camera
  change. Ranges move at 1–4 % of the camera's motion across, far to
  near, and hardly at all up and down (0.4–1.5 %: they sit still in the
  view however high or low on the surface you are); the ranges' tiles are placed to whole screen pixels
  from the camera (whole cells hopped 2 pixels: jitter). The cloud heaps,
  drifting on their own too slowly for a pixel at a time to look like
  motion, are scaled up 3× and sampled smoothly instead: they glide by
  fractions of a pixel, their blocks' edges blended a pixel wide. Below a range's
  feet, one flat colour (the skirt stretches the bottom row down). A sky
  gradient behind, tinted by the hour's sky colour. `platypus-backdrop
  <dir>` writes `peaks.png` and each look by day, at dusk and at night.
- **The sky:** the sun crossing by day (low and golden at either end): a
  small white core in a warm bloom, the sky brightened wide about it. By
  night single-pixel twinkling stars and a big moon with seas in two tones
  and a soft halo (stars behind the disc hidden, near it washed out), in
  its phase: eight nights a cycle, the first a full
  moon, then waning (lit from the west) to new and waxing back (lit from
  the east), the unlit part faintly there and the halo as bright as it's
  lit (`PLATYPUS_MOON`=0–7 picks one: 0 new, 2 waxing half, 4 full, 6
  waning half). Drawn over the lighting where the sky is open (no cell, no
  back wall, no mountain, dimmed by cloud), each of the sky image's pixels
  (not square unless the window is 16:9) shaded by where its centre is
  from the disc's: not snapped, the disc glides, its edge and terminator
  anti-aliased, its tones blended; laid over what's there (the moon over
  its halo over the stars), the alpha dithered so faint glows don't ring.
  Sun, moon and stars are fixed to the view: too far off for the camera to
  move them. Nights are near black
  (`moonlight` 0.45, a darker sky).
- **Underground:** behind the back walls, a dark void, tinted a little by
  the band and zone the camera is in (earth, stone, the deep blue-black;
  teal in fungal zones, violet in crystal, sickly green in toxic, red over
  the underworld), eased as they change, under the lighting: only near a
  light does the tint show. In its dark a few very faint twinkles, one
  pixel each, very far off (2 % of the camera's motion), each swelling and fading on its own slow
  beat, in the zone's colour, where the cave is open to the void. (Tried
  and turned down: far rock layers by band, per-zone art, and
  "underdarks": vast lit caverns with vistas.) Scenario `voidlook`.

## 5. Items and inventory

- **Item definitions** (RON), made of optional parts:
  - `stack`, `icon` (derived from the sprite by default);
  - `material` (a block of something), `tool` (tier, speed, reach);
  - `weapon` (moveset, damage, weight), `armour` (slot, skin, stats), `spell`;
  - `container`, `light`, `burns`, `value`.
- **Item instances** are a definition id plus state: stack count, durability, and later rolled properties.
- **Inventory** is a component with slots, on any creature or container. **Equipment** is a component with slots: head, body, legs, feet, hands, back, main hand, off hand, rings. The player and NPCs use the same components.
- **Containers:**
  - A chest is furniture: an entity with a body (it falls, blasts throw it, fire burns it) and a key to its contents. It breaks and its contents scatter when it's had enough. (Changed from "bound to the cells of its footprint": see §4.2b.)
  - General rule: furniture (chests, barrels, lanterns, crates) is entities with bodies, made by worldgen as spawns alongside the cells.
- **Potions** (`potion.rs`, built 2026-09-27): an item's `use: Potion(heal,
  over, sickness)`. Click it in the hand, or H drinks the first healing one
  carried (Terraria's quick heal): `heal` back over `over` s, then potion
  sickness for `sickness` s (a status timer) when no healing potion works;
  not drunk at full health or while sick. Healing comes back smoothly, 10
  a second (`over` = heal / 10). First: the small health potion (40 over
  4 s, 30 s sickness), from glass and a mushroom stem at a workbench (a
  placeholder), in chests at every depth. Drinking: a pop and a fizz.
- **The screen's edges** (`screen_fx.rs`): a red flash fading in and out
  when hurt (stronger the harder), a slow red breath below 30 % health (a
  cycle every 2.4 s, 1.7 s near death; smooth, never down to nothing, its
  phase stepped on each frame so a change of health never jumps it: the
  first one, up to 1.5 beats a second, sharpened, its phase from the clock
  times a pace that moved with health, flickered), a green glow while a potion heals: full as it's
  drunk (easing in), then dissolving in place as the healing runs out, its
  soft inner reach fading first and the rim at the edge last, continuously.
  Each glow is a soft band (15 % of the screen's height deep, as deep from
  every edge) and a thin rim (6 %).
- **The hotbar and inventory** (Terraria's layout): the hotbar top left,
  what's held named over it; rounded slots, each with its key at its top
  left, the chosen one bigger and gold. Esc opens the inventory under it
  (the hotbar is its top row; X for the next hotbar): five rows of pack,
  no box behind them, a discard slot (Terraria's trash: a stack dropped in,
  or Ctrl-clicked from a slot, is kept there to take back until the next
  goes in), the gear worn and the stats to the right, an open chest below.
  The keys' line bottom centre; the debug text bottom left; the arena
  panel under the hotbar (away while the inventory's open).
- **Loot tables** (RON) by context, such as `crypt_deep` or `troll`: weighted entries, counts, rarity by depth.

## 6. Creatures and bodies

A creature is a body with movement and a brain (as now), plus optional parts:
- `rig`: what it looks like and where things attach;
- `inventory` and `equipment`;
- `stats`;
- `natural_weapons`: bites and claws as weapons with no picture;
- `loot`: a table for what it carries beyond its equipment;
- `faction`.

### 6.1 Rigs, not a humanoid type

- **Humanoid rig classes:** `small` (goblin, ~18 px), `human` (player, bandits, knights, ~27–36 px), `large` (troll, ogre, ~54–72 px).
  - Each class has one frame template: idle, walk cycle, jump, fall, dash, wall slide, a fan of arm angles for holding and swinging, hurt, death.
  - Each frame records anchors: hand front and back, head, back, and the weapon grip angle.
- **Body art** for a race is drawn on its class's template, as layers: back arm, legs, torso, head, front arm. Races differ in body art and base stats, not in code.
- **Gear on the body (D3):**
  - The template marks regions in every frame: head, torso, arms, legs, feet.
  - An armour piece is a **skin**: a pattern and palette painted into its regions, plus optional extra silhouette parts drawn once and placed at an anchor (pauldrons, a plume, a hood, a cape).
  - So one helmet definition fits every frame, and every humanoid class. A troll can wear the knight's armour, and it looks like the knight's armour at troll size. Hand-drawn per-frame overrides are allowed where a skin isn't enough.
- **Held items** rotate around the hand anchor. This is the only free rotation; body parts are never rotated, because rotated pixel art turns to mush.
  - Weapons aren't scaled with the wielder. A troll's club is huge because it's a huge item. If you pick it up, you swing a colossal weapon, slowly (Elden Ring style).
- **Other rigs:**
  - Frame-sheet creatures, like the current orc: beasts, slimes, spiders, flyers.
  - Segmented creatures such as worms and serpents: a chain of sprites following a path.
  - These use natural weapons and abilities, not gear.
- **Hands decide what can be wielded.** A rig with hands can use any weapon or spell (one- or two-handed). A wolf can't hold a sword, but anything with hands can pick one up and use it.

### 6.2 Stats (kept small at first)

Health, stamina (dash, attacks, blocking), poise (Elden Ring stagger), and movement stats (already data). Weight comes from equipment load and slows movement. Attributes and scaling (strength, dexterity, arcane) come later and only change numbers.

## 7. Combat and abilities

- **Moves are data.** A moveset belongs to a weapon class, and a weapon can override parts of it. A move is:
  - keyframes over time: weapon angle, arm frame, body lean, a step forward;
  - wind-up, active and recovery windows;
  - stamina cost, poise damage, cancel rules, and the next move in a combo;
  - swing trail colours.

  Different swords swing differently because their data differs.
- **Hits** are the weapon sprite's alpha mask, rotated and swept between frames in sub-steps: pixel-accurate, no tunnelling. Then hit-stop, knockback and a `Hit` message.
- **Damage types map to the element system.**
  - Fire damage is heat exposure and can set the target burning.
  - Frost chills. Lightning passes through water and wet creatures (the planned electricity).
  - Status and coating rules apply as they do now.
- **Weapons act on the world** through the same edits as tools:
  - A burning sword ignites grass.
  - A greathammer slam loosens dirt and knocks down loose cells.
  - A spear thrust into water splashes.
- **Spells are abilities.** A cast pose, then an effect: a projectile, a field, a beam or a summon. The effect applies world edits and exposure. A fireball is a projectile that explodes into heat and fire, so the existing systems carry it.
  - Noita-style wand modifiers can come later, as a list of effect modifiers.
- **The down-strike (the pogo; built 2026-09-27):** in the air, S and
  attack (or attack aimed more than ~20° below level) is a down-strike: the
  weapon's `down` move, its angles from straight down, apart from the combo.
  What it meets bounces you up a jump's worth (445 cells/s) and gives back
  air jumps and the dash: a creature, a hostile spell (cut out of the air,
  its path this tick checked against the blade), a hazard (lava, fire,
  acid, web; not plain ground, or every landing and fall could be skipped).
  Two kinds, by weapon: a **slash** (the shortsword, the dagger: a quick arc
  under you, once a strike) and a **plunge** (the longsword: point down,
  falling at least `dive` cells/s, past the fall cap, live until it lands;
  with S held it bounces off all it meets on the way down, let go it ends at
  the next bounce; landing, it slams what's within `slam` cells along the
  ground). The body shows it (`strike_down`, `plunge` clips). Weapons
  without a `down` move turn their first move down. Fall damage stays
  height-based: a bounce starts the fall over; a plunge from high up still
  lands as a fall.
- **AI** picks moves from what the creature actually has equipped, using tags on moves: range, wind-up, area, gap-closer. An NPC with a spear pokes from range; the same NPC with a greatsword charges.

## 7b. Magic, weapons and crafting (agreed 2026-09-26, branch `magic-arc`)

The cell simulation is the magic: every spell acts on it through the same
edits and exposure as tools, bombs and weather, so its interactions aren't
scripted (a fireball into an oil pool, frost on water building an ice
bridge, lightning into a flooded cave, acid eating a crypt door).

**Spells are their own things, made of runes; wands and staffs are what
powerful spells need (decided 2026-09-26, replacing "runes in a wand").**
Closer to Skyrim and BG3 than Noita: the combo lives in the spell, not the
wand.

- **A spell is a recipe of runes** (a carrier, what it carries, how it
  behaves; the kinds below). You equip spells (hotbar slots), not wands.
- **Runes are knowledge.** A spell found in the world (a scroll, a tome, a
  shrine) can be *studied*: learning it breaks it into its runes, and every
  rune you know you can use in spells of your own (a spell editor at an
  arcane table). A found spell can be cast only once you know all of its
  runes; studying it is how you learn the ones you don't.
- **Runes have levels** (spark I, II, III; blast I, II…). You start knowing
  the level-1 runes of the basic spells, so the early spells found near the
  surface and in the upper underground can be equipped and tried at once;
  deeper ones need study first.
- **Wands and staffs gate and shape power.** Each spell has a tier; the
  weakest cast bare-handed (a spark, a small lift), stronger ones need a
  wand or staff equipped, of at least that tier. The focus adds its stats
  (mana, cast speed, an element's strength, spread), so the same spell is
  stronger from a better staff, but the wand holds no spells.
- **Mana is the caster's**, as now.
- **Anyone with hands casts the same way**: an orc shaman is an orc who
  knows some runes, with a staff.

(C1 built wands that hold their runes. REVISED 2026-09-26 by the user, after
trying a free spellbook: **each wand or staff holds its spells** (a wand one,
a staff two: left and right button), picked from spells.ron recipes; a found
focus rolls its spells from its element's up to its tier; each focus has an
aura (glow and sparks at its tip) so wands look like what they cast. The rune
knowledge model survives as re-inscribing a focus at an arcane table, later.)
The rune kinds:

| Kind | Decides | Examples |
|---|---|---|
| Carrier | how it travels | bolt (fast, straight), orb (slow, bouncing, falls), beam (instant line), stream (spray), cloud, wall, touch, rune/trap, self, lightning (the sim's lightning, from the wand toward one or more targets) |
| Payload | what it does where it lands | matter (water, acid, oil, sand, lava, ice…), heat (+ fire / − frost), force (push, pull, blast), charge (electricity), light, grab (telekinesis: loose cells and bodies into a floating ball of cells, a rigid body, to throw or drop), transmute (stone→sand, water→ice), blink (teleport to it), carve |
| Modifier | how it behaves | bigger, faster, gravity (arc), bounces, homing, split / multicast, spread, pierce, longer, delay, trail (leaves material along its path), trigger (when it lands, cast the next rune from there) |

The classics are compositions: fireball = orb + spark trail → trigger heat
burst + blast; acid arrow = bolt + gravity carrying acid; flamethrower =
stream of burning oil mist; call lightning = a strike from the sky at a
point; wand lightning = the same bolt physics, smaller, from the wand to
its targets; heat object = heat beam; lift and drop matter = grab; teleport
= bolt + blink. Runes are loot and are crafted from gems and ores (ruby
fire, emerald acid and nature, amethyst arcane, mithril force). Anyone with
hands casts (orc shamans use the same runes). Guard rails: trigger depth,
particles per spell, mana.

Electricity comes with it: conducting through water, metals and wet
creatures.

**Families and carriers (agreed 2026-09-26, branch `gear-arc`).** A spell
is a family (its element) on a carrier; the matrix fills in as data once a
carrier exists. Carriers: bolt, orb, beam (an instant line from the hand,
held; its payloads go off at its tip every tick), stream, cloud (a lingering
drifting area), wall (conjured into the world for a while), call (from above:
the sky in the overworld, the ceiling underground; sky calls need open sky),
field (held at the cursor: well, force). Families: fire, frost, storm, acid,
force, gravity, **radiant** (light: cuts and reveals; its hurt goes through
armour), **void** (space: blink, portals, stasis), earth later. Built so far:
the beam carrier with the fire ray, the frost ray (ice bridges, lava crusted
to basalt: lava now freezes below 950 °C, which only a sustained cold beam
reaches) and radiant's vaporiser (`Vaporise`: cells worn down by hardness,
gone to smoke); the spark bolt moved to radiant, and the star bomb (a
bouncing star: `Vaporise` + `Nova`, a blast that leaves cells be); storm's
shock bolt (`Arc`: a spark of lightning where it lands); the wall carrier
(ice wall, left to melt; fire wall, kept burning for 5 s: `conjure.rs`)
and the cloud carrier (toxic cloud: miasma, a new *heavy* gas that sinks
and pools, corrosive and flammable); the call carrier (call lightning:
a storm forced over the spot, then the sky's lightning; meteor: a burning
rock falling from high up, crater, fire, lava; both need open sky); void
(`void.rs`): blink (you're where the bolt lands), portal pairs (bodies,
spells in flight, and liquids and sand pressing into a mouth come out of
the twin, speed turned to face out; a minute each), stasis (a bubble that
holds bodies and spells, which go on as they were when it bursts). First
batch done; the rest of the matrix fills in as data, and stasis could hold
particles and falling cells too.

**Weapons.** Melee as §7 (moves as data per weapon class, sprite-mask hits,
stamina and poise), plus coatings on blades (a sword dipped in oil and lit
burns; dipped in acid, it corrodes). Ranged: bows and crossbows (arrows are
bodies that stick in walls), guns whose ammo carries material (incendiary,
acid, a gravel shotgun, a musket that burns real gunpowder), thrown spears;
a grappling hook.

**Crafting (decided): stations and recipes as data first**, Terraria-style
(workbench, furnace smelting ore to bars with coal or charcoal, anvil,
alchemy table, arcane altar for runes); flasks and buckets hold cells of a
material (throw acid, pour water, drink a potion); alchemy inside the sim
(mixing in a cauldron) later.

## 7c. Gear, loot and classes (agreed 2026-09-26, branch `gear-arc`)

A general, data-driven gear system: adding a piece is a data entry, adding
a stat is naming it once (`gear/stats.rs`) and reading it where it acts.

- **No locked classes: gear makes the fighter.** Armour comes in three
  weights, and each piece brings its weight's stats (gear.ron `weights`):
  Light (cloth: mages; mana regen), Medium (leather: rangers), Heavy (mail
  and plate: warriors; armour and poise, at a cost in stamina and mana regen
  and a little speed). A battle-mage is possible, and paid for.
- **Slots:** worn: head, body, hands, legs, feet, two trinkets, a hook.
  Held: the hotbar item in the hand counts while it's there (weapons, tools,
  foci).
- **Stats** add up from everything worn and held (`Stats`): armour
  (`armor / (armor + 50)` of physical hurt stopped), health, poise,
  fire / frost / storm / acid / fall resistance, damage, attack speed, crit
  and crit damage, knockback, stamina and regen, mana and regen, spell power,
  cast speed, a power per element (fire, frost, storm, acid, force,
  gravity), move speed, jump height, air jumps, luck. Every hurt goes
  through `Health::harm(amount, kind)`, which applies the ward.
- **Rarity and item level:** common, uncommon, rare, epic (random bonuses
  from a data table, named prefixes and suffixes, scaled by item level:
  where it was found), and legendary uniques (hand-made, fixed). A piece
  carries only its roll (rarity, level, seed): its bonuses are worked out
  from it.
- **Gear is drawn on the body** as skins: recolours of the humanoid rig's
  palette roles (tunic, trousers, boots, skin) in chosen parts, and overlay
  parts drawn over named parts (a helm over the head) wherever they're drawn,
  in every pose and aiming frame. One drawing fits every humanoid.
- **Corpses:** a death leaves a body that falls and can be thrown; it holds
  what the creature wore and carried and a roll of its loot table. Right-
  click opens it with the chest window (chests and corpses are containers);
  an emptied one fades.
- **Spells and foci:** a wand holds one spell, a staff two (left/right
  button), of its tier or less; spells are recipes in spells.ron; found foci
  roll theirs from their element's; each has an aura (see §7b).
- **Enemies wear gear** from their loot tables, and what they wear is what
  they drop.
- **Gear that moves you:** boots with a trick (rocket boots: hold jump in the
  air to thrust up, their exhaust real fire; refilling whenever they're not firing (in
  the air too; after 0.25 s; not while jump is held on empty, or they'd
  sputter on for ever), a second's charge a second, so as long as they fire; under water they fire at half thrust and 45 % of the
  speed, bubbles, no flame; their charge a round timer top right while
  worn; thrust 3450 up to 315 cells/s, a little gentler than at first for
  control; cloud boots: air jumps), and a
  **grappling hook** in its own slot, used with E whatever is in the hand:
  Terraria's by default, on a real rope (2026-09-27: the hybrid, after the
  pure Terraria pull got stuck on walls). Thrown at the cursor, it takes hold
  of what it meets (a solid cell or a platform, a chest or a body, a
  creature) and reels you in to hang there; S stops the reel. The rope is a
  length, not a spring: S pays it out (rappel), W climbs it, and hanging
  loose you swing as a pendulum, A/D pumping it (gently: they never brake
  it). Against a wall on the rope, pressing away kicks off it. The rope wraps
  round corners it's pulled over and unwraps as you swing back; reeled into
  a corner and stuck, you slip round it (and round a stall against a wall).
  Hung near a ledge's lip, W or pressing toward it mantles up onto the top.
  Jump lets go with a full jump on top of the swing's momentum, E again
  hooks somewhere else. The cell it holds holds only while it's solid (dig
  it, blast it, and it comes loose); a body anchor comes loose when
  something comes between you. Something smaller than you (a chest,
  a body, a small creature) is pulled to you instead. On the rope you
  never take a slam's damage against rock (reeled in fast, or flung round
  a corner by the rope). A hook's reach, pull
  speed, throw speed, bite and look are data (`hook` in gear.ron).
- Not now: durability and repair, coins and merchants.

Build order (a commit each): stats + equipment → skins → rarity and bonuses
→ corpses and containers → spells and foci → enemies equipped and example
content.

## 7d. Crafting and progression (built 2026-09-27, branch `progression-arc`)

Systems first, with example content to iterate on.

- **Stations** (`crafting.ron`): furniture (workbench, furnace, anvil,
  arcane altar), drawn as text art, each also an item that sets it down
  (like a chest); a pickaxe knocks it back into its item. A furnace glows.
- **Recipes** (`crafting.ron`): makes × count from needs (counted in the
  items' own units: blocks as blocks), by hand or at a station within
  reach. The inventory screen has a crafting panel: what you can make now
  first, what you're short of dimmed, each with what goes in and where;
  click to make one (a `CraftRequest`, so scripts and later co-op/AI can
  craft too).
- **Discovery**: a recipe shows once you've held anything that goes into
  it or comes out of it (Terraria's), and a `locked` one only once a
  milestone unlocks it.
- **Progress** (`progress.rs`, on the player, saved): items ever held, the
  deepest you've been, kills by kind, crafts, milestones reached, recipes
  unlocked.
- **Milestones** (`progression.ron`): a condition (holds, crafted, depth,
  killed, another milestone; all/any of several) and rewards (items,
  unlocked recipes), with a toast when reached.
- **The ladder so far**: wood → planks → workbench → furnace (stone,
  planks, torches) → bars from ore → anvil (iron) → tools, arms, armour,
  hooks; gems unlock the arcane altar → foci; the deep unlocks mithril and
  void recipes. Ores already gate by pickaxe tier and depth.

## 7e. Sound (built 2026-09-27, branch `sound-arc`)

- **Engine:** bevy_seedling (Firewheel) plays; Bevy's own audio is off. Three
  buses (effects, ambience, music), their volumes in `sounds.ron`; F11 mutes.
- **Made, not recorded:** every sound is a recipe in `sounds.ron` (thud, clang,
  whoosh, burst, drip, boom, zap, chime, blip, step, swish, impact; loops: fire, cave, rain,
  wind, water, lava, pads), rendered at startup in the background (fundsp's
  filters and reverb, our own oscillators and envelopes), a few takes each,
  and again when the file is saved. Any sound can be a recording instead
  (`file:`), under the same name: the plan is to swap in recordings only
  where a recipe falls short (fire, rain, wind are the likeliest).
- **Effects** are placed in the world (panned, fainter with distance from the
  camera; a cell is ~0.07 m) and asked for by name (`PlaySound`): hits, hurts,
  deaths, swings (heavier blades deeper), clangs off stone, pogo bounces, the
  plunge's slam, footsteps and mining by what's underfoot (stone, dirt, sand,
  wood, snow, grass, water), landings by how far, jumps, dashes, the hook,
  the bow, blasts, thunder, wand lightning, casts, spits, pickups, crafting,
  milestones.
- **Ambience from the world:** four times a second the cells around the
  camera are sampled (what burns, and where; lava; water that just moved;
  rain falling; rock overhead from open air) and looping beds fade toward
  that: a fire's roar and crackle panned toward it, a cave's rumble and air,
  wind and rain in the open. Underground is rock overhead from open air, or
  being more than ~23 cells below the ground as generated (so a shaft dug
  up to the sky, or a chasm, doesn't flip the music to the surface's).
  Underground, drops fall from real ceilings in view, and every effect is
  sent to a cave reverb as deep as you are (`cave_reverb`). Rocket boots
  roar while they fire (a bed, like the ambience). Everything that should
  carry is kept above ~150 Hz: laptop speakers lose what's under it.
- **Music:** chill synth pads, a loop per mood (day: D major; night: A
  minor; underground: low open fifths), all playing, crossfaded by where you
  are and the time of day.
- **Next:** creature voices, spell sounds per element, torches crackling,
  underwater and underground muffling, stretches of silence in the music,
  recordings where recipes don't hold up.

## 8. Death and loot (D4)

1. **Death.** The creature's current frame is turned into a rigid body of cells (a new `flesh` material plus blood coating). The body's cells keep their sprite pixels, so the corpse looks like the creature.
   - The existing body solver makes it fall, tumble and sink.
   - The cell world can burn it, blow it apart or dissolve it in acid, and the sprite loses those pixels.
2. **Loot.** The corpse holds the creature's inventory, its equipment and a roll from its loot table. Interact to open a loot window: take items, or take all.
   - If the corpse is destroyed, its loot scatters as item pickups (unless it burns up, such as scrolls in fire).
   - Creatures without inventories, such as a wolf, drop their loot-table items as pickups straight away.
3. **What's lootable.** Equipped gear is always lootable: if you saw it, you can take it. Loot tables add the rest.
4. **Persistence.** Named characters stay dead (saved). Ordinary spawns respawn by region rules later.

## 9. Characters and NPCs

A character definition is a race (rig class, body art, base stats), plus a loadout (equipment and inventory, or loot-table rolls), a brain, a faction, and later dialogue.
- A bandit is a race, a rolled loadout and a brain.
- A named knight is the same with a fixed loadout.

Spawn groups in structures reference character definitions.

## 10. Making assets, by hand and by model

The aim: the model can create, look at, and test a creature, item, room or structure without anyone drawing it by hand. A person can still draw in Aseprite and import it.

- **Text sprites.** A palette (named ramps: `steel: 4 shades`) plus character grids per layer and frame. The model can write these directly up to about 48 × 48 px.
- **Shape recipes** for anything bigger or regular: rects, ellipses, lines, polygons in palette colours. Compiled with:
  - auto-outline;
  - light-from-top-left shading;
  - dithering;
  - pattern fills.
- **Gear skins** (§6.1) are recipes too: a pattern, a palette, region choices and extra parts. One description gives a helmet for every frame and every rig class.
- **`platypus-art`**, a CLI:
  - `render`: frames to PNG, plus a 4× upscaled preview;
  - `sheet`: a contact sheet of every clip, with anchors and regions overlaid;
  - `gif`: an animation preview;
  - `check`: palette use, stray pixels, anchor sanity, template fit;
  - `import`: Aseprite JSON and PNG.

  The model writes a sprite, renders it, looks at it, fixes it, then tries it in the arena.
- **Rooms and structures** are text grids (§3.3). `platypus-worldview` renders one room, a generated layout, or a structure in place.
- **The arena** (`PLATYPUS_WORLD=arena`), a training ground:
  - a flat floor, walls, some water, lava, a slope;
  - a menu to spawn any creature, character or item;
  - a target dummy that shows damage numbers;
  - hitbox, hurtbox and anchor overlays, slow motion and frame stepping;
  - hot reload of every definition.

  Scenarios can script it: spawn a troll with a greatsword against a dummy, screenshot three frames, assert that hits landed. So new content is tested and seen the same way the sim is.

## 11. Order of work

**World arc (W7)**
1. The plan skeleton, world presets, `platypus-worldview`, and the determinism test.
2. Relief and water: biomes, mountains with snow caps, cliffs, sky islands (port legacy), lakes, waterfalls.
3. Underground: cave layers, caverns, chasms, walker tunnels, underground lakes, the lava sea.
4. Hands:
   - block mining and building, smart cursor, auto tool;
   - item definitions, pickups, inventory, hotbar;
   - chests, loot tables, material patterns.

   Ores and chests need items, so this comes before them.
5. Ores and gems in their bands. (Done: SPEC §3.4d.)
6. Structures (done: SPEC §3.4e; keys and locked doors wait for the RPG arc):
   - the room text format and assembly;
   - ruins leading to crypts, then castles;
   - the ageing pass and secrets.
7. Saving and loading.

**Combat and magic arc (C, now, on branch `magic-arc`; before structures v2 and saving, since it changes what gets saved):**
1. ✅ Casting core (2026-09-26): runes and wands as data, projectiles that collide with cells and bodies, mana, wands as items, starter spells (spark bolt, fireball, acid arrow, flamethrower, wand lightning), cursor aim; the orcs to try them on.
2. Spells and spellcrafting (as agreed above): spells as their own equippable things, scrolls to study, known runes with levels, the spell editor, wands and staffs as tiered foci, orc shamans. Before it: a tuning pass (bigger fireball dropping fire, acid that lasts and eats by hardness, lightning through water, fire as real cells), spell effects (looks as data, animated fire, impact feel), inventory v2 (Esc screen, drag and drop, tooltips, alternative hotbars, first pixel icons), and a channelled levitation / black hole spell with a distortion field (all four done 2026-09-26).
3. Melee and the art pipeline: now its own arc (A, below, on branch `combat-arc`), which also takes in the arena and art tool v1 (M1).
4. (Merged into A.)
5. Creatures per biome (spiders and skeletons in crypts, fungal beasts, slimes in the grottos, crystal golems, trolls), their AI using what they carry.
6. Crafting: stations, recipes, smelting, flasks and potions.

Then structures v2 (bigger crypts and castles, the ruin catalogue) and saving.

**Art and combat arc (A, agreed 2026-09-26, on branch `combat-arc`; `magic-arc` merged into `world-arc` first).** Decisions:
- Humanoids are ~30 px tall (the player's body grows from 9×23 to ~11×26 cells); a troll ~60.
- Combat is a hybrid: swings aimed toward the cursor, a Hollow Knight down-slash pogo in the air, and a light Souls layer (stamina, a dodge with invulnerability, poise and stagger).
- The editor lives in the game, next to the arena; everything it edits is text (`assets/art/*.ron`) that the model writes and reads too, with the `platypus-art` CLI (render, sheet, check, describe, import) as the model's eyes.
- Critters come first, as the pipeline's first test.

Stages:
1. The art format, its compiler and the CLI; the first critters (a rabbit, a bird, a frog) with a critter brain and ambient spawning.
2. The humanoid rig (parts with drawn variants, anchors, clips as data) and the player redesigned on it: idle (breathing, blinking), walk, run, jump, fall, land, dash, wall slide, swim, hurt, death.
3. ✅ The arena (`PLATYPUS_WORLD=arena`: dummies, spawning, overlays, slow motion, frame stepping) and the in-game editor (canvas, palette, layers, frames, anchors, onion skin, live preview) (2026-09-26; SPEC 5.3, 5.4).
4. ✅ The combat core: held weapons turning at the hand (pre-rotated, RotSprite-style), moves as data, pixel-mask hits, hit-stop and feedback, stamina, dodge; a shortsword and a longsword (2026-09-26; SPEC 6.2).
5. ✅ The bow and arrows (drawn by holding, physical arrows that stick and can be picked up, burning arrows) and the orc archer (2026-09-26; SPEC 6.2).
6. ✅ The orcs redesigned (swordsman, archer) and a troll with a club, their AI choosing moves from what they hold (2026-09-26; SPEC 6.2). Done before stage 5, as the user asked.
7. ✅ More critters and ambient life (fireflies that light the night, fish, bats) (2026-09-26; SPEC 5.2).

**Making arc (M1):** `platypus-art` and the arena: now part of A.

**RPG arc (R):**
1. Rigs and skins.
2. Combat moves and hits.
3. Death, corpses and loot.
4. Characters and loadouts.
5. Spells.
6. Darkness as gameplay.

**Then:** co-op, and the Hollow Knight layer (abilities, map, bosses).

## 13. The living world (agreed 2026-09-29)

Forests kept burning down and never came back: the fire physics is right
and stays; what was missing is a world that heals, and one big enough to
live in. The arc, in order:

1. **Dev: reset the world**, two ways: the world only (you and what you
   carry stay) and everything (a fresh start). Two clicks to confirm.
2. **A bigger world, sized from its preset.** `large` becomes 4× wider and
   2× deeper: 196 608 × 49 152 cells (3 072 × 768 chunks), 8× the area,
   ~24 minutes to walk across. Most of the new depth goes to the underground
   (the caverns and the deep), some to the sky. Everything is derived from
   the preset's dimensions from now on (biomes, lakes, structures, chambers
   per width or area; bands as shares of the height; the start and the
   village from the spawn; the clock's region grid), held by "same density
   at any size" tests over `small`, a new `medium`, and `large`. `small`
   stays for tests. Old saves don't carry over.
3. **Fire that spreads like it should**, the physics untouched: living
   plants carry moisture (the biome's humidity, recent rain, a dry spell)
   that makes them slow to catch; dry needles in a drought are tinder, a
   forest after rain barely lights. A storm rains on its own strikes. A
   scenario burns a forest and counts what's lost. Frequent fires are fine:
   the world heals.
4. **The world clock**: one system, a slow simulation of the whole world.
   - **Regions** (384 × 384 cells, ~65 000 in `large`), each with a small
     state (forest health, moisture, lairs' numbers, a village's state, when
     it last changed), saved with the world.
   - **Processes**, each at its own pace (weather fronts about every game
     minute, ecology every game hour), each with two faces: **abstract**
     where no one is (it moves the region's numbers) and **live** where
     someone is (it acts through the real simulation, gradually and
     preferably out of sight: grass creeps at its edges, a sapling grows, a
     raid walks in from the edge; nothing pops into view).
   - **Handing over**: arriving, a region catches up as its chunks load
     (they load beyond the screen's edge, so it's done before it's seen);
     leaving, its numbers are taken from what's there (the burnt cells, the
     spiders left alive). The numbers are the truth for slow things, the
     cells for fast ones.
   - **Weather moves into it**: the fronts, where it's wet or dry, storms and
     droughts and seasons, as a coarse row of regions the whole width; the
     live cloud field (clouds, rain, lightning) only around players, seeded
     and steered by it. The per-tick cost no longer grows with the width.
     Rain on regions no one's in wets their forests, fills their lakes and
     puts out their fires.
   - **Regrowth**: toward the generated world (the pristine chunk, from the
     seed): where there's ash, charcoal or bare soil where there was grass,
     leaves or wood, it comes back: grass in about a day, saplings by the
     second, whole trees by the fifth (~1.7 hours of play). A tree regrows
     whole (it grows), never half of one; nothing grows over what was built.
   - *As built (`world-clock`, SPEC §3.14)*: no region grid yet; each
     process keeps what it needs. Moisture is 512 columns across the world;
     regrowth keeps lost trees by x and healing chunks by position, and
     compares a chunk with itself made again from the seed. Healing happens
     as chunks load (catching up) and hourly; growth shows in view (it's
     gradual), clearing a dead tree waits until no one's looking. A lost tree
     is a sapling the next day and whole by the fifth, a new one of its kind.
     A chunk away an hour comes back with its fires out. A lost tree
     leaves a charred snag until its successor is a third grown. Wildfires
     happen where no one is: they mark trees lost and scorch the ground as
     a function of the days since (the same scar however often it's
     looked at). Lairs refill three days after their last keeper died;
     spreading into nearby caves is still to come.
   - **Processes from the start**: regrowth; moisture, rainy spells and
     droughts; distant wildfires (lightning in a dry region; you find the
     scar, and it heals); lairs refill (and, left alone, spread into the
     caves beside them). Then, with villages and gold: raids (a warband
     marches on a village; there, you fight it; away, it's damaged and
     rebuilds, faster for gold), merchant caravans on the roads, falling
     stars (a crater, rare ore, a guardian; villagers say where), seasons
     (the snow line, lakes freezing), earthquakes (with fracture, §13.1).
     A blood moon: later.
   - *As built (`world-events`, SPEC §3.15)*: one timetable rolled from
     the seed and the day for every kind; live within 1 050 cells, otherwise
     put into the land as its chunks load; the villagers tell the last two
     days' news. Falling stars first: a streak, a blast, a crater with a
     glowing meteorite and mithril, star wisps on a leash keeping it. Raids:
     a warband marching in from out of sight (the villagers hide at home),
     or holes in the houses while you're away; the village mends itself out
     of view by the hour (the guide takes gold to have it done by morning).
     Earthquakes: the screen shakes, cave ceilings round you fall as rubble,
     and a chasm opens at the quake's heart. A travelling pedlar stays a day
     with rarer goods (no roads yet, so no caravans). Seasons dropped: the
     biomes carry the climate.
5. **A start worth starting in**: a wide, flattened plain round the spawn
   (±450 cells, forest or plains), a village a short walk away (the
   structure system's rooms: timber houses, a well, a smithy, a path).
6. **NPCs, as data**: a file per kind (body and look, role, what they say,
   services: sell, craft, heal), a villager brain (a day's schedule, home
   at night, flee danger, talk when you're near), a friendly team that
   monsters hunt too. New roles are new files. First: a guide, a smith, a
   healer; the merchant with gold.
   *As built (`village`, SPEC §3.4i, §5.5)*: the plain is the median of the
   ground ±450 cells round the spawn (a gentle roll), eased back into the
   land; the village's buildings are text (`village.buildings`: a guide's
   house, a well, a smithy with its anvil and furnace, a healer's, a
   merchant's) set side by side from the spawn. A villager is a creature
   file on the player's rig (art `base: "player"`, its own colours) with
   the `villager` brain: potters about its home by day, home from dusk,
   runs from monsters, stops and turns to you when you're near. The one
   nearest you speaks its lines in a bubble; right-click it for its panel:
   buy for gold (click one, Shift ten), the healer heals whole, the
   merchant buys (half what anyone in the village sells it for; a gold
   apiece for the rest, nothing for plain blocks), the guide lists tips.
   The smith sells what it would make rather than crafting to order.
7. **Gold, Noita's way**: a count in the HUD (a nugget and a number), not a
   thing in the pack. Nuggets (1, 5, 25, 100) glint, give a faint warm
   light, bounce and roll, and drift to you; they burst out of the dying
   (not looted) and lie in chests. Gold stays forever (saved with its
   chunk; resting nuggets far off merge, the value kept). Acid doesn't
   touch it (it sinks and waits, glinting); it sinks in water; blasts
   scatter it; heat past its melting point (lava) melts it (at first it's
   gone; with metals, §13.1, into molten gold); the vaporiser and void
   magic erase it; off the world's edge it's lost. Nuggets can be kicked.
   *As built (`gold`, SPEC §3.4g)*: not nugget bodies but a material, gold
   dust (one cell a coin), so it acts as any other: it piles and slides,
   sinks, glitters and glows (a hoard lights a cave, as in Noita), blasts
   throw it whole, lava melts it into molten gold that sets back into dust;
   walked into, it's taken.
8. **Later**: mine carts on rails, stronger travel and building,
   teleport stations.

### 13.1 Matter: metals, casting, objects, boulders, the kick (agreed 2026-09-29)

**Metals.** Every metal (copper, iron, silver, gold, mithril; data, so more
later) has three forms as materials: its ore, molten metal (a hot, glowing
liquid that lights what it touches) and solid metal (heavy, conducts heat
and lightning). Heat moves between them as between ice, water and steam:
ore past its melting point melts into molten metal and slag (not lava, as
now); molten metal below its freezing point sets where it lies. **A bar is
metal**: 36 cells of it, as a block is 36 cells of its material, so ore,
melt, casting and bar are one thing. Mining solid metal gives its bars
back; solid gold gives its gold back. Gold nuggets in lava melt into
molten gold (so many cells for their value): nothing is lost, only moved.

**Casting, the player's own.** A mould is anything you build of a material
that stands the heat (stone holds copper and gold; iron wants firebrick, a
new craftable heat-proof block; mithril the best). Molten metal gets there
two ways: the furnace melts what you feed it and pours from its spout when
you open it (set it over your mould), and a **ladle** (a crucible on a
handle; a new held item) carries molten metal from a furnace or a pool to
pour at the cursor. (A general liquid container: a bucket for water, a
flask for acid and oil come the same way.) It cools and sets in the
mould's shape; mine the mould away and the casting is yours: a free
object in exactly the shape you made.

**Objects.** A free piece (a casting, a boulder, a log, a slab of rock)
stays a rigid body when it comes to rest, asleep (it costs nothing until
touched), instead of turning back into cells, so it can be moved: kicked,
pushed and lifted by force and gravity magic (the well, the force wand;
later a telekinesis spell that holds one), rolled, knocked by blasts,
crushed under. Objects are saved with their chunk. (Big settled
landslides still become cells: a size limit decides.) *As built (SPEC
§3.11)*: an object at rest *is* cells, a thing by its material (`object`):
kicked or pushed, its connected piece lifts out as a body again; so a
hundred sleeping objects cost nothing and are saved as the world is.

**Boulders and falling rock.** Rock pieces as bodies; round ones roll
down slopes. **Fracture**: an impact, a fall, a blow or a blast past a
body's strength (its material's hardness) breaks it into smaller bodies
and rubble (too small to be a body: it crumbles into particles).
**Crushing**: bodies push and hurt creatures by their momentum, and hit
each other. **Traps** (worldgen): a boulder over a tunnel held by a wooden
prop or a rope; a tripwire or a pressure plate (small mechanisms, new)
lets it go.

**The kick** (Noita's): a key (F) kicks what's in front of your feet: an
impulse by its mass to objects, items, gold, bodies, rubble and loose
particles, explosive barrels (kick one down a slope into a camp), small
creatures (a shove). A kick into rock does nothing; a kick with rocket
boots lit is a kick with fire.

## 14. Creatures, encounters and the bestiary (agreed 2026-10-01)

**Where this sits.** The outer loop: first the world, the sandbox, the
systems and the simulation, performant and fun to explore and play with
using the dev tools; then progression (character creation, difficulty,
story, a canonical path the player is free to leave, soft-gated by gear
and stats, as an RPG). Before progression, still to build, in order: this
arc (creatures, encounters, the bestiary and their tools), then authored
places and a world editor that is the game (sites: castles on the scale
of Elden Ring's, hand-built, placed by the world plan; edit and play on
the same world), mechanisms (doors, levers, gates, lifts, wiring), getting
around a big world (a discovered map, rest points as checkpoints and fast
travel, travel), factions with places they hold, and a vertical slice (one
authored castle with patrols, mechanisms and a legged boss, built with the
new tools, measured). Throughout: every editor action has a command-line
twin, so the model can make, render and check what a person can.

### 14.1 Every creature is data; a few also have code

- **Five data layers**, each a vocabulary a creature file picks from:
  - **Body**: size, mass, shape; sprite or a rig of parts; procedural limbs
    (legs that plant and carry the body, chains for tails and necks,
    segments); hit areas per part (weak points); blood; loot.
  - **Locomotion**: walker, hopper, flyer, swimmer, climber, burrower;
    speeds, jumps; a dig method (§14.4).
  - **Moves**: a library like `weapons.ron`: wind-up, active, recovery;
    hit shapes; lunges; projectiles (the arrows and spells we have);
    summons; grabs; beams; area slams. Every move has a tell and an
    animation; a creature lists its moves. As built (`moves`, 2026-10-01):
    `assets/data/moves.ron`, phases named (windup, hold, strike, recover)
    each easing a pose and doing acts (lunge, strike, cast, slam, summon,
    sound, grab and throw, beam), a phase's clip from the creature's art
    (the troll's reach); weapons stay `weapons.ron`'s. The troll grabs.
  - **Behaviour**: one general brain from settings: senses (sight,
    hearing, aggro), tactics (keep distance, flank, retreat when hurt,
    call others), rules for picking a move (range, cooldown, weight,
    conditions). The brains we have (`melee_walker`, `archer`, `swooper`,
    ...) become presets of it. Rules, not a scripting language: easier to
    test, render and reason about; the code hook is the escape hatch.
  - **Phases** (bosses): at a share of health, swap moves, summon adds,
    change the arena (light it, flood it, darken it), transform.
- **Custom modules** for what data can't say, named in the file
  (`custom: "wyrm"`). One file each under `creatures/custom/`, one trait,
  every hook optional: `on_spawn`, `think` (a decision, or none to let the
  general brain decide), `moves` (named moves the data can then use),
  `on_hit`, `on_phase`, `on_death`; one context (the creature, its
  targets, the world, requesting moves, spawning). A file naming a module
  that doesn't exist fails at load. A template with every hook stubbed and
  explained; each module has its scenario.
- **Custom code uses the engine, never goes round it**: its moves still
  have hit shapes, tells, limbs and animation, so every creature gets the
  same arena tools.
- **Promotion**: start in data; write a module for what data can't say;
  when a second creature needs the same trick, it moves into the
  vocabulary. `custom/` keeps only the truly one-of-a-kind. NPCs too: a
  talking villager is data; the pedlar's comings and goings a module.
- **Layout**: `crates/game/src/creatures/` with `body/` (sprites, rigs,
  limbs, hit areas), `locomotion/`, `moves/`, `brain/`, `custom/`
  (`mod.rs` the registry, `_template.rs`, one file per creature); every
  creature's file in `assets/data/creatures/`.
- **As built (BE 1, `creatures`)**: the layout is in place (`body/`,
  `brain/`, `moves/`, `custom/`; locomotion still in `platypus_physics`).
  Behaviour is one brain, `hunter`, from choices (`close`: Walk, Range,
  Swoop, Hop, Crawl; `attack`: Touch, Swing, Shoot); the old five brains
  are its settings. Picking among moves by rules comes with `moves` (BE
  2). A module is a component read from `params` plus whatever systems
  and observers it needs (`CustomCreature::build`), its thinking in
  `CustomSet::Think` after the brains: plain Bevy rather than one trait of
  hook methods, so a hook is only what it uses. An unknown brain or
  module is a loud error at start (not a refusal to load). Modules now:
  `dummy`, `explosive`, `hatchery`.

### 14.2 Damage types, resistances, and kinds

- **Ten damage types**, each with sources in the world: slash (swords,
  claws), pierce (arrows, thrusts, stingers, shot), blunt (clubs, kicks,
  falling rock, boulders, blasts), fire, frost, storm (lightning, charged
  water), acid, poison (venom, blight), radiant (light), void (the void
  wand, necromancy). Fall stays special (not an attack). The player's gear
  uses the same list: "fire resistance" on armour and "weak to fire" on a
  beast are one number seen from two sides.
- **One profile per creature** (and the same model for gear, replacing
  `Ward` and `Resist`): a multiplier per type (0 immune, 0.5 resistant, 1,
  1.5–2 weak, negative: it absorbs, healing); what it can't suffer
  (burning, chill, poison, webbing, stagger); what touching a material
  does to it.
- **Kinds as templates**, adjusted per creature: beast (weak to fire);
  humanoid (neutral); insect/arachnid (poison-resistant; fire and frost
  hurt, cold slows); undead (pierce and slash resisted; blunt, radiant,
  fire hurt; no poison; void heals); spirit (most physical resisted;
  radiant hurts; no blood); ooze (blunt resisted; fire hurts; absorbs what
  it's made of); construct (slash, pierce, storm resisted; blunt and acid
  hurt; no poison); elemental (absorbs its element; the opposite hurts).
- **Hard immunities are rare and obvious** (a skeleton can't be poisoned);
  otherwise strong resistance, so a determined player can brute-force it.
- **Absorbing is rare, on theme, and always visible** (§14.3), and works
  through the sim: an acid spider resting in acid heals (drain the pool,
  lure her out, freeze it); a fire wisp grows stronger in burning grass.
- **Regeneration as data**: a rate, and what stops it for how long. The
  troll regenerates unless fire or acid hurt it in the last 5 s: plain
  steel can't win, a torch can.
- **As built (BE 1)**: `nature.rs` and `kinds.ron`, the kinds above
  (elemental as `starfire` for now; no `arachnid` apart from `insect`).
  Gear keeps its `Ward` for slash, pierce and blunt, before the
  creature's multiplier; gear on the same profile comes with the gear's
  own arc. `regen` with `undying`: while it heals it can't go below 1 hp,
  so steel holds the troll at 1–3 hp and fire finishes it.

### 14.3 Seeing that it hurts (no health bars)

- **Reactions scale with the share of maximum health taken**: flash, how
  much blood sprays, knockback, the hit sound's pitch. A big hit on a small
  thing is a big reaction; the same on a colossus, small.
- **Wounds that show**: under half health it drips its own blood (ichor,
  ectoplasm, sparks); under a quarter it staggers, slows, breathes hard.
  (No darkening: decided 2026-10-01.)
- **Three distinct hits**: hurt (flash, blood); resisted (dull sparks, a
  clang, no flash, no blood); absorbed ("it drank that": a glow, the drip
  stops). Damage numbers stay an option.
- All from what every creature's data already has (size, blood, maximum
  health, its profile): no per-creature work.
- As built (`moves`, 2026-10-01): the three told apart by what got
  through against what was meant (half or less: resisted; below 0:
  absorbed); staggering is a falter (75 % speed). Every hurt from every
  source is kept by kind on the body and tallied once a tick: what the
  player's observations (§14.7) and the arena's readouts are made of.

### 14.4 Moving through the world: a path planner, and digging

- **A shared path planner**: a coarse grid over the nearby terrain;
  walking, climbing, jumping, swimming, flying each with a cost. Every
  creature navigates better for it.
- **Digging**, any creature's, by method and strength (the hardest
  material, how fast): claws (break the cells ahead, as a pickaxe:
  spiders, zombies at wooden doors), acid (spit at what's in the way and
  let the sim eat it: acid spiders, acid slimes), tunnel (swallow through
  soft ground: the sand wyrm, the centipede), blast (bombs: an orc
  sapper). For diggers, breaking a cell costs more the harder it is: dirt
  before stone, never what it can't break.
- **A tell**: scratching that grows louder, dust trickling from your
  ceiling, cracks, a hiss of acid behind the wall.
- **Building matters**: wood doesn't stop a spider, stone slows it,
  obsidian or crypt stone stops it, glass (`inert`) is acid-proof; most
  diggers won't swim (moats), lava trenches.
- Capped per creature and per frame: a swarm can't flood the sim with
  edits.

### 14.5 Bodies of parts, limbs that carry them, wounds you can see

- **Big creatures are made of parts**: a few drawn pieces (head, jaw, body
  plates) with generated limbs, chains and segments; the motion comes from
  the rig, not frame-by-frame drawing. Big parts are palette PNGs beside the
  text files; the rig, poses and moves are text.
- **General procedural limbs** (the spider's grow up): side-view legs (2,
  4, 6) whose planted feet hold the body (its height and tilt follow the
  ground); chains (tails, necks, a wraith's chains); segments (worms,
  wyrms); moves can drive limbs (a stomp lifts a leg and slams it).
- **Every limb and part can have its own health** (a share of the
  creature's; some damage passes to the body), hit where it's drawn, and
  can be **severed**: it comes off as a piece of its own that falls and
  lands; the stump bleeds the creature's own blood (a spider's acid, which
  pools and eats the floor). The creature carries on without it: fewer
  legs, a slower limping gait (the step rules already cope); below a
  threshold it can't climb (shoot the legs off a spider on the ceiling and
  it falls); a scorpion without its tail loses its sting; a zombie without
  legs crawls; a head can be a weak point; a boss can have parts designed
  to break (the colossus's legs). Spirit parts can be marked unbreakable.
- **Wounds carved into the picture**: each body part has depth (skin, fur,
  chitin, stone or cloth outside; then flesh; then bone), and a hit
  removes pixels where it landed: shallow shows flesh, deeper bone, deep
  enough a hole. Each damage type marks its own way: pierce a round hole
  with blood spurting along the shot; slash a cut along the swing; blunt a
  dent and limbs knocked loose; fire charred black; acid pixels eaten away
  with a green edge; frost frosted. What's inside comes from the kind
  (flesh and bone; ichor; stone with cracks; ectoplasm; goo): no gore art
  per creature. Small, the edges darkened a little: visible and
  satisfying, not cartoonish. Stored per part in its own coordinates (it
  stays put as the part moves), updated only on a hit; bodies keep their
  wounds. Zombies are the showcase.

### 14.6 The creature editor: from a sketch of the whole to a creature

One editor, five modes you can go back through; the whole creature is
always in view:
1. **Sketch**: a free canvas with layers (sketch, colour, shading, notes),
   the creature at its real size in cells.
2. **Slice**: select regions (box, lasso) and make them parts; set each
   pivot. The sketch stays as a faint layer underneath.
3. **Rig**: connect parts with joints; draw a line from a hip to the ground
   and it's a leg of that length; draw a curve and it's a chain.
4. **Pose and moves**: poses for each move, on the whole creature with its
   limbs working, onion-skinned.
5. **Test**: the live preview stage (§14.7), then fight it in the arena.

Editing a part is in place: click it on the assembled creature, the rest
dims, the whole updates live. Everything it writes the model can write
and render from scripts too.

**Image models (a spike, go/no-go)**: concept images from an image model
as a starting point, never finished art. Ask for the right image (side
view, flat shapes, few colours, plain background, whole in frame, or a
sheet of parts); convert (cut out the background; shrink by the commonest
colour per block, not by blurring; snap to the game's palette; a one-pixel
outline; remove stray pixels), with size and palette knobs and the result
beside the original. It lands in the sketch layer. Expected good for
props, objects, items, silhouettes, colour schemes and big creatures to
slice; less so for small sprites and animation frames. Castles: concept
images as a reference layer, not converted. Once an image API is
available: a dozen creatures and props run through it, shown side by
side, then decide.

### 14.7 The bestiary and arena v2

- **One bestiary panel, two homes**: the arena (pick something to fight)
  and later the world editor (place a spawn in a site). Cards: a portrait
  from its art; name, kind, size, health; a badge if it has custom code;
  filters (kind, biome, tier) and search. Expanded: a **live preview
  stage** (a strip of real terrain, the real creature, rendered to a
  texture) cycling idle, walk and each move with its tell and hit shapes
  drawn over; stats, profile, weak points, loot; moves (timings, damage,
  range); phases; its file. Buttons: place in the arena, fight it (a chosen
  loadout), open its file, reload.
- **Arena v2**: layouts (flat, cave, slopes, stairs, and a copy of a real
  piece of the world: "fight it here"); creature and move files reloaded
  live while fighting; readouts (a timeline of hits, damage per second,
  damage taken, by type); recorded inputs replayed, so a fight is a
  regression test. (Readouts built in `moves`: the arena panel's Fight
  section, to them and to you by kind, per second, a 20 s timeline. As
  built in `bestiary`: layouts flat, cave, slopes, stairs and the real
  world's terrain at a seed and place; files reloaded into the fight;
  fights recorded frame by frame and played back headless as a test,
  matched within 15 %.)
- **`platypus-bestiary`** (command line): the same cards and preview strips
  rendered to images, stats listed: the model checks a creature it wrote
  without opening the game. (As built: the game itself run headless, its
  real cards and stage caught: `bestiary.md`, a card and a strip each.
  The panel's filters are part and kind; biome and tier wait for the
  world to say where things live. The live stage is the arena's; outside
  it, a card shows its picture.)
- **The player's bestiary** (the progression arc, later; the data recorded
  from now): the same panel filtered by what the player knows. Seen (a
  silhouette, a guessed name); encountered (the real name and picture, and
  observations written from what happened in their fights: "It drank in
  the acid", "Arrows did little", "The flames stopped its wounds closing",
  "It came apart when struck hard"); studied (the full profile, from books
  bought from the pedlar or a scholar or found in crypt and castle
  libraries: one per kind, rare ones per boss; killing enough fills in
  some). Recorded since `moves` (`observe.rs`, saved with the player):
  per kind, met, moves seen, what they carried, how each kind of hurt
  landed, what felled them, healing seen and what stopped it.

### 14.8 Bosses as puzzles

Each boss uses only systems the player knows, and gets: a preparable
weakness (an element, a resistance, a weapon type); an arena trick
(something in the room that matters); an emergent route the sim allows; a
hint in the world (villager news, the arena itself, an old note). Its
scenario plays the intended solution once.
- **Broodmother**: webs are a flammable hanging material, so fire burns
  you free (bring fire resistance and a torch); her lair is full of web,
  so lighting it burns her; she heals in her acid pools; frost slows her.
  "Webs burn, if you're brave enough to stand in the fire."
- **Necromancer lord**: raises skeletons from the dead, so burn or crush
  the bodies first; radiant light weakens him; void heals his minions;
  phases: adds, a darkened arena, at last a wraith.
- **Ruin colossus**: stone (swords scratch; hammers, bombs and acid work,
  acid really dissolving its plates); break a leg and it falls, its core
  bare; lure it onto thin ground and blow the floor away.
- **Sand wyrm**: burrows through sand and dirt, not stone: build a stone
  floor and it must surface; water makes its sand mud.
Emergent, from the sim as it is: lure orcs into a pool and zap it; kick a
powder barrel into the adds; a grass fire upwind of a camp; a boulder trap
on a troll; a dead bloat toad's gas lit beside a crowd.

### 14.9 Firearms, black powder

- **Guns**: a flintlock pistol (quick, short); a musket (slow reload, long,
  hard-hitting); later a blunderbuss (a spread up close) and a hand cannon
  (an exploding shell).
- **Gunpowder is a material**: charcoal (we have) and **sulfur** (a new
  mineral near the underworld's lava: guns are a mid-game find). Poured in
  a trail and lit it burns along like a fuse; a barrel explodes; wet, it
  fails; a stray spark sets it off.
- **Shot**: iron (pierce), silver (radiant: for the undead), fire
  (incendiary).
- **Firing**: a muzzle flash that lights the scene, real smoke into the
  sim, recoil that pushes you (a blunderbuss in mid-air: a jump), a reload
  to time, real wounds where the ball lands.

### 14.10 New harmful liquids

- **Blight**: glowing purple sludge (the Watcher's spit). It doesn't eat
  stone; it poisons what it touches, withers grass and leaves, and slowly
  gives off a toxic mist.
- **Ectoplasm**: what wraiths leave: pale, glowing, sticky; slows you;
  drips from ceilings.

### 14.11 The roster that proves it

Chosen so each stresses a different part of the system:
- **Necromancer**: keeps away, bolts, raises skeletons, blinks away when
  cornered (spell moves, summons, retreat).
- **Risen skeletons**: claw out of the earth (summoned spawns, a rise).
- **Zombies**: slow, undead, hard to kill; pierce barely slows them, a
  headshot drops them, blunt knocks limbs off; claw through wooden doors
  (wounds, severing, weak points, digging).
- **Chain wraith**: night only, drifts through walls, chains swinging from
  its wrists, whips one to pull you in; can't cross running water (spawning
  by time, phasing, chains, a grab).
- **Watcher**: a floating eye; keeps its distance, follows you with its
  gaze, spits blight, charges a beam with a long tell; blinded by bright
  light (hovering, aiming, a beam).
- **Stilt stalker**: a small body on very long legs, stepping over
  boulders, stabbing down with a leg (long-legged limbs on rough ground, a
  leg as the weapon).
- **Crag crab**: six legs from the side, sideways, two claws, an armoured
  front, a soft back (side-view legs, attacks by limb, weak points).
- **Scorpion**: a segmented tail arched overhead aiming its sting; poison
  (chain aiming).
- **Cave centipede**: segmented, on walls and ceilings, drops on you
  (segments, ceiling climbing, tunnelling).
- **Cave spider** (reworked): legs with their own health, acid blood,
  claws and acid to dig to you.
- **Bloat toad**: hops; a tongue that grabs from range; bursts into
  flammable gas when it dies (hopping, a long grab, death reactions).
- **Splitting slime**: splits in two when killed (spawn on death as data).
- **Shield orc**: blocks from the front; flank it, kick it, break its
  guard (armour from one side, stagger).
- **Orc sapper**: bombs walls to reach you (blast digging).
- **Mimic**: a chest that bites (an ambush; a small module).
- **Bat swarm**: flocks and splits round you (group behaviour).
- **Bosses**, one at a time: the Broodmother, the Necromancer lord, the
  Ruin colossus, the Sand wyrm (§14.8).

## 15. Open questions and risks

- **Character pixel size (D5):** 27 px reads like Noita. Elden Ring-style gear may want about 36 px. Decide from mock-ups. The cost is zooming out a little and loading more.
- **Gear skins might look generic.** Fallback: hand-drawn per-frame gear for hero items only.
- **Corpses as cell bodies:** a rendering change. Bodies must carry per-cell colours (their sprite pixels), not just material shades.
- **Weather cost** grows with width. Plan: the coarse weather moves into the world clock (§13); the live field only round players.
- **Freezing the save format:** structures and items must be in before saving is finalised. That's why saving is step 7.
