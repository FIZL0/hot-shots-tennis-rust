# B119 — no separate Serve binding

`controls.rs`' `Action::Serve` (Space) never served: the serve phase (`play.rs`, the toss press) only reads the shot
press (✕ strong, ○ weak, △ underhand toss). The action fed `SlotPad::serve`, which goes on the wire as □ (SQUARE) —
the post-point's instant-replay request (`instant_replay::buttons`). So:

- `Action::Serve` → `Action::Replay` (`replay = Space PadWest` in controls.txt; the pad's □ had no binding before).
  The controls menu lists Replay, no Serve row.
- An older `controls.txt`'s `serve = …` line is skipped without an error (as B218's turn keys).
- Key hints (B301) no longer list Serve; the shots serve.
- Test `controls::tests::cross_serves_on_keyboard_and_pad`: J and pad South latch shot 0 (the strong toss) with no
  □; Space latches □; an old file with `serve = Space` parses to the defaults with no errors.
- REFERENCE.md controls line updated.

## Not verified / not 1:1
- The test stops at the input latch (`read_input`); the press → toss mapping it relies on is the existing serve code,
  not re-run here in a full match (a human-serving match harness doesn't exist yet).
- Adding a Replay row instead of only deleting Serve is my call: removing it would have left no local way to press □
  (replay request). PadWest = □ follows the PS2 layout; the original has no rebinding menu.
