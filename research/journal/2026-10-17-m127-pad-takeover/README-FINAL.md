# M127 — pad takeover prompt

Our own front-end feature (no original to match).

## What
`crates/hst/src/play/main_menu/takeover.rs`, state in `Menu::takeover`:
- `watch` (before `step`): with `p1 == Keys` and seat 1P Keys/empty, on a local menu screen (Main, Settings tabs,
  Controls, Mode, Mods, Extras) with no other popup, any *button* (`Gamepad::get_just_pressed`, d-pad included,
  sticks never) on a readable pad holding no seat opens the prompt with the pad's gilrs name.
- The opening frame swallows every press (`age == 0`), so the opening button doesn't pass through or answer it.
- While up, `Menu::step` / `Menu::point` go to `takeover_step` / `takeover_point`: only the asked pad moves/chooses
  (○ = No), the keyboard's Back (Esc/Backspace/Cut key) dismisses, clicks/taps on Yes/No choose, a tap outside = No.
- Yes: `p1` and seat 0 → the pad (the legend already follows the last-used device, now that pad), toast
  "1P: <name> · keyboard is free" 2.5 s; keyboard is in no seat, so it joins on the assignment like any device.
- No / Esc / 4 s: pad added to `declined`; dropped from it the frame it's not connected (replug asks again).
- Closed unanswered if its pad goes or the screen stops qualifying.
- Not on the splash (its press takes 1P), Assign/Chars/Confirm/Lobby (presses join there), online, paused
  match menus (`paused`, `net_pause`), nor a touch page (B247 already hands 1P to a pad's first press there).
- Legend: asked pad Move/Choose/Keep keyboard; keyboard Keep keyboard.
- `HST_MENU=takeover` / `takeover-toast` for shots. Shots at 16:9 and 1:1: `context/M127/` (centre-pinned box,
  keeps its shape).

Tests: `takeover::tests` (yes/no/esc/timeout/replug, screen gating, taps + hit rects), main_menu suite passes.

## Not verified / not 1:1
- Not tried with a real gamepad press end to end (`--shot` runs read no pads, P62; no physical pad here). The
  logic is unit-tested on `Menu`; the `watch` system's `get_just_pressed` gate isn't exercised by a test.
- Touch pages keep B247's direct takeover instead of the prompt (a deliberate choice: there 1P is the screen,
  not a keyboard); the prompt itself is tappable/clickable anywhere it appears.
- Prompt wording has no em dash (the menu font lacks it): "Controller detected" / "Use <pad> for 1P?".
- Nothing to match in the original (front-end, ours).
