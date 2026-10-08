#!/usr/bin/env python3
"""B24: record court 4's type-15 creature (the ball that bounces by) from a match start. Loads <slot> (a "Start the
match!" screen on court 4), presses ✕ (copy 1 confirms with ✕), waits for the match object, finds the creature
(vtable 0x1d2180, type 15) and its animation controller (+0x5c, frame at +0x38, header *(+0x24), length +0x2c),
then records <frames> samples: u32 vsync, u8 match phase (gm+0x54), the object (0x290), *(+0xb8) (8: playing,
route), the controller (0x40), its header (0x30), the shared MT (0x9d0). Header: u32 object address.
Usage (under tools/pcsx2.sh): b24_rec15.py <slot> <frames> <out.bin>"""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine

TRIG, GM = 0x1d2180, 0x422f80
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
p = Pine()
p.load_state(slot)
time.sleep(1.0)
subprocess.run([os.path.join(os.path.dirname(__file__), "..", "tools", "vpad.py"), "send", "press cross 150"], check=True)
t = time.time()
while not p.read32(GM):
    if time.time() - t > 60: sys.exit("no match object")
    time.sleep(0.02)
gm, obj = p.read32(GM), None
while obj is None:
    for a in range(0x1800000, 0x1e00000, 0x10000):
        b = p.read_block(a, 0x10000)
        for o in range(0, 0x10000, 0x10):
            if struct.unpack_from("<I", b, o)[0] == TRIG and b[o + 0x50] == 15:
                obj = a + o
                break
        if obj: break
    if time.time() - t > 90: sys.exit("no type-15 creature")
print("creature", hex(obj), "phase", p.read8(gm + 0x54), flush=True)
out.write(struct.pack("<I", obj))
last, n = p.read32(0x1d5780), 0
while n < want:
    v = p.next_frame(last)
    last = v
    if p.read32(GM) != gm: sys.exit(f"match object gone at {n}")
    c, b8 = p.read32(obj + 0x5c), p.read32(obj + 0xb8)
    mt = p.read32(p.read32(p.read32(gm + 0x84) + 0x154) + 0x50)
    a = p.settle([(gm + 0x50, 8), (obj, 0x290), (b8, 8), (c, 0x40), (p.read32(c + 0x24), 0x30), (mt, 0x9d0)], v)
    if a is None: continue
    out.write(struct.pack("<IB", v, a[4]) + a[8:])
    out.flush()
    n += 1
print("done", n)
