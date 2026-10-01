# Plan

Each phase ends with something playable or measurable. See SPEC.md for the design.

**Next:** the bestiary arc (BE, DESIGN §14, agreed 2026-10-01): `creatures`
and the 1.5× rescale (`scale`) done 2026-10-01, next branch `moves`; then the world editor and sites (WE); then progression.

## Status (2026-09-25)

| Phase | What | State |
|---|---|---|
| 0 | Workspace, pinned toolchain, SPEC, fixed timestep, seeds, bench harness | done |
| 1 | Cell world: chunks, dirty rects, materials as data, powder/liquid/gas/fire, reactions, parallel checkerboard, chunk textures, streaming, store | done |
| 2 | Bodies + shared movement (HK controller), creatures as data, brains, player + orc | core done; particles and body↔cell interaction open |
| — | Dev tools: pickaxe (hardness-based), bombs, spawner, igniter, eraser, heat/freeze, free camera | done |
| W1 | Temperature: heat, conduction, climate, melt/boil/freeze/ignite, glow, methane chain explosions, loose fragments | done |
| W1b | Fire burns in place; fragments fall after any destruction (fire, melt, acid) | done |
| W2 | Particles: blast debris + sparks, mining dust, embers, blood splashes | done |
| W3 | Background layer, plants, forests + tall grass, wind, foliage sway + creature rustle | done |
| W4 | Rigid bodies: felled trees topple, shed their crown, land as logs, hurt what they hit | done |
| W5 | Elements on creatures (heat, cold, corrosion, burning, wet), acid fumes, oil fires, weather: clouds, rain, snow | done |
| W6 | Lighting (glow/opacity data, light grid, lantern, flashlight, rim, haze) and day/night | done |
| 3 | Worldgen v2: world plan (legacy mountains, sky islands, walker caves), biomes, save/load | done (world-arc, then L1 `world-scale`) |
| C1 | Magic casting core: runes + wands as data, bolts/orbs/streams/wand lightning through the cell sim, mana, explosion damage for every blast (branch `magic-arc`) | done |
| A1–A3 | Art pipeline: sprites as text + `platypus-art` CLI (sheet/render/check/describe/get/set/paint/import), critters, the humanoid rig and player, casting arm; the arena (dummies, pause/step/slow motion, overlays, spawning) and the in-game art editor (branch `combat-arc`) | done |
| 4 | Combat: weapon swings, pixel masks, swept hits, hit-stop, knockback, stamina, dodge (SPEC 6.2); enemy attacks come with the orcs' redesign (arc A stage 6); the down-strike (S + attack in the air: a slash or a plunge per weapon, bouncing off creatures, hostile spells and hazards, a plunge's dive and slam; DESIGN §7, on `progression-arc`) | core done |
| G1 | Gear: stats from what's worn and held, armour and resistances on every hurt, equipment slots and panel, cloth/leather/iron sets and trinkets; gear drawn on every humanoid (skins: recolours + overlays) and icons from it; rarity (common → legendary) and rolled, named bonuses by item level, found in chests; bodies to loot (what the dead wore and carried + their loot table), chests and bodies both containers; wands and staffs hold their spells (a wand one, a staff two; found ones roll theirs) and glow with element auras; enemies wear and wield rolled gear and drop it; uniques (Troll's Club, Broodmother's Fang with venom on hit); trinket icons; rocket boots (fire exhaust) and cloud boots; spiders bite (lunge, knockback), spit acid globs, sting (rear up, stinger over the back, venom), crawl over any wall behind them (cave walls, built walls, trunks; legs gripping it), fewer spiders and bats; the grappling hook (Hook slot, E: Terraria's reel on a real rope: S pays out, W climbs, A/D pump a swing, wall kick, wraps round corners, slips round a stuck corner, mantles onto a ledge, jump off it; pulls small things to you; tears loose when its cell goes; three hooks in loot) (branch `gear-arc`, the rope on `progression-arc`, DESIGN §7c) ; magic batch 1: the beam carrier (fire ray, frost ray: ice bridges, lava to basalt), radiant (spark bolt, star bomb, vaporiser), storm's shock bolt; wall + cloud carriers (ice wall, fire wall, toxic cloud of heavy miasma), gases drawn as haze; call carrier (call lightning, meteor; open sky only); void (blink, portal pairs for bodies/spells/liquids, stasis) | done (first magic batch complete) |
| 5 | Terraria layer: items, inventory, mining yields, building, crafting, lighting | items, inventory, mining, building, lighting done (torches and glow sticks brighter, with a haze over the dark, 2026-09-27); P1 (branch `progression-arc`, DESIGN §3.5, §7d): saving (changed chunks, player, chests, stations, creatures; by name; autosave + Ctrl+S + on quit), crafting (stations as furniture, recipes as data, the crafting panel), progression (progress record, milestones with rewards and unlocks, recipe discovery, toasts): first versions with example content, done |
| S1 | Sound (branch `sound-arc`, DESIGN §7e): bevy_seedling buses, ~37 effects and 6 ambience beds and 3 music moods all made from recipes in `sounds.ron`, world-driven ambience (fire, lava, water, rain, wind, caves, drips), positional effects hooked into combat, movement, mining, the hook, the bow, blasts, spells; the arena panel's sound board | first version, to tune by ear |
| B1 | Backdrops (branch `backdrop-arc`, DESIGN §4.3b): Noita-like ranges by biome (`peaks.rs`: noita, alpine; running down behind the ground; 1–4 % parallax across, ~1 % up and down, pixel-snapped; behind the weather's clouds), drifting cloud heaps, a sky gradient, a blooming sun, a big moon in phases (8-night cycle) and twinkling stars over near-black nights; underground a tinted void with faint far twinkles | first version; next: big caverns without back walls in places |
| L1 | The living world (DESIGN §13, agreed 2026-09-29): dev world reset (world only / everything) → an 8× world sized from its preset (4× wide, 2× deep; density tests) → fire moisture and storm rain → the world clock (regions, processes at their own pace with abstract and live faces; weather's fronts, regrowth, moisture, wildfires, lairs) → a flat start and a village → NPCs as data → gold (Noita's) → raids, caravans, falling stars, seasons. Proposed: metals (ore → molten → solid, §13.1), boulders (§13.2). Later: mine carts, teleport stations | done 2026-10-01: world-scale, world-clock (moisture, regrowth, wildfires, lairs), gold, matter, village, world-events (stars, raids and mending, quakes, the pedlar); seasons dropped for biomes; travel moved later (after the world editor) |
| BE | The bestiary (DESIGN §14, agreed 2026-10-01): creatures as data with custom modules, ten damage types and resistance profiles, moves, damage feedback, the bestiary and arena v2, a path planner and digging, general limbs, the creature editor, wounds and severing, firearms and new materials, the roster, the bosses | `creatures` done; next: `moves` |
| WE | The world editor and sites (DESIGN §14 "Where this sits"): authored places placed by the world plan, edit and play on the same world, then mechanisms, a map with rest points and travel, factions, a vertical slice; then progression | after BE |
| 6 | Hollow Knight layer: ability unlocks, map, bosses, benches, set pieces | not started; the tempo (DESIGN D7) being tried: `tempo.ron` presets, `turn_accel` and `jump_hold` in movement, the arena panel's Tempo row, the `tempo` scenario; walking real terrain fixed (a 6-cell step up and walking down onto ground below, both eased on screen: full speed 12 % → 70–90 % of a walk; `walk` scenario) |
| 7 | Pixel rigid bodies (falling terrain chunks) | optional |
| — | Co-op networking (host-authoritative, checksum + chunk resync) | seams in place, not built |

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
1. The bestiary panel: cards, filters, search; the expanded view with the
   live preview stage, stats, profile, moves, phases, buttons (place, fight,
   open, reload).
2. Arena v2: layouts (flat, cave, slopes, stairs, a copy of real terrain);
   live reload of creature and move files mid-fight; recorded fights
   replayed as tests.
3. `platypus-bestiary`: cards and preview strips as images, stats listed.
Done when: every creature is in the bestiary with a working preview; one is
placed, fought, edited mid-fight and the fight replayed as a test; the CLI
renders the whole bestiary.

**4. `navigation`: the path planner, and digging.**
1. The shared path planner (walk, climb, jump, swim, fly, each a cost).
2. Digging: claws, acid, tunnel, blast; its cost by hardness; tells
   (scratching, dust, cracks, hiss); caps per creature and frame.
Done when: a creature finds its way round an obstacle course it used to
stick on; the cave spider digs and spits its way to a player walled in
dirt, slows at stone, stops at obsidian and glass; the bench holds with
diggers at work.

**5. `limbs`: general procedural limbs.**
1. Side-view legs (2, 4, 6) whose planted feet carry the body (height and
   tilt follow the ground); chains; segments; moves that drive limbs.
2. Hit areas per part.
3. Spiders onto the general system (their look kept).
Done when: a four-legged walker crosses the arena's real-terrain layout
with its body following the ground; a chain aims; a segmented crawler
climbs a wall and a ceiling.

**6. `creature-editor`: from a sketch of the whole to a creature.**
Sketch (layers), slice, rig, pose and moves, test; editing a part in place
on the whole; big parts as palette PNGs, the rest text; every step also
from the command line.
Done when: a new creature is sketched, sliced, rigged, given two moves and
fought, without leaving the game; the model makes one the same way from
scripts and renders it.

**7. `wounds`: wounds you can see, limbs that break.**
1. Depth by kind (skin, flesh, bone; ichor; stone; ectoplasm); each damage
   type's mark; blood from the wound along the hit.
2. Parts and limbs with their own health; severing (the piece falls, the
   stump bleeds); the consequences (gait, climbing, lost attacks); bodies
   keep their wounds.
Done when: shooting a spider's legs off on a ceiling drops it, bleeding
acid; a zombie loses an arm and a leg and crawls on; each damage type
leaves its own mark; a fight with a dozen wounded creatures stays in
budget.

**8. `powder`: firearms and new materials.**
The flintlock pistol and the musket; gunpowder (charcoal and sulfur, a new
mineral by the underworld's lava); iron and silver shot; muzzle flash,
smoke, recoil, reload. Blight and ectoplasm.
Done when: a powder trail burns like a fuse and a barrel blows; a musket
ball leaves a wound and a spray; silver shot hurts the undead more; blight
withers grass and poisons; ectoplasm slows.

**9. `roster`: the new creatures.**
Necromancer, risen skeletons, zombies, chain wraith, watcher, stilt
stalker, crag crab, scorpion, cave centipede, the reworked cave spider,
bloat toad, splitting slime, shield orc, orc sapper, mimic, bat swarm
(DESIGN §14.11): each a file (and a module only where needed), each in the
bestiary, each with a scenario and a recorded fight.
Done when: all of them are fought in arena v2 and in the world, and each
tests what it was chosen to test.

**10. `bosses`: one branch each.** The Broodmother, the Necromancer lord,
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
