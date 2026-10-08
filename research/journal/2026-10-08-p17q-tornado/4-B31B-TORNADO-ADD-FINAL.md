# B31b — tornado add per pixel

## Cause
- The port drew the tornado with the court's model light: `weather::apply` relights every GsMaterial on a weather
  change (ambient 0.638, light 0.60 × sun diffuse). The original lights it with the default light, ambient 0.5 plus
  0.49 × diffuse, with the light in world space along (0.408, 0.816, 0.408). From GS dump `context/b31b/on.gs`, the
  vertex RGB / model vc is 0.50 minimum and 0.50–0.55 on average, up to 0.72. A least-squares fit of the direction
  from the model normals gives (0.406, 0.803, 0.404), residual 0.014. 0.638/0.5 ≈ 1.28 was the gap.
- Fix (`play/tornado.rs`): the tornado is drawn as GS draws (`GsMaterial` per MTL material, raw texels, the first
  packet's PRIM). `draw` runs after `weather::apply` and resets the light to `gs::DEFAULT_LIGHT` each frame.

## Equal to the GS state (dump draws 2035–2039, 304 triangles)
- ALPHA 0x48 (Cs·As>>7 + Cd). TEST: alpha test NEVER, AFAIL RGB_ONLY, so no Z write; ZTST GEQUAL.
- PRIM 0x5c. TEX0 MODULATE, TCC 1. TEX1 MXL 0, linear. CLAMP REPEAT. All textures 128×128; the port's textures
  have no mips either.
- Vertex alpha = vc alpha × MTL alpha × opacity (61 = 128 × 0.5 × 0.953). The UV offsets match the dump.
- Geometry: the matrix is uniformly scaled by t (5.3308 over PINE).
  - The dump's DLT gives the drawn pose: rows [0.9585 −0.0025 0.2849] [0.0128 0.9993 −0.0344]
    [−0.2847 0.0367 0.9579], T (1.4892, −2.5297, −7.843).
  - Projection: fx 1359 per tangent for 320 px, fy 846 for 112 field lines, tan_v 0.1324. The port uses 0.1322.

## Measurement
- `research/b31b_rast.py`: a software raster of the dump's five tornado draws into the 640×224 field.
  - MODULATE with truncation, bilinear REPEAT, s/t/q screen-linear, Gouraud, Cs·As>>7 add.
  - Textures go in `context/b31b/tex{0,1}.rgba`.
- The port is frozen at the dump's camera, pose, scale and UVs (a temporary hook, not committed). Its on/off diff is
  resampled to the field grid by the exact tangent scale.

| | px > 3 | mean/ch | p50/90/99 | sum |
|---|---|---|---|---|
| original's draws (raster) | 1540 | 37.2 | 25 / 87 / 150 | 57.7k |
| same, without GS truncation | 1687 | 38.2 | 26 / 91 / 156 | 65.0k |
| port before (court light) | 1831 | 42 | 30 / 98 / 161 | 76.8k |
| port now | 1807 | 36.8 | 25 / 90 / 149 | 67.1k |

Per pixel the port now matches the original's draws within a few levels. Most of the remaining sum (57.7k → 65.0k
of 67.1k) is the GS's per-step integer truncation, which the port's float blend skips.

## Why the original's screenshot reads dimmer (≈0.85× its own draws)
Background clipping accounts for only 2%. The rest comes after the tornado:
- Draws 2040–2045 are other effects, normal-blended (ALPHA 0x44) over its box (TBP 0x23c0/0x23e0/0x2400/…).
- 2047 is a full-screen sprite, ALPHA 0x64 FIX 0x20: 25% of buffer TBP 0x8c0 mixed into the frame.

## Gaps (PLAN)
- B31c: the GS's integer truncation for `@add`/MODULATE draws.
- B31d: the draws after the tornado (2040–2045 effects over it, the 2047 25% frame blend) in the port, in the
  original's order.
