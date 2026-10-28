# Plan

Each phase ends with something playable or measurable. See SPEC.md for the design.

**Now (2026-10-28):** the world and the sky (WS, below): a world that
loops round, a sky of layers up to space, zero gravity, and a black hole.
On `main` (c816ff8) since the prologue was planned: `play-mode` and
`starter`, building (`blocks`), the enemies pass, physics caves, the hero
art, the combat loop (`rats`), painted spells (`painting`), the waters
(sealed flooded caverns, great halls, underground seas with tubes, mild
caverns, glowfish shoals, the drowned, the angler, the cave eel, the
diving helmet) and the swimmers (stroke-by-stroke swimming for sea
creatures, the leviathan rebuilt, the repulsor ward, Q as the right
button). The prologue (PR) has its first branch built (`play-mode`);
`sites` through onboarding are still ahead. The bestiary (BE) is paused
with `roster` mostly done (shield orc, orc sapper and bat swarm left) and
`bosses`, `creature-editor` and `powder` ahead.

## The world and the sky: implementation plan (WS, agreed 2026-10-28)

The world loops: walk west far enough and you come back from the east.
The two edge oceans are one great ocean across the seam, sailed or swum
across without a join; under it, and everywhere underground, the world
runs on across the seam as anywhere else. Above, the sky is layered: the
cloudsea with the sky islands rising out of it, the high air, and space,
gravity fading through the high air to none. Space is late game (a suit,
a jetpack, stabilisers), but a clever player can get there mid-game
(an updraft, rocket boots, the diving helmet's air) and struggle there.
Five ores beyond mithril, and one black hole.

Decided (2026-10-28):
- The loop is a world that wraps at the storage level (cells, generation
  and the sim work modulo its width; a chunk's neighbour across the seam
  is just a neighbour), with game positions unbounded and recentred:
  crossing the seam, everything loaded (the player, creatures, items,
  particles, the camera) shifts by one world width in the same frame
  (floating-origin rebasing); what's saved is stored modulo the width and
  spawns on the side nearest the player. The few systems that remember
  positions across frames (swimmers' ways, legs' trails, navigation, the
  camera's glide, cloth) take the shift.
- Generation wraps: per-column plans (biomes, the surface, lakes, water
  tables, chasms, structures) wrap; noise meets itself at the seam;
  caves link across it; the stretch at the seam's two sides keep a dam
  between them like any two.
- The sky islands and the cloudsea are one layer: the islands are rock
  rising out of a deck of cloud (a new half-solid material: you wade and
  sink in it, light things float on it, cloud creatures swim in it); rain
  forms in the deck and falls from its underside; holes in it (sky
  windows) look down on the world far below.
- The high air: jet-stream currents, then thinning, freezing air (breath
  runs down; an aurora); gravity fades from full to none through it
  (jumps get higher, falls slower).
- Zero gravity is fully Newtonian without space gear: no drag, momentum
  real, every push has its push back (a jump off rock, a thrown bomb, a
  force spell, a grappling hook, a heavy swing, rocket boots thrusting
  out of the feet wherever they point), and the player tumbles (off-centre
  pushes spin it; nothing but a surface stops the spin). Space gear tames
  it: a suit (air, cold, vacuum), a jetpack (thrust any way, fuel),
  stabilisers (spin cancelled, damped control), magnetic boots (walk on
  any rock, the climbers' movement), a tether.
- Ores beyond mithril, each a property not only numbers: aerolite (sky
  islands: feather-light gear), fulgurite (stormglass, where lightning
  strikes near the storm: storm-charged, it zaps you wet), starmetal
  (asteroid cores: the densest plain gear), solarite (sunlit asteroid
  faces, hot: radiant, self-lit), nullstone (only in the black hole's
  disk: gravity itself; the endgame metal).
- One black hole, fixed in space, modelled on Gargantua: the horizon
  (~120 cells across), the photon ring, the accretion disk near edge-on
  with its far side lensed over and under, Doppler-bright on one side,
  the stars and what's near bent round it (the warp pass's kind of bend);
  the whole ~700-900 cells, most of it on screen at the widest zoom.
  Nearing it: gravity growing, time slowing (the slow field's kind),
  tidal hurt near the horizon; the disk is the mine (nullstone, wrecks).
  Crossing the horizon kills, for now (the tesseract, a time jump and an
  ending kept as ideas, the choice left for later).

Branches in order; each merges to `main` when it holds.

**1. `wrap`: the world loops.** Generation that wraps (per-column plans,
noise, caves, water tables, structures and chasms away from or across
the seam); the sim's storage modulo the width (chunks, neighbours,
particles, bodies, weather); unbounded positions recentred on crossing;
the save modulo the width; the map wraps.
Done when: walking, swimming and digging across the seam is seamless
(the cells either side meet; water flows across; a creature chases you
over it); the great ocean joins; tests of the generation's seam and the
sim across it.

**2. `sky`: the cloudsea and the high air.** The cloud material and the
deck, the islands rising from it, sky windows, rain from its underside;
the high air's currents and cold; gravity fading with altitude (creatures
and cells); aerolite and fulgurite.

**3. `zerog`: weightless.** Newtonian zero gravity for the player and
creatures, recoil from every tool, tumbling; cells that don't fall above
the line (water in blobs, drifting dust); breath and cold in vacuum.

**4. `space`: the asteroids.** Asteroid fields, starmetal and solarite,
sun and shade (scorching and freezing), meteor showers down through the
layers (craters, meteorite ore), the space gear, a creature or two, the
planet seen curving below (the world's own map).

**5. `singularity`: the black hole.** Its look (lensing, the disk, the
ring), its pull, slowed time, tidal hurt, the disk's nullstone and
wrecks; crossing kills.

## The prologue: implementation plan (PR, agreed 2026-10-15)

The first hour, playable from a new game: you wake in a cell in the
dungeons under a giant wizard's tower (other cells about, some with people
in them you can talk to); something goes wrong in the tower above and you
escape with nothing but rags, into the sewers (a maze with dead ends that
hold something worth the detour), where you find a first weapon and a
pickaxe and fight rats and skeletons, and come out of a cave behind a
waterfall into the start valley in sunlight; then a Terraria start: a first
shelter before the first night (nights are dangerous), gear and ore, the
underground. The scaffold for playing and testing the game as a player.

Decided (2026-10-15): the tower stands at the edge of the start valley,
seen from it (you come out under its waterfall nearby; it's the looming
goal to come back for); the intro route is hand-authored (fixed pacing)
and placed into the generated world (procedural sewers maybe later, for
replays); the dialogue is drafted by Claude in a plain text format and
edited by Tobias; a fellow prisoner escapes too and is found camping in
the valley, the first NPC there (guide, goals). The intro shows the
simulation off: the tower's experiment goes wrong above (a quake, dust and
sand through the cracks, the ceiling falling in pieces, a cistern bursting
and the cell block flooding as you climb out, straw alight, steam where
the fire meets the flood); a swamp-gas pocket in the sewers lit from afar
blasts a way through (or kills you if you bring a torch to it); an oil
slick burning on the water; the dark, lit by a stolen torch; sunlight
through the falling water at the end. A rope hook (a lesser grappling hook:
thrown in an arc, short, catches only on ledges and corners, climbed, no
reel or swing) found in a side passage gets you up a ledge on the main way
that's too high to jump; the real hook later makes such ledges trivial.

Branches in order; each merges to `main` when it holds.

**1. `play-mode`: playing as a player, and a dev mode worth having.**
A title screen (the working title, New game, Continue, the dev world, the
arena), save slots, a new game in a humble kit (rags and a little), dev
tools off; real death and respawn, checkpoints. The dev mode refactored
whole: one structured, hierarchical menu (player cheats, items by category
and search straight into the pack, creatures to place, the world: time,
weather, events, resets; the dev tools and their settings; overlays; the
arena's time controls; the editors), replacing the dev panel, the arena
panel and the dev hotbar's scattered keys.
*Built (2026-10-15, `play-mode`):* `mode.rs` (play or dev, kept in the
save), kits in `items.ron` (play, dev, miner, fighter, mage, smith),
`respawn.rs` (dying in play: "You died", waking at the checkpoint, kept in
the save), `devmenu.rs` (tabs: Player, Items, Creatures, World, Tools,
View, Time, Sounds; replaces the dev and arena panels), `title.rs` (the
title over the last save, paused; Continue, New game, Load, Dev world,
Arena; the name in `game.ron`). Rags are the prologue's to give; until
then a new game starts in the valley with the play kit.

**2. `sites`: authored places in the generated world.** The world plan
places hand-authored sites (constraints, reserved space, blended into the
terrain); sites drawn as pixel images (a palette: a cell's material a
pixel, marker colours for spawns, people, chests, triggers), editable with
the art tools; what's opened, broken or taken stays so (saved). Then the
route in greybox: the tower, the cell block, the sewers, the waterfall
cave, walkable end to end.

**3. `interact`: interactables and mechanisms.** Doors (open, shut, locked,
keys), levers and valves, grates, a talk prompt, notes and signs, pickups
placed by sites.

**4. `story`: world state and scripting.** Flags and quest state (saved);
triggers (entering a place, picking up, a conversation ended, a flag);
scripted sequences (camera shake, world edits, spawns, doors, starting a
conversation). It runs the escape, what dialogue choices do, the guide's
goals.

**5. `dialogue`: talking.** Conversations as plain text (lines, speakers,
choices, conditions, effects); a panel sliding in from the right (the one
you're talking to large, from its art; the text typing out; choices by
mouse or number); villagers and prisoners spoken to.

**6. `water`: waterfalls and breath.** A spring and a drain (a waterfall
that runs forever and doesn't flood the world; walking through it wets
you); drowning and breath; iron bars, straw, wooden supports.

**7. `prologue`: the intro itself.** The cells and their people, the
escape set piece, the sewers (rats: a new swarming critter; skeletons; the
first weapon and the pickaxe; the rope hook; the gas pocket, the oil, the
dead ends), the waterfall cave, the valley arrival, the escaped prisoner.

**8. `survival`: the first nights.** Doors and beds in crafting (a bed sets
where you wake), a shelter check (an enclosed room, back wall, door, light:
nights are dangerous outside, safe in), night pressure, a death penalty.

**9. Onboarding.** Prompts as they're needed ("E to talk", climbing), a
pause menu, key bindings. Then play it, and tune.

Done when (the arc): a new game plays from the cell to the first night in
a shelter in the valley without the dev tools, saving and resuming on the
way.

## The living world: implementation plan (L1, agreed 2026-09-29, DESIGN §13)

Seven branches, in order; each merges to `main` when its "done when" holds
(tests, clippy, the bench, its scenarios and screenshots). Save format
versions rise where marked; an old save is refused, a new world made.

**0. Merge `backdrop-arc` into `main`.** 169 commits: the sky, the hotbar,
coatings as shares, fire as a share, explosives and camps, spiders, the
pickaxe's area mode, the world reset. Done when `main` is green.

**1. `world-scale`: an 8× world, sized from its preset.**
1. Presets: `large` 2 048 × 512 chunks (131 072 × 32 768 cells); a new
   `medium` (512 × 128) for tests; `small` stays. The bands as shares of
   the height: most of the new depth to the underground (the caverns and
   the deep taller), some to the sky; the surface band as before.
2. Audit worldgen for fixed counts and sizes, each derived from the
   preset: biomes (their sequence and widths), mountain ranges, lakes,
   sky islands, chasms, crypts and castles, lairs, cave chambers (per
   area, already), ores (per chunk, already), camps; loot's depth scale;
   the spawn and the start's enemies.
3. "Same density at any size" tests over small, medium and large: biomes,
   lakes, structures, chambers, chasms per width or area within a range.
4. The weather, until the clock takes it: its field only over a window
   round the players (frozen outside it), so its cost stays flat.
5. Look at it: `platypus-worldview` of the whole world and slices; walk it.
6. Save version 2 (a new world; `large-1` is left behind).
Done when: the tests hold at three sizes, the bench is unchanged, a large
world plans in under a second and plays as smoothly as today.

**2. `world-clock`: the slow simulation of the whole world.**
1. The clock: game time in days and hours; regions of 256 × 256 cells with
   a small state each (saved with the world, version 3); processes, each
   with its pace and two faces (abstract: the numbers; live: through the
   simulation, gradually, out of sight first); the handover (catch up as
   chunks load, beyond the screen; take the numbers back as they go).
2. Weather into it: fronts, wet and dry, storms and seasons as a coarse row
   of regions across the world; the live cloud field only round players,
   seeded and steered by it; rain on unloaded regions wets them and puts
   their fires out.
3. ✅ Moisture and fire: living plants slow to catch by the region's wetness
   (the biome's humidity, recent rain, a dry spell); a storm rains on its
   own strikes; `forestfire` scenario: a strike in a dry and in a wet
   forest, the cells lost counted.
4. ✅ Regrowth toward the generated world: grass (~1 day), saplings (~2),
   whole trees (by 5: a tree grows whole), never over what's built; live,
   grass creeps at its edges and saplings grow; `regrow` scenario (burn,
   skip days, look).
5. ✅ Distant wildfires (lightning in dry regions no one's in: the scar is
   found, and heals); lairs refill after they're cleared (and, left alone,
   spread into the caves beside them).
6. ✅ The dev panel: a day ahead, the region under the cursor (its numbers).
Done when: a burnt forest is back in 5 game days whether you stayed,
left, or came back halfway; fires spread less in wet forests than in dry
ones (measured); nothing pops into view.

**3. `gold`: Noita's gold.** Built as a material (user, 2026-09-29: "gold
can act as other materials ... a very large amount should look like"
Noita's glowing heaps): dust, one cell a coin, instead of nugget bodies; the
rest is the simulation's. SPEC §3.4g.
1. The count (on the player, saved) and the HUD's nugget and number.
2. Nuggets (1, 5, 25, 100): glinting, a faint warm light, bouncing and
   rolling, drifting to you, a clink; merging when they lie together far
   off; saved with their chunk, forever.
3. Out of the dying (loot tables give each creature's gold by kind and
   depth), in chests; acid can't touch it, water sinks it, blasts scatter
   it, lava takes it (for now), the vaporiser and void erase it.
4. `gold` scenario (a warband killed: the gold bursts, is collected).
Done when: gold flows from fights and chests and never disappears except
as designed.

**4. `matter`: metals, casting, objects, boulders, the kick (DESIGN §13.1).**
1. ✅ Objects: built simpler than planned (SPEC §3.11): at rest an object is
   cells (saved, stood on, free), a thing by its material (`object`); a
   kick or a push lifts its connected piece out as a body, and one that
   loses its hold falls whole. Bodies already hurt creatures (`crush`).
2. ✅ The kick (F): an impulse by mass to objects, items, gold, bodies,
   rubble, particles, barrels; a shove to small creatures.
3. ✅ Force and gravity magic move objects (the well lifts them, the force
   wand throws them).
4. ✅ (no slag yet) Metals: ore, molten and solid for each metal, and slag; the heat rules
   between them; molten metal glows and lights what it touches; a bar is 16
   cells of its metal (mining it gives bars back); firebrick (heat-proof).
5. ✅ (no spout: the ladle is filled at the furnace) Casting: the furnace melts what it's fed and pours from its spout; the
   ladle (and a bucket and a flask: carried liquids) pours at the cursor;
   a mould of what stands the heat; the casting freed as an object.
6. ✅ Gold in lava melts into molten gold and sets into gold (mined: gold).
7. ✅ (impact and fall; blows and blasts act on the cells as before) Boulders: rock bodies, round ones rolling; fracture (impact, fall, blow,
   blast past the material's strength: smaller bodies and rubble).
8. ✅ (a rope, not a prop) Traps: boulders held by a prop or a rope; tripwires and pressure plates
   (mechanisms); worldgen puts some in tunnels.
9. ✅ (copper, not iron; the barrel kicked in the arena) Scenarios: `cast` (a mould built, iron poured, mined out, the casting
   kicked), `boulder` (one down a slope into a wall: it breaks), `trap`,
   `kick` (a barrel kicked into a camp).
Done when: you can make an object of your own shape and kick it about;
boulders roll, crush and break; the bench holds with a hundred sleeping
objects about.

**5. `village`: a start worth starting in, and people.** SPEC §3.4i, §5.5.
1. ✅ The start: a wide flattened plain round the spawn (±300 cells, forest
   or plains) (worldgen, from the preset).
2. ✅ (the buildings as text in `village.buildings`; a stone well, no path)
   The village: a structure a short walk away (rooms as text: timber
   houses, a well, a smithy, a path), placed from the spawn.
3. ✅ NPCs as data: a file per kind (body and look on the humanoid rig, role,
   lines, services), the villager brain (a day's schedule, home at night,
   flee danger, talk when near), a friendly team monsters hunt.
4. ✅ Talking: a speech bubble over them; a panel for services.
5. ✅ (the smith sells what it would make, not crafts to order) First roles:
   a guide (tips), a smith (crafts at their anvil, for gold), a healer
   (heals, sells potions); a merchant (buys and sells for gold).
6. ✅ (and `shop`: a trade with each) `village` scenario (walk in by day
   and by night).
Done when: a new world starts on the plain with the village in sight, and
its people live their day.

**6. `world-events`: the clock's events.** Each on the pattern of the
wildfires: a timetable rolled from the seed and the day (so the abstract
and the live agree), a live face where someone is (through the simulation,
arriving from out of sight), an abstract face where no one is (applied to
the land as its chunks load), and a scenario.
1. ✅ The events' core (`events.rs`): the timetable, a log of what happened
   (saved with the world), news (villagers say what happened and where; a
   line on screen when it's near you), the dev panel's "An event".
2. ✅ (debris from the blast can hurt you far off: kept, it's real) Falling stars (at night): a streak across the sky, a flash and a boom; a
   crater with a glowing meteorite in it (a new material) and rare ore
   (mithril) at the surface; a guardian (a glowing creature, keeping it).
   Away: the crater's there when you come (dug as its chunks load).
3. ✅ (villagers run home and hide; killed, they come back as keepers) Raids: every few days a warband sets out for the village; there, it
   walks in from out of sight and you fight it (the villagers run);
   away, houses are damaged, and the village rebuilds toward what it was
   over days, faster for gold (the guide takes it).
4. ✅ Earthquakes (rare): the screen shakes; near you cave ceilings come
   loose and fall; a chasm opens in the surface (away: it's there when you
   come).
5. ✗ Dropped (user, 2026-10-01: biomes, not seasons; the tundra already
   snows, its lakes freeze and its trees are snowy). Was: Seasons (a short year): winter snows (snow falls instead of rain, the
   snow line comes down, lakes freeze over), spring blooms; the land
   follows the season as it loads and, in view, gradually.
6. ✅ (a "pedlar") A travelling merchant (no roads yet, so not a caravan): every few days
   one walks in to the village, stays a day with rarer goods, and leaves.
Done when: in a few days' play stars fall, raids come, the ground shakes
and the merchant visits, near you and away, all from one timetable.

**7. Later: travel.** Mine carts on rails, stronger travel and building,
teleport stations. A blood moon.

## The bestiary: implementation plan (BE, agreed 2026-10-01, DESIGN §14)

Creatures, encounters and their tools, before the world editor and, after
the rest of the sandbox (DESIGN §14 "Where this sits"), progression.
Branches in order; each merges to `main` when its "done when" holds (tests,
clippy, the bench, its scenarios and screenshots). The testing loop (the
bestiary and arena v2) comes third so everything after it is built and
judged in it. (Deep caves ran at ~42 ms a frame: fixed on main, 0b9a6cb,
before this arc began.)

**1. `creatures`: one model, every creature on it.**
1. ✅ `crates/game/src/creatures/` (the old `actors`, renamed: `def`,
   `nature`, `body/`, `brain/`, `moves/`, `custom/`; locomotion stays the
   physics crate's until it needs its own); the brain vocabulary:
   `hunter` (the general fighting brain: how it closes, how it attacks,
   what it does idle, a leash, a march) with the fighting brains we have as
   its presets; `critter` and `villager` (wild life and people: their own
   routines, shared by many creatures, so vocabulary, not custom).
2. ✅ Custom modules: the trait, the registry (an unknown name is reported
   at start), `_template.rs` (compiled with the tests); the dummy, the
   explosives and the egg sac's hatching moved onto it.
3. ✅ Ten damage types on every weapon, spell, blast, fall and hazard; one
   resistance profile for creatures and gear (replacing `Ward` and
   `Resist`); kinds as templates; absorbing; what each can't suffer;
   regeneration (the troll's, stopped by fire and acid). As built: gear
   keeps its `Ward` (slash, pierce, blunt) ahead of the creature's
   multiplier; gear on the profile comes with gear's own work.
4. ✅ Every existing creature moved onto it, with a kind: orcs, the archer,
   the troll, spiders, spiderlings, egg sacs, skeletons, slimes, acid
   slimes, bats, vampire bats, star wisps, villagers, the pedlar.
Done when: no hand-written brain is left outside the vocabulary
(`hunter`'s presets, `critter`, `villager`) and `custom/`;
the existing fight scenarios (`fight`, `melee`, `underground`, `warband`,
`archery`, `spider`, `raid`) hold their numbers; the troll can't be beaten
with steel alone and can with a torch.
**Done 2026-10-01**: the five fighting brains are `hunter`'s settings
(each creature's exact old numbers); `fight`, `melee`, `underground`,
`warband`, `spider`, `raid`, `star` match main within run-to-run noise
(`archery` fires no arrows on main either: a separate fix); the `troll`
scenario: held at 3 hp by the sword, dead 0.1 s after it's set alight.

**1½. `scale`: everything 1.5× in cells, seen at 2 px a cell (SC, agreed
2026-10-01).** The spike (`scale-spike`, spike/hd_spike.png) showed the
same scene on screen with 2.25× the pixels in every thing; the user: "love
it, lets do it". Measured: the cell sim +45 % in `chaos` with the same
debris (the particle cap must rise 2.25×), frame time unchanged. The rule:
lengths and distances ×1.5, speeds and accelerations ×1.5, areas and cell
counts ×2.25, per-cell decay (light falloff) to the 1/1.5 power; durations,
damage, hardness, chances and counts of decorative sparks stay (sparks get
finer: the point). `tools/scale/ron_scale.py` holds the rules for data.
1. ✅ Data: every creature file, weapons, runes (fireballs' blasts and fire),
   items, gear (rocket boots, hooks), tools, tempo, lighting, lairs, life,
   loot, progression.
2. ✅ Code: every constant in cells (camera, hands, combat, magic, fx, light,
   events, physics defaults, brain defaults); the zoom starts at 2.
3. ✅ The sim and worldgen: world presets 1.5× each way, terrain frequencies
   ÷1.5, caves, trees, villages, lairs, structures; the particle cap.
4. ✅ Art, redrawn by hand at 1.5× (not upscaled): the humanoid rig and its
   villagers, gear looks, the skeleton, the troll, spiders, critters,
   slimes, bats, wisps, props, weapons and held things.
5. ✅ Rocket boots fire from each boot (user, 2026-10-01): the leg parts mark
   their feet, the exhaust comes from both.
6. ✅ Scenarios and the bench re-measured; budgets kept or argued.
Done when: nothing on screen is drawn at the old scale, the scenarios hold
their stories (who wins, what breaks), the bench holds its budgets.
**Done 2026-10-01** (with sub-agents in parallel: art in five batches,
worldgen, scenarios, docs). Also: the backdrop's ranges generated 1.5× as
fine (not stretched); material patterns at 12 × 12; BLOCK is 6 cells (every
room, village and castle follows); the building grid, chests (18 × 15) and
stations redrawn. Found and fixed on the way: a liquid's pressure look ran
past the step's chunks (a pool deeper than a chunk panicked); lightning's
earthed stretch in two places (a 2-billion-wide texture); kicks scaled twice;
boulder traps held at their niche's corners; trees over cave and chasm
mouths. Scenarios now run with no window (`PLATYPUS_OFFSCREEN`, user: "not
in the foreground"). Bench: `chaos` 3.1 ms a tick (budget 4), streaming a
12-chunk column (the view at 2 px/cell) 1.7 / 3.3 ms. Known: a coal seam
in the `trap` scenario's tunnel pours onto its tripwire and buries it
(powder ores in cave walls fall at load: worldgen, separate); `archery` and
`beams` scenarios pick the wrong hotbar slots (separate task).

**2. `moves`: moves as data, and seeing that it hurts.**
1. ✅ The move vocabulary: wind-up, active, recovery; hit shapes; lunges;
   projectiles; summons; grabs; beams; area slams; every move with a tell.
   As built: `moves.ron` (phases with poses and acts: lunge, strike, cast,
   slam, summon, sound, grab, throw, beam; a phase's clip), chosen before
   the brain thinks (its weapon waits), carried out after; the troll's
   grab (`grab` scenario); `fire_ray` ready for a creature that burns.
2. ✅ Damage feedback (DESIGN §14.3): reactions scaled by the share of health
   taken; wound drips and staggers; hurt, resisted and absorbed hits told
   apart.
3. ✅ Observations recorded (what the player saw each kind of creature do),
   for the player's bestiary later.
4. ✅ Arena readouts: a timeline of hits, damage per second, damage taken, by
   type.
Done when: the existing creatures' attacks are moves in data, unchanged in
feel; a scenario hits a skeleton with each damage type and logs the three
reactions as designed.
**Done 2026-10-01**: the spider's bite, spit and sting are moves in
`moves.ron` (the `spider` scenario as before: the spit from 180 at
0.57 s, the sting's 30 at ~4.06 s, venom, the bite's 16); weapons stay
`weapons.ron`'s (`Swing`, `Shoot`), contact `touch`. `reactions`: slash,
blunt, fire, frost, storm, acid, radiant hurt; pierce and poison resisted;
void absorbed; drips under half, falters under a quarter; then the Fight
readout and the player's record of skeletons (met 1, felled by blunt).
Every hurt from every source is kept on the body by kind and tallied once
a tick (`Took`): observations and readouts both read it. `grab`: the
troll catches the player, squeezes twice, hurls it 235 cells; a staggering
blow breaks a grab; a skeleton's fire ray sets the player alight.

**3. `bestiary`: the browser, arena v2, and the command line.**
1. ✅ The bestiary panel: cards, filters, search; the expanded view with the
   live preview stage, stats, profile, moves, phases, buttons (place, fight,
   open, reload).
2. ✅ Arena v2: layouts (flat, cave, slopes, stairs, a copy of real terrain);
   live reload of creature and move files mid-fight; recorded fights
   replayed as tests.
3. ✅ `platypus-bestiary`: cards and preview strips as images, stats listed.
Done when: every creature is in the bestiary with a working preview; one is
placed, fought, edited mid-fight and the fight replayed as a test; the CLI
renders the whole bestiary.
**Done 2026-10-01**: the catalogue and panel (F12: search, part and kind
filters, cards, the opened card with Place, Fight, Open file, Reload); the
live stage (a sealed room in the arena, the real creature driven through
standing, walking, its weapon and each move, hit shapes drawn); creature
files reload into creatures already about (`reload` scenario); layouts
(flat, cave, slopes, stairs, real terrain; `layouts` scenario); fights
recorded and played back headless as a test (`record`, then
`PLATYPUS_REPLAY`: PASS twice, a doctored ending FAILs); creatures'
rolls from their own numbers, not entity ids; `platypus-bestiary`: all 29
creatures, cards, strips and numbers, in ~2 min. Not done: biome and tier
filters (the world doesn't say yet); the live stage outside the arena.

**4. `navigation`: the path planner, and digging.**
1. The shared path planner (walk, climb, jump, swim, fly, each a cost).
2. Digging: claws, acid, tunnel, blast; its cost by hardness; tells
   (scratching, dust, cracks, hiss); caps per creature and frame.
Done when: a creature finds its way round an obstacle course it used to
stick on; the cave spider digs and spits its way to a player walled in
dirt, slows at stone, stops at obsidian and glass; the bench holds with
diggers at work.
Done (branch `navigation`): `platypus_nav` (grid, moves, jumps from the
physics, A* sliced across ticks, fields), the follower, claws and acid;
the course: orc, skeleton, spider, bat, wisp get there, slime and troll
can't (their jumps are short of it), none of the walkers without it; the
spider through dirt in 29 s, stone in 60, not obsidian or glass; `chaos`
7.6 ms a frame, worst 24 (7.2 and 25 without), `nav_dig` a tick's
planning ≤ 2.3 ms. Not done: tunnel and blast digging, cracks as a tell,
fields used by packs; a digger's search into deep stone rarely finishes
inside its budget (it follows the part it found, and looks again).

**5. `limbs`: general procedural limbs.** In stages:
1. Side-view legs whose planted feet carry the body (height and tilt
   follow the ground), arms, a gait clock, footfalls; spiders onto the
   general system (their look kept). *Done 2026-10-05*: legs from above
   and the side (2, 4, 8; two or three bones; flesh or metal plates),
   arms with claws, footfalls (sound, dust, shake); test creatures: stilt
   stalker, crag crab, ridge raptor, ridge tyrant (T-rex scale), iron
   strider (a mech).
2. Chains: tails, necks, a wraith's chains; hanging and swinging with
   the body, held out (a tail's stiffness), aiming at a target (a
   scorpion's sting over its back). Tails for the raptor and the tyrant.
   *Done 2026-10-05*: chains (rest pose, stiffness, swing, sag, rings, a
   tip sprite, aiming by reshaping the arch); the raptor's and tyrant's
   tails; the cave scorpion, its sting aimed over its back.
3. Segments: a centipede, a wyrm, following the head's path; side-view
   bodies that climb walls and ceilings (the body and its legs turned to
   the surface).
   *Done 2026-10-07*: side-view bodies in the frame of the surface they're
   on (walls, ceilings, round inside and outside corners; climbers leap
   off walls); the spider seen from the side (from above on the wall
   behind); feet with weight (a damped spring with a top speed); segments
   trailing the head's way with legs stepping in a wave; the cave
   centipede. Not yet: hits on segments (stage 4), the wyrm (burrowing,
   with the sand wyrm in the roster), creatures spawned in the world.
4. Moves that drive limbs, hit areas per part, and the strider's arsenal:
   - aimed limbs (turrets): an arm that tracks the target at a turn rate,
     a barrel sprite, a muzzle shots leave from; moves name the limb that
     acts (left turret, right, in turn); a stomp lifts a leg and slams it,
     a claw snips, a jaw bites;
   - tells as data, switched on by a move's phase: a sight line from the
     muzzle to the target while it aims, a charge glow at the muzzle
     before it fires, a lock-on mark over the target before missiles;
   - heat in parts: each shot heats its barrel, which cools over time,
     drawn dull red to orange to white with a glow; past a limit it
     overheats and the move system forces a vent (steam, a hiss, a pause:
     the player's opening) (the first per-part state: `wounds` builds per-
     part health on it);
   - missiles: spell runes `homing` (turns toward its target at a rate)
     and a smoke trail; a phase can repeat an act (a salvo);
   - hit areas per part, weak points (more damage there).
   *Done 2026-10-07*: moves drive named limbs (a pose's `pitch` and
   `limbs`: legs raised, arms reached out, claws opened; strikes from a
   limb's end): the crab snips, the raptor bites, the tyrant stomps; hit
   areas per part and weak spots (`parts.rs`; the strider's viewport ×2.5);
   turrets that swing to their target, fire along their barrels and heat
   (glowing, then venting: moves wait); tells as data (sight lines, charge
   glows, a lock-on mark); salvos and homing missiles with smoke trails.
   Not yet: a segment's little legs as parts, spells' damage scaled by
   part, beams and blasts against parts.
Done when: a four-legged walker crosses the arena's real-terrain layout
with its body following the ground; a chain aims; a segmented crawler
climbs a wall and a ceiling; the iron strider tracks the player with two
laser turrets, shows a sight line and a charge glow before each shot,
its barrels glow after a burst and vent when they overheat, and it fires
a lock-on salvo of missiles that curve onto the player; the crab snips,
the raptor bites, the tyrant stomps.

**6. `behaviour`: creatures that live in the world.** (Added 2026-10-07,
user: everything moves and fights well but none of it lives in the world
yet.) In stages:
1. Where they live: spawn tables by biome, depth and zone for every
   creature (the new ones too), population caps, despawning far off; dens
   and lairs (spider nests, centipede holes, a raptor pack's ground).
2. Senses and alertness: sight (line of sight, light: the dark hides you,
   a torch gives you away), hearing (footsteps by what you walk on, a
   fight's noise, blasts), smell (blood); idle, suspicious, hunting, lost
   you (searching where it last saw you).
3. Tactics as data: keep a distance, flank, ambush (wait on a ceiling,
   drop), fall back when hurt, flee fire or light, call others; groups
   (packs that surround you, swarms) and their leaders.
4. Factions: who hunts whom (spiders take bats, orcs fight skeletons);
   creatures fighting each other; setting them on each other.
Done when: a walk through the world meets the new creatures where they
belong; a centipede waits on a cave ceiling and drops on you; raptors
flank you as a pack; a badly hurt creature flees; two factions fight with
you watching; creeping past in the dark works and a torch gives you away.

**7. `creature-editor`: from a sketch of the whole to a creature.**
Sketch (layers), slice, rig, pose and moves, test; editing a part in place
on the whole; big parts as palette PNGs, the rest text; every step also
from the command line.
Done when: a new creature is sketched, sliced, rigged, given two moves and
fought, without leaving the game; the model makes one the same way from
scripts and renders it.

**8. `wounds`: wounds you can see, limbs that break.** (Widened 2026-10-11
to the full body system: stages in order, each shown before the next.)
1. Ragdolls: a body is its parts (a humanoid's head, torso, arms, legs, as
   drawn and dressed; a walker's body and legs, thigh and shin; a tail's
   segments), pinned at their joints, each colliding with the cells: they
   flop, drape over ledges, lie along slopes, are thrown by blasts.
2. Severing: parts with their own health; a heavy blow on one cuts it off
   (the piece flies and lies as its own ragdoll, the stump and the piece
   bleed real blood: ash, acid, oil and sparks for the mech); what it costs
   the living (an arm: its weapon; a leg: a limp, then a crawl; legs off a
   wall-walker: it falls; the head: dead, but a skeleton fights on blind
   until its skull is crushed).
3. Marks: depth by kind (skin, flesh, bone; ichor; stone; ectoplasm), each
   damage type's mark on the body (cuts, holes, cracks, char, melt), blood
   from the wound along the hit; bodies keep their wounds.
4. Gibs: a blast breaks a body into pixel chunks that fly, bounce and pile.
   (Stages 1–4 built 2026-10-11 on `wounds`; open: marks on walkers, gibs
   that turn into cells, a skull to crush that's a puzzle in a fight.)
Done when: shooting a spider's legs off on a ceiling drops it, bleeding
acid; a zombie (until zombies come: an orc) loses an arm and a leg and
crawls on; bodies flop and drape as ragdolls; each damage type
leaves its own mark; a fight with a dozen wounded creatures stays in
budget.

**8b. `zombies`: the wounds showcase.** (Pulled forward from the roster,
2026-10-11.) Stages, each shown before the next:
1. The body: the humanoid rig on stepping legs (`legs` rig), dead-pale and
   bloodied (RE2's dead, not green), a man's height, arms out ahead, slow;
   dead flesh (`rotting`: fire and light undo it); dark `rot` blood; limbs
   come off (legless it drags itself on, armless it bites from its mouth:
   `armless`); its head off, dead for good (`headless: Dies`).
2. Dangerous (agreed 2026-10-11): 130 health, heft 4, poise 40; zones (head
   ×2, body ×0.6, a head blow staggers sooner); each its own pace; the
   lurch (a few fast hopping steps at you); the grab (arms up with a snarl,
   lunge, three bites; dash through it; held, mash to break free); a groan
   now and then (`voice`).
3. Burn the body or it comes back worse: it gets up twice (`times: 2`), the
   second time as a crimson head (`last`: red, runs, grabs harder); only
   fire destroys a body (a blow of fire, flames, a torch held to it): it
   burns 4 s and lies charred.
4. Up out of the ground: life.ron `emerges` (nearer, so it's seen: a hand,
   the head, heaved up in jerks, the earth puffing); groups one after
   another; the overworld at night, underground any time; the sun burns
   them and their bodies (`sunburns`); they see in the dark, hear well,
   smell the wounded.
   (Stages 1–4 built 2026-10-11 on `zombies`; scenarios `zombie` (rise,
   limbs, break, grab, shake, burn, torch, emerge, sun) and `night`.)
5. Next: noise draws hordes; more near crypts; the dead (villagers, orcs)
   killed at night rise as zombies of their kind (infection, and a bite's
   lasting harm); a cut-off arm crawls on; chop a body's legs off before it
   gets up; the bloated one bursts into gas; pile-ups; a struggle prompt.
Done when: a night on the surface brings them up out of the ground; a
fight leaves arms crawling, torsos dragging themselves, and bodies you
must burn.

**8c. `slimes`: real goo.** (Agreed 2026-10-11.) A slime is a core (a
nucleus with eyes, its box) holding a body of goo, cells of the world
(`body/goo.rs`; the sim's `HELD` cell flag: held goo stays put, nothing
pushes it, bodies pass through, let go a few ticks after it's no longer
put back). Its goo is its health: hurt, it flings goo off; dead, it spills.
1. The body: laid out each tick round the core (a dome on the ground, a
   ball in the air, squashed landing); fire burns it, acid eats it; sticky
   (slimy: slows); heals when let be, drawing its spilled goo back in;
   high hops, each its own height; it comes down on you, touching with its
   goo. Spikes: near you, its face shivers, then a fan of goo spikes stabs
   at you, pulsing in a wave, jittering.
2. Split and merge: a cutting blow big enough cuts off a slime of its own
   (smaller for good: Mass); slimes whose goo touches merge (up to 3x).
3. Swallowed: items (ore, gems, gear kept; the rest dissolved), small
   creatures (digested: it grows), and the player when it lands on them
   (digested; struggle out, or spat out in 3 s; blows from inside x2).
4. Kinds: green (burns), blue (bounces, won't burn), acid (eats what it's
   against), the glutton (takes on fire, frost, lightning; fire on frost
   goes off; drains; charged, it attacks with it: lightning, ice shards,
   a lava eruption, a third of its charge each; `attack` scenario mode,
   PLATYPUS_GLUT_HARM). Since 2026-10-12: drinks magic (immune; spells
   landing on it are swallowed), seven elements (+ acid globs, miasma
   belch, light ray, void blink), light+void implode, fire burns poison
   off, water douses/dilutes/conducts/freezes, oil flares up; `wand`
   mode (PLATYPUS_GLUT_SPELL list), PLATYPUS_GLUT_POOL=water|oil.
   (Stages 1-4 built 2026-10-11 on `slimes`; `slime` scenario: look, burn,
   kill, heal, split, merge, engulf, eat, glut, many; PLATYPUS_SLIME_KIND.)
5. Next: squeezing through cracks (the core shrinks to a gap), the slime
   king (one that's eaten enough), goo as a material to collect and use
   (torches from green goo and wood), bones out of what it digests.

**9. `powder`: firearms and new materials.**
The flintlock pistol and the musket; gunpowder (charcoal and sulfur, a new
mineral by the underworld's lava); iron and silver shot; muzzle flash,
smoke, recoil, reload. Blight and ectoplasm.
Done when: a powder trail burns like a fuse and a barrel blows; a musket
ball leaves a wound and a spray; silver shot hurts the undead more; blight
withers grass and poisons; ectoplasm slows.

**10. `roster`: the new creatures.** (Brought forward 2026-10-12, with
`bosses`, ahead of `creature-editor` and `powder`; built one at a time,
the most fun first, each tried before the next: mimic, bloat toad,
necromancer and risen skeletons, watcher, chain wraith, shield orc, orc
sapper, bat swarm. (2026-10-15: paused for the prologue with mimic, chain
wraith, dread knight, Jan Willy, Olmer the Defiler, the Fallen Templar
(IK arms holding a posed halberd), bloat toad, necromancer (reworked:
floating, cloth, skulls, grave hands, drain, zombie packs) and watcher
(blight) built; shield orc, orc sapper and bat swarm left.) Mimic built
2026-10-12 on `roster`: chests deep down
and in castles one in six; `Disguised`; `mimic` scenario. Chain wraith
built 2026-10-12 (asked for next): `Phasing`, simulated chains on a canvas,
spin/lash/pull/rake and whirl/sweep, water stops it, fades at dawn, night
surface and deep caves; `wraith` scenario. Dread knight (asked for,
2026-10-12): a Nazgul-like giant, cloth cloak and rags (cloth.rs, with
the player's Back slot and Traveler's cape), chains, a simulated spiked
flail: whirl and slam (crater), draw and sweep; drops a wearable Dread
cloak; `dread` scenario.)
Necromancer, risen skeletons, zombies, chain wraith, watcher, stilt
stalker, crag crab, scorpion, cave centipede, the reworked cave spider,
bloat toad, splitting slime, shield orc, orc sapper, mimic, bat swarm
(DESIGN §14.11): each a file (and a module only where needed), each in the
bestiary, each with a scenario and a recorded fight.
Done when: all of them are fought in arena v2 and in the world, and each
tests what it was chosen to test.

**11. `bosses`: one branch each.** The Broodmother, the Necromancer lord,
the Ruin colossus, the Sand wyrm, each to the puzzle pattern (DESIGN
§14.8): a preparable weakness, an arena trick, an emergent route, a hint in
the world; its scenario plays the intended solution once.

**Spike: concept images to pixel art** (DESIGN §14.6), once an image API is
available: the converter, a dozen generated creatures and props through
it, shown side by side; go or no-go before it goes into the editor.

Done when (the arc): every creature, old and new, is a data file and, where
needed, one custom module; the bestiary shows them all with live previews;
each can be placed, fought and recorded in the arena; each boss's intended
solution plays out in its scenario; the bench and a crowded fight stay in
budget.

## Measured (Apple M2 Max, 1080p window, 3 px per cell)

| | Legacy (per-tile sprites) | Now |
|---|---|---|
| Entities on screen | ~274 000 | ~650 |
| Walking / panning, frame avg | 40 ms | 2.2 ms (uncapped) / 8.3 ms (120 Hz vsync) |
| Walking / panning, worst second | 80 ms | ~10 ms |
| Sim tick, settled world | — | 0.7 µs |
| Sim tick, 80k cells avalanching | — | 0.7 ms |

## World track (chosen 2026-09-25: world before combat, walk-through trees)

1. ~~Temperature, phase changes, explosive gas~~ (done)
2. ~~Particles~~ (done; rain drops come with weather)
3. ~~Background layer, plants, forests, tall grass, wind, sway~~ (done)
4. ~~Rigid bodies: tree felling~~ (done, own solver, SPEC §3.11). Next steps
   for it: playfield pieces as bodies (a cut-loose slab of rock), bodies
   colliding with creatures and each other, saving bodies in flight, per-tree
   crown sway
5. ~~Sky: clouds, rain/snow by temperature, storms, lightning~~ (done, SPEC
   §3.13). Still to do: saving the weather, parallax far clouds, lightning
   through water and metal
6. Meteors: fireball, huge crater, ejecta, forest fires, hot meteorite ore
7. **Worldgen v2 / biomes**: now the world arc W7 on branch `world-arc`
   (DESIGN.md §3, §11). Stage 1 done: `WorldPlan`, presets, bands,
   `platypus-worldview`, determinism test. Stage 2 done: biomes, climate across
   the world (and an inversion over the peaks), mountains with snow, oceans,
   lakes, sky islands (SPEC §3.4b). Stage 3 done: the underground by band,
   chasms, flooded caverns, the underworld's lava sea. Stage 4 done: hands
   (blocks, smart cursor, items, inventory, chests, loot; SPEC §3.4c). Stage 5
   done: ores and gems by depth with the pickaxe ladder (SPEC §3.4d). Stage 6
   done: crypts and castles (rooms as text, assembly, ruins, shafts,
   guards, keeps and towers, stairs), age (moss, cobwebs, fallen ceilings),
   secrets (illusory and weak walls, lake chests); SPEC §3.4e. Next: stage
   7, saving.
   Originally:
   biome plan from temperature × moisture; tree species as RON data (conifers
   that hold snow, birch, jungle, dead trees); lakes and rivers in forests;
   grass varieties (short, tall, flowering, dry, snowy); port the legacy
   mountains / sky islands / walker caves into a `WorldPlan`.

## Lighting: next steps

- Darkness as gameplay: creatures unseen outside light, glowing eyes,
  light-seeking and light-fleeing enemies (with combat).
- Light as a resource: torches that burn down, a flashlight battery, lanterns
  that go out in water, throwable glowsticks, placeable torches.
- Sound before sight (needs audio).
- Far parallax clouds. (Stars, moon, sun, mountains and the underground's
  far rock: done, B1.) Maybe later: the underground's rock by depth band (a
  basalt glow near the underworld).

## Elemental ideas to explore later

Same shape as everything so far: a general rule plus material data, no
special cases.

- **Electricity**: lightning (done) chaining through water and metal, hurting
  anything standing in them; wet creatures take more.
- **Steam blasts**: water meeting lava bursts outward (a small explosion on the
  reaction) instead of quietly becoming steam; trapped steam builds pressure.
- **Poison / swamp gas**: harms without corroding (a `toxic` material value),
  heavier than air so it pools in hollows, explodes like methane when lit.
- **Violent mixtures**: reactions with an explosion (acid + water heating,
  lava + ice), data-driven like `explodes`.
- **Freezing solid**: a chilled, wet creature caught in freezing water is
  frozen in place; the ice around it is real ice you can break.
- **Mud, tar, honey**: sticky liquids that slow what wades through them
  (`MovementStats::slowed` is already the hook).
- **Water cycle that conserves water**: steam now mostly fades instead of
  condensing (an old fix for a steam/water loop that kept regions awake).
  Clouds should hold that moisture and rain it back (weather arc).
- **Status interplay**: oil-soaked burns hotter and longer, wet conducts
  lightning, burning thaws chilled.

## Next, in order (older list)

1. **Particles** (phase 2): SoA particle system; dug material, debris and blood fly
   as particles and become cells when they land. Swap the death blood blob for this.
2. **Bodies in the cell world**: creatures displace liquids and sand when they move
   through them, so they splash and push.
3. **Combat** (phase 4): weapon RON (pivot, swing curve, frames, damage, knockback,
   hit-stop), masks from sprite alpha, swept blade test, `Hit` message, orc attack
   using the existing `attack` clip. Pogo on down-strikes (done: DESIGN §7's down-strike).
4. **Worldgen v2** (phase 3): port legacy generators into a `WorldPlan`.
5. **Co-op transport**: pick the netcode crate, host/join, input + edit messages,
   per-chunk checksums every N ticks, resync on mismatch.

## Stress test (2026-09-26, `chaos` scenario, arena, uncapped, M2 Max)

Waves of 8×n (then 24×n) mixed enemies every 3 s, spells, bombs, blobs of
sand/water/lava, lightning; `--features spikes` + `PLATYPUS_PROFILE`.

| Fix | Before → after (frame avg, 40 s run) |
|---|---|
| Crash: a command on an entity despawned the same tick (a burning arrow) → `try_insert`, and the app's error handler warns instead of panicking | crashed at 5 s → runs |
| Creature hot-reload polled its watcher through `ResMut`, marking `Creatures` changed every frame: every dressed creature redrawn (atlas compiled + uploaded) and restatted every frame | 12.0 → 7.3 ms (8×n) |
| Chunk background re-uploaded whenever the playfield changed → own dirty flag | image uploads 10k/s → 5k/s |
| Bodies at rest skip physics; at most 150 bodies (the oldest go) | 9.3 → 7.6 ms (24×n; 1600 → 150 bodies) |
| Spider legs were a mesh of per-cell quads (190k vertices at ~40 spiders); particles likewise → pixel canvases over the view (`canvas.rs`) | mesh upload 2.4 → 0.4 ms; 13.1 → 6.8 ms at 380 creatures |

The sim at many active chunks (headless `platypus_bench chaos`: the arena,
blobs of sand/water/lava/blood, blasts, three deaths' blood bursts every six
ticks; ~100 chunks awake, 28k particles), sampled with macOS `sample`:

| Fix | Tick (cells / particles) |
|---|---|
| before | 2.81 ms (2.41 / 0.41; particles were 2.3 ms in-game, serial) |
| Particles fly in parallel (read-only `ParticleView`), their effects (land, douse, light the background) applied after in order | particles 2.3 → 0.4 ms |
| The dam-break pressure scan (`through_to_open`: every liquid cell under liquid looked up to 48 cells each way, every tick; a third of all sim time) keeps its run along the row for the row's next cells, extending it as needed | cells 2.41 → 1.42 ms |
| Jobs biggest-first in a pass; the sim's pool at ⅔ of the cores (efficiency cores slowed passes) | 1.42 → 1.29 ms (8 threads) |

Tried and dropped: keeping the workers spinning between the four passes
(waking them costs ~150 µs a pass); it collapsed to 20 ms on 12 threads
(a descheduled worker stalls every spinner). Left: per pass, the slowest
chunk (a chunk full of stirred liquid, ~250 µs) plus the wake-up.

At 380 creatures, 150 bodies, 11k particles, 100 active chunks: ~6.8 ms a
frame, sim ~3 ms a tick. In a generated world, moving (streaming) at 12×n:
~5 ms a frame (230 chunks loaded), sim 2–4 ms a tick. Note: measured with
the screen locked; a visible window is paced at 60 Hz by macOS (frame avg
pins at 16.6 ms), so compare uncapped runs only. Spikes seen: new outfit
combos compiled on a wave's spawn (2–4 ms), several bombs in one tick
(4–5 ms), two sim ticks in one frame. Next suspects: the sim at 100+ active chunks, the
render schedule (~3.5 ms), frame spikes to ~27 ms (not yet traced),
damage-number text entities in a big fight.

## Frame pacing (measured 2026-09-25, `--features spikes`, 120 Hz vsync)

- Storms and fires used to push sim ticks past 4 ms often (83 in 20 s of a
  lightning-struck forest): the weather field stepping all at once every
  fourth tick (1.2 ms), every raindrop checking 3 cells for fire through
  quiet air, and every burning leaf asking for a tree-wide support check
  twice. Fixed (lanes, a sleeping-chunk skip, wood-only checks, 8 background
  checks a tick): 1 in 20 s; average tick 2.1 → 1.6–1.8 ms.
- The cloud texture was reallocated on most redraws while moving (its width
  followed the view's rounding): now sized in 128-cell steps.
- What's left is about one missed vsync every 2–3 s, all of it waiting on the
  swapchain with our work at 2–3 ms. An empty flat world with lighting off
  does the same, and `desired_maximum_frame_latency` 1–3 doesn't change it:
  macOS windowed presentation, not ours. Worth trying fullscreen, and moving
  the sim tick off the main thread (like the light solve) for headroom.

## Known issues

- Worldgen, rare (seen in the 8× world's many more places, 2026-09-29):
  small specks of rock float in the caverns (about one per three places
  looked at); a fungus shelf grown from a wall can hold a felled giant
  mushroom up; a crypt's ruin can stand in a lake's shallows.

- Generated trees whose branches interlock with a neighbour's hold each other
  up: cut one and it stays standing. Physically right; the forest plan could
  keep wood apart (found by the felling test in the larger world).

- Liquids level to within 1–2 cells across long flat stretches (terracing); fine visually.
- Gas collects under the top of the loaded area (unloaded = wall).
- `find_ground` retries forever for a spawn over a bottomless column.
- Creatures outside loaded chunks freeze (by design) — they resume when loaded.
