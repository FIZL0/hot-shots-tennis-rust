#!/usr/bin/env python3
"""Record the ball-bounce effects (the character-effect manager's bounce object) every frame from a save-state load
(bot games: slot 5). Usage: record_bounce.py <slot> <frames> <out.bin>.

Header: u32 court, then per model (ballbound, smash chakudan) u32 MOR entry count, u32 MTA entry count.
Sample: u32 vsync + 4 pad, bounce object 0xfc0, effects object 0x100, ball 0x290, its first 3 contact records
(0x50 each, zeros while it has none), then per model: model 0x80, mesh +0xe0..+0x140, MOR player 0x30, MTA player
0x30, the MOR entries (0x28 each), the MTA entries (0x20 each)."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, FX_PTR, CE_PTR, COURT = 0x1d5780, 0x422f80, 0x423f80, 0x43b1d0, 0x422f90
p, want, out = Pine(), int(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(int(sys.argv[1]))
time.sleep(2)  # the load lands asynchronously; pointers read before it are stale
gm, fx = p.read32(GM_PTR), p.read32(FX_PTR)
bo, ball = p.read32(p.read32(CE_PTR) + 0x71c), p.read32(gm + 0x88)
r = [(bo, 0xfc0), (fx, 0x100), (ball, 0x290)]
head = [p.read32(COURT)]
models = []
for off in (0x50, 0x74):
    m = p.read32(bo + off)
    mor, mta = p.read32(p.read32(m) + 0xc), p.read32(p.read32(m + 12) + 0xc)
    nm, nt = p.read32(mor + 8), p.read32(mta + 8)
    me, te = [p.read32(p.read32(mor + 0xc) + 4 * i) for i in range(nm)], [p.read32(p.read32(mta + 0xc) + 4 * i) for i in range(nt)]
    models += [(m, 0x80), (p.read32(p.read32(m)) + 0xe0, 0x60), (mor, 0x30), (mta, 0x30)] + [(e, 0x28) for e in me] + [(e, 0x20) for e in te]
    head += [nm, nt]
out.write(struct.pack(f"<{len(head)}I", *head))
last, n = p.read32(VSYNC), 0
while n < want:
    v = p.read32(VSYNC)
    if v == last:
        continue
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    time.sleep(0.004)
    def grab():
        c = p.read32(ball + 0x8f8)
        ok = 0x100000 <= c < 0x2000000 - 0xf0 and c % 8 == 0  # a stale pointer between points fails the batch
        if not ok:
            d = p.read_regions(r + models)
            return d[:len(d) - sum(n for _, n in models)] + bytes(0xf0) + d[len(d) - sum(n for _, n in models):]
        return p.read_regions(r + [(c, 0xf0)] + models)
    a = grab()
    while True:
        b = grab()
        if a == b: break
        a = b
    if p.read32(VSYNC) != v:
        print(f"frame {v} read across a vsync", flush=True)
    out.write(struct.pack("<II", v, 0) + a)
    n += 1
    if n % 1200 == 0: print(n, flush=True)
print("done", n)
