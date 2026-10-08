#!/usr/bin/env python3
"""P3e1: the gallery's manager (*0x43b1c0, 0x1bd0 bytes), the umpire (*0x42d6c0, 0x600), the match (gm, 0x400)
and the court generator (*(*(*(gm+0x84)+0x154)+0x50), 0x9d0) every frame from a save-state load, lock-step.
Usage: p3e1_gallery_rec.py <slot> <frames> <out.bin>. Run under tools/pcsx2.sh with HST_LOCKSTEP=1.
Sample = u32 vsync, then the four blocks in that order."""
import struct, sys, time, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine

VSYNC, GM_PTR = 0x1d5780, 0x422f80
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
p = Pine(step=True)
p.load_state(slot)
time.sleep(0.3)
gm = p.read32(GM_PTR)
regions = [(p.read32(0x43b1c0), 0x1bd0), (p.read32(0x42d6c0), 0x600), (gm, 0x400),
           (p.read32(p.read32(p.read32(gm + 0x84) + 0x154) + 0x50), 0x9d0)]
print("regions", [hex(a) for a, _ in regions])
last, n, missed = p.read32(VSYNC), 0, 0
while n < want:
    v = last = p.next_frame(last)
    a = p.settle(regions, v)
    if a is None:
        missed += 1
        if missed > 300: sys.exit(f"CAPTURE FAILED: 300 frames in a row missed at vsync {v}, {n} samples")
        continue
    missed = 0
    out.write(struct.pack("<I", v) + a)
    n += 1
print("done", n)
