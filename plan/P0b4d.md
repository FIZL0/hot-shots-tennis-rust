# P0b4d

- [ ] **P0b4d — Instant replay.** The decision in `325cd0` (gm+0x32e; ball speed, shot count, options
  0x2ef2b6) and the replay run (saved block 0x4230d0/0x316640 restored, fast-forward/slow-mo ticks).
  Slow-mo frames also run the objects' in-between draws: the ball tornado's is ported but unwired
  (`hst_sim::tornado::Tornado::between`, P17q; feed it the slow-mo fraction and the UVA clock's last/this time).
