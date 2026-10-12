# M13a — disc import in the browser (s2, from s3's handoff)

Builds on HANDOFF-M13a.md (Iso over any `Source`, `iso::mount`, `write_pack_to`, `web.rs` `import_disc`/`run`).

## Added
- `web/index.html`: OPFS `disc.pack` there → read it into a `Uint8Array`, `init()`, `run(bytes)` (winit's
  control-flow exception swallowed). Not there → a file input; the pick goes to the worker, progress and errors are
  shown, then "the ISO is no longer needed; you can delete it" and the game starts. `?forget` removes the pack.
- `web/worker.js`: `disc.part` sync access handle → `import_disc` → flush/close → `move('disc.pack')` (copy+remove
  fallback).
- `tools/web.sh` (cargo wasm32 release into `target-wasm/` + `wasm-bindgen --target web` into `web/pkg/`, ignored);
  `tools/web_check.mjs` (headless chromium over CDP: visit 1 picks the ISO, visit 2 with the same profile picks
  nothing; screenshots every 10 s + console into the out dir).
- wasm fixes on the `?play` path: `SystemTime::now` (weather seed skip, rain seed) through `web-time` (std's panics on
  wasm32-unknown-unknown; native = std's); `graphics::limit` off in the browser (no `Instant`/sleep; the browser paces
  frames); `gs.wgsl` no longer calls `textureNumLevels` (not in WebGL2 GLSL: every GS pipeline failed and Bevy quit) —
  the level is `max(…, 0)` and the sampler clamps to the last level, the same result as the old clamp to
  `levels − 1` (the level is a whole number, the view has exactly the uploaded levels).

## Verified
- `node tools/web_check.mjs "Hot Shots Tennis (USA).iso" context/m13a/run2 "play&singles&stage=1" 60` (chromium 152
  headless, WebGL2 on SwiftShader): visit 1 imported, OPFS `disc.pack` 170 820 213 bytes (same size as the native
  pack); visit 2 with no file picked started the singles match: Carol vs the COM on the clay court, HUD, umpire, crowd
  (`context/m13a/run2/visit2-60s.png`); no errors in the console (only Bevy's software-adapter warnings and
  "CommandQueue has un-applied commands" warnings).
- `crates/hst-data/tests/pack.rs` (s3): `write_pack_to` into memory = the file's bytes; a mounted pack opens by path.
- `tools/check.sh` passes.

## Not verified / not 1:1
- The main menu in the browser (no `?play`): `play/main_menu/previews.rs` spawns threads (panics on wasm) — M3.
- Audio in the browser (needs a gesture to unlock — M3); WebGPU (built for WebGL2 only); real GPUs (SwiftShader only);
  frame rate (software rendering, not measured).
- A wrong disc picked in the browser: the error path (the `exe` check's message as the worker's error) wasn't run.
- Memory: the pack is held in JS (171 MB ArrayBuffer) and once in wasm during `run`; the JS copy isn't freed
  explicitly. `hst_bg.wasm` is 125 MB unoptimized (no `wasm-opt` yet).
- The "CommandQueue has un-applied commands being dropped" warnings: also native? Not checked.
- "Offer to forget the ISO" read as: the browser never keeps it; the page says it can go, and `?forget` re-imports.
