#!/usr/bin/env python3
"""P3e2: record the sound manager's generator with the hit sparks every frame from a save-state load (slot 5).
Usage: p3e2_sound_rec.py <slot> <frames> <out.bin>. Run under tools/pcsx2.sh.
Sample = u32 vsync, effects object 0x100 (+0xd0 shot kind, +0xd8+8p offset/grade/branch per player), sparks object
0x70, its 25 particles (0x40 each), its 25 rolls (0x50 each), the sound manager's MT19937 (0x9d0: state at +4,
index at +0x9c4). A frame that ticks mid-read is skipped."""
import struct, sys, time, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine

VSYNC, FX_PTR, SND, N = 0x1d5780, 0x423f80, 0x43b1d0, 25
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
p = Pine(step=True)
p.load_state(slot)
time.sleep(2)  # the load lands asynchronously; pointers read before it are stale
fx = p.read32(FX_PTR)
sp = p.read32(fx + 0xa8)
regions = [(fx, 0x100), (sp, 0x70), (p.read32(sp + 0x5c), N * 0x40), (p.read32(sp + 0x64), N * 0x50),
           (p.read32(p.read32(SND) + 0x740), 0x9d0)]
print("regions", [hex(a) for a, _ in regions], flush=True)
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
    if n % 300 == 0: print(n, v, flush=True)
print("done", n)
