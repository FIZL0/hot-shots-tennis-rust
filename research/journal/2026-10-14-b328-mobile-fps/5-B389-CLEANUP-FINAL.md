# B389 — speed-up switches: measure, keep or remove (2026-10-15)

Went over the four speed-up switches from B342/B343/B345 (Merge Court Meshes, Fewer Shadows, Batch Crowd,
Skip Off-Screen). Removed the two that don't help and kept the two that do.

## Method: the machine wasn't quiet, so fps wasn't used

The task said to re-measure on a quiet machine. Other agents kept the load average at 10–37 the whole time, so
fps and ms/frame swung by more than the differences between switches. I used numbers that don't depend on load:

- **CPU**: user-space instructions retired by all of the process's threads (perf_event_open; `tools/instr.py PID
  SECS all`). The sim runs at a fixed rate, so raw instructions per frame go *up* when fps drops (same sim work
  spread over fewer frames). I fitted instr/s = a_cfg·fps + b over all runs of one court, with b (fixed per-second
  work) shared across configs. a_cfg is the per-frame cost. Without this fit, Batch Crowd looked 5.5 % cheaper. It
  isn't.
- **GPU**: per-process amdgpu fdinfo `drm-engine-*` ns per frame. On the 9070 XT this is steady to about ±3 %. On the
  iGPU (Raphael, which the desktop also uses) it moves 6.5–8.9 ms with nothing changed, so it can't be used there.
- **Web**: `web_check` MOBILE=1 THROTTLE=4 WEB_TRACE=25:25, Chrome with Vulkan ANGLE, two runs per config. Columns
  are page instructions per frame, GPU-process instructions per frame and GPU busy time (`web_check` now logs the
  last two).

All runs: 1280×800, Shadows Low, 4 bots (`--play --chars 0,1,2,3`), 45 s bench (`tools/bench.sh` window), court 3
(Wild West: has props to merge or cull) and court 10 (biggest crowd, for Batch Crowd). Every config was measured
against a base taken in the same session.

## Numbers

Native, dGPU (fitted CPU M instr/frame ± s.e., GPU ms/frame):

| config | CPU | GPU | shadow casters |
|---|---|---|---|
| c3 base (Fewer Shadows on, Skip Off-Screen on) | 40.5 ± 0.4 | 1.40 | 108 |
| c3 Fewer Shadows **off** | 44.7 ± 0.5 | 1.70 | 239 |
| c3 Skip Off-Screen **off** | 40.6 ± 0.3 | 1.43 | |
| c3 Merge Court Meshes **on** (earlier set, base 41.0 ± 0.5) | 42.9 ± 0.8 | — | 157 |
| c10 base | 41.9 ± 1.0 | 1.40 | |
| c10 Batch Crowd **on** | 42.3 ± 1.0 | 1.41 | |

Skip Off-Screen over the whole 105 s match window, cut-aways included: CPU 40.4 vs 40.5 (no change). GPU with it on
was 1.39 / 1.40 / 1.39 and with it off 1.45 / 1.40 / 1.41. A second set gave 1.43 / 1.40 / 1.43 (on) and
1.45 / 1.46 (off). In all three sets it's about 2 % less GPU with it on.

Native, iGPU: CPU fit c3 base 39.8, Fewer Shadows off 44.0, Skip Off-Screen off 40.4; c10 base 41.2, Batch on 41.6.
GPU time too noisy to separate anything.

Web, court 3 / 10 (fps ×2 runs, page M instr/frame, GPU-process M instr/frame, GPU busy ms/frame):

| | WebGL2 | WebGPU |
|---|---|---|
| s3 base | 20.2, 20.9 · 251 · 13.1 · 1.28 | 21.9, 24.2 · 221 · 9.6 · 0.48 |
| s3 Fewer Shadows off | 19.8, 20.1 · 262 · 14.1 · 1.44 | 210 · 9.4 · 0.49 |
| s3 Merge on | 16.1, 21.6 · 243 · 13.6 · 1.44 | 211 · 9.5 · 0.48 |
| s3 Skip Off-Screen off | 21.7, 23.2 · 229 · 13.8 · 1.30 | 207 · 9.2 · 0.49 |
| s10 base | 21.1, 18.1 · 255 · 12.6 · 1.22 | 15.5, 16.8 · 276 · 9.7 · 0.51 |
| s10 Batch on | 15.9 (1 run) · 295 · 11.4 · 1.20 | 16.5, 20.8 · 239 · 10.8 · 0.48 |

Page instructions per frame on the web move about ±10 % between runs of the same config. An fps fit like the
native one gives standard errors as large as the values themselves with two runs, so I only read the
GPU-process columns, which are steadier. The second WebGL2 Batch run recorded no frames.

## Verdicts

- **Merge Court Meshes: removed.** No gain anywhere, and worse on native: +5 % CPU. The merged groups also lose
  Fewer Shadows' per-prop cut (casters 108 → 157), and WebGL2 GPU goes 1.28 → 1.44. B342's web recheck had already
  found no gain.
- **Batch Crowd: removed.** Native CPU and GPU unchanged on court 10 (42.3 vs 41.9, 1.41 vs 1.40). The web result
  is mixed and not reliable: WebGPU page cost went down, WebGL2 went up, and GPU-process cost went up on WebGPU.
  WebGL2 can't batch skinned meshes at all (B343).
- **Fewer Shadows: kept.** −10 % CPU and −18 % GPU on native. On WebGL2, GPU goes 1.44 → 1.28 with it on.
- **Skip Off-Screen: kept.** CPU is neutral and native GPU is about 2 % lower in all three sets. B345 measured
  larger gains in other views. It costs nothing to keep.

## What was removed

- `crates/hst/src/merge.rs` (the whole merger), its plugin, the `load` hook in `main.rs`, and the `gfx` parameter
  B342 had added to `load`.
- `Graphics::merge_meshes` / `batch_crowd`: fields, defaults, `settings.txt` keys (`merge-meshes`, `batch-crowd`),
  `--merge-meshes` / `--batch-crowd` flags, the matching `?merge-meshes=` / `?batch-crowd=` URL switches (same
  `set()` path), and the forced-merge static.
- Settings → Display: the Merge Court Meshes and Batch Crowd rows (enum, both platform lists, names, help, toggle).
- Crowd sharing in `play/npcs.rs` (shared loads, `shade::Shared` tagging) and in `shade.rs` (`Shared`,
  `SharedDraws`, tag-based shade). `gs.wgsl` no longer reads the per-draw MeshTag scale (it was 1.0 for every
  untagged mesh, so the default image is unchanged).
- An old `settings.txt` that still has the two keys loads fine: `set()` ignores keys it doesn't know. An old URL
  or flag with them gets the same treatment as any other unknown argument.

Tools: `tools/instr.py PID SECS all` adds up every thread. `tools/web_check.mjs` WEB_TRACE also logs GPU-process
instructions per frame and GPU busy ms/frame (amdgpu fdinfo).

## Checks after the change

- `tools/check.sh -p hst`: 393 tests passed.
- Settings → Display at 1280×720 and in a tall 640×1000 window: nine rows, Fewer Shadows and Skip Off-Screen still
  there, both layouts fine (`context/b389/menu/b3.png`, `t3.png`).
- Native match on court 3 draws normally (`context/b389/menu/match3.png`).
- Web build (WEB_FAST) with the edits: one 55 s match each on WebGPU and WebGL2, no errors. WebGPU 225.6 M
  instr / 0.48 ms GPU; WebGL2 236.3 M / 1.30 ms. Same as the base before the change
  (`context/b389/web-after/`).

## Not verified / not 1:1

- **No quiet machine**: load 10–37 throughout. Verdicts rest on instruction counts and GPU time instead of fps.
  Absolute fps numbers above are load-affected.
- **No real phone**: mobile is only Chrome's MOBILE=1 THROTTLE=4 emulation on the desktop GPU. Safari/iOS untested.
- **iGPU GPU time** is too noisy (iGPU shared with the desktop), so the GPU verdicts are dGPU only.
- **Close-up culling**: `HST_EYE` doesn't apply in `--play`, so Skip Off-Screen was only measured from the
  broadcast camera and cut-aways, not from a close fixed eye.
- **Web runs**: two per config, and one (WebGL2 court 10 Batch, run 2) recorded no frames.
- Rendering-only change with nothing to compare against PCSX2. Both removed switches were off by default, so the
  default image and match play are unchanged by construction. Gameplay code wasn't touched.
