# B413 — does the Phone preset use the speed-ups? (2026-10-15)

## What the presets already had

`Graphics::PHONE`, `LOW` and `STEAM_DECK` all have Fewer Shadows on (`FEWER_SHADOWS = true`) and Skip Off-Screen
(`cull`) on. iOS and Android (touch pad on) and a touch browser get `PHONE` on first run (`main_menu.rs` `first`).
B389 already measured both switches as worth having (Fewer Shadows −10% CPU / −18% GPU, Skip Off-Screen ~2% GPU).
The open questions were a lower Mesh and Sway LOD.

## Method

The load average stayed at 24–27 the whole time (other agents), so I measured load-independent numbers as B389 did.

- **Native dGPU** (9070 XT): `tools/bench.sh`, 4 bots, court 1. Phone settings: `--fps 60 --scale 67 --shadows low
  --msaa 2x`, which held 60 fps in every run. Measured from 20 to 50 s: user-space instructions summed over all
  threads (`tools/instr.py PID 30 all`) and amdgpu fdinfo GPU ns, per frame (1800 frames). Three alternating
  rounds. Script: a scratch copy of bench + instr + fdinfo; logs in `context/b413/`.
- **Native iGPU** (Raphael, `HST_GPU=igpu`, `HST_PASSES=1`): per-pass GPU ms from timestamps, 15–40 s, three rounds.
- **Web**: `tools/web_check.mjs` MOBILE=1 THROTTLE=4 WEB_TRACE=25:25, Vulkan ANGLE flags as in part 1, WEB_FAST
  build, `?play&stage=1&fps=60&scale=67&shadows=low&msaa=2x` (+ `&mesh=75` / `&sway_lod=on`). Three rounds. The
  per-frame cost is fitted as instr/s = a_cfg·fps + b, as in B389, because fps swung 10–48 between runs.

## Numbers

Native dGPU, M instructions per frame / GPU ms per frame (runs 1, 2, 3 → mean):

| config | CPU | GPU |
|---|---|---|
| Mesh 100 (base) | 40.19, 40.34, 40.23 → **40.25** | 1.37, 1.30, 1.43 → **1.37** |
| Mesh 75 | 39.56, 40.34, 40.41 → **40.10** | 1.36, 1.37, 1.32 → **1.35** |
| Mesh 50 | 39.36, 40.01, 39.94 → **39.77** | 1.34, 1.39, 1.32 → **1.35** |
| Sway LOD on | 39.85, 39.80, 39.68 → **39.78** | 1.38, 1.40, 1.35 → **1.38** |

iGPU, main opaque pass / all passes, GPU ms (runs 1, 2, 3):

| Mesh | opaque | total |
|---|---|---|
| 100 | 2.43, 1.90, 2.04 | 4.96, 4.43, 4.35 |
| 75 | 2.12, 1.92, 1.96 | 4.64, 4.34, 4.36 |
| 50 | 1.96, 2.01, 2.03 | 4.41, 4.46, 4.43 |

Web, 4× throttle, fitted page M instructions per frame: base 125.8 ± 20.3, Mesh 75 125.9 ± 14.1,
Sway LOD 131.9 ± 14.7 (fixed 617 M/s). GPU busy was 0.44–0.61 ms/frame in every run, and the page's main thread
(21–38 ms/frame) is the bottleneck.

## Verdict

- **Mesh stays 100%.** Mesh 75 is inside the noise on every measure. Mesh 50 is at most −1% CPU on the dGPU,
  inside the run-to-run spread on the GPU and the web, and it costs the characters' look. That fits the triangle
  counts: Mesh only reduces characters, ~2.5k triangles each on the disc (B364), so a few thousand triangles per
  frame. M91 found an iPhone GPU-bound on fill (that's why Phone draws at 67%), not vertices. Skinning runs on
  the GPU, so the CPU side doesn't change. Mesh stays a setting for heavy mod characters.
- **Fewer Shadows and Skip Off-Screen** were already on in Phone/Low/Steam Deck. Their measured gains are B389's.
- **Sway LOD**: a consistent −1.2% CPU natively (the three runs don't overlap base), nothing on the GPU, nothing
  detectable in the browser. Whether it stays is B411's call (noted on B411). It's a `Settings` flag, not a
  `Graphics` field, so a preset can't set it until B411/B412 decide.
- Code: `PHONE`'s doc says which speed-ups it runs and why Mesh stays at 100. `graphics::tests` pins Fewer Shadows,
  Skip Off-Screen and Mesh 100 on the three cheap presets. Noted on B412: once presets carry the switches, Phone,
  Low and Steam Deck carry these values.

## Not verified / not 1:1

- **No real phone**: no iOS/Android `hst-log.txt` from a device. Only desktop dGPU/iGPU and Chrome's 4×-throttled
  phone emulation, which uses the desktop GPU, so a phone GPU's vertex cost is inferred from the triangle counts
  and M91's fill-bound finding, not measured.
- **Loaded machine** (load 24–27): fps not used. The web fit's error (±14–20 M/frame) is larger than any gain
  Mesh could give there.
- Rendering-only, with nothing to compare against PCSX2. No gameplay code was touched, and the presets' values
  didn't change.
