#!/usr/bin/env python3
"""B36: read the ball's shadow state from a running game (slot 5): ball scale/radius/alpha, the shadow's
matrix and ground normal, the camera position; prints one line per ~10 frames. Usage: b36_shadow.py [frames]"""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from pine import Pine
p = Pine()
gm = p.read32(0x422f80)
v = p.read32(0x1d5780)
for _ in range(int(sys.argv[1]) if len(sys.argv) > 1 else 12):
    v = p.next_frame(v)
    b = p.read32(gm + 0x88)
    f = lambda a, n=1: struct.unpack(f"<{n}f", p.read_block(a, (4 * n + 7) & ~7)[:4 * n])
    print(v, "pos", f(b + 0xe0, 3), "r/scale", f(b + 0x8d0, 2), "alpha", f(b + 0x970), "n", f(b + 0x8e0, 4),
          "\n   m", [round(x, 4) for x in f(b + 0x930, 16)], "cam", f(0x1e7d20, 3), "fov", f(0x1e7d50),
          "mode", p.read8(gm + 0x55), "fl8b8", p.read8(b + 0x8b8))
    for _ in range(9): v = p.next_frame(v)
