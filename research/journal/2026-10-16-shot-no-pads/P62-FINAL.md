# P62 — `--shot` / `HST_MENU` runs ignore pads and the real keyboard

`hst-base/src/seats.rs` `ignore_input`, called from `main.rs` when `--shot` or `HST_MENU` is set:

- gilrs' plugin is left out (desktop/web), so no `Gamepad` entity exists at all; `seats::readable` also answers
  `false` for every pad (covers the touch pad / Android pads, which don't come through gilrs).
- `Messages<KeyboardInput>` is cleared in PreUpdate before `InputSystems`, so real key presses never reach
  `ButtonInput<KeyCode>` or text entry. `HST_KEYS` presses `ButtonInput` itself after `InputSystems`, so it still drives.

Check (`context/P62/run1/`): my own vpad (HST_PCSX2=1) pressing ✕ every 0.5 s through an `HST_MENU=assign` shot.
Gate forced off: the log shows both vpads (mine and another slot's) connected and 4P becomes "Controller 1". Gate on:
no gamepad line in the log, the frame matches the pad-free shot except for the wagging hand. `HST_KEYS="4:ArrowDown
5:ArrowDown"` still moves the hand.

Pitfall: a temporary `false && a || b` is `b` (precedence), so the first "gate off" run was really gated.

## Not verified / not 1:1
- Real keyboard presses were not injected into the shot window (it's unfocused and on another workspace; no safe
  way to type into it without touching the user's desktop). The message drop is the input path bevy uses, so it
  should hold, but only `HST_KEYS` was exercised.
- Mouse/touch input isn't gated (not in the task).
