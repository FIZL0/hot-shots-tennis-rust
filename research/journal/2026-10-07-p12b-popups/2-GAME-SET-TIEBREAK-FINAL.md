# P12b2 — game/set result board and tiebreak point show

Ported to `crates/hst/src/play/popups.rs` (`board`, `tiebreak_points`), fed by `hst_sim::flow`'s show and two new
`Score` fields (`set`, `set_games`: the result board's per-set games, a tiebreak set counted +1 on the previous set
as the original's history at `38b820`).

## Game / set (kinds 3/4, update `3876c0`/`387ac0`, start `387910`/`387d10`, draw `387540`/`387990`)

- Fade in over 14 (alpha `128 − 128·(t+1)/14`, RAM t=7 → 55), out over 5 (`128·(t+1)/5`, t=2 → 76).
- Phases: fade-in → 0 at scale 1; rise (stage 0, n 0..6) → 0, scale `1 + n/6`; drop (stage 1, n 10..0) → 1, scale
  `1 + n/10`, white copy alpha `(n<<7)/10`; settled → 2. 6 / 10 are `ScoreboardTiming.game_rise/drop`. Phase 0
  shows the scorer's old count (games − 1 in the current set; sets − 1 in set mode). Set draw returns at once when
  the match is over.
- Board `3a7890` (mode game / set): frame x0 152/88, middle width 304/432; 3 frame columns (u 0/16/40) of
  v0 16×16, v16 16×8 stretched to edge_h, v24 16×16; centre (65,41,14,14) stretched over (248/184, y0, 144/272,
  centre_h); separators (64,9,8,22) at x 248, 390 (+184, 454 for set). Sets to win 1–2: rows 6, edge_h 80, top 320,
  y0 328, centre_h 96, diagonal y 368; else 10, 144, 256, 264, 160, 336. Stripes (1+16p, 40, 14, 16) → 88×16 per
  row at x 160/392 (96/456); doubles rows/2 each (p > 1 offset rows·8) plus the diagonals (0, 56/80, 88, 24).
- Plates `3a72d0`: pill gameset00 (80,0,48,48) at x 164/432 (100/496), y 328/380 (short) or 272/372, midpoint
  for singles; face 64×64 at +2,+2; slot label inpane_p at label x 208/392 (144/456), y+6, panel colour.
- Games: columns 1/3/5 (sets to win 1/2/other) from y 360/328/264, step 32, team x 272/336; current set on
  gameset02 (white copies at v 32), others gameset01; a set's loser at half alpha; dash (320,0,24,32) at 308.
  Set mode: sets won on gameset03 (48×48 cells, white at v 48) at x 192/400, y 344/312, "Set" (288,0,56,24) at
  x−4, y+48. Scaled quads grow about their centre. Flush order gameset00, 01, 02, 03, then faces/labels.

## Tiebreak point (kind 6, update `388ec0`, start `3891d0`, draw `3883b0`)

- Plates as the point show. Points tiebreak00 64×64 cells (col 0,1,2,3,0,1,2,3,0,1,0 / row 0,0,0,0,1,1,1,1,2,2,3;
  8 Deuce, 9 Advantage, words 128 wide for the scorer with advantage) at x 336, y as the point show.
- Roll: the scorer's old points slide up 3n (stage 1..2; RAM shows n running on 8→11 through the swap, ported in
  `flow.rs`), alpha `128·t/3` over the swap, 0 after; new cell from the swap on with tiebreak01 white copy
  `128 − 128·t/3`, then `128·t/15` over the settle. With advantage the other team at a/2 (fade) or 64.
- Banner tiebreak02 (0,0,256,64) at (176,32) last; the original's rectangle is 256×128 but the lower half is clear.
- Deuce: the point show's deuce code on duce01 (red).

## Found on the way

- `ImageNode` defaults to an aspect-keeping mode: every stretched quad (frame edges, centre, and the point show's
  squeezed roll) drew at its own aspect. The pool now uses `NodeImageMode::Stretch`.
- Standalone TM2 sheets (INPANE, menus, effects, prizes: all but 7 of ~580 on the disc) author alpha 0–255; the
  board's centre texel is 0x80 and the PS2 shows the court through it at half. `tim2::decode_alpha8` keeps alpha as
  is; the INPANE loader (`panel::image`) uses it. The model/MTL path keeps the 0x80 = 1.0 expansion.

## Verified

`research/popup_shots2.py <slot> <frames> <outdir> [g0 g1]` (pokes games/history, sets the tiebreak flag at 6-6,
F8 at named stages, stops at match over): `context/shots_p12b2/` game 5-5 → 5-6 doubles best of 3 (in, rise3,
rise6, drop8, drop3, settled, out) and `tb/` tiebreak 0-0 → 0-1 (slide, swap 3..0, flash, settled, out). Screens
lag RAM by one frame. Ours with a temporary start score (`context/shots_p12b2/ours/game_25.png`, `tb_23.5.png`):
board frame, stripes, plates, digits and translucent centre match; tiebreak banner, digits and plates match.
Unit tests `game_board` and `tiebreak` pin the PS2 coordinates and alphas.

## Open

- No set show captured (the sets-to-win poke didn't extend the match); set mode is from the decompile only.
- The "Game / Set" + "Server / Receiver" banner over the board is a separate overlay (`3899d0`, started by
  `3899a0` 45 frames into the pre-show wait): flag sb+0x571, timer +0x574 = 14, stage +0x578. Word 1 result_game
  or result_set (0,0,144,64) at (x1, 32), stage 0 highlight (0,64) alpha `t·128/14`; word 2 verRed if team 0 won
  else verBlue, (0, recv·64, 256, 64) at (x2, 32), highlight (0, (recv+2)·64) alpha `t·128/16`, stage 1 timer 16,
  stage 2 hold. Receiver x1 124 / x2 276, server 148 / 300; recv = ((server player 0 or 2) ^ 1) ≠ winner team
  (server DAT_0042304c). Cleared by message 0xd/0x18 in `383350`. → P12b5.
