# P0b4b — serve placement (port: `hst_sim::flow::serve_placement`)

Player placement `0x3449f0`. Called from the player's message handler on 6 (phase 0 enter), 0xc (phase 1
enter; also sets +0x3b94 = 1) and 0xe (phase 2 enter), and once at construction. Skipped when count > 1 and the
placed flag +0x13f5 is set (set at the end of placement, cleared on 0xf = phase 2 exit) → a serve entry right
after change ends keeps the change-ends placement.

- Draws one RNG value (`19f5c0(gm+0x80)`) at entry, always. Doubles + first point (0x423040): formation +0x13f4
  from the AI data (+0x80 → +0xc → +2 / +0x11, 20% → 0) depending on 0x422fc8[i] < 0x20 — AI/P0c.
- Stance: +0x1408 = +0x140c (0x423040 set: both = 3.0). +0x140c = |x| when the serve motion enters state 2
  (toss, `3522b0`). Practice modes (count < 2, 0x3167ac 0..3) use other spots — P21.
- Facing +0x12b0 = ±1 by player parity, × −1 if 0x422f99 and count ≠ 1, × −1 if ends swapped (0x4230b4).
  Matrix +0x3d40 rows (f·s,0,0,0), (0,1,0,0), (0,0,f,0), translation +0x3d70; scale +0x12b4 = 1 in the match.
- Court sign c = side 0 ? 1 : −1. Server: (c·stance·f, −12.25·f). Same team as server: (−2.7425·f·c,
  f·(−7.9 if formation 2 else −3.2)). Receiver (0x423054): (3·f·c, f·(−12.25, or −11.0 if faults 0x316608)).
  Else: (−2.7425·f·c, f·(−7.9 / −5.2)). Singles: receiver index on the ad side is 2/3 → absent → "else" spot.
- Message 0xe: count == 2, +0x3fa4 == 1 and (first point or (+0x3b94 and RNG % 100 < 50)) → `3553d0(6,0,1,2)`
  (a pre-serve motion; P8).

Test `tests/score.rs::match_s05_serve_placement` (slot-5 doubles): 37 set-ups × 4 players bit-exact. Mutating the
second-serve depth fails at vsync 14555.
