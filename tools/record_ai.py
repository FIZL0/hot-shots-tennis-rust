#!/usr/bin/env python3
"""Record the computer players' AI objects every frame from a save-state load (slot 5: the all-bot doubles match).
Usage: record_ai.py <slot> <frames> <out.bin> [ai bytes, default 0x250] [extra u32 globals, comma-separated].
Header: u32 count (4), then count × u32 AI object address (*(player + 0x80), players at *(gm + 0xa8 + 4i)).
Sample = u32 vsync, the score globals 0x423048..0x423060, the AI's MT19937 at 0x427130 (0x9d0: state at +4, index
at +0x9c4), then the extra globals (8 bytes each from the given address, if given), then per AI its first 0x250 (or the given)
bytes. A frame that ticks mid-read is skipped (missed); stops when the
match object changes."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, MT, AI_SIZE = 0x1d5780, 0x422f80, 0x427130, 0x250
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
if len(sys.argv) > 4: AI_SIZE = int(sys.argv[4], 0)
EXTRA = [int(a, 0) for a in sys.argv[5].split(",")] if len(sys.argv) > 5 else []

p = Pine(step=True)
p.load_state(slot)
time.sleep(0.3)
gm0 = p.read32(GM_PTR)
ais = [p.read32(p.read32(gm0 + 0xa8 + 4 * i) + 0x80) for i in range(4)]
out.write(struct.pack("<5I", 4, *ais))
regions = [(0x423048, 0x18), (MT, 0x9d0)] + [(a, 4) for a in EXTRA] + [(a, AI_SIZE) for a in ais]

last, n, missed = p.read32(VSYNC), 0, 0
while n < want:
    v = p.next_frame(last)
    last = v
    if p.read32(GM_PTR) != gm0:
        sys.exit(f"match object gone at vsync {v}, {n} samples")
    a = p.settle(regions, v)
    if a is None:
        missed += 1
        if missed > 300: sys.exit(f"300 frames in a row missed at vsync {v}, {n} samples")
        continue
    missed = 0
    out.write(struct.pack("<I", v) + a)
    out.flush()
    n += 1
    if n % 1200 == 0: print(n, flush=True)
print("done", n)
