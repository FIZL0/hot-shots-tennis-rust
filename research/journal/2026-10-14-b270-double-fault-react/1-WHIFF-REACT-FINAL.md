# B270 — reactions after a double fault

## Finding
- A double fault from serves that land out/in the net already reacted in the port (headless bot match with the
  serves knocked long: server `di`, receiver `gu` at the flow's reaction tick). `flow.rs` sets `reacted` for call 3
  like any chained call; nothing to change there.
- The broken case is a double fault by **whiffed** serves: `motions` kept the server on its serve motions while
  `g.serving.whiffed` (it stays set through the point-over), so the miss motion 0x29 overrode `react()`'s reaction.
- The original (decompile): the players' message handler takes 0x19 → reaction mode for every player, no serve-state
  exception (only a body-hit player plays `re_ball` 0x2b).
- Capture `context/b270/run1/dfault.bin` (`tools/record_serve.py 3 1300 …`, slot 3, Carol serving, ○ toss + ○ 1.9 s
  later twice → two whiffs; decode with `research/serve_timeline.py`): point over at 7073 with the miss motion 0x29
  at t 2; it plays to t 83 (7155), the server stands (mode 0, motion 0) for 7 ticks, then mode 2 at 7162 (message
  0x19) and its reaction (team reaction 0x31, doubles) from 7164.

## Port
- `play.rs` `motions`: the whiffed-serve override ends when the players react; a miss motion that has run out goes
  to the stance (motion 0) until then.
- Test `play/serve_check.rs` `a_whiffed_double_fault_plays_the_reactions`: slot-3-like doubles, two whiffs, her call
  line held so the call outlasts the miss motion: the server stands, then all four play `gu`/`di` (or a team
  reaction). Fails on the old code (server stays on 0x29).

## Not verified / not 1:1
- The tick the miss motion turns into the stance (port: once its clock reaches the clip length) was not compared
  frame by frame with the capture; presentation only.
- The reaction's tick in the test isn't the original's 89 ticks after the point-over: the headless app has no umpire
  voice, so the call's length differs (the call timing itself is P0b4c's, unchanged here).
- Which team reaction each player draws (app RNG stands in for the game's, as before).
