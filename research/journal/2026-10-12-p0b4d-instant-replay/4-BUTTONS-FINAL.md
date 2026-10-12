# P0b4d5: between-points buttons (FINAL)

## The original (decompile + PINE on copy 3)

The match handler's update runs the phase handler, then a skip/button check, then counts its phase and rally ticks.
The check does nothing while fast-forwarding, in the first 3 ticks of the post-point, or in a demo. Otherwise:

- **Post-point, asking for a replay.** It needs the point to allow one (gm+0x346, set at the point unless the rally
  was cancelled), the board's pause over this point (gm+0x360 = the rally tick when it ran out, set only outside a
  replay), ≥ 2 frames since the last skip, not paused, and L3 not held. Then:
  - □ just pressed asks for a replay of the last point;
  - or, during a replay's post-point, △ just pressed asks again;
  - or L1 just pressed, if the point was recorded, asks for a "highlight" replay (counted like the auto one).
- **Serve/rally during a replay.** ✕ just pressed fast-forwards to 20 ticks before the next shot record. ○ just
  pressed fast-forwards to the replay's end (gm+0x360 − ticks + 3). The skip is a count of ticks the main loop runs
  without vsync or drawing.
- **Cancel.** In the rally, once the rally tick count passes 1799, or while the pad recorder is off, the point's
  replay is cancelled (allowed and recorded cleared). The "long-rally option" in the task line is the pad recorder's
  active flag, not an option: it is on from each serve. In practice the cancel is "a rally of 1800 ticks".
- **The decision's own replay** doesn't wait for the post-point to end. The post-point handler holds (no reaction,
  no score show) until the board's pause runs out, then sets "start now". The serve entry then restores the point's
  serve with the replay flags and counts kept. P0b4d2 started it at the next serve set-up instead; now both start the
  same way.
- **Score show skip.** With no cut-away, a just-pressed ✕ or ○ (pad 1) ends the score show (not △ or □). A cut-away
  isn't skippable by button: the post-point waits for it. The walk-back / change-ends phase has no button check (it
  runs on the camera's countdown).

The pad bits were checked over PINE with vpad on slot 3: ✕ 0x2000, ○ 0x4000, △ 0x1000, □ 0x8000, L1 0x400, L3 0x2.

## The port

- `play/instant_replay.rs` `buttons()` runs the check at the end of each tick (`run`):
  - the 1800-tick cancel, fed into `Point::cancelled`;
  - the shot ticks (the shot recorder's records, for ✕);
  - `shown` and the auto "start now" when `PostPoint::pausing_out()` (new in `flow.rs`);
  - □ / △ asks;
  - ✕ / ○ fast-forwards, through `Time<Fixed>::accumulate_overstep`, so the ticks run before the next frame without
    drawing.
- `start` now runs at the end of any tick with a replay pending. It keeps the point's flags, the counts (asked counts
  button replays) and the shot ticks, as the serve entry does with a replay pending. The replay's pads are handed back
  as soon as they run out, so the match carries on from the tick the replay was asked at.
- During a replay the presses come from the devices' latches; the match itself reads the replay's pads.
- `play.rs` `control`: the score show skip takes ✕ / ○ only (it was any face button or the serve key).
- Tests (`instant_replay.rs`):
  - the match point's replay starts at the pause end and catches up to the same `Game` (apart from `instant`);
  - □ after the first point's pause asks for its replay, which catches up the same way (`asked` 1).

Sim RNG and match timing: the replay restores the snapshot and replays the same pads, so every generator and the
score are where they were when it was asked (the tests compare the whole `Game`).

## Not verified / not 1:1

- **L1 highlight replay**: not ported. The port has no L1 binding.
- **L3 gate, "paused" gate, ≥ 2 frames since the last skip, and the first 3 post-point ticks**: not ported.
  - There is no L3 binding and no pause in the match.
  - The `shown` gate already implies more than 3 post-point ticks.
- **✕'s target**: an approximation. The original clamps it to after the previous shot and to the shot recorder's end − 20;
  the port takes the first shot tick − 20 still ahead, else the replay's end − 20. Not checked against the game.
- **The fast-forward from a press during a replay**: not tested headless. The tests run `FixedUpdate` by hand, so the
  overstep has no effect there. Not checked by eye either: there was no interactive session in this unattended run.
- **The replay's start-of-replay fast-forward windows** (data at the start-skip table): P0b4d3's pacing, not done here.
- **Which pad**: any seat's pad stands in for the original's pad 1 (0x2f0610 / 0x2f06a0 read the same with one pad).
- **□ on a gamepad**: the port's □ is the keyboard's serve key (Space). No gamepad button sends it. Binding West to
  it would also make it a serve button in play, which the original doesn't have.
- **Sounds at the restore**: a sound started on the tick the replay starts is not stopped.
- **The cut-away**: not skippable by button. This matches the original (no change); listed because the task line
  asked.
