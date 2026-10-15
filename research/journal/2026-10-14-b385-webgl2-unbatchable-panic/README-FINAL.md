# B385 — web panic `render_phase/mod.rs:1241:40: index out of bounds: the len is 0 but the index is 0`

B378 handled a browser with no WebGL2. With that fixed, the user's report now shows the real panic.

## Cause
WebGL2 has no storage buffers, so Bevy batches on the CPU (`batching/no_gpu_preprocessing.rs`), and skinned meshes are
"unbatchable": each gets its own uniform dynamic offset, held in a list (`UnbatchableBinnedEntityIndexSet::Dense`)
that lines up with the bin's entities by position. When a binned entity has no batch data this frame (no render mesh
instance), the prepare loop just `continue`d. The list came up short, and `BinnedRenderPhase::render` indexed it by
position (`dynamic_offsets[entity_index]`) and panicked. Any later entity would also have been drawn with the next
entity's offset.

Here: a skinned main-menu character (`Cullable`, dynamic skinned bounds) that the loading screen's warm-up (`mode.rs`
`warm`/`unwarm`) draws, then the next frame frustum-culls (ViewVisibility false, so no mesh instance), while a view's
retained Opaque3d bin still holds it. It happens once, right after "loading screen down".

## Reproduce natively
`HST_WEBGL2=1` (new, `main.rs`) runs the desktop build with WebGL2 limits (WebGL2 priority, downlevel_webgl2 limits,
8k 2D and 2k 3D textures like a browser). Then `HST_WINDOW="1280 720" hst --windowed`: before the fix the main menu
panicked with the same message in 2 of 3 runs (Opaque3d, `main_opaque_pass_3d`).

## Fix
`third_party/bevy_render` (copy of 0.19.1, `[patch.crates-io]`): the CPU prepare loop calls
`buffer_indices.skip()` for such an entity, which holds its place with an `instance_index: u32::MAX` marker. `render`
skips the marker, and an out-of-range index too. Each other entity keeps its own offset. After the fix: 5 + 3 menu
runs, no panic, and a temporary print showed the skip path running once per start. `cargo check` passes for wasm32.

## Not done
- Not re-run in the user's Chromium on the T490: needs a new web deploy.
- Bevy upstream has the same bug (no issue found). The patch can go once a Bevy release fixes it.
