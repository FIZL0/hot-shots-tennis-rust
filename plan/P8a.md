# P8a

- [ ] **P8a — Missed swings (whiffs).** Pressing a shot button when the contact search finds no ball (out of
  reach, too early/late, ball not in play) must swing at nothing exactly as the original: the miss branch of the
  hit routine / contact search (`3467b0`, `0x34d8a0`), which swing animation it plays (forehand/backhand/volley/
  smash variant, by ball side and height), its frame timing, and the movement lockout — the frames the player
  can't move or swing again after whiffing, plus any slowed recovery. Also whether a ball arriving during the
  whiff can still be hit. Verify with recorded whiffs (pad + player position/state over PINE): lockout start/end
  and positions frame-exact through P0's harness.

**Note (user, 2026-10-07): there must be a swing delay so presses can't be spammed (match the original).** Found
in the decomp, not yet ported. The swing state counts +0x3f00 only while the contact countdown +0x3ec4 is −1, and
the count is reset when the swing motion starts (`0x34afc0`). A search start (`0x34d8a0`) sets +0x3e50 = 30 (15 for
a slice, `0x3467b0`) and +0x3e54 = −1. There are two kinds of whiff:
- **The ball is theirs (the last hitter is on the other side) but no contact is found:** countdown −3. At motion
  time 8.0, `0x350840` plays the miss motion and shouts program 4, then sets +0x3e54 = 2 (30 for a smash). A
  re-press swings once +0x3f00 ≥ +0x3e54, so pose+2, and sets +0x3f04 = 1, which mutes that re-swing's whiff
  shout.
- **No ball for this player (own side's ball, not in play):** countdown −2 (search tail). At 8.0 the countdown
  just goes to −1: no miss motion, no shout, +0x3e54 stays −1. The next press needs +0x3f00 ≥ +0x3e50, so pose+30.
- In the app, `press()` sends the no-ball case through `whiff()`. That plays the miss motion, shouts, and frees
  a re-press at `WHIFF_REPRESS` (pose+2), so a player can swing every 10 frames. To fix it, give the no-ball swing
  pose+30 with no miss motion or shout. Keep pose+2 for the pending-timeout whiff, but mute the shout on a
  re-swing. Also, `motion::follow_over` ends on `played` alone, while the original holds the swing state until
  +0x3f00 ≥ +0x3e50 even if the motion has finished. Check that against a recording.
