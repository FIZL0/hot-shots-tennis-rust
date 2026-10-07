#!/usr/bin/env python3
"""Record the swing trails (effects object +0xac + 4·player) every frame from a save-state load (bot games: slot 5).
Usage: record_trails.py <slot> <frames> <out.bin>.

Sample: u32 vsync, trail clock base (0x423f88, u32 + 4 bytes after it), effects object 0x100, then per player (4): trail object 0xa8,
its first 48 points (0x30 each), the player's motion state 0x90 and the racket matrix it points to (0x40)."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, FX_PTR, CLOCK, PLAYERS, POINTS = 0x1d5780, 0x422f80, 0x423f80, 0x423f88, 4, 48
p, want, out = Pine(), int(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(int(sys.argv[1]))
time.sleep(2)  # the load lands asynchronously; pointers read before it are stale
gm, fx = p.read32(GM_PTR), p.read32(FX_PTR)
r = [(CLOCK, 8), (fx, 0x100)]
for i in range(PLAYERS):
    t = p.read32(fx + 0xac + 4 * i)
    m = p.read32(p.read32(gm + 0xa8 + 4 * i) + 0x54)  # the trail's player is only set on its first swing
    r += [(t, 0xa8), (p.read32(t + 0x64), POINTS * 0x30), (m, 0x90), (p.read32(m + 0x88), 0x40)]
last, n = p.read32(VSYNC), 0
while n < want:
    v = p.read32(VSYNC)
    if v == last:
        continue
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    time.sleep(0.004)
    a = p.read_regions(r)
    while True:
        b = p.read_regions(r)
        if a == b and p.read32(VSYNC) == v: break
        a = b
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 1200 == 0: print(n, flush=True)
print("done", n)
