#!/usr/bin/env python3
"""B36b: record the ball model's draw scale and the outline billboard every frame from slot 5 (bot match, court 10).
Usage: b36b_ball_rec.py out.bin [frames]. Record per frame (0x130 bytes): u32 vsync, pad to 0x10; ball pos +0xe0;
camera eye/look/up/fov 0x1e7d20 (0x40); view matrix 0x1e7e30 (0x40); match object +0x50 (0x10, phase at +5);
ball scale/radius +0x8d0 (0x10); alpha +0x970 (0x10); ball model's scale +0x120 (0x10); outline matrix +0xe0 and
scale +0x120 (0x50)."""
import os, struct, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from pine import Pine
p = Pine(step=True)
out, n = sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 600
p.load_state(5)
v = p.read32(0x1d5780)
for _ in range(10): v = p.next_frame(v)
gm = p.read32(0x422f80)
b = p.read32(gm + 0x88)
model = p.read32(p.read32(b + 0x280))
outline = p.read32(b + 0x28c)
regs = [(b + 0xe0, 0x10), (0x1e7d20, 0x40), (0x1e7e30, 0x40), (gm + 0x50, 0x10), (b + 0x8d0, 0x10), (b + 0x970, 0x10),
        (model + 0x120, 0x10), (outline + 0xe0, 0x50)]
with open(out, "wb") as f:
    got = 0
    while got < n:
        v = p.next_frame(v)
        r = p.settle(regs, v)
        if r is None: print("torn", v, file=sys.stderr); continue
        f.write(struct.pack("<I12x", v) + r); got += 1
print("wrote", got, "frames to", out)
