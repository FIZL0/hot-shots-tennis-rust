# N1d2 — Motion clock (FINAL)
- Anim object (player+0x54): +0x20 motion, +0x24 clip (length at clip+0x2c), +0x34 speed given to the setter, +0x38 time last sampled,
  +0x3c time, +0x40 current speed, +0x44..+0x74 crossfade (old motion +0x44/+0x48, its time +0x50 / speed +0x54, length +0x5c,
  countdown +0x60, weights +0x64/+0x68, hold flag +0x6c), +0x78 loop, +0x7a dirty.
  Correction to 4-SAMPLER: the speed advanced per frame is +0x40, not +0x34 (+0x34 is the setter's copy, restored into +0x40 by the dive hold).
- Setter 0x350280(speed, player, motion, loop, blend): stores +0x3df0 (tired +8 for <8), no-op if loop and same motion, crossfade 0x140130
  (blend 8 for looping sets and whiffs 0x27/0x28; 0x1c/0x1d soft follow with blend 8 and hold flag; else none), binds 0x140360,
  +0x78 = loop, +0x34 = speed, samples at 0 (0x1404c0), +0x40 = +0x34.
- Per frame, top update 0x3467b0 after the state's update (349410+350f70 / 351dd0 / 354050): 0x140810 — if not held: 0x1404c0(+0x3c) wraps
  (loop: repeated ps2 sub/add of length) or clamps, stores it in +0x3c and +0x38, samples; +0x3c += +0x40 (ps2 add, chop matters: 1.1428572
  steps); crossfade +0x50 += +0x54, countdown +0x60 -= 1. If held (+0x6c): only samples the old clip at +0x50. Then 0x140970: if held,
  blends the new clip at time 0 by +0x64, countdown -1; when it passes 0 clears the crossfade and +0x3c += +0x40 once.
- Soft follow-through 0x1c/0x1d: NOT started at frame 8 (old SOFT_FOLLOW_FROM was wrong): setter blend 8 -> countdown 7, held 7 frames at
  time 0 showing the frozen stroke (old time - old speed) crossfading to frame 0, then plays from 0. Port: SOFT_FOLLOW_HOLD = 7; the visual blend is N1e.
- End-of-motion tests read +0x38: `clip length <= +0x38` (349410 stand/return, 354050 reaction end, 351dd0 serve); contact frame test +0x38 == +0x3e4c.
- Dive (34ec70, play state, after 0x140810): when round(+0x38) == 4 sets +0x40 = 0 (holds the dive pose), later restores +0x40 = +0x34 and
  sets time 5.0 — left for N6.
- Tick count per frame: the game skips the anim tick on a phase change frame (gm+0x58 reset) and runs two the next; gm+0x50 also jumps by 2
  at times. The countdown +0x60 drops once per tick held or not, so the test uses it as the tick count.
- Recording: tools/record_anim.py (new) slot 5, 9000 frames at NominalScalar 0.25, no missed frames -> context/fixtures/anim_s05.bin
  (sample = vsync + gm 0x100 + per player +0x3c00 0x400, anim 0x80, clip length f32). Lengths were read over PINE after the recording and
  spliced in; the recorder now reads them per frame.
- Port: hst_sim::motion::Clock {time, sampled, speed, looping, hold} start/tick/done; pose::wrap free fn (Clip::wrap uses it). App:
  character::Motion { id, clock, prev, serial }, tick samples `rig` clip length, animate blends prev->sampled (across a loop wrap); play.rs
  Cmd.from -> hold. Test motion.rs anim_s05_clock: 35026 ticks (70 held) + 713 sets, sampled/time bit-exact.
- Next: N1d3 IK/body step, N1e crossfade (the 0x140130/0x140630/0x140970 weights above).
