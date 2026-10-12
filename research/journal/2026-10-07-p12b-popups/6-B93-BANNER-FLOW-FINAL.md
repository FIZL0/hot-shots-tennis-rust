# B93 — "the blue game receiver and between-points issue"

Read as: the "Game" + blue "Receiver" banner over the result board (P12b5) and what happens between points around it.
Outcome: the banner and its timing already match the original in every case; no port change was needed beyond a wider
unit test. The visible difference between points is the camera (B48), not the banner.

## What was checked in the original (copy 4, slot 5 doubles)

- `research/b93_banner_log.py 5 7000 context/b93/slot5_phase.txt`: per-vsync log of the banner (+0x571/+0x574/+0x578),
  server 0x42304c, winner 0x4230a8, result kind 0x4230b8, games and the match phase gm+0x55/0x56. Two game ends
  (0-1 receiver win with a change of ends, 0-2 server win without): the banner starts in the 90-tick game wait at 45,
  flashes 14 then 16, is held from wait 77 and goes off on the tick the post-point phase (4) ends, which is also the
  tick the server rotates, whether the next phase is change ends (1) or the serve (2). Example: held at vsync 13981,
  off at 14154 = the first change-ends tick (14154..14234, 80 ticks). The decompile agrees for the serve case (message
  0xe clears it); the change-ends message 0xc itself doesn't, but the log shows it is gone by then. Port: dropped with
  the post-point phase (`tick_result`): same. (I first changed the port to keep it through the change of ends, from
  reading the 0xc handler alone; the log disproved it, reverted.)
- `research/b93_banner_cases.py context/b93/orig`: save at the first held banner (scratch slot 8), then per case load
  it, poke the three words the draw reads every frame and screenshot. All six cases (`context/b93/orig/grid.png`):
  Game/Receiver blue (winner team 1, receiver) at the left position, Game/Server blue, Game/Server red, Game/Receiver
  red, Set/Server red, Set/Receiver blue. The rule: word by kind (1 game, 2 set), sheet by winning team (0 red,
  1 blue), Receiver row and 24 px further left when the winner isn't the server's team. The port's `Outcome` gives the
  same in each; `result_banner` now asserts all of them.

## The port at the same moment

Temporary hook (not committed) screenshotting the port 20 ticks into the held banner of a bot doubles match
(`context/b93/ours/held.png` stage 1, `held11.png` stage 11): banner correct (Game / red Server, team 0 served and
won), the result board fading in over it with the old games count before its grow, as the original
(`context/b93/orig/*.png`: board faint at that point). What differs is the camera: the original cuts to a close-up
of the winning player celebrating, the port shows a low wide shot (stage 1: from under the net with the sky as a
flat blue dome; stage 11: across the net). That is the post-point cut-away, B48 (another agent on it this run), and
the flat blue may be B72's "blue screens".

## Not verified / not 1:1

- The user's exact meaning: unattended run, couldn't ask. If they meant the camera or the blue sky between points,
  that is B48 / B72; a question is under PLAN.md *Needs the human*.
- Singles game ends and a real (not poked) set banner weren't watched in the original: the draw reads the same three
  words in every mode and the poked cases cover each branch of it; the start/clear timing was seen on two doubles game
  ends only.
- Same-court port vs original screenshot: slot 5's court wasn't run in the port (`--stage 11` isn't it; the court
  index mapping is unverified, REFERENCE).
