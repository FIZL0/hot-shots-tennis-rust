# P3d4 serve call

## Where 0x10 comes from

0x3522b0 (the server's serve state change) on state 2 (the toss) sets motion 0x23 (0x24 underhand), then
broadcasts 0x18b310(gm, 0x10, kind, 0) with kind = 0x35d1f0(toss): 1 → 1 (strong), 2 → 0 (weak), 4 → 2
(underhand). Every player object's 0x348a00 gets it; in doubles a player with +0x3b96 unset, not the server
(0x42304c), of the server's team, whose team is 0x38afd0's answer, on kind 1 calls 3553d0(p,6,2,2,-1) (keys 2..2,
no memory slot: one shared draw) and sets +0x3b96. Right after the broadcast 0x3522b0 does the same for the singles
server itself (players == 2, same flag, same call, ra 0x352704). Three players: nobody.

## 0x38afd0(hud): the match point

0x38ae80(hud, t) is `Score::game_point` (0x316620 deuce, 0x316628 advantage, 0x31661a tiebreak, 0x4230bc deuce
off; points 0x423064). 0x38afd0 takes the first team (0, then 1) at a game point, else -1; in a tiebreak with
hud+0x460 + 1 == 2·sets − 1 (the last possible set) that team; else -1 when games 0x42306c[t] < hud+0x54 (games)
− (1 if t leads in games) or sets 0x423074[t] < hud+0x50 (sets) − 1; else t. Ported as `Score::match_point`
(hud+0x460 = `Score::set`).

## +0x3b96

Only 0x3449f0 (the placement on 0xe/0xc/6) writes it besides the call: while 0x423040 (no serve struck this
match) it is cleared (with +0x3b94 and the voice memory), so once per match per player. When gm+0x344 is set it's
restored from +0x3b97 (saved at each other placement) — that is the instant replay re-running a point; the app has
no such replay, so it is left out.

## Live check (research/p3d4_serve_call.py, copy 3)

Pokes the score after the load (points 3-0, games and sets one short) and logs the shared draws until the server
has tossed.
- slot 5 (doubles, server 0, strong toss), team 0 at match point: placements ×4 at 7566, then one key draw at 7689
  (ra 0x35546c); player 2's +0x3b96 0 → 1.
- same, score as is: no extra draw, no flag. Receiving team at match point: none.
- slot 8 (singles bots, HST_V0=5030), server 0 at match point: placement, first-serve key (0xe), placement, then
  the toss key at 5464; player 0's flag set.
`serve_calls_like_the_game` reads the four files and checks flags and draw counts against the rule.

## Port

`serve_call` in play.rs runs at the toss press (serve_turn), clears with `first_serve` in `placement_draws`; the
play goes on `whooshes` as the other reaction voices.

Not done: the weak/underhand toss case was not recorded live (bots tossed strong); the rule is read off the
decompile. `startled_creatures_match_the_game` (npc.rs, untouched here) fails in the full check run.
