# B314 — HUD and menus out of the iPhone's top safe area

Portrait on an iPhone home-screen app (`viewport-fit=cover`, `black-translucent`): the page runs under the Dynamic
Island / status bar, which iOS blurs over, so the far side's score and the menus' top bars were unreadable.

## Fix
- `web/index.html`: `--safe-top: env(safe-area-inset-top, 0px)`; the canvas sits at `top: var(--safe-top)` and is
  `--app-h − --safe-top` high. Bevy's window is the canvas, so `widescreen::view` is that shorter rect and the HUD,
  menus and 3D all lay out below the island at its aspect. 0 in landscape, desktop and Safari with its own bar.
  Bevy's pointer/touch positions are canvas-relative and `web::page_size` reads the canvas rect, so taps line up.
- The Mods screen's Add-mod file input (`hstModPicker`, % of the window) is placed in the canvas's rect now.
- `web/touch.js`: L1/R1/Select/Start keep `safe-area-inset-top` clear; the far (half-turned) 1v1 pad keeps the
  top inset clear at its bottom edge (`--safe-b`), the near one the home indicator as before.

## Checked
Headless Chromium, 393×852 mobile, `Emulation.setSafeAreaInsetsOverride {top: 59, bottom: 34}`, real disc profile
(`context/b314/run2/1-menu.png`): canvas rect y 59..852, main menu's title bar starts under the 59 px strip, the
page's shoulder buttons below it too. Scratch page: modzip rect = 59 + 50 % of 793 px. Inset 0 → canvas full page.

## Not verified / not 1:1
- Not seen on a real iPhone (no device here); Chromium's safe-area override stands in for it.
- The in-match far-side score wasn't screenshotted (not driven into a match on the web build); it is inside the same
  canvas, so it moves down with everything else.
- Only the top inset is kept out of the view; landscape's side insets (island at the side) and the bottom home
  indicator still draw under, as before (not asked for).

## Tests
`tools/check.sh`: everything passes except `play::netplay::tests::matches::rollback_survives_a_tab_out` ("the
guest's picture flipped to the shared camera for […] rounds"), failing twice with different counts. B314's Rust
change is one doc comment, so the failure came with the branch base (likely B315's camera change); filed as its own
task. B314 ticked by hand (`tools/ctx.py tick`) instead of through `done.sh`, which won't tick on a red suite.
