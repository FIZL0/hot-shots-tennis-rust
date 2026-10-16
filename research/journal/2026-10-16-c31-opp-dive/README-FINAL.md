# C31 — aim.rs `OPP_SMASH` vs the decompile

Question (from C3c): the singles rally aim's "opponent smashed" branch compares `Look::opp_kind == 3`, and
`opp_kind` holds the hitter's swing branch, where 3 is the dive and 4 the smash. Name wrong, or comparison wrong?

## Finding: the comparison is the game's, the name was wrong

- The singles rally aim (game's NET/BASE aim chooser, entered after the volley/reach dispatch on the AI's own
  contact kind) tests `AI+0x124 == 3 && own contact kind != 4` — the port's
  `l.opp_kind == 3 && l.kind != contact::DIVE`, same constant, same field.
- `AI+0x120..0x15c` is the AI's copy of the last shot record, taken by the hit message from the ball object's
  `+0x1b0` block; the same message handler tests the same word `== 3` for the net player's dash (`Rally::heard_hit`,
  already `shot::branch::DIVE`).
- The ball object's record is written at the hit from the hitter's `+0x3ec1` byte (the contact-search branch,
  `motion.rs`'s doc). That same writer clears the sweet-spot flag when the byte is 3 (`shot.rs`'s
  `sweet = b != branch::DIVE && …`, bit-exact), and the search stores 3 there together with `+0x3e4c = 10.0`
  (the dive's `down = 10` in `swing::grade`). So 3 is the dive.

So the branch fires when the **opponent dove** (and we are not diving ourselves): aim by the fine zone, depth 2,
volley plan. Behaviour unchanged; `OPP_SMASH` → `OPP_DIVE` (= `shot::branch::DIVE`), docs on `Look::opp_kind` and
`Rally::opp_kind` say it is the hitter's swing branch.

`tests/ai.rs` `singles_aims_match_the_game` now also prints the recorded calls by opponent branch:
`[0, 50, 1, 0, 0]` — the fixture never has an opponent dive, so this branch isn't exercised by the recording.

## Not verified / not 1:1

- The opponent-dove branch of `rally_aim` has no recorded call (fixture `ai_aim_singles.bin`: 51 rally calls, none
  after a dive). The port's code for it was checked line by line against the decompile, not against a run; a
  recording with a dive (bot singles, long) or an oracle test of the aim chooser would cover it. Not opened as a
  task: the condition and constant are proven identical, only the branch body lacks a recorded case.
