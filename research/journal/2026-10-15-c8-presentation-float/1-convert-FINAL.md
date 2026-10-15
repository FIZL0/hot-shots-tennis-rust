# 1 — conversion (FINAL)

- Calls → operators in the original order (`add(add(a,b),c)` → `a + b + c`, `madd`/`msub` keep their names,
  unary minus on variables as `-F(x)`).
- `camera.rs` and `cutaway.rs` were deliberately plain IEEE `f32` ("not the game's sequence"); they now go through
  `F` (chop rounding). Docs updated. Their tolerance tests (`camera`, `cutaway`, `replay_camera`, `intro`,
  `platforms`) still pass.
- `effect.rs`: the ribbon/spline drawing helpers and `sound.rs`'s cheer speed / interval lines were plain `f32`;
  now FPU ops.
- `shade.rs` keeps two kinds of host float arithmetic under a per-function `#[allow(clippy::float_arithmetic)]`
  with the reason:
  - the `f64` raster that stands in for VU1/GS when building the shade map (`light_uv`, `to_world`, `raster`,
    `texture`, `alpha`, `sample`, `build`) — render only, converting it would change the map build;
  - the model of PCSX2's software GS (`pass_lanes`, `pass_colour`, `pass_tri`, `gs_tri`, `receiver_tile`): x86
    SSE singles rounding to nearest, which chop `F` ops would break;
  - `Frame::new`'s double-precision `tan` work (C library soft-float, already chopped by hand).
- `hash_trace` unchanged (these modules are checked by their recording tests, not the state hash).
- `tools/check.sh -p hst-sim`: 251 passed; clippy deny check clean; `cargo check -p hst` builds.

## Not verified / not 1:1
- camera / cutaway: still not the game's exact op sequence (musl transcendentals, own formulas); now chop
  rounding instead of IEEE, only checked to the existing tolerances.
- effect ribbon/spline and sound cheer speed/interval: switched from IEEE to FPU rounding without checking against
  the decompile or a recording (draw/audio only).
- shade: tests that need recordings absent here skip (`recording or disc missing`, `.rcv_pass.txt missing`,
  `pass.txt missing`); the camera test's `palais` / `slot5` recordings are absent too. Those paths are unverified
  by this change.
- PCSX2 not used (no game behaviour change).
