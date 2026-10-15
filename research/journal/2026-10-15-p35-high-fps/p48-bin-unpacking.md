# P48 — GPU side: why `bin_unpacking` cost more than drawing (2026-10-15)

## Why
Bevy 0.19's indirect drawing groups each view's binned meshes (opaque, alpha mask) into *batch sets*: one per
pipeline + material bind group + vertex/index slab. Every frame `unpack_bins` (`bevy_pbr` `gpu_preprocess.rs`)
runs **one compute dispatch per batch set**, each writing the shared work-item buffer, before the draws.
`GsMaterial` isn't bindless, so every court/character material is its own batch set. New `HST_PASSES=1` line
(`graphics::log_unpacking`, render world) in a 4-bot doubles match, court 1:

    bin unpacking at 20 s: 342 dispatches over 4 (view, phase)s of 3 views, 435 instances (largest batch set 13)

So ~1.3 meshes per dispatch. The pass is the fixed cost of ~350 tiny dispatches with barriers between them, not
of the instance count or rebinning: 0.63–0.68 ms GPU on the RX 9070 XT, about 1.9 µs each. On the CPU
`unpack_bins` takes ~290 µs/frame to record them and `prepare_preprocess_bind_groups` ~180 µs to rebuild one bind
group per job each frame (traced, `context/perf/p48/prof-ind1.txt`). Multi-draw had nothing to merge in batch sets
that small. Shadow cascades already draw directly: Bevy gives them `NoIndirectDrawing` unless it runs in its
occlusion-culling mode.

## Change
`graphics::direct_draws` (PostUpdate) adds `NoIndirectDrawing` to every `Camera3d`: Bevy still builds the mesh
uniforms on the GPU (`early_mesh_preprocessing`), but draws directly, so there are no batch-set unpacking jobs at
all. `HST_INDIRECT=1` keeps Bevy's indirect path (bench knob; listed in `tools/bench.sh`). Where the backend has no
multi-draw-indirect-count (Metal/iOS, WebGPU, GL), Bevy already drew directly, so this is a no-op there. It
changes Vulkan/DX12: desktop and the Steam Deck.

## Numbers (`tools/bench.sh`, 4-bot doubles, court 1, 1280×800, `HST_PASSES=1`; load average 20–30 from other agents)
GPU ms per pass (the last 5-s average). Before = `HST_INDIRECT=1`, after = default:

| run | bin_unpacking | main opaque | transparent | msaa_writeback | ui | fps (mean of 5-s samples) |
|---|---|---|---|---|---|---|
| 9070 XT, Default, before | 0.65 | 0.21 | 0.09 | 0.09 | 0.03 | 339 / 176 / 178 |
| 9070 XT, Default, after  | 0.00 | 0.21 | 0.09–0.10 | 0.09 | 0.03–0.04 | 214 / 263 / 219 |
| iGPU + 2 cores (`HST_GPU=igpu HST_SLOW=1`), Default, before | 0.47 | 3.75 | 1.76 | 1.49 | 0.86 | 70 |
| same, after | 0.00 | 4.02 | 1.73 | 1.55 | 0.60 | 71 |
| iGPU + 2 cores, Phone (`--scale 67 --shadows low --msaa 2x`, uncapped), before | 0.40 | 2.37 | 0.33 | 0.66 | 0.47 | 131 |
| same, after | 0.00 | 2.25 | 0.37 | 0.61 | 0.39 | 139 |

- 9070 XT: GPU frame ~1.3 → ~0.6 ms. The dGPU run is CPU-bound and its fps swings ±50 % with the load: averaged
  over the three alternating pairs, 231 before vs 232 after. CPU ms/frame (`cpu` line) 7.11 before vs 5.96 after
  (avg of the 3 pairs). Traced, the render graph's share (Core3d / Render) went from 0.24 to 0.19 (two pairs), and
  no new render-world system appeared on the direct path (`context/perf/p48/prof-ind*.txt`).
- iGPU (the Deck's stand-in): GPU-bound in main opaque (~4 ms at 100 % scale, 4x MSAA); −0.4–0.5 ms GPU.
  Default: 70 → 71 fps. Phone: 131 → 139 fps, so Phone's 60 cap now has more than 2× headroom on this stand-in.
- What weak GPUs wait on next is the opaque pass at full scale + 4x MSAA, plus msaa_writeback (1.5 ms at Default on
  the iGPU). Phone/Low already trade those away. Not changed here: they're settings, not waste.

`--shot` of the match with and without (`context/perf/p48/match-ind{0,1}.png`): court, players, rackets, umpire,
net, shadows, spectators and HUD all draw.

### Not verified / not 1:1
- Rendering only (no sim code touched); no pixel diff, as `--shot-at` is wall-clock, so the two shots are
  different moments. Checked by eye.
- Absolute fps is not reliable: other agents kept the load average at 20–30. Per-pass GPU times (timestamps) are
  solid; the CPU and fps comparisons are averages over alternating runs.
- Not run on a real Steam Deck, phone, Metal or WebGPU: on the last two the code is a no-op (Bevy already draws
  directly there).
- Making `GsMaterial` bindless (one batch set per pipeline, so indirect would batch for real) was not tried: it
  rewrites the material shader's bindings, and it isn't available on Metal/WebGPU.
