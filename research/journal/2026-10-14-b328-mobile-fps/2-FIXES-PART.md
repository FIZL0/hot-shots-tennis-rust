# B328: first CPU fixes (leads 1, 2, part of 3)

## Changes
- `effects::fill` compares the new positions/uvs/colours/indices against the mesh and only `get_mut`s when they
  differ; every caller (effects sparks/trails/ribbon/marks, `foot_fx`, `npcs::draw_cheers`, `weather/rain`) goes
  through it, so empty → empty and unchanged effect meshes no longer re-extract/re-upload.
- `noise::draw` writes the sway positions only when they changed (they still change on every drawn frame of a
  moving rig: that's the animation).
- HUD: `widescreen::put` sets a `Node`'s rect with `set_if_neq`; panel, intro, menu, popups, match_stats use it
  and `set_if_neq` their `Visibility`. A `Changed<Node>` counter in a match went from 17–29 per frame to 0.
- Shadows Low on WebGPU/native: 2048 × 1 cascade (was 1024 × 2), halving the shadow-pass draws (99 of 150 visible
  meshes cast). WebGL2 was already 1 cascade.

## Before/after (same setup as part 1, :8766, MOBILE=1, THROTTLE=4, frames 25–50 s)
| build | fps | median |
|---|---|---|
| WebGL2 before | 56.4 | 17.3 ms |
| WebGL2 after | 58.4 | 16.8 ms |
| WebGPU before | 50.6 | 19.4 ms |
| WebGPU after | 50.6 | 19.4 ms |

Native bench (`tools/bench.sh`, 4 bots): 216 → 223 fps. Mesh allocator 4.1 → 3.1 %; `ui_layout_system` stays
~5.8 % (taffy still walks the tree each frame even with nothing changed — its own cost, not ours).

## Honest result
Small: ~3 % on WebGL2, nothing measurable on WebGPU. The frame is dominated by the render graph itself on one
thread: submit 15 % + camera driver/encoding 13 % + extract ~9 %, plus WebGPU's per-call JS (labels decoded per
call). Bevy's `debug` feature (labels) can't be dropped: `mode.rs` needs type names.

## Next
1. Measure on the phone (Safari Web Inspector timeline): is it CPU or GPU there? Unknown until then.
2. Fewer draws: merge static court/prop meshes, crowd instancing, no shadow casting on small props.
3. Sway: rate-limit far rigs or move it to the vertex shader.
