# P17p2: the dive's inputs to the run object (FINAL)

## What the run object reads
- The hit event's dive branch, player +0x3f80 and +0x3ec9.
- The hit event fires on the dive's search frame (`0x34afc0` sets +0x3f97 after the search, see N3c6). In the app, that is the frame `play.rs` sets `p.dive`. No new dive can start the tick the old one ends, because the search needs `p.dive` to be none and it only goes none in `dive_frame`, after the search. So "dive Some now, None last tick" is exactly the event.
- +0x3f80 is the dive's slide (`Dive::slide`, N6). The player keeps it after the dive.
- +0x3ec9 is the "cut short" flag:
  - It is set to 0 only at the dive's start (`0x34d8a0`, alongside +0x3ec1 = 3).
  - `0x34ec70` sets it to 1 when a blocked step leaves the player more than 25° off the dive's direction. This happens both in the slide (which also jumps the counter) and in the root phase (which also clears +0x3f94, the root motion).
  - It is never cleared when the dive ends. Before a player's first dive it holds junk (14, 208, 240 in foot_s05x).
  - The port's `stopped` was only the root-phase half. `swing::Dive::cut()` is now both halves.

## Port
- `foot::DiveWatch::see(Option<(slide, cut)>) -> (dive, lunge, dive_over)` keeps the event (the start transition), the slide and the cut flag, and both persist past the dive.
- `foot_fx.rs` feeds it from `p.dive`.
- Visible change: the `run/dash` streak now stays until the motion leaves 0x1e. Before, it went when the app's dive ended, which is about 5 frames early on every recorded dive: motion 0x1e lasts 71 frames, the dive 66.

## Check
`foot.rs dive_inputs_s05` runs over foot_s05x/w/d (8 dives). The live window runs from the event for +0x3f84 + 1 frames, the same as `Dive::len`. From each player's first dive on, every frame's (start, lunge bits, dive_over) equals the recorded (event && branch 3, +0x3f80, +0x3ec9 != 0).

## Left
- No recorded dive was cut short, so `Dive::cut` going to 1 is checked against the decompile only.
