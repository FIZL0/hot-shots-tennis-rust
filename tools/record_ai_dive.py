#!/usr/bin/env python3
"""Record what the computer players' dive and call-out decisions read, every frame from a save-state load (P11h).
Usage: record_ai_dive.py <slot> <frames> <out.bin>.
Header: u32 player count; per player u32 player address, u32 AI address (*(player + 0x80)).
Sample = u32 vsync, the score globals 0x423048..0x423060, the AI's ball path 0x424f80 (16 steps of 0x30), then per
player its 0x12b0..0x12b8 (side sign f32), 0x13a8..0x13c0 (TParam reach block), 0x3d60..0x3d80 (facing, position),
0x3e00..0x3e10 (run velocity), 0x3f58..0x3f60 (dive flag), 0x3f80..0x3f90 (dive slide, counter), 0x3fa4..0x3fac
(+0x3fa5 locomotion state) and its AI's first 0x280 bytes. A frame that ticks mid-read is skipped."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, PATH, AI_SIZE = 0x1d5780, 0x422f80, 0x424f80, 0x280
PL = [(0x12b0, 8), (0x13a8, 0x18), (0x3d60, 0x20), (0x3e00, 0x10), (0x3f58, 8), (0x3f80, 0x10), (0x3fa4, 8)]
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")

p = Pine(step=True)
p.load_state(slot)
time.sleep(0.3)
gm0 = p.read32(GM_PTR)
pls = [a for a in (p.read32(gm0 + 0xa8 + 4 * i) for i in range(4)) if a]
ais = [p.read32(a + 0x80) for a in pls]
out.write(struct.pack("<I", len(pls)) + b"".join(struct.pack("<2I", a, b) for a, b in zip(pls, ais)))
regions = [(0x423048, 0x18), (PATH, 0x300)]
for pl, ai in zip(pls, ais): regions += [(pl + o, n) for o, n in PL] + [(ai, AI_SIZE)]

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
