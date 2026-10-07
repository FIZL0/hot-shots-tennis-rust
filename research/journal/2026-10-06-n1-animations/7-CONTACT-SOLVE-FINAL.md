# N1d3b — The contact solve (FINAL)
- Code: `contact_solve(table, i, contact, pos, scale)` in `pose.rs` ports the contact solve (0x3561e0): body step (x, z) toward the contact point, yaw toward it, CCD over forearm + aimed upper arm (≤50 passes, stop at 0.01 m or dot ≥ 0.99999, angle clamped to cos ≥ 0.866), 2 hand passes with the half x-step, then joint quaternions relative to the table pose. `ik_frames(n)` gives the frame pair (n,0) or (8, 8−n).
- Validation: the RAM images (context/ram/s0*.bin) don't hold a valid solve (fields at +0x3fb0..+0x4010 are stale/garbage), so the test `contact_solve_anim` uses context/fixtures/anim_s05.bin. On frames where flag +0x3fc8 rises 0→1, inputs are contact +0x3fb0, pos +0x3fe0, and stroke = first motion ≥0x10 the player starts within 20 frames, minus 0x10; arm tables come from context/ram/s05.bin.
- Findings:
  - (a) The side scale +0x12b0 is ±1 and flips at the change of ends (+1 on the −z side), so the static RAM value is wrong after ~frame 6899; the test derives it from pos.z.
  - (b) +0x3fc4 is recorded already incremented once in the solve frame (expected 8−n+1 / 1).
  - (c) The caller sends motions 0x1a/0x1b to a separate volley solve 0x357bd0 (index motion−0x1a). Other motions ≥0x1c (0x1f, serves 0x25/0x26) reach this solve with indices ≥10, past the table. Out of scope: new task N1d3d.
  - (d) Result: 35/35 in-range solves bit-exact on step; frames all match; 19 out-of-range events skipped. Quats are not verifiable from this capture (+0x4010 lies beyond the recorded +0x3c00..+0x4000 window).
- Verification: `contact_solve_anim` in crates/hst-sim/tests/motion.rs.
- Next: N1d3c (apply per frame), N1d3d (volley solve).
