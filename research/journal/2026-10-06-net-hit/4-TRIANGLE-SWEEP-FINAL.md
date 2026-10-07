# P0c3 — triangle sweep bit-exact (done)

Evidence: `crates/hst-sim/tests/live.rs` — 12498/12498 replayed frames of `context/live/net_s05.bin` bit-exact in
position and velocity through `Flight::step_world` on court 10 built from the disc (contacts: material 1 ×144,
13 ×12, 48 ×19, 26 ×5, 2 ×3). `crates/hst-data/tests/collision.rs` also checks wrap modes + textured flag vs RAM.
Asm: `context/notes/asm_tri.txt` (0x12fe40, 0x1580b0), `asm_mat.txt` (0x157e00 texel wrap, 0x1311e0 line meet,
0x3365b0 grid), `asm_box.txt`, `asm_matload.txt`.

## Triangle sweep 0x12fe40 (verts, hit, start(+0x10 r), end)
- face n = normalize((v1−v0)×(v2−v1)) (FPU subs, VU cross/normalize); same gates as the plane sweep; face only when
  d_start ≥ 0.98r; inside test per edge (v0,v2),(v1,v0),(v2,v1): normalize((−n)×(a−b))·(b−centre) ≥ −0.0005.
  t<0 or d_start<1.005r → penetration −(skin−d)/r, t 0. point = centre − n·r.
- edges (v0v1, v1v2, v2v0) only if no face hit: cylinder test with ax = n̂(m×e), bx = n̂(ax×e), FPU madd chains;
  t ≤ 0 → t 0, penetration −0.005, centre = start + n·0.005r. point = projection of centre on the edge (VU).
- corners only if no edge hit. All three of a kind are tried, nearest kept (strict <).
## Hit record (ball step stack 0x90): +0 t, +4 penetration, +0x10 centre, +0x20 point, +0x30 normal, +0x40 material,
  +0x44 default material (0 for the ball), +0x48 object. Initialised per sub-step: t MAX, pen 0.
## Per object 0x15c700 → 0x159630 (node tree)
- start/end w=1 → inst+0x10 (world→model); r × (1/scale) if scale ≠ 1; box = both end spheres (0x159490).
- triangle box test = VU0 micro 0xae (vcallms; code at 0x570): SUB max_sweep−min_tri, max_tri−min_sweep; any
  sign/zero sticky → fail (strict overlap); the second pair only distinguishes contained/partial (both pass).
- per triangle: order (0,1,2) if uv2.w ≥ 0 else (2,1,0); two-sided (state flag 1) → second pass reversed.
  Reversed hit swaps uv0/uv2 in the shared tri struct (persists into pass 2). Hit then 0x1580b0: material; 0 or in
  the ignore list ([32] once +0x264 ≠ 0) → hit restored from backup. Accepted: centre/point/normal × node matrix,
  normal renormalised; t > 0 → box rebuilt to lerp(end,start,min(1.1t,1)).
- hierarchy culls (node/batch/packet boxes) not ported: they contain their triangles' boxes.
## Material 0x1580b0
- no attribute map → 4. Not textured (state flag 8) → texel (0,0). Else: farthest-ish corner A (two compares),
  plane basis (normalize n, n×e0, …), transpose, 2D: Q = line A→P ∩ line C→B (0x1311e0), s1 = |Q−B|/|C−B|,
  E = lerp uv, s2 = |P−A|/|Q−A|, F = lerp; u,v = F.xy / F.z; texel 0x157e00: ×size, mode (mdl material header
  +8 u, +0xa v = GS CLAMP WMS/WMT) 0 fmod in double, 1/2 clamp, 3 → size. index iu + iv·(map hdr +0x14), nibble by iu
  parity → table.
## Grid 0x3365b0
- mid = (s+e)/2, reach = 1.1(r + |e−s|/2); box of both spheres grown 10% about its centre (low corner first, then
  high with the new centre); cells via cell_of; x,y,z order; each prop once; sphere test
  dot3(mid−centre) ≤ (reach + scale·hdr_radius)²; moving props (entry byte +8) none on court 10.
## Ball response (0x375e30)
- material table 0x410b70 (16 B): +0 court flag, +1 special flag, +4 restitution, +8 spin loss. Frame flags:
  bVar7 (special touched) and bVar3 (special first) stay set for the rest of the frame.
- non-court bounce: spin ×= 1−loss and **fVar27 (spin before, for the kick) := that reduced spin**.
- material 32 (' '): +0x264 count (reset each frame), no bounce counting, vn × +e, no kick.
