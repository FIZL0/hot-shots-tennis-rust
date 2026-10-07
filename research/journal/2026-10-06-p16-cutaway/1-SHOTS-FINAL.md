# P16 — Post-point cut-away shots (FINAL for the shot camera; pick + court views open)
- Data: shot records 0xbc bytes ×108 and scripts (op, f32) pairs ending 0x1a, read by `hst_data::exe::Game::camera_shots`;
  cut-away lists per point kind `cutaway_lists`.
- Recording: `tools/record_cutaway.py` → `context/fixtures/cutaway_s05.bin` (gm, handler, shot camera, reference frames).
- Builder 329440: second subject (+0x7c ≠ 0) — the final rot_y's translation row is the aim point in camera-local
  coords (DAT 1cc390 = origin when ch15 == 0), so with a turn angle the eye moves to the aim. Was ported as an
  in-place turn: shots 0x66/0x68 were off up to 0.95 m; fixed.
- PINE race: one sample has the frame counter ticked but the camera not yet updated; the test skips samples that
  match our previous frame and not this one (1 in 1203).
- Open: court views 0x0c/0x0d/0x11 (multi-player framing), `pick` unverified, not wired into the app.
