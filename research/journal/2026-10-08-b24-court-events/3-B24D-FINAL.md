# B24d court 4's passing ball, routes 1–3 against recordings — FINAL

## Recording
- `research/b24d_ends.py 3 9`: slot 3 (court 4, P1 human + 3 bots, 0-0) with ○ every 1.5 s until game 1 ends;
  scratch slot 9 saved there (state 3, change of ends next, ball not yet played).
- The roll can't be varied through the court generator's index: entering state 1 (message 0xe) the court MT is
  reseeded from newlib `rand()` (P3b), so every load rolled the same (no play). `b24_rec15.py -9 700 out.bin <lcg>`
  now sets the low word of `rand()`'s state (`*(0x1b80f0)+0xa8`) after the load lands (a write right after
  `load_state` is lost), and with a negative slot loads mid-match without ✕ and clears the played flags.
- Sweep lcg 1–38 (`context/b24d/m<lcg>.bin`): route 1 at 6/10/13/29/38, route 2 at 37 only, route 3 at
  3/20/21/24/26/28/30, route 0 at 8/9/22/34, the rest no play (1 in 2, as the coin).
- Fixtures `context/fixtures/ball_c04_r1.bin` (lcg 6), `_r2` (37), `_r3` (3): roll at vsync 9552, 699 ticks each.

## Test
`passing_ball_matches_the_game` runs over `ball_c04` (route 0) and `ball_c04_r1..3`: every tick bit-exact,
including the route matrix (normalize((p1−p0).xz), cross(up, fwd), × rot_y(π), at p0), the four bounce sounds at
each route's points (0x41ccd0) and the fade; each file must roll its own route. Dropping the × rot_y(π) fails
it at the roll (vsync 9552).

## Not verified / not 1:1
- The `rand()` state was poked to pick the route (the game's own code then reseeds, rolls and plays it); a route 1–3
  from an unpoked match wasn't recorded (needs a lucky match; the code path is the same).
- The drawn ball on routes 1–3 not compared to screenshots (rendering, B24a checked route 0 by eye).
