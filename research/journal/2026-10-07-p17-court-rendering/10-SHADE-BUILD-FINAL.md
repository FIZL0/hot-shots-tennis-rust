# P17r — sun-shade map build

The port used to cast the casters' silhouettes along the sun onto the hole (IoU 0.23 against the game's map on
court 10, `context/p17l/map05.bin`). It now draws the game's shadow textures onto the hole the way the game does.
The result is IoU **0.947** (game 1 108 k px; about 300 missed and 1 800 extra). This is not bit for bit; the
gaps are P17u and P17v.

## What the game does (court 10, slot 5 RAM `context/p17l/ee05.bin`)

- **Caster light frame** (the caster setup routine, called per static caster):
  - Box: the caster model's vertex box. The centre is kept and the half-extents are grown ×1.1 (slot +0x40 /
    +0x50).
  - Light axes: lu = (0,0,1) × sun, normalized (or (1,0,0) when |sun.z| ≥ 0.99999); lv = sun × lu. The sun is
    unit length, sun → ground.
  - Scale s = max(Σ|aᵢ·lu|, Σ|aᵢ·lv|), where aᵢ are the box's world half-axes.
  - The light matrix M is the inverse of the rows [s·lu; s·lv; s·sun; centre]. This was reproduced for all 17
    casters to < 2e-5 (shinpan 1e-4) of RAM.
- **Clip planes** (caster +0x50, stored transposed in two 4×4 blocks):
  - Plane 0 is a cap: the sun with its y dropped (unless |y| ≥ 0.99), through the box corner turned away from
    the sun.
  - Next, up to 6 side planes: sun × edge of the box's silhouette hexagon.
  - Unused slots are (0,0,0,1e18).
  - Block 1's last column (+0xc0) is the sun plane, which the build doesn't use.
  - All reproduced to < 1e-4 of RAM.
- **Shadow textures:** 128², 4-bit, with CLUT i·0x20 clamped to 0xff. The best fit for how they are drawn: the
  caster solid at 256² with centre 126, at 128·u + 126, with 1/16 vertex snapping and the top-left fill rule.
  Every other pixel is then kept. Texels still differ by about 3–5 % at edges. The tree casters' textures (casters
  0–5) are speckled by their leaf alpha; the port draws them solid.
  - Decoding the captured VRAM needs a column rotation of 8 within each 32-column block. (IoU 0.951 with the
    decoded textures; ≤ 0.48 with any other rotation.)
- **Receiver pass:**
  - Every hole triangle (all materials, identity node matrices) is drawn once per caster that isn't culled. A
    caster is culled when all 3 corners lie outside one of its planes 0–6.
  - There is no depth test. The caster's texture is sampled bilinearly (4-bit fraction, clamped) at
    (0.5u + 0.5, 0.5v + 0.5) from [x y z 1]·M, and the reds are added.
  - Clamped to 255, red > 0x6f is shade.
  - Variants scored:

    | Variant | IoU |
    | --- | --- |
    | Additive, cull | 0.951 |
    | Nearest surface | 0.924 |
    | No cull | 0.918 |
    | Per-pixel planes | 0.951 (same as cull) |

- **Screen:**
  - On paper the camera is the 40° perspective one the frame is derived from: 1280×896, read back through
    640×224 tiles with XYOFFSET.
  - The map, however, fits an orthographic screen:
    - col = 639.5 + 0.75·SZ·(z − CZ), where CZ = OZ + 640/SZ
    - row = 447.5 + SX·(x − CX), where CX = OX + 448/SX
    - (OZ, OX, SZ, SX) is `Frame`.
    - Vertices snap to 1/16 (floor), pixel centres are whole pixels, and the top-left fill rule applies.
  - Where the ¾ and the missing perspective come from is not found (VU1 path?) → P17v.

| Textures | IoU |
| --- | --- |
| port (solid) | 0.947 |
| game trees + port rest | 0.975 |
| all game (decoded VRAM) | 0.951 |

(Casters 7 and 8 share a texture page, so the VRAM decode only holds the last one drawn.)

## Port

- `hst-sim/src/shade.rs`: `Caster`, `build` (light frame, planes, texture, GS raster in fixed point, bilinear
  sample, sum and threshold). `fill` and `rasterize` are gone.
- `hst/src/shade.rs` / `main.rs` pass each caster's model triangles, axes and position.
- Test: `hst-sim/tests/shade.rs` builds court 10 from the disc. It checks IoU > 0.94 against `map05.bin` when
  both are present.

## Gaps

- **P17u:** leaf alpha in the tree casters' shadow textures (needs the shadow-texture draw's GS state).
- **P17v:** bit for bit, which needs:
  - the ¾ column scale and orthographic fit explained
  - the setup in the game's floats
  - the remaining texture edge texels

The method lives in scratch Python (sim.py/mcalc.py); not kept.
