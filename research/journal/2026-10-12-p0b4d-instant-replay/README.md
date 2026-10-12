# P0b4d: instant replay

The user asked for instant replay and between-points skipping, just like the original. The original's replay is a
whole subsystem: the decision, a snapshot taken at every serve and restored for the replay, a shot recorder, a
fast-forward/slow-mo schedule, its own cameras and the pad handling between points. So it was split
(`plan/P0b4d.md`, TODO P0b4d1–P0b4d5). P0b4d itself is the decision. See `1-DECISION-FINAL.md`.

P0b4d1 wired the decision into the app (`play/instant_replay.rs`), see `2-WIRED-FINAL.md`.
P0b4d2 runs the replay (snapshot at each serve set-up, restore + logged pads), see `3-RUN-FINAL.md`.
P0b4d5 adds the between-points buttons, the rally cancel and the replay starting at the board's pause end, see `4-BUTTONS-FINAL.md`.
P0b4d4 gives the replay its own cameras (`play/replay_camera.rs`, `cutaway::ReplayPicks`), see `4-CAMERAS-FINAL.md`.
P0b4d3 paces it (skip to the highlighted shot, slow-motion ramps, tornado in-betweens, replay debris), see `4-PACE-FINAL.md`.
P0b4d6 sounds the replay footsteps on the close-up closing shots, see `5-STEPS-FINAL.md`.
