# Plan

Each phase ends with something playable or measurable. See SPEC.md for the design.

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
| 4 | Combat: weapon swings, pixel masks, swept hits, hit-stop, knockback, enemy attacks | not started |
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
7. **Worldgen v2 / biomes** (its own arc, before the save format freezes):
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

## Known issues

- Liquids level to within 1–2 cells across long flat stretches (terracing); fine visually.
- Gas collects under the top of the loaded area (unloaded = wall).
- `find_ground` retries forever for a spawn over a bottomless column.
- Creatures outside loaded chunks freeze (by design) — they resume when loaded.
