# B24c — creature draw gate, fade and reset timing — FINAL

## Original
- The creature handler (+0xc0 ← message): 0x19 → callback 7; 0x17 → 6; 0xc → callback 8 then the motion reset
  (every type); 0xe / 0x1a → (0xe out of a replay keeps done/matrix/path place, in one puts them back) a reset
  unless type 37, and in match state (gm+0x54) 0 or 1 only type 15; 0xe also sets +0x283. 0x1b/6 clear +0xbc,
  +0xbd, +0x282, +0xd9, then reset.
- Who sends them (N3g journal 14-CHANGE-ENDS, P12b 3-CALLS): 0xc only on entering the change of ends (not at a game
  end that keeps the ends); 0xe at every serve set-up (and the replay's start).
- `ball_c04.bin` (court 4, state, +0xc0, +300, +0x281, +0x274): 0xc at sample 27 (state 4, no roll), 81 ticks later
  0xe in state 1 (the roll: the ball plays 108–348, +0x274 1.0 → −0.1 over its last 10 frames, then on = 0), a
  plain point's 0xe in state 4 at 1313 (played: active = 0). 81 ticks = the app's `CHANGE_ENDS` (80) + 1.
- Draw 0x3f3c30: only if !+0x282 && +300 && +0x281; the +0x27a countdown (already ported as `Trigger::stage`, P14c3b);
  then drawn only if +0x274 ≠ 0. Only type 15's callback writes +0x274 into the model's mesh object +0x5c (its alpha);
  for the other types +0x274 is the animation speed and only gates the draw.

## Port
- `npc::Trigger::hear(row, msg, replaying, phase, near, roll)`: the handler's 0xc / 0xe / 0x1a.
- `passing_ball_matches_the_game` now feeds every message change through `hear` with the recorded state: the 0xc
  reset (no roll), the state-1 0xe (the roll) and the state-4 0xe (off) match bit for bit.
- `play/npcs.rs`: 0xc at the ChangeEnds entry (all creatures: callback 8 + reset; was: only type 15, rolling);
  0xe on entering Serve (`next_point`, so faults and lets too; was: the first serve after a decided point, which
  also reset everything again after a change of ends) with state 0 at the match's first serve, 1 after a change of
  ends (type 15's roll), else 4. The game-end callback 8 is no longer sent at games that keep the ends.
- Every model creature: visible iff active && on && speed ≠ 0 (was: only types 2, 6, 15). Type 15's materials get
  alpha × `speed` (blended below 1).

## Not verified / not 1:1
- +0x282 (the draw's own off switch) is never set in the port: its setter wasn't found (only cleared by 0x1b/6).
- The replay's 0xe (`replaying = true`, restoring done/matrix/path) isn't sent by the app's instant replay: the app's
  replay doesn't step the creatures through it. Not checked in the app.
- The app's mapping of 0xc/0xe to its phase changes comes from the other ports' message timing (doubles AI, music,
  pop-ups) and the 81-tick gap in ball_c04, not from an app-vs-PINE run of the creatures.
- The match-over → new match path sends a state-4 0xe; the original's new-match messages (0x1b/6) aren't ported.
- The fade's look (alpha blend of the ball and its shadow) was not compared on screen with the original (rendering
  only has to look right; 10 frames).
