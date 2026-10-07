#!/usr/bin/env python3
"""Record the live ball every frame from a save-state load (bot games: slot 5).
Usage: record_live.py <slot> <frames> <out.bin>. Run PCSX2 slowed down ([Framerate] NominalScalar = 0.25) so
every frame is caught; a sample is taken only when two reads in a row agree (never torn mid-frame).
Sample = u32 vsync + live ball *(gm+0x88) 0x290 + predictor *(gm+0x98) 0x290 + rally block 0x3165f0 0x40."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR = 0x1d5780, 0x422f80
p, want, out = Pine(), int(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(int(sys.argv[1]))
time.sleep(0.3)
gm = p.read32(GM_PTR)
r = [(p.read32(gm + 0x88), 0x290), (p.read32(gm + 0x98), 0x290), (0x3165f0, 0x40)]
last, n = p.read32(VSYNC), 0
while n < want:
    v = p.read32(VSYNC)
    if v == last:
        continue
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    time.sleep(0.004)
    a = p.settle(r, v)
    if a is None:
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        continue
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 1200 == 0: print(n, flush=True)
print("done", n)
