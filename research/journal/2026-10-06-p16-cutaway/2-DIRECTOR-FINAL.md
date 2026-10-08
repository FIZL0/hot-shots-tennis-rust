# P16a — Cut-away director (FINAL; court-view orientation open)
- Recording: `tools/record_cutaway.py <out> <n> --players` → `context/fixtures/cutaway2_s05.bin` (1460 samples ×
  21404 B: the old 0x4d04 sample, then point globals 0x80, pick counters 0x18, gm+0x340 0x40, and per player 0x170:
  head bone 0x40, Spine2 bone 0x40, predicted head 0x10, +0x3d40 0x80, +0x3db0 0x10, +0x12b8 8, anim object 0x40,
  clip length 8). Test `director_s05`: 12 cut-aways, 10 with the pose held at the predicted time.
- Start: the tick the reactions start (`PostPoint::reacted`, motion time 2–3). Ends in the port when the phase
  leaves Post (the game's own end not traced).
- Roles: A = last hitter, B = the one before (server/receiver before any hit). Shown = A; on a game-ending point
  whichever of A/B is on the winning team.
- Predicted time: 117 frames for rally points (97 + 20, team reactions too), 378 for game/set points, clamped to the
  motion's length; team reactions (and gu_set) move along their root path.
- Frames (match within 1e-4): entry 6 = nearest corner (±5.485, ±11.885) more than 5 m from entry 19 looking at the
  origin; 29/30/45/46/61/62/77/78 from head + Spine2 (`subject_frames`: Z = −head row2, falls back to the head−spine
  ground direction when it faces away from the spine or the head lies flat (|row0.y| < 0.4); ground/−0.2/head/spine
  positions); 35 = live head frame (`head_frame`, x scaled by handedness).
- Mirror: alternates per cut-away, the first one not mirrored; court views count in the alternation.
- `pick` inputs: list 2 when the shown player is in a team reaction (motion ≥ 0x30), else list `game_end`;
  lost-skip of 0x62/0x63 only outside team lists; 0x63/0x66/0x67/0x68 skipped on game end with no replays;
  0x61→0x65 on a lost game end. Counters match the recording.
- Open — court views 0x0c/0x0d/0x11: eye and channels exact, orientation off (0x11 ≈ 21° yaw + pitch, 0x0c/0x0d
  ≈ 40–47°). Their records carry an extra block 0x48..0x70 (bytes 0x48–0x4e, floats 0x50–0x6c) that the shot
  updater passes, blended with three constant pairs, to a framing function whose result accumulates into the
  camera's yaw (+0x2ff4) when record byte 0x4d is set (copied to cam+0x17a). That framing function (≈250 lines,
  aims at the players) is not ported; the tests still skip these three shots.
