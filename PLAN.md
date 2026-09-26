# Plan

Each phase ends with something playable or measurable. See SPEC.md for the design.

**Next:** DESIGN.md (proposed 2026-09-25): the world arc (plan, relief, underground, mining and
building in 4 × 4 blocks, items and chests, ores, structures, saving), the asset tools and arena,
and the RPG arc (rigs and gear skins, moves, corpses and loot, characters, spells).

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
| 3 | Worldgen v2: world plan (legacy mountains, sky islands, walker caves), biomes, save/load | not started |
| C1 | Magic casting core: runes + wands as data, bolts/orbs/streams/wand lightning through the cell sim, mana, explosion damage for every blast (branch `magic-arc`) | done |
| A1–A3 | Art pipeline: sprites as text + `platypus-art` CLI (sheet/render/check/describe/get/set/paint/import), critters, the humanoid rig and player, casting arm; the arena (dummies, pause/step/slow motion, overlays, spawning) and the in-game art editor (branch `combat-arc`) | done |
| 4 | Combat: weapon swings, pixel masks, swept hits, hit-stop, knockback, stamina, dodge (SPEC 6.2); enemy attacks come with the orcs' redesign (arc A stage 6) | core done |
| G1 | Gear: stats from what's worn and held, armour and resistances on every hurt, equipment slots and panel, cloth/leather/iron sets and trinkets; gear drawn on every humanoid (skins: recolours + overlays) and icons from it; rarity (common → legendary) and rolled, named bonuses by item level, found in chests; bodies to loot (what the dead wore and carried + their loot table), chests and bodies both containers; wands and staffs hold their spells (a wand one, a staff two; found ones roll theirs) and glow with element auras; enemies wear and wield rolled gear and drop it; uniques (Troll's Club, Broodmother's Fang with venom on hit); trinket icons; rocket boots (fire exhaust) and cloud boots; spiders bite (lunge, knockback), spit acid globs, sting (rear up, stinger over the back, venom), fewer spiders and bats; the grappling hook (Hook slot, E: a swinging rope that wraps round corners, reels, leashes small things, tears loose when its cell goes; three hooks in loot) (branch `gear-arc`, DESIGN §7c) | done; next: first magic batch (beam/wall/cloud/call carriers, radiant, void) |
| 5 | Terraria layer: items, inventory, mining yields, building, crafting, lighting | not started |
| 6 | Hollow Knight layer: ability unlocks, map, bosses, benches, set pieces | not started |
| 7 | Pixel rigid bodies (falling terrain chunks) | optional |
| — | Co-op networking (host-authoritative, checksum + chunk resync) | seams in place, not built |

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
- Stars and a moon; far parallax clouds.
- A parallax far background per depth band and biome, lit by the light grid:
  rock strata and huge cavern shapes underground, a basalt glow near the
  underworld, distant mountain ranges and forest silhouettes on the surface
  (from the real WorldPlan). Replaces the flat underground backdrop. After
  structures, with the art tool.

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
   using the existing `attack` clip. Pogo on down-strikes.
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

- Generated trees whose branches interlock with a neighbour's hold each other
  up: cut one and it stays standing. Physically right; the forest plan could
  keep wood apart (found by the felling test in the larger world).

- Liquids level to within 1–2 cells across long flat stretches (terracing); fine visually.
- Gas collects under the top of the loaded area (unloaded = wall).
- `find_ground` retries forever for a spawn over a bottomless column.
- Creatures outside loaded chunks freeze (by design) — they resume when loaded.
