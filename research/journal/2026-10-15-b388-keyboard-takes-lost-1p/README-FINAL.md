# B388 — the keyboard takes over 1P when 1P's controller is gone

Problem: once seats are assigned (main menu `Menu::seats`, match `PadSlots` seats from `--pads`), the screens only 1P
drives (mode, assignment, select's Back…) check `seats[0] == dev`. With 1P's pad unplugged nothing matched, so the
player was stuck; in the match the keyboard only drove a `Keys` seat, so 1P stood still.

Fix:
- `main_menu.rs` `Menu::takes_lost_1p`: when `seats[0]` is a pad no longer connected (Bevy keeps the entity and drops
  its `Gamepad`, so it's missing from the readable list), the keyboard's first press (or a click/tap, which is the
  keyboard's while 1P's pad is out, `pointer::device`) takes 1P. A keyboard seated elsewhere trades seats with it, so
  the missing pad, plugged back in (same entity), drives the keyboard's old seat. An unseated pad takes 1P too; a pad
  seated as another player doesn't. Runs in the main menu and in the pause menu's controller assignment (same system).
- `controls.rs` `PadSlots::keys`: with seats and no keyboard seat, the keyboard drives 1P while 1P's slot is empty
  (pad out); a pad filling the slot again (its own, or a spare) takes it back. The legend (`key_hints`) and the pause
  assignment's `seats_now` follow `keys()`, so they show the keyboard on 1P. Pause/resume (`menu.rs`) already took
  any device.

Tests: `controls::tests::seated_pads` (B388 cases), `main_menu::tests::keyboard_takes_over_a_lost_1p`.

## Not verified / not 1:1
- Not tried with a physical pad unplugged in the running app (unattended run, no pad to pull); covered by unit
  tests of the seat logic and Bevy's disconnect behaviour (entity kept, `Gamepad` removed — bevy_input 0.19.1).
- Front-end behaviour, ours to design (PLAN.md); the original has no keyboard, so there's nothing to match.
- Only 1P is taken over: another player's pad going out still leaves that seat waiting (pause → assignment fixes it).
