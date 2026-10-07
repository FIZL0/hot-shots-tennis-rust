# P0b4a — post-point scoreboard timeline (port: `crates/hst-sim/src/flow.rs`)

## Phase machine (`0x323990`, gm update)
gm+0x56 = requested phase (0xff none); next tick: exit message of the old phase (0:7, 1:0xd, 2:0xf, 3:0x13,
4:0x18, 5:0x1b), gm+0x54 = old, +0x55 = new, +0x58 = 0, enter message (6, 0xc, 0xe, 0x12, 0x17, 0x1a), then the
phase handler (0:324810, 1:324b10, 2:325b90, 3:325be0, 4:326270, 5:3268e0). +0x58 (phase ticks) and +0x50 (match
ticks) count after the handler. Messages go out through `0x18b310` (broadcast; gm's own handler `0x323810`).
- The recorder samples per vsync; the game sometimes spends 3–4 vsyncs on one tick (msg 0x19 tick 31 and each
  4→2 transition: loading). Compare in gm+0x58 ticks.

## Phase 4 (point over)
- Enter 0x17: gm `325cd0` scores the point (`38b820`) and may pick an instant replay (+0x32e, P0b4d).
  Scoreboard (`*0x42d6c0`, handler `0x383350`): 0x426 call 0/6 → stage 0x180 = 0 (pause), else call show 5
  (0x427 = 1 out, 3 fault, 4 double fault, 2 out after net, 0 let) — P0b4c.
- Scoreboard update `0x382810` runs **after** gm in the tick. Stage 0: +0x184 counts, > 30 (> 1 when +0x188
  after a chained call) → +0x192 shown, stage 1, **faults 0x316608 = 0**. Stage 1: +0x184 counts; == 45
  (0x410ea0) and game marks > 0 → game announcement flag (`3899a0`); > wait → show 0x149 (`384280`).
  wait = 0x410e98 (90) for game/set, else +0x468 from msg 0x19 = min reaction length (0x410ed0 by player state
  +0x3db0 − 0x30, only for players whose motion +0x54→+0x20 ≥ 0x2c) or 0x410e90 (20). Always 20 in the recording.
- gm `326270`: sees +0x192 → +0x376, message 0x19 (players react; gm sets +0x375). Done when +0x151 (show over) —
  or +0x17f cleared if option bits 0x2f0653 (2..3 or 4..5) are 0 (0xaa in the save: not taken). Then
  `38d520` match-over check, `327ea0` change ends, set → 0x4230c0 = 0, sub = 1 if ends changed and not a set,
  else 2. Match over (0x4230be) instead: on +0x192 stop players + voice `3a23d0`, sub 5 once the voice stops.
- Exit 0x18: gm `3261d0` → `327d30` next point (unless match over), game marks > 0 → 0x422f9c/0x422fa0 += 1
  (per-game court state index used by phase-2 enter `32d420`/`3306d0`; presentation).
- Shows (`383fa0` drives them while +0x151 == 0): 1 point `384fa0`/`384c00`: fade-in 6, roll 5 (deuce 3), roll
  5 (deuce 3), settle 16 → +0x158; 6 tiebreak `3891d0`/`388ec0`: 6, 8 (deuce 3), 4 (deuce 3), 16; 3 game
  `387910` (clears points)/`3876c0` and 4 set `387d10` (clears points+games)/`387ac0`: fade-in 15, rise
  0x410eb8+1 = 7, drop 0x410ec0+1 = 11. From +0x158: hold until +0x14c ≥ 60·(0x410ea8 = 1.0 point/tiebreak,
  0x410eb0 = 2.0 otherwise), fade-out 6 → +0x151. Game show end with tiebreak on: ends flag 0x4230b4 = 0.
- Totals (ticks at the sub request sample): point 149, deuce 145, game 280.

## Phase 1 (change ends)
Camera (`gm+0xbc`, handler `0x3663f0`) on 0xc: `3667e0` cut 0x76, countdown +0xe8 = 0x50; `3694a0` → sub 2 when
it runs out: request sample tick 81.

## Calls (P0b4c)
`388190`: done when the call sprite anim ends (`38ee70`) and the voice is idle (`1a03f0(5, …)`) or the countdown
0x42d6e8 (table 0x410f0c[0x426·4 + 0x2ef7df·0x14 + 0x2ef7de·0x28], lang 4: 42,41,45,60,36) runs out; then hold
> 0x1d, fade-out 6; out/out-after-net/double fault chain into the score show with a 1-tick pause. Recorded
faults: 79 or 80 ticks (voice jitter) — needs sprite + voice lengths.

## Test
`tests/score.rs::match_s05_post_point`: 25 point-over phases (all scored points without a call, except the
match-over point and its replay) + 2 change-ends phases; faults, points, games compared every tick; request
tick and target phase exact. A PAUSE of 31 fails at the first point.
