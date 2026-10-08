#!/usr/bin/env python3
"""P17q: freeze the original on the ball tornado for a screenshot: slot <slot>, wait until the tornado has grown to
`frac` of its full size, then set slow motion to one update per 3000 frames (0x2ef090) and leave it running.
Usage: research/p17q_freeze.py <slot> <frac> [off] ('off' ends the slow motion)."""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine

VSYNC, CE_PTR, SLOW = 0x1d5780, 0x43b1d0, 0x2ef090
p = Pine(step=True)
if sys.argv[-1] == "off":
    p.write32(SLOW, 0); sys.exit()
p.load_state(int(sys.argv[1]))
time.sleep(2)
last = p.next_frame(p.read32(VSYNC))
tor = p.read32(p.read32(CE_PTR) + 0x728)
for _ in range(3000):
    last = p.next_frame(last)
    t, a = p.read_block(tor + 0xb8, 8), p.read_block(tor + 0xc8, 8)
    on = p.read8(tor + 0x62)
    if on and struct.unpack("<f", t[:4])[0] >= float(sys.argv[2]) * struct.unpack("<f", a[:4])[0]:
        break
for o, w in [(4, 0x3f800000), (8, 0x453b8000), (0xc, 0x453b8000), (0x10, 0), (0x14, 0), (0x18, 0), (0x1c, 0),
             (0x20, 0), (0x24, 3000), (0x28, 3000), (0x2c, 0), (0, 1)]:
    p.write32(SLOW + o, w)
print("frozen at vsync", last, "t", struct.unpack("<f", t[:4])[0])
