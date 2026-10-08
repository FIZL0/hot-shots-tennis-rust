#!/usr/bin/env python3
"""Fixture for shade.rs `receiver_tiles_match_the_game` (P17v3): the receiver pass's draws from a GS dump and the reds
the game read back.

Usage: p17v3_fixture.py dump.gs prims.pkl ref.red out.txt   (as p17v_gssim.py)

Lines: `X i hex` texture i (128² reds through its CLUT), `T gx gy` a tile (its 640×224 window's origin in the
1280×896 frame), `D i` a draw (one batch: one texture), `V` a triangle (per vertex: x y in 12.4 relative to XYOFFSET,
S T Q as f32 bits), `R hex` the tile's expected reds."""
import pickle, struct, sys
import numpy as np
from p17v_gssim import vram, texture

dump, pk, ref, out = sys.argv[1:5]
vm = vram(dump)
red = np.fromfile(ref, np.uint8).reshape(896, 1280)
bits = lambda f: "%08x" % struct.unpack("<I", struct.pack("<f", f))[0]
texs, lines, tile = {}, [f"# p17v3_fixture.py {dump} {pk} {ref}"], None
for f, st, vs in pickle.load(open(pk, "rb")):
    if st["frame"] & 0x1ff != 0xd2 or st["prim"] != 0x5c:
        continue
    xyo, tex0 = st["xyoffset"], st["tex0"]
    ox, oy = xyo & 0xffff, (xyo >> 32) & 0xffff
    if tile != f:
        if tile is not None:
            lines.append("R " + g.tobytes().hex())
        tile, key = f, None
        gx, gy = ox // 16 - 1728, oy // 16 - 1936
        g = red[gy:gy + 224, gx:gx + 640]
        lines.append(f"T {gx} {gy}")
    if tex0 not in texs:
        texs[tex0] = len(texs)
        lines.append(f"X {texs[tex0]} " + texture(vm, tex0).tobytes().hex())
    if key != tex0:
        key = tex0
        lines.append(f"D {texs[tex0]}")
    lines.append("V " + " ".join(f"{x - ox} {y - oy} {bits(s)} {bits(t)} {bits(q)}" for x, y, z, s, t, q, _, _ in vs))
lines.append("R " + g.tobytes().hex())
open(out, "w").write("\n".join(lines) + "\n")
