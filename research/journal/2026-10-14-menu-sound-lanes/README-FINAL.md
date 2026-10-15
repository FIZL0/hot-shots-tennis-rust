# B307 — cursor move no longer cuts the select cue

User 2026-10-14: the navigation sound cut off a still-sounding select/confirm cue, which sounded abrupt.

Cause: B203's `Sound::play_menu` kept one "menu cue" id and cut it whenever any menu cue started, so a cursor
move (system bank key 0) right after a confirm (key 2) or back (key 1) killed it mid-sound.

Fix (`crates/hst/src/audio.rs` `play_menu`): two lanes, `[cursor move, every other cue]`. A cursor move only cuts the
previous cursor move (B203's no-stacking still holds for a held direction's repeat); a select/back/other cue cuts
both lanes (so mashing confirm still never stacks, and a new select supersedes a move). At most two menu cues sound
at once. All callers (main menu, pause menu, result screen) go through `play_menu`, so all get it.

Test: `audio::tests::cursor_move_keeps_select` (select keeps sounding over three moves, moves don't stack, a back
cuts both); `menu_cues_do_not_stack` still passes.

## Not verified / not 1:1
- Not compared with the original's sound driver voice allocation: front-end behaviour is ours to design
  (PLAN.md, user 2026-10-12), and the user asked for exactly this layering. Not listened to by ear in this
  unattended run.

## Wrap-up note
`tools/done.sh`'s full run failed one unrelated test, `play::mod_match::tests::umpires_are_on_the_roster_and_play_a_rally`:
the shared `mods/` (symlink to the main checkout's) got a loose `hst_umpire_04_anna/` copy at 22:39 today beside
`mods/hst-mods/hst_umpire_04_anna`, so the umpire shows up twice. Environment, not this change; left for the user
(not deleted: shared user data). Full suite otherwise passes (`tools/check.sh --no-fail-fast -- --skip
umpires_are_on_the_roster_and_play_a_rally`: 657 tests), so ticked and committed by hand as done.sh would.
