# B102 — web build: lag spikes switching characters on the select

## Cause
In the browser `previews.rs` runs one character load per frame on the main thread. Profiling `character::load_disc`
natively (release, best of 7): ~45 ms a load, of which ~40 ms is `disc_motions` (every motion of the character's
`PCANIxx.XB`: ANI2 parse, first-frame pelvis rows, `Clip::new`, the arm table). The model, textures, racket and noise
are ~5 ms. A preview plays only three motions: the menu pose (`POSE`), win (0x2e) and loss (0x2f).

## Fix
`character::load_disc_with(.., only)` / `disc_motions_with(.., only)`: the same loaders, keeping only the listed
motions (and their faces); `load_disc`/`disc_motions` call them with `None` (unchanged for the match). The select's
loader asks for `inspect::MOTIONS` = [POSE, 0x2e, 0x2f]. Native: ~45 → ~7–10 ms a load (the worker thread too).
No Web Worker or OPFS PNG cache needed: loads now fit in a frame.

Test: `previews.rs preview_loads_only_its_motions`: every grid character's preview load has exactly those three
clips, equal (length, locals at t=3) to the full load's, and the same faces.

## Measured (headless chromium, real GPU via ANGLE/Vulkan, RX 9070 XT, WebGL2 build, 1280×720)
`tools/web_check.mjs` now writes every rAF frame to `OUT/framesN.json`. Same scripted walk both times: 3× Enter into
the singles select, then 4 key presses a second for 44 s (seeded random arrows and `e` = next costume), frames
from second 16 to 61; load times from a temporary `Date.now()` log around the browser's per-frame load
(removed again). Runs in `context/b102/{before1,after1,after2}`.

| build | loads | load mean / max | frames | > 33.4 ms | > 50 ms | worst frame |
|---|---|---|---|---|---|---|
| before (all motions) | 67 | 52.0 / 105 ms | 2618 | 32 | 12 | 116.6 ms |
| after, run 1 | 67 | 9.6 / 19 ms | 2699 | 0 | 0 | 33.3 ms |
| after, run 2 | 67 | 11.9 / 35 ms | 2698 | 0 | 0 | 33.4 ms |

Screenshots of the same moments before/after show the same posed previews (diffs only from the ticker and the
portrait cursor's animation).

## Not verified / not 1:1
- Slower machines / phones: measured on a fast desktop GPU+CPU only. A phone CPU 3× slower would put a load at
  ~30 ms, near the 33 ms line; then the next step is the OPFS PNG cache (the `ponytail:` in `bake`).
- Mods in the browser: there are none (no mods folder there), so `mods::load` (texture replacements, ~130 ms) was not
  changed or measured; natively it still runs on the worker thread.
- The walk visited 67 loads (characters × costumes) by random keys, not provably every card × costume.
- Headless screenshots in SwiftShader (no GPU) are black (B101); the GPU flags above were used instead.
