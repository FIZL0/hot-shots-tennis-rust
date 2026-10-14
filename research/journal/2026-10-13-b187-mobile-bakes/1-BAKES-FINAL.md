# 1 — measure, then shrink/defer the bakes (FINAL)

## Method
No phone: headless Chromium via `tools/web_check.mjs` (WebGL2 build, `MOBILE=1` = 844×390 touch viewport,
new `THROTTLE=n` = CDP CPU throttling n× on visit 2, console lines stamped with ms since load). Clean profile with the
disc pack imported and no cached cards; input `Enter ×3` → Singles character select. Temporary `Instant` timers
around the baker's steps (removed before commit). Build: release + wasm-bindgen + `wasm-opt -O2` (66.6 MB wasm).
Host load from other agents' builds (loadavg up to 30) makes 6× runs noisy; only runs at low load are quoted.

## Before (HEAD)
- Entering the select: **683 ms** freeze at 6× (quiet host; 0.7–1.8 s in other runs, 1.6 s at 4×, 417 ms unthrottled),
  on every page load, cached cards or not: the preview pipelines compile on the first preview's frame.
- Each bake (384×480 card + 320² face) at 6×, all on the main thread: model load 48–65 ms (first 130), straight-alpha
  21 + 11 ms, PNG encode 25 + 18 ms (wasm's IO pool runs in-frame), icons 7–18 ms → one 117–233 ms frame per bake,
  ~1 bake/s, 14 characters × 10 costumes = 140 bakes ≈ 2+ minutes of hitching; select median frame 50 ms (menus 33).

## Changes (`play/main_menu/previews.rs`)
1. **Sized to the device** (`sizes()`): native unchanged (384×480, face 320). In the browser the scale is the page's
   shorter side / 448, rounded up, 1–3×: a phone (390 lines) bakes 128×160 + face 128 (9× fewer pixels to draw, read back,
   convert, encode); 2× = 256×320 + 224. Own cache folder (`…-r8-160`), so desktop cards don't change and a bake still
   runs once per device (the OPFS cache is hit on later visits: no bake, no frame >100 ms in the select, run new4v3).
2. **Pipelines warm under the boot loading screen**: the first job may start on any screen; if every card is cached,
   one throw-away drawing (`Job.warm`) runs at the menu's start. "Loading screen down after 9–10 frames" (was 5).
3. **Lazy in the browser**: the background queue is costume 0 only (the icons); other costumes bake when hovered
   (the existing hovered-first path). Natively all costumes still bake in the background.
4. A job starts the frame after its model loads, not on the load's frame (in the browser a load runs inside a frame).

## After
- Select entry freeze: **gone** (6×: first select frame 33 ms; pipelines compiled behind the loading screen).
- Per bake at 6×: save 4–6 ms, straight <4 ms, load still ~50 ms (35 at 4×) → bake frames 83 ms at 4×, 117–133 ms at 6×.
- 14 icon bakes, then idle (was 140). 4× (closest to a mid phone): one frame over 100 ms (183) in the whole select
  visit; 6×: ~8 frames of 117–133 ms plus one 216 ms around the 4th–6th bake; done ~10 s after entering.
- Later visits (cards cached): no frame over 100 ms in the select at 4×.
- An instrumented run at loadavg 17 showed render+idle gaps of 100–200 ms on frames with no bake at all: at 6× under
  load the compositor/WebGL, not the baker, bounds the frame rate.

## Other mobile wins looked at (not changed)
- wasm: 138 MB → 66.6 MB after `wasm-opt -O2`. No `[profile.release]` (opt-level 3, no LTO, 16 CGUs): a
  `[profile.web]` with `lto = "thin"`/`codegen-units = 1`/`opt-level = "s"` for `tools/web.sh` should cut size and
  startup download; not tried (a 2+ min build per try on a loaded host, and it changes every web build).
- Textures/mipmaps, shadow map size, MSAA, startup asset load: not measured here; the select's cost was the bakes.

## Not verified / not 1:1
- **No real phone**: only desktop Chromium with CPU throttling (4×, 6×) and a phone viewport; GPU speed of a phone is
  not modelled (SwiftShader/host GPU). WebGPU build not measured, WebGL2 only.
- Host load from parallel agents made timings noisy (±50 ms per frame at 6×); numbers above are from the quietest runs.
- The ~100 ms target: met at 4× and on cached visits; at 6× the first visit still has 117–133 ms frames while the 14 icons
  bake (the in-frame character load, ~50 ms at 6×, can't move off the main thread without wasm threads) and one
  unexplained 183–216 ms frame.
- P23's render scale / shadow quality aren't used to size bakes: the page size alone picks the scale.
- Web bakes are sized when the page starts: a browser window grown later shows the smaller (softer) cards until a
  fresh device cache (another size folder) is made on a later visit at that size.
- wasm LTO/opt-level profile not tried (above).
- Front-end only (cards); no gameplay or sim RNG touched, nothing to compare with the original.
