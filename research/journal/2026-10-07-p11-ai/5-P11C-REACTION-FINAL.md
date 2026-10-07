# P11c: reaction delay (FINAL)

Port: `AiParams::reaction` and `CHOICE_DRAWS` (crates/hst-sim/src/ai.rs); `guess` no longer drops draws itself. Test
`reactions_match_the_game` (crates/hst-sim/tests/ai.rs) against `context/fixtures/ai_pos_s05.bin` (it has the players'
positions): 52 reactions of the slot-5 bots exact (4 after a smash, 23 near the net), right after the timing draw and
the 7 shot-choice draws; flipping the near-net test makes it fail. `guesses_match_the_game` now draws the reaction
before the guess and still passes. Not exercised by the recording: lob and special-serve parts (no △ lob or special
serve seen by a bot in it), and the doubles columns (all four players are bots).

## The original (doubles offsets; singles 3cbbe0 the same at −0x10)

- In the per-shot draw 3d24e0, after the timing errors and the shot-choice draws (quick serve, dive only at a reset or
  after its own dive, body, low, four level picks): if the opponent didn't hit last (0x423058 vs player index & 1),
  +0x248 = 0 with no draw.
- Else, if the hit message's last shot kind (+0x134) is 4 (smash): row 0x54 (after smash) + rand % (row 0x64 + 1).
  Otherwise fast-ball frames (P11d's) + rand % (row 0x64 + 1) + the base: AI +0x24 when |player z (+0x3d78)| ≤ 6.4,
  else AI +0x20.
- AI +0x20/+0x24 are set at reset (35ef00): beside a human partner (partner +0x13f0 < 0x20) the doubles baseline/net
  columns (row 0x48/0x50, with the doubles centre numbers and +0x28 = 1, the smash-third flag); else the singles ones
  (0x44/0x4c).
- Then on a hit message: kind 1–3 with record +8 == 3 adds row 0x68/2 + rand % (row 0x68/2 + 1) (lob); kind 0 with
  record +0xd adds the same of row 0x6c (special serve). Halves truncate toward zero.
- Shot record (built in 3467b0 at the strike): +4 kind = the hitter's branch +0x3ec1 (0 serve, 1 ground, 2 volley,
  3 dive, 4 smash; P11d's note had 3/4 swapped), +8 from 35d180: 3 when the press button +0x3ea0 is △, else 2 for code
  8, 1 for ○, 0; +0xd the special serve.
- A guess replaces +0x248 with its move frames.
- Use: receive state 3cf7b0 and both rally routines (3d02c0/3d1320) in substate 0 count it down and do nothing else
  while > 0 (the rally ones only from the second shot); in substate 3 (own stroke, contact frames < 0) they count it
  down too when the opponent hit last.

## In the app

`play.rs`: `ai_heard_hit` takes the kind straight from the branch (dive 3, smash 4) and the lob from the hitter's
△ press; `ai_draw_guess` skips the 7 choice draws, draws the reaction into `ai_hold`, then the guess; `ai_draw` with no
opponent shot clears hold and guess. `bot` counts the hold down during its own swing too; `ai_guessing` now holds on
any opponent hit in the rally (not only when an intercept is found). Also fixed a merge leftover in `bot`: a second
`let wait` dropped the guess target and called `ai_wait` twice. P11e2's singles net dash checks record kind 3 as the
original does; with the kind fix that is now a dive, as in the game.

Ponytails: special serves don't exist in the app (never drawn); the rally routine's "no reaction count before the
second shot" for the serving team's partner isn't modelled; rolls use the app's xorshift.

Smoke run (`HST_AUTOPLAY=1 … --play`): rallies with reactions play; after a couple of points the bot server re-tosses
for ever because `serve::search` finds no contact frame on the predicted toss path at all (None, not just later than
`bot_due`). The HEAD build without P11c stalls the same way after 2 points with other generator seeds (0x1357_9bdf,
0x0bad_f00d), so it is the serve stand-in's (B9's area), not the reaction.
