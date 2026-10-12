# M2a — same floats on every platform

std's `sin/cos/tan/atan/asin/atan2/hypot/powf` come from each platform's libm: over 400k inputs, x86_64-gnu (glibc)
gives different bits from aarch64-musl and wasm32-wasip1 for all six of sin, cos, tan, atan, asin, atan2. So every
one in the sim now goes through `hst_sim::musl` (the `libm` crate, musl's libm in pure Rust; already in Cargo.lock
via Bevy, renamed so it doesn't clash with `hst_sim::libm`, the game's own).

- Routed: `camera.rs` (20), `cutaway.rs` (35, incl. 3 `powf`, 2 `hypot`), `clouds.rs` (sin_cos), `ball.rs` (the
  rolling test's sin π/4), `shade.rs` (f64 tan), `libm.rs`' `table_sin` table (f64 sin; every entry is ≥0.004 from a
  rounding boundary, so the table is unchanged), and `hst/src/play/reaction_voices.rs` (`fov.tan()`, picks the
  reaction voice). Left: `sqrt` (IEEE-exact on every target), test-only calls, and the app's drawing-only calls
  (bodyhit/surprise/markers/ball_shadow/npcs/popups/main_menu quads, `play.rs` `yaw`/`orbit_from`: figure and camera
  transforms, nothing reads them back into `Game`).
- No `mul_add` anywhere in `hst-sim`/`hst`; rustc never contracts `a*b+c` into FMA, so plain f32 is the same everywhere.
- `HashMap`s: `court.rs` (lookups by name only), `play.rs` character cache (lookups only), `main_menu.rs` (menu
  glyphs/archives, lookups): none iterated. Nothing in the sim iterates a hash container.
- Existing results moved by ≤1 ulp where glibc and musl differ; all camera/cutaway tests against the PCSX2 recordings
  still pass (`tools/check.sh` 326 passed).

## The check
`crates/hst-sim/src/bin/state_hash.rs`: replays `camera_s05.bin` (a recorded bot match: players and ball each tick)
through the match camera as `tests/camera.rs` does, prints an FNV-1a hash of the camera state per tick (5999 ticks),
then one hash per transcendental over a 40k-input sweep (musl's and the game's `libm`). `tests/platforms.rs` builds
it for the host, `aarch64-unknown-linux-musl` (`cargo zigbuild`, run through qemu binfmt) and `wasm32-wasip1`
(`wasmtime`, release unpacked in `context/tools/wasmtime-v49.0.2-x86_64-linux/`) in `target/platforms` and asserts
the outputs are identical: 6020 lines, same on all three. Negative check: putting std `tan/atan` back in one camera
line makes aarch64 differ from tick 1797.

Found on the way: NaN bits differ by platform (x86 0/0 = −NaN, ARM +NaN). The sweep's first draft fed `asinf` 1.000006
and only that line differed. A NaN in state would read as a desync; the hash inputs stay in domain.

## Not verified / not 1:1
- The hash covers the match camera replayed from a recording plus every routed function, not the app's whole `Game`
  state ticking a match: `Game` lives in the Bevy app (`play.rs`), isn't hashable yet (M2c), and the app doesn't build
  for wasm32 or a cross aarch64 sysroot (M9; Bevy's audio/input/window crates need target system libraries). The
  cutaway isn't in the hash either: its replay needs the ISO's GAME.BIN data inside the wasm sandbox. Once M2c lands,
  the same three-target harness can hash `Game` per tick.
- aarch64 ran under qemu-user, not real ARM hardware (qemu's softfloat is IEEE-exact, so the same bits expected).
- The routed functions are musl's, not the game's own (`hst_sim::libm`): same platform-independence, but where the
  original calls its own sinf/atanf, porting those call sites to the game's versions on PS2 arithmetic is separate
  exactness work (camera.rs/cutaway.rs already say "plain f32, not the PS2 sequence").
