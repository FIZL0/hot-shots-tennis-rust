# B324 — Settings says what WebGL2 reaches on this device

## What
- `web/index.html`: while the page is still light (before the wasm loads), it measures the display's refresh rate from
  40 requestAnimationFrame gaps (median, snapped to a usual rate within 3 %) → `window.hstRefresh`, logged as
  `hst: display ≈ … Hz`.
- `web.rs`: `frame_rate` (Last) keeps the best whole second's frame rate, the menus' and a match's separately (a
  phone's GPU can keep up in the menus and not in a match; a second crossing in/out of a match counts for neither).
  `webgl2_reach()` hands back (fps, in a match, refresh), and `None` on the WebGPU build.
- `options.rs`: on the wasm build, the Renderer and Show FPS rows' side panel note gives the result of `webgl2_reach`: below 90 % of the
  display's rate it says "WebGL2 reaches up to N fps in a match/the menus on this device, below the display's H Hz.
  WebGPU (where the browser has it) or the desktop app (download the desktop build) gets the full refresh rate."
  Otherwise it says "the display's full H Hz". A match's figure is used once a match has run. WebGPU and native show
  nothing extra. Unit test `webgl2_reach_words`.
- I also fixed the Renderer row's Choices chips, which said Off/On (from B273); they now say Auto/WebGPU/WebGL2.

## Check
Headless Chromium (`tools/web_check.mjs`, WebGL2 build, SwiftShader, THROTTLE=4, runs in `context/b324/run6`,
`run7`): rAF measured 59.9 Hz → 60. Settings → Display → Renderer showed "WebGL2 reaches up to 9 fps in the menus on
this device, below the display's 60 Hz. …" at 1280×720. MOBILE=1 with rotation shows it laid out in both landscape and
portrait (panel below the list in portrait).

Note: visit 1's ISO import on port 8795 never started (empty status for 10 min). On 8765 it did. I didn't find out why.

## Not verified / not 1:1
- The user's 120 Hz phone: no phone here. Safari's rAF rate before the game loads is assumed to be the display's
  (e.g. 120 on ProMotion). With Low Power Mode / Safari's 60 fps preference it measures 60. Then the note says
  "full 60 Hz", which is true for that page.
- No uncapped WebGL2 run at a refresh-paced rate (headless SwiftShader only gets ~10–15 fps). The "full H Hz"
  wording is covered by the unit test only.
- A WebGPU build session wasn't run. It shows nothing because `webgl2_reach` returns `None` under the `webgpu` feature
  (compile-time).
- Front-end text: the original has no such setting, so nothing to compare against.
