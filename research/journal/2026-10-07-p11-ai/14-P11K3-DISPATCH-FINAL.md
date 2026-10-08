# P11k3 — doubles dispatcher wired into the app

## The dispatcher (0x3cebf0)

- State +0x54: 0 choose, 1 serve (0x3cee70), 2 receive (0x3cf7b0; true → state 3), 3 rally.
- State 3 picks the routine by the row's style: 1 NET (0x3d02c0), 2 BASE (0x3d1320), else the ALL coin +0x274
  (set → NET).
- State 0: player+0x12b8 == server → 1, == receiver → 2, else 3.
- At the end, if +0x60 (`Mind::active`) is 0 the stick quad is zeroed.
- State setter 0x3ced60: 3 → set_sub(0) (0x3d0110), 2 → receive set_state(0) (0x3cf6f0), 1 → 0x3cede0.
- +0x26c/+0x270/+0x274 are `Mind::{net_left, net_rate, net}`. `Rally` now holds a `Mind` for them, and set_sub(0)'s
  repick is `Mind::rally`.

## Message handler (0x3ce7c0)

- 0x14/0x16 (strike / new path): state 3 with sub ≠ 3 → set_sub(0); state 2 with receive state ≠ 3 → set_state(0).
  0x16 on shot 1 sets the hold flag +0xb0. 0x16 with phase 3 clears +0xbc.
- 0x15 (hit): state 3, shots > 1, sub ≠ 3 → set_sub(0). Then the shot memory copy (0xf0..0x230), the 0x3d51e0
  repick and the timing draw 0x3d24e0(…, 1) (the app's `ai_heard_hit`). Clears +0xbc.
- 0x17: +0x60 = chance(50). 0x19: net_rate ±5 (clamped).

## Check

`replay()` in ai_rally.rs asserts the routine each recorded call ran against the state choice:
- state 2 → receive;
- state 3 → `Mind::play(style)`.

Every call in all fixtures matches.

## Wiring (`crates/hst/src/play/doubles_ai.rs`)

- `bot()` gets one hook after the serve: `doubles_ai::step` runs in doubles while the Mind receives or rallies (serve,
  rally and point-over phases), in place of the stand-in (`intercept`, sweet-frame press, side split).
- Each frame it builds the routines' `Body`/`World` from the app (y flipped). The path is 180 frames of the app's own
  flight, copied once a tick. The hit's reaction/guess (`ai_hold`, `ai_guess`) go into `wait`/`guess`/`from`.
- It sends the stick through `bot_stick`/`stick_dir` to `locomote`. A button sets `ai_press` to the routine's aim
  and presses.
- The app's `Mind` stays the owner of the net pick. The routines' own set_sub(0) repicks write back to it; the
  repick on the entry/hit re-entry is dropped because `ai_update`/`ai_heard_shot` already drew it.
- Run: `HST_AUTOPLAY=1 hst … --stage 1 --play`, 4 min: 15 points, rallies up to 35 shots, receive → rally
  handover on every point, no panics.

Gaps: P11k6. A frame-exact replay through `step` is its done-when.
