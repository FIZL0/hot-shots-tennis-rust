# B247 — browser on mobile: controllers connected after load

## Findings
- The Rust side already handles hotplug. gilrs' wasm backend polls `navigator.getGamepads()` every frame and turns a
  new index into `Connected`. `controls::PadSlots` fills free slots as pads arrive, and the menus read every pad.
- On a touch screen, `web/touch.js` adds its virtual pad to `getGamepads()` and hides it only on its own
  `requestAnimationFrame` tick. Bevy's web loop doesn't run on rAF, so the game can poll between the real pad
  appearing and that tick. The real pad then takes 2P's slot (the touch pad holds 1P's). When the touch pad goes, it
  stays in 2P (`pad_slots_survive_unplug`), and 1P has no controller in the match.
- The page had no `gamepadconnected` listener. The game only polls.

## Change (`web/touch.js`)
- `getGamepads()` never lists the touch pad next to a connected real pad. A hotplugged pad arrives in the same poll
  that drops the touch pad, so it fills 1P's slot.
- `gamepadconnected` / `gamepaddisconnected` listeners run the show/hide update at once, and wake the engines that
  only watch for hotplug when a page listens.

## Check
Headless Chromium page (scratchpad) that stubs `navigator.getGamepads` under touch.js with `?touch`:
touch pad only → real pad plugged in (same poll: only the real pad listed; overlay hidden on the event) → unplugged
(touch pad back).

## Not verified / not 1:1
- Not tried on a real iPhone or Android phone with a Bluetooth controller (no device here). This is a guess that the
  missing listener was what blocked detection on some engine. The slot race is confirmed by reading the code.
  Human check: open opentennis.dev on a phone, start a match on touch, connect a controller, press a button. It
  should drive 1P without a reload.
- Not 1:1: none (browser shell, not gameplay).
