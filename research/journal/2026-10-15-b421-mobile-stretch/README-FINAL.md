# B421 — Mobile Stretch aspect ratio

User (2026-10-15): on iOS and Android, an Aspect Ratio choice that fills the whole screen, ignoring M106's rule that
fits the view into the safe area (notch, Dynamic Island, cutout, home indicator).

## What changed

- `play/widescreen.rs`: `ASPECTS` gets a sixth entry, "Stretch" (`STRETCH` = 5). `view_in` returns the whole window
  for it, the safe-area insets ignored. `aspect_choices()` says how many entries the setting offers: all six on the
  phone apps (or with `HST_SAFE`, for desktop shots), the first five elsewhere, where Auto already fills the window.
- Nothing is distorted despite the name: the 3D camera takes the view's aspect and the HUD keeps its 4:3 screen, as
  at Auto. HUD and menus can reach under the island at Stretch — that's the point of the option.
- `main_menu/options.rs`: the Aspect row cycles and lists `aspect_choices()` entries; its help text names Stretch on
  the phone apps. `main_menu.rs` `Settings::parse` only accepts an offered choice (a desktop given a phone's
  settings.txt with Stretch falls back to the default).
- `play/touch_pad.rs`: doc only. The pad's buttons keep clear of the safe area at Stretch too (a thumb there hits the
  island or the system's home gesture); the pad never followed the Aspect Ratio anyway (M106).
- `graphics.rs`: no change needed (the Original mode still forces 4:3).

## Verified

- `views_in_the_safe_area`: Stretch = the whole window with iPhone 15 Pro insets, landscape and portrait.
- Settings test: choices count, Stretch saved and read back where offered (run with and without `HST_SAFE`).
- Desktop shots with `HST_TOUCH=1 HST_SAFE=59,0,59,21` in an 852×393 window (`context/b421/run1`): main menu at Auto
  (bars over the insets) vs Stretch (edge to edge), and a `--play` match at Stretch: picture fills the window, pad in
  the same place.

## Not verified / not 1:1

- No real iPhone or Android phone here, and the iOS/Android builds weren't run: checked through the desktop `HST_SAFE`
  fake (same code path in `widescreen`; the only platform difference is where the insets come from).
- Front-end setting, remaster-only: nothing to match against the original.
