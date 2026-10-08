# 1 — The "second aim writer" is the serve aim; the 4 dropped aims are instant replays

## What 0x353000 is
Only three functions write P1's +0x3e90: 0x34ccf0 (the rally stick aim), 0x353000 and 0x3533d0 (the toss apex,
serve state only). The launch functions read it. 0x353000 is reached from the top player update 0x3467b0 only in
player state +0x3fa4 == 1 (the serve; 0 play → 0x349410, 2 post-point → 0x354050), through 0x351dd0: with the
sub-state +0x3fa6 == 3 (the swing) its countdown +0x3ec4 runs down, and at 0 with no miss (+0x3ec8 == 0) it calls
0x353000(player, stick, flags), then the swing setup 0x35b030, and sets +0x3ec4 = −1 (+0x3ec8 set: the miss
handler 0x350840 instead). So it is the serve's aim, not a net-volley one. It was ported approximately as
`serve::target` (P-serve work); this task makes it exact.

## 0x353000, bit for bit
- toss class c = 0x35d1f0(+0x3ea0): 4→2, 2→0, 1→1 (strong), else 0. sweet = c == 1 || |+0x3fa0| < 2.
- width = 2.0575 (off-sweet `sub(2.0575, 0.5)`, not the old port's 1.56), half = `div(3.4 or sub(3.4, 1.0), 2.0)`.
- base x = `mul(2.0575, end)`, `mul(x, −1.0)` when the global 0x423050 (the port's `score.side`) is 0; base z =
  `mul(add(3.0, half), end)`; y, w 0 (0x1cc310 is the zero vector).
- the rally aim's core 0x34c250 with button = flags & 0x20 (0 in every recorded human serve), drop 0: stick mapped onto the square,
  base added (`add.s` offset + base, as in the rally), the angle (players == 4: +3) + Serv CON +0x130c (the
  branch byte +0x3ec1 is 0 at the serve swing; recorded), body/rising/incoming not taken, no straight-depth
  rescale, 3 m minimum, the ±5/±10 nudge +0x3ed4 (2 RNG draws). `shot::aim` is split: `aim_from` is that core
  from a base, width and half; `aim` builds the rally base and calls it (the rally fixtures still pass).
- then +0x3f18 = +0x3f10 = 0 (+0x3f14/+0x3f1c are left stale: they read as garbage in the recordings). If c == 1
  and the grade byte +0x3ee8 is 3 or 4 (asm 0x353140..0x3533a8):
  - sign = +0x3fa0 < 1 ? −1 : 1; `3f18 = madd(adda(0, 3f18), 0.01, (float)(sign·T[lvl].0))`;
  - stick length `sqrt(madd(mula(z, z), x, x))`; > 0: each component `madd(adda(0, old), ((s·inv)·end)·T.1, 0.01)`
    (the stack copy is the normalised stick × end, then × T.1, the madd's 0.01 last);
  - else one RNG draw (bit 16 set → −1): `3f10 = madd(adda(0, 3f10), 0.01, (float)(sign·T.2))`;
  - all four × 0.6666667 (`mul.s`); then +0x3e48 = 1.
- T = 0x3fc760, 12-byte rows by the skill level +0x12d0: (50, 30, 100), (100, 60, 150), (150, 100, 200). The level
  is set at player init from the character +0x12bc: 0..2 and 5 → 0, 8..11 → 2, else 1 (checked against the
  recordings' +0x12d0). `exe::Game::serve_miss` reads the table, `serve::miss_of` picks the row; play.rs
  `serve_character` sets it and the serve angle (TParam Serv CON, column 22) per character instead of character
  0's values.

## Recording
`tools/record_aim.py` AIM_SERVE=1 (slot 3: doubles, P1 serving at the start): serving, it tosses with the next
button (✕ mostly), holds the next stick and presses again a cycled number of frames later. A pair is kept on the
frame the serve countdown ends without a miss (pre +0x3fa4 1, +0x3fa6 3, +0x3ec4 1, +0x3ec8 0; cur +0x3ec4 −1).
A swing press before the ball is in reach takes the earliest frame (offset −6); after it the offset grows quickly,
so most serves came out −6 or 11/12.

## The 4 dropped doubles aims (vsyncs 35354, 35372, 61634, 61657)
Not a second writer: messages 4/5 to the player save/restore its state with +0x4088 = 1 and re-run 0x349410 with
the replay's pad (gm+0x70 instead of gm+0x60, in 0x348ff0), so 0x34ccf0 re-aims from the replay's recorded stick
while the sample's live pad holds the vpad's current stick. Fitted: 35372 and 61657 are the lob aim with stick
(0, −1) (x = −0.0079·5.485 = −0.0432 from the vpad's 0x80 centre, z = −3.0), 35354 a ground lob off-sweet with
stick (0, 1). record_aim.py now skips every aim while +0x4088 is set (rally and serve), so they are never
recorded; the rally fixtures already leave them out.

## Fixture
`context/fixtures/aim_serve.bin`: 19 serve aims from slot 3 (two runs, AIM_SERVE_DELAY 64,66,62,68,58,72,65,63,67,70
then 69,70,71): strong (12 with nudges, the stick path and one centred-stick random path), weak and underhand,
off-sweet (offset −6, 11, 12) and grade 2, both service boxes, angle-limited aims. `human_serve_aims` (aim.rs) gets
every aim, both nudges and +0x3ed4 bit for bit; the coin flips are read back from the recorded results. The press
timing is wall-clock (the fifo), so a 1-frame delay change jumps the offset; no sweet strong toss came up.
