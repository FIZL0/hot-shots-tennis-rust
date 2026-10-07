# N1c — Motion state machine (FINAL)
Top update 3467b0: state +0x3fa4 0 play (349410 + 350f70), 1 serve (351dd0; motions in 3522b0), 2 post-point (354050).
- Stroke (34afc0 mode 2): see hst_sim::motion::stroke_start; countdown in 349410 switches +0x3e40 at +0x3ec4 == 8.
- Soft follow: set at contact (end of the hit in 3467b0) when |ball +0x140|² < 0.2143347, swing 0x10..0x15, branch 1;
  350f70 next frame plays 0x1c/0x1d from frame 8 (side +0x3f50 bit 2, xor lefty).
- Whiff 350840: 0x10/12/14/16/18→0x27, odd ground/volley→0x28, 0x1f/0x25→0x29, 0x26→0x2a.
- Serve 3522b0: walk 0x22 if stick.x·fwd > 0 (lefty swaps), toss 0x24 (kind 2) / 0x23, swing 0x26/0x25 at 8/n.
- Reactions 354940(0): 0x3db0 = 0x2b body hit (mode 3); <2 players 0x2f/0x2e; winner team (0x4230a8) 0x2c / 0x2e if
  0x4230b8 (game end), losers 0x2d / 0x2f. If gm+0x35c == 0, base 0x2c/0x2d and +0x1404 > 0: candidate co ids from
  0x3fc790 (A even chars 0-4, B odd 2-4, C char5 0,1,3,4, D chars 7/11 3,4) minus those of players with lower index,
  k = (rand>>16 & 0x7fff) % (n+1), k < n → 0x30 + id. Motion numbers ≥ 0x30: skeleton clip = number (co clips loaded
  by 3650f0 from data/taguchi/MtGrl/re_pc00_coNN.ANI2), face MOR/UVA index + 0x1404 (5 = "_2" variant) unless the
  player's team == 0x4230a8.
- 354050: plays 0x3db0 once, root motion from the *_dummy clips, voice triggers 3553d0, ends at motion end.
