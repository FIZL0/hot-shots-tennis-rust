# P57 — menu start freezes on the web

The user's hst-log 9 (Chrome 152, WebGL2, RX 9070 XT, dpr 1.6, 2400x1350 css) showed three problems:
- the first frame uploaded 14 images (40.7 MB): render stall 233 ms, then a 958 ms rAF gap;
- `main_menu::setup` ran 410 ms in one frame;
- the menu art build cost ~35–45 ms a frame from 1.9 s to 4.2 s ("started in 95 frames, 4380 ms").

## What was slow

- **First frame upload:** four window-sized render targets in `hud_gamma.rs`. `Image::new_target_texture` fills
  their CPU data with zeros, and Bevy uploads that data (at 4K that's four screens' worth), and again after every
  resize.
- **`setup`** (native breakdown): reading the sheets from disc took ~16 ms. `textures::prefetch_tim2` took ~25 ms,
  decoding every sheet even when no replacement pack is loaded. `menu_art::image` took ~50–75 ms, decoding ~50 disc
  sheets plus the walls' grid and `Steps::new`, all on the main thread. Reading the mods list took ~14 ms.
- **On the web, replacements were on in the menu:** `textures::init` ran at startup whenever "upscaled textures"
  was on. `textures::set` already turns them off in the browser for every match (B227), but the menu ran before any
  match. So the menu decoded every sheet for a pack look-up that can never match anything (64 ms + 44 ms of
  `read_sheets` in headless Chromium).
- **Art build:** at 2048² with K=4, the upscale took ~70% of each sheet's rebuild.

## Changes

- `hst-base/src/hud_gamma.rs`: the targets have no CPU data (`data: None`), so nothing is uploaded; wgpu creates
  them cleared. `copy_on_resize: false`: with no data, Bevy copies the old texture into the resized one, which
  needs `COPY_SRC` and failed WebGL2 validation ("copy_image_on_resize", found by the first headless run). A
  resized target used to come back zero-filled too, so nothing changes on screen.
- `hst-base/src/textures.rs`: `init` does nothing on wasm32, like `set`. `prefetch_tim2` returns early when
  replacements are off.
- `hst-play/src/play/menu_art.rs`: when building is deferred, `image()` sends every sheet that would decode on the
  main thread to the jobs: our rebuilt sheets (`Steps::new` now runs in their first step), walls and plain disc
  sheets. The upscale is rewritten: it caches the quad's weights and range per run of output pixels over the same
  four disc pixels, and a clear, flat quad gives transparent. It is about 3× faster and bit-identical to the old
  one: same output on word, option_00, characterselect2_00, confirmation_03 and title_mainmenu, and the old code is
  kept as `plain_upscale` under the test `upscale_matches_the_plain_one`. Unpremultiply skips alpha 0 (exact).
- `hst/src/play/main_menu.rs`: `setup` is split into three chained Startup systems, so the staged start runs one
  per frame: `read_sheets` (disc sheets and art handles), `read_text` (text tables, sounds, grades, roster) and
  `setup` (the UI).

## Numbers

Native (1280x720, context/p57/run1–3):
- before: setup 163 ms in one frame; "started in 117 frames, 259 ms"; 17.3 MB of images at 0 s.
- after: no start system over 50 ms except `previews::start_baker` (58 ms, the native-only sweep of stale files;
  not on the web, left alone); "started in 5–7 frames, ~160 ms"; 2.9 MB at 0 s.
- The menu shot is pixel-identical to the old build apart from the version text. Confirm, character select and a
  `--stage 1 --play` match shot look right. No wgpu errors in the logs.

Web, headless Chromium + SwiftShader WebGL2 (`tools/web_check.mjs`, context/p57/web2–4):
- before the `textures::init` fix: `read_sheets` 127–131 ms (timed inside: disc reads 22 ms, prefetch 64 ms,
  `image` 44 ms).
- after: no start system over 50 ms; "started in 23 frames, 1318 ms". In the user's log it was 95 frames, 4380 ms.
  During the art build `update` stays at ~2 ms; frames are 50–67 ms throughout, which is SwiftShader's software
  rendering.
- the first-frame images: 12, 17.5 MB (the render targets are gone from it).
- the menu draws correctly (visit1-10s.png).

## Not verified / not 1:1

- **Not re-measured on the user's machine** (real GPU, 4K canvas): headless SwiftShader is the only browser here.
  The first-frame saving (~33 MB at the user's size) is inferred from the targets' sizes; here the targets don't
  show in the first frame's upload any more.
- **No A/B of an old wasm build under SwiftShader**: the browser "before" numbers are the user's log, so they come
  from a different machine.
- **`previews::start_baker`** (58 ms natively, the native-only stale-file sweep) is unchanged.
- **The jobs' 25 ms per-frame budget on wasm is unchanged.** The build is now cheaper and runs off the start
  systems, but on a slow phone each frame can still spend up to that budget on the art while the cover is up.
- **Pre-existing, not P57:** the 2P card says "Connect a controller to join to join" (also in the old build).
- Menus don't have to match the original (PLAN), so the menu wasn't compared against PCSX2. PCSX2 wasn't used.
