# B273: Renderer setting, and WebGL2's frame rate

## (1) Renderer setting: done
- Settings → Display (browser only, first row): **Renderer** Auto / WebGPU / WebGL2 (`options.rs` `Renderer`). It's
  saved as `renderer = auto|webgpu|webgl2` in settings.txt (`main_menu.rs` `Settings.5`, `RENDERERS`). On the web
  that file is already in localStorage (`./settings.txt`, `web.rs` store), so `web/index.html` reads the line straight
  from there before it picks `pkg`/`pkg-webgpu`. No extra mirror is needed. Native settings.txt doesn't get the line.
- Auto works as before: WebGPU if the browser has an adapter, otherwise WebGL2, and a WebGPU build that fails before
  its first frame retries as WebGL2. WebGL2 skips the adapter request. WebGPU never falls back to WebGL2 on its own
  (the user asked for it); the error page's Try WebGL2 link is still there. `?webgl2` in the URL still wins.
- A change saves, then calls `web::renderer` → the page's `hstRenderer(word)`. That reloads 1.5 s after the last
  change (so stepping through Auto → WebGPU → WebGL2 doesn't reload on each press) and drops `?webgl2`. Going back to
  the value the page started with cancels the reload.
- Checked in headless Chromium (`tools/web_check.mjs`, real GPU via `--use-angle=vulkan --enable-features=Vulkan`;
  run in `context/b273/sw2`): settings.txt has `renderer = webgl2` → `hst: WebGL2 (renderer setting: webgl2)`.
  Main → Settings → Display → ◀ on Renderer, then the page reloads by itself → `hst: WebGPU (renderer setting:
  webgpu)`, so the value is kept across the reload. The screenshot shows the row and its description fitting.

## (2) What caps WebGL2 at 60 on the phone: no cap in the build
Frame times measured with `tools/web_check.mjs` (frames2.json, summed by `research/b273_frames.py`) and CPU profiles
(new `WEB_CPUPROFILE=s:n` → `visit2.cpuprofile`, summed by bucket by `research/b273_profile.py`). Setup: a match
(`?play&stage=1`, 3 COM + 1P), headless Chromium on the desktop GPU (RX 9070 XT), `--disable-frame-rate-limit
--disable-gpu-vsync`, so frames run as fast as the page can make them:

| run | WebGL2 | WebGPU |
|---|---|---|
| 1280×720, uncapped | 241 fps (p95 4.4 ms) | 186 fps (p95 9.5 ms) |
| phone-like (`MOBILE=1` 844×390 touch, `THROTTLE=4`) | 47 fps | 30 fps |

- Bevy's winit loop on wasm is the same for both builds (ControlFlow::Wait + request_redraw → requestAnimationFrame),
  and `graphics::limit` already does nothing on wasm (the phone's first-visit `frame_limit = 60` isn't applied there).
  wgpu-hal's GL web present is a blit (or an sRGB triangle) into the canvas, with no fence or wait. Nothing paces
  WebGL2 differently.
- CPU per frame: WebGL2 spends ~10% of its time in GL calls plus ~15% in wgpu-core/hal. WebGPU spends ~18% in the
  browser's WebGPU JS calls (writeBuffer, createBindGroup, `decode`), plus `wgpu::backend::webgpu::map_wgt_limits` on
  every frame. On Chromium, WebGL2 is the cheaper build.
- So the 60 the user saw comes from the phone's browser or GPU, not from the build. The likeliest explanation is
  WebKit's WebGL (every call goes to the GPU process): a frame just over 8.3 ms shows every other vsync, i.e. exactly
  60. Safari's "Prefer Page Rendering Updates near 60fps" flag would cap both builds, so it can't explain a gap
  between them. Can't measure this without the phone; it's under PLAN.md *Needs the human*. With the setting and
  Show FPS (Settings → Display), the user can now compare both builds on the phone themselves.

## Not verified / not 1:1
- WebGL2 at 120 fps on a 120 Hz phone: there's no phone here. Only an emulated touch viewport with 4× CPU throttle on
  the desktop GPU, which says nothing about the phone's GPU or WebKit's WebGL cost.
- A refresh-paced 120 Hz desktop run: the runs were headless and uncapped (throughput 241 fps ≫ 120). No headful
  browser window was opened on the user's 240 Hz desktop during an unattended run.
- Renderer menu (front-end): not compared with the original. It has no such setting.
