# P0b4d3: the replay's pacing (`play/replay_pace.rs`)

## The original (slot 5's match point, `research/p0b4d3_pace.py`, run in `context/p0b4d3/run2/pace.txt`)

`near 5 9` saves slot 9 in the match point's post-point phase with the replay pending. `rec 9 500 …` then logs one
line per vsync: the match tick, the skip words, the slow-motion block and the replay director's (camera handler's)
fields.

- **Skip.** At the replay request the match handler asks for a skip of n ticks (313 here). The main loop runs them
  unseen and with sound off, about 22 a vsync. The picture comes back at tick 313 and holds 3 vsyncs. The request is
  max(record[cnt−2].tick − 20, record[cnt−3].tick + 1) over the shot recorder's records (a table word gives the −20);
  special shots use the record's own lead instead. In replayed ticks the second-to-last shot (of 4: ticks 196, 293,
  316, 360) left the racket at 316, 3 ticks after the picture came back.
- **Slow motion** (block: on, frac, start/end period, ramp ticks/left, curve, period, in-between count). After each
  real tick, `period` in-between frames draw with frac = 1 − c/(period+1). A ramp from ×a to ×b over n ticks gives
  tick k: int(lo + (hi − lo)·(k/n)^curve), where lo = a − 1 and hi = b − 1. 0.999999 is added to the upper end, or
  to both ends when slowing down. When the ramp is done and the end period is 0, the block turns off.
- **Director** (per recorded shot, a row of a 10-row table in the game program: ramp ticks, factors and curves for
  both halves, trigger offset, first delay):
  - Tick 314, row 3: no first half. Second half: the default 62/2 = ×31 → ×1 over 2 ticks, curve 2.0. That gives
    frames 31, 24.
  - Tick 316, row 5: ×2 → ×6 over 3 ticks (2, 3, 5 frames), then ×6 → ×1 over 13 (6, 6, 6, 5, 5, 5, 4, 4, 3, 3, 3,
    2). The trigger at shot + 15 = 331 turns it off, and the camera cuts (0x53).
  - The rest, through the winner and the post-point phase, plays at full speed.

## The port

- `log` files each rally shot's tick (from `Game::shots` going up), outside a replay. The count resetting files the
  point as `last`.
- At the replay start, `pace` (after `instant_replay::run`, at the end of each tick) takes `last` and sets the next
  tick's clock on `Time<Virtual>`:
  - Before the shown tick (S − 3, S = second-to-last shot, never before the shot ahead + 1) the relative speed is
    1000 and max delta 1 s, so the fixed loop runs the hidden ticks within one frame. On the last hidden tick the
    rest of the frame's overstep is discarded.
  - After that, the speed is 1/(between + 1), with the ramps above anchored to S and cut at S + 15.
  - Speed and max delta go back to 1 / 250 ms when the replay ends.
- The draws already interpolate prev → current by `Time<Fixed>::overstep_fraction`, so slowed ticks get in-between
  frames for free. The tornado (`play/tornado.rs draw`) runs `Tornado::between` on a copy with that fraction while
  slowed (scale, aim, fade).
- `hush` (before `play_sounds`) drops the tick's queued sounds, crowd cheers and umpire word while skipping.
- `foot_fx` sets `Feet::replay = g.instant.replaying`, so dives throw their clods/blades in a replay (P17p1's
  ported debris).
- The sim and its generators are untouched: the same ticks run with the same pads. Only the frame clock changes.
  `instant_replay`'s two tests still pass, including "the replay ends where the point left the match".

## Check

- `replay_pace::the_match_point_paces_as_the_original`: the original's shot ticks give the shown tick 313 and the
  frames per tick 313..332 recorded in PCSX2 (31, 24, 2, 3, 5, 6, 6, 6, 5, 5, 5, 4, 4, 3, 3, 3, 2, 1).
- `replay_pace::a_replay_skips_then_slows_in_frames`: a headless bot match through the real frame loop, 60 Hz
  manual clock. The match point's replay (shots 216, 278, 323, 402) runs ticks 0..319 in one frame. It shows from
  320, then each tick takes 32, 24, 2, 3, 5, 6, 6, 6, 5, 5, 5, 4, 4, 3, 3, 3, 2 frames (±1 for clock rounding), then 1.

## Not verified / not 1:1

- **One director row for every replay.** The original picks a table row per recorded shot from its type (serve,
  rally shot, smash, special: types set by the shot recorder, 3/6 for the last record). Only one point was
  recorded: rows 3 (lead-in) and 5 (the shot). So the port paces every highlighted shot like that rally shot, with
  the constants inline. Smashes, specials and serves (rows 4, 6–9) would ramp differently. Porting the row pick
  needs the shot recorder's types (not ported) and the director's row choice (366e40/367eb0 region). Not chased:
  presentation only.
- **Skip point by shot ticks, not record ticks.** The original's skip formula reads the recorder's tick (−20), which
  isn't the contact tick. The port shows from 3 ticks before the second-to-last contact, as observed, clamped after
  the shot ahead. The special-shot branch isn't ported.
- **The skip's held picture.** The original holds the frame at tick 313 for 3 vsyncs after the skip. The port shows
  it for 1.
- **The ramp's powf curve** is evaluated with host `powf` (presentation only, not ps2 ops). Linear and squared,
  the two curves used here, match the original's periods exactly.
- **In-between draws** in the port interpolate between the last two ticks (Bevy's fixed-step interpolation). The
  original extrapolates from the last tick. Same look, one tick later. The tornado's UV scroll holds at the tick's
  value on in-between frames (no before/after UV time kept).
- **Muting** drops queued sounds, cheers and umpire words. The ball's flight whistle isn't stopped explicitly, and
  the BGM keeps playing.
- **Step sound not wired.** The game plays it only in a singles replay with a close-up camera (modes 0x56..0x59),
  and the port has no replay cameras yet. The gate (`sound::step`/`step_heard`, `Feet::stepped`) lands with P0b4d4,
  noted on its line.
- **No side-by-side screenshots.** The pacing was compared as frames per tick against the PCSX2 recording, not by
  eye in a running window, since a whole bot match (several minutes) has to play before the match point's replay.
