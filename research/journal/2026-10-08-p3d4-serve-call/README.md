# P3d4 doubles serve-call voice

Task: port 0x348a00's message 0x10 voice (doubles: the server's partner calls on match point).

Outcome: DONE. Message 0x10 is the server's toss start (0x3522b0 state 2) broadcasting the toss kind; the same
site gives the singles server the same call, ported too. Live recordings agree with the port.

Files:
- 01-serve-call-FINAL.md - findings
- research/p3d4_serve_call.py - live toss recorder (writes context/p3d4/*.bin)
- crates/hst-sim/src/score.rs (`Score::match_point`), crates/hst-sim/tests/score.rs (`match_points`,
  `serve_calls_like_the_game`), crates/hst/src/play.rs (`serve_call`)
