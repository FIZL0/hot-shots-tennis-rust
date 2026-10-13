# B95 — held keys must not repeat

**Finding.** Every keyboard read is already edge-based or on our own repeat timer: the match latches a shot/serve
only on `just_pressed` (`play.rs read_input`, `tick_input` sends it as one new bit), the pause menu repeats up/down
after 32 ticks then every 8 (`menu.rs`), the main menu after 0.4 s then every 0.1 s (`main_menu.rs Repeat`) — the
keyboard and the pad go through the same code. Bevy 0.19's `ButtonInput::press` ignores an OS auto-repeat
(`repeat: true` Pressed) while the key is down; winit's Wayland repeat sends Pressed only, no releases.

The one way a held key presses again: a release Bevy saw that the keyboard didn't (`KeyboardFocusLost` →
`release_all`, e.g. focus flickering away and back while J is held), after which the OS's repeat events count as a
fresh press, and keep the swing / menu OK firing at the OS repeat rate.

**Fix.** `controls.rs no_key_repeat` (PreUpdate, after `InputSystems`): a `just_pressed` that came from a repeat
event is cleared, so a held key presses once, like a held pad button; the menus' held-direction repeat stays their
own timer for both. Test `controls::tests::held_key_presses_once` (press, repeat, focus lost then repeats).

**Live check attempt.** Built a debug hst logging keyboard shot/serve presses and tried to hold J with `wtype` into
it; Hyprland's focus dispatch didn't move focus to the window, so the keys went to the user's focused foot terminal
(window "omarchy-desktop:agents": a held `j` with its OS repeats and a held Space, no Enter). Stopped there; no
further keystroke injection on the shared desktop.

## Not verified / not 1:1
- Not reproduced live: the user's exact repro (which platform/case made a held key repeat) is unknown and
  keystroke injection on the shared desktop isn't safe (see above). The fix covers the only path found in the code.
- Menu repeat timings (main menu 0.4 s / 0.1 s vs the pause menu's 32 / 8 ticks) unchanged: front-end menus need
  not match the original (PLAN.md), and both already treat keyboard and pad the same.
