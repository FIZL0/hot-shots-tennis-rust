# P12b1 — point score pop-up

Scope chosen unattended: exact only, no placeholder sprites. P12b split into P12b1 (this), P12b2 game/set/tiebreak
shows, P12b3 call models, P12b4 finish banners / Set/Match Point.

## What the original does (scoreboard kind 1)

- Kind picked by the scoring routine (`38b820`): 1 point, 3 game, 4 set, 6 tiebreak point. Started by the
  scoreboard update (`382810`) after the pause/wait that `hst_sim::flow` already ports; a call (fault, out…) shows
  kind 5 first (P12b3).
- Update `384c00` = `flow::Show::step` (stage +0x168, countdown +0x154, roll step +0x164, fading +0x150):
  fade in alpha `128 − 128·t/5`; roll 5 steps (deuce flag 0x316620: 3 up, then 3..1 back); stage 2 sets the white
  copy (kihontokuten01, sprite +0x94) to 128; stage 3 fades it `128·t/15`; hold `1.0 s` (0x410ea8 · 60); fade out
  `128·t/5`. Alpha is written before the countdown decrements, so after `step` alpha = `128·(t+1)/5`.
- Draw `384490`, not deuce: plates (`3a6f40`: pill i_status_00 112×48 + face 64×64 at the pill, slot label inpane_p
  40×24 and rank inpane_dani 64×24 at +8,+24): singles p0 (216, 292−s) label x 248, p1 (216, 116+s); doubles
  p0 (232, 312−s) label 264, p1 (232, 136+s), p2 (216, 272−s) label 248, p3 (216, 96+s); s = 176 when the
  scoreboard's swap flag (+0x190) is set. Points kihontokuten00 cells (col 0x410fe0, row 0x411000 = 0,1,0,1,0,1,0 /
  0,0,1,1,2,2,3; 4 Deuce, 5 Advantage) at x 336, y 104 + 180·((team == 0) ≠ swap). Scorer's old value (points − 1,
  or Deuce when 0x316628 advantage) drawn 128×64 texels into width `off = 128 − 25.6·step` (0 from stage 2), the
  new value at x 336+off width 128−off (squash, not crop: `38d970` maps the full UV rect to the screen rect). With
  advantage the other team is at half alpha (64, or fade/2).
- Deuce: only duce00's "Deuce!" (256×64 at 192,192); from deuce count 2 on, × + digits (32×32 cells: digits row
  v 64 / v 96 from 8, × at 64,96) at x 288/320 (≥10: 272, 308 or 304, 336), y/h: stage 0 `256−3t`/32,
  stages 1–2 `256+5n`/`32−5n`.
- Flush order of textures: 0 pills, 1 slots, 0x23 ranks, 3 points, 4 white points, 9/10 deuce.

## Verified

`research/popup_shots.py 5 9000 context/shots_p12b` (slot 5 doubles, polls +0x148/+0x168/+0x164, F8 at each roll
step and the settle): the original's 0-15 roll and settled frames match the layout above (plate and digit
positions within a pixel of the PS2 coordinates). Ours: `HST_AUTOPLAY=1 … --play --shot … --shot-at 34.5`
(`context/shots_p12b/ours_*.png`) shows the same layout. Unit tests in `play/popups.rs` pin the roll, alphas,
advantage and deuce-count geometry.

## Open

- Row order: the original fixes +0x190 at the serve from the match camera; we latch the panel's camera-side test
  during the serve (`ponytail:` in popups.rs).
- COM rank row is 0 (as the panel, P19).
