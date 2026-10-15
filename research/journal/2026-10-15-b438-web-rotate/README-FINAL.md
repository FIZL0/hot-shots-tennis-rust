# B438 — web app glitches on landscape ↔ portrait

Runs: `context/b438/run*` (`tools/web_check.mjs`, new `WEB_TURNS=s,s,…`: a touch phone (390×844, DPR 3) turned at
those seconds, sizes of page / canvas box / backing store logged every second; `TURN_SHOTS=ms,…` screenshots that
long after each turn). Headless Chromium on the real GPU (`CHROME_FLAGS="--enable-features=Vulkan --use-angle=vulkan"`),
WebGL2 build, own port (`WEB_PORT=8843`) and a profile with the pack imported.

## Found on the way: the WebGL2 build quit at start
`2f258567` ("8bit graphics", today) took `Hdr` off the scene cameras, so the main texture is `Rgba8Unorm`, and Bevy
asks for an `Rgba8UnormSrgb` view format on it. That needs `DownlevelFlags::VIEW_FORMATS`, which wgpu's GL backend
never has: "Downlevel flags VIEW_FORMATS are required but not supported" on `main_texture_a`, then Bevy quits on the
Validation error (run2/run3: black screen). Every WebGL2 browser (the iPhone fallback, Firefox) was affected.
Fix in the patched `third_party/bevy_render` (`prepare_view_targets`): the extra view format is only asked for when
the adapter has `VIEW_FORMATS`. Nothing in Bevy or hst makes an sRGB view of the main texture, so nothing else changes;
WebGPU/Vulkan/Metal still get it.

## Rotation
With the game running (run4–run9), Chromium's turns re-lay out cleanly: main menu, Local, character select, Confirm
Settings, a match (`?play&stage=1`) in both orientations. Canvas box and backing store follow within ~150 ms
(the shot at +0/+150 ms already shows the new layout), HUD, menus, touch pad all placed for the new aspect, no errors.
So Chromium/Android-style resizes are fine; the glitch is in how the page sizes the canvas on iOS, which I can't run:

- `html, body` were pinned to `--app-w` (px, from `innerWidth`). iOS fires `orientationchange` (and sometimes
  `resize`) before `innerWidth/innerHeight` follow, so right after a turn to portrait the page was pinned 844 px wide
  in a 390 px screen. iOS Safari ignores `user-scalable=no`, zooms out to fit a document wider than the screen, and
  a zoomed-out page's `innerWidth` is the wider width, which `fit` then wrote back: the layout could stay wrong until
  the next turn. Chromium honours `user-scalable=no` and never zooms (checked: pinning 844 px in portrait leaves
  innerWidth/clientWidth 390), so it doesn't show there.
- Stale sizes were applied as they came; the settle timeouts were at 50/150/400/1000/2000 ms only.

Fix (`web/index.html`):
- `html, body` width is `100%` (the layout viewport), not the px var; the fixed canvas keeps `--app-w/-h`.
- viewport meta: `minimum-scale=1, maximum-scale=1` (iOS keeps these for its own zooms on a turn).
- `fit` takes the width no wider than `documentElement.clientWidth` (a zoomed-out `innerWidth` is wider).
- For 2 s after a turn on a touch screen, a size whose `innerWidth > innerHeight` disagrees with
  `screen.orientation` (or `window.orientation`) is skipped as stale; outside that window, on desktop, or in an iPad
  split view (taller window than screen) every size is taken as before.
- After a turn `fit` runs every animation frame for 2 s, plus timeouts up to 3 s (frames may stop on a hidden page).

Checked with a `WEB_PRELOAD` that makes `innerWidth/innerHeight` report the pre-turn size for 500 ms after a change
(iOS's lag): run11 (first version) caught a mixed 390×390 canvas (new clamped width + stale height passed a
`w === h` escape); fixed by testing the raw inner pair. run12: the old size is kept whole until the new one is real,
then the canvas follows within a frame (+594 ms / +784 ms shots correct). Desktop (run10, 1280-wide window): canvas
fills the page, menu as before.

## Not verified / not 1:1
- On a real phone: no iPhone/Android device here. The iOS zoom-to-fit feedback loop is the known WebKit behaviour
  (it ignores `user-scalable=no`), not seen; the lag was emulated, not measured. If the user's phone still glitches,
  a screen recording of the turn says which part (canvas size, zoom, or a screen's layout).
- Only the WebGL2 build was run (rebuilding `pkg-webgpu` wasn't needed for the page change; the bevy_render change
  leaves WebGPU's request unchanged).
- The canvas's backing store is CSS-px sized on the DPR-3 emulated phone (844×390), so text is soft; that's the
  game's render scale, untouched here.
- Not 1:1: n/a (browser shell).
