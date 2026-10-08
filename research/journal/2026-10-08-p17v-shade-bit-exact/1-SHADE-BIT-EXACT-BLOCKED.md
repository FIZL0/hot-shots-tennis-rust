# P17v — shade map bit for bit: blocked on the reference

The target is `context/p17l/map05.bin`. It turns out to be what **PCSX2's Vulkan HW renderer** gives, not what the
game itself computes.

## Findings

- **The map is a GPU artifact.** The game builds it by drawing to the GS and reading the red bytes back
  (threshold red > 0x6f), so the map depends on the emulator's rasterizer.
  - A fresh rebuild on copy 4 with the HW renderer (`hw05.map`) equals map05.bin bit for bit.
  - The same rebuild with the SW renderer (`Renderer = 13`, `sw05.map`) differs in 2 926 of 1 146 880 bits
    (38 841 set vs 38 959).
  - Two SW rebuilds (`cap0.red`, `sw05.red`) differ in 16 277 red bytes, so the SW reds also depend on when the
    map is built. The varying input is not found.
  - A real PS2 GS would give yet another map.
  - Bit for bit against map05 therefore means emulating PCSX2's Vulkan rasterization and bilinear filtering on
    the host GPU. That is no target for a port.
- **Receiver pass GS state** (from a GS dump in the middle of a build: `p17v_capture.py`, then `p17v_gsprims.py`):
  - Prim and texture: PRIM tristrip, TME, FST off. TEX0 PSMT4 128², TBW 2, CLUT CT32 with entries i·0x20 clamped
    to 0xff. TFX DECAL, TCC 1. TEX1 bilinear. CLAMP is clamp.
  - Blend and masks: ALPHA Cs + Cd with FIX 0x80, COLCLAMP. FBMSK masks R and G.
  - Depth and screen: ZTST GEQUAL against a z-buffer cleared to 0, so every pixel passes. SCISSOR is 640×224,
    with XYOFFSET (1728, 1936) plus the tile offset.
  - The shadow textures are resident in VRAM before the receiver tiles are drawn.
- **Receiver vertices are affine in world space.** This is the "¾ column scale and orthographic fit" from P17r.
  - X = 613.27 + 3.2031·z and Y = 469.64 + 3.9871·x (pixels).
  - Q is constant at 1/411.59 over the pass, so the 40° camera's perspective is flattened out. The fit happens
    on the VU1 path, which is not traced to the instruction.
- **Dump-driven SW sim** (`p17v_gssim.py`) uses the game's own vertices and textures, with PCSX2 SW's AVX2
  scanline (v2.9.52 source). Its results against the reds read back from the same build:

  | Tile | Reds that differ | Max abs(diff) | Threshold bits that differ |
  | --- | --- | --- | --- |
  | 0 | 0 | 0 | 0 |
  | 1 | 157 | 16 | 3 |
  | 2 | 14 | 8 | 0 |
  | 3 | 552 | 16 | 13 |

  - Every residual is on the constant-Q (`q_div`, fst) path: a u/v offset of about 0x18–0xa7 (16.16) at a span
    start, which flips one bilinear fraction step.
  - Ruled out: vector length 4 vs 8, forcing constant Q off, and a PCSX2 version mismatch.
  - Not yet checked: draw batching and flush points, and the strip vertex order.

## Tools (research/)

| Script | What it does |
| --- | --- |
| `p17v_rebuild.py` | Rebuilds the map in game, keeping each pixel's red (patches the read-back loop) |
| `p17v_capture.py` | Same as `p17v_rebuild.py`, plus a GS dump (F11) of the same build |
| `p17v_gsprims.py` | Lists every prim of a dump, with its vertices and the context state |
| `p17v_gssim.py` | The SW replay above |

Captures are in `context/p17v/` (git-ignored): `cap0.*`, `hw05.*`, `sw05.*`.

## Status

`[~]` BLOCKED: the reference is renderer-specific (HW == map05, SW ≠). It needs a decision on which reference
the port matches: the HW map, the SW map, or the receiver-pass inputs (vertices and textures). The genuine port
parts are split out as P17v1–P17v4.
