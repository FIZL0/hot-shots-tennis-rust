# P0c4 — net through the mesh in live play (done)

## First-bounce turn (+0x1b0)
- Source: 0x37e240 (shot effects) fills curve (sp+0x37c → +0x250/254 via f16), bend (sp+0x378, f17) and the turn
  (sp+0x374 → f15 → 0x379080 → 0x375da0 → ball +0x1b0). Turn = per-character table 0x404110 (stride 0x7c,
  character ≥ 3) × π/180, only class 0 kind 0 (serves) when the special condition holds. round1: −10° on two serves
  (vsync 25558, 28213).
- Applied in 0x375e30 after the response, per sub-step, when: shots this rally (0x423060) > 0, live/path flag
  (param_4) 0, +0xa4 == 1 (bounced, not rolling), sub-step counter s0 == 1, bounces +0x224 == 1, +0x1b0 ≠ 0.
- 0x379bd0(pitch, yaw, v): pitch = atan2f(−v.y, √(z²+x²)) + p clamped [−π/2, π/2] (0x404058/60), heading =
  atan2f(x, z) + yaw wrapped ±π; M = I·rot_x(pitch)·rot_y(heading) (0x125ec0, 0x125f68); v = M row 2 · √(y²+x²+z²).
- atan2f 0x111b60 / atanf 0x115970: fdlibm with the game's own rounded constants (ELF 0x1c5968..), DAZ on the
  zero tests, pi_lo = +0x34222168 used as pi − (z − pi_lo). Asm: `context/notes/asm_atan.txt`, `asm_redirect.txt`.
- Evidence: `crates/hst/tests/line_calls.rs` without the exemption — 810 predictor frames, 0 mismatches
  (2 flight mismatches with the turn forced to 0).
- Not ported: 0x379ae0 (random bounce angles from +0x258/+0x25c and the 0x404040.. table) runs only under menu
  option 0x2ef7e2 (set by the menu program) — belongs with P21 (modes/options); needs a recording with it on.

## Live play on the mesh
- `hst_sim::court::{world, materials}` (moved out of `tests/live.rs`): any disc court 1..11, hole 1. Multi-node
  props (e.g. a carriage) now get node matrices local·parent (0x14cb50 walks the tree with each parent's
  matrices); verified only on court 10, where every prop is single-node. `mdl::Model::node_parent` added.
- App: `--stage N` builds the world; the live ball steps through `step_world`; no stage → flat court + net.
  Autoplay soak on stage 10: points/faults comparable to the flat court, no panics.
