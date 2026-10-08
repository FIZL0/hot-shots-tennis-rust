# The intro's length

## Recording
`research/p3f2_intro_log.py <slot> out.bin <vpad route>` hooks rand() as research/p3f_rand_log.py does, drives the menu
route, presses nothing once the match loads (a press may skip the intro) and prints the court (0x422f90), the players
and the intro's flare ticks (frame gm+0x58 of the last phase-0 flare tick). On pad copy 5 ✕ confirms, ○ goes back.
- Doubles: slot 2 → ✕ ×4 (Ashley + the COMs' first picks) → Start → confirm screen (default court 5 for these picks).
- Singles: slot 1 (the doubles controller screen) → ○ (singles/doubles) → left, ✕ → ✕ (1P = 1-A), ✕ (2P = COM),
  ✕ (Ashley), ✕ (Cody) → Start → confirm screen (default court 8).
- Another court: on the confirm screen down, ✕ (Select Court), left/right ×n, ✕, up, then ✕ (Start the match!). The
  list moves aren't a simple ring (some counts land on the same court); the court is read back after each run.
Logs are in `context/p3f2/` (by court: `by/<court>_<players>_*.bin`).

## Result
| court | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| doubles | 364 | 394 | – | 394 | 446 | 394 | 394 | 394 | 394 | 394 | 354 |
| singles | 364 | 394 | 394 | 394 | 446 | 394 | 394 | 394 | 394 | – | 354 |

The length follows the court only: equal in both modes wherever both were recorded, and the same over repeated runs
(court 5 doubles twice, court 11 doubles twice, court 8/9/2 singles several times). Court 3 never came up in the
doubles list runs and court 10 never in the singles ones. All recordings were clear (flare ticks = intro frames).

In code the intro phase (match phase 0, 0x324810) ends when a timer at `*(*(gm+0x84)+0x138)+0x3c` runs out, a streamed
sound (0x167310) is done and, in doubles, the intro presentation (0x43b1d8+0xd94/0xd95) is over; its per-court length
is not read from data here — it's the court intro of B42.

## App
`Rngs::intro_ticks(stage)` (rng.rs): 364 on court 1, 446 on 5, 354 on 11, 394 elsewhere; play.rs's setup ticks the
flare that many times (weather < 2) instead of the old court-4 constant. Test `intro_ticks_like_the_game` (rng.rs) on
`context/p3f2/intro.bin` (`p3f2_intro_log.py --fixture`, 20 logs): from the intro's first flare tick the court's ticks
land on the first point's shared reseed for all 20. With court 5's length off by one it fails.

## Gaps
- Courts 3 (doubles) and 10 (singles) are assumed equal to the other mode; the length's source (the intro's timer /
  presentation per court) isn't ported, only its recorded totals → P3f6.
