# R5 — hst-oracle game-agnostic

`crates/hst-oracle/src/` now has the kit's `ps2-oracle` shape (FIZL0/ps2-decomp-kit-linux, `crates/ps2-oracle/src`):

- `ee.rs` — the interpreter, no disc/path/game knowledge. Constructors as in the kit: `Ee::bare()`, `Ee::with_ram(Vec<u8>)`,
  `Ee::load(va, bytes)`; `RAM`, `DONE` public.
- `disc.rs` — `elf_segments(elf)` (the kit's function, error messages without its `kit.toml` hints) and
  `Ee::from_program(elf, &[(load, bytes)])`: boot ELF + raw overlays at given load addresses.
- `hst.rs` — the Hot Shots side: `root()`, `addr()`, `program()` (reads `context/iso`, checks the disc with
  `hst_data::exe::Game`, takes the GAME overlay's load address from its own header instead of a constant),
  `Ee::game()` / `Ee::from_ram(name)` as thin wrappers with the same signatures as before, so no test changed.

The "hst-side helper" lives in `hst-oracle/src/hst.rs`, not `crates/hst/src/disc.rs` (on the task line by
file-name match): the Bevy app must not depend on a test-only crate, and hst-oracle depending on the app would pull
Bevy into every oracle test. R6 moves `ee.rs` + `disc.rs` to FIZL0/ps2-oracle and leaves `hst.rs` here.

Differences from the kit kept on purpose (R6 reconciles): our RAM is fixed at 32 MB (`bare()` takes no size; stack
at the fixed `0x01ffc000`, which is the kit's `len − 0x4000` for 32 MB); `game()`/`from_ram` return `Option` (skip
when no disc) rather than the kit's `Result` + `game_or_skip`; no ISO reader or `kit.toml` (we read `context/iso/`).
Arithmetic still comes from `hst_sim::{ps2, vu0}` until R6 swaps it for ps2-float.

Tests: `tools/check.sh -p hst-oracle` — 13 pass, run against the disc and `context/ram/s03–s05.bin`. One existing skip,
unchanged by this task: `serve_ik` "no A_strong" (`context/b29/run3/` isn't in this checkout).

## Not verified / not 1:1
- None for gameplay: no game logic changed. The interpreter's instruction code was moved verbatim.
