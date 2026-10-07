#!/usr/bin/env python3
"""Record each player's motion player (anim object *(player+0x54)) every frame from a save-state load (slot 5).
Usage: record_anim.py <slot> <frames> <out.bin>. Run PCSX2 slowed down ([Framerate] NominalScalar = 0.25).
Sample = u32 vsync + gm 0x100 + per player (4): player +0x3c00 0x400, anim object 0x80, its clip's length (f32, clip +0x2c)."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR = 0x1d5780, 0x422f80
p, want, out = Pine(), int(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(int(sys.argv[1]))
time.sleep(0.3)

def regions():
    gm = p.read32(GM_PTR)
    r = [(gm, 0x100)]
    for i in range(4):
        pl = p.read32(gm + 0xa8 + 4 * i)
        a = p.read32(pl + 0x54)
        r += [(pl + 0x3c00, 0x400), (a, 0x80), (p.read32(a + 0x24) + 0x2c, 4)]
    return gm, r

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
        if a == b and p.read32(VSYNC) == v and p.read32(GM_PTR) == gm: break
        if p.read32(GM_PTR) != gm: sys.exit(f"match object gone at vsync {v}, {n} samples")
        a = b
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 1200 == 0: print(n, flush=True)
print("done", n)
