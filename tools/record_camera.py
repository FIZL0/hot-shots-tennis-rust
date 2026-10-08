#!/usr/bin/env python3
"""Record the in-match cameras every frame from a save-state load (bot games: slot 5), for P16.
Usage: record_camera.py <slot> <frames> <out.bin>. A PCSX2 copy runs at 1x (tools/pine.py; HST_LOCKSTEP=1: every frame, slowly); the user's own PCSX2: run it slowed (NominalScalar 0.25).
Sample = u32 vsync + gm 0x100 + camera handler *(gm+0xbc) 0x200 + its cameras *(+0x5c) 0x140, *(+0x60) and
*(+0x64) (0x200 head + 0x2f40..0x3100 each) + 4 player positions (+0x3d40 matrix, 0x40) + live ball 0x290."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR = 0x1d5780, 0x422f80
p, want, out = Pine(step=True), int(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(int(sys.argv[1]))
time.sleep(0.3)

def regions():
    gm = p.read32(GM_PTR)
    ch = p.read32(gm + 0xbc)
    r = [(gm, 0x100), (ch, 0x200), (p.read32(ch + 0x5c), 0x140)]
    for o in (0x60, 0x64):
        c = p.read32(ch + o)
        r += [(c, 0x200), (c + 0x2f40, 0x1c0)]
    r += [(p.read32(gm + 0xa8 + 4 * i) + 0x3d40, 0x40) for i in range(4)]
    return gm, r + [(p.read32(gm + 0x88), 0x290)]

last, n = p.read32(VSYNC), 0
while n < want:
    v = p.next_frame(last)
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    gm, r = regions()
    a = p.settle(r, v)
    if p.read32(GM_PTR) != gm: sys.exit(f"match object gone at vsync {v}, {n} samples")
    if a is None:
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        continue
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 1200 == 0: print(n, flush=True)
print("done", n)
