# B30: background NPCs facing the wrong way

## Which
The walking spectators, on every court, in every state. Every walker's layout yaw is ≈ 0 (all courts 1–12), so
the port drew them all facing +z. The original turns them at runtime:
- **Serve placement** (message 0xc, also 6 and the walkers' reset, `0x39ece0`): forward = (0 − x, 0, 0 − z) from
  the home position, normalised.
- **Reacting to a point** (mode 1 tick, `0x3a0ba0`, a1 = 1 in the usual branch: `li a1, 1` at 0x39f31c before the
  mode test): forward toward (0, ±6.4): +6.4 when the winning team's (0x4230a8) first player (match +0xa8 + 4·team)
  stands at z ≥ 0, else −6.4.
- Rows (up × forward, up, forward, home), the drawn matrix is +0x120 (`0x3a09e0` → `0x14c5b0`). Floats:
  `div(1, sqrt(madd(mul(dx, dx), dz, dz)))`, then muls; the cross product via mula/msub.
- Trigger creatures: in all four RAM images their current matrix (+0x1b0) equals their home (+0x70); the port draws
  them at home, so nothing to fix there (moving creatures standing still is B19's known gap).

## Port
`npc::walker_facing(home, z)`; `play/npcs.rs` faces walkers to the centre at setup and at each new point's serve,
to the winners' half on a decided point (`g.post_winner`, the team's first player's z).

## Proof (tests/npc.rs)
- `creatures_match_ram_on_four_courts`: every walker's drawn matrix in the RAM images (courts 1, 2, 4, 10) is
  `walker_facing(home, 0)` bit for bit.
- `walkers_face_like_the_game` on the walker recordings: court 10 8 reaction turns + 4 back to the centre, court 2
  16 + 16, courts 1/4 none (idle); bit-exact. A sample caught mid-reset (object zero-filled, 5 of ~50000) is skipped.
- `context/b30/port_3.png` vs `context/b19/orig_s05.png`: court 10's left walker now faces the court as in the original.

## Not here
Walker movement (dodge, collision) stays P14e; the 0x4230b8 = 2 branch (favoured side) picks player 0 or the
winner by the judge, not ported (never 2 in the recordings).
