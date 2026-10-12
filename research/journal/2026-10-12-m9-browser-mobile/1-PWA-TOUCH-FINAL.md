# M9 — browser and mobile play (PWA, touch controls)

Builds on M13a's browser build (`web/`, `tools/web.sh`, `crates/hst/src/web.rs`).

## Added
- **Touch controls** `web/touch.js`: an HTML overlay that shows up as a gamepad in the browser's standard layout,
  added to what `navigator.getGamepads()` returns (index 16). gilrs' wasm backend polls `getGamepads()` and reads the
  objects by property (`index`, `id`, `mapping`, `buttons[i].pressed/value`, `axes`), so the game sees an ordinary
  controller: menus, controller assignment, matches and `controls.txt` rebinding all work with no Rust input code
  (`play.rs read_input` unchanged). Layout: a floating stick where the left thumb lands (left 45 % of the screen,
  full tilt at the base's edge), ✕ ○ □ △ in a diamond bottom right, L1/R1 in the top corners, SELECT/START top centre;
  sized in `min(7vw, 12.5vh)` units, inside the safe-area insets; light vibration on press. Shown on touch screens
  (`pointer: coarse`, or after any touch; `?touch` forces it), hidden while a real controller is connected (the virtual
  pad leaves the list then) and on the import screen. Portrait on a touch screen shows "Turn your device sideways".
- **Audio unlock**: `touch.js` wraps `AudioContext` and resumes every context the game made on the next
  pointerup/touchend/keydown (cpal's web backend creates them suspended until a gesture).
- **Fullscreen + landscape**: the first tap on a touch screen asks for fullscreen and `screen.orientation.lock
  ('landscape')` (Android Chrome; skipped when already running installed).
- **Installable + offline**: `web/manifest.webmanifest` (display fullscreen, orientation landscape, icons 192/512/SVG
  drawn by us: `web/icon.svg` → PNGs with rsvg-convert, nothing from the disc), Apple meta tags + 180 px touch icon,
  `web/sw.js` (network first, the cache when offline; caches the page, scripts and the wasm, never the disc pack,
  which stays in OPFS).
- **Settings persist in the browser**: `crate::store` (main.rs) = `std::fs` natively, `web::read_to_string/write`
  (localStorage under the file's path, `./settings.txt`, `./controls.txt`) in the browser; settings.txt and
  controls.txt's four call sites go through it.
- **Phone speed**: the first visit on a touch screen seeds settings.txt with the existing Low preset (render scale
  67 %, low shadows; changeable in Settings → Graphics). `tools/web.sh` runs `wasm-opt -O2 --strip-debug` when
  binaryen is installed: `hst_bg.wasm` 126 MB → 59 MB (15 MB gzipped).
- **Main menu in the browser** (was M13a's open item): `previews.rs`' character loader thread is now a `step` closure;
  natively the same thread loops it, in the browser a `thread_local` pump runs one load per frame (Update, before
  `receive`).
- **Fix: the browser build rendered black** since P17b: `gs.wgsl`'s `@interpolate(linear)` fog has no WebGL2
  equivalent (naga: "doesn't support NOPERSPECTIVE_QUALIFIER"), every GS pipeline failed. On wasm `gs.rs` sets
  `GS_NO_LINEAR` on both stages: the vertex passes (F·w, w) perspective-correct and the fragment divides, which is F
  interpolated linearly on screen (Σbᵢwᵢ⁻¹·Fᵢwᵢ / Σbᵢwᵢ⁻¹·wᵢ = ΣbᵢFᵢ). Native shader unchanged.
- `tools/web_touch.mjs` (new) and `HST_WEB_PORT`/`HST_CDP_PORT` in both web scripts (side-by-side runs).

## Verified (headless chromium 152, SwiftShader WebGL2)
`tools/web_check.mjs` imported the ISO into a fresh profile (`context/m9/run1`), then `tools/web_touch.mjs
context/m9/run1` with an 844×390 landscape phone viewport, touch only:
- No query → main menu with the controls drawn (`1-menu.png`); `getGamepads()` lists the touch pad; settings.txt in
  localStorage holds the Low preset.
- Taps on ✕ go Main → Local → … → Character Selection with live 3D previews (`2-cross3.png`); a stick drag down gives
  axes [0, 1] and moves the cursor Ashley → Suzuki; release gives [0, 0] (`3-stick.png`).
- Eight more ✕ taps start a singles match on the clay court, played with taps (COM–1P points in `6-match-later.png`),
  rendered with fog (the WebGL2 fix).
- `Page.getAppManifest`: no errors; `Page.getInstallabilityErrors`: none; service worker activated, cache holds
  `/`, scripts, manifest, icons, `pkg/hst.js`, `pkg/hst_bg.wasm`.
- Web server killed, page reopened: the service worker served everything and the main menu came up from OPFS
  (`7-offline.png`).
- No panics/exceptions/ERRORs in the console (only Bevy's software-adapter and "CommandQueue" warnings, as M13a).
- `tools/check.sh`: 380 tests pass (native).

## Not verified / not 1:1
- **Real phones**: no iPhone/Android device here; only Chrome's mobile emulation with touch events. Frame rate on a
  phone GPU, iOS Safari's WebGL2/OPFS (`createSyncAccessHandle` in the import worker needs Safari 15.2+), Add to Home
  Screen on iOS and the 171 MB OPFS pack within iOS storage quotas are untested.
- **Audio**: the unlock code ran but headless chromium has no audio output to listen to; not heard.
- **Fullscreen/orientation lock**: needs a real user gesture on a device; headless can't show it.
- **Touch layout over the HUD**: the face buttons sit over the bottom-right score panel (translucent, still readable);
  the HUD isn't moved for the controls.
- **Real controller hides the overlay**: logic only (no pad in headless chromium).
- The preview loader in the browser runs one character load (~130 ms natively, more in wasm) inside a frame: a hitch
  when hovering a new card.
- Desktop non-touch browsers: controls hidden by `pointer: coarse` logic, not screenshotted.
- Online play in the browser: M3 / M2f (not part of this task).
