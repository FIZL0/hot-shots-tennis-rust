# B180 — court props toppling through the intro

## Cause
The intro (`play/intro.rs`) holds the match's fixed ticks and runs its own `pose_players` each intro tick, which
calls `character::tick` over **every** rig, not only the players'. During the match `npcs::step` pins each trigger
creature's clip to its own state's frame (`m.clock.sampled = t.frame`) right after that tick; in the intro `step`
doesn't run, so the clips free-ran and looped. The ball-hit obstacles (type 14 on court 4, 21 on court 6) have their
knock-over clip as clip 0, so they fell over every 80 frames. Not the ball, not their hit state: their `Trigger`
state stayed at frame 0 and not startled the whole time.

Measured (temporary log, `hst --play --stage 4`, `HST_INTRO=1`): type 14's sampled frame ran 0, 9, 18 … 74, 3, …
(clip length 80) while `t.frame` stayed 0.

## Fix
`npcs::hold_triggers`, run in `pose_players` right after `character::tick`: puts every trigger creature's clip
back on its `t.frame` (and `prev`, so no blend). Only drawn state; the sim, its RNG and match timing are untouched
(no `hst-sim` change; `tools/check.sh -p hst` 262 passed).

## Not verified / not 1:1
- The original's intro on court 4/6 was not watched in PCSX2: reaching a new court-4 match needs the menu carousel
  from a state on my copy, which I didn't have. The props standing still follows from the original's trigger
  callback only animating 14/21 when a player's or the ball's box hits them, and the intro moving neither.
- Walkers and the umpire still tick freely through the intro (their idle loops play, as before this change);
  whether the original animates them during its intro is unchecked and outside this bug.
