# P0b part 2: line calls, point-over check, umpire verdict (port: `crates/hst-sim/src/judge.rs`)

## Functions
- `0x378080` line call at a landing → `judge::call_landing`; `0x378370` line distance (+0x230) → `line_distance`.
  Called from the ball step `0x375e30` for each non-rolling contact while +0x224 == 1 or +0x228 == 1, with
  the contact point; skipped for the live ball outside the rally (gm+0x55 < 3 && gm+0x56 != 4). Wired into
  `Flight::step` via `Flight::lines` (None = no calls).
  - Ball +0xa5: 0 none, 1 in, 2 out, 3 net touched first (+0x22c != 0), 4 in after net, 5 out after net.
  - Serve (shots < 2): singles box 4.115 × 6.4, centre line slack 0.05; rally: 4.115 / 5.485 (> 2 players) × 11.885.
    All + margin `0x403bec` = 0.04 (GAME.BIN data, `exe::Game::line_margin`). Target half = hitter z > 0
    (gm+0xb8 +0x10fa8 from the hitter's model matrix z).
  - Ball stop (+0xa4 = 3: |vel| < 0.001 on a > 45° surface, or > 1800 frames in rally) sets 2/5 — NOT ported
    (needs the contact normal history; play uses the 1800-frame timeout as `stopped`).
- `0x1a9020` per-frame point-over check (rally phase handler `0x325be0`, sets gm+0x56 = 4) → `Rally::check`.
  Arena limits `0x379d80` (|x| 10.685, |z| 19.885 unless flag 0x410870).
- `0x1a92c0` hit check (message 0x15 → `0x325c60`, after the hit routine `0x3467b0` bumps shots and, for the
  return, sets "return bounced" 0x316618) → `Rally::on_hit`. Illegal: serve by non-server, return before the
  bounce / by the wrong receiver / of a net serve, same team twice. Serve struck after a bounce = fault.
- `0x38c550` judge → `Rally::judge` (verdict codes in `Verdict`); stats/HUD history skipped.
- Struct 0x3165f0: +0 call, +4 shots, +8 hitter (also "previous hitter" for the double-hit test), +0xc server,
  +0x14 illegal, +0x18 faults (0x316608), +0x1c let, +0x20 serve call, +0x24 last checked shots, +0x28 return
  bounced, +0x39 body hit on serve, +0x3a lose, +0x3b serve bounced. Reset: `0x1a9000` (snapshot only) from
  scoreboard `0x383350` — assumed per point (`Rally::new_point`); `0x327d30` clears let/+0x39/+0x20 and faults
  unless second serve/let (`Rally::next_point`). Faults also clear when the scoreboard shows a scored point
  (`0x382810`) — play does `faults = 0` on a scored point.

## Evidence (round1 capture, in progress)
- **The captured ball `gm+0x98` is the path predictor** (15 steps per game frame, ignores the net), not the
  live ball (`gm+0x88`, read by the judge). It makes the same line calls.
- `crates/hst/tests/line_calls.rs`: every predictor frame before its call is replayed from the previous
  frame's object, 15 steps: 604 frames, 120 calls (in/out, serve and rally) — call and line distance bit-exact;
  position/velocity bit-exact except one bent serve (class 0, bend 0.581, vsync 25559): the original kicks it
  sideways at the bounce (vel x −0.034 → +0.091; port +0.023). → TODO P6.
- `round1_verdicts` (hst-sim tests/score.rs): 22 decisions (21 in-play point, 1 out) — verdict = recorded
  outcome. 6 skipped: the live ball met the net (predictor re-seeded), 4 of them serve faults (b8 = −1) —
  need the live ball in the capture.
- `round1_replay` fixed: instant replays restore the saved score block (0x423100+ → 0x422fc0+) and replay the
  point; the score test now skips score states already seen.

## Open
- Capture the live ball `*(gm+0x88)` and 0x3165f0..0x316630 in the next recording (changes the sample layout;
  don't touch the running capture) → verify faults, lets, net points, illegal hits.
- Ball stop rule (+0xa4 = 3), body hits (0x42305c from player +0x12b8, `0x352...` at line 152092).
