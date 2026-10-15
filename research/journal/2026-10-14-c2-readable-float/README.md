# C2 — readable PS2 float math

Task: wrap the sim's hand-written `ps2::add/mul/…` and `vu0` calls in newtypes so ported code reads as plain math,
keeping chop rounding, guard-bit adds and the original operation order; no plain `f32` arithmetic left; hashes
unchanged after each step. Files: `hst-sim/src/ps2.rs` · `vu0.rs` · `ball.rs` · `shot.rs` · `player.rs`.

Iterations: `1-types-ball-shot-player-FINAL.md`. Method for the rest of the crate: `plan/C2.md` (tasks C4–C8).

C9 (oracle check of C2's three unchecked arithmetic changes): `2-c9-oracle-FINAL.md`.
