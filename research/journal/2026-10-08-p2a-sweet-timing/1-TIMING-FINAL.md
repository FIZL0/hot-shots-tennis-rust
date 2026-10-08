# 1 — Why P2's swings locked early, and timing by the game's path

## The original (decomp)
- A press (0x349410) runs the auto-approach 0x34d040 (+0x3f24 flag, +0x3f28 frame count), then the one contact
  search 0x34d8a0. The search sets +0x3ec1 (branch), +0x3ec4 = frame k and +0x3fa0 = k − 8.
- The search takes the best-depth frame (the contact point ahead of the player), not the first frame in reach.
- **There is no auto-swing.** A swing locks only from a press. The port's `play.rs` press → approach →
  `find_contact` already does the same, so play.rs is unchanged.

## Why P2's offsets looked arbitrary
1. **The idle ✕ press** (sent every 120 idle frames to skip past points) also fired mid-rally. With the ball still
   out of reach, the press ran the approach and then the search, so the offset came from the geometry (5, −6, 11,
   12), not from the timing.
2. **Straight-line eta.** The ball's z-speed changes at the bounce, so the same lead gave offsets from −3 to +2.

## Fix (tools/record_aim.py)
- The idle ✕ press is now sent only off-rally (gm+0x55 ≠ 3) or when serving.
- `contact_frame` reads the game's predicted path from P1's shot record (+0x1400: entries +0x50, count +0x54, now
  +0x58; 0x30 bytes each: pos, vel, bounces at +0x20). It picks the frame nearest the contact depth AIM_AHEAD (0.8 m)
  in front of P1, with ≤ 1 bounce. The press goes when that frame count is ≤ the lead.
- The leads cycle 8, 5, 8, 11 (AIM_LEAD), which gives sweet, early, sweet and late hits.
- Removed: P1's +0x3fa0 = 0 write, and the opponents' +0x3ee4 / +0x3fa0 writes. Only the singles player count
  (0x422fa4) is still written.
- Torn reads now use `Pine.settle`, and a torn frame is skipped. The old settle loop spun forever once vsync moved on.

## Calibration (cal4.log, slot 4 doubles, P1 standing in reach)

| lead | lock after the press | countdown | offset |
|-----:|---------------------:|----------:|-------:|
| 8    | 2 | 8  | 0 |
| 8    | 2 | 8  | 0 |
| 9    | 2 | 9  | 1 |
| 9    | 2 | 9  | 1 |
| 10   | 2 | 10 | 2 |
| 10   | 2 | 12 | 4 |

- Standing about 1 m to the side, the lock comes 2 frames after the press with the countdown equal to the lead, so the
  offset is lead − 8.
- If P1 isn't in place (side ≈ 2 m), the approach delays the lock by 6–10 frames and the offset is arbitrary
  (−5, −4, −3).
- A lead of 12 often gives no swing at all, so 11 is the latest lead in the cycle.

## Recordings (14 aims each; `human_aims` bit for bit)

| file | sweet full-diagonal | offsets | incoming slices |
|------|---------------------|---------|-----------------|
| aim_doubles.bin | 3 (two at (−5.485, ±11.885)) | −6..7 | 2 (both non-sweet) |
| aim_singles.bin | 2 (incl. (4.115, −11.885)) | −6..12 | 0 |

## Gaps
- **No sweet incoming slice.** Plain play gave only 2 incoming slices in doubles, both off-sweet. The ×0.45
  sweet-incoming case was covered by P2's forced-write run, but it isn't in these fixtures.
  - ponytail: the fix would be a pad-driven P2 (two pads in one copy), if that case ever needs a fixture.
- **AIM_AHEAD (0.8 m) is a calibration knob,** not a decomp constant. If a run gives off-sweet hits at lead 8,
  tune it.
- When P1 arrives late, the approach runs and the offset comes out arbitrary. Those aims are still valid
  recordings, just not sweet.
