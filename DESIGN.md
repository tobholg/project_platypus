# Design: the world, hands, RPG, and making things

Agreed 2026-09-25: D1–D4 and D6 as recommended, D5 left to mock-ups. Built on
the `world-arc` branch (off `v2`), so we can go back and try another path. Each
part moves into SPEC.md as its arc lands. Numbers are starting points.

## 0. Decisions to make

| # | Question | Decision |
|---|---|---|
| D1 | How big is the world? | 32 768 × 16 384 cells (512 × 256 chunks), with smaller presets for testing. |
| D2 | What unit do mining and building work in? | Blocks of 4 × 4 cells on a fixed grid. The world stays cells. |
| D3 | How are humanoids drawn? | Terraria-style: layered frame sheets on one shared frame layout per body size. Gear is a "skin" painted onto body regions. Only held items rotate. |
| D4 | How do you loot the dead? | The corpse becomes a physical cell body that holds the loot. Interact to loot it. Destroy the corpse and the loot scatters. |
| D5 | How big are characters? | Keep 1 art pixel = 1 cell. Decide 18 px vs ~24 px humanoids from rendered mock-ups when the RPG arc starts. The 4 × 4 block works for both. |
| D6 | In what order? | World plan and viewer → terrain → hands (mining, items, chests) → ores → structures → saving → art tool and arena → RPG. |

## 1. Principles (carried over)

- **Cells are the truth.** Everything physical is cells: terrain, built blocks, corpses. (Furniture is the exception: entities with bodies.) Special cases become material data plus general rules.
- **Data over code.** Creatures, items, moves, loot tables, rooms and structures are RON or text files, hot-reloaded.
- **Anyone can use anything.** A weapon, a spell or armour works the same for the player and for any creature with the body to use it. The player is a creature with a keyboard brain (already true).
- **Deterministic and chunk-pure.** A chunk is a pure function of (seed, position, plan). Co-op and saving depend on it.
- **Everything can be looked at.** Every generator and asset has a headless render the model can open as an image, and a sandbox to try it in.

## 2. Scale

At 1 cell = 1 art pixel, the player is 8 × 16 cells and runs 95 cells/s.

| | Cells | In player heights | Terraria large (in player heights) |
|---|---|---|---|
| Width | 32 768 | 2 048 | ~2 800 |
| Height | 16 384 | 1 024 | ~800 |

Crossing the world on foot takes about 6 minutes. Vertical bands (sea level at about 25 % from the top):

| Band | Height (cells, relative to sea level) | What's there |
|---|---|---|
| Sky | +2 500 … +4 000 | Sky islands, shrines on them, the cloud band, wyverns later |
| Peaks | +800 … +2 500 | Mountains above the snow line (the climate makes snow, it isn't painted on), castles, cliffs |
| Surface | −200 … +800 | Biomes, forests, lakes, ruins, crypt entrances |
| Underground | −200 … −2 500 | Caves, mines, ores (copper, iron), crypts |
| Caverns | −2 500 … −7 000 | Huge chambers, underground lakes, crystal and mushroom caves, silver and gold |
| Deep | −7 000 … −11 000 | Chasms, the deepest ores, old ruins |
| Underworld | −11 000 … bottom | A lava sea, obsidian, heat |

Costs that grow with the world:
- **Weather** spans the width: about 0.6 ms per tick at 32 768 wide after the lane fix. Away from players it can run at 8-cell texels.
- **The plan** is per-column arrays plus fields at 1/16 resolution (2 048 × 1 024), a few MB.
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

- **Rooms are text files.** Each is a grid at block resolution (one character per 4 × 4 cells), with a legend mapping characters to materials and markers:
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
  - the whole world at 1:16;
  - any region at 1:1;
  - overlays for biomes, bands, structures, ores and chests.

  The model reads these images to check its own changes. This is the main loop for tuning generation.
- **A determinism test.** Generate chunks in different orders and on different threads, then compare checksums.
- **World presets.** `small` (8 192 × 4 096) for tests and scenarios, `large` for play.

### 3.5 Saving

A save is a directory:
- `world.ron`: format version, seed, preset, tick, weather state;
- the chunk store: modified chunks, lz4, in region files of 32 × 32 chunks;
- `entities.ron`: creatures, items on the ground, chest contents, bodies in flight;
- per-player files: inventory, equipment, position.

The plan isn't saved; it's regenerated from the seed. A version bump migrates or refuses to load.

## 4. Hands: mining and building

The current per-cell radius pickaxe removes an uneven blob and gives nothing back. Terraria feels good because every hit has a clear target, the result is predictable, the feedback is immediate, and the progress adds up.

### 4.1 Blocks

- The grid is fixed at 4 × 4 cells (D2): block `(x >> 2, y >> 2)`. The player is 2 blocks wide and 4 tall. Terraria's is 2 × 3.
- **Mining a block.** Each hit adds damage to the block; cracks show. When it breaks, every cell in the block that the tool can mine is removed at once. Harder cells, such as ore in a weaker tool's hit, stay.
- **Generated terrain stays cell-detailed.** A block at a cave edge may be half air. Mining it yields what was there.
- **Yield.** The inventory counts materials in cells and shows whole blocks (16 cells = 1 block). Partial blocks add up, and nothing is lost to rounding.
- **Placing a block.** It fills the empty cells of one grid block, using 16 cells of material.
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

- The player's box became 6 × 15 cells (was 8 × 16): exactly two blocks wide,
  it never fit a 2-wide shaft unless perfectly aligned with the grid.
- Chests became furniture entities (2026-09-25): as cells they hung in the
  air when their floor was mined out, and a rigid body of chest cells would
  land tilted and lose its identity (contents were keyed by the corner).
- Building pushes aside tall grass, smoke and flames.

### 4.3 Items on the ground

Mined material and drops are item entities. They fly out a little, settle, and drift to a nearby player (a magnet). They're also physical: they burn, sink and float by their material. A scroll burns; an iron sword doesn't.

### 4.4 Patterns

A material can have a `pattern`: a small tile of shade indices anchored to the world grid (for example 8 × 4 for bricks). Cells of that material draw their shade from the pattern instead of at random. Built walls, castles, planks and cobble then look like Terraria tiles without being tiles.

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

- **Humanoid rig classes:** `small` (goblin, ~12 px), `human` (player, bandits, knights, ~18–24 px), `large` (troll, ogre, ~36–48 px).
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

(C1 built wands that hold their runes; C2 turns them into spells + foci.)
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
- **Slots:** worn: head, body, hands, legs, feet, two trinkets. Held: the
  hotbar item in the hand counts while it's there (weapons, tools, foci).
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
- **Spells and foci:** spells come out of the wands into a spell bar (Q
  cycles); wands and staffs are held foci with a tier (gating spells) and an
  element they favour. A focus is needed to cast (see §7b).
- **Enemies wear gear** from their loot tables, and what they wear is what
  they drop.
- Not now: durability and repair, coins and merchants.

Build order (a commit each): stats + equipment → skins → rarity and bonuses
→ corpses and containers → spells and foci → enemies equipped and example
content.

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
- Humanoids are ~20 px tall (the player's body grows from 6×15 to ~7×17 cells); a troll ~40.
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

## 12. Open questions and risks

- **Character pixel size (D5):** 18 px reads like Noita. Elden Ring-style gear may want about 24 px. Decide from mock-ups. The cost is zooming out a little and loading more.
- **Gear skins might look generic.** Fallback: hand-drawn per-frame gear for hero items only.
- **Corpses as cell bodies:** a rendering change. Bodies must carry per-cell colours (their sprite pixels), not just material shades.
- **Weather cost** grows with width. Plan: coarse weather far from players.
- **Freezing the save format:** structures and items must be in before saving is finalised. That's why saving is step 7.
