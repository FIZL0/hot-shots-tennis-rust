# N6 — diving (FINAL: search and body motion exact on all 8 recorded dives)

## When the game dives
The contact search (`0x34d8a0`) tries the dive last: after the smash, volley and ground branches find nothing,
and only when +0x3e58 == 1. +0x3e58 is the previous frame's locomotion state, copied from +0x3fa5 in `0x34afc0`;
1 means running.

## Candidate scan
- The search scans path frames k = 5 .. min(len, 16). The path buffer at 0x423f90 has a 0x40 stride.
- A frame is a candidate when all of these hold:
  - |z| ≥ 0.5.
  - The ball is on our side: sign(z) ≠ sign(+0x12b0).
  - 0.2·h ≤ height ≤ 1.8·h, where h is the stroke height +0x13b8.
  - The horizontal direction to the ball is within 22.5° of the facing: dot ≥ 0.9238795 (0x3f6c835e).
- The facing the check uses is the body's forward direction. In the recording it is +0x3d60 of the frame before. +0x3e60 is stale until the dive writes it.
- Lunge = |v_xz|·1.25·k, using the run velocity +0x3e00/+0x3e08. Limit = lunge + reach·1.3, with reach from +0x13b0.
- If dist ≤ limit and bounces ≤ 1, it is a contact candidate. Keep the one with the largest |z|.
- Otherwise, if dist ≤ 2·limit, it is a miss candidate. Keep the one with the largest dot; its slide is that frame's lunge.

## The dive itself
- Contact candidates win over miss candidates.
- On a dive:
  - The facing rows (3d60/3dc0/3e60) are set to the direction toward the chosen ball.
  - 3f8c = 3 (receive_f_dummy), 3f90 = 50, f88 = 0.
  - The motion is 0x1e (receive_f).
- Contact dive:
  - 3ec4 = k, offset = k−10, grade 4 (2 if |offset| < 2).
  - Slide from `0x3509e0`: the arm from shoulder to racket tip (0.7 m along the hand joint), turned to the new facing, reaching the ball.
  - f84 = k+50.
- Miss dive:
  - 3ec4 = −3, 3fa0 = 999.
  - Slide = lunge.
  - f84 = k+50.
- The hit routine treats branch 3 like a volley: class 2 tables, with no voice or grade calls.

## Per frame (`0x34ec70`)
- Frame n runs from 0 (the search frame) to f84.
- n ≤ k (slide): offset = dir·slide·(n/k). The offset is reset to 0 after n = k.
- n > k: offset = dir·root_z(n−k+5) from receive_f_dummy. The motion is held at time 4 through the slide.
- The body moves by the change in offset through the mover `0x34a960`.
  - The mover returns true when nothing altered the step.
  - When a step is altered and the direction from the dive's start is more than 25° off dir (0.9063078), the slide ends early: the counter jumps to k+1 and the motion goes to time 5. In the root phase, root motion stops instead.
- The dive ends after frame f84.

## Verification
`swing.rs new_recording_dives`: all 8 dives in new_recording.bin are miss dives, and each was checked as follows.
- Frame, kind and slide (+0x3f80) are bit-exact. The direction is within 1e-6, because the path is the recorded flight.
  - The game's predictor matches the live flight there. Replaying the ball from its object drifts by about 1 mm after the bounce. That is a ball-replay gap, not a dive issue.
- Body position through the slide and recovery is bit-exact on all 487 frames, using the recorded direction, `player::mover` and the partner as of its update.
  - The comparison stops where the game freezes (the dive counter stalls at 10541).
- Characters were found by their root paths:
  - p1 uses the shared path. Characters 0, 1, 2, 4, 6, 8, 9, 12 and 13 all have the same receive_f_dummy.
  - p2 = character 10, p3 = character 11.

## Left
- Contact dives: the arm slide and launch are unverified, because there are no recorded ones. The shoulder and tip are character 0's in the receive pose, from slot5_ee. `0x35ae50` and the `0x3573a0` IK step are not ported.
- The slide's early stop is ported from the decompile, with no recorded case. The root-phase stop is also unexercised; the one stall seen was a game freeze.
- `play.rs` uses this frame's run in place of the previous frame's locomotion state (+0x3e58).
- No balloon or voice on a dive hit.
