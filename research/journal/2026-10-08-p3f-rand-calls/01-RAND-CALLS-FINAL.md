# The rand() calls

## Logs
- `research/p3f_rand_log.py <slot> <frames> out.bin` patches rand()'s entry with a stub that logs vsync, caller,
  s0, gm+0x54, the state's low word, and the match frame. `context/p3f/rand_s05.bin` is slot 5 over a change of ends.
- `research/p3f_menu_log.py <char> <slot> out.bin [dir...]` logs from save slot 2 (the doubles character select)
  through to the first serve, then saves it. On pad copy 3, ✕ confirms and ○ goes back. `menu_c1.bin` and
  `menu_c2.bin` are court 4, clear. Slots 8 and 9 are now games where two players have the same character.
- `research/p3f_setup_seq.py` prints a menu log's calls by caller. `--fixture` writes `context/p3f/setup_rand.bin`.

## The menus
There were 4563 calls before the seed in c1 and 5794 in c2, for the same picks. Some menu screens draw once per
frame they are up, so the count follows timing. main.rs's clock-picked count is therefore the faithful stand-in.
Porting the menus' draws is only worth it if the app gets those menus (P3f1).

## Setup sequence (seed → first point, 9608 calls, identical in both logs)
1. the seed (the weather MT)
2. lens flare made: 48 (24 rays × brightness + angle jitter)
3. sound manager reseed #1
4. clouds: 5 × count. The count is `layout::cloud_count` with env 1 for singles and 0 for doubles; 0 gives 20. Doubles
   make them too: court 4 → 20, court 10 → 9, court 1 → 24, court 2 → 20.
5. the effects seed: 1 draw. The effects LCG (×0x343fd + 0x269ec3) restarts from it at each new point. Slot 8 shows
   0x614e839d.
6. sound manager reseed #2
7. the intro: 394 flare ticks × 24 (court 4, doubles, clear)
8. new point: shared, court

## In match
Each game tick draws 24 (the flare) while weather < 2. A new point draws shared and court. After a played point, a
sound reseed follows (none at the match's first point). `rand_draws_like_the_game` replays rng_s05: 1495 of 1498
frames are exact, the rest are torn samples, and both new points are exact.

## Voice banks (0x345ad0)
Each player always makes a draw (`r15 % 100 >= 70` → b). The rule then looks at the players before them with the same
character:
- two: the second takes the other bank
- three: the third takes the other of its partner's (i^2) when the partner is one of them
- four: the third takes b if the first two both have a, and a if both have b; the fourth takes a when exactly one of
  the first three has a, else b
- otherwise the player keeps their draw

Court 7 loads VCE n+2/n+3 (.hd c/d). This is checked against slots 8, 9 and 5 (`setup_draws_like_the_game`). With the
rule removed, slot 9 player 2 fails. The four-of-one cases are unrecorded (P3f4).

## App
- main.rs: `Rngs::setup_rand(cloud_count)` after the weather schedule. The effects seed goes to `play::set_effects_seed`.
- play.rs setup: reseed #2, then `INTRO_TICKS` flare ticks when the first game's weather < 2, then new_point. The
  `flare_tick` FixedUpdate system draws 24 per tick while weather < 2.
- rain.rs: the effects LCG restarts from the match's effects seed at each serve.
- Tests: `setup_rand_like_the_game` recovers the full state from boot by the low word and checks the effects seed and
  the first point's state for both logs. `rand_draws_like_the_game` and `setup_draws_like_the_game` cover the rest.
