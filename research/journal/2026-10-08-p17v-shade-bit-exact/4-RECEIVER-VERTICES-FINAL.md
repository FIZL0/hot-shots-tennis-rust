# P17v1: shade receiver vertices bit for bit

## Answer
`shade::receiver_vertex` and `shade::tex_matrix` reproduce every receiver triangle vertex of the court 10 build in
`context/p17v/cap0.pkl` bit for bit: 31 116 vertices (1 785 unique) across the 16 shadow textures, given the game's
camera, item and caster light matrices. Test: `tests/shade.rs` `receiver_vertices_match_the_game` on
`context/p17v/rcv.txt` (made by `research/p17v1_rcv.py`).

## The model
- **XY, Q (VU1, per draw item).** The setup multiplies the item matrix rows by the camera's world → screen matrix VP
  (VU 19..22 = camera + 0x1e0, uploaded with the camera block). The hole item matrix is diag(1, 0, 1, 1), so this is VP
  with row 1 zeroed.
  - clip = row-vector transform of (p, 1), VU chop math (`vu0::transform`).
  - Q = 1/clip.w (`vu0::div`). clip.w is the constant 411.591, because the camera looks straight down.
  - X, Y = FTOI4(clip.x·Q), FTOI4(clip.y·Q).
- **S, T (EE + VU0, at load).** Per caster, per receiver node, the build computes T = (node × L) × bias:
  - L is the caster's light matrix (caster + 0x10).
  - bias = [[.5,0,0,0],[0,.5,0,0],[0,0,0,0],[.5,.5,1,1]] (shade state + 0x1b7b0).
  - Both products are EE FPU madd chains summed in index order 1, 0, 2, 3. The hole's nodes are identity, which leaves
    L unchanged.
  - Then a VU0 macro `vmulax/vmadday/vmaddaz/vmaddw` gives (s, t, w', 1) per collected vertex (w set to 1 for the
    transform). That stream goes into the receiver packet next to the positions.
- **STQ.** The VU1 multiplies (s, t, w') by Q. So the GS Q is w'·Q, not Q.
  - w' = L[3][3] is 0.99999994, 0.99999988 or 1.0000001 for 5 of the 17 casters.
  - That is the 1–2 ulp "odd Q" on 7 308 vertices. Those triangles are not clipper output; the clipper path is never
    taken for this pass.
- **The EE adds need ps2.rs's guard bit.** A plain alignment truncation (`research/tools/ps2fpu.py`) misses T[3][0] by
  1 ulp (27/38 vertices of a test packet). With the guard bit, 38/38 match.

## Inputs taken from RAM (gaps)
- The caster light matrices L come from the halt state's RAM. Porting their load-time build from the disc → P17v5.
- Which hole triangles each caster draws (plane and sphere cull at load) is not ported; the fixture lists the caster
  per vertex → P17v6.
- The 40 prim-6 sprites per build in the receiver texture range (Z 0xffffffff, Q 1, constant ST) are not receiver mesh
  and are left out.

## How it was found
- **The halt state.** `research/p17v1_halt.py` patches the first tile's read-back loop into a self-branch and saves
  state 8. Its EE RAM is kept as `context/p17v/h8/eeMemory.bin`, and the VU1 microprogram listing as `h8/vu1.dis`.
- **Where S, T come from.** The ST stream in RAM (e.g. positions 0x2c4f80, (s, t, 1, 1) at 0x2c51e0, 38-vertex batches)
  is rebuilt bit for bit from caster 12's L.
  - A RAM scan for the node matrix first turned up random near-identity matrices. They only "fixed" T[3][0] by
    perturbing it; the real cause was the missing guard bit.
- **Odd-Q triangles.** All of them satisfy S = s·Q and T = t·Q with the normal Q. Only the output Q differs, which
  pointed at the third ST component.
