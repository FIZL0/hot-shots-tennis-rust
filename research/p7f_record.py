#!/usr/bin/env python3
"""P7f: record the computer players' stick bytes and run state every frame from a save-state load.
Usage: research/p7f_record.py <slot> <frames> <out.bin> (under tools/pcsx2.sh).
Header: u32 count; per player u32 player address, u32 AI address, the player's 0x12b0..0x12d0 (side, index, size
+0x12c8) and 0x1370..0x1390 (speed +0x1374, max stamina +0x1378, agility +0x1388).
Sample = u32 vsync, globals 0x422f80..0x423100, the match object's first 0x60 bytes (phase +0x55), then per player
0x17d0..0x17e0 (AI stick bytes +0x17d4: x high, z low), 0x3d70..0x3d80 (position), 0x3dc0..0x3dd0 (run target),
0x3df0..0x3e10 (motion, stamina, tick, run, velocity), 0x3fa4..0x3fac (play state, mode) and the AI's first 0x280
bytes. A frame that ticks mid-read is skipped."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, AI_SIZE = 0x1d5780, 0x422f80, 0x280
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
p = Pine()
p.load_state(slot)
time.sleep(0.3)
gm = p.read32(GM_PTR)
pls = [a for a in (p.read32(gm + 0xa8 + 4 * i) for i in range(4)) if a]
ais = [p.read32(a + 0x80) for a in pls]
out.write(struct.pack("<I", len(pls)))
for pl, ai in zip(pls, ais):
    out.write(struct.pack("<2I", pl, ai) + p.read_regions([(pl + 0x12b0, 0x20), (pl + 0x1370, 0x20)]))
regions = [(0x422f80, 0x180), (gm, 0x60)]
for pl, ai in zip(pls, ais):
    regions += [(pl + 0x17d0, 0x10), (pl + 0x3d70, 0x10), (pl + 0x3dc0, 0x10), (pl + 0x3df0, 0x20), (pl + 0x3fa4, 8), (ai, AI_SIZE)]
last, n, missed = p.read32(VSYNC), 0, 0
while n < want:
    v = p.read32(VSYNC)
    if v == last:
        continue
    if v != last + 1 and n:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    time.sleep(0.004)
    if p.read32(GM_PTR) != gm:
        sys.exit(f"match object gone at vsync {v}, {n} samples")
    a = p.settle(regions, v)
    if a is None:
        missed += 1
        if missed > 300: sys.exit(f"CAPTURE FAILED: 300 frames in a row missed at vsync {v}, {n} samples")
        continue
    missed = 0
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 1000 == 0: print(n, flush=True)
print("done", n)
