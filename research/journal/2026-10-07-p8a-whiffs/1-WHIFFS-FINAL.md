# P8a: missed swings (whiffs)

Ported as `hst_sim::motion::Whiff` (with `swing::smash_whiff`), wired in `play.rs` (`press`, `whiff`,
`whiff_frame`, `input_off`). Test: `tests/motion.rs::recorded_whiffs` replays all 66 recorded whiffs
(new_recording, round1, match_s05, lob_smash_s05) frame by frame: countdown, frames since the pose, re-press lock,
miss motion, re-presses (4, shout flag as recorded) and the frame each player is freed (45).

## The original

- Contact search on the press (0x34d8a0), once, no hold. Defaults: countdown +0x3ec4 = −3, pose +0x3e4c = 8.0,
  recovery +0x3e50 = 30, re-press lock +0x3e54 = −1.
  - Ball not this player's (no last hitter, or own team hit it last): countdown −2, the kind's ground stroke
    (2→0x10, 3→0x14, 1→0x12, 0→0x10) with no side offset. No miss motion, no shout, no re-press lock.
  - The search found no stroke or dive: countdown −3. A smash whiff (branch 4, 0x1f) if the ball is less than 2 m
    across the ground and any predicted sample before the 2nd bounce is above +0x13c4. +0x13c4 is TParam col 64's
    middle cell, which is the midpoint of the smash window for all 8 characters seen. Otherwise a ground whiff:
    the side comes from dz·v.x − dx·v.z, mirrored for the hand.
- Stroke state (0x349410), per frame. +0x3f00 counts up first, but only while the countdown is −1.
  - −3: on motion frame 8 (the 9th frame after the press) 0x350840 sets the miss motion (0x27/0x28/0x29/0x2a).
    It shouts unless +0x3f04 is set, and sets the lock to 30 for a smash and 2 otherwise. Countdown → −1.
  - −2: on frame 8, countdown → −1 and nothing else happens.
  - −1: a press once +0x3f00 ≥ the lock swings again quietly (+0x3f04 = 1). Otherwise, once +0x3f00 ≥ 30, a press
    swings (loud), a stick runs, or the player stands when the motion ends.
- A ball arriving during a whiff can only be hit by a re-press, because the search runs only on a press.
- Input between points: presses after the dead ball (phase 3→4) but before the reactions (+0x3fa4 = 2, usually
  31 frames later, 73–76 at a game end) still search the last hitter's ball and whiff with the miss motion
  (new_recording 1660, 2364). Once the reactions start, a press does nothing until the serve reset (0x3449f0).
  After a fault the players' logic stops (gm+0x58 halts) with no reaction.

## In the app

- `press` is ignored from `react()` until `next_point` and during the change of ends (`input_off`); `react` drops
  a whiff or pending press in progress.
- The ball is theirs, the phase is Rally, and the search finds nothing: the 28-frame `PRESS_FRAMES` hold (the
  P7 auto-approach stand-in) runs first, then the −3 whiff. Theirs outside Rally (after the dead ball): the −3
  whiff now. Not theirs: the −2 swing.
- Not done: the per-character smash middle (`g.reach` is character 0's, as for the search). Presses after a fault
  are still taken (`Phase::Over`); P12a owns the post-point timing.
