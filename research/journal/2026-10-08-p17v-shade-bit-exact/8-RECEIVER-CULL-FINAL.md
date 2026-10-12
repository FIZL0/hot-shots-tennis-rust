# P17v6 — receiver triangles per caster (FINAL)

Which hole triangles each caster's receiver pass draws, ported in game floats: `shade::receiver_tris`
(`node_planes`, `box_out`, `receiver_strip`), test `receiver_triangles_match_the_game`: all 17 court 10 casters'
triangle sets equal `cap0.pkl`'s receiver draws (2175 triangles, compared as x z bits since the receiver matrix
flattens y). Prototype and fixture writer: `research/p17v6_cull.py -w` → `context/p17v/cull.txt`.

## What the game does (at load, per caster, VU0)
- Planes: the caster keeps two blocks of four planes (+0x50, +0x90), rows = components (nx ny nz d), lanes =
  planes: block 0 cap + sides 1–3, block 1 sides 4–6 + the sun-facing plane; inside is positive.
- They are multiplied into the receiver node's space (mula/madda chain). The hole's nodes are identity at load:
  with the diag(1,0,1,1) matrices the dump holds now (receiver item / node 0 matrix after load) the result
  misses ~160 triangles, with identity 0. Same conclusion as `tex_matrix`.
- Walk: node tree from the root; a node's box (inf = empty → skip with its children), then its list of
  (material, batch) entries, the batch box, then each packet box: out when for some plane
  ((((((d + |nz|·hz) + |ny|·hy) + |nx|·hx) + nz·cz) + ny·cy) + nx·cx) ≤ 0 (−0 too). Centre = packet header +0x20,
  half = (hi − lo)·0.5 in EE floats. The radius (×0.55 in the old task line) isn't used by this test.
- The node lists hold exactly the batches the game collides with (static, attribute-mapped material); court 10's
  lines, water and the two plane/cylinder materials (17–19, no attribute map) never receive shade.
- Per strip vertex (not no-kick): the triangle it closes is culled when all three corners are outside one of the
  8 planes (value ((d + nz·z) + ny·y) + nx·x negative or −0); else drawn if no corner is outside the cap; else
  culled when N·sun < 0 with N = ((v2 − v1)·−w) × (v3 − v1) (OPMULA/OPMSUB), w the vertex's UV w.
- Compaction: 3-bit history of culled flags; on 7 the previous kept vertex and this one become no-kick
  (history 3); trailing culled vertices are popped. Triangles = kept non-no-kick vertices from the third.

## Variants tried (mismatches vs cap0, all casters)
- node-transformed planes everywhere: ~160; identity for the strip test only: 1; identity everywhere: 0.
- back-face test never applied: 9–10. Node and batch box tests off: still 0.

## Not verified / not 1:1
- Node and batch box tests and the material check (lighting flags & 6 == 0 and colour 128, or a global
  override) aren't ported: on court 10 they never drop a triangle the packet test keeps, all 17 materials pass
  the check; another court could differ. Port with the node → batch lists if a court shows it.
- The planes come from the RAM dump (`cull.txt` `P` rows), not built from the disc: the plane build is a 3.8 KB
  load-time function, its own task (P17v8 line extended). `shade::build` still uses the f64 corner test until then.
- That the node matrix is identity at load is inferred from the result, not seen in a load-time capture.
