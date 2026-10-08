#!/usr/bin/env python3
"""P17q: record the ball tornado through forced slow motion (bot game, slot 5). Waits for the tornado to come on,
then sets the game's slow-motion block (0x2ef090) to a constant ×4 (three in-between frames per update) and records
until it is off. Usage (from tools/): research/p17q_slowmo_rec.py <slot> <out.bin> [max_wait_frames].

Header u32 fade. Sample: u32 vsync, slow-mo block 0x30, tornado object 0x190, its model +0x120 (8), its UV
animation 0x30, ball 0x290."""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine

VSYNC, GM_PTR, CE_PTR, SLOW = 0x1d5780, 0x422f80, 0x43b1d0, 0x2ef090
p, out = Pine(step=True), open(sys.argv[2], "wb")
wait = int(sys.argv[3]) if len(sys.argv) > 3 else 3000
p.load_state(int(sys.argv[1]))
time.sleep(2)
last = p.next_frame(p.read32(VSYNC))
gm, ce = p.read32(GM_PTR), p.read32(CE_PTR)
tor, ball = p.read32(ce + 0x728), p.read32(gm + 0x88)
model = p.read32(p.read32(p.read32(tor + 0x50)))
anim = p.read32(p.read32(p.read32(tor + 0x50) + 4) + 0xc)
r = [(SLOW, 0x30), (tor, 0x190), (model + 0x120, 8), (anim, 0x30), (ball, 0x290)]
out.write(struct.pack("<I", 30))
n, poked, seen_on = 0, False, 0
while n < wait:
    v = p.next_frame(last)
    if v != last + 1: print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    a = p.settle(r, v)
    on = a[0x30 + 0x62]
    if not poked:
        n += 1
        if on:
            # +0 on, +4 frac 1, +8/+0xc step 3.0, ramp 0, +0x20 flags 0, +0x24/+0x28 3 frames, +0x2c 0
            for o, w in [(4, 0x3f800000), (8, 0x40400000), (0xc, 0x40400000), (0x10, 0), (0x14, 0), (0x18, 0),
                         (0x1c, 0), (0x20, 0), (0x24, 3), (0x28, 3), (0x2c, 0), (0, 1)]:
                p.write32(SLOW + o, w)
            poked = True
            out.write(struct.pack("<I", v) + a)
        continue
    out.write(struct.pack("<I", v) + a)
    seen_on = seen_on + 1 if not on else 0
    if seen_on > 8: break
p.write32(SLOW, 0)
print("done", "poked" if poked else "never on")
