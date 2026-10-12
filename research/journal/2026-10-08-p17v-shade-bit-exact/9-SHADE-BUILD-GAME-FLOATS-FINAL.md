# P17v8 — shade build on the game-float light frame

## What changed
- `hst_sim::shade::planes` ports the load-time caster setup's clip planes in game floats (EE ops plus VU0 dot3):
  the cap, up to six hex sides from ±p0 ±p1 ±p2 in the game's order, and the sun plane, stored as the game's two
  transposed 4-plane blocks. `light_matrices_match_the_game` now also compares them to the `P` rows of
  `context/p17v/cull.txt`: **bit-exact for all 17 casters** of court 10, slot 5.
- `light_matrix` shares `box_frame` / `dot_yxz` with `planes` (its output is unchanged; the test still passes).
- `Caster::new(shape, placement, axes)` builds the item matrix with `world::place` (code tilt/turn included,
  scaled when scale ≠ 1), then `light_matrix` + `planes`. The f64 `Light` frame and the vector helpers are gone.
- `shade::build(frame, casters, hole, mats)` rasters each caster's `receiver_tris(planes, …)` and samples through
  its disc light matrix. **Court 10 IoU 0.9760** vs the game's map05 (needs ≥ 0.975).
- App: `shadow::Sun` comes from `hst_sim::shade::sun` (game floats), with new `axes = sun_axes(dir)`.
  `main.rs` builds the casters from the placements and passes the hole model and mtl to `shade::build`.
  The Rust-trig `sun_dir` / `clamp_elevation` are removed; `sun_matches_game_ram` checks `shade::sun` against the
  RAM values.

## Not verified / not 1:1
- The raster and sampling are still f64 with a fitted ortho screen and a 0x7f fog threshold (IoU ≈ 0.976, not
  exact). This is acceptable: PLAN says rendering only has to look right.
- Only the sun's hour 1 / time-of-day 0 row is used, as before. Other times of day were never captured.
- The planes and light matrices are checked only on court 10, slot 5. No other court has a cull capture.
- A caster with scale 0 is passed through raw, as the decompile reads. No placement on court 10 has one, so this
  is unverified.
- I didn't compare the app's live map against a PCSX2 screenshot, only through the IoU test on the same build
  path. The app builds, and its tests pass.
