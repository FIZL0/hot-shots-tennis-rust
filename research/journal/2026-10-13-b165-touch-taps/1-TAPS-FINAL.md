# B165 — touch buttons miss taps

## Causes found (web/touch.js)
- **Taps between two polls.** The game sees the pad only through `getGamepads()`, which gilrs reads once per frame
  (it re-polls until a poll brings no change). A tap whose `pointerdown` and `pointerup` both land between two frames —
  a quick tap, or a busy phone frame that queues both events (more likely with the stick moving, which keeps the page
  busy with `pointermove`s) — was never seen down. Shots are `just_pressed` and latched by `play.rs` until used, so
  seen once is enough: now a button stays down until a poll has seen it *and* a later animation frame has begun.
- **Second finger on a button.** Up/down was per button, not per pointer: a second finger on ✕ re-pressed nothing and
  the first finger's lift let ✕ go while the second still held it. Now each button keeps the set of pointers on it.
- **Edge-swipe guard read the wrong touch.** It used `touches[0]` (the first touch still held, i.e. the stick thumb),
  so with the thumb within 20 px of the left edge every other finger's `touchstart` was `preventDefault`ed too. Now
  `changedTouches[0]` (the touch that just landed).
- **Browser gestures with two fingers.** iOS Safari can start a pinch/double-tap zoom out of two fingers on the controls
  despite `touch-action: none` (and `user-scalable=no`, which iOS ignores), cancelling both pointers. The overlay now
  `preventDefault`s its own `touchstart`/`touchmove` (non-passive) and `gesturestart`.
- **Just outside a button.** Each button's hit area extends 0.2 u past its drawn edge (`::after`); the face buttons'
  extended circles still don't overlap (centres 1.7 u apart, radius 0.825 u).

## Check
`node tools/web_touch_taps.mjs` (no build, disc or server): headless Chromium with touch, touch.js on a blank page,
a stand-in game polling every 200 ms. With the stick held: a 20 ms ✕ tap between polls is seen, the stick stays tilted,
two fingers keep ✕ down until both lift, a touch 3 px past ✕'s edge hits it. All pass; the previous touch.js fails the
first, second-finger and edge checks.
Found on the way: the first tap's fullscreen request moves the buttons in headless Chromium (the check stubs it).

## Not verified / not 1:1
- Not tried on a real phone (none attached): the iOS gesture cancel and the edge-guard fix are from reading the code
  and the browsers' documented behaviour, not reproduced. The headless check can't simulate iOS gesture recognition.
- Two taps of the same button less than ~2 frames apart merge into one press (`ponytail:` note in touch.js).
- Front-end/controls are ours (not from the original), so no 1:1 comparison applies.
