# P0b — rules and serve flow. Part 1: score, rotation, ends (port: `crates/hst-sim/src/score.rs`)

## Score globals (base 0x422f90 = match settings/state block)
| addr | meaning |
|---|---|
| 0x423044 / 0x423048 | games per set (4 in slots 3/4/5, round1) / sets to win (1) |
| 0x42304c | server (player 0..3); team = player & 1 (0,2 vs 1,3) |
| 0x423050 | court side 0 deuce / 1 ad |
| 0x423054 | receiver (doubles toggles bit 1 each point: 1↔3) |
| 0x423058 / 0x42305c | last hitter / ball-out side (-1 = none), reset per point |
| 0x423060 | shots this rally (1 = serve) |
| 0x423064/68, 0x42306c/70, 0x423074/78 | points, games, sets per team (point idx 0,15,30,40,Ad) |
| 0x4230a4 | set index (history column, cap 5) · 0x4230a8 point winner team |
| 0x4230ac | game changed (consumed by next-point) · 0x4230b0 server rotation count (server = n % players) |
| 0x4230b4 | ends swapped · 0x4230b8 1 point/game, 2 set, -1 no point (fault) · 0x4230bc deuce OFF |
| 0x4230be | match over · 0x4230c0 games this set (ends change on odd) |
| 0x316620 / 0x316628 | deuce / advantage · 0x316624 deuce count (cap 99 @0x411140..58) |
| 0x31661a / 0x31661c | tiebreak / tiebreak point counter (+1 per point, start 1) |
| 0x316608 | 1 = second serve pending (no rotation) · 0x31660c let · 0x3165f0/f4/f8/fc judge inputs (Part 2) |
| 0x42d6c8 | every point wins a game (mode flag) |
| scoreboard obj *0x42d6c0 | +0x50 sets, +0x54 games, +0x15c/+0x160 winner/loser team, +0x149 event (1 point, 3 game, 4 set, 6 tiebreak point), +0x426 call (judge), +0x5c8/+0x5cc rotation/ends at tiebreak start |

## Functions
- `0x38b820` score a point (calls the judge `0x38c550` first; returns 0 when no point). Ported minus stats (`*0x3165c0` records, HUD history arrays 0x1ac/0x2fc/0x3c4).
- `0x38ae80` game/set point check (umpire) → `Score::game_point`.
- `0x327d30` next point: server/receiver/side → `next_point`. `0x327ea0` change ends → `change_ends`.
  **Order:** `0x327ea0` must run before `0x327d30` (it reads 0x4230ac, which `0x327d30` clears). Callers:
  `0x326270` (post-point, calls `0x327ea0`) and `0x3261d0` (calls `0x327d30` when not match over) — exact
  frame order is Part 3 (flow).
- `0x387910` clears points (scoreboard game animation), `0x387d10` clears points+games (set).
- `0x3876c0` (scoreboard tiebreak animation) clears 0x4230b4 when a tiebreak is on — not ported yet, check
  against a recorded tiebreak.
- `0x38ad10` match init.

## Evidence
- Save states: slot 4 = 0-15 side 1 receiver 3; round1 start = 0-30 side 0 receiver 1 (doubles, 4 players).
- round1 capture (in progress at 22:58): point at frame 484 (0-30 → 0-40), next-point applied at frame 632
  (side 0→1, receiver 1→3). `cargo test -p hst-sim --test score` replays every point of the fixture.
