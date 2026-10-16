# B453 — iPhone Safari: freezes in the menus and on loading screens

User report (2026-10-15, build 54125cc): the browser build on the iPhone installs fine but often freezes in the
menus and on loading screens. Task: reproduce in the browser build with stall logging, find what blocks the frame.

**Status: READY — needs a log from the iPhone** (plan/HUMAN.md). The cause on the phone can't be reproduced here
(no iPhone, no WebGPU in any WebKit on this machine); what blocks the frame in desktop WebKit and throttled Chromium
is measured below, and the build now carries a frame log a phone can save without a desktop inspector.

## What changed

- **Frame log in the browser** (`?fps`, or the site's new "Frame log for bug reports" box under Play, which sets
  localStorage `hst-fps` for an installed home-screen app that has no address bar):
  - `web.rs` turns on the game's own `HST_FPS` log (fps, `game stall`, `render stall`, pipelines; `stall.rs`).
  - `stall.rs` `diag!`: those lines (stall.rs, main.rs fps/frame split/assets, graphics.rs) now go through
    `bevy::log::info!` on wasm, so they reach the browser console (before: `eprintln!`, lost on wasm).
  - `web/index.html`: keeps every console line (ring of 20000, ms since load, UA header), adds the page's view of
    the main thread — `raf gap N ms` (two animation frames >100 ms apart), `blocked N ms` (a 50 ms timer >250 ms
    late), `page hidden/visible` — and a fixed "Save log" button that downloads `hst-log.txt`.
  - Checked in headless Chromium: no button by default, button with `?fps`, button with localStorage `hst-fps`=1.
- **`tools/web_webkit.mjs`**: the browser build in the system WebKitGTK MiniBrowser (JavaScriptCore + WebKit's
  wasm tiers, Safari's engine) with the same probe, console streamed back to `OUT/console.txt`, scripted keys.
  WebKitGTK's OPFS has no sync access handles ("NotSupportedError: Backend does not support this operation"), so it
  writes the native `~/.local/share/hst/disc.pack` into OPFS first via `createWritable`. Playwright's WebKit build
  doesn't run here (missing libicudata.so.66).

## Measurements (captures in `context/b453/`)

Runs: `run1` (Chromium, THROTTLE=4, MOBILE=1), `run2` (menus → match, webgl2), `run3`; WebKit `wk/run1–4` (run4 under
`systemd-run --user --scope -p CPUQuota=60%`). Machine load ~26 from other agents, so Chromium numbers are noisy.

What blocks a frame, biggest first:

1. **Synchronous render-pipeline creation** on the main thread (WebGL2 compile + link; one per frame thanks to P47's
   compile budget): 100–235 ms each in desktop WebKit, up to 704 ms at 60 % CPU, ~200–320 ms in throttled Chromium.
   They dominate the loading screen (a match start creates ~60) and the first entry into each screen.
2. **`main_menu::setup`**, one system in P50's sliced start: 222–452 ms in WebKit, 782 ms at 60 % CPU, 655–1337 ms
   in throttled Chromium. One long boot frame.
3. **wasm compile/instantiate**: ~0.5–1.9 s block at canvas creation.
4. **Card bakes** (`previews.rs`) on first opens of a menu: 40–65 ms frames in WebKit, 150–400 ms throttled Chromium.

Otherwise the menus run ~59 fps in desktop WebKit (13–17 ms frames once the cover is down), no multi-second freeze.
Throttled Chromium's menu start took 166–253 frames (9.7–34 s) because SwiftShader frames cost ~200 ms and the jobs
queue gets 25 ms of each.

Headless Chromium's WebGPU dies at the first frame ("Caught DeviceLost … A valid external Instance reference no longer
exists" / "OperationError: Instance dropped in popErrorScope") and falls back to WebGL2 — the same error as B452
(Android); noted there.

## Hypothesis for the phone (unverified)

The iPhone runs the WebGPU build. Safari's `createRenderPipeline` is synchronous to the page but the Metal pipeline is
compiled in its GPU process; the next call that has to wait on that process (e.g. `getCurrentTexture`) stalls. That
would show in the log as `raf gap` / `blocked` lines with no long game frame (`game stall`) next to them, clustered
around `pipeline at …` lines — on loading screens and first screen entries, matching the report. If so, the fixes are
fewer distinct pipelines before the cover lifts / spreading them over more frames, or async creation
(`createRenderPipelineAsync`, which wgpu's API doesn't expose today). If instead the log shows long `game stall`
frames, it's items 2/4 above on a slower CPU.

## Not verified / not 1:1

- **The freeze on the iPhone itself**: not reproduced — no iPhone here; WebKitGTK has no WebGPU (the phone's path);
  Chromium's headless WebGPU dies at the first frame. Needs the phone's `hst-log.txt` (plan/HUMAN.md).
- **Phone timings**: WebKit ran on a desktop CPU/GPU (and a 60 % CPU quota); Metal shader-compile cost on the phone is
  unknown, so the ms above are only relative.
- **"Save log" on iOS**: the download (`<a download>` of a Blob) is untested on Safari/iOS and in a home-screen app.
- No gameplay touched; nothing here needs to match the original.
