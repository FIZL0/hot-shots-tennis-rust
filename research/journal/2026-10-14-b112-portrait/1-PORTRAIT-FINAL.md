# B112 — play in portrait

Most of it was already there:
- Camera: every match/intro/popup/ceremony camera sets `fov = 2·atan(tan·max(0.75, 1/aspect))`, so a window narrower
  than 4:3 keeps the original's full width and widens vertically: the whole court shows upright (shot
  `context/b112/portrait_match.png`, 405×860).
- HUD/menus: B232's `widescreen::vplace` letterboxes the 4:3 screen mid-height, score panels pinned top/bottom
  (`context/b112/portrait_menu.png`).
- Live rotation: the canvas is CSS 100%×100%, winit's ResizeObserver resizes the Bevy window, the layout systems
  run every frame — no reload. Native desktop windows of any shape already worked the same way.

What blocked phones was the page itself, now changed:
- `web/touch.js`: the "Turn your device sideways" overlay is gone, fullscreen no longer locks to landscape;
  upright, the touch pad is sized by the width (`--u: min(12vw, 7vh)`) so it fits in the band under the picture, the
  stick zone is the lower half (checked in headless Chromium at 390×844 and 844×390, landscape unchanged).
- `web/manifest.webmanifest`: `orientation: any`.

## Not verified / not 1:1
- Not tried on a real phone (no device here): rotating an installed/fullscreen PWA on iOS/Android, safe-area insets
  upright. Checked with a desktop window and headless Chromium only.
- Upright, the bottom-left score panel can sit under the stick zone (as it does in landscape); the stick still works
  there since it's transparent.
