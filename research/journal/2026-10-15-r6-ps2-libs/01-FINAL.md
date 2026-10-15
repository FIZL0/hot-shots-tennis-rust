# R6 — done

All five repositories build and test; replay stays bit-exact. Details in the README.

## Not verified / not 1:1

- **sinf/cosf/tanf for |x| above ~201 rad**: ps2-float reduces large arguments exactly; the SDK's
  `__kernel_rem_pio2f` does not, and 461/10000 (4.6%) arbitrary finite arguments come out one ulp off the game's
  (oracle). HST never calls them that large (the old HST port panicked there). Follow-up: R7.
- **Inf/NaN arguments to the C maths functions now read as ±max** (the kit's behaviour). Not checked against the
  game's own functions; the old HST port returned NaN. Gameplay never passes one.
- **ps2-float v0.2.0's rsqrt.s / max.s / min.s rules** came from openmx (proved there against an integer model of
  PCSX2's recompiler), not from HST recordings. HST's sim doesn't use `rsqrt`, and its `F::min/max` callers never
  depended on the ±0 order; hst-sim (replay) and hst-oracle pass on it, but no HST-specific test targets them.
- **Kits' VU0 `vsqrt` now chops** and their libm keeps denormal arguments (both from HST's oracle runs). This
  changes kit users' results. Checked only against HST's disc, not against another game.
- **`F::min/max/clamp` now flush denormals and don't panic when lo > hi** (FPU semantics instead of Rust's f32
  ones). Same replay/oracle results in HST.
- **pcpyud** follows the manual in ps2-oracle; HST's old interpreter had the operands swapped. No HST oracle test
  executes it.
- **Windows** was tested under Wine only, not on real Windows.
- **Pre-existing in the Windows kit, not fixed here**: `tests/test_live.py` fails one check ("a state that is not
  a PCSX2 one says what it holds"), and `kit check` says there is no test gate. An untouched checkout of its
  `main` does the same; the Linux kit has fixes for both that the Windows kit hasn't picked up.
- PCSX2 wasn't used: nothing in this task needs a recording.
