# N1b — Facing turn (FINAL)

Top-level player update 3467b0: input (human pad quantized to bytes ±127 around 0x80, clamped 0x3fc660..678, back to
float /127, normalized if >1; AI via 348ff0), then by player state +0x3fa4: 0 play → 349410 (+350f70), 1 serve →
351dd0, 2 post-point → 354050. The turn is the tail of 349410 (runs every play-state frame, any mode).

Turn (asm context/notes/asm_move.txt 0x349c78..0x34a810, builder 1b3560 asm_turn.txt, VU0 mat×vec 125a50):
- target == facing (bitwise) → clear 3dd0/3dd1, 3dd4 = -1, 3de0 = 0.
- dot(target, facing) >= cos 22.5 (0x3f6c835e) → snap facing = target, clear as above (then 0x3d40 side row).
- else: t = target (+0.01 x nudge for lefties); if sign(t×f).y differs from sign(3de0.y) → 3dd4 = -1; if 3dd4 == -1
  decide: P1 = pelvis fwd of start motion (raw number, row 2 x·hand, z normalized, ×fwd), Q1 = rotY(f.z·fwd, sign
  (t×f).y)·P1; same motion: Q2 = rotY(dot(f,t), sign(f.z t.x − f.x t.z))·Q1, P2 = P1; else P2 = pelvis fwd of new motion,
  Q2 = rotY(t.z·fwd, same sign)·P2. a = ±acos(Q2·Q1), b = ±acos(P2·P1) (signs from −(B.z A.x − B.x A.z)), g = acos(t·f),
  other = −(2π − g); way 0, or 1 with g, other negated when (t×f).y > 0; flip way if |a−(b+g)| > |a−(b+other)|.
  Step: sign = way ? −1 : 1, negated when 3dd0 and the start base motion isn't the dash (7); facing = rotY(cos 22.5,
  sign)·facing; 3de0 = t × facing.
- rotY(c, s): axis (0,1,0) negated, FPU order in player.rs::rot_y.
- 0x6b0 table: 35aab0(time 0, motion m, node slot 6) = Bip01Pelvis model matrix (node names table 0x41e088: Racket,
  Head, Neck, Spine2, Spine1, Spine, Pelvis, RThigh, LThigh, RHand, LHand, RForearm, RUpperArm, LUpperArm, LFinger21,
  RFoot, LFoot). From the disc: first keys, local = R(key quat) rows, pos row 3, model = local·parent — within 5e-7.
- Stroke/dive entry (mode 2/3) sets facing and target to (0,0,fwd) before the turn.
Verified: tests/player.rs match_s05_facing (78255 frames, 5480 turning).
