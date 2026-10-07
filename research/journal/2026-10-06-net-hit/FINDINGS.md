# Live net hit — findings (session cut off mid-recording)
- Live ball = step 0x375e30(ball, 0, 1, 0, zero plane): plane is zero → world mesh query every sub-step
  (0x32f690 → court object 0x14cdc0 + grid objects 0x3365b0 → per-model sweep 0x15c700/0x159630), not the plane.
- Net contact seen naturally in bot match (slot 5, ~shot 55 after load): material byte +0x220 = 26, normal +0x210
  = (0,0,-1), +0x22c special count 1, ball at height 0.57 m, x≈-0.15. The frame counter +0xac resets at the
  net contact (path recomputed), so shots split there.
- Poking the live ball's velocity (tools/trace_live.py --net, slot 4, x≈3.7) went *through* the net at 0.3 m
  with no contact — unexplained; maybe per-polygon flag 0x8000 / object filter. Use natural bot net shots instead.
- After the first bounce 0x379bd0 rotates velocity by +0x1b0 angles (and 0x379ae0 under global 0x2ef7e2) —
  live/path both; not ported (fixtures have +0x1b0 = 0).
- Polling with one PINE message per frame still skips ~every other frame; replay can step across gaps.
Next: record shots 50–60 of slot 5 whole (watch script in this folder's notes), dump the net/court collision
model (world+0x84 → +0x138 court object, grid at 0x4238a8), port the triangle sweep, test bit-exact.
