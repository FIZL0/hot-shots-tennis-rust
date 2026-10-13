# B59 — dash right after a whiff

## The original
- `research/b59_whiff_dash.py 4 6 5000 30` (copy 4, slot 4, P1 human) → `context/b59/run1/log.txt`. P1 runs
  sideways with the stick held 30 frames (run count +0x3dfc 0→29, dash motion 7 from run 18), presses ✕ with the
  stick still held and keeps it held; or stands, presses ✕ and holds the stick from the next frame.
- Every whiff: the stroke state (+0x3fa5 = 2) holds the player still from the press frame; the miss motion comes on
  the 9th frame (pose), and the first run frame is 30 frames after the pose (39 after the press), with the stick held
  throughout. No recovery slowdown beyond that: the run count starts again at **0** (it stayed 30 through the
  whiff, frozen), so the run speed and motions build up from a standing start (run motion 5, dash 7 only from run
  18 again, velocities equal (to the logged 5 decimals) to the run before the whiff at the same count).
- The existing recordings agree (a probe in `tests/motion.rs recorded_whiffs`, not kept): all 45 freed whiffs in
  new_recording, round1, match_s05 (human p0 and CPUs p1–p3) that run off start at run 0, from counts up to 104
  before the press.

## The port
- The whiff's timing (pose, re-press lock, 30-frame recovery, freed by stick/press/motion end) was already right
  (P8a, `recorded_whiffs`). The bug: `whiff()` left `body.running` set, so `Body::step` didn't reset the run counter
  on the first run frame and the count went on from before the whiff — a player who ran into the whiff dashed off
  at full dash speed right away. A locked stroke (`square_up`) and a dive already clear it.
- Fix: `whiff()` clears `body.running` (play.rs, one line; it is the shared path for human and CPU whiffs, the −2
  and −3 kinds; `miss_contact` comes from a locked swing, already squared up).
- Test `play::snapshot::tests::run_restarts_after_a_whiff`: round1.bin's P1 pads in a headless match; every player
  (human and bots) freed from a whiff into a run has run count 0. It fails without the fix (run 68); with it, 17 runs off a whiff in round1.bin, all from 0.

## Not verified / not 1:1
- The port isn't run side by side with `context/b59` position for position: the capture's game state (slot 4,
  doubles) can't be loaded in the app. The check is the rule (frozen during the whiff, run from 0 after) on
  round1.bin plus the recorded whiffs' timing test.
- Facing/turn state after a whiff: the port keeps the body's facing fields from before the whiff (only the run
  flag is cleared); not compared against the original's +0x3d60/+0x3dd1 on the first run frame.
