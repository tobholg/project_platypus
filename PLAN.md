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
3. Background layer + plants (non-solid, flammable) + forests and tall grass in
   worldgen + wind (drift for gases/fire/embers; sway as a render effect)
4. Rigid bodies spike (Rapier): detached pieces > 64 cells become pixel bodies;
   tree felling (trees live in the background layer, fall as real bodies)
5. Sky: parallax clouds, cloud cells, rain/snow by temperature, lightning
6. Meteors: fireball, huge crater, ejecta, forest fires, hot meteorite ore

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
