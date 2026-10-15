# B379 — 1v1 on this phone taken out (user 2026-10-14)

B118's two-players-on-one-phone mode is gone from every build:

- **Menu entry**: `home.rs` `Row::Duo` ("1v1 Here", its preview words, facts and device tiles); `main_menu.rs`
  `Menu::duo`, `duo_start`/`duo_sync`, `Menu::turned` and the menu root's half turn (`Turned`, `turn`), the
  `phone_duo` test; `pointer.rs`'s turned-select hit test, drag direction and far-player tap device.
- **Split/turned layout**: `widescreen::SIDEWAYS`/`turned` and the turned `in_view`/`view_size` (and the
  `sideways_hud` test); `hud_gamma.rs` no longer swaps the HUD target's size and `hud_gamma.wgsl` samples the HUD
  straight; the native `--sideways` flag. `scene_aspect` stays (now the view's aspect, same as `aspect`) so its many
  callers needn't change.
- **Own camera**: the local two-player branch of `own_camera::link` (1P's own camera, `cam_of = [0, 0]`). The own
  camera is online-only again; `step` still steps without a lockstep (the B315 test drives it that way).
- **Touch controls** (`web/touch.js`): the far pad (gamepad 17, turned 180°), its CSS (`.far`, `.duo`), the portrait
  orientation lock, `hstTouchPads`. One pad (`touch = side(16)`); the vibration setting now reaches the page through
  `hstTouchBuzz(on)` (`web::touch_buzz`). `tools/web_duo.mjs` deleted; the duo checks in `tools/web_touch_taps.mjs`
  dropped; `tools/web_vibrate.mjs` calls `hstTouchBuzz`.
- Kept: the Mobile settings tab and Vibration (B118/B329), not part of the 1v1.
- B308/B309 (1v1 split screen / sideways HUD fixes) noted obsolete; `plan/HUMAN.md`'s B310 ask now says "play a match".

## Checks
- `tools/check.sh -p hst`: 356 passed, 1 failed: `rollback_survives_a_tab_out` (picture flips to the shared camera
  for a few rounds after the tab-out). It fails the same way on the code before this change (3 of 3 runs with
  B379's edits reverted, real-time sleeps under load from the parallel run); with the change it passed 1 of 3.
  Not touched here.
- `node tools/web_touch_taps.mjs`, `node tools/web_vibrate.mjs`: all PASS.
- `cargo check -p hst --target wasm32-unknown-unknown`: builds, no warnings in the touched files.

## Not verified / not 1:1
- Not tried on a real phone or in the full browser build with a disc (headless touch scripts and the wasm
  type-check only); no PCSX2 needed (front-end removal, no gameplay touched).
