# P54 — WebGL2: MSAA off, greyed in Options

## What the user's log showed
`hst-log (10).txt` (WebGL2, ANGLE/RX 9070 XT, Auto → Default, msaa 4x): the menus compiled their mesh pipelines at
samples 1 (6–56 s), then the first match built them all again at samples 4 between 93 and 106 s: 24 opaque,
30 alpha-blend, 16 pbr alpha-blend, 7 premultiplied, 1 pbr opaque, 1 blit (79 pipelines). WebGL2 compiles on the main
thread, so the match sat stalled through that.

The 26 `prepass_pipeline`s are the shadow pass's (`GsMaterial::enable_prepass`, PREPASS_PIPELINE + DEPTH_PREPASS,
always samples 1): no camera has `DepthPrepass`/`NormalPrepass`/motion vectors, so there's no main-view prepass to
drop. They go with Shadows Off, which is already a setting (and Auto's last steps). Nothing else doubles variants:
tonemapping/postprocessing are single pipelines.

## Change
- `graphics::webgl2()`: the WebGL2 build (`wasm32` without `webgpu`), or `HST_WEBGL2=1` natively.
- `Graphics::msaa_drawn()`: 1 on WebGL2, else the setting. The camera `msaa` system and the Detailed FPS readout
  use it; `cheaper` (Auto) skips its MSAA step there (it would be a no-op step).
- The stored setting is left alone, so the preset names still match, and WebGPU (same saved settings after a
  renderer switch) and native draw it as chosen.
- Options: `locked(Msaa)` on WebGL2 — the row greys, shows "Off", can't be edited, and its note says why and points
  to Display → Renderer (WebGPU) or the desktop app.

## Checked
- Native `HST_WEBGL2=1 HST_FPS=1 … --play` (context/p54/run1.log): every mesh pipeline at samples 1, prepass 19 (shadows).
  Same without `HST_WEBGL2` (run2.log): mesh pipelines at samples 4 — native keeps MSAA.
- `cargo check --target wasm32-unknown-unknown -p hst`: no new warnings. `check.sh -p hst-base graphics`, `-p hst options`.

## Not verified / not 1:1
- Not run in a real browser (no WebGL2 browser session in the unattended run); the native WebGL2 emulation
  (`HST_WEBGL2`) went through the same `webgl2()` path, and the wasm build compiles.
- The greyed row wasn't screenshotted; it's the existing `locked` look (Upscaled Textures in the browser).
- Not 1:1 with the original: n/a (remaster-only renderer setting).
