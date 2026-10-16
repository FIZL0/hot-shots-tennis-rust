# B452 — Android Chrome WebGPU: "The game stopped: Instance dropped in popErrorScope"

## Cause
The text is Chrome's own: `GPUDevice.popErrorScope()` rejects with `OperationError: Instance dropped in popErrorScope`
when Dawn cancels the callback (the device's instance / GPU-process connection went away) instead of resolving it.
Nothing in wgpu turns that into a stop: its webgpu backend maps a rejected pop to "no error"
(`future_pop_error_scope`), and Bevy's shader cache only pushes/pops one Validation scope per module through it.
The rejection came from the page itself: B223's `createShaderModule` wrapper in `web/index.html` pushes three scopes
and pops them with `Promise.all(...).then(...)` and no rejection handler, so the cancelled pop went to
`unhandledrejection` → `fail()` → "The game stopped: Instance dropped in popErrorScope". Not a wgpu lifetime bug on
our side (the game creates no device or instance of its own; only Bevy's one).

## Fix (web/index.html)
- The wrapper handles the rejection (`dropped()`): logged; before the first frame the page reloads on the WebGL2 build
  (`hstWebGL2`, which also respects the Renderer = WebGPU setting); after it, the game keeps going, and a really lost
  device still goes through Bevy's DeviceLost error (reload) as before.
- Any other unhandled rejection whose message says "Instance dropped" goes the same way instead of stopping the game.
- `fail()` treats "Instance dropped" like B244's validation errors (WebGL2 build) if it reaches there another way.

## Verified
`node tools/web_pop_error_scope.mjs` (headless chromium, the page's boot script with a stand-in GPUDevice whose
popErrorScope rejects like Chrome's): before first frame → WebGL2 fallback, not stopped; after → running, no reload;
stray rejection → running; another rejection → still stops with its message. Run against the previous index.html the
first three FAIL with exactly the user's "The game stopped: Instance dropped in popErrorScope". `web_script_error.mjs`
still passes.

## Not verified / not 1:1
- Not run on a real Android phone (none here): the fix is checked with a stand-in device, not Chrome's real
  cancellation. Why Android Chrome's GPU process drops the instance at start (driver/GPU-process crash) isn't known;
  if it keeps happening the page now lands on WebGL2, which the user reports plays fine there.
- Gameplay untouched (page script only), so no PINE/oracle check applies.
