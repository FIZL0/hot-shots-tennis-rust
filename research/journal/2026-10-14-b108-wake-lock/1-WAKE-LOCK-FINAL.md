# B108 — screen stays awake while the game is open

User 2026-10-13 widened it: no auto-sleep at all while the page is open (not only with a controller), so the lock
isn't tied to pads or matches.

`web/index.html` (the boot script): `navigator.wakeLock.request('screen')` on load, again on `visibilitychange`
back to visible (browsers release it when the tab hides), and on `pointerdown`/`keydown`/`gamepadconnected` in case
a browser (Safari) only grants it after user activation. One lock at a time (`wake` holds the pending/held request,
cleared on release or refusal). No `web.rs` change needed. Native Android keep-screen-on: dropped (no native mobile,
user 2026-10-12).

Checked: the logic against a stubbed `navigator.wakeLock` in node (one request, re-taken after release); headless
Chromium grants the lock on page load without a gesture (`held`).

## Not verified / not 1:1
- On a real phone (iPhone Safari, Android Chrome): no device here. If Safari still dims, check the console for a
  refused request; a tap re-requests.
- Not a gameplay port: nothing in the original to match.
