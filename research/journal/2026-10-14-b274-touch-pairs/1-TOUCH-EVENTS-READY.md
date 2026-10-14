# B274 — two touch controls at once still block on phones

Unattended run: no phone and nobody to ask which pair blocked or what it looked like.

## What was checked
- **Chain from finger to game.** `web/touch.js` → `navigator.getGamepads()` (virtual pad, index 16) → gilrs' wasm
  backend (diffs every button and axis on each poll; polls until nothing changes, so a frame sees the stick and all
  buttons from the same `getGamepads()`) → Bevy `Gamepad` → `play.rs` (stick + `just_pressed` shots per slot) and
  the menus (`main_menu.rs`: stick/d-pad held + buttons `just_pressed`, all from the same pad). Nothing there drops
  one input while another is held.
- **Desktop Chromium with real two-finger CDP touches** (each `Input.dispatchTouchEvent` carries every finger
  still down, the way a phone sends them): the old pointer-event code already worked here for every pair. So the
  blocking is in how phones deliver *pointer* events for a second finger, which headless Chromium can't reproduce.

## Change (web/touch.js)
- Fingers now come only through **touch events**, each by its own `identifier`, only from `changedTouches`. A
  finger keeps the control it landed on (its `target`) until it lifts. Pointer events remain for a mouse or pen
  (`?touch` on a desktop) and skip `pointerType === 'touch'`. iOS has always delivered touch events per finger;
  its pointer events for multi-touch are newer and are the likeliest place for a lift/cancel to go missing.
- **A finger that's gone lets go.** Every `touchstart`/`touchend`/`touchcancel` lists the fingers still down
  (`touches`). Any control held by a finger not on that list is released. A lost end used to keep a button held,
  or kept the stick taken so it ignored every new thumb (`if (stick) return`), until the page reloaded.
- Hiding the pad (a real controller connected) now also lets the stick go. Before, a held stick stayed taken.
- The B165 rules still hold: per-finger sets per button, a tap held until one poll has seen it.

## Check
`node tools/web_touch_taps.mjs` (extended): B165's checks, plus, with full multi-touch, 3 rounds of:
- stick + each of the 8 buttons, held together; the stick moves while the button is held; the button lifts while
  the stick stays tilted; the button tapped with the stick held;
- every ordered pair of buttons (56): both held, the first lifts while the second stays down, the first tapped
  beside the held second;
- a `touchCancel` lets the stick and ✕ go, and both work again after it;
- a touch event whose `touches` no longer lists the held fingers releases their controls, and the stick takes a
  new thumb after that.
All pass. Against the previous touch.js everything passes except the lost-end check (a held ✕ and the stick stay
down for good), so the Chromium pairs alone wouldn't have caught the phones' bug.
The check stops the 200 ms stand-in poller for this part and polls itself once per check: a poll between a tap and
the check would otherwise see the tap first and let it go by the check's poll (B165's rule working, not a miss).

## Not verified / not 1:1
- **Not tried on an iPhone or an Android phone** (none attached; the task's done condition needs both). The
  cause is inferred: the whole chain works with real multi-touch in desktop Chromium, so the phone-only failure
  must be in the phones' pointer-event delivery, which the switch to touch events avoids. Whoever has the phones:
  play a match and a menu, then hold the stick with each button and every two buttons, held and tapped, many times.
  If one still blocks, note which pair and what it looked like (held stick stops, press lost, page zooms/selects).
- In a match, pressing a shot locks the player into the swing the way the original does: the stick then aims
  rather than moves. That's gameplay, not the touch pad, and a real controller does the same.
- The touch pad is our front-end (not in the original), so there's no 1:1 comparison.
