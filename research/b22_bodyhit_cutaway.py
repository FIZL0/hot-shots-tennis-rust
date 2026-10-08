#!/usr/bin/env python3
"""B22: does a body-hit point get a cut-away, and of whom? Loads <slot> (default 5), sets every player's collision
size (+0x13ec) to 4 m once the rally is on (as bodyhit_rec.py), then prints at the point end and at each cut-away
start: the shot, roles A/B (scene +0x10f98/c), shown/other (+0x10fa0/4), last hitter 0x423058, winner 0x4230a8,
game end 0x4230b8 and each player's motion (anim +0x20) / reaction (+0x3db0). Usage: b22_bodyhit_cutaway.py [slot]
[frames=1500]"""
import struct, sys, time
sys.path.insert(0, "tools")
from pine import Pine

VSYNC = 0x1d5780
slot = int(sys.argv[1]) if len(sys.argv) > 1 else 5
want = int(sys.argv[2]) if len(sys.argv) > 2 else 1500
p = Pine(); p.load_state(slot); time.sleep(1)
gm = p.read32(0x422f80)
while p.read8(gm + 0x55) != 3:
    time.sleep(0.01)
pl = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
for a in pl:
    p.write32(a + 0x13ec, struct.unpack("<I", struct.pack("<f", 4.0))[0])
i32 = lambda a: struct.unpack("<i", struct.pack("<I", p.read32(a)))[0]
def state(tag):
    sc, ch = p.read32(gm + 0xb8), p.read32(gm + 0xbc)
    cam = p.read32(ch + 0x64)
    mo = [(hex(p.read32(p.read32(a + 0x54) + 0x20)), hex(p.read32(a + 0x3db0))) for a in pl]
    print(tag, "vsync", p.read32(VSYNC), "gm55", p.read8(gm + 0x55), "mode", p.read8(ch + 0x51), "shot", hex(p.read8(cam + 0x114)),
          "A/B", i32(sc + 0x10f98), i32(sc + 0x10f9c), "shown", i32(sc + 0x10fa0), i32(sc + 0x10fa4), "last", i32(0x423058),
          "win", i32(0x4230a8), "ge", i32(0x4230b8), "motions", mo, flush=True)
v0 = p.read32(VSYNC); prev55 = prev51 = None
while p.read32(VSYNC) < v0 + want:
    g55, ch = p.read8(gm + 0x55), p.read32(gm + 0xbc)
    m = p.read8(ch + 0x51)
    if g55 != prev55 or (m not in (0, 0xff)) != (prev51 not in (0, 0xff)):
        state("change")
    prev55, prev51 = g55, m
    time.sleep(0.005)
