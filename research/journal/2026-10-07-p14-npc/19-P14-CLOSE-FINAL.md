# P14 closed (2026-10-16)

This closes the parent of P14a–f. Every subtask (P14a, b, c with c1–c5, d, e, f) was already ticked in `plan/DONE.md`,
so this pass only checks P14's acceptance line: NPC positions and trigger frames match PINE recordings on at least
three courts.

`tools/check.sh -p hst-sim --test npc -- --nocapture`: 13 passed. In this worktree, `context/p14d/run1/c5.bin` (P14d's
court 5 crowd) was copied in from slot s1's `context/`, and `court5_crowd_like_the_game` passes too (9444 ticks,
8 cheers). What the tests cover:

- **Positions** (`creatures_match_ram_on_four_courts`): courts 1, 2, 4 and 10. Every creature record's position and yaw
  is bit-exact against RAM images, and the spawned walker/trigger set matches.
- **Walkers** (`walkers_animate/face_like_the_game`): courts 1, 4, 10, against `npc_s*` recordings.
- **Trigger frames**: emitters on courts 1, 2, 4, 10. The trigger engine on courts 5, 8, 9. Startled creatures on
  courts 1, 2, 7, 11. Ball hits on courts 4, 9, 10. Court 4's passing ball. Court 5's crowd. The serve set-up
  keep/restore.

## Not verified / not 1:1

- Match-event creatures (P14c5) have no recording. They were checked by eye, and only their RNG draws are tested
  (in-match presentation under PLAN).
- Type 21's ball hits on court 6 (P14c4b) have no recording. The user closed this on 2026-10-16.
- The visuals are stand-in/disc models, checked by eye (rendering only has to look right).
- `prox_c08.bin` and `prox_c09.bin` don't exist. Those optional cases of the startled-creature test skip, and courts
  1, 2, 7 and 11 cover it.
