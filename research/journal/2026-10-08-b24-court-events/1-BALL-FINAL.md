# B24a court 4's passing ball (trigger type 15) — FINAL

## What triggers it
- Match state gm+0x54: 0 start, 1 change of ends, 2 play, 3 point decided, 4 between points, 5 game/set end.
  Exhibition runs 0 → 2 → 3 → 4 → 2 …; a game that changes ends goes 4 → 1 → 2.
- Creature messages (+0xc0): 0xc at a game end (entering 4), reset; 0xe entering 4 and entering 1, reset (at 1 only
  type 15 resets, otherwise all but 37); 18 entering 2, no reset; 0x1a reset; 0x1b/6 clear +0xbc, then reset.
- Reset 0x3f23b0: row byte +0x6c == 0 and +0xbc (played) set → only active (+300) = 0. Else zero 0x128..0x290,
  active = on (+0x281) = 1, moving = path (+0x58) present, +0x274 = 1, then callback 4 (type 15 skips the
  animation restart).
- Callback 0x3f9200, msg 4: if state == 1, players ≥ 2, not played and roll bit 16 == 0 → route = (roll>>16)&3.
  Route 0 = home matrix. Routes 1–3 (two points each at 0x41cc70): fwd = normalize((p1−p0).xz), right =
  cross(up, fwd), then × rot_y(π) (0x125f68), position p0. Then +0x274 = 1, animation restart, on, playing.
  Tail: not playing → on = moving = 0.
- Msg 2 while playing: sound 33 at frames 86/147/200/240 (at the route's sound position, 0x41ccd0, volume
  int(global × 1.2)); len − frame ≤ 10 → +0x274 −= 0.1 (also the model alpha); frame ≥ len → playing = on = 0,
  +0xbc = +0xbd = 1.
- So: once per match at most, a 1 in 2 roll at each change of ends until it plays, route uniform 0–3.

## Draw
- 0x3f3c30 draws only when !+0x282 && active && on && +0x274 ≠ 0. Its +0x27a countdown, at 0, clears +300 and the
  type's flag at *(0x43b820)+0x424+type (the answer to P14c3b's question).
- Model trg_M-05: nodes root, persp, top, front, side, ball1, shadow; one 240-frame clip, 80 ticks/frame. ball1 and
  shadow move ~32 units along local x; home (-17.73, -0.07, 2.16) faces +z, so route 0 crosses the court along x.
  The app needed `NoFrustumCulling` on its parts (bind-pose bounds cull it mid-flight).

## Port
- `npc::Trigger` playing/route/routes/done, `Near.ends/players`, `TriggerRow.rearm`, `Exe::ball_routes`.
- `play/npcs.rs`: on entering `Phase::ChangeEnds` the type-15 triggers reset with `ends`; drawn from `world` while
  active && on && speed ≠ 0.
- Test `passing_ball_matches_the_game` (`context/fixtures/ball_c04.bin`: samples 9200–10600 of
  `context/b24/r15_long.bin`, recorded by `research/b24_rec15.py`, parsed by `b24_parse15.py`): 1399 ticks,
  3 resets, the roll and 4 bounce sounds bit-exact (route 0).
- Screens: `context/b24/orig_ball_100.png` vs the app's frame 100: ball at the same spot over the far court.

## Open (sub-tasks)
- B24b creature sounds in the app (the ball's 33 at its route position, ×1.2).
- B24c fade alpha (+0x274) and the active/on-gated draw for every type; reset timing (original resets entering
  state 4, the port at the serve).
- B24d routes 1–3 against a recording (only route 0 was recorded).
