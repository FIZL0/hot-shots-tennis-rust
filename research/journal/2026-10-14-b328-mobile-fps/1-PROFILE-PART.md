# B328: where a browser match frame goes (first profile)

## Setup
- `tools/web_check.mjs`, `MOBILE=1` (844×390 touch), `THROTTLE=4`, desktop GPU (`--use-angle=vulkan
  --enable-features=Vulkan --disable-frame-rate-limit --disable-gpu-vsync`), `?play&stage=1` (3 COM + 1P),
  `WEB_CPUPROFILE=30:15`. Runs in `context/b328/run{1,3}`.
- Function names: `wasm-bindgen --keep-debug` on `target-wasm/.../hst.wasm` (no wasm-opt) into
  `context/b328/pkgnames`, served from a copy of `web/` on :8766 (another session was rebuilding `web/`). That
  build was the WebGL2 one; the WebGPU run (run1) has no names.
- Summed with `research/b273_profile.py` (self time) and the new `research/b328_inclusive.py` (inclusive).

## Numbers
| run | fps | idle |
|---|---|---|
| WebGPU, phone-like, 4× throttle (run1) | 30 (median 22.8 ms, p95 60) | 0.4 % |
| WebGL2, same (run3) | 47 (median 17.3, p95 34.6) | 0.1 % |

The page is CPU-bound on the main thread: no idle at all. Wasm has no threads here, so Bevy's single-threaded
executor runs the sim, the app systems, extract, render-graph encoding and submit one after another on one core.
~19 ms/frame at 4× throttle ≈ 4.7 ms unthrottled on this desktop; Safari's wasm and its WebGPU/WebGL (every call
goes to the GPU process) are slower, which fits "under 60" on the phone.

## Inclusive split (WebGL2 run3, 837 frames)
- **Render world 49 %**: submit (GL replay) 15 %, camera driver / pass encoding 12.5 %, extract 8.6 %, main pass
  6.8 %, shadow pass 3.5 %, **mesh allocator 4.2 %** + mesh re-extract/prepare ~2.5 % + `write_packed_vertex_buffer_data` 1.8 %.
- **Main schedule 38 %**: `ui_layout_system` **5.9 %** (taffy relayout every frame), ui stack/clipping ~2.5 %,
  FixedUpdate 5.9 % (sim; `doubles_ai::path` 2.5 %), `noise::draw` 2.8 %, `character::animate` 1.9 %,
  `foot_fx::tick` 1.1 %, `bodyhit::pose` 1.0 %, `net_hide::hide_around_eye` 0.9 %.
- WebGPU (run1, no names): 19 % in the browser's WebGPU JS (`decode`/`decodeText` 3.7 % = wasm→JS strings, i.e. labels,
  `writeBuffer` 2 %, `createBindGroup` 1.5 + 0.5 %, `submit` 1.7 %), so the WebGPU build pays per call on top.

## Leads, cheapest first
1. **Meshes rewritten every frame even when nothing changed.** `effects.rs` (marks/puffs/dust ~743–755),
   `play/foot_fx.rs` (both puff meshes + prints ~245–262, the unused one is set to empty every frame),
   `play/npcs.rs draw_cheers`, `noise::draw` (character sway: whole character meshes re-uploaded each frame),
   `flare.rs`, `rain.rs`. Every `meshes.get_mut` marks the asset changed: Bevy re-extracts, repacks and re-uploads it
   and the slab allocator frees/allocates when the size changes (→ new bind groups on WebGPU). Fix: only `get_mut`
   when the contents differ (at least skip empty → empty); for sway, consider doing it in the vertex shader or only
   on the near/visible rigs.
2. **UI relayout every frame (6–8 %)**: something in the HUD/match UI mutates a `Node`/`Text` each frame
   (assigning instead of `set_if_neq`), so taffy redoes the tree. Find it with a `Changed<Node>`/`Changed<Text>`
   counter in a match and gate the writes.
3. **Draw-call count** (camera driver + submit ≈ 28 %): count draw items per phase in a match (crowd, props,
   effects, shadow casters); merge static court/props meshes, drop shadow casting where it can't show at Low.
4. WebGPU: labels cost a JS string decode per call; strip them in release web builds if wgpu allows.
5. Small: `bevy_ui_render::debug_overlay::extract_debug_overlay` runs in release (0.3 %): drop the plugin on web.

## Not done
- No measurement on the phone (Safari Web Inspector timeline): the GPU side (fill rate at 50 % scale, shadow map)
  is unknown; the desktop GPU hides it. If the CPU fixes don't reach 120 there, that's next.
- No named WebGPU profile (needs a `--features webgpu` wasm built with names; `target-wasm` held the WebGL2 one).
