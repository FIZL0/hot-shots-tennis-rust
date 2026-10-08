# P3d — the shared generator's draws

Who draws on the shared MT19937 (`*(gm+0x80)`, draw 0x19f5c0) and when, against `context/p3b/rng_s05.bin` (slot 5,
doubles bots) and the same run's `context/fixtures/match_s05.bin`.

- `01-CALL-SITES-FINAL.md`: the draw log, every call site with what it is and whether it runs in a match, the app
  fixes (toss uniforms, lazy serve coins, placement draws), the replay test and the gaps left (P3d1–P3d3).
