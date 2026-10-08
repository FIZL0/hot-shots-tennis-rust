# P3 — timing grade effects

## What the grade does to a rally shot (strokes class 1, volleys class 2)

- **Grade and bias tables** (`swing::timing`): built per character from TParam's after/before counts (sweet frame 8,
  grade 1; grades 2/3/4 behind, bias ±1 per grade-3 frame, ±2 more per grade-4 frame). Checked against every
  recorded player's +0x1510/+0x3f98 tables. The app now uses each player's own grades (`character_reach`) instead
  of character 0's hard-coded list.
- **Error at the lock** (`swing::timing_error`): side/depth off the bias and offset, plus a mode pressure toward the
  down tables from the contact height (ideal stroke/volley height, miss steps), body/back-hand and volley-down stats;
  dives by their direction. The lob *button* (shot code +0x3ee4 == 4) keeps its tables (mode 0); the ball kind does
  not decide this.
- **At the launch** (`swing::timing_launch`): a clean grade (1, 2) drops side/depth (keeps the aim's nudge),
  clamps ±1 m, scatter = 1.5·(sx, 0, sz) in the hit→pulled-aim frame (`serve::scatter_along`); the blend is scaled
  by the power stats; class 2 off a ground stroke uses `high_blend`; a power-gap counter uses blend 0 and the
  incoming hitter's tables.
- **Table mode / variant** (`table_mode`, `mode_variant`): up1 above 0.1 (class 2), dw1..dw3 by the per-character
  thresholds (GAME.BIN 0x3fc980); a grade-4 non-lob is at least dw1. Variant flag ↔ suffix: 0 up1, 1 dw1, 2 dw2,
  3 dw3. Lobs pick by Lob POW vs Lob2 POW on a clean grade (`lob_variant`; only characters 5 and 9 list one).
- **Late lift** (`late_lift`): grade 4, ground stroke, flat/topspin, no mis-hit → table elevation × 0.95 (37b110's
  param_2 scales the elevation, not the speed).
- Launch: lookup from (hit − scatter) to the pulled aim; launch to pulled aim + scatter.

## Verification (`crates/hst-sim/tests/timing.rs`, P0's replay harness)

211 recorded strokes/volleys over 4 recordings, every grade (1 sweet, 2 quick, 4 slow; early and late offsets):
grade, bias, offset, stored error (+0x3ecc/+0x3ed0/+0x3ed8) and the scattered target exact; unscattered launches
within the shot_tables tolerance (4e-6); scattered ones within 5e-4; 10 (~5%) are near-misses within 2e-3 and a
frame — mostly grade-4 scattered volleys, plus two clean lob_smash volleys with a small elevation-only difference.

## Not done (ponytail notes in code)

- The scattered residual (~1e-4): scanning the lookup's shift (α·scatter on hit and/or target) gives α = 1 on the
  hit as the best fit everywhere but not exact; next suspect is 37dbf0 / the curve angles in 37e950.
- Slices (class 1 kind 1) miss at every grade, as already in shot_tables — excluded from the test.
- Mis-hits (grade 4 low contact, +0x3f44 ≤ 0.2/0.4: the RNG's elevation scale via +0x3f0c or the wild shot +0x3f06)
  and the reactions (3553d0) are not ported; the test skips mis-hit launches.
- The aim's random nudge (+0x3ed4) and short-only flag are 0/off in the app; dives keep their old grade and take no
  timing error; practice's lob-variant exception is left out; character 10's listed voly1_up1 is not on the disc
  (base table used).
- `find_contact`'s approximate wind-up reach growth was already gone (it uses `swing::search`); nothing to remove.
