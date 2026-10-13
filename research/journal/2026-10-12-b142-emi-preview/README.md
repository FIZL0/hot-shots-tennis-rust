# B142 — Emi's baked preview missing most of her model

## What dropped out

The broken bake (`cache/previews/SCUS_976.10-r4/m-getagrip_pc00_emi-…-0.png` in the main checkout) shows only the
face and the racket: 11266 covered pixels against 31513 for a whole bake. The parts that are missing are exactly the
glTF materials with `alphaMode: MASK` (`body300_f00` and `head300_f00`: body, limbs, hair). The face (`zero_face0`,
opaque) and the racket (another glb) drew. No disc character has a MASK material: a mod's MASK material maps to GS
TEST mode 10 (`mods.rs materials_of`), so the first one baked needs a new GS pipeline.

## Why

The baker read back after a fixed frame count (30 for the first job, 4 after that). The menu runs uncapped, so 4
frames take milliseconds. A cold pipeline compile takes far longer, and Bevy skips a draw whose pipeline isn't ready
yet. So the bake caught the rig before the mask pipeline existed. Whether it hit depended on the driver's shader
cache, which is why it came and went (a warm cache compiled in time).

Ruled out on the way: the upscaled textures (same alpha range as the glb's own; a warm run with them draws whole);
the GS material hand-off (at read-back every part has its `GsMaterial`, layer 6, visible, mesh loaded); loading
order (same in good and bad runs).

## Fix

- `mode.rs`: `Compiling` (the render world's count of render pipelines still compiling, already used by the loading
  screen) is `pub(crate)` with `busy()`.
- `previews.rs bake`: while a pipeline is compiling and the read-back hasn't started, the job's frame count resets,
  so the read-back comes `wait` frames after the last compile. `RENDER` 4 → 5, so every preview is drawn again.

## Check

Scratch folder with the ISO, `mods/texture-replacements` and only Emi in `mods/`. Each run baked from an empty
cache (`HST_MENU=chars HST_BAKE=all`) with `MESA_SHADER_CACHE_DISABLE=true`, so every compile was cold. Both
binaries were built from the same tree, with the gate switched by an environment variable for this test only:

| gate | runs | Emi costume 0 covered pixels |
|------|------|------------------------------|
| off  | 6    | 10334 every time (face + racket) |
| on   | 6    | 31513 every time (whole) |

10 more gated runs: all 31513. Costume 1 baked whole in every run, with or without the gate. By then the mask
pipeline was already compiled. Images: `context/b142/emi0_before.png`, `emi0_after.png`.

Pitfall: slots share `target/`, and another slot's build replaced `target/release/hst` mid-test. That gave a run of
"failures" that came from a different binary. Copy the binary out (`scratchpad/hst-gate`) before a series of runs.

## Not verified / not 1:1

- **Live select cards**: not changed. They show the PNG for `COVER` frames over a new live model, and the live
  model keeps drawing, so a late pipeline only shows for that moment. The cover could use the same gate if a gap
  ever shows.
- **`--inspect` window**: not shot. It draws every frame, so a late compile only affects its first frames.
- **Full 92-mod re-bake**: not run. The gate only adds waiting, so it can delay a bake but can't drop a part.
  Bakes get slower only while pipelines compile.
- **Cache key**: B140 (another slot, not merged yet) also bumps `RENDER` to 5. Both changes are the same line, so
  they merge cleanly. A cache baked by a B140-only build would not be redrawn, but no release sits between the two.
- Front end, so there is no comparison with the original (PLAN: menus don't have to match).
