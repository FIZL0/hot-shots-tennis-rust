# B24b creature sounds in the app — FINAL

- The startled types' sounds (P14c3d) were already played by `play/npcs.rs` (`step_near`'s ids → `g.sounds`, 19/6
  through `heard`); nothing to add.
- Court 4's passing ball (type 15), callback msg 2: at frames 86/147/200/240 it plays its row's key on program 7 at
  the point `0x41ccd0 + route·0x40 + k·0x10` (k = 0..3 by frame), then sets the voice's volume to
  int(effect level × 1.2) = int(115 × 1.2) = 138.
- Port: `Exe::ball_sounds` (4 routes × 4 points), `Trigger::bounces`, `Trigger::sound_at` (the bounce's point for
  type 15, else where it stands), `trigger_sound(15, …)` = program 7 at `level(1.2)`. The app plays every trigger
  sound at `sound_at()`.
- Test `passing_ball_matches_the_game` now checks each of the 4 recorded bounces' point and volume. Route 0's
  points: (-1.7, 0, 2.5), (6.3, 0, 2.5), (11.361, 0, 2.5), (14.7, 0, 2.5) — along x, the ball's path.

## Not verified / not 1:1
- Not compared against PCSX2's audio output: the fixture has the sound ids and frames, not the voice's position or
  volume; the position/volume come from the decompile (callback 0x3f9200) and the exe table. Mixing is presentation
  (PLAN: SFX cues by feel), and the app's 3D attenuation is its own.
- The original starts the voice at 0x40 and raises it to 138 straight after; the app starts it at 138.
- Routes 1–3's points are read but only route 0 has a recording (B24d).
