# P2d — Aim leftovers: nudge, short-only, the smash's held depth; a singles ×0.45 that matters

Status: FINAL.

The rally aim's core (0x34c250) leaves three values for the launch that `shot::aim` dropped: the centred
stick's ±5/±10 nudge (+0x3ed4), the drop shot's short-only flag (+0x3eca) and the plain smash's held depth
(+0x3edc). `shot::aim` now returns them (`shot::Aim`), the rally launch feeds the nudge and flag into
`swing::timing_launch`, and the smash launch, which had no timing scatter at all in the port, gets the original's
(0x35b030 branch 4 → 0x35b640 → the smash call in 0x3467b0).

## Files
- `crates/hst-sim/src/shot.rs`: `aim` returns `Aim` and takes the game's RNG (`roll`) for the nudge's 2 draws.
- `crates/hst-sim/src/swing.rs`: `smash_scatter`, `smash_scale`; `height_steps` split out of `timing_error`.
- `crates/hst/src/play.rs`: `Game::aim`, `Game::smash_scatter`; `aim_target` keeps the aim; `strike` scatters a
  smash. `play/timing.rs`: `launch` reads the nudge/flag, `smash` builds the smash scatter.
- `tools/record_aim.py`: `AIM_INCOMING=sweet`, `AIM_STICKS`, `AIM_BUTTONS`; 4 sweet-incoming singles aims appended to `aim_singles.bin`.
- Tests: `aim.rs` `human_aims` (nudge, flag, held per aim; ×0.6 swap must fail a sweet incoming slice in each
  mode), `timing.rs` `timed_smashes_like_the_game`.

## Journal
- [1-LEFTOVERS-FINAL.md](1-LEFTOVERS-FINAL.md)
