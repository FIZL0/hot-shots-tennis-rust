#!/usr/bin/env python3
"""Record the in-match cameras every frame from a save-state load (bot games: slot 5), for P16.
Usage: record_camera.py <slot> <frames> <out.bin>. Run PCSX2 slowed down ([Framerate] NominalScalar = 0.25).
Sample = u32 vsync + gm 0x100 + camera handler *(gm+0xbc) 0x200 + its cameras *(+0x5c) 0x140, *(+0x60) and
*(+0x64) (0x200 head + 0x2f40..0x3100 each) + 4 player positions (+0x3d40 matrix, 0x40) + live ball 0x290."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR = 0x1d5780, 0x422f80
p, want, out = Pine(), int(sys.argv[2]), open(sys.argv[3], "wb")
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
    v = p.read32(VSYNC)
    if v == last:
        continue
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    time.sleep(0.004)
    gm, r = regions()
    a = p.read_regions(r)
    while True:
        b = p.read_regions(r)
        if a == b and p.read32(VSYNC) == v: break
        if p.read32(GM_PTR) != gm: sys.exit(f"match object gone at vsync {v}, {n} samples")
        a = b
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 1200 == 0: print(n, flush=True)
print("done", n)
