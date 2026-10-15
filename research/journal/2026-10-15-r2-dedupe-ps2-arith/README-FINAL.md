# R2 — dedupe PS2 arithmetic

`daz` and `chop` were byte-for-byte copies in `hst-sim/src/ps2.rs` and `vu0.rs`. Kept the ps2.rs ones:
`daz` is now `pub(crate)`, `chop` stays `pub` (hst-oracle's `cvt.s.w` and `shade.rs` call `ps2::chop`, so
`pub(crate)` as the task line said would break hst-oracle). `vu0.rs` imports both from `crate::ps2`.

Pure refactor, no behaviour change: `tools/check.sh` passed (707 tests, incl. the bit-exact replay tests).

## Not verified / not 1:1
none (no game behaviour touched; nothing to check against PCSX2).
