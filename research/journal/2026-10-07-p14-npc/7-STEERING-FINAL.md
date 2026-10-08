# 7 — Steering creatures (P14c3a)

## What the generic tick does when the row's steer (+0x24) ≠ 0 and mode ≠ 6
- **Direction.** n = unit(target +0x150 − pos +0x1e0). This is the dist2 form with w included.
- **Heading.** Yaw = atan2f(n.x, n.z). Pitch = atan2f(−n.y, sqrt(z·z + x·x)), as madd.
- **Easing.**
  - Pitch (+0x184) and then yaw (+0x188) each become wrap(angle + steer · turn(target_angle, angle)).
  - turn(a, b) is the short way: d = (a+π) − (b+π), and whichever of d, d+2π, d−2π has the smallest magnitude.
  - wrap brings the angle back into [−π, π].
- **Step.**
  - The matrix is rot_x(pitch) · rot_y(yaw), and forward = its row 2.
  - With orient set, the creature faces forward. Row orient 2 levels it: (f.x, 0, f.z, 0). The flag is not cleared, unlike the plain branch.
  - pos += forward · speed.
- **Arrival.**
  - First test: dot(unit3(target − +0x260), forward) < 0 means not arrived. +0x260 is never written and is 0 in every recording.
  - If that test does not rule it out: arrived when dot(unit3(target − new pos), forward) ≤ 0. Arrival takes the next waypoint.
- **Initial angles.** A first waypoint with orient sets both angles from unit(target − pos). The startle callback (msg 0/4) sets them from home row 2, which is not normalised.

## What clears the type flags (this resolves 6's open item)
- A one-shot path's end (mode 5) sets stage (+0x27a) to 1.
- The draw pass after each tick runs when the creature is active, on and not +0x282. It counts stage down; at 0 the creature goes inactive and its type's startled flag clears.
- Row byte +0x71 adds a view-cull check before that count. Among the moving types only type 32 has the byte; this is not ported.
- step_near now does the countdown. The test checks both that a startle sets the flag and that a path's end clears it.

## Looping controller
- The spawn sets the animation controller's loop flag (+0x78) when the row's repeat gap (+0x3c) is 0. Type 5 has this (12 frames, looping); types 0 and 1 hold at the end. RAM from the save state confirms this.
- The trigger's frame set now wraps by repeated subtraction and addition of len, like Walker::set_frame.

## Recording stall
- In prox_c02, vsyncs 31937–31939 show no game tick, and 31940 then carries 3 ticks. The test now tries 0–3 steps per pair.

## Result
`startled_creatures_match_the_game` no longer skips steering ticks and is bit-exact:
- court 1: 5992 ticks, 3 startles;
- court 2: 7056 ticks, 1 startle;
- court 11: 2384 ticks, 6 scrub turns.
