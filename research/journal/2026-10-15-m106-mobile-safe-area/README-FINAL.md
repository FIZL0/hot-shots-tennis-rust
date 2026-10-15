# M106 — mobile screen fit (safe area, aspect override, no Quit)

## What changed

- `play/widescreen.rs`: the view (what `hud_gamma` draws the 3D picture and the UI into) now fits inside the window's
  safe area: Auto = the whole safe rect, a fixed ratio = the largest rect of that ratio, centred in the safe rect. The
  insets live in `SAFE` as shares of the window size, so the physical-px callers (`hud_gamma`) and the logical-px ones
  (menus, pointer, chat) agree. `follow_safe_area` reads them twice a second on iOS (`ios::safe_area`) and Android
  (`android::safe_area`). `HST_SAFE=l,t,r,b` (logical px) fakes them anywhere, for desktop `--shot`s. Elsewhere
  (desktop, browser) the insets stay zero, so nothing changes there.
- Since the FPS counter is a UI node inside the view, it's inside the safe area now too (it was cut off on iOS at Auto).
- `play/touch_pad.rs`: takes its insets from `widescreen::safe_insets` and stops reading them itself. Its layout is
  unchanged: it's still laid out on the whole window on its own camera (M102), so neither the safe area's view nor
  the Aspect Ratio moves or resizes it. The "aspect changes mess the pad up" bug was the one M102 had already fixed
  (the pad used to be drawn into the 16:9 view). This task confirmed the fix on Android and on desktop.
- Default Aspect Ratio on the phone apps is now Auto (it was 16:9, a workaround for the notch). The browser on a
  phone keeps 16:9. Existing installs keep the `aspect` value already saved in their settings.txt.
- Main menu Quit row removed on iOS and Android (as on web). Nothing else on mobile quits: the pause screen's
  "Return to the Menu" goes back to the menu, and match results go back to the menu.
- Android `HstActivity.onCreate`: `layoutInDisplayCutoutMode` set to ALWAYS (API 30+) / SHORT_EDGES (28–29), so the
  window reaches under the cutout on every Android, as Android 15+ already forces at targetSdk 36. So one inset path
  serves every version, and the emulator (API 34, which letterboxed the cutout before) exercises it.

## Checks

- Unit test `views_in_the_safe_area` (iPhone 15 Pro landscape insets 59/0/59/21 at Auto and every fixed ratio; a
  one-sided Android cutout); existing widescreen/touch_pad/home tests pass.
- Desktop with `HST_TOUCH=1 HST_SAFE=59,0,59,21` in an 852×393 window (iPhone 15 Pro landscape in points), main
  menu and match at Auto / 16:9 / 4:3 / 21:9 (`context/m106/run2`, `run3`). UI and FPS counter stay inside the safe
  rect and the fixed-ratio rect, and the pad is pixel-for-pixel in the same place at every aspect.
- Android emulator `hst34` (API 34, x86_64, SwiftShader, 2400×1080 landscape, "tall" cutout overlay), own instance on
  port 5584: window 914×411 pt (now under the cutout), logged safe area [48, 0, 48, 0] (cutout left, nav bar right).
  Main menu at Auto and 4:3 (`context/m106/emu`): UI inside the safe area, no Quit row, FPS visible, pad identical in
  both. At 4:3 a tap on the Mods row opened Mods, so taps still map into the smaller view.
- iOS: `tools/ios-build.sh` cross-build compiles cleanly (no warnings in the touched files).

## Not verified / not 1:1

- No real notched iPhone or Android phone here: the iPhone was checked through the desktop `HST_SAFE` fake with
  iPhone 15 Pro insets (the iOS inset read is the same `safeAreaInsets` call the pad has used since M91a1, not run here).
  Android was checked on the emulator only.
- A match on the emulator wasn't screenshotted (SwiftShader takes about 8 min to load one). The match HUD was checked
  in the desktop fake only.
- The strips outside the safe area are black (like the aspect bars) instead of showing the 3D court under the notch.
  The picture and HUD share one view in `hud_gamma`, so splitting them would be a bigger change. It's a design choice,
  not something the original has (the PS2 has no notch).
- Front-end layout is ours (not 1:1 by design). Seen in passing and not changed: on the Mods screen the FPS counter
  overlaps the bottom hint line.
