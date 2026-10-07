# P0c1 — live-ball recording + per-frame replay (done)

- **Live ball = `*(gm+0x88)`** (advances 1 frame/vsync, collides with the net); `*(gm+0x98)` is the path
  predictor (jumps ahead). The older note "live = gm+0x98" (1-FIND-BALL-PART) was the predictor.
- `tools/record_live.py 5 20000 context/live/net_s05.bin` at NominalScalar 0.25: 20000 consecutive vsyncs
  (7522–27521), no misses. Sample = u32 vsync + live 0x290 + predictor 0x290 + rally block 0x3165f0 (0x40), 1380 B.
  Slot 5 court index `*(0x422f90)` = 10.
- Don't open a second PINE connection while recording (hangs); `pkill -f <pattern>` kills the calling shell if the
  pattern appears in its own command line.
- `crates/hst-sim/tests/live.rs`: each recorded frame → `Flight` (net off) → step → compare with the next frame.
  **12309 airborne frames bit-exact.** 189 contact frames (mesh, not ported):
  material 1 court 144, 48 fence/wall 19, 13 (outside ground, y>0) 12, **26 net body 5**, **2 net cord 3**
  (y≈−0.88), plus 6 "uncounted" pushes near |z|≈20.9 with y > −2r (velocity changes, +0x210/+0x224.. untouched).
- Court bounces through the mesh: **velocity already bit-exact**, position off ~6.4e-6 in y — the mesh hit
  struct gives a different t/penetration to the same contact-point routine 0x12fc30 (LAB_00376878).
- Net contacts: vsync 14450, 16039, 20089, 21197, 26299 (material 26); cord 15016, 19501, 24979 (material 2).
- Mesh query dispatch (asm_mesh.txt): 0x32f690(world, hit, start, end, t0 filter 0|0x41e608, t1 0, t2 out flag,
  t3 grid?) → 0x335b70(world+0x138) → 0x14cdc0 (model update 0x14ca00, then 0x15c700 on +0x120 scale, +0xc model);
  t3≠0 → 0x3365b0 (grid objects, gated by 0x3fc008); stack byte → 0x14cdc0(world+0x14c).
