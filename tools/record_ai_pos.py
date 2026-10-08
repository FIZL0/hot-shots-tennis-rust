#!/usr/bin/env python3
"""Record the computer players' AI objects, positions and the ball every frame from a save-state load (P11e).
Usage: record_ai_pos.py <slot> <frames> <out.bin>.
Header: u32 count (players, | 0x100 when the extras are there); per player u32 player address, u32 AI address (*(player + 0x80)), then the player's
0x12b0..0x12c0 (side sign f32, ?, index u32, ?) and 0x13f0..0x13f8 (pad u32, formation byte at +4); u32 ball address; u32 shot-record table address (*(gm + 0xa4)).
Sample = u32 vsync, the score globals 0x423048..0x423060, the MT19937 at 0x427130 (0x9d0), the ball's 0xb0..0xf0,
the 4 per-player shot records (table + 0x70 + 0x30i, 0x30 each; the shot's position at +0x20),
then per player its 0x3d70..0x3d80 (position) and its AI's first 0x280 bytes, then (extras) per player its 0x3e90..0x3ea0
(the shot's target), 0x3ec0..0x3ed0 and 0x3f90..0x3fa0 (swing state). A frame that ticks mid-read is skipped."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, MT, AI_SIZE = 0x1d5780, 0x422f80, 0x427130, 0x280
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")

p = Pine(step=True)
p.load_state(slot)
time.sleep(0.3)
gm0 = p.read32(GM_PTR)
pls = [a for a in (p.read32(gm0 + 0xa8 + 4 * i) for i in range(4)) if a]  # 2 in singles
ais = [p.read32(a + 0x80) for a in pls]
ball, recs = p.read32(gm0 + 0x88), p.read32(gm0 + 0xa4)
out.write(struct.pack("<I", len(pls) | 0x100))
for pl, ai in zip(pls, ais):
    out.write(struct.pack("<2I", pl, ai) + p.read_regions([(pl + 0x12b0, 0x10), (pl + 0x13f0, 8)]))
out.write(struct.pack("<2I", ball, recs))
regions = [(0x423048, 0x18), (MT, 0x9d0), (ball + 0xb0, 0x40), (recs + 0x70, 0xc0)]
for pl, ai in zip(pls, ais): regions += [(pl + 0x3d70, 0x10), (ai, AI_SIZE)]
for pl in pls: regions += [(pl + 0x3e90, 0x10), (pl + 0x3ec0, 0x10), (pl + 0x3f90, 0x10)]

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
