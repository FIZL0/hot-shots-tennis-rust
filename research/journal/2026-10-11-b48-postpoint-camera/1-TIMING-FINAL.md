# B48 timing: when the cut-away takes over and when it gives back

## Recordings (context/b48/, git-ignored)

- `every_s05.txt` comes from `tools/record_postpoint.py 5 6000 --every`. It covers seven slot-5 points, one of them a
  fault, with every tick of phase 4. Each line holds: game tick gm+0x58, next phase gm+0x56, camera mode ch+0x51,
  pending ch+0x50, shot number and on flag.
- `timeline_s05.txt` is slot 5 played to match over, with changes only.
- `timeline_s03.txt` is slot 3 with P1 pressing Cross every second. It includes human double faults, umpire calls,
  two change-ends and the match-ending point.
- The port's side was traced with a temporary print in `play/cutaway.rs`. The command was
  `--play --pads -,-,-,-`, and the logs are `port_auto.log` and `port_fix.log`.

## What the original does (every rally point, all 18 recorded)

- Phase 4 lasts game ticks 1..149. Tick 149 requests phase 2 (gm+0x56 = 2), the same as the port's `PostPoint`, whose
  Post runs 149 ticks.
- **Start.** The shot is picked when the reactions start, at tick 33. At that point the camera handler's pending mode
  gets the shot number and the shot camera turns on. The camera switches to the shot on the *next* iteration, so it
  is visible from tick 34. Ticks 1..33 show the match camera.
  - `cutaway_s05.bin` confirms this. At the first visible sample of every run (tick 34), the shot has been stepped
    0–1 times since it was created, so it is stepped first on the frame it becomes visible.
  - Under load lag (tick 31 is held for about 4 vsyncs while the shot loads, then the counter jumps to 33 or 34),
    the switch is still one iteration after the pick.
- **End before a serve.** The first phase-2 iteration sets the pending mode to 0. The camera switches back on the
  next iteration. So the serve's first tick (tick 0) still shows the cut-away, and the match camera is back from the
  iteration after it.
- **End before change-ends.** At the first phase-1 sample the mode is already 0 and the shot camera is reset to 0x2.
  The cut-away ends at once.
- **No cut-away.** Faults and lets (no point event) have none, the same as the port.
- **Match-ending point.** Slot 3 and slot 5 each record one. The original runs the instant replay (replay flag
  gm+0x344 and count gm+0x358 = 1), with cameras 0x1d, 0x06, 0x38, 0x53 during the replayed serve and rally. **No
  cut-away follows.** Camera 0x53 holds through phase 4 into phase 5 (match over).
- **After an umpire call** (a human's double fault or a call), the cut-away starts later: visible at tick 77, 79 or 93.

## What the port did

It picked the shot and showed it on the same tick, `PostPoint.reacted` (phase-4 tick 33). That is one tick early.
On the first Serve tick it dropped the shot, which is one tick early again. Then the draw interpolation blended the
cut-away's last view into the match view over that frame, where the original cuts.

## Fix (`crates/hst/src/play/cutaway.rs`)

- The shot is still picked on the `reacted` tick, so the pick counters, the mirror flip and the prediction stay the
  same. It is first stepped and shown on the next tick. Port trace: picked at phase-4 tick 33 (fov still 0, match
  camera shown); shown from tick 34.
- The shot holds on the serve's first tick, stepped once more (`Director.held`). From the next tick it is gone, with
  `prev_view` reset so the cut back doesn't blend. Port trace: Serve tick 0 shows the shot and tick 1 has none;
  `ChangeEnds` drops it at once. Both match the original.

## Not verified / not 1:1

- **Match-ending point.** The original shows the instant replay with no cut-away after it. The port has no replay
  and cuts away as on any game point. This is P0b4d's work (the instant replay itself); a note has been added to its
  line. Not changed here: suppressing the cut-away without the replay would show a static match camera, which is
  wrong too.
- **Points a human wins.** Per B22 these may replay. With the `--press` driving, the slot-3 human never won a rally
  point, so it is unrecorded. It also belongs to P0b4d's replay decision.
- **Late cut-aways after umpire calls** (tick 77/79/93 in the original). The port starts its shot one tick after
  `PostPoint.reacted` in these cases too. Whether `reacted` lands at the original's tick 76/78/92 there was not
  checked: it needs a human double-fault recording with player reaction times. The bot recording has only one such
  case (tick 77 in `cutaway_s05.bin`).
- **Load-lag ticks.** The original holds tick 31 for about 4 vsyncs, then jumps 2–3 counter values. The port runs one
  tick per frame, so in wall time its cut-away comes about 4 frames earlier than the original's. This is the same
  load-lag gap as every other lagged tick and wasn't treated as a cut-away difference.
