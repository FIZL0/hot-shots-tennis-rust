#!/usr/bin/env python3
"""A real ball-hits-player in the original: every player's collision size (+0x13ec) is set to <radius> once the
rally is on, so the game's own body test fires and its handler runs (motion 0x2b, message 0x14 to the ball, voice).
Usage: bodyhit_rec.py <slot> <out.bin> <radius> [frames=150] [skip=0: rally frames before poking].
Sample = u32 vsync + gm 0x60 + live ball *(gm+0x88) 0x290 + per player (4): +0x3b40 0x80, +0x3f80 0x40,
anim object *(+0x54) 0x80."""
import struct, sys, time
sys.path.insert(0, "tools")
from pine import Pine

VSYNC, GM_PTR = 0x1d5780, 0x422f80
slot, out, radius = int(sys.argv[1]), open(sys.argv[2], "wb"), float(sys.argv[3])
want = int(sys.argv[4]) if len(sys.argv) > 4 else 150
skip = int(sys.argv[5]) if len(sys.argv) > 5 else 0
p = Pine()
p.load_state(slot)
time.sleep(1)
gm = p.read32(GM_PTR)
pl = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
while p.read8(gm + 0x55) != 3:
    time.sleep(0.01)
v0 = p.read32(VSYNC)
while p.read32(VSYNC) < v0 + skip:
    time.sleep(0.005)
r = [(gm, 0x60), (p.read32(gm + 0x88), 0x290)]
for a in pl:
    r += [(a + 0x3b40, 0x80), (a + 0x3f80, 0x40), (p.read32(a + 0x54), 0x80)]
for a in pl:
    p.write32(a + 0x13ec, struct.unpack("<I", struct.pack("<f", radius))[0])
print("radius set at vsync", p.read32(VSYNC), flush=True)
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
print("done", n)
