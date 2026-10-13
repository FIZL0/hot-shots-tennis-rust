# B132 — ran to the ball after a press, then whiffed

## The original (decomp + PINE)
- A press with the ball on the player's side runs the auto-approach (stroke state 1, +0x3f24 flag, +0x3f28 count).
  It plans with the **player's own** reach fields (+0x13ac base, +0x13b0 reach, +0x13bc/+0x13c0/+0x13c8 heights),
  its own timing table (+0x1510) and its horizon (+0x154c) + 20.
- `research/b132_approach.py 4 10 6000` (copy 2, slot 4, P1 human, stick let go on the press, ✕ at leads 5–20,
  standing 1.8–3.0 m off the ball's line) → `context/b132/run1/log.txt`. Every approach (planned 8, 6, 4, 18
  frames) runs exactly that many frames, the press frame included. The next frame is the stroke state (+0x3fa5 = 2):
  the search runs there and **the player doesn't move on it**. All four approaches locked (countdowns 6, 14, 4, 5).
  A press with nothing reachable (count −1) goes to the search on the same frame, also without a move. Those
  presses whiffed (countdown −3), and so did a running one that dived.

## What the port did differently (play.rs)
1. `approach` planned with `g.reach` (character 0's reach, heights and 28-frame grades) for everyone. The search
   after it used the player's own `reaches[i]`. A character with a shorter reach or lower window ran to a spot
   that only character 0 could reach, then whiffed. This was the reported bug.
2. On the search frame (`pending == Some(0)`) `human` still ran `locomote` (the approach direction, or the stick
   when there was no approach), and the bot ran its approach. That made N+1 run steps instead of N, plus a stick
   step before an in-reach search.

## Fix (additions only)
- `approach` uses `g.reaches[i]` (its reach and horizon).
- `searching(p)`: on the search frame neither `human` nor `bot` runs, and the player is held (`prev = pos`).
- Test `play::snapshot::tests::approach_runs_its_frames_then_locks`, a headless match on round1.bin's P1 pads
  (5 approaches). Each approach runs its planned frames, the search frame doesn't move, and the approach locks.
  It fails on either old behaviour: "the search frame moved the player" and "an approach to a reached ball didn't
  lock".

## Not verified / not 1:1
- The port isn't run side by side with the capture. The capture's game state (slot 4, doubles) isn't something
  the app can load, so the check is the rule (N runs, a still search frame, a lock) on round1.bin, not
  position-for-position against `context/b132`.
- The planned frame counts against the game's own path are still only checked by `swing.rs match_s05_approaches`
  (from earlier work). This run didn't dump the predicted path at each press.
- `whiff`'s smash-whiff path length still uses `g.reach.grades.len()` (character 0's horizon). Not recorded; it
  only decides the miss motion.
- The contact search's `ahead`/`smash_ahead`/body widths and the dive arm are still character 0's measured values
  for every character (an existing ponytail in `reach()`). They change which frame and swing are chosen, not
  whether the ball is in reach.
