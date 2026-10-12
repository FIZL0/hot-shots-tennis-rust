# B109 — web app stuck loading on a first visit (iOS)

Runs: `context/b109/run*` (`tools/web_check.mjs`, headless Chromium; `console.txt`, shots).

B101's status line and B103's `move(dir, name)` were already in. What was left: the page awaited
several things with no limit, and one of them quit silently. Before the disc picker a first visit waits on
`navigator.gpu.requestAdapter()`, `import(hst.js)` and `navigator.storage.getDirectory()`. After the import it waits on
the wasm download/compile and Bevy's first frames. Any of these that never answers left "Loading…" up for good, and
before B101 the page was just black.

## Found
- **Bevy quits without a panic on a render error.** run1, headless Chromium on WebGPU: `Caught DeviceLost error` →
  `Quitting the application due to DeviceLost RenderError`, logged with `console.log` (tracing-wasm's `%cERROR`), so
  B101's console.error/panic hook never saw it and the page stayed on "Starting…" for good. A WebGPU device lost
  or failing at startup on iOS would hang the same way. Then a reload (or a later visit) can pick a working
  adapter, which fits "later loads worked".

## Fix (`web/index.html` only)
- Bevy's `%cERROR` lines are captured. "Quitting the application" before the first frame on WebGPU reloads the page
  with `?webgl2`; anywhere else the page shows "The game stopped: <first error>" with the Reload/WebGL2/ISO links.
- `requestAdapter()` gets 5 s, then counts as no adapter (WebGL2).
- Every boot step is named (graphics → loading the game → opening this browser's storage → download → compiling →
  reading the pack → starting). A step with no progress for 30 s says "No progress for 30 s: the browser may have
  stalled here…" and shows the links (Reload etc.) without stopping anything, so a slow step that does finish
  still goes on.
- `tools/web_check.mjs`: `WEB_PRELOAD=js` runs before the page (to make an API hang).

## Checked
- run1 (WebGPU, DeviceLost): visit 2 logs "WebGPU stopped before the first frame, reloading with WebGL2" →
  `hst: WebGL2` → the match draws (visit2-30s.png).
- run2: fresh profile, `requestAdapter` made to never resolve → WebGL2 after 5 s → disc picker → import → main menu
  (visit1-20s.png), and visit 2 too.
- run3: `getDirectory` made to never resolve → after 30 s "Opening this browser's storage… No progress for 30 s…"
  with the links (visit2-35s.png).
- WebKitGTK 2.52 MiniBrowser (OPFS features on), fresh profile, first visit: no `navigator.gpu` → WebGL2 → storage →
  disc picker shown (step log sent back to a local server from a scratch copy of the page).

## Not verified / not 1:1
- A real iPhone with cleared site data, through the import to the menu: no iOS device or Web Inspector here, so
  which step actually hung on the user's phone is unknown. The page now either recovers from a stall (WebGPU
  failure → WebGL2, adapter timeout) or names the stalled step on screen. If it still stalls, that message says where.
- WebKitGTK can't run the import (no sync access handles, see B103), and it has no WebGPU, so the WebKit WebGPU
  path and the post-import start weren't run in WebKit.
- The "Quitting the application" match depends on Bevy's log text. If a Bevy update rewords it, only the 30 s
  stall message is left.
- Not 1:1: n/a (browser shell).
