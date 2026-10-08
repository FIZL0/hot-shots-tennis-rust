#!/usr/bin/env python3
"""P3b: record the game's random sources every frame from a save-state load (slot 5: the all-bot doubles match).
Usage: p3b_rng_rec.py <slot> <frames> <out.bin>. Run under tools/pcsx2.sh.
Sample = u32 vsync, u64 newlib rand() state (reent +0xa8), then four MT19937 blocks of 0x9d0 (state at +4, index at
+0x9c4): the match's shared one (*(gm+0x80)), the AI's (0x427130), the court's (*(*(*(gm+0x84)+0x154)+0x50)) and the
sound manager's (*(0x43b1d0)+0x740). A frame that ticks mid-read is skipped."""
import struct, sys, time, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine

VSYNC, GM_PTR, AI_MT, REENT, SND = 0x1d5780, 0x422f80, 0x427130, 0x1b80f0, 0x43b1d0
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
p = Pine(step=True)
p.load_state(slot)
time.sleep(0.3)
gm = p.read32(GM_PTR)
mts = [p.read32(gm + 0x80), AI_MT, p.read32(p.read32(p.read32(gm + 0x84) + 0x154) + 0x50), p.read32(p.read32(SND) + 0x740)]
print("generators", [hex(a) for a in mts])
regions = [(p.read32(REENT) + 0xa8, 8)] + [(a, 0x9d0) for a in mts]
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
