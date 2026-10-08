# P3d3 setup and singles shared draws

Task: port the shared-generator draws the match setup (0x322d90) and the singles new point (message 0xe) make, which slot 5's recorded rally never ran.

Outcome: DONE. Setup order confirmed offline on 7 save states; the app was missing one placement draw per player. The singles server's program-6 voice is ported. A live draw log shows the 0xe order. Gaps: doubles message 0x10 voice (P3d4), gallery used-flags across matches (P3d5).

Files:
- 01-setup-and-serve-draws-FINAL.md - findings
- research/p3d3_setup_ram.py - offline setup check (writes context/p3d3/setup_ram.bin)
- research/p3e5_draw_log.py - live draw capture (now with HST_V0)
- crates/hst-sim/src/rng.rs (`Rngs::setup_gallery`), crates/hst-sim/tests/rng.rs (`setup_draws_like_the_game`), crates/hst/src/play.rs, crates/hst/src/audio.rs
