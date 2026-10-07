# N1d1 — Exact ANI sampler (FINAL)
- Player anim object = player+0x54: +0x20 motion id, +0x24 bound clip, +0x34 speed, +0x38 time last sampled, +0x3c time, +0x78 loop flag.
  0x1404c0(t, anim): loop → t wrapped by repeated ±length (ps2 sub/add), else clamped [0, length]; samples (0x13e470) only if
  t != +0x38 or dirty. In a RAM dump +0x3c is already the next frame's time: the pose matches +0x38.
- Clip (0x13d950): tracks with any keys whose name is a skeleton node, file order; length = max last key tick / ticks-per-frame
  (div.s); position scale per track = 1 for Bip01Pelvis or zero track length, else |rest.y| (Bip01) or |rest translation|
  (sqrt of y²+x²+z² madd chain) / track length. +0x14 scales, +0x18 per-track cache (3 rotation rows + position), +0x1c node
  indices, +0x20/+0x24 key-search caches (don't change results).
- 0x140360 bind: all nodes reset to rest (+0x40 → +0x80), 1-key tracks set once (0x13dfa0), 0-key → rest. Per frame only >1-key
  lists resampled.
- Rotation keys: squad. Load (0x17a3f0 → 0x175c00) computes control quats s_i = q_i·exp(−(log(q_i⁻¹q_{i−1}) + log(q_i⁻¹q_{i+1}))/8)
  via 0x1759f0 (log 0x175f60 = atan2f(|v|, w)/|v|·v; exp 0x176010 with sinf/cosf, |v| ≤ 1e-4 → factor 1); ends use (q1,q0,q1)
  and (q[n−2],q[n−1],q[n−2]). Interp vtable 0x1d11c0+0xc = 0x1758c0: u==1/u==0 shortcuts, else slerp(q_i,q_i+1,u),
  slerp(s_i,s_i+1,u), slerp of those at 2(1−u)u — all through bare VU0 micro 0x288 (0x175d80, no shortest-arc flip). Key fraction
  (t − k_i)/(k_i+1 − k_i), ticks as ints then cvt.
- Position keys: cubic Hermite (vtable 0x1d1160+0xc = 0x17b010; EE mula/madd/msub chain); tangents 0x17b160: interior in/out =
  (p[i+1]−p[i−1])·(t_i−t_{i−1})/(t_{i+1}−t_{i−1}) and ·(t_{i+1}−t_i)/…; ends ((Δp)·3 − neighbour tangent)·0.5; 2 keys: all = p1−p0.
  Result × track scale.
- Quat → rows 0x175eb0 (plain, no re-orthonormalisation). Node +0xe0 sampled quat, +0xf0 scaled position.
- Characters share motion files; costumes 5–9 of character 6 have a different skeleton (68 nodes) from 0–4 (79), so the bound
  skeleton depends on costume.
- Port: hst_sim::pose::Clip (new/wrap/sample/locals), model(), q_matrix(); libm::cosf; quat::micro_slerp now pub. App
  character::animate samples with it (unkeyed joints at rest). Test motion.rs clip_sampler_ram: s03/s04/s05/s08/s09, 20 players,
  1061 tracks — quats, rows, positions, scales, lengths bit-exact.
- Also fixed `--shot` exiting before the async screenshot save landed (uncapped fps since F0): main.rs auto_shot waits for the file.
- Next: N1d2 time/speed per frame, N1d3 IK/body step, N1d4 faces, N1e crossfade.
