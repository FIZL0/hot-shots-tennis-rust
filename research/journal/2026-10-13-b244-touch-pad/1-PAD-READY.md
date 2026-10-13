# B244 iteration 1: pad in every menu, drives P1; crash not reproduced (needs a phone)

## Changes
- `web/touch.js`: the pad shows on a touch screen in menus too (it used to hide while `hstTouchMenu(true)`). It hides
  only while a real controller is connected, on the disc picker or on a boot error. In menus the stick is a fixed
  circle at the bottom left, centred, so a tap anywhere else stays a menu tap. In a match the stick floats as before.
- `play/controls.rs` `PadSlots::update`: in the browser the keyboard seat also takes a pad no other seat wants. A
  phone's menus are tapped (the keyboard's device), so 1P sat on `Seat::Keys` and the touch pad belonged to no seat:
  it now drives that seat. Native keeps the old rule (`update_with(.., false)`); unit test in `seated_pads`.
- `tools/web_touch.mjs`: `CHROME_FLAGS`, a stick-hold check that 1P moves (`8-p1-before/held`), an
  `HST_WEB_MATCH_SECS` play-on loop (double ✕ serves) and pad-state logging.

## Checked (headless chromium, 844×390 touch viewport; shots in `context/b244/`)
- run1/dbg (WebGL2): pad visible in the main menu; menus tapped through to a match; holding the stick moved 1P; ✕
  tosses and hits the serve; no panic.
- gpu2 (WebGPU on the desktop AMD GPU): 3 whole matches, back to the menu and rematch via the pad's ✕, no panic.

## Crash (`pkg-webgpu.js:2108:34 156458@wasm-function[156458]`)
Line 2108 of the build's `hst.js` is `new Error()` in the panic hook's stack capture: the frame is the panic, not the
cause. The message is the `console.error` line just before (`panicked at …`), which the page's error box already
shows (`index.html` `fail`). Not reproduced on desktop WebGPU (3 matches) or WebGL2. Needs the human: on the iPhone,
play a match on the WebGPU build until it crashes and copy the whole error text from the page ("The game stopped:
panicked at <file:line>: <message>"), or Safari Web Inspector's console. Then bisect B227/B228/B162/B196 from that.

## Not verified / not 1:1
- No real phone or iPhone; only emulated touch in headless chromium.
- The iOS WebGPU crash: not reproduced, not fixed.
- One long WebGL2 run (gl2) showed no pad on a result screen and later in a match; a run with logging on every
  show/hide change (dbg) never hid it and the pad was in every shot. Cause unknown (maybe the headless run).
- Layout: the L1 button overlaps the top-left of some menu titles ("Main Menu"); the menu stick circle can cover a
  bottom-left menu item on short screens.
- Pause/settings/mods/character-select were driven with taps plus the pad; not every screen of the mods store was
  stepped through with the pad alone.
