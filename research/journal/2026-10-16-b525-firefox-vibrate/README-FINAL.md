# B525 — Firefox for Android: no vibration

## Why
Not our call path. MDN's browser-compat-data (`api/Navigator.json`, `vibrate`) for `firefox_android` since 79:
"Vibration is disabled. If the window is visible, then `navigator.vibrate()` returns `true`, but no vibration takes
place (regardless of hardware support)" — Mozilla bug 1591113 / 1653318 (kept off over abuse concerns). Desktop
Firefox removed the method in 129. No other web API reaches the phone's motor (Gamepad `vibrationActuator` is only for
real pads), so Firefox can't be made to buzz.

## Change
- `web/touch.js`: Android + `Gecko/<digit>` in the user agent (Fenix, Focus, Iceraven, Mull; Chrome/Samsung say
  "like Gecko") → `vibrates` false, no call; `hstVibration()` returns `firefox`.
- `options.rs` `vibration_about`: `firefox` → "Firefox for Android doesn't let web pages vibrate, so this does nothing
  here. Chrome on Android buzzes." (also moved B329's doc line back onto `vibration_about`; it had drifted onto
  `aspect_about`). `web.rs` doc lists the new value.
- Checked the UA test in node against Fenix, Chrome Android, Samsung Internet and desktop Firefox UAs.

## Not verified / not 1:1
- Not tried on a real phone with Firefox (no device here; the shared emulator has no Firefox). Relies on MDN's
  compat note plus the UA test above.
- Front-end text only; nothing to match against the original.
