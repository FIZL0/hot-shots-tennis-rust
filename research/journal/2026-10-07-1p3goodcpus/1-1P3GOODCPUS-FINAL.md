# 1p3goodcpus: doubles, 1 human + 3 hard-outfit CPUs, court 11 (FINAL)
Commit d629148. Verifies the port against a doubles match with one human and three CPUs on the hardest AI rows.

## Recording and fixture (context/, not in git)
- Recording `context/recordings/1p3goodcpus.p2m2` (state `1p3goodcpus_SaveState.p2s`): Carol (char 6, lefty, P0, human);
  CPUs chars 11 (Will, lefty), 2, 10; outfits 9, 9, 4, 9; court 11.
- Fixture `context/fixtures/1p3goodcpus.bin` (+ `.pad` `.mark` `.path` `.setup.json`, `_ee.bin` RAM), 3910 frames. Two
  runs: identical pad bytes, game state equal on 3904/3910 frames; run 1 kept as `1p3goodcpus_run1.bin`.
- App: `--play --stage 11 --court 11 --chars 6,11,2,10 --outfits 9,9,4,9`. New `--outfits` picks the model, the panel
  face (`face_CC_OO`) and the CPUs' AI row; the game's AI rows equal the port's `menu_row` for outfits 9/4 (hardest
  block); stats are per character only.

## New tool: tools/play_p2m2.py
Plays a p2m2 unattended through vpad. Calibrates the sticks with the match state loaded (other screens skip pad polls,
so calibrating at boot failed) and waits up to 6 frames per stick byte (host lag jitters under load).
Known: its final pad check fails on 3 frames, deterministically and the same in both runs: R2 pressures 0x2a/0x0f
arrive as 0x5f (trigger calibration maps low pressures wrongly) and a one-frame cross tap at frame 1237 never arrives.
Tests use the game's own pad bytes, so the fixture is consistent.

## Divergences fixed
1. Serve aim pulled inside the service box (margins 0.15 across / 0.10 along, times direction). The game did it on all 5
   of Carol's serves; the earlier match's CPUs never aimed near a line.
2. Toss hand/apex per character from its own animations, mirrored for lefties (held ball too). Disc-sampled values for
   chars 6/10/11 are up to 4e-6 off RAM (sampling rounding, open).
3. Strokes/volleys/smashes: aim pulled inside the court before the scatter (doubles width 5.485, baseline depth,
   angled-topspin margin with tanf). Bit-exact on all unscattered launches of 5 recordings. Low/special smash modes
   not modelled.
4. Counters launch from the incoming hitter's tables and shot record (Carol's counters at frames 19143/19727 matched
   char 10's tables).
5. Umpire call countdown was tested after the decrement; now before. Calls that run on the countdown (out after net,
   umpire 2) settled one tick early.

## Verified bit-exact with no change
Live ball over 3092+ frames incl. 5 net-cord contacts; lob serve (kind 3) by Carol; lob stroke (Will, frame 19585) and
lob volley (char 2, frame 20682) launches; cross-button smash and 3 triangle smashes, launch + flight; all 4 yellow
smash markers (placed 19589, 20779, 21213, 21353: frame, x/z bits, cleared on next bounce or hit; the first was cleared
by Carol's smash before the bounce); dives, approaches, contact search (38 decisions), locomotion incl. 24 partner
pushes, facing, lefty stroke stamina (forehand bit 1), reactions, rally block, serve placement.

## Not covered
CPU serves (only Carol served), lefty serve walk, char 10 gu_set reaction, camera (needs a `record_camera.py` capture).

## Open
- Other ServeData values (timing grades, depth bias, mistiming, max angle) still char 0's.
- Ground strokes 4/7 are table-exact (known pre-existing slice/char-1-style 1e-6 gap).

## Process hazard
Twice during this session a parallel task merge ran `git stash` on main, removing all uncommitted work (including the
untracked tools/play_p2m2.py) until it was popped. Merges must not stash on a tree other tasks are using.
