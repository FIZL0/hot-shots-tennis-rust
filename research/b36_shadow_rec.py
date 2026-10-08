#!/usr/bin/env python3
"""B36: record the ball shadow's inputs and matrix every frame from slot 5 (bot match, court 10).
Usage: b36_shadow_rec.py out.bin [frames]. Record per frame (0x80 bytes): u32 vsync, pad to 0x10; ball pos +0xe0
(0x10); camera eye 0x1e7d20 (0x10); shadow ground normal +0x8e0 (0x10); shadow matrix +0x930 (0x40)."""
import os, struct, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from pine import Pine
p = Pine(step=True)
out, n = sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 600
p.load_state(5)
v = p.read32(0x1d5780)
for _ in range(10): v = p.next_frame(v)
b = p.read32(p.read32(0x422f80) + 0x88)
regs = [(b + 0xe0, 0x10), (0x1e7d20, 0x10), (b + 0x8e0, 0x10), (b + 0x930, 0x40)]
with open(out, "wb") as f:
    got = 0
    while got < n:
        v = p.next_frame(v)
        r = p.settle(regs, v)
        if r is None: print("torn", v, file=sys.stderr); continue
        f.write(struct.pack("<I12x", v) + r); got += 1
print("wrote", got, "frames to", out)
