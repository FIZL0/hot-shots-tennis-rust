# P0b4d — instant replay (split 2026-10-12)

- [x] **P0b4d — the decision.** `325cd0` (the match handler's message 0x17, the point): `hst_sim::instant::decide`,
  checked against the original over generated points (`hst-oracle/tests/instant.rs`).
- [ ] **P0b4d1 — wire the decision** (inputs from the app at each point, checked per point against a recording).
- [ ] **P0b4d2 — the run.** `324b50` (message 0xe, the serve phase's entry): with the replay pending (gm+0x345)
  it restores the match globals 0x422f90.. and the rally block 0x3165f0.. from the copies it saved at the point's
  serve (0x4230d0.., 0x316640..; the same entry saves them when no replay is pending, with the RNG seed at
  0x42303c), sets gm+0x344, counts gm+0x358 (and +0x354 when the decision didn't ask: a button press did), and the
  point plays again. The shot recorder (gm+0xb8: 0xd0-byte shot records, count +0xd0a0, +0xd0a8) holds the shots.
- [ ] **P0b4d3 — speed and slow-mo.** `323ee0` steps the replay through the shot records with frame windows
  (game data words at 0x3fbc80..0x3fbcb8, `3719c0`). Slow-mo frames also run the objects' in-between draws: the
  ball tornado's is ported but unwired (`hst_sim::tornado::Tornado::between`, P17q; feed it the slow-mo fraction and
  the UVA clock's last/this time); dive debris and the step sound need `Feet::replay` (P17p1).
- [ ] **P0b4d4 — cameras** (0x1d, 0x06, 0x38, 0x53; `Decision::reset_cut` clears the post-point pick, gm+0xbc+0x84).
- [ ] **P0b4d5 — buttons.** `323ee0` reads the pad (0x2f06e0..e3) between points: skip, ask for a replay (sets
  gm+0x345 with gm+0x32e from the press), and cancels the point's replay (gm+0x347) in the rally phase once it ran
  over 1799 ticks or with option 0x2f073c off.
