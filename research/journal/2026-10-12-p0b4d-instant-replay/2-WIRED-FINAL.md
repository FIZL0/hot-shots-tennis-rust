# P0b4d1: the decision wired (`play/instant_replay.rs`)

`instant_replay::point` runs right after the scoreboard awards a point (`play.rs`, before the tiebreak note) and
keeps `instant::decide`'s result in `Game::instant.decision` (logged as `instant replay: …` when it replays).
P0b4d2–5 read it from there.

## Where each input comes from

| `Point` field | Game | Port |
|---|---|---|
| ball_kind / ball_sub | ball +0x58 / +0x5c: shot class / kind (`0x37b110` writes them) | `g.shot.class` / `g.shot.kind` |
| ball_state | ball +0xa5: line call (4/5 = in/out after the net) | `g.flight.call` |
| ball_bounces | ball +0x228: court contacts | `g.flight.contacts` |
| ball_vel | ball +0x150: the launch velocity (`375da0` writes +0x130/+0x140/+0x150; only +0x130 moves) | `g.hit_flight.ball.vel` |
| out_side | 0x42305c: the body-hit player (−1 none, P25) | `g.body_hit.is_some()` |
| bounces | ball-effect object (`*0x43b1d0+0x71c`, `38f510`): at the ball's first bounce on a court material (0x410b70 row b0) during a point, +0xfb0 = landing spot, +0xfa0 = spot + unit(v_xz)·\|v_xz\|·0.8 (the skid mark's two ends); zeroed at serve set-up (msg 0xe), frozen at the point (msg 0x17) | `instant_replay::bounce` from the existing first-court-bounce hook, Rally phase only |
| rally / faults | rally block 0x3165f0 +0 call, +0x18 faults | `g.rally.call` / `g.rally.faults` |
| verdict | scoreboard +0x426 = the judge's call | `verdict.call` |
| timing | recorder +0x10f34 + 4·player | swing offset kept per player at `strike` |
| controller | 0x422fc8 + 4·player | 0 for a human seat, 0x20 a bot |
| event / match_won | 0x4230b8 / 0x4230be | `Event` (point 0, game 1, set 2) / `score.match_over` |
| ticks | gm+0x36c: +1 every match-handler tick, cleared at serve set-up | `Instant::ticks`, +1 per `simulate`, cleared in `next_point` |
| allowed / recorded | gm+0x346 / +0x32f, cleared at serve set-up (`324b50` clears gm+0x340..0x380 and +0x32f) | cleared in `next_point`, carried in `decision` |
| option | 0x2ef2b6 | 1 (the value in every save state) |

## Check

`play::instant_replay::tests::a_bot_match_replays_its_match_point_only` plays a bot-only singles match (1 set,
2 games) headless through the app: 10 points, only the match-ending one replays (as B22/B48 saw on slot 5's
match-ending point); the same match with 1P flagged human also replays only match point (no fast game-point winner
by 1P came up). 4 s in release.

## Not verified / not 1:1

- **Not watched against the original side by side.** There is no replay run yet (P0b4d2), so the port only logs
  the decision; the kinds of points come from the oracle-checked `decide` plus B22/B48's match-point replay. Human
  game-point winners, aces and option-2 cases weren't hit in the headless match.
- **record_shots** = the rally's shots: no shot recorder yet (P0b4d2).
- **cancelled** is always false: the rally's cancel (gm+0x55 == 3 with ticks > 1799 or 0x2f073c == 0) is P0b4d5.
  0x2f073c is 0 in slots 3 and 5, which would cancel every rally if that branch ran each tick, yet slot 5's match
  point replayed in B22 — so the branch must be gated by something in `323ee0` not yet read; P0b4d5 has to find it.
- **option** fixed at 1: the app has no instant-replay option yet (front-end, ours to add).
- **ticks** count from `simulate`'s start, the game's at the handler's end: the 900-tick rally limit may be one tick
  off.
- **Ball mark tail** in host floats (v·0.8, not unit·len·0.8 with ps2 ops): presentation only, within the 0.1 m test
  this can only flip a call that sits exactly on the threshold.
- **demo** false: no attract play in the port.
