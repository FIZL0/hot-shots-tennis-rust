# B409 — mobile browser freezes loading into a match

## Cause
After P50/B415 the match start is sliced over frames, but in the browser two things still ran in one long block:
- **P50's "async" work is not async on the web.** Bevy's task pool there is `wasm_bindgen_futures::spawn_local`: every
  spawned task runs as a microtask right after the frame that spawned it, all back to back, before the browser can
  draw again. The deferred menu sheets (`menu_art::rebuilt`, ~0.6 s of wasm per start on desktop) and the early
  character loads (`preload`) were one long freeze. A single sheet alone was 0.2–0.27 s of wasm (K = 4: each sheet is
  upscaled 16× in pixels).
- **WebGL compiles every queued pipeline in one frame** (`PipelineCache::process_queue`, synchronous on the page's
  thread): 0.3–0.43 s on desktop in the same frames.
Measured with Chrome CPU profiles (`research/b409_frames.py`, `b409_window.py`) of `tools/web_check.mjs` runs on the real
GPU (`CHROME_FLAGS="--enable-features=Vulkan --use-angle=vulkan --enable-gpu"`; headless SwiftShader is too slow to
tell a load from a normal frame).

## Fix
- `jobs.rs` (new): `spawn` / `spawn_steps` hand work off. Native: the async pool as before. Browser: a queue that
  `jobs::run` (First, before `menu_art::land`) works through between frames, 25 ms a frame, a step at a time. `wait`
  does what's left right away (the old fallback).
- `menu_art`: a sheet is rebuilt by `Steps`/`Rebuild` a few ms at a time (the upscale in 64k-pixel row bands, the
  pills, one label, the stamps one by one, icons, unpremultiply in bands). Same pixels: a temporary test compared all
  36 rebuilt sheets on the disc with the old one-shot code, byte for byte (passed; removed again since it carried the
  old code). `rebuild` (relabel, tests) uses the same steps.
- `preload`: character loads go through `jobs::spawn` (one per frame budget in the browser).
- `third_party/bevy_render` `PipelineCache::process_queue`: on wasm32, once 12 ms of the frame are spent, the
  remaining *queued* pipelines wait for the next frame (they stay in `waiting_pipelines`, so `mode::Compiling` and the
  loading screen keep waiting for them).
- `hud_gamma.rs`: the missing `RenderLayers` import (main's wasm build didn't compile after P45).
- Test: `jobs::tests::steps_run_to_the_result`.

## Numbers (browser, WebGL2, AMD GPU via ANGLE/Vulkan, CPU throttled 4× as a phone, MOBILE=1)
Real menu → match drive (`WEB_INPUT` Enters), same profile and disc, old vs new build:

| | old | new |
|---|---|---|
| menu → match: longest frame | 1350 ms (frames 1350, 1033, 883, 667…) | 600 ms (600, 467, 300…; the single `menu::setup` system) |
| menu → match: frames / time | 10 / 4.9 s | 107 / 6.9 s |
| menu boot start: longest frame | 5083 ms | 733 ms (+ the page-boot frame, below) |
| `--play` match start, unthrottled | 1.2–1.3 s stretches | longest task ~320 ms |

The machine was shared with other agents' runs, so single frames are noisy (±30 %). The whole load takes ~2 s longer
throttled (the loading screen now draws ~100 frames instead of freezing), a trade for a page that keeps answering.
Wasm heap: 469 MB peak, reached loading the menu; loading the match didn't grow it.

## Not verified / not 1:1
- **No phone.** No iOS/Android device or emulator log; a 4×-throttled desktop Chrome stood in. Whether the crash the
  user saw on mobile is gone can't be confirmed here: if it was the page being killed for hanging (iOS kills a
  WebContent process that blocks long enough), the shorter frames help; if it was memory (469 MB wasm heap + the
  171 MB pack copy in JS during boot + GPU textures, the K = 4 sheets are up to 16 MB each), this doesn't change that.
- **Still single frames:** `menu::setup` (~320 ms throttled), `hst::load` (~200 ms), `play::setup` (~120 ms) and the
  page's boot frame before the menu (2.3–2.9 s throttled, ~570 ms unthrottled: wasm instantiation, the pack mount and
  the app build — at page load, not when loading into a match). Splitting those means editing play.rs/main.rs, which
  other agents edit concurrently.
- A single pipeline or job step is never cut, so one slow shader compile on a phone can still be a long frame.
- WebGPU build not run in the browser (the job queue and pipeline budget are the same code; only WebGL2 measured).
- Loading/menus are ours to design (PLAN.md): no comparison with the original needed; native match screenshot
  (`context/b409/native-play.png`) shows the rebuilt HUD art as before.
