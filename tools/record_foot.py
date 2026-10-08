#!/usr/bin/env python3
"""Record the footstep puffs, footprints and the tornado ball (character-effect manager +0x720 run object, +0x728
tornado object) every frame from a save-state load (bot games: slot 5). PINE pads each 4-byte region to 8. Usage: record_foot.py <slot> <frames> <out.bin>.

Header: u32 court, u32 players. Sample: u32 vsync, manager +0x740..+0x780, run object 0x130 (head) + 0x1000 (first 32
puffs) + 0xc80 (40 footprints) + 4 (footprint count) + 0x10 (+0xa090 flags), tornado object 0x100, ball 0x290,
weather block (gm+0x84) +0x130..+0x140 and +0x1a20..+0x1a30, then per player (4): player +0x3d40 0x40, +0x3fa4 4, +0x3db0 4,
motion object 0xa0, right toe bone 0x40, left toe bone 0x40."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, CE_PTR, COURT, NPL = 0x1d5780, 0x422f80, 0x43b1d0, 0x422f90, 0x422fa4
p, want, out = Pine(step=True), int(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(int(sys.argv[1]))
time.sleep(2)  # the load lands asynchronously; pointers read before it are stale
last = p.next_frame(p.read32(VSYNC))  # one update sets the toe-bone pointers
gm, ce = p.read32(GM_PTR), p.read32(CE_PTR)
run, tor, ball, w = p.read32(ce + 0x720), p.read32(ce + 0x728), p.read32(gm + 0x88), p.read32(gm + 0x84)
r = [(ce + 0x740, 0x40), (run, 0x130), (run + 0x130, 0x1000), (run + 0x6530, 0xc80), (run + 0x71b0, 4), (run + 0xa090, 0x10),
     (tor, 0x100), (ball, 0x290), (w + 0x130, 0x10), (w + 0x1a20, 0x10)]
for i in range(4):
    pl = p.read32(gm + 0xa8 + 4 * i)
    r += [(pl + 0x3d40, 0x40), (pl + 0x3fa4, 4), (pl + 0x3db0, 4), (p.read32(pl + 0x54), 0xa0),
          (p.read32(run + 0x9c + 8 * i), 0x40), (p.read32(run + 0xa0 + 8 * i), 0x40)]
out.write(struct.pack("<II", p.read32(COURT), p.read32(NPL)))
n = 0
while n < want:
    v = p.next_frame(last)
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    a = p.settle(r, v)
    if a is None:
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        continue
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 1200 == 0: print(n, flush=True)
print("done", n)
