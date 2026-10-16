# P55 — compile the match's pipelines ahead of time

Task: hst-log 10 (Chrome, WebGL2) spent ~15 s of a 19 s match load compiling ~105 render pipelines one by one
(1–2 s page freezes). Warm them while the menu sits idle (native: at boot/menu) so the load finds them ready.

## What the log showed
- Match load 93 → 112 s, pipelines 46 → 150. Labels: 38 alpha_blend_mesh_pipeline, 37 opaque_mesh_pipeline (the GS
  material, `GS_*` defs), 26 prepass_pipeline, 29 pbr_*; ~69 distinct fragment def sets.
- Which pipelines a match needs depends on the disc's materials, the court, the characters and the graphics
  settings, so the menu can't build a fake scene to cover them without the match's own assets.

## Approach
- **Record and replay** (`crates/hst-base/src/pipelines.rs`): every render pipeline the game asks for is written,
  once built, to `pipelines.json` beside settings.txt (`store`: localStorage on the web). Shaders are stored by asset
  path, and layouts and shader-def lists are interned (~190 KB for ~110 pipelines). The record is keyed to
  "adapter | backend | driver" and dropped if any of them changes. Entries no match asked for in 8 matches are pruned
  on save (at most 1000).
- **Dedup in the pipeline cache** (`third_party/bevy_render/.../pipeline_cache.rs`): new
  `prewarm_render_pipeline` queues a descriptor without marking it requested. `queue_render_pipeline` now looks up
  an equal descriptor (hashed on label, shaders, defs, buffers) and returns that id, whether the pipeline is
  already built or still compiling. `requested(id)` tells the game's pipelines from pre-warmed ones. `mode.rs`'s
  "compiling" count only waits on requested ones, so an unused pre-warmed pipeline doesn't hold up the load.
- **Feeding** (`mode.rs` `warm_up`): only in the menu (not in a match, while leaving, or behind a loading screen).
  On native Vulkan/DX12, pipelines compile on the async pool, so the whole record is queued at once. In the browser
  and on macOS they compile in place, inside the frame, so the menu queues one per frame once nobody has touched
  the controls for 1 s (keys, mouse, touch, pad buttons/sticks).
- Shader handles: `embedded://` shaders are resolved through the asset server (`get_handle`, else `load`). A strong
  handle taken from `Assets` isn't tracked by the server, so the match loaded `gs_prepass.wgsl` again as a new asset
  id and none of the pre-warmed prepass pipelines matched. Other shaders are looked up by unique path in
  `Assets<Shader>`.
- `HST_PREWARM=0` turns it all off.

## Numbers
Native (RX 9070 XT, RADV Vulkan, Mesa disk cache warm), menu → match with `HST_KEYS=3:Enter`, `HST_MENU=confirm`:
- No record: 68 pipelines compiled after Enter, load 0.94 s.
- With the record: 0–3 compiled at load (3 = a court whose GS variants weren't recorded yet), load 0.83–0.87 s.
  The menu finishes feeding the record ~0.45 s after start. Native load time barely moves because Mesa's own
  shader cache is already warm. The gain is where compiles are slow (WebGL2, cold driver caches).

Browser (headless Chromium, `?webgl2&fps`, ANGLE on Vulkan, same profile and ISO, `tools/web_check.mjs` with a
`WEB_INPUT` Enter script; autoplay set through `WEB_PRELOAD`), menu → match:

| record | match start (mode.rs "started in") | pipelines compiled during the load |
|---|---|---|
| cleared | 2498 ms, 1620 ms | ~75 |
| 108–117 recorded | 1142, 1205, 1250 ms | ~15 |

The menu fed 108 pipelines by 3.9 s after the game started. Chromium's GPU program cache was warm in every run
(same profile), so a real cold first load gains more than this.

Note: a page killed within seconds of a save can lose it. Chromium writes localStorage to disk lazily, and a 45 s
visit SIGTERMed ~12 s after the match started left only the menu's part saved; a 75 s visit kept it all. This only
happens when the browser is killed.

## Tests
- `pipelines::tests::a_recorded_pipeline_comes_back_equal` (hst-base): descriptor → record → JSON → descriptor is
  equal; pruning drops an entry unused for `KEEP` matches together with its def list.
- `hst-play` mode tests (`warm_up` takes the asset server and shaders as optional, for the headless test apps).

## Not verified / not 1:1
- A user's real cold browser load (hst-log 10's 19 s): only measured headless with a warm Chromium GPU cache, on a
  desktop GPU. The ~15 → ~1 s gain on a phone/cold cache is expected, not measured.
- The first session ever (no record yet) still compiles everything at match load, as before. So does a court,
  character set or graphics setting not played in the last 8 matches, a GPU/driver/backend change, or a record that
  was never flushed because the browser was killed.
- WebGPU in the browser, macOS, iOS, Android and Windows weren't run. Their code paths are the same as the ones
  tested: in-place for wasm/macOS, async for the rest.
- In the browser, ~15 pipelines still compiled at match load with a full record. They weren't broken down further:
  likely the random court/characters of that run plus pipelines first asked for after the save. Native showed 0–3.
- Presentation-only task (load times). It doesn't touch the sim or match timing, so there is no original to match.
