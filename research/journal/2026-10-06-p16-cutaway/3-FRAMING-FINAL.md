# P16c — Court views 0x0c/0x0d/0x11: target framing (FINAL)
- The shot updater's framing call (record block 0x48..0x70) aims the built view at a target frame:
  record 0x48 names it (9; the post-point mode swaps 9 for 19 = the shown player's ground spot, live).
  0x4a = pitch follows, 0x4b = pitch only upward, 0x4c = yaw follows, 0x4d = keep the yaw overshoot,
  0x70 = clamp the yaw error. Floats 0x50/0x60 offset (× vfov / fov), 0x54/0x64 dead zone as a fraction of the
  half-angle (atan(k·tan)), 0x58/0x68 its scale to the full pull, 0x5c/0x6c ease exponent (1 − (1−t)^e).
- Per frame: reference = the view with its yaw + the held total (cam+0x2ff4), eye kept; error to the target
  minus the eased pull; the view is rebuilt rot_y(yaw)·rot_x(pitch) at the same eye. When the yaw pull is at its
  limit, the rest (error − limit) is returned and added to the held total for the next frames.
- Order: after clear/band/head (all off for these shots), so it is the last step of `Shot::build`; the held total
  advances in `step`, so a repeated `build` is idempotent.
- Not ported (no cut-away uses them): record 0x14's 90-frame blend of the dead-zone triples from (4, 1, 1),
  record 0x49's eye move, the second framing call (record 0x71), the target lock (record 0x46).
- Test `cutaway_s05` now checks the court views too: 1499 frames, court views within 1.1e-5; the held yaw
  (0.19 rad on 0x11) comes out of the catch-up steps without seeding. App: `Frames::spot` (frame 19) is the
  shown player's live position.
