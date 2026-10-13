# B157 — Grand Palais covered, Wild Green foliage: VU1's guard band (FINAL)

## Cause
Both courts draw court-model triangles the original never puts on screen.
- **Stage 7 (Grand Palais):** `w-house2` is a box around the court, and the eye is inside it. Its node is
  0x18dec30: state 0x300 (drawn through the clip pass), centre (−0.007, −3.567, −35.168), r 21.63.
  `research/b157_nodes.py dump.ram [all]` lists every node's state/centre/radius/eye distance.
- **Stage 8 (Wild Green):** large near-camera foliage triangles on the left. Same mechanism.

## What the original does (VU1 microcode, `context/b157/vu1.dis`)
- **Transform loop:** a screen-space backface test.
  - vf27 = (p_i−p_{i−1}) × (p_{i−1}−p_{i−2}); its .z is multiplied by the strip's UV w sign.
  - The z sign flag is compared with VU 205.w; a mismatch sets ADC.
  - Enabled when 97.x bit 0 = 0.
  - With the real camera matrices it agrees with the port's Face::Back cull for every in-front triangle, so it is not the cause.
- **Reject pass and clip pass:** both are gated by 206.x & 0x200; only 0x300 nodes run them.
  - Reject pass (0x37b) flags vertices over a 3-vertex window with FCAND and sets w |= 0xC000.
  - Clip pass (0x30b) does a trivial reject by FCOR masks, then Sutherland–Hodgman (0x4ad, kick 0x54a).
- **GS dump of a stage-7 frame** (`context/b157/c07.gs` → `c07.pkl`, 204k prims over 5 frames):
  - No prim reaches the wall's Z.
  - Not one clip-pass (re-tessellated) polygon is present.
  - Every drawn 3D vertex lies within GS 2048 ± 1261; the wall's corners project to GS x 112 / 3970.
  - So the original drops any triangle with a vertex outside the guard band instead of clipping it.

## Port
- `gs.wgsl` `GS_GUARD` (court models only, `GsKey.guard`, set by `for_court_batch`) drops a triangle when any
  vertex has w ≤ 0 or lies outside GS 2048 ± 1280. That band is ±4 half-widths across, taken at the original's
  4:3 so widescreen keeps the same world-space band, and ±1280/112 half-heights of the 224-line field.
- The vertex position is set to NaN, so the GPU drops every triangle that uses that vertex.
- Applying the rule to all court nodes is the same as the original's per-node classification: 0x100 nodes are fully
  inside the guard band.
- **Not proven:** the band size 1280 is inferred (≥ the observed 1261, a power-of-two-friendly GS value), not read
  from the microcode's constant. Rendering only has to look right.

## Check
- `context/b157/cmp_s07.png` and `cmp_s08.png` (original on top, port below): the wall and the foliage are gone and
  both match.
- Stages 1–6, 9, 10 and 11, guard on vs off (`context/b157/all/`): only animated characters differ (run-to-run
  timing), plus a uniform brightness change on 11 (weather). Stage 0 isn't a court.
- Stage 8's tree shadow is sharper than the original's. The user likes it high-res (2026-10-13), so that is stretch
  task G2: original-resolution shadows in the "Original" look.
