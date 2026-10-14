# B345: skip what can't be seen (frustum culling for characters, shade copies, effects)

## Changes
- Characters' skinned parts and rackets, their second-draw shade copies (`shade.rs`), every effect model part and
  CPU-written effect mesh (`effects.rs`, `play/foot_fx.rs`) were `NoFrustumCulling`. They now carry
  `cull::Cullable` (new `crates/hst/src/cull.rs`):
  - skinned parts get Bevy's `DynamicSkinnedMeshBounds` over per-joint bind-pose boxes that `cull::bound` builds
    itself. Each vertex's box is widened by its morph targets' reach (per axis, the sum of the negative / positive
    offsets) and the box gets a margin of 0.1 × the part's largest extent for the costume sway (`noise.rs`). Rigs
    animate on the CPU, so the boxes follow them.
  - rigid / CPU-written meshes use Bevy's own box of their positions, recomputed on each write (`cpu_morph` copies
    the skinned bounds onto its rebuilt mesh).
  - a skinned mesh with no skinned vertices (an odd mod mesh) stays unculled (`unbounded`).
- Setting: `Graphics::cull` (not part of a preset), **Settings → Graphics → "Skip Off-Screen"**, default **on**;
  `cull = on|off` in settings.txt; `--cull on|off` (wins over settings.txt for that run); browser `?cull=off`.
  Turning it on or off mid-run updates every mesh already spawned (`cull::apply`). I put it in the Graphics tab, not
  Display, next to the other performance options.
- `HST_CULL_CHECK=1`: skins every culled part on the CPU each frame (morph weights included) and logs how far the
  farthest vertex lay outside its box.
- `tools/web_check.mjs` `WEB_TRACE=s:n`: Chrome trace main-thread CPU ms/frame, plus user-space
  instructions/frame from the busiest renderer thread via the new `tools/instr.py` (perf_event_open).

## Bounds proof
`HST_CULL_CHECK` native 4-bot match: farthest vertex outside its box **0.0000** over 15,199 frames
(~157 parts/frame, ~75 % of them drawn). Mutations: a box 0.02 short gives 1.46 outside, and no sway margin gives 0.876
(the check catches both). A whole-mesh morph reach overshot by up to 2.7, so the reach is per vertex / per joint.

## Looks the same
Native `tools/shot.sh --play --stage 1 --cull on|off` (context/b345/look/): at 20 s, 0.005 % of pixels differ by
more than 16 (players' idle animation phase); at 6 s, 1.5 % differ (players running in at slightly different
times). Same scene, same draw counts. The browser builds look the same (context/b345/web/d-on, d-off).

## Numbers
Native `tools/bench.sh` (court 1, 4 bots, 50 s, mean fps after 15 s). Other agents were loading the machine
(load average 12–18), so the runs are noisy. Off → on, each pair run back to back:

| setup | off | on |
|---|---|---|
| dGPU, shadows High | 123.2 / 127.0 / 146.2 | 175.0 / 237.9 / 179.8 |
| iGPU, HST_SLOW=1 | 80.8 / 64.9 | 96.8 / 84.2 |
| iGPU, `--shadows low` | 101.7 / 74.9 | 196.0 / 102.7 |

On was faster in all 10 pairs (≈ +50 % dGPU, +23 % iGPU slow). In the default broadcast view the mesh counter
shows the same count (580 drawn, 170 cast), so the gain comes from cut-aways, close-ups and the other views where
players, crowd figures and effects leave the camera's and the sun's view.

Browser (web_check, :8766, profile with the OPFS pack, MOBILE=1):
- `THROTTLE=4`, frames 25–50 s: too noisy to read. Sequential runs ranged WebGPU off 17–52 / on 22–37 fps, WebGL2
  off 16–63 / on 25–62. Running off and on at the same time gave identical numbers because they share the GPU, so
  those paired runs don't count.
- Unthrottled main-thread instructions per frame (`WEB_TRACE=25:20`, ×10⁶): WebGPU off 60.25 / 59.36 / 58.32 vs
  on 61.36 / 59.05 / 59.30; WebGL2 off 51.14 / 50.49 / 49.82 vs on 49.75 / 50.59 / 49.70. No measurable
  difference either way: the per-joint box tests cost about what the skipped draws save on the browser's main
  thread.
- Decision: **default on**. It's clearly faster natively and no slower in the browser. Anyone who finds it slower
  can turn it off in Settings or with `?cull=off`.

## Occlusion culling (tried, dropped)
Bevy's `OcclusionCulling` needs a camera `DepthPrepass`. `GsMaterial` has the prepass off, so without changes the
HZB would hold no court and cull nothing. With the prepass switched on for GS materials plus `DepthPrepass` and
`OcclusionCulling` on the camera (native test, temporary env switch):
- **wrong picture**: the clear colour shows through along the hedge border and the far fence line. Coplanar GS
  draws that rely on draw order / `depth_bias` lose to the prepass depth.
- **slower**: 42 vs 81 fps in the same loaded moment (prep 1.9 vs 0.4 ms).
The broadcast camera looks down onto the court, so little sits behind the stands anyway. Not shipped: no toggle and
no URL switch. Making it work would mean redoing how the GS draw order is modelled in depth, for little to gain.

## Not verified / not 1:1
- **Phone not measured**: no phone here. The 4× throttled desktop browser numbers were too noisy to show a
  difference either way. The default stays on because nothing measured got slower.
- **Browser fps gain not shown**: browser main-thread instructions per frame are unchanged. The gain is native
  only, as far as measured.
- **Sway margin is a heuristic** (0.1 × part size). It held every vertex over a full match at court 1. Mods with
  much bigger sway or morph-free deformation aren't covered by the check; such a mesh would pop out at the screen
  edge, and turning the setting off fixes it.
- **Occlusion culling**: tried and dropped (wrong picture, slower), see above.
- Rendering and performance only: no gameplay code touched, so there's nothing to match against the original.
