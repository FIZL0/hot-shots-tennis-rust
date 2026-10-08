# B40g: handicap and offbeat rules (deferred to stretch)

The user moved both out of B40g (2026-10-08: "don't port the rule changes yet, add that to stretch way down the line";
"we dont need handicap either add to stretch"). Now B40g1/B40g2 in PLAN's Stretch. What was found before that:

## Set Handicap (B40g1)
- Menu strings in message0.dat (MENU00B.XB0): 298–300 (None/x1/x2), help 327, 330, 335–341.
- Match settings block 0x422f90: chars +0x18[4], costumes +0x28[4], control +0x38[4], handicap +0x68[4] (0x422ff8).
- Player constructor 0x344740 sets player +0x12c8 to 100/80/70 from the handicap byte; the run-speed formula reads it.
  The port's `run_speed(s, run, stamina, size)` already takes it; callers pass 100 (player.rs ×2, play.rs, doubles_ai.rs ×2).
- Weight models: CMN EFFCT.XB0 `data/azuma/chara_eff/weight/{weight, weight_pc05, weight_pc07, weight_pc09_c}`, on
  Bip01RCalf/Bip01LCalf. Attach 0x3bc4b0 / 0x3bcd50, placement 0x3bc930, matrix registration 0x3484d0.
- Original sub-screen screenshots: context/b40g/{confirm,hover_court,hover_handicap,handicap}.png
  (`research/b40g_confirm.py` walks save slot 2 → confirm screen, saves scratch slot 9).

## Offbeat Rules (B40g2)
- Strings 331, 343–346. Option bytes 0x2ef7e2 (rule 1) and 0x2ef7e3 (rule 2); rules 3/4 are placeholders.
- Rule 1, irregular bounce: `ball_irregular.mdl` in CMN GAME.XB `hatsuyama/ball`, loader 0x3788a0; the bounce reads the
  gate at 0x375e30, 0x37a2e0, 0x38f510.
- Rule 2, slow motion: 0x326f10 toggles the global slow-mo 0x2ef090 (needs P0b4d's slow-mo first).

## Not verified / not 1:1
- Nothing ported: both features are deferred by the user; the confirm screen still has only court/sets/games/umpire.
- The handicap → 100/80/70 mapping and the bounce/slow-mo gates are read from the decomp, not checked in lock-step.
- The M7 cameras setting (the rest of B40g) waits on M7.
