# B328: fewer draws (court props merged) and less per-draw work

## Correction to part 2
`tools/bench.sh` defaults to `--court 1`, which does not load a court model: `--stage N` does. Part 2's native
"216 → 223 fps" was on an empty court. Everything below is on `--stage 1` (`&stage=1` on the web).

## Where the frame went (named WebGPU build, after the merge)
Render schedule 47 %, camera driver 15.6 %, shadow pass 7.1 %, extract 7.6 %, FixedUpdate (sim) ~10 %,
`ui_layout_system` 3.6 %, label strings decoded per WebGPU call ~5 %, and **wgpu's WebGPU backend rebuilding the
device's features/limits from JS on every call: 8.8 %** (`map_wgt_limits` 3.5 + `map_wgt_features` 3.6 +
`Device::features` 1.7). Bevy asks for `limits()` once per mesh draw (`SetMeshBindGroup` → skinning uniform check)
and `features()` in each material's bind-group prepare.

## Changes
- `crates/hst/src/merge.rs`: a court's placed props (plants) put their non-blended parts into one mesh per (material,
  casts shadow), vertices taken into place by the plant's matrix (normals by the inverse transpose, normalised;
  zero normals stay zero). Blended parts (back-to-front order), parts with another vertex layout and mirrored
  placements stay their own entities. Each plant keeps its entity and `Visibility`, so the net/eye/camera hiders and
  `HideInside` still work: when a plant is hidden or shown, the groups holding it are rebuilt without it (only on a
  change). `HST_NO_MERGE=1` spawns everything as before.
  Stage 1, 4 bots: **589 → 289 meshes drawn, 182 → 139 casting**. Stage 8 alike.
- `third_party/wgpu` (29.0.4, `[patch.crates-io]`): the WebGPU backend reads the device's features and limits once
  when the device is created and returns those (a device's limits never change). Native backends untouched.
- Materials written every frame for nothing (each write re-prepares the material's bind group):
  `play/markers.rs` (four player markers' uv/colour), `effects::pose` (effect alphas) and the ball glow disc now
  only write when the value changed. Checked with an `AssetEvent::Modified` counter (temporary, removed): no
  `StandardMaterial` is modified every frame any more.
- `effects`: an effect mesh that `fill` emptied gets `RenderLayers::none()` (`Emptied`) until it's filled again,
  so empty spark/trail meshes aren't extracted or queued.
- `graphics.rs`: the detailed fps overlay also logs a `frame split ms: …` line, so Safari's Web Inspector console on
  the phone shows the split.

## Before/after (web, WebGPU, `web_check.mjs` MOBILE=1 THROTTLE=4, `play&stage=1`, 25–55 s)
The machine was shared with other agents (load 11–50), so wall-clock fps swings 15–39 between identical runs.
Compared by ratios inside each run instead: (cpu − sim) / sim (the sim's work per frame is fixed, so this is the
non-sim cost per frame in units of it) and render / (game − sim).

| run | before (main) | after |
|---|---|---|
| 21 | 9.39 / 1.97 | 7.68 / 1.68 |
| 22 | 8.01 / 2.17 | 7.71 / 1.68 |
| 23 | 11.68 / 1.91 | 9.38 / 1.71 |
| mean | 9.69 / 2.02 | 8.26 / 1.69 |

≈ 15 % less non-sim CPU per frame; render relative to the game's own systems −16 % (the wgpu limits fix; the
merge alone, earlier runs 9/10, gave 8.3 → 7.45 with render/(game−sim) unchanged). Meshes 580 → 276, casters
170 → 127. Native fps doesn't move (render runs on its own thread there and wasn't the limit).

Visual check (`tools/shot.sh`, stages 1, 4, 8, merge on vs `HST_NO_MERGE=1`, same weather seed; in `context/b328/shots`):
props, court and their shadows identical; only character-animation pixels differ between the two shots.

## Not verified / not 1:1
- **No phone measurement**: no iPhone here. The user's numbers (15–50 fps) need re-measuring on the iPhone 15 Pro;
  the 120 fps target is not shown. The `frame split ms` console line is there for that.
- The desktop-browser figures are throttled Chromium (4× CPU) on a loaded machine; ratios, not fps.
- Merged props lose per-prop frustum culling (one bounding box per material group); off-screen props are drawn
  as part of their group. Cheaper on the CPU, more vertices on the GPU (small: court props are low-poly).
- Blended prop parts (~40 at stage 1) are still one entity each (their back-to-front order would be lost).
- Still modified every frame: the ball's two GS materials (~half the frames, shade/fade) and the characters' sway
  meshes (whole rig part re-uploaded each drawn frame, ~13 meshes); moving sway into the vertex shader is the next
  lead. Rig parts (99 meshes for 4 players) are not merged (morphs, faces, per-rig materials).
- Remaining big costs are Bevy's own: render graph/camera driver, extract, UI layout (taffy walks the tree every
  frame), WebGPU label strings.
- Rendering compared by eye only (no pixel diff of the merged vs separate meshes beyond the screenshots).
