#!/usr/bin/env python3
"""P3f3: does a weather change remake the clouds, and how many? Loads save slot 5 (court 10, bot doubles, clear),
logs every rand() call as research/p3f_rand_log.py does, and rewrites the whole weather schedule (game manager
+0xd8, 65 bytes) to each weather in turn, waiting for the court manager's weather byte (+0x135) to follow at a new
point before the next. Prints, per change, the cloud count after it. Right after each change it pauses and appends
to <out.bin>.clouds: vsync, the court's cloud models (count, then their pointers), the cloud count and each cloud's
0x1e0 bytes in list order (head 0x423208, next at +0x144; x, y, z at +0x150).
Usage (under tools/pcsx2.sh): p3f3_cloud_log.py <out.bin> [weather ...] (default 3 5 1 0)."""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from p3f_rand_log import BUF, CODE, DATA, END, GM_PTR, RAND, VSYNC, build, j

CLOUDS = 0x423218
out, seq = sys.argv[1], [int(w) for w in sys.argv[2:]] or [3, 5, 1, 0]
if os.path.exists(out + ".clouds"): os.remove(out + ".clouds")
p = Pine()
p.load_state(5)
while not 7500 <= p.read32(VSYNC) < 7550: time.sleep(0.01)
v0 = p.read32(VSYNC)
while p.read32(VSYNC) - v0 < 4: time.sleep(0.01)
a, b = p.read32(RAND), p.read32(RAND + 4)
gm = p.read32(GM_PTR)
cm = p.read32(gm + 0x84)
p.pause()
p.write32(DATA, BUF)
for n, w in enumerate(build(a, b)): p.write32(CODE + 4 * n, w)
p.write32(RAND, j(CODE))
p.write32(RAND + 4, 0)
p.resume()
try:
    for w in seq:
        # word writes (PINE has no byte write); the 3 bytes past the 65 keep theirs
        for k in range(0, 0x44, 4):
            old = p.read32(gm + 0xd8 + k)
            p.write32(gm + 0xd8 + k, w * 0x01010101 if k < 0x40 else old & ~0xff | w)
        t = time.time()
        while p.read8(cm + 0x135) != w and time.time() - t < 120: time.sleep(0.05)
        p.pause()
        n, models = p.read32(0x423218), p.read32(0x423240)
        snap = struct.pack("<2I", p.read32(VSYNC), models) + b"".join(struct.pack("<I", p.read32(p.read32(0x423248) + 4 * k)) for k in range(models))
        snap += struct.pack("<I", n)
        c = p.read32(0x423208)
        while c:
            snap += p.read_block(c, 0x1e0)
            c = p.read32(c + 0x144)
        p.resume()
        open(out + ".clouds", "ab").write(snap)
        print(f"weather {w}: court {p.read8(cm + 0x135)} at vsync {p.read32(VSYNC)}, clouds {p.read32(CLOUDS)}", flush=True)
        if p.read8(cm + 0x135) != w: break
        time.sleep(1)
finally:
    p.pause()
    p.write32(RAND + 4, b)
    p.write32(RAND, a)
    end = p.read32(DATA)
    p.resume()
data = b"".join(p.read_block(x, min(0x10000, end - x)) for x in range(BUF, end, 0x10000)) if end > BUF else b""
open(out, "wb").write(data)
print(f"{len(data) // 24} calls")
