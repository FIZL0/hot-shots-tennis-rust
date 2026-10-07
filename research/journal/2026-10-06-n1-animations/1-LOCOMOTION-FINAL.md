# N1a — Locomotion: run speed, stamina, stand/run motions (FINAL)

Code: mode setter 34afc0 (mode 0 stand, 1 run, 2 stroke, 3 ?), per-frame player update 349410 (run counter +0x3dfc,
stamina tick +0x3df8, stamina +0x3df4), mover 34a960 (bounds |x|<=8.685, 1.5<=|z|<=17.885 own half; doubles partner
1 m push not ported), stance 34bdf0, motion setter 350280 (motions <8 get +8 "tired" when stamina <10), acosf 111398.
Asm: context/notes/asm_move.txt, asm_acos.txt.

Player fields: +0x1374 speed = TParam SPE/10, +0x1378 stamina = STA, +0x1388 agility = Agili (x150/100 on court
surface byte (court+0x135) 2/3), +0x12b4 hand (-1 lefty; model flag +0x135 = TParam hand==2 -> 0.01 dir nudge),
+0x12b0 forward z sign, +0x12c8 size percent (100), +0x13fc live ball. Table 0x2f07xx stride 0x118 = TParam rows.

speed = 0.0717024 * (1 + 0.3*min(run,agi)/agi) * SPE/10 * (stamina<10 ? (1-0.03*(10-st))*size : size)/100.
Stamina: -1 per 60 running frames in rally (phase 3, >1 player), floor 0 (0x3fc578); reset to STA every point
(3449f0). Strokes also cost stamina (bigger drops) — not ported (shots).

Verification (tests/player.rs, match_s05.bin): run velocity bit-exact on 9657 frames, motion number exact on all
70575 stand/run frames. Update-order findings: players update by index; the hitter sets last hitter (0x423058) and
the ball during its own update, so lower-index players see the old ones that frame; the turned flag +0x3dd1 is
cleared by the facing turn after the motion is chosen. The recording's tail from frame 26108 runs ~20 game ticks per
sample (phase timer), excluded.

Open (next sub-prompts): facing turn (rest of 349410: per-motion root matrices at player+motion*0x40+0x6b0, 22.5°/f),
when each mode applies (stroke/serve/reaction state machine), blending, IK, faces.

## Follow-up (same day): missing pieces
- Mover 34a960 ported literally (asm context/notes/asm_mover.txt): partner push before and after the step; first-pass
  off-court push falls back to the nearest on-court of 4 points around the partner (x = mate.x ± sqrt(1-dz²) at the
  old z; z = mate.z ± sqrt(1-dx²) at the old x, raw diffs); second pass restores. Half-court singles mode
  (DAT_003167ac == 2) clamps z to ±6.4. Verified bit-exact on 69701 frames (frames where gm+0x50 frame counter steps by
  1 and phase unchanged — the game stalls at point end, gm+0x50 unchanged for 3 samples). No push occurs in the match.
- Stroke stamina 35ae50 (at contact): branch 1 (ground) costs +0x1380 (TParam col 42 #2 backhand) when the forehand
  side bit (+0x3f50 & 1 for righties, 2 lefties) is clear; branch 3 (dive) +0x137c (#1); branch 4 (smash) +0x1384 (#3);
  clamp [0 (0x3fc8d8), STA]. All recorded drops match.
- court+0x135 = weather (sky colour rows at 0x3fc060; chosen from gm+0xd8 table by DAT_00422f9c % 65); 2/3 slow agility.
