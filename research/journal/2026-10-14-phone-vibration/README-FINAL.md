# B329 — phone vibration never felt

The buzz was `navigator.vibrate?.(8)` in `web/touch.js` on each on-screen button press (Settings → Mobile →
Vibration, sent by `web::touch_pads`). Here's why it was never felt:

- **iOS Safari has no `navigator.vibrate`** (no iOS version has had the Vibration API), so the `?.` call did
  nothing. Since iOS 18, flipping a `<input type="checkbox" switch>` from a tap plays the system's light haptic tick
  (the trick used by the `ios-haptics` library). On an iPhone, touch.js now keeps a hidden switch inside a label
  and clicks the label on each press. The label stops the click from bubbling, so the page never sees it.
- **Android Chrome**: the API is there. But 8 ms is shorter than many phone motors (LRAs) need to spin up, so the
  pulse can't be felt. It's now 15 ms. Chrome also skips `vibrate` while the ringer is on Silent (its Android
  vibration service checks the ringer mode). It only works after the page's first tap (sticky user activation),
  which the menus always provide before a match.
- The setting now says which case applies (`hstVibration()` → `web::vibration()` → `options.rs`
  `vibration_about`): Android shows "not while the ringer is on Silent", iPhone shows "Safari has no vibration:
  iOS 18's haptic tick instead, nothing on older iOS", and any other browser shows "can't vibrate here".

Check: `node tools/web_vibrate.mjs` (headless Chromium, touch.js alone). A ✕ tap calls `vibrate(15)` with the
setting on and doesn't with it off. With an iPhone user agent and no `vibrate`, it flips the switch instead, and the
click never reaches the page. All 7 checks pass.

## Not verified / not 1:1
- Real phones: there's no Android or iOS device here. Not checked: that 15 ms is felt on Android Chrome, that the
  ringer-Silent rule matches the user's phone, or that iOS 18 Safari ticks when the click comes from a `touchstart`
  handler. The trick is documented for click handlers; if WebKit doesn't count touchstart as a user gesture for
  it, an iPhone still gets nothing, and the fix is to tick from `touchend` instead.
- No original-game comparison: the press buzz is our own phone addition (front-end), not something the PS2 game
  has.
