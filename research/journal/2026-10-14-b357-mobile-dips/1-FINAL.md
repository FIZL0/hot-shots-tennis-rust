# 1 — FINAL: stop re-uploading meshes and re-preparing materials every frame

## Method

- Browser build (WebGPU, `--keep-debug`, no wasm-opt), Chrome with mobile emulation (`MOBILE=1`, 4x CPU throttle),
  `tools/web_check.mjs` with a 10 s CPU profile 30 s into a bots doubles match on stage 1, shadows/msaa off,
  `show-fps-detailed`. Old and new wasm profiled back to back (`context/b357/cmp*-{old,new}`).
- `research/b357_spikes.py FILE [N]`: cuts a cpuprofile into frames (`App>::update` runs) and lists what the slowest
  frames spend beyond a typical one.
- Natively: temporary diagnostics counting `AssetEvent::Modified` per mesh/material and naming the entities using
  them (removed again).
- The host was shared with other agents (load average 15–35), so whole-frame medians/maxima swing ±30 % between
  identical runs; comparisons below are per-function costs, which hold steady across runs.

## Findings

1. **Costume sway re-uploaded 12 meshes every frame.** `noise::draw` did `Assets<Mesh>::get_mut` + new POSITION on
   each rig's sway copies (≈3,600 vertices/frame). Each `get_mut` makes Bevy re-extract the mesh, free and
   re-allocate its slab space, repack every attribute and write it all again (`allocate_and_free_meshes`,
   `write_packed_vertex_buffer_data`, `writeBuffer`): 0.84–0.94 ms/frame on the throttled profile, max 3–5 ms.
2. **Materials re-prepared every frame for nothing new.** On the web, every material bind-group preparation calls
   wgpu's `Device::features` (and `limits`), which round-trips the feature list as JS strings
   (`map_wgt_features`, `decodeText`): 1.3–1.7 ms/frame, with single calls of 29 and 57 ms — the biggest one-frame
   dips in the profiles. Per 600 frames, natively:
   - one effect StandardMaterial (`effects::pose`, alpha rewritten while unchanged): 597–600×
   - the four player markers (`play/markers.rs`, uv + colour rewritten while unchanged): ≈455× each
   - the HUD composite (`hud_gamma::persist` re-pointed the one Composite at `last[0]`/`last[1]` alternately
     every match frame): every frame
   - the ball's GS draws (shade value genuinely changes as the ball moves): ≈455× — left as is
3. The flare rewrote its meshes every frame even while hidden (`on` false).
4. Pipelines: natively (`HST_PIPELINES`, a temporary counter) every render pipeline was compiled on the match's
   first frame and none after in 150 s of bots, so on-the-spot pipeline compiles aren't the in-match dips (only bot
   matches checked).

## Changes

- `crates/hst/src/vertex_patch.rs` (new): `Patched` keeps a mesh's vertex data packed exactly as Bevy's mesh
  allocator lays it out (attributes in id order, interleaved); `set` moves one vertex's position, `send` queues the
  runs of moved vertices (gaps ≤16 merged) and the render world writes them straight into the mesh's slice of the
  vertex slab (`MeshAllocator::mesh_vertex_slice` + `RenderQueue::write_buffer`) after the frame's allocations.
  Writes for a mesh not allocated yet wait up to 60 frames. The asset itself is never touched after it is added.
- `noise.rs`: the sway copies are `Patched`; `draw` computes exactly the same positions (same maths) and sends
  only the moved vertices; no more `get_mut`.
- `effects.rs` `pose`: alpha written only when it differs (`EffectMaterial::base`).
- `play/markers.rs`: uv/colour written only when they differ.
- `hud_gamma.rs`: three Composite materials (scene, last[0], last[1]) built once; `persist` swaps the composite
  node's `MaterialNode` handle instead of mutating the material.
- `play/flare.rs`: hidden (`!on`) → no mesh write.

## Numbers (browser, mobile emulation, 4x throttle, per frame over the 10 s window)

| | old (2 runs) | new (2 runs) |
|---|---|---|
| `allocate_and_free_meshes` | 0.94 / 0.84 ms | 0.01 / 0.02 ms |
| `map_wgt_features` (`Device::features`) | 1.70 / 1.26 ms, max 29.2 | 0.79 / 0.84 ms |
| `unprepared_bind_group` | 0.79 / 0.64 ms, max 31.6 | 0.43 / 0.39 ms |
| RenderMesh prep (first pair) | 3.18 ms | 1.76 ms |

Natively, StandardMaterial modifications per 600 frames went from ≈2,400 to ≈135 (balloons/words fading), and the
rig meshes no longer appear among modified meshes (the top writers are now the flare and 3–8-vertex quads,
≈300 vertices/frame).

Whole-frame medians (33–41 ms) and maxima moved with the host load, not with the build, so they don't show the
change; on a phone the removed work is ≈1.5–2 ms per frame plus the one-off 30–60 ms `Device::features` stalls.

## Leads not fixed (outside our code or too big for this task)

- Bevy calls `Device::limits` / `features` per frame inside its own mesh bind-group preparation (skinned/morphed
  layouts, batching checks): ≈0.8 ms/frame left. Needs a Bevy/wgpu patch (cache the limits/features).
- The ball's GsMaterials are re-prepared whenever its shade value changes (most frames of a rally); would need the
  shade scale moved to a per-instance value or a shared uniform.
- `hud_gamma::Mix` weight is written every frame (frame-time dependent).
- `doubles_ai::path` (sim, must stay bit-exact) up to 15.8 ms in one frame under throttle; submit; ui_layout;
  wgpu label `decodeText`.

## Not verified / not 1:1

- No measurement on a real phone: only Chrome's mobile emulation with CPU throttle, on a host shared with other
  agents, so absolute FPS on the user's device isn't known; per-function costs are the evidence.
- Sway, markers, composite/trail checked by eye in native screenshots (`context/b357/shots/`), not pixel-compared
  with the build before the change.
- Only bot matches on stage 1 profiled; touch HUD, replays and other courts not profiled.
- The vertex patch assumes Bevy's packed vertex layout (`Mesh::get_vertex_size`, attributes in id order); a Bevy
  upgrade that changes it would garble swaying parts (checked against bevy_mesh 0.19's packing).
