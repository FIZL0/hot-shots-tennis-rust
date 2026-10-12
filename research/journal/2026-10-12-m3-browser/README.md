# M3 — the browser build, with online play

The browser runs the whole app (menus, character select, match) on WebGPU or WebGL2. Sound starts on the first tap or
key. The disc pack comes from OPFS (M13a). Online play goes over iroh's relays and works with desktop players.

## What changed

- **Two builds** (`tools/web.sh`): `web/pkg` is WebGL2 and `web/pkg-webgpu` is WebGPU (cargo feature `webgpu` =
  `bevy/webgpu`; Bevy's `webgpu` overrides `webgl`, so one wasm can't do both). `index.html` loads the WebGPU build when
  `navigator.gpu.requestAdapter()` gives an adapter, else WebGL2; `?webgl2` forces WebGL2.
  - wasm-opt `-O2 --strip-debug` with the feature flags rustc emits: 126 MB → ~59 MB per build.
- **Audio unlock**: a classic script in `index.html` wraps `AudioContext` (cpal's WebAudio backend makes them) and
  resumes every suspended one on the first pointer, touch or key event.
- **Menus in the browser**: the character-select preview loader ran on a thread. On wasm the same loop runs one load
  per frame from `receive` (`previews.rs`). The PNG card cache is off there (no files): the cards draw the live model.
- **WebGL2 shader**: GLSL ES 3.0 has no `noperspective`, so on WebGL2 the fog varying (`gs.wgsl`) uses the default
  interpolation (`NO_ARRAY_TEXTURES_SUPPORT` marks the WebGL2 build).
- **Online** (`net.rs`):
  - The transport code builds for both targets. Its timers and tasks go through `n0_future`, which is tokio natively
    and the page's event loop in the browser.
  - On wasm, `Net::start` spawns the network task instead of a thread. `endpoint` binds relay-only (no DHT or mDNS
    lookup), and relay Off is refused there.
  - The DHT and mDNS parts stay native-only. Find answers with an empty list, Quick play hosts, and announce does
    nothing.
  - `iroh`, `iroh-tickets`, `n0-future` and `tokio` (sync, macros) are now all-target deps.
- **Join by Code in the browser**: the browser's clipboard can only be read asynchronously. `web::paste` (the page's
  `paste` event) and `?code=<code>` set the text that Fx::Paste falls back to (`online.rs`), and the "no code" status
  says so.
- **GitHub Pages**: `.github/workflows/pages.yml` builds with `tools/web.sh` (wasm-bindgen-cli at Cargo.lock's version,
  binaryen 123) and deploys `web/`. Nothing from the disc is in it.
- **`tools/web_check.mjs`** gained `WEB_INPUT` (clicks and keys at set seconds), `PROFILE`, `CHROME_FLAGS`,
  `SHOT_EVERY`, cache disabled, and an audio-state/peak log.

## Verified (headless Chromium, captures in `context/m3/`)

- **Menus and character select**: main menu → character select with the live 3D preview, on WebGL2/SwiftShader
  (`run3`).
- **WebGPU on the real GPU**: the AMD RDNA4 GPU through `--enable-features=Vulkan --use-angle=vulkan` (`run5`). The
  menus advance and the music plays.
- **Audio unlock**: the AudioContext is suspended before the first click and running after it, with a non-zero sample
  peak.
- **Online, browser ↔ desktop over n0's relay**:
  - Native `hst net host 1 --play <iso>`; the browser opened with `?code=<ticket>` and went Online → Join by Code →
    singles → assign → select → confirm.
  - The host saw the join, a round trip of 57 ms and an input delay of 3 ticks.
  - The match ran 5400+ ticks with identical lockstep state hashes on both sides (ticks 0, 1800, 3600 and 5400), and
    no desync (`net3`). The browser drew the match (`net3/b/visit2-120s.png`).
- **Native**: `tools/check.sh` passes. The net tests still pass on the n0_future timers.

## Not verified / not 1:1

- **SwiftShader WebGPU**: Chromium's software WebGPU loses the device ("A valid external Instance reference no longer
  exists"). A bare WebGPU test page did the same, so it's SwiftShader's limit. The real GPU works, and machines without
  WebGPU get the WebGL2 build.
- **Other browsers and devices**: Firefox, Safari and phones are untested. Only headless Chromium on Linux was
  available.
- **No lobby list or LAN in the browser**: a browser can't send UDP, so there's no DHT Find, no mDNS and no relay-off
  play. Find shows no lobbies and Quick play hosts. This is a platform limit; M2k is the WebRTC idea for it.
- **Two networks**: the online test ran on one machine, through n0's public relay. The two-network test still needs
  the human (PLAN *Needs the human*, M2f).
- **Two browsers**: no browser ↔ browser match was run, only browser ↔ desktop. It's the same code path on both
  sides, just untested.
- **Preview cards**: the browser has no PNG cache for them (no files), so it always draws the live model.
- **WebGL2 fog**: interpolated perspective-correct instead of linear. It looks the same; rendering only has to look
  right.
- **Pages workflow**: never run (no push from here). Its steps match `tools/web.sh`, which was run locally.
- **Settings**: `online.txt` and `settings.txt` aren't saved in the browser (no files), so the defaults come back each
  visit.
