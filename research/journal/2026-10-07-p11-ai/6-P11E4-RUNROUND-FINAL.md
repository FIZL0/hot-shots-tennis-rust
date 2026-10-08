# P11e4 — strong-side run-round (FINAL)

Port: `AiParams::run_round`, `AiParams::run_round_width`, `stand_side` (crates/hst-sim/src/ai.rs). Test
`stand_sides_match_the_game` (crates/hst-sim/tests/ai.rs) against `context/fixtures/ai_side_s05.bin`: the width bits
equal `run_round_width`, and `stand_side`'s choice equals the game's for every decision — 61 sides checked bit-exact,
5 run-rounds, 9 passed rolls that kept the nearer side.

## The original

Both AI contact searches (the receive search with tiers 0..3, and the body-shot search used only for BASE-style rows)
pick the best path entry on each side of the ball: "minus" stands at ball x − reach·side, "plus" at ball x + reach·side.

- Both found (both indices > 0): stand x = x ∓ mul(reach, side); stand z = z − div(mul(player depth field, side), 2);
  distance = madd(mul(dz,dz), dx, dx); minus is kept when dist_minus ≤ dist_plus.
- If the farther side is the strong side (player hand from TParam: 1 = minus/right-hander's forehand, 2 = plus; it
  ignores the select-screen hand toggle — Carol toggled right-handed still has 2), and the roll passed, and
  |strong stand x| < width (strict), it keeps the strong side.
- Width = add(singles ? 4.115 : 5.485, row 0x7c extend). Roll = MT draw, (r>>16 & 0x7fff) % 100 < row 0x78; drawn on
  every search call, then voided for a doubles AI beside a human.
- Only one side found: minus if its index ≥ 1, else plus.
- The receive state calls the search every frame of its first substate until it finds the ball, then stores
  stand/contact/frames/kind and does not search again.
- Slot-5 rows 84/85/86/89: strong_side_rate 10/95/10/10, extend 0/0/0/2.0; all four bots strong = 1, none beside a
  human.

## Recording

`tools/record_ai_side.py <slot> <frames> <out.bin>`. There is no RAM trace of the decision, so it patches jumps into
the running game (stubs in free RAM 0x1e00000, buffer 0x1e01000..) before the decision and at each search's return;
0x50-byte records (format in the script docstring); hooks are removed at the end. Slot 5, 18007 frames: 156 records
(34+27 pre-decision, 63+32 returns) → context/fixtures/ai_side_s05.bin.

## In the app

`play.rs`: `ai_stand_x` — the bot's goal x = ball x ∓ reach·end per the chosen side (replaces the old 1.1 offset);
`Player` field `ai_side`.

Ponytails / open: the app's stand-in intercept has a single contact point (both spots are the same ball point, depth
0), so distance ties go minus. The app rolls once per shot (cached by shot count) with its xorshift rather than on
every search call on the MT stream. True parity waits on the real intercept path search.
