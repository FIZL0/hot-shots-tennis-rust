# The replay decision (port: `hst_sim::instant::decide`)

The match handler's point message (0x17, `325cd0`) runs once per decided point in a match. Practice (under two
players) skips it. It first calls the scoreboard's point award (`38b820`, `hst_sim::score`). That returns 0 only
once the match is over, and then nothing else happens. Then the decision runs:

- The recorder flag `allowed` (gm+0x346) is set unless the rally cancelled it (gm+0x347). No decision while a
  replay runs (gm+0x344), or with no last hitter.
- No replay for: rally block state 3; fewer than 3 shots with rally state 0; a return (2 shots) not hit by the
  receiver; a double fault.
- `recorded` (gm+0x32f) is set when allowed and the recorder holds a shot. Past here the last hitter must have won
  the point (the rally block's point winner).
- **Replay** when: the match was won and recorded; otherwise not when the ball's state is 4/5, nothing is recorded or
  the option (0x2ef2b6) is 0 (attract play skips this check). Then, for a human hitter (pad under 0x20), on a
  game/set point (scoreboard event > 0) with a plain verdict: an ace over 100 km/h, a special shot (ball kind 3,
  sub-kind 0), a winning return, or kind 1 over 90 / kind 2 over 70 km/h. With option 2, on a plain point by a human:
  special shots, aces, returns and line calls of a near-perfect shot (|timing| ≤ 1, kind 1/2, sub-kind ≠ 3,
  ≥ 100 km/h), and rallies of 10+ shots within 900 ticks with a fast kind 1/2 shot.
- A replayed line call (`327000`: the last two bounces within 0.1 m of a line or on both sides of one) after 3+
  shots, not special, also clears the post-point camera's pick (gm+0xbc+0x84 = 0).
- Speed: `375d10`, sqrt(mula y², madda x², madd z²)·60·3600/1000 (from the disassembly).

## Check

`crates/hst-oracle/tests/instant.rs` writes 100,000 generated points into the slot-5 dump. Each point covers the
match object, ball, recorder, rally block, the judge's verdict, the ball path's last two bounces and the globals.
Bounces are biased onto the lines. The test calls the original's `325cd0` with the award stubbed to return
`awarded`, then compares replay (gm+0x32e/0x350/0x35c), the pick reset, `allowed`, `recorded` and gm+0x376.
All 100,000 points agree, about 3.4% of them replays. The first run caught one difference: the kind test
`kind − 1 > 1` is unsigned in the game, so kind 0 passes. Mutations of the 10-shot limit, the 70 km/h limit and
the timing abs each fail the test (21, 4 and 16 points).

## Not verified / not 1:1

- **Not wired.** The app doesn't call `decide` yet. Its inputs (ball kind/sub-kind/state, swing timing, verdict
  kind, the recorder flags) need mapping from the port's own state. That is P0b4d1, checked per point against a
  recording.
- **The score award is stubbed in the check.** The scoreboard event is an input here. The award itself is
  `hst_sim::score`'s (checked by `tests/score.rs`).
- **Names of the ball's kind/sub-kind/state bytes** are taken from how the decision uses them, not from what the
  ball code means by them. P0b4d1 has to find which port fields they are.
- **B22's "set-ending point replays even with bots".** By this decision a bot-only point replays only when it wins
  the match (or in attract play). The B22/B48 recordings saw the replay at slot 5's match-ending point. Whether a
  bot's set point that isn't match point replays is P0b4d1's recording to confirm.
- The replay run, speed, cameras and buttons are P0b4d2–P0b4d5.
