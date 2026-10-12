#!/usr/bin/env python3
"""Record court 5's own creatures (roster kinds 61..66, vtable 0x1d1d80, 0xc0 bytes) from the menu on.
Usage (under tools/pcsx2.sh): p14d_record.py <load slot> <frames> <out.bin> <vpad command>...
The vpad commands are the route from the menu to the match (slot 2 doubles: ✕ ×4, Start, ✕ on the confirm screen,
default court 5). Nothing is pressed until the first serve set-up (match phase 2), then ○ every 80 samples (P1
serves and swings). The creatures are found by walking the match object's tree (+0xc children, list sentinel at
+0x10, first node at +0x14, node {?, next, object}) once all six exist.
Header: u32 count, count × u32 creature address. Sample: u32 vsync, gm (*0x422f80) +0x50..+0x60 (phase at +0x55),
gm +0x328..+0x338 (+0x32e instant replay picked), 0x4230b8 (8 bytes: result kind), judge *0x42d6c0 +0x420 (8: call at
+0x426), the shared MT (0x9d0, index at +0x9c4), then each creature's 0xc0 bytes."""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from pine import Pine

VSYNC, GM, VT = 0x1d5780, 0x422f80, 0x1d1d80
VPAD = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools", "vpad.py")
slot, want, out, route = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb"), sys.argv[4:]
p = Pine()
p.load_state(slot)
time.sleep(1.5)
subprocess.run([VPAD, "send", *route], check=True)

def walk(o, depth=0):
    yield o
    if depth > 8 or not p.read32(o + 0xc):
        return
    n = p.read32(o + 0x14)
    for _ in range(400):
        if n == o + 0x10 or not 0x100000 <= n < 0x2000000:
            return
        yield from walk(p.read32(n + 8), depth + 1)
        n = p.read32(n + 4)

t, objs = time.time(), []
while len(objs) < 6:
    if time.time() - t > 120:
        sys.exit(f"court 5's creatures not found (court {p.read32(0x422f90)}, gm {p.read32(GM):#x})")
    gm = p.read32(GM)
    if gm and p.read32(0x422f90) not in (0, 5):
        sys.exit(f"court {p.read32(0x422f90)}, not 5: run it again (the confirm screen's court varies)")
    if gm:
        objs = [o for o in walk(gm) if p.read32(o) == VT]
    time.sleep(0.05)
print("court", p.read32(0x422f90), "players", p.read32(0x422fa4), "creatures", [hex(o) for o in objs], flush=True)
out.write(struct.pack(f"<I{len(objs)}I", len(objs), *objs))

def regions():
    gm = p.read32(GM)
    return [(gm + 0x50, 0x10), (gm + 0x328, 0x10), (0x4230b8, 8), (p.read32(0x42d6c0) + 0x420, 8),
            (p.read32(p.read32(p.read32(gm + 0x84) + 0x154) + 0x50), 0x9d0)] + [(o, 0xc0) for o in objs]

last, n, missed, gm0, served = p.read32(VSYNC), 0, 0, p.read32(GM), None
while n < want:
    v = p.next_frame(last)
    last = v
    if p.read32(GM) != gm0 or any(p.read32(o) != VT for o in objs):
        sys.exit(f"match over at vsync {v}, {n} samples")
    a = p.settle(regions(), v)
    if a is None:
        missed += 1
        if missed > 300: sys.exit(f"300 frames in a row missed at vsync {v}, {n} samples")
        continue
    missed = 0
    out.write(struct.pack("<I", v) + a)
    out.flush()
    n += 1
    if served is None and a[5] == 2:
        served = n
    if served is not None and (n - served) % 80 == 79:
        subprocess.Popen([VPAD, "send", "press circle 150"])
    if n % 600 == 0: print(n, "phase", a[5], flush=True)
print("done", n)
