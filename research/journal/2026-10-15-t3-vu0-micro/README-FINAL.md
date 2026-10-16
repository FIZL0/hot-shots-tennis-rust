# T3 — VU0 micro mode in ps2-oracle; slerp's micro program run, not hooked

## What was done
- **ps2-oracle v0.1.3** (FIZL0/ps2-oracle, tagged and pushed; kits' `kit-oracle` bumped too):
  - `src/vu0.rs`: VU0 micro mode. `vcallms`/`vcallmsr` run a program from VU0's 4 KB micro memory to its E bit
    (+ delay slot). Upper ops (all FMAC forms incl. bc/Q/I/acc, max/mini, abs, clip, itof/ftoi), lower ops
    (lq/sq/lqi/sqi/lqd/sqd, ilw/isw/ilwr/iswr, integer ALU, b/bal/jr/jalr/ib*, div/sqrt/rsqrt/waitq,
    fs*/fm*/fc* flag ops, mtir/mfir, move/mr32). Timing as PCSX2's microVU: per-lane FMAC stall (4 cycles), Q lands
    7 cycles after div/sqrt (13 rsqrt) and earlier reads see the old Q, MAC/status/clip flags land 4 cycles after the
    setting instruction, I-bit constant visible from the next pair, upper/lower read before either writes.
  - `src/vif.rs`: `Ee::vif0_dma(tadr, tte)` walks a DMA source chain and runs VIF0 codes (MPG, MSCAL, no-op setup
    codes); `cfc2`/`ctc2`; VU0 micro/data memory mapped at 0x11000000 / 0x11004000.
  - `tests/vu0_micro.rs`: hand-assembled programs for the VIF upload + vcallms, Q latency + waitq, flag latency and
    −0 flags, stalls + branch delay slot + I constant, cfc2/ctc2.
- **Where the micro program comes from**: the boot code points VIF0's DMA at a chain in the boot program's data
  (`vu0_boot_chain` in `research/oracle.txt`: one refe tag → NOP + MPG of 251 instructions at 0). `HstEe::game()` /
  `from_ram()` replay it (`hst.rs::boot_vu0`). Checked: the uploaded image equals `vu0MicroMem.bin` of save states
  3 and 5 byte for byte. Slerp's EE stub does `lqc2 vf12/vf13`, `qmtc2 t → vf14`, `vcallms 0x51`, `qmfc2.i vf15`.
- **rotation.rs**: the `quat_micro_slerp` hook is gone; `quat_slerp` (30k) now runs the real micro program, and a
  new `quat_micro_slerp` test (30k) calls the bare micro entry: negative dot, −0 dot, dot ≥ 0.999, unit and off-unit
  quaternions, t outside [0, 1]. Mutating the model (Q at 8 cycles, flags at 5) makes ~1/3 of both tests fail, so
  the timing really is exercised.
- **Port fix found by it** (`hst_sim::quat::micro_slerp`): the lerp fallback tests the S flag, which is the sign
  bit (PCSX2 `mVUupdateFlags` takes S with `movmskps`), so a −0 dot lerps; the port tested `dot < 0.0`. Now
  `dot.0.is_sign_negative()`. Only reachable when every product is −0; replays unaffected (hst-sim 246 tests pass).

## Not verified / not 1:1
- The micro-mode model's O/U flags are not computed (microVU doesn't either; only Z and S). Slerp reads only Z/S.
- The branch-after-integer-write VI hazard (microVU's "Branch VI-Delay") isn't modelled; slerp's branches read
  VI registers set two or more pairs earlier, so it doesn't arise here.
- `vcallms` runs the program to completion synchronously; EE code that reads VU0 registers mid-run without an
  interlock would see final values. Slerp's stub interlocks (`qmfc2.i`).
- Timing rules are taken from microVU (PCSX2 = what the recordings come from), not from a real console.
