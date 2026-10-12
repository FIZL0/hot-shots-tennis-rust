# B101 — web build black screen, audio above real time

Captures and screenshots: `context/b101/run*` (`tools/web_check.mjs`, headless Chromium; `console.txt`, frame shots).

## Where it stopped
- **Black until ~30–40 s.** Not a hang: the loading cover (`mode.rs` `Compiling`) waits until the PipelineCache has
  no waiting pipelines, capped at 600 frames. On WebGL2 there is no compute, and Bevy's sparse buffer update *compute*
  pipeline stays queued for good, so every loading screen ran to the cap: 600 frames at the low frame rates seen while
  shaders compile (SwiftShader ~15 fps) is 30–40 s of black (run2: "loading screen down after 600 frames").
  Fix: count only waiting *render* pipelines. After the fix: WebGL2 menu at 2–4 s, "after 5 frames" (run4, run5);
  WebGPU on the real GPU (Vulkan ANGLE), fresh load + reload, menu at ~4 s (run6).
- **Silent page.** Nothing showed what was loading or why it failed. `index.html` now has a status line
  (checking graphics → downloading the game N MB → reading the disc pack → starting), hidden by `web::shown` once the
  game has drawn 3 frames, and turns into "The game stopped: …" with the panic message / uncaught error / rejected
  promise, plus Reload · Try WebGL2 · Pick the ISO again links. Seen working on a real failure (missing pkg-webgpu:
  "Failed to fetch dynamically imported module").
- **Stale pair after a deploy.** Pages serves everything `max-age=600`; the service worker's network-first fetch
  could still take a cached `hst.js` with a newer `hst_bg.wasm` (or the reverse), which fails at init. `sw.js` now
  fetches sub-resources with `cache: 'no-cache'` (revalidate, 304 when unchanged).

## Audio above 100% speed
The sim clock isn't the cause (wasm has no frame limiter; the browser paces rAF). The music and sounds run at the
rate cpal's WebAudio output pulls them: two buffers (46 ms) played in turn, each started at cpal's own time cursor.
After a main-thread stall (loading, compile, hidden tab) the cursor is behind `currentTime`, both buffers start
"now" and overlap, so the mix is pulled at ~2× until the cursor catches up. Measured with `WEB_STALL=8:3000`
(3 s busy loop): rate 1.7–1.8 for ~3 s after the stall before (run7, run8), 1.0 after (run9). Fix in `touch.js`
(which already wraps AudioContext): each buffer source starts at max(asked, now, end of the previous one).
The harness's AudioContext Proxy dropped `new.target`, which bypassed the subclass in run8; now `Reflect.construct`.

## Not verified / not 1:1
- Firefox: not installed on this machine, so not run.
- A real headed desktop Chrome and the deployed GitHub Pages site: only headless Chromium against a local server
  (SwiftShader WebGL2 and Vulkan WebGPU) was run.
- Whether the old build also held the cover on WebGPU (compute exists there) wasn't checked; the fix is the same.
- The wasm is streamed through a counting `Response`, which may lose Chrome's compiled-code cache for it
  (reload compile time not measured).
- Audio overlap on a hidden tab (rAF stops, audio keeps running) only tested as a main-thread stall.
