# B118 — 1v1 on this phone (READY: needs a phone check)

## What was built
- **Entry**: on touch pages the main menu gets "1v1 on this phone" (`main_menu.rs` `DUO_ROW`, `duo_start`/`duo_sync`).
  It turns on the far pad (`web::touch_pads(duo, buzz)` → `hstTouchPads`), both pick at the character select
  (far ✕ takes 2P), and the match launches with `widescreen::SIDEWAYS` set (local 1P vs 2P singles).
- **Portrait match**: `widescreen::SIDEWAYS` turns the HUD a quarter left (`in_view_at`, `hud_gamma.wgsl` uv), so
  each player's score panel sits at their own end (1P at the bottom); 3D framing uses `scene_aspect`. The camera is
  the own-camera (`own_camera.rs`, B218) on 1P with `cam_of = [0, 0]`, so both sticks are screen-relative to the one
  picture. Native `--sideways` shows it for shots.
- **Touch pads** (`web/touch.js`): two pads (gamepad indices 16 near, 17 far), far one rotated 180° at the top, sized
  `min(11vw, 6.5vh)`, independent multi-touch per pad; portrait orientation lock in fullscreen while duo.
- **Mobile settings tab** (touch pages only): Vibration (buzz on each on-screen press, `navigator.vibrate(8)`).
- Fixed on the way: the settings tab strip drew with `Anchor::Span`, which stretched the 6th tab (Mobile) to the
  window's right edge; tabs now sit in the centred screen's units from the view's top-left (checked 16:9, 600×800,
  1400×450, with and without the Mobile tab).

## Checks
- `tools/check.sh -p hst`: 305 passed (new `phone_duo`, `every_page` over the shown tabs, `sideways_hud`).
- `node tools/web_touch_taps.mjs`: all PASS, incl. "far stick reads screen right/down, near stick left, both ✕ down at
  once".
- `tools/web_duo.mjs` (new; real disc profile, WebGL2 build): entry → both pick → match; both sticks held at once
  moved both players; sideways HUD with panels at each end; long run (`HST_WEB_MATCH_SECS=1000`, cut by its 1150 s timeout) played 660 s of the match with both pads mashing ✕ and reached 4–4, Deuce; the end/result screen wasn't reached.
- Shots in `context/b118/` (sideways.png, upright.png, landscape.png, mobile_native*.png, sound_*.png, web/, long/).

## Not verified / not 1:1
- Not tried on a real phone: two people, real multi-touch, a portrait match to the end. Only headless Chromium
  emulation (CDP touch events). Needs the human.
- A full match to its end on the two-pad setup: the headless run reached 4–4 before its timeout (headless frame rate is slow); not seen to the result screen.
- The turned score panels sit under the pads' face buttons at each end (readable, partly covered).
- Menus can't be used held upright (the "Turn your device sideways" cover; portrait menus are B112): the entry and
  the select are used sideways, then the phone is turned flat/upright for the match.
- Character select and other menus read the right way up only for the near player; the far pad overlays the
  select's top while duo.
- Seat labels at the select show "1P Controller 2 / 2P Controller 1" (labels follow pad entity order); the match
  mapping is right (near = 1P).
- 3D call pop-ups (scene-camera ones in `popups.rs`) and player markers are not turned with the HUD.
- The far pad uses the near side's safe-area insets.
- The camera keeps 1P's end at the bottom (own camera, presentation only), not the original's shared match camera;
  sim and match timing untouched.
- Mobile tab has only Vibration.
- Menu taps act as P1.
- The tab-strip fix also changes the 5-tab strip on very wide windows (the last tab no longer stretches).
