# B343: fewer shadow draws and crowd batching as switches, measured

## What landed
- **Settings → Display → Fewer Shadows** (`Graphics::fewer_shadows`, default **on**; `--fewer-shadows on|off`,
  URL `?fewer-shadows=off`): B351's cut (at Shadows Low only players, umpire and props whose shadow reaches the
  court cast, `shadow::Minor`) now has a switch; off, Low casts as many as High. Applies right away.
- **Settings → Display → Batch Crowd** (`Graphics::batch_crowd`, default **off**; `--batch-crowd on|off`,
  `?batch-crowd=on`): a court's repeated creatures load their model once and share its meshes and GS materials
  (`shade::Shared`); each one's sun-shade light scale goes into its draws' `MeshTag` (top bit set + f32 bits,
  read in `gs.wgsl`) instead of into its own material, so the renderer can instance them. Fading creatures (types
  15, 31, 32: their alpha is per-material) and the umpire never share. Takes effect from the next match.
- Both survive presets (`Graphics::keep`), are saved like the other settings and only appear in `flags()` when
  off their default.
- `HST_FPS=1` also logs every 5 s: meshes the scene cameras see, distinct (mesh, material) pairs (the lowest
  draw count batching could reach), the creatures' share, and the shadow casters per view and cascade.
- **`tools/bench.sh` drew no court**: it passed `--court 1` (physics) but not `--stage 1` (what is drawn), so
  every earlier native bench number, including B328's 216 → 223 fps, was players + sky on an empty plane. It now
  passes `--stage 1 --court 1`; give `--stage N --court N` for other courts.

## Draw counts (native, HST_FPS draws line)
- Court 3, Shadows Low: shadow casters in the Low cascade **239 → 108** with Fewer Shadows (players and
  umpire parts 79, props reaching the court 29). Main view ~285–295 meshes, ~280–290 distinct pairs: almost no
  two meshes share a mesh and a material, so nothing batches today.
- Creatures per court: the 4 walkers are always 4 different models; repeats are trigger creatures only (court 2:
  6 of one; 6: 6; 10: 2 + 6; 9: 5; others 2–4). Court 5's crowd is 8 stand models, each with its own per-figure
  UV frames in its material (can't share). Batch Crowd on court 10: pairs 288 → 285 (npc 26 → 23). WebGL2: Bevy
  never batches skinned meshes, so Batch Crowd can only matter on WebGPU/native.

## Frame rates
Machine under heavy load from other agents' runs (load average 10–18 on 16 threads, several chromiums), so fps
swings 2× between identical runs. Alternating A/B/C, medians of runs; "cpu" = process CPU ms per frame, the
steadier number.

Native, `tools/bench.sh … 30 --stage N --court N --shadows low`, 1280×800, 4 bots:

| GPU, court | Fewer Shadows off | on | on + Batch Crowd |
|---|---|---|---|
| 9070 XT, c3 (series 1, 3 runs) | 103 fps, 14.27 ms cpu | 158, 12.47 | 200, 10.84 |
| 9070 XT, c3 (series 2, 5 runs) | 138, 13.95 | 110, 12.84 | – |
| 9070 XT, c10 (series 1, 3 runs) | 130, 14.06 | 172, 12.37 | 167, 12.20 |
| 9070 XT, c10 (series 2, 4 runs) | – | 184, 11.54 | 154, 11.61 |
| iGPU (Raphael), c3 (series 1) | 117, 12.43 | 91, 13.51 | 104, 12.34 |
| iGPU, c3 (series 2, 5 runs) | 75, 15.16 | 69, 15.57 | – |
| iGPU, c10 (series 1) | 152, 12.43 | 159, 11.00 | 110, 13.51 |

Browser, `tools/web_check.mjs` MOBILE=1 THROTTLE=4 (Vulkan ANGLE flags as part 1), `?play&shadows=low&stage=N`,
frames 25–50 s, 2 runs each, mean fps (median frame ms). WEB_FAST build (no wasm-opt):

| | c3 Fewer off | c3 Fewer on | c10 Batch off | c10 Batch on |
|---|---|---|---|---|
| WebGL2 | 20.1 (47 ms) | 17.7 (52–60) | 19.9 (44–57) | 19.4 (53) |
| WebGPU | 20.6 (48–50) | 19.8 (48–52) | 17.8 (57) | 20.6 (47–48) |

(Part 2 measured ~56 fps on the same setup on a quiet machine; today's ~20 fps is the load, so only large
differences would show.)

## Verdict
- **Fewer Shadows: stays on.** Half the shadow-pass draws at Low (239 → 108 on court 3), and in both native
  9070 XT series ~1.1–1.8 ms less CPU per frame (8–13 %). Fps, the iGPU and the browser show nothing measurable
  through the noise either way; no run showed it costing anything consistently. B351 already checked it looks the same.
- **Batch Crowd: off by default, no measurable gain.** At most 3–6 fewer draws on the courts with repeated
  creatures, nothing at all on WebGL2 (skinned meshes never batch there), and native/web fps changes are in the
  noise in both directions. Kept as a switch in case a device with costly draw calls gains from it.

## Visual check
- Batch Crowd on vs off, court 10, fixed eye (`HST_EYE`) on a shared type-43 urn: its crop is pixel-identical
  (max diff 0); the rest of the frame differs only where walkers' animation timing differs from run to run
  (two off-runs differ as much). WebGL2 and WebGPU runs with it on show the shared creatures normally and log no
  new warnings.
- Settings → Display with the two new rows at 1280×720 and 600×800: lays out like the other rows.

## Not verified / not 1:1
- No phone measurement: the desktop's throttled Chromium stands in (as in parts 1–2); Safari/iOS untested.
- Fps numbers are dominated by other agents' load; only draw counts and the CPU ms/frame trend are solid. A
  quiet-machine rerun (`tools/bench.sh`, web_check as above) would firm them up.
- Court 5's spectators and the walkers are not batched (per-figure UV frames; every walker a different model).
- Shared creatures multiply the light scale in the vertex shader instead of into the material colour: the same
  value, float order slightly different (rendering only has to look right; crop matched exactly).
- B343 adds no menu-to-original comparison: the settings rows are our own UI.
- Lead not taken here (filed as B359): the main menu's 3D cursor camera stays active through a match.
