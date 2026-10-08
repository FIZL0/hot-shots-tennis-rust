#!/usr/bin/env python3
"""Record the hit sparks (effects object +0xa8) every frame from a save-state load (bot games: slot 5).
Usage: record_sparks.py <slot> <frames> <out.bin>.

Sample: u32 vsync, effects object 0x100, sparks object 0x70, its 25 particles (0x40 each), its 25 random entries
(0x50 each: rotation 0x40, three uniforms), ball 0x290, court marker 8 (0x40)."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, FX_PTR, N = 0x1d5780, 0x422f80, 0x423f80, 25
p, want, out = Pine(step=True), int(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(int(sys.argv[1]))
time.sleep(2)  # the load lands asynchronously; pointers read before it are stale
gm, fx = p.read32(GM_PTR), p.read32(FX_PTR)
sp = p.read32(fx + 0xa8)
r = [(fx, 0x100), (sp, 0x70), (p.read32(sp + 0x5c), N * 0x40), (p.read32(sp + 0x64), N * 0x50),
     (p.read32(gm + 0x88), 0x290), (p.read32(gm + 0xb8) + 0xd130 + 8 * 0x40, 0x40)]
last, n = p.read32(VSYNC), 0
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
