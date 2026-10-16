# 1 — measured: nothing allocates on a smash

## Native (release, Vulkan, `HST_FPS=1 HST_AUTOPLAY=1`, doubles, 120–150 s)
A temporary log line at the smash launch (`strike`, branch 4), next to `HST_FPS`'s pipeline, asset, GPU-allocator and
stall logs plus an RSS sampler (`/proc/<pid>/status` every 0.25 s); runs in `context/b469/run1..3`.
- Six smashes, some landing at up to 199 km/h (crater + dust cloud + smash spark look + smash impact model): no
  pipeline queued after the loading screen (all 122 by 1.4 s), assets steady at 472 images / 291 meshes /
  145 + 51 materials, GPU allocator 644–645 MB within ±0.5 MB, RSS flat, no stall lines.
- Temporarily turned on the human-only smash marker (`marks.smash` search, yellow `smash_p`) for bot receivers: same.
- Instant replays (a likely "replay buffer") are compiled out of the app (`instant_replay::REPLAYS = cfg!(test)`,
  user 2026-10-12), so nothing replay-sized is held or built at a smash. The snapshot at each serve set-up runs
  every point, not only after a smash.
- Code read: the human smash's own paths (`timing::smash`, `swing::smash_scatter`, the predicted-path search capped
  at `PATH_MAX`) are math on existing state. Every effect model, spark/flight/bounce look and dust/puff mesh
  loads at match start (`effects::load*`), and the loading screen's warm-up (`mode::warm`) draws them all, so
  their pipelines compile there. Voice and SFX banks load at match start (`audio::voice_bank` …).

## Browser (WebGL2 build, headless Chromium, `?play&pads=-,-,-,-&log&webgl2`)
New: `?log` turns the `HST_FPS` logs on in the browser (`web.rs`), and `web::heap_growth` logs each growth of the
wasm linear memory (`wasm heap N MB at T s`). `context/b469/web1`, `web2`.
- Bot doubles, 150 s, two smashes: heap 202 MB at start (the 171 MB disc pack is mounted in it), growing to 388 MB
  while the match loads (by 2 s), then **no growth** for the whole match, smashes included.
- Human 1P singles (keys pressed every second; the player never got to a smash: keys don't move it), a set: heap
  367 MB through the match, then **443 → 460 MB as the match ends** (result screen / back to the menu, 216–219 s).
  That's a real growth, but at the match end, not at a smash: noted on B468.

## So
No first-use allocation to preload or cap. On iOS a "page reset" is either WebKit killing the tab over its memory
limit or a lost GPU device under memory pressure (index.html reloads on `DeviceLost`, B121). Either way it's the
standing footprint: the pack held in wasm memory (171 MB), the load high-water (~200 MB more, never given back) and
the GPU's textures and targets. The smash only happened to be what was on screen when the limit was hit. That's
B468's job (measure peak, stream or drop). With the friend's phone on a Mac's Web Inspector, `?log` shows
each growth.

## Not verified / not 1:1
- The crash itself: not reproduced. No iPhone or Safari here, and WebKit's tab or GPU memory can't be measured in
  Chromium. It may still be something WebKit-only (e.g. its GPU process allocating at a draw) that Chromium and
  Vulkan don't show.
- A human's smash in the browser: not reached (an unattended key script can't move the player to a lob). Its own
  code paths were exercised natively (marker forced on) and read (scatter math), not run in the browser.
- GPU memory in the browser: not measurable from the page. Native GPU-allocator numbers stand in for it.
- The `HST_FPS` logs that use `eprintln` (frame, stall, pipeline, asset lines) still print nothing in the browser.
  Only `info!` lines (the heap growth) reach the console.
