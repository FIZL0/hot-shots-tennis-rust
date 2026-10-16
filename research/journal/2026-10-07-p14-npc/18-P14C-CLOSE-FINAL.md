# P14c closed (2026-10-16)

The parent of P14c1–5: all subtasks (P14c1, 2, 3, 3a–f, 4, 4a, 4b, 5) were already ticked in `plan/DONE.md`, so this
pass only checks the acceptance line: trigger frames and paths match PINE recordings on at least three courts.

`tools/check.sh -p hst-sim --test npc`: 13 passed, with every P14c fixture present:

- trigger engine / idle animators: `trig_c05`, `trig_c08`, `trig_c09` (types 18, 34, 37)
- emitters: `trig_s03/05/07/10` (courts 4, 10, 1, 2)
- startled creatures (paths, one-shot animation, ball strike): `prox_c01`, `prox_c02`, `prox_c07(_ball)`, `prox_c11`
- ball-hit obstacles: `hit_c04`, `hit_c09`, `hit_c10(b)`; court 4's passing ball: `ball_c04(_r1–3)`
- serve set-up keep/restore: `trig_replay_s05`

That's nine courts (1, 2, 4, 5, 7, 8, 9, 10, 11). The only skip in this worktree is `context/p14d/run1/c5.bin`
(P14d's court 5 crowd, not part of P14c).

## Not verified / not 1:1

- Match-event creatures (P14c5: court 1's landing-spot creature, court 2's flyover, courts 5/8's game-end rolls,
  courts 6/10's deciding-set pause, court 9's cheer): ported from the code and checked by eye, with no recording.
  PLAN counts them as in-match presentation, and only their draws on the sim RNG are tested.
- Type 21's ball hits on court 6 (P14c4b): no recording. The user closed the task on 2026-10-16 as no longer needed.
