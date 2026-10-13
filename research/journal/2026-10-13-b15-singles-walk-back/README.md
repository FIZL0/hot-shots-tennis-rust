# B15 — CPUs sometimes don't hit the ball

## Repro
Headless all-bot singles (`HST_AUTOPLAY`, chars 0/1, court 1, 54000 ticks), points ending with an unreturned ball:

| | rally points | unreturned | avg shots | short (≤4 shots) |
|---|---|---|---|---|
| before | 84 | 58 | 4.3 | 38 of 60 |
| net reach only | 81 | 57 | 4.9 | — |
| both fixes | 68 | 44 | 7.9 | 12 of 44 |

Before, the bot that just hit walked forward and stood pinned at |z| ≈ 1.5 (at the net), so the next ball (lob or
pass) went past it.

## Causes
The ported rally routines (`hst_sim::rally`) are bit-exact against recordings; the app fed them wrong state:

1. **Walk-back spot never set.** The original's singles point set-up puts the spot the AI returns to after its own
   shot at x = 0, z = side × −10 (NET style) or side × −11, and clears dash/short/hold/held. The app never ran it,
   so `Rally.spot` stayed (0,0): `singles_rally` walked the bot to the net every time.
2. **Net reach unset.** `Rally.reach` (the AI object's net reach, 3.2 in every recording) came from nothing (0), so
   dashes targeted z = 0. Now taken from `hst_sim::ai::Choice::new(..).reach` (level 3 → 3.2).

Fix: `crates/hst/src/play/doubles_ai.rs` — set-up on the bot's first step of each singles point (re-armed by the
serve message 0xe), reach set every step. Test `singles_bots_go_back_to_the_baseline` (fails with 170 pinned frames
without the fix).

## Comparison with the original
PINE poll of slot 5 (bot doubles, 37 points, `context/b15/orig_s05.txt`): ~5.6 shots per rally, points also end on
far-away dives and deep balls let go. Singles fixtures (`ai_rally_singles*`) show 5–16-shot rallies. App doubles was
unaffected by the fixes (unreturned points avg 6.9 shots). Remaining app misses (15 no contact found, 17 futile
dives, 12 found + pressed) are the same kinds the original shows.

## Not verified / not 1:1
- The set-up runs on the bot's first AI step of the point, not inside the original's point set-up routine (same
  values, set before the AI reads them).
- Not ported from that set-up: the +0x60 flag, the first-point reaction fields, the aim-stick reset, the shot-history
  reset and its timing draw — the app already has its own equivalents/draws; not checked one by one.
- No same-match recording comparison app vs original; only aggregate rally lengths (the app can't load slot 5's
  exact match state).
- The 12 "contact found and pressed but missed" points were not investigated (press/contact mechanics, not AI choice).

## Tests adjusted
Two singles tests depended on the old bot play: `a_replay_skips_then_slows_in_frames` took the first replay, which is now an
ace's (one shot, no skip to pace), so it waits for a rally's replay. `receiver_gives_up_a_net_touched_serve` needed a
longer run to see a net-touched serve, and now also stops when that serve's fault ends the game (the players are placed
for the change of ends while `shots` is still 1).
