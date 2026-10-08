# B22 — Post-point camera priority (FINAL)
- Director traced: message 0x15 (each hit, 371270) sets A = the hitter (0x423058), B = old B on the serve (hit
  count 0x423060 == 1), else old A; the serve init sets A/B = server/receiver. At the point (message 0x17, 3717f0):
  shown/other = A/B, swapped when A isn't on the last hitter's team (never in a rally), on a game/set-ending point
  (0x4230b8 > 0) when A isn't on the winning team (0x4230a8), and every third instant replay (gm+0x358 % 3 == 0,
  ≠ 0) unless the other's character byte +0x3fa5 == 3. The cut-away starts (366e40) at state 4 once the frame
  table's flag (scene +0x10f95, set when gm+0x374 and the predicted time are on) is up; the pick (368d90) is the
  ported `pick`. No smash/net/ace/fault priority exists: only point kind (game end), team reaction, lost, crouched,
  replays, character.
- Port: the role rule is now `hst_sim::cutaway::roles`; `director_s05` derives shown/other from the recorded A/B
  and globals instead of reading them (12/12). New 4800-sample recording (`context/b22/cutaway3_s05.bin`, slot 5):
  27 cut-aways before the set point all match roles, pick, counters and side (only 3 reach the held pose there).
- Set-ending point (ge 2): the original runs an **instant replay first** (replay cameras 0x1d, 0x06, 0x53, gm+0x358
  → 1), even in the bot game; the port cuts away at once. Replays are P0b4d (not ported); replay count stays 0, so
  the game-end long-shot skip and the every-third-replay swap differ only once replays exist. Likely what the user
  saw on game/set points (and after human winners, which also replay).
- Body hit (`research/b22_bodyhit_cutaway.py 5`): the original cuts away on body-hit points too (shot 0x61/0x0c,
  shown = last hitter, the hit player plays 0x2b as its reaction). The port had no reaction root for the standing
  hit player, so `predicted` returned None and no cut-away started when they were shown/other; it now poses them on
  their current motion. Faults/lets (state 4, winner −1): no cut-away in either.
